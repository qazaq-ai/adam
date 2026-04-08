use std::collections::HashSet;

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
    pub segmentation_roots_manifest: String,
    pub segmentation_rules_manifest: String,
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
#[serde(rename_all = "snake_case")]
pub enum SegmentationPartOfSpeech {
    Noun,
    Pronoun,
    Verb,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VowelHarmony {
    Front,
    Back,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinalSoundClass {
    Vowel,
    VoicedConsonant,
    VoicelessConsonant,
    Nasal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentationState {
    Stem,
    Number,
    Possessive,
    Voice,
    Tense,
    Person,
    Case,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SegmentationRootEntry {
    pub id: String,
    pub root: String,
    pub part_of_speech: SegmentationPartOfSpeech,
    pub vowel_harmony: VowelHarmony,
    pub final_sound_class: FinalSoundClass,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SegmentationLexicon {
    pub version: String,
    pub name: String,
    pub target_language: String,
    pub script: String,
    pub roots: Vec<SegmentationRootEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SegmentationSuffixRule {
    pub id: String,
    pub form: String,
    pub part_of_speech: SegmentationPartOfSpeech,
    pub from_state: SegmentationState,
    pub to_state: SegmentationState,
    pub label: String,
    pub allowed_harmonies: Vec<VowelHarmony>,
    pub allowed_final_sound_classes: Vec<FinalSoundClass>,
    pub terminal: bool,
    #[serde(default)]
    pub allowed_previous_labels: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SegmentationRuleSet {
    pub version: String,
    pub name: String,
    pub target_language: String,
    pub script: String,
    pub rules: Vec<SegmentationSuffixRule>,
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
    #[error("segmentation roots manifest path must not be empty")]
    EmptySegmentationRootsManifest,
    #[error("segmentation rules manifest path must not be empty")]
    EmptySegmentationRulesManifest,
    #[error("dry-run sample pack must not be empty")]
    EmptySamplePack,
    #[error("segmentation dataset entries must not be empty")]
    EmptySegmentationDataset,
    #[error("segmentation lexicon roots must not be empty")]
    EmptySegmentationLexicon,
    #[error("segmentation rule set rules must not be empty")]
    EmptySegmentationRuleSet,
    #[error("segmentation root ids must be unique and non-empty")]
    InvalidSegmentationRootId,
    #[error("segmentation suffix rule ids must be unique and non-empty")]
    InvalidSegmentationRuleId,
    #[error("segmentation rule constraints must not be empty")]
    EmptySegmentationRuleConstraint,
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

        if self.segmentation_roots_manifest.trim().is_empty() {
            return Err(TokenizerError::EmptySegmentationRootsManifest);
        }

        if self.segmentation_rules_manifest.trim().is_empty() {
            return Err(TokenizerError::EmptySegmentationRulesManifest);
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

impl SegmentationLexicon {
    pub fn validate(&self) -> Result<(), TokenizerError> {
        if self.target_language != "kazakh" {
            return Err(TokenizerError::NonKazakhLanguage);
        }

        if self.script != "cyrillic" {
            return Err(TokenizerError::NonCyrillicScript);
        }

        if self.roots.is_empty() {
            return Err(TokenizerError::EmptySegmentationLexicon);
        }

        let mut seen = HashSet::new();
        for root in &self.roots {
            if root.id.trim().is_empty()
                || root.root.trim().is_empty()
                || contains_latin(&root.root)
                || !seen.insert(root.id.clone())
            {
                return Err(TokenizerError::InvalidSegmentationRootId);
            }
        }

        Ok(())
    }
}

impl SegmentationRuleSet {
    pub fn validate(&self) -> Result<(), TokenizerError> {
        if self.target_language != "kazakh" {
            return Err(TokenizerError::NonKazakhLanguage);
        }

        if self.script != "cyrillic" {
            return Err(TokenizerError::NonCyrillicScript);
        }

        if self.rules.is_empty() {
            return Err(TokenizerError::EmptySegmentationRuleSet);
        }

        let mut seen = HashSet::new();
        for rule in &self.rules {
            if rule.id.trim().is_empty()
                || rule.form.trim().is_empty()
                || rule.label.trim().is_empty()
                || contains_latin(&rule.form)
                || !seen.insert(rule.id.clone())
            {
                return Err(TokenizerError::InvalidSegmentationRuleId);
            }

            if rule.allowed_harmonies.is_empty() || rule.allowed_final_sound_classes.is_empty() {
                return Err(TokenizerError::EmptySegmentationRuleConstraint);
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
    lexicon: &SegmentationLexicon,
    rules: &SegmentationRuleSet,
) -> Result<TokenizerSegmentationReport, TokenizerError> {
    dataset.validate()?;
    lexicon.validate()?;
    rules.validate()?;

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
        .filter(|entry| {
            deterministic_segment_token(&entry.token, lexicon, rules).as_ref()
                == Some(&entry.expected_segments)
        })
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
    lexicon: &SegmentationLexicon,
    rules: &SegmentationRuleSet,
) -> Result<TokenizerExperimentReport, TokenizerError> {
    let dry_run_report = build_dry_run_report(experiment, pack)?;
    dataset.validate()?;
    lexicon.validate()?;
    rules.validate()?;

    let failures = dataset
        .entries
        .iter()
        .filter_map(|entry| {
            let predicted_segments = deterministic_segment_token(&entry.token, lexicon, rules);
            if predicted_segments.as_ref() == Some(&entry.expected_segments) {
                None
            } else {
                Some(TokenizerSegmentationFailure {
                    id: entry.id.clone(),
                    token: entry.token.clone(),
                    expected_segments: entry.expected_segments.clone(),
                    predicted_segments: predicted_segments.unwrap_or_default(),
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

pub fn deterministic_segment_token(
    token: &str,
    lexicon: &SegmentationLexicon,
    rules: &SegmentationRuleSet,
) -> Option<Vec<String>> {
    let mut candidates = lexicon
        .roots
        .iter()
        .filter(|root| token.starts_with(&root.root))
        .filter_map(|root| {
            let remaining = &token[root.root.len()..];
            if remaining.is_empty() {
                return Some(vec![root.root.clone()]);
            }

            let mut parses = Vec::new();
            let mut suffix_chain = Vec::new();
            collect_suffix_parses(
                remaining,
                &root.part_of_speech,
                SegmentationState::Stem,
                None,
                &root.vowel_harmony,
                &root.final_sound_class,
                rules,
                &mut suffix_chain,
                &mut parses,
            );

            if parses.len() == 1 {
                let mut segments = vec![root.root.clone()];
                segments.extend(parses.pop().expect("single parse"));
                Some(segments)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    candidates.sort();
    candidates.dedup();

    if candidates.len() == 1 {
        candidates.pop()
    } else {
        None
    }
}

fn collect_suffix_parses(
    remaining: &str,
    part_of_speech: &SegmentationPartOfSpeech,
    state: SegmentationState,
    previous_label: Option<&str>,
    harmony: &VowelHarmony,
    final_sound_class: &FinalSoundClass,
    rules: &SegmentationRuleSet,
    current: &mut Vec<String>,
    parses: &mut Vec<Vec<String>>,
) {
    for rule in rules.rules.iter().filter(|rule| {
        &rule.part_of_speech == part_of_speech
            && rule.from_state == state
            && (rule.allowed_previous_labels.is_empty()
                || previous_label.is_some_and(|label| {
                    rule.allowed_previous_labels
                        .iter()
                        .any(|allowed_label| allowed_label == label)
                }))
            && rule.allowed_harmonies.contains(harmony)
            && rule.allowed_final_sound_classes.contains(final_sound_class)
            && remaining.starts_with(&rule.form)
    }) {
        let next_remaining = &remaining[rule.form.len()..];
        current.push(rule.form.clone());
        let next_final_sound_class = classify_final_sound(&rule.form);

        if next_remaining.is_empty() {
            parses.push(current.clone());
        } else if !rule.terminal {
            collect_suffix_parses(
                next_remaining,
                part_of_speech,
                rule.to_state.clone(),
                Some(&rule.label),
                harmony,
                &next_final_sound_class,
                rules,
                current,
                parses,
            );
        }

        current.pop();
    }
}

fn classify_final_sound(form: &str) -> FinalSoundClass {
    let last = form
        .chars()
        .last()
        .expect("segmentation suffix forms must not be empty");

    if is_kazakh_vowel(last) {
        FinalSoundClass::Vowel
    } else if matches!(last, 'м' | 'н' | 'ң') {
        FinalSoundClass::Nasal
    } else if matches!(
        last,
        'п' | 'ф' | 'к' | 'қ' | 'т' | 'с' | 'ш' | 'щ' | 'ч' | 'ц' | 'х' | 'һ'
    ) {
        FinalSoundClass::VoicelessConsonant
    } else {
        FinalSoundClass::VoicedConsonant
    }
}

fn is_kazakh_vowel(ch: char) -> bool {
    matches!(
        ch,
        'а' | 'ә' | 'е' | 'и' | 'о' | 'ө' | 'ұ' | 'ү' | 'у' | 'ы' | 'і' | 'э'
    )
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
        FinalSoundClass, SegmentationLexicon, SegmentationPartOfSpeech, SegmentationRootEntry,
        SegmentationRuleSet, SegmentationState, SegmentationSuffixRule, TokenizerDryRunPack,
        TokenizerDryRunSample, TokenizerError, TokenizerExperiment, TokenizerProfile,
        TokenizerSegmentationDataset, TokenizerSegmentationExample, VowelHarmony,
        build_dry_run_report, build_experiment_report, build_segmentation_report,
        deterministic_segment_token, normalize_text,
    };

    fn test_lexicon() -> SegmentationLexicon {
        SegmentationLexicon {
            version: "0.0.24".to_string(),
            name: "adam-kazakh-segmentation-roots".to_string(),
            target_language: "kazakh".to_string(),
            script: "cyrillic".to_string(),
            roots: vec![
                SegmentationRootEntry {
                    id: "noun_mekeme".to_string(),
                    root: "мекеме".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Noun,
                    vowel_harmony: VowelHarmony::Front,
                    final_sound_class: FinalSoundClass::Vowel,
                },
                SegmentationRootEntry {
                    id: "noun_anyqtama".to_string(),
                    root: "анықтама".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Noun,
                    vowel_harmony: VowelHarmony::Back,
                    final_sound_class: FinalSoundClass::Vowel,
                },
                SegmentationRootEntry {
                    id: "noun_qujat".to_string(),
                    root: "құжат".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Noun,
                    vowel_harmony: VowelHarmony::Back,
                    final_sound_class: FinalSoundClass::VoicelessConsonant,
                },
                SegmentationRootEntry {
                    id: "noun_otinish".to_string(),
                    root: "өтініш".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Noun,
                    vowel_harmony: VowelHarmony::Front,
                    final_sound_class: FinalSoundClass::VoicelessConsonant,
                },
                SegmentationRootEntry {
                    id: "noun_shagym".to_string(),
                    root: "шағым".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Noun,
                    vowel_harmony: VowelHarmony::Back,
                    final_sound_class: FinalSoundClass::Nasal,
                },
                SegmentationRootEntry {
                    id: "noun_martebe".to_string(),
                    root: "мәртебе".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Noun,
                    vowel_harmony: VowelHarmony::Front,
                    final_sound_class: FinalSoundClass::Vowel,
                },
                SegmentationRootEntry {
                    id: "noun_qyzmet".to_string(),
                    root: "қызмет".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Noun,
                    vowel_harmony: VowelHarmony::Front,
                    final_sound_class: FinalSoundClass::VoicelessConsonant,
                },
                SegmentationRootEntry {
                    id: "pron_ol".to_string(),
                    root: "ол".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Pronoun,
                    vowel_harmony: VowelHarmony::Back,
                    final_sound_class: FinalSoundClass::VoicedConsonant,
                },
                SegmentationRootEntry {
                    id: "pron_o".to_string(),
                    root: "о".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Pronoun,
                    vowel_harmony: VowelHarmony::Back,
                    final_sound_class: FinalSoundClass::Vowel,
                },
                SegmentationRootEntry {
                    id: "pron_biz".to_string(),
                    root: "біз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Pronoun,
                    vowel_harmony: VowelHarmony::Front,
                    final_sound_class: FinalSoundClass::VoicedConsonant,
                },
                SegmentationRootEntry {
                    id: "pron_siz".to_string(),
                    root: "сіз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Pronoun,
                    vowel_harmony: VowelHarmony::Front,
                    final_sound_class: FinalSoundClass::VoicedConsonant,
                },
                SegmentationRootEntry {
                    id: "verb_kel".to_string(),
                    root: "кел".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    vowel_harmony: VowelHarmony::Front,
                    final_sound_class: FinalSoundClass::VoicedConsonant,
                },
                SegmentationRootEntry {
                    id: "verb_qara".to_string(),
                    root: "қара".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    vowel_harmony: VowelHarmony::Back,
                    final_sound_class: FinalSoundClass::Vowel,
                },
            ],
        }
    }

    fn test_rules() -> SegmentationRuleSet {
        SegmentationRuleSet {
            version: "0.0.24".to_string(),
            name: "adam-kazakh-segmentation-rules".to_string(),
            target_language: "kazakh".to_string(),
            script: "cyrillic".to_string(),
            rules: vec![
                SegmentationSuffixRule {
                    id: "noun_number_ler".to_string(),
                    form: "лер".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Noun,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Number,
                    label: "plural".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "noun_number_tar".to_string(),
                    form: "тар".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Noun,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Number,
                    label: "plural".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicelessConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "noun_possessive_3sg_sy".to_string(),
                    form: "сы".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Noun,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Possessive,
                    label: "possessive_3sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "noun_possessive_3sg_si".to_string(),
                    form: "сі".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Noun,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Possessive,
                    label: "possessive_3sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "noun_possessive_3sg_y".to_string(),
                    form: "ы".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Noun,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Possessive,
                    label: "possessive_3sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![
                        FinalSoundClass::VoicedConsonant,
                        FinalSoundClass::VoicelessConsonant,
                        FinalSoundClass::Nasal,
                    ],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "noun_possessive_3sg_i".to_string(),
                    form: "і".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Noun,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Possessive,
                    label: "possessive_3sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![
                        FinalSoundClass::VoicedConsonant,
                        FinalSoundClass::VoicelessConsonant,
                        FinalSoundClass::Nasal,
                    ],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "noun_case_ablative_den".to_string(),
                    form: "ден".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Noun,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Case,
                    label: "ablative".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![
                        FinalSoundClass::Vowel,
                        FinalSoundClass::VoicedConsonant,
                    ],
                    terminal: true,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "noun_case_accusative_ny".to_string(),
                    form: "ны".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Noun,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Case,
                    label: "accusative".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "noun_case_accusative_dy".to_string(),
                    form: "ды".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Noun,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Case,
                    label: "accusative".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![
                        FinalSoundClass::VoicedConsonant,
                        FinalSoundClass::Nasal,
                    ],
                    terminal: true,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "noun_case_dative_ge".to_string(),
                    form: "ге".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Noun,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Case,
                    label: "dative".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![
                        FinalSoundClass::Vowel,
                        FinalSoundClass::VoicedConsonant,
                        FinalSoundClass::Nasal,
                    ],
                    terminal: true,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "noun_case_dative_ke".to_string(),
                    form: "ке".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Noun,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Case,
                    label: "dative".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicelessConsonant],
                    terminal: true,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "noun_case_genitive_nin".to_string(),
                    form: "нің".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Noun,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Case,
                    label: "genitive".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "noun_case_locative_te".to_string(),
                    form: "те".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Noun,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Case,
                    label: "locative".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicelessConsonant],
                    terminal: true,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "noun_number_case_accusative_dy".to_string(),
                    form: "ды".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Noun,
                    from_state: SegmentationState::Number,
                    to_state: SegmentationState::Case,
                    label: "accusative".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: true,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "noun_number_case_dative_ge".to_string(),
                    form: "ге".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Noun,
                    from_state: SegmentationState::Number,
                    to_state: SegmentationState::Case,
                    label: "dative".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: true,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "noun_number_possessive_3sg_y".to_string(),
                    form: "ы".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Noun,
                    from_state: SegmentationState::Number,
                    to_state: SegmentationState::Possessive,
                    label: "possessive_3sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "noun_number_possessive_3sg_i".to_string(),
                    form: "і".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Noun,
                    from_state: SegmentationState::Number,
                    to_state: SegmentationState::Possessive,
                    label: "possessive_3sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "noun_possessive_case_accusative_n".to_string(),
                    form: "н".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Noun,
                    from_state: SegmentationState::Possessive,
                    to_state: SegmentationState::Case,
                    label: "accusative".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front, VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "noun_possessive_case_dative_na".to_string(),
                    form: "на".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Noun,
                    from_state: SegmentationState::Possessive,
                    to_state: SegmentationState::Case,
                    label: "dative".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "noun_possessive_case_dative_ne".to_string(),
                    form: "не".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Noun,
                    from_state: SegmentationState::Possessive,
                    to_state: SegmentationState::Case,
                    label: "dative".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "noun_possessive_case_locative_nda".to_string(),
                    form: "нда".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Noun,
                    from_state: SegmentationState::Possessive,
                    to_state: SegmentationState::Case,
                    label: "locative".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "noun_possessive_case_locative_nde".to_string(),
                    form: "нде".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Noun,
                    from_state: SegmentationState::Possessive,
                    to_state: SegmentationState::Case,
                    label: "locative".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "noun_possessive_case_ablative_nan".to_string(),
                    form: "нан".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Noun,
                    from_state: SegmentationState::Possessive,
                    to_state: SegmentationState::Case,
                    label: "ablative".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "noun_possessive_case_ablative_nen".to_string(),
                    form: "нен".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Noun,
                    from_state: SegmentationState::Possessive,
                    to_state: SegmentationState::Case,
                    label: "ablative".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "noun_possessive_case_genitive_nyn".to_string(),
                    form: "ның".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Noun,
                    from_state: SegmentationState::Possessive,
                    to_state: SegmentationState::Case,
                    label: "genitive".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "noun_possessive_case_genitive_nin".to_string(),
                    form: "нің".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Noun,
                    from_state: SegmentationState::Possessive,
                    to_state: SegmentationState::Case,
                    label: "genitive".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "pron_case_accusative_ny".to_string(),
                    form: "ны".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Pronoun,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Case,
                    label: "accusative".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "pron_case_accusative_di".to_string(),
                    form: "ді".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Pronoun,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Case,
                    label: "accusative".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: true,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "pron_case_dative_ghan".to_string(),
                    form: "ған".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Pronoun,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Case,
                    label: "dative".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "pron_case_dative_ge".to_string(),
                    form: "ге".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Pronoun,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Case,
                    label: "dative".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: true,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "pron_case_locative_nda".to_string(),
                    form: "нда".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Pronoun,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Case,
                    label: "locative".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "pron_case_locative_de".to_string(),
                    form: "де".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Pronoun,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Case,
                    label: "locative".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: true,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "pron_case_ablative_dan".to_string(),
                    form: "дан".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Pronoun,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Case,
                    label: "ablative".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "pron_case_ablative_den".to_string(),
                    form: "ден".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Pronoun,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Case,
                    label: "ablative".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: true,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "pron_case_genitive_nyn".to_string(),
                    form: "ның".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Pronoun,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Case,
                    label: "genitive".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "pron_case_genitive_ding".to_string(),
                    form: "дің".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Pronoun,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Case,
                    label: "genitive".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: true,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_voice_l".to_string(),
                    form: "л".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Voice,
                    label: "voice".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front, VowelHarmony::Back],
                    allowed_final_sound_classes: vec![
                        FinalSoundClass::Vowel,
                        FinalSoundClass::VoicedConsonant,
                    ],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_di_from_stem".to_string(),
                    form: "ді".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Tense,
                    label: "past".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![
                        FinalSoundClass::Vowel,
                        FinalSoundClass::VoicedConsonant,
                    ],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_dy_from_stem".to_string(),
                    form: "ды".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Tense,
                    label: "past".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![
                        FinalSoundClass::Vowel,
                        FinalSoundClass::VoicedConsonant,
                    ],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_di_from_voice".to_string(),
                    form: "ді".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Voice,
                    to_state: SegmentationState::Tense,
                    label: "past".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_dy_from_voice".to_string(),
                    form: "ды".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Voice,
                    to_state: SegmentationState::Tense,
                    label: "past".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_e_from_stem".to_string(),
                    form: "е".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Tense,
                    label: "future".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![
                        FinalSoundClass::Vowel,
                        FinalSoundClass::VoicedConsonant,
                    ],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_a_from_stem".to_string(),
                    form: "а".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Tense,
                    label: "future".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![
                        FinalSoundClass::Vowel,
                        FinalSoundClass::VoicedConsonant,
                    ],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_e_from_voice".to_string(),
                    form: "е".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Voice,
                    to_state: SegmentationState::Tense,
                    label: "future".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_a_from_voice".to_string(),
                    form: "а".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Voice,
                    to_state: SegmentationState::Tense,
                    label: "future".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_gen_from_stem".to_string(),
                    form: "ген".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Tense,
                    label: "evidential_past".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![
                        FinalSoundClass::Vowel,
                        FinalSoundClass::VoicedConsonant,
                    ],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_gan_from_stem".to_string(),
                    form: "ған".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Tense,
                    label: "evidential_past".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![
                        FinalSoundClass::Vowel,
                        FinalSoundClass::VoicedConsonant,
                    ],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_ken_from_stem".to_string(),
                    form: "кен".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Tense,
                    label: "evidential_past".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicelessConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_qan_from_stem".to_string(),
                    form: "қан".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Tense,
                    label: "evidential_past".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicelessConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_gen_from_voice".to_string(),
                    form: "ген".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Voice,
                    to_state: SegmentationState::Tense,
                    label: "evidential_past".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_gan_from_voice".to_string(),
                    form: "ған".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Voice,
                    to_state: SegmentationState::Tense,
                    label: "evidential_past".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_megen_from_stem".to_string(),
                    form: "меген".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Tense,
                    label: "negative_evidential_past".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_maghan_from_stem".to_string(),
                    form: "маған".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Tense,
                    label: "negative_evidential_past".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![
                        FinalSoundClass::Vowel,
                        FinalSoundClass::VoicedConsonant,
                    ],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_megen_from_voice".to_string(),
                    form: "меген".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Voice,
                    to_state: SegmentationState::Tense,
                    label: "negative_evidential_past".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_maghan_from_voice".to_string(),
                    form: "маған".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Voice,
                    to_state: SegmentationState::Tense,
                    label: "negative_evidential_past".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_etin_from_stem".to_string(),
                    form: "етін".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Tense,
                    label: "habitual_past".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_atyn_from_stem".to_string(),
                    form: "атын".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Tense,
                    label: "habitual_past".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_ytin_from_stem".to_string(),
                    form: "йтін".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Tense,
                    label: "habitual_past".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_ytyn_from_stem".to_string(),
                    form: "йтын".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Tense,
                    label: "habitual_past".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_etin_from_voice".to_string(),
                    form: "етін".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Voice,
                    to_state: SegmentationState::Tense,
                    label: "habitual_past".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_atyn_from_voice".to_string(),
                    form: "атын".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Voice,
                    to_state: SegmentationState::Tense,
                    label: "habitual_past".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_meytin_from_stem".to_string(),
                    form: "мейтін".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Tense,
                    label: "negative_habitual_past".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_maityn_from_stem".to_string(),
                    form: "майтын".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Tense,
                    label: "negative_habitual_past".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_meytin_from_voice".to_string(),
                    form: "мейтін".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Voice,
                    to_state: SegmentationState::Tense,
                    label: "negative_habitual_past".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_maityn_from_voice".to_string(),
                    form: "майтын".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Voice,
                    to_state: SegmentationState::Tense,
                    label: "negative_habitual_past".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_er_from_stem".to_string(),
                    form: "ер".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Tense,
                    label: "aorist".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_ar_from_stem".to_string(),
                    form: "ар".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Tense,
                    label: "aorist".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_r_from_stem".to_string(),
                    form: "р".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Tense,
                    label: "aorist".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front, VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_er_from_voice".to_string(),
                    form: "ер".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Voice,
                    to_state: SegmentationState::Tense,
                    label: "aorist".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_ar_from_voice".to_string(),
                    form: "ар".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Voice,
                    to_state: SegmentationState::Tense,
                    label: "aorist".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_mek_from_stem".to_string(),
                    form: "мек".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Tense,
                    label: "intentional_future".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_maq_from_stem".to_string(),
                    form: "мақ".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Tense,
                    label: "intentional_future".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_mek_from_voice".to_string(),
                    form: "мек".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Voice,
                    to_state: SegmentationState::Tense,
                    label: "intentional_future".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_maq_from_voice".to_string(),
                    form: "мақ".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Voice,
                    to_state: SegmentationState::Tense,
                    label: "intentional_future".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_se_from_stem".to_string(),
                    form: "се".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Tense,
                    label: "conditional".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_sa_from_stem_vowel".to_string(),
                    form: "са".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Tense,
                    label: "conditional".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_sa_from_stem_consonant".to_string(),
                    form: "са".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Tense,
                    label: "conditional".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_se_from_voice".to_string(),
                    form: "се".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Voice,
                    to_state: SegmentationState::Tense,
                    label: "conditional".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_sa_from_voice".to_string(),
                    form: "са".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Voice,
                    to_state: SegmentationState::Tense,
                    label: "conditional".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_mese_from_stem".to_string(),
                    form: "месе".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Tense,
                    label: "negative_conditional".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_masa_from_stem".to_string(),
                    form: "маса".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Tense,
                    label: "negative_conditional".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_mese_from_voice".to_string(),
                    form: "месе".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Voice,
                    to_state: SegmentationState::Tense,
                    label: "negative_conditional".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_masa_from_voice".to_string(),
                    form: "маса".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Voice,
                    to_state: SegmentationState::Tense,
                    label: "negative_conditional".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_mes_from_stem".to_string(),
                    form: "мес".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Tense,
                    label: "negative_aorist".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_mas_from_stem".to_string(),
                    form: "мас".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Tense,
                    label: "negative_aorist".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_mas_from_voice".to_string(),
                    form: "мас".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Voice,
                    to_state: SegmentationState::Tense,
                    label: "negative_aorist".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_mey_from_stem".to_string(),
                    form: "мей".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Tense,
                    label: "negative_future".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_mai_from_stem".to_string(),
                    form: "май".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Stem,
                    to_state: SegmentationState::Tense,
                    label: "negative_future".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_mey_from_voice".to_string(),
                    form: "мей".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Voice,
                    to_state: SegmentationState::Tense,
                    label: "negative_future".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_tense_mai_from_voice".to_string(),
                    form: "май".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Voice,
                    to_state: SegmentationState::Tense,
                    label: "negative_future".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: false,
                    allowed_previous_labels: vec![],
                },
                SegmentationSuffixRule {
                    id: "verb_person_1sg_m".to_string(),
                    form: "м".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front, VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_person_2sg_ng".to_string(),
                    form: "ң".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front, VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_person_1pl_k".to_string(),
                    form: "к".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_person_1pl_q".to_string(),
                    form: "қ".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_person_2pl_ndar".to_string(),
                    form: "ңдар".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_person_2pl_nder".to_string(),
                    form: "ңдер".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_person_2polite_nyz".to_string(),
                    form: "ңыз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_person_2polite_niz".to_string(),
                    form: "ңіз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_future_person_1sg_min".to_string(),
                    form: "мін".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_evidential_person_1sg_min".to_string(),
                    form: "мін".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["evidential_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_evidential_person_1sg_myn".to_string(),
                    form: "мын".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["evidential_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_evidential_person_1pl_biz".to_string(),
                    form: "біз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["evidential_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_evidential_person_1pl_byz".to_string(),
                    form: "быз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["evidential_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_evidential_person_2sg_sing".to_string(),
                    form: "сің".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["evidential_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_evidential_person_2sg_syng".to_string(),
                    form: "сың".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["evidential_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_evidential_person_2pl_singder".to_string(),
                    form: "сіңдер".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["evidential_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_evidential_person_2pl_syngdar".to_string(),
                    form: "сыңдар".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["evidential_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_evidential_person_2polite_siz".to_string(),
                    form: "сіз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["evidential_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_evidential_person_2polite_syz".to_string(),
                    form: "сыз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["evidential_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_evidential_person_2polite_plural_sizder".to_string(),
                    form: "сіздер".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite_plural".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["evidential_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_evidential_person_2polite_plural_syzdar".to_string(),
                    form: "сыздар".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite_plural".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["evidential_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_evidential_person_1sg_min".to_string(),
                    form: "мін".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_evidential_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_evidential_person_1sg_myn".to_string(),
                    form: "мын".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_evidential_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_evidential_person_1pl_biz".to_string(),
                    form: "біз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_evidential_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_evidential_person_1pl_byz".to_string(),
                    form: "быз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_evidential_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_evidential_person_2sg_sing".to_string(),
                    form: "сің".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_evidential_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_evidential_person_2sg_syng".to_string(),
                    form: "сың".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_evidential_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_evidential_person_2pl_singder".to_string(),
                    form: "сіңдер".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_evidential_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_evidential_person_2pl_syngdar".to_string(),
                    form: "сыңдар".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_evidential_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_evidential_person_2polite_siz".to_string(),
                    form: "сіз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_evidential_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_evidential_person_2polite_syz".to_string(),
                    form: "сыз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_evidential_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_evidential_person_2polite_plural_sizder".to_string(),
                    form: "сіздер".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite_plural".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_evidential_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_evidential_person_2polite_plural_syzdar".to_string(),
                    form: "сыздар".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite_plural".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_evidential_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_habitual_person_1sg_min".to_string(),
                    form: "мін".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["habitual_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_habitual_person_1sg_myn".to_string(),
                    form: "мын".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["habitual_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_habitual_person_1pl_biz".to_string(),
                    form: "біз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["habitual_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_habitual_person_1pl_byz".to_string(),
                    form: "быз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["habitual_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_habitual_person_2sg_sing".to_string(),
                    form: "сің".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["habitual_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_habitual_person_2sg_syng".to_string(),
                    form: "сың".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["habitual_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_habitual_person_2pl_singder".to_string(),
                    form: "сіңдер".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["habitual_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_habitual_person_2pl_syngdar".to_string(),
                    form: "сыңдар".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["habitual_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_habitual_person_2polite_siz".to_string(),
                    form: "сіз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["habitual_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_habitual_person_2polite_syz".to_string(),
                    form: "сыз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["habitual_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_habitual_person_2polite_plural_sizder".to_string(),
                    form: "сіздер".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite_plural".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["habitual_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_habitual_person_2polite_plural_syzdar".to_string(),
                    form: "сыздар".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite_plural".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["habitual_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_habitual_person_1sg_min".to_string(),
                    form: "мін".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_habitual_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_habitual_person_1sg_myn".to_string(),
                    form: "мын".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_habitual_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_habitual_person_1pl_biz".to_string(),
                    form: "біз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_habitual_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_habitual_person_1pl_byz".to_string(),
                    form: "быз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_habitual_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_habitual_person_2sg_sing".to_string(),
                    form: "сің".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_habitual_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_habitual_person_2sg_syng".to_string(),
                    form: "сың".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_habitual_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_habitual_person_2pl_singder".to_string(),
                    form: "сіңдер".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_habitual_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_habitual_person_2pl_syngdar".to_string(),
                    form: "сыңдар".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_habitual_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_habitual_person_2polite_siz".to_string(),
                    form: "сіз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_habitual_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_habitual_person_2polite_syz".to_string(),
                    form: "сыз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_habitual_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_habitual_person_2polite_plural_sizder".to_string(),
                    form: "сіздер".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite_plural".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_habitual_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_habitual_person_2polite_plural_syzdar".to_string(),
                    form: "сыздар".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite_plural".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Nasal],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_habitual_past".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_aorist_person_1sg_min".to_string(),
                    form: "мін".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["aorist".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_aorist_person_1sg_myn".to_string(),
                    form: "мын".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["aorist".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_aorist_person_1pl_miz".to_string(),
                    form: "міз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["aorist".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_aorist_person_1pl_myz".to_string(),
                    form: "мыз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["aorist".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_aorist_person_2sg_sing".to_string(),
                    form: "сің".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["aorist".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_aorist_person_2sg_syng".to_string(),
                    form: "сың".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["aorist".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_aorist_person_2pl_singder".to_string(),
                    form: "сіңдер".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["aorist".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_aorist_person_2pl_syngdar".to_string(),
                    form: "сыңдар".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["aorist".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_aorist_person_2polite_siz".to_string(),
                    form: "сіз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["aorist".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_aorist_person_2polite_syz".to_string(),
                    form: "сыз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["aorist".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_aorist_person_2polite_plural_sizder".to_string(),
                    form: "сіздер".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite_plural".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["aorist".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_aorist_person_2polite_plural_syzdar".to_string(),
                    form: "сыздар".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite_plural".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["aorist".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_intentional_future_person_1sg_pin".to_string(),
                    form: "пін".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicelessConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["intentional_future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_intentional_future_person_1sg_pyn".to_string(),
                    form: "пын".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicelessConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["intentional_future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_intentional_future_person_1pl_piz".to_string(),
                    form: "піз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicelessConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["intentional_future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_intentional_future_person_1pl_pyz".to_string(),
                    form: "пыз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicelessConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["intentional_future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_intentional_future_person_2sg_sing".to_string(),
                    form: "сің".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicelessConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["intentional_future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_intentional_future_person_2sg_syng".to_string(),
                    form: "сың".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicelessConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["intentional_future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_intentional_future_person_2pl_singder".to_string(),
                    form: "сіңдер".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicelessConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["intentional_future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_intentional_future_person_2pl_syngdar".to_string(),
                    form: "сыңдар".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicelessConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["intentional_future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_intentional_future_person_2polite_siz".to_string(),
                    form: "сіз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicelessConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["intentional_future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_intentional_future_person_2polite_syz".to_string(),
                    form: "сыз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicelessConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["intentional_future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_intentional_future_person_2polite_plural_sizder".to_string(),
                    form: "сіздер".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite_plural".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicelessConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["intentional_future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_intentional_future_person_2polite_plural_syzdar".to_string(),
                    form: "сыздар".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite_plural".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicelessConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["intentional_future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_conditional_person_1sg_m".to_string(),
                    form: "м".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front, VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["conditional".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_conditional_person_2sg_ng".to_string(),
                    form: "ң".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front, VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["conditional".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_conditional_person_1pl_k".to_string(),
                    form: "к".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["conditional".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_conditional_person_1pl_q".to_string(),
                    form: "қ".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["conditional".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_conditional_person_2pl_nder".to_string(),
                    form: "ңдер".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["conditional".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_conditional_person_2pl_ndar".to_string(),
                    form: "ңдар".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["conditional".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_conditional_person_2polite_niz".to_string(),
                    form: "ңіз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["conditional".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_conditional_person_2polite_nyz".to_string(),
                    form: "ңыз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["conditional".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_conditional_person_1sg_m".to_string(),
                    form: "м".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front, VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_conditional".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_conditional_person_2sg_ng".to_string(),
                    form: "ң".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front, VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_conditional".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_conditional_person_1pl_k".to_string(),
                    form: "к".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_conditional".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_conditional_person_1pl_q".to_string(),
                    form: "қ".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_conditional".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_conditional_person_2pl_nder".to_string(),
                    form: "ңдер".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_conditional".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_conditional_person_2pl_ndar".to_string(),
                    form: "ңдар".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_conditional".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_conditional_person_2polite_niz".to_string(),
                    form: "ңіз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_conditional".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_conditional_person_2polite_nyz".to_string(),
                    form: "ңыз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_conditional".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_aorist_person_1sg_pin".to_string(),
                    form: "пін".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicelessConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_aorist".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_aorist_person_1sg_pyn".to_string(),
                    form: "пын".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicelessConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_aorist".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_aorist_person_1pl_piz".to_string(),
                    form: "піз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicelessConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_aorist".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_aorist_person_1pl_pyz".to_string(),
                    form: "пыз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicelessConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_aorist".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_aorist_person_2sg_sing".to_string(),
                    form: "сің".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicelessConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_aorist".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_aorist_person_2sg_syng".to_string(),
                    form: "сың".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicelessConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_aorist".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_aorist_person_2pl_singder".to_string(),
                    form: "сіңдер".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicelessConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_aorist".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_aorist_person_2pl_syngdar".to_string(),
                    form: "сыңдар".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicelessConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_aorist".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_aorist_person_2polite_siz".to_string(),
                    form: "сіз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicelessConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_aorist".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_aorist_person_2polite_syz".to_string(),
                    form: "сыз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicelessConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_aorist".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_aorist_person_2polite_plural_sizder".to_string(),
                    form: "сіздер".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite_plural".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicelessConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_aorist".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_aorist_person_2polite_plural_syzdar".to_string(),
                    form: "сыздар".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite_plural".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicelessConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_aorist".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_future_person_1sg_min".to_string(),
                    form: "мін".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_future_person_1sg_myn".to_string(),
                    form: "мын".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_future_person_1pl_miz".to_string(),
                    form: "міз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_future_person_1pl_myz".to_string(),
                    form: "мыз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_future_person_2sg_sing".to_string(),
                    form: "сің".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_future_person_2sg_syng".to_string(),
                    form: "сың".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_future_person_2pl_singder".to_string(),
                    form: "сіңдер".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_future_person_2pl_syngdar".to_string(),
                    form: "сыңдар".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_future_person_2polite_siz".to_string(),
                    form: "сіз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_future_person_2polite_syz".to_string(),
                    form: "сыз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_future_person_2polite_plural_sizder".to_string(),
                    form: "сіздер".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite_plural".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_negative_future_person_2polite_plural_syzdar".to_string(),
                    form: "сыздар".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite_plural".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::VoicedConsonant],
                    terminal: true,
                    allowed_previous_labels: vec!["negative_future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_future_person_1sg_myn".to_string(),
                    form: "мын".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_future_person_1pl_miz".to_string(),
                    form: "міз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_future_person_1pl_myz".to_string(),
                    form: "мыз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_1pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_future_person_2sg_sing".to_string(),
                    form: "сің".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_future_person_2sg_syng".to_string(),
                    form: "сың".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2sg".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_future_person_2pl_syngdar".to_string(),
                    form: "сыңдар".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_future_person_2pl_singder".to_string(),
                    form: "сіңдер".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2pl".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_future_person_2polite_siz".to_string(),
                    form: "сіз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_future_person_2polite_syz".to_string(),
                    form: "сыз".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_future_person_2polite_plural_sizder".to_string(),
                    form: "сіздер".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite_plural".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Front],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["future".to_string()],
                },
                SegmentationSuffixRule {
                    id: "verb_future_person_2polite_plural_syzdar".to_string(),
                    form: "сыздар".to_string(),
                    part_of_speech: SegmentationPartOfSpeech::Verb,
                    from_state: SegmentationState::Tense,
                    to_state: SegmentationState::Person,
                    label: "person_2polite_plural".to_string(),
                    allowed_harmonies: vec![VowelHarmony::Back],
                    allowed_final_sound_classes: vec![FinalSoundClass::Vowel],
                    terminal: true,
                    allowed_previous_labels: vec!["future".to_string()],
                },
            ],
        }
    }

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
            version: "0.0.24".to_string(),
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

        assert!(experiment.validate().is_ok());
    }

    #[test]
    fn builds_dry_run_report() {
        let experiment = TokenizerExperiment {
            version: "0.0.24".to_string(),
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
        let pack = TokenizerDryRunPack {
            version: "0.0.24".to_string(),
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

        assert_eq!(report.experiment_name, "adam-tokenizer-deterministic");
        assert_eq!(report.sample_count, 2);
        assert_eq!(report.normalized_nonempty_count, 2);
        assert_eq!(report.domains.len(), 2);
    }

    #[test]
    fn validates_segmentation_dataset_and_builds_report() {
        let dataset = TokenizerSegmentationDataset {
            version: "0.0.24".to_string(),
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
                TokenizerSegmentationExample {
                    id: "seg_03".to_string(),
                    token: "өтінішке".to_string(),
                    expected_segments: vec!["өтініш".to_string(), "ке".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_04".to_string(),
                    token: "құжаттарды".to_string(),
                    expected_segments: vec![
                        "құжат".to_string(),
                        "тар".to_string(),
                        "ды".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_05".to_string(),
                    token: "мекемесін".to_string(),
                    expected_segments: vec![
                        "мекеме".to_string(),
                        "сі".to_string(),
                        "н".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_06".to_string(),
                    token: "құжатын".to_string(),
                    expected_segments: vec!["құжат".to_string(), "ы".to_string(), "н".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_07".to_string(),
                    token: "мекемесіне".to_string(),
                    expected_segments: vec![
                        "мекеме".to_string(),
                        "сі".to_string(),
                        "не".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_08".to_string(),
                    token: "құжатында".to_string(),
                    expected_segments: vec![
                        "құжат".to_string(),
                        "ы".to_string(),
                        "нда".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_09".to_string(),
                    token: "өтінішінен".to_string(),
                    expected_segments: vec![
                        "өтініш".to_string(),
                        "і".to_string(),
                        "нен".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_10".to_string(),
                    token: "анықтамасының".to_string(),
                    expected_segments: vec![
                        "анықтама".to_string(),
                        "сы".to_string(),
                        "ның".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_11".to_string(),
                    token: "оны".to_string(),
                    expected_segments: vec!["о".to_string(), "ны".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_12".to_string(),
                    token: "бізге".to_string(),
                    expected_segments: vec!["біз".to_string(), "ге".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_13".to_string(),
                    token: "құжаттары".to_string(),
                    expected_segments: vec![
                        "құжат".to_string(),
                        "тар".to_string(),
                        "ы".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_14".to_string(),
                    token: "мекемелеріне".to_string(),
                    expected_segments: vec![
                        "мекеме".to_string(),
                        "лер".to_string(),
                        "і".to_string(),
                        "не".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_15".to_string(),
                    token: "келдім".to_string(),
                    expected_segments: vec!["кел".to_string(), "ді".to_string(), "м".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_16".to_string(),
                    token: "қаралдым".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "ды".to_string(),
                        "м".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_17".to_string(),
                    token: "келдіңдер".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "ді".to_string(),
                        "ңдер".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_18".to_string(),
                    token: "қаралдыңыз".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "ды".to_string(),
                        "ңыз".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_19".to_string(),
                    token: "келемін".to_string(),
                    expected_segments: vec!["кел".to_string(), "е".to_string(), "мін".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_20".to_string(),
                    token: "келесіз".to_string(),
                    expected_segments: vec!["кел".to_string(), "е".to_string(), "сіз".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_21".to_string(),
                    token: "қараламыз".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "а".to_string(),
                        "мыз".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_22".to_string(),
                    token: "қараласыңдар".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "а".to_string(),
                        "сыңдар".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_23".to_string(),
                    token: "келесің".to_string(),
                    expected_segments: vec!["кел".to_string(), "е".to_string(), "сің".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_24".to_string(),
                    token: "қараласың".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "а".to_string(),
                        "сың".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_25".to_string(),
                    token: "келесіңдер".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "е".to_string(),
                        "сіңдер".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_26".to_string(),
                    token: "келесіздер".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "е".to_string(),
                        "сіздер".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_27".to_string(),
                    token: "қараласыз".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "а".to_string(),
                        "сыз".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_28".to_string(),
                    token: "қараласыздар".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "а".to_string(),
                        "сыздар".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_29".to_string(),
                    token: "келгенмін".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "ген".to_string(),
                        "мін".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_30".to_string(),
                    token: "келгенсің".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "ген".to_string(),
                        "сің".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_31".to_string(),
                    token: "келгенбіз".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "ген".to_string(),
                        "біз".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_32".to_string(),
                    token: "келгенсіздер".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "ген".to_string(),
                        "сіздер".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_33".to_string(),
                    token: "қаралғанмын".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "ған".to_string(),
                        "мын".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_34".to_string(),
                    token: "қаралғансыңдар".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "ған".to_string(),
                        "сыңдар".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_35".to_string(),
                    token: "келетінмін".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "етін".to_string(),
                        "мін".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_36".to_string(),
                    token: "келетінсің".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "етін".to_string(),
                        "сің".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_37".to_string(),
                    token: "келетінбіз".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "етін".to_string(),
                        "біз".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_38".to_string(),
                    token: "келетінсіздер".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "етін".to_string(),
                        "сіздер".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_39".to_string(),
                    token: "қарайтынмын".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "йтын".to_string(),
                        "мын".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_40".to_string(),
                    token: "қаралатынбыз".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "атын".to_string(),
                        "быз".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_41".to_string(),
                    token: "келермін".to_string(),
                    expected_segments: vec!["кел".to_string(), "ер".to_string(), "мін".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_42".to_string(),
                    token: "келерсің".to_string(),
                    expected_segments: vec!["кел".to_string(), "ер".to_string(), "сің".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_43".to_string(),
                    token: "келерміз".to_string(),
                    expected_segments: vec!["кел".to_string(), "ер".to_string(), "міз".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_44".to_string(),
                    token: "келерсіздер".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "ер".to_string(),
                        "сіздер".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_45".to_string(),
                    token: "қарармын".to_string(),
                    expected_segments: vec!["қара".to_string(), "р".to_string(), "мын".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_46".to_string(),
                    token: "қаралармыз".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "ар".to_string(),
                        "мыз".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_47".to_string(),
                    token: "келмекпін".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "мек".to_string(),
                        "пін".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_48".to_string(),
                    token: "келмексің".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "мек".to_string(),
                        "сің".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_49".to_string(),
                    token: "келмекпіз".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "мек".to_string(),
                        "піз".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_50".to_string(),
                    token: "келмексіздер".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "мек".to_string(),
                        "сіздер".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_51".to_string(),
                    token: "қарамақпын".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "мақ".to_string(),
                        "пын".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_52".to_string(),
                    token: "қаралмақпыз".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "мақ".to_string(),
                        "пыз".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_53".to_string(),
                    token: "келсем".to_string(),
                    expected_segments: vec!["кел".to_string(), "се".to_string(), "м".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_54".to_string(),
                    token: "келсеңіз".to_string(),
                    expected_segments: vec!["кел".to_string(), "се".to_string(), "ңіз".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_55".to_string(),
                    token: "келсеңдер".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "се".to_string(),
                        "ңдер".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_56".to_string(),
                    token: "келсек".to_string(),
                    expected_segments: vec!["кел".to_string(), "се".to_string(), "к".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_57".to_string(),
                    token: "қарасам".to_string(),
                    expected_segments: vec!["қара".to_string(), "са".to_string(), "м".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_58".to_string(),
                    token: "қаралсақ".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "са".to_string(),
                        "қ".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_59".to_string(),
                    token: "келмеспін".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "мес".to_string(),
                        "пін".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_60".to_string(),
                    token: "келмессің".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "мес".to_string(),
                        "сің".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_61".to_string(),
                    token: "келмеспіз".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "мес".to_string(),
                        "піз".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_62".to_string(),
                    token: "келмессіздер".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "мес".to_string(),
                        "сіздер".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_63".to_string(),
                    token: "қарамаспын".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "мас".to_string(),
                        "пын".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_64".to_string(),
                    token: "қаралмаспыз".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "мас".to_string(),
                        "пыз".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_65".to_string(),
                    token: "келмеймін".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "мей".to_string(),
                        "мін".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_66".to_string(),
                    token: "келмейсің".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "мей".to_string(),
                        "сің".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_67".to_string(),
                    token: "келмейміз".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "мей".to_string(),
                        "міз".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_68".to_string(),
                    token: "келмейсіздер".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "мей".to_string(),
                        "сіздер".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_69".to_string(),
                    token: "қарамайсыңдар".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "май".to_string(),
                        "сыңдар".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_70".to_string(),
                    token: "қаралмаймыз".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "май".to_string(),
                        "мыз".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_71".to_string(),
                    token: "келмегенмін".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "меген".to_string(),
                        "мін".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_72".to_string(),
                    token: "келмегенсің".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "меген".to_string(),
                        "сің".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_73".to_string(),
                    token: "келмегенбіз".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "меген".to_string(),
                        "біз".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_74".to_string(),
                    token: "келмегенсіздер".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "меген".to_string(),
                        "сіздер".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_75".to_string(),
                    token: "қаралмағанмын".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "маған".to_string(),
                        "мын".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_76".to_string(),
                    token: "қаралмағансыңдар".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "маған".to_string(),
                        "сыңдар".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_77".to_string(),
                    token: "келмейтінмін".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "мейтін".to_string(),
                        "мін".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_78".to_string(),
                    token: "келмейтінсің".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "мейтін".to_string(),
                        "сің".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_79".to_string(),
                    token: "келмейтінбіз".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "мейтін".to_string(),
                        "біз".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_80".to_string(),
                    token: "келмейтінсіздер".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "мейтін".to_string(),
                        "сіздер".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_81".to_string(),
                    token: "қарамайтынсыңдар".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "майтын".to_string(),
                        "сыңдар".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_82".to_string(),
                    token: "қаралмайтынсыздар".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "майтын".to_string(),
                        "сыздар".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_83".to_string(),
                    token: "келмесем".to_string(),
                    expected_segments: vec!["кел".to_string(), "месе".to_string(), "м".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_84".to_string(),
                    token: "келмесеңіз".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "месе".to_string(),
                        "ңіз".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_85".to_string(),
                    token: "келмесеңдер".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "месе".to_string(),
                        "ңдер".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_86".to_string(),
                    token: "келмесек".to_string(),
                    expected_segments: vec!["кел".to_string(), "месе".to_string(), "к".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_87".to_string(),
                    token: "қарамасам".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "маса".to_string(),
                        "м".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_88".to_string(),
                    token: "қаралмасақ".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "маса".to_string(),
                        "қ".to_string(),
                    ],
                },
            ],
        };

        let report = build_segmentation_report(&dataset, &test_lexicon(), &test_rules())
            .expect("segmentation report");
        assert_eq!(report.example_count, 88);
        assert_eq!(report.average_segment_count, 3);
        assert_eq!(
            report.longest_token_length,
            "қаралмайтынсыздар".chars().count()
        );
        assert_eq!(report.exact_match_count, 88);
        assert_eq!(report.exact_match_rate_bps, 10_000);
    }

    #[test]
    fn rejects_segmentation_dataset_with_mismatched_segments() {
        let dataset = TokenizerSegmentationDataset {
            version: "0.0.24".to_string(),
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
    fn deterministic_segmenter_handles_core_kazakh_examples() {
        assert_eq!(
            deterministic_segment_token("мекемеден", &test_lexicon(), &test_rules()),
            Some(vec!["мекеме".to_string(), "ден".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("келді", &test_lexicon(), &test_rules()),
            Some(vec!["кел".to_string(), "ді".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("келдім", &test_lexicon(), &test_rules()),
            Some(vec!["кел".to_string(), "ді".to_string(), "м".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("келдің", &test_lexicon(), &test_rules()),
            Some(vec!["кел".to_string(), "ді".to_string(), "ң".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("келдік", &test_lexicon(), &test_rules()),
            Some(vec!["кел".to_string(), "ді".to_string(), "к".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("келдіңдер", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "ді".to_string(),
                "ңдер".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келдіңіз", &test_lexicon(), &test_rules()),
            Some(vec!["кел".to_string(), "ді".to_string(), "ңіз".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("қаралды", &test_lexicon(), &test_rules()),
            Some(vec!["қара".to_string(), "л".to_string(), "ды".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("қаралдым", &test_lexicon(), &test_rules()),
            Some(vec![
                "қара".to_string(),
                "л".to_string(),
                "ды".to_string(),
                "м".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("қаралдыңдар", &test_lexicon(), &test_rules()),
            Some(vec![
                "қара".to_string(),
                "л".to_string(),
                "ды".to_string(),
                "ңдар".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("қаралдыңыз", &test_lexicon(), &test_rules()),
            Some(vec![
                "қара".to_string(),
                "л".to_string(),
                "ды".to_string(),
                "ңыз".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келемін", &test_lexicon(), &test_rules()),
            Some(vec!["кел".to_string(), "е".to_string(), "мін".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("келесіз", &test_lexicon(), &test_rules()),
            Some(vec!["кел".to_string(), "е".to_string(), "сіз".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("қараламыз", &test_lexicon(), &test_rules()),
            Some(vec![
                "қара".to_string(),
                "л".to_string(),
                "а".to_string(),
                "мыз".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("қараласыңдар", &test_lexicon(), &test_rules()),
            Some(vec![
                "қара".to_string(),
                "л".to_string(),
                "а".to_string(),
                "сыңдар".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келесің", &test_lexicon(), &test_rules()),
            Some(vec!["кел".to_string(), "е".to_string(), "сің".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("қараласың", &test_lexicon(), &test_rules()),
            Some(vec![
                "қара".to_string(),
                "л".to_string(),
                "а".to_string(),
                "сың".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келесіңдер", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "е".to_string(),
                "сіңдер".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келесіздер", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "е".to_string(),
                "сіздер".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("қараласыз", &test_lexicon(), &test_rules()),
            Some(vec![
                "қара".to_string(),
                "л".to_string(),
                "а".to_string(),
                "сыз".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("қараласыздар", &test_lexicon(), &test_rules()),
            Some(vec![
                "қара".to_string(),
                "л".to_string(),
                "а".to_string(),
                "сыздар".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келгенмін", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "ген".to_string(),
                "мін".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келгенсің", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "ген".to_string(),
                "сің".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келгенбіз", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "ген".to_string(),
                "біз".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келгенсіздер", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "ген".to_string(),
                "сіздер".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("қаралғанмын", &test_lexicon(), &test_rules()),
            Some(vec![
                "қара".to_string(),
                "л".to_string(),
                "ған".to_string(),
                "мын".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("қаралғансыңдар", &test_lexicon(), &test_rules()),
            Some(vec![
                "қара".to_string(),
                "л".to_string(),
                "ған".to_string(),
                "сыңдар".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келетінмін", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "етін".to_string(),
                "мін".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келетінсің", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "етін".to_string(),
                "сің".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келетінбіз", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "етін".to_string(),
                "біз".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келетінсіздер", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "етін".to_string(),
                "сіздер".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("қарайтынмын", &test_lexicon(), &test_rules()),
            Some(vec![
                "қара".to_string(),
                "йтын".to_string(),
                "мын".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("қаралатынбыз", &test_lexicon(), &test_rules()),
            Some(vec![
                "қара".to_string(),
                "л".to_string(),
                "атын".to_string(),
                "быз".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келермін", &test_lexicon(), &test_rules()),
            Some(vec!["кел".to_string(), "ер".to_string(), "мін".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("келерсің", &test_lexicon(), &test_rules()),
            Some(vec!["кел".to_string(), "ер".to_string(), "сің".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("келерміз", &test_lexicon(), &test_rules()),
            Some(vec!["кел".to_string(), "ер".to_string(), "міз".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("келерсіздер", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "ер".to_string(),
                "сіздер".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("қарармын", &test_lexicon(), &test_rules()),
            Some(vec!["қара".to_string(), "р".to_string(), "мын".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("қаралармыз", &test_lexicon(), &test_rules()),
            Some(vec![
                "қара".to_string(),
                "л".to_string(),
                "ар".to_string(),
                "мыз".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келмекпін", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "мек".to_string(),
                "пін".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келмексің", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "мек".to_string(),
                "сің".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келмекпіз", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "мек".to_string(),
                "піз".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келмексіздер", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "мек".to_string(),
                "сіздер".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("қарамақпын", &test_lexicon(), &test_rules()),
            Some(vec![
                "қара".to_string(),
                "мақ".to_string(),
                "пын".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("қаралмақпыз", &test_lexicon(), &test_rules()),
            Some(vec![
                "қара".to_string(),
                "л".to_string(),
                "мақ".to_string(),
                "пыз".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келсем", &test_lexicon(), &test_rules()),
            Some(vec!["кел".to_string(), "се".to_string(), "м".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("келсеңіз", &test_lexicon(), &test_rules()),
            Some(vec!["кел".to_string(), "се".to_string(), "ңіз".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("келсеңдер", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "се".to_string(),
                "ңдер".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келсек", &test_lexicon(), &test_rules()),
            Some(vec!["кел".to_string(), "се".to_string(), "к".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("қарасам", &test_lexicon(), &test_rules()),
            Some(vec!["қара".to_string(), "са".to_string(), "м".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("қаралсақ", &test_lexicon(), &test_rules()),
            Some(vec![
                "қара".to_string(),
                "л".to_string(),
                "са".to_string(),
                "қ".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келмеспін", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "мес".to_string(),
                "пін".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келмессің", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "мес".to_string(),
                "сің".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келмеспіз", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "мес".to_string(),
                "піз".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келмессіздер", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "мес".to_string(),
                "сіздер".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("қарамаспын", &test_lexicon(), &test_rules()),
            Some(vec![
                "қара".to_string(),
                "мас".to_string(),
                "пын".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("қаралмаспыз", &test_lexicon(), &test_rules()),
            Some(vec![
                "қара".to_string(),
                "л".to_string(),
                "мас".to_string(),
                "пыз".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келмеймін", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "мей".to_string(),
                "мін".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келмейсің", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "мей".to_string(),
                "сің".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келмейміз", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "мей".to_string(),
                "міз".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келмейсіздер", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "мей".to_string(),
                "сіздер".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("қарамайсыңдар", &test_lexicon(), &test_rules()),
            Some(vec![
                "қара".to_string(),
                "май".to_string(),
                "сыңдар".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("қаралмаймыз", &test_lexicon(), &test_rules()),
            Some(vec![
                "қара".to_string(),
                "л".to_string(),
                "май".to_string(),
                "мыз".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келмегенмін", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "меген".to_string(),
                "мін".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келмегенсің", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "меген".to_string(),
                "сің".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келмегенбіз", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "меген".to_string(),
                "біз".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келмегенсіздер", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "меген".to_string(),
                "сіздер".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("қаралмағанмын", &test_lexicon(), &test_rules()),
            Some(vec![
                "қара".to_string(),
                "л".to_string(),
                "маған".to_string(),
                "мын".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("қаралмағансыңдар", &test_lexicon(), &test_rules()),
            Some(vec![
                "қара".to_string(),
                "л".to_string(),
                "маған".to_string(),
                "сыңдар".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келмейтінмін", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "мейтін".to_string(),
                "мін".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келмейтінсің", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "мейтін".to_string(),
                "сің".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келмейтінбіз", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "мейтін".to_string(),
                "біз".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келмейтінсіздер", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "мейтін".to_string(),
                "сіздер".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("қарамайтынсыңдар", &test_lexicon(), &test_rules()),
            Some(vec![
                "қара".to_string(),
                "майтын".to_string(),
                "сыңдар".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("қаралмайтынсыздар", &test_lexicon(), &test_rules()),
            Some(vec![
                "қара".to_string(),
                "л".to_string(),
                "майтын".to_string(),
                "сыздар".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келмесем", &test_lexicon(), &test_rules()),
            Some(vec!["кел".to_string(), "месе".to_string(), "м".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("келмесеңіз", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "месе".to_string(),
                "ңіз".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келмесеңдер", &test_lexicon(), &test_rules()),
            Some(vec![
                "кел".to_string(),
                "месе".to_string(),
                "ңдер".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("келмесек", &test_lexicon(), &test_rules()),
            Some(vec!["кел".to_string(), "месе".to_string(), "к".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("қарамасам", &test_lexicon(), &test_rules()),
            Some(vec![
                "қара".to_string(),
                "маса".to_string(),
                "м".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("қаралмасақ", &test_lexicon(), &test_rules()),
            Some(vec![
                "қара".to_string(),
                "л".to_string(),
                "маса".to_string(),
                "қ".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("мекемеге", &test_lexicon(), &test_rules()),
            Some(vec!["мекеме".to_string(), "ге".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("өтінішке", &test_lexicon(), &test_rules()),
            Some(vec!["өтініш".to_string(), "ке".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("шағымды", &test_lexicon(), &test_rules()),
            Some(vec!["шағым".to_string(), "ды".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("мәртебенің", &test_lexicon(), &test_rules()),
            Some(vec!["мәртебе".to_string(), "нің".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("қызметте", &test_lexicon(), &test_rules()),
            Some(vec!["қызмет".to_string(), "те".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("құжаттар", &test_lexicon(), &test_rules()),
            Some(vec!["құжат".to_string(), "тар".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("құжаттарды", &test_lexicon(), &test_rules()),
            Some(vec![
                "құжат".to_string(),
                "тар".to_string(),
                "ды".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("мекемелерге", &test_lexicon(), &test_rules()),
            Some(vec![
                "мекеме".to_string(),
                "лер".to_string(),
                "ге".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("мекемесін", &test_lexicon(), &test_rules()),
            Some(vec![
                "мекеме".to_string(),
                "сі".to_string(),
                "н".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("құжатын", &test_lexicon(), &test_rules()),
            Some(vec!["құжат".to_string(), "ы".to_string(), "н".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("өтінішін", &test_lexicon(), &test_rules()),
            Some(vec!["өтініш".to_string(), "і".to_string(), "н".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("анықтамасын", &test_lexicon(), &test_rules()),
            Some(vec![
                "анықтама".to_string(),
                "сы".to_string(),
                "н".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("мекемесіне", &test_lexicon(), &test_rules()),
            Some(vec![
                "мекеме".to_string(),
                "сі".to_string(),
                "не".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("құжатында", &test_lexicon(), &test_rules()),
            Some(vec![
                "құжат".to_string(),
                "ы".to_string(),
                "нда".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("өтінішінен", &test_lexicon(), &test_rules()),
            Some(vec![
                "өтініш".to_string(),
                "і".to_string(),
                "нен".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("анықтамасының", &test_lexicon(), &test_rules()),
            Some(vec![
                "анықтама".to_string(),
                "сы".to_string(),
                "ның".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("оны", &test_lexicon(), &test_rules()),
            Some(vec!["о".to_string(), "ны".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("оған", &test_lexicon(), &test_rules()),
            Some(vec!["о".to_string(), "ған".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("онда", &test_lexicon(), &test_rules()),
            Some(vec!["о".to_string(), "нда".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("оның", &test_lexicon(), &test_rules()),
            Some(vec!["о".to_string(), "ның".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("бізді", &test_lexicon(), &test_rules()),
            Some(vec!["біз".to_string(), "ді".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("бізге", &test_lexicon(), &test_rules()),
            Some(vec!["біз".to_string(), "ге".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("сіздің", &test_lexicon(), &test_rules()),
            Some(vec!["сіз".to_string(), "дің".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("сізде", &test_lexicon(), &test_rules()),
            Some(vec!["сіз".to_string(), "де".to_string()])
        );
        assert_eq!(
            deterministic_segment_token("құжаттары", &test_lexicon(), &test_rules()),
            Some(vec![
                "құжат".to_string(),
                "тар".to_string(),
                "ы".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("мекемелері", &test_lexicon(), &test_rules()),
            Some(vec![
                "мекеме".to_string(),
                "лер".to_string(),
                "і".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("құжаттарын", &test_lexicon(), &test_rules()),
            Some(vec![
                "құжат".to_string(),
                "тар".to_string(),
                "ы".to_string(),
                "н".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("мекемелеріне", &test_lexicon(), &test_rules()),
            Some(vec![
                "мекеме".to_string(),
                "лер".to_string(),
                "і".to_string(),
                "не".to_string()
            ])
        );
        assert_eq!(
            deterministic_segment_token("өтінішпен", &test_lexicon(), &test_rules()),
            None
        );
    }

    #[test]
    fn rejects_harmony_and_final_sound_mismatches() {
        assert_eq!(
            deterministic_segment_token("мекемеда", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("құжатге", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("құжатлер", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("келды", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("құжатсін", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("өтінішсын", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("мекемесіна", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("құжатынде", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("олге", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("біза", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("сізың", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("құжаттарі", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("мекемелеріна", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("келдіқ", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("қаралдык", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("келдіңдар", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("қаралдыңіз", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("келемын", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("қараласіз", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("келдімін", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("келесың", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("қараласің", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("келесіңдар", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("келесіздар", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("қараласіздер", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("келгенмын", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("қаралғансің", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("келгенбыз", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("қаралғансіздер", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("келетінмын", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("келетінбыз", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("қарайтынмін", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("қаралатынсіздер", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("келермын", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("келермыз", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("қарармін", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("қараларміз", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("келмекпын", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("келмекпыз", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("қарамақпін", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("қаралмақсіздер", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("келсам", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("келсеқ", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("келсеңдар", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("келсеңыз", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("қарасем", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("қаралсак", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("келмеспын", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("келмеспыз", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("қарамаспін", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("қаралмассіздер", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("келмеймын", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("келмеймыз", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("қарамаймін", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("қаралмайміз", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("келмегенмын", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("келмегенбыз", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("қаралмағанмін", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("қаралмағансіздер", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("келмейтінмын", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("келмейтінбыз", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("қарамайтынмін", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("қаралмайтынсіздер", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("келмасам", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("келмесеқ", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("келмесеңдар", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("келмесеңыз", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("қарамесем", &test_lexicon(), &test_rules()),
            None
        );
        assert_eq!(
            deterministic_segment_token("қаралмасак", &test_lexicon(), &test_rules()),
            None
        );
    }

    #[test]
    fn builds_experiment_report_with_segmentation_scoring() {
        let experiment = TokenizerExperiment {
            version: "0.0.24".to_string(),
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
        let pack = TokenizerDryRunPack {
            version: "0.0.24".to_string(),
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
            version: "0.0.24".to_string(),
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
                TokenizerSegmentationExample {
                    id: "seg_03".to_string(),
                    token: "құжаттарды".to_string(),
                    expected_segments: vec![
                        "құжат".to_string(),
                        "тар".to_string(),
                        "ды".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_04".to_string(),
                    token: "мекемелерге".to_string(),
                    expected_segments: vec![
                        "мекеме".to_string(),
                        "лер".to_string(),
                        "ге".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_05".to_string(),
                    token: "мекемесін".to_string(),
                    expected_segments: vec![
                        "мекеме".to_string(),
                        "сі".to_string(),
                        "н".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_06".to_string(),
                    token: "анықтамасының".to_string(),
                    expected_segments: vec![
                        "анықтама".to_string(),
                        "сы".to_string(),
                        "ның".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_07".to_string(),
                    token: "оның".to_string(),
                    expected_segments: vec!["о".to_string(), "ның".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_08".to_string(),
                    token: "құжаттарын".to_string(),
                    expected_segments: vec![
                        "құжат".to_string(),
                        "тар".to_string(),
                        "ы".to_string(),
                        "н".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_09".to_string(),
                    token: "келдім".to_string(),
                    expected_segments: vec!["кел".to_string(), "ді".to_string(), "м".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_10".to_string(),
                    token: "келдіңіз".to_string(),
                    expected_segments: vec!["кел".to_string(), "ді".to_string(), "ңіз".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_11".to_string(),
                    token: "келемін".to_string(),
                    expected_segments: vec!["кел".to_string(), "е".to_string(), "мін".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_12".to_string(),
                    token: "келесіз".to_string(),
                    expected_segments: vec!["кел".to_string(), "е".to_string(), "сіз".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_13".to_string(),
                    token: "қараламыз".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "а".to_string(),
                        "мыз".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_14".to_string(),
                    token: "қараласыңдар".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "а".to_string(),
                        "сыңдар".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_15".to_string(),
                    token: "келесің".to_string(),
                    expected_segments: vec!["кел".to_string(), "е".to_string(), "сің".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_16".to_string(),
                    token: "келесіңдер".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "е".to_string(),
                        "сіңдер".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_17".to_string(),
                    token: "келесіздер".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "е".to_string(),
                        "сіздер".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_18".to_string(),
                    token: "қараласыз".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "а".to_string(),
                        "сыз".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_19".to_string(),
                    token: "қараласыздар".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "а".to_string(),
                        "сыздар".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_20".to_string(),
                    token: "келгенмін".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "ген".to_string(),
                        "мін".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_21".to_string(),
                    token: "келгенсің".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "ген".to_string(),
                        "сің".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_22".to_string(),
                    token: "келгенбіз".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "ген".to_string(),
                        "біз".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_23".to_string(),
                    token: "келгенсіздер".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "ген".to_string(),
                        "сіздер".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_24".to_string(),
                    token: "қаралғанмын".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "ған".to_string(),
                        "мын".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_25".to_string(),
                    token: "қаралғансыңдар".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "ған".to_string(),
                        "сыңдар".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_26".to_string(),
                    token: "келетінмін".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "етін".to_string(),
                        "мін".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_27".to_string(),
                    token: "келетінсің".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "етін".to_string(),
                        "сің".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_28".to_string(),
                    token: "келетінбіз".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "етін".to_string(),
                        "біз".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_29".to_string(),
                    token: "келетінсіздер".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "етін".to_string(),
                        "сіздер".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_30".to_string(),
                    token: "қарайтынмын".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "йтын".to_string(),
                        "мын".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_31".to_string(),
                    token: "қаралатынбыз".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "атын".to_string(),
                        "быз".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_32".to_string(),
                    token: "келермін".to_string(),
                    expected_segments: vec!["кел".to_string(), "ер".to_string(), "мін".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_33".to_string(),
                    token: "келерсің".to_string(),
                    expected_segments: vec!["кел".to_string(), "ер".to_string(), "сің".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_34".to_string(),
                    token: "келерміз".to_string(),
                    expected_segments: vec!["кел".to_string(), "ер".to_string(), "міз".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_35".to_string(),
                    token: "келерсіздер".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "ер".to_string(),
                        "сіздер".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_36".to_string(),
                    token: "қарармын".to_string(),
                    expected_segments: vec!["қара".to_string(), "р".to_string(), "мын".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_37".to_string(),
                    token: "қаралармыз".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "ар".to_string(),
                        "мыз".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_38".to_string(),
                    token: "келмекпін".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "мек".to_string(),
                        "пін".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_39".to_string(),
                    token: "келмексің".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "мек".to_string(),
                        "сің".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_40".to_string(),
                    token: "келмекпіз".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "мек".to_string(),
                        "піз".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_41".to_string(),
                    token: "келмексіздер".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "мек".to_string(),
                        "сіздер".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_42".to_string(),
                    token: "қарамақпын".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "мақ".to_string(),
                        "пын".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_43".to_string(),
                    token: "қаралмақпыз".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "мақ".to_string(),
                        "пыз".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_44".to_string(),
                    token: "келсем".to_string(),
                    expected_segments: vec!["кел".to_string(), "се".to_string(), "м".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_45".to_string(),
                    token: "келсеңіз".to_string(),
                    expected_segments: vec!["кел".to_string(), "се".to_string(), "ңіз".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_46".to_string(),
                    token: "келсеңдер".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "се".to_string(),
                        "ңдер".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_47".to_string(),
                    token: "келсек".to_string(),
                    expected_segments: vec!["кел".to_string(), "се".to_string(), "к".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_48".to_string(),
                    token: "қарасам".to_string(),
                    expected_segments: vec!["қара".to_string(), "са".to_string(), "м".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_49".to_string(),
                    token: "қаралсақ".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "са".to_string(),
                        "қ".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_50".to_string(),
                    token: "келмеспін".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "мес".to_string(),
                        "пін".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_51".to_string(),
                    token: "келмессің".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "мес".to_string(),
                        "сің".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_52".to_string(),
                    token: "келмеспіз".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "мес".to_string(),
                        "піз".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_53".to_string(),
                    token: "келмессіздер".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "мес".to_string(),
                        "сіздер".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_54".to_string(),
                    token: "қарамаспын".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "мас".to_string(),
                        "пын".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_55".to_string(),
                    token: "қаралмаспыз".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "мас".to_string(),
                        "пыз".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_56".to_string(),
                    token: "келмеймін".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "мей".to_string(),
                        "мін".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_57".to_string(),
                    token: "келмейсің".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "мей".to_string(),
                        "сің".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_58".to_string(),
                    token: "келмейміз".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "мей".to_string(),
                        "міз".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_59".to_string(),
                    token: "келмейсіздер".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "мей".to_string(),
                        "сіздер".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_60".to_string(),
                    token: "қарамайсыңдар".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "май".to_string(),
                        "сыңдар".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_61".to_string(),
                    token: "қаралмаймыз".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "май".to_string(),
                        "мыз".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_62".to_string(),
                    token: "келмегенмін".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "меген".to_string(),
                        "мін".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_63".to_string(),
                    token: "келмегенсің".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "меген".to_string(),
                        "сің".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_64".to_string(),
                    token: "келмегенбіз".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "меген".to_string(),
                        "біз".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_65".to_string(),
                    token: "келмегенсіздер".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "меген".to_string(),
                        "сіздер".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_66".to_string(),
                    token: "қаралмағанмын".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "маған".to_string(),
                        "мын".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_67".to_string(),
                    token: "қаралмағансыңдар".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "маған".to_string(),
                        "сыңдар".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_68".to_string(),
                    token: "келмейтінмін".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "мейтін".to_string(),
                        "мін".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_69".to_string(),
                    token: "келмейтінсің".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "мейтін".to_string(),
                        "сің".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_70".to_string(),
                    token: "келмейтінбіз".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "мейтін".to_string(),
                        "біз".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_71".to_string(),
                    token: "келмейтінсіздер".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "мейтін".to_string(),
                        "сіздер".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_72".to_string(),
                    token: "қарамайтынсыңдар".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "майтын".to_string(),
                        "сыңдар".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_73".to_string(),
                    token: "қаралмайтынсыздар".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "майтын".to_string(),
                        "сыздар".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_74".to_string(),
                    token: "келмесем".to_string(),
                    expected_segments: vec!["кел".to_string(), "месе".to_string(), "м".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_75".to_string(),
                    token: "келмесеңіз".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "месе".to_string(),
                        "ңіз".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_76".to_string(),
                    token: "келмесеңдер".to_string(),
                    expected_segments: vec![
                        "кел".to_string(),
                        "месе".to_string(),
                        "ңдер".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_77".to_string(),
                    token: "келмесек".to_string(),
                    expected_segments: vec!["кел".to_string(), "месе".to_string(), "к".to_string()],
                },
                TokenizerSegmentationExample {
                    id: "seg_78".to_string(),
                    token: "қарамасам".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "маса".to_string(),
                        "м".to_string(),
                    ],
                },
                TokenizerSegmentationExample {
                    id: "seg_79".to_string(),
                    token: "қаралмасақ".to_string(),
                    expected_segments: vec![
                        "қара".to_string(),
                        "л".to_string(),
                        "маса".to_string(),
                        "қ".to_string(),
                    ],
                },
            ],
        };

        let report =
            build_experiment_report(&experiment, &pack, &dataset, &test_lexicon(), &test_rules())
                .expect("experiment report");
        assert_eq!(report.sample_count, 1);
        assert_eq!(report.segmentation_example_count, 79);
        assert_eq!(report.exact_match_count, 79);
        assert!(report.failures.is_empty());
    }
}
