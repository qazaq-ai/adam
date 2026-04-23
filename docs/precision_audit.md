# Precision audit — native-speaker review

**Target:** 50-fact sample + 50-derivation sample from the committed artifacts, seed `42`.

- `facts.json`: 13889 facts total (upstream status: `completed`) — sampled 50 here.
- `derived_facts.json`: 6579 derivations total (upstream status: `completed`) — sampled 50 here.

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

- Triple: `(жыл — after — уақыт)`
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

- Triple: `(моңғол — does_to — иран)`
- Predicate: `does_to`
- Pattern: `X Y-ні ал-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0018172`
- Sentence:

    > моңғол әскерлеріның басшылары Жебе мен Сүбедей нояндар басқарған 30 мыңдық жасақ Солтүстік Иранды басып алды.

- [ ] Correct
- Comment:

### Fact #2414

- Triple: `(мұқағали — goes_to — дүние)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0018250`
- Sentence:

    > » Мұқағали Мақатаев атындағы әдеби сыйлықтың лауреаты Оразақын Асқар ақынның екінші ту­ған күніне байланысты мынадай сөз айта­ды: «Ал құжат бойынша Мұқағали 9 ақпан­да дүниеге келген.

- [ ] Correct
- Comment:

### Fact #2767

- Triple: `(солтүстік — does_to — иран)`
- Predicate: `does_to`
- Pattern: `X Y-ні ал-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0023187`
- Sentence:

    > Жеңіліске ұшыраған солтүстік ғұндардың бір бөлігі 5 ғасырда Оңтүстік Қазақстан мен Орта Азияда өз мемлекетін құрып, Ауғанстан мен Иранды, Үндістанның біраз бөлігін жаулап алды.

- [ ] Correct
- Comment:

### Fact #2887

- Triple: `(әйел — does_to — білім)`
- Predicate: `does_to`
- Pattern: `X Y-ні бітір-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0025129`
- Sentence:

    > Сайланған 77 депутаттың 8-і - әйелдер, 74-і – жоғары білімді, 30-ға жуығы екі жоғары оқу орындарын бітірген адамдар болды.

- [ ] Correct
- Comment:

### Fact #3020

- Triple: `(бөлігі — related_to — ұсақ)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0027639`
- Sentence:

    > Ұлыбритания (), немесе Біріккен Патшалық (, қысқартылған: ), толық ресми аталуы — Ұлыбритания және Солтүстік Ирландия Біріккен Патшалығы () — Еуропаның солтүстік-батысында, Британ аралдарында (ең ірісі — Ұлыбритания), Ирландия аралының солтүстік-шығыс бөлігі мен ұсақ аралдарда орналасқан мемлекет.

- [ ] Correct
- Comment:

### Fact #3231

- Triple: `(тағы — does_to — жер)`
- Predicate: `does_to`
- Pattern: `X Y-ні бұл-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0031489`
- Sentence:

    > Кейін тағы 3 айрық жол пайда болды：Юймынның батыс солтүстігінен басталып, қазіргі Құмыл, Тұрпан, Жемсары сияқты жерлерді басып, бұрынғы дала жолына тұтасатын жол, бұл кейінірек «солтүстік жол» деп аталды.

- [ ] Correct
- Comment:

### Fact #3421

- Triple: `(субэкваторлық — does_to — ылғал)`
- Predicate: `does_to`
- Pattern: `X Y-ні жаз-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0035007`
- Sentence:

    > Климаты субэкваторлық, ыстық және ылғалды, қысы құрғақ (қараша-сәуір) және ылғалды жаз мезгілі (мамыр-қазан).

- [ ] Correct
- Comment:

### Fact #3441

- Triple: `(самарқан — related_to — бұхара)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0035228`
- Sentence:

    > Осы кезде Самарқан мен Бұхара әмірі Сұлтан Ахмет мырза қайтыс болып, оның артында ұрпақ қалмады.

- [ ] Correct
- Comment:

### Fact #3449

- Triple: `(темір — goes_to — дүние)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0035350`
- Sentence:

    > Осы тұста, Темір әулетінде 1394 жылы 22 наурыз күні дүниеге Мұхаммед Тарағай деген бала келді.

- [ ] Correct
- Comment:

### Fact #3764

- Triple: `(ара — goes_to — дүние)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0040408`
- Sentence:

    > Сүтқоректілердің ұрығының дамуы кезінде оң жүрекше мен сол жүрекшенің арасы сопақша тесік (боталлов өзегі) арқылы байланысып тұрады, кейін ұрпақ дүниеге келген соң ол бітеліп қалады.

- [ ] Correct
- Comment:

### Fact #3961

- Triple: `(күш — related_to — америкалық)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0043641`
- Sentence:

    > Бірақ ол шаралар жергілікті кертартпа күштер мен америкалық компаниялар тарапынан қарсылыққа ұшырады.

- [ ] Correct
- Comment:

### Fact #4348

- Triple: `(ғылым — does_to — пікір)`
- Predicate: `does_to`
- Pattern: `X Y-ні айт-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0049919`
- Sentence:

    > Тек сөздерін ғылым жолына салып реттесе, ешбір жұрттың тілінен кем болмайды», - деген пікірді айтқан.

- [ ] Correct
- Comment:

### Fact #4833

- Triple: `(азия — related_to — қазақстан)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0057537`
- Sentence:

    > Орта Азия мен Қазақстан жеріне Бактрия мен Соғды елі уақытша бағындырылды.

- [ ] Correct
- Comment:

### Fact #4880

- Triple: `(итбалық — does_to — мұз)`
- Predicate: `does_to`
- Pattern: `X Y-ні көрсет-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0058448`
- Sentence:

    > Итбалық теңіздің бір кезде Солтүстік Мұзды мұхитпен байланыста болғанын көрсетеді.

- [ ] Correct
- Comment:

### Fact #5009

- Triple: `(жалпылық — does_to — секіл)`
- Predicate: `does_to`
- Pattern: `X Y-ні аш-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0060538`
- Sentence:

    > Бұл Жалпылық себеп, Айырым себеп, Нақты себеп, Астыртын себеп деген секілді маңызды ұғымдарды қалыптастыруға жол ашты.

- [ ] Correct
- Comment:

### Fact #5563

- Triple: `(тіл — after — жыл)`
- Predicate: `after`
- Pattern: `X Y-дан кейін`
- Source: `wikipedia_kz_pack.json / wiki_kz_0070047`
- Sentence:

    > Литван тілі мен латыш тілдерінің арасында елеулі айырмашылықтар 800 жылдан кейін пайдан бола бастаған, бірақ сонда да ұзақ уақытқа дейін оларды бір тілдің диалектілері ретінде қарастыруға болатын.

- [ ] Correct
- Comment:

### Fact #5812

- Triple: `(шығыс — lives_in — бөлігін)`
- Predicate: `lives_in`
- Pattern: `X Y-да тұрады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0073738`
- Sentence:

    > Уездің шығыс бөлігінде солармен қатар түрлі болыстарға бөлшектеліп кірген Қамбар руы тұрған.

- [ ] Correct
- Comment:

### Fact #5825

- Triple: `(үй — related_to — ақш)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0073920`
- Sentence:

    > Мұндай ғимараттардың ішінде Ақ үй мен АҚШ банкі де болды.

- [ ] Correct
- Comment:

### Fact #5850

- Triple: `(жыл — after — уақыт)`
- Predicate: `after`
- Pattern: `X Y-дан кейін`
- Source: `wikipedia_kz_pack.json / wiki_kz_0074492`
- Sentence:

    > 1979 жылы Оңтүстік Қазақстан ГРЭС құрылысы үшін алаң таңдалып, Үлкен кенті құрылды, алайда біраз уақыттан кейін жұмыстар тоқтатылды.

- [ ] Correct
- Comment:

### Fact #5883

- Triple: `(жыл — has — желтоқсан)`
- Predicate: `has`
- Pattern: `X-тың Y-сы бар`
- Source: `wikipedia_kz_pack.json / wiki_kz_0075406`
- Sentence:

    > 1930 жылдың 17 желтоқсаны мен 1933 жылдың 1 шілдесі аралығында Гурьев ауданы Батыс Қазақстан облысына қараса, 1933 жылдың 1 шілдесінен құрамында 4 ауданы бар Гурьев округі қайта құрылды.

- [ ] Correct
- Comment:

### Fact #6089

- Triple: `(негізі — does_to — жер)`
- Predicate: `does_to`
- Pattern: `X Y-ні жата-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0078870`
- Sentence:

    > Жасыл революцияның негізі болып табылатын басты мәселелерге — мәдени өсімдіктердің өнімділігін арттыратын және егістік жерлерді пайдалану мүмкіндігін кеңейтетін дақылдардың тез пісетін сорттарын шығару, суландыру шараларын ұлғайту жатады.

- [ ] Correct
- Comment:

### Fact #6621

- Triple: `(қор — related_to — материал)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0088313`
- Sentence:

    > Ондағы жиналған қорлар мен материалдар ақынның шығармашылық өміріне арналған.

- [ ] Correct
- Comment:

### Fact #6837

- Triple: `(бала — does_to — шөлмек)`
- Predicate: `does_to`
- Pattern: `X Y-ні сын-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0092328`
- Sentence:

    > |- | Бала шөлмекті сындырды.

- [ ] Correct
- Comment:

### Fact #7105

- Triple: `(тұмсық — lives_in — кезін)`
- Predicate: `lives_in`
- Pattern: `X Y-да тұрады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0097308`
- Sentence:

    > Глиссердің тұмсық жағы қозғалыс кезінде су үстінде көтеріліп тұрады да, сырғанап келе жатқан тәрізді көрінеді.

- [ ] Correct
- Comment:

### Fact #7583

- Triple: `(төбет — does_to — ас)`
- Predicate: `does_to`
- Pattern: `X Y-ні кіргіз-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0105618`
- Sentence:

    > Аидтың аяғының астында үш басты төбет отырады, ол жер асты патшалығына келгендерді кіргізеді де, бірақ одан ешкімді де шығармайтын болған.

- [ ] Correct
- Comment:

### Fact #8239

- Triple: `(мұз — goes_to — азия)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0116534`
- Sentence:

    > «Кіші мұз дәуірі» көшпенді халықтың Орта Азияға қоныс аударуына алып келді (шамамен 250-400 мың).

- [ ] Correct
- Comment:

### Fact #8482

- Triple: `(жер — related_to — өлке)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0121801`
- Sentence:

    > Хасановтың қаламынан туған жер мен өлке тарихына қатысты бірнеше кітаптары жарық көрген.

- [ ] Correct
- Comment:

### Fact #8496

- Triple: `(топырақ — does_to — құмдақ)`
- Predicate: `does_to`
- Pattern: `X Y-ні тас-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0122230`
- Sentence:

    > Топырақ жамылғысынан құмдақты қиыршық тасты сұр топырақ, шалшықты-батпақты келген солтүстігінде сор топырақ қалыптасқан.

- [ ] Correct
- Comment:

### Fact #9075

- Triple: `(бөліну — does_to — мәселе)`
- Predicate: `does_to`
- Pattern: `X Y-ні ата-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0133026`
- Sentence:

    > Кейбір зерттеулердің пайымдауынша, шешімдердің бөліну теорисы адамдар мен ұйымдардың күрделі мәселелерді тияанақты шешіп отыруға ұйытқы болатынын ерекше атайды.

- [ ] Correct
- Comment:

### Fact #9130

- Triple: `(солтүстік — does_to — ат)`
- Predicate: `does_to`
- Pattern: `X Y-ні толықтыр-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0134234`
- Sentence:

    > Солтүстік Қазақстан облысының тұрғыны Қанзада Қазыбаева мұражайға сыйға тартқан 19- ғасырдың аяғы 20- ғасырдың басында сол жердің нақышымен жасалған сақина «Қазақ халқының зергерлік бұйымдары» атты бөлімді толықтырды.

- [ ] Correct
- Comment:

### Fact #9329

- Triple: `(тұжырым — does_to — тәріз)`
- Predicate: `does_to`
- Pattern: `X Y-ні түсіндір-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0137499`
- Sentence:

    > Ол Бор қағидалары (постулаттары) деп аталған тұжырымдар негізінде атомның орнықтылығын және сутек тәрізді атомдардың спектрлік заңдылықтарын түсіндірді.

- [ ] Correct
- Comment:

### Fact #9850

- Triple: `(болу — does_to — мәлімет)`
- Predicate: `does_to`
- Pattern: `X Y-ні ете-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0147771`
- Sentence:

    > ) дәлдігін анықтау мағлұматтардың пайда болу жағдайын, нақты мәліметтерді тексеруді, автордың саяси көзқарасын ашуды қажет етеді.

- [ ] Correct
- Comment:

### Fact #9870

- Triple: `(қатынас — related_to — ұлт)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0148317`
- Sentence:

    > Негізгі ғылыми еңбектері тарихи прогресс, ұлттық қатынастар мен ұлт мәдениетін дамыту, ұлттық психология мәселелеріне арналған.

- [ ] Correct
- Comment:

### Fact #9952

- Triple: `(түр — goes_to — көтеру)`
- Predicate: `goes_to`
- Pattern: `X Y-ке барады`
- Source: `wikipedia_kz_pack.json / wiki_kz_0149664`
- Sentence:

    > Өндірісте грунттық сорғының екі түрі бар: 1) сорғы станциясы ғимаратында орналасқан тұрақты грунттық сорғы; 2) ұңғымалар мен құдықтардан су қоспасын көтеруге арналған ұңғымалық грунттық сорғы.

- [ ] Correct
- Comment:

### Fact #10326

- Triple: `(қызғалдақ — does_to — жіңіш)`
- Predicate: `does_to`
- Pattern: `X Y-ні лақтыр-лайды`
- Source: `synthetic_sentences_pack.json / synth_02022`
- Sentence:

    > кеше қызғалдақ жіңішті лақтырды әйтпесе ресми көле кеңеседі.

- [ ] Correct
- Comment:

### Fact #10999

- Triple: `(ғұн — does_to — жағасын)`
- Predicate: `does_to`
- Pattern: `X Y-ні гүлде-лайды`
- Source: `synthetic_sentences_pack.json / synth_06039`
- Sentence:

    > ғұн жағасынды гүлдеді немесе үстем міндетті болайды.

- [ ] Correct
- Comment:

### Fact #11863

- Triple: `(қабылдау — does_to — тайпалық)`
- Predicate: `does_to`
- Pattern: `X Y-ні есе-лайды`
- Source: `synthetic_sentences_pack.json / synth_11007`
- Sentence:

    > қабылдау қояның тайпалықты еседі себебі қаң өңдейді.

- [ ] Correct
- Comment:

### Fact #12286

- Triple: `(тір — does_to — атағын)`
- Predicate: `does_to`
- Pattern: `X Y-ні қат-лайды`
- Source: `synthetic_sentences_pack.json / synth_13690`
- Sentence:

    > ыстық тір атағынды қатты сөйтіп ешқашан қағды.

- [ ] Correct
- Comment:

### Fact #12414

- Triple: `(ғарыштық — does_to — жүгер)`
- Predicate: `does_to`
- Pattern: `X Y-ні өсе-лайды`
- Source: `synthetic_sentences_pack.json / synth_14439`
- Sentence:

    > ғарыштық бостандықтың жүгерді өсейді.

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

- Triple: `(ұйымдастырушы — does_to — сәлем)`
- Predicate: `does_to`
- Pattern: `X Y-ні шау-лайды`
- Source: `synthetic_sentences_pack.json / synth_17266`
- Sentence:

    > ұйымдастырушы ақмолаға нақты сәлемді шауды.

- [ ] Correct
- Comment:

### Fact #13095

- Triple: `(салмақ — does_to — хандық)`
- Predicate: `does_to`
- Pattern: `X Y-ні ете-лайды`
- Source: `synthetic_sentences_pack.json / synth_18389`
- Sentence:

    > суық салмақ хандықты етеді.

- [ ] Correct
- Comment:

### Fact #13098

- Triple: `(дәулет — does_to — көйлек)`
- Predicate: `does_to`
- Pattern: `X Y-ні тынық-лайды`
- Source: `synthetic_sentences_pack.json / synth_18398`
- Sentence:

    > дәл дәулет көйлекті тынықты.

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

### Derivation #645

- Triple: `(температура — related_to — тіл)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/biology_basic.jsonl/bio_036, world_core/proverbs.jsonl/prov_006

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

### Derivation #1290

- Triple: `(елу — related_to — сексен)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/numbers.jsonl/num_015, world_core/numbers.jsonl/num_018

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

### Derivation #2348

- Triple: `(бағдарлама — goes_to — орталық азия)`
- Rule: `R7_goes_to_via_part_of`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0051743, world_core/geography_kz.jsonl/geo_kz_002

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

- Triple: `(күріш — has — түр)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: world_core/food.jsonl/food_033, world_core/food.jsonl/food_037

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #2683

- Triple: `(жыл — has — ауыз)`
- Rule: `R2_has_inheritance`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0046261, world_core/proverbs.jsonl/prov_006

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #2727

- Triple: `(абай — related_to — бұқар)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/kz_literature.jsonl/lit_001, world_core/kz_literature.jsonl/lit_018

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #2958

- Triple: `(мақал — related_to — эпос)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/kz_literature.jsonl/lit_026, world_core/kz_literature.jsonl/lit_055

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #3205

- Triple: `(бұғы — related_to — сауысқан)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/animals.jsonl/anm_008, world_core/animals.jsonl/anm_012

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #3343

- Triple: `(жәндік — related_to — көбелек)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/animals.jsonl/anm_029, world_core/animals.jsonl/anm_027

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #3380

- Triple: `(киік — related_to — үйрек)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/animals.jsonl/anm_009, world_core/animals.jsonl/anm_019

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #3653

- Triple: `(ана — related_to — ыбырай)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/proverbs.jsonl/prov_005, world_core/kz_literature.jsonl/lit_007

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4018

- Triple: `(алма — related_to — жая)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/food.jsonl/food_020, wikipedia_kz_pack.json/wiki_kz_0058346

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4041

- Triple: `(балық — related_to — жүзім)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/food.jsonl/food_016, world_core/food.jsonl/food_021

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4059

- Triple: `(бауырсақ — related_to — картоп)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/food.jsonl/food_011, world_core/food.jsonl/food_031

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4363

- Triple: `(арқар — related_to — әсел)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/animals.jsonl/anm_010, wikipedia_kz_pack.json/wiki_kz_0146217

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4373

- Triple: `(ағаш — related_to — жәндік)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/biology_basic.jsonl/bio_014, world_core/animals.jsonl/anm_029

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4388

- Triple: `(ағаш — related_to — өсімдік)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/biology_basic.jsonl/bio_014, world_core/biology_basic.jsonl/bio_013

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4556

- Triple: `(сүтқоректі — related_to — өрік)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/animals.jsonl/anm_036, wikipedia_kz_pack.json/wiki_kz_0081700

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4603

- Triple: `(сары — related_to — түндік)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/colors.jsonl/color_003, wikipedia_kz_pack.json/wiki_kz_0056466

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4695

- Triple: `(кезеңі — related_to — сия)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0083073, world_core/colors.jsonl/color_019

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4700

- Triple: `(кезеңі — related_to — қара)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0083073, world_core/colors.jsonl/color_005

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4710

- Triple: `(кеш — related_to — сары)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/time.jsonl/time_012, world_core/colors.jsonl/color_003

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #4951

- Triple: `(жеті — related_to — жыл)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/numbers.jsonl/num_008, world_core/time.jsonl/time_019

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #5280

- Triple: `(білім — related_to — химия)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/proverbs.jsonl/prov_028, world_core/society.jsonl/soc_026

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #5314

- Triple: `(түндік — related_to — қазақ тілі)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0056466, world_core/society.jsonl/soc_039

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #5353

- Triple: `(ана — related_to — құмырсқа)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0069231, world_core/animals.jsonl/anm_025

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #5361

- Triple: `(аю — related_to — халқы)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/animals.jsonl/anm_005, wikipedia_kz_pack.json/wiki_kz_0012411

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #5397

- Triple: `(халқы — related_to — үкі)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: wikipedia_kz_pack.json/wiki_kz_0012411, world_core/animals.jsonl/anm_016

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #5517

- Triple: `(жамбыл — related_to — мұхтар)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/kz_literature.jsonl/lit_004, world_core/kz_literature.jsonl/lit_008

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #5643

- Triple: `(ағаш — related_to — тырна)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/biology_basic.jsonl/bio_014, world_core/animals.jsonl/anm_014

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #5686

- Triple: `(бүркіт — related_to — жануар)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/animals.jsonl/anm_013, world_core/biology_basic.jsonl/bio_012

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #5851

- Triple: `(тары — related_to — қасқыр)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/food.jsonl/food_036, wikipedia_kz_pack.json/wiki_kz_0041416

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #6037

- Triple: `(алтын — related_to — нөл)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/colors.jsonl/color_011, world_core/numbers.jsonl/num_001

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

### Derivation #6043

- Triple: `(алтын — related_to — тақ)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: world_core/colors.jsonl/color_011, world_core/numbers.jsonl/num_029

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
- `X Y-дан кейін`: 3
- `X Y-ке барады`: 5
- `X Y-ні айт-лайды`: 1
- `X Y-ні ал-лайды`: 2
- `X Y-ні ата-лайды`: 1
- `X Y-ні аш-лайды`: 1
- `X Y-ні біл-лайды`: 1
- `X Y-ні бітір-лайды`: 1
- `X Y-ні бұл-лайды`: 1
- `X Y-ні гүлде-лайды`: 1
- `X Y-ні есе-лайды`: 1
- `X Y-ні ете-лайды`: 2
- `X Y-ні жаз-лайды`: 1
- `X Y-ні жата-лайды`: 1
- `X Y-ні кір-лайды`: 1
- `X Y-ні кіргіз-лайды`: 1
- `X Y-ні көрсет-лайды`: 1
- `X Y-ні лақтыр-лайды`: 1
- `X Y-ні сын-лайды`: 1
- `X Y-ні тас-лайды`: 1
- `X Y-ні тақ-лайды`: 1
- `X Y-ні толықтыр-лайды`: 1
- `X Y-ні тынық-лайды`: 1
- `X Y-ні түсіндір-лайды`: 1
- `X Y-ні шау-лайды`: 1
- `X Y-ні қайта-лайды`: 1
- `X Y-ні қалдыр-лайды`: 1
- `X Y-ні қат-лайды`: 1
- `X Y-ні өсе-лайды`: 1
- `X пен Y`: 9
- `X-тың Y-сы бар`: 1

Sampled derivations by rule:

- `R1_is_a_transitivity`: 3
- `R2_has_inheritance`: 4
- `R3_has_inheritance_via_part_of`: 1
- `R5_shared_is_a_target`: 39
- `R7_goes_to_via_part_of`: 3
