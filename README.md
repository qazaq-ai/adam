<p align="center">
  <img src="assets/shanraq.svg" alt="adam logo" width="128" height="128">
</p>

<h1 align="center">adam</h1>

<p align="center">
  <i>Predictable Kazakh-first dialog, built in pure Rust.</i><br>
  <i>Қазақ тіліне арналған, толық болжамды диалог жүйесі — таза Rust тілінде.</i>
</p>

<p align="center">
  <a href="https://github.com/qazaq-ai/adam/releases"><img src="https://img.shields.io/badge/version-4.7.13-2EA44F?style=for-the-badge" alt="version"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-BUSL%201.1-orange?style=for-the-badge" alt="license"></a>
  <img src="https://img.shields.io/badge/language-Rust-CE412B?style=for-the-badge&logo=rust&logoColor=white" alt="rust">
  <img src="https://img.shields.io/badge/script-Cyrillic-8338EC?style=for-the-badge" alt="cyrillic">
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Linux-lightgrey?style=for-the-badge" alt="platform">
</p>

<p align="center">
  <img src="https://img.shields.io/badge/intents-26-2EA44F?style=flat-square" alt="intents">
  <img src="https://img.shields.io/badge/surface-Kazakh--only-9CCC65?style=flat-square" alt="Kazakh only">
  <img src="https://img.shields.io/badge/lexicon-25.5%20k%20roots-FBC02D?style=flat-square" alt="lexicon">
  <img src="https://img.shields.io/badge/corpus-77.9%20M%20local%20/%204.57%20M%20committed-FBC02D?style=flat-square" alt="corpus">
  <img src="https://img.shields.io/badge/retrieval-morpheme%20index-8338EC?style=flat-square" alt="retrieval">
  <img src="https://img.shields.io/badge/tests-745%20passing-2EA44F?style=flat-square" alt="tests">
  <img src="https://img.shields.io/badge/cognitive%20eval-65%2F65%20canonical-2EA44F?style=flat-square" alt="cognitive eval">
  <img src="https://img.shields.io/badge/repl%20replay-69%2F69%20canonical-2EA44F?style=flat-square" alt="repl replay">
  <img src="https://img.shields.io/badge/p50%20turn%20latency-1.07%20ms%20on%20M2-2EA44F?style=flat-square" alt="p50 turn latency">
  <img src="https://img.shields.io/badge/RSS-~76--80%20MB-2EA44F?style=flat-square" alt="rss">
  <img src="https://img.shields.io/badge/reasoning%20rules-10%20active-2EA44F?style=flat-square" alt="reasoning rules">
  <img src="https://img.shields.io/badge/predicate%20coverage-11%2F11-2EA44F?style=flat-square" alt="predicate coverage">
  <img src="https://img.shields.io/badge/world%20core-1142%20curated%20/%201305%20facts-9CCC65?style=flat-square" alt="world core">
  <img src="https://img.shields.io/badge/domains-33-9CCC65?style=flat-square" alt="domains">
  <img src="https://img.shields.io/badge/policy-Rust--only%20%2B%20Graph--first-1976D2?style=flat-square" alt="policies">
  <img src="https://img.shields.io/badge/ungrounded%20generation-none%20on%20deterministic%20path-2EA44F?style=flat-square" alt="ungrounded generation">
</p>

---

## Why adam (v4.4)

adam is a **deterministic cognitive kernel for Kazakh** — rule-based dialog with auditable belief revision, morpheme-indexed retrieval, and a forward-chaining reasoner over typed facts, all running as a single tool-driven pipeline. It trades **generalisation for integrity**: every output is traceable, every belief revisable, every conclusion sourced. Every layer is **Rust-only** and **graph-first** by repository invariant — both enforced by contract tests.

**v4.7.13 follow-up — Rust Book Chapter 13 (Функционал тілдік мүмкіндіктер: итераторлар мен жабулар) translated, in pack («глава = патч» cadence).** Full Kazakh translation of Rust Book Chapter 13 — Functional Language Features: Iterators and Closures — covering Rust's two key functional-programming primitives: closures (anonymous functions capturing the environment, `FnOnce` / `FnMut` / `Fn` trait hierarchy, `move` for ownership transfer, the `Vec::sort_by_key` example) and iterators (the `Iterator` trait + `next` method, `iter`/`iter_mut`/`into_iter` distinction, lazy evaluation, consuming adapters `sum`/`count`/`collect` vs producing adapters `map`/`filter`, the `(1..=10).filter(...).map(...).sum()` chaining pattern, capturing closures inside iterator chains). Also: refactoring the v4.7.12 minigrep with iterators (removing `clone` from `Config::build` by passing an iterator instead of a slice, condensing `search` / `search_case_insensitive` to one-line iterator chains). Closes with the **zero-cost abstraction** explanation: iterator chains compile to assembly indistinguishable from hand-rolled loops. New `data/raw/rust_book_kk/chapter_13.md` (~5 000 words). Chapter-13-specific terminology: capture → **ұстау**, `Fn`/`FnMut`/`FnOnce` → kept verbatim, iterator adapter → **итератор-бейімдеуіш**, lazy evaluation → **лазай есептеу**, zero-cost abstraction → **нөлдік шығынды абстракция**, consuming adapter → **тұтынатын бейімдеу**. Pack: 12 chapters / 911 samples → **13 chapters / 985 samples**. Morpheme index unchanged (rust_book pack still at 500-per-pack ceiling). Workspace 745 unchanged.

**v4.7.12 follow-up — Rust Book Chapter 12 (Кіріс-шығыс жобасы: команда жолы бағдарламасын құру) translated, in pack («глава = патч» cadence).** Full Kazakh translation of Rust Book Chapter 12 — An I/O Project: Building a Command Line Program (mini-grep) — the largest practical chapter that ties together everything from chapters 1–11 into one real working CLI program. Sections: accepting command-line arguments via `std::env::args`; reading a file with `std::fs::read_to_string`; refactoring for modularity and error handling (separation of concerns, extracting `Config` struct, `Config::build` constructor, `Result` + `unwrap_or_else` + `eprintln!` + `process::exit` error pipeline, extracting a `run` function, splitting code into `src/lib.rs`); TDD development of `search` (failing test → minimum passing code → refactor); working with environment variables (`env::var("IGNORE_CASE")`, `search_case_insensitive` companion function); writing error messages to standard error instead of standard output (`eprintln!` vs `println!`, Unix stdout/stderr separation, `> output.txt` redirection demonstration). This is the practical chapter showing how all earlier concepts come together: modules, ownership, references, traits, error handling, tests — in one ~150-line program. New `data/raw/rust_book_kk/chapter_12.md` (~5 500 words). Chapter-12-specific terminology: command-line argument → **командалық жол аргументі**, separation of concerns → **жауапкершіліктерді бөлу**, test-driven development → **тестке негізделген әзірлеу**, standard output → **стандартты шығару**, standard error → **стандартты қате**, environment variable → **орта айнымалысы**, case-insensitive → **әріп регистрін ескермеу**, trait object → **трейт-нысан**. Pack: 11 chapters / 832 samples → **12 chapters / 911 samples**. Morpheme index unchanged (rust_book pack still at 500-per-pack ceiling). Workspace 745 unchanged.

**v4.7.11 follow-up — Rust Book Chapter 11 (Автоматты сынақтар жазу) translated, in pack («глава = патч» cadence).** Full Kazakh translation of Rust Book Chapter 11 — Writing Automated Tests — covering Rust's built-in testing infrastructure: how to write tests (`#[test]` attribute, `assert!`/`assert_eq!`/`assert_ne!`, custom failure messages, `#[should_panic]` with `expected = "..."`, tests returning `Result<T, E>` with `?`); controlling test runs (parallel-vs-sequential `--test-threads=1`, `--show-output`, name filtering by substring, `#[ignore]` and `--ignored` / `--include-ignored`); test organization (unit tests inside `#[cfg(test)] mod tests` testing private functions; integration tests in the `tests/` directory as separate crates exercising the public API only; `tests/common/mod.rs` shared-helper convention; the lib-vs-bin split for testable binary crates). New `data/raw/rust_book_kk/chapter_11.md` (~4 000 words). Chapter-11-specific terminology: automated test → **автоматты сынақ**, assertion → **бекіту**, test runner → **тест жүгіртушісі**, parallel → **параллельді**, sequential → **дәйекті**, subset → **ішкі жиын**, ignore → **елемеу**, unit test → **бірлік тесті**, integration test → **интеграциялық тест**. Pack: 10 chapters / 767 samples → **11 chapters / 832 samples**. Morpheme index unchanged (rust_book pack still at 500-per-pack ceiling). Workspace 745 unchanged.

**v4.7.10 follow-up — Rust Book Chapter 10 (Жалпылама типтер, трейттер мен тіршілік мерзімі) translated, in pack («глава = патч» cadence; the most theoretically dense chapter of the entire book).** Full Kazakh translation of Rust Book Chapter 10 — Generic Types, Traits, and Lifetimes — covering the three abstraction layers in one chapter: types (via generic type parameters), behaviour (via traits), references' validity (via lifetimes). Sections: 10.1 Generic data types — generics in functions/structs/enums/methods, the **monomorphization** explanation (zero runtime cost via per-type code generation); 10.2 Traits: defining shared behaviour — defining traits, implementing on types, the **orphan rule**, default implementations, traits as parameters via `&impl Trait` vs trait-bound `<T: Trait>`, multiple bounds with `+`, `where` clauses, returning `impl Trait`, conditional method implementations, blanket implementations (`impl<T: Display> ToString for T`); 10.3 Lifetimes — preventing dangling references, the borrow checker, generic lifetimes in functions (`fn longest<'a>(x: &'a str, y: &'a str) -> &'a str`), lifetimes in struct definitions, the three lifetime elision rules, lifetimes in method definitions, the `'static` lifetime, generic types + trait bounds + lifetimes combined in one signature. New `data/raw/rust_book_kk/chapter_10.md` (~5 500 words). Chapter-10-specific terminology: monomorphization → **мономорфтау**, default implementation → **әдепкі іске асыру**, blanket implementation → **жалпы іске асыру**, trait bound → **трейт шектеуі**, where clause → **where клаузасы**, lifetime elision → **тіршілік мерзімін түсіріп жазу**, orphan rule → **орфан-ереже**. Pack: 9 chapters / 675 samples → **10 chapters / 767 samples**. Morpheme index unchanged (rust_book pack still at 500-per-pack ceiling). Workspace 745 unchanged.

**v4.7.9 follow-up — Rust Book Chapter 9 (Қатені өңдеу) translated, in pack («глава = патч» cadence).** Full Kazakh translation of Rust Book Chapter 9 — Error Handling — covering Rust's two-tier error model: unrecoverable errors with `panic!` (panic message format, array-indexing example, `RUST_BACKTRACE=1`, the `panic = "abort"` profile setting and unwind-vs-abort trade-off) and recoverable errors with `Result<T, E>` (the enum, `File::open` returning `Result<File, io::Error>`, matching with deeply-nested handler arms, `error.kind()` and `ErrorKind::NotFound`, `unwrap`/`expect` as shortcuts for prototypes/tests, explicit propagation via `match`, the `?` operator and how it returns `Err` from the function, `?` chained, `fs::read_to_string` as canonical fully-condensed form, error type conversion via `From` trait, where `?` may be used — `Result`, `Option`, `main` returning `Result<(), Box<dyn Error>>`); when to panic vs when to return Result (prototypes/tests, contract violations, user-input parsing, trait-encoded invariants like the `Guess` 1–100 example). New `data/raw/rust_book_kk/chapter_09.md` (~4 000 words). Chapter-9 terminology: error propagation → **қатені тарату**, error conversion → **қатені түрлендіру**, backtrace → **шегініс ізі**, stack unwinding → **стек жадын кері айналдыру**, abort → **үзу**. Pack: 8 chapters / 608 samples → **9 chapters / 675 samples**. Morpheme index unchanged (rust_book pack still at 500-per-pack ceiling). E2E threshold stays ≥490. Workspace 745 unchanged.

**v4.7.8 follow-up — Rust Book Chapter 8 (Жалпы ұжымдар) translated, in pack («глава = патч» cadence; past committed-index ceiling).** Full Kazakh translation of Rust Book Chapter 8 — Common Collections — covering the three most-used standard-library collection types: `Vec<T>` (creating with `Vec::new` and `vec!`, `push`, reading via `&v[i]` panic vs `v.get(i)` Option, the borrow rule preventing concurrent index reads with `push` due to potential reallocation, iteration with `&v` and `&mut v` + `*i` dereferencing, storing multiple types via enum variants, drop semantics); `String` (UTF-8 commitment, `String::new`/`to_string`/`String::from`, `push_str`/`push`/`+`/`format!`, why indexing is forbidden with the `Здравствуйте` 24-byte example, byte-aligned slicing with `&s[a..b]` and panic on mid-codepoint cut, `chars` for Unicode scalars vs `bytes` for raw bytes, why grapheme clusters require external crates); `HashMap<K, V>` (creating, `get` → `Option<&V>` with `.copied().unwrap_or(0)` idiom, ownership transfer for non-`Copy` keys/values, three update strategies — `insert` overwriting, `entry().or_insert()` for missing keys, word-counter `*count += 1` pattern with mutable references, SipHash default and DoS-resistance trade-off). New `data/raw/rust_book_kk/chapter_08.md` (~4 500 words). Chapter-8 terminology: grapheme cluster → **графема кластері**, hash function → **хэш функциясы**, dereference → **дереференс**, byte boundary → **байт шегі**. Pack: 7 chapters / 525 samples → **8 chapters / 608 samples**. Morpheme index **unchanged** — the rust_book pack hit the 500-per-pack default-mode ceiling at v4.7.7. Chapter-8 sentences live in the pack file (auditable, `--full`-mode ready) but don't contribute to the committed morpheme index. E2E threshold stays at ≥490. Workspace 745 unchanged.

**v4.7.7 follow-up — Rust Book Chapter 7 (Бумалармен, сандықтармен, модульдермен жобаны басқару) translated and ingested («глава = патч» cadence).** Full Kazakh translation of Rust Book Chapter 7 — Managing Growing Projects — covering the four-layer modular system: packages (`Cargo.toml`, at-most-one library + any number of binaries, `src/main.rs` / `src/lib.rs` / `src/bin/*.rs` conventions); crates (binary vs library, crate root); modules (`mod`, the module tree from `crate`, in-line vs separate-file); paths (absolute via `crate::…`, relative via `self`/`super`/module names; the everything-private-by-default rule, `pub` opening one layer, `pub struct` requiring per-field `pub`, `pub enum` exposing all variants); `use` (idiomatic patterns — import parent for functions, import the type for `HashMap`/`String`/`Vec`; `as` for renaming; `pub use` re-exporting; nested paths `{…}`; `self` in nested paths; `*` glob and when not to use it); external crates and `std` as the always-available special case; separating modules into different files. New `data/raw/rust_book_kk/chapter_07.md` (~5 000 words). Chapter-7-specific terminology: package → **бума**, binary crate → **екілік сандық**, library crate → **кітапхана сандығы**, crate root → **сандық түбірі**, module tree → **модуль ағашы**, privacy → **жекелік**, absolute path → **абсолюттік жол**, relative path → **салыстырмалы жол**, re-export → **қайта экспорттау**, nested paths → **тоғыспалы жолдар**, items → **элементтер**. Pack: 6 chapters / 464 samples → **7 chapters / 525 samples**. Morpheme index: 3 350→**3 362 morphemes** (+12), 21 747→**22 145 postings** (+398), 3 655→**3 691 indexed samples** (+36 — rust_book pack hit the 500-per-pack default-mode ceiling). E2E threshold capped at ≥490 with documenting comment. Workspace 745 unchanged.

**v4.7.6 follow-up — Rust Book Chapter 6 (Енам мен үлгіге сай келтіру) translated and ingested («глава = патч» cadence).** Full Kazakh translation of Rust Book Chapter 6 — Enums and Pattern Matching — covering: defining enums (variants, attaching data of different types per variant, methods via `impl`); the `Option<T>` enum and the case against `null` (Tony Hoare's "billion-dollar mistake"); `Some(T)` vs `None` and why `Option<T>` and `T` are separate types; the `match` control flow construct (matching on enum variants, patterns that bind to inner values, exhaustiveness checking, catch-all patterns with named binding vs `_` placeholder, unit `()` arms); `if let` as concise syntax for matching one variant, with optional `else`. New `data/raw/rust_book_kk/chapter_06.md` (~3 500 words). Chapter-6-specific terminology: variant → **нұсқа**, exhaustive → **барлық нұсқаны қамту**, catch-all pattern → **жалпы тармақ**, placeholder `_` → **орынтолтырғыш**, null → **нөлдік мән**, pattern matching → **үлгіге сай келтіру**. Pack: 5 chapters / 402 samples → **6 chapters / 464 samples**. Morpheme index: 3 339→**3 350 morphemes** (+11), 21 121→**21 747 postings** (+626), 3 593→**3 655 indexed samples** (+62). E2E threshold tightened ≥380 → ≥440 rust_book sentences. Workspace 745 unchanged.

**v4.7.5 follow-up — Rust Book Chapter 5 (Байланысты деректерді структ арқылы құру) translated and ingested («глава = патч» cadence).** Full Kazakh translation of Rust Book Chapter 5 — Using Structs to Structure Related Data — covering: named-field structs (dot-access, instance mutability, field init shorthand, struct update syntax `..` with ownership/Copy interaction); tuple structs and unit-like structs; struct data ownership (why `String` over `&str` in struct fields without lifetime annotations); a worked rectangle-area example showing progression `(width, height)` → tuple → struct; derived traits (`#[derive(Debug)]`, `{:?}` and `{:#?}` pretty-print, `dbg!` macro); method syntax (`impl` blocks, `&self`/`&mut self`/`self` first parameter, automatic referencing/dereferencing, methods with extra parameters like `can_hold`); associated functions (no `self`, `Self` as impl's type, conventional constructors, `::` call syntax); multiple `impl` blocks. New `data/raw/rust_book_kk/chapter_05.md` (~4 000 words). Chapter-5-specific terminology: field init shorthand → **өрісті қысқа жариялау**, struct update syntax → **структты жаңарту синтаксисі**, derived trait → **алынған трейт**, automatic referencing/dereferencing → **автоматты сілтемелеу**, pretty-print → **әдемі басып шығару**, instance → **дана**. Pack: 4 chapters / 328 samples → **5 chapters / 402 samples**. Morpheme index: 3 330→**3 339 morphemes** (+9), 20 430→**21 121 postings** (+691), 3 519→**3 593 indexed samples** (+74). E2E threshold tightened ≥300 → ≥380 rust_book sentences. Workspace 745 unchanged.

**v4.7.4 follow-up — Rust Book Chapter 4 (Иелікті түсіну) translated and ingested («глава = патч» cadence).** The central, most conceptual chapter of the entire book — Understanding Ownership. Full Kazakh translation covering: stack vs heap, the three ownership rules, variable scope, the `String` type vs string literals, memory allocation and `drop`, ownership transfer (move), `clone` for deep copy, the `Copy` trait, ownership and function calls, return values; references and borrowing (`&T` immutable, `&mut T` mutable, the two reference rules — exclusivity of mutable references vs. shared immutable references — and how data races are prevented at compile time, dangling reference prevention); the slice type (`&str` string slices, `&[T]` array slices, range `..` syntax variants, `&str` as the more general parameter type). This is the chapter for which v4.7.0 specifically locked the terminology (иелік / қарызға алу / қарыз тексергіш / тіршілік мерзімі / сілтеме / өзгермелі / тұрақты), and it is now canonically applied. New `data/raw/rust_book_kk/chapter_04.md` (~6 000 words). Chapter-4-specific terminology: ownership rules → **иелік ережелері**, move → **иелікті ауыстыру**, deep copy → **терең көшіру**, data race → **жарыс шарты**, dangling reference → **жабайы сілтеме**, slice → **тілім**, string slice → **жол тілімі**. Pack: 3 chapters / 231 samples → **4 chapters / 328 samples**. Morpheme index: 3 307→**3 330 morphemes** (+23), 19 447→**20 430 postings** (+983), 3 422→**3 519 indexed samples** (+97). E2E threshold tightened ≥200 → ≥300 rust_book sentences. Workspace 745 unchanged.

**v4.7.3 follow-up — Rust Book Chapter 3 (Жалпы бағдарламалау ұғымдары) translated and ingested («глава = патч» cadence).** Full Kazakh translation of Rust Book Chapter 3 — Common Programming Concepts — covering the foundational concepts that recur throughout Rust: variables and mutability (default-immutable bindings, `mut`, constants `const` with mandatory type annotation and SCREAMING_SNAKE_CASE convention, shadowing via `let` and how it differs from `mut` including type-changing); data types (scalar — integer types `i8`/`i16`/`i32`/`i64`/`i128` with unsigned/signed pairs and `usize`/`isize`, integer overflow behaviour in debug vs. release, floating-point `f32`/`f64`, numeric operations, boolean, character; compound — tuples with destructuring and dot-index access, the unit `()`, arrays with type/length annotation `[i32; 5]` and out-of-bounds panic); functions (`fn` keyword, snake_case convention, mandatory parameter type annotations, the critical statement-vs-expression distinction, block expressions, return values via `->`); comments (`//`, `/* */`, doc comments `///`); control flow (`if` / `else if` / `else` with bool-only conditions, `if` as expression in `let`, `loop` with `break value`, loop labels for nested loops, `while`, `for` over arrays and ranges, range expressions `1..4` exclusive vs `1..=4` inclusive, `.rev()`). New `data/raw/rust_book_kk/chapter_03.md` (~5 000 words). Chapter-3-specific terminology: scalar → **жалғыз**, compound → **құрама**, integer overflow → **бүтін санның асып кетуі**, floating-point → **қалқымалы үтірлі**, numeric operations → **сандық амалдар**, tuple destructuring → **бөлшектеу**, statement vs expression → **сөйлем мен өрнек**, doc comment → **құжаттама түсініктемесі**, loop label → **цикл белгісі**, range → **диапазон**, inclusive/exclusive range → **қамтылған/қамтылмаған диапазон**, mutability → **өзгермелілік**. Pack: 2 chapters / 134 samples → **3 chapters / 231 samples**. Morpheme index: 3 265→**3 307 morphemes** (+42), 18 485→**19 447 postings** (+962), 3 325→**3 422 indexed samples** (+97). E2E threshold tightened ≥120 → ≥200 rust_book sentences. Workspace 745 unchanged.

**v4.7.2 follow-up — Rust Book Chapter 2 (Санды табу ойыны) translated and ingested («глава = патч» cadence).** Full Kazakh translation of Rust Book Chapter 2 — Programming a Guessing Game — covering the hands-on guessing game project: new Cargo project, `std::io::stdin().read_line()` user input, mutable variables (`let mut guess = String::new()`), references / mutable references (`&mut guess`), `Result`-based error handling with `.expect()`, adding the external `rand` crate (Cargo `[dependencies]`, semantic versioning), `rand::thread_rng().gen_range(1..=100)`, comparing values with `std::cmp::Ordering` + `match` (`Less / Greater / Equal`), type mismatch and shadowing for type conversion (`let guess: u32 = guess.trim().parse().expect(...)`), `loop` + `break` on success, graceful invalid-input handling via `match Result { Ok(num) => num, Err(_) => continue }`. New `data/raw/rust_book_kk/chapter_02.md` (~3 500 words). Chapter-2-specific terminology: random number → **кездейсоқ сан**, mutable variable → **өзгермелі айнымалы**, scope → **аумақ**, parse → **талдау**, type inference → **түр-қорытынды**, semantic versioning → **семантикалық нұсқалау**. Pack: 1 chapter / 60 samples → **2 chapters / 134 samples**. Morpheme index: 3 213→**3 265 morphemes** (+52), 17 637→**18 485 postings** (+848), 3 251→**3 325 indexed samples** (+74). E2E threshold tightened ≥50 → ≥120 rust_book sentences. Workspace 745 unchanged.

**v4.7.1 follow-up — Rust Book Chapter 1 (Бастау) translated and ingested into the morpheme_index (phase 2 begins, «глава = патч» cadence).** Full Kazakh translation of Rust Book Chapter 1 — Getting Started — covering installation (rustup, Linux/macOS, Windows, troubleshooting, updating, local docs), Hello World (project directory, writing/running first program, anatomy of a Rust program, compile-vs-run as separate steps), and Hello Cargo (Cargo project creation, build/run/check, release build). New `data/raw/rust_book_kk/chapter_01.md` (~3 000 words, code blocks verbatim, all v4.7.0 terminology applied). New `crates/adam-corpus/src/bin/process_rust_book_kk.rs` Rust binary reads chapters, strips fenced code blocks + markdown decoration, splits prose into sentences (preserving backtick spans so the dot in `Cargo.toml` is not a sentence boundary), emits standard pack format. Pack registered in `SOURCE_PACKS` of `build_morpheme_index`, `morpheme_coverage`, `mine_lexicon_gaps`. Morpheme index: 3 082 → **3 213 morphemes** (+131), 16 262 → **17 637 postings** (+1 375), 3 117 → **3 251 indexed samples** (60 chapter-1 sentences in `sample_texts`, 247 morphemes referencing rust_book samples). 1 new e2e test + 4 unit tests in the corpus binary. Workspace 740→**745**. Retrieval-ranker behaviour: world_core definitions preferred over corpus citations (correct priority); chapter sentences surface when query hits Rust-specific morpheme with no world_core match. Latin-name query limitation unchanged from v4.7.0 (deferred).

**v4.7.0 milestone — `programming_rust.jsonl` glossary + corpus-purity carve-out for technical text (phase 1 of Rust knowledge ingestion).** Fifth v4.x minor. User strategic ask: «обучить нашу модель языку программирования Rust». Honest scope: adam can't generate code (retrieval-only architecture), but it CAN serve as a deterministic Kazakh-language Rust glossary — and Kazakh-language Rust documentation virtually doesn't exist outside this domain. v4.7.0 = phase 1 (curated glossary). Phases 2+ = Rust Book chapter translations as patch releases (v4.7.1, v4.7.2, …). New `data/world_core/programming_rust.jsonl` carries **110 entries / 110 facts** covering Rust core (Rust, Cargo, rustc, сандық/crate, модуль), ownership / borrowing / lifetimes (иелік, иелік моделі, сілтеме, қарызға алу, қарыз тексергіш, тіршілік мерзімі), variables / functions / primitive types (i32 / i64 / u32 / u64 / usize / f32 / f64 / bool / char / str / String / кортеж / жиым / тілім), collections (Vec / HashMap / BTreeMap / HashSet / VecDeque), structs / enums / Option / Result, control flow (if / match / loop / while / for), traits and generics (трейт, derive, жалпылама тип, тип параметрі, шектеу), error handling (panic / unwrap / expect / `?` / Drop), smart pointers / concurrency (Box / Rc / Arc / RefCell / Mutex / ағын / канал / async / await / Future), iterators (map / filter / collect / жабу), unsafe blocks, modules / visibility, Cargo workflow (build / run / test / check, clippy, rustfmt). Terminology decisions locked at start of phase 1 (will guide all chapter translations in phase 2): `ownership` → **иелік**, `borrow` → **қарызға алу**, `borrow checker` → **қарыз тексергіш**, `reference` → **сілтеме**, `lifetime` → **тіршілік мерзімі**, `crate` → **сандық** (preserves the wooden-crate metaphor), `trait` → **трейт** (transliteration), `enum/struct` → **енам/структ**. Code identifiers (`Vec<T>`, `Option::Some`, `match`, `let`, `fn`) are NEVER translated, kept verbatim in backticks. **Corpus-purity carve-out**: `validate_world_core::non_kazakh_reason` now skips characters inside paired backticks; bare Latin prose outside backticks is still flagged. Pipeline impact: facts.json 15 721 → **15 831** (+110), derived 22 962 → **23 418** (+456 via R1/R2/R5/R8 inheritance through new IsA hubs `бағдарламалау тілі / мәлімет түрі / ұжымдық тип / басқару құрылымы / жад моделі / трейт`), MULTIWORD_ENTITIES +52 compounds, lexicon +24 noun roots. **Known limitations** (resolved in later phases): direct Latin-name queries («Rust дегеніміз не?», «Cargo дегеніміз не?») don't tokenize through the Cyrillic-only FST and fall through to Unknown; Kazakh-paraphrased queries (Иелік / Трейт / Сілтеме / Тіршілік мерзімі) work correctly. ASCII-identifier passthrough is deferred to a later patch once Rust Book chapter content surfaces enough Latin-prose context to justify it. No code generation — retrieval-only architecture stands. World Core 1032/1195/32→**1142/1305/33**, REPL replay 68/68→**69/69**, workspace 739→**740**. Why minor: new world_core domain shipping with the corpus-purity rule (architectural carve-out) + 24-root lexicon expansion + 52-compound MULTIWORD_ENTITIES growth + 110-fact knowledge base.

**v4.6.20 follow-up — 5 more innovations bundled (20 total cumulative on the v4.6.x minor): reflexive identity question + adj+noun compound noun-hint + `SelfComparison` aspect + discourse-preamble stripper + `UserAcknowledgement` intent.** Real-REPL 2026-04-29 (fifth transcript) surfaced 5 distinct defects sharing one theme: adam couldn't make sense of long, multi-clause Kazakh sentences — greedy noun-hint extraction grabbed adverbs (`әлі`) or modifier-stripped head nouns (`оқыту` from «машиналық оқыту»), then surfaced random poetry/contract quotes. **(1)** `detect_ask_about_system` extended for reflexive identity questions «Өзіңізді кім деп санайсыз?» / «Өзіңді қалай таныстырасың?» (marker: `өзіңді / өзіңізді` + 2nd-person verb), routed to `SystemAspect::General`. **(2)** New `discourse::find_adj_noun_compound` returns the longest matching closed-list compound (`машиналық оқыту`, `жасанды интеллект`, `табиғи тіл`, `терең оқыту`, `нейрондық желі` …) found in input — wired as the FIRST strategy in `best_noun_hint`. **(3)** New `SystemAspect::SelfComparison` (9th variant) + `system_self_comparison` slot + `ask_about_system.self_comparison` template family. Detector pairs comparison marker (`артық / жақсырақ / озасың`) with addressee marker including the `-сың/-сыз` ability suffix; honest framing — adam articulates the trade-off (narrow Kazakh-only competence with strong invariants vs. broad LLM coverage), not superiority. Closes «Басқа модельдерден несімен артықсыз?», «Қолданыстағы модельдерден қалай жақсырақ бола аласыз?». **(4)** New `discourse::strip_preamble` runs at the top of `Conversation::turn_with_trace` BEFORE FST parsing. Closed list of 24 leading preambles (`айтайын дегенім`, `қысқаша айтқанда`, `шынында`, `сұрағым мынау`, `жалпы алғанда`, `айтпақшы` …); when matched at input start with a clause separator after, the preamble is removed and the residual goes to the parser. Russian/math/anaphor detection still see the raw input. **(5)** New `Intent::UserAcknowledgement` (27th variant) + `user_acknowledgement` template family. Detector: addressee marker + 1sg perfective realisation verb (`түсіндім / білдім / көрдім / байқадым / ұқтым / аңғардым / сезіндім`) + not-a-question. Polite acknowledgement reply («рахмет, түсінгеніңізге қуаныштымын. Мен әлі дамып келемін …»). Closes «Мен сенің әлі бәрін білмейтініңді … түсіндім». 5 new e2e + 5 new REPL replay dialogs. `discourse.rs` helpers 3→**7**, SystemAspect variants 8→**9**, Intent variants 26→**27**, template families 57→**59**, REPL replay 63/63→**68/68**, workspace 734→**739**.

**v4.6.15 follow-up — 3 more innovations bundled (15 total cumulative on the v4.6.x minor): integer arithmetic calculator + `mathematics_basic` world_core domain + `informatics_basic` world_core domain.** User strategic ask: «необходимо дать ему знания школьной программы по математике и информатике … Он должен понимать диалог, того, что от него хотят». v4.6.12 detected math expressions and refused; v4.6.15 evaluates them deterministically and adds two new world_core domains so adam knows what the school terms *mean*. **(1)** New `discourse::try_evaluate_arithmetic` deterministic two-pass tokeniser/evaluator over `+ − × ÷ :` (`:` normalised to `/`), respects precedence, rejects non-integer results / division-by-zero / overflow. The conversation layer first attempts evaluation when the v4.6.12 math detector fires; on success the planner routes to a new `math_answer` template family. Closes from real-REPL: «5+5 → 10», «6:2= → 3», «12*4 → 48», «2+3*4 → 14» — pure stdlib `i64`, no external numeric library. **(2)** `mathematics_basic.jsonl` (37 entries / 37 facts): математика, сан, амал, қосу/азайту/көбейту/бөлу, теңдік, теңдеу, бөлшек, пайыз, алгебра/геометрия/тригонометрия, фигура, нүкте, түзу, бұрыш, шеңбер, дөңгелек, үшбұрыш/төртбұрыш/шаршы/тіктөртбұрыш/көпбұрыш, жұп/тақ/жай/бүтін/натурал сан, көбейту кестесі, аудан/көлем/периметр, шама, функция. **(3)** `informatics_basic.jsonl` (40 entries / 40 facts): информатика, ақпарат, дерек, алгоритм, бағдарлама, бағдарламалау тілі, компьютер, процессор, жад, файл, бит/байт, айнымалы/тұрақты, цикл/шарт/функция/жиым, деректер базасы, желі/интернет/сайт/шолғыш/сервер, кодтау/шифрлау/пароль, вирус/антивирус, операциялық жүйе, драйвер, қолданба, пернетақта/тінтуір/монитор/принтер/сканер. World Core **947 → 1032 entries / 1116 → 1195 facts across 30 → 32 domains**, derivations **22 387 → 22 962** (+575 via R1/R2/R5/R8 inheritance through new IsA hubs `ғылым / бағдарлама / құрылғы / арифметикалық амал / математикалық ұғым / геометриялық фигура`), `MULTIWORD_ENTITIES` +41 compounds, lexicon +3 loanword roots (информатика / компьютер / функция). Workspace **727 → 734**, REPL replay **62/62 → 63/63**, template families **56 → 57** (+ math_answer).

**v4.6.12 follow-up — 7 more innovations bundled (12 total cumulative on the v4.6.x minor).** Real-REPL 2026-04-29 (third transcript) surfaced 7 distinct issues. **(1)** AskHowAreYou +polite-plural «Қалыңыз қалай?». **(2)** Fix `greeting.intro_proposal` template grammar — pre-v4.6.12 4th variant said «сіз қалай танысамыз?» (2sg-polite + 1pl-future, ungrammatical), replaced with «сізді қалай атаймын?». **(3)** Russian-input refusal — new `discourse::input_is_likely_russian` detector + `__non_kazakh__` marker + `unknown.non_kazakh` template family. Real-REPL «Это очень круто, а кто тебя создал?» pre-v4.6.12 produced half-Russian half-Kazakh hybrid violating `project_kazakh_only_directive`. **(4)** Birthdate detector +verb forms (`қашан жаратты / қашан дамытты / қашан дамытқан / қашан дайындады`) mirroring v4.6.5 Creator extension. **(5)** AskAge +«неше жастасың/жастасыз» surface forms — adam-self age inquiries («Сіз неше жастасыз?») now route correctly. **(6)** Math-expression refusal — new `discourse::input_is_math_expression` detector (arithmetic ops near digits OR Kazakh math verbs + numerals) + `math_refusal` template family. Closes «5+5», «6:2=», «5-ті 7-ге көбейткенде», «алтыны екіге бөліңіз» (last one also resolves the алты/алтын homonym ambiguity by short-circuiting before topic extraction). **(7)** Closed-class case-suffix hygiene — bare suffixes `ге / ке / де / те / да / та / бе / ма` added to NOT_A_TOPIC. Workspace **715 → 727**, REPL replay **55/55 → 62/62**, cognitive 65/65 unchanged, template families **54 → 56** (+ unknown.non_kazakh, math_refusal).

**v4.6.5 follow-up — 5 innovations bundled: Creator detector +3 verbs + capitalization + period gate + Principles aspect + forbidden-pattern filter.** Real-REPL 2026-04-29 (second transcript) surfaced a new defect class + the user requested orthographic + value-articulation layers. **(1) Creator detector** extended with `жаратты` / `дамытқан` / `дамытты` / `дайындады` / `жаратушың` / `қай бағдарламашы` patterns — pre-v4.6.5 «Ал сені кім жаратты?» / «Сізді кім дамытқан?» / «Сізді қай бағдарламашы дайындады?» all fell through to refusal. **(2) Capitalization** — every reply now starts with an uppercase letter (sentence-case). New `capitalise_first_letter` orthographic pass in `realiser::realise` past leading whitespace/punctuation; Cyrillic-Kazakh-aware via `char::to_uppercase`. **(3) Sentence-final period** — declarative replies ≥10 codepoints ending in an alphabetic character now get `.` appended. Short interjections («Сәлем», «Иә») stay as-is. **(4) `SystemAspect::Principles`** — new variant + `principles_summary` field on `SystemIdentity` listing operational values adam upholds: respect humans, no fabrication, no incitement, privacy, no illegal-act assistance, audit trail, Kazakh-cultural respect, scope discipline. New `ask_about_system.principles` template family + detector for `принциптерің / ұстанымдарың / заңдарың / ережелерің / құндылықтарың`. Articulation layer — the underlying guarantees are already safe-by-construction. **(5) Forbidden-pattern filter** — new `ResponseQualityIssue::ForbiddenPatternLeak` defensive backstop in `audit_response`. Catches a regression that bypasses curation; not the primary safety mechanism (adam's retrieval-only design keeps that at the curation layer). Workspace **703 → 715**, cognitive **63/63 → 65/65**, REPL replay **50/50 → 55/55**, template families **53 → 54**, `SystemAspect` variants **7 → 8**.

**v4.6.0 milestone — self-awareness layer + discourse anaphora + closed-class hygiene.** Fourth v4.x minor. Real-REPL 2026-04-29 transcript surfaced 6 distinct defects + a strategic ask for self-awareness. All landed in one release. **(1) Self-awareness — three new SystemAspect variants** (`Capabilities`, `Knowledge`, `Limitations`) with corresponding template families (`ask_about_system.capabilities` / `.knowledge` / `.limitations`) and three new `SystemIdentity` summary fields rendered as honest, grounded Kazakh prose: «не істей аласың?» → adam lists what it can do (KZ morphology, slot recall, KZ geography knowledge, contradiction handling, audit trail); «не білесің?» → adam lists its world_core knowledge domains; «нені істей алмайсың?» → adam states its limitations (Kazakh-only, no novel generation, no online learning, no internet, no multimedia, no math). The Limitations detector is gated on an explicit interrogative marker so declarative criticism «сен ештеңе білмейсің» does NOT route here (preserves the v4.4.10 `qysqasy_discourse_particle` Tentative floor). **(2) Discourse anaphora** — new `crates/adam-dialog/src/discourse.rs` module + `last_query_topic` session slot. When the user's input contains a discourse anaphor («онда / сонда / осында / мұнда / бұнда / одан / содан / бұдан / осыдан»), the conversation layer **overrides** the current turn's `noun_hint` with the previous turn's topic. So «Қазақстан туралы не білесіз?» → «Ал онда қанша аймақ бар?» now answers with «Қазақстанның аймақтары — 17 облыс пен 3 республикалық маңызы бар қала» instead of «Он — сан». **(3) Compound self-introduction** — extended `detect_ask_about_system` to fire on `өзіңіз туралы айт` openers (real-REPL «Өзіңіз туралы айтып беріңізші»). **(4) Closed-class hygiene** — added `өте` (intensifier "very") and `жалпы` (in-general adverb) to NOT_A_TOPIC; added bare numeral roots `он` / `сон` to filter the FST misanalysis of `онда / сонда` as `Locative(он/сон)`. **(5) New world_core landmarks fact** — «Қазақстандағы көрікті жерлер мен табиғи орындар: Бурабай, Шарын каньоны, Хан Тәңірі, …» surfaced when user asks «Қазақстанда қандай көрікті жерлер бар?». World Core **947 → 949 entries / 1116 → 1120 facts** (+1 landmarks list + 1 area-quantity fact). Cognitive eval **59/59 → 63/63**, REPL replay **43/43 → 50/50**, workspace **693 → 703**. Why minor: 3 new SystemAspect enum variants + 1 new module (`discourse.rs`) + 1 new session-state slot — multiple architectural type-system additions.

**v4.5.0 milestone — `Case::LocativeAttributive` FST morphotactics rule.** Closes the v4.4.12/13 carry-forward by replacing the v4.4.12 string-side fallback with a proper morphotactics rule. New `Case::LocativeAttributive` enum variant in `crates/adam-kernel-fst/src/morphotactics.rs` + new `LOCATIVE_ATTRIBUTIVE` suffix template `-{D}{A}{G}{I}` covering all four allomorphs (`-дағы` back-voiced, `-дегі` front-voiced, `-тағы` back-voiceless, `-тегі` front-voiceless) via the existing archiphoneme machinery. Pronominal-н buffer rule extended to apply on P3 + LocativeAttributive. The parser's `try_noun_analyses` enumerates the new case, so analyse() reverse-parses surface forms back to the base noun. **Full round-trip verified**: `synthesise(қазақстан, LocativeAttributive)` → `қазақстандағы`; `analyse(қазақстандағы)` → `Noun(қазақстан, LocativeAttributive)`. CLI gained `--case locattr`. The v4.4.12 string-side `locative_attributive_hint` stays in place as a backstop for stems whose base form isn't yet in the lexicon. Why minor: new code-level Case variant + new morphotactics rule + round-trip synthesis support — architectural addition by the post-1.0 cadence rule. Workspace **692 → 693**.

**v4.4.13 follow-up — lexicon hygiene patch.** Two long-standing FST/lexicon defects, surfaced during the v4.4.12 trace and queued at the time. **(1) Multi-POS homonym dedup.** `Lexicon::load` deduplicated by surface root via a `HashMap<String, RootEntry>`, so multi-POS homonyms like `тау` (verb_tau + noun_apt_tau, both keyed on the same surface) silently lost one reading and only the last-inserted one survived in `entries_ordered` — the FST analyser's iteration source. Result: `тау` parsed only as a verb root, never as the noun "mountain". **Fix**: separate `entries_ordered` (full union of both lexicon files, deduplicated only by `id` to handle exact-copy entries) from `by_surface` (intentionally lossy single-POS lookup table for downstream code that doesn't care about POS). **(2) Missing core nouns.** Audit found `су` (water), `от` (fire), `ер` (saddle / man-as-hero) absent from both lexicon files entirely. Added to `data/tokenizer/segmentation_roots.json`. **(3) `best_noun_hint` chain reorder** — `locative_attributive_hint` was running AFTER `first_noun_root` (correct as a fallback in v4.4.12), but v4.4.13's lexicon-dedup fix unblocked content nouns like `тау` so `first_noun_root` started masking the locative-attributive signal on questions like `Қазақстандағы таулар қандай?`. Reordered so locative-attributive runs first — `-дағы` is a strong topic-narrowing signal semantically equivalent to a `туралы` marker for the word it attaches to. All 5 listing-style questions now answer correctly with both locative (`Қазақстанда қандай таулар бар?`) and locative-attributive (`Қазақстандағы таулар қандай?`) phrasings. Cognitive **59/59 → 59/59** (no scenario added — the locking is at REPL replay layer), REPL replay **40/40 → 43/43** (+3 listing-via-locative-attributive dialogs), workspace **690 → 692** (+2 lexicon e2e).

**v4.4.12 follow-up — locative-attributive `-дағы / -дегі / -тағы / -тегі` suffix recovery.** Closes the v4.4.11 carry-forward where `Қазақстандағы таулар қандай?` fell through to the generic refusal because the FST morphotactics has no rule for the locative-attributive derivation (Kazakh forms «located in X» attributives by attaching `-ғы / -гі / -қы / -кі` to the locative-cased stem). The FST returns no analysis for `қазақстандағы`, so `best_noun_hint` recovered nothing. v4.4.12 added a string-level `locative_attributive_hint` fallback in `crates/adam-dialog/src/semantics.rs` that scans tokens for the four allomorphs, strips the 4-char tail, and recovers the base noun (≥ 3 codepoints, not in NOT_A_TOPIC). Combined with v4.4.11's input-overlap reranker the question now answers with the literal mountains list. The fallback is conservative — promoted to a proper `Case::LocativeAttributive` morphotactics rule in a future minor. Cognitive **58/58 → 59/59**, REPL replay **39/39 → 40/40**, workspace **688 → 690**.

**v4.4.11 follow-up — input-overlap retrieval reranker + list-summary renderer fix.** Closes the v4.4.10 carry-forward where listing-style questions («Қазақстан аймақтарының атауларын білесіз бе?», «Қазақстанда қандай көлдер бар?») retrieved the most-central «Қазақстан — Орталық Азиядағы ел» IsA fact instead of the specific list-summary entries. Two-part fix: (1) **input-morpheme-overlap reranker** in `Tool::dispatch(SearchGraph)` — `ToolContext` gained a `query_input: Option<&str>` field which the rank function uses to score each fact's `raw_text` against the user's content tokens (4-char prefix substring match handles agglutinative inflection); higher overlap wins, ties fall through to the v4.0.x predicate-rank tier (IsA → LivesIn → HasQuantity → …). (2) **list-summary RelatedTo renderer** in `tool.rs::render_grounded_fact` — when the fact's object root contains «тізім», surface `raw_text` directly instead of the generic «X мен Y өзара байланысты» template. All 5 v4.4.10 carry-forward listing questions now answer with literal lists: aliases / lakes / rivers / mountains / deserts. Cognitive **57/57 → 58/58**, REPL replay **35/35 → 39/39**, workspace **687 → 688**.

**v4.4.10 follow-up — Kazakhstan administrative + physical geography expansion + `Танысайық` intent + `Қысқасы` topic-marker guard.** Real-REPL transcript on 2026-04-28 surfaced 5 distinct issues. Three knowledge gaps fixed by authoring **76 new world_core entries** in `data/world_core/geography_kz.jsonl` (873 → **947 entries**, 995 → **1116 facts**, 21 415 → **22 387 derivations** from new `өзен/көл/теңіз IsA су денесі`, `тау/шөл/каньон IsA жер бедері`, `облыс IsA әкімшілік бөлік`, `қала/ауыл IsA елді мекен` bridge facts): all **17 Kazakh oblasts** as administrative entities (`Абай`, `Ақмола`, `Ақтөбе`, `Алматы`, `Атырау`, `Батыс Қазақстан`, `Жамбыл`, `Жетісу`, `Қарағанды`, `Қостанай`, `Қызылорда`, `Маңғыстау`, `Павлодар`, `Солтүстік Қазақстан`, `Түркістан`, `Ұлытау`, `Шығыс Қазақстан`), oblast → administrative-center mappings (Семей→Абай, Көкшетау→Ақмола, …, Қонаев→Алматы (post-2022 reform), Петропавл→СҚО, Жезқазған→Ұлытау, Түркістан→Түркістан oblast), 3 cities of republican significance upgrade (Астана / Алматы / Шымкент `IsA республикалық маңызы бар қала`), country + 3-republic-city population facts (~20 / ~2 / ~1.5 / ~1.2 млн), 6 new rivers (`Жайық`, `Есіл`, `Тобыл`, `Шу`, `Қаратал`, `Талас`), 4 new lakes (`Зайсан`, `Алакөл`, `Тенгіз`, `Маркакөл`), 5 mountains (`Тянь-Шань`, `Жетісу Алатауы`, `Хан Тәңірі`, `Қаратау`, `Ұлытау`), 4 deserts (`Бетпақдала`, `Қызылқұм`, `Үстірт`, `Мойынқұм`), `Шарын каньоны`, `Бурабай`. Two dialog issues: new `GreetingKind::IntroProposal` + `greeting.intro_proposal` template family routes `Танысайық` to a self-introduction reply (was falling through to safe-fallback refusal); NOT_A_TOPIC additions for `қысқа` / `ештеңе` / `ешкім` / `ешбір` / `еш` close the discourse-particle / quantifier-pronoun topic-extraction misfires. Cognitive eval **55/55 → 57/57**, REPL replay **31/31 → 35/35**, workspace **683 → 687**. The repl_replay harness gained runtime artefact loading (facts.json + derived_facts.json + morpheme_index.json) so retrieval-dependent dialogs reach the same code path as production `adam_chat`.

**v4.4.9 follow-up — AskName 1sg self-recall + slot-echo aspirationals promoted.** (1) `менің атым кім?` after `менің атым Дәулет` was misclassified as `StatementOfName { name: "Кім" }` pre-v4.4.9 — `detect_statement_of_name`'s "атым X" pattern blindly grabbed the question word `Кім` as a name, then logged a phantom `BeliefConflict` (Дәулет vs Кім) and emitted a clarifying question naming both. Symmetric to v4.4.5 / v4.4.6 self-recall fixes but worse: belief got mutated, not just surface text. Fix: interrogative-pronoun guard in `detect_statement_of_name` (refuses `кім / не / қандай / қайсысы` across all three patterns) + extended `detect_ask_name` (accepts 1sg `атым / есімім + кім / не`). The REPL replay battery surfaced this on v4.4.6 first run. (2) Promoted all 3 aspirational REPL replay dialogs (`city_statement_acknowledged`, `age_statement_acknowledged`, `occupation_statement_acknowledged`) to canonical by rewriting the 5 bare variants in `statement_of_*` template families to interpolate their slot. REPL replay baseline 27/27 + 3 → **31/31 + 0 aspirational**, cognitive eval 54/54 → **55/55**, workspace 681 → 683. Performance regression policy clarified: thermal throttling on the M2 8 GB can uniformly elevate p50 by ~70 % under sustained `cargo` load; comparison must be apples-to-apples on thermal state (proven environmental in v4.4.9 via stash-and-re-bench).

**v4.4.7 follow-up — performance baseline.** Per-turn latency, cold-start cost, and RSS measured on M2 8 GB: **p50 1.07 ms** (`сәлем`) → **6.04 ms** (3-turn dismiss-contradiction dialog), cold start **~14 ms**, max RSS **~76–80 MB** depending on the metric (`/usr/bin/time -l` reports `maximum resident set size` ≈ 80 MB and `peak memory footprint` ≈ 76 MB on the same run). Honest "when adam, when LLM" comparison block: latency / memory delta is 100×–2 000× vs a local LLM, but only meaningful **inside adam's competence envelope** — Kazakh dialog intents recognised by the recogniser, slots filled from FST parses or curated entities, knowledge queries that hit `world_core` or the retrieval shards. Outside that envelope adam refuses or admits uncertainty; it does not fabricate. Numbers and methodology in [docs/performance.md](docs/performance.md); reproduce with `cargo bench -p adam-dialog --bench turn_latency`. Performance regressions > 20 % p50 are release blockers per [CONTRIBUTING.md](CONTRIBUTING.md).

**v4.4.6 follow-up — REPL replay battery + AskOccupation 1sg self-recall.** New `crates/adam-dialog/tests/repl_replay.rs` harness running `data/eval/repl_dialogs.json` (30 hand-authored multi-turn KZ dialogs) — complementary to `cognitive_eval` (which checks trace signals); this checks what the user actually sees. Same `expected_failing` aspirational contract. Current baseline **27/27 canonical + 3 aspirational** (the 3 aspirational document a real `statement_of_*` slot-echo gap; promotion-ready when every variant interpolates the slot). Plus a v4.4.5-class detector extension surfaced by the harness on first run: `менің мамандығым не?` now classifies as `AskOccupation` and recalls the stored value via `ask_occupation.with_known_user`. New `CONTRIBUTING.md` codifies the load-bearing rule that's been informal for several releases — every dialog defect ships with at least one new scenario / dialog.

**v4.4.5 follow-up — real-dialog adequacy fixes.** External review (Codex, 2026-04-27 live REPL) caught two user-visible defects the internal suite missed: (1) `Action::CheckContradiction` rendered as a confirmation because the planner kept keying on `intent_key(intent)` instead of the action — answer was «Алматыда екеніңізді есте сақтаймын» where it should have been a clarifying question; (2) `менің жасым қанша?` misclassified as `StatementOfAge` because the detector keyed on substring `жасым` and ran before `detect_ask_age`. Both fixed via a new `check_contradiction` template family + planner override and a question-particle guard with reordered detector dispatch. The cognitive contour was already correct in v4.4.0 — only the surface text and 1sg-self-recall classification leaked.

**v4.4.0 milestone — belief-poisoning recovery.** A v4.3.2 follow-on: once `BeliefState.contradictions` was non-empty for *any* reason — real conflict, transient typo, or upstream parse glitch — the planner clamped every subsequent turn to `CheckContradiction` with no clean exit. v4.4.0 adds two complementary escape hatches:

- **Explicit user-initiated dismissal.** New `BeliefState::dismiss_contradiction` (symmetric to v4.1.0 `resolve_contradiction`) supersedes both contested facts and clears the pending question. `Conversation::try_dismiss_pending_contradiction` fires on nine KZ phrases (`екеуі де жоқ`, `білмеймін`, `ұмыт`, …) — runs *before* resolution so a "білмеймін" doesn't accidentally pick a candidate. New `Action::DismissContradiction` variant + `dismiss_contradiction` template family confirm the dismissal in plain Kazakh.
- **Implicit time-bounded priority cap.** New `ActionPlanner::CONTRADICTION_PRIORITY_TURNS = 3` + `plan_with_turn(...)` API. A conflict logged at turn `T` dominates `T`/`T+1`/`T+2`; on `T+3` it falls through to normal action paths. The conflict stays in `belief.contradictions` for audit — only planner priority changes.

**v4.3.0 milestone — language core, typed evidence, and ontology gates.** Three architectural layers landed in tandem:

- **Language Core** (`crates/adam-dialog/src/language_core.rs`) — orthography, mixed-script Latin/Cyrillic cleanup, proper-noun normalization, and **canonical entity resolution** for geography. Place mentions like `Алма-Ата`, `Усть-Каменогорск`, `Каспий теңізі` resolve to stable `geo_kz_NNN` ids from `data/world_core/geography_kz.jsonl`. Memory now stores entities by canonical id, not surface string.
- **Typed Evidence** (`ToolEvidence` in `crates/adam-dialog/src/tool.rs`) — `ToolResult` now carries machine-readable `Vec<ToolEvidence>` alongside textual `findings`, with variants `BeliefFact` / `GraphFact` / `RetrievalSample` / `DerivedFact { rule_id, support_chain }`. Higher layers verify *which typed claim* justifies a user-facing answer.
- **Ontology Gates** (`crates/adam-reasoning/src/ontology.rs`) — type constraints on admissible facts (rule-predicate match, place-object validation for spatial predicates, time-like validation for temporal predicates). Graph and reasoner consumers reject structurally invalid facts before verbalisation.
- **Quality Module** (`crates/adam-dialog/src/quality.rs`) — deterministic response-quality gate that audits every reply for placeholder leaks, Latin debug artifacts, surface-vs-trace faithfulness, and typed-evidence faithfulness.

**v4.2.0 milestone**: the dialog turn is a **data-driven tool-loop interpreter**. `Conversation::turn_with_trace` builds a `Vec<ToolCall>`, dispatches in one uniform pass, folds results back through `apply_tool_results`. The `inject_*` framing is retired; adding a new tool consult is a one-line append.

**v4.1.0 milestone**: cognitive eval reaches **22/22 canonical, 0 aspirational**. Kernel detects contradictions across turns, asks the user for resolution, revises belief state with full audit trail.

Three things make the trade viable specifically for Kazakh:

- **Agglutinative advantage** — Kazakh's rich morphology means the FST unpacks each word into a typed bundle (root + case + number + possessive + predicate-person), which the retrieval index and reasoner both exploit. What would be a 10⁶-parameter subword model in English is a 14 k-root Lexicon + deterministic rules here.
- **Mathematical determinism** — same input + same session + same seed produces a byte-identical answer across runs. No temperature, no sampling, no GPU.
- **No ungrounded generation by design** — every output is either a template realisation, a corpus quote, or a rule derivation with a full `source_chain`. There is no free-text generator anywhere in the pipeline that could invent content not traceable to its source.

| | adam v4.3 | mainstream LLM |
|---|---|---|
| Outputs | template + verbatim quote + FST synthesis + **rule-derived chain** | probabilistic token generation |
| Ungrounded generation | **none by construction** (retrieval quotes verbatim; reasoner derives only from typed facts) | non-zero, non-auditable |
| Inference | ms on laptop CPU | dollars on GPU / datacentre |
| **Reasoning** | **forward-chaining over typed facts, every conclusion has a `rule_id`** | opaque emergent reasoning |
| **Belief revision** | **explicit `BeliefState` with `Active`/`Superseded`/`Contested` lifecycle; user-driven contradiction resolution (v4.1.0)** | implicit, untraceable across turns |
| **Canonical entities (v4.3.0)** | **stable `geo_kz_NNN` ids resolved from `world_core/geography_kz.jsonl`; `Алма-Ата` / `Каспий теңізі` / typo aliases all collapse to one canonical record** | string-keyed; memory drift across surface forms |
| **Typed evidence (v4.3.0)** | **`ToolResult.evidence: Vec<ToolEvidence>` carries `BeliefFact` / `GraphFact` / `RetrievalSample` / `DerivedFact { rule_id, support_chain }` per dispatch — every claim is auditable to its source class** | text-only, no machine-readable provenance |
| **Ontology gates (v4.3.0)** | **`adam_reasoning::ontology` validates rule-predicate matches, place-object types for spatial predicates, time-like types for temporal predicates** | none |
| **Response-quality audit (v4.3.0)** | **`audit_response` + `audit_trace_faithfulness` + `audit_typed_faithfulness` reject placeholder leaks, Latin debug artifacts, surface-vs-trace divergence, evidence-class mismatches** | none — generation isn't introspected |
| **Provenance** | **`source_chain: Vec<FactSource>` per derivation; `(pack, sample_id)` per quote; `Provenance::UserStatement { turn_id }` per belief fact; `EntityMemory.canonical_id` per remembered place** | ~none for free-form output |
| **Inference marker** | **«байланыс-» on every reasoned claim, test-enforced** | — |
| **Stack policy** | **Rust-only + graph-first, contract-test enforced (no Python/JS/TS, no external graph DB, no Cypher/SPARQL)** | typically polyglot Python + neural runtime |
| Determinism | byte-identical across runs for same `(input, session, seed)` | temperature-dependent |
| Language coverage | Kazakh only | many, but shallow for low-resource |
| Knowledge depth | bounded by curated corpus + deterministic rules | broad, but fabricated edges |
| Self-improvement | ships by commit, reviewed by humans | parametric updates through training |

adam is **intentionally narrower** than an LLM. In return it is **predictable, cheap, safe, auditable, and — as of v4.1.0 — capable of holding conflicting beliefs simultaneously, surfacing them to the user, and revising them on demand**, while every conclusion carries a textual trust marker and every fact carries a source chain.

### Rust-Only Policy

The implementation language of `adam` is **Rust only**.

- No Python runtime.
- No Node / TypeScript runtime.
- No auxiliary model code in a second language.
- If the project needs a graph engine, verifier, trainer, retrieval index, or any other subsystem, it is either sourced from the Rust ecosystem or written in Rust inside this repository.
- POSIX shell wrappers in `scripts/` are allowed only as thin launchers around `cargo run` / `cargo test`; they must not introduce a second execution runtime.

This is a project invariant, not a preference. The repository carries contract tests that fail if non-Rust source files or foreign-language runtime invocations are introduced.

### Graph-First Policy

The graph layer of `adam` is **Rust-native and repository-native**.

- No external graph database as a required runtime.
- No Cypher / Gremlin / SPARQL query layer in the core pipeline.
- No Python graph stack (`networkx`, `igraph`, `graph-tool`) hidden behind scripts.
- The canonical graph representation, traversal, and artifact builders live in Rust crates and Rust binaries inside this repository.
- Shell scripts may orchestrate graph builds only as thin wrappers around `cargo run`.

This is also a repository invariant. Contract tests fail if a foreign graph stack is introduced or if the canonical Rust graph entrypoints disappear.

### Current state (v4.4.7 — honest numbers, verified 2026-04-27)

The cognitive contour shipped through v4.3.0 (language core, typed evidence, ontology gates, response-quality audit, stack policies) and v4.4.0 (belief-poisoning recovery: `Action::DismissContradiction` + contradiction-priority cap) remains the architectural baseline. v4.4.5–v4.4.7 added real-dialog adequacy fixes (`check_contradiction` template family + age/occupation 1sg-self-recall), the REPL replay battery, `CONTRIBUTING.md`, and a measurement / regression-policy layer. No new architectural layer in the v4.4.x patch series — those land at minor bumps.

Live numbers (verified 2026-04-29 against the actual repo): cognitive eval **65 / 65 canonical, 0 aspirational**. REPL replay: **69 / 69 canonical + 0 aspirational**. World Core: **1142 entries / 1305 curated facts across 33 domains**. Reasoner: **10 of 11 rules firing** with **23 418 derived facts** over **15 831 extracted + curated facts**. Morpheme index: **3 362 morphemes / 22 145 postings / 3 691 indexed samples** (incl. ~499 sentences across chapters 1–7 of the Rust Book Kazakh translation; rust_book pack at the 500-per-pack default-mode ceiling). Workspace: **745 tests passing**, 0 warnings. SystemAspect variants: **9** (General / Creator / Birthdate / Architecture / Capabilities / Knowledge / Limitations / Principles / SelfComparison). Template families: **59**. FST: **8 cases** (7 inflectional + 1 derivational LocativeAttributive at v4.5.0) × 2 numbers × 7 possessives × 7 predicate-person copulas; full synthesise / analyse round-trip across all four `-дағы / -дегі / -тағы / -тегі` allomorphs. Lexicon: **~25.5 k roots** (13 606 pure Kazakh + 11 919 Apertium imports). Every curated fact carries `ConfidenceKind::HumanApproved` with a named reviewer; every derivation has a `rule_id` + non-empty `source_chain`; every belief fact carries `Provenance` + `FactStatus`; every remembered place carries `EntityMemory.canonical_id`; every dialog turn's lookups are declared as `ToolCall`s and recorded as `ToolResult`s with typed `ToolEvidence` on `TurnTrace.tool_calls`. Nothing ungrounded leaves the deterministic recognised / grounded runtime path.

#### Verified-on-2026-04-27 quick reference

| Claim | Verified value | Verification path |
|---|---|---|
| Workspace tests | **727 passing, 0 failing, 4 ignored** | `cargo test --workspace` |
| Cognitive eval canonical | **59 / 59** | `cargo test -p adam-dialog --test cognitive_eval` |
| REPL replay | **62 / 62 canonical + 0 aspirational** | `cargo test -p adam-dialog --test repl_replay` |
| World Core entries / facts / domains | **1142 / 1305 / 33** | `find data/world_core -name '*.jsonl' \| xargs cat \| jq -s 'length'` |
| Extracted runtime facts | **15 642** | `jq '.counts.facts_total' data/retrieval/facts.json` |
| Derived facts | **23 418** | sum of `data/retrieval/derived_facts.json` `.counts.by_rule` values |
| Template families | **50** | `grep -c '^\[\[families\]\]' data/dialog/templates/v1.toml` |
| Tokenizer segmentation eval | **464 / 464 hand-authored** | `data/eval/tokenizer_segmentation_eval_dataset.json` (this is a hand-authored coverage eval, **not** a general "Kazakh tokenizer accuracy" benchmark) |
| Tiny training validation | **15 / 15 next-token checks on tiny clean prototype** | `data/training/baseline_training_manifest.json` (this is a clean-pipeline prototype check, **not** an ML-model accuracy claim) |
| `data/eval/benchmark_manifest.json` | **coverage / contract benchmark manifest** with 4 task families + guards + layers | not a single AI-benchmark score; see `docs/foundation_scope.md` for scope |
| Scaling report | T5 target was 1 M, scanned **940 288** before `status: "timed_out"` | `data/scaling/scaling_report.json`; useful as a scaling artefact, **not** a "1 M benchmark completed without caveat" |
| Per-turn p50 latency | **1.07 ms → 6.04 ms** by scenario class | `cargo bench -p adam-dialog --bench turn_latency` (M2 8 GB, `--release`) |
| Cold-start (lexicon-dominated) | **~14 ms** | same bench file — `cold_start_lexicon` ≈ 13.32 ms |
| Max RSS, one-shot dispatch | **~76–80 MB** depending on metric | `/usr/bin/time -l ./target/release/adam_chat --once "сәлем"` reports `maximum resident set size` ≈ 80 MB and `peak memory footprint` ≈ 76 MB |
| Hallucination contract | **zero ungrounded generation inside the deterministic recognised / grounded runtime path** (refusal or `unknown.tentative` outside the envelope) | `crates/adam-dialog/src/quality.rs::audit_response` + `audit_typed_faithfulness` + `audit_trace_faithfulness` + `audit_graph_admissibility` |

| | value |
|---|---|
| Dialog intents | 26 |
| Lexicon roots | **~25.5 k** (13 606 pure Kazakh + 11 919 Apertium imports, before deduplication) |
| Corpus (committed / local) | **4.57 M** (v3.5.0: 10 textbooks) / 77.9 M words across 9 committed source packs |
| **World Core** | **1142 entries / 1305 curated facts across 33 domains** (v4.7.0 added 110-entry `programming_rust` Kazakh glossary + corpus-purity carve-out for backtick-quoted technical text; v4.6.15 added 37-entry `mathematics_basic` + 40-entry `informatics_basic`; v4.3.5 added kz_literature surname keyings + new `notable_kazakhstanis` domain; v4.4.10 added 73 entries to `geography_kz` covering all 17 oblasts + admin centers + republic-cities + populations + 6 rivers + 4 lakes + 5 mountains + 4 deserts + canyon + Бурабай + 9 IsA-hub bridge facts): animals, astronomy, biology_basic, body_parts, clothing, colors, constellations_kz, cooking_methods, directions, emotions, food, geography_kz, house_parts, informatics_basic, kinship_extended, kz_literature, language_features, materials, mathematics_basic, measurements, music_kz, notable_kazakhstanis, numbers, plants, professions, proverbs, society, sports, time, tools_household, transport, weather_phenomena. All `approved` by `shaman`. Schema + validator: `data/world_core/README.md` |
| Morpheme coverage over committed corpus | 79.48 % |
| Workspace tests | **727 passing, 0 failing, 4 ignored** |
| **Cognitive eval baseline** | **65 / 65 canonical, 0 aspirational** (v4.4.12). Closed scenarios: parse-failure distinction (v4.0.40), contradiction resolution (v4.1.0), AnswerDirect rendering + digit-token (v4.2.5), multi-slot lifecycle / compound flows (v4.2.6), self/other distinction + SystemIdentity (v4.3.3–4), topic-marker extraction + famous Kazakhs (v4.3.5), belief-poisoning recovery (v4.4.0), CheckContradiction renderer + AskAge self-recall (v4.4.5), AskName 1sg self-recall + interrogative-pronoun guard (v4.4.9), `Танысайық` IntroProposal + `Қысқасы` discourse-particle guard (v4.4.10). Tracked in `data/eval/cognitive_dialog_dataset.json`; harness in `crates/adam-dialog/tests/cognitive_eval.rs` |
| **REPL replay baseline (v4.4.6 → v4.4.13)** | **43 / 43 canonical, 0 aspirational** (v4.4.13). v4.4.13 added 3 locative-attributive listing dialogs (rivers, lakes, deserts) on top of v4.4.12's mountains dialog; v4.4.11 had added 4 listing-question regression dialogs against the locative phrasing. The harness loads runtime artefacts (facts.json + derived_facts.json + morpheme_index.json) so retrieval-dependent dialogs reach the same code path as production `adam_chat`. Tracked in `data/eval/repl_dialogs.json`; harness in `crates/adam-dialog/tests/repl_replay.rs` |
| **Performance baseline (v4.4.7, M2 8 GB)** | Per-turn p50 **1.07 ms → 6.04 ms** by scenario class; cold start **~14 ms** (lexicon load dominates); max RSS **~76–80 MB** for `./target/release/adam_chat --once "сәлем"` with full retrieval index + 21 415 derived facts loaded; throughput ~900 / ~400 / ~200 turns/sec single-thread by class. Numbers + methodology + honest "when adam, when LLM" tradeoff block in `docs/performance.md`; reproduce with `cargo bench -p adam-dialog --bench turn_latency` |
| **Language Core (v4.3.0)** | `crates/adam-dialog/src/language_core.rs` — orthography + mixed-script Latin/Cyrillic cleanup + proper-noun normalization + canonical entity resolution. `canonical_geo_entity(surface)` → `GeoEntity { id, canonical, kind }` resolved from `data/world_core/geography_kz.jsonl`. Place mentions like `Алма-Ата`, `Усть-Каменогорск`, `Каспий теңізі` collapse to one stable `geo_kz_NNN` id. `EntityMemory.canonical_id` carries the id through belief; session has `city_id` + `geo_kind` alongside `city` |
| **Typed Evidence (v4.3.0)** | `ToolResult.evidence: Vec<ToolEvidence>` carries machine-readable claims alongside textual `findings`. Variants: `BeliefFact { subject, predicate, object }`, `GraphFact { subject, predicate, object, confidence, rendered }`, `RetrievalSample { text }`, `DerivedFact { subject, predicate, object, rule_id, confidence, rendered, support_chain }`. Used by `audit_typed_faithfulness` to verify the user-facing answer is backed by the evidence class the planner intended |
| **Ontology gates (v4.3.0)** | `crates/adam-reasoning/src/ontology.rs` — type constraints on admissible facts. `validate_fact` / `validate_derived_fact` reject `RulePredicateMismatch`, `PlaceObjectRequired` (spatial predicates need place-typed objects), `TimeLikeRequired` (temporal predicates need time-typed objects), `EmptySupportChain`, `SupportPatternMismatch`, `MissingSupportSource`. Graph admissibility audited via `audit_graph_admissibility` |
| **Response-quality audit (v4.3.0)** | `crates/adam-dialog/src/quality.rs` — `audit_response` rejects empty / placeholder-leaked / Latin-debug-artifact / double-space replies. `audit_trace_faithfulness` verifies surface text matches the trace's chosen action + evidence. `audit_typed_faithfulness` verifies the surfaced answer comes from the right `ToolEvidence` class |
| **Belief revision (v4.1.0)** | `BeliefState` with `Active`/`Superseded`/`Contested` lifecycle, `BeliefConflict` log, `ContradictionToResolve` pending-question lifecycle. `resolve_contradiction(subject, predicate, chosen_object)` flips chosen → Active, others → Superseded, drops the matching conflict + pending question. Single-active-fact invariant (v4.0.28) preserved across resolution; nothing is ever deleted |
| **Stack policies (v4.3.0)** | **Rust-only**: no Python / Node / TypeScript / other-language source files in the repo (contract-tested in `crates/adam-eval/tests/rust_only_contracts.rs`). **Graph-first**: graph layer is Rust-native, no external graph DB, no Cypher / Gremlin / SPARQL (contract-tested in `crates/adam-eval/tests/graph_first_contracts.rs`). POSIX shell scripts in `scripts/` are thin wrappers around `cargo run` / `cargo test` only |
| Pattern matchers | **11** — v2.x baseline (4) + v3.5.0 (6) + v3.5.5 structural_part_of, all behind v3.9.0's `is_fragment_root` central hygiene gate |
| **Reasoning rules active** | **10 of 11 firing on v4.4.10 corpus** — R1 IsA-transitivity (**735**), R2 Has-inheritance (**1 160**), R3 Has-via-PartOf (**55**), R5 shared-IsA → RelatedTo (**18 226**), R6 LivesIn-via-PartOf (**88**), R7 GoesTo-via-PartOf (**544**), R8 After-transitivity (**999**), R9 PartOf-transitivity (**305**), R10 InDomain-inheritance (**124**), R11 InDomain-shared-target (**151**). R4 IsA-symmetry is curator-warning only. v4.4.10 +972 derivations from new bridge facts (`өзен/көл/теңіз IsA су денесі`, `тау/шөл/каньон IsA жер бедері`, `облыс IsA әкімшілік бөлік`). |
| Predicates defined | **11** — IsA, LivesIn, Has, GoesTo, PartOf, RelatedTo, Causes, After, HasQuantity, DoesTo, InDomain |
| **Dialog closed-class sync** (v3.9.5) | `NOT_A_TOPIC` mirrors `adam_reasoning::patterns::is_closed_class` — closes the pre-v3.9.5 «Неліктен → Нелікте тұрасыз ба» misparse where the FST correctly analysed `Неліктен` as ablative of a noun stem but the dialog layer had no interrogative filter |
| **Lexicon gap candidates queued for review (v3.4.0)** | **200** pre-tagged roots in `docs/lexicon_gap_candidates.md` (top-ranked of 104 657 distinct uncovered surfaces across the 4.32 M-word committed pool) |
| Facts (committed runtime) | **15 831 total** = **14 526 extracted (Grammar)** + **1305 curated (HumanApproved, 33 domains)**. T4_200k scale |
| **Rule-derived facts (committed runtime)** | **23 418 derivations** (10 active rules; numeric breakdown in the rules row above) |
| Fact-graph nodes / edges | **3 515 / 13 725** (committed v4.0.20); most-connected content nouns scaled with Lexicon sync |
| **Tooling throughput (v4.0.8 → v4.0.9 validation)** | `extract_facts --world-core-only` — v4.0.8 infra. v4.0.9 confirmed empirically: 3-domain batch (105 new facts, full rebuild of facts + derived_facts + lexical_graph) took **~4 s total** vs ~135 min under the pre-v4.0.8 per-domain workflow — **~2 000× pipeline speedup on a 3-domain batch**. |
| **Predicate coverage (v3.9.5)** | **11 / 11 = 100 %** — every declared predicate fires. Causes = 6, InDomain = 5 (v3.9.5 biology/anatomy/society entries extended the v3.9.0 foothold) |
| Iteration harness (v3.1.0) | `--time-budget <SEC>`, `--progress-interval <SEC>`, SIGINT→graceful-commit; Rayon par_iter on extract hot loop |
| Scaling bench (v3.3.0) | `adam-scaling::scaling_bench` + `audit_precision` — emits `data/scaling/scaling_report.json` + `docs/scaling_report.md` + `docs/precision_audit.md`. Budget-aware `run_tier_with_budget` (chunked at 128 samples, SIGINT / `--time-budget` stops within ~1 s). Normalized metrics per tier: `facts_per_10k_words`, `derivations_per_fact`, `predicate_coverage_pct`, `duplicate_fact_rate_pct`. **Measured scaling on 4.32 M-word committed pool (textbooks + wiki + Abai)**: T3_10k (19 facts, 0 deriv) → T4_50k (120 facts, 51 deriv) — reasoning activates once graph density crosses threshold. |
| Determinism (v3.2.0 + v3.3.0) | dual-storage Lexicon (`HashMap` get + `entries_ordered: Vec<RootEntry>` for `analyse`). Fixes a 2-year latent non-determinism where `analyse().next()` returned different first analyses across runs for ambiguous surfaces. **4 regression tests** guard the invariant, including expected-order assertions that fail ≈ 50 % on pre-v3.2.0 code. |
| Lexicon mining (v3.4.0) | `adam-corpus::mine_lexicon_gaps` scans all 9 committed packs, finds uncovered tokens, ranks globally by frequency, auto-tags (vowel harmony + final-sound class), extracts 3 context sentences per candidate. Produces `docs/lexicon_gap_candidates.md` for native-speaker review. First scan: top-5 candidates **validated against the v1.5.5-era `project_morpheme_coverage_baseline` memory** — exact match on all 5 predicted gaps (`деп, оның, осы, деген, пен`). |
| Gold corpus (v3.3.0) | 3 Kazakh secondary-school textbooks OCR'd via tesseract-kaz @ 200 DPI (pdftotext drops Қ/Ң/Ғ/Ө/Ү/Ұ/Һ on custom-font PDFs). **108 913 raw words → 8 421 samples** in `kazakh_textbooks_pack.json`, per-book provenance. 7 more textbooks staged for v3.4. |

The scale-up path is explicit: scale coverage of the four existing matchers to the full 77.9 M-word corpus, add `PartOf` / `Causes` extractors, activate R3/R4. Nothing in the architecture is gated on more data — the engine already produces derivations with full provenance.

### Trust stack

```
 template realisation              →  recognised intent, 0 % fabrication
 verbatim quote «…»                →  corpus citation, byte-identical to source
 «бейімд-» adaptation marker       →  quote was rewritten (v1.9.5)
 «байланыс-» reasoning marker      →  derivation, not a quote (v3.0)
 BeliefFact { status, provenance } →  belief layer with audit lifecycle (v4.0.27)
 BeliefConflict + resolve_*        →  contradictions revisable on demand (v4.1.0)
 ToolEvidence { typed, structured }→  every reply is back-tied to typed evidence (v4.3.0)
 EntityMemory.canonical_id         →  remembered places stable as geo_kz_NNN ids (v4.3.0)
 ontology::validate_*              →  structurally invalid facts rejected before render (v4.3.0)
 audit_response + audit_*_faith    →  every reply audited for placeholder / faithfulness (v4.3.0)
 contract: Rust-only + graph-first →  no Python/JS/TS, no external graph DB (v4.3.0)
```

Every marker is test-enforced in both directions: it fires when and only when the underlying path fired.

The name *adam* (Kazakh: **адам**) means "human".

## What is adam?

A **predictable, auditable Kazakh dialog system**, built **entirely in Rust**. Every output is produced by a five-layer pipeline you can trace end-to-end:

```
  input ─▶ parser ─▶ semantics ─▶ [ retrieval + compose ] ─▶ planner ─▶ realiser ─▶ FST synth ─▶ output
          (Layer 1) (Layer 2)       (Layer 2.5–2.75)       (Layer 3)   (Layer 4)   (Layer 5)
```

No transformer. No embeddings. No probabilistic generation. For any input, a developer can dump every layer's state and audit why the model chose what it said.

**Design principles:**

- **Predictable** — every stage is deterministic or samples from a finite, inspectable set.
- **Auditable** — `adam_chat --trace` dumps every layer per turn; every corpus citation names its `(pack, sample_id)`.
- **Grammatically correct by construction** on the slot path — `{slot|features}` placeholders go through the FST synthesiser, so no morphologically invalid inflected form can leave the system.
- **No ungrounded generation by default** — the retrieved quote is byte-identical to the corpus. Adaptation (`ComposeMode::InSampleCitySwap`) is opt-in and every adapted response is explicitly marked with «бейімд-» so the user always knows.
- **Small** — runs on a MacBook Air M2 8 GB. No GPU.
- **Kazakh-native** — built on a 14 k-entry curated pre-modern Kazakh Lexicon and a 77.9 M-word local corpus, not translated from English.

See [**`docs/architecture_v3.md`**](docs/architecture_v3.md) for the single canonical architecture reference ([`architecture_v2.md`](docs/architecture_v2.md) remains as a v2.0–v2.3 snapshot).

## Demo

Three ways to watch adam think — all deterministic, all traceable, all safe to record.

### 1. Scripted walkthrough (`adam_demo`)

```
$ cargo run --release -p adam-dialog --bin adam_demo
```

Four parts, fully deterministic:
- **Part 1** — 12 canonical conversational turns with retrieval on, `ComposeMode::Verbatim`. Every cited quote is byte-identical to the corpus.
- **Part 2** — same 12 turns with `ComposeMode::InSampleCitySwap` (opt-in composition). On the real corpus the safety guards refuse most swaps — this is the *safe case* (marker fires only when a swap actually happened).
- **Part 3** — synthetic sample explicitly triggering the swap path, so the v1.9.5 «бейімд-» adaptation marker is visible in action.
- **Part 4** — loads committed `facts.json` + `derived_facts.json`, picks one representative derivation per rule id (R1, R2, R3, R5, R6, R7), runs a user probe against each. Every response cites the **reasoned** chain — not a quote — and carries the «байланыс-» trust marker.

### 2. Interactive REPL (`adam_chat`)

Ten turns from a real v4.0 session. Inline comments explain each v4.0 capability; nothing fabricated, nothing probabilistic, every claim traceable.

```
$ cargo run --release -p adam-dialog --bin adam_chat
adam-chat v4.0 — пікірлесейік! Қазақ тілінде сөйлесейік; ^D to quit.

> сәлем                                    # greeting
сәлем

> менің атым Дәулет                        # entity extraction → session.name
қош келдіңіз Дәулет

> мен Қостанайдамын                        # FST locative+P1Sg parse (v1.4.0 + v1.8.5)
жақсы жер                                  # session.city = Қостанай

> бағдарламашымын                          # FST predicate-person parse (v1.4.0)
жақсы кәсіп                                # session.occupation = бағдарламашы

> жер туралы айтшы                         # reasoning probe — no direct quote needed
жер туралы мынадай байланыс анықтадым:
қорытынды: жер — аспан денесі
(байланысты ой-тізбек арқылы)
# R1_is_a_transitivity: (жер IsA ғаламшар) ∧ (ғаламшар IsA аспан денесі) ⟹
#   derived conclusion, source_chain=[world_core/astronomy.jsonl/astro_001,
#                                     world_core/astronomy.jsonl/astro_012].
# «байланыс-» marker = REASONED, not quoted. Test-enforced invariant.

> Қазақстан туралы айтшы                   # emergent conclusion from curated facts
қазақстан туралы мынадай байланыс анықтадым:
қорытынды: қазақстан — ұйым
(байланысты ой-тізбек арқылы)
# R1 via world_core/society.jsonl: (қазақстан IsA мемлекет) ∧ (мемлекет IsA
#   ұйым) ⟹ қазақстан IsA ұйым. adam did not memorise this — it inferred it.

> Абай жайында не дейсің                   # retrieval fallback (v1.6.0 + v1.7.0)
абай жайында осындай мысал бар:
«Абай Құнанбайұлы (10 тамыз 1845 — 6 шілде 1904)»
# byte-identical quote from wikipedia_kz_pack.json / wiki_kz_0000190.

> әке туралы бір мысал айтшы               # proverb-depth retrieval
әке жайында осындай мысал бар:
«Атаның баласы болма, адамның баласы бол»
# kazakh_proverbs_pack.json / proverb_077.

> сен ақымақсың                            # Insult intent (v1.1.0 revert of escalation)
сізге ренжімеймін                          # polite non-engagement. Never retaliates.

> сау бол
сау бол
```

**Every line above is traceable to one of five things**: (1) a template realisation, (2) a verbatim corpus quote with `(pack, sample_id)` provenance, (3) an FST-synthesised slot fill, (4) a rule-derived chain with `rule_id` + non-empty `source_chain` carrying the «байланыс-» marker, (5) a curated World Core fact with a named reviewer. Nothing else can leave the system. Zero free-form generation, zero LLM calls, zero GPU.

### 3. Interactive knowledge query (`adam_inspect`, v3.7.0+)

The opposite of a scripted demo — the investor types any Kazakh root they care about, and adam prints *everything* it knows about it:

```
$ cargo run --release -p adam-dialog --bin adam_inspect -- жер
adam_inspect — committed runtime: 13 745 facts, 7 866 derivations, 3 315 nodes, 12 350 edges

# Graph position for `жер`
  out-degree: 83   in-degree: 138   total: 221
  outgoing: after=3, does_to=45, goes_to=15, has=2, has_quantity=1, is_a=2,
            lives_in=4, part_of=1, related_to=10
  incoming: does_to=80, goes_to=30, lives_in=18, part_of=2, related_to=8

# Curated facts (world_core — HumanApproved): 5 as subject, 3 as object
  As subject:
    `жер` --is_a--> `ғаламшар`   [astronomy; world_core/astronomy.jsonl/astro_001]
      kk: «Жер — Күн жүйесіндегі ғаламшар.»
    `жер` --part_of--> `күн жүйесі`   [astronomy; ...astro_001]
      kk: «Жер — Күн жүйесіндегі ғаламшар.»
    `жер` --has--> `тартылыс`   [astronomy; ...astro_014]
      kk: «Жер тартылыс күшіне ие.»
    `жер` --goes_to--> `күн`   [astronomy; ...astro_017]
      kk: «Жер күнді айналады.»
    `жер` --has_quantity--> `серік`   [astronomy; ...astro_027]
      kk: «Жердің бір серігі бар.»

# Extracted facts (Grammar — corpus text patterns): 152 as subject, 151 as object
  [full list with (pack, sample_id) per fact]

# Rule-derived facts (inferred): … as subject, … as object
  [derivations with rule_id + source_chain]

# Summary: `жер` has degree 221 (83 out + 138 in) across 9 graph predicates.
  5 curated (world_core) + 152 extracted (text) facts and N rule-derived facts
  reference it directly. Every claim above is traceable via
  `(pack, sample_id)` or `rule_id` + `source_chain`.
```

This is the "prove it" mode: pick any Kazakh content noun, watch adam show its full evidence stack — curated World Core entries first (each with a named reviewer), then corpus-extracted facts with source quotes, then rule-derived conclusions. Everything provenance-first, nothing from a black box.

## Architecture

Nine Rust crates, three layers:

| Layer | Crate | Role |
|---|---|---|
| **L0** | [`adam-kernel`](crates/adam-kernel) | Core identity + foundation contracts |
| **L0** | [`adam-kernel-fst`](crates/adam-kernel-fst) | **FST morphology** — phonology (11 archiphonemes, 22+ twol rules), morphotactics (36 suffix templates incl. v1.4.0 predicate-person copula), synthesiser + parser, 14 k-entry Lexicon |
| **L1** | [`adam-tokenizer`](crates/adam-tokenizer) | Pre-tokenizer + BPE trainer + encoder |
| **L1** | [`adam-corpus`](crates/adam-corpus) | Source acceptance, streaming processors (Wikipedia, CC-100, classics, Common Voice, Tatoeba), synthetic generator, `corpus_audit`, `morpheme_coverage` (v1.5.5) |
| **L1** | [`adam-eval`](crates/adam-eval) | Evaluation suite + benchmark reports |
| **L1** | [`adam-dialog`](crates/adam-dialog) | **Dialog pipeline** — intent recognisers (26 intents), multi-turn session + DST, template planner with `{slot\|features}` syntax, slot-expanding realiser |
| **L1** | [`adam-retrieval`](crates/adam-retrieval) | **Retrieval engine** (v1.6.0+) — morpheme inverted index (`MorphemeIndex`), deterministic `rank(input_morphemes, config)` with overlap + pack-purity + length + loanword scoring (v1.7.0), `SampleRef` provenance, `sample_texts` for direct quoting, `compose::compose_with_city` (v1.9.0) for opt-in in-sample city swap |
| **L1** | [`adam-reasoning`](crates/adam-reasoning) | **Reasoning bootstrap** (v2.1+) — structured-fact extraction over FST parses + lexical graph projection + forward-chaining rule reasoner. `Fact { subject, predicate, object, pattern, source, confidence, raw_text }`, typed `ConfidenceKind` (grammar / curated / repeated / human / rule-inferred — **not an LLM probability**), `Predicate { IsA, LivesIn, Has, GoesTo, PartOf, RelatedTo }`. Four deterministic pattern matchers. v2.3: `LexicalGraph` with `from_facts` / `outgoing` / `incoming` — nodes + typed edges with full provenance. v2.4: `reasoner::run` forward-chaining with explicit `rule_id` + `source_chain` on every `DerivedFact`. v2.5: dative-motion pattern + `GoesTo` predicate. v2.6: `PartOf` + `RelatedTo` predicates, R5 rule active → first real derivation (`кітап RelatedTo ілім`). Binaries: `extract_facts`, `build_lexical_graph`, `run_reasoner`. Implementation of **ILMRR** — Intelligent Lexical-Morphemic Retrieval & Reasoning |
| **L2** | [`adam-train`](crates/adam-train) | Legacy transformer baseline (see [History](#history)) |

Every layer outputs deterministic, regression-tested JSON artifacts. `bash ./scripts/validate_foundation.sh` runs the full foundation validation end-to-end.

## Quick start

```bash
# Build the dialog REPL
cargo build --release -p adam-dialog --bin adam_chat

# Run it (auto-loads data/dialog/templates/v1.toml)
./target/release/adam_chat

# Single-shot
./target/release/adam_chat --once "менің атым Дәулет"

# Full Layer 1..5 trace per turn
./target/release/adam_chat --trace
```

Also available:

```bash
# FST synthesiser + analyser CLI
cargo run --release -p adam-kernel-fst --bin adam_fst -- synth --root бала --plural --case dat
# → балаларға

cargo run --release -p adam-kernel-fst --bin adam_fst -- analyse мектебім
# → noun: мектеп +P1Sg

# Full foundation validation (~30 s on M2)
bash ./scripts/validate_foundation.sh
```

## Capabilities

### 26 intents

| family | intents |
|---|---|
| Social | Greeting (Casual / Polite / Morning / Day / Evening), Farewell, Affirmation, Negation, Thanks, Apology, Compliment, Request, WellWishes |
| Conversational | AskHowAreYou, StatementOfWellbeing, AskName, StatementOfName { name } |
| Social topics | AskAge, StatementOfAge { years }, AskLocation, StatementOfLocation { city }, AskOccupation, StatementOfOccupation { occupation }, AskFamily, StatementOfFamily, AskWeather, StatementOfWeather, AskTime |
| Boundary | **Insult** (v1.1.0) — polite non-engagement for rude input |
| Fallback | Unknown { raw_tokens, noun_hint, example } — v1.6.5+ smart handler retrieves a corpus sample for the topic and cites it verbatim |

Every `Statement*` intent with an `Option<T>` payload carries an extracted entity that persists into the session and feeds downstream templates.

### Retrieval engine (v1.6.0–v1.9.5)

When no intent matches, `adam` falls back to **retrieve → rank → compose**:

1. Parse the user's input through the FST; extract every **content root** (no pronouns, no closed-class tokens).
2. Look those morphemes up in the committed `MorphemeIndex` (`data/retrieval/morpheme_index.json`, built offline from `tatoeba`, `wikipedia_kz`, `common_voice_kk`, `cc100_kk`, `abai_wikisource`, `kazakh_proverbs`, `synthetic_sentences`, and `kazakh_classics`).
3. Rank the candidate samples by a **deterministic composite score**:
   ```
   score = 0.40 · overlap_ratio          // main "smart" signal
         + 0.30 · pack_purity            // Abai 1.00, Wikipedia 0.85, CC-100 0.75
         + 0.15 · length_goodness        // Gaussian μ=8 words, σ=6
         − 0.15 · loanword_density       // native-Kazakh thesis
   ```
4. Quote the top-1 hit **verbatim** — guaranteed to exist in the corpus. Every quote carries a `(pack, sample_id)` provenance.
5. Choose a **session-aware template** (v1.8.0) to frame the quote — `template_is_fillable` auto-activates personalised variants when the session has `name` / `city` / `age` / `occupation`. FST-aware placeholders like `{city|locative}` (v1.8.5) render with correct vowel-harmonic suffixes.

This path is:

- **Deterministic** — rank has zero randomness; ties break on `(pack, sample_id)`. Same input + same index → byte-identical output.
- **Traceable** — every response cites its source.
- **No ungrounded generation** — we quote, never invent. The retrieved sentence is always a real sentence from a real source.

### Opt-in in-sample composition (v1.9.0+)

By default, the cited quote is **byte-identical** to the corpus sample — zero fabrication. Embedders who want composition can opt into `ComposeMode::InSampleCitySwap`:

```rust
use adam_dialog::{ComposeMode, Conversation};

let conv = Conversation::new()
    .with_morpheme_index(idx)
    .with_compose_mode(ComposeMode::InSampleCitySwap);
```

With swap mode on **and** the session carrying a known Kazakh city, city mentions inside the cited quote are rewritten to the user's city, feature-preserving via the FST (locative stays locative, etc.). Safety guards:

- **Closed list of 20 cities** — only roots in `adam_retrieval::compose::PLACE_NAMES` are swappable.
- **User's city must be in the list** — otherwise the FST can't re-synthesise reliably.
- **Biographical-year guard** — quotes containing a 4-digit year in [1500, 2100] are refused outright, so biographies like "Абай 1845 жылы Қарқаралыда туған" are never rewritten.
- **No name or number swaps** — those are the highest-fabrication-risk categories and are explicitly out of scope for v1.9.0.

**Trust contract — when we adapt, we say so (v1.9.5).** The planner routes any adapted response through the `unknown.with_adapted_evidence` template family, whose every template contains the Kazakh stem «бейімд-» ("adapt-"). Two invariants are test-enforced: when a swap happened the marker MUST fire, and when no swap happened the marker MUST NOT fire. A user can always distinguish a verbatim corpus quote from an adapted one at the textual level alone.

Every swap produces provenance via `Composition::trace()` — `[2] Алматыда → Шымкентте (root=шымкент, case=Some(Locative))` — so `adam_chat --trace` can explain every change.

### Kazakh-only recogniser (v1.1.0 revert)

v0.9.6 shipped Russian / English trigger phrasings for all 25 intents. Post-v1.0.0 testing showed the multilingual path diluted the Kazakh-first thesis without delivering real generalisation — a Russian speaker typing "Я разработчик" got "түсінбедім" because "разработчик" isn't in the Kazakh Lexicon. **The multilingual surface was removed in v1.1.0.** Non-Kazakh input now falls through to `Intent::Unknown`, which since v1.6.5 routes through the retrieval engine above.

The project's path to handling unbounded inputs is **not translation and not a trained neural LM** — it is the retrieval engine above, scaled to a ~100 M-token Kazakh corpus. See [History](#history) and [roadmap](docs/roadmap.md#post-v10-direction) for the architectural rationale.

Self-introduction patterns (Kazakh only):

- `менің атым X`, `атым X`, `мені X деп атайды`, `есімім X`

### Slot syntax for FST-backed templates

Template `{slot|features}` renders via `adam_kernel_fst::morphotactics::synthesise_noun`. Features combine `+`-separated:

| family | tokens |
|---|---|
| Case | `nominative/nom, genitive/gen, dative/dat, accusative/acc, locative/loc, ablative/abl, instrumental/inst` |
| Number | `singular/sg, plural/pl` |
| Derivation | `agent, abstract/abs, privative/priv, endowed/end, similative/sim, comparative/comp, verbalnoun/vnoun, actionnoun/anoun, diminutive/dim, ordinal/ord, collective/coll` |
| Possessive | `p1sg, p2sg/p2sg_pol, p2sg_inf, p3, p1pl, p2pl/p2pl_pol, p2pl_inf` |

Example template: `"{name|instrumental} танысқаныма қуаныштымын"` → `"Дәулетпен танысқаныма қуаныштымын"`. Latin names transliterate to Cyrillic before FST synthesis: `John → Джохн → Джохнмен`.

### Session state (`Conversation`)

```rust
use adam_dialog::{Conversation, TemplateRepository};

let repo = TemplateRepository::load_default()?;
let lex  = adam_kernel_fst::lexicon::LexiconV1::load_default()?;
let mut conv = Conversation::new();

let response = conv.turn("менің атым Дәулет", &lex, &repo, seed);
// conv.session == { "name": "Дәулет" }

// next turn — {name}-referencing templates are now eligible:
let response = conv.turn("сәлем", &lex, &repo, seed);
// possible output: "сәлем Дәулет"
```

### Cross-slot templates

Multi-entity templates fire only when every referenced slot is filled. Eligibility is determined by the template filter; non-fillable templates stay in the repository but aren't picked.

| template | eligibility | example output |
|---|---|---|
| `"сәлем {name}, {city\|ablative} хабар жақсы ма"` | requires name + city | сәлем Дәулет, Алматыдан хабар жақсы ма |
| `"{name}, {age} жас — керемет кезең"` | requires name + age | Дәулет, 30 жас — керемет кезең |
| `"{name}, сіз {city\|locative} {occupation} екенсіз"` | requires all three | Дәулет, сіз Алматыда мұғалім екенсіз |

## Technical specification

| Component | Value |
|---|---|
| Lexicon roots | **~25.5 k** (13 606 pure Kazakh + 11 919 Apertium imports, curated, ≥ 3 chars) |
| Abai Qunanbayuly coverage | **97.8 %** (word forms → root prefix match) |
| Committed corpus words | **4.57 M** across 9 source packs (10 textbooks in `kazakh_textbooks_pack.json` — 434 581 raw words / 28 110 samples) |
| Local corpus words (with Wikipedia + CC-100 shards) | **77.9 M** |
| Morpheme-coverage baseline (v1.5.5 historical) | **79.48 %** prefix-match over 3.84 M committed words at v1.5.5; re-run on every Lexicon PR (see `project_morpheme_coverage_baseline` memory) |
| FST archiphonemes | **11** |
| FST twol phonology rules | **22+** of Apertium's 54 catalogued, all implemented |
| Suffix templates | **36** (cases × numbers × possessives × derivations × predicate-person copula) |
| FST synthesis → analysis roundtrip | **100.0 %** on 36 238 forms |
| FST parser throughput | **1.155 ms / word** single-threaded M2 |
| Dialog intents | **26** (v1.1.0 added Insult) |
| Template families | **49** (`grep -c '^\[\[families\]\]' data/dialog/templates/v1.toml`); v4.4.0 added `dismiss_contradiction`, v4.4.5 added `check_contradiction`, v4.3.4 added the four `ask_about_system.*` aspect families |
| Slot types (session) | `name`, `age`, `city`, `occupation` (string slots, plus `{slot\|features}` FST-aware variants); v4.3.0 adds canonical-id auxiliaries `city_id` + `geo_kind` for geography |
| Canonical entity ids (v4.3.0) | `EntityMemory.canonical_id`; geography places stored under `geo_kz_NNN` ids resolved via `language_core::canonical_geo_entity` from `data/world_core/geography_kz.jsonl` |
| Cognitive eval baseline (v4.4.13) | **65 / 65 canonical, 0 aspirational** — every scenario the harness has tracked since v4.0.34 now passes; growth log in `docs/roadmap.md` |
| REPL replay baseline (v4.4.13) | **43 / 43 canonical, 0 aspirational** — v4.4.13 added 3 locative-attributive listing dialogs (rivers / lakes / deserts) closing the chain that v4.4.12 started with the mountains dialog |
| Belief revision (v4.1.0) | `BeliefState::resolve_contradiction(subject, predicate, chosen_object)` — flips chosen → Active, others → Superseded, drops matching `BeliefConflict` + `ContradictionToResolve` pending question |
| Tool layer (v4.0.37 → v4.3.0) | `Tool::dispatch(call, ctx)` — `SearchBelief`, `SearchGraph`, `SearchRetrieval`, `RunLocalReasoner`. v4.2.0 retired `inject_*`; `tool_plan_for_turn` declares the call list, `apply_tool_results` folds findings back. v4.3.0 added `ToolResult.evidence: Vec<ToolEvidence>` carrying typed claims (BeliefFact / GraphFact / RetrievalSample / DerivedFact) |
| Ontology gates (v4.3.0) | `adam_reasoning::ontology` — type constraints on admissible facts; `validate_fact` / `validate_derived_fact_with_supports` / `find_support_fact` |
| Response-quality audit (v4.3.0) | `adam_dialog::quality::audit_response` (placeholder leaks, Latin debug, double-space) + `audit_trace_faithfulness` + `audit_typed_faithfulness` + `audit_graph_admissibility` |
| Pattern matchers | **11** — v2.x (4) + v3.5.0 (6) + v3.5.5 structural_part_of, all behind v3.9.0's `is_fragment_root` central hygiene gate |
| Reasoning rules active | **10 of 11** — R1 IsA-transitivity, R2 Has-inheritance, R3 Has-via-PartOf, R5 shared-IsA → RelatedTo, R6 LivesIn-via-PartOf, R7 GoesTo-via-PartOf, R8 After-transitivity, R9 PartOf-transitivity, R10 InDomain-inheritance, R11 InDomain-shared-target. R4 IsA-symmetry is curator-warning only |
| Predicates defined | **11** — IsA, LivesIn, Has, GoesTo, PartOf, RelatedTo, Causes, After, HasQuantity, DoesTo, InDomain |
| Extracted / curated / derived facts (committed runtime) | **14 526 extracted + 1305 curated (world_core, 33 domains) / 23 418 derived** (T4_200k text-extraction scale; numeric per-rule breakdown in the Capabilities table) |
| Ungrounded generation rate | **zero ungrounded generation inside the deterministic recognised / grounded runtime path** — retrieval quotes verbatim, reasoner derives only from typed facts, refusal or `unknown.tentative` outside the envelope. Not a general open-domain hallucination benchmark; it's a runtime-path contract |
| Workspace tests | **727 passing, 0 failing, 4 ignored** |
| Extraction throughput (v3.1.0) | **~3 000 samples / 12 s** on M2 8-core (Rayon) — ~3.5× over v3.0 sequential |

## Directory layout

See [data/README.md](data/README.md) for a top-level map of the `data/` tree, and per-subdirectory READMEs for details:

- [data/dialog/README.md](data/dialog/README.md) — template repository + schema
- [data/curated/README.md](data/curated/README.md) — source packs + manifest hierarchy
- [data/lexicon_v1/README.md](data/lexicon_v1/README.md) — Lexicon provenance
- [data/training/README.md](data/training/README.md) — legacy transformer artifacts

## History

`adam` went through three major architectural eras and a v1.1.0 course-correction:

- **v0.1.0 – v0.4.0 (transformer era)** — authentic Kazakh corpus curation (Tatoeba, Wikipedia KZ, Common Voice KK, CC-100, Abai Wikisource), BPE tokenizer, baseline transformer training. The v0.4.0 checkpoint (24.2 M parameters, PPL 1691.89 on 12 k held-out samples) is preserved in `data/training/` as a regression reference but is **not** on the current codepath.
- **v0.4.5 – v1.0.0 (FST + dialog era)** — deterministic FST morphology, 14 k-entry pure Kazakh Lexicon, 25-intent dialog pipeline with multi-turn session state, FST-backed slot expansion.
- **v1.1.0 course-correction** — post-v1.0.0 testing showed the v0.9.6 multilingual surface was a mistake. Removing it and committing to a Kazakh-only input surface is the honest path toward a thinking Kazakh model.
- **v1.5.0 – v1.8.5 (retrieval era)** — the path to v2.0 is **retrieval**, not a trained neural LM. v1.5.0 re-extracted CC-100 into a 77.9 M-word local corpus. v1.5.5 measured the 79.48 % morpheme-coverage baseline. v1.6.0 shipped `adam-retrieval` with the morpheme inverted index. v1.6.5 wired retrieval into `Intent::Unknown` so dialog cites Abai / Wikipedia / CC-100 verbatim. v1.7.0 added deterministic ranking (overlap + purity + length + loanword density). v1.8.0 introduced **session-aware composition (option A)** — the retrieved quote stays verbatim, the frame around it personalises via the session. v1.8.5 fixed the `-мын` greedy-strip bug and wired FST-aware `{city|locative}` into session-aware templates.
- **v1.9.0 (option B entry)** — first step where the retrieved quote is no longer guaranteed byte-identical. `ComposeMode::InSampleCitySwap` (opt-in; default stays `Verbatim`) rewrites city mentions inside the cited quote to the user's session city via feature-preserving FST synthesis. Safety guards: closed 20-city list, biographical-year refusal (any year 1500–2100), no name/number swaps. Grammaticality FST-guaranteed; semantic truthfulness now a trade-off, explicitly marked in the mode setter.

See [CHANGELOG.md](CHANGELOG.md) for the full version-by-version history and [docs/roadmap.md](docs/roadmap.md) for the phase-by-phase overview, including the v1.9.0+ roadmap toward in-sample slot swap (option B/C territory, with semantic sanity guards).

## Foundation policies

- [corpus policy](docs/corpus_policy.md)
- [corpus sources](docs/corpus_sources.md)
- [curation workflow](docs/curation_workflow.md)
- [source classification](docs/source_classification.md)
- [source scoring](docs/source_scoring.md)
- [tokenizer policy](docs/tokenizer_policy.md)
- [evaluation policy](docs/evaluation_policy.md)
- [dialog architecture](docs/kazakh_grammar/07_dialog_architecture.md)
- [Kazakh grammar reference](docs/kazakh_grammar/README.md)

## Out of scope

- **Multilingual input and output** (v1.1.0 revert). The v0.9.6 Russian / English triggers were removed; `adam` accepts and produces only Kazakh. Generalisation comes via the retrieval engine over the 77.9 M-word Kazakh corpus, not translation.
- **Speech / multimodal** — deferred until the retrieval engine is a solid baseline.
- **Cloud platform work.**
- **Probabilistic / LLM-style free generation.** Every response is either a template realisation (26-intent path), a verbatim corpus quote (retrieval path), or a rule derivation over typed facts with a full `source_chain` (reasoning path). Nothing invented.
- **Trained neural LM components in the runtime.** v4.x is committed to deterministic retrieval + composition + reasoning + belief revision over a curated Kazakh corpus. No transformer, no embeddings, no probabilistic generation in the answer path. See [`project_retrieval_not_neural_v2`](docs/roadmap.md#post-v10-direction) and [`project_v4_direction`](docs/roadmap.md#v4-direction).

The repo grows from clean data, tight scope, and deterministic composition. Not from broad claims, and not from gradient descent.

## License

Business Source License 1.1. Converts automatically to Apache License 2.0 on **2029-01-01**. See [LICENSE](LICENSE) for full terms.

Non-commercial and research use is unrestricted today. Commercial use is permitted unless it competes directly with Qazna Technologies LLP products or services.

Copyright © 2024–2026 Qazna Technologies LLP.

For commercial licensing inquiries: **hello@qazaq.ai**
