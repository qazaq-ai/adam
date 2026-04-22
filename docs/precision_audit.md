# Precision audit — native-speaker review

**Target:** 50-fact sample + 50-derivation sample from the committed artifacts, seed `42`.

- `facts.json`: 14430 facts total (upstream status: `completed`) — sampled 50 here.
- `derived_facts.json`: 207 derivations total (upstream status: `completed`) — sampled 50 here.

## How to review

For each fact, mark the checkbox if the triple `(subject, predicate, object)` is **correct**: the sentence genuinely asserts that the subject has the claimed relation to the object, and both root resolutions are correct. When unsure, leave unchecked and add a one-line note in the Comments row. Update the **Tally** section at the bottom with your counts. Precision is defined as `correct / reviewed`.

---

## Fact sample

### Fact #1218

- Triple: `(биологиялық — does_to — әсер)`
- Predicate: `does_to`
- Pattern: `X Y-ні қайта-лайды`
- Source: `kazakh_textbooks_pack.json / kz_textbook_physics_11_emn_p0252_s14`
- Sentence:

    > Сәулелену дозасының төмендеуі биологиялық әсерді азайтады, нәтижесінде ағзаны қайта қалпына келтіру мүмкіндігі пайда болады

- [ ] Correct
- Comment:

### Fact #1440

- Triple: `(мұндай — goes_to — өсіру)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0000357`
- Sentence:

    > Мұндай агроклиматтық жағдайда бидай, қарабидай, арпа, бұршақ, зығыр, қарақұмық, картоп, кейбір көкөніс түрлерін, малазықтық дақылдарды өсіруге мүмкіндік бар.

- [ ] Correct
- Comment:

### Fact #1770

- Triple: `(қазақ — lives_in — уез)`
- Predicate: `lives_in`
- Pattern: `X Y-да тұрады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0005775`
- Sentence:

    > Қазақтар негізінен Алтай, Іле, Тарбағатай округтері кіретін автономиялық облыста, сондай-ақ ҚХР ШҰАА Мори Қазақ автономиялық ауданы, Баркөл Қазақ уездерінде, Ганьсу провинциясының Ақсай Қазақ автономиялық уезінде және аз мөлшерде Бейжіңде тұрады.

- [ ] Correct
- Comment:

### Fact #1799

- Triple: `(мемлекет — related_to — қоғам)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0006317`
- Sentence:

    > Ол ақпараттық және білім беру функцияларын орындай отырып, мемлекет пен қоғам арасындағы қатынастарды үйлестіруге бағытталған.

- [ ] Correct
- Comment:

### Fact #1806

- Triple: `(ағаш — does_to — тиек)`
- Predicate: `does_to`
- Pattern: `X Y-ні көр-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0006428`
- Sentence:

    > Қолына алып қараған батыр аспаптың мойын тұсынан титтей ағаш тиекті көреді, оны әлдебіреу ішектің астынан келтіріп орнатып қойған екен.

- [ ] Correct
- Comment:

### Fact #2280

- Triple: `(жүк — does_to — ағаш)`
- Predicate: `does_to`
- Pattern: `X Y-ні атқара-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0013448`
- Sentence:

    > Мысалы, әмбебап ауыр жүк тартушы комбайн ағаштарды кесу, тамырларды, бұтақтарды қию, діңдерді жинау мен оларды жолға сүйреп шығару жұмыстарын атқара алады.

- [ ] Correct
- Comment:

### Fact #2405

- Triple: `(айын — goes_to — қажылық)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0015688`
- Sentence:

    > Зүл-Хижжа айының 9-шы күні Мұхаммед, Меккеге қажылыққа келген, барлық мұсылмандарды Араваттағы Жабәл әл-Рахман тауына шақырып сөйлесті.

- [ ] Correct
- Comment:

### Fact #2414

- Triple: `(шетелдік — does_to — жергілік)`
- Predicate: `does_to`
- Pattern: `X Y-ні бұз-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0015908`
- Sentence:

    > Елде тұрақтылық сақталды да, саяси өзгеріс орын алды және жаңа реформалар жүрді, алайда оған қарамастан Тоқаевтың басшылығы шетелдік және жергілікті ақпарат көздерінен кейде адам құқықтарын бұзған және авторитарлық болып келеді деген сыни пікір көреді.

- [ ] Correct
- Comment:

### Fact #2767

- Triple: `(табиғат — does_to — айырмашылық)`
- Predicate: `does_to`
- Pattern: `X Y-ні көрсет-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0020032`
- Sentence:

    > Алайда ғалым табиғат зандары мен қоғам арасындағы айырмашылықты ажыратып көрсетеді.

- [ ] Correct
- Comment:

### Fact #2887

- Triple: `(би — does_to — көңіл)`
- Predicate: `does_to`
- Pattern: `X Y-ні біл-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0021365`
- Sentence:

    > Төле би іштей қабылдаса да, сыр білдірмей, қайта көңілді сыңай білдіреді.

- [ ] Correct
- Comment:

### Fact #3020

- Triple: `(қытай — does_to — әскер)`
- Predicate: `does_to`
- Pattern: `X Y-ні түсір-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0023142`
- Sentence:

    > Қытай әскерлері де үш бағытта шабуылдап, мыңдаған әскерді тұтқынға түсіреді.

- [ ] Correct
- Comment:

### Fact #3231

- Triple: `(аяғы — related_to — күз)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0026510`
- Sentence:

    > Көбінесе, жаздың аяғы мен күз айларында болатын тайфундар нөсер жауын алып келеді.

- [ ] Correct
- Comment:

### Fact #3421

- Triple: `(жуық — does_to — ел)`
- Predicate: `does_to`
- Pattern: `X Y-ні ал-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0029357`
- Sentence:

    > Көтерілісті басу үшін Францияның 1 млн-ға жуық әскерлері елді мекендерді бақылауға алды.

- [ ] Correct
- Comment:

### Fact #3441

- Triple: `(мәжіліс — does_to — атқару)`
- Predicate: `does_to`
- Pattern: `X Y-ні жалғастыра-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0029812`
- Sentence:

    > Мәжіліс сенім білдірген жағдайда, егер Республика Президенті өзгеше шешім қабылдамаса, Үкімет өз міндеттерін атқаруды жалғастыра береді.

- [ ] Correct
- Comment:

### Fact #3449

- Triple: `(ғылыми — does_to — мәселе)`
- Predicate: `does_to`
- Pattern: `X Y-ні бер-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0029942`
- Sentence:

    > ) дамуының түрлі мәселелеріне ғылыми практикалық зерттеулер жасап, қалалардың нақты жағдайын түсіндіріп, сақталған мәселелерді көрсетіп береді және шешу жолдарын ұсынады.

- [ ] Correct
- Comment:

### Fact #3764

- Triple: `(ілім — related_to — аралық)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0035160`
- Sentence:

    > “Ұлы ілім” мен “Аралық туралы ілім”' атты туындылар да Конфуцийдің өзінікі емес, ол тек мұның екеуін де қайта әңгімелеп берген деп саналады.

- [ ] Correct
- Comment:

### Fact #3961

- Triple: `(мұн — goes_to — болу)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0037966`
- Sentence:

    > Мұның өзі жүрегінде оты бар жасты туған елін, атамекенін жаудан азат ету тұралы арманға жетеледі, батыр болуға құлшындырды.

- [ ] Correct
- Comment:

### Fact #4348

- Triple: `(серік — goes_to — өту)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0043902`
- Sentence:

    > Бүгінде белгілі режиссер Серік Жармұхамедовтің Мәскеудегі Жоғары курстың екінші курсын тәмамдап, «Қазақфильмге» тәжірибеден өтуге келген шағы.

- [ ] Correct
- Comment:

### Fact #4833

- Triple: `(түйе — related_to — ат)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0051294`
- Sentence:

    > Ол заман­да негізгі көлік қатынасы түйе мен ат болғанын ескерсек, бұл сапа­рының өзі 2-3 жыл уақытты қамтыса керек.

- [ ] Correct
- Comment:

### Fact #4880

- Triple: `(төмен — does_to — рөл)`
- Predicate: `does_to`
- Pattern: `X Y-ні атқар-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0051911`
- Sentence:

    > Төмен вакуум арқылы ток жүргенде шешуші рөлді газ молекулаларының иондалуы атқарады.

- [ ] Correct
- Comment:

### Fact #5009

- Triple: `(өзара — goes_to — құлдырау)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0053257`
- Sentence:

    > Олардың қазіргі өзара тәуелділігі соншалықты, ұлттық және аймақтық нарықтардың әлемдік нарықтан кез келген оқшаулануы ол ұлтты немесе аймақты құлдырауға ұшыратады.

- [ ] Correct
- Comment:

### Fact #5563

- Triple: `(кейбір — lives_in — рет)`
- Predicate: `lives_in`
- Pattern: `X Y-да тұрады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0061593`
- Sentence:

    > Бірақ кейбір тіл ғалымдары «һамза» белгісін әріп ретінде санап, араб әліппесі 29 әріптен тұрады деп санайды.

- [ ] Correct
- Comment:

### Fact #5812

- Triple: `(жол — does_to — ат)`
- Predicate: `does_to`
- Pattern: `X Y-ні тау-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0064931`
- Sentence:

    > шамасында) “Қаған алты бөріг алаш(а) ерті” деген жолдар, Алтай тауының Сібір жағында (Тува) Алаш атты өзен, Алаш атты тау сілемдері бар.

- [ ] Correct
- Comment:

### Fact #5825

- Triple: `(жәдитшілік — goes_to — түркістан)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0065029`
- Sentence:

    > Кейін, жәдитшілік идеялары қазақ даласына және Түркістанға, Орта Азияға келген.

- [ ] Correct
- Comment:

### Fact #5850

- Triple: `(ресей — does_to — қатынас)`
- Predicate: `does_to`
- Pattern: `X Y-ні қос-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0065502`
- Sentence:

    > ) - Армения мен Ресей халықтары арасындағы дәстүрлі достық қатынастарды нығайтуға, Армения Республикасы мен Ресей Федерациясы арасындағы одақтас стратегиялық қатынастарды тереңдетуге және кеңейтуге, сонымен қатар бейбітшілік пен халықаралық қауіпсіздікті қорғауға қосқан жеке үлесі үшін.

- [ ] Correct
- Comment:

### Fact #5883

- Triple: `(қызыл — does_to — рең)`
- Predicate: `does_to`
- Pattern: `X Y-ні аш-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0066125`
- Sentence:

    > Мамыр, маусым айларында қызыл немесе қызыл күлгін реңді гүл ашады, шілде, тамыз айларында жемістенеді.

- [ ] Correct
- Comment:

### Fact #6089

- Triple: `(мысал — does_to — қолдау)`
- Predicate: `does_to`
- Pattern: `X Y-ні қабылда-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0069445`
- Sentence:

    > Баспасөзде билеушілердің экстравагант байлығы туралы мысалдар айтылған Махатхир патшалық үй шаруашылығын қаржылай қолдауды қысқартуға шешім қабылдады.

- [ ] Correct
- Comment:

### Fact #6621

- Triple: `(жер — does_to — жара)`
- Predicate: `does_to`
- Pattern: `X Y-ні ал-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0077874`
- Sentence:

    > Техника мен жер суарудың тым қара дүрсін болуы жағдайында ауыл шаруашылығына жарамды жерлер тапшы, ал мал жайылатын жерлер кеп болды.

- [ ] Correct
- Comment:

### Fact #6837

- Triple: `(бұта — related_to — астық)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0080711`
- Sentence:

    > әдетте 200 – 300 м, пассат желдердің бағытына қарай созыла орналасқан; құм бұталар мен астық тұқымдас өсімдіктермен бекіген.

- [ ] Correct
- Comment:

### Fact #7105

- Triple: `(кездесетін — does_to — жергілік)`
- Predicate: `does_to`
- Pattern: `X Y-ні іле-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0085758`
- Sentence:

    > Сирек кездесетін жергілікті өсімдіктерге : өрік, Іле барбарисі, долана, ирис Альберта жатады.

- [ ] Correct
- Comment:

### Fact #7583

- Triple: `(егер — goes_to — есеп)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0093827`
- Sentence:

    > Егер Антарктиданың мұздық сауыты ери бастаса, ол біздің планетамыздың барлық өзендерін, қазіргі өзендердегі бар суды есепке алғанда, 500 жылдан астам уақыт қоректендіруге жетер еді, ал дүние жүзілік мұхиттық деңгейі мұздық суынан 60 метрден астам көтерілген болар еді.

- [ ] Correct
- Comment:

### Fact #8239

- Triple: `(киіз — does_to — үй)`
- Predicate: `does_to`
- Pattern: `X Y-ні ұсын-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0104597`
- Sentence:

    > Газет рәміздік-бейне ретінде киіз үйді ұсынды.

- [ ] Correct
- Comment:

### Fact #8482

- Triple: `(домалақ — does_to — қабат)`
- Predicate: `does_to`
- Pattern: `X Y-ні түзіл-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0108078`
- Sentence:

    > Лизосома ( - еріту, - төн) - домалақ немесе сопақша пішінді, бір қабатты жарғақшалы түзіліс.

- [ ] Correct
- Comment:

### Fact #8496

- Triple: `(дәріс — does_to — жұмыс)`
- Predicate: `does_to`
- Pattern: `X Y-ні жаз-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0108402`
- Sentence:

    > Лотман бәріне үлгеретін: дәріс оқиды, курстық жұмыстарды басқарады, мақалалар жазады.

- [ ] Correct
- Comment:

### Fact #9075

- Triple: `(өнімдері — is_a — күріш)`
- Predicate: `is_a`
- Pattern: `X — Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0117263`
- Sentence:

    > Өнімдері — арахис, жүгері, күріш, мақта.

- [ ] Correct
- Comment:

### Fact #9130

- Triple: `(қала — has — атау)`
- Predicate: `has`
- Pattern: `X-тың Y-сы бар`
- Source: `wikipedia_kz_pack.json / wiki_kz_0118247`
- Sentence:

    > Тақтада Жайық өзенінің жағасында орналасқан қалалардың атаулары бар лента бейнеленген.

- [ ] Correct
- Comment:

### Fact #9329

- Triple: `(ауа — does_to — күш)`
- Predicate: `does_to`
- Pattern: `X Y-ні жете-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0122447`
- Sentence:

    > Жылы атлант ауа массаларының да аудан аума- ғының ылғалына әсері мардымсыз, өйткені жолда күшті өзгеріске үшырап жетеді және жазық жер бедерінің үстімен тоқтамай өтіп кетеді.

- [ ] Correct
- Comment:

### Fact #9850

- Triple: `(тігу — has_quantity — тәсіл)`
- Predicate: `has_quantity`
- Pattern: `X-тың N Y-ы бар`
- Source: `wikipedia_kz_pack.json / wiki_kz_0131640`
- Sentence:

    > Байпақ тігудің екі тәсілі бар.

- [ ] Correct
- Comment:

### Fact #9870

- Triple: `(жыл — lives_in — шіл)`
- Predicate: `lives_in`
- Pattern: `X Y-да тұрады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0131968`
- Sentence:

    > Ол қорғаныс (1942 жылы 17 шілде – 18 қараша) және шабуыл (1942 жылы 19 қараша – 1943 жылы 2 ақпан) кезеңдерінен тұрады.

- [ ] Correct
- Comment:

### Fact #9952

- Triple: `(соны — does_to — ақыл)`
- Predicate: `does_to`
- Pattern: `X Y-ні жібер-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0132871`
- Sentence:

    > Не көрдің, не есттің, әрнешік білдің, соны тездікпенен ұғып, ұққандықпенен тұрмай, арты қайдан шығады, алды қайда барады, сол екі жағына да ақылды жіберіп қарамаққа тез қозғап жібереді.

- [ ] Correct
- Comment:

### Fact #10326

- Triple: `(дін — does_to — әлем)`
- Predicate: `does_to`
- Pattern: `X Y-ні тыр-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0138890`
- Sentence:

    > Дін атаулы адам мен әлемді Құдіретті күшпен тұтастыруға тырысады.

- [ ] Correct
- Comment:

### Fact #10999

- Triple: `(шар — does_to — ара)`
- Predicate: `does_to`
- Pattern: `X Y-ні аш-лайды`
- Source: `synthetic_sentences_pack.json / synth_00195`
- Sentence:

    > алты шар арамды ашады.

- [ ] Correct
- Comment:

### Fact #11863

- Triple: `(абай — does_to — психологиялық)`
- Predicate: `does_to`
- Pattern: `X Y-ні ата-лайды`
- Source: `synthetic_sentences_pack.json / synth_05141`
- Sentence:

    > абай ауылдан психологиялықты атады.

- [ ] Correct
- Comment:

### Fact #12286

- Triple: `(жалпақ — does_to — түбег)`
- Predicate: `does_to`
- Pattern: `X Y-ні кеңі-лайды`
- Source: `synthetic_sentences_pack.json / synth_07575`
- Sentence:

    > жатқанда жалпақ түбегінді кеңіді.

- [ ] Correct
- Comment:

### Fact #12414

- Triple: `(ғарыш — does_to — қарж)`
- Predicate: `does_to`
- Pattern: `X Y-ні тік-лайды`
- Source: `synthetic_sentences_pack.json / synth_08197`
- Sentence:

    > жылдам ғарыш қаржды тікті әрі кішке талады.

- [ ] Correct
- Comment:

### Fact #12858

- Triple: `(жұлдыз — does_to — сүт)`
- Predicate: `does_to`
- Pattern: `X Y-ні кеңі-лайды`
- Source: `synthetic_sentences_pack.json / synth_10609`
- Sentence:

    > әдемі жұлдыз сүтті кеңіді.

- [ ] Correct
- Comment:

### Fact #12903

- Triple: `(салқын — does_to — үй)`
- Predicate: `does_to`
- Pattern: `X Y-ні қаба-лайды`
- Source: `synthetic_sentences_pack.json / synth_10857`
- Sentence:

    > көші-қонде салқын үйлерді қабады.

- [ ] Correct
- Comment:

### Fact #13095

- Triple: `(атқаруш — does_to — логикалық)`
- Predicate: `does_to`
- Pattern: `X Y-ні қайыр-лайды`
- Source: `synthetic_sentences_pack.json / synth_12034`
- Sentence:

    > атқаруш логикалықты қайырды себебі ертең жезқазеді.

- [ ] Correct
- Comment:

### Fact #13098

- Triple: `(төбе — does_to — өзін)`
- Predicate: `does_to`
- Pattern: `X Y-ні аула-лайды`
- Source: `synthetic_sentences_pack.json / synth_12065`
- Sentence:

    > төбе өзінді аулады өйткені ендіг ішкті ішейді.

- [ ] Correct
- Comment:

### Fact #14045

- Triple: `(президенттік — does_to — эфир)`
- Predicate: `does_to`
- Pattern: `X Y-ні тірке-лайды`
- Source: `synthetic_sentences_pack.json / synth_17333`
- Sentence:

    > президенттік ұзын эфирді тіркеді.

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

- `X Y-да тұрады`: 3
- `X Y-ке барады`: 7
- `X Y-ні ал-лайды`: 2
- `X Y-ні ата-лайды`: 1
- `X Y-ні атқар-лайды`: 1
- `X Y-ні атқара-лайды`: 1
- `X Y-ні аула-лайды`: 1
- `X Y-ні аш-лайды`: 2
- `X Y-ні бер-лайды`: 1
- `X Y-ні біл-лайды`: 1
- `X Y-ні бұз-лайды`: 1
- `X Y-ні жаз-лайды`: 1
- `X Y-ні жалғастыра-лайды`: 1
- `X Y-ні жете-лайды`: 1
- `X Y-ні жібер-лайды`: 1
- `X Y-ні кеңі-лайды`: 2
- `X Y-ні көр-лайды`: 1
- `X Y-ні көрсет-лайды`: 1
- `X Y-ні тау-лайды`: 1
- `X Y-ні тыр-лайды`: 1
- `X Y-ні тік-лайды`: 1
- `X Y-ні тірке-лайды`: 1
- `X Y-ні түзіл-лайды`: 1
- `X Y-ні түсір-лайды`: 1
- `X Y-ні іле-лайды`: 1
- `X Y-ні қаба-лайды`: 1
- `X Y-ні қабылда-лайды`: 1
- `X Y-ні қайта-лайды`: 1
- `X Y-ні қайыр-лайды`: 1
- `X Y-ні қос-лайды`: 1
- `X Y-ні ұсын-лайды`: 1
- `X пен Y`: 5
- `X — Y`: 1
- `X-тың N Y-ы бар`: 1
- `X-тың Y-сы бар`: 1

Sampled derivations by rule:

- `R1_is_a_transitivity`: 8
- `R2_has_inheritance`: 24
- `R5_shared_is_a_target`: 18
