use std::{env, fs, process::ExitCode};

use adam_train::{
    TinyCleanTrainingProfileExperimentMatrixPolicyManifest,
    TinyCleanTrainingProfileExperimentMatrixReport,
    build_tiny_clean_training_profile_experiment_matrix_policy_report,
};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(policy_manifest_path) = args.next() else {
        eprintln!("usage: tiny_profile_experiment_matrix_policy <policy-manifest> <matrix-report>");
        return ExitCode::FAILURE;
    };
    let Some(matrix_report_path) = args.next() else {
        eprintln!("usage: tiny_profile_experiment_matrix_policy <policy-manifest> <matrix-report>");
        return ExitCode::FAILURE;
    };

    let policy_manifest: TinyCleanTrainingProfileExperimentMatrixPolicyManifest =
        match read_json(&policy_manifest_path) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("failed to read tiny profile experiment matrix policy manifest: {error}");
                return ExitCode::FAILURE;
            }
        };
    let matrix_report: TinyCleanTrainingProfileExperimentMatrixReport =
        match read_json(&matrix_report_path) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("failed to read tiny profile experiment matrix report: {error}");
                return ExitCode::FAILURE;
            }
        };

    match build_tiny_clean_training_profile_experiment_matrix_policy_report(
        &policy_manifest,
        &matrix_report,
    ) {
        Ok(report) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report)
                    .expect("tiny profile experiment matrix policy report serializes")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to build tiny profile experiment matrix policy report: {error}");
            ExitCode::FAILURE
        }
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let value = serde_json::from_str(&contents)?;
    Ok(value)
}
