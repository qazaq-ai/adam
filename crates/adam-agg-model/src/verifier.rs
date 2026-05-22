// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! L6 verifier for the v6.0 algebra-anchored neural composition layer.
//!
//! Implements the contract specified in
//! [`architecture_neural_v6.md`](../../../docs/architecture_neural_v6.md)
//! §3.2 (verifier check) as a reusable Rust module. The previous
//! `verifier_demo` binary stays as a CLI for ad-hoc inspection; this
//! module is what the production L5.5 → L6 wiring imports.
//!
//! ## Not to be confused with `adam_dialog::Verifier`
//!
//! There is a second, older `Verifier` type in
//! [`adam_dialog::verifier`]. The two operate at **different layers**
//! of the pipeline and have no overlap:
//!
//! | Module                          | Layer | Gates             | Input                       |
//! |---------------------------------|-------|-------------------|-----------------------------|
//! | `adam_dialog::Verifier` (v4.0.32) | L3-4 | (intent, action, belief) | dialog turn before render |
//! | `adam_agg_model::Verifier` (v6.0) | L6   | FST round-trip + facts.json | surface string from L5.5 neural composer |
//!
//! Both are kept; v6.0 wiring uses this one *in addition* to the
//! dialog-layer Verifier, not as a replacement.
//!
//! Four gates run in sequence per neural output. Gates 1-3 are
//! **always on** — they describe what a well-formed Kazakh surface
//! must look like and have nothing to do with strict vs permissive
//! grounding. Only Gate 4 (grounding) is affected by `strict`.
//!
//! 1. **Script gate.** Every character must be in the Kazakh
//!    Cyrillic alphabet (plus internal hyphen for compounds, apostrophe
//!    for hamza). If not — the model emitted Latin / mixed-script /
//!    digit-bearing junk → BLOCK as [`BlockReason::NonKazakhScript`].
//!    Closes the v6.0 Codex-review finding where Latin "blarg" passed
//!    the round-trip byte-identically and the verifier silently
//!    allowed it.
//!
//! 2. **Unk gate.** After tokenising, the leading token must be a
//!    [`MorphToken::Root`]. If the tokenizer fell back to
//!    [`MorphToken::Unk`] — the surface is not a known Kazakh word and
//!    no grounded analysis is possible → BLOCK as
//!    [`BlockReason::UnkSurface`]. Unk surfaces can byte-roundtrip
//!    (Unk preserves the input verbatim), so the FST gate alone is
//!    not enough.
//!
//! 3. **FST round-trip.** Detokenise the morpheme sequence; re-tokenise
//!    the resulting surface through [`AggTokenizer`]; require the
//!    sequence to survive the round-trip exactly. If not — the model
//!    emitted a string the deterministic morphology cannot reproduce
//!    → BLOCK as [`BlockReason::FstRoundTripFailed`].
//!
//! 4. **Factual grounding.** Extract the root from the round-tripped
//!    analysis. Look it up against an indexed set of roots / surfaces
//!    drawn from `data/retrieval/facts.json`. If missing in strict
//!    mode (default for v6 L5.5 wiring) → BLOCK as
//!    [`BlockReason::Ungrounded`]; in permissive mode → ALLOW with a
//!    `grounded == false` flag for downstream telemetry.
//!
//! Performance: ~microseconds per check; dominated by the
//! `AggTokenizer::tokenize_word` call.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use adam_agg_tokenizer::{AggTokenizer, MorphToken};
use serde::Deserialize;

/// Failure modes the verifier surfaces to the caller. Mirrors the
/// `FallbackReason` enum from architecture_neural_v6 §3.1 plus the
/// in-scope verifier-specific blocks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockReason {
    /// The candidate surface contains characters outside the Kazakh
    /// Cyrillic alphabet (Latin, digits, CJK, mixed-script). Always
    /// blocks, independent of strict mode.
    NonKazakhScript,
    /// The tokenizer fell back to [`MorphToken::Unk`] for the
    /// surface — no morphological analysis available. Unk surfaces
    /// byte-roundtrip (`Unk` carries the verbatim text) so the FST
    /// gate alone misses this case. Always blocks, independent of
    /// strict mode.
    UnkSurface,
    /// The model output, when detokenised, did not re-tokenise to
    /// the same morpheme sequence. The deterministic morphology
    /// does not recognise the surface — block.
    FstRoundTripFailed,
    /// The model output has a recognised morphological analysis but
    /// the underlying root is not present in the knowledge graph.
    /// In strict mode this is a block; in permissive mode the
    /// caller decides.
    Ungrounded,
}

/// Characters allowed in addition to the Cyrillic block: internal
/// hyphen used in compound words (e.g. «қара-көк»), and apostrophe
/// occasionally used in transliterated names. Both must appear
/// *inside* a Cyrillic-flanked surface; a bare hyphen is not a word.
fn is_kazakh_char(c: char) -> bool {
    // U+0400-U+04FF is the full Cyrillic block; the Kazakh-specific
    // letters (Ә Ғ Қ Ң Ө Ұ Ү І Һ and their lowercase) all live in
    // this range, as do the shared Russian letters.
    matches!(c, '\u{0400}'..='\u{04FF}' | '-' | '\u{2019}' | '\'')
}

/// Returns true iff every character in `surface` is in the Kazakh
/// alphabet and at least one character is Cyrillic (rules out
/// surfaces that are pure punctuation).
fn surface_is_kazakh_script(surface: &str) -> bool {
    let mut saw_cyrillic = false;
    for c in surface.chars() {
        if !is_kazakh_char(c) {
            return false;
        }
        if matches!(c, '\u{0400}'..='\u{04FF}') {
            saw_cyrillic = true;
        }
    }
    saw_cyrillic
}

/// Result of a single verifier call. PASS means downstream consumers
/// may use the surface; BLOCK names the gate that fired.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Pass {
        /// The round-tripped surface (always equals the input under
        /// PASS).
        surface: String,
        /// FST-extracted root from the surface, if any.
        root: Option<String>,
        /// Was the root present in the knowledge graph index?
        grounded: bool,
    },
    Block(BlockReason),
}

/// Verifier configuration. Constructed once per process and held in
/// the dialog runtime alongside the trained model.
pub struct Verifier {
    tokenizer: AggTokenizer,
    facts_index: HashSet<String>,
    strict: bool,
}

/// One audit record per verifier call. Append to the
/// `TurnTrace::neural_calls` field per architecture_neural_v6 §3.3.
#[derive(Debug, Clone)]
pub struct AuditRecord {
    pub input_surface: String,
    pub verdict: Verdict,
}

#[derive(Deserialize)]
struct FactsFile {
    facts: Vec<FactEntry>,
}

#[derive(Deserialize)]
struct FactEntry {
    subject: FactNoun,
    object: FactNoun,
}

#[derive(Deserialize)]
struct FactNoun {
    #[serde(default)]
    surface: String,
    #[serde(default)]
    root: String,
}

impl Verifier {
    /// Construct a verifier holding the tokenizer and a frozen
    /// snapshot of the facts index. `strict = true` makes
    /// `Ungrounded` a `BLOCK`; `strict = false` lets ungrounded
    /// outputs PASS with the `grounded = false` flag set for the
    /// caller to decide.
    ///
    /// For v6.0 L5.5 neural-composer wiring prefer
    /// [`Verifier::new_strict`] — strict-by-default is the
    /// architecturally correct posture; the explicit boolean is
    /// retained for telemetry pipelines that need to compare strict
    /// vs permissive gate behaviour on the same input stream.
    pub fn new(tokenizer: AggTokenizer, facts_index: HashSet<String>, strict: bool) -> Self {
        Self {
            tokenizer,
            facts_index,
            strict,
        }
    }

    /// Recommended constructor for v6.0 L5.5 wiring: strict grounding
    /// is on, script and Unk gates are always on. Use this everywhere
    /// in production; only fall back to [`Verifier::new`] with
    /// `strict = false` if you have an explicit reason to allow
    /// ungrounded surfaces (offline analysis, ablation, etc).
    pub fn new_strict(tokenizer: AggTokenizer, facts_index: HashSet<String>) -> Self {
        Self::new(tokenizer, facts_index, true)
    }

    /// Load the canonical facts index from `data/retrieval/facts.json`.
    /// Both subject/object surface and root are indexed.
    pub fn load_facts_index(path: impl AsRef<Path>) -> Result<HashSet<String>, std::io::Error> {
        let bytes = fs::read(path)?;
        let parsed: FactsFile = serde_json::from_slice(&bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        let mut idx: HashSet<String> = HashSet::new();
        for f in &parsed.facts {
            if !f.subject.root.is_empty() {
                idx.insert(f.subject.root.clone());
            }
            if !f.object.root.is_empty() {
                idx.insert(f.object.root.clone());
            }
            if !f.subject.surface.is_empty() {
                idx.insert(f.subject.surface.clone());
            }
            if !f.object.surface.is_empty() {
                idx.insert(f.object.surface.clone());
            }
        }
        Ok(idx)
    }

    /// Gate a candidate surface. Returns a `Verdict` and emits an
    /// `AuditRecord` for the caller to log.
    ///
    /// Gate order: script → Unk → FST round-trip → grounding. Earlier
    /// gates short-circuit later ones; the verdict carries the first
    /// gate that fired.
    pub fn check(&self, surface: &str) -> AuditRecord {
        // Gate 1: script. Must be Kazakh Cyrillic (with internal
        // hyphen / apostrophe permitted). Catches Latin nonsense like
        // "blarg" before it can reach byte-identical Unk round-trip.
        if !surface_is_kazakh_script(surface) {
            return AuditRecord {
                input_surface: surface.to_string(),
                verdict: Verdict::Block(BlockReason::NonKazakhScript),
            };
        }
        // Gate 2: Unk. Tokenise; if the leading token is Unk the
        // surface is not a recognised Kazakh word and no grounded
        // analysis can be produced. Unk round-trips byte-identically,
        // so the FST gate alone misses this.
        let tokens = self.tokenizer.tokenize_word(surface);
        let root = match tokens.first() {
            Some(MorphToken::Root { root, .. }) => Some(root.clone()),
            Some(MorphToken::Unk { .. }) | None => {
                return AuditRecord {
                    input_surface: surface.to_string(),
                    verdict: Verdict::Block(BlockReason::UnkSurface),
                };
            }
            _ => None,
        };
        // Gate 3: FST round-trip.
        let detok = self.tokenizer.detokenize_word(&tokens).ok();
        let fst_valid = matches!(&detok, Some(s) if s == surface);
        if !fst_valid {
            return AuditRecord {
                input_surface: surface.to_string(),
                verdict: Verdict::Block(BlockReason::FstRoundTripFailed),
            };
        }
        // Gate 4: factual grounding.
        let grounded = match &root {
            Some(r) => self.facts_index.contains(r) || self.facts_index.contains(surface),
            None => self.facts_index.contains(surface),
        };
        let verdict = if !grounded && self.strict {
            Verdict::Block(BlockReason::Ungrounded)
        } else {
            Verdict::Pass {
                surface: surface.to_string(),
                root,
                grounded,
            }
        };
        AuditRecord {
            input_surface: surface.to_string(),
            verdict,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adam_kernel_fst::lexicon::LexiconV1;

    /// Build a minimal Lexicon + facts-index pair for unit tests.
    fn fixture(strict: bool) -> Verifier {
        // Load the production Lexicon so the tokenizer can analyse
        // real Kazakh inputs. The path is relative to the workspace
        // root which is also `cargo test`'s working directory.
        let lex = LexiconV1::load(
            "../../data/tokenizer/segmentation_roots.json",
            "../../data/lexicon_v1/apertium_imported_roots.json",
        )
        .expect("lexicon load");
        let tokenizer = AggTokenizer::build(lex);
        // Seed a tiny facts index: only "адам" is grounded.
        let mut idx = HashSet::new();
        idx.insert("адам".to_string());
        Verifier::new(tokenizer, idx, strict)
    }

    #[test]
    fn grounded_root_passes_in_strict_mode() {
        let v = fixture(true);
        let r = v.check("адам");
        match r.verdict {
            Verdict::Pass {
                grounded, ref root, ..
            } => {
                assert!(grounded);
                assert_eq!(root.as_deref(), Some("адам"));
            }
            other => panic!("expected Pass, got {other:?}"),
        }
    }

    #[test]
    fn ungrounded_root_blocks_in_strict_mode() {
        let v = fixture(true);
        // "бала" is a real Kazakh root but not in our seeded
        // facts index → strict mode blocks.
        let r = v.check("бала");
        assert_eq!(r.verdict, Verdict::Block(BlockReason::Ungrounded));
    }

    #[test]
    fn ungrounded_root_passes_in_permissive_mode() {
        let v = fixture(false);
        let r = v.check("бала");
        match r.verdict {
            Verdict::Pass { grounded, .. } => assert!(!grounded),
            other => panic!("expected Pass with grounded=false, got {other:?}"),
        }
    }

    #[test]
    fn latin_surface_blocks_as_non_kazakh_script() {
        // **v6.0 hardening (Codex finding):** pre-hardening, Latin
        // "blarg" round-tripped byte-identically through the
        // tokenizer's Unk path and the verifier let it pass in
        // permissive mode. The script gate now blocks at the front
        // door, independent of strict / permissive.
        for v in [fixture(true), fixture(false)] {
            let r = v.check("blarg");
            assert_eq!(r.verdict, Verdict::Block(BlockReason::NonKazakhScript));
        }
    }

    #[test]
    fn mixed_script_blocks_as_non_kazakh_script() {
        let v = fixture(false);
        // Mixed Cyrillic + Latin — exactly the failure mode a
        // partially-trained neural composer would emit when one
        // morpheme drops out of vocabulary.
        let r = v.check("балaq");
        assert_eq!(r.verdict, Verdict::Block(BlockReason::NonKazakhScript));
    }

    #[test]
    fn digits_block_as_non_kazakh_script() {
        let v = fixture(false);
        let r = v.check("бала1");
        assert_eq!(r.verdict, Verdict::Block(BlockReason::NonKazakhScript));
    }

    #[test]
    fn unknown_cyrillic_surface_blocks_as_unk() {
        // **v6.0 hardening:** a string that survives the script gate
        // (all Cyrillic) but is not in the Lexicon should not
        // silently pass. The tokenizer falls back to Unk and the new
        // Unk gate catches it before grounding has a chance.
        let v = fixture(false);
        // A nonsense Cyrillic word the Lexicon will not recognise.
        let r = v.check("зщшщзщ");
        assert_eq!(r.verdict, Verdict::Block(BlockReason::UnkSurface));
    }

    #[test]
    fn new_strict_constructor_matches_strict_flag() {
        // Behavioural parity: Verifier::new_strict and
        // Verifier::new(.., .., true) must produce identical
        // verdicts on the same input.
        let lex = LexiconV1::load(
            "../../data/tokenizer/segmentation_roots.json",
            "../../data/lexicon_v1/apertium_imported_roots.json",
        )
        .expect("lexicon load");
        let tok_a = AggTokenizer::build(lex);
        let lex_b = LexiconV1::load(
            "../../data/tokenizer/segmentation_roots.json",
            "../../data/lexicon_v1/apertium_imported_roots.json",
        )
        .expect("lexicon load");
        let tok_b = AggTokenizer::build(lex_b);
        let mut idx = HashSet::new();
        idx.insert("адам".to_string());
        let v_flag = Verifier::new(tok_a, idx.clone(), true);
        let v_strict = Verifier::new_strict(tok_b, idx);
        for word in ["адам", "бала", "blarg", "бала1"] {
            assert_eq!(v_flag.check(word).verdict, v_strict.check(word).verdict);
        }
    }
}
