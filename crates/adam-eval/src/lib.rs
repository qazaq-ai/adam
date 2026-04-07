use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalSuite {
    pub name: String,
    pub target_language: String,
    pub layers: Vec<String>,
}

impl Default for EvalSuite {
    fn default() -> Self {
        Self {
            name: "kazakh-foundation-baseline".to_string(),
            target_language: "kazakh".to_string(),
            layers: vec![
                "corpus_quality".to_string(),
                "tokenizer_quality".to_string(),
                "model_eval".to_string(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::EvalSuite;

    #[test]
    fn default_eval_suite_targets_kazakh() {
        let suite = EvalSuite::default();

        assert_eq!(suite.target_language, "kazakh");
        assert_eq!(suite.layers.len(), 3);
    }
}
