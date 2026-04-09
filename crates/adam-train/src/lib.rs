use std::collections::BTreeMap;

use adam_corpus::{
    CorpusManifest, SourceAcceptanceRecord, SourceAcceptanceReport, SourceDomain, SourceRegistry,
    SourceRegistryEntry, SourceScoringRules, SourceType, build_source_acceptance_report,
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
    pub validation_split_bps: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineTrainingPlan {
    pub run_name: String,
    pub corpus_name: String,
    pub accepted_source_count: usize,
    pub rejected_source_count: usize,
    pub tokenizer_experiment_name: String,
    pub eval_task_count: usize,
    pub max_steps: u32,
    pub batch_token_budget: u32,
    pub context_window: u32,
    pub validation_split_bps: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineTrainingSourceAllocation {
    pub source_id: String,
    pub domain: SourceDomain,
    pub source_type: SourceType,
    pub score: i32,
    pub allocation_weight: u64,
    pub train_sequence_count: u64,
    pub validation_sequence_count: u64,
    pub train_token_budget: u64,
    pub validation_token_budget: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineTrainingAssemblyReport {
    pub run_name: String,
    pub corpus_name: String,
    pub accepted_source_count: usize,
    pub rejected_source_count: usize,
    pub tokenizer_experiment_name: String,
    pub eval_task_count: usize,
    pub max_steps: u32,
    pub batch_token_budget: u32,
    pub context_window: u32,
    pub total_token_budget: u64,
    pub assigned_token_budget: u64,
    pub remainder_token_budget: u64,
    pub total_sequence_count: u64,
    pub train_token_budget: u64,
    pub validation_token_budget: u64,
    pub train_sequence_count: u64,
    pub validation_sequence_count: u64,
    pub validation_split_bps: u16,
    pub source_allocations: Vec<BaselineTrainingSourceAllocation>,
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
    #[error("validation split must be between 1 and 9999 basis points")]
    InvalidValidationSplit,
    #[error("referenced manifests do not satisfy current training contracts")]
    ReferencedManifestInvalid,
    #[error("source acceptance report does not match current registry and rules")]
    AcceptanceReportMismatch,
    #[error("training manifest linkage is inconsistent")]
    ManifestLinkageMismatch,
    #[error("baseline training requires at least one accepted source")]
    NoAcceptedTrainingSources,
    #[error("training token budget is insufficient for at least one full sequence")]
    InsufficientSequenceBudget,
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

        if self.validation_split_bps == 0 || self.validation_split_bps >= 10_000 {
            return Err(TrainingError::InvalidValidationSplit);
        }

        Ok(())
    }
}

pub fn build_baseline_training_plan(
    manifest: &BaselineTrainingManifest,
    corpus: &CorpusManifest,
    registry: &SourceRegistry,
    rules: &SourceScoringRules,
    report: &SourceAcceptanceReport,
    tokenizer_experiment: &TokenizerExperiment,
    eval_suite: &EvalSuite,
) -> Result<BaselineTrainingPlan, TrainingError> {
    let summary = summarize_training_inputs(
        manifest,
        corpus,
        registry,
        rules,
        report,
        tokenizer_experiment,
        eval_suite,
    )?;

    Ok(BaselineTrainingPlan {
        run_name: manifest.run_name.clone(),
        corpus_name: corpus.name.clone(),
        accepted_source_count: summary.accepted_sources.len(),
        rejected_source_count: summary.rejected_source_count,
        tokenizer_experiment_name: tokenizer_experiment.name.clone(),
        eval_task_count: eval_suite.tasks.len(),
        max_steps: manifest.max_steps,
        batch_token_budget: manifest.batch_token_budget,
        context_window: manifest.context_window,
        validation_split_bps: manifest.validation_split_bps,
    })
}

pub fn build_baseline_training_assembly_report(
    manifest: &BaselineTrainingManifest,
    corpus: &CorpusManifest,
    registry: &SourceRegistry,
    rules: &SourceScoringRules,
    report: &SourceAcceptanceReport,
    tokenizer_experiment: &TokenizerExperiment,
    eval_suite: &EvalSuite,
) -> Result<BaselineTrainingAssemblyReport, TrainingError> {
    let summary = summarize_training_inputs(
        manifest,
        corpus,
        registry,
        rules,
        report,
        tokenizer_experiment,
        eval_suite,
    )?;

    let total_token_budget = manifest.max_steps as u64 * manifest.batch_token_budget as u64;
    let total_sequence_count = total_token_budget / manifest.context_window as u64;
    if total_sequence_count == 0 {
        return Err(TrainingError::InsufficientSequenceBudget);
    }

    let source_totals = allocate_sequences_by_weight(
        total_sequence_count,
        &summary
            .accepted_sources
            .iter()
            .map(|source| source.weight)
            .collect::<Vec<_>>(),
    );
    let validation_sequences =
        allocate_validation_sequences(&source_totals, manifest.validation_split_bps);

    let source_allocations = summary
        .accepted_sources
        .iter()
        .zip(source_totals.iter().zip(validation_sequences.iter()))
        .map(|(source, (total_sequences, validation_sequence_count))| {
            let train_sequence_count = total_sequences - validation_sequence_count;
            BaselineTrainingSourceAllocation {
                source_id: source.entry.id.clone(),
                domain: source.entry.domain.clone(),
                source_type: source.entry.source_type.clone(),
                score: source.record.score,
                allocation_weight: source.weight,
                train_sequence_count,
                validation_sequence_count: *validation_sequence_count,
                train_token_budget: train_sequence_count * manifest.context_window as u64,
                validation_token_budget: validation_sequence_count * manifest.context_window as u64,
            }
        })
        .collect::<Vec<_>>();

    let train_sequence_count = source_allocations
        .iter()
        .map(|source| source.train_sequence_count)
        .sum();
    let validation_sequence_count = source_allocations
        .iter()
        .map(|source| source.validation_sequence_count)
        .sum();
    let train_token_budget = source_allocations
        .iter()
        .map(|source| source.train_token_budget)
        .sum();
    let validation_token_budget = source_allocations
        .iter()
        .map(|source| source.validation_token_budget)
        .sum();
    let assigned_token_budget = train_token_budget + validation_token_budget;

    Ok(BaselineTrainingAssemblyReport {
        run_name: manifest.run_name.clone(),
        corpus_name: corpus.name.clone(),
        accepted_source_count: summary.accepted_sources.len(),
        rejected_source_count: summary.rejected_source_count,
        tokenizer_experiment_name: tokenizer_experiment.name.clone(),
        eval_task_count: eval_suite.tasks.len(),
        max_steps: manifest.max_steps,
        batch_token_budget: manifest.batch_token_budget,
        context_window: manifest.context_window,
        total_token_budget,
        assigned_token_budget,
        remainder_token_budget: total_token_budget - assigned_token_budget,
        total_sequence_count,
        train_token_budget,
        validation_token_budget,
        train_sequence_count,
        validation_sequence_count,
        validation_split_bps: manifest.validation_split_bps,
        source_allocations,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AcceptedTrainingSource<'a> {
    entry: &'a SourceRegistryEntry,
    record: &'a SourceAcceptanceRecord,
    weight: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TrainingInputSummary<'a> {
    accepted_sources: Vec<AcceptedTrainingSource<'a>>,
    rejected_source_count: usize,
}

fn summarize_training_inputs<'a>(
    manifest: &BaselineTrainingManifest,
    corpus: &CorpusManifest,
    registry: &'a SourceRegistry,
    rules: &SourceScoringRules,
    report: &'a SourceAcceptanceReport,
    tokenizer_experiment: &TokenizerExperiment,
    eval_suite: &EvalSuite,
) -> Result<TrainingInputSummary<'a>, TrainingError> {
    manifest.validate()?;
    corpus
        .validate()
        .map_err(|_| TrainingError::ReferencedManifestInvalid)?;
    registry
        .validate()
        .map_err(|_| TrainingError::ReferencedManifestInvalid)?;
    report
        .validate(registry, rules)
        .map_err(|_| TrainingError::AcceptanceReportMismatch)?;
    tokenizer_experiment
        .validate()
        .map_err(|_| TrainingError::ReferencedManifestInvalid)?;
    eval_suite
        .validate()
        .map_err(|_| TrainingError::ReferencedManifestInvalid)?;

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

    if tokenizer_experiment.training_manifest != manifest.corpus_manifest
        || manifest.source_registry_manifest != report.source_registry_manifest
        || manifest.scoring_rules_manifest != report.scoring_rules_manifest
    {
        return Err(TrainingError::ManifestLinkageMismatch);
    }

    let registry_by_id = registry
        .entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let mut accepted_sources = report
        .records
        .iter()
        .filter(|record| record.accepted_for_training)
        .map(|record| {
            let entry = *registry_by_id
                .get(record.source_id.as_str())
                .ok_or(TrainingError::AcceptanceReportMismatch)?;
            Ok(AcceptedTrainingSource {
                entry,
                record,
                weight: record.score.max(1) as u64,
            })
        })
        .collect::<Result<Vec<_>, TrainingError>>()?;
    accepted_sources.sort_by(|left, right| left.entry.id.cmp(&right.entry.id));

    if accepted_sources.is_empty() {
        return Err(TrainingError::NoAcceptedTrainingSources);
    }

    Ok(TrainingInputSummary {
        rejected_source_count: report.records.len() - accepted_sources.len(),
        accepted_sources,
    })
}

fn allocate_sequences_by_weight(total_sequences: u64, weights: &[u64]) -> Vec<u64> {
    if weights.is_empty() {
        return Vec::new();
    }

    let weight_sum = weights.iter().sum::<u64>() as u128;
    let mut allocations = weights
        .iter()
        .enumerate()
        .map(|(index, weight)| {
            let numerator = total_sequences as u128 * *weight as u128;
            (
                index,
                (numerator / weight_sum) as u64,
                numerator % weight_sum,
            )
        })
        .collect::<Vec<_>>();

    let assigned = allocations.iter().map(|(_, share, _)| *share).sum::<u64>();
    let mut remaining = total_sequences - assigned;
    allocations.sort_by(|left, right| right.2.cmp(&left.2).then_with(|| left.0.cmp(&right.0)));
    for allocation in &mut allocations {
        if remaining == 0 {
            break;
        }
        allocation.1 += 1;
        remaining -= 1;
    }
    allocations.sort_by_key(|(index, _, _)| *index);

    allocations
        .into_iter()
        .map(|(_, allocation, _)| allocation)
        .collect()
}

fn allocate_validation_sequences(source_totals: &[u64], split_bps: u16) -> Vec<u64> {
    let total_sequences = source_totals.iter().sum::<u64>();
    if total_sequences == 0 {
        return vec![0; source_totals.len()];
    }

    let mut target_validation_sequences =
        (total_sequences as u128 * split_bps as u128 / 10_000) as u64;
    if split_bps > 0 && total_sequences > 1 && target_validation_sequences == 0 {
        target_validation_sequences = 1;
    }
    if target_validation_sequences >= total_sequences {
        target_validation_sequences = total_sequences - 1;
    }

    let mut allocations = source_totals
        .iter()
        .enumerate()
        .map(|(index, total)| {
            let numerator = *total as u128 * split_bps as u128;
            (index, (numerator / 10_000) as u64, numerator % 10_000)
        })
        .collect::<Vec<_>>();

    let assigned = allocations.iter().map(|(_, share, _)| *share).sum::<u64>();
    let mut remaining = target_validation_sequences.saturating_sub(assigned);
    allocations.sort_by(|left, right| right.2.cmp(&left.2).then_with(|| left.0.cmp(&right.0)));
    for allocation in &mut allocations {
        if remaining == 0 {
            break;
        }
        let source_total = source_totals[allocation.0];
        if allocation.1 < source_total {
            allocation.1 += 1;
            remaining -= 1;
        }
    }
    allocations.sort_by_key(|(index, _, _)| *index);

    allocations
        .into_iter()
        .map(|(_, allocation, _)| allocation)
        .collect()
}

#[cfg(test)]
mod tests {
    use adam_corpus::{
        CorpusManifest, CorpusStage, LicenseClass, QualityTier, SourceAcceptanceRecord,
        SourceAcceptanceReport, SourceDomain, SourcePolicy, SourceRegistry, SourceRegistryEntry,
        SourceScoringRules, SourceType,
    };
    use adam_eval::EvalSuite;
    use adam_tokenizer::TokenizerExperiment;

    use crate::{
        BaselineTrainingManifest, TrainingError, build_baseline_training_assembly_report,
        build_baseline_training_plan,
    };

    #[test]
    fn rejects_empty_training_objective() {
        let manifest = BaselineTrainingManifest {
            version: "0.0.40".to_string(),
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
            validation_split_bps: 1000,
        };

        assert_eq!(manifest.validate(), Err(TrainingError::EmptyObjective));
    }

    #[test]
    fn builds_baseline_training_plan_from_valid_contracts() {
        let manifest = BaselineTrainingManifest {
            version: "0.0.40".to_string(),
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
            validation_split_bps: 1000,
        };
        let corpus = CorpusManifest {
            version: "0.0.40".to_string(),
            name: "adam-foundation-curated".to_string(),
            language: "kazakh".to_string(),
            script: "cyrillic".to_string(),
            stage: CorpusStage::Curated,
            source_policy: SourcePolicy::KazakhOnly,
            domains: vec![
                "general_text".to_string(),
                "administrative_text".to_string(),
                "reference_text".to_string(),
            ],
        };
        let registry = SourceRegistry {
            version: "0.0.40".to_string(),
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
            version: "0.0.40".to_string(),
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
            version: "0.0.40".to_string(),
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
            version: "0.0.40".to_string(),
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
        assert_eq!(plan.validation_split_bps, 1000);
    }

    #[test]
    fn builds_deterministic_training_assembly_report() {
        let manifest = BaselineTrainingManifest {
            version: "0.0.40".to_string(),
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
            objective: "assemble deterministic baseline training split".to_string(),
            max_steps: 128,
            batch_token_budget: 8192,
            context_window: 1024,
            validation_split_bps: 1000,
        };
        let corpus = CorpusManifest {
            version: "0.0.40".to_string(),
            name: "adam-foundation-curated".to_string(),
            language: "kazakh".to_string(),
            script: "cyrillic".to_string(),
            stage: CorpusStage::Curated,
            source_policy: SourcePolicy::KazakhOnly,
            domains: vec![
                "general_text".to_string(),
                "administrative_text".to_string(),
                "reference_text".to_string(),
            ],
        };
        let registry = SourceRegistry {
            version: "0.0.40".to_string(),
            entries: vec![
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
            ],
        };
        let rules = SourceScoringRules {
            version: "0.0.40".to_string(),
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
            version: "0.0.40".to_string(),
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
            version: "0.0.40".to_string(),
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
        assert_eq!(report.total_token_budget, 1_048_576);
        assert_eq!(report.total_sequence_count, 1024);
        assert_eq!(report.validation_sequence_count, 102);
        assert_eq!(report.train_sequence_count, 922);
        assert_eq!(report.remainder_token_budget, 0);
        assert_eq!(report.source_allocations.len(), 1);
        assert_eq!(
            report.source_allocations[0].source_id,
            "curated_reference_kazakh"
        );
        assert_eq!(report.source_allocations[0].validation_sequence_count, 102);
    }
}
