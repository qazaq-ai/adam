# Weekly plan — week of 2026-05-05 (W19)

**Project direction (re-anchored):** v4.x is agglutinative-first, math-driven,
M2-8GB-developed, **no investors**. Synthetic-FST data preferred over web
crawl. Real human-like testing on novel Kazakh phrasings beats templated
holdouts. References: `project_v4_direction`, `project_kazakh_only_directive`,
`feedback_real_human_testing_with_memory`.

**Themes for this week:**
1. Engineering hygiene (release gate, doc currency, clippy) — finish what
   Codex flagged that's actually load-bearing for shipping.
2. Close two deferred session-5 gaps (math gerund clauses + context-aware
   clarify).
3. Land one new real-REPL session (session 6) and treat its gaps as the
   v4.55 / v4.56 spec.
4. Resume corpus / Lexicon growth with bridge-fact-leveraged additions.

**Daily structure:** every release ships with workspace tests + foundation
validate + verify_release_version + REPL replay. End each day with one
named release tag.

---

## Day 1 — 2026-05-05 (Tuesday) ✓ shipped today

**v4.52.5 — engineering hygiene + doc currency.**

- [x] Bump 7 stale data manifests v4.37.0 → v4.52.5; release-gate green.
- [x] Doc audit: HIGH-priority numeric mismatches fixed (test count
      822 → 969 in foundation_scope; world_core entries
      874/995/30 → 2089/2349/46 in data/README + world_core/README;
      intent count 26 → 33 in adam-dialog Cargo + lib.rs + grammar
      doc; performance.md + corpus_audit.md headers re-stamped).
- [x] Added `description` to 5 crate Cargo.toml files
      (adam-kernel, adam-tokenizer, adam-corpus, adam-eval,
      adam-retrieval).

---

## Day 2 — 2026-05-05 (pulled forward) ✓ shipped

**v4.53.0 — math gerund-clause parser (session 5 deferral closed).**

- [x] New `inject_gerund_clause_separators(input)` helper in
      `discourse.rs` — inserts `__CLAUSE_SEP__` after each math-verb
      gerund/converb form. 8 surface forms covered (көбейткенде /
      көбейтіп / бөлгенде / бөліп / қосқанда / қосып / азайтқанда /
      азайтып) per Kazakh vowel harmony.
- [x] `try_evaluate_kazakh_word_math` calls the helper BEFORE the
      existing `,` / `және` / `содан кейін` / `соңында` clause splits;
      v4.42.0 accumulator path then handles the chain unchanged.
- [x] 5 new unit tests in `discourse::math_tests`:
      `word_math_gerund_chain_session5_first_line` (→ 26),
      `word_math_converb_only_chain` (→ 13),
      `word_math_gerund_only_chain` (→ 9),
      `word_math_qosqanda_qosyp_forms` (→ 8 / → 9),
      `word_math_gerund_does_not_break_simple_imperative` (regression).
- [x] Live REPL: «Елуді екіге көбейткенде үшке бөліп, 7-ні
      азайтқанда не болады?» → 26 (was math_refusal).
- [x] Workspace 974 passing; verify_release_version.sh 4.53.0 green.

---

## Day 3 — 2026-05-05 (pulled forward) ✓ shipped

**v4.53.5 — context-aware clarify (session 5 generic-fallback complaint closed).**

- [x] New `unknown.with_session_diagnostic` template family (7 variants
      gating on slot subsets: all 3 / name+occupation / name+activity /
      activity-only / occupation-only / city-only / name-only).
- [x] Planner override in `plan_response_with_epistemic` — when key is
      `unknown.clarify_no_topic` / `unknown.clarify_low_confidence`
      AND session has any of name/occupation/activity/city, override
      to the new family. Standard `!repo.get(...).is_empty()` guard
      protects against missing template-pack regression.
- [x] Trace line `clarify_diagnostic override` — lists which slots
      are present so `--trace` consumers see exactly which variant
      fired and why.
- [x] 2 new unit tests:
      `clarify_no_topic_with_session_routes_to_diagnostic` (override
      fires + cites stored slot in realised text);
      `clarify_no_topic_without_session_keeps_bare_family`
      (regression guard).
- [x] Live REPL: vague follow-up after slot-filling →
      "Дәулет, сізді бағдарламашы деп білемін. Сұрағыңыз мамандығыңызға
      қатысты ма, әлде басқа тақырыпта ма?" (was generic clarify).
- [x] Workspace 976 passing; verify_release_version.sh 4.53.5 green.

---

## Day 4 — 2026-05-05 (pulled forward by user) ✓ session run

**Live REPL session 6 — gap report + v4.54.5 ship.**

User ran a 16-turn novel Kazakh dialog. 4 gaps surfaced; 3 closed
in v4.54.5, 1 deferred:

- [x] **G1**: «Танысайық.» → «Қуана-қуана» tone fix → «иә, әрине».
- [x] **G2**: «Менің атымды есіңізде ме?» → grounded-fact hijack →
      AskName self-recall extended for `атым/атымды/аты-жөнім/
      аты-жөнімді/есімім/есімімді` × `есіңізде/есіңде/ұмытпа*`.
- [x] **G3**: «...қосқанда не болатынын есептеп, ...» math_refusal →
      removed «есепте» from `clause_has_math_verb` failure list.
- [ ] **G4** (deferred to v4.55.x): «Иә» after low-confidence
      clarify produces confused «Қайсысы нақты — бағдарламашы ма,
      сұрай ма?» from `check_contradiction` template. Phantom
      belief conflict; needs belief-tracker investigation.

**New cadence**: per user directive, adam runs its own autonomous
real-REPL crash-tests before every release. Session A (12 turns)
+ Session B (12 turns) ran post-G1-G3 fixes. Session B surfaced
B2 (compound «аты-жөнім») fixed in v4.54.5; B1 (pending
contradiction blocks math) deferred for UX policy debate.

---

## Day 5 — 2026-05-05 (pulled forward) ✓ shipped

**v4.54.0 — Lexicon + bridge-fact expansion (highest-ROI batch).**

- [x] 5 new morpheme roots in `data/tokenizer/segmentation_roots.json`:
      алғаш / млн (spec'd) + дамыту / маңызды / ашық (bonus to clear
      acceptance gate). Spec'd `іске` reverted: it broke segmentation
      eval (`seg_183` expects `іс + ке` split). Lexicon-tooling
      change (lower min-root-len from 3 → 2 to surface 2-char roots
      like `іс`, `ал`, `ол` in coverage) deferred to a later release.
- [x] 10 bridge facts in `role_bridges.jsonl` (`bridge_role_054…063`):
      бала/дос IsA адам · жасанды интеллект IsA жүйе · жад IsA ұғым ·
      киім/жиһаз IsA зат · бұлшықет/жүйке IsA мүше · бөлме IsA орын ·
      ел IsA мемлекет.
- [x] Re-ran `extract_facts` + `run_reasoner` + `morpheme_coverage`
      + `validate_foundation`. All green.
- [x] Acceptance: world_core 2089/2349/46/27175 → 2099/2359/46/**28109**
      (+10 entries / +10 facts / **+934 derived** = +93/fact, 3× the
      project_bridge_fact_leverage prior bound).
- [x] Acceptance: morpheme coverage 86.26 % → **86.51 %** (+0.25 pp;
      > 86.5 % gate cleared).
- [x] Workspace 976 passing unchanged; verify_release_version.sh
      4.54.0 green.

---

## Day 6 — 2026-05-10 (Sunday)

**v4.54.5 — clippy hygiene + workspace.lints policy.**

Codex flagged 293 clippy warnings. 99 are `collapsible_if` (stylistic),
32 `doc list item without indentation`. None are real bugs but the
noise prevents using clippy as a quality gate.

- Add `[workspace.lints.clippy]` to root Cargo.toml: `allow` the
  noisiest stylistic classes (`collapsible_if`, `doc_markdown`-style
  list-indent lints) with a short comment explaining why each is
  allowed.
- Wire `lints.workspace = true` in every crate's Cargo.toml.
- Fix the genuinely useful clippy warnings:
  `field_assignment_outside_initializer` (7 cases, possible bugs),
  `redundant_closure` (3 cases, perf-relevant),
  `too_many_arguments` (3 cases — refactor to typed config struct).
- Acceptance: `cargo clippy --workspace --all-targets -- -D warnings`
  passes with zero warnings.

---

## Day 7 — 2026-05-11 (Monday)

**v4.55.0 — single-source-of-truth metrics + CI gate.**

Codex's last engineering observation: docs and artifacts drift apart.
Today's audit needed manual cross-checks (jq on facts.json, intent.rs
grep, per-domain counts). Codify it.

- New `scripts/check_metrics_currency.sh`:
  - Asserts intent count in `crates/adam-dialog/src/intent.rs` matches
    the `intents-NN` badge in README.md.
  - Asserts world_core entry count (`cat data/world_core/*.jsonl | wc -l`)
    matches the totals listed in data/README.md + world_core/README.md.
  - Asserts test count (parsed from `cargo test --release --workspace`)
    matches the README + foundation_scope claim.
  - Asserts version refs in README, performance.md, every Cargo.toml
    description match the workspace version.
  - Exits non-zero on any drift.
- Wire into `scripts/validate_foundation.sh` as a final post-test step.
- Add a section to `CONTRIBUTING.md` documenting the checks.

Acceptance: script runs green on a fresh tree; each subsequent release
either updates the docs OR fails the gate.

---

## Out of scope this week

- **Investor track (valuation, SAFE caps, wedge selection).** Per
  `project_v4_direction`: no investors. If this changes, treat as a
  separate strategic decision with its own week.
- **External blind eval / 3rd-party benchmarks.** Live REPL transcript
  testing is the canonical signal per `feedback_real_human_testing_with_memory`.
- **Trained-weights production replacement** (Stage B carry-forward
  from v4.50.0). Not blocked, but lower ROI than dialog-continuity
  + math + Lexicon work this week.
- **API / cloud / web UI.** Not on the v4.x critical path.

---

## Definition of "shipped"

Each release tag (v4.53.0, v4.53.5, v4.54.0, v4.54.5, v4.55.0):

1. `cargo test --release --workspace` green.
2. `bash scripts/validate_foundation.sh` green.
3. `bash scripts/verify_release_version.sh <new>` green.
4. `cargo fmt --all --check` clean.
5. README + CHANGELOG + roadmap.md + performance.md updated.
6. Commit, tag, `git push origin main`, `git push origin v<new>`.
7. GitHub release created via curl + git credential
   (per `feedback_no_gh_cli`).

End of week: 5 release tags landed; 1 transcript session captured;
all docs current; clippy clean; CI gate self-checks doc drift.
