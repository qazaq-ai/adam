# Precision audit — native-speaker review

**Target:** 50-fact sample + 50-derivation sample from the committed artifacts, seed `42`.

- `facts.json`: 13627 facts total (upstream status: `completed`) — sampled 50 here.
- `derived_facts.json`: 205 derivations total (upstream status: `completed`) — sampled 50 here.

## How to review

For each fact, mark the checkbox if the triple `(subject, predicate, object)` is **correct**: the sentence genuinely asserts that the subject has the claimed relation to the object, and both root resolutions are correct. When unsure, leave unchecked and add a one-line note in the Comments row. Update the **Tally** section at the bottom with your counts. Precision is defined as `correct / reviewed`.

---

## Fact sample

### Fact #1218

- Triple: `(жоба — does_to — қамту)`
- Predicate: `does_to`
- Pattern: `X Y-ні көзде-лайды`
- Source: `kazakh_textbooks_pack.json / kz_textbook_physics_11_ogn_p0068_s15`
- Sentence:

    > Жоба Нұр-сұлтан, Алматы қалаларындағы және ҚР-ның барлық облыс орталық- тарындағы көпқабатты үйлер мен коттедж құрылыс- тарын толық қамтуды көздеді

- [ ] Correct
- Comment:

### Fact #1366

- Triple: `(өткен — after — өркениет)`
- Predicate: `after`
- Pattern: `X Y-дан кейін`
- Source: `wikipedia_kz_pack.json / wiki_kz_0000542`
- Sentence:

    > Династиялар ауысқан сайын түрлі даму кезеңдерінен өткен ежелгі әкімшілік басқарудың баршылығы, өзге өркениеттен кейін қалып қойған көшпенді көршілер мен тау халықтарына қарағанда, жер өңдеу саласы дамыған экономикасының баршылығы анық артықшылық болды.

- [ ] Correct
- Comment:

### Fact #1440

- Triple: `(іргетас — has — үст)`
- Predicate: `has`
- Pattern: `X-тың Y-сы бар`
- Source: `wikipedia_kz_pack.json / wiki_kz_0001865`
- Sentence:

    > Іргетастың үстінде шөгінді қорабы бар, ол тұз асты, тұзды, тұз үсті кешендеріне бөлінеді, палеозой және мезокайнозой кезеңдерінде қалыптасқан, жыныстарының жалпы тереңдігі 12–14 километрден асады.

- [ ] Correct
- Comment:

### Fact #1770

- Triple: `(ереуіл — does_to — ардақ)`
- Predicate: `does_to`
- Pattern: `X Y-ні қара-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0007389`
- Sentence:

    > «Ереуіл атқа ер салмай», «Ұлы арман», «Жайықтың бойы көк шалғын», «Атадан туған ардақты ер» жырларында ақын алдағы күндеріне үмітпен, зор сеніммен қарады.

- [ ] Correct
- Comment:

### Fact #1799

- Triple: `(қабылан — related_to — аю)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0007774`
- Sentence:

    > Тау ормандарында марал, кербұғы, елік, жабайы шошқа, қабылан мен аюлар кездеседі.

- [ ] Correct
- Comment:

### Fact #1806

- Triple: `(түркия — does_to — күш)`
- Predicate: `does_to`
- Pattern: `X Y-ні қалыптас-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0007801`
- Sentence:

    > 1923 жылы республика ретінде құрылғаннан бері Түркия зайырлылықтың күшті дәстүрін қалыптастырды.

- [ ] Correct
- Comment:

### Fact #2280

- Triple: `(әлеуметтік — does_to — өнімділік)`
- Predicate: `does_to`
- Pattern: `X Y-ні айт-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0015964`
- Sentence:

    > Оған қоса ол Қазақстанның әлеуметтік жүйесін оңтайландыру, өнімділікті арттыру және жасыл экономиканы құру, бизнес жағдайын теңестіру, білімге көбірек инвестиция салу және басқаруды қадағалау туралы айтты.

- [ ] Correct
- Comment:

### Fact #2405

- Triple: `(асқан — does_to — хан)`
- Predicate: `does_to`
- Pattern: `X Y-ні жаса-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0017373`
- Sentence:

    > Есім ханның ел ішіндегі беделін, асқан абыройын іштей қызғанып жүретін Тұрсын Бұхара ханының ниетін тез түсініп, Сырдария жағасында екеуі астыртын келіссөз жүргізеді де, Есім ханды жоюдың жоспарын жасайды.

- [ ] Correct
- Comment:

### Fact #2414

- Triple: `(қазақ — does_to — халық)`
- Predicate: `does_to`
- Pattern: `X Y-ні ұшырат-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0017515`
- Sentence:

    > 18 ғасыр қазақ халқы үшін ел басына күн туған кезең болды, жоңғарлар тарапынан болатын шабуылдар халықты көп күйзеліске ұшыратты.

- [ ] Correct
- Comment:

### Fact #2767

- Triple: `(айша — goes_to — тіл)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0022114`
- Sentence:

    > Бірақ Айша тілге келмейді.

- [ ] Correct
- Comment:

### Fact #2887

- Triple: `(жент — does_to — қыпшақ)`
- Predicate: `does_to`
- Pattern: `X Y-ні талқанда-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0024173`
- Sentence:

    > Жент қаласынан Дешті қыпшақ даласына тереңдетіп соққы жасаған Атсыз (хорезмшахы) қыпшақтарды талқандайды.

- [ ] Correct
- Comment:

### Fact #3020

- Triple: `(-дүниежүзілік — after — соғыс)`
- Predicate: `after`
- Pattern: `X Y-дан кейін`
- Source: `wikipedia_kz_pack.json / wiki_kz_0026224`
- Sentence:

    > 2-дүниежүзілік соғыстан кейін халық толқулары қайтадан күшейді.

- [ ] Correct
- Comment:

### Fact #3231

- Triple: `(мұхит — goes_to — пішін)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0029779`
- Sentence:

    > Тау көтерілуіне байланысты олигоценде мұхиттар мен құрлықтар біртіндеп қазіргі пішінге келді.

- [ ] Correct
- Comment:

### Fact #3421

- Triple: `(өткен — does_to — мәселе)`
- Predicate: `does_to`
- Pattern: `X Y-ні уағдалас-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0033180`
- Sentence:

    > Дегенмен, 2007 жылы өткен Оңтүстік Азия өңірлік ынтымақтастық қауымдастығының саммитінде екі жақ шекара мәселелерін, сондай-ақ қауіпсіздік пен экономикалық дамуға қатысты мәселелерді бірлесіп шешуге уағдаласты [43].

- [ ] Correct
- Comment:

### Fact #3441

- Triple: `(грек — does_to — үн)`
- Predicate: `does_to`
- Pattern: `X Y-ні жүр-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0033522`
- Sentence:

    > арасында жағалауына грек, мысыр, қытай, үнді, араб теңіз жиһанкездері жиі-жиі келіп-кетіп жүреді.

- [ ] Correct
- Comment:

### Fact #3449

- Triple: `(ұлыбритания — does_to — бур)`
- Predicate: `does_to`
- Pattern: `X Y-ні ерік-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0033675`
- Sentence:

    > Ұлыбритания 19 ғасырдың 70-жылдары бурларды жаулауға кірісіп, нәтижесінде 1902 жылы Трансвааль мен Ерікті Қызғылт республикалары ағылшын отарына айналды.

- [ ] Correct
- Comment:

### Fact #3764

- Triple: `(шаруашылық — does_to — тағам)`
- Predicate: `does_to`
- Pattern: `X Y-ні ете-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0038810`
- Sentence:

    > Мал шаруашылық өңірлерінің ауа-райы суық, теңіз деңгейінен биік болғандықтан, олар жылдың төрт маусымында ет пен сүттен жасалған тағамдарды басты азық етеді.

- [ ] Correct
- Comment:

### Fact #3961

- Triple: `(бала — does_to — қала)`
- Predicate: `does_to`
- Pattern: `X Y-ні өзгер-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0041664`
- Sentence:

    > Немесе бала ұйықтай береді; Кейде талып қалады (тырысу), немесе дене қозғалысы да бір түрлі болып өзгереді; Көбінесе баланың жағдайы бірте-бірте нашарлай бастайды, ол мүлде есінен танған кезде ғана тынышталады; Туберкулез менингиті өте жай өрбиді, ол талай күндерге немесе апталарға созылады.

- [ ] Correct
- Comment:

### Fact #4348

- Triple: `(ақш — related_to — куба)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0047909`
- Sentence:

    > 1977 — АҚШ пен Куба дипломаттық өкілдерімен алмасу келісімін жасады (1 қыркүйектен бері).

- [ ] Correct
- Comment:

### Fact #4833

- Triple: `(қазақ — related_to — түркістан)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0055491`
- Sentence:

    > реформаларға сәйкес қазақтар мен Түркістан өлкесіндегі өзге де мұсылман халықтарының діни істері ешқандай діни басқарма қарамағына қаратылмай, жергілікті орт.

- [ ] Correct
- Comment:

### Fact #4880

- Triple: `(классикалық — does_to — көле)`
- Predicate: `does_to`
- Pattern: `X Y-ні айғақта-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0056391`
- Sentence:

    > Классикалық араб лексикографиясының қағидалары бойынша дайындалған бұл сөздік Қашғарлы Махмұттың түркі тілі туралы білімінің ауқымдылығын ғана емес, араб филологиясы ғылымы бойынша дайындығының да көлемді екендігін айғақтайды.

- [ ] Correct
- Comment:

### Fact #5009

- Triple: `(жазушы — goes_to — дүние)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0058648`
- Sentence:

    > Жазушы Қабдеш Жұмаділов Тарбағатай тауының күнгей бетіндегі Малдыбай бұлағының бойында дүниеге келді.

- [ ] Correct
- Comment:

### Fact #5563

- Triple: `(үгедей — does_to — қытай)`
- Predicate: `does_to`
- Pattern: `X Y-ні аяқта-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0067131`
- Sentence:

    > 1232—34 жылы Үгедей мен Төле басқарған әскерлер Цзань империясын толық талқандап, Солтүстік Қытайды бағындыруды аяқтады.

- [ ] Correct
- Comment:

### Fact #5812

- Triple: `(күрес — related_to — үміт)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0071508`
- Sentence:

    > Бүгінде айтулы мемориалдық кешен «Қайғы қақпасы» монументі, «Күрес пен үміт» және «Ашыну мен дәрменсіздік» секілді екі мүсіндік композициядан, сондай-ақ «Еске алу қабырғасынан» құралған.

- [ ] Correct
- Comment:

### Fact #5825

- Triple: `(қайырымдылық — related_to — өсиет)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0071726`
- Sentence:

    > Холдингтер Үшінші республикадан бері қайырымдылық пен өсиет арқылы тұрақты өсті.

- [ ] Correct
- Comment:

### Fact #5850

- Triple: `(қазақ — does_to — түсінік)`
- Predicate: `does_to`
- Pattern: `X Y-ні түсіндір-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0072286`
- Sentence:

    > Алда әңгімемізге арқау болатын оқиғалар шоғырының қазақ оқырмандарына түсінікті болуы үшін ең алдымен осы сөздердің біздің тілімізде қандай мағыналарға ие болатыны туралы түсіндірме сөздік түрінде қысқаша болса да түсінік бере кеткенді жөн көрдік.

- [ ] Correct
- Comment:

### Fact #5883

- Triple: `(климаттық — does_to — жағдай)`
- Predicate: `does_to`
- Pattern: `X Y-ні екпе-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0072768`
- Sentence:

    > Теріс климаттық жағдайларды болдырмау үшін қала аумағының қажетті микроклиматын құрайтын, қорғайтын екпе ағаштары кешенін құру керек.

- [ ] Correct
- Comment:

### Fact #6089

- Triple: `(үйрек — related_to — тауық)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0076460`
- Sentence:

    > Мажарстан Мажарстанда дастарқанға үйрек пен тауық еті қойылмайды.

- [ ] Correct
- Comment:

### Fact #6621

- Triple: `(өсетін — goes_to — өзен)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0085383`
- Sentence:

    > Жай өсетін нәсіл негізінен өзендерді және көлдің суы аз шығанақтарын мекендейді, ал тез өсетін нәсіл терең суларды мекендейді және өзендерге тек уылдырық шашу үшін келеді.

- [ ] Correct
- Comment:

### Fact #6837

- Triple: `(науқас — goes_to — жұмыс)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0089147`
- Sentence:

    > Науқас жүдеп, ашуланшақ келеді, жұмысқа қабілеті төмендейді.

- [ ] Correct
- Comment:

### Fact #7105

- Triple: `(ақша — does_to — ат)`
- Predicate: `does_to`
- Pattern: `X Y-ні көр-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0094087`
- Sentence:

    > Кейнстің «Ақша реформасы туралы трактат» атты еңбегі жарық көрді, онда автор Англия банкінің саясатымен келіспейді.

- [ ] Correct
- Comment:

### Fact #7583

- Triple: `(бүкіл — does_to — билік)`
- Predicate: `does_to`
- Pattern: `X Y-ні жинақта-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0102318`
- Sentence:

    > Өз қолына бүкіл билікті жинақтаған Цезарь іс жүзінде басқарудың римдік респ.

- [ ] Correct
- Comment:

### Fact #8239

- Triple: `(көз — does_to — жұлдыз)`
- Predicate: `does_to`
- Pattern: `X Y-ні қос-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0113515`
- Sentence:

    > Қырағы көз мамандар болашақ «жұлдызды» дер кезінде байқап, құрама сапына қосты.

- [ ] Correct
- Comment:

### Fact #8482

- Triple: `(өнімдері — is_a — күріш)`
- Predicate: `is_a`
- Pattern: `X — Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0117263`
- Sentence:

    > Өнімдері — арахис, жүгері, күріш, мақта.

- [ ] Correct
- Comment:

### Fact #8496

- Triple: `(ескерткіш — does_to — атақ)`
- Predicate: `does_to`
- Pattern: `X Y-ні әңгімеле-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0117506`
- Sentence:

    > «Яссауи мазасы» (2016) тарихи ескерткіш туралы әйгілі ислам ғалымының мұрасы туралы баяндайды «Әбу Ханифа» (2016) атақты ғалымның өмірі, адамгершілік қасиеттері мен жетістіктері туралы әңгімелейді «Ғибратты ғұмырлар» (2017) Қазақстанның рухани және мәдени өміріне ерекше үлес қосқан белгілі тұлғалар туралы әңгімелейді.

- [ ] Correct
- Comment:

### Fact #9075

- Triple: `(ұлттық — goes_to — ұлан)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0129144`
- Sentence:

    > Бірақ ұлттық құраманың бас бапкері Тұрсынғали Еділов Бекзаттың көзінде от барын байқап, ел намысын қорғауды Түркістандық жас ұланға сеніп тапсырады.

- [ ] Correct
- Comment:

### Fact #9130

- Triple: `(торғай — related_to — ырғыз)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0130323`
- Sentence:

    > ==Қажетті қорғау шаралары== Іле, Қара Ертіс өзендерінің атыраптарында, Торғай мен Ырғыз өзендерінің аңғарларындағы көлдерде қорықтар ұйымдастыру.

- [ ] Correct
- Comment:

### Fact #9329

- Triple: `(еңбек — does_to — іздеу)`
- Predicate: `does_to`
- Pattern: `X Y-ні жұм-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0133696`
- Sentence:

    > Сондықтан еңбек нарығында белгілі ағымдар қалыптасады: жұмысшы күші құрамынан шығушылар, жұмысшы күші құрамына кірушілер; жұмыс іздеуден бас тартқандар; жұмыс іздеуді бітіргендер; жұмыс тапқандар және т.

- [ ] Correct
- Comment:

### Fact #9850

- Triple: `(бұзу — goes_to — құқық)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0142240`
- Sentence:

    > Ресми түрде белгіленген және бекітілген сызығын мемлекеттердің бір жақты бұзуы немесе қайта қарауы халықаралық құқыққа қайшы келеді.

- [ ] Correct
- Comment:

### Fact #9870

- Triple: `(нығайту — does_to — ауру)`
- Predicate: `does_to`
- Pattern: `X Y-ні жұм-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0142774`
- Sentence:

    > Алғашқы кезде медицина ғылымы өзінің басты міндеті – денсаулықты сақтау, нығайту деп қараған, ал ауруды емдеуді қосалқы жұмыс ретінде санаған.

- [ ] Correct
- Comment:

### Fact #9952

- Triple: `(идея — does_to — материал)`
- Predicate: `does_to`
- Pattern: `X Y-ні ғыл-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0144767`
- Sentence:

    > Мұның өзі көптеген тың идеялар мен материалдарды ғыл.

- [ ] Correct
- Comment:

### Fact #10326

- Triple: `(түр — does_to — студенттік)`
- Predicate: `does_to`
- Pattern: `X Y-ні жүкте-лайды`
- Source: `synthetic_sentences_pack.json / synth_00588`
- Sentence:

    > түр -кенің студенттікті жүктеді себебі қандай туылады.

- [ ] Correct
- Comment:

### Fact #10999

- Triple: `(физикалық — does_to — мәжбүр)`
- Predicate: `does_to`
- Pattern: `X Y-ні сәл-лайды`
- Source: `synthetic_sentences_pack.json / synth_04460`
- Sentence:

    > физикалық жалптың мәжбүрді сәледі.

- [ ] Correct
- Comment:

### Fact #11863

- Triple: `(сайт — does_to — тый)`
- Predicate: `does_to`
- Pattern: `X Y-ні соқ-лайды`
- Source: `synthetic_sentences_pack.json / synth_09326`
- Sentence:

    > экономикалықта сайт тыйды соқты.

- [ ] Correct
- Comment:

### Fact #12286

- Triple: `(піші — does_to — сарайшық)`
- Predicate: `does_to`
- Pattern: `X Y-ні қыдыр-лайды`
- Source: `synthetic_sentences_pack.json / synth_11841`
- Sentence:

    > піші сарайшықты қыдырды бірақ ертең жұмайды.

- [ ] Correct
- Comment:

### Fact #12414

- Triple: `(бөлшек — does_to — киел)`
- Predicate: `does_to`
- Pattern: `X Y-ні кеңес-лайды`
- Source: `synthetic_sentences_pack.json / synth_12668`
- Sentence:

    > бөлшек киелді кеңесті әрі бірден біледі.

- [ ] Correct
- Comment:

### Fact #12858

- Triple: `(ара — does_to — жүк)`
- Predicate: `does_to`
- Pattern: `X Y-ні тынық-лайды`
- Source: `synthetic_sentences_pack.json / synth_15141`
- Sentence:

    > жаман ара жүкті тынықты.

- [ ] Correct
- Comment:

### Fact #12903

- Triple: `(сұхбат — does_to — ақш-)`
- Predicate: `does_to`
- Pattern: `X Y-ні жұмса-лайды`
- Source: `synthetic_sentences_pack.json / synth_15380`
- Sentence:

    > тоғыз сұхбат ақш-ды жұмсайды.

- [ ] Correct
- Comment:

### Fact #13095

- Triple: `(шың — does_to — қуаныш)`
- Predicate: `does_to`
- Pattern: `X Y-ні сала-лайды`
- Source: `synthetic_sentences_pack.json / synth_16404`
- Sentence:

    > шың мүліктің қуанышты салады бірақ мұсылман атанады.

- [ ] Correct
- Comment:

### Fact #13098

- Triple: `(қақпақ — does_to — дәулет)`
- Predicate: `does_to`
- Pattern: `X Y-ні тау-лайды`
- Source: `synthetic_sentences_pack.json / synth_16431`
- Sentence:

    > біраз қақпақ дәулетті тауды бірақ негізгі ұрпағ кірейді.

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

### Derivation #16

- Triple: `(-қа — is_a — көшбасшы)`
- Rule: `R1_is_a_transitivity`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0103523, wikipedia_kz_pack.json/wiki_kz_0047666

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

- Triple: `(қазақстан — related_to — қызылжар)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0135507, wikipedia_kz_pack.json/wiki_kz_0047327

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #127

- Triple: `(ауғанстан — related_to — үндістан)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0026147, wikipedia_kz_pack.json/wiki_kz_0004297

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #128

- Triple: `(ауғанстан — related_to — ұлыбритания)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0026147, wikipedia_kz_pack.json/wiki_kz_0027741

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #136

- Triple: `(арыс — related_to — ақмешіт)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0110255, wikipedia_kz_pack.json/wiki_kz_0055603

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #139

- Triple: `(арыс — related_to — қазақ)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0110255, wikipedia_kz_pack.json/wiki_kz_0001219

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #140

- Triple: `(ақмешіт — related_to — сәтбаев)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0055603, wikipedia_kz_pack.json/wiki_kz_0098675

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #143

- Triple: `(сәтбаев — related_to — талғар)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0098675, wikipedia_kz_pack.json/wiki_kz_0110712

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #147

- Triple: `(дуадақ — related_to — жағалтай)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0130217, wikipedia_kz_pack.json/wiki_kz_0135186

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #150

- Triple: `(әсел — related_to — өрік)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0146217, wikipedia_kz_pack.json/wiki_kz_0081700

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #152

- Triple: `(сәуір — is_a — өкіметі)`
- Rule: `R1_is_a_transitivity`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0009181, wikipedia_kz_pack.json/wiki_kz_0009182

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #156

- Triple: `(қаңтар — has — астанадағ)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0009192, wikipedia_kz_pack.json/wiki_kz_0141921

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #159

- Triple: `(жыл — has — көбі)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0046286, wikipedia_kz_pack.json/wiki_kz_0027741

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #161

- Triple: `(жыл — has — солтүстік-батыс)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0046286, wikipedia_kz_pack.json/wiki_kz_0027741

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #165

- Triple: `(жыл — has — халқ)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0046286, wikipedia_kz_pack.json/wiki_kz_0027741

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #170

- Triple: `(-қа — has — ара)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0103523, wikipedia_kz_pack.json/wiki_kz_0027741

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #176

- Triple: `(-қа — has — таул)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0103523, wikipedia_kz_pack.json/wiki_kz_0027741

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #181

- Triple: `(-қа — related_to — абай)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0103523, wikipedia_kz_pack.json/wiki_kz_0088327

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #191

- Triple: `(абай — related_to — алматы)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0088327, wikipedia_kz_pack.json/wiki_kz_0127358

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #193

- Triple: `(еңбек — related_to — қайнар)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: kazakh_proverbs_pack.json/proverb_068, wikipedia_kz_pack.json/wiki_kz_0139793

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #196

- Triple: `(-қа — related_to — ауғанстан)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0103523, wikipedia_kz_pack.json/wiki_kz_0026147

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #197

- Triple: `(-қа — related_to — үндістан)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0103523, wikipedia_kz_pack.json/wiki_kz_0004297

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #198

- Triple: `(-қа — related_to — ұлыбритания)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0103523, wikipedia_kz_pack.json/wiki_kz_0027741

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #200

- Triple: `(ақпан — related_to — қыркүйек)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0001080, wikipedia_kz_pack.json/wiki_kz_0009178

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #201

- Triple: `(желтоқсан — related_to — сәуір)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0001072, wikipedia_kz_pack.json/wiki_kz_0009181

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #203

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

- `X Y-дан кейін`: 2
- `X Y-ке барады`: 7
- `X Y-ні айт-лайды`: 1
- `X Y-ні айғақта-лайды`: 1
- `X Y-ні аяқта-лайды`: 1
- `X Y-ні екпе-лайды`: 1
- `X Y-ні ерік-лайды`: 1
- `X Y-ні ете-лайды`: 1
- `X Y-ні жаса-лайды`: 1
- `X Y-ні жинақта-лайды`: 1
- `X Y-ні жүкте-лайды`: 1
- `X Y-ні жүр-лайды`: 1
- `X Y-ні жұм-лайды`: 2
- `X Y-ні жұмса-лайды`: 1
- `X Y-ні кеңес-лайды`: 1
- `X Y-ні көзде-лайды`: 1
- `X Y-ні көр-лайды`: 1
- `X Y-ні сала-лайды`: 1
- `X Y-ні соқ-лайды`: 1
- `X Y-ні сәл-лайды`: 1
- `X Y-ні талқанда-лайды`: 1
- `X Y-ні тау-лайды`: 1
- `X Y-ні тынық-лайды`: 1
- `X Y-ні түсіндір-лайды`: 1
- `X Y-ні уағдалас-лайды`: 1
- `X Y-ні ғыл-лайды`: 1
- `X Y-ні қалыптас-лайды`: 1
- `X Y-ні қара-лайды`: 1
- `X Y-ні қос-лайды`: 1
- `X Y-ні қыдыр-лайды`: 1
- `X Y-ні ұшырат-лайды`: 1
- `X Y-ні әңгімеле-лайды`: 1
- `X Y-ні өзгер-лайды`: 1
- `X пен Y`: 7
- `X — Y`: 1
- `X-тың Y-сы бар`: 1

Sampled derivations by rule:

- `R1_is_a_transitivity`: 8
- `R2_has_inheritance`: 24
- `R5_shared_is_a_target`: 18
