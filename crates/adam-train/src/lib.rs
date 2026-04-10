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
    pub category_breakdown: Vec<BaselineTrainingAssemblyCategoryReport>,
    pub critical_breakdown: Vec<BaselineTrainingAssemblyGuardReport>,
    pub source_allocations: Vec<BaselineTrainingSourceAllocation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineTrainingAssemblyCategoryReport {
    pub category: String,
    pub source_count: usize,
    pub allocation_weight: u64,
    pub train_sequence_count: u64,
    pub validation_sequence_count: u64,
    pub train_token_budget: u64,
    pub validation_token_budget: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineTrainingAssemblyGuardReport {
    pub guard: String,
    pub source_count: usize,
    pub train_sequence_count: u64,
    pub validation_sequence_count: u64,
    pub total_sequence_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineTrainingConsistencyReport {
    pub run_name: String,
    pub corpus_name: String,
    pub tokenizer_experiment_name: String,
    pub accepted_source_count: usize,
    pub rejected_source_count: usize,
    pub eval_task_count: usize,
    pub max_steps: u32,
    pub batch_token_budget: u32,
    pub context_window: u32,
    pub validation_split_bps: u16,
    pub total_token_budget: u64,
    pub total_sequence_count: u64,
    pub train_sequence_count: u64,
    pub validation_sequence_count: u64,
    pub assembly_category_count: usize,
    pub assembly_guard_count: usize,
    pub source_allocation_count: usize,
    pub consistency_checks: Vec<BaselineTrainingConsistencyCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineTrainingConsistencyCheck {
    pub check: String,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineTrainingDeltaReport {
    pub run_name: String,
    pub assembly_matches_expected: bool,
    pub consistency_matches_expected: bool,
    pub field_drifts: Vec<BaselineTrainingFieldDrift>,
    pub category_drifts: Vec<BaselineTrainingNamedCountDrift>,
    pub guard_drifts: Vec<BaselineTrainingNamedCountDrift>,
    pub source_allocation_drifts: Vec<BaselineTrainingSourceAllocationDrift>,
    pub consistency_check_drifts: Vec<BaselineTrainingNamedBoolDrift>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineTrainingFieldDrift {
    pub field: String,
    pub expected: String,
    pub actual: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineTrainingNamedCountDrift {
    pub scope: String,
    pub key: String,
    pub expected: Option<u64>,
    pub actual: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineTrainingNamedBoolDrift {
    pub scope: String,
    pub key: String,
    pub expected: Option<bool>,
    pub actual: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineTrainingSourceAllocationDrift {
    pub source_id: String,
    pub expected_total_sequence_count: Option<u64>,
    pub actual_total_sequence_count: Option<u64>,
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
    let category_breakdown = build_category_breakdown(&source_allocations);
    let critical_breakdown = build_guard_breakdown(&source_allocations);

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
        category_breakdown,
        critical_breakdown,
        source_allocations,
    })
}

pub fn build_baseline_training_consistency_report(
    manifest: &BaselineTrainingManifest,
    corpus: &CorpusManifest,
    registry: &SourceRegistry,
    rules: &SourceScoringRules,
    report: &SourceAcceptanceReport,
    tokenizer_experiment: &TokenizerExperiment,
    eval_suite: &EvalSuite,
) -> Result<BaselineTrainingConsistencyReport, TrainingError> {
    let plan = build_baseline_training_plan(
        manifest,
        corpus,
        registry,
        rules,
        report,
        tokenizer_experiment,
        eval_suite,
    )?;
    let assembly = build_baseline_training_assembly_report(
        manifest,
        corpus,
        registry,
        rules,
        report,
        tokenizer_experiment,
        eval_suite,
    )?;

    let consistency_checks = vec![
        BaselineTrainingConsistencyCheck {
            check: "run_name_matches".to_string(),
            passed: plan.run_name == assembly.run_name,
        },
        BaselineTrainingConsistencyCheck {
            check: "corpus_name_matches".to_string(),
            passed: plan.corpus_name == assembly.corpus_name,
        },
        BaselineTrainingConsistencyCheck {
            check: "tokenizer_experiment_name_matches".to_string(),
            passed: plan.tokenizer_experiment_name == assembly.tokenizer_experiment_name,
        },
        BaselineTrainingConsistencyCheck {
            check: "accepted_source_count_matches".to_string(),
            passed: plan.accepted_source_count == assembly.accepted_source_count,
        },
        BaselineTrainingConsistencyCheck {
            check: "rejected_source_count_matches".to_string(),
            passed: plan.rejected_source_count == assembly.rejected_source_count,
        },
        BaselineTrainingConsistencyCheck {
            check: "eval_task_count_matches".to_string(),
            passed: plan.eval_task_count == assembly.eval_task_count,
        },
        BaselineTrainingConsistencyCheck {
            check: "training_budget_matches".to_string(),
            passed: plan.max_steps == assembly.max_steps
                && plan.batch_token_budget == assembly.batch_token_budget
                && plan.context_window == assembly.context_window
                && plan.validation_split_bps == assembly.validation_split_bps,
        },
        BaselineTrainingConsistencyCheck {
            check: "token_budget_matches_sequences".to_string(),
            passed: assembly.total_token_budget
                == assembly.max_steps as u64 * assembly.batch_token_budget as u64
                && assembly.total_sequence_count
                    == assembly.total_token_budget / assembly.context_window as u64,
        },
        BaselineTrainingConsistencyCheck {
            check: "assigned_budget_matches_split_totals".to_string(),
            passed: assembly.assigned_token_budget
                == assembly.train_token_budget + assembly.validation_token_budget
                && assembly.total_sequence_count
                    == assembly.train_sequence_count + assembly.validation_sequence_count,
        },
        BaselineTrainingConsistencyCheck {
            check: "source_allocations_match_global_totals".to_string(),
            passed: assembly
                .source_allocations
                .iter()
                .map(|entry| entry.train_sequence_count)
                .sum::<u64>()
                == assembly.train_sequence_count
                && assembly
                    .source_allocations
                    .iter()
                    .map(|entry| entry.validation_sequence_count)
                    .sum::<u64>()
                    == assembly.validation_sequence_count,
        },
    ];

    Ok(BaselineTrainingConsistencyReport {
        run_name: plan.run_name,
        corpus_name: plan.corpus_name,
        tokenizer_experiment_name: plan.tokenizer_experiment_name,
        accepted_source_count: plan.accepted_source_count,
        rejected_source_count: plan.rejected_source_count,
        eval_task_count: plan.eval_task_count,
        max_steps: plan.max_steps,
        batch_token_budget: plan.batch_token_budget,
        context_window: plan.context_window,
        validation_split_bps: plan.validation_split_bps,
        total_token_budget: assembly.total_token_budget,
        total_sequence_count: assembly.total_sequence_count,
        train_sequence_count: assembly.train_sequence_count,
        validation_sequence_count: assembly.validation_sequence_count,
        assembly_category_count: assembly.category_breakdown.len(),
        assembly_guard_count: assembly.critical_breakdown.len(),
        source_allocation_count: assembly.source_allocations.len(),
        consistency_checks,
    })
}

pub fn build_baseline_training_delta_report(
    manifest: &BaselineTrainingManifest,
    corpus: &CorpusManifest,
    registry: &SourceRegistry,
    rules: &SourceScoringRules,
    report: &SourceAcceptanceReport,
    tokenizer_experiment: &TokenizerExperiment,
    eval_suite: &EvalSuite,
    expected_assembly: &BaselineTrainingAssemblyReport,
    expected_consistency: &BaselineTrainingConsistencyReport,
) -> Result<BaselineTrainingDeltaReport, TrainingError> {
    let actual_assembly = build_baseline_training_assembly_report(
        manifest,
        corpus,
        registry,
        rules,
        report,
        tokenizer_experiment,
        eval_suite,
    )?;
    let actual_consistency = build_baseline_training_consistency_report(
        manifest,
        corpus,
        registry,
        rules,
        report,
        tokenizer_experiment,
        eval_suite,
    )?;

    Ok(BaselineTrainingDeltaReport {
        run_name: manifest.run_name.clone(),
        assembly_matches_expected: expected_assembly == &actual_assembly,
        consistency_matches_expected: expected_consistency == &actual_consistency,
        field_drifts: build_field_drifts(
            expected_assembly,
            &actual_assembly,
            expected_consistency,
            &actual_consistency,
        ),
        category_drifts: build_named_count_drifts(
            "category",
            expected_assembly
                .category_breakdown
                .iter()
                .map(|entry| {
                    (
                        entry.category.as_str(),
                        entry.train_sequence_count + entry.validation_sequence_count,
                    )
                })
                .collect(),
            actual_assembly
                .category_breakdown
                .iter()
                .map(|entry| {
                    (
                        entry.category.as_str(),
                        entry.train_sequence_count + entry.validation_sequence_count,
                    )
                })
                .collect(),
        ),
        guard_drifts: build_named_count_drifts(
            "guard",
            expected_assembly
                .critical_breakdown
                .iter()
                .map(|entry| (entry.guard.as_str(), entry.total_sequence_count))
                .collect(),
            actual_assembly
                .critical_breakdown
                .iter()
                .map(|entry| (entry.guard.as_str(), entry.total_sequence_count))
                .collect(),
        ),
        source_allocation_drifts: build_source_allocation_drifts(
            &expected_assembly.source_allocations,
            &actual_assembly.source_allocations,
        ),
        consistency_check_drifts: build_named_bool_drifts(
            "consistency_check",
            expected_consistency
                .consistency_checks
                .iter()
                .map(|entry| (entry.check.as_str(), entry.passed))
                .collect(),
            actual_consistency
                .consistency_checks
                .iter()
                .map(|entry| (entry.check.as_str(), entry.passed))
                .collect(),
        ),
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct AssemblyStats {
    source_count: usize,
    allocation_weight: u64,
    train_sequence_count: u64,
    validation_sequence_count: u64,
    train_token_budget: u64,
    validation_token_budget: u64,
}

fn build_category_breakdown(
    source_allocations: &[BaselineTrainingSourceAllocation],
) -> Vec<BaselineTrainingAssemblyCategoryReport> {
    let mut category_stats = BTreeMap::<String, AssemblyStats>::new();

    for allocation in source_allocations {
        for category in assembly_categories(allocation) {
            let stats = category_stats.entry(category).or_default();
            stats.source_count += 1;
            stats.allocation_weight += allocation.allocation_weight;
            stats.train_sequence_count += allocation.train_sequence_count;
            stats.validation_sequence_count += allocation.validation_sequence_count;
            stats.train_token_budget += allocation.train_token_budget;
            stats.validation_token_budget += allocation.validation_token_budget;
        }
    }

    category_stats
        .into_iter()
        .map(|(category, stats)| BaselineTrainingAssemblyCategoryReport {
            category,
            source_count: stats.source_count,
            allocation_weight: stats.allocation_weight,
            train_sequence_count: stats.train_sequence_count,
            validation_sequence_count: stats.validation_sequence_count,
            train_token_budget: stats.train_token_budget,
            validation_token_budget: stats.validation_token_budget,
        })
        .collect()
}

fn build_guard_breakdown(
    source_allocations: &[BaselineTrainingSourceAllocation],
) -> Vec<BaselineTrainingAssemblyGuardReport> {
    let mut guard_stats = BTreeMap::<String, AssemblyStats>::new();
    let source_count = source_allocations.len();

    for allocation in source_allocations {
        for guard in assembly_guards(allocation, source_count) {
            let stats = guard_stats.entry(guard).or_default();
            stats.source_count += 1;
            stats.train_sequence_count += allocation.train_sequence_count;
            stats.validation_sequence_count += allocation.validation_sequence_count;
        }
    }

    guard_stats
        .into_iter()
        .map(|(guard, stats)| BaselineTrainingAssemblyGuardReport {
            guard,
            source_count: stats.source_count,
            train_sequence_count: stats.train_sequence_count,
            validation_sequence_count: stats.validation_sequence_count,
            total_sequence_count: stats.train_sequence_count + stats.validation_sequence_count,
        })
        .collect()
}

fn build_field_drifts(
    expected_assembly: &BaselineTrainingAssemblyReport,
    actual_assembly: &BaselineTrainingAssemblyReport,
    expected_consistency: &BaselineTrainingConsistencyReport,
    actual_consistency: &BaselineTrainingConsistencyReport,
) -> Vec<BaselineTrainingFieldDrift> {
    let mut drifts = Vec::new();
    push_field_drift(
        &mut drifts,
        "accepted_source_count",
        expected_assembly.accepted_source_count,
        actual_assembly.accepted_source_count,
    );
    push_field_drift(
        &mut drifts,
        "rejected_source_count",
        expected_assembly.rejected_source_count,
        actual_assembly.rejected_source_count,
    );
    push_field_drift(
        &mut drifts,
        "total_token_budget",
        expected_assembly.total_token_budget,
        actual_assembly.total_token_budget,
    );
    push_field_drift(
        &mut drifts,
        "total_sequence_count",
        expected_assembly.total_sequence_count,
        actual_assembly.total_sequence_count,
    );
    push_field_drift(
        &mut drifts,
        "train_sequence_count",
        expected_assembly.train_sequence_count,
        actual_assembly.train_sequence_count,
    );
    push_field_drift(
        &mut drifts,
        "validation_sequence_count",
        expected_assembly.validation_sequence_count,
        actual_assembly.validation_sequence_count,
    );
    push_field_drift(
        &mut drifts,
        "assembly_category_count",
        expected_consistency.assembly_category_count,
        actual_consistency.assembly_category_count,
    );
    push_field_drift(
        &mut drifts,
        "assembly_guard_count",
        expected_consistency.assembly_guard_count,
        actual_consistency.assembly_guard_count,
    );
    push_field_drift(
        &mut drifts,
        "source_allocation_count",
        expected_consistency.source_allocation_count,
        actual_consistency.source_allocation_count,
    );
    drifts
}

fn push_field_drift<T: ToString + PartialEq>(
    drifts: &mut Vec<BaselineTrainingFieldDrift>,
    field: &str,
    expected: T,
    actual: T,
) {
    if expected != actual {
        drifts.push(BaselineTrainingFieldDrift {
            field: field.to_string(),
            expected: expected.to_string(),
            actual: actual.to_string(),
        });
    }
}

fn build_named_count_drifts(
    scope: &str,
    expected: Vec<(&str, u64)>,
    actual: Vec<(&str, u64)>,
) -> Vec<BaselineTrainingNamedCountDrift> {
    let mut expected_map = expected.into_iter().collect::<BTreeMap<_, _>>();
    let mut actual_map = actual.into_iter().collect::<BTreeMap<_, _>>();
    let mut keys = expected_map
        .keys()
        .chain(actual_map.keys())
        .copied()
        .collect::<Vec<_>>();
    keys.sort_unstable();
    keys.dedup();

    let mut drifts = Vec::new();
    for key in keys {
        let expected_value = expected_map.remove(key);
        let actual_value = actual_map.remove(key);
        if expected_value != actual_value {
            drifts.push(BaselineTrainingNamedCountDrift {
                scope: scope.to_string(),
                key: key.to_string(),
                expected: expected_value,
                actual: actual_value,
            });
        }
    }
    drifts
}

fn build_named_bool_drifts(
    scope: &str,
    expected: Vec<(&str, bool)>,
    actual: Vec<(&str, bool)>,
) -> Vec<BaselineTrainingNamedBoolDrift> {
    let mut expected_map = expected.into_iter().collect::<BTreeMap<_, _>>();
    let mut actual_map = actual.into_iter().collect::<BTreeMap<_, _>>();
    let mut keys = expected_map
        .keys()
        .chain(actual_map.keys())
        .copied()
        .collect::<Vec<_>>();
    keys.sort_unstable();
    keys.dedup();

    let mut drifts = Vec::new();
    for key in keys {
        let expected_value = expected_map.remove(key);
        let actual_value = actual_map.remove(key);
        if expected_value != actual_value {
            drifts.push(BaselineTrainingNamedBoolDrift {
                scope: scope.to_string(),
                key: key.to_string(),
                expected: expected_value,
                actual: actual_value,
            });
        }
    }
    drifts
}

fn build_source_allocation_drifts(
    expected: &[BaselineTrainingSourceAllocation],
    actual: &[BaselineTrainingSourceAllocation],
) -> Vec<BaselineTrainingSourceAllocationDrift> {
    let mut expected_map = expected
        .iter()
        .map(|entry| {
            (
                entry.source_id.as_str(),
                entry.train_sequence_count + entry.validation_sequence_count,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut actual_map = actual
        .iter()
        .map(|entry| {
            (
                entry.source_id.as_str(),
                entry.train_sequence_count + entry.validation_sequence_count,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut keys = expected_map
        .keys()
        .chain(actual_map.keys())
        .copied()
        .collect::<Vec<_>>();
    keys.sort_unstable();
    keys.dedup();

    let mut drifts = Vec::new();
    for key in keys {
        let expected_value = expected_map.remove(key);
        let actual_value = actual_map.remove(key);
        if expected_value != actual_value {
            drifts.push(BaselineTrainingSourceAllocationDrift {
                source_id: key.to_string(),
                expected_total_sequence_count: expected_value,
                actual_total_sequence_count: actual_value,
            });
        }
    }
    drifts
}

fn assembly_categories(allocation: &BaselineTrainingSourceAllocation) -> Vec<String> {
    vec![
        format!("domain_{}", source_domain_slug(&allocation.domain)),
        format!("source_type_{}", source_type_slug(&allocation.source_type)),
    ]
}

fn assembly_guards(
    allocation: &BaselineTrainingSourceAllocation,
    accepted_source_count: usize,
) -> Vec<String> {
    let mut guards = Vec::new();

    if accepted_source_count == 1 {
        guards.push("single_source_concentration".to_string());
    } else {
        guards.push("multi_source_distribution".to_string());
    }

    if allocation.train_sequence_count > 0 {
        guards.push("train_coverage".to_string());
    }
    if allocation.validation_sequence_count > 0 {
        guards.push("validation_coverage".to_string());
    }
    if allocation.train_sequence_count > 0 && allocation.validation_sequence_count > 0 {
        guards.push("full_split_coverage".to_string());
    }

    guards.push(format!(
        "{}_domain_allocation",
        source_domain_slug(&allocation.domain)
    ));
    guards.push(format!(
        "{}_source_type_allocation",
        source_type_slug(&allocation.source_type)
    ));

    guards
}

fn source_domain_slug(domain: &SourceDomain) -> &'static str {
    match domain {
        SourceDomain::General => "general",
        SourceDomain::Reference => "reference",
        SourceDomain::Administrative => "administrative",
        SourceDomain::Education => "education",
    }
}

fn source_type_slug(source_type: &SourceType) -> &'static str {
    match source_type {
        SourceType::PublicText => "public_text",
        SourceType::ReferenceText => "reference_text",
        SourceType::AdministrativeText => "administrative_text",
        SourceType::EducationalText => "educational_text",
    }
}

#[cfg(test)]
mod tests {
    use adam_corpus::{
        CorpusManifest, CorpusStage, LicenseClass, QualityTier, SourceAcceptanceRecord,
        SourceAcceptanceReport, SourceDomain, SourcePolicy, SourceRegistry, SourceRegistryEntry,
        SourceScoringRules, SourceType, build_source_acceptance_report,
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
            version: "0.0.49".to_string(),
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
            version: "0.0.49".to_string(),
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
            version: "0.0.49".to_string(),
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
            version: "0.0.49".to_string(),
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
            version: "0.0.49".to_string(),
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
            version: "0.0.49".to_string(),
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
            version: "0.0.49".to_string(),
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
            version: "0.0.49".to_string(),
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
            version: "0.0.49".to_string(),
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
            version: "0.0.49".to_string(),
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
            version: "0.0.49".to_string(),
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
            version: "0.0.49".to_string(),
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
            version: "0.0.49".to_string(),
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
                .category_breakdown
                .iter()
                .any(|entry| entry.category == "source_type_reference_text")
        );
        assert!(
            report
                .critical_breakdown
                .iter()
                .any(|entry| entry.guard == "single_source_concentration")
        );
        assert!(
            report
                .critical_breakdown
                .iter()
                .any(|entry| entry.guard == "validation_coverage")
        );
        assert_eq!(report.source_allocations.len(), 1);
        assert_eq!(
            report.source_allocations[0].source_id,
            "curated_reference_kazakh"
        );
        assert_eq!(report.source_allocations[0].validation_sequence_count, 102);
    }

    #[test]
    fn builds_multi_source_training_assembly_distribution() {
        let manifest = BaselineTrainingManifest {
            version: "0.0.49".to_string(),
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
            objective: "assemble deterministic multi-source baseline training split".to_string(),
            max_steps: 128,
            batch_token_budget: 8192,
            context_window: 1024,
            validation_split_bps: 1000,
        };
        let corpus = CorpusManifest {
            version: "0.0.49".to_string(),
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
            version: "0.0.49".to_string(),
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
        let rules = SourceScoringRules {
            version: "0.0.49".to_string(),
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
        let report = build_source_acceptance_report(
            "adam-source-acceptance-report",
            "data/raw/source_registry.json",
            "data/raw/source_scoring_rules.json",
            &registry,
            &rules,
        )
        .expect("source acceptance report");
        let experiment = TokenizerExperiment {
            version: "0.0.49".to_string(),
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
        .expect("multi-source baseline training assembly");

        assert_eq!(report.accepted_source_count, 3);
        assert_eq!(report.rejected_source_count, 0);
        assert_eq!(report.total_sequence_count, 1024);
        assert_eq!(
            report.train_sequence_count + report.validation_sequence_count,
            1024
        );
        assert_eq!(report.source_allocations.len(), 3);
        assert!(
            report
                .critical_breakdown
                .iter()
                .any(|entry| entry.guard == "multi_source_distribution")
        );
        assert!(
            !report
                .critical_breakdown
                .iter()
                .any(|entry| entry.guard == "single_source_concentration")
        );
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
                .category_breakdown
                .iter()
                .any(|entry| entry.category == "domain_education")
        );

        let allocations_by_id = report
            .source_allocations
            .iter()
            .map(|entry| (entry.source_id.as_str(), entry))
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(
            allocations_by_id["curated_reference_kazakh"].train_sequence_count
                + allocations_by_id["curated_reference_kazakh"].validation_sequence_count,
            455
        );
        assert_eq!(
            allocations_by_id["curated_general_kazakh"].train_sequence_count
                + allocations_by_id["curated_general_kazakh"].validation_sequence_count,
            285
        );
        assert_eq!(
            allocations_by_id["reviewed_education_kazakh"].train_sequence_count
                + allocations_by_id["reviewed_education_kazakh"].validation_sequence_count,
            284
        );
    }
}
