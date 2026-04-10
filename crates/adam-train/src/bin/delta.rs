use std::{env, fs, process::ExitCode};

use adam_corpus::{CorpusManifest, SourceAcceptanceReport, SourceRegistry, SourceScoringRules};
use adam_eval::EvalSuite;
use adam_tokenizer::TokenizerExperiment;
use adam_train::{
    BaselineTrainingAssemblyReport, BaselineTrainingConsistencyReport, BaselineTrainingManifest,
    build_baseline_training_delta_report,
};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(manifest_path) = args.next() else {
        eprintln!(
            "usage: delta <training-manifest> <expected-assembly-report> <expected-consistency-report>"
        );
        return ExitCode::FAILURE;
    };
    let Some(expected_assembly_path) = args.next() else {
        eprintln!(
            "usage: delta <training-manifest> <expected-assembly-report> <expected-consistency-report>"
        );
        return ExitCode::FAILURE;
    };
    let Some(expected_consistency_path) = args.next() else {
        eprintln!(
            "usage: delta <training-manifest> <expected-assembly-report> <expected-consistency-report>"
        );
        return ExitCode::FAILURE;
    };

    let manifest: BaselineTrainingManifest = match read_json(&manifest_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read training manifest: {error}");
            return ExitCode::FAILURE;
        }
    };
    let corpus: CorpusManifest = match read_json(&manifest.corpus_manifest) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read corpus manifest: {error}");
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
    let expected_assembly: BaselineTrainingAssemblyReport = match read_json(&expected_assembly_path)
    {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read expected assembly report: {error}");
            return ExitCode::FAILURE;
        }
    };
    let expected_consistency: BaselineTrainingConsistencyReport =
        match read_json(&expected_consistency_path) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("failed to read expected consistency report: {error}");
                return ExitCode::FAILURE;
            }
        };

    match build_baseline_training_delta_report(
        &manifest,
        &corpus,
        &registry,
        &rules,
        &report,
        &tokenizer_experiment,
        &eval_suite,
        &expected_assembly,
        &expected_consistency,
    ) {
        Ok(report) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report).expect("delta report serializes")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to build baseline training delta report: {error}");
            ExitCode::FAILURE
        }
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let value = serde_json::from_str(&contents)?;
    Ok(value)
}
