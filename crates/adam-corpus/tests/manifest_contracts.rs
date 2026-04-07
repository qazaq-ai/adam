use std::fs;

use adam_corpus::CorpusManifest;

#[test]
fn curated_corpus_manifest_stays_kazakh_only_and_valid() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/corpus_manifest.json"
    );
    let manifest: CorpusManifest =
        serde_json::from_str(&fs::read_to_string(path).expect("curated manifest file"))
            .expect("valid curated manifest json");

    manifest.validate().expect("curated manifest contract");
    assert_eq!(manifest.language, "kazakh");
    assert_eq!(manifest.script, "cyrillic");
}
