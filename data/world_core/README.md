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
   (no Latin, no Chinese, no Arabic script).

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

## Current scale (v4.0.15)

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
- `language_features.jsonl` — 18 entries / 18 facts  *(new in v4.0.15)*
- `cooking_methods.jsonl` — 10 entries / 10 facts  *(new in v4.0.15)*
- `directions.jsonl` — 9 entries / 9 facts  *(new in v4.0.15)*
- **Total: 792 entries / 886 facts across 26 domains**, all `approved`
  at `high` or `medium` confidence by `shaman`.

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
