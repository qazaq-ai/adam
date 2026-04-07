use std::{env, fs, process::ExitCode};

use adam_corpus::{
    SourceRegistry, SourceScoringRules, build_source_acceptance_report,
    default_source_acceptance_report_name,
};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(registry_path) = args.next() else {
        eprintln!("usage: generate_acceptance_report <registry> <scoring_rules> [output]");
        return ExitCode::FAILURE;
    };
    let Some(scoring_rules_path) = args.next() else {
        eprintln!("usage: generate_acceptance_report <registry> <scoring_rules> [output]");
        return ExitCode::FAILURE;
    };
    let output_path = args.next();

    let registry: SourceRegistry = match read_json(&registry_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read source registry: {error}");
            return ExitCode::FAILURE;
        }
    };
    let rules: SourceScoringRules = match read_json(&scoring_rules_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read scoring rules: {error}");
            return ExitCode::FAILURE;
        }
    };

    let report = match build_source_acceptance_report(
        &default_source_acceptance_report_name(&registry),
        &registry_path,
        &scoring_rules_path,
        &registry,
        &rules,
    ) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to build source acceptance report: {error}");
            return ExitCode::FAILURE;
        }
    };

    let rendered = serde_json::to_string_pretty(&report).expect("report serializes");
    if let Some(path) = output_path {
        if let Err(error) = fs::write(&path, format!("{rendered}\n")) {
            eprintln!("failed to write acceptance report: {error}");
            return ExitCode::FAILURE;
        }
    } else {
        println!("{rendered}");
    }

    ExitCode::SUCCESS
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let value = serde_json::from_str(&contents)?;
    Ok(value)
}
