// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! **Phase 22 step A (2026-06-03)** — context-aware STT corrections.
//!
//! The dialog kernel is deterministic and cannot «learn» new
//! associations at runtime — every fix has to land in source so it
//! ships to every user. But each new live-REPL audit surfaces
//! Whisper STT-drift patterns that are unambiguous given the
//! surrounding context, so we codify them here as deterministic
//! pattern substitutions BEFORE the input reaches fuzzy / LM
//! rescoring / intent routing.
//!
//! ## What counts as a context correction
//!
//! Only patterns where the corrected form is grammatically valid AND
//! the original form is grammatically invalid (or pragmatically
//! absurd) in the same context. Example:
//!
//!   - «Менің атом X» → «Менің атым X»  (Whisper-drifts «атым» to
//!     «атом»; «менің атом» without a possessive «-ым» is grammatical
//!     nonsense in standalone position. Always safe to correct.)
//!
//! We do NOT codify drifts that could be either correct in context —
//! those go through fuzzy / LM rescoring instead.
//!
//! ## Why this lives in voice REPL, not adam-dialog
//!
//! The corrections are STT-drift artifacts of audio recognition;
//! the text REPL (`adam_chat`) never sees them. Keeping the patch
//! here keeps the deterministic dialog kernel STT-agnostic.

/// Apply known context-aware STT corrections. Returns the input
/// unchanged when no pattern matches.
pub fn apply(input: &str) -> String {
    let lower = input.to_lowercase();

    // Pattern 1: «менің атом ____» → «менің атым ____»
    // «атым» (my-name, possessive-1sg) Whisper-drifts to «атом» (atom).
    // The drift collides with the `Атом` chemistry definition in
    // world_core, so adam answers «Атом — заттың химиялық қасиеттерін…»
    // when the user is introducing themselves. The standalone «менің
    // атом» form (without further possessive suffix) is ungrammatical
    // in Kazakh — there's no «my atom» reading here that wouldn't
    // require «менің атомым». Always safe to correct.
    if lower.contains("менің атом ") || lower.contains("менім атом ") {
        return rewrite_word(input, "атом ", "атым ");
    }
    // Same with terminal «атом.» / «атом?» / «атом!».
    if lower.contains("менің атом")
        && (lower.ends_with("атом")
            || lower.ends_with("атом.")
            || lower.ends_with("атом?")
            || lower.ends_with("атом!"))
    {
        return rewrite_word(input, "атом", "атым");
    }

    // Pattern 2: «сенің атом» / «сенім атом» → «сенің атың» / «сенің атың».
    // Mirror of pattern 1 for second-person form. Less common but
    // appears when adam is asked "what's your name" with a slip.
    if lower.contains("сенің атом ") || lower.contains("сенім атом ") {
        return rewrite_word(input, "атом ", "атың ");
    }

    input.to_string()
}

/// Case-preserving single-token replace. Operates on lowercased
/// pattern matching against the lowercased input, applies the same
/// byte ranges back to the original-cased input.
fn rewrite_word(input: &str, needle_lower: &str, replacement: &str) -> String {
    let lower = input.to_lowercase();
    if let Some(pos) = lower.find(needle_lower) {
        let mut out = String::with_capacity(input.len());
        out.push_str(&input[..pos]);
        out.push_str(replacement);
        out.push_str(&input[pos + needle_lower.len()..]);
        return out;
    }
    input.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixes_atom_to_atym_in_name_frame() {
        let out = apply("Менің атом Дәулет.");
        assert_eq!(out, "Менің атым Дәулет.");
    }

    #[test]
    fn fixes_atom_to_atym_lowercase() {
        let out = apply("менің атом дәулет");
        assert_eq!(out, "менің атым дәулет");
    }

    #[test]
    fn fixes_terminal_atom() {
        let out = apply("Менің атом");
        assert_eq!(out, "Менің атым");
    }

    #[test]
    fn fixes_menim_typo() {
        // «менім» typo for «менің» — also common in transcripts.
        let out = apply("Менім атом Дауыл.");
        assert_eq!(out, "Менім атым Дауыл.");
    }

    #[test]
    fn leaves_real_atom_query_alone() {
        // «Атом деген не?» — real question about atom, no «менің» prefix.
        let input = "Атом деген не?";
        assert_eq!(apply(input), input);
    }

    #[test]
    fn leaves_proper_grammar_alone() {
        // Grammatically correct «менің атомым» — possessive. Don't touch.
        let input = "Менің атомым — Уран.";
        assert_eq!(apply(input), input);
    }

    #[test]
    fn fixes_senin_atom() {
        let out = apply("Сенің атом кім?");
        assert_eq!(out, "Сенің атың кім?");
    }
}
