//! Layer 4 — response realiser.
//!
//! v0.8.0: plain `{slot}` placeholders, text substitution only.
//! v0.9.5: `{slot|features}` FST-backed expansion. `features` is a
//! `+`-separated spec (see `slot_syntax`) that describes the noun
//! case + number to synthesise via `adam_kernel_fst::morphotactics`.
//! v1.1.0: transliteration module removed (Kazakh-only surface
//! reverts the v0.9.6 multilingual decision). Non-Cyrillic slots are
//! now a bug and return the raw value unchanged.
//!
//! Examples:
//! ```text
//!   "сәлем {name}"                    → "сәлем Дәулет"
//!   "{city|locative} тұрасыз ба?"     → "Алматыда тұрасыз ба?"
//!   "{occupation|plural} қажет"       → "мұғалімдер қажет"
//! ```
//!
//! Unfilled placeholders stay visible as literal `{name}` / `{city|loc}`
//! — deliberately ugly so missing-slot bugs surface during QA rather
//! than silently dropping information.

use adam_kernel_fst::morphotactics::synthesise_noun;

use crate::language_core::kazakh_respectful_address;
use crate::planner::ResponsePlan;
use crate::slot_inventory::{SlotInventory, VariantStrategy};
use crate::slot_syntax::{parse_noun_features, parse_placeholder};

/// Render a response plan into the final output string. Scans the
/// template left-to-right, expanding `{...}` placeholders as they
/// appear; everything else is emitted verbatim.
///
/// **v4.6.1** — final-output orthographic pass: capitalise the
/// first letter of the first alphabetic codepoint. Templates are
/// authored lowercase by convention (so `{name|locative}` etc.
/// produce the expected lowercase morphology), but the surface
/// reply should start with a capital letter like every other
/// well-formed Kazakh sentence. Skips leading whitespace /
/// punctuation; preserves quote-led replies («...») by stepping
/// into the first alphabetic codepoint past the opening quote.
pub fn realise(plan: &ResponsePlan) -> String {
    let raw = expand_template(&plan.literal, &plan.slots, None, 0);
    if contains_unfilled_placeholder(&raw) {
        return TEMPLATE_LEAK_FALLBACK.to_string();
    }
    let cased = capitalise_first_letter(&raw);
    ensure_sentence_final(&cased)
}

/// **v5.7.5 — G1.5 of the proof-carrying generation arc.** Render a
/// response plan with an attached typed slot inventory, enabling
/// per-slot variant selection.
///
/// Templates can now opt into variation via the `vary` directive:
/// `{name|vary}` tells the realiser to consult the inventory's
/// `variants` list for the slot and rng-pick a strategy (e.g.
/// `Literal` vs `RespectfulAddress` for `{name}`). Without `vary`
/// in the feature spec the realiser falls back to the v0.8.0 path
/// (literal substitution + optional FST features) bit-for-bit.
///
/// `rng_seed` is the same per-turn seed used by the planner — pinning
/// the seed makes variant selection deterministic.
///
/// **Backward compatibility.** `realise(plan)` (no inventory) is
/// preserved as a thin wrapper; every existing template renders
/// identically. Variation is opt-in at the template authorship
/// level — no existing template surface changes.
pub fn realise_with_inventory(
    plan: &ResponsePlan,
    inventory: Option<&SlotInventory>,
    rng_seed: u64,
) -> String {
    let raw = expand_template(&plan.literal, &plan.slots, inventory, rng_seed);
    if contains_unfilled_placeholder(&raw) {
        return TEMPLATE_LEAK_FALLBACK.to_string();
    }
    let cased = capitalise_first_letter(&raw);
    ensure_sentence_final(&cased)
}

/// **v5.17.1 — adversarial D1 fix (mta_02 critical).** Last-line
/// safety net for the realiser. Any `{slot}` placeholder still
/// present in the expanded template means a planner / slot-extraction
/// bug routed an intent to a template whose required slots were not
/// populated (e.g. `StatementOfAge { years: None }` routed to the
/// `statement_of_age` family whose templates all reference `{age}`).
/// Pre-v5.17.1 the unfilled placeholder leaked to the user as
/// literal text — and worse, `capitalise_first_letter` then
/// uppercased the placeholder's first letter, producing strings
/// like `{Age} — жақсы жас.`. v5.17.0 adversarial_dialog_v1
/// surfaced this in `mta_02`.
///
/// This guard is **defensive** — it doesn't try to recover the
/// missing data, only to keep the user-facing output trustworthy.
/// The real fix lives in the planner / slot extractor (don't route
/// to `statement_of_age` without an `age` slot); this is the last
/// line of defense.
const TEMPLATE_LEAK_FALLBACK: &str = "Сұрағыңызды толық түсінбедім. Аздап нақтылап жіберіңізші.";

fn contains_unfilled_placeholder(rendered: &str) -> bool {
    // Every unfilled adam placeholder leaves a literal `{name}` or
    // `{name|features}` substring (see `expand_placeholder`'s
    // `return format!("{{{inner}}}")` path). The identifier part
    // (everything before the optional `|`) is ASCII alphabetic +
    // underscore.
    //
    // **Min-length gate (v5.17.2):** Rust pedagogical exercises
    // embed format-string placeholders like `println!("{s}")` and
    // `{:?}` — these are part of the *content*, not unfilled adam
    // slots. Adam slot names in `data/dialog/templates/v1.toml`
    // are all ≥ 3 chars (`age`, `name`, `topic`, `fact`, ...), so a
    // 3-char minimum on the identifier separates the two
    // namespaces without losing any real slot. If a 2-char slot
    // is ever introduced, relax this here and rename the colliding
    // Rust example to use a longer variable name.
    let mut search_from = 0;
    while let Some(open_rel) = rendered[search_from..].find('{') {
        let open = search_from + open_rel;
        let Some(rel_close) = rendered[open..].find('}') else {
            return false;
        };
        let close = open + rel_close;
        let inner = &rendered[open + 1..close];
        let identifier_end = inner.find('|').unwrap_or(inner.len());
        let identifier = &inner[..identifier_end];
        if identifier.len() >= 3
            && identifier
                .chars()
                .all(|c| c.is_ascii_alphabetic() || c == '_')
        {
            return true;
        }
        search_from = close + 1;
    }
    false
}

/// **v4.6.5** — declarative replies should end with a sentence-
/// final punctuation mark. If the rendered reply ends with an
/// alphabetic character AND is at least 10 codepoints long
/// (filtering out short interjections like «иә», «сәлем» that
/// idiomatically don't take a period), append `.`. Any reply
/// already ending in `.`/`!`/`?`/`…`/`»`/`"`/`)` is left as-is.
/// Length-based gate is intentionally simple — Kazakh punctuation
/// conventions follow the same Cyrillic-orthographic rules as
/// Russian, and short interjections / one-word answers shouldn't
/// be forced to carry a period.
fn ensure_sentence_final(s: &str) -> String {
    let trimmed = s.trim_end();
    if trimmed.chars().count() < 10 {
        return s.to_string();
    }
    match trimmed.chars().last() {
        Some(last) if matches!(last, '.' | '!' | '?' | '…' | '»' | '"' | ')' | ']') => {
            s.to_string()
        }
        Some(last) if last.is_alphabetic() => {
            let mut out = trimmed.to_string();
            out.push('.');
            out
        }
        _ => s.to_string(),
    }
}

/// **v4.6.1** — uppercase the first alphabetic codepoint of the
/// reply. Cyrillic-Kazakh-aware: `қ`/`ң`/`ғ`/`ө`/`ү`/`ұ`/`һ` map to
/// `Қ`/`Ң`/`Ғ`/`Ө`/`Ү`/`Ұ`/`Һ` via standard Unicode case mapping
/// (Rust's `char::to_uppercase` handles all Cyrillic correctly).
/// Iterates past leading non-alphabetic characters (whitespace,
/// punctuation, quotation marks) so a quote-led reply still
/// capitalises the first letter of the actual word, not the quote
/// glyph. No-op on empty strings or all-non-alphabetic strings.
fn capitalise_first_letter(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 1);
    let mut chars = s.chars();
    let mut capitalised = false;
    for ch in chars.by_ref() {
        if !capitalised && ch.is_alphabetic() {
            // Push the uppercase version (may expand to multiple
            // chars in some scripts; for Kazakh Cyrillic always 1).
            for u in ch.to_uppercase() {
                out.push(u);
            }
            capitalised = true;
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod template_leak_guard_tests_v5171 {
    use super::contains_unfilled_placeholder;

    #[test]
    fn flags_bare_slot_v5171() {
        assert!(contains_unfilled_placeholder("{age} — жақсы жас"));
    }

    #[test]
    fn flags_capitalized_slot_v5171() {
        // Realiser's `capitalise_first_letter` runs after expansion,
        // so a leaked `{age}` at the start of the string becomes
        // `{Age}` by the time the user sees it. Both shapes must be
        // caught here — the guard runs BEFORE the casing pass.
        assert!(contains_unfilled_placeholder("{Age} — жақсы жас"));
    }

    #[test]
    fn flags_slot_with_features_v5171() {
        assert!(contains_unfilled_placeholder("{city|locative} тұрасыз ба"));
    }

    #[test]
    fn does_not_flag_plain_text_v5171() {
        assert!(!contains_unfilled_placeholder("Сәлем, Дәулет!"));
    }

    #[test]
    fn does_not_flag_empty_braces_v5171() {
        // Empty `{}` is suspicious but not a template placeholder
        // (placeholders always have an alphabetic identifier inside).
        // Better to let it through than to spuriously fire the guard.
        assert!(!contains_unfilled_placeholder("{}"));
    }

    #[test]
    fn does_not_flag_punctuation_inside_braces_v5171() {
        // Defensive: braces around non-alphabetic content (Kazakh
        // typesetting rarely uses curly braces, but they could appear
        // in user-pasted code/math).
        assert!(!contains_unfilled_placeholder("{1+2}"));
    }

    #[test]
    fn does_not_flag_rust_format_placeholders_v5172() {
        // v5.17.2 regression: Rust pedagogical exercises embed
        // format-string placeholders. They are CONTENT, not
        // unfilled adam slots.
        assert!(!contains_unfilled_placeholder("println!(\"{s}\");"));
        assert!(!contains_unfilled_placeholder("println!(\"{}\", s);"));
        assert!(!contains_unfilled_placeholder("println!(\"{:?}\", s);"));
        assert!(!contains_unfilled_placeholder("dbg!(\"{x}\")"));
    }

    #[test]
    fn still_flags_real_slot_after_rust_format_v5172() {
        // Mixed content: a real unfilled adam slot AFTER Rust format
        // placeholders must still trigger the guard (scan-through).
        assert!(contains_unfilled_placeholder(
            "println!(\"{s}\") демо ал {age} жасыңыз"
        ));
    }
}

#[cfg(test)]
mod realiser_tests {
    use super::capitalise_first_letter;

    #[test]
    fn capitalises_kazakh_first_letter() {
        assert_eq!(capitalise_first_letter("сәлем"), "Сәлем");
        assert_eq!(capitalise_first_letter("қазақстан"), "Қазақстан");
        assert_eq!(capitalise_first_letter("ңайып"), "Ңайып");
        assert_eq!(capitalise_first_letter("өзгеше"), "Өзгеше");
        assert_eq!(capitalise_first_letter("үлгі"), "Үлгі");
    }

    #[test]
    fn capitalises_past_leading_punctuation() {
        assert_eq!(
            capitalise_first_letter("«қазақстан туралы»"),
            "«Қазақстан туралы»"
        );
        assert_eq!(capitalise_first_letter("  сәлем"), "  Сәлем");
        assert_eq!(capitalise_first_letter("— иә"), "— Иә");
    }

    #[test]
    fn already_capitalised_stays_capitalised() {
        assert_eq!(capitalise_first_letter("Сәлем"), "Сәлем");
    }

    #[test]
    fn no_op_on_empty_or_non_alphabetic() {
        assert_eq!(capitalise_first_letter(""), "");
        assert_eq!(capitalise_first_letter("   "), "   ");
        assert_eq!(capitalise_first_letter("!?."), "!?.");
    }

    use super::ensure_sentence_final;

    #[test]
    fn appends_period_to_declarative_reply() {
        assert_eq!(
            ensure_sentence_final("Қазақстан — Орталық Азиядағы ел"),
            "Қазақстан — Орталық Азиядағы ел."
        );
        assert_eq!(
            ensure_sentence_final("Маркпен танысқаныма қуаныштымын"),
            "Маркпен танысқаныма қуаныштымын."
        );
    }

    #[test]
    fn leaves_existing_punctuation_alone() {
        assert_eq!(ensure_sentence_final("Қазақстан — ел."), "Қазақстан — ел.");
        assert_eq!(
            ensure_sentence_final("Сіз қайда тұрасыз?"),
            "Сіз қайда тұрасыз?"
        );
        assert_eq!(ensure_sentence_final("Иә!"), "Иә!");
    }

    #[test]
    fn skips_short_interjections() {
        assert_eq!(ensure_sentence_final("Сәлем"), "Сәлем");
        assert_eq!(ensure_sentence_final("Иә"), "Иә");
        assert_eq!(ensure_sentence_final("Жақсы"), "Жақсы");
    }

    #[test]
    fn handles_quoted_or_paren_endings() {
        assert_eq!(
            ensure_sentence_final("Кітап «Абай жолы» дейді»"),
            "Кітап «Абай жолы» дейді»"
        );
    }
}

fn expand_template(
    template: &str,
    slots: &std::collections::HashMap<String, String>,
    inventory: Option<&SlotInventory>,
    rng_seed: u64,
) -> String {
    let mut out = String::with_capacity(template.len());
    let mut i = 0;
    // **v5.7.5** — per-placeholder rng salt. Each placeholder gets a
    // distinct salt derived from its position in the template, so
    // multi-slot templates with `{name|vary}` ... `{name|vary}` (same
    // slot referenced twice) can pick the same variant deterministically
    // OR differ across positions if a future strategy tier wants that.
    // For G1.5 the salt is the byte offset of the placeholder; same
    // template + same seed = same expansion.
    let mut placeholder_index: u64 = 0;
    let bytes = template.as_bytes();
    while i < bytes.len() {
        // `{` is a single byte; safe to compare at a byte offset since
        // the preceding iterations are char-aligned.
        if bytes[i] == b'{' {
            if let Some(end_rel) = template[i + 1..].find('}') {
                let inner = &template[i + 1..i + 1 + end_rel];
                out.push_str(&expand_placeholder(
                    inner,
                    slots,
                    inventory,
                    rng_seed.wrapping_add(placeholder_index),
                ));
                placeholder_index = placeholder_index.wrapping_add(1);
                i += 1 + end_rel + 1;
                continue;
            }
        }
        let ch = template[i..].chars().next().expect("valid utf-8 boundary");
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn expand_placeholder(
    inner: &str,
    slots: &std::collections::HashMap<String, String>,
    inventory: Option<&SlotInventory>,
    rng_seed: u64,
) -> String {
    let (slot_name, feature_spec) = parse_placeholder(inner);
    let Some(root) = slots.get(slot_name) else {
        // Unfilled — leave the raw placeholder visible.
        return format!("{{{inner}}}");
    };
    let (vary_requested, fst_spec) = split_vary_directive(feature_spec);
    // **v5.7.5** — when the template asks for `vary` AND an inventory
    // is attached AND the slot is registered with non-trivial variants,
    // pick a `VariantStrategy` per turn (rng-seeded) and apply it.
    // Falls back to the literal path if any precondition fails so
    // existing behaviour is preserved bit-for-bit when inventory is
    // absent or the slot isn't documented yet.
    let varied: Option<String> = if vary_requested {
        inventory
            .and_then(|inv| inv.get(slot_name))
            .filter(|slot| slot.supports_variation())
            .and_then(|slot| pick_and_apply_variant(slot, root, rng_seed))
    } else {
        None
    };
    let value: String = varied.unwrap_or_else(|| root.clone());
    match fst_spec {
        None => value,
        Some(spec) => {
            // Kazakh-only surface (v1.1.0): non-Cyrillic roots aren't
            // expected. FST phonology operates directly on whatever
            // glyphs are passed in; if a Latin-only root leaks through
            // from an upstream bug it's better surfaced as visibly
            // wrong output than silently transliterated.
            let features = parse_noun_features(&spec);
            synthesise_noun(&value, features)
        }
    }
}

/// **v5.7.5** — strip the `vary` token from a feature spec. Returns
/// `(vary_requested, remaining_feature_spec)`. The remaining spec is
/// what `parse_noun_features` consumes for FST inflection. Conservative
/// — only strips an exact `vary` token; longer tokens that include
/// the string (`varying`, etc.) pass through untouched.
fn split_vary_directive(spec: Option<&str>) -> (bool, Option<String>) {
    let Some(spec) = spec else {
        return (false, None);
    };
    let mut vary = false;
    let mut kept: Vec<&str> = Vec::new();
    for tok in spec.split('+') {
        let trimmed = tok.trim();
        if trimmed.eq_ignore_ascii_case("vary") {
            vary = true;
        } else if !trimmed.is_empty() {
            kept.push(trimmed);
        }
    }
    let kept = if kept.is_empty() {
        None
    } else {
        Some(kept.join("+"))
    };
    (vary, kept)
}

/// **v5.7.5** — apply a `VariantStrategy` from the inventory. Returns
/// `Some(transformed_value)` if a non-Literal strategy fired and
/// produced a meaningful difference; `None` otherwise (caller falls
/// back to the literal value).
///
/// Strategy selection: rng-pick uniformly from the slot's `variants`
/// list. The first strategy in the list (typically `Literal`) is
/// included in the pool so variation isn't always-on — sometimes the
/// turn just uses the literal value. This is what makes the variation
/// feel natural rather than mechanically alternating.
fn pick_and_apply_variant(
    slot: &crate::slot_inventory::Slot,
    value: &str,
    rng_seed: u64,
) -> Option<String> {
    if slot.variants.is_empty() {
        return None;
    }
    let idx = (rng_seed as usize) % slot.variants.len();
    let strategy = slot.variants[idx];
    apply_variant_strategy(strategy, value)
}

/// **v5.7.5** — execute a single `VariantStrategy` against a literal
/// value. Returns `Some(transformed)` on success, `None` when the
/// strategy is `Literal` (no transformation) or doesn't apply (e.g.
/// `RespectfulAddress` on a name without a recognised first
/// consonant). The caller treats `None` as "use the literal value".
///
/// FST-case strategies (`FstLocative` / `FstGenitive` / etc.) return
/// `Some` with the synthesised inflected form. They overlap with the
/// existing `{slot|case=X}` template syntax — the variant strategy
/// is for inventory-driven variation; the template-side syntax is
/// for explicit author intent. Both paths feed the same FST.
fn apply_variant_strategy(strategy: VariantStrategy, value: &str) -> Option<String> {
    use adam_kernel_fst::morphotactics::{Case, NounFeatures};
    match strategy {
        VariantStrategy::Literal => None,
        VariantStrategy::RespectfulAddress => kazakh_respectful_address(value),
        VariantStrategy::FstLocative => Some(synthesise_noun(
            value,
            NounFeatures {
                case: Some(Case::Locative),
                ..Default::default()
            },
        )),
        VariantStrategy::FstGenitive => Some(synthesise_noun(
            value,
            NounFeatures {
                case: Some(Case::Genitive),
                ..Default::default()
            },
        )),
        VariantStrategy::FstDative => Some(synthesise_noun(
            value,
            NounFeatures {
                case: Some(Case::Dative),
                ..Default::default()
            },
        )),
        VariantStrategy::FstAblative => Some(synthesise_noun(
            value,
            NounFeatures {
                case: Some(Case::Ablative),
                ..Default::default()
            },
        )),
    }
}

#[cfg(test)]
mod variation_tests {
    use super::*;
    use crate::slot_inventory::{Slot, SlotKind, VariantStrategy};
    use std::collections::HashMap;

    fn name_slot(variants: Vec<VariantStrategy>) -> SlotInventory {
        SlotInventory {
            schema_version: 1,
            slots: vec![Slot {
                name: "name".into(),
                kind: SlotKind::PersonName,
                description: "test".into(),
                example: "Дәулет".into(),
                variants,
                fst_features: vec![],
            }],
        }
    }

    #[test]
    fn vary_directive_picks_respectful_form_v575() {
        let inv = name_slot(vec![VariantStrategy::RespectfulAddress]);
        let mut slots = HashMap::new();
        slots.insert("name".into(), "Дәулет".into());
        // RespectfulAddress is the only variant — selection is forced.
        let out = expand_template("Сәлем, {name|vary}!", &slots, Some(&inv), 0);
        assert_eq!(out, "Сәлем, Дәке!");
    }

    #[test]
    fn vary_directive_falls_back_to_literal_for_non_transformable_v575() {
        let inv = name_slot(vec![VariantStrategy::RespectfulAddress]);
        let mut slots = HashMap::new();
        // Two-char name: kazakh_respectful_address rejects (requires
        // ≥ 3 chars per the v4.18.0 / v4.51.5 rules). Variant strategy
        // returns None, realiser falls back to the literal value.
        slots.insert("name".into(), "Әл".into());
        let out = expand_template("Сәлем, {name|vary}!", &slots, Some(&inv), 0);
        assert_eq!(out, "Сәлем, Әл!");
    }

    #[test]
    fn vary_directive_no_op_without_inventory_v575() {
        let mut slots = HashMap::new();
        slots.insert("name".into(), "Дәулет".into());
        // No inventory attached — `vary` is silently ignored, value
        // is rendered literally. This is the v0.8.0-compat path.
        let out = expand_template("Сәлем, {name|vary}!", &slots, None, 0);
        assert_eq!(out, "Сәлем, Дәулет!");
    }

    #[test]
    fn vary_directive_deterministic_per_seed_v575() {
        // Two strategies — Literal and RespectfulAddress. Same seed =
        // same outcome.
        let inv = name_slot(vec![
            VariantStrategy::Literal,
            VariantStrategy::RespectfulAddress,
        ]);
        let mut slots = HashMap::new();
        slots.insert("name".into(), "Дәулет".into());
        let out_seed_0_a = expand_template("{name|vary}", &slots, Some(&inv), 0);
        let out_seed_0_b = expand_template("{name|vary}", &slots, Some(&inv), 0);
        assert_eq!(out_seed_0_a, out_seed_0_b);
    }

    #[test]
    fn vary_directive_combinable_with_fst_features_v575() {
        // `{name|vary+dative}` — first pick variant strategy, then
        // apply FST dative on the result. With RespectfulAddress
        // alone, "Дәулет" → "Дәке" → dative "Дәкеге".
        let inv = name_slot(vec![VariantStrategy::RespectfulAddress]);
        let mut slots = HashMap::new();
        slots.insert("name".into(), "Дәулет".into());
        let out = expand_template("{name|vary+dative}", &slots, Some(&inv), 0);
        assert_eq!(out, "Дәкеге");
    }
}
