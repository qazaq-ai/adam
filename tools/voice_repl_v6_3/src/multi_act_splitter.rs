// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/agm
//! # Multi-act utterance splitter — v6.5.0-rc10
//!
//! ## What this closes
//!
//! rc9 audit T36:
//!
//! ```text
//! you said: «Өте жақсы аңгмелестік енді сау бол.»
//! intent (neural) → AskAboutTopic (conf=0.38) ?
//! adam → «Сау болыңыз.»
//! ```
//!
//! The classifier collapsed a compound utterance — appreciation
//! («Өте жақсы аңгмелестік» = "great conversation") + farewell
//! («сау бол») — into one weak label.  adam responded only to the
//! farewell.  The user explicitly flagged it: the appreciation
//! deserved an acknowledgement of its own.
//!
//! This is a structural gap, not a classifier mis-call.  The whole
//! pipeline assumes one speech-act per turn.  rc10 takes a first
//! pragmatic cut: detect a TRAILING FAREWELL after a substantive
//! head, and emit a compound response that acknowledges both.
//!
//! ## Scope
//!
//! The splitter only handles trailing farewells today.  Connectors
//! considered:
//!
//!   - bare: «Сау бол.», «Қош бол.»  (no head — NOT a split)
//!   - with head: «<head> сау бол», «<head>, сау бол», «<head>
//!     енді сау бол», «<head> онда сау бол», «<head> рахмет сау бол»
//!
//! Future scope (when more audits surface them):
//!
//!   - leading greeting + question:  «Сәлем, X деген кім»
//!   - thanks + farewell:             «Рахмет, сау бол»
//!   - confirmation + farewell:       «Иә, сау бол»
//!   - question + appended affirmer:  «X-ты білесің бе, иә»
//!
//! ## Why a deterministic splitter, not a model
//!
//! - The pattern is tiny — a closed set of trailing-farewell
//!   tokens.  Training a model to detect them needs ~1 000 labelled
//!   compounds; we have 1.
//! - It is byte-deterministic and inspectable — matches the v6.2
//!   architectural directive.
//! - Future multi-act classes that DON'T fit a closed-set pattern
//!   are candidates for a learned splitter in rc11+ (joint router
//!   territory).
//!
//! ## Integration
//!
//! In `main.rs`, BEFORE the intent classifier runs:
//!
//! ```ignore
//! let head = match multi_act_splitter::split_trailing_farewell(&text) {
//!     Some(split) => {
//!         farewell_compound = true;
//!         split.head
//!     }
//!     None => text.clone(),
//! };
//! // classify `head` (not `text`) — head is the substantive turn
//! ```
//!
//! After the cascade produces the head's response, append the
//! farewell acknowledgement.

/// Trailing-farewell phrases the splitter recognises (lowercased,
/// punctuation-stripped before matching).  Order matters: longer
/// phrases first so «сау болыңыз» wins over «сау бол».
const TRAILING_FAREWELLS: &[&str] = &[
    // Polite forms
    "сау болыңыз",
    "қош болыңыз",
    "сау болыңдар",
    "көріскенше",
    "көрісуге дейін",
    "тағы кездескенше",
    "хош болыңыз",
    // Informal
    "сау бол",
    "қош бол",
    "хош бол",
];

/// Connectors that may sit between the head and the farewell — the
/// splitter strips them off the head so the head reads as a clean
/// utterance.
const CONNECTORS: &[&str] = &[
    "енді",
    "онда",
    "ал енді",
    "ал онда",
    "сонымен",
    "рахмет",
    "рақмет",
];

/// Acknowledgement template returned after the head's response.
/// Polite by default; the head's content stays grammatically
/// independent of this trailing acknowledgement.
pub const FAREWELL_ACK: &str = "Сау болыңыз, тағы кездескенше!";

/// Outcome of a successful split.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Split {
    /// The substantive head — what the classifier should see and
    /// what the cascade should respond to.
    pub head: String,
    /// The detected farewell tail — preserved for diagnostics.
    pub tail: String,
}

/// Detect a substantive head followed by a trailing farewell.
/// Returns `None` if the input is a bare farewell, has no farewell
/// at all, or the head would be empty after stripping connectors.
///
/// The threshold for «substantive head» is 2 tokens — a single-word
/// head (e.g. just «иә сау бол» / «жарайды сау бол») is too thin to
/// route through the cascade meaningfully and falls back to the
/// bare-farewell path.  Two-word heads like «Жақсы болды» («it was
/// good») ARE substantive in Kazakh and deserve their own ack.
pub fn split_trailing_farewell(input: &str) -> Option<Split> {
    let lower = input.to_lowercase();
    let stripped = lower.trim_end_matches(['.', '!', '?']).trim();

    // Find the LATEST trailing-farewell match (longest first via
    // the const ordering above).  We need it at the END of the
    // utterance — not in the middle — otherwise we'd split «сау бол
    // деген сөз қайдан шыққан» (a question ABOUT the farewell word).
    let (farewell_idx, _farewell) = TRAILING_FAREWELLS
        .iter()
        .filter_map(|f| stripped.rfind(f).map(|i| (i, f)))
        .filter(|(i, f)| i + f.len() == stripped.len())
        .max_by_key(|(_, f)| f.len())?;

    let head_raw = stripped[..farewell_idx].trim_end_matches([',', '—', '–', '-', ' ', '.']);
    let mut head = head_raw.trim().to_string();

    // Strip a single trailing connector if present («енді», «онда»,
    // «рахмет»…).  Multiple connectors back-to-back are rare; one
    // pass is enough in practice.
    for c in CONNECTORS {
        let pat = format!(" {c}");
        if head.to_lowercase().ends_with(&pat) {
            head.truncate(head.len() - pat.len());
            head = head.trim_end_matches([',', '—', '–', '-', ' ']).to_string();
            break;
        }
        // Connector as the WHOLE head — strip it entirely.
        if head.to_lowercase() == *c {
            head.clear();
            break;
        }
    }

    let token_count = head.split_whitespace().count();
    if head.is_empty() || token_count < 2 {
        // Bare farewell, or thin head like «иә сау бол» — let the
        // normal cascade handle it (treats the whole thing as a
        // farewell, which is fine).
        return None;
    }

    Some(Split {
        head: head.trim().to_string(),
        tail: stripped[farewell_idx..].to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The exact rc9 audit T36 case the splitter exists to handle.
    #[test]
    fn rc9_audit_t36_compound_farewell() {
        let split = split_trailing_farewell("Өте жақсы аңгмелестік енді сау бол.")
            .expect("compound utterance must split");
        // Connector «енді» stripped, comma/period gone.
        assert_eq!(split.head, "өте жақсы аңгмелестік");
        assert_eq!(split.tail, "сау бол");
    }

    /// Polite form preferred over the informal.
    #[test]
    fn polite_farewell_wins_when_present() {
        let split =
            split_trailing_farewell("Әңгімеміз керемет болды, сау болыңыз").expect("must split");
        assert!(split.head.contains("керемет"));
        assert_eq!(split.tail, "сау болыңыз");
    }

    /// A bare farewell — no head — returns None so the normal
    /// cascade handles it.
    #[test]
    fn bare_farewell_is_not_a_split() {
        assert!(split_trailing_farewell("сау бол").is_none());
        assert!(split_trailing_farewell("Сау болыңыз.").is_none());
        assert!(split_trailing_farewell("Қош болыңыз!").is_none());
    }

    /// A short head (under 3 tokens) — let the normal cascade
    /// handle the whole utterance.  This avoids over-splitting on
    /// «иә сау бол» where the «иә» is just a one-word ack.
    #[test]
    fn thin_head_falls_back_to_cascade() {
        assert!(split_trailing_farewell("иә сау бол").is_none());
        assert!(split_trailing_farewell("жарайды сау бол").is_none());
    }

    /// Farewell-as-topic — «сау бол» appears mid-sentence in a
    /// question ABOUT the farewell phrase.  Must NOT split.
    #[test]
    fn farewell_inside_sentence_is_not_a_trailing_split() {
        let r = split_trailing_farewell("«сау бол» деген қайдан шыққан сөз");
        assert!(
            r.is_none(),
            "non-trailing match should not split, got {r:?}"
        );
    }

    /// Connector at the boundary — strip from the head.
    #[test]
    fn connector_at_boundary_stripped() {
        let split =
            split_trailing_farewell("Сұхбатымыз жақсы болды онда сау бол").expect("must split");
        assert!(
            !split.head.ends_with("онда") && !split.head.ends_with(" онда"),
            "connector «онда» not stripped, head={:?}",
            split.head
        );
    }

    /// `тағы кездескенше` — the longer farewell variant.
    #[test]
    fn longer_farewell_variant_recognised() {
        let split = split_trailing_farewell("Жақсы болды тағы кездескенше").expect("must split");
        assert_eq!(split.tail, "тағы кездескенше");
    }
}
