# Dialog Layer Data

Template repository for the predictable Kazakh dialog pipeline
(`adam-dialog` crate, v0.7.5+). Loaded at runtime by
`TemplateRepository::load_default()`.

## Files

- `templates/v1.toml` — current template repository (**34+ families** at v4.1.0; grew over v1.8.0 session-aware evidence + v1.9.5 adapted-evidence + v2.7 `unknown.with_derived_chain` reasoning-marker family + v4.0.34 Tentative / Conflicted families for epistemic-status banding). One file per major schema version. Older schema versions, when they arrive, will live alongside as `v2.toml`, `v3.toml`, … with the crate loader picking the newest it supports.

## Schema (v1)

```toml
version = "0.8.5"       # populates ResponseRepository.version; informational

[[families]]
key = "greeting.casual" # stable identifier, matches Intent via planner::intent_key
templates = [           # 1+ surface forms, seed-mod picked by planner
    "сәлем",
    "сәлем достым",
    "сәлем {name}",     # {slot} placeholders, filled by realiser from session+intent
]
```

### Slot placeholders

Any `{name}` (future: `{age}`, `{city}`, `{root|features}`) is a **slot
placeholder**. The realiser substitutes it from the planner's slot map.
A template is eligible for a turn only if every placeholder it
references can be filled — otherwise the planner skips it. See
`crates/adam-dialog/src/planner.rs::template_is_fillable`.

### Adding a new intent family

1. Add the variant to `Intent` in `crates/adam-dialog/src/intent.rs`.
2. Map it to a key in `planner::intent_key`.
3. Write a recogniser in `semantics.rs`.
4. Add the `[[families]]` block here with 2–4 response variants.
5. Add end-to-end tests in `crates/adam-dialog/tests/end_to_end.rs`.

## Versioning

Bump the `version` string in `v1.toml` to the release version (e.g.
`"0.9.0"`). Breaking schema changes (new placeholder syntax, new
required fields) get a new file `v2.toml` and a crate-side migration
path in `templates.rs`.
