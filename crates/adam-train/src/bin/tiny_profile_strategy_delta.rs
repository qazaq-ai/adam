use std::{env, fs, process::ExitCode};

use adam_train::{
    TinyCleanTrainingProfileStrategyDeltaReport, TinyCleanTrainingProfileStrategyReport,
    build_tiny_clean_training_profile_strategy_delta_report,
};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(expected_report_path) = args.next() else {
        eprintln!(
            "usage: tiny_profile_strategy_delta <expected-strategy-report> <actual-strategy-report>"
        );
        return ExitCode::FAILURE;
    };
    let Some(actual_report_path) = args.next() else {
        eprintln!(
            "usage: tiny_profile_strategy_delta <expected-strategy-report> <actual-strategy-report>"
        );
        return ExitCode::FAILURE;
    };

    let expected: TinyCleanTrainingProfileStrategyReport = match read_json(&expected_report_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read expected tiny profile strategy report: {error}");
            return ExitCode::FAILURE;
        }
    };
    let actual: TinyCleanTrainingProfileStrategyReport = match read_json(&actual_report_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read actual tiny profile strategy report: {error}");
            return ExitCode::FAILURE;
        }
    };

    let report: TinyCleanTrainingProfileStrategyDeltaReport =
        build_tiny_clean_training_profile_strategy_delta_report(&expected, &actual);
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .expect("tiny profile strategy delta report serializes")
    );
    ExitCode::SUCCESS
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let value = serde_json::from_str(&contents)?;
    Ok(value)
}
