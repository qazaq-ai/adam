# Phase hybrid-wiring v4 — N-best + SMART verifier

**Pack:** `experiments/hybrid_qlm/paraphrase_eval/paraphrase_coverage_eval.json`
**Verifier:** topic+predicate noun preservation (asked-ABOUT vs asked-FOR distinction)

## Summary

- Deterministic baseline:        **81 / 100** (81%)
- Hybrid N-best + smart verifier:**85 / 100** (85%)
- Recoveries:                    **4 / 19** misses (21% recovery rate)
- Verifier rejections:           **17** candidates blocked at the gate
- Lift:                          **+4 pp**

### Comparison across hybrid versions

| Variant                     | Recoveries | Recovery rate | Lift  | Rejections |
| --------------------------- | ---------- | ------------- | ----- | ---------- |
| v1 single-shot (no verifier)| 4 / 19     | 21 %          | +4 pp | 0          |
| v2 N-best (no verifier)     | 9 / 19     | 47 %          | +9 pp | 0          |
| v3 N-best + naive verifier  | 5 / 19     | 26 %          | +5 pp | 17         |
| v4 N-best + smart verifier  | 4 / 19     | 21 %         | +4 pp | 17         |

### Per-prompt wins

- `synonym-swap`: 1 recoveries
- `rephrase-loose`: 2 recoveries
- `canonical-form`: 1 recoveries

## Recoveries (4)

### ✓ `Азот қышқылының формуласы.`

- variant: `Азот қышқылының белгісі қандай?`
- ✗ [synonym-swap] `Сірке қышқылының белгісі қандай?` → «Қысқаша айтсам, Сірке қышқылы — тағамда және тұрмы»  [topic «қышқ» preserved]
- ✓ [rephrase-loose] `Азот қышқылының химиялық формуласы қандай?` → «Азот қышқылының формуласы — HNO₃.»  [topic «қышқ» preserved]
- ✗ [canonical-form] `Азот қышқылының белгісі неде?` → «Қысқаша айтсам, Азот қышқылы — күшті тотықтырғыш қ»  [topic «қышқ» preserved]

### ✓ `Алгоритм дегеніміз не?`

- variant: `Алгоритм не?`
- ✗ [synonym-swap] `Синоним: Тәртіп.` → REJECTED: NEITHER topic «алго» NOR predicate «None» in paraphrase
- ✓ [rephrase-loose] `Алгоритм деген не?` → «Алгоритм — қадамдар тізбегі.»  [topic «алго» preserved]
- ✓ [canonical-form] `Алгоритм деген не?` → «Алгоритм — қадамдар тізбегі.»  [topic «алго» preserved]

### ✓ `Алгоритм дегеніміз не?`

- variant: `Алгоритм — бұл не?`
- ✗ [synonym-swap] `Нұсқа жолдары.` → REJECTED: NEITHER topic «алго» NOR predicate «None» in paraphrase
- ✗ [rephrase-loose] `Алгоритм — нə?` → «Қадамдар тізбегі»  [topic «алго» preserved]
- ✓ [canonical-form] `Алгоритм деген не?` → «Алгоритм — қадамдар тізбегі.»  [topic «алго» preserved]

### ✓ `Жүректің қызметі қандай?`

- variant: `Жүректің ағзадағы қызметі?`
- ✓ [synonym-swap] `Жүрек қан айналуы` → «Жүрек — қан айналыс мүшесі.»  [topic «жүре» preserved]
- ✓ [rephrase-loose] `Жүрек қанды айналдырады ба?` → «Жүрек қан айналымы жүйесі құрамына кіреді.»  [topic «жүре» preserved]
- ✗ [canonical-form] `Жүрек қанды ағзадағы қызметі неде?` → «Ағза жасуша иеленеді.»  [topic «жүре» preserved]

## No recovery (15)

### ✗ `Темірдің химиялық таңбасы.`

- variant: `Темірдің символын айтшы.`
- ✗ [synonym-swap] `Темірдің белгісін беретін белгіні айтшы.` → «Белгі — сипат.»
- ✗ [rephrase-loose] `Темірдің химиялық белгісі неде?` → «Белгі — сипат.»
- ✗ [canonical-form] `Темірдің химиялық белгісі неде?` → «Белгі — сипат.»

### ✗ `Темірдің химиялық таңбасы.`

- variant: `Темір қандай символмен белгіленеді?`
- ✗ [synonym-swap] `Темір не белгіленеді?` → «Темір — жансыз нәрсе.»
- ✗ [rephrase-loose] `Жеде белгісі неде?` → «Белгі — сипат.»
- ✗ [canonical-form] `Темірдің химиялық белгісі неде?` → «Белгі — сипат.»

### ✗ `Мыстың химиялық таңбасы.`

- variant: `Мыстың символы не?`
- ✗ [synonym-swap] `Жарайды, жауап:` → REJECTED: NEITHER topic «мыс» NOR predicate «симв» in paraphrase
- ✗ [rephrase-loose] `Мықтардың белгісі неде?` → REJECTED: NEITHER topic «мыс» NOR predicate «симв» in paraphrase
- ✗ [canonical-form] `Мысырдың белгісі неде?` → «Белгі — сипат.»

### ✗ `Мыстың химиялық таңбасы.`

- variant: `Мысты қалай белгілейді?`
- ✗ [synonym-swap] `Жазық қалай белгілейді?` → «Бәлкім, Қалай туралы айтасыз ба.»
- ✗ [rephrase-loose] `Мысты нәрсені қалай анықтап білуге болады?` → REJECTED: NEITHER topic «белг» NOR predicate «None» in paraphrase
- ✗ [canonical-form] `Мысты белгілеу қалай?` → «Мыс — металл шикізаты.»

### ✗ `Сынаптың химиялық таңбасы.`

- variant: `Сынаптың символын айтшы.`
- ✗ [synonym-swap] `Қиындықтың белгісін сипаттаңыз.` → REJECTED: NEITHER topic «сына» NOR predicate «симв» in paraphrase
- ✗ [rephrase-loose] `Сыныптың белгісін көрсетші.` → REJECTED: NEITHER topic «сына» NOR predicate «симв» in paraphrase
- ✗ [canonical-form] `Сынаптың символы неде?` → «Кешіріңіз, сұрағыңызды дұрыс түсінбедім. Басқаша а»

### ✗ `Сынаптың химиялық таңбасы.`

- variant: `Сынапты қалай жазады?`
- ✗ [synonym-swap] `Жазады қалай?` → REJECTED: NEITHER topic «сына» NOR predicate «None» in paraphrase
- ✗ [rephrase-loose] `Сыныпты қалай оқыту керек?` → REJECTED: NEITHER topic «сына» NOR predicate «None» in paraphrase
- ✗ [canonical-form] `Сынапты қалай жазады?` → «Кешіріңіз, сұрағыңызды дұрыс түсінбедім. Басқаша а»

### ✗ `Тұз қышқылының формуласы.`

- variant: `Тұз қышқылын қалай жазады?`
- ✗ [synonym-swap] `Тұзды қышқышты қандаймен емзеп тастайды?` → «Бәлкім, Тұзды туралы айтасыз ба.»
- ✗ [rephrase-loose] `Тұз қышқылын қандаймен емдеуге болады?` → «Қысқаша айтсам, Тұз қышқылы — өткір иісті, түссіз »
- ✗ [canonical-form] `Тұз қышқылын қандай түрде жазады?` → «Қысқаша айтсам, Тұз қышқылы — өткір иісті, түссіз »

### ✗ `Алгоритм дегеніміз не?`

- variant: `Алгоритм дегенді түсіндір.`
- ✗ [synonym-swap] `Алгоритм дегенді түсіндір - **Қадам-қадам шешім**` → «Қадамдар тізбегі»
- ✗ [rephrase-loose] `Алгоритм не екенін түсіндір не?` → «Қадамдар тізбегі»
- ✗ [canonical-form] `Алгоритм не дегенді білу керек?` → «Қадамдар тізбегі»

### ✗ `Алгоритм дегеніміз не?`

- variant: `Алгоритм деген сөздің мағынасы?`
- ✗ [synonym-swap] `Алгоритм деген сөздің мағынасы? – **Қадам-қадам тәсіл**.` → «Қадамдар тізбегі»
- ✗ [rephrase-loose] `Алгоритм деген не?` → REJECTED: NEITHER topic «сөз» NOR predicate «мағы» in paraphrase
- ✗ [canonical-form] `Алгоритм деген не?` → REJECTED: NEITHER topic «сөз» NOR predicate «мағы» in paraphrase

### ✗ `Процессор — қандай құрылғы?`

- variant: `Процессор не?`
- ✗ [synonym-swap] `Миғақ.` → REJECTED: NEITHER topic «проц» NOR predicate «None» in paraphrase
- ✗ [rephrase-loose] `Процессор қаншалықты?` → «Бәлкім, Қаншалықты туралы айтасыз ба.»
- ✗ [canonical-form] `Процессор қаншалықты?` → «Бәлкім, Қаншалықты туралы айтасыз ба.»

### ✗ `Процессор — қандай құрылғы?`

- variant: `Процессор қалай жұмыс істейді?`
- ✗ [synonym-swap] `Миғалым қаншалай жұмыс істейді?` → REJECTED: NEITHER topic «проц» NOR predicate «None» in paraphrase
- ✗ [rephrase-loose] `Микропроцессор қандай жағдайда жұмыс істейді?` → «Жағдай туралы нақты бір мысал: «Қолайлы инвестиция»
- ✗ [canonical-form] `Процессордің ішінде не болады?` → «Кешіріңіз, сұрағыңызды дұрыс түсінбедім. Басқаша а»

### ✗ `Процессор — қандай құрылғы?`

- variant: `Процессор не үшін керек?`
- ✗ [synonym-swap] `Микропроцессор қандай мақсатқа жұмшақталады?` → «Қандай ұғымның мақсатын білгіңіз келеді — нақтылап»
- ✗ [rephrase-loose] `Процессор қандай мақсатқа жұмшақталады?` → «Қандай ұғымның мақсатын білгіңіз келеді — нақтылап»
- ✗ [canonical-form] `Процессор қандай мақсатқа жұмшақталады?` → «Қандай ұғымның мақсатын білгіңіз келеді — нақтылап»

### ✗ `Жүзден отыз бесті азайт.`

- variant: `Жүзден отыз бес кеміт.`
- ✗ [synonym-swap] `Жүзден бес он бес кеміт.` → «Жүз — қазақ халқының тарихи рулық бөлінісі.»
- ✗ [rephrase-loose] `Жүзден он бес кеміт.` → «Жүз — қазақ халқының тарихи рулық бөлінісі.»
- ✗ [canonical-form] `Жүзден отыз бес кемітсе қандай санды алсамыз?` → «Жүз — қазақ халқының тарихи рулық бөлінісі.»

### ✗ `Менің жасым қаңша.`

- variant: `Мен қанша жастамын?`
- ✗ [synonym-swap] `Қанша өнім?` → REJECTED: NEITHER topic «жаст» NOR predicate «None» in paraphrase
- ✗ [rephrase-loose] `Жасым қанша?` → REJECTED: NEITHER topic «жаст» NOR predicate «None» in paraphrase
- ✗ [canonical-form] `Жасым қанша?` → REJECTED: NEITHER topic «жаст» NOR predicate «None» in paraphrase

### ✗ `Қандай партияны жақтайсың?`

- variant: `Сенің саяси көзқарасың қандай?`
- ✗ [synonym-swap] `Сіздің қоғамдық пікірім сіз несін айттыңыз?` → REJECTED: NEITHER topic «көзқ» NOR predicate «саяс» in paraphrase
- ✗ [rephrase-loose] `Сіздің саяси пікіріңіз не?` → «Кешіріңіз, сұрағыңызды дұрыс түсінбедім. Басқаша а»
- ✗ [canonical-form] `Саяси көзқарасыңды түсіндірсең қалайсың?` → «Жақсымын, рахмет.»
