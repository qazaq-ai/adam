//! Template slot syntax (v0.9.5).
//!
//! Slot placeholders in a template take one of two shapes:
//!
//! ```text
//!   {slot}                   — plain: substitute slot value verbatim
//!   {slot|feat1+feat2+...}   — FST: synthesise via morphotactics
//! ```
//!
//! Feature tokens are case-insensitive and map to [`NounFeatures`]:
//!
//! | token | → field |
//! |---|---|
//! | `nominative` / `nom` | `case = Nominative` |
//! | `genitive` / `gen`   | `case = Genitive` |
//! | `dative` / `dat`     | `case = Dative` |
//! | `accusative` / `acc` | `case = Accusative` |
//! | `locative` / `loc`   | `case = Locative` |
//! | `ablative` / `abl`   | `case = Ablative` |
//! | `instrumental` / `inst` | `case = Instrumental` |
//! | `singular` / `sg`    | `number = Singular` |
//! | `plural` / `pl`      | `number = Plural` |
//!
//! Unknown feature tokens are silently ignored — this keeps templates
//! forward-compatible when new features arrive. Feature order doesn't
//! matter; later tokens of the same class win.

use adam_kernel_fst::morphotactics::{Case, NounFeatures, Number};

/// Parse the inner text of a `{...}` placeholder into (slot-name,
/// optional feature spec). The slot name is what the planner looks
/// up in its slot map; the feature spec, if present, describes the
/// morphology to apply via `synthesise_noun`.
///
/// Example:
/// ```
/// use adam_dialog::slot_syntax::parse_placeholder;
/// let (name, feats) = parse_placeholder("city|locative");
/// assert_eq!(name, "city");
/// assert!(feats.is_some());
/// ```
pub fn parse_placeholder(inner: &str) -> (&str, Option<&str>) {
    match inner.split_once('|') {
        Some((name, feats)) => (name.trim(), Some(feats.trim())),
        None => (inner.trim(), None),
    }
}

/// Parse a feature spec ("locative", "plural+dative", "pl+loc") into
/// a [`NounFeatures`] bundle. Unknown tokens are ignored; empty input
/// gives `NounFeatures::default()`.
pub fn parse_noun_features(spec: &str) -> NounFeatures {
    let mut feats = NounFeatures::default();
    for tok in spec.split('+') {
        let tok = tok.trim().to_lowercase();
        match tok.as_str() {
            "nominative" | "nom" => feats.case = Some(Case::Nominative),
            "genitive" | "gen" => feats.case = Some(Case::Genitive),
            "dative" | "dat" => feats.case = Some(Case::Dative),
            "accusative" | "acc" => feats.case = Some(Case::Accusative),
            "locative" | "loc" => feats.case = Some(Case::Locative),
            "ablative" | "abl" => feats.case = Some(Case::Ablative),
            "instrumental" | "inst" => feats.case = Some(Case::Instrumental),
            "singular" | "sg" => feats.number = Some(Number::Singular),
            "plural" | "pl" => feats.number = Some(Number::Plural),
            _ => {}
        }
    }
    feats
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_placeholder() {
        assert_eq!(parse_placeholder("name"), ("name", None));
    }

    #[test]
    fn placeholder_with_features() {
        let (name, feats) = parse_placeholder("city|locative");
        assert_eq!(name, "city");
        assert_eq!(feats, Some("locative"));
    }

    #[test]
    fn features_default_is_empty() {
        let f = parse_noun_features("");
        assert_eq!(f, NounFeatures::default());
    }

    #[test]
    fn features_single_case() {
        let f = parse_noun_features("locative");
        assert_eq!(f.case, Some(Case::Locative));
        assert_eq!(f.number, None);
    }

    #[test]
    fn features_combined_plural_dative() {
        let f = parse_noun_features("plural+dative");
        assert_eq!(f.case, Some(Case::Dative));
        assert_eq!(f.number, Some(Number::Plural));
    }

    #[test]
    fn features_short_aliases() {
        let f = parse_noun_features("pl+loc");
        assert_eq!(f.case, Some(Case::Locative));
        assert_eq!(f.number, Some(Number::Plural));
    }

    #[test]
    fn features_unknown_token_ignored() {
        let f = parse_noun_features("dative+wat");
        assert_eq!(f.case, Some(Case::Dative));
    }

    #[test]
    fn features_last_wins_on_collision() {
        let f = parse_noun_features("dative+ablative");
        assert_eq!(f.case, Some(Case::Ablative));
    }
}
