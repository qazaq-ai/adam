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

    /// **v4.17.5** — willingness / readiness-to-improve question:
    /// «Сіз жақсаруды үйренуге дайынсыз ба?» / «Жақсырақ болуға
    /// ашықсыз ба?». Live REPL 2026-05-01 turn 19: the user
    /// asked whether adam is willing to learn and improve; the
    /// pre-v4.17.5 pipeline mis-routed to Compliment (because of
    /// the leading «өте жақсы») then to SelfComparison (because
    /// of «жақсырақ»). Neither fits — the user is asking about
    /// adam's stance toward improvement, not asking for a
    /// compliment-response or a comparison.
    ///
    /// The honest answer acknowledges that adam doesn't
    /// self-improve at runtime (its updates come from the author),
    /// but the project IS open to refinement based on user
    /// feedback. Routes to the new `ask_willingness` template
    /// family.
    AskWillingness,

    /// **v4.14.0** — curriculum-content question: «Оқушылар не
    /// оқиды?» / «Оқушылар мектепте физика пәнінен не оқиды?». A
    /// factual question about *what students learn*, not about
    /// adam itself. adam doesn't have school-curriculum-level
    /// content — only domain-summary breadth. The honest answer:
    /// «Бұл сұрағыңыз оқу бағдарламасының мазмұнына қатысты —
    /// менде нақты пәндік дерек жоқ».
    /// Closes the third 2026-05-01 transcript failure pattern that
    /// v4.13.5 didn't reach (`Оқушылар не оқиды?` surfaced
    /// `Оқушы мектеп құрамына кіреді` greedy IsA fact).
    AskCurriculumContent,

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
        /// **v4.12.0** — question shape detected at the top of
        /// `Conversation::turn_with_trace` via
        /// `crate::question_shape::detect`. Routes the planner to
        /// the right template family per `(intent, question_shape)`.
        /// `None` when the input does not look like a question at
        /// all (statement, greeting, etc.) — falls back to the
        /// existing v4.11.x template selection.
        #[serde(default)]
        question_shape: Option<crate::question_shape::QuestionShape>,
        /// **v4.23.0** — `true` iff the input is a temporal-scope
        /// query that adam has no time-series data for: temporal
        /// adverb (`кеше / бүгін / ертең / қазір / бұрын / былтыр /
        /// келесі`) co-occurring with a question word or particle
        /// asking about state-at-a-time. Routes the planner to
        /// `unknown.temporal_no_data` for an honest "I don't track
        /// time-bound state" answer instead of letting the topic
        /// extractor fall through to a general fact about the
        /// non-temporal subject (the post-v4.22.5 «Кеше ауа райы»
        /// → «Ауа тыныс себебі болады» behaviour). Detected in
        /// `interpret_text_with_lexicon` via
        /// `detect_temporal_scope_question`.
        #[serde(default)]
        temporal_scope: bool,
        /// **v4.23.5** — `true` iff the input is a compositional
        /// possessive function question: X-Genitive Y-Possessive
        /// + a function-asking phrase (`не атқарады / не істейді /
        /// қандай қызмет / неге қажет / не үшін керек`). Pattern
        /// surfaced by the 2026-05-01 live-dialog battery on
        /// «Жасушаның ядросы не атқарады?» — the topic extractor
        /// correctly picks `ядро`, but the only world_core fact
        /// available is structural (`Ядро жасуша құрамына кіреді`),
        /// so the response circular: "the nucleus is part of the
        /// cell" doesn't answer "what does the nucleus do?"
        /// Routes the planner to
        /// `unknown.compositional_function.{with_fact,bare}` which
        /// explicitly acknowledges that the available fact is
        /// structural, not functional, and hedges honestly. Detected
        /// in `interpret_text_with_lexicon` via
        /// `detect_compositional_function_question`.
        #[serde(default)]
        compositional_function: bool,
        /// **v4.33.5** — first consumption of a populated SemFrame
        /// field by the response generator. When the input contains
        /// a sentence-level negation pattern («X емес»), the SemFrame
        /// for the noun-headed predicate carries
        /// `Polarity::Negated`. `Conversation::turn` looks up the
        /// frame whose root matches `noun_hint` and copies that
        /// polarity here; the planner then routes the turn to
        /// `unknown.with_negated_topic` instead of asserting a
        /// definition that contradicts the user's claim. Default
        /// `Affirmative` preserves all pre-v4.33.5 routing exactly.
        ///
        /// This is the first SemFrame field that influences the
        /// answer text; v4.31.0–v4.33.0 all populated fields without
        /// consumption. Consumption discipline starts here.
        #[serde(default)]
        noun_hint_polarity: adam_kernel_fst::Polarity,
        /// **v4.34.7** — second SemFrame field consumed by the
        /// response generator. When the input carries a periphrastic-
        /// modality construction («X керек / тиіс / мүмкін» or «-а
        /// ал-» ability), the SemFrame for the lexical predicate
        /// carries `Modality::{Necessity, Possibility, Ability}`.
        /// `Conversation::turn` walks the frame stream looking for
        /// any non-None modality and copies it here; the planner
        /// then routes the turn to `unknown.with_modal_acknowledge`
        /// instead of asserting a generic fact about the noun_hint
        /// (which is typically a different word from the modal-bearer
        /// — e.g. «Кітап оқу керек» has noun_hint=кітап but
        /// modality=Necessity on оқу). Default `None` preserves all
        /// pre-v4.34.7 routing exactly. Polarity-aware routing
        /// (v4.33.5) takes precedence when both fields are set —
        /// negation overrides modality (rare edge case «X V керек
        /// емес»).
        #[serde(default)]
        input_modality: Option<adam_kernel_fst::Modality>,
        /// **v4.36.0** — third SemFrame field consumed by the
        /// response generator. When the input carries a verb in
        /// `Tense::PastEvidential` (the «-{Y}п(ты)» reportative
        /// form: «X-ті естідім» / «X болыпты»), the verb's SemFrame
        /// has `evidence: Some(EvidenceKind::Hearsay)`.
        /// `Conversation::turn` walks the frame stream and copies
        /// the first non-None evidence here; the planner routes
        /// Hearsay-marked Unknowns to `unknown.with_hearsay_hedge`
        /// — a family that hedges the response: "сізге айтылған ба,
        /// нақты білмеймін". Asserting a definition of the topic
        /// noun would either ignore the hearsay framing OR appear
        /// to confirm a story adam can't verify. Default `None`
        /// preserves all pre-v4.36.0 routing exactly.
        #[serde(default)]
        input_evidence: Option<adam_kernel_fst::EvidenceKind>,
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
