use std::fs;

use adam_corpus::{CorpusManifest, SourceAcceptanceReport, SourceRegistry, SourceScoringRules};
use adam_eval::EvalSuite;
use adam_tokenizer::TokenizerExperiment;
use adam_train::{
    BaselineTrainingManifest, build_baseline_training_assembly_report, build_baseline_training_plan,
};

#[test]
fn baseline_training_manifest_stays_kazakh_only_and_valid() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/baseline_training_manifest.json"
    );
    let manifest: BaselineTrainingManifest =
        serde_json::from_str(&fs::read_to_string(path).expect("training manifest file"))
            .expect("valid training manifest json");

    manifest.validate().expect("training manifest contract");
    assert_eq!(manifest.target_language, "kazakh");
    assert_eq!(manifest.script, "cyrillic");
    assert!(manifest.validation_split_bps > 0);
    assert!(manifest.validation_split_bps < 10_000);
}

#[test]
fn baseline_training_plan_can_be_built_from_manifests() {
    let manifest_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/baseline_training_manifest.json"
    );
    let manifest: BaselineTrainingManifest =
        serde_json::from_str(&fs::read_to_string(manifest_path).expect("training manifest file"))
            .expect("valid training manifest json");
    let corpus_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/corpus_manifest.json"
    );
    let corpus: CorpusManifest =
        serde_json::from_str(&fs::read_to_string(corpus_path).expect("corpus manifest file"))
            .expect("valid corpus manifest json");
    let registry_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/raw/source_registry.json"
    );
    let registry: SourceRegistry =
        serde_json::from_str(&fs::read_to_string(registry_path).expect("source registry file"))
            .expect("valid source registry json");
    let rules_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/raw/source_scoring_rules.json"
    );
    let rules: SourceScoringRules =
        serde_json::from_str(&fs::read_to_string(rules_path).expect("source scoring rules file"))
            .expect("valid source scoring rules json");
    let report_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/source_acceptance_report.json"
    );
    let report: SourceAcceptanceReport =
        serde_json::from_str(&fs::read_to_string(report_path).expect("acceptance report file"))
            .expect("valid source acceptance report json");
    let experiment_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/eval/tokenizer_experiment_manifest.json"
    );
    let experiment: TokenizerExperiment =
        serde_json::from_str(&fs::read_to_string(experiment_path).expect("experiment file"))
            .expect("valid tokenizer experiment json");
    let eval_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/eval/benchmark_manifest.json"
    );
    let eval_suite: EvalSuite =
        serde_json::from_str(&fs::read_to_string(eval_path).expect("eval suite file"))
            .expect("valid eval suite json");

    let plan = build_baseline_training_plan(
        &manifest,
        &corpus,
        &registry,
        &rules,
        &report,
        &experiment,
        &eval_suite,
    )
    .expect("baseline training plan");

    assert_eq!(plan.accepted_source_count, 1);
    assert_eq!(plan.rejected_source_count, 1);
    assert_eq!(plan.eval_task_count, 4);
    assert_eq!(plan.corpus_name, "adam-foundation-curated");
}

#[test]
fn baseline_training_assembly_can_be_built_from_manifests() {
    let manifest_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/baseline_training_manifest.json"
    );
    let manifest: BaselineTrainingManifest =
        serde_json::from_str(&fs::read_to_string(manifest_path).expect("training manifest file"))
            .expect("valid training manifest json");
    let corpus_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/corpus_manifest.json"
    );
    let corpus: CorpusManifest =
        serde_json::from_str(&fs::read_to_string(corpus_path).expect("corpus manifest file"))
            .expect("valid corpus manifest json");
    let registry_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/raw/source_registry.json"
    );
    let registry: SourceRegistry =
        serde_json::from_str(&fs::read_to_string(registry_path).expect("source registry file"))
            .expect("valid source registry json");
    let rules_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/raw/source_scoring_rules.json"
    );
    let rules: SourceScoringRules =
        serde_json::from_str(&fs::read_to_string(rules_path).expect("source scoring rules file"))
            .expect("valid source scoring rules json");
    let report_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/source_acceptance_report.json"
    );
    let report: SourceAcceptanceReport =
        serde_json::from_str(&fs::read_to_string(report_path).expect("acceptance report file"))
            .expect("valid source acceptance report json");
    let experiment_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/eval/tokenizer_experiment_manifest.json"
    );
    let experiment: TokenizerExperiment =
        serde_json::from_str(&fs::read_to_string(experiment_path).expect("experiment file"))
            .expect("valid tokenizer experiment json");
    let eval_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/eval/benchmark_manifest.json"
    );
    let eval_suite: EvalSuite =
        serde_json::from_str(&fs::read_to_string(eval_path).expect("eval suite file"))
            .expect("valid eval suite json");

    let report = build_baseline_training_assembly_report(
        &manifest,
        &corpus,
        &registry,
        &rules,
        &report,
        &experiment,
        &eval_suite,
    )
    .expect("baseline training assembly");

    assert_eq!(report.accepted_source_count, 1);
    assert_eq!(report.rejected_source_count, 1);
    assert_eq!(report.total_sequence_count, 1024);
    assert_eq!(report.validation_sequence_count, 102);
    assert_eq!(report.train_sequence_count, 922);
    assert!(!report.category_breakdown.is_empty());
    assert!(!report.critical_breakdown.is_empty());
    assert!(
        report
            .category_breakdown
            .iter()
            .any(|entry| entry.category == "domain_reference")
    );
    assert!(
        report
            .critical_breakdown
            .iter()
            .any(|entry| entry.guard == "single_source_concentration")
    );
    assert_eq!(report.source_allocations.len(), 1);
}
