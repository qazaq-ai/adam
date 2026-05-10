//! Typed slot inventory — v5.7.0 (G1.0 of the proof-carrying
//! generation arc).
//!
//! Pre-v5.7.0 dialog slots were a typeless `HashMap<String, String>`:
//! any template could pull any key, an unfamiliar key would render as
//! `{slot_name}` literal in the output, and variation in responses
//! came entirely from picking among template strings — never from the
//! slot side.
//!
//! This module ships the **inventory schema + loader** for typed
//! slots. The on-disk source is [`data/dialog/slot_inventory.toml`].
//! At G1.0 the inventory is **descriptive**: it documents the slot
//! surface and lets future code query "is this slot registered?" /
//! "what kinds of variation does this slot support?". The realiser
//! does not yet consult the inventory to pick variants — that's
//! G1.5.
//!
//! ## Trajectory
//!
//! - **G1.0 (this milestone)** — schema + loader + 26 slot
//!   definitions covering every key the existing 117-family template
//!   repository consumes. No behavioural change.
//! - **G1.5** — realiser optionally enumerates `Variant`s per slot
//!   and rng-picks one per turn. Lets us add per-slot variation
//!   without touching templates.
//! - **G2.0** — `ProofObject = { conclusion, support, derivation,
//!   hedges }`; the chain-query + safety-refusal layers retrofit to
//!   produce proof objects; verifier gates the emit.
//! - **G3.0** — typed composer over proof objects → answer IR →
//!   realiser. Generation as proof-carrying composition.
//!
//! Per `MISSION.md`, Kazakh agglutinative morphology is itself a
//! proof system at the word level (FST roundtrip 100 %). The G1→G3
//! arc extends that discipline upward to discourse.

use std::{
    collections::{BTreeMap, HashMap},
    fs,
    path::Path,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

const DEFAULT_INVENTORY_PATH: &str = "data/dialog/slot_inventory.toml";
const SCHEMA_VERSION: u32 = 1;

/// Categorical type of a slot's value. Different `kind`s warrant
/// different validation / variation strategies; downstream code uses
/// this for routing decisions (e.g. only `person_name` slots can use
/// the `respectful_address` variant strategy).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotKind {
    /// First name of a person ({name}, {name_respect}).
    PersonName,
    /// Geographic place name ({city}).
    PlaceName,
    /// Profession noun ({occupation}).
    Profession,
    /// Numeric literal — age, count, year ({age}).
    Numeric,
    /// Generic noun term — topic, subject, predicate ({topic}, {noun},
    /// {subject_term}).
    NounTerm,
    /// IsA-question predicate term ({predicate_term}).
    PredicateTerm,
    /// Rendered derivation chain ({chain}).
    Chain,
    /// Verbatim curated fact text ({fact}).
    CuratedFact,
    /// adam's self-identity ({system_name}, {system_full_name}, …).
    SystemSelf,
    /// Curriculum-stage body — pre-rendered exercise / purpose /
    /// contrast text ({exercise_body}, {purpose_body},
    /// {contrast_body}).
    CurriculumTopic,
    /// Compiler-error identifier ({error_code}).
    ErrorCode,
    /// Rust language concept ({rust_concept}).
    RustConcept,
    /// Conflict-resolution value ({old_value}, {new_value}).
    ConflictValue,
    /// Internal flag / classifier — not user-visible text
    /// ({cargo_status}, {city_id}, {predicate}, {geo_kind}).
    Sentinel,
}

/// Surface-realisation strategy for a slot value. Each strategy is a
/// rule for transforming a literal value into a possibly-different
/// surface form. The runtime can pick among the listed strategies
/// per turn (rng-seeded) to add controlled variation.
///
/// Strategies are pure data — adding a new strategy means extending
/// this enum + implementing the transform; existing inventory entries
/// can adopt it without recompilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VariantStrategy {
    /// Use the value verbatim. Default for slots without registered
    /// variation.
    Literal,
    /// Apply Kazakh-tradition respectful address transformation
    /// (Дәулет → Дәке). Falls back to literal if the value doesn't
    /// support the transform.
    RespectfulAddress,
    /// FST-synthesise the value in the locative case (X-да / X-де).
    /// Documentation alias for `{slot|case=loc}` template syntax.
    FstLocative,
    /// FST-synthesise in the genitive case.
    FstGenitive,
    /// FST-synthesise in the dative case.
    FstDative,
    /// FST-synthesise in the ablative case.
    FstAblative,
}

/// Documented FST feature spec a template may legitimately attach to
/// this slot via the `{slot|features}` syntax. Documentation only —
/// the realiser parses these in `slot_syntax.rs`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotFstFeature(pub String);

/// One slot in the inventory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Slot {
    /// Placeholder key as it appears in templates.
    pub name: String,
    /// Categorical type of the slot's value.
    pub kind: SlotKind,
    /// Human-readable description of what the slot carries.
    pub description: String,
    /// Representative literal example value.
    pub example: String,
    /// Surface-realisation strategies the runtime may pick among per
    /// turn. The first strategy is the canonical fallback. At G1.0
    /// this is descriptive; G1.5 wires the realiser to consult it.
    #[serde(default)]
    pub variants: Vec<VariantStrategy>,
    /// Documented FST features (locative / genitive / …) templates
    /// may apply to this slot. Documentation only.
    #[serde(default)]
    pub fst_features: Vec<String>,
}

impl Slot {
    /// True when this slot can produce more than one surface form
    /// from the same literal value (i.e. has a non-trivial variant
    /// set). G1.5 routing will key off this predicate.
    pub fn supports_variation(&self) -> bool {
        self.variants
            .iter()
            .any(|v| !matches!(v, VariantStrategy::Literal))
    }
}

/// Top-level inventory — parses the `data/dialog/slot_inventory.toml`
/// file. A successful load guarantees `schema_version == SCHEMA_VERSION`
/// (caller fails fast on mismatch).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotInventory {
    pub schema_version: u32,
    #[serde(default)]
    pub slots: Vec<Slot>,
}

#[derive(Debug, Error)]
pub enum SlotInventoryLoadError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml parse: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("schema version mismatch: file has {got}, runtime expects {expected}")]
    SchemaVersion { expected: u32, got: u32 },
    #[error("duplicate slot name: {0}")]
    DuplicateSlot(String),
}

impl SlotInventory {
    /// Load the inventory from the canonical default path
    /// (`data/dialog/slot_inventory.toml` relative to CWD).
    pub fn load_default() -> Result<Self, SlotInventoryLoadError> {
        Self::load(DEFAULT_INVENTORY_PATH)
    }

    /// Load the inventory from a specific path.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, SlotInventoryLoadError> {
        let raw = fs::read_to_string(path.as_ref())?;
        let inv: Self = toml::from_str(&raw)?;
        inv.validate()?;
        Ok(inv)
    }

    /// Validate schema version + slot uniqueness. Called automatically
    /// from `load`; callers building an inventory in code should call
    /// this themselves.
    pub fn validate(&self) -> Result<(), SlotInventoryLoadError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(SlotInventoryLoadError::SchemaVersion {
                expected: SCHEMA_VERSION,
                got: self.schema_version,
            });
        }
        let mut seen: HashMap<String, ()> = HashMap::new();
        for slot in &self.slots {
            if seen.insert(slot.name.clone(), ()).is_some() {
                return Err(SlotInventoryLoadError::DuplicateSlot(slot.name.clone()));
            }
        }
        Ok(())
    }

    /// Look up a slot by name. Returns `None` if the slot is not
    /// registered in the inventory.
    pub fn get(&self, name: &str) -> Option<&Slot> {
        self.slots.iter().find(|s| s.name == name)
    }

    /// True when the slot is registered in the inventory. Used for
    /// future diagnostic hooks (warn on unknown slots in templates).
    pub fn contains(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// Total number of registered slots.
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Whether the inventory is empty.
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Group slots by kind for diagnostic / documentation use.
    pub fn by_kind(&self) -> BTreeMap<&'static str, Vec<&Slot>> {
        let mut map: BTreeMap<&'static str, Vec<&Slot>> = BTreeMap::new();
        for slot in &self.slots {
            map.entry(slot.kind.slug()).or_default().push(slot);
        }
        map
    }
}

impl SlotKind {
    /// Stable string slug for the kind — used in diagnostic output
    /// and the TOML schema (matches `serde(rename_all = "snake_case")`).
    pub fn slug(&self) -> &'static str {
        match self {
            Self::PersonName => "person_name",
            Self::PlaceName => "place_name",
            Self::Profession => "profession",
            Self::Numeric => "numeric",
            Self::NounTerm => "noun_term",
            Self::PredicateTerm => "predicate_term",
            Self::Chain => "chain",
            Self::CuratedFact => "curated_fact",
            Self::SystemSelf => "system_self",
            Self::CurriculumTopic => "curriculum_topic",
            Self::ErrorCode => "error_code",
            Self::RustConcept => "rust_concept",
            Self::ConflictValue => "conflict_value",
            Self::Sentinel => "sentinel",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_default_inventory_v570() {
        // Test runs from the workspace root — the inventory file is
        // at `data/dialog/slot_inventory.toml`. Skip silently if the
        // file isn't present (trimmed CI checkouts).
        let path = Path::new("../../data/dialog/slot_inventory.toml");
        if !path.exists() {
            return;
        }
        let inv = SlotInventory::load(path).expect("inventory must parse");
        assert_eq!(inv.schema_version, SCHEMA_VERSION);
        assert!(
            inv.len() >= 20,
            "G1.0 inventory documents at least 20 slots (got {})",
            inv.len()
        );
    }

    #[test]
    fn rejects_duplicate_slot_names_v570() {
        let raw = r#"
            schema_version = 1
            [[slots]]
            name = "name"
            kind = "person_name"
            description = "first"
            example = "Дәулет"
            variants = ["literal"]

            [[slots]]
            name = "name"
            kind = "person_name"
            description = "duplicate"
            example = "Айгүл"
            variants = ["literal"]
        "#;
        let inv: SlotInventory = toml::from_str(raw).unwrap();
        let err = inv.validate().unwrap_err();
        assert!(matches!(err, SlotInventoryLoadError::DuplicateSlot(_)));
    }

    #[test]
    fn rejects_schema_version_mismatch_v570() {
        let raw = r#"
            schema_version = 999
            [[slots]]
            name = "name"
            kind = "person_name"
            description = ""
            example = ""
            variants = ["literal"]
        "#;
        let inv: SlotInventory = toml::from_str(raw).unwrap();
        let err = inv.validate().unwrap_err();
        assert!(matches!(err, SlotInventoryLoadError::SchemaVersion { .. }));
    }

    #[test]
    fn slot_supports_variation_v570() {
        let s = Slot {
            name: "name".into(),
            kind: SlotKind::PersonName,
            description: "...".into(),
            example: "Дәулет".into(),
            variants: vec![VariantStrategy::Literal, VariantStrategy::RespectfulAddress],
            fst_features: vec![],
        };
        assert!(s.supports_variation());

        let only_literal = Slot {
            name: "city_id".into(),
            kind: SlotKind::Sentinel,
            description: "...".into(),
            example: "geo_kz_001".into(),
            variants: vec![VariantStrategy::Literal],
            fst_features: vec![],
        };
        assert!(!only_literal.supports_variation());
    }

    #[test]
    fn slot_kind_slugs_are_stable_v570() {
        assert_eq!(SlotKind::PersonName.slug(), "person_name");
        assert_eq!(SlotKind::CurriculumTopic.slug(), "curriculum_topic");
        assert_eq!(SlotKind::Sentinel.slug(), "sentinel");
    }

    #[test]
    fn by_kind_groups_correctly_v570() {
        let inv = SlotInventory {
            schema_version: SCHEMA_VERSION,
            slots: vec![
                Slot {
                    name: "name".into(),
                    kind: SlotKind::PersonName,
                    description: "".into(),
                    example: "".into(),
                    variants: vec![],
                    fst_features: vec![],
                },
                Slot {
                    name: "name_respect".into(),
                    kind: SlotKind::PersonName,
                    description: "".into(),
                    example: "".into(),
                    variants: vec![],
                    fst_features: vec![],
                },
                Slot {
                    name: "city".into(),
                    kind: SlotKind::PlaceName,
                    description: "".into(),
                    example: "".into(),
                    variants: vec![],
                    fst_features: vec![],
                },
            ],
        };
        let by = inv.by_kind();
        assert_eq!(by.get("person_name").map(|v| v.len()), Some(2));
        assert_eq!(by.get("place_name").map(|v| v.len()), Some(1));
    }
}
