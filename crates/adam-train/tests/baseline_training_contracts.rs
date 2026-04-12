use std::fs;

use adam_corpus::{
    CorpusManifest, CorpusStage, LicenseClass, QualityTier, SourceAcceptanceDeltaReport,
    SourceAcceptanceReport, SourceAcceptanceSummaryReport, SourceDomain, SourcePolicy,
    SourceRegistry, SourceRegistryEntry, SourceScoringRules, SourceType,
    build_source_acceptance_report,
};
use adam_eval::{EvalBenchmarkDeltaReport, EvalBenchmarkReport, EvalSuite};
use adam_tokenizer::{
    TokenizerExperiment, TokenizerExperimentDeltaReport, TokenizerExperimentReport,
};
use adam_train::{
    BaselineTrainingAssemblyReport, BaselineTrainingConsistencyReport, BaselineTrainingDeltaReport,
    BaselineTrainingManifest, CleanTrainingCorpusManifest, CleanTrainingCorpusPack,
    CleanTrainingCorpusReport, FoundationOverviewDeltaReport, FoundationOverviewReport,
    TinyCleanTrainingDomainPack, TinyCleanTrainingPack,
    TinyCleanTrainingProfileBaselineDeltaReport, TinyCleanTrainingProfileBaselineManifest,
    TinyCleanTrainingProfileBaselineReport, TinyCleanTrainingProfileComparisonReport,
    TinyCleanTrainingProfileExperimentMatrixManifest,
    TinyCleanTrainingProfileExperimentMatrixReport, TinyCleanTrainingProfilePromotionDeltaReport,
    TinyCleanTrainingProfilePromotionManifest, TinyCleanTrainingProfilePromotionReport,
    TinyCleanTrainingProfileStrategyDeltaReport, TinyCleanTrainingProfileStrategyManifest,
    TinyCleanTrainingProfileStrategyReport, TinyCleanTrainingProfileSuiteManifest,
    TinyCleanTrainingProfileSuiteReport, TinyCleanTrainingReport,
    TinyCleanTrainingSelectionManifest, assemble_clean_training_corpus_pack,
    assemble_tiny_clean_training_pack_from_corpus,
    assemble_tiny_clean_training_pack_from_promotion, build_baseline_training_assembly_report,
    build_baseline_training_consistency_report, build_baseline_training_delta_report,
    build_baseline_training_plan, build_clean_training_corpus_report,
    build_foundation_overview_delta_report, build_foundation_overview_report,
    build_tiny_clean_training_profile_baseline_delta_report,
    build_tiny_clean_training_profile_baseline_report,
    build_tiny_clean_training_profile_comparison_report,
    build_tiny_clean_training_profile_experiment_matrix_report,
    build_tiny_clean_training_profile_promotion_delta_report,
    build_tiny_clean_training_profile_promotion_report,
    build_tiny_clean_training_profile_strategy_delta_report,
    build_tiny_clean_training_profile_strategy_report,
    build_tiny_clean_training_profile_suite_report, build_tiny_clean_training_report,
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

    assert_eq!(plan.accepted_source_count, 3);
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

    assert_eq!(report.accepted_source_count, 3);
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
            .any(|entry| entry.category == "domain_general")
    );
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
            .any(|entry| entry.guard == "multi_source_distribution")
    );
    assert_eq!(report.source_allocations.len(), 3);
}

#[test]
fn baseline_training_assembly_report_matches_expected_regression_artifact() {
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
    let expected_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/baseline_training_assembly_report.json"
    );
    let expected: BaselineTrainingAssemblyReport =
        serde_json::from_str(&fs::read_to_string(expected_path).expect("expected assembly report"))
            .expect("valid expected assembly report json");

    let actual = build_baseline_training_assembly_report(
        &manifest,
        &corpus,
        &registry,
        &rules,
        &report,
        &experiment,
        &eval_suite,
    )
    .expect("baseline training assembly");

    assert_eq!(actual, expected);
}

#[test]
fn baseline_training_consistency_report_matches_expected_regression_artifact() {
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
    let expected_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/baseline_training_consistency_report.json"
    );
    let expected: BaselineTrainingConsistencyReport = serde_json::from_str(
        &fs::read_to_string(expected_path).expect("expected consistency report"),
    )
    .expect("valid expected consistency report json");

    let actual = build_baseline_training_consistency_report(
        &manifest,
        &corpus,
        &registry,
        &rules,
        &report,
        &experiment,
        &eval_suite,
    )
    .expect("baseline training consistency");

    assert_eq!(actual, expected);
}

#[test]
fn baseline_training_delta_report_matches_expected_regression_artifact() {
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
    let expected_assembly_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/baseline_training_assembly_report.json"
    );
    let expected_assembly: BaselineTrainingAssemblyReport = serde_json::from_str(
        &fs::read_to_string(expected_assembly_path).expect("expected assembly report"),
    )
    .expect("valid expected assembly report json");
    let expected_consistency_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/baseline_training_consistency_report.json"
    );
    let expected_consistency: BaselineTrainingConsistencyReport = serde_json::from_str(
        &fs::read_to_string(expected_consistency_path).expect("expected consistency report"),
    )
    .expect("valid expected consistency report json");
    let expected_delta_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/baseline_training_delta_report.json"
    );
    let expected_delta: BaselineTrainingDeltaReport =
        serde_json::from_str(&fs::read_to_string(expected_delta_path).expect("expected delta"))
            .expect("valid expected delta report json");

    let actual = build_baseline_training_delta_report(
        &manifest,
        &corpus,
        &registry,
        &rules,
        &report,
        &experiment,
        &eval_suite,
        &expected_assembly,
        &expected_consistency,
    )
    .expect("baseline training delta");

    assert_eq!(actual, expected_delta);
}

#[test]
fn baseline_training_assembly_tracks_multi_source_distribution() {
    let manifest_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/baseline_training_manifest.json"
    );
    let manifest: BaselineTrainingManifest =
        serde_json::from_str(&fs::read_to_string(manifest_path).expect("training manifest file"))
            .expect("valid training manifest json");
    let corpus = CorpusManifest {
        version: "0.0.41".to_string(),
        name: "adam-foundation-curated".to_string(),
        language: "kazakh".to_string(),
        script: "cyrillic".to_string(),
        stage: CorpusStage::Curated,
        source_policy: SourcePolicy::KazakhOnly,
        domains: vec![
            "general_text".to_string(),
            "administrative_text".to_string(),
            "reference_text".to_string(),
            "education_text".to_string(),
        ],
    };
    let registry = SourceRegistry {
        version: "0.0.41".to_string(),
        entries: vec![
            SourceRegistryEntry {
                id: "curated_general_kazakh".to_string(),
                stage: CorpusStage::Curated,
                language: "kazakh".to_string(),
                script: "cyrillic".to_string(),
                source_type: SourceType::PublicText,
                domain: SourceDomain::General,
                license_class: LicenseClass::Open,
                quality_tier: QualityTier::Reviewed,
                provenance: "manual_general_seed".to_string(),
                allowed_for_training: true,
            },
            SourceRegistryEntry {
                id: "curated_reference_kazakh".to_string(),
                stage: CorpusStage::Curated,
                language: "kazakh".to_string(),
                script: "cyrillic".to_string(),
                source_type: SourceType::ReferenceText,
                domain: SourceDomain::Reference,
                license_class: LicenseClass::Open,
                quality_tier: QualityTier::TrainingReady,
                provenance: "manual_reference_seed".to_string(),
                allowed_for_training: true,
            },
            SourceRegistryEntry {
                id: "reviewed_education_kazakh".to_string(),
                stage: CorpusStage::Curated,
                language: "kazakh".to_string(),
                script: "cyrillic".to_string(),
                source_type: SourceType::EducationalText,
                domain: SourceDomain::Education,
                license_class: LicenseClass::Open,
                quality_tier: QualityTier::Reviewed,
                provenance: "manual_education_seed".to_string(),
                allowed_for_training: true,
            },
        ],
    };
    let rules_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/raw/source_scoring_rules.json"
    );
    let rules: SourceScoringRules =
        serde_json::from_str(&fs::read_to_string(rules_path).expect("source scoring rules file"))
            .expect("valid source scoring rules json");
    let report = build_source_acceptance_report(
        "adam-source-acceptance-report",
        "data/raw/source_registry.json",
        "data/raw/source_scoring_rules.json",
        &registry,
        &rules,
    )
    .expect("acceptance report");
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

    assert_eq!(report.accepted_source_count, 3);
    assert_eq!(report.rejected_source_count, 0);
    assert_eq!(report.source_allocations.len(), 3);
    assert!(
        report
            .critical_breakdown
            .iter()
            .any(|entry| entry.guard == "multi_source_distribution")
    );
    assert!(
        report
            .category_breakdown
            .iter()
            .any(|entry| entry.category == "domain_education")
    );
    assert!(
        report
            .category_breakdown
            .iter()
            .any(|entry| entry.category == "source_type_public_text")
    );
}

#[test]
fn tiny_clean_training_report_matches_expected_regression_artifact() {
    let manifest_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/baseline_training_manifest.json"
    );
    let manifest: BaselineTrainingManifest =
        serde_json::from_str(&fs::read_to_string(manifest_path).expect("training manifest file"))
            .expect("valid training manifest json");
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
    let profile_suite_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/tiny_clean_training_profile_suite_manifest.json"
    );
    let profile_suite: TinyCleanTrainingProfileSuiteManifest = serde_json::from_str(
        &fs::read_to_string(profile_suite_path).expect("tiny training profile suite manifest file"),
    )
    .expect("valid tiny training profile suite manifest json");
    let promotion_report_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_profile_promotion_report.json"
    );
    let promotion_report: TinyCleanTrainingProfilePromotionReport = serde_json::from_str(
        &fs::read_to_string(promotion_report_path).expect("tiny training profile promotion report"),
    )
    .expect("valid tiny training profile promotion report json");
    let clean_manifest_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/clean_training_corpus_manifest.json"
    );
    let clean_pack_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/clean_training_corpus_pack.json"
    );
    let clean_manifest: CleanTrainingCorpusManifest = serde_json::from_str(
        &fs::read_to_string(clean_manifest_path).expect("clean corpus manifest file"),
    )
    .expect("valid clean corpus manifest json");
    let clean_pack: CleanTrainingCorpusPack =
        serde_json::from_str(&fs::read_to_string(clean_pack_path).expect("clean corpus pack file"))
            .expect("valid clean corpus pack json");
    let pack = assemble_tiny_clean_training_pack_from_promotion(
        &profile_suite,
        &promotion_report,
        &clean_manifest,
        &clean_pack,
    )
    .expect("assembled tiny clean training pack from promotion");
    let expected_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_report.json"
    );
    let expected: TinyCleanTrainingReport =
        serde_json::from_str(&fs::read_to_string(expected_path).expect("tiny training report"))
            .expect("valid tiny training report json");

    let actual = build_tiny_clean_training_report(&manifest, &registry, &rules, &report, &pack)
        .expect("tiny clean training report");

    assert_eq!(actual, expected);
}

#[test]
fn tiny_clean_training_pack_matches_domain_manifest_assembly() {
    let selection_manifest_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/tiny_clean_training_selection_manifest.json"
    );
    let selection_manifest: TinyCleanTrainingSelectionManifest = serde_json::from_str(
        &fs::read_to_string(selection_manifest_path)
            .expect("tiny training selection manifest file"),
    )
    .expect("valid tiny training selection manifest json");
    let clean_manifest_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/clean_training_corpus_manifest.json"
    );
    let clean_pack_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/clean_training_corpus_pack.json"
    );
    let clean_manifest: CleanTrainingCorpusManifest = serde_json::from_str(
        &fs::read_to_string(clean_manifest_path).expect("clean corpus manifest file"),
    )
    .expect("valid clean corpus manifest json");
    let clean_pack: CleanTrainingCorpusPack =
        serde_json::from_str(&fs::read_to_string(clean_pack_path).expect("clean corpus pack file"))
            .expect("valid clean corpus pack json");
    let expected_pack_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/tiny_clean_training_pack.json"
    );
    let expected_pack: TinyCleanTrainingPack = serde_json::from_str(
        &fs::read_to_string(expected_pack_path).expect("tiny training pack file"),
    )
    .expect("valid tiny training pack json");

    let actual_pack = assemble_tiny_clean_training_pack_from_corpus(
        &selection_manifest,
        &clean_manifest,
        &clean_pack,
    )
    .expect("assembled tiny clean training pack");

    assert_eq!(actual_pack, expected_pack);
}

#[test]
fn clean_training_corpus_pack_matches_manifest_assembly() {
    let manifest_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/clean_training_corpus_manifest.json"
    );
    let manifest: CleanTrainingCorpusManifest =
        serde_json::from_str(&fs::read_to_string(manifest_path).expect("clean corpus manifest"))
            .expect("valid clean corpus manifest json");
    let pack_paths = [
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/curated/tiny_clean_general_pack.json"
        ),
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/curated/tiny_clean_reference_pack.json"
        ),
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/curated/tiny_clean_education_pack.json"
        ),
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/curated/clean_general_extension_pack.json"
        ),
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/curated/clean_reference_extension_pack.json"
        ),
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/curated/clean_education_extension_pack.json"
        ),
    ];
    let domain_packs: Vec<TinyCleanTrainingDomainPack> = pack_paths
        .into_iter()
        .map(|path| {
            serde_json::from_str(&fs::read_to_string(path).expect("clean corpus domain pack"))
                .expect("valid clean corpus domain pack json")
        })
        .collect();
    let expected_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/clean_training_corpus_pack.json"
    );
    let expected: CleanTrainingCorpusPack =
        serde_json::from_str(&fs::read_to_string(expected_path).expect("clean corpus pack"))
            .expect("valid clean corpus pack json");

    let actual = assemble_clean_training_corpus_pack(&manifest, &domain_packs)
        .expect("assembled clean corpus pack");

    assert_eq!(actual, expected);
}

#[test]
fn clean_training_corpus_report_matches_expected_regression_artifact() {
    let manifest_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/clean_training_corpus_manifest.json"
    );
    let manifest: CleanTrainingCorpusManifest =
        serde_json::from_str(&fs::read_to_string(manifest_path).expect("clean corpus manifest"))
            .expect("valid clean corpus manifest json");
    let pack_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/clean_training_corpus_pack.json"
    );
    let pack: CleanTrainingCorpusPack =
        serde_json::from_str(&fs::read_to_string(pack_path).expect("clean corpus pack"))
            .expect("valid clean corpus pack json");
    let expected_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/clean_training_corpus_report.json"
    );
    let expected: CleanTrainingCorpusReport = serde_json::from_str(
        &fs::read_to_string(expected_path).expect("clean training corpus report"),
    )
    .expect("valid clean training corpus report json");

    let actual =
        build_clean_training_corpus_report(&manifest, &pack).expect("clean training corpus report");

    assert_eq!(actual, expected);
}

#[test]
fn tiny_clean_training_profile_suite_report_matches_expected_regression_artifact() {
    let training_manifest_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/baseline_training_manifest.json"
    );
    let training_manifest: BaselineTrainingManifest = serde_json::from_str(
        &fs::read_to_string(training_manifest_path).expect("training manifest file"),
    )
    .expect("valid training manifest json");
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
    let acceptance_report: SourceAcceptanceReport =
        serde_json::from_str(&fs::read_to_string(report_path).expect("acceptance report file"))
            .expect("valid source acceptance report json");
    let suite_manifest_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/tiny_clean_training_profile_suite_manifest.json"
    );
    let suite_manifest: TinyCleanTrainingProfileSuiteManifest = serde_json::from_str(
        &fs::read_to_string(suite_manifest_path).expect("tiny profile suite manifest file"),
    )
    .expect("valid tiny profile suite manifest json");
    let clean_manifest_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/clean_training_corpus_manifest.json"
    );
    let clean_manifest: CleanTrainingCorpusManifest = serde_json::from_str(
        &fs::read_to_string(clean_manifest_path).expect("clean corpus manifest file"),
    )
    .expect("valid clean corpus manifest json");
    let clean_pack_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/clean_training_corpus_pack.json"
    );
    let clean_pack: CleanTrainingCorpusPack =
        serde_json::from_str(&fs::read_to_string(clean_pack_path).expect("clean corpus pack file"))
            .expect("valid clean corpus pack json");
    let expected_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_profile_suite_report.json"
    );
    let expected: TinyCleanTrainingProfileSuiteReport = serde_json::from_str(
        &fs::read_to_string(expected_path).expect("tiny profile suite report"),
    )
    .expect("valid tiny profile suite report json");

    let actual = build_tiny_clean_training_profile_suite_report(
        &training_manifest,
        &registry,
        &rules,
        &acceptance_report,
        &suite_manifest,
        &clean_manifest,
        &clean_pack,
    )
    .expect("tiny profile suite report");

    assert_eq!(actual, expected);
}

#[test]
fn tiny_clean_training_profile_comparison_report_matches_expected_regression_artifact() {
    let suite_report_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_profile_suite_report.json"
    );
    let suite_report: TinyCleanTrainingProfileSuiteReport = serde_json::from_str(
        &fs::read_to_string(suite_report_path).expect("tiny profile suite report"),
    )
    .expect("valid tiny profile suite report json");
    let expected_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_profile_comparison_report.json"
    );
    let expected: TinyCleanTrainingProfileComparisonReport = serde_json::from_str(
        &fs::read_to_string(expected_path).expect("tiny profile comparison report"),
    )
    .expect("valid tiny profile comparison report json");

    let actual = build_tiny_clean_training_profile_comparison_report(&suite_report)
        .expect("tiny profile comparison report");

    assert_eq!(actual, expected);
}

#[test]
fn tiny_clean_training_profile_baseline_report_matches_expected_regression_artifact() {
    let manifest_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/tiny_clean_training_profile_baseline_manifest.json"
    );
    let manifest: TinyCleanTrainingProfileBaselineManifest = serde_json::from_str(
        &fs::read_to_string(manifest_path).expect("tiny profile baseline manifest"),
    )
    .expect("valid tiny profile baseline manifest json");
    let comparison_report_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_profile_comparison_report.json"
    );
    let comparison_report: TinyCleanTrainingProfileComparisonReport = serde_json::from_str(
        &fs::read_to_string(comparison_report_path).expect("tiny profile comparison report"),
    )
    .expect("valid tiny profile comparison report json");
    let expected_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_profile_baseline_report.json"
    );
    let expected: TinyCleanTrainingProfileBaselineReport = serde_json::from_str(
        &fs::read_to_string(expected_path).expect("tiny profile baseline report"),
    )
    .expect("valid tiny profile baseline report json");

    let actual = build_tiny_clean_training_profile_baseline_report(&manifest, &comparison_report)
        .expect("tiny profile baseline report");

    assert_eq!(actual, expected);
}

#[test]
fn tiny_clean_training_profile_baseline_delta_report_matches_expected_regression_artifact() {
    let baseline_report_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_profile_baseline_report.json"
    );
    let baseline_report: TinyCleanTrainingProfileBaselineReport = serde_json::from_str(
        &fs::read_to_string(baseline_report_path).expect("tiny profile baseline report"),
    )
    .expect("valid tiny profile baseline report json");
    let expected_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_profile_baseline_delta_report.json"
    );
    let expected: TinyCleanTrainingProfileBaselineDeltaReport = serde_json::from_str(
        &fs::read_to_string(expected_path).expect("tiny profile baseline delta report"),
    )
    .expect("valid tiny profile baseline delta report json");

    let actual =
        build_tiny_clean_training_profile_baseline_delta_report(&baseline_report, &baseline_report);

    assert_eq!(actual, expected);
}

#[test]
fn tiny_clean_training_profile_strategy_report_matches_expected_regression_artifact() {
    let manifest_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/tiny_clean_training_profile_strategy_manifest.json"
    );
    let manifest: TinyCleanTrainingProfileStrategyManifest = serde_json::from_str(
        &fs::read_to_string(manifest_path).expect("tiny profile strategy manifest"),
    )
    .expect("valid tiny profile strategy manifest json");
    let baseline_report_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_profile_baseline_report.json"
    );
    let baseline_report: TinyCleanTrainingProfileBaselineReport = serde_json::from_str(
        &fs::read_to_string(baseline_report_path).expect("tiny profile baseline report"),
    )
    .expect("valid tiny profile baseline report json");
    let comparison_report_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_profile_comparison_report.json"
    );
    let comparison_report: TinyCleanTrainingProfileComparisonReport = serde_json::from_str(
        &fs::read_to_string(comparison_report_path).expect("tiny profile comparison report"),
    )
    .expect("valid tiny profile comparison report json");
    let expected_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_profile_strategy_report.json"
    );
    let expected: TinyCleanTrainingProfileStrategyReport = serde_json::from_str(
        &fs::read_to_string(expected_path).expect("tiny profile strategy report"),
    )
    .expect("valid tiny profile strategy report json");

    let actual = build_tiny_clean_training_profile_strategy_report(
        &manifest,
        &baseline_report,
        &comparison_report,
    )
    .expect("tiny profile strategy report");

    assert_eq!(actual, expected);
}

#[test]
fn tiny_clean_training_profile_strategy_delta_report_matches_expected_regression_artifact() {
    let strategy_report_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_profile_strategy_report.json"
    );
    let strategy_report: TinyCleanTrainingProfileStrategyReport = serde_json::from_str(
        &fs::read_to_string(strategy_report_path).expect("tiny profile strategy report"),
    )
    .expect("valid tiny profile strategy report json");
    let expected_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_profile_strategy_delta_report.json"
    );
    let expected: TinyCleanTrainingProfileStrategyDeltaReport = serde_json::from_str(
        &fs::read_to_string(expected_path).expect("tiny profile strategy delta report"),
    )
    .expect("valid tiny profile strategy delta report json");

    let actual =
        build_tiny_clean_training_profile_strategy_delta_report(&strategy_report, &strategy_report);

    assert_eq!(actual, expected);
}

#[test]
fn tiny_clean_training_profile_promotion_report_matches_expected_regression_artifact() {
    let manifest_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/tiny_clean_training_profile_promotion_manifest.json"
    );
    let manifest: TinyCleanTrainingProfilePromotionManifest = serde_json::from_str(
        &fs::read_to_string(manifest_path).expect("tiny profile promotion manifest"),
    )
    .expect("valid tiny profile promotion manifest json");
    let strategy_report_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_profile_strategy_report.json"
    );
    let strategy_report: TinyCleanTrainingProfileStrategyReport = serde_json::from_str(
        &fs::read_to_string(strategy_report_path).expect("tiny profile strategy report"),
    )
    .expect("valid tiny profile strategy report json");
    let expected_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_profile_promotion_report.json"
    );
    let expected: TinyCleanTrainingProfilePromotionReport = serde_json::from_str(
        &fs::read_to_string(expected_path).expect("tiny profile promotion report"),
    )
    .expect("valid tiny profile promotion report json");

    let actual = build_tiny_clean_training_profile_promotion_report(&manifest, &strategy_report)
        .expect("tiny profile promotion report");

    assert_eq!(actual, expected);
}

#[test]
fn tiny_clean_training_profile_promotion_delta_report_matches_expected_regression_artifact() {
    let promotion_report_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_profile_promotion_report.json"
    );
    let promotion_report: TinyCleanTrainingProfilePromotionReport = serde_json::from_str(
        &fs::read_to_string(promotion_report_path).expect("tiny profile promotion report"),
    )
    .expect("valid tiny profile promotion report json");
    let expected_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_profile_promotion_delta_report.json"
    );
    let expected: TinyCleanTrainingProfilePromotionDeltaReport = serde_json::from_str(
        &fs::read_to_string(expected_path).expect("tiny profile promotion delta report"),
    )
    .expect("valid tiny profile promotion delta report json");

    let actual = build_tiny_clean_training_profile_promotion_delta_report(
        &promotion_report,
        &promotion_report,
    );

    assert_eq!(actual, expected);
}

#[test]
fn tiny_clean_training_profile_experiment_matrix_report_matches_expected_regression_artifact() {
    let training_manifest_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/baseline_training_manifest.json"
    );
    let training_manifest: BaselineTrainingManifest = serde_json::from_str(
        &fs::read_to_string(training_manifest_path).expect("baseline training manifest"),
    )
    .expect("valid baseline training manifest json");
    let registry_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/raw/source_registry.json"
    );
    let registry: SourceRegistry =
        serde_json::from_str(&fs::read_to_string(registry_path).expect("source registry"))
            .expect("valid source registry json");
    let rules_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/raw/source_scoring_rules.json"
    );
    let rules: SourceScoringRules =
        serde_json::from_str(&fs::read_to_string(rules_path).expect("source scoring rules"))
            .expect("valid source scoring rules json");
    let acceptance_report_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/source_acceptance_report.json"
    );
    let acceptance_report: SourceAcceptanceReport = serde_json::from_str(
        &fs::read_to_string(acceptance_report_path).expect("source acceptance report"),
    )
    .expect("valid source acceptance report json");
    let suite_manifest_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/tiny_clean_training_profile_suite_manifest.json"
    );
    let suite_manifest: TinyCleanTrainingProfileSuiteManifest = serde_json::from_str(
        &fs::read_to_string(suite_manifest_path).expect("tiny profile suite manifest"),
    )
    .expect("valid tiny profile suite manifest json");
    let clean_corpus_manifest_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/clean_training_corpus_manifest.json"
    );
    let clean_corpus_manifest: CleanTrainingCorpusManifest = serde_json::from_str(
        &fs::read_to_string(clean_corpus_manifest_path).expect("clean training corpus manifest"),
    )
    .expect("valid clean training corpus manifest json");
    let clean_corpus_pack_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/clean_training_corpus_pack.json"
    );
    let clean_corpus_pack: CleanTrainingCorpusPack = serde_json::from_str(
        &fs::read_to_string(clean_corpus_pack_path).expect("clean training corpus pack"),
    )
    .expect("valid clean training corpus pack json");
    let strategy_report_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_profile_strategy_report.json"
    );
    let strategy_report: TinyCleanTrainingProfileStrategyReport = serde_json::from_str(
        &fs::read_to_string(strategy_report_path).expect("tiny profile strategy report"),
    )
    .expect("valid tiny profile strategy report json");
    let promotion_report_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_profile_promotion_report.json"
    );
    let promotion_report: TinyCleanTrainingProfilePromotionReport = serde_json::from_str(
        &fs::read_to_string(promotion_report_path).expect("tiny profile promotion report"),
    )
    .expect("valid tiny profile promotion report json");
    let matrix_manifest_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/tiny_clean_training_profile_experiment_matrix_manifest.json"
    );
    let matrix_manifest: TinyCleanTrainingProfileExperimentMatrixManifest = serde_json::from_str(
        &fs::read_to_string(matrix_manifest_path).expect("tiny profile experiment matrix manifest"),
    )
    .expect("valid tiny profile experiment matrix manifest json");
    let expected_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_profile_experiment_matrix_report.json"
    );
    let expected: TinyCleanTrainingProfileExperimentMatrixReport = serde_json::from_str(
        &fs::read_to_string(expected_path).expect("tiny profile experiment matrix report"),
    )
    .expect("valid tiny profile experiment matrix report json");

    let actual = build_tiny_clean_training_profile_experiment_matrix_report(
        &matrix_manifest,
        &training_manifest,
        &registry,
        &rules,
        &acceptance_report,
        &suite_manifest,
        &strategy_report,
        &promotion_report,
        &clean_corpus_manifest,
        &clean_corpus_pack,
    )
    .expect("tiny profile experiment matrix report");

    assert_eq!(actual, expected);
}

#[test]
fn foundation_overview_report_matches_expected_regression_artifact() {
    let corpus_summary_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/source_acceptance_summary_report.json"
    );
    let corpus_summary: SourceAcceptanceSummaryReport = serde_json::from_str(
        &fs::read_to_string(corpus_summary_path).expect("source acceptance summary report"),
    )
    .expect("valid source acceptance summary report json");
    let corpus_delta_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/source_acceptance_delta_report.json"
    );
    let corpus_delta: SourceAcceptanceDeltaReport = serde_json::from_str(
        &fs::read_to_string(corpus_delta_path).expect("source acceptance delta report"),
    )
    .expect("valid source acceptance delta report json");
    let tokenizer_report_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/eval/tokenizer_experiment_report.json"
    );
    let tokenizer_report: TokenizerExperimentReport = serde_json::from_str(
        &fs::read_to_string(tokenizer_report_path).expect("tokenizer experiment report"),
    )
    .expect("valid tokenizer experiment report json");
    let tokenizer_delta_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/eval/tokenizer_experiment_delta_report.json"
    );
    let tokenizer_delta: TokenizerExperimentDeltaReport = serde_json::from_str(
        &fs::read_to_string(tokenizer_delta_path).expect("tokenizer experiment delta report"),
    )
    .expect("valid tokenizer experiment delta report json");
    let eval_report_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/eval/benchmark_report.json"
    );
    let eval_report: EvalBenchmarkReport =
        serde_json::from_str(&fs::read_to_string(eval_report_path).expect("benchmark report"))
            .expect("valid benchmark report json");
    let eval_delta_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/eval/benchmark_delta_report.json"
    );
    let eval_delta: EvalBenchmarkDeltaReport =
        serde_json::from_str(&fs::read_to_string(eval_delta_path).expect("benchmark delta report"))
            .expect("valid benchmark delta report json");
    let training_consistency_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/baseline_training_consistency_report.json"
    );
    let training_consistency: BaselineTrainingConsistencyReport = serde_json::from_str(
        &fs::read_to_string(training_consistency_path).expect("training consistency report"),
    )
    .expect("valid training consistency report json");
    let training_delta_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/baseline_training_delta_report.json"
    );
    let training_delta: BaselineTrainingDeltaReport = serde_json::from_str(
        &fs::read_to_string(training_delta_path).expect("training delta report"),
    )
    .expect("valid training delta report json");
    let tiny_training_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_report.json"
    );
    let tiny_training: TinyCleanTrainingReport = serde_json::from_str(
        &fs::read_to_string(tiny_training_path).expect("tiny clean training report"),
    )
    .expect("valid tiny clean training report json");
    let tiny_profile_policy_delta_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_profile_baseline_delta_report.json"
    );
    let tiny_profile_policy_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_profile_baseline_report.json"
    );
    let tiny_profile_policy: TinyCleanTrainingProfileBaselineReport = serde_json::from_str(
        &fs::read_to_string(tiny_profile_policy_path).expect("tiny profile baseline report"),
    )
    .expect("valid tiny profile baseline report json");
    let tiny_profile_policy_delta: TinyCleanTrainingProfileBaselineDeltaReport =
        serde_json::from_str(
            &fs::read_to_string(tiny_profile_policy_delta_path)
                .expect("tiny profile baseline delta report"),
        )
        .expect("valid tiny profile baseline delta report json");
    let tiny_profile_strategy_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_profile_strategy_report.json"
    );
    let tiny_profile_strategy: TinyCleanTrainingProfileStrategyReport = serde_json::from_str(
        &fs::read_to_string(tiny_profile_strategy_path).expect("tiny profile strategy report"),
    )
    .expect("valid tiny profile strategy report json");
    let tiny_profile_strategy_delta_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_profile_strategy_delta_report.json"
    );
    let tiny_profile_strategy_delta: TinyCleanTrainingProfileStrategyDeltaReport =
        serde_json::from_str(
            &fs::read_to_string(tiny_profile_strategy_delta_path)
                .expect("tiny profile strategy delta report"),
        )
        .expect("valid tiny profile strategy delta report json");
    let tiny_profile_promotion_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_profile_promotion_report.json"
    );
    let tiny_profile_promotion: TinyCleanTrainingProfilePromotionReport = serde_json::from_str(
        &fs::read_to_string(tiny_profile_promotion_path).expect("tiny profile promotion report"),
    )
    .expect("valid tiny profile promotion report json");
    let tiny_profile_promotion_delta_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_profile_promotion_delta_report.json"
    );
    let tiny_profile_promotion_delta: TinyCleanTrainingProfilePromotionDeltaReport =
        serde_json::from_str(
            &fs::read_to_string(tiny_profile_promotion_delta_path)
                .expect("tiny profile promotion delta report"),
        )
        .expect("valid tiny profile promotion delta report json");
    let expected_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/foundation/foundation_overview_report.json"
    );
    let expected: FoundationOverviewReport = serde_json::from_str(
        &fs::read_to_string(expected_path).expect("foundation overview report"),
    )
    .expect("valid foundation overview report json");

    let actual = build_foundation_overview_report(
        &corpus_summary,
        &corpus_delta,
        &tokenizer_report,
        &tokenizer_delta,
        &eval_report,
        &eval_delta,
        &training_consistency,
        &training_delta,
        &tiny_training,
        &tiny_profile_policy,
        &tiny_profile_policy_delta,
        &tiny_profile_strategy,
        &tiny_profile_strategy_delta,
        &tiny_profile_promotion,
        &tiny_profile_promotion_delta,
    );

    assert_eq!(actual, expected);
}

#[test]
fn foundation_overview_policy_readiness_tracks_substantive_policy_checks() {
    let corpus_summary_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/source_acceptance_summary_report.json"
    );
    let corpus_summary: SourceAcceptanceSummaryReport = serde_json::from_str(
        &fs::read_to_string(corpus_summary_path).expect("source acceptance summary report"),
    )
    .expect("valid source acceptance summary report json");
    let corpus_delta_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/curated/source_acceptance_delta_report.json"
    );
    let corpus_delta: SourceAcceptanceDeltaReport = serde_json::from_str(
        &fs::read_to_string(corpus_delta_path).expect("source acceptance delta report"),
    )
    .expect("valid source acceptance delta report json");
    let tokenizer_report_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/eval/tokenizer_experiment_report.json"
    );
    let tokenizer_report: TokenizerExperimentReport = serde_json::from_str(
        &fs::read_to_string(tokenizer_report_path).expect("tokenizer experiment report"),
    )
    .expect("valid tokenizer experiment report json");
    let tokenizer_delta_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/eval/tokenizer_experiment_delta_report.json"
    );
    let tokenizer_delta: TokenizerExperimentDeltaReport = serde_json::from_str(
        &fs::read_to_string(tokenizer_delta_path).expect("tokenizer experiment delta report"),
    )
    .expect("valid tokenizer experiment delta report json");
    let eval_report_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/eval/benchmark_report.json"
    );
    let eval_report: EvalBenchmarkReport =
        serde_json::from_str(&fs::read_to_string(eval_report_path).expect("eval benchmark report"))
            .expect("valid eval benchmark report json");
    let eval_delta_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/eval/benchmark_delta_report.json"
    );
    let eval_delta: EvalBenchmarkDeltaReport = serde_json::from_str(
        &fs::read_to_string(eval_delta_path).expect("eval benchmark delta report"),
    )
    .expect("valid eval benchmark delta report json");
    let training_consistency_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/baseline_training_consistency_report.json"
    );
    let training_consistency: BaselineTrainingConsistencyReport = serde_json::from_str(
        &fs::read_to_string(training_consistency_path).expect("baseline training consistency"),
    )
    .expect("valid baseline training consistency report json");
    let training_delta_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/baseline_training_delta_report.json"
    );
    let training_delta: BaselineTrainingDeltaReport = serde_json::from_str(
        &fs::read_to_string(training_delta_path).expect("baseline training delta"),
    )
    .expect("valid baseline training delta report json");
    let tiny_training_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_report.json"
    );
    let tiny_training: TinyCleanTrainingReport = serde_json::from_str(
        &fs::read_to_string(tiny_training_path).expect("tiny clean training report"),
    )
    .expect("valid tiny clean training report json");
    let baseline_report_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_profile_baseline_report.json"
    );
    let mut baseline_report: TinyCleanTrainingProfileBaselineReport = serde_json::from_str(
        &fs::read_to_string(baseline_report_path).expect("tiny profile baseline report"),
    )
    .expect("valid tiny profile baseline report json");
    baseline_report.matches_expected_best_profile = false;
    let baseline_delta_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_profile_baseline_delta_report.json"
    );
    let baseline_delta: TinyCleanTrainingProfileBaselineDeltaReport = serde_json::from_str(
        &fs::read_to_string(baseline_delta_path).expect("tiny profile baseline delta report"),
    )
    .expect("valid tiny profile baseline delta report json");
    let strategy_report_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_profile_strategy_report.json"
    );
    let strategy_report: TinyCleanTrainingProfileStrategyReport = serde_json::from_str(
        &fs::read_to_string(strategy_report_path).expect("tiny profile strategy report"),
    )
    .expect("valid tiny profile strategy report json");
    let strategy_delta_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_profile_strategy_delta_report.json"
    );
    let strategy_delta: TinyCleanTrainingProfileStrategyDeltaReport = serde_json::from_str(
        &fs::read_to_string(strategy_delta_path).expect("tiny profile strategy delta report"),
    )
    .expect("valid tiny profile strategy delta report json");
    let promotion_report_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_profile_promotion_report.json"
    );
    let promotion_report: TinyCleanTrainingProfilePromotionReport = serde_json::from_str(
        &fs::read_to_string(promotion_report_path).expect("tiny profile promotion report"),
    )
    .expect("valid tiny profile promotion report json");
    let promotion_delta_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/training/tiny_clean_training_profile_promotion_delta_report.json"
    );
    let promotion_delta: TinyCleanTrainingProfilePromotionDeltaReport = serde_json::from_str(
        &fs::read_to_string(promotion_delta_path).expect("tiny profile promotion delta report"),
    )
    .expect("valid tiny profile promotion delta report json");

    let actual = build_foundation_overview_report(
        &corpus_summary,
        &corpus_delta,
        &tokenizer_report,
        &tokenizer_delta,
        &eval_report,
        &eval_delta,
        &training_consistency,
        &training_delta,
        &tiny_training,
        &baseline_report,
        &baseline_delta,
        &strategy_report,
        &strategy_delta,
        &promotion_report,
        &promotion_delta,
    );

    assert!(!actual.tiny_profile_policy_matches_expected);
    assert!(actual.tiny_profile_strategy_matches_expected);
    assert!(actual.tiny_profile_promotion_matches_expected);
    assert!(!actual.all_layers_match_expected);
}

#[test]
fn foundation_overview_delta_report_matches_expected_regression_artifact() {
    let overview_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/foundation/foundation_overview_report.json"
    );
    let overview: FoundationOverviewReport =
        serde_json::from_str(&fs::read_to_string(overview_path).expect("foundation overview"))
            .expect("valid foundation overview report json");
    let expected_delta_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/foundation/foundation_overview_delta_report.json"
    );
    let expected_delta: FoundationOverviewDeltaReport = serde_json::from_str(
        &fs::read_to_string(expected_delta_path).expect("foundation overview delta report"),
    )
    .expect("valid foundation overview delta report json");

    let actual = build_foundation_overview_delta_report(&overview, &overview);

    assert_eq!(actual, expected_delta);
}
