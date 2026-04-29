# `data/raw/rust_book_kk/` — Rust Book Kazakh translation (phase 2 of Rust knowledge ingestion)

This directory holds the Kazakh-language translation of *The Rust
Programming Language* book, chapter by chapter. It is the corpus
source for the v4.7.x patch series.

## Cadence

Each chapter is a separate patch release on the v4.7.x minor:

- v4.7.0 — phase 1: `data/world_core/programming_rust.jsonl` (110-entry
  curated glossary). Locks the Kazakh terminology used here.
- v4.7.1 — Chapter 1 (Бастау): Installation + Hello World + Hello Cargo.
- v4.7.2 — Chapter 2 (Санды табу ойыны): first hands-on guessing game.
- v4.7.3+ — subsequent chapters (Common Programming Concepts, Ownership,
  Structs, Enums, Modules, Collections, Error Handling, Generics &
  Traits, Lifetimes, Tests, …).

## Pipeline

Each `chapter_NN.md` file is the human-readable Kazakh source for review
and audit. Sentences are extracted into `data/curated/rust_book_kk_pack.json`
(a corpus pack in the standard format) and from there flow through:

```
data/raw/rust_book_kk/chapter_*.md
  ↓ cargo run --release -p adam-corpus --bin process_rust_book_kk
data/curated/rust_book_kk_pack.json
  ↓ cargo run --release -p adam-retrieval --bin build_morpheme_index
data/retrieval/morpheme_index.json
  ↓ adam_chat (loaded at startup)
runtime retrieval samples
```

The pack is registered in `SOURCE_PACKS` constants of:
- `crates/adam-retrieval/src/bin/build_morpheme_index.rs`
- `crates/adam-corpus/src/bin/morpheme_coverage.rs`
- `crates/adam-corpus/src/bin/mine_lexicon_gaps.rs`

## Status

| Chapter | File | Patch | Status |
|---|---|---|---|
| 1 — Бастау (Getting Started) | `chapter_01.md` | v4.7.1 | translated, in pack |
| 2 — Санды табу ойыны (Programming a Guessing Game) | `chapter_02.md` | v4.7.2 | translated, in pack |
| 3 — Жалпы бағдарламалау ұғымдары (Common Programming Concepts) | `chapter_03.md` | v4.7.3 | translated, in pack |
| 4 — Иелікті түсіну (Understanding Ownership) | `chapter_04.md` | v4.7.4 | translated, in pack |
| 5 — Байланысты деректерді структ арқылы құру (Using Structs to Structure Related Data) | `chapter_05.md` | v4.7.5 | translated, in pack |

## Corpus-purity rule

Backtick-quoted spans (Rust code identifiers, file names, commands) are
preserved verbatim and bypass the Cyrillic-only check per the v4.7.0
carve-out (see `data/world_core/README.md`). Bare Latin prose outside
backticks is still flagged.

## License

See `LICENSE.md`. The translation is offered under the same MIT /
Apache-2.0 dual license as the original Rust Book.

## Translation quality

The translations are first-pass drafts produced by Claude Opus 4.7 (the
assistant), with terminology decisions locked in v4.7.0. Native-speaker
review is needed for technical accuracy and idiomatic phrasing — review
status of each chapter is tracked in this README's status table.
