# Precision audit — native-speaker review

> **v6.0 forward-looking note (2026-05-16).** This audit covers the
> deterministic retrieval + derivation pipeline (v5.x). The v6.0
> neural composition layer carries its own factual-grounding gate
> ([`verifier_demo` prototype](../crates/adam-agg-model/src/bin/verifier_demo.rs);
> production wiring tracked in [`roadmap_v6_v7.md`](roadmap_v6_v7.md)
> as release-blocker #2) that must reach 0-hallucination on a
> 100-prompt factual eval set before v6.0.0 GA. This document's
> precision baseline (deterministic side) becomes the lower bound
> the v6.0 verifier-gated path must match or exceed.

**Target:** 50-fact sample + 50-derivation sample from the committed artifacts, seed `42`.

- `facts.json`: 13745 facts total (upstream status: `completed`) — sampled 50 here.
- `derived_facts.json`: 7866 derivations total (upstream status: `completed`) — sampled 50 here.

## How to review

For each fact, mark the checkbox if the triple `(subject, predicate, object)` is **correct**: the sentence genuinely asserts that the subject has the claimed relation to the object, and both root resolutions are correct. When unsure, leave unchecked and add a one-line note in the Comments row. Update the **Tally** section at the bottom with your counts. Precision is defined as `correct / reviewed`.

---

## Fact sample

### Fact #1218

- Triple: `(уран — does_to — қабілет)`
- Predicate: `does_to`
- Pattern: `X Y-ні тап-лайды`
- Source: `kazakh_textbooks_pack.json / kz_textbook_physics_11_ogn_p0152_s05`
- Sentence:

    > Ол уран тұздары салқын жарқыл - люминесценцияны тудыратын, мөлдір емес заттардың қабаттары арқылы өтіп, газ- дарды иондайтын, фотографиялық пластиналарды қара түске бояуға қабілетті көзге көрінбейтін сәуле- лерді шығаратынын тапты

- [ ] Correct
- Comment:

### Fact #1366

- Triple: `(әзірбайжан — related_to — шығыс)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0001374`
- Sentence:

    > Империяға қызмет еткен оғыз топтары қазіргі Әзірбайжан мен шығыс Түркияның қоныс аударған кезде түркі мәдениетінің таралуында маңызды рөл атқарды.

- [ ] Correct
- Comment:

### Fact #1440

- Triple: `(табиғи — does_to — өсімдік)`
- Predicate: `does_to`
- Pattern: `X Y-ні төле-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0003217`
- Sentence:

    > Табиғи өсімдіктерді, жануарлар дүниесін сақтап қалу үшін Төле би, Түлкібас аудандары аумағында мемлекеттік Ақсу-Жабағылы қорығы (1926) ұйымдастырылған.

- [ ] Correct
- Comment:

### Fact #1770

- Triple: `(өтініш — lives_in — жер)`
- Predicate: `lives_in`
- Pattern: `X Y-да тұрады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0008193`
- Sentence:

    > Директордың өтініші бойынша ол пиротехникалық құрылғыларға жақын жерде қауіпті болып тұрды.

- [ ] Correct
- Comment:

### Fact #1799

- Triple: `(ақтөбе — does_to — ел)`
- Predicate: `does_to`
- Pattern: `X Y-ні кең-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0009005`
- Sentence:

    > «Ақтөбе» және осыған ұқсас елді мекен атаулары Қазақстаннан басқа елдерде де кең таралған (Ахтуба, Актюба, Ақтепа, Ақ Таппе, Ақдепе) .

- [ ] Correct
- Comment:

### Fact #1806

- Triple: `(құлдырау — related_to — халық)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0009052`
- Sentence:

    > Бастапқы уақытта тез дамымағанымен, Ақтөбе қаласы бірқалыпты өсу қарқынын сақтап, Қостанай секілді құлдырау мен халық санының азаюына душар болған жоқ.

- [ ] Correct
- Comment:

### Fact #2280

- Triple: `(хан — goes_to — шығу)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0016993`
- Sentence:

    > Тұғырыл хан Темучинды ұлы санап, оған Керей хандығының билігін қалдыру жайлы ойы барын білгендіктен Сенгум Темучинге қарсы жорық шығуға әкесін көндірді.

- [ ] Correct
- Comment:

### Fact #2405

- Triple: `(болу — does_to — әлем)`
- Predicate: `does_to`
- Pattern: `X Y-ні ұста-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0018301`
- Sentence:

    > Поэзияда философ болу өзін қоршаған әлемді ұғыну, әр заттың мәнін білу, ақырына дейін “адам жанының инженері болып қалу” дегенді ұстанды (Күнделік, 10.

- [ ] Correct
- Comment:

### Fact #2414

- Triple: `(жұрт — does_to — сақал)`
- Predicate: `does_to`
- Pattern: `X Y-ні құй-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0018424`
- Sentence:

    > Жұрт осылайша мәз–мейрам болып жатқанда, бір ақ сақалды қарт құйған мүсіндей болып, шеттеу отырады.

- [ ] Correct
- Comment:

### Fact #2767

- Triple: `(ат — related_to — ағаш)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0023676`
- Sentence:

    > Үлкен обаға үйілген төбе астында жерден қазылған қабырда өлген кісінің мәйіті жатады, ал кіші қорғанда үйінді астындағы қабырға ат пен ағаш ыдыс-аяқтар қойылады.

- [ ] Correct
- Comment:

### Fact #2887

- Triple: `(құқық — goes_to — дүние)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0025532`
- Sentence:

    > Крепостнойлық құқық жойылғаннан кейін шаруашылық өмірмен және жазба тілмен бірге ұлттық мәдениет (әдебиет, музыка, өнер) дүниеге келді.

- [ ] Correct
- Comment:

### Fact #3020

- Triple: `(еуропа — does_to — тиім)`
- Predicate: `does_to`
- Pattern: `X Y-ні қайта-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0028041`
- Sentence:

    > Еуропа Одақ шеңберінде өнеркәсіптің осы саласын дамытуға бағытталған шаралар нәтижесінде өндірісті неғұрлым тиімді ұйымдастырып, қайта жабдықтауға мүмкіндік туды.

- [ ] Correct
- Comment:

### Fact #3231

- Triple: `(мұсылман — does_to — жергілік)`
- Predicate: `does_to`
- Pattern: `X Y-ні ұста-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0032148`
- Sentence:

    > Халқының 50%-ы мұсылмандар, 40%-ы христиандар, қалған бөлігі жергілікті діни наным-сенімдерді ұстанады.

- [ ] Correct
- Comment:

### Fact #3421

- Triple: `(адам — related_to — дәрігер)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0035592`
- Sentence:

    > Науқас адамдар мен дәрігерлер денсаулық сақтау облысындағы соңғы ашылған жаңалықтарды біліп отырады.

- [ ] Correct
- Comment:

### Fact #3441

- Triple: `(жанр — has — көшпел)`
- Predicate: `has`
- Pattern: `X-тың Y-сы бар`
- Source: `wikipedia_kz_pack.json / wiki_kz_0035900`
- Sentence:

    > Ақындар айтысына ұқсас жанрдың көшпелі араб тайпаларында бар екеніне (мұғалақат) тоқталып, қазір ірі жанр ретінде жоқ болғанмен, ертерек уақыттарда Еуропа халықтарында да айтысқа ұқсас жанрлар болғанын атап көрсетеді.

- [ ] Correct
- Comment:

### Fact #3449

- Triple: `(дық — goes_to — рәсім)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0035988`
- Sentence:

    > Көңілінде дығы бар, береке жағына беттемегендер ғана бұл рәсімге бой ұрмаған.

- [ ] Correct
- Comment:

### Fact #3764

- Triple: `(рұқсат — does_to — жүктілік)`
- Predicate: `does_to`
- Pattern: `X Y-ні қабылда-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0041152`
- Sentence:

    > Мемлекетте аборт жасау заңмен рұқсат етілгендіктен, индуистер қыз баланың дүниеге келуіне бөгет жасауды, қалаусыз болған жүктілікті тоқтатуды дұрыс деп қабылдайды.

- [ ] Correct
- Comment:

### Fact #3961

- Triple: `(өткен — does_to — ат)`
- Predicate: `does_to`
- Pattern: `X Y-ні ал-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0044930`
- Sentence:

    > 2002 жылы Женевада өткен «Ғасырлық сапа кезеңі» атты көрмеде түркімен кілемі алтын жүлде алған.

- [ ] Correct
- Comment:

### Fact #4348

- Triple: `(түйе — related_to — ат)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0051294`
- Sentence:

    > Ол заман­да негізгі көлік қатынасы түйе мен ат болғанын ескерсек, бұл сапа­рының өзі 2-3 жыл уақытты қамтыса керек.

- [ ] Correct
- Comment:

### Fact #4833

- Triple: `(сортаң — does_to — тұз)`
- Predicate: `does_to`
- Pattern: `X Y-ні қалыпта-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0058820`
- Sentence:

    > Шөлдің сұр топырағы, сортаң топырақ, тақыр кездеседі, тұйық тұзды көлдер айналасында құрғақшылық әсерінен қалыптасқан кең алқапты сорлар тараған.

- [ ] Correct
- Comment:

### Fact #4880

- Triple: `(солтүстік — does_to — мұз)`
- Predicate: `does_to`
- Pattern: `X Y-ні ұшыра-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0059563`
- Sentence:

    > Солтүстік мұзды мұхитта апатқа ұшыраған У.

- [ ] Correct
- Comment:

### Fact #5009

- Triple: `(арғын — does_to — әбілқайыр)`
- Predicate: `does_to`
- Pattern: `X Y-ні кет-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0061671`
- Sentence:

    > Сонда арғындар (және керейлер) Жәнібек пен Керей сұлтанға ілесіп, Әбілқайырды тастап кетеді.

- [ ] Correct
- Comment:

### Fact #5563

- Triple: `(ереже — does_to — жергілік)`
- Predicate: `does_to`
- Pattern: `X Y-ні туғыз-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0071129`
- Sentence:

    > Ереже бойынша белгіленген 8 сыртқы дуанның [Қарқаралы, Құсмұрын (1824), Аягөз (1831), Ақмола, Аманқарағай (1832), Баянауыл, Үшбұлақ (1833), Көкпекті (1844)] ашылуы жергілікті халықтың наразылығын туғызды.

- [ ] Correct
- Comment:

### Fact #5812

- Triple: `(жарық — goes_to — табу)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0075854`
- Sentence:

    > 1-томында (1826) Абелдің 7 мақаласы жарық көреді, оның ішінде 5-дәрежелі алгебралық теңдеудің шешуі әдеттегі түбір табу жолымен табуға болмайтындығының дәлелдеуі бар еді.

- [ ] Correct
- Comment:

### Fact #5825

- Triple: `(өткен — goes_to — соғысу)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0076156`
- Sentence:

    > 1943 жылы өткен Каир және Тегеран конференцияларында ағылшын – американ үкіметтері 1944 жылы мамыр айында Еуропада екінші майдан ашуға келісті, ал Кеңес Одағы Германиямен соғыс аяқталғаннан кейін Жапониямен соғысуға міндеттенді.

- [ ] Correct
- Comment:

### Fact #5850

- Triple: `(қолға — does_to — жауынгер)`
- Predicate: `does_to`
- Pattern: `X Y-ні шық-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0076512`
- Sentence:

    > Әскерге басшылық жасауды қолға алған Уошингтон жауынгерлерді қоршаудан алып шығады.

- [ ] Correct
- Comment:

### Fact #5883

- Triple: `(күрдел — lives_in — жағдай)`
- Predicate: `lives_in`
- Pattern: `X Y-да тұрады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0077002`
- Sentence:

    > Аудан тұрғындарының денсаулығы аса күрделі жағдайда тұр.

- [ ] Correct
- Comment:

### Fact #6089

- Triple: `(нағыз — does_to — үйлесім)`
- Predicate: `does_to`
- Pattern: `X Y-ні піш-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0080087`
- Sentence:

    > Нағыз үйлесімді пішінді Лакколиттер жер қыртысында аз кездеседі.

- [ ] Correct
- Comment:

### Fact #6621

- Triple: `(киел — goes_to — болу)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0090276`
- Sentence:

    > Сені киелі ететін қасиет – басқалардан жақсырақ болуға қабілетті екендігің… Біздің мына зұлмат заманымызда сен адалдық, тазалық пен жан жомартты­ғының үлгісін көрсетумен келесің…”.

- [ ] Correct
- Comment:

### Fact #6837

- Triple: `(аргентиналық — does_to — ат)`
- Predicate: `does_to`
- Pattern: `X Y-ні қолдан-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0094172`
- Sentence:

    > Өзінің аргентиналық шығу тегін білдіру үшін Че деген лақап атты қолданды.

- [ ] Correct
- Comment:

### Fact #7105

- Triple: `(қырғыз — does_to — мәселе)`
- Predicate: `does_to`
- Pattern: `X Y-ні жүгін-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0099199`
- Sentence:

    > Қырғыздар ежелден маңызды мәселелерді шешкенде ақсақалдар кеңесіне жүгінген.

- [ ] Correct
- Comment:

### Fact #7583

- Triple: `(көк — does_to — сабақ)`
- Predicate: `does_to`
- Pattern: `X Y-ні жаз-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0107602`
- Sentence:

    > Көк сабақтарды жаз бойы бірнеше рет кесіп алады, бірақ тамыздың бірінші жартысынан кейін тоқтатады, өйткені мұның салдары келер жылдың өніміне кері әсер етеді.

- [ ] Correct
- Comment:

### Fact #8239

- Triple: `(ақш — does_to — ат)`
- Predicate: `does_to`
- Pattern: `X Y-ні қат-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0118836`
- Sentence:

    > «АҚШ кітапханаларының менеджменті» атты халықаралық бағдарламаға қатысты.

- [ ] Correct
- Comment:

### Fact #8482

- Triple: `(дуа — does_to — сиқыр)`
- Predicate: `does_to`
- Pattern: `X Y-ні дәріпте-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0124249`
- Sentence:

    > , сондай-ақ бір кездері өте кең тараған дуа мен сиқырларды поэтикалық түрде дәріптейді.

- [ ] Correct
- Comment:

### Fact #8496

- Triple: `(ақтабан — does_to — шұбырын)`
- Predicate: `does_to`
- Pattern: `X Y-ні көр-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0124478`
- Sentence:

    > Ол кезде кешегі Ақтабан шұбырынды уақытындағы қалмақтардың тарапынан аяусыз қырғынды көрген адам­дардың көбінің көзі тірі болатын.

- [ ] Correct
- Comment:

### Fact #9075

- Triple: `(қазақстан — is_a — ауыл)`
- Predicate: `is_a`
- Pattern: `X — Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0135513`
- Sentence:

    > Қазақстан — Келес ауданындағы ауыл.

- [ ] Correct
- Comment:

### Fact #9130

- Triple: `(адам — does_to — тамақ)`
- Predicate: `does_to`
- Pattern: `X Y-ні ете-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0136304`
- Sentence:

    > Қышқылы аз адамдар бір қасық балды тамақты ішер алдында ғана жесе, ол қорытатын шырынның көбірек бөлінуіне әсер етеді.

- [ ] Correct
- Comment:

### Fact #9329

- Triple: `(арал — does_to — зиян)`
- Predicate: `does_to`
- Pattern: `X Y-ні тигіз-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0139656`
- Sentence:

    > Арал апаты сол өңірдегі 60 мыңға жуық тұрғынының денсаулығына зиянды әсерін тигізді.

- [ ] Correct
- Comment:

### Fact #9850

- Triple: `(жеңіс — does_to — тайпалық)`
- Predicate: `does_to`
- Pattern: `X Y-ні қақ-лайды`
- Source: `synthetic_sentences_pack.json / synth_00017`
- Sentence:

    > жеңіс тайпалықты қақты себебі ішек қызылорды қыдырады.

- [ ] Correct
- Comment:

### Fact #9870

- Triple: `(ақ- — does_to — мәңг)`
- Predicate: `does_to`
- Pattern: `X Y-ні өркенде-лайды`
- Source: `synthetic_sentences_pack.json / synth_00130`
- Sentence:

    > ақ- мәңгді өркендеді сөйтіп қайш өзіңізді қасынады.

- [ ] Correct
- Comment:

### Fact #9952

- Triple: `(төсек — does_to — ғарыш)`
- Predicate: `does_to`
- Pattern: `X Y-ні кешір-лайды`
- Source: `synthetic_sentences_pack.json / synth_00625`
- Sentence:

    > төсек қаржыландыруға таза ғарышты кешірді.

- [ ] Correct
- Comment:

### Fact #10326

- Triple: `(техникалық — does_to — қысым)`
- Predicate: `does_to`
- Pattern: `X Y-ні қыра-лайды`
- Source: `synthetic_sentences_pack.json / synth_02941`
- Sentence:

    > техникалық қысымды қырады бірақ жоқ жатайды.

- [ ] Correct
- Comment:

### Fact #10999

- Triple: `(академиялық — does_to — болмақ)`
- Predicate: `does_to`
- Pattern: `X Y-ні көшір-лайды`
- Source: `synthetic_sentences_pack.json / synth_06975`
- Sentence:

    > мұрагерда академиялық болмақты көшірді.

- [ ] Correct
- Comment:

### Fact #11863

- Triple: `(шах — does_to — сортаң)`
- Predicate: `does_to`
- Pattern: `X Y-ні жүгір-лайды`
- Source: `synthetic_sentences_pack.json / synth_12151`
- Sentence:

    > кеше шах сортаңды жүгірді.

- [ ] Correct
- Comment:

### Fact #12286

- Triple: `(емен — does_to — қор)`
- Predicate: `does_to`
- Pattern: `X Y-ні көш-лайды`
- Source: `synthetic_sentences_pack.json / synth_14659`
- Sentence:

    > емен өлеңнің қорды көшеді.

- [ ] Correct
- Comment:

### Fact #12414

- Triple: `(атай — does_to — теңіз)`
- Predicate: `does_to`
- Pattern: `X Y-ні тірке-лайды`
- Source: `synthetic_sentences_pack.json / synth_15426`
- Sentence:

    > атай сүйікке қиын теңізінді тіркесті.

- [ ] Correct
- Comment:

### Fact #12858

- Triple: `(қолдану — does_to — әрк)`
- Predicate: `does_to`
- Pattern: `X Y-ні тоқтат-лайды`
- Source: `synthetic_sentences_pack.json / synth_18002`
- Sentence:

    > қолдану әркті тоқтатты өйткені әлі қозғайды.

- [ ] Correct
- Comment:

### Fact #12903

- Triple: `(ақпан — does_to — наразылық)`
- Predicate: `does_to`
- Pattern: `X Y-ні бұл-лайды`
- Source: `synthetic_sentences_pack.json / synth_18294`
- Sentence:

    > ақпан өзгеруге аз наразылықты бұлды.

- [ ] Correct
- Comment:

### Fact #13095

- Triple: `(тани — does_to — университетін)`
- Predicate: `does_to`
- Pattern: `X Y-ні ұйықта-лайды`
- Source: `synthetic_sentences_pack.json / synth_19386`
- Sentence:

    > қалсада тани университетінді ұйықтады.

- [ ] Correct
- Comment:

### Fact #13098

- Triple: `(жиһаз — does_to — қайырымдылық)`
- Predicate: `does_to`
- Pattern: `X Y-ні шай-лайды`
- Source: `synthetic_sentences_pack.json / synth_19426`
- Sentence:

    > осылай жиһаз қайырымдылықты шайды және жаман ақындық сәйкеседі.

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

- Triple: `(тас жол — is_a — құрылыс)`
- Rule: `R1_is_a_transitivity`
- Confidence: `rule_inferred`
- Source chain: world_core/transport.jsonl/tr_019, world_core/transport.jsonl/tr_016

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #369

- Triple: `(дуадақ — has — түр)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0130217, wikipedia_kz_pack.json/wiki_kz_0085083

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #528

- Triple: `(роман — has — көшпел)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: world_core/kz_literature.jsonl/lit_021, wikipedia_kz_pack.json/wiki_kz_0035900

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #821

- Triple: `(жылан — related_to — сиыр)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/animals.jsonl/anm_024, world_core/biology_basic.jsonl/bio_007

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #842

- Triple: `(жылқы — related_to — құс)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/biology_basic.jsonl/bio_005, world_core/animals.jsonl/anm_022

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #858

- Triple: `(жәндік — related_to — мал)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/animals.jsonl/anm_029, world_core/animals.jsonl/anm_038

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #930

- Triple: `(қой — related_to — құс)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/biology_basic.jsonl/bio_006, world_core/animals.jsonl/anm_022

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #939

- Triple: `(қарбыз — related_to — қауын)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/food.jsonl/food_022, world_core/food.jsonl/food_023

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #1843

- Triple: `(алтын — related_to — сары)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/colors.jsonl/color_011, world_core/colors.jsonl/color_003

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #1895

- Triple: `(жасыл — related_to — қызғылт)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/colors.jsonl/color_004, world_core/colors.jsonl/color_009

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #1940

- Triple: `(көк — related_to — қызыл)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/colors.jsonl/color_002, world_core/colors.jsonl/color_001

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #2317

- Triple: `(қарағанды — related_to — қызылорда)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/geography_kz.jsonl/geo_kz_006, world_core/geography_kz.jsonl/geo_kz_015

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #2451

- Triple: `(блог — lives_in — апта)`
- Rule: `R6_lives_in_via_part_of`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0048220, world_core/time.jsonl/time_002

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #2666

- Triple: `(дін — goes_to — ел)`
- Rule: `R7_goes_to_via_part_of`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0138845, world_core/proverbs.jsonl/prov_035

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #2683

- Triple: `(неше — after — наразылық)`
- Rule: `R8_after_transitivity`
- Confidence: `rule_inferred`
- Source chain: kazakh_textbooks_pack.json/kz_textbook_algebra_7_p0166_s05, wikipedia_kz_pack.json/wiki_kz_0006688

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #2727

- Triple: `(бойындағ — after — үзіліс)`
- Rule: `R8_after_transitivity`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0006195, wikipedia_kz_pack.json/wiki_kz_0023126

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #3205

- Triple: `(шықпай — after — қуғындау)`
- Rule: `R8_after_transitivity`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0117248, wikipedia_kz_pack.json/wiki_kz_0048591

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #3380

- Triple: `(зоология — is_a — мәлімет)`
- Rule: `R1_is_a_transitivity`
- Confidence: `rule_inferred`
- Source chain: world_core/animals.jsonl/anm_039, world_core/society.jsonl/soc_023

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #3653

- Triple: `(алтын — related_to — мейірім)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/colors.jsonl/color_011, world_core/proverbs.jsonl/prov_027

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4018

- Triple: `(аю — related_to — үйрек)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/animals.jsonl/anm_005, world_core/animals.jsonl/anm_019

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4041

- Triple: `(аққу — related_to — қой)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/animals.jsonl/anm_015, world_core/biology_basic.jsonl/bio_006

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4363

- Triple: `(тағы жануар — related_to — үйрек)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/animals.jsonl/anm_037, world_core/animals.jsonl/anm_019

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4373

- Triple: `(тиін — related_to — үйрек)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/animals.jsonl/anm_007, world_core/animals.jsonl/anm_019

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4388

- Triple: `(тырна — related_to — қоян)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/animals.jsonl/anm_014, world_core/animals.jsonl/anm_006

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4556

- Triple: `(абай — related_to — капитан)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/kz_literature.jsonl/lit_001, world_core/transport.jsonl/tr_029

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4603

- Triple: `(ата — related_to — жазушы)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/proverbs.jsonl/prov_029, world_core/kz_literature.jsonl/lit_030

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4695

- Triple: `(дәрігер — related_to — ілияс)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/society.jsonl/soc_032, world_core/kz_literature.jsonl/lit_010

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4700

- Triple: `(жазушы — related_to — махамбет)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/kz_literature.jsonl/lit_030, world_core/kz_literature.jsonl/lit_003

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4710

- Triple: `(жазушы — related_to — ғабиден)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/kz_literature.jsonl/lit_030, world_core/kz_literature.jsonl/lit_013

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4951

- Triple: `(кітап — related_to — құлақ)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/society.jsonl/soc_021, world_core/body_parts.jsonl/body_004

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #5280

- Triple: `(сәбіз — related_to — ірімшік)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/food.jsonl/food_028, world_core/food.jsonl/food_008

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #5314

- Triple: `(қаймақ — related_to — қырыққабат)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/food.jsonl/food_007, world_core/food.jsonl/food_032

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #5353

- Triple: `(ана — related_to — жылан)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0069231, world_core/animals.jsonl/anm_024

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #5361

- Triple: `(ана — related_to — шөп)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0069231, world_core/biology_basic.jsonl/bio_015

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #5397

- Triple: `(ағаш — related_to — түйе)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/biology_basic.jsonl/bio_014, world_core/biology_basic.jsonl/bio_008

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #5517

- Triple: `(жануар — related_to — әсел)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/biology_basic.jsonl/bio_012, wikipedia_kz_pack.json/wiki_kz_0146217

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #5851

- Triple: `(жыл — related_to — қазақ)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0046261, wikipedia_kz_pack.json/wiki_kz_0001219

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #6037

- Triple: `(алпыс — related_to — уақыт)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/numbers.jsonl/num_016, world_core/time.jsonl/time_020

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #6043

- Triple: `(алты — related_to — таң)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/numbers.jsonl/num_007, world_core/time.jsonl/time_010

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #6670

- Triple: `(ертіс — related_to — сырға)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/geography_kz.jsonl/geo_kz_019, world_core/clothing.jsonl/cloth_030

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #6678

- Triple: `(керосин — related_to — іле)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/transport.jsonl/tr_035, world_core/geography_kz.jsonl/geo_kz_021

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #6911

- Triple: `(ағаш — related_to — жағалтай)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/biology_basic.jsonl/bio_014, wikipedia_kz_pack.json/wiki_kz_0135186

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #6942

- Triple: `(бидай — related_to — дуадақ)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/biology_basic.jsonl/bio_017, wikipedia_kz_pack.json/wiki_kz_0130217

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #7266

- Triple: `(алпыс — related_to — боз)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/numbers.jsonl/num_016, world_core/colors.jsonl/color_017

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #7392

- Triple: `(боз — related_to — төрт)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/colors.jsonl/color_017, world_core/numbers.jsonl/num_005

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #7395

- Triple: `(боз — related_to — үш)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/colors.jsonl/color_017, world_core/numbers.jsonl/num_004

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #7683

- Triple: `(нөл — related_to — құла)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/numbers.jsonl/num_001, world_core/colors.jsonl/color_018

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

- `X Y-да тұрады`: 2
- `X Y-ке барады`: 6
- `X Y-ні ал-лайды`: 1
- `X Y-ні бұл-лайды`: 1
- `X Y-ні дәріпте-лайды`: 1
- `X Y-ні ете-лайды`: 1
- `X Y-ні жаз-лайды`: 1
- `X Y-ні жүгін-лайды`: 1
- `X Y-ні жүгір-лайды`: 1
- `X Y-ні кет-лайды`: 1
- `X Y-ні кешір-лайды`: 1
- `X Y-ні кең-лайды`: 1
- `X Y-ні көр-лайды`: 1
- `X Y-ні көш-лайды`: 1
- `X Y-ні көшір-лайды`: 1
- `X Y-ні піш-лайды`: 1
- `X Y-ні тап-лайды`: 1
- `X Y-ні тигіз-лайды`: 1
- `X Y-ні тоқтат-лайды`: 1
- `X Y-ні туғыз-лайды`: 1
- `X Y-ні тірке-лайды`: 1
- `X Y-ні төле-лайды`: 1
- `X Y-ні шай-лайды`: 1
- `X Y-ні шық-лайды`: 1
- `X Y-ні қабылда-лайды`: 1
- `X Y-ні қайта-лайды`: 1
- `X Y-ні қалыпта-лайды`: 1
- `X Y-ні қат-лайды`: 1
- `X Y-ні қақ-лайды`: 1
- `X Y-ні қолдан-лайды`: 1
- `X Y-ні қыра-лайды`: 1
- `X Y-ні құй-лайды`: 1
- `X Y-ні ұйықта-лайды`: 1
- `X Y-ні ұста-лайды`: 2
- `X Y-ні ұшыра-лайды`: 1
- `X Y-ні өркенде-лайды`: 1
- `X пен Y`: 5
- `X — Y`: 1
- `X-тың Y-сы бар`: 1

Sampled derivations by rule:

- `R1_is_a_transitivity`: 5
- `R2_has_inheritance`: 2
- `R5_shared_is_a_target`: 38
- `R6_lives_in_via_part_of`: 1
- `R7_goes_to_via_part_of`: 1
- `R8_after_transitivity`: 3
