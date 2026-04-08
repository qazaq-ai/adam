use adam_corpus::{
    SourceAcceptanceReport, SourceRegistry, SourceScoringRules, build_source_acceptance_report,
};
use adam_eval::EvalSuite;
use adam_tokenizer::TokenizerExperiment;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineTrainingManifest {
    pub version: String,
    pub run_name: String,
    pub target_language: String,
    pub script: String,
    pub corpus_manifest: String,
    pub source_registry_manifest: String,
    pub scoring_rules_manifest: String,
    pub acceptance_report_manifest: String,
    pub tokenizer_experiment_manifest: String,
    pub eval_suite_manifest: String,
    pub objective: String,
    pub max_steps: u32,
    pub batch_token_budget: u32,
    pub context_window: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineTrainingPlan {
    pub run_name: String,
    pub accepted_source_count: usize,
    pub rejected_source_count: usize,
    pub tokenizer_experiment_name: String,
    pub eval_task_count: usize,
    pub max_steps: u32,
    pub batch_token_budget: u32,
    pub context_window: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TrainingError {
    #[error("training language must be kazakh")]
    NonKazakhLanguage,
    #[error("training script must be cyrillic")]
    NonCyrillicScript,
    #[error("training objective must not be empty")]
    EmptyObjective,
    #[error("training manifest references must not be empty")]
    EmptyManifestReference,
    #[error("training steps and budgets must be positive")]
    NonPositiveTrainingBudget,
    #[error("source acceptance report does not match current registry and rules")]
    AcceptanceReportMismatch,
}

impl BaselineTrainingManifest {
    pub fn validate(&self) -> Result<(), TrainingError> {
        if self.target_language != "kazakh" {
            return Err(TrainingError::NonKazakhLanguage);
        }

        if self.script != "cyrillic" {
            return Err(TrainingError::NonCyrillicScript);
        }

        if self.objective.trim().is_empty() {
            return Err(TrainingError::EmptyObjective);
        }

        if [
            &self.corpus_manifest,
            &self.source_registry_manifest,
            &self.scoring_rules_manifest,
            &self.acceptance_report_manifest,
            &self.tokenizer_experiment_manifest,
            &self.eval_suite_manifest,
        ]
        .iter()
        .any(|value| value.trim().is_empty())
        {
            return Err(TrainingError::EmptyManifestReference);
        }

        if self.max_steps == 0 || self.batch_token_budget == 0 || self.context_window == 0 {
            return Err(TrainingError::NonPositiveTrainingBudget);
        }

        Ok(())
    }
}

pub fn build_baseline_training_plan(
    manifest: &BaselineTrainingManifest,
    registry: &SourceRegistry,
    rules: &SourceScoringRules,
    report: &SourceAcceptanceReport,
    tokenizer_experiment: &TokenizerExperiment,
    eval_suite: &EvalSuite,
) -> Result<BaselineTrainingPlan, TrainingError> {
    manifest.validate()?;
    registry
        .validate()
        .map_err(|_| TrainingError::AcceptanceReportMismatch)?;
    report
        .validate(registry, rules)
        .map_err(|_| TrainingError::AcceptanceReportMismatch)?;
    tokenizer_experiment
        .validate()
        .map_err(|_| TrainingError::AcceptanceReportMismatch)?;
    eval_suite
        .validate()
        .map_err(|_| TrainingError::AcceptanceReportMismatch)?;

    let expected_report = build_source_acceptance_report(
        &report.name,
        &report.source_registry_manifest,
        &report.scoring_rules_manifest,
        registry,
        rules,
    )
    .map_err(|_| TrainingError::AcceptanceReportMismatch)?;

    if &expected_report != report {
        return Err(TrainingError::AcceptanceReportMismatch);
    }

    let accepted_source_count = report
        .records
        .iter()
        .filter(|record| record.accepted_for_training)
        .count();
    let rejected_source_count = report.records.len() - accepted_source_count;

    Ok(BaselineTrainingPlan {
        run_name: manifest.run_name.clone(),
        accepted_source_count,
        rejected_source_count,
        tokenizer_experiment_name: tokenizer_experiment.name.clone(),
        eval_task_count: eval_suite.tasks.len(),
        max_steps: manifest.max_steps,
        batch_token_budget: manifest.batch_token_budget,
        context_window: manifest.context_window,
    })
}

#[cfg(test)]
mod tests {
    use adam_corpus::{
        CorpusStage, LicenseClass, QualityTier, SourceAcceptanceRecord, SourceAcceptanceReport,
        SourceDomain, SourceRegistry, SourceRegistryEntry, SourceScoringRules, SourceType,
    };
    use adam_eval::EvalSuite;
    use adam_tokenizer::TokenizerExperiment;

    use crate::{BaselineTrainingManifest, TrainingError, build_baseline_training_plan};

    #[test]
    fn rejects_empty_training_objective() {
        let manifest = BaselineTrainingManifest {
            version: "0.0.28".to_string(),
            run_name: "baseline".to_string(),
            target_language: "kazakh".to_string(),
            script: "cyrillic".to_string(),
            corpus_manifest: "data/curated/corpus_manifest.json".to_string(),
            source_registry_manifest: "data/raw/source_registry.json".to_string(),
            scoring_rules_manifest: "data/raw/source_scoring_rules.json".to_string(),
            acceptance_report_manifest: "data/curated/source_acceptance_report.json".to_string(),
            tokenizer_experiment_manifest: "data/eval/tokenizer_experiment_manifest.json"
                .to_string(),
            eval_suite_manifest: "data/eval/benchmark_manifest.json".to_string(),
            objective: String::new(),
            max_steps: 128,
            batch_token_budget: 8192,
            context_window: 1024,
        };

        assert_eq!(manifest.validate(), Err(TrainingError::EmptyObjective));
    }

    #[test]
    fn builds_baseline_training_plan_from_valid_contracts() {
        let manifest = BaselineTrainingManifest {
            version: "0.0.28".to_string(),
            run_name: "adam-baseline-plan".to_string(),
            target_language: "kazakh".to_string(),
            script: "cyrillic".to_string(),
            corpus_manifest: "data/curated/corpus_manifest.json".to_string(),
            source_registry_manifest: "data/raw/source_registry.json".to_string(),
            scoring_rules_manifest: "data/raw/source_scoring_rules.json".to_string(),
            acceptance_report_manifest: "data/curated/source_acceptance_report.json".to_string(),
            tokenizer_experiment_manifest: "data/eval/tokenizer_experiment_manifest.json"
                .to_string(),
            eval_suite_manifest: "data/eval/benchmark_manifest.json".to_string(),
            objective: "plan first kazakh-only baseline training run".to_string(),
            max_steps: 128,
            batch_token_budget: 8192,
            context_window: 1024,
        };
        let registry = SourceRegistry {
            version: "0.0.28".to_string(),
            entries: vec![
                SourceRegistryEntry {
                    id: "seed_public_admin_text".to_string(),
                    stage: CorpusStage::Raw,
                    language: "kazakh".to_string(),
                    script: "cyrillic".to_string(),
                    source_type: SourceType::AdministrativeText,
                    domain: SourceDomain::Administrative,
                    license_class: LicenseClass::ReviewRequired,
                    quality_tier: QualityTier::Seed,
                    provenance: "manual_seed".to_string(),
                    allowed_for_training: false,
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
            ],
        };
        let rules = SourceScoringRules {
            version: "0.0.28".to_string(),
            minimum_acceptance_score: 3,
            open_license_bonus: 3,
            reviewed_quality_bonus: 2,
            training_ready_bonus: 4,
            administrative_domain_bonus: 1,
            reference_domain_bonus: 1,
            raw_stage_penalty: 3,
            review_required_penalty: 3,
            internal_only_penalty: 5,
            seed_quality_penalty: 2,
        };
        let report = SourceAcceptanceReport {
            version: "0.0.28".to_string(),
            name: "adam-source-acceptance-report".to_string(),
            source_registry_manifest: "data/raw/source_registry.json".to_string(),
            scoring_rules_manifest: "data/raw/source_scoring_rules.json".to_string(),
            records: vec![
                SourceAcceptanceRecord {
                    source_id: "curated_reference_kazakh".to_string(),
                    score: 8,
                    accepted_for_training: true,
                    positive_signals: vec![
                        "open_license".to_string(),
                        "reference_domain".to_string(),
                        "training_ready_quality".to_string(),
                    ],
                    negative_signals: Vec::new(),
                },
                SourceAcceptanceRecord {
                    source_id: "seed_public_admin_text".to_string(),
                    score: -7,
                    accepted_for_training: false,
                    positive_signals: vec!["administrative_domain".to_string()],
                    negative_signals: vec![
                        "raw_stage".to_string(),
                        "review_required_license".to_string(),
                        "seed_quality".to_string(),
                    ],
                },
            ],
        };
        let experiment = TokenizerExperiment {
            version: "0.0.28".to_string(),
            name: "adam-tokenizer-deterministic".to_string(),
            target_language: "kazakh".to_string(),
            script: "cyrillic".to_string(),
            profile_name: "adam-kazakh-cyrillic".to_string(),
            training_manifest: "data/curated/corpus_manifest.json".to_string(),
            sample_pack_manifest: "data/curated/tokenizer_dry_run_pack.json".to_string(),
            segmentation_eval_manifest: "data/eval/tokenizer_segmentation_eval_dataset.json"
                .to_string(),
            segmentation_roots_manifest: "data/tokenizer/segmentation_roots.json".to_string(),
            segmentation_rules_manifest: "data/tokenizer/segmentation_rules.json".to_string(),
            objective: "measure deterministic segmentation quality on kazakh text".to_string(),
        };
        let eval_suite = EvalSuite::default();

        let plan = build_baseline_training_plan(
            &manifest,
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
    }
}
