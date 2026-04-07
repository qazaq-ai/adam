use std::fs;

use adam_eval::EvalSuite;

#[test]
fn benchmark_manifest_stays_kazakh_only_and_valid() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/eval/benchmark_manifest.json"
    );
    let suite: EvalSuite =
        serde_json::from_str(&fs::read_to_string(path).expect("benchmark manifest file"))
            .expect("valid benchmark manifest json");

    suite.validate().expect("benchmark manifest contract");
    assert_eq!(suite.target_language, "kazakh");
    assert!(
        suite
            .tasks
            .iter()
            .all(|task| task.target_language == "kazakh")
    );
}
