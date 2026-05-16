# Архитектурное видение agglutinative-neural (черновик Day 1)

**Статус:** черновик пера v0.1, 2026-05-15 (Phase 0 Day 1). Параллельно с lit-scan, чтобы потом сравнить «что я думал ДО литературы» с «что я думаю ПОСЛЕ».

> Цель документа — зафиксировать первую гипотезу до того, как литература сместит мышление. После lit_scan'а вернёмся и пересмотрим.

---

## Тезис в одной строке

**adam-нейро — это не «маленький LLM для казахского». Это система композиции, которая использует weights и tokens, чтобы выбирать КАК сказать то, что детерминистический слой уже решил сказать.**

Граница чёткая: **факты — детерминистические**. **Композиция фраз — нейро**. Verifier между ними не даёт нейро-слою изобретать факты.

---

## Three-layer architecture

```
┌─────────────────────────────────────────────────────────────────┐
│  L0 — DETERMINISTIC FACTUAL CORE (main, unchanged)             │
│                                                                 │
│  Input → intent → topic extraction → fact retrieval →           │
│        → reasoning chain → proof_object{claim, support}         │
│                                                                 │
│  Output: typed semantic graph + provenance                      │
└──────────────────────────┬──────────────────────────────────────┘
                           │ frozen factual content
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│  L1 — NEURO-COMPOSITIONAL LAYER (new)                          │
│                                                                 │
│  proof_object  +  user context (session, register, formality)   │
│                ↓                                                │
│  morpheme-level small AGT (≤10M params, CPU-first)              │
│                ↓                                                │
│  sequence of typed morphemes                                    │
└──────────────────────────┬──────────────────────────────────────┘
                           │ morpheme sequence
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│  L2 — DETERMINISTIC SURFACE FORMER (FST, unchanged)            │
│                                                                 │
│  morpheme sequence → FST synth → surface text                   │
│                    ↓                                            │
│  verifier checks: claim ⊆ proof_object support set              │
│                    ↓                                            │
│  rendered Kazakh text + provenance citation                     │
└─────────────────────────────────────────────────────────────────┘
```

**Ключевая инвариантность:** между L0 и L1 — frozen контракт. L1 НЕ ВИДИТ внешнего мира, не делает retrieval, не получает текстовых документов. Видит только структурированный `proof_object` от L0. Поэтому изобрести факт **архитектурно невозможно** — у L1 нет источника фактов кроме того, что выдал L0.

---

## Что выбирает L1 (декомпозиция)

Когда L0 выдал, например:

```
proof_object {
  claim: { subject: "лейтенант", predicate: "is_a", object: "кіші офицер" },
  support: [fact_id: "mil_kz_024", source: "adilet:Z1200000561/art19.2"],
  question_shape: YesNoCheck,
  register: formal,
  user_addressed_form: respectful,
  session_anaphora: [...]
}
```

L1 должен выбрать **последовательность морфем** для выдачи. Конкретные точки выбора:

1. **Шаблон ответа.** «Иә, X — Y. Дәлел тізбегі: X → Y.» vs «X шынында Y. Бұл туралы мынадай дерек бар: …» vs «Расында да, X — Y.». Это рангирование шаблонов (что уже частично делает selection-ranker, но perceptron-уровень).

2. **Лексический выбор.** Из синонимов, которые в Lexicon помечены как эквивалентные — какой брать? «Лейтенант» vs «офицер». «Иә» vs «дұрыс» vs «расында». Выбор зависит от register, formality, session-anaphora.

3. **Морфологические окончания на slot'ах.** В шаблоне `"{name|respect+vocative}"` — слот выбирает FST-форму, но **типизированной операцией**. Сейчас детерминистически (одна правильная форма). С L1 — можем выбирать между несколькими валидными формами по контексту (например, более длинная вежливая «-ыңыз» vs более лаконичная «-сың» по register).

4. **Связки/частицы.** «Иә, ... екен», «Иә, ... болады», «Иә, ... екенін айтайын». Выбор оттенка модальности на основе verifier confidence + session state.

5. **Длина и распространённость.** «Иә, лейтенант — офицер» (минимум) vs «Иә, лейтенант — кіші офицер, ал ол өз кезегінде офицер ретінде есептеледі» (распространённое объяснение). Выбор на основе детектированного level пользователя (если есть session: первокурсник vs курсант старший vs офицер запаса).

**Чего L1 НЕ выбирает:** факт. Никогда. Если в `proof_object.support` нет источника — L1 не имеет права произвести factual клейм. В худшем случае выдаст «Кешіріңіз, бұл туралы дерек жоқ».

---

## Vocabulary дизайн

### Концепт «typed morpheme token»

Каждый token имеет:

```rust
struct AggToken {
    id: u32,                        // dense integer for embedding lookup
    surface: String,                // visible form, e.g. "лер" (plural)
    morpheme_type: MorphemeType,    // Root | Plural | CaseSuffix | PossSuffix | ...
    features: TokenFeatures {       // FST-derived structural features
        vowel_class: VowelClass,    // Front | Back | Neutral
        voicing: Voicing,           // Voiced | Voiceless | Sonorant
        position_role: PositionRole, // RootSlot | Suffix(n)
        is_question_marker: bool,
        ...
    }
}
```

Embedding = `concat(id_embedding, type_embedding, feature_embeddings)`. Это даёт:
- **inductive bias**: одинаково-типизированные морфемы лежат рядом в embedding space (все падежные суффиксы → cluster).
- **inspectable**: каждое измерение embedding соответствует **именованному феномену** (vowel_class, voicing, etc.), не диффузному «байт-уровню».

### Соотношение к нашему Lexicon

| Источник | Количество |
|---|---|
| Roots в `data/lexicon_v1/`+`data/lexicon/` | ~25 500 |
| Суффиксы (case × number × possessive × tense × …) | ~4 000 уникальных морфемных вариантов |
| Служебные (пунктуация, числа, специальные) | ~500 |
| **Total vocab** | **~30 000 typed tokens** |

Сравнение:
- BPE-словарь Llama-3 = 128 000 байтовых subword'ов.
- BPE-словарь KazLLM = 64 000 (унаследован, ломает казахскую морфологию посередине).
- Наш = 30 000 **семантически осмысленных морфем**, не байтов.

### Inductive bias через гипотезу

**Гипотеза:** агглютинативная композиция в казахском подчиняется **математически чистой алгебре**, не «вероятностной близости». Конкретно:

- Концатенация морфем — это **группа** с identity (пустой суффикс) + inverse (некоторые морфемы взаимно отменяют, например двойное отрицание).
- Vowel harmony — это **constraint relation** на vowel_class, эквивалент сужения domain.
- Possessive + Case — это **commutative pair** в certain orderings.
- Plurality — это **monad-like operator** (добавляется один раз, дальнейшие нолевые).

Если мы **закодируем эти инварианты в архитектуру** (а не учим заново через миллион примеров), модель получает inductive bias, который в 100× сокращает нужное количество примеров для генерализации. Phi-3 показал, что **«textbook quality» данные** > «массовое количество». Мы идём дальше: **«algebraically-correct» данные через FST-генератор** > «textbook».

---

## Training data — синтетический FST corpus

У нас уже есть FST, который умеет:

```
synth(root: "бала", plural: true, case: Dative) → "балаларға"
```

→ Можем сгенерировать **бесконечный** корпус морфологически валидного казахского. Не «обучаемся на интернете» — обучаемся на собственной алгебре.

**Конкретный pipeline:**

1. Из Lexicon 25.5k корней + FST → синтез всех valid suffix-chain'ов на каждом корне = **~50M уникальных словоформ**.
2. Из этих словоформ + templates (43 семейства, 144 sub-families) → **~10M уникальных коротких фраз**.
3. Из proof_objects (3650 курируемых фактов) → **~3-5M уникальных Q&A-пар** (factual + paraphrase variants).
4. Live REPL transcripts + adversarial battery → **~10k human-evaluated пар** для finetune.

Это даст ~63M training-пар без единой запинки в интернет. Каждая пара построена из правил, которые мы пишем сами. **Контролируемая чистота.**

Сравнение масштабов:
- Llama-3 обучен на ~15 trillion токенов.
- Phi-3 — на ~3.3 trillion (но «textbook-curated»).
- KazLLM — на ~80B tokens (Common Crawl + Kazakh subset).
- **Наш target — 63M пар** (~10B токенов морфемного уровня). **1500× меньше Phi-3.**

**Вопрос:** хватит ли 63M пар на 10M-параметровую модель? Из правила Chinchilla (~20 tokens per param) — для 10M params нужно ~200M tokens. У нас 10B — overkill в 50×, но это здоровый запас.

---

## FST-guided decoding

Стандартный transformer decoder делает softmax по 30k vocab → choose top-k. Наш decoder делает **то же**, но softmax работает **по подмножеству morphemes, валидных в текущей позиции** согласно FST.

Пример. После «бала» + «лар» (root + plural) FST знает: следующие валидные слоты — possessive или case. Из 30k vocab остаётся ~50 валидных tokens. Softmax работает на этих 50.

**Преимущества:**
- Архитектурная **невозможность** произвести «балаларка» (несуществующая форма).
- 50× уменьшение **effective output space** на каждый шаг → быстрее inference.
- 50× **бóльшая концентрация вероятностной массы** на правильных токенах → меньше нужно training data.

**Прецеденты для проверки в lit-scan:**
- Hokamp & Liu 2017 «Lexically Constrained Decoding»
- PICARD 2021 (Scholak et al.) — incremental parser-constrained decoding для SQL
- Geng et al. 2023 «Grammar-Constrained Decoding»

Это известная техника, но **FST-constrained в агглютинативной семье** — мы возможно первые на open-source.

---

## Algebraic loss

Стандартный seq2seq loss = `CrossEntropy(predicted_token, gold_token)`.

Наш loss =

```
L = α · CrossEntropy(predicted, gold)                  [standard]
  + β · VowelHarmonyPenalty(predicted_sequence)        [vowel harmony violations]
  + γ · SuffixChainValidityPenalty(predicted)          [invalid morphotactics]
  + δ · ProvenanceConsistencyPenalty(predicted, proof) [factual claims w/o support]
  + ε · MorphologicalAmbiguityRegularizer(predicted)   [optional: prefer canonical form]
```

Каждый штраф — **FST-detectable**, не learned. То есть мы не учим сеть, что vowel harmony важен, мы **наказываем её математически** при каждом её нарушении. Это превращает агглютинативную алгебру в **direct training signal**.

Сравнение: типичные seq2seq модели учат грамматику через миллионы примеров (implicit). Мы encode её прямо в loss (explicit). Должно дать **порядковое ускорение convergence**.

---

## Размерность модели — детальный бюджет

Target 10M params (для PoC), вариативные точки:

| Гиперпараметр | Значение | Контрибуция в params |
|---|---|---|
| Embedding dim | 256 | 30k × 256 = 7.7M |
| Layers | 4 | — |
| Heads | 4 | — |
| FFN hidden | 1024 | 4 × (256·1024 + 1024·256) = 2.1M |
| Attention | scaled-dot | 4 × 4 × (256·64·2) = 130K |
| Output head | tied with embedding | 0 (reuse) |
| **Total** | | **~10M** |

Сравнение:
- Llama-3 8B: 800× больше.
- Phi-3 mini 3.8B: 380× больше.
- TinyLlama 1.1B: 110× больше.
- Apple on-device 3B: 300× больше.

10M — это **территория BERT-tiny** (4.4M params, 2020). Но BERT-tiny решал NER. Мы целимся в narrow-Kazakh-conversation. **Гипотеза**: на узком домене с algebraic loss + FST-guided decoding + curated data — 10M достаточно.

**Если не достаточно** — masштабируем до 50M (стандартный «small BERT» class) → 100M (BERT-base class). До этого момента всё inference-friendly на M2 CPU.

---

## Inference flow на CPU (без GPU)

```
input: "Лейтенант — офицер ме?" (~20 bytes)
            ↓
[L0 deterministic, FST + retrieval] ~21 ms (existing main perf)
            ↓
proof_object { claim, support, ... }
            ↓
[L1 small AGT, 4-layer transformer, ~10M params]
  - tokenize proof_object → ~30 morpheme tokens
  - 4 transformer layers on M2 CPU @ 256 dim → ~150 ms
  - FST-guided decoding, ~20 output morphemes → ~50 ms
  - total L1: ~200 ms
            ↓
morpheme sequence: ["иә", "лейтенант", "—", "офицер", ".", "дәлел", ...]
            ↓
[L2 FST synthesizer] ~5 ms
            ↓
"Иә, Лейтенант — офицер. Дәлел тізбегі: лейтенант → кіші офицер → офицер."
```

**Total p50 budget**: ~225 ms. Цель ≤ 200 ms — почти укладываемся. Если не — можно оптимизировать через quantization (int8 → ~4× speedup), что приведёт нас к ~50 ms.

**С GPU** (M2 Ultra, RTX 3090, M2 Pro): L1 → ~20-40 ms. Total ~50 ms. Это полезно, но **не критично** для основного юзкейса.

---

## Чего мы хотим избежать

**Failure mode 1: «Просто маленький LLM, который чуть хуже большого».**
- Триггер: если модель в эвалюации показывает «Phi-3 mini × 0.6» — мы проиграли. Мы должны **уметь то, чего Phi-3 mini не умеет**, а не «то же самое, но похуже».
- Защита: чёткий differentiator — *FST-guaranteed валидность + provenance-bound factuality*. Phi-3 этого не даёт. Наш eval должен включать «Find me a Phi-3 hallucination, demonstrate adam refuses to make same claim».

**Failure mode 2: «Излишняя жёсткость, нет fluency».**
- Триггер: FST-guided decoding ограничивает так сильно, что модель звучит роботизированно. Не пройдёт human eval.
- Защита: разрешать L1 выбирать между несколькими valid forms, дать ему регистра / возраста / контекста выбор. Templates с variation.

**Failure mode 3: «Overfit на synthetic FST corpus».**
- Триггер: модель блестит на synthetic, но проваливается на live human Kazakh.
- Защита: live REPL transcript'ы + adversarial human-evaluated set с самого начала. Synthetic — это **inductive bias setup**, не финальная цель.

**Failure mode 4: «Эксперимент успешен, но в production такой шаг нельзя».**
- Триггер: PoC показал улучшение, но требует кардинальной переделки L0 / L2, нарушает существующих пользователей.
- Защита: L1 проектируется как **PURE ADD-ON** к существующему main. main без него работает как сейчас. Включить L1 — opt-in флаг `--use-neural-composition`.

---

## Что я хочу проверить в lit_scan (запрос к Day 1 agent)

После возврата literature scan'а — спорные точки моего вышеизложенного:

1. **FST-constrained decoding в агглютинативной семье** — точно ли никто не делал? Турция?
2. **Algebraic loss с FST-detection** — есть ли прецеденты? Финский? Корейский?
3. **«Phi-mini для one language» бюджет** — реально ли 10M достаточно? Какие precedent'ы?
4. **Morpheme-token embedding vs BPE** — empirical results? Что показал Воikko для финского?
5. **Symbolic verifier поверх neural** — какие architecture'ы в production? Microsoft Grounding?
6. **Self-distillation from FST** — есть ли прецеденты использования FST как teacher для neural student?

---

## Открытые вопросы (для дискуссии)

1. **Tokenization at train time vs inference time.** Sentence «Лейтенант — офицер ме?» tokenize'итcя FST'ом или есть pre-baked split? У FST есть ambiguity для некоторых форм (пара возможных decompositions). Resolve at training time (data preprocessing) or at inference (slower)?

2. **Decoder vs encoder-decoder.** Чистый decoder (GPT-style) проще, но для conditional generation (proof_object → text) encoder-decoder (T5-style) теоретически лучше. Phi mini — pure decoder. Что брать?

3. **Quantization strategy.** int8 quantization vs float16 vs full float32 — где компромисс качества и скорости лучше для нашего юзкейса?

4. **Multi-task vs single-task.** Учить ОДНУ модель на (а) compose dialog reply, (б) compose explanation, (в) compose Q&A — multi-task. Vs специализированные модели per task. Бюджет позволит ли?

5. **Training time на M2 8GB.** Если 100M params + 10B tokens — это сколько дней? Если не помещается в RAM — gradient checkpointing? Может быть нужен один интенсивный спринт на RTX 3090 в облаке (одноразово, до production)?

   **Wait — мы запретили cloud.** Значит local-only training. Нужно проверить: 10M params x 10B tokens = ~M2 8GB на сколько недель? Или сразу думаем про локальный RTX-class GPU как покупку?

---

## Что в следующих документах

- `lit_scan.md` (от research agent) — что мир уже сделал; чем мы отличаемся.
- `math_foundations.md` (Day 2) — формализация: vector space морфем, vowel harmony как constraint, loss formulation.
- `tokenizer_spec.md` + код `crates/adam-agg-tokenizer/` (Day 3) — первая реальная имплементация.

Этот документ (`architecture_vision.md`) — black-box-видение, чтобы после lit_scan'а я мог сравнить и честно сказать, где я ошибался / где идея банальная / где есть реальная новизна.
