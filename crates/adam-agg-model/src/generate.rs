//! FST-constrained inference: generate token sequences from a trained
//! model while masking out morphologically invalid continuations.
//!
//! The neural side proposes a distribution over next-token ids; the
//! FST side keeps **only** those continuations that are valid
//! morphotactic next-steps from the current state. We pick the
//! highest-probability survivor (greedy decoding) until we hit EOS
//! or a max-length cap.
//!
//! Phase 0 simplification:
//! - The "FST state" tracked by the validator is the **canonical
//!   feature-slot order** we expose in tokenizer (root → optional
//!   derivation → number → possessive → case → predicate for nouns;
//!   root → voice → negation → tense → person for verbs). The
//!   validator allows any token whose suffix-kind position is
//!   strictly **after** the most recently emitted suffix-kind
//!   position. Bare roots / nouns vs verbs are routed by inspecting
//!   the leading Root token in the prefix.
//! - This is **not** the full morphotactic FST yet (Phase 1 will wire
//!   the real adam_kernel_fst state machine). It is, however, a
//!   strictly sound subset: every sequence the validator allows is
//!   morphotactically legal; some legal sequences may be rejected
//!   (false negatives). For Phase 0 PoC that's acceptable — we
//!   prove the hard-constrained-decoding pattern, not perfect
//!   completeness.

use burn::prelude::*;
use burn::tensor::activation::softmax;
use burn::tensor::backend::Backend;

use crate::TinyAgt;

/// Coarse classifier for tokens by their label string. Phase 0 uses
/// the same label format produced by `adam-agg-synth` (`R:` / `S:` /
/// `BOS` / `EOS` / etc.) so the validator can route by prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenRole {
    Root,
    Suffix(SuffixCategory),
    Bos,
    Eos,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SuffixCategory {
    Derivation,
    Number,
    Possessive,
    Case,
    Predicate,
    Voice,
    Negation,
    Tense,
    Person,
}

/// Classify a token label into a TokenRole. Labels follow the
/// `adam-agg-synth` convention: `R:..` root, `S:..` suffix, `BOS`,
/// `EOS`, etc.
pub fn classify_label(label: &str) -> TokenRole {
    if label == "BOS" {
        return TokenRole::Bos;
    }
    if label == "EOS" {
        return TokenRole::Eos;
    }
    if label.starts_with("R:") {
        return TokenRole::Root;
    }
    if let Some(rest) = label.strip_prefix("S:") {
        // Inspect the SuffixKind Debug-string for the category.
        if rest.contains("Derivation") {
            return TokenRole::Suffix(SuffixCategory::Derivation);
        }
        if rest.contains("Number") {
            return TokenRole::Suffix(SuffixCategory::Number);
        }
        if rest.contains("Possessive") {
            return TokenRole::Suffix(SuffixCategory::Possessive);
        }
        if rest.contains("Case") {
            return TokenRole::Suffix(SuffixCategory::Case);
        }
        if rest.contains("Predicate") {
            return TokenRole::Suffix(SuffixCategory::Predicate);
        }
        if rest.contains("Voice") {
            return TokenRole::Suffix(SuffixCategory::Voice);
        }
        if rest.contains("Negation") {
            return TokenRole::Suffix(SuffixCategory::Negation);
        }
        if rest.contains("Tense") {
            return TokenRole::Suffix(SuffixCategory::Tense);
        }
        if rest.contains("Person") {
            return TokenRole::Suffix(SuffixCategory::Person);
        }
    }
    TokenRole::Other
}

/// Phase 0 morphotactic validator: tracks which slot was last emitted
/// and whether we're on the noun or verb side. Returns `true` for
/// candidate continuations that are strictly forward in the slot order.
#[derive(Debug, Clone, Copy)]
pub struct MorphValidator {
    /// `Some(true)` after the first Root if we're on the noun side;
    /// `Some(false)` for verb side. `None` before root.
    is_noun: Option<bool>,
    last_slot: i32,
}

impl Default for MorphValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl MorphValidator {
    pub fn new() -> Self {
        Self {
            is_noun: None,
            last_slot: -1,
        }
    }

    /// Update state from an emitted token's label. Phase 0 makes a
    /// best-effort POS guess: if we see any noun-style suffix
    /// (number/possessive/case/predicate) → noun. If we see a
    /// verb-style suffix → verb. Until we see one, the root could be
    /// either; the validator allows both branches until disambiguation.
    pub fn observe(&mut self, label: &str) {
        match classify_label(label) {
            TokenRole::Bos => {}
            TokenRole::Root => {
                self.last_slot = 0;
            }
            TokenRole::Eos => {
                self.last_slot = 999;
            }
            TokenRole::Suffix(cat) => {
                let (slot, side) = match cat {
                    SuffixCategory::Derivation => (1, None),
                    SuffixCategory::Number => (2, Some(true)),
                    SuffixCategory::Possessive => (3, Some(true)),
                    SuffixCategory::Case => (4, Some(true)),
                    SuffixCategory::Predicate => (5, Some(true)),
                    SuffixCategory::Voice => (1, Some(false)),
                    SuffixCategory::Negation => (2, Some(false)),
                    SuffixCategory::Tense => (3, Some(false)),
                    SuffixCategory::Person => (4, Some(false)),
                };
                if let Some(noun) = side {
                    self.is_noun = Some(noun);
                }
                self.last_slot = slot;
            }
            TokenRole::Other => {}
        }
    }

    /// Compact `u8` encoding of the current validator state. Used by
    /// the algebraic-loss training path to look up an invalid-token
    /// mask per training position without re-running the validator
    /// inside the autograd graph.
    ///
    /// Encoding: `is_noun_idx * 8 + slot_idx`. There are 24 possible
    /// states (3 × 8) in Phase 0.
    pub fn state_id(&self) -> u8 {
        let is_noun_idx: u8 = match self.is_noun {
            None => 0,
            Some(true) => 1,
            Some(false) => 2,
        };
        let slot_idx: u8 = match self.last_slot {
            -1 => 0,
            0 => 1,
            1 => 2,
            2 => 3,
            3 => 4,
            4 => 5,
            5 => 6,
            999 => 7,
            _ => 0,
        };
        is_noun_idx * 8 + slot_idx
    }

    /// Reconstruct a validator from its compact state id. Inverse of
    /// [`state_id`].
    pub fn from_state_id(id: u8) -> Self {
        let is_noun_idx = id / 8;
        let slot_idx = id % 8;
        let is_noun = match is_noun_idx {
            1 => Some(true),
            2 => Some(false),
            _ => None,
        };
        let last_slot = match slot_idx {
            0 => -1,
            1 => 0,
            2 => 1,
            3 => 2,
            4 => 3,
            5 => 4,
            6 => 5,
            7 => 999,
            _ => -1,
        };
        Self { is_noun, last_slot }
    }

    /// Number of distinct validator states (3 × 8).
    pub const NUM_STATES: u8 = 24;

    /// Is this candidate token a valid next step given current state?
    pub fn allows(&self, candidate_label: &str) -> bool {
        match classify_label(candidate_label) {
            TokenRole::Bos => false,               // BOS only at the very start.
            TokenRole::Eos => self.last_slot >= 0, // EOS only after at least a Root.
            TokenRole::Root => self.last_slot < 0, // Root only at position 0.
            TokenRole::Suffix(cat) => {
                let (slot, side) = suffix_slot(cat);
                // Slot must be strictly forward.
                if slot <= self.last_slot {
                    return false;
                }
                // POS consistency: once a side is committed, suffix
                // candidates must match.
                match (self.is_noun, side) {
                    (Some(noun), Some(s)) => noun == s,
                    _ => true, // not yet committed → allow both
                }
            }
            TokenRole::Other => false,
        }
    }
}

fn suffix_slot(cat: SuffixCategory) -> (i32, Option<bool>) {
    match cat {
        SuffixCategory::Derivation => (1, None),
        SuffixCategory::Number => (2, Some(true)),
        SuffixCategory::Possessive => (3, Some(true)),
        SuffixCategory::Case => (4, Some(true)),
        SuffixCategory::Predicate => (5, Some(true)),
        SuffixCategory::Voice => (1, Some(false)),
        SuffixCategory::Negation => (2, Some(false)),
        SuffixCategory::Tense => (3, Some(false)),
        SuffixCategory::Person => (4, Some(false)),
    }
}

/// Greedy FST-constrained generation. Starts from `prefix` token ids
/// and extends with the highest-probability **valid** continuation at
/// each step.
pub fn generate_constrained<B: Backend>(
    model: &TinyAgt<B>,
    compact_to_label: &[String],
    prefix: &[i64],
    max_new_tokens: usize,
    device: &B::Device,
) -> Vec<i64> {
    let mut tokens: Vec<i64> = prefix.to_vec();
    let mut validator = MorphValidator::new();
    for &t in prefix {
        if let Some(label) = compact_to_label.get(t as usize) {
            validator.observe(label);
        }
    }

    let max_seq_len = model.max_seq_len();

    for _ in 0..max_new_tokens {
        // Truncate left if we exceed max_seq_len.
        let start = tokens.len().saturating_sub(max_seq_len);
        let window: Vec<i64> = tokens[start..].to_vec();
        let seq_len = window.len();
        let input: Tensor<B, 2, Int> =
            Tensor::from_data(burn::tensor::TensorData::new(window, [1, seq_len]), device);

        let logits = model.forward(input); // [1, seq_len, vocab]
        // Take logits at the final position.
        let last = logits.slice([0..1, (seq_len - 1)..seq_len, 0..model.vocab_size()]);
        let last_2d = last.squeeze::<2>(1); // [1, vocab]
        let probs = softmax(last_2d, 1);
        let probs_vec: Vec<f32> = probs.into_data().as_slice::<f32>().unwrap_or(&[]).to_vec();

        // Pick highest-probability valid token.
        let mut best_id: Option<usize> = None;
        let mut best_p: f32 = f32::NEG_INFINITY;
        for (id, &p) in probs_vec.iter().enumerate() {
            let Some(label) = compact_to_label.get(id) else {
                continue;
            };
            if validator.allows(label) && p > best_p {
                best_p = p;
                best_id = Some(id);
            }
        }

        let Some(id) = best_id else {
            // Validator blocks everything → stop. (In practice
            // shouldn't happen because EOS is always allowed after
            // root; but guard anyway.)
            break;
        };
        tokens.push(id as i64);
        if let Some(label) = compact_to_label.get(id) {
            validator.observe(label);
            if classify_label(label) == TokenRole::Eos {
                break;
            }
        }
    }
    tokens
}

/// Unconstrained greedy generation (no FST mask). For comparison.
pub fn generate_unconstrained<B: Backend>(
    model: &TinyAgt<B>,
    prefix: &[i64],
    max_new_tokens: usize,
    device: &B::Device,
) -> Vec<i64> {
    let mut tokens: Vec<i64> = prefix.to_vec();
    let max_seq_len = model.max_seq_len();
    for _ in 0..max_new_tokens {
        let start = tokens.len().saturating_sub(max_seq_len);
        let window: Vec<i64> = tokens[start..].to_vec();
        let seq_len = window.len();
        let input: Tensor<B, 2, Int> =
            Tensor::from_data(burn::tensor::TensorData::new(window, [1, seq_len]), device);
        let logits = model.forward(input);
        let last = logits.slice([0..1, (seq_len - 1)..seq_len, 0..model.vocab_size()]);
        let last_2d = last.squeeze::<2>(1);
        let probs_vec: Vec<f32> = last_2d
            .into_data()
            .as_slice::<f32>()
            .unwrap_or(&[])
            .to_vec();
        let mut best_id: usize = 0;
        let mut best_p: f32 = f32::NEG_INFINITY;
        for (id, &p) in probs_vec.iter().enumerate() {
            if p > best_p {
                best_p = p;
                best_id = id;
            }
        }
        tokens.push(best_id as i64);
        if best_id == 2 {
            break; // EOS literal
        }
    }
    tokens
}

/// Build a `[NUM_STATES][vocab_size]` table of *invalid-token* flags.
/// `table[s][v] == 1.0` means: from validator state `s`, predicting
/// token `v` is morphotactically illegal. The table is used as a
/// dense penalty tensor during training (algebraic loss).
///
/// `vocab_size` must match the model's vocabulary (which may exceed
/// `compact_to_label.len()` because of reserved-but-unmapped slots).
/// Any id beyond `compact_to_label.len()` is treated as invalid for
/// every state (it has no label and thus no defined role).
pub fn build_invalid_mask_table(compact_to_label: &[String], vocab_size: usize) -> Vec<Vec<f32>> {
    (0..MorphValidator::NUM_STATES)
        .map(|s| {
            let v = MorphValidator::from_state_id(s);
            let mut row = vec![1.0f32; vocab_size];
            for (id, label) in compact_to_label.iter().enumerate() {
                if id >= vocab_size {
                    break;
                }
                row[id] = if v.allows(label) { 0.0 } else { 1.0 };
            }
            row
        })
        .collect()
}

/// For a single training sequence, compute the validator state at each
/// position **before** predicting that position. The result has the
/// same length as `seq`. `state_ids[i]` is the validator state after
/// observing `seq[0..i]` (i.e. when the model is being asked to
/// predict `seq[i]`).
pub fn compute_state_ids(seq: &[i64], compact_to_label: &[String]) -> Vec<u8> {
    let mut v = MorphValidator::new();
    let mut out = Vec::with_capacity(seq.len());
    for &tok in seq {
        out.push(v.state_id());
        if let Some(label) = compact_to_label.get(tok as usize) {
            v.observe(label);
        }
    }
    out
}

/// Check a label sequence against the same morphotactic validator
/// used during generation. Returns the number of valid transitions
/// (out of `labels.len() - 1`).
pub fn count_valid_transitions(labels: &[&str]) -> (usize, usize) {
    let mut validator = MorphValidator::new();
    let mut valid = 0usize;
    let mut total = 0usize;
    for (i, l) in labels.iter().enumerate() {
        if i == 0 {
            validator.observe(l);
            continue;
        }
        total += 1;
        if validator.allows(l) {
            valid += 1;
            validator.observe(l);
        }
    }
    (valid, total)
}

/// A full word is **morphotactically valid** if every transition is
/// legal under [`MorphValidator`] AND the word terminates in an
/// admissible state — i.e. (a) it ends in `EOS`, (b) the last
/// observed slot is `>= 0` (at least a Root has been emitted), and
/// (c) no `Other` tokens snuck in.
///
/// This is a strictly stronger metric than per-transition validity:
/// a sequence with 4/5 valid transitions is still **not** a valid
/// word.
pub fn is_valid_word(labels: &[&str]) -> bool {
    if labels.is_empty() {
        return false;
    }
    let mut v = MorphValidator::new();
    let mut saw_root = false;
    let mut saw_eos = false;
    for (i, l) in labels.iter().enumerate() {
        match classify_label(l) {
            TokenRole::Bos => {
                if i != 0 {
                    return false;
                }
            }
            TokenRole::Root => {
                if !v.allows(l) {
                    return false;
                }
                saw_root = true;
            }
            TokenRole::Suffix(_) => {
                if !v.allows(l) {
                    return false;
                }
            }
            TokenRole::Eos => {
                if !v.allows(l) {
                    return false;
                }
                saw_eos = true;
            }
            TokenRole::Other => return false,
        }
        v.observe(l);
        if saw_eos {
            break;
        }
    }
    saw_root && saw_eos
}

/// Beam-search FST-constrained generation. Maintains `beam_width`
/// hypotheses ranked by cumulative log-prob; at each step expands
/// every hypothesis with all morpho-valid continuations, then prunes
/// back to `beam_width`. Returns the best-scoring completed sequence.
pub fn generate_constrained_beam<B: Backend>(
    model: &TinyAgt<B>,
    compact_to_label: &[String],
    prefix: &[i64],
    max_new_tokens: usize,
    beam_width: usize,
    device: &B::Device,
) -> Vec<i64> {
    #[derive(Clone)]
    struct Hyp {
        tokens: Vec<i64>,
        validator: MorphValidator,
        logp: f32,
        done: bool,
    }

    let mut initial_validator = MorphValidator::new();
    for &t in prefix {
        if let Some(label) = compact_to_label.get(t as usize) {
            initial_validator.observe(label);
        }
    }
    let mut beam: Vec<Hyp> = vec![Hyp {
        tokens: prefix.to_vec(),
        validator: initial_validator,
        logp: 0.0,
        done: false,
    }];
    let max_seq_len = model.max_seq_len();

    for _ in 0..max_new_tokens {
        let mut next: Vec<Hyp> = Vec::new();
        for h in &beam {
            if h.done {
                next.push(h.clone());
                continue;
            }
            let start = h.tokens.len().saturating_sub(max_seq_len);
            let window: Vec<i64> = h.tokens[start..].to_vec();
            let seq_len = window.len();
            let input: Tensor<B, 2, Int> =
                Tensor::from_data(burn::tensor::TensorData::new(window, [1, seq_len]), device);
            let logits = model.forward(input);
            let last = logits.slice([0..1, (seq_len - 1)..seq_len, 0..model.vocab_size()]);
            let last_2d = last.squeeze::<2>(1);
            let probs = softmax(last_2d, 1);
            let probs_vec: Vec<f32> = probs.into_data().as_slice::<f32>().unwrap_or(&[]).to_vec();

            // Take all valid candidates (no top-K cut — vocab is moderate).
            for (id, &p) in probs_vec.iter().enumerate() {
                let Some(label) = compact_to_label.get(id) else {
                    continue;
                };
                if !h.validator.allows(label) {
                    continue;
                }
                let logp_add = if p > 0.0 { p.ln() } else { -1e9 };
                let mut new_tokens = h.tokens.clone();
                new_tokens.push(id as i64);
                let mut new_validator = h.validator;
                new_validator.observe(label);
                let done = classify_label(label) == TokenRole::Eos;
                next.push(Hyp {
                    tokens: new_tokens,
                    validator: new_validator,
                    logp: h.logp + logp_add,
                    done,
                });
            }
        }
        if next.is_empty() {
            break;
        }
        next.sort_by(|a, b| {
            b.logp
                .partial_cmp(&a.logp)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        next.truncate(beam_width);
        beam = next;
        if beam.iter().all(|h| h.done) {
            break;
        }
    }
    beam.sort_by(|a, b| {
        b.logp
            .partial_cmp(&a.logp)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    beam.into_iter()
        .next()
        .map(|h| h.tokens)
        .unwrap_or_else(|| prefix.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_recognises_known_labels() {
        assert_eq!(classify_label("BOS"), TokenRole::Bos);
        assert_eq!(classify_label("EOS"), TokenRole::Eos);
        assert_eq!(classify_label("R:бала"), TokenRole::Root);
        assert!(matches!(
            classify_label("S:Number(Plural)"),
            TokenRole::Suffix(SuffixCategory::Number)
        ));
        assert!(matches!(
            classify_label("S:Case(Dative)"),
            TokenRole::Suffix(SuffixCategory::Case)
        ));
    }

    #[test]
    fn validator_rejects_root_after_root() {
        let mut v = MorphValidator::new();
        v.observe("R:бала");
        assert!(!v.allows("R:ат"));
    }

    #[test]
    fn validator_allows_canonical_noun_order() {
        let mut v = MorphValidator::new();
        for tok in [
            "R:бала",
            "S:Number(Plural)",
            "S:Possessive(P1Sg)",
            "S:Case(Dative)",
        ] {
            assert!(v.allows(tok), "should allow {} from current state", tok);
            v.observe(tok);
        }
    }

    #[test]
    fn validator_rejects_reversed_order() {
        let mut v = MorphValidator::new();
        v.observe("R:бала");
        v.observe("S:Case(Dative)");
        // Possessive (slot 3) after Case (slot 4) is invalid.
        assert!(!v.allows("S:Possessive(P1Sg)"));
    }

    #[test]
    fn validator_rejects_verb_suffix_after_noun_suffix() {
        let mut v = MorphValidator::new();
        v.observe("R:бала");
        v.observe("S:Number(Plural)");
        // Now committed to noun side.
        assert!(!v.allows("S:Voice(Active)"));
    }

    #[test]
    fn state_id_round_trips() {
        for id in 0..MorphValidator::NUM_STATES {
            let v = MorphValidator::from_state_id(id);
            assert_eq!(v.state_id(), id, "round-trip mismatch at state {}", id);
        }
    }

    #[test]
    fn invalid_mask_table_has_expected_shape() {
        let labels: Vec<String> = vec![
            "<unk>".into(),
            "BOS".into(),
            "EOS".into(),
            "R:бала".into(),
            "S:Number(Plural)".into(),
            "S:Case(Dative)".into(),
            "S:Voice(Active)".into(),
        ];
        let vocab = labels.len();
        let table = build_invalid_mask_table(&labels, vocab);
        assert_eq!(table.len(), MorphValidator::NUM_STATES as usize);
        for row in &table {
            assert_eq!(row.len(), vocab);
        }
        // At the initial state (id=0: no root yet) the Root entry must be valid.
        let initial = &table[0];
        let root_idx = 3;
        assert_eq!(initial[root_idx], 0.0, "Root must be allowed initially");
        // EOS must NOT be valid initially (no root yet).
        let eos_idx = 2;
        assert_eq!(initial[eos_idx], 1.0, "EOS not allowed before any root");
    }

    #[test]
    fn is_valid_word_recognises_canonical_inflection() {
        let labels = vec![
            "R:бала",
            "S:Number(Plural)",
            "S:Possessive(P1Sg)",
            "S:Case(Dative)",
            "EOS",
        ];
        assert!(is_valid_word(&labels));
    }

    #[test]
    fn is_valid_word_rejects_no_root() {
        let labels = vec!["S:Number(Plural)", "EOS"];
        assert!(!is_valid_word(&labels));
    }

    #[test]
    fn is_valid_word_rejects_no_eos() {
        let labels = vec!["R:бала", "S:Number(Plural)"];
        assert!(!is_valid_word(&labels));
    }

    #[test]
    fn is_valid_word_rejects_reversed_order() {
        let labels = vec![
            "R:бала",
            "S:Case(Dative)",
            "S:Number(Plural)", // backward — illegal
            "EOS",
        ];
        assert!(!is_valid_word(&labels));
    }

    #[test]
    fn count_valid_transitions_on_synthetic_kazakh_inflection() {
        let labels = vec![
            "R:бала",
            "S:Number(Plural)",
            "S:Possessive(P1Sg)",
            "S:Case(Dative)",
            "EOS",
        ];
        let (v, t) = count_valid_transitions(&labels);
        assert_eq!(v, t, "all transitions should be valid: {}/{}", v, t);
    }
}
