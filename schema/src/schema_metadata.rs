use serde_json::{Map, Value};

pub(crate) const JSONCOMPAT_METADATA_KEY: &str = "x-jsoncompat";

pub(crate) const SCHEMA_METADATA_KEYS: [&str; 9] = [
    "$schema",
    "$id",
    "$anchor",
    "$dynamicAnchor",
    "$comment",
    "$defs",
    "definitions",
    "title",
    JSONCOMPAT_METADATA_KEY,
];

pub(crate) const PRESERVED_SCHEMA_METADATA_KEYS: [&str; 8] = [
    "$schema",
    "$id",
    "$anchor",
    "$dynamicAnchor",
    "$defs",
    "definitions",
    "title",
    JSONCOMPAT_METADATA_KEY,
];

// Terminal schemas preserve identity metadata, but not `$defs` / `definitions`:
// once an object collapses to `{"not": true}` there are no remaining subschemas
// that can reference local definitions.
pub(crate) const TERMINAL_SCHEMA_METADATA_KEYS: [&str; 6] = [
    "$schema",
    "$id",
    "$anchor",
    "$dynamicAnchor",
    "title",
    JSONCOMPAT_METADATA_KEY,
];

#[must_use]
pub(crate) fn is_schema_metadata_key(key: &str) -> bool {
    SCHEMA_METADATA_KEYS.contains(&key)
}

pub(crate) fn strip_schema_metadata(obj: &mut Map<String, Value>) {
    for key in SCHEMA_METADATA_KEYS {
        obj.remove(key);
    }
}
