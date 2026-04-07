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
    #[error("evaluation tasks must not be empty")]
    EmptyTasks,
}

impl Default for EvalSuite {
    fn default() -> Self {
        Self {
            version: "0.0.1".to_string(),
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
                },
                EvalTask {
                    target_language: "kazakh".to_string(),
                    name: "kazakh-morphology-sensitivity".to_string(),
                    kind: EvalTaskKind::MorphologySensitivity,
                    source_manifest: "data/eval/benchmark_manifest.json".to_string(),
                },
                EvalTask {
                    target_language: "kazakh".to_string(),
                    name: "kazakh-hallucination-audit".to_string(),
                    kind: EvalTaskKind::HallucinationAudit,
                    source_manifest: "data/eval/benchmark_manifest.json".to_string(),
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

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{EvalError, EvalSuite};

    #[test]
    fn default_eval_suite_targets_kazakh() {
        let suite = EvalSuite::default();

        assert_eq!(suite.target_language, "kazakh");
        assert_eq!(suite.version, "0.0.1");
        assert_eq!(suite.layers.len(), 4);
        assert_eq!(suite.tasks.len(), 3);
        assert!(suite.validate().is_ok());
    }

    #[test]
    fn rejects_non_kazakh_tasks() {
        let mut suite = EvalSuite::default();
        suite.tasks[0].target_language = "mixed".to_string();

        assert_eq!(suite.validate(), Err(EvalError::NonKazakhLanguage));
    }
}
