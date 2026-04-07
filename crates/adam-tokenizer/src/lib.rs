use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrainingTarget {
    KazakhOnly,
    KazakhPrimary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NormalizerPolicy {
    LowercaseTrim,
    LowercaseTrimPreserveCyrillic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenizerExperiment {
    pub version: String,
    pub name: String,
    pub target_language: String,
    pub script: String,
    pub profile_name: String,
    pub training_manifest: String,
    pub objective: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenizerProfile {
    pub name: String,
    pub language: String,
    pub script: String,
    pub strategy: String,
    pub training_target: TrainingTarget,
    pub normalizer: NormalizerPolicy,
    pub special_tokens: Vec<String>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TokenizerError {
    #[error("tokenizer language must be kazakh")]
    NonKazakhLanguage,
    #[error("tokenizer script must be cyrillic")]
    NonCyrillicScript,
    #[error("special tokens must not be empty")]
    EmptySpecialTokens,
    #[error("tokenizer objective must not be empty")]
    EmptyObjective,
}

impl Default for TokenizerProfile {
    fn default() -> Self {
        Self {
            name: "adam-kazakh-cyrillic".to_string(),
            language: "kazakh".to_string(),
            script: "cyrillic".to_string(),
            strategy: "train_kazakh_first_tokenizer".to_string(),
            training_target: TrainingTarget::KazakhOnly,
            normalizer: NormalizerPolicy::LowercaseTrimPreserveCyrillic,
            special_tokens: vec![
                "<pad>".to_string(),
                "<bos>".to_string(),
                "<eos>".to_string(),
                "<unk>".to_string(),
            ],
        }
    }
}

impl TokenizerProfile {
    pub fn validate(&self) -> Result<(), TokenizerError> {
        if self.language != "kazakh" {
            return Err(TokenizerError::NonKazakhLanguage);
        }

        if self.script != "cyrillic" {
            return Err(TokenizerError::NonCyrillicScript);
        }

        if self.special_tokens.is_empty() {
            return Err(TokenizerError::EmptySpecialTokens);
        }

        Ok(())
    }
}

impl TokenizerExperiment {
    pub fn validate(&self) -> Result<(), TokenizerError> {
        if self.target_language != "kazakh" {
            return Err(TokenizerError::NonKazakhLanguage);
        }

        if self.script != "cyrillic" {
            return Err(TokenizerError::NonCyrillicScript);
        }

        if self.objective.trim().is_empty() {
            return Err(TokenizerError::EmptyObjective);
        }

        Ok(())
    }
}

pub fn normalize_text(text: &str) -> String {
    text.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::{TokenizerError, TokenizerExperiment, TokenizerProfile, normalize_text};

    #[test]
    fn normalizes_basic_input() {
        assert_eq!(normalize_text(" Сәлем "), "сәлем");
    }

    #[test]
    fn default_profile_is_kazakh_cyrillic() {
        let profile = TokenizerProfile::default();

        assert_eq!(profile.language, "kazakh");
        assert_eq!(profile.script, "cyrillic");
        assert_eq!(profile.special_tokens.len(), 4);
        assert!(profile.validate().is_ok());
    }

    #[test]
    fn rejects_non_cyrillic_profile() {
        let mut profile = TokenizerProfile::default();
        profile.script = "latin".to_string();

        assert_eq!(profile.validate(), Err(TokenizerError::NonCyrillicScript));
    }

    #[test]
    fn accepts_kazakh_tokenizer_experiment() {
        let experiment = TokenizerExperiment {
            version: "0.0.1".to_string(),
            name: "adam-tokenizer-baseline".to_string(),
            target_language: "kazakh".to_string(),
            script: "cyrillic".to_string(),
            profile_name: "adam-kazakh-cyrillic".to_string(),
            training_manifest: "data/curated/corpus_manifest.json".to_string(),
            objective: "measure token efficiency on kazakh text".to_string(),
        };

        assert!(experiment.validate().is_ok());
    }
}
