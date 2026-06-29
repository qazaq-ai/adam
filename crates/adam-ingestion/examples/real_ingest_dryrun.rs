// One-off binary — run the extractor on real wikibooks text,
// populate data/ingestion/, run validator, surface results.
// Not committed; lives in scratchpad.
use std::fs;

use adam_ingestion::{
    CandidateStore, IngestionStatus, SourceKind, extract_facts_from_text, validate_fact,
    validator::WorldCoreIndex,
};

fn main() {
    let text = fs::read_to_string("data/raw/ingest_demo/wikibooks_kk_sample.txt")
        .expect("read wikibooks text");
    println!("Source: {} chars", text.len());

    // 1. Extract.
    let extracted = extract_facts_from_text(
        &text,
        "data/external/wikibooks_kk_500pages",
        SourceKind::TextFile,
        "2026-06-29",
    );
    println!("Extracted: {} candidates", extracted.len());

    // Save to ingestion queue.
    let store = CandidateStore::open("data/ingestion").expect("open store");
    store.save_facts(&extracted).expect("save");

    // 2. Validate against current world_core.
    let index = WorldCoreIndex::load_from_dir("data/world_core");
    println!("World_core index: {} unique triples", index.len());

    let mut loaded = store.load_facts().expect("load");
    let mut counts = std::collections::BTreeMap::new();
    for fact in &mut loaded {
        let outcome = validate_fact(fact, &index);
        fact.status = outcome.new_status;
        if !fact.notes.is_empty() {
            fact.notes.push_str("; ");
        }
        fact.notes.push_str(&outcome.note);
        *counts
            .entry(format!("{:?}", outcome.new_status))
            .or_insert(0u32) += 1;
    }
    store.save_facts(&loaded).expect("write back");

    println!("\nPost-validation status histogram:");
    for (k, v) in &counts {
        println!("  {:25} {:>5}", k, v);
    }

    // Show first 5 NeedsReview candidates so we can eyeball quality.
    println!("\n=== first 10 NeedsReview candidates ===");
    let needs: Vec<_> = loaded
        .iter()
        .filter(|f| f.status == IngestionStatus::NeedsReview)
        .take(10)
        .collect();
    for (i, f) in needs.iter().enumerate() {
        let subj_short: String = f.subject.chars().take(40).collect();
        let obj_short: String = f.object.chars().take(60).collect();
        let ctx_short: String = f.source_sentence.chars().take(90).collect();
        println!(
            "  [{:2}] {} → {} → {}",
            i + 1,
            subj_short,
            f.predicate,
            obj_short
        );
        println!("       ctx: {}", ctx_short);
    }
}
