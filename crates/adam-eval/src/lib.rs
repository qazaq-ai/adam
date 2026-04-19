use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvalLayer {
    CorpusQuality,
    TokenizerQuality,
    ModelEval,
    LinguisticAudit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvalTaskKind {
    TokenEfficiency,
    TokenizerSegmentation,
    NextTokenPrediction,
    ReadingComprehension,
    MorphologySensitivity,
    HallucinationAudit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalTask {
    pub target_language: String,
    pub name: String,
    pub kind: EvalTaskKind,
    pub source_manifest: String,
    pub dataset_manifest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvalSplit {
    Dev,
    Test,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalExample {
    pub id: String,
    pub split: EvalSplit,
    pub kind: EvalTaskKind,
    pub prompt: String,
    pub reference_answer: Option<String>,
    pub must_answer_in_kazakh: bool,
    pub must_avoid_fabrication: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalDataset {
    pub version: String,
    pub name: String,
    pub target_language: String,
    pub script: String,
    pub entries: Vec<EvalExample>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalSuite {
    pub version: String,
    pub name: String,
    pub target_language: String,
    pub layers: Vec<EvalLayer>,
    pub tasks: Vec<EvalTask>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalBenchmarkReport {
    pub suite_name: String,
    pub target_language: String,
    pub layer_count: usize,
    pub task_count: usize,
    pub category_breakdown: Vec<EvalBenchmarkCategoryReport>,
    pub critical_breakdown: Vec<EvalBenchmarkGuardReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalBenchmarkCategoryReport {
    pub category: String,
    pub task_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalBenchmarkGuardReport {
    pub guard: String,
    pub task_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalBenchmarkDeltaReport {
    pub suite_name: String,
    pub matches_expected: bool,
    pub field_drifts: Vec<EvalBenchmarkFieldDrift>,
    pub category_drifts: Vec<EvalBenchmarkNamedCountDrift>,
    pub guard_drifts: Vec<EvalBenchmarkNamedCountDrift>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalBenchmarkFieldDrift {
    pub field: String,
    pub expected: String,
    pub actual: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalBenchmarkNamedCountDrift {
    pub scope: String,
    pub key: String,
    pub expected: Option<u64>,
    pub actual: Option<u64>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EvalError {
    #[error("evaluation language must be kazakh")]
    NonKazakhLanguage,
    #[error("evaluation script must be cyrillic")]
    NonCyrillicScript,
    #[error("evaluation tasks must not be empty")]
    EmptyTasks,
    #[error("evaluation entries must not be empty")]
    EmptyEntries,
    #[error("dataset manifest path must not be empty")]
    EmptyDatasetManifest,
    #[error("latin characters are not allowed in kazakh-only evaluation text")]
    LatinCharactersForbidden,
}

impl Default for EvalSuite {
    fn default() -> Self {
        Self {
            version: "1.3.0".to_string(),
            name: "kazakh-foundation-baseline".to_string(),
            target_language: "kazakh".to_string(),
            layers: vec![
                EvalLayer::CorpusQuality,
                EvalLayer::TokenizerQuality,
                EvalLayer::ModelEval,
                EvalLayer::LinguisticAudit,
            ],
            tasks: vec![
                EvalTask {
                    target_language: "kazakh".to_string(),
                    name: "kazakh-token-efficiency".to_string(),
                    kind: EvalTaskKind::TokenEfficiency,
                    source_manifest: "data/eval/benchmark_manifest.json".to_string(),
                    dataset_manifest: "data/eval/kazakh_foundation_eval_dataset.json".to_string(),
                },
                EvalTask {
                    target_language: "kazakh".to_string(),
                    name: "kazakh-tokenizer-segmentation".to_string(),
                    kind: EvalTaskKind::TokenizerSegmentation,
                    source_manifest: "data/eval/benchmark_manifest.json".to_string(),
                    dataset_manifest: "data/eval/tokenizer_segmentation_eval_dataset.json"
                        .to_string(),
                },
                EvalTask {
                    target_language: "kazakh".to_string(),
                    name: "kazakh-morphology-sensitivity".to_string(),
                    kind: EvalTaskKind::MorphologySensitivity,
                    source_manifest: "data/eval/benchmark_manifest.json".to_string(),
                    dataset_manifest: "data/eval/kazakh_foundation_eval_dataset.json".to_string(),
                },
                EvalTask {
                    target_language: "kazakh".to_string(),
                    name: "kazakh-hallucination-audit".to_string(),
                    kind: EvalTaskKind::HallucinationAudit,
                    source_manifest: "data/eval/benchmark_manifest.json".to_string(),
                    dataset_manifest: "data/eval/kazakh_foundation_eval_dataset.json".to_string(),
                },
            ],
        }
    }
}

impl EvalSuite {
    pub fn validate(&self) -> Result<(), EvalError> {
        if self.target_language != "kazakh"
            || self
                .tasks
                .iter()
                .any(|task| task.target_language != "kazakh")
        {
            return Err(EvalError::NonKazakhLanguage);
        }

        if self.tasks.is_empty() {
            return Err(EvalError::EmptyTasks);
        }

        if self
            .tasks
            .iter()
            .any(|task| task.dataset_manifest.trim().is_empty())
        {
            return Err(EvalError::EmptyDatasetManifest);
        }

        Ok(())
    }
}

impl EvalDataset {
    pub fn validate(&self) -> Result<(), EvalError> {
        if self.target_language != "kazakh" {
            return Err(EvalError::NonKazakhLanguage);
        }

        if self.script != "cyrillic" {
            return Err(EvalError::NonCyrillicScript);
        }

        if self.entries.is_empty() {
            return Err(EvalError::EmptyEntries);
        }

        for entry in &self.entries {
            if contains_latin(&entry.prompt)
                || entry
                    .reference_answer
                    .as_ref()
                    .is_some_and(|value| contains_latin(value))
            {
                return Err(EvalError::LatinCharactersForbidden);
            }
        }

        Ok(())
    }
}

pub fn build_eval_benchmark_report(suite: &EvalSuite) -> Result<EvalBenchmarkReport, EvalError> {
    suite.validate()?;

    let mut category_breakdown = suite
        .tasks
        .iter()
        .fold(
            std::collections::BTreeMap::<String, usize>::new(),
            |mut acc, task| {
                *acc.entry(task_kind_slug(&task.kind).to_string())
                    .or_default() += 1;
                acc
            },
        )
        .into_iter()
        .map(|(category, task_count)| EvalBenchmarkCategoryReport {
            category,
            task_count,
        })
        .collect::<Vec<_>>();
    category_breakdown.sort_by(|left, right| left.category.cmp(&right.category));

    let mut critical_breakdown = suite
        .tasks
        .iter()
        .flat_map(|task| benchmark_guards_for_task(task))
        .fold(
            std::collections::BTreeMap::<String, usize>::new(),
            |mut acc, guard| {
                *acc.entry(guard).or_default() += 1;
                acc
            },
        )
        .into_iter()
        .map(|(guard, task_count)| EvalBenchmarkGuardReport { guard, task_count })
        .collect::<Vec<_>>();
    critical_breakdown.sort_by(|left, right| left.guard.cmp(&right.guard));

    Ok(EvalBenchmarkReport {
        suite_name: suite.name.clone(),
        target_language: suite.target_language.clone(),
        layer_count: suite.layers.len(),
        task_count: suite.tasks.len(),
        category_breakdown,
        critical_breakdown,
    })
}

pub fn build_eval_benchmark_delta_report(
    suite: &EvalSuite,
    expected: &EvalBenchmarkReport,
) -> Result<EvalBenchmarkDeltaReport, EvalError> {
    let actual = build_eval_benchmark_report(suite)?;

    Ok(EvalBenchmarkDeltaReport {
        suite_name: suite.name.clone(),
        matches_expected: expected == &actual,
        field_drifts: build_benchmark_field_drifts(expected, &actual),
        category_drifts: build_named_count_drifts(
            "category",
            expected
                .category_breakdown
                .iter()
                .map(|entry| (entry.category.as_str(), entry.task_count as u64))
                .collect(),
            actual
                .category_breakdown
                .iter()
                .map(|entry| (entry.category.as_str(), entry.task_count as u64))
                .collect(),
        ),
        guard_drifts: build_named_count_drifts(
            "guard",
            expected
                .critical_breakdown
                .iter()
                .map(|entry| (entry.guard.as_str(), entry.task_count as u64))
                .collect(),
            actual
                .critical_breakdown
                .iter()
                .map(|entry| (entry.guard.as_str(), entry.task_count as u64))
                .collect(),
        ),
    })
}

fn contains_latin(value: &str) -> bool {
    value.chars().any(|ch| ch.is_ascii_alphabetic())
}

fn build_benchmark_field_drifts(
    expected: &EvalBenchmarkReport,
    actual: &EvalBenchmarkReport,
) -> Vec<EvalBenchmarkFieldDrift> {
    let mut drifts = Vec::new();
    push_benchmark_field_drift(
        &mut drifts,
        "layer_count",
        expected.layer_count,
        actual.layer_count,
    );
    push_benchmark_field_drift(
        &mut drifts,
        "task_count",
        expected.task_count,
        actual.task_count,
    );
    drifts
}

fn push_benchmark_field_drift<T: ToString + PartialEq>(
    drifts: &mut Vec<EvalBenchmarkFieldDrift>,
    field: &str,
    expected: T,
    actual: T,
) {
    if expected != actual {
        drifts.push(EvalBenchmarkFieldDrift {
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
) -> Vec<EvalBenchmarkNamedCountDrift> {
    let mut expected_map = expected
        .into_iter()
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut actual_map = actual
        .into_iter()
        .collect::<std::collections::BTreeMap<_, _>>();
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
            drifts.push(EvalBenchmarkNamedCountDrift {
                scope: scope.to_string(),
                key: key.to_string(),
                expected: expected_value,
                actual: actual_value,
            });
        }
    }
    drifts
}

fn task_kind_slug(kind: &EvalTaskKind) -> &'static str {
    match kind {
        EvalTaskKind::TokenEfficiency => "token_efficiency",
        EvalTaskKind::TokenizerSegmentation => "tokenizer_segmentation",
        EvalTaskKind::NextTokenPrediction => "next_token_prediction",
        EvalTaskKind::ReadingComprehension => "reading_comprehension",
        EvalTaskKind::MorphologySensitivity => "morphology_sensitivity",
        EvalTaskKind::HallucinationAudit => "hallucination_audit",
    }
}

fn benchmark_guards_for_task(task: &EvalTask) -> Vec<String> {
    let mut guards = vec!["full_suite_coverage".to_string()];

    match task.kind {
        EvalTaskKind::TokenEfficiency => {
            guards.push("corpus_quality_task_family".to_string());
            guards.push("deterministic_efficiency_guard".to_string());
        }
        EvalTaskKind::TokenizerSegmentation => {
            guards.push("tokenizer_quality_task_family".to_string());
            guards.push("deterministic_segmentation_guard".to_string());
        }
        EvalTaskKind::NextTokenPrediction | EvalTaskKind::ReadingComprehension => {
            guards.push("model_eval_task_family".to_string());
        }
        EvalTaskKind::MorphologySensitivity => {
            guards.push("linguistic_audit_task_family".to_string());
            guards.push("morphology_guard".to_string());
        }
        EvalTaskKind::HallucinationAudit => {
            guards.push("linguistic_audit_task_family".to_string());
            guards.push("hallucination_guard".to_string());
        }
    }

    guards
}

#[cfg(test)]
mod tests {
    use super::{
        EvalBenchmarkReport, EvalDataset, EvalError, EvalSuite, build_eval_benchmark_delta_report,
        build_eval_benchmark_report,
    };

    #[test]
    fn default_eval_suite_targets_kazakh() {
        let suite = EvalSuite::default();

        assert_eq!(suite.target_language, "kazakh");
        assert_eq!(suite.version, "1.3.0");
        assert_eq!(suite.layers.len(), 4);
        assert_eq!(suite.tasks.len(), 4);
        assert!(suite.validate().is_ok());
    }

    #[test]
    fn rejects_non_kazakh_tasks() {
        let mut suite = EvalSuite::default();
        suite.tasks[0].target_language = "mixed".to_string();

        assert_eq!(suite.validate(), Err(EvalError::NonKazakhLanguage));
    }

    #[test]
    fn dataset_rejects_latin_text() {
        let mut dataset = EvalDataset {
            version: "1.3.0".to_string(),
            name: "test".to_string(),
            target_language: "kazakh".to_string(),
            script: "cyrillic".to_string(),
            entries: vec![super::EvalExample {
                id: "ex_01".to_string(),
                split: super::EvalSplit::Dev,
                kind: super::EvalTaskKind::ReadingComprehension,
                prompt: "Hello".to_string(),
                reference_answer: Some("Жауап".to_string()),
                must_answer_in_kazakh: true,
                must_avoid_fabrication: true,
            }],
        };

        assert_eq!(dataset.validate(), Err(EvalError::LatinCharactersForbidden));

        dataset.entries[0].prompt = "Сәлем".to_string();
        assert!(dataset.validate().is_ok());
    }

    #[test]
    fn builds_eval_benchmark_report_with_task_family_breakdown() {
        let suite = EvalSuite::default();

        let report = build_eval_benchmark_report(&suite).expect("benchmark report");

        assert_eq!(report.task_count, 4);
        assert_eq!(report.layer_count, 4);
        assert!(
            report
                .category_breakdown
                .iter()
                .any(|entry| entry.category == "tokenizer_segmentation" && entry.task_count == 1)
        );
        assert!(report
            .critical_breakdown
            .iter()
            .any(|entry| entry.guard == "linguistic_audit_task_family" && entry.task_count == 2));
    }

    #[test]
    fn builds_eval_benchmark_delta_report_without_drift() {
        let suite = EvalSuite::default();
        let expected = EvalBenchmarkReport {
            suite_name: "kazakh-foundation-baseline".to_string(),
            target_language: "kazakh".to_string(),
            layer_count: 4,
            task_count: 4,
            category_breakdown: vec![
                super::EvalBenchmarkCategoryReport {
                    category: "hallucination_audit".to_string(),
                    task_count: 1,
                },
                super::EvalBenchmarkCategoryReport {
                    category: "morphology_sensitivity".to_string(),
                    task_count: 1,
                },
                super::EvalBenchmarkCategoryReport {
                    category: "token_efficiency".to_string(),
                    task_count: 1,
                },
                super::EvalBenchmarkCategoryReport {
                    category: "tokenizer_segmentation".to_string(),
                    task_count: 1,
                },
            ],
            critical_breakdown: vec![
                super::EvalBenchmarkGuardReport {
                    guard: "corpus_quality_task_family".to_string(),
                    task_count: 1,
                },
                super::EvalBenchmarkGuardReport {
                    guard: "deterministic_efficiency_guard".to_string(),
                    task_count: 1,
                },
                super::EvalBenchmarkGuardReport {
                    guard: "deterministic_segmentation_guard".to_string(),
                    task_count: 1,
                },
                super::EvalBenchmarkGuardReport {
                    guard: "full_suite_coverage".to_string(),
                    task_count: 4,
                },
                super::EvalBenchmarkGuardReport {
                    guard: "hallucination_guard".to_string(),
                    task_count: 1,
                },
                super::EvalBenchmarkGuardReport {
                    guard: "linguistic_audit_task_family".to_string(),
                    task_count: 2,
                },
                super::EvalBenchmarkGuardReport {
                    guard: "morphology_guard".to_string(),
                    task_count: 1,
                },
                super::EvalBenchmarkGuardReport {
                    guard: "tokenizer_quality_task_family".to_string(),
                    task_count: 1,
                },
            ],
        };

        let delta = build_eval_benchmark_delta_report(&suite, &expected).expect("delta report");

        assert!(delta.matches_expected);
        assert!(delta.field_drifts.is_empty());
        assert!(delta.category_drifts.is_empty());
        assert!(delta.guard_drifts.is_empty());
    }
}
