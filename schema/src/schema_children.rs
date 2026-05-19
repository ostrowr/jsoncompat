//! Shared JSON Schema child-keyword families for source-tree traversals.
//!
//! These families describe where JSON Schema stores nested schemas in a raw
//! document. Keeping them in one place prevents validators, compatibility
//! guards, and OpenAPI lowering from drifting apart as the supported surface
//! grows.

/// Keywords whose value is a single child schema.
pub const SINGLE_SCHEMA_CHILD_KEYWORDS: [&str; 11] = [
    "additionalProperties",
    "contains",
    "contentSchema",
    "else",
    "if",
    "items",
    "not",
    "propertyNames",
    "then",
    "unevaluatedItems",
    "unevaluatedProperties",
];

/// Keywords whose value is an object map of child schemas.
pub const SCHEMA_MAP_CHILD_KEYWORDS: [&str; 5] = [
    "$defs",
    "definitions",
    "dependentSchemas",
    "patternProperties",
    "properties",
];

/// Keywords whose value is an array of child schemas.
pub const SCHEMA_ARRAY_CHILD_KEYWORDS: [&str; 4] = ["allOf", "anyOf", "oneOf", "prefixItems"];

#[must_use]
pub fn is_single_schema_child_keyword(keyword: &str) -> bool {
    SINGLE_SCHEMA_CHILD_KEYWORDS.contains(&keyword)
}

#[must_use]
pub fn is_schema_map_child_keyword(keyword: &str) -> bool {
    SCHEMA_MAP_CHILD_KEYWORDS.contains(&keyword)
}

#[must_use]
pub fn is_schema_array_child_keyword(keyword: &str) -> bool {
    SCHEMA_ARRAY_CHILD_KEYWORDS.contains(&keyword)
}

#[cfg(test)]
mod tests {
    use super::{
        SCHEMA_ARRAY_CHILD_KEYWORDS, SCHEMA_MAP_CHILD_KEYWORDS, SINGLE_SCHEMA_CHILD_KEYWORDS,
        is_schema_array_child_keyword, is_schema_map_child_keyword, is_single_schema_child_keyword,
    };
    use std::collections::BTreeSet;

    #[test]
    fn schema_child_keyword_families_are_disjoint_and_queryable() {
        let mut seen = BTreeSet::new();

        for keyword in SINGLE_SCHEMA_CHILD_KEYWORDS {
            assert!(
                seen.insert(keyword),
                "duplicate keyword family member: {keyword}"
            );
            assert!(is_single_schema_child_keyword(keyword));
            assert!(!is_schema_map_child_keyword(keyword));
            assert!(!is_schema_array_child_keyword(keyword));
        }

        for keyword in SCHEMA_MAP_CHILD_KEYWORDS {
            assert!(
                seen.insert(keyword),
                "duplicate keyword family member: {keyword}"
            );
            assert!(is_schema_map_child_keyword(keyword));
            assert!(!is_single_schema_child_keyword(keyword));
            assert!(!is_schema_array_child_keyword(keyword));
        }

        for keyword in SCHEMA_ARRAY_CHILD_KEYWORDS {
            assert!(
                seen.insert(keyword),
                "duplicate keyword family member: {keyword}"
            );
            assert!(is_schema_array_child_keyword(keyword));
            assert!(!is_single_schema_child_keyword(keyword));
            assert!(!is_schema_map_child_keyword(keyword));
        }
    }
}
