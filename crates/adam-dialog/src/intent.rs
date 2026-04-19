//! Intent taxonomy — the semantic categories the v0.7.0 MVP recognises.
//!
//! Each variant is a self-contained bundle of (intent-kind, entities).
//! Adding a new intent means: (a) extend this enum, (b) write a
//! recogniser rule in `semantics.rs`, (c) register templates in
//! `data/dialog/templates/`.

use adam_kernel_fst::morphotactics::Number;
use serde::{Deserialize, Serialize};

/// The top-level semantic category of a user utterance.
///
/// v0.7.0 MVP covers 5 intents. Subsequent releases widen this enum —
/// all downstream code (planner, realiser, templates) dispatches on
/// the enum so expanding it is strictly additive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Intent {
    /// Social greeting: сәлем, сәлеметсіз бе, қайырлы таң, т.б.
    Greeting { kind: GreetingKind },

    /// Goodbye: сау бол, кездескенше.
    Farewell,

    /// Yes / affirmation: иә, дұрыс, рас.
    Affirmation,

    /// No / denial: жоқ, қате, емес.
    Negation,

    /// Thank you: рахмет, көп рахмет, рахметім.
    Thanks,

    /// Sorry / excuse me: кешіріңіз, ғафу етіңіз.
    Apology,

    /// "How are you?": қалайсың, қалайсыз, жағдайыңыз қалай.
    AskHowAreYou,

    /// User is reporting their wellbeing: жақсымын, жаман емеспін.
    StatementOfWellbeing,

    /// "What's your name?": атың кім, есіміңіз қалай.
    AskName,

    /// Nothing matched. Fallback response is a polite
    /// "түсінбедім" (I didn't understand).
    Unknown { raw_tokens: Vec<String> },
}

/// Which flavour of greeting the user used. Determines whether the
/// response is a mirror ("сәлем"→"сәлем") or an upgrade
/// ("сәлем"→"сәлеметсіз бе").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GreetingKind {
    /// Casual "сәлем".
    Casual,
    /// Polite "сәлеметсіз бе".
    Polite,
    /// Time-specific "қайырлы таң" / "қайырлы күн" / "қайырлы кеш".
    TimeOfDay(TimeOfDay),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeOfDay {
    Morning,
    Day,
    Evening,
}

/// Person + number of a subject as recognised in an utterance.
/// Future intents (asks/statements-of-location etc.) will carry this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubjectPerson {
    First(Number),
    Second(Number, Politeness),
    Third(Number),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Politeness {
    Informal,
    Polite,
}
