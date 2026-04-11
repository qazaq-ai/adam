use std::{env, fs, path::Path, process::ExitCode};

use adam_train::{
    CleanTrainingCorpusManifest, TinyCleanTrainingDomainPack, assemble_clean_training_corpus_pack,
};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(manifest_path) = args.next() else {
        eprintln!("usage: assemble_clean_corpus <clean-corpus-manifest>");
        return ExitCode::FAILURE;
    };

    let manifest: CleanTrainingCorpusManifest = match read_json(&manifest_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read clean corpus manifest: {error}");
            return ExitCode::FAILURE;
        }
    };

    let manifest_dir = Path::new(&manifest_path)
        .parent()
        .unwrap_or_else(|| Path::new("."));
    let mut packs = Vec::new();
    for pack_path in &manifest.pack_manifests {
        let path = manifest_dir.join(pack_path);
        let pack: TinyCleanTrainingDomainPack = match read_json_path(&path) {
            Ok(value) => value,
            Err(error) => {
                eprintln!(
                    "failed to read clean corpus pack {}: {error}",
                    path.display()
                );
                return ExitCode::FAILURE;
            }
        };
        packs.push(pack);
    }

    match assemble_clean_training_corpus_pack(&manifest, &packs) {
        Ok(pack) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&pack).expect("clean corpus pack serializes")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to assemble clean corpus pack: {error}");
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
