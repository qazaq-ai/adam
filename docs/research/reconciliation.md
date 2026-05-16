# Reconciliation: vision до литературы vs. реальный landscape

**Цель:** честно сопоставить мой `architecture_vision.md` (написанный ДО lit_scan'а) с тем, что показал [lit_scan.md](lit_scan.md). Где я был прав, где банален, где упустил, где переоценил.

**Дата:** 2026-05-15 (Phase 0 Day 1)

Это самый важный документ за Day 1. Vision был интеллектуальным экспериментом: «что я буду думать, если ничего не знаю про литературу». Reconciliation — это «что я думаю теперь, увидев реальный landscape».

---

## 1. Где я был прав по существу

### 1.1 ≤10M params для narrow domain — feasible

Я зафиксировал 10M как PoC-цель чисто инженерным расчётом (256-dim × 4 layers × 30k vocab → ~10M). **TinyStories** (Eldan & Li 2023) показал ровно то же эмпирически: **1-33M params на ~2M synthetic stories с vocab ~1500 слов** → связная narrow-domain генерация на английском.

Наш сценарий — Kazakh tutor, vocab 30k морфем — попадает **точно в TinyStories-feasibility zone**. Это не «я угадал», это «параметры узкого домена попадают в установленный диапазон».

→ **Validated.** 10M остаётся PoC-целью. План не меняется.

### 1.2 Morpheme tokens > BPE для агглютинативных языков

Зафиксировал на основе общего инженерного знания о Kazakh морфологии. **Корейское комьюнити** показало это в production: KoBERT, KR-BERT, MeCab-ko → morpheme-primary как стандарт; Park & Kim 2018 — +2-4 F1 на NER. Bostrom & Durrett 2020 — Unigram > BPE на турецком/финском.

→ **Validated.** Морфемно-типизированный словарь — мейнстрим в Korean, нам нужно адаптировать на казахский. Не реинвент.

### 1.3 FST-guided decoding имеет precedent

Я думал, что мы будем делать что-то новое; на самом деле **PICARD** (Scholak et al. 2021) — это **точный архитектурный паттерн**. Incremental parser-constrained decoding для SQL: невалидные continuations → zero-out до softmax → точность +7% на Spider. **Grammar-Constrained Decoding** (Geng et al. 2023) обобщает на CFG; **outlines** и **lm-format-enforcer** — production-libraries.

→ **Validated, но не оригинально.** Паттерн известен (PICARD/GCD). Наш вклад — **применить именно FST к именно агглютинативному казахскому в гено-системе**. Не сам паттерн.

### 1.4 Verifier-bounded factuality — направление верное

Зафиксировал как architectural gate. **GopherCite** (DeepMind 2022) — ближайший academic-прецедент, но soft RLHF. **Self-RAG** (Asai et al. 2023) — soft reflection tokens. **Phi-3 Grounding** — eval-time check. Все они soft.

Наш `proof_object` + `verifier::is_supported` — **hard architectural gate** (тоже не post-hoc, а pre-emit). Это **строже всего, что я нашёл в литературе**.

→ **Validated + потенциально novel.**

### 1.5 CPU inference достижим

Я бюджетил ~225ms p50 на 10M params. Lit_scan: candle на Apple Silicon рассчитан как сотни-тысячи tok/s для модели нашего размера. **Бюджет с запасом, не вызов**. Бенчмарки 4-bit Phi-2 (2.7B) — 10-20 tok/s; наша 10M будет на порядок быстрее.

→ **Validated с запасом.** Возможно убираю «250ms» из жёстких бюджетов — реалистичная цель ≤50ms на M2 CPU без оптимизаций.

---

## 2. Где я был наивен или ошибся

### 2.1 «FST-constrained at every step — наш unique vклад»

Зафиксировал как ядро инновации. **Не уникально.** PICARD уже делает «incremental constraint validation at every decoder step». Наш вклад уже:
- Применение к **agglutinative morphology** (не SQL/CFG).
- Применение к **generation** (а не code-completion).
- Применение к **Kazakh specifically** (никто из казахской NLP-сцены этого не сделал).

→ **Update vision:** не «новый паттерн», а «новая area-of-application известного паттерна». Это **слабее claim**, но всё ещё публикуемо. Честная формулировка важна.

### 2.2 «Algebraic loss — мы первые»

Я был уверен это новизна. **Почти.** Нашёл два соседа:
- **Constitutional AI** (Bai et al. 2022) — rule-violation как preference signal, но soft (RLHF).
- **RLVR — Reinforcement Learning from Verifiable Reward** (Lambert et al. 2024, Tulu-3) — **точно наш паттерн** для math/code: rule-checker как reward. Активная research line 2024-2026.

→ **Update vision:** наш **vowel-harmony / suffix-validity как training signal** — это **новая application** RLVR-семейства к phonological/morphological rules. Прямого прецедента действительно не нашёл. **Defensible novelty,** но в семействе известных техник.

### 2.3 «18k samples хватит»

В черновике я писал «у нас 18k образцов + synthetic-via-FST». **TinyStories had 2M historjes.** Наши 18k — это **в 100× меньше**. Без агрессивной synthetic-augmentation модель overfit'нется на 18k и не обобщит.

→ **Update vision:** synthetic-via-FST становится **критическим path**, не «приятным добавлением». Нужно:
- Сгенерировать минимум **2-5M synthetic пар** из FST + Lexicon + templates.
- Eval на 18k curated как **valid set**, не train set.
- Live human-evaluated set отдельно как gold test.

Без этого 10M params overfit'нется. Это **самый высокий риск direction'а**.

### 2.4 «Diversity не проблема»

В vision я мельком упомянул «failure mode 2: излишняя жёсткость». **Lit_scan показал это как главный риск.** Korean morpheme-models имеют документированную issue с diversity collapse под constrained decoding. **PICARD** в SQL — diversity not a goal (SQL должен быть exact). Naш случай — Kazakh natural language — **diversity критична для human-like ощущения**.

→ **Update vision:** добавить explicit diversity-preserving механизмы:
- Sampling с temperature ≥0.7 (не greedy).
- Multiple valid morpheme paths через FST (если FST имеет несколько decompositions — модель должна научиться выбирать стилистически).
- Unlikelihood training (Welleck et al. 2020) — penalty за template-repetition.

### 2.5 «training на M2 8GB»

В vision-черновике я хотел чтобы всё было на M2. **Lit_scan показал:** для 10M × ~100M synthetic токенов нужно ~2-3 GPU-дня **на RTX 3090**. На M2 это будет **30+ дней** — практически неприемлемо для итераций.

→ **Update vision/charter:** training **на локальном RTX 3090 / M2 Ultra / 4090** — это правильный путь. Это **не нарушает air-gap** (не cloud), но требует **физическое железо**. Либо инвестируем в локальный GPU, либо PoC станет очень медленным. **Не cloud — точка.**

User specifically clarified: «локальный GPU OK». Так что план — найти/арендовать локальный RTX-class GPU для training-фазы. Inference остаётся на M2 CPU.

---

## 3. Что я упустил полностью

### 3.1 Конкретный Rust ML стек

Я писал в vision «candle / burn / tch — выберем потом». **Lit_scan дал чёткую рекомендацию**:

- **Training**: **PyTorch** (Python) на локальном GPU → export в ONNX/safetensors. Faster research iteration.
- **Inference**: **candle** с Metal backend на M2. Native Rust, интегрируется в существующий adam.
- **Стандартный паттерн 2025-2026:** PyTorch → ONNX → candle.

Альтернатива — **burn** для целиком-Rust стека (Tracel AI), идеологически чище, но 2-3× медленнее в research iterations.

→ **Concrete plan:** PyTorch+candle гибрид. PyTorch для experimentation; candle для production inference в основном adam-binary.

### 3.2 PICARD как готовый template

Я думал, что должен будет вывести FST-constrained decoding с нуля. **PICARD** уже даёт точный паттерн:
- Парсер хранит partial state.
- На каждом decoder step парсер пытается продолжить с каждым кандидатом-токеном.
- Невалидные → log-prob = -inf до softmax.

→ Прямо переносится на FST: partial-state = текущая FST-конфигурация; продолжение = checking FST.transition(state, candidate_morpheme).

### 3.3 Korean morpheme-tokenization как production proof

Я говорил «morpheme-primary новизна», но Korean уже это делает в production десять лет. **Сильнее аргумент**: если KoBERT и production translation systems в Korean работают на morpheme-primary tokenization — это работает. Не теория, а production.

Наша задача — **адаптировать корейский подход на казахский** + добавить FST hard-constraint (которого Korean MeCab-ko не делает в decoding).

### 3.4 Симвалическо-only synthetic pipeline — наш cleanest novelty

В vision я мельком про FST-synth. **Lit_scan показал:** прямого прецедента **symbolic-only synthetic-data pipeline для LM training не нашёл**. Hartmann et al. 2023 близко, но GEC-task, не LM.

→ **Это, возможно, наш самый сильный novelty-claim для публикации.** TinyStories полагается на GPT-3.5 (LLM-teacher → наследует LLM-bias). Мы — на FST (deterministic teacher → нет hallucination-bias).

Если PoC работает — пишем paper «Training small language models on symbolic-only synthetic data: a case study on Kazakh».

---

## 4. Что меняется в плане

### Pre-lit vision (Day 1 morning):
- Phase 0: 3 дня research.
- Phase 1: 3 недели — tokenizer + pipeline.
- Phase 2: 4 недели — first AGT 1-3M.
- Phase 3: 4 недели — verifier integration.
- Decision: 15 августа.

### Post-lit plan (Day 1 evening):
- Phase 0: ✅ остаётся 3 дня research.
- Phase 1 (3 недели): morpheme tokenizer (port'ируем Korean MeCab pattern, заменяем engine на наш FST) + **2-5M synthetic data generation** через FST + Lexicon + templates. **Synthetic pipeline — критический path**, не оptional.
- Phase 2 (4 недели): PyTorch transformer 10M params + **algebraic loss** + **FST-constrained decoder через PICARD-pattern**. Training на локальном GPU (RTX 3090 если есть, иначе временно через M2 + долгая итерация).
- Phase 3 (4 недели): ONNX export → candle integration + verifier integration. Eval против main.
- **Decision gate ~15 августа** остаётся.

### Что НЕ меняется:

- CPU-target M2 для inference. Остаётся бюджет ≤1GB RSS, ≤200ms p50.
- Cloud forbidden. Localонный GPU допустим (clarification от user 2026-05-15).
- Verifier как hard architectural gate. Не soft RLHF.
- Никаких HF pretrained checkpoints как стартовая точка.
- Никаких LLM-teacher'ов для distillation.

### Что меняется:

| Aspect | Pre-lit | Post-lit |
|---|---|---|
| Synthetic data | «добавим» | **критический path, 2-5M пар обязательно** |
| Training compute | M2 преимущественно | **локальный GPU необходим для разумных итераций** |
| FST-decoding | «наш новый паттерн» | **адаптация PICARD/GCD pattern** |
| Algebraic loss | «полностью новый» | **новая application RLVR-семейства** |
| Diversity | мельком | **критический риск — нужны explicit механизмы** |
| Stack | candle/burn/tch — выберем | **PyTorch training + candle inference** |
| Novelty positioning | размытое | **5 конкретных claims, 2-3 защищаемых = ACL workshop, 5 = full ACL** |

---

## 5. Перепозиционирование novelty-claims

Pre-lit я думал «мы делаем что-то революционное». Post-lit — **5 чётких claims, упорядоченных по defensibility**:

| # | Claim | Defensibility | Тип публикации если defendable |
|---|---|---|---|
| 1 | Morpheme-typed vocab + FST hard-constraint at every decoder step **для агглютинативного языка** | **Высокая** — Korean ближе всех (morpheme-primary), но без FST hard-constraint в decoding | ACL/EMNLP workshop |
| 2 | Symbolic-only synthetic-data pipeline для LM training | **Высокая** — прямого прецедента не нашёл | ACL/EMNLP main |
| 3 | Verifier as architectural gate (не post-hoc filter) | **Средняя** — GopherCite/Self-RAG близки, но soft | EMNLP findings |
| 4 | Algebraic loss с FST-detection (vowel harmony + suffix validity) | **Средняя** — RLVR близок но math/code, не phonology | EMNLP findings |
| 5 | CPU-primary deployment для agglutinative gen LM | **Низкая** — TinyStories показала feasibility для English; мы просто extend | Workshop / technical report |

**Стратегия:** делать все 5 как часть PoC. Если 2-3 sopable — workshop paper готов. Если все 5 — full conference paper.

---

## 6. Главные риски после lit-scan

### 6.1 «Just a smaller LLM that's mediocre» (failure mode #1)

**Lit_scan усиливает этот риск.** Phi-3-mini уже 3.8B и неплохо генерирует. Если наш 10M на казахском окажется хуже Phi-3-mini-translated-to-Kazakh — провал.

**Mitigation:** наш differentiator должен быть **архитектурный**, не качественный. Мы должны **уметь то, что Phi-3 не умеет**:
- Architectural inability to hallucinate facts (verifier).
- Architectural impossibility of invalid Kazakh (FST).
- CPU on M2 8GB (Phi-3 mini требует 4GB+ модель, едва тянет).

Eval должен включать «here's a known Phi-3 hallucination on Kazakh — adam architecturally refuses to make same claim».

### 6.2 «Diversity collapse»

**Lit_scan усиливает.** Korean morpheme-models показывают это в production. PICARD не борется (SQL exact).

**Mitigation:**
- Multi-path FST (несколько decompositions per word → variety).
- Temperature sampling в decoder.
- Unlikelihood loss за template-repetition.
- Live human eval с focus на «насколько ответ скучный/механический».

### 6.3 «18k мало даже после synthetic»

**Lit_scan усиливает.** TinyStories had 2M, Phi-1 had 7B.

**Mitigation:**
- FST × Lexicon × templates → минимум 2-5M synthetic пар (target).
- Augmentation: paraphrasing existing curated 18k через FST inflection-variation.
- Adversarial generation: для каждого факта генерируем 10-100 question phrasings.

### 6.4 «GPU dependency через черный ход»

User clarified локальный GPU OK. Но если model **требует** GPU для тренировки, и у user нет RTX 3090 — стартуем медленно. **Riski:** план застрянет в фазе 2 на месяцы потому что M2 8GB training слишком медленный.

**Mitigation:**
- Phase 1 (tokenizer + synth data) можно сделать только на CPU. Не требует GPU.
- Phase 2 (training): начать с **1-3M params** на M2 8GB. Это можно тянуть, медленно но реально. Шкалировать до 10M только когда инфраструктура готова.
- Если PoC на 1-3M показывает promising → инвестировать в локальный GPU.

---

## 7. Что делаем Day 2 (завтра)

Учитывая reconciliation:

**Day 2 — Математическая формализация** должна теперь покрывать:

1. **Morpheme algebra как vector space**: какие группа-теоретические свойства, какие constraint relations (vowel harmony как cluster restriction).
2. **FST как finite-state-machine for decoder constraint**: формальная модель how PICARD-pattern adapts to FST.
3. **Algebraic loss formal definition**: уравнения для vowel-harmony penalty, suffix-validity penalty, provenance penalty. С градиентами (как loss будет backprop'иться).
4. **Synthetic-data generation algorithm**: формальный процесс «sample root → attach valid suffix chain → fill template → verify». 
5. **Verifier formal spec**: что значит «`proof_object.support` covers `claim`» формально. Нужно для архитектуры (когда L1 выдаёт что-то — что именно verifier проверяет).

→ Document готовится `docs/research/math_foundations.md`.

---

## 8. Что делаем Day 3 (Monday)

**Tokenizer spec + Rust prototype** становится:

1. **Spec** в `docs/research/tokenizer_spec.md`:
   - Token = `(id: u32, type: MorphemeType, features: TokenFeatures)`.
   - 30k vocab construction: 25.5k roots + 4k suffix variants + 500 service.
   - Pre-baked training-time tokenization (using FST analyse).
   - Inference-time tokenization (with FST disambiguation when needed).

2. **Прототип** в новом crate `crates/adam-agg-tokenizer/`:
   - Reuse existing `adam-kernel-fst` for morphological analysis.
   - Output morpheme sequence.
   - Round-trip test: word → tokens → reconstruct → match.
   - Performance: tokenize 100 Kazakh words; benchmark.

Это будет первый РЕАЛЬНЫЙ Rust код в experimental ветке.

---

## 9. Conclusion of Day 1

**Что выяснили:**

1. Наша направление **defensible**: 5 novelty claims, 2-3 из них чётко новые в литературе.
2. Технически **feasible на 12-недельном PoC** — TinyStories даёт template, PICARD даёт pattern, RLVR даёт loss family.
3. **18k samples — главный риск.** Synthetic-via-FST становится критическим path.
4. **Локальный GPU нужен для разумного training**, но inference остаётся CPU M2.
5. **Stack**: PyTorch training + candle inference. Standard 2025-2026 pattern.

**Что НЕ выяснили (нужно копать дальше):**

1. Точные training-recipes для morpheme-level models на agglutinative — будем экспериментально подбирать.
2. Performance trade-offs FST-hard-constraint vs soft (gradient через парсер).
3. Конкретные benchmarks adam-AGT vs KAZ-LLM 8B на тех же вопросах.

**Решение по charter:** оставляем v0.2 (после Day 1 update'а). PostMoD pitch (после 26 мая) стартуем Phase 1 с увеличенным акцентом на synthetic data generation.

Vision-черновик `architecture_vision.md` сохраняется **как есть** для исторической ценности — «как я думал на берегу». Этот документ — **то, что мы делаем теперь зная литературу**.
