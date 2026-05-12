//! **v4.42.0** — Stage A bundle 1: rule-based natural-language
//! generation (NLG) over typed semantic frames.
//!
//! ## What this is
//!
//! A new module that takes a [`SentenceFrame`] (typed propositional
//! content + sentence-shape operators) and emits a Kazakh surface
//! sentence via a small set of hand-written rules. Each rule is
//! matched on `(predicate × mood)` and renders a canonical surface
//! shape, with morphology delegated to the existing
//! [`adam_kernel_fst`] FST.
//!
//! ## Why a new module
//!
//! Pre-v4.42.0 sentence-level generation was via TOML templates —
//! pre-written strings with `{slot}` placeholders. Coverage was
//! therefore limited to whatever templates a human wrote in
//! `data/dialog/templates/v1.toml`. When the planner picked a
//! template family, the seed-mod selected one variant, and the
//! realiser substituted slots. There was no notion of *generating*
//! a sentence from a propositional frame — only *picking* and
//! filling.
//!
//! Per the project thesis (`project_retrieval_not_neural_v2.md`,
//! refined 2026-05-03): adam aims to be a NEW class of generative
//! AI that uses the agglutinative paradigm — typed primitives +
//! rule-based composition + tiny selection weights — to be
//! safe / cheap / predictable while reaching LLM-comparable
//! abilities. Templates are the rough draft of this; rule-based
//! NLG over typed frames is the proper architecture.
//!
//! ## Layered stack
//!
//! ```text
//! 1. FST morphology         (already)
//! 2. Typed SemFrame IR      (already, partial)
//! 3. world_core knowledge   (already)
//! 4. Reasoner               (already)
//! 5. Rule-based sentence NLG ← THIS MODULE
//! 6. Selection weights      (Stage B, future)
//! 7. Realiser (FST forward) (already)
//! ```
//!
//! ## Status (v4.42.0 — bundle 1 of Stage A)
//!
//! Foundation only:
//! - `SentenceFrame` typed wrapper around `ReasFact` + sentence-
//!   shape operators (mood, introducer).
//! - `NlgRule` trait — pluggable rule shape.
//! - 5 starter rules covering the most common factoid response
//!   patterns: `IsA-Declarative` / `PartOf-Declarative` /
//!   `HasQuantity-Declarative` / `RelatedToList-Declarative` /
//!   `LivesIn-Declarative`.
//! - `render_sentence` public entry point — first matching rule
//!   wins.
//!
//! NOT YET in v4.42.0:
//! - Selection weights (Stage B).
//! - Interrogative / Imperative moods (only Declarative covered).
//! - Replacement of templates in the dialog pipeline (rule-based
//!   NLG runs as a parallel CHECK, not a replacement, in v4.42.0).
//! - Verb-frame rules (only nominal-predicate facts covered).

use adam_reasoning::Fact as ReasFact;

/// Sentence-shape operators carried alongside the propositional
/// content. Mirrors the agglutinative pattern: a "root" (the fact)
/// + ordered "operators" (mood / tense / register / introducer)
/// that compose into the surface form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SentenceMood {
    /// Plain assertion: «X — Y», «X-та Y бар». Default for factoid
    /// answers.
    Declarative,
    /// Question form. Reserved for v4.42.5+; not used by the
    /// v4.42.0 starter rules.
    Interrogative,
    /// Command. Reserved for v4.42.5+.
    Imperative,
}

/// Optional preamble framing the answer. **v4.43.0** — Stage A
/// bundle 3 completed the introducer migration: the existing
/// template family `unknown.with_grounded_fact` had 5 phrasings;
/// all 5 are now first-class `Introducer` enum variants. The
/// templates simplify to a single `{fact}` slot whose value is a
/// full preamble+body sentence produced by [`compose_introducer`].
///
/// Order of variants matches template-array order (idx 0..=4) in
/// `data/dialog/templates/v1.toml::unknown.with_grounded_fact`
/// so that [`pick_introducer`] reproduces the v4.42.x seed-mod
/// rotation byte-for-byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Introducer {
    /// «{topic} туралы қысқаша айтсам: {body}» (template idx 0)
    BrieflyAbout,
    /// «{topic} жайында негізгі дерек мынау: {body}» (template idx 1)
    OnTheSubjectMain,
    /// «{topic} туралы ең әуелі мынаны айтуға болады: {body}»
    /// (template idx 2)
    AboutTopicFirst,
    /// «{name_respect}, {topic} туралы қысқа жауап: {body}»
    /// (template idx 3) — added in v4.43.0. Requires
    /// [`SentenceFrame::name_respect`] to be `Some`; otherwise
    /// the variant is filtered out by [`pick_introducer`].
    NameRespectAnswer,
    /// «{topic} жайында нақты дерек: {body}» (template idx 4) —
    /// added in v4.43.0.
    ExactFact,
    /// Direct: just the body, no preamble. Used by internal
    /// evidence rendering; not in the user-facing pick pool.
    Direct,
}

/// Typed propositional content + sentence-shape operators.
///
/// The "fact" is the agglutinative root — what is said. The
/// other fields are the operators applied to it: mood, introducer,
/// the user's topic anchor for capitalisation / preamble.
pub struct SentenceFrame<'a> {
    /// The propositional content (subject / predicate / object).
    pub fact: &'a ReasFact,
    /// Mood operator. v4.42.0 supports only `Declarative`.
    pub mood: SentenceMood,
    /// Optional preamble framing.
    pub introducer: Introducer,
    /// **v4.43.0** — respectful name form for
    /// [`Introducer::NameRespectAnswer`]. `None` for all other
    /// introducer variants (and for callers that don't have a
    /// session name). When `Some`, expected to already be in the
    /// respectful surface form (e.g. «Дәулет мырза»).
    pub name_respect: Option<&'a str>,
}

/// A rule that knows how to render some `(predicate × mood)`
/// combinations. Each rule has its own `matches` and `render`
/// — match is purely structural (no surface-text inspection),
/// render delegates morphology to the FST.
pub trait NlgRule: Sync {
    /// Does this rule apply to the given frame?
    fn matches(&self, frame: &SentenceFrame) -> bool;
    /// Produce the surface sentence. `None` if the rule applies
    /// but cannot render this specific frame (e.g. missing field).
    fn render(&self, frame: &SentenceFrame) -> Option<String>;
    /// Identifier for traces and tests.
    fn name(&self) -> &'static str;
}

mod rules;

/// All rules in priority order. First matching rule wins.
///
/// Order rationale:
/// - Special-case RelatedTo rules (`шектес`, list-summary) run
///   BEFORE the general RelatedTo rule so curated raw_text wins
///   over mechanical composition where the curated text is richer.
/// - HasQuantity uses raw_text and runs early — count phrasings
///   («Қазақстанда 17 облыс бар») are richer than mechanical
///   templating could produce.
/// - The remaining declarative rules (IsA / PartOf / LivesIn /
///   Has / Causes / InDomain) compose from typed primitives in
///   order matching the existing `tool::render_grounded_fact`
///   behavior preserved bit-for-bit at the v4.42.5 NLG migration
///   point.
fn all_rules() -> [&'static dyn NlgRule; 14] {
    [
        // Curated-raw-text rules first (special cases).
        &rules::HasQuantityDeclarative,
        &rules::RelatedToShectesDeclarative,
        &rules::RelatedToListDeclarative,
        // **v4.78.0** — office→person direct rendering (Codex Bug 1).
        // Must run BEFORE the generic RelatedTo «өзара байланысты»
        // rule below so «Қазақстан Президенті — Тоқаев» wins over
        // «X мен Y өзара байланысты» for office-holder facts.
        &rules::RelatedToOfficeHolderDeclarative,
        // Composed rules (typed-primitives → surface).
        &rules::IsACopulaDeclarative,
        &rules::PartOfDeclarative,
        &rules::LivesInDeclarative,
        &rules::HasDeclarative,
        &rules::CausesDeclarative,
        &rules::InDomainDeclarative,
        // **v4.43.5** — added GoesTo + After (raw_text-prefer);
        // **v4.43.6** — added DoesTo, closing Stage A declarative
        // coverage to **11/11** reasoner-emitted predicates.
        &rules::GoesToDeclarative,
        &rules::AfterDeclarative,
        &rules::DoesToDeclarative,
        // General RelatedTo last (catches everything not caught
        // by шектес / list-summary / office-holder specialisations
        // above).
        &rules::RelatedToOzaraDeclarative,
    ]
}

/// Public entry point. Iterates rules in priority order; the first
/// rule whose `matches` returns true tries to `render`. If render
/// returns `None`, fall through to the next matching rule. If no
/// rule matches at all, return `None` — the caller should fall back
/// to the existing template-based realiser.
pub fn render_sentence(frame: &SentenceFrame) -> Option<String> {
    for rule in all_rules() {
        if rule.matches(frame)
            && let Some(text) = rule.render(frame)
        {
            return Some(text);
        }
    }
    None
}

/// **v4.43.0** — Stage A bundle 3. Deterministic introducer pick
/// for the migrated `unknown.with_grounded_fact` family.
///
/// Mirrors the v4.42.x template-rotation algorithm bit-for-bit:
/// the candidate pool has 5 introducers when the session has a
/// respectful-form name (matching the 5 fillable templates), and
/// 4 when it doesn't (matching the post-fillability-filter pool
/// when the `{name_respect}` template was filtered out). Order
/// inside the pool matches the original template-array order,
/// so `seed % pool.len()` produces the same surface text after
/// the migration as before.
pub fn pick_introducer(seed: u64, has_name_respect: bool) -> Introducer {
    let pool: &[Introducer] = if has_name_respect {
        &[
            Introducer::BrieflyAbout,      // template idx 0
            Introducer::OnTheSubjectMain,  // template idx 1
            Introducer::AboutTopicFirst,   // template idx 2
            Introducer::NameRespectAnswer, // template idx 3
            Introducer::ExactFact,         // template idx 4
        ]
    } else {
        &[
            Introducer::BrieflyAbout,     // template idx 0
            Introducer::OnTheSubjectMain, // template idx 1
            Introducer::AboutTopicFirst,  // template idx 2
            Introducer::ExactFact,        // template idx 4 (filtered: idx 3 needs name_respect)
        ]
    };
    pool[(seed as usize) % pool.len()]
}

/// **v4.43.0** — Stage A bundle 3. Public composer that wraps a
/// body sentence in the chosen introducer's preamble, threading
/// the topic anchor and (for `NameRespectAnswer`) the respectful
/// name form. Used by the planner to fill the `{fact}` slot of
/// the simplified `unknown.with_grounded_fact` template family.
///
/// For `Direct`, returns the body unchanged. For
/// `NameRespectAnswer` with `name_respect == None`, falls back to
/// `BrieflyAbout` (the variant should have been filtered out by
/// `pick_introducer` in normal flow; this fallback prevents a
/// crash if a caller composes manually without checking).
pub fn compose_introducer(
    introducer: Introducer,
    _topic: &str,
    name_respect: Option<&str>,
    body: &str,
) -> String {
    match introducer {
        // **v5.23.0** — anti-meta-opener pass. The body already
        // mentions the topic; repeating «X туралы …» / «X жайында …»
        // at the start sounds like a Wikipedia citation, not
        // conversation. 3 of 5 variants now emit the body directly;
        // 1 keeps a light «Қысқаша айтсам,» frame; 1 personalises
        // with the respectful name when available. Live feedback
        // 2026-05-12: «Білгенім бойынша: X туралы ең әуелі мынаны
        // айтуға болады: …» (three-layer stacking) flagged as
        // «шаблонно» — student wants direct answers.
        Introducer::Direct => body.to_string(),
        Introducer::BrieflyAbout => body.to_string(),
        Introducer::OnTheSubjectMain => body.to_string(),
        Introducer::AboutTopicFirst => format!("Қысқаша айтсам, {body}"),
        Introducer::NameRespectAnswer => match name_respect {
            Some(name) => format!("{name}, {body}"),
            None => body.to_string(),
        },
        Introducer::ExactFact => body.to_string(),
    }
}

/// Capitalise the first character (Kazakh-aware — uppercase
/// preserves Cyrillic semantics).
pub(crate) fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// **v5.23.0** — capitalise every token (space- and hyphen-separated)
/// of a proper name. world_core stores canonical-lowercase surfaces
/// («қасым-жомарт тоқаев») for lookup uniformity; render-time
/// presentation needs «Қасым-Жомарт Тоқаев».
///
/// Live feedback 2026-05-12: «Қазақстанның президенті — қасым-жомарт
/// тоқаев» — student noted И.Ф.О. wrote lowercase. This helper fixes
/// the rendering without touching storage.
pub(crate) fn capitalize_proper_name(s: &str) -> String {
    s.split_inclusive([' ', '-'])
        .map(|chunk| {
            // Each chunk ends with its trailing separator (or is the
            // last token without one). Capitalise everything up to the
            // separator; pass the separator through unchanged.
            let (word, sep): (&str, &str) = match chunk.chars().last() {
                Some(c) if c == ' ' || c == '-' => {
                    let split_at = chunk.len() - c.len_utf8();
                    (&chunk[..split_at], &chunk[split_at..])
                }
                _ => (chunk, ""),
            };
            format!("{}{}", capitalize_first(word), sep)
        })
        .collect()
}

/// Pick the best surface form from a SlotRef — prefer the curated
/// surface, fall back to the canonical root when surface is empty.
/// Mirrors the pre-v4.42.5 `tool::preferred_slot_text` behavior.
fn preferred_surface(slot: &adam_reasoning::SlotRef) -> &str {
    if slot.surface.trim().is_empty() {
        slot.root.trim()
    } else {
        slot.surface.trim()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adam_reasoning::{ConfidenceKind, FactSource, Predicate as ReasPredicate, SlotRef};

    fn make_fact(subject: &str, predicate: ReasPredicate, object: &str, raw: &str) -> ReasFact {
        ReasFact {
            subject: SlotRef {
                surface: subject.to_string(),
                root: subject.to_string(),
                pos: "noun".to_string(),
            },
            predicate,
            object: SlotRef {
                surface: object.to_string(),
                root: object.to_string(),
                pos: "noun".to_string(),
            },
            pattern: "test".to_string(),
            source: FactSource {
                pack: "test".to_string(),
                sample_id: "test".to_string(),
            },
            confidence: ConfidenceKind::HumanApproved,
            raw_text: raw.to_string(),
        }
    }

    #[test]
    fn isa_declarative_direct() {
        let fact = make_fact(
            "қазақстан",
            ReasPredicate::IsA,
            "мемлекет",
            "Қазақстан — мемлекет.",
        );
        let frame = SentenceFrame {
            fact: &fact,
            mood: SentenceMood::Declarative,
            introducer: Introducer::Direct,
            name_respect: None,
        };
        let out = render_sentence(&frame).expect("rule should match");
        assert_eq!(out, "Қазақстан — мемлекет.");
    }

    #[test]
    fn isa_declarative_about_topic() {
        let fact = make_fact(
            "қазақстан",
            ReasPredicate::IsA,
            "мемлекет",
            "Қазақстан — мемлекет.",
        );
        let frame = SentenceFrame {
            fact: &fact,
            mood: SentenceMood::Declarative,
            introducer: Introducer::AboutTopicFirst,
            name_respect: None,
        };
        let out = render_sentence(&frame).expect("rule should match");
        // v5.23.0 — AboutTopicFirst now emits a light «Қысқаша
        // айтсам, …» frame; the older «X туралы ең әуелі мынаны
        // айтуға болады:» phrasing dropped per anti-meta-opener pass.
        assert_eq!(out, "Қысқаша айтсам, Қазақстан — мемлекет.");
    }

    #[test]
    fn part_of_declarative() {
        let fact = make_fact(
            "астана",
            ReasPredicate::PartOf,
            "қазақстан",
            "Астана Қазақстан құрамына кіреді.",
        );
        let frame = SentenceFrame {
            fact: &fact,
            mood: SentenceMood::Declarative,
            introducer: Introducer::Direct,
            name_respect: None,
        };
        let out = render_sentence(&frame).expect("rule should match");
        assert_eq!(out, "Астана қазақстан құрамына кіреді.");
    }

    #[test]
    fn has_quantity_declarative_uses_raw_text() {
        // HasQuantity surface forms are quantity-bearing and the
        // raw_text already encodes the count phrase. The starter
        // rule reuses raw_text rather than re-rendering — closer to
        // the existing v4.38.0 behavior.
        let fact = make_fact(
            "қазақстан",
            ReasPredicate::HasQuantity,
            "облыс",
            "Қазақстанда 17 облыс бар.",
        );
        let frame = SentenceFrame {
            fact: &fact,
            mood: SentenceMood::Declarative,
            introducer: Introducer::Direct,
            name_respect: None,
        };
        let out = render_sentence(&frame).expect("rule should match");
        assert_eq!(out, "Қазақстанда 17 облыс бар.");
    }

    #[test]
    fn lives_in_declarative() {
        let fact = make_fact(
            "абай",
            ReasPredicate::LivesIn,
            "семей",
            "Абай Семейде өмір сүрген.",
        );
        let frame = SentenceFrame {
            fact: &fact,
            mood: SentenceMood::Declarative,
            introducer: Introducer::Direct,
            name_respect: None,
        };
        let out = render_sentence(&frame).expect("rule should match");
        assert_eq!(out, "Абай мекені — семей.");
    }

    #[test]
    fn related_to_list_uses_raw_text() {
        let fact = make_fact(
            "қазақстан",
            ReasPredicate::RelatedTo,
            "облыстар тізімі",
            "Қазақстанның 17 облысы: Абай, Ақмола, ...",
        );
        let frame = SentenceFrame {
            fact: &fact,
            mood: SentenceMood::Declarative,
            introducer: Introducer::Direct,
            name_respect: None,
        };
        let out = render_sentence(&frame).expect("rule should match");
        // List-summary facts surface as their raw_text — preserves
        // the curated comma-separated list.
        assert_eq!(out, "Қазақстанның 17 облысы: Абай, Ақмола, ...");
    }

    #[test]
    fn all_eleven_predicates_have_a_rule() {
        // **v4.43.6** — Stage A declarative NLG covers all 11
        // reasoner-emitted predicates (full coverage). This test
        // pins the contract so a future predicate addition without
        // a corresponding rule trips a red CI.
        let predicates = [
            ReasPredicate::IsA,
            ReasPredicate::HasQuantity,
            ReasPredicate::RelatedTo,
            ReasPredicate::PartOf,
            ReasPredicate::LivesIn,
            ReasPredicate::Has,
            ReasPredicate::Causes,
            ReasPredicate::InDomain,
            ReasPredicate::GoesTo,
            ReasPredicate::After,
            ReasPredicate::DoesTo,
        ];
        for p in predicates {
            let fact = make_fact("адам", p, "зат", "Адам затпен әрекет етеді.");
            let frame = SentenceFrame {
                fact: &fact,
                mood: SentenceMood::Declarative,
                introducer: Introducer::Direct,
                name_respect: None,
            };
            assert!(
                render_sentence(&frame).is_some(),
                "predicate {p:?} has no NLG rule",
            );
        }
    }

    #[test]
    fn does_to_declarative_uses_raw_text_when_present() {
        let fact = make_fact("адам", ReasPredicate::DoesTo, "үй", "Адам үйді тазалайды.");
        let frame = SentenceFrame {
            fact: &fact,
            mood: SentenceMood::Declarative,
            introducer: Introducer::Direct,
            name_respect: None,
        };
        let out = render_sentence(&frame).expect("DoesTo rule should match");
        assert_eq!(out, "Адам үйді тазалайды.");
    }

    #[test]
    fn does_to_declarative_falls_back_to_dative_when_raw_empty() {
        let fact = make_fact("жаңбыр", ReasPredicate::DoesTo, "жер", "");
        let frame = SentenceFrame {
            fact: &fact,
            mood: SentenceMood::Declarative,
            introducer: Introducer::Direct,
            name_respect: None,
        };
        let out = render_sentence(&frame).expect("DoesTo rule should match");
        assert_eq!(out, "Жаңбыр жер-ге әсер етеді.");
    }

    #[test]
    fn causes_declarative() {
        let fact = make_fact(
            "жаңбыр",
            ReasPredicate::Causes,
            "сел",
            "Жаңбыр сел себебі болады.",
        );
        let frame = SentenceFrame {
            fact: &fact,
            mood: SentenceMood::Declarative,
            introducer: Introducer::Direct,
            name_respect: None,
        };
        let out = render_sentence(&frame).expect("Causes rule should match");
        assert_eq!(out, "Жаңбыр сел себебі болады.");
    }

    #[test]
    fn has_declarative() {
        let fact = make_fact("ел", ReasPredicate::Has, "тіл", "Елдің тілі бар.");
        let frame = SentenceFrame {
            fact: &fact,
            mood: SentenceMood::Declarative,
            introducer: Introducer::Direct,
            name_respect: None,
        };
        let out = render_sentence(&frame).expect("Has rule should match");
        assert_eq!(out, "Ел тіл иеленеді.");
    }

    #[test]
    fn isa_with_richer_raw_text_uses_raw() {
        // Mirrors the existing tool.rs test
        // `grounded_fact_keeps_richer_raw_text_for_is_a` — IsA rule
        // prefers curated raw_text over mechanical "subj — obj"
        // composition.
        let fact = make_fact(
            "қазақстан",
            ReasPredicate::IsA,
            "ел",
            "Қазақстан — Орталық Азиядағы ел",
        );
        let frame = SentenceFrame {
            fact: &fact,
            mood: SentenceMood::Declarative,
            introducer: Introducer::Direct,
            name_respect: None,
        };
        let out = render_sentence(&frame).expect("rule should match");
        assert_eq!(out, "Қазақстан — Орталық Азиядағы ел.");
    }

    #[test]
    fn related_to_shectes_uses_raw() {
        // Border-relation special case — raw_text wins over the
        // generic «X мен Y өзара байланысты» phrasing.
        let fact = make_fact(
            "қазақстан",
            ReasPredicate::RelatedTo,
            "ресей",
            "Қазақстан Ресеймен шектес.",
        );
        let frame = SentenceFrame {
            fact: &fact,
            mood: SentenceMood::Declarative,
            introducer: Introducer::Direct,
            name_respect: None,
        };
        let out = render_sentence(&frame).expect("шектес rule should match");
        assert_eq!(out, "Қазақстан Ресеймен шектес.");
    }

    #[test]
    fn related_to_general_uses_ozara_template() {
        let fact = make_fact("кітап", ReasPredicate::RelatedTo, "ілім", "");
        let frame = SentenceFrame {
            fact: &fact,
            mood: SentenceMood::Declarative,
            introducer: Introducer::Direct,
            name_respect: None,
        };
        let out = render_sentence(&frame).expect("general RelatedTo should match");
        assert_eq!(out, "Кітап мен ілім өзара байланысты.");
    }

    #[test]
    fn in_domain_declarative() {
        let fact = make_fact("атом", ReasPredicate::InDomain, "физика", "");
        let frame = SentenceFrame {
            fact: &fact,
            mood: SentenceMood::Declarative,
            introducer: Introducer::Direct,
            name_respect: None,
        };
        let out = render_sentence(&frame).expect("InDomain rule should match");
        assert_eq!(out, "Атом физика саласына жатады.");
    }

    // ========================================================================
    // **v4.43.0** — Stage A bundle 3: introducer migration tests.
    // ========================================================================

    #[test]
    fn compose_introducer_briefly_about() {
        let out = compose_introducer(
            Introducer::BrieflyAbout,
            "алгоритм",
            None,
            "Алгоритм — ереже.",
        );
        // v5.23.0 — bare body (the «X туралы қысқаша айтсам:» frame
        // dropped per live-feedback anti-meta-opener pass).
        assert_eq!(out, "Алгоритм — ереже.");
    }

    #[test]
    fn compose_introducer_on_the_subject_main() {
        let out = compose_introducer(
            Introducer::OnTheSubjectMain,
            "алгоритм",
            None,
            "Алгоритм — ереже.",
        );
        // v5.23.0 — bare body (the «X жайында негізгі дерек мынау:»
        // frame dropped per anti-meta-opener pass).
        assert_eq!(out, "Алгоритм — ереже.");
    }

    #[test]
    fn compose_introducer_about_topic_first() {
        let out = compose_introducer(
            Introducer::AboutTopicFirst,
            "алгоритм",
            None,
            "Алгоритм — ереже.",
        );
        // v5.23.0 — light «Қысқаша айтсам,» frame; the older «X
        // туралы ең әуелі мынаны айтуға болады:» phrasing dropped.
        assert_eq!(out, "Қысқаша айтсам, Алгоритм — ереже.");
    }

    #[test]
    fn compose_introducer_name_respect_answer() {
        let out = compose_introducer(
            Introducer::NameRespectAnswer,
            "алгоритм",
            Some("Дәулет мырза"),
            "Алгоритм — ереже.",
        );
        // v5.23.0 — personalisation kept but the meta «X туралы
        // қысқа жауап:» frame dropped.
        assert_eq!(out, "Дәулет мырза, Алгоритм — ереже.");
    }

    #[test]
    fn compose_introducer_exact_fact() {
        let out = compose_introducer(Introducer::ExactFact, "алгоритм", None, "Алгоритм — ереже.");
        // v5.23.0 — bare body (the «X жайында нақты дерек:» frame
        // dropped per anti-meta-opener pass).
        assert_eq!(out, "Алгоритм — ереже.");
    }

    #[test]
    fn compose_introducer_direct_returns_body_unchanged() {
        let out = compose_introducer(Introducer::Direct, "алгоритм", None, "Алгоритм — ереже.");
        assert_eq!(out, "Алгоритм — ереже.");
    }

    // **v5.23.0** — proper-name capitalisation tests.

    #[test]
    fn capitalize_proper_name_handles_single_word() {
        assert_eq!(capitalize_proper_name("тоқаев"), "Тоқаев");
    }

    #[test]
    fn capitalize_proper_name_handles_two_words() {
        assert_eq!(
            capitalize_proper_name("нұрсұлтан назарбаев"),
            "Нұрсұлтан Назарбаев"
        );
    }

    #[test]
    fn capitalize_proper_name_handles_hyphenated_first_name() {
        assert_eq!(
            capitalize_proper_name("қасым-жомарт тоқаев"),
            "Қасым-Жомарт Тоқаев"
        );
    }

    #[test]
    fn capitalize_proper_name_preserves_already_capitalised() {
        assert_eq!(capitalize_proper_name("Тоқаев"), "Тоқаев");
    }

    #[test]
    fn capitalize_proper_name_empty_string() {
        assert_eq!(capitalize_proper_name(""), "");
    }

    #[test]
    fn compose_introducer_name_respect_answer_falls_back_when_name_missing() {
        // v5.23.0 — when name_respect is None, NameRespectAnswer
        // degrades to bare body (matches BrieflyAbout / Direct /
        // OnTheSubjectMain / ExactFact post anti-meta-opener pass).
        let out = compose_introducer(
            Introducer::NameRespectAnswer,
            "алгоритм",
            None,
            "Алгоритм — ереже.",
        );
        assert_eq!(out, "Алгоритм — ереже.");
    }

    #[test]
    fn pick_introducer_with_name_respect_rotates_5_variants() {
        // Pool order matches v4.42.x template-array order (idx 0..=4).
        assert_eq!(pick_introducer(0, true), Introducer::BrieflyAbout);
        assert_eq!(pick_introducer(1, true), Introducer::OnTheSubjectMain);
        assert_eq!(pick_introducer(2, true), Introducer::AboutTopicFirst);
        assert_eq!(pick_introducer(3, true), Introducer::NameRespectAnswer);
        assert_eq!(pick_introducer(4, true), Introducer::ExactFact);
        assert_eq!(pick_introducer(5, true), Introducer::BrieflyAbout); // wraps
    }

    #[test]
    fn pick_introducer_without_name_respect_rotates_4_variants() {
        // NameRespectAnswer filtered out; pool shrinks to 4.
        assert_eq!(pick_introducer(0, false), Introducer::BrieflyAbout);
        assert_eq!(pick_introducer(1, false), Introducer::OnTheSubjectMain);
        assert_eq!(pick_introducer(2, false), Introducer::AboutTopicFirst);
        assert_eq!(pick_introducer(3, false), Introducer::ExactFact);
        assert_eq!(pick_introducer(4, false), Introducer::BrieflyAbout); // wraps
    }

    // ========================================================================
    // **v4.43.5** — GoesTo + After NLG rule tests.
    // ========================================================================

    #[test]
    fn goes_to_declarative_uses_raw_text_when_present() {
        let fact = make_fact(
            "адам",
            ReasPredicate::GoesTo,
            "үй",
            "Адам үйге жолын жасайды.",
        );
        let frame = SentenceFrame {
            fact: &fact,
            mood: SentenceMood::Declarative,
            introducer: Introducer::Direct,
            name_respect: None,
        };
        let out = render_sentence(&frame).expect("GoesTo rule should match");
        assert_eq!(out, "Адам үйге жолын жасайды.");
    }

    #[test]
    fn goes_to_declarative_falls_back_to_dative_when_raw_empty() {
        let fact = make_fact("адам", ReasPredicate::GoesTo, "үй", "");
        let frame = SentenceFrame {
            fact: &fact,
            mood: SentenceMood::Declarative,
            introducer: Introducer::Direct,
            name_respect: None,
        };
        let out = render_sentence(&frame).expect("GoesTo rule should match");
        assert_eq!(out, "Адам үй-ге барады.");
    }

    #[test]
    fn after_declarative_uses_raw_text_when_present() {
        let fact = make_fact(
            "көктем",
            ReasPredicate::After,
            "жаз",
            "Көктемнен кейін жаз келеді.",
        );
        let frame = SentenceFrame {
            fact: &fact,
            mood: SentenceMood::Declarative,
            introducer: Introducer::Direct,
            name_respect: None,
        };
        let out = render_sentence(&frame).expect("After rule should match");
        assert_eq!(out, "Көктемнен кейін жаз келеді.");
    }

    #[test]
    fn after_declarative_falls_back_to_ablative_when_raw_empty() {
        let fact = make_fact("көктем", ReasPredicate::After, "жаз", "");
        let frame = SentenceFrame {
            fact: &fact,
            mood: SentenceMood::Declarative,
            introducer: Introducer::Direct,
            name_respect: None,
        };
        let out = render_sentence(&frame).expect("After rule should match");
        assert_eq!(out, "Көктем-тен кейін жаз болады.");
    }

    #[test]
    fn pick_introducer_is_deterministic() {
        // Same seed + same has_name_respect always picks the same variant.
        for seed in [0u64, 7, 42, 1024, u64::MAX] {
            assert_eq!(pick_introducer(seed, true), pick_introducer(seed, true));
            assert_eq!(pick_introducer(seed, false), pick_introducer(seed, false));
        }
    }
}
