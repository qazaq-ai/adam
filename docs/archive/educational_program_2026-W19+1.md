# Educational Program — adam Kazakh Knowledge Portal

**Strategic priority (2026-05-05 directive):** adam = #1 қазақ тілді білім порталы РК.
Coverage: 0-я школьная группа → школа → университет → академический уровень.
Special tracks: Programming (Rust первым), Medicine.

**Cadence:** 10 releases per day × 10 substantive items per release × 7 days
= 70 releases / ~700 facts in one calendar week (W19+1, 2026-05-06 → 2026-05-12).

**Discipline rules per release:**
- ONE focused topic / curriculum-band / JSONL file
- ≥10 new world_core entries (each passes `project_corpus_purity_directive`)
- ≥1 bridge fact to top-tier hub (адам / маман / зат / ұғым / ғылым / оқу пәні)
- check_metrics_currency green / clippy `-D warnings` / tests green
- Live REPL spot-check
- Standard ceremony: bump → manifests → CHANGELOG → README → roadmap → commit → tag → GH release

---

## Day 1 (2026-05-06, Wed) — Pre-school + Elementary 1-2

| # | Release | Domain | Content |
|---|---|---|---|
| 1 | v4.56.0 | preschool_alphabet.jsonl | Kazakh letters 1-10 (а, ә, б, в, г, ғ, д, е, ё, ж) |
| 2 | v4.56.5 | preschool_alphabet.jsonl | Letters 11-20 (з, и, й, к, қ, л, м, н, ң, о) |
| 3 | v4.57.0 | preschool_alphabet.jsonl | Letters 21-30 (ө, п, р, с, т, у, ұ, ү, ф, х) |
| 4 | v4.57.5 | preschool_alphabet.jsonl | Letters 31-42 (һ, ц, ч, ш, щ, ъ, ы, і, ь, э, ю, я) + alphabet bridges |
| 5 | v4.58.0 | preschool_numbers.jsonl | Cardinal + ordinal numbers 1-20 |
| 6 | v4.58.5 | preschool_numbers.jsonl | Numbers 20-100 + counting patterns |
| 7 | v4.59.0 | preschool_shapes.jsonl | Geometric shapes (дөңгелек / шаршы / үшбұрыш / …) |
| 8 | v4.59.5 | preschool_sizes.jsonl | Sizes + comparisons (үлкен/кіші/биік/аласа/…) |
| 9 | v4.60.0 | preschool_routine.jsonl | Daily routine (тұру / жуыну / тамақтану / ұйықтау) |
| 10 | v4.60.5 | preschool_emotions.jsonl | Age-appropriate emotion vocabulary |

## Day 2 (2026-05-07, Thu) — Elementary 3-4 + Kazakh language depth

| # | Release | Domain | Content |
|---|---|---|---|
| 1 | v4.61.0 | kazakh_grammar.jsonl | Parts of speech (зат есім, етістік, сын есім, …) |
| 2 | v4.61.5 | kazakh_grammar.jsonl | 7 cases + functions |
| 3 | v4.62.0 | kazakh_grammar.jsonl | Possessive system + agreement |
| 4 | v4.62.5 | kazakh_grammar.jsonl | Tenses + evidential |
| 5 | v4.63.0 | kazakh_proverbs.jsonl | +10 curated proverbs |
| 6 | v4.63.5 | abai_canon.jsonl | Абайдың 10 өлеңінен дерек |
| 7 | v4.64.0 | shakarim_canon.jsonl | Шәкәрім 10 fact |
| 8 | v4.64.5 | mathematics_basic.jsonl | Fractions + decimals + percentages |
| 9 | v4.65.0 | natural_world.jsonl | Дүниетану: nature + environment basics |
| 10 | v4.65.5 | reading_skills.jsonl | Reading comprehension primitives |

## Day 3 (2026-05-08, Fri) — Middle school 5-7

| # | Release | Domain | Content |
|---|---|---|---|
| 1 | v4.66.0 | history_kazakhstan.jsonl | Medieval khanates / batyrs (+10) |
| 2 | v4.66.5 | history_world.jsonl | Ancient civilizations |
| 3 | v4.67.0 | geography_world.jsonl | Continents + capitals |
| 4 | v4.67.5 | algebra_school.jsonl | Variables + equations + linear functions |
| 5 | v4.68.0 | geometry_school.jsonl | Triangles + circles + Pythagorean + area/volume |
| 6 | v4.68.5 | biology_school.jsonl | +10 botany/zoology |
| 7 | v4.69.0 | physics_school.jsonl | +10 mechanics |
| 8 | v4.69.5 | chemistry_school.jsonl | +10 elements/compounds |
| 9 | v4.70.0 | computer_science_basics.jsonl | +10 algorithms / data structures intro |
| 10 | v4.70.5 | english_basics.jsonl | English vocab (Kazakh meta) |

## Day 4 (2026-05-09, Sat) — Middle 8-9 + High school 10

| # | Release | Domain | Content |
|---|---|---|---|
| 1 | v4.71.0 | history_kazakhstan.jsonl | Soviet era + independence |
| 2 | v4.71.5 | history_world.jsonl | Modern era (industrial → WWII) |
| 3 | v4.72.0 | trigonometry.jsonl | sin/cos/tan + identities + unit circle |
| 4 | v4.72.5 | physics_school.jsonl | Electromagnetism intro |
| 5 | v4.73.0 | chemistry_school.jsonl | Organic chemistry intro |
| 6 | v4.73.5 | biology_school.jsonl | Genetics + Mendel + DNA basics |
| 7 | v4.74.0 | economy_basic.jsonl | +10 micro/macro intro |
| 8 | v4.74.5 | civic_studies.jsonl | Constitution + government + rights |
| 9 | v4.75.0 | russian_literature.jsonl | Pushkin/Dostoevsky/Tolstoy (Kazakh meta) |
| 10 | v4.75.5 | world_literature.jsonl | Shakespeare/Dante/Cervantes (Kazakh meta) |

## Day 5 (2026-05-10, Sun) — High 11 + University foundations

| # | Release | Domain | Content |
|---|---|---|---|
| 1 | v4.76.0 | calculus.jsonl | Derivatives + integrals + limits |
| 2 | v4.76.5 | linear_algebra.jsonl | Matrices + vectors + eigenvalues |
| 3 | v4.77.0 | statistics.jsonl | Mean/median/variance + distributions |
| 4 | v4.77.5 | probability.jsonl | Discrete + continuous + Bayes |
| 5 | v4.78.0 | physics_advanced.jsonl | Quantum mechanics intro |
| 6 | v4.78.5 | physics_advanced.jsonl | Relativity (special) |
| 7 | v4.79.0 | molecular_biology.jsonl | DNA/RNA/protein synthesis |
| 8 | v4.79.5 | organic_chemistry.jsonl | Carbon compounds + functional groups |
| 9 | v4.80.0 | economics_advanced.jsonl | Micro deep |
| 10 | v4.80.5 | economics_advanced.jsonl | Macro deep |

## Day 6 (2026-05-11, Mon) — Programming Rust + CS

| # | Release | Domain | Content |
|---|---|---|---|
| 1 | v4.81.0 | programming_rust.jsonl | Ownership semantics deep |
| 2 | v4.81.5 | programming_rust.jsonl | Borrow checker + lifetimes |
| 3 | v4.82.0 | programming_rust.jsonl | Traits + generics + trait objects |
| 4 | v4.82.5 | programming_rust.jsonl | Iterators + closures + fn-pointers |
| 5 | v4.83.0 | programming_rust.jsonl | Error handling (Result/Option/?) |
| 6 | v4.83.5 | programming_rust.jsonl | Async/await + tokio basics |
| 7 | v4.84.0 | programming_rust.jsonl | Cargo + crates.io + workspace |
| 8 | v4.84.5 | programming_rust.jsonl | Testing + benchmarks + criterion |
| 9 | v4.85.0 | algorithms.jsonl | Sorting + searching + complexity |
| 10 | v4.85.5 | data_structures.jsonl | Arrays / lists / trees / hashmaps / graphs |

## Day 7 (2026-05-12, Tue) — Medicine + capstone

| # | Release | Domain | Content |
|---|---|---|---|
| 1 | v4.86.0 | medicine_anatomy.jsonl | Body systems |
| 2 | v4.86.5 | medicine_physiology.jsonl | How body systems work |
| 3 | v4.87.0 | medicine_diseases.jsonl | Common diseases + symptoms |
| 4 | v4.87.5 | medicine_first_aid.jsonl | First aid procedures |
| 5 | v4.88.0 | medicine_pharmacology.jsonl | Drug categories + safe-use basics |
| 6 | v4.88.5 | medicine_pediatrics.jsonl | Child health + vaccinations |
| 7 | v4.89.0 | medicine_nutrition.jsonl | Macronutrients + balanced diet |
| 8 | v4.89.5 | medicine_mental_health.jsonl | Mental health basics |
| 9 | v4.90.0 | medicine_hygiene.jsonl | Personal hygiene + prevention |
| 10 | v4.90.5 | EDUCATION_PORTAL_v1 | **Capstone:** «adam — қазақстандағы №1 қазақ тілді білім порталы» — meta-fact + cross-domain bridges (every new domain IsA «оқу пәні» / «ғылым саласы») |

---

## Final state (end of Day 7 / v4.90.5)

- world_core: 2102 → ~2800 entries (+~700)
- facts: 2362 → ~3100 (+~700)
- derived facts: 28112 → ~50000+ (cascading via new IsA bridges)
- New domains: ~20-25
- adam knows curriculum across PreK → Academic for: Kazakh language, Math, Science, History, Geography, CS+Rust, Medicine

## Risks + mitigations

- **derived_facts.json size growth** — currently 16 MB, may approach 30-40 MB. Per `feedback_git_ignore_policy` (50 MB threshold) we're within limits; if exceeded, shard by domain.
- **Loanword purity** — many academic/medical terms are international. Apply per-domain judgement: «биология» / «медицина» / «алгебра» are established Kazakh academic terms. «Аутентификация» is not — prefer Kazakh equivalents.
- **Bridge-fact discipline** — every release must add ≥1 IsA edge to a top-tier hub or the cascade ROI drops.
- **Verification fatigue** — autonomous crash-tests required per release; if too costly, batch into end-of-day rollup.
