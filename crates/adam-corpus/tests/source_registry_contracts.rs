use std::fs;

use adam_corpus::SourceRegistry;

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
