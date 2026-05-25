// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Stage 3 latency benchmark — full neurosymbolic pipeline.
//!
//! Measures the end-to-end deterministic understanding pipeline:
//!
//! ```text
//! word lattice → Composition[] → Frame → QueryIR → match_frame(candidate) → AnswerSlot
//! ```
//!
//! For comparison context: a single GPT-class LLM inference call
//! over the network is typically 100–500 ms end-to-end (and 30–80 ms
//! on the host with a dedicated GPU). adam's deterministic pipeline
//! runs on a single CPU core with no model loaded — the entire
//! understanding step is pure typed-data manipulation.
//!
//! Two timing modes are reported:
//!
//! - **Total (cold)** — first iteration; includes lazy initialisation
//!   of the allocator, branch predictor caches, etc. Useful as an
//!   upper bound for a literal "first request after boot" latency.
//! - **Pure (warm)** — median over N iterations after a warmup pass.
//!   Useful for steady-state throughput; this is the number that
//!   compares directly with «per-token» LLM latency.
//!
//! Run: `cargo run --release --example bench_pipeline -p adam-algebra`

use std::time::Instant;

use adam_algebra::{
    AnswerShape, Composition, Domain, Frame, FrameIndex, FramePredicate, Modifier, ModifierRole,
    PartOfSpeech, QueryFocus, QueryIR, QuestionFocus, QuestionForm, Root, SuffixOp, TimeAnchor,
};
use adam_kernel_fst::morphotactics::{Case, Tense};

const WARMUP_ITERATIONS: usize = 100;
const MEASURE_ITERATIONS: usize = 10_000;

fn noun(surface: &str) -> Composition {
    Composition::identity(Root::new(surface, PartOfSpeech::Noun))
}

fn verb(surface: &str) -> Composition {
    Composition::identity(Root::new(surface, PartOfSpeech::Verb))
}

fn noun_with_case(surface: &str, case: Case) -> Composition {
    let mut c = noun(surface);
    c.operators.push(SuffixOp::Case(case));
    c
}

/// One realistic query the pipeline must handle.
/// «Ахмет Байтұрсынұлы 1872 жылы Қостанай облысында туылған.»
/// Question:    «Ахмет Байтұрсынұлы қашан туылған?»
struct Scenario {
    label: &'static str,
    lattice: Vec<Composition>,
    candidate: Frame,
    question_focus: QuestionFocus,
    expected_role: ModifierRole,
}

fn biographical_scenario() -> Scenario {
    // Lattice for: «Ахмет Байтұрсынұлы 1872 жылы туылған.»
    let agent = noun("ахмет байтұрсынұлы");
    let mut year_phrase = Composition::identity(Root::new("жыл", PartOfSpeech::Noun));
    year_phrase.operators.push(SuffixOp::Case(Case::Locative));
    let mut tuyl = verb("туыл");
    tuyl.operators.push(SuffixOp::Tense(Tense::PastEvidential));
    let lattice = vec![agent.clone(), year_phrase.clone(), tuyl];

    // Candidate fact in the knowledge graph.
    let candidate = Frame::assertion(Some(agent), FramePredicate::BornIn, None)
        .with_modifier(Modifier::TimeAnchor(TimeAnchor::Phrase(year_phrase)))
        .with_tense(Tense::PastEvidential);

    Scenario {
        label: "biographical / BornIn",
        lattice,
        candidate,
        question_focus: QuestionFocus::Time,
        expected_role: ModifierRole::Time,
    }
}

fn location_scenario() -> Scenario {
    // «Қостанай облысы — Қазақстанның бір бөлігі.»
    let agent = noun("қостанай облысы");
    let object = noun("қазақстан");
    let mut tense_verb = verb("орналас");
    tense_verb.operators.push(SuffixOp::Tense(Tense::Present));
    let lattice = vec![agent.clone(), object.clone(), tense_verb];

    let candidate = Frame::assertion(
        Some(agent),
        FramePredicate::LocatedIn,
        Some(noun_with_case("қазақстан", Case::Locative)),
    )
    .with_modifier(Modifier::Location(noun_with_case(
        "қазақстан",
        Case::Locative,
    )))
    .with_tense(Tense::Present);

    Scenario {
        label: "location / LocatedIn",
        lattice,
        candidate,
        question_focus: QuestionFocus::Place,
        expected_role: ModifierRole::Location,
    }
}

fn authored_scenario() -> Scenario {
    // «Ахмет «Ұлт мектебі» атты кітапты жазған.»
    let agent = noun("ахмет байтұрсынұлы");
    let book = noun_with_case("ұлт мектебі", Case::Accusative);
    let mut zhaz = verb("жаз");
    zhaz.operators.push(SuffixOp::Tense(Tense::PastEvidential));
    let lattice = vec![agent.clone(), book.clone(), zhaz];

    let candidate = Frame::assertion(Some(agent), FramePredicate::Authored, Some(book))
        .with_tense(Tense::PastEvidential);

    Scenario {
        label: "authored / Authored",
        lattice,
        candidate,
        question_focus: QuestionFocus::Object,
        expected_role: ModifierRole::Time, // unused for Object focus
    }
}

/// Run the full pipeline once on one scenario. Returns the answer-slot
/// found.
#[inline(always)]
fn run_pipeline(scenario: &Scenario) -> bool {
    use adam_algebra::{ContextSentenceType, SentenceContext};

    // 1. Lattice → Frame.
    let ctx = SentenceContext {
        sentence_type: ContextSentenceType::Question,
        question_focus: Some(scenario.question_focus),
    };
    let Some(frame) = Frame::from_morph_lattice_in_context(&scenario.lattice, ctx) else {
        return false;
    };

    // 2. Frame → QueryIR.
    let Some(query) = QueryIR::from_question_frame(&frame) else {
        return false;
    };

    // 3. QueryIR.match_frame(candidate).
    query.match_frame(&scenario.candidate).is_some()
}

fn main() {
    let scenarios = [
        biographical_scenario(),
        location_scenario(),
        authored_scenario(),
    ];

    println!("=== adam ARK Stage 3 latency benchmark ===");
    println!("(deterministic; CPU-only; no model loaded)");
    println!();
    println!("Pipeline: word-lattice → Composition[] → Frame → QueryIR → match_frame → AnswerSlot");
    println!();

    let mut overall_cold = 0u128;
    let mut overall_warm = Vec::<u128>::new();

    for scenario in &scenarios {
        println!("Scenario: {}", scenario.label);

        // ---- COLD (first iteration including caches warming up) ----
        let cold_start = Instant::now();
        let ok_cold = run_pipeline(scenario);
        let cold_ns = cold_start.elapsed().as_nanos();
        assert!(
            ok_cold,
            "pipeline must produce a match for {}",
            scenario.label
        );
        overall_cold += cold_ns;

        // ---- WARMUP ----
        for _ in 0..WARMUP_ITERATIONS {
            std::hint::black_box(run_pipeline(scenario));
        }

        // ---- WARM (per-iteration steady-state) ----
        let warm_start = Instant::now();
        let mut hits = 0usize;
        for _ in 0..MEASURE_ITERATIONS {
            if std::hint::black_box(run_pipeline(scenario)) {
                hits += 1;
            }
        }
        let warm_total_ns = warm_start.elapsed().as_nanos();
        let per_iter_ns = warm_total_ns / MEASURE_ITERATIONS as u128;
        overall_warm.push(per_iter_ns);

        assert_eq!(
            hits, MEASURE_ITERATIONS,
            "{} produced wrong matches",
            scenario.label
        );

        println!(
            "  COLD (1st call):   {:>9} ns  ({:.3} µs)",
            cold_ns,
            cold_ns as f64 / 1_000.0
        );
        println!(
            "  WARM (median of {}): {:>7} ns  ({:.3} µs)  per pipeline run",
            MEASURE_ITERATIONS,
            per_iter_ns,
            per_iter_ns as f64 / 1_000.0
        );
        let throughput_qps = 1_000_000_000.0 / per_iter_ns as f64;
        println!(
            "  Throughput:        {:>9.0} queries/sec on one core",
            throughput_qps
        );
        let _ = scenario.expected_role;
        println!();
    }

    let warm_sum: u128 = overall_warm.iter().sum();
    let warm_avg_ns = warm_sum / overall_warm.len() as u128;
    println!("=== Pipeline Summary ===");
    println!("Scenarios run: {}", scenarios.len());
    println!(
        "Total COLD time across scenarios: {} ns ({:.3} µs)",
        overall_cold,
        overall_cold as f64 / 1_000.0
    );
    println!(
        "Average WARM time per query:     {} ns ({:.3} µs)",
        warm_avg_ns,
        warm_avg_ns as f64 / 1_000.0
    );

    // === Stage 4 indexed retrieval bench ===
    println!();
    println!("=== Stage 4: indexed retrieval ===");
    println!("(deterministic; CPU-only; 0 MB model)");
    println!();
    bench_indexed_retrieval();

    // Reference point for the user.
    println!();
    println!("--- For comparison ---");
    println!("LLM inference (single forward pass):");
    println!("  - GPT-class API call (cloud):     ~100–500 ms  (≈ 100,000,000–500,000,000 ns)");
    println!("  - 7B model local GPU:             ~50–150 ms   (≈ 50,000,000–150,000,000 ns)");
    println!("  - 70B model local GPU:            ~200–600 ms");
    println!();
    let ratio = 100_000_000.0 / warm_avg_ns as f64; // vs 100ms LLM call
    println!(
        "adam ARK pipeline is ≈ {:.0}× faster than a fast (100 ms) LLM call.",
        ratio
    );
    println!("  (deterministic, CPU-only, 0 MB model — pure typed-data manipulation)");
}

/// Stage 4: build a 1k-frame index, run a representative QueryIR
/// many times, report per-query latency.
fn bench_indexed_retrieval() {
    let n = 1_000;
    let mut idx = FrameIndex::new();
    let preds = [
        FramePredicate::IsA,
        FramePredicate::LivesIn,
        FramePredicate::Has,
        FramePredicate::BornIn,
        FramePredicate::DiedIn,
        FramePredicate::Authored,
        FramePredicate::FoundedIn,
        FramePredicate::LocatedIn,
        FramePredicate::HasQuantity,
        FramePredicate::PartOf,
    ];
    let domains = [Domain::Person, Domain::Geography, Domain::Institution];
    for i in 0..n {
        let p = preds[i % preds.len()].clone();
        let mut year_phrase = noun(&format!("year_{}", i % 30));
        year_phrase.operators.push(SuffixOp::Case(Case::Locative));
        let frame = Frame::assertion(
            Some(noun(&format!("agent_{}", i % 100))),
            p,
            Some(noun(&format!("object_{}", i % 50))),
        )
        .with_modifier(Modifier::TimeAnchor(TimeAnchor::Phrase(year_phrase)));
        idx.insert(frame, Some(domains[i % domains.len()].clone()));
    }
    println!("Index populated: {} frames", idx.len());

    // Query 1: predicate-only — wide hit set (every 10th frame).
    let q_wide = QueryIR::new(
        QueryFocus::Subject,
        QuestionForm::Definition,
        AnswerShape::BareNoun,
    )
    .with_predicate(FramePredicate::BornIn);

    // Query 2: predicate + agent — narrow hit set (agent_0 +
    // IsA both have 10 entries each; intersection = 10).
    let q_narrow = QueryIR::new(
        QueryFocus::Modifier(ModifierRole::Time),
        QuestionForm::Definition,
        AnswerShape::DateAnchor,
    )
    .with_agent(noun("agent_0"))
    .with_predicate(FramePredicate::IsA);

    // Query 3: agent only — predicate variable.
    let q_agent_only = QueryIR::new(
        QueryFocus::Predicate,
        QuestionForm::Definition,
        AnswerShape::DefinitionalNP,
    )
    .with_agent(noun("agent_42"));

    let cases: &[(&str, &QueryIR)] = &[
        ("predicate-only  (wide)", &q_wide),
        ("predicate+agent (narrow)", &q_narrow),
        ("agent-only      (predicate variable)", &q_agent_only),
    ];

    for (label, q) in cases {
        // COLD
        let cold_start = Instant::now();
        let cold_hits = idx.query(q);
        let cold_ns = cold_start.elapsed().as_nanos();
        let hit_count = cold_hits.len();

        // WARMUP
        for _ in 0..WARMUP_ITERATIONS {
            std::hint::black_box(idx.query(q));
        }

        // WARM
        let warm_start = Instant::now();
        for _ in 0..MEASURE_ITERATIONS {
            std::hint::black_box(idx.query(q));
        }
        let warm_total = warm_start.elapsed().as_nanos();
        let per_iter = warm_total / MEASURE_ITERATIONS as u128;
        let qps = 1_000_000_000.0 / per_iter as f64;

        println!("Query: {}", label);
        println!("  Hits:              {}", hit_count);
        println!(
            "  COLD (1st call):   {:>9} ns  ({:.3} µs)",
            cold_ns,
            cold_ns as f64 / 1_000.0
        );
        println!(
            "  WARM (median of {}): {:>7} ns  ({:.3} µs)",
            MEASURE_ITERATIONS,
            per_iter,
            per_iter as f64 / 1_000.0
        );
        println!("  Throughput:        {:>9.0} queries/sec on one core", qps);
        println!();
    }
}
