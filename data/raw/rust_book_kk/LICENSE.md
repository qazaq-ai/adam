# License — Rust Book Kazakh translation

This directory contains a Kazakh-language translation of selected
chapters of *The Rust Programming Language* book (commonly referred
to as "the Rust Book"), published at https://doc.rust-lang.org/book/.

## Original work

The original Rust Book is authored by Steve Klabnik, Carol Nichols, and
the Rust community. The original is licensed under either of:

- Apache License, Version 2.0
  (https://www.apache.org/licenses/LICENSE-2.0)
- MIT License
  (https://opensource.org/licenses/MIT)

at the user's option. The full original source is available at:
https://github.com/rust-lang/book

## This translation

The Kazakh translation in this directory (files matching
`chapter_*.md`) is a derivative work of the original Rust Book. As a
derivative work, it is offered under the same dual MIT / Apache-2.0
licensing terms as the original. Authorship attribution for the
translation: see the `adam` repository's git history.

The translation is **not** an official Rust project translation. It is
maintained as part of the `adam` deterministic Kazakh dialog kernel
project (https://github.com/qazaq-ai/adam) for the purpose of populating
the morpheme-indexed retrieval corpus with technical Kazakh text. Any
errors in the translation are the responsibility of `adam` maintainers,
not of the original Rust Book authors.

## Code samples

Code blocks (delimited by backticks or fenced code blocks) within the
translated chapters are copied verbatim from the original Rust Book and
remain under the original MIT / Apache-2.0 dual license. Code identifiers
(e.g. `fn`, `let`, `Vec<T>`, `Option::Some`) are never translated; they
are preserved exactly as in the original source.

## Terminology mapping

The Kazakh-to-English terminology mapping used throughout the translation
is documented in `data/world_core/programming_rust.jsonl` and the v4.7.0
release notes. Key mappings: `ownership` → иелік, `borrow` → қарызға алу,
`borrow checker` → қарыз тексергіш, `reference` → сілтеме, `lifetime` →
тіршілік мерзімі, `crate` → сандық, `trait` → трейт, `enum` / `struct` →
енам / структ.
