use std::{env, fs, process::ExitCode};

use adam_tokenizer::{
    TokenizerDryRunPack, TokenizerExperiment, TokenizerSegmentationDataset, build_experiment_report,
};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(experiment_path) = args.next() else {
        eprintln!("usage: run_experiment <experiment-manifest>");
        return ExitCode::FAILURE;
    };

    let experiment: TokenizerExperiment = match read_json(&experiment_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read tokenizer experiment: {error}");
            return ExitCode::FAILURE;
        }
    };
    let pack: TokenizerDryRunPack = match read_json(&experiment.sample_pack_manifest) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read tokenizer dry-run pack: {error}");
            return ExitCode::FAILURE;
        }
    };
    let dataset: TokenizerSegmentationDataset =
        match read_json(&experiment.segmentation_eval_manifest) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("failed to read segmentation eval dataset: {error}");
                return ExitCode::FAILURE;
            }
        };

    match build_experiment_report(&experiment, &pack, &dataset) {
        Ok(report) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report).expect("report serializes")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to build tokenizer experiment report: {error}");
            ExitCode::FAILURE
        }
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let value = serde_json::from_str(&contents)?;
    Ok(value)
}
