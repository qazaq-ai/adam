use std::{env, fs, process::ExitCode};

use adam_corpus::{
    SourceAcceptanceReport, SourceRegistry, SourceScoringRules,
    build_source_acceptance_summary_report,
};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(report_path) = args.next() else {
        eprintln!("usage: report <source-acceptance-report> <source-registry> <scoring-rules>");
        return ExitCode::FAILURE;
    };
    let Some(registry_path) = args.next() else {
        eprintln!("usage: report <source-acceptance-report> <source-registry> <scoring-rules>");
        return ExitCode::FAILURE;
    };
    let Some(rules_path) = args.next() else {
        eprintln!("usage: report <source-acceptance-report> <source-registry> <scoring-rules>");
        return ExitCode::FAILURE;
    };

    let report: SourceAcceptanceReport = match read_json(&report_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read source acceptance report: {error}");
            return ExitCode::FAILURE;
        }
    };
    let registry: SourceRegistry = match read_json(&registry_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read source registry: {error}");
            return ExitCode::FAILURE;
        }
    };
    let rules: SourceScoringRules = match read_json(&rules_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read scoring rules: {error}");
            return ExitCode::FAILURE;
        }
    };

    match build_source_acceptance_summary_report(&report, &registry, &rules) {
        Ok(summary) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&summary).expect("summary serializes")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to build source acceptance summary report: {error}");
            ExitCode::FAILURE
        }
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let value = serde_json::from_str(&contents)?;
    Ok(value)
}
