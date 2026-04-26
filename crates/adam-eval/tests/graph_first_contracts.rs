use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn relative_to_repo(path: &Path) -> String {
    let root = repo_root();
    path.strip_prefix(&root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn is_ignored_dir(name: &OsStr) -> bool {
    matches!(
        name.to_str(),
        Some(".git") | Some("target") | Some("node_modules")
    )
}

fn manifest_and_shell_files(dir: &Path, out: &mut Vec<PathBuf>) {
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
            manifest_and_shell_files(&path, out);
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let file_name = path.file_name().and_then(OsStr::to_str);
        let ext = path.extension().and_then(OsStr::to_str);
        if matches!(file_name, Some("Cargo.toml") | Some("Cargo.lock")) || ext == Some("sh") {
            out.push(path);
        }
    }
}

fn forbidden_graph_stack_markers() -> &'static [&'static str] {
    &[
        "neo4j",
        "arangodb",
        "janusgraph",
        "dgraph",
        "tigergraph",
        "cypher-shell",
        "gremlin",
        "networkx",
        "python-igraph",
        "graph-tool",
        "rdflib",
        "sparql-client",
    ]
}

#[test]
fn canonical_rust_graph_entrypoints_exist() {
    let root = repo_root();
    let required = [
        root.join("crates/adam-reasoning/src/graph.rs"),
        root.join("crates/adam-reasoning/src/bin/build_lexical_graph.rs"),
    ];

    let missing: Vec<String> = required
        .iter()
        .filter(|path| !path.is_file())
        .map(|path| relative_to_repo(path))
        .collect();

    assert!(
        missing.is_empty(),
        "Graph-first policy violated; canonical Rust graph entrypoints missing: {missing:?}"
    );
}

#[test]
fn repository_does_not_introduce_foreign_graph_stack_markers() {
    let root = repo_root();
    let mut candidates = Vec::new();
    manifest_and_shell_files(&root, &mut candidates);
    candidates.sort();

    let mut violations = Vec::new();
    for path in candidates {
        let Ok(contents) = fs::read_to_string(&path) else {
            continue;
        };
        let contents = contents.to_lowercase();
        if let Some(marker) = forbidden_graph_stack_markers()
            .iter()
            .find(|marker| contents.contains(**marker))
        {
            violations.push(format!("{} ({marker})", relative_to_repo(&path)));
        }
    }

    assert!(
        violations.is_empty(),
        "Graph-first policy violated; foreign graph stack markers found: {violations:?}"
    );
}

#[test]
fn readme_declares_graph_first_policy() {
    let readme = fs::read_to_string(repo_root().join("README.md")).expect("read README.md");
    assert!(
        readme.contains("### Graph-First Policy"),
        "README.md must declare Graph-First Policy explicitly"
    );
    assert!(
        readme.contains("Rust-native and repository-native"),
        "README.md Graph-First Policy must state that the graph layer stays Rust-native"
    );
}
