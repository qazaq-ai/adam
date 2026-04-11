use std::{env, fs, process::ExitCode};

use adam_train::{
    FoundationOverviewDeltaReport, FoundationOverviewReport, build_foundation_overview_delta_report,
};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(expected_path) = args.next() else {
        eprintln!("usage: foundation_delta <expected-overview-report> <actual-overview-report>");
        return ExitCode::FAILURE;
    };
    let Some(actual_path) = args.next() else {
        eprintln!("usage: foundation_delta <expected-overview-report> <actual-overview-report>");
        return ExitCode::FAILURE;
    };

    let expected: FoundationOverviewReport = match read_json(&expected_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read expected foundation overview report: {error}");
            return ExitCode::FAILURE;
        }
    };
    let actual: FoundationOverviewReport = match read_json(&actual_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read actual foundation overview report: {error}");
            return ExitCode::FAILURE;
        }
    };

    let report: FoundationOverviewDeltaReport =
        build_foundation_overview_delta_report(&expected, &actual);
    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("foundation delta serializes")
    );
    ExitCode::SUCCESS
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let value = serde_json::from_str(&contents)?;
    Ok(value)
}
