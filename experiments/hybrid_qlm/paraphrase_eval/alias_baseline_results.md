# Phase v5-prep — DETERMINISTIC alias-table baseline

**Pack:** `experiments/hybrid_qlm/paraphrase_eval/paraphrase_coverage_eval.json`
**Method:** deterministic cascade → on miss, apply curated Kazakh synonym rules → re-run cascade
**LM in loop:** NONE.

## Summary

- Deterministic baseline:   **81 / 100** (81%)
- Alias-rewrite path:       **89 / 100** (89%)
- Recoveries (no LM):       **8 / 19** misses (42% recovery rate)
- Lift (no LM):             **+8 pp**

### Comparison vs hybrid variants

| Variant                          | Recoveries | Recovery | Lift  | LM calls |
| -------------------------------- | ---------- | -------- | ----- | -------- |
| deterministic alias (no LM)      | 8 / 19     | 42 %    | +8 pp | 0        |
| v1 single-shot LM (no verifier)  | 4 / 19     | 21 %     | +4 pp | 19       |
| v2 N-best LM (no verifier)       | 9 / 19     | 47 %     | +9 pp | 57       |
| v3 N-best + naive verifier       | 5 / 19     | 26 %     | +5 pp | 57       |
| v4 N-best + smart verifier       | 4 / 19     | 21 %     | +4 pp | 57       |

### Rules that pulled their weight

- `(?i)\bсимвол(ын|ы|ың|ыңды)?\b`: 2 recoveries
- `(?i)\bқалай\s+жазады\b`: 2 recoveries
- `(?i)\bсимвол(ын|ы|ың|ыңды)?\b + (?i)\b([А-ЯЁа-яёҚқҒғҢңӨөҰұҮүҺһІі]{5,})\s+не\?`: 1 recoveries
- `(?i)\bқалай\s+белгілейді\b`: 1 recoveries
- `(?i)\bбелгі(сін|сі|сің|сіңіз|ңіз|сіңізді)?\b`: 1 recoveries
- `(?i)\b([А-ЯЁа-яёҚқҒғҢңӨөҰұҮүҺһІі]{5,})\s+не\?`: 1 recoveries

## Recoveries (8)

### ✓ `Темірдің химиялық таңбасы.`

- variant:    `Темірдің символын айтшы.`
- rewritten:  `Темірдің таңбасы айтшы.`
- rules:      `(?i)\bсимвол(ын|ы|ың|ыңды)?\b`
- cascade:    «Темірдің формуласы — Fe.»

### ✓ `Мыстың химиялық таңбасы.`

- variant:    `Мыстың символы не?`
- rewritten:  `Мыстың таңбасы дегеніміз не?`
- rules:      `(?i)\bсимвол(ын|ы|ың|ыңды)?\b + (?i)\b([А-ЯЁа-яёҚқҒғҢңӨөҰұҮүҺһІі]{5,})\s+не\?`
- cascade:    «Мыстың формуласы — Cu.»

### ✓ `Мыстың химиялық таңбасы.`

- variant:    `Мысты қалай белгілейді?`
- rewritten:  `Мысты таңбасы қандай?`
- rules:      `(?i)\bқалай\s+белгілейді\b`
- cascade:    «Мыстың формуласы — Cu.»

### ✓ `Сынаптың химиялық таңбасы.`

- variant:    `Сынаптың символын айтшы.`
- rewritten:  `Сынаптың таңбасы айтшы.`
- rules:      `(?i)\bсимвол(ын|ы|ың|ыңды)?\b`
- cascade:    «Сынаптың формуласы — Hg.»

### ✓ `Сынаптың химиялық таңбасы.`

- variant:    `Сынапты қалай жазады?`
- rewritten:  `Сынапты формуласы қандай?`
- rules:      `(?i)\bқалай\s+жазады\b`
- cascade:    «Сынаптың формуласы — Hg.»

### ✓ `Азот қышқылының формуласы.`

- variant:    `Азот қышқылының белгісі қандай?`
- rewritten:  `Азот қышқылының таңбасы қандай?`
- rules:      `(?i)\bбелгі(сін|сі|сің|сіңіз|ңіз|сіңізді)?\b`
- cascade:    «Азот қышқылының формуласы — HNO₃.»

### ✓ `Тұз қышқылының формуласы.`

- variant:    `Тұз қышқылын қалай жазады?`
- rewritten:  `Тұз қышқылын формуласы қандай?`
- rules:      `(?i)\bқалай\s+жазады\b`
- cascade:    «Тұз қышқылының формуласы — HCl.»

### ✓ `Процессор — қандай құрылғы?`

- variant:    `Процессор не?`
- rewritten:  `Процессор дегеніміз не?`
- rules:      `(?i)\b([А-ЯЁа-яёҚқҒғҢңӨөҰұҮүҺһІі]{5,})\s+не\?`
- cascade:    «Процессор компьютер құрамына кіреді.»

## No recovery (11)

### ✗ `Темірдің химиялық таңбасы.`

- variant:    `Темір қандай символмен белгіленеді?`
- (no alias rule matched this variant)

### ✗ `Алгоритм дегеніміз не?`

- variant:    `Алгоритм не?`
- rewritten:  `Алгоритм дегеніміз не?`
- rules:      `(?i)\b([А-ЯЁа-яёҚқҒғҢңӨөҰұҮүҺһІі]{5,})\s+не\?`
- cascade:    «Қадамдар тізбегі»

### ✗ `Алгоритм дегеніміз не?`

- variant:    `Алгоритм дегенді түсіндір.`
- rewritten:  `Алгоритм дегеніміз не.`
- rules:      `(?i)\bдегенді\s+түсіндір[іңіздерше]*\b`
- cascade:    «Қадамдар тізбегі»

### ✗ `Алгоритм дегеніміз не?`

- variant:    `Алгоритм — бұл не?`
- rewritten:  `Алгоритм — дегеніміз дегеніміз не?`
- rules:      `(?i)\bбұл\s+не\b + (?i)\b([А-ЯЁа-яёҚқҒғҢңӨөҰұҮүҺһІі]{5,})\s+не\?`
- cascade:    «Қадамдар тізбегі»

### ✗ `Алгоритм дегеніміз не?`

- variant:    `Алгоритм деген сөздің мағынасы?`
- rewritten:  `Алгоритм дегеніміз дегеніміз не?`
- rules:      `(?i)\bдеген\s+сөздің\s+мағынасы\b + (?i)\b([А-ЯЁа-яёҚқҒғҢңӨөҰұҮүҺһІі]{5,})\s+не\?`
- cascade:    «Қадамдар тізбегі»

### ✗ `Процессор — қандай құрылғы?`

- variant:    `Процессор қалай жұмыс істейді?`
- (no alias rule matched this variant)

### ✗ `Процессор — қандай құрылғы?`

- variant:    `Процессор не үшін керек?`
- (no alias rule matched this variant)

### ✗ `Жүректің қызметі қандай?`

- variant:    `Жүректің ағзадағы қызметі?`
- (no alias rule matched this variant)

### ✗ `Жүзден отыз бесті азайт.`

- variant:    `Жүзден отыз бес кеміт.`
- (no alias rule matched this variant)

### ✗ `Менің жасым қаңша.`

- variant:    `Мен қанша жастамын?`
- (no alias rule matched this variant)

### ✗ `Қандай партияны жақтайсың?`

- variant:    `Сенің саяси көзқарасың қандай?`
- (no alias rule matched this variant)
