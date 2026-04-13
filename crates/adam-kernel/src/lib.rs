use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ── Identity ──────────────────────────────────────────────────────────────────

pub const MODEL_NAME: &str = "adam";
pub const MODEL_SCOPE: &str = "kazakh-first text model foundation";
pub const SUPPORTED_LANGUAGE: &str = "kazakh";
pub const SUPPORTED_SCRIPT: &str = "cyrillic";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelIdentity {
    pub name: String,
    pub scope: String,
    pub language: String,
    pub script: String,
    pub phase: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundationPrinciples {
    pub kazakh_only: bool,
    pub cyrillic_only: bool,
    pub corpus_first: bool,
    pub eval_first: bool,
    pub no_multilingual_scope: bool,
}

impl Default for ModelIdentity {
    fn default() -> Self {
        Self {
            name: MODEL_NAME.to_string(),
            scope: MODEL_SCOPE.to_string(),
            language: SUPPORTED_LANGUAGE.to_string(),
            script: SUPPORTED_SCRIPT.to_string(),
            phase: "foundation".to_string(),
        }
    }
}

impl Default for FoundationPrinciples {
    fn default() -> Self {
        Self {
            kazakh_only: true,
            cyrillic_only: true,
            corpus_first: true,
            eval_first: true,
            no_multilingual_scope: true,
        }
    }
}

// ── Kernel errors ─────────────────────────────────────────────────────────────

#[derive(Debug, Error, PartialEq, Eq)]
pub enum KernelError {
    #[error("language must be kazakh")]
    NonKazakhLanguage,
    #[error("script must be cyrillic")]
    NonCyrillicScript,
    #[error("segmentation lexicon roots must not be empty")]
    EmptySegmentationLexicon,
    #[error("segmentation root ids must be unique and non-empty")]
    InvalidSegmentationRootId,
    #[error("segmentation rule set rules must not be empty")]
    EmptySegmentationRuleSet,
    #[error("segmentation suffix rule ids must be unique and non-empty")]
    InvalidSegmentationRuleId,
    #[error("segmentation rule constraints must not be empty")]
    EmptySegmentationRuleConstraint,
}

// ── Morphological types ───────────────────────────────────────────────────────

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

impl SegmentationLexicon {
    pub fn validate(&self) -> Result<(), KernelError> {
        if self.target_language != "kazakh" {
            return Err(KernelError::NonKazakhLanguage);
        }

        if self.script != "cyrillic" {
            return Err(KernelError::NonCyrillicScript);
        }

        if self.roots.is_empty() {
            return Err(KernelError::EmptySegmentationLexicon);
        }

        let mut seen = HashSet::new();
        for root in &self.roots {
            if root.id.trim().is_empty()
                || root.root.trim().is_empty()
                || contains_latin(&root.root)
                || !seen.insert(root.id.clone())
            {
                return Err(KernelError::InvalidSegmentationRootId);
            }
        }

        Ok(())
    }
}

impl SegmentationRuleSet {
    pub fn validate(&self) -> Result<(), KernelError> {
        if self.target_language != "kazakh" {
            return Err(KernelError::NonKazakhLanguage);
        }

        if self.script != "cyrillic" {
            return Err(KernelError::NonCyrillicScript);
        }

        if self.rules.is_empty() {
            return Err(KernelError::EmptySegmentationRuleSet);
        }

        let mut seen = HashSet::new();
        for rule in &self.rules {
            if rule.id.trim().is_empty()
                || rule.form.trim().is_empty()
                || rule.label.trim().is_empty()
                || contains_latin(&rule.form)
                || !seen.insert(rule.id.clone())
            {
                return Err(KernelError::InvalidSegmentationRuleId);
            }

            if rule.allowed_harmonies.is_empty() || rule.allowed_final_sound_classes.is_empty() {
                return Err(KernelError::EmptySegmentationRuleConstraint);
            }
        }

        Ok(())
    }
}

// ── FSM parse types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeterministicSegmentationParse {
    pub part_of_speech: SegmentationPartOfSpeech,
    pub segments: Vec<String>,
    pub labels: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SuffixParse {
    segments: Vec<String>,
    labels: Vec<String>,
}

// ── Segmentation functions ────────────────────────────────────────────────────

pub fn deterministic_segment_token(
    token: &str,
    lexicon: &SegmentationLexicon,
    rules: &SegmentationRuleSet,
) -> Option<Vec<String>> {
    deterministic_segment_parse(token, lexicon, rules).map(|parse| parse.segments)
}

pub fn deterministic_segment_parse(
    token: &str,
    lexicon: &SegmentationLexicon,
    rules: &SegmentationRuleSet,
) -> Option<DeterministicSegmentationParse> {
    let mut candidates = lexicon
        .roots
        .iter()
        .filter(|root| token.starts_with(&root.root))
        .filter_map(|root| {
            let remaining = &token[root.root.len()..];
            if remaining.is_empty() {
                return Some(DeterministicSegmentationParse {
                    part_of_speech: root.part_of_speech.clone(),
                    segments: vec![root.root.clone()],
                    labels: Vec::new(),
                });
            }

            let mut parses = Vec::new();
            let mut suffix_segments = Vec::new();
            let mut suffix_labels = Vec::new();
            collect_suffix_parses(
                remaining,
                &root.part_of_speech,
                SegmentationState::Stem,
                None,
                &root.vowel_harmony,
                &root.final_sound_class,
                rules,
                &mut suffix_segments,
                &mut suffix_labels,
                &mut parses,
            );
            parses.sort_by(|left, right| {
                left.segments
                    .cmp(&right.segments)
                    .then(left.labels.cmp(&right.labels))
            });
            parses.dedup_by(|left, right| {
                left.segments == right.segments && left.labels == right.labels
            });

            if parses.len() == 1 {
                let parse = parses.pop().expect("single parse");
                let mut segments = vec![root.root.clone()];
                segments.extend(parse.segments);
                Some(DeterministicSegmentationParse {
                    part_of_speech: root.part_of_speech.clone(),
                    segments,
                    labels: parse.labels,
                })
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| {
        left.segments
            .cmp(&right.segments)
            .then(left.labels.cmp(&right.labels))
    });
    candidates
        .dedup_by(|left, right| left.segments == right.segments && left.labels == right.labels);

    if candidates.len() == 1 {
        candidates.pop()
    } else {
        None
    }
}

pub fn expected_segmentation_parse(
    expected_segments: &[String],
    lexicon: &SegmentationLexicon,
    rules: &SegmentationRuleSet,
) -> Option<DeterministicSegmentationParse> {
    let (root_segment, suffix_segments) = expected_segments.split_first()?;
    let mut candidates = lexicon
        .roots
        .iter()
        .filter(|root| &root.root == root_segment)
        .filter_map(|root| {
            if suffix_segments.is_empty() {
                return Some(DeterministicSegmentationParse {
                    part_of_speech: root.part_of_speech.clone(),
                    segments: expected_segments.to_vec(),
                    labels: Vec::new(),
                });
            }

            let mut labels = Vec::new();
            let mut parses = Vec::new();
            collect_expected_labels(
                suffix_segments,
                &root.part_of_speech,
                SegmentationState::Stem,
                None,
                &root.vowel_harmony,
                &root.final_sound_class,
                rules,
                &mut labels,
                &mut parses,
            );

            parses.sort();
            parses.dedup();

            if parses.len() == 1 {
                Some(DeterministicSegmentationParse {
                    part_of_speech: root.part_of_speech.clone(),
                    segments: expected_segments.to_vec(),
                    labels: parses.pop().expect("single expected parse"),
                })
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| {
        left.segments
            .cmp(&right.segments)
            .then(left.labels.cmp(&right.labels))
    });
    candidates
        .dedup_by(|left, right| left.segments == right.segments && left.labels == right.labels);

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
    current_segments: &mut Vec<String>,
    current_labels: &mut Vec<String>,
    parses: &mut Vec<SuffixParse>,
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
        current_segments.push(rule.form.clone());
        current_labels.push(rule.label.clone());
        let next_final_sound_class = classify_final_sound(&rule.form);

        if next_remaining.is_empty() {
            parses.push(SuffixParse {
                segments: current_segments.clone(),
                labels: current_labels.clone(),
            });
        } else if !rule.terminal {
            collect_suffix_parses(
                next_remaining,
                part_of_speech,
                rule.to_state.clone(),
                Some(&rule.label),
                harmony,
                &next_final_sound_class,
                rules,
                current_segments,
                current_labels,
                parses,
            );
        }

        current_segments.pop();
        current_labels.pop();
    }
}

fn collect_expected_labels(
    remaining_segments: &[String],
    part_of_speech: &SegmentationPartOfSpeech,
    state: SegmentationState,
    previous_label: Option<&str>,
    harmony: &VowelHarmony,
    final_sound_class: &FinalSoundClass,
    rules: &SegmentationRuleSet,
    current_labels: &mut Vec<String>,
    parses: &mut Vec<Vec<String>>,
) {
    let Some(next_segment) = remaining_segments.first() else {
        parses.push(current_labels.clone());
        return;
    };

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
            && &rule.form == next_segment
    }) {
        current_labels.push(rule.label.clone());
        let next_final_sound_class = classify_final_sound(&rule.form);

        if remaining_segments.len() == 1 {
            parses.push(current_labels.clone());
        } else if !rule.terminal {
            collect_expected_labels(
                &remaining_segments[1..],
                part_of_speech,
                rule.to_state.clone(),
                Some(&rule.label),
                harmony,
                &next_final_sound_class,
                rules,
                current_labels,
                parses,
            );
        }

        current_labels.pop();
    }
}

// ── Phonological utilities ────────────────────────────────────────────────────

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

pub fn contains_latin(value: &str) -> bool {
    value.chars().any(|ch| ch.is_ascii_alphabetic())
}

pub fn normalize_text(text: &str) -> String {
    text.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::{FoundationPrinciples, ModelIdentity};

    #[test]
    fn exposes_kazakh_first_identity() {
        let identity = ModelIdentity::default();

        assert_eq!(identity.name, "adam");
        assert_eq!(identity.language, "kazakh");
        assert_eq!(identity.script, "cyrillic");
        assert_eq!(identity.phase, "foundation");
    }

    #[test]
    fn foundation_principles_are_strict() {
        let principles = FoundationPrinciples::default();

        assert!(principles.kazakh_only);
        assert!(principles.cyrillic_only);
        assert!(principles.corpus_first);
        assert!(principles.eval_first);
    }
}
