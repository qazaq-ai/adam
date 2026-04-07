use std::{env, fs, process::ExitCode};

use adam_corpus::{SourceAcceptanceReport, SourceRegistry, SourceScoringRules};
use adam_eval::EvalSuite;
use adam_tokenizer::TokenizerExperiment;
use adam_train::{BaselineTrainingManifest, build_baseline_training_plan};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(manifest_path) = args.next() else {
        eprintln!("usage: plan <training-manifest>");
        return ExitCode::FAILURE;
    };

    let manifest: BaselineTrainingManifest = match read_json(&manifest_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read training manifest: {error}");
            return ExitCode::FAILURE;
        }
    };
    let registry: SourceRegistry = match read_json(&manifest.source_registry_manifest) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read source registry: {error}");
            return ExitCode::FAILURE;
        }
    };
    let rules: SourceScoringRules = match read_json(&manifest.scoring_rules_manifest) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read scoring rules: {error}");
            return ExitCode::FAILURE;
        }
    };
    let report: SourceAcceptanceReport = match read_json(&manifest.acceptance_report_manifest) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read acceptance report: {error}");
            return ExitCode::FAILURE;
        }
    };
    let tokenizer_experiment: TokenizerExperiment =
        match read_json(&manifest.tokenizer_experiment_manifest) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("failed to read tokenizer experiment: {error}");
                return ExitCode::FAILURE;
            }
        };
    let eval_suite: EvalSuite = match read_json(&manifest.eval_suite_manifest) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read eval suite: {error}");
            return ExitCode::FAILURE;
        }
    };

    match build_baseline_training_plan(
        &manifest,
        &registry,
        &rules,
        &report,
        &tokenizer_experiment,
        &eval_suite,
    ) {
        Ok(plan) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&plan).expect("plan serializes")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to build baseline training plan: {error}");
            ExitCode::FAILURE
        }
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let value = serde_json::from_str(&contents)?;
    Ok(value)
}
