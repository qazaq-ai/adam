// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! End-to-end pipeline test — exercises every stage of the
//! ingestion arc in sequence on a single tmpdir-isolated
//! corpus:
//!
//!   extract → store → validate → review → integrate → world_core
//!
//! The unit tests in each module cover the stage in
//! isolation; this integration test catches composition
//! bugs — e.g. an extractor confidence floor that always
//! routes into AutoReject, a validator note format the
//! reviewer trips over, an integrator ID prefix the
//! Cargo.toml workspace lints reject.

use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;

use adam_ingestion::{
    CandidateStore, IngestionStatus, IntegrationTarget, ReviewDecision, Reviewer, SourceKind,
    candidate::CandidateFact, extract_facts_from_text, integrate_approved_facts,
    integrator::load_world_core_entries, run_review_session, validate_fact,
    validator::WorldCoreIndex,
};

/// Scripted reviewer — pre-recorded FIFO decisions.
/// Defaults to Skip when exhausted so this test can't
/// deadlock if a stage stops emitting candidates as
/// expected.
struct ScriptedReviewer {
    decisions: VecDeque<ReviewDecision>,
    seen: Vec<String>,
}

impl ScriptedReviewer {
    fn new(seq: Vec<ReviewDecision>) -> Self {
        Self {
            decisions: seq.into(),
            seen: Vec::new(),
        }
    }
}

impl Reviewer for ScriptedReviewer {
    fn review_fact(&mut self, fact: &CandidateFact) -> ReviewDecision {
        self.seen.push(fact.id.clone());
        self.decisions.pop_front().unwrap_or(ReviewDecision::Skip)
    }
}

fn tmp_root(tag: &str) -> PathBuf {
    let tid = format!("{:?}", std::thread::current().id()).replace([':', ' ', '(', ')'], "_");
    let dir = std::env::temp_dir().join(format!("adam-e2e-{tag}-{tid}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn full_pipeline_extracts_validates_reviews_and_integrates() {
    let root = tmp_root("full");
    let queue_dir = root.join("queue");
    let world_core_path = root.join("colors_kz.jsonl");
    let store = CandidateStore::open(&queue_dir).expect("store open");

    // 1. Extract — synthetic Kazakh source with 4 colour
    //    declarations.  All match the em-dash pattern.
    let raw_text = "Қызыл — түс.\nКөк — түс.\nЖасыл — түс.\nСары — түс.";
    let extracted = extract_facts_from_text(
        raw_text,
        "data/raw/colors_kz.txt",
        SourceKind::TextFile,
        "2026-06-29",
    );
    assert_eq!(extracted.len(), 4, "extractor should produce 4 candidates");
    store.save_facts(&extracted).expect("seed queue");

    // 2. Validate — empty world_core index, so every
    //    candidate clears the duplicate / contradiction
    //    gates.  Extractor confidence is 0.7 → all land
    //    NeedsReview at the validator's default
    //    thresholds.
    let index = WorldCoreIndex::new();
    let mut loaded = store.load_facts().expect("load");
    for fact in &mut loaded {
        let outcome = validate_fact(fact, &index);
        fact.status = outcome.new_status;
        if !fact.notes.is_empty() {
            fact.notes.push_str("; ");
        }
        fact.notes.push_str(&outcome.note);
    }
    store.save_facts(&loaded).expect("write back");
    let post_validate = store.load_facts().expect("reload");
    assert!(
        post_validate
            .iter()
            .all(|f| f.status == IngestionStatus::NeedsReview),
        "all candidates should land NeedsReview at confidence 0.7, got: {:?}",
        post_validate.iter().map(|f| f.status).collect::<Vec<_>>()
    );

    // 3. Review — accept the first two, reject the third,
    //    quit before the fourth.  Tests that all branches
    //    of the reviewer fire across one session.
    let mut reviewer = ScriptedReviewer::new(vec![
        ReviewDecision::Approve,
        ReviewDecision::Approve,
        ReviewDecision::Reject,
        ReviewDecision::Quit,
    ]);
    let summary = run_review_session(&store, &mut reviewer).expect("session ok");
    // examined counts every candidate the reviewer was
    // shown — Approve + Approve + Reject + Quit = 4
    // (Quit increments examined before breaking the loop).
    assert_eq!(summary.examined, 4);
    assert_eq!(summary.approved, 2);
    assert_eq!(summary.rejected, 1);
    assert!(summary.quit);

    let post_review = store.load_facts().expect("reload");
    let approved_count = post_review
        .iter()
        .filter(|f| f.status == IngestionStatus::ApprovedByHuman)
        .count();
    let rejected_count = post_review
        .iter()
        .filter(|f| f.status == IngestionStatus::RejectedByHuman)
        .count();
    let still_pending = post_review
        .iter()
        .filter(|f| f.status == IngestionStatus::NeedsReview)
        .count();
    assert_eq!(approved_count, 2);
    assert_eq!(rejected_count, 1);
    assert_eq!(
        still_pending, 1,
        "the post-Quit candidate stays NeedsReview"
    );

    // 4. Integrate — both approved candidates land in the
    //    world_core file; rejected + still-pending stay
    //    out.  Status transitions to IntegratedIntoWorldCore
    //    for the two integrated entries.
    let target = IntegrationTarget {
        world_core_path: world_core_path.clone(),
        domain: "colors_kz".into(),
        id_prefix: "color_kz".into(),
        reviewer: "shaman".into(),
        reviewed_at: "2026-06-29".into(),
    };
    let isummary = integrate_approved_facts(&store, &target).expect("integrate");
    assert_eq!(isummary.integrated, 2);
    assert_eq!(isummary.already_integrated, 0);

    let entries = load_world_core_entries(&world_core_path).expect("read world_core");
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].id, "color_kz_001");
    assert_eq!(entries[1].id, "color_kz_002");
    assert_eq!(entries[0].domain, "colors_kz");
    assert_eq!(entries[0].review_status, "approved");
    assert_eq!(entries[0].source, "ingestion");
    assert!(entries[0].kk.contains("түс"));
    assert!(entries[0].facts[0].predicate == "is_a");

    // Integrated candidates now in terminal status.
    let final_facts = store.load_facts().expect("final");
    let integrated = final_facts
        .iter()
        .filter(|f| f.status == IngestionStatus::IntegratedIntoWorldCore)
        .count();
    assert_eq!(integrated, 2);

    // 5. Re-run integrator — no double-write, no new ids.
    let rerun = integrate_approved_facts(&store, &target).expect("rerun");
    assert_eq!(rerun.integrated, 0);
    assert_eq!(rerun.already_integrated, 2);
    let entries_after = load_world_core_entries(&world_core_path).unwrap();
    assert_eq!(
        entries_after.len(),
        2,
        "rerun should not duplicate the world_core file"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn pipeline_respects_validator_duplicate_gate_against_existing_world_core() {
    // Seed: world_core already says «алматы — қала».  The
    // extractor surfaces the same fact from raw text; the
    // validator should AutoReject it and the review stage
    // should see nothing in the NeedsReview bucket for
    // that subject.
    let root = tmp_root("dup");
    let queue_dir = root.join("queue");
    let store = CandidateStore::open(&queue_dir).expect("store open");

    let extracted = extract_facts_from_text(
        "Алматы — қала.\nАстана — қала.",
        "data/raw/geography.txt",
        SourceKind::TextFile,
        "2026-06-29",
    );
    assert_eq!(extracted.len(), 2);
    store.save_facts(&extracted).expect("seed");

    // Validator index says «алматы is_a қала» already
    // exists — that candidate should AutoReject; the
    // «астана» one should land NeedsReview.
    let mut index = WorldCoreIndex::new();
    index.insert("алматы", "is_a", "қала");

    let mut loaded = store.load_facts().expect("load");
    for fact in &mut loaded {
        let outcome = validate_fact(fact, &index);
        fact.status = outcome.new_status;
        if !fact.notes.is_empty() {
            fact.notes.push_str("; ");
        }
        fact.notes.push_str(&outcome.note);
    }
    store.save_facts(&loaded).expect("write back");

    let after = store.load_facts().expect("reload");
    let alma = after.iter().find(|f| f.subject == "алматы").unwrap();
    let asta = after.iter().find(|f| f.subject == "астана").unwrap();
    assert_eq!(alma.status, IngestionStatus::AutoRejected);
    assert!(alma.notes.contains("duplicate"));
    assert_eq!(asta.status, IngestionStatus::NeedsReview);

    // Review session sees ONLY астана — алматы is no
    // longer in the NeedsReview bucket.
    let mut reviewer = ScriptedReviewer::new(vec![ReviewDecision::Reject]);
    let summary = run_review_session(&store, &mut reviewer).expect("session ok");
    assert_eq!(summary.examined, 1);
    assert_eq!(reviewer.seen.len(), 1);
    // The one candidate the reviewer saw has «астана» as
    // its subject (алматы got duplicate-rejected before
    // review).
    let seen_id = &reviewer.seen[0];
    let seen_fact = store
        .load_facts()
        .unwrap()
        .into_iter()
        .find(|f| &f.id == seen_id)
        .unwrap();
    assert_eq!(seen_fact.subject, "астана");

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn pipeline_handles_zero_extractions_gracefully() {
    // Empty / non-matching source text — pipeline should
    // chug along producing zero candidates at every stage,
    // not panic and not write empty files.
    let root = tmp_root("empty");
    let queue_dir = root.join("queue");
    let world_core_path = root.join("test.jsonl");
    let store = CandidateStore::open(&queue_dir).expect("store open");

    let extracted = extract_facts_from_text(
        "# only a comment\n\nҚалай Алматы?",
        "data/raw/empty.txt",
        SourceKind::TextFile,
        "2026-06-29",
    );
    assert!(extracted.is_empty());
    store.save_facts(&extracted).expect("save empty");

    let mut reviewer = ScriptedReviewer::new(vec![]);
    let summary = run_review_session(&store, &mut reviewer).expect("session ok");
    assert_eq!(summary.examined, 0);

    let target = IntegrationTarget {
        world_core_path: world_core_path.clone(),
        domain: "test".into(),
        id_prefix: "t".into(),
        reviewer: "shaman".into(),
        reviewed_at: "2026-06-29".into(),
    };
    let isummary = integrate_approved_facts(&store, &target).expect("integrate");
    assert_eq!(isummary.integrated, 0);
    assert!(
        !world_core_path.exists(),
        "world_core file should not be created when there's nothing to integrate"
    );

    let _ = fs::remove_dir_all(&root);
}
