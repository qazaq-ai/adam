use std::fs;

use adam_tokenizer::{
    SegmentationLexicon, SegmentationRuleSet, TokenizerDryRunPack, TokenizerExperiment,
    TokenizerSegmentationDataset, build_dry_run_report, build_experiment_report,
    build_segmentation_report,
};

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
    assert_eq!(
        experiment.segmentation_eval_manifest,
        "data/eval/tokenizer_segmentation_eval_dataset.json"
    );
    assert_eq!(
        experiment.segmentation_roots_manifest,
        "data/tokenizer/segmentation_roots.json"
    );
    assert_eq!(
        experiment.segmentation_rules_manifest,
        "data/tokenizer/segmentation_rules.json"
    );
}

#[test]
fn tokenizer_dry_run_pack_manifest_is_valid() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/tokenizer_dry_run_pack.json"
    );
    let pack: TokenizerDryRunPack =
        serde_json::from_str(&fs::read_to_string(path).expect("dry-run pack file"))
            .expect("valid dry-run pack json");

    pack.validate().expect("dry-run pack contract");
    assert_eq!(pack.target_language, "kazakh");
    assert_eq!(pack.script, "cyrillic");
    assert!(!pack.samples.is_empty());
}

#[test]
fn dry_run_report_can_be_built_from_manifests() {
    let experiment_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/eval/tokenizer_experiment_manifest.json"
    );
    let experiment: TokenizerExperiment =
        serde_json::from_str(&fs::read_to_string(experiment_path).expect("experiment file"))
            .expect("valid experiment json");
    let pack_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/tokenizer_dry_run_pack.json"
    );
    let pack: TokenizerDryRunPack =
        serde_json::from_str(&fs::read_to_string(pack_path).expect("dry-run pack file"))
            .expect("valid dry-run pack json");

    let report = build_dry_run_report(&experiment, &pack).expect("dry-run report");
    assert_eq!(report.experiment_name, experiment.name);
    assert_eq!(report.sample_count, pack.samples.len());
}

#[test]
fn tokenizer_segmentation_dataset_contract_is_valid() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/eval/tokenizer_segmentation_eval_dataset.json"
    );
    let dataset: TokenizerSegmentationDataset =
        serde_json::from_str(&fs::read_to_string(path).expect("segmentation dataset file"))
            .expect("valid segmentation dataset json");

    dataset
        .validate()
        .expect("tokenizer segmentation dataset contract");
    let roots_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/tokenizer/segmentation_roots.json"
    );
    let lexicon: SegmentationLexicon =
        serde_json::from_str(&fs::read_to_string(roots_path).expect("segmentation roots file"))
            .expect("valid segmentation roots json");
    let rules_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/tokenizer/segmentation_rules.json"
    );
    let rules: SegmentationRuleSet =
        serde_json::from_str(&fs::read_to_string(rules_path).expect("segmentation rules file"))
            .expect("valid segmentation rules json");

    lexicon.validate().expect("segmentation roots contract");
    rules.validate().expect("segmentation rules contract");
    assert_eq!(lexicon.version, dataset.version);
    assert_eq!(rules.version, dataset.version);
    let report =
        build_segmentation_report(&dataset, &lexicon, &rules).expect("segmentation report");
    assert_eq!(report.example_count, dataset.entries.len());
    assert!(report.average_segment_count >= 2);
    assert_eq!(report.exact_match_count, dataset.entries.len());
    assert!(!report.category_breakdown.is_empty());
    assert!(
        report
            .category_breakdown
            .iter()
            .any(|entry| entry.category == "verb_negative_plural_imperative")
    );
}

#[test]
fn tokenizer_experiment_report_scores_segmentation_matches() {
    let experiment_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/eval/tokenizer_experiment_manifest.json"
    );
    let experiment: TokenizerExperiment =
        serde_json::from_str(&fs::read_to_string(experiment_path).expect("experiment file"))
            .expect("valid experiment json");
    let pack_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/tokenizer_dry_run_pack.json"
    );
    let pack: TokenizerDryRunPack =
        serde_json::from_str(&fs::read_to_string(pack_path).expect("dry-run pack file"))
            .expect("valid dry-run pack json");
    let dataset_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/eval/tokenizer_segmentation_eval_dataset.json"
    );
    let dataset: TokenizerSegmentationDataset =
        serde_json::from_str(&fs::read_to_string(dataset_path).expect("segmentation dataset file"))
            .expect("valid segmentation dataset json");
    let roots_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/tokenizer/segmentation_roots.json"
    );
    let lexicon: SegmentationLexicon =
        serde_json::from_str(&fs::read_to_string(roots_path).expect("segmentation roots file"))
            .expect("valid segmentation roots json");
    let rules_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/tokenizer/segmentation_rules.json"
    );
    let rules: SegmentationRuleSet =
        serde_json::from_str(&fs::read_to_string(rules_path).expect("segmentation rules file"))
            .expect("valid segmentation rules json");

    let report = build_experiment_report(&experiment, &pack, &dataset, &lexicon, &rules)
        .expect("experiment report");
    assert_eq!(report.segmentation_example_count, dataset.entries.len());
    assert_eq!(report.exact_match_count, dataset.entries.len());
    assert!(!report.category_breakdown.is_empty());
    assert!(report.failures.is_empty());
}
