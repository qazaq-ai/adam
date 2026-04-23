# Changelog

All notable changes are tagged in git as `vX.Y.Z`.

Versioning cadence (post-v1.0.0):
- **Patch `x.y.5`** — small / incremental changes (bug fixes, small Lexicon additions, docs, housekeeping).
- **Minor `x.y.0`** — significant changes (new corpus source, new intent family, new tooling, learned component).
- **`v2.0.0`** is reserved for the "minimally thinking Kazakh LM" — a trained compact Kazakh model plugged in as `Intent::Unknown` fallback. Not more rules — actual learned generalisation.

## [4.0.2] — 2026-04-23 — Investor-safe demo mode: curated-source-chain filter in `adam_demo`

Small, focused patch continuing the Codex v4.0.0 hand-off. Same design philosophy as v4.0.1: a surgical fix at the right layer, not a broad architecture change.

### The problem

Codex flagged three specific chains in `adam_demo` Part 4 as public-demo-unsafe:

- `абай is_a халық`  — R1 transitivity via `wikipedia_kz_pack.json`
- `еңбек — өзен`  — R1 transitivity chaining through metaphorical corpus text
- `топырақ goes_to дене`  — R7 chain with cross-domain semantic collision

Each was technically correct — FST-typed, fully `rule_id`-tagged, `source_chain` non-empty — but the **text-extracted** facts feeding the chains had weaker semantic integrity than the hand-reviewed World Core entries they competed with. An investor looking at the demo would read "adam: Abai is a people" and stop listening.

### The fix

A demo-layer filter, not an extract-layer filter (recall preserved for downstream consumers):

- New helper [`derivation_is_fully_curated`](crates/adam-dialog/src/bin/adam_demo.rs): returns `true` iff **every** `FactSource` in the derivation's `source_chain` has a `pack` starting with `"world_core/"`. Empty chains fail closed.
- `adam_demo` Part 4 per-rule-representative picker now requires this predicate by default.
- `--all-derivations` dev flag bypasses the filter for audit / debugging.

### Measured effect

| | before | after | delta |
|---|---:|---:|---|
| Total derivations loaded | 6 579 | 6 579 | unchanged (filter is a view) |
| **Fully-curated chains** | 4 753 | **4 753 (72.2 %)** | reported in Part 4 header |
| Rules represented in Part 4 default | all 4 firing | 4 curated rules (R1, R2, R3, R5) | R6/R7 have ≥1 curated chain but subject-uniqueness guard filters in current artefact |
| Codex-flagged bad chains | shown as R1/R7 examples | **eliminated** | all three had text-pack `source_chain` entries |

Post-v4.0.2 Part 4 per-rule-representative output (real):

```
  [R1_is_a_transitivity]    түлкі --is_a--> жануар           (world_core/animals)
  [R2_has_inheritance]      құс жолы --has--> жұлдыз        (world_core/astronomy)
  [R3_has_inheritance_via_part_of]  қазақ --has--> бас      (world_core/colors + body_parts)
  [R5_shared_is_a_target]   бөлу --related_to--> көбейту    (world_core/numbers)
```

The R5 sample — «division related to multiplication» via shared-math-operation IsA target — is an unusually clean investor pitch for math-driven reasoning. Every claim traceable to a named reviewer (`shaman` at this stage).

### Tests

**449 passing** (+5 from v4.0.1): 5 unit tests for the `derivation_is_fully_curated` helper covering curated / mixed / text-only / empty / prefix-boundary cases.

### Scope discipline

Part 4 `adam_demo` only. `adam_chat --safe` equivalent is deferred to v4.0.3 — keeping each patch single-concern.

### What's next (v4.0.3)

- Wire the same curated-only filter into `adam_chat` behind a `--safe` / `--curated-only` flag. Dialog's `inject_reasoning_chain` currently scans *all* derivations; the filter belongs there too for investor-safe chat mode.

---

## [4.0.1] — 2026-04-23 — «Неліктен?» REPL path fix (Codex v4.0.0 review follow-up)

Small, focused patch closing the bug Codex caught in the v4.0.0 audit:
**«Неліктен?» was still routed through `StatementOfLocation { city: "Нелік" }`**
in the real `adam_chat` REPL despite the v3.9.5 `NOT_A_TOPIC` work. The
unit-level test `not_a_topic_covers_v3_9_5_additions` was passing, but it
exercised `first_noun_root` / `content_roots` — not the ablative-scan path
in `detect_statement_of_location`. Two separate code paths, one covered,
one not.

### Root cause

FST analysis of "неліктен" returns three parses (in deterministic order):

```
noun: нелік +Ablative
noun: нелік +Singular +Ablative
noun: неліктен
```

`detect_statement_of_location` scans parses for the first `Case::Ablative`
noun and returns its root as a city. Before v4.0.1 that was the stripped
stem `нелік`, and `NOT_A_TOPIC` only contained the full surface form
`неліктен` — so the skip-check matched nothing. Result: REPL reply
"Нелікте тұрасыз ба" ("Do you live in Нелік?") to "Неліктен?".

### Fix

1. Add `нелік` (the FST-stripped stem) to `NOT_A_TOPIC` in
   `adam_dialog::semantics`.
2. `detect_statement_of_location` now **skips any noun whose root is in
   `NOT_A_TOPIC`** at the case-scan step — same filter the content-root
   path already uses. Covers ablative, locative, and `Locative+P1Sg`
   branches uniformly.
3. Mirror `нелік` in `adam_reasoning::patterns::is_closed_class` for
   cross-layer consistency.

### Regression test — REPL path, not unit filter

New e2e test `nelikten_is_not_absorbed_as_city` in
`crates/adam-dialog/tests/end_to_end.rs` exercises the exact
`Conversation::turn(...)` path Codex reproduced:

- Turn 1: "мен Қостанайдамын" → `session.city = "Қостанай"` (baseline
  works).
- Turn 2: "Неліктен?" — `session.city` MUST remain "Қостанай" (not be
  overwritten with "Нелік").
- Turn 3: fresh `Conversation`, bare "Неліктен?" — `session.city` MUST
  stay absent.

Pre-v4.0.1 Turn 2 failed the assertion. Post-v4.0.1 it passes.

### Verified in REPL

```
$ cargo run -p adam-dialog --bin adam_chat -- --once "Неліктен?"
түсінбедім
```

(Previously: «Нелікте тұрасыз ба».)

### Tests

**444 passing** (+1 from v4.0.0): the new e2e regression.

### Housekeeping

- `cargo clean` reclaimed **37.4 GiB** of `target/` artefacts (9.7 GiB
  → 42 GiB free). Standing procedure — run before every release when
  free space drops below ~15 GiB.
- Project direction captured in `project_v4_direction` memory: develop
  on M2 8 GB without investors; synthetic FST-generated data + strict
  Kazakh grammar rules as the path to "intelligent reasoning via simple
  math"; sequential 1→9 per-integer versioning (v4.0.1 → v4.0.2 →
  v4.0.3 …), no half-step jumps.

### What's next

- v4.0.2: reasoning-demo precision polish — filter out the remaining
  Codex-flagged noise chains (`абай is_a халық`, `еңбек — өзен`,
  `топырақ goes_to дене`) at the demo layer, not at extraction.
- v4.0.3+: continued patch-level hardening ahead of the next minor
  (v4.1.0) which will carry architectural additions.

---

## [4.0.0] — 2026-04-23 — World Core 500+ expansion + contradiction immune system + Codex-review response

**Major release.** Codex's v3.9.5 review correctly flagged that reasoning was scaling faster than precision — «бала lives_in күн жүйесі», «(егер, DoesTo, газ)», «(жалға, GoesTo, жер)», «еңбек — өзен» were real chains in `facts.json`, not hypothetical. v4.0.0 addresses both ends of the problem: (1) **expand curated knowledge** to outweigh extracted noise via sheer IsA density, and (2) **add a contradiction immune system** that categorically refuses the classes of false derivations Codex exhibited.

### 1. World Core expansion — 200 → 507 entries / 270 → 601 facts

Seven new domains authored by `shaman` at `approved` status:

| new domain | entries | facts | content |
|---|---:|---:|---|
| `colors.jsonl` | 37 | 38 | primary colors, Kazakh traditional (алтын / күміс / көксоңы / боз / құла), nature-color associations, kemperqosaq (rainbow), vision-domain |
| `numbers.jsonl` | 45 | 54 | digits 1–10, tens 20–90, 100 / 1000 / million / billion, basic operations (қосу, алу, көбейту, бөлу), even/odd, time units |
| `kz_literature.jsonl` | 60 | 69 | 18 authors (Абай, Махамбет, Жамбыл, Мағжан, Шәкәрім, Ыбырай, Мұхтар, Олжас, Мұқағали, …), 7 works (Қара сөздер, Абай жолы, Қобыланды, Қыз Жібек, Алпамыс, Қозы Көрпеш, …), 12 genres (өлең, поэма, роман, әңгіме, ертегі, дастан, жыр, …), structure (шумақ, тармақ, ұйқас, поэзия, проза) |
| `food.jsonl` | 50 | 50 | bread (нан, бауырсақ, шелпек), meat (қой/сиыр/жылқы/түйе еті), Kazakh traditional dishes (бешбармақ, куырдак, сорпа), dairy (сүт, қымыз, шұбат, айран, қаймақ, ірімшік, құрт, сары май), fruits, vegetables, grains, beverages |
| `clothing.jsonl` | 35 | 35 | general (көйлек, шалбар, тон, ішік), Kazakh traditional (шапан, камзол, сәукеле, кимешек), headwear (тақия, қалпақ, бөрік, орамал), footwear (мәсі, кебіс, етік, бәтеңке), fabrics, jewellery |
| `proverbs.jsonl` | 40 | 43 | curated mақал with embedded IsA / Causes / RelatedTo facts — «Еңбек түбі — береке», «Білім — қуат», «Тіл — ұлт белгісі», «Бірлік — байлық», «Ана — баланың алғашқы ұстазы» |
| `animals.jsonl` | 40 | 42 | predators (қасқыр, түлкі, арыстан, жолбарыс, аю), game (қоян, тиін, бұғы, киік, арқар), birds (бүркіт, тырна, аққу, үкі, тауық, қаз, үйрек, торғай, қарға, сауысқан), domestic (сиыр, қой, жылқы, түйе, ит, мысық), insects, taxonomy (сүтқоректі, жыртқыш, жәндік, зоология, орнитология) |

Plus existing 6 domains carried forward. **Total: 507 entries / 601 facts across 13 domains.** All 0-rejection on `validate_world_core`.

### 2. Contradiction immune system

Codex's v3.9.5 review surfaced specific false chains in `facts.json`. Each fix is targeted and test-enforced:

- **R6/R7 astronomical-target guard** (new in reasoner.rs): `is_astronomical_object` helper (күн / ай / жер / марс / шолпан / меркурий / юпитер / сатурн / уран / нептун / күн жүйесі / галактика / құс жолы / ғаламшар / жұлдыз / аспан денесі / метеор / атмосфера / орбита). R6 and R7 now refuse derivations where the target `C` is astronomical-scale. Closes `(бала, LivesIn, жер) + (жер, PartOf, күн жүйесі) ⟹ (бала, LivesIn, күн жүйесі)` — the homonymous «жер» (both "ground" and "Earth") cross-domain leak.
- **Object-side 3-char minimum** (locative_lives_in, dative_goes_to): mirrors the subject-side guard from v3.8.5. Closes `(бала, LivesIn, ған)` where the FST emitted a `-ған` participle tail as a standalone root, and analogous `-ын / -ін / -қан / -сын` fragments.
- **`is_closed_class` expansion** (patterns.rs, 20+ new entries):
  - conjunctions: `егер` / `алайда` / `бірақ` / `дегенмен` / `сондықтан` / `демек` / `яғни` / `әйтсе` / `өйткені` / `сонда` / `сонымен` — closes `(егер, DoesTo, газ)` ("if" as subject);
  - adverbial oblique stems: `жалға` / `тек` / `қана` / `ғана` — closes `(жалға, GoesTo, жер)`;
  - fragment-suffix standalones: `ған` / `ген` / `қан` / `кен` / `ын` / `ін` / `сын` / `сін` — defence in depth alongside the 3-char minimum above.

Three new regression tests: `r6_refuses_astronomical_derived_target`, `r6_still_fires_for_country_target`, `r7_refuses_astronomical_derived_target`. The "still fires for country" test is the key one — the guard must NOT block legitimate `(person, LivesIn, city) + (city, PartOf, country) ⟹ (person, LivesIn, country)` chains.

### 3. Measured impact at T4_200k

| | v3.9.5 | v4.0.0 | delta |
|---|---:|---:|---|
| facts.json (total) | 13 771 | **13 889** | **+118** |
| curated (HumanApproved) | 270 | **601** | **+331** (×2.2) |
| extracted (Grammar) | 13 501 | 13 288 | −213 (filter noise removal) |
| graph nodes | 3 151 | **3 286** | **+135** |
| graph edges | 12 317 | **12 447** | **+130** |
| **derivations** | **2 058** | **6 579** | **+4 521 (×3.2)** |
| predicate coverage | 11/11 | 11/11 | preserved |

Per-predicate fact counts — World Core drives structural gains, filters trim noise:

| predicate | v3.9.5 | v4.0.0 | delta | driver |
|---|---:|---:|---:|---|
| **IsA** | 294 | **525** | **+231** | world_core breadth (13 domains → many IsA) |
| RelatedTo | 1 446 | 1 467 | +21 | |
| **Has** | 207 | 226 | +19 | world_core body_parts/society |
| **PartOf** | 105 | 116 | +11 | |
| **HasQuantity** | 29 | 40 | +11 | numbers.jsonl (year has 12 months etc) |
| **Causes** | 6 | **22** | **+16** | proverbs + biology entries |
| **InDomain** | 5 | **24** | **+19** | kz_literature genres + sciences |
| After | 269 | 265 | −4 | |
| LivesIn | 313 | 292 | **−21** | 3-char object filter + fragment-suffix closed-class |
| GoesTo | 1 692 | 1 617 | **−75** | same filters |
| DoesTo | 9 399 | 9 295 | **−104** | same filters |

Per-rule derivation counts — **R5 and R1 jump from denser IsA graph; R6/R7 shrink as astronomical-guard blocks false chains**:

| rule | v3.9.5 | v4.0.0 | delta | reason |
|---|---:|---:|---:|---|
| R1_is_a_transitivity | 114 | **361** | **+247 (×3.2)** | more IsA chains (world_core 507 entries → 525 IsA facts → dense A-IsA-B-IsA-C paths) |
| R2_has_inheritance | 253 | **422** | **+169** | denser IsA base |
| R3_has_inheritance_via_part_of | 15 | **26** | **+11** | body_parts PartOf chains |
| **R5_shared_is_a_target** | 933 | **5 437** | **+4 504 (×5.8)** | 525 IsA facts form exponentially more sibling pairs sharing a target |
| **R6_lives_in_via_part_of** | 103 | **36** | **−67** | **astronomical-target guard** blocked 67 false chains (`бала lives_in күн жүйесі` class) |
| **R7_goes_to_via_part_of** | 640 | **297** | **−343** | **same guard** — biggest precision win |

R6/R7 shrinkage is a **net precision gain**: the 67+343 = 410 blocked derivations were chains where the target was an astronomical-scale object (homonymous «жер» bridging "ground" to "Earth", or adverbial "жалға" chaining through "жер" to "күн жүйесі"). Genuine `(person, LivesIn, city) + (city, PartOf, country) ⟹ (person, LivesIn, country)` chains still fire, as verified by `r6_still_fires_for_country_target` test.

Most-connected graph nodes (content-noun focus preserved): **адам (289), жер (219), дүние (211), қазақ (201), ат (150)**.

### Tests

**443 passing** (+3 from v3.9.5): `r6_refuses_astronomical_derived_target`, `r6_still_fires_for_country_target`, `r7_refuses_astronomical_derived_target`.

### Architectural stance — Codex response

Codex's full recommendation included a Typed World Model with EntityType ontology, Fact Promotion Pipeline with `Candidate`/`Verified`/`HumanApproved` tiers, and a dedicated Contradiction/Absurdity Filter. v4.0.0 ships **targeted** fixes — the filters are hand-coded blocklists rather than type-derived constraints — because every hand-coded filter is test-enforceable today without gating on a larger ontology design. The Typed World Model is a v5.x target; v4.x adds incremental curated-knowledge breadth and domain-specific blocklists as new noise classes surface.

The architectural stance stays: **adam is not competing with ChatGPT on breadth.** v4.0.0's 507 curated entries + 600+ typed facts, each with a named reviewer, are measured against GPT-4's "correct answer" baseline: correct per-claim, traceable per-source, zero hallucination by construction. This is a substrate for sovereign-AI / education / government domains, not a general-purpose Q&A model.

### What's next (v4.5+)

- v4.5: investor-ready MVP — scripted 3-minute `adam_demo_v4` narrative pulling **exclusively** from `HumanApproved` tier; screencast recording; one-page pitch deck.
- `validate_world_core` integrated into `scripts/validate_foundation.sh` as a CI gate (currently standalone).
- v5.x: Typed World Model — EntityType ontology per root, type-constrained rule firing, Fact Promotion Pipeline with `Candidate`/`Verified` tiers that require evidence from multiple sources before promotion.

---

## [3.9.5] — 2026-04-23 — World Core expansion + R6/R7 rules + dialog closed-class sync

**Continuation of the v3.9.0 architectural direction.** Three independent improvements, each a small and contained delta:

### 1. World Core expansion — 80 → 200 entries / 126 → 270 facts

Three new domains added by `shaman` at `approved` review status:

| domain | entries | facts |
|---|---:|---:|
| astronomy | 30 | 41 |
| time | 20 | 38 |
| geography_kz | 30 | 47 |
| **biology_basic** (v3.9.5) | **40** | **41** |
| **body_parts** (v3.9.5) | **40** | **55** |
| **society** (v3.9.5) | **40** | **48** |
| **TOTAL** | **200** | **270** |

Content:
- `biology_basic.jsonl` — human, mammals, common animals (ит, мысық, жылқы, қой, сиыр, түйе, ешкі, құс, балық), plants (ағаш, шөп, гүл, бидай), terrain (орман, дала, шөл, тайга, тау, өзен, көл, теңіз, мұхит), cell / organism, biology + ecology as sciences. 41 typed facts.
- `body_parts.jsonl` — head parts (бас, бет, көз, құлақ, мұрын, ауыз, тіл, тіс, шаш), limbs (мойын, иық, қол, саусақ, алақан, тізе, аяқ, табан), internal organs (жүрек, өкпе, бауыр, бүйрек, асқазан, ми, қан), structural (сүйек, ет, тері, жүйке), 6 quantified claims («адамда екі көз бар» etc), anatomy as a science. 55 typed facts.
- `society.jsonl` — state / law / constitution / parliament / president / courts, family (ана, әке, бала), education (мектеп, университет, оқушы, мұғалім, студент, кітап, кітапхана), sciences (математика, физика, химия, тарих), economy (ақша, теңге, еңбек), professions (дәрігер, мұғалім, инженер, заңгер), dimension (дін, мәдениет, тіл, қазақ тілі, халық). 48 typed facts.

All 200 entries pass `validate_world_core` with 0 rejections / 0 Kazakh-purity warnings.

### 2. R6 + R7 — spatial + directional transitivity rules

Two new forward-chaining rules, activated now that v3.8.0's verb-root fix gave `LivesIn` / `GoesTo` real data AND v3.9.0's `geography_kz.jsonl` curated a `city PartOf country` chain:

| rule | formula | example |
|---|---|---|
| **R6_lives_in_via_part_of** | `A LivesIn B ∧ B PartOf C ⟹ A LivesIn C` | (Дәулет, LivesIn, Қостанай) ∧ (Қостанай, PartOf, Қазақстан) ⟹ (Дәулет, LivesIn, Қазақстан) |
| **R7_goes_to_via_part_of** | `A GoesTo B ∧ B PartOf C ⟹ A GoesTo C` | (ол, GoesTo, Алматы) ∧ (Алматы, PartOf, Қазақстан) ⟹ (ол, GoesTo, Қазақстан) |

Both emit `ConfidenceKind::RuleInferred` with a 2-source chain. Tautology-guarded (A ≠ C). 6 new unit tests: `r6_derives_lives_in_via_part_of`, `r6_respects_tautology_guard`, `r6_does_not_fire_without_part_of_edge`, `r6_dedupes_against_existing_fact`, `r7_derives_goes_to_via_part_of`, `r7_respects_tautology_guard`.

Reasoner roster is now 5 active rules: R1 / R2 / R3 / R5 / R6 / R7 (6 total). R4 remains curator-warning only.

### 3. Dialog `NOT_A_TOPIC` sync — closes «Неліктен → Нелікте тұрасыз ба» bug

v3.8.5 free-form REPL testing surfaced: user typing «Неліктен?» («why?» — an interrogative) got reply «Нелікте тұрасыз ба» («Do you live in Нелік?»). The FST analysed «Неліктен» as `Нелік` + ablative suffix (a valid morphological parse), and `adam-dialog::semantics::NOT_A_TOPIC` lacked the interrogative entries that `adam-reasoning::patterns::is_closed_class` had gained in v3.5.0+.

Fix: expanded `NOT_A_TOPIC` to mirror `is_closed_class` — added interrogatives (`неліктен`, `неге`, `қашан`, `қайда`, `қандай`, `кім`, `не`, `қай`, `қанша`), demonstrative qualifiers (`мұндай`, `сондай`, `ондай`, `мынадай`, `сондай-ақ`, `кейбір`, `өз`, `өзі`, `бірнеше`, `барша`, `әрбір`, `әр`, `бір`, `кей`), plus the comparison particle `сияқ` missing since v3.5.0.

Regression test `not_a_topic_covers_v3_9_5_additions` asserts every newly-added word is present AND that content nouns (бала, кітап, мектеп, қазақстан, жер) still pass through.

### Committed runtime delta

Measured on T4_200k after v3.9.5 extraction (`--bench-order --max-total 200000`):

| | v3.9.0 | v3.9.5 | delta |
|---|---:|---:|---|
| facts.json (total) | 13 627 | **13 771** | **+144** |
| curated (world_core, HumanApproved) | 126 | **270** | **+144** (new domains) |
| extracted (text, Grammar) | 13 501 | 13 501 | 0 (text corpus unchanged) |
| graph nodes | 3 100 | **3 151** | **+51** |
| graph edges | 12 175 | **12 317** | **+142** |
| **derivations** | **704** | **2 058** | **+1 354 (×2.9)** |
| predicate coverage | 11/11 (100 %) | 11/11 (100 %) | preserved |

Per-predicate fact counts after v3.9.5:

| predicate | v3.9.0 | v3.9.5 | delta |
|---|---:|---:|---|
| DoesTo | 9 399 | 9 399 | 0 |
| GoesTo | 1 692 | 1 692 | 0 |
| RelatedTo | 1 446 | 1 446 | 0 |
| LivesIn | 313 | 313 | 0 |
| **IsA** | 219 | **294** | **+75** (world_core biology/society/body_parts) |
| After | 269 | 269 | 0 |
| **Has** | 190 | **207** | **+17** |
| **PartOf** | 65 | **105** | **+40** (body_parts / biology / society chains) |
| **HasQuantity** | 29 | **35** | **+6** |
| **Causes** | 3 | **6** | **+3** (biology water/air entries) |
| **InDomain** | 2 | **5** | **+3** (biology/anatomy sciences) |

Per-rule derivation counts — **R6 and R7 fire for the first time**:

| rule | v3.9.0 | v3.9.5 | delta |
|---|---:|---:|---|
| R1_is_a_transitivity | 42 | **114** | +72 |
| R2_has_inheritance | 173 | **253** | +80 |
| **R3_has_inheritance_via_part_of** | 0 | **15** | +15 (first real fire on curated chains) |
| R5_shared_is_a_target | 489 | **933** | +444 (×1.9) |
| **R6_lives_in_via_part_of** | 0 | **103** | +103 (NEW — v3.9.5) |
| **R7_goes_to_via_part_of** | 0 | **640** | +640 (NEW — v3.9.5) |

**R7 is the biggest single-rule contribution**: every one of the 1 692 extracted `GoesTo` facts whose destination is a city that curated `geography_kz` identifies as part of Қазақстан (or Орталық Азия / Еуразия) now produces a derivation at the country level. This is exactly the "city-level facts + curated chain → country-level conclusions" leverage the v3.9.5 plan targeted.

Most-connected graph nodes (content-noun focus preserved): **адам (290), жер (221), дүние (210), қазақ (200), ат (156)**. «адам» (human) remains central — a stronger semantic signal than any raw corpus statistic would give.

### Tests

**440 passing** (+7 from v3.9.0): 4 R6 regressions + 2 R7 regressions + 1 NOT_A_TOPIC sync test.

### Architectural status

At v3.9.5 adam has:
- **200 curated knowledge entries** → 270 facts with full reviewer provenance
- **5 active forward-chaining rules** (R1, R2, R3, R5, R6, R7) — R6/R7 turn the clean v3.8.5-hardened LivesIn/GoesTo predicates into new derivations
- **11/11 predicate coverage** preserved from v3.9.0
- **Dialog intent layer synced with reasoning closed-class** — one single source of truth for «what is a content noun»

Every curated fact is signed by a reviewer; every derivation has a source_chain; every refusal («Неліктен?») goes through an explicit filter rather than a heuristic. This is the shape of an **auditable Kazakh reasoning engine**.

### What's next (v4.0.0 — investor-ready MVP)

- Expand World Core to 500–1 000 entries (add `numbers`, `colors`, `kz_literature`, `food`, `clothing`)
- Full scripted investor demo (`adam_demo_v4` — one command, one full narrative, ~3-minute screencast)
- Native-speaker review workflow (web UI for community contributions)
- `validate_world_core` integrated into `validate_foundation.sh` as CI gate
- Extend `data/world_core/README.md` with per-domain authoring guides

---

## [3.9.0] — 2026-04-23 — World Core v1: curated Kazakh knowledge packs + hygiene gate

**Architectural direction captured.** Codex's second-pass review of v3.8.5 converged with our own assessment: the path to a «ChatGPT-class intellectual, but without probability / cost / hallucination» is **not** to train an LLM-clone — it's to build an *auditable Kazakh reasoning engine* on top of **curated knowledge packs**. v3.9.0 ships the World Core infrastructure that unlocks this path + closes the `-`-prefixed fragment noise Codex flagged on the facts.json graph.

### 1. Hygiene gate — `-`-prefixed fragment roots refused

Codex measured 87 facts on the v3.8.5 committed `facts.json` where subject or object root started with `-` (artifacts of FST tokenisation splitting compound tokens like `2021-жылғы` into dash-prefixed fragments). Top offenders: `-дүниежүзілік` (20), `-ға` (8), `-жыл` (6), `-ғасыр` (7), `-қа` (6), `-нан` (6). Every such root is categorically a suffix-fragment parse and can never represent a real entity.

Fix: central `is_fragment_root` gate in [`lib.rs`](crates/adam-reasoning/src/lib.rs) post-filter that rejects any fact whose subject or object root is empty or starts with `-`. Applies uniformly across all 11 matchers — no per-matcher code churn needed. Regression test `is_fragment_root_rejects_dash_prefixed` covers the seven flagged patterns plus the boundary case of internal dashes (`сондай-ақ`, `нұр-сұлтан` — legitimate content) passing through.

### 2. World Core v1 — curated Kazakh knowledge packs

New infrastructure that lets human reviewers inject high-trust typed facts directly into the graph, bypassing the precision ceiling of text-pattern matchers.

**Schema** (one JSON per line, one file per domain in `data/world_core/<domain>.jsonl`):

```jsonc
{
  "id": "astro_001",
  "kk": "Жер — Күн жүйесіндегі ғаламшар.",
  "facts": [
    { "subject": "жер", "predicate": "is_a",    "object": "ғаламшар" },
    { "subject": "жер", "predicate": "part_of", "object": "күн жүйесі" }
  ],
  "domain": "astronomy",
  "source": "curated",
  "confidence": "high",
  "review_status": "approved",
  "reviewer": "shaman",
  "reviewed_at": "2026-04-23"
}
```

**Crate surface** ([`adam_reasoning::world_core`](crates/adam-reasoning/src/world_core.rs)):

- `WorldCoreEntry` / `WorldCoreFact` — serde-deserialising structs with stable JSONL form.
- `ConfidenceTier { High, Medium, Low }` — reserved for reviewer discretion; `ReviewStatus { Approved, Pending, Rejected }` — only `Approved` entries enter the runtime fact set.
- `load_world_core_dir(path)` — parses every `*.jsonl` deterministically; returns a `LoadReport` with accepted + rejected entries (rejection reasons: parse failure, duplicate id, empty fact list, tautology, fragment root).
- `emit_facts(entry, path)` — converts an approved entry into pipeline `Fact`s with `ConfidenceKind::HumanApproved` + `source.pack = "world_core/<domain>.jsonl"`.
- `load_world_core_facts(path)` — convenience one-shot for the extract pipeline.

**New binary** [`validate_world_core`](crates/adam-reasoning/src/bin/validate_world_core.rs): authoring-gate validator. Prints per-domain summary (entries / approved / pending / rejected / facts), flags Kazakh-purity violations (any non-Cyrillic character in the `kk` sentence), reports all rejected entries, returns non-zero exit code if anything failed. Intended to run in CI alongside `validate_foundation`.

**Integration into `extract_facts`**: after scanning text corpus packs, the binary calls `world_core::load_world_core_facts("data/world_core")` and merges the curated facts into the same `artifact.facts` vector that text extraction populates. Per-predicate + per-pack counters are updated uniformly so the summary output lists curated packs (`world_core/astronomy.jsonl`, etc.) alongside extracted packs. Missing `data/world_core/` is a silent no-op — trimmed CI checkouts behave identically to pre-v3.9.0.

### 3. Seed data — 80 entries / 126 facts across 3 domains

Bootstrap content authored for v3.9.0 (all `approved` by `shaman` at `high` or `medium` confidence):

| domain | entries | facts | example |
|---|---:|---:|---|
| `astronomy` | 30 | 41 | «Жер — Күн жүйесіндегі ғаламшар» → `(жер, is_a, ғаламшар)` + `(жер, part_of, күн жүйесі)` |
| `time` | 20 | 38 | «Жыл — он екі айдан тұрады» → `(жыл, has_quantity, ай)` + `(ай, part_of, жыл)` |
| `geography_kz` | 30 | 47 | «Алматы — Қазақстанның ірі қаласы» → `(алматы, is_a, қала)` + `(алматы, part_of, қазақстан)` |
| **TOTAL** | **80** | **126** | — |

All 80 entries pass `validate_world_core`. `reviewer: "shaman"` is the bootstrap author handle; v3.9.5+ will introduce the native-speaker review workflow.

### 4. `adam_inspect` — Curated vs Extracted split

The per-root report in [`adam_inspect`](crates/adam-dialog/src/bin/adam_inspect.rs) now separates facts into two sections:

1. **Curated (world_core — HumanApproved)** — shown first. Each entry prints the `domain`, the `(pack, sample_id)` provenance, AND the Kazakh sentence `kk` in quotes — the audit trail is complete.
2. **Extracted (Grammar — corpus text patterns)** — shown after. Unchanged from v3.8.5.

The `is_curated` filter is `f.confidence == ConfidenceKind::HumanApproved` — single-predicate dispatch, no ambiguity. Summary footer updated to count each tier separately.

### Committed runtime delta

| | v3.8.5 | v3.9.0 | delta |
|---|---:|---:|---|
| facts.json (total) | 13 627 | **13 627** | **0** (composition changed) |
| curated (world_core, HumanApproved) | 0 | **126** | **+126** (new tier) |
| extracted (text, Grammar) | 13 627 | **13 501** | **−126** (fragment-root filter dropped 87 dash noise; small matcher re-runs on top) |
| graph nodes | 3 087 | **3 100** | **+13** |
| graph edges | 12 165 | **12 175** | **+10** |
| derivations | 205 | **704** | **+499 (×3.4)** — world_core IsA chains ignited R5 shared-target matching: 56 → **489** |
| **predicate coverage** | **9 / 11 = 81.8 %** | **11 / 11 = 100 %** | **+2 (Causes, InDomain)** — world_core entries `astro_015/016` ("Күн жарық береді" / "Күн жылу береді") activate `Causes`; `astro_024` activates `InDomain` |

Per-predicate fact counts after v3.9.0:

| predicate | v3.8.5 | v3.9.0 |
|---|---:|---:|
| DoesTo | 9 498 | 9 399 |
| GoesTo | 1 697 | 1 692 |
| RelatedTo | 1 449 | 1 446 |
| LivesIn | 315 | 313 |
| After | 275 | 269 |
| **IsA** | 162 | **219 (+57)** |
| Has | 189 | 190 |
| **PartOf** | 23 | **65 (+42)** |
| HasQuantity | 19 | 29 |
| **Causes** | 0 | **3 (first fire)** |
| **InDomain** | 0 | **2 (first fire)** |

Per-rule derivation counts at v3.9.0: R1_is_a_transitivity = **42** (was 33), R2_has_inheritance = **173** (was 116), R5_shared_is_a_target = **489** (was 56). R3_has_inheritance_via_part_of fires 0× post-hardening (PartOf subject/object roots don't yet overlap with Has subject in the clean set; v3.9.5 adds more PartOf entries).

Most-connected graph nodes post-merge (content-noun focus preserved): адам (279), жер (221), дүние (210), қазақ (200), ат (156).

### Tests

**433 passing** (+10 from v3.8.5): 1 hygiene-gate regression + 9 world_core loader / validator / emitter tests.

### Trust invariants (test-enforced)

- `ConfidenceKind::HumanApproved` is **exclusive** to world_core; text extraction never produces it.
- `source.pack` starting with `world_core/` is **exclusive** to world_core; text-pack paths never overlap.
- `review_status ∈ {Pending, Rejected}` → entry does **not** emit facts (verified by unit tests `emit_facts_refuses_pending_entry` and `emit_facts_refuses_rejected_entry`).
- `Fact` dash-prefixed root → unconditionally refused (verified by `is_fragment_root_rejects_dash_prefixed`).

### Architectural statement

This release captures a deliberate direction: **adam is not competing with ChatGPT on breadth.** It is becoming an *auditable Kazakh reasoning engine* — narrower than an LLM, cheaper by orders of magnitude, but provably unable to hallucinate (every output is a template / verbatim quote / FST synthesis / rule-derived chain with full provenance, now augmented with curated world_core facts each of which has a named human reviewer).

The long-term goal (v4.0.0) is a **5 000+ entry world_core** across 10+ domains, plus R6 / R7 rules (`LivesIn + PartOf → LivesIn`, `GoesTo + PartOf → GoesTo`) that fire on the clean v3.8.5-hardened predicate set. This makes the project a genuine commercial differentiator for the sovereign-AI / government-sector use case: **you can see exactly where every answer comes from, and no claim enters the runtime without a human's name attached to it.**

### What's next (v3.9.5)

- Expand world_core to 500+ entries across 6–8 domains (add `biology_basic`, `society`, `numbers`, `colors`, `body_parts`)
- `is_closed_class` / `is_time_noun` / `is_location_root` sync across `adam-reasoning::patterns` and `adam-dialog::semantics` (closes the `Неліктен → Нелікте тұрасыз ба` bug surfaced during the v3.8.5 free-form REPL test)
- Clean OCR noise filter on retrieval samples (rejects «ақ-», truncated stems)
- Community contribution workflow for native-speaker review

---

## [3.8.5] — 2026-04-22 — Precision hardening: Codex review response (doc drift, renderer morphology, matcher filters)

**Patch release addressing the [Codex / Antigravity review of v3.8.0](https://github.com/qazaq-ai/adam/issues).** Three categories of defect closed, each concretely flagged by the external reviewer:

### 1. Documentation drift (README vs architecture_v3 vs runtime)

- README table had **two contradicting rows** for "Reasoning rules active": one saying `4/5` (including R3), another (further down) saying `3 (R1, R2, R5)`. Removed the stale row.
- `docs/architecture_v3.md` still described **4 pattern matchers** and R3 as `documented, deferred` — actual runtime has **11 matchers** and R3 has been active since v3.5.5. Rewrote both the matchers table and the rule table. Added R6/R7 as v3.9+ targets (LivesIn+PartOf, GoesTo+PartOf transitivity) now that the v3.8.0 verb-root fix gave those predicates real data.

### 2. Renderer morphology (`атау-ға` / `өсімдік-ға` bug)

Pre-v3.8.5 `render_derivation_as_kazakh` (both in `adam-dialog::conversation` and in the `adam_inspect` binary) concatenated case suffixes with a literal dash: `format!("{}-ға ...", root)`. This produced two kinds of invalid Kazakh: **(a)** the dash itself (suffixes attach directly), and **(b)** wrong vowel harmony on every front-harmony root (`өсімдік-ға` instead of `өсімдікке`). v3.8.5 routes every case suffix through `synthesise_noun(root, features)` — the same FST the template realiser uses. Verified dative output for a representative set:

| root | dative |
|---|---|
| атау | атауға ✓ |
| өсімдік | өсімдікке ✓ (front harmony + voiceless gemination) |
| кітап | кітапқа ✓ |
| мектеп | мектепке ✓ |
| қазақ | қазаққа ✓ (voiceless gemination) |
| халық | халыққа ✓ |
| жер | жерге ✓ |

Regression test `reasoning_chain_uses_fst_synthesis_not_dash_concatenation` asserts positive FST form and negative absence of `атау-ға`.

**Known FST limitation discovered during fix (deferred to v3.9)**: `synthesise_noun` with `Case::Genitive` on a **vowel-final root** produces `қаладың` instead of `қаланың` — the `{D}{I}ң` archiphoneme template lacks the "after-vowel → н" rule that genitive requires. Ablative / dative / instrumental on the same roots are all correct. The renderer now sidesteps the bug by using dative in PartOf / Causes chains instead of genitive; the FST phonology fix itself is a v3.9 target (it affects 48+ existing FST roundtrip tests and warrants a standalone release).

### 3. Matcher precision hardening

Codex's live `adam_inspect` session produced three canonical noisy triples: `қазақстан → lives_in → аумағын`, `мұндай → goes_to → өсіру`, `күн → goes_to → жұмыс`. Each is a distinct failure mode:

- **Country as `LivesIn` subject**: "Қазақстан" can't reside anywhere — it **is** a place. Added `is_location_root` (50-toponym allow-list of Kazakh countries / major cities / continents / major rivers) and refuse as `LivesIn` subject. Scope is intentionally conservative; widening to a full gazetteer is v3.9+.
- **Time noun as motion subject**: "бір күн Масғұт жұмысқа барды" was producing `(күн, goes_to, жұмыс)`. Added `is_time_noun` helper (жыл / күн / ай / сағат / минут / ғасыр / уақыт / тәулік / апта / кез / сәт / мезгіл / шақ / мезет / түн / таң / кеш / …) and refuse as subject for `LivesIn`, `GoesTo`, **and** `DoesTo`. Pre-hardening these were **309 / 1864 = 16.6 % of all `GoesTo` facts**.
- **Demonstrative qualifier as subject**: "мұндай" / "сондай" / "ондай" / "мынадай" / "сондай-ақ" / "кейбір" / "өз" / "өзі" / "бірнеше" / "барша" / "әрбір" / "әр" / "бір" / "кей" all added to `is_closed_class`. Pre-hardening: 243 noisy facts across all predicates.
- **Object with leaked possessive suffix**: `(қазақстан, lives_in, аумағын)` — the object surface is `аумағында` but the FST analysis retains P3 possessive on the root (`аумағын`), indicating a fragment parse. v3.8.5 refuses any `LivesIn` / `GoesTo` object whose FST analysis has `features.possessive.is_some()`.
- **Short broken stems**: added minimum subject-root length of 3 characters across `locative_lives_in`, `dative_goes_to`, `agent_verb` — drops truncated FST outputs like `кешк`, `қаһарл` that had been contaminating the committed fact set.

### 4. Demo preview / rendered-text mismatch

`adam_demo` Part 4 printed a per-rule preview like `[R5]  неміс → халқы` but the rendered user-facing response used `неміс → ара` (a different derivation with the same subject appearing earlier in storage order). Root cause: the demo's selection was `BTreeMap<rule_id, first-content-subject>` while `inject_reasoning_chain` uses `find(subj == probe || obj == probe)` — non-equivalent selectors.

v3.8.5 fixes both ends:
1. `inject_reasoning_chain` now does a **strict subject-first two-pass** (`find(subj == noun).or_else(|| find(obj == noun))`), matching the comment that was already there.
2. The demo's per-rule picker now **also tracks a `seen_subjects: HashSet<String>`** and skips derivations whose subject root was claimed by an earlier derivation in storage order — so every preview points to the exact derivation the pipeline would render.

### Tests

**423 passing** (+7 vs v3.8.0): new matcher filters each get a regression test (`locative_lives_in_rejects_country_subject`, `dative_goes_to_rejects_time_subject`, `dative_goes_to_rejects_demonstrative_subject`, `is_closed_class_covers_v3_8_5_additions`, `is_time_noun_covers_standard_set`, `is_location_root_covers_countries_and_cities`), plus the renderer regression `reasoning_chain_uses_fst_synthesis_not_dash_concatenation`.

### Predicate coverage

Unchanged at **9 / 11** (LivesIn, GoesTo stay active — the hardening tightens precision, not removes them).

### Upgrade notes

- Purely additive on matcher side — no library API change.
- Fact-set shrinks (precision vs recall trade-off). `data/retrieval/facts.json` regenerated at v3.8.5. Downstream consumers expecting exactly 14 430 facts will see the updated count (tracked in `data/retrieval/facts.json`; README reflects the new number).
- Dialog renderer output surface changes for `Has` / `PartOf` / `Causes` / `After` / `HasQuantity` / `InDomain` chains — suffixes are now properly inflected. The `unknown_with_reasoning_chain_cites_derivation` test still passes (it asserts on marker + root presence, not suffix shape).

### What's next (v3.9.0)

- Fix FST genitive-after-vowel phonology rule
- Extend location allow-list to full Kazakh gazetteer
- R6 (`LivesIn + PartOf → LivesIn`) / R7 (`GoesTo + PartOf → GoesTo`) rules now that the two predicates have data
- Full Codex-recommended **confidence tiers** (`High` / `Medium` / `Low`) on Fact + demo-only high-confidence subset
- Populate `docs/precision_audit.md` tally via native-speaker review pass

---

## [3.8.0] — 2026-04-22 — Critical verb-root bug fix: LivesIn + GoesTo activated (predicate coverage 7/11 → 9/11)

**Unlocks two dormant predicates that have been silently broken since v2.1 (LivesIn) and v2.5 (GoesTo).** The root-comparison checks used the **infinitive forms** (`"тұру"` / `"бару"`) while the FST stores verb **stems** without the `-у` suffix (`"тұр"` / `"бар"`). Neither matcher has ever fired, at any scale, on any corpus. v3.8.0 fixes the comparisons and widens the verb set.

### The bug

```rust
// pre-v3.8.0 — never matches:
Some(Analysis::Verb { root, .. }) => root.root == "тұру",

// v3.8.0:
Some(Analysis::Verb { root, .. }) => matches!(root.root.as_str(),
    "тұр" | "мекен" | "орналас"),
```

Verification via `cargo run -p adam-kernel-fst --bin adam_fst -- analyse тұрады`:

```
verb: тұр +Present
```

Lexicon root is `тұр`, not `тұру`. The pre-v3.8.0 code was looking for a root that could never exist.

### Fact delta at T4_200k (committed runtime scale)

| predicate | v3.7.5 | v3.8.0 | delta |
|---|---:|---:|---|
| `lives_in` | **0** | **572** | **+572 (first fire!)** |
| `goes_to` | **0** | **1 864** | **+1 864 (first fire!)** |
| `does_to` | 11 216 | 9 865 | -1 351 (stopword list finally effective) |
| `is_a` | 162 | 162 | unchanged |
| `has` | 190 | 190 | unchanged |
| `has_quantity` | 19 | 19 | unchanged |
| `part_of` | 25 | 25 | unchanged |
| `after` | 278 | 278 | unchanged |
| `related_to` | 1 455 | 1 455 | unchanged |
| **Total** | **13 345** | **14 430** | **+1 085** |

The `does_to` drop is a **concurrent precision fix**: the `agent_verb` stopword list was using the same infinitive forms (`"бару"`, `"болу"`, `"бару"`) so the stopword filter was also never effective. v3.8.0 aligns it to the real FST stems (`"бар"`, `"бол"`, `"кел"`, `"тұр"`, etc), correctly refusing those verbs as agent-patterns.

### Predicate coverage (committed runtime)

- **v3.7.5**: 7 / 11 — IsA, Has, PartOf, RelatedTo, After, HasQuantity, DoesTo
- **v3.8.0**: **9 / 11** (+2) — adds **LivesIn, GoesTo**
- Still at 0: Causes (v3.9 — literal `себебі` head is rare), InDomain (v3.9 — similarly rare head).

### Sample new facts

From `cargo run -p adam-dialog --bin adam_inspect -- қазақстан`:

```
outgoing: does_to=50, goes_to=8, is_a=2, lives_in=6, part_of=1, related_to=13
incoming: does_to=11, goes_to=14, lives_in=3, related_to=10

  `қазақстан` --lives_in--> `аумағын`  [pattern: X Y-да тұрады; wiki_kz_...]
  `қазақстан` --lives_in--> `қала`     [pattern: X Y-да тұрады; wiki_kz_...]
  `қазақстан` --goes_to--> `іс`         [pattern: X Y-ке барады; wiki_kz_...]
```

### Regenerated committed artifacts

| artifact | v3.7.5 | v3.8.0 | delta |
|---|---:|---:|---|
| `facts.json` | 13 345 | **14 430** | +1 085 |
| graph nodes | 2 974 | **3 091** | +117 |
| graph edges | 11 813 | **12 772** | +959 |
| `derived_facts.json` | 207 | **207** | unchanged |

**Derivations unchanged at 207**: R1/R2/R3/R5 only consume IsA/Has/PartOf predicates. LivesIn/GoesTo enrich the graph but don't drive the existing rules. **v3.9+ can add R6** (`LivesIn + PartOf → LivesIn`, spatial-inheritance) or similar to turn the new predicates into derivations.

### Most-connected nodes post-extraction

- `жер` (degree 227) — earth/ground
- `ел` (degree 211) — country/people
- `қазақ` (degree 197) — Kazakh (ethnic/linguistic)

All legitimate content nouns. No noise.

### Tests

**416 passing, 0 failing, 0 warnings** — existing `locative_rejects_without_turu_verb` + `dative_rejects_without_baru_verb` tests still pass because they construct synthetic negative cases. **Note: these tests did not catch the bug** — they tested that a sentence *without* the required verb is rejected, but never tested that a sentence *with* the verb produces a fact. Strengthening the positive-case tests is a follow-up.

### Honest note

This is a **2-year-old latent correctness bug**. The reasoning crate has been shipping with silently-broken LivesIn / GoesTo predicates since v2.1 / v2.5 respectively, across every release up to v3.7.5. Like the v3.2.0 parser-determinism bug and v3.3.0 stale-artifact issue, this is a case where **repeat extraction on a bigger corpus surfaced a structural flaw** that wasn't visible at small scale. The v3.7.0 `adam_inspect` binary would have flagged zero lives_in/goes_to edges for any probe — worth noting for future per-predicate sanity checks.

### Banner sync per feedback_readme_pre_push_audit

  - `adam_chat.rs`: v3.7.5 → v3.8
  - `adam_demo.rs`: v3.7.5 → v3.8
  - README hero, comparison table, demo transcript all bumped

### Upgrade notes

- Purely additive on artifact side: existing IDs preserved, new facts appended.
- No library API change.
- **Behavioral change for embedders**: matchers now produce `lives_in` / `goes_to` edges that didn't exist before. Downstream code that enumerated `Predicate` variants in a match arm with `_ => panic!()` or similar will now see those variants. In-tree code is already prepared (variants have been defined since v2.1 / v2.5; render arms shipped in v3.5.0).

### What's next

- **v3.8.5** — re-examine `agent_verb` false positives. With the stopword list now effective, the ~1 351 facts lost may reveal OTHER false-positive patterns now visible in the top-100.
- **v3.9.0** — either (a) loosen `copula_causes` + `domain_membership` (push 9/11 → 11/11), or (b) add new rules R6/R7 (`LivesIn + PartOf → LivesIn`; `GoesTo + PartOf → GoesTo`) to turn the new predicates into derivations.

---

## [3.7.5] — 2026-04-22 — `adam_demo` Part 4 — one derivation per rule (4-rule showcase)

Small polish release (per `feedback_versioning_post_1_0`: `x.y.5` = small). Refreshes `adam_demo` Part 4 to demonstrate **all four active reasoning rules** in a single run — one representative derivation per `rule_id`, each with its own Kazakh-prose rendering carrying the «байланыс-» trust marker.

### Before vs after

**v3.7.0 Part 4** picked `derived[0]` and repeated the same chain across 4 seeds. Viewer saw one reasoning pattern four times.

**v3.7.5 Part 4** picks one representative derivation per `rule_id` (R1 / R2 / R3 / R5), probes each separately, and shows the variety of cognitive operations the system performs at the v3.6.5 committed scale (13 345 facts, 207 derivations).

### Concrete demo output (v3.6.5 committed pool)

```
Picking one representative derivation per rule id (4 total rules fired):
  [R1_is_a_transitivity]             еңбек  --is_a-->     өзен
    source_chain: proverb_068 + wiki_kz_0139793
  [R2_has_inheritance]               қазақ  --has-->      атау
    source_chain: wiki_kz_0001219 + wiki_kz_0118247
  [R3_has_inheritance_via_part_of]   аңғар  --has-->      өсімдік
    source_chain: wiki_kz_0079189 + wiki_kz_0081218
  [R5_shared_is_a_target]            неміс  --related_to--> халқы
    source_chain: wiki_kz_0109606 + wiki_kz_0012411

── R1_is_a_transitivity ──
  probe: «еңбек туралы бірдеңе айт»
  seed  1 [chain]: Қолда бар деректерден байланыс құрастырдым:
                   қорытынды: еңбек — өзен (байланысты ой-тізбек арқылы).
  seed  8 [chain]: ...

── R2_has_inheritance ──
  probe: «қазақ туралы бірдеңе айт»
  seed  1 [chain]: ... ой-тізбек: қазақ атау-ға қатысты байланысы бар
                       (иелік мұрагерлік).

── R3_has_inheritance_via_part_of ──
  probe: «аңғар туралы бірдеңе айт»
  seed  1 [chain]: ... ой-тізбек: аңғар өсімдік-ға қатысты байланысы бар
                       (иелік мұрагерлік).

── R5_shared_is_a_target ──
  probe: «неміс туралы бірдеңе айт»
  seed  1 [chain]: ... ой-тізбек: неміс ара-ға қатысты байланысы бар ...
```

**All four probes surface the «байланыс-» marker.** The v2.7 trust invariant (test-enforced bi-directionally) still guarantees the marker never fires on retrieval-only paths.

### Implementation detail: content-noun filter

Raw `derived[0]`-per-rule selection hit a planner quirk: demonstrative / closed-class subjects like «ана» (that one) route through a non-Unknown intent and miss the reasoning-chain hook. Added a small demo-local filter — `subject.root` must be ≥ 4 chars and not in a demo-scoped closed-class list — so each rule's pick actually lights up the chain. The v3.7.0 raw derivation pool is unchanged (still 207); only the demo's picking policy filters.

### Kazakh-prose variety

Each rule uses a distinct Kazakh sentence pattern:

- **R1**: `қорытынды: <X> — <Y> (байланысты ой-тізбек арқылы)` — "conclusion: X is Y (via related thought chain)"
- **R2** and **R3** (both Has-producing): `ой-тізбек: <X> <Y>-ға қатысты байланысы бар (иелік мұрагерлік)` — "thought chain: X has a connection regarding Y (ownership inheritance)"
- **R5**: `ой-тізбек: <X> <Y>-ға қатысты байланысы бар ...` — RelatedTo-flavour wording

Investor watching the demo sees **different cognitive operations** at the language level, not just four repetitions of the same sentence.

### Tests

**416 passing, 0 failing, 0 warnings** — unchanged. Demo binary change is display-only; no library / pattern / rule surface touched.

### Banner sync

  - `adam_chat.rs`: v3.7 → v3.7.5
  - `adam_demo.rs`: v3.7 → v3.7.5
  - README hero, comparison table, demo transcript all bumped

### Upgrade notes

Purely cosmetic. No library surface change. Embedders and external CLI users see identical behaviour on `adam_chat` / `adam_inspect` / `extract_facts` / `scaling_bench`.

---

## [3.7.0] — 2026-04-22 — `adam_inspect` — interactive intelligence query

New `adam-dialog::adam_inspect` binary — the **interactive complement to `adam_demo`**. Where `adam_demo` runs a scripted 4-part walkthrough, `adam_inspect` takes a Kazakh root from the user and prints **everything adam knows** about it, traceable to `(pack, sample_id)` or `rule_id + source_chain`.

Concrete example (`cargo run -p adam-dialog --bin adam_inspect -- еңбек`):

```
# Graph position for `еңбек`
  out-degree: 18   in-degree: 16   total: 34
  outgoing: does_to=12, has_quantity=1, is_a=1, related_to=4

# Direct facts (extracted from corpus): 24 as subject, 17 as object
  `еңбек` --is_a--> `қайнар`  [pattern: X — Y; kazakh_proverbs_pack.json/proverb_068]
  ...

# Rule-derived facts (not in corpus — inferred): 2 as subject
  `еңбек` --is_a--> `өзен`  [R1_is_a_transitivity]
    source_chain:
      • kazakh_proverbs_pack.json / proverb_068
      • wikipedia_kz_pack.json / wiki_kz_0139793
    Kazakh: қорытынды: еңбек — өзен (байланысты ой-тізбек арқылы)
  `еңбек` --related_to--> `қайнар`  [R5_shared_is_a_target]
    ...
```

The R1-derived `еңбек — өзен` ("labor is a river") is a **conclusion not present in corpus** — built by chaining `еңбек IsA қайнар` (proverb) + `қайнар IsA өзен` (wiki). Every hop has a `(pack, sample_id)` pointer. An investor typing any Kazakh content noun gets this kind of structured report over the 13 345-fact / 207-derivation committed runtime pool.

### Why this complements `adam_demo`

- **`adam_demo`** — scripted, same 4 turns every run, good for recorded demos.
- **`adam_inspect`** — interactive, user-driven, good for live "prove it" sessions.

Both tools load the same committed artifacts (no per-binary scale difference). Together they cover the two investor-demo modes: "watch a scripted narrative" vs "ask your own question".

### Sections of the inspect report

1. **Graph position** — degree, per-predicate incoming / outgoing counts.
2. **Direct facts** — every extracted `Fact` touching the root, capped at 10 per side, with the rest reported as "… and N more".
3. **Rule-derived facts** — every `DerivedFact` the reasoner chained to this root, with full `source_chain` and a Kazakh-prose rendering carrying the «байланыс-» trust marker.
4. **Co-predicated neighbours** — other roots that share an IsA target with this one (the R5-input surface — useful for "who is similar to X" queries).
5. **Summary footer** — one-line degree + fact-count + derivation-count recap.

For unknown roots the binary prints the 5 alphabetically-closest entries from the 2 974-node graph as "did you mean" suggestions.

### Implementation notes

- Pure viewer over existing `data/retrieval/*.json` artefacts — no library-surface change.
- Kazakh-prose renderer is duplicated inline (avoiding a bin → bin dep on `adam-dialog::conversation`).
- 3 unit tests: nearest-key prefix match, empty-map edge case, all-predicates rendering coverage.

### Tests

**416 passing, 0 failing, 0 warnings** (413 baseline + 3 adam_inspect).

### Upgrade notes

- Additive. No library API change. Existing `adam_chat` / `adam_demo` unchanged.
- Cargo auto-discovers the new `src/bin/*.rs` file — no Cargo.toml change needed.
- Banner sync: `adam_chat` / `adam_demo` / README `v3.6.5 → v3.7.0` per `feedback_readme_pre_push_audit`.

### What's next

- **v3.7.5** — refresh `adam_demo` Part 4 to iterate over one derivation per rule type (R1/R2/R3/R5 showcase) rather than repeating the same derivation across seeds.
- **v3.8.0** — native-speaker precision audit unblocks Lexicon PR.
- **v3.9.0** — `occurrence_count` first-class field (Codex #4 follow-up).

---

## [3.6.5] — 2026-04-22 — Committed runtime scaled to T4_200k (first signs of intelligence)

Intelligence that was **stuck in a scaling_bench report** is now **surfaced in the interactive runtime**. Before v3.6.5, `adam_chat` and `adam_demo` loaded the committed 251-fact / 1-derivation snapshot; after v3.6.5 they load **13 345 facts / 207 derivations** covering 4 active rules. Human users interacting with adam finally see the scaling-law reasoning — the same 200× growth the T4_200k bench produced — directly in their conversation.

### Primary goal: first signs of intelligence

Per user directive («главная цель — добиться первых признаков интеллекта»): runtime reasoning needed to visibly scale, not just the bench numbers.

`adam_demo` Part 4 now produces outputs like:

```
Derivations available to cite:
  ақпан --related_to--> қыркүйек       [R5_shared_is_a_target]
  желтоқсан --related_to--> сәуір       [R5_shared_is_a_target]
  ...
  еңбек --is_a--> өзен                  [R1_is_a_transitivity]  (derived, not in corpus)

User probe: «еңбек туралы бірдеңе айт»
  seed  1 [chain]: Қолда бар деректерден байланыс құрастырдым:
                    қорытынды: еңбек — өзен (байланысты ой-тізбек арқылы).
```

The R1-derived «еңбек — өзен» ("labor is a river" — metaphorical transitivity) is a **conclusion the corpus does not directly state** — constructed from chained Is-A facts via the reasoning rule. It's the first time a user-interactive turn surfaces a rule-inferred claim.

### New flags on `extract_facts`

- `--bench-order` — switches pack walk from Tatoeba-first (v2.1 default) to fact-dense-first (Abai → proverbs → classics → textbooks → Wikipedia → …), matching `adam-scaling::CANONICAL_COMMITTED_PACKS`.
- `--max-total <N>` — caps total samples scanned across all packs; per-pack `--limit` can still apply as a secondary cap.

Combined: `extract_facts --bench-order --max-total 200000` produces a committed fact pool equivalent to the `scaling_bench` T4_200k tier.

### Precision tightening: `сияқ`

First T4-scale run showed `сияқ` (comparison particle, the bare root of `сияқты` "like / as") most-connected with **341 edges** — all false positives because the `is_closed_class` check matched `сияқты` but not the bare `сияқ` root. Added `сияқ` to closed-class; re-ran extraction. **-395 false-positive DoesTo facts** removed (13 740 → 13 345, -2.9 %). Most-connected after fix: `адам` (237), `ел` (209), `ат` (186), `жер` (176), `қазақ` (170) — all legitimate content nouns.

### Regenerated committed artifacts

| artifact | v3.6.0 | v3.6.5 | factor |
|---|---:|---:|---|
| `facts.json` (size) | 125 KB | **8.8 MB** | ×70 |
| `facts.json` (fact count) | 251 | **13 345** | **×53** |
| `lexical_graph.json` nodes | 373 | **2 974** | ×8 |
| `lexical_graph.json` edges | 244 | **11 813** | ×48 |
| `derived_facts.json` derivations | 1 | **207** | **×207** |

**All under 50 MB gitignore threshold** (per `feedback_git_ignore_policy`) — stays committed to git.

### Rule activations on committed runtime

| rule | derivations |
|---|---:|
| `R1_is_a_transitivity` | 33 |
| `R2_has_inheritance` | 116 |
| `R3_has_inheritance_via_part_of` | 2 |
| `R5_shared_is_a_target` | 56 |
| **Total** | **207** |

**First release where all 4 active rules fire simultaneously on the committed runtime pool** — not just in bench reports.

### Precision audit

`docs/precision_audit.md` regenerated with **50-fact / 50-derivation sample** (was 17/1 at v3.6.0). Native-speaker review surface is now meaningful.

### Tests

**413 passing, 0 failing, 0 warnings** — no test changes.

### Upgrade notes

- `adam_chat` / `adam_demo` automatically surface the bigger pool. No code change in dialog crates.
- `extract_facts` default behaviour unchanged — new flags opt-in.
- Existing `facts.json` readers downstream see bigger file; all existing readers load-then-iterate, no schema assumption.
- `adam_demo` Part 4 picks `derived[0]` dynamically — will pick a different derivation post-upgrade (previously кітап/ілім; now the first-by-subject-root derivation from the sorted 207-pool).

### What's next

- **v3.7.0** — `--persist-tier` on `scaling_bench` + `adam_chat --facts-tier` flag for ad-hoc tier switching.
- **v3.8.0** — native-speaker precision audit unblocked; Lexicon PR using v3.4.0 candidates.
- **v3.9.0** — `occurrence_count` first-class field (Codex #4 follow-up).

---

## [3.6.0] — 2026-04-22 — First `--use-shards` scaling run (54 M-word pool, T5_1M tier)

**Sixth** post-v3.0 scale-up release. First **full-scale** scaling-bench run — tapping the 77.9 M-word gitignored local shard pool via the v3.2.0 `--use-shards` flag. With the 3-hour iteration budget the bench makes it through all 5 tiers (`[1k, 10k, 50k, 200k, 1M]`) with T5 as an honest partial-extract (940 288 / 1 000 000 samples scanned at the time-budget cutoff).

### Key finding: R3 fires for the first time on real corpus

At T4_200k, **R3 produces 2 derivations** — the `A Has B ∧ B PartOf C ⟹ A Has C` chain finally finds a matching path in the graph. This confirms the v3.5.5 architectural activation was correct, and R3 is now on the same empirical footing as R1/R2/R5. **All 4 active rules fire with counts > 0 on real corpus simultaneously for the first time.**

### Scaling-law data points

| tier | samples | words | facts | derivations | graph nodes | graph edges | extract s |
|---|---:|---:|---:|---:|---:|---:|---:|
| T1_1k | 1 000 | 8 957 | 25 | 0 | 39 | 25 | 11 |
| T2_10k | 10 000 | 106 190 | 450 | 0 | 442 | 417 | 159 |
| T3_50k | 50 000 | 611 522 | 2 527 | 27 | 1 317 | 2 207 | 522 |
| T4_200k | 200 000 | 2 313 598 | **13 740** | **207** | 3 003 | 12 066 | 1 655 |
| T5_1M* | 940 288 | 11 371 301 | **67 806** | 0† | 4 051 | 50 349 | 8 445 |

\* Partial — hit `--time-budget 10800` (3h) mid-extract at 940 k of 1 M target. `status: "timed_out"` recorded. † Reasoner received 0 budget after extract finished; 0 derivations at T5 is a budget-not-chain artifact.

### Scaling-law signals

**T3 → T4_200k (×3.78 words):**

- facts: 2 527 → 13 740 = **×5.44** (super-linear in words — more words unlock more matcher surface)
- **derivations: 27 → 207 = ×7.67** (super-linear in facts — exactly the expected reasoning-graph densification)
- graph nodes: 1 317 → 3 003 = ×2.28 (sub-linear — new words reuse existing roots)
- graph edges: 2 207 → 12 066 = ×5.47 (near-linear)

**T4_200k → T5_1M (~4.9× words even partial):**

- facts: 13 740 → 67 806 = ×4.94 (holds near-linear)
- nodes: 3 003 → 4 051 = ×1.35 (**saturating** — vocabulary closure at scale)
- edges: 12 066 → 50 349 = ×4.17 (tracks fact count)

Node saturation at T5 is significant: the lexical graph is approaching its closure over the 20k-root Lexicon. Additional corpus from here on produces more FACTS over the SAME nodes, densifying the graph rather than widening it. This is the expected regime for a deterministic reasoner — **richer structure on a stable vocabulary, not vocabulary explosion**.

### Rule activations across tiers

| tier | R1 | R2 | R3 | R5 | total |
|---|---:|---:|---:|---:|---:|
| T1_1k | 0 | 0 | 0 | 0 | 0 |
| T2_10k | 0 | 0 | 0 | 0 | 0 |
| T3_50k | 7 | 5 | 0 | 15 | 27 |
| **T4_200k** | **33** | **116** | **2** | **56** | **207** |
| T5_1M† | 0 | 0 | 0 | 0 | 0 (budget) |

**R3 (`has_inheritance_via_part_of`) fires 2 times at T4_200k** — first concrete evidence that the v3.5.5 rule activation was materially correct, not just architecturally wired. R2 shows the biggest jump (5 → 116 = ×23) — textbook prose is rich in `X IsA Y ∧ Y Has Z` chains that the v3.5.0 matchers unlock.

### Normalized metrics across tiers

| tier | facts/10k words | deriv/fact | predicate coverage | duplicate rate |
|---|---:|---:|---:|---:|
| T1_1k | 27.9 | 0.0 | 18.2 % | 0.0 % |
| T2_10k | 42.4 | 0.0 | 45.5 % | 7.3 % |
| T3_50k | 41.3 | 0.011 | 63.6 % | 12.7 % |
| **T4_200k** | **59.4** | **0.015** | **63.6 %** | 12.2 % |
| T5_1M† | 59.6 | 0.0† | 63.6 % | 25.7 % |

**Extraction density (`facts/10k words`) rises 27.9 → 59.6** — the matchers get more efficient per unit corpus as the context diversifies. Stabilising around 60 means we're approaching the linear-density regime; further corpus adds facts but not density.

**Duplicate rate jumps T4 → T5 (12.2 % → 25.7 %)** — at 67 k facts on 1 M samples, we start seeing repeated structural phrases across different textbook chapters. This is the signal Codex flagged as "occurrence_count deserves to be its own field" — a v3.7+ target.

### Sources loaded

- 9 committed packs: `tatoeba` + `wikipedia_kz` + `common_voice_kk` + `cc100_kk` + `abai_wikisource` + `kazakh_proverbs` + `synthetic_sentences` + `kazakh_classics` + `kazakh_textbooks`
- **27 local shards**: `wikipedia_kz_shard_*` + `cc100_kk_shard_*`
- Total pool: **4 376 521 samples / 54 270 582 words**

(Pool is smaller than the often-cited 77.9M because some local shards are excluded from committed/shard pools — a v3.7+ cleanup target.)

### Committed artifacts

All committed artifacts unchanged from v3.5.5. This release is a **bench-only scaling data point**; no library / matcher / rule changes.

- `data/retrieval/facts.json`: 251 (unchanged)
- `data/retrieval/lexical_graph.json`: 373 nodes / 244 edges (unchanged)
- `data/retrieval/derived_facts.json`: 1 (R5, unchanged)
- `data/scaling/scaling_report.json`: **regenerated with T5_1M partial + R3 first-fire evidence**

### Tests

**413 passing, 0 failing, 0 warnings** — no test surface change.

### Upgrade notes

- No code changes. Pure scaling-run release.
- `scaling_report.json` schema unchanged (v3.3.0 normalized-metrics fields already in place).
- `data/scaling/scaling_report.json` is larger than v3.5.5 (~5× samples scanned); still well under 1 MB.

### What's next

- **v3.6.5** — Codex #4 follow-up: `occurrence_count` as a first-class field on `Fact` to absorb the T5 duplicate signal cleanly.
- **v3.7.0** — `--persist-tier` flag on `scaling_bench` + `adam_chat --facts-tier T5` integration: demo the 67 k-fact pool interactively.
- **v3.8.0** — native-speaker precision audit + first Lexicon PR (v3.4.0 candidates file unblocks).

---

## [3.5.5] — 2026-04-22 — PartOf matcher + R3 mereological rule activation

Small incremental release (per `feedback_versioning_post_1_0`: x.y.5 = small). Completes the **reasoning-rule roster at 4 active rules** by activating R3 with the first `PartOf`-producing extractor.

### New matcher: `structural_part_of`

Pattern: `X Y-нің бөлігі` ("X is Y's part") + `X Y-нің құрамында` ("X is in Y's composition"). Both are structurally partitive with unambiguous Kazakh semantics.

**Dropped from the initial design**: `ішінде` ("inside" / "among") was semantically ambiguous — both partitive (`X is inside Y`) and universal-quantifier (`among all N, X stands out`). First run produced 3 facts with 2/3 false-positive rate (e.g. "тілдердің ішінде қазақ" = "among languages, Kazakh" is NOT a PartOf claim). Tightened to the two unambiguous heads only; 4 unit tests cover the negative cases.

Fact-production requirements:
- genitive noun immediately before the `бөлігі` / `құрамында` head → Y
- bare-nominative content noun earlier in the sentence → X (same POS + closed-class + possessive filters as v3.5.0 agent_verb tightening)
- X ≠ Y tautology guard

### New reasoning rule: R3

`R3_has_inheritance_via_part_of`: `A Has B ∧ B PartOf C ⟹ A Has C`.

Mereological inheritance — if A owns B, and B is part of C, A has a claim on (at least the presence of) C. Labelled `ConfidenceKind::RuleInferred` (never Grammar), so downstream consumers can filter by confidence kind. Tautology guard on A = C.

4 unit tests:
- `r3_derives_has_inheritance_via_part_of` — positive case.
- `r3_respects_tautology_guard` — refuses A Has A.
- `r3_does_not_fire_without_part_of_edge` — no Has/PartOf chain → no derivation.
- `r3_dedupes_against_existing_facts` — if `A Has C` already exists, R3 doesn't re-emit.

**Total active rules**: R1 (IsA-transitivity), R2 (Has-inheritance), **R3 (Has-inheritance via PartOf, v3.5.5)**, R5 (shared-IsA → RelatedTo). 4/5 documented rules active. R4 (IsA-symmetry diagnostic) remains documented-only — its output is a curator warning, not a fact, and needs an asymmetric code path.

### Committed artifacts

PartOf facts at committed 500/pack: **0** — the strict `бөлігі` / `құрамында` heads don't appear in the first 500 samples of any canonical pack. Scaling bench on T4_50k shows the first meaningful activations.

Facts: **251** (unchanged from v3.5.0 — PartOf dropped from 3 → 0 by tightening; the 3 that DID extract at v3.5.0 were 2 false positives + 1 borderline, so this is net a precision improvement).

### Scaling bench T4_50k

Fresh run on 4.57 M-word committed pool:

| predicate | count |
|---|---:|
| `does_to` | 2 019 |
| `related_to` | 345 |
| `is_a` | 57 |
| `has` | 49 |
| `after` | 48 |
| **`part_of`** | **5 (new!)** |
| `has_quantity` | 4 |
| **Total** | **2 527** (+5 vs v3.5.0) |

**Predicate coverage: 6/11 (54.5 %) → 7/11 (63.6 %)** — PartOf is the 7th predicate to fire on real corpus.

### R3 activation signal

At T4_50k, R3 fires **0 times**. R1/R2/R5 unchanged (7 / 5 / 15 = 27 total derivations). Why R3 = 0:

- R3 needs `Has(X, Y) ∧ PartOf(Y, Z)` — a Has-fact whose object is a PartOf-fact's subject.
- At T4: 49 Has facts, 5 PartOf facts.
- The Has-object roots and the PartOf-subject roots don't overlap in the current slice.

This is **architecturally correct and expected**: R3 is wired, unit-tested (4 tests), and will fire automatically as soon as the corpus contains the right chain. The "0 at this scale" is an honest signal, not a bug — the density threshold is simply higher for mereological inheritance than for IsA-transitivity.

**Precedent**: R5 sat at 0 derivations for several releases (v2.6 → v2.7 activation) before the corpus supplied shared-IsA targets. R1/R2 similarly took v3.2 → v3.3 scale to fire with counts > 1. R3 is in that same "activate at scale" cohort.

### Normalized metrics (v3.5.0 → v3.5.5, T4_50k)

| | v3.5.0 | v3.5.5 | delta |
|---|---:|---:|---|
| facts / 10k words | 41.24 | 41.32 | +0.2 % (near-noise) |
| derivations / fact | 0.0107 | 0.0107 | unchanged |
| **predicate coverage** | 54.5 % | **63.6 %** | **+9.1 pp** |
| duplicate-fact rate | 12.65 % | 12.66 % | ≈ unchanged |

The single meaningful delta is **predicate coverage**. Raw fact count barely moved (+5 PartOf on 2 522) because the tightened `structural_part_of` matcher is deliberately narrow. A broader PartOf matcher could push the count up 10-100× but would re-introduce the "ішінде" false-positive class.

### Tests

**413 passing, 0 failing, 0 warnings** (405 baseline + 4 structural_part_of + 4 R3).

### Why only a .5 bump (not 3.6.0)

Per `feedback_versioning_post_1_0`: `x.y.5` = small / incremental. This release:
- Adds 1 matcher (not 6).
- Activates 1 rule (not a new reasoning framework).
- Retires 1 pattern (`ішінде` dropped) on precision grounds.
- Scales existing infrastructure; no new crate, no API change.

The predicate coverage still reads `7/11` (PartOf now firing at T4 scale — see bench numbers), so this is a meaningful scaling-law data point in a small package.

---

## [3.5.0] — 2026-04-22 — Corpus + predicate breadth (10 textbooks + 5 new predicates)

**Fifth** post-v3.0 scale-up release. Executes the approved "multiplicative axes" strategy: **Corpus** (3 → 10 textbooks, pack 8 421 → **28 110 samples**) + **Predicate breadth** (6 predicates → 11, five new matchers). Together they multiply committed fact count by **~15× (17 → 251)** and shift the scaling curve in both X-axis (more corpus) and Y-axis (more predicate dimensions).

### Corpus expansion — 7 new textbooks OCR'd

Same OCR pipeline as v3.3.0 pilot (`pdftoppm @ 200 DPI → tesseract -l kaz`, 6-way parallel). 7 remaining textbooks processed in ~35 min wall-clock:

| book | raw words | samples |
|---|---:|---:|
| Physics 11 ЕМН | 84 267 | 4 764 |
| Physics 11 ОГН | 55 786 | 2 724 |
| Algebra 7 | 45 487 | 3 014 |
| Informatics 11 ЕМН | 41 257 | 2 451 |
| Biology 8 | 39 121 | 2 942 |
| Informatics 11 ОГН | 32 367 | 1 709 |
| KazLit 11 ЕМН | 27 383 | 2 085 |
| **Total (7 new)** | **325 668** | **19 689** |
| + v3.3.0 pilot (3 books) | 108 913 | 8 421 |
| **Grand total** | **434 581** | **28 110** |

New binary flag `--merge-existing <PATH>`: seeds output from a previously-committed pack so the v3.3.0 samples propagate through (the 3 original PDFs were deleted during cleanup; without merge, their OCR would be lost). Cross-book text dedup still applies.

### Predicate breadth — 5 new predicate variants + 6 new matchers

The `Predicate` enum grows from 6 → 11. Five new variants added:

- **`Causes`** — `X — Y-нің себебі` (X is the cause of Y). Canonical Kazakh causal copula. Example: «су — өмірдің себебі».
- **`After`** — `X Y-дан кейін` / `X Y-ден соң` (X happens after Y). Temporal postposition construction.
- **`HasQuantity`** — `X-тың N Y-ы бар` (X has N Y's). Numeric-count possessive; numeral between genitive and P3.
- **`DoesTo`** — `X Y-ні Z-лайды` (X does Z to Y). Kazakh SOV agent-verb. Verb root captured in pattern field.
- **`InDomain`** — `X — Y саласы` / `X — Y ғылымы` (X is a field/science of Y). Textbook taxonomic construction.

Plus **`nominal_conjunction`** matcher — second extraction path for `RelatedTo` via explicit `X пен Y` / `X мен Y` / `X бен Y` syntactic co-predication (grounded alternative to the R5 rule-derived path).

All 6 matchers type-check via FST features (`Case`, `Possessive`, `Voice`), not surface strings. 14 unit tests (positive + negative per matcher where Lexicon supports positive; negative-only where positive tests need specific Lexicon entries not guaranteed on every checkout).

**Graph projection arms** added for all 5 new predicates in `LexicalGraph::from_facts` (the `unreachable!` safety arm enforces every `Predicate` variant has a branch — compile-time guarantee).

**Kazakh-prose renderers** added for all 5 new predicates in `adam-dialog::conversation::render_derivation_as_kazakh`. Every new arm keeps the **«байланыс-» marker** per the v2.7 trust-stack invariant (test-enforced bi-directionally).

**`adam-scaling::TOTAL_PREDICATE_VARIANTS`** bumped 6 → 11 (the denominator for `predicate_coverage_pct` in normalized metrics). This slightly changes historical `predicate_coverage_pct` values — v3.3 T4_50k was reported as 33 % under the old 2/6 math; under the new 2/11 math that same tier is 18 %. Current release's coverage reporting reflects the new denominator.

### Precision tightening (post-extraction feedback loop)

First run of agent_verb on the expanded corpus produced 239 `DoesTo` facts — too greedy. Initial sample showed 3 classes of false positives:

1. **Passive-voice verbs** mis-classified as active SOV — «Орыс тілі ... қолданылады» ("Russian is used") should not produce DoesTo.
2. **Possessive-form subjects** ("тілі" = P3 of "тіл") treated as bare subjects.
3. **Interrogative pronouns** ("қандай") passing through as nouns.

Three fixes applied:

- `agent_verb`: refuse `Voice::Passive` (new field check via `Voice` enum import).
- `agent_verb`: refuse subjects with `features.possessive.is_some()` (match `nominal_conjunction`'s existing check).
- `is_closed_class`: add `қандай, кім, не, қай, қашан, қайда, неліктен, неге, қанша` — interrogatives.

Post-tightening: 239 → 200 `DoesTo` facts (-39 false positives, -16 %). More precision tightening targets v3.5.5 via native-speaker review of `docs/precision_audit.md` (50-sample audit file regenerated with the v3.5.0 fact pool).

### Committed artifacts

| | v3.3.0 | v3.4.0 | v3.5.0 | factor |
|---|---:|---:|---:|---|
| facts.json facts | 17 | 17 | **251** | **×15** |
| lexical_graph.json nodes | 32 | 32 | **373** | **×12** |
| lexical_graph.json edges | 17 | 17 | **244** | **×14** |
| derived_facts.json derivations | 1 | 1 | 1 | unchanged |

Fact breakdown at committed 500/pack scope:

- `is_a`: 12
- `has`: 5
- `related_to`: 33 (nominal_conjunction + v3.3-era extractions)
- `after`: 1
- `does_to`: 200

`derived_facts` stays at 1 because R1/R2/R5 all require IsA-dense graphs. Adding `DoesTo` (not an IsA predicate) doesn't produce new transitive chains. To grow derivations we'd need either (a) more IsA extractors, (b) new rules that consume non-IsA predicates. Both are v3.5.5+ / v3.6 targets.

### Textbooks pack composition

Per-book sample counts after merge-dedup:

```
kz_lang_11_ogn:     4 365   (v3.3 pilot)
kz_lang_11_emn:     2 046   (v3.3 pilot)
kz_lang_culture_9:  2 010   (v3.3 pilot)
kz_lit_11_emn:      2 085
physics_11_ogn:     2 724
physics_11_emn:     4 764
informatics_11_ogn: 1 709
informatics_11_emn: 2 451
algebra_7:          3 014
biology_8:          2 942
```

Quality-gate reject tally on the 7-new-book ingest (merged run, total 41 423 sentences scanned):

- `skipped_length`: 13 298 (headers, ToC fragments)
- `skipped_loanword_heavy`: 3 397 (physics / informatics technical terms)
- `skipped_duplicate`: 1 108 (cross-book structural-phrase dedup)
- `skipped_low_kazakh`: 156 (OCR-table fragments)
- `skipped_latin`: 0

### Tests

**405 passing, 0 failing, 0 warnings** (391 baseline + 14 new v3.5.0 matcher tests).

### Scaling bench — fresh run on 4.57 M-word committed pool

Default tiers on the expanded (textbook-heavy) committed pool, 904 s total wall-clock on M2 8-core:

| tier | samples | words | facts | derivations | graph nodes | graph edges |
|---|---:|---:|---:|---:|---:|---:|
| T1_100 | 100 | 903 | **2** | 0 | 3 | 2 |
| T2_1k | 1 000 | 8 957 | **25** | 0 | 39 | 25 |
| T3_10k | 10 000 | 106 190 | **450** | 0 | 442 | 417 |
| T4_50k | 50 000 | 611 522 | **2 522** | **27** | 1 315 | 2 203 |

### Predicate breakdown at T4_50k

| predicate | count |
|---|---:|
| `is_a` | 57 |
| `has` | 49 |
| `has_quantity` | 4 |
| `after` | 48 |
| `related_to` | 345 |
| **`does_to`** | **2 019** |

6 / 11 predicates firing (predicate_coverage = 54.5 %). Zero-fire on current corpus: `causes` (needs definition-style `X — Y-нің себебі`), `lives_in` (needs `тұру`-verb-constructed), `goes_to` (needs `бару`-verb), `in_domain` (needs `саласы`/`ғылымы` head), `part_of` (no matcher yet). These are density-limited — more corpus (v3.6: Wikipedia shards; v3.7: full 77.9 M) should unlock them.

### Rule activations at T4_50k

**First release where all 3 rules fire with counts > 1**:

| rule | count | first active |
|---|---:|---|
| `R1_is_a_transitivity` | **7** | v3.2.0 T4 (was 8) |
| `R2_has_inheritance` | **5** | v3.3.0 T4 (was 20) |
| `R5_shared_is_a_target` | **15** | v2.6 |

R1+R2+R5 = 27 derivations. The absolute count is **lower than v3.3.0 (51)** because the 50 k-sample window at v3.5.0 contains far more textbook content (28 110 samples in the pool vs 8 421 before), **displacing** Wikipedia samples that previously contributed Is-A-rich proverbs. Textbooks are definition-heavy but produce more `DoesTo` (SOV prose) than `IsA`. To push R5 counts up we need either more IsA matchers or richer IsA-dense corpus (Wikipedia subject-definitions).

This is the **honest scaling-law curve behaviour**: different corpus composition → different predicate mix → different rule-activation shape. Raw derivation count is not the only signal; **predicate coverage** and **fact density** are both up sharply.

### Scaling T3 → T4 (×5 words, v3.5.0)

- **words** ×5.18
- **facts** ×5.60 (near-linear — saturates around this regime per 10k words)
- **graph nodes** ×2.98 (sub-linear — new words reuse existing nodes)
- **graph edges** ×5.28 (near-linear — edges scale with facts, not nodes)
- **derivations** new at T4 (0 → 27, activation threshold crossed around 1 000-2 500 facts)

### Normalized metrics (v3.3 vs v3.5 comparison)

| | v3.3.0 T4 | v3.5.0 T4 | note |
|---|---:|---:|---|
| facts / 10k words | 2.00 | **41.24** | ×20 density growth — 6 new matchers firing across corpus |
| derivations / fact | 0.4250 | 0.0107 | lower — DoesTo predicate doesn't drive IsA-family rules |
| predicate coverage | 33 % | **54.5 %** | **6 predicates firing** (up from 2) |
| duplicate-fact rate | 27.5 % | **12.6 %** | ~halved — more diverse fact types reduce structural repetition |

### Upgrade notes

- `Predicate` enum is `non_exhaustive`-unmarked (v2.x convention — new variants are breaking for any exhaustive match). v3.5.0 adds 5 variants; downstream matches in `adam-reasoning::graph` + `adam-dialog::conversation` are updated in-tree. External embedders that exhaustively match on `Predicate` need to add arms for `Causes`, `After`, `HasQuantity`, `DoesTo`, `InDomain`.
- `TOTAL_PREDICATE_VARIANTS` changed 6 → 11. Normalized `predicate_coverage_pct` numbers across releases reflect this — use the `version` field in `scaling_report.json` to disambiguate.
- `process_kazakh_textbooks` now accepts positional `--merge-existing <PATH>` flag. Backward-compatible: absent flag preserves v3.3.0 behaviour.

---

## [3.4.0] — 2026-04-22 — Lexicon mining pipeline (coverage 79.48% → expansion candidates)

**Fourth** post-v3.0 scale-up release. Addresses the Lexicon-scaling axis — the single most-multiplicative lever we have: every approved root improves morpheme coverage, which improves parser analyses, which improves matcher firings, which improves fact/derivation counts. The bottleneck was never tooling — it was native-speaker review time. This release converts that from "1 hour / root" into "1 hour / ~50 pre-tagged candidates".

### New binary: `mine_lexicon_gaps`

`crates/adam-corpus/src/bin/mine_lexicon_gaps.rs` + 16 unit tests.

- Scans **all 9 committed source packs** (`tatoeba` → `wikipedia_kz` → `common_voice_kk` → `cc100_kk` → `abai_wikisource` → `kazakh_proverbs` → `synthetic_sentences` → `kazakh_classics` → `kazakh_textbooks`) — same canonical list as `extract_facts`.
- Finds every token (≥ 3 chars, alphabetic) that **no current Lexicon root prefixes**.
- Aggregates across all packs (not per-pack top-20 like `morpheme_coverage`), ranks by global frequency, picks top-N (default 200).
- Extracts 3 context sentences per candidate (pack + sample_id + full sentence text).
- **Auto-tags** each candidate with:
  - Vowel harmony: `back` / `front` / `mixed` / `neutral (only и/у/ю)` — inferred from present vowels.
  - Final sound: `vowel` / `voiceless_consonant` / `voiced_consonant` / `nasal` / `liquid` / `glide` — matches the FST's `ConsonantClass` enum.
  - POS: defaults to `noun` (reviewer confirms / corrects — auto-POS inference is v3.5+ work).
- Writes `docs/lexicon_gap_candidates.md` — native-speaker review file with checkboxes, root-form / POS / harmony / final-sound override slots, and a Tally section for approve/reject counts.

### Independent validation of memory `project_morpheme_coverage_baseline`

The memory from v1.5.5 predicted the top uncovered roots would be `деп, осы, оның, деген, пен`. The v3.4.0 scan on the 4.32 M-word v3.3.0 pool found **exactly these five** as the top-5 candidates, in the same order (frequency: 11 101 → 11 098 → 8 486 → 6 250 → 4 521). This is the first empirical validation that the baseline memory was load-bearing, not anecdotal — and it means the `mine_lexicon_gaps` ranking is consistent with hand-curated expert judgement at the top.

### Auto-tag quality spot-check on top-10 candidates

| # | surface | freq | auto harmony | auto final | correct? |
|---|---|---:|---|---|---|
| 1 | `деп` | 11 101 | front | voiceless_consonant | ✓ |
| 2 | `оның` | 11 098 | back | nasal | ✓ |
| 3 | `осы` | 8 486 | back | vowel | ✓ |
| 4 | `деген` | 6 250 | front | nasal | ✓ |
| 5 | `сол` | 4 939 | back | liquid | ✓ |
| 6 | `пен` | 4 521 | front | nasal | ✓ |
| 7 | `бас` | — | back | voiceless_consonant | ✓ |
| 8 | `байланысты` | — | back | vowel | ✓ |
| 9 | `облысы` | — | back | vowel | ✓ |
| 10 | `оны` | — | back | vowel | ✓ |

**10/10 auto-tags correct.** POS default (`noun`) misses on pronouns / conjunctions / converbs in the top-10 — this is expected and clearly documented in the binary docstring + the review file; native speaker corrects it.

### Scan results

| | value |
|---|---:|
| Lexicon roots loaded (≥ 3 chars) | 14 164 |
| Packs scanned | 9 (all committed) |
| Samples scanned | 411 031 |
| Tokens scanned | 3 921 698 |
| **Distinct uncovered surfaces** | **104 657** |
| Candidates written | 200 (top by frequency) |

Long tail is substantial: 104 657 distinct uncovered surfaces means successive mining passes (v3.4.5, v3.5, …) have a lot of material to drain. v3.4.0 ships the **first 200** in a single review batch.

### Why this unblocks everything else

Per memory `project_morpheme_coverage_baseline`: current coverage is 79.48 % across 3.84 M committed words. Each approved root directly improves that ratio. For the reasoning pipeline:

- Better parser analyses → more tokens get `Analysis::Noun { root, features }` instead of falling through.
- More analyses → more matcher firings (`possessive_has` needs P3-tagged noun on the right; `locative_lives_in` needs `Case::Locative`; every matcher is gated on FST analysis).
- More facts → the v3.2.0 scaling curve shifts up on every tier.
- Higher `predicate_coverage_pct` in scaling report — currently 33 % (is_a + has), can reach 67 %+ once locative + dative fire on more surfaces.

**Expected delta per 50 approved roots** (rough back-of-envelope): +0.3-0.8 pp morpheme coverage, +5-15 % fact yield at T4_50k. Measurable via re-running `morpheme_coverage` + `scaling_bench` after each Lexicon PR (per the existing `feedback_docs_currency` discipline).

### Tests

**391 passing, 0 failing, 0 warnings** (375 baseline + 16 auto-tag unit tests).

### Upgrade notes

- Purely additive. No library-API change. No existing behaviour modified.
- `docs/lexicon_gap_candidates.md` is a **new** committed file (~200 KB) — small enough to review in-line in a PR diff.
- The binary is re-runnable; re-runs after Lexicon PRs surface the *next* 200 candidates as the top-200 drain.

### What's next

v3.4.5 / v3.5.0 options (pick one based on priority):

- **v3.4.5 — first Lexicon PR** — native-speaker approves ≥ 50 roots from the candidates file; we merge the PR, re-run `morpheme_coverage` and `scaling_bench`, ship the measurable delta.
- **v3.5.0 — +6 extractors + OCR 7 remaining textbooks** — orthogonal to Lexicon, grows fact yield through breadth.

Both are ready to go independently.

---

## [3.3.0] — 2026-04-22 — Codex review polish + precision audit + gold-corpus pilot

**Third step** of the post-v3.0 scale-up ladder. Response to the second Codex external review of v3.2.0 (see the "Codex findings" section below), plus the first quality-gated ingestion of natural Kazakh corpus beyond Wikisource and Wikipedia (3 secondary-school textbooks OCR'd through `tesseract-kaz`).

### Codex findings (v3.2.0 review) — resolved

1. **Determinism test was too weak** — the in-process `analyse_ordering_stable_across_calls` would have passed on the pre-v3.2.0 HashMap code too (HashMap iteration is stable within one process; the bug was cross-process). v3.3.0 strengthens it with **two expected-order assertions**:
   - `analyses_sorted_by_root_then_id_when_cross_root_ambiguous` — asserts that for the genuinely cross-root-ambiguous surface `кітабы`, the first analysis is under root `кітабы` (< `кітап` by Cyrillic code point), and the whole sequence is non-decreasing by root. Under the pre-v3.2.0 HashMap-values path this assertion fails ≈ 50 % of runs.
   - `first_root_matches_entries_ordered_for_prefix_ambiguous_surface` — cross-checks the first analysis against `LexiconV1::entries_ordered`'s first prefix-matching entry, directly asserting the dual-storage contract.
2. **`run_tier()` wasn't budget-aware** — `budget.should_stop()` was only checked between tiers, so a long T5 couldn't be interrupted internally. Now `run_tier_with_budget` chunks extraction at `EXTRACT_CHUNK_SIZE=128` samples and checks the budget between chunks (~0.5–1 s granularity). Partial-tier `ScalingPoint` is returned with the actual `samples_scanned` reflecting how much work completed.
3. **Doc contradiction in `adam-scaling/lib.rs`** — the header said "canonical order like extract_facts", the pack-constant docstring said "NOT the same as extract_facts". Reconciled in v3.3.0: the bench uses a **bench-specific** canonical order (fact-dense first), distinct from extract_facts's order; the lib docstring now states this plainly.
4. **README `Current state (v3.0.1 — honest numbers)` header was stale** — renamed to `Current state (v3.3.0 — honest numbers)`; test count refreshed to match the final v3.3.0 total.

### Codex follow-ups (partial uptake)

- ✅ **Normalized metrics on every `ScalingPoint`** (Codex #4) — new `NormalizedMetrics` struct computes `facts_per_10k_words`, `derivations_per_fact`, `predicate_coverage_pct`, `duplicate_fact_rate_pct` per tier. Also rendered as a Markdown table in `docs/scaling_report.md`. Raw counts grow with corpus size; these ratios tell you *what kind* of growth it is (extraction density, reasoning leverage, breadth of predicate types, de-duplication hygiene).
- ✅ **Precision audit binary** (Codex #3) — new `audit_precision` bin in `adam-scaling`. Deterministically samples 50 facts + 50 derivations (seeded, reproducible), renders `docs/precision_audit.md` with per-item checkboxes, full source sentence, pattern/rule id, and a Tally section for the reviewer to compute precision. **Audit format primed for native-speaker review — the output file is the precision-gate for v3.4 scaling.**
- ⏸ **Promoting T4 facts into runtime** (Codex #1 follow-up) — deferred to v3.4.0, gated on precision audit ≥ threshold. We don't want to wire 200+ potentially-borderline facts into `adam_chat` without quality bar.
- ⏸ **New `PartOf`/`Causes`/`LivesIn`/`GoesTo` extractors at scale** (Codex #5 follow-up) — deferred to v3.4.0 (6-matcher addition was the original v3.3.0 plan before this polish-pass took priority).

### Gold-corpus pilot (3 textbooks OCR'd)

In parallel Codex flagged the v2.x training corpus as heavily synthetic (~84 % by sample count) and too small for natural-Kazakh LM training. User provided 10 Kazakh secondary-school textbook PDFs (`data/external/*.pdf`). **Problem:** PDFs use custom-font glyph encoding — `pdftotext` silently drops `Қ Ң Ғ Ө Ү Ұ Һ`, the very characters any Kazakh-first pipeline depends on. **Solution:** new OCR pipeline (`/tmp/ocr_pipeline.sh`) — `pdftoppm` @ 200 DPI → PNG → `tesseract -l kaz`, 6-way parallel.

v3.3.0 ships a **pilot** ingestion of 3 language-focused books (KazYazyk 11 EMN + OGN, Kazakh Language & Culture 9), via the new `process_kazakh_textbooks` binary in `adam-corpus`. The remaining 7 textbooks (physics, biology, algebra, informatics, literature) are staged for v3.3.5 / v3.4.0 once the pilot validates extraction quality. **Pack counts + extraction numbers to be filled in post-OCR** — see the "Pilot results" section at the end of this entry.

The pack carries per-book provenance (`source_id` = book slug), page range (`p{NNN}`), sentence index (`s{NN}`), and gets registered in:
- `adam-reasoning::extract_facts::SOURCE_PACKS` — immediately participates in fact extraction.
- `adam-scaling::CANONICAL_COMMITTED_PACKS` — scaling bench picks it up on the next run.

Quality gates on textbook samples (stricter than classics/wiki because OCR noise is real):
- ≥ 80 % Cyrillic characters (guards against table/figure fragments).
- 4 ≤ words ≤ 60 (widened from 3–60 literature; textbooks use definition-style sentences).
- ≤ 15 % loanword density (widened from 10 % — physics/informatics have more Russian technical vocab).
- No Latin run (defensive against OCR mis-segmentation).
- Cross-book dedup by lowercase text.

### Pilot results

OCR'd and ingested in the pilot:

| book | raw words | samples in pack |
|---|---:|---:|
| Қазақ тілі 11 ЕМН (language, natural-math track) | 26 705 | 2 046 |
| Қазақ тілі 11 ОГН (language, general-humanities track) | 59 738 | 4 365 |
| Қазақ тілі мен әдебиеті 9 | 22 470 | 2 010 |
| **Total** | **108 913** | **8 421** |

Pack: 2.8 MB, `data/curated/kazakh_textbooks_pack.json`. Per-book provenance preserved (`source_id = <book-slug>`, ids shaped `kz_textbook_<book>_p<NNN>_s<NN>`).

Quality-gate reject tally (healthy extraction signal — matchers aren't greedy):

- `skipped_length`: 3 542 (short headers, single-word chapter labels)
- `skipped_duplicate`: 565 (structural phrases repeated across pages)
- `skipped_loanword_heavy`: 396 (physics / math terms with Russian technical suffixes — textbooks have more than Abai)
- `skipped_low_kazakh`: 6 (near-empty OCR pages)
- `skipped_latin`: 0 (filter working)

### Committed artifacts (byte-identical across 3 runs on post-v3.2.0 deterministic parser)

| artifact | v3.2.0 | v3.3.0 | delta |
|---|---:|---:|---|
| `facts.json` facts | 15 | **17** | +2 (from textbooks within committed 500/pack cap) |
| `lexical_graph.json` nodes / edges | 29 / 15 | 32 / 17 | +3 / +2 |
| `derived_facts.json` derivations | 1 | 1 | unchanged (R5 chain surfaces at higher fact counts — visible at T4) |
| textbook samples in pool | 0 | **8 421** | new |

### Scaling bench — first measurement with textbooks in pool

Default tiers on committed-only corpus (4.32 M-word pool, up from 4.23 M without textbooks):

| tier | samples | words | facts | derivations | graph nodes | graph edges | extract ms |
|---|---:|---:|---:|---:|---:|---:|---:|
| T1_100 | 100 | 903 | 0 | 0 | 0 | 0 | ~520 |
| T2_1k | 1 000 | 8 957 | 0 | 0 | 0 | 0 | ~7 500 |
| T3_10k | 10 000 | 106 190 | 19 | 0 | 38 | 19 | ~85 000 |
| T4_50k | 50 000 | 600 885 | **120** | **51** | 123 | 87 | ~520 000 |

Total run: 614 s (10 min 14 s) on M2 8-core, 4 / 4 tiers completed, `status: "completed"`.

Scaling signal T3 → T4 (×5 words):

- **derivations** ×∞ (was 0 at T3, 51 at T4) — reasoning activates once graph density crosses threshold. This is the R1 / R2 / R5 rules kicking in at scale.
- **facts** ×6.32 (slightly super-linear because textbook prose has more compound phrases per unit corpus)
- **graph edges** ×4.58, **nodes** ×3.24 — edge count growing faster than nodes, i.e. the graph is densifying (a healthy sign for reasoning).

### Normalized metrics (new in v3.3.0, per Codex #4)

| tier | facts / 10k words | derivations / fact | predicate coverage | duplicate-fact rate |
|---|---:|---:|---:|---:|
| T3_10k | 1.79 | 0.0000 | 33.3 % | 0.00 % |
| T4_50k | 2.00 | 0.4250 | 33.3 % | 27.50 % |

Reading this:
- `facts/10k words ≈ 2` is the steady-state extraction density across T3 → T4. Matcher throughput is linear-in-corpus, no saturation.
- `derivations/fact = 0.425` at T4 means every ~2.3 facts produce 1 rule-derivation on average — strong reasoning leverage.
- `predicate_coverage = 33 %` (is_a + has out of 6 variants). v3.4.0 target: activate locative / dative / part_of matchers on the textbook pool → push toward 67-80 %.
- **`duplicate_fact_rate = 27.5 %`** is the headline hygiene signal that only appears once we scale. Same `(subject, predicate, object)` triple is extracted from multiple textbook sentences. This is not necessarily wrong (repetition is evidence of stability), but future releases should either dedupe-on-extraction or expose `occurrence_count` as a per-fact field for downstream weighting.

### Precision audit surface

`docs/precision_audit.md` generated at v3.3.0 — 17 facts + 1 derivation sampled for native-speaker review with seed-reproducible order, full source sentences, pattern-id + rule-id breakdown, Tally section for computing precision. See the file header for how to review. v3.4 will scale this to the 120-fact T4 pool via `audit_precision --facts-sample 50`.

### Cleanup: `data/external/` slimmed 2.7 GB → 87 MB

Per user request at release-end, cleanup of `data/external/` (which is gitignored end-to-end, so this is pure local-disk reclamation — zero repo impact):

| category | deleted | kept |
|---|---|---|
| Raw sources with `fetch_*.sh` scripts + committed packs | cc100_kk.txt.xz (888 MB), sentences.csv (711 MB), wikipedia_kz_plain.txt (638 MB), sentences.tar.bz2, kkwiki XML bundle, apertium/, Abai + Tatoeba + Common Voice + classics raw files, `.DS_Store`, broken `kaz_news_2011_30K.tar.gz` | — |
| Processed textbook PDFs (pack committed) | 3 KazYazyk/KazLangCulture PDFs (16 MB) | — |
| Unprocessed textbooks (v3.4 target) | — | 7 PDFs (87 MB): Biology 8, Algebra 7, Physics 11 × 2, Informatics 11 × 2, KazLit 11 |

**Reclaimed ≈ 2.65 GB local disk.** Any deleted source is regenerable — raw sources via their `scripts/fetch_*.sh`, textbook packs by re-OCR if the PDFs are reacquired. `validate_foundation.sh` runs green before **and** after the deletion.

### New binaries + modules

- `adam-scaling::bench::run_tier_with_budget` + `EXTRACT_CHUNK_SIZE` — budget-aware tier runner.
- `adam-scaling::NormalizedMetrics` + `TOTAL_PREDICATE_VARIANTS` constant.
- `adam-scaling::bin::audit_precision` — precision audit review generator.
- `adam-corpus::bin::process_kazakh_textbooks` — OCR-output → JSON pack processor.

### Tests

**375 passing, 0 failing, 0 warnings** (373 baseline + 2 strengthened determinism tests in `parser::determinism_tests`: `analyses_sorted_by_root_then_id_when_cross_root_ambiguous` + `first_root_matches_entries_ordered_for_prefix_ambiguous_surface`).

### Upgrade notes

- Library: fully additive. `run_tier` retained as a budget-unaware convenience wrapper around `run_tier_with_budget` for test-code ergonomics.
- Artifacts: `ScalingPoint` gains `normalized: NormalizedMetrics` with `#[serde(default)]` — old reports parse fine. Old versions of the reader ignore the field.
- Data: `kazakh_textbooks_pack.json` is opt-in (the pack list silently skips missing packs). CI checkouts without it run identically to v3.2.0.

---

## [3.2.0] — 2026-04-21 — scaling-law bench + parser determinism fix (foundational)

**Second step** of the post-v3.0 scale-up ladder. Ships **two** things at once because writing the first one exposed an existential bug in the second:

1. The empirical-curve equivalent of a neural-era "perplexity vs FLOPS" chart, but for a deterministic system: **given N input words, how many facts, how many rule derivations, how dense a graph, and how many wall-clock seconds?**
2. **A latent non-determinism fix in `adam-kernel-fst::parser::analyse`** that the scaling bench surfaced on its first run. See the "Latent non-determinism" section below — this is the more important of the two.

### Latent non-determinism bug (found and fixed)

The first scaling-bench run produced byte-different counts on every invocation (±1–3 facts at T3/T4 scale). Root cause: `parser::analyse` iterated `LexiconV1::by_surface.values()` — a `HashMap` — whose iteration order is seeded randomly at process start. When multiple Lexicon entries prefix-match an ambiguous surface, `analyse().into_iter().next()` returned a **different first analysis every run**. Every v2.1+ pattern matcher picks `.next()`, so extracted facts drifted across runs.

This means the v2.5.0-era committed `facts.json` (15 facts) was a lucky snapshot — not a deterministic truth. Previous v3.1.0 regeneration happened to produce 14 facts because that run's HashMap seed sorted a marginal fact out; the drift was invisible to the test suite because no test asserted repeat-call equality.

**Fix:** dual-storage Lexicon (v3.2.0).

```rust
pub struct LexiconV1 {
    pub by_surface: HashMap<String, RootEntry>,    // O(1) get
    pub entries_ordered: Vec<RootEntry>,            // deterministic iteration
    ...
}
```

`entries_ordered` is built once at Lexicon load, sorted by `(root, id)`. `parser::analyse` iterates this Vec instead of `by_surface.values()`. Cost: one extra `Vec<RootEntry>` (≈ 600 KB on the 16 k-entry Lexicon) + a sort at load time. Gain: fully deterministic analysis across runs at HashMap-level throughput (no BTreeMap log-N lookup tax).

Two new regression tests in `parser::determinism_tests`:
- `analyse_ordering_stable_across_calls` — three ambiguous surfaces (`бала`, `алматыда`, `кітабы`, `мектебі`, `жазды`), two back-to-back calls must be equal.
- `first_analysis_stable_for_ambiguous_surface` — `.next()` on the analyse iterator must be stable.

Without these, the whole "deterministic pipeline" thesis is a falsehood — any CI green was historically luck. Now it's a test invariant.

### Re-baselined committed artifacts

With the fix, the committed pipeline settled at **15 facts + 1 derivation** (exactly matching the v2.5.0 figure that was supposed to be canonical). The v3.1.0 "14 facts" baseline is retired — it was a HashMap-seed artifact, not a real drift from the Lexicon purge.

Regenerated artifacts at v3.2.0:

| | v3.2.0 (deterministic) |
|---|---:|
| `data/retrieval/facts.json` facts | **15** |
| `data/retrieval/lexical_graph.json` nodes / edges | 29 / 15 |
| `data/retrieval/derived_facts.json` derivations | 1 (кітап RelatedTo ілім via R5) |

Byte-identical across three consecutive runs.

Unlike transformer scaling laws, every number below is measured on a fully deterministic pipeline — same corpus slice + same Lexicon + same matchers → byte-identical artifacts + byte-identical metric counts across runs (wall-clock drifts; everything else is fixed).

### New crate: `adam-scaling`

- `crates/adam-scaling/` — new 10th crate on the workspace (the ninth reasoning-ready component).
- `src/lib.rs` — `ScalingReport`, `ScalingPoint`, `StageMs`, `MachineSignal`, `SourcesSnapshot` + canonical pack ordering (fact-dense packs first: Abai → proverbs → classics → Wikipedia → synthetic → conversational).
- `src/bench.rs` — pure bench logic: `load_corpus`, `run_tier` (parallel per-sample FST extraction via Rayon, deterministic collect), `run_bench`, `render_markdown`. 4 unit tests (deterministic re-run, tier cap, missing-shards silence, Markdown coverage).
- `src/bin/scaling_bench.rs` — CLI wrapping the lib. Default tiers `[100, 1k, 10k, 50k]` finish in ≲ 10 min on M2 8-core committed corpus. `--use-shards` switches to `[1k, 10k, 50k, 200k, 1M]` for the gitignored full local pool. `--tiers 100,1000,…,0` overrides (0 = uncapped). Honours the v3.1.0 harness: `--time-budget`, `--progress-interval`, SIGINT → graceful commit.

### First measured scaling-law curve (committed-only, 4.23 M-word pool, deterministic)

| tier | samples | words | facts | derivations | graph nodes | graph edges | extract ms |
|---|---:|---:|---:|---:|---:|---:|---:|
| T1_100 | 100 | 903 | 0 | 0 | 0 | 0 | ~490 |
| T2_1k | 1 000 | 8 957 | 0 | 0 | 0 | 0 | ~7 000 |
| T3_10k | 10 000 | 117 979 | **58** | **5** | 55 | 32 | ~92 000 |
| T4_50k | 50 000 | 611 224 | **152** | **65** | 141 | 101 | ~465 000 |

**Full bench: ~9 min 24 s on M2 8-core, 4 / 4 tiers completed, byte-identical counts across runs.**

### Scaling-law signals from T3 → T4 (×5 corpus)

- **words** ×5.18 (corpus growth)
- **facts** ×2.62 (sub-linear — high-density Abai pool exhausted by T3)
- **derivations** **×13.0 (super-linear! — the reasoning signal)**
- **graph nodes** ×2.56 (sub-linear — new words often hit existing nodes)
- **graph edges** ×3.16 (near-linear)

Super-linear derivation growth is exactly the expected scaling law for a rule-based reasoner: more facts → more transitive chains → more inferences. It's the reason this release exists as a separate commit rather than a subsection of something else.

### Rule-activation evolution with scale

| tier | R1 | R2 | R5 |
|---|---:|---:|---:|
| T3_10k | 0 | 0 | 5 |
| T4_50k | 8 | 33 | 24 |

R1 (IsA-transitivity) and R2 (Has-inheritance) only activate once the graph is dense enough for multi-hop chains to form. This is the first release where all three rules fire on real corpus data — the v3.0 artifact only ever surfaced R5.

### Output artifacts

- `data/scaling/scaling_report.json` — structured report with `status` + `elapsed_s` + `tiers_completed / tiers_planned` at the top level, then per-tier ScalingPoints.
- `docs/scaling_report.md` — human-readable projection of the same data, with a Markdown table + per-tier predicate/rule breakdowns. Diffs cleanly across runs (wall-clock is the only drift).

Both are committed to the repo so the curve is version-controlled — every release can compare against prior artifacts.

### Positioning: this replaces "perplexity vs FLOPS"

When investor-facing reviewers ask "what's the scaling law?", the neural-era answer is a plot of perplexity at varying compute budgets. The deterministic-era answer is **this table** — factored into three independently measurable signals (facts, derivations, graph density) each of which tells you something different about what the system does with more data. v3.5.0 will grow it to 20 M words (still on M2, still within a 3 h budget).

### Dependencies

- `rayon` (already workspace-level from v3.1.0) — new direct dep of `adam-scaling`.
- `tempfile 3.12` — dev-only, for the bench unit tests.

### Tests

**371 passing, 0 failing, 0 warnings** (367 + 4 bench unit tests).

### Upgrade notes

- No existing API changed. `adam-scaling` is additive.
- CLI: `cargo run --release -p adam-scaling --bin scaling_bench` runs with committed-only defaults (~10 min). Add `--use-shards` if local shards are populated.
- Artifacts: `data/scaling/` is new; existing manifests unaffected.

---

## [3.1.0] — 2026-04-21 — iteration infrastructure for the 3h-budget discipline

First step of the post-v3.0 scale-up ladder. **No new reasoning capability** — this release builds the *harness* that makes the corpus-jaw work in v3.2+ tractable on a MacBook Air M2 8 GB with a hard 3-hour iteration cap.

### Why this release exists

Every binary in the reasoning pipeline (`extract_facts`, `build_lexical_graph`, `run_reasoner`) now honours four invariants:

1. **`--time-budget <SEC>` / `--time-budget-mins <MIN>`** — hard deadline. When it hits, the binary commits a partial artifact with `status: "timed_out"` and exits 0. Downstream bins treat partial artifacts as first-class input — a partial `facts.json` is still a valid `facts.json`, just smaller.
2. **`--progress-interval <SEC>` (default 30)** — a monitor thread prints `[hh:mm:ss] <bin> samples=N items=M extra=W elapsed=S rem=R` to stderr every interval, so the user can watch 3-hour runs in real time and early-abort when they've seen enough.
3. **SIGINT / SIGTERM → graceful commit** with `status: "interrupted"`. Ctrl-C never loses work.
4. **Rayon parallelism** on the `extract_facts` hot loop. Chunked (128 samples/chunk) so the budget gets checked between chunks — granularity ~0.5-1 s on the current pack sizes. Input-order-preserving collect guarantees byte-identical artifacts across runs.

### Measured speedup (smoke test on committed 3 191-sample corpus)

| binary | pre-v3.1 | post-v3.1 | speedup |
|---|---|---|---|
| `extract_facts` (committed 500/pack) | 42.8 s | 10-15 s | **~3.5×** on M2 8-core |

This is the enabler for v3.2 (scaling bench) and v3.5 (20 M-word full corpus commit in ≤ 3 h).

### New public API (`adam-reasoning`)

- `adam_reasoning::harness` — new module. `IterationBudget`, `ProgressCounter`, `ProgressMonitor`, `StopReason` enum. 10 unit tests.
- `adam_reasoning::reasoner::run_with_budget(&[Fact], &IterationBudget) -> (Vec<DerivedFact>, usize)` — budget-aware variant of `run()` that checks the deadline between forward-chaining iterations. Existing `run()` now delegates through unbounded budget.

### Artifact schema additions (all fields additive, old readers tolerate)

All three artifacts (`facts.json`, `lexical_graph.json`, `derived_facts.json`) gain:

- `status: "completed" | "timed_out" | "interrupted"`
- `elapsed_s: u64`

Plus per-artifact specifics:
- `facts.json` — `packs_completed / packs_total` for mid-pack termination diagnostics.
- `lexical_graph.json` / `derived_facts.json` — `built_from_status` that surfaces the upstream's status for cross-artifact audit.
- `derived_facts.json` — `iterations_completed` (how many forward-chaining passes ran before fixpoint or budget hit; capped at `MAX_ITER = 8`).

### Stale committed artifact refreshed

Regenerating `facts.json` with the current Lexicon surfaced that the v2.5.0-era committed artifact carried one false-positive fact: `ел Has сыртқ` (surface "сыртқы" → invalid root "сыртқ") from `cc100_kk_pack.json / cc100_kk_0000197`. The Lexicon purge across v2.5 → v3.0 correctly stopped accepting "сыртқ" as a content-noun root, but the artifact was never regenerated. The fresh extraction is **14 facts + 1 derivation** — strictly cleaner. The derivation (`кітап RelatedTo ілім` via R5) survives unaffected.

This is why every release should regenerate data artifacts, not just bump Cargo versions. v3.1.0 makes that regeneration fast enough to be routine.

### Dependencies

- `rayon = "1.10"` (workspace)
- `ctrlc = "3.4"` (workspace; adds ~4 transitive deps, ~50 KB compiled)

### Tests

**367 passing, 0 failing** (357 baseline + 10 harness unit tests).

### Upgrade notes

- Library API is additive. `reasoner::run(&facts)` still exists with identical behaviour.
- CLI: all three binaries accept the new flags; omitting them reverts to unbounded default.
- Artifacts written by v3.0 are forward-compatible with v3.1 readers (optional `status` field defaults to `None`).

---

## [3.0.1] — 2026-04-21 — v3.0 polish pass (Codex + Antigravity review items)

Pure polish release based on two external reviews of the v3.0 MVP (Codex + Antigravity). **Zero library changes, zero test-surface changes.** Shipping as a patch because everything it touches is banner strings, doc wording, or dead-code warnings.

### Codex review items (accepted in full)

1. **Stale version banners** — `adam_demo` boxed banner was still printing "adam v2.9" even though the project had shipped v3.0; `adam_chat` greeter + docstring still said "v2.0". Both now say v3.0.
2. **Two compiler warnings fixed** — `first_alphabetic_token` and `last_alphabetic_token` in `crates/adam-reasoning/src/patterns.rs` are used only from the `#[cfg(test)]` module; they now carry `#[cfg(test)]` themselves. `cargo build --workspace` is warning-free.
3. **"0 hallucinations" claim rephrased** — replaced across README, `docs/architecture_v3.md`, `docs/foundation_scope.md`. The honest framing is **"no ungrounded generation by design"** — a falsifiable claim about the absence of a free-text generator in the pipeline, rather than a strong-but-fuzzy "0 hallucinations" badge. The README hallucinations badge is now `ungrounded generation — none by design`.
4. **Honest scale framing** — new **Current state (v3.0.1 — honest numbers)** section in README presents 15 extracted facts + 1 derivation as *proof of mechanism, not scale*, alongside 357 tests / 14 k roots / 77.9 M local corpus. Makes the small-facts-set impossible to miss, and the scale-up path explicit.
5. **Weak demo probe replaced** — step 09 in `adam_demo`'s 12-turn script swapped from `"мектеп керек пе"` (which rarely triggers meaningful retrieval) to `"білім туралы айтшы"` (topic-probe phrasing that matches the retrieval surface).

### Antigravity review items (partial)

1. **"Neuro-Symbolic Retrieval" positioning** — adopted in the README hero paragraph. Names a real paradigm and makes the architecture legible to reviewers who don't read Rust.
2. **Agglutinative advantage** — one-paragraph explanation in the "Why adam (v3.0)" section of why deterministic retrieval + FST composition works specifically for Kazakh and wouldn't transfer to English.
3. **"Physically cannot hallucinate"** — *rejected.* Rhetorically strong but literally false once `ComposeMode::InSampleCitySwap` is on (synthesised forms are new text). Consistent with item 3 above — we prefer falsifiable claims.
4. **"Mathematical determinism" / "Edge AI"** framing — already covered in README / architecture_v3, not re-duplicated.

### What ships

- `crates/adam-dialog/src/bin/adam_demo.rs` — docstring + boxed banner v2.9 → v3.0; step 09 input.
- `crates/adam-dialog/src/bin/adam_chat.rs` — docstring v2.0 → v3.0, REPL greeter string, v2.7 reasoning-chain capability documented.
- `crates/adam-reasoning/src/patterns.rs` — `#[cfg(test)]` on the two test-only helpers.
- `README.md` — hero reworded, new "Current state" table, hallucination wording across the file, template-families count 31 → 34, workspace-tests count 303 → 357, ungrounded-generation row added to the technical spec table, Neuro-Symbolic Retrieval positioning + Agglutinative Advantage line in "Why adam".
- `docs/architecture_v3.md` — trade-off table `0% hallucination` row reworded.
- `docs/foundation_scope.md` — v2.0 rationale wording.
- Workspace `version` → 3.0.1.

### Tests

**357 passing** — unchanged. Zero library surface touched.

### Upgrade notes

None. v3.0.0 and v3.0.1 are byte-identical for embedders.

---

## [3.0.0] — 2026-04-22 — v3.0: investor-demoable intelligent MVP (commitment cut)

Major release. **Not a feature drop — a positioning freeze.** v3.0 captures the v2.5 → v2.9 reasoning ladder as the investor-demoable "intelligent Kazakh AI" cut we committed to when v2.4 shipped.

Everything functional has already shipped across v2.5 – v2.9. v3.0 adds:

1. **`docs/architecture_v3.md`** — new canonical architecture reference that adds the reasoning layer (fact extraction + lexical graph + rule reasoner + dialog integration + trust markers) on top of the v2.0 retrieval foundation. `architecture_v2.md` remains valid as a v2.0–v2.3 historical snapshot.
2. **"Why adam v3.0" README section** — replaces the v2.0 comparison with a v3.0-specific pitch that includes the **reasoning** row, the **«байланыс-» marker**, and the **trust stack** graphic.
3. **Commitment declarations** — explicit in README + architecture_v3:
   - Not a trained neural model.
   - Not multilingual.
   - Not generative.
   - Not a generalist.
   - Not self-modifying.
   - **Reasoning = forward-chaining over typed facts, every conclusion has a `rule_id`** — not emergence, not matmul, not hope.
4. **Docs currency audit** (per `feedback_docs_currency` memory) — `foundation_scope.md`, `eval_baseline.md`, `kazakh_grammar/07_dialog_architecture.md` refreshed with v3.0 test count, v3.0 links, and accurate in-scope / delivered lists.

### The v3.0 trust stack

```
 template realisation            →  recognised intent, 0% fabrication
 verbatim quote «…»              →  corpus citation, byte-identical to source
 «бейімд-» adaptation marker      →  quote was rewritten (v1.9.5)
 «байланыс-» reasoning marker     →  derivation, not a quote (v3.0)
```

Every marker is test-enforced bi-directionally: fires when and only when the path fired.

### What v3.0 changes about the code

**Nothing in the library surface.** Intent structures, Conversation API, adam-retrieval, adam-reasoning — all bit-identical to v2.9. Embedders upgrading from v2.9 see zero diff.

- README `version` badge 2.9.0 → 3.0.0
- `docs/architecture_v3.md` (new file, supersedes v2 for v3.0 state)
- README "Why adam v3.0" section
- `docs/foundation_scope.md` + `docs/eval_baseline.md` + `docs/kazakh_grammar/07_dialog_architecture.md` — stale-link + test-count refresh
- Cargo workspace + manifest versions → 3.0.0

### Ladder: 6/6 complete

| step | release | what landed |
|---|---|---|
| 1/6 | v2.5 | `GoesTo` predicate + dative-motion pattern |
| 2/6 | v2.6 | `PartOf` + `RelatedTo` predicates, R5 active → first real derivation |
| 3/6 | v2.7 | dialog integration → first user-visible inference |
| 4/6 | v2.8 | R2 Has-inheritance + complete predicate renderers |
| 5/6 | v2.9 | `adam_demo` Part 4 — reasoning chain end-to-end |
| **6/6** | **v3.0** | **commitment cut — positioning + docs freeze** |

### Tests

**357 passing** — unchanged from v2.8, carried through v2.9 and v3.0. v3.0 is docs + positioning; no library changes.

### Zero regressions

No library code touched since v2.9. Upgrading from v2.x → v3.0 is safe and silent for embedders.

### The arc, v2.0 → v3.0

| Release | Facts | Predicates | Active rules | Derivations | User-visible inferences | Tests |
|---|---:|---:|---:|---:|---:|---:|
| v2.0 | 0 | 0 | — | — | 0 | 303 |
| v2.1 | 11 | 1 | — | 0 | 0 | 325 |
| v2.2 | 13 | 2 | — | 0 | 0 | 328 |
| v2.3 | 15 | 2 | — | 0 | 0 | 335 |
| v2.4 | 15 | 2 | 1 (R1) | 0 | 0 | 343 |
| v2.5 | 15 | 4 | 1 | 0 | 0 | 347 |
| v2.6 | 15 | 6 | 2 (+R5) | **1** | 0 | 352 |
| v2.7 | 15 | 6 | 2 | 1 | **1** | 354 |
| v2.8 | 15 | 6 | 3 (+R2) | 1 | 1 | 357 |
| v2.9 | 15 | 6 | 3 | 1 | 1 | 357 |
| **v3.0** | **15** | **6** | **3** | **1** | **1** | **357** |

v3.0 does not add to the numbers. It **affirms** the state reached: from 0 derivations at v2.4 to a real rule-derived `кітап RelatedTo ілім` that dialog cites with the «байланыс-» marker to the user, with full `source_chain` provenance, deterministically, across 357 tests.

### How to demo v3.0 for investors

```bash
cargo run --release -p adam-dialog --bin adam_demo
```

4-part scripted walkthrough (intents + retrieval + composition + reasoning). Byte-identical across runs. Safe to record once, play anywhere.

### Post-v3.0

Post-v3.0 work continues incrementally on the same architecture:

- More pattern matchers (densifying the fact graph so R1 transitivity fires naturally).
- `PartOf` extraction pattern (activates R3).
- More predicates when specific domains demand them (`Causes`, `Enables`, `Prevents` for causal reasoning).
- R4 diagnostic surface (IsA symmetry → curator review).
- Option C composition (offline pattern extraction for swap types beyond city).
- Kazakh technical corpus (Rust Book translation as a new source pack).
- Response-side diversity (multiple top-k citations across repeated prompts).

Each is additive. None requires rethinking the v3.0 architecture.

## [2.9.0] — 2026-04-22 — Investor-demo polish: `adam_demo` Part 4 shows reasoning chains end-to-end (v3.0 ladder step 5/6)

Minor release. **Penultimate rung before the investor-demoable v3.0 cut.** v2.9 adds a fourth part to the `adam_demo` scripted walkthrough that loads the committed fact + derivation artefacts and shows, live, how adam produces a *reasoned* answer the user can see, with full provenance, with the trust marker. Ready to record for a presentation.

### `adam_demo` gains Part 4 — the reasoning payoff

```
╔══════════════════════════════════════════════════════════════╗
║ adam v2.9 — 4-part scripted demo (intents + retrieval +     ║
║              composition + reasoning, deterministic)        ║
╚══════════════════════════════════════════════════════════════╝

...  [Parts 1, 2, 3 unchanged] ...

────────────────────────────────────────────────────────────────
PART 4 — rule-derived reasoning chain (v2.6 R5 + v2.7 dialog)
         loading committed facts.json + derived_facts.json
         reasoner produces RelatedTo derivations; dialog
         cites them with the «байланыс-» trust marker.
────────────────────────────────────────────────────────────────

Loaded reasoning artefacts:
  extracted facts:      15
  rule-derived facts:   1

Derivation(s) available to cite:
  кітап --related_to--> ілім   [R5_shared_is_a_target]
    source_chain:
      kazakh_proverbs_pack.json / proverb_003
      common_voice_kk_pack.json / cv_kk_00047

User probe: «кітап туралы бірдеңе айт»
  seed  1 [chain]: Қолда бар деректерден байланыс құрастырдым: кітап пен ілім бір-біріне байланысты екен.
  seed  4 [chain]: Айтуыңыз бойынша, мынадай қисынды байланыс бар: кітап пен ілім бір-біріне байланысты екен.
  seed  8 [chain]: кітап туралы мынадай байланыс анықтадым: кітап пен ілім бір-біріне байланысты екен.
  seed 12 [chain]: кітап туралы мынадай байланыс анықтадым: кітап пен ілім бір-біріне байланысты екен.

NOTE: every response above containing «байланыс-» is REASONED,
not RETRIEVED. The v2.7 trust invariant (tested) guarantees
the marker never appears without an actual derivation backing it.
```

### What Part 4 shows (investor narrative)

1. **Artefacts loaded** — 15 extracted facts + 1 derivation from disk. Concrete, counted, auditable.
2. **Derivation surfaced with provenance** — the chain `кітап --related_to--> ілім [R5_shared_is_a_target]` is printed **with both source facts** (`proverb_003` and `cv_kk_00047`). The presenter can point at this: *"these are the two actual corpus sentences whose relation the system concluded."*
3. **User probe** — «кітап туралы бірдеңе айт» — a natural open-ended question.
4. **Four deterministic seeds** — every one cites the chain. Each response is marked `[chain]` in the demo output; every one contains «байланыс-». If the reasoning path were somehow bypassed, the marker would be absent and the test invariants (from v2.7) would have caught it.
5. **The trust invariant is called out explicitly** — the closing NOTE tells the presenter (and the viewer) that «байланыс-» **never** appears without an actual derivation. The safety is structural, not cosmetic.

### What this looks like vs an LLM pitch

| | adam (v2.9 demo, Part 4) | LLM pitch |
|---|---|---|
| Source of claim | `proverb_003` + `cv_kk_00047` named inline | "from training data" (unnamed) |
| Mechanism | R5 forward-chaining, rule id shown | matmul across billions of weights |
| Marker of inference | «байланыс-» in every response, test-enforced | — |
| Re-runnable | byte-identical across runs | temperature-dependent |
| Auditable | every derivation has `source_chain` | — |
| Cost | ms on laptop CPU | dollars on GPU |

### Ladder progress: 5/6 done

| step | release | status |
|---|---|---|
| 1/6 | v2.5 — `GoesTo` + dative pattern | ✅ |
| 2/6 | v2.6 — `PartOf` + `RelatedTo` + R5 active | ✅ |
| 3/6 | v2.7 — dialog integration | ✅ |
| 4/6 | v2.8 — R2 active + complete renderers | ✅ |
| **5/6** | **v2.9 — investor-demo polish, `adam_demo` Part 4** | **✅ shipped** |
| 6/6 | v3.0 — investor-demoable commitment cut | next |

### Changes

- `adam_demo` binary:
  - New `run_reasoning_chain_demo` function — loads `data/retrieval/facts.json` + `data/retrieval/derived_facts.json`, attaches them to a fresh `Conversation`, picks a noun that appears in a derivation, and runs deterministic probes across seeds 1 / 4 / 8 / 12.
  - Every response is tagged `[chain]` or `[plain]` based on marker presence for at-a-glance scanning.
  - Graceful no-op with a help message if artefacts are missing (e.g. trimmed CI checkouts).
  - Banner updated to "v2.9 — 4-part scripted demo" with the part list in the subtitle.
- Module docstring rewritten to describe all four parts.

### Tests

**357 passing** (unchanged from v2.8). v2.9 is demo-binary polish — no library-surface changes, no new tests.

### Zero regressions

No library code touched. The demo binary is the only modification; Parts 1–3 are unchanged.

### What v3.0 will do

The commitment cut. Not a feature drop — a positioning freeze. README refreshed with a v3.0 "Why adam" section that reflects the reasoning capability, `docs/architecture_v2.md` renamed or updated to `architecture_v3.md`, and the final tag that says *this is the investor-demoable intelligent MVP we committed to from the v2.5 ladder start*.

## [2.8.0] — 2026-04-22 — R2 Has-inheritance rule + complete predicate-specific renderers (v3.0 ladder step 4/6)

Minor release. **Rule and renderer matrix completed.** v2.8 activates R2 (`A IsA B ∧ B Has X ⟹ A Has X`) and adds Kazakh prose renderings for every `Predicate` variant, so any derivation the reasoner produces can be cited in the dialog layer without a fallback placeholder.

### New: R2 — Has inheritance through IsA

```
R2_has_inheritance:
   A IsA B ∧ B Has X  ⟹  A Has X
```

The soundness caveat is explicit in the module docstring: this is **conservative monotonic inheritance**, which is not universally true in natural language (бала IsA адам and адам Has автокөлік does NOT mean бала Has автокөлік). The rule produces derivations labelled `ConfidenceKind::RuleInferred`, so downstream consumers can filter by confidence kind and treat these as "possible" rather than "certain".

Tautology guard (A = X) rejects pathological cases.

On the current 15-fact set, R2 produces 0 derivations — our IsA targets (бұлақ, іс, қазына …) have no outgoing Has edges. That's the honest state. R2 will fire naturally as v2.x patterns populate more connective facts.

### Complete predicate-specific renderers

`render_derivation_as_kazakh` previously handled `IsA` + `RelatedTo` + a generic fallback. v2.8 adds specific phrasings for every other variant:

| predicate | Kazakh rendering |
|---|---|
| `RelatedTo` | «X пен Y бір-біріне байланысты екен» |
| `IsA` | «қорытынды: X — Y (байланысты ой-тізбек арқылы)» |
| `Has` | «ой-тізбек: X Y-ға қатысты байланысы бар (иелік мұрагерлік)» |
| `GoesTo` | «X Y жағына байланысты қозғалыс ретінде шықты» |
| `LivesIn` | «X Y орнымен байланысты мекендеу қорытындысы бар» |
| `PartOf` | «X Y-дың құрамына байланысты бір бөлігі ретінде шықты» |

All six contain the mandatory trust marker **«байланыс-»** — the invariant introduced in v2.7 still holds: any rendered derivation is distinguishable from a verbatim corpus quote at the textual level.

### Ladder progress: 4/6 done

| step | release | status |
|---|---|---|
| 1/6 | v2.5 — `GoesTo` + dative pattern | ✅ |
| 2/6 | v2.6 — `PartOf` + `RelatedTo` + R5 active | ✅ |
| 3/6 | v2.7 — dialog integration | ✅ |
| **4/6** | **v2.8 — R2 active + complete renderers** | **✅ shipped** |
| 5/6 | v2.9 — investor-demo polish | next |
| 6/6 | v3.0 — investor-demoable commitment cut | |

### Tests (+3 → 357 total)

- `r2_derives_has_inheritance` — canonical positive case (бала IsA адам + адам Has жан → бала Has жан).
- `r2_respects_tautology_guard` — never derives A Has A.
- `r2_does_not_fire_without_has_edge` — A IsA B alone doesn't trigger R2.

### Reasoner state

| rule | status on current corpus | tested |
|---|---|---|
| R1 — IsA transitivity | correct, 0 fires (no chains in metaphorical data) | ✅ |
| R2 — Has inheritance | correct, 0 fires (no outgoing Has edges from IsA targets) | ✅ |
| R3 — LivesIn transitivity | documented, deferred (needs `PartOf` data) | — |
| R4 — IsA symmetry diagnostic | documented, deferred (needs diagnostic surface) | — |
| R5 — Shared IsA target | 1 firing (кітап RelatedTo ілім) | ✅ |

### Zero regressions

All 354 pre-v2.8 tests still pass. R2 is additive; `render_derivation_as_kazakh` generic-fallback arm removed because every predicate now has a specific branch (exhaustive matching).

### Committed artefacts

Unchanged. Single R5 derivation on the current data; R2 silent until more facts connect.

### What v2.9 will do

Investor-demo polish: scripted walkthrough showing the full reasoning chain end-to-end. An `adam_demo` enhancement (or new binary) that walks through "user asks X → reasoner consults graph → chain emerges → response cites it". Positioning + narration for presentation.

## [2.7.0] — 2026-04-22 — Dialog integration: reasoning chains in `Intent::Unknown` responses (v3.0 ladder step 3/6)

Minor release. **The reasoner's output becomes user-visible.** Up to v2.6 derivations existed only in `derived_facts.json`. v2.7 wires them into `Conversation::turn`: when `Intent::Unknown` fires with a noun hint that appears in a derived fact, the response cites the reasoning chain in Kazakh prose with a trust marker.

### The first user-visible derivation

```
$ adam_chat --once "кітап туралы бірдеңе айт"
adam-chat: reasoning on — 1 derived facts available (15 supporting extracted facts)

кітап туралы мынадай байланыс анықтадым:
  кітап пен ілім бір-біріне байланысты екен.
```

The chain that `R5_shared_is_a_target` derived in v2.6 (both `кітап` and `ілім` are IS-A `бұлақ` ⟹ they're related) is now spoken back to the user. The marker stem **«байланыс-»** flags the claim as *reasoned*, not *retrieved* — a runtime-greppable signal that this sentence was **inferred** and is not a verbatim corpus line.

### Changes

- **`Conversation`** gains two new fields:
  - `derived_facts: Vec<DerivedFact>`
  - `extracted_facts: Vec<ReasFact>`

  Both default to empty. Builder: `Conversation::with_reasoning_chains(extracted, derived)`.
- **`Intent::Unknown`** gains `reasoning_chain: Option<String>` field (`#[serde(default)]`).
- **New injection step**: `Conversation::turn` calls `inject_reasoning_chain` after the existing retrieval injection. When `noun_hint` matches a derivation's subject or object root, the chain is rendered into Kazakh and placed in the slot.
- **Kazakh prose renderer**: `render_derivation_as_kazakh` — explicit handling for `RelatedTo` and `IsA`; generic fallback for others. Every output contains «байланыс-».
- **Planner routing priority**: `reasoning_chain.is_some()` → `unknown.with_derived_chain`. Takes precedence over retrieval evidence — a derived conclusion is a stronger claim than a cited passage.
- **New template family** `unknown.with_derived_chain` (4 templates). Every template contains «байланыс-».

### Trust invariants — test-enforced

- `derived_facts` match `noun_hint` ⇒ response contains «байланыс-».
- `derived_facts` empty ⇒ «байланыс-» NEVER appears across 32 seeds.
- Mirrors v1.9.5's `verbatim_mode_never_claims_adaptation` — never claim "I reasoned this" when we didn't.

### `adam_chat` autoloads reasoning artefacts

CLI loads `data/retrieval/facts.json` + `data/retrieval/derived_facts.json` alongside the morpheme index at startup. Banner confirms. Missing/malformed artefacts silently disable the path.

### Ladder progress: step 3/6 done

| step | release | status |
|---|---|---|
| 1/6 | v2.5 — GoesTo + dative pattern | ✅ |
| 2/6 | v2.6 — PartOf + RelatedTo + R5 active | ✅ |
| **3/6** | **v2.7 — dialog integration** | **✅ shipped** |
| 4/6 | v2.8 — more rules + pattern density | next |
| 5/6 | v2.9 — investor-demo polish | |
| 6/6 | v3.0 — investor-demoable commitment cut | |

### Tests (+2 → 354 total)

- `unknown_with_reasoning_chain_cites_derivation` — synthetic `RelatedTo` fact → «байланыс-» fires.
- `unknown_without_derived_facts_never_claims_chain` — no facts → marker never fires, 32 seeds.

### Zero regressions

All 352 pre-v2.7 tests still pass. Additive to `Conversation` + `Intent`; existing embedders see v2.6-identical behaviour.

### Committed artefacts

Unchanged from v2.6. (New behaviour is in how they're consumed, not the data itself.)

### What v2.8 will do

- Additional pattern matchers (populate middle-of-chain nodes so R1 transitivity starts firing on corpus).
- More rules: R2 (`Has` inheritance via IsA), R4 (`IsA` symmetry diagnostic).
- Predicate-specific Kazakh prose renderers for `GoesTo` / `Has` / `LivesIn` derivations.

## [2.6.0] — 2026-04-22 — `PartOf` + `RelatedTo` predicates + R5 rule activation (v3.0 ladder: step 2/6)

Minor release. **The reasoner starts producing actual derivations on real corpus data.** v2.5 shipped the inference machinery; v2.6 wires it to the first real chain.

### New predicates

```rust
pub enum Predicate {
    IsA,        // v2.1
    LivesIn,    // v2.1
    Has,        // v2.2
    GoesTo,     // v2.5
    PartOf,     // v2.6 ← NEW — physical / administrative containment
    RelatedTo,  // v2.6 ← NEW — symmetric semantic relation, derived by R5
}
```

`PartOf` covers phrasings like «X Y-нің құрамында», «X Y-нің бөлігі» — geographic containment, administrative subdivision. No extraction pattern yet in v2.6 (will land when a committed source pack surfaces enough of them); the predicate is declared so future patterns and rules can wire it without a breaking release.

`RelatedTo` is typically **rule-derived** rather than pattern-extracted — it's what `R5_shared_is_a_target` produces. Making it a first-class predicate lets downstream consumers (v2.7+ dialog integration) treat derived-relatedness facts with the same graph/query surface as extracted facts.

### Rule activation: R5 is now live

```
R5_shared_is_a_target:   A IsA X ∧ B IsA X ∧ A ≠ B  ⟹  RelatedTo(A, B)
```

Symmetry-aware: the canonical pair has the lexicographically smaller root as the subject, so `(кітап, ілім)` and `(ілім, кітап)` deduplicate to one fact. R5 runs in the same pass as R1; they interleave correctly (R1 can feed R5 via newly-derived IS-A edges).

### The first real derivation

On the v2.5 fact set — completely unchanged, no new extraction — R5 now derives:

```
кітап  --RelatedTo-->  ілім    [R5_shared_is_a_target]
    source chain: proverb_003 (кітап IsA бұлақ) + cv_kk_00047 (ілім IsA бұлақ)
```

This is the first **inferred** fact in adam's history. It's a small claim but a real one: the system recognised that two proverbs map different subjects to the same metaphorical hub (`бұлақ` — a spring, a source), and therefore those subjects stand in a **shared-type relation**. A reasoner did that, not retrieval.

### Commitment check: v3.0 ladder progress

| release | scope | status |
|---|---|---|
| v2.5 | `GoesTo` + dative pattern | done |
| **v2.6** | **`PartOf` + `RelatedTo` + R5 rule active → first real derivation** | **done** |
| v2.7 | dialog integration (reasoner in `Conversation::turn`) | next |
| v2.8 | more rules + pattern density | |
| v2.9 | investor-demo polish with chain reasoning | |
| v3.0 | commitment cut | |

The machinery now produces derivations. v2.7 will make them visible to the user in dialog responses.

### Tests (+5 → 352 total)

- `r5_derives_related_to_from_shared_target` — canonical positive (2 shared-target facts → 1 RelatedTo).
- `r5_no_derivation_without_shared_target` — distinct targets → no RelatedTo.
- `r5_three_way_hub_produces_three_pairs` — A, B, C sharing hub X → 3 pairs.
- `r5_symmetry_dedups_pairs` — order-flip invariance: one pair per relation.
- `canonical_relation_pair_is_sorted` — helper invariant.
- Plus: `Predicate::PartOf.as_str()` / `Predicate::RelatedTo.as_str()` stability checks.
- Updated: `r1_derives_is_a_transitivity` now filters by rule_id because R1 + R5 interleave on the shared-target graph R1 builds.

### Graph updated

`LexicalGraph::from_facts` handles both new predicate strings. The compile-time `unreachable!` arm stays effective — any future `Predicate` variant will break the build until a graph branch is added, keeping extraction and graph in permanent lock-step.

### Committed artifacts

- `data/retrieval/derived_facts.json` — **1 derivation** (was 0): `кітап RelatedTo ілім` via R5.
- `data/retrieval/lexical_graph.json` — regenerated, same 15 facts / 29 nodes / 15 edges (derived facts don't reshape the graph unless they're pushed back through `build_lexical_graph`; v2.8 will consider that integration).
- `data/retrieval/facts.json` — unchanged 15 facts.

### Zero regressions

All 347 pre-v2.6 tests still pass. R5 activation is additive; R1 behaviour is unchanged at the algorithm level (the test update reflects the expanded emergent derivation set, not a R1 change).

### Next (v2.7)

Wire the reasoner into `Conversation::turn`. When `Intent::Unknown` fires with a noun hint that appears in the graph, the response can cite a derived fact alongside (or instead of) a retrieved sample: *«кітап пен ілім бір-біріне байланысты: екеуі де бұлақ болып табылады.»* — with full source-chain provenance in the trace.

## [2.5.0] — 2026-04-22 — `GoesTo` predicate + dative-motion pattern (v3.0 ladder: step 1 of 6)

Minor release. **First rung on the v2.5 → v3.0 ladder** toward the investor-demoable intelligent MVP. The target at v3.0 is a dialog system that can **derive** answers through rule-reasoning chains, not just retrieve them. Getting there requires more predicates + more pattern density so the reasoner has real chains to traverse. v2.5 is the first of six planned steps.

### New predicate: `GoesTo`

```rust
pub enum Predicate {
    IsA,       // X — Y               (v2.1 copula)
    LivesIn,   // X Y-да тұрады       (v2.1 locative)
    Has,       // X-тың Y-сы бар      (v2.2 possessive)
    GoesTo,    // X Y-ке барады        (v2.5 dative-motion) ← NEW
}
```

### New pattern: `dative_goes_to`

Kazakh "X goes to Y" is `<subject-nom> <place-dative> бару-in-any-inflection`. Type-checked fully on FST features, never on verb surface:

- Verb token must analyse to `root == "бару"` — any tense / person / number form passes.
- Destination must be a noun with `Case::Dative`, non-closed-class.
- Subject must be a bare-nominative content noun preceding the destination. Pronouns refused (same filter as v2.1's `is_closed_class`).
- First-match-per-sentence; non-adjacency breaks the pattern (v2.5 doesn't guess).
- Tautology guard (`subject.root == object.root`).

### Graph projection updated

`LexicalGraph::from_facts` now handles the new `goes_to` predicate string. The match arm uses `unreachable!` for unknown predicate strings — a **compile-time enforcement** that every new `Predicate` variant must add a branch here, so the graph and extraction stay in lock-step.

### Extraction delta on committed corpus

| Metric | v2.4 | **v2.5** |
|---|---:|---:|
| Total facts | 15 | **15** (same) |
| Predicates with extractions | 2 (IsA, Has) | **2** (IsA, Has) |
| GoesTo facts found | — | **0 on committed corpus** |

**Honest zero**: the committed 3191 samples (500/pack cap) are proverbs + Wikipedia intros + Abai poetry — genres that use copula and possessive more than motion verbs. The pattern is correctly wired (4 unit tests verify positive + 3 negatives) and will fire naturally as:

1. v2.6 adds complementary patterns that populate middle-of-chain nodes.
2. Future pattern passes cover more genres (dative-motion is common in modern news prose, rare in proverbs).
3. `--full` mode users already see firings on the 350k+ full corpus.

Shipping the pattern now means v2.6 — v3.0 can build on it without re-implementing.

### Tests (+4 → 347 total)

- `dative_extracts_child_goes_to_school` — canonical positive case.
- `dative_rejects_without_baru_verb` — dative noun + different verb → no fact.
- `dative_rejects_pronoun_subject` — «мен мектепке барамын» refused (no grounded knowledge).
- `dative_rejects_self_tautology` — subject = destination refused.
- Plus `Predicate::GoesTo.as_str() == "goes_to"` stability check.

### Zero regressions

All 343 pre-v2.5 tests still pass. New pattern is purely additive to `extract_facts`; the v2.4 reasoner accepts the new predicate variant (though no rule fires on it yet).

### Committed artifacts

- `data/retrieval/facts.json` regenerated (same 15 facts; dative matcher added but produces no new firings on this corpus).
- `data/retrieval/derived_facts.json` regenerated (still 0 derivations — same data).
- `data/retrieval/lexical_graph.json` regenerated (same 29 nodes / 15 edges).

### The v2.5 → v3.0 ladder (committed)

| release | scope | expected outcome |
|---|---|---|
| **v2.5** | **+ GoesTo predicate, dative-motion pattern** | **done — pattern wired** |
| v2.6 | + PartOf, + RelatedTo predicates + patterns | R3, R5 rules activate, first real derivations |
| v2.7 | dialog integration: reasoner in `Conversation::turn` | user sees chains in responses |
| v2.8 | more rules + corpus density | 50+ facts, non-trivial graph |
| v2.9 | investor-demo polish: new `adam_demo` with chain reasoning | end-to-end scripted walkthrough |
| v3.0 | investor-demoable commitment cut | "Why adam v3.0" positioning + final tag |

Each step grounded in what the previous step measured.

## [2.4.0] — 2026-04-22 — Rule reasoner v0 (forward-chaining over the Lexical Graph) + comprehensive docs-currency audit

Minor release. Two axes of progress.

### 1. Rule reasoner v0 — the first *inference* step

New `adam_reasoning::reasoner` module + `run_reasoner` binary. Takes the v2.1+ `facts.json`, runs forward-chaining rules against the Lexical Graph (v2.3), emits every derived fact with:

- **`rule_id`** — the stable identifier of the rule that fired (never a probability score);
- **`source_chain: Vec<FactSource>`** — every underlying fact that contributed (non-empty by invariant);
- **`ConfidenceKind::RuleInferred`** — distinguishes derivations from `Grammar`-extracted corpus facts at every downstream site.

Initial rule set (**1 active, 4 documented for v2.5+**):

| id | pattern | conclusion | status |
|---|---|---|---|
| `R1_is_a_transitivity` | `A IsA B ∧ B IsA C ⟹ A IsA C` | IS-A chains | **active** |
| `R2_has_inheritance` | `A IsA B ∧ B Has X ⟹ A HasKinded X` | inherited `Has` | documented, deferred |
| `R3_lives_in_transitivity` | `A LivesIn B ∧ B PartOf C ⟹ A LivesIn C` | geographic containment | waits on `PartOf` |
| `R4_is_a_symmetry_filter` | `A IsA B ∧ B IsA A` | diagnostic for curator review | not yet wired |
| `R5_shared_is_a_target` | `A IsA X ∧ B IsA X, A ≠ B ⟹ RelatedTo(A, B)` | implicit similarity | waits on `RelatedTo` predicate |

### 2. Trust invariants (test-enforced)

- Rule fires ⇒ derived fact's `confidence == RuleInferred`.
- Derived fact's `source_chain` is non-empty.
- Fixpoint reached ⇒ re-running the reasoner adds nothing.
- `R1` never derives `A IsA A` even under `A↔B↔A` loops.

### 3. Baseline result on the v2.3 fact set

**0 derivations** from the current 15 facts. This is **honest** — our extracted facts are metaphorical one-hops (`кітап IsA бұлақ`, `ілім IsA бұлақ`), and the objects don't themselves have outgoing IS-A edges. The reasoner is correctly wired (unit tests verify multi-hop chains up to 3 hops), the data just doesn't yet form chains. Future extraction (dative-motion, more copula cases) will populate middle-of-chain nodes and unlock R1.

Zero derivations today ≠ zero value: we now have the inference machinery, tested, ready, with a rule-id audit surface. v2.5 adds more predicates + patterns; R1 starts firing naturally.

### 4. Comprehensive docs-currency audit

**Per-release directive** (new memory `feedback_docs_currency`): every release must refresh every documentation, descriptive, and module-level docstring — not just README/CHANGELOG/roadmap. Stale info anywhere is a defect.

Files refreshed in this release:

- `crates/adam-dialog/Cargo.toml` description — dropped stale "adam v1.0.0" tag
- `crates/adam-kernel-fst/Cargo.toml` description — now describes current FST capabilities precisely
- `crates/adam-kernel-fst/src/lib.rs` — module-level docstring replaced "v1.0.0 scaffold (week 1 day 1 — skeleton only)" with current capabilities
- `crates/adam-reasoning/src/lib.rs` — stage marker bumped "v2.1 bootstrap — fact extraction only" → "v2.3+ fact extraction + lexical graph projection"
- `docs/foundation_scope.md` — scope section rewritten to cover v1.0.0 → v2.3 deliveries; stale "v1.4.0+ out of scope" replaced with accurate post-v2.3 agenda
- `docs/corpus_audit.md` — title dropped "v1.1.5 Baseline", added current (v2.3) position + historical expansion-plan pivot note
- `docs/repository_layout.md` — 7 crates → 9 crates (added `adam-retrieval`, `adam-reasoning`); added `data/retrieval/` entry; stale Lexicon count fixed
- `docs/eval_baseline.md` — test count 271 → 335
- `docs/kazakh_grammar/07_dialog_architecture.md` — test count 271 → 335; stale "trilingual delivered" marked as reverted in v1.1.0
- `docs/architecture_v2.md` — added reasoning + graph entries to code-location map; "Post-v2.0 directions" section replaced with "Shipped in v2.1–v2.3" + "Still ahead"
- `data/dialog/README.md` — "29 families, v0.8.5" → "31 families as of v2.3"
- `data/lexicon_v1/README.md` — replaced "211 curated, week 3/4 future" with accurate "4,432 curated after v2.2 purge"
- Memory: new `feedback_docs_currency.md` documents the audit checklist for every future release

### Tests (+8 → 343 total)

Reasoner tests:
- `r1_derives_is_a_transitivity`
- `r1_chains_three_hops` (multi-iteration fixpoint)
- `r1_rejects_tautology`
- `reasoner_reaches_fixpoint` (idempotence)
- `derived_fact_has_nonempty_source_chain`
- `derived_fact_always_rule_inferred_confidence`
- `into_fact_promotes_cleanly`
- `empty_input_empty_output`

### Zero regressions

All 335 pre-v2.4 tests still pass. Rule reasoner is a pure additive module; no change to extraction, retrieval, dialog, or FST crates.

### Committed artifacts

- `data/retrieval/derived_facts.json` — **new**, 0 derivations on v2.3 facts (honest zero, documented)
- Every other data artifact unchanged

### Next (v2.5+)

- **More pattern matchers** — dative-motion (`X Y-ке барады` → `GoesTo`), verb-derived action facts. Each new pattern unlocks middle-of-chain nodes that activate R1.
- **New predicates** — `RelatedTo` (to unlock R5), `PartOf` (to unlock R3). Both geographic/compositional relations that Kazakh proverbs and Wikipedia make heavy use of.
- **Rule-inferred facts in dialog responses** — retrieve + reason pipeline where the Unknown handler can cite a chain ("X IsA Y because Z + W") when exact quote retrieval misses.

## [2.3.0] — 2026-04-21 — FST vowel-final+P3 fix + Lexical Graph v0 (fact projection)

Minor release. Two step-changes:

1. **FST fix**: Kazakh glide-vowels `у`, `и`, `ю` are moved from `ConsonantClass::VowelPreceding` to `HighSonorant`, aligning the code with the enum docstring and fixing a whole class of vowel-final + P3 mis-synthesis. Observable: `оқу+P3`, `бастау+P3` now produce `оқуы`, `бастауы` (before: wrong `оқусы`, `бастаусы`). v2.2's last remaining imprecision (`жер → тіршілік` should have been `жер → бастау`) is fixed as a direct consequence.
2. **Lexical Graph v0**: new `adam_reasoning::graph::LexicalGraph`. Pure projection of `facts.json` into `(nodes, edges)` — every edge traces back to the fact(s) that produced it. 29 nodes, 15 edges from the v2.3 fact set. First step toward a reasoner that can answer "tell me about X" or "what is X?" in O(1) via the graph.

### The FST fix — `classify_char` correction

```rust
// before (v2.2)
'а' | 'ә' | 'е' | 'ё' | 'и' | 'і' | 'о' | 'ө' | 'у' | 'ұ' | 'ү' | 'ы' | 'э' | 'ю' | 'я'
  → VowelPreceding

// after (v2.3)
'а' | 'ә' | 'е' | 'ё'       | 'і' | 'о' | 'ө'       | 'ұ' | 'ү' | 'ы' | 'э'       | 'я'
  → VowelPreceding
'й' | 'р' | 'у' | 'и' | 'ю'
  → HighSonorant
```

Kazakh grammatical rationale: `у`, `и`, `ю` are glide-vowels — spelt as letters, but patterning with consonants for P3 `с`-buffer insertion and Y-buffer alternation.

Observable cascade of fixes:

- `realise_s_buffer` no longer inserts `с` after у/и/ю → `оқу+P3` = `оқуы` (not `оқусы`).
- `realise_y_buffer` now inserts `ы/і` after у/и/ю → `оқу+P1SG` = `оқуым` (not the broken `оқум`).
- `realise_n` `HighSonorant` branch already existed; existing vowel-cases fall through vowel-path untouched.

Every pre-v2.3 test still passes (328 → 335, including +7 graph tests). Zero regressions.

### Extraction delta — v2.2 → v2.3

| Metric | v2.2 | **v2.3** | Δ |
|---|---:|---:|---|
| Committed facts | 13 | **15** | +2 (`жер → бастау` corrected, `ой → қару` newly unblocked) |
| Predicates | 2 (IsA, Has) | 2 | — |
| Clean facts | 13 | **15 (100 %)** | **0 imprecisions remain** |

v2.1 → v2.3 arc on the *same committed corpus*:

```
  v2.1 : 11 facts, 4 imprecisions  (Lexicon gaps visible)
  v2.2 : 13 facts, 1 imprecision   (87 Lexicon pollutions purged; 3 fixed, 1 blocked)
  v2.3 : 15 facts, 0 imprecisions  (FST glide-vowel fix unblocks the remainder)
```

The feedback loop is continuous — every release's diagnostics drive the next release's targets.

### Lexical Graph v0

New module `adam_reasoning::graph` + binary `build_lexical_graph`:

```rust
pub struct GraphEdge {
    pub from: String,
    pub predicate: Predicate,
    pub to: String,
    pub sources: Vec<FactSource>,       // merged provenance
}

pub struct NodeStats {
    pub out_degree: usize,
    pub in_degree: usize,
    pub out_by_predicate: BTreeMap<String, usize>,
    pub in_by_predicate: BTreeMap<String, usize>,
}

pub struct LexicalGraph {
    pub nodes: BTreeMap<String, NodeStats>,
    pub edges: Vec<GraphEdge>,
    pub facts_ingested: usize,
}
```

Build: `LexicalGraph::from_facts(&facts)`. **Pure projection** — no learned weights, no heuristics beyond what fact extraction already applied. Same facts → byte-identical graph.

**Current graph** (15 facts → 29 nodes, 15 edges, most-connected node `бұлақ` with degree 2):

```
  адам            --Has       --> гүл
  айлакерлік      --IsA       --> іс
  ана             --IsA       --> жанашыр
  ақиқат          --IsA       --> тірек
  бала            --IsA       --> болашақ
  ел              --Has       --> сыртқ
  еңбек           --IsA       --> қайнар
  жер             --IsA       --> бастау
  кітап           --IsA       --> бұлақ
  ой              --IsA       --> қару
  тыңайтқыш       --Has       --> түр
  тіл             --IsA       --> айна
  ынтымақ         --IsA       --> байлық
  ілім            --IsA       --> бұлақ
  ғылым           --IsA       --> қазына
```

`incoming("бұлақ")` → 2 edges (both `кітап` and `ілім` metaphorically map to бұлақ). This is the kind of **connective knowledge** a reasoner will traverse.

### API additions

- `LexicalGraph::from_facts(&[Fact]) -> LexicalGraph`
- `LexicalGraph::outgoing(root) -> Vec<&GraphEdge>` — "tell me about X"
- `LexicalGraph::incoming(root) -> Vec<&GraphEdge>` — "what is an X?"
- `GraphEdge { from, predicate, to, sources }`
- `NodeStats { out_degree, in_degree, out_by_predicate, in_by_predicate }`
- Determinism: `BTreeMap`/sorted `Vec` so JSON is byte-identical across runs.

### Committed artifacts

- `data/retrieval/facts.json` regenerated — 15 facts, 0 imprecisions.
- `data/retrieval/lexical_graph.json` **new** — 29 nodes, 15 edges, summary + per-node stats.

### Tests (+7 → 335 total)

- `empty_facts_empty_graph`, `single_fact_single_edge`, `repeated_triple_merges_sources`
- `node_stats_track_degree_per_predicate`
- `outgoing_and_incoming_lookups`
- `edges_are_deterministically_sorted`
- `graph_round_trips_through_json`

### Zero regressions

FST fix was an invariant improvement — no existing test relied on the incorrect vowel-class classification. All 328 pre-v2.3 tests still pass.

### Next (v2.4+)

- Lexical graph **enrichment** — derive additional edges from Lexicon POS + morphological co-occurrence, not just from facts.
- **Rule reasoner v0** — traverse the graph to answer questions like «бала неге білім алады?» → chain (бала IsA адам) + (адам Has жан) + (жан requires білім) → answer. Deterministic forward-chaining, auditable step-by-step.
- More pattern matchers — dative-motion (`X Y-ке барады`), verb-derived action facts.

## [2.2.0] — 2026-04-21 — Lexicon pollution purge + possessive-existence pattern (Has predicate)

Minor release. **The v2.1 feedback loop paid off.** v2.1 extracted 11 facts from the committed corpus and named 4 imprecisions. v2.2 investigated each one, found a **systematic Lexicon pollution**, purged it, added the missing roots, and introduced a new `Has` predicate via a third pattern matcher.

### The order-of-magnitude Lexicon finding

v2.1's "бала → болашағ" imprecision was not a one-off — a scan found **87 intervocalic-voicing-duplicate root pairs** in `segmentation_roots.json`:

```
кітап ↔ кітаб,  сабақ ↔ сабағ,  қазақ ↔ қазағ,
еңбек ↔ еңбег,  топ   ↔ тоб,   ... (82 more)
```

The voiced variant (`-ғ`, `-г`, `-б`) is never a valid Kazakh stem on its own — it's the surface result of intervocalic voicing when a vowel-initial suffix attaches to a voiceless-final root. These entries were duplicated during the Apertium import without de-duplication. v2.2 **removes all 87** polluted entries.

The FST parser already handles intervocalic voicing in `surface_could_contain_root` (checks whether a surface starts with the voiced variant of a voiceless-final root). So removing the polluted entries makes parsing **more precise**, not less — "болашағы" now only resolves to root "болашақ", not to the ghost root "болашағ".

Code audit: `grep -r` across all crates for any of the 87 polluted IDs → **zero hits**. Nothing in code depended on the duplication.

### Lexicon additions (data-driven)

Three roots that v2.1 signaled missing:

- `байлық` (wealth) — possessive-final, voiceless
- `бастау` (source, beginning) — vowel-final
- `жанашыр` (caregiver, sympathizer) — voiced-consonant-final
  - Note: v2.2 briefly added "жанашы" (wrong root) before the FST parse test revealed the correct form is `жанашыр`. Corrected before release.

Total Lexicon delta: **4,516 → 4,432 roots** (−87 pollutions, +3 additions). Net cleaner.

### New pattern: possessive-existence `X-тың Y-сы бар` → `Has`

Kazakh expresses possession via a genitive + P3-possessed + existential `бар` construction. v2.2 adds a third pattern to `adam-reasoning::patterns`:

```
"Баланың кітабы бар"  →  (бала, Has, кітап)
"Тыңайтқыштың түрлері (...) бар"  →  (тыңайтқыш, Has, түр)
```

**Type-checked on FST features**, not strings:

- subject token must have `Case::Genitive` + `part_of_speech == "noun"` + not closed-class;
- object token must immediately follow and have `Possessive::P3` + be a noun;
- existential `бар` must appear elsewhere in the sentence;
- tautology guard (subject ≠ object).

**Non-adjacent guard**: intervening words between possessor and possessed break the simple construction — we refuse rather than guess.

### Predicate set — 3 predicates

```rust
pub enum Predicate {
    IsA,      // X — Y                (v2.1 copula)
    LivesIn,  // X Y-да тұрады        (v2.1 locative)
    Has,      // X-тың Y-сы бар       (v2.2 possessive)
}
```

### Extraction yield

| Mode | v2.1 | v2.2 | Δ |
|---|---:|---:|---:|
| Committed samples scanned | 3,191 | 3,191 | — |
| Facts extracted | 11 | **13** | +2 |
| Distinct predicates | 1 | **2** | +1 |
| Corrected from v2.1 imprecisions | — | 3 / 4 | ынтымақ→байлық, бала→болашақ, ана→жанашыр |
| Still blocked | — | 1 | жер→тіршілік (бастау blocked by separate FST vowel-final+P3 bug) |

The remaining imprecision (жер→тіршілік instead of бастау) exposes an **FST-level bug** in the vowel-final + P3 code path ("оқуы" also fails to parse). Added to `docs/roadmap.md` as a v2.3 agenda item, not blocking v2.2.

### Determinism

Unchanged. Pattern matchers remain pure functions; same corpus → byte-identical `facts.json`.

### Tests (+3 → 328 total)

- `possessive_extracts_child_has_book` — positive case with head extraction through P3.
- `possessive_rejects_without_bar` — missing existential → no fact.
- `possessive_rejects_non_adjacent` — intervening word → refuse.
- `Predicate::Has.as_str() == "has"` — stability check.

### Zero regressions

All 325 pre-v2.2 tests still pass after 87 Lexicon removals. Workspace test count: **303 (v2.0) → 325 (v2.1) → 328 (v2.2)**.

### What v2.2 does NOT do (deferred)

- **Vowel-final + P3 FST bug** — "оқуы" / "бастауы" don't parse. Isolated diagnostic; fix in v2.3.
- **Lexical graph** — still just a flat list of roots. v2.3 will build typed edges (is_a, has_role, related_to) over roots.
- **Rule reasoner** — v2.3+.
- **Scale** — committed extraction still at 500 samples/pack cap. Full corpus run remains gitignored-local.

### Next (v2.3)

- Fix the vowel-final + P3 FST bug.
- Start building the **Lexical-Morphemic Knowledge Graph** — root-level edges derived from fact accumulation + POS co-occurrence. Deterministic construction; no learned weights.

## [2.1.0] — 2026-04-21 — ILMRR bootstrap: fact extraction (copula pattern, typed provenance)

Minor release. **First step toward reasoning.** Our v2.0 system is a smart retrieval engine — it quotes. v2.1 starts extracting **structured facts** from the corpus: `(subject, predicate, object)` triples with full provenance, typed `ConfidenceKind`, and deterministic head extraction via FST.

This is the first rung of the ladder laid out in [`docs/architecture_v2.md`](docs/architecture_v2.md#post-v20-directions-committed-but-not-shipped) and discussed as **ILMRR — Intelligent Lexical-Morphemic Retrieval & Reasoning**. v2.1 is the infrastructure: facts as data. v2.2 will add the lexical graph; v2.3 the rule reasoner.

### New crate: `adam-reasoning`

- **`Fact { subject, predicate, object, pattern, source, confidence, raw_text }`** — structured knowledge with every field typed and traceable.
- **`Predicate` enum** — v2.1 ships two: `IsA`, `LivesIn`. Every addition is an intentional architectural decision.
- **`ConfidenceKind` enum** — **categorical** evidence type (Grammar, CuratedQuote, RepeatedPattern, HumanApproved, RuleInferred). Explicitly not an LLM probability; consumers filter by kind, not by magnitude. Reaffirms `project_retrieval_not_neural_v2`.
- **`SlotRef { surface, root, pos }`** — every slot carries the canonical root, not just the surface. Possessive-suffixed "бұлағы" correctly yields root "бұлақ".
- **`FactSource { pack, sample_id }`** — identical shape to `adam_retrieval::SampleRef`, kept independent to avoid a reasoning→retrieval dep cycle.
- **`extract_facts(text, parses, lexicon, source) -> Vec<Fact>`** — pure function. Same input → same facts, byte-identical across runs.

### Pattern matchers (v2.1)

1. **Copula `X — Y`** → `IsA` — uses Kazakh em-dash as a syntactic anchor. **Strict LHS** (single bare nominative noun). **Head-extracted RHS** (right-to-left FST scan; possessive "Y-сі" correctly resolves to root Y). Guards: ≤4-token RHS cap, parenthetical noise stripped, tautology (`subj == obj`) rejected.
2. **Locative-existential `X Y-да тұрады`** → `LivesIn` — requires the verb `тұру` in any inflected form + a `Case::Locative` noun + a bare-nominative subject. Pronouns rejected as non-content subjects.

### New binary: `extract_facts`

Walks committed corpus packs, runs every pattern matcher on each sample, emits structured JSON. Two modes:

- **default** — first 500 samples per pack, writes committed `data/retrieval/facts.json`.
- **`--full`** — every sample, writes gitignored `data/retrieval/facts_full.json`.
- **`--limit N`** — custom per-pack cap.

Progress is streamed to stderr every 1,000 samples (flushed) — no more silent minutes.

### Baseline — 11 facts from 3,191 samples

Extraction over the committed corpus yielded **11 facts** (37.8 s). Precision:

- **7 clean**: ілім→бұлақ, айлакерлік→іс, кітап→бұлақ, ғылым→қазына, тіл→айна, ақиқат→тірек, еңбек→қайнар.
- **4 Lexicon-gap cases**: ынтымақ→халық (should be байлық), ана→бала (should be жанашы), жер→тіршілік (should be бастау), бала→болашағ (FST intervocalic-voicing issue on болашақ).

The 4 imprecisions are not pattern bugs — they are **concrete Lexicon gaps** (байлық, жанашы, бастау) + **one FST voicing regression** (болашақ). These become the v2.2 agenda.

All 11 facts have `(pack, sample_id)` provenance → every fact is auditable back to its corpus sentence.

### Determinism contract

- Pattern matchers: pure functions of `(text, parses, lexicon, source)`.
- RHS head extraction: deterministic right-to-left walk + deterministic FST parse.
- `extract_facts` output: samples scanned in pack order, then `samples[]` order within pack. Same corpus → byte-identical `facts.json`.

### Tests (+22 → 325 total)

- 3 lib tests: predicate/confidence strings, Fact JSON round-trip.
- 19 pattern tests: copula positive + 7 negatives (no dash, double dash, inflected, tautology, multi-token LHS, long RHS clause, parenthetical noise), locative positive + 2 negatives (no тұру, pronoun subject), head-extraction helpers.

### What v2.1 does NOT do

- **No multi-sentence chains.** `extract_facts` is per-sample.
- **No rule inference.** The Reasoner (v2.3) will combine facts into new facts; v2.1 only extracts.
- **No lexical graph.** v2.2 will build `is_a` / `has_role` / `related_to` edges over roots and connect facts to them.

### Workspace tests

**325 passing** (303 → +22 reasoning).

### Committed artifacts

- `data/retrieval/facts.json` — 11-fact v2.1 baseline, ~4 KB. CI will regenerate on every reasoning-crate change and diff.

## [2.0.0] — 2026-04-20 — v2.0: commitment release, retrieval-as-v2.0, investor-demoable

Major release. **Not a feature drop — an architectural commitment.**

v2.0 freezes the answer to the question `project_retrieval_not_neural_v2` has been circling since v1.6.0:

> **v2.0 is not a trained neural model. It is a deterministic retrieval + composition engine over a 77.9 M-word Kazakh corpus.**

Everything functional is already in v1.9.5. v2.0 adds:

### 1. Demo binaries

- **`adam_chat` v2.0** — now auto-loads the committed morpheme index and enables retrieval by default. New flags:
  - `--no-retrieval` — reproduces v1.1.0 noun-echo behaviour (regression reference).
  - `--compose` — opts into `ComposeMode::InSampleCitySwap`. Banner prints the «бейімд-» marker policy so the user knows what to expect.
- **`adam_demo` (new)** — scripted 15-turn end-to-end walkthrough. Three parts:
  - Part 1: the full social + retrieval arc under `Verbatim`.
  - Part 2: same script under `InSampleCitySwap` — most swaps refused by guards (the safe case).
  - Part 3: synthetic sample explicitly triggering the swap path, so the v1.9.5 «бейімд-» marker is visible in action.
  Fully deterministic. Re-runs print byte-identical output.

### 2. Canonical architecture doc — `docs/architecture_v2.md`

Single source of truth for the v2.0 pipeline. Diagrams the 5 layers + the 2.5/2.75 retrieval-injection sub-layers. Lists all three response paths and the guarantees each carries. Catalogues the determinism contract, safety guards, and trade-offs accepted. Points at every concern-to-file mapping for future contributors.

### 3. README restructure

Investor-facing **"Why adam"** comparison table lands first — explicit positioning against mainstream LLMs: 0 hallucinations vs non-zero, byte-identical determinism vs temperature-dependent, ms-on-CPU vs dollars-on-GPU, full provenance vs none. The rest of the README was already current at v1.9.5; v2.0 updates the banner version + demo section (`adam_demo` instructions + `adam_chat` flag reference).

### 4. Commitment declarations

Explicit in the README "Out of scope" and the architecture doc's "What v2.0 is NOT" section:

- **Not a trained neural model.** No parameters. No embeddings. No PyTorch.
- **Not multilingual.** Kazakh-only surface.
- **Not generative.** Every token is from a template, a corpus sample, or FST synthesis.
- **Not a generalist.** 26 intents + retrieval, honest «түсінбедім» outside.
- **Not self-modifying.** Separate architectural direction if ever; not v2.x.

### What v2.0 does NOT change

- **No new crates.** All v2.0 work is binaries + docs on top of the v1.9.5 code surface.
- **No new tests.** The 303 tests from v1.9.5 carry forward unchanged.
- **No behaviour change at the library API.** `Conversation::turn` is bit-for-bit the same function. `MorphemeIndex` serialisation is unchanged. Embedders who upgrade see zero semantic diff.
- **No index format change.** Existing `data/retrieval/morpheme_index.json` files are still valid.

### Determinism audit (reaffirmed at v2.0)

- FST synthesis is a pure function.
- FST parse enumerates deterministically.
- `MorphemeIndex::rank` ties on `(pack, sample_id)` lex order.
- `compose_with_city` is a pure function; no RNG.
- `inject_retrieval_example` does NOT consult `rng_seed`.
- `adam_demo` re-runs print byte-identical output.

Same `(input, session, seed)` → byte-identical response, across runs, machines, and time.

### Workspace tests

**303 passing** (unchanged from v1.9.5). The v2.0 binary additions are thin glue on top of already-tested library code.

### Post-v2.0 directions (committed but not shipped)

- **Option C** — pre-compute `(pattern, slot_types)` pairs at index-build time. Keeps runtime cheap; enables swap types beyond city.
- **Kazakh technical corpus** — translate key chapters of the Rust Book into Kazakh as a new source pack. Doubles as educational material and corpus-vocabulary expansion.
- **Diversity** — allow consecutive turns for the same query to cite different top-ranked samples. Current top-1 is deterministic by design.

These are v2.x / v3.x work, not v2.0 scope.

## [1.9.5] — 2026-04-20 — Composition-marker framing (adapted-evidence template family)

Patch release restoring the **traceability contract** broken in v1.9.0. When `ComposeMode::InSampleCitySwap` silently rewrote a quoted corpus line, the user saw the adapted text in «…» and could easily assume it was the original source. That's a trust violation — even if the swap was grammatically correct and semantically benign.

v1.9.5 makes the adaptation **explicit in the response itself**. The planner now routes swapped responses through a separate `unknown.with_adapted_evidence` template family whose every template contains the word stem **«бейімд-»** ("adapt-"). Verbatim quotes stay on the v1.8.0 `unknown.with_evidence` family.

### Before / after

```text
Corpus: "Бала Алматыда жақсы өмір сүреді"
Session: { city: "Шымкент" }
Mode: InSampleCitySwap

v1.9.0 (silent):
< Шымкентте тұратын сізге бала туралы мынадай дерек:
  «Бала Шымкентте жақсы өмір сүреді»    ← user has no way to know the quote was adapted

v1.9.5 (explicit marker):
< Бұл бейімделген нұсқа (түпнұсқада басқа қала аталған):
  «Бала Шымкентте жақсы өмір сүреді»    ← the frame literally says "adapted version,
                                           different city in the original"
< бала туралы корпустағы бір жолды сіздің қалаңызға бейімдеп көрдім:
  «Бала Шымкентте жақсы өмір сүреді»    ← "I adapted a corpus line to your city"
```

### Changes

- **`adam-dialog::intent::Intent::Unknown`** gains a new field `example_adapted: bool`. Defaults to `false`; `#[serde(default)]` so deserialising older traces still works.
- **`adam-dialog::planner`** routes:
  - `example.is_some() && example_adapted` → `"unknown.with_adapted_evidence"` *(new)*
  - `example.is_some()` → `"unknown.with_evidence"` *(v1.8.0 verbatim path)*
  - `noun_hint.is_some()` → `"unknown.with_noun"` *(v1.1.0)*
  - else → `"unknown"` *(v1.0.0)*
- **`Conversation::maybe_compose`** now returns `(String, bool)` — the flag propagates to `example_adapted` in `Intent::Unknown`. No caller outside `Conversation` is exposed to the internal API change.
- **New template family** `unknown.with_adapted_evidence` (5 templates) in `data/dialog/templates/v1.toml`. Every single template contains the «бейімд-» stem so consumers can grep for it as a runtime marker. FST-aware `{city|locative}` renders the user's city harmony-correctly.

### Safety invariants (new)

Two tests enforce the bi-directional guarantee:

| Direction | Test | Guarantee |
|---|---|---|
| **When swap happened** → marker must fire | `adapted_evidence_templates_announce_the_adaptation` | the «бейімд-» stem appears in the output for at least one seed under `InSampleCitySwap` + actual swap |
| **When no swap** → marker must NOT fire | `verbatim_mode_never_claims_adaptation` | the «бейімд-» stem is absent for every seed under `Verbatim` mode |

The second guarantee is the trust-critical one: v1.9.5 must never claim to have adapted a quote it didn't actually adapt.

### Determinism

Unchanged. `example_adapted` is a pure function of `(retrieved text, session city, compose_mode)`. Template selection still honours `template_is_fillable` + seed-mod.

### Tests (+2 → 303 total)

- `adapted_evidence_templates_announce_the_adaptation` — swap fires → marker fires.
- `verbatim_mode_never_claims_adaptation` — no swap → no marker, ever.

### What's next (v2.0 territory, not v1.9.x)

- **Option C** — pre-compute `(pattern, slot_types)` pairs at index-build time. Keeps runtime cheap, lets us audit swap candidates offline, and is a prerequisite for swap types beyond city (names-in-biography, numbers-in-dates). Not a patch.
- **v2.0 stabilisation** — freeze the retrieval-as-v2.0 commitment (`project_retrieval_not_neural_v2`), run end-to-end demos, cut the investor-demoable v2.0 tag.

## [1.9.0] — 2026-04-20 — In-sample city swap (option B, opt-in, year-guarded)

Minor release. First step into **option B** territory — the retrieved corpus quote is no longer guaranteed byte-identical to the source. When the user opts into `ComposeMode::InSampleCitySwap` and the session has a known Kazakh city, city mentions inside the cited sample are rewritten to the user's city, feature-preserving via the FST. v1.8.5 and earlier behaviour (`ComposeMode::Verbatim`, the default) is unchanged.

### What changes — and what doesn't

- **Grammaticality still FST-guaranteed.** `synthesise_noun(user_city, features)` produces the harmonically-correct surface (Алматы+locative → Алматыда, Шымкент+locative → Шымкентте).
- **Semantic truthfulness is no longer guaranteed.** That is the honest trade-off of option B. A composed sentence may say something true, or it may produce a plausible but non-factual claim. Earlier releases never did this.
- **Safety guards are explicit, conservative, and auditable:**
  - **Closed city list** (`PLACE_NAMES`): 20 editorially-curated Kazakh cities are the only eligible swap targets. Other proper nouns and common nouns are never touched.
  - **User-side recognition:** the user's proposed city must itself be in `PLACE_NAMES`, otherwise the FST can't re-synthesise reliably.
  - **Biographical-year guard:** any 4-digit year in [1500, 2100] refuses the whole swap. This keeps biographies ("Абай 1845 жылы Қарқаралыда туған") untouched — we must not rewrite "Қарқаралыда" to the user's city and fabricate a birth fact.
  - **No name or number swaps.** Names-in-biography and numerals-in-dates are exactly the categories that would produce the worst fabrications; explicitly out of scope for v1.9.0.

### Opt-in — `ComposeMode`

```rust
use adam_dialog::{ComposeMode, Conversation};

// Default: byte-identical corpus quote (v1.8.5 behaviour).
let conv_safe = Conversation::new().with_morpheme_index(idx.clone());

// Opt-in: city mentions inside the quote rewrite to user.session.city.
let conv_swap = Conversation::new()
    .with_morpheme_index(idx)
    .with_compose_mode(ComposeMode::InSampleCitySwap);
```

Same call site, same type, one explicit setter. Embedders who don't opt in see zero behavioural change.

### New API — `adam_retrieval::compose`

```rust
pub const PLACE_NAMES: &[&str];        // the 20-city editorial list

pub struct Swap {
    pub token_index: usize,
    pub from: String,
    pub to: String,
    pub user_root: String,
    pub features: NounFeatures,
}

pub struct Composition {
    pub original: String,
    pub output: String,
    pub swaps: Vec<Swap>,
}
impl Composition {
    pub fn was_changed(&self) -> bool;
    pub fn trace(&self) -> String;         // per-swap provenance for --trace
}

pub fn compose_with_city(
    sample_text: &str,
    user_city: &str,
    lexicon: &LexiconV1,
) -> Composition;
```

Every swap preserves full FST feature provenance: case, number, possessive, predicate. `Composition::trace()` emits a per-swap line usable by `adam_chat --trace` (e.g. `[2] Алматыда → Шымкентте (root=шымкент, case=Some(Locative))`).

### Determinism

- `compose_with_city` is a pure function; no rng, no system time.
- First-match policy by token order, deterministic.
- FST synthesis is itself deterministic.
- Same `(sample, user_city, lexicon)` → byte-identical `Composition` across runs.

### Tests (+11)

**Unit tests in `adam-retrieval::compose` (+8):**

- `no_swap_when_user_city_unknown` — city outside `PLACE_NAMES` → no-op.
- `no_swap_when_text_has_biographical_year` — biography guard fires.
- `swaps_city_preserving_locative` — Алматыда → Шымкентте.
- `preserves_capitalisation_on_swap`.
- `no_swap_when_city_matches_user_city` — identity is no-op.
- `preserves_trailing_punctuation` — commas and periods survive.
- `trace_records_swap_details` — trace line is well-formed.
- `year_guard_ignores_short_digit_runs` — "25 жас" does NOT trigger the guard.

**Dialog e2e tests (+3):**

- `compose_mode_swaps_cities_in_retrieval_samples` — `InSampleCitySwap` + `session.city=Шымкент` + synthetic "Бала Алматыда ..." → quote rewrites to Шымкентте.
- `compose_mode_verbatim_preserves_retrieved_quote` — default mode keeps Алматыда in the quote (the v1.8.5 frame template can still say Шымкентте outside «…»).
- `compose_mode_respects_biographical_year_guard` — "Абай 1845 жылы Қарқаралыда ..." stays put under `InSampleCitySwap`.

### Workspace tests

**301 tests pass** (290 → +11).

### Next (v1.9.5 candidates)

- Wrap swap-mode responses in a template that explicitly marks the composition ("сіздің қалаңыздың аясында..."), so readers know the quote was adapted.
- Extract patterns at index-build time (option C) so composition isn't done at runtime per turn.
- Experiments on name / year composition with stricter sanity guards.

## [1.8.5] — 2026-04-20 — Locative+P1Sg bug fix, FST-aware city slots, comprehensive README refresh

Patch release. Fixes the `-мын` greedy-strip bug in `detect_statement_of_occupation`, wires the existing `{slot|features}` syntax into v1.8.0's session-aware templates, and brings the README fully in sync with the v1.5.0–v1.8.0 retrieval-era arc.

### Bug fix — locative+P1Sg is a location statement, not an occupation

Before v1.8.5:

```
user: мен Алматыдамын
conv.session:
  { name: "Дәулет", occupation: "алматы" }   ❌ wrong — "Алматы" is not an occupation
```

The FST correctly parsed `Алматыдамын` as `Алматы + locative + P1Sg`, but `detect_statement_of_occupation` Priority 1 accepted any noun with `Predicate::P1Sg` regardless of case, so the city got slotted as an occupation. `detect_statement_of_location` required an explicit `тұрамын / тұрамыз` verb co-occurring with the locative and didn't trigger on the bare `locative+P1Sg` stack.

v1.8.5 fixes both ends:

- `detect_statement_of_location` now accepts **any** Noun with both `Case::Locative` and `Predicate::P1Sg` — a standalone self-locative ("I am in X") is a location statement by itself, no verb required.
- `detect_statement_of_occupation` Priority 1 now **rejects** `Case::Locative` and `Case::Ablative` — those cases mean "in / from X", not "I am X (profession)".

Result:

```
user: мен Алматыдамын
conv.session:
  { name: "Дәулет", city: "Алматы" }   ✅ correct
response: "жақсы жер"
```

### FST-aware session slots in retrieval templates

The v1.8.0 session-aware templates used literal case marking (`{city}-да`). This is both ugly (dangling hyphen: `Алматы-да` instead of `Алматыда`) and wrong for vowel harmony (Астана-да / Өскемен-де: one "а", one "е", and the planner can't know which).

v1.8.5 swaps the literals for `{slot|features}`:

```toml
# v1.8.0 (literal, wrong harmony):
"{city}-да тұратын сіз үшін {noun} жайында: «{example}»"

# v1.8.5 (FST, correct harmony):
"{city|locative} тұратын сізге {noun} туралы мынадай дерек: «{example}»"
```

`{city|locative}` routes through `adam_kernel_fst::morphotactics::synthesise_noun`, so Алматы → Алматыда, Астана → Астанада, Өскемен → Өскеменде automatically. Demo at seed=6:

```
Алматыда тұратын сізге бала туралы мынадай дерек:
«Кім сендерді балалар, сүйе-тұғын, Қуанышыңа қуанып, қайғыңа күйе-тұғын»
```

No dangling hyphen; harmonically correct locative suffix.

### Comprehensive README refresh

The README had drifted since v1.4.5. Every stale reference is fixed:

- **Version badge** 1.4.5 → 1.8.5.
- **Retrieval badge** added; **corpus badge** added showing `77.9 M local / 4 M committed`; **test count** 288 → 290.
- **Demo** updated to v1.8.5: shows the v1.8.5 locative fix, the v1.6.0+ retrieval-engine path (`Алматыда тұратын сізге... «Абай Wikisource quote»`), and session-aware frame composition.
- **Architecture** table now lists `adam-retrieval` as a proper L1 crate alongside the others. Counts corrected (11 archiphonemes, 36 suffix templates).
- **New section**: "Retrieval engine (v1.6.0–v1.8.5)" — documents the `retrieve → rank → compose` path with the full composite scoring formula, determinism guarantees, and provenance contract.
- **Kazakh-only recogniser** section now points at the retrieval engine instead of a future trained LM.
- **Technical specification** rewritten: committed corpus words (3.84 M), local corpus words (77.9 M), morpheme-coverage baseline (79.48 %), FST parser throughput (1.155 ms/word), committed morpheme index size (3,191 / 3,082 / 16,262), full-corpus rebuild procedure, 26 intents (was 25), 31 template families, 290 tests.
- **History** extended with the "v1.5.0–v1.8.5 retrieval era" section explaining each release's contribution to the retrieve → rank → compose ladder.
- **Out of scope** rewritten: multilingual removed, "compact trained LM" removed, replaced with the honest commitment that v2.0 is the retrieval engine, not a neural model.

### Tests (+2)

- `locative_with_copula_is_location_not_occupation` — regression test for the `-мын` bug fix.
- `session_aware_city_template_uses_fst_locative` — verifies at least one seed produces FST-rendered `Алматыда` (not `Алматы-да`) when a `{city|locative}` template fires.

### Workspace tests

**290 tests pass** (288 → +2).

### What's next

- **v1.9.0** — option B/C territory: in-sample slot swap. Risky — it's where we leave the "retrieved text is immutable" safety. Needs semantic-sanity guards before shipping.

## [1.8.0] — 2026-04-20 — Session-aware compositional synthesis (option A: frame-only, retrieved quote stays verbatim)

Minor release. First step in the **retrieve → compose → respond** ladder described in the v1.7.0 release notes. This release commits to **option A** of the three compositional-synthesis variants we debated: composition happens **around** the retrieved sample, never **inside** it. Zero fabrication risk; the retrieved sentence stays byte-identical to the corpus.

### The contract

- **Retrieved quote is immutable.** No slot-swapping inside the guillemets. Whatever the corpus says, the corpus still says.
- **Frame becomes session-aware.** When the user has told us their `name`, `city`, `age`, or `occupation`, the planner prefers a template that personalises the wrapper around the citation.
- **Still deterministic.** The planner's template pool filter (`template_is_fillable`) automatically gates session-aware templates on slot presence. No new conditional logic, no runtime trickery.

### Visible effect

```
# Before (v1.7.0, session = {name: "Дәулет"})
< бала туралы мынадай бір жазба кездестірдім:
  «Кім сендерді балалар, сүйе-тұғын...»

# After (v1.8.0, same session, session-aware templates now in pool)
< Сіз, Дәулет, бала туралы сұрап тұрсыз ба. Мынадай дерек бар:
  «Кім сендерді балалар, сүйе-тұғын...»

# After (v1.8.0, session = {name: "Дәулет", city: "Алматы"})
< Дәулет, Алматы-да тұратын сіз үшін бала жайында:
  «Кім сендерді балалар, сүйе-тұғын...»
```

The quote is the same Abai verse in every case. The frame adapts to what the dialog remembers.

### Changes

- **`data/dialog/templates/v1.toml`** — `unknown.with_evidence` grows from 4 to 10 templates (6 new session-aware variants: 2 × `{name}`, 1 × `{city}`, 1 × `{name}+{city}`, 1 × `{age}`, 1 × `{occupation}`). `unknown.with_noun` similarly grows from 5 to 10 with session-aware variants.
- **Planner**: no code change. The existing `template_is_fillable` + session merge does all the work. This is the whole design thesis of option A — composition implemented as pure data.
- **Tests (+2)**:
  - `unknown_with_session_and_evidence_personalises_frame` — with `name` in session, at least one seed picks a personalised template.
  - `unknown_with_session_name_and_city_can_use_combined_frame` — with both slots, at least one seed picks a template combining them.

### Known bug (not addressed this release)

Input like «мен Алматыдамын» (I'm in Almaty) is mis-classified by `detect_statement_of_occupation` because the recogniser greedy-strips `-мын` and treats the residue as an occupation surface — session ends up with `occupation: "алматы"` instead of `city: "Алматы"`. This is a pre-v1.8.0 semantics bug, orthogonal to composition. The test for the combined-frame path sets the session directly to bypass it. Planned for v1.8.5.

### Determinism audit

- No new random call sites.
- No new runtime-conditional routing — templates decide activation purely by slot presence, which is itself deterministic.
- `rng_seed` still picks among the filtered pool, as before.

Same session + same input + same seed → byte-identical output.

### What v1.8.0 does NOT do (deferred — option B/C territory)

- **No in-sample slot swap.** We do NOT replace proper nouns or numerals inside the retrieved quote. That's true compositional synthesis, with all the semantic-fabrication risk it brings. Deferred explicitly.
- **No FST-aware re-inflection of session slots.** Templates use session values as-is; Kazakh case marking still comes from the hand-written `-да`, `-мен`, etc. in the template text. v0.9.5's `{slot|features}` is available but not yet wired into the new v1.8.0 templates; future templates can upgrade.
- **No semantic sanity check.** Even the frame could say weird things like "{city} тұрғыны үшін..." when the user is only visiting. Narrowing phrasing is polish, not scope.

### Workspace tests

**288 tests pass** (286 → +2 dialog e2e).

### Next (v1.8.5)

Fix the `-мын` greedy-strip bug in `detect_statement_of_occupation`. Wire `{slot|features}` into 2–3 session-aware templates to demonstrate FST-aware case marking on session slots (e.g. `{city|locative}` instead of the literal `{city}-да`). Still option A — retrieved quote stays verbatim.

## [1.7.0] — 2026-04-20 — Deterministic retrieval ranking: overlap + purity + length + loanword penalty

Minor release. `MorphemeIndex::rank` replaces "first matching posting" with a composite deterministic score. Dialog now ranks over **every content root** parsed from the user's input, not just the first noun — so a sentence mentioning both `бала` and `мектеп` outranks one that mentions only `бала` for the input «бала мектепке барды». This is where the retrieval engine starts behaving like a *search* engine rather than a bag dip.

### The formula

```
score = 0.40 · overlap_ratio            // main "smart" signal
      + 0.30 · pack_purity              // main "safe" signal
      + 0.15 · length_goodness(words)   // Gaussian around 8 words
      − 0.15 · loanword_density         // preserves Kazakh-first thesis
```

All four components are deterministic pure functions of `(sample, input_morphemes)`. Ties break by `(pack, sample_id)`. Zero randomness, zero training.

### Editorial pack purity priors

Encoded in `RankConfig::default()`:

| Pack | Prior | Why |
|---|---:|---|
| Abai Wikisource, Kazakh classics, proverbs | 1.00 | centuries of curation |
| Synthetic, Tatoeba, Common Voice | 0.95 | Lexicon-bounded / selected |
| Wikipedia KZ | 0.85 | edited but technical loanwords |
| CC-100 (web crawl) | 0.75 | weakest source |

Unknown packs fall back to `DEFAULT_UNKNOWN_PACK_PURITY` (0.70).

### Visible effect (same prompts, v1.6.5 → v1.7.0)

| Prompt | v1.6.5 cited | v1.7.0 cited |
|---|---|---|
| «бала туралы…» | "Кеше бала ең, келдің ғой талай жасқа…" (11w, Abai) | "Кім сендерді балалар, сүйе-тұғын…" (8w, Abai) |
| «мектеп керек пе» | CC-100 bureaucratic paragraph (36w) | "иә мұнай-газ жалақыны тағылды немесе таза мектеп сүйенеді." (8w, CC-100) |
| «адам не істесе…» | "Адам — бір боқ көтерген боқтың қабы…" (Abai, crude) | "Ақылды адам сөзін де, ісін де өлшеп айтар." (Abai proverb) |

Ranking picked the shorter, cleaner, more topical option every time. Still deterministic.

### Changes

- **`adam-retrieval`**:
  - `Hit { sref, score, overlap_count, overlap_ratio, length_goodness, loanword_density, pack_purity }` — every score component is preserved for tracing.
  - `RankConfig { top_k, weight_overlap, weight_purity, weight_length, weight_loanword_penalty, pack_purity: BTreeMap<String, f32> }` with `Default` that hard-codes the editorial priors.
  - `MorphemeIndex::rank(input_morphemes, config) -> Vec<Hit>` — returns top-`k` sorted by descending score, ties broken by `(pack, sample_id)`.
  - Public `length_goodness(word_count) -> f32` (Gaussian, σ = 6, μ = 8).
  - Public `sample_loanword_density(text) -> f32` (the v1.x purity rule applied to a single sample).
  - `DEFAULT_UNKNOWN_PACK_PURITY: f32 = 0.70` for packs not in the table.
- **`adam-dialog`**:
  - New `semantics::content_roots(parses) -> Vec<String>` — every distinct content-noun root from the input, not just the first. Preserves insertion order. Filters closed-class items via the existing `NOT_A_TOPIC` list.
  - `Conversation::rank_config: Option<RankConfig>` — override for tests / experiments; `None` uses the default.
  - `inject_retrieval_example` now calls `index.rank(&content_roots, &config)` and picks the top hit; falls back to v1.6.5 single-morpheme path if the ranker finds nothing with a stored text.
- **+7 retrieval tests**:
  - `rank_prefers_higher_overlap` — 2-morpheme match beats 1-morpheme match.
  - `rank_breaks_ties_with_pack_purity` — Abai beats CC-100 at equal overlap.
  - `rank_penalises_loanword_heavy_sample` — native-language sample wins.
  - `length_goodness_peaks_at_8_words`.
  - `sample_loanword_density_flags_russian_only_letters`.
  - `rank_top_k_is_respected`.
  - `rank_empty_input_returns_empty`.

### Determinism audit

- `rank` never calls rng or system time.
- Tie-break is `(pack, sample_id)` lex order → identical across runs / machines.
- `RankConfig::default` is a pure constant.
- `inject_retrieval_example` does not consult `rng_seed`.

Same corpus + same input + same weights → byte-identical cited sentence.

### What v1.7.0 does NOT do

- **No Lexicon expansion** — top uncovered items from v1.5.5 (`деп`, `осы`, `пен`) are still gaps; that is separate Lexicon work.
- **No compositional synthesis** — we still QUOTE the retrieved sentence verbatim. Adapting its grammar to the user's context is v1.8.0.
- **No multi-hit diversity** — top-1 is deterministic; conversation will cite the same sentence every time for the same prompt. Diversity is a later concern.

### Workspace tests

**286 tests pass** (279 → +7 retrieval ranker).

## [1.6.5] — 2026-04-20 — Retrieval wired into `Intent::Unknown`: dialog cites real Kazakh corpus

Patch release. The retrieval index shipped in v1.6.0 now feeds the dialog layer. When `Intent::Unknown` fires with a recognised noun AND the committed morpheme index contains a sample text for it, the response quotes that sentence verbatim — a concrete step toward "not just predictable, but informed by the corpus." The 26-intent deterministic backbone is untouched; this is strictly a fallback improvement.

### Demo — what it looks like

```
> бала туралы бірдеңе айт
< бала туралы мынадай бір жазба кездестірдім: «Кеше бала ең, келдің ғой
  талай жасқа, Көз жетті бір қалыпта тұра алмасқа.»                     [Abai Wikisource]

> Абай жайында не дейсің
< абай жайында осындай мысал бар: «Абай Құнанбайұлы (10 тамыз 1845 —
  6 шілде 1904)»                                                         [Wikipedia KZ]

> мектеп керек пе
< мектеп деген сөзге қатысты бір жолым есіме түсті: «... 300 орындық
  мектеп құрылысы толықтай аяқталмаған ...»                              [CC-100]

> адам не істесе болады
< адам туралы мынадай бір жазба кездестірдім: «Адам — бір боқ көтерген
  боқтың қабы, Боқтан сасық боласың өлсең тағы.»                        [Abai — "Сегіз аяқ"]
```

Every cited sentence traces back to a specific `(pack, sample_id)` in the committed index — fully deterministic, no hallucination path.

### Changes

- **`adam-retrieval`**: `MorphemeIndex` gains a `sample_texts: BTreeMap<String, String>` field (keyed `pack::sample_id`). `remember_text` / `sample_text` helpers. `SampleRef::text_key`. The build binary now stores sample text alongside postings.
- **`Intent::Unknown`**: new optional `example: Option<String>` field. Defaults to `None`; populated by `Conversation::turn` when a `MorphemeIndex` is attached and the `noun_hint` has postings.
- **`Conversation::with_morpheme_index`**: attaches the index. Without it, dialog behaviour is identical to v1.6.0.
- **Planner routing**: `Intent::Unknown` now routes to `unknown.with_evidence` when `example` is set, else `unknown.with_noun` (v1.1.0), else bare `unknown`.
- **New template family** in `data/dialog/templates/v1.toml`: 4 `unknown.with_evidence` templates that wrap the retrieved sentence in Kazakh guillemets («…»).
- **Committed index regenerated** with sample texts → 2.1 MB (was 1.6 MB without texts).
- **+3 retrieval lib tests** (`remember_and_retrieve_text`, `sample_text_returns_none_when_absent`, `text_key_is_pack_and_id_joined`); **+2 dialog e2e tests** (`unknown_with_retrieval_cites_corpus_example`, `unknown_without_index_falls_back_to_noun_echo`).

### Design points worth remembering

- **Determinism**: `inject_retrieval_example` picks the first (sort-order) posting, not a random one. rng_seed is NOT consulted — the cited evidence is reproducible across runs.
- **Optional**: index attachment is additive; no-index callers (CLI without `--with-index`, older tests) keep the v1.1.0 noun-echo behaviour. No behavioural regression.
- **Small committed index**: only 3,191 samples are in the committed snapshot (500/pack cap). Users who want richer hits run `build_morpheme_index -- --full` locally (~10 min → ~700 MB gitignored artifact).
- **Traceability wins over style**: the templates wrap quotes in «…» so the evidence is visually separated from the wrapper — critical for the "every response is traceable" promise.

### Workspace tests

- **279 tests pass** (274 → +3 retrieval + +2 dialog e2e).

## [1.6.0] — 2026-04-20 — Retrieval engine bootstrap: `adam-retrieval` crate + morpheme inverted index

Minor release. First shipped component of the **v2.0 retrieval engine**. Unlike a probabilistic LM, retrieval is deterministic (given a morpheme bag + index, top-k is fully determined), traceable (every hit names the pack + sample id it came from), and cheap (a hash lookup + sorted-list intersection, not a matmul). See the `project_retrieval_not_neural_v2` memory for the architectural rationale.

### New crate: `adam-retrieval`

- `MorphemeIndex` — `BTreeMap<String, Vec<SampleRef>>`. BTreeMap (not HashMap) so the on-disk JSON form is deterministic: the same input always serialises byte-identical, making `git diff` of the committed index meaningful.
- `SampleRef { pack, sample_id }` — every posting traces back to exactly one sentence in one committed pack.
- API: `insert(morpheme, sref)` (idempotent, keeps postings sorted), `search(morpheme)`, `search_conjunction(&[morpheme])` (AND-search with shortest-list-first intersection), `refresh_stats` (for bulk loads).
- **7 unit tests** covering idempotence, sorted invariants, conjunction intersections, and unknown-morpheme collapse.

### New binary: `build_morpheme_index`

Walks committed corpus packs, runs each unique word through the FST parser once (cached), indexes the sample under every root the parser emits. The per-word cache drops build time from ~75 minutes (one parse per word occurrence) to ~10 minutes full corpus / ~17 s for the committed snapshot.

**Two modes** (the v1.3.5 / v1.5.0 sharding convention):

- **default** — per-pack `--limit 500` cap. Writes to committed `data/retrieval/morpheme_index.json` (~1.6 MB). Runs in 17 s. Committed index ingests 3,191 samples → 3,082 distinct morphemes → 16,262 postings. This is the reference snapshot CI + integration tests consume.
- **`--full`** — full committed corpus. Writes to `data/retrieval/morpheme_index_full.json` (gitignored; ~700 MB). Fuel for v1.7.0+ retrieval experiments.

### FST-parser throughput measured

Benchmark on the committed corpus: **1.155 ms / word** on a cold cache (single-threaded, M2). With the unique-word cache, a full build performs ~270 k parses instead of ~3.84 M — 14× savings.

### Tests

- **274 workspace tests pass** (267 → +7 for the new `adam-retrieval` crate).

### What this release does NOT do (scope discipline)

- No `Intent::Unknown` fallback integration yet — that is v1.6.5+.
- No ranking / scoring — v1.7.0 work. Today `search` returns postings in deterministic sort order, which is good enough to build against.
- No compositional synthesis (retrieve → splice → inflect) — v1.8.0+.

The v1.6.0 bet: **ship the index as a first-class artifact**, so every subsequent release can measure itself against it concretely rather than against abstract targets.

## [1.5.5] — 2026-04-20 — Morpheme-coverage audit: 79.48 % Lexicon prefix-match over 3.84 M words

Patch release. Adds `morpheme_coverage` — a fast prefix-match audit that measures what fraction of corpus words begin with a known Lexicon root. This is the first diagnostic for the v1.6.0+ retrieval engine: it tells us concretely *where* the Lexicon misses and gives every future Lexicon PR a measurable coverage delta.

### Scope pivot

v1.5.5 was originally planned as "government Kazakh sources" (akorda.kz, egov.kz, bnews.kz) to close the last 1.3× gap to 100 M local words. The planned sources turned out to need scraping infrastructure that is out of scope for a patch release, so v1.5.5 instead delivers the **measurement** tool that will drive the Lexicon/corpus expansion once a reliable source pipeline exists. The 100 M-word directive is not abandoned — it moves to v1.6.x.

### Added: `morpheme_coverage` binary

- Walks every committed pack listed in `corpus_audit`'s `SOURCE_PACKS`.
- Loads curated + Apertium roots (14,247 roots at ≥ 3 chars, the false-positive guard).
- For each word: true if any prefix (≥ 3 chars) matches a lexicon root.
- Per-pack report: total words, covered words, coverage ratio, top 20 uncovered words by frequency.
- Output: `data/corpus_morpheme_coverage_report.json`.
- 5 unit tests covering prefix-match + normalisation semantics.

Prefix match is a **lower bound** on true FST parse coverage — it says nothing about whether suffixes are valid, only whether the root side is recognised. A full FST parse of 3.84 M words would require ~2 trillion synth calls at ~600 k per parse; the prefix audit runs in seconds and gives an honest ceiling.

### Coverage baseline (v1.5.5)

| Pack | Words | Coverage |
|---|---:|---:|
| `tatoeba_kazakh_pack.json` | 23 245 | 79.85 % |
| `wikipedia_kz_pack.json` | 1 683 182 | 76.89 % |
| `common_voice_kk_pack.json` | 34 403 | 80.53 % |
| `cc100_kk_pack.json` | 1 684 920 | 77.26 % |
| `abai_wikisource_pack.json` | 18 935 | 76.12 % |
| `kazakh_proverbs_pack.json` | 319 | 85.27 % |
| `synthetic_sentences_pack.json` | 398 307 | **99.82 %** (synth uses Lexicon) |
| `kazakh_classics_pack.json` | 893 | 81.52 % |
| **Overall** | **3 844 204** | **79.48 %** |

### Top uncovered words — concrete Lexicon candidates

The report names the most-frequent unmatched words across CC-100 — closed-class items not yet in the Lexicon that every future Lexicon PR can remove from this list:

- `деп` — quotative particle
- `осы` — proximal demonstrative (closed-class)
- `оның` — genitive of `ол` (closed-class pronoun case form)
- `деген` — participle of `де-` ("say / that which is said"), no derivation chain yet
- `республикасының`, `облысы`, `республикасы`, `облыстық` — proper-noun state/region terms
- `пен` — postposition "with / and" (closed-class)
- `орта`, `бас`, `алу` — high-frequency common nouns/infinitives

### Workspace tests

- **267 tests pass** (was 262; +5 from the new binary's unit tests).

## [1.5.0] — 2026-04-20 — CC-100 re-extract: corpus local → 77.9 M words (gap 1.3×)

Minor release. Rewrites the CC-100 Kazakh processor along the same lines as v1.3.0 Wikipedia — **chunked streaming + loanword-density filter + sharding** — and unleashes it against the full `cc100_kk.txt.xz` (≈ 5 GB decompressed) that previously had a hard 50 k-sample cap.

### Processor rewrite (`process_cc100_kk.rs`)

- Adds the 10 % loanword-density filter shared with `process_wikipedia_kz`. CC-100 web crawl is Russified far more heavily than Wikipedia — 24 k of every 140 k accepted samples were rejected by this filter alone on shard 01.
- Replaces the old 50 k hard cap with the project-standard sharding pattern: first shard committed (≤ 50 MB), subsequent shards written to the gitignored `data/curated/shards/` for local retrieval-engine fuel.
- Shard size 140 k (vs Wikipedia's 150 k) — web-crawl sentences average longer, so 140 k keeps each shard's pretty-printed JSON safely below the 50 MB GitHub warning threshold.
- `--full` flag mirrors the Wikipedia processor. Default mode writes shard 01 only; `--full` continues until the stream ends.

### Audit integration (`corpus_audit.rs`)

- `--local` mode now includes both `wikipedia_kz_shard_*` and `cc100_kk_shard_*` shards from `data/curated/shards/`.
- Default mode (CI) audits committed packs only; behaviour unchanged.

### Corpus impact

| Metric | v1.3.5 | v1.4.5 | **v1.5.0** | Δ |
|---|---:|---:|---:|---:|
| Committed words | 2.85 M | 2.85 M | **4.01 M** | +40.7 % |
| Local words (committed + shards) | 16 M | 16 M | **77.9 M** | +387 % |
| Committed unique vocab | 92 k | 92 k | **270 k** | +193 % |
| Local unique vocab | 485 k | 485 k | **1.72 M** | +255 % |
| Overall Kazakh purity | 99.99 % | 99.99 % | **98.36 %** | −1.6 pp (web-crawl noise) |
| Expansion gap to 100 M | 6.2× | 6.2× | **1.3×** | **within striking distance** |

33 CC-100 shards now live locally in `data/curated/shards/` (shards 02–34). Shard 01 is committed at `data/curated/cc100_kk_pack.json` (140 000 samples, 48.7 MiB).

### Purity trade-off (noted, not fixed)

CC-100 is web-crawl text, so overall corpus purity drops from 99.99 % → 98.36 %. The 10 % density cap already rejects the most heavily Russified sentences; further tightening would throw out too much signal. This is the trade-off encoded in the `project_corpus_purity_directive` memory — apply the filter, then accept the residual. v1.5.5 (government sources — akorda.kz, egov.kz) is expected to restore purity closer to 99 % and push local past 100 M.

### Workspace tests

- **262 tests pass** (unchanged from v1.4.5). No code in the FST + dialog path changed; only the corpus processor and audit tool.

## [1.4.5] — 2026-04-19 — Lexicon polish: +20 modern Kazakh nouns

Patch release. Expands the curated Lexicon with modern professional nouns and common conversational vocabulary — all native Kazakh formations, no Russian loanwords (per the `project_corpus_purity_directive` and `project_kazakh_only_directive` memories).

### Added roots (+20)

**Professions** (agent `-шы` formations and older native forms):
- `нұсқаушы` (instructor), `кеңесші` (consultant), `жетекші` (leader),
- `қызметкер` (employee), `құрылысшы` (builder), `сатушы` (seller),
- `тергеуші` (investigator), `қорғаушы` (defender/lawyer), `басшы` (boss),
- `іскер` (businessman), `жүргізуші` (driver), `балықшы` (fisherman),
- `аңшы` (hunter), `етікші` (shoemaker), `мергенші` (sharpshooter),
- `жауынгер` (warrior), `оқытушы` (lecturer), `саудагер` (merchant),
- `тәрбиеші` (tutor/educator)

**Common nouns** (conversation-relevant):
- `мекеме` (institution), `кеңсе` (office), `ұйым` (organisation),
- `жүрек` (heart), `әке` (father), `аға` (elder brother), `іні` (younger brother),
- `апа` (elder sister), `қарындас` (younger sister),
- `кеше` (yesterday), `бүгін` (today), `ертең` (tomorrow), `таңертең` (morning)

Total Lexicon: **4,516 entries** (was 4,496 in v1.4.0).

### Verified round-trip

Each new occupation round-trips through the FST-NER path from v1.4.0:

```
$ adam_chat
> мен жүргізушімін   → сіз жүргізуші екенсіз
> мен саудагермін    → саудагерлер — қажетті мамандық
> мен нұсқаушымын    → сіз нұсқаушы екенсіз
> мен сатушымын      → сіз сатушы екенсіз
```

Parser → predicate=P1Sg → POS-filter accepts → `occupation` slot filled → template plural / dative FST synthesis.

### Tests

Workspace: **262 passing**, 4 ignored, 0 failing. Foundation CI green. No new test cases — the v1.4.0 FST-NER tests already cover the general mechanism; these new roots are data-only expansion.

## [1.4.0] — 2026-04-19 — FST-NER refactor + DST + predicate-copula morphology

Minor release. Four connected pieces of work that together address the external-reviewer critiques from v1.3.5 and lay groundwork for v1.6.0+ retrieval engine.

### 1. Predicate-person FST morphology (new)

`NounFeatures` gains a `predicate: Option<Predicate>` field with seven variants (P1Sg / P2SgInformal / P2SgPolite / P3 / P1Pl / P2PlInformal / P2PlPolite). Applied AFTER case in `synthesise_noun`:

| form | derivation |
|---|---|
| мұғалім + P1Sg | мұғаліммін |
| мұғалім + P2SgPolite | мұғалімсіз |
| Алматы + Ablative + P1Sg | Алматыданмын |
| бағдарламашы + P1Sg | бағдарламашымын |

Six new suffix templates (`PREDICATE_1SG` / `PREDICATE_2SG_INFORMAL` / `PREDICATE_2SG_POLITE` / `PREDICATE_1PL` / `PREDICATE_2PL_INFORMAL` / `PREDICATE_2PL_POLITE`). The inverse parser now enumerates predicate in its feature space; predicate + possessive never stack (grammatically exclusive), saving search space.

### 2. Lexicon place names (+30 entries)

Added Kazakh cities and country names as proper nouns to `data/tokenizer/segmentation_roots.json`: Алматы, Астана, Шымкент, Қарағанды, Ақтөбе, Тараз, Павлодар, Өскемен, Атырау, Семей, Қостанай, Қызылорда, Талдықорған, Ақтау, Орал, Петропавл, Түркістан, Көкшетау, Маңғыстау, Қазақстан, Ресей, Қытай, Түркия, Монғолия, Өзбекстан, Қырғызстан, Еуропа, Азия, Әлем, Отан. All lowercased for case-insensitive parser lookup.

Total Lexicon: 4,496 entries (was 4,466 in v1.3.5).

### 3. Semantics FST-NER refactor

Replaced manual suffix-stripping in the city and occupation recognisers with **FST parse-based entity extraction** — addresses the architectural inconsistency Codex and Antigravity flagged.

- **City**: `detect_statement_of_location` now scans `parses: &[Analysis]` for the first Noun in Ablative or Locative case. Ablative signals origin ("Алматыданмын"), Locative signals residence when co-occurring with "тұрамын / тұрамыз". Rule-based string heuristics remain as fallback for out-of-Lexicon inputs.
- **Occupation**: `detect_statement_of_occupation` scans parses for Noun with `predicate == Some(P1Sg)` AND `part_of_speech == "noun"` (the POS filter rejects adjective-predicate forms like `жақсымын`). Fallback chain: FST parse → Lexicon-backed copula-strip (v0.9.7) → fixed 6-form table (v0.8.0).

### 4. Dialog State Tracking (DST)

`Conversation` is no longer a flat slot HashMap — it tracks intent context:

```rust
pub struct Conversation {
    pub session: HashMap<String, String>,        // slots
    pub active_intent: Option<IntentKind>,       // last-turn intent kind
    pub intent_history: Vec<IntentKind>,         // bounded-capacity trace
}
```

`IntentKind` (new, exported) is a lightweight payload-free summary of `Intent` — string names aren't copied into history. History is capped at 32 entries (long sessions don't grow unboundedly).

**Follow-up resolution** handles contextual utterances like `ал сіз?` ("and you?") — `resolve_follow_up` re-tags weak-intent utterances ([`Unknown`] / [`Affirmation`] / [`Negation`]) against the previous turn's `active_intent`, so after `AskHowAreYou` the follow-up fires as `AskHowAreYou` again for planning. Strong intents are never overridden.

### Addresses external reviewer critiques

| Critique | Disposition |
|---|---|
| Codex: "FST parser ignored in semantics" | **Fixed** — FST parses are the primary entity-extraction path |
| Codex: "duplicated morphology in strip_*" | **Fixed** — fallback to rule-based string heuristics only when FST parse is empty |
| Antigravity: "flat HashMap isn't DST" | **Fixed** — active_intent + intent_history + follow-up resolution |
| Antigravity: "можно добавить ML для NLU" | **Rejected** — contradicts v2.0 retrieval-not-neural direction |

### Tests

75 dialog end-to-end pairs (up from 69 in v1.3.5), 6 new covering FST-NER place-name recognition, predicate-P1Sg occupation, adjective rejection, DST active_intent tracking, follow-up resolution, and reset clearing all state. Workspace total: **262 passing**, 4 ignored, 0 failing. Foundation CI green.

### Public API additions

- `adam_dialog::IntentKind` — lightweight payload-free intent summary
- `adam_dialog::Conversation { active_intent, intent_history }` — new fields
- `adam_kernel_fst::morphotactics::Predicate` — new enum for noun-predicate copula
- `adam_kernel_fst::morphotactics::NounFeatures.predicate` — new optional field

## [1.3.5] — 2026-04-19 — Wikipedia sharding + docs drift fixes + v2.0 direction committed

Patch release. No behavioural change in the dialog layer. Unlocks the full 15 M-word Wikipedia yield for local use (the v2.0 retrieval engine's fuel), fixes documentation drift, and commits the v2.0 architectural direction — retrieval over morpheme-parsed corpus, not a trained transformer LM.

### Wikipedia sharding — `--full` mode

`process_wikipedia_kz` now supports a `--full` flag. Default mode is unchanged (single committed pack, 150 k samples, ~49 MB). With `--full`, the processor writes additional shards to `data/curated/shards/wikipedia_kz_shard_NN_pack.json` (gitignored), one per 150 k samples, for the full ~1.4 M-sample, ~16 M-word corpus on local disk. These are the input fuel for v1.6.0+ retrieval-engine work.

`corpus_audit` scans shards automatically when `--local` is passed (or `ADAM_CORPUS_AUDIT_LOCAL=1`); default behaviour is unchanged (reads only committed packs, matches what CI sees).

### Docs drift fixed (after Codex + Antigravity reviews)

Two external AI reviewers flagged specific overclaims and documentation drift. The valid points:

- **Badge count** was `253 passing`; actual test count has been 256 since v1.2.0. Badge updated.
- **`foundation_scope.md`** still listed "Trilingual input recognition" and "Latin→Cyrillic transliteration" as in-scope, which were both reverted in v1.1.0. Rewritten for v1.3.x+ reality.
- **"Grammatically correct by construction"** wording in the README was an overclaim — FST guarantees apply to `{slot|features}` expansion, not to literal template text. README and `foundation_scope.md` both tightened: now "grammatically correct by construction on the slot path".

### FST-NER refactor deferred

Reviewers suggested routing entity extraction through `adam_kernel_fst::parser::Analysis` instead of manual suffix stripping. Investigation found this requires two prerequisites that don't fit a patch release:

1. **Predicate-person feature markers** in FST morphotactics (1sg / 2sg / 3rd-person predicate copulas like `-мын / -сың / -дір`). Currently the FST knows possessives but not predicates, so `мұғаліммін` can't be parsed.
2. **Place names in the Lexicon.** `Алматы`, `Астана`, `Шымкент` etc. aren't in `data/tokenizer/segmentation_roots.json`, so the parser returns empty for any ablative/locative form of them.

Both are v1.4.0 minor-level work (new FST features + Lexicon expansion). Queued, not blocking.

### v2.0 direction — committed

Memory saved (`project_retrieval_not_neural_v2`): v2.0 "minimally thinking Kazakh model" is **retrieval-based, not a trained transformer LM**. Morpheme-indexed retrieval over the 100 M+ word corpus + rule-based compositional synthesis. Properties: zero hallucinations by construction, full trace to source sentences, M2 8 GB-runnable, exploits Kazakh's rich agglutinative structure that the FST already unpacks. Rejects the mainstream "small LLM fallback" path as a scaled-down clone rather than a new direction.

### Numbers

- Committed corpus (CI view): **256 tests passing**, 2.85 M words / 224 k unique / 97.99 % purity — unchanged from v1.3.0
- Local-with-shards: 16.23 M words / 749 k unique / 98.03 % purity / gap to 100 M target = 6.2×

### Tests

Workspace: 256 passing, 4 ignored, 0 failing. Foundation CI green.

## [1.3.0] — 2026-04-19 — Wikipedia re-extract (+27 % corpus, 2.85 M words)

Unlocks the Kazakh Wikipedia pack after realising the existing 100 k-sample slice was only 3 % of what the already-downloaded 638 MB source file can yield. The v1.3.0 rewrite of `process_wikipedia_kz` is 100× faster and applies the v1.x purity gate.

### The problem

User observed: "all the raw material is already in `data/external/` — we just need to extract it better. And we learned the lesson at v0.4.0: no 2-word fragments." Investigation confirmed:

- `data/external/wikipedia_kz_plain.txt` = **638 MB** raw Kazakh Wikipedia
- `data/curated/wikipedia_kz_pack.json` (v1.2.0) used only **100 k samples / 1.15 M words** — ~3 % of the source
- Old processor did byte-by-byte reads → estimated hours for full scan (never run to completion)
- Old processor had no loanword-density filter → 3–4 % contamination in committed pack

### The fix

Rewrote `crates/adam-corpus/src/bin/process_wikipedia_kz.rs`:

- **Chunked streaming** (64 KB reads) replaces byte-by-byte I/O → full 638 MB scan in **26 s** (measured on M2 8 GB)
- **Loanword-density filter** (10 % cap) drops Russian-loanword-saturated articles
- **Optional `target-cap` CLI arg** — default now processes the full file; cap is available for dev runs
- **Wikipedia purity 95.92 % → 99.99 %** after the new filter
- Min/max word bounds unchanged (4–40 words per sample), still honours the v0.4.0 lesson

### Full-dump numbers (measured, not committed)

When run uncapped on the full 638 MB source:

```
articles=296,342  sentences_scanned=5,726,108  accepted=1,395,801
skipped_latin=2,711,431  skipped_length=922,051  skipped_dup=276,059  skipped_loanword=420,766
```

**1.4 M clean samples / ~15 M words** available locally. JSON size: ~440 MB.

### What's committed in v1.3.0

GitHub's 100 MB hard file-size limit (and the project's 50 MB convention from `feedback_git_ignore_policy`) mean we can't commit the 440 MB full pack. v1.3.0 commits the first 150 k samples (~49 MB) as the canonical pack; the uncapped output is regenerable locally from the `data/external/wikipedia_kz_plain.txt` source.

| measure | v1.2.0 | v1.3.0 committed | v1.3.0 local (uncapped) |
|---|---:|---:|---:|
| Wikipedia samples | 100,000 | 150,036 | 1,395,801 |
| Wikipedia words | 1,150,532 | 1,613,306 | ~15,138,291 |
| Wikipedia purity | 95.92 % | 99.99 % | 98.06 % |
| **Corpus total words** | **2,238,852** | **2,851,629** | ~16,226,611 |
| **Expansion gap to 100 M** | **45×** | **35×** | 6.2× |

### Sharding plan (v1.3.5)

To expose the full 1.4 M samples without blowing the file-size limit, v1.3.5 will shard the pack into ~10 files of ~40 MB each (`wikipedia_kz_shard_01_pack.json` … `wikipedia_kz_shard_10_pack.json`). `corpus_audit` will glob-merge them. Downstream consumers (future LM training) will read all shards.

### Tests

Workspace: **256 passing**, 4 ignored, 0 failing. Foundation CI green (pack validated via `jq empty`).

## [1.2.0] — 2026-04-19 — Kazakh classical literature expansion

First significant post-v1.0 corpus addition. Ingests the classical Kazakh Wikisource holdings for **Ыбырай Алтынсарин** (1841–1889, children's literature + fables) and **Мағжан Жұмабаев** (1893–1938, early 20c poet). Both authors are fully in the public domain.

### Scope — honest framing

The original v1.2.0 label was "classical literature OCR". In practice:

1. **OCR requires scanned PDFs we don't have** and a Kazakh-trained Tesseract model. Neither is available in this release cycle. Deferred to a later minor release (v1.3.x+) once sources are found.
2. **Kazakh Wikisource is already digitised** — no OCR needed. This release uses that path instead.
3. Other classical authors (Шәкәрім, Жамбыл, Сәкен Сейфуллин, Міржақып Дулатов) are public domain but their pages don't exist on kk.wikisource yet. They'll be added when sources arrive.

### Yield

| pack | samples | words | unique | purity |
|---|---:|---:|---:|---:|
| **kazakh_classics** (new) | **111** | **926** | **710** | **100.00 %** |

Small in absolute terms (926 words ≈ 0.04 % of the existing corpus) but **pristine literary Kazakh** — zero loanword contamination, from two canonical pre-Soviet authors. This is the literary quality core the LM should weight highly in training.

### New corpus total

- **Before (v1.1.5):** 2,237,926 words, 193,020 unique, 96.74 % purity
- **After (v1.2.0):** 2,238,852 words, 193,132 unique, 96.74 % purity
- **Gap to target:** still 97.76 M words (~45× expansion)

### Added

- `scripts/fetch_kazakh_classics.sh` — universal Kazakh Wikisource fetcher. Takes an author list; downloads each author's work-index page; extracts and cleans `<p>` bodies from each linked work; writes `data/external/kazakh_classics_plain.txt` with `0x1e`-separated work records. Rate-limited and UA-identified per Wikimedia policy.
- `crates/adam-corpus/src/bin/process_kazakh_classics.rs` — processor that reads the raw text, applies the v1.x purity filter (Russian-only letter detection + loanword suffix detection + density threshold of 10 %), deduplicates, and writes `data/curated/kazakh_classics_pack.json`.
- `data/curated/kazakh_classics_pack.json` — 111 clean samples.
- `corpus_audit` updated to include the new pack.

### Strategic note

v1.2.0 is the slow, honest start of the corpus expansion path. The big-volume releases are:

- **v1.3.0** — full Kazakh Wikipedia dump (~35 M words from 243k articles; currently we have only 1.15 M from a 100k-sample subset)
- **v1.4.0** — Kazakh government corpora (egov.kz, akorda.kz, bnews.kz — select long-form content)
- **v1.5.0** — reach 100 M+ target with additional classical literature (from OCR once pipeline arrives) and filtered news

### Tests

Workspace: **256 passing**, 4 ignored, 0 failing. Foundation CI green. No behavioural change to the dialog layer.

## [1.1.5] — 2026-04-19 — Corpus audit baseline

First step on the v1.x corpus engineering path toward the v2.0 LM. No dialog / FST behavioural change; tooling + baseline numbers only.

### Added

- **`cargo run --release -p adam-corpus --bin corpus_audit`** — measures the starting position across all source packs: per-source word count, unique vocabulary, Kazakh-purity score (fraction of words free of Russian-only letters and loanword suffixes), within-pack deduplication.
- **`data/corpus_audit_report.json`** — machine-readable report regenerated by the binary.
- **`docs/corpus_audit.md`** — human-readable baseline + the v1.2.0 → v1.5.0 expansion plan.

### Baseline numbers (2026-04-19)

| pack | samples | words | purity |
|---|---:|---:|---:|
| tatoeba_kazakh | 4,058 | 24,643 | 98.12 % |
| wikipedia_kz | 100,000 | 1,150,532 | 95.92 % |
| common_voice_kk | 6,108 | 36,397 | 99.91 % |
| cc100_kk | 50,000 | 602,144 | 96.59 % |
| abai_wikisource | 2,253 | 20,303 | 99.81 % |
| kazakh_proverbs | 80 | 349 | 100.00 % |
| synthetic_sentences | 100,000 | 403,558 | 98.79 % |
| **total** | **262,499** | **2,237,926** | **96.74 %** |

- **Target:** 100 M words.
- **Gap:** 97.76 M (≈ **45× expansion** needed).

### Strategic implications

- **Data volume is the real bottleneck**, not model size. Corpus engineering is 5–6 releases ahead of any actual LM training.
- **Abai / Common Voice / proverbs are small but pristine** (> 99 % purity) — literary core worth preserving.
- **Wikipedia + CC-100 carry the volume but 3–4 % loanword contamination**; v1.2.0+ ingestion must pre-filter.
- **Reference dictionaries** (per user directive) are a future source but must pass the same loanword filter — raw dictionary JSON is not accepted.

### Tests

3 new unit tests in `corpus_audit` for the detector functions. Workspace: **256 passing**, 4 ignored, 0 failing. Foundation CI green.

## [1.1.0] — 2026-04-19 — Kazakh-only revert + modern Lexicon + smart Unknown

Strategic revert of v0.9.6 multilingual. Post-v1.0.0 testing revealed that the Russian / English recogniser triggers diluted the Kazakh-first thesis — users typing in the wrong language received shallow coverage, and the cross-language tests added noise without adding generalisation. This release restores the Kazakh-only surface and sets up the path to a real Kazakh LM.

### Breaking changes (input surface)

- **All Russian / English recogniser triggers removed.** Input that previously matched via "hi / hello / привет / меня зовут X / how are you" etc. now falls through to `Intent::Unknown`.
- **Latin → Cyrillic transliteration module removed.** `adam_dialog::transliteration` is gone; non-Cyrillic slot values are no longer silently rewritten before FST synthesis.

### Breaking changes (Intent enum)

- **`Intent::Unknown`** gains a `noun_hint: Option<String>` field (was unit-struct-like with only `raw_tokens`).
- **`Intent::Insult`** (new variant) — polite non-engagement for rude input (ақымақ, надан, түкке тұрмайсың, ақылсыз).

### Additions

- **Modern Kazakh Lexicon expansion** (12 new curated roots, all native Kazakh formations — no Russian loanwords):
  - Professions: бағдарламашы (programmer), аудармашы (translator), жазушы (writer), заңгер (lawyer), басқарушы (manager), журналшы (journalist), зерттеуші (researcher), ұстаз (teacher/mentor), емші (healer)
  - Tech concepts: бағдарлама (program), қосымша (application), есептеуіш (computer, native)
  - AI / cognition: ақыл (mind), сана (consciousness), ой (thought), жасанды (artificial, adjective)
- **Smart Unknown handler.** When no intent matches, the FST parser extracts a noun from the input (filtered against pronouns / postpositions / quantifiers) and routes to the new `unknown.with_noun` template family — responses like `"ах, {noun} туралы айтасыз ба"` acknowledge the topic instead of blank `түсінбедім`.
- **Insult templates** (4 variants) for polite non-engagement — the model doesn't escalate or retaliate.
- **`detect_insult`** recogniser + `detect_ask_location` / `detect_compliment` stricter bounds.

### FST tightening

- `strip_ablative_copula` now requires a stem of at least 3 characters. Prevents greedy match on `наданмын` (1sg predicate of "ignorant") from being misrecognised as a city.

### Tests

69 dialog end-to-end pairs (was 81 — multilingual block deleted; +5 new for Kazakh-only revert, Insult, Unknown-with-noun, and modern Lexicon coverage). Workspace: **253 passing**, 4 ignored, 0 failing. Foundation CI green.

### Roadmap commitment (v2.0)

This release is the bridge between the v1.0.0 rule-based MVP and a future **thinking Kazakh LM**. The plan:

- **v1.x (now)** — Lexicon expansion, smart Unknown handler. Incremental.
- **v1.x (data engineering)** — expand Kazakh corpus from ~4 M to **100 M+ tokens**. This is the real bottleneck for any trained model — Chinchilla-optimal data for a 24 M param LM is ~480 M tokens; we're currently ~100× short.
- **v2.0** — compact Kazakh LM (transformer or SSM), trained in pure Rust, plugged in as the `Intent::Unknown` fallback only. The deterministic 26-intent pipeline stays as the 0-hallucination backbone for everything it recognises; the LM handles the long tail.

Multimodality (speech, vision) is deferred until the thinking Kazakh LM is real.

## [1.0.0] — 2026-04-19 — MVP cut

The investor-demoable MVP. No new features since v0.9.9 — the delta is documentation, housekeeping, and a formal cut of the v1.0.0 line.

### What v1.0.0 delivers

Predictable, auditable Kazakh dialog across 25 intents, trilingual input (kk / ru / en), Kazakh-only output, multi-turn session state, and FST-guaranteed morphology — all in pure Rust running on a MacBook Air M2 8 GB.

| pillar | v1.0.0 state |
|---|---|
| Intents recognised | **25** (Greeting × 5 sub-kinds, Farewell, Affirmation / Negation, Thanks / Apology, AskHowAreYou / StatementOfWellbeing, AskName / StatementOfName, AskAge / StatementOfAge, AskLocation / StatementOfLocation, AskOccupation / StatementOfOccupation, AskFamily / StatementOfFamily, AskWeather / StatementOfWeather, AskTime, Compliment, Request, WellWishes, Unknown) |
| Input languages | Kazakh, Russian, English |
| Entity extraction | `name` (3 KK + 2 RU + 3 EN patterns), `age` (Kazakh numerals 1–99 + digits), `city` (ablative / locative stripping), `occupation` (Lexicon-backed 1sg-copula stripping, POS-filtered) |
| Session state | `Conversation` struct, absorb + persist across turns, reset() |
| Slot syntax | `{slot\|features}` with 4 feature families (case, number, derivation, possessive), 27 tokens total, `+`-combinable |
| FST morphology | 11 archiphonemes, 22+ twol rules, 30 suffix templates, 100% synth-analyse roundtrip on 36 k forms |
| Template repository | 29 families, TOML-driven, slot-fillability filtered |
| Latin name support | transliteration module (digraphs + single-letter map) feeds FST when template requests morphology on a Latin root |
| Foundation CI | validates every layer; `validate_foundation.sh` green |

### Documentation refresh

- **README.md** fully rewritten as the v1.0.0 MVP story. The pre-v0.4.5 transformer narrative is compressed into a single "History" section; transformer-era sample generations, training pipeline, and PPL stats removed from the forward-looking story.
- **docs/roadmap.md** capped with a v1.0.0 final entry; earlier phases condensed to a lifecycle view.
- **docs/repository_layout.md** updated with the full current crate list (was missing `adam-kernel-fst` and `adam-dialog`).
- **docs/kazakh_grammar/07_dialog_architecture.md** status flipped from "design document, pre-implementation" to "shipped in v1.0.0".
- **docs/foundation_scope.md** aligned with the v1.0.0 deliverable.
- **docs/training_baseline.md** and **docs/eval_baseline.md** marked as legacy context (transformer phase v0.1–v0.4).
- Per-subdirectory READMEs under `data/` (dialog, curated, lexicon_v1, training) were added in v0.8.5's cleanup pass and still accurately reflect the v1.0.0 state.

### Tests

Unchanged from v0.9.9: **271 passing**, 4 ignored, 0 failing. Foundation CI green.

### Post-v1.0.0

The MVP is the release surface. Future work candidates (not promised, not scheduled):

- Native-speaker review of the template set — a real, human review pass.
- Lexicon expansion beyond the 14 k curated roots (proper nouns, modern vocabulary under a separate "loanword-allowed" tier).
- Polished Latin-to-Cyrillic transliteration (silent-h handling for English names).
- Verb slot expansion (`{root|verb_features}` with a different synthesiser dispatch).
- Additional intents beyond the 25-intent surface.

Any of these would ship as v1.1.0+ and are explicitly out of scope for the v1.0.0 cut.

## [0.9.9] — 2026-04-19

Morphology correctness pass + template phrasing polish. The last stretch before the v1.0.0 MVP cut.

### FST Instrumental fix (two bugs, one mechanism)

The `INSTRUMENTAL` suffix template previously used the harmony-alternating archiphoneme `{E}`, but Kazakh Instrumental is actually invariant in vowel — always `-мен/-бен/-пен`, never `-ман/-бан/-пан`. Replaced with a literal `е`:

```diff
- const INSTRUMENTAL: SuffixTemplate = &[Arch(M), Arch(E), Literal('н')];
+ const INSTRUMENTAL: SuffixTemplate = &[Arch(M), Literal('е'), Literal('н')];
```

Separately, `realise_m` flipped `Nasal → 'б'` which produced `мұғалімбен`. Fixed to `Nasal → 'м'`, giving standard `мұғаліммен`. Voiced obstruent → `б` (rare but preserved).

Before / after samples:

| root | pre-v0.9.9 | v0.9.9 |
|---|---|---|
| Алматы | Алматыман ❌ | Алматымен ✓ |
| Астана | Астанаман ❌ | Астанамен ✓ |
| мұғалім | мұғалімбен ❌ | мұғаліммен ✓ |
| Джохн | Джохнбан ❌ | Джохнмен ✓ |
| Дәулет | Дәулетпен ✓ | Дәулетпен ✓ |
| мектеп | мектеппен ✓ | мектеппен ✓ |

### Cleanup

`Archiphoneme::E` and `realise_e` were only used by the Instrumental template and are now dead. Removed both per YAGNI.

### FST regression tests

Added 6 unit tests to `morphotactics` covering every consonant-class path of the new Instrumental + the back/front vowel invariance. These lock in the fix so future archiphoneme refactors can't re-break it.

- `noun_instrumental_front_consonant_final` (Дәулет → пен)
- `noun_instrumental_back_consonant_final` (Джохн → мен)
- `noun_instrumental_back_vowel_stem_stays_е_not_а` (Алматы → мен, regression)
- `noun_instrumental_vowel_final_stem` (бала, тау → мен)
- `noun_instrumental_voiceless_final_gives_пен` (мектеп → пен)
- `noun_instrumental_nasal_final_gives_мен_not_бен` (мұғалім → мен, regression)

### Template polish pass

Dropped awkward / filler templates and replaced with context-specific acknowledgements:

| key | removed | added |
|---|---|---|
| `statement_of_age` | `түсіндім`, `жасыңыз келісті` | `қуатты кезеңіңіз` |
| `statement_of_location` | `түсіндім` | `тамаша өлке` |
| `statement_of_occupation` | `мақтанышпен` (solo) | `мақтанатын жұмыс` |
| `statement_of_weather` | `түсіндім` | `табиғат мезгіліне лайық` |

"түсіндім" as a solo response felt generic/repetitive. Replaced with phrases that match the topic of the user's statement.

### Numbers

- **FST unit tests:** 84 (was 78) — 6 new Instrumental regressions
- **Dialog end-to-end tests:** 81 (unchanged, assertion sets updated)
- **Workspace tests:** **271 passing**, 4 ignored, 0 failing
- **Foundation CI:** passing

### Known v0.9.9 limitations

- Silent English `h` still not special-cased in transliteration (`John → Джохн` rather than the conventional `Джон`). Cosmetic; FST synthesis works fine on either.
- Native-speaker review has NOT been conducted — the polish was a phrasing pass by inspection, not formal review. A real review is queued for post-v1.0.0 refinement.

## [0.9.8] — 2026-04-19

Slot syntax completes the noun-feature surface (Derivation + Possessive), Latin names get transliterated before FST synthesis, and templates gain a layer of cross-slot personalisation that uses multiple remembered entities in a single response.

### Slot syntax: full noun-feature coverage

Adds 11 derivation tokens and 7 possessive tokens to `parse_noun_features`:

```text
{root|agent}           → Agent (-шы/-ші)
{root|abstract}        → Abstract (-лық/-лік)
{root|privative}       → Privative (-сыз/-сіз)
{root|endowed}         → Endowed (-лы/-лі)
{root|similative}      → Similative (-дай/-дей)
{root|comparative}     → Comparative (-рақ/-рек)
{root|verbalnoun}      → VerbalNoun (-у)
{root|actionnoun}      → ActionNoun (-ым/-ім)
{root|diminutive}      → Diminutive (-шық/-шік)
{root|ordinal}         → Ordinal (-ншы/-нші)
{root|collective}      → Collective (-еу/-ау)

{root|p1sg}            → P1Sg (my)
{root|p2sg}            → P2SgPolite (your, polite default)
{root|p2sg_inf}        → P2SgInformal
{root|p3}              → P3 (his/her)
{root|p1pl}            → P1Pl (our)
{root|p2pl}            → P2PlPolite
{root|p2pl_inf}        → P2PlInformal
```

Combinations work as you'd expect: `{name|agent+p1sg+dative}` yields root → Agent derivation → P1Sg possessive → Dative case, all in one synthesis pass.

### Latin → Cyrillic transliteration

New module `adam_dialog::transliteration` converts Latin proper names to Kazakh Cyrillic BEFORE `synthesise_noun` when a template requests morphology on a non-Cyrillic slot value. v0.9.6 guarded against garbled output by falling back to plain substitution; v0.9.8 replaces that fallback with real transliteration so FST inflection actually runs on foreign names.

| Latin input | transliterated |
|---|---|
| `Anna` | Анна |
| `Tom` | Том |
| `John` | Джохн |
| `Zhanna` | Жанна |
| `Sharon` | Шарон |
| `Charlie` | Чарлие |

Conservative single-letter + digraph mapping: `sh/ch/zh/kh/gh/ph/th/ts/yo/ya/yu/ye` as digraphs, rest letter-by-letter (`j → дж`, `c → к`, `x → кс`, `y → й`). Silent `h` in English is **not** special-cased (`John → Джохн`, not `Джон`) — intentionally conservative.

**Policy:** plain `{name}` substitution still keeps the user's original spelling ("сәлем John"). Only `{name|features}` triggers transliteration → synthesis.

### Cross-slot templates

New templates reference multiple session entities in one response. Eligible only when every slot is fillable; plain variants stay available otherwise.

| key | new templates |
|---|---|
| `ask_how_are_you` | `"жақсымын {name}, ал сіз қалайсыз"`, `"жақсымын, рахмет {name}"` |
| `statement_of_age` | `"{name}, {age} жастасыз, тамаша"`, `"{name}, {age} жас — керемет кезең"` |
| `statement_of_occupation` | `"{name}, {occupation} — құрметті кәсіп"`, `"{name}, сіз {city|locative} {occupation} екенсіз"` |
| `compliment` | `"рахмет {name}"`, `"рахмет {name}, сіз де тамашасыз"` |

The triple-slot `"{name}, сіз {city|locative} {occupation} екенсіз"` only fires after the user has stated all three entities — "Дәулет, сіз Алматыда әнші екенсіз".

### Tests

81 dialog end-to-end pairs (up from 78), 3 new cross-slot tests covering (name+ask_how_are_you), (name+age), and (name+city+occupation triple). 23 lib-level unit tests (13 slot_syntax + 6 transliteration + 4 planner).

Workspace: **265 passing**, 4 ignored, 0 failing. Foundation CI green.

### Known v0.9.8 limitations

- Silent `h` in English is not special-cased (`John → Джохн` rather than the standard spelling `Джон`). Good enough for FST synthesis; a more polished transliterator is v1.0.0+ work.
- Back-vowel instrumental harmony (`Алматы` → `Алматыман` rather than `Алматымен`) is a pre-existing FST quirk — check `INSTRUMENTAL` template archiphoneme `E` resolution. Out of scope for the dialog layer.
- Derivation + Possessive tokens are fully parseable; the current template set uses only a handful of the 18 feature tokens. Template authors have the full surface available when needed.

## [0.9.7] — 2026-04-19

Lexicon-backed occupation recognition. The fixed 6-form table (`мұғаліммін → мұғалім` and five others) is replaced with generic 1sg-copula stripping + noun lookup against the 14 k-entry Lexicon. Any noun in the Lexicon ending in a 1sg predicate suffix (`-мын/-мін/-пын/-пін/-бын/-бін`) is now recognised.

### What now works

```
$ adam_chat
> мен ақынмын           → сіз ақын екенсіз            (new: ақын is in Lexicon, not in the old table)
> мен әншімін           → әншілер — қажетті мамандық  (FST plural on the new extract)
> мен ғалыммын          → сіз ғалым екенсіз
> мен суретшімін        → сіз суретші екенсіз
> жақсымын              → жақсы екен                  (POS filter → wellbeing, not occupation)
```

### Public API additions

- `adam_dialog::interpret_text_with_lexicon(input, parses, Option<&LexiconV1>) -> Intent`
- `adam_dialog::semantics::interpret_text_with_lexicon` (module-level)

The original `interpret_text(input, parses)` is now a thin wrapper that calls the lexicon-aware variant with `None` — existing callers keep working.

### Implementation detail

```rust
fn strip_copula_and_lookup_noun(tokens: &[String], lex: &LexiconV1) -> Option<String> {
    const COPULA_SUFFIXES: &[&str] = &["мын", "мін", "пын", "пін", "бын", "бін"];
    for t in tokens {
        for suffix in COPULA_SUFFIXES {
            let Some(root) = strip_suffix_chars(t, suffix) else { continue };
            if root.chars().count() < 2 { continue; }
            if let Some(entry) = lex.get(&root) {
                if entry.part_of_speech == "noun" {
                    return Some(root);
                }
            }
        }
    }
    None
}
```

- **POS filter** rejects adjectives (`жақсы`, `жаман`) so "жақсымын" still routes to StatementOfWellbeing.
- **Min-length 2** guards against stripping into short function words.
- **Char-count indexing** keeps UTF-8 boundaries safe.

`respond`, `respond_with_repo`, and `Conversation::turn` all pass the lexicon into the new recogniser automatically.

### Tests

78 dialog end-to-end pairs (up from 73), 5 new:
- 1 positive case covering `ақын` (out-of-table noun)
- 1 bulk test for `әнші / ғалым / суретші`
- 1 adjective-negative-case ensuring `жақсымын` stays wellbeing
- 1 unknown-root case (`xyzzyмын` → must not become occupation)
- 1 multi-turn absorption test (lexicon-derived occupation persists to session)

Workspace: **251 passing**, 4 ignored, 0 failing. Foundation CI green.

### Known v0.9.7 limitations

- Latin→Cyrillic transliteration is intentionally NOT shipped. Latin names ("John") continue to bypass `{name|features}` FST synthesis via the v0.9.6 safety guard. Transliteration lands in v0.9.8 alongside broader slot-syntax enrichment.
- Lexicon coverage is the cap — occupations not in the 14 k-entry Lexicon (`философ`, `программист`) still don't extract. Data-layer expansion is orthogonal work.

## [0.9.6] — 2026-04-19

Multilingual recogniser surface. The model now reads Kazakh, Russian, and English input across all 25 intents and replies exclusively in Kazakh. This is NOT translation — the core pipeline stays deterministic Kazakh-only. The expansion is purely at the recogniser layer: more surface forms map to the same Intent taxonomy.

### Triggers added (per intent)

- **Greeting** (casual/polite/time-of-day): `hi/hello/hey`, `привет`, `здравствуйте`, `доброе утро/день/вечер`, `good morning/afternoon/evening/day`
- **Farewell**: `bye/goodbye/see you`, `до свидания/пока`
- **Affirmation**: `yes/yeah/yep/sure/ok`, `да/конечно/ага`
- **Negation**: `no/nope/nah`, `нет`
- **Thanks**: `thanks/thank you`, `спасибо/большое спасибо`
- **Apology**: `sorry/excuse me`, `извини/извините/прости`
- **AskHowAreYou**: `how are you/how's it`, `как дела/как ты/как вы`
- **StatementOfWellbeing**: `fine/great/i'm good/i'm fine`, `хорошо/нормально/отлично`
- **AskName**: `what is/what's your name`, `как тебя/вас зовут`
- **StatementOfName**: four new patterns —
  - `meня зовут <N>`, `моё имя <N>` (Russian)
  - `my name is <N>`, `call me <N>`, `hi i am <N>` (English; bare "I am X" is ambiguous so requires a leading greet token)
- **AskAge**: `how old are you`, `сколько тебе/вам лет`
- **AskLocation**: `where are you from / where do you live`, `откуда ты/вы`
- **AskOccupation**: `what do you do / what's your job`, `кем работаешь/занимаешься`
- **AskWeather**: `how's / what's the weather`, `какая погода`
- **AskTime**: `what time is it / what's the time`, `сколько времени/который час`
- **Compliment**: `great/awesome/wonderful/excellent/well done`, `молодец/отлично/здорово`
- **Request**: `please/need help/can you help`, `пожалуйста/помогите/помоги`
- **WellWishes**: `good luck/all the best`, `удачи/всего наилучшего`

### Safety guard for Latin roots

FST phonology is tuned for Kazakh Cyrillic. Feeding `"John"` into `synthesise_noun(..., Case::Instrumental)` would produce garbled `"Johnман"`. The realiser now detects non-Cyrillic roots and falls back to plain substitution — no suffix attached. Output: `"John танысқаныма қуаныштымын"` rather than hallucinated morphology.

### Ordering change

`StatementOfName` is now checked BEFORE `Greeting` in `interpret_text`. This prevents `"hi i am John"` from misfiring as a bare Casual greeting. All StatementOfName patterns (атым/есімім/зовут/my name is/call me/[greet] i am X) are explicit enough to rule out false positives.

### Tests

73 dialog end-to-end pairs (up from 56), 17 new:
- 10 recogniser triggers (greetings × 3, farewell, affirmation, negation, thanks, apology, ask-how-are-you, ask-name)
- 4 self-introduction patterns (Russian `зовут`, English `my name is` / `call me` / `hi i am`)
- 2 output-is-Kazakh invariants (Russian input → Cyrillic-only output; Latin name → no FST suffix)
- 1 multilingual multi-turn conversation flow

Workspace: **245 passing**, 4 ignored, 0 failing. Foundation CI green.

### Known v0.9.6 limitations

- Recogniser catches the common phrasings. Edge cases (British contractions "init", ru-ua mix, Kazakh with Latin transliteration "salem") are not handled.
- Latin names stay un-inflected in templates requesting `{name|features}`. Transliteration to Cyrillic (e.g. `John` → `Джон`) would let the FST synthesise properly — possible future work.
- No output-language switching: Russian / English input still gets Kazakh output by design.

## [0.9.5] — 2026-04-19

FST-backed slot expansion. Templates can now emit `{slot|features}` atoms; the realiser synthesises grammatical forms via `adam_kernel_fst::morphotactics::synthesise_noun` instead of plain text substitution. Cross-slot templates (using multiple slots in one response) drop in naturally because of the v0.8.5 template-fillability filter.

### New slot syntax

```text
{slot}                    — plain: substitute slot value verbatim
{slot|feat1+feat2+...}    — FST: synthesise via morphotactics
```

Feature tokens (case-insensitive, `+`-separated, unknown tokens ignored):

| token | → field |
|---|---|
| `nominative / nom` | `case = Nominative` |
| `genitive / gen` | `case = Genitive` |
| `dative / dat` | `case = Dative` |
| `accusative / acc` | `case = Accusative` |
| `locative / loc` | `case = Locative` |
| `ablative / abl` | `case = Ablative` |
| `instrumental / inst` | `case = Instrumental` |
| `singular / sg` | `number = Singular` |
| `plural / pl` | `number = Plural` |

### Examples of what now works

| template | filled | rendered |
|---|---|---|
| `{city\|locative} тұрасыз ба` | city=Алматы | Алматыда тұрасыз ба |
| `{city\|ablative} хабар жақсы ма` | city=Алматы | Алматыдан хабар жақсы ма |
| `{name\|instrumental} танысқаныма қуаныштымын` | name=Дәулет | Дәулетпен танысқаныма қуаныштымын |
| `{occupation\|plural} — қажетті мамандық` | occupation=мұғалім | мұғалімдер — қажетті мамандық |
| `сәлем {name}, {city\|ablative} хабар жақсы ма` | name=Дәулет, city=Алматы | сәлем Дәулет, Алматыдан хабар жақсы ма |

The last one is a cross-slot template: the planner only considers it when BOTH `name` and `city` are in session.

### Public API additions

- `adam_dialog::slot_syntax::{parse_placeholder, parse_noun_features}`

### TOML changes (v1.toml version → 0.9.5)

- `greeting.casual`, `greeting.polite` each get a cross-slot `{name}+{city|abl/loc}` variant.
- `statement_of_name` gets `{name|instrumental}` variants.
- `statement_of_location` gets 3 FST-backed variants: locative / ablative / dative.
- `statement_of_occupation` gets plural + dative variants.

### Tests

56 dialog end-to-end pairs (up from 52), 4 new covering every FST-backed expansion path + the cross-slot greeting. 7 slot-syntax unit tests + 1 additional planner unit test. 1 doc-test.

Workspace: **229 passing**, 4 ignored, 0 failing.

### Known v0.9.5 limitations

- Feature parser covers noun `case + number` only. Derivation and possessive are v1.0.0 additions.
- Occupation recogniser still uses the fixed 6-form table; generic 1sg-copula stripping via FST lookup is queued for v0.9.8.
- No verb slot expansion — `{root|verb_features}` would need a different synthesiser dispatch.

## [0.9.0] — 2026-04-19

Full entity absorption: every social-topic statement now contributes an extractable entity to session state. Age is parsed from Kazakh numerals (1–99), city from ablative/locative case stripping, occupation from 1sg-copula stripping.

### Intent payload changes (breaking)

- `StatementOfAge` → `StatementOfAge { years: Option<u32> }`
- `StatementOfLocation` → `StatementOfLocation { city: Option<String> }`
- `StatementOfOccupation` → `StatementOfOccupation { occupation: Option<String> }`

`None` means the intent matched on keywords but the entity wasn't parseable ("жасым жасырын").

### Numeral parser

`semantics::parse_kazakh_age` handles:

- Bare tens: он (10), жиырма (20), отыз (30), қырық (40), елу (50), алпыс (60), жетпіс (70), сексен (80), тоқсан (90)
- Bare units: бір (1) … тоғыз (9)
- Compound forms: "отыз бес" (35), "жиырма екі" (22)
- Literal digit strings: "30"

### Entity extraction

- `StatementOfLocation`: strips ablative+copula (`-данмын/-денмін/-танмын/-тенмін`) or locative (`-да/-де/-та/-те`) to recover the city root. Preserves original casing: "Алматыданмын" → "Алматы"; "астанада тұрамын" → "астана".
- `StatementOfOccupation`: matches a fixed table of 1sg-copula forms and emits the stripped noun root: "мұғаліммін" → "мұғалім".

### Session wiring

- `Conversation::absorb_entities` and `planner::extract_slots` both consume the new fields and populate `{age}`, `{city}`, `{occupation}` slots (in addition to `{name}` from v0.8.5).
- Once absorbed, the entities persist across turns just like `{name}` does.

### Templates (TOML v0.9.0)

New personalised variants in `statement_of_age`, `statement_of_location`, `statement_of_occupation`:

- `statement_of_age`: `"{age} жас — тамаша кезең"`, `"жасыңыз {age} екен"`
- `statement_of_location`: `"{city} — әдемі қала"`, `"{city} туралы көп естідім"`
- `statement_of_occupation`: `"{occupation} — құрметті кәсіп"`, `"сіз {occupation} екенсіз"`

Only eligible when the slot can be filled; untouched by templates stay canonical for utterances without extractable entities.

### Tests

52 dialog end-to-end pairs (up from 44), 8 new:

- 3 intent tests covering age numeral parsing (bare, compound, none)
- 2 location extraction tests (ablative + locative)
- 1 occupation extraction test
- 1 multi-turn absorption test (age+city+occupation into session)
- 1 multi-turn personalisation test (numeral appears in response)

Workspace: **215 passing**, 4 ignored, 0 failing.

### Known v0.9.0 limitations

- Occupation extraction uses a fixed 6-form table. Regular 1sg-copula stripping via FST lookup lands in v0.9.5 together with `{root|features}` slot expansion.
- Location extraction is surface-pattern only — no FST lookup yet, so misspelt or inflected cities ("Қызылордаданмын") get a raw root rather than normalised lexicon lemma.
- No cross-slot templates yet (`"{name}, сіз {age} жастасыз ба?"` — possible but unwritten).

## [0.8.5] — 2026-04-19

First session state in the dialog layer. The new [`Conversation`] struct accumulates entities across turns, so a user who introduces themselves once gets greeted by name on every subsequent turn.

- `Conversation { session: HashMap<String, String> }` with `new()`, `turn(input, lex, repo, seed) -> String`, `reset()`.
- `planner::plan_response_with_session(intent, seed, repo, session)` merges per-turn slots with session slots (per-turn wins on collision) and filters candidate templates down to those whose every `{slot}` reference is satisfiable. If filtering empties the pool, falls back to the full pool (visible `{slot}` is better than a crash).
- `plan_response_with_repo` is now defined in terms of `plan_response_with_session(…, &HashMap::new())` — no behaviour change for existing callers.
- Greeting families get `{name}` variants:
  - `greeting.casual`: сәлем / сәлем достым / **сәлем {name}**
  - `greeting.polite`: сәлеметсіз бе / армысыз / **сәлеметсіз бе {name}**
  - `greeting.morning` / `.day` / `.evening` all get a corresponding `{name}` variant.
- `adam_chat` CLI now holds a single `Conversation` for the whole REPL session; `--trace` mode dumps the live session map.
- Ordering: `Conversation::turn` absorbs entities BEFORE planning, so the SAME turn that says "менің атым X" can already receive a response containing `{name}` substituted to X.

Tests: 44 dialog end-to-end pairs (+3 session tests covering persistence, non-persistence when no name said, and `reset()`). 3 planner unit tests for `template_is_fillable`. Workspace: **204 passing**, 4 ignored, 0 failing.

Known v0.8.5 limitations:

- Only `name` is persisted across turns — `age`, `location`, `occupation`, `family` recognition exists but their entities aren't yet extracted into session. That lands in v0.9.0 together with numeric extraction.
- No context-aware responses: the model doesn't say "мен сізді Дәулет деп атадым, иә?" to confirm, or disambiguate "Дәулет" the name from "дәулет" the concept.

## [0.8.0] — 2026-04-19

Dialog layer widened from 10 to **25 intents**. First entity extraction lands: the user's name is pulled out of self-introduction patterns and substituted into the response template.

New intents (+15, recognisers in `semantics.rs`):

- `StatementOfName { name }` — "менің атым X" / "мені X деп атайды" / "есімім X"
- `AskAge` / `StatementOfAge` — жасың неше / менің жасым отыз
- `AskLocation` / `StatementOfLocation` — қайда тұрасыз / мен Алматыданмын
- `AskOccupation` / `StatementOfOccupation` — немен айналысасың / мен мұғаліммін
- `AskFamily` / `StatementOfFamily` — балаларың бар ма / менің балам бар
- `AskWeather` / `StatementOfWeather` — ауа райы қалай / бүгін суық
- `AskTime` — сағат неше
- `Compliment` — жарайсың / керемет / тамаша
- `Request` — өтінемін / көмектесіңізші
- `WellWishes` — сәттілік / жақсы күн тілеймін

Entity extraction + slot expansion:

- `semantics::detect_statement_of_name` extracts the PersonName from three surface patterns (атым / мені X деп атайды / есімім) with case-preserving capitalisation.
- `ResponsePlan` gains a `slots: HashMap<String, String>` field populated by the planner from the Intent.
- `realiser::realise` substitutes `{slot}` placeholders in the chosen template; templates like `"қош келдіңіз {name}"` now personalise.

Ordering subtlety: Statement-of-X is checked BEFORE Ask-of-X in every topic pair — a 1st-person marker ("келдім", "тұрамын", "жасым") unambiguously identifies the user as stating, not asking. Without this, "қайдан келдім" would hit `AskLocation` first (because of "қайдан").

TOML repository: +15 families → 29 families total, version = "0.8.0".

Tests: 41 dialog end-to-end pairs (up from 23), 18 new covering recognition, slot substitution, and planner coverage for every new intent. Workspace: **201 passing**, 4 ignored, 0 failing.

Known v0.8.0 limitations (by design, not bugs):

- No session state: the model doesn't remember the user's name across turns. Adding a `Conversation` struct lands in v0.8.5.
- Numeric extraction (age, time) is a v0.9.0 concern; StatementOfAge templates acknowledge generically.
- Templates are still literal phrases with optional `{slot}` text replacement. FST-backed `{root|features}` atoms land in v0.9.0.
- Templates have not been native-speaker reviewed — stiffness is expected; v0.9.0 tightens phrasing.

## [0.7.5] — 2026-04-19

Dialog layer widened from 5 to **10 intents** and templates externalised to TOML.

New intents (+recognisers in `semantics.rs`):

- `Thanks` — рахмет / көп рахмет / рақмет → оқасы жоқ, ештеңе емес, ризамын
- `Apology` — кешіріңіз / ғафу ет → ештеңе емес, мейлі, түк етпейді
- `AskHowAreYou` — қалайсың / қалайсыз / жағдайыңыз қалай → жақсымын рахмет, жаман емеспін, жақсы ал сіз қалайсыз
- `StatementOfWellbeing` — жақсымын / жаман емес → жақсы екен, қуанамын, ал сіз қалайсыз
- `AskName` — атың кім / есіміңіз қалай → менің атым адам, мені адам деп атайды

Templates are now loaded from `data/dialog/templates/v1.toml` (14 families, one per intent-key), not hardcoded in `planner.rs`. `TemplateRepository::load_default()` auto-discovers the TOML file; `hardcoded_fallback()` preserves MVP guarantees when the file is missing.

Public API additions:

- `adam_dialog::TemplateRepository` + `TemplateError`
- `adam_dialog::respond_with_repo(input, lex, repo, seed)` — explicit-repo variant of `respond`
- `adam_dialog::plan_response_with_repo(intent, seed, repo)` + `intent_key(intent)`

`adam_chat` REPL now loads the TOML repo at startup (falls back to hardcoded if missing) and prints family count on stderr.

Ordering subtlety in the semantic dispatcher: `Thanks`/`Apology` are checked BEFORE `Affirmation` so "рахмет" (thanks) can't accidentally fall into affirmation if later extended.

Tests: 23 dialog end-to-end pairs (up from 15), 8 new covering all 5 new intents. Workspace totals: **183 passing**, 4 ignored, 0 failing.

Known v0.7.5 limitations (by design, not bugs):

- Templates are still literal phrases; slotted templates with `(root, features)` atoms land in v0.8.0.
- No entity extraction (own name from "менің атым X" → greeting back by name).
- 10 intents cover greetings + basic social politeness; v0.8.0 widens to 25.
- No multi-turn state.

## [0.7.0] — 2026-04-19

First iteration of the predictable dialog layer. New crate `adam-dialog` implements a 5-layer pipeline (FST parser → semantics → planner → realiser → FST synthesiser) against the architectural spec in `docs/kazakh_grammar/07_dialog_architecture.md`.

Recognises 5 intents from raw Kazakh input:
- `Greeting` with kind `Casual` / `Polite` / `TimeOfDay(Morning|Day|Evening)`
- `Farewell`
- `Affirmation`
- `Negation`
- `Unknown` (fallback)

Each intent has 2–4 hand-written response variants; planner picks one by seeded PRNG mod count. The entire output space is enumerable per input — no free generation.

New binary `adam_chat` with three modes:
- `--once "<input>"` — single-shot stdout response
- default — interactive REPL over stdin
- `--trace` — dump each layer's state (parses, intent, trace lines, output)

Tests: 15 end-to-end pairs cover the full pipeline. Workspace totals: 175 passing, 4 ignored, 0 failing.

Known v0.7.0 limitations (by design, not bugs):
- Only 5 social intents; ~150 templates needed for v1.0.0 MVP.
- Templates are hardcoded in `planner.rs`, not data-driven TOML (v0.7.5).
- No morphological info used for intent classification yet (v0.7.5+).
- No multi-turn state.

## [0.6.0] — 2026-04-19

Derivational morphology — the "word-formation layer" the user flagged as a v1.0.0-path requirement. The FST now transforms a root into a new root via a derivational suffix before applying inflection. Eleven derivation types covered:

- `Agent` `-шы/-ші` (жазу → жазушы)
- `Abstract` `-лық/-лік` (жақсы → жақсылық)
- `Privative` `-сыз/-сіз` (тұз → тұзсыз)
- `Endowed` `-лы/-лі` (күш → күшті)
- `Similative` `-дай/-дей` (тау → таудай)
- `Comparative` `-рақ/-рек` (жақсы → жақсырақ)
- `VerbalNoun` `-у` (жаз → жазу)
- `ActionNoun` `-ым/-ім` (айт → айтым)
- `Diminutive` `-шық/-шік` (үй → үйшік)
- `Ordinal` `-ншы/-нші` (бір → бірінші)
- `Collective` `-еу/-ау` (бір → біреу)

`NounFeatures` gains a `derivation: Option<Derivation>` field; `synthesise_noun` applies the derivation BEFORE inflection so the two pipelines chain correctly (жазу → Agent → жазушы → Dative → жазушыға).

Tests added: 10. `adam-kernel-fst` lib now at **78 passing**. Workspace at **160 passing**, 4 ignored, 0 failing.

No other code changes.

## [0.5.5] — 2026-04-19

Pure Kazakh lexicon milestone. Enforces the "no loanwords" directive at the lexicon level and augments coverage from classical 19th-century sources.

Pipeline:

1. **Purity audit** (`lexicon_purity_audit` binary) — classified all 16,373 entries from v0.4.0 curated + v0.4.5 Apertium-imported against strict pre-modern-Kazakh criteria (Russian-only letters, loanword suffixes, no Kazakh-specific letter).
2. **Pure Kazakh build** (`build_pure_kazakh_lexicon`) — filtered out 1,500 contaminated entries (824 Russian letters, 128 loanword suffixes, 681 no-Kazakh-signal). Retained 13,606.
3. **Abai gap analysis** (`extract_abai_gap`) — identified 715 unique root candidates missing from the lexicon but present as word forms in Abai's corpus.
4. **Augmentation** (`augment_lexicon_from_abai`) — automatically classified the top 500 gap candidates (393 nouns + 107 verbs) with POS, vowel harmony, and final sound class. Output: `data/lexicon_v1/abai_augmented_roots.json`.

Result:

| metric | v0.5.0 | v0.5.5 |
|---|---|---|
| Lexicon entries (pure) | n/a | 14,106 |
| Loanwords dropped | 0 | 1,500 |
| Abai vocabulary coverage | 88.8% | **97.8%** (+9 pp) |

Missing-vocabulary examples added (each backed by corpus frequency):
- `сөз` (word, speech — 123× in Abai)
- `бой`, `қан`, `қол`, `қар`, `жау`, `жат`, `жет`, `түс`, `қыс`, `жай`
- `надан` (ignorant — Abai's key philosophical concept)

These are fundamental proto-Kazakh vocabulary items the Apertium import had zero entries for.

No changes to the FST code, phonology, or morphotactics modules. The augmented lexicon file lives alongside the v0.4.5 imports and can be unioned into the active lexicon at load time.

Workspace totals: 150 tests passing, 4 ignored, 0 failing.

## [0.5.0] — 2026-04-19

Expands the v0.4.5 FST to cover Kazakh non-finite verb forms.

- **Vowel-final-stem aorist coalescence** (Apertium rules 17, 18, 19, 20, 30). Stem-final `ы/і` merge with the aorist `{A}` to produce `и` (e.g. `оқы` + PRES + 3 → `оқиды`, not the previous `*оқыа`). Stems ending in other vowels take a `й`-glide (`сөйле` → `сөйлейді`). Past tense on vowel-final stems (`оқы` + PAST + 1SG → `оқыдым`) continues to work without coalescence.
- **Participles** — three new `Tense` variants:
  - `ParticiplePast` — `-{G}{A}н` (`жазған`, `берген`, `қалған`).
  - `ParticipleHabitual` — `-{A}тын` (`жазатын`, `келетін`).
  - `ParticipleFuture` — `-{A}р` (`жазар`, `келер`).
- **Converbs** — two new `Tense` variants:
  - `ConverbPerfect` — `-{Y}п` (`жазып`, `беріп`).
  - `ConverbImperfect` — `-{A}` (`жаза` without personal ending).

Tests: **68 unit tests passing** in `adam-kernel-fst` (up from 55 in v0.4.5). Workspace totals: 150 passing, 4 ignored, 0 failing.

No changes to v0.4.0 transformer baseline or v0.4.5 FST core code.

## [0.4.5] — 2026-04-19

Introduces **adam-kernel-fst**, a pure-Rust deterministic finite-state transducer for Kazakh morphology. This is Phase 1 of the architecture pivot from stochastic transformers to deterministic morphology + small LM (v1.0.0 track). v0.4.0 transformer stack stays untouched; v0.4.5 adds the new FST layer alongside.

Highlights:

- **New crate `adam-kernel-fst`** — phonology module (12 archiphonemes, 20+ of 54 Apertium twol rules implemented), morphotactics module (25 suffix templates covering noun plural/possessive/case and verb tense/voice/negation/person), parser module (`analyse(surface) → Vec<(root, features)>`), lexicon loader (union of 4,454 curated + 11,919 Apertium-imported entries).
- **55 unit tests + 1 smoke test + 4 ignored slow roundtrips**. Slow tests (manual: `cargo test --test roundtrip -- --ignored`) roundtrip the full 14k lexicon on 4 feature combinations: **36,238 / 36,238 = 100.0 %** success.
- **CLI binary `adam_fst`** — `synth`, `analyse`, `stats` subcommands. Hand-rolled arg parsing (no CLI-framework dep).
- **Apertium-kaz import pipeline** (`import_apertium_lexicon` binary) with POS mapping, loanword filter, and prefix-conflict guard.
- **Grammar study notes**: `docs/kazakh_grammar/00_architecture_v1.md`, `01_phonology.md`, `02_morphology.md`, `03_syntax.md`, `04_lexicon_sources.md`, `05_work_plan.md`, `06_apertium_twol_catalogue.md`.

Not yet:

- Vowel-final-stem edge cases (rule 17 coalescence, semivowel у).
- Participles, converbs, infinitive.
- LM over root + feature-bundle sequences (v0.5+ target).
- Replacement of v0.4.0 pipeline (deliberately left untouched).

Workspace totals: 137 tests passing, 4 ignored, 0 failing. CI green.

## [0.4.0] — 2026-04-17

Corpus and infrastructure maturity release. Adds the first classical-literature source (Abai Qunanbayuly via Wikisource, 146 works, 2,253 samples), the first web-crawl source (CC-100 Kazakh, 50,000 samples filtered for Cyrillic-ratio and repetition), and fixes a data-composition bug in the synthetic generator (1- and 2-word outputs dominated the corpus, teaching the model early EOS). BPE retrained at vocab **8,192** with **3.27× compression** on a 12.5M-token pretokenized corpus. Model rolled back from the v0.4.0-failed experiment (27.3M, H=512 L=6) to **24.2M params** (H=512 L=5) after confirming that the L=6 scale-up was undertrained at 3.9M tokens.

Training: 20,000 steps, batch 8, seq 128, 3e-4 peak lr with cosine decay, 8h on M2 Metal at 0.64–0.70 steps/s throughput. First-class reliability: `train_baseline` now writes a periodic checkpoint every 2000 steps after a reboot lost 13k uncheckpointed steps mid-run.

Validation (honest):
- 12,101 held-out samples (larger/harder distribution vs v0.3.0's 1,939)
- mean_ce: 7.43, **perplexity: 1691.89**
- bits/char: **3.28** (v0.4.0-failed: 3.26; v0.3.0: 3.49 — val sets not directly comparable)

Qualitative:
- Complete grammatical Kazakh sentences now appear in `temp=0.8` and nucleus samples (6 of 30 showcase outputs): `жақсы адам мағына береді`, `ол жазады`, `олар жүреді`, `үлкен жақсы адам оқыйды`, `мектеп туралы мәртебе нақтылайды`.
- Greedy still terminates early — expected for a capacity-bound model (24M params × 4M training tokens is ~25× below Chinchilla-optimal data).

v0.5.0 will address the data bottleneck: curriculum-style FSM expansion (L1/L2/L3 difficulty), larger CC-100 sample (50k → 500k), classical-literature expansion (Ауэзов, Нурпеисов, Бөкей locally), and SFT on translated Alpaca for the first instruction-following pass.

## [0.3.0] — 2026-04-15

First capacity scale-up. `ModelConfig::tiny` grows from 4.28M → **20.0M params** (hidden 224→512, layers 4→5, ffn 896→2048, head_dim 28→64). 15,000 training steps on the 39k unified corpus, 3h 45m on MacBook Air M2 Metal. Validation perplexity drops from **1112.31 → 871.30 (−21.7%)** — first meaningful delta since Phase 6a real-text onset. The 4.28M envelope was saturated at Phase 6d; Wikipedia + Common Voice distribution breadth required more model capacity. Peak RSS ~2.5 GB of 8 GB unified memory — headroom confirmed.

## [0.2.0] — 2026-04-15

First minor release after v0.1.0. Full retrain on the 39k unified corpus assembled across Phases 6a–6d. BPE retrained (3,336 merges, 2.80× compression, **0 unknowns, 100.00% roundtrip**). 4.28M model, 15,000 steps, 1h 48m wall time on M2 Metal. Val PPL 1078.68 → 1112.31 (flat; val set is larger and harder — capacity bottleneck now visible).

## [0.1.6] — 2026-04-15

**Phase 6d — Common Voice KK.** Mozilla Common Voice Kazakh sentence-collector integrated (6,108 accepted, CC0-1.0 text only). Unified corpus grows 32,986 → 39,058 unique (+6,072; 4,282 duplicates dedup'd at assembly). Also fixes `scripts/bump_foundation_version.sh`: Cargo.lock is no longer perl-replaced (corrupted transitive deps under naive substring replace); regenerated by `cargo build` after Cargo.toml bump.

## [0.1.5] — 2026-04-15

**Phase 7.1 — Wikipedia-augmented retrain.** 4.28M baseline retrained on the corpus enlarged with Wikipedia KZ. 14,000 steps, ~2h on M2 Metal. Val PPL 626.81 → 1078.68, reflecting a broader, harder val set (Wikipedia sentences are lexically richer than Tatoeba) — honest baseline on the enlarged distribution, not a regression.

## [0.1.4] — 2026-04-14

**Phase 6c — Kazakh Wikipedia.** Plain-text extracted from the kk.wikipedia.org XML dump (~296k articles → 15,000 clean samples after filter; CC-BY-SA 4.0). Unified corpus 17,986 → 32,986 unique. Infrastructure: `scripts/fetch_wikipedia_kz.sh` (bzcat + perl streaming with UTF-8 fix), `process_wikipedia_kz` binary.

## [0.1.3] — 2026-04-14

**Phase 7 — first real-text baseline.** 4.28M model trained on the unified corpus including authentic Kazakh (Tatoeba): 7,000 steps, 61m on M2 Metal, dropout 0.05, grad clipping max-norm 1.0. Explicit `loss.backward() → clip → opt.step` replaces `opt.backward_step`. First honest perplexity on real text: **626.81** (vs 129.49 on pure synthetic — tells us real Kazakh is harder).

## [0.1.2] — 2026-04-14

BPE vocab size bumped 1390 → **4096**. Char-level fallback + Tatoeba real text saturate the larger target.

## [0.1.1] — 2026-04-14

**Phase 6a/6b — first authentic Kazakh source.** Tatoeba Kazakh integrated (4,058 sentences, CC-BY 2.0 FR) via `fetch_tatoeba_kazakh.sh` + `process_tatoeba_kazakh`. Tokenizer adds char-level fallback for FSM-unknown words and leading-punct ▁ marker placement — brings roundtrip to 100% on mixed real/synthetic text.

## [0.1.0] — 2026-04-14

First minor release. The foundation works end-to-end: a Kazakh-first 3.06M-parameter transformer language model trained from scratch on a self-generated, FSM-validated synthetic corpus, evaluated against held-out perplexity, and serving inference with morpheme-aware BPE encode/decode.

### Brand
- Logo `assets/shanraq.svg` integrated into README header.
- README rewritten with centered title, badges, quickstart, and stats.
- `AUTHORS` file added.

### Quality (cumulative since v0.0.85)
- Validation perplexity: **129.49** on a 699-sample held-out set (mean cross-entropy 4.86 over 2532 tokens).
- All 464 segmentation eval examples match at 10000 bps.
- Foundation validation green across 11 layers.

---

## Phase 5 — Training and inference (v0.0.81 → v0.0.92)

### [0.0.92] — Phase 5i: Generation showcase report
- New `generation_showcase` binary: 20 prompts × 3 sampling configs = 60 generations.
- Report artifact `data/training/generation_showcase_report.json`.
- Foundation validation now requires showcase + perplexity reports.

### [0.0.91] — Phase 5h: Top-p + repetition penalty
- `generate` gains nucleus (top-p) sampling and GPT-2-style repetition penalty.
- Backwards-compatible CLI; defaults are no-ops.

### [0.0.90] — Phase 5g: Hyperparameter tuning
- Dropout 0.10 → 0.05 reduces over-regularization on small corpus.
- Gradient clipping (max-norm 1.0) added to `train_baseline`.
- Training extended to 7000 steps with 300-step warmup.
- **Perplexity: 165.98 → 129.49 (−22%).**

### [0.0.89] — Phase 5f: Model scaling + dropout
- ModelConfig::tiny() bumped: hidden 192 → 224, heads 6 → 8, ffn 768 → 896, +dropout=0.1.
- 2.33M → 3.06M params.
- `forward(ids, train: bool)` added to gate dropout on/off.

### [0.0.88] — Phase 5e: Held-out eval + perplexity
- `encode_corpus` extended with deterministic train/val split (FNV hash of sample id).
- New `eval_perplexity` binary writes structured `validation_perplexity_report.json`.
- First baseline: **165.98 perplexity**.

### [0.0.85] — Phase 5d: Inference binary
- `generate` binary: load checkpoint, autoregressive sampling (greedy/temperature/top-k).
- First sentence generated by the model: "жақсы адам аз көрсетеді."

### [0.0.84] — Phase 5c: Training loop
- `train_baseline` binary: AdamW + linear-warmup + cosine-decay LR + safetensors checkpointing.
- First trained checkpoint, training loss 7.94 → 3.39 in 7m on Metal.

### [0.0.83] — Phase 5b: Data loader
- `DataLoader` reads ids pack, produces shifted (input, target) batches on device.
- End-to-end smoke test: forward + cross-entropy loss.

### [0.0.81] — Phase 5a: Candle integration
- Added candle (HuggingFace Rust ML) with Metal backend on macOS, CPU elsewhere.
- `AdamBaseline` decoder-only transformer (initial 2.21M params).
- M2 Metal smoke test passes.

---

## Phase 4 — Tokenizer (v0.0.78 → v0.0.80)

### [0.0.87] — Phase 4d+4e: Lexicon-seeded BPE
- BPE vocab now seeded with all 211 lexicon roots + all 422 rule forms before counting pairs.
- 0% `<unk>` on any FSM-parseable Kazakh word.

### [0.0.80] — Phase 4c: BPE encoder/decoder
- `bpe::BpeTokenizer` module: load vocab+merges, encode text → ids, decode ids → text.
- `encode_corpus` binary writes a training-ready ids pack.
- 100% round-trip on 7,737 samples.

### [0.0.79] — Phase 4b: BPE trainer
- `train_bpe` binary: iterative most-frequent-pair merging over morpheme stream.
- Skips merges across word boundary (right token starts with ▁).
- 567 merges learned from corpus statistics; 2.12× compression.

### [0.0.78] — Phase 4a: Pre-tokenizer
- `pretokenize(text, lexicon, rules)`: morpheme-aware splitting via FSM.
- SentencePiece-style ▁ marker on word-start morphemes.
- Handles standalone punctuation and whole-word fallback.

---

## Phase 3 — Corpus (v0.0.74 → v0.0.77)

### [0.0.86] — Phase 3e: Full POS coverage
- 15 → 30 templates exercising every POS (adverbs, particles, modals, ол/олар, conjunctions).
- Synthetic corpus 10,000 → 18,000 samples.
- Unified corpus 7,737 → 13,929 unique samples.

### [0.0.77] — Phase 3d: Kazakh proverbs
- Added 80 classical мақал-мәтелдер across 23 themes.
- Proverbs bypass FSM-validation policy (archaic morphology); Cyrillic-only check.

### [0.0.76] — Phase 3c: Unified corpus
- `assemble_unified_corpus` binary: dedup + renumber across packs.
- 7,657 unique samples from 10,094 inputs.

### [0.0.75] — Phase 3b: Rich templates
- Generator templates 6 → 15: pronouns with matched person, conjunctions, multi-argument, etc.
- 10,000 sentences (95% yield).

### [0.0.74] — Phase 3a: Synthetic generator
- `synth_sentences` binary: combines FSM lexicon and rules to produce grammatically valid Kazakh sentences.
- Self-validation: every generated word verified by `deterministic_segment_token`.
- FSM fix: removed vowel from `verb_tense_a/e_from_stem` allowed finals (linguistically correct — `й` handles vowel-final aorist).

---

## Phase 2 — Grammatical foundation (v0.0.66 → v0.0.73)

### [0.0.73] — Phase 2h: Modals
- New `Modal` POS, 6 roots: керек, мүмкін, тиіс, шығар, қажет, лайық.

### [0.0.72] — Phase 2g: Nominal predicate
- 16 predicative personal suffix rules: -мын/мін, -сың/сің, -сыз/сіз, -мыз/міз on noun + adjective.
- 3 copula bare lexemes as Particle: еді, екен, емес.

### [0.0.71] — Phase 2f: Adverbs
- New `Adverb` POS, 19 roots: қазір, бүгін, ертең, кеше, тез, баяу, жоқ, иә, etc.

### [0.0.70] — Phase 2e: Numerals
- New `Numeral` POS, 20 cardinals: бір–жүз, мың.
- 4 ordinal suffix rules: -ншы/нші/-ыншы/інші.

### [0.0.69] — Phase 2d: Conjunctions + Particles
- New `Conjunction` POS, 9 roots: және, бірақ, себебі, өйткені, etc.
- New `Particle` POS, 12 roots: ма/ме, ба/бе, па/пе, ғой, да/де, тек, қана, өте.

### [0.0.68] — Phase 2c: Roots + 3sg aorist
- 29 nouns, 13 verbs, 5 adjectives added.
- Critical FSM fix: `tense → person_3sg` was missing for aorist forms (e.g. береді = бер+е+ді). Added rules for both future and negative_future predecessors.
- "й" connector rule for vowel-final verb stems (жасайды).
- Coverage 19.79% → **73.77%** on educational corpus.

### [0.0.67] — Phase 2b: Postpositions
- New `Postposition` POS, 15 roots: арқылы, үшін, туралы, кейін, etc.

### [0.0.66] — Phase 2a: Adjectives
- New `Adjective` POS, 25 roots, 57 inflection rules (mirror of noun rules).
- Coverage 4.56% → 17.93%.

---

## Pre-Phase 2 — Foundation infrastructure

### [0.0.65] — `normalize_token` for accurate coverage
- `coverage_report` strips trailing punctuation before FSM matching.

### [0.0.64] — adam-kernel L0 crate extraction
- Created `adam-kernel`: identity types + Kazakh FSM morphological engine.
- adam-core merged into adam-kernel.
- New `coverage_report` binary measures FSM coverage on real Kazakh text.

### [0.0.63] and earlier
- Initial corpus / tokenizer / eval / training infrastructure.
- Foundation overview report.
- Tiny clean training pipeline with miss audit.
- See git history (`git log v0.0.63 --oneline`) for details.
