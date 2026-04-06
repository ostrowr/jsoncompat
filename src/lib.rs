//! Backward-compatibility checks for evolving JSON Schema documents.
//!
//! Build input documents with [`SchemaDocument::from_json`], then call
//! [`check_compat`] with a [`Role`]. This crate intentionally exposes only the
//! document-level compatibility API; lower-level resolved IR types live in
//! `json_schema_ast`.

// Re-export the document type needed by `check_compat` so application callers
// do not need a second direct dependency just to construct inputs.
use json_schema_ast::{NodeId, SchemaNode, SchemaNodeKind};
pub use json_schema_ast::{SchemaBuildError, SchemaDocument};
use std::collections::HashSet;

mod stamp;
mod subset;

pub use stamp::{
    ENVELOPE_DATA_KEY, ENVELOPE_VERSION_KEY, STAMP_MANIFEST_VERSION, SchemaHistory,
    SchemaVersionEntry, StampBundle, StampError, StampManifest, StampResult, StampStatus,
    canonical_schema_hash, stamp_schema, write_stamp_manifest_atomic,
};
use subset::is_subschema_of;

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
    /// Compatibility checks do not approximate fractional `number.multipleOf`
    /// inclusion with floating-point arithmetic.
    #[error("non-integral number multipleOf constraints are not supported by compatibility checks")]
    UnsupportedNonIntegralNumberMultipleOf,
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
    let old = old.root()?;
    let new = new.root()?;

    reject_unsupported_compatibility_features(old)?;
    reject_unsupported_compatibility_features(new)?;

    match role {
        Role::Serializer => Ok(is_subschema_of(new, old)),
        Role::Deserializer => Ok(is_subschema_of(old, new)),
        Role::Both => Ok(is_subschema_of(new, old) && is_subschema_of(old, new)),
    }
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
    use super::{CompatibilityError, Role, SchemaDocument, check_compat};
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
}
