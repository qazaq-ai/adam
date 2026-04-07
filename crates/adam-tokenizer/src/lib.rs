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
    pub sample_pack_manifest: String,
    pub objective: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenizerDryRunSample {
    pub id: String,
    pub text: String,
    pub domain: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenizerDryRunPack {
    pub version: String,
    pub name: String,
    pub target_language: String,
    pub script: String,
    pub samples: Vec<TokenizerDryRunSample>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenizerDryRunReport {
    pub experiment_name: String,
    pub sample_count: usize,
    pub normalized_nonempty_count: usize,
    pub total_character_count: usize,
    pub average_character_count: usize,
    pub domains: Vec<String>,
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
    #[error("sample pack manifest path must not be empty")]
    EmptySamplePackManifest,
    #[error("dry-run sample pack must not be empty")]
    EmptySamplePack,
    #[error("latin characters are not allowed in kazakh-only tokenizer data")]
    LatinCharactersForbidden,
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

        if self.sample_pack_manifest.trim().is_empty() {
            return Err(TokenizerError::EmptySamplePackManifest);
        }

        Ok(())
    }
}

impl TokenizerDryRunPack {
    pub fn validate(&self) -> Result<(), TokenizerError> {
        if self.target_language != "kazakh" {
            return Err(TokenizerError::NonKazakhLanguage);
        }

        if self.script != "cyrillic" {
            return Err(TokenizerError::NonCyrillicScript);
        }

        if self.samples.is_empty() {
            return Err(TokenizerError::EmptySamplePack);
        }

        if self
            .samples
            .iter()
            .any(|sample| contains_latin(&sample.text))
        {
            return Err(TokenizerError::LatinCharactersForbidden);
        }

        Ok(())
    }
}

pub fn build_dry_run_report(
    experiment: &TokenizerExperiment,
    pack: &TokenizerDryRunPack,
) -> Result<TokenizerDryRunReport, TokenizerError> {
    experiment.validate()?;
    pack.validate()?;

    let normalized_samples = pack
        .samples
        .iter()
        .map(|sample| normalize_text(&sample.text))
        .collect::<Vec<_>>();
    let total_character_count = normalized_samples
        .iter()
        .map(|text| text.chars().count())
        .sum();
    let sample_count = normalized_samples.len();
    let normalized_nonempty_count = normalized_samples
        .iter()
        .filter(|text| !text.is_empty())
        .count();
    let average_character_count = if sample_count == 0 {
        0
    } else {
        total_character_count / sample_count
    };
    let mut domains = pack
        .samples
        .iter()
        .map(|sample| sample.domain.clone())
        .collect::<Vec<_>>();
    domains.sort();
    domains.dedup();

    Ok(TokenizerDryRunReport {
        experiment_name: experiment.name.clone(),
        sample_count,
        normalized_nonempty_count,
        total_character_count,
        average_character_count,
        domains,
    })
}

fn contains_latin(value: &str) -> bool {
    value.chars().any(|ch| ch.is_ascii_alphabetic())
}

pub fn normalize_text(text: &str) -> String {
    text.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::{
        TokenizerDryRunPack, TokenizerDryRunSample, TokenizerError, TokenizerExperiment,
        TokenizerProfile, build_dry_run_report, normalize_text,
    };

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
            sample_pack_manifest: "data/curated/tokenizer_dry_run_pack.json".to_string(),
            objective: "measure token efficiency on kazakh text".to_string(),
        };

        assert!(experiment.validate().is_ok());
    }

    #[test]
    fn builds_dry_run_report() {
        let experiment = TokenizerExperiment {
            version: "0.0.1".to_string(),
            name: "adam-tokenizer-baseline".to_string(),
            target_language: "kazakh".to_string(),
            script: "cyrillic".to_string(),
            profile_name: "adam-kazakh-cyrillic".to_string(),
            training_manifest: "data/curated/corpus_manifest.json".to_string(),
            sample_pack_manifest: "data/curated/tokenizer_dry_run_pack.json".to_string(),
            objective: "measure token efficiency on kazakh text".to_string(),
        };
        let pack = TokenizerDryRunPack {
            version: "0.0.1".to_string(),
            name: "adam-tokenizer-dry-run".to_string(),
            target_language: "kazakh".to_string(),
            script: "cyrillic".to_string(),
            samples: vec![
                TokenizerDryRunSample {
                    id: "sample_01".to_string(),
                    text: "Анықтама алу үшін өтініш жазылады.".to_string(),
                    domain: "administrative".to_string(),
                },
                TokenizerDryRunSample {
                    id: "sample_02".to_string(),
                    text: "Қазақ тілі агглютинативті тіл.".to_string(),
                    domain: "general".to_string(),
                },
            ],
        };

        let report = build_dry_run_report(&experiment, &pack).expect("dry-run report");

        assert_eq!(report.experiment_name, "adam-tokenizer-baseline");
        assert_eq!(report.sample_count, 2);
        assert_eq!(report.normalized_nonempty_count, 2);
        assert_eq!(report.domains.len(), 2);
    }
}
