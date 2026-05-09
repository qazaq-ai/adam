# World Core — curated Kazakh knowledge packs

**v3.9.0 introduction.** The text-pattern matchers in
`adam-reasoning::patterns` extract facts from raw corpus — their
precision is bounded by what Kazakh prose makes explicit, and a lot of
foundational "world-structure" knowledge is never written down as a
plain sentence. **World Core** closes that gap with human-authored,
native-reviewed curated knowledge packs.

Every entry is one short Kazakh sentence plus 1–3 typed facts. The
facts feed directly into `data/retrieval/facts.json` alongside text-
extracted facts — distinguished by `ConfidenceKind::HumanApproved`
(world_core) vs `ConfidenceKind::Grammar` (text-extracted).

## Format

One JSON object per line (JSONL), one file per domain:

```
data/world_core/
  astronomy.jsonl        # cosmology, planets, celestial bodies
  time.jsonl             # calendar units, seasons, periods
  geography_kz.jsonl     # Kazakhstan cities, rivers, mountains, neighbours
  README.md              # this file
```

**Schema:**

```jsonc
{
  "id": "astro_001",                          // stable unique id (namespace: <domain_prefix>_<number>)
  "kk": "Жер — Күн жүйесіндегі ғаламшар.",    // Kazakh sentence, author's formulation
  "facts": [                                   // 1–3 typed facts asserted by the sentence
    { "subject": "жер", "predicate": "is_a",    "object": "ғаламшар" },
    { "subject": "жер", "predicate": "part_of", "object": "күн жүйесі" }
  ],
  "domain": "astronomy",                       // matches filename stem
  "source": "curated",                         // curated | wiki_distilled | textbook_distilled | community
  "confidence": "high",                        // high | medium | low
  "review_status": "approved",                 // approved | pending | rejected
  "reviewer": "shaman",                        // git handle of the reviewer
  "reviewed_at": "2026-04-23"                  // ISO-8601 date (YYYY-MM-DD)
}
```

## Predicates available

All 11 defined in `Predicate` enum: `is_a`, `lives_in`, `has`,
`goes_to`, `part_of`, `related_to`, `causes`, `after`, `has_quantity`,
`does_to`, `in_domain`.

## Validation

Before committing any new or changed entries, run:

```
cargo run -p adam-reasoning --bin validate_world_core
```

Validator checks:

1. Schema round-trips through serde.
2. `id` / `kk` / `facts` / `domain` are all non-empty.
3. Every fact: non-empty subject + object, no self-tautologies, no
   dash-prefixed fragment roots.
4. `id` is globally unique across all domain files.
5. `kk` contains only Kazakh Cyrillic letters + standard punctuation
   (no Latin, no Chinese, no Arabic script). **v4.7.0 carve-out**:
   text inside paired backticks is treated as a code identifier and
   bypasses the Cyrillic-only check, so technical domains may embed
   keywords / type names / commands verbatim (e.g.
   ``` `let` арқылы анықталатын ```, ``` `Vec<T>` ```,
   ``` `Cargo.toml` ```). The carve-out applies ONLY inside backticks;
   bare Latin prose outside backticks is still flagged.

## Adding an entry

1. Pick the right domain file (or create a new `<domain>.jsonl`).
2. Increment the numeric suffix on the `id` (`geo_kz_028 → geo_kz_029`).
3. Write a short, factually-unambiguous Kazakh sentence. Avoid
   metaphors; avoid "I / you / we"; avoid temporal hedges ("кейбір",
   "көбіне"). The sentence should be something a curriculum author
   would put in a textbook definition.
4. Decompose into 1–3 typed facts. Prefer more general predicates
   (`is_a`, `part_of`) over specific ones where either would be valid.
5. Set `review_status` to `pending` if you're authoring without being
   the reviewer; set to `approved` if you're the reviewer for that
   domain. Only `approved` entries enter the runtime fact set.
6. Run `cargo run -p adam-reasoning --bin validate_world_core` — it
   must pass.
7. Commit.

## Trust invariants

- `ConfidenceKind::HumanApproved` on the emitted `Fact` is **exclusive**
  to world_core — text-pattern matchers never produce it.
- `source.pack` for world_core facts starts with `world_core/` — text-
  extraction never uses that prefix.
- `review_status: pending` and `rejected` entries are **loaded** by
  the validator (so they get reported on) but are **not emitted** as
  facts. They never reach `facts.json` and never reach the user.
- `adam_inspect` (v3.9.0+) separates world_core hits from text-extracted
  hits into distinct sections of its per-root report, so the user can
  tell at a glance where each fact came from.

## Current scale (v5.3.9)

**Live totals: 3003 entries / 3245 facts across 54 domains**, all `approved`
by `shaman`. Re-run `bash scripts/validate_foundation.sh` to refresh.
Per-domain counts in the listing below are a **historical v4.11.7 snapshot**
preserved for change-tracking; verify current counts via
`find data/world_core -name '*.jsonl' | xargs cat | jq -s 'length'`.

### Per-domain (v4.11.7 snapshot — see live totals above)


- `astronomy.jsonl` — 30 entries / 41 facts
- `time.jsonl` — 20 entries / 38 facts
- `geography_kz.jsonl` — 30 entries / 47 facts
- `biology_basic.jsonl` — 40 entries / 41 facts
- `body_parts.jsonl` — 40 entries / 55 facts
- `society.jsonl` — 40 entries / 48 facts
- `colors.jsonl` — 37 entries / 38 facts
- `numbers.jsonl` — 45 entries / 54 facts
- `kz_literature.jsonl` — 60 entries / 69 facts
- `food.jsonl` — 50 entries / 50 facts
- `clothing.jsonl` — 35 entries / 35 facts
- `proverbs.jsonl` — 40 entries / 43 facts
- `animals.jsonl` — 40 entries / 42 facts
- `transport.jsonl` — 42 entries / 42 facts
- `plants.jsonl` — 35 entries / 35 facts
- `professions.jsonl` — 40 entries / 40 facts
- `tools_household.jsonl` — 30 entries / 30 facts
- `music_kz.jsonl` — 16 entries / 16 facts
- `sports.jsonl` — 18 entries / 18 facts
- `house_parts.jsonl` — 20 entries / 20 facts
- `emotions.jsonl` — 18 entries / 18 facts
- `weather_phenomena.jsonl` — 15 entries / 15 facts
- `materials.jsonl` — 14 entries / 14 facts
- `language_features.jsonl` — 18 entries / 18 facts
- `cooking_methods.jsonl` — 10 entries / 10 facts
- `directions.jsonl` — 9 entries / 9 facts
- `kinship_extended.jsonl` — 18 entries / 20 facts  *(new in v4.0.19)*
- `constellations_kz.jsonl` — 6 entries / 6 facts  *(new in v4.0.19)*
- `measurements.jsonl` — 10 entries / 10 facts  *(new in v4.0.19)*
- `mathematics_basic.jsonl` — 37 entries / 37 facts  *(new in v4.6.15)*
- `informatics_basic.jsonl` — 40 entries / 40 facts  *(new in v4.6.15)*
- `programming_rust.jsonl` — 179 entries / 179 facts  *(110 baseline in v4.7.0; +40 Latin-alias entries in v4.26.0; +10 keyword/concept aliases in v4.26.5; +18 advanced concepts in v4.27.0; +1 in v4.27.5. rust_111…rust_179 are auto-curated `reviewer: "claude"` — pending native-speaker review per `docs/rust_glossary_review_v4.28.md`. Approve via `bash scripts/approve_rust_entry.sh <id>` after review)*
- `physics_school.jsonl` — 102 entries / 102 facts  *(new in v4.8.0)*
- `chemistry_school.jsonl` — 105 entries / 105 facts  *(new in v4.9.0)*
- `biology_school.jsonl` — 120 entries / 120 facts  *(new in v4.10.0)*
- `history_kazakhstan.jsonl` — 124 entries / 124 facts  *(new in v4.11.0)*
- `adam_self.jsonl` — 33 entries / 33 facts  *(new in v4.11.5; +6 rich subject claims in v4.11.6)*
- **Snapshot total: 1625 entries / 1791 facts across 38 domains** (v4.11.7 — preserved as a historical milestone; live totals at top of section), all `approved`
  at `high` or `medium` confidence by `shaman`. v4.3.5 added **kz_literature
  surname-keyed entries** (Әуезов / Сейфуллин / Мүсірепов / Жұмабаев / …) and a
  new **`notable_kazakhstanis.jsonl` domain** (presidents, khans, scientists,
  war heroes, athletes, judges). v4.4.10 added **73 entries to `geography_kz`**:
  all 17 Kazakh oblasts as administrative entities, oblast → administrative-center
  mappings, three cities of republican significance upgrade, country + 3-republic-city
  population facts, 6 new rivers (Жайық / Есіл / Тобыл / Шу / Қаратал / Талас),
  4 new lakes (Зайсан / Алакөл / Тенгіз / Маркакөл), 5 mountains (Тянь-Шань /
  Жетісу Алатауы / Хан Тәңірі / Қаратау / Ұлытау), 4 deserts (Бетпақдала /
  Қызылқұм / Үстірт / Мойынқұм), Шарын каньоны, Бурабай, plus 9 IsA-hub bridge
  facts (`өзен/көл/теңіз IsA су денесі`, `тау/шөл/каньон IsA жер бедері`,
  `облыс IsA әкімшілік бөлік`, `қала/ауыл IsA елді мекен`) which delivered
  +972 R1/R2/R5 derivations on the next reasoner re-run.
  `geography_kz.jsonl` doubles as the canonical source for the v4.3.0
  `language_core::canonical_geo_entity` resolver — every place mention in dialog
  memory carries the matching `geo_kz_NNN` id as `EntityMemory.canonical_id`.
  Verify in-tree with `find data/world_core -name '*.jsonl' | xargs cat | jq -s 'length'`
  (entries) and `... | jq -s 'map(.facts | length) | add'` (facts).

## Roadmap

- **v4.0.0** (shipped) — **507 entries / 601 facts** across 13 domains,
  including 7 new domains (colors, numbers, kz_literature, food,
  clothing, proverbs, animals). Contradiction filter (R6/R7 refuse
  astronomical-scale derived targets) and object-side 3-char minimum
  close the Codex-flagged v3.9.5 noise (`бала lives_in күн жүйесі`,
  `бала lives_in ған`, `(егер, DoesTo, X)`, `(жалға, GoesTo, жер)`).
- **v4.x** — `validate_world_core` integrated into
  `scripts/validate_foundation.sh` as a CI gate. Native-speaker
  review tool (web UI); every entry gets at least two reviewers
  before promoting to `approved`.
- **v5.x** — target 1 500+ entries, 20+ domains, typed ontology
  layer (Codex's «Typed World Model» recommendation — every root
  gets an EntityType, rules become type-constrained, Fact Promotion
  Pipeline with `Candidate` → `Verified` → `HumanApproved` tiers).
