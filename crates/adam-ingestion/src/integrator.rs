// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Integration of `ApprovedByHuman` candidates into the
//! curated `world_core` JSONL.
//!
//! Closes the «raw → candidates → validator → review →
//! world_core» pipeline.  For every `ApprovedByHuman`
//! candidate, the integrator:
//!
//!   1. Synthesises a [`WorldCoreEntry`] from the candidate
//!      plus the [`IntegrationTarget`] (domain, reviewer,
//!      review-date, id prefix).
//!   2. Allocates a new id by scanning the target file's
//!      existing entries with the same prefix and picking
//!      the next free numeric suffix.
//!   3. Appends the entry to the target file atomically
//!      (write whole file to `<file>.tmp` then rename).
//!   4. Transitions the candidate to
//!      [`IngestionStatus::IntegratedIntoWorldCore`] via the
//!      store's state-machine-enforcing update.
//!
//! Re-runs are safe — only `ApprovedByHuman` candidates are
//! processed; ones that already reached
//! `IntegratedIntoWorldCore` get skipped.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::candidate::CandidateFact;
use crate::status::IngestionStatus;
use crate::store::{CandidateStore, StoreError};

/// World-core JSONL entry shape — mirrors the
/// `data/world_core/*.jsonl` format production already
/// consumes («id», «kk» canonical sentence, «facts»,
/// «domain», «source», «confidence», «review_status»,
/// «reviewer», «reviewed_at»).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldCoreEntry {
    pub id: String,
    /// Canonical Kazakh sentence the entry expresses.
    pub kk: String,
    pub facts: Vec<WorldCoreFact>,
    pub domain: String,
    /// Provenance label — «curated» for human-authored,
    /// «ingestion» (default for this module) for candidates
    /// that came through the pipeline.
    pub source: String,
    /// Free-text confidence bucket: «high» / «medium» /
    /// «low».  ApprovedByHuman candidates land as «high»
    /// (human reviewed it), unless the caller overrides.
    pub confidence: String,
    /// Always «approved» on entries this integrator writes
    /// — by definition they cleared the human review gate.
    pub review_status: String,
    pub reviewer: String,
    pub reviewed_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldCoreFact {
    pub subject: String,
    pub predicate: String,
    pub object: String,
}

/// Caller-provided configuration for one integration run.
/// Bundled so that the function signature stays compact
/// even as the integrator picks up more knobs.
#[derive(Debug, Clone)]
pub struct IntegrationTarget {
    /// Absolute or working-dir-relative path to the world_core
    /// jsonl file the candidates land in.  Missing file is
    /// created on first write.
    pub world_core_path: PathBuf,
    /// Domain label written into the entry's `domain` field
    /// AND used as a sanity check that the file's name
    /// matches.
    pub domain: String,
    /// Prefix the integrator uses to allocate new ids
    /// («bio» → `bio_NNN`, «geo_kz» → `geo_kz_NNN`).
    /// Matched case-sensitively against existing ids in the
    /// target file so re-runs continue the same number line.
    pub id_prefix: String,
    /// Name written into the entry's `reviewer` field.
    pub reviewer: String,
    /// ISO date for the entry's `reviewed_at` field
    /// (`YYYY-MM-DD`).
    pub reviewed_at: String,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct IntegrationSummary {
    /// How many `ApprovedByHuman` candidates were inspected.
    pub examined: usize,
    /// How many were written into world_core this run.
    pub integrated: usize,
    /// How many were skipped because they were already
    /// `IntegratedIntoWorldCore` from a prior run.
    pub already_integrated: usize,
}

#[derive(Debug, Error)]
pub enum IntegrationError {
    #[error("store error: {0}")]
    Store(#[from] StoreError),
    #[error("io error on {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Integrate every `ApprovedByHuman` fact candidate in
/// `store` into the world_core file pointed at by
/// `target`.  Returns a summary of what was integrated /
/// skipped.  See module docs for the full algorithm.
pub fn integrate_approved_facts(
    store: &CandidateStore,
    target: &IntegrationTarget,
) -> Result<IntegrationSummary, IntegrationError> {
    let mut summary = IntegrationSummary::default();
    let snapshot = store.load_facts()?;
    let mut existing = load_world_core_entries(&target.world_core_path)?;
    let mut next_seq = next_id_sequence(&existing, &target.id_prefix);
    let mut written_ids = Vec::new();
    for fact in snapshot.iter() {
        match fact.status {
            IngestionStatus::ApprovedByHuman => {
                summary.examined += 1;
            }
            IngestionStatus::IntegratedIntoWorldCore => {
                summary.already_integrated += 1;
                continue;
            }
            _ => continue,
        }
        let entry = candidate_to_entry(fact, target, next_seq);
        next_seq += 1;
        written_ids.push(fact.id.clone());
        existing.push(entry);
    }
    if !written_ids.is_empty() {
        save_world_core_entries(&target.world_core_path, &existing)?;
        // Transition candidates AFTER the world_core write
        // succeeds — otherwise a crash between transition
        // and write would leave the candidate marked
        // integrated without an entry on disk.
        for id in &written_ids {
            store.update_fact_status(
                id,
                IngestionStatus::IntegratedIntoWorldCore,
                "integrator: written to world_core",
            )?;
            summary.integrated += 1;
        }
    }
    Ok(summary)
}

/// Pure function — build a `WorldCoreEntry` from a single
/// candidate plus the integration target.  Exposed so tests
/// can exercise it without writing files.
pub fn candidate_to_entry(
    fact: &CandidateFact,
    target: &IntegrationTarget,
    seq: u32,
) -> WorldCoreEntry {
    let kk = if !fact.source_sentence.trim().is_empty() {
        fact.source_sentence.clone()
    } else {
        // Synthesise a canonical sentence when the candidate
        // doesn't carry the original surface (e.g. manual
        // entry).  Capitalises the subject to match
        // production world_core sentence convention.
        let subject = capitalise_first(&fact.subject);
        format!("{subject} — {}.", fact.object)
    };
    WorldCoreEntry {
        id: format!("{}_{:03}", target.id_prefix, seq),
        kk,
        facts: vec![WorldCoreFact {
            subject: fact.subject.clone(),
            predicate: fact.predicate.clone(),
            object: fact.object.clone(),
        }],
        domain: target.domain.clone(),
        source: "ingestion".into(),
        confidence: "high".into(),
        review_status: "approved".into(),
        reviewer: target.reviewer.clone(),
        reviewed_at: target.reviewed_at.clone(),
    }
}

/// Find the next numeric suffix to use for `id_prefix`
/// given the existing entries.  Returns 1 when no existing
/// entry matches.
pub fn next_id_sequence(existing: &[WorldCoreEntry], id_prefix: &str) -> u32 {
    let prefix_with_sep = format!("{id_prefix}_");
    let mut max_seen = 0u32;
    for entry in existing {
        let Some(rest) = entry.id.strip_prefix(&prefix_with_sep) else {
            continue;
        };
        // Parse trailing digits — entries may have suffixes
        // like `_v2` etc. that we want to skip rather than
        // crash on.
        if let Ok(n) = rest.parse::<u32>() {
            if n > max_seen {
                max_seen = n;
            }
        }
    }
    max_seen + 1
}

fn load_world_core_entries(path: &Path) -> Result<Vec<WorldCoreEntry>, IntegrationError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(path).map_err(|e| IntegrationError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    let mut out = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        out.push(serde_json::from_str::<WorldCoreEntry>(line)?);
    }
    Ok(out)
}

fn save_world_core_entries(
    path: &Path,
    entries: &[WorldCoreEntry],
) -> Result<(), IntegrationError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| IntegrationError::Io {
            path: parent.to_path_buf(),
            source: e,
        })?;
    }
    let tmp = path.with_extension("jsonl.tmp");
    let mut file = fs::File::create(&tmp).map_err(|e| IntegrationError::Io {
        path: tmp.clone(),
        source: e,
    })?;
    for e in entries {
        let line = serde_json::to_string(e)?;
        writeln!(file, "{line}").map_err(|err| IntegrationError::Io {
            path: tmp.clone(),
            source: err,
        })?;
    }
    drop(file);
    fs::rename(&tmp, path).map_err(|e| IntegrationError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    Ok(())
}

fn capitalise_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceRef;

    fn tmp_root(tag: &str) -> PathBuf {
        let tid = format!("{:?}", std::thread::current().id()).replace([':', ' ', '(', ')'], "_");
        let dir = std::env::temp_dir().join(format!("adam-integrator-test-{tag}-{tid}"));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn approved_fact(id: &str, subject: &str, object: &str) -> CandidateFact {
        CandidateFact {
            id: id.into(),
            subject: subject.into(),
            predicate: "is_a".into(),
            object: object.into(),
            source_sentence: format!("{} — {}.", capitalise_first(subject), object),
            source: SourceRef::manual("reviewer"),
            status: IngestionStatus::ApprovedByHuman,
            confidence: 1.0,
            created_at: "2026-06-29".into(),
            notes: String::new(),
        }
    }

    fn make_target(root: &Path, file_name: &str, prefix: &str) -> IntegrationTarget {
        IntegrationTarget {
            world_core_path: root.join(file_name),
            domain: "test_domain".into(),
            id_prefix: prefix.into(),
            reviewer: "shaman".into(),
            reviewed_at: "2026-06-29".into(),
        }
    }

    #[test]
    fn integrates_approved_candidate_into_empty_file() {
        let root = tmp_root("empty");
        fs::create_dir_all(&root).unwrap();
        let store = CandidateStore::open(root.join("queue")).expect("store open");
        store
            .save_facts(&[approved_fact("a", "темір", "металл")])
            .expect("save");
        let target = make_target(&root, "test.jsonl", "test");
        let summary = integrate_approved_facts(&store, &target).expect("integrate");
        assert_eq!(summary.integrated, 1);
        assert_eq!(summary.already_integrated, 0);
        // World_core file got the entry.
        let entries = load_world_core_entries(&target.world_core_path).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "test_001");
        assert_eq!(entries[0].kk, "Темір — металл.");
        assert_eq!(entries[0].facts[0].subject, "темір");
        assert_eq!(entries[0].domain, "test_domain");
        assert_eq!(entries[0].source, "ingestion");
        // Candidate transitioned to IntegratedIntoWorldCore.
        let loaded = store.load_facts().unwrap();
        assert_eq!(loaded[0].status, IngestionStatus::IntegratedIntoWorldCore);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn appends_to_existing_world_core_file_with_correct_id_sequence() {
        let root = tmp_root("append");
        fs::create_dir_all(&root).unwrap();
        let world_path = root.join("test.jsonl");
        // Pre-seed with two existing entries.
        let seed = vec![
            WorldCoreEntry {
                id: "test_001".into(),
                kk: "Сұр — түс.".into(),
                facts: vec![WorldCoreFact {
                    subject: "сұр".into(),
                    predicate: "is_a".into(),
                    object: "түс".into(),
                }],
                domain: "test_domain".into(),
                source: "curated".into(),
                confidence: "high".into(),
                review_status: "approved".into(),
                reviewer: "shaman".into(),
                reviewed_at: "2026-04-01".into(),
            },
            WorldCoreEntry {
                id: "test_002".into(),
                kk: "Қызыл — түс.".into(),
                facts: vec![WorldCoreFact {
                    subject: "қызыл".into(),
                    predicate: "is_a".into(),
                    object: "түс".into(),
                }],
                domain: "test_domain".into(),
                source: "curated".into(),
                confidence: "high".into(),
                review_status: "approved".into(),
                reviewer: "shaman".into(),
                reviewed_at: "2026-04-01".into(),
            },
        ];
        save_world_core_entries(&world_path, &seed).unwrap();

        let store = CandidateStore::open(root.join("queue")).expect("store open");
        store
            .save_facts(&[approved_fact("c1", "көк", "түс")])
            .expect("save");
        let target = make_target(&root, "test.jsonl", "test");
        let summary = integrate_approved_facts(&store, &target).expect("integrate");
        assert_eq!(summary.integrated, 1);

        let entries = load_world_core_entries(&target.world_core_path).unwrap();
        assert_eq!(entries.len(), 3);
        // New entry gets the next id in sequence.
        assert_eq!(entries[2].id, "test_003");
        assert_eq!(entries[2].facts[0].subject, "көк");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rerun_does_not_double_integrate() {
        let root = tmp_root("rerun");
        fs::create_dir_all(&root).unwrap();
        let store = CandidateStore::open(root.join("queue")).expect("store open");
        store
            .save_facts(&[approved_fact("a", "темір", "металл")])
            .expect("save");
        let target = make_target(&root, "test.jsonl", "test");
        let s1 = integrate_approved_facts(&store, &target).expect("integrate 1");
        let s2 = integrate_approved_facts(&store, &target).expect("integrate 2");
        assert_eq!(s1.integrated, 1);
        assert_eq!(s2.integrated, 0);
        assert_eq!(s2.already_integrated, 1);
        let entries = load_world_core_entries(&target.world_core_path).unwrap();
        assert_eq!(entries.len(), 1, "rerun should not add a second copy");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn non_approved_candidates_left_alone() {
        let root = tmp_root("nonapproved");
        fs::create_dir_all(&root).unwrap();
        let store = CandidateStore::open(root.join("queue")).expect("store open");
        let mut pending = approved_fact("p1", "темір", "металл");
        pending.status = IngestionStatus::Pending;
        let mut needs_review = approved_fact("p2", "темір", "элемент");
        needs_review.status = IngestionStatus::NeedsReview;
        let mut rejected = approved_fact("p3", "темір", "сұйық");
        rejected.status = IngestionStatus::RejectedByHuman;
        store
            .save_facts(&[pending, needs_review, rejected])
            .expect("save");
        let target = make_target(&root, "test.jsonl", "test");
        let summary = integrate_approved_facts(&store, &target).expect("integrate");
        assert_eq!(summary.integrated, 0);
        let entries = load_world_core_entries(&target.world_core_path).unwrap_or_default();
        assert!(entries.is_empty(), "got: {entries:?}");
        // Candidate statuses unchanged.
        let loaded = store.load_facts().unwrap();
        assert_eq!(loaded[0].status, IngestionStatus::Pending);
        assert_eq!(loaded[1].status, IngestionStatus::NeedsReview);
        assert_eq!(loaded[2].status, IngestionStatus::RejectedByHuman);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn next_id_sequence_handles_non_matching_ids() {
        // Mix of matching and non-matching prefixes.
        let entries = vec![
            WorldCoreEntry {
                id: "bio_001".into(),
                kk: "x".into(),
                facts: vec![],
                domain: "bio".into(),
                source: "curated".into(),
                confidence: "high".into(),
                review_status: "approved".into(),
                reviewer: "x".into(),
                reviewed_at: "2026-01-01".into(),
            },
            WorldCoreEntry {
                id: "geo_kz_017".into(),
                kk: "x".into(),
                facts: vec![],
                domain: "geo_kz".into(),
                source: "curated".into(),
                confidence: "high".into(),
                review_status: "approved".into(),
                reviewer: "x".into(),
                reviewed_at: "2026-01-01".into(),
            },
            WorldCoreEntry {
                id: "geo_kz_005".into(),
                kk: "x".into(),
                facts: vec![],
                domain: "geo_kz".into(),
                source: "curated".into(),
                confidence: "high".into(),
                review_status: "approved".into(),
                reviewer: "x".into(),
                reviewed_at: "2026-01-01".into(),
            },
        ];
        assert_eq!(next_id_sequence(&entries, "bio"), 2);
        assert_eq!(next_id_sequence(&entries, "geo_kz"), 18);
        assert_eq!(next_id_sequence(&entries, "absent"), 1);
    }

    #[test]
    fn synthesises_kk_sentence_when_source_missing() {
        let mut f = approved_fact("a", "темір", "металл");
        f.source_sentence.clear();
        let target = make_target(Path::new("/tmp/unused"), "test.jsonl", "test");
        let entry = candidate_to_entry(&f, &target, 1);
        assert_eq!(entry.kk, "Темір — металл.");
    }

    #[test]
    fn batch_of_three_yields_consecutive_ids() {
        let root = tmp_root("batch");
        fs::create_dir_all(&root).unwrap();
        let store = CandidateStore::open(root.join("queue")).expect("store open");
        store
            .save_facts(&[
                approved_fact("a", "темір", "металл"),
                approved_fact("b", "алтын", "металл"),
                approved_fact("c", "мыс", "металл"),
            ])
            .expect("save");
        let target = make_target(&root, "test.jsonl", "test");
        let summary = integrate_approved_facts(&store, &target).expect("integrate");
        assert_eq!(summary.integrated, 3);
        let entries = load_world_core_entries(&target.world_core_path).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].id, "test_001");
        assert_eq!(entries[1].id, "test_002");
        assert_eq!(entries[2].id, "test_003");
        let _ = fs::remove_dir_all(&root);
    }
}
