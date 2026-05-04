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

use super::{Introducer, NlgRule, SentenceFrame, SentenceMood, capitalize_first};
use adam_reasoning::Predicate as ReasPredicate;

/// Wrap `body` in the introducer's preamble.
fn wrap_introducer(introducer: Introducer, topic: &str, body: &str) -> String {
    let topic_cap = capitalize_first(topic);
    match introducer {
        Introducer::Direct => body.to_string(),
        Introducer::AboutTopicFirst => {
            format!("{topic_cap} туралы ең әуелі мынаны айтуға болады: {body}")
        }
        Introducer::OnTheSubjectMain => {
            format!("{topic_cap} жайында негізгі дерек мынау: {body}")
        }
        Introducer::BrieflyAbout => {
            format!("{topic_cap} туралы қысқаша айтсам: {body}")
        }
    }
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

/// IsA copula declarative: «X — Y».
///
/// Surfaces a definitional fact via the em-dash copula — the most
/// frequent factoid response shape in adam.
pub struct IsACopulaDeclarative;

impl NlgRule for IsACopulaDeclarative {
    fn matches(&self, frame: &SentenceFrame) -> bool {
        matches!(frame.fact.predicate, ReasPredicate::IsA)
            && matches!(frame.mood, SentenceMood::Declarative)
    }

    fn render(&self, frame: &SentenceFrame) -> Option<String> {
        let subject_cap = capitalize_first(&frame.fact.subject.surface);
        let body = ensure_period(format!("{} — {}", subject_cap, frame.fact.object.surface));
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
        let subject_cap = capitalize_first(&frame.fact.subject.surface);
        let body = ensure_period(format!(
            "{} {} құрамына кіреді",
            subject_cap, frame.fact.object.surface
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
        let subject_cap = capitalize_first(&frame.fact.subject.surface);
        let body = ensure_period(format!(
            "{} мекені — {}",
            subject_cap, frame.fact.object.surface
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
