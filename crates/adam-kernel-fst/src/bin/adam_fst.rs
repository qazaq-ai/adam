//! adam-fst — minimal CLI for the v1.0.0 deterministic FST.
//!
//! Usage:
//!   adam-fst synthesise --root жаз --tense past --person 1sg
//!   adam-fst analyse жаздым
//!   adam-fst lexicon-stats
//!
//! This is not a polished tool — it exists so that the FST can be
//! demonstrated without writing Rust. For programmatic use, call
//! `adam_kernel_fst::morphotactics::synthesise_noun` / `synthesise_verb`
//! and `adam_kernel_fst::parser::analyse` directly.

use std::env;
use std::process::ExitCode;

use adam_kernel_fst::lexicon::LexiconV1;
use adam_kernel_fst::morphotactics::{
    Case, NounFeatures, Number, Person, Possessive, Tense, VerbFeatures, Voice, synthesise_noun,
    synthesise_verb,
};
use adam_kernel_fst::parser::{Analysis, analyse};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        return ExitCode::FAILURE;
    }
    match args[1].as_str() {
        "synthesise" | "synth" | "gen" => cmd_synthesise(&args[2..]),
        "analyse" | "parse" => cmd_analyse(&args[2..]),
        "lexicon-stats" | "stats" => cmd_stats(),
        "-h" | "--help" | "help" => {
            print_usage();
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("unknown command: {other}");
            print_usage();
            ExitCode::FAILURE
        }
    }
}

fn print_usage() {
    eprintln!(
        "adam-fst — Kazakh morphology transducer (v1.0.0-alpha)

usage:
  adam-fst synthesise --root <ROOT> [noun|verb flags]
  adam-fst analyse <SURFACE>
  adam-fst lexicon-stats

noun flags (any combination):
  --plural
  --poss <1sg|2sg|3|1pl>
  --case <nom|gen|dat|acc|loc|abl|ins>

verb flags (any combination):
  --voice <active|passive|reflexive|reciprocal|causative>
  --negation
  --tense <past|evidential|present>
  --person <1|2|3>
  --plural
  --polite

examples:
  adam-fst synth --root бала --plural --case dat      → балаларға
  adam-fst synth --root жаз --tense past --person 1   → жаздым
  adam-fst analyse жаздым                              → жаз + past + 1sg
"
    );
}

fn cmd_synthesise(args: &[String]) -> ExitCode {
    let root = match find_flag_value(args, "--root") {
        Some(r) => r,
        None => {
            eprintln!("--root is required");
            return ExitCode::FAILURE;
        }
    };
    let is_verb = args.iter().any(|a| {
        matches!(
            a.as_str(),
            "--tense" | "--voice" | "--negation" | "--polite" | "--person"
        )
    });
    if is_verb {
        let features = parse_verb_flags(args);
        println!("{}", synthesise_verb(&root, features));
    } else {
        let features = parse_noun_flags(args);
        println!("{}", synthesise_noun(&root, features));
    }
    ExitCode::SUCCESS
}

fn parse_noun_flags(args: &[String]) -> NounFeatures {
    let mut f = NounFeatures::default();
    if args.iter().any(|a| a == "--plural") {
        f.number = Some(Number::Plural);
    }
    if let Some(v) = find_flag_value(args, "--poss") {
        f.possessive = Some(match v.as_str() {
            "1sg" => Possessive::P1Sg,
            "2sg" => Possessive::P2SgInformal,
            "3" => Possessive::P3,
            "1pl" => Possessive::P1Pl,
            _ => {
                eprintln!("unknown possessive: {v}");
                std::process::exit(2);
            }
        });
    }
    if let Some(v) = find_flag_value(args, "--case") {
        f.case = Some(match v.as_str() {
            "nom" => Case::Nominative,
            "gen" => Case::Genitive,
            "dat" => Case::Dative,
            "acc" => Case::Accusative,
            "loc" => Case::Locative,
            "abl" => Case::Ablative,
            "ins" => Case::Instrumental,
            _ => {
                eprintln!("unknown case: {v}");
                std::process::exit(2);
            }
        });
    }
    f
}

fn parse_verb_flags(args: &[String]) -> VerbFeatures {
    let mut f = VerbFeatures::default();
    if let Some(v) = find_flag_value(args, "--voice") {
        f.voice = Some(match v.as_str() {
            "active" => Voice::Active,
            "passive" => Voice::Passive,
            "reflexive" => Voice::Reflexive,
            "reciprocal" => Voice::Reciprocal,
            "causative" => Voice::Causative,
            _ => {
                eprintln!("unknown voice: {v}");
                std::process::exit(2);
            }
        });
    }
    f.negation = args.iter().any(|a| a == "--negation");
    if let Some(v) = find_flag_value(args, "--tense") {
        f.tense = Some(match v.as_str() {
            "past" => Tense::PastDefinite,
            "evidential" => Tense::PastEvidential,
            "present" | "aorist" => Tense::Present,
            _ => {
                eprintln!("unknown tense: {v}");
                std::process::exit(2);
            }
        });
    }
    if let Some(v) = find_flag_value(args, "--person") {
        f.person = Some(match v.as_str() {
            "1" | "1sg" | "1pl" => Person::First,
            "2" | "2sg" | "2pl" => Person::Second,
            "3" => Person::Third,
            _ => {
                eprintln!("unknown person: {v}");
                std::process::exit(2);
            }
        });
    }
    if args.iter().any(|a| a == "--plural") {
        f.number = Some(Number::Plural);
    } else if f.person.is_some() {
        f.number = Some(Number::Singular);
    }
    f.polite = args.iter().any(|a| a == "--polite");
    f
}

fn cmd_analyse(args: &[String]) -> ExitCode {
    if args.is_empty() {
        eprintln!("analyse requires a surface form");
        return ExitCode::FAILURE;
    }
    let surface = &args[0];
    let lex = match LexiconV1::load_default() {
        Ok(l) => l,
        Err(e) => {
            eprintln!("could not load lexicon: {e}");
            return ExitCode::FAILURE;
        }
    };
    let analyses = analyse(surface, &lex);
    if analyses.is_empty() {
        println!("no analysis for '{surface}'");
        return ExitCode::SUCCESS;
    }
    for a in &analyses {
        match a {
            Analysis::Noun { root, features } => {
                println!(
                    "noun: {}{}{}{}",
                    root.root,
                    features
                        .number
                        .map(|n| format!(" +{n:?}"))
                        .unwrap_or_default(),
                    features
                        .possessive
                        .map(|p| format!(" +{p:?}"))
                        .unwrap_or_default(),
                    features
                        .case
                        .map(|c| format!(" +{c:?}"))
                        .unwrap_or_default(),
                );
            }
            Analysis::Verb { root, features } => {
                let mut tags: Vec<String> = Vec::new();
                if let Some(v) = features.voice {
                    if !matches!(v, Voice::Active) {
                        tags.push(format!("{v:?}"));
                    }
                }
                if features.negation {
                    tags.push("Negation".to_string());
                }
                if let Some(t) = features.tense {
                    tags.push(format!("{t:?}"));
                }
                if let Some(p) = features.person {
                    tags.push(format!("{p:?}"));
                }
                if let Some(n) = features.number {
                    tags.push(format!("{n:?}"));
                }
                if features.polite {
                    tags.push("Polite".to_string());
                }
                println!(
                    "verb: {} +{}",
                    root.root,
                    if tags.is_empty() {
                        "—".to_string()
                    } else {
                        tags.join("+")
                    }
                );
            }
        }
    }
    ExitCode::SUCCESS
}

fn cmd_stats() -> ExitCode {
    let lex = match LexiconV1::load_default() {
        Ok(l) => l,
        Err(e) => {
            eprintln!("could not load lexicon: {e}");
            return ExitCode::FAILURE;
        }
    };
    let mut by_pos: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for e in lex.by_surface.values() {
        *by_pos.entry(e.part_of_speech.clone()).or_insert(0) += 1;
    }
    println!("adam-fst v1.0.0-alpha lexicon stats");
    println!("  total:    {} entries", lex.total());
    println!("  curated:  {} entries", lex.curated_count);
    println!("  apertium: {} entries", lex.apertium_count);
    println!();
    let mut rows: Vec<(String, usize)> = by_pos.into_iter().collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1));
    for (pos, count) in rows {
        println!("  {pos:<14} {count}");
    }
    ExitCode::SUCCESS
}

fn find_flag_value(args: &[String], name: &str) -> Option<String> {
    for (i, a) in args.iter().enumerate() {
        if a == name {
            return args.get(i + 1).cloned();
        }
    }
    None
}
