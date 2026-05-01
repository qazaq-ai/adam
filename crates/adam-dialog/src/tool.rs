//! Layer 3.11 — controlled tool layer, introduced in v4.0.37.
//!
//! Codex v4.0.26 roadmap "Phase 6". Pre-v4.0.37 the dialog reached
//! into [`crate::belief::BeliefState`], `extracted_facts`,
//! [`adam_retrieval::MorphemeIndex`], and `derived_facts` directly
//! from `inject_retrieval_example` / `inject_reasoning_chain` /
//! `ActionPlanner`. Each lookup was a one-off function call —
//! convenient for the v4.0.31 substrate but invisible to the trace
//! and impossible for the planner to *intend* a tool call as a
//! distinct action.
//!
//! Phase 6 introduces a uniform tool interface so:
//!
//! 1. Every internal lookup goes through one named, traced channel.
//! 2. The `ActionPlanner` (Phase 7+) can return a list of `ToolCall`s
//!    to execute, instead of inlining the lookup at the call site.
//! 3. New tools (calculator, calendar, external API, future learned
//!    rerankers) plug in without touching every consumer.
//!
//! **Substrate (v4.0.37) → fully wired (v4.0.38+).** Phase 6 was
//! split across two releases following the same pattern as Phases
//! 1 / 2 / 5: substrate first, behavior second, each Codex-
//! reviewable independently. As of v4.0.38+ all four tools are
//! live and `Conversation::turn_with_trace` **does** auto-dispatch
//! the planned tool sequence — `tool_calls: Vec<ToolResult>` on
//! `TurnTrace` carries every executed call. `SearchBelief`,
//! `SearchGraph`, `SearchRetrieval`, and `RunLocalReasoner` all
//! return real results; the v4.0.37 stub paths have been removed.
//!
//! **Subsequent expansions:** v4.13.0 added DialogContext-aware
//! list-class ranking inside `Tool::dispatch(SearchGraph)`;
//! v4.14.5 added a domain-aware tiebreaker that consults
//! `current_domain` + `DomainIndex` when both are attached;
//! v4.17.5 reordered `list_intent_rank` before overlap so
//! synonym-aware queries (`аймақ ↔ облыс`) match correctly;
//! v4.18.0 added `previous_grounded_fact` to `ToolContext` for
//! list-anaphor cross-turn tracking.

use crate::belief::{BeliefState, FactStatus};
use adam_reasoning::ontology::{
    find_support_fact, validate_derived_fact_with_supports, validate_fact,
};
use adam_reasoning::reasoner::DerivedFact;
use adam_reasoning::{ConfidenceKind, Fact as ReasFact, FactSource, Predicate as ReasPredicate};
use adam_retrieval::MorphemeIndex;

/// One callable internal tool. Each variant carries the inputs the
/// dispatcher needs; results come back through [`ToolResult`].
///
/// Naming follows the Codex v4.0.26 roadmap proposal verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolCall {
    /// Look up active belief facts by subject (e.g. user name,
    /// city, occupation).
    ///
    /// **v4.1.5** — accepts an optional predicate filter so the
    /// `ActionPlanner::belief_direct_answer` lookup can route through
    /// the tool layer instead of bypassing it. Two output shapes:
    /// - `predicate: None`  → audit-friendly: every active fact for
    ///   `subject` rendered as `"{subject} {predicate} {object}"`
    ///   (preserves the v4.0.37 contract).
    /// - `predicate: Some(p)` → typed-lookup-friendly: 0 or 1
    ///   findings (single-active-fact invariant, v4.0.28), each
    ///   finding is the **object string only** so the caller can
    ///   use it as a slot value without re-parsing.
    SearchBelief {
        subject: String,
        predicate: Option<String>,
    },
    /// Look up extracted (base) facts by subject and optional
    /// predicate. Proxies for "search the lexical graph" — the
    /// graph index itself isn't yet exposed, so we filter the flat
    /// `extracted_facts` Vec instead. v4.0.37 — fully implemented.
    SearchGraph {
        subject: String,
        predicate: Option<String>,
    },
    /// Corpus retrieval via [`adam_retrieval::MorphemeIndex::rank`].
    /// v4.0.38 — fully implemented; takes the morpheme list the
    /// caller would have built for `inject_retrieval_example` and
    /// returns the top-k surface texts as `findings`. When no
    /// `MorphemeIndex` is attached to the context, returns
    /// `success=false` with an explicit reason.
    SearchRetrieval { morphemes: Vec<String> },
    /// Find an existing rule-derived chain involving the topic.
    ///
    /// **v4.1.2** — full picker. Scans `derived_facts` for matches
    /// (subject or object), filters by `curated_only` (when `true`,
    /// only fully-curated derivations pass — same gate as
    /// `Conversation::curated_only_reasoning`), scores each
    /// candidate via `score_derivation`, breaks ties on IsA-chain
    /// depth (closer parent wins) then on canonical-triple ordering
    /// (deterministic), renders the top match via
    /// `render_derivation_as_kazakh`, and returns it as a single
    /// finding. Drives `Conversation::inject_reasoning_chain` —
    /// reply-text identical to the pre-v4.1.2 direct-scan path.
    ///
    /// Pre-v4.1.2 this tool returned the top 3 raw triples for
    /// audit only; the `inject_reasoning_chain` helper did its own
    /// pick + render. The two paths could disagree under tie-breaks
    /// because the audit dispatch had no IsA-depth knowledge. v4.1.2
    /// makes the tool dispatch authoritative for reasoning-chain
    /// resolution.
    RunLocalReasoner {
        topic: String,
        /// **v4.1.2** — when `true`, only derivations whose every
        /// `source_chain` entry is rooted in `world_core/` qualify
        /// (mirrors `derivation_is_fully_curated`). When `false`,
        /// any derivation involving the topic is in scope.
        curated_only: bool,
    },
}

/// What the tool returned. `findings` are short opaque strings the
/// caller can render or further process; `trace` is a per-call audit
/// log mirroring `ResponsePlan.trace`. `success` is the binary
/// outcome — useful for the `ActionPlanner` to decide whether to
/// fall back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolResult {
    pub call: ToolCall,
    pub success: bool,
    pub findings: Vec<String>,
    /// Structured evidence backing `findings`. This stays machine-
    /// readable so higher layers can audit not just "some string was
    /// found", but which typed fact / derivation / retrieval sample
    /// justified the user-facing answer.
    pub evidence: Vec<ToolEvidence>,
    pub trace: Vec<String>,
}

impl ToolResult {
    fn ok(
        call: ToolCall,
        findings: Vec<String>,
        evidence: Vec<ToolEvidence>,
        trace: Vec<String>,
    ) -> Self {
        Self {
            call,
            success: true,
            findings,
            evidence,
            trace,
        }
    }

    /// Dispatch ran successfully, but the tool found nothing —
    /// e.g. `SearchBelief` on a subject with no Active facts, or
    /// `RunLocalReasoner` on a topic that has no derivations.
    /// Distinct from [`unsupported`](Self::unsupported), which means
    /// the tool *couldn't* run for lack of context.
    fn empty(call: ToolCall, reason: &str) -> Self {
        Self {
            call,
            success: false,
            findings: Vec::new(),
            evidence: Vec::new(),
            trace: vec![reason.to_string()],
        }
    }

    /// Dispatch couldn't run — the `ToolContext` lacks the store
    /// the tool needs (e.g. `SearchRetrieval` with no
    /// `MorphemeIndex`). Distinct from [`empty`](Self::empty),
    /// which means the tool ran fine and produced no findings.
    fn unsupported(call: ToolCall, why: &str) -> Self {
        Self {
            call,
            success: false,
            findings: Vec::new(),
            evidence: Vec::new(),
            trace: vec![why.to_string()],
        }
    }
}

/// Machine-readable typed evidence emitted by tools. This is the
/// audit substrate for response faithfulness checks: the dialog layer
/// can now verify that a surfaced grounded fact came from the graph,
/// that a quote came from retrieval, and that a reasoning answer came
/// from a rule-derived fact with real rule metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolEvidence {
    BeliefFact {
        subject: String,
        predicate: String,
        object: String,
    },
    GraphFact {
        subject: String,
        predicate: ReasPredicate,
        object: String,
        confidence: ConfidenceKind,
        rendered: String,
    },
    RetrievalSample {
        text: String,
    },
    DerivedFact {
        subject: String,
        predicate: ReasPredicate,
        object: String,
        rule_id: String,
        confidence: ConfidenceKind,
        rendered: String,
        support_chain: Vec<SupportFactEvidence>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportFactEvidence {
    pub subject: String,
    pub predicate: ReasPredicate,
    pub object: String,
    pub confidence: ConfidenceKind,
    pub source: FactSource,
}

/// v4.0.38 — bundle of read-only references the dispatcher needs.
/// Adding a tool that needs a new store (e.g. retrieval ranker
/// config, future calculator state) means adding a field here, not
/// changing the `Tool::dispatch` signature.
pub struct ToolContext<'a> {
    pub belief: &'a BeliefState,
    pub extracted: &'a [ReasFact],
    pub derived: &'a [DerivedFact],
    pub retrieval: Option<&'a MorphemeIndex>,
    /// **v4.1.1** — caller-supplied retrieval ranker config. `None`
    /// means "use [`adam_retrieval::RankConfig::default`]". Threaded
    /// through `ToolContext` rather than the `ToolCall::SearchRetrieval`
    /// payload because `RankConfig` is a sizeable struct (weights +
    /// per-pack purity prior map) and would otherwise be cloned into
    /// every tool call. Required for `inject_retrieval_example` to
    /// route through `Tool::dispatch(SearchRetrieval)` while still
    /// honouring `Conversation::rank_config` (the per-conversation
    /// override).
    pub rank_config: Option<&'a adam_retrieval::RankConfig>,
    /// **v4.4.11** — user's raw input string. When set, `SearchGraph`
    /// rerankings boost facts whose `raw_text` shares more content
    /// morphemes with the input than competitor facts on the same
    /// subject. Closes the v4.4.10 carry-forward where listing-style
    /// questions like «Қазақстанда қандай өзендер мен көлдер бар?»
    /// retrieved the most-central `қазақстан IsA ел` fact rather than
    /// the specific `қазақстан related_to өзендер тізімі` /
    /// `қазақстан related_to көлдер тізімі` list-summary facts. The
    /// overlap-boost runs as a primary sort tier; the v4.0.x
    /// predicate-rank tier (IsA → LivesIn → HasQuantity → …) becomes
    /// the tie-breaker when two facts share the same overlap count.
    /// `None` (default) preserves pre-v4.4.11 behaviour bit-for-bit.
    pub query_input: Option<&'a str>,
    /// **v4.14.5** — the World-Core domain currently being
    /// discussed (computed by `DialogContext::recompute_domain`
    /// from the last few turns). When set, `SearchGraph` reranking
    /// prefers candidates whose subject's primary domain matches —
    /// useful for cross-domain ambiguous topics like `тіл`
    /// (linguistics OR biology body part), `көз` (biology OR
    /// geography spring), `сай` (botany OR geography).
    /// Strictly additive — runs as a tiebreaker AFTER overlap +
    /// priority cascades, so existing single-domain queries are
    /// bit-identical pre/post-v4.14.5. `None` (default) preserves
    /// pre-v4.14.5 behaviour exactly.
    pub current_domain: Option<&'a str>,
    /// **v4.14.5** — companion to `current_domain`: the
    /// `DomainIndex` used to look up each candidate fact's primary
    /// domain. The lookup is per-fact via `subject.root`; tying it
    /// to a per-Conversation index keeps the lookup O(1) without
    /// re-walking world_core on every retrieval call.
    pub domain_index: Option<&'a crate::domain_index::DomainIndex>,
    /// **v4.18.0** — previous turn's rendered grounded_fact text,
    /// when one was surfaced. Used by `list_intent_rank` as a
    /// fallback context when the current query has no list-class
    /// token. Lets «Оларды тізімдей аласыз ба?» (after a turn
    /// surfacing the regions count) infer that «облыс» is the
    /// implied list class. `None` when the previous turn produced
    /// no grounded fact, or when this is the first turn.
    pub previous_grounded_fact: Option<&'a str>,
}

/// Pure-function dispatcher. Reads `ToolContext` references, never
/// mutates them. Future tools that need write access must take a
/// `&mut` field explicitly.
pub struct Tool;

impl Tool {
    /// Execute a single `ToolCall` against the bundled context.
    pub fn dispatch(call: ToolCall, ctx: &ToolContext) -> ToolResult {
        let mut trace = Vec::new();
        trace.push(format!("tool: dispatch {call:?}"));
        match &call {
            ToolCall::SearchBelief { subject, predicate } => {
                let active: Vec<&crate::belief::BeliefFact> = ctx
                    .belief
                    .facts
                    .iter()
                    .filter(|f| f.subject == *subject && f.status == FactStatus::Active)
                    .filter(|f| match predicate {
                        Some(p) => f.predicate == *p,
                        None => true,
                    })
                    .collect();
                trace.push(format!(
                    "search_belief: subject={subject} predicate={:?} active_matches={}",
                    predicate,
                    active.len()
                ));
                if active.is_empty() {
                    return ToolResult::empty(call, "search_belief: no active facts");
                }
                let evidence: Vec<ToolEvidence> = active
                    .iter()
                    .map(|f| ToolEvidence::BeliefFact {
                        subject: f.subject.clone(),
                        predicate: f.predicate.clone(),
                        object: f.object.clone(),
                    })
                    .collect();
                let findings: Vec<String> = match predicate {
                    Some(_) => active.iter().map(|f| f.object.clone()).collect(),
                    None => active
                        .iter()
                        .map(|f| format!("{} {} {}", f.subject, f.predicate, f.object))
                        .collect(),
                };
                ToolResult::ok(call, findings, evidence, trace)
            }
            ToolCall::SearchGraph { subject, predicate } => {
                // **v4.11.7** — case-insensitive subject lookup. The
                // upstream noun-hint extractor occasionally returns a
                // title-cased proper-noun form (`Ұлытау` via
                // `normalize_proper_noun` when the FST has no lemma
                // for the surface), but world_core stores subjects
                // lowercase. Pre-v4.11.7 the direct equality
                // `f.subject.root == *subject` failed on case
                // mismatch and the planner fell to
                // `unknown.tentative` ("Бәлкім, Ұлытау туралы
                // айтасыз ба") even though `subject="ұлытау"` facts
                // existed. Closes the live-REPL gap on Ұлытау /
                // Жетісу-style bare proper-noun queries.
                let subject_lower = subject.to_lowercase();
                let mut matches: Vec<&ReasFact> = ctx
                    .extracted
                    .iter()
                    .filter(|f| f.subject.root.to_lowercase() == subject_lower)
                    .filter(|f| match predicate {
                        Some(p) => predicate_name_matches(f.predicate, p),
                        None => true,
                    })
                    .filter(|f| f.confidence == ConfidenceKind::HumanApproved)
                    .collect();
                // v4.4.11 — input-overlap reranker. When `ctx.query_input`
                // is set, score each fact by how many content tokens
                // from the user's question appear as substrings of
                // `fact.raw_text`. Higher overlap wins. Ties fall
                // through to the v4.0.x predicate-rank tier (IsA → …).
                // `None` (default) preserves the pre-v4.4.11 sort
                // exactly. Closes the v4.4.10 carry-forward where
                // «Қазақстанда қандай өзендер мен көлдер бар?» picked
                // the most-central `қазақстан IsA ел` fact instead of
                // the specific `қазақстан related_to өзендер тізімі`
                // list-summary fact.
                let query_tokens: Vec<String> = ctx
                    .query_input
                    .map(|raw| query_content_tokens(raw, subject))
                    .unwrap_or_default();
                // **v4.17.5** — list-intent boost. When the user's
                // input contains list-request tokens (`тізім /
                // тізімде / тізімдей / атаулары / атап шық /
                // барлық атаулары`), prefer facts whose object
                // root contains «тізім» (i.e. the curated
                // list-summary facts like `қазақстан related_to
                // облыстар тізімі`). Strictly additive: fires
                // only when the query carries an explicit list
                // request, otherwise the v4.4.11 cascade is
                // unchanged. Live REPL 2026-05-01 turn 14-15:
                // the user asked for the regions list; the
                // priority cascade preferred `IsA мемлекет` over
                // `related_to облыстар тізімі`. List-intent
                // boost flips that.
                let query_lower = ctx.query_input.unwrap_or("").to_lowercase();
                let has_list_intent = query_lower.contains("тізім")
                    || query_lower.contains("атаулары")
                    || query_lower.contains("атап шық")
                    || query_lower.contains("атап өт")
                    || query_lower.contains("барлық атау");
                // **v4.17.5** — synonym table for list-intent
                // disambiguation. Hoisted out of the sort closure
                // so debug code can also reference it.
                const LIST_TYPE_SYNONYMS: &[(&str, &str)] =
                    &[("аймақ", "облыс"), ("аумақ", "облыс"), ("гора", "тау")];
                matches.sort_by(|a, b| {
                    let overlap_a = if query_tokens.is_empty() {
                        0
                    } else {
                        fact_overlap_score(a, &query_tokens)
                    };
                    let overlap_b = if query_tokens.is_empty() {
                        0
                    } else {
                        fact_overlap_score(b, &query_tokens)
                    };
                    // **v4.14.5** — domain-match tiebreaker. After
                    // overlap + priority cascades, prefer candidates
                    // whose subject's primary domain matches the
                    // currently-discussed domain (computed by
                    // `DialogContext::recompute_domain`). Useful for
                    // cross-domain ambiguous topics. Strictly
                    // additive — only fires when both `current_domain`
                    // AND `domain_index` are attached to the
                    // ToolContext (i.e. when running inside a
                    // domain-aware Conversation v4.14.0+). Returns
                    // 0 for a match, 1 for non-match, so
                    // `match_score_a.cmp(&match_score_b)` ascending
                    // puts matches first.
                    let domain_match = |fact: &ReasFact| -> usize {
                        match (ctx.current_domain, ctx.domain_index) {
                            (Some(curr), Some(idx)) => {
                                match idx.lookup_domain(&fact.subject.root) {
                                    Some(d) if d == curr => 0,
                                    _ => 1,
                                }
                            }
                            _ => 0, // no signal, treat all as equal
                        }
                    };
                    // **v4.17.5** — list-intent boost: when the
                    // query carries an explicit list request,
                    // facts whose object root contains «тізім»
                    // (i.e. list-summary facts) win the tier
                    // BEFORE the v4.0.x predicate-rank cascade.
                    // Strictly additive — fires only when
                    // has_list_intent is true.
                    //
                    // **Synonym-aware sub-rank**: among list-
                    // summary candidates, prefer the one whose
                    // object root overlaps tokens from the
                    // user's input. Hand-coded synonyms cover
                    // common Kazakh pairs where the input uses
                    // one form and the curated fact uses the
                    // other (e.g. «аймақ» in input, «облыс» in
                    // the curated regions list).
                    let list_intent_rank = |fact: &ReasFact| -> i32 {
                        // 0 = best (list-summary AND overlap),
                        // 1 = list-summary without overlap,
                        // 2 = non-list (default).
                        if !has_list_intent {
                            return 2;
                        }
                        let object_lower = fact.object.root.to_lowercase();
                        if !object_lower.contains("тізім") {
                            return 2;
                        }
                        // Direct token overlap (input → object root).
                        // Exclude generic list-marker prefixes (`тізі`,
                        // `атау`) — they appear in every list-summary
                        // object and don't disambiguate. Use 4-char
                        // prefix match like `fact_overlap_score` so
                        // inflected forms still hit.
                        let direct_overlap = query_tokens.iter().any(|t| {
                            let prefix_4: String = t.chars().take(4).collect();
                            if prefix_4 == "тізі" || prefix_4 == "атау" || prefix_4 == "барл"
                            {
                                return false;
                            }
                            object_lower.contains(&prefix_4)
                        });
                        // Synonym-aware overlap (e.g. аймақ ↔ облыс
                        // for region queries).
                        // **v4.18.0** — also scan the previous
                        // grounded fact's text for list-class
                        // hints. When the current query has no
                        // explicit list-class but the prior turn
                        // surfaced «Қазақстанның аймақтары — 17
                        // облыс ...», the «облыс» token in that
                        // fact tells us the implicit referent is
                        // regions, not landmarks. Strictly
                        // additive — fires only when
                        // previous_grounded_fact is attached.
                        let prev_fact_lower = ctx
                            .previous_grounded_fact
                            .map(|s| s.to_lowercase())
                            .unwrap_or_default();
                        let synonym_overlap =
                            LIST_TYPE_SYNONYMS.iter().any(|(input_tok, obj_tok)| {
                                (query_lower.contains(input_tok)
                                    || prev_fact_lower.contains(input_tok))
                                    && object_lower.contains(obj_tok)
                            });
                        // Direct prior-fact list-class match: if
                        // the previous fact mentions a list-class
                        // word that's also in this candidate's
                        // object root, prefer this candidate.
                        let prev_class_match = if prev_fact_lower.is_empty() {
                            false
                        } else {
                            const CLASS_TOKENS: &[&str] =
                                &["облыс", "өзен", "көл", "тау", "шөл", "көрікті жер"];
                            CLASS_TOKENS.iter().any(|class| {
                                prev_fact_lower.contains(class) && object_lower.contains(class)
                            })
                        };
                        if direct_overlap || synonym_overlap || prev_class_match {
                            0
                        } else {
                            1
                        }
                    };
                    // **v4.17.5** — when has_list_intent fires,
                    // list_intent_rank takes precedence over the
                    // v4.4.11 overlap reranker. Reason: spurious
                    // overlap can pollute the bigram-style match
                    // (e.g. «атау» 4-char prefix accidentally
                    // matches «АлАТАУы» in the landmarks fact's
                    // raw_text). When the user explicitly asks
                    // for a list, the list-summary fact whose
                    // OBJECT root matches the list type should
                    // win regardless of accidental raw-text
                    // overlap. Outside list-intent mode all facts
                    // get rank=2 → no effect, overlap dominates
                    // as before.
                    list_intent_rank(a)
                        .cmp(&list_intent_rank(b))
                        .then_with(|| overlap_b.cmp(&overlap_a))
                        .then_with(|| {
                            user_facing_fact_priority(a).cmp(&user_facing_fact_priority(b))
                        })
                        .then_with(|| domain_match(a).cmp(&domain_match(b)))
                        // **v4.11.6** — longer fact wins after overlap +
                        // priority tie. Pre-v4.11.6 the tiebreaker was
                        // `length(a) cmp length(b)` (shorter wins),
                        // which surfaced the scant definition
                        // `Химия — ғылым.` over the rich
                        // `Химия — заттардың құрамын, құрылысын,
                        // қасиеттерін және түрленулерін зерттейтін
                        // ғылым.` whenever both matched the same
                        // morpheme and predicate. For "what do you
                        // know about X?" questions, longer is
                        // measurably more informative — the user
                        // wants the school-curriculum definition
                        // over the one-word `X — ғылым.` stub.
                        .then_with(|| b.raw_text.chars().count().cmp(&a.raw_text.chars().count()))
                        .then_with(|| a.raw_text.cmp(&b.raw_text))
                });
                let total_matches = matches.len();
                let skipped_inadmissible = matches
                    .iter()
                    .filter(|fact| {
                        validate_fact(&fact.subject.root, fact.predicate, &fact.object.root)
                            .is_err()
                    })
                    .count();
                let surfaced: Vec<(String, ToolEvidence)> = matches
                    .into_iter()
                    .filter(|fact| {
                        validate_fact(&fact.subject.root, fact.predicate, &fact.object.root).is_ok()
                    })
                    .filter_map(render_grounded_graph_evidence)
                    .take(3)
                    .collect();
                let findings: Vec<String> = surfaced.iter().map(|(text, _)| text.clone()).collect();
                let evidence: Vec<ToolEvidence> =
                    surfaced.into_iter().map(|(_, evidence)| evidence).collect();
                trace.push(format!(
                    "search_graph: subject={subject} predicate={predicate:?} curated_matches={} admissible={} skipped_inadmissible={skipped_inadmissible}",
                    total_matches,
                    findings.len()
                ));
                if findings.is_empty() {
                    ToolResult::empty(call, "search_graph: no matching facts")
                } else {
                    ToolResult::ok(call, findings, evidence, trace)
                }
            }
            ToolCall::SearchRetrieval { morphemes } => {
                let Some(index) = ctx.retrieval else {
                    return ToolResult::unsupported(
                        call,
                        "search_retrieval: no MorphemeIndex attached to context",
                    );
                };
                let refs: Vec<&str> = morphemes.iter().map(|s| s.as_str()).collect();
                let default_cfg;
                let cfg = match ctx.rank_config {
                    Some(c) => c,
                    None => {
                        default_cfg = adam_retrieval::RankConfig::default();
                        &default_cfg
                    }
                };
                let hits = index.rank(&refs, cfg);
                let safe_hits: Vec<_> = hits
                    .iter()
                    .filter(|h| pack_is_chat_safe(&h.sref.pack))
                    .take(3)
                    .filter_map(|h| index.sample_text(&h.sref).map(String::from))
                    .collect();
                let evidence: Vec<ToolEvidence> = safe_hits
                    .iter()
                    .map(|text| ToolEvidence::RetrievalSample { text: text.clone() })
                    .collect();
                trace.push(format!(
                    "search_retrieval: morphemes={} hits={} safe_hits={}",
                    morphemes.len(),
                    hits.len(),
                    safe_hits.len()
                ));
                if safe_hits.is_empty() {
                    ToolResult::empty(call, "search_retrieval: no hits for given morphemes")
                } else {
                    ToolResult::ok(call, safe_hits, evidence, trace)
                }
            }
            ToolCall::RunLocalReasoner {
                topic,
                curated_only,
            } => {
                let passes_safety = |d: &&DerivedFact| -> bool {
                    !curated_only || adam_reasoning::reasoner::derivation_is_fully_curated(d)
                };
                let support_chain_evidence = |d: &DerivedFact| -> Option<Vec<SupportFactEvidence>> {
                    match validate_derived_fact_with_supports(d, ctx.extracted) {
                        Ok(()) => Some(
                            d.source_chain
                                .iter()
                                .filter_map(|source| find_support_fact(source, ctx.extracted))
                                .map(|fact| SupportFactEvidence {
                                    subject: fact.subject.root.clone(),
                                    predicate: fact.predicate,
                                    object: fact.object.root.clone(),
                                    confidence: fact.confidence,
                                    source: fact.source.clone(),
                                })
                                .collect(),
                        ),
                        Err(_) => None,
                    }
                };
                let candidates: Vec<&DerivedFact> = ctx
                    .derived
                    .iter()
                    .filter(|d| {
                        (d.subject.root == *topic || d.object.root == *topic) && passes_safety(d)
                    })
                    .filter(|d| support_chain_evidence(d).is_some())
                    .collect();
                trace.push(format!(
                    "run_local_reasoner: topic={topic} curated_only={curated_only} candidates={}",
                    candidates.len()
                ));
                let picked = candidates.iter().copied().max_by(|a, b| {
                    crate::conversation::score_derivation(a, topic)
                        .cmp(&crate::conversation::score_derivation(b, topic))
                        .then_with(|| {
                            if a.predicate == adam_reasoning::Predicate::IsA
                                && b.predicate == adam_reasoning::Predicate::IsA
                            {
                                let da = crate::conversation::isa_chain_depth(
                                    ctx.extracted,
                                    &a.subject.root,
                                    &a.object.root,
                                );
                                let db = crate::conversation::isa_chain_depth(
                                    ctx.extracted,
                                    &b.subject.root,
                                    &b.object.root,
                                );
                                da.cmp(&db).reverse()
                            } else {
                                std::cmp::Ordering::Equal
                            }
                        })
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
                                .reverse()
                        })
                });
                match picked {
                    None => {
                        ToolResult::empty(call, "run_local_reasoner: no derivation found for topic")
                    }
                    Some(d) => {
                        let rendered = crate::conversation::render_derivation_as_kazakh(d);
                        let support_chain = support_chain_evidence(d)
                            .expect("candidate passed validation so support chain must resolve");
                        let evidence = vec![ToolEvidence::DerivedFact {
                            subject: d.subject.root.clone(),
                            predicate: d.predicate,
                            object: d.object.root.clone(),
                            rule_id: d.rule_id.clone(),
                            confidence: d.confidence,
                            rendered: rendered.clone(),
                            support_chain,
                        }];
                        ToolResult::ok(call, vec![rendered], evidence, trace)
                    }
                }
            }
        }
    }
}

pub(crate) fn pack_is_chat_safe(pack: &str) -> bool {
    matches!(
        pack,
        "kazakh_classics_pack.json"
            | "abai_wikisource_pack.json"
            | "kazakh_proverbs_pack.json"
            | "tatoeba_kazakh_pack.json"
            | "common_voice_kk_pack.json"
            | "wikipedia_kz_pack.json"
            | "kazakh_textbooks_pack.json"
    )
}

/// **v4.4.11** — split user input into content tokens, lowercase,
/// stripped of punctuation, with the noun_hint itself removed (every
/// fact about that subject contains it, so it carries zero
/// discriminative signal). Tokens shorter than 4 codepoints are
/// dropped — Kazakh discourse particles / pronouns / case suffixes
/// are typically ≤ 3 characters and would inflate overlap scores
/// without informing relevance.
fn query_content_tokens(input: &str, subject: &str) -> Vec<String> {
    let subject_lower = subject.to_lowercase();
    input
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter_map(|w| {
            let trimmed = w.trim();
            if trimmed.is_empty() {
                return None;
            }
            if trimmed.chars().count() < 4 {
                return None;
            }
            if trimmed == subject_lower {
                return None;
            }
            // Strip the most common case suffix that would prevent
            // a substring match (locative -да/-де/-та/-те, ablative
            // -дан/-ден/-тан/-тен, dative -ға/-ге/-қа/-ке, plural
            // -лар/-лер/-дар/-дер/-тар/-тер, possessive -ы/-і).
            // We only strip if the resulting stem is ≥ 3 chars so
            // we don't over-aggressively chop. Also keep the
            // original — a fact may match either form.
            Some(trimmed.to_string())
        })
        .collect()
}

/// **v4.4.11** — count how many of the query's content tokens appear
/// as substrings of the fact's raw_text (case-folded). Substring
/// match is intentional — Kazakh is agglutinative and the question
/// often surfaces a different inflectional form than the fact's
/// canonical phrasing (e.g. user types «аймақтарының» but the
/// fact's surface text says «аймақтары»).
fn fact_overlap_score(fact: &ReasFact, query_tokens: &[String]) -> usize {
    let raw_lower = fact.raw_text.to_lowercase();
    let object_lower = fact.object.root.to_lowercase();
    query_tokens
        .iter()
        .filter(|tok| {
            // Whole-word check on raw_text first (more discriminating);
            // then a relaxed substring fallback against the fact's
            // object root so e.g. «өзендер» matches a fact whose
            // object root is «өзендер тізімі». Trimming the query
            // token to a 4-char prefix lets «аймақтарының» match
            // «аймақтары» without expensive stemming.
            let prefix_4: String = tok.chars().take(4).collect();
            raw_lower.contains(tok.as_str())
                || raw_lower.contains(&prefix_4)
                || object_lower.contains(&prefix_4)
        })
        .count()
}

fn user_facing_fact_priority(fact: &ReasFact) -> (usize, usize, isize) {
    let predicate_rank = match fact.predicate {
        ReasPredicate::IsA => 0,
        ReasPredicate::LivesIn => 1,
        ReasPredicate::HasQuantity => 2,
        ReasPredicate::PartOf => 3,
        ReasPredicate::Has => 4,
        ReasPredicate::InDomain => 5,
        ReasPredicate::RelatedTo => 6,
        ReasPredicate::GoesTo => 7,
        ReasPredicate::Causes => 8,
        ReasPredicate::After => 9,
        ReasPredicate::DoesTo => 10,
    };
    let subject_surface_rank = if fact.subject.surface == fact.subject.root {
        1
    } else {
        0
    };
    // **v4.11.7** — object-length component now NEGATED so longer
    // objects win in the priority ASC sort. For "what is X?" /
    // "tell me about X" questions, the more informative object
    // wins — `жасуша IsA тіршілік бірлігі` (compound) over
    // `жасуша IsA материя` (bare noun); `физика IsA табиғат
    // ғылымы` (compound) over `физика IsA ғылым`. Pre-v4.11.7 the
    // ASC sort preferred shorter objects, which favoured
    // generic over specific. Live REPL test 2026-04-30 confirmed
    // the regression on `Жасуша туралы не білесіз?` where the
    // v4.11.6 length tiebreaker never fired because this priority
    // tier already chose the scant version.
    (
        predicate_rank,
        subject_surface_rank,
        -(fact.object.root.chars().count() as isize),
    )
}

fn render_grounded_fact(fact: &ReasFact) -> Option<String> {
    let subject = preferred_subject_text(&fact.subject);
    let object = preferred_object_text(&fact.object);
    // v4.4.11 — when the fact's object encodes a structured
    // collection (its root contains «тізім» = "list", or it's
    // explicitly a multi-word "X тізімі" object), the canned
    // «{subject} мен {object} өзара байланысты» template reads
    // awkwardly («Қазақстан мен көлдер тізімі өзара байланысты»)
    // and hides the informative content from `raw_text`. Prefer the
    // raw sentence in that case — it's curated and carries the
    // actual list. Mirror of the existing «шектес» special-case for
    // border facts.
    let object_root_lower = fact.object.root.to_lowercase();
    let is_list_summary = object_root_lower.contains("тізім");
    let rendered = match fact.predicate {
        ReasPredicate::IsA => None,
        ReasPredicate::PartOf => Some(format!("{subject} {object} құрамына кіреді")),
        ReasPredicate::RelatedTo if fact.raw_text.contains("шектес") => {
            Some(fact.raw_text.trim().to_string())
        }
        ReasPredicate::RelatedTo if is_list_summary => Some(fact.raw_text.trim().to_string()),
        ReasPredicate::RelatedTo => Some(format!("{subject} мен {object} өзара байланысты")),
        ReasPredicate::InDomain => Some(format!("{subject} {object} саласына жатады")),
        ReasPredicate::LivesIn => Some(format!("{subject} мекені — {object}")),
        ReasPredicate::Has => Some(format!("{subject} {object} иеленеді")),
        ReasPredicate::Causes => Some(format!("{subject} {object} себебі болады")),
        ReasPredicate::GoesTo
        | ReasPredicate::After
        | ReasPredicate::HasQuantity
        | ReasPredicate::DoesTo => None,
    };
    rendered
        .filter(|text| !text.trim().is_empty())
        .map(ensure_sentence_period)
        .or_else(|| {
            let text = fact.raw_text.trim();
            if text.is_empty() {
                None
            } else {
                Some(ensure_sentence_period(text.to_string()))
            }
        })
}

fn render_grounded_graph_evidence(fact: &ReasFact) -> Option<(String, ToolEvidence)> {
    let rendered = render_grounded_fact(fact)?;
    Some((
        rendered.clone(),
        ToolEvidence::GraphFact {
            subject: fact.subject.root.clone(),
            predicate: fact.predicate,
            object: fact.object.root.clone(),
            confidence: fact.confidence,
            rendered,
        },
    ))
}

fn predicate_name_matches(predicate: ReasPredicate, needle: &str) -> bool {
    let normalised = needle.to_lowercase().replace('_', "");
    predicate.as_str().replace('_', "") == normalised
}

fn preferred_subject_text(slot: &adam_reasoning::SlotRef) -> String {
    capitalise_first(preferred_slot_text(slot))
}

fn preferred_object_text(slot: &adam_reasoning::SlotRef) -> String {
    preferred_slot_text(slot)
}

fn preferred_slot_text(slot: &adam_reasoning::SlotRef) -> String {
    let text = if slot.surface.trim().is_empty() {
        slot.root.trim()
    } else {
        slot.surface.trim()
    };
    text.to_string()
}

fn capitalise_first(text: String) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn ensure_sentence_period(text: String) -> String {
    if text.ends_with(['.', '!', '?']) {
        text
    } else {
        format!("{text}.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::belief::{BeliefState, USER_SELF_KEY};
    use adam_reasoning::{ConfidenceKind, FactSource, Predicate, SlotRef};

    fn fact(subject: &str, pred: Predicate, object: &str) -> ReasFact {
        ReasFact {
            subject: SlotRef {
                surface: subject.into(),
                root: subject.into(),
                pos: "noun".into(),
            },
            predicate: pred,
            object: SlotRef {
                surface: object.into(),
                root: object.into(),
                pos: "noun".into(),
            },
            pattern: "test".into(),
            source: FactSource {
                pack: "test".into(),
                sample_id: "t1".into(),
            },
            confidence: ConfidenceKind::Grammar,
            raw_text: format!("{subject} — {object}"),
        }
    }

    fn derived(subject: &str, pred: Predicate, object: &str) -> DerivedFact {
        DerivedFact {
            subject: SlotRef {
                surface: subject.into(),
                root: subject.into(),
                pos: "noun".into(),
            },
            predicate: pred,
            object: SlotRef {
                surface: object.into(),
                root: object.into(),
                pos: "noun".into(),
            },
            rule_id: "R1_is_a_transitivity".into(),
            source_chain: vec![FactSource {
                pack: "test".into(),
                sample_id: "t1".into(),
            }],
            confidence: ConfidenceKind::RuleInferred,
        }
    }

    fn ctx<'a>(belief: &'a BeliefState, extracted: &'a [ReasFact]) -> ToolContext<'a> {
        ToolContext {
            belief,
            extracted,
            derived: &[],
            retrieval: None,
            rank_config: None,
            query_input: None,
            current_domain: None,
            domain_index: None,
            previous_grounded_fact: None,
        }
    }

    #[test]
    fn search_belief_finds_active_fact() {
        let mut belief = BeliefState::new();
        belief.record_user_fact(USER_SELF_KEY, "name", "Дәулет", 0);
        let r = Tool::dispatch(
            ToolCall::SearchBelief {
                subject: USER_SELF_KEY.into(),
                predicate: None,
            },
            &ctx(&belief, &[]),
        );
        assert!(r.success);
        assert_eq!(r.findings.len(), 1);
        assert!(r.findings[0].contains("Дәулет"));
    }

    #[test]
    fn search_belief_empty_on_no_match() {
        let belief = BeliefState::new();
        let r = Tool::dispatch(
            ToolCall::SearchBelief {
                subject: "nonexistent".into(),
                predicate: None,
            },
            &ctx(&belief, &[]),
        );
        assert!(!r.success);
        assert!(r.findings.is_empty());
    }

    #[test]
    fn search_belief_skips_contested_facts() {
        let mut belief = BeliefState::new();
        belief.record_user_fact(USER_SELF_KEY, "city", "алматы", 0);
        belief.record_user_fact(USER_SELF_KEY, "city", "астана", 1);
        let r = Tool::dispatch(
            ToolCall::SearchBelief {
                subject: USER_SELF_KEY.into(),
                predicate: None,
            },
            &ctx(&belief, &[]),
        );
        assert!(!r.success, "contested facts must not surface as Active");
    }

    /// **v4.1.5** — when `predicate` is `Some(p)`, findings are the
    /// raw object strings only (no `subject {predicate} object`
    /// triple) so callers like `ActionPlanner::belief_direct_answer`
    /// can use the value directly as a slot fill.
    #[test]
    fn search_belief_with_predicate_returns_object_only() {
        let mut belief = BeliefState::new();
        belief.record_user_fact(USER_SELF_KEY, "name", "Дәулет", 0);
        belief.record_user_fact(USER_SELF_KEY, "city", "алматы", 1);
        let r = Tool::dispatch(
            ToolCall::SearchBelief {
                subject: USER_SELF_KEY.into(),
                predicate: Some("city".into()),
            },
            &ctx(&belief, &[]),
        );
        assert!(r.success);
        assert_eq!(r.findings, vec!["алматы".to_string()]);
    }

    /// **v4.1.5** — `predicate` filter respects single-active-fact
    /// invariant: at most one finding for a given `(subject, predicate)`
    /// because the same gate as `BeliefState::active_fact` applies.
    #[test]
    fn search_belief_with_predicate_returns_empty_on_no_active() {
        let mut belief = BeliefState::new();
        // Both contested → no Active fact for `city`.
        belief.record_user_fact(USER_SELF_KEY, "city", "алматы", 0);
        belief.record_user_fact(USER_SELF_KEY, "city", "астана", 1);
        let r = Tool::dispatch(
            ToolCall::SearchBelief {
                subject: USER_SELF_KEY.into(),
                predicate: Some("city".into()),
            },
            &ctx(&belief, &[]),
        );
        assert!(!r.success);
        assert!(r.findings.is_empty());
    }

    #[test]
    fn search_graph_filters_by_subject() {
        let mut a = fact("жер", Predicate::IsA, "аспан денесі");
        a.confidence = ConfidenceKind::HumanApproved;
        let mut b = fact("күн", Predicate::IsA, "жұлдыз");
        b.confidence = ConfidenceKind::HumanApproved;
        let mut c = fact("жер", Predicate::Has, "ауа");
        c.confidence = ConfidenceKind::HumanApproved;
        let extracted = vec![a, b, c];
        let belief = BeliefState::new();
        let r = Tool::dispatch(
            ToolCall::SearchGraph {
                subject: "жер".into(),
                predicate: None,
            },
            &ctx(&belief, &extracted),
        );
        assert!(r.success);
        assert_eq!(r.findings.len(), 2);
    }

    #[test]
    fn search_graph_filters_by_subject_and_predicate() {
        let mut a = fact("жер", Predicate::IsA, "аспан денесі");
        a.confidence = ConfidenceKind::HumanApproved;
        let mut b = fact("жер", Predicate::Has, "ауа");
        b.confidence = ConfidenceKind::HumanApproved;
        let extracted = vec![a, b];
        let belief = BeliefState::new();
        let r = Tool::dispatch(
            ToolCall::SearchGraph {
                subject: "жер".into(),
                predicate: Some("isa".into()),
            },
            &ctx(&belief, &extracted),
        );
        assert!(r.success);
        assert_eq!(r.findings.len(), 1);
        assert!(r.findings[0].contains("аспан денесі"));
    }

    #[test]
    fn search_graph_only_surfaces_human_approved_facts() {
        let mut curated = fact("қазақстан", Predicate::IsA, "мемлекет");
        curated.confidence = ConfidenceKind::HumanApproved;
        curated.raw_text = "Қазақстан — мемлекет".into();
        let noisy = fact("қазақстан", Predicate::IsA, "ұйым");
        let belief = BeliefState::new();
        let r = Tool::dispatch(
            ToolCall::SearchGraph {
                subject: "қазақстан".into(),
                predicate: None,
            },
            &ctx(&belief, &[curated, noisy]),
        );
        assert!(r.success);
        assert_eq!(r.findings, vec!["Қазақстан — мемлекет.".to_string()]);
    }

    #[test]
    fn search_graph_filters_out_ontology_invalid_facts() {
        let mut invalid = fact("адам", Predicate::LivesIn, "ақын");
        invalid.confidence = ConfidenceKind::HumanApproved;
        invalid.raw_text = "Адам ақында тұрады".into();
        let belief = BeliefState::new();
        let r = Tool::dispatch(
            ToolCall::SearchGraph {
                subject: "адам".into(),
                predicate: None,
            },
            &ctx(&belief, &[invalid]),
        );
        assert!(!r.success);
        assert!(r.findings.is_empty());
    }

    #[test]
    fn grounded_fact_composer_renders_part_of_as_sentence() {
        let mut fact = fact("әке", Predicate::PartOf, "отбасы");
        fact.confidence = ConfidenceKind::HumanApproved;
        fact.raw_text = "Әке — отбасының мүшесі".into();
        assert_eq!(
            render_grounded_fact(&fact),
            Some("Әке отбасы құрамына кіреді.".to_string())
        );
    }

    #[test]
    fn grounded_fact_composer_falls_back_to_raw_text_when_needed() {
        let mut fact = fact("таң", Predicate::After, "түн");
        fact.confidence = ConfidenceKind::HumanApproved;
        fact.raw_text = "Таң түннен кейін келеді".into();
        assert_eq!(
            render_grounded_fact(&fact),
            Some("Таң түннен кейін келеді.".to_string())
        );
    }

    #[test]
    fn grounded_fact_keeps_richer_raw_text_for_is_a() {
        let mut fact = fact("қазақстан", Predicate::IsA, "ел");
        fact.confidence = ConfidenceKind::HumanApproved;
        fact.raw_text = "Қазақстан — Орталық Азиядағы ел".into();
        assert_eq!(
            render_grounded_fact(&fact),
            Some("Қазақстан — Орталық Азиядағы ел.".to_string())
        );
    }

    #[test]
    fn pack_allowlist_blocks_synthetic_and_cc100_for_chat() {
        assert!(pack_is_chat_safe("abai_wikisource_pack.json"));
        assert!(!pack_is_chat_safe("synthetic_sentences_pack.json"));
        assert!(!pack_is_chat_safe("cc100_kk_pack.json"));
    }

    /// v4.0.38 — SearchRetrieval without a MorphemeIndex returns
    /// `success=false` with explicit reason. Useful for callers
    /// that try to dispatch unconditionally.
    #[test]
    fn search_retrieval_unsupported_without_index() {
        let belief = BeliefState::new();
        let r = Tool::dispatch(
            ToolCall::SearchRetrieval {
                morphemes: vec!["жер".into()],
            },
            &ctx(&belief, &[]),
        );
        assert!(!r.success);
        assert!(
            r.trace.iter().any(|t| t.contains("no MorphemeIndex")),
            "must explain why dispatch failed, got {r:?}"
        );
    }

    /// v4.0.38 — RunLocalReasoner finds derivations whose subject
    /// or object matches the topic. Up to 3 matches returned.
    #[test]
    fn run_local_reasoner_finds_matching_derivations() {
        let derived_facts = vec![
            derived("жер", Predicate::IsA, "аспан денесі"),
            derived("күн", Predicate::IsA, "жұлдыз"),
        ];
        let support_facts = vec![
            fact("жер", Predicate::IsA, "ғаламшар"),
            fact("күн", Predicate::IsA, "жұлдыз"),
        ];
        let belief = BeliefState::new();
        let context = ToolContext {
            belief: &belief,
            extracted: &support_facts,
            derived: &derived_facts,
            retrieval: None,
            rank_config: None,
            query_input: None,
            current_domain: None,
            domain_index: None,
            previous_grounded_fact: None,
        };
        let r = Tool::dispatch(
            ToolCall::RunLocalReasoner {
                topic: "жер".into(),
                curated_only: false,
            },
            &context,
        );
        assert!(r.success);
        assert_eq!(r.findings.len(), 1);
        assert!(r.findings[0].contains("аспан денесі"));
    }

    #[test]
    fn run_local_reasoner_empty_when_no_match() {
        let belief = BeliefState::new();
        let r = Tool::dispatch(
            ToolCall::RunLocalReasoner {
                topic: "nonexistent".into(),
                curated_only: false,
            },
            &ctx(&belief, &[]),
        );
        assert!(!r.success);
    }

    #[test]
    fn run_local_reasoner_filters_out_ontology_invalid_derivations() {
        let derived_facts = vec![DerivedFact {
            subject: SlotRef {
                surface: "жер".into(),
                root: "жер".into(),
                pos: "noun".into(),
            },
            predicate: Predicate::LivesIn,
            object: SlotRef {
                surface: "аспан денесі".into(),
                root: "аспан денесі".into(),
                pos: "noun".into(),
            },
            rule_id: "R1_is_a_transitivity".into(),
            source_chain: vec![FactSource {
                pack: "world_core/test.jsonl".into(),
                sample_id: "t1".into(),
            }],
            confidence: ConfidenceKind::RuleInferred,
        }];
        let belief = BeliefState::new();
        let context = ToolContext {
            belief: &belief,
            extracted: &[],
            derived: &derived_facts,
            retrieval: None,
            rank_config: None,
            query_input: None,
            current_domain: None,
            domain_index: None,
            previous_grounded_fact: None,
        };
        let r = Tool::dispatch(
            ToolCall::RunLocalReasoner {
                topic: "жер".into(),
                curated_only: false,
            },
            &context,
        );
        assert!(!r.success);
        assert!(r.findings.is_empty());
    }

    #[test]
    fn dispatch_records_call_in_result() {
        let belief = BeliefState::new();
        let call = ToolCall::SearchBelief {
            subject: "x".into(),
            predicate: None,
        };
        let r = Tool::dispatch(call.clone(), &ctx(&belief, &[]));
        assert_eq!(r.call, call);
    }
}
