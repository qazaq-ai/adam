# Phase rescore-eval v2 — deterministic vocab-membership rescorer

**Pack:** `experiments/hybrid_qlm/rescore_eval/rescore_nbest_eval.json`
**Method:** for each candidate, count tokens whose 4-char stem prefixes some `world_core` subject/object or curated high-frequency entry.  Highest count wins.
**LM in loop:** NONE.

## Summary

- Correct picks (deterministic):  **7 / 20** (35%)
- LM-4B rescore baseline:          6 / 20 (30 %)
- Random-baseline floor:           20%
- Deterministic lift over random:  +15 pp
- Deterministic lift over LM-4B:   +5 pp

### Per-topic breakdown

- `elderly`: 0/1 (0%)
- `kappacism`: 4/5 (80%)
- `lambdacism`: 1/3 (33%)
- `nasalization`: 1/3 (33%)
- `rhotacism`: 0/2 (0%)
- `sigmatism`: 1/5 (20%)
- `whisper`: 0/1 (0%)

## Wrong picks (13)

### ✗ `Ақмет.` (topic: `rhotacism`)

- clean canonical: `Рақмет.`
- picked (idx 4): `Жүйек не үшін керек?`
- scores: [0, 1, 0, 1, 3]

### ✗ `Төйттің түбійі.` (topic: `rhotacism`)

- clean canonical: `Төрттің түбірі.`
- picked (idx 4): `Қазақстанның мемлекеттік тілі.`
- scores: [2, 0, 1, 1, 3]

### ✗ `Фудың формуласы.` (topic: `sigmatism`)

- clean canonical: `Судың формуласы.`
- picked (idx 1): `Қазақтың ұлттық тағамы.`
- scores: [1, 3, 1, 1, 2]

### ✗ `Сас ұзындығы.` (topic: `sigmatism`)

- clean canonical: `Шаш ұзындығы.`
- picked (idx 3): `Қазақтың ұлттық тағамы.`
- scores: [1, 1, 1, 3, 1]

### ✗ `Зафым отыз екі.` (topic: `sigmatism`)

- clean canonical: `Жасым отыз екі.`
- picked (idx 2): `Күкіт қышқылының формулафы.`
- scores: [1, 1, 3, 1, 2]

### ✗ `Күкіт қышқылының формулафы.` (topic: `sigmatism`)

- clean canonical: `Күкірт қышқылының формуласы.`
- picked (idx 0): `Күкіт қышқылының формулафы.`
- scores: [3, 3, 1, 0, 1]

### ✗ `Бая деген не?` (topic: `lambdacism`)

- clean canonical: `Бала деген не?`
- picked (idx 0): `Байқаш көйі туралы.`
- scores: [2, 2, 1, 1, 2]

### ✗ `Тійдің қызметі.` (topic: `lambdacism`)

- clean canonical: `Тілдің қызметі.`
- picked (idx 0): `Қазақтың ұйттық тағамы.`
- scores: [2, 2, 1, 2, 1]

### ✗ `Хышхыл деген не?` (topic: `kappacism`)

- clean canonical: `Қышқыл деген не?`
- picked (idx 1): `Тальцийдің химиялық таңбасы.`
- scores: [1, 2, 0, 2, 1]

### ✗ `Атм дегеніміз не?` (topic: `nasalization`)

- clean canonical: `Атом дегеніміз не?`
- picked (idx 3): `Жүректң қызметі.`
- scores: [0, 1, 0, 2, 2]

### ✗ `Алгрітм.` (topic: `nasalization`)

- clean canonical: `Алгоритм.`
- picked (idx 2): `Қазақстнда қандай тіл мемлекетк?`
- scores: [1, 0, 2, 0, 2]

### ✗ `Казхстан Республикасы.` (topic: `elderly`)

- clean canonical: `Қазақстан Республикасы.`
- picked (idx 0): `Кмістің химиялық таңбасы.`
- scores: [2, 1, 2, 1, 2]

### ✗ `Менім атым - дәулет.` (topic: `whisper`)

- clean canonical: `Менің атым — Дәулет.`
- picked (idx 0): `Менің есімім даулет.`
- scores: [2, 1, 0, 1, 1]
