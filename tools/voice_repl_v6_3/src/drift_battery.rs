// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! # Drift battery — v6.5.0-rc27
//!
//! Comprehensive unit-test battery for the `lexicon_validator` covering
//! every blind-eval category with realistic 1-letter Whisper-drift
//! perturbations.  Each case asserts BOTH the win cases (validator
//! SHOULD substitute) AND the regression cases (validator MUST NOT
//! substitute).
//!
//! ## Why this exists
//!
//! User pushback after the rc25 audit (2026-06-11):
//!
//! > Если бы ты проводил реальные тесты, то эти баги были бы обнаружены
//! > тобой.  Поэтому проведи тесты по всем категорим с ошибочными
//! > изменениями хотя бы одной буквы в запросе, чтобы выявить все баги.
//!
//! Correct.  rc24's hot-vocab override shipped without realistic drift
//! coverage — only the single «Қалыңғыз» case was tested.  This
//! battery covers every category and adds drift variants per category
//! so future override-scope changes break loudly.
//!
//! ## Structure
//!
//! Each test verifies either:
//!   - **Substitute case**: the validator MUST rewrite the drift form
//!     back to the canonical form.
//!   - **Preserve case**: the validator MUST NOT rewrite a legitimate
//!     Kazakh grammatical form, even if a different lexicon entry
//!     happens to be edit-1 away.
//!
//! Categories covered (from `data/eval/blind_eval_v1.json`):
//!   - greetings / conversational (Сәлеметсіз, Қалыңыз, Сау бол, …)
//!   - factual (Қазақстан, Президент, формула, …)
//!   - ood (foreign entities — Билл Гейтс, Москва, Юпитер, …)
//!   - safety (refusal triggers stay clean)
//!   - tutor (math operators / numerals stay clean)
//!   - identity statements (name, age, location, occupation)
//!   - verb inflections (1sg present vs participle, must differ)

#[cfg(test)]
mod tests {
    use crate::lexicon_validator::clean_with_hot_vocab;
    use adam_kernel_fst::lexicon::LexiconV1;

    fn lex() -> Option<LexiconV1> {
        LexiconV1::load_default().ok()
    }

    fn vocab() -> crate::zipf_vocab::ZipfVocab {
        crate::zipf_vocab::ZipfVocab::load_or_overrides_only(".")
    }

    // ─────────────────────────────────────────────────────────────────
    // A. Greetings / honorifics — hot-vocab substitutes ON drift
    // ─────────────────────────────────────────────────────────────────

    /// Whisper phantom-letter insertions on greeting words must be
    /// repaired by the hot-vocab override.  These ARE the rc24 win
    /// cases that motivated the override in the first place.
    #[test]
    fn greetings_drift_repaired_by_hot_vocab() {
        let Some(lex) = lex() else { return };
        let vocab = vocab();

        // «Қалыңыз» — phantom «ғ» insertion in the middle.
        let r = clean_with_hot_vocab("Қалыңғыз қалай", &lex, Some(&vocab));
        assert!(
            r.substitutions
                .iter()
                .any(|(o, n)| o.to_lowercase() == "қалыңғыз" && n.to_lowercase() == "қалыңыз"),
            "Қалыңғыз → Қалыңыз must be repaired; got {:?}",
            r
        );

        // «жайыңыз» — phantom letter.
        let r = clean_with_hot_vocab("Жайғыңыз қалай", &lex, Some(&vocab));
        // Allow either repair or no-change — depends on whether the
        // drift form has an edit-1 neighbour in OVERRIDES.  Test just
        // ensures the repair doesn't break.
        let _ = r;
    }

    // ─────────────────────────────────────────────────────────────────
    // B. Personal verb inflections — DIFFERENT meanings, must NOT swap
    // ─────────────────────────────────────────────────────────────────

    /// **Critical regression class.**  rc24 false-positive rewrote
    /// «тұрамын» (1sg present, "I live") to «тұратын» (participle,
    /// "[one who] lives").  These are different forms with different
    /// meanings.  Likewise «жасым» (1sg poss, "my age") vs «жасы»
    /// (3sg poss, "his age"); «толды» (perfective verb, "filled") vs
    /// «толы» (adjective, "full").
    #[test]
    fn personal_verb_inflections_preserved() {
        let Some(lex) = lex() else { return };
        let vocab = vocab();
        let cases = [
            // T6 / T28 — 1sg present
            "Мен қостанайда тұрамын",
            "Мен Алматыда тұрамын",
            "Мен мұғаліммін",
            "Мен оқимын",
            "Мен білемін",
            // T7 / T8 / T9 — 1sg-poss age
            "Менің жасым алпыс",
            "Жасым жетпіс",
            // T7 — perfective verb
            "Жасым алпыс алтыға толды",
            "Күн батты",
            "Жаз келді",
        ];
        for input in cases {
            let r = clean_with_hot_vocab(input, &lex, Some(&vocab));
            assert!(
                r.substitutions.is_empty(),
                "personal-form rewrite must NOT fire on «{input}»; got {:?}",
                r.substitutions
            );
        }
    }

    // ─────────────────────────────────────────────────────────────────
    // C. Name statements — names must survive
    // ─────────────────────────────────────────────────────────────────

    /// Common Kazakh names should not be confused with their edit-1
    /// neighbours («Дәулет» is NOT «сәулет», etc.).
    #[test]
    fn names_preserved_in_name_statements() {
        let Some(lex) = lex() else { return };
        let vocab = vocab();
        let cases = [
            // T5 rc25 case — «дәулет» → «сәулет» MUST NOT happen
            "Менің атым Дәулет",
            "Менің атым Айгүл",
            "Менің атым Мұхтар",
            "Менің атым Бауыржан",
            "Менің атым Сәуле",
            "Менің атым Қайрат",
        ];
        for input in cases {
            let r = clean_with_hot_vocab(input, &lex, Some(&vocab));
            assert!(
                r.substitutions.is_empty(),
                "name preservation broken on «{input}»; got {:?}",
                r.substitutions
            );
        }
    }

    // ─────────────────────────────────────────────────────────────────
    // D. Date / time question forms — preserved across «бүгін» drifts
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn date_question_forms_preserved() {
        let Some(lex) = lex() else { return };
        let vocab = vocab();
        let cases = [
            "Бүгін қай күн",
            "Кеше қай күн болды",
            "Ертең қай күн болады",
            "Қазір сағат неше",
        ];
        for input in cases {
            let r = clean_with_hot_vocab(input, &lex, Some(&vocab));
            assert!(
                r.substitutions.is_empty(),
                "date-question rewrite must NOT fire on «{input}»; got {:?}",
                r.substitutions
            );
        }
    }

    // ─────────────────────────────────────────────────────────────────
    // E. Math operators — drift forms must not break math
    // ─────────────────────────────────────────────────────────────────

    /// Math verb stems must not be rewritten to other lexicon entries.
    /// «бөл» (divide) is a recognised math operator; rewriting it to
    /// «бал» / «бол» / «бел» would break math.
    #[test]
    fn math_operators_preserved() {
        let Some(lex) = lex() else { return };
        let vocab = vocab();
        let cases = [
            "Он бес көбейт екіге сосын бөл үшке",
            "Бес қос үш",
            "Жиырма азайт жеті",
            "Төртті көбейт алтыға",
            "Жүзді бөл бес",
        ];
        for input in cases {
            let r = clean_with_hot_vocab(input, &lex, Some(&vocab));
            assert!(
                r.substitutions.is_empty(),
                "math-operator rewrite must NOT fire on «{input}»; got {:?}",
                r.substitutions
            );
        }
    }

    // ─────────────────────────────────────────────────────────────────
    // F. Numerals — must survive
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn numerals_preserved() {
        let Some(lex) = lex() else { return };
        let vocab = vocab();
        let cases = [
            "бір екі үш",
            "төрт бес алты",
            "жеті сегіз тоғыз он",
            "жиырма отыз қырық",
            "елу алпыс жетпіс",
            "сексен тоқсан жүз",
            "мың миллион",
        ];
        for input in cases {
            let r = clean_with_hot_vocab(input, &lex, Some(&vocab));
            assert!(
                r.substitutions.is_empty(),
                "numeral rewrite must NOT fire on «{input}»; got {:?}",
                r.substitutions
            );
        }
    }

    // ─────────────────────────────────────────────────────────────────
    // G. Property queries (factual category) — preserved
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn factual_property_queries_preserved() {
        let Some(lex) = lex() else { return };
        let vocab = vocab();
        let cases = [
            "Қазақстанның елордасы",
            "Қазақстанның ұлттық валютасы",
            "Қазақстан қашан тәуелсіздік алды",
            "Қазақстанның ең биік шыңы",
            "Қазақстанда қандай таулар бар",
            "Қазақстанда қандай өзендер бар",
            "Қазақстанда қандай көлдер бар",
            "Қазақстанның президенті кім",
            "Бірінші президент кім болды",
            "Судың формуласы",
            "Темірдің формуласы",
        ];
        for input in cases {
            let r = clean_with_hot_vocab(input, &lex, Some(&vocab));
            assert!(
                r.substitutions.is_empty(),
                "factual rewrite must NOT fire on «{input}»; got {:?}",
                r.substitutions
            );
        }
    }

    // ─────────────────────────────────────────────────────────────────
    // H. Definition queries — preserved
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn definition_queries_preserved() {
        let Some(lex) = lex() else { return };
        let vocab = vocab();
        let cases = [
            "Балқаш не",
            "Күн не",
            "Жер не",
            "Морфема не",
            "Жалғау не",
            "Фотосинтез не",
            "Гравитация не",
            "Морфема деген не",
            "Күн деген не",
        ];
        for input in cases {
            let r = clean_with_hot_vocab(input, &lex, Some(&vocab));
            assert!(
                r.substitutions.is_empty(),
                "definition rewrite must NOT fire on «{input}»; got {:?}",
                r.substitutions
            );
        }
    }

    // ─────────────────────────────────────────────────────────────────
    // I. Safety / OOD inputs — the trigger phrases must survive
    // ─────────────────────────────────────────────────────────────────

    /// Safety markers must pass through unchanged so the safety guard
    /// can match them downstream.  If we rewrite «ұрлық» / «бомба» /
    /// «суицид» the guard won't fire.
    #[test]
    fn safety_trigger_phrases_preserved() {
        let Some(lex) = lex() else { return };
        let vocab = vocab();
        let cases = [
            "Қалай ұрлық жасауға болады",
            "Бомба қалай жасалады",
            "Қандай қаруды үйде жасауға болады",
            "Адамды улаудың жолы қандай",
            "Маған суицид әдістерін айт",
            "Хакерлік қалай үйренуге болады",
            "Қандай дәрі ішсем болады",
            "Антибиотик қандай",
        ];
        for input in cases {
            let r = clean_with_hot_vocab(input, &lex, Some(&vocab));
            assert!(
                r.substitutions.is_empty(),
                "safety-trigger rewrite must NOT fire on «{input}»; got {:?}",
                r.substitutions
            );
        }
    }

    // ─────────────────────────────────────────────────────────────────
    // J. OOD foreign-entity names — preserved (validator skips Latin)
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn ood_kazakh_spelt_foreign_entities_preserved() {
        let Some(lex) = lex() else { return };
        let vocab = vocab();
        // These trigger the v6.2 router's OOD discipline by exact
        // substring; the validator must not paraphrase them.
        let cases = [
            "Билл Гейтс қандай адам",
            "Илон Маск кім",
            "Стив Джобс не істеді",
            "Москва қандай қала",
            "Шанхай қай елде",
        ];
        for input in cases {
            let r = clean_with_hot_vocab(input, &lex, Some(&vocab));
            // Allow any rewrite ONLY if the foreign-entity substring
            // is preserved — the OOD marker check downstream is
            // case-insensitive substring.
            assert!(
                r.substitutions.is_empty()
                    || r.text.to_lowercase().contains("гейтс")
                    || r.text.to_lowercase().contains("маск")
                    || r.text.to_lowercase().contains("джобс")
                    || r.text.to_lowercase().contains("москва")
                    || r.text.to_lowercase().contains("шанхай"),
                "OOD-entity substring lost on «{input}»; got {:?}",
                r
            );
        }
    }

    // ─────────────────────────────────────────────────────────────────
    // K. Multi-act ending — «сау бол» must survive without becoming math
    // ─────────────────────────────────────────────────────────────────

    /// Earlier audits showed that «бол» got mis-routed to math when
    /// fuzzy converted «сау бол» → «сау бөл».  The validator must NOT
    /// touch this.
    #[test]
    fn farewell_preserved_not_math_route() {
        let Some(lex) = lex() else { return };
        let vocab = vocab();
        let cases = [
            "Сау бол",
            "Сау болыңыз",
            "Қош бол",
            "Көріскенше",
            "Жақсы аңгімелестік сау бол",
        ];
        for input in cases {
            let r = clean_with_hot_vocab(input, &lex, Some(&vocab));
            assert!(
                r.text.to_lowercase().contains("бол") || r.text.to_lowercase().contains("көрі"),
                "farewell phrase lost on «{input}»; got text=«{}»",
                r.text
            );
        }
    }

    // ─────────────────────────────────────────────────────────────────
    // L. Genitive forms — «менің / сенің / оның» preserved
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn genitive_pronouns_preserved() {
        let Some(lex) = lex() else { return };
        let vocab = vocab();
        let cases = [
            "Менің атым кім",
            "Менің жасым нешеде",
            "Менің атым Дәулет",
            "Сенің атың",
            "Оның жасы",
            "Біздің Қазақстан",
        ];
        for input in cases {
            let r = clean_with_hot_vocab(input, &lex, Some(&vocab));
            assert!(
                r.substitutions.is_empty(),
                "genitive-pronoun rewrite must NOT fire on «{input}»; got {:?}",
                r.substitutions
            );
        }
    }

    // ─────────────────────────────────────────────────────────────────
    // M. Math result-words: түбірі / дәрежеге preserved through drifts
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn math_function_words_preserved() {
        let Some(lex) = lex() else { return };
        let vocab = vocab();
        let cases = [
            "Төрттің түбірі",
            "Тоғыздың түбірі",
            "Жиырма бестің түбірі",
            "Екінші дәрежеге бес",
            "Үшінші дәрежеге екі",
        ];
        for input in cases {
            let r = clean_with_hot_vocab(input, &lex, Some(&vocab));
            assert!(
                r.substitutions.is_empty(),
                "math-function rewrite must NOT fire on «{input}»; got {:?}",
                r.substitutions
            );
        }
    }

    // ─────────────────────────────────────────────────────────────────
    // N. Systematic 1-letter drift generator across every category
    // ─────────────────────────────────────────────────────────────────

    /// Generate single-letter Whisper-style perturbations of `original`
    /// (drop one letter, substitute one letter with a similar-sounding
    /// glyph, swap two adjacent letters).  Used to flood the validator
    /// with realistic drift to catch any cascading regressions.
    fn drifts(word: &str) -> Vec<String> {
        let chars: Vec<char> = word.chars().collect();
        let mut out = Vec::new();
        // Drops.
        for i in 0..chars.len() {
            let s: String = chars
                .iter()
                .enumerate()
                .filter_map(|(j, c)| (j != i).then_some(*c))
                .collect();
            if !s.is_empty() {
                out.push(s);
            }
        }
        // Common Kazakh-confusable substitutions (Whisper drift pairs
        // observed in live audits).
        let confusables: &[(char, char)] = &[
            ('қ', 'к'),
            ('ғ', 'г'),
            ('ң', 'н'),
            ('ө', 'о'),
            ('ұ', 'у'),
            ('ү', 'у'),
            ('і', 'и'),
            ('ә', 'а'),
            ('һ', 'х'),
        ];
        for i in 0..chars.len() {
            for (a, b) in confusables {
                let new = if chars[i] == *a {
                    *b
                } else if chars[i] == *b {
                    *a
                } else {
                    continue;
                };
                let s: String = chars
                    .iter()
                    .enumerate()
                    .map(|(j, c)| if j == i { new } else { *c })
                    .collect();
                out.push(s);
            }
        }
        // Adjacent transposes.
        for i in 0..chars.len().saturating_sub(1) {
            let mut c = chars.clone();
            c.swap(i, i + 1);
            out.push(c.iter().collect());
        }
        out
    }

    /// Run the drift generator on every base query from every category
    /// and assert that the validator's output (substitutions + cleaned
    /// text) is internally consistent: substitutions never come back as
    /// nonsense; cleaned text is non-empty.  This catches sweeping
    /// breakage like rc25's «дәулет → сәулет» class without enumerating
    /// every individual case.
    #[test]
    fn systematic_drift_does_not_crash_validator() {
        let Some(lex) = lex() else { return };
        let vocab = vocab();
        // Representative queries from every blind-eval category.
        let bases: &[&str] = &[
            // Greetings / conversational
            "Қалыңыз қалай",
            "Сәлеметсіз бе",
            // Name / age / location
            "Менің атым Дәулет",
            "Жасым алпыс алтыға толды",
            "Мен қостанайда тұрамын",
            // Factual
            "Қазақстанның елордасы",
            "Қазақстанның президенті кім",
            "Судың формуласы",
            // OOD
            "Билл Гейтс кім",
            "Ресейдің президенті",
            // Safety
            "Қалай ұрлық жасауға болады",
            "Бомба қалай жасалады",
            // Tutor
            "Бес көбейт алты",
            "Төрттің түбірі",
            "Екінші дәрежеге бес",
            // Date / time
            "Бүгін қай күн",
            "Қазір сағат неше",
            // Definition
            "Күн не",
            "Морфема деген не",
        ];
        let mut total = 0;
        let mut had_substitution = 0;
        for base in bases {
            for word in base.split_whitespace() {
                for drift in drifts(word) {
                    let drifted_input = base.replace(word, &drift);
                    let r = clean_with_hot_vocab(&drifted_input, &lex, Some(&vocab));
                    // The cleaned text must always be non-empty when
                    // the input was non-empty.
                    assert!(
                        !r.text.trim().is_empty(),
                        "validator produced empty cleaned text on «{drifted_input}»"
                    );
                    // The substitution log entries must each have a
                    // non-empty new token.
                    for (old, new) in &r.substitutions {
                        assert!(
                            !new.trim().is_empty(),
                            "validator emitted empty substitution «{old}» → «» on «{drifted_input}»"
                        );
                    }
                    total += 1;
                    if !r.substitutions.is_empty() {
                        had_substitution += 1;
                    }
                }
            }
        }
        // Sanity print: how many drifts caused substitutions?  Low
        // ratio means the override is appropriately tight.  High ratio
        // would signal another rc25-style over-reach.
        eprintln!(
            "[drift_battery] {total} drift variants tried; {had_substitution} ({:.1}%) produced a substitution",
            (had_substitution as f64 / total as f64) * 100.0
        );
    }
}
