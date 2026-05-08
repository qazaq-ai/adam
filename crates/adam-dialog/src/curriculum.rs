//! **v4.98.0** — lesson-state curriculum tree (long-term roadmap step 1).
//!
//! Defines an ordered Rust pedagogical curriculum. Each `Stage` names
//! a core Rust concept (ownership / borrow / lifetime / traits /
//! async); advancement is gated by `exercises_to_pass` successful
//! `SubmitSolution` verdicts on that stage's topic. Prereqs constrain
//! which stage is the recommended next step.
//!
//! ## Architectural role
//!
//! Curriculum data is **read-only** and shared across conversations
//! (loaded once from `data/dialog/curriculum/rust_progression.json`).
//! Per-conversation progress lives on [`crate::Conversation`] as a
//! `HashMap<String, StageProgress>`. The planner (in v4.98.5+) will
//! consult both to decide whether a successful `SubmitSolution`
//! closes a stage and triggers a "next topic" hint.
//!
//! v4.98.0 ships the data + tracking infrastructure; user-facing
//! response changes are deferred to v4.98.5.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Default curriculum file (relative to workspace root).
pub const DEFAULT_CURRICULUM_PATH: &str = "data/dialog/curriculum/rust_progression.json";

/// One stage in a curriculum — a discrete pedagogical concept the
/// student progresses through.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stage {
    /// Lowercase canonical identifier — matches the topic slot used
    /// by `Intent::AskExercise`, `Intent::SubmitSolution`, etc.
    /// Examples: `ownership`, `borrow`, `lifetime`.
    pub id: String,
    /// Kazakh display label. Used by future templates («Иелік
    /// тақырыбы аяқталды»).
    pub label_kk: String,
    /// Russian display label (informational; not currently surfaced —
    /// adam is Kazakh-only per `project_kazakh_only_directive`).
    #[serde(default)]
    pub label_ru: String,
    /// Stage IDs that must be **closed** (their `passed` count ≥
    /// `exercises_to_pass`) before this stage is recommended as the
    /// next step. Empty for the curriculum's entry point.
    #[serde(default)]
    pub prereqs: Vec<String>,
    /// Number of `SubmitSolution` passes required to close this
    /// stage. Once `progress.passed >= exercises_to_pass`, the stage
    /// is "closed" and prereqs of dependent stages are satisfied.
    pub exercises_to_pass: usize,
    /// Short Kazakh summary the planner can surface in
    /// `next_suggestion` templates. Optional.
    #[serde(default)]
    pub summary_kk: String,
}

/// The curriculum — an ordered list of stages with prereq edges.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Curriculum {
    pub version: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub stages: Vec<Stage>,
}

/// Per-stage progress on a single conversation. Mutated by
/// [`crate::Conversation`] after every `SubmitSolution` turn whose
/// `cargo_status` is `passed` or `failed`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageProgress {
    pub passed: usize,
    pub failed: usize,
}

impl StageProgress {
    /// `true` when the student has accumulated enough passes to
    /// close this stage according to its `exercises_to_pass` gate.
    pub fn is_closed(&self, stage: &Stage) -> bool {
        self.passed >= stage.exercises_to_pass
    }

    /// Increment passes by one.
    pub fn record_pass(&mut self) {
        self.passed = self.passed.saturating_add(1);
    }

    /// Increment failures by one.
    pub fn record_fail(&mut self) {
        self.failed = self.failed.saturating_add(1);
    }
}

/// **v4.98.5** — adaptive-difficulty hook. Coarse signal that future
/// content-selection layers ([`crate::pedagogical`] in v4.99.0+) can
/// consume to scale exercise difficulty per stage. v4.98.5 only ships
/// the API surface — exercise selection itself is not yet adaptive.
///
/// Threshold logic (deliberately simple):
/// - `Easy`     — student has failed ≥ 2 times → struggling, ease up.
/// - `Hard`     — student has 0 failures AND ≥ 1 pass → confident,
///                push harder.
/// - `Normal`   — everything else (initial state, mixed performance).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DifficultyHint {
    Easy,
    Normal,
    Hard,
}

impl StageProgress {
    /// Coarse difficulty signal for adaptive content selection. See
    /// [`DifficultyHint`] for the thresholds.
    pub fn difficulty_hint(&self) -> DifficultyHint {
        if self.failed >= 2 {
            DifficultyHint::Easy
        } else if self.failed == 0 && self.passed >= 1 {
            DifficultyHint::Hard
        } else {
            DifficultyHint::Normal
        }
    }
}

impl Curriculum {
    /// Try to load `DEFAULT_CURRICULUM_PATH`. Returns `Ok(None)` when
    /// the file is absent (trimmed checkout) — callers treat this as
    /// "curriculum disabled" rather than a hard error.
    pub fn load_default() -> std::io::Result<Option<Self>> {
        let path = workspace_path(DEFAULT_CURRICULUM_PATH);
        if !path.exists() {
            return Ok(None);
        }
        Self::load_from_path(&path).map(Some)
    }

    /// Load from an explicit path.
    pub fn load_from_path(path: &Path) -> std::io::Result<Self> {
        let raw = std::fs::read_to_string(path)?;
        serde_json::from_str(&raw)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Look up a stage by ID (case-insensitive on the id).
    pub fn stage(&self, id: &str) -> Option<&Stage> {
        let lower = id.to_lowercase();
        self.stages.iter().find(|s| s.id == lower)
    }

    /// Return the next unclosed stage whose prereqs are all closed.
    /// Returns `None` when the entire curriculum is complete.
    pub fn next_unlocked<'a>(
        &'a self,
        progress: &HashMap<String, StageProgress>,
    ) -> Option<&'a Stage> {
        self.stages.iter().find(|s| {
            let p = progress.get(&s.id).copied().unwrap_or_default();
            !p.is_closed(s)
                && s.prereqs.iter().all(|prereq_id| {
                    self.stage(prereq_id)
                        .map(|prereq| {
                            progress
                                .get(&prereq.id)
                                .copied()
                                .unwrap_or_default()
                                .is_closed(prereq)
                        })
                        .unwrap_or(false)
                })
        })
    }

    /// `true` when every stage's progress satisfies its closure gate.
    pub fn is_complete(&self, progress: &HashMap<String, StageProgress>) -> bool {
        self.stages.iter().all(|s| {
            progress
                .get(&s.id)
                .copied()
                .unwrap_or_default()
                .is_closed(s)
        })
    }
}

/// Resolve a workspace-relative path. Prefers `CARGO_MANIFEST_DIR`
/// when running tests (so `../../data/...` works from a crate
/// subdirectory), else falls back to the literal path (binary running
/// from the repo root).
fn workspace_path(rel: &str) -> PathBuf {
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        // adam-dialog/ → workspace root is two parents up.
        let p = PathBuf::from(manifest).join("../..").join(rel);
        if p.exists() {
            return p;
        }
    }
    PathBuf::from(rel)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Curriculum {
        Curriculum {
            version: "test".into(),
            name: "test".into(),
            description: String::new(),
            stages: vec![
                Stage {
                    id: "ownership".into(),
                    label_kk: "Иелік".into(),
                    label_ru: String::new(),
                    prereqs: vec![],
                    exercises_to_pass: 2,
                    summary_kk: String::new(),
                },
                Stage {
                    id: "borrow".into(),
                    label_kk: "Қарыз".into(),
                    label_ru: String::new(),
                    prereqs: vec!["ownership".into()],
                    exercises_to_pass: 2,
                    summary_kk: String::new(),
                },
                Stage {
                    id: "lifetime".into(),
                    label_kk: "Өмір кезеңі".into(),
                    label_ru: String::new(),
                    prereqs: vec!["ownership".into(), "borrow".into()],
                    exercises_to_pass: 1,
                    summary_kk: String::new(),
                },
            ],
        }
    }

    #[test]
    fn stage_lookup_is_case_insensitive() {
        let c = fixture();
        assert!(c.stage("ownership").is_some());
        assert!(c.stage("Ownership").is_some());
        assert!(c.stage("OWNERSHIP").is_some());
        assert!(c.stage("missing").is_none());
    }

    #[test]
    fn stage_progress_closure_threshold() {
        let stage = &fixture().stages[0]; // ownership, exercises_to_pass = 2
        let mut p = StageProgress::default();
        assert!(!p.is_closed(stage));
        p.record_pass();
        assert!(!p.is_closed(stage)); // 1 / 2
        p.record_pass();
        assert!(p.is_closed(stage)); // 2 / 2
        p.record_pass();
        assert!(p.is_closed(stage)); // 3 / 2 — still closed
    }

    #[test]
    fn next_unlocked_starts_with_zero_prereq_stage() {
        let c = fixture();
        let progress = HashMap::new();
        let next = c.next_unlocked(&progress).expect("entry stage exists");
        assert_eq!(next.id, "ownership");
    }

    #[test]
    fn next_unlocked_advances_when_prereq_closes() {
        let c = fixture();
        let mut progress = HashMap::new();
        progress.insert(
            "ownership".into(),
            StageProgress {
                passed: 2,
                failed: 0,
            },
        );
        let next = c.next_unlocked(&progress).expect("borrow unlocked");
        assert_eq!(next.id, "borrow");
    }

    #[test]
    fn next_unlocked_skips_unmet_prereqs() {
        let c = fixture();
        let mut progress = HashMap::new();
        // Skip ownership entirely; lifetime requires both ownership
        // AND borrow — neither closed.
        progress.insert(
            "borrow".into(),
            StageProgress {
                passed: 5,
                failed: 0,
            },
        );
        // Should still recommend ownership (prereqs are met for it
        // because it has none), not lifetime.
        let next = c.next_unlocked(&progress).expect("ownership still open");
        assert_eq!(next.id, "ownership");
    }

    #[test]
    fn next_unlocked_returns_none_when_complete() {
        let c = fixture();
        let mut progress = HashMap::new();
        for s in &c.stages {
            progress.insert(
                s.id.clone(),
                StageProgress {
                    passed: s.exercises_to_pass,
                    failed: 0,
                },
            );
        }
        assert!(c.next_unlocked(&progress).is_none());
        assert!(c.is_complete(&progress));
    }

    #[test]
    fn difficulty_hint_initial_state_is_normal() {
        let p = StageProgress::default();
        assert_eq!(p.difficulty_hint(), DifficultyHint::Normal);
    }

    #[test]
    fn difficulty_hint_easy_when_two_failures() {
        let p = StageProgress {
            passed: 0,
            failed: 2,
        };
        assert_eq!(p.difficulty_hint(), DifficultyHint::Easy);
        let p = StageProgress {
            passed: 1,
            failed: 3,
        };
        assert_eq!(p.difficulty_hint(), DifficultyHint::Easy);
    }

    #[test]
    fn difficulty_hint_hard_when_clean_passes() {
        let p = StageProgress {
            passed: 1,
            failed: 0,
        };
        assert_eq!(p.difficulty_hint(), DifficultyHint::Hard);
        let p = StageProgress {
            passed: 5,
            failed: 0,
        };
        assert_eq!(p.difficulty_hint(), DifficultyHint::Hard);
    }

    #[test]
    fn difficulty_hint_normal_for_mixed_below_easy_threshold() {
        let p = StageProgress {
            passed: 1,
            failed: 1,
        };
        assert_eq!(p.difficulty_hint(), DifficultyHint::Normal);
    }

    #[test]
    fn load_default_finds_committed_curriculum() {
        // The committed `data/dialog/curriculum/rust_progression.json`
        // must load and have at least the canonical 5 stages.
        let Some(c) = Curriculum::load_default().expect("load result") else {
            // Trimmed checkout — skip.
            return;
        };
        assert!(
            c.stages.len() >= 5,
            "expected ≥5 stages, got {}",
            c.stages.len()
        );
        assert!(c.stage("ownership").is_some());
        assert!(c.stage("borrow").is_some());
        assert!(c.stage("lifetime").is_some());
        assert!(c.stage("traits").is_some());
        assert!(c.stage("async").is_some());
        // Prereq invariant: every prereq ID resolves to an existing stage.
        for s in &c.stages {
            for prereq in &s.prereqs {
                assert!(
                    c.stage(prereq).is_some(),
                    "stage `{}` references missing prereq `{}`",
                    s.id,
                    prereq
                );
            }
        }
    }
}
