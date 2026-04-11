use std::{env, fs, process::ExitCode};

use adam_train::{
    TinyCleanTrainingProfileSuiteReport, build_tiny_clean_training_profile_comparison_report,
};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(suite_report_path) = args.next() else {
        eprintln!("usage: tiny_profile_comparison <profile-suite-report>");
        return ExitCode::FAILURE;
    };

    let suite_report: TinyCleanTrainingProfileSuiteReport = match read_json(&suite_report_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read tiny profile suite report: {error}");
            return ExitCode::FAILURE;
        }
    };

    match build_tiny_clean_training_profile_comparison_report(&suite_report) {
        Ok(report) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report)
                    .expect("tiny profile comparison report serializes")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to build tiny profile comparison report: {error}");
            ExitCode::FAILURE
        }
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let value = serde_json::from_str(&contents)?;
    Ok(value)
}
