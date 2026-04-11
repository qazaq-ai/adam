use std::{env, fs, process::ExitCode};

use adam_corpus::{
    SourceAcceptanceReport, SourceAcceptanceSummaryReport, SourceRegistry, SourceScoringRules,
    build_source_acceptance_delta_report,
};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(report_path) = args.next() else {
        eprintln!(
            "usage: delta <source-acceptance-report> <source-registry> <scoring-rules> <expected-summary>"
        );
        return ExitCode::FAILURE;
    };
    let Some(registry_path) = args.next() else {
        eprintln!(
            "usage: delta <source-acceptance-report> <source-registry> <scoring-rules> <expected-summary>"
        );
        return ExitCode::FAILURE;
    };
    let Some(rules_path) = args.next() else {
        eprintln!(
            "usage: delta <source-acceptance-report> <source-registry> <scoring-rules> <expected-summary>"
        );
        return ExitCode::FAILURE;
    };
    let Some(expected_path) = args.next() else {
        eprintln!(
            "usage: delta <source-acceptance-report> <source-registry> <scoring-rules> <expected-summary>"
        );
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
    let expected: SourceAcceptanceSummaryReport = match read_json(&expected_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read expected source acceptance summary: {error}");
            return ExitCode::FAILURE;
        }
    };

    match build_source_acceptance_delta_report(&report, &registry, &rules, &expected) {
        Ok(delta) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&delta).expect("delta serializes")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to build source acceptance delta report: {error}");
            ExitCode::FAILURE
        }
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let value = serde_json::from_str(&contents)?;
    Ok(value)
}
