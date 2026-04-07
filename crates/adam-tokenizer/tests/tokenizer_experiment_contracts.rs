use std::fs;

use adam_tokenizer::TokenizerExperiment;

#[test]
fn tokenizer_experiment_manifest_stays_kazakh_only_and_valid() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/eval/tokenizer_experiment_manifest.json"
    );
    let experiment: TokenizerExperiment =
        serde_json::from_str(&fs::read_to_string(path).expect("tokenizer experiment file"))
            .expect("valid tokenizer experiment json");

    experiment
        .validate()
        .expect("tokenizer experiment contract");
    assert_eq!(experiment.target_language, "kazakh");
    assert_eq!(experiment.script, "cyrillic");
}
