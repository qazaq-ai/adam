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

/// Optional preamble framing the answer. Mirrors the existing
/// template family `unknown.with_grounded_fact` which has three
/// introducer variants («X туралы ең әуелі мынаны айтуға болады»,
/// «X жайында негізгі дерек мынау», none / direct).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Introducer {
    /// «{topic} туралы ең әуелі мынаны айтуға болады: {body}»
    AboutTopicFirst,
    /// «{topic} жайында негізгі дерек мынау: {body}»
    OnTheSubjectMain,
    /// «{topic} туралы қысқаша айтсам: {body}»
    BrieflyAbout,
    /// Direct: just the body, no preamble.
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
fn all_rules() -> [&'static dyn NlgRule; 10] {
    [
        // Curated-raw-text rules first (special cases).
        &rules::HasQuantityDeclarative,
        &rules::RelatedToShectesDeclarative,
        &rules::RelatedToListDeclarative,
        // Composed rules (typed-primitives → surface).
        &rules::IsACopulaDeclarative,
        &rules::PartOfDeclarative,
        &rules::LivesInDeclarative,
        &rules::HasDeclarative,
        &rules::CausesDeclarative,
        &rules::InDomainDeclarative,
        // General RelatedTo last (catches everything not caught
        // by шектес / list-summary specialisations above).
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

/// Capitalise the first character (Kazakh-aware — uppercase
/// preserves Cyrillic semantics).
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
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
        };
        let out = render_sentence(&frame).expect("rule should match");
        assert_eq!(
            out,
            "Қазақстан туралы ең әуелі мынаны айтуға болады: Қазақстан — мемлекет."
        );
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
        };
        let out = render_sentence(&frame).expect("rule should match");
        // List-summary facts surface as their raw_text — preserves
        // the curated comma-separated list.
        assert_eq!(out, "Қазақстанның 17 облысы: Абай, Ақмола, ...");
    }

    #[test]
    fn unknown_predicate_returns_none() {
        // **v4.42.5** — predicates without an NLG rule fall through.
        // Causes was added in this bundle, so use one that's still
        // unmatched (GoesTo / After / DoesTo).
        let fact = make_fact("адам", ReasPredicate::GoesTo, "үй", "Адам үйге барады.");
        let frame = SentenceFrame {
            fact: &fact,
            mood: SentenceMood::Declarative,
            introducer: Introducer::Direct,
        };
        assert_eq!(render_sentence(&frame), None);
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
        };
        let out = render_sentence(&frame).expect("InDomain rule should match");
        assert_eq!(out, "Атом физика саласына жатады.");
    }
}
