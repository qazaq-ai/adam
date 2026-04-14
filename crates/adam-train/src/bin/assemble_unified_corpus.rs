use std::{collections::HashSet, env, fs, path::Path, process::ExitCode};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct Manifest {
    version: String,
    name: String,
    target_language: String,
    script: String,
    pack_manifests: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct InputPack {
    #[serde(rename = "name")]
    #[allow(dead_code)]
    _name: Option<String>,
    samples: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct UnifiedPack {
    version: String,
    name: String,
    target_language: String,
    script: String,
    samples: Vec<serde_json::Value>,
}

fn main() -> ExitCode {
    let Some(manifest_path) = env::args().nth(1) else {
        eprintln!("usage: assemble_unified_corpus <manifest-path>");
        return ExitCode::FAILURE;
    };

    let manifest_raw = match fs::read_to_string(&manifest_path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("cannot read manifest: {e}");
            return ExitCode::FAILURE;
        }
    };
    let manifest: Manifest = match serde_json::from_str(&manifest_raw) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("invalid manifest: {e}");
            return ExitCode::FAILURE;
        }
    };

    let manifest_dir = Path::new(&manifest_path).parent().unwrap_or(Path::new("."));
    let mut seen_texts: HashSet<String> = HashSet::new();
    let mut samples: Vec<serde_json::Value> = Vec::new();
    let mut dupes = 0usize;

    for pack_name in &manifest.pack_manifests {
        let pack_path = manifest_dir.join(pack_name);
        let pack_raw = match fs::read_to_string(&pack_path) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("cannot read pack {}: {}", pack_path.display(), e);
                return ExitCode::FAILURE;
            }
        };
        let pack: InputPack = match serde_json::from_str(&pack_raw) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("invalid pack {}: {}", pack_path.display(), e);
                return ExitCode::FAILURE;
            }
        };
        for sample in pack.samples {
            let text = sample
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if text.is_empty() {
                continue;
            }
            if seen_texts.insert(text) {
                samples.push(sample);
            } else {
                dupes += 1;
            }
        }
    }

    // Renumber ids to the unified id scheme
    for (idx, sample) in samples.iter_mut().enumerate() {
        if let Some(obj) = sample.as_object_mut() {
            obj.insert(
                "id".to_string(),
                serde_json::Value::String(format!("adam_train_{:06}", idx + 1)),
            );
        }
    }

    let unified = UnifiedPack {
        version: manifest.version,
        name: manifest.name,
        target_language: manifest.target_language,
        script: manifest.script,
        samples,
    };

    eprintln!(
        "assembled {} unique samples ({} duplicates removed)",
        unified.samples.len(),
        dupes
    );
    println!(
        "{}",
        serde_json::to_string_pretty(&unified).expect("serialize")
    );
    ExitCode::SUCCESS
}
