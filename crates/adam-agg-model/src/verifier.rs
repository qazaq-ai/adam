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
//! Two gates run in sequence per neural output:
//!
//! 1. **FST round-trip.** Detokenise the morpheme sequence; re-tokenise
//!    the resulting surface through [`AggTokenizer`]; require the
//!    sequence to survive the round-trip exactly. If not — the model
//!    emitted a string the deterministic morphology cannot reproduce
//!    → BLOCK.
//!
//! 2. **Factual grounding.** Extract the root from the round-tripped
//!    analysis. Look it up against an indexed set of roots / surfaces
//!    drawn from `data/retrieval/facts.json`. If missing in strict
//!    mode → BLOCK; in permissive mode (default) → ALLOW with a
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
    pub fn new(tokenizer: AggTokenizer, facts_index: HashSet<String>, strict: bool) -> Self {
        Self {
            tokenizer,
            facts_index,
            strict,
        }
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
    pub fn check(&self, surface: &str) -> AuditRecord {
        // Gate 1: FST round-trip.
        let tokens = self.tokenizer.tokenize_word(surface);
        let root = match tokens.first() {
            Some(MorphToken::Root { root, .. }) => Some(root.clone()),
            _ => None,
        };
        let detok = self.tokenizer.detokenize_word(&tokens).ok();
        let fst_valid = matches!(&detok, Some(s) if s == surface);
        if !fst_valid {
            return AuditRecord {
                input_surface: surface.to_string(),
                verdict: Verdict::Block(BlockReason::FstRoundTripFailed),
            };
        }
        // Gate 2: factual grounding.
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
    fn nonsense_blocks_on_round_trip() {
        let v = fixture(false);
        // Latin script — not a valid Kazakh surface. The tokenizer
        // returns Unk; detokenise yields the same string; round-trip
        // succeeds at the byte level. Strict mode would block on
        // ungrounded; permissive would pass with grounded=false.
        // This documents the current behaviour: round-trip is a
        // morphology gate, not a script gate.
        let r = v.check("blarg");
        match r.verdict {
            Verdict::Pass { ref root, .. } => assert!(root.is_none()),
            other => panic!("expected permissive Pass, got {other:?}"),
        }
    }
}
