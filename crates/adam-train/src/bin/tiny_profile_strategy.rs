use std::{env, fs, process::ExitCode};

use adam_train::{
    TinyCleanTrainingProfileBaselineReport, TinyCleanTrainingProfileComparisonReport,
    TinyCleanTrainingProfileStrategyManifest, build_tiny_clean_training_profile_strategy_report,
};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(strategy_manifest_path) = args.next() else {
        eprintln!(
            "usage: tiny_profile_strategy <strategy-manifest> <baseline-report> <comparison-report>"
        );
        return ExitCode::FAILURE;
    };
    let Some(baseline_report_path) = args.next() else {
        eprintln!(
            "usage: tiny_profile_strategy <strategy-manifest> <baseline-report> <comparison-report>"
        );
        return ExitCode::FAILURE;
    };
    let Some(comparison_report_path) = args.next() else {
        eprintln!(
            "usage: tiny_profile_strategy <strategy-manifest> <baseline-report> <comparison-report>"
        );
        return ExitCode::FAILURE;
    };

    let strategy_manifest: TinyCleanTrainingProfileStrategyManifest =
        match read_json(&strategy_manifest_path) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("failed to read tiny profile strategy manifest: {error}");
                return ExitCode::FAILURE;
            }
        };
    let baseline_report: TinyCleanTrainingProfileBaselineReport =
        match read_json(&baseline_report_path) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("failed to read tiny profile baseline report: {error}");
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

    match build_tiny_clean_training_profile_strategy_report(
        &strategy_manifest,
        &baseline_report,
        &comparison_report,
    ) {
        Ok(report) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report)
                    .expect("tiny profile strategy report serializes")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to build tiny profile strategy report: {error}");
            ExitCode::FAILURE
        }
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let value = serde_json::from_str(&contents)?;
    Ok(value)
}
