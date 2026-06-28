// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Provenance / source-reference types.
//!
//! Every [`crate::CandidateFact`] / [`crate::CandidateProcedure`]
//! carries a [`SourceRef`] so the reviewer can trace WHERE
//! the candidate came from — manual entry, a specific text
//! file at a specific line, a PDF extract, a URL scrape.
//! Auditable provenance is the deterministic-kernel
//! discipline applied to the ingestion side.

use serde::{Deserialize, Serialize};

/// Where a candidate originated.  Closed-set enum so the
/// pipeline can route handlers per source type
/// (extractor-specific normalisation, source-aware
/// confidence floors, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    /// Hand-authored by a curator at the keyboard.  Carries
    /// the highest implicit trust — no extraction noise.
    ManualEntry,
    /// Plain-text Kazakh corpus file (`*.txt`, `*.kk` etc.).
    /// Extractor surfaces candidate sentences with line
    /// numbers for traceability.
    TextFile,
    /// PDF text extract — same shape as TextFile but the
    /// PDF-to-text step itself can introduce artefacts
    /// (ligature drops, RTL contamination, missing
    /// diacritics).  Validators should bias more
    /// conservative for this source.
    PdfExtract,
    /// Web-scraped page.  Source URL stored in the
    /// `identifier`.  Lowest implicit trust; expect noise.
    UrlScrape,
    /// JSONL ingestion from a structured upstream (e.g.
    /// ССГПО internal SOP export, Adilet-Zan article
    /// scraping pipeline).  Schema-checked at the
    /// extractor; carries moderate trust.
    StructuredJsonl,
}

/// Reference back to a specific source location for a
/// candidate.  `identifier` is the path / URL / fixture id;
/// `line` is optional because not every source has line
/// granularity (URL scrapes report `None`).  `notes` is
/// free-text for the curator (e.g. «extracted from §3.2,
/// before the Russian translation block»).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRef {
    pub kind: SourceKind,
    /// Path, URL, or stable id of the source.
    pub identifier: String,
    /// Line number within the source when available
    /// (text / pdf extracts).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    /// Free-text reviewer notes about this source.  Empty
    /// string at default; the «context I want a future
    /// reviewer to see» slot.
    #[serde(default)]
    pub notes: String,
}

impl SourceRef {
    /// Convenience constructor for hand-authored entries —
    /// no path, no line, identifier = «manual:{curator}».
    pub fn manual(curator: &str) -> Self {
        Self {
            kind: SourceKind::ManualEntry,
            identifier: format!("manual:{curator}"),
            line: None,
            notes: String::new(),
        }
    }

    /// Convenience constructor for text-file sources with
    /// a known line number.
    pub fn text_file(path: impl Into<String>, line: u32) -> Self {
        Self {
            kind: SourceKind::TextFile,
            identifier: path.into(),
            line: Some(line),
            notes: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_constructor_shape() {
        let r = SourceRef::manual("shaman");
        assert_eq!(r.kind, SourceKind::ManualEntry);
        assert_eq!(r.identifier, "manual:shaman");
        assert!(r.line.is_none());
        assert!(r.notes.is_empty());
    }

    #[test]
    fn text_file_constructor_shape() {
        let r = SourceRef::text_file("data/raw/abai_words.txt", 42);
        assert_eq!(r.kind, SourceKind::TextFile);
        assert_eq!(r.identifier, "data/raw/abai_words.txt");
        assert_eq!(r.line, Some(42));
    }

    #[test]
    fn round_trip_json() {
        let r = SourceRef {
            kind: SourceKind::PdfExtract,
            identifier: "data/external/labour_code_414-V.pdf".into(),
            line: Some(184),
            notes: "before the Russian translation block".into(),
        };
        let j = serde_json::to_string(&r).expect("serialize");
        let back: SourceRef = serde_json::from_str(&j).expect("deserialize");
        assert_eq!(r, back);
    }

    #[test]
    fn line_field_skips_when_absent() {
        let r = SourceRef {
            kind: SourceKind::UrlScrape,
            identifier: "https://adilet.zan.kz/...".into(),
            line: None,
            notes: String::new(),
        };
        let j = serde_json::to_string(&r).expect("serialize");
        // Field absent in JSON when None — keeps JSONL terse.
        assert!(!j.contains("\"line\""), "got: {j}");
    }
}
