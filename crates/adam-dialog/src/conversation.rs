//! Layer 3.5 — multi-turn session state + dialog state tracking.
//!
//! v0.8.5 introduced a flat slot map. v1.4.0 adds proper Dialog State
//! Tracking (DST): the `Conversation` now carries
//!
//! ```text
//!   { slots: HashMap, active_intent: Option<IntentKind>, intent_history: Vec<IntentKind> }
//! ```
//!
//! - `slots` — accumulated entities across turns (name, age, city, occupation).
//! - `active_intent` — the kind of the LAST recognised intent. Used for
//!   follow-up resolution like "ал сіз?" ("and you?") which re-interpretates
//!   against the previous frame.
//! - `intent_history` — ordered list of every recognised intent kind this
//!   session (bounded to MAX_HISTORY to avoid unbounded growth).
//!
//! Deterministic-by-construction: given (slots, active_intent, input, seed),
//! the next turn's output is fully determined. No probabilistic decisions.

use std::collections::HashMap;

use adam_kernel_fst::lexicon::LexiconV1;
use adam_kernel_fst::morphotactics::{Case, NounFeatures, synthesise_noun};
use adam_reasoning::reasoner::DerivedFact;
use adam_reasoning::{Fact as ReasFact, Predicate as ReasPredicate};
use adam_retrieval::{MorphemeIndex, RankConfig, compose::compose_with_city};

use crate::intent::Intent;
use crate::planner::plan_response_with_session;
use crate::realiser::realise;
use crate::semantics::{content_roots, interpret_text_with_lexicon};
use crate::templates::TemplateRepository;

/// Maximum intent-history length retained across turns. Bounded so a
/// long-running session doesn't accumulate an unbounded trace.
const MAX_HISTORY: usize = 32;

/// How the retrieval-fallback path treats the cited corpus sample.
///
/// v1.9.0 introduces [`InSampleCitySwap`](ComposeMode::InSampleCitySwap),
/// which opts into the option-B composition: when the session has a
/// known city and the cited sample mentions a **different** known city
/// in a biography-free context, the mention is rewritten to the user's
/// city. The quote is no longer byte-identical to the corpus. Default
/// stays [`Verbatim`](ComposeMode::Verbatim) — existing callers see no
/// behavioural change.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ComposeMode {
    /// v1.8.5 and earlier behaviour: cited quote is byte-identical to
    /// the corpus sample. Zero fabrication risk.
    #[default]
    Verbatim,
    /// v1.9.0 option-B experiment: city mentions in the cited sample
    /// are swapped for the user's session city when safe. Feature
    /// bundle is preserved (locative stays locative, etc.). See
    /// [`adam_retrieval::compose`] for the safety guards.
    InSampleCitySwap,
}

/// A running multi-turn dialog.
///
/// v1.6.5 adds an optional [`MorphemeIndex`]: when attached, the
/// `Intent::Unknown` fallback consults the index for the parsed
/// `noun_hint` and cites a concrete Kazakh sentence from the committed
/// corpus in place of a bare "түсінбедім". The index is optional so
/// embedders (CLI, tests) that don't want the retrieval dependency
/// can continue to run the 26-intent pipeline stand-alone.
#[derive(Debug, Clone, Default)]
pub struct Conversation {
    /// Entity slot values accumulated across turns
    /// (`name`, `age`, `city`, `occupation`).
    pub session: HashMap<String, String>,
    /// Kind of the last recognised intent. `None` before the first turn.
    /// Used by follow-up resolution (v1.4.0).
    pub active_intent: Option<IntentKind>,
    /// Ordered history of recognised intent kinds, oldest-first.
    /// Bounded to [`MAX_HISTORY`] items.
    pub intent_history: Vec<IntentKind>,
    /// Optional retrieval index. When `Some`, `Intent::Unknown` gets an
    /// `example` injected for any `noun_hint` that has postings in the
    /// index. Added v1.6.5.
    pub morpheme_index: Option<MorphemeIndex>,
    /// Tunable weights + pack purity priors for the v1.7.0 ranker.
    /// Defaults to `RankConfig::default()` — only override for tests
    /// or experiments. Uses `Option` rather than a direct value to keep
    /// `Conversation::default()` cheap and avoid threading the config
    /// through every embedder.
    pub rank_config: Option<RankConfig>,
    /// How the retrieval fallback treats the cited sample. Default
    /// [`ComposeMode::Verbatim`] keeps the v1.8.5 behaviour (byte-
    /// identical corpus quote); [`ComposeMode::InSampleCitySwap`]
    /// enables the v1.9.0 option-B city rewrite. Added v1.9.0.
    pub compose_mode: ComposeMode,
    /// Rule-derived facts produced by `adam_reasoning::reasoner::run`.
    /// When attached, `Intent::Unknown` can cite a reasoning chain
    /// (e.g. «кітап пен ілім бір-біріне байланысты: екеуі де бұлақ
    /// болып табылады») in addition to or instead of a verbatim corpus
    /// sample. Added v2.7. When absent, dialog behaviour matches v2.6
    /// exactly — retrieval path only.
    pub derived_facts: Vec<DerivedFact>,
    /// Extracted (non-derived) facts, used by the reasoning-integration
    /// path to build human-readable "source_chain" renderings. Typically
    /// loaded alongside `derived_facts` from the same artefact pair.
    /// Added v2.7.
    pub extracted_facts: Vec<ReasFact>,
    /// v4.0.3 — investor-safe reasoning mode. When `true`,
    /// `inject_reasoning_chain` only considers derivations whose entire
    /// `source_chain` is rooted in `data/world_core/*.jsonl` (every
    /// supporting fact human-reviewed). Off by default for backwards
    /// compatibility. `adam_chat --safe` flips it on. Mirrors the
    /// investor-safe filter `adam_demo` Part 4 has applied by default
    /// since v4.0.2.
    pub curated_only_reasoning: bool,
    /// v4.0.27 — structured belief state (Codex v4.0.26 roadmap Phase 1).
    /// Tracks every user assertion with provenance, confidence band,
    /// and contradiction history. Lives **alongside** the flat
    /// `session` map — `absorb_entities` writes to both so existing
    /// template-slot consumers keep working, but higher-level
    /// reasoning now has access to the fuller picture.
    pub belief: crate::belief::BeliefState,
    /// v4.0.29 — goal + task lifecycle state (Codex v4.0.26 roadmap
    /// Phase 2). Rolled forward on every turn AFTER belief
    /// absorption so status reflects the newest intent + belief.
    /// Non-breaking: existing consumers ignore it; Phase 3
    /// `ActionPlanner` will consume it to pick next action.
    pub task: crate::task::TaskState,
    /// v4.0.30 — monotone, **unbounded** turn counter. Codex v4.0.29
    /// review #1 flagged that pre-v4.0.30 turn ids were derived from
    /// `intent_history.len()`, which caps at `MAX_HISTORY = 32`.
    /// After 32 turns the counter plateaued, so belief
    /// `recorded_at_turn` and task `goal_set_at_turn` stopped being
    /// real turn indices — breaking any goal-age / fact-age signal
    /// Phase 3+ will consume.
    ///
    /// This counter increments by 1 at the start of every
    /// `turn_with_trace` call and never resets except on
    /// [`reset`](Self::reset). Saturating-add insures we never panic
    /// even at astronomical turn counts.
    pub turn_counter: usize,
}

/// v4.0.25 — intermediate state captured by
/// [`Conversation::turn_with_trace`]. Each field is the authoritative
/// post-injection snapshot used to drive the final template render.
///
/// Exposed so CLI `--trace` mode surfaces the real runtime path
/// (including `reasoning_chain` and `example` slots populated by
/// `inject_reasoning_chain` / `inject_retrieval_example`) instead of
/// a pre-injection placeholder.
#[derive(Debug, Clone)]
pub struct TurnTrace {
    /// FST parses for each token in the raw input.
    pub parses: Vec<adam_kernel_fst::parser::Analysis>,
    /// Intent after follow-up resolution AND retrieval/reasoning
    /// injection — this is the shape the planner actually saw.
    pub intent_after_injection: Intent,
    /// v4.0.32 — intent that actually went to `plan_response_with_session`
    /// AFTER Phase 4 verifier gating. Identical to
    /// `intent_after_injection` when `verification.supported`; else
    /// evidence slots stripped so the renderer falls through to safe
    /// templates.
    pub intent_after_verification: Intent,
    /// Session slot snapshot taken immediately after entity absorption.
    pub session_snapshot: HashMap<String, String>,
    /// v4.0.27 — belief-state digest taken after entity absorption.
    /// Cheap to clone (six counters) and sufficient for --trace
    /// output. Consumers needing the full picture can clone
    /// `Conversation::belief` directly.
    pub belief_digest: crate::belief::BeliefDigest,
    /// v4.0.27 — the full belief state at the moment of trace
    /// capture. Lets `adam_chat --trace` render the new / updated
    /// facts and any fresh contradictions.
    pub belief_snapshot: crate::belief::BeliefState,
    /// v4.0.29 — task-state digest (five scalars) for quick trace
    /// rendering.
    pub task_digest: crate::task::TaskDigest,
    /// v4.0.29 — full task state at the moment of trace capture.
    pub task_snapshot: crate::task::TaskState,
    /// v4.0.31 — compact action digest (action + expected output +
    /// rationale count). Phase 3 non-breaking: this is for audit
    /// only; the existing template planner still drives rendering.
    pub action_digest: crate::action::ActionDigest,
    /// v4.0.31 — full `ActionPlan` including rationale list and
    /// required inputs, for auditors who want the full picture.
    pub action_plan: crate::action::ActionPlan,
    /// v4.0.32 — Phase 4 pre-render verification report. When
    /// `supported == false` the turn loop has stripped evidence
    /// from `intent_after_verification` to avoid rendering an
    /// answer on top of an unresolved issue.
    pub verification: crate::verifier::VerificationReport,
    /// v4.0.33 — Phase 5 (part 1) epistemic-status band derived
    /// from `(plan, verification, intent, belief)`. Pure classifier
    /// output; not yet consumed by templates (v4.0.34 wires that).
    pub epistemic_status: crate::uncertainty::EpistemicStatus,
    /// Per-step plan trace emitted by `plan_response_with_session`.
    pub plan_trace: Vec<String>,
}

/// Lightweight "kind" summary of an `Intent` — the payload (name /
/// years / city / …) is already held in `slots`, so history doesn't
/// need to copy it. Keeping this separate from `intent::Intent` avoids
/// retaining potentially large `String`s in the session log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntentKind {
    Greeting,
    Farewell,
    Affirmation,
    Negation,
    Thanks,
    Apology,
    AskHowAreYou,
    StatementOfWellbeing,
    AskName,
    StatementOfName,
    AskAge,
    StatementOfAge,
    AskLocation,
    StatementOfLocation,
    AskOccupation,
    StatementOfOccupation,
    AskFamily,
    StatementOfFamily,
    AskWeather,
    StatementOfWeather,
    AskTime,
    Compliment,
    Request,
    WellWishes,
    Insult,
    Unknown,
}

impl From<&Intent> for IntentKind {
    fn from(intent: &Intent) -> Self {
        match intent {
            Intent::Greeting { .. } => Self::Greeting,
            Intent::Farewell => Self::Farewell,
            Intent::Affirmation => Self::Affirmation,
            Intent::Negation => Self::Negation,
            Intent::Thanks => Self::Thanks,
            Intent::Apology => Self::Apology,
            Intent::AskHowAreYou => Self::AskHowAreYou,
            Intent::StatementOfWellbeing => Self::StatementOfWellbeing,
            Intent::AskName => Self::AskName,
            Intent::StatementOfName { .. } => Self::StatementOfName,
            Intent::AskAge => Self::AskAge,
            Intent::StatementOfAge { .. } => Self::StatementOfAge,
            Intent::AskLocation => Self::AskLocation,
            Intent::StatementOfLocation { .. } => Self::StatementOfLocation,
            Intent::AskOccupation => Self::AskOccupation,
            Intent::StatementOfOccupation { .. } => Self::StatementOfOccupation,
            Intent::AskFamily => Self::AskFamily,
            Intent::StatementOfFamily => Self::StatementOfFamily,
            Intent::AskWeather => Self::AskWeather,
            Intent::StatementOfWeather => Self::StatementOfWeather,
            Intent::AskTime => Self::AskTime,
            Intent::Compliment => Self::Compliment,
            Intent::Request => Self::Request,
            Intent::WellWishes => Self::WellWishes,
            Intent::Insult => Self::Insult,
            Intent::Unknown { .. } => Self::Unknown,
        }
    }
}

impl Conversation {
    /// Start a fresh session — no remembered entities and no history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Attach a retrieval index so `Intent::Unknown` can quote a
    /// concrete Kazakh sentence for any recognised noun. Without this,
    /// the fallback behaviour is the v1.1.0 noun-echo path.
    pub fn with_morpheme_index(mut self, index: MorphemeIndex) -> Self {
        self.morpheme_index = Some(index);
        self
    }

    /// Opt into v1.9.0 option-B in-sample composition. When the mode
    /// is `InSampleCitySwap` and the session has a recognised city,
    /// `Intent::Unknown` responses rewrite city mentions in the cited
    /// sample to the user's city (feature-preserving, biography-guarded).
    /// Leave at the default `Verbatim` for the v1.8.5 behaviour.
    pub fn with_compose_mode(mut self, mode: ComposeMode) -> Self {
        self.compose_mode = mode;
        self
    }

    /// Attach rule-derived facts + their supporting extracted facts so
    /// `Intent::Unknown` can cite reasoning chains ("X and Y are both
    /// kinds of Z, therefore related"). `extracted` is used to render
    /// the source_chain into Kazakh evidence phrases; `derived` is the
    /// list of `RuleInferred` facts to match against noun hints.
    ///
    /// Both lists default to empty, in which case dialog behaviour is
    /// bit-identical to v2.6 — retrieval path only, zero reasoning
    /// citations.
    pub fn with_reasoning_chains(
        mut self,
        extracted: Vec<ReasFact>,
        derived: Vec<DerivedFact>,
    ) -> Self {
        self.extracted_facts = extracted;
        self.derived_facts = derived;
        self
    }

    /// v4.0.3 — builder: enable investor-safe reasoning mode.
    /// When enabled, `inject_reasoning_chain` only cites derivations
    /// whose full `source_chain` comes from `data/world_core/*.jsonl`
    /// (every supporting fact human-reviewed). Extracted-pack sources
    /// (wikipedia, textbooks, abai) are refused.
    ///
    /// Default (`false`) preserves v4.0.2 behaviour — the chat cites
    /// any derivation that references the user's topic. Only
    /// `adam_chat --safe` flips this on.
    pub fn with_curated_only_reasoning(mut self, enabled: bool) -> Self {
        self.curated_only_reasoning = enabled;
        self
    }

    /// Run one conversational turn. Parses the input, recognises the
    /// intent, folds any new entities into [`session`](Self::session),
    /// updates [`active_intent`](Self::active_intent) and
    /// [`intent_history`](Self::intent_history), then plans + realises
    /// a response using the merged slot map.
    ///
    /// Deterministic given (current state, input, seed). The state
    /// mutation is the ONLY side-effect.
    pub fn turn(
        &mut self,
        input: &str,
        lexicon: &LexiconV1,
        repo: &TemplateRepository,
        rng_seed: u64,
    ) -> String {
        let (out, _) = self.turn_with_trace(input, lexicon, repo, rng_seed);
        out
    }

    /// v4.0.25 — trace-aware variant of [`turn`]. Identical runtime
    /// behaviour (same state mutation, same output); additionally
    /// returns a [`TurnTrace`] capturing the intermediate state
    /// **after** retrieval + reasoning-chain injection, so consumers
    /// like `adam_chat --trace` can print the real runtime path.
    ///
    /// Codex v4.0.23 re-review #2 flagged that the pre-v4.0.25 trace
    /// mode manually duplicated `turn` but stopped before
    /// `inject_retrieval_example` + `inject_reasoning_chain`, making
    /// trace output materially false for v4.0.20+ features. This
    /// method closes that gap by making trace a first-class output
    /// of the single canonical code path.
    pub fn turn_with_trace(
        &mut self,
        input: &str,
        lexicon: &LexiconV1,
        repo: &TemplateRepository,
        rng_seed: u64,
    ) -> (String, TurnTrace) {
        let parses = crate::parse_input_public(input, lexicon);
        let raw_intent = interpret_text_with_lexicon(input, &parses, Some(lexicon));

        // v1.4.0: follow-up resolution. "ал сіз?" after AskHowAreYou
        // becomes a re-interpretation: "user is asking me the same
        // thing back", which is still AskHowAreYou for planning
        // purposes (planner picks a response without asking back).
        let mut intent = resolve_follow_up(raw_intent, input, self.active_intent);

        // v1.6.5 / v1.7.0 / v1.9.0: inject a retrieval example into
        // Unknown, optionally composing it with session slots per
        // compose_mode. Deterministic throughout — ranker ties break
        // on (pack, sample_id), compose_with_city is a pure function.
        self.inject_retrieval_example(&mut intent, &parses, lexicon);

        // v2.7: inject a rule-derived reasoning chain into Unknown
        // when `derived_facts` are attached and the noun_hint appears
        // in a derivation. Pure function of (derived_facts, noun_hint);
        // no RNG, no side-effects.
        self.inject_reasoning_chain(&mut intent);

        // v4.0.30 — unbounded monotone turn id (Codex v4.0.29 #1 fix).
        // Captured BEFORE absorption so belief.record_user_fact,
        // task.roll_forward, and entity touches all share the same
        // turn number for this turn. Increment after capture so the
        // next turn sees the next integer.
        let turn_id = self.turn_counter;
        self.turn_counter = self.turn_counter.saturating_add(1);
        self.absorb_entities(&intent, turn_id);
        // v4.0.29 — roll task state forward AFTER belief absorption,
        // BEFORE record_intent so the turn id used by task matches
        // the one already used by absorb_entities.
        self.task.roll_forward(&intent, &self.belief, turn_id);
        // v4.0.31 Phase 3 — classify the chosen action for this
        // turn AFTER belief + task are up to date. The result is
        // stored on task.last_action and echoed in TurnTrace for
        // audit. The template planner below still drives the
        // surface form in v4.0.31 — the verifier (Phase 4) is what
        // will actually gate outputs on `ActionPlan`.
        let action_plan = crate::action::ActionPlanner::plan(&intent, &self.belief, &self.task);
        self.task.last_action = Some(action_plan.clone());
        // v4.0.32 Phase 4 — verify the chosen plan against evidence
        // present. If the verifier rejects (e.g. the plan is
        // RunReasoner under a belief contradiction), we strip the
        // injected evidence from a clone of the intent before
        // template planning. The template planner then naturally
        // falls through to `unknown.with_noun` → «ах, X туралы
        // айтасыз ба» or `unknown` → «түсінбедім», neither of which
        // can make a false claim on top of an unresolved conflict.
        // The **original** intent is preserved on TurnTrace so
        // auditors can see what the injection passes actually
        // produced before the gate intervened.
        let verification = crate::verifier::Verifier::verify(&action_plan, &intent, &self.belief);
        // v4.0.33 Phase 5 (part 1) — derive the epistemic status
        // band from `(plan, verification, intent, belief)`. Pure
        // classifier in v4.0.33 — the status is recorded on the
        // trace but templates don't yet consume it. v4.0.34 (Phase 5
        // part 2) adds `unknown.conflicted` / `unknown.tentative`
        // template families and the reply text starts reflecting
        // the status.
        let epistemic_status = crate::uncertainty::UncertaintyPolicy::derive(
            &action_plan,
            &verification,
            &intent,
            &self.belief,
        );
        let intent_for_render = if verification.supported {
            intent.clone()
        } else {
            crate::verifier::strip_evidence(intent.clone())
        };
        self.record_intent(&intent);
        let plan = plan_response_with_session(&intent_for_render, rng_seed, repo, &self.session);
        let output = realise(&plan);
        let trace = TurnTrace {
            parses,
            intent_after_injection: intent,
            intent_after_verification: intent_for_render,
            session_snapshot: self.session.clone(),
            belief_digest: self.belief.digest(),
            belief_snapshot: self.belief.clone(),
            task_digest: self.task.digest(),
            task_snapshot: self.task.clone(),
            action_digest: action_plan.digest(),
            action_plan,
            verification,
            epistemic_status,
            plan_trace: plan.trace.clone(),
        };
        (output, trace)
    }

    /// v2.7: for `Intent::Unknown { noun_hint: Some(n), .. }`, scan
    /// the attached `derived_facts` for any derivation whose subject
    /// OR object root equals `n`. If found, render the source_chain
    /// into a Kazakh prose sentence and fill the `reasoning_chain`
    /// slot. The planner then routes to `unknown.with_derived_chain`.
    ///
    /// **Trust invariant** — every rendered chain carries the marker
    /// stem «байланыс-» ("connect-/relate-") so the user can always
    /// tell a reasoning citation apart from a verbatim corpus quote.
    /// This mirrors v1.9.5's «бейімд-» marker for adapted evidence.
    ///
    /// Deterministic: picks the first matching derivation (sorted
    /// order from the reasoner). No-op when `derived_facts` is empty,
    /// `example` is already set (retrieval wins if it ran), or the
    /// intent isn't Unknown / has no noun_hint.
    /// v4.0.24 — BFS depth from `subject` to `target` through **base**
    /// IsA edges only (`extracted_facts`). Used as a tie-breaker in
    /// `inject_reasoning_chain`: when two equally-scored R1 derivations
    /// both claim «X IsA Y», prefer the one whose Y is closer to X in
    /// the IsA graph (more direct).
    ///
    /// **Critically excludes derived facts** — otherwise the R1
    /// transitive closure itself creates depth-1 IsA edges from
    /// subject to every reachable object, collapsing all tied
    /// candidates to equal depth and defeating the tie-break. The
    /// base-fact-only walk recovers the authentic hop count.
    ///
    /// Returns `usize::MAX` when unreachable so tied callers fall through
    /// to the canonical-triple tie-break. Guarded by `MAX_DEPTH = 8` to
    /// bound pathological cases (graph cycles blocked by `visited`).
    fn isa_chain_depth(&self, subject: &str, target: &str) -> usize {
        const MAX_DEPTH: usize = 8;
        if subject == target {
            return 0;
        }
        let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut frontier: Vec<String> = vec![subject.to_string()];
        visited.insert(subject.to_string());
        for depth in 1..=MAX_DEPTH {
            let mut next: Vec<String> = Vec::new();
            for node in &frontier {
                for f in &self.extracted_facts {
                    if f.predicate == ReasPredicate::IsA && f.subject.root == *node {
                        if f.object.root == target {
                            return depth;
                        }
                        if visited.insert(f.object.root.clone()) {
                            next.push(f.object.root.clone());
                        }
                    }
                }
            }
            if next.is_empty() {
                break;
            }
            frontier = next;
        }
        usize::MAX
    }

    fn inject_reasoning_chain(&self, intent: &mut Intent) {
        if self.derived_facts.is_empty() {
            return;
        }
        if let Intent::Unknown {
            noun_hint: Some(noun),
            reasoning_chain,
            ..
        } = intent
        {
            if reasoning_chain.is_some() {
                return;
            }
            // Find the first derivation involving `noun`. Prefer the
            // subject side so the rendered sentence starts with the
            // user's mentioned word.
            //
            // v3.8.5 — two-pass to make this preference strict. Pre-
            // v3.8.5 did `find(|d| subj == noun || obj == noun)`, which
            // picks whichever side matches first in storage order — so
            // `adam_demo` Part 4 could preview «неміс → халқы» but
            // render «неміс → ара» (a different derivation with
            // object=неміс earlier in the list). Subject-first makes
            // the preview and the rendered text always refer to the
            // same derivation.
            //
            // v4.0.3 — when `curated_only_reasoning` is on, every
            // candidate derivation must pass
            // `derivation_is_fully_curated` (every `source_chain`
            // entry rooted in `world_core/`). Otherwise, no chain
            // fires at all for this noun — the Unknown fallback
            // continues with retrieval-only behaviour. This is the
            // investor-safe chat mode promised by `adam_chat --safe`.
            let passes_safety = |d: &&DerivedFact| -> bool {
                !self.curated_only_reasoning
                    || adam_reasoning::reasoner::derivation_is_fully_curated(d)
            };
            // v4.0.22 — Codex-reranker: score every candidate derivation
            // involving `noun` and pick the highest-scoring one. Replaces
            // the pre-v4.0.22 "first match wins" picker, which Codex
            // flagged for surfacing weak chains («алматы күшке қатысты
            // байланысы бар», «абай — маман») when stronger curated
            // chains were available in the same set.
            let matched = self
                .derived_facts
                .iter()
                .filter(|d| (d.subject.root == *noun || d.object.root == *noun) && passes_safety(d))
                .max_by(|a, b| {
                    score_derivation(a, noun)
                        .cmp(&score_derivation(b, noun))
                        // v4.0.24 — graph-distance tie-break for IsA
                        // derivations. Codex v4.0.23 re-review flagged
                        // that for «математика туралы айтшы» we had 4
                        // fully-curated R1 derivations all scoring 11
                        // (math IsA білім / байлық / мәлімет / қазына).
                        // The canonical-triple fallback picked байлық
                        // (through the metaphorical proverb-chain
                        // «білім IsA байлық») instead of the 1-hop
                        // direct answer білім. Lower IsA-path depth
                        // from subject to object wins — the direct
                        // parent beats transitive ancestors.
                        //
                        // Lower depth sorts GREATER under max_by, so
                        // `Ord::reverse` on the depth comparison makes
                        // shorter paths win.
                        .then_with(|| {
                            if a.predicate == adam_reasoning::Predicate::IsA
                                && b.predicate == adam_reasoning::Predicate::IsA
                            {
                                let da = self.isa_chain_depth(&a.subject.root, &a.object.root);
                                let db = self.isa_chain_depth(&b.subject.root, &b.object.root);
                                da.cmp(&db).reverse()
                            } else {
                                std::cmp::Ordering::Equal
                            }
                        })
                        // Deterministic tie-break by canonical triple key
                        // so runs stay byte-identical.
                        .then_with(|| {
                            (
                                a.subject.root.as_str(),
                                a.predicate.as_str(),
                                a.object.root.as_str(),
                            )
                                .cmp(&(
                                    b.subject.root.as_str(),
                                    b.predicate.as_str(),
                                    b.object.root.as_str(),
                                ))
                                .reverse() // lower triple wins on tie
                        })
                });
            let Some(d) = matched else { return };
            let rendered = render_derivation_as_kazakh(d);
            *reasoning_chain = Some(rendered);
        }
    }

    /// For `Intent::Unknown { noun_hint: Some(n), .. }`, if an index is
    /// attached, call `MorphemeIndex::rank` with every content root
    /// parsed from the input (v1.7.0) and fill the `example` slot with
    /// the top-1 hit's text. Falls back to `search(noun_hint)[0]` when
    /// ranking returns nothing.
    ///
    /// v1.9.0: when [`compose_mode`](Self::compose_mode) is
    /// `InSampleCitySwap` and the session has a recognised city, the
    /// cited text is passed through `compose_with_city` before being
    /// assigned to the `example` slot. If the composer reports a swap,
    /// we use the rewritten version; otherwise we keep the verbatim
    /// quote. Original-sample provenance is preserved on the `Hit` —
    /// the swap is a cosmetic layer on top.
    ///
    /// No-op for every non-Unknown intent, for unknown-without-noun,
    /// and when the index is absent.
    fn inject_retrieval_example(
        &self,
        intent: &mut Intent,
        parses: &[adam_kernel_fst::parser::Analysis],
        lexicon: &LexiconV1,
    ) {
        let Some(index) = self.morpheme_index.as_ref() else {
            return;
        };
        if let Intent::Unknown {
            noun_hint: Some(noun),
            example,
            example_adapted,
            ..
        } = intent
        {
            if example.is_some() {
                return;
            }
            // v1.7.0: rank over all content roots, not just the first.
            let roots = content_roots(parses);
            let root_refs: Vec<&str> = if roots.is_empty() {
                vec![noun.as_str()]
            } else {
                roots.iter().map(|s| s.as_str()).collect()
            };
            let default_cfg;
            let config = match self.rank_config.as_ref() {
                Some(c) => c,
                None => {
                    default_cfg = RankConfig::default();
                    &default_cfg
                }
            };
            let ranked = index.rank(&root_refs, config);
            let candidate_text = ranked
                .first()
                .and_then(|hit| index.sample_text(&hit.sref).map(|s| s.to_string()))
                .or_else(|| {
                    // Fallback: single-morpheme first-posting (v1.6.5 path).
                    index
                        .search(noun)
                        .first()
                        .and_then(|s| index.sample_text(s).map(|t| t.to_string()))
                });
            let Some(text) = candidate_text else {
                return;
            };
            let (composed_text, was_adapted) = self.maybe_compose(&text, lexicon);
            *example = Some(composed_text);
            *example_adapted = was_adapted;
        }
    }

    /// v1.9.0 option-B step. If [`compose_mode`](Self::compose_mode) is
    /// `InSampleCitySwap` and the session has a recognised city,
    /// rewrite city mentions in `text` to the user's city. Returns the
    /// (possibly rewritten) text and a flag indicating whether any
    /// swap actually happened. v1.9.5: the flag propagates to
    /// `Intent::Unknown.example_adapted` so the planner can route to
    /// the `unknown.with_adapted_evidence` family and the user sees an
    /// explicit "this quote was adapted" framing.
    ///
    /// All safety guards (biography year, known-place list) live in
    /// [`compose_with_city`].
    fn maybe_compose(&self, text: &str, lexicon: &LexiconV1) -> (String, bool) {
        if !matches!(self.compose_mode, ComposeMode::InSampleCitySwap) {
            return (text.to_string(), false);
        }
        let Some(user_city) = self.session.get("city") else {
            return (text.to_string(), false);
        };
        let composition = compose_with_city(text, user_city, lexicon);
        if composition.was_changed() {
            (composition.output, true)
        } else {
            (composition.original, false)
        }
    }

    /// Extract persistent entities from an intent and push them into
    /// the running session AND (v4.0.27) the structured belief state.
    ///
    /// The dual write is intentional: legacy template-slot consumers
    /// keep reading from `self.session`; the new belief-aware paths
    /// (contradiction detection, provenance tracing, future
    /// verifier / action planner) read from `self.belief`. Keeping
    /// them in lock-step avoids a disruptive migration.
    ///
    /// v4.0.30 — `turn_id` is threaded from the caller instead of
    /// derived from `intent_history.len()`. Codex v4.0.29 review #1:
    /// the old derivation plateaued at `MAX_HISTORY = 32`, breaking
    /// `recorded_at_turn` as a real turn index in long sessions.
    pub(crate) fn absorb_entities(&mut self, intent: &Intent, turn_id: usize) {
        use crate::belief::{EntityKind, USER_SELF_KEY};
        match intent {
            Intent::StatementOfName { name } => {
                self.session.insert("name".into(), name.clone());
                self.belief
                    .touch_entity(USER_SELF_KEY, EntityKind::User, name, turn_id);
                self.belief
                    .record_user_fact(USER_SELF_KEY, "name", name, turn_id);
            }
            Intent::StatementOfAge { years: Some(years) } => {
                self.session.insert("age".into(), years.to_string());
                self.belief
                    .touch_entity(USER_SELF_KEY, EntityKind::User, "__self__", turn_id);
                self.belief
                    .record_user_fact(USER_SELF_KEY, "age", &years.to_string(), turn_id);
            }
            Intent::StatementOfLocation { city: Some(city) } => {
                self.session.insert("city".into(), city.clone());
                self.belief
                    .touch_entity(city, EntityKind::Place, city, turn_id);
                self.belief
                    .record_user_fact(USER_SELF_KEY, "city", city, turn_id);
            }
            Intent::StatementOfOccupation {
                occupation: Some(occupation),
            } => {
                self.session.insert("occupation".into(), occupation.clone());
                self.belief
                    .touch_entity(occupation, EntityKind::Occupation, occupation, turn_id);
                self.belief
                    .record_user_fact(USER_SELF_KEY, "occupation", occupation, turn_id);
            }
            _ => {}
        }
    }

    /// Update `active_intent` + push to `intent_history` with a bounded
    /// capacity. Called on every turn by [`turn`](Self::turn).
    fn record_intent(&mut self, intent: &Intent) {
        let kind = IntentKind::from(intent);
        self.active_intent = Some(kind);
        if self.intent_history.len() >= MAX_HISTORY {
            self.intent_history.remove(0);
        }
        self.intent_history.push(kind);
    }

    /// Clear all conversation state — slots, active intent, history,
    /// (v4.0.27) the belief state, (v4.0.29) the task state, and
    /// (v4.0.30) the turn counter.
    pub fn reset(&mut self) {
        self.session.clear();
        self.active_intent = None;
        self.intent_history.clear();
        self.belief = crate::belief::BeliefState::new();
        self.task = crate::task::TaskState::new();
        self.turn_counter = 0;
    }
}

/// v1.4.0 follow-up resolution. Some Kazakh utterances are meaningless
/// out of context but carry a pointer back to the previous turn:
///
///   "ал сіз?"     — "and you?"       (flip subject of last Ask-)
///   "ал сен?"     — informal version
///
/// When we detect a bare reflective-query as `Unknown` (or any
/// weak-intent), and the PREVIOUS turn was an Ask- intent, we re-tag
/// the current turn as the same Ask kind, so the planner picks a
/// matching response template.
/// Render a [`DerivedFact`] as a Kazakh prose sentence suitable for the
/// `unknown.with_derived_chain` template family. Every output contains
/// the marker stem «байланыс-» so consumers can distinguish a reasoning
/// citation from a verbatim corpus quote at the textual level alone
/// (trust invariant; mirrors v1.9.5's «бейімд-» marker for adapted
/// evidence).
///
/// v2.7 only handles `Predicate::RelatedTo` (the sole rule-derived
/// predicate currently emitted). Other predicates produce a generic
/// fallback that still contains the marker. Future releases will add
/// predicate-specific renderings as reasoner output grows.

/// v4.0.22 — composite score for ranking candidate derivations. Higher
/// scores pick first. Implements Codex v4.0.19 review recommendation
/// #3 — prefer curated+short+taxonomically-direct chains, penalize
/// text-only and R5/R2 shared-target fan-out.
///
/// The score ranges roughly 0..10. Terms:
///
///   + 4 if every `source_chain.pack` starts with `world_core/` (fully
///       curated — strongest trust signal)
///   + 1 if at least one source is curated but not all (mixed)
///   - 2 if no source is curated (text-only — last resort)
///
///   + 2 if `source_chain.len() <= 1`
///   + 1 if `source_chain.len() == 2`
///   (+ 0 longer — deep chains drift semantically)
///
///   + rule_weight — Codex ordering:
///       R1 IsA-transitivity, R10 InDomain-inheritance: 3 (clean taxonomic)
///       R2 Has-inheritance: 2
///       R3 Has-via-PartOf, R6 LivesIn-via-PartOf, R7 GoesTo-via-PartOf,
///       R9 PartOf-transitivity, R8 After-transitivity: 2 (mereological/temporal)
///       R5 shared-IsA, R11 shared-InDomain: 1 (weakest — combinatorial fan-out)
///       unknown rule: 1 (defensive default)
///
///   + 1 if subject matches `noun` (preserves pre-v4.0.22 subject-first preference)
///
/// Tie-break is by canonical triple (see `inject_reasoning_chain`).
fn score_derivation(d: &DerivedFact, noun: &str) -> i32 {
    let mut score: i32 = 0;

    // 1. Trust (source_chain provenance).
    if d.source_chain.is_empty() {
        score -= 2;
    } else {
        let wc_count = d
            .source_chain
            .iter()
            .filter(|s| s.pack.starts_with("world_core/"))
            .count();
        if wc_count == d.source_chain.len() {
            score += 4; // fully curated
        } else if wc_count > 0 {
            score += 1; // mixed
        } else {
            score -= 2; // text-only
        }
    }

    // 2. Chain length.
    match d.source_chain.len() {
        0 | 1 => score += 2,
        2 => score += 1,
        _ => {}
    }

    // 3. Rule weight.
    score += match d.rule_id.as_str() {
        "R1_is_a_transitivity" | "R10_in_domain_inheritance" => 3,
        "R2_has_inheritance"
        | "R3_has_inheritance_via_part_of"
        | "R6_lives_in_via_part_of"
        | "R7_goes_to_via_part_of"
        | "R8_after_transitivity"
        | "R9_part_of_transitivity" => 2,
        "R5_shared_is_a_target" | "R11_in_domain_shared_target" => 1,
        _ => 1,
    };

    // 4. Subject-side preference.
    if d.subject.root == noun {
        score += 1;
    }

    // v4.0.24 — 5. Predicate preference. For "tell me about X" dialog
    // queries, an IsA answer ("X is a Y") is the most semantically
    // direct form. Codex v4.0.23 re-review flagged that the pre-v4.0.24
    // reranker tied multiple curated R1 + R10 candidates on the same
    // noun and the canonical-triple tie-break surfaced weaker picks
    // (InDomain over IsA for `немере`). This +2 boost makes IsA the
    // default winner when all other trust/length/rule terms are equal.
    if d.predicate == adam_reasoning::Predicate::IsA {
        score += 2;
    }

    score
}

fn render_derivation_as_kazakh(d: &DerivedFact) -> String {
    // Every arm MUST include the marker stem «байланыс-» (or one of its
    // forms) so downstream consumers can distinguish a reasoning
    // citation from a verbatim corpus quote at the textual level alone.
    // v2.7 handled IsA + RelatedTo + generic fallback. v2.8 adds
    // predicate-specific renderings for Has / GoesTo / LivesIn /
    // PartOf so every derived variant produces idiomatic Kazakh.
    //
    // v3.8.5: all case suffixes synthesised via FST (no dash-
    // concatenation) — the previous `"{}-ға"` template produced
    // morphologically-invalid surfaces like `атау-ға` / `өсімдік-ға`
    // which broke vowel harmony and the no-invalid-form invariant.
    match d.predicate {
        ReasPredicate::RelatedTo => {
            // "X пен Y бір-біріне байланысты" — shared-type relation
            format!(
                "{} пен {} бір-біріне байланысты екен",
                d.subject.root, d.object.root
            )
        }
        ReasPredicate::IsA => {
            // Transitivity-derived IsA — the reasoner chained.
            format!(
                "қорытынды: {} — {} (байланысты ой-тізбек арқылы)",
                d.subject.root, d.object.root
            )
        }
        ReasPredicate::Has => {
            // Inheritance-derived Has via R2.
            format!(
                "ой-тізбек: {} {} қатысты байланысы бар (иелік мұрагерлік)",
                d.subject.root,
                inflect(&d.object.root, Case::Dative)
            )
        }
        ReasPredicate::GoesTo => {
            format!(
                "{} {} жағына байланысты қозғалыс ретінде шықты",
                d.subject.root, d.object.root
            )
        }
        ReasPredicate::LivesIn => {
            format!(
                "{} {} орнымен байланысты мекендеу қорытындысы бар",
                d.subject.root, d.object.root
            )
        }
        ReasPredicate::PartOf => {
            // v3.8.5 — use dative instead of genitive to sidestep the
            // pre-v3.9 FST bug where genitive-after-vowel produces
            // `қаладың` instead of `қаланың` (the `{D}{I}ң` template's
            // {D} archiphoneme lacks the "after-vowel → н" rule that
            // genitive requires but ablative does not).
            format!(
                "{} {} құрамына байланысты бір бөлігі ретінде шықты",
                d.subject.root,
                inflect(&d.object.root, Case::Dative)
            )
        }
        // v3.5.0 additions. Each keeps the «байланыс-» marker per the
        // trust-stack invariant (test-enforced in v2.7+).
        ReasPredicate::Causes => {
            // v3.8.5 — same FST genitive bug → avoid genitive here too.
            format!(
                "{} {} себеп болатыны байланысты ой-тізбек арқылы шықты",
                d.subject.root,
                inflect(&d.object.root, Case::Dative)
            )
        }
        ReasPredicate::After => {
            format!(
                "{} {} кейін болатындығы байланысты уақыт-тізбек арқылы шықты",
                d.subject.root,
                inflect(&d.object.root, Case::Ablative)
            )
        }
        ReasPredicate::HasQuantity => {
            format!(
                "{} {} байланысты санды қатынас ретінде шықты",
                d.subject.root,
                inflect(&d.object.root, Case::Instrumental)
            )
        }
        ReasPredicate::DoesTo => {
            format!(
                "{} {} үстінде байланысты әрекет иесі ретінде шықты",
                d.subject.root, d.object.root
            )
        }
        ReasPredicate::InDomain => {
            format!(
                "{} {} байланысты мүше ретінде шықты",
                d.subject.root,
                inflect(&d.object.root, Case::Dative)
            )
        }
    }
}

/// v3.8.5 — synthesise a noun in the requested grammatical case via FST.
/// Replaces the pre-v3.8.5 practice of manually concatenating a hyphen +
/// invariant suffix (`{root}-ға` etc.), which produced surface forms
/// like `атау-ға` that violate Kazakh vowel harmony.
fn inflect(root: &str, case: Case) -> String {
    let mut features = NounFeatures::default();
    features.case = Some(case);
    synthesise_noun(root, features)
}

fn resolve_follow_up(raw: Intent, input: &str, active: Option<IntentKind>) -> Intent {
    let normalised: String = input.to_lowercase();
    let is_reflective = normalised.trim() == "ал сіз"
        || normalised.trim() == "ал сіз?"
        || normalised.trim() == "ал сен"
        || normalised.trim() == "ал сен?"
        || normalised.trim() == "сіз ше"
        || normalised.trim() == "сен ше";
    if !is_reflective {
        return raw;
    }
    // Only reroute when the raw intent is weak (Unknown / Affirmation)
    // — never override a clearly-recognised strong intent.
    if !matches!(
        raw,
        Intent::Unknown { .. } | Intent::Affirmation | Intent::Negation
    ) {
        return raw;
    }
    match active {
        Some(IntentKind::AskHowAreYou) | Some(IntentKind::StatementOfWellbeing) => {
            Intent::AskHowAreYou
        }
        Some(IntentKind::AskName) | Some(IntentKind::StatementOfName) => Intent::AskName,
        Some(IntentKind::AskAge) | Some(IntentKind::StatementOfAge) => Intent::AskAge,
        Some(IntentKind::AskLocation) | Some(IntentKind::StatementOfLocation) => {
            Intent::AskLocation
        }
        Some(IntentKind::AskOccupation) | Some(IntentKind::StatementOfOccupation) => {
            Intent::AskOccupation
        }
        _ => raw,
    }
}
