//! Entity recognisers: extract person names, numbers, time expressions
//! from a parse sequence. v0.7.0 stub — no intents in the MVP set need
//! entity extraction yet (all 5 intents are fixed-form social moves).
//!
//! v0.7.5+ will fill this out as intents like `SelfIntroduction` (needs
//! person-name extraction) and `StatementOfAge` (needs numeral
//! extraction) come online.

/// Stub enum reserved for future entity types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entity {
    PersonName(String),
    Numeral(u64),
    LocationRoot(String),
}
