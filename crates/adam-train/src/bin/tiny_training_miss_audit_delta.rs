use std::{env, fs, process::ExitCode};

use adam_train::{
    TinyCleanTrainingMissAuditDeltaReport, TinyCleanTrainingMissAuditReport,
    build_tiny_clean_training_miss_audit_delta_report,
};

const USAGE: &str = "usage: tiny_training_miss_audit_delta <expected-report> <actual-report>";

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(expected_path) = args.next() else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let Some(actual_path) = args.next() else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };

    let expected: TinyCleanTrainingMissAuditReport = match read_json(&expected_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read expected miss audit report: {error}");
            return ExitCode::FAILURE;
        }
    };
    let actual: TinyCleanTrainingMissAuditReport = match read_json(&actual_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read actual miss audit report: {error}");
            return ExitCode::FAILURE;
        }
    };

    let delta: TinyCleanTrainingMissAuditDeltaReport =
        build_tiny_clean_training_miss_audit_delta_report(&expected, &actual);
    println!(
        "{}",
        serde_json::to_string_pretty(&delta)
            .expect("tiny clean training miss audit delta serializes")
    );
    ExitCode::SUCCESS
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let value = serde_json::from_str(&contents)?;
    Ok(value)
}
