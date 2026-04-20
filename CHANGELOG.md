# Changelog

All notable changes are tagged in git as `vX.Y.Z`.

Versioning cadence (post-v1.0.0):
- **Patch `x.y.5`** — small / incremental changes (bug fixes, small Lexicon additions, docs, housekeeping).
- **Minor `x.y.0`** — significant changes (new corpus source, new intent family, new tooling, learned component).
- **`v2.0.0`** is reserved for the "minimally thinking Kazakh LM" — a trained compact Kazakh model plugged in as `Intent::Unknown` fallback. Not more rules — actual learned generalisation.

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
