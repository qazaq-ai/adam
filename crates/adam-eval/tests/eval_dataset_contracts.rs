use std::fs;

use adam_eval::EvalDataset;

#[test]
fn eval_dataset_manifest_stays_kazakh_only_and_valid() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/eval/kazakh_foundation_eval_dataset.json"
    );
    let dataset: EvalDataset =
        serde_json::from_str(&fs::read_to_string(path).expect("eval dataset file"))
            .expect("valid eval dataset json");

    dataset.validate().expect("eval dataset contract");
    assert_eq!(dataset.target_language, "kazakh");
    assert_eq!(dataset.script, "cyrillic");
    assert!(!dataset.entries.is_empty());
}
