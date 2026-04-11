use std::{env, fs, process::ExitCode};

use adam_corpus::{SourceAcceptanceReport, SourceRegistry, SourceScoringRules};
use adam_train::{
    BaselineTrainingManifest, TinyCleanTrainingDomainPack, TinyCleanTrainingManifest,
    assemble_tiny_clean_training_pack, build_tiny_clean_training_report,
};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(manifest_path) = args.next() else {
        eprintln!("usage: tiny_train <training-manifest> <tiny-clean-manifest>");
        return ExitCode::FAILURE;
    };
    let Some(tiny_manifest_path) = args.next() else {
        eprintln!("usage: tiny_train <training-manifest> <tiny-clean-manifest>");
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
    let tiny_manifest: TinyCleanTrainingManifest = match read_json(&tiny_manifest_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read tiny clean training manifest: {error}");
            return ExitCode::FAILURE;
        }
    };
    let manifest_dir = std::path::Path::new(&tiny_manifest_path)
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let mut domain_packs = Vec::new();
    for entry in &tiny_manifest.domain_packs {
        let path = manifest_dir.join(&entry.pack_manifest);
        let pack: TinyCleanTrainingDomainPack = match read_json_path(&path) {
            Ok(value) => value,
            Err(error) => {
                eprintln!(
                    "failed to read tiny clean domain pack {}: {error}",
                    path.display()
                );
                return ExitCode::FAILURE;
            }
        };
        domain_packs.push(pack);
    }
    let pack = match assemble_tiny_clean_training_pack(&tiny_manifest, &domain_packs) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to assemble tiny clean training pack: {error}");
            return ExitCode::FAILURE;
        }
    };

    match build_tiny_clean_training_report(&manifest, &registry, &rules, &report, &pack) {
        Ok(report) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report).expect("tiny training report serializes")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to build tiny clean training report: {error}");
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
    path: &std::path::Path,
) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let value = serde_json::from_str(&contents)?;
    Ok(value)
}
