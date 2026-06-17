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
fn allof_subset_can_be_proven_by_a_single_conjunct() {
    let old = resolve(json!({
        "type": "number",
        "minimum": 0
    }));
    let new = resolve(json!({
        "allOf": [
            { "type": "number", "minimum": 1, "maximum": 5 },
            { "type": "number", "maximum": 10 }
        ]
    }));

    assert!(is_subschema_of(&new, &old));
}

#[test]
fn allof_subset_proof_can_ignore_an_unhelpful_sibling_conjunct() {
    let old = resolve(json!({
        "type": "object",
        "properties": {
            "id": { "type": "string" }
        },
        "required": ["id"],
        "additionalProperties": false
    }));
    let new = resolve(json!({
        "allOf": [
            {
                "type": "object",
                "properties": {
                    "id": { "type": "string" }
                },
                "required": ["id"],
                "additionalProperties": false
            },
            {
                "type": "object",
                "properties": {
                    "trace": { "type": "string" }
                }
            }
        ]
    }));

    assert!(is_subschema_of(&new, &old));
}

#[test]
fn allof_subset_proof_does_not_invent_a_useful_conjunct() {
    let old = resolve(json!({ "type": "string" }));
    let new = resolve(json!({
        "allOf": [
            { "minLength": 1 },
            { "pattern": "^x$" }
        ]
    }));

    assert!(!is_subschema_of(&new, &old));
}

#[test]
fn not_false_behaves_like_any_schema_for_subset_checks() {
    let any = resolve(json!({ "not": false }));
    let string = resolve(json!({ "type": "string" }));

    assert!(is_subschema_of(&string, &any));
    assert!(!is_subschema_of(&any, &string));
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
fn constrained_number_enum_ignores_dead_literals_when_proving_subset() {
    let old = resolve(json!({
        "type": "number",
        "enum": [1]
    }));
    let new = resolve(json!({
        "type": "number",
        "enum": [0, 1],
        "minimum": 1
    }));

    assert!(is_subschema_of(&new, &old));
    assert!(is_subschema_of(&old, &new));
}

#[test]
fn large_number_bounds_do_not_prune_live_enum_literals() {
    let raw = json!({
        "type": "number",
        "minimum": 9_007_199_254_740_993_i64,
        "enum": [9_007_199_254_740_993_i64]
    });
    let sub_document = SchemaDocument::from_json(&raw).unwrap();
    assert!(
        sub_document
            .is_valid(&json!(9_007_199_254_740_993_i64))
            .unwrap()
    );

    let sub = sub_document.root().unwrap().clone();
    let sup = resolve(json!({ "type": "integer", "enum": [0] }));

    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn constrained_enum_with_unsupported_pattern_is_not_treated_as_empty() {
    let raw = json!({
        "type": "string",
        "pattern": "^\\cC$",
        "enum": ["\u{3}"]
    });
    let sub_document = SchemaDocument::from_json(&raw).unwrap();
    assert!(sub_document.is_valid(&json!("\u{3}")).unwrap());

    let sub = sub_document.root().unwrap().clone();
    let sup = resolve(json!({ "type": "integer" }));

    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn nested_unsupported_patterns_keep_object_enum_literals_live() {
    let raw = json!({
        "type": "object",
        "properties": {
            "value": {
                "type": "string",
                "pattern": "^\\cC$"
            }
        },
        "required": ["value"],
        "additionalProperties": false,
        "enum": [{ "value": "\u{3}" }]
    });
    let sub_document = SchemaDocument::from_json(&raw).unwrap();
    assert!(sub_document.is_valid(&json!({ "value": "\u{3}" })).unwrap());

    let sub = sub_document.root().unwrap().clone();
    let sup = resolve(json!({
        "type": "object",
        "properties": {
            "value": { "type": "integer" }
        },
        "required": ["value"],
        "additionalProperties": false
    }));

    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn nested_unsupported_patterns_keep_array_enum_literals_live() {
    let raw = json!({
        "type": "array",
        "prefixItems": [{
            "type": "string",
            "pattern": "^\\cC$"
        }],
        "items": false,
        "enum": [["\u{3}"]]
    });
    let sub_document = SchemaDocument::from_json(&raw).unwrap();
    assert!(sub_document.is_valid(&json!(["\u{3}"])).unwrap());

    let sub = sub_document.root().unwrap().clone();
    let sup = resolve(json!({
        "type": "array",
        "prefixItems": [{ "type": "integer" }],
        "items": false
    }));

    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn unsupported_contains_patterns_keep_array_enum_literals_live() {
    let raw = json!({
        "type": "array",
        "contains": {
            "type": "string",
            "pattern": "^\\cC$"
        },
        "minContains": 1,
        "enum": [["\u{3}"]]
    });
    let sub_document = SchemaDocument::from_json(&raw).unwrap();
    assert!(sub_document.is_valid(&json!(["\u{3}"])).unwrap());

    let sub = sub_document.root().unwrap().clone();
    let sup = resolve(json!({
        "type": "array",
        "contains": { "type": "integer" },
        "minContains": 1
    }));

    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn unsupported_pattern_property_matchers_keep_object_enum_literals_live() {
    let raw = json!({
        "type": "object",
        "patternProperties": {
            "^\\cC$": { "type": "integer" }
        },
        "additionalProperties": false,
        "enum": [{ "\u{3}": 1 }]
    });
    let sub_document = SchemaDocument::from_json(&raw).unwrap();
    assert!(sub_document.is_valid(&json!({ "\u{3}": 1 })).unwrap());

    let sub = sub_document.root().unwrap().clone();
    let sup = resolve(json!({
        "type": "object",
        "patternProperties": {
            "^\\cC$": { "type": "string" }
        },
        "additionalProperties": false
    }));

    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn unsupported_superset_pattern_properties_do_not_overaccept_object_enum_literals() {
    let sub_document = SchemaDocument::from_json(&json!({
        "type": "object",
        "enum": [{ "\u{3}": "not an integer" }]
    }))
    .unwrap();
    let sup_document = SchemaDocument::from_json(&json!({
        "type": "object",
        "patternProperties": {
            "^\\cC$": { "type": "integer" }
        },
        "additionalProperties": true
    }))
    .unwrap();
    let witness = json!({ "\u{3}": "not an integer" });

    assert!(sub_document.is_valid(&witness).unwrap());
    assert!(!sup_document.is_valid(&witness).unwrap());

    let sub = sub_document.root().unwrap().clone();
    let sup = sup_document.root().unwrap().clone();
    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn negated_unsupported_patterns_do_not_overaccept_enum_literals() {
    let sub_document = SchemaDocument::from_json(&json!({
        "type": "string",
        "enum": ["\u{3}"]
    }))
    .unwrap();
    let sup_document = SchemaDocument::from_json(&json!({
        "not": {
            "type": "string",
            "pattern": "^\\cC$"
        }
    }))
    .unwrap();
    let witness = json!("\u{3}");

    assert!(sub_document.is_valid(&witness).unwrap());
    assert!(!sup_document.is_valid(&witness).unwrap());

    let sub = sub_document.root().unwrap().clone();
    let sup = sup_document.root().unwrap().clone();
    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn one_of_with_unsupported_patterns_does_not_overaccept_enum_literals() {
    let sub_document = SchemaDocument::from_json(&json!({
        "type": "string",
        "enum": ["\u{3}"]
    }))
    .unwrap();
    let sup_document = SchemaDocument::from_json(&json!({
        "oneOf": [
            {
                "type": "string",
                "pattern": "^\\cC$"
            },
            { "const": "\u{3}" }
        ]
    }))
    .unwrap();
    let witness = json!("\u{3}");

    assert!(sub_document.is_valid(&witness).unwrap());
    assert!(!sup_document.is_valid(&witness).unwrap());

    let sub = sub_document.root().unwrap().clone();
    let sup = sup_document.root().unwrap().clone();
    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn conditionals_with_unsupported_patterns_do_not_overaccept_enum_literals() {
    let sub_document = SchemaDocument::from_json(&json!({
        "type": "string",
        "enum": ["\u{3}"]
    }))
    .unwrap();
    let sup_document = SchemaDocument::from_json(&json!({
        "if": {
            "type": "string",
            "pattern": "^\\cC$"
        },
        "then": false,
        "else": true
    }))
    .unwrap();
    let witness = json!("\u{3}");

    assert!(sub_document.is_valid(&witness).unwrap());
    assert!(!sup_document.is_valid(&witness).unwrap());

    let sub = sub_document.root().unwrap().clone();
    let sup = sup_document.root().unwrap().clone();
    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn max_contains_with_unsupported_patterns_does_not_overaccept_enum_literals() {
    let sub_document = SchemaDocument::from_json(&json!({
        "type": "array",
        "enum": [["\u{3}"]]
    }))
    .unwrap();
    let sup_document = SchemaDocument::from_json(&json!({
        "type": "array",
        "contains": {
            "type": "string",
            "pattern": "^\\cC$"
        },
        "maxContains": 0
    }))
    .unwrap();
    let witness = json!(["\u{3}"]);

    assert!(sub_document.is_valid(&witness).unwrap());
    assert!(!sup_document.is_valid(&witness).unwrap());

    let sub = sub_document.root().unwrap().clone();
    let sup = sup_document.root().unwrap().clone();
    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn recursive_enum_schemas_do_not_recurse_forever_while_scanning_patterns() {
    let raw = json!({
        "$defs": {
            "node": {
                "type": "object",
                "properties": {
                    "next": { "$ref": "#/$defs/node" }
                },
                "additionalProperties": false,
                "enum": [{}]
            }
        },
        "$ref": "#/$defs/node"
    });
    let sub_document = SchemaDocument::from_json(&raw).unwrap();
    assert!(sub_document.is_valid(&json!({})).unwrap());

    let sub = sub_document.root().unwrap().clone();
    let sup = resolve(json!({ "type": "integer" }));

    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn recursive_allof_enum_literals_stay_live_when_raw_validation_accepts_them() {
    let raw = json!({
        "$defs": {
            "Value": {
                "allOf": [
                    { "$ref": "#/$defs/Value" },
                    { "type": "string" }
                ]
            }
        },
        "type": "object",
        "properties": {
            "value": { "$ref": "#/$defs/Value" }
        },
        "required": ["value"],
        "additionalProperties": false,
        "enum": [{ "value": "leaf" }]
    });
    let sub_document = SchemaDocument::from_json(&raw).unwrap();
    assert!(sub_document.is_valid(&json!({ "value": "leaf" })).unwrap());

    let sub = sub_document.root().unwrap().clone();
    let sup = resolve(json!({
        "type": "object",
        "properties": {
            "value": { "type": "integer" }
        },
        "required": ["value"],
        "additionalProperties": false
    }));

    assert!(!is_subschema_of(&sub, &sup));
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
fn singleton_number_bounds_can_be_integer_subset() {
    let old = resolve(json!({ "type": "integer", "multipleOf": 2 }));
    let new = resolve(json!({ "type": "number", "minimum": 4, "maximum": 4 }));
    let fractional = resolve(json!({ "type": "number", "minimum": 4.5, "maximum": 4.5 }));

    assert!(is_subschema_of(&new, &old));
    assert!(!is_subschema_of(&fractional, &old));
}

#[test]
fn singleton_number_bounds_empty_after_multiple_of_is_vacuous() {
    let old = resolve(json!({ "type": "integer", "const": 7 }));
    let new = resolve(json!({
        "type": "number",
        "minimum": 4,
        "maximum": 4,
        "multipleOf": 3
    }));

    assert!(is_subschema_of(&new, &old));
}

#[test]
fn integral_number_multiple_of_alone_is_not_treated_as_integer() {
    let old = resolve(json!({ "type": "integer" }));
    let new = resolve(json!({ "type": "number", "multipleOf": 1 }));

    // The validator permits tiny floating-point deviations for multipleOf,
    // so values such as 1.0000000000000002 can satisfy the number schema
    // without satisfying the integer schema.
    assert!(!is_subschema_of(&new, &old));
}

#[test]
fn number_enum_can_be_subsumed_by_integer_schema() {
    let old = resolve(json!({ "type": "integer", "minimum": 2, "maximum": 8, "multipleOf": 2 }));
    let new = resolve(json!({
        "type": "number",
        "enum": [2, 4.0, 8],
        "minimum": 0,
        "maximum": 10
    }));

    assert!(is_subschema_of(&new, &old));
}

#[test]
fn number_enum_fractional_member_blocks_integer_subset() {
    let old = resolve(json!({ "type": "integer" }));
    let new = resolve(json!({ "type": "number", "enum": [2, 2.5] }));

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
fn same_value_recursive_anyof_is_not_subsumed_by_same_value_recursive_allof() {
    let sub = resolve(json!({
        "$defs": {
            "Node": {
                "anyOf": [
                    { "$ref": "#/$defs/Node" },
                    { "type": "string" }
                ]
            }
        },
        "$ref": "#/$defs/Node"
    }));
    let sup = resolve(json!({
        "$defs": {
            "Node": {
                "allOf": [
                    { "$ref": "#/$defs/Node" },
                    { "type": "string" }
                ]
            }
        },
        "$ref": "#/$defs/Node"
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
fn guaranteed_prefix_items_can_witness_contains_lower_bounds() {
    let old = resolve(json!({
        "type": "array",
        "contains": { "type": "integer" },
        "minContains": 1
    }));
    let new = resolve(json!({
        "type": "array",
        "prefixItems": [
            { "type": "integer" },
            { "type": "string" }
        ],
        "items": false,
        "minItems": 1
    }));

    assert!(is_subschema_of(&new, &old));
}

#[test]
fn discriminator_disjoint_items_satisfy_zero_max_contains() {
    let old = resolve(json!({
        "type": "array",
        "contains": {
            "type": "object",
            "required": ["kind"],
            "properties": { "kind": { "const": "blocked" } }
        },
        "minContains": 0,
        "maxContains": 0
    }));
    let new = resolve(json!({
        "type": "array",
        "items": {
            "type": "object",
            "required": ["kind"],
            "properties": { "kind": { "const": "allowed" } }
        }
    }));

    assert!(is_subschema_of(&new, &old));
}

#[test]
fn range_disjoint_items_satisfy_zero_max_contains() {
    let old = resolve(json!({
        "type": "array",
        "contains": { "type": "number", "minimum": 10 },
        "minContains": 0,
        "maxContains": 0
    }));
    let new = resolve(json!({
        "type": "array",
        "items": { "type": "number", "maximum": 0 }
    }));

    assert!(is_subschema_of(&new, &old));
}

#[test]
fn type_disjoint_items_satisfy_zero_max_contains() {
    let old = resolve(json!({
        "type": "array",
        "contains": { "type": "number" },
        "minContains": 0,
        "maxContains": 0
    }));
    let new = resolve(json!({
        "type": "array",
        "items": { "type": "string" }
    }));

    assert!(is_subschema_of(&new, &old));
}

#[test]
fn discriminator_disjoint_tuple_positions_make_unique_items_redundant() {
    let old = resolve(json!({
        "type": "array",
        "minItems": 2,
        "maxItems": 2,
        "uniqueItems": true
    }));
    let tagged = |tag: &str| {
        json!({
            "type": "object",
            "required": ["kind"],
            "properties": { "kind": { "const": tag } }
        })
    };
    let new = resolve(json!({
        "type": "array",
        "prefixItems": [tagged("left"), tagged("right")],
        "items": false,
        "minItems": 2,
        "maxItems": 2
    }));

    assert!(is_subschema_of(&new, &old));
}

#[test]
fn disjoint_tuple_positions_make_unique_items_redundant() {
    let old = resolve(json!({
        "type": "array",
        "minItems": 2,
        "maxItems": 2,
        "uniqueItems": true
    }));
    let new = resolve(json!({
        "type": "array",
        "prefixItems": [
            { "type": "string" },
            { "type": "number" }
        ],
        "items": false,
        "minItems": 2,
        "maxItems": 2
    }));

    assert!(is_subschema_of(&new, &old));
}

#[test]
fn unique_items_finite_item_domain_can_force_contains_lower_bound() {
    let old = resolve(json!({
        "type": "array",
        "contains": { "const": "a" },
        "minContains": 1
    }));
    let new = resolve(json!({
        "type": "array",
        "items": { "enum": ["a", "b"] },
        "uniqueItems": true,
        "minItems": 2
    }));

    assert!(is_subschema_of(&new, &old));
    assert!(explain_subschema_failure(&new, &old).is_none());
}

#[test]
fn unique_items_finite_tail_domain_with_prefix_can_force_contains_lower_bound() {
    let old = resolve(json!({
        "type": "array",
        "contains": { "enum": [2, 3] },
        "minContains": 1
    }));
    let new = resolve(json!({
        "type": "array",
        "prefixItems": [{ "const": "header" }],
        "items": { "enum": [1, 2, 3] },
        "uniqueItems": true,
        "minItems": 3
    }));

    assert!(is_subschema_of(&new, &old));
    assert!(explain_subschema_failure(&new, &old).is_none());
}

#[test]
fn unique_nested_array_domain_filters_contains_candidates() {
    let old = resolve(json!({
        "type": "array",
        "maxItems": 1
    }));
    let new = resolve(json!({
        "type": "array",
        "items": {
            "type": "array",
            "items": { "const": 0 },
            "maxItems": 1,
            "contains": { "const": 0 },
            "minContains": 1
        },
        "uniqueItems": true
    }));

    assert!(is_subschema_of(&new, &old));
}

#[test]
fn unique_nested_tuple_item_domain_infers_false_tail_capacity() {
    let old = resolve(json!({
        "type": "array",
        "maxItems": 2
    }));
    let new = resolve(json!({
        "type": "array",
        "items": {
            "type": "array",
            "prefixItems": [{ "const": 0 }],
            "items": false
        },
        "uniqueItems": true
    }));

    assert!(is_subschema_of(&new, &old));
}

#[test]
fn unique_nested_small_array_item_domain_has_finite_capacity() {
    let old = resolve(json!({
        "type": "array",
        "maxItems": 2
    }));
    let new = resolve(json!({
        "type": "array",
        "items": {
            "type": "array",
            "items": { "const": 0 },
            "maxItems": 1
        },
        "uniqueItems": true
    }));

    assert!(is_subschema_of(&new, &old));
}

#[test]
fn unique_required_prefix_consumes_tail_match_for_max_contains() {
    let old = resolve(json!({
        "type": "array",
        "contains": { "const": "x" },
        "maxContains": 1
    }));
    let new = resolve(json!({
        "type": "array",
        "prefixItems": [{ "const": "x" }],
        "items": { "enum": ["x", "y"] },
        "uniqueItems": true,
        "minItems": 1
    }));

    assert!(is_subschema_of(&new, &old));
}

#[test]
fn unique_required_prefix_consumes_tail_nonmatch_for_contains() {
    let old = resolve(json!({
        "type": "array",
        "contains": { "const": "y" },
        "minContains": 1
    }));
    let new = resolve(json!({
        "type": "array",
        "prefixItems": [{ "const": "x" }],
        "items": { "enum": ["x", "y"] },
        "uniqueItems": true,
        "minItems": 2
    }));

    assert!(is_subschema_of(&new, &old));
}

#[test]
fn unique_items_finite_item_domain_does_not_force_contains_when_room_to_avoid() {
    let old = resolve(json!({
        "type": "array",
        "contains": { "const": "a" },
        "minContains": 1
    }));
    let new = resolve(json!({
        "type": "array",
        "items": { "enum": ["a", "b"] },
        "uniqueItems": true,
        "minItems": 1
    }));

    assert!(!is_subschema_of(&new, &old));
}

#[test]
fn unique_items_finite_item_domain_disjoint_contains_is_impossible() {
    let old = resolve(json!({
        "type": "array",
        "maxItems": 0
    }));
    let new = resolve(json!({
        "type": "array",
        "items": { "enum": [1, 2] },
        "uniqueItems": true,
        "contains": { "const": 3 },
        "minContains": 1
    }));

    assert!(is_subschema_of(&new, &old));
    assert!(explain_subschema_failure(&new, &old).is_none());
}

#[test]
fn unique_items_inferred_max_can_make_broad_min_contains_impossible() {
    let old = resolve(json!({
        "type": "array",
        "maxItems": 0
    }));
    let new = resolve(json!({
        "type": "array",
        "items": { "enum": [1, 2] },
        "uniqueItems": true,
        "contains": true,
        "minContains": 3
    }));

    assert!(is_subschema_of(&new, &old));
    assert!(explain_subschema_failure(&new, &old).is_none());
}

#[test]
fn unique_items_finite_contains_domain_can_make_min_contains_impossible() {
    let old = resolve(json!({
        "type": "array",
        "maxItems": 0
    }));
    let new = resolve(json!({
        "type": "array",
        "uniqueItems": true,
        "contains": { "enum": [1, 2] },
        "minContains": 3
    }));

    assert!(is_subschema_of(&new, &old));
    assert!(explain_subschema_failure(&new, &old).is_none());
}

#[test]
fn unique_items_finite_item_domain_satisfies_max_items() {
    let old = resolve(json!({
        "type": "array",
        "maxItems": 2
    }));
    let new = resolve(json!({
        "type": "array",
        "items": { "enum": ["a", "b"] },
        "uniqueItems": true
    }));

    assert!(is_subschema_of(&new, &old));
    assert!(explain_subschema_failure(&new, &old).is_none());
}

#[test]
fn unique_items_finite_tail_domain_with_prefix_satisfies_max_items() {
    let old = resolve(json!({
        "type": "array",
        "maxItems": 3
    }));
    let new = resolve(json!({
        "type": "array",
        "prefixItems": [{ "type": "string" }],
        "items": { "enum": [1, 2] },
        "uniqueItems": true
    }));

    assert!(is_subschema_of(&new, &old));
    assert!(explain_subschema_failure(&new, &old).is_none());
}

#[test]
fn unique_items_finite_item_domain_can_make_min_items_impossible() {
    let old = resolve(json!({
        "type": "array",
        "maxItems": 0
    }));
    let new = resolve(json!({
        "type": "array",
        "items": { "enum": [1, 2] },
        "uniqueItems": true,
        "minItems": 3
    }));

    assert!(is_subschema_of(&new, &old));
    assert!(explain_subschema_failure(&new, &old).is_none());
}

#[test]
fn unique_items_finite_tail_domain_with_prefix_can_make_min_items_impossible() {
    let old = resolve(json!({
        "type": "array",
        "maxItems": 0
    }));
    let new = resolve(json!({
        "type": "array",
        "prefixItems": [{ "type": "string" }],
        "items": { "enum": [1, 2] },
        "uniqueItems": true,
        "minItems": 4
    }));

    assert!(is_subschema_of(&new, &old));
    assert!(explain_subschema_failure(&new, &old).is_none());
}

#[test]
fn unique_items_bounds_max_contains_for_finite_contains_domain() {
    let old = resolve(json!({
        "type": "array",
        "contains": { "enum": [1, 2] },
        "minContains": 0,
        "maxContains": 2
    }));
    let new = resolve(json!({
        "type": "array",
        "items": { "enum": [1, 2, 3, 4] },
        "uniqueItems": true
    }));

    assert!(is_subschema_of(&new, &old));
    assert!(explain_subschema_failure(&new, &old).is_none());
}

#[test]
fn unique_items_does_not_bound_max_contains_past_finite_domain_size() {
    let old = resolve(json!({
        "type": "array",
        "contains": { "enum": [1, 2, 3] },
        "minContains": 0,
        "maxContains": 2
    }));
    let new = resolve(json!({
        "type": "array",
        "items": { "enum": [1, 2, 3, 4] },
        "uniqueItems": true
    }));

    assert!(!is_subschema_of(&new, &old));
}

#[test]
fn object_count_disjoint_oneof_can_partition_objects() {
    let sub = resolve(json!({
        "type": "object",
        "maxProperties": 1
    }));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "object", "maxProperties": 1 },
            { "type": "object", "minProperties": 2 }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn overlapping_object_count_oneof_stays_conservative() {
    let sub = resolve(json!({
        "type": "object",
        "maxProperties": 2
    }));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "object", "maxProperties": 2 },
            { "type": "object", "minProperties": 2 }
        ]
    }));

    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn closed_declared_names_imply_object_count_partition() {
    let sub = resolve(json!({
        "type": "object",
        "properties": { "a": true, "b": true },
        "additionalProperties": false
    }));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "object", "properties": { "a": true, "b": true }, "additionalProperties": false },
            { "type": "object", "minProperties": 3 }
        ]
    }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn finite_property_names_imply_object_count_partition() {
    let sub = resolve(json!({
        "type": "object",
        "propertyNames": { "enum": ["a", "b"] }
    }));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "object", "propertyNames": { "enum": ["a", "b"] } },
            { "type": "object", "minProperties": 3 }
        ]
    }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn wrapped_integer_lattice_gap_can_partition_oneof() {
    let sub = resolve(json!({
        "allOf": [
            { "type": "integer", "minimum": 2, "maximum": 4 },
            { "type": "integer", "multipleOf": 2 }
        ]
    }));
    let sup = resolve(json!({
        "oneOf": [
            {
                "allOf": [
                    { "type": "integer", "minimum": 2, "maximum": 4 },
                    { "type": "integer", "multipleOf": 2 }
                ]
            },
            {
                "allOf": [
                    { "type": "integer", "minimum": 1, "maximum": 5 },
                    { "type": "integer", "multipleOf": 3 }
                ]
            }
        ]
    }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn property_disjointness_does_not_ignore_shared_non_objects() {
    let branch_a = json!({
        "type": ["object", "null"],
        "required": ["kind"],
        "properties": { "kind": { "const": "a" } }
    });
    let branch_b = json!({
        "type": ["object", "null"],
        "required": ["kind"],
        "properties": { "kind": { "const": "b" } }
    });
    let sub = resolve(branch_a.clone());
    let sup = resolve(json!({ "oneOf": [branch_a, branch_b] }));
    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn tuple_disjointness_does_not_ignore_shared_non_arrays() {
    let branch_a = json!({
        "type": ["array", "null"],
        "minItems": 1,
        "prefixItems": [{ "const": "a" }]
    });
    let branch_b = json!({
        "type": ["array", "null"],
        "minItems": 1,
        "prefixItems": [{ "const": "b" }]
    });
    let sub = resolve(branch_a.clone());
    let sup = resolve(json!({ "oneOf": [branch_a, branch_b] }));
    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn conditional_property_disjointness_keeps_shared_non_objects() {
    let branch_a = json!({
        "if": { "type": "object" },
        "then": {
            "required": ["kind"],
            "properties": { "kind": { "const": "a" } }
        }
    });
    let branch_b = json!({
        "if": { "type": "object" },
        "then": {
            "required": ["kind"],
            "properties": { "kind": { "const": "b" } }
        }
    });
    let sub = resolve(branch_a.clone());
    let sup = resolve(json!({ "oneOf": [branch_a, branch_b] }));
    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn conditional_tuple_disjointness_keeps_shared_non_arrays() {
    let branch_a = json!({
        "if": { "type": "array" },
        "then": {
            "minItems": 1,
            "prefixItems": [{ "const": "a" }]
        }
    });
    let branch_b = json!({
        "if": { "type": "array" },
        "then": {
            "minItems": 1,
            "prefixItems": [{ "const": "b" }]
        }
    });
    let sub = resolve(branch_a.clone());
    let sup = resolve(json!({ "oneOf": [branch_a, branch_b] }));
    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn integral_number_lattice_gap_is_not_used_for_number_oneof() {
    let sub = resolve(json!({
        "type": "number",
        "minimum": 2,
        "maximum": 4,
        "multipleOf": 2
    }));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "number", "minimum": 2, "maximum": 4, "multipleOf": 2 },
            { "type": "number", "minimum": 1, "maximum": 5, "multipleOf": 3 }
        ]
    }));
    // Numeric multipleOf uses an epsilon tolerance, so near-multiple floats
    // can overlap even when the integer lattice projection has a gap.
    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn integer_lattice_gap_can_partition_oneof() {
    let sub = resolve(json!({
        "type": "integer",
        "minimum": 2,
        "maximum": 4,
        "multipleOf": 2
    }));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "integer", "minimum": 2, "maximum": 4, "multipleOf": 2 },
            { "type": "integer", "minimum": 1, "maximum": 5, "multipleOf": 3 }
        ]
    }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn forbidden_closed_declared_property_tightens_count_partition() {
    let sub = resolve(json!({
        "type": "object",
        "properties": { "a": true, "b": false },
        "additionalProperties": false
    }));
    let sup = resolve(json!({
        "oneOf": [
            {
                "type": "object",
                "properties": { "a": true, "b": false },
                "additionalProperties": false
            },
            { "type": "object", "minProperties": 2 }
        ]
    }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn forbidden_finite_property_names_tighten_object_count_partition() {
    let sub = resolve(json!({
        "type": "object",
        "propertyNames": { "enum": ["a", "b"] },
        "properties": { "b": false }
    }));
    let sup = resolve(json!({
        "oneOf": [
            {
                "type": "object",
                "propertyNames": { "enum": ["a", "b"] },
                "properties": { "b": false }
            },
            { "type": "object", "minProperties": 2 }
        ]
    }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn conditional_finite_property_names_imply_object_count_partition() {
    let names = json!({
        "if": { "type": "string" },
        "then": { "enum": ["a", "b"] }
    });
    let sub = resolve(json!({
        "type": "object",
        "propertyNames": names.clone()
    }));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "object", "propertyNames": names },
            { "type": "object", "minProperties": 3 }
        ]
    }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn array_length_disjoint_oneof_can_partition_arrays() {
    let sub = resolve(json!({
        "type": "array",
        "maxItems": 1
    }));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "array", "maxItems": 1 },
            { "type": "array", "minItems": 2 }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn contains_minimum_implies_array_length_partition() {
    let sub = resolve(json!({
        "type": "array",
        "contains": {},
        "minContains": 2
    }));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "array", "contains": {}, "minContains": 2 },
            { "type": "array", "maxItems": 1 }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn universal_contains_maximum_implies_array_length_partition() {
    let sub = resolve(json!({
        "type": "array",
        "contains": {},
        "maxContains": 1
    }));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "array", "contains": {}, "maxContains": 1 },
            { "type": "array", "minItems": 2 }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn false_tail_implies_array_length_partition() {
    let sub = resolve(json!({
        "type": "array",
        "prefixItems": [{ "const": 1 }, { "const": 2 }],
        "items": false
    }));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "array", "prefixItems": [{ "const": 1 }, { "const": 2 }], "items": false },
            { "type": "array", "minItems": 3 }
        ]
    }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn unique_finite_items_imply_array_length_partition() {
    let sub = resolve(json!({
        "type": "array",
        "uniqueItems": true,
        "items": { "enum": [1, 2] }
    }));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "array", "uniqueItems": true, "items": { "enum": [1, 2] } },
            { "type": "array", "minItems": 3 }
        ]
    }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn tuple_item_type_disjoint_oneof_can_partition_arrays() {
    let sub = resolve(json!({
        "type": "array",
        "minItems": 1,
        "prefixItems": [{ "type": "string" }]
    }));
    let sup = resolve(json!({
        "oneOf": [
            {
                "type": "array",
                "minItems": 1,
                "prefixItems": [{ "type": "string" }]
            },
            {
                "type": "array",
                "minItems": 1,
                "prefixItems": [{ "type": "integer" }]
            }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn tuple_item_const_disjoint_oneof_can_partition_arrays() {
    let sub = resolve(json!({
        "type": "array",
        "minItems": 1,
        "prefixItems": [{ "const": "left" }]
    }));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "array", "minItems": 1, "prefixItems": [{ "const": "left" }] },
            { "type": "array", "minItems": 1, "prefixItems": [{ "const": "right" }] }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn finite_tuple_item_rejected_by_pattern_can_partition_oneof() {
    let sub = resolve(json!({
        "type": "array",
        "minItems": 1,
        "prefixItems": [{ "const": "ok" }]
    }));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "array", "minItems": 1, "prefixItems": [{ "const": "ok" }] },
            { "type": "array", "minItems": 1, "prefixItems": [{ "type": "string", "pattern": "^err$" }] }
        ]
    }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn conditional_tuple_item_shape_can_partition_oneof() {
    let conditional = json!({
        "if": { "type": "array" },
        "then": {
            "type": "array",
            "minItems": 1,
            "prefixItems": [{ "type": "string" }]
        }
    });
    let sub = resolve(conditional.clone());
    let sup = resolve(json!({
        "oneOf": [
            conditional,
            { "type": "array", "minItems": 1, "prefixItems": [{ "type": "integer" }] }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn conditional_tuple_item_rejection_can_partition_oneof() {
    let sub = resolve(json!({
        "type": "array",
        "minItems": 1,
        "prefixItems": [{ "const": "tag-a" }]
    }));
    let conditional_other = json!({
        "if": { "type": "array" },
        "then": {
            "type": "array",
            "minItems": 1,
            "prefixItems": [{ "const": "tag-b" }]
        }
    });
    let sup = resolve(json!({
        "oneOf": [
            { "type": "array", "minItems": 1, "prefixItems": [{ "const": "tag-a" }] },
            conditional_other
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn conditional_tuple_item_values_can_partition_oneof() {
    let conditional = json!({
        "if": { "type": "array" },
        "then": {
            "type": "array",
            "minItems": 1,
            "prefixItems": [{ "const": "tag-a" }]
        }
    });
    let sub = resolve(conditional.clone());
    let sup = resolve(json!({
        "oneOf": [
            conditional,
            {
                "type": "array",
                "minItems": 1,
                "prefixItems": [{ "type": "string", "pattern": "^tag-b$" }]
            }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn optional_tuple_item_partition_stays_conservative() {
    let sub = resolve(json!({
        "type": "array",
        "prefixItems": [{ "type": "string" }]
    }));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "array", "prefixItems": [{ "type": "string" }] },
            { "type": "array", "prefixItems": [{ "type": "integer" }] }
        ]
    }));

    // The empty array matches both branches, so the tuple discriminator is
    // only sound when both sides force the discriminating position to exist.
    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn overlapping_array_length_oneof_stays_conservative() {
    let sub = resolve(json!({
        "type": "array",
        "maxItems": 2
    }));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "array", "maxItems": 2 },
            { "type": "array", "minItems": 2 }
        ]
    }));

    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn literal_string_and_array_lengths_have_intervals() {
    let string_const = resolve(json!({ "const": "hé" }));
    let string_interval =
        string_length_interval_bound(&string_const).expect("string const interval");
    assert_eq!(string_interval.lower, 2);
    assert_eq!(string_interval.upper, Some(2));

    let string_enum = resolve(json!({ "enum": ["a", "abcd", 7] }));
    let string_interval = string_length_interval_bound(&string_enum).expect("string enum interval");
    assert_eq!(string_interval.lower, 1);
    assert_eq!(string_interval.upper, Some(4));

    let array_const = resolve(json!({ "const": [1, 2, 3] }));
    let array_interval = array_length_interval_bound(&array_const).expect("array const interval");
    assert_eq!(array_interval.lower, 3);
    assert_eq!(array_interval.upper, Some(3));

    let array_enum = resolve(json!({ "enum": [[], [1, 2], "skip"] }));
    let array_interval = array_length_interval_bound(&array_enum).expect("array enum interval");
    assert_eq!(array_interval.lower, 0);
    assert_eq!(array_interval.upper, Some(2));
}

#[test]
fn string_length_disjoint_oneof_can_partition_strings() {
    let sub = resolve(json!({
        "type": "string",
        "maxLength": 2
    }));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "string", "maxLength": 2 },
            { "type": "string", "minLength": 3 }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn union_wrapped_string_length_partition_is_disjoint() {
    let sub = resolve(json!({
        "type": "string",
        "maxLength": 1
    }));
    let sup = resolve(json!({
        "oneOf": [
            {
                "anyOf": [
                    { "type": "string", "maxLength": 1 },
                    { "type": "string", "minLength": 2, "maxLength": 2 }
                ]
            },
            { "type": "string", "minLength": 3 }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn union_wrapped_numeric_range_partition_is_disjoint() {
    let sub = resolve(json!({
        "type": "number",
        "maximum": 1
    }));
    let sup = resolve(json!({
        "oneOf": [
            {
                "anyOf": [
                    { "type": "number", "maximum": 1 },
                    { "type": "number", "minimum": 2, "maximum": 2 }
                ]
            },
            { "type": "number", "minimum": 3 }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn overlapping_string_length_oneof_stays_conservative() {
    let sub = resolve(json!({
        "type": "string",
        "maxLength": 3
    }));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "string", "maxLength": 3 },
            { "type": "string", "minLength": 3 }
        ]
    }));

    // Length 3 strings hit both branches, so the oneOf is not a superset.
    assert!(!is_subschema_of(&sub, &sup));
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
fn finite_enum_can_be_split_across_anyof_literals() {
    let sub = resolve(json!({ "enum": ["red", "blue"] }));
    let sup = resolve(json!({
        "anyOf": [
            { "const": "red" },
            { "const": "blue" }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn finite_anyof_split_requires_every_possible_value() {
    let sub = resolve(json!({ "enum": ["red", "blue"] }));
    let sup = resolve(json!({
        "anyOf": [
            { "const": "red" }
        ]
    }));

    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn obvious_anyof_type_cover_accepts_unconstrained_schema() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "type": "null" },
            { "type": "boolean" },
            { "type": "number" },
            { "type": "string" },
            { "type": "array" },
            { "type": "object" }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn anyof_complement_cover_accepts_unconstrained_schema() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "type": "integer" },
            { "not": { "const": 1 } }
        ]
    }));

    // `const 1` is contained by `integer`, so integer ∪ not(1) is all JSON.
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn anyof_complement_of_union_cover_accepts_unconstrained_schema() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "type": "string" },
            { "type": "number" },
            { "not": { "anyOf": [
                { "type": "string" },
                { "type": "number" }
            ] } }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn anyof_finite_split_complement_cover_accepts_unconstrained_schema() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "not": { "enum": [1, 2, "x"] } },
            { "const": 1 },
            { "const": 2 },
            { "const": "x" }
        ]
    }));

    // The finite excluded enum is covered by several sibling branches;
    // together with its complement this is universal.
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn anyof_split_integer_range_complement_cover_accepts_unconstrained_schema() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "not": { "allOf": [
                { "type": "integer" },
                { "minimum": 0 },
                { "maximum": 2 }
            ] } },
            { "const": 0 },
            { "const": 1 },
            { "const": 2 }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn oneof_complement_of_disjoint_union_accepts_unconstrained_schema() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "string" },
            { "type": "number" },
            { "not": { "anyOf": [
                { "type": "string" },
                { "type": "number" }
            ] } }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn oneof_complement_of_overlapping_union_is_not_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "integer" },
            { "type": "number" },
            { "not": { "anyOf": [
                { "type": "integer" },
                { "type": "number" }
            ] } }
        ]
    }));

    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn oneof_infinite_complement_pair_accepts_unconstrained_schema() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "string" },
            { "not": { "type": "string" } }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn oneof_non_equivalent_complement_pair_stays_conservative() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "integer" },
            { "not": { "type": "number" } }
        ]
    }));

    // Fractional numbers match neither arm, so this is not universal.
    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn oneof_finite_complement_partition_accepts_unconstrained_schema() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "oneOf": [
            { "not": { "enum": [1, 2] } },
            { "const": 1 },
            { "const": 2 }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn oneof_finite_complement_partition_rejects_overlapping_cover() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "oneOf": [
            { "not": { "enum": [1, 2] } },
            { "enum": [1, 2] },
            { "const": 1 }
        ]
    }));

    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn split_allof_small_integer_range_uses_finite_conditional_target() {
    let sub = resolve(json!({
        "allOf": [
            { "type": "integer", "minimum": 0 },
            { "type": "integer", "maximum": 1 }
        ]
    }));
    let sup = resolve(json!({
        "if": { "enum": [1, "a", null] },
        "then": { "type": "number" },
        "else": true
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn conditional_disjoint_from_negated_const_target() {
    let sub = resolve(json!({
        "if": { "enum": [1, "a", null] },
        "then": { "type": "number" },
        "else": true
    }));
    let sup = resolve(json!({ "not": { "const": null } }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn identical_type_guard_prunes_both_vacuous_then_sides() {
    let sub = resolve(json!({ "if": {"type":"integer"}, "then": {"type":"null"}, "else": true }));
    let sup = resolve(json!({ "if": {"type":"integer"}, "then": {"type":"object"}, "else": true }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn identical_conditional_guard_prunes_vacuous_then_side() {
    let sub = resolve(json!({
        "if": { "enum": ["a", "b"] },
        "then": { "type": "integer" },
        "else": { "type": "object" }
    }));
    let sup = resolve(json!({
        "if": { "enum": ["a", "b"] },
        "then": { "type": "object" },
        "else": true
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn conditional_guard_covered_then_with_universal_else_is_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "if": { "enum": ["a", "b"] },
        "then": { "type": "string" },
        "else": true
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn oneof_universal_arm_excludes_disjoint_conditional() {
    let sub = resolve(json!({ "type": "integer", "multipleOf": 2 }));
    let sup = resolve(json!({
        "oneOf": [
            true,
            {
                "if": { "type": "integer", "multipleOf": 1 },
                "then": { "type": "array", "items": { "type": "boolean" } },
                "else": { "type": "number", "minimum": 0, "maximum": 3 }
            }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn anyof_conditional_tautology_branch_is_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "if": { "const": 1 }, "then": { "const": 1 } },
            { "type": "boolean" }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn conditional_else_complement_of_guard_is_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "if": { "type": "string" },
        "then": true,
        "else": { "not": { "const": "blocked-string" } }
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn conditional_else_complement_must_be_guard_subset() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "if": { "type": "string" },
        "then": true,
        "else": { "not": { "const": 1 } }
    }));

    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn conditional_then_enum_filtered_by_guard_fits_target() {
    let sub = resolve(json!({
        "if": { "type": "integer" },
        "then": { "enum": [1, "a"] },
        "else": false
    }));
    let sup = resolve(json!({ "type": "integer" }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn conditional_finite_guard_filters_infinite_then_branch() {
    let sub = resolve(json!({
        "if": { "enum": [1, "a"] },
        "then": { "type": "integer" },
        "else": false
    }));
    let sup = resolve(json!({ "const": 1 }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn conditional_negated_type_guard_false_then_is_inner_type() {
    let sub = resolve(json!({
        "if": { "not": { "type": "string" } },
        "then": false,
        "else": true
    }));
    let sup = resolve(json!({ "type": "string" }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn conditional_negated_finite_guard_with_universal_else_fits_target() {
    let sub = resolve(json!({
        "if": { "not": { "const": 1 } },
        "then": { "type": "integer" },
        "else": true
    }));
    let sup = resolve(json!({ "type": "integer" }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn conditional_negated_finite_guard_filters_infinite_else_branch() {
    let sub = resolve(json!({
        "if": { "not": { "const": 1 } },
        "then": false,
        "else": { "type": "integer" }
    }));
    let sup = resolve(json!({ "const": 1 }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn conditional_else_enum_filtered_by_guard_fits_target() {
    let sub = resolve(json!({
        "if": { "type": "integer" },
        "then": { "type": "string" },
        "else": { "enum": [1, "a"] }
    }));
    let sup = resolve(json!({ "type": "string" }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn same_guard_conditional_filters_then_by_finite_guard() {
    let sub = resolve(json!({
        "if": { "enum": [1, "a"] },
        "then": { "type": "integer" },
        "else": true
    }));
    let sup = resolve(json!({
        "if": { "enum": [1, "a"] },
        "then": { "const": 1 },
        "else": true
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn same_guard_conditional_filters_else_by_guard() {
    let sub = resolve(json!({
        "if": { "const": 1 },
        "then": true,
        "else": { "enum": [1, "a"] }
    }));
    let sup = resolve(json!({
        "if": { "const": 1 },
        "then": true,
        "else": { "const": "a" }
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn same_guard_conditional_missing_then_uses_finite_guard() {
    let sub = resolve(json!({
        "if": { "allOf": [{ "enum": [1, 2] }, { "const": 1 }] },
        "else": false
    }));
    let sup = resolve(json!({
        "if": { "allOf": [{ "enum": [1, 2] }, { "const": 1 }] },
        "then": { "const": 1 },
        "else": false
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn same_guard_conditional_missing_else_uses_negated_finite_guard() {
    let sub = resolve(json!({
        "if": { "not": { "enum": [1, "a"] } },
        "then": false
    }));
    let sup = resolve(json!({
        "if": { "not": { "enum": [1, "a"] } },
        "then": false,
        "else": { "enum": [1, "a"] }
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn conditional_with_false_then_is_subset_of_guard_complement() {
    let sub = resolve(json!({
        "if": { "type": "null" },
        "then": false,
        "else": true
    }));
    let sup = resolve(json!({ "not": { "type": "null" } }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn conditional_missing_else_uses_target_complement_cover() {
    let sub = resolve(json!({
        "if": { "type": "string" },
        "then": { "not": { "const": "reserved" } }
    }));
    let sup = resolve(json!({ "not": { "const": "reserved" } }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn conditional_explicit_else_uses_target_complement_cover() {
    let sub = resolve(json!({
        "if": { "type": "string" },
        "then": { "not": { "const": "reserved" } },
        "else": { "const": 1 }
    }));
    let sup = resolve(json!({ "not": { "const": "reserved" } }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn conditional_with_false_else_is_subset_of_guard_target() {
    let sub = resolve(json!({
        "if": { "type": "null" },
        "then": { "not": { "const": 1 } },
        "else": false
    }));
    let sup = resolve(json!({ "type": "null" }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn allof_to_allof_uses_whole_intersection_for_conjunct() {
    let sub = resolve(json!({
        "allOf": [
            { "type": "integer" },
            { "minimum": 1 },
            { "maximum": 3 }
        ]
    }));
    let sup = resolve(json!({
        "allOf": [
            { "type": "integer", "minimum": 0, "maximum": 5 }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn same_guard_explicit_then_uses_guard_cover() {
    let sub = resolve(json!({
        "if": { "enum": ["a", "b"] },
        "then": true,
        "else": false
    }));
    let sup = resolve(json!({
        "if": { "enum": ["a", "b"] },
        "then": { "type": "string" },
        "else": false
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn same_guard_explicit_else_uses_complement_cover() {
    let sub = resolve(json!({
        "if": { "type": "string" },
        "then": { "maxLength": 5 },
        "else": { "not": { "const": 1 } }
    }));
    let sup = resolve(json!({
        "if": { "type": "string" },
        "then": { "maxLength": 10 },
        "else": { "not": { "const": "reserved" } }
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn same_guard_missing_else_accepts_complement_cover() {
    let sub = resolve(json!({
        "if": { "type": "string" },
        "then": { "maxLength": 5 }
    }));
    let sup = resolve(json!({
        "if": { "type": "string" },
        "then": { "maxLength": 10 },
        "else": { "not": { "const": "reserved" } }
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn same_guard_missing_else_rejects_incomplete_complement_cover() {
    let sub = resolve(json!({
        "if": { "type": "string" },
        "then": { "maxLength": 5 }
    }));
    let sup = resolve(json!({
        "if": { "type": "string" },
        "then": { "maxLength": 10 },
        "else": { "not": { "const": 1 } }
    }));

    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn conditional_guard_not_covered_then_is_not_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "if": { "enum": ["a", "b"] },
        "then": { "type": "number" },
        "else": true
    }));

    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn incomplete_anyof_complement_cover_stays_conservative() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "type": "string" },
            { "not": { "const": 1 } }
        ]
    }));

    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn incomplete_anyof_type_cover_stays_conservative() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "type": "null" },
            { "type": "boolean" },
            { "type": "number" },
            { "type": "string" },
            { "type": "array" }
        ]
    }));

    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn dependent_required_implied_finite_tag_can_partition_oneof() {
    let sub = resolve(json!({
        "type": "object",
        "required": ["trigger"],
        "dependentRequired": { "trigger": ["tag"] },
        "properties": { "tag": { "const": "left" } }
    }));
    let sup = resolve(json!({
        "oneOf": [
            {
                "type": "object",
                "required": ["trigger"],
                "dependentRequired": { "trigger": ["tag"] },
                "properties": { "tag": { "const": "left" } }
            },
            {
                "type": "object",
                "required": ["trigger"],
                "dependentRequired": { "trigger": ["tag"] },
                "properties": { "tag": { "const": "right" } }
            }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn dependent_required_implied_count_disjoints_small_object_branch() {
    let sub = resolve(json!({
        "type": "object",
        "required": ["a"],
        "dependentRequired": {"a": ["b"]}
    }));
    let sup = resolve(json!({
        "oneOf": [
            {"type": "object", "required": ["a"], "dependentRequired": {"a": ["b"]}},
            {"type": "object", "maxProperties": 1}
        ]
    }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn max_properties_zero_forbids_implied_required_name_partition() {
    let sub = resolve(json!({
        "type": "object",
        "required": ["trigger"],
        "dependentRequired": {"trigger": ["tag"]}
    }));
    let sup = resolve(json!({
        "oneOf": [
            {"type": "object", "required": ["trigger"], "dependentRequired": {"trigger": ["tag"]}},
            {"type": "object", "maxProperties": 0}
        ]
    }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn negated_required_property_can_partition_oneof() {
    let sub = resolve(json!({
        "type": "object",
        "required": ["present"]
    }));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "object", "required": ["present"] },
            { "type": "object", "not": { "required": ["present"] } }
        ]
    }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn split_allof_dependent_required_implied_tag_shape_can_partition_oneof() {
    let left = json!({"allOf": [
        {"type": "object", "required": ["trigger"]},
        {"type": "object", "dependentRequired": {"trigger": ["tag"]}},
        {"type": "object", "properties": {"tag": {"type": "string"}}}
    ]});
    let right = json!({"allOf": [
        {"type": "object", "required": ["trigger"]},
        {"type": "object", "dependentRequired": {"trigger": ["tag"]}},
        {"type": "object", "properties": {"tag": {"type": "number"}}}
    ]});
    let sub = resolve(left.clone());
    let sup = resolve(json!({"oneOf": [left, right]}));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn dependent_required_implied_tag_shape_can_partition_oneof() {
    let sub = resolve(json!({
        "type": "object",
        "required": ["trigger"],
        "dependentRequired": { "trigger": ["tag"] },
        "properties": { "tag": { "type": "string" } }
    }));
    let sup = resolve(json!({
        "oneOf": [
            {
                "type": "object",
                "required": ["trigger"],
                "dependentRequired": { "trigger": ["tag"] },
                "properties": { "tag": { "type": "string" } }
            },
            {
                "type": "object",
                "required": ["trigger"],
                "dependentRequired": { "trigger": ["tag"] },
                "properties": { "tag": { "type": "number" } }
            }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn dependent_required_implied_property_can_partition_oneof() {
    let sub = resolve(json!({
        "type": "object",
        "required": ["a"],
        "dependentRequired": { "a": ["b"] },
        "properties": { "a": true, "b": { "type": "string" } }
    }));
    let sup = resolve(json!({
        "oneOf": [
            {
                "type": "object",
                "required": ["a"],
                "dependentRequired": { "a": ["b"] },
                "properties": { "a": true, "b": { "type": "string" } }
            },
            { "type": "object", "properties": { "b": false } }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn optional_dependent_required_trigger_does_not_partition_oneof() {
    let sub = resolve(json!({
        "type": "object",
        "dependentRequired": { "a": ["b"] }
    }));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "object", "dependentRequired": { "a": ["b"] } },
            { "type": "object", "properties": { "b": false } }
        ]
    }));

    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn required_property_forbidden_by_closed_other_branch_is_disjoint() {
    let sub = resolve(json!({
        "type": "object",
        "required": ["a"],
        "properties": { "a": { "type": "string" } },
        "additionalProperties": false
    }));
    let sup = resolve(json!({
        "oneOf": [
            {
                "type": "object",
                "required": ["a"],
                "properties": { "a": { "type": "string" } },
                "additionalProperties": false
            },
            {
                "type": "object",
                "required": ["b"],
                "properties": { "b": { "type": "string" } },
                "additionalProperties": false
            }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn required_property_forbidden_by_property_names_other_branch_is_disjoint() {
    let sub = resolve(json!({
        "type": "object",
        "required": ["a"],
        "properties": { "a": { "type": "string" } }
    }));
    let sup = resolve(json!({
        "oneOf": [
            {
                "type": "object",
                "required": ["a"],
                "properties": { "a": { "type": "string" } }
            },
            {
                "type": "object",
                "required": ["b"],
                "propertyNames": { "enum": ["b"] },
                "properties": { "b": { "type": "string" } }
            }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn required_property_forbidden_in_other_oneof_branch_is_disjoint() {
    let sub = resolve(json!({
        "type": "object",
        "required": ["a"],
        "properties": {
            "a": { "type": "string" },
            "b": false
        }
    }));
    let sup = resolve(json!({
        "oneOf": [
            {
                "type": "object",
                "required": ["a"],
                "properties": { "a": { "type": "string" }, "b": false }
            },
            {
                "type": "object",
                "required": ["b"],
                "properties": { "b": { "type": "string" }, "a": false }
            }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn conditional_guard_narrows_possible_type_mask_for_oneof() {
    let sub = resolve(json!({
        "if": { "type": "string" },
        "then": true,
        "else": { "type": "number" }
    }));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "string" },
            { "type": "number" }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn complement_type_oneof_branch_is_disjoint_by_mask() {
    let sub = resolve(json!({ "type": "string", "minLength": 1 }));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "string" },
            { "not": { "type": "string" } }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn complement_null_oneof_branch_is_disjoint_by_mask() {
    let sub = resolve(json!({ "type": "string" }));
    let sup = resolve(json!({
        "oneOf": [
            { "not": { "const": null } },
            { "const": null }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn type_disjoint_oneof_can_be_a_superset_of_string_schema() {
    let sub = resolve(json!({
        "type": "string",
        "minLength": 2
    }));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "string" },
            { "type": "integer" },
            { "type": "null" }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn finite_enum_can_be_split_across_disjoint_oneof_literals() {
    let sub = resolve(json!({ "enum": ["red", "blue"] }));
    let sup = resolve(json!({
        "oneOf": [
            { "const": "red" },
            { "const": "blue" }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn finite_enum_overlapping_oneof_branch_stays_conservative() {
    let sub = resolve(json!({ "enum": ["red"] }));
    let sup = resolve(json!({
        "oneOf": [
            { "enum": ["red", "blue"] },
            { "const": "red" }
        ]
    }));

    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn finite_disjoint_oneof_literals_can_partition_same_type() {
    let sub = resolve(json!({ "const": "red" }));
    let sup = resolve(json!({
        "oneOf": [
            { "const": "red" },
            { "const": "blue" }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn negated_conditional_required_property_can_partition_branch() {
    let sub = resolve(json!({
        "type": "object",
        "required": ["flag"]
    }));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "object", "required": ["flag"] },
            { "not": {
                "if": { "type": "object" },
                "then": { "required": ["flag"] }
            }}
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn conditional_forbidden_property_can_partition_required_branch() {
    let sub = resolve(json!({
        "type": "object",
        "required": ["flag"]
    }));
    let conditional_other = json!({
        "if": { "type": "object" },
        "then": {
            "type": "object",
            "properties": { "flag": false }
        }
    });
    let sup = resolve(json!({
        "oneOf": [
            { "type": "object", "required": ["flag"] },
            conditional_other
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn conditional_object_property_rejection_can_partition_other_branch() {
    let sub = resolve(json!({
        "type": "object",
        "required": ["kind"],
        "properties": { "kind": { "const": "a" } }
    }));
    let conditional_other = json!({
        "if": { "type": "object" },
        "then": {
            "type": "object",
            "required": ["kind"],
            "properties": { "kind": { "const": "b" } }
        }
    });
    let sup = resolve(json!({
        "oneOf": [
            { "type": "object", "required": ["kind"], "properties": { "kind": { "const": "a" } } },
            conditional_other
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn conditional_object_property_shape_can_partition_other_branch() {
    let conditional = json!({
        "if": { "type": "object" },
        "then": {
            "type": "object",
            "required": ["kind"],
            "properties": { "kind": { "type": "string" } }
        }
    });
    let sub = resolve(conditional.clone());
    let sup = resolve(json!({
        "oneOf": [
            conditional,
            {
                "type": "object",
                "required": ["kind"],
                "properties": { "kind": { "type": "integer" } }
            }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn conditional_then_only_object_tag_can_partition_other_branch() {
    let conditional = json!({
        "if": { "type": "object" },
        "then": {
            "type": "object",
            "required": ["kind"],
            "properties": { "kind": { "const": "a" } }
        }
    });
    let sub = resolve(conditional.clone());
    let sup = resolve(json!({
        "oneOf": [
            conditional,
            {
                "type": "object",
                "required": ["kind"],
                "properties": { "kind": { "const": "b" } }
            }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn conditional_object_tag_values_can_partition_other_branch() {
    let conditional = json!({
        "if": { "type": "object", "required": ["kind"] },
        "then": {
            "type": "object",
            "required": ["kind"],
            "properties": { "kind": { "const": "a" } }
        },
        "else": {
            "type": "object",
            "required": ["kind"],
            "properties": { "kind": { "const": "b" } }
        }
    });
    let sub = resolve(conditional.clone());
    let sup = resolve(json!({
        "oneOf": [
            conditional,
            {
                "type": "object",
                "required": ["kind"],
                "properties": { "kind": { "const": "c" } }
            }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn conditional_then_only_guard_all_strings_bounds_string_partition() {
    let conditional = json!({
        "if": { "type": "string" },
        "then": { "type": "string", "maxLength": 2 }
    });
    let sub = resolve(conditional.clone());
    let sup = resolve(json!({
        "oneOf": [
            conditional,
            { "type": "string", "minLength": 8 }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn conditional_else_only_impossible_string_guard_bounds_partition() {
    let conditional = json!({
        "if": { "type": "number" },
        "else": { "type": "string", "maxLength": 2 }
    });
    let sub = resolve(conditional.clone());
    let sup = resolve(json!({
        "oneOf": [
            conditional,
            { "type": "string", "minLength": 8 }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn conditional_string_branch_hull_can_partition_length() {
    let conditional = json!({
        "if": { "type": "string", "maxLength": 2 },
        "then": { "type": "string", "maxLength": 2 },
        "else": { "type": "string", "minLength": 4, "maxLength": 5 }
    });
    let sub = resolve(conditional.clone());
    let sup = resolve(json!({
        "oneOf": [
            conditional,
            { "type": "string", "minLength": 8 }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn conditional_array_branch_hull_can_partition_length() {
    let conditional = json!({
        "if": { "type": "array", "maxItems": 1 },
        "then": { "type": "array", "maxItems": 1 },
        "else": { "type": "array", "minItems": 3, "maxItems": 4 }
    });
    let sub = resolve(conditional.clone());
    let sup = resolve(json!({
        "oneOf": [
            conditional,
            { "type": "array", "minItems": 8 }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn conditional_object_branch_hull_can_partition_count() {
    let conditional = json!({
        "if": { "type": "object", "maxProperties": 1 },
        "then": { "type": "object", "maxProperties": 1 },
        "else": { "type": "object", "minProperties": 3, "maxProperties": 4 }
    });
    let sub = resolve(conditional.clone());
    let sup = resolve(json!({
        "oneOf": [
            conditional,
            { "type": "object", "minProperties": 8 }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn conditional_numeric_branch_hull_can_partition_range() {
    let conditional = json!({
        "if": { "type": "number", "maximum": 0 },
        "then": { "type": "number", "maximum": 0 },
        "else": { "type": "number", "minimum": 10, "maximum": 20 }
    });
    let sub = resolve(conditional.clone());
    let sup = resolve(json!({
        "oneOf": [
            conditional,
            { "type": "number", "minimum": 30 }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn numeric_literal_interval_can_partition_range_branch() {
    let sub = resolve(json!({ "const": 7 }));
    let sup = resolve(json!({
        "oneOf": [
            { "const": 7 },
            { "type": "number", "maximum": 0 }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn numeric_enum_interval_can_partition_range_branch() {
    let sub = resolve(json!({ "enum": [2, 3] }));
    let sup = resolve(json!({
        "oneOf": [
            { "enum": [2, 3] },
            { "type": "number", "minimum": 10 }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn numeric_range_disjoint_oneof_can_be_a_superset_of_bounded_integer_schema() {
    let sub = resolve(json!({
        "type": "integer",
        "minimum": 2,
        "maximum": 4
    }));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "integer", "minimum": 0, "maximum": 10 },
            { "type": "integer", "minimum": 11, "maximum": 20 }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn split_allof_numeric_range_can_imply_plain_number_range() {
    let sub = resolve(json!({
        "allOf": [
            { "type": "number", "minimum": 1 },
            { "type": "number", "exclusiveMaximum": 5 }
        ]
    }));
    let sup = resolve(json!({
        "type": "number",
        "minimum": 0,
        "maximum": 5
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn split_allof_integer_range_can_imply_plain_integer_range() {
    let sub = resolve(json!({
        "allOf": [
            { "type": "integer", "minimum": 2 },
            { "type": "integer", "maximum": 4 }
        ]
    }));
    let sup = resolve(json!({
        "type": "integer",
        "minimum": 1,
        "maximum": 5
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn split_allof_number_range_does_not_imply_integer_range() {
    let sub = resolve(json!({
        "allOf": [
            { "type": "number", "minimum": 2 },
            { "type": "number", "maximum": 4 }
        ]
    }));
    let sup = resolve(json!({
        "type": "integer",
        "minimum": 1,
        "maximum": 5
    }));

    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn split_allof_numeric_bounds_without_type_stay_conservative() {
    let sub = resolve(json!({
        "allOf": [
            { "minimum": 1 },
            { "maximum": 3 }
        ]
    }));
    let sup = resolve(json!({
        "type": "number",
        "minimum": 0,
        "maximum": 5
    }));

    // Numeric keywords alone also admit strings/objects in JSON Schema.
    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn split_allof_string_length_can_imply_plain_string_range() {
    let sub = resolve(json!({
        "allOf": [
            { "type": "string", "minLength": 2 },
            { "type": "string", "maxLength": 4 }
        ]
    }));
    let sup = resolve(json!({
        "type": "string",
        "minLength": 1,
        "maxLength": 5
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn split_allof_array_length_can_imply_plain_array_range() {
    let sub = resolve(json!({
        "allOf": [
            { "type": "array", "minItems": 2 },
            { "type": "array", "maxItems": 4 }
        ]
    }));
    let sup = resolve(json!({
        "type": "array",
        "minItems": 1,
        "maxItems": 5
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn split_allof_object_count_can_imply_plain_object_range() {
    let sub = resolve(json!({
        "allOf": [
            { "type": "object", "minProperties": 2 },
            { "type": "object", "maxProperties": 4 }
        ]
    }));
    let sup = resolve(json!({
        "type": "object",
        "minProperties": 1,
        "maxProperties": 5
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn split_allof_length_bounds_without_type_stay_conservative() {
    let sub = resolve(json!({
        "allOf": [
            { "minLength": 2 },
            { "maxLength": 4 }
        ]
    }));
    let sup = resolve(json!({
        "type": "string",
        "minLength": 1,
        "maxLength": 5
    }));

    // Length keywords alone also admit non-strings in JSON Schema.
    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn discriminator_disjoint_oneof_can_be_a_superset_of_object_schema() {
    let sub = resolve(json!({
        "type": "object",
        "required": ["kind", "name"],
        "properties": {
            "kind": { "const": "cat" },
            "name": { "type": "string" }
        }
    }));
    let sup = resolve(json!({
        "oneOf": [
            {
                "type": "object",
                "required": ["kind"],
                "properties": { "kind": { "const": "cat" } }
            },
            {
                "type": "object",
                "required": ["kind"],
                "properties": { "kind": { "const": "dog" } }
            }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn pattern_properties_shape_can_partition_required_property() {
    let sub = resolve(json!({
        "type": "object",
        "required": ["tag"],
        "patternProperties": { "^tag$": { "type": "string" } }
    }));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "object", "required": ["tag"], "patternProperties": { "^tag$": { "type": "string" } } },
            { "type": "object", "required": ["tag"], "patternProperties": { "^tag$": { "type": "integer" } } }
        ]
    }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn branchwise_wrapped_range_partition_is_disjoint() {
    let sub = resolve(json!({ "type": "number", "maximum": 0 }));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "number", "maximum": 0 },
            { "anyOf": [
                { "type": "number", "minimum": 1 },
                { "type": "string" }
            ] }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn explicit_not_guard_can_partition_oneof() {
    let sub = resolve(json!({
        "type": "object",
        "required": ["kind"],
        "properties": { "kind": { "const": "cat" } }
    }));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "object", "required": ["kind"], "properties": { "kind": { "const": "cat" } } },
            { "allOf": [
                { "not": { "type": "object", "required": ["kind"], "properties": { "kind": { "const": "cat" } } } },
                { "type": "object" }
            ] }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn saturated_required_slots_forbid_extra_property_partition() {
    let sub = resolve(json!({
        "type": "object",
        "required": ["a"],
        "maxProperties": 1
    }));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "object", "required": ["a"], "maxProperties": 1 },
            { "type": "object", "required": ["b"], "maxProperties": 1 }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn enum_without_name_forbids_required_property_partition() {
    let sub = resolve(json!({
        "type": "object",
        "required": ["tag"],
        "properties": { "tag": { "type": "string" } }
    }));
    let sup = resolve(json!({
        "oneOf": [
            {
                "type": "object",
                "required": ["tag"],
                "properties": { "tag": { "type": "string" } }
            },
            { "enum": [{}, {"other": 1}, null] }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn matching_false_pattern_forbids_required_property_partition() {
    let sub = resolve(json!({
        "type": "object",
        "required": ["x_tag"],
        "properties": { "x_tag": { "type": "string" } }
    }));
    let sup = resolve(json!({
        "oneOf": [
            {
                "type": "object",
                "required": ["x_tag"],
                "properties": { "x_tag": { "type": "string" } }
            },
            {
                "type": "object",
                "patternProperties": { "^x_": false }
            }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn property_type_disjoint_oneof_can_partition_objects() {
    let sub = resolve(json!({
        "type": "object",
        "required": ["tag"],
        "properties": { "tag": { "type": "string" } }
    }));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "object", "required": ["tag"], "properties": { "tag": { "type": "string" } } },
            { "type": "object", "required": ["tag"], "properties": { "tag": { "type": "integer" } } }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn property_length_disjoint_oneof_can_partition_objects() {
    let sub = resolve(json!({
        "type": "object",
        "required": ["code"],
        "properties": { "code": { "type": "string", "maxLength": 2 } }
    }));
    let sup = resolve(json!({
        "oneOf": [
            {
                "type": "object",
                "required": ["code"],
                "properties": { "code": { "type": "string", "maxLength": 2 } }
            },
            {
                "type": "object",
                "required": ["code"],
                "properties": { "code": { "type": "string", "minLength": 3 } }
            }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn finite_property_value_rejected_by_pattern_can_partition_oneof() {
    let sub = resolve(json!({
        "type": "object",
        "required": ["code"],
        "properties": { "code": { "const": "ok" } }
    }));
    let sup = resolve(json!({
        "oneOf": [
            {
                "type": "object",
                "required": ["code"],
                "properties": { "code": { "const": "ok" } }
            },
            {
                "type": "object",
                "required": ["code"],
                "properties": { "code": { "type": "string", "pattern": "^err$" } }
            }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn overlapping_property_length_partition_stays_conservative() {
    let sub = resolve(json!({
        "type": "object",
        "required": ["code"],
        "properties": { "code": { "type": "string", "maxLength": 3 } }
    }));
    let sup = resolve(json!({
        "oneOf": [
            {
                "type": "object",
                "required": ["code"],
                "properties": { "code": { "type": "string", "maxLength": 3 } }
            },
            {
                "type": "object",
                "required": ["code"],
                "properties": { "code": { "type": "string", "minLength": 3 } }
            }
        ]
    }));

    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn split_allof_overlapping_property_partition_stays_conservative() {
    let sub = resolve(json!({
        "allOf": [
            { "type": "object", "required": ["tag"] },
            { "type": "object", "properties": { "tag": { "type": "string" } } }
        ]
    }));
    let sup = resolve(json!({
        "oneOf": [
            { "allOf": [
                { "type": "object", "required": ["tag"] },
                { "type": "object", "properties": { "tag": { "type": "string" } } }
            ] },
            { "allOf": [
                { "type": "object", "required": ["tag"] },
                { "type": "object", "properties": { "tag": { "minLength": 1 } } }
            ] }
        ]
    }));

    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn split_allof_required_property_type_disjoint_partition() {
    let sub = resolve(json!({
        "allOf": [
            { "type": "object", "required": ["tag"] },
            { "type": "object", "properties": { "tag": { "type": "string" } } }
        ]
    }));
    let sup = resolve(json!({
        "oneOf": [
            { "allOf": [
                { "type": "object", "required": ["tag"] },
                { "type": "object", "properties": { "tag": { "type": "string" } } }
            ] },
            { "allOf": [
                { "type": "object", "required": ["tag"] },
                { "type": "object", "properties": { "tag": { "type": "integer" } } }
            ] }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn union_wrapped_discriminators_are_still_disjoint_for_oneof() {
    let sub = resolve(json!({
        "type": "object",
        "required": ["kind"],
        "properties": { "kind": { "const": "cat" } }
    }));
    let sup = resolve(json!({
        "oneOf": [
            {
                "anyOf": [
                    { "type": "object", "required": ["kind"], "properties": { "kind": { "const": "cat" } } },
                    { "type": "object", "required": ["kind"], "properties": { "kind": { "enum": ["cat"] }, "extra": { "type": "string" } } }
                ]
            },
            { "type": "object", "required": ["kind"], "properties": { "kind": { "const": "dog" } } }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn mixed_object_enum_without_common_discriminator_stays_conservative() {
    let sub = resolve(json!({
        "enum": [
            { "kind": "cat" },
            { "shared": true }
        ]
    }));
    let sup = resolve(json!({
        "oneOf": [
            { "enum": [{ "kind": "cat" }, { "shared": true }] },
            { "enum": [{ "kind": "dog" }, { "shared": true }] }
        ]
    }));

    // The {"shared": true} value matches both branches, so discriminator
    // extraction must not ignore enum objects that lack the discriminator.
    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn type_disjoint_negation_accepts_string_subset() {
    let sub = resolve(json!({ "type": "string", "minLength": 1 }));
    let sup = resolve(json!({ "not": { "type": "number" } }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn finite_negated_values_rejected_by_subset_are_disjoint() {
    let sub = resolve(json!({ "type": "string", "minLength": 2 }));
    let sup = resolve(json!({ "not": { "const": "" } }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn finite_subset_values_can_be_disjoint_from_infinite_negated_schema() {
    let sub = resolve(json!({ "const": "" }));
    let sup = resolve(json!({ "not": { "type": "string", "minLength": 1 } }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn negated_anyof_disjoint_bounds_are_handled_branchwise() {
    let sub = resolve(json!({ "type": "integer", "maximum": 0 }));
    let sup = resolve(json!({
        "not": {
            "anyOf": [
                { "type": "integer", "minimum": 1 },
                { "type": "string" }
            ]
        }
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn allof_of_negations_implies_negated_anyof() {
    let sub = resolve(json!({
        "allOf": [
            { "not": { "type": "string" } },
            { "not": { "type": "integer" } }
        ]
    }));
    let sup = resolve(json!({
        "not": {
            "anyOf": [
                { "type": "string" },
                { "type": "integer" }
            ]
        }
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn negated_allof_of_complements_is_covered_by_positive_union() {
    let sub = resolve(json!({
        "not": { "allOf": [
            { "not": { "const": 2 } },
            { "not": { "enum": [2, 3] } }
        ] }
    }));
    let sup = resolve(json!({ "anyOf": [
        { "enum": [2, 3] },
        { "enum": ["a", "b"] }
    ] }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn negated_union_with_finite_complement_arm_filters_sibling_values() {
    let sub = resolve(json!({
        "not": {
            "anyOf": [
                { "const": 1 },
                { "not": { "enum": [1, 2] } }
            ]
        }
    }));
    let sup = resolve(json!({ "anyOf": [
        { "enum": [2, 3] },
        { "enum": ["a", "b"] }
    ] }));

    // The subset simplifies to the singleton value 2.
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn negated_union_with_complement_arm_implies_positive_arm() {
    let sub = resolve(json!({
        "not": {
            "anyOf": [
                { "not": { "const": "ok" } },
                { "const": "bad" }
            ]
        }
    }));
    let sup = resolve(json!({ "enum": ["ok", "other"] }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn negated_oneof_complement_pair_disjoint_reduces_to_union() {
    let sub = resolve(json!({
        "not": { "oneOf": [
            { "const": false },
            { "not": { "type": "integer" } }
        ] }
    }));
    let sup = resolve(json!({ "anyOf": [
        { "const": false },
        { "type": "integer" }
    ] }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn negated_oneof_complement_pair_finite_cancellation() {
    let sub = resolve(json!({
        "not": { "oneOf": [
            { "not": { "enum": [null, false] } },
            { "type": "null" }
        ] }
    }));
    let sup = resolve(json!({ "const": false }));

    // The xor complement reduces to the finite symmetric difference
    // between {null,false} and {null}, i.e. just false.
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn negated_oneof_complement_pair_overlap_uses_union_bound() {
    let sub = resolve(json!({
        "not": { "oneOf": [
            { "type": "string" },
            { "not": { "enum": [1, "a", null] } }
        ] }
    }));
    let sup = resolve(json!({ "anyOf": [
        { "type": "string" },
        { "enum": [1, "a", null] }
    ] }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn negated_oneof_complement_pair_comparable_reduces_to_larger_side() {
    let sub = resolve(json!({
        "not": { "oneOf": [
            { "enum": [1, 2] },
            { "not": { "type": "integer" } }
        ] }
    }));
    let sup = resolve(json!({ "type": "integer" }));

    // The positive enum arm is contained in the integer arm, so the
    // complement of the xor can only contain integers.
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn negated_singleton_covered_by_finite_complement_gap_union() {
    let sub = resolve(json!({ "not": { "const": 1 } }));
    let sup = resolve(json!({
        "anyOf": [
            { "not": { "enum": [1, 2] } },
            { "const": 2 }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn allof_of_finite_negations_implies_negated_enum() {
    let sub = resolve(json!({
        "allOf": [
            { "not": { "const": 1 } },
            { "not": { "const": 2 } }
        ]
    }));
    let sup = resolve(json!({ "not": { "enum": [1, 2] } }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn negated_allof_is_covered_by_anyof_of_negated_applicability_conjuncts() {
    // `minLength` without an explicit type is normalized as an
    // applicability union (non-strings plus constrained strings).  The
    // De Morgan cover still holds after that expansion.
    let sub = resolve(json!({
        "not": {
            "allOf": [
                { "type": "string" },
                { "minLength": 2 }
            ]
        }
    }));
    let sup = resolve(json!({
        "anyOf": [
            { "not": { "type": "string" } },
            { "not": { "minLength": 2 } }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn double_negation_on_left_delegates_to_inner_subset() {
    let sub = resolve(json!({ "not": { "not": { "enum": [1, "a"] } } }));
    let sup = resolve(json!({ "anyOf": [
        { "enum": [1, "a"] },
        { "const": 2 }
    ] }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn double_negation_on_right_delegates_to_inner_subset() {
    let sub = resolve(json!({ "enum": [1, "a"] }));
    let sup = resolve(json!({ "not": { "not": { "anyOf": [
        { "const": 1 },
        { "const": "a" },
        { "const": 2 }
    ] } } }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn negation_is_contravariant_for_proved_inner_subset() {
    let sub = resolve(json!({ "not": { "type": "string" } }));
    let sup = resolve(json!({ "not": { "enum": ["blocked"] } }));

    // enum["blocked"] is a subset of string, so complement(string) is a
    // subset of complement(enum["blocked"]).
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn negation_contravariance_does_not_reverse_unproved_inner_subset() {
    let sub = resolve(json!({ "not": { "enum": ["blocked"] } }));
    let sup = resolve(json!({ "not": { "type": "string" } }));

    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn overlapping_negation_stays_conservative() {
    let sub = resolve(json!({ "type": "string" }));
    let sup = resolve(json!({ "not": { "type": "string", "minLength": 2 } }));

    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn numeric_disjoint_negation_accepts_bounded_subset() {
    let sub = resolve(json!({ "type": "integer", "minimum": 0, "maximum": 4 }));
    let sup = resolve(json!({ "not": { "type": "number", "minimum": 5, "maximum": 10 } }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn discriminator_disjoint_negation_accepts_object_subset() {
    let sub = resolve(json!({
        "type": "object",
        "required": ["kind"],
        "properties": { "kind": { "const": "cat" } }
    }));
    let sup = resolve(json!({
        "not": {
            "type": "object",
            "required": ["kind"],
            "properties": { "kind": { "const": "dog" } }
        }
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn overlapping_oneof_branch_stays_conservative_for_nonfinite_schema() {
    let sub = resolve(json!({ "type": "string" }));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "string" },
            { "anyOf": [{ "type": "string" }, { "type": "integer" }] }
        ]
    }));

    // Every string would match both branches, so this is not a superset.
    assert!(!is_subschema_of(&sub, &sup));
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
fn optional_false_prefix_item_implies_shorter_max_items() {
    let old = resolve(json!({
        "type": "array",
        "maxItems": 1
    }));
    let new = resolve(json!({
        "type": "array",
        "prefixItems": [
            { "const": "ok" },
            false,
            { "type": "string" }
        ]
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
fn dependent_required_transitive_chain_preserves_a_required_dependency() {
    let old = resolve(json!({
        "type": "object",
        "properties": {
            "a": true,
            "c": true
        },
        "dependentRequired": {
            "a": ["c"]
        }
    }));
    let new = resolve(json!({
        "type": "object",
        "properties": {
            "a": true,
            "b": true,
            "c": true,
            "extra": { "type": "string" }
        },
        "dependentRequired": {
            "a": ["b"],
            "b": ["c"]
        }
    }));

    assert!(is_subschema_of(&new, &old));
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
fn dependent_required_trigger_that_overflows_max_properties_is_vacuous() {
    let old = resolve(json!({
        "type": "object",
        "dependentRequired": {
            "a": ["c"]
        }
    }));
    let new = resolve(json!({
        "type": "object",
        "maxProperties": 1,
        "dependentRequired": {
            "a": ["b"]
        }
    }));

    // In the subset schema, an object containing `a` would also need `b`,
    // which cannot fit under maxProperties: 1. Therefore `a` is not a
    // realizable trigger for the superset dependency.
    assert!(is_subschema_of(&new, &old));
}

#[test]
fn dependent_required_trigger_forcing_rejected_name_is_vacuous() {
    let old = resolve(json!({
        "type": "object",
        "dependentRequired": { "a": ["c"] }
    }));
    let new = resolve(json!({
        "type": "object",
        "propertyNames": { "pattern": "^a$" },
        "dependentRequired": { "a": ["b"] }
    }));

    assert!(is_subschema_of(&new, &old));
}

#[test]
fn dependent_required_trigger_forcing_false_property_is_vacuous() {
    let old = resolve(json!({
        "type": "object",
        "dependentRequired": { "a": ["c"] }
    }));
    let new = resolve(json!({
        "type": "object",
        "properties": { "b": false },
        "dependentRequired": { "a": ["b"] }
    }));

    assert!(is_subschema_of(&new, &old));
}

#[test]
fn dependent_required_trigger_forcing_dead_enum_property_is_vacuous() {
    let old = resolve(json!({
        "type": "object",
        "dependentRequired": { "a": ["c"] }
    }));
    let new = resolve(json!({
        "type": "object",
        "properties": {
            "b": { "type": "integer", "enum": ["not-an-integer"] }
        },
        "dependentRequired": { "a": ["b"] }
    }));

    assert!(is_subschema_of(&new, &old));
}

#[test]
fn dependent_required_trigger_with_unsupported_property_name_pattern_is_not_vacuous() {
    let old = resolve(json!({
        "type": "object",
        "dependentRequired": {
            "x": ["y"]
        }
    }));
    let new = resolve(json!({
        "type": "object",
        "propertyNames": {
            "pattern": "^(?=x$)x$"
        }
    }));

    assert!(!is_subschema_of(&new, &old));
}

#[test]
fn dependent_required_trigger_with_recursive_property_names_is_not_vacuous() {
    let old = resolve(json!({
        "type": "object",
        "dependentRequired": {
            "x": ["y"]
        }
    }));
    let raw = json!({
        "$defs": {
            "Name": {
                "allOf": [
                    { "$ref": "#/$defs/Name" },
                    { "type": "string" }
                ]
            }
        },
        "type": "object",
        "propertyNames": { "$ref": "#/$defs/Name" }
    });
    let new_document = SchemaDocument::from_json(&raw).unwrap();
    assert!(new_document.is_valid(&json!({ "x": 1 })).unwrap());
    let new = new_document.root().unwrap().clone();

    assert!(!is_subschema_of(&new, &old));
}

#[test]
fn dependent_required_trigger_admitted_by_unsupported_pattern_properties_is_not_vacuous() {
    let old = resolve(json!({
        "type": "object",
        "dependentRequired": {
            "x": ["y"]
        }
    }));
    let new = resolve(json!({
        "type": "object",
        "patternProperties": {
            "^(?=x$)x$": true
        },
        "additionalProperties": false
    }));

    assert!(!is_subschema_of(&new, &old));
}

#[test]
fn dependent_required_trigger_forbidden_by_matching_pattern_property_is_vacuous() {
    let old = resolve(json!({
        "type": "object",
        "dependentRequired": {
            "x": ["y"]
        }
    }));
    let new = resolve(json!({
        "type": "object",
        "properties": {
            "x": true
        },
        "patternProperties": {
            "^x$": false
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
fn unsupported_superset_pattern_properties_may_constrain_explicit_subset_properties() {
    let old = resolve(json!({
        "type": "object",
        "patternProperties": {
            "^(?=x$)x$": { "type": "integer" }
        },
        "additionalProperties": true
    }));
    let new = resolve(json!({
        "type": "object",
        "properties": {
            "x": true
        },
        "additionalProperties": false
    }));

    assert!(!is_subschema_of(&new, &old));
}

#[test]
fn overlapping_pattern_properties_must_preserve_every_superset_constraint() {
    let old = resolve(json!({
        "type": "object",
        "patternProperties": {
            "^x": { "type": "string" },
            "x$": { "type": "integer" }
        },
        "additionalProperties": false
    }));
    let new = resolve(json!({
        "type": "object",
        "patternProperties": {
            "^x": { "type": "string" }
        },
        "additionalProperties": false
    }));

    assert!(!is_subschema_of(&new, &old));
}

#[test]
fn closed_object_property_capacity_must_respect_implicit_max_properties() {
    let old = resolve(json!({
        "type": "object",
        "properties": {
            "x": true
        },
        "maxProperties": 1,
        "additionalProperties": false
    }));
    let new = resolve(json!({
        "type": "object",
        "properties": {
            "x": true
        },
        "maxProperties": 3,
        "additionalProperties": false
    }));

    assert!(is_subschema_of(&new, &old));
}

#[test]
fn unsupported_subset_pattern_properties_cannot_fall_back_to_additional_properties() {
    let old = resolve(json!({
        "type": "object",
        "properties": {
            "x": { "type": "string" }
        },
        "additionalProperties": true
    }));
    let new = resolve(json!({
        "type": "object",
        "patternProperties": {
            "^\\cC$": { "type": "integer" }
        },
        "additionalProperties": false
    }));

    assert!(!is_subschema_of(&new, &old));
    assert_eq!(
        explain_subschema_failure(&new, &old)
            .expect("failure should be explainable")
            .render("new", "old"),
        "old schema #/properties/x: property 'x' can appear with values the comparison target rejects",
    );
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

#[test]
fn differing_string_patterns_are_not_treated_as_subsumed() {
    let old = resolve(json!({
        "type": "string",
        "pattern": "^a+$"
    }));
    let new = resolve(json!({
        "type": "string",
        "pattern": "^b+$"
    }));

    assert!(!is_subschema_of(&new, &old));
}

#[test]
fn differing_string_formats_remain_subsumed_as_annotations() {
    let old = resolve(json!({
        "type": "string",
        "format": "email"
    }));
    let new = resolve(json!({
        "type": "string",
        "format": "uuid"
    }));

    assert!(is_subschema_of(&new, &old));
}

#[test]
fn identical_string_language_constraints_remain_subsumed() {
    let old = resolve(json!({
        "type": "string",
        "pattern": "^a+$"
    }));
    let new = resolve(json!({
        "type": "string",
        "pattern": "^a+$",
        "minLength": 1
    }));

    assert!(is_subschema_of(&new, &old));
}

#[test]
fn impossible_required_false_property_is_vacuous() {
    let impossible = resolve(json!({
        "type": "object",
        "required": ["x"],
        "properties": {
            "x": false
        }
    }));
    let arbitrary_object = resolve(json!({
        "type": "object",
        "required": ["y"]
    }));

    assert!(is_subschema_of(&impossible, &arbitrary_object));
}

#[test]
fn impossible_required_property_with_dead_enum_is_vacuous() {
    let impossible = resolve(json!({
        "type": "object",
        "required": ["x"],
        "properties": {
            "x": { "type": "integer", "enum": ["not-an-integer"] }
        }
    }));
    let arbitrary_object = resolve(json!({
        "type": "object",
        "required": ["y"]
    }));

    assert!(is_subschema_of(&impossible, &arbitrary_object));
}

#[test]
fn impossible_required_name_rejected_by_property_names_is_vacuous() {
    let impossible = resolve(json!({
        "type": "object",
        "required": ["x"],
        "propertyNames": {
            "const": "y"
        }
    }));
    let arbitrary_object = resolve(json!({
        "type": "object",
        "required": ["z"]
    }));

    assert!(is_subschema_of(&impossible, &arbitrary_object));
}

#[test]
fn impossible_min_properties_exceeds_closed_declared_names_is_vacuous() {
    let impossible = resolve(json!({
        "type": "object",
        "minProperties": 2,
        "properties": {
            "a": { "type": "string" }
        },
        "additionalProperties": false
    }));
    let arbitrary_object = resolve(json!({
        "type": "object",
        "required": ["z"]
    }));

    assert!(is_subschema_of(&impossible, &arbitrary_object));
}

#[test]
fn impossible_min_properties_exceeds_finite_property_names_is_vacuous() {
    let impossible = resolve(json!({
        "type": "object",
        "minProperties": 2,
        "propertyNames": { "enum": ["only"] }
    }));
    let arbitrary_object = resolve(json!({
        "type": "object",
        "required": ["z"]
    }));

    assert!(is_subschema_of(&impossible, &arbitrary_object));
}

#[test]
fn impossible_min_properties_exceeds_conditional_finite_property_names_is_vacuous() {
    let names = json!({
        "if": { "type": "string" },
        "then": { "enum": ["only"] }
    });
    let impossible = resolve(json!({
        "type": "object",
        "minProperties": 2,
        "propertyNames": names
    }));
    let arbitrary_object = resolve(json!({
        "type": "object",
        "required": ["z"]
    }));

    assert!(is_subschema_of(&impossible, &arbitrary_object));
}

#[test]
fn conditional_missing_then_finite_property_names_bound_count() {
    let names = json!({
        "if": { "enum": ["a"] },
        "else": { "enum": ["b"] }
    });
    let sub = resolve(json!({
        "type": "object",
        "propertyNames": names.clone()
    }));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "object", "propertyNames": names },
            { "type": "object", "minProperties": 3 }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn conditional_missing_else_negated_finite_property_names_bound_count() {
    let names = json!({
        "if": { "not": { "enum": ["a"] } },
        "then": { "enum": ["b"] }
    });
    let sub = resolve(json!({
        "type": "object",
        "propertyNames": names.clone()
    }));
    let sup = resolve(json!({
        "oneOf": [
            { "type": "object", "propertyNames": names },
            { "type": "object", "minProperties": 3 }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn impossible_min_properties_excludes_declared_names_with_impossible_dependencies() {
    let impossible = resolve(json!({
        "type": "object",
        "minProperties": 2,
        "properties": {
            "a": true,
            "b": true
        },
        "dependentRequired": {
            "a": ["missing"],
            "b": ["missing"]
        },
        "additionalProperties": false
    }));
    let arbitrary_object = resolve(json!({
        "type": "object",
        "required": ["z"]
    }));

    assert!(is_subschema_of(&impossible, &arbitrary_object));
}

#[test]
fn finite_property_names_avoid_global_additional_properties_requirement() {
    let sub = resolve(json!({
        "type": "object",
        "propertyNames": { "enum": ["a"] },
        "additionalProperties": { "type": "string" }
    }));
    let sup = resolve(json!({
        "type": "object",
        "properties": {
            "a": { "type": "string" }
        },
        "additionalProperties": false
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn dependent_required_from_required_property_guarantees_sup_required_name() {
    let sub = resolve(json!({
        "type": "object",
        "required": ["a"],
        "dependentRequired": { "a": ["b"] },
        "maxProperties": 2
    }));
    let sup = resolve(json!({
        "type": "object",
        "required": ["b"]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn dependent_required_from_required_property_counts_toward_min_properties() {
    let sub = resolve(json!({
        "type": "object",
        "required": ["a"],
        "dependentRequired": { "a": ["b"] }
    }));
    let sup = resolve(json!({
        "type": "object",
        "minProperties": 2
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn optional_dependent_required_does_not_imply_min_properties() {
    let sub = resolve(json!({
        "type": "object",
        "dependentRequired": { "a": ["b"] }
    }));
    let sup = resolve(json!({
        "type": "object",
        "minProperties": 2
    }));

    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn impossible_dependent_required_overflows_property_budget_is_vacuous() {
    let impossible = resolve(json!({
        "type": "object",
        "required": ["a"],
        "maxProperties": 1,
        "dependentRequired": { "a": ["b"] }
    }));
    let arbitrary_object = resolve(json!({
        "type": "object",
        "required": ["z"]
    }));

    assert!(is_subschema_of(&impossible, &arbitrary_object));
}

#[test]
fn impossible_required_false_prefix_item_is_vacuous() {
    let impossible = resolve(json!({
        "type": "array",
        "minItems": 1,
        "prefixItems": [false]
    }));
    let arbitrary_array = resolve(json!({
        "type": "array",
        "minItems": 2
    }));

    assert!(is_subschema_of(&impossible, &arbitrary_array));
}

#[test]
fn impossible_required_prefix_item_with_dead_enum_is_vacuous() {
    let impossible = resolve(json!({
        "type": "array",
        "minItems": 1,
        "prefixItems": [{ "type": "string", "enum": [1] }]
    }));
    let arbitrary_array = resolve(json!({
        "type": "array",
        "minItems": 2
    }));

    assert!(is_subschema_of(&impossible, &arbitrary_array));
}

#[test]
fn impossible_required_allof_finite_domain_item_is_vacuous() {
    let impossible = resolve(json!({
        "type": "array",
        "minItems": 1,
        "prefixItems": [{
            "allOf": [
                { "enum": [1] },
                { "type": "string" }
            ]
        }]
    }));
    let arbitrary_array = resolve(json!({
        "type": "array",
        "minItems": 2
    }));

    assert!(is_subschema_of(&impossible, &arbitrary_array));
}

#[test]
fn impossible_required_false_tail_item_is_vacuous() {
    let impossible = resolve(json!({
        "type": "array",
        "minItems": 2,
        "prefixItems": [{ "type": "string" }],
        "items": false
    }));
    let arbitrary_array = resolve(json!({
        "type": "array",
        "minItems": 3
    }));

    assert!(is_subschema_of(&impossible, &arbitrary_array));
}

#[test]
fn impossible_unique_tuple_duplicate_singletons_is_vacuous() {
    let impossible = resolve(json!({
        "type": "array",
        "uniqueItems": true,
        "minItems": 2,
        "prefixItems": [
            { "const": "same" },
            { "enum": ["same"] }
        ]
    }));
    let arbitrary_array = resolve(json!({
        "type": "array",
        "minItems": 10
    }));

    assert!(is_subschema_of(&impossible, &arbitrary_array));
}

#[test]
fn impossible_unique_tuple_hall_subset_is_vacuous() {
    // The first three required positions have only two possible values.
    // A fourth, unrelated position makes the total union large enough that
    // a simple global pigeonhole check would miss the contradiction.
    let impossible = resolve(json!({
        "type": "array",
        "uniqueItems": true,
        "minItems": 4,
        "prefixItems": [
            { "enum": [1, 2] },
            { "enum": [1, 2] },
            { "enum": [1, 2] },
            { "enum": [3, 4, 5] }
        ]
    }));
    let arbitrary_array = resolve(json!({
        "type": "array",
        "minItems": 10
    }));

    assert!(is_subschema_of(&impossible, &arbitrary_array));
}

#[test]
fn unique_prefix_tail_overlap_tightens_effective_max_items() {
    let subset = resolve(json!({
        "type": "array",
        "uniqueItems": true,
        "prefixItems": [{ "const": 1 }],
        "items": { "enum": [1, 2] }
    }));
    let superset = resolve(json!({
        "type": "array",
        "maxItems": 2
    }));

    assert!(is_subschema_of(&subset, &superset));
}

#[test]
fn finite_item_domain_disjoint_from_contains_satisfies_max_contains_zero() {
    let subset = resolve(json!({
        "type": "array",
        "items": { "enum": [1, 2] }
    }));
    let superset = resolve(json!({
        "type": "array",
        "contains": { "const": 3 },
        "minContains": 0,
        "maxContains": 0
    }));

    assert!(is_subschema_of(&subset, &superset));
}

#[test]
fn unique_boolean_items_have_finite_capacity() {
    let subset = resolve(json!({
        "type": "array",
        "uniqueItems": true,
        "items": { "type": "boolean" }
    }));
    let superset = resolve(json!({
        "type": "array",
        "maxItems": 2
    }));

    assert!(is_subschema_of(&subset, &superset));
}

#[test]
fn unique_anyof_finite_item_domain_has_finite_capacity() {
    let subset = resolve(json!({
        "type": "array",
        "uniqueItems": true,
        "items": {
            "anyOf": [
                { "const": "a" },
                { "const": "b" }
            ]
        }
    }));
    let superset = resolve(json!({
        "type": "array",
        "maxItems": 2
    }));

    assert!(is_subschema_of(&subset, &superset));
}

#[test]
fn unique_conditional_finite_item_domain_has_finite_capacity() {
    let subset = resolve(json!({
        "type": "array",
        "uniqueItems": true,
        "items": {
            "if": { "type": "string" },
            "then": { "enum": ["a"] },
            "else": { "enum": [1] }
        }
    }));
    let superset = resolve(json!({
        "type": "array",
        "maxItems": 2
    }));

    assert!(is_subschema_of(&subset, &superset));
}

#[test]
fn unique_conditional_missing_then_finite_condition_has_capacity() {
    let subset = resolve(json!({
        "type": "array",
        "uniqueItems": true,
        "items": {
            "if": { "enum": [1, 2] },
            "else": { "const": 3 }
        }
    }));
    let superset = resolve(json!({
        "type": "array",
        "maxItems": 3
    }));

    assert!(is_subschema_of(&subset, &superset));
}

#[test]
fn unique_conditional_missing_else_finite_negated_condition_has_capacity() {
    let subset = resolve(json!({
        "type": "array",
        "uniqueItems": true,
        "items": {
            "if": { "not": { "enum": [1, 2] } },
            "then": { "const": 3 }
        }
    }));
    let superset = resolve(json!({
        "type": "array",
        "maxItems": 3
    }));

    assert!(is_subschema_of(&subset, &superset));
}

#[test]
fn unique_zero_sized_object_and_array_items_have_singleton_capacity() {
    let empty_objects = resolve(json!({
        "type": "array",
        "uniqueItems": true,
        "items": { "type": "object", "maxProperties": 0 }
    }));
    let at_most_one = resolve(json!({
        "type": "array",
        "maxItems": 1
    }));
    assert!(is_subschema_of(&empty_objects, &at_most_one));

    let empty_arrays = resolve(json!({
        "type": "array",
        "uniqueItems": true,
        "items": { "type": "array", "maxItems": 0 }
    }));
    assert!(is_subschema_of(&empty_arrays, &at_most_one));
}

#[test]
fn unique_closed_object_item_domain_has_finite_capacity() {
    let subset = resolve(json!({
        "type": "array",
        "uniqueItems": true,
        "items": {
            "type": "object",
            "properties": {
                "flag": { "type": "boolean" }
            },
            "additionalProperties": false
        }
    }));
    let superset = resolve(json!({
        "type": "array",
        "maxItems": 3
    }));

    // The only possible item objects are {}, {flag:false}, and {flag:true}.
    assert!(is_subschema_of(&subset, &superset));
}

#[test]
fn unique_closed_object_ignores_impossible_recursive_optional_property() {
    let subset = resolve(json!({
        "$defs": {
            "Obj": {
                "type": "object",
                "properties": {
                    "flag": { "type": "boolean" },
                    "child": { "$ref": "#/$defs/Obj" }
                },
                "required": ["flag"],
                "maxProperties": 1,
                "additionalProperties": false
            }
        },
        "type": "array",
        "uniqueItems": true,
        "items": { "$ref": "#/$defs/Obj" }
    }));
    let superset = resolve(json!({
        "type": "array",
        "maxItems": 2
    }));

    assert!(is_subschema_of(&subset, &superset));
}

#[test]
fn disjoint_tuple_positions_imply_unique_items() {
    let subset = resolve(json!({
        "type": "array",
        "prefixItems": [
            { "const": "id" },
            { "type": "integer", "minimum": 1, "maximum": 2 },
            { "const": true }
        ],
        "items": false
    }));
    let superset = resolve(json!({
        "type": "array",
        "prefixItems": [
            { "const": "id" },
            { "type": "integer", "minimum": 1, "maximum": 2 },
            { "const": true }
        ],
        "items": false,
        "uniqueItems": true
    }));

    assert!(is_subschema_of(&subset, &superset));
}

#[test]
fn identical_conditionals_compare_branchwise() {
    let subset = resolve(json!({
        "if": { "type": "string" },
        "then": { "type": "string", "maxLength": 3 },
        "else": { "type": "integer", "minimum": 2, "maximum": 4 }
    }));
    let superset = resolve(json!({
        "if": { "type": "string" },
        "then": { "type": "string", "maxLength": 5 },
        "else": { "type": "integer", "minimum": 1, "maximum": 5 }
    }));

    assert!(is_subschema_of(&subset, &superset));
}

#[test]
fn subset_implying_superset_condition_only_needs_then_branch() {
    let subset = resolve(json!({ "type": "string", "maxLength": 3 }));
    let superset = resolve(json!({
        "if": { "type": "string" },
        "then": { "type": "string", "maxLength": 5 },
        "else": { "type": "integer" }
    }));

    assert!(is_subschema_of(&subset, &superset));
}

#[test]
fn subset_disjoint_from_superset_condition_only_needs_else_branch() {
    let subset = resolve(json!({ "type": "integer", "minimum": 2, "maximum": 4 }));
    let superset = resolve(json!({
        "if": { "type": "string" },
        "then": { "type": "string", "maxLength": 5 },
        "else": { "type": "integer", "minimum": 1, "maximum": 5 }
    }));

    assert!(is_subschema_of(&subset, &superset));
}

#[test]
fn subset_with_negated_guard_only_needs_else_branch() {
    let subset = resolve(json!({
        "allOf": [
            { "type": "string" },
            { "not": { "const": "debug" } }
        ]
    }));
    let superset = resolve(json!({
        "if": { "const": "debug" },
        "then": { "type": "integer" },
        "else": { "type": "string" }
    }));

    assert!(is_subschema_of(&subset, &superset));
}

#[test]
fn unrelated_negated_guard_does_not_force_else_branch() {
    let subset = resolve(json!({
        "allOf": [
            { "type": "string" },
            { "not": { "const": "release" } }
        ]
    }));
    let superset = resolve(json!({
        "if": { "const": "debug" },
        "then": { "type": "integer" },
        "else": { "type": "string" }
    }));

    assert!(!is_subschema_of(&subset, &superset));
}

#[test]
fn subset_contained_by_both_superset_conditional_branches() {
    let subset = resolve(json!({
        "type": "string",
        "maxLength": 3
    }));
    let superset = resolve(json!({
        "if": { "type": "number" },
        "then": { "type": "string", "maxLength": 5 },
        "else": { "type": "string", "maxLength": 5 }
    }));

    assert!(is_subschema_of(&subset, &superset));
}

#[test]
fn superset_conditional_requires_all_possible_branches() {
    let subset = resolve(json!({ "type": "string", "maxLength": 3 }));
    let superset = resolve(json!({
        "if": { "type": "number" },
        "then": { "type": "string", "maxLength": 5 },
        "else": { "type": "integer" }
    }));

    assert!(!is_subschema_of(&subset, &superset));
}

#[test]
fn conditional_with_bounded_branches_subsets_common_target() {
    let subset = resolve(json!({
        "if": { "type": "string" },
        "then": { "enum": ["a", "b"] },
        "else": { "enum": [1, 2] }
    }));
    let superset = resolve(json!({
        "anyOf": [
            { "enum": ["a", "b"] },
            { "enum": [1, 2] }
        ]
    }));

    assert!(is_subschema_of(&subset, &superset));
}

#[test]
fn conditional_string_branches_subset_common_string_target() {
    let subset = resolve(json!({
        "if": { "maxLength": 3 },
        "then": { "type": "string", "minLength": 1, "maxLength": 3 },
        "else": { "type": "string", "minLength": 4, "maxLength": 8 }
    }));
    let superset = resolve(json!({
        "type": "string",
        "maxLength": 8
    }));

    assert!(is_subschema_of(&subset, &superset));
}

#[test]
fn conditional_missing_then_uses_guard_and_else_cover() {
    let subset = resolve(json!({
        "if": { "type": "integer" },
        "else": { "enum": [false, null] }
    }));
    let superset = resolve(json!({
        "anyOf": [
            { "type": "integer" },
            { "enum": [false, null] }
        ]
    }));

    assert!(is_subschema_of(&subset, &superset));
}

#[test]
fn conditional_missing_then_requires_guard_cover() {
    let subset = resolve(json!({
        "if": { "type": "integer" },
        "else": { "const": false }
    }));
    let superset = resolve(json!({ "enum": [false] }));

    assert!(!is_subschema_of(&subset, &superset));
}

#[test]
fn conditional_vacuous_then_branch_can_use_else_subset() {
    let subset = resolve(json!({
        "if": { "type": "boolean" },
        "then": { "anyOf": [{ "type": "object" }] },
        "else": { "const": 1 }
    }));
    let superset = resolve(json!({ "type": "integer" }));

    assert!(is_subschema_of(&subset, &superset));
}

#[test]
fn conditional_vacuous_then_detects_negated_guard_literal() {
    let subset = resolve(json!({
        "if": { "not": { "const": 1 } },
        "then": { "const": 1 },
        "else": { "const": 1 }
    }));
    let superset = resolve(json!({ "const": 1 }));

    // The then side is `not const(1) && const(1)`, so it is empty even
    // though both schemas have the same broad numeric type mask.
    assert!(is_subschema_of(&subset, &superset));
}

#[test]
fn conditional_vacuous_else_branch_can_use_then_subset() {
    let subset = resolve(json!({
        "if": { "type": "integer" },
        "then": { "const": 1 },
        "else": { "type": "integer" }
    }));
    let superset = resolve(json!({ "const": 1 }));

    assert!(is_subschema_of(&subset, &superset));
}

#[test]
fn conditional_overlapping_else_branch_stays_live() {
    let subset = resolve(json!({
        "if": { "type": "integer" },
        "then": { "const": 1 },
        "else": { "type": "string" }
    }));
    let superset = resolve(json!({ "const": 1 }));

    assert!(!is_subschema_of(&subset, &superset));
}

#[test]
fn conditional_branches_can_target_union_collectively() {
    let subset = resolve(json!({
        "if": { "type": "string" },
        "then": { "type": "string", "maxLength": 4 },
        "else": { "type": "integer", "minimum": 0, "maximum": 10 }
    }));
    let superset = resolve(json!({
        "anyOf": [
            { "type": "string", "maxLength": 8 },
            { "type": "integer", "minimum": 0, "maximum": 20 }
        ]
    }));

    assert!(is_subschema_of(&subset, &superset));
}

#[test]
fn unique_small_integer_range_has_finite_capacity() {
    let subset = resolve(json!({
        "type": "array",
        "uniqueItems": true,
        "items": {
            "type": "integer",
            "minimum": 1,
            "maximum": 3
        }
    }));
    let superset = resolve(json!({
        "type": "array",
        "maxItems": 3
    }));

    assert!(is_subschema_of(&subset, &superset));
}

#[test]
fn unique_split_allof_integer_range_has_finite_capacity() {
    let subset = resolve(json!({
        "type": "array",
        "uniqueItems": true,
        "items": {
            "allOf": [
                { "type": "integer", "minimum": 0 },
                { "type": "integer", "maximum": 1 }
            ]
        }
    }));
    let superset = resolve(json!({
        "type": "array",
        "maxItems": 2
    }));

    assert!(is_subschema_of(&subset, &superset));
}

#[test]
fn finite_allof_integer_domain_can_prove_enum_subset() {
    let subset = resolve(json!({
        "allOf": [
            {
                "type": "integer",
                "minimum": 1,
                "maximum": 3
            },
            { "type": "integer", "multipleOf": 2 }
        ]
    }));
    let superset = resolve(json!({ "enum": [2] }));

    assert!(is_subschema_of(&subset, &superset));
}

#[test]
fn finite_allof_domain_filters_rejected_candidates_for_capacity() {
    let subset = resolve(json!({
        "type": "array",
        "uniqueItems": true,
        "items": {
            "allOf": [
                { "enum": [1, 2] },
                { "not": { "const": 2 } }
            ]
        }
    }));
    let superset = resolve(json!({
        "type": "array",
        "maxItems": 1
    }));

    assert!(is_subschema_of(&subset, &superset));
}

#[test]
fn zero_length_property_names_cap_object_capacity() {
    let impossible = resolve(json!({
        "type": "object",
        "minProperties": 2,
        "propertyNames": { "type": "string", "maxLength": 0 }
    }));
    let arbitrary_object = resolve(json!({
        "type": "object",
        "required": ["z"]
    }));

    assert!(is_subschema_of(&impossible, &arbitrary_object));
}

#[test]
fn allof_finite_child_caps_property_names() {
    let impossible = resolve(json!({
        "type": "object",
        "minProperties": 3,
        "propertyNames": {
            "allOf": [
                { "enum": ["a", "b"] },
                { "type": "string" }
            ]
        }
    }));
    let arbitrary_object = resolve(json!({
        "type": "object",
        "required": ["z"]
    }));

    assert!(is_subschema_of(&impossible, &arbitrary_object));
}

#[test]
fn recursive_trivial_universal_probe_does_not_overflow() {
    let schema = resolve(json!({
        "$defs": {
            "U": { "allOf": [true, { "$ref": "#/$defs/U" }] }
        },
        "$ref": "#/$defs/U"
    }));

    // Cyclic proofs deliberately give up rather than recursing forever.
    assert!(!schema_is_trivially_universal(&schema));
}

#[test]
fn unique_number_multiple_domain_is_not_treated_as_finite_capacity() {
    let subset = resolve(json!({
        "type": "array",
        "uniqueItems": true,
        "items": {
            "type": "number",
            "minimum": 0,
            "maximum": 4,
            "multipleOf": 2
        }
    }));
    let superset = resolve(json!({
        "type": "array",
        "maxItems": 3
    }));

    // Near-multiple floating values make this domain infinite under the
    // validator's epsilon semantics, so do not cap uniqueItems by the
    // projected integer multiples.
    assert!(!is_subschema_of(&subset, &superset));
}

#[test]
fn unique_finite_property_names_with_supported_patterns_has_capacity() {
    let subset = resolve(json!({
        "type": "array",
        "uniqueItems": true,
        "items": {
            "type": "object",
            "propertyNames": { "enum": ["x", "y"] },
            "patternProperties": {
                "^x$": { "const": 1 }
            },
            "additionalProperties": { "const": 2 }
        }
    }));
    let superset = resolve(json!({
        "type": "array",
        "maxItems": 4
    }));

    assert!(is_subschema_of(&subset, &superset));
}

#[test]
fn unique_open_object_finite_property_names_and_values_has_capacity() {
    let subset = resolve(json!({
        "type": "array",
        "uniqueItems": true,
        "items": {
            "type": "object",
            "propertyNames": { "enum": ["a"] },
            "additionalProperties": { "type": "boolean" }
        }
    }));
    let superset = resolve(json!({
        "type": "array",
        "maxItems": 3
    }));

    assert!(is_subschema_of(&subset, &superset));
}

#[test]
fn impossible_false_contains_requirement_is_vacuous() {
    let impossible = resolve(json!({
        "type": "array",
        "contains": false,
        "minContains": 1
    }));
    let arbitrary_array = resolve(json!({
        "type": "array",
        "minItems": 10
    }));

    assert!(is_subschema_of(&impossible, &arbitrary_array));
}

#[test]
fn impossible_contains_count_above_max_items_is_vacuous() {
    let impossible = resolve(json!({
        "type": "array",
        "maxItems": 1,
        "contains": { "type": "string" },
        "minContains": 2
    }));
    let arbitrary_array = resolve(json!({
        "type": "array",
        "minItems": 10
    }));

    assert!(is_subschema_of(&impossible, &arbitrary_array));
}

#[test]
fn anyof_required_or_property_schema_is_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "required": ["tag"] },
            { "properties": { "tag": { "const": "ok" } } }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn property_presence_cover_with_extra_absence_constraint_is_not_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "required": ["tag"] },
            { "properties": { "tag": { "const": "ok" }, "other": { "const": 1 } } }
        ]
    }));

    // An object like {"other": 2} has no tag and fails the second branch.
    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn explicit_non_object_complement_with_presence_split_is_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "not": { "type": "object" } },
            { "type": "object", "required": ["tag"] },
            { "type": "object", "properties": { "tag": { "const": "ok" } } }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn required_or_same_trigger_dependent_required_is_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "required": ["tag"] },
            { "dependentRequired": { "tag": ["other"] } }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn required_or_other_trigger_dependent_required_is_not_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "required": ["tag"] },
            { "dependentRequired": { "other": ["missing"] } }
        ]
    }));

    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn anyof_object_count_threshold_cover_is_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "maxProperties": 0 },
            { "minProperties": 1 }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn anyof_empty_object_arm_plus_nonempty_count_cover_is_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "properties": { "tag": { "const": "ok" } } },
            { "minProperties": 1 }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn anyof_object_count_bridge_intervals_are_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "not": { "type": "object" } },
            { "type": "object", "maxProperties": 0 },
            { "type": "object", "minProperties": 1, "maxProperties": 2 },
            { "type": "object", "minProperties": 3 }
        ]
    }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn anyof_object_count_threshold_gap_is_not_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "maxProperties": 1 },
            { "minProperties": 3 }
        ]
    }));

    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn property_absence_and_dependent_target_presence_cover_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "properties": { "tag": { "const": "ok" } } },
            { "dependentRequired": { "other": ["tag"] } }
        ]
    }));

    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn dependent_target_presence_cover_with_extra_dependency_is_not_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "properties": { "tag": { "const": "ok" } } },
            { "dependentRequired": { "other": ["tag", "extra"] } }
        ]
    }));

    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn anyof_single_property_name_partition_with_count_gap_is_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "minProperties": 2 },
            { "maxProperties": 1, "propertyNames": { "enum": ["a"] } },
            { "maxProperties": 1, "propertyNames": { "not": { "enum": ["a"] } } }
        ]
    }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn property_name_partition_without_high_count_gap_is_not_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "maxProperties": 1, "propertyNames": { "enum": ["a"] } },
            { "maxProperties": 1, "propertyNames": { "not": { "enum": ["a"] } } }
        ]
    }));
    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn anyof_required_property_value_partition_is_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "not": { "required": ["a"] } },
            { "required": ["a"], "properties": { "a": { "enum": [1] } } },
            { "required": ["a"], "properties": { "a": { "not": { "enum": [1] } } } }
        ]
    }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn property_value_partition_with_extra_required_name_is_not_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "not": { "required": ["a"] } },
            { "required": ["a", "b"], "properties": { "a": { "enum": [1] } } },
            { "required": ["a"], "properties": { "a": { "not": { "enum": [1] } } } }
        ]
    }));
    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn anyof_prefix_item_complement_partition_is_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "prefixItems": [ { "enum": [1] } ] },
            { "prefixItems": [ { "not": { "enum": [1] } } ] }
        ]
    }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn prefix_item_partition_with_tail_bound_is_not_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "maxItems": 1, "prefixItems": [ { "enum": [1] } ] },
            { "maxItems": 1, "prefixItems": [ { "not": { "enum": [1] } } ] }
        ]
    }));
    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn anyof_items_or_contains_complement_is_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "items": { "enum": [1] } },
            { "contains": { "not": { "enum": [1] } } }
        ]
    }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn items_contains_partition_with_max_contains_is_not_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "items": { "enum": [1] } },
            { "contains": { "not": { "enum": [1] } }, "maxContains": 1 }
        ]
    }));
    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn anyof_not_items_or_contains_positive_is_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "items": { "not": { "enum": [1] } } },
            { "contains": { "enum": [1] } }
        ]
    }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn anyof_array_count_threshold_cover_is_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "maxItems": 1 },
            { "minItems": 2 }
        ]
    }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn anyof_array_count_bridge_intervals_are_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "not": { "type": "array" } },
            { "type": "array", "maxItems": 0 },
            { "type": "array", "minItems": 1, "maxItems": 2 },
            { "type": "array", "minItems": 3 }
        ]
    }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn anyof_array_count_gap_is_not_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "maxItems": 1 },
            { "minItems": 3 }
        ]
    }));
    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn anyof_string_length_threshold_cover_is_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "maxLength": 1 },
            { "minLength": 2 }
        ]
    }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn anyof_string_length_bridge_intervals_are_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "not": { "type": "string" } },
            { "type": "string", "maxLength": 1 },
            { "type": "string", "minLength": 2, "maxLength": 3 },
            { "type": "string", "minLength": 4 }
        ]
    }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn anyof_string_length_gap_is_not_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "maxLength": 1 },
            { "minLength": 3 }
        ]
    }));
    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn anyof_numeric_touching_range_cover_is_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "maximum": 1 },
            { "exclusiveMinimum": 1 }
        ]
    }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn anyof_numeric_bridge_intervals_are_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "not": { "type": "number" } },
            { "type": "number", "maximum": 0 },
            { "type": "number", "minimum": 0, "maximum": 2 },
            { "type": "number", "minimum": 2 }
        ]
    }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn anyof_numeric_bridge_open_point_gap_is_not_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "not": { "type": "number" } },
            { "type": "number", "exclusiveMaximum": 0 },
            { "type": "number", "exclusiveMinimum": 0, "maximum": 2 },
            { "type": "number", "minimum": 2 }
        ]
    }));
    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn anyof_numeric_open_point_gap_is_not_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "exclusiveMaximum": 1 },
            { "exclusiveMinimum": 1 }
        ]
    }));
    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn anyof_integer_lattice_with_noninteger_complement_is_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "not": { "type": "integer" } },
            { "type": "integer", "maximum": 1 },
            { "type": "integer", "minimum": 2 }
        ]
    }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn anyof_integer_lattice_bridge_intervals_cover_gap() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "not": { "type": "integer" } },
            { "type": "integer", "maximum": 0 },
            { "type": "integer", "minimum": 1, "maximum": 4 },
            { "type": "integer", "minimum": 5 }
        ]
    }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn anyof_integer_lattice_gap_with_noninteger_complement_is_not_universal() {
    let sub = resolve(json!({}));
    let sup = resolve(json!({
        "anyOf": [
            { "not": { "type": "integer" } },
            { "type": "integer", "maximum": 1 },
            { "type": "integer", "minimum": 3 }
        ]
    }));
    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn finite_oneof_of_complements_can_prove_xor_subset() {
    // The new schema accepts only []: it is the symmetric difference of
    // { } and { [], {} }.  The old schema accepts arrays (and 3), so the
    // finite xor is a safe subset even though neither complement branch is
    // finite by itself.
    let old = resolve(json!({
        "oneOf": [
            { "not": { "const": 3 } },
            { "not": { "type": "array" } }
        ]
    }));
    let new = resolve(json!({
        "oneOf": [
            { "not": { "const": {} } },
            { "not": { "enum": [[], {}] } }
        ]
    }));
    assert!(is_subschema_of(&new, &old));
}

#[test]
fn impossible_type_intersection_is_subset_of_anything() {
    let sub = resolve(json!({
        "allOf": [
            { "type": "string" },
            { "type": "number" }
        ]
    }));
    let sup = resolve(json!({ "const": 42 }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn disjoint_complement_xor_collapses_to_union_subset() {
    let old = resolve(json!({
        "anyOf": [
            { "type": "string" },
            { "type": "number" }
        ]
    }));
    let new = resolve(json!({
        "oneOf": [
            { "not": { "type": "string" } },
            { "not": { "type": "number" } }
        ]
    }));
    assert!(is_subschema_of(&new, &old));
}

#[test]
fn overlapping_complement_xor_does_not_collapse_to_union() {
    // Excluded regions overlap (integers are numbers), so the xor is the
    // non-integer-number gap rather than their union. Do not prove it as a
    // subset of integers.
    let old = resolve(json!({ "type": "integer" }));
    let new = resolve(json!({
        "oneOf": [
            { "not": { "type": "integer" } },
            { "not": { "type": "number" } }
        ]
    }));
    assert!(!is_subschema_of(&new, &old));
}

#[test]
fn three_way_complement_xor_needs_only_two_covering_inners() {
    // In a three-way xor of complements, each accepted arm satisfies the
    // other two excluded regions.  A and B are both contained by `old`, so
    // every arm is covered even though C also admits arrays.
    let old = resolve(json!({ "type": ["string", "number", "boolean"] }));
    let new = resolve(json!({
        "oneOf": [
            { "not": { "type": ["string", "number"] } },
            { "not": { "type": ["number", "boolean"] } },
            { "not": { "type": ["boolean", "array"] } }
        ]
    }));
    assert!(is_subschema_of(&new, &old));
}

#[test]
fn negated_singleton_untyped_numeric_assertion_forces_number() {
    let old = resolve(json!({ "type": "number" }));
    let new = resolve(json!({ "not": { "oneOf": [ { "maximum": 1 } ] } }));
    assert!(is_subschema_of(&new, &old));
}

#[test]
fn negated_singleton_untyped_string_assertion_forces_string() {
    let old = resolve(json!({ "type": "string" }));
    let new = resolve(json!({ "not": { "oneOf": [ { "maxLength": 1 } ] } }));
    assert!(is_subschema_of(&new, &old));
}

#[test]
fn disjoint_three_way_complement_xor_is_empty() {
    let old = resolve(json!({ "const": 42 }));
    let new = resolve(json!({
        "oneOf": [
            { "not": { "type": "string" } },
            { "not": { "type": "number" } },
            { "not": { "type": "boolean" } }
        ]
    }));
    assert!(is_subschema_of(&new, &old));
}

#[test]
fn three_way_complement_xor_with_one_covering_inner_stays_conservative() {
    let old = resolve(json!({ "type": "number" }));
    let new = resolve(json!({
        "oneOf": [
            { "not": { "type": "number" } },
            { "not": { "type": ["number", "array"] } },
            { "not": { "type": ["number", "string"] } }
        ]
    }));
    assert!(!is_subschema_of(&new, &old));
}

#[test]
fn overlapping_complement_xor_is_still_subset_of_excluded_union() {
    // Symmetric difference of integers and numbers is the non-integer
    // numbers, which is still contained by the broader number schema.
    let old = resolve(json!({ "type": "number" }));
    let new = resolve(json!({
        "oneOf": [
            { "not": { "type": "integer" } },
            { "not": { "type": "number" } }
        ]
    }));
    assert!(is_subschema_of(&new, &old));
}

#[test]
fn duplicate_two_arm_oneof_is_empty() {
    let old = resolve(json!(false));
    let new = resolve(json!({ "oneOf": [ { "type": "string" }, { "type": "string" } ] }));
    assert!(is_subschema_of(&new, &old));
}

#[test]
fn false_arm_two_way_oneof_normalizes_to_live_branch() {
    let old = resolve(json!({ "type": "string" }));
    let new = resolve(json!({ "oneOf": [ false, { "type": "string", "maxLength": 3 } ] }));
    assert!(is_subschema_of(&new, &old));

    let wrapped_old = resolve(json!({ "oneOf": [ { "type": "string" }, false ] }));
    assert!(is_subschema_of(&new, &wrapped_old));
}

#[test]
fn oneof_with_multiple_empty_arms_reduces_to_live_branch() {
    let old = resolve(json!({ "type": "integer" }));
    let new =
        resolve(json!({ "oneOf": [ false, { "enum": [] }, { "type": "integer", "minimum": 0 } ] }));
    assert!(is_subschema_of(&new, &old));
}

#[test]
fn negated_impossible_intersection_is_universal_superset() {
    let old = resolve(json!({ "not": { "allOf": [ { "type": "array" }, { "type": "string" } ] } }));
    let new = resolve(json!({ "anyOf": [ { "maxLength": 1 }, { "not": false } ] }));
    assert!(is_subschema_of(&new, &old));
}

#[test]
fn negated_oneof_complement_disjoint_pair_contains_positive_side() {
    let old = resolve(json!({
        "not": { "oneOf": [ { "not": { "type": "null" } }, { "type": "number" } ] }
    }));
    let new = resolve(json!({ "type": "number" }));
    assert!(is_subschema_of(&new, &old));

    let null_side = resolve(json!({ "type": "null" }));
    assert!(is_subschema_of(&null_side, &old));
}

#[test]
fn negated_two_arm_oneof_contains_intersection_side() {
    let old = resolve(json!({ "not": { "oneOf": [ { "minLength": 1 }, { "maximum": 1 } ] } }));
    let new = resolve(json!({ "oneOf": [ { "type": "object" }, { "enum": ["a", "aa"] } ] }));
    assert!(is_subschema_of(&new, &old));
}

#[test]
fn negated_two_arm_oneof_contains_neither_side() {
    let old =
        resolve(json!({ "not": { "oneOf": [ { "type": "string" }, { "type": "number" } ] } }));
    let new = resolve(json!({ "type": "object" }));
    assert!(is_subschema_of(&new, &old));
}

#[test]
fn negated_untyped_minimum_becomes_number_upper_halfline() {
    let old = resolve(json!({ "type": "number", "exclusiveMaximum": 0 }));
    let new = resolve(json!({ "not": { "minimum": 0 } }));
    assert!(is_subschema_of(&new, &old));
}

#[test]
fn negated_untyped_maximum_becomes_number_lower_halfline() {
    let old = resolve(json!({ "type": "number", "exclusiveMinimum": 1 }));
    let new = resolve(json!({ "not": { "maximum": 1 } }));
    assert!(is_subschema_of(&new, &old));
}

#[test]
fn negated_untyped_minlength_becomes_string_upper_count() {
    let old = resolve(json!({ "type": "string", "maxLength": 0 }));
    let new = resolve(json!({ "not": { "minLength": 1 } }));
    assert!(is_subschema_of(&new, &old));
}

#[test]
fn negated_untyped_maxlength_becomes_string_lower_count() {
    let old = resolve(json!({ "type": "string", "minLength": 2 }));
    let new = resolve(json!({ "not": { "maxLength": 1 } }));
    assert!(is_subschema_of(&new, &old));
}

#[test]
fn negated_untyped_minitems_becomes_array_upper_count() {
    let old = resolve(json!({ "type": "array", "maxItems": 1 }));
    let new = resolve(json!({ "not": { "minItems": 2 } }));
    assert!(is_subschema_of(&new, &old));
}

#[test]
fn negated_untyped_maxitems_becomes_array_lower_count() {
    let old = resolve(json!({ "type": "array", "minItems": 3 }));
    let new = resolve(json!({ "not": { "maxItems": 2 } }));
    assert!(is_subschema_of(&new, &old));
}

#[test]
fn negated_untyped_minproperties_becomes_object_upper_count() {
    let old = resolve(json!({ "type": "object", "maxProperties": 1 }));
    let new = resolve(json!({ "not": { "minProperties": 2 } }));
    assert!(is_subschema_of(&new, &old));
}

#[test]
fn negated_untyped_maxproperties_becomes_object_lower_count() {
    let old = resolve(json!({ "type": "object", "minProperties": 3 }));
    let new = resolve(json!({ "not": { "maxProperties": 2 } }));
    assert!(is_subschema_of(&new, &old));
}

#[test]
fn mixed_oneof_disjoint_complement_is_subset_of_negated_side() {
    let old = resolve(json!({ "not": { "type": "array" } }));
    let new = resolve(json!({
        "oneOf": [ { "type": "array" }, { "not": { "const": {} } } ]
    }));
    assert!(is_subschema_of(&new, &old));

    let old_other = resolve(json!({ "not": { "const": {} } }));
    assert!(is_subschema_of(&new, &old_other));
}

#[test]
fn mixed_oneof_disjoint_complement_fits_negated_union_arm() {
    let old = resolve(json!({
        "anyOf": [ { "type": "object" }, { "not": { "enum": [1, 2] } } ]
    }));
    let new = resolve(json!({
        "oneOf": [ { "type": "number" }, { "not": { "type": "object" } } ]
    }));
    assert!(is_subschema_of(&new, &old));
}

#[test]
fn mixed_oneof_disjoint_complement_fits_negated_intersection_target() {
    let old = resolve(json!({
        "allOf": [ { "not": { "const": 0 } }, { "not": { "const": 1 } } ]
    }));
    let new = resolve(json!({
        "oneOf": [ { "type": "integer" }, { "not": { "const": [1] } } ]
    }));
    assert!(is_subschema_of(&new, &old));
}

#[test]
fn mixed_oneof_disjoint_complement_fits_comparable_mixed_xor_target() {
    let old = resolve(json!({
        "oneOf": [ { "const": false }, { "not": { "type": "boolean" } } ]
    }));
    let new = resolve(json!({
        "oneOf": [ { "const": true }, { "not": { "const": {} } } ]
    }));
    assert!(is_subschema_of(&new, &old));
}

#[test]
fn mixed_oneof_disjoint_complement_fits_union_with_finite_negated_remainder() {
    let old = resolve(json!({
        "anyOf": [ { "const": 1 }, { "not": { "enum": [1, 2] } } ]
    }));
    let new = resolve(json!({
        "oneOf": [ { "const": 2 }, { "not": { "type": "string" } } ]
    }));
    assert!(is_subschema_of(&new, &old));
}

#[test]
fn comparable_positive_oneof_difference_fits_negated_union() {
    let old = resolve(json!({
        "not": { "anyOf": [ { "const": false }, { "const": "b" } ] }
    }));
    let new = resolve(json!({
        "oneOf": [ { "const": "b" }, { "type": "string" } ]
    }));
    assert!(is_subschema_of(&new, &old));
}

#[test]
fn negated_double_complement_xor_fits_mixed_finite_difference() {
    let old = resolve(json!({
        "oneOf": [ { "enum": [1, "a", null] }, { "not": { "const": 1 } } ]
    }));
    let new = resolve(json!({
        "not": { "oneOf": [ { "not": { "type": "null" } }, { "not": { "type": "string" } } ] }
    }));
    assert!(is_subschema_of(&new, &old));
}

#[test]
fn negated_disjoint_xor_fits_mixed_finite_difference() {
    let old = resolve(json!({
        "oneOf": [ { "enum": [[], {}] }, { "not": { "const": {} } } ]
    }));
    let new = resolve(json!({
        "not": { "oneOf": [ { "const": 3 }, { "type": "array" } ] }
    }));
    assert!(is_subschema_of(&new, &old));
}

#[test]
fn negated_disjoint_xor_fits_mixed_opposite_finite_difference() {
    let old = resolve(json!({
        "oneOf": [ { "const": 1 }, { "not": { "enum": [1, 2] } } ]
    }));
    let new = resolve(json!({
        "not": { "oneOf": [ { "const": [2] }, { "enum": [2, 3] } ] }
    }));
    assert!(is_subschema_of(&new, &old));
}

#[test]
fn negated_comparable_xor_excludes_mixed_finite_gap() {
    let old = resolve(json!({
        "oneOf": [ { "enum": [[], {}] }, { "not": { "const": {} } } ]
    }));
    let new = resolve(json!({
        "not": { "oneOf": [ { "const": 0 }, { "not": { "type": "object" } } ] }
    }));
    assert!(is_subschema_of(&new, &old));
}

#[test]
fn negated_xor_covers_infinite_mixed_gap_with_finite_overlap() {
    let old = resolve(json!({
        "oneOf": [ { "type": "object" }, { "not": { "const": {} } } ]
    }));
    let new = resolve(json!({
        "not": { "oneOf": [ { "enum": [[], {}] }, { "type": "object" } ] }
    }));
    assert!(is_subschema_of(&new, &old));
}

#[test]
fn constant_true_conditional_reduces_to_then_branch() {
    let sup = resolve(json!({
        "if": true,
        "then": { "allOf": [ { "type": "number" }, { "multipleOf": 1 } ] },
        "else": false
    }));
    let sub = resolve(json!({ "enum": [1, 3] }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn constant_false_conditional_reduces_to_else_branch() {
    let sup = resolve(json!({
        "if": false,
        "then": false,
        "else": { "type": "string", "enum": ["a", "b"] }
    }));
    let sub = resolve(json!({ "const": "a" }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn array_schema_fits_negated_numeric_union_by_type_mask() {
    let sup = resolve(json!({
        "not": { "anyOf": [ { "type": "number", "minimum": 0, "maximum": 2 } ] }
    }));
    let sub = resolve(json!({
        "type": "array",
        "prefixItems": [true],
        "items": { "type": "array" },
        "maxItems": 1
    }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn non_arrays_fit_negated_constant_true_conditional_array_branch() {
    let sup = resolve(json!({
        "not": {
            "if": true,
            "then": { "type": "array" },
            "else": { "const": 0 }
        }
    }));
    let sub = resolve(json!({
        "anyOf": [
            { "type": "integer" },
            { "type": "string" },
            { "type": "boolean" }
        ]
    }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn bounded_number_lattice_singleton_fits_integer_conditional_else() {
    let sup = resolve(json!({
        "if": { "type": "array", "minItems": 2 },
        "then": { "type": "object" },
        "else": { "type": "integer", "multipleOf": 2 }
    }));
    let sub = resolve(json!({
        "type": "number",
        "minimum": 0,
        "maximum": 0,
        "multipleOf": 2
    }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn nested_singleton_oneof_fits_negated_empty_enum_intersection() {
    let sub = resolve(json!({ "oneOf": [{ "oneOf": [true] }] }));
    let sup = resolve(json!({
        "not": {
            "allOf": [
                { "type": "string", "minLength": 2, "maxLength": 2, "enum": ["b"] }
            ]
        }
    }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn impossible_contains_requirement_makes_array_branch_empty() {
    let sub = resolve(json!({
        "type": "array",
        "contains": false,
        "minContains": 1
    }));
    let sup = resolve(json!({ "type": "string" }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn disjoint_contains_matcher_makes_array_branch_empty_across_types() {
    let sup = resolve(json!({ "type": "object" }));
    let homogeneous = resolve(json!({
        "type": "array",
        "items": { "type": "string" },
        "contains": { "type": "number" },
        "minContains": 1
    }));
    assert!(is_subschema_of(&homogeneous, &sup));

    let closed_tuple = resolve(json!({
        "type": "array",
        "prefixItems": [{ "type": "string" }],
        "items": false,
        "minItems": 1,
        "contains": { "type": "number" },
        "minContains": 1
    }));
    assert!(is_subschema_of(&closed_tuple, &sup));
}

#[test]
fn locally_impossible_unique_array_branch_is_vacuous_across_types() {
    let sup = resolve(json!({ "type": "string" }));
    let finite_tail = resolve(json!({
        "type": "array",
        "uniqueItems": true,
        "minItems": 2,
        "items": { "enum": [1] }
    }));
    assert!(is_subschema_of(&finite_tail, &sup));

    let repeated_prefix = resolve(json!({
        "type": "array",
        "uniqueItems": true,
        "prefixItems": [{ "const": 1 }, { "const": 1 }],
        "minItems": 2
    }));
    assert!(is_subschema_of(&repeated_prefix, &sup));
}

#[test]
fn negated_split_allof_object_property_contradiction_is_universal() {
    let sup = resolve(json!({
        "not": {
            "allOf": [
                { "type": "object", "required": ["a"], "properties": { "a": { "type": "string" } } },
                { "type": "object", "required": ["a"], "properties": { "a": { "type": "number" } } }
            ]
        }
    }));
    let sub = resolve(json!(true));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn negated_split_allof_array_items_contains_contradiction_is_universal() {
    let sup = resolve(json!({
        "not": {
            "allOf": [
                { "type": "array", "items": { "type": "string" } },
                { "type": "array", "contains": { "type": "number" }, "minContains": 1 }
            ]
        }
    }));
    let sub = resolve(json!(true));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn negated_split_allof_array_length_contradiction_is_universal() {
    let sup = resolve(json!({
        "not": { "allOf": [
            { "type": "array", "minItems": 3 },
            { "type": "array", "maxItems": 1 }
        ] }
    }));
    assert!(is_subschema_of(&resolve(json!(true)), &sup));
}

#[test]
fn negated_split_allof_object_count_contradiction_is_universal() {
    let sup = resolve(json!({
        "not": { "allOf": [
            { "type": "object", "minProperties": 3 },
            { "type": "object", "maxProperties": 1 }
        ] }
    }));
    assert!(is_subschema_of(&resolve(json!(true)), &sup));
}

#[test]
fn negated_split_allof_numeric_range_contradiction_is_universal() {
    let sup = resolve(json!({
        "not": {
            "allOf": [
                { "type": "number", "minimum": 3 },
                { "type": "number", "maximum": 1 }
            ]
        }
    }));
    let sub = resolve(json!(true));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn negated_split_allof_string_length_contradiction_is_universal() {
    let sup = resolve(json!({
        "not": {
            "allOf": [
                { "type": "string", "minLength": 3 },
                { "type": "string", "maxLength": 1 }
            ]
        }
    }));
    let sub = resolve(json!(true));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn negated_duplicate_oneof_branches_are_universal() {
    let sup = resolve(json!({
        "not": { "oneOf": [ { "type": "string" }, { "type": "string" } ] }
    }));
    let sub = resolve(json!(true));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn negated_conditional_with_locally_impossible_branches_is_universal() {
    let sup = resolve(json!({
        "not": {
            "if": { "type": "string" },
            "then": {
                "type": "array",
                "uniqueItems": true,
                "minItems": 3,
                "items": { "type": "boolean" }
            },
            "else": {
                "type": "object",
                "properties": { "a": false },
                "required": ["a"]
            }
        }
    }));
    let sub = resolve(json!(true));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn negated_locally_impossible_unique_array_is_universal() {
    let sup = resolve(json!({
        "not": {
            "type": "array",
            "uniqueItems": true,
            "minItems": 3,
            "items": { "type": "boolean" }
        }
    }));
    let sub = resolve(json!(true));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn locally_impossible_object_branch_is_vacuous_across_types() {
    let sub = resolve(json!({
        "type": "object",
        "properties": { "a": false },
        "required": ["a"]
    }));
    let sup = resolve(json!({ "type": "array" }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn locally_empty_contains_matcher_makes_array_branch_empty() {
    let sub = resolve(json!({
        "type": "array",
        "contains": { "type": "string", "enum": [1] },
        "minContains": 1
    }));
    let sup = resolve(json!({ "type": "number" }));
    assert!(is_subschema_of(&sub, &sup));

    let constrained = resolve(json!({
        "type": "array",
        "contains": { "type": "string", "minLength": 2, "maxLength": 2, "enum": ["b"] },
        "minContains": 1
    }));
    assert!(is_subschema_of(&constrained, &sup));
}

#[test]
fn forbidden_optional_property_skips_superset_property_constraint() {
    let sub = resolve(json!({
        "type": "object",
        "propertyNames": { "enum": ["b"] }
    }));
    let sup = resolve(json!({
        "type": "object",
        "properties": { "k": { "type": "number" } }
    }));
    assert!(is_subschema_of(&sub, &sup));
}

#[test]
fn oneof_object_presence_partition_allows_optional_property_target() {
    let sub = resolve(json!({
        "oneOf": [
            { "type": "object" },
            { "type": "object", "required": ["a"] }
        ]
    }));
    let sup = resolve(json!({
        "type": "object",
        "properties": { "a": { "type": "number" } }
    }));
    assert!(is_subschema_of(&sub, &sup));

    let conditional = resolve(json!({
        "if": { "type": "string", "minLength": 1 },
        "then": { "type": "integer" },
        "else": {
            "type": "object",
            "properties": { "a": { "type": "number" } }
        }
    }));
    assert!(is_subschema_of(&sub, &conditional));
}

#[test]
fn oneof_object_presence_partition_does_not_ignore_other_requirements() {
    let sub = resolve(json!({
        "oneOf": [
            { "type": "object" },
            { "type": "object", "required": ["a"] }
        ]
    }));
    let sup = resolve(json!({
        "type": "object",
        "required": ["b"]
    }));
    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn oneof_array_nonempty_partition_is_empty_array() {
    let sub = resolve(json!({
        "oneOf": [
            { "type": "array" },
            { "type": "array", "minItems": 1 }
        ]
    }));
    let sup = resolve(json!({ "type": "array", "maxItems": 1 }));
    assert!(is_subschema_of(&sub, &sup));

    let conditional = resolve(json!({
        "if": { "type": "object" },
        "then": false,
        "else": { "type": "array", "maxItems": 0 }
    }));
    assert!(is_subschema_of(&sub, &conditional));
}

#[test]
fn oneof_array_nonempty_partition_respects_contains_minimum() {
    let sub = resolve(json!({
        "oneOf": [
            { "type": "array" },
            { "type": "array", "minItems": 1 }
        ]
    }));
    let sup = resolve(json!({
        "type": "array",
        "contains": true,
        "minContains": 1
    }));
    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn oneof_string_nonempty_partition_is_empty_string() {
    let sub = resolve(json!({
        "oneOf": [
            { "type": "string" },
            { "type": "string", "minLength": 1 }
        ]
    }));
    let sup = resolve(json!({ "type": "string", "maxLength": 0 }));
    assert!(is_subschema_of(&sub, &sup));

    let conditional = resolve(json!({
        "if": { "type": "array" },
        "then": false,
        "else": { "type": "string", "maxLength": 0 }
    }));
    assert!(is_subschema_of(&sub, &conditional));
}

#[test]
fn oneof_string_nonempty_partition_does_not_assume_patterns() {
    let sub = resolve(json!({
        "oneOf": [
            { "type": "string" },
            { "type": "string", "minLength": 1 }
        ]
    }));
    let sup = resolve(json!({ "type": "string", "pattern": "^x" }));
    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn oneof_object_nonempty_partition_is_empty_object() {
    let sub = resolve(json!({
        "oneOf": [
            { "type": "object" },
            { "type": "object", "minProperties": 1 }
        ]
    }));
    let sup = resolve(json!({
        "type": "object",
        "maxProperties": 0,
        "properties": { "a": false }
    }));
    assert!(is_subschema_of(&sub, &sup));

    let conditional = resolve(json!({
        "if": { "type": "array" },
        "then": false,
        "else": { "type": "object", "maxProperties": 0 }
    }));
    assert!(is_subschema_of(&sub, &conditional));
}

#[test]
fn oneof_object_nonempty_partition_respects_required_target() {
    let sub = resolve(json!({
        "oneOf": [
            { "type": "object" },
            { "type": "object", "minProperties": 1 }
        ]
    }));
    let sup = resolve(json!({ "type": "object", "required": ["a"] }));
    assert!(!is_subschema_of(&sub, &sup));
}

#[test]
fn oneof_exact_partition_shortcuts_preserve_extra_union_types() {
    let cases = [
        (
            json!({
                "oneOf": [
                    { "type": "string" },
                    {
                        "anyOf": [
                            { "type": "string", "minLength": 1 },
                            { "type": "number" }
                        ]
                    }
                ]
            }),
            json!({ "type": "string", "maxLength": 0 }),
        ),
        (
            json!({
                "oneOf": [
                    { "type": "array" },
                    {
                        "anyOf": [
                            { "type": "array", "minItems": 1 },
                            { "type": "string" }
                        ]
                    }
                ]
            }),
            json!({ "type": "array", "maxItems": 0 }),
        ),
        (
            json!({
                "oneOf": [
                    { "type": "object" },
                    {
                        "anyOf": [
                            { "type": "object", "minProperties": 1 },
                            { "type": "string" }
                        ]
                    }
                ]
            }),
            json!({ "type": "object", "maxProperties": 0 }),
        ),
        (
            json!({
                "oneOf": [
                    { "type": "object" },
                    {
                        "anyOf": [
                            { "type": "object", "required": ["p"] },
                            { "type": "string" }
                        ]
                    }
                ]
            }),
            json!({ "type": "object", "properties": { "p": false } }),
        ),
    ];

    for (sub, sup) in cases {
        assert!(!is_subschema_of(&resolve(sub), &resolve(sup)));
    }
}
