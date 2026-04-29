//! v4.6.0 — Discourse-anaphora resolution.
//!
//! Kazakh (like English) uses **discourse demonstratives** to refer
//! back to a topic introduced in a previous turn. The most common
//! standalone forms:
//!
//! | Surface | Gloss | Resolves to |
//! |---|---|---|
//! | `онда` | "in it / there" | previous turn's topic, locative |
//! | `сонда` | "in that / there" | same |
//! | `осында` | "in this / here" | same |
//! | `мұнда` | "in this / here" | same |
//! | `бұнда` | "in this" | same |
//! | `одан` | "from it" | same, ablative |
//! | `содан` | "from that" | same |
//! | `бұдан` | "from this" | same |
//! | `осыдан` | "from this" | same |
//!
//! Pre-v4.6.0 these surfaces were in `NOT_A_TOPIC` to suppress the
//! FST misanalysis (e.g. `онда` parsing as `он + Locative`), which
//! prevented adam from picking up `Он` as a phantom topic. Good as
//! a defence; but it also meant `онда` carried **no semantic
//! signal** — the system silently lost the discourse anaphor.
//!
//! This module adds the missing signal by tracking the **last
//! query topic** across turns. When the user's input contains a
//! discourse anaphor (per the surface list above) and `best_noun_hint`
//! returned None for the current turn, the conversation layer
//! looks up the previous turn's topic and reuses it. So:
//!
//! ```text
//! T1: «Қазақстан туралы не білесіз?»  → topic = қазақстан, surfaced as fact
//! T2: «Ал онда қанша аймақ бар?»     → topic = (anaphora) → қазақстан
//!                                       → answers about Kazakhstan, not «он»
//! ```
//!
//! Implementation is intentionally simple — single-slot LRU. No
//! attempt to model coreference chains, multiple referents, or
//! discourse stacks. The single most-recent topic covers the
//! 80%-case observed in real REPL traces; richer modelling is
//! deferred to a future minor.

/// Discourse-anaphor surface forms (lowercased) that signal
/// "refer to the previous turn's topic". Kept aligned with the
/// `NOT_A_TOPIC` entries added in v4.3.5 (which suppress the
/// FST misanalysis side) — these are the FORMS that carry
/// anaphoric meaning.
const DISCOURSE_ANAPHORS: &[&str] = &[
    "онда",
    "сонда",
    "осында",
    "мұнда",
    "бұнда",
    "одан",
    "содан",
    "бұдан",
    "осыдан",
];

/// Returns `true` if any whitespace-separated lowercase token of
/// the input matches a known discourse anaphor. The check is
/// intentionally surface-level — we don't want to lean on the
/// FST here because the FST's analysis of these forms is exactly
/// what `NOT_A_TOPIC` suppresses.
pub fn input_contains_discourse_anaphor(input: &str) -> bool {
    let lower = input.to_lowercase();
    lower
        .split(|c: char| !c.is_alphabetic())
        .any(|word| DISCOURSE_ANAPHORS.contains(&word))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_standalone_discourse_anaphors() {
        for input in [
            "Ал онда қанша аймақ бар?",
            "Сонда не бар?",
            "Осында тұрамын",
            "Мұнда барлығы жақсы",
            "Бұнда да солай",
            "Одан кейін не?",
            "Содан соң айт",
            "Бұдан шықпайды",
            "Осыдан бастаймыз",
        ] {
            assert!(
                input_contains_discourse_anaphor(input),
                "input {input:?} must register as carrying a discourse anaphor"
            );
        }
    }

    #[test]
    fn does_not_match_unrelated_words() {
        for input in ["Қазақстан туралы не білесіз?", "Алматы — қала", "Сәлем"]
        {
            assert!(
                !input_contains_discourse_anaphor(input),
                "input {input:?} must NOT register as discourse anaphor"
            );
        }
    }

    #[test]
    fn handles_punctuation_around_anaphor() {
        assert!(input_contains_discourse_anaphor("Ал онда?"));
        assert!(input_contains_discourse_anaphor("онда, иә"));
        assert!(input_contains_discourse_anaphor("онда."));
    }
}
