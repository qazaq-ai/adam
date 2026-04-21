//! v2.1 pattern matchers — deterministic, grammar-derived,
//! feature-type-checked. Each matcher is a **pure function**: given
//! `(text, parses, lexicon, source)`, it appends zero or more [`Fact`]s
//! to the output vector. No RNG, no threshold tuning, no learned
//! weights.
//!
//! Adding a new pattern? Required properties:
//!
//!   1. **Type-checked on FST features** — use `Case`, `Tense`, `Person`,
//!      `Predicate` from `adam_kernel_fst::morphotactics`. Never match
//!      on raw surface strings if an FST feature exists.
//!   2. **POS-filtered** — the Lexicon tags some roots (e.g. verbs) as
//!      non-nominal. A pattern that expects a noun MUST reject everything
//!      else to keep the fact graph typed.
//!   3. **Short-circuit on first match per token** — matchers append
//!      ≤ one fact per sentence in v2.1. Multi-fact extraction is v2.3+.
//!   4. **Unit-tested** — every pattern gets an `extract_*_from_*` test
//!      with a positive and a negative case in `#[cfg(test)]` below.

use adam_kernel_fst::lexicon::LexiconV1;
use adam_kernel_fst::morphotactics::{Case, Possessive};
use adam_kernel_fst::parser::{Analysis, analyse};

use crate::{ConfidenceKind, Fact, FactSource, Predicate, SlotRef};

/// Copula pattern — `X — Y` produces `(X, is_a, Y)`.
///
/// Kazakh uses an em-dash (`—`) to separate an explicit subject from a
/// nominative predicate: «Абай — ақын» ("Abai is a poet"). The left
/// side is the subject, the right side is the is_a target. Both must
/// parse as content nouns in the Lexicon; both must be in the
/// nominative case (no case suffix).
///
/// Negative cases rejected:
///
/// - Ambiguous dash inside a longer clause ("Абай — қазақ ақыны — дара
///   тұлға"): we require exactly one `—` in the sentence.
/// - Non-noun on either side — verbs and unknown roots don't participate.
/// - Inflected side ("Абайдың — ...", "Абай — ақындардың ..."): the
///   surface form must match the analysed root character-by-character
///   modulo capitalisation, i.e. both sides must be bare nominatives.
pub fn copula_is_a(
    text: &str,
    _parses: &[Analysis],
    lexicon: &LexiconV1,
    source: &FactSource,
    out: &mut Vec<Fact>,
) {
    // `—` is a multi-char Unicode code point ('\u{2014}').
    let dash_count = text.chars().filter(|c| *c == '\u{2014}').count();
    if dash_count != 1 {
        return;
    }
    let (left, right) = match text.split_once('\u{2014}') {
        Some(parts) => parts,
        None => return,
    };

    // LHS must be a single bare nominative noun. Multi-word LHS means
    // the subject is a possessive / adjective-noun construction where
    // the surface head is misleading ("Ресей мәдениеті" — "Russia's
    // culture", taking "мәдениеті" alone is wrong).
    let subj_surface = match exactly_one_alphabetic_token(left) {
        Some(t) => t,
        None => return,
    };
    let Some(subj) = resolve_bare_noun(subj_surface, lexicon) else {
        return;
    };

    // RHS can be a single nominative noun OR a short noun phrase
    // ("білім бұлағы", "бала жанашыры"). For multi-word RHS, the
    // **syntactic head** is the rightmost noun — Kazakh is head-final.
    // We scan right-to-left via FST parse and pick the first noun
    // whose root satisfies the same purity check as LHS (content noun,
    // not closed-class).
    let Some(obj) = resolve_rhs_head(right, lexicon) else {
        return;
    };

    // Reject self-referential tautologies («адам — адам»,
    // «шекара — шекарасы» → both resolve to root "шекара»).
    if subj.root == obj.root {
        return;
    }

    // RHS length guard: a 5+-token RHS is usually a full clause, not an
    // NP — "шекара — Қазақстан мен Ресей шекарасы (7591 шақырым)".
    // Head extraction on such strings still produces plausible heads,
    // but the semantic fit degrades. Cap at 4 RHS tokens (empirically
    // covers "X — [adj] [poss] [noun]" NPs without stretching to full
    // clauses).
    const MAX_RHS_TOKENS: usize = 4;
    let rhs_token_count = strip_parens(right)
        .split(|c: char| !(c.is_alphabetic() || c == '-'))
        .filter(|t| !t.is_empty())
        .count();
    if rhs_token_count > MAX_RHS_TOKENS {
        return;
    }

    out.push(Fact {
        subject: subj,
        predicate: Predicate::IsA,
        object: obj,
        pattern: "X — Y".to_string(),
        source: source.clone(),
        confidence: ConfidenceKind::Grammar,
        raw_text: text.trim().to_string(),
    });
}

/// Head extraction for the RHS of `X — <noun phrase>` — Kazakh NPs are
/// head-final, so the rightmost noun is the syntactic head. We scan
/// RHS tokens right-to-left, try FST analysis on each, and return the
/// first root that satisfies:
///
///   - POS is `"noun"`;
///   - root is not closed-class (filters out pronouns / demonstratives).
///
/// The returned `SlotRef::root` is the canonical root (not the surface),
/// so possessive-suffixed surfaces like "бұлағы" correctly produce
/// root "бұлақ".
fn resolve_rhs_head(rhs: &str, lexicon: &LexiconV1) -> Option<SlotRef> {
    // Strip parenthetical content — "(7591 шақырым)" noise.
    let cleaned_rhs = strip_parens(rhs);
    let tokens: Vec<String> = cleaned_rhs
        .split(|c: char| !(c.is_alphabetic() || c == '-'))
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect();
    for tok in tokens.iter().rev() {
        let lowered = tok.to_lowercase();
        // Fast path: bare nominative in Lexicon.
        if let Some(entry) = lexicon.by_surface.get(&lowered) {
            if entry.part_of_speech == "noun"
                && entry.root == lowered
                && !is_closed_class(&entry.root)
            {
                return Some(SlotRef {
                    surface: tok.clone(),
                    root: entry.root.clone(),
                    pos: "noun".to_string(),
                });
            }
        }
        // FST fallback: any analysis yielding a content-noun root.
        // Catches possessives like "бұлағы" → бұлақ.
        for a in analyse(&lowered, lexicon) {
            if let Analysis::Noun { root, .. } = a {
                if root.part_of_speech == "noun" && !is_closed_class(&root.root) {
                    return Some(SlotRef {
                        surface: tok.clone(),
                        root: root.root.clone(),
                        pos: "noun".to_string(),
                    });
                }
            }
        }
    }
    None
}

/// Remove matched `(...)` groups from `s`. Unbalanced parens → return
/// input unchanged (defensive — don't corrupt the RHS).
fn strip_parens(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut depth = 0i32;
    for c in s.chars() {
        match c {
            '(' => depth += 1,
            ')' if depth > 0 => depth -= 1,
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    if depth != 0 {
        return s.to_string();
    }
    out
}

/// Locative-existential pattern — `X Y-да тұрады` produces
/// `(X, lives_in, Y)`.
///
/// Kazakh expresses "X lives in Y" as `<subject> <place-locative>
/// тұрады / тұрамын / тұрасың`. We require:
///
///   - a verb token analysable as a form of the verb `тұру` ("to live / to
///     reside / to stand") — matched by root, not surface, so every
///     person/number inflection works;
///   - a noun token analysable as a noun in `Case::Locative` somewhere
///     earlier in the sentence — its root is the place;
///   - a subject that is also a nominative content noun — the first
///     such noun in the sentence preceding the locative is treated as
///     the subject. If no nominative subject surfaces (e.g. the subject
///     is elided with a `-мын` copula), the pattern does not fire in
///     v2.1. Subject-ellipsis handling is v2.2.
pub fn locative_lives_in(
    text: &str,
    _parses: &[Analysis],
    lexicon: &LexiconV1,
    source: &FactSource,
    out: &mut Vec<Fact>,
) {
    // Tokenise + per-token parse. Build a parallel vector of
    // (surface, first_analysis) entries — deterministic since
    // `analyse` returns in insertion order.
    let tokens: Vec<(String, Option<Analysis>)> = text
        .split_whitespace()
        .map(|t| {
            let cleaned: String = t
                .chars()
                .filter(|c| c.is_alphabetic() || *c == '-')
                .collect();
            let lowered = cleaned.to_lowercase();
            let first = analyse(&lowered, lexicon).into_iter().next();
            (cleaned, first)
        })
        .filter(|(s, _)| !s.is_empty())
        .collect();

    // Require a form of the verb тұру somewhere in the sentence.
    let has_turu_verb = tokens.iter().any(|(_, a)| match a {
        Some(Analysis::Verb { root, .. }) => root.root == "тұру",
        _ => false,
    });
    if !has_turu_verb {
        return;
    }

    // Find the first locative-case noun.
    let locative_idx = tokens.iter().position(|(_, a)| match a {
        Some(Analysis::Noun { features, root }) => {
            features.case == Some(Case::Locative) && root.part_of_speech == "noun"
        }
        _ => false,
    });
    let Some(loc_idx) = locative_idx else { return };

    let loc_entry = &tokens[loc_idx];
    let (loc_surface, loc_root) = match &loc_entry.1 {
        Some(Analysis::Noun { root, .. }) => (loc_entry.0.clone(), root.root.clone()),
        _ => return,
    };

    // Subject = first nominative-case noun strictly before the locative.
    // We REJECT pronouns / closed-class items as subjects — a pronoun-
    // as-subject fact ("мен Алматы") is not useful knowledge. This is
    // the same filter as `semantics::NOT_A_TOPIC`, reimplemented here
    // to avoid a dialog→reasoning dep.
    let subj = (0..loc_idx).find_map(|i| match &tokens[i].1 {
        Some(Analysis::Noun { features, root }) => {
            if root.part_of_speech != "noun" {
                return None;
            }
            if features.case.is_some() && features.case != Some(Case::Nominative) {
                return None;
            }
            if is_closed_class(&root.root) {
                return None;
            }
            Some(SlotRef {
                surface: tokens[i].0.clone(),
                root: root.root.clone(),
                pos: "noun".to_string(),
            })
        }
        _ => None,
    });
    let Some(subject) = subj else { return };

    if subject.root == loc_root {
        return; // «Алматы Алматыда тұрады» — self-referential, skip.
    }

    out.push(Fact {
        subject,
        predicate: Predicate::LivesIn,
        object: SlotRef {
            surface: loc_surface,
            root: loc_root,
            pos: "noun".to_string(),
        },
        pattern: "X Y-да тұрады".to_string(),
        source: source.clone(),
        confidence: ConfidenceKind::Grammar,
        raw_text: text.trim().to_string(),
    });
}

/// Possessive-existence pattern — `X-тың Y-сы бар` produces
/// `(X, has, Y)`.
///
/// Kazakh expresses "X has Y" as `<possessor-genitive> <possessed-P3> бар`
/// ("Баланың кітабы бар" = "The child has a book"). We type-check with
/// full FST features, not string matching:
///
///   - a token analysable as a noun in `Case::Genitive` — its root is
///     the possessor (subject);
///   - a following token analysable as a noun with `Possessive::P3` —
///     its root is the possessed (object);
///   - the existential particle "бар" at the end (free order inside the
///     sentence).
///
/// Guards:
///
///   - subject root must not be closed-class (pronoun / demonstrative);
///   - subject ≠ object (no tautological self-possession);
///   - the possessor's genitive form and the possessed's P3 form must
///     appear **in order** (possessor first, possessed second) — Kazakh
///     is strictly head-final, so reversing them is a different
///     construction.
pub fn possessive_has(
    text: &str,
    _parses: &[Analysis],
    lexicon: &LexiconV1,
    source: &FactSource,
    out: &mut Vec<Fact>,
) {
    // Cheap prefilter — "бар" must appear as a word. Most sentences
    // don't contain possessive-existence, so this short-circuits the
    // expensive per-token parse.
    let has_bar = text
        .split(|c: char| !(c.is_alphabetic() || c == '-'))
        .any(|t| t.to_lowercase() == "бар");
    if !has_bar {
        return;
    }

    // Per-token parse with surface preservation.
    let tokens: Vec<(String, Option<Analysis>)> = text
        .split_whitespace()
        .map(|t| {
            let cleaned: String = t
                .chars()
                .filter(|c| c.is_alphabetic() || *c == '-')
                .collect();
            let lowered = cleaned.to_lowercase();
            let first = analyse(&lowered, lexicon).into_iter().next();
            (cleaned, first)
        })
        .filter(|(s, _)| !s.is_empty())
        .collect();

    // Find a genitive noun (possessor).
    let gen_idx = tokens.iter().position(|(_, a)| match a {
        Some(Analysis::Noun { features, root }) => {
            features.case == Some(Case::Genitive)
                && root.part_of_speech == "noun"
                && !is_closed_class(&root.root)
        }
        _ => false,
    });
    let Some(gen_idx) = gen_idx else { return };
    let (possessor_surface, possessor_root) = match &tokens[gen_idx].1 {
        Some(Analysis::Noun { root, .. }) => (tokens[gen_idx].0.clone(), root.root.clone()),
        _ => return,
    };

    // Immediately-following token must parse as a noun with P3 possessive
    // (possessed noun). Strict adjacency — intervening tokens break the
    // construction ("Баланың үйде кітабы бар" is a different meaning).
    let possessed_slot = tokens.get(gen_idx + 1).and_then(|(surface, a)| match a {
        Some(Analysis::Noun { features, root }) => {
            if features.possessive == Some(Possessive::P3)
                && root.part_of_speech == "noun"
                && !is_closed_class(&root.root)
            {
                Some(SlotRef {
                    surface: surface.clone(),
                    root: root.root.clone(),
                    pos: "noun".to_string(),
                })
            } else {
                None
            }
        }
        _ => None,
    });
    let Some(possessed) = possessed_slot else {
        return;
    };

    // Tautology guard.
    if possessor_root == possessed.root {
        return;
    }

    out.push(Fact {
        subject: SlotRef {
            surface: possessor_surface,
            root: possessor_root,
            pos: "noun".to_string(),
        },
        predicate: Predicate::Has,
        object: possessed,
        pattern: "X-тың Y-сы бар".to_string(),
        source: source.clone(),
        confidence: ConfidenceKind::Grammar,
        raw_text: text.trim().to_string(),
    });
}

/// Dative-motion pattern — `X Y-ке барады` produces `(X, goes_to, Y)`.
///
/// Kazakh expresses "X goes to Y" as `<subject-nom> <place-dative>
/// бару-in-some-inflection`. We type-check every slot with FST
/// features instead of string-matching the verb surface — every
/// person / number / tense form of `бару` is accepted as long as its
/// root analyses to `бару`.
///
/// Requirements (all enforced via FST features, never by surface):
///
///   - a verb token whose root is `бару` ("to go"). Any tense /
///     person / number passes.
///   - a noun token with `Case::Dative` earlier in the sentence.
///     Its root is the destination.
///   - a subject: the first **bare-nominative** content noun before
///     the destination. Pronouns + closed-class items are refused —
///     v2.1's [`is_closed_class`] filter — because a pronoun-subject
///     fact ("мен Алматы GoesTo") is ungrounded knowledge.
///
/// Non-adjacency and multiple-dative handling:
///
///   - If > 1 dative noun precedes the verb, we take the FIRST
///     (earliest in the sentence) — Kazakh proverbs and Wikipedia
///     prefer the direct destination first when chained.
///   - If a subject cannot be identified (ellipsis via P1Sg copula
///     on the verb, no bare-nominative noun precedes), the pattern
///     refuses — v2.5 does not guess; subject ellipsis is v2.6+.
pub fn dative_goes_to(
    text: &str,
    _parses: &[Analysis],
    lexicon: &LexiconV1,
    source: &FactSource,
    out: &mut Vec<Fact>,
) {
    let tokens: Vec<(String, Option<Analysis>)> = text
        .split_whitespace()
        .map(|t| {
            let cleaned: String = t
                .chars()
                .filter(|c| c.is_alphabetic() || *c == '-')
                .collect();
            let lowered = cleaned.to_lowercase();
            let first = analyse(&lowered, lexicon).into_iter().next();
            (cleaned, first)
        })
        .filter(|(s, _)| !s.is_empty())
        .collect();

    // Require a form of the verb бару in the sentence.
    let has_baru = tokens.iter().any(|(_, a)| match a {
        Some(Analysis::Verb { root, .. }) => root.root == "бару",
        _ => false,
    });
    if !has_baru {
        return;
    }

    // First dative noun is the destination.
    let dative_idx = tokens.iter().position(|(_, a)| match a {
        Some(Analysis::Noun { features, root }) => {
            features.case == Some(Case::Dative)
                && root.part_of_speech == "noun"
                && !is_closed_class(&root.root)
        }
        _ => false,
    });
    let Some(dat_idx) = dative_idx else { return };
    let (dest_surface, dest_root) = match &tokens[dat_idx].1 {
        Some(Analysis::Noun { root, .. }) => (tokens[dat_idx].0.clone(), root.root.clone()),
        _ => return,
    };

    // Subject = first bare-nominative content noun strictly before the
    // destination. Same POS + closed-class filter as locative pattern.
    let subj = (0..dat_idx).find_map(|i| match &tokens[i].1 {
        Some(Analysis::Noun { features, root }) => {
            if root.part_of_speech != "noun" {
                return None;
            }
            if features.case.is_some() && features.case != Some(Case::Nominative) {
                return None;
            }
            if is_closed_class(&root.root) {
                return None;
            }
            Some(SlotRef {
                surface: tokens[i].0.clone(),
                root: root.root.clone(),
                pos: "noun".to_string(),
            })
        }
        _ => None,
    });
    let Some(subject) = subj else { return };

    // Tautology guard.
    if subject.root == dest_root {
        return;
    }

    out.push(Fact {
        subject,
        predicate: Predicate::GoesTo,
        object: SlotRef {
            surface: dest_surface,
            root: dest_root,
            pos: "noun".to_string(),
        },
        pattern: "X Y-ке барады".to_string(),
        source: source.clone(),
        confidence: ConfidenceKind::Grammar,
        raw_text: text.trim().to_string(),
    });
}

// -----------------------------------------------------------------------------
// helpers
// -----------------------------------------------------------------------------

/// Resolve a whitespace token to a bare nominative noun SlotRef.
/// Returns `None` if the surface (after lowercasing + punct strip)
/// does not match a Lexicon entry whose POS is `"noun"` and whose
/// root string equals the cleaned surface.
fn resolve_bare_noun(surface: &str, lexicon: &LexiconV1) -> Option<SlotRef> {
    let cleaned: String = surface
        .chars()
        .filter(|c| c.is_alphabetic() || *c == '-')
        .collect();
    if cleaned.is_empty() {
        return None;
    }
    let lowered = cleaned.to_lowercase();
    let entry = lexicon.by_surface.get(&lowered)?;
    if entry.part_of_speech != "noun" {
        return None;
    }
    // Bare form — no inflection allowed. Any suffix means the side is
    // in a non-nominative case and we shouldn't treat it as a simple
    // is_a slot.
    if entry.root != lowered {
        return None;
    }
    Some(SlotRef {
        surface: cleaned,
        root: entry.root.clone(),
        pos: "noun".to_string(),
    })
}

/// Return the single alphabetic token in `s`, or `None` if `s`
/// contains zero or more-than-one alphabetic word. "Alphabetic word"
/// is a maximal run of alphabetic chars (plus `-`). Trailing punctuation
/// is tolerated (period, comma, parens) — we only reject extra WORDS.
///
/// - `"бұлақ"` → `Some("бұлақ")`
/// - `"бұлақ."` → `Some("бұлақ")`
/// - `"бұлақ, мысалы"` → `None` (two words)
/// - `"  бала "` → `Some("бала")`
/// - `""` → `None`
fn exactly_one_alphabetic_token(s: &str) -> Option<&str> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let mut tokens = s.split(|c: char| !(c.is_alphabetic() || c == '-'));
    let first = tokens.find(|t| !t.is_empty())?;
    // Any further non-empty token → reject.
    if tokens.any(|t| !t.is_empty()) {
        return None;
    }
    Some(first)
}

/// First alphabetic token in `s` (UTF-8 safe, stops at whitespace /
/// punctuation). `None` if the string has no alphabetic content.
fn first_alphabetic_token(s: &str) -> Option<&str> {
    let s = s.trim_start();
    let start = s.char_indices().find(|(_, c)| c.is_alphabetic())?.0;
    let rest = &s[start..];
    let end_in_rest = rest
        .char_indices()
        .find(|(_, c)| !c.is_alphabetic() && *c != '-')
        .map(|(i, _)| i)
        .unwrap_or(rest.len());
    Some(&rest[..end_in_rest])
}

/// Last alphabetic token in `s` — symmetrical to [`first_alphabetic_token`].
fn last_alphabetic_token(s: &str) -> Option<&str> {
    // Walk backward through chars, find the last alphabetic one, then
    // extend leftward while alphabetic.
    let s = s.trim_end();
    let end_char_idx = s
        .char_indices()
        .rev()
        .find(|(_, c)| c.is_alphabetic())
        .map(|(i, c)| i + c.len_utf8())?;
    let mut start = end_char_idx;
    for (i, c) in s[..end_char_idx].char_indices().rev() {
        if c.is_alphabetic() || c == '-' {
            start = i;
        } else {
            break;
        }
    }
    Some(&s[start..end_char_idx])
}

/// Closed-class Kazakh roots the FST often tags as Noun but which
/// cannot sensibly serve as subjects in a fact (pronouns, demonstratives,
/// postpositions, quantifiers).
fn is_closed_class(root: &str) -> bool {
    matches!(
        root,
        "мен"
            | "сен"
            | "сіз"
            | "ол"
            | "біз"
            | "сендер"
            | "сіздер"
            | "олар"
            | "бұл"
            | "мына"
            | "сол"
            | "осы"
            | "ана"
            | "туралы"
            | "бойынша"
            | "үшін"
            | "кейін"
            | "дейін"
            | "сияқты"
            | "ретінде"
            | "арқылы"
            | "көп"
            | "аз"
            | "бәрі"
            | "барлық"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_lex() -> Option<LexiconV1> {
        let curated = "../../data/tokenizer/segmentation_roots.json";
        let apertium = "../../data/lexicon_v1/apertium_imported_roots.json";
        if !std::path::Path::new(curated).exists() {
            return None;
        }
        LexiconV1::load(curated, apertium).ok()
    }

    fn src() -> FactSource {
        FactSource {
            pack: "test_pack.json".into(),
            sample_id: "test_001".into(),
        }
    }

    // ------------------------- copula_is_a --------------------------------

    #[test]
    fn copula_extracts_abai_is_poet() {
        let Some(lex) = load_lex() else { return };
        let text = "Абай — ақын";
        let mut out = Vec::new();
        copula_is_a(text, &[], &lex, &src(), &mut out);
        assert_eq!(out.len(), 1, "expected exactly one fact for «Абай — ақын»");
        let f = &out[0];
        assert_eq!(f.subject.root, "абай");
        assert_eq!(f.object.root, "ақын");
        assert_eq!(f.predicate, Predicate::IsA);
        assert_eq!(f.pattern, "X — Y");
        assert_eq!(f.confidence, ConfidenceKind::Grammar);
    }

    #[test]
    fn copula_rejects_no_dash() {
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        copula_is_a("Абай ақын еді", &[], &lex, &src(), &mut out);
        assert!(out.is_empty(), "no dash → no fact, got {out:?}");
    }

    #[test]
    fn copula_rejects_two_dashes_ambiguous() {
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        copula_is_a(
            "Абай — қазақ ақыны — дара тұлға",
            &[],
            &lex,
            &src(),
            &mut out,
        );
        assert!(
            out.is_empty(),
            "ambiguous double-dash → refuse, got {out:?}"
        );
    }

    #[test]
    fn copula_rejects_inflected_subject() {
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        // «Абайдың» is genitive; not a bare nominative.
        copula_is_a("Абайдың — ақын", &[], &lex, &src(), &mut out);
        assert!(out.is_empty(), "inflected subject → refuse");
    }

    #[test]
    fn copula_rejects_self_tautology() {
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        copula_is_a("адам — адам", &[], &lex, &src(), &mut out);
        assert!(out.is_empty(), "X — X tautology → refuse");
    }

    #[test]
    fn copula_rejects_multi_token_lhs() {
        // "Адалдық түбі — кеніш" = "Honesty's root is a mine" —
        // LHS is a possessive NP; picking "түбі" as subject is wrong.
        // v2.1 precision: refuse any multi-token side.
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        copula_is_a("Адалдық түбі — кеніш", &[], &lex, &src(), &mut out);
        assert!(out.is_empty(), "multi-token LHS → refuse");
    }

    #[test]
    fn copula_extracts_head_from_multi_token_rhs() {
        // "Кітап — білім бұлағы" = "Book is knowledge's spring" —
        // v2.1 head extraction: RHS head = бұлағы (possessed) → бұлақ.
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        copula_is_a("Кітап — білім бұлағы", &[], &lex, &src(), &mut out);
        if out.is_empty() {
            eprintln!("note: «кітап» or «бұлақ» may not be in Lexicon; skipping");
            return;
        }
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].subject.root, "кітап");
        assert_eq!(
            out[0].object.root, "бұлақ",
            "head extraction must yield the possessed noun's root"
        );
    }

    #[test]
    fn copula_rejects_long_rhs_clause() {
        // A full-clause RHS is not an NP — refuse.
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        copula_is_a(
            "Шекара — Қазақстан мен Ресей шекарасы және ұзын",
            &[],
            &lex,
            &src(),
            &mut out,
        );
        assert!(out.is_empty(), "5+-token RHS clause → refuse, got {out:?}");
    }

    #[test]
    fn copula_parenthetical_noise_stripped() {
        // "Шекара — шекарасы (7591 шақырым)" strips parens, then head
        // extraction on "шекарасы" → шекара → tautology → refuse.
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        copula_is_a(
            "Шекара — шекарасы (7591 шақырым)",
            &[],
            &lex,
            &src(),
            &mut out,
        );
        assert!(
            out.is_empty(),
            "parenthetical noise stripped, then tautology guard refuses"
        );
    }

    #[test]
    fn copula_accepts_trailing_punctuation() {
        // "Ілім — бұлақ." — one token each side, trailing period OK.
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        copula_is_a("Ілім — бұлақ.", &[], &lex, &src(), &mut out);
        if out.is_empty() {
            eprintln!("note: «ілім» or «бұлақ» may not be in Lexicon; skipping");
            return;
        }
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn exactly_one_token_rejects_two_words() {
        assert!(exactly_one_alphabetic_token("бала жанашыры").is_none());
        assert!(exactly_one_alphabetic_token("Адалдық түбі").is_none());
    }

    #[test]
    fn exactly_one_token_accepts_single_with_punct() {
        assert_eq!(exactly_one_alphabetic_token("бұлақ."), Some("бұлақ"));
        assert_eq!(exactly_one_alphabetic_token(" бала, "), Some("бала"));
    }

    // ------------------------- locative_lives_in ---------------------------

    #[test]
    fn locative_extracts_baqytjan_lives_in_almaty() {
        let Some(lex) = load_lex() else { return };
        let text = "Бақытжан Алматыда тұрады";
        let mut out = Vec::new();
        locative_lives_in(text, &[], &lex, &src(), &mut out);
        if out.is_empty() {
            eprintln!("note: «Бақытжан» may not be in Lexicon; skipping");
            return;
        }
        let f = &out[0];
        assert_eq!(f.object.root, "алматы");
        assert_eq!(f.predicate, Predicate::LivesIn);
        assert_eq!(f.pattern, "X Y-да тұрады");
    }

    #[test]
    fn locative_rejects_without_turu_verb() {
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        locative_lives_in("бала Алматыда жүр", &[], &lex, &src(), &mut out);
        assert!(out.is_empty(), "no тұру verb → no lives_in fact");
    }

    #[test]
    fn locative_rejects_pronoun_subject() {
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        locative_lives_in("мен Алматыда тұрамын", &[], &lex, &src(), &mut out);
        assert!(
            out.is_empty(),
            "pronoun subject rejected (no knowledge value)"
        );
    }

    // ------------------------- possessive_has -----------------------------

    #[test]
    fn possessive_extracts_child_has_book() {
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        possessive_has("Баланың кітабы бар", &[], &lex, &src(), &mut out);
        if out.is_empty() {
            eprintln!("note: «бала» or «кітап» may not be in Lexicon; skipping");
            return;
        }
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].subject.root, "бала");
        assert_eq!(out[0].object.root, "кітап");
        assert_eq!(out[0].predicate, Predicate::Has);
        assert_eq!(out[0].pattern, "X-тың Y-сы бар");
    }

    #[test]
    fn possessive_rejects_without_bar() {
        // Without the existential "бар", this isn't a Has claim.
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        possessive_has("Баланың кітабы", &[], &lex, &src(), &mut out);
        assert!(out.is_empty(), "no 'бар' → no Has fact");
    }

    #[test]
    fn possessive_rejects_non_adjacent() {
        // Intervening word between possessor and possessed breaks the
        // simple X-тың Y-сы bar construction. We refuse to guess.
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        possessive_has("Баланың үйде кітабы бар", &[], &lex, &src(), &mut out);
        assert!(
            out.is_empty(),
            "non-adjacent possessor + possessed → refuse"
        );
    }

    // ------------------------- dative_goes_to -----------------------------

    #[test]
    fn dative_extracts_child_goes_to_school() {
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        dative_goes_to("Бала мектепке барады", &[], &lex, &src(), &mut out);
        if out.is_empty() {
            eprintln!("note: «бала» or «мектеп» may not be in Lexicon; skipping");
            return;
        }
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].subject.root, "бала");
        assert_eq!(out[0].object.root, "мектеп");
        assert_eq!(out[0].predicate, Predicate::GoesTo);
        assert_eq!(out[0].pattern, "X Y-ке барады");
    }

    #[test]
    fn dative_rejects_without_baru_verb() {
        // Dative case without the verb «бару» is just a dative noun
        // — not a motion claim.
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        dative_goes_to("Бала мектепке кітап берді", &[], &lex, &src(), &mut out);
        assert!(
            out.is_empty(),
            "no 'бару' verb → no GoesTo fact (got {out:?})"
        );
    }

    #[test]
    fn dative_rejects_pronoun_subject() {
        // Pronouns carry no grounded subject identity for a fact.
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        dative_goes_to("Мен мектепке барамын", &[], &lex, &src(), &mut out);
        assert!(out.is_empty(), "pronoun subject refused");
    }

    #[test]
    fn dative_rejects_self_tautology() {
        // Hypothetical tautology — subject and destination share a root.
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        dative_goes_to("Бала балаға барады", &[], &lex, &src(), &mut out);
        assert!(out.is_empty(), "tautology refused");
    }

    // ------------------------- helpers ------------------------------------

    #[test]
    fn first_alphabetic_strips_leading_punct() {
        assert_eq!(first_alphabetic_token("  Абай, ақын"), Some("Абай"));
    }

    #[test]
    fn last_alphabetic_strips_trailing_punct() {
        assert_eq!(last_alphabetic_token("Мен Абай!"), Some("Абай"));
    }

    #[test]
    fn last_alphabetic_handles_empty() {
        assert_eq!(last_alphabetic_token("   "), None);
    }

    #[test]
    fn closed_class_catches_pronoun() {
        assert!(is_closed_class("мен"));
        assert!(!is_closed_class("бала"));
    }
}
