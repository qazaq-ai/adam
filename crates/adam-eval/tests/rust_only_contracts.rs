use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn forbidden_source_extensions() -> &'static [&'static str] {
    &[
        "py", "pyw", "js", "mjs", "cjs", "ts", "tsx", "jsx", "java", "go", "rb", "php", "pl",
        "lua", "jl", "r", "R", "scala", "kt", "swift", "cpp", "cc", "cxx", "c", "h", "hpp",
    ]
}

fn is_ignored_dir(name: &OsStr) -> bool {
    matches!(
        name.to_str(),
        Some(".git")
            | Some("target")
            | Some("node_modules")
            // **2026-06-02** — Piper TTS (v6.3 voice REPL) and the
            // Whisper-model conversion tooling under
            // `data/stt_models/_convert/` both create Python
            // virtualenvs on a dev machine (`.venv` / `venv`). The
            // venvs vendor PyTorch + KleidiAI headers (~1 GB of
            // `.h` files and `.py` modules) which are NOT in-repo
            // source code — they're third-party artefacts. Skip
            // every venv directory so the Rust-only policy doesn't
            // false-fire on the virtualenv contents.
            | Some(".venv")
            | Some("venv")
            | Some("__pycache__")
            | Some("site-packages")
    )
}

/// **2026-06-02** — off-runtime data-prep exemption. The Rust-only
/// kernel directive applies to the *shipped runtime* — `crates/*`
/// and the user-facing binaries in `tools/*`. Data-prep tools that
/// only emit checked-in JSON packs and never run inside the kernel
/// (e.g. `tools/build_*/build.py`, `tools/scrape_*/scrape.py`,
/// `data/stt_models/_convert/*.py` upstream whisper.cpp conversion
/// utilities) are allowed in Python because rewriting them in Rust
/// would buy us nothing — their output, not their implementation,
/// is what the kernel consumes.
fn path_is_off_runtime_data_prep(path: &Path, root: &Path) -> bool {
    let Ok(rel) = path.strip_prefix(root) else {
        return false;
    };
    let comps: Vec<_> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    if comps.len() >= 2 && comps[0] == "tools" {
        let crate_name = &comps[1];
        if crate_name.starts_with("build_")
            || crate_name.starts_with("scrape_")
            || crate_name == "intent_dataset"
        {
            return true;
        }
    }
    if comps.len() >= 3 && comps[0] == "data" && comps[1] == "stt_models" && comps[2] == "_convert"
    {
        return true;
    }
    false
}

fn collect_forbidden_source_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let root = repo_root();
    collect_forbidden_source_files_with_root(dir, &root, out);
}

fn collect_forbidden_source_files_with_root(dir: &Path, root: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).expect("read_dir");
    for entry in entries.flatten() {
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        if file_type.is_dir() {
            if is_ignored_dir(&entry.file_name()) {
                continue;
            }
            collect_forbidden_source_files_with_root(&path, root, out);
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let ext = path.extension().and_then(OsStr::to_str);
        if ext.is_some_and(|ext| forbidden_source_extensions().contains(&ext))
            && !path_is_off_runtime_data_prep(&path, root)
        {
            out.push(path);
        }
    }
}

fn relative_to_repo(path: &Path) -> String {
    let root = repo_root();
    path.strip_prefix(&root)
        .unwrap_or(path)
        .display()
        .to_string()
}

#[test]
fn repository_contains_no_non_rust_source_files() {
    let root = repo_root();
    let mut forbidden = Vec::new();
    collect_forbidden_source_files(&root, &mut forbidden);
    forbidden.sort();

    assert!(
        forbidden.is_empty(),
        "Rust-only policy violated; forbidden non-Rust source files found: {:?}",
        forbidden
            .iter()
            .map(|path| relative_to_repo(path))
            .collect::<Vec<_>>()
    );
}

#[test]
fn shell_scripts_do_not_invoke_foreign_language_runtimes() {
    let scripts_dir = repo_root().join("scripts");
    let entries = fs::read_dir(&scripts_dir).expect("read scripts/");
    let mut violations = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(OsStr::to_str) != Some("sh") {
            continue;
        }
        let contents = fs::read_to_string(&path).expect("read shell script");
        if shell_script_invokes_foreign_runtime(&contents) {
            violations.push(relative_to_repo(&path));
        }
    }

    assert!(
        violations.is_empty(),
        "Rust-only policy violated; shell scripts invoke foreign runtimes: {violations:?}"
    );
}

fn shell_script_invokes_foreign_runtime(contents: &str) -> bool {
    contents.lines().any(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return false;
        }
        let normalized = trimmed.replace(['|', ';', '&', '(', ')'], " ");
        let tokens: Vec<&str> = normalized.split_whitespace().collect();
        if tokens.is_empty() {
            return false;
        }
        if tokens.iter().any(|token| {
            matches!(
                *token,
                "python"
                    | "python3"
                    | "node"
                    | "npm"
                    | "npx"
                    | "deno"
                    | "ruby"
                    | "php"
                    | "perl"
                    | "java"
                    | "javac"
            )
        }) {
            return true;
        }
        tokens.windows(2).any(|pair| pair == ["go", "run"])
    })
}

#[test]
fn executable_shebangs_do_not_target_foreign_runtimes() {
    let root = repo_root();
    let mut violations = Vec::new();
    let mut stack = vec![root.clone()];

    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir).expect("read_dir recursive");
        for entry in entries.flatten() {
            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            if file_type.is_dir() {
                if is_ignored_dir(&entry.file_name()) {
                    continue;
                }
                stack.push(path);
                continue;
            }
            if !file_type.is_file() {
                continue;
            }
            // Off-runtime data-prep tools (build_*, scrape_*) are
            // allowed to use Python shebangs — see
            // `path_is_off_runtime_data_prep` doc comment.
            if path_is_off_runtime_data_prep(&path, &root) {
                continue;
            }
            let Ok(contents) = fs::read_to_string(&path) else {
                continue;
            };
            let first_line = contents.lines().next().unwrap_or_default().to_lowercase();
            if !first_line.starts_with("#!") {
                continue;
            }
            if [
                "python", "node", "ruby", "php", "perl", "deno", "lua", "julia",
            ]
            .iter()
            .any(|runtime| first_line.contains(runtime))
            {
                violations.push(relative_to_repo(&path));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Rust-only policy violated; forbidden shebang runtimes found: {violations:?}"
    );
}
