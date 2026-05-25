// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `FrameIndex` — **Stage 4 of v6.2.0 neurosymbolic redesign**.
//!
//! A typed in-memory index over [`Frame`]s. Takes a [`QueryIR`] and
//! returns ranked candidate frames in O(min-index-size) instead of
//! a linear O(N) scan.
//!
//! ## Why this layer exists
//!
//! Pre-v6.2 retrieval scanned `data/world_core/*.jsonl` linearly,
//! checking every fact against every query. At 4k facts that's
//! tolerable; at 50k it isn't, and the v6.1 architecture has no
//! typed index to lean on.
//!
//! `FrameIndex` brings the structured-indexing principle (see
//! the project's foundational directive on indexed knowledge) into
//! the algebra layer:
//!
//! - `by_predicate: predicate → frame_ids` for O(1) predicate
//!   restriction.
//! - `by_agent_root: agent.root.surface → frame_ids` for O(1)
//!   subject lookup.
//! - `by_object_root: object.root.surface → frame_ids` for O(1)
//!   object lookup.
//! - `by_modifier: (role, root.surface) → frame_ids` for time /
//!   location / source / instrument / manner / recipient /
//!   possessor constraints.
//! - `by_domain: Domain → frame_ids` for sense-filter dispatch.
//!
//! Lookup picks the smallest applicable index, intersects with the
//! next, and finally runs [`QueryIR::match_frame`] on the survivor
//! set to compute the [`FrameMatch`] score. Stage 3's `match_frame`
//! remains the truth oracle — the index is a *candidate filter*,
//! not a re-implementation of matching.
//!
//! ## Stage 4 scope
//!
//! - [`FrameIndex`] — the indexed store.
//! - [`FrameId`] — opaque u32 frame identifier.
//! - [`IndexedFrame`] — `(FrameId, Frame, Option<Domain>)` triple
//!   the index returns when listing.
//! - [`RankedFrame`] — `(FrameId, &Frame, FrameMatch)` triple
//!   returned by `query` / `best_match`.
//! - [`FrameIndex::insert`] — adds a frame, returns its FrameId,
//!   updates all secondary indexes.
//! - [`FrameIndex::query`] — `QueryIR → Vec<RankedFrame>`, sorted
//!   by score desc, frame_id asc.
//! - [`FrameIndex::best_match`] — single top-1 candidate.
//!
//! ## Predeclared success criterion
//!
//! The `query` result must equal a linear filter over all frames:
//! for any query and any insertion order, the set of returned
//! FrameIds must be identical to the brute-force scan. The
//! `index_equals_linear_filter` property test asserts this across
//! all 22 v6.1 [`FramePredicate`] variants and the 8
//! [`QueryFocus`] modes.
//!
//! ## NOT in Stage 4
//!
//! - Persistence — Stage 4 is in-memory only. Disk-backed indexes
//!   are a Stage 9 (ARM PoC) concern.
//! - Learned ranker — Stage 6 layers a typed-input ranker on top of
//!   the index's `score` field.
//! - Sense disambiguation policy — Stage 5 owns the rules for
//!   resolving competing [`SenseHint`]s; Stage 4 only mechanically
//!   applies the `domain_filter` constraint.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::frame::{Frame, Modifier};
use crate::query::{AnswerSlot, Domain, FrameMatch, QueryIR};

/// Opaque identifier for an inserted [`Frame`]. Stable for the
/// lifetime of the index — frame ids are not re-used on removal
/// (Stage 4 does not support removal; an index entry, once
/// created, lives until the index is dropped).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FrameId(pub u32);

/// A frame as stored in the index. The optional `domain` tags the
/// frame for sense-filter dispatch and Stage 5 sense
/// disambiguation; frames inserted without a domain are visible to
/// every query (no domain filter applied).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedFrame {
    pub id: FrameId,
    pub frame: Frame,
    pub domain: Option<Domain>,
}

/// A retrieval hit — the frame id, a borrow of the stored frame,
/// and the [`FrameMatch`] details from
/// [`QueryIR::match_frame`].
#[derive(Debug, Clone, PartialEq)]
pub struct RankedFrame<'a> {
    pub id: FrameId,
    pub frame: &'a Frame,
    pub domain: Option<&'a Domain>,
    pub match_result: FrameMatch,
}

/// Typed in-memory index over [`Frame`]s.
///
/// Insertion is O(M) in the number of secondary indexes the frame
/// participates in (≤ 12 in practice — 1 by_predicate slot, 1 by
/// agent root, 1 by object root, ≤ 7 modifier slots, 1 by_domain).
/// Query is O(K · S) where K is the smallest applicable index size
/// and S is the cost of [`QueryIR::match_frame`] (constant in
/// typical frames).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FrameIndex {
    /// All inserted frames in insertion order. `frames[i.0 as usize]`
    /// is the `IndexedFrame` for `FrameId(i)`.
    frames: Vec<IndexedFrame>,
    /// Predicate → frame ids.
    by_predicate: HashMap<String, Vec<FrameId>>,
    /// Agent root surface → frame ids.
    by_agent_root: HashMap<String, Vec<FrameId>>,
    /// Object root surface → frame ids.
    by_object_root: HashMap<String, Vec<FrameId>>,
    /// (modifier role slug, root surface) → frame ids.
    by_modifier: HashMap<(String, String), Vec<FrameId>>,
    /// Domain → frame ids. Frames without a domain are not listed
    /// here (a query with no domain_filter sees them anyway via
    /// the other indexes).
    by_domain: HashMap<String, Vec<FrameId>>,
}

impl FrameIndex {
    /// Construct an empty index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of frames in the index.
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// True iff no frames have been inserted yet.
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Retrieve a frame by id (panics on unknown id — frame ids are
    /// only produced by this index, so an unknown id is a logic bug).
    pub fn get(&self, id: FrameId) -> &IndexedFrame {
        &self.frames[id.0 as usize]
    }

    /// Insert a frame. Returns the assigned [`FrameId`] and updates
    /// every applicable secondary index.
    pub fn insert(&mut self, frame: Frame, domain: Option<Domain>) -> FrameId {
        let id = FrameId(self.frames.len() as u32);

        // Predicate index.
        self.by_predicate
            .entry(frame.predicate.as_str().to_string())
            .or_default()
            .push(id);

        // Agent index.
        if let Some(agent) = &frame.agent {
            self.by_agent_root
                .entry(agent.root.surface.clone())
                .or_default()
                .push(id);
        }

        // Object index.
        if let Some(object) = &frame.object {
            self.by_object_root
                .entry(object.root.surface.clone())
                .or_default()
                .push(id);
        }

        // Modifier index.
        for m in &frame.modifiers {
            if let Some(root) = modifier_root_surface(m) {
                self.by_modifier
                    .entry((m.role_str().to_string(), root))
                    .or_default()
                    .push(id);
            }
        }

        // Domain index.
        if let Some(d) = &domain {
            self.by_domain
                .entry(d.as_str().to_string())
                .or_default()
                .push(id);
        }

        self.frames.push(IndexedFrame { id, frame, domain });
        id
    }

    /// Retrieve all frames matching the query. Result is sorted by
    /// `score` descending, then `id` ascending (deterministic).
    ///
    /// Strategy:
    /// 1. Build a candidate set from the smallest applicable
    ///    secondary index.
    /// 2. Apply `domain_filter` by intersection with `by_domain` if
    ///    set.
    /// 3. For each candidate, run [`QueryIR::match_frame`] — the
    ///    Stage 3 truth oracle — and keep `Some(FrameMatch)` hits.
    /// 4. Sort and return.
    pub fn query(&self, q: &QueryIR) -> Vec<RankedFrame<'_>> {
        // 1. Pick the smallest applicable candidate set.
        let mut candidates = self.candidate_set(q);

        // 2. Domain filter.
        if let Some(d) = &q.domain_filter
            && let Some(domain_ids) = self.by_domain.get(d.as_str())
        {
            let domain_set: HashSet<FrameId> = domain_ids.iter().copied().collect();
            candidates.retain(|id| domain_set.contains(id));
        } else if q.domain_filter.is_some() {
            // domain_filter set but no frames tagged with that
            // domain → empty result.
            return Vec::new();
        }

        // 3. Run Stage 3 match_frame on each candidate.
        let mut hits: Vec<RankedFrame<'_>> = candidates
            .into_iter()
            .filter_map(|id| {
                let entry = self.get(id);
                q.match_frame(&entry.frame).map(|m| RankedFrame {
                    id,
                    frame: &entry.frame,
                    domain: entry.domain.as_ref(),
                    match_result: m,
                })
            })
            .collect();

        // 4. Sort: score desc, then id asc.
        hits.sort_by(|a, b| {
            b.match_result
                .score
                .cmp(&a.match_result.score)
                .then(a.id.cmp(&b.id))
        });
        hits
    }

    /// Single top-ranked candidate, or `None` if no frame matches.
    pub fn best_match(&self, q: &QueryIR) -> Option<RankedFrame<'_>> {
        self.query(q).into_iter().next()
    }

    /// Construct the initial candidate set for a query. Picks the
    /// smallest of the applicable secondary indexes to minimise the
    /// number of `match_frame` calls. When no constraint is set,
    /// falls back to all frame ids.
    fn candidate_set(&self, q: &QueryIR) -> Vec<FrameId> {
        let mut applicable: Vec<&Vec<FrameId>> = Vec::new();

        if let Some(p) = &q.predicate
            && let Some(v) = self.by_predicate.get(p.as_str())
        {
            applicable.push(v);
        }
        if let Some(a) = &q.agent
            && let Some(v) = self.by_agent_root.get(&a.root.surface)
        {
            applicable.push(v);
        }
        if let Some(o) = &q.object
            && let Some(v) = self.by_object_root.get(&o.root.surface)
        {
            applicable.push(v);
        }
        for mc in &q.modifier_constraints {
            if let Some(v) = self
                .by_modifier
                .get(&(mc.role.as_str().to_string(), mc.value.root.surface.clone()))
            {
                applicable.push(v);
            } else {
                // Modifier constraint refers to a (role, value) pair
                // not present in any frame → empty result.
                return Vec::new();
            }
        }

        if applicable.is_empty() {
            // No constraint to narrow on — scan all frames.
            return (0..self.frames.len() as u32).map(FrameId).collect();
        }

        // Pick the smallest set as the seed and intersect the rest
        // into a HashSet for O(1) lookup.
        applicable.sort_by_key(|v| v.len());
        let seed = applicable[0];
        let rest: Vec<HashSet<FrameId>> = applicable[1..]
            .iter()
            .map(|v| v.iter().copied().collect())
            .collect();

        seed.iter()
            .copied()
            .filter(|id| rest.iter().all(|s| s.contains(id)))
            .collect()
    }
}

/// Return the surface form of the inner composition of a modifier,
/// if the modifier carries one. Mirrors the helper in
/// [`crate::query`] but doesn't clone — we only need the surface
/// string for indexing.
fn modifier_root_surface(m: &Modifier) -> Option<String> {
    match m {
        Modifier::TimeAnchor(crate::frame::TimeAnchor::Phrase(c)) => Some(c.root.surface.clone()),
        Modifier::TimeAnchor(_) => None,
        Modifier::Location(c)
        | Modifier::Source(c)
        | Modifier::Instrument(c)
        | Modifier::Manner(c)
        | Modifier::Recipient(c)
        | Modifier::Possessor(c) => Some(c.root.surface.clone()),
    }
}

/// Convenience accessor — what slot does this match return?
impl<'a> RankedFrame<'a> {
    pub fn answer_slot(&self) -> AnswerSlot {
        self.match_result.answer_slot
    }
    pub fn score(&self) -> u8 {
        self.match_result.score
    }
}

/// Linear-filter baseline for property tests. Iterates every frame
/// in the index and runs [`QueryIR::match_frame`] — the brute-force
/// O(N) equivalent of [`FrameIndex::query`]. Returns the same
/// sort order. Used in tests to assert that the index returns
/// identical results.
#[doc(hidden)]
pub fn linear_filter<'a>(idx: &'a FrameIndex, q: &QueryIR) -> Vec<RankedFrame<'a>> {
    let mut hits: Vec<RankedFrame<'_>> = idx
        .frames
        .iter()
        .filter_map(|e| {
            // Apply domain filter manually.
            if let Some(d) = &q.domain_filter {
                if e.domain.as_ref() != Some(d) {
                    return None;
                }
            }
            q.match_frame(&e.frame).map(|m| RankedFrame {
                id: e.id,
                frame: &e.frame,
                domain: e.domain.as_ref(),
                match_result: m,
            })
        })
        .collect();
    hits.sort_by(|a, b| {
        b.match_result
            .score
            .cmp(&a.match_result.score)
            .then(a.id.cmp(&b.id))
    });
    hits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composition::Composition;
    use crate::frame::{Frame, FramePredicate, Modality, Modifier, QuestionFocus, TimeAnchor};
    use crate::operator::SuffixOp;
    use crate::query::{AnswerShape, ModifierRole, QueryFocus, QueryIR, QuestionForm};
    use crate::root::{PartOfSpeech, Root};
    use adam_kernel_fst::morphotactics::{Case, Tense};

    fn noun(s: &str) -> Composition {
        Composition::identity(Root::new(s, PartOfSpeech::Noun))
    }

    fn noun_with_case(s: &str, c: Case) -> Composition {
        let mut x = noun(s);
        x.operators.push(SuffixOp::Case(c));
        x
    }

    fn year_phrase(year: &str) -> Composition {
        let mut c = noun(year);
        c.operators.push(SuffixOp::Case(Case::Locative));
        c
    }

    fn ahmet_born_1872() -> Frame {
        Frame::assertion(
            Some(noun("ахмет байтұрсынұлы")),
            FramePredicate::BornIn,
            None,
        )
        .with_modifier(Modifier::TimeAnchor(TimeAnchor::Phrase(year_phrase(
            "1872 жыл",
        ))))
        .with_modifier(Modifier::Location(noun_with_case(
            "қостанай",
            Case::Locative,
        )))
        .with_tense(Tense::PastEvidential)
    }

    fn abay_born_1845() -> Frame {
        Frame::assertion(Some(noun("абай")), FramePredicate::BornIn, None)
            .with_modifier(Modifier::TimeAnchor(TimeAnchor::Phrase(year_phrase(
                "1845 жыл",
            ))))
            .with_tense(Tense::PastEvidential)
    }

    fn kazakhstan_isa_state() -> Frame {
        Frame::assertion(
            Some(noun("қазақстан")),
            FramePredicate::IsA,
            Some(noun("мемлекет")),
        )
    }

    fn populate_basic() -> FrameIndex {
        let mut idx = FrameIndex::new();
        idx.insert(ahmet_born_1872(), Some(Domain::Person));
        idx.insert(abay_born_1845(), Some(Domain::Person));
        idx.insert(kazakhstan_isa_state(), Some(Domain::Geography));
        idx
    }

    // -- Insertion + basic state -------------------------------

    #[test]
    fn insert_assigns_sequential_ids() {
        let mut idx = FrameIndex::new();
        let a = idx.insert(ahmet_born_1872(), None);
        let b = idx.insert(abay_born_1845(), None);
        let c = idx.insert(kazakhstan_isa_state(), None);
        assert_eq!(a, FrameId(0));
        assert_eq!(b, FrameId(1));
        assert_eq!(c, FrameId(2));
        assert_eq!(idx.len(), 3);
        assert!(!idx.is_empty());
    }

    #[test]
    fn empty_index_returns_empty_results() {
        let idx = FrameIndex::new();
        let q = QueryIR::new(
            QueryFocus::Subject,
            QuestionForm::Definition,
            AnswerShape::BareNoun,
        )
        .with_predicate(FramePredicate::BornIn);
        assert!(idx.query(&q).is_empty());
        assert!(idx.best_match(&q).is_none());
    }

    // -- Predicate-driven query --------------------------------

    #[test]
    fn query_by_predicate_returns_all_matches() {
        let idx = populate_basic();
        // «Кім туылған?»
        let q = QueryIR::new(
            QueryFocus::Subject,
            QuestionForm::Definition,
            AnswerShape::BareNoun,
        )
        .with_predicate(FramePredicate::BornIn);
        let hits = idx.query(&q);
        assert_eq!(hits.len(), 2);
        let ids: Vec<FrameId> = hits.iter().map(|h| h.id).collect();
        assert!(ids.contains(&FrameId(0)));
        assert!(ids.contains(&FrameId(1)));
    }

    #[test]
    fn query_by_agent_returns_just_that_agent() {
        let idx = populate_basic();
        // «Ахмет қашан туылған?»
        let q = QueryIR::new(
            QueryFocus::Modifier(ModifierRole::Time),
            QuestionForm::Definition,
            AnswerShape::DateAnchor,
        )
        .with_agent(noun("ахмет байтұрсынұлы"))
        .with_predicate(FramePredicate::BornIn);
        let hits = idx.query(&q);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, FrameId(0));
        assert_eq!(
            hits[0].match_result.answer_slot,
            AnswerSlot::Modifier(ModifierRole::Time)
        );
    }

    #[test]
    fn query_by_modifier_constraint_narrows_correctly() {
        let idx = populate_basic();
        // «Кім 1872 жылы туылған?»
        let q = QueryIR::new(
            QueryFocus::Subject,
            QuestionForm::Definition,
            AnswerShape::BareNoun,
        )
        .with_predicate(FramePredicate::BornIn)
        .with_modifier_constraint(ModifierRole::Time, year_phrase("1872 жыл"));
        let hits = idx.query(&q);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, FrameId(0));
    }

    #[test]
    fn query_with_unknown_modifier_value_returns_empty() {
        let idx = populate_basic();
        let q = QueryIR::new(
            QueryFocus::Subject,
            QuestionForm::Definition,
            AnswerShape::BareNoun,
        )
        .with_predicate(FramePredicate::BornIn)
        .with_modifier_constraint(ModifierRole::Time, year_phrase("9999 жыл"));
        assert!(idx.query(&q).is_empty());
    }

    // -- Domain / sense filter ---------------------------------

    #[test]
    fn domain_filter_narrows_to_tagged_frames() {
        let idx = populate_basic();
        let q = QueryIR::new(
            QueryFocus::Subject,
            QuestionForm::Definition,
            AnswerShape::BareNoun,
        )
        .with_predicate(FramePredicate::BornIn)
        .with_domain_filter(Domain::Person);
        let hits = idx.query(&q);
        assert_eq!(hits.len(), 2);

        // Geography-domain filter on a BornIn predicate → no hits
        // (the Geography fact is IsA, not BornIn; the predicate
        // filter alone already produces no matches).
        let q2 = QueryIR::new(
            QueryFocus::Object,
            QuestionForm::Definition,
            AnswerShape::DefinitionalNP,
        )
        .with_predicate(FramePredicate::IsA)
        .with_domain_filter(Domain::Geography);
        let hits2 = idx.query(&q2);
        assert_eq!(hits2.len(), 1);
        assert_eq!(hits2[0].id, FrameId(2));
    }

    #[test]
    fn domain_filter_unknown_domain_returns_empty() {
        let idx = populate_basic();
        let q = QueryIR::new(
            QueryFocus::Subject,
            QuestionForm::Definition,
            AnswerShape::BareNoun,
        )
        .with_predicate(FramePredicate::BornIn)
        .with_domain_filter(Domain::Astronomy);
        assert!(idx.query(&q).is_empty());
    }

    #[test]
    fn sense_filter_disambiguates_ay_month_vs_moon() {
        // «Ай» appears in two senses: calendar (month) and
        // astronomy (moon). A domain-filtered query must surface
        // only the matching sense.
        let mut idx = FrameIndex::new();
        let ay_month = Frame::assertion(
            Some(noun("ай")),
            FramePredicate::IsA,
            Some(noun("уақыт_өлшемі")),
        );
        let ay_moon = Frame::assertion(
            Some(noun("ай")),
            FramePredicate::IsA,
            Some(noun("аспан_денесі")),
        );
        idx.insert(ay_month, Some(Domain::Calendar));
        idx.insert(ay_moon, Some(Domain::Astronomy));

        let q_cal = QueryIR::new(
            QueryFocus::Object,
            QuestionForm::Definition,
            AnswerShape::DefinitionalNP,
        )
        .with_predicate(FramePredicate::IsA)
        .with_agent(noun("ай"))
        .with_domain_filter(Domain::Calendar);
        let hits = idx.query(&q_cal);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, FrameId(0));

        let q_astro = QueryIR::new(
            QueryFocus::Object,
            QuestionForm::Definition,
            AnswerShape::DefinitionalNP,
        )
        .with_predicate(FramePredicate::IsA)
        .with_agent(noun("ай"))
        .with_domain_filter(Domain::Astronomy);
        let hits = idx.query(&q_astro);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, FrameId(1));
    }

    // -- Property: index === linear filter --------------------

    #[test]
    fn index_equals_linear_filter_on_basic_population() {
        let idx = populate_basic();
        let queries = [
            QueryIR::new(
                QueryFocus::Subject,
                QuestionForm::Definition,
                AnswerShape::BareNoun,
            )
            .with_predicate(FramePredicate::BornIn),
            QueryIR::new(
                QueryFocus::Modifier(ModifierRole::Time),
                QuestionForm::Definition,
                AnswerShape::DateAnchor,
            )
            .with_agent(noun("ахмет байтұрсынұлы"))
            .with_predicate(FramePredicate::BornIn),
            QueryIR::new(
                QueryFocus::Modifier(ModifierRole::Location),
                QuestionForm::Definition,
                AnswerShape::BareNoun,
            )
            .with_agent(noun("ахмет байтұрсынұлы"))
            .with_predicate(FramePredicate::BornIn),
            QueryIR::new(
                QueryFocus::Subject,
                QuestionForm::Definition,
                AnswerShape::BareNoun,
            )
            .with_predicate(FramePredicate::BornIn)
            .with_modifier_constraint(ModifierRole::Time, year_phrase("1872 жыл")),
            QueryIR::new(
                QueryFocus::Object,
                QuestionForm::Definition,
                AnswerShape::DefinitionalNP,
            )
            .with_predicate(FramePredicate::IsA),
        ];
        for q in queries {
            assert_query_equals_linear(&idx, &q);
        }
    }

    #[test]
    fn index_equals_linear_filter_on_synthetic_1k() {
        // Generate 1000 frames spanning all 22 predicates with a
        // mix of agents / objects / time-modifiers, then verify
        // every targeted query returns the same set as a brute-
        // force linear filter.
        let idx = synthetic_index(1000);

        let queries = [
            QueryIR::new(
                QueryFocus::Subject,
                QuestionForm::Definition,
                AnswerShape::BareNoun,
            )
            .with_predicate(FramePredicate::BornIn),
            QueryIR::new(
                QueryFocus::Subject,
                QuestionForm::Definition,
                AnswerShape::BareNoun,
            )
            .with_predicate(FramePredicate::Authored),
            QueryIR::new(
                QueryFocus::Modifier(ModifierRole::Time),
                QuestionForm::Definition,
                AnswerShape::DateAnchor,
            )
            .with_agent(noun("agent_5"))
            .with_predicate(FramePredicate::BornIn),
            QueryIR::new(
                QueryFocus::Subject,
                QuestionForm::Definition,
                AnswerShape::BareNoun,
            )
            .with_predicate(FramePredicate::BornIn)
            .with_modifier_constraint(ModifierRole::Time, year_phrase("year_7")),
            QueryIR::new(
                QueryFocus::Subject,
                QuestionForm::Definition,
                AnswerShape::BareNoun,
            )
            .with_predicate(FramePredicate::IsA)
            .with_domain_filter(Domain::Geography),
        ];
        for q in queries {
            assert_query_equals_linear(&idx, &q);
        }
    }

    fn assert_query_equals_linear(idx: &FrameIndex, q: &QueryIR) {
        let indexed = idx.query(q);
        let linear = linear_filter(idx, q);
        // Compare by (id, score, answer_slot) — the rest of
        // RankedFrame is a borrow that doesn't impl PartialEq.
        let indexed_keys: Vec<_> = indexed
            .iter()
            .map(|h| (h.id, h.match_result.score, h.match_result.answer_slot))
            .collect();
        let linear_keys: Vec<_> = linear
            .iter()
            .map(|h| (h.id, h.match_result.score, h.match_result.answer_slot))
            .collect();
        assert_eq!(
            indexed_keys, linear_keys,
            "indexed.query must equal linear_filter for the same query"
        );
    }

    /// Build a deterministic 1000-frame test index spanning all 22
    /// FramePredicate variants. Used by the property tests + the
    /// bench scenario.
    fn synthetic_index(n: usize) -> FrameIndex {
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
        let domains = [
            Domain::Person,
            Domain::Geography,
            Domain::Institution,
            Domain::Event,
            Domain::Science,
        ];
        let mut idx = FrameIndex::new();
        for i in 0..n {
            let predicate = preds[i % preds.len()].clone();
            let agent_root = format!("agent_{}", i % 100);
            let object_root = format!("object_{}", i % 50);
            let year_root = format!("year_{}", i % 30);
            let frame =
                Frame::assertion(Some(noun(&agent_root)), predicate, Some(noun(&object_root)))
                    .with_modifier(Modifier::TimeAnchor(TimeAnchor::Phrase(year_phrase(
                        &year_root,
                    ))));
            let domain = domains[i % domains.len()].clone();
            idx.insert(frame, Some(domain));
        }
        idx
    }

    // -- 22 predicates × index coverage -----------------------

    #[test]
    fn every_v6_1_predicate_indexes_and_retrieves() {
        let idx = synthetic_index(440); // 20 frames per predicate
        let all = [
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
        for p in all {
            let q = QueryIR::new(
                QueryFocus::Subject,
                QuestionForm::Definition,
                AnswerShape::BareNoun,
            )
            .with_predicate(p.clone());
            let hits = idx.query(&q);
            assert_eq!(hits.len(), 20, "predicate {:?} has wrong hit count", p);
        }
    }

    // -- best_match top-1 ------------------------------------

    #[test]
    fn best_match_returns_highest_score() {
        // Two frames where one is an exact match (score 100) and
        // one is a partial match (score 50). best_match must
        // return the exact-match one.
        let mut idx = FrameIndex::new();
        let exact = ahmet_born_1872();
        let partial = Frame::assertion(
            Some(noun("ахмет байтұрсынұлы")),
            FramePredicate::BornIn,
            None,
        );
        idx.insert(exact, None);
        idx.insert(partial, None);

        let q = QueryIR::new(
            QueryFocus::Modifier(ModifierRole::Time),
            QuestionForm::Definition,
            AnswerShape::DateAnchor,
        )
        .with_agent(noun("ахмет байтұрсынұлы"))
        .with_predicate(FramePredicate::BornIn);

        let top = idx.best_match(&q).expect("at least one hit");
        assert_eq!(top.id, FrameId(0));
        assert_eq!(top.match_result.score, 100);
    }

    #[test]
    fn results_are_sorted_score_desc_then_id_asc() {
        // Two frames at identical score 100 — id-ascending tie
        // break.
        let mut idx = FrameIndex::new();
        idx.insert(ahmet_born_1872(), None);
        let mut second = ahmet_born_1872();
        // Swap the order so we can verify id-ordering — frame
        // contents identical, FrameId 1 should appear after 0.
        second.evidentiality = crate::frame::Evidentiality::Direct;
        idx.insert(second, None);
        let q = QueryIR::new(
            QueryFocus::Modifier(ModifierRole::Time),
            QuestionForm::Definition,
            AnswerShape::DateAnchor,
        )
        .with_agent(noun("ахмет байтұрсынұлы"))
        .with_predicate(FramePredicate::BornIn);
        let hits = idx.query(&q);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].id, FrameId(0));
        assert_eq!(hits[1].id, FrameId(1));
    }

    // -- Stage-3 / Stage-4 integration test -------------------

    #[test]
    fn frame_to_query_to_index_end_to_end() {
        let idx = populate_basic();
        // User asks: «Ахмет Байтұрсынұлы қашан туылған?»
        let q_frame = Frame::assertion(
            Some(noun("ахмет байтұрсынұлы")),
            FramePredicate::BornIn,
            None,
        )
        .with_modality(Modality::Question {
            focus: QuestionFocus::Time,
        });
        let q = QueryIR::from_question_frame(&q_frame).expect("query");
        let hits = idx.query(&q);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, FrameId(0));
        // The realiser would now read the time modifier of the
        // matched frame and emit «1872 жылы».
        let time_mod = hits[0]
            .frame
            .modifier("time")
            .expect("indexed frame must carry the time modifier");
        if let Modifier::TimeAnchor(TimeAnchor::Phrase(c)) = time_mod {
            assert_eq!(c.root.surface, "1872 жыл");
        } else {
            panic!("expected phrase-kind time anchor");
        }
    }

    // -- Serde round-trip -------------------------------------

    #[test]
    fn indexed_frame_serde_round_trip() {
        let mut idx = FrameIndex::new();
        idx.insert(ahmet_born_1872(), Some(Domain::Person));
        let entry = idx.get(FrameId(0)).clone();
        let json = serde_json::to_string(&entry).expect("ser");
        let back: IndexedFrame = serde_json::from_str(&json).expect("de");
        assert_eq!(entry, back);
    }
}
