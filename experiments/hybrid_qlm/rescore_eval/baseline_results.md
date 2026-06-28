# Phase rescore-eval — baseline (no fine-tune)

**Pack:** `experiments/hybrid_qlm/rescore_eval/rescore_nbest_eval.json`
**Items:** 20 Whisper-style N-best probes (5 candidates each, exactly 1 clean canonical, 1 defective input, 3 same-topic distractors).
**Method:** prompt LM with numbered candidate list, parse first digit of response.

## Summary

- Correct picks:           **6 / 20** (30%)
- Random-baseline floor:   20% (uniform over 5 candidates)
- LM lift over random:     +10 pp

### Per-topic breakdown

- `elderly`: 1/1 (100%)
- `kappacism`: 2/5 (40%)
- `lambdacism`: 0/3 (0%)
- `nasalization`: 0/3 (0%)
- `rhotacism`: 1/2 (50%)
- `sigmatism`: 2/5 (40%)
- `whisper`: 0/1 (0%)

## Wrong picks (14)

### ✗ `Төйттің түбійі.` (topic: `rhotacism`)

- clean canonical: `Төрттің түбірі.`
- LM picked:       `Қазақстанның мемлекеттік тілі.` (idx 4)
- raw LM response: `'Ең дұрыс нөмір - 4.\nҚазақстанның мемлекеттік ті'`

Candidates:
  - [0] `Төрттің түбірі.` ✓ CLEAN
  - [1] `Ақмет.`        
  - [2] `Теінің қызметі.`        
  - [3] `Төйттің түбійі.`        
  - [4] `Қазақстанның мемлекеттік тілі.` ← picked

### ✗ `Фудың формуласы.` (topic: `sigmatism`)

- clean canonical: `Судың формуласы.`
- LM picked:       `Қазақтың ұлттық тағамы.` (idx 1)
- raw LM response: `'Ең дұрыс нөмір: **1. Қазақтың ұлттық таға'`

Candidates:
  - [0] `Судың формуласы.` ✓ CLEAN
  - [1] `Қазақтың ұлттық тағамы.` ← picked
  - [2] `Зафым отыз екі.`        
  - [3] `Фудың формуласы.`        
  - [4] `Фегізді жетіге көбейт.`        

### ✗ `Сас ұзындығы.` (topic: `sigmatism`)

- clean canonical: `Шаш ұзындығы.`
- LM picked:       `Қазақтың ұлттық тағамы.` (idx 3)
- raw LM response: `'Ең дұрыс нөмір - 3.\n**3. Қазақтың ұлт'`

Candidates:
  - [0] `Шаш ұзындығы.` ✓ CLEAN
  - [1] `Фудың формуласы.`        
  - [2] `Сас ұзындығы.`        
  - [3] `Қазақтың ұлттық тағамы.` ← picked
  - [4] `Қанның қызметі қандай?`        

### ✗ `Бефті жетіге қоф.` (topic: `sigmatism`)

- clean canonical: `Бесті жетіге қос.`
- LM picked:       `Қанның қызметі қандай?` (idx 3)
- raw LM response: `'Ең дұрыс нөмір: 3. Қанның қызметі қандай?'`

Candidates:
  - [0] `Фәлем`        
  - [1] `Бесті жетіге қос.` ✓ CLEAN
  - [2] `Зафым отыз екі.`        
  - [3] `Қанның қызметі қандай?` ← picked
  - [4] `Бефті жетіге қоф.`        

### ✗ `Қазақстанда қандай тій мемйекеттік?` (topic: `lambdacism`)

- clean canonical: `Қазақстанда қандай тіл мемлекеттік?`
- LM picked:       `Тійдің қызметі.` (idx 3)
- raw LM response: `'Ең дұрыс нөмір: **3. Тійдің қызметі.**\nСө'`

Candidates:
  - [0] `Қазақстанда қандай тіл мемлекеттік?` ✓ CLEAN
  - [1] `Бая деген не?`        
  - [2] `Эектр тогы деген не?`        
  - [3] `Тійдің қызметі.` ← picked
  - [4] `Қазақстанда қандай тій мемйекеттік?`        

### ✗ `Бая деген не?` (topic: `lambdacism`)

- clean canonical: `Бала деген не?`
- LM picked:       `Қазақтың ұйттық тағамы.` (idx 1)
- raw LM response: `'Ең дұрыс нөмір: **1. Қазақтың ұйттық таға'`

Candidates:
  - [0] `Байқаш көйі туралы.`        
  - [1] `Қазақтың ұйттық тағамы.` ← picked
  - [2] `Айгоритм деген не?`        
  - [3] `Бая деген не?`        
  - [4] `Бала деген не?` ✓ CLEAN

### ✗ `Тійдің қызметі.` (topic: `lambdacism`)

- clean canonical: `Тілдің қызметі.`
- LM picked:       `Қазақтың ұйттық тағамы.` (idx 0)
- raw LM response: `'Ең дұрыс нөмір - 0.\n**0. Қазақтың ұйт'`

Candidates:
  - [0] `Қазақтың ұйттық тағамы.` ← picked
  - [1] `Тілдің қызметі.` ✓ CLEAN
  - [2] `Тійдің қызметі.`        
  - [3] `Байқаш көйі туралы.`        
  - [4] `Айгоритм деген не?`        

### ✗ `Төмірқышқыл газының формуласы.` (topic: `kappacism`)

- clean canonical: `Көмірқышқыл газының формуласы.`
- LM picked:       `Төмірқышқыл газының формуласы.` (idx 0)
- raw LM response: `'Ең дұрыс нөмір: 0 және 4.\nМұндағы сұрақ'`

Candidates:
  - [0] `Төмірқышқыл газының формуласы.` ← picked
  - [1] `Түміс деген зат хандай?`        
  - [2] `Хазах деген не?`        
  - [3] `Тітап деген не?`        
  - [4] `Көмірқышқыл газының формуласы.` ✓ CLEAN

### ✗ `Хазах деген не?` (topic: `kappacism`)

- clean canonical: `Қазақ деген не?`
- LM picked:       `Тітап деген не?` (idx 4)
- raw LM response: `'4. Хазах деген не?'`

Candidates:
  - [0] `Қазақ деген не?` ✓ CLEAN
  - [1] `Ханша уаххыт хазір?`        
  - [2] `Хышхыл деген не?`        
  - [3] `Хазах деген не?`        
  - [4] `Тітап деген не?` ← picked

### ✗ `Тітап деген не?` (topic: `kappacism`)

- clean canonical: `Кітап деген не?`
- LM picked:       `Хазахстанның мемлекеттік тілі.` (idx 4)
- raw LM response: `'4. Хазахстанның мемлекеттік тілі.'`

Candidates:
  - [0] `Хазах деген не?`        
  - [1] `Тітап деген не?`        
  - [2] `Бір байтта неше біт бал?`        
  - [3] `Кітап деген не?` ✓ CLEAN
  - [4] `Хазахстанның мемлекеттік тілі.` ← picked

### ✗ `Казхстан туралы.` (topic: `nasalization`)

- clean canonical: `Қазақстан туралы.`
- LM picked:       `Қазақстнда қандай тіл мемлекетк?` (idx 3)
- raw LM response: `'Ең дұрыс нөмір - 3.\n**3. Қазақстанның мем'`

Candidates:
  - [0] `Қазақстан туралы.` ✓ CLEAN
  - [1] `Қзл түс.`        
  - [2] `Казхстан туралы.`        
  - [3] `Қазақстнда қандай тіл мемлекетк?` ← picked
  - [4] `Жүректң қызметі.`        

### ✗ `Атм дегеніміз не?` (topic: `nasalization`)

- clean canonical: `Атом дегеніміз не?`
- LM picked:       `Атм дегеніміз не?` (idx 1)
- raw LM response: `'Ең дұрыс нөмір: 1. Атм дегеніміз не?'`

Candidates:
  - [0] `Қзл түс.`        
  - [1] `Атм дегеніміз не?` ← picked
  - [2] `Сәлмесіз бе.`        
  - [3] `Жүректң қызметі.`        
  - [4] `Атом дегеніміз не?` ✓ CLEAN

### ✗ `Алгрітм.` (topic: `nasalization`)

- clean canonical: `Алгоритм.`
- LM picked:       `Қазақстнда қандай тіл мемлекетк?` (idx 2)
- raw LM response: `'Ең дұрыс нөмір: 2. Қазақстанда қандай тіл'`

Candidates:
  - [0] `Алгоритм.` ✓ CLEAN
  - [1] `Алгрітм.`        
  - [2] `Қазақстнда қандай тіл мемлекетк?` ← picked
  - [3] `Қзл түс.`        
  - [4] `Тмірдің химиялық таңбасы.`        

### ✗ `Менім атым - дәулет.` (topic: `whisper`)

- clean canonical: `Менің атым — Дәулет.`
- LM picked:       `Менің есімім даулет.` (idx 0)
- raw LM response: `'Ең дұрыс нөмір - **0. Менің есімім даулет.**'`

Candidates:
  - [0] `Менің есімім даулет.` ← picked
  - [1] `Менім атым - дәулет.`        
  - [2] `Сәлімі түсбе.`        
  - [3] `Менің атым — Дәулет.` ✓ CLEAN
  - [4] `Алды мен таныс айық.`        
