use std::{env, fs, process::ExitCode};

use adam_train::{
    TinyCleanTrainingProfileBaselineManifest, TinyCleanTrainingProfileComparisonReport,
    build_tiny_clean_training_profile_baseline_report,
};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(policy_manifest_path) = args.next() else {
        eprintln!("usage: tiny_profile_baseline <policy-manifest> <profile-comparison-report>");
        return ExitCode::FAILURE;
    };
    let Some(comparison_report_path) = args.next() else {
        eprintln!("usage: tiny_profile_baseline <policy-manifest> <profile-comparison-report>");
        return ExitCode::FAILURE;
    };

    let policy_manifest: TinyCleanTrainingProfileBaselineManifest =
        match read_json(&policy_manifest_path) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("failed to read tiny profile baseline manifest: {error}");
                return ExitCode::FAILURE;
            }
        };
    let comparison_report: TinyCleanTrainingProfileComparisonReport =
        match read_json(&comparison_report_path) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("failed to read tiny profile comparison report: {error}");
                return ExitCode::FAILURE;
            }
        };

    match build_tiny_clean_training_profile_baseline_report(&policy_manifest, &comparison_report) {
        Ok(report) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report)
                    .expect("tiny profile baseline report serializes")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to build tiny profile baseline report: {error}");
            ExitCode::FAILURE
        }
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let value = serde_json::from_str(&contents)?;
    Ok(value)
}
