//! v3.9.0 — **World Core**: curated Kazakh knowledge packs.
//!
//! The text-pattern matchers in [`crate::patterns`] extract facts from
//! raw corpus. Their precision is bounded by what the Kazakh text makes
//! explicit — and Kazakh prose often leaves ontology implicit.
//! "Қазақстан Орталық Азияда орналасқан" yields a locative fact about
//! Kazakhstan, but it does not yield the foundational claim **«Жер —
//! ғаламшар»** (Earth is a planet). That kind of foundational
//! "mироустройство" ("world-structure") knowledge has to be curated.
//!
//! `WorldCoreEntry` is the authored unit: one short Kazakh statement
//! plus 1–3 typed facts. Each entry carries `domain`, `source`,
//! `confidence`, `review_status` — the full trust stack.
//!
//! Design decisions:
//!
//! 1. **Human-authored, not mined.** Every entry is a deliberate act by
//!    a reviewer (default `shaman` during bootstrap).
//! 2. **Facts are the primary payload.** The `kk` sentence exists for
//!    human audit + future NLU; the `facts` array is what enters the
//!    graph.
//! 3. **`ConfidenceKind::HumanApproved`** is the v2.1-baked enum variant
//!    that was reserved exactly for this path. Text-extracted facts get
//!    `Grammar`; world-core facts get `HumanApproved`. Downstream
//!    consumers (`adam_inspect`, the demo, the planner) can filter on
//!    this to get a **curated-only** view of the graph.
//! 4. **JSONL per domain.** `data/world_core/astronomy.jsonl`,
//!    `data/world_core/geography_kz.jsonl`, etc. One entry per line;
//!    stable sort key is the `id` field; easy to diff in PRs.
//! 5. **No inline code in data.** Reviewers edit pure JSONL; the loader
//!    does all validation.
//!
//! The v3.9.0 goal is **bootstrap**: ship the schema + loader + a
//! couple of hundred seed entries so `adam_chat` / `adam_inspect` /
//! `adam_demo` can cite curated facts today. The long-term goal
//! (v4.0.0) is a 5 000+ entry world_core that makes the project a
//! genuine «auditable Kazakh reasoning engine».

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{ConfidenceKind, Fact, FactSource, Predicate, SlotRef};

/// One curated knowledge entry — a short Kazakh sentence plus the
/// typed facts it asserts. Serialised one-per-line in
/// `data/world_core/<domain>.jsonl`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldCoreEntry {
    /// Stable unique id within the domain file (e.g. `earth_001`).
    /// Loader validates uniqueness across all domains.
    pub id: String,
    /// The Kazakh sentence expressing the claim. Kept for human audit +
    /// for dialog layers that want to cite the sentence verbatim with
    /// the «байланыс-» trust marker.
    pub kk: String,
    /// 1–N typed facts the entry asserts. Each becomes one `Fact` in
    /// the merged `facts.json` with `ConfidenceKind::HumanApproved`.
    pub facts: Vec<WorldCoreFact>,
    /// Coarse category — used for organising authoring + filtering
    /// at `adam_inspect` time. Free-form string; convention is lower-
    /// snake (e.g. `astronomy`, `geography_kz`, `time`, `society`).
    pub domain: String,
    /// Where this claim came from. Bootstrap vocabulary:
    /// `curated` = author's own formulation from common knowledge;
    /// `wiki_distilled` = condensed from Wikipedia KZ;
    /// `textbook_distilled` = condensed from a Kazakh textbook;
    /// `community` = contributed by a native speaker reviewer.
    pub source: String,
    /// `high` / `medium` / `low`. Reserved for review workflow. At
    /// bootstrap every entry is authored at `high`; entries queued
    /// for native-speaker review sit at `medium` until approved.
    pub confidence: ConfidenceTier,
    /// `approved` / `pending` / `rejected`. Only `approved` entries
    /// enter the committed `facts.json`. `pending` and `rejected`
    /// are loaded by tooling but filtered out of the runtime fact set.
    pub review_status: ReviewStatus,
    /// Git handle of the reviewer who approved this entry. Convention:
    /// `shaman` during bootstrap; native-speaker handles afterwards.
    pub reviewer: String,
    /// ISO-8601 date of the approval (`YYYY-MM-DD`).
    pub reviewed_at: String,
}

/// One typed fact inside a `WorldCoreEntry`. Cheap-copy struct mapped
/// directly to the main `Fact` type at load time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldCoreFact {
    /// Subject root. Must be FST-analysable; validator rejects fragments
    /// and dash-prefixed noise (see v3.9.0 Part A hygiene gate).
    pub subject: String,
    /// One of the 11 `Predicate` variants. Serialised as its `as_str()`
    /// form (`"is_a"`, `"part_of"`, …) so the JSONL stays
    /// human-readable in diffs.
    #[serde(with = "predicate_serde")]
    pub predicate: Predicate,
    /// Object root. Same validation rules as subject.
    pub object: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfidenceTier {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReviewStatus {
    Approved,
    Pending,
    Rejected,
}

/// Serde adapter so `Predicate` round-trips through its stable string
/// form in JSONL — keeps PR diffs readable.
mod predicate_serde {
    use super::Predicate;
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub fn serialize<S: Serializer>(p: &Predicate, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(p.as_str())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Predicate, D::Error> {
        let raw = String::deserialize(d)?;
        match raw.as_str() {
            "is_a" => Ok(Predicate::IsA),
            "lives_in" => Ok(Predicate::LivesIn),
            "has" => Ok(Predicate::Has),
            "goes_to" => Ok(Predicate::GoesTo),
            "part_of" => Ok(Predicate::PartOf),
            "related_to" => Ok(Predicate::RelatedTo),
            "causes" => Ok(Predicate::Causes),
            "after" => Ok(Predicate::After),
            "has_quantity" => Ok(Predicate::HasQuantity),
            "does_to" => Ok(Predicate::DoesTo),
            "in_domain" => Ok(Predicate::InDomain),
            other => Err(D::Error::custom(format!("unknown predicate: {other}"))),
        }
    }
}

/// Errors produced by the loader / validator.
#[derive(Debug)]
pub enum WorldCoreError {
    Io(std::io::Error, PathBuf),
    Parse {
        path: PathBuf,
        line: usize,
        message: String,
    },
    DuplicateId {
        id: String,
        first_at: PathBuf,
        second_at: PathBuf,
    },
    InvalidEntry {
        path: PathBuf,
        line: usize,
        id: String,
        reason: String,
    },
}

impl std::fmt::Display for WorldCoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e, p) => write!(f, "{}: {e}", p.display()),
            Self::Parse {
                path,
                line,
                message,
            } => write!(f, "{}:{line}: {message}", path.display()),
            Self::DuplicateId {
                id,
                first_at,
                second_at,
            } => write!(
                f,
                "duplicate id `{id}` (first seen in {}, again in {})",
                first_at.display(),
                second_at.display(),
            ),
            Self::InvalidEntry {
                path,
                line,
                id,
                reason,
            } => write!(
                f,
                "{}:{line} entry `{id}` invalid — {reason}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for WorldCoreError {}

/// Outcome of loading a world-core directory. Separates valid entries
/// from rejected ones so the validator binary can print a detailed
/// report while the pipeline still gets a usable set.
#[derive(Debug, Default)]
pub struct LoadReport {
    /// Entries that parsed cleanly AND passed structural validation.
    pub entries: Vec<(WorldCoreEntry, PathBuf)>,
    /// Entries that were rejected, paired with the reason.
    pub rejected: Vec<WorldCoreError>,
}

/// Load every `*.jsonl` under `root` (one entry per line). Returns a
/// `LoadReport` so the caller can decide whether to fail fast or keep
/// going. Rejected entries never appear in `entries`.
pub fn load_world_core_dir(root: &Path) -> Result<LoadReport, WorldCoreError> {
    let mut report = LoadReport::default();
    let mut seen_ids: std::collections::BTreeMap<String, PathBuf> =
        std::collections::BTreeMap::new();

    if !root.exists() {
        return Ok(report);
    }

    let mut files: Vec<PathBuf> = std::fs::read_dir(root)
        .map_err(|e| WorldCoreError::Io(e, root.to_path_buf()))?
        .filter_map(|r| r.ok())
        .map(|d| d.path())
        .filter(|p| p.extension().is_some_and(|e| e == "jsonl"))
        .collect();
    // Deterministic file order — alphabetical.
    files.sort();

    for path in files {
        let raw =
            std::fs::read_to_string(&path).map_err(|e| WorldCoreError::Io(e, path.clone()))?;
        for (idx, line) in raw.lines().enumerate() {
            let line_number = idx + 1;
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("//") {
                continue;
            }
            let entry: WorldCoreEntry = match serde_json::from_str(trimmed) {
                Ok(e) => e,
                Err(e) => {
                    report.rejected.push(WorldCoreError::Parse {
                        path: path.clone(),
                        line: line_number,
                        message: e.to_string(),
                    });
                    continue;
                }
            };
            if let Err(reason) = validate_entry(&entry) {
                report.rejected.push(WorldCoreError::InvalidEntry {
                    path: path.clone(),
                    line: line_number,
                    id: entry.id.clone(),
                    reason,
                });
                continue;
            }
            if let Some(first_path) = seen_ids.get(&entry.id) {
                report.rejected.push(WorldCoreError::DuplicateId {
                    id: entry.id.clone(),
                    first_at: first_path.clone(),
                    second_at: path.clone(),
                });
                continue;
            }
            seen_ids.insert(entry.id.clone(), path.clone());
            report.entries.push((entry, path.clone()));
        }
    }

    // Deterministic final ordering: (domain file path, id).
    report
        .entries
        .sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.id.cmp(&b.0.id)));
    Ok(report)
}

/// Validate a single entry beyond what serde can tell us.
fn validate_entry(entry: &WorldCoreEntry) -> Result<(), String> {
    if entry.id.is_empty() {
        return Err("empty id".into());
    }
    if entry.kk.trim().is_empty() {
        return Err("empty kk sentence".into());
    }
    if entry.facts.is_empty() {
        return Err("no facts asserted".into());
    }
    if entry.domain.is_empty() {
        return Err("empty domain".into());
    }
    for (i, f) in entry.facts.iter().enumerate() {
        if f.subject.is_empty() {
            return Err(format!("facts[{i}] has empty subject"));
        }
        if f.object.is_empty() {
            return Err(format!("facts[{i}] has empty object"));
        }
        if f.subject == f.object {
            return Err(format!(
                "facts[{i}] is a tautology ({} → {} → {})",
                f.subject,
                f.predicate.as_str(),
                f.object,
            ));
        }
        // Hygiene: never accept fragment roots in curated data either.
        if f.subject.starts_with('-') || f.object.starts_with('-') {
            return Err(format!(
                "facts[{i}] uses dash-prefixed fragment root ({} / {})",
                f.subject, f.object,
            ));
        }
    }
    Ok(())
}

/// Convert a loaded entry + its source file path into `Fact`s for the
/// main pipeline. Every emitted fact carries
/// `ConfidenceKind::HumanApproved` and `source.pack` equal to
/// `"world_core/<domain>.jsonl"` — filename only, no directory prefix.
pub fn emit_facts(entry: &WorldCoreEntry, source_path: &Path) -> Vec<Fact> {
    // Filter by review_status at emit time — only approved entries
    // enter the committed fact set.
    if entry.review_status != ReviewStatus::Approved {
        return Vec::new();
    }
    let pack = source_path
        .file_name()
        .map(|f| format!("world_core/{}", f.to_string_lossy()))
        .unwrap_or_else(|| "world_core/unknown.jsonl".to_string());

    entry
        .facts
        .iter()
        .map(|f| Fact {
            subject: SlotRef {
                surface: f.subject.clone(),
                root: f.subject.clone(),
                pos: "noun".to_string(),
            },
            predicate: f.predicate,
            object: SlotRef {
                surface: f.object.clone(),
                root: f.object.clone(),
                pos: "noun".to_string(),
            },
            pattern: format!("world_core/{}", entry.domain),
            source: FactSource {
                pack: pack.clone(),
                sample_id: entry.id.clone(),
            },
            confidence: ConfidenceKind::HumanApproved,
            raw_text: entry.kk.clone(),
        })
        .collect()
}

/// Flat: load + emit all approved facts from `root` in one call.
/// Convenience for `extract_facts` and friends. Rejected / pending /
/// parse-failing entries are silently dropped — use
/// `load_world_core_dir` directly if you need the reject list (e.g. in
/// the `validate_world_core` binary).
pub fn load_world_core_facts(root: &Path) -> Result<Vec<Fact>, WorldCoreError> {
    let report = load_world_core_dir(root)?;
    let mut out = Vec::new();
    for (entry, path) in &report.entries {
        out.extend(emit_facts(entry, path));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry() -> WorldCoreEntry {
        WorldCoreEntry {
            id: "earth_001".into(),
            kk: "Жер — Күн жүйесіндегі ғаламшар.".into(),
            facts: vec![
                WorldCoreFact {
                    subject: "жер".into(),
                    predicate: Predicate::IsA,
                    object: "ғаламшар".into(),
                },
                WorldCoreFact {
                    subject: "жер".into(),
                    predicate: Predicate::PartOf,
                    object: "күн жүйесі".into(),
                },
            ],
            domain: "astronomy".into(),
            source: "curated".into(),
            confidence: ConfidenceTier::High,
            review_status: ReviewStatus::Approved,
            reviewer: "shaman".into(),
            reviewed_at: "2026-04-23".into(),
        }
    }

    #[test]
    fn entry_round_trips_through_jsonl() {
        let e = sample_entry();
        let line = serde_json::to_string(&e).unwrap();
        let back: WorldCoreEntry = serde_json::from_str(&line).unwrap();
        assert_eq!(back, e);
    }

    #[test]
    fn emit_facts_sets_human_approved_confidence() {
        let e = sample_entry();
        let path = PathBuf::from("data/world_core/astronomy.jsonl");
        let facts = emit_facts(&e, &path);
        assert_eq!(facts.len(), 2);
        for f in &facts {
            assert_eq!(f.confidence, ConfidenceKind::HumanApproved);
            assert!(f.source.pack.starts_with("world_core/"));
            assert_eq!(f.source.sample_id, "earth_001");
        }
    }

    #[test]
    fn emit_facts_refuses_pending_entry() {
        let mut e = sample_entry();
        e.review_status = ReviewStatus::Pending;
        let path = PathBuf::from("data/world_core/astronomy.jsonl");
        assert!(emit_facts(&e, &path).is_empty());
    }

    #[test]
    fn emit_facts_refuses_rejected_entry() {
        let mut e = sample_entry();
        e.review_status = ReviewStatus::Rejected;
        let path = PathBuf::from("data/world_core/astronomy.jsonl");
        assert!(emit_facts(&e, &path).is_empty());
    }

    #[test]
    fn validate_rejects_empty_facts() {
        let mut e = sample_entry();
        e.facts.clear();
        assert!(validate_entry(&e).is_err());
    }

    #[test]
    fn validate_rejects_fragment_root() {
        let mut e = sample_entry();
        e.facts[0].subject = "-ға".into();
        let err = validate_entry(&e).unwrap_err();
        assert!(err.contains("fragment"), "got: {err}");
    }

    #[test]
    fn validate_rejects_tautology() {
        let mut e = sample_entry();
        e.facts[0].object = "жер".into();
        let err = validate_entry(&e).unwrap_err();
        assert!(err.contains("tautology"), "got: {err}");
    }

    #[test]
    fn load_world_core_dir_handles_missing_root() {
        let missing = PathBuf::from("/tmp/nonexistent-world-core-path-xyz");
        let report = load_world_core_dir(&missing).unwrap();
        assert!(report.entries.is_empty());
        assert!(report.rejected.is_empty());
    }

    #[test]
    fn load_world_core_dir_parses_jsonl_and_flags_duplicates() {
        let dir = tempfile::tempdir().unwrap();
        let path_a = dir.path().join("a.jsonl");
        let path_b = dir.path().join("b.jsonl");
        let e1 = serde_json::to_string(&sample_entry()).unwrap();
        let mut dup = sample_entry();
        dup.kk = "(duplicate)".into();
        let e2 = serde_json::to_string(&dup).unwrap();
        let mut other = sample_entry();
        other.id = "earth_002".into();
        other.facts[0].object = "аспан денесі".into();
        let e3 = serde_json::to_string(&other).unwrap();
        std::fs::write(&path_a, format!("{e1}\n{e3}\n")).unwrap();
        std::fs::write(&path_b, format!("{e2}\n")).unwrap();
        let report = load_world_core_dir(dir.path()).unwrap();
        assert_eq!(report.entries.len(), 2);
        assert_eq!(report.rejected.len(), 1);
        match &report.rejected[0] {
            WorldCoreError::DuplicateId { id, .. } => assert_eq!(id, "earth_001"),
            other => panic!("expected DuplicateId, got {other:?}"),
        }
    }
}
