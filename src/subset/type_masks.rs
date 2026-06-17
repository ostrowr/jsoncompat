//! Conservative JSON type-mask facts used by subset proof helpers.
//!
//! Masks are approximations: callers must treat missing precision as unknown.

use crate::SchemaNode;
use json_schema_ast::{NodeId, SchemaNodeKind};
use serde_json::Value;
use std::collections::HashSet;

use super::boolean::{
    JSON_TYPE_ALL, JSON_TYPE_ARRAY, JSON_TYPE_BOOL, JSON_TYPE_NULL, JSON_TYPE_NUMBER,
    JSON_TYPE_OBJECT, JSON_TYPE_STRING, schema_obviously_accepts_json_type,
};

pub(super) fn whole_json_types_accepted_mask(schema: &SchemaNode) -> u8 {
    [
        JSON_TYPE_NULL,
        JSON_TYPE_BOOL,
        JSON_TYPE_NUMBER,
        JSON_TYPE_STRING,
        JSON_TYPE_ARRAY,
        JSON_TYPE_OBJECT,
    ]
    .into_iter()
    .fold(0, |mask, bit| {
        if schema_obviously_accepts_json_type(schema, bit) {
            mask | bit
        } else {
            mask
        }
    })
}

/// Upper bound on the JSON types that may satisfy `not schema`. If `schema`
/// accepts every value of a type, the complement cannot contain that type;
/// otherwise keep the type as possible.
pub(super) fn complement_type_mask_upper_bound(schema: &SchemaNode) -> u8 {
    JSON_TYPE_ALL & !whole_json_types_accepted_mask(schema)
}

/// Return a sound upper bound on the JSON value types a schema may accept.
/// Disjoint upper bounds imply disjoint languages; overlapping bounds say
/// nothing. Applicators are handled with ordinary set algebra where it is
/// safe, and unknown cases fall back to all types.
pub(super) fn possible_json_type_mask(schema: &SchemaNode) -> u8 {
    fn value_mask(value: &Value) -> u8 {
        match value {
            Value::Null => JSON_TYPE_NULL,
            Value::Bool(_) => JSON_TYPE_BOOL,
            Value::Number(_) => JSON_TYPE_NUMBER,
            Value::String(_) => JSON_TYPE_STRING,
            Value::Array(_) => JSON_TYPE_ARRAY,
            Value::Object(_) => JSON_TYPE_OBJECT,
        }
    }

    fn inner(schema: &SchemaNode, active: &mut HashSet<NodeId>) -> u8 {
        if !active.insert(schema.id()) {
            return JSON_TYPE_ALL;
        }
        let mask = match schema.kind() {
            SchemaNodeKind::BoolSchema(false) => 0,
            SchemaNodeKind::BoolSchema(true) | SchemaNodeKind::Any => JSON_TYPE_ALL,
            SchemaNodeKind::String { enumeration, .. } => {
                enumeration.as_ref().map_or(JSON_TYPE_STRING, |values| {
                    values.iter().fold(0, |m, v| m | value_mask(v))
                })
            }
            SchemaNodeKind::Number { enumeration, .. }
            | SchemaNodeKind::Integer { enumeration, .. } => {
                enumeration.as_ref().map_or(JSON_TYPE_NUMBER, |values| {
                    values.iter().fold(0, |m, v| m | value_mask(v))
                })
            }
            SchemaNodeKind::Boolean { enumeration } => {
                enumeration.as_ref().map_or(JSON_TYPE_BOOL, |values| {
                    values.iter().fold(0, |m, v| m | value_mask(v))
                })
            }
            SchemaNodeKind::Null { enumeration } => {
                enumeration.as_ref().map_or(JSON_TYPE_NULL, |values| {
                    values.iter().fold(0, |m, v| m | value_mask(v))
                })
            }
            SchemaNodeKind::Object { enumeration, .. } => {
                enumeration.as_ref().map_or(JSON_TYPE_OBJECT, |values| {
                    values.iter().fold(0, |m, v| m | value_mask(v))
                })
            }
            SchemaNodeKind::Array { enumeration, .. } => {
                enumeration.as_ref().map_or(JSON_TYPE_ARRAY, |values| {
                    values.iter().fold(0, |m, v| m | value_mask(v))
                })
            }
            SchemaNodeKind::Const(value) => value_mask(value),
            SchemaNodeKind::Enum(values) => values.iter().fold(0, |m, v| m | value_mask(v)),
            SchemaNodeKind::AllOf(children) => children
                .iter()
                .fold(JSON_TYPE_ALL, |m, child| m & inner(child, active)),
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children) => {
                children.iter().fold(0, |m, child| m | inner(child, active))
            }
            SchemaNodeKind::Not(child) => match child.kind() {
                SchemaNodeKind::BoolSchema(true) => 0,
                SchemaNodeKind::BoolSchema(false) => JSON_TYPE_ALL,
                _ => {
                    // If the negated schema accepts *every* value of a JSON
                    // type, then `not` excludes that entire type. This is a
                    // cheap upper-bound refinement (for example, `not: {type:
                    // string}` cannot accept strings) and remains conservative
                    // for partial constraints such as `not: {minLength: 2}`.
                    complement_type_mask_upper_bound(child)
                }
            },
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => {
                // A conditional accepts values from the guarded-then side or
                // from the negated-guard else side. Include cheap type facts
                // from the guard itself; this keeps schemas like
                // `if type=string, then true, else number` from looking like
                // they may accept every JSON type.
                let guard_mask = inner(if_schema, active);
                let not_guard_mask = complement_type_mask_upper_bound(if_schema);
                let then_mask = then_schema
                    .as_ref()
                    .map_or(JSON_TYPE_ALL, |child| inner(child, active));
                let else_mask = else_schema
                    .as_ref()
                    .map_or(JSON_TYPE_ALL, |child| inner(child, active));
                (guard_mask & then_mask) | (not_guard_mask & else_mask)
            }
            _ => JSON_TYPE_ALL,
        };
        active.remove(&schema.id());
        mask
    }

    inner(schema, &mut HashSet::new())
}
