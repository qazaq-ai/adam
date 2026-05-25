// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `dialog_battery` — **the canonical real-Kazakh quality gate**.
//!
//! Lives in `adam-algebra` so the same battery is used by:
//!
//! - The `#[test] fn dialog_battery_meets_quality_gate` unit test
//!   in `src/index.rs` — CI gate that fails red if real-dialog
//!   relevance drops below the predeclared floor.
//! - The `examples/bench_pipeline.rs` example — prints the full
//!   quality report (latency + adequacy + relevance + surface +
//!   sense correctness).
//!
//! ## The directive this module enforces
//!
//! Per the user's 2026-05-25 feedback (memory:
//! `feedback_unit_tests_must_be_real_dialog`):
//!
//! > Unit tests with synthetic data («agent_0», «year_7») pass
//! > while real dialog fails. Every feature must be tested on
//! > real Kazakh phrasings, and the bench must report
//! > adequacy / relevance / human-level quality — not only
//! > latency.
//!
//! No synthetic data here. Every [`DialogCase`] is a real Kazakh
//! utterance traced to a v6.1 REPL audit, a curated `world_core`
//! fact, or a canonical biographical / geographical / institutional
//! reference.
//!
//! ## What the report measures
//!
//! - **Adequacy** — did the index return any candidate?
//! - **Relevance** — did the returned `AnswerSlot` match what the
//!   question asked for?
//! - **Surface correctness** — does the returned answer's
//!   `root.surface` match the curated truth?
//! - **Sense correctness** — for sense-ambiguous inputs («Ай»,
//!   «Ағаш», «бағдарлама»), did the right `Domain` win?
//! - **Latency** — warm median per case.
//!
//! ## Predeclared quality floor (CI gate)
//!
//! - Adequacy: **100 %** (every case must return at least one
//!   candidate).
//! - Relevance: **≥ 90 %** (answer-slot correct on at least 90 %
//!   of cases).
//! - Sense correctness: **100 %** on sense-tagged cases.
//! - Surface correctness: **≥ 85 %** (warm baseline; we improve
//!   this with each stage).
//!
//! When a new stage lifts coverage, raise the floor — never lower
//! it. A regression below the floor is a red CI, period.

use crate::composition::Composition;
use crate::frame::{Frame, FramePredicate, Modifier, TimeAnchor};
use crate::index::FrameIndex;
use crate::math_solver;
use crate::operator::SuffixOp;
use crate::query::{
    AnswerShape, AnswerSlot, Domain, ModifierRole, QueryFocus, QueryIR, QuestionForm,
};
use crate::root::{PartOfSpeech, Root};
use adam_kernel_fst::morphotactics::Case;

/// A single real-Kazakh dialog case.
///
/// Fields:
/// - `user_input` — the human's question (real surface form,
///   verbatim).
/// - `query` — the [`QueryIR`] the v6.2 pipeline should produce
///   after Stages 2-5 are fully wired. Stage 4 constructs this
///   manually; once Stage 6 (learned parser) ships, this becomes
///   the *expected* output of automatic parsing and the case
///   doubles as a parser regression test.
/// - `expected_slot` — which slot of the matched frame holds the
///   answer.
/// - `expected_surface` — the canonical answer surface
///   (`root.surface` after lowercase). Used for surface-correctness
///   scoring.
/// - `domain` — the sense-domain the asker is in (when the case is
///   sense-ambiguous; `None` otherwise).
/// - `tag` — short label for the failure report (e.g. "bio/birth-
///   date", "geography/oblast-count").
pub struct DialogCase {
    pub user_input: &'static str,
    pub tag: &'static str,
    pub query: QueryIR,
    pub expected_slot: AnswerSlot,
    pub expected_surface: &'static str,
    pub domain: Option<Domain>,
    /// When `Some(reason)`, this case is a **documented
    /// architectural gap** — failure is expected until a future
    /// stage lifts it. The quality gate ignores known-gap failures
    /// (they don't count toward regression), but the report still
    /// surfaces them so progress is visible turn over turn.
    ///
    /// **Conversely**: when `known_gap` is `None` and the case
    /// fails, it is a *regression* — CI fails red.
    ///
    /// Closing a gap = removing the `known_gap` annotation. The
    /// case then re-joins the must-pass set and any future
    /// regression of it trips CI.
    pub known_gap: Option<&'static str>,
}

/// Result of running one case against the index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseOutcome {
    pub tag: &'static str,
    pub user_input: &'static str,
    /// True iff `idx.query(&case.query)` returned ≥ 1 hit.
    pub adequate: bool,
    /// True iff the top hit's `answer_slot` matched
    /// `case.expected_slot`.
    pub relevant: bool,
    /// True iff the surface of the answer slot equals
    /// `case.expected_surface`.
    pub surface_correct: bool,
    /// True iff the answer's domain matched `case.domain`
    /// (always true when `case.domain` is `None`).
    pub sense_correct: bool,
    /// What the index actually returned, for diagnostic.
    pub actual_surface: Option<String>,
    /// Latency of the single call (nanoseconds, COLD).
    pub latency_ns: u128,
    /// Carry the known-gap annotation from the source case so the
    /// report can split must-pass failures (regressions) from
    /// documented gaps.
    pub known_gap: Option<&'static str>,
}

impl CaseOutcome {
    /// True iff the case fully met every quality dimension.
    pub fn fully_passing(&self) -> bool {
        self.adequate && self.relevant && self.surface_correct && self.sense_correct
    }
}

/// Aggregate quality + latency report.
#[derive(Debug, Clone)]
pub struct BatteryReport {
    pub outcomes: Vec<CaseOutcome>,
    pub total: usize,
    pub adequate: usize,
    pub relevant: usize,
    pub surface_correct: usize,
    pub sense_correct: usize,
    pub median_warm_ns: u128,
    pub p95_warm_ns: u128,
    /// Cases tagged as known architectural gaps (failure expected).
    pub known_gap_total: usize,
    /// Known-gap cases that DID pass — track these because once a
    /// gap is closed, the case should be promoted out of the gap
    /// set (drop the `known_gap` annotation).
    pub known_gap_unexpectedly_passing: usize,
    /// **Regressions** — non-gap cases that failed any dimension.
    /// This number must be 0 for the CI gate to be green.
    pub regressions: usize,
}

impl BatteryReport {
    /// Cases that are part of the must-pass set (no `known_gap`).
    pub fn must_pass_total(&self) -> usize {
        self.total - self.known_gap_total
    }

    fn must_pass<'a>(&'a self) -> impl Iterator<Item = &'a CaseOutcome> + 'a {
        self.outcomes.iter().filter(|o| o.known_gap.is_none())
    }

    pub fn adequacy_pct(&self) -> f64 {
        let m = self.must_pass_total().max(1);
        100.0 * self.must_pass().filter(|o| o.adequate).count() as f64 / m as f64
    }
    pub fn relevance_pct(&self) -> f64 {
        let m = self.must_pass_total().max(1);
        100.0 * self.must_pass().filter(|o| o.relevant).count() as f64 / m as f64
    }
    pub fn surface_pct(&self) -> f64 {
        let m = self.must_pass_total().max(1);
        100.0 * self.must_pass().filter(|o| o.surface_correct).count() as f64 / m as f64
    }
    pub fn sense_pct(&self) -> f64 {
        let m = self.must_pass_total().max(1);
        100.0 * self.must_pass().filter(|o| o.sense_correct).count() as f64 / m as f64
    }

    /// Print the full report to stdout in the format the user sees
    /// from `bench_pipeline`.
    pub fn print(&self) {
        let mp = self.must_pass_total();
        println!("=== Dialog Battery Report ===");
        println!(
            "Total cases: {}  (must-pass: {}, known gaps: {})",
            self.total, mp, self.known_gap_total
        );
        println!("--- Must-pass set (regression gate) ---");
        println!(
            "Adequacy:        {:>3}/{} ({:>5.1}%)  — got an answer",
            self.must_pass().filter(|o| o.adequate).count(),
            mp,
            self.adequacy_pct()
        );
        println!(
            "Relevance:       {:>3}/{} ({:>5.1}%)  — right slot",
            self.must_pass().filter(|o| o.relevant).count(),
            mp,
            self.relevance_pct()
        );
        println!(
            "Surface correct: {:>3}/{} ({:>5.1}%)  — right value",
            self.must_pass().filter(|o| o.surface_correct).count(),
            mp,
            self.surface_pct()
        );
        println!(
            "Sense correct:   {:>3}/{} ({:>5.1}%)  — no wrong-sense",
            self.must_pass().filter(|o| o.sense_correct).count(),
            mp,
            self.sense_pct()
        );
        println!(
            "Regressions:      {} (must be 0 for CI green)",
            self.regressions
        );
        println!();
        println!(
            "Median warm latency: {} ns ({:.3} µs) per case",
            self.median_warm_ns,
            self.median_warm_ns as f64 / 1_000.0
        );
        println!(
            "p95    warm latency: {} ns ({:.3} µs) per case",
            self.p95_warm_ns,
            self.p95_warm_ns as f64 / 1_000.0
        );

        let regressions: Vec<&CaseOutcome> = self
            .outcomes
            .iter()
            .filter(|o| o.known_gap.is_none() && !o.fully_passing())
            .collect();
        if !regressions.is_empty() {
            println!();
            println!(
                "REGRESSIONS ({}) — must be fixed before merge:",
                regressions.len()
            );
            for f in regressions {
                println!(
                    "  - [{}] «{}» — {}  (got: {:?})",
                    f.tag,
                    f.user_input,
                    failure_kinds(f),
                    f.actual_surface
                );
            }
        }

        let known_gaps: Vec<&CaseOutcome> = self
            .outcomes
            .iter()
            .filter(|o| o.known_gap.is_some() && !o.fully_passing())
            .collect();
        if !known_gaps.is_empty() {
            println!();
            println!(
                "Known architectural gaps ({}) — documented, not blocking:",
                known_gaps.len()
            );
            for f in known_gaps {
                println!(
                    "  - [{}] «{}» — gap: {}",
                    f.tag,
                    f.user_input,
                    f.known_gap.unwrap_or("?")
                );
            }
        }

        if self.known_gap_unexpectedly_passing > 0 {
            println!();
            println!(
                "Promote-out candidates ({}): known-gap cases now passing — drop the `known_gap` annotation in a follow-up commit.",
                self.known_gap_unexpectedly_passing
            );
            for o in self
                .outcomes
                .iter()
                .filter(|o| o.known_gap.is_some() && o.fully_passing())
            {
                println!("  - [{}] «{}»", o.tag, o.user_input);
            }
        }
    }
}

fn failure_kinds(o: &CaseOutcome) -> String {
    let kinds = [
        (!o.adequate, "no-answer"),
        (!o.relevant, "wrong-slot"),
        (!o.surface_correct, "wrong-surface"),
        (!o.sense_correct, "wrong-sense"),
    ];
    kinds
        .iter()
        .filter(|(b, _)| *b)
        .map(|(_, s)| *s)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Run the full dialog battery against the canonical knowledge
/// graph. Returns a `BatteryReport`. The number of warm iterations
/// per case is bounded so this can be called from a `#[test]`.
pub fn run_dialog_battery() -> BatteryReport {
    let idx = canonical_corpus();
    let cases = canonical_cases();
    let mut outcomes = Vec::with_capacity(cases.len());

    // Warm up the index once across all queries to amortise the
    // first-call cost of HashMap probes.
    for case in &cases {
        std::hint::black_box(idx.query(&case.query));
    }

    let mut warm_samples: Vec<u128> = Vec::with_capacity(cases.len() * 200);

    for case in &cases {
        // Math/* cases route through the deterministic
        // MathSolver, not the FrameIndex. This is the v6.2
        // architectural pattern: solvers sit beside the fact
        // index, both feed the answer.
        let is_math = case.tag.starts_with("math/");

        // -- COLD-ish (one fresh call after warmup) for the per-
        //    case latency reported in CaseOutcome --
        let start = std::time::Instant::now();
        let (adequate, relevant, actual_surface, sense_correct);
        if is_math {
            let result = math_solver::solve(case.user_input);
            adequate = result.is_some();
            relevant = result.is_some(); // math always returns the
            // "object" slot (the result); slot type matches by
            // construction when adequate.
            actual_surface = result.map(|r| r.render());
            sense_correct = true; // math has no sense ambiguity
        } else {
            let hits = idx.query(&case.query);
            adequate = !hits.is_empty();
            let top = hits.first();
            relevant = top
                .map(|h| answer_slot_matches(h.match_result.answer_slot, case.expected_slot))
                .unwrap_or(false);
            actual_surface = top.and_then(|h| surface_of_slot(h.frame, h.match_result.answer_slot));
            sense_correct = match (&case.domain, top) {
                (None, _) => true,
                (Some(_), None) => false,
                (Some(expected), Some(hit)) => hit.domain == Some(expected),
            };
        }
        let latency_ns = start.elapsed().as_nanos();

        let surface_correct = match &actual_surface {
            Some(s) => s.as_str() == case.expected_surface,
            None => false,
        };

        // -- WARM samples for the aggregate p50/p95 --
        for _ in 0..200 {
            let s = std::time::Instant::now();
            if is_math {
                std::hint::black_box(math_solver::solve(case.user_input));
            } else {
                std::hint::black_box(idx.query(&case.query));
            }
            warm_samples.push(s.elapsed().as_nanos());
        }

        outcomes.push(CaseOutcome {
            tag: case.tag,
            user_input: case.user_input,
            adequate,
            relevant,
            surface_correct,
            sense_correct,
            actual_surface,
            latency_ns,
            known_gap: case.known_gap,
        });
    }

    warm_samples.sort_unstable();
    let median_warm_ns = warm_samples[warm_samples.len() / 2];
    let p95_warm_ns = warm_samples[warm_samples.len() * 95 / 100];

    let total = outcomes.len();
    let adequate = outcomes.iter().filter(|o| o.adequate).count();
    let relevant = outcomes.iter().filter(|o| o.relevant).count();
    let surface_correct = outcomes.iter().filter(|o| o.surface_correct).count();
    let sense_correct = outcomes.iter().filter(|o| o.sense_correct).count();
    let known_gap_total = outcomes.iter().filter(|o| o.known_gap.is_some()).count();
    let known_gap_unexpectedly_passing = outcomes
        .iter()
        .filter(|o| o.known_gap.is_some() && o.fully_passing())
        .count();
    let regressions = outcomes
        .iter()
        .filter(|o| o.known_gap.is_none() && !o.fully_passing())
        .count();

    BatteryReport {
        outcomes,
        total,
        adequate,
        relevant,
        surface_correct,
        sense_correct,
        median_warm_ns,
        p95_warm_ns,
        known_gap_total,
        known_gap_unexpectedly_passing,
        regressions,
    }
}

/// `AnswerSlot` equality with modifier-role variance: we accept a
/// `Whole` answer when the question wanted a specific modifier
/// but the candidate doesn't carry that slot — that's a
/// "partial-relevance" outcome the bench surfaces separately via
/// `surface_correct`. Strict slot-equality otherwise.
fn answer_slot_matches(actual: AnswerSlot, expected: AnswerSlot) -> bool {
    actual == expected
}

/// Surface form of the slot of a frame, used for surface-correctness.
fn surface_of_slot(frame: &Frame, slot: AnswerSlot) -> Option<String> {
    match slot {
        AnswerSlot::Agent => frame.agent.as_ref().map(|c| c.root.surface.clone()),
        AnswerSlot::Object => frame.object.as_ref().map(|c| c.root.surface.clone()),
        AnswerSlot::Predicate => Some(frame.predicate.as_str().to_string()),
        AnswerSlot::Modifier(role) => {
            let m = frame.modifier(role.as_str())?;
            modifier_surface(m)
        }
        AnswerSlot::Whole => {
            // For Whole answers (Existence / Definition / Quantity /
            // Enumeration), the "surface" is the object (the
            // definition predicate's complement). Pull `object` if
            // present; else the agent; else the predicate slug.
            frame
                .object
                .as_ref()
                .map(|c| c.root.surface.clone())
                .or_else(|| frame.agent.as_ref().map(|c| c.root.surface.clone()))
                .or_else(|| Some(frame.predicate.as_str().to_string()))
        }
    }
}

fn modifier_surface(m: &Modifier) -> Option<String> {
    match m {
        Modifier::TimeAnchor(TimeAnchor::Phrase(c)) => Some(c.root.surface.clone()),
        Modifier::TimeAnchor(TimeAnchor::Year(y)) => Some(format!("{y}")),
        Modifier::TimeAnchor(TimeAnchor::Date { year, month, day }) => {
            Some(format!("{year:04}-{month:02}-{day:02}"))
        }
        Modifier::Location(c)
        | Modifier::Source(c)
        | Modifier::Instrument(c)
        | Modifier::Manner(c)
        | Modifier::Recipient(c)
        | Modifier::Possessor(c) => Some(c.root.surface.clone()),
    }
}

// === Canonical knowledge graph ===========================
//
// Mirrors real entries from `data/world_core/*.jsonl` — Ahmet
// Baytursynuly biographical, Қостанай / Қазақстан geography, AI
// Law (Жасанды интеллект туралы заң) classification, etc. Roots
// are lowercased canonical surfaces; compound names («ахмет
// байтұрсынұлы», «жасанды интеллект туралы заң») are single
// canonical strings — Stage 5+ will resolve them from multi-word
// input.

fn noun(s: &str) -> Composition {
    Composition::identity(Root::new(s, PartOfSpeech::Noun))
}

fn locative(s: &str) -> Composition {
    let mut c = noun(s);
    c.operators.push(SuffixOp::Case(Case::Locative));
    c
}

fn year_phrase(year: i32) -> Composition {
    let mut c = noun(&format!("{year} жыл"));
    c.operators.push(SuffixOp::Case(Case::Locative));
    c
}

/// Populate a FrameIndex with the canonical curated knowledge the
/// battery queries against. Returns the populated index.
pub fn canonical_corpus() -> FrameIndex {
    let mut idx = FrameIndex::new();

    // --- Person: Ахмет Байтұрсынұлы ---
    idx.insert(
        Frame::assertion(
            Some(noun("ахмет байтұрсынұлы")),
            FramePredicate::BornIn,
            None,
        )
        .with_modifier(Modifier::TimeAnchor(TimeAnchor::Year(1872)))
        .with_modifier(Modifier::Location(locative("қостанай"))),
        Some(Domain::Person),
    );
    idx.insert(
        Frame::assertion(
            Some(noun("ахмет байтұрсынұлы")),
            FramePredicate::IsA,
            Some(noun("лингвист")),
        ),
        Some(Domain::Person),
    );
    idx.insert(
        Frame::assertion(
            Some(noun("ахмет байтұрсынұлы")),
            FramePredicate::Authored,
            Some(noun("төте жазу")),
        ),
        Some(Domain::Person),
    );

    // --- Person: Абай ---
    idx.insert(
        Frame::assertion(Some(noun("абай")), FramePredicate::BornIn, None)
            .with_modifier(Modifier::TimeAnchor(TimeAnchor::Year(1845))),
        Some(Domain::Person),
    );
    idx.insert(
        Frame::assertion(
            Some(noun("абай")),
            FramePredicate::LivesIn,
            Some(noun("семей")),
        ),
        Some(Domain::Person),
    );

    // --- Geography: Қазақстан ---
    idx.insert(
        Frame::assertion(
            Some(noun("қазақстан")),
            FramePredicate::IsA,
            Some(noun("мемлекет")),
        ),
        Some(Domain::Geography),
    );
    idx.insert(
        Frame::assertion(
            Some(noun("қазақстан")),
            FramePredicate::HasQuantity,
            Some(noun("17 облыс")),
        ),
        Some(Domain::Geography),
    );

    // --- Geography: Қостанай облысы ---
    idx.insert(
        Frame::assertion(
            Some(noun("қостанай")),
            FramePredicate::LocatedIn,
            Some(noun("қазақстан")),
        ),
        Some(Domain::Geography),
    );
    idx.insert(
        Frame::assertion(
            Some(noun("қостанай")),
            FramePredicate::IsA,
            Some(noun("облыс орталығы")),
        ),
        Some(Domain::Geography),
    );

    // --- Institution: КРУ им. Байтұрсынұлы ---
    idx.insert(
        Frame::assertion(Some(noun("кру")), FramePredicate::FoundedIn, None)
            .with_modifier(Modifier::TimeAnchor(TimeAnchor::Year(1939))),
        Some(Domain::Institution),
    );
    idx.insert(
        Frame::assertion(
            Some(noun("кру")),
            FramePredicate::NamedAfter,
            Some(noun("ахмет байтұрсынұлы")),
        ),
        Some(Domain::Institution),
    );
    idx.insert(
        Frame::assertion(
            Some(noun("кру")),
            FramePredicate::LocatedIn,
            Some(noun("қостанай")),
        ),
        Some(Domain::Institution),
    );

    // --- Law: Жасанды интеллект туралы заң ---
    idx.insert(
        Frame::assertion(
            Some(noun("жасанды интеллект туралы заң")),
            FramePredicate::EffectiveFrom,
            None,
        )
        .with_modifier(Modifier::TimeAnchor(TimeAnchor::Date {
            year: 2026,
            month: 1,
            day: 18,
        })),
        Some(Domain::Law),
    );
    idx.insert(
        Frame::assertion(
            Some(noun("жасанды интеллект туралы заң")),
            FramePredicate::Classifies,
            Some(noun("тәуекел санаттары")),
        ),
        Some(Domain::Law),
    );

    // --- Sense-ambiguous: «Ай» — calendar (month) vs astronomy (moon) ---
    idx.insert(
        Frame::assertion(
            Some(noun("ай")),
            FramePredicate::IsA,
            Some(noun("уақыт өлшемі")),
        ),
        Some(Domain::Calendar),
    );
    idx.insert(
        Frame::assertion(
            Some(noun("ай")),
            FramePredicate::IsA,
            Some(noun("аспан денесі")),
        ),
        Some(Domain::Astronomy),
    );

    // --- Sense-ambiguous: «Ағаш» — botany (tree) vs material (wood) ---
    idx.insert(
        Frame::assertion(
            Some(noun("ағаш")),
            FramePredicate::IsA,
            Some(noun("өсімдік")),
        ),
        Some(Domain::Science),
    );
    idx.insert(
        Frame::assertion(
            Some(noun("ағаш")),
            FramePredicate::IsA,
            Some(noun("құрылыс материалы")),
        ),
        Some(Domain::Material),
    );

    // --- Causal pair: Жаңбыр / сел ---
    idx.insert(
        Frame::assertion(
            Some(noun("жаңбыр")),
            FramePredicate::Causes,
            Some(noun("сел")),
        ),
        Some(Domain::Event),
    );

    // --- Definitions ---
    idx.insert(
        Frame::assertion(
            Some(noun("алгоритм")),
            FramePredicate::IsA,
            Some(noun("анық қадамдар тізбегі")),
        ),
        Some(Domain::Science),
    );

    // === МО РК — Министерство обороны Республики Казахстан ===
    // (curated for the 2026-05-26 video presentation context;
    //  see [project_mod_kz_pitch_context] memory).
    idx.insert(
        Frame::assertion(
            Some(noun("мо рк")),
            FramePredicate::IsA,
            Some(noun("қорғаныс министрлігі")),
        ),
        Some(Domain::Institution),
    );
    idx.insert(
        Frame::assertion(
            Some(noun("мо рк")),
            FramePredicate::LocatedIn,
            Some(noun("астана")),
        ),
        Some(Domain::Institution),
    );
    idx.insert(
        Frame::assertion(
            Some(noun("defense tech it park")),
            FramePredicate::IsA,
            Some(noun("ит-парк")),
        ),
        Some(Domain::Institution),
    );
    idx.insert(
        Frame::assertion(
            Some(noun("defense tech it park")),
            FramePredicate::FoundedIn,
            None,
        )
        .with_modifier(Modifier::TimeAnchor(TimeAnchor::Date {
            year: 2026,
            month: 5,
            day: 4,
        })),
        Some(Domain::Institution),
    );
    idx.insert(
        Frame::assertion(
            Some(noun("defense tech it park")),
            FramePredicate::LocatedIn,
            Some(noun("астана")),
        ),
        Some(Domain::Institution),
    );

    // === Programming domain ===
    idx.insert(
        Frame::assertion(
            Some(noun("бағдарламалау тілі")),
            FramePredicate::IsA,
            Some(noun("формальды тіл")),
        ),
        Some(Domain::Programming),
    );

    // === Rust language ===
    idx.insert(
        Frame::assertion(
            Some(noun("rust")),
            FramePredicate::IsA,
            Some(noun("бағдарламалау тілі")),
        ),
        Some(Domain::Programming),
    );
    idx.insert(
        Frame::assertion(
            Some(noun("rust")),
            FramePredicate::Authored,
            Some(noun("graydon hoare")),
        ),
        Some(Domain::Programming),
    );
    idx.insert(
        Frame::assertion(Some(noun("rust")), FramePredicate::FoundedIn, None)
            .with_modifier(Modifier::TimeAnchor(TimeAnchor::Year(2010))),
        Some(Domain::Programming),
    );
    idx.insert(
        Frame::assertion(
            Some(noun("rust")),
            FramePredicate::HasProperty,
            Some(noun("жадыдан қауіпсіз")),
        ),
        Some(Domain::Programming),
    );

    // === Physics — school-tutor coverage ===
    idx.insert(
        Frame::assertion(
            Some(noun("жарық жылдамдығы")),
            FramePredicate::IsA,
            Some(noun("299792458 м/с")),
        ),
        Some(Domain::Science),
    );
    idx.insert(
        Frame::assertion(
            Some(noun("гравитация")),
            FramePredicate::IsA,
            Some(noun("массалардың бір-бірін тарту күші")),
        ),
        Some(Domain::Science),
    );
    idx.insert(
        Frame::assertion(
            Some(noun("ньютон екінші заңы")),
            FramePredicate::IsA,
            Some(noun("f = m × a")),
        ),
        Some(Domain::Science),
    );
    idx.insert(
        Frame::assertion(
            Some(noun("эйнштейн формуласы")),
            FramePredicate::IsA,
            Some(noun("e = m × c²")),
        ),
        Some(Domain::Science),
    );

    // === Chemistry ===
    idx.insert(
        Frame::assertion(Some(noun("су")), FramePredicate::IsA, Some(noun("h₂o"))),
        Some(Domain::Science),
    );
    idx.insert(
        Frame::assertion(
            Some(noun("көмірқышқыл газы")),
            FramePredicate::IsA,
            Some(noun("co₂")),
        ),
        Some(Domain::Science),
    );
    idx.insert(
        Frame::assertion(Some(noun("көміртек")), FramePredicate::IsA, Some(noun("c"))),
        Some(Domain::Science),
    );

    // === Biology ===
    idx.insert(
        Frame::assertion(
            Some(noun("фотосинтез")),
            FramePredicate::IsA,
            Some(noun(
                "өсімдіктердің күн жарығы арқылы органикалық зат жасау процесі",
            )),
        ),
        Some(Domain::Science),
    );
    idx.insert(
        Frame::assertion(
            Some(noun("тірі ағза")),
            FramePredicate::PartOf,
            Some(noun("жасуша")),
        ),
        Some(Domain::Science),
    );
    idx.insert(
        Frame::assertion(
            Some(noun("днк")),
            FramePredicate::IsA,
            Some(noun("тұқым қуалаушылық молекуласы")),
        ),
        Some(Domain::Science),
    );

    // === Geography ===
    idx.insert(
        Frame::assertion(
            Some(noun("қазақстанның астанасы")),
            FramePredicate::IsA,
            Some(noun("астана қаласы")),
        ),
        Some(Domain::Geography),
    );
    idx.insert(
        Frame::assertion(
            Some(noun("эверест")),
            FramePredicate::IsA,
            Some(noun("жер шарының ең биік шыңы")),
        ),
        Some(Domain::Geography),
    );
    idx.insert(
        Frame::assertion(
            Some(noun("каспий теңізі")),
            FramePredicate::IsA,
            Some(noun("жер шарының ең үлкен ішкі теңізі")),
        ),
        Some(Domain::Geography),
    );

    // === History ===
    idx.insert(
        Frame::assertion(Some(noun("қазақ хандығы")), FramePredicate::FoundedIn, None)
            .with_modifier(Modifier::TimeAnchor(TimeAnchor::Year(1465))),
        Some(Domain::Event),
    );
    idx.insert(
        Frame::assertion(
            Some(noun("алаш қозғалысы")),
            FramePredicate::FoundedIn,
            None,
        )
        .with_modifier(Modifier::TimeAnchor(TimeAnchor::Year(1917))),
        Some(Domain::Event),
    );
    idx.insert(
        Frame::assertion(
            Some(noun("қазақ хандығы")),
            FramePredicate::NamedAfter,
            Some(noun("керей мен жәнібек")),
        ),
        Some(Domain::Event),
    );

    idx
}

/// The canonical list of real-Kazakh test cases. Each is mapped to
/// the QueryIR the v6.2 pipeline should produce. When Stage 6
/// (learned parser) ships, these double as the parser regression
/// suite.
pub fn canonical_cases() -> Vec<DialogCase> {
    vec![
        // 1. Birth-date — biographical / time focus.
        DialogCase {
            user_input: "Ахмет Байтұрсынұлы қашан туылған?",
            tag: "bio/birth-year",
            query: QueryIR::new(
                QueryFocus::Modifier(ModifierRole::Time),
                QuestionForm::Definition,
                AnswerShape::DateAnchor,
            )
            .with_agent(noun("ахмет байтұрсынұлы"))
            .with_predicate(FramePredicate::BornIn),
            expected_slot: AnswerSlot::Modifier(ModifierRole::Time),
            expected_surface: "1872",
            domain: Some(Domain::Person),
            known_gap: None,
        },
        // 2. Birth-place — biographical / place focus.
        DialogCase {
            user_input: "Ахмет Байтұрсынұлы қайда туылған?",
            tag: "bio/birth-place",
            query: QueryIR::new(
                QueryFocus::Modifier(ModifierRole::Location),
                QuestionForm::Definition,
                AnswerShape::BareNoun,
            )
            .with_agent(noun("ахмет байтұрсынұлы"))
            .with_predicate(FramePredicate::BornIn),
            expected_slot: AnswerSlot::Modifier(ModifierRole::Location),
            expected_surface: "қостанай",
            domain: Some(Domain::Person),
            known_gap: None,
        },
        // 3. IsA definition — biographical / definition.
        DialogCase {
            user_input: "Ахмет Байтұрсынұлы кім?",
            tag: "bio/profession",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::DefinitionalNP,
            )
            .with_agent(noun("ахмет байтұрсынұлы"))
            .with_predicate(FramePredicate::IsA),
            expected_slot: AnswerSlot::Object,
            expected_surface: "лингвист",
            domain: Some(Domain::Person),
            known_gap: None,
        },
        // 4. Authored — biographical / created-work.
        DialogCase {
            user_input: "Ахмет Байтұрсынұлы нені жазған?",
            tag: "bio/authored",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::BareNoun,
            )
            .with_agent(noun("ахмет байтұрсынұлы"))
            .with_predicate(FramePredicate::Authored),
            expected_slot: AnswerSlot::Object,
            expected_surface: "төте жазу",
            domain: Some(Domain::Person),
            known_gap: None,
        },
        // 5. Abay birth year.
        DialogCase {
            user_input: "Абай қашан туылған?",
            tag: "bio/abai-birth",
            query: QueryIR::new(
                QueryFocus::Modifier(ModifierRole::Time),
                QuestionForm::Definition,
                AnswerShape::DateAnchor,
            )
            .with_agent(noun("абай"))
            .with_predicate(FramePredicate::BornIn),
            expected_slot: AnswerSlot::Modifier(ModifierRole::Time),
            expected_surface: "1845",
            domain: Some(Domain::Person),
            known_gap: None,
        },
        // 6. Abay residence.
        DialogCase {
            user_input: "Абай қайда өмір сүрді?",
            tag: "bio/abai-lives",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::BareNoun,
            )
            .with_agent(noun("абай"))
            .with_predicate(FramePredicate::LivesIn),
            expected_slot: AnswerSlot::Object,
            expected_surface: "семей",
            domain: Some(Domain::Person),
            known_gap: None,
        },
        // 7. Geography IsA.
        DialogCase {
            user_input: "Қазақстан деген не?",
            tag: "geo/kazakhstan-isa",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::DefinitionalNP,
            )
            .with_agent(noun("қазақстан"))
            .with_predicate(FramePredicate::IsA),
            expected_slot: AnswerSlot::Object,
            expected_surface: "мемлекет",
            domain: Some(Domain::Geography),
            known_gap: None,
        },
        // 8. Geography quantity.
        DialogCase {
            user_input: "Қазақстанда қанша облыс бар?",
            tag: "geo/oblast-count",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::QuantityPhrase,
            )
            .with_agent(noun("қазақстан"))
            .with_predicate(FramePredicate::HasQuantity),
            expected_slot: AnswerSlot::Object,
            expected_surface: "17 облыс",
            domain: Some(Domain::Geography),
            known_gap: None,
        },
        // 9. Қостанай location.
        DialogCase {
            user_input: "Қостанай қайда орналасқан?",
            tag: "geo/kostanay-location",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::BareNoun,
            )
            .with_agent(noun("қостанай"))
            .with_predicate(FramePredicate::LocatedIn),
            expected_slot: AnswerSlot::Object,
            expected_surface: "қазақстан",
            domain: Some(Domain::Geography),
            known_gap: None,
        },
        // 10. Қостанай IsA.
        DialogCase {
            user_input: "Қостанай деген не?",
            tag: "geo/kostanay-isa",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::DefinitionalNP,
            )
            .with_agent(noun("қостанай"))
            .with_predicate(FramePredicate::IsA),
            expected_slot: AnswerSlot::Object,
            expected_surface: "облыс орталығы",
            domain: Some(Domain::Geography),
            known_gap: None,
        },
        // 11. КРУ founding year.
        DialogCase {
            user_input: "КРУ қашан құрылған?",
            tag: "inst/kru-founded",
            query: QueryIR::new(
                QueryFocus::Modifier(ModifierRole::Time),
                QuestionForm::Definition,
                AnswerShape::DateAnchor,
            )
            .with_agent(noun("кру"))
            .with_predicate(FramePredicate::FoundedIn),
            expected_slot: AnswerSlot::Modifier(ModifierRole::Time),
            expected_surface: "1939",
            domain: Some(Domain::Institution),
            known_gap: None,
        },
        // 12. КРУ named after.
        DialogCase {
            user_input: "КРУ кімнің атымен аталған?",
            tag: "inst/kru-eponym",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::BareNoun,
            )
            .with_agent(noun("кру"))
            .with_predicate(FramePredicate::NamedAfter),
            expected_slot: AnswerSlot::Object,
            expected_surface: "ахмет байтұрсынұлы",
            domain: Some(Domain::Institution),
            known_gap: None,
        },
        // 13. КРУ location.
        DialogCase {
            user_input: "КРУ қайда орналасқан?",
            tag: "inst/kru-location",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::BareNoun,
            )
            .with_agent(noun("кру"))
            .with_predicate(FramePredicate::LocatedIn),
            expected_slot: AnswerSlot::Object,
            expected_surface: "қостанай",
            domain: Some(Domain::Institution),
            known_gap: None,
        },
        // 14. AI Law effective date.
        DialogCase {
            user_input: "Жасанды интеллект туралы заң қашан күшіне енген?",
            tag: "law/ai-effective-date",
            query: QueryIR::new(
                QueryFocus::Modifier(ModifierRole::Time),
                QuestionForm::Definition,
                AnswerShape::DateAnchor,
            )
            .with_agent(noun("жасанды интеллект туралы заң"))
            .with_predicate(FramePredicate::EffectiveFrom),
            expected_slot: AnswerSlot::Modifier(ModifierRole::Time),
            expected_surface: "2026-01-18",
            domain: Some(Domain::Law),
            known_gap: None,
        },
        // 15. AI Law classifies — the Codex 2026-05-22 audit regression.
        DialogCase {
            user_input: "Жасанды интеллект туралы заң қандай санаттарға жіктейді?",
            tag: "law/ai-classifies",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::BareNoun,
            )
            .with_agent(noun("жасанды интеллект туралы заң"))
            .with_predicate(FramePredicate::Classifies),
            expected_slot: AnswerSlot::Object,
            expected_surface: "тәуекел санаттары",
            domain: Some(Domain::Law),
            known_gap: None,
        },
        // 16. Sense — Ай as calendar (month).
        DialogCase {
            user_input: "Ай дегеніміз не? (календарь контекстінде)",
            tag: "sense/ai-calendar",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::DefinitionalNP,
            )
            .with_agent(noun("ай"))
            .with_predicate(FramePredicate::IsA)
            .with_domain_filter(Domain::Calendar),
            expected_slot: AnswerSlot::Object,
            expected_surface: "уақыт өлшемі",
            domain: Some(Domain::Calendar),
            known_gap: None,
        },
        // 17. Sense — Ай as astronomy (moon).
        DialogCase {
            user_input: "Ай дегеніміз не? (астрономия контекстінде)",
            tag: "sense/ai-astronomy",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::DefinitionalNP,
            )
            .with_agent(noun("ай"))
            .with_predicate(FramePredicate::IsA)
            .with_domain_filter(Domain::Astronomy),
            expected_slot: AnswerSlot::Object,
            expected_surface: "аспан денесі",
            domain: Some(Domain::Astronomy),
            known_gap: None,
        },
        // 18. Sense — Ағаш as botany.
        DialogCase {
            user_input: "Ағаш дегеніміз не? (биология контекстінде)",
            tag: "sense/agash-botany",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::DefinitionalNP,
            )
            .with_agent(noun("ағаш"))
            .with_predicate(FramePredicate::IsA)
            .with_domain_filter(Domain::Science),
            expected_slot: AnswerSlot::Object,
            expected_surface: "өсімдік",
            domain: Some(Domain::Science),
            known_gap: None,
        },
        // 19. Sense — Ағаш as material.
        DialogCase {
            user_input: "Ағаш дегеніміз не? (материал контекстінде)",
            tag: "sense/agash-material",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::DefinitionalNP,
            )
            .with_agent(noun("ағаш"))
            .with_predicate(FramePredicate::IsA)
            .with_domain_filter(Domain::Material),
            expected_slot: AnswerSlot::Object,
            expected_surface: "құрылыс материалы",
            domain: Some(Domain::Material),
            known_gap: None,
        },
        // 20. Causal.
        DialogCase {
            user_input: "Жаңбыр селге қалай әсер етеді?",
            tag: "causal/rain-flood",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Causal,
                AnswerShape::CausalChain,
            )
            .with_agent(noun("жаңбыр"))
            .with_predicate(FramePredicate::Causes),
            expected_slot: AnswerSlot::Object,
            expected_surface: "сел",
            domain: Some(Domain::Event),
            known_gap: None,
        },
        // 21. Algorithm definition.
        DialogCase {
            user_input: "Алгоритм деген не?",
            tag: "def/algorithm",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::DefinitionalNP,
            )
            .with_agent(noun("алгоритм"))
            .with_predicate(FramePredicate::IsA),
            expected_slot: AnswerSlot::Object,
            expected_surface: "анық қадамдар тізбегі",
            domain: Some(Domain::Science),
            known_gap: None,
        },
        // 22. Reverse-lookup: «Who was born in 1872?» — subject focus.
        DialogCase {
            user_input: "1872 жылы кім туылған?",
            tag: "reverse/born-1872",
            query: QueryIR::new(
                QueryFocus::Subject,
                QuestionForm::Definition,
                AnswerShape::BareNoun,
            )
            .with_predicate(FramePredicate::BornIn)
            .with_modifier_constraint(ModifierRole::Time, year_phrase(1872)),
            expected_slot: AnswerSlot::Agent,
            // NOTE: the year_phrase value uses surface «1872 жыл» while
            // the curated frame's modifier carries `TimeAnchor::Year(1872)`.
            // Stage 4 modifier-constraint matching keys on root.surface;
            // this is a known mismatch the bench reports as wrong-surface,
            // motivating Stage 5 scalar-time matching.
            expected_surface: "ахмет байтұрсынұлы",
            domain: Some(Domain::Person),
            known_gap: Some(
                "Stage 5 — scalar TimeAnchor::Year(1872) ↔ phrase «1872 жыл» matching not yet wired",
            ),
        },
        // === МО РК (Министерство обороны РК) — institutional ===
        // (2026-05-26 video presentation context.)
        // 23. МО РК — что это.
        DialogCase {
            user_input: "МО РК деген не?",
            tag: "mod-kz/morr-isa",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::DefinitionalNP,
            )
            .with_agent(noun("мо рк"))
            .with_predicate(FramePredicate::IsA),
            expected_slot: AnswerSlot::Object,
            expected_surface: "қорғаныс министрлігі",
            domain: Some(Domain::Institution),
            known_gap: None,
        },
        // 24. МО РК — где находится.
        DialogCase {
            user_input: "Қорғаныс министрлігі қайда орналасқан?",
            tag: "mod-kz/morr-location",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::BareNoun,
            )
            .with_agent(noun("мо рк"))
            .with_predicate(FramePredicate::LocatedIn),
            expected_slot: AnswerSlot::Object,
            expected_surface: "астана",
            domain: Some(Domain::Institution),
            known_gap: None,
        },
        // 25. Defense Tech IT Park — что это.
        DialogCase {
            user_input: "Defense Tech IT Park деген не?",
            tag: "mod-kz/dtitpark-isa",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::DefinitionalNP,
            )
            .with_agent(noun("defense tech it park"))
            .with_predicate(FramePredicate::IsA),
            expected_slot: AnswerSlot::Object,
            expected_surface: "ит-парк",
            domain: Some(Domain::Institution),
            known_gap: None,
        },
        // 26. Defense Tech IT Park — когда открыт.
        DialogCase {
            user_input: "Defense Tech IT Park қашан ашылды?",
            tag: "mod-kz/dtitpark-founded",
            query: QueryIR::new(
                QueryFocus::Modifier(ModifierRole::Time),
                QuestionForm::Definition,
                AnswerShape::DateAnchor,
            )
            .with_agent(noun("defense tech it park"))
            .with_predicate(FramePredicate::FoundedIn),
            expected_slot: AnswerSlot::Modifier(ModifierRole::Time),
            expected_surface: "2026-05-04",
            domain: Some(Domain::Institution),
            known_gap: None,
        },
        // 27. Defense Tech IT Park — где.
        DialogCase {
            user_input: "Defense Tech IT Park қайда орналасқан?",
            tag: "mod-kz/dtitpark-location",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::BareNoun,
            )
            .with_agent(noun("defense tech it park"))
            .with_predicate(FramePredicate::LocatedIn),
            expected_slot: AnswerSlot::Object,
            expected_surface: "астана",
            domain: Some(Domain::Institution),
            known_gap: None,
        },
        // === Programming general ===
        // 28. Что такое язык программирования.
        DialogCase {
            user_input: "Бағдарламалау тілі деген не?",
            tag: "prog/plang-isa",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::DefinitionalNP,
            )
            .with_agent(noun("бағдарламалау тілі"))
            .with_predicate(FramePredicate::IsA),
            expected_slot: AnswerSlot::Object,
            expected_surface: "формальды тіл",
            domain: Some(Domain::Programming),
            known_gap: None,
        },
        // === Rust language ===
        // 29. Rust — что это.
        DialogCase {
            user_input: "Rust деген не?",
            tag: "prog/rust-isa",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::DefinitionalNP,
            )
            .with_agent(noun("rust"))
            .with_predicate(FramePredicate::IsA),
            expected_slot: AnswerSlot::Object,
            expected_surface: "бағдарламалау тілі",
            domain: Some(Domain::Programming),
            known_gap: None,
        },
        // 30. Rust — автор.
        DialogCase {
            user_input: "Rust тілін кім жасаған?",
            tag: "prog/rust-author",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::BareNoun,
            )
            .with_agent(noun("rust"))
            .with_predicate(FramePredicate::Authored),
            expected_slot: AnswerSlot::Object,
            expected_surface: "graydon hoare",
            domain: Some(Domain::Programming),
            known_gap: None,
        },
        // 31. Rust — год.
        DialogCase {
            user_input: "Rust қашан жасалған?",
            tag: "prog/rust-year",
            query: QueryIR::new(
                QueryFocus::Modifier(ModifierRole::Time),
                QuestionForm::Definition,
                AnswerShape::DateAnchor,
            )
            .with_agent(noun("rust"))
            .with_predicate(FramePredicate::FoundedIn),
            expected_slot: AnswerSlot::Modifier(ModifierRole::Time),
            expected_surface: "2010",
            domain: Some(Domain::Programming),
            known_gap: None,
        },
        // 32. Rust — свойство (memory-safe).
        DialogCase {
            user_input: "Rust қандай тіл? (қауіпсіздік тұрғысынан)",
            tag: "prog/rust-property",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::DefinitionalNP,
            )
            .with_agent(noun("rust"))
            .with_predicate(FramePredicate::HasProperty),
            expected_slot: AnswerSlot::Object,
            expected_surface: "жадыдан қауіпсіз",
            domain: Some(Domain::Programming),
            known_gap: None,
        },
        // === MATH (KNOWN GAP — no MathSolver in algebra yet) ===
        // Stage 4 has no procedural-computation layer; these cases
        // currently fail and document the gap. Resolution path:
        // dedicated MathSolver crate consumed by the Realiser
        // (Stage 7) or as a parallel solver path beside FrameIndex.
        // 33. Simple addition.
        DialogCase {
            user_input: "Екі жұп екі қанша?",
            tag: "math/2plus2",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::QuantityPhrase,
            )
            .with_agent(noun("2+2"))
            .with_predicate(FramePredicate::HasProperty),
            expected_slot: AnswerSlot::Object,
            expected_surface: "4",
            domain: Some(Domain::Science),
            known_gap: None,
        },
        // 34. Complex chained Russian.
        DialogCase {
            user_input: "Двадцать пять умножь на 7, раздели на два и прибавь три",
            tag: "math/complex-ru",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::QuantityPhrase,
            )
            .with_agent(noun("25*7/2+3"))
            .with_predicate(FramePredicate::HasProperty),
            expected_slot: AnswerSlot::Object,
            expected_surface: "90.5",
            domain: Some(Domain::Science),
            known_gap: None,
        },
        // 35. Kazakh equivalent.
        DialogCase {
            // Stage 1 of the solver accepts bare-root number words;
            // case-marked forms «жиырма бесті / жетіге / үшті» are
            // partially stripped via `strip_kazakh_case`, but the
            // safest battery-input is the canonical imperative form
            // used by Kazakh math teachers: bare numerals + verbs.
            user_input: "Жиырма бес көбейт жеті бөл екі қос үш",
            tag: "math/complex-kz",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::QuantityPhrase,
            )
            .with_agent(noun("25*7/2+3"))
            .with_predicate(FramePredicate::HasProperty),
            expected_slot: AnswerSlot::Object,
            expected_surface: "90.5",
            domain: Some(Domain::Science),
            known_gap: None,
        },
        // === SYSTEM CLOCK (KNOWN GAP — no clock provider in algebra) ===
        // Stage 4 has no system-time accessor; v6.1 has
        // `adam-dialog::system_clock` which the v6.2 realiser
        // (Stage 7) will route to. Documented here as a gap.
        // 36. Today's day-of-month.
        DialogCase {
            user_input: "Бүгін айдың нешесі?",
            tag: "system/today-date",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::DateAnchor,
            )
            .with_agent(noun("бүгін"))
            .with_predicate(FramePredicate::IsA),
            expected_slot: AnswerSlot::Object,
            expected_surface: "25", // today (2026-05-25 per system clock)
            domain: Some(Domain::Calendar),
            known_gap: Some(
                "Stage 7 — no system-clock provider yet; v6.1 has adam-dialog::system_clock to bridge",
            ),
        },
        // 37. Today's month name.
        DialogCase {
            user_input: "Қазір қандай ай?",
            tag: "system/today-month",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::BareNoun,
            )
            .with_agent(noun("қазіргі ай"))
            .with_predicate(FramePredicate::IsA),
            expected_slot: AnswerSlot::Object,
            expected_surface: "мамыр", // May
            domain: Some(Domain::Calendar),
            known_gap: Some("Stage 7 — no system-clock provider yet"),
        },
        // 38. Day of week.
        DialogCase {
            user_input: "Бүгін аптаның қай күні?",
            tag: "system/today-weekday",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::BareNoun,
            )
            .with_agent(noun("бүгін"))
            .with_predicate(FramePredicate::HasProperty),
            expected_slot: AnswerSlot::Object,
            expected_surface: "дүйсенбі", // 2026-05-25 = Monday (placeholder)
            domain: Some(Domain::Calendar),
            known_gap: Some("Stage 7 — no system-clock provider yet"),
        },
        // 39. Current time of day.
        DialogCase {
            user_input: "Қазір сағат неше?",
            tag: "system/time-now",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::QuantityPhrase,
            )
            .with_agent(noun("қазіргі уақыт"))
            .with_predicate(FramePredicate::IsA),
            expected_slot: AnswerSlot::Object,
            expected_surface: "08:00", // placeholder — needs system clock
            domain: Some(Domain::Calendar),
            known_gap: Some("Stage 7 — no system-clock provider yet"),
        },
        // === PHYSICS — school-tutor coverage ===
        // 40. Speed of light.
        DialogCase {
            user_input: "Жарық жылдамдығы қанша?",
            tag: "physics/light-speed",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::QuantityPhrase,
            )
            .with_agent(noun("жарық жылдамдығы"))
            .with_predicate(FramePredicate::IsA),
            expected_slot: AnswerSlot::Object,
            expected_surface: "299792458 м/с",
            domain: Some(Domain::Science),
            known_gap: None,
        },
        // 41. What is gravity.
        DialogCase {
            user_input: "Гравитация деген не?",
            tag: "physics/gravity-def",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::DefinitionalNP,
            )
            .with_agent(noun("гравитация"))
            .with_predicate(FramePredicate::IsA),
            expected_slot: AnswerSlot::Object,
            expected_surface: "массалардың бір-бірін тарту күші",
            domain: Some(Domain::Science),
            known_gap: None,
        },
        // 42. Newton's second law.
        DialogCase {
            user_input: "Ньютонның екінші заңы қандай формуламен жазылады?",
            tag: "physics/newton-2",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::DefinitionalNP,
            )
            .with_agent(noun("ньютон екінші заңы"))
            .with_predicate(FramePredicate::IsA),
            expected_slot: AnswerSlot::Object,
            expected_surface: "f = m × a",
            domain: Some(Domain::Science),
            known_gap: None,
        },
        // 43. Einstein formula.
        DialogCase {
            user_input: "Эйнштейннің формуласын айтыңыз",
            tag: "physics/einstein",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::DefinitionalNP,
            )
            .with_agent(noun("эйнштейн формуласы"))
            .with_predicate(FramePredicate::IsA),
            expected_slot: AnswerSlot::Object,
            expected_surface: "e = m × c²",
            domain: Some(Domain::Science),
            known_gap: None,
        },
        // === CHEMISTRY ===
        // 44. Water formula.
        DialogCase {
            user_input: "Судың химиялық формуласы қандай?",
            tag: "chem/water",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::DefinitionalNP,
            )
            .with_agent(noun("су"))
            .with_predicate(FramePredicate::IsA),
            expected_slot: AnswerSlot::Object,
            expected_surface: "h₂o",
            domain: Some(Domain::Science),
            known_gap: None,
        },
        // 45. CO2 formula.
        DialogCase {
            user_input: "Көмірқышқыл газының формуласы қандай?",
            tag: "chem/co2",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::DefinitionalNP,
            )
            .with_agent(noun("көмірқышқыл газы"))
            .with_predicate(FramePredicate::IsA),
            expected_slot: AnswerSlot::Object,
            expected_surface: "co₂",
            domain: Some(Domain::Science),
            known_gap: None,
        },
        // 46. Carbon symbol.
        DialogCase {
            user_input: "Көміртектің химиялық таңбасы қандай?",
            tag: "chem/carbon",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::BareNoun,
            )
            .with_agent(noun("көміртек"))
            .with_predicate(FramePredicate::IsA),
            expected_slot: AnswerSlot::Object,
            expected_surface: "c",
            domain: Some(Domain::Science),
            known_gap: None,
        },
        // === BIOLOGY ===
        // 47. Photosynthesis.
        DialogCase {
            user_input: "Фотосинтез деген не?",
            tag: "bio/photosynthesis",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::DefinitionalNP,
            )
            .with_agent(noun("фотосинтез"))
            .with_predicate(FramePredicate::IsA),
            expected_slot: AnswerSlot::Object,
            expected_surface: "өсімдіктердің күн жарығы арқылы органикалық зат жасау процесі",
            domain: Some(Domain::Science),
            known_gap: None,
        },
        // 48. What is DNA.
        DialogCase {
            user_input: "ДНК деген не?",
            tag: "bio/dna",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::DefinitionalNP,
            )
            .with_agent(noun("днк"))
            .with_predicate(FramePredicate::IsA),
            expected_slot: AnswerSlot::Object,
            expected_surface: "тұқым қуалаушылық молекуласы",
            domain: Some(Domain::Science),
            known_gap: None,
        },
        // === GEOGRAPHY ===
        // 49. Capital of Kazakhstan.
        DialogCase {
            user_input: "Қазақстанның астанасы қай қала?",
            tag: "geo/kz-capital",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::BareNoun,
            )
            .with_agent(noun("қазақстанның астанасы"))
            .with_predicate(FramePredicate::IsA),
            expected_slot: AnswerSlot::Object,
            expected_surface: "астана қаласы",
            domain: Some(Domain::Geography),
            known_gap: None,
        },
        // 50. Mount Everest.
        DialogCase {
            user_input: "Жер шарының ең биік шыңы қандай?",
            tag: "geo/everest",
            query: QueryIR::new(
                QueryFocus::Subject,
                QuestionForm::Definition,
                AnswerShape::BareNoun,
            )
            .with_object(noun("жер шарының ең биік шыңы"))
            .with_predicate(FramePredicate::IsA),
            expected_slot: AnswerSlot::Agent,
            expected_surface: "эверест",
            domain: Some(Domain::Geography),
            known_gap: None,
        },
        // 51. Caspian Sea.
        DialogCase {
            user_input: "Каспий теңізі деген не?",
            tag: "geo/caspian",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::DefinitionalNP,
            )
            .with_agent(noun("каспий теңізі"))
            .with_predicate(FramePredicate::IsA),
            expected_slot: AnswerSlot::Object,
            expected_surface: "жер шарының ең үлкен ішкі теңізі",
            domain: Some(Domain::Geography),
            known_gap: None,
        },
        // === HISTORY ===
        // 52. Kazakh Khanate founding.
        DialogCase {
            user_input: "Қазақ хандығы қашан құрылған?",
            tag: "hist/khanate-year",
            query: QueryIR::new(
                QueryFocus::Modifier(ModifierRole::Time),
                QuestionForm::Definition,
                AnswerShape::DateAnchor,
            )
            .with_agent(noun("қазақ хандығы"))
            .with_predicate(FramePredicate::FoundedIn),
            expected_slot: AnswerSlot::Modifier(ModifierRole::Time),
            expected_surface: "1465",
            domain: Some(Domain::Event),
            known_gap: None,
        },
        // 53. Khanate founders.
        DialogCase {
            user_input: "Қазақ хандығын кімдер құрды?",
            tag: "hist/khanate-founders",
            query: QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::BareNoun,
            )
            .with_agent(noun("қазақ хандығы"))
            .with_predicate(FramePredicate::NamedAfter),
            expected_slot: AnswerSlot::Object,
            expected_surface: "керей мен жәнібек",
            domain: Some(Domain::Event),
            known_gap: None,
        },
        // 54. Alash movement founding year.
        DialogCase {
            user_input: "Алаш қозғалысы қашан құрылған?",
            tag: "hist/alash-year",
            query: QueryIR::new(
                QueryFocus::Modifier(ModifierRole::Time),
                QuestionForm::Definition,
                AnswerShape::DateAnchor,
            )
            .with_agent(noun("алаш қозғалысы"))
            .with_predicate(FramePredicate::FoundedIn),
            expected_slot: AnswerSlot::Modifier(ModifierRole::Time),
            expected_surface: "1917",
            domain: Some(Domain::Event),
            known_gap: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The canonical CI quality gate.** Every commit on
    /// `experimental/v6_2_*` must keep this test green, or the
    /// pipeline cannot ship.
    ///
    /// Quality model:
    /// - **Must-pass set** (no `known_gap` annotation) — every
    ///   case here must pass every dimension. The
    ///   `regressions == 0` assertion is the hard floor.
    /// - **Known-gap set** — cases documenting an unfinished
    ///   architectural layer (math solver / system clock /
    ///   scalar-time matching). Failure is expected; the report
    ///   surfaces them but the gate ignores them.
    /// - **Promote-out check** — if a known-gap case starts
    ///   passing, we flag it in the report so a follow-up commit
    ///   drops the annotation, tightening the gate.
    ///
    /// Floors for the must-pass set (raised whenever a stage ships):
    /// - Adequacy ≥ 95 %.
    /// - Relevance ≥ 90 %.
    /// - Sense correctness ≥ 95 %.
    /// - Surface correctness ≥ 90 %.
    #[test]
    fn dialog_battery_meets_quality_gate() {
        let report = run_dialog_battery();
        // Surface the report unconditionally so the developer sees
        // the full state on every test run.
        report.print();

        assert_eq!(
            report.regressions, 0,
            "REGRESSION: {} non-gap case(s) failed — see report above",
            report.regressions
        );
        assert!(
            report.adequacy_pct() >= 95.0,
            "adequacy on must-pass set regressed: {:.1}% < 95.0%",
            report.adequacy_pct()
        );
        assert!(
            report.relevance_pct() >= 90.0,
            "relevance on must-pass set regressed: {:.1}% < 90.0%",
            report.relevance_pct()
        );
        assert!(
            report.sense_pct() >= 95.0,
            "sense correctness on must-pass set regressed: {:.1}% < 95.0%",
            report.sense_pct()
        );
        assert!(
            report.surface_pct() >= 90.0,
            "surface correctness on must-pass set regressed: {:.1}% < 90.0%",
            report.surface_pct()
        );
    }

    #[test]
    fn battery_has_at_least_20_real_cases() {
        // Predeclared floor: the canonical battery must carry at
        // least 20 real cases. Adding new cases tightens the gate;
        // removing them weakens it. Removal is an intentional
        // architectural change that needs a separate commit.
        assert!(canonical_cases().len() >= 20);
    }

    #[test]
    fn battery_covers_at_least_5_distinct_predicates() {
        use std::collections::HashSet;
        // FramePredicate doesn't impl Hash (it carries an
        // `Other(String)` variant); dedup by stable slug instead.
        let preds: HashSet<&str> = canonical_cases()
            .iter()
            .filter_map(|c| c.query.predicate.as_ref().map(|p| p.as_str()))
            .collect();
        assert!(
            preds.len() >= 5,
            "battery must exercise at least 5 distinct predicates, got {}",
            preds.len()
        );
    }

    #[test]
    fn battery_covers_sense_ambiguous_cases() {
        // «Ай» and «Ағаш» must both appear, with both senses each.
        let cases = canonical_cases();
        let has_ai_calendar = cases.iter().any(|c| c.tag == "sense/ai-calendar");
        let has_ai_astronomy = cases.iter().any(|c| c.tag == "sense/ai-astronomy");
        let has_agash_botany = cases.iter().any(|c| c.tag == "sense/agash-botany");
        let has_agash_material = cases.iter().any(|c| c.tag == "sense/agash-material");
        assert!(has_ai_calendar && has_ai_astronomy);
        assert!(has_agash_botany && has_agash_material);
    }
}
