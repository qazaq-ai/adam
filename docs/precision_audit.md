# Precision audit — native-speaker review

**Target:** 50-fact sample + 50-derivation sample from the committed artifacts, seed `42`.

- `facts.json`: 13627 facts total (upstream status: `completed`) — sampled 50 here.
- `derived_facts.json`: 704 derivations total (upstream status: `completed`) — sampled 50 here.

## How to review

For each fact, mark the checkbox if the triple `(subject, predicate, object)` is **correct**: the sentence genuinely asserts that the subject has the claimed relation to the object, and both root resolutions are correct. When unsure, leave unchecked and add a one-line note in the Comments row. Update the **Tally** section at the bottom with your counts. Precision is defined as `correct / reviewed`.

---

## Fact sample

### Fact #1218

- Triple: `(жарық — does_to — тәжірибе)`
- Predicate: `does_to`
- Pattern: `X Y-ні сипатта-лайды`
- Source: `kazakh_textbooks_pack.json / kz_textbook_physics_11_ogn_p0081_s13`
- Sentence:

    > Ол жарық толқынының ұзындығын анықтауға арналған тәжірибелерді бірінші болып сипаттады

- [ ] Correct
- Comment:

### Fact #1366

- Triple: `(жер — does_to — ас)`
- Predicate: `does_to`
- Pattern: `X Y-ні қалыптас-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0000722`
- Sentence:

    > Жер асты суларының ағымы, айтарлықтай мөлшерде орын, өзендерді қалыптастырады, ағымдар, олардағы судың мөлдірлігі үшін «кара-су» деп аталады.

- [ ] Correct
- Comment:

### Fact #1440

- Triple: `(уездік — does_to — сауатсыздық)`
- Predicate: `does_to`
- Pattern: `X Y-ні қамты-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0002048`
- Sentence:

    > Уездік халық ағарту бөлімі жанынан сауатсыздықты жою жөніндегі комиссия құрылып 24 мектеп ашылып, 600 адамды қамтыған.

- [ ] Correct
- Comment:

### Fact #1770

- Triple: `(егер — does_to — ат)`
- Predicate: `does_to`
- Pattern: `X Y-ні ал-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0007583`
- Sentence:

    > Егер бірдей атты бөлімдер бірнешеу болса, сілтемеге реттік санын қосу арқылы, керектісіне апарта аласыз.

- [ ] Correct
- Comment:

### Fact #1799

- Triple: `(бұұ — related_to — нато)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0007836`
- Sentence:

    > Түркия Корея соғысынан кейін БҰҰ мен НАТО жанындағы халықаралық миссияларда, соның ішінде Сомали, Югославия мен Африка Мүйізіндегі бітімгершілік миссияларында күштерін сақтап келеді.

- [ ] Correct
- Comment:

### Fact #1806

- Triple: `(ішк — goes_to — көші-қон)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0007875`
- Sentence:

    > Сонымен қатар, ішкі көші-қонға байланысты күрдтердің диаспоралық қауымдастықтары Түркияның орталық және батысындағы барлық ірі қалаларда бар.

- [ ] Correct
- Comment:

### Fact #2280

- Triple: `(қыркүйек — related_to — желтоқсан)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0016106`
- Sentence:

    > Бас Ассамблея отырысы жыл сайын қыркүйек пен желтоқсан айлары арасында өтеді.

- [ ] Correct
- Comment:

### Fact #2405

- Triple: `(қазақ — does_to — тиім)`
- Predicate: `does_to`
- Pattern: `X Y-ні ал-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0017598`
- Sentence:

    > Абылай бастаған қазақ сұлтандары бұл таласты тиімді пайдаланып, жоңғарларды әлсірету арқылы қазақтың оңтүстік және шығыс жерлерінен азат етіп алған.

- [ ] Correct
- Comment:

### Fact #2414

- Triple: `(ресей — related_to — қытай)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0017646`
- Sentence:

    > Абылай Ресей мен Қытай империяларының арасында орналасқан Қазақ елінің геосаяси жағдайына икемделген саясат жүргізді.

- [ ] Correct
- Comment:

### Fact #2767

- Triple: `(билеуші — goes_to — азия)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0022405`
- Sentence:

    > Ахемен әулетінен шыққан, «төрт құбыланың тұтас билеушісі» атанған парсылардың Кир патшасы Орта Азияға басқыншылық жорықпен келген, «жеңілуді білмейтін» деп дәріптелген «өлместер» әскерін ашық шайқаста тас-талқанын шығарып жеңуімен тікелей байланысты.

- [ ] Correct
- Comment:

### Fact #2887

- Triple: `(қазақ — has — әйгіл)`
- Predicate: `has`
- Pattern: `X-тың Y-сы бар`
- Source: `wikipedia_kz_pack.json / wiki_kz_0024336`
- Sentence:

    > Оның жанында қазақтың әйгілі тарихшысы Қадырғали Жалайыр бар еді.

- [ ] Correct
- Comment:

### Fact #3020

- Triple: `(температуралық — goes_to — аймақ)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0026515`
- Sentence:

    > Жапония төрт мезгілі бар температуралық аймаққа жатады, бірақ оның климаты солтүстіктегі төмен температурадан оңтүстікте субтропиктікке дейін созылады.

- [ ] Correct
- Comment:

### Fact #3231

- Triple: `(әскери — does_to — үкімет)`
- Predicate: `does_to`
- Pattern: `X Y-ні жібер-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0030161`
- Sentence:

    > Әскери үкіметті таң қалдырып, Ұлыбритания Оңтүстік Атлантикаға әскерлерін жіберді және үш айдан кейін аралдардағы аргентиналық контингент тапсырылды.

- [ ] Correct
- Comment:

### Fact #3421

- Triple: `(мұнай — related_to — табиғи)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0033750`
- Sentence:

    > Жағалауларынан мұнай мен табиғи газ орындары барланған.

- [ ] Correct
- Comment:

### Fact #3441

- Triple: `(ән — related_to — би)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0034073`
- Sentence:

    > Мексикалықтарда ән мен би өнері жақсы дамыған.

- [ ] Correct
- Comment:

### Fact #3449

- Triple: `(өмір — does_to — сүру)`
- Predicate: `does_to`
- Pattern: `X Y-ні жалғас-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0034297`
- Sentence:

    > Иллириялықтар славяндармен ассимиляцияланды немесе таулы аймақтарға қоныс аударды, олар влахтар атымен өмір сүруді жалғастырды.

- [ ] Correct
- Comment:

### Fact #3764

- Triple: `(жоба — does_to — адам)`
- Predicate: `does_to`
- Pattern: `X Y-ні тыңда-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0039274`
- Sentence:

    > Жоба 16 қаладағы тыңдаудан: Астана, Тараз, Ақтау, Семей, Павлодар, Атырау, Шымкент, Қызылорда, Талдықорған, Қарағанды, Көкшетау, Ақтөбе және Алматы, нәтижесінде жюрилер 5563 адамды тыңдады.

- [ ] Correct
- Comment:

### Fact #3961

- Triple: `(британдық — does_to — үн)`
- Predicate: `does_to`
- Pattern: `X Y-ні таба-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0042260`
- Sentence:

    > Британдық биліктің мұрасы елдің саяси әкімшілігінде және үнді, африкалық, жергілікті американдық және көп ұлтты топтарды қамтитын сан алуан халықтардан көрініс табады.

- [ ] Correct
- Comment:

### Fact #4348

- Triple: `(дүниежүзілік — lives_in — аумағын)`
- Predicate: `lives_in`
- Pattern: `X Y-да тұрады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0048621`
- Sentence:

    > Бірінші дүниежүзілік соғыс басталған кезде Ленин Аустрия-Мажарстан аумағында Галисиядағы Поронин қаласында тұрды, ол 1912 жылдың аяғында келді.

- [ ] Correct
- Comment:

### Fact #4833

- Triple: `(яғни — does_to — материал)`
- Predicate: `does_to`
- Pattern: `X Y-ні кез-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0056249`
- Sentence:

    > Яғни, библиограф кітап, журнал, газет ішіндегі материалдарды кез келген басқа да хабарлама ресурстарын жазады және нақты жауап табуға, әдебиеттер тізімін, оқырман сұранысы бойынша, дәстүрлі емес тасушы хабарламасын табуға көмек теседі.

- [ ] Correct
- Comment:

### Fact #4880

- Triple: `(жұмыс — has — ұйытқы)`
- Predicate: `has`
- Pattern: `X-тың Y-сы бар`
- Source: `wikipedia_kz_pack.json / wiki_kz_0056991`
- Sentence:

    > Бүгінде Қазақстанда осы жұмыстардың ұйытқысы болып отырған танымал бірнеше азаматтар бар.

- [ ] Correct
- Comment:

### Fact #5009

- Triple: `(тағы — does_to — шөгін)`
- Predicate: `does_to`
- Pattern: `X Y-ні жайға-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0059166`
- Sentence:

    > Ташкенттің маңында Шыршық өзені тағы бірнеше өзендермен қосылады, сондықтан қаланың өзі қалың (15 метрге дейін) аллювийлі шөгінді жыныстардың үстінде жайғасқан.

- [ ] Correct
- Comment:

### Fact #5563

- Triple: `(өзара — does_to — қарым-қатынас)`
- Predicate: `does_to`
- Pattern: `X Y-ні біл-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0067773`
- Sentence:

    > Мемлекеттің және оның билігіне бағынатын адамның арасындағы өзара қарым-қатынасты білдіреді: мемлекет өз азаматының заңды құқылары мен мүдделерін қамтамасыз етуге, қорғауға және оған шет елде қамқорлық жасауға кепілдік береді; ал азамат мемлекеттің заңдарын және т.

- [ ] Correct
- Comment:

### Fact #5812

- Triple: `(бүкіл — does_to — үндістан)`
- Predicate: `does_to`
- Pattern: `X Y-ні шақыр-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0072308`
- Sentence:

    > Ол бүкіл Үндістанды жаяу аралап, будда, христиан, мұсылман дін орындарында болып, ешкімді ұлтына, дініне, әдет-ғұрпына алаламай ортақ күреске шақырады.

- [ ] Correct
- Comment:

### Fact #5825

- Triple: `(тым — does_to — кешірім)`
- Predicate: `does_to`
- Pattern: `X Y-ні көр-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0072476`
- Sentence:

    > Сен тым кешірімдісің, кешіруді жақсы көресің, мені де кешіре гөр!

- [ ] Correct
- Comment:

### Fact #5850

- Triple: `(қазан — does_to — атау)`
- Predicate: `does_to`
- Pattern: `X Y-ні шық-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0072908`
- Sentence:

    > 1993 жылғы 7 қазан шешім бойынша, орыс тіліндегі атауларды транскрипциялау туралы ҚР Жоғарғы Кеңесі Президиумының қаулысы шықты: ұлттық топонимиканы жаңғырту мақсатында қала атауы орыс тілінде Кокчетавтан Көкшетауға болып өзгертілді.

- [ ] Correct
- Comment:

### Fact #5883

- Triple: `(ереуіл — related_to — тәртіпсіздік)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0073338`
- Sentence:

    > Ереуілдер мен тәртіпсіздіктер салданған қалаларда азық-түлік тапшылығы сезіліп, тәртіпсіздік қаупі төнген жағдайда Черчилль 50 мың сарбазды жұмылдырады және армияны тек жергілікті азаматтық биліктің өтініші бойынша енгізуге болатын ережені жойды.

- [ ] Correct
- Comment:

### Fact #6089

- Triple: `(бөлігі — does_to — ел)`
- Predicate: `does_to`
- Pattern: `X Y-ні шоғырлан-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0076982`
- Sentence:

    > Халқының басым бөлігі Ойыл және Сағыз өзендеріне жақын елді мекендерге шоғырланған.

- [ ] Correct
- Comment:

### Fact #6621

- Triple: `(тері — has — қабат)`
- Predicate: `has`
- Pattern: `X-тың Y-сы бар`
- Source: `wikipedia_kz_pack.json / wiki_kz_0086270`
- Sentence:

    > Гиалурон қышқылы біздің теріміздің қабатында бар қосылыс.

- [ ] Correct
- Comment:

### Fact #6837

- Triple: `(мөлдір — does_to — бұйым)`
- Predicate: `does_to`
- Pattern: `X Y-ні қанық-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0090219`
- Sentence:

    > Мөлдір емес эмаль бояу­лы бұйымды отқа қойғаннан кей­ін керемет қанық түске ие бола­тын көрінеді.

- [ ] Correct
- Comment:

### Fact #7105

- Triple: `(шығанақ — does_to — мұхит)`
- Predicate: `does_to`
- Pattern: `X Y-ні ата-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0094801`
- Sentence:

    > Олар мұхитқа оңтүстіктегі ашық шығанақ арқылы шыққандықтан, Бальбоа бұл мұхитты Оңтүстік теңіз () деп атады.

- [ ] Correct
- Comment:

### Fact #7583

- Triple: `(жол — related_to — әлеуметтік)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0102950`
- Sentence:

    > Бұл қаражат жолдар мен әлеуметтік инфрақұрылымды дамытуға, шағын және орта бизнестің бәсекеге қабілеттілігін арттыруға, білім беруге, денсаулық сақтауға, қоршаған ортаны қорғауға және т.

- [ ] Correct
- Comment:

### Fact #8239

- Triple: `(сүңгі — related_to — садақ)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0114185`
- Sentence:

    > Мыс қоры бар жерлерде кен қазылып, қасында қола балқытылып, одан балта, орақ, түрлі әшекейлер, сүңгі мен садақ ұштары жасалды.

- [ ] Correct
- Comment:

### Fact #8482

- Triple: `(сібір — does_to — бөлу)`
- Predicate: `does_to`
- Pattern: `X Y-ні ұсын-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0118158`
- Sentence:

    > Генерал-губернаторлықты құру туралы шешім 1822 жылы 26 қаңтарда (7 ақпанда) Сібір генерал-губернаторлығын батыс және шығыс бөліктерге бөлуді ұсынған М.

- [ ] Correct
- Comment:

### Fact #8496

- Triple: `(жайық — does_to — ағын)`
- Predicate: `does_to`
- Pattern: `X Y-ні етпе-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0118375`
- Sentence:

    > Жайық бойында орналасқан үлкен қалалардың арналы тазалық құралғылары ескірген, қайта жаңғыртуды қажет етеді және арналық ағындарды қажетті дәрежеде тазартуды қамтамасыз етпейді.

- [ ] Correct
- Comment:

### Fact #9075

- Triple: `(кездесетін — does_to — сан)`
- Predicate: `does_to`
- Pattern: `X Y-ні жете-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0130263`
- Sentence:

    > Үлкен құралай () - сирек кездесетін, аз санды, жете зерттелмеген құс.

- [ ] Correct
- Comment:

### Fact #9130

- Triple: `(көбінесе — does_to — жолақ)`
- Predicate: `does_to`
- Pattern: `X Y-ні кең-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0131557`
- Sentence:

    > Ұлы жүз қазақтарының шапандары көбінесе жолақты, сырмалы, етек-жеңдері ұзын, әшекейлі келсе,Орта жүз тұрғындарының шапандары көбінесе бір беткей матадан, сырусыз, сирек қабылып, етектері шалғайлы, жеңдері кең, жағалары шолақ оймалы немесе түймелі болған.

- [ ] Correct
- Comment:

### Fact #9329

- Triple: `(өнер — related_to — білім)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0134931`
- Sentence:

    > «Классика» камералық ансамблі 2006 жылдың сәуірінде Мәскеу қаласында өткен «Қазіргі заманғы өнер мен білім беру» атты Халықаралық конкурс-фестивалінің дипломанты атағын иеленді.

- [ ] Correct
- Comment:

### Fact #9850

- Triple: `(жүрек — related_to — арқа)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0143516`
- Sentence:

    > Ол жүрек пен арқа қолқасынан құралған.

- [ ] Correct
- Comment:

### Fact #9870

- Triple: `(халық — does_to — өзек)`
- Predicate: `does_to`
- Pattern: `X Y-ні айт-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0143941`
- Sentence:

    > Қазақтың халық тілі мен әдеби тілінің өзекті мөселелерін зерттеп, соны пікірлер айтты.

- [ ] Correct
- Comment:

### Fact #9952

- Triple: `(салу — does_to — тиім)`
- Predicate: `does_to`
- Pattern: `X Y-ні тап-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0145671`
- Sentence:

    > Ол азықты сүрлемге салу кезіндегі термиялық, биохимиялық және микробиологиялық процестердің өзара байланысын, тиімді параметрлерін тапты.

- [ ] Correct
- Comment:

### Fact #10326

- Triple: `(қабанбай — does_to — уәлиханов)`
- Predicate: `does_to`
- Pattern: `X Y-ні үйлес-лайды`
- Source: `synthetic_sentences_pack.json / synth_00892`
- Sentence:

    > қабанбай арктикалыққа негізгі уәлихановды үйлестірді.

- [ ] Correct
- Comment:

### Fact #10999

- Triple: `(өріс — does_to — ағы)`
- Predicate: `does_to`
- Pattern: `X Y-ні құтқар-лайды`
- Source: `synthetic_sentences_pack.json / synth_04850`
- Sentence:

    > өріс өсунің ағымды құтқарады.

- [ ] Correct
- Comment:

### Fact #11863

- Triple: `(қауіпсіздік — does_to — тәуекел)`
- Predicate: `does_to`
- Pattern: `X Y-ні сез-лайды`
- Source: `synthetic_sentences_pack.json / synth_09869`
- Sentence:

    > қауіпсіздік тәуекелді сезді бірақ өнеркәсіптік ететінді ілейді.

- [ ] Correct
- Comment:

### Fact #12286

- Triple: `(қарсаңын — does_to — соғыс)`
- Predicate: `does_to`
- Pattern: `X Y-ні өлтір-лайды`
- Source: `synthetic_sentences_pack.json / synth_12473`
- Sentence:

    > қарсаңын бөлігінге сенімді соғысты өлтірді.

- [ ] Correct
- Comment:

### Fact #12414

- Triple: `(сібір — does_to — шығыс)`
- Predicate: `does_to`
- Pattern: `X Y-ні сала-лайды`
- Source: `synthetic_sentences_pack.json / synth_13250`
- Sentence:

    > сібір жинағден шығысты салады.

- [ ] Correct
- Comment:

### Fact #12858

- Triple: `(кеақ — does_to — шағын)`
- Predicate: `does_to`
- Pattern: `X Y-ні туыс-лайды`
- Source: `synthetic_sentences_pack.json / synth_15790`
- Sentence:

    > суық кеақ шағынды туысты бірақ жуан шақырды.

- [ ] Correct
- Comment:

### Fact #12903

- Triple: `(өзгеру — does_to — таңертең)`
- Predicate: `does_to`
- Pattern: `X Y-ні түсір-лайды`
- Source: `synthetic_sentences_pack.json / synth_16017`
- Sentence:

    > сенімді өзгеру таңертеңді түсірді.

- [ ] Correct
- Comment:

### Fact #13095

- Triple: `(еңбек — does_to — бұқар)`
- Predicate: `does_to`
- Pattern: `X Y-ні қайта-лайды`
- Source: `synthetic_sentences_pack.json / synth_17150`
- Sentence:

    > еңбек аумағды көбейді және айла бұқарды қайтайды.

- [ ] Correct
- Comment:

### Fact #13098

- Triple: `(аграрлық — does_to — атындағ)`
- Predicate: `does_to`
- Pattern: `X Y-ні жек-лайды`
- Source: `synthetic_sentences_pack.json / synth_17164`
- Sentence:

    > сенімді аграрлық атындағды жекті.

- [ ] Correct
- Comment:

---

## Derivation sample

### Derivation #2

- Triple: `(сәуір — is_a — қала)`
- Rule: `R1_is_a_transitivity`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0003423, world_core/geography_kz.jsonl/geo_kz_009

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #40

- Triple: `(үндістан — has — көбі)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0004297, wikipedia_kz_pack.json/wiki_kz_0138207

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #50

- Triple: `(абай — has — ен)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0007158, wikipedia_kz_pack.json/wiki_kz_0073463

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #56

- Triple: `(халқы — has — ұрпақ)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0012411, wikipedia_kz_pack.json/wiki_kz_0132475

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #74

- Triple: `(ұлыбритания — has — таул)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0027741, wikipedia_kz_pack.json/wiki_kz_0030414

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #101

- Triple: `(неміс — has — іш)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0109606, kazakh_textbooks_pack.json/kz_textbook_kz_lang_11_emn_p0009_s08

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #105

- Triple: `(арыс — has — ен)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0110255, wikipedia_kz_pack.json/wiki_kz_0073463

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #126

- Triple: `(шолпан — has — серік)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: world_core/astronomy.jsonl/astro_005, wikipedia_kz_pack.json/wiki_kz_0061386

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #143

- Triple: `(астана — has — атау)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: world_core/geography_kz.jsonl/geo_kz_003, wikipedia_kz_pack.json/wiki_kz_0118247

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #147

- Triple: `(шымкент — has — атау)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: world_core/geography_kz.jsonl/geo_kz_005, wikipedia_kz_pack.json/wiki_kz_0118247

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #165

- Triple: `(ақтау — has — атау)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: world_core/geography_kz.jsonl/geo_kz_014, wikipedia_kz_pack.json/wiki_kz_0118247

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #170

- Triple: `(талдықорған — has — ен)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: world_core/geography_kz.jsonl/geo_kz_016, wikipedia_kz_pack.json/wiki_kz_0073463

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #191

- Triple: `(абай — related_to — қызылжар)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0007148, wikipedia_kz_pack.json/wiki_kz_0047327

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #196

- Triple: `(қазақстан — related_to — ұлыбритания)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/geography_kz.jsonl/geo_kz_002, wikipedia_kz_pack.json/wiki_kz_0027741

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #197

- Triple: `(үндістан — related_to — ұлыбритания)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0004297, wikipedia_kz_pack.json/wiki_kz_0027741

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #205

- Triple: `(ауғанстан — related_to — өзбекстан)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0026052, world_core/geography_kz.jsonl/geo_kz_030

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #236

- Triple: `(жер — related_to — марс)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/astronomy.jsonl/astro_001, world_core/astronomy.jsonl/astro_004

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #251

- Triple: `(меркурий — related_to — уран)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/astronomy.jsonl/astro_006, wikipedia_kz_pack.json/wiki_kz_0062376

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #255

- Triple: `(нептун — related_to — уран)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/astronomy.jsonl/astro_010, wikipedia_kz_pack.json/wiki_kz_0062376

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #263

- Triple: `(шолпан — related_to — юпитер)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/astronomy.jsonl/astro_005, world_core/astronomy.jsonl/astro_007

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #264

- Triple: `(абай — related_to — алматы)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0007158, world_core/geography_kz.jsonl/geo_kz_004

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #266

- Triple: `(абай — related_to — астана)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0007158, world_core/geography_kz.jsonl/geo_kz_003

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #268

- Triple: `(абай — related_to — ақмешіт)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0007158, wikipedia_kz_pack.json/wiki_kz_0055603

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #272

- Triple: `(абай — related_to — орал)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0007158, world_core/geography_kz.jsonl/geo_kz_018

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #332

- Triple: `(астана — related_to — шымкент)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/geography_kz.jsonl/geo_kz_003, world_core/geography_kz.jsonl/geo_kz_005

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #333

- Triple: `(астана — related_to — қазақ)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/geography_kz.jsonl/geo_kz_003, wikipedia_kz_pack.json/wiki_kz_0001219

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #341

- Triple: `(атырау — related_to — көкшетау)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/geography_kz.jsonl/geo_kz_008, world_core/geography_kz.jsonl/geo_kz_017

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #342

- Triple: `(атырау — related_to — орал)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/geography_kz.jsonl/geo_kz_008, world_core/geography_kz.jsonl/geo_kz_018

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #369

- Triple: `(ақмешіт — related_to — қызылорда)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0055603, world_core/geography_kz.jsonl/geo_kz_015

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #386

- Triple: `(ақтөбе — related_to — көкшетау)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/geography_kz.jsonl/geo_kz_007, world_core/geography_kz.jsonl/geo_kz_017

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #420

- Triple: `(орал — related_to — қостанай)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/geography_kz.jsonl/geo_kz_018, world_core/geography_kz.jsonl/geo_kz_013

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #461

- Triple: `(талғар — related_to — тараз)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0110712, world_core/geography_kz.jsonl/geo_kz_011

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #462

- Triple: `(талғар — related_to — шымкент)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0110712, world_core/geography_kz.jsonl/geo_kz_005

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #469

- Triple: `(тараз — related_to — қазақ)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/geography_kz.jsonl/geo_kz_011, wikipedia_kz_pack.json/wiki_kz_0001219

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #472

- Triple: `(тараз — related_to — қызылорда)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/geography_kz.jsonl/geo_kz_011, world_core/geography_kz.jsonl/geo_kz_015

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #490

- Triple: `(ертіс — related_to — сырдария)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/geography_kz.jsonl/geo_kz_019, world_core/geography_kz.jsonl/geo_kz_020

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #501

- Triple: `(сәуір — is_a — өкіметі)`
- Rule: `R1_is_a_transitivity`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0009181, wikipedia_kz_pack.json/wiki_kz_0009182

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #527

- Triple: `(қыркүйек — has — атау)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0009178, world_core/geography_kz.jsonl/geo_kz_009

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #528

- Triple: `(қыркүйек — has — ен)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0009178, world_core/geography_kz.jsonl/geo_kz_009

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #610

- Triple: `(жыл — related_to — тараз)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0046261, world_core/geography_kz.jsonl/geo_kz_011

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #617

- Triple: `(жыл — related_to — өскемен)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0046261, world_core/geography_kz.jsonl/geo_kz_012

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #636

- Triple: `(сәуір — related_to — өскемен)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0003423, world_core/geography_kz.jsonl/geo_kz_012

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #645

- Triple: `(қазан — related_to — өскемен)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0003416, world_core/geography_kz.jsonl/geo_kz_012

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #646

- Triple: `(ай — related_to — уақыт)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/time.jsonl/time_003, world_core/time.jsonl/time_020

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #663

- Triple: `(кезеңі — related_to — меркурий)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0083073, world_core/astronomy.jsonl/astro_006

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #665

- Triple: `(кезеңі — related_to — сатурн)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0083073, world_core/astronomy.jsonl/astro_008

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #669

- Triple: `(кезеңі — related_to — ғаламшар)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0083073, world_core/astronomy.jsonl/astro_012

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #685

- Triple: `(орал — related_to — қыркүйек)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/geography_kz.jsonl/geo_kz_018, wikipedia_kz_pack.json/wiki_kz_0009178

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #698

- Triple: `(кезеңі — related_to — уақыт)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0083073, world_core/time.jsonl/time_020

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #700

- Triple: `(желтоқсан — related_to — сәуір)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0001072, wikipedia_kz_pack.json/wiki_kz_0009181

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

- `X Y-да тұрады`: 1
- `X Y-ке барады`: 3
- `X Y-ні айт-лайды`: 1
- `X Y-ні ал-лайды`: 2
- `X Y-ні ата-лайды`: 1
- `X Y-ні біл-лайды`: 1
- `X Y-ні етпе-лайды`: 1
- `X Y-ні жайға-лайды`: 1
- `X Y-ні жалғас-лайды`: 1
- `X Y-ні жек-лайды`: 1
- `X Y-ні жете-лайды`: 1
- `X Y-ні жібер-лайды`: 1
- `X Y-ні кез-лайды`: 1
- `X Y-ні кең-лайды`: 1
- `X Y-ні көр-лайды`: 1
- `X Y-ні сала-лайды`: 1
- `X Y-ні сез-лайды`: 1
- `X Y-ні сипатта-лайды`: 1
- `X Y-ні таба-лайды`: 1
- `X Y-ні тап-лайды`: 1
- `X Y-ні туыс-лайды`: 1
- `X Y-ні тыңда-лайды`: 1
- `X Y-ні түсір-лайды`: 1
- `X Y-ні шақыр-лайды`: 1
- `X Y-ні шоғырлан-лайды`: 1
- `X Y-ні шық-лайды`: 1
- `X Y-ні қайта-лайды`: 1
- `X Y-ні қалыптас-лайды`: 1
- `X Y-ні қамты-лайды`: 1
- `X Y-ні қанық-лайды`: 1
- `X Y-ні құтқар-лайды`: 1
- `X Y-ні үйлес-лайды`: 1
- `X Y-ні ұсын-лайды`: 1
- `X Y-ні өлтір-лайды`: 1
- `X пен Y`: 10
- `X-тың Y-сы бар`: 3

Sampled derivations by rule:

- `R1_is_a_transitivity`: 2
- `R2_has_inheritance`: 13
- `R5_shared_is_a_target`: 35
