# Precision audit — native-speaker review

**Target:** 50-fact sample + 50-derivation sample from the committed artifacts, seed `42`.

- `facts.json`: 251 facts total (upstream status: `completed`) — sampled 50 here.
- `derived_facts.json`: 1 derivations total (upstream status: `completed`) — sampled 1 here.

## How to review

For each fact, mark the checkbox if the triple `(subject, predicate, object)` is **correct**: the sentence genuinely asserts that the subject has the claimed relation to the object, and both root resolutions are correct. When unsure, leave unchecked and add a one-line note in the Comments row. Update the **Tally** section at the bottom with your counts. Precision is defined as `correct / reviewed`.

---

## Fact sample

### Fact #1

- Triple: `(әсем — does_to — ыдыс)`
- Predicate: `does_to`
- Pattern: `X Y-ні сын-лайды`
- Source: `tatoeba_kazakh_pack.json / tatoeba_kz_00402`
- Sentence:

    > Сол түнде, Әсем бес ыдысты сындырды.

- [ ] Correct
- Comment:

### Fact #2

- Triple: `(қауіпсіздік — does_to — сияқ)`
- Predicate: `does_to`
- Pattern: `X Y-ні кір-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0000060`
- Sentence:

    > Сонымен қатар Еуропадағы қауіпсіздік және ынтымақтастық ұйымына, Ұжымдық қауіпсіздік туралы шарт ұйымына, Шанхай Ынтымақтастық Ұйымына және Еуразиялық Экономикалық Қауымдастығы сияқты басқа да бірнеше халықаралық ұйымдардың құрамына кіреді.

- [ ] Correct
- Comment:

### Fact #17

- Triple: `(ұлттық — does_to — ұтым)`
- Predicate: `does_to`
- Pattern: `X Y-ні кел-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0000256`
- Sentence:

    > Олардың белсене қатысуы және қолдауымен ұлттық саясат және тұрғындардың бос уақытын ұтымды ұйымдастыру мәселелері оң шешімін тауып келеді.

- [ ] Correct
- Comment:

### Fact #36

- Triple: `(ұлт — related_to — ұлыс)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `wikipedia_kz_pack.json / wiki_kz_0000407`
- Sentence:

    > Елдің солтүстігін, Қиыр Шығысты саны жағынан аз ұлттар мен ұлыстар мекендейді.

- [ ] Correct
- Comment:

### Fact #38

- Triple: `(атап — does_to — сияқ)`
- Predicate: `does_to`
- Pattern: `X Y-ні үнде-лайды`
- Source: `wikipedia_kz_pack.json / wiki_kz_0000439`
- Sentence:

    > Атап айтқанда, Ресей мәдениеті Византиямен қатар, көршілес жатқан Болгария, Сербия, Армения, Грузия сияқты елдердің мәдениетімен үндестік тапты.

- [ ] Correct
- Comment:

### Fact #47

- Triple: `(дос — does_to — аяул)`
- Predicate: `does_to`
- Pattern: `X Y-ні сұра-лайды`
- Source: `common_voice_kk_pack.json / cv_kk_00340`
- Sentence:

    > Айырылар дос аяулыңды сұрайды.

- [ ] Correct
- Comment:

### Fact #50

- Triple: `(қазақстан — does_to — қадағалау)`
- Predicate: `does_to`
- Pattern: `X Y-ні бол-лайды`
- Source: `cc100_kk_pack.json / cc100_kk_0000006`
- Sentence:

    > 1992 жылғы 17 қаңтарда Қазақстан Республикасы Жоғарғы Кеңесінің Қаулысымен «Қазақстан Республикасының прокуратурасы туралы» Заң күшіне еніп, соған сәйкес прокуратура Жоғарғы Кеңеске есеп беретін, заңдардың орындалуына жоғарғы қадағалауды жүзеге асыратын орган болды.

- [ ] Correct
- Comment:

### Fact #52

- Triple: `(павлодар — does_to — зат)`
- Predicate: `does_to`
- Pattern: `X Y-ні өткіз-лайды`
- Source: `cc100_kk_pack.json / cc100_kk_0000035`
- Sentence:

    > 2018 жылғы 2 қарашада Павлодар медициналық жоғары колледжінде СӨС қолдау орталығы ЖШС психологы К.Ж. Байгожина психоактивті заттарды қолданудың алдын алуы бойынша семинар тренингі өткізді. Тренингке «Стоматология» және»Ортопедиялық Стоматология» мамандықтарының студенттері

- [ ] Correct
- Comment:

### Fact #58

- Triple: `(сұрақ — related_to — жауап)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `cc100_kk_pack.json / cc100_kk_0000097`
- Sentence:

    > Соңғы сұрақтар мен жауаптар Халықтық емдеу - Сұрағым бар сайты

- [ ] Correct
- Comment:

### Fact #66

- Triple: `(алматы — does_to — ағай)`
- Predicate: `does_to`
- Pattern: `X Y-ні қат-лайды`
- Source: `cc100_kk_pack.json / cc100_kk_0000200`
- Sentence:

    > Алматы облыстық сотында Көксу ауданының тұрғындары, мал сатумен айналысқан ағайынды кісілердің өліміне қатысты қылмыстық істің алдын ала тыңдауы өтті. Ағайынды үш ер адам - Жарқынбек Долданұлы, Мұқият Зарлықанұлы мен Қадылбек Әлімхан осы күзде жоғалып, кейін өлі күйінде табылған.

- [ ] Correct
- Comment:

### Fact #67

- Triple: `(ақш — does_to — құжат)`
- Predicate: `does_to`
- Pattern: `X Y-ні айт-лайды`
- Source: `cc100_kk_pack.json / cc100_kk_0000202`
- Sentence:

    > АҚШ президенті Барак Обама елдің екі жылдық бюджеті туралы заңға қол қойды. Құжатты өткен аптада конгресс мақұлдаған. Ақ үйде заңға қол қою рәсімінде Обама конгресмендер енді министрліктер мен ведомстволарға қажетті қаржыны бөлу туралы заңдар қабылдауы қажетін айтты.

- [ ] Correct
- Comment:

### Fact #70

- Triple: `(мемлекет — does_to — қатынас)`
- Predicate: `does_to`
- Pattern: `X Y-ні қой-лайды`
- Source: `cc100_kk_pack.json / cc100_kk_0000235`
- Sentence:

    > Мемлекет басшысы төлем нарығы саласындағы қоғамдық қатынастарды реттеуге бағытталған «Төлемдер және төлем жүйелері туралы» Қазақстан Республикасының Заңына қол қойды, деп хабарлады Ақорданың баспасөз қызметі.

- [ ] Correct
- Comment:

### Fact #75

- Triple: `(өзара — does_to — тиім)`
- Predicate: `does_to`
- Pattern: `X Y-ні ұсын-лайды`
- Source: `cc100_kk_pack.json / cc100_kk_0000263`
- Sentence:

    > Өз кезегінде, К.Расулзода Қазақстан-Тәжікстан ынтымақтастығының деңгейіне жоғары баға беріп, мемлекетаралық қатынастардың барлық спектрі бойынша өзара тиімді серіктестікті одан әрі дамытуды ұсынды.

- [ ] Correct
- Comment:

### Fact #88

- Triple: `(топ — does_to — сүю)`
- Predicate: `does_to`
- Pattern: `X Y-ні үйрен-лайды`
- Source: `cc100_kk_pack.json / cc100_kk_0000362`
- Sentence:

    > Тәрбиелілік: Топ мүшелері сабаққа белсене араласады, өз елін жерін, тау - тасын сүюді үйренеді.

- [ ] Correct
- Comment:

### Fact #90

- Triple: `(жаттығу — does_to — мәтін)`
- Predicate: `does_to`
- Pattern: `X Y-ні оқы-лайды`
- Source: `cc100_kk_pack.json / cc100_kk_0000401`
- Sentence:

    > 28 - жаттығу. Мәтінді оқы. Қарамен жазылған сөздерді сөз құрамына талда. Бұл сөздер қандай сұраққа жауап береді?

- [ ] Correct
- Comment:

### Fact #95

- Triple: `(өткен — does_to — тыңайтқыш)`
- Predicate: `does_to`
- Pattern: `X Y-ні ал-лайды`
- Source: `cc100_kk_pack.json / cc100_kk_0000418`
- Sentence:

    > 1) ағымдағы жылы және өткен жылғы 4 (төртінші)-тоқсанда тыңайтқыштарды жеткізушіден және (немесе) тiкелей шетелдiк тыңайтқыштарды өндiрушiден сатып алынған тыңайтқыштарға (органикалық тыңайтқыштарды қоспағанда) жұмсалған шығындарды өтеу үшін ауыл шаруашылығы тауарын өндірушілерге;

- [ ] Correct
- Comment:

### Fact #96

- Triple: `(ауыл — does_to — жергілік)`
- Predicate: `does_to`
- Pattern: `X Y-ні бекіт-лайды`
- Source: `cc100_kk_pack.json / cc100_kk_0000424`
- Sentence:

    > Ауыл шаруашылығы тауарын өндіруші құны субсидия нормасына есептелген тыңайтқыштың құнынан жоғары тыңайтқыштарға өтінім берген жағдайда, субсидиялар облыстың, республикалық маңызы бар қаланың, астананың жергілікті атқарушы органы бекіткен 1 тоннаға (литрге, килограмға) арналған субсидиялар нормасы есептелген тыңайтқыш құны бойынша төленеді.

- [ ] Correct
- Comment:

### Fact #103

- Triple: `(дәме — does_to — жұрт)`
- Predicate: `does_to`
- Pattern: `X Y-ні қой-лайды`
- Source: `abai_wikisource_pack.json / abai_00196`
- Sentence:

    > Байлар жүр жиған малын қорғалатып, Өз жүзін, онын беріп, алар сатып, Онын алып, тоқсаннан дәме қылып, Бұл жұртты қойған жоқ па құдай атып?

- [ ] Correct
- Comment:

### Fact #106

- Triple: `(ат — does_to — жара)`
- Predicate: `does_to`
- Pattern: `X Y-ні күт-лайды`
- Source: `abai_wikisource_pack.json / abai_00253`
- Sentence:

    > Старшын, биді жиғыздым: «Береке қыл» деп, «бекін» деп, «Ат жарамды, үй жақсы Болсын, бәрің күтін» деп.

- [ ] Correct
- Comment:

### Fact #108

- Triple: `(өткен — does_to — дәурен)`
- Predicate: `does_to`
- Pattern: `X Y-ні қуала-лайды`
- Source: `abai_wikisource_pack.json / abai_00392`
- Sentence:

    > Мезгілі өткен дәуренді қуалаған, Не қылсын бір қартайған өу сүйекті.

- [ ] Correct
- Comment:

### Fact #112

- Triple: `(арық — does_to — мұрын)`
- Predicate: `does_to`
- Pattern: `X Y-ні қыл-лайды`
- Source: `abai_wikisource_pack.json / abai_00487`
- Sentence:

    > Арық қара, кең маңдай, қыр мұрынды, Екі көзден от жанған не қылған жас?

- [ ] Correct
- Comment:

### Fact #115

- Triple: `(ғылым — is_a — қазына)`
- Predicate: `is_a`
- Pattern: `X — Y`
- Source: `kazakh_proverbs_pack.json / proverb_007`
- Sentence:

    > Ғылым — таусылмас қазына.

- [ ] Correct
- Comment:

### Fact #117

- Triple: `(ақиқат — is_a — тірек)`
- Predicate: `is_a`
- Pattern: `X — Y`
- Source: `kazakh_proverbs_pack.json / proverb_049`
- Sentence:

    > Ақиқат — тіршіліктің тірегі.

- [ ] Correct
- Comment:

### Fact #118

- Triple: `(ынтымақ — is_a — байлық)`
- Predicate: `is_a`
- Pattern: `X — Y`
- Source: `kazakh_proverbs_pack.json / proverb_053`
- Sentence:

    > Ынтымақ — халықтың байлығы.

- [ ] Correct
- Comment:

### Fact #125

- Triple: `(сәуір — does_to — ғарыш)`
- Predicate: `does_to`
- Pattern: `X Y-ні шег-лайды`
- Source: `synthetic_sentences_pack.json / synth_00005`
- Sentence:

    > сәуір ғарышты шегді өйткені осылай түзілейді.

- [ ] Correct
- Comment:

### Fact #139

- Triple: `(суре — does_to — мөлшер)`
- Predicate: `does_to`
- Pattern: `X Y-ні ұр-лайды`
- Source: `synthetic_sentences_pack.json / synth_00063`
- Sentence:

    > суре мөлшерінді ұрады және түйе түкті ашады.

- [ ] Correct
- Comment:

### Fact #142

- Triple: `(еркі — does_to — төмендегідей)`
- Predicate: `does_to`
- Pattern: `X Y-ні емде-лайды`
- Source: `synthetic_sentences_pack.json / synth_00077`
- Sentence:

    > анық еркі төмендегідейді емдеді әйтпесе уәли жарқырады.

- [ ] Correct
- Comment:

### Fact #144

- Triple: `(дүниежүзілік — does_to — қызығушылық)`
- Predicate: `does_to`
- Pattern: `X Y-ні сің-лайды`
- Source: `synthetic_sentences_pack.json / synth_00079`
- Sentence:

    > қырық дүниежүзілік қызығушылықты сіңеді.

- [ ] Correct
- Comment:

### Fact #147

- Triple: `(зиян — does_to — мемлекет)`
- Predicate: `does_to`
- Pattern: `X Y-ні асыр-лайды`
- Source: `synthetic_sentences_pack.json / synth_00088`
- Sentence:

    > зиян мемлекетті асырды немесе мұра мекемені жалайды.

- [ ] Correct
- Comment:

### Fact #156

- Triple: `(киел — does_to — куәлік)`
- Predicate: `does_to`
- Pattern: `X Y-ні қалыптас-лайды`
- Source: `synthetic_sentences_pack.json / synth_00139`
- Sentence:

    > киел яғниден куәлікті қалыптастырды.

- [ ] Correct
- Comment:

### Fact #158

- Triple: `(толығырақ — does_to — аграрлық)`
- Predicate: `does_to`
- Pattern: `X Y-ні көшір-лайды`
- Source: `synthetic_sentences_pack.json / synth_00149`
- Sentence:

    > жүз толығырақ аграрлықты көшіреді.

- [ ] Correct
- Comment:

### Fact #159

- Triple: `(ғимарат — does_to — ғаламшар)`
- Predicate: `does_to`
- Pattern: `X Y-ні шал-лайды`
- Source: `synthetic_sentences_pack.json / synth_00151`
- Sentence:

    > ғимарат ғаламшарды шалды әйтпесе жіберу түнді ұсынылайды.

- [ ] Correct
- Comment:

### Fact #160

- Triple: `(көтеру — does_to — мәжіліс)`
- Predicate: `does_to`
- Pattern: `X Y-ні тігіл-лайды`
- Source: `synthetic_sentences_pack.json / synth_00157`
- Sentence:

    > көтеру болғадан мәжілісті тігілді.

- [ ] Correct
- Comment:

### Fact #162

- Triple: `(бәсеке — does_to — ғал)`
- Predicate: `does_to`
- Pattern: `X Y-ні қаша-лайды`
- Source: `synthetic_sentences_pack.json / synth_00166`
- Sentence:

    > сексен бәсеке ғалымды қашайды.

- [ ] Correct
- Comment:

### Fact #173

- Triple: `(әуле — does_to — шаруашылығын)`
- Predicate: `does_to`
- Pattern: `X Y-ні жөнде-лайды`
- Source: `synthetic_sentences_pack.json / synth_00209`
- Sentence:

    > әуле жеңіл шаруашылығынды жөндеді.

- [ ] Correct
- Comment:

### Fact #181

- Triple: `(шіл — does_to — қайғыл)`
- Predicate: `does_to`
- Pattern: `X Y-ні сала-лайды`
- Source: `synthetic_sentences_pack.json / synth_00280`
- Sentence:

    > тұжырымда шіл қайғылды салады.

- [ ] Correct
- Comment:

### Fact #182

- Triple: `(жалақ — does_to — негізін)`
- Predicate: `does_to`
- Pattern: `X Y-ні айт-лайды`
- Source: `synthetic_sentences_pack.json / synth_00287`
- Sentence:

    > жалақ жүлдегерден негізінді айтты.

- [ ] Correct
- Comment:

### Fact #188

- Triple: `(жәңгір — does_to — жұбай)`
- Predicate: `does_to`
- Pattern: `X Y-ні жүс-лайды`
- Source: `synthetic_sentences_pack.json / synth_00317`
- Sentence:

    > жәңгір ильяні көшті сөйтіп қауымдастық жұбайды жүседі.

- [ ] Correct
- Comment:

### Fact #195

- Triple: `(түсіну — does_to — күш)`
- Predicate: `does_to`
- Pattern: `X Y-ні ұлас-лайды`
- Source: `synthetic_sentences_pack.json / synth_00381`
- Sentence:

    > пәнде түсіну күшінді ұласты.

- [ ] Correct
- Comment:

### Fact #206

- Triple: `(сәби — does_to — жалақ)`
- Predicate: `does_to`
- Pattern: `X Y-ні күшейт-лайды`
- Source: `synthetic_sentences_pack.json / synth_00453`
- Sentence:

    > сәби ұзын жалақты күшейтті.

- [ ] Correct
- Comment:

### Fact #210

- Triple: `(өзгеше — does_to — ұсақ)`
- Predicate: `does_to`
- Pattern: `X Y-ні күзет-лайды`
- Source: `synthetic_sentences_pack.json / synth_00484`
- Sentence:

    > өзгеше дәннің ұсақты күзетті бірақ тыңайтқыш қабайды.

- [ ] Correct
- Comment:

### Fact #221

- Triple: `(қауым — does_to — күмән)`
- Predicate: `does_to`
- Pattern: `X Y-ні қос-лайды`
- Source: `kazakh_textbooks_pack.json / kz_textbook_kz_lang_11_emn_p0008_s03`
- Sentence:

    > Ойталқыға жиналған қауым өз көңілін- дегі күмәнді, екіұшты ойларын нақтылау, ортақ шешімін қарастыру, мәселенің шынайы мүмкіндіктерін іздестіру мақсатында бас қосады

- [ ] Correct
- Comment:

### Fact #223

- Triple: `(ақтабан — does_to — шұбырын)`
- Predicate: `does_to`
- Pattern: `X Y-ні ұшыра-лайды`
- Source: `kazakh_textbooks_pack.json / kz_textbook_kz_lang_11_emn_p0010_s29`
- Sentence:

    > Ақтабан шұбырынды, Алқакөл сұлама «Ақтабан шұбырынды, Алқакөл сұлама» - қазақ хал- қының жоңғар шапқыншылығы (1728-1725) салдарынан басынан кешкен ауыр қасіретін, босқыншылыққа ұшыраған кезеңін бейнелейтін ұғым

- [ ] Correct
- Comment:

### Fact #224

- Triple: `(қазақ — does_to — шұбырын)`
- Predicate: `does_to`
- Pattern: `X Y-ні қалыпта-лайды`
- Source: `kazakh_textbooks_pack.json / kz_textbook_kz_lang_11_emn_p0011_s06`
- Sentence:

    > Қазақ тарихындағы бұл ұлы басқыншылық этностың санасында өшпестей сақталып, «Ақтабан шұбырынды, Алқакөл сұла- ма» деген тарихи фразеологизм қалыптасты

- [ ] Correct
- Comment:

### Fact #227

- Triple: `(ет — related_to — сүт)`
- Predicate: `related_to`
- Pattern: `X пен Y`
- Source: `kazakh_textbooks_pack.json / kz_textbook_kz_lang_11_emn_p0011_s08`
- Sentence:

    > Негізгі асы ет пен сүт болған босқын қазақтар қайыңның қабығын тіліп, шырынын ішуге, алғыр (ашы өсім- діктің тамыры), қозықұйрық (саңырауқұлақтың түрі)сияқты жеуге жарамды шөптесін өсімдіктерді талғажау етуге мәжбүр болды

- [ ] Correct
- Comment:

### Fact #232

- Triple: `(бұқара — does_to — ел)`
- Predicate: `does_to`
- Pattern: `X Y-ні шақыр-лайды`
- Source: `kazakh_textbooks_pack.json / kz_textbook_kz_lang_11_emn_p0014_s18`
- Sentence:

    > Ол бұқара халықтың осындай ауыр жағдайға ұшырау себептерінің түп-тамырын ашып, елді патшаға бас имеуге, бағынбауға, оның озбырлығына қарсы күресуге шақырады

- [ ] Correct
- Comment:

### Fact #234

- Triple: `(жыл — does_to — шындық)`
- Predicate: `does_to`
- Pattern: `X Y-ні ал-лайды`
- Source: `kazakh_textbooks_pack.json / kz_textbook_kz_lang_11_emn_p0015_s15`
- Sentence:

    > Өйткені ол 1916 жыл оқиғасына арналған жырларында тарихи шындықты нақты түрде баяндап бере алған

- [ ] Correct
- Comment:

### Fact #236

- Triple: `(өнер — does_to — ғылым)`
- Predicate: `does_to`
- Pattern: `X Y-ні көне-лайды`
- Source: `kazakh_textbooks_pack.json / kz_textbook_kz_lang_11_emn_p0016_s02`
- Sentence:

    > Шешендік өнер тарихы Шешендік өнер туралы ғылымды халықаралық тілде риторика (көне грек сөзі) дейді

- [ ] Correct
- Comment:

### Fact #241

- Triple: `(қосымша — does_to — әдебиет)`
- Predicate: `does_to`
- Pattern: `X Y-ні көр-лайды`
- Source: `kazakh_textbooks_pack.json / kz_textbook_kz_lang_11_emn_p0021_s23`
- Sentence:

    > Қосымша әдебиеттерді пайдаланып, «Маған бүкіл қазақ дала- сы ән салып тұрғандай көрінеді» тақырыбында презентация дайындаңдар

- [ ] Correct
- Comment:

### Fact #248

- Triple: `(үй — has — сырт)`
- Predicate: `has`
- Pattern: `X-тың Y-сы бар`
- Source: `kazakh_textbooks_pack.json / kz_textbook_kz_lang_11_emn_p0031_s14`
- Sentence:

    > Анадай жерде, қонақ үйдің сыртында, қасында екі-үш үлкен кісі бар әкесі Құнан- бай тұр екен

- [ ] Correct
- Comment:

---

## Derivation sample

### Derivation #0

- Triple: `(кітап — related_to — ілім)`
- Rule: `R5_shared_is_a_target`
- Confidence: `rule_inferred`
- Source chain: kazakh_proverbs_pack.json/proverb_003, common_voice_kk_pack.json/cv_kk_00047

- [ ] Derivation is semantically valid
- [ ] Underlying facts are both correct
- Comment:

---

## Tally

Fill in after review. `N` = number of items you ended up reviewing; `C` = number you marked correct.

- Facts: C = __ / N = 50 (precision = ___%)
- Derivations (semantic): C = __ / N = 1 (precision = ___%)
- Derivations (both underlying facts): C = __ / N = 1 (precision = ___%)

## By-pattern + by-rule summary

Sampled facts by pattern:

- `X Y-ні айт-лайды`: 2
- `X Y-ні ал-лайды`: 2
- `X Y-ні асыр-лайды`: 1
- `X Y-ні бекіт-лайды`: 1
- `X Y-ні бол-лайды`: 1
- `X Y-ні емде-лайды`: 1
- `X Y-ні жүс-лайды`: 1
- `X Y-ні жөнде-лайды`: 1
- `X Y-ні кел-лайды`: 1
- `X Y-ні кір-лайды`: 1
- `X Y-ні күзет-лайды`: 1
- `X Y-ні күт-лайды`: 1
- `X Y-ні күшейт-лайды`: 1
- `X Y-ні көне-лайды`: 1
- `X Y-ні көр-лайды`: 1
- `X Y-ні көшір-лайды`: 1
- `X Y-ні оқы-лайды`: 1
- `X Y-ні сала-лайды`: 1
- `X Y-ні сын-лайды`: 1
- `X Y-ні сің-лайды`: 1
- `X Y-ні сұра-лайды`: 1
- `X Y-ні тігіл-лайды`: 1
- `X Y-ні шал-лайды`: 1
- `X Y-ні шақыр-лайды`: 1
- `X Y-ні шег-лайды`: 1
- `X Y-ні қалыпта-лайды`: 1
- `X Y-ні қалыптас-лайды`: 1
- `X Y-ні қат-лайды`: 1
- `X Y-ні қаша-лайды`: 1
- `X Y-ні қой-лайды`: 2
- `X Y-ні қос-лайды`: 1
- `X Y-ні қуала-лайды`: 1
- `X Y-ні қыл-лайды`: 1
- `X Y-ні үйрен-лайды`: 1
- `X Y-ні үнде-лайды`: 1
- `X Y-ні ұлас-лайды`: 1
- `X Y-ні ұр-лайды`: 1
- `X Y-ні ұсын-лайды`: 1
- `X Y-ні ұшыра-лайды`: 1
- `X Y-ні өткіз-лайды`: 1
- `X пен Y`: 3
- `X — Y`: 3
- `X-тың Y-сы бар`: 1

Sampled derivations by rule:

- `R5_shared_is_a_target`: 1
