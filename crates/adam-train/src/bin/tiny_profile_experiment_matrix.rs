use std::{env, fs, path::Path, process::ExitCode};

use adam_corpus::{SourceAcceptanceReport, SourceRegistry, SourceScoringRules};
use adam_train::{
    BaselineTrainingManifest, CleanTrainingCorpusManifest, CleanTrainingCorpusPack,
    TinyCleanTrainingProfileExperimentMatrixManifest, TinyCleanTrainingProfilePromotionReport,
    TinyCleanTrainingProfileStrategyReport, TinyCleanTrainingProfileSuiteManifest,
    build_tiny_clean_training_profile_experiment_matrix_report,
};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(training_manifest_path) = args.next() else {
        eprintln!(
            "usage: tiny_profile_experiment_matrix <training-manifest> <profile-suite-manifest> <experiment-matrix-manifest> <strategy-report> <promotion-report>"
        );
        return ExitCode::FAILURE;
    };
    let Some(profile_suite_path) = args.next() else {
        eprintln!(
            "usage: tiny_profile_experiment_matrix <training-manifest> <profile-suite-manifest> <experiment-matrix-manifest> <strategy-report> <promotion-report>"
        );
        return ExitCode::FAILURE;
    };
    let Some(matrix_manifest_path) = args.next() else {
        eprintln!(
            "usage: tiny_profile_experiment_matrix <training-manifest> <profile-suite-manifest> <experiment-matrix-manifest> <strategy-report> <promotion-report>"
        );
        return ExitCode::FAILURE;
    };
    let Some(strategy_report_path) = args.next() else {
        eprintln!(
            "usage: tiny_profile_experiment_matrix <training-manifest> <profile-suite-manifest> <experiment-matrix-manifest> <strategy-report> <promotion-report>"
        );
        return ExitCode::FAILURE;
    };
    let Some(promotion_report_path) = args.next() else {
        eprintln!(
            "usage: tiny_profile_experiment_matrix <training-manifest> <profile-suite-manifest> <experiment-matrix-manifest> <strategy-report> <promotion-report>"
        );
        return ExitCode::FAILURE;
    };

    let training_manifest: BaselineTrainingManifest = match read_json(&training_manifest_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read training manifest: {error}");
            return ExitCode::FAILURE;
        }
    };
    let registry: SourceRegistry = match read_json(&training_manifest.source_registry_manifest) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read source registry: {error}");
            return ExitCode::FAILURE;
        }
    };
    let rules: SourceScoringRules = match read_json(&training_manifest.scoring_rules_manifest) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read scoring rules: {error}");
            return ExitCode::FAILURE;
        }
    };
    let acceptance_report: SourceAcceptanceReport =
        match read_json(&training_manifest.acceptance_report_manifest) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("failed to read acceptance report: {error}");
                return ExitCode::FAILURE;
            }
        };
    let suite: TinyCleanTrainingProfileSuiteManifest = match read_json(&profile_suite_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read tiny profile suite manifest: {error}");
            return ExitCode::FAILURE;
        }
    };
    let matrix_manifest: TinyCleanTrainingProfileExperimentMatrixManifest =
        match read_json(&matrix_manifest_path) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("failed to read tiny profile experiment matrix manifest: {error}");
                return ExitCode::FAILURE;
            }
        };
    let strategy_report: TinyCleanTrainingProfileStrategyReport =
        match read_json(&strategy_report_path) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("failed to read tiny profile strategy report: {error}");
                return ExitCode::FAILURE;
            }
        };
    let promotion_report: TinyCleanTrainingProfilePromotionReport =
        match read_json(&promotion_report_path) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("failed to read tiny profile promotion report: {error}");
                return ExitCode::FAILURE;
            }
        };

    let suite_dir = Path::new(&profile_suite_path)
        .parent()
        .unwrap_or_else(|| Path::new("."));
    let clean_corpus_manifest_path = suite_dir.join(&suite.source_clean_corpus_manifest);
    let clean_corpus_manifest: CleanTrainingCorpusManifest =
        match read_json_path(&clean_corpus_manifest_path) {
            Ok(value) => value,
            Err(error) => {
                eprintln!(
                    "failed to read clean corpus manifest {}: {error}",
                    clean_corpus_manifest_path.display()
                );
                return ExitCode::FAILURE;
            }
        };
    let clean_corpus_pack_path = suite_dir.join(&suite.source_clean_corpus_pack);
    let clean_corpus_pack: CleanTrainingCorpusPack = match read_json_path(&clean_corpus_pack_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!(
                "failed to read clean corpus pack {}: {error}",
                clean_corpus_pack_path.display()
            );
            return ExitCode::FAILURE;
        }
    };

    match build_tiny_clean_training_profile_experiment_matrix_report(
        &matrix_manifest,
        &training_manifest,
        &registry,
        &rules,
        &acceptance_report,
        &suite,
        &strategy_report,
        &promotion_report,
        &clean_corpus_manifest,
        &clean_corpus_pack,
    ) {
        Ok(report) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report)
                    .expect("tiny profile experiment matrix report serializes")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to build tiny profile experiment matrix report: {error}");
            ExitCode::FAILURE
        }
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let value = serde_json::from_str(&contents)?;
    Ok(value)
}

fn read_json_path<T: serde::de::DeserializeOwned>(
    path: &Path,
) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let value = serde_json::from_str(&contents)?;
    Ok(value)
}
