use std::{env, fs, process::ExitCode};

use adam_kernel::{
    FinalSoundClass, SegmentationLexicon, SegmentationPartOfSpeech, SegmentationRootEntry,
    SegmentationRuleSet, SegmentationState, SegmentationSuffixRule, VowelHarmony,
    deterministic_segment_token,
};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
struct SynthSample {
    id: String,
    pack_name: String,
    source_id: String,
    domain: String,
    text: String,
}

#[derive(Debug, Clone, Serialize)]
struct SynthPack {
    version: String,
    name: String,
    target_language: String,
    script: String,
    samples: Vec<SynthSample>,
}

struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.state
    }
    fn range(&mut self, max: usize) -> usize {
        (self.next_u64() as usize) % max.max(1)
    }
    fn pick<'a, T>(&mut self, slice: &'a [T]) -> Option<&'a T> {
        if slice.is_empty() {
            None
        } else {
            Some(&slice[self.range(slice.len())])
        }
    }
}

fn classify_final_sound(form: &str) -> FinalSoundClass {
    let last = form.chars().last().expect("non-empty form");
    if matches!(
        last,
        'а' | 'ә' | 'е' | 'и' | 'о' | 'ө' | 'ұ' | 'ү' | 'у' | 'ы' | 'і' | 'э'
    ) {
        FinalSoundClass::Vowel
    } else if matches!(last, 'м' | 'н' | 'ң') {
        FinalSoundClass::Nasal
    } else if matches!(
        last,
        'п' | 'ф' | 'к' | 'қ' | 'т' | 'с' | 'ш' | 'щ' | 'ч' | 'ц' | 'х' | 'һ'
    ) {
        FinalSoundClass::VoicelessConsonant
    } else {
        FinalSoundClass::VoicedConsonant
    }
}

fn matching_rules<'a>(
    rules: &'a SegmentationRuleSet,
    pos: &SegmentationPartOfSpeech,
    from_state: &SegmentationState,
    label: &str,
    harmony: &VowelHarmony,
    fsc: &FinalSoundClass,
    prev_label: Option<&str>,
) -> Vec<&'a SegmentationSuffixRule> {
    rules
        .rules
        .iter()
        .filter(|r| {
            r.part_of_speech == *pos
                && r.from_state == *from_state
                && r.label == label
                && r.allowed_harmonies.contains(harmony)
                && r.allowed_final_sound_classes.contains(fsc)
                && (r.allowed_previous_labels.is_empty()
                    || prev_label
                        .is_some_and(|pl| r.allowed_previous_labels.iter().any(|al| al == pl)))
        })
        .collect()
}

fn inflect(
    root: &SegmentationRootEntry,
    labels: &[&str],
    rules: &SegmentationRuleSet,
    rng: &mut Lcg,
) -> Option<String> {
    let mut word = root.root.clone();
    let mut state = SegmentationState::Stem;
    let harmony = root.vowel_harmony.clone();
    let mut fsc = root.final_sound_class.clone();
    let mut prev: Option<String> = None;

    for label in labels {
        let candidates = matching_rules(
            rules,
            &root.part_of_speech,
            &state,
            label,
            &harmony,
            &fsc,
            prev.as_deref(),
        );
        let rule = rng.pick(&candidates)?;
        word.push_str(&rule.form);
        state = rule.to_state.clone();
        fsc = classify_final_sound(&rule.form);
        prev = Some(rule.label.clone());
    }
    Some(word)
}

#[allow(clippy::too_many_arguments)]
fn generate_one(
    rng: &mut Lcg,
    nouns: &[&SegmentationRootEntry],
    verbs: &[&SegmentationRootEntry],
    adjectives: &[&SegmentationRootEntry],
    postpositions: &[&SegmentationRootEntry],
    numerals: &[&SegmentationRootEntry],
    adverbs: &[&SegmentationRootEntry],
    conjunctions: &[&SegmentationRootEntry],
    particles: &[&SegmentationRootEntry],
    modals: &[&SegmentationRootEntry],
    rules: &SegmentationRuleSet,
) -> Option<String> {
    let template_idx = rng.range(30);
    match template_idx {
        0 => {
            // N + V(aorist 3sg)
            let subj = *rng.pick(nouns)?;
            let verb = *rng.pick(verbs)?;
            let v = inflect(verb, &["future", "person_3sg"], rules, rng)?;
            Some(format!("{} {}.", subj.root, v))
        }
        1 => {
            // Adj + N
            let adj = *rng.pick(adjectives)?;
            let n = *rng.pick(nouns)?;
            Some(format!("{} {}.", adj.root, n.root))
        }
        2 => {
            // N(acc) + V(aorist 3sg)
            let obj = *rng.pick(nouns)?;
            let verb = *rng.pick(verbs)?;
            let o = inflect(obj, &["accusative"], rules, rng)?;
            let v = inflect(verb, &["future", "person_3sg"], rules, rng)?;
            Some(format!("{} {}.", o, v))
        }
        3 => {
            // N + postposition
            let n = *rng.pick(nouns)?;
            let p = *rng.pick(postpositions)?;
            Some(format!("{} {}.", n.root, p.root))
        }
        4 => {
            // Num + N
            let num = *rng.pick(numerals)?;
            let n = *rng.pick(nouns)?;
            Some(format!("{} {}.", num.root, n.root))
        }
        5 => {
            // Adj + predicative_1sg
            let adj = *rng.pick(adjectives)?;
            let text = inflect(adj, &["predicative_1sg"], rules, rng)?;
            Some(format!("{}.", text))
        }
        6 => {
            // N + V(past)
            let subj = *rng.pick(nouns)?;
            let verb = *rng.pick(verbs)?;
            let v = inflect(verb, &["past"], rules, rng)?;
            Some(format!("{} {}.", subj.root, v))
        }
        7 => {
            // N(plural) + V(aorist 3sg)
            let subj = *rng.pick(nouns)?;
            let verb = *rng.pick(verbs)?;
            let s = inflect(subj, &["plural"], rules, rng)?;
            let v = inflect(verb, &["future", "person_3sg"], rules, rng)?;
            Some(format!("{} {}.", s, v))
        }
        8 => {
            // N(dat) + V(aorist 3sg)
            let np = *rng.pick(nouns)?;
            let verb = *rng.pick(verbs)?;
            let n = inflect(np, &["dative"], rules, rng)?;
            let v = inflect(verb, &["future", "person_3sg"], rules, rng)?;
            Some(format!("{} {}.", n, v))
        }
        9 => {
            // N(loc) + V(aorist 3sg)
            let np = *rng.pick(nouns)?;
            let verb = *rng.pick(verbs)?;
            let n = inflect(np, &["locative"], rules, rng)?;
            let v = inflect(verb, &["future", "person_3sg"], rules, rng)?;
            Some(format!("{} {}.", n, v))
        }
        10 => {
            // N(gen) + N(poss_3sg)
            let owner = *rng.pick(nouns)?;
            let thing = *rng.pick(nouns)?;
            let og = inflect(owner, &["genitive"], rules, rng)?;
            let tp = inflect(thing, &["possessive_3sg"], rules, rng)?;
            Some(format!("{} {}.", og, tp))
        }
        11 => {
            // Adj + N + V(aorist 3sg)
            let adj = *rng.pick(adjectives)?;
            let subj = *rng.pick(nouns)?;
            let verb = *rng.pick(verbs)?;
            let v = inflect(verb, &["future", "person_3sg"], rules, rng)?;
            Some(format!("{} {} {}.", adj.root, subj.root, v))
        }
        12 => {
            // Pronoun (1/2) + V(matched person)
            let subjects: &[(&str, &str)] = &[
                ("мен", "person_1sg"),
                ("сен", "person_2sg"),
                ("сіз", "person_2polite"),
                ("біз", "person_1pl"),
            ];
            let (pron, person_label) = subjects[rng.range(subjects.len())];
            let verb = *rng.pick(verbs)?;
            let v = inflect(verb, &["future", person_label], rules, rng)?;
            Some(format!("{} {}.", pron, v))
        }
        13 => {
            // N + V(negative_past)
            let subj = *rng.pick(nouns)?;
            let verb = *rng.pick(verbs)?;
            let v = inflect(verb, &["negative_past"], rules, rng)?;
            Some(format!("{} {}.", subj.root, v))
        }
        14 => {
            // Adj + N + postposition
            let adj = *rng.pick(adjectives)?;
            let n = *rng.pick(nouns)?;
            let p = *rng.pick(postpositions)?;
            Some(format!("{} {} {}.", adj.root, n.root, p.root))
        }

        // ─── New in v0.0.86: full POS coverage ─────────────────────────
        15 => {
            // N + Adverb + V(aorist 3sg): "адам тез келеді"
            let subj = *rng.pick(nouns)?;
            let adv = *rng.pick(adverbs)?;
            let verb = *rng.pick(verbs)?;
            let v = inflect(verb, &["future", "person_3sg"], rules, rng)?;
            Some(format!("{} {} {}.", subj.root, adv.root, v))
        }
        16 => {
            // Adverb + N + V(aorist 3sg): "қазір адам келеді"
            let adv = *rng.pick(adverbs)?;
            let subj = *rng.pick(nouns)?;
            let verb = *rng.pick(verbs)?;
            let v = inflect(verb, &["future", "person_3sg"], rules, rng)?;
            Some(format!("{} {} {}.", adv.root, subj.root, v))
        }
        17 => {
            // ол + V(aorist 3sg): "ол келеді"
            let verb = *rng.pick(verbs)?;
            let v = inflect(verb, &["future", "person_3sg"], rules, rng)?;
            Some(format!("ол {}.", v))
        }
        18 => {
            // олар + V(aorist 3sg): "олар жасайды"
            let verb = *rng.pick(verbs)?;
            let v = inflect(verb, &["future", "person_3sg"], rules, rng)?;
            Some(format!("олар {}.", v))
        }
        19 => {
            // N + V(aorist 3sg) + particle: "адам келеді ме"
            let subj = *rng.pick(nouns)?;
            let verb = *rng.pick(verbs)?;
            let v = inflect(verb, &["future", "person_3sg"], rules, rng)?;
            let p = *rng.pick(particles)?;
            Some(format!("{} {} {}.", subj.root, v, p.root))
        }
        20 => {
            // Num + N + V(aorist 3sg): "екі адам келеді"
            let num = *rng.pick(numerals)?;
            let subj = *rng.pick(nouns)?;
            let verb = *rng.pick(verbs)?;
            let v = inflect(verb, &["future", "person_3sg"], rules, rng)?;
            Some(format!("{} {} {}.", num.root, subj.root, v))
        }
        21 => {
            // N + V(past) + conj + V(past): "адам келді және кетті"
            let subj = *rng.pick(nouns)?;
            let v1 = *rng.pick(verbs)?;
            let v2 = *rng.pick(verbs)?;
            let iv1 = inflect(v1, &["past"], rules, rng)?;
            let iv2 = inflect(v2, &["past"], rules, rng)?;
            let c = *rng.pick(conjunctions)?;
            Some(format!("{} {} {} {}.", subj.root, iv1, c.root, iv2))
        }
        22 => {
            // N + particle: "адам ғой"
            let subj = *rng.pick(nouns)?;
            let p = *rng.pick(particles)?;
            Some(format!("{} {}.", subj.root, p.root))
        }
        23 => {
            // N + modal: "адам керек"
            let subj = *rng.pick(nouns)?;
            let m = *rng.pick(modals)?;
            Some(format!("{} {}.", subj.root, m.root))
        }
        24 => {
            // Adj + Adj + N: "үлкен жақсы адам"
            let a1 = *rng.pick(adjectives)?;
            let a2 = *rng.pick(adjectives)?;
            let n = *rng.pick(nouns)?;
            Some(format!("{} {} {}.", a1.root, a2.root, n.root))
        }
        25 => {
            // Pronoun + Adverb + V(matched): "мен қазір келемін"
            let subjects: &[(&str, &str)] = &[
                ("мен", "person_1sg"),
                ("сен", "person_2sg"),
                ("сіз", "person_2polite"),
                ("біз", "person_1pl"),
            ];
            let (pron, person_label) = subjects[rng.range(subjects.len())];
            let adv = *rng.pick(adverbs)?;
            let verb = *rng.pick(verbs)?;
            let v = inflect(verb, &["future", person_label], rules, rng)?;
            Some(format!("{} {} {}.", pron, adv.root, v))
        }
        26 => {
            // N(gen) + N(poss_3sg) + V(aorist 3sg): "баланың кітабы келеді"
            let owner = *rng.pick(nouns)?;
            let thing = *rng.pick(nouns)?;
            let verb = *rng.pick(verbs)?;
            let og = inflect(owner, &["genitive"], rules, rng)?;
            let tp = inflect(thing, &["possessive_3sg"], rules, rng)?;
            let v = inflect(verb, &["future", "person_3sg"], rules, rng)?;
            Some(format!("{} {} {}.", og, tp, v))
        }
        27 => {
            // N(plural) + V(past): "адамдар келді"
            let subj = *rng.pick(nouns)?;
            let verb = *rng.pick(verbs)?;
            let s = inflect(subj, &["plural"], rules, rng)?;
            let v = inflect(verb, &["past"], rules, rng)?;
            Some(format!("{} {}.", s, v))
        }
        28 => {
            // N(dat) + N + V(aorist 3sg): "мектепке бала барады"
            let dat_n = *rng.pick(nouns)?;
            let subj = *rng.pick(nouns)?;
            let verb = *rng.pick(verbs)?;
            let d = inflect(dat_n, &["dative"], rules, rng)?;
            let v = inflect(verb, &["future", "person_3sg"], rules, rng)?;
            Some(format!("{} {} {}.", d, subj.root, v))
        }
        _ => {
            // N(loc) + N + V(aorist 3sg): "қалада адам тұрады"
            let loc_n = *rng.pick(nouns)?;
            let subj = *rng.pick(nouns)?;
            let verb = *rng.pick(verbs)?;
            let l = inflect(loc_n, &["locative"], rules, rng)?;
            let v = inflect(verb, &["future", "person_3sg"], rules, rng)?;
            Some(format!("{} {} {}.", l, subj.root, v))
        }
    }
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let target_n: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(18_000);
    let seed: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(42);

    let lexicon: SegmentationLexicon = match load("data/tokenizer/segmentation_roots.json") {
        Ok(v) => v,
        Err(e) => {
            eprintln!("lexicon: {e}");
            return ExitCode::FAILURE;
        }
    };
    let rules: SegmentationRuleSet = match load("data/tokenizer/segmentation_rules.json") {
        Ok(v) => v,
        Err(e) => {
            eprintln!("rules: {e}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = lexicon.validate() {
        eprintln!("lexicon invalid: {e}");
        return ExitCode::FAILURE;
    }
    if let Err(e) = rules.validate() {
        eprintln!("rules invalid: {e}");
        return ExitCode::FAILURE;
    }

    let nouns: Vec<&SegmentationRootEntry> = lexicon
        .roots
        .iter()
        .filter(|r| matches!(r.part_of_speech, SegmentationPartOfSpeech::Noun))
        .collect();
    let verbs: Vec<&SegmentationRootEntry> = lexicon
        .roots
        .iter()
        .filter(|r| matches!(r.part_of_speech, SegmentationPartOfSpeech::Verb))
        .collect();
    let adjectives: Vec<&SegmentationRootEntry> = lexicon
        .roots
        .iter()
        .filter(|r| matches!(r.part_of_speech, SegmentationPartOfSpeech::Adjective))
        .collect();
    let postpositions: Vec<&SegmentationRootEntry> = lexicon
        .roots
        .iter()
        .filter(|r| matches!(r.part_of_speech, SegmentationPartOfSpeech::Postposition))
        .collect();
    let numerals: Vec<&SegmentationRootEntry> = lexicon
        .roots
        .iter()
        .filter(|r| matches!(r.part_of_speech, SegmentationPartOfSpeech::Numeral))
        .collect();
    let adverbs: Vec<&SegmentationRootEntry> = lexicon
        .roots
        .iter()
        .filter(|r| matches!(r.part_of_speech, SegmentationPartOfSpeech::Adverb))
        .collect();
    let conjunctions: Vec<&SegmentationRootEntry> = lexicon
        .roots
        .iter()
        .filter(|r| matches!(r.part_of_speech, SegmentationPartOfSpeech::Conjunction))
        .collect();
    let particles: Vec<&SegmentationRootEntry> = lexicon
        .roots
        .iter()
        .filter(|r| matches!(r.part_of_speech, SegmentationPartOfSpeech::Particle))
        .collect();
    let modals: Vec<&SegmentationRootEntry> = lexicon
        .roots
        .iter()
        .filter(|r| matches!(r.part_of_speech, SegmentationPartOfSpeech::Modal))
        .collect();

    let mut rng = Lcg::new(seed);
    let mut samples: Vec<SynthSample> = Vec::new();
    let mut attempts: usize = 0;
    let max_attempts = target_n * 20;

    while samples.len() < target_n && attempts < max_attempts {
        attempts += 1;
        let Some(text) = generate_one(
            &mut rng,
            &nouns,
            &verbs,
            &adjectives,
            &postpositions,
            &numerals,
            &adverbs,
            &conjunctions,
            &particles,
            &modals,
            &rules,
        ) else {
            continue;
        };
        let words: Vec<&str> = text.trim_end_matches('.').split_whitespace().collect();
        if words
            .iter()
            .all(|w| deterministic_segment_token(w, &lexicon, &rules).is_some())
        {
            samples.push(SynthSample {
                id: format!("synth_{:05}", samples.len() + 1),
                pack_name: "adam-synthetic-sentences-pack".to_string(),
                source_id: "generated_template_pool_v1".to_string(),
                domain: "synthetic".to_string(),
                text,
            });
        }
    }

    let pack = SynthPack {
        version: "0.0.91".to_string(),
        name: "adam-synthetic-sentences-pack".to_string(),
        target_language: "kazakh".to_string(),
        script: "cyrillic".to_string(),
        samples: samples.clone(),
    };

    eprintln!(
        "generated {} valid samples in {} attempts",
        pack.samples.len(),
        attempts
    );
    println!(
        "{}",
        serde_json::to_string_pretty(&pack).expect("serialize")
    );
    ExitCode::SUCCESS
}

fn load<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&contents)?)
}
