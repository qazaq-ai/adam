# Математические основания agglutinative-neural

**Дата:** 2026-05-15 (Phase 0 Day 2, скорректированный темп).
**Статус:** v0.1 — focused, не exhaustive. Только то, что нужно для tokenizer + training-pipeline. Расширения по мере необходимости.

> Документ покрывает 5 формализаций: (1) морфема как тип, (2) слово как кортеж морфем, (3) FST как функция, (4) algebraic loss как сумма штрафов с FST-detectable нарушениями, (5) verifier как функция-предикат. Этого достаточно, чтобы начать программировать tokenizer и формулировать loss.

---

## 1. Морфема как типизированный элемент

### 1.1 Базовое определение

**Морфема** в нашей системе — это пара $m = (s, \tau, \phi)$, где:
- $s \in \Sigma^*$ — поверхностная форма (строка казахских графем).
- $\tau \in \mathcal{T}$ — тип морфемы (`Root | Plural | Case(c) | Possessive(p) | Predicate(p) | Voice(v) | Negation | Tense(t) | Person(p) | Polite`).
- $\phi$ — структурные features (vowel class, voicing class, position role).

Множество всех морфем нашего языка обозначаем $\mathcal{M}$. На v5.32.0:

$$
|\mathcal{M}| = |\mathcal{M}_{\text{root}}| + |\mathcal{M}_{\text{suffix}}| + |\mathcal{M}_{\text{service}}| \approx 25\,500 + 50 + 50 \approx 25\,600
$$

(Note: pre-lit оценка была ~30k. Точнее ~25.6k, потому что суффиксов меньше чем казалось — они **типизированы**, а не **сурфейс-уникальны**. Один «Plural» token покрывает все вариации `-лар/-лер/-дар/-дер/-тар/-тер/-нар/-нер` — выбор делает FST на этапе synth.)

### 1.2 Vowel harmony как кластер-отношение

Каждая морфема имеет vowel class $h(m) \in \{\text{Front}, \text{Back}, \text{Neutral}\}$. Для root — фиксирован Lexicon'ом; для suffix — varies based on stem (front/back archiphonemes).

**Vowel harmony constraint** в казахском:

$$
\forall i, j : \text{ если } h(m_i) \neq \text{Neutral} \text{ и } h(m_j) \neq \text{Neutral}, \text{ то } h(m_i) = h(m_j)
$$

То есть: все морфемы слова, имеющие нетривиальный vowel class, должны быть одного класса. Это **constraint**, не закон вероятности — нарушение делает слово фонетически невалидным.

В нашей системе **FST уже это enforce'ит**: synthesise выбирает правильный allomorph суффикса в зависимости от $h$(root). Tokenizer не нарушает это; algebraic loss penalises model'ы, которые предлагают $m_j$ с конфликтным $h$.

### 1.3 Voicing harmony как вторичное constraint

Для некоторых суффиксов есть второй constraint:

$$
v(m_j) = f(\text{final\_phoneme}(m_{j-1}))
$$

где $v(m_j) \in \{\text{Voiced}, \text{Voiceless}, \text{Sonorant}\}$. Пример: суффикс множественного числа `-лар/-дар/-тар` — выбор зависит от того, как заканчивается предыдущая морфема (Sonorant → `-лар`, Voiced consonant → `-дар`, Voiceless → `-тар`).

Это **дискретная функция от предыдущей морфемы**. FST решает её. Tokenizer-уровневые токены типизированные, а не сурфейс-вариативные — поэтому tokenizer **не видит** эти allomorphs; они скрыты за типом.

---

## 2. Слово как кортеж морфем

### 2.1 Каноническое разложение

Слово $w$ — это упорядоченный кортеж морфем:

$$
w = (m_1, m_2, \ldots, m_n)
$$

где $m_1$ — root (POS-determining), $m_2, \ldots, m_n$ — суффиксы в порядке morphotactic stacking.

**Tokenization function** $T : \Sigma^* \to \mathcal{M}^*$ — отображение surface формы в кортеж морфем. В нашей системе $T$ реализован через существующий `parser::analyse`:

$$
T(w) = \arg\max_{\text{analysis} \in \text{analyse}(w)} \text{rank}(\text{analysis})
$$

(где rank — детерминистический tie-break: lexicographic root + Lexicon id).

### 2.2 Round-trip property

**Synthesis function** $S : \mathcal{M}^* \to \Sigma^*$:

$$
S(m_1, m_2, \ldots, m_n) = \text{surface form via FST synthesise}
$$

**Round-trip property** — главное требование к нашему tokenizer'у:

$$
\forall w \in \mathcal{L} : S(T(w)) = w
$$

где $\mathcal{L}$ — подмножество казахского, покрываемое Lexicon'ом. Для всех слов из Lexicon — FST round-trip уже работает в production (мы используем `analyse` + `synthesise` через каждый dialog turn).

Слова вне $\mathcal{L}$ (proper nouns не в Lexicon, foreign words, неологизмы) — отдельный случай для OOV-handling. **Phase 0 prototype не handle'ит OOV**, эта задача в Phase 1.

### 2.3 Алгебра морфем

Конкатенация morphemes имеет следующие алгебраические свойства:

**(а) Закрытость:** $\oplus : \mathcal{M}^* \times \mathcal{M}^* \to \mathcal{M}^*$ (concatenation очевидно даёт ещё одно сlovo, если итог в $\mathcal{L}$ — иначе invalid).

**(б) Identity:** $w \oplus \epsilon = w$ (пустой суффикс ничего не меняет).

**(в) Не-коммутативность:** $m_1 \oplus m_2 \neq m_2 \oplus m_1$ в общем случае. Порядок stack'инга суффиксов фиксирован morphotactic'ами:

$$
\text{root} \to \text{derivation} \to \text{number} \to \text{possessive} \to \text{case} \to \text{predicate}
$$

**(г) Частичная коммутативность:** некоторые пары (e.g., case + possessive в некоторых порядках) могут переставляться, давая разные сurface формы, но эквивалентный SemFrame. Это **тонкий момент** для tokenizer'а — мы фиксируем canonical порядок и поляризуем модель только на нём.

---

## 3. FST как функция

### 3.1 Формальное определение

FST $F$ — это finite-state transducer, моделирующий частичную функцию:

$$
F : \Sigma^*_{\text{deep}} \to \Sigma^*_{\text{surface}}
$$

где deep representation — это морфемная конкатенация (root + типизированные суффиксы), surface — реальная казахская поверхностная форма.

В нашей реализации FST разделён на два слоя:
- **Morphotactics:** разрешает stack'инг (root → suffix1 → suffix2 → ...).
- **Phonology:** applies allomorph selection (vowel harmony, voicing assimilation, vowel elision).

### 3.2 FST-decode constraint

Для PICARD-pattern decoder'а мы нуждаемся в функции:

$$
\text{valid\_next} : F_{\text{state}} \times \mathcal{M} \to \{\text{True}, \text{False}\}
$$

То есть: дан текущий state FST'а и кандидат-морфема — может ли она быть следующей? Эта функция **constant-time** (FST state — это просто текущая позиция в morphotactic graph + накопленный vowel class).

На decoder'е:

$$
P(m_{n+1} = m \mid m_1, \ldots, m_n) = \begin{cases}
\text{softmax}(z_m) & \text{если valid\_next}(F_{\text{state}}, m) = \text{True} \\
0 & \text{иначе}
\end{cases}
$$

То есть: log-probabilities для невалидных морфем zero'д out **до** softmax, а не после. Это PICARD-pattern (Scholak et al. 2021). На каждом шаге активные кандидаты — это типично 5-50 морфем из 25.6k vocab → **500-5000× уменьшение эффективного output space**.

---

## 4. Algebraic loss

### 4.1 Стандартный seq2seq cross-entropy

$$
L_{\text{CE}}(\theta) = -\sum_{t=1}^{T} \log P_\theta(m_t^* \mid m_{<t}, x)
$$

где $m_t^*$ — gold-truth морфема, $x$ — input (proof_object у нас).

### 4.2 Algebraic penalties

К стандартному CE добавляем **detectable** через FST штрафы:

**(а) Vowel harmony violation:**

$$
P_{\text{VH}}(m_1, \ldots, m_n) = \sum_{i,j} \mathbb{1}[h(m_i) \neq \text{Neutral} \wedge h(m_j) \neq \text{Neutral} \wedge h(m_i) \neq h(m_j)]
$$

**(б) Suffix chain validity:**

$$
P_{\text{SV}}(m_1, \ldots, m_n) = \sum_{i=2}^{n} \mathbb{1}[\neg \text{morphotactic\_valid}(m_{i-1}, m_i)]
$$

**(в) Provenance consistency** — для factual claims:

$$
P_{\text{PV}}(\text{claim}, \text{proof}) = \mathbb{1}[\text{claim} \not\subseteq \text{proof.support}]
$$

### 4.3 Полная loss-функция

$$
L_{\text{total}}(\theta) = L_{\text{CE}}(\theta) + \alpha \cdot \mathbb{E}[P_{\text{VH}}] + \beta \cdot \mathbb{E}[P_{\text{SV}}] + \gamma \cdot \mathbb{E}[P_{\text{PV}}]
$$

с гиперпараметрами $\alpha, \beta, \gamma \approx 1\text{-}10$ (подбор экспериментально).

### 4.4 Differentiability

$P_{\text{VH}}, P_{\text{SV}}$ — индикаторные функции, не дифференцируемые. **Два способа** инкорпорировать:

**(а) REINFORCE-style:** treat penalty как reward signal в RLVR-семействе (Lambert et al. 2024, Tulu-3). Использовать как baseline-вычитаемый reward в policy gradient.

**(б) Constrained decoding eliminates VH/SV violations at decode time → no need for differentiable penalty during training.** Это **наш approach по умолчанию**: FST hard-constraint на decode → penalty = 0 архитектурно. Backprop только на $L_{\text{CE}}$.

**Решение:** начинаем с (б) — FST hard-constraint на decode + чистый CE. Если diversity collapse случается → переходим на (а) с soft-constraint + RL signal.

---

## 5. Verifier как функция-предикат

### 5.1 Формальное определение

$$
\text{Verifier} : (\text{Claim}, \text{ProofObject}) \to \{\text{True}, \text{False}\}
$$

где:
- $\text{Claim}$ — typed semantic structure (subject, predicate, object).
- $\text{ProofObject}$ — структура $\{\text{support}: \text{List}[\text{Fact}], \text{shape}: \text{IntentShape}, \ldots\}$.

Verifier returns True, если:

$$
\exists f \in \text{ProofObject.support} : f \text{ supports } \text{Claim}
$$

где «$f$ supports Claim» определено через типизированную unification (см. существующий `verifier.rs`).

### 5.2 Architectural gate

В отличие от RLHF-soft подходов (GopherCite, Self-RAG), наш Verifier — **hard gate** перед emit:

```
Neural output: morpheme sequence M
              ↓
Reconstruct: extract semantic claim from M
              ↓
If Claim factual:
    if Verifier(Claim, ProofObject) == False:
        BLOCK emit; return fallback «дерек жоқ»
    else:
        emit M
If Claim non-factual (greeting, ack, etc.):
    emit M
```

Это означает: модель **физически не может** вывести factual claim, который не покрыт `ProofObject.support`. Это не «training discouraged» — это **post-generation block**.

### 5.3 Trade-off

Hard gate vs soft preference:
- **+** Architectural guarantee: 0 hallucinated facts at runtime.
- **+** Inspectable: каждый блок логируется как «verifier rejected; claim X, support insufficient».
- **−** Возможны блокировки valid claims, если verifier consistency check недостаточно flexible.
- **Mitigation:** verifier работает на типизированных claims, не на сurface strings; типизированная unification более liberal чем string matching.

---

## 6. Резюме формальной спеки

| Symbol | Что | Где реализован |
|---|---|---|
| $\mathcal{M}$ | Множество морфем (~25.6k tokens) | tokenizer crate (будет) |
| $T(w)$ | Tokenization: surface → morpheme tuple | `parser::analyse` ✅ |
| $S(\vec m)$ | Synthesis: morpheme tuple → surface | `morphotactics::synthesise_*` ✅ |
| $h(m), v(m)$ | Vowel class / voicing class | `RootEntry.vowel_harmony` / `final_sound_class` ✅ |
| $F$ | FST для constrained decode | `lexicon` + `morphotactics` ✅ |
| $L_{\text{CE}}, P_{\text{VH}}, P_{\text{SV}}, P_{\text{PV}}$ | Loss components | Phase 2 (PyTorch training) |
| `Verifier(Claim, Proof)` | Architectural gate | `crates/adam-dialog/src/verifier.rs` ✅ |

**Что готово:** $T, S, h, v, F, \text{Verifier}$ — всё уже в main-ветке, нужно только wrapper'ы для нейро-вывода.

**Что строим:** morpheme tokenizer (Day 3 prototype), потом PyTorch training pipeline (Phase 1+).

---

## 7. Эпиграф

Стандартный LLM учит грамматику через миллионы примеров — **implicit knowledge**. Мы encode'им её прямо в loss и decode-constraint — **explicit invariants**. Это даёт inductive bias, который, если работает, делает 10M params достаточными там, где LLM нужны миллиарды. Это и есть «третий путь».
