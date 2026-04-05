//! Back‑compatibility checking library.
//!
//! This crate depends on `json_schema_ast`, which provides a strict
//! in‑memory representation (`SchemaDocument` / `SchemaNode`) of a
//! Draft 2020‑12 JSON Schema.  The only responsibility of this crate is to
//! offer algorithms that compare two schemas and decide whether a change is
//! backward‑compatible from the point of view of a serializer or deserializer.

// Re‑export the fundamental building blocks from the core schema crate so that
// downstream crates can just depend on *this* crate for both parsing and
// compatibility checking if they wish.
pub use json_schema_ast::{
    ContainsConstraint, CountRange, IntegerBounds, NodeId, NumberBound, NumberBounds,
    PatternConstraint, PatternProperty, PatternSupport, SchemaBuildError, SchemaDocument,
    SchemaNode, SchemaNodeKind,
};
use std::collections::HashSet;

mod subset;

pub use subset::{is_subschema_of, type_constraints_subsumed};

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
    #[error(transparent)]
    Schema(#[from] SchemaBuildError),
    #[error("non-integral number multipleOf constraints are not supported by compatibility checks")]
    UnsupportedNonIntegralNumberMultipleOf,
}

/// Top‑level convenience wrapper:
///
/// * `Role::Serializer`   ⇒   `new ⊆ old`
/// * `Role::Deserializer` ⇒   `old ⊆ new`
/// * `Role::Both`         ⇒   bidirectional inclusion
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
