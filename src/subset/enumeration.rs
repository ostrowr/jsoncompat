//! Small helpers for reading explicit enum caps from typed schema nodes.

use crate::SchemaNode;
use json_schema_ast::SchemaNodeKind;
use serde_json::Value;

pub(super) fn constrained_enumeration(schema: &SchemaNode) -> Option<&[Value]> {
    match schema.kind() {
        SchemaNodeKind::String {
            enumeration: Some(values),
            ..
        }
        | SchemaNodeKind::Number {
            enumeration: Some(values),
            ..
        }
        | SchemaNodeKind::Integer {
            enumeration: Some(values),
            ..
        }
        | SchemaNodeKind::Boolean {
            enumeration: Some(values),
        }
        | SchemaNodeKind::Null {
            enumeration: Some(values),
        }
        | SchemaNodeKind::Object {
            enumeration: Some(values),
            ..
        }
        | SchemaNodeKind::Array {
            enumeration: Some(values),
            ..
        } => Some(values),
        _ => None,
    }
}
