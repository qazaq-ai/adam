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
    }

    #[test]
    fn default_returns_canonical() {
        assert_eq!(SystemIdentity::default(), SystemIdentity::canonical());
    }
}
