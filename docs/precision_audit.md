# Precision audit — native-speaker review

**Target:** 50-fact sample + 50-derivation sample from the committed artifacts, seed `42`.

- `facts.json`: 13887 facts total (upstream status: `completed`) — sampled 50 here.
- `derived_facts.json`: 7293 derivations total (upstream status: `completed`) — sampled 50 here.

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

- Triple: `(қала — lives_in — аумағын)`
- Predicate: `lives_in`
- Pattern: `X Y-да тұрады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0001253`
- Sentence:

    > Қала аумағында 113 ұлттың өкілдері тұрады.

- [ ] Correct
- Comment:

### Fact #1440

- Triple: `(маңғыстау — after — уақыт)`
- Predicate: `after`
- Pattern: `X Y-дан кейін`
- Source: `wikipedia_kz_pack.json / wiki_kz_0003084`
- Sentence:

    > 1928 жылы Адай уезі таратылып, Маңғыстау округі аз уақыттан соң Маңғыстау ауданы болып қайта құрылды.

- [ ] Correct
- Comment:

### Fact #1770

- Triple: `(бағдарлама — related_to — құжат)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0008064`
- Sentence:

    > Компьютерді ұйқылық күйінен шығарған кезде, ашық тұрған барлық бағдарламалар мен құжаттар жұмыс үстелінде қалпына келтіріледі.

- [ ] Correct
- Comment:

### Fact #1799

- Triple: `(аумақ — does_to — үн)`
- Predicate: `does_to`
- Pattern: `X Y-ні қалдыр-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0008657`
- Sentence:

    > Аумақ арктикалық бассейнге ашылған, бірақ Азияның ең биік тау шыңдары оны Үнді мұхитының әсерінен оқшау қалдырған.

- [ ] Correct
- Comment:

### Fact #1806

- Triple: `(байлық — does_to — күш)`
- Predicate: `does_to`
- Pattern: `X Y-ні біл-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0008926`
- Sentence:

    > Елтаңбаның түсі: сары – байлық пен күшті, көк – сұлулық пен ұлылықты,жасыл – табиғи байлықты білдіреді.

- [ ] Correct
- Comment:

### Fact #2280

- Triple: `(қосымша — does_to — жоба)`
- Predicate: `does_to`
- Pattern: `X Y-ні кір-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0016821`
- Sentence:

    > Содан барлық қосымша жобаларды жиып қойып, Елтаңбаның сызбасын сызуға кірістім.

- [ ] Correct
- Comment:

### Fact #2405

- Triple: `(шағатай — related_to — үгедей)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0018165`
- Sentence:

    > Сөйтіп Отырар қамалын жермен-жексен еткен Шағатай мен Үгедей бастапан әскер Шыңғыс ханға қосылды.

- [ ] Correct
- Comment:

### Fact #2414

- Triple: `(солтүстік — goes_to — қазақстан)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0018233`
- Sentence:

    > Батыс, солтүстік Қазақстанға олар келді.

- [ ] Correct
- Comment:

### Fact #2767

- Triple: `(қытай — does_to — әскер)`
- Predicate: `does_to`
- Pattern: `X Y-ні түсір-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0023142`
- Sentence:

    > Қытай әскерлері де үш бағытта шабуылдап, мыңдаған әскерді тұтқынға түсіреді.

- [ ] Correct
- Comment:

### Fact #2887

- Triple: `(қазақстан — goes_to — жыл)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0025063`
- Sentence:

    > Қазақстан Сенаты 50 мүшесінің 40 депутатын 17 облыс пен үш республикалық маңызы бар қаланың мәслихаттары алты жылға жанама түрде сайлайды.

- [ ] Correct
- Comment:

### Fact #3020

- Triple: `(түсу — goes_to — сағат)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0027583`
- Sentence:

    > 925 |} Шыңжаңда күн сәулесінің түсуі 3400 сағатқа шейін барады.

- [ ] Correct
- Comment:

### Fact #3231

- Triple: `(жібек — goes_to — айрық)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0031486`
- Sentence:

    > Ерте кездегі «Жібек жолы» Шыңжаңнан 3 айрыққа бөлінген, Тянь-Шань тауының солтүстігіндегі ежелден бар дала жолы: Жемсары, Іле өңірін басып, Балқаш өңіріне барады, онан ары батыс солтүстікке жүргенде Қара теңіздің шығыс жағалауына жетеді.

- [ ] Correct
- Comment:

### Fact #3421

- Triple: `(кук — does_to — жергілік)`
- Predicate: `does_to`
- Pattern: `X Y-ні ата-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0034917`
- Sentence:

    > Кук келіп, жергілікті халықтың жылы қабылдауын көргендіктен Қзгі ниет аралдары деп атады.

- [ ] Correct
- Comment:

### Fact #3441

- Triple: `(жауап — does_to — билік)`
- Predicate: `does_to`
- Pattern: `X Y-ні ал-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0035181`
- Sentence:

    > ” – деген сұраққа Конфуций: “Халықтың сүйіспеншілігіне қол жеткіз, – деп жауап берген, – сонда сен билікке жетесің, егер халықтың сүйіспеншілігін жоғалтар болсаң, онда билікті де жоғалтып аласың”.

- [ ] Correct
- Comment:

### Fact #3449

- Triple: `(мұн — goes_to — мәуереннаһр)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0035300`
- Sentence:

    > Мұны естіген Шайбани әулеті Мәуереннаһрға қайтып келеді.

- [ ] Correct
- Comment:

### Fact #3764

- Triple: `(жүрек — is_a — мүше)`
- Predicate: `is_a`
- Pattern: `X — Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0040376`
- Sentence:

    > Жүрек — іші қуыс бұлшықетті мүше.

- [ ] Correct
- Comment:

### Fact #3961

- Triple: `(ағаш — related_to — темір)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0043586`
- Sentence:

    > Қызыл ағаштар мен темір ағаштардың көптеген түрлері кездеседі.

- [ ] Correct
- Comment:

### Fact #4348

- Triple: `(мәселе — does_to — міндет)`
- Predicate: `does_to`
- Pattern: `X Y-ні ал-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0049876`
- Sentence:

    > Келесі мәселе оқу-ағарту саласы: «міндетті бастауыш оқу енгізу», «бастапқы екі жылда оқу баланың ана тілінде» жүргізілу керектігі айтылып, тіл мәселесін айрықша назарға алған және білім берудің тегін болуы талап етілген.

- [ ] Correct
- Comment:

### Fact #4833

- Triple: `(тұс — does_to — қасиет)`
- Predicate: `does_to`
- Pattern: `X Y-ні тау-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0057520`
- Sentence:

    > Бұл тұс және де Келес өңірі, қасиетті Қазығұрттың маңы, Қазығұртты айтқанда біз барлық оқиғаны сол бір таудың іргесіне апарып тіреп қоюдан аулақпыз – бүкіл Қазығұрт өңірін қоса айтып отырмыз.

- [ ] Correct
- Comment:

### Fact #4880

- Triple: `(жатқан — goes_to — бөлік)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0058413`
- Sentence:

    > жыл бұрын) құрамында қазіргі аумақты алып жатқан Қара және Каспий теңіздері бар Сармат тайпасы бірнеше бөлікке бөлінді.

- [ ] Correct
- Comment:

### Fact #5009

- Triple: `(чили — does_to — үн)`
- Predicate: `does_to`
- Pattern: `X Y-ні жата-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0060392`
- Sentence:

    > Оның бейорганикалық қосылыстарының негізгілеріне натрий нитраты (чили селитрі), калий нитраты (үнді селитрі) жатады.

- [ ] Correct
- Comment:

### Fact #5563

- Triple: `(төреші — does_to — ой)`
- Predicate: `does_to`
- Pattern: `X Y-ні бер-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0069849`
- Sentence:

    > Бақтиярдың айқын басымдығын көрген төрешілер ойынды 3-ші раунд-ақ тоқтатып, жеңісті Бақтиярға береді.

- [ ] Correct
- Comment:

### Fact #5812

- Triple: `(шаңырақ — does_to — бөлу)`
- Predicate: `does_to`
- Pattern: `X Y-ні өтіне-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0073718`
- Sentence:

    > 1850 жылғы 18 қаңтарда Мәмбет-Тобықты болысының старшындары мен билері Қарқаралы округтік приказына бұл болысты (16 старшындық; 1723 шаңырақ) екіге бөлуді, одан 8 старшындықты (825 шаңырақ) бөліп шығаруды өтіне келіп, олар былай деген: «.

- [ ] Correct
- Comment:

### Fact #5825

- Triple: `(қызылша — part_of — көкөніс)`
- Predicate: `part_of`
- Pattern: `X Y-нің құрамында`
- Source: `wikipedia_kz_pack.json / wiki_kz_0073878`
- Sentence:

    > Сахароза қанттың, кондитер өнімдерінің, тосаптың, балмұздақтың, тәтті шырындардың, сонымен қатар қызылша, шабдалы, сәбіз, тәтті қараөрік және тағы басқа жемістер мен көкөністердің құрамында кездеседі.

- [ ] Correct
- Comment:

### Fact #5850

- Triple: `(өзен — does_to — құм)`
- Predicate: `does_to`
- Pattern: `X Y-ні кет-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0074385`
- Sentence:

    > Қарашаған шығанағынан өзен атырауына дейінгі Оңтүстік жағалаулар немесе төмен (1-2 м) және құмды, мезгіл-мезгіл жоғары суға батып кетеді (соның салдарынан көптеген таяз көлдер бар), кейбір жерлерде биіктігі 5-10 м жағалаудағы төбелер кездеседі.

- [ ] Correct
- Comment:

### Fact #5883

- Triple: `(ғұсыл — does_to — иіс)`
- Predicate: `does_to`
- Pattern: `X Y-ні жолық-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0075315`
- Sentence:

    > Құрбан айт күні ертерек тұрып, ғұсыл алып, таза киім киіп, хош иісті әтір сеуіп, көшеде жолыққан адамға ашық жүзбен қарау, құрбандық етімен ауыз ашу үшін намаздан алдын ештеңе жемей, намазға бара жатқанда тәкбір айту – хазіреті Мұхаммед (с.

- [ ] Correct
- Comment:

### Fact #6089

- Triple: `(кофе — related_to — жержаңғақ)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0078856`
- Sentence:

    > Мысалы, Оңтүстік-Шығыс Азия елдері — күріш, Африка елдері — кофе мен жержаңғақ, ал Латын Америкасы елдері — қант құрағы мен какао өсіруге маманданған.

- [ ] Correct
- Comment:

### Fact #6621

- Triple: `(бүкіл — does_to — халық)`
- Predicate: `does_to`
- Pattern: `X Y-ні тап-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0088241`
- Sentence:

    > Бүкіл халықты тап, топқа жіктемей, Қазақ елін әлемдік мәдени жетістіктерге қол жеткізуге қандай күш кедергі деген сауал қойып, оған басты кедергі – отаршылдық деген шешімге келді.

- [ ] Correct
- Comment:

### Fact #6837

- Triple: `(бүл — does_to — пікір)`
- Predicate: `does_to`
- Pattern: `X Y-ні бұл-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0092227`
- Sentence:

    > Әрине, бүл пікірді бірден қабылдай қою қиын, алайда, әдебиетінің кәсіпкерлік жолға шығуын айтып отырса, онда бұл тұжырымға тарихи сабақтастықты ескеріп, ойлана қараудың артықтығы бола қоймас.

- [ ] Correct
- Comment:

### Fact #7105

- Triple: `(әсер — goes_to — күн)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0097155`
- Sentence:

    > Осы принциппен жұмыс істейтін іштен жанатын қозғалтқыштардың бу қозғалтқыштарына қарағанда пайдалы әсер ету коэффициентінің аса жоғары болғандығынан бүгінгі күнге дейін ең көп тараған қозғалтқыш түрі болып келеді.

- [ ] Correct
- Comment:

### Fact #7583

- Triple: `(түр — goes_to — дүние)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0105584`
- Sentence:

    > Осы кезеңде лирика, драма, прозаның көп түрі дүниеге келген.

- [ ] Correct
- Comment:

### Fact #8239

- Triple: `(әлеуметтік — does_to — жағдай)`
- Predicate: `does_to`
- Pattern: `X Y-ні сипатта-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0116461`
- Sentence:

    > Осы оқиғаға байланысты анасы әлеуметтік желілерге бейне жүктеп, жағдайды егжей-тегжейлі сипаттады, бұл кеңінен қоғамдық резонанс тудырды.

- [ ] Correct
- Comment:

### Fact #8482

- Triple: `(ақбақай — goes_to — дүние)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0121751`
- Sentence:

    > Қаратаев Бақытжан Бейсәліұлы 1860 жылы 10 мамырда Қаратөбе ауданы Ақбақай ауылында дүниеге келген.

- [ ] Correct
- Comment:

### Fact #8496

- Triple: `(бүкіл — does_to — жергілік)`
- Predicate: `does_to`
- Pattern: `X Y-ні тигіз-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0122036`
- Sentence:

    > Бүкіл акционерлік қоғам мен оның бөлімшелерінде жұмысшы, қызметкерлердің 60 пайыздан астамы жергілікті тұрғындарды құраса, мекеме өз кезегінде аудан бюджетінің көтерілуіне септігін тигізді.

- [ ] Correct
- Comment:

### Fact #9075

- Triple: `(әкімшілік — related_to — әлеуметтік)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0132964`
- Sentence:

    > Адлер әкімшілік пен әлеуметтік билікке талпыныс, ал К.

- [ ] Correct
- Comment:

### Fact #9130

- Triple: `(кәсіпорын — does_to — ескерткіш)`
- Predicate: `does_to`
- Pattern: `X Y-ні бер-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0134182`
- Sentence:

    > Акция кезінде кейбір кәсіпорындар мен мекемелер ескерткіштерді жөндеп, көгалдандыру жұмысына көмек берді.

- [ ] Correct
- Comment:

### Fact #9329

- Triple: `(ырым — does_to — уақыт)`
- Predicate: `does_to`
- Pattern: `X Y-ні қолдан-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0137388`
- Sentence:

    > Батырлардың қаруы өзіне арналып соғылып, түрлі магиялық ырымдар жасалып, кететін уақытты анықтауға киелі сандарды қолданған.

- [ ] Correct
- Comment:

### Fact #9850

- Triple: `(қозы — does_to — есі)`
- Predicate: `does_to`
- Pattern: `X Y-ні жаз-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0147656`
- Sentence:

    > Барданес Аягөздің бойында тастан қаланған Қозы Көрпеш есімді ғимараттың сақталғанын жазған.

- [ ] Correct
- Comment:

### Fact #9870

- Triple: `(туған — does_to — уыз)`
- Predicate: `does_to`
- Pattern: `X Y-ні кептір-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0148089`
- Sentence:

    > Шала туған қозының жұмыршағындағы уызды алып кептіреді.

- [ ] Correct
- Comment:

### Fact #9952

- Triple: `(қазал — goes_to — дүние)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0149453`
- Sentence:

    > Қырымбек Көшербаев Қазалы ауданында дүниеге келген, Сырдария ауданында өскен.

- [ ] Correct
- Comment:

### Fact #10326

- Triple: `(рим — does_to — моңғол)`
- Predicate: `does_to`
- Pattern: `X Y-ні арттыр-лайды`
- Source: `synthetic_sentences_pack.json / synth_02005`
- Sentence:

    > рим моңғолды арттырды сөйтіп өкілет ілмекті буынады.

- [ ] Correct
- Comment:

### Fact #10999

- Triple: `(жұқпал — does_to — шатқал)`
- Predicate: `does_to`
- Pattern: `X Y-ні күшейт-лайды`
- Source: `synthetic_sentences_pack.json / synth_06019`
- Sentence:

    > жұқпал көшенің шатқалды күшейтеді.

- [ ] Correct
- Comment:

### Fact #11863

- Triple: `(жекешелендір — does_to — шығын)`
- Predicate: `does_to`
- Pattern: `X Y-ні көш-лайды`
- Source: `synthetic_sentences_pack.json / synth_10983`
- Sentence:

    > әлдеқашан жекешелендір шығынды соғды сондықтан қысқа саябақ көшеді.

- [ ] Correct
- Comment:

### Fact #12286

- Triple: `(мәтін — does_to — как)`
- Predicate: `does_to`
- Pattern: `X Y-ні ұлғайт-лайды`
- Source: `synthetic_sentences_pack.json / synth_13686`
- Sentence:

    > мәтін какты ұлғайтты немесе баяу жарқырайды.

- [ ] Correct
- Comment:

### Fact #12414

- Triple: `(түгел — does_to — бойындағ)`
- Predicate: `does_to`
- Pattern: `X Y-ні бос-лайды`
- Source: `synthetic_sentences_pack.json / synth_14438`
- Sentence:

    > түгел аз бойындағды босты.

- [ ] Correct
- Comment:

### Fact #12858

- Triple: `(естелік — does_to — әскер)`
- Predicate: `does_to`
- Pattern: `X Y-ні тақ-лайды`
- Source: `synthetic_sentences_pack.json / synth_16946`
- Sentence:

    > нұрда естелік әскерді тақты.

- [ ] Correct
- Comment:

### Fact #12903

- Triple: `(илья — does_to — тарихындағ)`
- Predicate: `does_to`
- Pattern: `X Y-ні жарқыра-лайды`
- Source: `synthetic_sentences_pack.json / synth_17284`
- Sentence:

    > илья күштің тарихындағды жарқырайды.

- [ ] Correct
- Comment:

### Fact #13095

- Triple: `(қарқарал — does_to — бұлақ)`
- Predicate: `does_to`
- Pattern: `X Y-ні тұтас-лайды`
- Source: `synthetic_sentences_pack.json / synth_18394`
- Sentence:

    > жақсы қарқарал бұлақты тұтасты.

- [ ] Correct
- Comment:

### Fact #13098

- Triple: `(сағ — does_to — кездесетін)`
- Predicate: `does_to`
- Pattern: `X Y-ні туғыз-лайды`
- Source: `synthetic_sentences_pack.json / synth_18405`
- Sentence:

    > сағ жібектің кездесетінді туғызды сөйтіп мына туынады.

- [ ] Correct
- Comment:

---

## Derivation sample

### Derivation #40

- Triple: `(бұғы — is_a — тіршілік иесі)`
- Rule: `R1_is_a_transitivity`
- Confidence: `rule_inferred`
- Source chain: world_core/animals.jsonl/anm_008, world_core/biology_basic.jsonl/bio_012

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #143

- Triple: `(өрік — is_a — тағам)`
- Rule: `R1_is_a_transitivity`
- Confidence: `rule_inferred`
- Source chain: world_core/food.jsonl/food_019, world_core/food.jsonl/food_024

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #251

- Triple: `(таң — is_a — ұғым)`
- Rule: `R1_is_a_transitivity`
- Confidence: `rule_inferred`
- Source chain: world_core/time.jsonl/time_010, world_core/time.jsonl/time_020

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #272

- Triple: `(абай — has — ен)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0007158, wikipedia_kz_pack.json/wiki_kz_0073463

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #369

- Triple: `(қарға — has — іш)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: world_core/animals.jsonl/anm_011, wikipedia_kz_pack.json/wiki_kz_0002165

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #528

- Triple: `(табу — has — бас)`
- Rule: `R3_has_inheritance_via_part_of`
- Confidence: `rule_inferred`
- Source chain: kazakh_textbooks_pack.json/kz_textbook_informatics_11_emn_p0119_s10, world_core/body_parts.jsonl/body_003

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #821

- Triple: `(жәндік — related_to — мысық)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/animals.jsonl/anm_029, world_core/biology_basic.jsonl/bio_004

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #842

- Triple: `(киік — related_to — мысық)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/animals.jsonl/anm_009, world_core/biology_basic.jsonl/bio_004

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #858

- Triple: `(мал — related_to — қоян)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/animals.jsonl/anm_038, world_core/animals.jsonl/anm_006

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #930

- Triple: `(бояу — related_to — сусын)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/colors.jsonl/color_033, world_core/food.jsonl/food_049

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #939

- Triple: `(тұз — related_to — әтір)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/food.jsonl/food_017, wikipedia_kz_pack.json/wiki_kz_0102432

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #1843

- Triple: `(анатомия — related_to — орнитология)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/body_parts.jsonl/body_040, world_core/animals.jsonl/anm_040

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #1895

- Triple: `(абай — related_to — алматы)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0007158, world_core/geography_kz.jsonl/geo_kz_004

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #1940

- Triple: `(арыс — related_to — сәтбаев)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0110255, wikipedia_kz_pack.json/wiki_kz_0098675

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #2317

- Triple: `(жоғар — goes_to — ғасыр)`
- Rule: `R7_goes_to_via_part_of`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0025290, world_core/time.jsonl/time_005

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #2451

- Triple: `(ғибадатхана — goes_to — апта)`
- Rule: `R7_goes_to_via_part_of`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0136487, world_core/time.jsonl/time_002

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #2666

- Triple: `(әйгіл — after — соғыс)`
- Rule: `R8_after_transitivity`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0042866, kazakh_textbooks_pack.json/kz_textbook_kz_lang_culture_9_p0132_s14

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #2683

- Triple: `(революциялық — after — шеру)`
- Rule: `R8_after_transitivity`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0044156, wikipedia_kz_pack.json/wiki_kz_0043753

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #2727

- Triple: `(фон — after — наразылық)`
- Rule: `R8_after_transitivity`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0053163, wikipedia_kz_pack.json/wiki_kz_0006688

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #3205

- Triple: `(желтоқсан — has — ішк)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0141921, world_core/geography_kz.jsonl/geo_kz_028

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #3343

- Triple: `(ала — related_to — мейірім)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/colors.jsonl/color_016, world_core/proverbs.jsonl/prov_027

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #3380

- Triple: `(жасыл — related_to — түс)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/colors.jsonl/color_004, world_core/colors.jsonl/color_027

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #3653

- Triple: `(арыстан — related_to — торғай)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/animals.jsonl/anm_003, world_core/animals.jsonl/anm_020

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4018

- Triple: `(сауысқан — related_to — қоян)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/animals.jsonl/anm_012, world_core/animals.jsonl/anm_006

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4041

- Triple: `(сүтқоректі — related_to — үйрек)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/animals.jsonl/anm_036, world_core/animals.jsonl/anm_019

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4059

- Triple: `(тағы жануар — related_to — қасқыр)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/animals.jsonl/anm_037, wikipedia_kz_pack.json/wiki_kz_0041416

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4363

- Triple: `(жырау — related_to — мұхтар)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/kz_literature.jsonl/lit_033, world_core/kz_literature.jsonl/lit_008

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4373

- Triple: `(жыршы — related_to — мағжан)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/kz_literature.jsonl/lit_028, world_core/kz_literature.jsonl/lit_005

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4388

- Triple: `(заңгер — related_to — олжас)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/society.jsonl/soc_035, world_core/kz_literature.jsonl/lit_014

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4556

- Triple: `(мұхит — related_to — су)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/biology_basic.jsonl/bio_034, world_core/food.jsonl/food_048

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4603

- Triple: `(алма — related_to — картоп)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/food.jsonl/food_020, world_core/food.jsonl/food_031

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4695

- Triple: `(жая — related_to — шелпек)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0058346, world_core/food.jsonl/food_012

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4700

- Triple: `(жая — related_to — қырыққабат)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0058346, world_core/food.jsonl/food_032

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4710

- Triple: `(жеміс — related_to — қауын)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/food.jsonl/food_024, world_core/food.jsonl/food_023

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4951

- Triple: `(ағаш — related_to — ешкі)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/biology_basic.jsonl/bio_014, world_core/biology_basic.jsonl/bio_009

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #5280

- Triple: `(кезеңі — related_to — таң)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0083073, world_core/time.jsonl/time_010

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #5314

- Triple: `(күн — related_to — сұр)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/time.jsonl/time_001, world_core/colors.jsonl/color_007

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #5353

- Triple: `(түн — related_to — құла)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/time.jsonl/time_013, world_core/colors.jsonl/color_018

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #5361

- Triple: `(достық — related_to — тарих)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/proverbs.jsonl/prov_003, world_core/society.jsonl/soc_027

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #5397

- Triple: `(жыл — related_to — шымкент)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0046261, world_core/geography_kz.jsonl/geo_kz_005

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #5517

- Triple: `(елу — related_to — кеш)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/numbers.jsonl/num_015, world_core/time.jsonl/time_012

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #5851

- Triple: `(инженерлік — after — шеру)`
- Rule: `R8_after_transitivity`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0049342, wikipedia_kz_pack.json/wiki_kz_0028274

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #6037

- Triple: `(алпамыс — related_to — әңгіме)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/kz_literature.jsonl/lit_038, world_core/kz_literature.jsonl/lit_022

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #6043

- Triple: `(мәтел — related_to — қобыланды)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/kz_literature.jsonl/lit_027, world_core/kz_literature.jsonl/lit_036

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #6670

- Triple: `(мемлекет — related_to — қаңтар)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/society.jsonl/soc_002, wikipedia_kz_pack.json/wiki_kz_0009192

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #6678

- Triple: `(ала — related_to — жеті)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/colors.jsonl/color_016, world_core/numbers.jsonl/num_008

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #6911

- Triple: `(жетпіс — related_to — түндік)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/numbers.jsonl/num_017, wikipedia_kz_pack.json/wiki_kz_0056466

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #6942

- Triple: `(жирен — related_to — отыз)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/colors.jsonl/color_015, world_core/numbers.jsonl/num_013

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #7266

- Triple: `(еңбек — related_to — сусын)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: kazakh_proverbs_pack.json/proverb_068, world_core/food.jsonl/food_049

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #7280

- Triple: `(оқу — related_to — әшекей)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/proverbs.jsonl/prov_025, world_core/clothing.jsonl/cloth_028

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
- `X Y-дан кейін`: 1
- `X Y-ке барады`: 10
- `X Y-ні ал-лайды`: 2
- `X Y-ні арттыр-лайды`: 1
- `X Y-ні ата-лайды`: 1
- `X Y-ні бер-лайды`: 2
- `X Y-ні бос-лайды`: 1
- `X Y-ні біл-лайды`: 1
- `X Y-ні бұл-лайды`: 1
- `X Y-ні жаз-лайды`: 1
- `X Y-ні жарқыра-лайды`: 1
- `X Y-ні жата-лайды`: 1
- `X Y-ні жолық-лайды`: 1
- `X Y-ні кептір-лайды`: 1
- `X Y-ні кет-лайды`: 1
- `X Y-ні кір-лайды`: 1
- `X Y-ні күшейт-лайды`: 1
- `X Y-ні көш-лайды`: 1
- `X Y-ні сипатта-лайды`: 1
- `X Y-ні тап-лайды`: 1
- `X Y-ні тау-лайды`: 1
- `X Y-ні тақ-лайды`: 1
- `X Y-ні тигіз-лайды`: 1
- `X Y-ні туғыз-лайды`: 1
- `X Y-ні түсір-лайды`: 1
- `X Y-ні тұтас-лайды`: 1
- `X Y-ні қайта-лайды`: 1
- `X Y-ні қалдыр-лайды`: 1
- `X Y-ні қолдан-лайды`: 1
- `X Y-ні ұлғайт-лайды`: 1
- `X Y-ні өтіне-лайды`: 1
- `X Y-нің құрамында`: 1
- `X пен Y`: 5
- `X — Y`: 1

Sampled derivations by rule:

- `R1_is_a_transitivity`: 3
- `R2_has_inheritance`: 3
- `R3_has_inheritance_via_part_of`: 1
- `R5_shared_is_a_target`: 37
- `R7_goes_to_via_part_of`: 2
- `R8_after_transitivity`: 4
