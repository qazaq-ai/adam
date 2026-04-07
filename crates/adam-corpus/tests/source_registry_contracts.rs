use std::fs;

use adam_corpus::{SourceRegistry, SourceScoringRules};

#[test]
fn source_registry_stays_kazakh_cyrillic_and_valid() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/raw/source_registry.json"
    );
    let registry: SourceRegistry =
        serde_json::from_str(&fs::read_to_string(path).expect("source registry file"))
            .expect("valid source registry json");

    registry.validate().expect("source registry contract");
    assert!(!registry.entries.is_empty());
    assert!(
        registry
            .entries
            .iter()
            .all(|entry| entry.language == "kazakh")
    );
    assert!(
        registry
            .entries
            .iter()
            .all(|entry| entry.script == "cyrillic")
    );
}

#[test]
fn source_scoring_rules_manifest_is_present_and_usable() {
    let rules_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/raw/source_scoring_rules.json"
    );
    let rules: SourceScoringRules =
        serde_json::from_str(&fs::read_to_string(rules_path).expect("source scoring rules file"))
            .expect("valid source scoring rules json");
    let registry_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/raw/source_registry.json"
    );
    let registry: SourceRegistry =
        serde_json::from_str(&fs::read_to_string(registry_path).expect("source registry file"))
            .expect("valid source registry json");

    assert!(rules.minimum_acceptance_score >= 0);
    let score = rules.score(&registry.entries[0]);
    assert!(score.score <= rules.minimum_acceptance_score);
    assert!(!score.accepted_for_training);
}
