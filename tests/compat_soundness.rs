use fancy_regex::Regex;
use jsoncompat::{Role, SchemaDocument, check_compat};
use serde_json::{Value, json};

#[test]
fn claimed_compatibility_survives_small_raw_validator_witness_space() {
    let schemas = [
        ("any", json!(true)),
        ("string", json!({ "type": "string" })),
        ("string_x", json!({ "type": "string", "pattern": "^x$" })),
        (
            "short_string",
            json!({ "type": "string", "minLength": 1, "maxLength": 2 }),
        ),
        (
            "string_enum",
            json!({ "type": "string", "enum": ["x", "xy"] }),
        ),
        (
            "unsupported_control_c_enum",
            json!({
                "type": "string",
                "pattern": "^\\cC$",
                "enum": ["\u{3}"]
            }),
        ),
        (
            "object_enum_with_nested_unsupported_control_c_pattern",
            json!({
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
            }),
        ),
        (
            "object_enum_with_recursive_allof_string_value",
            json!({
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
            }),
        ),
        (
            "array_enum_with_nested_unsupported_control_c_pattern",
            json!({
                "type": "array",
                "prefixItems": [{
                    "type": "string",
                    "pattern": "^\\cC$"
                }],
                "items": false,
                "enum": [["\u{3}"]]
            }),
        ),
        (
            "array_enum_with_unsupported_control_c_contains",
            json!({
                "type": "array",
                "contains": {
                    "type": "string",
                    "pattern": "^\\cC$"
                },
                "minContains": 1,
                "enum": [["\u{3}"]]
            }),
        ),
        (
            "not_unsupported_control_c_pattern",
            json!({
                "not": {
                    "type": "string",
                    "pattern": "^\\cC$"
                }
            }),
        ),
        (
            "one_of_unsupported_control_c_pattern_or_const",
            json!({
                "oneOf": [
                    {
                        "type": "string",
                        "pattern": "^\\cC$"
                    },
                    { "const": "\u{3}" }
                ]
            }),
        ),
        (
            "conditional_unsupported_control_c_pattern",
            json!({
                "if": {
                    "type": "string",
                    "pattern": "^\\cC$"
                },
                "then": false,
                "else": true
            }),
        ),
        (
            "array_max_contains_unsupported_control_c_pattern",
            json!({
                "type": "array",
                "contains": {
                    "type": "string",
                    "pattern": "^\\cC$"
                },
                "maxContains": 0
            }),
        ),
        (
            "email_format_string",
            json!({ "type": "string", "format": "email" }),
        ),
        (
            "uuid_format_string",
            json!({ "type": "string", "format": "uuid" }),
        ),
        ("string_const", json!({ "const": "x" })),
        ("boolean_true", json!({ "const": true })),
        ("null_only", json!({ "type": "null" })),
        ("integer", json!({ "type": "integer" })),
        (
            "non_negative_integer",
            json!({ "type": "integer", "minimum": 0 }),
        ),
        (
            "small_integer_enum",
            json!({ "type": "integer", "enum": [-1, 0, 1] }),
        ),
        (
            "small_even_integer",
            json!({
                "type": "integer",
                "minimum": 0,
                "maximum": 4,
                "multipleOf": 2
            }),
        ),
        (
            "small_number",
            json!({ "type": "number", "minimum": -1, "maximum": 2 }),
        ),
        (
            "small_number_enum",
            json!({ "type": "number", "enum": [0, 0.5, 2] }),
        ),
        (
            "number_allof_single_conjunct_subset",
            json!({
                "allOf": [
                    { "type": "number", "minimum": 1, "maximum": 5 },
                    { "type": "number", "maximum": 10 }
                ]
            }),
        ),
        (
            "integer_array",
            json!({ "type": "array", "items": { "type": "integer" } }),
        ),
        (
            "bounded_string_array",
            json!({
                "type": "array",
                "items": { "type": "string" },
                "minItems": 1,
                "maxItems": 2
            }),
        ),
        (
            "single_unique_item_array",
            json!({
                "type": "array",
                "items": true,
                "maxItems": 1,
                "uniqueItems": true
            }),
        ),
        (
            "array_with_integer_contains",
            json!({
                "type": "array",
                "items": true,
                "contains": { "type": "integer" },
                "minContains": 1
            }),
        ),
        (
            "array_without_integer_contains",
            json!({
                "type": "array",
                "items": true,
                "contains": { "type": "integer" },
                "maxContains": 0
            }),
        ),
        (
            "array_with_one_string_contains",
            json!({
                "type": "array",
                "items": true,
                "contains": { "type": "string" },
                "minContains": 1,
                "maxContains": 1
            }),
        ),
        (
            "array_with_guaranteed_integer_prefix_contains",
            json!({
                "type": "array",
                "prefixItems": [
                    { "type": "integer" },
                    { "type": "string" }
                ],
                "items": false,
                "minItems": 1
            }),
        ),
        (
            "single_integer_tuple",
            json!({
                "type": "array",
                "prefixItems": [{ "type": "integer" }],
                "items": false
            }),
        ),
        (
            "integer_string_tuple",
            json!({
                "type": "array",
                "prefixItems": [{ "type": "integer" }, { "type": "string" }],
                "items": false
            }),
        ),
        (
            "unique_array",
            json!({ "type": "array", "uniqueItems": true }),
        ),
        (
            "closed_x_string_object",
            json!({
                "type": "object",
                "properties": { "x": { "type": "string" } },
                "required": ["x"],
                "additionalProperties": false
            }),
        ),
        (
            "optional_x_string_object",
            json!({
                "type": "object",
                "properties": { "x": { "type": "string" } },
                "additionalProperties": false
            }),
        ),
        (
            "one_or_two_property_object",
            json!({
                "type": "object",
                "properties": {
                    "x": { "type": "string" },
                    "y": { "type": "integer" }
                },
                "minProperties": 1,
                "maxProperties": 2,
                "additionalProperties": false
            }),
        ),
        (
            "x_integer_pattern_object",
            json!({
                "type": "object",
                "patternProperties": { "^x$": { "type": "integer" } },
                "additionalProperties": false
            }),
        ),
        (
            "x_string_property_and_matching_pattern",
            json!({
                "type": "object",
                "properties": { "x": { "type": "string" } },
                "patternProperties": { "^x$": { "type": "string" } },
                "additionalProperties": false
            }),
        ),
        (
            "x_property_pattern_conflict",
            json!({
                "type": "object",
                "properties": { "x": { "type": "string" } },
                "patternProperties": { "^x$": { "type": "integer" } },
                "additionalProperties": false
            }),
        ),
        (
            "x_pattern_integer_string_additional",
            json!({
                "type": "object",
                "patternProperties": { "^x$": { "type": "integer" } },
                "additionalProperties": { "type": "string" }
            }),
        ),
        (
            "x_prefix_string_closed",
            json!({
                "type": "object",
                "patternProperties": {
                    "^x": { "type": "string" }
                },
                "additionalProperties": false
            }),
        ),
        (
            "x_prefix_string_and_suffix_integer_closed",
            json!({
                "type": "object",
                "patternProperties": {
                    "^x": { "type": "string" },
                    "x$": { "type": "integer" }
                },
                "additionalProperties": false
            }),
        ),
        (
            "named_x_or_y_object",
            json!({
                "type": "object",
                "propertyNames": { "enum": ["x", "y"] }
            }),
        ),
        (
            "dependent_x_requires_y",
            json!({
                "type": "object",
                "dependentRequired": { "x": ["y"] }
            }),
        ),
        (
            "dependent_chain_x_requires_z",
            json!({
                "type": "object",
                "dependentRequired": {
                    "x": ["y"],
                    "y": ["z"]
                }
            }),
        ),
        (
            "x_property_forbidden",
            json!({
                "type": "object",
                "properties": { "x": false }
            }),
        ),
        (
            "x_pattern_forbidden",
            json!({
                "type": "object",
                "patternProperties": { "^x$": false }
            }),
        ),
        (
            "unsupported_x_pattern_property",
            json!({
                "type": "object",
                "patternProperties": { "^(?=x$)x$": true },
                "additionalProperties": false
            }),
        ),
        (
            "unsupported_x_property_name",
            json!({
                "type": "object",
                "propertyNames": { "pattern": "^(?=x$)x$" }
            }),
        ),
        (
            "recursive_string_property_names",
            json!({
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
            }),
        ),
        (
            "string_or_integer",
            json!({
                "anyOf": [
                    { "type": "string" },
                    { "type": "integer" }
                ]
            }),
        ),
        (
            "string_allof_min_length",
            json!({
                "allOf": [
                    { "type": "string" },
                    { "minLength": 1 }
                ]
            }),
        ),
        (
            "string_or_null_one_of",
            json!({
                "oneOf": [
                    { "type": "string" },
                    { "type": "null" }
                ]
            }),
        ),
        (
            "overlapping_string_one_of",
            json!({
                "oneOf": [
                    { "type": "string" },
                    { "enum": ["x"] }
                ]
            }),
        ),
        (
            "string_or_x_any_of",
            json!({
                "anyOf": [
                    { "type": "string" },
                    { "enum": ["x"] }
                ]
            }),
        ),
        (
            "integer_or_one_one_of",
            json!({
                "oneOf": [
                    { "type": "integer" },
                    { "const": 1 }
                ]
            }),
        ),
        (
            "integer_or_one_any_of",
            json!({
                "anyOf": [
                    { "type": "integer" },
                    { "const": 1 }
                ]
            }),
        ),
        (
            "non_negative_integer_or_string_conditional",
            json!({
                "if": { "type": "integer" },
                "then": { "minimum": 0 },
                "else": { "type": "string" }
            }),
        ),
        (
            "object_tag_conditional",
            json!({
                "type": "object",
                "properties": {
                    "kind": { "enum": ["full", "thin"] },
                    "detail": { "type": "string" }
                },
                "if": {
                    "properties": {
                        "kind": { "const": "full" }
                    },
                    "required": ["kind"]
                },
                "then": { "required": ["detail"] },
                "additionalProperties": false
            }),
        ),
        ("not_false", json!({ "not": false })),
        ("not_true", json!({ "not": true })),
        (
            "recursive_node",
            json!({
                "$defs": {
                    "node": {
                        "type": "object",
                        "properties": {
                            "next": { "$ref": "#/$defs/node" }
                        },
                        "additionalProperties": false
                    }
                },
                "$ref": "#/$defs/node"
            }),
        ),
        (
            "recursive_node_with_value",
            json!({
                "$defs": {
                    "node": {
                        "type": "object",
                        "properties": {
                            "value": { "type": "string" },
                            "next": { "$ref": "#/$defs/node" }
                        },
                        "required": ["value"],
                        "additionalProperties": false
                    }
                },
                "$ref": "#/$defs/node"
            }),
        ),
        (
            "recursive_string_list",
            json!({
                "$defs": {
                    "node": {
                        "type": "object",
                        "properties": {
                            "value": { "type": "string" },
                            "next": { "$ref": "#/$defs/node" }
                        },
                        "required": ["value"],
                        "additionalProperties": false
                    }
                },
                "$ref": "#/$defs/node"
            }),
        ),
        (
            "recursive_integer_list",
            json!({
                "$defs": {
                    "node": {
                        "type": "object",
                        "properties": {
                            "value": { "type": "integer" },
                            "next": { "$ref": "#/$defs/node" }
                        },
                        "required": ["value"],
                        "additionalProperties": false
                    }
                },
                "$ref": "#/$defs/node"
            }),
        ),
        (
            "recursive_optional_string_list",
            json!({
                "$defs": {
                    "node": {
                        "type": "object",
                        "properties": {
                            "value": { "type": "string" },
                            "next": { "$ref": "#/$defs/node" }
                        },
                        "additionalProperties": false
                    }
                },
                "$ref": "#/$defs/node"
            }),
        ),
        (
            "recursive_string_or_integer_leaf",
            json!({
                "$defs": {
                    "node": {
                        "anyOf": [
                            { "type": "string" },
                            {
                                "type": "object",
                                "properties": {
                                    "next": { "$ref": "#/$defs/node" }
                                },
                                "required": ["next"],
                                "additionalProperties": false
                            }
                        ]
                    }
                },
                "$ref": "#/$defs/node"
            }),
        ),
        (
            "recursive_string_or_boolean_leaf",
            json!({
                "$defs": {
                    "node": {
                        "anyOf": [
                            { "type": "boolean" },
                            {
                                "type": "object",
                                "properties": {
                                    "next": { "$ref": "#/$defs/node" }
                                },
                                "required": ["next"],
                                "additionalProperties": false
                            }
                        ]
                    }
                },
                "$ref": "#/$defs/node"
            }),
        ),
    ];
    let schemas = schemas
        .into_iter()
        .map(|(name, raw)| {
            (
                name,
                SchemaDocument::from_json(&raw).expect("soundness schema should build"),
            )
        })
        .collect::<Vec<_>>();

    let witnesses = [
        json!(null),
        json!(false),
        json!(0),
        json!(1),
        json!(-1),
        json!(0.5),
        json!(2.0),
        json!(""),
        json!("\u{3}"),
        json!("x"),
        json!("xy"),
        json!("y"),
        json!("xyz"),
        json!([]),
        json!(["\u{3}"]),
        json!([0]),
        json!(["x"]),
        json!(["x", "xy"]),
        json!(["x", "y"]),
        json!([0, 1]),
        json!([0, 0]),
        json!([0, "x"]),
        json!([0, "x", true]),
        json!(["x", 0]),
        json!(["x", "x"]),
        json!({}),
        json!({ "x": null }),
        json!({ "x": 1 }),
        json!({ "x": true }),
        json!({ "x": "value" }),
        json!({ "value": "\u{3}" }),
        json!({ "value": "leaf" }),
        json!({ "x": "value", "y": 1 }),
        json!({ "x": 1, "y": "ok" }),
        json!({ "x": 1, "y": "ok", "z": true }),
        json!({ "x": "ok", "y": true }),
        json!({ "x": "ok", "z": true }),
        json!({ "z": "value" }),
        json!({ "y": "ok" }),
        json!({ "z": true }),
        json!({ "kind": "full" }),
        json!({ "kind": "full", "detail": "ready" }),
        json!({ "kind": "thin" }),
        json!({ "next": {} }),
        json!({ "value": "x" }),
        json!({ "value": "x", "next": { "value": "y" } }),
        json!({ "next": { "value": "y" } }),
        json!({ "value": 1 }),
        json!({ "value": 1, "next": { "value": 2 } }),
        json!({ "value": "x", "next": { "value": 2 } }),
        json!({ "next": "leaf" }),
        json!({ "next": true }),
        json!({ "next": { "next": "leaf" } }),
    ];

    for (old_name, old_schema) in &schemas {
        for (new_name, new_schema) in &schemas {
            for (role, source_name, source, target_name, target) in [
                (Role::Serializer, new_name, new_schema, old_name, old_schema),
                (
                    Role::Deserializer,
                    old_name,
                    old_schema,
                    new_name,
                    new_schema,
                ),
            ] {
                if !check_compat(old_schema, new_schema, role)
                    .expect("soundness corpus stays within the supported checker surface")
                {
                    continue;
                }

                assert_witnesses_preserve_inclusion(
                    role,
                    source_name,
                    source,
                    target_name,
                    target,
                    &witnesses,
                );
            }
        }
    }
}

#[test]
fn claimed_array_contains_compatibility_survives_cardinality_witnesses() {
    let schemas = [
        ("array_any", json!({ "type": "array" })),
        (
            "array_string_items_min_one",
            json!({
                "type": "array",
                "items": { "type": "string" },
                "minItems": 1
            }),
        ),
        (
            "array_integer_items_min_two",
            json!({
                "type": "array",
                "items": { "type": "integer" },
                "minItems": 2
            }),
        ),
        (
            "array_contains_string_once",
            json!({
                "type": "array",
                "contains": { "type": "string" },
                "minContains": 1
            }),
        ),
        (
            "array_contains_string_twice",
            json!({
                "type": "array",
                "contains": { "type": "string" },
                "minContains": 2
            }),
        ),
        (
            "array_contains_string_at_most_once",
            json!({
                "type": "array",
                "contains": { "type": "string" },
                "maxContains": 1
            }),
        ),
        (
            "array_contains_integer_never",
            json!({
                "type": "array",
                "contains": { "type": "integer" },
                "maxContains": 0
            }),
        ),
        (
            "tuple_string_integer",
            json!({
                "type": "array",
                "prefixItems": [
                    { "type": "string" },
                    { "type": "integer" }
                ],
                "items": false
            }),
        ),
        (
            "tuple_integer_string_min_one",
            json!({
                "type": "array",
                "prefixItems": [
                    { "type": "integer" },
                    { "type": "string" }
                ],
                "items": false,
                "minItems": 1
            }),
        ),
        (
            "tuple_string_then_any_tail",
            json!({
                "type": "array",
                "prefixItems": [{ "type": "string" }],
                "items": true
            }),
        ),
        (
            "tuple_string_then_any_tail_with_one_string_cap",
            json!({
                "type": "array",
                "prefixItems": [{ "type": "string" }],
                "items": true,
                "contains": { "type": "string" },
                "maxContains": 1
            }),
        ),
    ]
    .into_iter()
    .map(|(name, raw)| {
        (
            name,
            SchemaDocument::from_json(&raw).expect("array soundness schema should build"),
        )
    })
    .collect::<Vec<_>>();

    let witnesses = [
        json!([]),
        json!(["x"]),
        json!(["x", "y"]),
        json!(["x", 1]),
        json!([1]),
        json!([1, 2]),
        json!([1, "x"]),
        json!([true]),
        json!([true, "x"]),
        json!(["x", "x"]),
        json!(["x", 1, "y"]),
    ];

    for (old_name, old_schema) in &schemas {
        for (new_name, new_schema) in &schemas {
            for (role, source_name, source, target_name, target) in [
                (Role::Serializer, new_name, new_schema, old_name, old_schema),
                (
                    Role::Deserializer,
                    old_name,
                    old_schema,
                    new_name,
                    new_schema,
                ),
            ] {
                if !check_compat(old_schema, new_schema, role)
                    .expect("array soundness corpus stays within supported checker behavior")
                {
                    continue;
                }

                assert_witnesses_preserve_inclusion(
                    role,
                    source_name,
                    source,
                    target_name,
                    target,
                    &witnesses,
                );
            }
        }
    }
}

#[test]
fn claimed_array_item_and_contains_compatibility_survives_dense_small_witness_space() {
    let prefix_shapes = [
        ("no_prefix", None),
        ("integer_prefix", Some(json!([{ "type": "integer" }]))),
        ("string_prefix", Some(json!([{ "type": "string" }]))),
    ];
    let item_shapes = [
        ("any_items", json!(true)),
        ("integer_items", json!({ "type": "integer" })),
        ("string_items", json!({ "type": "string" })),
        ("closed_items", json!(false)),
    ];
    let contains_shapes = [
        ("no_contains", None),
        (
            "contains_integer_once",
            Some(json!({
                "contains": { "type": "integer" },
                "minContains": 1
            })),
        ),
        (
            "contains_integer_never",
            Some(json!({
                "contains": { "type": "integer" },
                "maxContains": 0
            })),
        ),
        (
            "contains_string_once",
            Some(json!({
                "contains": { "type": "string" },
                "minContains": 1
            })),
        ),
        (
            "contains_string_at_most_once",
            Some(json!({
                "contains": { "type": "string" },
                "maxContains": 1
            })),
        ),
    ];

    let mut schemas = Vec::new();
    for (prefix_name, prefix_items) in &prefix_shapes {
        for (items_name, items) in &item_shapes {
            for (contains_name, contains) in &contains_shapes {
                for unique_items in [false, true] {
                    let mut raw = json!({
                        "type": "array",
                        "items": items,
                        "uniqueItems": unique_items
                    });
                    if let Some(prefix_items) = prefix_items {
                        raw.as_object_mut()
                            .expect("array soundness schema should stay an object")
                            .insert("prefixItems".to_owned(), prefix_items.clone());
                    }
                    if let Some(contains) = contains {
                        let object = raw
                            .as_object_mut()
                            .expect("array soundness schema should stay an object");
                        object.extend(
                            contains
                                .as_object()
                                .expect("contains fragments should be objects")
                                .clone(),
                        );
                    }
                    let name = format!(
                        "array_{prefix_name}_{items_name}_{contains_name}_unique_{unique_items}"
                    );
                    schemas.push((
                        name,
                        SchemaDocument::from_json(&raw)
                            .expect("dense array soundness schema should build"),
                    ));
                }
            }
        }
    }

    let witnesses = [
        json!([]),
        json!([0]),
        json!([1]),
        json!(["x"]),
        json!([0, 1]),
        json!([0, "x"]),
        json!(["x", 0]),
        json!(["x", "y"]),
        json!([0, 0]),
        json!(["x", "x"]),
        json!([0, "x", 1]),
        json!(["x", 0, "y"]),
    ];

    for (old_name, old_schema) in &schemas {
        for (new_name, new_schema) in &schemas {
            for (role, source_name, source, target_name, target) in [
                (Role::Serializer, new_name, new_schema, old_name, old_schema),
                (
                    Role::Deserializer,
                    old_name,
                    old_schema,
                    new_name,
                    new_schema,
                ),
            ] {
                if !check_compat(old_schema, new_schema, role)
                    .expect("dense array soundness corpus stays within supported checker behavior")
                {
                    continue;
                }

                assert_witnesses_preserve_inclusion(
                    role,
                    source_name,
                    source,
                    target_name,
                    target,
                    &witnesses,
                );
            }
        }
    }
}

#[test]
fn claimed_array_cardinality_compatibility_survives_dense_small_witness_space() {
    let prefix_shapes = [
        ("no_prefix", None),
        ("one_prefix", Some(json!([true]))),
        ("two_prefixes", Some(json!([true, true]))),
    ];
    let item_shapes = [
        ("open_tail", Value::Bool(true)),
        ("closed_tail", Value::Bool(false)),
    ];
    let minimums = [
        ("no_min", None),
        ("min_one", Some(json!(1))),
        ("min_two", Some(json!(2))),
    ];
    let maximums = [
        ("no_max", None),
        ("max_one", Some(json!(1))),
        ("max_two", Some(json!(2))),
    ];

    let mut schemas = Vec::new();
    for (prefix_name, prefix_items) in &prefix_shapes {
        for (items_name, items) in &item_shapes {
            for (minimum_name, minimum) in &minimums {
                for (maximum_name, maximum) in &maximums {
                    let mut raw = json!({
                        "type": "array",
                        "items": items
                    });
                    let object = raw
                        .as_object_mut()
                        .expect("array cardinality schema should stay an object");
                    if let Some(prefix_items) = prefix_items {
                        object.insert("prefixItems".to_owned(), prefix_items.clone());
                    }
                    if let Some(value) = minimum {
                        object.insert("minItems".to_owned(), value.clone());
                    }
                    if let Some(value) = maximum {
                        object.insert("maxItems".to_owned(), value.clone());
                    }
                    let name =
                        format!("array_{prefix_name}_{items_name}_{minimum_name}_{maximum_name}");
                    schemas.push((
                        name,
                        SchemaDocument::from_json(&raw)
                            .expect("dense array cardinality schema should build"),
                    ));
                }
            }
        }
    }

    let witnesses = [json!([]), json!([0]), json!([0, 1]), json!([0, 1, 2])];

    for (old_name, old_schema) in &schemas {
        for (new_name, new_schema) in &schemas {
            for (role, source_name, source, target_name, target) in [
                (Role::Serializer, new_name, new_schema, old_name, old_schema),
                (
                    Role::Deserializer,
                    old_name,
                    old_schema,
                    new_name,
                    new_schema,
                ),
            ] {
                if !check_compat(old_schema, new_schema, role).expect(
                    "dense array cardinality corpus stays within supported checker behavior",
                ) {
                    continue;
                }

                assert_witnesses_preserve_inclusion(
                    role,
                    source_name,
                    source,
                    target_name,
                    target,
                    &witnesses,
                );
            }
        }
    }
}

#[test]
fn claimed_object_dependency_compatibility_survives_presence_witnesses() {
    let schemas = [
        ("object_any", json!({ "type": "object" })),
        (
            "object_property_names_x_only",
            json!({
                "type": "object",
                "propertyNames": { "pattern": "^x$" }
            }),
        ),
        (
            "object_property_names_x_or_y",
            json!({
                "type": "object",
                "propertyNames": { "enum": ["x", "y"] }
            }),
        ),
        (
            "object_pattern_x_integer_closed",
            json!({
                "type": "object",
                "patternProperties": { "^x$": { "type": "integer" } },
                "additionalProperties": false
            }),
        ),
        (
            "object_explicit_x_integer_closed",
            json!({
                "type": "object",
                "properties": { "x": { "type": "integer" } },
                "additionalProperties": false
            }),
        ),
        (
            "object_dependent_x_requires_y",
            json!({
                "type": "object",
                "dependentRequired": { "x": ["y"] }
            }),
        ),
        (
            "object_required_x_and_y",
            json!({
                "type": "object",
                "properties": {
                    "x": true,
                    "y": true
                },
                "required": ["x", "y"]
            }),
        ),
        (
            "object_x_forbidden_by_matching_pattern",
            json!({
                "type": "object",
                "properties": { "x": true },
                "patternProperties": { "^x$": false }
            }),
        ),
    ]
    .into_iter()
    .map(|(name, raw)| {
        (
            name,
            SchemaDocument::from_json(&raw).expect("object soundness schema should build"),
        )
    })
    .collect::<Vec<_>>();

    let witnesses = [
        json!({}),
        json!({ "x": 1 }),
        json!({ "x": "bad" }),
        json!({ "y": "ready" }),
        json!({ "x": 1, "y": "ready" }),
        json!({ "x": "bad", "y": "ready" }),
        json!({ "z": true }),
        json!({ "x": 1, "z": true }),
    ];

    for (old_name, old_schema) in &schemas {
        for (new_name, new_schema) in &schemas {
            for (role, source_name, source, target_name, target) in [
                (Role::Serializer, new_name, new_schema, old_name, old_schema),
                (
                    Role::Deserializer,
                    old_name,
                    old_schema,
                    new_name,
                    new_schema,
                ),
            ] {
                if !check_compat(old_schema, new_schema, role)
                    .expect("object soundness corpus stays within supported checker behavior")
                {
                    continue;
                }

                assert_witnesses_preserve_inclusion(
                    role,
                    source_name,
                    source,
                    target_name,
                    target,
                    &witnesses,
                );
            }
        }
    }
}

#[test]
fn claimed_object_property_compatibility_survives_dense_small_witness_space() {
    let schemas = [
        (
            "object_open",
            json!({
                "type": "object"
            }),
        ),
        (
            "object_x_integer",
            json!({
                "type": "object",
                "properties": { "x": { "type": "integer" } }
            }),
        ),
        (
            "object_required_x_integer",
            json!({
                "type": "object",
                "properties": { "x": { "type": "integer" } },
                "required": ["x"]
            }),
        ),
        (
            "object_y_pattern_string",
            json!({
                "type": "object",
                "patternProperties": { "^y$": { "type": "string" } }
            }),
        ),
        (
            "object_y_pattern_string_closed",
            json!({
                "type": "object",
                "patternProperties": { "^y$": { "type": "string" } },
                "additionalProperties": false
            }),
        ),
        (
            "object_x_integer_y_string_named_only",
            json!({
                "type": "object",
                "properties": { "x": { "type": "integer" } },
                "patternProperties": { "^y$": { "type": "string" } },
                "additionalProperties": false,
                "propertyNames": { "enum": ["x", "y"] }
            }),
        ),
        (
            "object_x_requires_y",
            json!({
                "type": "object",
                "properties": { "x": { "type": "integer" } },
                "patternProperties": { "^y$": { "type": "string" } },
                "dependentRequired": { "x": ["y"] }
            }),
        ),
        (
            "object_y_requires_x",
            json!({
                "type": "object",
                "properties": { "x": { "type": "integer" } },
                "patternProperties": { "^y$": { "type": "string" } },
                "dependentRequired": { "y": ["x"] }
            }),
        ),
        (
            "object_required_xy_closed",
            json!({
                "type": "object",
                "properties": { "x": { "type": "integer" } },
                "patternProperties": { "^y$": { "type": "string" } },
                "additionalProperties": false,
                "required": ["x", "y"]
            }),
        ),
        (
            "object_x_forbidden",
            json!({
                "type": "object",
                "properties": { "x": false }
            }),
        ),
        (
            "object_y_pattern_forbidden_closed",
            json!({
                "type": "object",
                "patternProperties": { "^y$": false },
                "additionalProperties": false
            }),
        ),
    ]
    .into_iter()
    .map(|(name, raw)| {
        (
            name,
            SchemaDocument::from_json(&raw).expect("dense object soundness schema should build"),
        )
    })
    .collect::<Vec<_>>();

    let witnesses = [
        json!({}),
        json!({ "x": 1 }),
        json!({ "x": "x" }),
        json!({ "y": 1 }),
        json!({ "y": "y" }),
        json!({ "z": 1 }),
        json!({ "z": "z" }),
        json!({ "x": 1, "y": 1 }),
        json!({ "x": 1, "y": "y" }),
        json!({ "x": "x", "y": 1 }),
        json!({ "x": "x", "y": "y" }),
        json!({ "x": 1, "z": 1 }),
        json!({ "y": 1, "z": 1 }),
        json!({ "x": 1, "y": 1, "z": 1 }),
    ];

    for (old_name, old_schema) in &schemas {
        for (new_name, new_schema) in &schemas {
            for (role, source_name, source, target_name, target) in [
                (Role::Serializer, new_name, new_schema, old_name, old_schema),
                (
                    Role::Deserializer,
                    old_name,
                    old_schema,
                    new_name,
                    new_schema,
                ),
            ] {
                if !check_compat(old_schema, new_schema, role)
                    .expect("dense object soundness corpus stays within supported checker behavior")
                {
                    continue;
                }

                assert_witnesses_preserve_inclusion(
                    role,
                    source_name,
                    source,
                    target_name,
                    target,
                    &witnesses,
                );
            }
        }
    }
}

#[test]
fn claimed_object_cardinality_compatibility_survives_dense_small_witness_space() {
    let additional_shapes = [("open", Value::Bool(true)), ("closed", Value::Bool(false))];
    let required_shapes = [
        ("required_none", json!([])),
        ("required_x", json!(["x"])),
        ("required_xy", json!(["x", "y"])),
    ];
    let minimums = [
        ("no_min", None),
        ("min_one", Some(json!(1))),
        ("min_two", Some(json!(2))),
    ];
    let maximums = [
        ("no_max", None),
        ("max_one", Some(json!(1))),
        ("max_two", Some(json!(2))),
    ];

    let mut schemas = Vec::new();
    for (additional_name, additional_properties) in &additional_shapes {
        for (required_name, required) in &required_shapes {
            for (minimum_name, minimum) in &minimums {
                for (maximum_name, maximum) in &maximums {
                    let mut raw = json!({
                        "type": "object",
                        "properties": {
                            "x": true,
                            "y": true
                        },
                        "required": required,
                        "additionalProperties": additional_properties
                    });
                    let object = raw
                        .as_object_mut()
                        .expect("object cardinality schema should stay an object");
                    if let Some(value) = minimum {
                        object.insert("minProperties".to_owned(), value.clone());
                    }
                    if let Some(value) = maximum {
                        object.insert("maxProperties".to_owned(), value.clone());
                    }
                    let name = format!(
                        "object_{additional_name}_{required_name}_{minimum_name}_{maximum_name}"
                    );
                    schemas.push((
                        name,
                        SchemaDocument::from_json(&raw)
                            .expect("dense object cardinality schema should build"),
                    ));
                }
            }
        }
    }

    let witnesses = [
        json!({}),
        json!({ "x": true }),
        json!({ "y": true }),
        json!({ "z": true }),
        json!({ "x": true, "y": true }),
        json!({ "x": true, "z": true }),
        json!({ "x": true, "y": true, "z": true }),
    ];

    for (old_name, old_schema) in &schemas {
        for (new_name, new_schema) in &schemas {
            for (role, source_name, source, target_name, target) in [
                (Role::Serializer, new_name, new_schema, old_name, old_schema),
                (
                    Role::Deserializer,
                    old_name,
                    old_schema,
                    new_name,
                    new_schema,
                ),
            ] {
                if !check_compat(old_schema, new_schema, role).expect(
                    "dense object cardinality corpus stays within supported checker behavior",
                ) {
                    continue;
                }

                assert_witnesses_preserve_inclusion(
                    role,
                    source_name,
                    source,
                    target_name,
                    target,
                    &witnesses,
                );
            }
        }
    }
}

#[test]
fn claimed_conditional_compatibility_survives_branch_witnesses() {
    let schemas = [
        ("string", json!({ "type": "string" })),
        ("integer", json!({ "type": "integer" })),
        (
            "non_negative_integer",
            json!({ "type": "integer", "minimum": 0 }),
        ),
        ("const_x", json!({ "const": "x" })),
        ("const_negative_one", json!({ "const": -1 })),
        ("const_positive_one", json!({ "const": 1 })),
        (
            "if_integer_then_non_negative_else_string",
            json!({
                "if": { "type": "integer" },
                "then": { "minimum": 0 },
                "else": { "type": "string" }
            }),
        ),
        (
            "if_true_then_string_else_integer",
            json!({
                "if": true,
                "then": { "type": "string" },
                "else": { "type": "integer" }
            }),
        ),
        (
            "if_false_then_string_else_integer",
            json!({
                "if": false,
                "then": { "type": "string" },
                "else": { "type": "integer" }
            }),
        ),
        (
            "object_kind_detail_conditional",
            json!({
                "type": "object",
                "properties": {
                    "kind": { "enum": ["full", "thin"] },
                    "detail": { "type": "string" }
                },
                "if": {
                    "properties": {
                        "kind": { "const": "full" }
                    },
                    "required": ["kind"]
                },
                "then": { "required": ["detail"] },
                "additionalProperties": false
            }),
        ),
    ]
    .into_iter()
    .map(|(name, raw)| {
        (
            name,
            SchemaDocument::from_json(&raw).expect("conditional soundness schema should build"),
        )
    })
    .collect::<Vec<_>>();

    let witnesses = [
        json!(""),
        json!("x"),
        json!(-1),
        json!(0),
        json!(1),
        json!(true),
        json!({}),
        json!({ "kind": "full" }),
        json!({ "kind": "full", "detail": "ready" }),
        json!({ "kind": "thin" }),
        json!({ "kind": "thin", "detail": "ready" }),
    ];

    for (old_name, old_schema) in &schemas {
        for (new_name, new_schema) in &schemas {
            for (role, source_name, source, target_name, target) in [
                (Role::Serializer, new_name, new_schema, old_name, old_schema),
                (
                    Role::Deserializer,
                    old_name,
                    old_schema,
                    new_name,
                    new_schema,
                ),
            ] {
                if !check_compat(old_schema, new_schema, role)
                    .expect("conditional soundness corpus stays within supported checker behavior")
                {
                    continue;
                }

                assert_witnesses_preserve_inclusion(
                    role,
                    source_name,
                    source,
                    target_name,
                    target,
                    &witnesses,
                );
            }
        }
    }
}

#[test]
fn claimed_combinator_compatibility_survives_overlap_witnesses() {
    let schemas = [
        ("any", json!(true)),
        ("string", json!({ "type": "string" })),
        ("null_only", json!({ "type": "null" })),
        ("const_x", json!({ "const": "x" })),
        ("const_y", json!({ "const": "y" })),
        (
            "const_x_or_y_anyof",
            json!({
                "anyOf": [
                    { "const": "x" },
                    { "const": "y" }
                ]
            }),
        ),
        (
            "const_x_or_y_oneof",
            json!({
                "oneOf": [
                    { "const": "x" },
                    { "const": "y" }
                ]
            }),
        ),
        (
            "string_const_x_allof",
            json!({
                "allOf": [
                    { "type": "string" },
                    { "const": "x" }
                ]
            }),
        ),
        (
            "closed_object_allof_with_unhelpful_metadata_sibling",
            json!({
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
            }),
        ),
        (
            "string_or_null_anyof",
            json!({
                "anyOf": [
                    { "type": "string" },
                    { "type": "null" }
                ]
            }),
        ),
        (
            "string_or_null_oneof",
            json!({
                "oneOf": [
                    { "type": "string" },
                    { "type": "null" }
                ]
            }),
        ),
        (
            "overlapping_string_oneof",
            json!({
                "oneOf": [
                    { "type": "string" },
                    { "const": "x" }
                ]
            }),
        ),
        (
            "string_with_min_length_allof",
            json!({
                "allOf": [
                    { "type": "string" },
                    { "minLength": 1 }
                ]
            }),
        ),
        (
            "string_matching_x_allof",
            json!({
                "allOf": [
                    { "type": "string" },
                    { "pattern": "^x$" }
                ]
            }),
        ),
        ("not_false", json!({ "not": false })),
        ("not_true", json!({ "not": true })),
    ]
    .into_iter()
    .map(|(name, raw)| {
        (
            name,
            SchemaDocument::from_json(&raw).expect("combinator soundness schema should build"),
        )
    })
    .collect::<Vec<_>>();

    let witnesses = [
        json!(null),
        json!(""),
        json!("x"),
        json!("y"),
        json!("xy"),
        json!(1),
        json!(true),
        json!({ "id": "abc" }),
        json!({ "id": "abc", "trace": "ok" }),
    ];

    for (old_name, old_schema) in &schemas {
        for (new_name, new_schema) in &schemas {
            for (role, source_name, source, target_name, target) in [
                (Role::Serializer, new_name, new_schema, old_name, old_schema),
                (
                    Role::Deserializer,
                    old_name,
                    old_schema,
                    new_name,
                    new_schema,
                ),
            ] {
                if !check_compat(old_schema, new_schema, role)
                    .expect("combinator soundness corpus stays within supported checker behavior")
                {
                    continue;
                }

                assert_witnesses_preserve_inclusion(
                    role,
                    source_name,
                    source,
                    target_name,
                    target,
                    &witnesses,
                );
            }
        }
    }
}

#[test]
fn claimed_numeric_compatibility_survives_boundary_witnesses() {
    let schemas = [
        ("number_any", json!({ "type": "number" })),
        ("integer_any", json!({ "type": "integer" })),
        (
            "number_closed_zero_to_ten",
            json!({ "type": "number", "minimum": 0, "maximum": 10 }),
        ),
        (
            "number_open_zero_to_ten",
            json!({
                "type": "number",
                "exclusiveMinimum": 0,
                "exclusiveMaximum": 10
            }),
        ),
        (
            "integer_zero_to_ten",
            json!({ "type": "integer", "minimum": 0, "maximum": 10 }),
        ),
        (
            "integer_even_zero_to_ten",
            json!({
                "type": "integer",
                "minimum": 0,
                "maximum": 10,
                "multipleOf": 2
            }),
        ),
        (
            "integer_half_step_zero_to_ten",
            json!({
                "type": "integer",
                "minimum": 0,
                "maximum": 10,
                "multipleOf": 0.5
            }),
        ),
        (
            "integer_three_halves_zero_to_ten",
            json!({
                "type": "integer",
                "minimum": 0,
                "maximum": 10,
                "multipleOf": 1.5
            }),
        ),
        (
            "integer_three_fifths_zero_to_ten",
            json!({
                "type": "integer",
                "minimum": 0,
                "maximum": 10,
                "multipleOf": 0.6
            }),
        ),
        (
            "number_even_zero_to_ten",
            json!({
                "type": "number",
                "minimum": 0,
                "maximum": 10,
                "multipleOf": 2
            }),
        ),
        (
            "integer_small_enum",
            json!({ "type": "integer", "enum": [0, 2, 10] }),
        ),
        (
            "number_mixed_enum",
            json!({ "type": "number", "enum": [0, 0.5, 2, 10] }),
        ),
        (
            "number_enum_with_dead_zero",
            json!({ "type": "number", "minimum": 1, "enum": [0, 1] }),
        ),
        ("const_zero", json!({ "const": 0 })),
        ("const_half", json!({ "const": 0.5 })),
        ("const_two", json!({ "const": 2 })),
    ]
    .into_iter()
    .map(|(name, raw)| {
        (
            name,
            SchemaDocument::from_json(&raw).expect("numeric soundness schema should build"),
        )
    })
    .collect::<Vec<_>>();

    let witnesses = [
        json!(-1),
        json!(0),
        json!(0.5),
        json!(1),
        json!(2),
        json!(3),
        json!(6),
        json!(9),
        json!(9.5),
        json!(10),
        json!(11),
    ];

    for (old_name, old_schema) in &schemas {
        for (new_name, new_schema) in &schemas {
            for (role, source_name, source, target_name, target) in [
                (Role::Serializer, new_name, new_schema, old_name, old_schema),
                (
                    Role::Deserializer,
                    old_name,
                    old_schema,
                    new_name,
                    new_schema,
                ),
            ] {
                if !check_compat(old_schema, new_schema, role)
                    .expect("numeric soundness corpus stays within supported checker behavior")
                {
                    continue;
                }

                assert_witnesses_preserve_inclusion(
                    role,
                    source_name,
                    source,
                    target_name,
                    target,
                    &witnesses,
                );
            }
        }
    }
}

#[test]
fn claimed_numeric_compatibility_survives_dense_small_witness_space() {
    let numeric_types = [("integer", "integer"), ("number", "number")];
    let minimums = [
        ("no_min", None),
        ("min_zero", Some(("minimum", json!(0)))),
        ("exclusive_min_zero", Some(("exclusiveMinimum", json!(0)))),
    ];
    let maximums = [
        ("no_max", None),
        ("max_two", Some(("maximum", json!(2)))),
        ("exclusive_max_two", Some(("exclusiveMaximum", json!(2)))),
    ];
    let multiples = [
        ("no_multiple", None),
        ("multiple_one", Some(json!(1))),
        ("multiple_two", Some(json!(2))),
    ];
    let enumerations = [
        ("no_enum", None),
        ("enum_zero_one_two", Some(json!([0, 1, 2]))),
        ("enum_zero_two", Some(json!([0, 2]))),
    ];

    let mut schemas = Vec::new();
    for (type_name, schema_type) in numeric_types {
        for (minimum_name, minimum) in &minimums {
            for (maximum_name, maximum) in &maximums {
                for (multiple_name, multiple_of) in &multiples {
                    for (enum_name, enumeration) in &enumerations {
                        let mut raw = json!({
                            "type": schema_type
                        });
                        let object = raw
                            .as_object_mut()
                            .expect("numeric soundness schema should stay an object");
                        if let Some((keyword, value)) = minimum {
                            object.insert((*keyword).to_owned(), value.clone());
                        }
                        if let Some((keyword, value)) = maximum {
                            object.insert((*keyword).to_owned(), value.clone());
                        }
                        if let Some(value) = multiple_of {
                            object.insert("multipleOf".to_owned(), value.clone());
                        }
                        if let Some(value) = enumeration {
                            object.insert("enum".to_owned(), value.clone());
                        }
                        let name = format!(
                            "numeric_{type_name}_{minimum_name}_{maximum_name}_{multiple_name}_{enum_name}"
                        );
                        schemas.push((
                            name,
                            SchemaDocument::from_json(&raw)
                                .expect("dense numeric soundness schema should build"),
                        ));
                    }
                }
            }
        }
    }

    let witnesses = [
        json!(-1),
        json!(0),
        json!(0.5),
        json!(1),
        json!(1.5),
        json!(2),
        json!(3),
    ];

    for (old_name, old_schema) in &schemas {
        for (new_name, new_schema) in &schemas {
            for (role, source_name, source, target_name, target) in [
                (Role::Serializer, new_name, new_schema, old_name, old_schema),
                (
                    Role::Deserializer,
                    old_name,
                    old_schema,
                    new_name,
                    new_schema,
                ),
            ] {
                if !check_compat(old_schema, new_schema, role).expect(
                    "dense numeric soundness corpus stays within supported checker behavior",
                ) {
                    continue;
                }

                assert_witnesses_preserve_inclusion(
                    role,
                    source_name,
                    source,
                    target_name,
                    target,
                    &witnesses,
                );
            }
        }
    }
}

#[test]
fn claimed_string_compatibility_survives_dense_small_witness_space() {
    let length_shapes = [
        ("unbounded", None, None),
        ("min_one", Some(("minLength", json!(1))), None),
        ("max_two", None, Some(("maxLength", json!(2)))),
        (
            "one_to_two",
            Some(("minLength", json!(1))),
            Some(("maxLength", json!(2))),
        ),
    ];
    let patterns = [
        ("no_pattern", None),
        ("pattern_x", Some(json!("^x$"))),
        ("pattern_xy", Some(json!("^xy$"))),
    ];
    let enumerations = [
        ("no_enum", None),
        ("enum_empty_x_xy", Some(json!(["", "x", "xy"]))),
        ("enum_x_xy", Some(json!(["x", "xy"]))),
    ];
    let consts = [
        ("no_const", None),
        ("const_x", Some(json!("x"))),
        ("const_xy", Some(json!("xy"))),
    ];

    let mut schemas = Vec::new();
    for (length_name, minimum, maximum) in &length_shapes {
        for (pattern_name, pattern) in &patterns {
            for (enum_name, enumeration) in &enumerations {
                for (const_name, const_value) in &consts {
                    let mut raw = json!({
                        "type": "string"
                    });
                    let object = raw
                        .as_object_mut()
                        .expect("string soundness schema should stay an object");
                    if let Some((keyword, value)) = minimum {
                        object.insert((*keyword).to_owned(), value.clone());
                    }
                    if let Some((keyword, value)) = maximum {
                        object.insert((*keyword).to_owned(), value.clone());
                    }
                    if let Some(value) = pattern {
                        object.insert("pattern".to_owned(), value.clone());
                    }
                    if let Some(value) = enumeration {
                        object.insert("enum".to_owned(), value.clone());
                    }
                    if let Some(value) = const_value {
                        object.insert("const".to_owned(), value.clone());
                    }
                    let name =
                        format!("string_{length_name}_{pattern_name}_{enum_name}_{const_name}");
                    schemas.push((
                        name,
                        SchemaDocument::from_json(&raw)
                            .expect("dense string soundness schema should build"),
                    ));
                }
            }
        }
    }

    let witnesses = [json!(""), json!("x"), json!("xy"), json!("xyz"), json!("y")];

    for (old_name, old_schema) in &schemas {
        for (new_name, new_schema) in &schemas {
            for (role, source_name, source, target_name, target) in [
                (Role::Serializer, new_name, new_schema, old_name, old_schema),
                (
                    Role::Deserializer,
                    old_name,
                    old_schema,
                    new_name,
                    new_schema,
                ),
            ] {
                if !check_compat(old_schema, new_schema, role)
                    .expect("dense string soundness corpus stays within supported checker behavior")
                {
                    continue;
                }

                assert_witnesses_preserve_inclusion(
                    role,
                    source_name,
                    source,
                    target_name,
                    target,
                    &witnesses,
                );
            }
        }
    }
}

#[test]
fn claimed_primitive_type_union_compatibility_survives_dense_witness_space() {
    let schemas = [
        ("string", json!({ "type": "string" })),
        ("integer", json!({ "type": "integer" })),
        ("number", json!({ "type": "number" })),
        ("boolean", json!({ "type": "boolean" })),
        ("null", json!({ "type": "null" })),
        (
            "string_or_integer",
            json!({ "type": ["string", "integer"] }),
        ),
        ("string_or_null", json!({ "type": ["string", "null"] })),
        (
            "integer_or_number",
            json!({ "type": ["integer", "number"] }),
        ),
        ("boolean_or_null", json!({ "type": ["boolean", "null"] })),
        (
            "string_or_integer_or_null",
            json!({ "type": ["string", "integer", "null"] }),
        ),
    ]
    .into_iter()
    .map(|(name, raw)| {
        (
            name,
            SchemaDocument::from_json(&raw)
                .expect("primitive type-union soundness schema should build"),
        )
    })
    .collect::<Vec<_>>();

    let witnesses = [
        json!(null),
        json!(false),
        json!(true),
        json!(""),
        json!("x"),
        json!(-1),
        json!(0),
        json!(1),
        json!(0.5),
        json!([]),
        json!({}),
    ];

    for (old_name, old_schema) in &schemas {
        for (new_name, new_schema) in &schemas {
            for (role, source_name, source, target_name, target) in [
                (Role::Serializer, new_name, new_schema, old_name, old_schema),
                (
                    Role::Deserializer,
                    old_name,
                    old_schema,
                    new_name,
                    new_schema,
                ),
            ] {
                if !check_compat(old_schema, new_schema, role).expect(
                    "primitive type-union soundness corpus stays within supported checker behavior",
                ) {
                    continue;
                }

                assert_witnesses_preserve_inclusion(
                    role,
                    source_name,
                    source,
                    target_name,
                    target,
                    &witnesses,
                );
            }
        }
    }
}

#[test]
fn claimed_constrained_enum_compatibility_survives_dense_witness_space() {
    let schemas = [
        (
            "string_enum_all",
            json!({
                "type": "string",
                "enum": ["", "x", "xy"]
            }),
        ),
        (
            "string_enum_min_one",
            json!({
                "type": "string",
                "enum": ["", "x", "xy"],
                "minLength": 1
            }),
        ),
        (
            "string_enum_pattern_x",
            json!({
                "type": "string",
                "enum": ["", "x", "xy"],
                "pattern": "^x$"
            }),
        ),
        (
            "number_enum_all",
            json!({
                "type": "number",
                "enum": [0, 1, 2]
            }),
        ),
        (
            "number_enum_min_one",
            json!({
                "type": "number",
                "enum": [0, 1, 2],
                "minimum": 1
            }),
        ),
        (
            "integer_enum_even",
            json!({
                "type": "integer",
                "enum": [0, 1, 2],
                "multipleOf": 2
            }),
        ),
        (
            "object_enum_all",
            json!({
                "type": "object",
                "enum": [{}, { "x": 1 }, { "x": "x" }]
            }),
        ),
        (
            "object_enum_required_integer_x",
            json!({
                "type": "object",
                "properties": { "x": { "type": "integer" } },
                "required": ["x"],
                "additionalProperties": false,
                "enum": [{}, { "x": 1 }, { "x": "x" }]
            }),
        ),
        (
            "array_enum_all",
            json!({
                "type": "array",
                "enum": [[], [0], ["x"]]
            }),
        ),
        (
            "array_enum_integer_items",
            json!({
                "type": "array",
                "items": { "type": "integer" },
                "enum": [[], [0], ["x"]]
            }),
        ),
    ]
    .into_iter()
    .map(|(name, raw)| {
        (
            name,
            SchemaDocument::from_json(&raw)
                .expect("constrained-enum soundness schema should build"),
        )
    })
    .collect::<Vec<_>>();

    let witnesses = [
        json!(""),
        json!("x"),
        json!("xy"),
        json!(0),
        json!(1),
        json!(2),
        json!({}),
        json!({ "x": 1 }),
        json!({ "x": "x" }),
        json!([]),
        json!([0]),
        json!(["x"]),
    ];

    for (old_name, old_schema) in &schemas {
        for (new_name, new_schema) in &schemas {
            for (role, source_name, source, target_name, target) in [
                (Role::Serializer, new_name, new_schema, old_name, old_schema),
                (
                    Role::Deserializer,
                    old_name,
                    old_schema,
                    new_name,
                    new_schema,
                ),
            ] {
                if !check_compat(old_schema, new_schema, role).expect(
                    "constrained-enum soundness corpus stays within supported checker behavior",
                ) {
                    continue;
                }

                assert_witnesses_preserve_inclusion(
                    role,
                    source_name,
                    source,
                    target_name,
                    target,
                    &witnesses,
                );
            }
        }
    }
}

#[test]
fn claimed_recursive_applicator_compatibility_survives_nested_witnesses() {
    let schemas = [
        (
            "recursive_anyof_string_or_next",
            json!({
                "$defs": {
                    "node": {
                        "anyOf": [
                            { "type": "string" },
                            {
                                "type": "object",
                                "properties": {
                                    "next": { "$ref": "#/$defs/node" }
                                },
                                "required": ["next"],
                                "additionalProperties": false
                            }
                        ]
                    }
                },
                "$ref": "#/$defs/node"
            }),
        ),
        (
            "recursive_anyof_string_integer_or_next",
            json!({
                "$defs": {
                    "node": {
                        "anyOf": [
                            { "type": "string" },
                            { "type": "integer" },
                            {
                                "type": "object",
                                "properties": {
                                    "next": { "$ref": "#/$defs/node" }
                                },
                                "required": ["next"],
                                "additionalProperties": false
                            }
                        ]
                    }
                },
                "$ref": "#/$defs/node"
            }),
        ),
        (
            "recursive_allof_object_with_optional_string_value",
            json!({
                "$defs": {
                    "node": {
                        "allOf": [
                            {
                                "type": "object",
                                "properties": {
                                    "next": { "$ref": "#/$defs/node" }
                                },
                                "additionalProperties": false
                            },
                            {
                                "type": "object",
                                "properties": {
                                    "value": { "type": "string" }
                                }
                            }
                        ]
                    }
                },
                "$ref": "#/$defs/node"
            }),
        ),
        (
            "recursive_allof_object_with_required_string_value",
            json!({
                "$defs": {
                    "node": {
                        "allOf": [
                            {
                                "type": "object",
                                "properties": {
                                    "next": { "$ref": "#/$defs/node" }
                                },
                                "additionalProperties": false
                            },
                            {
                                "type": "object",
                                "properties": {
                                    "value": { "type": "string" }
                                },
                                "required": ["value"]
                            }
                        ]
                    }
                },
                "$ref": "#/$defs/node"
            }),
        ),
    ]
    .into_iter()
    .map(|(name, raw)| {
        (
            name,
            SchemaDocument::from_json(&raw)
                .expect("recursive applicator soundness schema should build"),
        )
    })
    .collect::<Vec<_>>();

    let witnesses = [
        json!("leaf"),
        json!(1),
        json!({ "next": "leaf" }),
        json!({ "next": 1 }),
        json!({ "value": "root" }),
        json!({ "value": "root", "next": { "value": "child" } }),
        json!({ "next": { "value": "child" } }),
    ];

    for (old_name, old_schema) in &schemas {
        for (new_name, new_schema) in &schemas {
            for (role, source_name, source, target_name, target) in [
                (Role::Serializer, new_name, new_schema, old_name, old_schema),
                (
                    Role::Deserializer,
                    old_name,
                    old_schema,
                    new_name,
                    new_schema,
                ),
            ] {
                if !check_compat(old_schema, new_schema, role).expect(
                    "recursive applicator soundness corpus stays within supported checker behavior",
                ) {
                    continue;
                }

                assert_witnesses_preserve_inclusion(
                    role,
                    source_name,
                    source,
                    target_name,
                    target,
                    &witnesses,
                );
            }
        }
    }
}

#[test]
fn claimed_recursive_oneof_compatibility_survives_nested_witnesses() {
    let schemas = [
        (
            "recursive_oneof_string_or_next",
            json!({
                "$defs": {
                    "node": {
                        "oneOf": [
                            { "type": "string" },
                            {
                                "type": "object",
                                "properties": {
                                    "next": { "$ref": "#/$defs/node" }
                                },
                                "required": ["next"],
                                "additionalProperties": false
                            }
                        ]
                    }
                },
                "$ref": "#/$defs/node"
            }),
        ),
        (
            "recursive_oneof_string_integer_or_next",
            json!({
                "$defs": {
                    "node": {
                        "oneOf": [
                            { "type": "string" },
                            { "type": "integer" },
                            {
                                "type": "object",
                                "properties": {
                                    "next": { "$ref": "#/$defs/node" }
                                },
                                "required": ["next"],
                                "additionalProperties": false
                            }
                        ]
                    }
                },
                "$ref": "#/$defs/node"
            }),
        ),
        (
            "recursive_oneof_string_const_x_or_next",
            json!({
                "$defs": {
                    "node": {
                        "oneOf": [
                            { "type": "string" },
                            { "const": "x" },
                            {
                                "type": "object",
                                "properties": {
                                    "next": { "$ref": "#/$defs/node" }
                                },
                                "required": ["next"],
                                "additionalProperties": false
                            }
                        ]
                    }
                },
                "$ref": "#/$defs/node"
            }),
        ),
    ]
    .into_iter()
    .map(|(name, raw)| {
        (
            name,
            SchemaDocument::from_json(&raw).expect("recursive oneOf soundness schema should build"),
        )
    })
    .collect::<Vec<_>>();

    let witnesses = [
        json!("leaf"),
        json!("x"),
        json!(1),
        json!({ "next": "leaf" }),
        json!({ "next": "x" }),
        json!({ "next": 1 }),
        json!({ "next": { "next": "leaf" } }),
    ];

    for (old_name, old_schema) in &schemas {
        for (new_name, new_schema) in &schemas {
            for (role, source_name, source, target_name, target) in [
                (Role::Serializer, new_name, new_schema, old_name, old_schema),
                (
                    Role::Deserializer,
                    old_name,
                    old_schema,
                    new_name,
                    new_schema,
                ),
            ] {
                if !check_compat(old_schema, new_schema, role).expect(
                    "recursive oneOf soundness corpus stays within supported checker behavior",
                ) {
                    continue;
                }

                assert_witnesses_preserve_inclusion(
                    role,
                    source_name,
                    source,
                    target_name,
                    target,
                    &witnesses,
                );
            }
        }
    }
}

#[test]
fn claimed_same_value_recursive_applicator_compatibility_survives_witnesses() {
    let schemas = [
        (
            "same_value_recursive_anyof_string",
            json!({
                "$defs": {
                    "node": {
                        "anyOf": [
                            { "$ref": "#/$defs/node" },
                            { "type": "string" }
                        ]
                    }
                },
                "$ref": "#/$defs/node"
            }),
        ),
        (
            "same_value_recursive_oneof_string",
            json!({
                "$defs": {
                    "node": {
                        "oneOf": [
                            { "$ref": "#/$defs/node" },
                            { "type": "string" }
                        ]
                    }
                },
                "$ref": "#/$defs/node"
            }),
        ),
        (
            "same_value_recursive_allof_string",
            json!({
                "$defs": {
                    "node": {
                        "allOf": [
                            { "$ref": "#/$defs/node" },
                            { "type": "string" }
                        ]
                    }
                },
                "$ref": "#/$defs/node"
            }),
        ),
    ]
    .into_iter()
    .map(|(name, raw)| {
        (
            name,
            SchemaDocument::from_json(&raw)
                .expect("same-value recursive applicator schema should build"),
        )
    })
    .collect::<Vec<_>>();

    let witnesses = [
        json!("leaf"),
        json!(""),
        json!(1),
        json!(null),
        json!({}),
        json!([]),
    ];

    for (old_name, old_schema) in &schemas {
        for (new_name, new_schema) in &schemas {
            for (role, source_name, source, target_name, target) in [
                (Role::Serializer, new_name, new_schema, old_name, old_schema),
                (
                    Role::Deserializer,
                    old_name,
                    old_schema,
                    new_name,
                    new_schema,
                ),
            ] {
                if !check_compat(old_schema, new_schema, role).expect(
                    "same-value recursive applicator soundness corpus stays within supported checker behavior",
                ) {
                    continue;
                }

                assert_witnesses_preserve_inclusion(
                    role,
                    source_name,
                    source,
                    target_name,
                    target,
                    &witnesses,
                );
            }
        }
    }
}

fn assert_witnesses_preserve_inclusion(
    role: Role,
    source_name: &str,
    source: &SchemaDocument,
    target_name: &str,
    target: &SchemaDocument,
    witnesses: &[Value],
) {
    for witness in witnesses {
        if source
            .is_valid(witness)
            .expect("source witness validation should succeed")
        {
            if role == Role::Deserializer
                && witness_uses_only_additional_object_properties(source, witness)
            {
                continue;
            }
            assert!(
                target
                    .is_valid(witness)
                    .expect("target witness validation should succeed"),
                "{role:?} compatibility claimed {source_name} ⊆ {target_name}, but target rejected witness {witness}",
            );
        }
    }
}

// Deserializer checks intentionally model what the old serializer can emit,
// not every object accepted by `additionalProperties`.
fn witness_uses_only_additional_object_properties(
    source: &SchemaDocument,
    witness: &Value,
) -> bool {
    let Some(object) = witness.as_object() else {
        return false;
    };
    let Some(schema) = source.source_schema_json().as_object() else {
        return false;
    };

    let explicit_properties = schema.get("properties").and_then(Value::as_object);
    let pattern_properties = schema.get("patternProperties").and_then(Value::as_object);

    object.keys().any(|property| {
        let explicitly_declared =
            explicit_properties.is_some_and(|properties| properties.contains_key(property));
        let pattern_declared = pattern_properties.is_some_and(|patterns| {
            patterns.keys().any(|pattern| {
                Regex::new(pattern)
                    .and_then(|regex| regex.is_match(property))
                    .unwrap_or(true)
            })
        });

        !explicitly_declared && !pattern_declared
    })
}
