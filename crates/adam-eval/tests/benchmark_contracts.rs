use std::fs;

use adam_eval::{
    EvalBenchmarkDeltaReport, EvalBenchmarkReport, EvalSuite, build_eval_benchmark_delta_report,
    build_eval_benchmark_report,
};

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
    assert_eq!(suite.tasks.len(), 4);
    assert!(
        suite
            .tasks
            .iter()
            .all(|task| task.target_language == "kazakh")
    );
    assert!(
        suite
            .tasks
            .iter()
            .any(|task| task.name == "kazakh-tokenizer-segmentation")
    );
}

#[test]
fn benchmark_report_matches_expected_regression_artifact() {
    let suite_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/eval/benchmark_manifest.json"
    );
    let suite: EvalSuite =
        serde_json::from_str(&fs::read_to_string(suite_path).expect("benchmark manifest file"))
            .expect("valid benchmark manifest json");
    let expected_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/eval/benchmark_report.json"
    );
    let expected: EvalBenchmarkReport =
        serde_json::from_str(&fs::read_to_string(expected_path).expect("benchmark report file"))
            .expect("valid benchmark report json");

    let actual = build_eval_benchmark_report(&suite).expect("benchmark report");

    assert_eq!(actual, expected);
}

#[test]
fn benchmark_delta_report_matches_expected_regression_artifact() {
    let suite_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/eval/benchmark_manifest.json"
    );
    let suite: EvalSuite =
        serde_json::from_str(&fs::read_to_string(suite_path).expect("benchmark manifest file"))
            .expect("valid benchmark manifest json");
    let expected_report_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/eval/benchmark_report.json"
    );
    let expected_report: EvalBenchmarkReport = serde_json::from_str(
        &fs::read_to_string(expected_report_path).expect("benchmark report file"),
    )
    .expect("valid benchmark report json");
    let expected_delta_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/eval/benchmark_delta_report.json"
    );
    let expected_delta: EvalBenchmarkDeltaReport = serde_json::from_str(
        &fs::read_to_string(expected_delta_path).expect("benchmark delta report file"),
    )
    .expect("valid benchmark delta report json");

    let actual = build_eval_benchmark_delta_report(&suite, &expected_report)
        .expect("benchmark delta report");

    assert_eq!(actual, expected_delta);
}
