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

## Day 3 — 2026-05-07 (Thursday)

**v4.53.5 — context-aware clarify (replaces generic «сұрағыңызды
нақтылай түсіңізші»).**

User's session-5 complaint: "ответы должны вытекать из контекста
беседы". When adam can't recognise an intent, the response should
mention WHAT specifically wasn't understood, not a catch-all.

- New planner branch: when intent = `Unknown` AND session has stored
  slots (name / occupation / activity / location), surface the
  diagnostic: «Сұрағыңыздағы X сөзін мен әлі түсінбедім — Y туралы
  айтып жатырсыз ба?» where X = the unknown noun-hint, Y = the
  best-matching stored slot.
- New template family `unknown.with_session_diagnostic` (3-4 variants).
- Trace addition: emit `clarify_diagnostic` line listing the
  unrecognised tokens + which slot the planner offered as a guess.

Acceptance: session-5 line «Сондай дерек естідіңіз бе, иә?» becomes
something like «Контексттегі қандай мәліметті айтып отырсыз — атыңыз,
мамандығыңыз, әлде ағымдағы ісіңіз бе?».

---

## Day 4 — 2026-05-08 (Friday)

**Live REPL session 6 — fresh transcript-driven gap report.**

- Run a 10-15-turn novel Kazakh dialog covering: math chain (digit
  + word + gerund), name + occupation + activity capture, recall
  queries across all three slots, knowledge questions on world_core
  (geography, science, programming), follow-up turns testing
  context continuity.
- Capture every adam response; flag every gap with a numbered ID.
- Write findings into `docs/transcripts/session_06_2026-05-08.md`:
  per gap, what input triggered it, what response came back, what
  the desired response is, suggested fix scope.
- Record p50 latency + max RSS per turn (per `feedback_unified_kpi`).

No release. Output: gap report becomes the v4.54 / v4.55 spec.

---

## Day 5 — 2026-05-09 (Saturday)

**v4.54.0 — Lexicon + bridge-fact expansion (highest-ROI batch).**

Per `project_bridge_fact_leverage`: a single
`{sub_hub} IsA {адам/маман/зат/...}` fact multiplies leverage to
+47–67 derivations vs +15 without bridge.

- Top 3 missing morphemes (per `project_morpheme_coverage_baseline`
  v4.41.0 baseline): алғашқы, іске, млн. Add roots + suffix paradigms.
- Bridge-fact pass: identify 10 sub-hub nouns currently lacking IsA
  edges to a top-tier hub (адам / маман / зат / жер бедері / су
  денесі / етістік / ұғым). Add bridge facts.
- Per `project_corpus_purity_directive`: every new entry passes
  loanword filter; reject Russian/English-rooted candidates without
  established Kazakh form.
- Re-run `extract_facts` + `run_reasoner` + `validate_foundation` to
  confirm derivation count moves up by ≥ 30/fact added.

Acceptance: morpheme coverage > 86.5 % (currently 86.21 %); derived
facts count up by ≥ 300 from current 27175.

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
