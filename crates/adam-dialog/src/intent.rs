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

    // --- v0.8.0 social topic intents --------------------------------------
    /// User introduces self by name: "менің атым X", "мені X деп атайды".
    /// The extracted `name` is surfaced so templates can personalise:
    /// e.g. "қош келдіңіз {name}".
    StatementOfName { name: String },

    /// "How old are you?": жасың неше, жасыңыз қанша, қанша жастасың.
    AskAge,

    /// User states age: "менің жасым отыз", "жиырма жастамын".
    /// `years` is `Some(n)` when a Kazakh numeral 1–99 was parsed out
    /// of the utterance, `None` when the intent matched on keywords
    /// alone ("жасым жасырын").
    StatementOfAge { years: Option<u32> },

    /// "Where are you from / where do you live?":
    /// қай жерденсің, қайда тұрасыз, қай қаладансың.
    AskLocation,

    /// User says where they are from / live: "мен Алматыданмын",
    /// "астанада тұрамын". `city` is the extracted root (nominative
    /// form, case-preserved) when the case+copula stripping succeeded.
    StatementOfLocation { city: Option<String> },

    /// "What do you do?": немен айналысасың, жұмысың не, кәсібің қандай.
    AskOccupation,

    /// User states occupation: "мен мұғаліммін", "дәрігер болып жұмыс
    /// істеймін". `occupation` is the extracted noun root (1sg copula
    /// stripped) when possible.
    StatementOfOccupation { occupation: Option<String> },

    /// "Are you married? / Do you have children?": үйлендің бе,
    /// балаларың бар ма, отбасың бар ма.
    AskFamily,

    /// User talks about their family: "менің екі балам бар",
    /// "үйленгенмін", "менің отбасым бар".
    StatementOfFamily,

    /// "How's the weather?": ауа райы қалай, бүгін ауа райы.
    AskWeather,

    /// User describes weather: "бүгін суық", "жылы", "қар жауып тұр",
    /// "ауа райы жақсы".
    StatementOfWeather,

    /// "What time is it? / What day?": сағат неше, қазір қай уақыт,
    /// бүгін қандай күн.
    AskTime,

    /// Compliment / praise: жарайсың, өте жақсы, керемет.
    Compliment,

    /// Polite request: өтінемін, сұраймын, көмектесіңізші.
    Request,

    /// Well-wishes: жақсы күн тілеймін, сәттілік, табысты болыңыз.
    WellWishes,

    /// User is rude / insulting (ақымақ, надан, түкке тұрмайсың). The
    /// response is polite non-engagement — the model does not escalate
    /// or retaliate. Added v1.1.0.
    Insult,

    /// Nothing matched. Fallback may carry a `noun_hint` extracted from
    /// the input by the FST parser so the response can at least
    /// acknowledge what the user is talking about, rather than blank
    /// "түсінбедім". `example` (v1.6.5) optionally carries a native
    /// Kazakh sentence retrieved from the committed morpheme index for
    /// the `noun_hint`, so the response can cite concrete evidence
    /// rather than just echoing the noun.
    Unknown {
        raw_tokens: Vec<String>,
        /// First parsed noun root, if any — populated by the lexicon-aware
        /// `interpret_text_with_lexicon` path.
        noun_hint: Option<String>,
        /// A sample sentence from the committed corpus that contains
        /// the `noun_hint`. Populated by `Conversation::turn` when a
        /// `MorphemeIndex` is attached. Added v1.6.5.
        #[serde(default)]
        example: Option<String>,
    },
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
