//! Layer 2 — semantic interpreter.
//!
//! **Kazakh-only surface (post-v1.1.0).** The v0.9.6 multilingual
//! recogniser (Russian / English triggers) was reverted in v1.1.0 per
//! the Kazakh-first directive — `adam` accepts and produces only
//! Kazakh. All detectors below match Kazakh keywords / phrases
//! exclusively. For MVP social intents the FST parser is more than we
//! need; we work directly on the lowercased-cleaned token list.

use adam_kernel_fst::lexicon::LexiconV1;
use adam_kernel_fst::parser::Analysis;

use crate::intent::{GreetingKind, Intent, TimeOfDay};
use crate::language_core::{
    looks_like_named_place_candidate, normalize_place_name, normalize_proper_noun,
};
// **v4.24.0** — topic-extraction primitives moved to a dedicated
// module (NOT_A_TOPIC + MULTIWORD_ENTITIES + LATIN_TECH_SUBJECTS
// + the noun-hint heuristics). Only the entry points used
// directly by `interpret_text_with_lexicon` and `interpret` are
// re-imported here; the lower-level helpers (latin / multiword /
// topic-marker / accusative / locative-attributive) are called
// transitively through `best_noun_hint` and don't need a direct
// import in this file.
use crate::topic_extraction::{
    NOT_A_TOPIC, TopicConfidence, best_noun_hint_with_confidence, first_noun_root,
};
// `MULTIWORD_ENTITIES` and `multiword_entity_hint` are
// referenced by the in-file
// `world_core_multiword_coverage` /
// `multiword_entity_hint_*` tests.
#[cfg(test)]
use crate::topic_extraction::{MULTIWORD_ENTITIES, multiword_entity_hint};

/// Entry point. Takes the raw input text; tokenises, lowercases, strips
/// punctuation, then dispatches by keyword rules.
///
/// The `_parses` argument is kept so callers stay forward-compatible:
/// v0.7.5 intents can start using morphological info without changing
/// the call site.
pub fn interpret_text(input: &str, parses: &[Analysis]) -> Intent {
    interpret_text_with_lexicon(input, parses, None)
}

/// Lexicon-aware variant used by `Conversation::turn` and
/// `respond_with_repo`. When a lexicon is supplied, the occupation
/// recogniser does a generic 1sg-copula strip + noun lookup instead
/// of consulting a fixed 6-form table — giving full Lexicon reach
/// (e.g. `философпын` → `философ` if present).
pub fn interpret_text_with_lexicon(
    input: &str,
    parses: &[Analysis],
    lexicon: Option<&LexiconV1>,
) -> Intent {
    // Keep two parallel token streams:
    //   `tokens`  — cleaned lowercase, used for keyword matching
    //   `raw_tokens` — case-preserving, used for PersonName extraction
    //                  (so "Дәулет" isn't turned into "дәулет")
    //
    // **v4.2.5** — digit characters now pass the filter. Pre-v4.2.5
    // both streams stripped digits via `c.is_alphabetic() || '-'`,
    // so a literal age like `30` in "менің жасым 30" never reached
    // `parse_kazakh_age` and `Intent::StatementOfAge` came out with
    // `years: None`. The age slot then never made it into session,
    // so AskAge couldn't surface a stored value via `belief_direct_answer`.
    // Surfaced by the v4.2.1 cognitive eval expansion's
    // `aspirational_direct_answer_age_surfaces_stored_value` scenario.
    let tokens: Vec<String> = input
        .split_whitespace()
        .map(|t| {
            t.chars()
                .filter(|c| c.is_alphabetic() || c.is_ascii_digit() || *c == '-')
                .collect::<String>()
                .to_lowercase()
        })
        .filter(|t| !t.is_empty())
        .collect();
    let raw_tokens: Vec<String> = input
        .split_whitespace()
        .map(|t| {
            t.chars()
                .filter(|c| c.is_alphabetic() || c.is_ascii_digit() || *c == '-')
                .collect::<String>()
        })
        .filter(|t| !t.is_empty())
        .collect();
    let joined = tokens.join(" ");

    // StatementOfName must come BEFORE greeting: "hi i am John" starts
    // with "hi" which would otherwise trip Greeting::Casual. The
    // statement-of-name rule requires an explicit pattern (атым/есімім/
    // зовут/my name is/call me/[greet] i am X) so false positives from
    // a bare greeting are ruled out.
    if let Some(name) = detect_statement_of_name(&tokens, &raw_tokens, &joined) {
        return Intent::StatementOfName { name };
    }
    if detect_ask_how_are_you(&joined) {
        return Intent::AskHowAreYou;
    }
    if let Some(g) = detect_greeting(&tokens, &joined) {
        return g;
    }
    if detect_farewell(&tokens, &joined) {
        return Intent::Farewell;
    }
    // Order matters: thanks/apology should be checked before affirmation,
    // because "рахмет" is a single-token thanks and shouldn't accidentally
    // fall into affirmation if we ever add "рахмет" there.
    if detect_thanks(&tokens, &joined) {
        return Intent::Thanks;
    }
    if detect_apology(&tokens, &joined) {
        return Intent::Apology;
    }
    // **v4.42.7** — disagreement / correction detection. User is
    // pushing back on adam's previous answer. Don't engage the
    // topic-extraction path (which surfaces the same disputed fact
    // again). Route to dedicated ack template family.
    if detect_user_disagreement(&joined) {
        return Intent::UserDisagrees;
    }
    // **v4.3.3 / v4.3.4** — `сен кімсің` / `сені кім жасады` /
    // `қашан пайда болдың` / `ерекшелігің не` ask about adam's
    // identity (general / creator / birthdate / architecture
    // aspects). The detector returns the `SystemAspect` it matched
    // so the planner can pick the right `ask_about_system.*`
    // template family. Must be checked BEFORE `detect_ask_name`.
    if let Some(aspect) = detect_ask_about_system(&tokens, &joined, input) {
        return Intent::AskAboutSystem { aspect };
    }
    // **v4.14.0** — curriculum-content question detector.
    // Pattern: subject in {`оқушы`, `студент`, `шәкірт`, `бала`} +
    // education locus (`мектеп`, `сабақ`, `сыныпта`, `университет`)
    // + question word `не` + learning verb (`оқу`, `үйрену`,
    // `өту`). Catches «Оқушылар мектепте физика пәнінен не оқиды?»
    // — pre-v4.14.0 surfaced a greedy IsA fact about the first
    // content noun (`оқушы` → `Оқушы мектеп құрамына кіреді`).
    // The honest fallback says: this is curriculum content, I
    // don't have it. Distinct from `MultiTopicCapability` (v4.13.5,
    // about LISTING subjects), `Knowledge` (v4.6.0, about adam's
    // domain breadth), and `Limitations` (v4.6.0, about what adam
    // can't do).
    if detect_curriculum_content_question(&joined) {
        return Intent::AskCurriculumContent;
    }
    // **v4.17.5** — willingness/readiness question. Detected
    // BEFORE Compliment / SelfComparison so the v4.17.5
    // tightening of those detectors doesn't accidentally let
    // these turns fall through. Surface forms: «дайынсыз ба
    // үйренуге», «жақсаруға ашықсыз ба», «жақсырақ болуға
    // дайынсың ба».
    if detect_ask_willingness(&joined) {
        return Intent::AskWillingness;
    }
    // **v4.95.0** — student submission detection runs FIRST among
    // pedagogical intents. The triple-backtick code block is a much
    // stronger signal than the verb-noun heuristics used by the
    // P2 detectors below, and a code submission shouldn't risk
    // routing to AskExercise / CodeRequest / ExplainCompilerError
    // just because it mentions «жаттығу» / «код» / «қате» in the
    // surrounding prose.
    if let Some((code, topic)) = detect_submit_solution(input) {
        return Intent::SubmitSolution { code, topic };
    }
    // **v4.99.0** — student-side curriculum-query intents. Routed
    // BEFORE the pedagogical detectors so «келесі тақырыпты
    // үйренсем?» doesn't trip AskExercise on the surrounding
    // «үйренсем» (= "let me learn") + AskPurpose on «не».
    if detect_ask_next_topic(input) {
        return Intent::AskNextTopic;
    }
    if detect_ask_current_progress(input) {
        return Intent::AskCurrentProgress;
    }
    // **v4.96.0** — Codex round-2 audit Bug 7: cross-language
    // contrast. Routed BEFORE the other pedagogical detectors so
    // «Python-да ownership бар ма?» doesn't accidentally match
    // AskPurpose's «бар» / «ма» on the surrounding ownership token.
    if let Some((other_language, rust_concept)) = detect_cross_language_contrast(input) {
        return Intent::CrossLanguageContrast {
            other_language,
            rust_concept,
        };
    }
    // **v4.93.5** — pedagogical intents (Codex 2026-05-07 audit P2).
    // Routed BEFORE ask-name so «жаттығу беріңіз» / «код жазып
    // беріңіз» / «E0382 қатесін түсіндіріңіз» / «X-нің мақсаты не?»
    // bypass the generic-noun fallback that surfaced unrelated
    // dictionary definitions pre-v4.93.5.
    if let Some(code) = detect_explain_compiler_error(&joined, &raw_tokens) {
        let topic = pedagogical_topic_hint(input);
        return Intent::ExplainCompilerError {
            error_code: code,
            topic,
        };
    }
    if detect_code_request(&joined) {
        let topic = pedagogical_topic_hint(input);
        return Intent::CodeRequest { topic };
    }
    if detect_ask_exercise(&joined) {
        let topic = pedagogical_topic_hint(input);
        return Intent::AskExercise { topic };
    }
    if detect_ask_purpose(&joined) {
        let topic = pedagogical_topic_hint(input);
        return Intent::AskPurpose { topic };
    }
    if detect_ask_name(&joined) {
        return Intent::AskName;
    }
    // Statement-* is checked BEFORE Ask-* inside each topic pair: a
    // 1st-person marker ("келдім", "тұрамын", "жасым") unambiguously
    // means the user is stating, not asking. Without this ordering,
    // "қайдан келдім" would hit AskLocation (because of "қайдан")
    // before StatementOfLocation (which keys on "келдім").
    // **v4.4.5** — `detect_ask_age` runs BEFORE
    // `detect_statement_of_age` so a 1sg-self-recall like
    // `менің жасым қанша?` reaches `AskAge` instead of being
    // swallowed by `StatementOfAge { years: None }`. The 1sg-form
    // overlap (`жасым` is both "my age (statement)" and "my age
    // (in a self-recall question)") forced the change; the
    // ask-form detector now also matches `жасым + қанша/неше` and
    // the statement-form detector refuses to match when a
    // question particle is present.
    if detect_ask_age(&joined) {
        return Intent::AskAge;
    }
    if let Some(years) = detect_statement_of_age(&tokens, &joined) {
        return Intent::StatementOfAge { years };
    }
    // **v4.73.0** — gate `detect_statement_of_location` on input NOT
    // containing a discourse anaphor (оны / ол / бұл / сол / etc.).
    // Codex 2026-05-06 review surfaced critical security bug: «Оны
    // қалай шешеміз?» (anaphoric "how do we solve it?", referring to
    // a previously-discussed equation) hit StatementOfLocation
    // because (a) `шешеміз` is a 1Pl verb that satisfies
    // `has_first_person_location_context`, and (b) FST analysed
    // «Оны» (Acc of pronoun «ол») as a noun candidate, recovering
    // it as a "city" via the rule-based fallback. The system then
    // saved «Оны» as the user's city in session — a false biographical
    // claim. Discourse anaphors are NEVER first-person location
    // statements; if input contains one, leave intent as Unknown so
    // the existing v4.13.0 anaphora-resolution path (lines 668-708 of
    // conversation.rs) can surface the prior topic.
    if !crate::discourse::input_contains_discourse_anaphor(input)
        && let Some(city) = detect_statement_of_location(&tokens, &raw_tokens, &joined, parses)
    {
        return Intent::StatementOfLocation { city };
    }
    if detect_ask_location(&joined) {
        return Intent::AskLocation;
    }
    if let Some(occupation) =
        detect_statement_of_occupation(&tokens, &raw_tokens, &joined, lexicon, parses)
    {
        return Intent::StatementOfOccupation { occupation };
    }
    if detect_ask_occupation(&joined) {
        return Intent::AskOccupation;
    }
    // **v4.51.0** — activity statement: «X-ны әзірлеймін»,
    // «X-мен айналысамын», «X жасаймын». Distinct from occupation
    // statement (profession label); captures the CURRENT-WORK
    // CONTENT. Runs AFTER occupation so cleaner forms like «мен
    // дәрігермін» go to occupation; activity verbs are the
    // discriminating signal.
    if detect_ask_activity(&joined) {
        return Intent::AskActivity;
    }
    if let Some(activity) = detect_statement_of_activity(&tokens, &joined) {
        return Intent::StatementOfActivity { activity };
    }
    if detect_statement_of_family(&joined) {
        return Intent::StatementOfFamily;
    }
    if detect_ask_family(&joined) {
        return Intent::AskFamily;
    }
    if detect_statement_of_weather(&tokens, &joined) {
        return Intent::StatementOfWeather;
    }
    if detect_ask_weather(&joined) {
        return Intent::AskWeather;
    }
    if detect_ask_time(&joined) {
        return Intent::AskTime;
    }
    if detect_compliment(&tokens, &joined) {
        return Intent::Compliment;
    }
    if detect_request(&tokens, &joined) {
        return Intent::Request;
    }
    if detect_well_wishes(&joined) {
        return Intent::WellWishes;
    }
    if detect_insult(&tokens, &joined) {
        return Intent::Insult;
    }
    // **v4.6.20** — long, gracious user-acknowledgement: «Мен
    // сенің әлі бәрін білмейтініңді … түсіндім». Detected by the
    // pair (addressee + 1sg perfective realisation verb) in
    // `discourse::input_is_user_acknowledgement`. Must be checked
    // BEFORE the noun-hint fallback so the greedy `әлі`-grabbing
    // path is suppressed. Not gated on `pronoun` because the
    // realisation verb itself carries 1sg agreement.
    if crate::discourse::input_is_user_acknowledgement(&joined) {
        return Intent::UserAcknowledgement;
    }
    if detect_statement_of_wellbeing(&tokens, &joined) {
        return Intent::StatementOfWellbeing;
    }
    if detect_affirmation(&tokens, &joined) {
        return Intent::Affirmation;
    }
    if detect_negation(&tokens, &joined) {
        return Intent::Negation;
    }

    // Unknown — but try to extract a noun hint from the parses so the
    // fallback response can at least acknowledge context. `example` is
    // filled later by `Conversation::turn` via retrieval (v1.6.5).
    // `example_adapted` (v1.9.5) is also set there.
    // v4.0.21 — prefer multi-word entity match before single-noun fallback
    // so «Құс жолы туралы айтшы» stays intact (not reduced to «құс»).
    // **v4.37.5** — best_noun_hint_with_confidence threads a
    // TopicConfidence band through every extraction strategy so the
    // planner can route low-confidence picks to the
    // `unknown.clarify_low_confidence` family.
    let (mut noun_hint, mut noun_hint_confidence): (Option<String>, TopicConfidence) =
        match best_noun_hint_with_confidence(input, parses) {
            Some((root, conf)) => (Some(root), conf),
            None => (None, TopicConfidence::High),
        };
    // **v4.12.0** — detect question shape at the same point we
    // populate `noun_hint`. Pure surface-level scan; cheap and
    // independent of the FST analyses. `None` for non-questions.
    let question_shape = crate::question_shape::detect(input);
    // **v4.23.0** — detect temporal-scope queries (kese / büginnen
    // / etc. + question marker). Routes to `unknown.temporal_no_data`
    // for an honest "no time-series data" answer instead of letting
    // the topic extractor fall through to a tangential general fact
    // about the non-temporal subject.
    let temporal_scope = detect_temporal_scope_question(input);
    // **v4.23.5** — detect compositional possessive function
    // questions: `X-Genitive Y-Possessive + (не атқарады / не
    // істейді / неге қажет / не үшін керек / қандай қызмет ...)`.
    // Routes to `unknown.compositional_function.*` so the response
    // can hedge honestly when only structural facts are available
    // for the owned part Y.
    let compositional_function = detect_compositional_function_question(input);
    // **v4.14.5** — sentence_decomp fallback. When greedy
    // `best_noun_hint` returns None (FST recovered nothing
    // useful), but `sentence_decomp::decompose` resolved a
    // structural focus (Subject / Object / Source / Locus), use
    // that as fallback. STRICTLY additive: existing turns that
    // already have a noun_hint are bit-identical pre/post-v4.14.5.
    // Limited to focus_role ∈ {Subject, Object, Source, Locus} so
    // we don't promote a bare predicate root as a topic noun.
    if noun_hint.is_none() {
        let decomp = crate::sentence_decomp::decompose(input, parses, None);
        if let Some(focus) = decomp.focus.as_deref() {
            // Re-apply the same NOT_A_TOPIC filter `first_noun_root`
            // uses, so the v4.4.10 closed-class additions
            // (`қысқа` / `ештеңе` / etc) remain effective on this
            // fallback path too. sentence_decomp keeps its own
            // smaller closed-class list (it filters tokens during
            // decomposition); without this re-check the fallback
            // could promote a discourse particle that semantics.rs
            // explicitly rejects. Anti-regression:
            // `qysqasy_does_not_get_extracted_as_topic`.
            let lower = focus.to_lowercase();
            let is_closed_class = NOT_A_TOPIC.iter().any(|s| *s == lower);
            if !focus.is_empty()
                && !is_closed_class
                && matches!(
                    decomp.focus_role,
                    Some(crate::sentence_decomp::Role::Subject)
                        | Some(crate::sentence_decomp::Role::Object)
                        | Some(crate::sentence_decomp::Role::Source)
                        | Some(crate::sentence_decomp::Role::Locus)
                )
            {
                noun_hint = Some(focus.to_string());
                // v4.37.5 — sentence_decomp identifies a *structural*
                // role (Subject/Object/Source/Locus), which is a
                // strong topical signal — confidence stays High.
                noun_hint_confidence = TopicConfidence::High;
            }
        }
    }
    Intent::Unknown {
        raw_tokens: tokens,
        noun_hint,
        example: None,
        grounded_fact: None,
        example_adapted: false,
        reasoning_chain: None,
        question_shape,
        temporal_scope,
        compositional_function,
        // v4.33.5 — populated downstream in Conversation::turn after
        // sem_frames is built (semantics.rs has no access to FST
        // analyses here). Default Affirmative preserves all pre-
        // v4.33.5 routing exactly.
        noun_hint_polarity: adam_kernel_fst::Polarity::Affirmative,
        input_modality: None,
        input_evidence: None,
        input_is_inversion_question: false,
        noun_hint_confidence,
    }
}

/// Legacy-compatible wrapper: runs intent recognition on parse surface
/// forms only. Kept for tests that don't have raw input handy.
pub fn interpret(parses: &[Analysis]) -> Intent {
    let tokens = surface_tokens(parses);
    let joined = tokens.join(" ");

    if let Some(g) = detect_greeting(&tokens, &joined) {
        return g;
    }
    if detect_farewell(&tokens, &joined) {
        return Intent::Farewell;
    }
    if detect_thanks(&tokens, &joined) {
        return Intent::Thanks;
    }
    if detect_apology(&tokens, &joined) {
        return Intent::Apology;
    }
    // **v4.42.7** — disagreement / correction detection. User is
    // pushing back on adam's previous answer. Don't engage the
    // topic-extraction path (which surfaces the same disputed fact
    // again). Route to dedicated ack template family.
    if detect_user_disagreement(&joined) {
        return Intent::UserDisagrees;
    }
    if detect_ask_how_are_you(&joined) {
        return Intent::AskHowAreYou;
    }
    if detect_ask_name(&joined) {
        return Intent::AskName;
    }
    if detect_statement_of_wellbeing(&tokens, &joined) {
        return Intent::StatementOfWellbeing;
    }
    if detect_affirmation(&tokens, &joined) {
        return Intent::Affirmation;
    }
    if detect_negation(&tokens, &joined) {
        return Intent::Negation;
    }

    // Legacy path has no raw input, so multi-word matching is skipped —
    // callers using raw-text-aware `interpret_text_with_lexicon` get the
    // full v4.0.21 multiword treatment.
    // v4.37.5 — legacy parses-only path defaults confidence to High
    // (preserves bit-identical pre-v4.37.5 routing on this path).
    Intent::Unknown {
        raw_tokens: tokens,
        noun_hint: first_noun_root(parses),
        example: None,
        grounded_fact: None,
        example_adapted: false,
        reasoning_chain: None,
        // **v4.12.0** — legacy path has no raw input, so question
        // shape cannot be detected (the detector is surface-level).
        // Always None on this code path.
        question_shape: None,
        // **v4.23.0** — same: temporal-scope detection is surface-
        // level, so the parses-only legacy path can't fire it.
        temporal_scope: false,
        // **v4.23.5** — same: compositional-function detection is
        // surface-level, so the parses-only legacy path can't fire
        // it.
        compositional_function: false,
        // **v4.33.5** — same: noun_hint_polarity defaults to
        // Affirmative on the legacy parses-only path. Real polarity
        // detection requires sem_frames built in Conversation::turn
        // from the rich FST analyses.
        noun_hint_polarity: adam_kernel_fst::Polarity::Affirmative,
        input_modality: None,
        input_evidence: None,
        input_is_inversion_question: false,
        noun_hint_confidence: TopicConfidence::High,
    }
}

fn surface_tokens(parses: &[Analysis]) -> Vec<String> {
    parses
        .iter()
        .map(|a| match a {
            Analysis::Noun { root, .. } => root.root.clone(),
            Analysis::Verb { root, .. } => root.root.clone(),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Individual recognisers.
// Each returns `Some(Intent)` or a `bool` for match/no-match.
// ---------------------------------------------------------------------------

fn detect_greeting(tokens: &[String], joined: &str) -> Option<Intent> {
    // Time-of-day: "қайырлы таң / күн / кеш".
    if joined.contains("қайырлы") {
        let tod = if joined.contains("таң") {
            TimeOfDay::Morning
        } else if joined.contains("кеш") {
            TimeOfDay::Evening
        } else {
            TimeOfDay::Day
        };
        return Some(Intent::Greeting {
            kind: GreetingKind::TimeOfDay(tod),
        });
    }
    // Polite: "сәлеметсіз бе" / "сәлеметсің бе".
    if joined.contains("сәлеметсіз") || joined.contains("сәлеметсің") {
        return Some(Intent::Greeting {
            kind: GreetingKind::Polite,
        });
    }
    // Casual: "сәлем".
    if tokens.first().is_some_and(|t| t == "сәлем") {
        return Some(Intent::Greeting {
            kind: GreetingKind::Casual,
        });
    }
    // v4.4.10 — Introduction proposal: «танысайық» / «танысалық»
    // («let's get acquainted») as a stand-alone opener, plus
    // compound forms «танысып алайық» / «танысып алыңыз».
    // Surfaced by a 2026-04-28 real-REPL trace that landed on the
    // generic unknown refusal pre-v4.4.10. The polite imperative
    // `танысайық` is a 1pl exhortative, not a greeting in the
    // strict sense, but it opens an introduction exchange and
    // belongs in the Greeting bucket so the planner volunteers
    // adam's name and asks for the user's.
    if tokens
        .iter()
        .any(|t| t == "танысайық" || t == "танысалық" || t == "танысыңыз")
        || joined.contains("танысып алайық")
        || joined.contains("танысып алыңыз")
    {
        return Some(Intent::Greeting {
            kind: GreetingKind::IntroProposal,
        });
    }
    None
}

fn detect_farewell(tokens: &[String], joined: &str) -> bool {
    tokens.first().is_some_and(|t| t == "сау" || t == "қош")
        || joined.contains("кездескенше")
        || joined.contains("сау бол")
        || joined.contains("қош бол")
        || joined.contains("аман бол")
}

fn detect_affirmation(tokens: &[String], joined: &str) -> bool {
    if tokens.len() == 1 {
        let w = &tokens[0];
        return matches!(
            w.as_str(),
            "иә" | "ия" | "дұрыс" | "рас" | "мақұл" | "әрине"
        );
    }
    joined.contains("дұрыс айтасыз") || joined == "иә дұрыс"
}

fn detect_negation(tokens: &[String], joined: &str) -> bool {
    if tokens.len() == 1 {
        let w = &tokens[0];
        return matches!(w.as_str(), "жоқ" | "қате" | "емес");
    }
    joined.contains("жоқ емес") || joined.starts_with("жоқ")
}

// --- v0.7.5 new recognisers ------------------------------------------------

fn detect_thanks(tokens: &[String], joined: &str) -> bool {
    tokens
        .iter()
        .any(|t| matches!(t.as_str(), "рахмет" | "рахметім" | "рақмет"))
        || joined.contains("көп рахмет")
        || joined.contains("көп рақмет")
}

fn detect_apology(tokens: &[String], joined: &str) -> bool {
    tokens
        .iter()
        .any(|t| matches!(t.as_str(), "кешіріңіз" | "кешір" | "ғафу"))
        || joined.contains("кешір")
        || joined.contains("ғафу ет")
}

/// **v4.42.7** — Detect a disagreement / correction signal from the
/// user. Triggers `Intent::UserDisagrees` so the planner picks the
/// `disagreement_ack` template family instead of re-surfacing the
/// disputed fact.
///
/// Markers:
///   - «қателесесің» / «қателесесіз» — "you are wrong" (2sg
///     informal / polite).
///   - «дұрыс емес» / «дұрыс емессіз» — "[that is] not right".
///   - «олай емес» / «бұлай емес» — "not so" / "not like that".
///   - «теріс» — "wrong / incorrect" (when paired with адам / сіз
///     — to avoid false-positive on geography facts about reverse
///     direction).
///   - «бұл қате» / «қателесіп тұрсыз» — explicit "this is a
///     mistake" / "you are mistaking".
///
/// Light detector — does NOT extract WHAT was incorrect or model
/// the correction's content. Future bundles may add proper
/// correction-content extraction (filed in
/// `project_retrieval_not_neural_v2.md` Stage A roadmap).
fn detect_user_disagreement(joined: &str) -> bool {
    joined.contains("қателесесің")
        || joined.contains("қателесесіз")
        || joined.contains("қателесіп тұр")
        || joined.contains("қателесіп")
        || joined.contains("дұрыс емес")
        || joined.contains("олай емес")
        || joined.contains("бұлай емес")
        || joined.contains("бұл қате")
        || joined.contains("сіз қате")
        || joined.contains("сен қате")
}

fn detect_ask_how_are_you(joined: &str) -> bool {
    joined.contains("қалайсың")
        || joined.contains("қалайсыз")
        || joined.contains("жағдайың қалай")
        || joined.contains("жағдайыңыз қалай")
        || joined.contains("халің қалай")
        || joined.contains("халіңіз қалай")
        || joined.contains("қалдарың қалай")
        || joined.contains("қалдарыңыз қалай")
        || joined == "қалың қалай"
        // **v4.6.12** — polite singular/plural forms surfaced by a
        // 2026-04-29 real-REPL transcript: «Қалыңыз қалай?». Maps
        // to «How is your state?» / «How are you (polite)?» —
        // standard Kazakh greeting-inquiry. Also covers the bare
        // 2sg-informal «қалың қалай» that was already there as an
        // exact-match (now matched as substring for robustness in
        // sentences like «Айтшы, қалың қалай?»).
        || joined.contains("қалыңыз қалай")
        || joined.contains("қалың қалай")
}

/// **v4.3.3** — match identity-question phrasings clearly addressed
/// to adam itself: pronoun + identity question.
///
/// Triggers (informal `сен` and formal `сіз` paired):
/// - `сен кімсің` / `сіз кімсіз` ("who are you")
/// - `сен қандай моделсің` / `сіз қандай моделсіз` ("what kind of
///   model are you")
/// - `сен қандайсың` / `сіз қандайсыз` ("what are you like" — used
///   in "what kind of system" sense in this dialog domain)
/// - `сен немен айналысасың` / `сіз немен айналысасыз` ("what do
///   you do" — addressed to system as worker)
///
/// Does NOT include the bare `атың кім` / `есіміңіз` phrasings —
/// those stay with `detect_ask_name` (and the v4.2.5 slot-aware
/// path) to preserve the v4.2.5 cognitive scenarios that exercise
/// the AnswerDirect rendering for stored user names. The
/// pronoun-led patterns here are unambiguously about adam.
/// **v4.14.0** — curriculum-content question detector.
///
/// Pattern: subject (student-class noun) + education locus +
/// question word `не` + learning verb. Conservative — requires all
/// four signals so a generic «оқушылар туралы не білесіз?» (the
/// IsA-of-students question) doesn't accidentally route here.
///
/// Surface anchors:
/// - subject: `оқушы` / `оқушылар` / `студент` / `шәкірт` / `бала`
///   (NOT `балалар` if used as "kids" generically — paired with
///   education locus to disambiguate)
/// - education locus: `мектеп` / `сабақ` / `сыныпта` / `университет`
/// - question word: `не` (the WHAT)
/// - learning verb: `оқу` / `оқиды` / `үйрену` / `үйренеді` / `өту`
fn detect_curriculum_content_question(joined: &str) -> bool {
    let has_student =
        joined.contains("оқушы") || joined.contains("студент") || joined.contains("шәкірт");
    let has_education_locus = joined.contains("мектеп")
        || joined.contains("сабақ")
        || joined.contains("сыныпта")
        || joined.contains("сыныпқа")
        || joined.contains("университет")
        || joined.contains("колледж");
    let has_what = joined.contains("не ")
        || joined.contains("не?")
        || joined.ends_with("не")
        || joined.contains("нені")
        || joined.contains("неден");
    let has_learning_verb = joined.contains("оқиды")
        || joined.contains("оқимыз")
        || joined.contains("оқисың")
        || joined.contains("оқисыз")
        || joined.contains("үйренеді")
        || joined.contains("үйрене");
    has_student && has_education_locus && has_what && has_learning_verb
}

/// **v4.23.0** — temporal-scope question detector. Returns `true`
/// when the input contains a temporal adverb (`кеше / бүгін /
/// ертең / қазір / бұрын / былтыр / келесі`) co-occurring with a
/// question marker (question word `қандай / не / қашан / қалай /
/// неше / қанша` or yes/no particle `ма/ме/ба/бе/па/пе`). The
/// pattern flags queries about *state at a specific point in
/// time* — e.g. «Кеше ауа райы қандай болды?» («What was the
/// weather yesterday?») — which adam has no time-series data for.
///
/// Routes to `unknown.temporal_no_data` for an honest "I don't
/// track time-bound state" answer instead of letting the topic
/// extractor fall through to a tangential general fact about the
/// non-temporal subject (the post-v4.22.5 behaviour where кеше
/// was filtered out and the response collapsed to a fact about
/// `ауа` — accurate about air in general, but missed the actual
/// "what was yesterday's weather" question).
///
/// **What this does NOT catch:** temporal markers without a
/// question (e.g. statement «Кеше ауа райы жақсы болды.» —
/// detector returns false because no question marker is present;
/// the existing path handles it as a statement). Also excluded:
/// clock-time questions like «Қазір сағат қанша?» where adam
/// could in principle integrate a clock — those would route to
/// a different specialised handler when added.
fn detect_temporal_scope_question(input: &str) -> bool {
    let lower = input.to_lowercase();
    // Temporal adverb anchors. Each must appear as a whole token —
    // we check `surrounded by spaces or string boundaries` via the
    // simple `.contains` after padding the input. A bare substring
    // test is acceptable here because all the listed forms are
    // distinct enough not to appear inside content nouns (e.g.
    // «бүгін» doesn't substring any common Kazakh content noun).
    let has_temporal = [
        "кеше",
        "бүгін",
        "ертең",
        "қазір",
        "бұрын",
        "былтыр",
        "келесі",
    ]
    .iter()
    .any(|adv| {
        lower.split_whitespace().any(|tok| {
            // Strip trailing punctuation / case-suffix garbage.
            let cleaned: String = tok
                .chars()
                .filter(|c| c.is_alphabetic() || *c == '-')
                .collect();
            cleaned == *adv
        })
    });
    if !has_temporal {
        return false;
    }
    // Question marker: question word OR question particle. Mirrors
    // the same set the v4.12.0 question_shape detector uses.
    let has_question_word = lower.contains("қандай")
        || lower.contains("қашан")
        || lower.contains("қалай")
        || lower.contains("неше")
        || lower.contains("қанша")
        || lower.contains("неліктен")
        || lower.contains("неге")
        // «не» as a standalone question word — must be a separate
        // token, not a fragment of a longer word.
        || lower.split_whitespace().any(|t| t == "не" || t == "не?");
    let has_question_particle = lower.contains(" ма?")
        || lower.contains(" ме?")
        || lower.contains(" ба?")
        || lower.contains(" бе?")
        || lower.contains(" па?")
        || lower.contains(" пе?")
        || lower.ends_with("ма?")
        || lower.ends_with("ме?")
        || lower.ends_with("ба?")
        || lower.ends_with("бе?")
        || lower.ends_with("па?")
        || lower.ends_with("пе?");
    has_question_word || has_question_particle
}

/// **v4.23.5** — compositional possessive function-question
/// detector. Returns `true` when the input has the structural
/// shape `X-Genitive Y-Possessive + function-asking phrase`.
///
/// Examples it catches:
/// - «Жасушаның ядросы не атқарады?»
/// - «Митохондрияның қызметі қандай?»
/// - «Атомның ядросы неге қажет?»
/// - «Машинаның мотор не істейді?»
/// - «Тілдің рөлі не үшін керек?»
///
/// Detection is intentionally lightweight — it doesn't require
/// the FST to confirm Genitive + Possessive case marking on the
/// nouns, just looks for the surface signal pattern:
///
/// 1. SOME token ends in a Genitive suffix (`-ның / -нің / -тың /
///    -тің / -дың / -дің`).
/// 2. SOME token ends in a 3sg-Possessive suffix (`-сы / -сі /
///    -ы / -і / -ысы / -ісі`).
/// 3. The input contains at least one function-asking phrase from
///    a small closed list.
///
/// All three must be present. Conservative — false positives
/// would route legitimate questions to a hedge template, but the
/// hedge is mild (acknowledges the structural fact, says "no
/// functional data") so the cost of over-firing is low.
///
/// Why this matters: post-v4.22.5 the topic extractor correctly
/// picks `ядро` for «Жасушаның ядросы не атқарады?», but the
/// available world_core fact is structural (`Ядро жасуша
/// құрамына кіреді`), and the response template surfaced that
/// fact verbatim — answering "the nucleus is part of the cell"
/// instead of "what does the nucleus do". Routes the planner to
/// `unknown.compositional_function.with_fact` (when grounded_fact
/// is available — surface it as the structural-only fact we have)
/// or `unknown.compositional_function.bare` (when no fact at all).
fn detect_compositional_function_question(input: &str) -> bool {
    let lower = input.to_lowercase();
    // Tokenise on whitespace, strip punctuation tail.
    let tokens: Vec<String> = lower
        .split_whitespace()
        .map(|tok| {
            tok.chars()
                .filter(|c| c.is_alphabetic() || *c == '-')
                .collect::<String>()
        })
        .filter(|t| !t.is_empty())
        .collect();
    // (1) Genitive suffix — must appear on at least one token,
    // not at the very end of the sentence (Genitive is always
    // followed by something).
    let has_genitive = tokens.iter().any(|t| {
        t.ends_with("ның")
            || t.ends_with("нің")
            || t.ends_with("тың")
            || t.ends_with("тің")
            || t.ends_with("дың")
            || t.ends_with("дің")
    });
    if !has_genitive {
        return false;
    }
    // (2) 3sg-Possessive suffix on a different token. Order long
    // suffixes first so we don't accidentally match `-сы` inside
    // `-ысы`.
    let has_possessive = tokens.iter().any(|t| {
        t.ends_with("ысы")
            || t.ends_with("ісі")
            || t.ends_with("сы")
            || t.ends_with("сі")
            || (t.ends_with('ы') && t.chars().count() >= 3)
            || (t.ends_with('і') && t.chars().count() >= 3)
    });
    if !has_possessive {
        return false;
    }
    // (3) Function-asking phrase. Closed list — covers the common
    // ways Kazakh asks "what does Y do / what is Y for / what is
    // Y's role".

    lower.contains("не атқарады")
        || lower.contains("не атқарад")
        || lower.contains("не істейді")
        || lower.contains("не істей")
        || lower.contains("не үшін керек")
        || lower.contains("неге қажет")
        || lower.contains("қандай қызмет")
        || lower.contains("қандай рөл")
        || lower.contains("қандай міндет")
        || lower.contains("қалай жұмыс іс")
        || lower.contains("рөлі қандай")
        || lower.contains("қызметі қандай")
        || lower.contains("міндеті қандай")
}

/// **v4.17.5** — willingness / readiness-to-improve detector.
/// Pattern: `(дайынсыз/дайынсың/ашықсыз/ашықсың) ба + verb-уге`
/// where the verb expresses growth (`үйрену / жақсару / дамыту /
/// өсу / жетілдір`). Conservative — requires both the readiness
/// marker AND a growth-verb so generic «дайын ба» doesn't
/// accidentally fire.
/// **v4.93.5** — Codex 2026-05-07 audit P2: detect "give me an
/// exercise / task / problem" requests. Pre-fix queries like
/// «жаттығу беріңіз» fell through to topic extraction on the noun
/// `жаттығу` (= "exercise"), surfacing the dictionary definition
/// instead of generating an exercise.
fn detect_ask_exercise(joined: &str) -> bool {
    // Trigger: noun «жаттығу/тапсырма/есеп/практика» + give-verb
    // («бер»/«бер...»/«ұсын»/«құрастыр») or modal-necessity with the
    // practice noun («жаттығу керек» = "I need an exercise").
    // Conservative — requires both pieces so a bare «жаттығу деген
    // не?» (definition request) doesn't fire here.
    let has_practice_noun = joined.contains("жаттығу")
        || joined.contains("тапсырма")
        || joined.contains("есеп бер")
        || joined.contains("практика")
        || joined.contains("упражнение");
    let has_give_verb = joined.contains("бер")
        || joined.contains("ұсын")
        || joined.contains("құрастыр")
        || joined.contains("жасайын")
        || joined.contains("жаттыққым")
        || joined.contains("жаттығайын")
        // **v4.93.5** — modal forms «X керек / қажет» also signal
        // an exercise request when paired with a practice noun.
        // Live test: «lifetime жаттығуы керек.» pre-fix routed to
        // modal-Necessity hedge instead of the new ask_exercise path.
        || joined.contains("керек")
        || joined.contains("қажет");
    // Refuse definitional requests like «жаттығу деген не?» / «жаттығу
    // дегеніміз не?» — those should surface the dictionary entry.
    let is_definitional = joined.contains("деген не")
        || joined.contains("дегеніміз не")
        || joined.contains("деген сөз")
        || joined.contains("дегеніміз — не");
    // **v4.93.5** — refuse when the sentence is an acknowledgement.
    // «...көп жаттығу керек екенін түсіндім» (= "I understood a
    // lot of practice is needed") is the user reflecting on adam's
    // limits, not asking for an exercise. Existing `end_to_end`
    // test caught this regression. Acknowledgement markers:
    // түсіндім / түсіндік / мақұл / келісемін / қабыл аламын.
    let is_acknowledgement = joined.contains("түсіндім")
        || joined.contains("түсіндік")
        || joined.contains("мақұл")
        || joined.contains("келісемін")
        || joined.contains("қабыл алам");
    has_practice_noun && has_give_verb && !is_definitional && !is_acknowledgement
}

/// **v4.93.5** — Codex P2: detect "write code / show example" requests.
fn detect_code_request(joined: &str) -> bool {
    // Either explicit «код жаз/көрсет/бер» / «мысал бер» / «программа
    // жаз», OR a "show me Hello World" style framed as imperative.
    let has_code_noun = joined.contains("код")
        || joined.contains("snippet")
        || joined.contains("программа")
        || joined.contains("listing");
    let has_show_or_write_verb = joined.contains("жаз")
        || joined.contains("көрсет")
        || joined.contains("бер")
        || joined.contains("ұсын");
    let has_example_request = (joined.contains("мысал") || joined.contains("үлгі"))
        && (joined.contains("бер")
            || joined.contains("көрсет")
            || joined.contains("жаз")
            // **v4.94.5** — modal form «мысалы керек» also signals
            // example request. Live test: «lifetime мысалы керек.»
            // pre-fix routed to modal-Necessity hedge.
            || joined.contains("керек")
            || joined.contains("қажет"));
    // **v4.93.5** — modal forms «код керек / қажет» also signal a
    // code request. Live test: «Маған ownership коды керек.» pre-fix
    // routed to modal-Necessity hedge.
    let has_code_modal = has_code_noun && (joined.contains("керек") || joined.contains("қажет"));
    let is_definitional = joined.contains("деген не") || joined.contains("дегеніміз не");
    if is_definitional {
        return false;
    }
    (has_code_noun && has_show_or_write_verb) || has_example_request || has_code_modal
}

/// **v4.93.5** — Codex P2: detect "explain this compiler error / E0xxx".
/// Returns Some(error_code_uppercase) when an `E0xxx` code is present;
/// Some(None) coverage handled at call site as `error_code = None` for
/// generic «бұл қате не білдіреді» framings without a numeric code.
fn detect_explain_compiler_error(joined: &str, raw_tokens: &[String]) -> Option<Option<String>> {
    // Numeric code E0xxx (case-insensitive). Pre-extract from raw
    // tokens so we preserve the canonical uppercase form.
    let code = raw_tokens.iter().find_map(|t| {
        let lower = t.to_lowercase();
        let lower = lower.trim_end_matches([',', '.', '?', '!', ':', ';', ')']);
        let lower = lower.trim_start_matches('(');
        if lower.starts_with('e')
            && lower.len() >= 2
            && lower[1..].chars().all(|c| c.is_ascii_digit())
        {
            Some(lower.to_uppercase())
        } else {
            None
        }
    });
    if code.is_some() {
        return Some(code);
    }
    // **v4.93.5** — map common Rust error TEXT patterns to canonical
    // E-codes when the user pastes the error message rather than
    // typing the code. Order: most-specific first (longer match).
    let has_kz_error_marker = joined.contains("қате") || joined.contains("түсіндір");
    if !has_kz_error_marker {
        return None;
    }
    if joined.contains("cannot borrow as mutable more than once")
        || joined.contains("more than one mutable borrow")
    {
        return Some(Some("E0499".to_string()));
    }
    if joined.contains("cannot borrow as mutable") {
        return Some(Some("E0596".to_string()));
    }
    if joined.contains("cannot move out") {
        return Some(Some("E0507".to_string()));
    }
    if joined.contains("does not live long enough") {
        return Some(Some("E0597".to_string()));
    }
    if joined.contains("use of moved value") {
        return Some(Some("E0382".to_string()));
    }
    // Generic Rust error markers — fire intent without specific code.
    if joined.contains("cannot borrow")
        || joined.contains("borrow checker")
        || (joined.contains("expected") && joined.contains("found"))
    {
        return Some(None);
    }
    None
}

/// **v4.93.5** — Codex P2: detect "what is the purpose of X / why is
/// X for". Distinct from the v4.93.0 function-asking-phrase fix
/// (which catches «X не үшін керек?») — this one catches the
/// possessive form «X-нің мақсаты не?» / «X-тің мақсаты не?» plus
/// shorter «X не үшін?».
fn detect_ask_purpose(joined: &str) -> bool {
    // **v4.93.5** — keep this NARROW. «мәні» (essence/value) is too
    // broad — it collides with «қайтару мәні» (= "return value")
    // and other unrelated noun-phrases. Require explicit purpose
    // markers only.
    let has_purpose_noun = joined.contains("мақсаты")
        || joined.contains("мақсат")
        || joined.contains("міндеті")
        || joined.contains("пайдасы");
    let has_why_marker = joined.contains("не үшін арналған")
        || joined.contains("неге арналған")
        || joined.contains("себебі қандай");
    has_purpose_noun || has_why_marker
}

/// **v4.99.0** — student-side curriculum query: "what's next?".
/// Surface forms: «Келесі қандай тақырыпты үйренсем?», «Енді нені
/// үйренсем болады?», «Әрі қарай қалай?», «Келесі тақырып не?»,
/// «Кейін не үйрену керек?».
pub fn detect_ask_next_topic(input: &str) -> bool {
    let lower = input.to_lowercase();
    // Two-token-conjunction patterns — needs both an "advance" marker
    // AND a learning marker so generic «келесі сұрақ» doesn't match.
    let has_advance = lower.contains("келесі")
        || lower.contains("әрі қарай")
        || lower.contains("әрі-қарай")
        || lower.contains("кейін")
        || lower.contains("енді");
    let has_learn_or_topic =
        lower.contains("үйрен") || lower.contains("тақырып") || lower.contains("сабақ");
    if !(has_advance && has_learn_or_topic) {
        return false;
    }
    // Reject reflective / past-tense uses: «келесі тақырыпты
    // үйрендім» (= "I learned the next topic") is a statement, not
    // a question. Crude check: must contain a question marker.
    lower.contains("?")
        || lower.contains("қалай")
        || lower.contains("не")
        || lower.contains("қандай")
}

/// **v4.99.0** — student-side curriculum query: "where am I?".
/// Surface forms: «Мен қай жерде тұрмын?», «Менің прогресім қандай?»,
/// «Қай тақырыпты бітірдім?», «Қанша тақырыпты үйрендім?», «Жалпы
/// қалай тұр?».
pub fn detect_ask_current_progress(input: &str) -> bool {
    let lower = input.to_lowercase();
    // Direct phrases first — high-precision matches.
    let direct_phrases = [
        "қай жерде тұрмын",
        "қай жердемін",
        "прогресім қандай",
        "прогресім қалай",
        "менің прогресім",
        "қанша тақырып",
        "қай тақырыпты бітір",
        "қай тақырыпты аяқта",
        "қай тақырыптар",
        "жалпы қалай тұр",
        "қандай жетістік",
    ];
    if direct_phrases.iter().any(|p| lower.contains(p)) {
        return true;
    }
    // Compositional: progress noun + question marker.
    let has_progress_noun = lower.contains("прогрес") || lower.contains("жетістік");
    let has_question = lower.contains("?") || lower.contains("қалай") || lower.contains("қандай");
    has_progress_noun && has_question
}

/// **v5.3.5** — compound-statement secondary fact extraction.
///
/// Pre-fix «Менің атым Дәулет, мамандығым бағдарламашы.» fired only
/// `StatementOfName { name: "Дәулет" }` (the first clause); the
/// occupation in the second clause was lost because `interpret_text`
/// returns ONE primary `Intent`. This function scans the input for
/// **additional** profile-fact patterns AFTER the primary intent
/// fires, returning `(predicate, object)` tuples for the caller to
/// absorb separately.
///
/// Patterns detected:
/// - «мамандығым X» / «кәсібім X» / «менің мамандығым X» →
///   `("occupation", X)`
/// - «жасым N» where N is a numeric token (with optional «менің»)
///   → `("age", N)`
///
/// The next non-particle token after the P1Sg-marked profile noun
/// is taken as the value. Trailing punctuation is stripped.
///
/// Used by [`crate::Conversation::turn_with_trace`] post-`absorb_entities`
/// to fill in compound profile statements that the single-Intent
/// pipeline doesn't catch.
pub fn extract_secondary_profile_facts(input: &str) -> Vec<(String, String)> {
    let mut facts = Vec::new();
    let tokens: Vec<&str> = input.split_whitespace().collect();

    fn alpha_only(s: &str) -> String {
        s.chars().filter(|c| c.is_alphabetic()).collect()
    }
    fn alpha_or_digit(s: &str) -> String {
        s.chars()
            .filter(|c| c.is_alphabetic() || c.is_ascii_digit())
            .collect()
    }

    for (i, tok) in tokens.iter().enumerate() {
        let tl = alpha_only(tok).to_lowercase();
        // Occupation: «мамандығым X» / «кәсібім X»
        if tl == "мамандығым" || tl == "кәсібім" {
            // **v5.4.6** — walk past punctuation-only tokens so the
            // em-dash form «Мамандығым — бағдарламашы» reaches the
            // value just like the bare form. Sister fix to the same
            // walk in `detect_occupation_in_compound`.
            let mut j = i + 1;
            while let Some(next) = tokens.get(j) {
                let value = alpha_only(next);
                if value.is_empty() {
                    j += 1;
                    continue;
                }
                let value_lc = value.to_lowercase();
                if value_lc != "болып" && value_lc != "ретінде" && value_lc != "екен"
                {
                    facts.push(("occupation".into(), value));
                }
                break;
            }
        }
        // Age: «жасым 30» / «жасым отыз»
        if tl == "жасым" {
            let mut j = i + 1;
            while let Some(next) = tokens.get(j) {
                let value = alpha_or_digit(next);
                if value.is_empty() {
                    j += 1;
                    continue;
                }
                facts.push(("age".into(), value));
                break;
            }
        }
    }
    facts
}

/// **v4.95.0** — Codex P3 follow-up: detect "student submits a
/// solution" pattern. Returns `Some((code, topic))` when the input
/// contains a triple-backtick code block whose body looks
/// syntactically like Rust. Returns `None` otherwise.
///
/// **Detection logic:**
/// 1. Locate the first ` ```...``` ` block in the input.
/// 2. Strip the optional `rust` / `Rust` language tag on the
///    opening fence.
/// 3. Reject when the body is shorter than 3 chars (probably a
///    snippet of inline code, not a solution).
/// 4. Heuristic acceptance: body contains at least one of `fn `,
///    `let `, `use `, `struct `, `enum ` — common Rust openers.
///    Refuse otherwise (treats `\`\`\`bash` blocks as non-Rust).
/// 5. Topic: re-uses `pedagogical_topic_hint` on the WHOLE input
///    (not just the code body) so the surrounding prose can carry
///    the topic ("Менің ownership шешімім: ```rust ... ```").
/// **v4.96.0** — Codex round-2 audit Bug 7. Detect cross-language
/// contrast questions: «Python-да ownership бар ма?», «Java-да
/// lifetime деген ұғым бар ма?». The pattern is:
///   {NON_RUST_LANGUAGE}-{Locative}? + {RUST_CONCEPT} + {existence/comparison-question}
/// Returns Some((other_language_canonical, rust_concept_canonical))
/// when matched.
pub fn detect_cross_language_contrast(input: &str) -> Option<(String, String)> {
    let lower = input.to_lowercase();
    // Step 1: find a non-Rust language token (Latin word).
    const NON_RUST_LANGS: &[&str] = &[
        "python",
        "java",
        "javascript",
        "js",
        "typescript",
        "ts",
        "go",
        "golang",
        "kotlin",
        "swift",
        "ruby",
        "php",
        "c",
        "cpp",
        "c++",
        "csharp",
        "c#",
        "haskell",
        "ocaml",
        "scala",
        "elixir",
        "erlang",
        "clojure",
    ];
    let mut other_lang: Option<&'static str> = None;
    for &lang in NON_RUST_LANGS {
        // Match as standalone token; cover common Kazakh case
        // suffixes attached via dash («Python-да» / «Java-да»).
        let patterns = [
            format!(" {lang} "),
            format!(" {lang}-"),
            format!("{lang}-да"),
            format!("{lang}-та"),
            format!("{lang}-де"),
            format!("{lang}-те"),
            format!(" {lang}?"),
            format!(" {lang}."),
        ];
        let starts_with_lang = lower.starts_with(&format!("{lang} "))
            || lower.starts_with(&format!("{lang}-"))
            || lower == lang;
        if starts_with_lang || patterns.iter().any(|p| lower.contains(p)) {
            other_lang = Some(lang);
            break;
        }
    }
    let other_lang = other_lang?;
    // Step 2: find a Rust concept word in the same input.
    const RUST_CONCEPTS: &[&str] = &[
        "ownership",
        "borrow",
        "borrowing",
        "lifetime",
        "lifetimes",
        "trait",
        "match",
        "iterator",
        "future",
        "async",
        "await",
        "pin",
        "tokio",
        "stream",
        "result",
        "option",
        "macro",
        "pattern",
        "closure",
    ];
    let mut concept: Option<&'static str> = None;
    for &c in RUST_CONCEPTS {
        if lower.contains(c) && Some(c) != Some(other_lang) {
            concept = Some(c);
            break;
        }
    }
    let concept = concept?;
    // Step 3: existence/comparison question marker.
    let has_question = lower.contains("бар ма")
        || lower.contains("бар ма?")
        || lower.contains("ма?")
        || lower.contains("ба?")
        || lower.contains("деген ұғым")
        || lower.contains("сияқты")
        || lower.contains("сияқ");
    if !has_question {
        return None;
    }
    Some((other_lang.to_string(), concept.to_string()))
}

pub fn detect_submit_solution(input: &str) -> Option<(String, Option<String>)> {
    let start = input.find("```")?;
    let after_open = &input[start + 3..];
    let end_offset = after_open.find("```")?;
    let body_with_tag = &after_open[..end_offset];
    // Strip optional language tag on the first line.
    //
    // **v4.96.0** — Codex round-2 audit Bug 4 fix: the pre-fix
    // logic stripped only when the language tag was followed by a
    // newline (multi-line block). Single-line «```rust let x=5;```»
    // kept «rust » as part of the body, which then failed the
    // Rust-syntax heuristic because of the leading «rust » token
    // before the actual code. Now strip on whitespace OR newline
    // — whichever comes first — and treat the prefix tag as
    // language identifier when it matches.
    let body = if let Some(nl) = body_with_tag.find('\n') {
        let first_line = body_with_tag[..nl].trim().to_lowercase();
        if first_line == "rust" || first_line == "rs" || first_line.is_empty() {
            &body_with_tag[nl + 1..]
        } else {
            body_with_tag
        }
    } else if let Some(sp) = body_with_tag.find(char::is_whitespace) {
        // Single-line block with inline language tag.
        let first_token = body_with_tag[..sp].trim().to_lowercase();
        if first_token == "rust" || first_token == "rs" {
            &body_with_tag[sp + 1..]
        } else {
            body_with_tag
        }
    } else {
        body_with_tag
    };
    let trimmed = body.trim();
    if trimmed.len() < 3 {
        return None;
    }
    let looks_like_rust = trimmed.contains("fn ")
        || trimmed.contains("let ")
        || trimmed.contains("use ")
        || trimmed.contains("struct ")
        || trimmed.contains("enum ")
        || trimmed.contains("impl ")
        || trimmed.contains("trait ");
    if !looks_like_rust {
        return None;
    }
    // **v5.6.6 — Codex follow-up review.** Filter pedagogical_topic_
    // hint to canonical curriculum-stage IDs only when used as
    // SubmitSolution.topic. Pre-v5.6.6 a clean snippet like
    // ` ```rust\nprintln!("hello")\n``` ` made `pedagogical_topic_
    // hint` return `Some("println")` (the snippet contained the Rust
    // syntax token «println» which is in `LATIN_TECH_SUBJECTS`),
    // which the planner then surfaced as «println тапсырмаңыз
    // шешілді» — treating an implementation detail as a lesson topic.
    // The fix passes through only canonical curriculum stages
    // (ownership / borrow / lifetime / traits / async); other
    // Latin-token hits are dropped so the planner falls back to
    // `session.last_exercise_topic` (set by the prior AskExercise
    // turn) per the v4.95.5 multi-turn lesson-state path.
    const CURRICULUM_STAGES: &[&str] = &["ownership", "borrow", "lifetime", "traits", "async"];
    let topic = pedagogical_topic_hint(input).filter(|t| CURRICULUM_STAGES.contains(&t.as_str()));
    Some((trimmed.to_string(), topic))
}

/// **v4.93.5** — pedagogical-intent topic extractor. Scans the
/// input for a Latin tech subject (`ownership`, `tokio`, ...) or
/// the most-recent multiword Rust subject. Returns the topic in
/// lowercase canonical form, or None if no topic is recoverable
/// (the planner falls back to a generic prompt in that case).
fn pedagogical_topic_hint(input: &str) -> Option<String> {
    // **v5.2.5** — Codex round-3 audit Bug 3. Kazakh-first tutor UX.
    // The 5 canonical curriculum stages have natural Kazakh names;
    // canonicalise to the stage id BEFORE the generic latin /
    // multiword hints fire (otherwise multiword_entity_hint catches
    // «қарыз алу» as the literal phrase before we get a chance to
    // map it to "borrow"). Order: kazakh_aliases > latin > multiword.
    let lower = input.to_lowercase();
    let kazakh_aliases: &[(&str, &str)] = &[
        ("иелік", "ownership"),
        ("қарызға алу", "borrow"),
        ("қарыз алу", "borrow"),
        ("қарыз", "borrow"),
        ("өмір сүру кезеңі", "lifetime"),
        ("өмір кезеңі", "lifetime"),
        ("қасиеттер", "traits"),
        ("қасиет", "traits"),
        ("асинхронды", "async"),
        ("асинхрон", "async"),
    ];
    for (alias, canonical) in kazakh_aliases {
        if lower.contains(alias) {
            return Some(canonical.to_string());
        }
    }
    if let Some(latin) = crate::topic_extraction::latin_subject_hint(input) {
        return Some(latin);
    }
    if let Some(mw) = crate::topic_extraction::multiword_entity_hint(input) {
        return Some(mw);
    }
    None
}

fn detect_ask_willingness(joined: &str) -> bool {
    let has_readiness = joined.contains("дайынсыз ба")
        || joined.contains("дайынсың ба")
        || joined.contains("ашықсыз ба")
        || joined.contains("ашықсың ба");
    let has_growth_verb = joined.contains("үйренуге")
        || joined.contains("жақсаруға")
        || joined.contains("дамуға")
        || joined.contains("дамытуға")
        || joined.contains("өсуге")
        || joined.contains("жетілуге")
        || joined.contains("жетілдіруге")
        // Composite forms like «жақсырақ болу + ға»: «жақсырақ
        // болуға дайынсыз ба» — the user is asking whether you're
        // open to BECOMING better.
        || joined.contains("жақсырақ болуға")
        || joined.contains("ақылды болуға");
    has_readiness && has_growth_verb
}

/// **v4.17.5** — list-request with anaphoric subject. Pattern:
/// `оларды/соларды/мұны + тізімде/атап шық/атаулары + (optional)
/// аласыз ба`. The user is asking adam to enumerate something
/// from the previous turn; routing to GenericCapability would
/// dismiss the request as "I can't do that". Instead detect the
/// pattern early and let the turn fall through to Intent::Unknown
/// so the discourse-anaphora resolver substitutes `оларды → last
/// topic` and SearchGraph surfaces the curated list.
fn is_list_request_with_anaphor(joined: &str) -> bool {
    let has_anaphor = joined.contains("оларды")
        || joined.contains("соларды")
        || joined.contains("оны")
        || joined.contains("соны")
        || joined.contains("мұны")
        || joined.contains("бұны");
    let has_list_verb = joined.contains("тізімде")
        || joined.contains("тізімдей")
        || joined.contains("тізімдеп")
        || joined.contains("тізім жаса")
        || joined.contains("атап шық")
        || joined.contains("атап өт")
        || joined.contains("атаулары");
    has_anaphor && has_list_verb
}

fn detect_ask_about_system(
    tokens: &[String],
    joined: &str,
    raw_input: &str,
) -> Option<crate::system_identity::SystemAspect> {
    use crate::system_identity::SystemAspect;
    let pronoun = tokens.iter().any(|t| t == "сен" || t == "сіз");
    let has_addressee = pronoun || joined.contains("сені") || joined.contains("сізді");

    // **v4.18.5** — composite (identity + capabilities) question.
    // Pre-v4.18.5 detectors picked one aspect per turn. The 2026-
    // 05-01 live REPL turn 4 — «Өзіңіз туралы, кім екеніңіз және
    // не істей алатыныңыз туралы аздап айтып беріңізші» — asks
    // both who you are AND what you can do; adam answered only
    // the first half.
    //
    // Detect the composite pattern BEFORE individual-aspect
    // detectors so the combined template fires:
    // - identity marker: «кім екен» / «өзіңіз туралы» / «өзің
    //   туралы» / «не екен» / `сіз кімсіз`
    // - connector: «және» (or no connector but multiple markers)
    // - capabilities marker: «не істей ала» / «мүмкіндіктері»
    let identity_marker = joined.contains("кім екен")
        || joined.contains("кім ексің")
        || joined.contains("сіз кімсіз")
        || joined.contains("сен кімсің")
        || joined.contains("өзіңіз туралы")
        || joined.contains("өзің туралы")
        || joined.contains("не екен");
    let capabilities_marker_short = joined.contains("не істей ала")
        || joined.contains("мүмкіндіктер")
        || joined.contains("қандай қызмет");
    if identity_marker && capabilities_marker_short && joined.contains("және") {
        return Some(SystemAspect::IntroAndCapabilities);
    }

    // **v4.3.4** — Creator aspect: "who made you" / "who built you".
    // Triggered by addressee-marker (сені/сізді/сен/сіз) plus
    // creator-question keyword. Checked first so multi-question
    // utterances ("сен кімсің және сені кім жасады?") pick this up
    // when the creator part is present — the architecture/birthdate
    // / general checks below all resolve to a less-specific aspect.
    if has_addressee
        && (joined.contains("кім жасады")
            || joined.contains("кім құрды")
            || joined.contains("кім жасап шығарды")
            || joined.contains("кім ойлап тапты")
            || joined.contains("авторың")
            || joined.contains("авторыңыз")
            || joined.contains("жасаушың")
            || joined.contains("жасаушыңыз")
            || joined.contains("кім құрастырды")
            // **v4.6.1** — additional creator-question verb forms
            // surfaced by a 2026-04-29 real-REPL transcript:
            //   «Ал сені кім жаратты?»     (created)
            //   «Сізді кім дамытқан?»      (developed)
            //   «Сізді қай бағдарламашы дайындады?» (which programmer prepared)
            // The first two key off the verb directly; the third
            // also covers the «бағдарламашы» (programmer) framing
            // since that's a creator-role question even without
            // a `кім жасады`-style verb.
            || joined.contains("кім жаратты")
            || joined.contains("кім дамытқан")
            || joined.contains("кім дамытты")
            || joined.contains("кім дайындады")
            || joined.contains("жаратушың")
            || joined.contains("жаратушыңыз")
            || joined.contains("қай бағдарламашы")
            || joined.contains("қандай бағдарламашы")
            || joined.contains("бағдарламашы дайындады")
            || joined.contains("бағдарламашы жасады")
            // **v4.17.5** — «кім тәрбиеледі» (who raised/educated
            // you) and related verbs surfaced by the 2026-05-01
            // live REPL transcript. Pre-v4.17.5 «А, сені кім
            // тәрбиеледі?» fell through to greedy retrieval and
            // surfaced `Бәлкім, тәрбиеле туралы айтасыз ба` —
            // unparseable verb stem treated as topic noun. Adam
            // wasn't raised; it was created. Route to Creator so
            // the canonical creator answer fires.
            || joined.contains("кім тәрбиеледі")
            || joined.contains("кім баптады")
            || joined.contains("кім үйретті")
            || joined.contains("кім бапкер")
            || joined.contains("тәрбиешің")
            || joined.contains("тәрбиешіңіз"))
    {
        return Some(SystemAspect::Creator);
    }

    // **v4.3.4** — Birthdate aspect: "when were you born" / "when
    // did you appear". Pronoun gate optional because phrasings like
    // `туған күнің қашан` carry the addressee in the possessive
    // suffix.
    if joined.contains("қашан пайда болдың")
        || joined.contains("қашан пайда болдыңыз")
        || joined.contains("қашан жасалдың")
        || joined.contains("қашан жасалдыңыз")
        || joined.contains("қашан туылдың")
        || joined.contains("қашан туылдыңыз")
        || joined.contains("туған күнің")
        || joined.contains("туған күніңіз")
        // **v4.17.5** — «дүниеге кел» fixed expression for "to be
        // born / to come into being". 2026-05-01 live REPL:
        // «Ал сіз алғаш қашан дүниеге келдіңіз?» pre-v4.17.5 fell
        // through to greedy retrieval and surfaced a poetry quote
        // about `дүние` (world). Adam's `birthdate` is 2026-04-07;
        // route there.
        || joined.contains("дүниеге келдің")
        || joined.contains("дүниеге келдіңіз")
        || joined.contains("дүниеге келген")
        || (has_addressee && joined.contains("дүниеге кел"))
        || (has_addressee
            && (joined.contains("қашан жасады")
                || joined.contains("қашан құрды")
                || joined.contains("қашан жасап шығарды")
                // **v4.6.12** — additional creation-verb forms
                // mirroring the v4.6.5 Creator-aspect extension.
                // Real-REPL 2026-04-29: «Ал ол сені қашан
                // жаратты?» fell through pre-v4.6.12. Same
                // surface-level reasoning: `жаратты / дамытты /
                // дамытқан / дайындады` are common Kazakh
                // creation verbs that should pair with `қашан`
                // for the Birthdate aspect.
                || joined.contains("қашан жаратты")
                || joined.contains("қашан дамытты")
                || joined.contains("қашан дамытқан")
                || joined.contains("қашан дайындады")))
    {
        return Some(SystemAspect::Birthdate);
    }

    // **v4.3.4** — Architecture aspect: "how are you different" /
    // "what's special about you". Pronoun-led; the question targets
    // the system's distinguishing characteristics.
    if has_addressee
        && (joined.contains("ерекшелігің")
            || joined.contains("ерекшелігіңіз")
            || joined.contains("айырмашылығың")
            || joined.contains("айырмашылығыңыз")
            || joined.contains("неге басқашасың")
            || joined.contains("неге басқашасыз")
            || joined.contains("неге басқа модельдерден ерекшеленесің")
            || joined.contains("неге басқа модельдерден ерекшеленесіз")
            || joined.contains("қалай ерекшеленесің")
            || joined.contains("қалай ерекшеленесіз"))
    {
        return Some(SystemAspect::Architecture);
    }

    // **v4.6.0** — Capabilities aspect: "what can you do?".
    // Pronoun-led OR `сені/сізді`-marked OR a 2sg/2pl ability-modal
    // copula (`істей аласың / істей аласыз`). The verb form
    // `аласың / аласыз` is itself the addressee marker even without
    // a free-standing pronoun (the morpheme `-сың/-сыз` is 2nd
    // person), so the pronoun gate is loosened here. Real-REPL
    // 2026-04-29 transcript: «Не істей аласың?» (no leading pronoun).
    // **v4.12.0** — Implementation aspect. Surface forms:
    // «сіз қандай тілде жазылғансыз?», «не тілінде жасалғансың?»,
    // «қандай бағдарламалау тілінде жазылған?», «архитектурада не
    // тілі қолданылған?». Distinct from `Architecture` ("how are
    // you different?") and `SelfComparison` ("trade-off vs other
    // models"): this asks the literal "what programming language /
    // stack are you written with?". Closes the v4.11.7 known gap.
    // Runs BEFORE Capabilities (which has overlap on `жазылған`-
    // suffix forms).
    let implementation_marker = joined.contains("қандай тілде жазылған")
        || joined.contains("қандай тілде жазылғансың")
        || joined.contains("қандай тілде жазылғансыз")
        || joined.contains("не тілінде жазылған")
        || joined.contains("не тілінде жасалғансың")
        || joined.contains("не тілінде жасалғансыз")
        || joined.contains("қандай бағдарламалау тілінде жазылған")
        || joined.contains("қандай бағдарламалау тілінде жазылғансың")
        || joined.contains("қандай бағдарламалау тілінде жазылғансыз")
        || joined.contains("кодыңыз қай тілде")
        || joined.contains("коды қай тілде")
        || joined.contains("қандай тілде жасалғансыз")
        || joined.contains("қандай тілде жасалғансың");
    if implementation_marker {
        return Some(SystemAspect::Implementation);
    }

    let capabilities_marker = joined.contains("істей аласың")
        || joined.contains("істей аласыз")
        || joined.contains("қолыңнан не келеді")
        || joined.contains("қолыңыздан не келеді")
        || joined.contains("мүмкіндіктерің")
        || joined.contains("мүмкіндіктеріңіз")
        || (has_addressee
            && (joined.contains("не істей аласың")
                || joined.contains("не істей аласыз")
                || joined.contains("қандай мүмкіндіктер")))
        // **v4.11.7** — language-capability questions. Real-REPL
        // 2026-04-30: «Сіз қазақша білесіз бе?» pre-v4.11.7
        // returned "Түсінбедім." because no detector matched the
        // {language}-ша + 2nd-person-knowledge-verb pattern.
        // Routes to Capabilities aspect because
        // `capabilities_summary` already states «Қазақ тілінде
        // сөйлесе аламын». Pattern: closed list of common
        // language adverbs (`қазақша / орысша / ағылшынша /
        // түрікше`) paired with `білесің / білесіз / сөйлей
        // аласың / сөйлей аласыз / түсінесің / түсінесіз`.
        || joined.contains("қазақша білесің")
        || joined.contains("қазақша білесіз")
        || joined.contains("қазақша сөйлей аласың")
        || joined.contains("қазақша сөйлей аласыз")
        || joined.contains("қазақша түсінесің")
        || joined.contains("қазақша түсінесіз")
        || joined.contains("орысша білесің")
        || joined.contains("орысша білесіз")
        || joined.contains("орысша сөйлей аласың")
        || joined.contains("орысша сөйлей аласыз")
        || joined.contains("ағылшынша білесің")
        || joined.contains("ағылшынша білесіз")
        || joined.contains("ағылшынша сөйлей аласың")
        || joined.contains("ағылшынша сөйлей аласыз")
        || joined.contains("түрікше білесің")
        || joined.contains("түрікше білесіз");
    if capabilities_marker {
        return Some(SystemAspect::Capabilities);
    }

    // **v4.6.0** — Limitations aspect: "what CAN'T you do?". Same
    // 2nd-person ability marker but in the negative — `алмайсың /
    // алмайсыз`. Gated on an explicit interrogative marker
    // (question pronouns / particles / question mark) so a
    // declarative criticism «сен ештеңе білмейсің» (= "you know
    // nothing") doesn't accidentally route here — that's user
    // venting, not a query about limitations. The cognitive eval
    // scenario `qysqasy_discourse_particle_does_not_capture_topic`
    // pinned this distinction at v4.4.10 and the gate keeps it.
    let has_interrogative = joined.contains("?")
        || joined.contains("не ")
        || joined.ends_with("не")
        || joined.contains("нені")
        || joined.contains("қандай")
        || joined.contains("қалай")
        || joined.contains(" бе")
        || joined.ends_with("бе")
        || joined.contains(" ма")
        || joined.ends_with("ма");
    let limitations_marker_active = joined.contains("істей алмайсың")
        || joined.contains("істей алмайсыз")
        || joined.contains("шектеулерің")
        || joined.contains("шектеулеріңіз")
        || joined.contains("әлсіз тұстарың")
        || joined.contains("әлсіз тұстарыңыз")
        || joined.contains("несің әлсіз")
        || joined.contains("несіз әлсіз");
    let limitations_marker_passive = joined.contains("білмейсің") || joined.contains("білмейсіз");
    if limitations_marker_active || (limitations_marker_passive && has_interrogative) {
        return Some(SystemAspect::Limitations);
    }

    // **v4.6.0** — Knowledge aspect: "what do you know?". Surface
    // forms: `не білесің / не білесіз`, `қандай тақырыптар / қандай
    // салалар жайлы білесің`. Note: bare `Қазақстан туралы не
    // білесіз` should NOT route here (that's an Unknown topic
    // query about қазақстан). The disambiguator: `не білесің` /
    // `не білесіз` as standalone or with general scope qualifiers
    // (`қандай / жалпы`), but if there's a content noun + туралы
    // before it, fall through to the Unknown path.
    let knowledge_general_marker = (joined.contains("не білесің") || joined.contains("не білесіз"))
        && !joined.contains("туралы")
        && !joined.contains("жайында")
        && !joined.contains("жөнінде");
    let knowledge_explicit_marker = joined.contains("қандай салаларды білесің")
        || joined.contains("қандай салаларды білесіз")
        || joined.contains("қандай тақырыптар")
        || joined.contains("қандай тақырыптарды")
        || joined.contains("білімің не")
        || joined.contains("білімің қандай")
        || joined.contains("білімініз қандай");
    if knowledge_general_marker || knowledge_explicit_marker {
        return Some(SystemAspect::Knowledge);
    }

    // **v4.6.5** — Principles aspect: "what are your principles?".
    // Surface forms: `принциптерің / ұстанымдарың / заңдарың /
    // ережелерің / құндылықтарың` plus an interrogative qualifier
    // (`не`, `қандай`). The articulation layer — adam states the
    // values it upholds (respect humans, no fabrication, no
    // incitement, privacy, no illegal assistance, audit trail,
    // Kazakh-cultural respect, scope discipline). The underlying
    // guarantees are already safe-by-construction; this aspect
    // makes the value contract discoverable.
    let principles_marker = joined.contains("принциптерің")
        || joined.contains("принциптеріңіз")
        || joined.contains("ұстанымдарың")
        || joined.contains("ұстанымдарыңыз")
        || joined.contains("заңдарың")
        || joined.contains("заңдарыңыз")
        || joined.contains("ережелерің")
        || joined.contains("ережелеріңіз")
        || joined.contains("құндылықтарың")
        || joined.contains("құндылықтарыңыз");
    if principles_marker {
        return Some(SystemAspect::Principles);
    }

    // **v4.6.20** — SelfComparison aspect: "how are you
    // better/different from other AI models?". Real-REPL
    // 2026-04-29 transcript surfaced two phrasings —
    // «Басқа жасанды интеллект модельдерінен несімен артықсыз?»
    // and «… қалай жақсырақ бола аласыз?». Pre-v4.6.20 these fell
    // through to the greedy noun-hint path which grabbed
    // `басқа` / `қолданыс` and quoted random corpus material.
    // The detector lives in `discourse.rs` as a pair-signal
    // (comparison marker + addressee anchor); routing here
    // makes the planner pick the dedicated `self_comparison`
    // family that articulates the trade-off honestly.
    if crate::discourse::input_is_self_comparison_question(joined) {
        return Some(SystemAspect::SelfComparison);
    }

    // **v4.13.5** — Multi-topic capability marker. Pattern: 3+
    // comma-separated content nouns + `және` (Kazakh "and") + a
    // capability verb (`білесің / білесіз`). 2026-05-01 live REPL:
    // «Сіз математика, физика, химия, биология, астрономия және
    // тағы басқа пәндер бойынша мектептегі біліміңізді білесіз бе?»
    // pre-v4.13.5 grabbed `мектеп` as topic and surfaced an IsA
    // fact. The honest answer is: adam has surface-level
    // understanding of these subjects (knowledge_summary covers
    // them at the domain level) but no school-curriculum-level
    // depth. Route to the dedicated `multi_topic_capability`
    // template family. Detection requires:
    //   - 2+ commas (signals a list of nouns)
    //   - `және` (the canonical "and" connector)
    //   - one of the capability verbs (`білесің / білесіз` —
    //     other capability verbs like `сөйлей аласыз` are too
    //     specific to be safe to fire on a random list)
    // **Important**: `joined` strips punctuation; commas are gone by
    // the time we reach this detector. Use `raw_input` (lower-cased
    // for case-insensitive matching) for the comma-count, but
    // `joined` for the textual markers since the rest of this
    // function is `joined`-based.
    let raw_lower = raw_input.to_lowercase();
    let comma_count = raw_lower.matches(',').count();
    let multi_topic_capability = comma_count >= 2
        && raw_lower.contains("және")
        && (joined.contains("білесің") || joined.contains("білесіз"));
    if multi_topic_capability {
        return Some(SystemAspect::MultiTopicCapability);
    }

    // **v4.13.5** — Generic verb-capability marker. Pattern:
    // `<verb-converb> ала<person-suffix> <ма/ба/па>?` for any verb
    // OTHER than the language-capability ones already handled
    // above. 2026-05-01 live REPL: «Сіз оны бағдарламалай аласыз
    // ба, әлі жоқ па?» — pre-v4.13.5 the language-capabilities
    // detector required a {language adverb} prefix, so this fell
    // through to greedy retrieval and surfaced poetry. v4.13.5
    // catches the broader pattern: ANY converb + `ала+person` +
    // question particle = capability question on the verb. The
    // honest answer: «Жоқ, мен ондай әрекетті орындай алмаймын —
    // мен тілдік модельмін» (preserves the v4.6.0 trust contract
    // that adam doesn't pretend to do things it can't).
    //
    // Surface forms that signal the "ала + person" auxiliary:
    //   аласың / аласыз / алады / ала ма / ала ме
    // Combined with a question particle (бе/ма/ба/па + alternants)
    // OR a question mark.
    let aux_capability = joined.contains("аласың ба")
        || joined.contains("аласыз ба")
        || joined.contains("аласың ма")
        || joined.contains("аласыз ма")
        || joined.contains("ала ма?")
        || joined.contains("ала ме?")
        || joined.contains("алады ма")
        || joined.contains("алады ме");
    if aux_capability {
        // **v4.17.5** — list-request gate. Live REPL 2026-05-01:
        // «Оларды тізімдей аласыз ба?» (after the previous turn
        // mentioned 17 regions) pre-v4.17.5 surfaced the
        // GenericCapability honest fallback. The user wasn't
        // asking adam to perform an action — they were asking it
        // to enumerate the previous topic. Detect list-verb +
        // anaphor and DON'T route to GenericCapability; let the
        // turn fall through to Intent::Unknown so the v4.13.0
        // discourse-anaphora resolver can replace the anaphor
        // with the last topic, then SearchGraph surfaces the
        // curated list-summary fact.
        let is_list_anaphor = is_list_request_with_anaphor(joined);
        // **v4.41.7** — explain/teach gate. Live REPL 2026-05-03:
        // «Маған Rust бағдарламалауын үйрете аласыз ба?» /
        // «Оның қалай жұмыс істейтінін түсіндіріп бере аласыз
        // ба?» pre-v4.41.7 routed to GenericCapability ("can't do
        // physical action"), missing that "explain / teach via
        // facts" IS adam's main capability. The verbs «түсіндір»
        // / «үйрет» / «айтып бер» are about *describing concepts*,
        // not executing actions — they should fall through to
        // topic-extraction so SearchGraph surfaces the relevant
        // facts about the topic.
        let is_explain_teach = joined.contains("түсіндір")
            || joined.contains("үйрет")
            || joined.contains("айтып бер")
            || joined.contains("баянда");
        if !is_list_anaphor && !is_explain_teach {
            return Some(SystemAspect::GenericCapability);
        }
    }

    // **v4.17.5** — combined "how-to-do-X knowledge" question.
    // Pattern: `<verb-action> керектігін білесіз бе`. Live REPL
    // 2026-05-01: «Rust бағдарламалау тілінде қалай бағдарламалау
    // керектігін білесіз бе?» pre-v4.17.5 matched the language-
    // capability detector via «білесіз бе» but lacked a leading
    // language adverb (қазақша / орысша / ...), so it fell through
    // to greedy retrieval and surfaced the IsA fact about Rust.
    // The user's actual question is a capability check: "do you
    // know HOW to program in X?". Route to GenericCapability so
    // the honest «Жоқ, бағдарлама жаза алмаймын» fallback fires.
    let how_to_capability = (joined.contains("қалай")
        && (joined.contains("керектігін білесің") || joined.contains("керектігін білесіз")))
        || joined.contains("істеуді білесің")
        || joined.contains("істеуді білесіз")
        || joined.contains("жасауды білесің")
        || joined.contains("жасауды білесіз");
    if how_to_capability {
        return Some(SystemAspect::GenericCapability);
    }

    // **v4.3.3** — General aspect: pronoun-led identity question.
    // **v4.6.0** — Also fires on `өзіңіз туралы айт` style requests
    // (compound self-introduction openers from a 2026-04-29 real-
    // REPL transcript: «Өзіңіз туралы айтып беріңізші, …»). The
    // marker `өзің / өзіңіз` + `туралы` + speech-act verb (айт /
    // айтып бер / таныстыр) is unambiguous self-reference.
    let self_intro_request = (joined.contains("өзің туралы")
        || joined.contains("өзіңіз туралы")
        || joined.contains("өзің жайлы")
        || joined.contains("өзіңіз жайлы"))
        && (joined.contains("айт")
            || joined.contains("таныс")
            || joined.contains("берші")
            || joined.contains("беріңіз"));
    // **v4.6.20** — reflexive identity questions where the user
    // asks adam to *describe itself* in 2nd-person reflexive form:
    // «Өзіңді кім деп санайсың?» / «Өзіңізді кім деп санайсыз?» /
    // «Өзіңді қалай таныстырасың?» / «Өзіңізді қалай көресіз?». The
    // marker is `өзіңді / өзіңізді` (reflexive accusative) plus a
    // 2nd-person verb. Real-REPL 2026-04-29 fell through to a
    // misclassification ("user wants to talk about themselves").
    let reflexive_self_question = (joined.contains("өзіңді") || joined.contains("өзіңізді"))
        && (joined.contains("санайсың")
            || joined.contains("санайсыз")
            || joined.contains("таныстырасың")
            || joined.contains("таныстырасыз")
            || joined.contains("көресің")
            || joined.contains("көресіз")
            || joined.contains("қалай атайсың")
            || joined.contains("қалай атайсыз")
            || joined.contains("кім дейсің")
            || joined.contains("кім дейсіз"));
    // **v5.4.6** — 2nd-person name question: «Сіздің атыңыз кім?» /
    // «Сенің атың не?» / «Сіздің есіміңіз қандай?». Pre-v5.4.6 these
    // routed to `Intent::AskName` and surfaced the *user's* stored
    // name ("Дәулет деп танысқан едіңіз") instead of adam's. The bug
    // was a 1st/2nd-person polysemy in detect_ask_name. The fix has
    // two halves: detect_ask_name now bails when the 2nd-person
    // possessive «сіздің / сенің» appears without 1st-person «менің»;
    // here we route the same pattern to `SystemAspect::General` so
    // the canonical adam-introduction template fires.
    let asks_system_name = (joined.contains("сіздің") || joined.contains("сенің"))
        && (joined.contains("атыңыз")
            || joined.contains("атың")
            || joined.contains("есіміңіз")
            || joined.contains("есімің"))
        && (joined.contains("кім") || joined.contains("не") || joined.contains("қандай"))
        && !joined.contains("менің");
    // **v5.4.6** — 2nd-person speaking-language question:
    // «Сіз қандай тілде сөйлейсіз?» / «Сен қай тілде сөйлейсің?».
    // Pre-v5.4.6 these routed to Definition over the noun «тіл» and
    // surfaced «Тіл — сөйлеу мүшесі.» (anatomical organ definition).
    // The speaking-language question is a system-self property —
    // adam speaks Kazakh only — so route to General. Distinct from
    // Implementation (programming language) which uses
    // «жазылғансыз» / «жасалғансыз» verb stems.
    let asks_speaking_language = pronoun
        && (joined.contains("сөйлейсің")
            || joined.contains("сөйлейсіз")
            || joined.contains("білесіз")
            || joined.contains("білесің"))
        && (joined.contains("қандай тілде")
            || joined.contains("қай тілде")
            || joined.contains("қандай тілдерде")
            || joined.contains("қай тілдерде")
            || joined.contains("қандай тіл")
            || joined.contains("қай тіл"));
    if pronoun
        && (joined.contains("кімсің")
            || joined.contains("кімсіз")
            || joined.contains("қандай моделсің")
            || joined.contains("қандай моделсіз")
            || joined.contains("қандай ботсың")
            || joined.contains("қандай ботсыз")
            || joined.contains("қандай жасанды интеллектсің")
            || joined.contains("қандай жасанды интеллектсіз")
            || joined.contains("немен айналысасың")
            || joined.contains("немен айналысасыз"))
        || self_intro_request
        || reflexive_self_question
        || asks_system_name
        || asks_speaking_language
    {
        return Some(SystemAspect::General);
    }

    None
}

fn detect_ask_name(joined: &str) -> bool {
    // **v5.4.6** — 2nd-person possessive disambiguation. When the
    // input has «сіздің» / «сенің» (2nd-person possessive) WITHOUT
    // «менің» (1st-person), the name question is about ADAM, not
    // the user. Bail so `detect_ask_about_system` picks it up via
    // `SystemAspect::General` and the canonical adam-introduction
    // template fires instead of surfacing the user's stored name.
    let asks_system_name =
        (joined.contains("сіздің") || joined.contains("сенің")) && !joined.contains("менің");
    if asks_system_name {
        return false;
    }
    (joined.contains("атың") && joined.contains("кім"))
        || (joined.contains("атыңыз") && joined.contains("кім"))
        || joined.contains("есімің")
        || joined.contains("есіміңіз")
        // v4.4.9 — 1sg self-recall form: "менің атым кім?",
        // "есімім кім / не / қандай?". Mirrors the v4.4.5 fix to
        // `detect_ask_age` for `менің жасым қанша?` and the v4.4.6
        // fix to `detect_ask_occupation` for `менің мамандығым не?`.
        // Pre-v4.4.9 the 1sg-possessive `атым` matched
        // `detect_statement_of_name`'s pattern 1 ("атым X") and
        // grabbed the literal `кім` as the user's name, then logged
        // a phantom contradiction (Дәулет vs Кім) the next time the
        // session already had a real name. The asymmetric guard in
        // `detect_statement_of_name` (refuses interrogative
        // pronouns as names) is the actual bug fix; this extension
        // routes the 1sg-self-recall question to `Intent::AskName`
        // so it answers from session storage via
        // `ask_name.with_known_user`.
        || (joined.contains("атым")
            && (joined.contains("кім") || joined.contains("не")))
        || (joined.contains("есімім")
            && (joined.contains("кім") || joined.contains("не")))
        // **v4.54.5** — recall-question variants:
        // «менің атымды есіңізде ме?» / «атымды ұмытпадыңыз ба?» /
        // «есімімді есіңізде ме?». The Acc form «атымды»/«есімімді»
        // (1sg-poss + Acc) co-occurring with a memory-probe phrase
        // («есіңізде ме» = "do you remember", «ұмытпадың» = "haven't
        // forgotten") is unambiguous self-recall — pre-v4.54.5
        // adam routed «менің атымды есіңізде ме» to Unknown with
        // noun_hint=ат and surfaced a grounded fact about «ат» (horse).
        // **session 6 transcript fix.**
        // Also covers compound «аты-жөн» (name + family-name)
        // surfaced by autotest B post-v4.54.5.
        || ((joined.contains("атым")
            || joined.contains("атымды")
            || joined.contains("аты-жөнім")
            || joined.contains("аты-жөнімді"))
            && (joined.contains("есіңізде")
                || joined.contains("есіңде")
                || joined.contains("ұмытпа")
                // **v4.93.0** — Codex 2026-05-07 audit: extend
                // recall-question markers to forgetting-verbs and
                // knowledge-verbs in their full inflected forms.
                // «Атымды ұмыттыңыз ба?» / «Сен менің атымды
                // білесіз бе?» / «Атымды естідіңіз бе?» pre-v4.93.0
                // fell through to topic extraction on `ат` (horse).
                || joined.contains("ұмытты")
                || joined.contains("ұмытқа")
                || joined.contains("ұмыттың")
                || joined.contains("білесіз")
                || joined.contains("білесің")
                || joined.contains("білесіздер")
                || joined.contains("білдіңіз")
                || joined.contains("білдің")
                || joined.contains("естіді")
                || joined.contains("естіді")))
        || ((joined.contains("есімім") || joined.contains("есімімді"))
            && (joined.contains("есіңізде")
                || joined.contains("есіңде")
                || joined.contains("ұмытпа")
                || joined.contains("ұмытты")
                || joined.contains("білесіз")
                || joined.contains("білесің")
                || joined.contains("білдіңіз")
                || joined.contains("білдің")))
}

fn detect_statement_of_wellbeing(tokens: &[String], joined: &str) -> bool {
    let wellbeing_token = tokens.iter().any(|t| {
        matches!(
            t.as_str(),
            "жақсымын" | "жаманмын" | "жақсы" | "жаман" | "дұрысмын"
        )
    });
    wellbeing_token || joined.contains("жаман емес")
}

// --- v0.8.0 new recognisers ------------------------------------------------

/// User introduces self. Kazakh patterns only (v1.1.0):
///   1. [менің] атым <NAME>
///   2. мені <NAME> деп атайды
///   3. есімім <NAME>
///
/// Returns the extracted name (case-preserved, first-letter title-
/// cased) or `None` when none of the three patterns fire.
fn detect_statement_of_name(
    tokens: &[String],
    raw_tokens: &[String],
    joined: &str,
) -> Option<String> {
    // **v4.93.0** — Codex 2026-05-07 audit: memory poisoning fix.
    // «Менің атым есіңізде ме?» (= "do you remember my name?") was
    // misclassified as `Intent::StatementOfName { name: "Есіңізде" }`
    // because pattern 1 below ("атым X") grabbed the verb-phrase
    // `есіңізде` as the literal name. The interrogative-pronoun
    // guard (v4.4.9) only blocked question-WORDS (кім / не / қандай);
    // it didn't block question-PARTICLES (ме / бе / ма / ба / па / пе)
    // or memory-probe verbal forms (есіңізде / есіңде / ұмытпа /
    // білесіз / ұмыттыңыз). Defer to `detect_ask_name` first —
    // single source of truth for "this looks like a name question".
    if detect_ask_name(joined) {
        return None;
    }

    // v4.4.9 — interrogative-pronoun guard. The 1sg-possessive
    // forms `атым` / `есімім` collide with the user asking about
    // their OWN stored name: `менің атым кім?` lexes as
    // `[менің, атым, кім, ?]`, pattern 1 below ("атым X" → name
    // is the next token) would grab the literal `Кім` as a name
    // and — once a real name was already stored — log a phantom
    // `BeliefConflict` (Дәулет vs Кім) followed by a clarifying
    // question that asked the user to pick between their actual
    // name and the question word. Refuse the match when the
    // candidate is an interrogative pronoun. Mirror of v4.4.5
    // `detect_statement_of_age` question-particle guard.
    let is_interrogative_pronoun = |t: &str| {
        let lower = t.to_lowercase();
        matches!(
            lower.as_str(),
            "кім" | "кiм" | "не" | "қандай" | "қайсысы" | "ким"
        )
    };

    // Pattern 1: "атым X".
    if let Some(i) = tokens.iter().position(|t| t == "атым") {
        if let Some(name) = raw_tokens.get(i + 1) {
            if is_interrogative_pronoun(name) {
                return None;
            }
            return Some(normalize_proper_noun(name));
        }
    }
    // Pattern 3: "есімім X".
    if let Some(i) = tokens.iter().position(|t| t == "есімім") {
        if let Some(name) = raw_tokens.get(i + 1) {
            if is_interrogative_pronoun(name) {
                return None;
            }
            return Some(normalize_proper_noun(name));
        }
    }
    // Pattern 2: "мені X деп атайды".
    if joined.contains("деп атайды") {
        if let (Some(start), Some(end)) = (
            tokens.iter().position(|t| t == "мені"),
            tokens.iter().position(|t| t == "деп"),
        ) {
            if end > start + 1 {
                if let Some(name) = raw_tokens.get(start + 1) {
                    if is_interrogative_pronoun(name) {
                        return None;
                    }
                    return Some(normalize_proper_noun(name));
                }
            }
        }
    }
    None
}

fn detect_ask_age(joined: &str) -> bool {
    let has_q = joined.contains("неше") || joined.contains("қанша");
    ((joined.contains("жасың") || joined.contains("жасыңыз")) && has_q)
        || joined.contains("қанша жастасың")
        || joined.contains("қанша жастасыз")
        // **v4.6.12** — `неше` (alongside `қанша`) variant of the
        // adessive-copula age question. Real-REPL 2026-04-29:
        // «Сіз неше жастасыз?» (= "how many years old are you,
        // polite") fell through to topic-extraction on `неше`
        // pre-v4.6.12, surfaced a tangential proverb. Adding the
        // `неше жастасың / неше жастасыз` patterns. Also catches
        // adam-self age questions: with no session.age, the path
        // falls through to the `ask_age` family («менің жасым
        // адамзат жасындай», «мен әлі жаспын») which is the
        // correct response for system-self age inquiries.
        || joined.contains("неше жастасың")
        || joined.contains("неше жастасыз")
        // **v4.11.7** — verb-form variants of the age question.
        // `жасайсың / жасайсыз` (= 2nd-person of `жасау` "to live")
        // is colloquial Kazakh for "how old are you?" alongside the
        // adessive `жастасың / жастасыз`. Live REPL 2026-04-30:
        // «Қанша жасайсыз?» pre-v4.11.7 returned "Түсінбедім."
        // because the existing `жастасың/жастасыз` patterns required
        // the adessive form. Adds the four `қанша/неше + жасайсың/
        // жасайсыз` permutations; pairs with v4.6.12's adessive set.
        || joined.contains("қанша жасайсың")
        || joined.contains("қанша жасайсыз")
        || joined.contains("неше жасайсың")
        || joined.contains("неше жасайсыз")
        // v4.4.5 — 1sg self-recall form: "менің жасым қанша?" /
        // "жасым неше?". Pre-v4.4.5 this matched
        // `detect_statement_of_age` (keyed on `жасым`) and emitted
        // a confirmation template that interpolated session.age
        // — coincidentally correct for the recorded value but
        // wrong for any phrasing where the template doesn't echo
        // the slot, and semantically misclassifying the user's
        // question as a statement.
        || (joined.contains("жасым") && has_q)
}

/// User reports age: "менің жасым N", "N жастамын", "N жасында".
/// Returns `Some(Some(n))` when the pattern matched AND a Kazakh
/// numeral 1–99 was parsed, `Some(None)` when the pattern matched but
/// no numeral was found, and `None` when the pattern didn't match at
/// all (so the caller can continue dispatching).
fn detect_statement_of_age(tokens: &[String], joined: &str) -> Option<Option<u32>> {
    let matched = joined.contains("жасым")
        || tokens
            .iter()
            .any(|t| t == "жастамын" || t == "жастаймын" || t == "жаспын");
    if !matched {
        return None;
    }
    // v4.4.5 — guard: a question particle (`қанша`/`неше`) flips
    // the polarity from statement → question. `detect_ask_age`
    // runs first now, but this defends against any future caller
    // ordering and keeps the matcher honest in isolation.
    if joined.contains("қанша") || joined.contains("неше") {
        return None;
    }
    Some(parse_kazakh_age(tokens))
}

/// Parse a Kazakh numeral in the range 1–99 out of a token stream.
/// Supports compound forms ("отыз бес" = 35) and bare tens/units, as
/// well as literal digit strings ("30"). Returns the first hit.
fn parse_kazakh_age(tokens: &[String]) -> Option<u32> {
    for (i, t) in tokens.iter().enumerate() {
        // Literal digit form, e.g. "30".
        if let Ok(n) = t.parse::<u32>() {
            if (1..200).contains(&n) {
                return Some(n);
            }
        }
        // Tens word, maybe followed by a unit word.
        if let Some(tens) = kazakh_tens_value(t) {
            if let Some(next) = tokens.get(i + 1) {
                if let Some(units) = kazakh_units_value(next) {
                    return Some(tens + units);
                }
            }
            return Some(tens);
        }
        // Bare unit word (rare for ages but handle it).
        if let Some(units) = kazakh_units_value(t) {
            return Some(units);
        }
    }
    None
}

fn kazakh_tens_value(token: &str) -> Option<u32> {
    match token {
        "он" => Some(10),
        "жиырма" => Some(20),
        "отыз" => Some(30),
        "қырық" => Some(40),
        "елу" => Some(50),
        "алпыс" => Some(60),
        "жетпіс" => Some(70),
        "сексен" => Some(80),
        "тоқсан" => Some(90),
        _ => None,
    }
}

fn kazakh_units_value(token: &str) -> Option<u32> {
    match token {
        "бір" => Some(1),
        "екі" => Some(2),
        "үш" => Some(3),
        "төрт" => Some(4),
        "бес" => Some(5),
        "алты" => Some(6),
        "жеті" => Some(7),
        "сегіз" => Some(8),
        "тоғыз" => Some(9),
        _ => None,
    }
}

fn detect_ask_location(joined: &str) -> bool {
    // **v4.73.5** — Codex 2026-05-06 review: «Қазақстанның астанасы
    // қай қала?» previously hit AskLocation, producing system-
    // identity «Мен Қазақстан елімде» — false claim. AskLocation
    // is for asking the USER (or system) where they live; factual
    // location questions about third-party subjects must stay
    // Unknown so retrieval handles them. Gate on explicit user-
    // addressing markers: 2nd-person pronoun «сіз / сен» as
    // separate token, possessive «сіздің / сенің», or live-verb
    // forms «тұрасыз / тұрасың / қайдансыз / қайдансың».
    let tokens: Vec<&str> = joined.split_whitespace().collect();
    let has_user_marker = tokens
        .iter()
        .any(|t| matches!(*t, "сіз" | "сен" | "сіздің" | "сенің"))
        || joined.contains("тұрасыз")
        || joined.contains("тұрасың")
        || joined.contains("қайдансыз")
        || joined.contains("қайдансың");
    // **v5.3.0** — Codex round-3 audit Bug 2 (self-recall sub-fix).
    // Also accept 1st-person reflexive «Қай қалада тұрамын?» / «Қайдамын?»
    // — user asking adam to recall their own location from session.
    // Without this branch the question fell through to a definitional
    // surface ("Қала — елді мекен") instead of «Сіз Алматыда тұрасыз».
    let has_self_recall_marker = tokens
        .iter()
        .any(|t| matches!(*t, "тұрамын" | "тұрамыз" | "қайдамын" | "қайдамыз"));
    if !has_user_marker && !has_self_recall_marker {
        return false;
    }
    joined.contains("қай жерден")
        || joined.contains("қайдан")
        || joined.contains("қайда тұра")
        || joined.contains("қай қала")
        || joined.contains("қай аудан")
        // self-recall variants: «Қай қалада тұрамын?»
        || (has_self_recall_marker && joined.contains("қай"))
        // **v5.6.5** — bare locative «қайда» self-recall: «Мен қайда
        // тұрамын?». Codex 2026-05-09 review caught this falling
        // through to StatementOfLocation. Conservative — requires
        // both the locative interrogative AND a self-recall verb
        // («тұрамын / тұрамыз»).
        || (has_self_recall_marker
            && tokens.iter().any(|t| *t == "қайда" || *t == "қайдан"))
}

/// User states location: "мен Алматыданмын", "астанада тұрамын",
/// "ауылдан келдім". Returns `Some(city)` when the pattern fires,
/// with `city` being the extracted root — or `None` inside `Some`
/// when the pattern fires without a parseable city token.
///
/// v1.4.0 FST-NER primary path: scan the FST `parses` for a
/// Noun in Ablative or Locative case (possibly with P1Sg predicate
/// stacked). If found, its root is the city. Rule-based string-
/// stripping stays as fallback for words not in the Lexicon.
fn detect_statement_of_location(
    tokens: &[String],
    raw_tokens: &[String],
    joined: &str,
    parses: &[Analysis],
) -> Option<Option<String>> {
    use adam_kernel_fst::morphotactics::Case;

    if !has_first_person_location_context(tokens, joined, parses) {
        return None;
    }

    // **v5.3.0** — Codex round-3 audit Bug 2 (sub-fix). «Қай қалада
    // тұрамын?» is a QUESTION about location («which city do I live
    // in?»), not a statement. Pre-fix this detector matched on
    // «тұрамын» (1sg present "I live") + locative noun «қалада»
    // («in city») and routed to `StatementOfLocation { city: "Қала" }`
    // — polluting the belief state with the noun "city" as if the
    // user had asserted living there. Post-fix question markers
    // («қай», «қашан», «неге», «қандай», «?») bail out so the turn
    // falls through to AskLocation or other detectors.
    //
    // **v5.6.5** — Codex 2026-05-09 review caught «Мен қайда
    // тұрамын?» still falling through (`Қай екен, түсіндім`).
    // Root cause: `joined.contains("қай ")` requires a trailing
    // space, so the locative form «қайда» (single token, no space
    // before its case suffix) didn't match. Adding token-level
    // membership in the question-pronoun closed list catches
    // «қайда / қашан / қалай / неге / қандай / қанша» when they
    // appear standalone, regardless of surrounding whitespace.
    let question_pronouns: &[&str] = &[
        "қайда",
        "қайдан",
        "қашан",
        "қалай",
        "неге",
        "қандай",
        "қанша",
        "неше",
        "неліктен",
    ];
    let has_question_pronoun = tokens
        .iter()
        .any(|t| question_pronouns.contains(&t.as_str()));
    let is_question = joined.contains('?')
        || joined.contains("қай ")
        || joined.contains(" қай ")
        || joined.starts_with("қай ")
        || joined.contains("қандай")
        || joined.contains("қашан")
        || has_question_pronoun;
    if is_question {
        return None;
    }

    // Primary: look for a parsed Noun in Ablative or Locative case.
    // Prefer Ablative (stronger signal for origin: "X-дан+мын") over
    // bare Locative. Also accept Locative if co-occurring with
    // "тұрамын / тұрамыз".
    //
    // **v4.0.1** — Codex v4.0.0 review caught that «Неліктен?»
    // («why?» — interrogative) was parsed by the FST as `Нелік` +
    // Ablative, so this detector returned `StatementOfLocation { city:
    // "Нелік" }` and the REPL replied with «Нелікте тұрасыз ба» («Do
    // you live in Нелік?»). The v3.9.5 `NOT_A_TOPIC` sync only
    // filtered `first_noun_root` / `content_roots` — it never touched
    // this detector. Fix: skip any noun whose root is in `NOT_A_TOPIC`
    // (interrogatives, demonstratives, closed-class function words)
    // at the case-scan step. A legitimate city root is never in
    // `NOT_A_TOPIC`; an interrogative is.
    let mut ablative_root: Option<String> = None;
    let mut locative_root: Option<String> = None;
    for p in parses {
        if let Analysis::Noun { root, features } = p {
            if NOT_A_TOPIC.contains(&root.root.as_str()) {
                continue;
            }
            match features.case {
                Some(Case::Ablative) if ablative_root.is_none() => {
                    ablative_root = Some(normalize_place_name(&root.root));
                }
                Some(Case::Locative) if locative_root.is_none() => {
                    locative_root = Some(normalize_place_name(&root.root));
                }
                _ => {}
            }
        }
    }
    if let Some(c) = ablative_root {
        if generic_place_root(&c) {
            if let Some(name) = recover_named_place_before_generic_location(tokens, raw_tokens) {
                return Some(Some(name));
            }
        }
        return Some(Some(c));
    }
    // v1.8.5: if a Noun stacks Locative + P1Sg ("Алматыдамын" = "I am
    // in Almaty"), that's a location statement on its own — no need for
    // a separate "тұрамын" verb. Without this branch, Locative+P1Sg
    // falls through to `detect_statement_of_occupation` and "Алматы"
    // gets miscategorised as an occupation.
    use adam_kernel_fst::morphotactics::Predicate;
    for p in parses {
        if let Analysis::Noun { root, features } = p {
            if NOT_A_TOPIC.contains(&root.root.as_str()) {
                continue;
            }
            if features.case == Some(Case::Locative) && features.predicate == Some(Predicate::P1Sg)
            {
                return Some(Some(normalize_place_name(&root.root)));
            }
        }
    }
    let live_verb = tokens.iter().any(|t| t == "тұрамын" || t == "тұрамыз");
    if live_verb {
        if let Some(c) = locative_root {
            if generic_place_root(&c) {
                if let Some(name) = recover_named_place_before_generic_location(tokens, raw_tokens)
                {
                    return Some(Some(name));
                }
            }
            return Some(Some(c));
        }
    }

    // Fallback for out-of-lexicon inputs: string-based heuristics.
    detect_statement_of_location_rule_based(tokens, raw_tokens, joined)
}

fn has_first_person_location_context(tokens: &[String], joined: &str, parses: &[Analysis]) -> bool {
    use adam_kernel_fst::morphotactics::{Number, Person, Predicate};

    if tokens.iter().any(|t| t == "мен" || t == "біз") {
        return true;
    }
    if tokens.iter().any(|t| {
        matches!(
            t.as_str(),
            "тұрамын" | "тұрамыз" | "келдім" | "келдік" | "тұрмын" | "тұрмыз"
        )
    }) {
        return true;
    }
    if tokens.iter().any(|t| strip_ablative_copula(t).is_some()) {
        return true;
    }
    if tokens.iter().any(|t| strip_locative_copula(t).is_some()) {
        return true;
    }
    if joined.contains("данмын")
        || joined.contains("денмін")
        || joined.contains("танмын")
        || joined.contains("тенмін")
    {
        return true;
    }
    parses.iter().any(|p| match p {
        Analysis::Noun { features, .. } => {
            matches!(features.predicate, Some(Predicate::P1Sg | Predicate::P1Pl))
        }
        Analysis::Verb { features, .. } => {
            features.person == Some(Person::First)
                && matches!(
                    features.number,
                    None | Some(Number::Singular) | Some(Number::Plural)
                )
        }
    })
}

fn generic_place_root(root: &str) -> bool {
    matches!(
        root.to_lowercase().as_str(),
        "ауыл" | "қала" | "аудан" | "облыс" | "өңір" | "кент" | "ел"
    )
}

fn recover_named_place_before_generic_location(
    tokens: &[String],
    raw_tokens: &[String],
) -> Option<String> {
    for i in 1..tokens.len() {
        if !token_mentions_generic_place(&tokens[i]) {
            continue;
        }
        let prev_raw = raw_tokens.get(i - 1)?;
        let prev_token = tokens.get(i - 1)?;
        if NOT_A_TOPIC.contains(&prev_token.as_str()) || generic_place_root(prev_token) {
            continue;
        }
        if raw_looks_like_named_place(prev_raw) {
            return Some(normalize_place_name(prev_raw));
        }
    }
    None
}

fn recover_named_place_before_origin_marker(
    tokens: &[String],
    raw_tokens: &[String],
) -> Option<String> {
    for i in 1..tokens.len() {
        let marker = tokens[i].as_str();
        if !matches!(
            marker,
            "жақтанмын" | "жақтанбыз" | "маңынанмын" | "маңынанбыз"
        ) {
            continue;
        }
        let prev_raw = raw_tokens.get(i - 1)?;
        let prev_token = tokens.get(i - 1)?;
        if i >= 2 && token_mentions_geo_descriptor(&tokens[i - 1]) {
            let prev_prev_raw = raw_tokens.get(i - 2)?;
            let prev_prev_token = tokens.get(i - 2)?;
            if !NOT_A_TOPIC.contains(&prev_prev_token.as_str())
                && !generic_place_root(prev_prev_token)
                && raw_looks_like_named_place(prev_prev_raw)
            {
                let phrase = format!("{} {}", prev_prev_raw, prev_raw);
                return Some(normalize_place_name(&phrase));
            }
        }
        if NOT_A_TOPIC.contains(&prev_token.as_str()) || generic_place_root(prev_token) {
            continue;
        }
        if raw_looks_like_named_place(prev_raw) {
            return Some(normalize_place_name(prev_raw));
        }
    }
    None
}

/// **v4.3.2 — fix: prefix match, not substring match.**
///
/// Pre-v4.3.2 this used `token.contains(stem)`. The 2-letter stem
/// `ел` (country) is incidentally a substring of common modern
/// Kazakh tokens — `интеллект`, `келдім`, `белгі`, `сенделді`, etc.
/// — and produced a false positive that propagated up through
/// `recover_named_place_before_generic_location`, mis-extracting
/// the *preceding* word as a city. Concrete failure mode (real
/// dialog test): the input
///
///   «Мен жаңа жасанды интеллект моделін әзірлейтін бағдарламашымын»
///
/// matched `token.contains("ел")` on `интеллект`, so the recoverer
/// promoted `жасанды` to a city, the belief layer logged
/// `(USER, city, Жасанды)` against `(USER, city, Атырау)`, the
/// planner went into a permanent `CheckContradiction` for every
/// subsequent topic question, and the dialog became unrecoverable.
///
/// Switching to `starts_with` keeps every real word-formation
/// pattern that mentions a generic place (`қалада`, `ауылдан`,
/// `елде`, `елден`, `өңірде`) and rejects intra-word substring
/// matches. Validated by a regression test that re-runs the exact
/// failing dialog turn.
fn token_mentions_generic_place(token: &str) -> bool {
    ["ауыл", "қала", "аудан", "облыс", "өңір", "кент", "ел"]
        .iter()
        .any(|stem| token.starts_with(stem))
}

/// **v4.3.2 — same fix as `token_mentions_generic_place`** for the
/// wider geo-descriptor set used by
/// `recover_named_place_before_origin_marker`. Same false-positive
/// risk, same prefix-match resolution.
fn token_mentions_geo_descriptor(token: &str) -> bool {
    [
        "ауыл",
        "қала",
        "аудан",
        "облыс",
        "өңір",
        "кент",
        "ел",
        "өзен",
        "көл",
        "теңіз",
        "тау",
    ]
    .iter()
    .any(|stem| token.starts_with(stem))
}

fn raw_looks_like_named_place(token: &str) -> bool {
    looks_like_named_place_candidate(token)
}

/// Pre-v1.4.0 string-based heuristic retained as fallback when the FST
/// can't parse the form (e.g. the city isn't in the Lexicon yet).
fn detect_statement_of_location_rule_based(
    tokens: &[String],
    raw_tokens: &[String],
    joined: &str,
) -> Option<Option<String>> {
    // 1st-person "live" verb: `X-да/-де тұрамын` — the city is the
    // token ending in locative that precedes the verb.
    if let Some(verb_idx) = tokens.iter().position(|t| t == "тұрамын" || t == "тұрамыз")
    {
        if let Some(name) = recover_named_place_before_generic_location(tokens, raw_tokens) {
            return Some(Some(name));
        }
        let city = (0..verb_idx)
            .rev()
            .find_map(|i| strip_locative(&tokens[i]).map(|_| raw_tokens[i].clone()))
            .map(|raw| strip_locative_preserving(&raw));
        return Some(city);
    }
    // Ablative + 1sg copula: "Алматыданмын" → "Алматы".
    if let Some(name) = recover_named_place_before_generic_location(tokens, raw_tokens) {
        for token in tokens {
            if token_mentions_generic_place(token) {
                return Some(Some(name));
            }
        }
    }
    if let Some(name) = recover_named_place_before_origin_marker(tokens, raw_tokens) {
        return Some(Some(name));
    }
    for (i, t) in tokens.iter().enumerate() {
        if let Some(root) = strip_locative_copula(t) {
            let raw = raw_tokens
                .get(i)
                .map(|r| strip_locative_copula_preserving(r).unwrap_or_else(|| root.clone()))
                .unwrap_or(root);
            return Some(Some(normalize_place_name(&raw)));
        }
        if let Some(root) = strip_ablative_copula(t) {
            let raw = raw_tokens
                .get(i)
                .map(|r| strip_ablative_copula_preserving(r).unwrap_or_else(|| root.clone()))
                .unwrap_or(root);
            return Some(Some(normalize_place_name(&raw)));
        }
    }
    // "келдім" + ауыл/қала somewhere — matched but no precise city.
    if joined.contains("келдім") && (joined.contains("ауыл") || joined.contains("қала"))
    {
        return Some(None);
    }
    None
}

/// Strip a trailing locative suffix (`-да/-де/-та/-те`) returning the
/// root. Lower-case variant; returns `None` when the suffix isn't
/// present.
fn strip_locative(token: &str) -> Option<String> {
    for suffix in ["да", "де", "та", "те"] {
        if token.ends_with(suffix) && token.chars().count() > suffix.chars().count() + 1 {
            let take = token.chars().count() - suffix.chars().count();
            return Some(token.chars().take(take).collect());
        }
    }
    None
}

/// Case-preserving version used on `raw_tokens` so proper nouns keep
/// their surface capitalisation.
fn strip_locative_preserving(token: &str) -> String {
    for suffix in ["да", "де", "та", "те", "Да", "Де", "Та", "Те"] {
        if token.ends_with(suffix) && token.chars().count() > suffix.chars().count() + 1 {
            let take = token.chars().count() - suffix.chars().count();
            return token.chars().take(take).collect();
        }
    }
    token.to_string()
}

/// Strip ablative + 1sg copula: `-данмын / -денмін / -танмын / -тенмін`.
/// Requires a root stem of at least 3 characters — prevents a greedy
/// match on short words like "наданмын" ("I am ignorant") leaving
/// stem "на" that would be misrecognised as a city.
fn strip_ablative_copula(token: &str) -> Option<String> {
    const MIN_STEM: usize = 3;
    for suffix in ["данмын", "денмін", "танмын", "тенмін"] {
        if token.ends_with(suffix) && token.chars().count() >= suffix.chars().count() + MIN_STEM {
            let take = token.chars().count() - suffix.chars().count();
            return Some(token.chars().take(take).collect());
        }
    }
    None
}

fn strip_ablative_copula_preserving(token: &str) -> Option<String> {
    const MIN_STEM: usize = 3;
    let lower = token.to_lowercase();
    for suffix in ["данмын", "денмін", "танмын", "тенмін"] {
        if lower.ends_with(suffix) && lower.chars().count() >= suffix.chars().count() + MIN_STEM {
            let take = token.chars().count() - suffix.chars().count();
            return Some(token.chars().take(take).collect());
        }
    }
    None
}

fn strip_locative_copula(token: &str) -> Option<String> {
    const MIN_STEM: usize = 3;
    for suffix in ["дамын", "демін", "тамын", "темін"] {
        if token.ends_with(suffix) && token.chars().count() >= suffix.chars().count() + MIN_STEM {
            let take = token.chars().count() - suffix.chars().count();
            return Some(token.chars().take(take).collect());
        }
    }
    None
}

fn strip_locative_copula_preserving(token: &str) -> Option<String> {
    const MIN_STEM: usize = 3;
    let lower = token.to_lowercase();
    for suffix in ["дамын", "демін", "тамын", "темін"] {
        if lower.ends_with(suffix) && lower.chars().count() >= suffix.chars().count() + MIN_STEM {
            let take = token.chars().count() - suffix.chars().count();
            return Some(token.chars().take(take).collect());
        }
    }
    None
}

fn detect_ask_occupation(joined: &str) -> bool {
    joined.contains("немен айналыс")
        || (joined.contains("жұмысың") && joined.contains("не"))
        || (joined.contains("жұмысыңыз") && joined.contains("не"))
        || joined.contains("кәсібің")
        || joined.contains("кәсібіңіз")
        || joined.contains("мамандығың")
        || joined.contains("мамандығыңыз")
        // v4.4.6 — 1sg self-recall form: "менің мамандығым не?",
        // "менің кәсібім қандай?". Mirrors the v4.4.5 fix to
        // `detect_ask_age` for `менің жасым қанша?`. Pre-v4.4.6
        // these fell through to `Intent::Unknown` with
        // `noun_hint = мамандық/кәсіп` and routed to
        // `unknown.with_grounded_fact`, surfacing a generic
        // definition («Мамандық — адамның кәсібі») instead of
        // recalling the user's stored value via
        // `ask_occupation.with_known_user`. The 1sg-possessive
        // morphemes (`-ым`/`-ім`) plus a question particle
        // (`не`/`қандай`) are an unambiguous self-recall signal.
        || ((joined.contains("мамандығым") || joined.contains("кәсібім"))
            && (joined.contains("не") || joined.contains("қандай")))
        // **v5.3.5** — recall-question variants mirror the v4.54.5
        // detect_ask_name fix: «менің мамандығым есіңізде ме?»,
        // «мамандығымды ұмытпадыңыз ба?», «кәсібімді білесіз бе?».
        // Pre-v5.3.5 «есіңізде ме» co-occurring with «мамандығым»
        // / «кәсібім» fell through to Unknown and surfaced the
        // generic definition «Мамандық — адамның кәсібі» — same
        // class of bug as the original v4.54.5 ask_name fix.
        || ((joined.contains("мамандығым")
            || joined.contains("мамандығымды")
            || joined.contains("кәсібім")
            || joined.contains("кәсібімді"))
            && (joined.contains("есіңізде")
                || joined.contains("есіңде")
                || joined.contains("ұмытпа")
                || joined.contains("ұмытты")
                || joined.contains("білесіз")
                || joined.contains("білесің")
                || joined.contains("білдіңіз")
                || joined.contains("білдің")))
}

/// User states occupation.
///
/// Priority chain (v1.4.0):
/// 1. **FST parse**: any Noun with `predicate = P1Sg` in the parses is
///    the occupation. Root from the Analysis is returned.
/// 2. **Lexicon fallback**: strip 1sg-copula suffix + lookup noun POS.
///    Runs when parser didn't produce a Noun+P1Sg (e.g. word not in
///    Lexicon).
/// 3. **Fixed table**: final fallback with 6 hand-written forms for
///    callers that pass neither parses nor a lexicon.
/// 4. Bare "жұмыс істеймін" matches as occupation = None.
fn detect_statement_of_occupation(
    tokens: &[String],
    _raw_tokens: &[String],
    joined: &str,
    lexicon: Option<&LexiconV1>,
    parses: &[Analysis],
) -> Option<Option<String>> {
    use adam_kernel_fst::morphotactics::Predicate;

    // Priority 1 — FST parse with P1Sg predicate. Only accept real
    // nouns (POS-filtered) — the parser also returns adjective
    // analyses under Analysis::Noun, but "жақсымын" (adj жақсы +
    // P1Sg) is wellbeing, not an occupation.
    //
    // v1.8.5 guard: reject Locative / Ablative case on the noun.
    // "Алматыдамын" (loc+P1Sg) and "Алматыданмын" (abl+P1Sg) are
    // location statements ("I am in / from Almaty"), NOT occupation
    // statements — they're handled by `detect_statement_of_location`.
    use adam_kernel_fst::morphotactics::Case;
    for p in parses {
        if let Analysis::Noun { root, features } = p {
            if features.predicate == Some(Predicate::P1Sg)
                && root.part_of_speech == "noun"
                && !matches!(features.case, Some(Case::Locative) | Some(Case::Ablative))
            {
                return Some(Some(root.root.clone()));
            }
        }
    }

    // Priority 2 — Lexicon-backed copula-strip lookup.
    if let Some(lex) = lexicon {
        if let Some(root) = strip_copula_and_lookup_noun(tokens, lex) {
            return Some(Some(root));
        }
    } else {
        // Priority 3 — fixed table for no-lexicon callers.
        const OCCUPATIONS: &[(&str, &str)] = &[
            ("мұғаліммін", "мұғалім"),
            ("дәрігермін", "дәрігер"),
            ("студентпін", "студент"),
            ("инженермін", "инженер"),
            ("оқушымын", "оқушы"),
            ("жұмысшымын", "жұмысшы"),
        ];
        for t in tokens {
            for (form, root) in OCCUPATIONS {
                if t == form {
                    return Some(Some((*root).to_string()));
                }
            }
        }
    }
    if joined.contains("жұмыс істеймін") {
        // **v5.2.5** — Codex round-3 audit Bug 1. «Мен кім болып
        // жұмыс істеймін?» is a QUESTION («what do I work as?»),
        // not a statement. Pre-fix this path returned
        // `Some(None)` → `Intent::StatementOfOccupation { occupation:
        // None }` → planner picked a template with `{occupation}`
        // placeholder that template_is_fillable couldn't reject (no
        // family-wide guard) → user saw the literal placeholder in
        // the rendered output. Skip on interrogative markers so the
        // turn falls through to `detect_ask_occupation` below.
        let is_question = joined.contains("?")
            || joined.contains("кім болып")
            || joined.contains("кім ретінде")
            || joined.contains("қалай ");
        if is_question {
            return None;
        }
        return Some(None);
    }
    None
}

/// **v4.51.0** — Detect «what are you working on?» / «what do I do?»
/// — companion to `detect_ask_occupation`. Distinct because activity
/// queries ask about CURRENT-WORK-CONTENT, not the profession label.
///
/// Patterns:
/// - «не істейсіз» / «не істейсің» («what are you doing/working on?»)
/// - «не әзірлейсіз» / «не әзірлеп жатырсыз» («what are you developing?»)
/// - «не жасап жатырсыз» («what are you making?»)
/// - «менің ісім не» / «менің ісім қандай» (1sg self-recall)
/// - «менің не істейтінім» (1sg self-recall, embedded clause)
/// - «менің не істейтіні» / «істейтіні» (variants)
fn detect_ask_activity(joined: &str) -> bool {
    // **v4.93.0** — Codex 2026-05-07 audit: tighten 2sg/pl matcher
    // so 3sg verbs don't accidentally fire user-activity routing.
    // Pre-fix `joined.contains("не істей")` matched `не істейді`
    // (3sg "what does X do") just as readily as `не істейсіз`
    // (2pl/polite "what are you doing"). Result: «match өрнегі не
    // істейді?» / «Tokio runtime не істейді?» were routed to
    // ask-about-user-activity instead of falling through to topic
    // extraction → grounded fact. The same applied to `не жасап` —
    // matches 3sg «не жасап жатыр» (what is X making). Tighten to
    // explicit 2nd-person endings.
    let second_person = (joined.contains("не істейсіз")
        || joined.contains("не істейсің")
        || joined.contains("не істейсіңдер")
        || joined.contains("не істейміз")
        || joined.contains("не әзірлейсіз")
        || joined.contains("не әзірлейсің")
        || joined.contains("не әзірлеп жатырсыз")
        || joined.contains("не әзірлеп жатырсың")
        || joined.contains("не жасайсыз")
        || joined.contains("не жасайсың")
        || joined.contains("не жасап жатырсыз")
        || joined.contains("не жасап жатырсың")
        || joined.contains("немен айналысасыз")
        || joined.contains("немен айналысасың"))
        && !joined.contains("кәсіб");
    // 1sg self-recall: «менің ісім», «менің не істейтінім»,
    // «не істейтінімді» (Acc-marked embedded clause).
    // **v4.51.5** — also catches «не істейтінімді» / «не
    // істейтінін» (Acc on the embedded participle), used in
    // recall queries like «Менің атымды және не істейтінімді
    // есіңізде ме?».
    let self_recall = (joined.contains("менің")
        && (joined.contains("ісім")
            || joined.contains("істейтін")
            || joined.contains("не істе")
            // **v4.52.0** — broader 1sg self-recall: any of the
            // other activity-verb participle stems used in
            // «менің нені [V]атыным/йтінім?» queries.
            || joined.contains("дамытатын")
            || joined.contains("әзірлейтін")
            || joined.contains("жасайтын")
            || joined.contains("жазатын")
            || joined.contains("зерттейтін")
            || joined.contains("құрастыратын")))
        || joined.contains("не істеймін")
        || joined.contains("не істейтінімді")
        || joined.contains("не істейтінім");
    second_person || self_recall
}

/// **v4.51.0** — Detect «I'm working on X» / «X жасаймын» activity
/// statements. Returns `Some(Some(noun_phrase))` when an activity
/// verb fires AND a noun-phrase object is identifiable; `Some(None)`
/// when the verb fires but no object can be recovered («жұмыс
/// істеймін» bare); `None` when no activity verb is present.
///
/// Activity-verb stems (1sg-conjugated form, present-future):
/// - «әзірлеймін» / «әзірлеп жатырмын» — develop / am developing
/// - «жасаймын» / «жасап жатырмын» — make / am making
/// - «жазамын» — write (code / text)
/// - «зерттеймін» — research
/// - «айналысамын» — engage with (preceded by Comitative «-мен»)
///
/// Object extraction: the noun phrase preceding the activity verb,
/// stripped of Accusative/Comitative case suffixes when possible.
/// Multi-word phrases like «жасанды интеллект» are kept whole.
fn detect_statement_of_activity(tokens: &[String], joined: &str) -> Option<Option<String>> {
    const ACTIVITY_VERBS: &[&str] = &[
        // v4.51.0 — finite 1sg present-future forms.
        "әзірлеймін",
        "жасаймын",
        "жазамын",
        "зерттеймін",
        "айналысамын",
        "құрастырамын",
        // **v4.52.0** — additional roots: дамыту (develop), құру (build).
        "дамытамын",
        "құрамын",
        // **v4.51.5** — participle-form (Aorist Participle, -йтін /
        // -етін / -атын / -йтын) used in compound clauses like
        // «жасанды интеллект моделімді әзірлейтін бағдарламашымын» —
        // "(I am) a programmer who develops AI models". The
        // participle modifies an occupation noun; the activity is
        // the noun phrase preceding the participle.
        "әзірлейтін",
        "жасайтын",
        "жазатын",
        "зерттейтін",
        "айналысатын",
        "құрастыратын",
        // **v4.52.0** — participle for new roots.
        "дамытатын",
        "құратын",
    ];
    // **v4.52.0** — converb stems used in present-progressive
    // «X-ды Y-іп жатырмын» = "I am Y-ing X". Bigram match: the
    // converb is followed by the «жатырмын» (1sg) / «жатырмыз»
    // (1pl) auxiliary. The converb itself sits in verb-position
    // for the walk-back logic — the object precedes it.
    const CONTINUOUS_CONVERBS: &[&str] = &[
        "әзірлеп",
        "жасап",
        "жазып",
        "зерттеп",
        "құрастырып",
        "айналысып",
        "дамытып",
        "құрып",
    ];
    // Find the activity verb position in tokens (verb at end of clause).
    let mut verb_idx: Option<usize> = None;
    for (i, t) in tokens.iter().enumerate() {
        let t_clean = t.trim_end_matches('.').trim_end_matches('!');
        if ACTIVITY_VERBS.contains(&t_clean) {
            verb_idx = Some(i);
            break;
        }
        // **v4.52.0** — continuous-form: converb + «жатыр*» auxiliary.
        if CONTINUOUS_CONVERBS.contains(&t_clean) {
            if let Some(next) = tokens.get(i + 1) {
                let next_clean = next.trim_end_matches('.').trim_end_matches('!');
                if next_clean.starts_with("жатыр") {
                    verb_idx = Some(i);
                    break;
                }
            }
        }
    }
    let verb_idx = verb_idx?;
    // Extract the noun phrase preceding the verb. Walk backwards
    // from verb_idx, stopping at clause boundaries («және», «ал»,
    // commas attached to tokens) or pronouns («мен», «менің»).
    let mut object_tokens: Vec<&str> = Vec::new();
    for i in (0..verb_idx).rev() {
        let t = tokens[i].as_str();
        // Stop on clause boundary tokens.
        if matches!(
            t,
            "және"
                | "ал"
                | "содан"
                | "бірақ"
                | "сондықтан"
                | "мен"
                | "менің"
                // **v4.51.5** — reflexive-genitive boundary («of myself»)
                // for compound participle clauses like «мен өзімнің
                // X-ді әзірлейтін бағдарламашымын» — without this
                // boundary the noun phrase walk would include «өзімнің»
                // which is meta-grammatical, not part of the activity.
                | "өзімнің"
                | "өзіңнің"
                | "өзіміздің"
        ) {
            break;
        }
        // Strip trailing comma if present.
        let t = t.trim_end_matches(',');
        if t.is_empty() {
            break;
        }
        object_tokens.push(t);
    }
    if object_tokens.is_empty() {
        // «жұмыс істеймін» bare — verb fires but no object.
        if joined.contains("жұмыс істеймін") || joined.contains("істеп жатырмын")
        {
            return Some(None);
        }
        return None;
    }
    // Reverse to natural order.
    object_tokens.reverse();
    // Strip Acc/Comitative case suffixes from the LAST token (head of
    // the noun phrase) to surface the bare-form root. Conservative —
    // only the most common 1-2 suffix variants.
    let last = object_tokens.pop()?;
    let bare = strip_object_case_suffix(last);
    object_tokens.push(bare.as_str());
    let activity = object_tokens.join(" ");
    // Reject single-character / closed-class objects.
    if activity.chars().count() < 3 {
        return None;
    }
    Some(Some(activity))
}

/// **v4.51.0** — Secondary scan for compound «X және Y» inputs where
/// X is an occupation statement and Y contains an activity verb.
/// Called by `Conversation::turn` AFTER primary intent absorption to
/// rescue the activity slot when the primary detector fired
/// occupation. Returns the same `Option<Option<String>>` shape as
/// `detect_statement_of_activity`.
pub(crate) fn detect_activity_in_compound(input: &str) -> Option<Option<String>> {
    let lower = input.to_lowercase();
    // **v4.51.0 path** — explicit «және» split.
    if lower.contains("және") {
        let after = lower.split("және").nth(1)?.trim();
        if !after.is_empty() {
            let after_tokens: Vec<String> = after
                .split_whitespace()
                .map(|t| {
                    t.trim_matches(|c: char| c == '.' || c == '!' || c == '?' || c == ',')
                        .to_string()
                })
                .filter(|t| !t.is_empty())
                .collect();
            if !after_tokens.is_empty()
                && let Some(activity) = detect_statement_of_activity(&after_tokens, after)
            {
                return Some(activity);
            }
        }
    }
    // **v4.51.5 path** — participle-modifier same-clause. The
    // input has no «және» but contains a participle-form activity
    // verb («әзірлейтін» / «жасайтын» / etc.) modifying an
    // occupation noun («әзірлейтін бағдарламашымын»). Run activity
    // detector on the full input — it picks up the participle and
    // walks back to extract the noun phrase.
    let tokens: Vec<String> = lower
        .split_whitespace()
        .map(|t| {
            t.trim_matches(|c: char| c == '.' || c == '!' || c == '?' || c == ',')
                .to_string()
        })
        .filter(|t| !t.is_empty())
        .collect();
    if tokens.is_empty() {
        return None;
    }
    detect_statement_of_activity(&tokens, &lower)
}

/// **v4.52.0** — Secondary scan for compound inputs where the user
/// states multiple things at once (name + occupation + activity)
/// e.g. «Менің атым Дәулет, мен бағдарламашымын және ...». The
/// primary intent detector picks ONE intent (StatementOfName wins
/// priority); this scan rescues the `occupation` slot when it sits
/// in a non-primary clause.
///
/// Mirrors `detect_activity_in_compound`. Returns `Some(Some(root))`
/// when a noun + 1sg copula form is identifiable, `None` otherwise.
pub(crate) fn detect_occupation_in_compound(
    input: &str,
    lexicon: Option<&LexiconV1>,
) -> Option<Option<String>> {
    let lower = input.to_lowercase();
    let tokens: Vec<String> = lower
        .split_whitespace()
        .map(|t| {
            t.trim_matches(|c: char| c == '.' || c == ',' || c == '!' || c == '?')
                .to_string()
        })
        .filter(|t| !t.is_empty())
        .collect();
    if tokens.is_empty() {
        return None;
    }
    // **v5.3.5** — possessive-form pattern «мамандығым X» / «кәсібім X»
    // (1sg-poss-marked profession noun + bare-noun complement).
    // Pre-fix the user's compound «Менің атым Дәулет, мамандығым
    // бағдарламашы» surfaced StatementOfName for the first clause
    // and lost the occupation from the second — `strip_copula_and_lookup_noun`
    // only handled copula-suffixed forms («бағдарламашымын»). The
    // bare form needs an explicit possessive-anchor scan.
    for (i, t) in tokens.iter().enumerate() {
        let alpha: String = t.chars().filter(|c| c.is_alphabetic()).collect();
        if alpha == "мамандығым" || alpha == "кәсібім" {
            // **v5.4.6** — skip punctuation-only tokens between the
            // possessive anchor and the value. Pre-v5.4.6 «Мамандығым
            // — бағдарламашы.» tokenised as
            // [«мамандығым», «—», «бағдарламашы»]; the detector read
            // tokens[i+1] = «—» (em-dash), the alphabetic-filter
            // returned the empty string, and the fact was never
            // absorbed. Walk forward through punctuation glyphs so
            // both bare («Мамандығым бағдарламашы») and dash-separated
            // («Мамандығым — бағдарламашы») forms reach the same code
            // path.
            let mut j = i + 1;
            while let Some(tok) = tokens.get(j) {
                let alpha_only: String = tok.chars().filter(|c| c.is_alphabetic()).collect();
                if alpha_only.is_empty() {
                    j += 1;
                    continue;
                }
                if alpha_only != "болып" && alpha_only != "ретінде" && alpha_only != "екен"
                {
                    return Some(Some(alpha_only));
                }
                break;
            }
        }
    }
    if let Some(lex) = lexicon
        && let Some(root) = strip_copula_and_lookup_noun(&tokens, lex)
    {
        return Some(Some(root));
    }
    // Fallback fixed table for callers without lexicon access.
    const OCCUPATIONS: &[(&str, &str)] = &[
        ("мұғаліммін", "мұғалім"),
        ("дәрігермін", "дәрігер"),
        ("студентпін", "студент"),
        ("инженермін", "инженер"),
        ("оқушымын", "оқушы"),
        ("жұмысшымын", "жұмысшы"),
        ("бағдарламашымын", "бағдарламашы"),
        ("ғалыммын", "ғалым"),
        ("аудармашымын", "аудармашы"),
        ("суретшімін", "суретші"),
        ("дизайнермін", "дизайнер"),
        ("әншімін", "әнші"),
        ("саудагермін", "саудагер"),
        ("кәсіпкермін", "кәсіпкер"),
        ("заңгермін", "заңгер"),
    ];
    for t in &tokens {
        for (form, root) in OCCUPATIONS {
            if t == form {
                return Some(Some((*root).to_string()));
            }
        }
    }
    None
}

/// Strip Accusative («-ды/-ді/-ны/-ні/-ты/-ті»), Comitative («-мен»),
/// or other case suffixes from a noun head. Returns the bare-form
/// root string. Conservative — only strips when the token is long
/// enough that the residue is at least 3 chars.
fn strip_object_case_suffix(token: &str) -> String {
    // **v4.51.5** — handles compound P1Sg+Acc forms like «моделімді»
    // («модель» + P1Sg «-ім» + Acc «-ді»). Strip Acc first; if
    // residue ends in P1Sg «-ім»/«-ім», strip that too.
    const ACC_SUFFIXES: &[&str] = &["ды", "ді", "ны", "ні", "ты", "ті", "мен", "пен", "бен"];
    const POS_SUFFIXES: &[&str] = &["ім", "ім", "ым", "м", "сі", "сы", "і", "ы"];
    let mut current = token.to_string();
    // Pass 1: strip Acc/Comitative.
    for suf in ACC_SUFFIXES {
        if let Some(stripped) = current.strip_suffix(suf) {
            if stripped.chars().count() >= 3 {
                current = stripped.to_string();
                break;
            }
        }
    }
    // Pass 2: strip Possessive (P1Sg / P3) if residue still ≥ 3 chars.
    for suf in POS_SUFFIXES {
        if let Some(stripped) = current.strip_suffix(suf) {
            if stripped.chars().count() >= 3 {
                current = stripped.to_string();
                break;
            }
        }
    }
    current
}

/// For every token ending in a Kazakh 1sg-copula suffix, strip the
/// suffix and check whether the residue is a `noun` in the lexicon.
/// Returns the first hit. Skips residues that are tagged as adjectives
/// (otherwise `"жақсымын"` — adj "жақсы" + 1sg — would falsely register
/// as an occupation statement).
fn strip_copula_and_lookup_noun(tokens: &[String], lex: &LexiconV1) -> Option<String> {
    const COPULA_SUFFIXES: &[&str] = &["мын", "мін", "пын", "пін", "бын", "бін"];
    for t in tokens {
        for suffix in COPULA_SUFFIXES {
            let Some(root) = strip_suffix_chars(t, suffix) else {
                continue;
            };
            // Minimum stem length of 2 chars — guards against stripping
            // short function words.
            if root.chars().count() < 2 {
                continue;
            }
            if let Some(entry) = lex.get(&root) {
                if entry.part_of_speech == "noun" {
                    return Some(root);
                }
            }
        }
    }
    None
}

/// Strip `suffix` from the end of `token` using Unicode-aware
/// character counting (avoids byte-slicing into a UTF-8 codepoint).
fn strip_suffix_chars(token: &str, suffix: &str) -> Option<String> {
    if !token.ends_with(suffix) {
        return None;
    }
    let take = token.chars().count() - suffix.chars().count();
    Some(token.chars().take(take).collect())
}

/// "Family question" — үйлендің бе, балаларың бар ма, отбасың бар ма.
fn detect_ask_family(joined: &str) -> bool {
    joined.contains("үйлендің")
        || joined.contains("үйлендіңіз")
        || (joined.contains("балаларың") && joined.contains("бар"))
        || (joined.contains("балаларыңыз") && joined.contains("бар"))
        || (joined.contains("отбасың") && joined.contains("бар"))
        || (joined.contains("отбасыңыз") && joined.contains("бар"))
}

/// User talks about family: "менің балам бар", "үйленгенмін",
/// "отбасым бар".
fn detect_statement_of_family(joined: &str) -> bool {
    joined.contains("балам бар")
        || joined.contains("балаларым бар")
        || joined.contains("үйленгенмін")
        || joined.contains("отбасым бар")
        || joined.contains("отбасым жақсы")
}

fn detect_ask_weather(joined: &str) -> bool {
    (joined.contains("ауа райы") && joined.contains("қалай"))
        || (joined.contains("бүгін") && joined.contains("ауа райы"))
        || (joined.contains("сыртта") && joined.contains("қалай"))
}

/// User describes weather: "бүгін суық", "жылы", "қар жауып тұр",
/// "күн ашық".
fn detect_statement_of_weather(tokens: &[String], joined: &str) -> bool {
    let weather_token = tokens.iter().any(|t| {
        matches!(
            t.as_str(),
            "суық" | "жылы" | "ыстық" | "салқын" | "жаңбырлы" | "қарлы"
        )
    });
    let weather_phrase = joined.contains("қар жауып")
        || joined.contains("жаңбыр жауып")
        || joined.contains("күн ашық")
        || joined.contains("ауа райы жақсы")
        || joined.contains("ауа райы жаман");
    // Guard: "жақсы" / "жаман" alone is wellbeing, not weather —
    // require the keyword OR the phrase, never bare goodness tokens.
    (weather_token && (joined.contains("бүгін") || joined.contains("қазір"))) || weather_phrase
}

fn detect_ask_time(joined: &str) -> bool {
    (joined.contains("сағат") && (joined.contains("неше") || joined.contains("қанша")))
        || joined.contains("қазір уақыт")
        || joined.contains("қандай күн")
        || joined.contains("қай күн")
}

fn detect_compliment(tokens: &[String], joined: &str) -> bool {
    // **v4.17.5** — Compliment detection tightened. Pre-v4.17.5 the
    // detector fired on `өте жақсы` regardless of context, so live
    // REPL «Бұл өте жақсы, бірақ ақылды болу жеткіліксіз. Сіз
    // жақсаруды үйренуге дайынсыз ба?» routed to Compliment (turn
    // 19, 2026-05-01 transcript) and adam responded with the
    // «Сіз де жақсы жансыз» template — completely missing the
    // follow-up question. Now: a `бірақ` + 2nd-person yes/no
    // question (`дайынсыз ба` etc.) downstream of `өте жақсы`
    // means this isn't a compliment turn.
    let has_followup_question =
        joined.contains("бірақ") && (joined.contains("ма?") || joined.contains("ба?"));
    if has_followup_question {
        return false;
    }
    // Also defer when the user explicitly asks adam something —
    // `дайынсыз ба / дайынсың ба` (are you ready?) is an
    // introspection question, not a compliment, even if the
    // sentence opens with «өте жақсы».
    if joined.contains("дайынсыз ба") || joined.contains("дайынсың ба") {
        return false;
    }
    tokens.iter().any(|t| {
        matches!(
            t.as_str(),
            "жарайсың" | "жарайсыз" | "керемет" | "тамаша" | "мықты"
        )
    }) || joined.contains("өте жақсы")
}

fn detect_request(tokens: &[String], joined: &str) -> bool {
    tokens.iter().any(|t| {
        matches!(
            t.as_str(),
            "өтінемін" | "сұраймын" | "көмектесіңізші" | "көмектесіңіз" | "көмектес"
        )
    }) || joined.contains("көмек керек")
}

fn detect_well_wishes(joined: &str) -> bool {
    joined.contains("жақсы күн тіле")
        || joined.contains("сәттілік")
        || joined.contains("табысты бол")
        || joined.contains("денсаулық тіле")
}

/// Insult / rudeness — polite non-engagement (v1.1.0).
/// The model doesn't escalate or retaliate; responds with dignity.
fn detect_insult(tokens: &[String], joined: &str) -> bool {
    tokens.iter().any(|t| {
        matches!(
            t.as_str(),
            "ақымақ"
                | "ақымақсың"
                | "ақымақсыз"
                | "надан"
                | "наданмын"
                | "надансың"
                | "өтірік"
        )
    }) || joined.contains("ақылсыз")
        || joined.contains("түкке тұрмайсың")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// v3.9.5 regression: «Неліктен» / «Неге» / «Қашан» etc. must not be
    /// extracted as a topic-noun. Pre-v3.9.5 the REPL would reply with
    /// «Нелікте тұрасыз ба» (= "Do you live in Нелік?") to the input
    /// «Неліктен?» because the FST parsed it as `Нелік` + ablative and
    /// `NOT_A_TOPIC` did not include the interrogative.
    #[test]
    fn not_a_topic_covers_v3_9_5_additions() {
        // Interrogatives — primary fix.
        for word in [
            "неліктен",
            "неге",
            "қашан",
            "қайда",
            "қандай",
            "кім",
            "қанша",
        ] {
            assert!(
                NOT_A_TOPIC.contains(&word),
                "interrogative `{word}` must be in NOT_A_TOPIC"
            );
        }
        // Demonstrative qualifiers.
        for word in ["мұндай", "сондай", "ондай", "кейбір", "өз", "әрбір"]
        {
            assert!(
                NOT_A_TOPIC.contains(&word),
                "demonstrative `{word}` must be in NOT_A_TOPIC"
            );
        }
        // Content nouns still pass through the gate.
        for word in ["бала", "кітап", "мектеп", "қазақстан", "жер"] {
            assert!(
                !NOT_A_TOPIC.contains(&word),
                "content noun `{word}` must NOT be in NOT_A_TOPIC"
            );
        }
    }

    /// v4.0.21 — multi-word entity matcher returns the compound entity
    /// before the single-word fallback. Pre-v4.0.21 «Құс жолы туралы
    /// айтшы» (tell me about the Milky Way) tokenised to «құс» + «жолы»
    /// and the reply was about birds («құс»), losing the galaxy referent.
    #[test]
    fn multiword_entity_hint_matches_compound_entities() {
        assert_eq!(
            multiword_entity_hint("Құс жолы туралы айтшы"),
            Some("құс жолы".to_string())
        );
        assert_eq!(
            multiword_entity_hint("Күн жүйесі өте кең"),
            Some("күн жүйесі".to_string())
        );
        assert_eq!(
            multiword_entity_hint("Аспан денесі жайлы"),
            Some("аспан денесі".to_string())
        );
        // Inflected last-word: substring match still fires (Kazakh
        // agglutinates on the compound tail).
        assert_eq!(
            multiword_entity_hint("Құс жолының бейнесі"),
            Some("құс жолы".to_string())
        );
    }

    /// **v4.11.5** — query-time school-curriculum compounds are
    /// resolved to the canonical compound topic, regardless of
    /// inflection on the last word. Real-REPL 2026-04-30:
    /// «Адам, сен мектептің физика бағдарламасын білесің бе?»
    /// pre-v4.11.5 fell through to either `физика` (ignoring the
    /// program-of context) or — worse — `адам` (the vocative).
    #[test]
    fn multiword_entity_hint_matches_curriculum_compounds() {
        assert_eq!(
            multiword_entity_hint("мектептің физика бағдарламасын білесің бе?"),
            Some("физика бағдарламасы".to_string())
        );
        assert_eq!(
            multiword_entity_hint("Сен мектеп пәндерін білесің бе?"),
            Some("мектеп пәндері".to_string())
        );
        assert_eq!(
            multiword_entity_hint("Биология бағдарламасы туралы айтшы"),
            Some("биология бағдарламасы".to_string())
        );
        assert_eq!(
            multiword_entity_hint("Информатика бағдарламасын білесің бе?"),
            Some("информатика бағдарламасы".to_string())
        );
    }

    /// v4.0.21 — no multi-word match ⇒ None, so the single-word fallback
    /// activates downstream.
    #[test]
    fn multiword_entity_hint_returns_none_for_simple_input() {
        assert_eq!(multiword_entity_hint("құс туралы айтшы"), None);
        assert_eq!(multiword_entity_hint("мектеп керек пе"), None);
        assert_eq!(multiword_entity_hint(""), None);
    }

    #[test]
    fn generic_location_phrase_recovers_named_place_prefix() {
        let got = interpret_text("Мен Қашар ауылында тұрамын", &[]);
        assert_eq!(
            got,
            Intent::StatementOfLocation {
                city: Some("Қашар".into())
            }
        );
    }

    #[test]
    fn statement_of_name_normalises_lowercase_and_mixed_script() {
        let got = interpret_text("атым дӘУЛEТ", &[]);
        assert_eq!(
            got,
            Intent::StatementOfName {
                name: "Дәулет".into()
            }
        );
    }

    #[test]
    fn statement_of_location_recovers_lowercase_named_place_prefix() {
        let got = interpret_text("мен қашар ауылында тұрамын", &[]);
        assert_eq!(
            got,
            Intent::StatementOfLocation {
                city: Some("Қашар".into())
            }
        );
    }

    #[test]
    fn statement_of_location_normalises_mixed_script_city() {
        let got = interpret_text("мен Aлматыданмын", &[]);
        assert_eq!(
            got,
            Intent::StatementOfLocation {
                city: Some("Алматы".into())
            }
        );
    }

    #[test]
    fn statement_of_location_recovers_geo_feature_before_origin_marker() {
        let got = interpret_text("мен каспий жақтанмын", &[]);
        assert_eq!(
            got,
            Intent::StatementOfLocation {
                city: Some("Каспий".into())
            }
        );
    }

    #[test]
    fn statement_of_location_normalises_curated_geo_alias() {
        let got = interpret_text("мен Алма-Атадамын", &[]);
        assert_eq!(
            got,
            Intent::StatementOfLocation {
                city: Some("Алматы".into())
            }
        );
    }

    #[test]
    fn statement_of_location_recovers_geo_feature_phrase_before_origin_marker() {
        let got = interpret_text("мен каспий теңізі жақтанмын", &[]);
        assert_eq!(
            got,
            Intent::StatementOfLocation {
                city: Some("Каспий".into())
            }
        );
    }

    /// v4.0.26 — Codex v4.0.23 residual: the v4.0.21 MULTIWORD_ENTITIES
    /// docstring referenced a `world_core_multiword_coverage_test` that
    /// didn't actually exist. This test closes that maintenance trap.
    ///
    /// It scans every JSONL entry in `data/world_core/` and asserts that
    /// every compound subject/object (value containing a space) appears
    /// in the `MULTIWORD_ENTITIES` const. Adding a new compound to
    /// world_core without extending the const will now fail CI.
    ///
    /// Skips silently when the data directory is absent (trimmed CI
    /// checkouts, external crate consumers). Production CI runs from
    /// the repo root so the data is always present.
    #[test]
    fn world_core_multiword_coverage() {
        let dir = std::path::Path::new("../../data/world_core");
        if !dir.exists() {
            eprintln!(
                "world_core_multiword_coverage: skipping — {} not present",
                dir.display()
            );
            return;
        }
        let mut observed: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let entries = std::fs::read_dir(dir).expect("read_dir data/world_core");
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            let contents = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            for line in contents.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let parsed: serde_json::Value = match serde_json::from_str(line) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let Some(facts) = parsed.get("facts").and_then(|v| v.as_array()) else {
                    continue;
                };
                for fact in facts {
                    for key in ["subject", "object"] {
                        if let Some(value) = fact.get(key).and_then(|v| v.as_str()) {
                            if value.contains(' ') {
                                observed.insert(value.to_string());
                            }
                        }
                    }
                }
            }
        }
        let const_set: std::collections::BTreeSet<String> =
            MULTIWORD_ENTITIES.iter().map(|s| s.to_string()).collect();
        let missing: Vec<&String> = observed.difference(&const_set).collect();
        assert!(
            missing.is_empty(),
            "world_core has {} compound entities not in MULTIWORD_ENTITIES; add them to the const in semantics.rs: {missing:?}",
            missing.len()
        );
    }
}
