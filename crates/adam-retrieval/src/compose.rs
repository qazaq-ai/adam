//! In-sample composition (v1.9.0, option B territory).
//!
//! Unlike v1.8.0's frame-only composition — where the retrieved quote
//! stays byte-identical to the corpus — this module goes **inside**
//! the quote and substitutes tokens with the user's session values.
//! That is where the v1.x-so-far "no fabrication" guarantee narrows:
//! the composed output is a sentence that does not exist in any source
//! pack. Grammaticality is still FST-guaranteed, but semantic
//! truthfulness is NOT.
//!
//! For the v1.9.0 MVP the swap is deliberately limited to **cities**,
//! the safest slot category:
//!
//!   - a closed, editorially-curated list of ~20 Kazakh cities
//!     (`PLACE_NAMES`) is the only root set we recognise as swappable;
//!   - the user's proposed replacement city must also be in that list
//!     — otherwise the FST can't re-synthesise it reliably;
//!   - quotes containing a 4-digit year (1500–2100) are refused
//!     outright, to keep biographical / historical contexts safe
//!     (we must not rewrite "Абай 1845 жылы Қарқаралыда туған" into
//!     "Абай 1845 жылы Алматыда туған").
//!
//! Names, numerals, and free-form proper nouns are **not** swappable
//! in v1.9.0. Those categories need additional semantic guards that
//! belong in a later release.
//!
//! The returned [`Composition`] carries full provenance — every
//! [`Swap`] names the original surface form, the replacement, the
//! shared FST feature bundle, and the user's root. Downstream
//! consumers (dialog `--trace`, audit logs) can reconstruct every
//! decision.

use adam_kernel_fst::{
    lexicon::LexiconV1,
    morphotactics::{NounFeatures, synthesise_noun},
    parser::{Analysis, analyse},
};

/// Editorial list of Kazakh city roots eligible for in-sample swap.
///
/// Hardcoded rather than derived from the Lexicon: the Lexicon tags
/// cities as `part_of_speech = "noun"`, same as every other noun, so
/// dynamic detection would sweep in too many false positives (rivers,
/// mountains, common objects that happen to share a root with a city).
/// Keeping the set editorial makes the swap auditable.
pub const PLACE_NAMES: &[&str] = &[
    "алматы",
    "астана",
    "шымкент",
    "қарағанды",
    "тараз",
    "павлодар",
    "ақтөбе",
    "семей",
    "атырау",
    "өскемен",
    "талдықорған",
    "қостанай",
    "орал",
    "қызылорда",
    "ақтау",
    "петропавл",
    "көкшетау",
    "жезқазған",
    "қарқаралы",
    "түркістан",
];

/// One atomic substitution inside a retrieved sample.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Swap {
    /// Whitespace-split token index of the swapped word (0-based).
    pub token_index: usize,
    /// Original surface form (including any trailing punctuation).
    pub from: String,
    /// Replacement surface form (FST-synthesised, with original
    /// capitalisation and trailing punctuation preserved).
    pub to: String,
    /// Canonical root the user's session value mapped to.
    pub user_root: String,
    /// FST feature bundle shared between `from` and `to` — both are
    /// the same morpheme chain (case, number, possessive, predicate).
    pub features: NounFeatures,
}

/// Result of composing a sample. `was_changed()` tells a caller
/// whether to prefer `output` over `original`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Composition {
    pub original: String,
    pub output: String,
    pub swaps: Vec<Swap>,
}

impl Composition {
    pub fn was_changed(&self) -> bool {
        !self.swaps.is_empty()
    }

    /// Short human-readable trace line — useful for dialog `--trace`.
    pub fn trace(&self) -> String {
        if self.swaps.is_empty() {
            return "compose: no swap applied".to_string();
        }
        let parts: Vec<String> = self
            .swaps
            .iter()
            .map(|s| {
                format!(
                    "[{}] {} → {} (root={}, case={:?})",
                    s.token_index, s.from, s.to, s.user_root, s.features.case
                )
            })
            .collect();
        format!("compose: {}", parts.join("; "))
    }
}

/// Substitute every city mention in `sample_text` with the user's
/// city, preserving the original FST feature bundle on each
/// replacement. Returns an unchanged `Composition` when:
///
///   - `user_city` is not in [`PLACE_NAMES`] (we can't FST-synthesise
///     it reliably);
///   - the text contains a 4-digit year 1500–2100 (biographical /
///     historical-context guard);
///   - no token in the text parses as a city in [`PLACE_NAMES`];
///   - every city mention is already `user_city` (no-op).
pub fn compose_with_city(sample_text: &str, user_city: &str, lexicon: &LexiconV1) -> Composition {
    let user_city_norm: String = user_city.trim().to_lowercase();
    let unchanged = || Composition {
        original: sample_text.to_string(),
        output: sample_text.to_string(),
        swaps: Vec::new(),
    };

    if !PLACE_NAMES.contains(&user_city_norm.as_str()) {
        return unchanged();
    }
    if contains_biographical_year(sample_text) {
        return unchanged();
    }

    let tokens: Vec<&str> = sample_text.split_whitespace().collect();
    let mut output_tokens: Vec<String> = tokens.iter().map(|s| s.to_string()).collect();
    let mut swaps: Vec<Swap> = Vec::new();

    for (i, token) in tokens.iter().enumerate() {
        let (alpha_core, trailing_punct) = split_punct(token);
        if alpha_core.is_empty() {
            continue;
        }
        let normalized = alpha_core.to_lowercase();

        let analyses = analyse(&normalized, lexicon);
        // Pick the first analysis whose root is in PLACE_NAMES and
        // differs from the user's city. "First" = deterministic
        // token-order; parser's analysis order is itself deterministic.
        let swap_candidate = analyses.iter().find_map(|a| {
            if let Analysis::Noun { root, features } = a {
                let root_lower = root.root.to_lowercase();
                if PLACE_NAMES.contains(&root_lower.as_str()) && root_lower != user_city_norm {
                    return Some((root_lower, *features));
                }
            }
            None
        });

        let Some((_, features)) = swap_candidate else {
            continue;
        };

        let replaced = synthesise_noun(&user_city_norm, features);
        let final_alpha = if first_char_is_upper(token) {
            capitalise_first(&replaced)
        } else {
            replaced
        };
        let final_token = format!("{final_alpha}{trailing_punct}");

        swaps.push(Swap {
            token_index: i,
            from: token.to_string(),
            to: final_token.clone(),
            user_root: user_city_norm.clone(),
            features,
        });
        output_tokens[i] = final_token;
    }

    Composition {
        original: sample_text.to_string(),
        output: output_tokens.join(" "),
        swaps,
    }
}

/// True iff the text contains any 4-digit number in [1500, 2100].
/// Those are the years that almost always signal a biographical or
/// historical context where swapping a city would fabricate a fact
/// (e.g., "Абай 1845 жылы Қарқаралыда туған" must not become
/// "Абай 1845 жылы Алматыда туған").
fn contains_biographical_year(text: &str) -> bool {
    for token in text.split(|c: char| !c.is_ascii_digit()) {
        if token.len() != 4 {
            continue;
        }
        if let Ok(n) = token.parse::<u32>() {
            if (1500..=2100).contains(&n) {
                return true;
            }
        }
    }
    false
}

fn split_punct(token: &str) -> (String, String) {
    let alpha: String = token
        .chars()
        .filter(|c| c.is_alphabetic() || *c == '-')
        .collect();
    let punct: String = token
        .chars()
        .filter(|c| !c.is_alphabetic() && *c != '-')
        .collect();
    (alpha, punct)
}

fn first_char_is_upper(s: &str) -> bool {
    s.chars().next().is_some_and(|c| c.is_uppercase())
}

fn capitalise_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().chain(chars).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_lex() -> Option<LexiconV1> {
        LexiconV1::load_default().ok()
    }

    #[test]
    fn no_swap_when_user_city_unknown() {
        let Some(lex) = load_lex() else { return };
        let c = compose_with_city("Алматыда жақсы күн", "Неизвестный", &lex);
        assert!(!c.was_changed());
        assert_eq!(c.original, c.output);
    }

    #[test]
    fn no_swap_when_text_has_biographical_year() {
        let Some(lex) = load_lex() else { return };
        let c = compose_with_city(
            "Абай Құнанбайұлы 1845 жылы Қарқаралыда туған",
            "Алматы",
            &lex,
        );
        assert!(!c.was_changed(), "year guard must block biographical swap");
        assert!(c.output.contains("Қарқаралыда"));
    }

    #[test]
    fn swaps_city_preserving_locative() {
        let Some(lex) = load_lex() else { return };
        let c = compose_with_city("Мен Алматыда тұрамын", "Шымкент", &lex);
        assert!(c.was_changed());
        // Шымкент + locative + harmony → Шымкентте
        assert!(
            c.output.contains("Шымкентте"),
            "expected FST-rendered locative, got {}",
            c.output
        );
        assert!(!c.output.contains("Алматыда"));
        assert_eq!(c.swaps.len(), 1);
    }

    #[test]
    fn preserves_capitalisation_on_swap() {
        let Some(lex) = load_lex() else { return };
        let c = compose_with_city("Алматы үлкен қала", "Шымкент", &lex);
        assert!(c.was_changed());
        assert!(
            c.output.starts_with("Шымкент"),
            "capitalisation must be preserved: {}",
            c.output
        );
    }

    #[test]
    fn no_swap_when_city_matches_user_city() {
        let Some(lex) = load_lex() else { return };
        let c = compose_with_city("Мен Алматыдамын", "Алматы", &lex);
        assert!(
            !c.was_changed(),
            "swapping X → X should be a no-op, got {}",
            c.output
        );
    }

    #[test]
    fn preserves_trailing_punctuation() {
        let Some(lex) = load_lex() else { return };
        let c = compose_with_city(
            "Алматы, Тараз және Астана — үлкен қалалар.",
            "Шымкент",
            &lex,
        );
        // Multiple cities swappable, all should flip to Шымкент.
        assert!(c.was_changed());
        // Comma and period must still be there.
        assert!(c.output.contains(","));
        assert!(c.output.ends_with('.'));
    }

    #[test]
    fn trace_records_swap_details() {
        let Some(lex) = load_lex() else { return };
        let c = compose_with_city("Мен Алматыда тұрамын", "Шымкент", &lex);
        let trace = c.trace();
        assert!(trace.contains("Алматыда"));
        assert!(trace.contains("Шымкентте"));
        assert!(trace.contains("Locative"));
    }

    #[test]
    fn year_guard_ignores_short_digit_runs() {
        // "25 жас" (age) should NOT trigger the guard — 25 is 2 digits.
        let Some(lex) = load_lex() else { return };
        let c = compose_with_city("Алматыда 25 жастан асқан адамдар көп", "Шымкент", &lex);
        assert!(
            c.was_changed(),
            "only 4-digit years should guard; got {:?}",
            c.output
        );
    }
}
