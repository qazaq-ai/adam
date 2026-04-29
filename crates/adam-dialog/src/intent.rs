//! Intent taxonomy — the semantic categories the v0.7.0 MVP recognises.
//!
//! Each variant is a self-contained bundle of (intent-kind, entities).
//! Adding a new intent means: (a) extend this enum, (b) write a
//! recogniser rule in `semantics.rs`, (c) register templates in
//! `data/dialog/templates/`.

use adam_kernel_fst::morphotactics::Number;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnknownAnswerMode {
    General,
    Example,
    Reasoning,
}

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

    /// **v4.3.3** — "Who are you?" / "What kind of system are you?"
    /// addressed to adam itself. Distinct from `AskName` (which only
    /// asks about the addressee's *name*) — this asks about adam's
    /// **identity / nature** as a system.
    ///
    /// **v4.3.4** — extended with `SystemAspect` so the planner can
    /// differentiate the four self-introduction paths the user may
    /// ask about: `General` (name + kind), `Creator` (who made you),
    /// `Birthdate` (when did you appear), `Architecture` (how are
    /// you different from existing models). All four resolve via
    /// the `system_identity` slot family, never from belief.
    ///
    /// Pre-v4.3.3 these phrasings either fell through to Unknown or
    /// got accidentally routed via `AskName` substring matching, so
    /// the dialog answered with the user's stored name instead of
    /// adam's self-introduction. Real-test 2026-04-26 dialog showed
    /// the failure mode (`А, сен кімсің және атың кім?` → `сіздің
    /// атыңыз Мәулет`). Tracked by `intelligence_roadmap.md` Track
    /// B (self/other distinction).
    AskAboutSystem {
        aspect: crate::system_identity::SystemAspect,
    },

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

    /// **v4.6.20** — User-acknowledgement: a long, gracious admission
    /// or observation about adam's state, abilities, or limits.
    /// Surface form: «Мен сенің әлі бәрін білмейтініңді … түсіндім /
    /// білдім / көрдім / байқадым / ұқтым / аңғардым». Distinct from
    /// [`Compliment`] (which is short praise like «жарайсың»):
    /// acknowledgement is a multi-clause statement *about* adam,
    /// often empathetic, often containing closed-class words like
    /// `әлі / бәрін / сенің / сізді`. Pre-v4.6.20 the greedy
    /// noun-hint extractor grabbed an adverb like `әлі` and pulled a
    /// random poetry quote — fixing that misclassification is the
    /// reason this variant exists.
    UserAcknowledgement,

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
        /// v4.2.7: short grounded fact selected from the curated
        /// graph (`SearchGraph`). Unlike `example`, this is not a
        /// corpus quote; it is a direct knowledge statement the
        /// verbalizer can surface without quotation marks.
        #[serde(default)]
        grounded_fact: Option<String>,
        /// v1.9.5: `true` iff the text in `example` was **adapted** from
        /// the retrieved corpus sample (e.g. a city mention was swapped
        /// to the user's session city via `ComposeMode::InSampleCitySwap`).
        /// The planner routes to the `unknown.with_adapted_evidence`
        /// family when this is set, so the user is explicitly informed
        /// that the quote is not byte-identical to the source. Defaults
        /// to `false` — verbatim quotes stay on the v1.8.0 evidence
        /// path.
        #[serde(default)]
        example_adapted: bool,
        /// v2.7: rendered reasoning chain — Kazakh prose describing a
        /// rule-derived RelatedTo fact involving the `noun_hint`
        /// («X пен Y бір-біріне байланысты: екеуі де Z болып табылады»).
        /// Populated by `Conversation::turn` when `derived_facts` are
        /// attached and the noun_hint appears in a derivation's
        /// subject or object. Routes the planner to the
        /// `unknown.with_derived_chain` template family.
        #[serde(default)]
        reasoning_chain: Option<String>,
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
    /// **v4.4.10** — introduction-proposal opener: «танысайық» /
    /// «танысалық» / «танысып алайық» / «танысып алыңыз». User
    /// invites a name exchange, equivalent to «давайте знакомиться».
    /// Routes to a dedicated template family that volunteers
    /// adam's own self-intro and asks the user for theirs. Surfaced
    /// by a 2026-04-28 real-REPL transcript that landed on the
    /// generic refusal `qайта айтыңызшы` pre-v4.4.10.
    IntroProposal,
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

pub fn unknown_answer_mode(raw_tokens: &[String]) -> UnknownAnswerMode {
    if raw_tokens.iter().any(|t| {
        matches!(
            t.as_str(),
            "мысал" | "мысалы" | "дәйек" | "дәйексөз" | "үзінді"
        )
    }) {
        return UnknownAnswerMode::Example;
    }
    if raw_tokens.iter().any(|t| {
        matches!(
            t.as_str(),
            "неге" | "неліктен" | "себеп" | "себебі" | "байланыс" | "байланысты" | "қалайша"
        )
    }) {
        return UnknownAnswerMode::Reasoning;
    }
    UnknownAnswerMode::General
}

pub fn unknown_prefers_quoted_example(raw_tokens: &[String]) -> bool {
    unknown_answer_mode(raw_tokens) == UnknownAnswerMode::Example
}
