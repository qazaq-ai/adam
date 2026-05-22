# Литературный скан: позиционирование adam-agglutinative-neural

**Статус:** research input для 12-недельной программы (ветка `experimental/agglutinative-neural`)
**Дата:** 2026-05-16

> Целевой скан под нашу гипотезу: ≤10M-параметрный морфемно-токенизированный казахский генератор с FST-guided decoding и verifier-bounded factuality. Каждое утверждение снабжено ссылкой; где данных нет — указано явно.

---

## 1. Малые языковые модели (≈10M-1B) — state of the art 2023-2026

Микрософтовское семейство **Phi** — главный референс. Phi-1 (1.3B) описан в Gunasekar et al. «Textbooks Are All You Need» (arXiv:2306.11644, июнь 2023, <https://arxiv.org/abs/2306.11644>): модель училась на ~7B токенов synthetic «textbook-quality» Python-кода и побила гораздо большие модели на HumanEval. Главная мысль: **качество данных доминирует над масштабом**.

Phi-1.5 (1.3B, sept 2023, <https://arxiv.org/abs/2309.05463>) обобщил рецепт на natural-language reasoning. Phi-3-mini (3.8B, apr 2024, <https://arxiv.org/abs/2404.14219>) и Phi-4 (14B, dec 2024, <https://arxiv.org/abs/2412.08905>) показали, что **synthetic + filtered web** даёт лучшие numerical-reasoning баллы, чем 10× больший Llama на raw Common Crawl. Прямой прецедент для нашего «18k курируемых + FST-синтетика».

**Mistral 7B** (Jiang et al. 2023, <https://arxiv.org/abs/2310.06825>) — grouped-query attention + sliding-window для CPU/RAM-эффективности. **Llama-3** (Meta 2024, <https://arxiv.org/abs/2407.21783>) выпустила 1B и 3B варианты как edge-kit. **TinyLlama-1.1B** (Zhang et al. 2024, <https://arxiv.org/abs/2401.02385>) — публично доступный baseline для «как мало хватает».

**Pythia** (Biderman et al. 2023, <https://arxiv.org/abs/2304.01373>) — checkpoints от 70M до 12B; **Chinchilla scaling laws** (Hoffmann et al. 2022, <https://arxiv.org/abs/2203.15556>) рекомендуют ≈20 токенов на параметр.

**Apple Intelligence on-device** (~3B, WWDC 2024, <https://machinelearning.apple.com/research/introducing-apple-foundation-models>) — production-демонстрация 3B на нейроядре телефона: LoRA-адаптеры, 4-bit квантизация.

**TinyStories** (Eldan & Li 2023, <https://arxiv.org/abs/2305.07759>) — **самый прямой прецедент**. Модели **1-33M параметров** научились писать связные английские истории, после обучения на ~2M историй, сгенерированных GPT-3.5 в ограниченном словаре (~1500 слов). Вывод: для **narrow domain** 10M достаточно для грамматически связной генерации. **Сильнейший аргумент в пользу нашего 10M-PoC**.

**Эмпирический пол:** TinyStories показывает 10M как нижнюю границу связной генерации в **ограниченном domain и vocab**. Для open-domain нужно ≥1B (Pythia-160M плохо генерирует на free text). Наш сценарий — Kazakh tutor (узкий domain) + 30k morpheme vocab (узкий vocab) — попадает ровно в зону TinyStories-feasibility.

## 2. Морфемно-уровневая токенизация за рамками BPE

**BPE** (Sennrich et al. 2016, <https://arxiv.org/abs/1508.07909>) и **SentencePiece/Unigram** (Kudo 2018, <https://arxiv.org/abs/1804.10959>) разрезают слова частотно-статистически. Для агглютинативных языков это документированная проблема: Bostrom & Durrett 2020 («BPE is Suboptimal for Language Model Pretraining», <https://arxiv.org/abs/2004.03720>) показали, что Unigram превосходит BPE на турецком и финском, но **даже Unigram режет морфемы внутри**. Park et al. 2021 для корейского NMT (<https://aclanthology.org/2021.findings-acl.41/>) — morpheme-aware токенизация даёт +1-3 BLEU.

**Morfessor** (Creutz & Lagus 2002, <https://aclanthology.org/W02-0603/>; Smit et al. 2014, <https://aclanthology.org/E14-2006/>) — классический MDL-based unsupervised морфологический сегментатор, до сих пор baseline.

**Турецкий**: BOUN-NLP (<https://github.com/boun-tabi>) построили BERTurk (Schweter 2020, <https://huggingface.co/dbmdz/bert-base-turkish-cased>) на WordPiece. Toraman et al. 2023 («Impact of Tokenization on LMs: Analysis for Turkish», <https://arxiv.org/abs/2204.08832>) — morphology-aware tokenization снижает perplexity на ~7% при том же бюджете.

**Финский**: TurkuNLP FinBERT (Virtanen et al. 2019, <https://arxiv.org/abs/1912.07076>) — WordPiece; **Voikko/Omorfi** (Pirinen 2015, <https://aclanthology.org/W15-2305/>) — FST-морфоанализатор, иногда используется в препроцессинге **до** нейросетевого этапа.

**Корейский**: KoBERT, KR-BERT и большинство production-систем используют **morpheme-level vocab через MeCab-ko** как стандартную практику (Park et al. 2020, <https://arxiv.org/abs/2010.02534>). **Доказательство, что morpheme-primary tokenization работает в production** в agglutinative-семье.

**Японский**: MeCab + transformer — стандарт (Kudo 2006, <https://taku910.github.io/mecab/>); Tohoku BERT (<https://huggingface.co/cl-tohoku/bert-base-japanese>) — MeCab + WordPiece.

**Казахский**: **ISSAI KAZ-LLM 8B** (Nazarbayev University, 2024, <https://huggingface.co/issai/KAZ-LLM-8B>) использует **обычный SentencePiece-Unigram** (~64k vocab), inherited from Llama-3. Морфологически-aware токенизации **нет**. **Apertium-kaz** (<https://github.com/apertium/apertium-kaz>) — FST-морфология, без neural-интеграции. **Yandex Kazakh translation** — закрытое; по blog-постам BPE-based NMT.

**Прямой gap**: ни один из известных казахских NLP-проектов не использовал FST-paired morpheme tokenization для **генерации**.

**Метрики trade-off**: byte-level (ByT5, Xue et al. 2022, <https://arxiv.org/abs/2105.13626>) → ноль OOV, но длинные последовательности (×4); BPE → компактные, но семантически случайные границы; morpheme-level → семантически осмысленные, **но** требуют качественной морфологии (у нас она в production).

## 3. Constrained / grammar-guided decoding

**Constrained Beam Search** (Hokamp & Liu 2017, <https://arxiv.org/abs/1704.07138>; Post & Vilar 2018, <https://aclanthology.org/N18-1119/>) — заставляет вывод содержать заданные tokens.

**PICARD** (Scholak et al. 2021, EMNLP, <https://arxiv.org/abs/2109.05093>) — **самый сильный прямой прецедент для FST-decoding**. Парсер инкрементально валидирует partial SQL во время beam-search; невалидные continuations zeroed-out до softmax. На Spider — точность с 68% до 75%. **Архитектурный паттерн прямо переносится** на «FST validates partial Kazakh word during decoding».

**Grammar-Constrained Decoding (GCD)** (Geng et al. 2023, <https://arxiv.org/abs/2305.13971>) — обобщение PICARD до CFG; библиотеки **outlines** (<https://github.com/dottxt-ai/outlines>) и **lm-format-enforcer** (<https://github.com/noamgat/lm-format-enforcer>). Покрывают regular grammars и CFG; **FST-грамматика подвида regular, тривиально интегрируется**.

**Synchromesh** (Poesia et al. 2022, ICLR, <https://arxiv.org/abs/2201.11227>) — constrained code generation через target parser.

**OpenFst + neural hybrids**: явных peer-reviewed публикаций нет. Mohri/Riley/Allauzen FST-toolkit (<https://www.openfst.org/twiki/bin/view/FST/WebHome>) — инфраструктура без integration. В моём скане arxiv 2020-2026 **нет статьи «FST-constrained neural decoder for agglutinative language»** — это **гэп, который мы можем закрыть**.

**Output-space restriction как регуляризация** — Welleck et al. 2020 «Unlikelihood Training» (<https://arxiv.org/abs/1908.04319>) — explicit penalisation плохих continuations снижает repetition. Родственная идея нашему algebraic loss.

## 4. Нейро-символические гибриды

**DeepProbLog** (Manhaeve et al. 2018, NeurIPS, <https://arxiv.org/abs/1805.10872>) — Prolog + neural predicates, differentiable. **Logic Tensor Networks** (Serafini & Garcez 2016, <https://arxiv.org/abs/1606.04422>; Badreddine et al. 2022, <https://arxiv.org/abs/2012.13635>) — first-order логика как loss term. **Academic-grade прецеденты для нашего algebraic-loss**.

**Neuro-Symbolic Concept Learner** (Mao et al. 2019, ICLR, <https://arxiv.org/abs/1904.12584>); **NeurASP** (Yang et al. 2020, <https://arxiv.org/abs/2009.10256>).

В NLP: **MRKL Systems** (Karpas et al. 2022, AI21, <https://arxiv.org/abs/2205.00445>) — LLM как router к symbolic-tools. **Toolformer** (Schick et al. 2023, <https://arxiv.org/abs/2302.04761>) — neural выбирает, symbolic исполняет. **Это ровно наша архитектура**: neural выбирает формулировку, symbolic (FST + verifier) исполняет/валидирует.

**Apple «Cocoon»** — peer-reviewed референса не нашёл (название упомянуто в задаче, возможно internal codename). Apple opt for on-device LoRA-adapter-routing (WWDC 2024).

**IBM Watson** (Ferrucci 2010, <https://www.ibm.com/journals/rd/543/ferrucci.html>) — question decomposition → parallel hypothesis generators → confidence scoring. Этот «hypothesis-then-verify» паттерн — наш дизайн в миниатюре.

**Symbolic verifier post-neural**: **Self-RAG** (Asai et al. 2023, <https://arxiv.org/abs/2310.11511>) — reflection tokens. **Microsoft Grounding** (Phi-3 technical report, §5) — strict-mode prompting + post-hoc citation check. Наш `verifier::is_supported` + `proof_object` — **более жёсткая** версия: не post-hoc, а architectural gate.

## 5. Прецеденты в агглютинативных языках

**Турецкий**: BOUN-NLP, Yıldız Tech — десятилетия работы. TRMor2018 dataset, TRMOR morphology. **BERTurk** — WordPiece. Главный урок: morpheme-aware tokenization помогает on the margin (~5-10% perplexity), но **standard BPE доминирует в production** из inertia.

**Финский**: TurkuNLP FinBERT, Voikko — **отдельные стеки**, редко используются вместе. Финская academic NLP больше в parsing/POS, чем в generation.

**Венгерский**: huBERT (Nemeskey 2021, <https://hlt.bme.hu/en/resources/hubert>) — WordPiece; Szeged Treebank — корпусная традиция.

**Корейский**: KoBERT (<https://github.com/SKTBrain/KoBERT>), KoGPT — **morpheme tokenization через MeCab-ko как стандарт**. Park & Kim 2018 (<https://aclanthology.org/P18-1226/>) — +2-4 F1 на NER от morpheme-level. **Единственный agglutinative язык, где morpheme-primary стало production-default**.

**Японский**: MeCab + BERT — стандарт; Sudachi (<https://github.com/WorksApplications/Sudachi>) — production tokenizer.

**Казахский конкретно**:
- **ISSAI KAZ-LLM 8B** — Llama-3 fork, ~150B токенов, machine-translated + web. Bench: KazMMLU ~50%. **Tokenization standard Unigram, не morpheme-aware**.
- **Apertium-kaz** — FST через HFST/lttoolbox, Уральская школа (Washington, Tyers).
- **Yandex Translate Kazakh** — закрытая; BPE-based NMT.
- **KazNERD** (<https://github.com/IS2AI/KazNERD>) — NER dataset.
- **adam** — первая известная попытка morpheme-primary + FST-paired generation для казахского.

**Что worked**: morpheme-level в корейском, FST в финском parsing.
**Что didn't**: пытаться приклеить FST к large LLM post-hoc (Apertium-kaz never integrated with большой моделью).
**Cost**: KAZ-LLM — 8 GPU-недель + DPO; корейская morpheme-tokenization — десятилетие community-effort.

## 6. Open-vocabulary vs closed-domain trade-off

**«Textbooks Are All You Need»** (Gunasekar et al. 2023) — Phi-1 (1.3B) на 7B synthetic токенах побил Codex-12B на HumanEval. **В narrow domain высококачественные данные побеждают параметры**.

**TinyStories** (Eldan & Li 2023) — **наш ближайший template**. 1-33M параметров, vocab ~1500 слов, domain — детские истории. **GPT-2-mini (33M)** генерирует связные истории на synthetic narrow-domain корпусе. Если заменить «детские истории» на «казахские tutor-ответы» и vocab «1500 слов» на «30k морфем», масштаб задачи **сравним**.

**Synthetic-data through symbolic generator** — у нас FST + Lexicon **может породить миллионы валидных казахских предложений**. Аналог GPT-3.5-as-teacher в TinyStories, **но без зависимости от large model** — наш «teacher» — детерминистический FST. Прецедент: Wang et al. 2022 «Self-Instruct» (<https://arxiv.org/abs/2212.10560>) — synthetic data, но через LLM. **Чисто-symbolic synthetic-data generation для small LM training — прямого прецедента не нашёл**, потенциально novel.

**Self-distillation от large LM** — Hinton et al. 2015 (<https://arxiv.org/abs/1503.02531>), DistilBERT (<https://arxiv.org/abs/1910.01108>). **Мы это явно отвергаем** — distillation от LLM означает наследование LLM-галлюцинаций и web-bias.

## 7. Verifier-bounded генерация / RAG-strict

**RAG original**: Lewis et al. 2020 (<https://arxiv.org/abs/2005.11401>). Evolution: **REALM** (Guu et al. 2020, <https://arxiv.org/abs/2002.08909>), **Atlas** (Izacard et al. 2022, <https://arxiv.org/abs/2208.03299>), **Self-RAG** (Asai et al. 2023, <https://arxiv.org/abs/2310.11511>).

**Citation-enforced**: **GopherCite** (Menick et al. 2022, DeepMind, <https://arxiv.org/abs/2203.11147>) — выдаёт тексты только с supporting quotes. **Microsoft Phi-3 Grounding eval** (Abdin et al. 2024, §5) — измеряют ungrounded-claim rate, штрафуют в RLHF.

**LayerNorm/steering для anti-hallucination**: **DoLa** (Chuang et al. 2024, ICLR, <https://arxiv.org/abs/2309.03883>) — contrasting layer outputs снижает hallucination на 12-20% на TruthfulQA. **ITI** (Li et al. 2023, <https://arxiv.org/abs/2306.03341>) — стирание «лжи»-направлений в активациях.

Наш `proof_object` / `verifier::is_supported` — **более строгий** вариант: не «model trained to cite», а «architecturally cannot emit factual claim without citation». Ближайший academic-analog — **Constitutional AI strict-modes** (Bai et al. 2022, <https://arxiv.org/abs/2212.08073>), но они soft, у нас hard.

## 8. Алгебраическая / лингвистическая loss

**Auxiliary loss в seq2seq**: copy-mechanism (See et al. 2017, <https://arxiv.org/abs/1704.04368>), coverage (Tu et al. 2016, <https://arxiv.org/abs/1601.04811>) — infrastructure.

**Phonological-rule penalties в TTS** для турецкого/финского vowel harmony — **точечных публикаций о vowel-harmony как loss term не нашёл** в arxiv-skan 2020-2026. В TTS общая практика — **post-hoc filter**, не training-time penalty (Tacotron 2 <https://arxiv.org/abs/1712.05884>, VITS <https://arxiv.org/abs/2106.06103>).

**Symbolic-rule violation as training signal**:
- **Constitutional AI** (Bai et al. 2022, <https://arxiv.org/abs/2212.08073>) — rule-violation → preference signal.
- **RLVR — Reinforcement Learning from Verifiable Reward** (Lambert et al. 2024, Tulu-3, <https://arxiv.org/abs/2411.15124>) — точно наш паттерн: rule-checker как reward. Активная research-line 2024-2026.

**Vowel-harmony / suffix-validity как direct loss — прямого прецедента не нашёл**. Yarmohammadi et al. для морфологического inflection — но там целевая задача inflection, не gen-time penalty. **Потенциально новая угловая область**.

## 9. Честное конкурентное позиционирование

| Система | Размер | Tokenizer | Decoding | Корпус | CPU? | Казахский |
|---|---|---|---|---|---|---|
| Phi-3-mini | 3.8B | SentencePiece BPE | unconstrained | filtered web + synthetic | едва | плохо |
| Apple Intelligence | ~3B | BPE | LoRA-routed | curated + web | да (Neural Engine) | нет |
| Mistral 7B | 7B | BPE | unconstrained | web | едва | нет |
| TinyLlama | 1.1B | BPE | unconstrained | 3T web | да | очень плохо |
| ISSAI KAZ-LLM | 8B | Unigram | unconstrained | translated + web | нет | средне |
| TinyStories-GPT | 33M | word-level | unconstrained | synthetic narrow | да | нет |
| adam-main (prod) | 0 (rule-based) | FST морфемы | — | 18k curated | да | да, narrow |
| **adam-AGT (эта ветка)** | ≤10M | morpheme typed | FST hard-constrained | 18k + FST-synth | да | да |

**Где мы реально novel:**
1. **Morpheme-typed primary vocab + FST hard-constrained decoding в combination** — на казахском такого не было; «morpheme + FST hard constraint at every step» в peer-reviewed gen-системе **не нашёл** ни для одного языка.
2. **Verifier as architectural gate, не eval-time filter** — GopherCite ближе всех, но RLHF-soft.
3. **Symbolic-only synthetic data pipeline** — без LLM-teacher; **нет прямых прецедентов**.
4. **CPU-target на M2 с ≤10M params** для agglutinative gen — TinyStories доказала feasibility для английского, мы расширяем.
5. **Algebraic loss с FST-validation и vowel-harmony как direct training signal** — прямых прецедентов нет; адъяцент — RLVR (math/code, не phonology).

**Где мы уязвимы:**
- Если PoC выдаёт **связные, но скучные ответы** хуже existing template-based main — провал. **Цена входа: ответы должны быть лучше шаблонов**.
- 18k samples — **очень мало** для transformer даже с synthetic augmentation; риск overfitting + low diversity. TinyStories имела 2M.
- KAZ-LLM 8B уже существует; критики скажут «зачем 10M». Ответ: **8B не запускается на M2 8GB и галлюцинирует факты**; наш use-case — оффлайн tutor с verified facts.
- Если FST-constraint режет diversity слишком сильно — модель деградирует в template-equivalent. Получим то же, что main, но в 100× дороже compute.

## 10. Rust ML фреймворки

| Framework | URL | Зрелость 2026 | GPU | Inference | Training |
|---|---|---|---|---|---|
| candle | <https://github.com/huggingface/candle> | high; основной HF Rust | CUDA+Metal+WGPU | very good | OK |
| burn | <https://github.com/tracel-ai/burn> | high; modular backend | CUDA+Metal+WGPU+ndarray | good | very good |
| tch-rs | <https://github.com/LaurentMazare/tch-rs> | mature | full PyTorch | full | full |
| dfdx | <https://github.com/coreylowman/dfdx> | maintained, niche | CUDA | good | OK |

**candle** (HF, 2023+) — production-grade inference, поддерживает Llama, Mistral, Phi-3. Metal-backend на M2 работает (<https://github.com/huggingface/candle/tree/main/candle-examples>). Минус: training-API менее зрелый.

**burn** (Tracel AI) — самый продвинутый Rust-native training framework. Modular backend. Активная community (<https://burn.dev>). **Лучший выбор для training в pure Rust** на M2.

**tch-rs** — bindings к libtorch, full PyTorch, но 200+ MB dependency.

**dfdx** — compile-time tensor shapes; интересный safety-pattern, не production-scale.

**Рекомендация для adam-AGT**:
- **Training**: PyTorch (Python) на RTX 3090 (faster ecosystem, jupyter), затем **export в ONNX/safetensors**.
- **Inference**: **candle** с Metal-backend на M2. Native Rust интегрируется с существующим adam-кодом.
- **Альтернатива**: целиком на **burn** — pure-Rust pipeline; идеологически чище, но 2-3× медленнее research-iterations.

**Стандартный паттерн 2025-2026**: PyTorch → ONNX → candle (см. HF blog <https://huggingface.co/blog/candle>).

## 11. Datasets / data engineering прецеденты

**TinyStories pipeline**: vocab 1500 слов → GPT-3.5 «write story using only these words» → фильтр → 2.1M историй → train. **Наш FST-аналог**: vocab 30k морфем → FST-генератор «sample root from Lexicon, attach valid suffix chain, fill template slot» → миллионы предложений → rule-based семантический фильтр.

**Phi «textbook quality»** — filtered Stack-overflow + GPT-3.5 textbook examples. **Наш аналог**: curated 18k tutor pairs + FST-synthetic. Ключевое отличие: Phi полагается на GPT-3.5 (unbounded vocab, hallucination-prone teacher). **Мы — на детерминистический FST (bounded, hallucination-free teacher)**.

**Self-Instruct** (Wang et al. 2022, <https://arxiv.org/abs/2212.10560>); **OpenInstruct, Tulu-3** — продолжения.

**Symbolic-only synthetic for LM training** — прямого прецедента не нашёл. Hartmann et al. 2023 «SOTA on GEC without human data» (<https://arxiv.org/abs/2310.09696>) — rule-based perturbations, но GEC-task, не LM-training. **Это может быть наш cleanest novelty claim**.

## 12. Реалистичные бюджеты и сроки

**Training cost для 10M-100M на curated + synthetic**:
- TinyStories-33M на 2M историй: **1 GPU-день A100** (Eldan & Li, §4).
- Phi-1 1.3B на 7B токенов: **8 A100 × 4 дня** (Gunasekar, §3).
- Для нас: **10M params × ~100M synthetic токенов** ≈ **2-3 дня на RTX 3090**. Если 100M params → ~10-14 дней.

**Inference на M2 8GB**:
- candle Phi-2 (2.7B, 4-bit): ~10-20 tok/s (HF discussions).
- 10M-params: сотни-тысячи tok/s; latency p50 ≤200мс (наш бюджет) — **достижим с запасом**.

**Можно ли 1 человеку за 12 недель?** Честный breakdown:
- Недели 1-2: morpheme tokenizer + FST-synth pipeline (FST уже работает — реально).
- Недели 3-5: PyTorch transformer + algebraic loss + FST-constrained decoder. Реально для опытного, **тяжело** для новичка в трансформерах.
- Недели 6-8: training runs, hyperparam search, ablations. **Bottleneck = GPU time**. 3090 — OK для 10M.
- Недели 9-10: ONNX export, candle integration в adam-REPL.
- Недели 11-12: eval против main, отчёт, reproducible recipe.

**Реалистично:** PoC, работающий на narrow tutor-set с измеримым качеством vs main.
**Нереалистично:** open-domain Kazakh chat, побеждающий KAZ-LLM. Не пытайтесь.

**Главный risk:** **diversity collapse** — модель повторяет FST-template, ничего не добавляет сверх существующих шаблонов. Mitigation: hold-out eval с novel phrasings.

---

## Что у нас по факту НОВОГО

- **Morpheme-typed vocab × FST hard-constraint at every decoder step** — combination, которого нет в peer-reviewed литературе ни для одного агглютинативного языка (Korean — morpheme primary, но без FST hard-constraint; Finnish/Turkish — FST в анализе, не в decoding).
- **Verifier как architectural gate, не post-hoc filter** — `proof_object` блокирует emit любого factual claim без citation; ближайший прецедент GopherCite (RLHF-soft), у нас hard.
- **Symbolic-only synthetic-data pipeline** — FST + Lexicon генерируют training-corpus без LLM-teacher; устраняет hallucination-наследование, прямого аналога не нашёл.
- **Algebraic loss с vowel-harmony / suffix-validity как direct training signal** — для phonological rules в seq2seq training прямого прецедента нет (RLVR близок, но на math/code).
- **CPU-primary (M2 8GB) target для agglutinative generative LM** — TinyStories доказала feasibility для английского narrow-domain; мы расширяем на agglutinative + verifier-bounded; для казахского — первая попытка.

Если хотя бы 3 из 5 защитимы по итогам PoC — это публикация уровня ACL/EMNLP workshop. Если все 5 — full ACL.
