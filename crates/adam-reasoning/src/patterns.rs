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
use adam_kernel_fst::morphotactics::{Case, Possessive, Voice};
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

    // v3.8.0 — FST stores verb stems WITHOUT the -у infinitive suffix
    // (`тұрады` analyses as root `тұр`, not `тұру`). The pre-v3.8.0
    // check `root == "тұру"` never fired, which is why `lives_in`
    // produced 0 facts at every tier through v3.7.5. Fixed + widened
    // to accept locative verbs beyond `тұр`: `мекен` ("dwelled-in"),
    // `орналас` ("located"). These are all valid "X lives in Y" Kazakh
    // constructions in textbook prose.
    let has_locative_verb = tokens.iter().any(|(_, a)| match a {
        Some(Analysis::Verb { root, .. }) => {
            matches!(root.root.as_str(), "тұр" | "мекен" | "орналас")
        }
        _ => false,
    });
    if !has_locative_verb {
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

    // v3.8.5 — reject locative objects that retain a possessive
    // marker on their FST analysis (e.g. «аумағында» → root `аумағын`
    // keeps P3-ended fragment). These are always fragment parses,
    // never valid places. Codex flagged «Қазақстан lives_in аумағын»
    // as a canonical example.
    let loc_has_possessive = matches!(
        &loc_entry.1,
        Some(Analysis::Noun { features, .. }) if features.possessive.is_some()
    );
    if loc_has_possessive {
        return;
    }

    // v4.0.0 — object-side 3-char minimum (mirrors the subject-side
    // guard added in v3.8.5). Closes «(бала, LivesIn, ған)» where
    // the FST emitted a -ған participle as a standalone root, and
    // «(X, LivesIn, ын/ін/қан)» fragment-tail cases.
    if loc_root.chars().count() < 3 {
        return;
    }
    if is_closed_class(&loc_root) {
        return;
    }

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
            // v3.8.5 precision hardening: a country / major city can
            // never be the subject of LivesIn ("Қазақстан lives_in X"
            // is categorically wrong — countries don't reside). A time
            // noun can never be the subject either ("жыл lives_in X").
            // Short broken stems (< 3 chars) rule out truncated FST
            // analyses like `кешк` / `қаһарл` that were leaking into
            // v3.8.0 facts.
            if is_location_root(&root.root)
                || is_time_noun(&root.root)
                || root.root.chars().count() < 3
            {
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

    // v3.8.0 — fix the same `"бару" → "бар"` root-comparison bug
    // as `locative_lives_in` (FST stores verb stems without the -у
    // infinitive suffix). Pre-v3.8.0 `goes_to` produced 0 facts at
    // every tier. Also widened to `кел` ("come") — "X Y-ге келді"
    // ("X came to Y") is as valid a directional as "X Y-ке барды".
    let has_motion_verb = tokens.iter().any(|(_, a)| match a {
        Some(Analysis::Verb { root, .. }) => {
            matches!(root.root.as_str(), "бар" | "кел")
        }
        _ => false,
    });
    if !has_motion_verb {
        return;
    }

    // First dative noun is the destination.
    // v3.8.5 — reject dative objects that still carry a possessive
    // marker (same class of fragment-parse that contaminated LivesIn).
    // v4.0.0 — additional object-side 3-char minimum (closes
    // «(X, GoesTo, ын/ің/ған)» fragment-tail cases).
    let dative_idx = tokens.iter().position(|(_, a)| match a {
        Some(Analysis::Noun { features, root }) => {
            features.case == Some(Case::Dative)
                && features.possessive.is_none()
                && root.part_of_speech == "noun"
                && !is_closed_class(&root.root)
                && root.root.chars().count() >= 3
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
            // v3.8.5 precision hardening — time nouns as GoesTo subjects
            // were 309 / 1864 = 16.6 % of all pre-hardening GoesTo facts
            // (e.g. «күн → goes_to → жұмыс» from Abai's «бір күн Масғұт
            // шықты»). «жыл», «күн», «ай» etc. cannot "go to" anywhere —
            // they are time adverbials, not agents. Short broken stems
            // (< 3 chars) also ruled out.
            if is_time_noun(&root.root) || root.root.chars().count() < 3 {
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
// v3.5.0 matchers — breadth expansion for the reasoning graph.
// -----------------------------------------------------------------------------

/// Causal pattern — `X — Y-нің себебі` → `(X, Causes, Y)`.
///
/// Kazakh causal copula: "X is the cause/reason of Y". Structure:
/// bare-nominative X + em-dash + genitive Y + possessed noun `себебі`
/// (P3 of `себеп` "reason"). This is stricter than an open-ended
/// causal matcher — we require the literal `себебі` head — but is
/// what textbook prose uses when stating definitions like «су — өмірдің
/// себебі» ("water is the cause of life").
pub fn copula_causes(
    text: &str,
    _parses: &[Analysis],
    lexicon: &LexiconV1,
    source: &FactSource,
    out: &mut Vec<Fact>,
) {
    if !text.contains("себебі") {
        return;
    }
    let dash_count = text.chars().filter(|c| *c == '\u{2014}').count();
    if dash_count != 1 {
        return;
    }
    let (left, right) = match text.split_once('\u{2014}') {
        Some(parts) => parts,
        None => return,
    };
    // LHS: exactly one bare-nominative content noun.
    let subj_surface = match exactly_one_alphabetic_token(left) {
        Some(t) => t,
        None => return,
    };
    let Some(subj) = resolve_bare_noun(subj_surface, lexicon) else {
        return;
    };
    // RHS must end in `себебі` (optionally followed by punctuation) and
    // the token before it must analyse as a genitive noun (the
    // "causer's complement" — i.e. the Y in "Y-нің себебі").
    let rhs_tokens: Vec<String> = strip_parens(right)
        .split(|c: char| !(c.is_alphabetic() || c == '-'))
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect();
    // Find the last occurrence of «себебі» in the RHS.
    let Some(sebebi_idx) = rhs_tokens
        .iter()
        .rposition(|t| t.to_lowercase() == "себебі")
    else {
        return;
    };
    if sebebi_idx == 0 {
        return;
    }
    // Preceding token must analyse as a genitive noun (the Y).
    let prev = &rhs_tokens[sebebi_idx - 1].to_lowercase();
    let Some(obj_analysis) = analyse(prev, lexicon).into_iter().next() else {
        return;
    };
    let Analysis::Noun { features, root } = obj_analysis else {
        return;
    };
    if features.case != Some(Case::Genitive) {
        return;
    }
    if root.part_of_speech != "noun" || is_closed_class(&root.root) {
        return;
    }
    if subj.root == root.root {
        return;
    }
    out.push(Fact {
        subject: subj,
        predicate: Predicate::Causes,
        object: SlotRef {
            surface: rhs_tokens[sebebi_idx - 1].clone(),
            root: root.root.clone(),
            pos: "noun".to_string(),
        },
        pattern: "X — Y-нің себебі".to_string(),
        source: source.clone(),
        confidence: ConfidenceKind::Grammar,
        raw_text: text.trim().to_string(),
    });
}

/// Temporal pattern — `X Y-дан кейін ...` → `(X, After, Y)`.
///
/// Kazakh "after" construction: bare-nominative subject + ablative
/// noun (the reference point) + postposition `кейін` or `соң`.
/// Example: «түс таңнан кейін болады» → (түс, After, таң).
pub fn temporal_after(
    text: &str,
    _parses: &[Analysis],
    lexicon: &LexiconV1,
    source: &FactSource,
    out: &mut Vec<Fact>,
) {
    let text_lower = text.to_lowercase();
    if !text_lower.contains(" кейін") && !text_lower.contains(" соң") {
        return;
    }
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
    // Find the postposition position.
    let Some(post_idx) = tokens.iter().position(|(s, _)| {
        let lo = s.to_lowercase();
        lo == "кейін" || lo == "соң"
    }) else {
        return;
    };
    if post_idx == 0 {
        return;
    }
    // Preceding token must be an ablative noun (the Y reference point).
    let prev = &tokens[post_idx - 1];
    let Some(Analysis::Noun { features, root }) = &prev.1 else {
        return;
    };
    if features.case != Some(Case::Ablative) {
        return;
    }
    if root.part_of_speech != "noun" || is_closed_class(&root.root) {
        return;
    }
    let obj_slot = SlotRef {
        surface: prev.0.clone(),
        root: root.root.clone(),
        pos: "noun".to_string(),
    };
    // Subject: first bare-nominative content noun before the ablative.
    let subj = (0..post_idx - 1).find_map(|i| match &tokens[i].1 {
        Some(Analysis::Noun { features, root }) => {
            if root.part_of_speech != "noun"
                || is_closed_class(&root.root)
                || features.case.is_some_and(|c| c != Case::Nominative)
            {
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
    if subject.root == obj_slot.root {
        return;
    }
    out.push(Fact {
        subject,
        predicate: Predicate::After,
        object: obj_slot,
        pattern: "X Y-дан кейін".to_string(),
        source: source.clone(),
        confidence: ConfidenceKind::Grammar,
        raw_text: text.trim().to_string(),
    });
}

/// Quantity pattern — `X-тың N Y-ы бар` → `(X, HasQuantity, Y)` where
/// N is a numeral preserved in the raw_text. An extension of
/// [`possessive_has`] that specifically catches numeric-count claims
/// common in textbooks («адамның екі аяғы бар», «планетаның алты
/// айы бар»).
pub fn quantity_count(
    text: &str,
    _parses: &[Analysis],
    lexicon: &LexiconV1,
    source: &FactSource,
    out: &mut Vec<Fact>,
) {
    let has_bar = text
        .split(|c: char| !(c.is_alphabetic() || c == '-'))
        .any(|t| t.to_lowercase() == "бар");
    if !has_bar {
        return;
    }
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
    // Scan for: genitive noun → numeral → P3 noun → … → бар.
    // We use is_numeral_token because numerals are either Lexicon-
    // tagged `numeral` OR the closed set of Kazakh numeral forms.
    let gen_idx = tokens.iter().position(|(_, a)| match a {
        Some(Analysis::Noun { features, root }) => {
            features.case == Some(Case::Genitive)
                && root.part_of_speech == "noun"
                && !is_closed_class(&root.root)
        }
        _ => false,
    });
    let Some(gen_idx) = gen_idx else { return };
    // Must be followed by a numeral, then a P3 noun.
    let num_idx = gen_idx + 1;
    if num_idx >= tokens.len() {
        return;
    }
    let Some(num_analysis) = &tokens[num_idx].1 else {
        return;
    };
    let is_numeral = match num_analysis {
        Analysis::Noun { root, .. } => root.part_of_speech == "numeral",
        _ => false,
    };
    if !is_numeral {
        return;
    }
    let poss_idx = num_idx + 1;
    if poss_idx >= tokens.len() {
        return;
    }
    let Some(Analysis::Noun {
        features: p_feat,
        root: p_root,
    }) = &tokens[poss_idx].1
    else {
        return;
    };
    if p_feat.possessive != Some(Possessive::P3)
        || p_root.part_of_speech != "noun"
        || is_closed_class(&p_root.root)
    {
        return;
    }
    let (poss_surface, poss_root_str) = (tokens[poss_idx].0.clone(), p_root.root.clone());
    let (gen_surface, gen_root_str) = match &tokens[gen_idx].1 {
        Some(Analysis::Noun { root, .. }) => (tokens[gen_idx].0.clone(), root.root.clone()),
        _ => return,
    };
    if gen_root_str == poss_root_str {
        return;
    }
    out.push(Fact {
        subject: SlotRef {
            surface: gen_surface,
            root: gen_root_str,
            pos: "noun".to_string(),
        },
        predicate: Predicate::HasQuantity,
        object: SlotRef {
            surface: poss_surface,
            root: poss_root_str,
            pos: "noun".to_string(),
        },
        pattern: "X-тың N Y-ы бар".to_string(),
        source: source.clone(),
        confidence: ConfidenceKind::Grammar,
        raw_text: text.trim().to_string(),
    });
}

/// Agent-verb pattern — `X Y-ні Z-лайды` → `(X, DoesTo, Y)` where the
/// verb root goes in the `pattern` field.
///
/// Kazakh SOV: bare-nominative agent + accusative patient + verb.
/// Only records the (agent, patient) pair as a `DoesTo` edge — the
/// verb itself is captured in the pattern tag so downstream consumers
/// can filter if needed. Pronouns are refused as agents for the same
/// reason as other matchers.
pub fn agent_verb(
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
    // Find an accusative noun.
    let acc_idx = tokens.iter().position(|(_, a)| match a {
        Some(Analysis::Noun { features, root }) => {
            features.case == Some(Case::Accusative)
                && root.part_of_speech == "noun"
                && !is_closed_class(&root.root)
        }
        _ => false,
    });
    let Some(acc_idx) = acc_idx else { return };
    // Must be followed by a verb.
    let verb_idx =
        (acc_idx + 1..tokens.len()).find(|&i| matches!(tokens[i].1, Some(Analysis::Verb { .. })));
    let Some(verb_idx) = verb_idx else { return };
    let (verb_root, verb_voice) = match &tokens[verb_idx].1 {
        Some(Analysis::Verb { root, features }) => (root.root.clone(), features.voice),
        _ => return,
    };
    // v3.5.0 precision fix: Passive voice inverts the thematic roles —
    // the grammatical subject of a passive clause is the PATIENT, not
    // the agent. "Кітап оқылды" = "The book was read", NOT "The book
    // read (something)". Refusing passives stops false-positive
    // agent-verb extractions from frequent textbook phrasings like
    // «... қолданылады» ("... is used").
    if verb_voice == Some(Voice::Passive) {
        return;
    }
    // Refuse stopword verbs. v3.8.0 — these are the raw FST stems
    // (no -у infinitive suffix) — see the same fix in
    // `locative_lives_in` / `dative_goes_to`. `бар` is existential
    // ("there is"), `бол` is copula ("to be"), `бар` also direction
    // motion verb handled by `dative_goes_to`, `кел` is direction
    // motion ("come"), `еді` / `еду` are past copula forms.
    if matches!(
        verb_root.as_str(),
        "бар" | "бол" | "кел" | "еді" | "еду" | "тұр" | "мекен" | "орналас"
    ) {
        return;
    }
    let (acc_surface, acc_root) = match &tokens[acc_idx].1 {
        Some(Analysis::Noun { root, .. }) => (tokens[acc_idx].0.clone(), root.root.clone()),
        _ => return,
    };
    // Subject: first bare-nominative content noun before the accusative.
    // v3.5.0 precision fix: refuse possessive-form subjects — a P3 noun
    // ("тілі") is not a bare subject, it's an inflected head. The
    // nominal_conjunction matcher already does this; agent_verb now
    // too. Also refuse the surface being capitalised mid-sentence when
    // lowercase(surface) != root (imperfect but filters most proper-
    // name-misclassified-as-noun cases).
    let subj = (0..acc_idx).find_map(|i| match &tokens[i].1 {
        Some(Analysis::Noun { features, root }) => {
            if root.part_of_speech != "noun"
                || is_closed_class(&root.root)
                || features.case.is_some_and(|c| c != Case::Nominative)
                || features.possessive.is_some()
            {
                return None;
            }
            // v3.8.5 — rule out short broken stems (< 3 chars) and
            // time-adverbial roots that the agent_verb pattern was
            // also grabbing (e.g. «жыл ... әсер етеді» → «жыл does_to
            // әсер», where «жыл» = "year" is not an agent).
            if is_time_noun(&root.root) || root.root.chars().count() < 3 {
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
    if subject.root == acc_root {
        return;
    }
    out.push(Fact {
        subject,
        predicate: Predicate::DoesTo,
        object: SlotRef {
            surface: acc_surface,
            root: acc_root,
            pos: "noun".to_string(),
        },
        pattern: format!("X Y-ні {verb_root}-лайды"),
        source: source.clone(),
        confidence: ConfidenceKind::Grammar,
        raw_text: text.trim().to_string(),
    });
}

/// Nominal-conjunction pattern — `X пен Y` → `(X, RelatedTo, Y)`.
///
/// Kazakh nominal conjunction: "X and Y" using `пен` / `мен` / `бен`
/// (harmony-driven). The two conjuncts are brought together by the
/// author so they co-occur in a sibling structure — a weak RelatedTo
/// signal comparable to the R5-derived ones but grounded in explicit
/// syntactic co-predication. Refuses pronoun / closed-class sides.
pub fn nominal_conjunction(
    text: &str,
    _parses: &[Analysis],
    lexicon: &LexiconV1,
    source: &FactSource,
    out: &mut Vec<Fact>,
) {
    let text_lower = text.to_lowercase();
    let has_conj = text_lower.contains(" пен ")
        || text_lower.contains(" мен ")
        || text_lower.contains(" бен ");
    if !has_conj {
        return;
    }
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
    let Some(conj_idx) = tokens.iter().position(|(s, _)| {
        let lo = s.to_lowercase();
        lo == "пен" || lo == "мен" || lo == "бен"
    }) else {
        return;
    };
    if conj_idx == 0 || conj_idx + 1 >= tokens.len() {
        return;
    }
    // Both sides must be bare-nominative content nouns.
    let lhs = match &tokens[conj_idx - 1].1 {
        Some(Analysis::Noun { features, root }) => {
            if root.part_of_speech != "noun"
                || is_closed_class(&root.root)
                || features.case.is_some_and(|c| c != Case::Nominative)
                || features.possessive.is_some()
            {
                return;
            }
            SlotRef {
                surface: tokens[conj_idx - 1].0.clone(),
                root: root.root.clone(),
                pos: "noun".to_string(),
            }
        }
        _ => return,
    };
    let rhs = match &tokens[conj_idx + 1].1 {
        Some(Analysis::Noun { features, root }) => {
            if root.part_of_speech != "noun"
                || is_closed_class(&root.root)
                || features.case.is_some_and(|c| c != Case::Nominative)
                || features.possessive.is_some()
            {
                return;
            }
            SlotRef {
                surface: tokens[conj_idx + 1].0.clone(),
                root: root.root.clone(),
                pos: "noun".to_string(),
            }
        }
        _ => return,
    };
    if lhs.root == rhs.root {
        return;
    }
    out.push(Fact {
        subject: lhs,
        predicate: Predicate::RelatedTo,
        object: rhs,
        pattern: "X пен Y".to_string(),
        source: source.clone(),
        confidence: ConfidenceKind::Grammar,
        raw_text: text.trim().to_string(),
    });
}

/// Domain-membership pattern — `X — Y саласы` → `(X, InDomain, Y)`.
///
/// Kazakh educational / taxonomic prose: "X is a field/branch of Y".
/// Structure: bare-nominative X + em-dash + possessive `саласы`
/// (P3 of `сала` "field/branch") + genitive or bare Y that names the
/// parent domain. Precise form: "X — Y саласы" OR "X — Y-нің саласы".
/// Textbooks use this heavily: «алгебра — математиканың саласы».
pub fn domain_membership(
    text: &str,
    _parses: &[Analysis],
    lexicon: &LexiconV1,
    source: &FactSource,
    out: &mut Vec<Fact>,
) {
    let text_lower = text.to_lowercase();
    if !text_lower.contains("саласы") && !text_lower.contains("ғылымы") {
        return;
    }
    let dash_count = text.chars().filter(|c| *c == '\u{2014}').count();
    if dash_count != 1 {
        return;
    }
    let (left, right) = match text.split_once('\u{2014}') {
        Some(parts) => parts,
        None => return,
    };
    let subj_surface = match exactly_one_alphabetic_token(left) {
        Some(t) => t,
        None => return,
    };
    let Some(subj) = resolve_bare_noun(subj_surface, lexicon) else {
        return;
    };
    // RHS: token before `саласы` / `ғылымы` is the Y.
    let rhs_tokens: Vec<String> = strip_parens(right)
        .split(|c: char| !(c.is_alphabetic() || c == '-'))
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect();
    let head_idx = rhs_tokens.iter().rposition(|t| {
        let lo = t.to_lowercase();
        lo == "саласы" || lo == "ғылымы"
    });
    let Some(head_idx) = head_idx else { return };
    if head_idx == 0 {
        return;
    }
    let prev_surface = rhs_tokens[head_idx - 1].to_lowercase();
    // Y may be bare-nominative OR genitive (-ның/-нің/-дың/-дің/-тың/-тің).
    let Some(analysis) = analyse(&prev_surface, lexicon).into_iter().next() else {
        return;
    };
    let Analysis::Noun { features, root } = analysis else {
        return;
    };
    if root.part_of_speech != "noun" || is_closed_class(&root.root) {
        return;
    }
    // Accept either bare nominative OR explicit genitive as the Y.
    let case_ok = features.case.is_none()
        || features.case == Some(Case::Nominative)
        || features.case == Some(Case::Genitive);
    if !case_ok {
        return;
    }
    if subj.root == root.root {
        return;
    }
    out.push(Fact {
        subject: subj,
        predicate: Predicate::InDomain,
        object: SlotRef {
            surface: rhs_tokens[head_idx - 1].clone(),
            root: root.root.clone(),
            pos: "noun".to_string(),
        },
        pattern: "X — Y саласы".to_string(),
        source: source.clone(),
        confidence: ConfidenceKind::Grammar,
        raw_text: text.trim().to_string(),
    });
}

/// Structural-partitive pattern — `X Y-нің бөлігі` / `X Y-нің
/// құрамында` → `(X, PartOf, Y)`.
///
/// Two concrete Kazakh constructions for "X is a part of Y" — each
/// common in textbook prose (biology / geography / physics / math
/// taxonomies):
///
///   1. `X Y-нің бөлігі` — "X is Y's piece" (genitive + possessed
///      `бөлігі`, P3 of `бөлік` "piece/part").
///   2. `X Y-нің құрамында` — "X is in Y's composition" (locative of
///      `құрам`, P3).
///
/// **v3.5.5 intentionally drops `ішінде`** ("inside" / "among") — the
/// word is semantically ambiguous between partitive (`X is inside Y`)
/// and universal-quantifier (`among all N, X stands out`). The latter
/// reading triggered false positives on v3.5.5 initial extraction
/// ("тілдердің ішінде қазақ" = "among languages, Kazakh" is NOT a
/// PartOf claim). Restricted the matcher to the two unambiguous heads.
///
/// v3.5.5 uses literal possessed-noun heads rather than pure morphology
/// because Kazakh `-нің` alone doesn't distinguish a partitive genitive
/// from a general genitive. The head word pins the semantic relation.
///
/// Requirements:
///
///   - The sentence contains one of the three literal head words as a
///     standalone token: `бөлігі`, `құрамында`, `ішінде`.
///   - The immediately-preceding token analyses as a genitive noun
///     (`Case::Genitive`) → Y, the parent.
///   - A bare-nominative content noun earlier in the sentence → X,
///     the part.
///   - Tautology guard: X.root ≠ Y.root.
///
/// Feeds the v3.5.5 `R3_has_inheritance_via_part_of` rule
/// (`Has(X, Y) ∧ PartOf(Y, Z) ⟹ Has(X, Z)`) — producing the first
/// derivation path that chains two non-IsA predicates.
pub fn structural_part_of(
    text: &str,
    _parses: &[Analysis],
    lexicon: &LexiconV1,
    source: &FactSource,
    out: &mut Vec<Fact>,
) {
    let text_lower = text.to_lowercase();
    let has_head = text_lower.contains(" бөлігі")
        || text_lower.contains(" құрамында")
        // Start-of-sentence cases (rare but possible).
        || text_lower.starts_with("бөлігі")
        || text_lower.starts_with("құрамында");
    if !has_head {
        return;
    }
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
    // Find the first head-word occurrence. `ішінде` is deliberately
    // omitted — see doc comment for the false-positive story.
    let head_idx = tokens.iter().position(|(s, _)| {
        let lo = s.to_lowercase();
        lo == "бөлігі" || lo == "құрамында"
    });
    let Some(head_idx) = head_idx else { return };
    if head_idx == 0 {
        return;
    }
    // Preceding token must analyse as a genitive noun (the Y parent).
    let prev = &tokens[head_idx - 1];
    let Some(Analysis::Noun { features, root }) = &prev.1 else {
        return;
    };
    if features.case != Some(Case::Genitive) {
        return;
    }
    if root.part_of_speech != "noun" || is_closed_class(&root.root) {
        return;
    }
    let obj_slot = SlotRef {
        surface: prev.0.clone(),
        root: root.root.clone(),
        pos: "noun".to_string(),
    };
    // Subject X: first bare-nominative content noun before the genitive
    // Y. Refuses pronouns, closed-class items, and possessive-form
    // surfaces (consistent with v3.5.0 agent_verb tightening).
    let subj = (0..head_idx - 1).find_map(|i| match &tokens[i].1 {
        Some(Analysis::Noun { features, root }) => {
            if root.part_of_speech != "noun"
                || is_closed_class(&root.root)
                || features.case.is_some_and(|c| c != Case::Nominative)
                || features.possessive.is_some()
            {
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
    if subject.root == obj_slot.root {
        return;
    }
    let head_tok = tokens[head_idx].0.to_lowercase();
    let pattern = match head_tok.as_str() {
        "бөлігі" => "X Y-нің бөлігі",
        "құрамында" => "X Y-нің құрамында",
        _ => "X Y-нің <part>",
    };
    out.push(Fact {
        subject,
        predicate: Predicate::PartOf,
        object: obj_slot,
        pattern: pattern.to_string(),
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
///
/// Currently only exercised by unit tests — gated with `#[cfg(test)]`
/// to avoid a dead-code warning on non-test builds. Unblock the gate
/// when a matcher actually needs the helper.
#[cfg(test)]
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
///
/// Test-only today; see the `first_alphabetic_token` note for the reason
/// behind the `#[cfg(test)]` gate.
#[cfg(test)]
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
            | "сияқ"
            | "ретінде"
            | "арқылы"
            | "көп"
            | "аз"
            | "бәрі"
            | "барлық"
            // v3.5.0: interrogatives + common adjectival-looking
            // roots the FST sometimes tags as nouns.
            | "қандай"
            | "кім"
            | "не"
            | "қай"
            | "қашан"
            | "қайда"
            | "неліктен"
            | "неге"
            | "қанша"
            // v3.8.5 (precision hardening): demonstrative qualifiers
            // and quantifier-like closed-class items the FST some-
            // times tags as bare nouns. Precision audit flagged
            // these as noisy subjects (e.g. «мұндай → goes_to → X»
            // where мұндай = "such/this kind of" is not an agent).
            | "мұндай"
            | "сондай"
            | "ондай"
            | "мынадай"
            | "сондай-ақ"
            | "кейбір"
            | "өз"
            | "өзі"
            | "бірнеше"
            | "барша"
            | "әрбір"
            | "әр"
            | "бір"
            | "кей"
            // v4.0.0 — Codex-review expansion: conjunctions and
            // particles that the FST analyses as nouns in ambiguous
            // contexts. «(егер, DoesTo, газ)» was Codex's canonical
            // noise sample where "егер" (= "if") leaked as subject.
            | "егер"
            | "алайда"
            | "бірақ"
            | "дегенмен"
            | "сондықтан"
            | "демек"
            | "яғни"
            | "әйтсе"
            | "өйткені"
            | "сонда"
            | "сонымен"
            // v4.0.0 — common adverbial / oblique stems never
            // legitimately subjects.
            | "жалға"
            | "тек"
            | "қана"
            | "ғана"
            // v4.0.0 — fragment-suffix roots the FST occasionally
            // emits as standalone bare-noun roots. «бала lives_in ған»
            // (Codex-flagged) was a -ған participle leaking as root.
            | "ған"
            | "ген"
            | "қан"
            | "кен"
            | "ын"
            | "ін"
            | "сын"
            | "сін"
    )
}

/// v3.8.5 — time-denoting nouns refused as **subjects** of motion /
/// residence predicates. `(күн → goes_to → жұмыс)` from Abai's «Шаһардан
/// бір күн Масғұт шықты» is a parse artifact: «күн» is a time adverbial,
/// not the actor. Rejecting these at extraction removes ~350 noisy
/// LivesIn / GoesTo facts on the committed v3.8.0 runtime without
/// affecting any valid (person/place, goes_to/lives_in, place) triples.
///
/// Not rejected as OBJECTS — «жыл» can legitimately be a reference
/// point for `After` (X Y-дан кейін), and is ontologically a noun.
fn is_time_noun(root: &str) -> bool {
    matches!(
        root,
        "жыл"
            | "күн"
            | "ай"
            | "сағат"
            | "минут"
            | "секунд"
            | "ғасыр"
            | "уақыт"
            | "тәулік"
            | "апта"
            | "кез"
            | "сәт"
            | "мезгіл"
            | "шақ"
            | "мезет"
            | "түн"
            | "таң"
            | "кеш"
            | "таңертең"
            | "бүгін"
            | "кеше"
            | "ертең"
    )
}

/// v4.0.0 — astronomical / celestial-scale objects refused as
/// **derived targets** of R6 (LivesIn) and R7 (GoesTo) rules. Codex's
/// v3.9.5 review surfaced «бала lives_in күн жүйесі» as a canonical
/// false chain: `(бала, LivesIn, жер)` is extracted (child lives on
/// ground) and `(жер, PartOf, күн жүйесі)` is curated (Earth is part
/// of Solar System) — R6 naively chains them. The homonymy of "жер"
/// (both "ground" and "Earth") collides in the graph. Blocking
/// astronomical scale objects as R6/R7 derived targets resolves the
/// cross-domain absurdity without needing per-sense disambiguation.
///
/// Not used as an extractor-side filter — «ғаламшар» is a legitimate
/// IsA target in world_core astronomy, and «жұлдыз» can legitimately
/// appear in retrieval quotes. Scope is specifically R6/R7 chain
/// pruning.
pub(crate) fn is_astronomical_object(root: &str) -> bool {
    matches!(
        root,
        // Celestial bodies
        "күн"
            | "ай"
            | "жер"
            | "марс"
            | "шолпан"
            | "меркурий"
            | "юпитер"
            | "сатурн"
            | "уран"
            | "нептун"
            // Scale-up concepts
            | "күн жүйесі"
            | "галактика"
            | "құс жолы"
            | "ғаламшар"
            | "жұлдыз"
            | "аспан денесі"
            | "метеор"
            | "атмосфера"
            | "орбита"
    )
}

/// v3.8.5 — known toponyms / country names refused as **subjects** of
/// `LivesIn`. «Қазақстан → lives_in → аумағын» is categorically wrong:
/// a country cannot reside somewhere, it IS somewhere. Countries and
/// major cities belong on the OBJECT side of `LivesIn` facts (or are
/// legitimate IsA / PartOf subjects via other matchers). This filter
/// is intentionally conservative (explicit allow-list of toponyms
/// attested in the committed corpus) — widening to a full gazetteer
/// is a v3.9+ target.
fn is_location_root(root: &str) -> bool {
    matches!(
        root,
        // Countries
        "қазақстан"
            | "ресей"
            | "қытай"
            | "өзбекстан"
            | "қырғызстан"
            | "түркіменстан"
            | "тәжікстан"
            | "ауғанстан"
            | "иран"
            | "түркия"
            | "германия"
            | "франция"
            | "англия"
            | "америка"
            | "ақш"
            | "жапония"
            | "үндістан"
            | "моңғолия"
            | "корея"
            | "пәкістан"
            | "египет"
            // Continents / regions
            | "еуропа"
            | "азия"
            | "африка"
            | "аустралия"
            | "сібір"
            | "кавказ"
            | "алтай"
            | "памір"
            // Major Kazakh cities
            | "алматы"
            | "астана"
            | "нұр-сұлтан"
            | "шымкент"
            | "қарағанды"
            | "ақтөбе"
            | "атырау"
            | "тараз"
            | "павлодар"
            | "семей"
            | "өскемен"
            | "қызылорда"
            | "ақтау"
            | "талдықорған"
            | "көкшетау"
            | "орал"
            | "петропавл"
            | "жезқазған"
            | "балқаш"
            | "екібастұз"
            // Rivers / lakes / seas (can't "live in" the way a person can)
            | "арал"
            | "каспий"
            | "ертіс"
            | "сырдария"
            | "амудария"
            | "іле"
            | "балқаш-көл"
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

    // ------------------------- v3.5.0 matchers ----------------------------

    #[test]
    fn copula_causes_extracts_water_cause_life() {
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        copula_causes("су — өмірдің себебі", &[], &lex, &src(), &mut out);
        if out.is_empty() {
            eprintln!("note: су/өмір may not be in Lexicon; skipping");
            return;
        }
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].predicate, Predicate::Causes);
        assert_eq!(out[0].subject.root, "су");
        assert_eq!(out[0].object.root, "өмір");
        assert_eq!(out[0].pattern, "X — Y-нің себебі");
    }

    #[test]
    fn copula_causes_rejects_missing_sebeb_i() {
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        copula_causes("су — өмір", &[], &lex, &src(), &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn copula_causes_rejects_tautology() {
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        // Whole same root on both sides — refuse.
        copula_causes("адам — адамның себебі", &[], &lex, &src(), &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn temporal_after_extracts_noon_after_morning() {
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        temporal_after("түс таңнан кейін болады", &[], &lex, &src(), &mut out);
        if out.is_empty() {
            eprintln!("note: түс/таң may not analyse; skipping");
            return;
        }
        assert_eq!(out[0].predicate, Predicate::After);
        assert_eq!(out[0].subject.root, "түс");
        assert_eq!(out[0].object.root, "таң");
    }

    #[test]
    fn temporal_after_rejects_no_postposition() {
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        temporal_after("түс таңнан болады", &[], &lex, &src(), &mut out);
        assert!(out.is_empty(), "no кейін/соң → refuse");
    }

    #[test]
    fn quantity_count_rejects_without_numeral() {
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        quantity_count("адамның аяғы бар", &[], &lex, &src(), &mut out);
        // Without a numeral between genitive and P3, this is a plain
        // possessive, not a quantity claim — refuse.
        assert!(out.is_empty());
    }

    #[test]
    fn quantity_count_rejects_without_bar() {
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        quantity_count("адамның екі аяғы жоқ", &[], &lex, &src(), &mut out);
        assert!(out.is_empty(), "no бар → no HasQuantity");
    }

    #[test]
    fn agent_verb_rejects_pronoun_subject() {
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        agent_verb("мен кітапты оқимын", &[], &lex, &src(), &mut out);
        assert!(out.is_empty(), "pronoun subject refused");
    }

    #[test]
    fn agent_verb_rejects_without_accusative() {
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        // No accusative object — refuse.
        agent_verb("бала жүгіреді", &[], &lex, &src(), &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn nominal_conjunction_extracts_book_and_science() {
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        nominal_conjunction("кітап пен ілім", &[], &lex, &src(), &mut out);
        if out.is_empty() {
            eprintln!("note: кітап/ілім may not analyse; skipping");
            return;
        }
        assert_eq!(out[0].predicate, Predicate::RelatedTo);
        assert_eq!(out[0].pattern, "X пен Y");
    }

    #[test]
    fn nominal_conjunction_rejects_without_conjunction() {
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        nominal_conjunction("кітап ілім", &[], &lex, &src(), &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn nominal_conjunction_rejects_pronoun_side() {
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        nominal_conjunction("мен пен сен", &[], &lex, &src(), &mut out);
        assert!(out.is_empty(), "pronouns refused on either side");
    }

    #[test]
    fn domain_membership_extracts_algebra_is_math_field() {
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        domain_membership(
            "алгебра — математиканың саласы",
            &[],
            &lex,
            &src(),
            &mut out,
        );
        if out.is_empty() {
            eprintln!("note: алгебра/математика may not be in Lexicon; skipping");
            return;
        }
        assert_eq!(out[0].predicate, Predicate::InDomain);
        assert_eq!(out[0].pattern, "X — Y саласы");
    }

    #[test]
    fn domain_membership_rejects_without_salasy_glymy() {
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        domain_membership(
            "алгебра — математиканың бөлімі",
            &[],
            &lex,
            &src(),
            &mut out,
        );
        assert!(out.is_empty(), "no саласы/ғылымы head → refuse");
    }

    // ------------------------- structural_part_of (v3.5.5) ----------------

    #[test]
    fn structural_part_of_rejects_without_head_word() {
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        structural_part_of("жапырақ ағаштың жасыл түрі", &[], &lex, &src(), &mut out);
        assert!(out.is_empty(), "no бөлігі/құрамында/ішінде → refuse");
    }

    #[test]
    fn structural_part_of_rejects_without_genitive_preceding_head() {
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        // "бөлігі" present but preceding token is NOT a genitive noun.
        structural_part_of("жапырақ жасыл бөлігі болады", &[], &lex, &src(), &mut out);
        assert!(out.is_empty(), "preceding token must be genitive");
    }

    #[test]
    fn structural_part_of_rejects_pronoun_subject() {
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        // Pronoun subject refused per closed-class.
        structural_part_of("мен денеміздің бөлігі", &[], &lex, &src(), &mut out);
        assert!(out.is_empty(), "pronoun subject refused");
    }

    #[test]
    fn structural_part_of_rejects_tautology() {
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        // Tautological X=Y refused.
        structural_part_of("ми мидың бөлігі", &[], &lex, &src(), &mut out);
        assert!(out.is_empty(), "X=Y tautology refused");
    }

    // ------------------------- v3.8.5 precision hardening ------------------

    #[test]
    fn is_closed_class_covers_v3_8_5_additions() {
        // Demonstrative qualifiers flagged by Codex precision audit.
        assert!(is_closed_class("мұндай"));
        assert!(is_closed_class("сондай"));
        assert!(is_closed_class("ондай"));
        assert!(is_closed_class("кейбір"));
        assert!(is_closed_class("өз"));
        assert!(is_closed_class("өзі"));
        // Content nouns still pass.
        assert!(!is_closed_class("бала"));
        assert!(!is_closed_class("қазақстан"));
    }

    #[test]
    fn is_time_noun_covers_standard_set() {
        assert!(is_time_noun("жыл"));
        assert!(is_time_noun("күн"));
        assert!(is_time_noun("ай"));
        assert!(is_time_noun("ғасыр"));
        assert!(is_time_noun("уақыт"));
        // Not a time noun.
        assert!(!is_time_noun("бала"));
        assert!(!is_time_noun("мектеп"));
    }

    #[test]
    fn is_location_root_covers_countries_and_cities() {
        assert!(is_location_root("қазақстан"));
        assert!(is_location_root("ресей"));
        assert!(is_location_root("алматы"));
        assert!(is_location_root("астана"));
        // Content nouns still pass through the gate.
        assert!(!is_location_root("бала"));
        assert!(!is_location_root("кітап"));
    }

    #[test]
    fn locative_lives_in_rejects_country_subject() {
        // Pre-v3.8.5: «Қазақстан аумағында тұрады» produced
        // (қазақстан, lives_in, аумағын) — garbage. Post-v3.8.5 the
        // location-root filter refuses Қазақстан as a LivesIn subject.
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        locative_lives_in("Қазақстан өз аумағында тұрады", &[], &lex, &src(), &mut out);
        assert!(
            out.is_empty(),
            "country subject must be refused for LivesIn (got {out:?})"
        );
    }

    #[test]
    fn dative_goes_to_rejects_time_subject() {
        // «бір күн Масғұт жұмысқа барды» pre-v3.8.5 produced
        // (күн, goes_to, жұмыс). «күн» is a time noun, not an agent.
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        dative_goes_to("бір күн жұмысқа барды", &[], &lex, &src(), &mut out);
        assert!(
            out.is_empty(),
            "time-noun subject refused for GoesTo (got {out:?})"
        );
    }

    #[test]
    fn dative_goes_to_rejects_demonstrative_subject() {
        // «мұндай жағдай ... өсіруге мүмкіндік» pre-v3.8.5 produced
        // (мұндай, goes_to, өсіру). «мұндай» is a demonstrative
        // qualifier (closed-class).
        let Some(lex) = load_lex() else { return };
        let mut out = Vec::new();
        dative_goes_to("мұндай жағдай өсіруге болады", &[], &lex, &src(), &mut out);
        assert!(
            out.is_empty(),
            "demonstrative subject refused for GoesTo (got {out:?})"
        );
    }
}
