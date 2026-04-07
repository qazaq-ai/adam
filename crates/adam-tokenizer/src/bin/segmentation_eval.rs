use std::{env, fs, process::ExitCode};

use adam_tokenizer::{
    SegmentationLexicon, SegmentationRuleSet, TokenizerSegmentationDataset,
    build_segmentation_report,
};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(dataset_path) = args.next() else {
        eprintln!("usage: segmentation_eval <dataset-manifest> <roots-manifest> <rules-manifest>");
        return ExitCode::FAILURE;
    };
    let Some(roots_path) = args.next() else {
        eprintln!("usage: segmentation_eval <dataset-manifest> <roots-manifest> <rules-manifest>");
        return ExitCode::FAILURE;
    };
    let Some(rules_path) = args.next() else {
        eprintln!("usage: segmentation_eval <dataset-manifest> <roots-manifest> <rules-manifest>");
        return ExitCode::FAILURE;
    };

    let dataset: TokenizerSegmentationDataset = match read_json(&dataset_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read segmentation dataset: {error}");
            return ExitCode::FAILURE;
        }
    };
    let lexicon: SegmentationLexicon = match read_json(&roots_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read segmentation roots: {error}");
            return ExitCode::FAILURE;
        }
    };
    let rules: SegmentationRuleSet = match read_json(&rules_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read segmentation rules: {error}");
            return ExitCode::FAILURE;
        }
    };

    match build_segmentation_report(&dataset, &lexicon, &rules) {
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
