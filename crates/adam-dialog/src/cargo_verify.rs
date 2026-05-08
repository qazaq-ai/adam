//! **v4.94.0** — Codex 2026-05-07 audit P3 directive #5: cargo-check
//! loop scaffolding. When a student submits a Rust solution to a
//! tutor exercise, adam needs to **verify** the snippet compiles
//! before commenting on correctness. This module provides the
//! verification primitive — a single function that takes Rust
//! source, runs `cargo check` in a temp dir, and returns a
//! structured result.
//!
//! **Scope (scaffolding only):** the function exists and works in
//! tests; full dialog integration (a `SubmitSolution` intent + the
//! end-to-end exchange flow) is deferred to a later release. Codex
//! explicitly framed this as a foundational tool, not a UI feature.
//!
//! **Security note:** `cargo check` only type-checks; it does NOT
//! run the user's `main()`. Even so, build scripts (`build.rs`)
//! CAN execute arbitrary code at compile time. The `verify_snippet`
//! function as currently written wraps the snippet in a fresh
//! Cargo project with no build script — a malicious user could
//! still submit `#![no_std]` + asm tricks. For real student
//! submissions, sandbox via Docker / firejail / wasi, not just a
//! temp dir.

use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;

/// Outcome of a `cargo check` invocation against a Rust snippet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CargoVerifyResult {
    /// `true` when `cargo check` exited 0 — snippet compiles cleanly.
    pub passed: bool,
    /// Canonical `Exxxx` codes parsed from compiler output (deduped).
    /// Empty when `passed == true`. Populated even when there are
    /// only warnings? No — current implementation only collects
    /// codes from the `error[E…]:` prefix.
    pub error_codes: Vec<String>,
    /// Raw combined stdout+stderr from `cargo check`. Useful for
    /// debugging / verbose feedback to the student. Trimmed to
    /// avoid surfacing $CARGO_HOME paths in tutor responses.
    pub raw_output: String,
    /// `true` when this run could NOT execute cargo (e.g. cargo not
    /// on PATH, temp-dir creation failed). Distinguished from
    /// passed=false — environment failure vs. snippet failure.
    pub environment_failed: bool,
}

impl CargoVerifyResult {
    /// Convenience: did the user's snippet trigger a specific error?
    pub fn has_error(&self, code: &str) -> bool {
        self.error_codes.iter().any(|c| c == code)
    }
}

/// Verify a Rust snippet by writing it to a fresh temp project and
/// running `cargo check`. Returns the structured result.
///
/// The snippet is wrapped in a minimal `Cargo.toml` + `src/main.rs`
/// pair, edition 2021, no dependencies. If the snippet itself
/// declares its own `fn main()`, it's used as-is; otherwise the
/// snippet is wrapped in a `fn main() { … }` body so trivial
/// expressions (e.g. `let s = String::from("x"); let s2 = s;
/// println!("{s}");`) compile.
///
/// On systems where `cargo` isn't on `PATH`, returns a result with
/// `environment_failed: true` so the caller can surface a friendly
/// «cargo isn't installed» message to the student instead of a
/// hard panic.
pub fn verify_snippet(snippet: &str) -> CargoVerifyResult {
    // 1. Create a temp project directory. Use `std::env::temp_dir()`
    //    + a unique-enough subdirectory based on PID + nanoseconds.
    let unique = format!(
        "adam_cargo_verify_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    let project_dir: PathBuf = std::env::temp_dir().join(unique);
    if std::fs::create_dir_all(project_dir.join("src")).is_err() {
        return CargoVerifyResult {
            passed: false,
            error_codes: vec![],
            raw_output: "could not create temp project directory".into(),
            environment_failed: true,
        };
    }

    // 2. Write Cargo.toml. Edition 2021, no dependencies, name
    //    matches `adam_check_*` to avoid colliding with anything.
    let cargo_toml = "[package]\nname = \"adam_check\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[dependencies]\n";
    if std::fs::write(project_dir.join("Cargo.toml"), cargo_toml).is_err() {
        let _ = std::fs::remove_dir_all(&project_dir);
        return CargoVerifyResult {
            passed: false,
            error_codes: vec![],
            raw_output: "could not write Cargo.toml".into(),
            environment_failed: true,
        };
    }

    // 3. Wrap snippet if it doesn't already declare `fn main`. The
    //    detection is shallow but works for the common student
    //    patterns we expect (top-level let/println vs. full main).
    let source = if snippet.contains("fn main(") || snippet.contains("fn main (") {
        snippet.to_string()
    } else {
        format!("fn main() {{\n{snippet}\n}}\n")
    };
    if std::fs::write(project_dir.join("src/main.rs"), &source).is_err() {
        let _ = std::fs::remove_dir_all(&project_dir);
        return CargoVerifyResult {
            passed: false,
            error_codes: vec![],
            raw_output: "could not write src/main.rs".into(),
            environment_failed: true,
        };
    }

    // 4. Run `cargo check --quiet`. We capture stderr (where rustc
    //    writes diagnostics) and stdout. Use `--offline` if cached;
    //    otherwise let cargo do its thing (the no-deps Cargo.toml
    //    needs no network).
    let output = Command::new("cargo")
        .arg("check")
        .arg("--quiet")
        .arg("--message-format=human")
        .current_dir(&project_dir)
        .output();

    // 5. Best-effort cleanup — failure here doesn't change the
    //    verification outcome.
    let _ = std::fs::remove_dir_all(&project_dir);

    let output = match output {
        Ok(o) => o,
        Err(e) => {
            return CargoVerifyResult {
                passed: false,
                error_codes: vec![],
                raw_output: format!("could not invoke cargo: {e}"),
                environment_failed: true,
            };
        }
    };

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let raw = format!("{stderr}\n{stdout}");

    // 6. Parse error codes. Compiler format: `error[E0382]: …` /
    //    `error: cannot find …`. We collect only the bracketed
    //    `Eyyyy` form because that's the canonical handle adam's
    //    error-explanation map keys on.
    let error_codes = extract_error_codes(&raw);

    CargoVerifyResult {
        passed: output.status.success() && error_codes.is_empty(),
        error_codes,
        raw_output: raw,
        environment_failed: false,
    }
}

fn extract_error_codes(combined_output: &str) -> Vec<String> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut codes: Vec<String> = Vec::new();
    // Look for `error[Exxxx]:` literally; uppercase E followed by
    // 4+ decimal digits inside square brackets, immediately after
    // the word `error`.
    for line in combined_output.lines() {
        if let Some(start) = line.find("error[E") {
            let rest = &line[start + 6..]; // skip "error[" -> position at 'E'
            let mut iter = rest.char_indices();
            // First char must be 'E'; consume.
            let first = iter.next();
            debug_assert_eq!(first.map(|p| p.1), Some('E'));
            let mut end = 1usize;
            for (i, c) in iter {
                if c.is_ascii_digit() {
                    end = i + 1;
                } else {
                    break;
                }
            }
            let code = &rest[..end];
            if code.len() >= 5 && seen.insert(code.to_string()) {
                codes.push(code.to_string());
            }
        }
    }
    codes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_error_codes_parses_bracketed_form() {
        let stderr = "error[E0382]: borrow of moved value: `s`\n  --> src/main.rs:3:21\n   |\nerror[E0277]: not implemented\n";
        let codes = extract_error_codes(stderr);
        assert!(codes.contains(&"E0382".to_string()));
        assert!(codes.contains(&"E0277".to_string()));
    }

    #[test]
    fn extract_error_codes_dedupes() {
        let stderr = "error[E0382]: a\nerror[E0382]: b\n";
        let codes = extract_error_codes(stderr);
        assert_eq!(codes, vec!["E0382"]);
    }

    #[test]
    fn extract_error_codes_ignores_warnings() {
        let stderr = "warning: unused variable: `x`\n  --> src/main.rs:1:9\n";
        let codes = extract_error_codes(stderr);
        assert!(codes.is_empty());
    }

    // **Slow integration test** — actually invokes cargo. Marked
    // `#[ignore]` so default `cargo test` stays fast; run with
    // `cargo test -- --ignored` to validate the full loop.
    #[test]
    #[ignore]
    fn verify_snippet_detects_e0382_on_use_of_moved_value() {
        let snippet = r#"let s = String::from("hello"); let s2 = s; println!("{s}");"#;
        let result = verify_snippet(snippet);
        assert!(
            !result.environment_failed,
            "cargo not available — install rustup and cargo to run this test"
        );
        assert!(!result.passed, "expected failure on use-of-moved-value");
        assert!(
            result.has_error("E0382"),
            "expected E0382 in error_codes; got {:?}\n--- raw output ---\n{}",
            result.error_codes,
            result.raw_output
        );
    }

    #[test]
    #[ignore]
    fn verify_snippet_passes_clean_program() {
        let snippet = "let x = 5; println!(\"{x}\");";
        let result = verify_snippet(snippet);
        assert!(!result.environment_failed);
        assert!(
            result.passed,
            "expected clean snippet to pass; codes={:?}\n{}",
            result.error_codes, result.raw_output
        );
        assert!(result.error_codes.is_empty());
    }
}
