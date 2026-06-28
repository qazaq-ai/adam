# Phase hybrid-wiring v3 — N-best + topic-preservation verifier

**Pack:** `experiments/hybrid_qlm/paraphrase_eval/paraphrase_coverage_eval.json`
**Path:** deterministic → on miss, 3 LM paraphrase candidates → topic-noun verifier gate → first-hit wins

## Summary

- Deterministic baseline:       **81 / 100** (81%)
- Hybrid N-best + verifier:     **86 / 100** (86%)
- Recoveries:                   **5 / 19** misses (26% recovery rate)
- Verifier rejections:          **17** candidates blocked at the gate
- Lift:                         **+5 pp**

### Verifier impact

- Of 17 paraphrases that lost the topic noun, the verifier blocked **17/17** (100% by construction).
- The remaining `(3 × 19) - 17 = 40` candidates reached the cascade.

### Per-prompt wins

- `synonym-swap`: 1 recoveries
- `rephrase-loose`: 3 recoveries
- `canonical-form`: 1 recoveries

## Recoveries (5)

### ✓ `Азот қышқылының формуласы.`

- variant: `Азот қышқылының белгісі қандай?`
- ✗ [synonym-swap] `Сірке тотысқысының белгісі қандай?` → REJECTED: topic-noun «қышқылының» (stem «қышқ») LOST in paraphrase
- ✓ [rephrase-loose] `Азот қышқылының химиялық формуласы қандай?` → «Азот қышқылының формуласы — HNO₃.»
- ✗ [canonical-form] `Азот қышқылының белгісі неде?` → «Қысқаша айтсам, Азот қышқылы — күшті тотықтырғыш қ»

### ✓ `Алгоритм дегеніміз не?`

- variant: `Алгоритм не?`
- ✗ [synonym-swap] `Синоним: Тәртіп.` → REJECTED: topic-noun «Алгоритм» (stem «алго») LOST in paraphrase
- ✓ [rephrase-loose] `Алгоритм деген не?` → «Алгоритм — қадамдар тізбегі.»
- ✓ [canonical-form] `Алгоритм деген не?` → «Алгоритм — қадамдар тізбегі.»

### ✓ `Алгоритм дегеніміз не?`

- variant: `Алгоритм — бұл не?`
- ✗ [synonym-swap] `Нұсқа жолдары.` → REJECTED: topic-noun «Алгоритм» (stem «алго») LOST in paraphrase
- ✗ [rephrase-loose] `Алгоритм — нə?` → «Қадамдар тізбегі»
- ✓ [canonical-form] `Алгоритм деген не?` → «Алгоритм — қадамдар тізбегі.»

### ✓ `Алгоритм дегеніміз не?`

- variant: `Алгоритм деген сөздің мағынасы?`
- ✗ [synonym-swap] `Алгоритм деген сөздің мағынасы? – **Қадам-қадам тәсіл**.` → «Қадамдар тізбегі»
- ✓ [rephrase-loose] `Алгоритм деген не?` → «Алгоритм — қадамдар тізбегі.»
- ✓ [canonical-form] `Алгоритм деген не?` → «Алгоритм — қадамдар тізбегі.»

### ✓ `Жүректің қызметі қандай?`

- variant: `Жүректің ағзадағы қызметі?`
- ✓ [synonym-swap] `Жүрек қан айналуы.` → «Жүрек — қан айналыс мүшесі.»
- ✓ [rephrase-loose] `Жүрек қан айналуын жүзеге асырады.` → «Жүрек — қан айналыс мүшесі.»
- ✗ [canonical-form] `Жүрек қанды ағзадағы қызметі неде?` → «Ағза жасуша иеленеді.»

## No recovery (14)

### ✗ `Темірдің химиялық таңбасы.`

- variant: `Темірдің символын айтшы.`
- ✗ [synonym-swap] `Темірдің белгісін беретін белгісін айтшы.` → «Белгі — сипат.»
- ✗ [rephrase-loose] `Темірдің химиялық белгісі неде?` → «Белгі — сипат.»
- ✗ [canonical-form] `Темірдің химиялық белгісі неде?` → «Белгі — сипат.»

### ✗ `Темірдің химиялық таңбасы.`

- variant: `Темір қандай символмен белгіленеді?`
- ✗ [synonym-swap] `Темір не белгіленеді?` → «Темір — жансыз нәрсе.»
- ✗ [rephrase-loose] `Жеңілталды ілмек қандай белгімен белгіленеді?` → «Ілмек туралы нақты бір мысал: «Айтқызған — жаманды»
- ✗ [canonical-form] `Темірдің химиялық белгісі неде?` → «Белгі — сипат.»

### ✗ `Мыстың химиялық таңбасы.`

- variant: `Мыстың символы не?`
- ✗ [synonym-swap] `Жарайды, жауап:` → REJECTED: topic-noun «символы» (stem «симв») LOST in paraphrase
- ✗ [rephrase-loose] `Мыңның белгісі неде?` → REJECTED: topic-noun «символы» (stem «симв») LOST in paraphrase
- ✗ [canonical-form] `Мысырдің белгісі неде?` → REJECTED: topic-noun «символы» (stem «симв») LOST in paraphrase

### ✗ `Мыстың химиялық таңбасы.`

- variant: `Мысты қалай белгілейді?`
- ✗ [synonym-swap] `Жазық қалай белгілейді?` → «Бәлкім, Қалай туралы айтасыз ба.»
- ✗ [rephrase-loose] `Мысты қандай белгімен атайды?` → «Мыс — металл шикізаты.»
- ✗ [canonical-form] `Мысты белгілеу қалай?` → «Мыс — металл шикізаты.»

### ✗ `Сынаптың химиялық таңбасы.`

- variant: `Сынаптың символын айтшы.`
- ✗ [synonym-swap] `Қиындықтың белгісін сипаттаңыз.` → REJECTED: topic-noun «Сынаптың» (stem «сына») LOST in paraphrase
- ✗ [rephrase-loose] `Сыныптың белгісін көрсетші.` → REJECTED: topic-noun «Сынаптың» (stem «сына») LOST in paraphrase
- ✗ [canonical-form] `Сынаптың символы неде?` → «Кешіріңіз, сұрағыңызды дұрыс түсінбедім. Басқаша а»

### ✗ `Сынаптың химиялық таңбасы.`

- variant: `Сынапты қалай жазады?`
- ✗ [synonym-swap] `Жазады қалай?` → REJECTED: topic-noun «Сынапты» (stem «сына») LOST in paraphrase
- ✗ [rephrase-loose] `Сыныпты қалай оқыту керек?` → REJECTED: topic-noun «Сынапты» (stem «сына») LOST in paraphrase
- ✗ [canonical-form] `Сынапты қалай жазады?` → «Кешіріңіз, сұрағыңызды дұрыс түсінбедім. Басқаша а»

### ✗ `Тұз қышқылының формуласы.`

- variant: `Тұз қышқылын қалай жазады?`
- ✗ [synonym-swap] `Тұзды қышқышты қандаймен емзеп тастайды?` → «Бәлкім, Тұзды туралы айтасыз ба.»
- ✗ [rephrase-loose] `Тұз қышқылын қаншамен емзеп, жеңуге болады?` → «Қысқаша айтсам, Тұз қышқылы — өткір иісті, түссіз »
- ✗ [canonical-form] `Тұз қышқылын қандай түрде жазады?` → «Қысқаша айтсам, Тұз қышқылы — өткір иісті, түссіз »

### ✗ `Алгоритм дегеніміз не?`

- variant: `Алгоритм дегенді түсіндір.`
- ✗ [synonym-swap] `Алгоритм дегенді түсіндір - **Қадам-қадам шешім**` → «Қадамдар тізбегі»
- ✗ [rephrase-loose] `Алгоритм не екенін түсіндір не?` → «Қадамдар тізбегі»
- ✗ [canonical-form] `Алгоритм не дегенді білу керек?` → «Қадамдар тізбегі»

### ✗ `Процессор — қандай құрылғы?`

- variant: `Процессор не?`
- ✗ [synonym-swap] `Миғақ.` → REJECTED: topic-noun «Процессор» (stem «проц») LOST in paraphrase
- ✗ [rephrase-loose] `Процессор қаншалықты?` → «Бәлкім, Қаншалықты туралы айтасыз ба.»
- ✗ [canonical-form] `Процессор қаншалықты күшті?` → «Бәлкім, Қаншалықты туралы айтасыз ба.»

### ✗ `Процессор — қандай құрылғы?`

- variant: `Процессор қалай жұмыс істейді?`
- ✗ [synonym-swap] `Миғалым қаншалай жұмыс істейді?` → REJECTED: topic-noun «Процессор» (stem «проц») LOST in paraphrase
- ✗ [rephrase-loose] `Микропроцессор қандай жолмен жұмыс істейді?` → «Қысқаша айтсам, Жұмыс — күштің әсер ету бағытында »
- ✗ [canonical-form] `Процессордің ішінде не болады?` → «Кешіріңіз, сұрағыңызды дұрыс түсінбедім. Басқаша а»

### ✗ `Процессор — қандай құрылғы?`

- variant: `Процессор не үшін керек?`
- ✗ [synonym-swap] `Микропроцессор қандай мақсатқа жұмшақталады?` → «Қандай ұғымның мақсатын білгіңіз келеді — нақтылап»
- ✗ [rephrase-loose] `Процессор қандай мақсатқа жұмшалады?` → «Қандай ұғымның мақсатын білгіңіз келеді — нақтылап»
- ✗ [canonical-form] `Процессор қандай мақсатқа жұмшақталады?` → «Қандай ұғымның мақсатын білгіңіз келеді — нақтылап»

### ✗ `Жүзден отыз бесті азайт.`

- variant: `Жүзден отыз бес кеміт.`
- ✗ [synonym-swap] `Жүзден жиырма бес-кеңіп.` → «Жүз — қазақ халқының тарихи рулық бөлінісі.»
- ✗ [rephrase-loose] `Жүзден он бес кеміт.` → «Жүз — қазақ халқының тарихи рулық бөлінісі.»
- ✗ [canonical-form] `Жүзден отыз бес кемітсе қандай санды алсамыз?` → «Жүз — қазақ халқының тарихи рулық бөлінісі.»

### ✗ `Менің жасым қаңша.`

- variant: `Мен қанша жастамын?`
- ✗ [synonym-swap] `Қанша өнім?` → REJECTED: topic-noun «жастамын» (stem «жаст») LOST in paraphrase
- ✗ [rephrase-loose] `Жасым қанша?` → REJECTED: topic-noun «жастамын» (stem «жаст») LOST in paraphrase
- ✗ [canonical-form] `Жасым қанша?` → REJECTED: topic-noun «жастамын» (stem «жаст») LOST in paraphrase

### ✗ `Қандай партияны жақтайсың?`

- variant: `Сенің саяси көзқарасың қандай?`
- ✗ [synonym-swap] `Сіздің саяси пікірім сіз қалай көретініңіз?` → REJECTED: topic-noun «көзқарасың» (stem «көзқ») LOST in paraphrase
- ✗ [rephrase-loose] `Сіздің саяси пікіріңіз не?` → REJECTED: topic-noun «көзқарасың» (stem «көзқ») LOST in paraphrase
- ✗ [canonical-form] `Саяси көзқарасыңды түсіндірсең қалайсың?` → «Жақсымын, рахмет.»
