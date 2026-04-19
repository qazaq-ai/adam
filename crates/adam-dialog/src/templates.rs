//! Template repository loader.
//!
//! v0.7.0 stub — templates are hardcoded in `planner.rs` for the MVP.
//! This module will become substantive in v0.7.5 when templates move to
//! TOML files under `data/dialog/templates/`.
//!
//! Design sketch (for v0.7.5 implementation):
//!
//! ```ignore
//! // One .toml file per intent family.
//! // Load all at startup into an in-memory HashMap.
//!
//! pub struct TemplateRepository {
//!     by_intent: HashMap<IntentKey, Vec<Template>>,
//! }
//!
//! pub struct Template {
//!     pub id: String,
//!     pub applicable_when: Conditions,
//!     pub atoms: Vec<Atom>,  // literal, root-slot, or (root, features)
//! }
//! ```

/// Stub — becomes the real repository in v0.7.5.
pub struct TemplateRepository;

impl TemplateRepository {
    /// Placeholder constructor. Returns an empty repository; v0.7.0
    /// `planner.rs` doesn't consult it.
    pub fn new() -> Self {
        Self
    }
}

impl Default for TemplateRepository {
    fn default() -> Self {
        Self::new()
    }
}
