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
use json_pointer::JsonPointer;
use json_schema_ast::{
    NodeId, SCHEMA_ARRAY_CHILD_KEYWORDS, SCHEMA_MAP_CHILD_KEYWORDS, SINGLE_SCHEMA_CHILD_KEYWORDS,
    SchemaNode, SchemaNodeKind,
};
pub use json_schema_ast::{SchemaBuildError, SchemaDocument};
use serde_json::{Map, Value};
use std::collections::HashSet;

mod json_pointer;
mod openapi_compat;
mod subset;

pub use jsoncompat_openapi::{OpenApiDocument, OpenApiError, OpenApiLoweringError};
pub use openapi_compat::{
    OpenApiCompatibilityError, OpenApiCompatibilityIssue, OpenApiCompatibilityReport,
    OpenApiCompatibilitySurface, check_openapi_compat, validate_openapi_compatibility_input,
};
use subset::{explain_subschema_failure, is_subschema_of};

/// The role under which a compatibility check is performed.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Role {
    /// Evolving the *producer* (serializer).  A change is safe if every value
    /// produced by the _new_ schema is still accepted by the _old_ one.
    Serializer,
    /// Evolving the *consumer* (deserializer).  A change is safe if every value
    /// accepted by the _old_ schema is still valid under the _new_ one.
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
    /// Reference-scope keywords change how the document resolves names. They
    /// stay hard errors until the resolver models those scopes precisely.
    #[error(
        "JSON Schema compatibility checks do not support keyword '{keyword}' at '{pointer}' yet"
    )]
    UnsupportedCompatibilityKeyword { pointer: String, keyword: String },
    /// Number-schema bounds beyond the adjacent-integer-safe `f64` range can
    /// collapse distinct JSON integers in the resolved IR, so subset proofs
    /// must fail before comparison rather than overclaim.
    #[error(
        "JSON Schema compatibility checks do not support number bound '{keyword}' at '{pointer}' outside the exact f64 integer range [-9007199254740991, 9007199254740991] yet"
    )]
    UnsupportedCompatibilityNumberBound { pointer: String, keyword: String },
    /// Compatibility checks do not approximate fractional `number.multipleOf`
    /// inclusion with floating-point arithmetic.
    #[error("non-integral number multipleOf constraints are not supported by compatibility checks")]
    UnsupportedNonIntegralNumberMultipleOf,
}

/// Compatibility diagnostics that do not prevent a modeled comparison.
///
/// These warnings mean a schema uses valid JSON Schema syntax whose semantics
/// are not represented by the subset checker yet. Callers that need a complete
/// contract verdict should surface the warning alongside the modeled result.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum CompatibilityWarning {
    #[error(
        "JSON Schema compatibility checks do not model keyword '{keyword}' at '{pointer}'; comparison ignores that keyword"
    )]
    UnsupportedKeyword { pointer: String, keyword: String },
}

/// Return whether `new` is backward-compatible with `old` under `role`.
///
/// The checker is a structural subset proof over the documents' resolved
/// schema graphs:
///
/// * [`Role::Serializer`] checks `new ⊆ old`: every value produced under the
///   new schema must still be accepted by clients using the old schema.
/// * [`Role::Deserializer`] checks `old ⊆ new`: every previously accepted
///   value must still be accepted by the new schema.
/// * [`Role::Both`] requires both directions.
///
/// A return value of `Ok(false)` is a proven or conservative incompatibility.
/// A return value of `Err(_)` means the checker cannot soundly run on the input
/// schema or feature set.
pub fn check_compat(
    old: &SchemaDocument,
    new: &SchemaDocument,
    role: Role,
) -> Result<bool, CompatibilityError> {
    let old = compatibility_input(old)?.root;
    let new = compatibility_input(new)?.root;

    match role {
        Role::Serializer => Ok(is_subschema_of(new, old)),
        Role::Deserializer => Ok(is_subschema_of(old, new)),
        Role::Both => Ok(is_subschema_of(new, old) && is_subschema_of(old, new)),
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
    let old = compatibility_input(old)?.root;
    let new = compatibility_input(new)?.root;

    let explanation =
        match role {
            Role::Serializer => explain_subschema_failure(new, old)
                .map(|explanation| explanation.render("new", "old")),
            Role::Deserializer => explain_subschema_failure(old, new)
                .map(|explanation| explanation.render("old", "new")),
            Role::Both => explain_subschema_failure(new, old)
                .map(|explanation| explanation.render("new", "old"))
                .or_else(|| {
                    explain_subschema_failure(old, new)
                        .map(|explanation| explanation.render("old", "new"))
                }),
        };
    Ok(explanation)
}

/// Return whether this schema can participate in compatibility checks.
///
/// `SchemaDocument::from_json` accepts the full document-level schema surface
/// modeled by the schema frontend. Compatibility still rejects inputs that
/// would make the subset check unsound; valid-but-unmodeled keywords are
/// reported separately through [`compatibility_warnings`].
pub fn validate_compatibility_input(schema: &SchemaDocument) -> Result<(), CompatibilityError> {
    compatibility_input(schema).map(|_| ())
}

/// Return non-fatal compatibility diagnostics for one schema document.
pub fn compatibility_warnings(
    schema: &SchemaDocument,
) -> Result<Vec<CompatibilityWarning>, CompatibilityError> {
    compatibility_input(schema).map(|input| input.warnings)
}

struct CompatibilityInput<'a> {
    root: &'a SchemaNode,
    warnings: Vec<CompatibilityWarning>,
}

fn compatibility_input(
    schema: &SchemaDocument,
) -> Result<CompatibilityInput<'_>, CompatibilityError> {
    let warnings = collect_source_keyword_warnings(schema)?;
    match schema.root() {
        Ok(root) => {
            schema.validate_source_schema()?;
            reject_unsupported_reference_keywords_after_source_validation(schema)?;
            reject_unsupported_compatibility_features(root)?;
            Ok(CompatibilityInput { root, warnings })
        }
        Err(source @ SchemaBuildError::UnsupportedReference { .. }) => {
            validate_source_schema_ignoring_non_local_refs(schema)?;
            reject_unsupported_reference_keywords_after_source_validation(schema)?;
            Err(source.into())
        }
        Err(source) => Err(source.into()),
    }
}

const UNSUPPORTED_COMPATIBILITY_KEYWORDS: &[&str] = &[
    "additionalItems",
    "contentEncoding",
    "contentMediaType",
    "contentSchema",
    "dependencies",
    "dependentSchemas",
    "unevaluatedItems",
    "unevaluatedProperties",
];
const UNSUPPORTED_COMPATIBILITY_REFERENCE_KEYWORDS: &[&str] =
    &["$id", "$anchor", "$dynamicRef", "$dynamicAnchor"];
const MAX_EXACT_F64_INTEGER: f64 = 9_007_199_254_740_991.0;

fn collect_source_keyword_warnings(
    schema: &SchemaDocument,
) -> Result<Vec<CompatibilityWarning>, CompatibilityError> {
    reject_unsafe_number_bounds_in_schema_value(
        schema.source_schema_json(),
        &mut JsonPointer::root(),
    )?;
    Ok(collect_unsupported_keyword_family(
        schema.source_schema_json(),
        UNSUPPORTED_COMPATIBILITY_KEYWORDS,
    ))
}

fn reject_unsupported_reference_keywords_after_source_validation(
    schema: &SchemaDocument,
) -> Result<(), CompatibilityError> {
    reject_unsupported_keyword_family(
        schema.source_schema_json(),
        UNSUPPORTED_COMPATIBILITY_REFERENCE_KEYWORDS,
    )
}

fn reject_unsupported_keyword_family(
    schema: &Value,
    keywords: &[&str],
) -> Result<(), CompatibilityError> {
    if let Some(CompatibilityWarning::UnsupportedKeyword { pointer, keyword }) =
        collect_unsupported_keyword_family(schema, keywords)
            .into_iter()
            .next()
    {
        return Err(CompatibilityError::UnsupportedCompatibilityKeyword { pointer, keyword });
    }

    Ok(())
}

fn collect_unsupported_keyword_family(
    schema: &Value,
    keywords: &[&str],
) -> Vec<CompatibilityWarning> {
    let mut warnings = Vec::new();
    collect_unsupported_keywords_in_schema_value(
        schema,
        &mut JsonPointer::root(),
        keywords,
        &mut warnings,
    );
    warnings
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

fn collect_unsupported_keywords_in_schema_value(
    schema: &Value,
    pointer: &mut JsonPointer,
    keywords: &[&str],
    warnings: &mut Vec<CompatibilityWarning>,
) {
    match schema {
        Value::Bool(_) => {}
        Value::Object(object) => {
            collect_unsupported_keywords_in_schema_object(object, pointer, keywords, warnings)
        }
        _ => {}
    }
}

fn collect_unsupported_keywords_in_schema_object(
    object: &Map<String, Value>,
    pointer: &mut JsonPointer,
    keywords: &[&str],
    warnings: &mut Vec<CompatibilityWarning>,
) {
    for keyword in keywords {
        if object.contains_key(*keyword) {
            let mut keyword_pointer = pointer.clone();
            keyword_pointer.push(*keyword);
            warnings.push(CompatibilityWarning::UnsupportedKeyword {
                pointer: keyword_pointer.render(),
                keyword: (*keyword).to_owned(),
            });
        }
    }

    for keyword in SINGLE_SCHEMA_CHILD_KEYWORDS {
        if let Some(child) = object.get(keyword) {
            pointer.push(keyword);
            collect_unsupported_keywords_in_schema_value(child, pointer, keywords, warnings);
            pointer.pop();
        }
    }

    for keyword in SCHEMA_MAP_CHILD_KEYWORDS {
        if let Some(children) = object.get(keyword).and_then(Value::as_object) {
            pointer.push(keyword);
            for (name, child) in children {
                pointer.push(name);
                collect_unsupported_keywords_in_schema_value(child, pointer, keywords, warnings);
                pointer.pop();
            }
            pointer.pop();
        }
    }

    for keyword in SCHEMA_ARRAY_CHILD_KEYWORDS {
        if let Some(children) = object.get(keyword).and_then(Value::as_array) {
            pointer.push(keyword);
            for (index, child) in children.iter().enumerate() {
                pointer.push(index.to_string());
                collect_unsupported_keywords_in_schema_value(child, pointer, keywords, warnings);
                pointer.pop();
            }
            pointer.pop();
        }
    }
}

fn reject_unsafe_number_bounds_in_schema_value(
    schema: &Value,
    pointer: &mut JsonPointer,
) -> Result<(), CompatibilityError> {
    match schema {
        Value::Bool(_) => Ok(()),
        Value::Object(object) => reject_unsafe_number_bounds_in_schema_object_tree(object, pointer),
        _ => Ok(()),
    }
}

fn reject_unsafe_number_bounds_in_schema_object_tree(
    object: &Map<String, Value>,
    pointer: &mut JsonPointer,
) -> Result<(), CompatibilityError> {
    reject_unsafe_number_bounds_in_schema_object(object, pointer)?;

    for keyword in SINGLE_SCHEMA_CHILD_KEYWORDS {
        if let Some(child) = object.get(keyword) {
            pointer.push(keyword);
            reject_unsafe_number_bounds_in_schema_value(child, pointer)?;
            pointer.pop();
        }
    }

    for keyword in SCHEMA_MAP_CHILD_KEYWORDS {
        if let Some(children) = object.get(keyword).and_then(Value::as_object) {
            pointer.push(keyword);
            for (name, child) in children {
                pointer.push(name);
                reject_unsafe_number_bounds_in_schema_value(child, pointer)?;
                pointer.pop();
            }
            pointer.pop();
        }
    }

    for keyword in SCHEMA_ARRAY_CHILD_KEYWORDS {
        if let Some(children) = object.get(keyword).and_then(Value::as_array) {
            pointer.push(keyword);
            for (index, child) in children.iter().enumerate() {
                pointer.push(index.to_string());
                reject_unsafe_number_bounds_in_schema_value(child, pointer)?;
                pointer.pop();
            }
            pointer.pop();
        }
    }

    Ok(())
}

fn reject_unsafe_number_bounds_in_schema_object(
    object: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), CompatibilityError> {
    if schema_object_has_integer_only_numeric_domain(object) {
        return Ok(());
    }

    for keyword in ["minimum", "maximum", "exclusiveMinimum", "exclusiveMaximum"] {
        let Some(value) = object.get(keyword) else {
            continue;
        };
        if !number_bound_is_outside_exact_f64_integer_range(value) {
            continue;
        }

        let mut keyword_pointer = pointer.clone();
        keyword_pointer.push(keyword);
        return Err(CompatibilityError::UnsupportedCompatibilityNumberBound {
            pointer: keyword_pointer.render(),
            keyword: keyword.to_owned(),
        });
    }

    Ok(())
}

fn schema_object_has_integer_only_numeric_domain(object: &Map<String, Value>) -> bool {
    match object.get("type") {
        Some(Value::String(schema_type)) => schema_type == "integer",
        Some(Value::Array(schema_types)) => {
            let mut has_integer = false;
            for schema_type in schema_types {
                let Some(schema_type) = schema_type.as_str() else {
                    return false;
                };
                match schema_type {
                    "integer" => has_integer = true,
                    "number" => return false,
                    _ => {}
                }
            }
            has_integer
        }
        _ => false,
    }
}

fn number_bound_is_outside_exact_f64_integer_range(value: &Value) -> bool {
    value
        .as_f64()
        .is_some_and(|value| value.is_finite() && value.abs() > MAX_EXACT_F64_INTEGER)
}

fn reject_unsupported_compatibility_features(
    schema: &SchemaNode,
) -> Result<(), CompatibilityError> {
    reject_unsupported_node(schema, &mut HashSet::new())
}

fn reject_unsupported_node(
    schema: &SchemaNode,
    visited_nodes: &mut HashSet<NodeId>,
) -> Result<(), CompatibilityError> {
    if !visited_nodes.insert(schema.id()) {
        return Ok(());
    }

    match schema.kind() {
        SchemaNodeKind::Number {
            multiple_of: Some(multiple_of),
            ..
        } if !multiple_of.is_integer_valued() => {
            return Err(CompatibilityError::UnsupportedNonIntegralNumberMultipleOf);
        }
        SchemaNodeKind::Object {
            properties,
            pattern_properties,
            additional,
            property_names,
            ..
        } => {
            for property in properties.values() {
                reject_unsupported_node(property, visited_nodes)?;
            }
            for property in pattern_properties.values() {
                reject_unsupported_node(&property.schema, visited_nodes)?;
            }
            reject_unsupported_node(additional, visited_nodes)?;
            reject_unsupported_node(property_names, visited_nodes)?;
        }
        SchemaNodeKind::Array {
            prefix_items,
            items,
            contains,
            ..
        } => {
            for prefix_item in prefix_items {
                reject_unsupported_node(prefix_item, visited_nodes)?;
            }
            reject_unsupported_node(items, visited_nodes)?;
            if let Some(contains) = contains {
                reject_unsupported_node(&contains.schema, visited_nodes)?;
            }
        }
        SchemaNodeKind::AllOf(subschemas)
        | SchemaNodeKind::AnyOf(subschemas)
        | SchemaNodeKind::OneOf(subschemas) => {
            for subschema in subschemas {
                reject_unsupported_node(subschema, visited_nodes)?;
            }
        }
        SchemaNodeKind::Not(subschema) => reject_unsupported_node(subschema, visited_nodes)?,
        SchemaNodeKind::IfThenElse {
            if_schema,
            then_schema,
            else_schema,
        } => {
            reject_unsupported_node(if_schema, visited_nodes)?;
            if let Some(then_schema) = then_schema {
                reject_unsupported_node(then_schema, visited_nodes)?;
            }
            if let Some(else_schema) = else_schema {
                reject_unsupported_node(else_schema, visited_nodes)?;
            }
        }
        _ => {}
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        CompatibilityError, CompatibilityWarning, Role, SchemaBuildError, SchemaDocument,
        check_compat, compatibility_warnings,
    };
    use serde_json::json;

    fn schema(raw: serde_json::Value) -> SchemaDocument {
        SchemaDocument::from_json(&raw).expect("schema should parse")
    }

    #[test]
    fn check_compat_rejects_non_integral_number_multiple_of() {
        let old = schema(json!({ "type": "number", "multipleOf": 0.2 }));
        let new = schema(json!({ "type": "number" }));

        let error = check_compat(&old, &new, Role::Serializer)
            .expect_err("non-integral number multipleOf is unsupported");
        assert!(matches!(
            error,
            CompatibilityError::UnsupportedNonIntegralNumberMultipleOf
        ));
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
    fn check_compat_rejects_number_bounds_beyond_the_exact_f64_integer_range() {
        let old = schema(json!({
            "type": "number",
            "maximum": 9_007_199_254_740_992_i64
        }));
        let new = schema(json!({
            "enum": [9_007_199_254_740_993_i64]
        }));

        assert!(new.is_valid(&json!(9_007_199_254_740_993_i64)).unwrap());
        assert!(!old.is_valid(&json!(9_007_199_254_740_993_i64)).unwrap());

        let error = check_compat(&old, &new, Role::Serializer)
            .expect_err("unsafe number bounds must fail before subset comparison");
        assert!(matches!(
            error,
            CompatibilityError::UnsupportedCompatibilityNumberBound {
                ref pointer,
                ref keyword,
            } if pointer == "#/maximum" && keyword == "maximum"
        ));
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
    fn compatibility_warnings_report_valid_but_unmodeled_schema_keywords_with_precise_pointers() {
        for (raw, pointer, keyword) in [
            (
                json!({
                    "type": "object",
                    "properties": {
                        "payload": {
                            "dependentSchemas": {
                                "kind": { "required": ["detail"] }
                            }
                        }
                    }
                }),
                "#/properties/payload/dependentSchemas",
                "dependentSchemas",
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
            let warnings = compatibility_warnings(&old).expect("warning collection should succeed");

            assert_eq!(
                warnings,
                vec![CompatibilityWarning::UnsupportedKeyword {
                    pointer: pointer.to_owned(),
                    keyword: keyword.to_owned(),
                }]
            );
        }
    }

    #[test]
    fn check_compat_rejects_reference_scope_keywords_with_precise_pointers() {
        for (raw, pointer, keyword) in [
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

            let error = check_compat(&old, &new, Role::Both)
                .expect_err("reference-scope keywords must remain hard compatibility errors");
            assert!(matches!(
                error,
                CompatibilityError::UnsupportedCompatibilityKeyword {
                    pointer: ref actual_pointer,
                    keyword: ref actual_keyword,
                } if actual_pointer == pointer && actual_keyword == keyword
            ));
        }
    }

    #[test]
    fn check_compat_rejects_reference_scope_keywords_inside_unused_defs() {
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

        let error = check_compat(&old, &new, Role::Both)
            .expect_err("unused defs must not hide unsupported reference-scope keywords");
        assert!(matches!(
            error,
            CompatibilityError::UnsupportedCompatibilityKeyword {
                pointer: ref actual_pointer,
                keyword: ref actual_keyword,
            } if actual_pointer == "#/$defs/Unused/$id" && actual_keyword == "$id"
        ));
    }

    #[test]
    fn compatibility_warnings_report_valid_but_unmodeled_keywords_inside_unused_defs() {
        let old = schema(json!({
            "$defs": {
                "Unused": {
                    "type": "object",
                    "dependentSchemas": {
                        "kind": { "required": ["detail"] }
                    }
                }
            },
            "type": "string"
        }));
        let warnings =
            compatibility_warnings(&old).expect("warning collection should inspect unused defs");

        assert_eq!(
            warnings,
            vec![CompatibilityWarning::UnsupportedKeyword {
                pointer: "#/$defs/Unused/dependentSchemas".to_owned(),
                keyword: "dependentSchemas".to_owned(),
            }]
        );
    }

    #[test]
    fn check_compat_accepts_identical_schemas_with_unmodeled_keyword_warnings() {
        let old = schema(json!({
            "type": "object",
            "dependentSchemas": {
                "kind": { "required": ["detail"] }
            }
        }));
        let new = schema(json!({
            "type": "object",
            "dependentSchemas": {
                "kind": { "required": ["detail"] }
            }
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
}
