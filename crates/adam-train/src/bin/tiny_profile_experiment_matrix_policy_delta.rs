use std::{env, fs, process::ExitCode};

use adam_train::{
    TinyCleanTrainingProfileExperimentMatrixPolicyDeltaReport,
    TinyCleanTrainingProfileExperimentMatrixPolicyReport,
    build_tiny_clean_training_profile_experiment_matrix_policy_delta_report,
};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(expected_path) = args.next() else {
        eprintln!(
            "usage: tiny_profile_experiment_matrix_policy_delta <expected-policy-report> <actual-policy-report>"
        );
        return ExitCode::FAILURE;
    };
    let Some(actual_path) = args.next() else {
        eprintln!(
            "usage: tiny_profile_experiment_matrix_policy_delta <expected-policy-report> <actual-policy-report>"
        );
        return ExitCode::FAILURE;
    };

    let expected: TinyCleanTrainingProfileExperimentMatrixPolicyReport =
        match read_json(&expected_path) {
            Ok(value) => value,
            Err(error) => {
                eprintln!(
                    "failed to read expected tiny profile experiment matrix policy report: {error}"
                );
                return ExitCode::FAILURE;
            }
        };
    let actual: TinyCleanTrainingProfileExperimentMatrixPolicyReport = match read_json(&actual_path)
    {
        Ok(value) => value,
        Err(error) => {
            eprintln!(
                "failed to read actual tiny profile experiment matrix policy report: {error}"
            );
            return ExitCode::FAILURE;
        }
    };

    let report: TinyCleanTrainingProfileExperimentMatrixPolicyDeltaReport =
        build_tiny_clean_training_profile_experiment_matrix_policy_delta_report(&expected, &actual);
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .expect("tiny profile experiment matrix policy delta report serializes")
    );
    ExitCode::SUCCESS
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let value = serde_json::from_str(&contents)?;
    Ok(value)
}
