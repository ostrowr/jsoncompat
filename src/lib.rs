//! Backward-compatibility checks for evolving JSON Schema documents and
//! OpenAPI 3.1 contracts.
//!
//! Build input documents with [`SchemaDocument::from_json`], then call
//! [`check_compat`] with a [`Role`]. This crate intentionally exposes only the
//! document-level JSON Schema compatibility API; lower-level resolved IR types
//! live in `json_schema_ast`. OpenAPI validation and lowering live in the
//! sibling `jsoncompat_openapi` crate; this crate layers compatibility reports
//! over those lowered request and response schemas.

// Re-export the document type needed by `check_compat` so application callers
// do not need a second direct dependency just to construct inputs.
use json_schema_ast::{
    SCHEMA_ARRAY_CHILD_KEYWORDS, SCHEMA_MAP_CHILD_KEYWORDS, SINGLE_SCHEMA_CHILD_KEYWORDS,
    SchemaNode,
};
pub use json_schema_ast::{SchemaBuildError, SchemaDocument, SchemaOptions};
use serde_json::{Map, Value};

mod compatibility_result;
mod json_pointer;
mod openapi_compat;
mod stamp;
mod subset;
pub use compatibility_result::{
    AnalysisOptions, CompatibilityResult, analyze_compat, analyze_compat_with_options,
};

pub use jsoncompat_openapi::{OpenApiDocument, OpenApiError, OpenApiLoweringError};
pub use openapi_compat::{
    OpenApiCompatibilityError, OpenApiCompatibilityIssue, OpenApiCompatibilityIssueKind,
    OpenApiCompatibilityReport, OpenApiCompatibilitySurface, check_openapi_compat,
    validate_openapi_compatibility_input,
};
pub use stamp::{
    ENVELOPE_DATA_KEY, ENVELOPE_VERSION_KEY, STAMP_MANIFEST_VERSION, SchemaHistory,
    SchemaVersionEntry, StampBundle, StampError, StampManifest, StampResult, StampStatus,
    canonical_schema_hash, stamp_schema, write_stamp_manifest_atomic,
};
use subset::{
    explain_subschema_failure, explain_subschema_failure_emitted_values, is_subschema_of,
    is_subschema_of_emitted_values,
};

/// The role under which a compatibility check is performed.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Evolving the *producer* (serializer).  A change is safe if every value
    /// produced by the _new_ schema is still accepted by the _old_ one.
    Serializer,
    /// Evolving the *consumer* (deserializer).  A change is safe if every value
    /// the old serializer could emit is still valid under the _new_ schema.
    Deserializer,
    /// We need to maintain full equivalence in both directions.
    Both,
}

/// Compatibility-check failures that are distinct from a proven incompatibility.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CompatibilityError {
    /// The old or new schema document failed canonicalization or resolution.
    #[error(transparent)]
    Schema(#[from] SchemaBuildError),
}

/// Return whether `new` is backward-compatible with `old` under `role`.
///
/// The checker is a structural subset proof over the documents' resolved
/// schema graphs:
///
/// * [`Role::Serializer`] checks `new ⊆ old`: every value produced under the
///   new schema must still be accepted by clients using the old schema.
/// * [`Role::Deserializer`] checks whether every value the old serializer
///   could emit is still accepted by the new schema.
/// * [`Role::Both`] requires both directions.
///
/// For [`Role::Deserializer`], object schemas are interpreted under the
/// assumption that serializers do not emit undeclared properties solely because
/// `additionalProperties` would permit them.
///
/// A return value of `Ok(false)` means no inclusion proof was found. Use
/// [`analyze_compat`] to distinguish a counterexample from an unknown result.
/// A return value of `Err(_)` means the checker cannot soundly run on the input
/// schema or feature set.
pub fn check_compat(
    old: &SchemaDocument,
    new: &SchemaDocument,
    role: Role,
) -> Result<bool, CompatibilityError> {
    let old = compatibility_input(old)?;
    let new = compatibility_input(new)?;

    match role {
        Role::Serializer => Ok(is_subschema_of(new, old)),
        Role::Deserializer => Ok(is_subschema_of_emitted_values(old, new)),
        Role::Both => Ok(is_subschema_of(new, old) && is_subschema_of_emitted_values(old, new)),
    }
}

/// Return a best-effort static explanation for the first incompatibility under
/// `role`, or `Ok(None)` when the checker finds no incompatibility to explain.
///
/// This diagnostic path is intentionally narrower than [`check_compat`]: it
/// preserves the sound compatibility verdict while surfacing the most useful
/// structural reason the checker can identify.
pub fn explain_compat_failure(
    old: &SchemaDocument,
    new: &SchemaDocument,
    role: Role,
) -> Result<Option<String>, CompatibilityError> {
    let old = compatibility_input(old)?;
    let new = compatibility_input(new)?;

    let explanation = match role {
        Role::Serializer => {
            explain_subschema_failure(new, old).map(|explanation| explanation.render("new", "old"))
        }
        Role::Deserializer => explain_subschema_failure_emitted_values(old, new)
            .map(|explanation| explanation.render("old", "new")),
        Role::Both => explain_subschema_failure(new, old)
            .map(|explanation| explanation.render("new", "old"))
            .or_else(|| {
                explain_subschema_failure_emitted_values(old, new)
                    .map(|explanation| explanation.render("old", "new"))
            }),
    };
    Ok(explanation)
}

/// Return whether this schema can participate in compatibility checks.
///
/// Valid unsupported proof cases remain [`CompatibilityResult::Unknown`].
pub fn validate_compatibility_input(schema: &SchemaDocument) -> Result<(), CompatibilityError> {
    compatibility_input(schema).map(|_| ())
}

fn compatibility_input(schema: &SchemaDocument) -> Result<&SchemaNode, CompatibilityError> {
    match schema.root() {
        Ok(root) => {
            schema.validate_source_schema()?;
            Ok(root)
        }
        Err(source @ SchemaBuildError::UnsupportedReference { .. }) => {
            validate_source_schema_ignoring_non_local_refs(schema)?;
            Err(source.into())
        }
        Err(source) => Err(source.into()),
    }
}

fn validate_source_schema_ignoring_non_local_refs(
    schema: &SchemaDocument,
) -> Result<(), CompatibilityError> {
    let stripped = strip_non_local_schema_refs(schema.source_schema_json());
    let stripped = SchemaDocument::from_json(&stripped)?;
    stripped.validate_source_schema()?;
    Ok(())
}

fn strip_non_local_schema_refs(schema: &Value) -> Value {
    match schema {
        Value::Object(object) => {
            let mut stripped = Map::new();
            for (key, value) in object {
                let stripped_value = match key.as_str() {
                    "$ref"
                        if value
                            .as_str()
                            .is_some_and(|reference| !reference.starts_with("#/")) =>
                    {
                        None
                    }
                    key if SINGLE_SCHEMA_CHILD_KEYWORDS.contains(&key) => {
                        Some(strip_non_local_schema_refs(value))
                    }
                    key if SCHEMA_MAP_CHILD_KEYWORDS.contains(&key) => {
                        Some(strip_non_local_schema_ref_map(value))
                    }
                    key if SCHEMA_ARRAY_CHILD_KEYWORDS.contains(&key) => {
                        Some(strip_non_local_schema_ref_array(value))
                    }
                    _ => Some(value.clone()),
                };
                if let Some(stripped_value) = stripped_value {
                    stripped.insert(key.clone(), stripped_value);
                }
            }
            Value::Object(stripped)
        }
        _ => schema.clone(),
    }
}

fn strip_non_local_schema_ref_map(value: &Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .iter()
                .map(|(name, schema)| (name.clone(), strip_non_local_schema_refs(schema)))
                .collect(),
        ),
        _ => value.clone(),
    }
}

fn strip_non_local_schema_ref_array(value: &Value) -> Value {
    match value {
        Value::Array(items) => {
            Value::Array(items.iter().map(strip_non_local_schema_refs).collect())
        }
        _ => value.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CompatibilityError, Role, SchemaBuildError, SchemaDocument, check_compat,
        validate_compatibility_input,
    };
    use serde_json::json;

    fn schema(raw: serde_json::Value) -> SchemaDocument {
        SchemaDocument::from_json(&raw).expect("schema should parse")
    }

    #[test]
    fn check_compat_compares_non_integral_number_multiple_of() {
        let old = schema(json!({ "type": "number", "multipleOf": 0.2 }));
        let new = schema(json!({ "type": "number" }));

        assert!(!check_compat(&old, &new, Role::Serializer).unwrap());
        assert!(check_compat(&old, &new, Role::Deserializer).unwrap());
    }

    #[test]
    fn check_compat_accepts_integral_number_multiple_of() {
        let old = schema(json!({ "type": "number", "multipleOf": 2 }));
        let new = schema(json!({ "type": "integer", "multipleOf": 4 }));

        assert!(
            check_compat(&old, &new, Role::Serializer)
                .expect("integral number multipleOf remains supported")
        );
    }

    #[test]
    fn check_compat_compares_number_bounds_beyond_the_exact_f64_integer_range() {
        let old = schema(json!({
            "type": "number",
            "maximum": 9_007_199_254_740_992_i64
        }));
        let new = schema(json!({
            "enum": [9_007_199_254_740_993_i64]
        }));

        assert!(new.is_valid(&json!(9_007_199_254_740_993_i64)).unwrap());
        assert!(!old.is_valid(&json!(9_007_199_254_740_993_i64)).unwrap());

        assert!(!check_compat(&old, &new, Role::Serializer).unwrap());
    }

    #[test]
    fn check_compat_keeps_large_integer_bounds_supported_exactly() {
        let old = schema(json!({
            "type": "integer",
            "maximum": 9_007_199_254_740_992_i64
        }));
        let new = schema(json!({
            "type": "integer",
            "maximum": 9_007_199_254_740_991_i64
        }));

        assert!(
            check_compat(&old, &new, Role::Serializer).expect("integer-only bounds remain exact")
        );
    }

    #[test]
    fn supported_keywords_do_not_warn() {
        for (raw, _pointer, _keyword) in [
            (
                json!({
                    "type": "object",
                    "properties": {
                        "payload": {
                            "unevaluatedProperties": false
                        }
                    }
                }),
                "#/properties/payload/unevaluatedProperties",
                "unevaluatedProperties",
            ),
            (
                json!({
                    "type": "object",
                    "dependencies": {
                        "kind": ["detail"]
                    }
                }),
                "#/dependencies",
                "dependencies",
            ),
            (
                json!({
                    "additionalItems": false
                }),
                "#/additionalItems",
                "additionalItems",
            ),
        ] {
            let old = schema(raw);
            validate_compatibility_input(&old).expect("schema should participate in analysis");
        }
    }

    #[test]
    fn check_compat_accepts_reference_scope_keywords() {
        for (raw, _pointer, _keyword) in [
            (
                json!({
                    "$id": "https://example.com/schemas/value.json",
                    "type": "string"
                }),
                "#/$id",
                "$id",
            ),
            (
                json!({
                    "$anchor": "value",
                    "type": "string"
                }),
                "#/$anchor",
                "$anchor",
            ),
            (
                json!({
                    "$dynamicRef": "#",
                    "type": "string"
                }),
                "#/$dynamicRef",
                "$dynamicRef",
            ),
            (
                json!({
                    "$dynamicAnchor": "value",
                    "type": "string"
                }),
                "#/$dynamicAnchor",
                "$dynamicAnchor",
            ),
        ] {
            let old = schema(raw);
            let new = schema(json!({}));

            assert!(!check_compat(&old, &new, Role::Both).unwrap());
        }
    }

    #[test]
    fn check_compat_accepts_reference_scope_keywords_inside_unused_defs() {
        let old = schema(json!({
            "$defs": {
                "Unused": {
                    "$id": "https://example.com/schemas/unused.json",
                    "type": "string"
                }
            },
            "type": "string"
        }));
        let new = schema(json!({ "type": "string" }));

        assert!(check_compat(&old, &new, Role::Both).unwrap());
    }

    #[test]
    fn supported_keywords_inside_unused_defs_do_not_warn() {
        let old = schema(json!({
            "$defs": {
                "Unused": {
                    "type": "object",
                    "unevaluatedProperties": false
                }
            },
            "type": "string"
        }));
        validate_compatibility_input(&old).expect("unused definitions should be supported");
    }

    #[test]
    fn check_compat_accepts_identical_schemas_with_unmodeled_keyword_warnings() {
        let old = schema(json!({
            "type": "object",
            "unevaluatedProperties": false
        }));
        let new = schema(json!({
            "type": "object",
            "unevaluatedProperties": false
        }));

        assert!(
            check_compat(&old, &new, Role::Both)
                .expect("unmodeled keywords should warn instead of failing the modeled verdict")
        );
    }

    #[test]
    fn check_compat_rejects_backend_invalid_ref_bearing_schemas_before_comparison() {
        let old = schema(json!({
            "$defs": {
                "Value": { "type": "string" }
            },
            "$ref": "#/$defs/Value",
            "deprecated": "eventually"
        }));
        let new = schema(json!({ "type": "string" }));

        let error = check_compat(&old, &new, Role::Serializer)
            .expect_err("raw-schema backend validation must still run after local refs resolve")
            .to_string();

        assert!(
            error.contains("failed to compile raw schema validator"),
            "{error}"
        );
    }

    #[test]
    fn check_compat_rejects_backend_invalid_identity_ref_bearing_schemas_before_comparison() {
        let old = schema(json!({
            "$id": "https://example.com/schemas/value.json",
            "type": "string",
            "deprecated": "eventually"
        }));
        let new = schema(json!({ "type": "string" }));

        let error = check_compat(&old, &new, Role::Serializer)
            .expect_err("raw-schema backend validation must run before unsupported identity refs")
            .to_string();

        assert!(
            error.contains("failed to compile raw schema validator"),
            "{error}"
        );
    }

    #[test]
    fn check_compat_rejects_backend_invalid_non_local_ref_bearing_schemas_before_comparison() {
        let old = schema(json!({
            "$ref": "https://example.com/schemas/value.json",
            "deprecated": "eventually"
        }));
        let new = schema(json!({ "type": "string" }));

        let error = check_compat(&old, &new, Role::Serializer)
            .expect_err("raw-schema backend validation must inspect siblings of non-local refs")
            .to_string();

        assert!(
            error.contains("failed to compile raw schema validator"),
            "{error}"
        );
    }

    #[test]
    fn check_compat_keeps_non_local_ref_errors_explicit_after_source_validation() {
        let old = schema(json!({
            "$ref": "https://example.com/schemas/value.json"
        }));
        let new = schema(json!({ "type": "string" }));

        let error = check_compat(&old, &new, Role::Serializer)
            .expect_err("non-local refs stay unsupported even after raw validation runs");

        assert!(matches!(
            error,
            CompatibilityError::Schema(SchemaBuildError::UnsupportedReference { ref_path })
                if ref_path == "https://example.com/schemas/value.json"
        ));
    }

    #[test]
    fn check_compat_does_not_treat_const_payload_keys_as_schema_keywords() {
        let old = schema(json!({
            "const": {
                "dependentSchemas": {
                    "kind": { "required": ["detail"] }
                }
            }
        }));
        let new = schema(json!({
            "const": {
                "dependentSchemas": {
                    "kind": { "required": ["detail"] }
                }
            }
        }));

        assert!(
            check_compat(&old, &new, Role::Both)
                .expect("const payload keys are data, not schema keywords")
        );
    }

    #[test]
    fn check_compat_treats_optional_added_property_as_deserializer_compatible() {
        let old = schema(json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "age": { "type": "integer" }
            },
            "required": ["name", "age"]
        }));
        let new = schema(json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "age": { "type": "integer" },
                "is_active": { "type": "boolean", "default": true }
            },
            "required": ["name", "age"]
        }));

        assert!(
            check_compat(&old, &new, Role::Deserializer)
                .expect("optional added property should remain deserializer-compatible")
        );
        assert!(
            check_compat(&old, &new, Role::Both)
                .expect("optional added property should remain compatible in both directions")
        );
    }
}
