use std::{env, fs, process::ExitCode};

use adam_train::{
    TinyCleanTrainingProfileBaselineDeltaReport, TinyCleanTrainingProfileBaselineReport,
    build_tiny_clean_training_profile_baseline_delta_report,
};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(expected_report_path) = args.next() else {
        eprintln!(
            "usage: tiny_profile_baseline_delta <expected-baseline-report> <actual-baseline-report>"
        );
        return ExitCode::FAILURE;
    };
    let Some(actual_report_path) = args.next() else {
        eprintln!(
            "usage: tiny_profile_baseline_delta <expected-baseline-report> <actual-baseline-report>"
        );
        return ExitCode::FAILURE;
    };

    let expected: TinyCleanTrainingProfileBaselineReport = match read_json(&expected_report_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read expected tiny profile baseline report: {error}");
            return ExitCode::FAILURE;
        }
    };
    let actual: TinyCleanTrainingProfileBaselineReport = match read_json(&actual_report_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read actual tiny profile baseline report: {error}");
            return ExitCode::FAILURE;
        }
    };

    let report: TinyCleanTrainingProfileBaselineDeltaReport =
        build_tiny_clean_training_profile_baseline_delta_report(&expected, &actual);
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .expect("tiny profile baseline delta report serializes")
    );
    ExitCode::SUCCESS
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let value = serde_json::from_str(&contents)?;
    Ok(value)
}
