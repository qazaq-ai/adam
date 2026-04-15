pub mod data;
pub mod model;

use std::collections::BTreeMap;

use adam_corpus::{
    CorpusManifest, SourceAcceptanceDeltaReport, SourceAcceptanceRecord, SourceAcceptanceReport,
    SourceAcceptanceSummaryReport, SourceDomain, SourceRegistry, SourceRegistryEntry,
    SourceScoringRules, SourceType, build_source_acceptance_report,
};
use adam_eval::{EvalBenchmarkDeltaReport, EvalBenchmarkReport, EvalSuite};
use adam_tokenizer::{
    TokenizerExperiment, TokenizerExperimentDeltaReport, TokenizerExperimentReport, normalize_text,
};
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingSample {
    pub id: String,
    pub source_id: String,
    pub domain: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingPack {
    pub version: String,
    pub name: String,
    pub target_language: String,
    pub script: String,
    pub samples: Vec<TinyCleanTrainingSample>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingDomainManifestEntry {
    pub domain: String,
    pub source_id: String,
    pub pack_manifest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingManifest {
    pub version: String,
    pub name: String,
    pub target_language: String,
    pub script: String,
    pub domain_packs: Vec<TinyCleanTrainingDomainManifestEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingDomainPack {
    pub version: String,
    pub name: String,
    pub target_language: String,
    pub script: String,
    pub domain: String,
    pub source_id: String,
    pub samples: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingSelectionManifest {
    pub version: String,
    pub name: String,
    pub target_language: String,
    pub script: String,
    pub source_clean_corpus_manifest: String,
    pub source_clean_corpus_pack: String,
    pub max_samples_per_domain: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MiniCleanTrainingManifest {
    pub version: String,
    pub name: String,
    pub target_language: String,
    pub script: String,
    pub source_clean_corpus_manifest: String,
    pub source_clean_corpus_pack: String,
    pub domain_sample_limits: BTreeMap<String, usize>,
    #[serde(default)]
    pub preferred_sample_texts_by_domain: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingProfileManifest {
    pub profile: String,
    pub pack_name: String,
    pub domain_sample_limits: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingProfileSuiteManifest {
    pub version: String,
    pub name: String,
    pub target_language: String,
    pub script: String,
    pub source_clean_corpus_manifest: String,
    pub source_clean_corpus_pack: String,
    pub profiles: Vec<TinyCleanTrainingProfileManifest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingProfileSummary {
    pub profile: String,
    pub pack_name: String,
    pub sample_count: usize,
    pub train_token_count: usize,
    pub validation_exact_match_rate_bps: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingProfileSuiteReport {
    pub suite_name: String,
    pub profile_count: usize,
    pub clean_corpus_sample_count: usize,
    pub profile_summaries: Vec<TinyCleanTrainingProfileSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingProfileGapReport {
    pub profile: String,
    pub pack_name: String,
    pub validation_exact_match_rate_bps: usize,
    pub validation_gap_bps: usize,
    pub sample_count: usize,
    pub sample_count_gap: usize,
    pub train_token_count: usize,
    pub train_token_gap: usize,
    pub is_best_profile: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingProfileComparisonReport {
    pub suite_name: String,
    pub profile_count: usize,
    pub best_profile: String,
    pub best_pack_name: String,
    pub best_validation_exact_match_rate_bps: usize,
    pub worst_profile: String,
    pub worst_pack_name: String,
    pub worst_validation_exact_match_rate_bps: usize,
    pub profile_gaps: Vec<TinyCleanTrainingProfileGapReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingProfileBaselineManifest {
    pub version: String,
    pub name: String,
    pub target_language: String,
    pub script: String,
    pub expected_best_profile: String,
    pub minimum_validation_exact_match_rate_bps: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingProfileBaselineCheck {
    pub check: String,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingProfileBaselineReport {
    pub policy_name: String,
    pub suite_name: String,
    pub expected_best_profile: String,
    pub actual_best_profile: String,
    pub selected_pack_name: String,
    pub selected_validation_exact_match_rate_bps: usize,
    pub matches_expected_best_profile: bool,
    pub meets_minimum_validation_threshold: bool,
    pub policy_checks: Vec<TinyCleanTrainingProfileBaselineCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingProfileBaselineDeltaReport {
    pub policy_name: String,
    pub matches_expected: bool,
    pub field_drifts: Vec<BaselineTrainingFieldDrift>,
    pub check_drifts: Vec<BaselineTrainingNamedBoolDrift>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingProfileStrategyManifest {
    pub version: String,
    pub name: String,
    pub target_language: String,
    pub script: String,
    pub minimum_candidate_validation_exact_match_rate_bps: usize,
    pub maximum_candidate_validation_gap_bps: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingProfileStrategyCandidateReport {
    pub profile: String,
    pub pack_name: String,
    pub validation_exact_match_rate_bps: usize,
    pub validation_gap_bps: usize,
    pub meets_minimum_validation_threshold: bool,
    pub within_gap_budget: bool,
    pub promotable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingProfileStrategyReport {
    pub strategy_name: String,
    pub suite_name: String,
    pub baseline_profile: String,
    pub baseline_validation_exact_match_rate_bps: usize,
    pub promotable_profile_count: usize,
    pub promotable_profiles: Vec<String>,
    pub candidate_reports: Vec<TinyCleanTrainingProfileStrategyCandidateReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingProfileStrategyDeltaReport {
    pub strategy_name: String,
    pub matches_expected: bool,
    pub field_drifts: Vec<BaselineTrainingFieldDrift>,
    pub promotable_profile_drifts: Vec<BaselineTrainingNamedBoolDrift>,
    pub candidate_flag_drifts: Vec<BaselineTrainingNamedBoolDrift>,
    pub candidate_metric_drifts: Vec<BaselineTrainingNamedCountDrift>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingProfilePromotionManifest {
    pub version: String,
    pub name: String,
    pub target_language: String,
    pub script: String,
    pub expected_active_profile: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingProfilePromotionCheck {
    pub check: String,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingProfilePromotionReport {
    pub promotion_name: String,
    pub policy_name: String,
    pub suite_name: String,
    pub expected_active_profile: String,
    pub active_profile: String,
    pub active_pack_name: String,
    pub promotable_profile_count: usize,
    pub promotable_profiles: Vec<String>,
    pub matches_expected_active_profile: bool,
    pub promotion_checks: Vec<TinyCleanTrainingProfilePromotionCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingProfilePromotionDeltaReport {
    pub promotion_name: String,
    pub matches_expected: bool,
    pub field_drifts: Vec<BaselineTrainingFieldDrift>,
    pub check_drifts: Vec<BaselineTrainingNamedBoolDrift>,
    pub promotable_profile_drifts: Vec<BaselineTrainingNamedBoolDrift>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingProfileExperimentMatrixManifest {
    pub version: String,
    pub name: String,
    pub target_language: String,
    pub script: String,
    pub expected_active_profile: String,
    pub require_promotable_profiles: bool,
    pub candidate_profiles: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingProfileExperimentEntryReport {
    pub profile: String,
    pub pack_name: String,
    pub is_active_profile: bool,
    pub is_promotable_profile: bool,
    pub sample_count: usize,
    pub train_token_count: usize,
    pub vocabulary_size: usize,
    pub deterministic_context_count: usize,
    pub ambiguous_context_count: usize,
    pub validation_exact_match_rate_bps: usize,
    pub validation_gap_bps: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingProfileExperimentMatrixCheck {
    pub check: String,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingProfileExperimentMatrixReport {
    pub matrix_name: String,
    pub suite_name: String,
    pub strategy_name: String,
    pub promotion_name: String,
    pub expected_active_profile: String,
    pub active_profile: String,
    pub candidate_count: usize,
    pub best_profile: String,
    pub best_validation_exact_match_rate_bps: usize,
    pub matrix_checks: Vec<TinyCleanTrainingProfileExperimentMatrixCheck>,
    pub entry_reports: Vec<TinyCleanTrainingProfileExperimentEntryReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingProfileExperimentMatrixDeltaReport {
    pub matrix_name: String,
    pub matches_expected: bool,
    pub field_drifts: Vec<BaselineTrainingFieldDrift>,
    pub check_drifts: Vec<BaselineTrainingNamedBoolDrift>,
    pub entry_flag_drifts: Vec<BaselineTrainingNamedBoolDrift>,
    pub entry_metric_drifts: Vec<BaselineTrainingNamedCountDrift>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingProfileExperimentMatrixPolicyManifest {
    pub version: String,
    pub name: String,
    pub target_language: String,
    pub script: String,
    pub expected_selected_profile: String,
    pub minimum_validation_exact_match_rate_bps: usize,
    pub maximum_validation_gap_bps: usize,
    pub require_selected_profile_is_best: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingProfileExperimentMatrixPolicyCheck {
    pub check: String,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingProfileExperimentMatrixPolicyCandidateDecision {
    pub profile: String,
    pub pack_name: String,
    pub is_promotable_profile: bool,
    pub meets_minimum_validation_threshold: bool,
    pub within_gap_budget: bool,
    pub is_eligible: bool,
    pub rejection_reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingProfileExperimentMatrixPolicyReport {
    pub policy_name: String,
    pub matrix_name: String,
    pub suite_name: String,
    pub expected_selected_profile: String,
    pub selected_profile: String,
    pub selected_pack_name: String,
    pub selected_validation_exact_match_rate_bps: usize,
    pub selected_validation_gap_bps: usize,
    pub best_profile: String,
    pub best_validation_exact_match_rate_bps: usize,
    pub candidate_count: usize,
    pub eligible_profile_count: usize,
    pub eligible_profiles: Vec<String>,
    pub matches_expected_selected_profile: bool,
    pub selected_profile_is_best: bool,
    pub selected_profile_is_promotable: bool,
    pub meets_minimum_validation_threshold: bool,
    pub within_gap_budget: bool,
    pub policy_checks: Vec<TinyCleanTrainingProfileExperimentMatrixPolicyCheck>,
    pub candidate_decisions: Vec<TinyCleanTrainingProfileExperimentMatrixPolicyCandidateDecision>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingProfileExperimentMatrixPolicyDeltaReport {
    pub policy_name: String,
    pub matches_expected: bool,
    pub field_drifts: Vec<BaselineTrainingFieldDrift>,
    pub check_drifts: Vec<BaselineTrainingNamedBoolDrift>,
    pub eligible_profile_drifts: Vec<BaselineTrainingNamedBoolDrift>,
    pub candidate_flag_drifts: Vec<BaselineTrainingNamedBoolDrift>,
    pub candidate_reason_drifts: Vec<BaselineTrainingNamedBoolDrift>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingCategoryReport {
    pub category: String,
    pub sample_count: usize,
    pub token_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingGuardReport {
    pub guard: String,
    pub sample_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingReport {
    pub run_name: String,
    pub pack_name: String,
    pub sample_ordering_strategy: String,
    pub accepted_source_count: usize,
    pub sample_count: usize,
    pub train_sample_count: usize,
    pub validation_sample_count: usize,
    pub validation_domain_count: usize,
    pub train_token_count: usize,
    pub validation_token_count: usize,
    pub vocabulary_size: usize,
    pub unique_context_count: usize,
    pub unique_transition_count: usize,
    pub deterministic_context_count: usize,
    pub ambiguous_context_count: usize,
    pub validation_next_token_count: usize,
    pub validation_exact_match_count: usize,
    pub validation_exact_match_rate_bps: usize,
    pub category_breakdown: Vec<TinyCleanTrainingCategoryReport>,
    pub critical_breakdown: Vec<TinyCleanTrainingGuardReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingMissAuditCategoryReport {
    pub category: String,
    pub miss_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingMissAuditGuardReport {
    pub guard: String,
    pub miss_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingMissAuditEntry {
    pub sample_id: String,
    pub source_id: String,
    pub domain: String,
    pub context: String,
    pub actual_next_token: String,
    pub predicted_next_token: Option<String>,
    pub predicted_transition_count: usize,
    pub candidate_count: usize,
    pub unseen_context: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingMissAuditReport {
    pub run_name: String,
    pub pack_name: String,
    pub validation_next_token_count: usize,
    pub validation_exact_match_count: usize,
    pub validation_miss_count: usize,
    pub validation_miss_rate_bps: usize,
    pub unseen_context_miss_count: usize,
    pub ambiguous_context_miss_count: usize,
    pub category_breakdown: Vec<TinyCleanTrainingMissAuditCategoryReport>,
    pub critical_breakdown: Vec<TinyCleanTrainingMissAuditGuardReport>,
    pub misses: Vec<TinyCleanTrainingMissAuditEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TinyCleanTrainingMissAuditDeltaReport {
    pub run_name: String,
    pub matches_expected: bool,
    pub field_drifts: Vec<BaselineTrainingFieldDrift>,
    pub miss_drifts: Vec<BaselineTrainingNamedBoolDrift>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanTrainingCorpusManifest {
    pub version: String,
    pub name: String,
    pub target_language: String,
    pub script: String,
    pub pack_manifests: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanTrainingCorpusSample {
    pub id: String,
    pub pack_name: String,
    pub source_id: String,
    pub domain: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanTrainingCorpusPack {
    pub version: String,
    pub name: String,
    pub target_language: String,
    pub script: String,
    pub samples: Vec<CleanTrainingCorpusSample>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanTrainingCorpusCategoryReport {
    pub category: String,
    pub sample_count: usize,
    pub token_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanTrainingCorpusGuardReport {
    pub guard: String,
    pub sample_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanTrainingCorpusReport {
    pub corpus_name: String,
    pub pack_count: usize,
    pub sample_count: usize,
    pub total_token_count: usize,
    pub category_breakdown: Vec<CleanTrainingCorpusCategoryReport>,
    pub critical_breakdown: Vec<CleanTrainingCorpusGuardReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundationOverviewLayerReport {
    pub layer: String,
    pub ready: bool,
    pub primary_metric_name: String,
    pub primary_metric_value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundationOverviewCheck {
    pub check: String,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundationOverviewReport {
    pub foundation_name: String,
    pub all_layers_match_expected: bool,
    pub accepted_source_count: usize,
    pub tokenizer_exact_match_rate_bps: usize,
    pub eval_task_count: usize,
    pub training_total_sequence_count: u64,
    pub tiny_training_vocabulary_size: usize,
    pub tiny_training_validation_exact_match_rate_bps: usize,
    pub tiny_training_validation_miss_count: usize,
    pub tiny_training_miss_audit_matches_expected: bool,
    pub tiny_profile_policy_matches_expected: bool,
    pub tiny_profile_strategy_matches_expected: bool,
    pub tiny_profile_experiment_matrix_policy_matches_expected: bool,
    pub tiny_profile_promotion_matches_expected: bool,
    pub tiny_profile_experiment_matrix_matches_expected: bool,
    pub layer_breakdown: Vec<FoundationOverviewLayerReport>,
    pub critical_breakdown: Vec<FoundationOverviewCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundationOverviewDeltaReport {
    pub foundation_name: String,
    pub matches_expected: bool,
    pub field_drifts: Vec<BaselineTrainingFieldDrift>,
    pub layer_drifts: Vec<BaselineTrainingNamedBoolDrift>,
    pub check_drifts: Vec<BaselineTrainingNamedBoolDrift>,
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
    #[error("tiny clean training pack must not be empty")]
    EmptyTinyTrainingPack,
    #[error("tiny clean training pack references non-accepted or inconsistent sources")]
    TinyTrainingSourceMismatch,
    #[error("tiny clean training manifest references are invalid or inconsistent")]
    TinyTrainingManifestMismatch,
    #[error("tiny clean training selection manifest is invalid or inconsistent")]
    TinyTrainingSelectionMismatch,
    #[error("mini clean training manifest is invalid or inconsistent")]
    MiniTrainingManifestMismatch,
    #[error("tiny clean training profile suite manifest is invalid or inconsistent")]
    TinyTrainingProfileSuiteMismatch,
    #[error("clean training corpus manifest references are invalid or inconsistent")]
    CleanTrainingCorpusManifestMismatch,
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

impl TinyCleanTrainingPack {
    pub fn validate(&self) -> Result<(), TrainingError> {
        if self.target_language != "kazakh" {
            return Err(TrainingError::NonKazakhLanguage);
        }

        if self.script != "cyrillic" {
            return Err(TrainingError::NonCyrillicScript);
        }

        if self.samples.is_empty() {
            return Err(TrainingError::EmptyTinyTrainingPack);
        }

        for sample in &self.samples {
            if sample.id.trim().is_empty()
                || sample.source_id.trim().is_empty()
                || sample.domain.trim().is_empty()
                || sample.text.trim().is_empty()
                || contains_latin(&sample.text)
            {
                return Err(TrainingError::TinyTrainingSourceMismatch);
            }
        }

        Ok(())
    }
}

impl TinyCleanTrainingManifest {
    pub fn validate(&self) -> Result<(), TrainingError> {
        if self.target_language != "kazakh" {
            return Err(TrainingError::NonKazakhLanguage);
        }

        if self.script != "cyrillic" {
            return Err(TrainingError::NonCyrillicScript);
        }

        if self.domain_packs.is_empty() {
            return Err(TrainingError::EmptyTinyTrainingPack);
        }

        let mut seen_domains = std::collections::BTreeSet::new();
        let mut seen_sources = std::collections::BTreeSet::new();
        let mut seen_paths = std::collections::BTreeSet::new();
        for entry in &self.domain_packs {
            if entry.domain.trim().is_empty()
                || entry.source_id.trim().is_empty()
                || entry.pack_manifest.trim().is_empty()
                || !seen_domains.insert(entry.domain.as_str())
                || !seen_sources.insert(entry.source_id.as_str())
                || !seen_paths.insert(entry.pack_manifest.as_str())
            {
                return Err(TrainingError::TinyTrainingManifestMismatch);
            }
        }

        Ok(())
    }
}

impl TinyCleanTrainingDomainPack {
    pub fn validate(&self) -> Result<(), TrainingError> {
        if self.target_language != "kazakh" {
            return Err(TrainingError::NonKazakhLanguage);
        }

        if self.script != "cyrillic" {
            return Err(TrainingError::NonCyrillicScript);
        }

        if self.domain.trim().is_empty()
            || self.source_id.trim().is_empty()
            || self.samples.is_empty()
        {
            return Err(TrainingError::TinyTrainingManifestMismatch);
        }

        for sample in &self.samples {
            if sample.trim().is_empty() || contains_latin(sample) {
                return Err(TrainingError::TinyTrainingManifestMismatch);
            }
        }

        Ok(())
    }
}

impl TinyCleanTrainingSelectionManifest {
    pub fn validate(&self) -> Result<(), TrainingError> {
        if self.target_language != "kazakh" {
            return Err(TrainingError::NonKazakhLanguage);
        }

        if self.script != "cyrillic" {
            return Err(TrainingError::NonCyrillicScript);
        }

        if self.source_clean_corpus_manifest.trim().is_empty()
            || self.source_clean_corpus_pack.trim().is_empty()
            || self.max_samples_per_domain == 0
        {
            return Err(TrainingError::TinyTrainingSelectionMismatch);
        }

        Ok(())
    }
}

impl MiniCleanTrainingManifest {
    pub fn validate(&self) -> Result<(), TrainingError> {
        if self.target_language != "kazakh" {
            return Err(TrainingError::NonKazakhLanguage);
        }

        if self.script != "cyrillic" {
            return Err(TrainingError::NonCyrillicScript);
        }

        if self.source_clean_corpus_manifest.trim().is_empty()
            || self.source_clean_corpus_pack.trim().is_empty()
            || self.domain_sample_limits.is_empty()
        {
            return Err(TrainingError::MiniTrainingManifestMismatch);
        }

        for (domain, limit) in &self.domain_sample_limits {
            if domain.trim().is_empty() || *limit == 0 {
                return Err(TrainingError::MiniTrainingManifestMismatch);
            }
        }

        for (domain, sample_texts) in &self.preferred_sample_texts_by_domain {
            if domain.trim().is_empty()
                || !self.domain_sample_limits.contains_key(domain)
                || sample_texts.is_empty()
            {
                return Err(TrainingError::MiniTrainingManifestMismatch);
            }

            if sample_texts.iter().any(|sample| sample.trim().is_empty()) {
                return Err(TrainingError::MiniTrainingManifestMismatch);
            }

            let limit = self
                .domain_sample_limits
                .get(domain)
                .copied()
                .ok_or(TrainingError::MiniTrainingManifestMismatch)?;
            if sample_texts.len() > limit {
                return Err(TrainingError::MiniTrainingManifestMismatch);
            }
        }

        Ok(())
    }
}

impl TinyCleanTrainingProfileSuiteManifest {
    pub fn validate(&self) -> Result<(), TrainingError> {
        if self.target_language != "kazakh" {
            return Err(TrainingError::NonKazakhLanguage);
        }
        if self.script != "cyrillic" {
            return Err(TrainingError::NonCyrillicScript);
        }
        if self.source_clean_corpus_manifest.trim().is_empty()
            || self.source_clean_corpus_pack.trim().is_empty()
            || self.profiles.is_empty()
        {
            return Err(TrainingError::TinyTrainingProfileSuiteMismatch);
        }
        let mut seen_profiles = std::collections::BTreeSet::new();
        let mut seen_pack_names = std::collections::BTreeSet::new();
        for profile in &self.profiles {
            if profile.profile.trim().is_empty()
                || profile.pack_name.trim().is_empty()
                || profile.domain_sample_limits.is_empty()
                || !seen_profiles.insert(profile.profile.as_str())
                || !seen_pack_names.insert(profile.pack_name.as_str())
            {
                return Err(TrainingError::TinyTrainingProfileSuiteMismatch);
            }
            for (domain, limit) in &profile.domain_sample_limits {
                if domain.trim().is_empty() || *limit == 0 {
                    return Err(TrainingError::TinyTrainingProfileSuiteMismatch);
                }
            }
        }
        Ok(())
    }
}

impl TinyCleanTrainingProfileBaselineManifest {
    pub fn validate(&self) -> Result<(), TrainingError> {
        if self.target_language != "kazakh" {
            return Err(TrainingError::NonKazakhLanguage);
        }
        if self.script != "cyrillic" || self.expected_best_profile.trim().is_empty() {
            return Err(TrainingError::TinyTrainingProfileSuiteMismatch);
        }
        Ok(())
    }
}

impl TinyCleanTrainingProfileStrategyManifest {
    pub fn validate(&self) -> Result<(), TrainingError> {
        if self.target_language != "kazakh" {
            return Err(TrainingError::NonKazakhLanguage);
        }
        if self.script != "cyrillic" || self.name.trim().is_empty() {
            return Err(TrainingError::TinyTrainingProfileSuiteMismatch);
        }
        Ok(())
    }
}

impl TinyCleanTrainingProfilePromotionManifest {
    pub fn validate(&self) -> Result<(), TrainingError> {
        if self.target_language != "kazakh" {
            return Err(TrainingError::NonKazakhLanguage);
        }
        if self.script != "cyrillic" || self.expected_active_profile.trim().is_empty() {
            return Err(TrainingError::TinyTrainingProfileSuiteMismatch);
        }
        Ok(())
    }
}

impl TinyCleanTrainingProfileExperimentMatrixManifest {
    pub fn validate(&self) -> Result<(), TrainingError> {
        if self.target_language != "kazakh" {
            return Err(TrainingError::NonKazakhLanguage);
        }
        if self.script != "cyrillic"
            || self.name.trim().is_empty()
            || self.expected_active_profile.trim().is_empty()
            || self.candidate_profiles.is_empty()
        {
            return Err(TrainingError::TinyTrainingProfileSuiteMismatch);
        }

        let mut seen_profiles = std::collections::BTreeSet::new();
        for profile in &self.candidate_profiles {
            if profile.trim().is_empty() || !seen_profiles.insert(profile.as_str()) {
                return Err(TrainingError::TinyTrainingProfileSuiteMismatch);
            }
        }

        Ok(())
    }
}

impl TinyCleanTrainingProfileExperimentMatrixPolicyManifest {
    pub fn validate(&self) -> Result<(), TrainingError> {
        if self.target_language != "kazakh" {
            return Err(TrainingError::NonKazakhLanguage);
        }
        if self.script != "cyrillic"
            || self.name.trim().is_empty()
            || self.expected_selected_profile.trim().is_empty()
        {
            return Err(TrainingError::TinyTrainingProfileSuiteMismatch);
        }
        Ok(())
    }
}

impl CleanTrainingCorpusManifest {
    pub fn validate(&self) -> Result<(), TrainingError> {
        if self.target_language != "kazakh" {
            return Err(TrainingError::NonKazakhLanguage);
        }

        if self.script != "cyrillic" {
            return Err(TrainingError::NonCyrillicScript);
        }

        if self.pack_manifests.is_empty() {
            return Err(TrainingError::CleanTrainingCorpusManifestMismatch);
        }

        let mut seen_paths = std::collections::BTreeSet::new();
        for path in &self.pack_manifests {
            if path.trim().is_empty() || !seen_paths.insert(path.as_str()) {
                return Err(TrainingError::CleanTrainingCorpusManifestMismatch);
            }
        }

        Ok(())
    }
}

pub fn assemble_tiny_clean_training_pack(
    manifest: &TinyCleanTrainingManifest,
    domain_packs: &[TinyCleanTrainingDomainPack],
) -> Result<TinyCleanTrainingPack, TrainingError> {
    manifest.validate()?;

    if manifest.domain_packs.len() != domain_packs.len() {
        return Err(TrainingError::TinyTrainingManifestMismatch);
    }

    let domain_packs_by_domain = domain_packs
        .iter()
        .map(|pack| {
            pack.validate()?;
            Ok((pack.domain.as_str(), pack))
        })
        .collect::<Result<BTreeMap<_, _>, TrainingError>>()?;

    let mut samples = Vec::new();
    let mut next_index = 1usize;
    for entry in &manifest.domain_packs {
        let Some(pack) = domain_packs_by_domain.get(entry.domain.as_str()) else {
            return Err(TrainingError::TinyTrainingManifestMismatch);
        };
        if pack.source_id != entry.source_id
            || pack.domain != entry.domain
            || pack.target_language != manifest.target_language
            || pack.script != manifest.script
        {
            return Err(TrainingError::TinyTrainingManifestMismatch);
        }

        for text in &pack.samples {
            samples.push(TinyCleanTrainingSample {
                id: format!("clean_sample_{next_index:02}"),
                source_id: pack.source_id.clone(),
                domain: pack.domain.clone(),
                text: text.clone(),
            });
            next_index += 1;
        }
    }

    Ok(TinyCleanTrainingPack {
        version: manifest.version.clone(),
        name: manifest.name.clone(),
        target_language: manifest.target_language.clone(),
        script: manifest.script.clone(),
        samples,
    })
}

pub fn assemble_tiny_clean_training_pack_from_corpus(
    manifest: &TinyCleanTrainingSelectionManifest,
    clean_corpus_manifest: &CleanTrainingCorpusManifest,
    clean_corpus_pack: &CleanTrainingCorpusPack,
) -> Result<TinyCleanTrainingPack, TrainingError> {
    manifest.validate()?;
    clean_corpus_manifest.validate()?;

    if clean_corpus_manifest.name != clean_corpus_pack.name
        || clean_corpus_manifest.version != clean_corpus_pack.version
        || clean_corpus_manifest.target_language != clean_corpus_pack.target_language
        || clean_corpus_manifest.script != clean_corpus_pack.script
    {
        return Err(TrainingError::TinyTrainingSelectionMismatch);
    }

    if clean_corpus_pack.target_language != manifest.target_language
        || clean_corpus_pack.script != manifest.script
        || clean_corpus_pack.samples.is_empty()
    {
        return Err(TrainingError::TinyTrainingSelectionMismatch);
    }

    let domain_limits = clean_corpus_pack
        .samples
        .iter()
        .map(|sample| (sample.domain.clone(), manifest.max_samples_per_domain))
        .collect::<BTreeMap<_, _>>();

    assemble_tiny_clean_training_pack_with_domain_limits(
        &manifest.version,
        &manifest.name,
        &manifest.target_language,
        &manifest.script,
        clean_corpus_pack,
        &domain_limits,
    )
    .map_err(|_| TrainingError::TinyTrainingSelectionMismatch)
}

pub fn assemble_tiny_clean_training_pack_from_profile(
    suite: &TinyCleanTrainingProfileSuiteManifest,
    profile: &TinyCleanTrainingProfileManifest,
    clean_corpus_manifest: &CleanTrainingCorpusManifest,
    clean_corpus_pack: &CleanTrainingCorpusPack,
) -> Result<TinyCleanTrainingPack, TrainingError> {
    suite.validate()?;
    if !suite
        .profiles
        .iter()
        .any(|entry| entry.profile == profile.profile)
    {
        return Err(TrainingError::TinyTrainingProfileSuiteMismatch);
    }
    if clean_corpus_manifest.name != clean_corpus_pack.name
        || clean_corpus_manifest.version != clean_corpus_pack.version
        || clean_corpus_manifest.target_language != clean_corpus_pack.target_language
        || clean_corpus_manifest.script != clean_corpus_pack.script
        || clean_corpus_pack.target_language != suite.target_language
        || clean_corpus_pack.script != suite.script
    {
        return Err(TrainingError::TinyTrainingProfileSuiteMismatch);
    }

    assemble_tiny_clean_training_pack_with_domain_limits(
        &suite.version,
        &profile.pack_name,
        &suite.target_language,
        &suite.script,
        clean_corpus_pack,
        &profile.domain_sample_limits,
    )
    .map_err(|_| TrainingError::TinyTrainingProfileSuiteMismatch)
}

pub fn assemble_mini_clean_training_pack(
    manifest: &MiniCleanTrainingManifest,
    clean_corpus_manifest: &CleanTrainingCorpusManifest,
    clean_corpus_pack: &CleanTrainingCorpusPack,
) -> Result<TinyCleanTrainingPack, TrainingError> {
    manifest.validate()?;
    clean_corpus_manifest.validate()?;

    if clean_corpus_manifest.name != clean_corpus_pack.name
        || clean_corpus_manifest.version != clean_corpus_pack.version
        || clean_corpus_manifest.target_language != clean_corpus_pack.target_language
        || clean_corpus_manifest.script != clean_corpus_pack.script
    {
        return Err(TrainingError::MiniTrainingManifestMismatch);
    }

    if clean_corpus_pack.target_language != manifest.target_language
        || clean_corpus_pack.script != manifest.script
        || clean_corpus_pack.samples.is_empty()
    {
        return Err(TrainingError::MiniTrainingManifestMismatch);
    }

    if manifest.preferred_sample_texts_by_domain.is_empty() {
        assemble_tiny_clean_training_pack_with_domain_limits(
            &manifest.version,
            &manifest.name,
            &manifest.target_language,
            &manifest.script,
            clean_corpus_pack,
            &manifest.domain_sample_limits,
        )
        .map_err(|_| TrainingError::MiniTrainingManifestMismatch)
    } else {
        assemble_tiny_clean_training_pack_with_domain_limits_and_preferences(
            &manifest.version,
            &manifest.name,
            &manifest.target_language,
            &manifest.script,
            clean_corpus_pack,
            &manifest.domain_sample_limits,
            &manifest.preferred_sample_texts_by_domain,
        )
        .map_err(|_| TrainingError::MiniTrainingManifestMismatch)
    }
}

pub fn assemble_tiny_clean_training_pack_from_promotion(
    suite: &TinyCleanTrainingProfileSuiteManifest,
    promotion_report: &TinyCleanTrainingProfilePromotionReport,
    clean_corpus_manifest: &CleanTrainingCorpusManifest,
    clean_corpus_pack: &CleanTrainingCorpusPack,
) -> Result<TinyCleanTrainingPack, TrainingError> {
    suite.validate()?;
    if promotion_report.suite_name != suite.name
        || !promotion_report.matches_expected_active_profile
        || !promotion_report
            .promotable_profiles
            .iter()
            .any(|entry| entry == &promotion_report.active_profile)
    {
        return Err(TrainingError::TinyTrainingProfileSuiteMismatch);
    }

    let profile = suite
        .profiles
        .iter()
        .find(|entry| entry.profile == promotion_report.active_profile)
        .ok_or(TrainingError::TinyTrainingProfileSuiteMismatch)?;

    assemble_tiny_clean_training_pack_from_profile(
        suite,
        profile,
        clean_corpus_manifest,
        clean_corpus_pack,
    )
}

pub fn build_tiny_clean_training_profile_suite_report(
    training_manifest: &BaselineTrainingManifest,
    registry: &SourceRegistry,
    rules: &SourceScoringRules,
    acceptance_report: &SourceAcceptanceReport,
    suite: &TinyCleanTrainingProfileSuiteManifest,
    clean_corpus_manifest: &CleanTrainingCorpusManifest,
    clean_corpus_pack: &CleanTrainingCorpusPack,
) -> Result<TinyCleanTrainingProfileSuiteReport, TrainingError> {
    suite.validate()?;
    let mut profile_summaries = Vec::new();
    for profile in &suite.profiles {
        let pack = assemble_tiny_clean_training_pack_from_profile(
            suite,
            profile,
            clean_corpus_manifest,
            clean_corpus_pack,
        )?;
        let report = build_tiny_clean_training_report(
            training_manifest,
            registry,
            rules,
            acceptance_report,
            &pack,
        )?;
        profile_summaries.push(TinyCleanTrainingProfileSummary {
            profile: profile.profile.clone(),
            pack_name: profile.pack_name.clone(),
            sample_count: report.sample_count,
            train_token_count: report.train_token_count,
            validation_exact_match_rate_bps: report.validation_exact_match_rate_bps,
        });
    }
    profile_summaries.sort_by(|left, right| left.profile.cmp(&right.profile));

    Ok(TinyCleanTrainingProfileSuiteReport {
        suite_name: suite.name.clone(),
        profile_count: suite.profiles.len(),
        clean_corpus_sample_count: clean_corpus_pack.samples.len(),
        profile_summaries,
    })
}

pub fn build_tiny_clean_training_profile_comparison_report(
    suite_report: &TinyCleanTrainingProfileSuiteReport,
) -> Result<TinyCleanTrainingProfileComparisonReport, TrainingError> {
    if suite_report.profile_summaries.is_empty() || suite_report.profile_count == 0 {
        return Err(TrainingError::TinyTrainingProfileSuiteMismatch);
    }

    let best = suite_report
        .profile_summaries
        .iter()
        .max_by(|left, right| compare_profile_summaries(left, right))
        .expect("non-empty profile summaries");
    let worst = suite_report
        .profile_summaries
        .iter()
        .min_by(|left, right| compare_profile_summaries(left, right))
        .expect("non-empty profile summaries");

    let mut profile_gaps = suite_report
        .profile_summaries
        .iter()
        .map(|summary| TinyCleanTrainingProfileGapReport {
            profile: summary.profile.clone(),
            pack_name: summary.pack_name.clone(),
            validation_exact_match_rate_bps: summary.validation_exact_match_rate_bps,
            validation_gap_bps: best
                .validation_exact_match_rate_bps
                .saturating_sub(summary.validation_exact_match_rate_bps),
            sample_count: summary.sample_count,
            sample_count_gap: best.sample_count.saturating_sub(summary.sample_count),
            train_token_count: summary.train_token_count,
            train_token_gap: best
                .train_token_count
                .saturating_sub(summary.train_token_count),
            is_best_profile: summary.profile == best.profile,
        })
        .collect::<Vec<_>>();
    profile_gaps.sort_by(|left, right| left.profile.cmp(&right.profile));

    Ok(TinyCleanTrainingProfileComparisonReport {
        suite_name: suite_report.suite_name.clone(),
        profile_count: suite_report.profile_count,
        best_profile: best.profile.clone(),
        best_pack_name: best.pack_name.clone(),
        best_validation_exact_match_rate_bps: best.validation_exact_match_rate_bps,
        worst_profile: worst.profile.clone(),
        worst_pack_name: worst.pack_name.clone(),
        worst_validation_exact_match_rate_bps: worst.validation_exact_match_rate_bps,
        profile_gaps,
    })
}

pub fn build_tiny_clean_training_profile_baseline_report(
    manifest: &TinyCleanTrainingProfileBaselineManifest,
    comparison_report: &TinyCleanTrainingProfileComparisonReport,
) -> Result<TinyCleanTrainingProfileBaselineReport, TrainingError> {
    manifest.validate()?;

    let selected_profile = comparison_report
        .profile_gaps
        .iter()
        .find(|entry| entry.profile == comparison_report.best_profile)
        .ok_or(TrainingError::TinyTrainingProfileSuiteMismatch)?;

    let matches_expected_best_profile =
        comparison_report.best_profile == manifest.expected_best_profile;
    let meets_minimum_validation_threshold = comparison_report.best_validation_exact_match_rate_bps
        >= manifest.minimum_validation_exact_match_rate_bps;

    Ok(TinyCleanTrainingProfileBaselineReport {
        policy_name: manifest.name.clone(),
        suite_name: comparison_report.suite_name.clone(),
        expected_best_profile: manifest.expected_best_profile.clone(),
        actual_best_profile: comparison_report.best_profile.clone(),
        selected_pack_name: selected_profile.pack_name.clone(),
        selected_validation_exact_match_rate_bps: comparison_report
            .best_validation_exact_match_rate_bps,
        matches_expected_best_profile,
        meets_minimum_validation_threshold,
        policy_checks: vec![
            TinyCleanTrainingProfileBaselineCheck {
                check: "expected_best_profile_matches".to_string(),
                passed: matches_expected_best_profile,
            },
            TinyCleanTrainingProfileBaselineCheck {
                check: "minimum_validation_threshold_met".to_string(),
                passed: meets_minimum_validation_threshold,
            },
            TinyCleanTrainingProfileBaselineCheck {
                check: "suite_has_profiles".to_string(),
                passed: comparison_report.profile_count > 0,
            },
        ],
    })
}

pub fn build_tiny_clean_training_profile_baseline_delta_report(
    expected: &TinyCleanTrainingProfileBaselineReport,
    actual: &TinyCleanTrainingProfileBaselineReport,
) -> TinyCleanTrainingProfileBaselineDeltaReport {
    TinyCleanTrainingProfileBaselineDeltaReport {
        policy_name: actual.policy_name.clone(),
        matches_expected: expected == actual,
        field_drifts: build_tiny_profile_baseline_field_drifts(expected, actual),
        check_drifts: build_named_bool_drifts(
            "policy_check",
            expected
                .policy_checks
                .iter()
                .map(|entry| (entry.check.clone(), entry.passed))
                .collect(),
            actual
                .policy_checks
                .iter()
                .map(|entry| (entry.check.clone(), entry.passed))
                .collect(),
        ),
    }
}

pub fn build_tiny_clean_training_profile_strategy_report(
    manifest: &TinyCleanTrainingProfileStrategyManifest,
    baseline_report: &TinyCleanTrainingProfileBaselineReport,
    comparison_report: &TinyCleanTrainingProfileComparisonReport,
) -> Result<TinyCleanTrainingProfileStrategyReport, TrainingError> {
    manifest.validate()?;

    let mut candidate_reports = comparison_report
        .profile_gaps
        .iter()
        .map(|entry| {
            let meets_minimum_validation_threshold = entry.validation_exact_match_rate_bps
                >= manifest.minimum_candidate_validation_exact_match_rate_bps;
            let within_gap_budget =
                entry.validation_gap_bps <= manifest.maximum_candidate_validation_gap_bps;
            TinyCleanTrainingProfileStrategyCandidateReport {
                profile: entry.profile.clone(),
                pack_name: entry.pack_name.clone(),
                validation_exact_match_rate_bps: entry.validation_exact_match_rate_bps,
                validation_gap_bps: entry.validation_gap_bps,
                meets_minimum_validation_threshold,
                within_gap_budget,
                promotable: meets_minimum_validation_threshold && within_gap_budget,
            }
        })
        .collect::<Vec<_>>();
    candidate_reports.sort_by(|left, right| left.profile.cmp(&right.profile));

    let promotable_profiles = candidate_reports
        .iter()
        .filter(|entry| entry.promotable)
        .map(|entry| entry.profile.clone())
        .collect::<Vec<_>>();

    Ok(TinyCleanTrainingProfileStrategyReport {
        strategy_name: manifest.name.clone(),
        suite_name: comparison_report.suite_name.clone(),
        baseline_profile: baseline_report.actual_best_profile.clone(),
        baseline_validation_exact_match_rate_bps: baseline_report
            .selected_validation_exact_match_rate_bps,
        promotable_profile_count: promotable_profiles.len(),
        promotable_profiles,
        candidate_reports,
    })
}

pub fn build_tiny_clean_training_profile_strategy_delta_report(
    expected: &TinyCleanTrainingProfileStrategyReport,
    actual: &TinyCleanTrainingProfileStrategyReport,
) -> TinyCleanTrainingProfileStrategyDeltaReport {
    TinyCleanTrainingProfileStrategyDeltaReport {
        strategy_name: actual.strategy_name.clone(),
        matches_expected: expected == actual,
        field_drifts: build_tiny_profile_strategy_field_drifts(expected, actual),
        promotable_profile_drifts: build_named_bool_drifts(
            "promotable_profile",
            build_promotable_profile_bools(expected),
            build_promotable_profile_bools(actual),
        ),
        candidate_flag_drifts: build_named_bool_drifts(
            "candidate_flag",
            build_strategy_candidate_flag_bools(expected),
            build_strategy_candidate_flag_bools(actual),
        ),
        candidate_metric_drifts: build_named_count_drifts(
            "candidate_metric",
            build_strategy_candidate_metrics(expected),
            build_strategy_candidate_metrics(actual),
        ),
    }
}

pub fn build_tiny_clean_training_profile_promotion_report(
    manifest: &TinyCleanTrainingProfilePromotionManifest,
    matrix_policy_report: &TinyCleanTrainingProfileExperimentMatrixPolicyReport,
) -> Result<TinyCleanTrainingProfilePromotionReport, TrainingError> {
    manifest.validate()?;

    if !matrix_policy_report.matches_expected_selected_profile
        || !matrix_policy_report
            .policy_checks
            .iter()
            .all(|entry| entry.passed)
    {
        return Err(TrainingError::TinyTrainingProfileSuiteMismatch);
    }

    let matches_expected_active_profile =
        matrix_policy_report.selected_profile == manifest.expected_active_profile;

    Ok(TinyCleanTrainingProfilePromotionReport {
        promotion_name: manifest.name.clone(),
        policy_name: matrix_policy_report.policy_name.clone(),
        suite_name: matrix_policy_report.suite_name.clone(),
        expected_active_profile: manifest.expected_active_profile.clone(),
        active_profile: matrix_policy_report.selected_profile.clone(),
        active_pack_name: matrix_policy_report.selected_pack_name.clone(),
        promotable_profile_count: matrix_policy_report.eligible_profile_count,
        promotable_profiles: matrix_policy_report.eligible_profiles.clone(),
        matches_expected_active_profile,
        promotion_checks: vec![
            TinyCleanTrainingProfilePromotionCheck {
                check: "expected_active_profile_matches".to_string(),
                passed: matches_expected_active_profile,
            },
            TinyCleanTrainingProfilePromotionCheck {
                check: "active_profile_is_promotable".to_string(),
                passed: matrix_policy_report
                    .eligible_profiles
                    .iter()
                    .any(|entry| entry == &matrix_policy_report.selected_profile),
            },
            TinyCleanTrainingProfilePromotionCheck {
                check: "promotable_profile_available".to_string(),
                passed: matrix_policy_report.eligible_profile_count > 0,
            },
        ],
    })
}

pub fn build_tiny_clean_training_profile_promotion_delta_report(
    expected: &TinyCleanTrainingProfilePromotionReport,
    actual: &TinyCleanTrainingProfilePromotionReport,
) -> TinyCleanTrainingProfilePromotionDeltaReport {
    TinyCleanTrainingProfilePromotionDeltaReport {
        promotion_name: actual.promotion_name.clone(),
        matches_expected: expected == actual,
        field_drifts: build_tiny_profile_promotion_field_drifts(expected, actual),
        check_drifts: build_named_bool_drifts(
            "promotion_check",
            expected
                .promotion_checks
                .iter()
                .map(|entry| (entry.check.clone(), entry.passed))
                .collect(),
            actual
                .promotion_checks
                .iter()
                .map(|entry| (entry.check.clone(), entry.passed))
                .collect(),
        ),
        promotable_profile_drifts: build_named_bool_drifts(
            "promotable_profile",
            expected
                .promotable_profiles
                .iter()
                .map(|entry| (entry.clone(), true))
                .collect(),
            actual
                .promotable_profiles
                .iter()
                .map(|entry| (entry.clone(), true))
                .collect(),
        ),
    }
}

pub fn build_tiny_clean_training_profile_experiment_matrix_report(
    manifest: &TinyCleanTrainingProfileExperimentMatrixManifest,
    training_manifest: &BaselineTrainingManifest,
    registry: &SourceRegistry,
    rules: &SourceScoringRules,
    acceptance_report: &SourceAcceptanceReport,
    suite: &TinyCleanTrainingProfileSuiteManifest,
    strategy_report: &TinyCleanTrainingProfileStrategyReport,
    clean_corpus_manifest: &CleanTrainingCorpusManifest,
    clean_corpus_pack: &CleanTrainingCorpusPack,
) -> Result<TinyCleanTrainingProfileExperimentMatrixReport, TrainingError> {
    manifest.validate()?;
    suite.validate()?;

    if suite.target_language != manifest.target_language
        || suite.script != manifest.script
        || strategy_report.suite_name != suite.name
    {
        return Err(TrainingError::TinyTrainingProfileSuiteMismatch);
    }

    let promotable_profiles = strategy_report
        .promotable_profiles
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let candidate_reports_by_profile = strategy_report
        .candidate_reports
        .iter()
        .map(|entry| (entry.profile.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let suite_profiles_by_profile = suite
        .profiles
        .iter()
        .map(|entry| (entry.profile.as_str(), entry))
        .collect::<BTreeMap<_, _>>();

    let mut entry_reports = Vec::new();
    for profile_name in &manifest.candidate_profiles {
        let Some(strategy_candidate) = candidate_reports_by_profile.get(profile_name.as_str())
        else {
            return Err(TrainingError::TinyTrainingProfileSuiteMismatch);
        };
        if manifest.require_promotable_profiles && !strategy_candidate.promotable {
            return Err(TrainingError::TinyTrainingProfileSuiteMismatch);
        }
        let Some(profile) = suite_profiles_by_profile.get(profile_name.as_str()) else {
            return Err(TrainingError::TinyTrainingProfileSuiteMismatch);
        };
        let pack = assemble_tiny_clean_training_pack_from_profile(
            suite,
            profile,
            clean_corpus_manifest,
            clean_corpus_pack,
        )?;
        let report = build_tiny_clean_training_report(
            training_manifest,
            registry,
            rules,
            acceptance_report,
            &pack,
        )?;
        entry_reports.push(TinyCleanTrainingProfileExperimentEntryReport {
            profile: profile.profile.clone(),
            pack_name: pack.name,
            is_active_profile: profile.profile == manifest.expected_active_profile,
            is_promotable_profile: promotable_profiles.contains(profile.profile.as_str()),
            sample_count: report.sample_count,
            train_token_count: report.train_token_count,
            vocabulary_size: report.vocabulary_size,
            deterministic_context_count: report.deterministic_context_count,
            ambiguous_context_count: report.ambiguous_context_count,
            validation_exact_match_rate_bps: report.validation_exact_match_rate_bps,
            validation_gap_bps: 0,
        });
    }

    let Some(best_entry) = entry_reports
        .iter()
        .max_by(|left, right| compare_profile_experiment_entries(left, right))
    else {
        return Err(TrainingError::TinyTrainingProfileSuiteMismatch);
    };
    let best_profile = best_entry.profile.clone();
    let best_validation_exact_match_rate_bps = best_entry.validation_exact_match_rate_bps;
    let active_validation_exact_match_rate_bps = entry_reports
        .iter()
        .find(|entry| entry.is_active_profile)
        .map(|entry| entry.validation_exact_match_rate_bps)
        .ok_or(TrainingError::TinyTrainingProfileSuiteMismatch)?;
    for entry in &mut entry_reports {
        entry.validation_gap_bps = active_validation_exact_match_rate_bps
            .saturating_sub(entry.validation_exact_match_rate_bps);
    }
    entry_reports.sort_by(|left, right| left.profile.cmp(&right.profile));

    Ok(TinyCleanTrainingProfileExperimentMatrixReport {
        matrix_name: manifest.name.clone(),
        suite_name: suite.name.clone(),
        strategy_name: strategy_report.strategy_name.clone(),
        promotion_name: "unselected".to_string(),
        expected_active_profile: manifest.expected_active_profile.clone(),
        active_profile: manifest.expected_active_profile.clone(),
        candidate_count: entry_reports.len(),
        best_profile,
        best_validation_exact_match_rate_bps,
        matrix_checks: vec![
            TinyCleanTrainingProfileExperimentMatrixCheck {
                check: "expected_active_profile_included".to_string(),
                passed: entry_reports.iter().any(|entry| entry.is_active_profile),
            },
            TinyCleanTrainingProfileExperimentMatrixCheck {
                check: "reference_profile_is_promotable".to_string(),
                passed: entry_reports
                    .iter()
                    .any(|entry| entry.is_active_profile && entry.is_promotable_profile),
            },
            TinyCleanTrainingProfileExperimentMatrixCheck {
                check: "candidate_profiles_are_promotable".to_string(),
                passed: !manifest.require_promotable_profiles
                    || entry_reports
                        .iter()
                        .all(|entry| entry.is_promotable_profile),
            },
            TinyCleanTrainingProfileExperimentMatrixCheck {
                check: "candidate_count_matches_manifest".to_string(),
                passed: entry_reports.len() == manifest.candidate_profiles.len(),
            },
        ],
        entry_reports,
    })
}

pub fn build_tiny_clean_training_profile_experiment_matrix_delta_report(
    expected: &TinyCleanTrainingProfileExperimentMatrixReport,
    actual: &TinyCleanTrainingProfileExperimentMatrixReport,
) -> TinyCleanTrainingProfileExperimentMatrixDeltaReport {
    TinyCleanTrainingProfileExperimentMatrixDeltaReport {
        matrix_name: actual.matrix_name.clone(),
        matches_expected: expected == actual,
        field_drifts: build_tiny_profile_experiment_matrix_field_drifts(expected, actual),
        check_drifts: build_named_bool_drifts(
            "matrix_check",
            expected
                .matrix_checks
                .iter()
                .map(|entry| (entry.check.clone(), entry.passed))
                .collect(),
            actual
                .matrix_checks
                .iter()
                .map(|entry| (entry.check.clone(), entry.passed))
                .collect(),
        ),
        entry_flag_drifts: build_named_bool_drifts(
            "matrix_entry_flag",
            build_experiment_matrix_entry_flags(expected),
            build_experiment_matrix_entry_flags(actual),
        ),
        entry_metric_drifts: build_named_count_drifts(
            "matrix_entry_metric",
            build_experiment_matrix_entry_metrics(expected),
            build_experiment_matrix_entry_metrics(actual),
        ),
    }
}

pub fn build_tiny_clean_training_profile_experiment_matrix_policy_report(
    manifest: &TinyCleanTrainingProfileExperimentMatrixPolicyManifest,
    matrix_report: &TinyCleanTrainingProfileExperimentMatrixReport,
) -> Result<TinyCleanTrainingProfileExperimentMatrixPolicyReport, TrainingError> {
    manifest.validate()?;

    let candidate_decisions = matrix_report
        .entry_reports
        .iter()
        .map(|entry| {
            let meets_minimum_validation_threshold = entry.validation_exact_match_rate_bps
                >= manifest.minimum_validation_exact_match_rate_bps;
            let within_gap_budget = entry.validation_gap_bps <= manifest.maximum_validation_gap_bps;
            let is_eligible = entry.is_promotable_profile
                && meets_minimum_validation_threshold
                && within_gap_budget;
            let mut rejection_reasons = Vec::new();
            if !entry.is_promotable_profile {
                rejection_reasons.push("not_promotable_profile".to_string());
            }
            if !meets_minimum_validation_threshold {
                rejection_reasons.push("below_minimum_validation_threshold".to_string());
            }
            if !within_gap_budget {
                rejection_reasons.push("exceeds_validation_gap_budget".to_string());
            }

            TinyCleanTrainingProfileExperimentMatrixPolicyCandidateDecision {
                profile: entry.profile.clone(),
                pack_name: entry.pack_name.clone(),
                is_promotable_profile: entry.is_promotable_profile,
                meets_minimum_validation_threshold,
                within_gap_budget,
                is_eligible,
                rejection_reasons,
            }
        })
        .collect::<Vec<_>>();
    let eligible_profiles = candidate_decisions
        .iter()
        .filter(|entry| entry.is_eligible)
        .map(|entry| entry.profile.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let eligible_entries = matrix_report
        .entry_reports
        .iter()
        .filter(|entry| eligible_profiles.contains(entry.profile.as_str()))
        .collect::<Vec<_>>();
    let selected_entry = eligible_entries
        .iter()
        .max_by(|left, right| compare_profile_experiment_entries(left, right))
        .copied()
        .ok_or(TrainingError::TinyTrainingProfileSuiteMismatch)?;
    let eligible_profiles = eligible_entries
        .iter()
        .map(|entry| entry.profile.clone())
        .collect::<Vec<_>>();
    let matches_expected_selected_profile =
        selected_entry.profile == manifest.expected_selected_profile;
    let selected_profile_is_best = selected_entry.profile == matrix_report.best_profile;
    let selected_profile_is_promotable = selected_entry.is_promotable_profile;
    let meets_minimum_validation_threshold = selected_entry.validation_exact_match_rate_bps
        >= manifest.minimum_validation_exact_match_rate_bps;
    let within_gap_budget =
        selected_entry.validation_gap_bps <= manifest.maximum_validation_gap_bps;

    Ok(TinyCleanTrainingProfileExperimentMatrixPolicyReport {
        policy_name: manifest.name.clone(),
        matrix_name: matrix_report.matrix_name.clone(),
        suite_name: matrix_report.suite_name.clone(),
        expected_selected_profile: manifest.expected_selected_profile.clone(),
        selected_profile: selected_entry.profile.clone(),
        selected_pack_name: selected_entry.pack_name.clone(),
        selected_validation_exact_match_rate_bps: selected_entry.validation_exact_match_rate_bps,
        selected_validation_gap_bps: selected_entry.validation_gap_bps,
        best_profile: matrix_report.best_profile.clone(),
        best_validation_exact_match_rate_bps: matrix_report.best_validation_exact_match_rate_bps,
        candidate_count: matrix_report.candidate_count,
        eligible_profile_count: eligible_profiles.len(),
        eligible_profiles,
        matches_expected_selected_profile,
        selected_profile_is_best,
        selected_profile_is_promotable,
        meets_minimum_validation_threshold,
        within_gap_budget,
        policy_checks: vec![
            TinyCleanTrainingProfileExperimentMatrixPolicyCheck {
                check: "expected_selected_profile_matches".to_string(),
                passed: matches_expected_selected_profile,
            },
            TinyCleanTrainingProfileExperimentMatrixPolicyCheck {
                check: "selected_profile_meets_minimum_validation_threshold".to_string(),
                passed: meets_minimum_validation_threshold,
            },
            TinyCleanTrainingProfileExperimentMatrixPolicyCheck {
                check: "selected_profile_within_gap_budget".to_string(),
                passed: within_gap_budget,
            },
            TinyCleanTrainingProfileExperimentMatrixPolicyCheck {
                check: "selected_profile_is_best".to_string(),
                passed: !manifest.require_selected_profile_is_best || selected_profile_is_best,
            },
            TinyCleanTrainingProfileExperimentMatrixPolicyCheck {
                check: "selected_profile_is_promotable".to_string(),
                passed: selected_profile_is_promotable,
            },
            TinyCleanTrainingProfileExperimentMatrixPolicyCheck {
                check: "eligible_profile_available".to_string(),
                passed: !eligible_entries.is_empty(),
            },
        ],
        candidate_decisions,
    })
}

pub fn build_tiny_clean_training_profile_experiment_matrix_policy_delta_report(
    expected: &TinyCleanTrainingProfileExperimentMatrixPolicyReport,
    actual: &TinyCleanTrainingProfileExperimentMatrixPolicyReport,
) -> TinyCleanTrainingProfileExperimentMatrixPolicyDeltaReport {
    TinyCleanTrainingProfileExperimentMatrixPolicyDeltaReport {
        policy_name: actual.policy_name.clone(),
        matches_expected: expected == actual,
        field_drifts: build_tiny_profile_experiment_matrix_policy_field_drifts(expected, actual),
        check_drifts: build_named_bool_drifts(
            "matrix_policy_check",
            expected
                .policy_checks
                .iter()
                .map(|entry| (entry.check.clone(), entry.passed))
                .collect(),
            actual
                .policy_checks
                .iter()
                .map(|entry| (entry.check.clone(), entry.passed))
                .collect(),
        ),
        eligible_profile_drifts: build_named_bool_drifts(
            "eligible_profile",
            expected
                .eligible_profiles
                .iter()
                .map(|entry| (entry.clone(), true))
                .collect(),
            actual
                .eligible_profiles
                .iter()
                .map(|entry| (entry.clone(), true))
                .collect(),
        ),
        candidate_flag_drifts: build_named_bool_drifts(
            "matrix_policy_candidate_flag",
            build_experiment_matrix_policy_candidate_flags(expected),
            build_experiment_matrix_policy_candidate_flags(actual),
        ),
        candidate_reason_drifts: build_named_bool_drifts(
            "matrix_policy_candidate_reason",
            build_experiment_matrix_policy_candidate_reasons(expected),
            build_experiment_matrix_policy_candidate_reasons(actual),
        ),
    }
}

pub fn assemble_clean_training_corpus_pack(
    manifest: &CleanTrainingCorpusManifest,
    packs: &[TinyCleanTrainingDomainPack],
) -> Result<CleanTrainingCorpusPack, TrainingError> {
    manifest.validate()?;

    if manifest.pack_manifests.len() != packs.len() {
        return Err(TrainingError::CleanTrainingCorpusManifestMismatch);
    }

    let mut seen_pack_names = std::collections::BTreeSet::new();
    let mut samples = Vec::new();
    let mut next_index = 1usize;
    for pack in packs {
        pack.validate()?;
        if pack.target_language != manifest.target_language
            || pack.script != manifest.script
            || !seen_pack_names.insert(pack.name.as_str())
        {
            return Err(TrainingError::CleanTrainingCorpusManifestMismatch);
        }

        for text in &pack.samples {
            samples.push(CleanTrainingCorpusSample {
                id: format!("clean_corpus_sample_{next_index:02}"),
                pack_name: pack.name.clone(),
                source_id: pack.source_id.clone(),
                domain: pack.domain.clone(),
                text: text.clone(),
            });
            next_index += 1;
        }
    }

    Ok(CleanTrainingCorpusPack {
        version: manifest.version.clone(),
        name: manifest.name.clone(),
        target_language: manifest.target_language.clone(),
        script: manifest.script.clone(),
        samples,
    })
}

pub fn build_clean_training_corpus_report(
    manifest: &CleanTrainingCorpusManifest,
    pack: &CleanTrainingCorpusPack,
) -> Result<CleanTrainingCorpusReport, TrainingError> {
    manifest.validate()?;

    if manifest.name != pack.name
        || manifest.version != pack.version
        || manifest.target_language != pack.target_language
        || manifest.script != pack.script
        || pack.samples.is_empty()
    {
        return Err(TrainingError::CleanTrainingCorpusManifestMismatch);
    }

    let mut category_stats = BTreeMap::<String, (usize, usize)>::new();
    let mut pack_counts = BTreeMap::<String, usize>::new();
    for sample in &pack.samples {
        if sample.id.trim().is_empty()
            || sample.pack_name.trim().is_empty()
            || sample.source_id.trim().is_empty()
            || sample.domain.trim().is_empty()
            || sample.text.trim().is_empty()
            || contains_latin(&sample.text)
        {
            return Err(TrainingError::CleanTrainingCorpusManifestMismatch);
        }

        let token_count = tokenize_clean_training_text(&sample.text).len();
        for category in [
            format!("domain_{}", sample.domain),
            format!("source_{}", sample.source_id),
            format!("pack_{}", sample.pack_name),
        ] {
            let stats = category_stats.entry(category).or_insert((0, 0));
            stats.0 += 1;
            stats.1 += token_count;
        }
        *pack_counts.entry(sample.pack_name.clone()).or_default() += 1;
    }

    let mut category_breakdown = category_stats
        .into_iter()
        .map(
            |(category, (sample_count, token_count))| CleanTrainingCorpusCategoryReport {
                category,
                sample_count,
                token_count,
            },
        )
        .collect::<Vec<_>>();
    category_breakdown.sort_by(|left, right| left.category.cmp(&right.category));

    let mut critical_breakdown = vec![
        CleanTrainingCorpusGuardReport {
            guard: "clean_pack_coverage".to_string(),
            sample_count: pack.samples.len(),
        },
        CleanTrainingCorpusGuardReport {
            guard: "multi_pack_coverage".to_string(),
            sample_count: usize::from(pack_counts.len() == manifest.pack_manifests.len())
                * pack.samples.len(),
        },
        CleanTrainingCorpusGuardReport {
            guard: "multi_domain_coverage".to_string(),
            sample_count: usize::from(
                pack.samples
                    .iter()
                    .map(|sample| sample.domain.as_str())
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    >= 3,
            ) * pack.samples.len(),
        },
    ];
    critical_breakdown.sort_by(|left, right| left.guard.cmp(&right.guard));

    Ok(CleanTrainingCorpusReport {
        corpus_name: pack.name.clone(),
        pack_count: pack_counts.len(),
        sample_count: pack.samples.len(),
        total_token_count: pack
            .samples
            .iter()
            .map(|sample| tokenize_clean_training_text(&sample.text).len())
            .sum(),
        category_breakdown,
        critical_breakdown,
    })
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
                        entry.category.clone(),
                        entry.train_sequence_count + entry.validation_sequence_count,
                    )
                })
                .collect(),
            actual_assembly
                .category_breakdown
                .iter()
                .map(|entry| {
                    (
                        entry.category.clone(),
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
                .map(|entry| (entry.guard.clone(), entry.total_sequence_count))
                .collect(),
            actual_assembly
                .critical_breakdown
                .iter()
                .map(|entry| (entry.guard.clone(), entry.total_sequence_count))
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
                .map(|entry| (entry.check.clone(), entry.passed))
                .collect(),
            actual_consistency
                .consistency_checks
                .iter()
                .map(|entry| (entry.check.clone(), entry.passed))
                .collect(),
        ),
    })
}

pub fn build_tiny_clean_training_report(
    manifest: &BaselineTrainingManifest,
    registry: &SourceRegistry,
    rules: &SourceScoringRules,
    report: &SourceAcceptanceReport,
    pack: &TinyCleanTrainingPack,
) -> Result<TinyCleanTrainingReport, TrainingError> {
    build_clean_training_report_with_run_name(
        &format!("{}-tiny-clean-prototype", manifest.run_name),
        manifest,
        registry,
        rules,
        report,
        pack,
    )
}

pub fn build_mini_clean_training_report(
    manifest: &BaselineTrainingManifest,
    registry: &SourceRegistry,
    rules: &SourceScoringRules,
    report: &SourceAcceptanceReport,
    pack: &TinyCleanTrainingPack,
) -> Result<TinyCleanTrainingReport, TrainingError> {
    build_clean_training_report_with_run_name(
        &format!("{}-mini-clean-prototype", manifest.run_name),
        manifest,
        registry,
        rules,
        report,
        pack,
    )
}

fn build_clean_training_report_with_run_name(
    run_name: &str,
    manifest: &BaselineTrainingManifest,
    registry: &SourceRegistry,
    rules: &SourceScoringRules,
    report: &SourceAcceptanceReport,
    pack: &TinyCleanTrainingPack,
) -> Result<TinyCleanTrainingReport, TrainingError> {
    let analysis = analyze_tiny_clean_training(manifest, registry, rules, report, pack)?;
    let accepted_source_count = report
        .records
        .iter()
        .filter(|record| record.accepted_for_training)
        .count();

    Ok(TinyCleanTrainingReport {
        run_name: run_name.to_string(),
        pack_name: pack.name.clone(),
        sample_ordering_strategy: "round_robin_by_domain_with_stratified_validation".to_string(),
        accepted_source_count,
        sample_count: analysis.ordered_samples.len(),
        train_sample_count: analysis.train_sample_count,
        validation_sample_count: analysis.validation_sample_count,
        validation_domain_count: analysis.validation_domain_count,
        train_token_count: analysis.train_token_count,
        validation_token_count: analysis.validation_token_count,
        vocabulary_size: analysis.vocabulary_size,
        unique_context_count: analysis.unique_context_count,
        unique_transition_count: analysis.unique_transition_count,
        deterministic_context_count: analysis.deterministic_context_count,
        ambiguous_context_count: analysis.ambiguous_context_count,
        validation_next_token_count: analysis.validation_next_token_count,
        validation_exact_match_count: analysis.validation_exact_match_count,
        validation_exact_match_rate_bps: analysis.validation_exact_match_rate_bps,
        category_breakdown: analysis.category_breakdown,
        critical_breakdown: analysis.critical_breakdown,
    })
}

pub fn build_tiny_clean_training_miss_audit_report(
    manifest: &BaselineTrainingManifest,
    registry: &SourceRegistry,
    rules: &SourceScoringRules,
    report: &SourceAcceptanceReport,
    pack: &TinyCleanTrainingPack,
) -> Result<TinyCleanTrainingMissAuditReport, TrainingError> {
    build_clean_training_miss_audit_report_with_run_name(
        &format!("{}-tiny-clean-miss-audit", manifest.run_name),
        manifest,
        registry,
        rules,
        report,
        pack,
    )
}

pub fn build_mini_clean_training_miss_audit_report(
    manifest: &BaselineTrainingManifest,
    registry: &SourceRegistry,
    rules: &SourceScoringRules,
    report: &SourceAcceptanceReport,
    pack: &TinyCleanTrainingPack,
) -> Result<TinyCleanTrainingMissAuditReport, TrainingError> {
    build_clean_training_miss_audit_report_with_run_name(
        &format!("{}-mini-clean-miss-audit", manifest.run_name),
        manifest,
        registry,
        rules,
        report,
        pack,
    )
}

fn build_clean_training_miss_audit_report_with_run_name(
    run_name: &str,
    manifest: &BaselineTrainingManifest,
    registry: &SourceRegistry,
    rules: &SourceScoringRules,
    report: &SourceAcceptanceReport,
    pack: &TinyCleanTrainingPack,
) -> Result<TinyCleanTrainingMissAuditReport, TrainingError> {
    let analysis = analyze_tiny_clean_training(manifest, registry, rules, report, pack)?;
    let validation_miss_count = analysis.validation_misses.len();
    let validation_miss_rate_bps = if analysis.validation_next_token_count == 0 {
        0
    } else {
        validation_miss_count * 10_000 / analysis.validation_next_token_count
    };
    let unseen_context_miss_count = analysis
        .validation_misses
        .iter()
        .filter(|entry| entry.unseen_context)
        .count();
    let ambiguous_context_miss_count = analysis
        .validation_misses
        .iter()
        .filter(|entry| !entry.unseen_context && entry.candidate_count > 1)
        .count();

    let mut category_counts = BTreeMap::<String, usize>::new();
    for miss in &analysis.validation_misses {
        for category in [
            format!("domain_{}", miss.domain),
            format!("source_{}", miss.source_id),
        ] {
            *category_counts.entry(category).or_default() += 1;
        }
    }
    let category_breakdown = category_counts
        .into_iter()
        .map(
            |(category, miss_count)| TinyCleanTrainingMissAuditCategoryReport {
                category,
                miss_count,
            },
        )
        .collect::<Vec<_>>();

    let mut critical_breakdown = vec![
        TinyCleanTrainingMissAuditGuardReport {
            guard: "validation_miss_coverage".to_string(),
            miss_count: validation_miss_count,
        },
        TinyCleanTrainingMissAuditGuardReport {
            guard: "unseen_context_miss".to_string(),
            miss_count: unseen_context_miss_count,
        },
        TinyCleanTrainingMissAuditGuardReport {
            guard: "ambiguous_context_miss".to_string(),
            miss_count: ambiguous_context_miss_count,
        },
    ];
    critical_breakdown.sort_by(|left, right| left.guard.cmp(&right.guard));

    Ok(TinyCleanTrainingMissAuditReport {
        run_name: run_name.to_string(),
        pack_name: pack.name.clone(),
        validation_next_token_count: analysis.validation_next_token_count,
        validation_exact_match_count: analysis.validation_exact_match_count,
        validation_miss_count,
        validation_miss_rate_bps,
        unseen_context_miss_count,
        ambiguous_context_miss_count,
        category_breakdown,
        critical_breakdown,
        misses: analysis
            .validation_misses
            .into_iter()
            .map(|entry| TinyCleanTrainingMissAuditEntry {
                sample_id: entry.sample_id,
                source_id: entry.source_id,
                domain: entry.domain,
                context: entry.context,
                actual_next_token: entry.actual_next_token,
                predicted_next_token: entry.predicted_next_token,
                predicted_transition_count: entry.predicted_transition_count,
                candidate_count: entry.candidate_count,
                unseen_context: entry.unseen_context,
            })
            .collect(),
    })
}

pub fn build_tiny_clean_training_miss_audit_delta_report(
    expected: &TinyCleanTrainingMissAuditReport,
    actual: &TinyCleanTrainingMissAuditReport,
) -> TinyCleanTrainingMissAuditDeltaReport {
    TinyCleanTrainingMissAuditDeltaReport {
        run_name: actual.run_name.clone(),
        matches_expected: expected == actual,
        field_drifts: build_tiny_training_miss_audit_field_drifts(expected, actual),
        miss_drifts: build_named_bool_drifts(
            "tiny_training_miss",
            build_tiny_training_miss_presence(expected),
            build_tiny_training_miss_presence(actual),
        ),
    }
}

pub fn build_foundation_overview_report(
    corpus_summary: &SourceAcceptanceSummaryReport,
    corpus_delta: &SourceAcceptanceDeltaReport,
    tokenizer_report: &TokenizerExperimentReport,
    tokenizer_delta: &TokenizerExperimentDeltaReport,
    eval_report: &EvalBenchmarkReport,
    eval_delta: &EvalBenchmarkDeltaReport,
    training_consistency: &BaselineTrainingConsistencyReport,
    training_delta: &BaselineTrainingDeltaReport,
    tiny_training: &TinyCleanTrainingReport,
    tiny_training_miss_audit: &TinyCleanTrainingMissAuditReport,
    tiny_training_miss_audit_delta: &TinyCleanTrainingMissAuditDeltaReport,
    tiny_profile_policy: &TinyCleanTrainingProfileBaselineReport,
    tiny_profile_policy_delta: &TinyCleanTrainingProfileBaselineDeltaReport,
    tiny_profile_strategy: &TinyCleanTrainingProfileStrategyReport,
    tiny_profile_strategy_delta: &TinyCleanTrainingProfileStrategyDeltaReport,
    tiny_profile_experiment_matrix: &TinyCleanTrainingProfileExperimentMatrixReport,
    tiny_profile_experiment_matrix_delta: &TinyCleanTrainingProfileExperimentMatrixDeltaReport,
    tiny_profile_experiment_matrix_policy: &TinyCleanTrainingProfileExperimentMatrixPolicyReport,
    tiny_profile_experiment_matrix_policy_delta: &TinyCleanTrainingProfileExperimentMatrixPolicyDeltaReport,
    tiny_profile_promotion: &TinyCleanTrainingProfilePromotionReport,
    tiny_profile_promotion_delta: &TinyCleanTrainingProfilePromotionDeltaReport,
) -> FoundationOverviewReport {
    let tiny_training_miss_audit_ready = tiny_training_miss_audit_delta.matches_expected
        && tiny_training_miss_audit.validation_miss_count
            + tiny_training_miss_audit.validation_exact_match_count
            == tiny_training_miss_audit.validation_next_token_count;
    let tiny_profile_policy_ready = tiny_profile_policy_delta.matches_expected
        && tiny_profile_policy.matches_expected_best_profile
        && tiny_profile_policy.meets_minimum_validation_threshold;
    let tiny_profile_strategy_ready = tiny_profile_strategy_delta.matches_expected
        && tiny_profile_strategy.promotable_profile_count > 0;
    let tiny_profile_experiment_matrix_ready = tiny_profile_experiment_matrix_delta
        .matches_expected
        && tiny_profile_experiment_matrix
            .matrix_checks
            .iter()
            .all(|entry| entry.passed)
        && tiny_profile_experiment_matrix.candidate_count > 0;
    let tiny_profile_experiment_matrix_policy_ready = tiny_profile_experiment_matrix_policy_delta
        .matches_expected
        && tiny_profile_experiment_matrix_policy
            .policy_checks
            .iter()
            .all(|entry| entry.passed)
        && tiny_profile_experiment_matrix_policy.eligible_profile_count > 0;
    let tiny_profile_promotion_ready = tiny_profile_promotion_delta.matches_expected
        && tiny_profile_promotion.matches_expected_active_profile
        && tiny_profile_promotion.promotable_profile_count > 0;

    let layer_breakdown = vec![
        FoundationOverviewLayerReport {
            layer: "corpus".to_string(),
            ready: corpus_delta.matches_expected,
            primary_metric_name: "accepted_source_count".to_string(),
            primary_metric_value: corpus_summary.accepted_source_count.to_string(),
        },
        FoundationOverviewLayerReport {
            layer: "tokenizer".to_string(),
            ready: tokenizer_delta.matches_expected,
            primary_metric_name: "exact_match_rate_bps".to_string(),
            primary_metric_value: tokenizer_report.exact_match_rate_bps.to_string(),
        },
        FoundationOverviewLayerReport {
            layer: "eval".to_string(),
            ready: eval_delta.matches_expected,
            primary_metric_name: "task_count".to_string(),
            primary_metric_value: eval_report.task_count.to_string(),
        },
        FoundationOverviewLayerReport {
            layer: "train".to_string(),
            ready: training_delta.assembly_matches_expected
                && training_delta.consistency_matches_expected,
            primary_metric_name: "total_sequence_count".to_string(),
            primary_metric_value: training_consistency.total_sequence_count.to_string(),
        },
        FoundationOverviewLayerReport {
            layer: "tiny_training".to_string(),
            ready: tiny_training.validation_next_token_count > 0,
            primary_metric_name: "validation_exact_match_rate_bps".to_string(),
            primary_metric_value: tiny_training.validation_exact_match_rate_bps.to_string(),
        },
        FoundationOverviewLayerReport {
            layer: "tiny_training_miss_audit".to_string(),
            ready: tiny_training_miss_audit_ready,
            primary_metric_name: "validation_miss_count".to_string(),
            primary_metric_value: tiny_training_miss_audit.validation_miss_count.to_string(),
        },
        FoundationOverviewLayerReport {
            layer: "tiny_profile_policy".to_string(),
            ready: tiny_profile_policy_ready,
            primary_metric_name: "policy_matches_expected".to_string(),
            primary_metric_value: tiny_profile_policy_ready.to_string(),
        },
        FoundationOverviewLayerReport {
            layer: "tiny_profile_strategy".to_string(),
            ready: tiny_profile_strategy_ready,
            primary_metric_name: "strategy_matches_expected".to_string(),
            primary_metric_value: tiny_profile_strategy_ready.to_string(),
        },
        FoundationOverviewLayerReport {
            layer: "tiny_profile_experiment_matrix".to_string(),
            ready: tiny_profile_experiment_matrix_ready,
            primary_metric_name: "matrix_matches_expected".to_string(),
            primary_metric_value: tiny_profile_experiment_matrix_ready.to_string(),
        },
        FoundationOverviewLayerReport {
            layer: "tiny_profile_experiment_matrix_policy".to_string(),
            ready: tiny_profile_experiment_matrix_policy_ready,
            primary_metric_name: "matrix_policy_matches_expected".to_string(),
            primary_metric_value: tiny_profile_experiment_matrix_policy_ready.to_string(),
        },
        FoundationOverviewLayerReport {
            layer: "tiny_profile_promotion".to_string(),
            ready: tiny_profile_promotion_ready,
            primary_metric_name: "promotion_matches_expected".to_string(),
            primary_metric_value: tiny_profile_promotion_ready.to_string(),
        },
    ];
    let critical_breakdown = vec![
        FoundationOverviewCheck {
            check: "corpus_matches_expected".to_string(),
            passed: corpus_delta.matches_expected,
        },
        FoundationOverviewCheck {
            check: "tokenizer_matches_expected".to_string(),
            passed: tokenizer_delta.matches_expected,
        },
        FoundationOverviewCheck {
            check: "eval_matches_expected".to_string(),
            passed: eval_delta.matches_expected,
        },
        FoundationOverviewCheck {
            check: "training_matches_expected".to_string(),
            passed: training_delta.assembly_matches_expected
                && training_delta.consistency_matches_expected,
        },
        FoundationOverviewCheck {
            check: "tiny_training_has_validation".to_string(),
            passed: tiny_training.validation_next_token_count > 0,
        },
        FoundationOverviewCheck {
            check: "tiny_training_uses_clean_sources".to_string(),
            passed: tiny_training
                .critical_breakdown
                .iter()
                .any(|entry| entry.guard == "clean_source_only" && entry.sample_count > 0),
        },
        FoundationOverviewCheck {
            check: "tiny_training_miss_audit_matches_expected".to_string(),
            passed: tiny_training_miss_audit_ready,
        },
        FoundationOverviewCheck {
            check: "tiny_profile_policy_matches_expected".to_string(),
            passed: tiny_profile_policy_ready,
        },
        FoundationOverviewCheck {
            check: "tiny_profile_strategy_matches_expected".to_string(),
            passed: tiny_profile_strategy_ready,
        },
        FoundationOverviewCheck {
            check: "tiny_profile_experiment_matrix_matches_expected".to_string(),
            passed: tiny_profile_experiment_matrix_ready,
        },
        FoundationOverviewCheck {
            check: "tiny_profile_experiment_matrix_policy_matches_expected".to_string(),
            passed: tiny_profile_experiment_matrix_policy_ready,
        },
        FoundationOverviewCheck {
            check: "tiny_profile_promotion_matches_expected".to_string(),
            passed: tiny_profile_promotion_ready,
        },
    ];

    FoundationOverviewReport {
        foundation_name: "adam-foundation-overview".to_string(),
        all_layers_match_expected: critical_breakdown.iter().all(|check| check.passed),
        accepted_source_count: corpus_summary.accepted_source_count,
        tokenizer_exact_match_rate_bps: tokenizer_report.exact_match_rate_bps,
        eval_task_count: eval_report.task_count,
        training_total_sequence_count: training_consistency.total_sequence_count,
        tiny_training_vocabulary_size: tiny_training.vocabulary_size,
        tiny_training_validation_exact_match_rate_bps: tiny_training
            .validation_exact_match_rate_bps,
        tiny_training_validation_miss_count: tiny_training_miss_audit.validation_miss_count,
        tiny_training_miss_audit_matches_expected: tiny_training_miss_audit_ready,
        tiny_profile_policy_matches_expected: tiny_profile_policy_ready,
        tiny_profile_strategy_matches_expected: tiny_profile_strategy_ready,
        tiny_profile_experiment_matrix_matches_expected: tiny_profile_experiment_matrix_ready,
        tiny_profile_experiment_matrix_policy_matches_expected:
            tiny_profile_experiment_matrix_policy_ready,
        tiny_profile_promotion_matches_expected: tiny_profile_promotion_ready,
        layer_breakdown,
        critical_breakdown,
    }
}

pub fn build_foundation_overview_delta_report(
    expected: &FoundationOverviewReport,
    actual: &FoundationOverviewReport,
) -> FoundationOverviewDeltaReport {
    FoundationOverviewDeltaReport {
        foundation_name: actual.foundation_name.clone(),
        matches_expected: expected == actual,
        field_drifts: build_foundation_field_drifts(expected, actual),
        layer_drifts: build_named_bool_drifts(
            "layer",
            expected
                .layer_breakdown
                .iter()
                .map(|entry| (entry.layer.clone(), entry.ready))
                .collect(),
            actual
                .layer_breakdown
                .iter()
                .map(|entry| (entry.layer.clone(), entry.ready))
                .collect(),
        ),
        check_drifts: build_named_bool_drifts(
            "check",
            expected
                .critical_breakdown
                .iter()
                .map(|entry| (entry.check.clone(), entry.passed))
                .collect(),
            actual
                .critical_breakdown
                .iter()
                .map(|entry| (entry.check.clone(), entry.passed))
                .collect(),
        ),
    }
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
    expected: Vec<(String, u64)>,
    actual: Vec<(String, u64)>,
) -> Vec<BaselineTrainingNamedCountDrift> {
    let mut expected_map = expected.into_iter().collect::<BTreeMap<_, _>>();
    let mut actual_map = actual.into_iter().collect::<BTreeMap<_, _>>();
    let mut keys = expected_map
        .keys()
        .chain(actual_map.keys())
        .cloned()
        .collect::<Vec<_>>();
    keys.sort_unstable();
    keys.dedup();

    let mut drifts = Vec::new();
    for key in keys {
        let expected_value = expected_map.remove(&key);
        let actual_value = actual_map.remove(&key);
        if expected_value != actual_value {
            drifts.push(BaselineTrainingNamedCountDrift {
                scope: scope.to_string(),
                key,
                expected: expected_value,
                actual: actual_value,
            });
        }
    }
    drifts
}

fn build_named_bool_drifts(
    scope: &str,
    expected: Vec<(String, bool)>,
    actual: Vec<(String, bool)>,
) -> Vec<BaselineTrainingNamedBoolDrift> {
    let mut expected_map = expected.into_iter().collect::<BTreeMap<_, _>>();
    let mut actual_map = actual.into_iter().collect::<BTreeMap<_, _>>();
    let mut keys = expected_map
        .keys()
        .chain(actual_map.keys())
        .cloned()
        .collect::<Vec<_>>();
    keys.sort_unstable();
    keys.dedup();

    let mut drifts = Vec::new();
    for key in keys {
        let expected_value = expected_map.remove(&key);
        let actual_value = actual_map.remove(&key);
        if expected_value != actual_value {
            drifts.push(BaselineTrainingNamedBoolDrift {
                scope: scope.to_string(),
                key,
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

fn build_foundation_field_drifts(
    expected: &FoundationOverviewReport,
    actual: &FoundationOverviewReport,
) -> Vec<BaselineTrainingFieldDrift> {
    let mut drifts = Vec::new();
    push_field_drift(
        &mut drifts,
        "accepted_source_count",
        expected.accepted_source_count,
        actual.accepted_source_count,
    );
    push_field_drift(
        &mut drifts,
        "tokenizer_exact_match_rate_bps",
        expected.tokenizer_exact_match_rate_bps,
        actual.tokenizer_exact_match_rate_bps,
    );
    push_field_drift(
        &mut drifts,
        "eval_task_count",
        expected.eval_task_count,
        actual.eval_task_count,
    );
    push_field_drift(
        &mut drifts,
        "training_total_sequence_count",
        expected.training_total_sequence_count,
        actual.training_total_sequence_count,
    );
    push_field_drift(
        &mut drifts,
        "tiny_training_vocabulary_size",
        expected.tiny_training_vocabulary_size,
        actual.tiny_training_vocabulary_size,
    );
    push_field_drift(
        &mut drifts,
        "tiny_training_validation_exact_match_rate_bps",
        expected.tiny_training_validation_exact_match_rate_bps,
        actual.tiny_training_validation_exact_match_rate_bps,
    );
    push_field_drift(
        &mut drifts,
        "tiny_training_validation_miss_count",
        expected.tiny_training_validation_miss_count,
        actual.tiny_training_validation_miss_count,
    );
    push_field_drift(
        &mut drifts,
        "tiny_training_miss_audit_matches_expected",
        expected.tiny_training_miss_audit_matches_expected,
        actual.tiny_training_miss_audit_matches_expected,
    );
    push_field_drift(
        &mut drifts,
        "tiny_profile_policy_matches_expected",
        expected.tiny_profile_policy_matches_expected,
        actual.tiny_profile_policy_matches_expected,
    );
    push_field_drift(
        &mut drifts,
        "tiny_profile_strategy_matches_expected",
        expected.tiny_profile_strategy_matches_expected,
        actual.tiny_profile_strategy_matches_expected,
    );
    push_field_drift(
        &mut drifts,
        "tiny_profile_experiment_matrix_matches_expected",
        expected.tiny_profile_experiment_matrix_matches_expected,
        actual.tiny_profile_experiment_matrix_matches_expected,
    );
    push_field_drift(
        &mut drifts,
        "tiny_profile_experiment_matrix_policy_matches_expected",
        expected.tiny_profile_experiment_matrix_policy_matches_expected,
        actual.tiny_profile_experiment_matrix_policy_matches_expected,
    );
    push_field_drift(
        &mut drifts,
        "tiny_profile_promotion_matches_expected",
        expected.tiny_profile_promotion_matches_expected,
        actual.tiny_profile_promotion_matches_expected,
    );
    drifts
}

fn build_tiny_training_miss_audit_field_drifts(
    expected: &TinyCleanTrainingMissAuditReport,
    actual: &TinyCleanTrainingMissAuditReport,
) -> Vec<BaselineTrainingFieldDrift> {
    let mut drifts = Vec::new();
    push_field_drift(
        &mut drifts,
        "run_name",
        expected.run_name.clone(),
        actual.run_name.clone(),
    );
    push_field_drift(
        &mut drifts,
        "pack_name",
        expected.pack_name.clone(),
        actual.pack_name.clone(),
    );
    push_field_drift(
        &mut drifts,
        "validation_next_token_count",
        expected.validation_next_token_count,
        actual.validation_next_token_count,
    );
    push_field_drift(
        &mut drifts,
        "validation_exact_match_count",
        expected.validation_exact_match_count,
        actual.validation_exact_match_count,
    );
    push_field_drift(
        &mut drifts,
        "validation_miss_count",
        expected.validation_miss_count,
        actual.validation_miss_count,
    );
    push_field_drift(
        &mut drifts,
        "validation_miss_rate_bps",
        expected.validation_miss_rate_bps,
        actual.validation_miss_rate_bps,
    );
    push_field_drift(
        &mut drifts,
        "unseen_context_miss_count",
        expected.unseen_context_miss_count,
        actual.unseen_context_miss_count,
    );
    push_field_drift(
        &mut drifts,
        "ambiguous_context_miss_count",
        expected.ambiguous_context_miss_count,
        actual.ambiguous_context_miss_count,
    );
    drifts
}

fn build_tiny_training_miss_presence(
    report: &TinyCleanTrainingMissAuditReport,
) -> Vec<(String, bool)> {
    report
        .misses
        .iter()
        .map(|entry| {
            (
                format!(
                    "{}::{}::{}",
                    entry.sample_id, entry.context, entry.actual_next_token
                ),
                true,
            )
        })
        .collect()
}

fn build_tiny_profile_baseline_field_drifts(
    expected: &TinyCleanTrainingProfileBaselineReport,
    actual: &TinyCleanTrainingProfileBaselineReport,
) -> Vec<BaselineTrainingFieldDrift> {
    let mut drifts = Vec::new();
    push_field_drift(
        &mut drifts,
        "policy_name",
        expected.policy_name.clone(),
        actual.policy_name.clone(),
    );
    push_field_drift(
        &mut drifts,
        "suite_name",
        expected.suite_name.clone(),
        actual.suite_name.clone(),
    );
    push_field_drift(
        &mut drifts,
        "expected_best_profile",
        expected.expected_best_profile.clone(),
        actual.expected_best_profile.clone(),
    );
    push_field_drift(
        &mut drifts,
        "actual_best_profile",
        expected.actual_best_profile.clone(),
        actual.actual_best_profile.clone(),
    );
    push_field_drift(
        &mut drifts,
        "selected_pack_name",
        expected.selected_pack_name.clone(),
        actual.selected_pack_name.clone(),
    );
    push_field_drift(
        &mut drifts,
        "selected_validation_exact_match_rate_bps",
        expected.selected_validation_exact_match_rate_bps,
        actual.selected_validation_exact_match_rate_bps,
    );
    push_field_drift(
        &mut drifts,
        "matches_expected_best_profile",
        expected.matches_expected_best_profile,
        actual.matches_expected_best_profile,
    );
    push_field_drift(
        &mut drifts,
        "meets_minimum_validation_threshold",
        expected.meets_minimum_validation_threshold,
        actual.meets_minimum_validation_threshold,
    );
    drifts
}

fn build_tiny_profile_strategy_field_drifts(
    expected: &TinyCleanTrainingProfileStrategyReport,
    actual: &TinyCleanTrainingProfileStrategyReport,
) -> Vec<BaselineTrainingFieldDrift> {
    let mut drifts = Vec::new();
    push_field_drift(
        &mut drifts,
        "strategy_name",
        expected.strategy_name.clone(),
        actual.strategy_name.clone(),
    );
    push_field_drift(
        &mut drifts,
        "suite_name",
        expected.suite_name.clone(),
        actual.suite_name.clone(),
    );
    push_field_drift(
        &mut drifts,
        "baseline_profile",
        expected.baseline_profile.clone(),
        actual.baseline_profile.clone(),
    );
    push_field_drift(
        &mut drifts,
        "baseline_validation_exact_match_rate_bps",
        expected.baseline_validation_exact_match_rate_bps,
        actual.baseline_validation_exact_match_rate_bps,
    );
    push_field_drift(
        &mut drifts,
        "promotable_profile_count",
        expected.promotable_profile_count,
        actual.promotable_profile_count,
    );
    drifts
}

fn build_tiny_profile_promotion_field_drifts(
    expected: &TinyCleanTrainingProfilePromotionReport,
    actual: &TinyCleanTrainingProfilePromotionReport,
) -> Vec<BaselineTrainingFieldDrift> {
    let mut drifts = Vec::new();
    push_field_drift(
        &mut drifts,
        "promotion_name",
        expected.promotion_name.clone(),
        actual.promotion_name.clone(),
    );
    push_field_drift(
        &mut drifts,
        "policy_name",
        expected.policy_name.clone(),
        actual.policy_name.clone(),
    );
    push_field_drift(
        &mut drifts,
        "suite_name",
        expected.suite_name.clone(),
        actual.suite_name.clone(),
    );
    push_field_drift(
        &mut drifts,
        "expected_active_profile",
        expected.expected_active_profile.clone(),
        actual.expected_active_profile.clone(),
    );
    push_field_drift(
        &mut drifts,
        "active_profile",
        expected.active_profile.clone(),
        actual.active_profile.clone(),
    );
    push_field_drift(
        &mut drifts,
        "active_pack_name",
        expected.active_pack_name.clone(),
        actual.active_pack_name.clone(),
    );
    push_field_drift(
        &mut drifts,
        "promotable_profile_count",
        expected.promotable_profile_count,
        actual.promotable_profile_count,
    );
    push_field_drift(
        &mut drifts,
        "matches_expected_active_profile",
        expected.matches_expected_active_profile,
        actual.matches_expected_active_profile,
    );
    drifts
}

fn build_tiny_profile_experiment_matrix_field_drifts(
    expected: &TinyCleanTrainingProfileExperimentMatrixReport,
    actual: &TinyCleanTrainingProfileExperimentMatrixReport,
) -> Vec<BaselineTrainingFieldDrift> {
    let mut drifts = Vec::new();
    push_field_drift(
        &mut drifts,
        "matrix_name",
        expected.matrix_name.clone(),
        actual.matrix_name.clone(),
    );
    push_field_drift(
        &mut drifts,
        "suite_name",
        expected.suite_name.clone(),
        actual.suite_name.clone(),
    );
    push_field_drift(
        &mut drifts,
        "strategy_name",
        expected.strategy_name.clone(),
        actual.strategy_name.clone(),
    );
    push_field_drift(
        &mut drifts,
        "promotion_name",
        expected.promotion_name.clone(),
        actual.promotion_name.clone(),
    );
    push_field_drift(
        &mut drifts,
        "expected_active_profile",
        expected.expected_active_profile.clone(),
        actual.expected_active_profile.clone(),
    );
    push_field_drift(
        &mut drifts,
        "active_profile",
        expected.active_profile.clone(),
        actual.active_profile.clone(),
    );
    push_field_drift(
        &mut drifts,
        "candidate_count",
        expected.candidate_count,
        actual.candidate_count,
    );
    push_field_drift(
        &mut drifts,
        "best_profile",
        expected.best_profile.clone(),
        actual.best_profile.clone(),
    );
    push_field_drift(
        &mut drifts,
        "best_validation_exact_match_rate_bps",
        expected.best_validation_exact_match_rate_bps,
        actual.best_validation_exact_match_rate_bps,
    );
    drifts
}

fn build_tiny_profile_experiment_matrix_policy_field_drifts(
    expected: &TinyCleanTrainingProfileExperimentMatrixPolicyReport,
    actual: &TinyCleanTrainingProfileExperimentMatrixPolicyReport,
) -> Vec<BaselineTrainingFieldDrift> {
    let mut drifts = Vec::new();
    push_field_drift(
        &mut drifts,
        "policy_name",
        expected.policy_name.clone(),
        actual.policy_name.clone(),
    );
    push_field_drift(
        &mut drifts,
        "matrix_name",
        expected.matrix_name.clone(),
        actual.matrix_name.clone(),
    );
    push_field_drift(
        &mut drifts,
        "suite_name",
        expected.suite_name.clone(),
        actual.suite_name.clone(),
    );
    push_field_drift(
        &mut drifts,
        "expected_selected_profile",
        expected.expected_selected_profile.clone(),
        actual.expected_selected_profile.clone(),
    );
    push_field_drift(
        &mut drifts,
        "selected_profile",
        expected.selected_profile.clone(),
        actual.selected_profile.clone(),
    );
    push_field_drift(
        &mut drifts,
        "selected_pack_name",
        expected.selected_pack_name.clone(),
        actual.selected_pack_name.clone(),
    );
    push_field_drift(
        &mut drifts,
        "selected_validation_exact_match_rate_bps",
        expected.selected_validation_exact_match_rate_bps,
        actual.selected_validation_exact_match_rate_bps,
    );
    push_field_drift(
        &mut drifts,
        "selected_validation_gap_bps",
        expected.selected_validation_gap_bps,
        actual.selected_validation_gap_bps,
    );
    push_field_drift(
        &mut drifts,
        "best_profile",
        expected.best_profile.clone(),
        actual.best_profile.clone(),
    );
    push_field_drift(
        &mut drifts,
        "best_validation_exact_match_rate_bps",
        expected.best_validation_exact_match_rate_bps,
        actual.best_validation_exact_match_rate_bps,
    );
    push_field_drift(
        &mut drifts,
        "candidate_count",
        expected.candidate_count,
        actual.candidate_count,
    );
    push_field_drift(
        &mut drifts,
        "eligible_profile_count",
        expected.eligible_profile_count,
        actual.eligible_profile_count,
    );
    push_field_drift(
        &mut drifts,
        "matches_expected_selected_profile",
        expected.matches_expected_selected_profile,
        actual.matches_expected_selected_profile,
    );
    push_field_drift(
        &mut drifts,
        "selected_profile_is_best",
        expected.selected_profile_is_best,
        actual.selected_profile_is_best,
    );
    push_field_drift(
        &mut drifts,
        "selected_profile_is_promotable",
        expected.selected_profile_is_promotable,
        actual.selected_profile_is_promotable,
    );
    push_field_drift(
        &mut drifts,
        "meets_minimum_validation_threshold",
        expected.meets_minimum_validation_threshold,
        actual.meets_minimum_validation_threshold,
    );
    push_field_drift(
        &mut drifts,
        "within_gap_budget",
        expected.within_gap_budget,
        actual.within_gap_budget,
    );
    drifts
}

fn build_promotable_profile_bools(
    report: &TinyCleanTrainingProfileStrategyReport,
) -> Vec<(String, bool)> {
    report
        .candidate_reports
        .iter()
        .map(|entry| (entry.profile.clone(), entry.promotable))
        .collect()
}

fn build_strategy_candidate_flag_bools(
    report: &TinyCleanTrainingProfileStrategyReport,
) -> Vec<(String, bool)> {
    let mut flags = Vec::new();
    for entry in &report.candidate_reports {
        flags.push((
            format!("{}::meets_minimum_validation_threshold", entry.profile),
            entry.meets_minimum_validation_threshold,
        ));
        flags.push((
            format!("{}::within_gap_budget", entry.profile),
            entry.within_gap_budget,
        ));
        flags.push((format!("{}::promotable", entry.profile), entry.promotable));
    }
    flags
}

fn build_strategy_candidate_metrics(
    report: &TinyCleanTrainingProfileStrategyReport,
) -> Vec<(String, u64)> {
    let mut metrics = Vec::new();
    for entry in &report.candidate_reports {
        metrics.push((
            format!("{}::validation_exact_match_rate_bps", entry.profile),
            entry.validation_exact_match_rate_bps as u64,
        ));
        metrics.push((
            format!("{}::validation_gap_bps", entry.profile),
            entry.validation_gap_bps as u64,
        ));
    }
    metrics
}

fn build_experiment_matrix_entry_flags(
    report: &TinyCleanTrainingProfileExperimentMatrixReport,
) -> Vec<(String, bool)> {
    let mut flags = Vec::new();
    for entry in &report.entry_reports {
        flags.push((
            format!("{}::is_active_profile", entry.profile),
            entry.is_active_profile,
        ));
        flags.push((
            format!("{}::is_promotable_profile", entry.profile),
            entry.is_promotable_profile,
        ));
    }
    flags
}

fn build_experiment_matrix_entry_metrics(
    report: &TinyCleanTrainingProfileExperimentMatrixReport,
) -> Vec<(String, u64)> {
    let mut metrics = Vec::new();
    for entry in &report.entry_reports {
        metrics.push((
            format!("{}::sample_count", entry.profile),
            entry.sample_count as u64,
        ));
        metrics.push((
            format!("{}::train_token_count", entry.profile),
            entry.train_token_count as u64,
        ));
        metrics.push((
            format!("{}::vocabulary_size", entry.profile),
            entry.vocabulary_size as u64,
        ));
        metrics.push((
            format!("{}::deterministic_context_count", entry.profile),
            entry.deterministic_context_count as u64,
        ));
        metrics.push((
            format!("{}::ambiguous_context_count", entry.profile),
            entry.ambiguous_context_count as u64,
        ));
        metrics.push((
            format!("{}::validation_exact_match_rate_bps", entry.profile),
            entry.validation_exact_match_rate_bps as u64,
        ));
        metrics.push((
            format!("{}::validation_gap_bps", entry.profile),
            entry.validation_gap_bps as u64,
        ));
    }
    metrics
}

fn build_experiment_matrix_policy_candidate_flags(
    report: &TinyCleanTrainingProfileExperimentMatrixPolicyReport,
) -> Vec<(String, bool)> {
    let mut flags = Vec::new();
    for entry in &report.candidate_decisions {
        flags.push((
            format!("{}::is_promotable_profile", entry.profile),
            entry.is_promotable_profile,
        ));
        flags.push((
            format!("{}::meets_minimum_validation_threshold", entry.profile),
            entry.meets_minimum_validation_threshold,
        ));
        flags.push((
            format!("{}::within_gap_budget", entry.profile),
            entry.within_gap_budget,
        ));
        flags.push((format!("{}::is_eligible", entry.profile), entry.is_eligible));
    }
    flags
}

fn build_experiment_matrix_policy_candidate_reasons(
    report: &TinyCleanTrainingProfileExperimentMatrixPolicyReport,
) -> Vec<(String, bool)> {
    let mut reasons = Vec::new();
    for entry in &report.candidate_decisions {
        for reason in &entry.rejection_reasons {
            reasons.push((format!("{}::{}", entry.profile, reason), true));
        }
    }
    reasons
}

fn compare_profile_summaries(
    left: &TinyCleanTrainingProfileSummary,
    right: &TinyCleanTrainingProfileSummary,
) -> std::cmp::Ordering {
    left.validation_exact_match_rate_bps
        .cmp(&right.validation_exact_match_rate_bps)
        .then_with(|| left.sample_count.cmp(&right.sample_count))
        .then_with(|| left.train_token_count.cmp(&right.train_token_count))
        .then_with(|| right.profile.cmp(&left.profile))
}

fn compare_profile_experiment_entries(
    left: &TinyCleanTrainingProfileExperimentEntryReport,
    right: &TinyCleanTrainingProfileExperimentEntryReport,
) -> std::cmp::Ordering {
    left.validation_exact_match_rate_bps
        .cmp(&right.validation_exact_match_rate_bps)
        .then_with(|| left.sample_count.cmp(&right.sample_count))
        .then_with(|| left.train_token_count.cmp(&right.train_token_count))
        .then_with(|| left.vocabulary_size.cmp(&right.vocabulary_size))
        .then_with(|| right.profile.cmp(&left.profile))
}

fn assemble_tiny_clean_training_pack_with_domain_limits(
    version: &str,
    name: &str,
    target_language: &str,
    script: &str,
    clean_corpus_pack: &CleanTrainingCorpusPack,
    domain_limits: &BTreeMap<String, usize>,
) -> Result<TinyCleanTrainingPack, TrainingError> {
    let mut domain_counts = BTreeMap::<String, usize>::new();
    let mut samples = Vec::new();
    let mut next_index = 1usize;
    for sample in &clean_corpus_pack.samples {
        let Some(limit) = domain_limits.get(&sample.domain) else {
            continue;
        };
        let count = domain_counts.entry(sample.domain.clone()).or_default();
        if *count >= *limit {
            continue;
        }
        samples.push(TinyCleanTrainingSample {
            id: format!("clean_sample_{next_index:02}"),
            source_id: sample.source_id.clone(),
            domain: sample.domain.clone(),
            text: sample.text.clone(),
        });
        *count += 1;
        next_index += 1;
    }

    let distinct_domains = samples
        .iter()
        .map(|sample| sample.domain.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    if samples.is_empty() || distinct_domains.len() < 3 {
        return Err(TrainingError::TinyTrainingSelectionMismatch);
    }

    Ok(TinyCleanTrainingPack {
        version: version.to_string(),
        name: name.to_string(),
        target_language: target_language.to_string(),
        script: script.to_string(),
        samples,
    })
}

fn assemble_tiny_clean_training_pack_with_domain_limits_and_preferences(
    version: &str,
    name: &str,
    target_language: &str,
    script: &str,
    clean_corpus_pack: &CleanTrainingCorpusPack,
    domain_limits: &BTreeMap<String, usize>,
    preferred_sample_texts_by_domain: &BTreeMap<String, Vec<String>>,
) -> Result<TinyCleanTrainingPack, TrainingError> {
    let mut selected_signatures = std::collections::BTreeSet::<(String, String, String)>::new();
    let mut domain_counts = BTreeMap::<String, usize>::new();

    for (domain, sample_texts) in preferred_sample_texts_by_domain {
        let Some(limit) = domain_limits.get(domain) else {
            return Err(TrainingError::MiniTrainingManifestMismatch);
        };
        if sample_texts.len() > *limit {
            return Err(TrainingError::MiniTrainingManifestMismatch);
        }

        for sample_text in sample_texts {
            let sample = clean_corpus_pack
                .samples
                .iter()
                .find(|entry| entry.domain == *domain && entry.text == *sample_text)
                .ok_or(TrainingError::MiniTrainingManifestMismatch)?;
            let signature = (
                sample.domain.clone(),
                sample.source_id.clone(),
                sample.text.clone(),
            );
            if !selected_signatures.insert(signature) {
                return Err(TrainingError::MiniTrainingManifestMismatch);
            }
            *domain_counts.entry(domain.clone()).or_default() += 1;
        }
    }

    for sample in &clean_corpus_pack.samples {
        let Some(limit) = domain_limits.get(&sample.domain) else {
            continue;
        };
        let signature = (
            sample.domain.clone(),
            sample.source_id.clone(),
            sample.text.clone(),
        );
        if selected_signatures.contains(&signature) {
            continue;
        }
        let count = domain_counts.entry(sample.domain.clone()).or_default();
        if *count >= *limit {
            continue;
        }
        selected_signatures.insert(signature);
        *count += 1;
    }

    let mut samples = Vec::new();
    let mut next_index = 1usize;
    for sample in &clean_corpus_pack.samples {
        let signature = (
            sample.domain.clone(),
            sample.source_id.clone(),
            sample.text.clone(),
        );
        if !selected_signatures.contains(&signature) {
            continue;
        }
        samples.push(TinyCleanTrainingSample {
            id: format!("clean_sample_{next_index:02}"),
            source_id: sample.source_id.clone(),
            domain: sample.domain.clone(),
            text: sample.text.clone(),
        });
        next_index += 1;
    }

    let distinct_domains = samples
        .iter()
        .map(|sample| sample.domain.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    if samples.is_empty() || distinct_domains.len() < 3 {
        return Err(TrainingError::MiniTrainingManifestMismatch);
    }

    Ok(TinyCleanTrainingPack {
        version: version.to_string(),
        name: name.to_string(),
        target_language: target_language.to_string(),
        script: script.to_string(),
        samples,
    })
}

fn deterministic_round_robin_tiny_samples(
    samples: &[TinyCleanTrainingSample],
) -> Vec<TinyCleanTrainingSample> {
    let mut ordered_samples = samples.to_vec();
    ordered_samples.sort_by(|left, right| left.id.cmp(&right.id));

    let mut buckets =
        BTreeMap::<String, std::collections::VecDeque<TinyCleanTrainingSample>>::new();
    for sample in ordered_samples {
        buckets
            .entry(sample.domain.clone())
            .or_default()
            .push_back(sample);
    }

    let mut round_robin = Vec::new();
    loop {
        let mut pushed_any = false;
        for bucket in buckets.values_mut() {
            if let Some(sample) = bucket.pop_front() {
                round_robin.push(sample);
                pushed_any = true;
            }
        }
        if !pushed_any {
            break;
        }
    }

    round_robin
}

fn split_tiny_training_samples(
    ordered_samples: &[TinyCleanTrainingSample],
    validation_sample_count: usize,
) -> (Vec<TinyCleanTrainingSample>, Vec<TinyCleanTrainingSample>) {
    let mut domain_buckets = BTreeMap::<String, Vec<TinyCleanTrainingSample>>::new();
    for sample in ordered_samples {
        domain_buckets
            .entry(sample.domain.clone())
            .or_default()
            .push(sample.clone());
    }

    let total_sample_count = ordered_samples.len();
    let mut allocations = BTreeMap::<String, usize>::new();
    let mut remainders = Vec::new();
    let mut allocated_total = 0usize;
    for (domain, samples) in &domain_buckets {
        let numerator = samples.len() as u128 * validation_sample_count as u128;
        let allocation = (numerator / total_sample_count as u128) as usize;
        allocations.insert(domain.clone(), allocation);
        allocated_total += allocation;
        remainders.push((
            domain.clone(),
            numerator % total_sample_count as u128,
            samples.len(),
        ));
    }

    remainders.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then_with(|| right.2.cmp(&left.2))
            .then_with(|| left.0.cmp(&right.0))
    });
    let mut remainder_index = 0usize;
    while allocated_total < validation_sample_count && !remainders.is_empty() {
        let domain = &remainders[remainder_index % remainders.len()].0;
        let current = allocations.get(domain).copied().unwrap_or_default();
        let domain_total = domain_buckets.get(domain).map(Vec::len).unwrap_or_default();
        if current < domain_total {
            allocations.insert(domain.clone(), current + 1);
            allocated_total += 1;
        }
        remainder_index += 1;
    }

    let desired_domain_coverage = validation_sample_count.min(domain_buckets.len());
    if desired_domain_coverage > 1 {
        let mut zero_domains = allocations
            .iter()
            .filter(|(_, count)| **count == 0)
            .map(|(domain, _)| domain.clone())
            .collect::<Vec<_>>();
        zero_domains.sort_unstable();
        for zero_domain in zero_domains {
            let allocated_domains = allocations.values().filter(|count| **count > 0).count();
            if allocated_domains >= desired_domain_coverage {
                break;
            }

            let donor_domain = allocations
                .iter()
                .filter_map(|(domain, count)| {
                    (*count > 1).then_some((
                        domain.clone(),
                        *count,
                        domain_buckets.get(domain).map(Vec::len).unwrap_or_default(),
                    ))
                })
                .max_by(|left, right| {
                    left.1
                        .cmp(&right.1)
                        .then_with(|| left.2.cmp(&right.2))
                        .then_with(|| right.0.cmp(&left.0))
                })
                .map(|entry| entry.0);

            let Some(donor_domain) = donor_domain else {
                break;
            };

            if let Some(donor_count) = allocations.get_mut(&donor_domain) {
                *donor_count -= 1;
            }
            if let Some(zero_count) = allocations.get_mut(&zero_domain) {
                *zero_count += 1;
            }
        }
    }

    let mut train_samples = Vec::new();
    let mut validation_samples = Vec::new();
    for (domain, samples) in domain_buckets {
        let validation_count = allocations.get(&domain).copied().unwrap_or_default();
        let train_count = samples.len().saturating_sub(validation_count);
        let (train_bucket, validation_bucket) = samples.split_at(train_count);
        train_samples.extend(train_bucket.iter().cloned());
        validation_samples.extend(validation_bucket.iter().cloned());
    }

    (train_samples, validation_samples)
}

fn best_transition_prediction(next_tokens: &BTreeMap<String, usize>) -> Option<(&str, usize)> {
    next_tokens
        .iter()
        .max_by(|left, right| left.1.cmp(right.1).then_with(|| right.0.cmp(left.0)))
        .map(|(token, count)| (token.as_str(), *count))
}

fn analyze_tiny_clean_training(
    manifest: &BaselineTrainingManifest,
    registry: &SourceRegistry,
    rules: &SourceScoringRules,
    report: &SourceAcceptanceReport,
    pack: &TinyCleanTrainingPack,
) -> Result<TinyTrainingAnalysis, TrainingError> {
    manifest.validate()?;
    registry
        .validate()
        .map_err(|_| TrainingError::ReferencedManifestInvalid)?;
    report
        .validate(registry, rules)
        .map_err(|_| TrainingError::AcceptanceReportMismatch)?;
    pack.validate()?;

    let accepted_sources = report
        .records
        .iter()
        .filter(|record| record.accepted_for_training)
        .map(|record| record.source_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let registry_by_id = registry
        .entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry))
        .collect::<BTreeMap<_, _>>();

    for sample in &pack.samples {
        let Some(entry) = registry_by_id.get(sample.source_id.as_str()) else {
            return Err(TrainingError::TinyTrainingSourceMismatch);
        };
        if !accepted_sources.contains(sample.source_id.as_str())
            || !entry.allowed_for_training
            || entry.stage != adam_corpus::CorpusStage::Curated
            || source_domain_slug(&entry.domain) != sample.domain
        {
            return Err(TrainingError::TinyTrainingSourceMismatch);
        }
    }

    let ordered_samples = deterministic_round_robin_tiny_samples(&pack.samples);
    let validation_sample_count =
        ((ordered_samples.len() as u128 * manifest.validation_split_bps as u128) / 10_000) as usize;
    let validation_sample_count = validation_sample_count
        .max(1)
        .min(ordered_samples.len().saturating_sub(1));
    let (train_samples, validation_samples) =
        split_tiny_training_samples(&ordered_samples, validation_sample_count);
    let train_sample_count = train_samples.len();
    let validation_domain_count = validation_samples
        .iter()
        .map(|sample| sample.domain.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len();

    let train_sequences = train_samples
        .iter()
        .map(|sample| (sample, tokenize_clean_training_text(&sample.text)))
        .collect::<Vec<_>>();
    let validation_sequences = validation_samples
        .iter()
        .map(|sample| (sample, tokenize_clean_training_text(&sample.text)))
        .collect::<Vec<_>>();

    let train_token_count = train_sequences
        .iter()
        .map(|(_, tokens)| tokens.len())
        .sum::<usize>();
    let validation_token_count = validation_sequences
        .iter()
        .map(|(_, tokens)| tokens.len())
        .sum::<usize>();
    let vocabulary_size = train_sequences
        .iter()
        .flat_map(|(_, tokens)| tokens.iter().cloned())
        .collect::<std::collections::BTreeSet<_>>()
        .len();

    let mut transition_counts = BTreeMap::<String, BTreeMap<String, usize>>::new();
    for (sample, tokens) in &train_sequences {
        for (context, next_token) in sample_bigram_pairs(&sample.domain, tokens) {
            *transition_counts
                .entry(context)
                .or_default()
                .entry(next_token)
                .or_default() += 1;
        }
    }

    let unique_context_count = transition_counts.len();
    let unique_transition_count = transition_counts
        .values()
        .map(|next_tokens| next_tokens.len())
        .sum::<usize>();
    let deterministic_context_count = transition_counts
        .values()
        .filter(|next_tokens| next_tokens.len() == 1)
        .count();
    let ambiguous_context_count = unique_context_count - deterministic_context_count;

    let mut validation_next_token_count = 0usize;
    let mut validation_exact_match_count = 0usize;
    let mut validation_misses = Vec::new();
    for (sample, tokens) in &validation_sequences {
        for (context, actual_next_token) in sample_bigram_pairs(&sample.domain, tokens) {
            validation_next_token_count += 1;
            let prediction = transition_counts
                .get(&context)
                .and_then(best_transition_prediction);
            if prediction.map(|(token, _)| token) == Some(actual_next_token.as_str()) {
                validation_exact_match_count += 1;
            } else {
                let candidate_count = transition_counts
                    .get(&context)
                    .map(|next_tokens| next_tokens.len())
                    .unwrap_or_default();
                validation_misses.push(TinyTrainingValidationMiss {
                    sample_id: sample.id.clone(),
                    source_id: sample.source_id.clone(),
                    domain: sample.domain.clone(),
                    context,
                    actual_next_token,
                    predicted_next_token: prediction.map(|(token, _)| token.to_string()),
                    predicted_transition_count: prediction.map(|(_, count)| count).unwrap_or(0),
                    candidate_count,
                    unseen_context: candidate_count == 0,
                });
            }
        }
    }
    validation_misses.sort_by(|left, right| {
        left.sample_id
            .cmp(&right.sample_id)
            .then_with(|| left.context.cmp(&right.context))
            .then_with(|| left.actual_next_token.cmp(&right.actual_next_token))
    });

    let validation_exact_match_rate_bps = if validation_next_token_count == 0 {
        0
    } else {
        validation_exact_match_count * 10_000 / validation_next_token_count
    };

    let mut category_stats = BTreeMap::<String, (usize, usize)>::new();
    for sample in &ordered_samples {
        let token_count = tokenize_clean_training_text(&sample.text).len();
        for category in [
            format!("domain_{}", sample.domain),
            format!("source_{}", sample.source_id),
        ] {
            let stats = category_stats.entry(category).or_insert((0, 0));
            stats.0 += 1;
            stats.1 += token_count;
        }
    }
    let category_breakdown = category_stats
        .into_iter()
        .map(
            |(category, (sample_count, token_count))| TinyCleanTrainingCategoryReport {
                category,
                sample_count,
                token_count,
            },
        )
        .collect::<Vec<_>>();

    let mut critical_breakdown = vec![
        FoundationOverviewCheck {
            check: "accepted_source_only".to_string(),
            passed: true,
        },
        FoundationOverviewCheck {
            check: "clean_source_only".to_string(),
            passed: true,
        },
        FoundationOverviewCheck {
            check: "validation_coverage".to_string(),
            passed: !validation_samples.is_empty() && validation_next_token_count > 0,
        },
        FoundationOverviewCheck {
            check: "validation_multi_domain_coverage".to_string(),
            passed: validation_domain_count > 1,
        },
        FoundationOverviewCheck {
            check: "deterministic_transition_coverage".to_string(),
            passed: deterministic_context_count > 0,
        },
        FoundationOverviewCheck {
            check: "round_robin_domain_ordering".to_string(),
            passed: ordered_samples.len() == pack.samples.len(),
        },
    ]
    .into_iter()
    .map(|check| TinyCleanTrainingGuardReport {
        guard: check.check,
        sample_count: usize::from(check.passed) * ordered_samples.len(),
    })
    .collect::<Vec<_>>();
    critical_breakdown.sort_by(|left, right| left.guard.cmp(&right.guard));

    Ok(TinyTrainingAnalysis {
        ordered_samples,
        train_sample_count,
        validation_sample_count,
        validation_domain_count,
        train_token_count,
        validation_token_count,
        vocabulary_size,
        unique_context_count,
        unique_transition_count,
        deterministic_context_count,
        ambiguous_context_count,
        validation_next_token_count,
        validation_exact_match_count,
        validation_exact_match_rate_bps,
        category_breakdown,
        critical_breakdown,
        validation_misses,
    })
}

fn tokenize_clean_training_text(text: &str) -> Vec<String> {
    normalize_text(text)
        .split_whitespace()
        .filter_map(|token| {
            let cleaned = token
                .chars()
                .filter(|ch| ch.is_alphabetic() && !ch.is_ascii())
                .collect::<String>();
            (!cleaned.is_empty()).then_some(cleaned)
        })
        .collect()
}

fn sample_bigram_pairs(domain: &str, tokens: &[String]) -> Vec<(String, String)> {
    if tokens.is_empty() {
        return Vec::new();
    }

    let mut pairs = Vec::with_capacity(tokens.len());
    pairs.push((format!("<bos>::{domain}"), tokens[0].clone()));
    pairs.extend(bigram_pairs(tokens));
    pairs
}

#[derive(Debug, Clone)]
struct TinyTrainingValidationMiss {
    sample_id: String,
    source_id: String,
    domain: String,
    context: String,
    actual_next_token: String,
    predicted_next_token: Option<String>,
    predicted_transition_count: usize,
    candidate_count: usize,
    unseen_context: bool,
}

#[derive(Debug, Clone)]
struct TinyTrainingAnalysis {
    ordered_samples: Vec<TinyCleanTrainingSample>,
    train_sample_count: usize,
    validation_sample_count: usize,
    validation_domain_count: usize,
    train_token_count: usize,
    validation_token_count: usize,
    vocabulary_size: usize,
    unique_context_count: usize,
    unique_transition_count: usize,
    deterministic_context_count: usize,
    ambiguous_context_count: usize,
    validation_next_token_count: usize,
    validation_exact_match_count: usize,
    validation_exact_match_rate_bps: usize,
    category_breakdown: Vec<TinyCleanTrainingCategoryReport>,
    critical_breakdown: Vec<TinyCleanTrainingGuardReport>,
    validation_misses: Vec<TinyTrainingValidationMiss>,
}

fn bigram_pairs(tokens: &[String]) -> Vec<(String, String)> {
    if tokens.is_empty() {
        return Vec::new();
    }

    let mut sequence = Vec::with_capacity(tokens.len() + 1);
    sequence.extend(tokens.iter().cloned());
    sequence.push("<eos>".to_string());

    sequence
        .windows(2)
        .map(|pair| (pair[0].clone(), pair[1].clone()))
        .collect()
}

fn contains_latin(value: &str) -> bool {
    value.chars().any(|ch| ch.is_ascii_alphabetic())
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
            version: "0.1.4".to_string(),
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
            version: "0.1.4".to_string(),
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
            version: "0.1.4".to_string(),
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
            version: "0.1.4".to_string(),
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
            version: "0.1.4".to_string(),
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
            version: "0.1.4".to_string(),
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
            version: "0.1.4".to_string(),
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
            version: "0.1.4".to_string(),
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
            version: "0.1.4".to_string(),
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
            version: "0.1.4".to_string(),
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
            version: "0.1.4".to_string(),
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
            version: "0.1.4".to_string(),
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
            version: "0.1.4".to_string(),
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
            version: "0.1.4".to_string(),
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
            version: "0.1.4".to_string(),
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
            version: "0.1.4".to_string(),
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
            version: "0.1.4".to_string(),
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
            version: "0.1.4".to_string(),
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
