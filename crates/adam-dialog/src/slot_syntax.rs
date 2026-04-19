//! Template slot syntax.
//!
//! Slot placeholders in a template take one of two shapes:
//!
//! ```text
//!   {slot}                   — plain: substitute slot value verbatim
//!   {slot|feat1+feat2+...}   — FST: synthesise via morphotactics
//! ```
//!
//! Feature tokens are case-insensitive and populate [`NounFeatures`].
//! Four families of tokens are recognised:
//!
//! ### Case (v0.9.5)
//!
//! | token | → |
//! |---|---|
//! | `nominative` / `nom` | `case = Nominative` |
//! | `genitive` / `gen`   | `case = Genitive` |
//! | `dative` / `dat`     | `case = Dative` |
//! | `accusative` / `acc` | `case = Accusative` |
//! | `locative` / `loc`   | `case = Locative` |
//! | `ablative` / `abl`   | `case = Ablative` |
//! | `instrumental` / `inst` | `case = Instrumental` |
//!
//! ### Number (v0.9.5)
//!
//! | token | → |
//! |---|---|
//! | `singular` / `sg` | `number = Singular` |
//! | `plural` / `pl`   | `number = Plural` |
//!
//! ### Derivation (v0.9.8)
//!
//! Applied BEFORE inflection — so `{root|agent+dative}` = root → Agent
//! derivation → Dative case (e.g. жаз → жазушы → жазушыға).
//!
//! | token | → `derivation = ...` | effect |
//! |---|---|---|
//! | `agent` | `Agent` | `-шы/-ші` (жаз → жазушы, "writer") |
//! | `abstract` / `abs` | `Abstract` | `-лық/-лік` (жақсы → жақсылық, "goodness") |
//! | `privative` / `priv` | `Privative` | `-сыз/-сіз` (тұз → тұзсыз, "saltless") |
//! | `endowed` / `end` | `Endowed` | `-лы/-лі` (күш → күшті, "strong") |
//! | `similative` / `sim` | `Similative` | `-дай/-дей` (тау → таудай) |
//! | `comparative` / `comp` | `Comparative` | `-рақ/-рек` |
//! | `verbalnoun` / `vnoun` | `VerbalNoun` | `-у` (жаз → жазу) |
//! | `actionnoun` / `anoun` | `ActionNoun` | `-ым/-ім` (айт → айтым) |
//! | `diminutive` / `dim` | `Diminutive` | `-шық/-шік` (үй → үйшік) |
//! | `ordinal` / `ord` | `Ordinal` | `-ншы/-нші` (бір → бірінші) |
//! | `collective` / `coll` | `Collective` | `-еу/-ау` (бір → біреу) |
//!
//! ### Possessive (v0.9.8)
//!
//! | token | → `possessive = ...` |
//! |---|---|
//! | `p1sg` | `P1Sg` (my) |
//! | `p2sg_inf` | `P2SgInformal` (your, informal) |
//! | `p2sg` / `p2sg_pol` | `P2SgPolite` (your, polite) |
//! | `p3` | `P3` (his/her/its/their) |
//! | `p1pl` | `P1Pl` (our) |
//! | `p2pl_inf` | `P2PlInformal` (your pl, informal) |
//! | `p2pl` / `p2pl_pol` | `P2PlPolite` (your pl, polite) |
//!
//! ### General rules
//!
//! - Unknown tokens are silently ignored — keeps templates forward-
//!   compatible when new features arrive.
//! - Feature order doesn't matter; later tokens of the same class win.
//! - Combination example: `{name|p1sg+dative}` → "менің Xке" sense.

use adam_kernel_fst::morphotactics::{Case, Derivation, NounFeatures, Number, Possessive};

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

/// Parse a feature spec into a [`NounFeatures`] bundle. Supports four
/// token families: case, number, derivation, possessive (see module
/// docs for the full mapping). Unknown tokens are silently ignored;
/// empty input gives `NounFeatures::default()`.
pub fn parse_noun_features(spec: &str) -> NounFeatures {
    let mut feats = NounFeatures::default();
    for tok in spec.split('+') {
        let tok = tok.trim().to_lowercase();
        match tok.as_str() {
            // Case
            "nominative" | "nom" => feats.case = Some(Case::Nominative),
            "genitive" | "gen" => feats.case = Some(Case::Genitive),
            "dative" | "dat" => feats.case = Some(Case::Dative),
            "accusative" | "acc" => feats.case = Some(Case::Accusative),
            "locative" | "loc" => feats.case = Some(Case::Locative),
            "ablative" | "abl" => feats.case = Some(Case::Ablative),
            "instrumental" | "inst" => feats.case = Some(Case::Instrumental),
            // Number
            "singular" | "sg" => feats.number = Some(Number::Singular),
            "plural" | "pl" => feats.number = Some(Number::Plural),
            // Derivation (applied BEFORE inflection)
            "agent" => feats.derivation = Some(Derivation::Agent),
            "abstract" | "abs" => feats.derivation = Some(Derivation::Abstract),
            "privative" | "priv" => feats.derivation = Some(Derivation::Privative),
            "endowed" | "end" => feats.derivation = Some(Derivation::Endowed),
            "similative" | "sim" => feats.derivation = Some(Derivation::Similative),
            "comparative" | "comp" => feats.derivation = Some(Derivation::Comparative),
            "verbalnoun" | "vnoun" => feats.derivation = Some(Derivation::VerbalNoun),
            "actionnoun" | "anoun" => feats.derivation = Some(Derivation::ActionNoun),
            "diminutive" | "dim" => feats.derivation = Some(Derivation::Diminutive),
            "ordinal" | "ord" => feats.derivation = Some(Derivation::Ordinal),
            "collective" | "coll" => feats.derivation = Some(Derivation::Collective),
            // Possessive
            "p1sg" => feats.possessive = Some(Possessive::P1Sg),
            "p2sg_inf" => feats.possessive = Some(Possessive::P2SgInformal),
            "p2sg" | "p2sg_pol" => feats.possessive = Some(Possessive::P2SgPolite),
            "p3" => feats.possessive = Some(Possessive::P3),
            "p1pl" => feats.possessive = Some(Possessive::P1Pl),
            "p2pl_inf" => feats.possessive = Some(Possessive::P2PlInformal),
            "p2pl" | "p2pl_pol" => feats.possessive = Some(Possessive::P2PlPolite),
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

    #[test]
    fn features_derivation_agent() {
        let f = parse_noun_features("agent");
        assert_eq!(f.derivation, Some(Derivation::Agent));
    }

    #[test]
    fn features_derivation_all_11_tokens() {
        for (tok, expected) in [
            ("agent", Derivation::Agent),
            ("abstract", Derivation::Abstract),
            ("privative", Derivation::Privative),
            ("endowed", Derivation::Endowed),
            ("similative", Derivation::Similative),
            ("comparative", Derivation::Comparative),
            ("verbalnoun", Derivation::VerbalNoun),
            ("actionnoun", Derivation::ActionNoun),
            ("diminutive", Derivation::Diminutive),
            ("ordinal", Derivation::Ordinal),
            ("collective", Derivation::Collective),
        ] {
            let f = parse_noun_features(tok);
            assert_eq!(
                f.derivation,
                Some(expected),
                "derivation token {tok:?} misparsed"
            );
        }
    }

    #[test]
    fn features_derivation_short_aliases() {
        assert_eq!(
            parse_noun_features("abs").derivation,
            Some(Derivation::Abstract)
        );
        assert_eq!(
            parse_noun_features("dim").derivation,
            Some(Derivation::Diminutive)
        );
        assert_eq!(
            parse_noun_features("ord").derivation,
            Some(Derivation::Ordinal)
        );
    }

    #[test]
    fn features_possessive_all() {
        for (tok, expected) in [
            ("p1sg", Possessive::P1Sg),
            ("p2sg_inf", Possessive::P2SgInformal),
            ("p2sg", Possessive::P2SgPolite),
            ("p2sg_pol", Possessive::P2SgPolite),
            ("p3", Possessive::P3),
            ("p1pl", Possessive::P1Pl),
            ("p2pl_inf", Possessive::P2PlInformal),
            ("p2pl", Possessive::P2PlPolite),
        ] {
            let f = parse_noun_features(tok);
            assert_eq!(
                f.possessive,
                Some(expected),
                "possessive token {tok:?} misparsed"
            );
        }
    }

    #[test]
    fn features_full_combination() {
        // Derivation + possessive + number + case together.
        let f = parse_noun_features("agent+p1sg+plural+dative");
        assert_eq!(f.derivation, Some(Derivation::Agent));
        assert_eq!(f.possessive, Some(Possessive::P1Sg));
        assert_eq!(f.number, Some(Number::Plural));
        assert_eq!(f.case, Some(Case::Dative));
    }
}
