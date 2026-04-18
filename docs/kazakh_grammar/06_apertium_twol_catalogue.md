# Apertium-kaz twol Rule Catalogue

Source: `data/external/apertium/apertium-kaz/apertium-kaz.kaz.twol` (476 lines, 54 rules)

This document catalogues every two-level rule so that Week 2's Rust port has a complete reference. Rules are grouped by phenomenon.

## Archiphoneme inventory (the underlying abstract forms)

Two-level morphology writes suffixes in an **abstract** form using archiphonemes — single symbols that resolve to concrete surface letters under context rules. Apertium's Kazakh archiphonemes:

| archiphoneme | default realisation | contexts that differ |
|---|---|---|
| `{K}` | к | (back harmony, voicing) |
| `{l}` `{L}` | л | (voicing) |
| `{M}` | м | (voicing) |
| `{N}` | н | (voicing) |
| `{G}` | г | (back harmony, voicing) |
| `{D}` | д | (voicing, nasal harmony) |
| `{S}` | с | (deletion context) |
| `{n}` | н | (buffer; may delete) |
| `{A}` | а | (vowel harmony: back↔front, a↔ә / о↔ө) |
| `{I}` | ы | (vowel harmony: back↔front, ы↔і / ұ↔ү) |
| `{E}` | а | (special — also deletes, or triggers й insertion) |
| `{y}` | ы | (buffer vowel, may delete) |

When our Rust FST writes suffix tables, they'll be written in this abstract form and realised by applying the rule catalogue below.

## Character classes (Sets)

Every rule restricts its trigger to specific character classes. Apertium defines:

- `Vow` — all vowels
- `LowVow` — а, ә, е, о, ө
- `HighVow` — і, ү, ы, ұ
- `FrontVow` — ә, е, і, ө, ү, э
- `BackVow` — а, о, ұ, ы, я, ё
- `Cns` — all consonants
- `Nasals` — м, н, ң
- `Liquid` — л
- `HighSonCns` — й, у, р, и, ю
- `VoicedObstruents` — б, в, г, ғ, д, ж, з
- `VoicelessCns` — к, қ, п, с, т, ф, х, ш, щ, ц, ч, һ

These are **the same classes we laid out in `01_phonology.md`**. Direct correspondence → direct Rust enum/const.

## The 54 Rules

### Group A: Desonorisation (voicing assimilation — voicing a suffix consonant backwards)

1. **N Desonorisation**: `{N}:д` after high-sonorant/liquid/nasal/voiced-obstruent; `{N}:т` after voiceless; `{N}:н` after nasal-Vow-nasal context.
2. **L Desonorisation**: `{L}:д` after liquid/nasal/voiced; `{L}:т` after voiceless; `{L}:л` elsewhere.
3. **M Desonorisation**: `{M}:б` after nasal/voiced; `{M}:п` after voiceless; `{M}:м` elsewhere.

Rust translation: single function `realise_sonorant(arch, preceding_class)` returning exactly one surface letter.

### Group B: Nasal harmony

4. **D nasal harmony**: `{D}:н` if the stem contains a nasal and the rule context matches; otherwise `{D}:д`.

### Group C: Voicing assimilation

5. **Forward voicing assimilation**: certain consonants in suffixes devoice after voiceless-final stems. Table:

| archiphoneme | after voiceless | elsewhere |
|---|---|---|
| `{M}` | п | б/м |
| `{D}` | т | д/н |
| `{L}` | т | д/л |
| `{N}` | т | д/н |

6. **Voicing assimilation of front G**: `{G}:ге`/`{G}:ке` based on preceding consonant voicing.
7. **Voicing assimilation of back G**: `{G}:ға`/`{G}:қа`.
8. **Voicing assimilation of G across space** — used in compound words (across space boundary).
9. **Normal G across space** — reciprocal rule, not needed for v1.0.0.
10. **Intervocalic voicing of п** — `п` → `б` between vowels in certain stems.
11. **Lenition of п to у in Ip forms** — marginal, certain verb forms.
12. **Intervocalic voicing of к/қ** — `к → г`, `қ → ғ` in some derivations.

### Group D: Vowel harmony (the biggest group)

13. **I Vowel Harmony**: `{I}` → ы (back) / і (front). Decision based on **last vowel in the stem**.
14. **A Vowel Harmony**: `{A}` → а (back) / е (front).
15. **E Vowel Harmony**: `{E}` → а (back) / е (front).
16. **Harmony of unepenthesised vowel in some stems**: specific stem patterns.
17. **Turn {E} into й after a vowel**: `{E}` deletes and inserts `й` in `V_V` contexts.
18. **Turn {E} into и to combine with a і and ы**: dialectal-style coalescence.
19. **Delete і before {E} so that the result can be и**: part of same coalescence.
20. **Delete ы before {E} so that the result can be и**: part of same coalescence.
21. **A back vowel harmony after и**: override — `и` triggers back harmony.
22. **E back vowel harmony after и**: same principle for `{E}`.
23. **Vowel harmony for archiphoneme {A} after й and и**: specific after-й/и case.
24. **Vowel harmony for archiphoneme {E} after й**: same for {E}.

### Group E: Glide / й epenthesis and deletion

25. **Deletion of й before yoticised vowels when not earlier in a stem**: avoid `й+йотиros. vowel` sequences.

### Group F: K/G back-/front-assimilation

26. **Back assimilation of G**: `{G}` → `ғ` in back contexts.
27. **Back assimilation of K**: `{K}` → `қ` in back contexts.
28. **Back assimilation and voicing of K**: combined.
29. **Voicing of K**: `{K}` → `г` in voiced contexts.

### Group G: Vowel deletion rules

30. **Deletion of {A} directly after vowel**: `бала+{A}м → баламыз`? No — this is `{A}` dropping in `V+{A}` contexts.
31. **Deletion of ы before у**: `ы → 0` / `_ у`.
32. **Deletion of і before у**: `і → 0` / `_ у`.
33. **у > ю after stems in /й/**: `у → ю` / `й _`.
34. **Deletion of {S} after a consonant**: buffer `с` disappears after consonants.
35. **Deletion of {I} after vowels**: buffer `ы/і` disappears after vowels (prevents `бала+ым → *бала-ым`, correct is `балам`).

### Group H: Dialectal and edge-case rules (low priority for v1.0.0)

36. **Dialectal deletion of л between low vowel and Ip**: dialect-specific.
37. **Dialectal deletion of I when л deletes before Iп**: dialect-specific.
38. **Epenthesis in some stems**: stem-specific irregulars.

### Group I: Case-specific grammatical rules

39. **Deletion of accusative {N} after {n}**: when possessive-bearing stem already ends in `н`, accusative `{N}` drops.
40. **Deletion of {n} before genitive {N}s**: buffer `н` disappears before genitive.
41. **Deletion of {n} before ablative {D}**: buffer `н` disappears before ablative.
42. **Deletion of {n} before instrumental {M}**: buffer `н` disappears before instrumental.
43. **Deletion of dative {G} after px1sg, px2sg**: dative form changes after certain possessives.
44. **/n/ deletion in px3 nominative**: the 3rd-person `сы/сі` form doesn't get `н` in nominative.
45. **Deletion of {I} after {n}**: buffer vowel drops after inserted `н`.

### Group J: Loanword adaptation rules

46. **Deletion of т when usually Russian word ending in ст precedes suffix starting in т**: e.g., `текст+тер → текстер` not `*тексттер`.
47. **Deletion of д when usually Russian word ending in зд precedes suffix starting in д**: same logic.
48. **Deletion of numeral final - in nominative**: handles hyphenated numerals.
49. **Deletion of ь when vowel-initial stem follows**: Russian soft sign drops.
50. **Deletion of с at end of сс stem with suffix**: geminate reduction.

### Group K: Realisation of disappearing underlying phonemes

51. **{д} surfaces before vowels**: underlying `{д}` appears as `д` before vowels.
52. **{т} surfaces before vowels**: underlying `{т}` appears as `т` before vowels.
53. **Deletion of {і} at end of word**: underlying `{і}` drops word-finally.
54. **Passive {l} becomes n after verb stem ending in /l/**: dissimilation.

## Rust translation design sketch

The 54 rules can be packed into a handful of Rust functions by grouping:

```rust
fn realise_archiphoneme(
    arch: Archiphoneme,
    left_ctx: &PhonologicalContext,
    right_ctx: &PhonologicalContext,
) -> Option<char> {
    match arch {
        Archiphoneme::A => realise_a_harmony(left_ctx),
        Archiphoneme::I => realise_i_harmony(left_ctx),
        Archiphoneme::E => realise_e_harmony(left_ctx, right_ctx),
        Archiphoneme::D => realise_d(left_ctx),
        Archiphoneme::L => realise_l(left_ctx),
        Archiphoneme::M => realise_m(left_ctx),
        Archiphoneme::N => realise_n(left_ctx, right_ctx),
        Archiphoneme::G => realise_g(left_ctx),
        Archiphoneme::K => realise_k(left_ctx),
        Archiphoneme::S => realise_s(left_ctx),
        Archiphoneme::Y => realise_y(left_ctx),
        Archiphoneme::N_buf => realise_n_buffer(left_ctx, right_ctx),
    }
}
```

Each `realise_*` is a small table lookup (maybe 5-10 branches). Total: ~12 functions, ~100 lines of Rust.

Plus deletion rules as **post-pass filters** on the output string (30-39, 45, 49, 53, 54 — each ~5 lines).

**Total estimated Rust code for the complete phonology module: ~500-800 lines.** Achievable in 2-3 working days next week.

## Gotchas to watch for

- **Rule ordering matters.** Apertium uses a two-level formalism where rules fire simultaneously, not sequentially. When we port to plain Rust functions we must either:
  - (a) compute the full realisation in one pass with all rules as lookup tables, or
  - (b) apply rules in a specific pipeline order and test against Apertium's output to detect ordering bugs.
  Option (a) is cleaner.
- **Abstract-only phonemes** (`{а}`, `{э}`, `{й}`, `{л}`, `{н}`, `{з}`, `{с}`) exist only in underlying forms — they never surface as themselves. They disappear with specific phonological effects. We list them in lexicon entries and resolve at realisation time.
- **YotVow** (я, е, ё, ю) — the yoticised vowels behave specially after certain consonants. Needs a dedicated check.
- **Russian loanword integration** — rules 46-49, 46 — cover Russian-origin stems. Per our v0.5.x choice to exclude loanwords, these can be **omitted from v1.0.0**, simplifying the port.
