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
            version: "0.0.32".to_string(),
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

fn contains_latin(value: &str) -> bool {
    value.chars().any(|ch| ch.is_ascii_alphabetic())
}

#[cfg(test)]
mod tests {
    use super::{EvalDataset, EvalError, EvalSuite};

    #[test]
    fn default_eval_suite_targets_kazakh() {
        let suite = EvalSuite::default();

        assert_eq!(suite.target_language, "kazakh");
        assert_eq!(suite.version, "0.0.32");
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
            version: "0.0.32".to_string(),
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
}
