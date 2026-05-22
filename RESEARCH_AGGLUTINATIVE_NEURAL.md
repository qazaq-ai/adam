# Research charter — Agglutinative-Neural (experimental)

**Status:** EXPERIMENTAL · branch `experimental/agglutinative-neural` · started 2026-05-15
**Owner:** Daulet Baimurza
**Production branch:** `main` (deterministic kernel, unaffected by this work)
**Why this project exists at all:** see [`docs/MANIFESTO.md`](docs/MANIFESTO.md)

> **Эта ветка — исследование, не продакт.** Цель — выяснить, можно ли построить казахскоязычный AI, который **сочетает** нейросети, обучение и генерацию **с** агглютинативной алгеброй и курируемым корпусом, и при этом **сохраняет** ключевые гарантии main-ветки (предсказуемость, провенанс, watch-battery deployment, нулевые галлюцинации в фактических утверждениях).

---

## Тезис

LLM-индустрия использует нейросети, веса, токены и генерацию **одним способом**: масштабировать всё подряд, обучаться на интернете, наращивать параметры до сотен миллиардов, принимать opacity и галлюцинации как цену.

Гипотеза этой ветки: **те же инструменты могут быть использованы иначе.** Малый корпус → чистые данные → агглютинативная структура → дискретные веса → CPU-инференс → детерминистический verifier поверх. Результат: модель, которая **умнее за счёт компактности и структуры**, а не за счёт массы.

Это не отказ от детерминистической архитектуры main. Это её расширение **с математически защищённой нейросетевой надстройкой**, которая:
- Не может генерировать факты, отсутствующие в курируемом корпусе (verifier blocks).
- Не может производить морфологически невалидный казахский (FST блокирует).
- Не может уйти в фантазию длиннее N токенов без проверки (бюджет токенов на ответ).
- Может улучшить **способ** изложения, выбор слов, композицию фразы — то, что сейчас в main делают шаблоны.

---

## Что попадает в нейро-слой, что остаётся детерминистическим

| Компонент | Реализация |
|---|---|
| **Распознавание интента** | детерминистическое (matchers, как сейчас) |
| **Извлечение топика** | детерминистическое (FST + multiword_entities) |
| **Поиск фактов** | детерминистическое (retrieval над world_core) |
| **Reasoning chain** | детерминистическое (R1-R10 forward chaining) |
| **🟡 Композиция формулировки** | **нейро** (выбор слов, порядок, стилистика) |
| **🟡 Выбор шаблона / варианта** | **нейро** (selection-ranker, уже частично есть в `selection/`) |
| **🟡 Inflection-выбор для слотов** | **нейро** (какая форма «адам» в данном контексте) |
| **Морфологический синтез** | детерминистический (FST) |
| **Verifier** | детерминистический (proof_object, ничего без support не уходит) |
| **Realiser** | детерминистический (template leak guard, quality checks) |

**Принцип:** нейро-слой может **выбирать как сказать**, но **не может изобретать что сказать**. Каждый factual claim проходит через `proof_object` и `verifier::is_supported`.

---

## Что мы строим — конкретные подсистемы

### 1. Морфемный токенизатор (Agglutinative Tokenizer)

**Зачем:** BPE-токены ломают казахский (разрезают слово посреди морфемы; «китап**та**рым**ыз**да» получает 4 случайных куска). Морфема — это семантическая единица. Естественный token = одна морфема.

**Что:** Каждое слово → последовательность типизированных морфем `[корень, суффикс1, суффикс2, …]`, каждая морфема имеет:
- `id` (целое число, как BPE)
- `morpheme_type` (root | case_suffix | tense_suffix | …)
- `features` (vowel_harmony, voicing, …)

**Размер словаря:** ~30 000 (25 500 корней + ~4 500 суффиксов и их вариантов). Это **в 30 раз меньше**, чем BPE-словарь современной LLM (~100 000 — 250 000).

**Преимущество:** каждый токен — semantically valid morpheme, не случайная байт-последовательность. Сразу решает половину классических LLM-проблем с агглютинативными языками.

### 2. Малый агглютинативный трансформер (Small AGT)

**Параметры:** target ≤ **10 миллионов** (1000× меньше Llama-3-8B).

**Архитектура:** скромный transformer-encoder + decoder, **но** с двумя структурными модификациями:
- **Position encoding** учитывает морфологическую позицию (root vs suffix-N), не только токен-индекс.
- **Output head ограничен** морфологически валидными морфемами для текущего контекста (FST-guided decoding).

**Корпус для обучения:** только **наш собственный** курируемый корпус (~18k образцов на v5.32.0), плюс генератор синтетических фраз через FST (мы уже умеем синтезировать миллионы валидных казахских словоформ из Лексикона).

**Цель:** на выходе сеть выдаёт не «следующий BPE-токен», а **следующую морфему**, что:
- Всегда даёт валидное казахское слово (FST blocks invalid).
- Не может «галлюцинировать» английский / русский (нет таких токенов в словаре).
- Можно интерпретировать каждый шаг (каждая морфема имеет тип).

### 3. Selection-Net (расширение существующего)

У нас **уже есть** в [`crates/adam-dialog/src/selection/`](crates/adam-dialog/src/selection/) обучаемый perceptron на ~30 features. Расширяем:
- До **~1000-10000 features**
- Обучаем на парах «лучший / худший ответ» из live-test-транскриптов
- Используется для выбора между applicable templates
- Полностью inspectable: каждый вес соответствует именованной feature

### 4. Algebraic Loss

Стандартный cross-entropy loss + **дополнительные штрафы**:
- За вывод морфологически невалидной последовательности (FST validation)
- За нарушение vowel harmony
- За пустой `proof_object` при factual claim
- За расхождение между «выбранной формой» и канонической FST-формой

Эти штрафы превращают агглютинативную алгебру в **directly enforceable signal** во время обучения, а не post-hoc filter.

---

## Бюджеты (жёсткие)

| Ресурс | Лимит |
|---|---|
| Параметры модели | ≤ 10M (proof-of-concept) → ≤ 100M (если PoC удался) |
| RSS при инференсе | ≤ 1 GB |
| Latency p50 на CPU | ≤ 200 мс (vs main 21 мс) |
| **Inference на CPU обязан работать** | M2 8GB — основная цель |
| Inference на GPU — допускается ускорение, не зависимость | если есть локальный GPU и реально быстрее — пользуемся |
| Training | M2 8GB CPU **или** локальный consumer-grade GPU (RTX 3090 / M2 Ultra / 4090) |
| **Cloud любой формы** | **ЗАПРЕЩЁН на всех этапах** (training, inference, fine-tuning) |
| Рантайм-зависимости | candle / burn / tch для inference, без cloud APIs |

**Принцип — efficient use of hardware, not GPU-free.** GPU допускается там, где даёт реальный speedup:
- **Training** (matmul-heavy → GPU быстрее CPU в 10-50×; не cloud, локально)
- **Batch precomputes** (embedding-таблицы, root-affinity matrix, suffix priors — один раз)
- **Inference**, если на конкретной машине есть локальный GPU **и** даёт реальный speedup **и** CPU-fallback при этом работает

Чего избегаем — **GPU dependency**:
- ❌ inference, который не работает без GPU
- ❌ модель, которая требует Mac Studio / multi-GPU / датацентр
- ❌ ситуация «запустить на M2 8GB CPU — невозможно»

**Watch-battery / air-gap** остаётся стратегической целью. Если эксперимент **требует** GPU для inference — это сигнал, что направление неверное. Если ускоряет, но не требует — это норма.

---

## Метрики успеха

PoC считается **успешным**, если:

1. **Качество не хуже main** на 30-вопросном demo battery (military_kz, knowledge_query) — минимум 28/30 с проверяемым provenance.
2. **Способность к новизне** — adam отвечает на казахском на вопросы, которые main не покрывает шаблонами, причём корректно. Метрика: на 50 OOD-вопросов (out-of-template), новая система даёт ≥ 70% валидных по смыслу ответов; main — 0% (т.к. fallback).
3. **Нулевые fabricated claims** — verifier ловит 100% попыток сказать что-либо без `support`. Метрика: 0 hallucinated facts в 1000-вопросном adversarial battery.
4. **Бюджет соблюдён** — ≤10M params, ≤1GB RSS, ≤200мс p50.
5. **Inspectable** — для любого ответа можно показать: какие морфемы выбраны, какой fact поддержал, какой template-вариант ranked top.

PoC **проваливается** (rollback в main), если:

- Качество хуже main даже после 3 итераций (отдача времени без улучшений).
- Параметры или RSS выходят за бюджет в 2× и не уменьшаются.
- Hallucination rate > 0 даже в 10-вопросной выборке (architectural failure).
- Зависим от GPU/cloud для inference.

---

## Таймлайн

### Phase 0 — Deep Research Sprint (3 дня)

**Пятница — Суббота — Понедельник (15-16 + 18 мая 2026).**

Полный фокус на теоретическую подготовку. **Никаких внешних дел** — МО, КРУ outreach, voice fixes, военные факты — всё отложено. Работаем в ветке `experimental/agglutinative-neural`. Никаких релизов в main за эти 3 дня.

| День | Цель | Артефакт |
|---|---|---|
| 1 (пт) | Literature scan: small LMs / morpheme tokenization / FST-guided decoding / agglutinative NLP (TR/FI/HU/KO precedents) / neuro-symbolic hybrids | `docs/research/lit_scan.md` |
| 2 (сб) | Математическая формализация: алгебра морфем как vector space, vowel harmony как constraint, algebraic loss как добавляемые члены | `docs/research/math_foundations.md` |
| 3 (пн) | Спецификация морфемного токенайзера + первый Rust-прототип, который tokenize 100 казахских слов с FST round-trip | `crates/adam-agg-tokenizer/` stub + passing tests |

### Phase 1 — Tokenizer + Training Pipeline (3 недели после MoD pitch)

**27 мая — 14 июня.** Полная training-инфраструктура. **Цель**: 100k казахских слов корректно tokenize, full FST round-trip.

### Phase 2 — First Small AGT (4 недели)

**15 июня — 14 июля.** First small AGT (1-3M params) на synthetic FST corpus. **Цель**: морфологически валидный казахский на простых задачах.

### Phase 3 — Verifier Integration + Real Data (4 недели)

**15 июля — 14 августа.** Интеграция с deterministic verifier; обучение на real dialog-data. **Цель**: воспроизвести качество main на 30-question battery.

### Decision Gate — 15 августа 2026

Полная оценка: продолжаем (готовим v6.0.0 как hybrid release), останавливаем, или rollback. **Без сожаления, если данные говорят rollback.**

---

## Что НЕ делаем в этой ветке

- ❌ Не трогаем main. Все продакт-фичи (МО, voice, current dialog) идут параллельно в main.
- ❌ Не cloud-тренируем. Если M2 не тянет — это сигнал, а не проблема.
- ❌ Не привлекаем сторонние pretrained веса. Никаких HF checkpoints как стартовая точка.
- ❌ Не переименовываем deterministic в «retrograde». Main остаётся основой бренда и MoD pitch.
- ❌ Не публикуем эту ветку как готовую модель до прохождения всех 5 метрик успеха.

---

## Связь с другими директивами

- [`project_retrieval_not_neural_v2`](.claude/.../memory/project_retrieval_not_neural_v2.md) — **эта ветка реализует именно этот документ**. «Weights/learning/generation OK if discrete/inspectable/cheap.»
- [`project_deterministic_directive_confirmed`](.claude/.../memory/project_deterministic_directive_confirmed.md) — **частично пересматривается**. Старая формулировка «reject LLM code-gen regardless of framing» остаётся в силе для main; для experimental — переходит в «reject LLM scale + opacity + cloud, accept neural tools used differently».
- [`project_v4_direction`](.claude/.../memory/project_v4_direction.md) — «no LLM-breadth race» остаётся: мы целимся в **умную узкую модель**, а не в широкую посредственную.
- [`project_engineering_framing`](.claude/.../memory/project_engineering_framing.md) — «agglutinative algebra of meaning» остаётся центральным концептом. Здесь — её математическая реализация.

---

## Первый коммит этой ветки

Этот файл (`RESEARCH_AGGLUTINATIVE_NEURAL.md`) + обновлённая директива в memory. Никакого кода. Следующие шаги — после МО pitch.
