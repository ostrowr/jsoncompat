//! Structural subset checks over the resolved schema IR.
//!
//! `is_subschema_of(sub, sup)` answers whether every instance accepted by
//! `sub` is also accepted by `sup`.  The checker is intentionally conservative
//! for hard cases such as regex implication and `oneOf` on the right-hand side.

use crate::SchemaNode;
use json_schema_ast::{NodeId, SchemaNodeKind, json_values_equal};
use serde_json::Value;
use std::collections::HashSet;

mod array;
mod object;
mod scalar;

use scalar::{
    check_enum_inclusion, integer_constraints_subsumed_by_number, scalar_constraints_subsumed,
};

#[derive(Default)]
pub(super) struct SubschemaCheckContext {
    active_pairs: HashSet<(NodeId, NodeId)>,
    assume_subset_omits_undeclared_properties: bool,
}

impl SubschemaCheckContext {
    pub(super) fn superset_contains_value(&mut self, sup: &SchemaNode, value: &Value) -> bool {
        sup.accepts_value(value)
    }

    pub(super) fn superset_contains_value_set(
        &mut self,
        sup: &SchemaNode,
        values: &[Value],
    ) -> bool {
        values
            .iter()
            .all(|value| self.superset_contains_value(sup, value))
    }
}

/// Returns `true` if **every** instance that satisfies `sub` also satisfies
/// `sup`.
pub(crate) fn is_subschema_of(sub: &SchemaNode, sup: &SchemaNode) -> bool {
    is_subschema_of_with_context(sub, sup, &mut SubschemaCheckContext::default())
}

/// Variant of [`is_subschema_of`] that models serializer output rather than
/// full JSON Schema validity for the subset side.
///
/// In this mode, object schemas on the subset side are treated as if
/// undeclared extra properties are never emitted, even when
/// `additionalProperties` would permit them.
pub(crate) fn is_subschema_of_emitted_values(sub: &SchemaNode, sup: &SchemaNode) -> bool {
    is_subschema_of_with_context(
        sub,
        sup,
        &mut SubschemaCheckContext {
            active_pairs: HashSet::new(),
            assume_subset_omits_undeclared_properties: true,
        },
    )
}

pub(super) fn is_subschema_of_with_context(
    sub: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    if sub == sup {
        return true;
    }

    let recursion_key = (sub.id(), sup.id());
    if !context.active_pairs.insert(recursion_key) {
        return true;
    }

    use SchemaNodeKind::*;

    let is_subschema = match (sub.kind(), sup.kind()) {
        (BoolSchema(false), _) => true,
        (_, BoolSchema(true)) => true,
        (Any, Any) => true,
        (_, Any) => true,
        (Any, _) => false,

        // Keep sub-combinator handlers before sup-combinator handlers so when
        // both sides are unions we reason branch-wise on `sub` first.
        (AnyOf(subs), _) | (OneOf(subs), _) => subs
            .iter()
            .all(|branch| is_subschema_of_with_context(branch, sup, context)),
        (AllOf(subs), _) => subs
            .iter()
            .all(|schema| is_subschema_of_with_context(schema, sup, context)),

        (Enum(sub_e), Enum(sup_e)) => check_enum_inclusion(Some(sub_e), Some(sup_e)),
        (Enum(sub_e), _) => context.superset_contains_value_set(sup, sub_e),

        (Const(sub_value), Const(sup_value)) => json_values_equal(sub_value, sup_value),
        (Const(sub_value), _) => context.superset_contains_value(sup, sub_value),

        (_, AnyOf(sups)) => sups
            .iter()
            .any(|branch| is_subschema_of_with_context(sub, branch, context)),
        (_, OneOf(_)) => false,
        (_, AllOf(sups)) => sups
            .iter()
            .all(|schema| is_subschema_of_with_context(sub, schema, context)),

        (
            Number {
                enumeration: Some(sub_enum),
                ..
            },
            Enum(_),
        ) => context.superset_contains_value_set(sup, sub_enum),

        (_, Enum(_)) => false,

        (Not(sub_negated), _) => match sub_negated.kind() {
            Any | BoolSchema(true) => true,
            BoolSchema(false) => !matches!(sup.kind(), Any | BoolSchema(true)),
            _ => false,
        },
        (_, Not(sup_negated)) => match sup_negated.kind() {
            Any | BoolSchema(true) => matches!(sub.kind(), BoolSchema(false)),
            BoolSchema(false) => matches!(sub.kind(), BoolSchema(true) | Any),
            _ => false,
        },

        (String { .. }, String { .. })
        | (Number { .. }, Number { .. })
        | (Integer { .. }, Integer { .. })
        | (Boolean { .. }, Boolean { .. })
        | (Null { .. }, Null { .. })
        | (Object { .. }, Object { .. })
        | (Array { .. }, Array { .. }) => type_constraints_subsumed_with_context(sub, sup, context),

        (Integer { .. }, Number { .. }) => integer_constraints_subsumed_by_number(sub, sup),
        (
            Number {
                enumeration: Some(sub_enum),
                ..
            },
            Integer { .. } | Const(_),
        ) => context.superset_contains_value_set(sup, sub_enum),

        (_, Const(_)) => false,

        _ => false,
    };

    context.active_pairs.remove(&recursion_key);
    is_subschema
}

fn type_constraints_subsumed_with_context(
    sub: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    use SchemaNodeKind::*;

    match (sub.kind(), sup.kind()) {
        (
            String {
                length: sub_length,
                enumeration: sub_enum,
                ..
            },
            String {
                length: sup_length,
                enumeration: sup_enum,
                ..
            },
        ) => scalar_constraints_subsumed(
            *sub_length,
            sub_enum.as_deref(),
            *sup_length,
            sup_enum.as_deref(),
        ),

        (
            Number {
                bounds: sub_bounds,
                multiple_of: sub_multiple_of,
                enumeration: sub_enum,
            },
            Number {
                bounds: sup_bounds,
                multiple_of: sup_multiple_of,
                enumeration: sup_enum,
            },
        ) => scalar::number_constraints_subsumed(
            *sub_bounds,
            sub_multiple_of.as_ref(),
            sub_enum.as_deref(),
            *sup_bounds,
            sup_multiple_of.as_ref(),
            sup_enum.as_deref(),
        ),

        (
            Integer {
                bounds: sub_bounds,
                multiple_of: sub_multiple_of,
                enumeration: sub_enum,
            },
            Integer {
                bounds: sup_bounds,
                multiple_of: sup_multiple_of,
                enumeration: sup_enum,
            },
        ) => scalar::integer_constraints_subsumed(
            *sub_bounds,
            sub_multiple_of.as_ref(),
            sub_enum.as_deref(),
            *sup_bounds,
            sup_multiple_of.as_ref(),
            sup_enum.as_deref(),
        ),

        (
            Boolean {
                enumeration: sub_enum,
            },
            Boolean {
                enumeration: sup_enum,
            },
        )
        | (
            Null {
                enumeration: sub_enum,
            },
            Null {
                enumeration: sup_enum,
            },
        ) => check_enum_inclusion(sub_enum.as_deref(), sup_enum.as_deref()),

        (
            Object {
                properties: sub_properties,
                pattern_properties: sub_pattern_properties,
                required: sub_required,
                additional: sub_additional,
                property_names: sub_property_names,
                property_count: sub_property_count,
                dependent_required: _sub_dependent_required,
                enumeration: sub_enum,
            },
            Object {
                properties: sup_properties,
                pattern_properties: sup_pattern_properties,
                required: sup_required,
                additional: sup_additional,
                property_names: sup_property_names,
                property_count: sup_property_count,
                dependent_required: sup_dependent_required,
                enumeration: sup_enum,
            },
        ) => object::object_constraints_subsumed(
            object::ObjectConstraints {
                properties: sub_properties,
                pattern_properties: sub_pattern_properties,
                required: sub_required,
                additional: sub_additional,
                property_names: sub_property_names,
                property_count: *sub_property_count,
                dependent_required: _sub_dependent_required,
                enumeration: sub_enum.as_deref(),
            },
            object::ObjectConstraints {
                properties: sup_properties,
                pattern_properties: sup_pattern_properties,
                required: sup_required,
                additional: sup_additional,
                property_names: sup_property_names,
                property_count: *sup_property_count,
                dependent_required: sup_dependent_required,
                enumeration: sup_enum.as_deref(),
            },
            context,
        ),

        (
            Array {
                prefix_items: sub_prefix_items,
                items: sub_items,
                item_count: sub_item_count,
                contains: sub_contains,
                unique_items: sub_unique_items,
                enumeration: sub_enum,
            },
            Array {
                prefix_items: sup_prefix_items,
                items: sup_items,
                item_count: sup_item_count,
                contains: sup_contains,
                unique_items: sup_unique_items,
                enumeration: sup_enum,
            },
        ) => array::array_constraints_subsumed(
            array::ArrayConstraints {
                prefix_items: sub_prefix_items,
                items: sub_items,
                item_count: *sub_item_count,
                contains: sub_contains.as_ref(),
                unique_items: *sub_unique_items,
                enumeration: sub_enum.as_deref(),
            },
            array::ArrayConstraints {
                prefix_items: sup_prefix_items,
                items: sup_items,
                item_count: *sup_item_count,
                contains: sup_contains.as_ref(),
                unique_items: *sup_unique_items,
                enumeration: sup_enum.as_deref(),
            },
            context,
        ),

        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use json_schema_ast::SchemaDocument;
    use serde_json::json;

    fn resolve(raw: Value) -> SchemaNode {
        SchemaDocument::from_json(&raw)
            .unwrap()
            .root()
            .unwrap()
            .clone()
    }

    #[test]
    fn allof_tighten_subset() {
        let old = resolve(json!({
            "allOf": [
                {"type": "integer", "minimum": 0},
                {"maximum": 10}
            ]
        }));
        let new = resolve(json!({
            "type": "integer",
            "minimum": 0,
            "maximum": 5
        }));
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn exclusive_bounds_subset() {
        let old = resolve(json!({
            "minimum": 1,
            "exclusiveMinimum": 1
        }));
        let new = resolve(json!({
            "exclusiveMinimum": 1,
            "maximum": 3
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn enum_with_numeric_bound_is_not_subsumed_by_wider_enum() {
        let old = resolve(json!({
                "type": "number",
                "enum": [0, 1],
                "minimum": 1
        }));
        let new = resolve(json!({
                "enum": [0, 1]
        }));

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn enum_inclusion_uses_json_schema_numeric_equality() {
        let old = resolve(json!({
            "enum": [1.0, 2]
        }));
        let new = resolve(json!({
            "enum": [1, 2.0]
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn integer_multiple_of_must_be_at_least_as_constrained_as_number_multiple_of() {
        let old = resolve(json!({
                "type": "number",
                "multipleOf": 2
        }));
        let new = resolve(json!({
                "type": "integer",
                "multipleOf": 3
        }));

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn huge_integer_multiple_of_ratio_must_be_exactly_divisible() {
        let old = resolve(json!({
                "type": "integer",
                "multipleOf": 3
        }));
        let new = resolve(json!({
                "type": "integer",
                "multipleOf": 9_007_199_254_740_994_f64
        }));

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn integer_schema_without_enum_is_not_subsumed_by_old_number_enum() {
        let old = resolve(json!({
                "type": "number",
                "enum": [1],
                "minimum": 1
        }));
        let new = resolve(json!({
                "type": "integer",
                "minimum": 1,
                "maximum": 2
        }));

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn enum_is_not_checked_by_serializing_recursive_sup_schema() {
        let sup = resolve(json!({
            "$defs": {
                "Node": {
                    "type": "object",
                    "properties": {
                        "next": { "$ref": "#/$defs/Node" }
                    }
                }
            },
            "$ref": "#/$defs/Node"
        }));
        let sub = resolve(json!({
            "enum": [1]
        }));

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn enum_values_are_checked_against_recursive_sup_schema_structurally() {
        let sup = resolve(json!({
            "$defs": {
                "Node": {
                    "type": "object",
                    "properties": {
                        "next": { "$ref": "#/$defs/Node" }
                    }
                }
            },
            "$ref": "#/$defs/Node"
        }));
        let sub = resolve(json!({
            "enum": [{ "next": {} }]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn const_values_are_checked_against_recursive_sup_schema_structurally() {
        let sup = resolve(json!({
            "$defs": {
                "Node": {
                    "type": "object",
                    "properties": {
                        "value": { "type": "integer" },
                        "next": { "$ref": "#/$defs/Node" }
                    },
                    "required": ["value"]
                }
            },
            "$ref": "#/$defs/Node"
        }));
        let sub = resolve(json!({
            "const": {
                "value": 1,
                "next": {
                    "value": 2
                }
            }
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn const_values_reject_non_productive_recursive_sup_anyof() {
        let sup = resolve(json!({
            "$defs": {
                "A": {
                    "anyOf": [
                        { "type": "integer" },
                        { "$ref": "#/$defs/A" }
                    ]
                }
            },
            "$ref": "#/$defs/A"
        }));
        let sub = resolve(json!({
            "const": "x"
        }));

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn array_contains_lower_bound_must_hold_for_every_subset_instance() {
        let old = resolve(json!({
            "type": "array",
            "contains": { "type": "string" },
            "minContains": 2
        }));
        let new = resolve(json!({
            "type": "array",
            "items": { "type": "integer" },
            "minItems": 2
        }));

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn array_items_can_witness_contains_lower_bound_when_every_item_matches() {
        let old = resolve(json!({
            "type": "array",
            "contains": { "type": "string" },
            "minContains": 1
        }));
        let new = resolve(json!({
            "type": "array",
            "items": { "type": "string" },
            "minItems": 2
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn duplicate_oneof_branches_are_not_a_deserializer_superset_of_the_branch_schema() {
        let old = resolve(json!({
            "type": "string"
        }));
        let new = resolve(json!({
            "oneOf": [
                { "type": "string" },
                { "type": "string" }
            ]
        }));

        assert!(!is_subschema_of(&old, &new));
    }

    #[test]
    fn prefix_items_are_checked_positionally() {
        let old = resolve(json!({
            "type": "array",
            "prefixItems": [{ "type": "string" }],
            "minItems": 1
        }));
        let new = resolve(json!({
            "type": "array",
            "prefixItems": [{ "type": "integer" }],
            "minItems": 1
        }));

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn prefix_items_with_false_items_are_subsumed_by_max_items_cap() {
        let old = resolve(json!({
            "type": "array",
            "maxItems": 2
        }));
        let new = resolve(json!({
            "type": "array",
            "prefixItems": [
                { "const": 1 },
                { "const": 2 }
            ],
            "items": false
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn prefix_items_with_ref_false_items_are_subsumed_by_max_items_cap() {
        let old = resolve(json!({
            "type": "array",
            "maxItems": 1
        }));
        let new = resolve(json!({
            "$defs": {
                "Never": false
            },
            "type": "array",
            "prefixItems": [{ "const": 1 }],
            "items": {
                "$ref": "#/$defs/Never"
            }
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn single_item_tuples_with_false_items_satisfy_unique_items() {
        let old = resolve(json!({
            "type": "array",
            "uniqueItems": true
        }));
        let new = resolve(json!({
            "type": "array",
            "prefixItems": [{ "const": 1 }],
            "items": false
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn dependent_required_trigger_can_be_admitted_by_subset_pattern_properties() {
        let old = resolve(json!({
            "type": "object",
            "patternProperties": {
                "^x$": true
            },
            "dependentRequired": {
                "x": ["y"]
            },
            "additionalProperties": false
        }));
        let new = resolve(json!({
            "type": "object",
            "patternProperties": {
                "^x$": true
            },
            "additionalProperties": false
        }));

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn dependent_required_trigger_forbidden_by_subset_property_names_is_vacuous() {
        let old = resolve(json!({
            "type": "object",
            "dependentRequired": {
                "x": ["y"]
            }
        }));
        let new = resolve(json!({
            "type": "object",
            "propertyNames": {
                "pattern": "^z$"
            }
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn subset_pattern_properties_must_satisfy_matching_superset_properties() {
        let old = resolve(json!({
            "type": "object",
            "properties": {
                "x": false
            },
            "additionalProperties": true
        }));
        let new = resolve(json!({
            "type": "object",
            "patternProperties": {
                "^x$": true
            },
            "additionalProperties": false
        }));

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn subset_pattern_properties_do_not_shadow_matching_explicit_subset_properties() {
        let old = resolve(json!({
            "type": "object",
            "properties": {
                "x": { "type": "integer" }
            },
            "additionalProperties": true
        }));
        let new = resolve(json!({
            "type": "object",
            "properties": {
                "x": { "type": "integer" }
            },
            "patternProperties": {
                "^x$": true
            },
            "additionalProperties": false
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn subset_property_patterns_can_jointly_satisfy_a_matching_superset_property() {
        let old = resolve(json!({
            "type": "object",
            "properties": {
                "x": { "type": "integer" }
            },
            "additionalProperties": true
        }));
        let new = resolve(json!({
            "type": "object",
            "patternProperties": {
                "^x$": true,
                "^.$": { "type": "integer" }
            },
            "additionalProperties": false
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn subset_pattern_properties_can_tighten_matching_explicit_subset_properties() {
        let old = resolve(json!({
            "type": "object",
            "properties": {
                "x": { "type": "integer" }
            },
            "additionalProperties": true
        }));
        let new = resolve(json!({
            "type": "object",
            "properties": {
                "x": true
            },
            "patternProperties": {
                "^x$": { "type": "integer" }
            },
            "additionalProperties": false
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn subset_additional_properties_must_satisfy_unmatched_superset_properties() {
        let old = resolve(json!({
            "type": "object",
            "properties": {
                "x": false
            },
            "additionalProperties": true
        }));
        let new = resolve(json!({
            "type": "object",
            "additionalProperties": true
        }));

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn subset_pattern_properties_can_fall_back_to_superset_additional_properties() {
        let old = resolve(json!({
            "type": "object",
            "additionalProperties": true
        }));
        let new = resolve(json!({
            "type": "object",
            "patternProperties": {
                "^x$": {
                    "type": "integer"
                }
            }
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn subset_additional_properties_must_satisfy_unmatched_superset_pattern_properties() {
        let old = resolve(json!({
            "type": "object",
            "additionalProperties": true
        }));
        let new = resolve(json!({
            "type": "object",
            "patternProperties": {
                "^x$": {
                    "type": "integer"
                }
            }
        }));

        assert!(!is_subschema_of(&old, &new));
    }

    #[test]
    fn number_enum_with_float_form_integer_values_can_be_subsumed_by_integer_const() {
        let sub = resolve(json!({
            "type": "number",
            "minimum": 0,
            "enum": [9_007_199_254_740_994.0_f64]
        }));
        let sup = resolve(json!({
            "const": 9_007_199_254_740_994_i64
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn numeric_const_subset_uses_json_schema_number_equality() {
        let int_const = resolve(json!({
            "const": 1
        }));
        let float_const = resolve(json!({
            "const": 1.0
        }));

        assert!(is_subschema_of(&int_const, &float_const));
        assert!(is_subschema_of(&float_const, &int_const));
    }

    #[test]
    fn numeric_enum_subset_uses_json_schema_number_equality() {
        let int_enum = resolve(json!({
            "enum": [1]
        }));
        let float_enum = resolve(json!({
            "enum": [1.0]
        }));

        assert!(is_subschema_of(&int_enum, &float_enum));
        assert!(is_subschema_of(&float_enum, &int_enum));
    }
}
