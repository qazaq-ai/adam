use std::{env, fs, path::Path, process::ExitCode};

use adam_train::{
    CleanTrainingCorpusManifest, CleanTrainingCorpusPack, TinyCleanTrainingSelectionManifest,
    assemble_tiny_clean_training_pack_from_corpus,
};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(selection_manifest_path) = args.next() else {
        eprintln!(
            "usage: assemble_tiny_clean_training <selection-manifest> <clean-corpus-manifest> <clean-corpus-pack>"
        );
        return ExitCode::FAILURE;
    };
    let Some(clean_corpus_manifest_path) = args.next() else {
        eprintln!(
            "usage: assemble_tiny_clean_training <selection-manifest> <clean-corpus-manifest> <clean-corpus-pack>"
        );
        return ExitCode::FAILURE;
    };
    let Some(clean_corpus_pack_path) = args.next() else {
        eprintln!(
            "usage: assemble_tiny_clean_training <selection-manifest> <clean-corpus-manifest> <clean-corpus-pack>"
        );
        return ExitCode::FAILURE;
    };

    let selection_manifest: TinyCleanTrainingSelectionManifest =
        match read_json(&selection_manifest_path) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("failed to read tiny training selection manifest: {error}");
                return ExitCode::FAILURE;
            }
        };
    let clean_corpus_manifest: CleanTrainingCorpusManifest =
        match read_json(&clean_corpus_manifest_path) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("failed to read clean corpus manifest: {error}");
                return ExitCode::FAILURE;
            }
        };
    let clean_corpus_pack: CleanTrainingCorpusPack = match read_json(&clean_corpus_pack_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read clean corpus pack: {error}");
            return ExitCode::FAILURE;
        }
    };

    match assemble_tiny_clean_training_pack_from_corpus(
        &selection_manifest,
        &clean_corpus_manifest,
        &clean_corpus_pack,
    ) {
        Ok(pack) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&pack).expect("tiny clean training pack serializes")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to assemble tiny clean training pack: {error}");
            ExitCode::FAILURE
        }
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(Path::new(path))?;
    let value = serde_json::from_str(&contents)?;
    Ok(value)
}
