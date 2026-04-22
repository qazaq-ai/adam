# Precision audit — native-speaker review

**Target:** 50-fact sample + 50-derivation sample from the committed artifacts, seed `42`.

- `facts.json`: 17 facts total (upstream status: `completed`) — sampled 17 here.
- `derived_facts.json`: 1 derivations total (upstream status: `completed`) — sampled 1 here.

## How to review

For each fact, mark the checkbox if the triple `(subject, predicate, object)` is **correct**: the sentence genuinely asserts that the subject has the claimed relation to the object, and both root resolutions are correct. When unsure, leave unchecked and add a one-line note in the Comments row. Update the **Tally** section at the bottom with your counts. Precision is defined as `correct / reviewed`.

---

## Fact sample

### Fact #0

- Triple: `(ілім — is_a — бұлақ)`
- Predicate: `is_a`
- Pattern: `X — Y`
- Source: `common_voice_kk_pack.json / cv_kk_00047`
- Sentence:

    > Ілім — бұлақ.

- [ ] Correct
- Comment:

### Fact #1

- Triple: `(адам — has — гүл)`
- Predicate: `has`
- Pattern: `X-тың Y-сы бар`
- Source: `common_voice_kk_pack.json / cv_kk_00130`
- Sentence:

    > Адам бар дүр, адамдардың гүлі дүр, адам бар дүр, хайуан одан жақсы дүр.

- [ ] Correct
- Comment:

### Fact #2

- Triple: `(айлакерлік — is_a — іс)`
- Predicate: `is_a`
- Pattern: `X — Y`
- Source: `common_voice_kk_pack.json / cv_kk_00273`
- Sentence:

    > Айлакерлік — әлсіздіктің ісі.

- [ ] Correct
- Comment:

### Fact #3

- Triple: `(ел — has — сыртқ)`
- Predicate: `has`
- Pattern: `X-тың Y-сы бар`
- Source: `cc100_kk_pack.json / cc100_kk_0000197`
- Sentence:

    > Ауғанстан жөніндегі ірі форумға 40-қа жуық елдің сыртқы істер министрлері, 70 шақты халықаралық өкілдер қатысып жатыр. Олардың арасында АҚШ мемлекеттік хатшысы Хиллари Клинтон, БҰҰ Бас хатшысы Пан Ги Мун, ЕҚЫҰ-ның қазіргі хатшысы, Қазақстан сыртқы істер министрі Қанат Саудабаев бар.

- [ ] Correct
- Comment:

### Fact #4

- Triple: `(тыңайтқыш — has — түр)`
- Predicate: `has`
- Pattern: `X-тың Y-сы бар`
- Source: `cc100_kk_pack.json / cc100_kk_0000417`
- Sentence:

    > 4. Облыстың, республикалық маңызы бар қаланың, астананың жергілікті атқарушы органының қаулысымен Қазақстан Республикасы Ауыл шаруашылығы министрлігімен (бұдан әрі – Министрлік) келісім бойынша субсидияланатын отандық тыңайтқыштардың түрлері және тыңайтқыштарды сатушыдан сатып алынған тыңайтқыштардың 1 тоннасына (килограмына, литрiне) арналған субсидиялардың нормалары белгіленеді.

- [ ] Correct
- Comment:

### Fact #5

- Triple: `(кітап — is_a — бұлақ)`
- Predicate: `is_a`
- Pattern: `X — Y`
- Source: `kazakh_proverbs_pack.json / proverb_003`
- Sentence:

    > Кітап — білім бұлағы.

- [ ] Correct
- Comment:

### Fact #6

- Triple: `(ғылым — is_a — қазына)`
- Predicate: `is_a`
- Pattern: `X — Y`
- Source: `kazakh_proverbs_pack.json / proverb_007`
- Sentence:

    > Ғылым — таусылмас қазына.

- [ ] Correct
- Comment:

### Fact #7

- Triple: `(тіл — is_a — айна)`
- Predicate: `is_a`
- Pattern: `X — Y`
- Source: `kazakh_proverbs_pack.json / proverb_020`
- Sentence:

    > Тіл — жүректің айнасы.

- [ ] Correct
- Comment:

### Fact #8

- Triple: `(ақиқат — is_a — тірек)`
- Predicate: `is_a`
- Pattern: `X — Y`
- Source: `kazakh_proverbs_pack.json / proverb_049`
- Sentence:

    > Ақиқат — тіршіліктің тірегі.

- [ ] Correct
- Comment:

### Fact #9

- Triple: `(ынтымақ — is_a — байлық)`
- Predicate: `is_a`
- Pattern: `X — Y`
- Source: `kazakh_proverbs_pack.json / proverb_053`
- Sentence:

    > Ынтымақ — халықтың байлығы.

- [ ] Correct
- Comment:

### Fact #10

- Triple: `(бала — is_a — болашақ)`
- Predicate: `is_a`
- Pattern: `X — Y`
- Source: `kazakh_proverbs_pack.json / proverb_056`
- Sentence:

    > Бала — елдің болашағы.

- [ ] Correct
- Comment:

### Fact #11

- Triple: `(ана — is_a — жанашыр)`
- Predicate: `is_a`
- Pattern: `X — Y`
- Source: `kazakh_proverbs_pack.json / proverb_058`
- Sentence:

    > Ана — бала жанашыры.

- [ ] Correct
- Comment:

### Fact #12

- Triple: `(жер — is_a — бастау)`
- Predicate: `is_a`
- Pattern: `X — Y`
- Source: `kazakh_proverbs_pack.json / proverb_060`
- Sentence:

    > Жер — анасы, тіршіліктің бастауы.

- [ ] Correct
- Comment:

### Fact #13

- Triple: `(еңбек — is_a — қайнар)`
- Predicate: `is_a`
- Pattern: `X — Y`
- Source: `kazakh_proverbs_pack.json / proverb_068`
- Sentence:

    > Еңбек — табыстың қайнары.

- [ ] Correct
- Comment:

### Fact #14

- Triple: `(ой — is_a — қару)`
- Predicate: `is_a`
- Pattern: `X — Y`
- Source: `kazakh_proverbs_pack.json / proverb_071`
- Sentence:

    > Ой — адамның қаруы.

- [ ] Correct
- Comment:

### Fact #15

- Triple: `(адам — has — іш)`
- Predicate: `has`
- Pattern: `X-тың Y-сы бар`
- Source: `kazakh_textbooks_pack.json / kz_textbook_kz_lang_11_emn_p0009_s08`
- Sentence:

    > Өздерің біле- тін адамдардың ішінде шешен сөйлейтіндер бар ма

- [ ] Correct
- Comment:

### Fact #16

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

- Facts: C = __ / N = 17 (precision = ___%)
- Derivations (semantic): C = __ / N = 1 (precision = ___%)
- Derivations (both underlying facts): C = __ / N = 1 (precision = ___%)

## By-pattern + by-rule summary

Sampled facts by pattern:

- `X — Y`: 12
- `X-тың Y-сы бар`: 5

Sampled derivations by rule:

- `R5_shared_is_a_target`: 1
