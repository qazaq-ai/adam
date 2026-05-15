//! `SystemIdentity` — adam's self-identity record.
//!
//! Distinct from [`crate::belief::BeliefState`]: belief stores facts
//! the dialog has learned about the **user**; system identity is the
//! build-time contract about what adam *is*. The belief layer's
//! invariants (single-active-fact, contradiction logging) do not
//! apply here — adam's name is fixed by the codebase, not by the
//! conversation.
//!
//! **v4.3.4** introduces this module to support the
//! `Intent::AskAboutSystem { aspect }` path: when the user asks
//! "who are you / who made you / when did you appear / how are you
//! different", the planner pulls slot values from this struct and
//! renders the appropriate `ask_about_system.*` template family.
//!
//! Per the v4.3.0 stack policies (Rust-only, graph-first), this
//! identity is also a small reminder: adam is intentionally
//! **not** a transformer / not statistical / not GPU-bound. The
//! `architecture_summary` field surfaces that fact in user-facing
//! Kazakh prose when the user asks how adam differs from existing
//! large language models.

/// Build-time record of adam's self-identity. Default constructs
/// the canonical record (Kazakh-first naming, English technical
/// abbreviation, creator + birthdate from the repository).
///
/// Configurable per-conversation so tests can override individual
/// fields without touching the canonical default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemIdentity {
    /// Short Kazakh name the user normally addresses. Lowercase
    /// canonical form: `адам`.
    pub name: String,
    /// Full English technical name the user may also encounter:
    /// `Agglutinative Reasoning Kernel` (renamed from `Nano Language
    /// Model` in v4.55.5).
    pub full_name: String,
    /// Three-letter abbreviation: `ARK` (renamed from `NLM` in
    /// v4.55.5).
    pub abbreviation: String,
    /// Kazakh kind label rendered in templates: `тілдік модель`.
    pub kind: String,
    /// Creator (per repository AUTHORS): `Баймурзин Даулет
    /// Абузарович`.
    pub creator: String,
    /// ISO-8601 date the project repository was initialised
    /// (`2026-04-07`). Treated as adam's "birthday" in dialog.
    pub birthdate: String,
    /// One-sentence Kazakh summary of how adam differs from
    /// mainstream large language models. Rendered when the user
    /// asks about distinguishing characteristics.
    pub architecture_summary: String,
    /// **v4.6.0** — multi-line Kazakh prose listing adam's current
    /// capabilities (what it can do). Rendered into the
    /// `ask_about_system.capabilities` template family when the
    /// user asks «не істей аласың?». Honest and grounded — every
    /// item must reflect actual code/data in this repository.
    pub capabilities_summary: String,
    /// **v4.6.0** — multi-line Kazakh prose listing adam's
    /// knowledge domains (what it has facts about). Rendered into
    /// the `ask_about_system.knowledge` template family when the
    /// user asks «не білесің?». Mirrors the `world_core/` domain
    /// inventory.
    pub knowledge_summary: String,
    /// **v4.6.0** — multi-line Kazakh prose listing adam's known
    /// limitations (what it does **not** do yet). Rendered into
    /// the `ask_about_system.limitations` template family when the
    /// user asks «нені істей алмайсың?» / «несің әлсіз?». Honest
    /// scope-statement. Critical for trust — adam refuses outside
    /// the envelope rather than fabricating, and the limitations
    /// list makes the envelope explicit.
    pub limitations_summary: String,
    /// **v4.6.5** — multi-line Kazakh prose listing adam's
    /// operational principles (the values it upholds). Rendered
    /// into `ask_about_system.principles` when the user asks
    /// «принциптерің / ұстанымдарың / заңдарың не?». Operationally
    /// grounded, not theological — every principle maps to actual
    /// code/data invariants in this repository (e.g. "respect
    /// humans" → the existing `insult` template family redirects
    /// to «мен адамды құрметтеймін»; "no fabrication" → the
    /// `audit_typed_faithfulness` quality gate; "audit trail" →
    /// `TurnTrace.tool_calls`). Articulating principles makes
    /// adam's value contract discoverable without changing what
    /// the system can actually do — those guarantees are already
    /// safe-by-construction in a deterministic retrieval-only
    /// system.
    pub principles_summary: String,
    /// **v4.6.20** — multi-line Kazakh prose answering "how are you
    /// better/different from other AI models?". Distinct from
    /// `architecture_summary` in tone: that field is a neutral
    /// statement of architectural difference (rules vs.
    /// transformer); this field articulates the *trade-off* —
    /// adam's narrow Kazakh-only competence with strong invariants
    /// (no hallucination, full audit, deterministic) vs. the broad
    /// generative coverage of mainstream LLMs. Honest framing — not
    /// claiming superiority, claiming a different position on the
    /// trade-off curve. Rendered into
    /// `ask_about_system.self_comparison` when the user asks
    /// «басқа модельдерден несімен артықсыз / қалай жақсырақсыз /
    /// айырмашылығың не».
    pub self_comparison_summary: String,
    /// **v4.12.0** — multi-line Kazakh prose answering "what
    /// programming language / stack are you written in?". Distinct
    /// from `architecture_summary` (high-level "rules vs transformer"
    /// framing) and `self_comparison_summary` (trade-off vs other
    /// models): this field answers the literal implementation
    /// question. Mentions `Rust` (the language), the FST / dialog /
    /// retrieval / reasoning crate stack, and the deterministic
    /// rule-based design. Surfaced via the
    /// `ask_about_system.implementation` template family when the
    /// user asks «сіз қандай тілде жазылғансыз?» / «не тілінде
    /// жасалғансың?».
    pub implementation_summary: String,
    /// **v4.13.5** — honest answer for arbitrary verb-capability
    /// questions («Сіз X істей аласыз ба?» where X is any verb
    /// adam can't actually do). Mentions adam's nature as a
    /// language-only system (no actuators, no compute, no internet)
    /// and what it CAN offer instead (description / explanation /
    /// retrieval-grounded info). Surfaced via the
    /// `ask_about_system.generic_capability` template family.
    pub generic_capability_summary: String,
    /// **v4.13.5** — honest answer for multi-topic capability
    /// questions («Сіз математика, физика, химия... білесіз бе?»).
    /// Acknowledges that adam has surface-level understanding
    /// across many subjects but does NOT carry curriculum-level
    /// depth, then redirects to what adam can actually answer at
    /// the domain level. Surfaced via the
    /// `ask_about_system.multi_topic_capability` template family.
    pub multi_topic_capability_summary: String,
}

impl SystemIdentity {
    /// Canonical adam-ARK identity. Use this for production; tests
    /// may override individual fields via direct construction.
    ///
    /// **v4.55.5** — architecture renamed from NLM (Nano Language
    /// Model) to **ARK** (Agglutinative Reasoning Kernel). Reason:
    /// «Language Model» buckets adam with the LLM family; ARK names
    /// the actual architecture — agglutinative morphology + curated
    /// knowledge graph + forward-chaining reasoner + tiny trained
    /// selection weights. «Kernel» (vs «Model») frames adam as a
    /// system-runtime, not a probabilistic estimator.
    pub fn canonical() -> Self {
        Self {
            name: "адам".into(),
            full_name: "Agglutinative Reasoning Kernel".into(),
            abbreviation: "ARK".into(),
            kind: "тілдік модель".into(),
            creator: "Баймурзин Даулет Абузарович".into(),
            birthdate: "2026-04-07".into(),
            architecture_summary: "Мен қолданыстағы үлкен тілдік модельдерден өзгеше \
                архитектурада құрылғанмын — ережелер мен таңбалық ой-тізбекке негізделген, \
                статистикалық генерацияға арналмаған"
                .into(),
            // **v5.30.0** — short, conversational summary.
            // Pre-v5.30.0 this field listed every capability inline
            // (~50-word wall). Live test (2026-05-15): user explicitly
            // asked for «по короче и в общих чертах. А не перечислять
            // все поименно, по фально и все подробности и детали. Это
            // надо делать, когда спросят конкретно и детали.»
            // The detailed lists still live in the per-domain world_core
            // facts — when the user asks specifically («Қазақстанның
            // өзендері?» / «Тарихтан не білесің?») retrieval surfaces
            // the relevant subset via grounded fact citation. This
            // summary is the *high-level handshake*.
            capabilities_summary: "Қазақ тілінде сөйлесе аламын, есіміңіз бен \
                деректеріңізді есте сақтаймын, білетін тақырыптарым бойынша \
                дереккөзіне сүйеніп жауап беремін. Қай тақырыпты білгіңіз келеді?"
                .into(),
            // **v5.30.0** — short, conversational. Same rationale as
            // capabilities_summary above. Detailed per-domain lists
            // surface only when the user asks a specific question.
            knowledge_summary: "Қазақстан жайында (география, тарих, әдебиет, \
                танымал тұлғалар), мектеп пәндері (математика, физика, химия, \
                биология, тарих), бағдарламалау тілі және жалпы білім жайында \
                деректерім бар. Қай тақырыпты нақтырақ қарасаңыз — сұрағыңызды \
                қойыңыз."
                .into(),
            limitations_summary: "Мен әлі мынаны істей алмаймын: тек қазақ тілінде ғана \
                сөйлесемін, орысша немесе ағылшынша түсінбеймін; жаңа мәтін шығармаймын — \
                тек жинақталған деректерден алып айтамын; әңгімеден жаттап алмаймын — әр \
                жаңа сұхбатта мен жадымды жаңадан бастайтын дерек қорынан бастап шығамын; \
                интернетке шықпаймын; сурет, дауыс, видео жоқ — тек жазбаша мәтін; \
                математикалық есептеулер жасамаймын; жаңалықтарды білмеймін; кейбір күрделі \
                ғылыми тақырыптар әлі жоқ; егер сұрағыңыз менің білімімнің шегінен шықса, \
                ойдан жасамаймын — оның орнына білмейтінімді ашық айтамын"
                .into(),
            principles_summary: "Менің ұстанымдарым: адамды құрметтеймін, ешкімді кемсітпеймін \
                және қорламаймын; жалған сөйлемеймін — білмейтінімді ашық айтамын, ойдан \
                ештеңе шығармаймын; зорлық-зомбылыққа, өшпенділікке немесе кемсітушілікке \
                шақырмаймын; жеке өмір құпиясын құрметтеймін — пайдаланушының деректерін \
                сақтаймын, бөтенге бермеймін; заңсыз әрекетке, зиянды нұсқауларға көмектеспеймін; \
                әрбір жауабымды дереккөзіне жалғап аудит ете аламын; қазақ тілі мен мәдениетіне \
                адал болып, оны құрметтеп таратуға тырысамын; өзімнің шегімді біліп, шегімнен \
                шықпаймын — қажет кезінде сыпайы түрде бас тартамын"
                .into(),
            implementation_summary: "Мен `Rust` бағдарламалау тілінде жазылғанмын. Менің \
                бастапқы кодым ашық, ол бірнеше Rust-сандықтарына (`adam-kernel`, \
                `adam-tokenizer`, `adam-dialog`, `adam-reasoning`, `adam-retrieval`, \
                `adam-corpus`) бөлінген. Архитектурам толығымен ережеге негізделген: \
                морфологиялық FST, шаблондармен жұмыс істейтін диалог қозғалтқышы, \
                фактілер графы, морфема бойынша корпусты іздеу. Статистикалық генерация \
                жоқ — әр жауабым нақты дереккөзге сүйенеді. Мен `macOS` пен `Linux` \
                жүйелерінде жұмыс істеймін, интернетке шықпаймын"
                .into(),
            generic_capability_summary: "Жоқ, ондай әрекетті өзім орындай алмаймын. Мен — \
                тілдік модельмін: тек қазақ тілінде сөйлесе аламын, фактілерді айтып бере \
                аламын, түсініктер мен анықтамаларды бере аламын. Бағдарлама жазу, есептеу \
                жүргізу, интернетке шығу немесе кез келген физикалық әрекет менің мүмкіндігімде \
                жоқ. Бірақ ол әрекеттің мағынасын немесе оған қатысты ұғымдарды түсіндіруге \
                көмектесе аламын"
                .into(),
            multi_topic_capability_summary: "Аталған пәндер бойынша негізгі түсініктерім бар: \
                әр пәннің не екенін, негізгі ұғымдарын, маңызды атауларын айтып бере аламын. \
                Бірақ мектеп бағдарламасының толық мазмұны менде жоқ — нақты тарауларды, \
                есептерді немесе жаттығуларды толық біле бермеймін. Қай пән жайлы білгіңіз \
                келсе, нақтылап сұрасаңыз көбірек көмектесе аламын"
                .into(),
            self_comparison_summary: "Мен басқа жасанды интеллект модельдерінен жақсырақ деп \
                айта алмаймын — басқашамын. Үлкен тілдік модельдер (GPT, Llama және басқалары) \
                көп тілде, кез келген тақырыпта еркін мәтін жазады, бірақ кейде жоқ нәрсені \
                ойдан шығарып, дереккөзін көрсете алмайды. Мен керісінше: тек қазақ тілінде, \
                тек өзім нақты білетін шектеулі тақырыпта жұмыс істеймін; бірақ айтқан \
                сөзімнің әрбір дерегін `world_core` немесе мәтіндік корпустағы нақты \
                дереккөзге жалғай аламын; ойдан ештеңе шығармаймын — білмейтінімді ашық \
                айтамын; шешімдер ережелер арқылы жасалады, статистикалық генерация жоқ, \
                сондықтан жауабым қайталанатын әрі тексеруге келеді; жұмысыма үлкен есептеу \
                қуаты керек емес — қарапайым ноутбукте бір секундтың бөлшегінде жауап беремін. \
                Қысқасы: ауқымы тар, бірақ сенімді — қазақ тіліндегі шектеулі салаға арналған \
                адал құрал"
                .into(),
        }
    }

    /// Render slots a planner can inject into `ask_about_system.*`
    /// templates. Slots are namespaced with the `system_` prefix to
    /// avoid colliding with the user-profile slots (`name`, `age`,
    /// `city`, `occupation`, `name_id`, `city_id`, `geo_kind`).
    ///
    /// Returns 7 fixed entries: `system_name`, `system_full_name`,
    /// `system_abbreviation`, `system_kind`, `system_creator`,
    /// `system_birthdate`, `system_architecture`.
    pub fn template_slots(&self) -> Vec<(String, String)> {
        vec![
            ("system_name".into(), self.name.clone()),
            ("system_full_name".into(), self.full_name.clone()),
            ("system_abbreviation".into(), self.abbreviation.clone()),
            ("system_kind".into(), self.kind.clone()),
            ("system_creator".into(), self.creator.clone()),
            ("system_birthdate".into(), self.birthdate.clone()),
            (
                "system_architecture".into(),
                self.architecture_summary.clone(),
            ),
            // v4.6.0 — capabilities / knowledge / limitations
            // self-introspection slots. Mirror the
            // architecture_summary slot's pattern: long Kazakh
            // prose suitable for direct interpolation.
            (
                "system_capabilities".into(),
                self.capabilities_summary.clone(),
            ),
            ("system_knowledge".into(), self.knowledge_summary.clone()),
            (
                "system_limitations".into(),
                self.limitations_summary.clone(),
            ),
            // v4.6.5 — operational principles slot.
            ("system_principles".into(), self.principles_summary.clone()),
            // v4.6.20 — self-comparison slot for «басқа модельдерден
            // несімен артықсыз?» style questions.
            (
                "system_self_comparison".into(),
                self.self_comparison_summary.clone(),
            ),
            // v4.13.5 — generic-capability + multi-topic-capability
            // slots. Honest fallback responses for arbitrary
            // verb-capability questions and multi-subject knowledge
            // queries the v4.12.0 specific detectors don't catch.
            (
                "system_generic_capability".into(),
                self.generic_capability_summary.clone(),
            ),
            (
                "system_multi_topic_capability".into(),
                self.multi_topic_capability_summary.clone(),
            ),
            // v4.12.0 — implementation slot. Renders the
            // `implementation_summary` field for the
            // `ask_about_system.implementation` template family.
            (
                "system_implementation".into(),
                self.implementation_summary.clone(),
            ),
        ]
    }
}

impl Default for SystemIdentity {
    fn default() -> Self {
        Self::canonical()
    }
}

/// Aspect the user is asking about when `Intent::AskAboutSystem`
/// fires. Drives template selection in the planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SystemAspect {
    /// General self-introduction: name + kind + abbreviation.
    /// Surface form: `сен кімсің?` / `сіз қандай моделсіз?`.
    General,
    /// "Who made you?" — surface form: `сені кім жасады?`.
    Creator,
    /// "When did you appear?" — surface form: `қашан пайда болдың?`,
    /// `туған күнің қашан?`.
    Birthdate,
    /// "How are you different from existing models?" — surface
    /// form: `ерекшелігің не?`, `неге басқашасың?`.
    Architecture,
    /// **v4.6.0** — "What can you do?" — surface form:
    /// `не істей аласың?` / `не істей аласыз?` / `мүмкіндіктерің не?`.
    /// Renders the `capabilities_summary` field as a structured
    /// list of what adam can actually do (KZ morphology, slot
    /// recall, knowledge-domain answers, contradiction handling,
    /// out-of-scope refusal, audit trail).
    Capabilities,
    /// **v4.6.0** — "What do you know?" — surface form:
    /// `не білесің?` / `не білесіз?` / `қандай тақырыптар жайлы
    /// дерегің бар?`. Renders the `knowledge_summary` field — a
    /// digest of the world_core domain inventory.
    Knowledge,
    /// **v4.6.0** — "What can't you do yet?" — surface form:
    /// `нені істей алмайсың?` / `несің әлсіз?` / `шектеулерің
    /// қандай?`. Renders the `limitations_summary` field. Critical
    /// for trust: adam refuses outside the envelope rather than
    /// fabricating, so an explicit limitations list makes the
    /// envelope discoverable.
    Limitations,
    /// **v4.6.5** — "What are your principles?" — surface form:
    /// `принциптерің қандай?` / `ұстанымдарың не?` / `заңдарың
    /// қандай?` / `ережелерің не?`. Renders the
    /// `principles_summary` field. Operational values adam
    /// upholds: respect humans, no fabrication, no incitement,
    /// privacy, no illegal-act assistance, audit trail,
    /// Kazakh-cultural respect, scope discipline. Articulation
    /// layer — the underlying guarantees are already
    /// safe-by-construction in this deterministic retrieval-only
    /// system; this aspect makes the value contract discoverable.
    Principles,
    /// **v4.6.20** — "How are you better/different from other AI
    /// models?" — surface form: «басқа модельдерден несімен
    /// артықсыз?», «қалай жақсырақсыз?», «ерекшелігің не?» (when
    /// scoped to comparison with other models). Renders the
    /// `self_comparison_summary` field. Honest framing of the
    /// trade-off: narrow Kazakh-only scope with strong invariants
    /// (no hallucination, full audit, deterministic) vs. the broad
    /// generative coverage of mainstream LLMs. Distinct from
    /// Architecture: Architecture states *what* adam is built from
    /// (rules, not transformer); SelfComparison states *why* that
    /// matters and what the trade-off looks like.
    SelfComparison,
    /// **v4.12.0** — "What programming language are you written in?"
    /// — surface form: «сіз қандай тілде жазылғансыз?», «не тілінде
    /// жасалғансың?», «қандай бағдарламалау тілінде жазылған?».
    /// Renders the `implementation_summary` field — adam's truthful
    /// statement about its implementation stack (`Rust`, FST,
    /// rule-based pipeline, deterministic). Distinct from
    /// `Architecture` (high-level "rules vs transformer" framing) and
    /// `SelfComparison` (trade-off vs other models): `Implementation`
    /// answers the literal "what is the system built with?" question.
    /// Closes the v4.11.7 known gap where the question grounded on
    /// the generic `бағдарламалау тілі IsA формалды тіл` fact instead
    /// of the self-knowledge claim `adam writtenIn rust`.
    Implementation,
    /// **v4.13.5** — generic verb-capability question: «Сіз X істей
    /// аласыз ба?» / «X жасай аласың ба?» where X is any verb other
    /// than language-related ones (those route to `Capabilities`).
    /// Surface forms: `аласың/аласыз ба/ма`, `ала ма?`, `алады ма`.
    /// 2026-05-01 live REPL: «Сіз оны бағдарламалай аласыз ба, әлі
    /// жоқ па?» pre-v4.13.5 surfaced poetry on `әлі`. The honest
    /// answer: adam is a language model, can't perform actions, only
    /// describe them. Renders the `generic_capability_summary`
    /// field. Distinct from `Capabilities` (which lists what adam
    /// CAN do — Kazakh dialog, slot recall, knowledge queries) —
    /// `GenericCapability` answers "no, I can't do that" for any
    /// arbitrary verb that falls outside the closed capability set.
    GenericCapability,
    /// **v4.13.5** — multi-topic capability question: «Сіз
    /// математика, физика, химия... білесіз бе?» where the user
    /// lists 3+ subjects and asks if adam knows them. Surface
    /// detection: 2+ commas + `және` + `білесің/білесіз`. The
    /// honest answer: adam has surface-level understanding across
    /// these subjects (the `knowledge_summary` covers domain-level
    /// breadth) but no school-curriculum depth. Renders the
    /// `multi_topic_capability_summary` field. Distinct from
    /// `Knowledge` (one-off "what do you know?") — this aspect
    /// specifically handles list-form questions where the user
    /// expects per-subject confirmation.
    MultiTopicCapability,
    /// **v4.18.5** — composite identity + capabilities question.
    /// Pattern: «кім екеніңіз және не істей алатыныңыз туралы
    /// айтыңыз» — user asks two aspects in one turn (who you are
    /// AND what you can do). Pre-v4.18.5 the detector would pick
    /// one aspect (whichever fired first) and miss the other.
    /// 2026-05-01 live REPL turn 4 surfaced this exact pattern;
    /// adam answered only the first half.
    ///
    /// The composite template renders both `system_kind` and
    /// `system_capabilities` in sequence. Routed via a dedicated
    /// detector that fires BEFORE individual-aspect detectors so
    /// the composite check has priority.
    IntroAndCapabilities,
}

impl SystemAspect {
    /// Template-family suffix the planner appends to `ask_about_system`
    /// to pick the right family for this aspect.
    /// `General → ""`, `Creator → ".creator"`, …
    pub fn template_key_suffix(&self) -> &'static str {
        match self {
            SystemAspect::General => "",
            SystemAspect::Creator => ".creator",
            SystemAspect::Birthdate => ".birthdate",
            SystemAspect::Architecture => ".architecture",
            SystemAspect::Capabilities => ".capabilities",
            SystemAspect::Knowledge => ".knowledge",
            SystemAspect::Limitations => ".limitations",
            SystemAspect::Principles => ".principles",
            SystemAspect::SelfComparison => ".self_comparison",
            SystemAspect::Implementation => ".implementation",
            SystemAspect::GenericCapability => ".generic_capability",
            SystemAspect::MultiTopicCapability => ".multi_topic_capability",
            SystemAspect::IntroAndCapabilities => ".intro_and_capabilities",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_identity_carries_all_required_fields() {
        let id = SystemIdentity::canonical();
        assert_eq!(id.name, "адам");
        assert_eq!(id.full_name, "Agglutinative Reasoning Kernel");
        assert_eq!(id.abbreviation, "ARK");
        assert_eq!(id.creator, "Баймурзин Даулет Абузарович");
        assert_eq!(id.birthdate, "2026-04-07");
        assert!(id.kind.contains("модель"));
        assert!(id.architecture_summary.contains("ережелер"));
    }

    #[test]
    fn template_slots_use_system_prefix() {
        let id = SystemIdentity::canonical();
        let slots: std::collections::HashMap<_, _> = id.template_slots().into_iter().collect();
        for key in slots.keys() {
            assert!(
                key.starts_with("system_"),
                "all system identity slots must be `system_`-prefixed; got {key}"
            );
        }
        assert_eq!(slots.get("system_name").map(String::as_str), Some("адам"));
        assert_eq!(
            slots.get("system_creator").map(String::as_str),
            Some("Баймурзин Даулет Абузарович")
        );
        assert_eq!(
            slots.get("system_birthdate").map(String::as_str),
            Some("2026-04-07")
        );
    }

    #[test]
    fn aspect_template_key_suffix_is_deterministic() {
        assert_eq!(SystemAspect::General.template_key_suffix(), "");
        assert_eq!(SystemAspect::Creator.template_key_suffix(), ".creator");
        assert_eq!(SystemAspect::Birthdate.template_key_suffix(), ".birthdate");
        assert_eq!(
            SystemAspect::Architecture.template_key_suffix(),
            ".architecture"
        );
        assert_eq!(
            SystemAspect::Capabilities.template_key_suffix(),
            ".capabilities"
        );
        assert_eq!(SystemAspect::Knowledge.template_key_suffix(), ".knowledge");
        assert_eq!(
            SystemAspect::Limitations.template_key_suffix(),
            ".limitations"
        );
        assert_eq!(
            SystemAspect::Principles.template_key_suffix(),
            ".principles"
        );
        assert_eq!(
            SystemAspect::SelfComparison.template_key_suffix(),
            ".self_comparison"
        );
    }

    /// **v4.6.0** — verify the new self-awareness fields are
    /// non-empty + Kazakh-only in canonical identity. Each must
    /// be substantial prose (not a placeholder), since they're
    /// rendered directly into user-facing replies.
    #[test]
    fn canonical_identity_has_substantial_self_awareness_summaries() {
        let id = SystemIdentity::canonical();
        for (label, value) in [
            ("capabilities", &id.capabilities_summary),
            ("knowledge", &id.knowledge_summary),
            ("limitations", &id.limitations_summary),
            ("principles", &id.principles_summary),
        ] {
            assert!(
                value.len() >= 100,
                "{label}_summary must be substantial Kazakh prose; got len={}",
                value.len()
            );
            // Sanity: must contain real Kazakh content (Cyrillic).
            assert!(
                value
                    .chars()
                    .any(|c| ('а'..='я').contains(&c) || c == 'қ' || c == 'ң'),
                "{label}_summary must be Kazakh text"
            );
        }
        // The canonical identity should mention what adam refuses
        // (limitations) — load-bearing for the trust contract.
        assert!(
            id.limitations_summary.contains("білмеймін")
                || id.limitations_summary.contains("істей алмаймын"),
            "limitations must explicitly state what adam refuses to do"
        );
    }

    #[test]
    fn default_returns_canonical() {
        assert_eq!(SystemIdentity::default(), SystemIdentity::canonical());
    }
}
