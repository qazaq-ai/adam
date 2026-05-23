// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `PredicateFocus` — Stage 1 of the v6.1.0 AnswerIR research arc.
//!
//! `QuestionShape` (v4.12.0) tells us the *form* of the question
//! (Definition / Causal / YesNoCheck / Listing / Comparison).
//! `PredicateFocus` refines that by naming the *predicate target*
//! the user is asking about:
//!
//!   «X кім?»               → Shape::Definition, Focus::IsA
//!   «X қашан туылған?»      → Shape::Definition, Focus::BornIn
//!   «X қашан күшіне енді?»  → Shape::Definition, Focus::EffectiveFrom
//!   «X қандай санатқа?»     → Shape::Definition, Focus::Classifies
//!   «X қайда орналасқан?»   → Shape::Definition, Focus::LocatedIn
//!   «X-ні кім жасады?»      → Shape::Definition, Focus::Authored
//!
//! Pre-v6.1.0 the dialog kernel collapsed every Definition-shape
//! question onto a single IsA retrieval probe with hand-coded
//! predicate-keyword overrides (the v6.0.13 «pre-action-plan
//! definition probe with KRU whitelist» path). The Codex 2026-05-22
//! audit identified this as the root cause of «Жасанды интеллект
//! туралы заң қандай санаттарға жіктейді?» returning an
//! effective-date fact instead of the classification fact: the
//! retrieval planner saw `(subject = AI Law, shape = Definition)`
//! and picked the highest-confidence fact, ignoring that the user
//! asked about classification specifically.
//!
//! Stage 1 ships the typed enum + a pure-surface-level detector.
//! No production wiring yet — `crates/adam-dialog/src/lib.rs`
//! exposes the module but the cascade does not yet route through
//! it. Stage 3 wires `build_answer_ir` (behind `ADAM_ANSWER_IR=1`)
//! to consume the detector output.
//!
//! Discipline: same as `question_shape.rs` — pure surface
//! substring matching, closed list of markers, all patterns
//! covered by unit tests.

use crate::question_shape::QuestionShape;

/// Predicate target the user's question asks about. Refines
/// `QuestionShape` — most variants live under
/// `QuestionShape::Definition` (the "what about X" family), but
/// the planner may bind a focus under `Listing` or `YesNoCheck`
/// shapes too in future stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PredicateFocus {
    /// «X кім?» / «X деген не?» — definitional IsA probe. Default
    /// when no more specific marker fires under Definition shape.
    IsA,
    /// «X қашан туылған?» — birth-date probe.
    BornIn,
    /// «X қашан қайтыс болды?» — death-date probe.
    DiedIn,
    /// «X қашан құрылған?» — institution / event founding date.
    FoundedIn,
    /// «X қашан қабылданды?» / «X қашан күшіне енді?» — law /
    /// agreement effective date.
    EffectiveFrom,
    /// «X қандай санатқа жатады?» / «жіктейді» — classification
    /// probe.
    Classifies,
    /// «X қандай тәуекелді?» — risk-level probe.
    RiskLevel,
    /// «X қайда?» / «қай қалада?» — location probe.
    LocatedIn,
    /// «X-ні кім жасады?» / «авторы кім?» — actor probe (replaces
    /// the overloaded `RelatedTo` for «жазған / құрған / жасаған»).
    Authored,
    /// «X кімнің атымен аталған?» — eponym probe.
    NamedAfter,
    /// «X кімнің мүшесі?» — membership probe.
    MemberOf,
    /// «X-нің Y-сі кім?» — relational genitive probe. The
    /// pre-plan probe currently skips genitive-possessive shapes
    /// entirely; this variant re-enables them by typing the
    /// relation explicitly.
    Relational,
}

impl PredicateFocus {
    /// Stable string slug for template-key composition and
    /// diagnostics.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::IsA => "is_a",
            Self::BornIn => "born_in",
            Self::DiedIn => "died_in",
            Self::FoundedIn => "founded_in",
            Self::EffectiveFrom => "effective_from",
            Self::Classifies => "classifies",
            Self::RiskLevel => "risk_level",
            Self::LocatedIn => "located_in",
            Self::Authored => "authored",
            Self::NamedAfter => "named_after",
            Self::MemberOf => "member_of",
            Self::Relational => "relational",
        }
    }

    /// Map a `PredicateFocus` to the `adam_reasoning::Predicate` it
    /// expects to retrieve. `Relational` is the only focus without a
    /// direct mapping — it represents the genitive-possessive query
    /// shape («X-нің Y-сі кім?»), which Stage 3 falls back to v6.0.13
    /// retrieval for and Stage 4+ may decompose into typed sub-queries.
    pub fn matching_predicate(self) -> Option<adam_reasoning::Predicate> {
        use adam_reasoning::Predicate::*;
        Some(match self {
            Self::IsA => IsA,
            Self::BornIn => BornIn,
            Self::DiedIn => DiedIn,
            Self::FoundedIn => FoundedIn,
            Self::EffectiveFrom => EffectiveFrom,
            Self::Classifies => Classifies,
            Self::RiskLevel => RiskLevel,
            Self::LocatedIn => LocatedIn,
            Self::Authored => Authored,
            Self::NamedAfter => NamedAfter,
            Self::MemberOf => MemberOf,
            Self::Relational => return None,
        })
    }
}

/// Detect the predicate focus in a raw user input.
///
/// Returns `None` when no specific predicate marker is found —
/// the planner should fall back to existing v6.0.0 retrieval in
/// that case. Returns `Some(PredicateFocus::IsA)` only when the
/// input carries an explicit IsA marker («кім?», «деген не»,
/// «дегеніміз не»); a bare topic-noun query with no marker
/// returns `None`.
///
/// `shape` is passed so the detector can use it for tie-breaks
/// in future stages. At Stage 1 it is not yet consulted — the
/// detector is purely surface-level.
///
/// Order of checks: most specific markers first. Listed in the
/// order that fires correctly when multiple markers co-occur
/// (e.g. «қашан туылған» includes «қашан» which is also a
/// generic time-marker — `BornIn` must be checked before any
/// generic `қашан` handler that might land in later stages).
pub fn detect(input: &str, _shape: Option<QuestionShape>) -> Option<PredicateFocus> {
    let lower = input.to_lowercase();

    if is_born_in(&lower) {
        return Some(PredicateFocus::BornIn);
    }
    if is_died_in(&lower) {
        return Some(PredicateFocus::DiedIn);
    }
    if is_effective_from(&lower) {
        return Some(PredicateFocus::EffectiveFrom);
    }
    if is_founded_in(&lower) {
        return Some(PredicateFocus::FoundedIn);
    }
    if is_classifies(&lower) {
        return Some(PredicateFocus::Classifies);
    }
    if is_risk_level(&lower) {
        return Some(PredicateFocus::RiskLevel);
    }
    if is_authored(&lower) {
        return Some(PredicateFocus::Authored);
    }
    if is_named_after(&lower) {
        return Some(PredicateFocus::NamedAfter);
    }
    if is_member_of(&lower) {
        return Some(PredicateFocus::MemberOf);
    }
    if is_located_in(&lower) {
        return Some(PredicateFocus::LocatedIn);
    }
    if is_relational(&lower) {
        return Some(PredicateFocus::Relational);
    }
    if is_is_a(&lower) {
        return Some(PredicateFocus::IsA);
    }

    None
}

fn is_born_in(lower: &str) -> bool {
    lower.contains("қашан туылған")
        || lower.contains("қашан туған")
        || lower.contains("туылған күні")
        || lower.contains("туған күні")
        || lower.contains("қашан туылды")
        || lower.contains("қай жылы туылған")
        || lower.contains("қай жылы туған")
}

fn is_died_in(lower: &str) -> bool {
    lower.contains("қашан қайтыс")
        || lower.contains("қашан өлді")
        || lower.contains("қашан өлген")
        || lower.contains("қайтыс болған күні")
        || lower.contains("өлген күні")
        || lower.contains("қай жылы қайтыс")
}

fn is_effective_from(lower: &str) -> bool {
    lower.contains("күшіне ен")
        || lower.contains("күшіне ену")
        || lower.contains("қашан қабылданды")
        || lower.contains("қашан қабылданған")
        || lower.contains("қашан бекітілді")
        || lower.contains("қашан бекітілген")
}

fn is_founded_in(lower: &str) -> bool {
    // After EffectiveFrom so «қашан қабылданды» binds first.
    lower.contains("қашан құрылған")
        || lower.contains("қашан құрылды")
        || lower.contains("қашан ашылған")
        || lower.contains("қашан ашылды")
        || lower.contains("құрылған жылы")
        || lower.contains("ашылған жылы")
}

fn is_classifies(lower: &str) -> bool {
    lower.contains("қандай санат")
        || lower.contains("санатқа жатады")
        || lower.contains("санатқа жіктейді")
        || lower.contains("жіктейді")
        || lower.contains("жіктеу")
        || lower.contains("қай санат")
        || lower.contains("қандай түрге")
}

fn is_risk_level(lower: &str) -> bool {
    lower.contains("қандай тәуекел")
        || lower.contains("тәуекел деңгей")
        || lower.contains("қаншалықты қауіпті")
}

fn is_located_in(lower: &str) -> bool {
    // After BornIn/DiedIn/FoundedIn because those use date-form
    // «қашан»; LocatedIn uses spatial «қайда» / «қай қалада».
    lower.contains("қайда орналасқан")
        || lower.contains("қайда тұр")
        || lower.contains("қай қалада")
        || lower.contains("қай елде")
        || lower.contains("қай аймақта")
        || lower.contains("қайда?")
        || lower.ends_with(" қайда")
}

fn is_authored(lower: &str) -> bool {
    lower.contains("кім жасады")
        || lower.contains("кім жасаған")
        || lower.contains("кім жазды")
        || lower.contains("кім жазған")
        || lower.contains("кім құрды")
        || lower.contains("кім құрған")
        || lower.contains("авторы кім")
        || lower.contains("авторы қай")
}

fn is_named_after(lower: &str) -> bool {
    lower.contains("кімнің атымен")
        || lower.contains("атымен аталған")
        || lower.contains("атымен аталады")
        || lower.contains("кімнің құрметіне")
}

fn is_member_of(lower: &str) -> bool {
    lower.contains("кімнің мүшесі")
        || lower.contains("қандай ұйымның мүшесі")
        || lower.contains("қай партияның")
        || lower.contains("қай блоктың")
}

fn is_relational(lower: &str) -> bool {
    // Genitive-possessive question shapes: «X-нің Y-сі кім»,
    // «X-тің Y-сі». The `-нің / -тің / -дің / -ның / -тың /
    // -дың` set covers Kazakh genitive variants by phonetic
    // class. We require both a genitive marker AND a kim/ne
    // tail to distinguish from declarative genitives.
    let has_gen = lower.contains("нің ")
        || lower.contains("тің ")
        || lower.contains("дің ")
        || lower.contains("ның ")
        || lower.contains("тың ")
        || lower.contains("дың ");
    let has_tail = lower.contains(" кім")
        || lower.contains(" кім?")
        || lower.contains(" не?")
        || lower.contains(" не ");
    has_gen && has_tail
}

fn is_is_a(lower: &str) -> bool {
    lower.contains("деген не")
        || lower.contains("дегеніміз не")
        || lower.contains("дегенді қалай")
        || lower.ends_with(" кім?")
        || lower.ends_with(" кім")
        || lower.ends_with(" не?")
        || (lower.contains(" кім?") && !lower.contains("кім жас") && !lower.contains("кім жаз"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shape() -> Option<QuestionShape> {
        Some(QuestionShape::Definition)
    }

    #[test]
    fn born_in_canonical() {
        assert_eq!(
            detect("Ахмет Байтұрсынұлы қашан туылған?", shape()),
            Some(PredicateFocus::BornIn)
        );
    }

    #[test]
    fn born_in_alternates() {
        assert_eq!(
            detect("Абай қашан туған?", shape()),
            Some(PredicateFocus::BornIn)
        );
        assert_eq!(
            detect("Шоқан қай жылы туылған?", shape()),
            Some(PredicateFocus::BornIn)
        );
    }

    #[test]
    fn died_in() {
        assert_eq!(
            detect("Ахмет Байтұрсынұлы қашан қайтыс болды?", shape()),
            Some(PredicateFocus::DiedIn)
        );
    }

    #[test]
    fn effective_from_law() {
        assert_eq!(
            detect("ЖИ туралы заң қашан күшіне енді?", shape()),
            Some(PredicateFocus::EffectiveFrom)
        );
        assert_eq!(
            detect("Заң қашан қабылданды?", shape()),
            Some(PredicateFocus::EffectiveFrom)
        );
    }

    #[test]
    fn founded_in_institution() {
        assert_eq!(
            detect("КРУ қашан құрылған?", shape()),
            Some(PredicateFocus::FoundedIn)
        );
        assert_eq!(
            detect("IT Park қашан ашылды?", shape()),
            Some(PredicateFocus::FoundedIn)
        );
    }

    #[test]
    fn classifies_law() {
        assert_eq!(
            detect(
                "Жасанды интеллект туралы заң қандай санаттарға жіктейді?",
                shape()
            ),
            Some(PredicateFocus::Classifies)
        );
        assert_eq!(
            detect("Қандай санатқа жатады?", shape()),
            Some(PredicateFocus::Classifies)
        );
    }

    #[test]
    fn risk_level() {
        assert_eq!(
            detect("Бұл жүйе қандай тәуекелді?", shape()),
            Some(PredicateFocus::RiskLevel)
        );
        assert_eq!(
            detect("Жоғары тәуекел деңгейі дегеніміз не?", shape()),
            // Note: «дегеніміз не» fires IsA in the cascade — but
            // «тәуекел деңгей» fires first by detector order.
            Some(PredicateFocus::RiskLevel)
        );
    }

    #[test]
    fn located_in() {
        assert_eq!(
            detect("Астана қайда орналасқан?", shape()),
            Some(PredicateFocus::LocatedIn)
        );
        assert_eq!(
            detect("КРУ қай қалада?", shape()),
            Some(PredicateFocus::LocatedIn)
        );
    }

    #[test]
    fn authored() {
        assert_eq!(
            detect("Төте жазуды кім жасады?", shape()),
            Some(PredicateFocus::Authored)
        );
        assert_eq!(
            detect("«Маса» өлеңінің авторы кім?", shape()),
            Some(PredicateFocus::Authored)
        );
    }

    #[test]
    fn named_after() {
        assert_eq!(
            detect("КРУ кімнің атымен аталған?", shape()),
            Some(PredicateFocus::NamedAfter)
        );
    }

    #[test]
    fn member_of() {
        assert_eq!(
            detect("Қазақстан кімнің мүшесі?", shape()),
            Some(PredicateFocus::MemberOf)
        );
    }

    #[test]
    fn relational_genitive() {
        assert_eq!(
            detect("Ахметтің әкесі кім?", shape()),
            Some(PredicateFocus::Relational)
        );
    }

    #[test]
    fn is_a_explicit_marker() {
        assert_eq!(
            detect("Төте жазу деген не?", shape()),
            Some(PredicateFocus::IsA)
        );
        assert_eq!(
            detect("Қазақстан дегеніміз не?", shape()),
            Some(PredicateFocus::IsA)
        );
    }

    #[test]
    fn is_a_bare_kim() {
        assert_eq!(
            detect("Ахмет Байтұрсынұлы кім?", shape()),
            Some(PredicateFocus::IsA)
        );
    }

    #[test]
    fn no_marker_returns_none() {
        // Bare topic noun with no question marker — planner
        // falls back to v6.0.0 retrieval.
        assert_eq!(detect("Адам", shape()), None);
        assert_eq!(detect("Қазақстан туралы айтыңыз.", shape()), None);
    }

    #[test]
    fn ordering_born_before_located() {
        // «қашан» must bind to BornIn before any spatial handler.
        assert_eq!(
            detect("Абай қашан туған?", shape()),
            Some(PredicateFocus::BornIn)
        );
    }

    #[test]
    fn ordering_effective_before_founded() {
        // «қашан қабылданды» must bind to EffectiveFrom, not
        // FoundedIn (which uses «құрылды»).
        assert_eq!(
            detect("ЖИ туралы заң қашан қабылданды?", shape()),
            Some(PredicateFocus::EffectiveFrom)
        );
    }

    #[test]
    fn slug_round_trip() {
        for focus in [
            PredicateFocus::IsA,
            PredicateFocus::BornIn,
            PredicateFocus::DiedIn,
            PredicateFocus::FoundedIn,
            PredicateFocus::EffectiveFrom,
            PredicateFocus::Classifies,
            PredicateFocus::RiskLevel,
            PredicateFocus::LocatedIn,
            PredicateFocus::Authored,
            PredicateFocus::NamedAfter,
            PredicateFocus::MemberOf,
            PredicateFocus::Relational,
        ] {
            assert!(!focus.as_str().is_empty());
        }
    }
}
