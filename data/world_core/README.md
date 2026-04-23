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

## Current scale (v3.9.0)

At bootstrap, World Core ships with:

- `astronomy.jsonl` — 30 entries / ≈ 45 facts
- `time.jsonl` — 20 entries / ≈ 35 facts
- `geography_kz.jsonl` — 30 entries / ≈ 50 facts
- **Total: 80 entries / ≈ 130 facts**, all `approved` at `high` or
  `medium` confidence by `shaman`.

## Roadmap

- **v3.9.5** — expand to 500+ entries across 6–8 domains (`biology_basic`,
  `society`, `numbers`, `kz_literature`, `colors`, `body_parts`).
- **v4.0.0** — 5 000+ entries; R6 (`LivesIn + PartOf → LivesIn`) and R7
  (`GoesTo + PartOf → GoesTo`) rules activated; investor demo built on
  the `HumanApproved`-only view.
- **v4.x** — native-speaker review tool (web UI) for community
  contributions; every entry gets at least two reviewers before
  promoting to `approved`.
