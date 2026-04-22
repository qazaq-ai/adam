# Precision audit — native-speaker review

**Target:** 50-fact sample + 50-derivation sample from the committed artifacts, seed `42`.

- `facts.json`: 13345 facts total (upstream status: `completed`) — sampled 50 here.
- `derived_facts.json`: 207 derivations total (upstream status: `completed`) — sampled 50 here.

## How to review

For each fact, mark the checkbox if the triple `(subject, predicate, object)` is **correct**: the sentence genuinely asserts that the subject has the claimed relation to the object, and both root resolutions are correct. When unsure, leave unchecked and add a one-line note in the Comments row. Update the **Tally** section at the bottom with your counts. Precision is defined as `correct / reviewed`.

---

## Fact sample

### Fact #1218

- Triple: `(бүкіл — does_to — толқын)`
- Predicate: `does_to`
- Pattern: `X Y-ні қайта-лайды`
- Source: `kazakh_textbooks_pack.json / kz_textbook_physics_11_ogn_p0145_s23`
- Sentence:

    > «һо108» - бүкіл және «отарһо» - жазу деген мағынаны бере- ді) когерентті сәулелену көмегімен толқынды өрістерді жазудың, қайта шығарудың және түрлендірудің фотографиялық әдісі

- [ ] Correct
- Comment:

### Fact #1366

- Triple: `(ақмола — does_to — ету)`
- Predicate: `does_to`
- Pattern: `X Y-ні асыр-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0001549`
- Sentence:

    > Ақмола облыстық әкімдігі — Ақмола облысы әкімінің аппараты қала әкімдігі мен әкімінің қызметін басқаруды және бақылауды, ұйымдастырушылық, құқықтық, ақпараттық-талдау, консультативтік және материалдық-техникалық қамтамасыз етуді жүзеге асырады.

- [ ] Correct
- Comment:

### Fact #1440

- Triple: `(табиғи — does_to — адам)`
- Predicate: `does_to`
- Pattern: `X Y-ні құр-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0003219`
- Sentence:

    > Халықтың табиғи өсімі 2025 жылғы қаңтар-қазанда 31436 адамды құрады (өткен жылдың сәйкес кезеңінде 36628 адам).

- [ ] Correct
- Comment:

### Fact #1770

- Triple: `(әд-дин — does_to — қасиет)`
- Predicate: `does_to`
- Pattern: `X Y-ні тона-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0008530`
- Sentence:

    > Шараф әд-Дин Әли Йездидің хабарларына қарағанда, 1388 жылы Ясыны Тоқтамыстың әскерлері талқандап, түрік тайпаларының қасиетті мекеніне айналған Қожа Ахмет Ясауи мазарын тонайды.

- [ ] Correct
- Comment:

### Fact #1799

- Triple: `(сәуір — is_a — қазан)`
- Predicate: `is_a`
- Pattern: `X — Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0009189`
- Sentence:

    > (сәуір1952 — қазан 1952) Құсайынов Т.

- [ ] Correct
- Comment:

### Fact #1806

- Triple: `(қала — related_to — арал)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0009374`
- Sentence:

    > Сырдария өзенінің төменгі ағысындағы көне қалалар мен Арал т-нің табанына жасалған ғылыми экспедициялар барысында құнды археологиялық материалдар жиналды.

- [ ] Correct
- Comment:

### Fact #2280

- Triple: `(тәуекел — does_to — самарқан)`
- Predicate: `does_to`
- Pattern: `X Y-ні тұр-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0017332`
- Sentence:

    > Тәуекел Бұхараны қоршағанда ол 20 мың әскерімен Самарқанды билеп тұрған.

- [ ] Correct
- Comment:

### Fact #2405

- Triple: `(экономикалық — does_to — енгізу)`
- Predicate: `does_to`
- Pattern: `X Y-ні ұсын-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0018943`
- Sentence:

    > Бұл ұсыныстарымен ол жоғарғы органдарға тұрақты түрде жүгініп, баспасөз беттерінде сөз сөйлеп, тіпті аймақтың дамуын КСРО-ның бесжылдық экономикалық даму жоспарына енгізуді ұсынды.

- [ ] Correct
- Comment:

### Fact #2414

- Triple: `(қаныш — does_to — салу)`
- Predicate: `does_to`
- Pattern: `X Y-ні бол-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0019066`
- Sentence:

    > Қаныш Сәтбаевтың бұл жұмыстары Қарағандыдағы металлургиялық завод салуды, Қостанай мен Алтайдағы темір және марганец кендерін, Қаратаудағы фосфорит кендерін игеруді, Ертіс-Қарағанды каналының қазылуын және бірқатар ғылыми зерттеу институттарының ашылуына бастау болды.

- [ ] Correct
- Comment:

### Fact #2767

- Triple: `(сыр — does_to — қала)`
- Predicate: `does_to`
- Pattern: `X Y-ні кір-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0024417`
- Sentence:

    > Ол сыр бойындағы қалаларды азат етуге кірісті.

- [ ] Correct
- Comment:

### Fact #2887

- Triple: `(кесіртке — does_to — тіс)`
- Predicate: `does_to`
- Pattern: `X Y-ні сүр-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0026473`
- Sentence:

    > Теңіздерде алып кесірткелер — мезозаврлар, плезиозаврлар, ихтиозаврлар, құрлықта — динозаврлар, ұзындығы 14 м-ге жететін жылан тәрізділер, әуе кеңістігінде қанаттарының өрісі 8 м-лік өткір тісті жыртқыш құстар өмір сүрді.

- [ ] Correct
- Comment:

### Fact #3020

- Triple: `(жол — related_to — порт)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0028663`
- Sentence:

    > Мұнайды экспортқа шығарудан түсетін орасан пайда қазіргі заманғы қалалар, жолдар мен порттар салуға, инфрақұрылымды дамытуға, мектептер мен ауруханаларды қазіргі заманғы құрал-жабдықтармен жарақтандыруға мүмкіндік берді.

- [ ] Correct
- Comment:

### Fact #3231

- Triple: `(испандық — does_to — секіл)`
- Predicate: `does_to`
- Pattern: `X Y-ні бөл-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0032247`
- Sentence:

    > 1532 — 36 жылдары испан отаршылары жаулап алғаннан кейін Перуліктер испан, креол (испандықтар ұрпақтары), испан-үндіс тектес метистер мен үндістер секілді этносаралық топтарға бөлінді.

- [ ] Correct
- Comment:

### Fact #3421

- Triple: `(пәлсапа — does_to — ақыл-ой)`
- Predicate: `does_to`
- Pattern: `X Y-ні бекіт-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0035662`
- Sentence:

    > Басқа ғылымдармен (пәлсапа, информациялық технологиялар, невроғылым) үлкейіп келе жатқан қарым қатынаспен бірге ақыл-ойды түсіну және шығындарды өнімді түрде пайдалану үшін әртүрлі салаларды бір шатыр астына бекіткен ғылым — когнитивттік ғылым жаратылды.

- [ ] Correct
- Comment:

### Fact #3441

- Triple: `(адалдық — does_to — ұғым)`
- Predicate: `does_to`
- Pattern: `X Y-ні біл-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0035976`
- Sentence:

    > Сол сияқты ақ түсі де киелі, адалдық ұғымды білдіреді.

- [ ] Correct
- Comment:

### Fact #3449

- Triple: `(еуропа — related_to — азия)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0036099`
- Sentence:

    > Еуразияны Еуропа мен Азия дүние бөліктеріне бөлу… ежелгі дәуірде қалыптасқан тарихи-дәстүрлі түсінік.

- [ ] Correct
- Comment:

### Fact #3764

- Triple: `(қазақ — does_to — соғыс)`
- Predicate: `does_to`
- Pattern: `X Y-ні ал-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0041272`
- Sentence:

    > Қазақтар бұл соғысты әрі қарай дамыта алмады.

- [ ] Correct
- Comment:

### Fact #3961

- Triple: `(кітап — does_to — оқу)`
- Predicate: `does_to`
- Pattern: `X Y-ні ұнат-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0045111`
- Sentence:

    > Бос уақытында кітап оқуды, атқа мінуді, шахмат ойнауды және суда жүзуді ұнатқан.

- [ ] Correct
- Comment:

### Fact #4348

- Triple: `(мұнай — related_to — газ)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0052534`
- Sentence:

    > Екінші қателік — мұнай мен газ секторларына мемлекеттің тікелей араласуы, мемлекеттің осы секторлардың өндіріс ошақтарын мемлекеттің қарамағына күштеп алуы, яғни «мемлекеттендіруі».

- [ ] Correct
- Comment:

### Fact #4833

- Triple: `(теориялық — does_to — ат)`
- Predicate: `does_to`
- Pattern: `X Y-ні бол-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0059777`
- Sentence:

    > Ол Иммануил Канттың теориялық және этикалық еңбектерінен бастау алған неміс идеализмі атты пәлсапалық ағымның бастаушыларының бірі болған.

- [ ] Correct
- Comment:

### Fact #4880

- Triple: `(көбінесе — does_to — секіл)`
- Predicate: `does_to`
- Pattern: `X Y-ні қат-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0060598`
- Sentence:

    > Оның қателіктері көбінесе масса, жылдамдық, күш, жылу секілді ұғымдарға қатысты.

- [ ] Correct
- Comment:

### Fact #5009

- Triple: `(дүние — after — жыл)`
- Predicate: `after`
- Pattern: `X Y-дан кейін`
- Source: `wikipedia_kz_pack.json / wiki_kz_0062673`
- Sentence:

    > Бірақ Мажарстаннан тыс жерде (дүние жүзінде) жұрт Рубик текшесі туралы алты жылдан кейін, 1980 ж.

- [ ] Correct
- Comment:

### Fact #5563

- Triple: `(қағаз — has — ие)`
- Predicate: `has`
- Pattern: `X-тың Y-сы бар`
- Source: `wikipedia_kz_pack.json / wiki_kz_0072639`
- Sentence:

    > Олар сонымен қатар бағалы қағаздардың иелері, жеке меншікке иелік етуге олардың құқықтары бар.

- [ ] Correct
- Comment:

### Fact #5812

- Triple: `(әйел — does_to — секіл)`
- Predicate: `does_to`
- Pattern: `X Y-ні бұл-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0077156`
- Sentence:

    > Біздің алдымызда күлімсіреген әйел емес, қандай да бір ажалдың мазағы секілді, бұл түпнұсқадан өте анық байқалады, ал көшірмелеріне бұл ғажап берілмеген.

- [ ] Correct
- Comment:

### Fact #5825

- Triple: `(шикізат — does_to — кен)`
- Predicate: `does_to`
- Pattern: `X Y-ні шоғырлан-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0077412`
- Sentence:

    > Қазақстанда мыс шикізат көздері Орталық, Шығыс (Кенді Алтай) және Оңтүстік Қазақстан облыстарында шоғырланған.

- [ ] Correct
- Comment:

### Fact #5850

- Triple: `(жібек — does_to — дін)`
- Predicate: `does_to`
- Pattern: `X Y-ні ене-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0077931`
- Sentence:

    > Қағанатқа Ұлы Жібек жолы арқылы әлемдік діндерді уағыздаушылар ене бастаған.

- [ ] Correct
- Comment:

### Fact #5883

- Triple: `(күлгін — does_to — күш)`
- Predicate: `does_to`
- Pattern: `X Y-ні бол-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0078316`
- Sentence:

    > Күлгін топырақ қара шіріндіге бай, үстіңгі қабаты едәуір, кейде күшті сілтіленген болады да, ал күлгін қабаты кремнийге бай келеді; одан теменгі қабаттарда темір қышқылы мен алюминий жиналады.

- [ ] Correct
- Comment:

### Fact #6089

- Triple: `(адам — does_to — тәріз)`
- Predicate: `does_to`
- Pattern: `X Y-ні тік-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0081240`
- Sentence:

    > Адам тәрізді маймылдарға салыстырғанда Австралопитектердің аяқтары қолдарынан ұзын, тік жүруге бейімделген.

- [ ] Correct
- Comment:

### Fact #6621

- Triple: `(бөксе — does_to — ен)`
- Predicate: `does_to`
- Pattern: `X Y-ні жалғас-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0090976`
- Sentence:

    > Сонан соң, зардап шегушінің үстіне бөксе жағына қарай отырып, айқастырылған білекпен, енді өзіңнен алға қарай және шамалы төмен жағынан ішін басуды жалғастыр.

- [ ] Correct
- Comment:

### Fact #6837

- Triple: `(жыл — has — қаңтар)`
- Predicate: `has`
- Pattern: `X-тың Y-сы бар`
- Source: `wikipedia_kz_pack.json / wiki_kz_0094776`
- Sentence:

    > 2020 жылдың 1 қаңтарында Вена халқының 30,8% -ы шетел азаматтары (589,015 адам), 36,7%-ы шетелде туылды (701,662 адам) және 41,3%-ы шетелдік текті - олар шетелдіктер немесе шетелде туылған Австрия азаматтығы бар адамдар болды (790 060 адам).

- [ ] Correct
- Comment:

### Fact #7105

- Triple: `(зығыр — related_to — кендір)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0100160`
- Sentence:

    > Мата жасау үшін зығыр мен кендір өсірілді.

- [ ] Correct
- Comment:

### Fact #7583

- Triple: `(ақсүйек — related_to — академиялық)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0108605`
- Sentence:

    > Ол Ұлыбритания, Германия, Аустрия, Нидерланд, Италия, Скандинавия елдерінде, Ресейде, Польшада, Венгрияда ақсүйектер мен академиялық ортада қолданылды.

- [ ] Correct
- Comment:

### Fact #8239

- Triple: `(ауыл — does_to — дән)`
- Predicate: `does_to`
- Pattern: `X Y-ні қарқ-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0120400`
- Sentence:

    > Осы кәсіптің шығарылатын өнімінің көлеміне байланысты, ауыл шаруашылығында мал шаруашылығы мен дәнді – дақылдар шаруашылығы өте қарқынды дамуда.

- [ ] Correct
- Comment:

### Fact #8482

- Triple: `(толық — does_to — жеткілік)`
- Predicate: `does_to`
- Pattern: `X Y-ні бол-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0125928`
- Sentence:

    > Ашығын айтқанда, орыс отаршылдарына қарсы соғысуға Шөмекей руының толық қауқары да дайындығы да жеткілікті болған.

- [ ] Correct
- Comment:

### Fact #8496

- Triple: `(әзербайжан — does_to — мемлекет)`
- Predicate: `does_to`
- Pattern: `X Y-ні қайта-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0126184`
- Sentence:

    > Әзербайжан ұлттық-демикратиялық қозғалысы тәуелсіз мемлекетті қайта жаңғыртуға күресті.

- [ ] Correct
- Comment:

### Fact #9075

- Triple: `(дулыға — does_to — сұс)`
- Predicate: `does_to`
- Pattern: `X Y-ні туғыз-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0135873`
- Sentence:

    > Акропольдің нақ ортасында дулыға киіп, қолына найза ұстағанданышпандық құдайы Афинаның алып қола мүсіні сұсты кейіппен төңіректі қалт жібермей бақылап тұрғандай әсер туғызған.

- [ ] Correct
- Comment:

### Fact #9130

- Triple: `(өсімдік — related_to — бұта)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0136739`
- Sentence:

    > Шөптесін өсімдіктер мен бұта арасында, бау-бақша маңайларында тіршілік етеді.

- [ ] Correct
- Comment:

### Fact #9329

- Triple: `(құс — does_to — қала)`
- Predicate: `does_to`
- Pattern: `X Y-ні іл-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0139888`
- Sentence:

    > Әлді құстар оның кергеніне қарамастан түлкіні сыпыра іледі де, басып қалады немесе сол ілген бойы көтеріп алып, басқа жерге апарып басады.

- [ ] Correct
- Comment:

### Fact #9850

- Triple: `(халық — does_to — билік)`
- Predicate: `does_to`
- Pattern: `X Y-ні асыр-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0149918`
- Sentence:

    > Халық билікті тікелей республикалық референдум және еркін сайлау арқылы жүзеге асырады, сондай-ақ өз билігін жүзеге асыруды мемлекеттік органдарға береді.

- [ ] Correct
- Comment:

### Fact #9870

- Triple: `(тест — does_to — қызыл)`
- Predicate: `does_to`
- Pattern: `X Y-ні үйрет-лайды`
- Source: `synthetic_sentences_pack.json / synth_00030`
- Sentence:

    > тест қиын қызылды үйретті.

- [ ] Correct
- Comment:

### Fact #9952

- Triple: `(имам — does_to — өнім)`
- Predicate: `does_to`
- Pattern: `X Y-ні жете-лайды`
- Source: `synthetic_sentences_pack.json / synth_00510`
- Sentence:

    > имам аумағдың өнімді жетеді әйтпесе қолданыс жарқырайды.

- [ ] Correct
- Comment:

### Fact #10326

- Triple: `(мәртебе — does_to — әрекет)`
- Predicate: `does_to`
- Pattern: `X Y-ні сіңір-лайды`
- Source: `synthetic_sentences_pack.json / synth_02731`
- Sentence:

    > мәртебе әйтенің әрекетті сіңіреді.

- [ ] Correct
- Comment:

### Fact #10999

- Triple: `(зерттеуші — does_to — ішек)`
- Predicate: `does_to`
- Pattern: `X Y-ні жақында-лайды`
- Source: `synthetic_sentences_pack.json / synth_06409`
- Sentence:

    > зерттеуші ішекті жақындады әрі қырғызс қырғызсты қасыйды.

- [ ] Correct
- Comment:

### Fact #11863

- Triple: `(қағаз — does_to — еңбегін)`
- Predicate: `does_to`
- Pattern: `X Y-ні құтыл-лайды`
- Source: `synthetic_sentences_pack.json / synth_11156`
- Sentence:

    > қағаз қамтамадан еңбегінді құтылды.

- [ ] Correct
- Comment:

### Fact #12286

- Triple: `(ету — does_to — қылмыстық)`
- Predicate: `does_to`
- Pattern: `X Y-ні өсір-лайды`
- Source: `synthetic_sentences_pack.json / synth_13679`
- Sentence:

    > өңдеуде ету қылмыстықты өсірді.

- [ ] Correct
- Comment:

### Fact #12414

- Triple: `(пышақ — does_to — аяқ)`
- Predicate: `does_to`
- Pattern: `X Y-ні төмендет-лайды`
- Source: `synthetic_sentences_pack.json / synth_14377`
- Sentence:

    > пышақ жұбановға ресми аяқты төмендетті.

- [ ] Correct
- Comment:

### Fact #12858

- Triple: `(апта — does_to — ғарыш)`
- Predicate: `does_to`
- Pattern: `X Y-ні қаба-лайды`
- Source: `synthetic_sentences_pack.json / synth_16735`
- Sentence:

    > әлбетте апта ғарышты қабады.

- [ ] Correct
- Comment:

### Fact #12903

- Triple: `(құнарл — does_to — әйгіл)`
- Predicate: `does_to`
- Pattern: `X Y-ні қара-лайды`
- Source: `synthetic_sentences_pack.json / synth_16999`
- Sentence:

    > құнарл құяның әйгілді қарайды.

- [ ] Correct
- Comment:

### Fact #13095

- Triple: `(қызылжар — does_to — жұмыс)`
- Predicate: `does_to`
- Pattern: `X Y-ні жүс-лайды`
- Source: `synthetic_sentences_pack.json / synth_18059`
- Sentence:

    > сенімді қызылжар жұмысынды жүсті әрі -тармағ кеңейтті.

- [ ] Correct
- Comment:

### Fact #13098

- Triple: `(қыркүйек — does_to — тәңір)`
- Predicate: `does_to`
- Pattern: `X Y-ні кеңей-лайды`
- Source: `synthetic_sentences_pack.json / synth_18092`
- Sentence:

    > баяу қыркүйек тәңірді кеңейді бірақ жақсы ескерткіш сілкінеді.

- [ ] Correct
- Comment:

---

## Derivation sample

### Derivation #0

- Triple: `(еңбек — is_a — өзен)`
- Rule: `R1_is_a_transitivity`
- Confidence: `rule_inferred`
- Source chain: kazakh_proverbs_pack.json/proverb_068, wikipedia_kz_pack.json/wiki_kz_0139793

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #2

- Triple: `(қыркүйек — is_a — қазан)`
- Rule: `R1_is_a_transitivity`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0009178, wikipedia_kz_pack.json/wiki_kz_0009181

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #6

- Triple: `(жыл — is_a — халық)`
- Rule: `R1_is_a_transitivity`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0046261, wikipedia_kz_pack.json/wiki_kz_0127358

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #18

- Triple: `(-қа — is_a — президенті)`
- Rule: `R1_is_a_transitivity`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0103523, wikipedia_kz_pack.json/wiki_kz_0047830

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #21

- Triple: `(-қа — is_a — іс)`
- Rule: `R1_is_a_transitivity`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0103523, wikipedia_kz_pack.json/wiki_kz_0047829

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #27

- Triple: `(өнімдері — is_a — мәдени)`
- Rule: `R1_is_a_transitivity`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0117263, wikipedia_kz_pack.json/wiki_kz_0100463

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #40

- Triple: `(үндістан — has — ішек)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0004297, wikipedia_kz_pack.json/wiki_kz_0056731

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #45

- Triple: `(халқы — has — көб)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0012411, wikipedia_kz_pack.json/wiki_kz_0130787

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #46

- Triple: `(халқы — has — сан)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0012411, wikipedia_kz_pack.json/wiki_kz_0087483

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #47

- Triple: `(халқы — has — іш)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0012411, kazakh_textbooks_pack.json/kz_textbook_kz_lang_11_emn_p0009_s08

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #50

- Triple: `(ауғанстан — has — ішк)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0026052, wikipedia_kz_pack.json/wiki_kz_0053179

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #56

- Triple: `(ауғанстан — has — сыртқ)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0026147, wikipedia_kz_pack.json/wiki_kz_0027915

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #71

- Triple: `(ұлыбритания — has — ішк)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0027741, kazakh_textbooks_pack.json/kz_textbook_kz_lang_11_ogn_p0143_s13

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #73

- Triple: `(жыл — has — пікір)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0040759, wikipedia_kz_pack.json/wiki_kz_0043828

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #74

- Triple: `(темір — has — тармақ)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0043190, wikipedia_kz_pack.json/wiki_kz_0007661

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #78

- Triple: `(түндік — has — ұл)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0056466, wikipedia_kz_pack.json/wiki_kz_0075877

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #82

- Triple: `(ана — has — сан)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0069231, wikipedia_kz_pack.json/wiki_kz_0087483

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #84

- Triple: `(ана — has — қайс)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0069231, wikipedia_kz_pack.json/wiki_kz_0018686

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #85

- Triple: `(ана — has — ұрпақ)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0069231, wikipedia_kz_pack.json/wiki_kz_0132475

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #88

- Triple: `(сәтбаев — has — атау)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0098675, wikipedia_kz_pack.json/wiki_kz_0118247

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #91

- Triple: `(-қа — has — желтоқсан)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0103523, wikipedia_kz_pack.json/wiki_kz_0075406

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #101

- Triple: `(арыс — has — атау)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0110255, wikipedia_kz_pack.json/wiki_kz_0118247

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #105

- Triple: `(дуадақ — has — тырнақ)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0130217, wikipedia_kz_pack.json/wiki_kz_0024867

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #115

- Triple: `(ит — has — шаңырақ)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0142270, wikipedia_kz_pack.json/wiki_kz_0003387

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #126

- Triple: `(абай — related_to — қызылжар)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0007148, wikipedia_kz_pack.json/wiki_kz_0047327

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #127

- Triple: `(қазақстан — related_to — қызылжар)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0135507, wikipedia_kz_pack.json/wiki_kz_0047327

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #128

- Triple: `(ауғанстан — related_to — үндістан)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0026147, wikipedia_kz_pack.json/wiki_kz_0004297

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #136

- Triple: `(абай — related_to — қазақ)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0007158, wikipedia_kz_pack.json/wiki_kz_0001219

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #139

- Triple: `(арыс — related_to — талғар)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0110255, wikipedia_kz_pack.json/wiki_kz_0110712

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #140

- Triple: `(арыс — related_to — қазақ)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0110255, wikipedia_kz_pack.json/wiki_kz_0001219

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #143

- Triple: `(ақмешіт — related_to — қазақ)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0055603, wikipedia_kz_pack.json/wiki_kz_0001219

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #147

- Triple: `(темір — related_to — томдық)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0043190, wikipedia_kz_pack.json/wiki_kz_0003061

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #150

- Triple: `(ақпан — related_to — қаңтар)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0001080, wikipedia_kz_pack.json/wiki_kz_0001075

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #152

- Triple: `(қыркүйек — is_a — ақпан)`
- Rule: `R1_is_a_transitivity`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0009178, wikipedia_kz_pack.json/wiki_kz_0009181

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #156

- Triple: `(қыркүйек — is_a — өкіметі)`
- Rule: `R1_is_a_transitivity`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0009178, wikipedia_kz_pack.json/wiki_kz_0009182

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #159

- Triple: `(жыл — has — арасындағ)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0046286, wikipedia_kz_pack.json/wiki_kz_0027741

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #161

- Triple: `(жыл — has — көгалдандыру)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0046286, wikipedia_kz_pack.json/wiki_kz_0027741

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #165

- Triple: `(жыл — has — түбег)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0046286, wikipedia_kz_pack.json/wiki_kz_0027741

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #170

- Triple: `(-қа — has — пікір)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0103523, wikipedia_kz_pack.json/wiki_kz_0040759

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #176

- Triple: `(-қа — has — сыртқ)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0103523, wikipedia_kz_pack.json/wiki_kz_0027741

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #181

- Triple: `(-қа — has — ішк)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0103523, wikipedia_kz_pack.json/wiki_kz_0027741

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #191

- Triple: `(жая — related_to — күлше)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0058346, wikipedia_kz_pack.json/wiki_kz_0062890

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #193

- Triple: `(абай — related_to — алматы)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0088327, wikipedia_kz_pack.json/wiki_kz_0127358

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #196

- Triple: `(желтоқсан — related_to — қазан)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0001072, wikipedia_kz_pack.json/wiki_kz_0009182

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #197

- Triple: `(қазан — related_to — қаңтар)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0009182, wikipedia_kz_pack.json/wiki_kz_0001075

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #198

- Triple: `(-қа — related_to — ауғанстан)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0103523, wikipedia_kz_pack.json/wiki_kz_0026147

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #200

- Triple: `(-қа — related_to — ұлыбритания)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0103523, wikipedia_kz_pack.json/wiki_kz_0027741

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #201

- Triple: `(-қа — related_to — алматы)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0103523, wikipedia_kz_pack.json/wiki_kz_0127358

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #203

- Triple: `(желтоқсан — related_to — сәуір)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0001072, wikipedia_kz_pack.json/wiki_kz_0009181

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #205

- Triple: `(сәуір — related_to — қаңтар)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0009181, wikipedia_kz_pack.json/wiki_kz_0001075

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

---

## Tally

Fill in after review. `N` = number of items you ended up reviewing; `C` = number you marked correct.

- Facts: C = __ / N = 50 (precision = ___%)
- Derivations (semantic): C = __ / N = 50 (precision = ___%)
- Derivations (both underlying facts): C = __ / N = 50 (precision = ___%)

## By-pattern + by-rule summary

Sampled facts by pattern:

- `X Y-дан кейін`: 1
- `X Y-ні ал-лайды`: 1
- `X Y-ні асыр-лайды`: 2
- `X Y-ні бекіт-лайды`: 1
- `X Y-ні бол-лайды`: 4
- `X Y-ні біл-лайды`: 1
- `X Y-ні бұл-лайды`: 1
- `X Y-ні бөл-лайды`: 1
- `X Y-ні ене-лайды`: 1
- `X Y-ні жалғас-лайды`: 1
- `X Y-ні жақында-лайды`: 1
- `X Y-ні жете-лайды`: 1
- `X Y-ні жүс-лайды`: 1
- `X Y-ні кеңей-лайды`: 1
- `X Y-ні кір-лайды`: 1
- `X Y-ні сіңір-лайды`: 1
- `X Y-ні сүр-лайды`: 1
- `X Y-ні тона-лайды`: 1
- `X Y-ні туғыз-лайды`: 1
- `X Y-ні тік-лайды`: 1
- `X Y-ні тұр-лайды`: 1
- `X Y-ні төмендет-лайды`: 1
- `X Y-ні шоғырлан-лайды`: 1
- `X Y-ні іл-лайды`: 1
- `X Y-ні қаба-лайды`: 1
- `X Y-ні қайта-лайды`: 2
- `X Y-ні қара-лайды`: 1
- `X Y-ні қарқ-лайды`: 1
- `X Y-ні қат-лайды`: 1
- `X Y-ні құр-лайды`: 1
- `X Y-ні құтыл-лайды`: 1
- `X Y-ні үйрет-лайды`: 1
- `X Y-ні ұнат-лайды`: 1
- `X Y-ні ұсын-лайды`: 1
- `X Y-ні өсір-лайды`: 1
- `X пен Y`: 7
- `X — Y`: 1
- `X-тың Y-сы бар`: 2

Sampled derivations by rule:

- `R1_is_a_transitivity`: 8
- `R2_has_inheritance`: 24
- `R5_shared_is_a_target`: 18
