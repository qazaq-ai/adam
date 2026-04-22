# Lexicon gap candidates — v3.4.0 mining pass

**Scan**: 9 committed source packs, 411031 samples, 3921698 tokens (≥ 3 chars alphabetic). **Candidates**: top 200 most-frequent surfaces that no current Lexicon root prefixes.

## How to review

Each candidate lists the observed surface form, its frequency, 3 sample sentences, and **auto-tagged features** (vowel harmony + final-sound class). Your job:

1. Decide the canonical **root form** — the surface may be inflected; the root is the bare nominative / infinitive.
2. Confirm the **POS**: `noun`, `verb`, `adjective`, `adverb`, `pronoun`, `numeral`, `conjunction`, `particle`, `postposition`.
3. Verify or correct the auto-tags.
4. **Reject** loanwords, proper nouns, OCR artefacts, and anything not a real Kazakh root.
5. Update the **Tally** section at the bottom with approve / reject counts.

Approved roots get added via a Lexicon PR against `data/tokenizer/segmentation_roots.json`. Re-run `cargo run --release -p adam-corpus --bin morpheme_coverage` after the PR to measure the coverage delta — per memory `project_morpheme_coverage_baseline`, every Lexicon PR must do this.

---

### Candidate #1 — `деп` (freq 11604)

- Vowel harmony (auto): **front**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00087`) «"Мысық?" - деп сұрады қарт адам.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00223`) «Мен тіпті сені ашуланар деп ойлаған жоқ едім.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00246`) «Бүркітімді құсбегіне сынатайын деп едім.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #2 — `оның` (freq 11098)

- Vowel harmony (auto): **back**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00040`) «Ол оның қолынан ұстады.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00229`) «Оның пікіріне мен де қосыламын.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00287`) «Жаман хабарлардан оның беті бозарыңқы тартқан.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #3 — `осы` (freq 8486)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00240`) «Ол осы үйдің тұтқасы еді.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00312`) «Енді осы арадан қала алыс емес.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00315`) «Тараптар осы шарт бойынша өз міндеттемелерін толық орындағанша шарт қолданыста болады.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #4 — `деген` (freq 6250)

- Vowel harmony (auto): **front**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00992`) «Шоколадты жасауды бірінші ацтектер үйренді деген пікір қалыптасқан.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00993`) «Олар шоколадты «құдайлар асы» деген.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00994`) «Еуропаға алғаш жеткізілген Испан конкистадорлары бұл тәттіге «қара алтын» деген атау берді және оны дене күшін нығайту үшін пайдаланды.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #5 — `сол` (freq 4939)

- Vowel harmony (auto): **back**
- Final sound (auto): **liquid**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00402`) «Сол түнде, Әсем бес ыдысты сындырды.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00506`) «Қазірде сол қоралы жаннан қалған ешкім жоқ.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00721`) «Ол сол араға қайтқыштай берді.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #6 — `пен` (freq 4521)

- Vowel harmony (auto): **front**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00007`) «Интернет пен телефонды бір жинақта сатыл ал!»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00113`) «Біздің халқымызға ең керегі - бірлік, ұлтаралық татулық пен саяси тұрақтылық.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00554`) «Біздің әскери қызметшілер жоғары кәсіби даярлық, үйлесімділік пен жауынгерлік дағды үлгісін керсетуде.(Н.Назарбаев)»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #7 — `бас` (freq 4282)

- Vowel harmony (auto): **back**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00195`) «Тамшы тас жарады,тіл бас жарады.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00295`) «Бас көтерген екі баласы бар.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00787`) «Шаш ал десе, бас алу.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #8 — `байланысты` (freq 4173)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00066`) «Бұл контекстіге байланысты.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00365`) «Бұл алаңдаушылық Солтүстік Кореяның «Ынха-3» зымыран тасымалдағышын ұшыруға байланысты болып отыр.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00530`) «Емдеудің ұзақтығы аурудың ауырлығына байланысты.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #9 — `облысы` (freq 4149)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00547`) «Оңтүстік Қазақстан облысы аграрлы аймақ болып есептеледі.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01400`) «Татар ғалымдары өздерінің білетін әдістермен Маңғыстау облысы мұнайшыларымен бөлісуге әзір.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01455`) ««Мектепке жол» іс-шарасы кезінде үй-үйді аралау барысында Екібастұз қаласы ІІД қызметкерлері үйлердің бірінен ішімдікке мас болған анасының жанынан 2010 жылы туған кішкентай баланы тауып алды, деп хаб …»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #10 — `оны` (freq 3943)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00064`) «Мен оны атып тастаймын.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00144`) «Сіз оны білесіз бе?»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00220`) «Оны көрмеймін.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #11 — `алып` (freq 3889)

- Vowel harmony (auto): **back**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00741`) «Балаларды ләйлек алып келеді.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00825`) «Шырақ алып іздесең де таппайсың.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01080`) «Біз әрдайым өз жұмысымызда өзіміздің клиенттеріміздің пікірлері мен талап-тілектерін нысанаға алып, олардың үміттерін толыққанды қанағаттандыруға тырысамыз.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #12 — `республикасының` (freq 3806)

- Vowel harmony (auto): **mixed**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00314`) «Тараптардың шарт бойынша реттелмеген дауларын Қазақстан Республикасының соттары Қазақстан Республикасының заңнамасына сәйкес қарайды.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00314`) «Тараптардың шарт бойынша реттелмеген дауларын Қазақстан Республикасының соттары Қазақстан Республикасының заңнамасына сәйкес қарайды.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00316`) «Шарт тараптардың өзара келісуі бойынша жазбаша түрде, Қазақстан Республикасының заңдарында тікелей айқындалмаған және енгізілген толықтыру Қазақстан Республикасының заңдарына қайшы келмейтін ережелер  …»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #13 — `орта` (freq 3673)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00546`) «Мен былтыр орта мектепті бітірдім.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_02470`) «Меніңше, орта мектептің екінші сыныбы ең көңілді болды.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0000085`) «Қазақстан қоңыржай белдеудің орта және оңтүстік ендіктерінде орналасқан.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #14 — `сондайақ` (freq 3224)

- Vowel harmony (auto): **back**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01296`) «Өлім жазасы адамдардың қаза болуымен байланысты террористік қылмыстар жасағаны үшін, сондай-ақ соғыс уақытында ерекше ауыр қылмыстар жасағаны үшін ең ауыр жаза ретінде заңмен белгіленеді, ондай жазаға …»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01310`) ««Астана-Алматы» телекөпірі ұйымдастырылды, сондай-ақ тікелей желіге Қарағанды қаласы қосылды.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01420`) «159 мүше-мемлекет делегацияларының жыл сайынғы басты кездесуі ауқымында МАГАТЭ-нің атом энергиясын бейбіт мақсатта пайдалану, қауіпсіздікті қамтамасыз ету саласындағы қызметін қарады, ядролық қаруды т …»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #15 — `орналасқан` (freq 3162)

- Vowel harmony (auto): **back**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00560`) «Венгрия – Еуропаның орталығында орналасқан мемлекет.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01349`) «Бүркіттер Алматы қаласынан 200 шақырым жерде орналасқан Манул аңшылық шаруашылығының аумағына жіберілді.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01427`) «Облыстық әкімдіктен хабарланғандай, мерекеге көршілес Астраханнан, Магнитогорскіден және біздің елге тиесілі «Қазақстан геологы» шипажайы орналасқан Железноводскіден делегациялар келеді деп күтілуде.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #16 — `алады` (freq 3121)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00500`) «Түйе сусыз, жемсіз ұзақ уақыт жүре алады.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00785`) «Балапан ұяда не көрсе, ұшқанда соны алады.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00842`) «Ол естігенді қағып алады.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #17 — `пайда` (freq 3039)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00537`) «Есекжем белгілері пайда болған кезде дәрігерге дереу қаралғаны жөн.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00998`) «Тек 20-шы ғасырдың басында шоколадты өндіретін өндіріс пайда болғаннан бастап, ақсүйектер әулетіне жатпайтын адамдар да шоколадты рақаттана пайдаланды.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01027`) «Сәби өлімінің негізгі себептері перинаталдық кезеңде пайда болатын жағдайлармен байланысты.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #18 — `республикасы` (freq 2924)

- Vowel harmony (auto): **mixed**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00327`) «Қазақстан Республикасы - көп ұлтты мемлекет.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00700`) «Делегация құрамына Корея Республикасы елшілігінің өкілдері кірді.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01488`) «Осыған дейін хабарланғандай, Қытай Халық Республикасы Төрағасының ресми сапары барысында екіжақты ынтымақтастық туралы бірқатар маңызды құжаттарға қол қойылды.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #19 — `басты` (freq 2912)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00293`) «Ол аяғын басты.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00557`) «Ресей тағы да басты компьютерлік қарақшылардың қара тізіміне енгізілді, - деп хабарлады Би-Би-Си.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01186`) «Бүгін дүние жүзі католиктері христиан дініндегі басты мереке - Пасханы тойлайды.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #20 — `алғашқы` (freq 2806)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00991`) «1995 жылы алғашқы рет француздар шоколад күнін ойлап тапты.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01132`) «Бұл сала бойынша еліміздің көптеген көрсеткіштермен жоғары орындарға шығуына мүмкіндік берген біздің экономикамыздың стратегиялық маңызды құрамдарының бірі болуға алғашқы қадам болды.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01150`) «Ауыз қуысының күтімі алғашқы тіс шыққанға дейін басталады.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #21 — `өтті` (freq 2666)

- Vowel harmony (auto): **front**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00394`) «Өткен жылы маған жақын адам дүниеден өтті.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00564`) «Содан бері үш жыл өтті.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00688`) «Темір жол даланы кесіп өтті.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #22 — `алу` (freq 2619)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00787`) «Шаш ал десе, бас алу.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00958`) «Бүгін - Еске алу және аза тұту күні.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01143`) «Ақылды адамдар ренжімейді, бірден кек алу жоспарын кұра бастайды.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #23 — `оған` (freq 2556)

- Vowel harmony (auto): **back**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00266`) «Мен оған екі дүркін айттым.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00347`) «Ақпанда оған он жеті жас толады.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00390`) «Бүгін оған қырық жасқа толып отыр.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #24 — `сөз` (freq 2517)

- Vowel harmony (auto): **front**
- Final sound (auto): **voiced_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00197`) «Өткен күн оралмас, кұнды сөз жоғалмас.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00198`) «Жақсы сөз жарым ырыс.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00200`) «Жақсы сөз - жан азығы.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #25 — `қол` (freq 2466)

- Vowel harmony (auto): **back**
- Final sound (auto): **liquid**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00231`) «Көз қорқақ, қол батыр.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00252`) «Мемлекет мүлкіне қол сұқпа!»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00668`) «Өтпес пышақ қол кесер.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #26 — `жатыр` (freq 2389)

- Vowel harmony (auto): **back**
- Final sound (auto): **liquid**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00339`) «Жыл құсы тізбектеліп қайтып жатыр.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00409`) ««Болашақ» бағдарламасы өте пайдалы болып жатыр.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00482`) «Жұмысшылар зауыттан келе жатыр.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #27 — `облысының` (freq 2368)

- Vowel harmony (auto): **back**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00910`) ««Жас қалам» фестивалі Павлодар облысының жас журналистерін жинады.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00947`) «Ақмола облысының екі ауданында ер адам мен бала суға кетті.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00990`) «Павлодар облысының прокуратурасы мүгедектің құқықтарын қорғады.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #28 — `оңтүстік` (freq 2342)

- Vowel harmony (auto): **mixed**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00547`) «Оңтүстік Қазақстан облысы аграрлы аймақ болып есептеледі.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00588`) «Қазақстанның оңтүстік бөлігінде бұршақ жауады.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00959`) «Республиканың оңтүстік және солтүстік аймақтарында күннің қатты ысуы сақталады.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #29 — `отырып` (freq 2326)

- Vowel harmony (auto): **back**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01082`) «Біз оларға өз алғысымызды білдіре отырып, ары қарай да серіктестігіміз жемісті әрі бірлескен жұмысымыз жалғасады деп үміттенеміз.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01087`) «Жоғары сапалы қызмет көрсете отырып, клиенттердің барлық қажеттіліктерін барынша қанағаттандыруға тырысамыз.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01559`) «Жүктелген міндеттерге сүйене отырып, «Қазақстанның тарих ғылымының тұжырымдамасы» сынды маңызды құжаттың дайындық барысы туралы да атап өткен орынды.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #30 — `орталық` (freq 2305)

- Vowel harmony (auto): **back**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01037`) «Елбасы Н.Назарбаев Қазақстан мен Кембридж университеті құрған Орталық Азия инновациялық қоры жұмысының басталғандығы туралы хабарлады.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01561`) «Бұл ауқымды әрі мейлінше ұқыпты жұмысты қажет ететіндігі ескеріліп, ағымдағы жылдың күзінде тарихи-гуманитарлық бейіндегі барлық академиялық ғылыми-зерттеу институттарының, ЖОО-лардың, жетекші отандық …»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01615`) «Орталық Азия арқылы Шығыстың суперөркениеті мен Жерорта теңізін бірінші рет қосқан Жібек жолы, терең археологиялық із қалдырды.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #31 — `алды` (freq 2296)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00005`) «Олар бізден барлық қагаз жұмысты өзіне алды.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00096`) «Олар бізден қағазбен жасайтын барлық жұмысты тартып алды.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00338`) «Көтірілісшілер байдың ат-айғырларын тізіп алды.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #32 — `алған` (freq 2252)

- Vowel harmony (auto): **back**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00984`) «Кубанда орын алған табиғат апаты 141 адамның өмірін қиды.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01054`) «Шымкенттен Новосібірге бағыт алған, ішінде 22 жолаушы отырған халықаралық автобус жыраға түсіп кетті.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01629`) «Дала өмірінде орын алған оның кездесетін көп санды варианттары 1949 жылы бірегей ережеге келтірілді.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #33 — `одан` (freq 2145)

- Vowel harmony (auto): **back**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00170`) «Мен одан ертерек келдім.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01200`) «Кездесу қорытындысымен оның жоғары нәтижелі болғаны атап өтілді және одан әрі өзара іс-қимылды дамытуға сенім білдірілді.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01557`) «Бұл тұрғыда, Еуразия ұлттық университеті қабырғасында өткен «Тарихшылардың ұлттық конгресі» қоғамдық бірлестігінің соңғы жиынының тікелей тарихи ғылымды одан әрі дамытудың өзекті проблемаларына, тарих …»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #34 — `облыстық` (freq 2134)

- Vowel harmony (auto): **back**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00977`) ««Тілдарын» қысқа мерзімде оқыту орталығы Екібастұз және Ақсу қалаларының оқытушылары үшін қазақ тілін оқытудың заманауи әдістеріне арналған оқыту семинарын өткізді, - деп хабарлады тілдерді дамыту Пав …»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01360`) «Салтанатты шараға Ахат Күленовтің туыстары, өңір басшысы, ардагерлер, металлургиялық кәсіпорындардң өкілдері, жастар облыстық мәслихат мүшелері қатысты.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01427`) «Облыстық әкімдіктен хабарланғандай, мерекеге көршілес Астраханнан, Магнитогорскіден және біздің елге тиесілі «Қазақстан геологы» шипажайы орналасқан Железноводскіден делегациялар келеді деп күтілуде.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #35 — `млн` (freq 1999)

- Vowel harmony (auto): **unknown (no vowels)**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00972`) «Иран тарабы тағы бір мәрте қазақ елінен 5 млн. тоннаға дейін астық сатып алуға дайын екендігін қуаттады.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01439`) «Қаржы полицейлері осы жылдың ақпан айында ол бір топ адамдармен келіскен түрде кедендік күзет орнының айналма жолына 2,2 млн. теңге сома тұратын «Орифлейм» опа-далабы және иіссуі салынған үш қорапты а …»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01450`) «Осылайша биылғы жылы 4,5 млн. тоннаға дейін жаңа өнім жиналады деген жоспар жүзеге асырылмақ.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #36 — `батыс` (freq 1994)

- Vowel harmony (auto): **back**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00127`) «Франция Батыс Еуропада орналасады.»
  2. (wikipedia_kz_pack.json / `wiki_kz_0000043`) «Батысында Еділдің төменгі ағысынан, шығысында Алтай тауларынан, солтүстіктегі Батыс Сібір жазығынан, оңтүстіктегі Қызылқұм шөлі мен Тянь-Шань тау жүйесіне созылады.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0000070`) «35 жылдан кейін мемлекеттің шығыс және батыс бөліктері бөлек кетті.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #37 — `дамыту` (freq 1906)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00977`) ««Тілдарын» қысқа мерзімде оқыту орталығы Екібастұз және Ақсу қалаларының оқытушылары үшін қазақ тілін оқытудың заманауи әдістеріне арналған оқыту семинарын өткізді, - деп хабарлады тілдерді дамыту Пав …»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01128`) «Сөйтіп, империя кезеңінде Қазақстанда телекоммуникацияны дамыту үшін негізі салынды.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01527`) «ҚР Президенті Н.Ә.Назарбаевтың үш тілділікті дамыту бойынша тапсырмасын орындау әрі әлемдік білім беру жүйесіне кірігудің келесі маңызды факторы - бірнеше тілді жетік меңгерген кадрлар дайындау.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #38 — `орталығы` (freq 1894)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00977`) ««Тілдарын» қысқа мерзімде оқыту орталығы Екібастұз және Ақсу қалаларының оқытушылары үшін қазақ тілін оқытудың заманауи әдістеріне арналған оқыту семинарын өткізді, - деп хабарлады тілдерді дамыту Пав …»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01049`) ««Сколково» инновациялық орталығы Қазақстанмен ынтымақтастықты кеңейтпек ниетте.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0000190`) «2018 жылғы 19 маусымға дейін Оңтүстік Қазақстан облысының орталығы болған, бұл қалада қазір 1 028 673 адам тұрады (2019).»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #39 — `іске` (freq 1888)

- Vowel harmony (auto): **front**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00783`) «Біткен іске сыншы көп, піскен асқа жеуші көп.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00858`) «Мен бұл іске мен таңмын.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00916`) «Екібастұзда эмульсиялық жарылғыш заттар шығаратын зауыт іске қосылды.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #40 — `отыр` (freq 1846)

- Vowel harmony (auto): **back**
- Final sound (auto): **liquid**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00188`) «Ол саудырлап соғып отыр.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00365`) «Бұл алаңдаушылық Солтүстік Кореяның «Ынха-3» зымыран тасымалдағышын ұшыруға байланысты болып отыр.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00390`) «Бүгін оған қырық жасқа толып отыр.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #41 — `онда` (freq 1813)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01089`) «Егер біздің қызметімізге қатысты сұрақтарыңыз болса немесе мамандардан түсініктеме алғыңыз келсе, онда төмендегі үлгіні толтырып, сұрақ қойыңыз.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01182`) «Егер препаратты кездейсоқ жағдайда жоғарғы дозада қабылдаған болса, онда асқазанды шаю және құсқызу керек.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01539`) «Университеттің ұлттық рейтинг агенттіктеріндегі ұстанымына қатысты айтар болсам, онда бұл жерде елдің үздік университеттері көшбасшылығын осымен 2 жыл қатарынан 1996 жылы Мемлекет басшысы Нұрсұлтан На …»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #42 — `дейді` (freq 1770)

- Vowel harmony (auto): **front**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_02092`) «Бай ат мінсе, "қайырлы болсын" дейді, жарлы ат мінсе, "қайдан алдың" дейді.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_02092`) «Бай ат мінсе, "қайырлы болсын" дейді, жарлы ат мінсе, "қайдан алдың" дейді.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_02588`) «Ол ештеңе көрмедім дейді.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #43 — `орын` (freq 1711)

- Vowel harmony (auto): **back**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00319`) «Қарттарға орын берейік, жастар!»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00984`) «Кубанда орын алған табиғат апаты 141 адамның өмірін қиды.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01356`) «Цунами қаупі орын алу мүмкіндігі туралы хабарланбады.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #44 — `бет` (freq 1673)

- Vowel harmony (auto): **front**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01810`) «Төңкеріс мен азамат соғысын (1917-22 жылдары) бастан кешкеннен кейін мемлекет бұқаралыққа қарай бет алып дене шынықтыру мен спортқа қамқорлық жасай бастады.»
  2. (wikipedia_kz_pack.json / `wiki_kz_0000438`) «Ұлттық топырақта пайда болып, өзіндік бет- бейнесімен ерекшеленсе де, мәдени өркендеудің жалпы арнасына келіп құйылатын Ресей мәдениеті, оның ішінде ұлттық бұлақтан нәр алған ежелгі Русь мәдениеті, ви …»
  3. (wikipedia_kz_pack.json / `wiki_kz_0003790`) «Денудациямен және жерге мүжіліп бет жағы ашылып қалған Көкшетаудың аласа тауларын түзетін гранитті құрам жыныстары көптеген таң қаларлықтай мүсіндерді кескіндейді.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #45 — `маңызды` (freq 1659)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00076`) «Бұл маңызды емес.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01132`) «Бұл сала бойынша еліміздің көптеген көрсеткіштермен жоғары орындарға шығуына мүмкіндік берген біздің экономикамыздың стратегиялық маңызды құрамдарының бірі болуға алғашқы қадам болды.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01488`) «Осыған дейін хабарланғандай, Қытай Халық Республикасы Төрағасының ресми сапары барысында екіжақты ынтымақтастық туралы бірқатар маңызды құжаттарға қол қойылды.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #46 — `орыс` (freq 1612)

- Vowel harmony (auto): **back**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00691`) «Мен орыс тілінен қазақ тіліне аударамын.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00966`) «"Домбыра" журналы қазақ, орыс және ағылшын тілдерінде шығады.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01675`) «Бортинженер Мұсабаев гитарамен өзін сүйемелдеп қазақ және орыс әндерін орындады.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #47 — `бастады` (freq 1589)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00302`) «Бірте-бірте аспан ашыла бастады.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00466`) «Олар өртті сөндіре бастады.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00716`) «Судың көлемі кішірейе бастады.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #48 — `бойы` (freq 1574)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00130`) «Жұмыс тәулік бойы істеледі.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00533`) «Әсері 30 минуттан кейін басталады және 6-12 сағат бойы жалғасады.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00698`) «Қонақ үй тәүлік бойы жұмыс істейді.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #49 — `мақсаты` (freq 1535)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01518`) «Жоғары білім беру жүйесінің басты мақсаты - әлемдік деңгейге қарай ілгері басу.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01632`) «Байқаудың мақсаты - Қазақстандағы интернет жүйесінің қарқынды дамуына, соның ішінде ақпараттық ресурстар мен сервистерді дамытуға, қоғамның ақпараттық мәдениеттілігін жоғарылатуға, желілік қауымдастық …»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_02044`) «Көкпаршылардың мақсаты – орталық шеңберде жатқан Көкпарды өз командасының “отауына” жеткізу.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #50 — `мал` (freq 1532)

- Vowel harmony (auto): **back**
- Final sound (auto): **liquid**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00674`) «Қарамаса қатын кетеді, бақпаса мал кетеді.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00819`) «Мал ашықта жайылып жүр.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00821`) «Ол мал азығын дайындады.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #51 — `ашық` (freq 1528)

- Vowel harmony (auto): **back**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00157`) «Бүгін күн ашық.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00556`) «Қазақстанның барлық халқына ашық аспан, өсіп-еркендеу мен бақ-береке, Отанымыз - Қазақстан Республикасының игілігі үшін жаңа жетістіктер тілеймін.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00818`) «Мәселе ашық қалды.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #52 — `елдің` (freq 1498)

- Vowel harmony (auto): **front**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00191`) «Отан – елдің анасы, ел – ердің анасы.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01025`) «Павлодар мемлекеттік университеті - елдің жоғары білім көшбасшыларының бестігінде.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01205`) «40 елдің сарапшылары Ташкентте инвестиция тарту мәселесін зерделеді.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #53 — `деді` (freq 1493)

- Vowel harmony (auto): **front**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (wikipedia_kz_pack.json / `wiki_kz_0000587`) «Ол сонымен қатар «өз күшіне сүйену» шетелдік кенеттен бас тарту немесе «тұйықтық» дегенді білдірмейді деді.»
  2. (wikipedia_kz_pack.json / `wiki_kz_0008407`) ««Ясауитану» ғылыми орталығын немесе институтын құру мәселесін ойластыруға болады», — деді.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0009655`) «ҰҚШҰ Бас хатшысы ретіндегі Тасмағамбетов Орешник ракетасы, Ресейдің Украинаға басып кіруі мен Украина егемендігі жайлы бірнеше даулы сөздер айтты және осы үшін «Миротворець» сайты оны 5 әскери қылмыс  …»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #54 — `спорт` (freq 1482)

- Vowel harmony (auto): **back**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01145`) «Спорт әр адамның байлығы болып табылатын денсаулықты нығайтатыны ешкім үшін құпия емес.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01308`) «Спорт күнінде Қарағандының Тәуелсіздік алаңында қол алысу кең ауқымды шарасы өтті, акцияға 4 мыңға жуық адам қатысты.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01313`) «Олардың арасында спортшылар, халықаралық жарыстардың жеңімпаздары, еңбек сіңірген спорт қайраткерлері, ардагерлер болды.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #55 — `даму` (freq 1365)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01072`) «Н. Назарбаев Қазақстанның 2050 жылға дейінгі даму бағдарламасын жариялайды.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01442`) «ҚР Парламенті Сенатының Экономикалық даму және кәсіпкерлік комитетінің төрағасы Талғатбек Абайділдин Беларусь Республикасына келді, деп хабарлады палатаның баспасөз қызметі.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01520`) «Біздің іс-қимылдарымыздың басты мақсаттары 2020 жылға дейін Л.Н.Гумилев атындағы Еуразия ұлттық университетінің даму стратегиясын жүзеге асыру болып саналады.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #56 — `облыс` (freq 1333)

- Vowel harmony (auto): **back**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01058`) «Павлодарлық археологтардың кешендік ғылыми экспедициясы облыс аумағындағы қазба жұмыстары кезінде табылған бірегей олжаларды ұсынды.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01361`) «Облыс әкімі Бердібек Сапарбаев өз сөзінде Ахат Сәлемхатұлының өз ісімен халық жадында мәңгі сақталып қалғандығын баса айтты.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0000093`) «Әкішілік құрылымы бойынша құрамына 17 облыс, 89 қала, соның ішінде 3 республикалық маңызы бар қалалар (Астана, Алматы, Шымкент), 186 аудан, 174 ауылдық округ кіреді.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #57 — `алайда` (freq 1283)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01411`) «Болжам бойынша, қазанның басында күн сәл жылынады, алайда ол ұзаққа созылмайды.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01472`) «Алайда бұл іске шетелдік әскерилердің араласуы күн тәртібінен ресми Дамаскі, сонымен бірге РФ мен АҚШ-тың арасындағы Сирияның химиялық қаруын халықаралық бақылауға алу туралы келесімінен кейін алынып  …»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01590`) «Алайда олардың барлығы зеректілік пен тез есептеу және санауға негізделген жекпе-жекке саяды.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #58 — `орынбасары` (freq 1232)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01731`) «Шымкент қаласындағы Әл-Фараби атындағы мәдениет институтының түлегі А.Нысанғалиев Индер ауылындағы «ИндерБорат» МҮ директоры болып, аудандық комсомол комитетінің екінші хатшысы, кинофикация басқармасы …»
  2. (wikipedia_kz_pack.json / `wiki_kz_0001389`) «Үкіметтің уақытша басшысы болып тағайындалған премьердің орынбасары Құрбанқұлы Бердімұхамедов 2007 жылдың ақпан айының басында өткен демократиялық емес кезектен тыс президенттік сайлауда жеңіске жетті …»
  3. (wikipedia_kz_pack.json / `wiki_kz_0005831`) «Бекмұхамет Мұсаұлы, 1993 жылы сәуір айынан бастап 1998 жылы наурыз айына дейін Іле Қазақ автономиялық облыстық партком хатшысының орынбасары, облыс бастығы болған.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #59 — `орташа` (freq 1223)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01453`) «Орташа көрсеткіш гектарынан 10,1 центнерден болады деп атап өтті.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_02042`) «Семіз серке терісі жыртылмайды. Орташа салмағы 20-30кг-дай келеді.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_02284`) «Орташа баллың қанша?»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #60 — `елді` (freq 1205)

- Vowel harmony (auto): **front**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00165`) «Індет көп елді жайлады.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01325`) «Соғыс елді жұтатты.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01506`) «Ұлыбританияның Стаффордшир графтығына қарайтын Стоук-он-Трент елді мекеніндегі әулие Петр атындағы католик шіркеуі жергілікті мұсылман жамағатына сатылды.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #61 — `отырған` (freq 1199)

- Vowel harmony (auto): **back**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00667`) «Екі қыз көрпе қабуға отырған.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01054`) «Шымкенттен Новосібірге бағыт алған, ішінде 22 жолаушы отырған халықаралық автобус жыраға түсіп кетті.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_02221`) «Ол студенттермен қоршалған отырған еді.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #62 — `етіп` (freq 1193)

- Vowel harmony (auto): **front**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00248`) «Біреу үйге сылаң етіп кіріп келді.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00346`) «Боран жолды тып-типыл етіп кетті.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00704`) «Бүркіттің болат тұяғы түлкінің арқасына кірш етіп кіріп кетті.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #63 — `денсаулық` (freq 1174)

- Vowel harmony (auto): **mixed**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00106`) «Тазалық - денсаулық кепілі.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00427`) «Сіздерге денсаулық, бақыт, барлық жақсылықтар мен игіліктерді тілейміз!»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00472`) «Сізге зор денсаулық тілейміз.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #64 — `мақсатында` (freq 1152)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01393`) «Өндіру үдерісіндегі бақылау (өндірістік бақылау) – бақылау мақсатында және қажеттілік кезде үдерістісті басқару кезінде, өзіндік ерекшелігіне байланысты өнімдердің сәйкестігін қамтамасыз ету үшін техн …»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01445`) «Жоғары Еуразиялық экономикалық кеңестің 2012 жылғы 19 желтоқсандағы «Трансшекаралық нарықтарға жататын өлшемдерді бекіту туралы» шешімінің тапсырмаларын орындау мақсатында Еуразиялық экономикалық коми …»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01556`) «Мемлекеттік хатшының тапсырмасына сәйкес біздерге, тарихшыларға «бүкәлемдік тарихпен өзара байланысты ұлт этногенезін мыңжылдық/бір жарым мың жылдық көкжиектегі ұлттық тарихты тұтас елестетуді қалыпта …»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #65 — `жамбыл` (freq 1144)

- Vowel harmony (auto): **back**
- Final sound (auto): **liquid**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00913`) «Жамбыл облысында ыстықтар басталғалы алты табиғи өрт тіркелді.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00933`) «Елбасының Жамбыл облысына сапары күтілуде.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01697`) «Жамбыл облысы полициясы 10 жастағы баланы қағып кетіп, оқиға орнынан ізін суытқан жүргізушіні іздестіруде.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #66 — `алуға` (freq 1118)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00171`) «Енді дем алуға болады.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00972`) «Иран тарабы тағы бір мәрте қазақ елінен 5 млн. тоннаға дейін астық сатып алуға дайын екендігін қуаттады.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_02409`) «Супермаркетке суық су сатып алуға бардым.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #67 — `осындай` (freq 1112)

- Vowel harmony (auto): **back**
- Final sound (auto): **glide**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01699`) «Технологиялық жарақталуы бойынша әлемде осындай заманауи аффинаж зауыты тек Қазақстанда ғана болады.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01937`) «Осындай артықшылықтарымызды тиімді пайдалана отырып, біз екі жақты жетістіктерге жетеміз.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0000341`) «Осындай табиғат жағдайы ауыл шаруашылығының мамандануына, халықтың орналасуы мен тығыздығына, өнеркәсіп пен құрылыс кешендерінің қалыптасуына өзіндік ықпалын тигізеді.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #68 — `облысында` (freq 1098)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00892`) «Алматы облысында психикалық науқас әйел бірге тұратын ерін оқтаумен ұрып өлтірді.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00906`) «67 жыл бұрын (1945) Қостанай облысында топографиялық-геодезиялық қызмет ұйымдастырылды.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00913`) «Жамбыл облысында ыстықтар басталғалы алты табиғи өрт тіркелді.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #69 — `басып` (freq 1068)

- Vowel harmony (auto): **back**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00845`) «Мас адам әлтек-тәлтек басып келе жатыр.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_03381`) «Қиындық - басып алуда емес, қолға түскенді сақтауда.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0000295`) «Кейінгі ғасырларда Орыс мемлекеті көршілес аумақтарды басып алып, өзінің құрамына қосу саясатын жүргізді.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #70 — `видео` (freq 1064)

- Vowel harmony (auto): **mixed**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_03160`) «Фото және видео түсірілімге тыйым салынады.»
  2. (wikipedia_kz_pack.json / `wiki_kz_0054884`) «Қазіргі уақытта «видео ойын» мен «компьютер ойыны» көбінесе синоним ретінде қарастырылады.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0088461`) «Сол жылдың 24-қыркүйегінде тұтынушылар видео және фотосуреттерді қарауға мүмкіндік беретін сайттың жаңа нұсқадағы көрінісін тамашалады.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #71 — `қыз` (freq 1062)

- Vowel harmony (auto): **back**
- Final sound (auto): **voiced_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00667`) «Екі қыз көрпе қабуға отырған.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00940`) «Қыз жүрегі неғұрлым көбірек аймалағанды сүйеді.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01008`) «Екі жасар қыз бала бесінші қабаттан құлап кетті.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #72 — `басым` (freq 1049)

- Vowel harmony (auto): **back**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00768`) «Басым ауырады.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01316`) «Командалық жарыста Ресей әлем құрамасынан 4:1 есебімен басым түсті.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0000283`) «Өйткені Еуразияда дүние-жүзі мемлекеттерінің 40%-дан астамы, халықтың басым көпшілігі орналасқан.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #73 — `маңызы` (freq 1049)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00175`) «Сөзінің маңызы жоқ.»
  2. (wikipedia_kz_pack.json / `wiki_kz_0000093`) «Әкішілік құрылымы бойынша құрамына 17 облыс, 89 қала, соның ішінде 3 республикалық маңызы бар қалалар (Астана, Алматы, Шымкент), 186 аудан, 174 ауылдық округ кіреді.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0000381`) «Олардың болашақ ұрпаққа сапалы білім мен саналы тәрбие берудегі және Ресей тарихын туристерге таныстырудағы танымдық маңызы өте жоғары.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #74 — `ала` (freq 1039)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00011`) «Алдын ала төлемсіз ал да, артық төлемсіз төле.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_02592`) «Бұл гитара өте қымбат болғандықтан, мен оны ала алмаймын.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_03176`) «Том дәл қалағанын ала алмады.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #75 — `орны` (freq 1030)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01345`) «Оның қорытындысы бойынша жалпы сомасы 110,9 млрд. теңге болатын 70 меморандумға қол қойылды, облыста 6 мыңға жуық жаңа жұмыс орны ашылады.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01347`) «Көрменің қорытынды күні бойынша ғана облыста 3 мыңға жуық жұмыс орны пайда болады.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01401`) «Ромашинск кен орны кезінде Кеңес Одағының энергетикалық қауіпсіздігін қамтамасыз еткен болатын.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #76 — `алғаш` (freq 1019)

- Vowel harmony (auto): **back**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00994`) «Еуропаға алғаш жеткізілген Испан конкистадорлары бұл тәттіге «қара алтын» деген атау берді және оны дене күшін нығайту үшін пайдаланды.»
  2. (wikipedia_kz_pack.json / `wiki_kz_0000185`) «Ал жылқы алғаш рет (б.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0000198`) «Біздің заманымызға жеткен жазба деректерде Шымкент алғаш рет елді мекен ретінде парсы тарихшысы Шараф ад-Дин Әли Йаздидің (1425 жыл) біздің жыл санауымыз бойынша 1365–1366 жылдардағы Әмір Темірдің әск …»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #77 — `млрд` (freq 1015)

- Vowel harmony (auto): **unknown (no vowels)**
- Final sound (auto): **voiced_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00971`) «Қазақстан мен Иран өзара сауда-саттық айналымын 5 млрд. долларға дейін жеткізбек.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01345`) «Оның қорытындысы бойынша жалпы сомасы 110,9 млрд. теңге болатын 70 меморандумға қол қойылды, облыста 6 мыңға жуық жаңа жұмыс орны ашылады.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01346`) «Көрменің бесінші күні аясында, әкімдіктер мен инвесторлар арасында жалпы сомасы 37,6 млрд. теңге болатын 23 меморандумға қол қойылды.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #78 — `қай` (freq 1008)

- Vowel harmony (auto): **back**
- Final sound (auto): **glide**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00493`) «Қай жеріңіз ауырады?»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00614`) «Бүгін аптаның қай күні?»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00617`) «Қазір қай жыл мезгілі?»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #79 — `көмек` (freq 1003)

- Vowel harmony (auto): **front**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00404`) «Маған көмек керек.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00999`) «Қазіргі заман ғылымының айтуы бойынша шоколадта демалуға көмек көрсететін, психологияны орнына келтіретін элементтері бар.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01720`) «Медициналық көмек көрсету кезінде қыз үзіліп кетті.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #80 — `қалды` (freq 993)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00078`) «Не болып қалды?»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00401`) «Сатушы қатты таң қалды.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00462`) «Ағаштарда жапырақтар солып қалды.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #81 — `баспасөз` (freq 984)

- Vowel harmony (auto): **mixed**
- Final sound (auto): **voiced_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01424`) «МАГАТЭ бас конференциясының жұмысы 20 қыркүйекке дейін созылады, деп қосты ҚР СІМ баспасөз қызметі.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01430`) «ҚР Табиғи монополияларды реттеу агенттігінде әдістемелік кеңестің кезекті отырысы өтті, деп хабарлады ведомствоның баспасөз қызметінен.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01442`) «ҚР Парламенті Сенатының Экономикалық даму және кәсіпкерлік комитетінің төрағасы Талғатбек Абайділдин Беларусь Республикасына келді, деп хабарлады палатаның баспасөз қызметі.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #82 — `ден` (freq 982)

- Vowel harmony (auto): **front**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00901`) «1977 жылдан бастап жыл Халықаралық мұражай күні сайын әлемнің 150-ден астам елінде атап өтіледі.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00930`) «Бакудегі мұнай-газ көрмесіне 300-ден астам әлемдік компаниялар қатысады.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01352`) «Бұл шараға түрлі елдерден 200-ден астам сарапшылар қатысты.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #83 — `бақылау` (freq 967)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01393`) «Өндіру үдерісіндегі бақылау (өндірістік бақылау) – бақылау мақсатында және қажеттілік кезде үдерістісті басқару кезінде, өзіндік ерекшелігіне байланысты өнімдердің сәйкестігін қамтамасыз ету үшін техн …»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01393`) «Өндіру үдерісіндегі бақылау (өндірістік бақылау) – бақылау мақсатында және қажеттілік кезде үдерістісті басқару кезінде, өзіндік ерекшелігіне байланысты өнімдердің сәйкестігін қамтамасыз ету үшін техн …»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01393`) «Өндіру үдерісіндегі бақылау (өндірістік бақылау) – бақылау мақсатында және қажеттілік кезде үдерістісті басқару кезінде, өзіндік ерекшелігіне байланысты өнімдердің сәйкестігін қамтамасыз ету үшін техн …»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #84 — `батыр` (freq 954)

- Vowel harmony (auto): **back**
- Final sound (auto): **liquid**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00231`) «Көз қорқақ, қол батыр.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00502`) «Біз Ақпан батыр көшесіндегі үш бөлмелі пәтерде тұрамыз.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0001830`) «Көтібар батыр — қазақ батыры, Сүйінғар батырдың баласы Түрікпенбай палуан — кіші жүздің жауырыны жерге тимеген палуаны.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #85 — `бұған` (freq 949)

- Vowel harmony (auto): **back**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00733`) «Бұған кім жауапты?»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00734`) «Бұған кім кінәлы?»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01568`) «Бұған дейін баламасы болмаған «Мәңгі ел» ғылыми-танымдық тарихи журналды жарыққа шығара отырып, оның қысқа ғана уақыттың ішінде үш тілде басылып шыққандығына ерекше назар аударғым келеді.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #86 — `президент` (freq 945)

- Vowel harmony (auto): **front**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01660`) «Кеше Қазақстан Республикасының Вена қаласындағы халықаралық ұйымдар жанындағы Тұрақты өкілдігі Ядролық сынақтарға жаппай тыйым салу жөніндегі шарт ұйымымен (ЯСЖТСШҰ) бірлесе отырып, Президент Н. Назар …»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_02660`) «Президент бейбітшілікті қалайды.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0000108`) «Қоғамдық қауіпсіздікті күшейту мақсатында Президент Қ.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #87 — `ойын` (freq 943)

- Vowel harmony (auto): **back**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01585`) «Бұл ойын математикалық ойлау қабілетін дамытады.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01599`) «Гиподинамикалық зияткерлік ойын ауыл өмірінде сирек жағдайда болды.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01798`) «Павел Коцур дебюттік сап түзеуде олқылықсыз ойын көрсетіп табанды күресті.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #88 — `пайдалану` (freq 937)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01420`) «159 мүше-мемлекет делегацияларының жыл сайынғы басты кездесуі ауқымында МАГАТЭ-нің атом энергиясын бейбіт мақсатта пайдалану, қауіпсіздікті қамтамасыз ету саласындағы қызметін қарады, ядролық қаруды т …»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01446`) ««Дөңгелек үстелге» қатысушылар бірлескен жұмыс басымдықты жағдайды асыра пайдалану және адал емес бәсекелестік жөніндегі бұзушылықтарды анықтауға, сондай-ақ бұл тәртіпті бұзушыларға тиісті шаралар қаб …»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01486`) «Қазақстан мен Қытай ғарыштық кеңістікті бейбіт мақсатта пайдалану және зерттеу салаларындағы ынтымақтастық туралы келісімге қол қойды.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #89 — `алдында` (freq 930)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00886`) ««П» дыбысы дауысты дыбыс алдында ұяңдап кетеді.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01111`) «Баланы тамақтандырар алдында банкінің ішіндегісін араластырыңыз.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01120`) «Пайдаланар алдында қыздыруға шыдамды ыдысқа салып, жылытыңыз да, араластырыңыз.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #90 — `байланыс` (freq 928)

- Vowel harmony (auto): **back**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01639`) «Тағы бір трамвай желісі бойынша байланыс желісіне ағаш құлаған.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01672`) «Ғарышкерлер Медеумен телерадио байланыс орнатты.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01681`) «Байланыс үзілді.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #91 — `бұдан` (freq 919)

- Vowel harmony (auto): **back**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00849`) «Бұдан не өнеді дейсін?»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01418`) «Бұдан өзге, ҚР Үкіметі басшысы Францияның іскер топтарының өкілдерімен кездеседі.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01523`) «ЖОО-ның ғылыми-зерттеу жұмыстарында Инженерлік бейімдегі зертхана (бұдан ары - ИБЗ) мен Инновациялық парк қызметі ерекше орынға ие.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #92 — `дене` (freq 916)

- Vowel harmony (auto): **front**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00994`) «Еуропаға алғаш жеткізілген Испан конкистадорлары бұл тәттіге «қара алтын» деген атау берді және оны дене күшін нығайту үшін пайдаланды.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01726`) «Атырауда облыстық мәдениет және облыстық туризм, дене шынықтыру және спорт басқармаларының жаңа жетекшілері тағайындалды.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01728`) «2006-2012 жылдары ол Атырау қаласының туризм, дене шынықтыру және спорт бөлімінің бастығы болған.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #93 — `істер` (freq 914)

- Vowel harmony (auto): **front**
- Final sound (auto): **liquid**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01243`) «1950 жылы мамырдың 9-ында Парижде Франция сыртқы істер министрі Робер Шуман Франция, Германия және басқа еуропа елдерінің көмір және болат құятын өнеркәсіп салаларын (әскери техника өндіру негіздерін) …»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01894`) «Ол Аргентинаның сол кездегі сыртқы істер министрі қызметін атқарған.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_03113`) «Ресей Сыртқы істер министрінің Қытайға сапары аяқталды.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #94 — `басында` (freq 906)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00221`) «Жыланның уы басында болады, бейғамның жауы қасында болады.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00559`) «Соңғы жылдары Ресей мен Қытай осы тізімнің көш басында келеді.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00739`) «Екінші дүниежүзілік соғыстың басында ағылшындар әскери министрлік өтіп кеткен соғысқа әзірленіп жатыр деп қалжыңдайды екен.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #95 — `алдын` (freq 896)

- Vowel harmony (auto): **back**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00011`) «Алдын ала төлемсіз ал да, артық төлемсіз төле.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00719`) «Келін қайынның алдын кес-кестемес болар.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01337`) «Бұл комиссия орталық-мемлекеттік органдардың және жергілікті басқару органдарының лаңкестіктің алдын алу жөніндегі қызметтерін үйлестіруді жүзеге асырады.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #96 — `бастаған` (freq 896)

- Vowel harmony (auto): **back**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01304`) «Ескі бұрғы ұңғымасы орнында жер астынан газ шыға бастаған.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01422`) «Форумға қатысу үшін Венаға индустрия және жаңа технологиялар вице-министрі Бақытжан Жақсалиев бастаған қазақстандық делегация келді, оның құрамында Венадағы халықаралық ұйымдар жанындағы тұрақты өкілд …»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01566`) «Жоғарыда аталған кеңейтілген отырыста Мемлекеттік хатшы Марат Тәжиннің тапсырмасымен жарық көре бастаған «Мәңгі ел» атты жаңа халықаралық ғылыми-танымдық тарих журнал да зор сілкініс тудырып, жанды та …»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #97 — `хабарлайды` (freq 872)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01439`) «Қаржы полицейлері осы жылдың ақпан айында ол бір топ адамдармен келіскен түрде кедендік күзет орнының айналма жолына 2,2 млн. теңге сома тұратын «Орифлейм» опа-далабы және иіссуі салынған үш қорапты а …»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01455`) ««Мектепке жол» іс-шарасы кезінде үй-үйді аралау барысында Екібастұз қаласы ІІД қызметкерлері үйлердің бірінен ішімдікке мас болған анасының жанынан 2010 жылы туған кішкентай баланы тауып алды, деп хаб …»
  3. (wikipedia_kz_pack.json / `wiki_kz_0001696`) «Ешкіөлмес жотасы және оң жағалуы Көксу өзенінің арасындағы жолында көптеген қола дәуірінен орта ғасыр дәуіріне дейінгі археологиялық ескерткіштер қоныс ежелден мекендегенін хабарлайды.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #98 — `шығады` (freq 866)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00305`) «Амалын тапқан қырғыннан да аман шығады.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00589`) «Ертең Астанада көшпелі бұлт шығады.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00591`) «Ертең елордасы Астанада көшпелі бұлт шығады, жауын-шашын болмайды.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #99 — `орай` (freq 855)

- Vowel harmony (auto): **back**
- Final sound (auto): **glide**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00058`) «Өкінішке орай, бұл шындық.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01020`) «Елбасының атына Астана күніне және туған күнге орай тілек айтылған хаттар көптеп түсуде.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0001756`) «Тарихи жеңістің құрметіне орай алыстан көрінетін монумент салынды.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #100 — `бай` (freq 828)

- Vowel harmony (auto): **back**
- Final sound (auto): **glide**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_02092`) «Бай ат мінсе, "қайырлы болсын" дейді, жарлы ат мінсе, "қайдан алдың" дейді.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_02093`) «Бай бауырын танымас, сауда досқа қарамас.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_02177`) «Бай болыңыз!»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #101 — `сай` (freq 814)

- Vowel harmony (auto): **back**
- Final sound (auto): **glide**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (wikipedia_kz_pack.json / `wiki_kz_0000172`) «1998 жылы ЮНЕСКО бас қаланы «Бейбітшілік қаласы» деген атауға сай деп танып, медальмен марапаттады.»
  2. (wikipedia_kz_pack.json / `wiki_kz_0000312`) «Ендігі жерде Ресей белгілі бір елдер тобына бағдарланған саясат жүргізуге қарағанда, өзінің экономикалық, саяси мүдделеріне сай келетін мемлекеттердің барлығымен өзара тиімді байланыстар орнатуға көшт …»
  3. (wikipedia_kz_pack.json / `wiki_kz_0002080`) «Мамандардың айтуынша, теннис корты халықаралық үлгідегі құрылыс талаптарына сай келеді.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #102 — `басталды` (freq 813)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00304`) «Дәнді дақылдарды жаппай себу басталды.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00384`) «Мектеп оқушыларының каникулы басталды.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00489`) «Қоңырау соғылды, сабақ басталды.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #103 — `отбасы` (freq 805)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01366`) «Мемлекет басшысы белгілі жазушы Д. Досжанның қайтыс болуына байланысты отбасы мен туған-туыстарына көңіл айту жеделхатын жолдады.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_02780`) «Сізде үлкен отбасы бар ма?»
  3. (wikipedia_kz_pack.json / `wiki_kz_0003388`) «Шаңырақ – отбасы құт-берекесінің, бейбітшілік пен тыныштықтың нышаны.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #104 — `сонда` (freq 788)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01582`) «Адамдар шығыстан көшкен кезде, Шинар жерінде бір жазық алқапты тауып, сонда көшті.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_02252`) «Мен дүйсенбіге қарай сонда боламын.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_03267`) «Сонда бұған уақыт болмады.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #105 — `алынған` (freq 783)

- Vowel harmony (auto): **back**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00957`) «Елбасы заңсыз жолмен алынған кірістерді заңдастыруға және терроризмді қаржыландыруға қарсы іс-қимыл мәселелері бойынша заңнаманы жетілдіруге бағытталған заңға қол қойды.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01457`) «Анасы ешқайда жұмыс істемейді, ер адаммен бірге тұрады, баспанасы жоқ, ІІБ-да есепке алынған.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_02853`) «Студенттер кітапханадан алынған кітаптарды пайдалануы керек.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #106 — `орнына` (freq 781)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00999`) «Қазіргі заман ғылымының айтуы бойынша шоколадта демалуға көмек көрсететін, психологияны орнына келтіретін элементтері бар.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01787`) «Қазір қирап қалған бұл үй сырылып тастап, орнына жаңа шіркеу салынды.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0000039`) «Қазақ кирилл әліпбиі 1940 жылы енгізіліп, соның алдында 1929 жылдан қолданылып жүрген латын графикасының орнына келген.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #107 — `бағалау` (freq 777)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (wikipedia_kz_pack.json / `wiki_kz_0001353`) «Олар жануардың популяциясын сандық бағалау, тіршілік ету ортасын зерттеу, генетикалық зерттеулерді қамтиды (Ерғалиев және т.»
  2. (wikipedia_kz_pack.json / `wiki_kz_0003137`) «2004 жылы құрылыстық мердігерлер келісімімен жасалған жұмыс көлемі бағалау бойын-ша 27,7 млрд теңгені құрады, ол 2003 жылға қарағанда 1,3 есеге артық.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0005303`) «Абайдың дін туралы толғамдарын дұрыс түсініп, бағалау оған тарихи тұрғыдан қарауды талап етеді.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #108 — `алатын` (freq 776)

- Vowel harmony (auto): **back**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00323`) «Лашын - аң алатын құстың бір түрі.»
  2. (wikipedia_kz_pack.json / `wiki_kz_0000140`) «шекарасында немесе аум-ның шегінде туған кез келген заңсыз қарулы күш көрсетуді тұмшалап, тойтара алатын деңгейде ұстау; әуе кеңістігін күзету, сондай-ақ, мемл.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0000265`) «Әрине, қаланың солтүстік бөлігін дендробақ, зообақ, ипподроммен байланыстыратын ұзындығы 6 шақырымдай балалар темір жолының алатын орыны айрықша.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #109 — `жоспары` (freq 772)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (wikipedia_kz_pack.json / `wiki_kz_0003285`) «Бұл бағытта облыста “Бақ өсіру және жүзім шаруашылығын қарқынды дамыту шараларының 2014-2016 жылдарғы арналған кешенді жоспары” бекітілді.»
  2. (wikipedia_kz_pack.json / `wiki_kz_0008672`) «Дарынды инженер, бекіністерді қайта салып, жолдар, көпірлер салды, жолдар мен қоршаулардың, бекіністердің жоспары мен картасын түсірді.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0009528`) «Қаланың бас жоспары 1968 жылы бекітілді.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #110 — `қаза` (freq 772)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00224`) «Олар соғыста қаза болғандарды әскери дәстүр бойынша қойып еді.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00943`) «Елбасының Өкімімен 5 маусым Қазақстанда шекарашылардың қаза болуына байланысты аза тұту күні болып жарияланды.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01055`) «Қаза тапқандар мен зардап шеккендер жоқ.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #111 — `өтіп` (freq 763)

- Vowel harmony (auto): **front**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00254`) «Көзінің сұғы өтіп кетті.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00366`) «Сіздің уақытың өтіп кетті.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00695`) «Баланың іші өтіп жүр.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #112 — `ашылды` (freq 757)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00962`) «Қашыр ауданында «Самал» балалар лагері ашылды.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01002`) «Еліміздің алдынан жаңа перспективалар ашылды.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01358`) «Өскеменде еліміздің атақты металлургы Ахат Күленовке ескерткіш ашылды.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #113 — `мамыр` (freq 756)

- Vowel harmony (auto): **back**
- Final sound (auto): **liquid**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00428`) «1 Мамыр – Қазақстан халқы бірлігі күні.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00517`) «Мамыр мерекелеріңізбен құттықтаймыз!»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00550`) «7 мамыр - Отан қорғаушы күні - Қазақстанда мемлекеттік мереке болады.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #114 — `еліміздің` (freq 742)

- Vowel harmony (auto): **front**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01002`) «Еліміздің алдынан жаңа перспективалар ашылды.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01132`) «Бұл сала бойынша еліміздің көптеген көрсеткіштермен жоғары орындарға шығуына мүмкіндік берген біздің экономикамыздың стратегиялық маңызды құрамдарының бірі болуға алғашқы қадам болды.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01358`) «Өскеменде еліміздің атақты металлургы Ахат Күленовке ескерткіш ашылды.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #115 — `салып` (freq 742)

- Vowel harmony (auto): **back**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00893`) «Ақсу ферроқорытпа зауыты өз қызметкерлері үшін салып жатқан үйді аяқтауда.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01120`) «Пайдаланар алдында қыздыруға шыдамды ыдысқа салып, жылытыңыз да, араластырыңыз.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01662`) «Жиналғандардың алдында сөз алған Кәріпбек Күйіков өз басынан кешкен ядролық сынақтардың қасіреті туралы тағы да тыңдаушылардың есіне салып, ядролық сынақтарға біржола және үзілді-кесілді тыйым салу ма …»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #116 — `елбасы` (freq 740)

- Vowel harmony (auto): **mixed**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00890`) «Елбасы РФ Президентін Қазақстанға ресми сапар жасауға шақырды.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00957`) «Елбасы заңсыз жолмен алынған кірістерді заңдастыруға және терроризмді қаржыландыруға қарсы іс-қимыл мәселелері бойынша заңнаманы жетілдіруге бағытталған заңға қол қойды.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00965`) «Елбасы шахматтан әлем чемпионатын ашады.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #117 — `азаматтық` (freq 734)

- Vowel harmony (auto): **back**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01848`) «Азаматтық татулық пен ұлтаралық келісім – біздің басты құндылығымыз.»
  2. (wikipedia_kz_pack.json / `wiki_kz_0000105`) «Оның негізгі мақсаты – жұртшылық, саяси партиялар, азаматтық қоғам өкілдерінің қатысуымен өтетін талқылаудың негізінде мемлекеттік саясаттың өзекті мәселелері бойынша ұсыныстар мен ұсынымдар әзірлеу.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0000136`) «“Ұлан”, азаматтық және аумақтық қорғанысты басқару мен құру органдары кіреді.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #118 — `ислам` (freq 731)

- Vowel harmony (auto): **back**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01716`) «Олардың пікірінше, мүсін спорттық этикаға және ислам қағидаларына қарама-қайшы келеді.»
  2. (wikipedia_kz_pack.json / `wiki_kz_0000073`) «Алтын Орда тұсында Шыңғыс ханның түркіленген ұрпағы Ислам дінін қабылдап, аймаққа билеуін жалғастырды.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0000886`) «Халық жаппай ислам дінін қабылдай бастады.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #119 — `банк` (freq 722)

- Vowel harmony (auto): **back**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_02139`) «Қаржылар банк жүйесінде айналады.»
  2. (wikipedia_kz_pack.json / `wiki_kz_0003432`) «Сонымен қатар, экономикалық деңгейдің осы көрсеткіштеріне өңірдің өзге де тартымды сипаттарын: дамыған банк саласын, шағын және орта бизнестің серпінді дамуын, біліктілігі жоғары мамандардың болуын, қ …»
  3. (wikipedia_kz_pack.json / `wiki_kz_0006104`) «БТА Банкі, АҚ (бұрынғы атауы «ТұранӘлем Банкі» АҚ (ТӘБ) 2008 жылдың бірінші тоқсанында ребрендинг нәтижесінде жаңа атауға ие болды) — Қазақстанның жүйе құрушы банкі, ТМД елдеріндегі банк желісін құруш …»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #120 — `маусым` (freq 712)

- Vowel harmony (auto): **back**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00049`) «Бүгін 18 маусым, енді бұл Мюриэлдің туған күні!»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00943`) «Елбасының Өкімімен 5 маусым Қазақстанда шекарашылардың қаза болуына байланысты аза тұту күні болып жарияланды.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0000094`) «Тұрғыны (1 маусым 2023 ж.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #121 — `осыған` (freq 712)

- Vowel harmony (auto): **back**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00445`) «Осыған сәйкес оқиға былтыр да болып еді.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01488`) «Осыған дейін хабарланғандай, Қытай Халық Республикасы Төрағасының ресми сапары барысында екіжақты ынтымақтастық туралы бірқатар маңызды құжаттарға қол қойылды.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_02574`) «Осыған қарамастан, мәселені ешкім шеше алмады.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #122 — `бағытталған` (freq 708)

- Vowel harmony (auto): **back**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00957`) «Елбасы заңсыз жолмен алынған кірістерді заңдастыруға және терроризмді қаржыландыруға қарсы іс-қимыл мәселелері бойынша заңнаманы жетілдіруге бағытталған заңға қол қойды.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01519`) «Қазақстан дамуының жаһандық мақсаттарына сәйкес біздің жоғары мектеп жүйеміз әлемдік санаттағы университтерді қалыптастыруды, әлемдік деңгейдегі мамандарды дайындауды қамтамасыз етуге бағытталған.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01522`) «Л.Н.Гумилев атындағы Еуразия ұлттық университетінің ғылыми-зерттеу қызметі басымдықты салаларда іргелі ғылымдар мен қолданбалы ізденістерді дамытуға бағытталған.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #123 — `директоры` (freq 683)

- Vowel harmony (auto): **mixed**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01421`) «Агенттік бас директоры лауазымына кезекті төрт жылғы кезеңге Юкия Амано бекітілді, сондай-ақ МАГАТЭ-ге жаңа мүшелер - Бруней-Даруссалам мен Багам аралдары достастығы қабылданды.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01731`) «Шымкент қаласындағы Әл-Фараби атындағы мәдениет институтының түлегі А.Нысанғалиев Индер ауылындағы «ИндерБорат» МҮ директоры болып, аудандық комсомол комитетінің екінші хатшысы, кинофикация басқармасы …»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01731`) «Шымкент қаласындағы Әл-Фараби атындағы мәдениет институтының түлегі А.Нысанғалиев Индер ауылындағы «ИндерБорат» МҮ директоры болып, аудандық комсомол комитетінің екінші хатшысы, кинофикация басқармасы …»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #124 — `отырады` (freq 682)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01701`) «Тәулік бойы пайдаланылған 3 тонна су тазартылып, қайта іске жаратылып отырады.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_02765`) «Гүлге сары көбелек отырады.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0001867`) «Геофизикалық зерттеулердің мәліметтері бойынша оның жалпы калыңдығы облыстың Солтүстік Каспий аймағында 1,5–3 километрге дейін, Каспий маңындағы ойпат ернеуінің оңтүстік-шығысында 8–10 километрге дейі …»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #125 — `жетті` (freq 681)

- Vowel harmony (auto): **front**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00354`) «Оның төбесі көкке жетті.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_02557`) «Біздің команда кеше жеңіске жетті.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0000699`) «Тарихи жолмен, КСРО металл комбинаты өндірісіндегі ең ірі зауыт болды және әлемдегі ең ірі зауыттардың бірі болып саналды: 1991 жылға дейін өндіріс жылына 17 мың тоннаға жетті (Әлемдік өндірістің 10%- …»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #126 — `қан` (freq 681)

- Vowel harmony (auto): **back**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_03779`) «Үйдің бәрінде қан төгілген еді.»
  2. (wikipedia_kz_pack.json / `wiki_kz_0003013`) «Киікотынан дайындалған тұнбамен ревматизм, құрысуды, жалпы қан тамырының соғысын көтереді.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0010358`) «Дәндерінің қан тоқтататын және ішек құртын түсіретін қасиеті бар.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #127 — `алдымен` (freq 678)

- Vowel harmony (auto): **mixed**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_02679`) «Бірақ алдымен кофемді аяқтағым келеді.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_03652`) «Алдымен, біз оны тауып алуымыз керек.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0000324`) «Ол, ең алдымен, алып жатқан аумағының өте үлкен болуымен байланысты.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #128 — `белсенді` (freq 675)

- Vowel harmony (auto): **front**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01083`) «Белсенді әрі ұзақ мерзімді серіктестікке үміт артамыз!»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01850`) «Біздің моделіміз шетелдік инвестицияларды тартудағы мемлекеттің белсенді рөліне негізделген.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0000393`) «Олардың басым көпшілігі (71 млн адам) экономикалық белсенді халық болып табылады.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #129 — `метр` (freq 675)

- Vowel harmony (auto): **front**
- Final sound (auto): **liquid**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (wikipedia_kz_pack.json / `wiki_kz_0001507`) «Облыстың оңтүстік жағы - орташа биіктігі 300-400 метр болатын бұйратты, жон-жоталы жазық.»
  2. (wikipedia_kz_pack.json / `wiki_kz_0001698`) «Көптеген сақ-көшпенділер қорғандары өзінің мөлшерімен таң қалдырады: диаметрі шамамен 100 метр.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0001722`) «Белгілі “Әуенді бархан” құмды тауының биіктігі 120 метр және шамамен 3 км ұзындығы саябақтың бір бұрышында өзеннен бірнеше километрде немесе Үлкен және Кіші Қалқан таулары арасында орналасады.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #130 — `маған` (freq 672)

- Vowel harmony (auto): **back**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00075`) «Бұл маған ұнаған жоқ.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00090`) «Бұл маған өте ұнайды.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00101`) «Маған аудармашы керек.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #131 — `ортақ` (freq 665)

- Vowel harmony (auto): **back**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01648`) «Топса – бөлшектердің кинематикалық айналмалы жұбын түзетін және ортақ осьі айналасында айналуына мүмкіндік беретін қозғалмалы қосылысы.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01830`) «Біздің басты мақсатымыз – 2050 жылға қарай мықты мемлекеттің, дамыған экономиканың және жалпыға ортақ еңбектің негізінде берекелі қоғам құру.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01924`) «Бұл жаңа БАҚ-тардың мүмкіншіліктерін ашуда үлкен рөл атқарып, екі елдің гуманитарлық әріптестігін нығайтып, екіжақты қарым-қатынастардың ортақ дамуына үлесін қосады.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #132 — `дамыған` (freq 664)

- Vowel harmony (auto): **back**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01576`) «Бүгінгі таңда біз бір нәрсені жетік түсінеміз: тарихты білмей «жаңа қазақстандық отансүйгіштіктің» болуы мүмкін емес, Ұлт көшбасышысы Н.Ә.Назарбаев көрегендікпен айтқв андай, патриотизмсіз әлемнің дам …»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01595`) «Егер болжанған кезеңмен келіссек, онда табылған затты ежелгі ойынның арабтардың әсерімен дамыған уақытына, шатрандждың өзіндік ерекшеліктерге ие болған кезеңдеріне жатқызған жөн.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01830`) «Біздің басты мақсатымыз – 2050 жылға қарай мықты мемлекеттің, дамыған экономиканың және жалпыға ортақ еңбектің негізінде берекелі қоғам құру.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #133 — `істейді` (freq 661)

- Vowel harmony (auto): **front**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00145`) «Ол қайда жұмыс істейді?»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00149`) «Менің әйелім "Арзан" дүкенінде сатушы болып жұмыс істейді.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00156`) «Шешем зауытта істейді.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #134 — `балық` (freq 660)

- Vowel harmony (auto): **back**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01039`) «Менде екі алтын балық бар.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01340`) «Атырау облысында полиция қызметкерлері бір тонна балық тәркіледі.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01341`) «Заңсыз ауланған балық фактісі бойынша тексеру жүргізілуде.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #135 — `дала` (freq 659)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01629`) «Дала өмірінде орын алған оның кездесетін көп санды варианттары 1949 жылы бірегей ережеге келтірілді.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01764`) «Бұл – Ұлы дала көшпенділері айрықша қастер тұтқан өмір мен мәңгіліктің символы.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01807`) «Тоғызқұмалақ пен шахмат ауыртпалықты жылдары да дала өмірімен бірге болды.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #136 — `оқиға` (freq 654)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00118`) «Полиция оқиға туралы бізге мәлімет берді.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00119`) «Полиция оқиға туралы бізге хабарлады.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00445`) «Осыған сәйкес оқиға былтыр да болып еді.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #137 — `түсіп` (freq 653)

- Vowel harmony (auto): **front**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01054`) «Шымкенттен Новосібірге бағыт алған, ішінде 22 жолаушы отырған халықаралық автобус жыраға түсіп кетті.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01914`) «Сондықтан қазіргі кездегі біздің басты мақсатымыз екіжақты саяси қарым-қатынасты одан сайын дамыта түсіп, жоғарыда атап өтілген ұзақмерзімді стратегияларды орындау жолында күш біріктіру болуы қажет.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01918`) «Біз халықтарымыздың да тығыз қарым-қатынасқа түсіп, мәдени, білім алу, туристік және басқа да гуманитарлық салаларда әріптестікті дамытып, екі ел арасындағы қарым-қатынасқа мықты рухани іргетас та орн …»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #138 — `еске` (freq 652)

- Vowel harmony (auto): **front**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00958`) «Бүгін - Еске алу және аза тұту күні.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01782`) «Бұрынғыны айтпай, соңғы еске түспейді.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0001723`) «Бұл құмды тау ұсақ таза құмнан, желі болғанда тау “ән” салып, даусы бірнеше километрге жетеді және органның даусын еске түсіреді.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #139 — `орындары` (freq 632)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (wikipedia_kz_pack.json / `wiki_kz_0000131`) «Атап айтқанда, бүгінде 500-ден астам кен орындары барланып, минералды шикізаттың 1220 түрінің барлығы анықталған.»
  2. (wikipedia_kz_pack.json / `wiki_kz_0000134`) «Қазақстан Республикасының әскери құрылымы әскери басқару органдарын, Қарулы Күштердің түрлерін, арнайы әскерлерді, тыл, әскери оқу орындары мен ғыл.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0000217`) «Қалада жаңа өндіріс орындары салынып, оқу орындары мен ғылым, мәдениет ошақтары ашылды.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #140 — `пайдалы` (freq 625)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00409`) ««Болашақ» бағдарламасы өте пайдалы болып жатыр.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_02203`) «Том маған көптеген пайдалы нәрселер үйретті.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_02652`) «Жүзу денсаулыққа пайдалы.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #141 — `дамуына` (freq 619)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00723`) «Мемлекет басшысы Николя Саркозиге көп жылдар бойы Қазақстан мен Франция арасындағы қарым-қатынастардың дамуына қосқан еңбегі үшін алғысын білдірді.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01632`) «Байқаудың мақсаты - Қазақстандағы интернет жүйесінің қарқынды дамуына, соның ішінде ақпараттық ресурстар мен сервистерді дамытуға, қоғамның ақпараттық мәдениеттілігін жоғарылатуға, желілік қауымдастық …»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01924`) «Бұл жаңа БАҚ-тардың мүмкіншіліктерін ашуда үлкен рөл атқарып, екі елдің гуманитарлық әріптестігін нығайтып, екіжақты қарым-қатынастардың ортақ дамуына үлесін қосады.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #142 — `сайлау` (freq 607)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (wikipedia_kz_pack.json / `wiki_kz_0001039`) «Ол бір палатадан тұрды және тең сайлау құқығы негізінде, жасырын дауыс беру жолымен 5 жыл мерзімге сайланды.»
  2. (wikipedia_kz_pack.json / `wiki_kz_0001171`) «1938 жылғы 24 маусым — Қазақ КСР Жоғарғы Кеңесіне алғашқы сайлау.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0001183`) «1963 жылғы 3 наурыз — 6-шы шақырылған Қазақ КСР Жоғарғы Кеңесіне және жергілікті кеңестерге сайлау.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #143 — `дамуы` (freq 603)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01456`) «Тексеру барысында отбасында баланың қалыпты өмір сүруі және дамуы үшін жағдай жасалмағаны анықталды.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01916`) «Сонымен қатар, шикізаттық емес саладағы әріптестікті де сәт сайын дамытып, инвестициялық әріптестіктің де қарқынды дамуы үшін жаңа қайнар көздер іздей беруіміз керек.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0000126`) «Дәл осылай деп тау-кен металлургия сала-соңғы оң жыл көлемінде жалпы қазақстандық экспорт және қызмет көрсету үлесінің көбеюі, әлемдік сауданың дамуымен салыстырғандағы оның қарқынды дамуы мен көлемін …»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #144 — `педагогикалық` (freq 602)

- Vowel harmony (auto): **mixed**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01544`) «Сіздің ЖОО-да жаңа тарих факультетінің ашылуы педагогикалық құрамға ерекше жауапкершілік жүктейтіндігі анық.»
  2. (wikipedia_kz_pack.json / `wiki_kz_0000858`) «Қасым Тыныстанов (ИГУ) Қырғыз мемлекеттік педагогикалық университеті ат.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0001107`) «1928 жылғы қыркүйек — Абай атындағы Қазақ педагогикалық институтының ашылуы.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #145 — `ерте` (freq 599)

- Vowel harmony (auto): **front**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00182`) «Бүгін мен ерте тұрдым.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_02052`) «Қар ерте кеткенмен, күн жылынбады.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_02319`) «Том да ерте тұрды еді.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #146 — `басына` (freq 586)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00784`) «Біткен істің басына жақсы келер қасына.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01933`) «Бүгінде Қазақстан жан басына шаққандағы ЖІӨ бойынша Еуразиялық аймақта алдыңғы қатарлы елдердің тобына еніп, тіпті әлемнің 50 дамыған елінің сабынан көріне білді.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_02539`) «Жақсы көріну үшін ол ағаштың басына шықты.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #147 — `пайдалануға` (freq 585)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01123`) «Сегіз айдан асқан балаларға пайдалануға кеңес беріледі.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01579`) «Қаптамасында көрсетілген мерзімінен кешіктіріп пайдалануға болайды.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0000349`) «Алайда жер қорының 1/3-і ғана пайдалануға жарамды, онда Ресей халқының 95%-ы қоныстанып, бүкіл шаруашылық салалары құрылымдарының 93%-ы шоғырланған.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #148 — `азамат` (freq 584)

- Vowel harmony (auto): **back**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00920`) «Астананың әуежайында жалған төлқұжаттармен Германияға ұшпақшы болған 6 ауғандық азамат ұсталды.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01053`) «Бұл қылмыста екі күдікті азамат ауруханада көз жұмды.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01068`) «10 ақтөбелік азамат «Жыл адамы» атағына ие болды.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #149 — `округі` (freq 583)

- Vowel harmony (auto): **mixed**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (wikipedia_kz_pack.json / `wiki_kz_0000149`) «Шығыс әскери округі қорғайтын жер аумағына: Шығ.»
  2. (wikipedia_kz_pack.json / `wiki_kz_0001441`) «КСРО-ның Қызыл Ту Түркістан әскери округі Орталық Азияның тәуелсіз мемлекеттері арасында бөлінгеннен кейін Түрікменстанның үлесіне екі ірі базада — Мары қаласы мен Ашхабадтың астында орналасқан Орталы …»
  3. (wikipedia_kz_pack.json / `wiki_kz_0001573`) «1929 – 32 жылдары Алматы округі аталды.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #150 — `алдыңғы` (freq 577)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00372`) «Алдыңғы күні өте суық болды.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01933`) «Бүгінде Қазақстан жан басына шаққандағы ЖІӨ бойынша Еуразиялық аймақта алдыңғы қатарлы елдердің тобына еніп, тіпті әлемнің 50 дамыған елінің сабынан көріне білді.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0000132`) «Осылардың көпшілігінен біздің еліміз дүние жүзі бойынша алдыңғы орында.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #151 — `республика` (freq 575)

- Vowel harmony (auto): **mixed**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01300`) «Президенттің кезектен тыс сайлауы Республика Президентінің шешімімен тағайындалады және конституциялық заңда белгіленген тәртіп пен мерзімде өткізіледі.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01819`) «Соғысқа дейінгі жылдарда шахмат қозғалысының негізгі даму кезеңдері - қалалардың жолдастық командалық кездесулері, «Динамо», «Буревестник», «Спартак», «Локомотив» спорт қоғамдарының ведомстволық бірін …»
  3. (wikipedia_kz_pack.json / `wiki_kz_0000081`) «Республика Еуразия құрлығының орталығында барлық мұхиттардан бірдей қашықтықта орналасқан.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #152 — `алтай` (freq 565)

- Vowel harmony (auto): **back**
- Final sound (auto): **glide**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (wikipedia_kz_pack.json / `wiki_kz_0000043`) «Батысында Еділдің төменгі ағысынан, шығысында Алтай тауларынан, солтүстіктегі Батыс Сібір жазығынан, оңтүстіктегі Қызылқұм шөлі мен Тянь-Шань тау жүйесіне созылады.»
  2. (wikipedia_kz_pack.json / `wiki_kz_0000082`) «Республика батысында Еділ өзені алабынан шығысында Алтай тауы шыңдарына дейін 3 мың км дейін, солтүстіктегі Батыс Сібір жазығынан (Солтүстік Қазақстан жазығы) оңтүстігінде Қызылқұм шөлі мен Тянь-Шань  …»
  3. (wikipedia_kz_pack.json / `wiki_kz_0000403`) «Тілдік құрамы жағынан Ресей халықтары, негізінен, үндіеуропалық (халықтың 89%-ы), алтай (7%), кавказ (2%) және орал (2%) 4 тіл семьяларына жатады.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #153 — `медициналық` (freq 564)

- Vowel harmony (auto): **mixed**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01720`) «Медициналық көмек көрсету кезінде қыз үзіліп кетті.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_03049`) «Сіз алғашқы медициналық көмек көрсетуді білесіз бе?»
  3. (wikipedia_kz_pack.json / `wiki_kz_0002417`) «Қарағанды облысында 95 аурухана, 359 амбулатория мен емхана болды Оларда 5777 дәрігер, 10866 орта медициналық мамандар жұмыс істеді.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #154 — `алдағы` (freq 563)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01406`) «Алдағы қыс Еуропа үшін қақаған аяздарымен есте қалмақ.»
  2. (wikipedia_kz_pack.json / `wiki_kz_0000384`) «Ресей ғалымдарының болжамы бойынша, алдағы онжылдықта елдегі халық санының қыскаруы одан әрі жалғаса береді.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0003379`) «Негізі 1720 жылы орыстың Ертіс бойындағы әскери бекіністерінің алдағы шебі ретінде қаланған; шебтің атауы одан 20 км жерде орналасқан Коряковка тұзды көлінен алынған.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #155 — `сирек` (freq 563)

- Vowel harmony (auto): **front**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01599`) «Гиподинамикалық зияткерлік ойын ауыл өмірінде сирек жағдайда болды.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_03312`) «Біз бір-бірімізге сирек қоңырау шаламыз.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0000213`) «Оңтүстік Қазақстанның табиғи-климаты дермененің өсуіне қолайлы, ал дермене сантонино өндіріс жершарында өте сирек кездеседі.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #156 — `жедел` (freq 562)

- Vowel harmony (auto): **front**
- Final sound (auto): **liquid**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00952`) «Шекара жасағының «Терісайрық» қосынындағы барлық 11 жедел әскери қызметкері анықталды.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01691`) «Жедел іс-шаралар кезінде үш күдікті ұсталған, олардың арасында 32 жастағы әйел бар.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01721`) «Жедел тергеу тобы қыз баланың сегізінші қабаттағы терезеден өзі секіргенін анықтады.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #157 — `елде` (freq 559)

- Vowel harmony (auto): **front**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01195`) «Елде бұл дата Стокгольмде Қазақстан Футбол одағы Футбол қауымдастықтарының еуропалық бірлестігіне (УЕФА) қабылданған 2002 жылдан бастап атап өтіледі.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_02095`) «Сусыз жерде қамыс жоқ, азған елде намыс жоқ.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_02413`) «Лондон қай елде орналасқан?»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #158 — `салдарынан` (freq 559)

- Vowel harmony (auto): **back**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00948`) «Петропавлда 5 қабатты үйде газдың ауаға жайылуы салдарынан өрт болды, 40 адам қауіпсіз аймаққа көшірілді.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00989`) «ШҚО-да автобус пен жеңіл көліктің соқтығысуы салдарынан төрт адам мерт болды.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01408`) «Қыста Еуропаға Арктиканың ауа массалары ығысады, оның салдарынан нормадан ауытқыған аяздар болады.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #159 — `бастайды` (freq 554)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01143`) «Ақылды адамдар ренжімейді, бірден кек алу жоспарын кұра бастайды.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01151`) «Тіс алты айлық жастан бастап өсе бастайды.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01707`) «Астанадағы аффинаж зауыты алғашқы алтын құймаларды 10 қарашада шығара бастайды.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #160 — `қар` (freq 553)

- Vowel harmony (auto): **back**
- Final sound (auto): **liquid**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00168`) «Қар еріді.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00218`) «Осакада қар жауып тұрды.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00222`) «Үй тар, сырт қар.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #161 — `дегенмен` (freq 551)

- Vowel harmony (auto): **front**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01709`) «Зауыттың ашылуына 47 күн қалды, дегенмен оның құрылысы ертерек аяқталады деп күтілуде.»
  2. (wikipedia_kz_pack.json / `wiki_kz_0000035`) «Қазақша сөйлескен жөн болар, өзімде байқамай орысшылап кетем, дегенмен қазақшалауға тырысам.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0000287`) «Дегенмен Ресей өзінің көршілік жағдайының қолайлы тұстарын әлі де болса толығымен пайдаланбай келеді.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #162 — `басы` (freq 550)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00619`) «Қаңтар – бұл жыл басы.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_02069`) «Қойдың басы — құданың асы.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_02946`) «Томның басы қызба жұмысын жалғастыра берді.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #163 — `бойында` (freq 549)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (wikipedia_kz_pack.json / `wiki_kz_0001612`) «Ірі көлі: Балқаш, Іле өзен бойында Қапшағай бөгені және СЭС-і салынған.»
  2. (wikipedia_kz_pack.json / `wiki_kz_0001672`) «Жетісудағы ерте темір дәуіріндегі жүйенің іздерімен ең ірі қоныс Шарын-Сары-Тоғай өзенінің бойында орналасқан.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0001710`) «Ортағасырлық қалалар керуен жолдарының бойында пайда болған.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #164 — `алмай` (freq 547)

- Vowel harmony (auto): **back**
- Final sound (auto): **glide**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01149`) «Ол тілегін орындата алмай сағы сынды.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_03370`) «Кенеттен Мэри өзін-өзі ұстай алмай күліп жіберді.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0005166`) «Бірақ бұл оқуын әрі жалғастыра алмай, небәрі 3 жылдан соң оның мұсылманша да, орысша да оқуы аяқталады.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #165 — `алмайды` (freq 539)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00073`) «Адамдар мәңгі өмір сүре алмайды.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01922`) «Қытай мен Қазақстанның түрлі салалардағы әріптестігі екі ел халқының қолдауынан жырақ өмір сүре алмайды.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_02062`) «Әлемнің бүкіл архитектурасы өзгереді. Барлық елдер осы күрделі кезеңнен лайықты өте алмайды.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #166 — `орман` (freq 538)

- Vowel harmony (auto): **back**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (wikipedia_kz_pack.json / `wiki_kz_0000361`) «Ең көп таралғаны қылқан-жапырақты ағаштар, олар бүкіл орман қорының 90%-ын құрайды.»
  2. (wikipedia_kz_pack.json / `wiki_kz_0000362`) «Орман аумағының көп бөлігі балқарағайдан тұрады, сондай-ақ қарағай, шырша және самырсын ағаштары да басым.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0000365`) «Әсіресе орман өрттері Ресейдің азиялық бөлігінде климат континенттілігіне байланысты жиі байқалады.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #167 — `хабарлады` (freq 538)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00119`) «Полиция оқиға туралы бізге хабарлады.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00557`) «Ресей тағы да басты компьютерлік қарақшылардың қара тізіміне енгізілді, - деп хабарлады Би-Би-Си.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00977`) ««Тілдарын» қысқа мерзімде оқыту орталығы Екібастұз және Ақсу қалаларының оқытушылары үшін қазақ тілін оқытудың заманауи әдістеріне арналған оқыту семинарын өткізді, - деп хабарлады тілдерді дамыту Пав …»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #168 — `мектебі` (freq 531)

- Vowel harmony (auto): **front**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00555`) «Қазақстан Республикасының Қарулы Күштер қатарында қызмет ету әрбір қазақстандықтың құрметті міндеті, аға буынның батырлық және жауынгерлік дәстүрлеріне негізделген патриотизмнің жоғары адамгершілік ме …»
  2. (wikipedia_kz_pack.json / `wiki_kz_0000645`) «8 ғасырдың орта шенінде Тан императоры Сюань-цзунь сарайының жанынан музыканттар мен бишілер, әншілер дайындайтын «Алмұрт бағы» («Лиюань») атты театр мектебі құрылды.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0001203`) «1925 жылғы тамыз — «Жаңа мектеп» (қазіргі «Қазақстан мектебі») журналының бірінші нөмірінің жарық көруі.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #169 — `орындау` (freq 529)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01338`) «Комиссияның міндеті - лаңкестікке қарсы әрекет ету саласында мемлекеттік саясатты орындау болып табылады.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01445`) «Жоғары Еуразиялық экономикалық кеңестің 2012 жылғы 19 желтоқсандағы «Трансшекаралық нарықтарға жататын өлшемдерді бекіту туралы» шешімінің тапсырмаларын орындау мақсатында Еуразиялық экономикалық коми …»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01525`) «ИБЗ базасында дипломдық жұмыстарды, магистрлық диссертацияларды және ғылыми жұмыстарды орындау жүзеге асырылады.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #170 — `ісі` (freq 529)

- Vowel harmony (auto): **front**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (wikipedia_kz_pack.json / `wiki_kz_0001558`) «Оқыту ауыл және орман шаруашылығы, Құрылыс және коммуналдық шаруашылық, педагогика, медицина, тау-кен ісі, қызмет көрсету саласы, өнер және мәдениет, энергетика, машина жасау технологиясы, көлікті пай …»
  2. (wikipedia_kz_pack.json / `wiki_kz_0002056`) «Атырау облысының аудандары мен ауылдарына арнап медицина кадрларын даярлау ісі колға алынды.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0003145`) «: Денсаулық сақтау ісі жөнінде Маңғыстау облысы әлі де артта қалып келеді.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #171 — `деңгейі` (freq 528)

- Vowel harmony (auto): **front**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (wikipedia_kz_pack.json / `wiki_kz_0000394`) «Жалпы алғанда, Ресейде жұмыссыздық деңгейі 9,2% деп есептеледі.»
  2. (wikipedia_kz_pack.json / `wiki_kz_0000395`) «Бірақ жұмыссыздық деңгейі аумақ, бойынша үлкен айырмашылықтар жасайды.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0000397`) «Ал экономикалық өрлеу тән Саха республикасында, сондай-ақ ірі қалаларда жұмыссыздың деңгейі 4—5%-дан аспайды.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #172 — `жөн` (freq 527)

- Vowel harmony (auto): **front**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00536`) «Егер науқас өзінде жоғарыда аталған қандай да бір жағымсыз әсерлерді немесе басқадай жағымсыз әсерлерді байқаса, емдеуші дәрігерден кеңес алғаны жөн.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00537`) «Есекжем белгілері пайда болған кезде дәрігерге дереу қаралғаны жөн.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01595`) «Егер болжанған кезеңмен келіссек, онда табылған затты ежелгі ойынның арабтардың әсерімен дамыған уақытына, шатрандждың өзіндік ерекшеліктерге ие болған кезеңдеріне жатқызған жөн.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #173 — `жеткен` (freq 524)

- Vowel harmony (auto): **front**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01466`) «Башар Асад Сирияның химиялық қаруды «батылы жеткен елге беруге» әзір екендігін атап өтті.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01499`) «Естеріңізге сала кетейік, 2012 жылдың қыркүйек айында Лондон Олимпиадасының чемпионы Александр Винокуров сол айтулы сайыста жеңіске жеткен велосипедін аукционға қойған болатын.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01636`) «Бельгия Корольдік метеорологиялық институтының мәліметтері бойынша, жел екпіні бүгін таңертеңнен сағатына 97 шақырымға жеткен.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #174 — `олимпиада` (freq 522)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01029`) «Лондонда бүгін Олимпиада қалашығы ашылады.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01036`) «Қазақстан Президенті Лондондағы Олимпиада ойындарының ашылу салтанатына қатысады.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01490`) «Төрт жасар Милена Мезенцева Олимпиада чемпионы Александр Винокуровтың велосипедін сатудан түскен қаражатқа ота жасаудың арқасында ақырындап жүре бастады.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #175 — `елге` (freq 520)

- Vowel harmony (auto): **front**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01427`) «Облыстық әкімдіктен хабарланғандай, мерекеге көршілес Астраханнан, Магнитогорскіден және біздің елге тиесілі «Қазақстан геологы» шипажайы орналасқан Железноводскіден делегациялар келеді деп күтілуде.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01466`) «Башар Асад Сирияның химиялық қаруды «батылы жеткен елге беруге» әзір екендігін атап өтті.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01496`) «Сондай-ақ, Айша Бексұлтанның да қалы жақсарып келеді, қыз баланың жүрегінде ақауы болған, оған Украинада ота жасалып, жақында елге оралды.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #176 — `бен` (freq 516)

- Vowel harmony (auto): **front**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00521`) «Жеңіс күні – абыройымыздың бен даңқымыздың мерекесі.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01085`) «Біз өзіміздің клиенттеріміз бен серіктестерімізбен тұрақты байланыстамыз және олардың көбісімен ұзақ мерзімді өзара тиімді ынтымақтастық құра алғандығымызды мақтаныш тұтамыз.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01088`) «Бізбен әрқашанда бірге болған клиенттеріміз бен серіктестерімізге алғыс білдіреміз!»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #177 — `дайын` (freq 507)

- Vowel harmony (auto): **back**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00972`) «Иран тарабы тағы бір мәрте қазақ елінен 5 млн. тоннаға дейін астық сатып алуға дайын екендігін қуаттады.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01387`) «Карантин – саудаға шығару, ақауға жатқызу немесе қайта өңдеу жөніндегі шешім қабылданға дейін физикалық немесе басқаша түрде оқшауланған бастапқы немесе қаптама материалдардың, аралық, буып-түйілмеген …»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01389`) «Дайын өнім – соңғы буып-түюді қоса алғанда технологиялық процестің барлық сатыларынан өткен өнім.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #178 — `басталады` (freq 505)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00533`) «Әсері 30 минуттан кейін басталады және 6-12 сағат бойы жалғасады.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01136`) «Отан отбасынан басталады.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01150`) «Ауыз қуысының күтімі алғашқы тіс шыққанға дейін басталады.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #179 — `дан` (freq 504)

- Vowel harmony (auto): **back**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01116`) «(18±3) °С температурада және ауаның салыстырмалы ылғалдылығы 75 %-дан аспайтын жерде сақтау керек.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01532`) «Екі жылдың ішінде Л.Н.Гумилев атындағы ЕҰУ-дың 30-дан астам магистранттары мен докторанттары екі дипломды білім беру бағдарламасы бойынша үздік шетелдік университеттерде білім алу мүмкіндігіне ие болд …»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01793`) «Төрт ашық халықаралық жартылай финал өткізілді, оларға Ресей, Украина, Белоруссия, Өзбекстан, Қырғызстан, Түрікмения және Қазақстаннан 120-дан астам білікті ойыншы қатысты.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #180 — `жай` (freq 502)

- Vowel harmony (auto): **back**
- Final sound (auto): **glide**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00159`) «Пойыз жай жүреспін кетті.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_02650`) «Біз жай ғана түскі ас ішіп отырдық.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_02761`) «Бұл жай керемет.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #181 — `осыдан` (freq 499)

- Vowel harmony (auto): **back**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01471`) «Осыдан кейін халықаралық қауымдастық Сирияға қарсы әскери операцияның басталу мүмкіндігін қарастыра бастады.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01931`) «Мен осыдан бірнеше жыл бұрын Астанада болғанмын.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0001973`) «Ғалымдардың тұжырымдауы бойынша бұл алтын киімді адамның өмір сүру кезеңі осыдан 2 мың жыл бұрынғы сарматтар дәуіріне жатады.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #182 — `академиясының` (freq 496)

- Vowel harmony (auto): **mixed**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01891`) «Бұл шараға департаменттің, полиция академиясының жеке құрамдары, әкімшіліктер мен басқармалардың қызметкерлері жұмылдырылатын болады.»
  2. (wikipedia_kz_pack.json / `wiki_kz_0001098`) «1932 жылғы 8 наурыз — КСРО Ғылым академиясының Қазақстандық базасының құрылуы.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0001099`) «1946 жылғы 1 маусым — Қазақ КСР Ғылым академиясының ашылуы.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #183 — `каспий` (freq 493)

- Vowel harmony (auto): **back**
- Final sound (auto): **glide**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01206`) «Қыркүйек айында Бакуде Каспий мәртебесі талқыланбақ.»
  2. (wikipedia_kz_pack.json / `wiki_kz_0000044`) «Қазақстан Каспий теңізі арқылы Әзербайжан, Иран елдеріне, Еділ өзені және Еділ-Дон каналы арқылы Азов теңізі мен Қара теңізге шыға алады.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0000050`) «Батыста Каспий теңізімен (2000 километр), оңтүстік батыста Арал теңізімен шайылады.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #184 — `бақ` (freq 489)

- Vowel harmony (auto): **back**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00807`) «Бақ түссе маңдайдан, тас түссе талайдан.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00850`) «Егін бақ таңдамайды, бап таңдайды.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0003284`) «Республика халқын жеміс-жидек өнімімен және жүзіммен толық қамтамасыз ету үшін облыста озық технологияларды енгізу, соның ішінде бақ егу жұмыстары қарқынды жүргізілуде.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #185 — `одағының` (freq 486)

- Vowel harmony (auto): **back**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01245`) «Жалпы Ұлы Отан соғысында 497 қазақстандық Кеңес Одағының Батыры атағын алды.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01246`) «Бес дивизия гвардиялық аталды, олардың ішінде Кеңес Одағының Батыры И.В.Панфилов атындағы атақты 8-гвардиялық дивизия бар.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01401`) «Ромашинск кен орны кезінде Кеңес Одағының энергетикалық қауіпсіздігін қамтамасыз еткен болатын.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #186 — `ойлау` (freq 484)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01585`) «Бұл ойын математикалық ойлау қабілетін дамытады.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_02561`) «Бұл туралы ойлау менің жүрегімді ауыртады.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0010072`) «Алланың бұл есімін зікір еткен адамның ойлау және жаттау қабілеті күшейеді.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #187 — `кей` (freq 482)

- Vowel harmony (auto): **front**
- Final sound (auto): **glide**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (wikipedia_kz_pack.json / `wiki_kz_0000396`) «Ингушетия, Дағыстан, Қалмақ Республикасында оның керсеткіші кей жылдары 50%-ға жеткен.»
  2. (wikipedia_kz_pack.json / `wiki_kz_0001008`) «Жылдық жауын-шашынның мөлшері әр қилы, кей шөлді өңірлерде 100 мм, кей жерлерде 200–400 мм, тау бөктерлерінде 900 мм.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0001008`) «Жылдық жауын-шашынның мөлшері әр қилы, кей шөлді өңірлерде 100 мм, кей жерлерде 200–400 мм, тау бөктерлерінде 900 мм.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #188 — `хабар` (freq 482)

- Vowel harmony (auto): **back**
- Final sound (auto): **liquid**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01813`) «Ол кезде ауық-ауық шығып тұратын «Шахмат парағында» Ақтөбе, Петропавл, Семей және басқа қалалардағы турнирлер жөнінде хабар беріліп тұрды.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_03686`) «Хабар маған тек таңертең келді.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0001223`) «Қазақ радиосы — Қазақ радиолары ЖШС-не қарасты, Қазақстан тұрғындарына, ТМД елдерінде және шет елдерде тұратын қазақ тыңдармандарына хабар тарататын радио желісі.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #189 — `деңгейде` (freq 478)

- Vowel harmony (auto): **front**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01003`) «Дағдарыс жаһандық деңгейде еңсерілген жоқ және әлемдік қоғамдастық қысымды турбулентті жағдайда тұр.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01004`) «Қазіргі әлем барлық плюстерімен және минустерімен төтенше деңгейде өзара байланысты болып отыр.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01923`) «Осы орайда қос елдің БАҚ-тары жоғары деңгейде әріптестік қарым-қатынас орнатуы уақыт талабы болып табылады.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #190 — `мерзімді` (freq 478)

- Vowel harmony (auto): **front**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01031`) «Елбасы қысқа мерзімді демалысқа шықты.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_01083`) «Белсенді әрі ұзақ мерзімді серіктестікке үміт артамыз!»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_01085`) «Біз өзіміздің клиенттеріміз бен серіктестерімізбен тұрақты байланыстамыз және олардың көбісімен ұзақ мерзімді өзара тиімді ынтымақтастық құра алғандығымызды мақтаныш тұтамыз.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #191 — `былай` (freq 470)

- Vowel harmony (auto): **back**
- Final sound (auto): **glide**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00060`) «Мен былай өмір сүре алмаймын.»
  2. (wikipedia_kz_pack.json / `wiki_kz_0000193`) «Мысалы, шымға қатысты қала атауы былай деп түсіндіріледі: Шымкент түркінің «Шым» және парсының «Кент» — қала, мекен деген сөздерінен құралған.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0002778`) «Далалар ары қарай да созылып жатыр, бірақ оларда мұндай суреттер бұдан былай кездеспейді.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #192 — `мекен` (freq 470)

- Vowel harmony (auto): **front**
- Final sound (auto): **nasal**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (wikipedia_kz_pack.json / `wiki_kz_0000193`) «Мысалы, шымға қатысты қала атауы былай деп түсіндіріледі: Шымкент түркінің «Шым» және парсының «Кент» — қала, мекен деген сөздерінен құралған.»
  2. (wikipedia_kz_pack.json / `wiki_kz_0000198`) «Біздің заманымызға жеткен жазба деректерде Шымкент алғаш рет елді мекен ретінде парсы тарихшысы Шараф ад-Дин Әли Йаздидің (1425 жыл) біздің жыл санауымыз бойынша 1365–1366 жылдардағы Әмір Темірдің әск …»
  3. (wikipedia_kz_pack.json / `wiki_kz_0000199`) «Қандай болғанда да, қала ескі заманнан-ақ адамдардың өмір сүруіне қолайлы мекен болған.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #193 — `дауыс` (freq 469)

- Vowel harmony (auto): **back**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_02287`) «Бір рет қана дауыс бере аласыз.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_03152`) «Монро 65 дауыс жинады.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0001039`) «Ол бір палатадан тұрды және тең сайлау құқығы негізінде, жасырын дауыс беру жолымен 5 жыл мерзімге сайланды.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #194 — `алғанда` (freq 468)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01389`) «Дайын өнім – соңғы буып-түюді қоса алғанда технологиялық процестің барлық сатыларынан өткен өнім.»
  2. (wikipedia_kz_pack.json / `wiki_kz_0000284`) «Жалпы алғанда, Ресейдің 12 мемлекетпен теңіздік шекарасы бар, ал 14 мемлекетпен құрлық арқылы байланысқан.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0000313`) «Геосаяси тұрғыдан алғанда, Ресей нағыз ашық мемлекетке айналды.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #195 — `салмағы` (freq 465)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_01062`) «Шақалақтың салмағы 3,5 келі, бойы 51 см.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_02042`) «Семіз серке терісі жыртылмайды. Орташа салмағы 20-30кг-дай келеді.»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_03080`) «Ол салмағы жоғалтып.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #196 — `алаш` (freq 463)

- Vowel harmony (auto): **back**
- Final sound (auto): **voiceless_consonant**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00956`) «11 жыл бұрын (2001) Қостанай қаласындағы Октябрьдің 50 жылдығы көшесіне Алаш қозғалысының қайраткері, жазушы, ақын, драматург, журналист, 20-шы ғасырдың бас кезіндегі қазақ мәдениеті мен әдебиетінің і …»
  2. (wikipedia_kz_pack.json / `wiki_kz_0001196`) «1921 жылғы 22 наурыз — «Лениншіл жас» газетінің шығуы (1921 — «Жас алаш» , 1922 — «Жас қайрат» , 1927 жылдан бастап — «Лениншіл жас» ).»
  3. (wikipedia_kz_pack.json / `wiki_kz_0002293`) «Берік Шаханұлы (1943-2020) - жазушы, прозаик «Алаш» халықаралық әдеби сыйлығының лауреаты; «Қазақстанның еңбек сіңірген қайраткері».»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #197 — `облысындағы` (freq 460)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (wikipedia_kz_pack.json / `wiki_kz_0001975`) «Ол — Атырау облысындағы Махамбет ауданының аумағында орналасқан ортағасырлық қала.»
  2. (wikipedia_kz_pack.json / `wiki_kz_0003404`) «Көмірді өндіру және тың игеру Павлодар облысындағы өндірістік күштерінің дамуына мықты серпін берді.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0003654`) «Ленинге Павлодар облысындағы бірінші ескерткіш 1928 жылы Павлодар қаласында орнатылған (Ленпарк).»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #198 — `аппараты` (freq 458)

- Vowel harmony (auto): **back**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (wikipedia_kz_pack.json / `wiki_kz_0000239`) «Шымкент қалалық әкімдігі — Шымкент қаласы әкімінің аппараты қала әкімінің қызметін басқаруды және бақылауды, оны ұйымдық, құқықтық, ақпараттық-талдау, консультативтік және материалдық-техникалық қамта …»
  2. (wikipedia_kz_pack.json / `wiki_kz_0001549`) «Ақмола облыстық әкімдігі — Ақмола облысы әкімінің аппараты қала әкімдігі мен әкімінің қызметін басқаруды және бақылауды, ұйымдастырушылық, құқықтық, ақпараттық-талдау, консультативтік және материалдық …»
  3. (wikipedia_kz_pack.json / `wiki_kz_0006385`) «Техникалық ғылымдар кандидаты, екі рет қорғаған: «Ғарыштық пилотталатын кешеннің бортында ғылыми-қолданбалы эксперименттер мен зерттеулер бағдарламасын орындау кезінде «Экипаж — пилотталатын ғарыш апп …»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #199 — `үйге` (freq 458)

- Vowel harmony (auto): **front**
- Final sound (auto): **vowel**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (tatoeba_kazakh_pack.json / `tatoeba_kz_00056`) «Мен оларға ертең соғамын, үйге келген соң.»
  2. (tatoeba_kazakh_pack.json / `tatoeba_kz_00132`) «АҚШ-та әскери ұшақ тұрғын үйге құлады»
  3. (tatoeba_kazakh_pack.json / `tatoeba_kz_00161`) «Олар қонақ үйге жайғасты.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

### Candidate #200 — `абылай` (freq 456)

- Vowel harmony (auto): **back**
- Final sound (auto): **glide**
- POS (default): **noun** — confirm or correct
- Contexts:
  1. (wikipedia_kz_pack.json / `wiki_kz_0001269`) «Олардың ішінде Абылай ханның ақылшысы, суырып салма ақын Бұқар жыраудың ескерткіші, қазақ әдебиетінің классигі, ұлы ақын Абай Құнанбаев ескерткіші және тағы басқалары көз тартарлықтай.»
  2. (wikipedia_kz_pack.json / `wiki_kz_0005667`) «1713 — Абылай хан туды.»
  3. (wikipedia_kz_pack.json / `wiki_kz_0005679`) «1733 — Әбілмәмбет хан бастаған қазақ әскері ойраттарды ауыр жеңіліске ұшыратты; жиырма жасар Абылай сұлтан атаққа шықты.»

- [ ] Approved
- Root form: __
- POS: __
- Harmony override: __
- Final-sound override: __
- Comment:

---

## Tally

Fill in after review. `N` = items reviewed; `A` = approved; `R` = rejected.

- Approve rate: A = __ / N = 200 = ___%
- Reject reasons (count each):
  - loanword: __
  - proper noun: __
  - OCR artefact: __
  - already in Lexicon (auto-tag miss): __
  - not a real Kazakh word: __
  - other: __

## Next step

Bundle approved roots into a single PR against `data/tokenizer/segmentation_roots.json`. Include for each:

```json
{
  "id": "noun_<root>",
  "root": "<root>",
  "part_of_speech": "noun",
  "vowel_harmony": "back|front",
  "final_sound_class": "vowel|voiceless_consonant|voiced_consonant|nasal|liquid|glide"
}
```

Then: `cargo run --release -p adam-corpus --bin morpheme_coverage` to measure delta. The PR description should include the before/after overall-coverage number (per memory `project_morpheme_coverage_baseline`).
