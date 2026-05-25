// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `corpus_loader` — **Stage 7 of v6.2.0**: load
//! `data/world_core/*.jsonl` into a [`FrameIndex`].
//!
//! Reads the v6.1 curated knowledge graph (3461 entries × multiple
//! facts each = ~4100 frames) and produces a populated
//! [`FrameIndex`] the v6.2 pipeline consumes. Replaces the
//! hardcoded `canonical_corpus()` test fixture for production
//! use.
//!
//! ## Source schema
//!
//! Each JSONL line is one curated entry:
//!
//! ```jsonl
//! {"id":"abai_001",
//!  "kk":"Абай Құнанбаев — қазақтың ұлы ақыны.",
//!  "facts":[{"subject":"абай","predicate":"is_a","object":"ақын"}],
//!  "domain":"abai_works",
//!  "source":"curated",
//!  "confidence":"high",
//!  "review_status":"approved",
//!  "reviewer":"shaman",
//!  "reviewed_at":"2026-05-19"}
//! ```
//!
//! Each `facts[]` entry becomes one [`Frame`] in the index. The
//! `domain` slug maps to [`Domain`] via [`domain_from_slug`]; the
//! `predicate` slug maps via [`predicate_from_slug`]. Frames are
//! tagged with `Language::Kazakh` since the curated corpus is
//! Kazakh-rooted (Russian aliases are added at the `canonical_corpus`
//! level in `dialog_battery`).

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use serde::Deserialize;

use crate::composition::Composition;
use crate::frame::{Frame, FramePredicate};
use crate::index::FrameIndex;
use crate::query::{Domain, Language};
use crate::root::{PartOfSpeech, Root};

/// One JSONL entry from `data/world_core/*.jsonl`.
#[derive(Debug, Clone, Deserialize)]
struct CuratedEntry {
    #[serde(default)]
    id: String,
    #[serde(default)]
    kk: String,
    facts: Vec<CuratedFact>,
    #[serde(default)]
    domain: String,
}

#[derive(Debug, Clone, Deserialize)]
struct CuratedFact {
    subject: String,
    predicate: String,
    object: String,
}

/// Statistics from a corpus load. Surfaced by the dialog battery
/// + bench so the user can see «3461 entries → N frames in the
/// index».
#[derive(Debug, Clone, Default)]
pub struct LoadStats {
    pub entries_read: usize,
    pub frames_inserted: usize,
    pub unknown_predicate_skipped: usize,
    pub files_loaded: usize,
}

/// Load every `*.jsonl` file in `dir` into the supplied index.
/// Returns a [`LoadStats`] summarising what was loaded.
pub fn load_world_core_into(
    idx: &mut FrameIndex,
    dir: impl AsRef<Path>,
) -> std::io::Result<LoadStats> {
    let mut stats = LoadStats::default();
    let dir = dir.as_ref();
    if !dir.exists() {
        return Ok(stats);
    }
    let mut files: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s == "jsonl")
                .unwrap_or(false)
        })
        .collect();
    files.sort_by_key(|e| e.path());
    for f in files {
        stats.files_loaded += 1;
        load_jsonl_file(idx, &f.path(), &mut stats)?;
    }
    Ok(stats)
}

/// Convenience: build a fresh `FrameIndex` from `data/world_core/`.
pub fn load_world_core(dir: impl AsRef<Path>) -> std::io::Result<(FrameIndex, LoadStats)> {
    let mut idx = FrameIndex::new();
    let stats = load_world_core_into(&mut idx, dir)?;
    Ok((idx, stats))
}

fn load_jsonl_file(
    idx: &mut FrameIndex,
    path: &Path,
    stats: &mut LoadStats,
) -> std::io::Result<()> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let entry: CuratedEntry = match serde_json::from_str(trimmed) {
            Ok(e) => e,
            Err(_) => continue, // skip unparseable lines
        };
        stats.entries_read += 1;
        let _ = entry.id; // currently unused; reserved for provenance trace
        let _ = entry.kk;
        let domain = domain_from_slug(&entry.domain);
        for fact in entry.facts {
            let Some(pred) = predicate_from_slug(&fact.predicate) else {
                stats.unknown_predicate_skipped += 1;
                continue;
            };
            let frame = Frame::assertion(Some(noun(&fact.subject)), pred, Some(noun(&fact.object)));
            idx.insert_with_language(frame, domain.clone(), Some(Language::Kazakh));
            stats.frames_inserted += 1;
        }
    }
    Ok(())
}

fn noun(s: &str) -> Composition {
    Composition::identity(Root::new(s, PartOfSpeech::Noun))
}

/// Map a `world_core` predicate slug to its typed
/// [`FramePredicate`]. Returns `None` for unknown slugs; the
/// loader skips those and bumps `LoadStats::unknown_predicate_skipped`.
pub fn predicate_from_slug(slug: &str) -> Option<FramePredicate> {
    Some(match slug {
        "is_a" => FramePredicate::IsA,
        "lives_in" => FramePredicate::LivesIn,
        "has" => FramePredicate::Has,
        "goes_to" => FramePredicate::GoesTo,
        "part_of" => FramePredicate::PartOf,
        "related_to" => FramePredicate::RelatedTo,
        "causes" => FramePredicate::Causes,
        "after" => FramePredicate::After,
        "has_quantity" => FramePredicate::HasQuantity,
        "does_to" => FramePredicate::DoesTo,
        "in_domain" => FramePredicate::InDomain,
        "born_in" => FramePredicate::BornIn,
        "died_in" => FramePredicate::DiedIn,
        "founded_in" => FramePredicate::FoundedIn,
        "renamed_in" => FramePredicate::RenamedIn,
        "effective_from" => FramePredicate::EffectiveFrom,
        "classifies" => FramePredicate::Classifies,
        "risk_level" => FramePredicate::RiskLevel,
        "located_in" => FramePredicate::LocatedIn,
        "named_after" => FramePredicate::NamedAfter,
        "member_of" => FramePredicate::MemberOf,
        "authored" => FramePredicate::Authored,
        _ => return None,
    })
}

/// Map a `world_core` domain slug to its typed [`Domain`]. Returns
/// `Domain::Other(slug)` for slugs that don't map to a canonical
/// variant — preserves the slug for diagnostic / future-extension
/// use.
pub fn domain_from_slug(slug: &str) -> Option<Domain> {
    if slug.is_empty() {
        return None;
    }
    Some(match slug {
        "geography_kz" | "world_geography" => Domain::Geography,
        "abai_works"
        | "notable_kazakhstanis"
        | "kinship_extended"
        | "kz_literature"
        | "professions" => Domain::Person,
        "government_kazakhstan" | "kz_industry" | "military_kz" => Domain::Institution,
        "history_kazakhstan" | "world_history" => Domain::Event,
        "kz_constitution" => Domain::Law,
        "astronomy" | "constellations_kz" => Domain::Astronomy,
        "biology_basic" | "biology_school" | "chemistry_school" | "physics_school"
        | "mathematics_basic" | "natural_phenomena" | "weather_phenomena" | "animals"
        | "plants" | "body_parts" | "medicine_basic" | "informatics_basic" | "philosophy_basic"
        | "psychology_basic" | "economics_basic" => Domain::Science,
        "time" => Domain::Calendar,
        "computer_science_basics"
        | "programming_languages"
        | "programming_rust_advanced"
        | "programming_java"
        | "rust_curriculum_concepts" => Domain::Programming,
        "materials" => Domain::Material,
        other => Domain::Other(other.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The real `data/world_core` directory loads without error,
    /// produces > 3000 frames, and skips zero unknown predicates
    /// (all 11 v3.x predicates plus 11 v6.1.0 typed predicates are
    /// in `predicate_from_slug`).
    #[test]
    fn load_real_world_core() {
        let path = std::path::Path::new("../../data/world_core");
        if !path.exists() {
            // Test fixture: skip when run outside repo root (e.g.
            // packaged crate, CI working-dir mismatch).
            return;
        }
        let (idx, stats) = load_world_core(path).expect("load");
        assert!(
            stats.entries_read > 3000,
            "expected >3000 entries, got {}",
            stats.entries_read
        );
        assert!(
            stats.frames_inserted > 4000,
            "expected >4000 frames, got {}",
            stats.frames_inserted
        );
        assert_eq!(
            stats.unknown_predicate_skipped, 0,
            "no fact should be skipped — all 22 v6.1 predicates are mapped"
        );
        assert!(idx.len() == stats.frames_inserted);
    }

    /// Slug mapping round-trips: every v6.1 predicate slug maps to
    /// the matching `FramePredicate::as_str()`.
    #[test]
    fn predicate_slug_round_trip() {
        let preds = [
            FramePredicate::IsA,
            FramePredicate::LivesIn,
            FramePredicate::Has,
            FramePredicate::GoesTo,
            FramePredicate::PartOf,
            FramePredicate::RelatedTo,
            FramePredicate::Causes,
            FramePredicate::After,
            FramePredicate::HasQuantity,
            FramePredicate::DoesTo,
            FramePredicate::InDomain,
            FramePredicate::BornIn,
            FramePredicate::DiedIn,
            FramePredicate::FoundedIn,
            FramePredicate::RenamedIn,
            FramePredicate::EffectiveFrom,
            FramePredicate::Classifies,
            FramePredicate::RiskLevel,
            FramePredicate::LocatedIn,
            FramePredicate::NamedAfter,
            FramePredicate::MemberOf,
            FramePredicate::Authored,
        ];
        for p in preds {
            let s = p.as_str();
            assert_eq!(predicate_from_slug(s), Some(p));
        }
    }

    #[test]
    fn unknown_predicate_returns_none() {
        assert_eq!(predicate_from_slug("dance_around"), None);
    }

    #[test]
    fn domain_unknown_slug_becomes_other() {
        assert!(matches!(
            domain_from_slug("brand_new_domain"),
            Some(Domain::Other(_))
        ));
    }

    /// Sanity: an in-memory mini-corpus loads correctly.
    #[test]
    fn load_inline_jsonl() {
        let tmp = std::env::temp_dir().join("adam_algebra_load_test");
        std::fs::create_dir_all(&tmp).unwrap();
        let path = tmp.join("test.jsonl");
        std::fs::write(
            &path,
            r#"{"id":"t1","kk":"Х — у.","facts":[{"subject":"х","predicate":"is_a","object":"у"}],"domain":"geography_kz"}
{"id":"t2","kk":"А кірді б.","facts":[{"subject":"а","predicate":"part_of","object":"б"}],"domain":"history_kazakhstan"}
"#,
        ).unwrap();
        let (idx, stats) = load_world_core(tmp.as_path()).unwrap();
        assert_eq!(stats.entries_read, 2);
        assert_eq!(stats.frames_inserted, 2);
        assert_eq!(stats.unknown_predicate_skipped, 0);
        assert_eq!(idx.len(), 2);
        let _ = path;
    }
}
