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
    pub segmentation_eval_manifest: String,
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
pub struct TokenizerSegmentationExample {
    pub id: String,
    pub token: String,
    pub expected_segments: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenizerSegmentationDataset {
    pub version: String,
    pub name: String,
    pub target_language: String,
    pub script: String,
    pub profile_name: String,
    pub entries: Vec<TokenizerSegmentationExample>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenizerSegmentationReport {
    pub dataset_name: String,
    pub example_count: usize,
    pub average_segment_count: usize,
    pub longest_token_length: usize,
    pub exact_match_count: usize,
    pub exact_match_rate_bps: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenizerSegmentationFailure {
    pub id: String,
    pub token: String,
    pub expected_segments: Vec<String>,
    pub predicted_segments: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenizerExperimentReport {
    pub experiment_name: String,
    pub sample_count: usize,
    pub normalized_nonempty_count: usize,
    pub total_character_count: usize,
    pub average_character_count: usize,
    pub domains: Vec<String>,
    pub segmentation_dataset_name: String,
    pub segmentation_example_count: usize,
    pub exact_match_count: usize,
    pub exact_match_rate_bps: usize,
    pub failures: Vec<TokenizerSegmentationFailure>,
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
    #[error("segmentation eval manifest path must not be empty")]
    EmptySegmentationEvalManifest,
    #[error("dry-run sample pack must not be empty")]
    EmptySamplePack,
    #[error("segmentation dataset entries must not be empty")]
    EmptySegmentationDataset,
    #[error("segmentation examples must include at least one expected segment")]
    EmptySegmentationSegments,
    #[error("segmentation examples must preserve the original token")]
    SegmentationTokenMismatch,
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

        if self.segmentation_eval_manifest.trim().is_empty() {
            return Err(TokenizerError::EmptySegmentationEvalManifest);
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

impl TokenizerSegmentationDataset {
    pub fn validate(&self) -> Result<(), TokenizerError> {
        if self.target_language != "kazakh" {
            return Err(TokenizerError::NonKazakhLanguage);
        }

        if self.script != "cyrillic" {
            return Err(TokenizerError::NonCyrillicScript);
        }

        if self.entries.is_empty() {
            return Err(TokenizerError::EmptySegmentationDataset);
        }

        for entry in &self.entries {
            if entry.expected_segments.is_empty() {
                return Err(TokenizerError::EmptySegmentationSegments);
            }

            if contains_latin(&entry.token)
                || entry
                    .expected_segments
                    .iter()
                    .any(|segment| contains_latin(segment))
            {
                return Err(TokenizerError::LatinCharactersForbidden);
            }

            if entry.expected_segments.concat() != entry.token {
                return Err(TokenizerError::SegmentationTokenMismatch);
            }
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

pub fn build_segmentation_report(
    dataset: &TokenizerSegmentationDataset,
) -> Result<TokenizerSegmentationReport, TokenizerError> {
    dataset.validate()?;

    let example_count = dataset.entries.len();
    let total_segment_count = dataset
        .entries
        .iter()
        .map(|entry| entry.expected_segments.len())
        .sum::<usize>();
    let average_segment_count = total_segment_count / example_count;
    let longest_token_length = dataset
        .entries
        .iter()
        .map(|entry| entry.token.chars().count())
        .max()
        .unwrap_or(0);
    let exact_match_count = dataset
        .entries
        .iter()
        .filter(|entry| baseline_segment_token(&entry.token) == entry.expected_segments)
        .count();
    let exact_match_rate_bps = exact_match_count * 10_000 / example_count;

    Ok(TokenizerSegmentationReport {
        dataset_name: dataset.name.clone(),
        example_count,
        average_segment_count,
        longest_token_length,
        exact_match_count,
        exact_match_rate_bps,
    })
}

pub fn build_experiment_report(
    experiment: &TokenizerExperiment,
    pack: &TokenizerDryRunPack,
    dataset: &TokenizerSegmentationDataset,
) -> Result<TokenizerExperimentReport, TokenizerError> {
    let dry_run_report = build_dry_run_report(experiment, pack)?;
    dataset.validate()?;

    let failures = dataset
        .entries
        .iter()
        .filter_map(|entry| {
            let predicted_segments = baseline_segment_token(&entry.token);
            if predicted_segments == entry.expected_segments {
                None
            } else {
                Some(TokenizerSegmentationFailure {
                    id: entry.id.clone(),
                    token: entry.token.clone(),
                    expected_segments: entry.expected_segments.clone(),
                    predicted_segments,
                })
            }
        })
        .collect::<Vec<_>>();
    let exact_match_count = dataset.entries.len() - failures.len();
    let exact_match_rate_bps = exact_match_count * 10_000 / dataset.entries.len();

    Ok(TokenizerExperimentReport {
        experiment_name: experiment.name.clone(),
        sample_count: dry_run_report.sample_count,
        normalized_nonempty_count: dry_run_report.normalized_nonempty_count,
        total_character_count: dry_run_report.total_character_count,
        average_character_count: dry_run_report.average_character_count,
        domains: dry_run_report.domains,
        segmentation_dataset_name: dataset.name.clone(),
        segmentation_example_count: dataset.entries.len(),
        exact_match_count,
        exact_match_rate_bps,
        failures,
    })
}

pub fn baseline_segment_token(token: &str) -> Vec<String> {
    const SUFFIXES: &[&str] = &[
        "ыңдар",
        "іңдер",
        "ңыздар",
        "ңіздер",
        "лардың",
        "лердің",
        "дардың",
        "дердің",
        "тардың",
        "тердің",
        "лар",
        "лер",
        "дар",
        "дер",
        "тар",
        "тер",
        "дан",
        "ден",
        "тан",
        "тен",
        "нан",
        "нен",
        "ның",
        "нің",
        "дың",
        "дің",
        "тың",
        "тің",
        "ды",
        "ді",
        "ты",
        "ті",
        "ны",
        "ні",
        "ға",
        "ге",
        "қа",
        "ке",
        "да",
        "де",
        "та",
        "те",
        "ла",
        "ле",
        "л",
    ];

    let mut remaining = token.to_string();
    let mut suffixes = Vec::new();

    loop {
        let Some(matched) = SUFFIXES.iter().find(|suffix| {
            remaining.ends_with(**suffix) && remaining.chars().count() > suffix.chars().count()
        }) else {
            break;
        };

        let split_at = remaining.len() - matched.len();
        let stem = remaining[..split_at].to_string();
        if stem.is_empty() {
            break;
        }

        suffixes.push((*matched).to_string());
        remaining = stem;
    }

    suffixes.reverse();
    let mut segments = vec![remaining];
    segments.extend(suffixes);
    segments
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
        TokenizerProfile, TokenizerSegmentationDataset, TokenizerSegmentationExample,
        baseline_segment_token, build_dry_run_report, build_experiment_report,
        build_segmentation_report, normalize_text,
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
            version: "0.0.3".to_string(),
            name: "adam-tokenizer-baseline".to_string(),
            target_language: "kazakh".to_string(),
            script: "cyrillic".to_string(),
            profile_name: "adam-kazakh-cyrillic".to_string(),
            training_manifest: "data/curated/corpus_manifest.json".to_string(),
            sample_pack_manifest: "data/curated/tokenizer_dry_run_pack.json".to_string(),
            segmentation_eval_manifest: "data/eval/tokenizer_segmentation_eval_dataset.json"
                .to_string(),
            objective: "measure token efficiency on kazakh text".to_string(),
        };

        assert!(experiment.validate().is_ok());
    }

    #[test]
    fn builds_dry_run_report() {
        let experiment = TokenizerExperiment {
            version: "0.0.3".to_string(),
            name: "adam-tokenizer-baseline".to_string(),
            target_language: "kazakh".to_string(),
            script: "cyrillic".to_string(),
            profile_name: "adam-kazakh-cyrillic".to_string(),
            training_manifest: "data/curated/corpus_manifest.json".to_string(),
            sample_pack_manifest: "data/curated/tokenizer_dry_run_pack.json".to_string(),
            segmentation_eval_manifest: "data/eval/tokenizer_segmentation_eval_dataset.json"
                .to_string(),
            objective: "measure token efficiency on kazakh text".to_string(),
        };
        let pack = TokenizerDryRunPack {
            version: "0.0.3".to_string(),
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

    #[test]
    fn validates_segmentation_dataset_and_builds_report() {
        let dataset = TokenizerSegmentationDataset {
            version: "0.0.3".to_string(),
            name: "adam-tokenizer-segmentation".to_string(),
            target_language: "kazakh".to_string(),
            script: "cyrillic".to_string(),
            profile_name: "adam-kazakh-cyrillic".to_string(),
            entries: vec![
                TokenizerSegmentationExample {
                    id: "seg_01".to_string(),
                    token: "мекемеден".to_string(),
                    expected_segments: vec!["мекеме".to_string(), "ден".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_02".to_string(),
                    token: "келді".to_string(),
                    expected_segments: vec!["кел".to_string(), "ді".to_string()],
                },
            ],
        };

        let report = build_segmentation_report(&dataset).expect("segmentation report");
        assert_eq!(report.example_count, 2);
        assert_eq!(report.average_segment_count, 2);
        assert_eq!(report.longest_token_length, "мекемеден".chars().count());
        assert_eq!(report.exact_match_count, 2);
        assert_eq!(report.exact_match_rate_bps, 10_000);
    }

    #[test]
    fn rejects_segmentation_dataset_with_mismatched_segments() {
        let dataset = TokenizerSegmentationDataset {
            version: "0.0.3".to_string(),
            name: "adam-tokenizer-segmentation".to_string(),
            target_language: "kazakh".to_string(),
            script: "cyrillic".to_string(),
            profile_name: "adam-kazakh-cyrillic".to_string(),
            entries: vec![TokenizerSegmentationExample {
                id: "seg_01".to_string(),
                token: "келді".to_string(),
                expected_segments: vec!["ке".to_string(), "л".to_string()],
            }],
        };

        assert_eq!(
            dataset.validate(),
            Err(TokenizerError::SegmentationTokenMismatch)
        );
    }

    #[test]
    fn baseline_segmenter_handles_core_kazakh_examples() {
        assert_eq!(
            baseline_segment_token("мекемеден"),
            vec!["мекеме".to_string(), "ден".to_string()]
        );
        assert_eq!(
            baseline_segment_token("келді"),
            vec!["кел".to_string(), "ді".to_string()]
        );
        assert_eq!(
            baseline_segment_token("қаралды"),
            vec!["қара".to_string(), "л".to_string(), "ды".to_string()]
        );
    }

    #[test]
    fn builds_experiment_report_with_segmentation_scoring() {
        let experiment = TokenizerExperiment {
            version: "0.0.3".to_string(),
            name: "adam-tokenizer-baseline".to_string(),
            target_language: "kazakh".to_string(),
            script: "cyrillic".to_string(),
            profile_name: "adam-kazakh-cyrillic".to_string(),
            training_manifest: "data/curated/corpus_manifest.json".to_string(),
            sample_pack_manifest: "data/curated/tokenizer_dry_run_pack.json".to_string(),
            segmentation_eval_manifest: "data/eval/tokenizer_segmentation_eval_dataset.json"
                .to_string(),
            objective: "measure token efficiency on kazakh text".to_string(),
        };
        let pack = TokenizerDryRunPack {
            version: "0.0.3".to_string(),
            name: "adam-tokenizer-dry-run".to_string(),
            target_language: "kazakh".to_string(),
            script: "cyrillic".to_string(),
            samples: vec![TokenizerDryRunSample {
                id: "sample_01".to_string(),
                text: "Мекемеден құжат алдым.".to_string(),
                domain: "administrative".to_string(),
            }],
        };
        let dataset = TokenizerSegmentationDataset {
            version: "0.0.3".to_string(),
            name: "adam-tokenizer-segmentation".to_string(),
            target_language: "kazakh".to_string(),
            script: "cyrillic".to_string(),
            profile_name: "adam-kazakh-cyrillic".to_string(),
            entries: vec![
                TokenizerSegmentationExample {
                    id: "seg_01".to_string(),
                    token: "мекемеден".to_string(),
                    expected_segments: vec!["мекеме".to_string(), "ден".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_02".to_string(),
                    token: "келді".to_string(),
                    expected_segments: vec!["кел".to_string(), "ді".to_string()],
                },
            ],
        };

        let report =
            build_experiment_report(&experiment, &pack, &dataset).expect("experiment report");
        assert_eq!(report.sample_count, 1);
        assert_eq!(report.segmentation_example_count, 2);
        assert_eq!(report.exact_match_count, 2);
        assert!(report.failures.is_empty());
    }
}
