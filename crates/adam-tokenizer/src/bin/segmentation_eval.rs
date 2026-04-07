use std::{env, fs, process::ExitCode};

use adam_tokenizer::{TokenizerSegmentationDataset, build_segmentation_report};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(dataset_path) = args.next() else {
        eprintln!("usage: segmentation_eval <dataset-manifest>");
        return ExitCode::FAILURE;
    };

    let dataset: TokenizerSegmentationDataset = match read_json(&dataset_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read segmentation dataset: {error}");
            return ExitCode::FAILURE;
        }
    };

    match build_segmentation_report(&dataset) {
        Ok(report) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report).expect("report serializes")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to evaluate segmentation dataset: {error}");
            ExitCode::FAILURE
        }
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let value = serde_json::from_str(&contents)?;
    Ok(value)
}
