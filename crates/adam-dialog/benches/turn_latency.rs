//! v4.4.7 — per-turn latency benchmarks.
//!
//! Six categories sized to the cognitive contour they exercise:
//!
//! - `social_greeting` — bare `сәлем`. Cheapest path: greeting
//!   detector matches, no FST parse needed for the slot extractor,
//!   no retrieval, no reasoning.
//! - `profile_statement` — `мен Алматыда тұрамын`. FST parse +
//!   `detect_statement_of_location` + entity absorption +
//!   `statement_of_location` template.
//! - `profile_recall` — preloaded conversation answers
//!   `мен қайда тұрамын?` via `ask_location.with_known_user`.
//! - `knowledge_query` — `Қазақстан туралы айтшы`. Topic extraction
//!   + `SearchGraph` tool dispatch + retrieval composition.
//! - `contradiction_check` — preloaded conflict answers turn 2 with
//!   the v4.4.5 `check_contradiction` template family.
//! - `dismiss_contradiction` — preloaded conflict answers `білмеймін`
//!   with the v4.4.0 dismissal pathway.
//!
//! Each scenario constructs the full `Conversation` runtime per
//! benchmark iteration so that the measured cost is the steady-state
//! per-turn work, *not* amortised lexicon / template / index loads.
//! For cold-start cost see the `cold_start` group at the bottom.
//!
//! Run with `cargo bench -p adam-dialog --bench turn_latency`. Numbers
//! reported in `docs/performance.md` are from a baseline M2 8 GB run
//! with `--release` defaults; re-run on the same hardware before
//! editing those numbers in the doc.

use std::path::Path;

use adam_dialog::{Conversation, TemplateRepository};
use adam_kernel_fst::lexicon::LexiconV1;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

const LEXICON_CURATED: &str = "../../data/tokenizer/segmentation_roots.json";
const LEXICON_APERTIUM: &str = "../../data/lexicon_v1/apertium_imported_roots.json";

fn require_lexicon() -> LexiconV1 {
    assert!(
        Path::new(LEXICON_CURATED).exists(),
        "turn_latency bench requires lexicon at {LEXICON_CURATED}"
    );
    LexiconV1::load(LEXICON_CURATED, LEXICON_APERTIUM).expect("lexicon must parse")
}

fn require_repo() -> TemplateRepository {
    TemplateRepository::load_default().expect("templates v1.toml must exist")
}

fn bench_single_turn(c: &mut Criterion, name: &str, input: &'static str) {
    let lex = require_lexicon();
    let repo = require_repo();
    c.bench_function(name, |b| {
        b.iter(|| {
            let mut conv = Conversation::new();
            black_box(conv.turn(black_box(input), &lex, &repo, 0));
        })
    });
}

fn social_greeting(c: &mut Criterion) {
    bench_single_turn(c, "social_greeting", "сәлем");
}

fn profile_statement(c: &mut Criterion) {
    bench_single_turn(c, "profile_statement", "мен Алматыда тұрамын");
}

fn profile_recall(c: &mut Criterion) {
    let lex = require_lexicon();
    let repo = require_repo();
    c.bench_function("profile_recall", |b| {
        b.iter(|| {
            let mut conv = Conversation::new();
            // Setup turn — not measured separately (each iteration
            // pays the same fixed setup cost; we report the
            // *combined* two-turn cost as the recall path's true
            // floor).
            conv.turn("Астанада тұрамын", &lex, &repo, 0);
            black_box(conv.turn(black_box("мен қайда тұрамын?"), &lex, &repo, 1));
        })
    });
}

fn knowledge_query(c: &mut Criterion) {
    bench_single_turn(c, "knowledge_query", "Қазақстан туралы айтшы");
}

fn contradiction_check(c: &mut Criterion) {
    let lex = require_lexicon();
    let repo = require_repo();
    c.bench_function("contradiction_check", |b| {
        b.iter(|| {
            let mut conv = Conversation::new();
            conv.turn("мен Астанада тұрамын", &lex, &repo, 0);
            black_box(conv.turn(black_box("мен Алматыда тұрамын"), &lex, &repo, 1));
        })
    });
}

fn dismiss_contradiction(c: &mut Criterion) {
    let lex = require_lexicon();
    let repo = require_repo();
    c.bench_function("dismiss_contradiction", |b| {
        b.iter(|| {
            let mut conv = Conversation::new();
            conv.turn("мен Астанада тұрамын", &lex, &repo, 0);
            conv.turn("мен Алматыда тұрамын", &lex, &repo, 1);
            black_box(conv.turn(black_box("білмеймін"), &lex, &repo, 2));
        })
    });
}

fn cold_start_lexicon(c: &mut Criterion) {
    c.bench_function("cold_start_lexicon", |b| {
        b.iter(|| {
            black_box(LexiconV1::load(LEXICON_CURATED, LEXICON_APERTIUM).expect("lexicon"));
        })
    });
}

fn cold_start_repo(c: &mut Criterion) {
    c.bench_function("cold_start_repo", |b| {
        b.iter(|| {
            black_box(TemplateRepository::load_default().expect("templates"));
        })
    });
}

fn cold_start_conversation(c: &mut Criterion) {
    c.bench_function("cold_start_conversation", |b| {
        b.iter(|| {
            black_box(Conversation::new());
        })
    });
}

criterion_group!(
    per_turn,
    social_greeting,
    profile_statement,
    profile_recall,
    knowledge_query,
    contradiction_check,
    dismiss_contradiction,
);
criterion_group!(
    cold_start,
    cold_start_lexicon,
    cold_start_repo,
    cold_start_conversation,
);
criterion_main!(per_turn, cold_start);
