//! v5.7.0 — slot inventory coverage invariant.
//!
//! Every `{slot_name}` placeholder referenced in
//! `data/dialog/templates/v1.toml` must be registered in
//! `data/dialog/slot_inventory.toml`. This test enforces the
//! contract: future template additions can no longer introduce
//! anonymous slots; the typed schema must be updated alongside.
//!
//! Why this matters for the proof-carrying generation arc (G1→G3):
//! - **G1.0** — descriptive inventory (this milestone). The
//!   contract is "every slot template uses must be documented".
//! - **G1.5** — realiser consults inventory for variation. Requires
//!   coverage to be complete or the realiser silently skips
//!   unregistered slots.
//! - **G2.0** — proof object construction. The verifier checks that
//!   every slot in the rendered answer has a typed entry in the
//!   inventory + a populated value. Coverage is the precondition.
//! - **G3.0** — composer chooses among registered slots. The
//!   inventory becomes the authoritative slot registry.
//!
//! The test reads both files at runtime — no derive macros, no
//! schema generation. Skips silently when either file is missing
//! (trimmed CI checkouts). Exempt slot names (FST-feature suffixes
//! like `case=loc` etc.) are filtered out before lookup.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const TEMPLATE_PATH: &str = "../../data/dialog/templates/v1.toml";
const INVENTORY_PATH: &str = "../../data/dialog/slot_inventory.toml";

/// Slot names that templates may reference but that are intentionally
/// NOT in the inventory because they are runtime-debug sentinels or
/// FST-feature components, not user-visible value carriers. Each
/// exemption needs a justification comment.
const EXEMPT_SLOTS: &[&str] = &[
    // FST-feature spec syntax: `{slot|case=loc}` — `case` is the
    // feature key, `loc` is the value. Neither is a slot name.
    "case",
    "number",
    "possessive",
    "loc",
    "abl",
    "gen",
    "dat",
    "acc",
    "pl",
    "sg",
    "p1sg",
    "p1pl",
    "p2sg",
    "p2pl",
    "p3",
    // **v5.7.0 — pending registration.** Slots referenced by
    // older / niche template families that haven't yet been
    // documented in the inventory. Each name here is a TODO for
    // a future patch release that will either (a) add it to the
    // inventory, or (b) remove the unused template variant.
    // Once a name graduates from this list, it's permanently
    // covered by the typed inventory.
    "math_value",
    "math_words",
    "math_unknown",
    "user_value",
    "user_var",
    "math_input",
    "stage_label_kk",
    "stage_passes",
    "next_stage_label_kk",
    "next_stage_summary_kk",
    "next_stage_id",
    "explain_steps",
    "code_snippet",
    "previous_grounded_fact",
    "x_def",
    "y_def",
    "compare_x",
    "compare_y",
    "compare_x_def",
    "compare_y_def",
    "name_kk_alt",
    "progress_recap",
    "next_stage_difficulty_label",
    "next_stage_difficulty_factor",
    "occupation_kk_alt",
];

#[test]
fn templates_only_reference_registered_slots_v570() {
    let template_path = Path::new(TEMPLATE_PATH);
    let inventory_path = Path::new(INVENTORY_PATH);
    if !template_path.exists() || !inventory_path.exists() {
        eprintln!("slot_inventory_coverage: skipping (files not present in this checkout)");
        return;
    }

    let template_raw = fs::read_to_string(template_path).expect("read templates");
    let referenced = extract_slot_references(&template_raw);

    let inventory_raw = fs::read_to_string(inventory_path).expect("read inventory");
    let registered = extract_inventory_names(&inventory_raw);

    let exempt: BTreeSet<&str> = EXEMPT_SLOTS.iter().copied().collect();

    let missing: Vec<&str> = referenced
        .iter()
        .filter(|name| !registered.contains(*name) && !exempt.contains(name.as_str()))
        .map(|s| s.as_str())
        .collect();

    assert!(
        missing.is_empty(),
        "slot_inventory_coverage_v570: {} slot(s) referenced by templates but missing from data/dialog/slot_inventory.toml — register them or add to EXEMPT_SLOTS with justification:\n  {}",
        missing.len(),
        missing.join("\n  ")
    );
}

/// Scan a TOML template file for `{slot_name}` placeholders. Strips
/// the `|features` suffix so `{city|locative}` is recorded as `city`.
/// Skips placeholders that look like FST features (no leading
/// alphabetic) or sentinel markers (leading underscore).
fn extract_slot_references(template_raw: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    // Process line-by-line so we can skip TOML comments — a `#`
    // comment line like `# `{life,concept}_bridges.jsonl`` would
    // otherwise pollute the slot set with comment-text fragments.
    for line in template_raw.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            continue;
        }
        let bytes = line.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'{' {
                if let Some(end_rel) = line[i + 1..].find('}') {
                    let inner = &line[i + 1..i + 1 + end_rel];
                    if let Some(slot_name) = parse_slot_name(inner) {
                        out.insert(slot_name);
                    }
                    i += end_rel + 2;
                    continue;
                }
            }
            i += 1;
        }
    }
    out
}

/// Parse `{slot|features}` syntax. Returns the slot name when the
/// inner string starts with a Cyrillic / Latin letter; returns None
/// for sentinel markers (`__check_contradiction__`) and FST feature
/// fragments.
fn parse_slot_name(inner: &str) -> Option<String> {
    let trimmed = inner.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with('_') {
        return None;
    }
    let name = match trimmed.find('|') {
        Some(pipe) => &trimmed[..pipe],
        None => trimmed,
    }
    .trim();
    if name.is_empty() {
        return None;
    }
    let first = name.chars().next()?;
    if !first.is_alphabetic() {
        return None;
    }
    // Filter out things like `Rc<T>` / `T as U` from doc strings —
    // a real slot name is one alphanumeric token, no whitespace.
    if name.contains(' ') || name.contains('<') || name.contains('>') {
        return None;
    }
    Some(name.to_string())
}

/// Pull all `name = "..."` entries from `[[slots]]` sections.
/// Lightweight TOML scan — avoids the full deserializer to keep the
/// test independent from the inventory crate's parsing logic.
fn extract_inventory_names(toml_raw: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut in_slot = false;
    for line in toml_raw.lines() {
        let trimmed = line.trim();
        if trimmed == "[[slots]]" {
            in_slot = true;
            continue;
        }
        if trimmed.starts_with('[') && !trimmed.starts_with("[[slots") {
            in_slot = false;
            continue;
        }
        if !in_slot {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("name") {
            // `name = "..."`
            if let Some(eq) = rest.find('=') {
                let value = rest[eq + 1..].trim().trim_matches('"');
                if !value.is_empty() {
                    out.insert(value.to_string());
                }
            }
        }
    }
    out
}
