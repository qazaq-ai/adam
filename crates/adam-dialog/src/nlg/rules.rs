//! **v4.42.0** — Stage A bundle 1 starter NLG rules.
//!
//! Each rule corresponds to a `(predicate × mood)` pair. Rules are
//! declarative-only in v4.42.0; interrogative / imperative come
//! later. Order in `super::all_rules` reflects priority.
//!
//! Each rule's `render` is a small composer over the FST-rendered
//! surface forms of subject and object. For predicates that already
//! curate the canonical surface phrasing in `raw_text`
//! (HasQuantity, RelatedTo-list), the rule reuses `raw_text` to
//! preserve the human-curated phrasing. For predicates with a
//! clean compositional shape (IsA, PartOf, LivesIn), the rule
//! generates from the typed primitives via a fixed template
//! skeleton.

use super::{
    Introducer, NlgRule, SentenceFrame, SentenceMood, capitalize_first, compose_introducer,
    preferred_surface,
};
use adam_reasoning::Predicate as ReasPredicate;

/// Wrap `body` in the introducer's preamble. **v4.43.0** —
/// delegates to the public [`super::compose_introducer`] so the
/// rules path and the planner path produce byte-identical output
/// for any given `(introducer, topic, name_respect, body)`.
fn wrap_introducer(introducer: Introducer, topic: &str, body: &str) -> String {
    compose_introducer(introducer, topic, None, body)
}

/// Ensure the body ends with a period.
fn ensure_period(s: String) -> String {
    if s.ends_with('.') || s.ends_with('!') || s.ends_with('?') {
        s
    } else {
        format!("{s}.")
    }
}

// ---------------------------------------------------------------------------

/// IsA copula declarative: «X — Y», or curated raw_text when richer.
///
/// Surfaces a definitional fact via the em-dash copula. Prefers
/// curated `raw_text` over mechanical composition — many world_core
/// IsA facts have rich descriptive raw_text («Қазақстан — Орталық
/// Азиядағы аумағы бойынша 9-шы үлкен тәуелсіз мемлекет; астанасы
/// — Астана, ірі қаласы — Алматы.») that mechanical "subject —
/// object" composition would lose. Mechanical composition fires
/// only when raw_text is empty.
pub struct IsACopulaDeclarative;

impl NlgRule for IsACopulaDeclarative {
    fn matches(&self, frame: &SentenceFrame) -> bool {
        matches!(frame.fact.predicate, ReasPredicate::IsA)
            && matches!(frame.mood, SentenceMood::Declarative)
    }

    fn render(&self, frame: &SentenceFrame) -> Option<String> {
        let raw = frame.fact.raw_text.trim();
        let body = if raw.is_empty() {
            // No curated text — compose from typed primitives.
            let subject_cap = capitalize_first(preferred_surface(&frame.fact.subject));
            ensure_period(format!(
                "{} — {}",
                subject_cap,
                preferred_surface(&frame.fact.object)
            ))
        } else {
            ensure_period(raw.to_string())
        };
        Some(wrap_introducer(
            frame.introducer,
            &frame.fact.subject.root,
            &body,
        ))
    }

    fn name(&self) -> &'static str {
        "IsACopulaDeclarative"
    }
}

// ---------------------------------------------------------------------------

/// PartOf declarative: «X Y құрамына кіреді».
pub struct PartOfDeclarative;

impl NlgRule for PartOfDeclarative {
    fn matches(&self, frame: &SentenceFrame) -> bool {
        matches!(frame.fact.predicate, ReasPredicate::PartOf)
            && matches!(frame.mood, SentenceMood::Declarative)
    }

    fn render(&self, frame: &SentenceFrame) -> Option<String> {
        let subject_cap = capitalize_first(preferred_surface(&frame.fact.subject));
        let body = ensure_period(format!(
            "{} {} құрамына кіреді",
            subject_cap,
            preferred_surface(&frame.fact.object)
        ));
        Some(wrap_introducer(
            frame.introducer,
            &frame.fact.subject.root,
            &body,
        ))
    }

    fn name(&self) -> &'static str {
        "PartOfDeclarative"
    }
}

// ---------------------------------------------------------------------------

/// HasQuantity declarative: reuses `raw_text` because the curated
/// phrase already encodes the count («Қазақстанда 17 облыс бар»)
/// with proper Kazakh number-word morphology that the typed
/// primitives don't carry.
pub struct HasQuantityDeclarative;

impl NlgRule for HasQuantityDeclarative {
    fn matches(&self, frame: &SentenceFrame) -> bool {
        matches!(frame.fact.predicate, ReasPredicate::HasQuantity)
            && matches!(frame.mood, SentenceMood::Declarative)
    }

    fn render(&self, frame: &SentenceFrame) -> Option<String> {
        let raw = frame.fact.raw_text.trim();
        if raw.is_empty() {
            return None;
        }
        let body = ensure_period(raw.to_string());
        Some(wrap_introducer(
            frame.introducer,
            &frame.fact.subject.root,
            &body,
        ))
    }

    fn name(&self) -> &'static str {
        "HasQuantityDeclarative"
    }
}

// ---------------------------------------------------------------------------

/// RelatedTo declarative for list-summary objects (object root
/// contains "тізім"): reuses `raw_text` because it carries the
/// curated comma-separated enumeration.
///
/// Non-list RelatedTo variants are NOT covered by this rule; they
/// fall through to `None` and the caller picks up via templates.
pub struct RelatedToListDeclarative;

impl NlgRule for RelatedToListDeclarative {
    fn matches(&self, frame: &SentenceFrame) -> bool {
        matches!(frame.fact.predicate, ReasPredicate::RelatedTo)
            && matches!(frame.mood, SentenceMood::Declarative)
            && frame.fact.object.root.to_lowercase().contains("тізім")
    }

    fn render(&self, frame: &SentenceFrame) -> Option<String> {
        let raw = frame.fact.raw_text.trim();
        if raw.is_empty() {
            return None;
        }
        let body = ensure_period(raw.to_string());
        Some(wrap_introducer(
            frame.introducer,
            &frame.fact.subject.root,
            &body,
        ))
    }

    fn name(&self) -> &'static str {
        "RelatedToListDeclarative"
    }
}

// ---------------------------------------------------------------------------

/// LivesIn declarative: «X мекені — Y».
pub struct LivesInDeclarative;

impl NlgRule for LivesInDeclarative {
    fn matches(&self, frame: &SentenceFrame) -> bool {
        matches!(frame.fact.predicate, ReasPredicate::LivesIn)
            && matches!(frame.mood, SentenceMood::Declarative)
    }

    fn render(&self, frame: &SentenceFrame) -> Option<String> {
        let subject_cap = capitalize_first(preferred_surface(&frame.fact.subject));
        let body = ensure_period(format!(
            "{} мекені — {}",
            subject_cap,
            preferred_surface(&frame.fact.object)
        ));
        Some(wrap_introducer(
            frame.introducer,
            &frame.fact.subject.root,
            &body,
        ))
    }

    fn name(&self) -> &'static str {
        "LivesInDeclarative"
    }
}

// ---------------------------------------------------------------------------

/// Has declarative: «X Y иеленеді».
///
/// **v4.42.5** — added to mirror existing `render_grounded_fact`
/// behavior in tool.rs so the NLG migration is byte-identical.
pub struct HasDeclarative;

impl NlgRule for HasDeclarative {
    fn matches(&self, frame: &SentenceFrame) -> bool {
        matches!(frame.fact.predicate, ReasPredicate::Has)
            && matches!(frame.mood, SentenceMood::Declarative)
    }

    fn render(&self, frame: &SentenceFrame) -> Option<String> {
        let subject_cap = capitalize_first(preferred_surface(&frame.fact.subject));
        let body = ensure_period(format!(
            "{} {} иеленеді",
            subject_cap,
            preferred_surface(&frame.fact.object)
        ));
        Some(wrap_introducer(
            frame.introducer,
            &frame.fact.subject.root,
            &body,
        ))
    }

    fn name(&self) -> &'static str {
        "HasDeclarative"
    }
}

// ---------------------------------------------------------------------------

/// Causes declarative: «X Y себебі болады».
///
/// **v4.42.5** — added.
pub struct CausesDeclarative;

impl NlgRule for CausesDeclarative {
    fn matches(&self, frame: &SentenceFrame) -> bool {
        matches!(frame.fact.predicate, ReasPredicate::Causes)
            && matches!(frame.mood, SentenceMood::Declarative)
    }

    fn render(&self, frame: &SentenceFrame) -> Option<String> {
        let subject_cap = capitalize_first(preferred_surface(&frame.fact.subject));
        let body = ensure_period(format!(
            "{} {} себебі болады",
            subject_cap,
            preferred_surface(&frame.fact.object)
        ));
        Some(wrap_introducer(
            frame.introducer,
            &frame.fact.subject.root,
            &body,
        ))
    }

    fn name(&self) -> &'static str {
        "CausesDeclarative"
    }
}

// ---------------------------------------------------------------------------

/// InDomain declarative: «X Y саласына жатады».
///
/// **v4.42.5** — added.
pub struct InDomainDeclarative;

impl NlgRule for InDomainDeclarative {
    fn matches(&self, frame: &SentenceFrame) -> bool {
        matches!(frame.fact.predicate, ReasPredicate::InDomain)
            && matches!(frame.mood, SentenceMood::Declarative)
    }

    fn render(&self, frame: &SentenceFrame) -> Option<String> {
        let subject_cap = capitalize_first(preferred_surface(&frame.fact.subject));
        let body = ensure_period(format!(
            "{} {} саласына жатады",
            subject_cap,
            preferred_surface(&frame.fact.object)
        ));
        Some(wrap_introducer(
            frame.introducer,
            &frame.fact.subject.root,
            &body,
        ))
    }

    fn name(&self) -> &'static str {
        "InDomainDeclarative"
    }
}

// ---------------------------------------------------------------------------

/// RelatedTo border-fact declarative: when raw_text contains
/// «шектес» (Kazakh "borders / adjacent to"), prefer raw_text
/// — these are curated geographic-border statements that lose
/// information through mechanical composition.
///
/// Must run BEFORE [`RelatedToOzaraDeclarative`] so the special
/// case wins.
pub struct RelatedToShectesDeclarative;

impl NlgRule for RelatedToShectesDeclarative {
    fn matches(&self, frame: &SentenceFrame) -> bool {
        matches!(frame.fact.predicate, ReasPredicate::RelatedTo)
            && matches!(frame.mood, SentenceMood::Declarative)
            && frame.fact.raw_text.contains("шектес")
    }

    fn render(&self, frame: &SentenceFrame) -> Option<String> {
        let raw = frame.fact.raw_text.trim();
        if raw.is_empty() {
            return None;
        }
        let body = ensure_period(raw.to_string());
        Some(wrap_introducer(
            frame.introducer,
            &frame.fact.subject.root,
            &body,
        ))
    }

    fn name(&self) -> &'static str {
        "RelatedToShectesDeclarative"
    }
}

// ---------------------------------------------------------------------------

/// RelatedTo general declarative: «X мен Y өзара байланысты».
///
/// Default RelatedTo phrasing. Runs AFTER list-summary and шектес
/// special cases via priority order in `super::all_rules`.
pub struct RelatedToOzaraDeclarative;

impl NlgRule for RelatedToOzaraDeclarative {
    fn matches(&self, frame: &SentenceFrame) -> bool {
        matches!(frame.fact.predicate, ReasPredicate::RelatedTo)
            && matches!(frame.mood, SentenceMood::Declarative)
    }

    fn render(&self, frame: &SentenceFrame) -> Option<String> {
        let subject_cap = capitalize_first(preferred_surface(&frame.fact.subject));
        let body = ensure_period(format!(
            "{} мен {} өзара байланысты",
            subject_cap,
            preferred_surface(&frame.fact.object)
        ));
        Some(wrap_introducer(
            frame.introducer,
            &frame.fact.subject.root,
            &body,
        ))
    }

    fn name(&self) -> &'static str {
        "RelatedToOzaraDeclarative"
    }
}

// ---------------------------------------------------------------------------

/// **v4.43.5** — GoesTo declarative: «X Y-ге барады», raw_text-prefer.
///
/// Most extracted GoesTo facts come from corpus pattern matching on
/// long source sentences whose raw_text is richer than mechanical
/// «X Y-ге барады» composition. The rule prefers raw_text when
/// non-empty (mirroring the IsACopulaDeclarative pattern), falling
/// back to the bare dative-motion shape when raw_text is empty
/// (curated facts that elect not to ship a long form).
pub struct GoesToDeclarative;

impl NlgRule for GoesToDeclarative {
    fn matches(&self, frame: &SentenceFrame) -> bool {
        matches!(frame.fact.predicate, ReasPredicate::GoesTo)
            && matches!(frame.mood, SentenceMood::Declarative)
    }

    fn render(&self, frame: &SentenceFrame) -> Option<String> {
        let raw = frame.fact.raw_text.trim();
        let body = if !raw.is_empty() {
            ensure_period(raw.to_string())
        } else {
            let subject_cap = capitalize_first(preferred_surface(&frame.fact.subject));
            ensure_period(format!(
                "{} {}-ге барады",
                subject_cap,
                preferred_surface(&frame.fact.object)
            ))
        };
        Some(wrap_introducer(
            frame.introducer,
            &frame.fact.subject.root,
            &body,
        ))
    }

    fn name(&self) -> &'static str {
        "GoesToDeclarative"
    }
}

// ---------------------------------------------------------------------------

/// **v4.43.5** — After declarative: «X-тен кейін Y болады»,
/// raw_text-prefer. Mirrors the [`GoesToDeclarative`] pattern.
pub struct AfterDeclarative;

impl NlgRule for AfterDeclarative {
    fn matches(&self, frame: &SentenceFrame) -> bool {
        matches!(frame.fact.predicate, ReasPredicate::After)
            && matches!(frame.mood, SentenceMood::Declarative)
    }

    fn render(&self, frame: &SentenceFrame) -> Option<String> {
        let raw = frame.fact.raw_text.trim();
        let body = if !raw.is_empty() {
            ensure_period(raw.to_string())
        } else {
            let subject_cap = capitalize_first(preferred_surface(&frame.fact.subject));
            ensure_period(format!(
                "{}-тен кейін {} болады",
                subject_cap,
                preferred_surface(&frame.fact.object)
            ))
        };
        Some(wrap_introducer(
            frame.introducer,
            &frame.fact.subject.root,
            &body,
        ))
    }

    fn name(&self) -> &'static str {
        "AfterDeclarative"
    }
}
