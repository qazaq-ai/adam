// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `ProcedureIR` — **v6.8.5 / L4.6 industrial-pilot foundation**.
//!
//! Typed standard-operating-procedure record for the industrial
//! knowledge assistant product line (ССГПО / Allur / KIA — охрана
//! труда / тех-карты / тренинг / maintenance, read-only voice in
//! цех).  The school-tutor product line keeps using [`Frame`] for
//! curriculum facts; ProcedureIR is the symmetric typed record for
//! the SOP path.
//!
//! ## Why a separate type from `Frame`
//!
//! A `Frame` is a single assertion (`agent`, `predicate`, `object`)
//! — well-suited to «X is Y», «X did Y when», «X has property Y».
//! An SOP is a *sequence* of conditional actions with hazards,
//! actors, and gate conditions — none of that fits cleanly into a
//! single-tuple shape.  Forcing it into `Frame` would either lose
//! the ordering (the most safety-relevant property of a procedure)
//! or smear one logical procedure across N facts and lose the
//! invariant that all steps must be retrieved together.
//!
//! Codex's L4.5 follow-up explicitly named this layer:
//!
//! > **`ProcedureIR` for SOP**: steps + prerequisite + hazards +
//! > role/authorization + confirmation gates.
//!
//! This module ships exactly that, plus a `ProcedureSource` block
//! that makes regulatory currency a first-class typed property —
//! not a comment — so a CI check can flag fixtures whose source
//! version pre-dates a configurable freshness window.
//!
//! ## Stage scope (v6.8.5 foundation)
//!
//! 1. The type itself, with `serde` round-trip + a few invariant
//!    checks.
//! 2. A small set of unit tests asserting the shape compiles and
//!    rejects obviously-broken inputs (empty step list, source
//!    with no version date, etc.).
//! 3. JSONL loader is in [`ProcedureIR::from_jsonl_line`] so the
//!    `crates/adam-dialog` retrieval handler (next commit) can
//!    consume fixtures from `data/procedures/*.jsonl`.
//!
//! Retrieval routing is the *next* commit — this one is pure type
//! foundation.

use serde::{Deserialize, Serialize};

/// **Typed standard-operating-procedure record.**
///
/// Every field carries Kazakh-first content; Russian fields are
/// optional because (a) the pilot voice surface is Kazakh, and
/// (b) Russian copy is often a *translation* derived from the
/// Kazakh master — recording the translation but not requiring
/// it lets us add Russian on-demand without a schema migration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcedureIR {
    /// Stable identifier — domain + sequential.  `kk_labor_001`,
    /// `metallurgy_loto_003`, etc.  Used by the retrieval handler
    /// as the canonical lookup key.
    pub id: String,
    /// Short Kazakh title (≤ 80 chars by convention; not enforced
    /// because regulations sometimes carry long official names).
    pub title_kk: String,
    /// Optional Russian title.  Often a verbatim translation of
    /// the Kazakh title; recording it lets the assistant answer
    /// «Что такое наряд-допуск?» without a re-render.
    pub title_ru: Option<String>,
    /// Optional English title.  Added in v6.8.27 for foreign-
    /// capital pilot contexts (Allur / KIA / new plants with
    /// Chinese / Turkish / Arab capital).  Defaults to `None`
    /// — the trilingual lift is gradual.
    #[serde(default)]
    pub title_en: Option<String>,
    /// **v6.8.27.**  Kazakh-language search aliases — synonyms,
    /// abbreviations, or alternate phrasings the user might type
    /// instead of `title_kk`.  E.g. for «Жұмыс орнын дайындау» a
    /// useful alias is «жұмыс орнының дайындығы» (genitive
    /// shape).  Empty default keeps existing JSONL records
    /// schema-compatible.
    #[serde(default)]
    pub aliases_kk: Vec<String>,
    /// **v6.8.27.**  Russian-language search aliases — what the
    /// SOP is known as in Russian.  Critical for ССГПО / KIA
    /// pilot where Russian is the operational language even
    /// when the canonical SOP is Kazakh-first.  E.g. «СИЗ»,
    /// «наряд-допуск», «инструктаж».
    #[serde(default)]
    pub aliases_ru: Vec<String>,
    /// **v6.8.27.**  English-language search aliases — the
    /// industrial-pilot ↔ foreign-management bridge.  Typical
    /// entries: «PPE», «work permit», «LOTO», «safety briefing».
    #[serde(default)]
    pub aliases_en: Vec<String>,
    /// Domain bucket — used by the retrieval handler to scope a
    /// query («Какая процедура для X?») to a candidate set.
    pub domain: ProcedureDomain,
    /// Free-form descriptor of *who this applies to*.  Kazakh-
    /// first.  Typically a job category («заводта жұмыс істейтін
    /// барлық қызметкерлер») or a hazard class.
    pub applies_to: Vec<String>,
    /// Things that MUST be true before step 1 is allowed to
    /// begin — typically training certifications, equipment
    /// checks, naryad-dopusk paperwork.
    pub prerequisites: Vec<String>,
    /// Ordered sequence of actions.  The sequence is significant:
    /// re-ordering steps is a safety violation in the real
    /// regulation, so the type preserves order.
    pub steps: Vec<ProcedureStep>,
    /// Hazards introduced by this procedure (and the mitigation
    /// the regulation requires for each).  Empty list means the
    /// authoring reviewer asserted there are none — distinct
    /// from "we didn't bother to fill it in".
    pub hazards: Vec<Hazard>,
    /// Roles authorised to *perform* / *approve* this procedure.
    /// Kazakh role names so they match the actual organisational
    /// chart («цех бастығы», «энергетик», «мастер»).
    pub authorization: Vec<String>,
    /// Confirmation gates — discrete checkpoints where the
    /// procedure must pause until an external condition is
    /// observed/signed.  Example: «наряд-допуск рәсімделуі
    /// тиіс», «газ концентрациясы өлшенуі тиіс».
    pub confirmation_gates: Vec<String>,
    /// Provenance + currency metadata.  Source carries the
    /// regulation version date so a CI lint can flag procedures
    /// whose underlying regulation has been superseded.
    pub source: ProcedureSource,
}

/// One ordered step of a procedure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcedureStep {
    /// 1-based step number.  Stored explicitly so a step can be
    /// inserted via JSONL edit without re-numbering the rest
    /// (loader checks for monotonic-strictly-increasing).
    pub sequence: u32,
    /// Imperative Kazakh action.  Voice assistant reads this
    /// aloud; keep it ≤ ~120 chars for natural pacing.
    pub action_kk: String,
    /// Optional explicit actor when it differs from the parent
    /// procedure's `authorization` (e.g. a step performed by a
    /// subordinate while the cell stays under supervisor's
    /// authorization).
    pub actor: Option<String>,
    /// Optional precondition specific to this step.  Different
    /// from parent `prerequisites` (those gate the whole
    /// procedure); these gate this single step.
    pub condition: Option<String>,
    /// Optional evidence required to consider the step complete
    /// — typically a signed checklist line, a meter reading, a
    /// supervisor sign-off.
    pub evidence: Option<String>,
}

/// One hazard + its mitigation, both in Kazakh.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hazard {
    /// Short noun phrase: «жоғары кернеу», «газ улануы», «биіктен
    /// құлау».
    pub kind_kk: String,
    /// Mitigation as the regulation prescribes it — typically a
    /// СИЗ requirement, isolation step, or monitoring action.
    pub mitigation_kk: String,
}

/// Provenance + currency metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcedureSource {
    /// Human-readable regulation name in Kazakh («Қазақстан
    /// Республикасының Еңбек кодексі», «СТ РК ...»).
    pub regulation_kk: String,
    /// Compact regulation identifier («414-V», «ГОСТ 12.0.004-2015»,
    /// «СТ РК 1356-2005»).  Used for de-duplication when multiple
    /// procedures cite the same article.
    pub regulation_id: String,
    /// Optional article / section pointer («182-бап»,
    /// «5.3-бөлім»).
    pub article: Option<String>,
    /// ISO date (`YYYY-MM-DD`) of the **regulation version** this
    /// procedure was derived from — when that version took
    /// effect.  Distinct from `retrieved_at`.  The CI freshness
    /// lint runs against this field.
    pub version_date: String,
    /// ISO date (`YYYY-MM-DD`) the curator ingested the
    /// procedure from the source.  Captures «when we last
    /// looked at it», independent of when the regulation
    /// itself was last amended.
    pub retrieved_at: String,
    /// Optional public URL — adilet.zan.kz, zakon.kz, etc.
    /// Kept optional because internal pilot fixtures may cite
    /// company SOPs that lack a public URL.
    pub url: Option<String>,
}

/// Coarse-grained domain bucket for retrieval scoping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcedureDomain {
    /// Охрана труда — general occupational safety (briefings,
    /// PPE, work permits, incident reporting).
    OkhranaTruda,
    /// Industrial safety in metallurgy / mining (LOTO, gas
    /// monitoring, hot work, confined space).
    Metallurgy,
    /// Automotive assembly safety + maintenance procedures
    /// (Allur / KIA pilot lines).
    Automotive,
    /// Construction-site procedures (height work, scaffolding,
    /// excavation).
    Construction,
    /// Generic / cross-domain procedures that don't fit the
    /// industry buckets above.
    Other,
}

impl ProcedureIR {
    /// Parse a single JSONL line into a `ProcedureIR`.  Returns
    /// `Err` on malformed JSON OR on an invariant violation
    /// (empty steps, non-monotonic sequence, blank version_date).
    pub fn from_jsonl_line(line: &str) -> Result<Self, ProcedureParseError> {
        let parsed: Self = serde_json::from_str(line).map_err(ProcedureParseError::Json)?;
        parsed.check_invariants()?;
        Ok(parsed)
    }

    /// Run structural invariants.  Called by `from_jsonl_line`;
    /// also exposed so unit tests can assert hand-built
    /// instances pass.
    pub fn check_invariants(&self) -> Result<(), ProcedureParseError> {
        if self.id.trim().is_empty() {
            return Err(ProcedureParseError::EmptyField("id"));
        }
        if self.title_kk.trim().is_empty() {
            return Err(ProcedureParseError::EmptyField("title_kk"));
        }
        if self.steps.is_empty() {
            return Err(ProcedureParseError::EmptyField("steps"));
        }
        // Step sequence must be strictly increasing from 1.
        for (idx, step) in self.steps.iter().enumerate() {
            let expected = idx as u32 + 1;
            if step.sequence != expected {
                return Err(ProcedureParseError::NonMonotonicSteps {
                    expected,
                    found: step.sequence,
                });
            }
            if step.action_kk.trim().is_empty() {
                return Err(ProcedureParseError::EmptyField("step.action_kk"));
            }
        }
        // Source must carry a version date.
        if self.source.version_date.trim().is_empty() {
            return Err(ProcedureParseError::EmptyField("source.version_date"));
        }
        if self.source.retrieved_at.trim().is_empty() {
            return Err(ProcedureParseError::EmptyField("source.retrieved_at"));
        }
        // ISO date format YYYY-MM-DD.
        Self::check_iso_date(&self.source.version_date, "source.version_date")?;
        Self::check_iso_date(&self.source.retrieved_at, "source.retrieved_at")?;
        Ok(())
    }

    fn check_iso_date(value: &str, field: &'static str) -> Result<(), ProcedureParseError> {
        let bytes = value.as_bytes();
        let ok = bytes.len() == 10
            && bytes[4] == b'-'
            && bytes[7] == b'-'
            && bytes[..4].iter().all(|b| b.is_ascii_digit())
            && bytes[5..7].iter().all(|b| b.is_ascii_digit())
            && bytes[8..].iter().all(|b| b.is_ascii_digit());
        if ok {
            Ok(())
        } else {
            Err(ProcedureParseError::MalformedDate { field })
        }
    }

    /// Currency check — returns `true` when the source's
    /// `version_date` is within `max_age_days` of `today`
    /// (`YYYY-MM-DD`).  Returns `false` when the source is older
    /// than the threshold.  Returns an `Err` when either date is
    /// malformed (defensive — callers should have run
    /// `check_invariants` first).
    ///
    /// Comparison uses plain calendar arithmetic on the
    /// `YYYY-MM-DD` strings — sufficient since both sides are
    /// guaranteed ISO-format after `check_invariants`.
    pub fn is_within_freshness_window(
        &self,
        today: &str,
        max_age_days: i64,
    ) -> Result<bool, ProcedureParseError> {
        let source_days = days_since_epoch(&self.source.version_date)?;
        let today_days = days_since_epoch(today)?;
        Ok((today_days - source_days) <= max_age_days)
    }
}

/// Parse errors surfaced by [`ProcedureIR::from_jsonl_line`] and
/// [`ProcedureIR::check_invariants`].
#[derive(Debug)]
pub enum ProcedureParseError {
    /// `serde_json` failed to parse the line.
    Json(serde_json::Error),
    /// A required field was empty / whitespace-only.
    EmptyField(&'static str),
    /// Step sequence must be `1, 2, 3, …` strictly increasing.
    NonMonotonicSteps { expected: u32, found: u32 },
    /// A date field doesn't match `YYYY-MM-DD`.
    MalformedDate { field: &'static str },
}

impl std::fmt::Display for ProcedureParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(e) => write!(f, "JSON parse error: {e}"),
            Self::EmptyField(name) => write!(f, "required field is empty: {name}"),
            Self::NonMonotonicSteps { expected, found } => write!(
                f,
                "step sequence must be strictly increasing from 1 \
                 (expected {expected}, found {found})",
            ),
            Self::MalformedDate { field } => {
                write!(f, "date field must be YYYY-MM-DD: {field}")
            }
        }
    }
}

impl std::error::Error for ProcedureParseError {}

/// Convert a `YYYY-MM-DD` string to a day-count since the proleptic
/// Gregorian epoch (1970-01-01).  Defensive: assumes ASCII digits.
fn days_since_epoch(iso: &str) -> Result<i64, ProcedureParseError> {
    if iso.len() != 10 {
        return Err(ProcedureParseError::MalformedDate {
            field: "<freshness-input>",
        });
    }
    let y: i64 = iso[..4]
        .parse()
        .map_err(|_| ProcedureParseError::MalformedDate {
            field: "<freshness-input>",
        })?;
    let m: i64 = iso[5..7]
        .parse()
        .map_err(|_| ProcedureParseError::MalformedDate {
            field: "<freshness-input>",
        })?;
    let d: i64 = iso[8..]
        .parse()
        .map_err(|_| ProcedureParseError::MalformedDate {
            field: "<freshness-input>",
        })?;
    // Howard Hinnant's civil_from_days inverse — see
    // https://howardhinnant.github.io/date_algorithms.html#days_from_civil
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Ok(era * 146097 + doe - 719468)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_minimal() -> ProcedureIR {
        ProcedureIR {
            id: "kk_labor_001".into(),
            title_kk: "Бастапқы инструктаж".into(),
            title_ru: Some("Первичный инструктаж".into()),
            title_en: None,
            aliases_kk: Vec::new(),
            aliases_ru: Vec::new(),
            aliases_en: Vec::new(),
            domain: ProcedureDomain::OkhranaTruda,
            applies_to: vec!["барлық жаңа қызметкерлер".into()],
            prerequisites: vec!["жұмысқа қабылдау бұйрығы шығарылуы тиіс".into()],
            steps: vec![ProcedureStep {
                sequence: 1,
                action_kk: "Кадр қызметі жаңа қызметкерді қабылдау бұйрығын ресімдейді.".into(),
                actor: Some("кадр қызметі".into()),
                condition: None,
                evidence: Some("қол қойылған бұйрық".into()),
            }],
            hazards: vec![],
            authorization: vec!["еңбекті қорғау инженері".into()],
            confirmation_gates: vec!["журналға қол қойылуы тиіс".into()],
            source: ProcedureSource {
                regulation_kk: "Қазақстан Республикасының Еңбек кодексі".into(),
                regulation_id: "414-V".into(),
                article: Some("184-бап".into()),
                version_date: "2024-04-15".into(),
                retrieved_at: "2026-06-22".into(),
                url: Some("https://adilet.zan.kz/kaz/docs/K1500000414".into()),
            },
        }
    }

    #[test]
    fn invariants_accept_minimal_sample() {
        sample_minimal()
            .check_invariants()
            .expect("minimal sample should pass invariants");
    }

    #[test]
    fn invariants_reject_empty_steps() {
        let mut p = sample_minimal();
        p.steps.clear();
        let err = p
            .check_invariants()
            .expect_err("empty steps must be rejected");
        assert!(matches!(err, ProcedureParseError::EmptyField("steps")));
    }

    #[test]
    fn invariants_reject_non_monotonic_steps() {
        let mut p = sample_minimal();
        p.steps.push(ProcedureStep {
            sequence: 3, // skips 2
            action_kk: "келесі қадам".into(),
            actor: None,
            condition: None,
            evidence: None,
        });
        let err = p.check_invariants().expect_err("step 1→3 must be rejected");
        assert!(matches!(
            err,
            ProcedureParseError::NonMonotonicSteps {
                expected: 2,
                found: 3
            }
        ));
    }

    #[test]
    fn invariants_reject_blank_version_date() {
        let mut p = sample_minimal();
        p.source.version_date.clear();
        let err = p
            .check_invariants()
            .expect_err("blank source.version_date must be rejected");
        assert!(matches!(
            err,
            ProcedureParseError::EmptyField("source.version_date"),
        ));
    }

    #[test]
    fn invariants_reject_malformed_date() {
        let mut p = sample_minimal();
        p.source.version_date = "2024/04/15".into(); // wrong separator
        let err = p
            .check_invariants()
            .expect_err("non-ISO date must be rejected");
        assert!(matches!(
            err,
            ProcedureParseError::MalformedDate {
                field: "source.version_date",
            },
        ));
    }

    #[test]
    fn freshness_window_passes_recent_source() {
        let p = sample_minimal();
        // source version 2024-04-15, today 2026-06-22 → 798 days.
        assert!(
            p.is_within_freshness_window("2026-06-22", 1825) // 5 years
                .expect("dates parse")
        );
    }

    #[test]
    fn freshness_window_rejects_stale_source() {
        let mut p = sample_minimal();
        p.source.version_date = "2010-01-01".into();
        assert!(
            !p.is_within_freshness_window("2026-06-22", 1825)
                .expect("dates parse")
        );
    }

    #[test]
    fn jsonl_round_trip() {
        let p = sample_minimal();
        let line = serde_json::to_string(&p).expect("serialize");
        let parsed = ProcedureIR::from_jsonl_line(&line).expect("round trip");
        assert_eq!(parsed, p);
    }

    #[test]
    fn trilingual_fields_default_to_empty_when_absent() {
        // v6.8.27 schema lift: title_en + aliases_{kk,ru,en} are
        // optional via #[serde(default)].  Pre-v6.8.27 JSONL
        // records without these fields must continue to load.
        let pre_lift = r#"{"id":"kk_legacy_001","title_kk":"тест",
                          "title_ru":null,"domain":"other",
                          "applies_to":[],"prerequisites":[],
                          "steps":[{"sequence":1,"action_kk":"x",
                            "actor":null,"condition":null,"evidence":null}],
                          "hazards":[],"authorization":[],
                          "confirmation_gates":[],
                          "source":{"regulation_kk":"r","regulation_id":"r",
                            "article":null,"version_date":"2024-01-01",
                            "retrieved_at":"2026-06-22","url":null}}"#;
        let p = ProcedureIR::from_jsonl_line(pre_lift)
            .expect("legacy JSONL without trilingual fields must load");
        assert!(p.title_en.is_none());
        assert!(p.aliases_kk.is_empty());
        assert!(p.aliases_ru.is_empty());
        assert!(p.aliases_en.is_empty());
    }

    #[test]
    fn trilingual_fields_round_trip() {
        let mut p = sample_minimal();
        p.title_en = Some("Initial briefing".into());
        p.aliases_kk = vec!["бастапқы нұсқаулық".into()];
        p.aliases_ru = vec!["вводный инструктаж".into(), "первичка".into()];
        p.aliases_en = vec!["initial briefing".into(), "induction".into()];
        let line = serde_json::to_string(&p).expect("serialize");
        let parsed = ProcedureIR::from_jsonl_line(&line).expect("round trip");
        assert_eq!(parsed, p);
        assert_eq!(parsed.title_en.as_deref(), Some("Initial briefing"));
        assert_eq!(parsed.aliases_ru.len(), 2);
    }

    #[test]
    fn jsonl_loader_rejects_invalid_record() {
        // Empty step list → invariant error.
        let bad = r#"{"id":"x","title_kk":"y","title_ru":null,"domain":"other",
                      "applies_to":[],"prerequisites":[],"steps":[],"hazards":[],
                      "authorization":[],"confirmation_gates":[],
                      "source":{"regulation_kk":"r","regulation_id":"r",
                                "article":null,"version_date":"2024-01-01",
                                "retrieved_at":"2026-06-22","url":null}}"#;
        let err = ProcedureIR::from_jsonl_line(bad)
            .expect_err("empty steps must be rejected at load time");
        assert!(matches!(err, ProcedureParseError::EmptyField("steps")));
    }
}
