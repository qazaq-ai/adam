// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `realiser` — **Stage 7 of v6.2.0**: typed Frame → Kazakh
//! surface form.
//!
//! Closes the v6.2 pipeline: every other layer produces typed
//! data; the realiser is the **only** place where Kazakh strings
//! are composed. Mirrors the v6.1 NLG rule families
//! (`adam_dialog::nlg::rules`) but consumes a typed
//! `(Frame, QueryFocus, RankedFrame)` triple instead of the
//! v6.1 ad-hoc `SentenceFrame`.
//!
//! ## Determinism contract
//!
//! - Pure function: input determines output byte-for-byte.
//! - No I/O, no randomness, no clock reads.
//! - Variant selection is `seed % pool.len()` — same seed →
//!   same surface.
//!
//! ## Coverage
//!
//! Every v6.1 NLG rule family is expressible through one of these
//! realisers:
//!
//! - `IsA`: «X — Y.»
//! - `PartOf`: «X Y құрамына кіреді.»
//! - `HasQuantity`: «X-да Y бар.»
//! - `LivesIn`: «X мекені — Y.»
//! - `Has`: «X Y иеленеді.»
//! - `Causes`: «X — Y себебі.»
//! - `After`: «X — Y-ден кейін.»
//! - `GoesTo`: «X Y-ге барады.»
//! - `DoesTo`: «X Y-ні N-лайды.»
//! - `InDomain`: «X Y саласына жатады.»
//! - `RelatedTo`: «X пен Y өзара байланысты.»
//! - `BornIn`: «X N жылы туылған.»
//! - `DiedIn`: «X N жылы қайтыс болған.»
//! - `FoundedIn`: «X N жылы құрылған.»
//! - `LocatedIn`: «X Y-да орналасқан.»
//! - `Authored`: «X-ны Y жазған.»
//! - `Classifies`: «X Y-ға жіктейді.»
//! - `EffectiveFrom`: «X N жылдың N1 N2-нен күшіне енген.»
//! - `MemberOf`: «X — Y мүшесі.»
//! - `NamedAfter`: «X Y атымен аталған.»
//! - `RenamedIn`: «X N жылы атауы өзгерген.»
//! - `RiskLevel`: «X — Y тәуекелді.»

use crate::frame::{Frame, FramePredicate, Modifier, TimeAnchor};
use crate::query::{AnswerSlot, ModifierRole, QueryFocus};

/// Realise a typed `(Frame, QueryFocus, AnswerSlot)` triple into
/// a Kazakh surface string.
///
/// The `frame` is the candidate the index returned. The `focus`
/// tells what slot the user asked about. The `answer_slot` (from
/// [`crate::FrameMatch`]) confirms what the frame supplied.
pub fn realise(frame: &Frame, focus: &QueryFocus, slot: AnswerSlot) -> String {
    match focus {
        // Slot-specific focuses: emit the slot's surface directly.
        QueryFocus::Subject => emit_subject(frame, slot),
        QueryFocus::Object => emit_object(frame, slot),
        QueryFocus::Predicate => emit_predicate_focus(frame),
        QueryFocus::Modifier(role) => emit_modifier(frame, *role),
        QueryFocus::Quantity => emit_quantity(frame),
        // Whole-frame focuses: render the full sentence.
        QueryFocus::Existence => emit_yes_no(frame),
        QueryFocus::Definition => emit_definition(frame),
        QueryFocus::Enumeration => emit_enumeration(frame),
    }
}

fn emit_subject(frame: &Frame, _slot: AnswerSlot) -> String {
    match &frame.agent {
        Some(a) => capitalize(&a.root.surface),
        None => "(белгісіз)".to_string(),
    }
}

fn emit_object(frame: &Frame, _slot: AnswerSlot) -> String {
    match &frame.object {
        Some(o) => capitalize(&o.root.surface),
        None => "(белгісіз)".to_string(),
    }
}

fn emit_predicate_focus(frame: &Frame) -> String {
    // Predicate-focused answers describe what happens between agent
    // and object — the verb / relation expressed in canonical
    // Kazakh form.
    let agent = frame
        .agent
        .as_ref()
        .map(|c| c.root.surface.as_str())
        .unwrap_or("ол");
    let object = frame
        .object
        .as_ref()
        .map(|c| c.root.surface.as_str())
        .unwrap_or("");
    match frame.predicate {
        FramePredicate::Causes => format!("{} — {} себебі.", capitalize(agent), object),
        FramePredicate::IsA => format!("{} — {}.", capitalize(agent), object),
        _ => format!(
            "{} {}.",
            capitalize(agent),
            predicate_short_form(frame.predicate.clone())
        ),
    }
}

fn emit_modifier(frame: &Frame, role: ModifierRole) -> String {
    let Some(m) = frame.modifier(role.as_str()) else {
        return "(нақты дерек жоқ)".to_string();
    };
    modifier_surface(m).unwrap_or_else(|| "(дерек жоқ)".to_string())
}

fn emit_quantity(frame: &Frame) -> String {
    // Quantity answers fall through to whichever slot holds the
    // count phrase. The curated corpus puts the count in `object`
    // («Қазақстанда 17 облыс бар» → object = «17 облыс»).
    let agent = frame
        .agent
        .as_ref()
        .map(|c| c.root.surface.as_str())
        .unwrap_or("");
    let object = frame
        .object
        .as_ref()
        .map(|c| c.root.surface.as_str())
        .unwrap_or("");
    if !agent.is_empty() && !object.is_empty() {
        format!("{}-да {} бар.", capitalize(agent), object)
    } else if !object.is_empty() {
        capitalize(object)
    } else {
        "(дерек жоқ)".to_string()
    }
}

fn emit_yes_no(frame: &Frame) -> String {
    // Yes/no confirmation — affirmative renders «Иә, ...».
    use crate::frame::Polarity;
    let body = emit_definition(frame);
    match frame.polarity {
        Polarity::Affirmative => format!("Иә, {}", body),
        Polarity::Negated => format!("Жоқ, {}", body),
    }
}

fn emit_definition(frame: &Frame) -> String {
    let agent = frame
        .agent
        .as_ref()
        .map(|c| c.root.surface.as_str())
        .unwrap_or("ол");
    let object = frame
        .object
        .as_ref()
        .map(|c| c.root.surface.as_str())
        .unwrap_or("");
    match frame.predicate {
        FramePredicate::IsA | FramePredicate::Definition => {
            format!("{} — {}.", capitalize(agent), object)
        }
        FramePredicate::PartOf => format!("{} {} құрамына кіреді.", capitalize(agent), object),
        FramePredicate::LivesIn => format!("{} мекені — {}.", capitalize(agent), object),
        FramePredicate::Has => format!("{} {} иеленеді.", capitalize(agent), object),
        FramePredicate::Causes => format!("{} — {} себебі.", capitalize(agent), object),
        FramePredicate::After => format!("{} {}-ден кейін.", capitalize(agent), object),
        FramePredicate::GoesTo => format!("{} {}-ге барады.", capitalize(agent), object),
        FramePredicate::DoesTo => format!("{} {}-ні әсер етеді.", capitalize(agent), object),
        FramePredicate::InDomain => format!("{} {} саласына жатады.", capitalize(agent), object),
        FramePredicate::RelatedTo => {
            format!("{} мен {} өзара байланысты.", capitalize(agent), object)
        }
        FramePredicate::BornIn => {
            let time = frame
                .modifier("time")
                .and_then(modifier_surface)
                .unwrap_or_default();
            if !time.is_empty() {
                format!("{} {} жылы туылған.", capitalize(agent), time)
            } else {
                format!("{} туылған.", capitalize(agent))
            }
        }
        FramePredicate::DiedIn => {
            let time = frame
                .modifier("time")
                .and_then(modifier_surface)
                .unwrap_or_default();
            if !time.is_empty() {
                format!("{} {} жылы қайтыс болған.", capitalize(agent), time)
            } else {
                format!("{} қайтыс болған.", capitalize(agent))
            }
        }
        FramePredicate::FoundedIn => {
            let time = frame
                .modifier("time")
                .and_then(modifier_surface)
                .unwrap_or_default();
            if !time.is_empty() {
                format!("{} {} жылы құрылған.", capitalize(agent), time)
            } else {
                format!("{} құрылған.", capitalize(agent))
            }
        }
        FramePredicate::RenamedIn => {
            let time = frame
                .modifier("time")
                .and_then(modifier_surface)
                .unwrap_or_default();
            format!("{} {} жылы атауы өзгерген.", capitalize(agent), time)
        }
        FramePredicate::EffectiveFrom => {
            let time = frame
                .modifier("time")
                .and_then(modifier_surface)
                .unwrap_or_default();
            format!("{} {}-нан күшіне енген.", capitalize(agent), time)
        }
        FramePredicate::Classifies => {
            format!("{} {}-ға жіктейді.", capitalize(agent), object)
        }
        FramePredicate::RiskLevel => format!("{} — {} тәуекелді.", capitalize(agent), object),
        FramePredicate::LocatedIn => {
            format!("{} {}-да орналасқан.", capitalize(agent), object)
        }
        FramePredicate::NamedAfter => {
            format!("{} {} атымен аталған.", capitalize(agent), object)
        }
        FramePredicate::MemberOf => format!("{} — {} мүшесі.", capitalize(agent), object),
        FramePredicate::Authored => {
            format!("{}-ны {} жазған.", capitalize(agent), object)
        }
        FramePredicate::HasQuantity => emit_quantity(frame),
        FramePredicate::HasProperty => format!("{} — {}.", capitalize(agent), object),
        FramePredicate::SystemSelf => agent.to_string(),
    }
}

fn emit_enumeration(frame: &Frame) -> String {
    // Enumeration falls back to object-as-list. Stage 8 lifts this
    // with curated list-summary frames.
    frame
        .object
        .as_ref()
        .map(|c| c.root.surface.clone())
        .unwrap_or_else(|| "(дерек жоқ)".to_string())
}

fn modifier_surface(m: &Modifier) -> Option<String> {
    match m {
        Modifier::TimeAnchor(TimeAnchor::Phrase(c)) => Some(c.root.surface.clone()),
        Modifier::TimeAnchor(TimeAnchor::Year(y)) => Some(format!("{y}")),
        Modifier::TimeAnchor(TimeAnchor::Date { year, month, day }) => {
            Some(format!("{year:04}-{month:02}-{day:02}"))
        }
        Modifier::Location(c)
        | Modifier::Source(c)
        | Modifier::Instrument(c)
        | Modifier::Manner(c)
        | Modifier::Recipient(c)
        | Modifier::Possessor(c) => Some(c.root.surface.clone()),
    }
}

fn predicate_short_form(p: FramePredicate) -> String {
    match p {
        FramePredicate::Causes => "себебі болады".to_string(),
        FramePredicate::BornIn => "туылған".to_string(),
        FramePredicate::DiedIn => "қайтыс болған".to_string(),
        FramePredicate::FoundedIn => "құрылған".to_string(),
        FramePredicate::Authored => "жазған".to_string(),
        FramePredicate::LivesIn => "тұрады".to_string(),
        FramePredicate::GoesTo => "барады".to_string(),
        _ => p.as_str().to_string(),
    }
}

/// Capitalise the first character of a Kazakh / Russian string.
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composition::Composition;
    use crate::frame::{Frame, FramePredicate, Modifier, TimeAnchor};
    use crate::root::{PartOfSpeech, Root};

    fn noun(s: &str) -> Composition {
        Composition::identity(Root::new(s, PartOfSpeech::Noun))
    }

    #[test]
    fn isa_definition_renders() {
        let f = Frame::assertion(
            Some(noun("қазақстан")),
            FramePredicate::IsA,
            Some(noun("мемлекет")),
        );
        let out = realise(&f, &QueryFocus::Definition, AnswerSlot::Whole);
        assert_eq!(out, "Қазақстан — мемлекет.");
    }

    #[test]
    fn born_in_with_year_modifier() {
        let f = Frame::assertion(Some(noun("ахмет")), FramePredicate::BornIn, None)
            .with_modifier(Modifier::TimeAnchor(TimeAnchor::Year(1872)));
        let out = realise(&f, &QueryFocus::Definition, AnswerSlot::Whole);
        assert_eq!(out, "Ахмет 1872 жылы туылған.");
    }

    #[test]
    fn time_focus_returns_year_string() {
        let f = Frame::assertion(Some(noun("ахмет")), FramePredicate::BornIn, None)
            .with_modifier(Modifier::TimeAnchor(TimeAnchor::Year(1872)));
        let out = realise(
            &f,
            &QueryFocus::Modifier(ModifierRole::Time),
            AnswerSlot::Modifier(ModifierRole::Time),
        );
        assert_eq!(out, "1872");
    }

    #[test]
    fn subject_focus_returns_capitalised_agent() {
        let f = Frame::assertion(
            Some(noun("ахмет байтұрсынұлы")),
            FramePredicate::BornIn,
            None,
        );
        let out = realise(&f, &QueryFocus::Subject, AnswerSlot::Agent);
        assert_eq!(out, "Ахмет байтұрсынұлы");
    }

    #[test]
    fn object_focus_returns_object() {
        let f = Frame::assertion(
            Some(noun("қазақстан")),
            FramePredicate::IsA,
            Some(noun("мемлекет")),
        );
        let out = realise(&f, &QueryFocus::Object, AnswerSlot::Object);
        assert_eq!(out, "Мемлекет");
    }

    #[test]
    fn has_quantity_renders_kazakhstan_oblasts() {
        let f = Frame::assertion(
            Some(noun("қазақстан")),
            FramePredicate::HasQuantity,
            Some(noun("17 облыс")),
        );
        let out = realise(&f, &QueryFocus::Quantity, AnswerSlot::Whole);
        assert_eq!(out, "Қазақстан-да 17 облыс бар.");
    }

    #[test]
    fn part_of_renders() {
        let f = Frame::assertion(
            Some(noun("астана")),
            FramePredicate::PartOf,
            Some(noun("қазақстан")),
        );
        let out = realise(&f, &QueryFocus::Definition, AnswerSlot::Whole);
        assert_eq!(out, "Астана қазақстан құрамына кіреді.");
    }

    #[test]
    fn located_in_renders() {
        let f = Frame::assertion(
            Some(noun("кру")),
            FramePredicate::LocatedIn,
            Some(noun("қостанай")),
        );
        let out = realise(&f, &QueryFocus::Definition, AnswerSlot::Whole);
        assert_eq!(out, "Кру қостанай-да орналасқан.");
    }

    #[test]
    fn yes_no_confirms_affirmative() {
        let f = Frame::assertion(
            Some(noun("қазақстан")),
            FramePredicate::IsA,
            Some(noun("мемлекет")),
        );
        let out = realise(&f, &QueryFocus::Existence, AnswerSlot::Whole);
        assert_eq!(out, "Иә, Қазақстан — мемлекет.");
    }

    #[test]
    fn yes_no_denies_negated() {
        use crate::frame::Polarity;
        let f = Frame::assertion(Some(noun("тас")), FramePredicate::IsA, Some(noun("тірі")))
            .with_polarity(Polarity::Negated);
        let out = realise(&f, &QueryFocus::Existence, AnswerSlot::Whole);
        assert_eq!(out, "Жоқ, Тас — тірі.");
    }

    #[test]
    fn all_22_v6_1_predicates_realise_without_panic() {
        // Smoke test: every FramePredicate emits a non-empty string
        // for the Definition focus.
        let all = [
            FramePredicate::IsA,
            FramePredicate::LivesIn,
            FramePredicate::Has,
            FramePredicate::GoesTo,
            FramePredicate::PartOf,
            FramePredicate::RelatedTo,
            FramePredicate::Causes,
            FramePredicate::After,
            FramePredicate::HasQuantity,
            FramePredicate::DoesTo,
            FramePredicate::InDomain,
            FramePredicate::BornIn,
            FramePredicate::DiedIn,
            FramePredicate::FoundedIn,
            FramePredicate::RenamedIn,
            FramePredicate::EffectiveFrom,
            FramePredicate::Classifies,
            FramePredicate::RiskLevel,
            FramePredicate::LocatedIn,
            FramePredicate::NamedAfter,
            FramePredicate::MemberOf,
            FramePredicate::Authored,
        ];
        for p in all {
            let f = Frame::assertion(Some(noun("x")), p.clone(), Some(noun("y")));
            let out = realise(&f, &QueryFocus::Definition, AnswerSlot::Whole);
            assert!(!out.is_empty(), "{:?} produced empty surface", p);
        }
    }
}
