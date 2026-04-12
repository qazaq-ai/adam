use std::{env, fs, process::ExitCode};

use adam_train::{
    TinyCleanTrainingProfileExperimentMatrixPolicyReport,
    TinyCleanTrainingProfilePromotionManifest, build_tiny_clean_training_profile_promotion_report,
};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(promotion_manifest_path) = args.next() else {
        eprintln!("usage: tiny_profile_promotion <promotion-manifest> <matrix-policy-report>");
        return ExitCode::FAILURE;
    };
    let Some(matrix_policy_report_path) = args.next() else {
        eprintln!("usage: tiny_profile_promotion <promotion-manifest> <matrix-policy-report>");
        return ExitCode::FAILURE;
    };

    let promotion_manifest: TinyCleanTrainingProfilePromotionManifest =
        match read_json(&promotion_manifest_path) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("failed to read tiny profile promotion manifest: {error}");
                return ExitCode::FAILURE;
            }
        };
    let matrix_policy_report: TinyCleanTrainingProfileExperimentMatrixPolicyReport =
        match read_json(&matrix_policy_report_path) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("failed to read tiny profile experiment matrix policy report: {error}");
                return ExitCode::FAILURE;
            }
        };

    match build_tiny_clean_training_profile_promotion_report(
        &promotion_manifest,
        &matrix_policy_report,
    ) {
        Ok(report) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report)
                    .expect("tiny profile promotion report serializes")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to build tiny profile promotion report: {error}");
            ExitCode::FAILURE
        }
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let value = serde_json::from_str(&contents)?;
    Ok(value)
}
