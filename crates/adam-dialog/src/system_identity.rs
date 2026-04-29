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
    /// `Nano Language Model`.
    pub full_name: String,
    /// Three-letter abbreviation: `NLM`.
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
}

impl SystemIdentity {
    /// Canonical adam-NLM identity. Use this for production; tests
    /// may override individual fields via direct construction.
    pub fn canonical() -> Self {
        Self {
            name: "адам".into(),
            full_name: "Nano Language Model".into(),
            abbreviation: "NLM".into(),
            kind: "тілдік модель".into(),
            creator: "Баймурзин Даулет Абузарович".into(),
            birthdate: "2026-04-07".into(),
            architecture_summary: "Мен қолданыстағы үлкен тілдік модельдерден өзгеше \
                архитектурада құрылғанмын — ережелер мен таңбалық ой-тізбекке негізделген, \
                статистикалық генерацияға арналмаған"
                .into(),
            capabilities_summary: "Қазақ тілінде сөйлесе аламын; есіміңізді, жасыңызды, \
                қалаңызды және мамандығыңызды есте сақтап, қажет болса айтып бере аламын; \
                Қазақстанның географиясы (17 облыс, әкімшілік орталықтар, өзендер, көлдер, \
                таулар, шөлдер) туралы білемін; қазақ әдебиеті мен танымал қазақстандықтар \
                жайлы дерек таба аламын; сіздің сөзіңіздегі қайшылықтарды байқап, \
                нақтылауды сұрай аламын; білмейтін нәрселерден бас тартып, ойдан шығармаймын; \
                әрбір жауабымның дереккөзін аудитке көрсете аламын"
                .into(),
            knowledge_summary: "Менің білімім негізінен Қазақстан туралы: 17 облыс пен \
                әкімшілік орталықтары, ірі қалалар, тұрғын саны; ірі өзендер (Ертіс, \
                Сырдария, Іле, Жайық, Есіл, Тобыл, Шу, Қаратал, Талас); көлдер мен теңіздер \
                (Балқаш, Каспий, Арал, Зайсан, Алакөл, Тенгіз, Маркакөл); таулар (Алтай, \
                Тянь-Шань, Жетісу Алатауы, Қаратау, Ұлытау, Хан Тәңірі); шөлдер (Бетпақдала, \
                Қызылқұм, Үстірт, Мойынқұм); көрікті жерлер (Бурабай, Шарын каньоны). Қазақ \
                әдебиеті: Әуезов, Сейфуллин, Мүсірепов, Жансүгіров, Жұмабаев, Махамбет, \
                Шәкәрім, Алтынсарин. Танымал қазақстандықтар: Назарбаев, Тоқаев, Қонаев, \
                Бөкейхан, Абылай хан, Сәтбаев, Уәлиханов, Молдағұлова. Жалпы білім: \
                жануарлар, өсімдіктер, дене мүшелері, киім, тағам, түстер, кәсіптер, \
                құрал-сайман, көлік, ауа-райы, аспан денелері, өлшемдер, ыдыс-аяқ, \
                туыстық, музыка, мақал-мәтелдер"
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
        assert_eq!(id.full_name, "Nano Language Model");
        assert_eq!(id.abbreviation, "NLM");
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
