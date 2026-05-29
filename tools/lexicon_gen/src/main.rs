// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `lexicon_gen` — emit `LexEntry::new(...)` literals from a
//! tab-separated `label\tcyrillic\tpos\tcategory` table.
//!
//! Input format (one entry per line, blank lines + `#` comments
//! ignored):
//!
//! ```text
//! n3_bal     бал     Noun        len3
//! v_kel      кел     Verb        verb_stem
//! adj_qara   қара    Adjective   color
//! ```
//!
//! Reads STDIN, writes Rust source to STDOUT. The phoneme
//! decomposition is the output of `cyrillic_to_phonemes(...,
//! is_native_root=true)`, so the v6.3 strict-orthographic rule
//! (every «ы» / «і» drops) is baked into every emitted entry.
//! Re-running the generator after a rule change atomically
//! refreshes the entire static lexicon, no per-word manual
//! audit needed.

use adam_phoneme::Phoneme;
use adam_phoneme::cyrillic::cyrillic_to_phonemes;
use std::io::{self, BufRead, Write};

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut emitted = 0_usize;
    let mut skipped = 0_usize;
    for line in stdin.lock().lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = trimmed.split('\t').collect();
        if parts.len() != 4 {
            eprintln!(
                "[lexicon_gen] skipping malformed line ({} cols): {trimmed}",
                parts.len()
            );
            skipped += 1;
            continue;
        }
        let (label, cyrillic, pos, category) = (parts[0], parts[1], parts[2], parts[3]);
        let phonemes = cyrillic_to_phonemes(cyrillic, /* is_native_root */ true);
        if phonemes.is_empty() {
            eprintln!(
                "[lexicon_gen] skipping {label} «{cyrillic}» — produces zero phonemes under the native-root rule"
            );
            skipped += 1;
            continue;
        }
        writeln!(
            out,
            "    LexEntry::new(\"{label}\", \"{cyrillic}\", &[{}], Pos::{pos}, \"{category}\"),",
            phonemes
                .iter()
                .map(phoneme_ident)
                .collect::<Vec<_>>()
                .join(", "),
        )?;
        emitted += 1;
    }
    eprintln!("[lexicon_gen] emitted {emitted}, skipped {skipped}");
    Ok(())
}

/// `Phoneme::A → "A"` — the variant name that downstream
/// `use Phoneme::*;` brings into scope.
fn phoneme_ident(p: &Phoneme) -> String {
    format!("{p:?}")
}
