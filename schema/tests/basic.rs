use json_schema_ast as schema;

use schema::{
    AstError, CompileError, ResolvedNodeKind as SchemaNodeKind, ResolvedSchema, SchemaError,
    SchemaNode, build_and_resolve_schema, compile,
};
use serde_json::Value;
use serde_json::json;

#[test]
fn roundtrip_compile_validate() {
    let raw = json!({"type":"string", "minLength":3});
    let ast = build_schema(&raw);
    let compiled = compile_ast(&ast);
    assert!(compiled.is_valid(&json!("abc")));
    assert!(!compiled.is_valid(&json!("ab")));
}

#[test]
fn compile_validates_original_type_union_schema() {
    let raw = json!({
        "type": ["integer", "string"],
        "minimum": 2,
        "minLength": 2
    });

    let compiled = compile(&raw).unwrap();

    assert!(compiled.is_valid(&json!(2)));
    assert!(compiled.is_valid(&json!("ab")));
    assert!(!compiled.is_valid(&json!(1)));
    assert!(!compiled.is_valid(&json!("a")));
    assert!(!compiled.is_valid(&json!(null)));
}

#[test]
fn resolved_schema_validates_with_raw_backend_and_exposes_canonicalized_debug_json() {
    let raw = json!({
        "type": ["integer", "string"],
        "minimum": 2,
        "minLength": 2,
        "not": {
            "const": "zz"
        }
    });

    let schema = ResolvedSchema::from_json(&raw).unwrap();
    assert_eq!(schema.raw_schema_json(), &raw);

    let canonical = schema.canonical_schema_json().unwrap();
    let canonical_compiled = compile(canonical).unwrap();

    for (candidate, expected_valid) in [
        (json!(2), true),
        (json!("ab"), true),
        (json!("zz"), false),
        (json!(1), false),
        (json!("a"), false),
        (json!(null), false),
    ] {
        assert_eq!(schema.is_valid(&candidate).unwrap(), expected_valid);
        assert_eq!(
            schema.is_valid(&candidate).unwrap(),
            schema.is_valid_canonicalized(&candidate).unwrap(),
            "raw and canonicalized validators disagree for {candidate}\n\nRaw schema:\n{}\n\nCanonicalized schema:\n{}",
            serde_json::to_string_pretty(schema.raw_schema_json()).unwrap(),
            serde_json::to_string_pretty(schema.canonical_schema_json().unwrap()).unwrap(),
        );
        assert_eq!(
            schema.is_valid_canonicalized(&candidate).unwrap(),
            canonical_compiled.is_valid(&candidate),
        );
        assert_eq!(
            schema.root().unwrap().accepts_value(&candidate),
            schema.is_valid_canonicalized(&candidate).unwrap(),
        );
    }
}

#[test]
fn compile_rejects_non_2020_12_schema_uri_before_validator_backend() {
    let raw = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "string"
    });

    let error = compile(&raw).unwrap_err();

    assert!(matches!(
        error,
        CompileError::Schema(SchemaError::UnsupportedSchemaDialect {
            pointer,
            expected_uri: "https://json-schema.org/draft/2020-12/schema",
            actual_uri,
        }) if pointer == "#/$schema" && actual_uri == "http://json-schema.org/draft-07/schema#"
    ));
}

#[test]
fn compile_does_not_treat_schema_keys_inside_const_values_as_nested_dialects() {
    let raw = json!({
        "const": {
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "string"
        }
    });

    let compiled = compile(&raw).unwrap();

    assert!(compiled.is_valid(&json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "string"
    })));
    assert!(!compiled.is_valid(&json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "string"
    })));
}

#[test]
fn compile_error_is_owned_after_input_schema_is_dropped() {
    let error = {
        let raw = json!({
            "type": 1
        });
        compile(&raw).unwrap_err()
    };

    assert!(matches!(
        error,
        CompileError::ValidatorRejectedSchema { .. }
    ));
    assert!(
        error
            .to_string()
            .contains("schema failed Draft 2020-12 validator compilation")
    );
}

#[test]
fn resolve_local_ref() {
    let raw = json!({
        "definitions": {"Int": {"type":"integer"}},
        "$ref": "#/definitions/Int"
    });
    let ast = build_and_resolve_schema(&raw).unwrap();
    assert!(matches!(ast.kind(), SchemaNodeKind::Integer { .. }));
}

#[test]
fn resolve_local_ref_through_escaped_map_keys() {
    let raw = json!({
        "$defs": {
            "a/b": {
                "properties": {
                    "x~y": {
                        "type": "string"
                    }
                }
            }
        },
        "$ref": "#/$defs/a~1b/properties/x~0y"
    });

    let ast = build_and_resolve_schema(&raw).unwrap();

    assert!(matches!(ast.kind(), SchemaNodeKind::String { .. }));
}

#[test]
fn resolve_local_ref_preserves_escaped_property_names_under_recursive_object() {
    let raw = json!({
        "$defs": {
            "node/root": {
                "type": "object",
                "properties": {
                    "next~node": {
                        "$ref": "#/$defs/node~1root"
                    }
                }
            }
        },
        "$ref": "#/$defs/node~1root"
    });

    let ast = build_and_resolve_schema(&raw).unwrap();

    let guard = ast.kind();
    if let SchemaNodeKind::Object { properties, .. } = guard {
        let next = properties.get("next~node").expect("escaped property");
        assert!(next.ptr_eq(&ast));
    } else {
        panic!("expected object schema");
    }
}

#[test]
fn resolve_recursive_ref() {
    let raw = json!({
        "$defs": {
            "Node": {
                "type": "object",
                "properties": {
                    "value": {"type": "integer"},
                    "next": {"$ref": "#/$defs/Node"}
                },
                "required": ["value"]
            }
        },
        "$ref": "#/$defs/Node"
    });
    let ast = build_and_resolve_schema(&raw).unwrap();
    {
        let guard = ast.kind();
        if let SchemaNodeKind::Object { properties, .. } = guard {
            let next = properties.get("next").expect("next property");
            assert!(next.ptr_eq(&ast));
        } else {
            panic!("expected object schema");
        }
    }
}

#[test]
fn resolve_duplicate_refs_share_pointer() {
    let raw = json!({
        "$defs": {
            "Thing": {"type": "integer"}
        },
        "type": "object",
        "properties": {
            "a": {"$ref": "#/$defs/Thing"},
            "b": {"$ref": "#/$defs/Thing"}
        }
    });
    let ast = build_and_resolve_schema(&raw).unwrap();

    let guard = ast.kind();
    if let SchemaNodeKind::Object { properties, .. } = guard {
        let a = properties.get("a").expect("property a");
        let b = properties.get("b").expect("property b");
        assert!(a.ptr_eq(b));
    } else {
        panic!("expected object schema");
    }
}

#[test]
fn resolve_self_recursive_allof_without_panicking() {
    let raw = json!({
        "$defs": {
            "A": {
                "allOf": [
                    { "$ref": "#/$defs/A" }
                ]
            }
        },
        "$ref": "#/$defs/A"
    });

    let ast = build_and_resolve_schema(&raw).unwrap();
    let guard = ast.kind();
    assert!(matches!(guard, SchemaNodeKind::AllOf(children) if children.len() == 1));
}

#[test]
fn boolean_schemas() {
    let sample = json!({"k":1});
    for b in [true, false] {
        let raw = json!(b);
        let ast = build_schema(&raw);
        let compiled = compile_ast(&ast);
        assert_eq!(compiled.is_valid(&sample), b);
    }
}

#[test]
fn conditional_roundtrip() {
    let raw = json!({
        "if": {"type": "integer"},
        "then": {"minimum": 0},
        "else": {"type": "string"}
    });
    let ast = build_schema(&raw);
    let json = ast.to_json();
    let ast2 = build_schema(&json);
    assert_eq!(ast, ast2);
}

#[test]
fn enum_with_minimum_uses_number_schema_shape() {
    let ast = build_schema(&json!({
        "type": "number",
        "enum": [0, 1],
        "minimum": 1
    }));

    let guard = ast.kind();
    let SchemaNodeKind::Number {
        minimum,
        enumeration,
        ..
    } = guard
    else {
        panic!("expected number schema, got {guard:?}");
    };

    assert_eq!(*minimum, Some(1.0));
    assert_eq!(enumeration.as_ref().unwrap(), &vec![json!(0), json!(1)]);
}

#[test]
fn non_numeric_enum_with_minimum_uses_terminal_enum_shape() {
    let ast = build_schema(&json!({
        "enum": ["x"],
        "minimum": 1
    }));

    let guard = ast.kind();
    let SchemaNodeKind::Enum(values) = guard else {
        panic!("expected enum schema, got {guard:?}");
    };

    assert_eq!(values, &vec![json!("x")]);
}

#[test]
fn non_object_enum_with_object_keywords_uses_terminal_enum_shape() {
    let ast = build_schema(&json!({
        "properties": {
            "x": true
        },
        "enum": [1]
    }));

    let guard = ast.kind();
    let SchemaNodeKind::Enum(values) = guard else {
        panic!("expected enum schema, got {guard:?}");
    };

    assert_eq!(values, &vec![json!(1)]);
}

#[test]
fn non_array_enum_with_array_keywords_uses_terminal_enum_shape() {
    let ast = build_schema(&json!({
        "items": true,
        "enum": [1]
    }));

    let guard = ast.kind();
    let SchemaNodeKind::Enum(values) = guard else {
        panic!("expected enum schema, got {guard:?}");
    };

    assert_eq!(values, &vec![json!(1)]);
}

#[test]
fn non_string_enum_with_string_keywords_uses_terminal_enum_shape() {
    let ast = build_schema(&json!({
        "minLength": 1,
        "enum": [1]
    }));

    let guard = ast.kind();
    let SchemaNodeKind::Enum(values) = guard else {
        panic!("expected enum schema, got {guard:?}");
    };

    assert_eq!(values, &vec![json!(1)]);
}

#[test]
fn const_with_pattern_uses_string_schema_shape() {
    let ast = build_schema(&json!({
        "const": "abc",
        "pattern": "^a"
    }));

    let guard = ast.kind();
    let SchemaNodeKind::String {
        pattern,
        enumeration,
        ..
    } = guard
    else {
        panic!("expected string schema, got {guard:?}");
    };

    assert_eq!(pattern.as_deref(), Some("^a"));
    assert_eq!(enumeration.as_ref().unwrap(), &vec![json!("abc")]);
}

#[test]
fn nested_format_only_schema_uses_string_branch_in_implicit_union() {
    let ast = build_schema(&json!({
        "type": "object",
        "properties": {
            "email": { "format": "email" }
        }
    }));

    let guard = ast.kind();
    let SchemaNodeKind::Object { properties, .. } = guard else {
        panic!("expected object schema, got {guard:?}");
    };

    let email = properties.get("email").expect("email property");
    let email_guard = email.kind();
    let SchemaNodeKind::AnyOf(branches) = email_guard else {
        panic!("expected implicit union for email property, got {email_guard:?}");
    };

    assert!(branches.iter().any(|branch| {
        matches!(
            branch.kind(),
            SchemaNodeKind::String {
                format: Some(format),
                ..
            } if format == "email"
        )
    }));
}

#[test]
fn bare_multiple_of_under_conditional_gets_the_same_implicit_union_as_root() {
    let ast = build_schema(&json!({
        "if": { "type": "boolean" },
        "then": { "multipleOf": 2 },
        "else": { "type": "string" }
    }));

    let guard = ast.kind();
    let SchemaNodeKind::IfThenElse {
        then_schema,
        else_schema,
        ..
    } = guard
    else {
        panic!("expected conditional schema, got {guard:?}");
    };

    let then_schema = then_schema.as_ref().expect("then schema");
    assert!(matches!(
        then_schema.kind(),
        SchemaNodeKind::AnyOf(branches)
            if branches
                .iter()
                .any(|branch| matches!(branch.kind(), SchemaNodeKind::Number { .. }))
    ));
    let else_schema = else_schema.as_ref().expect("else schema");
    assert!(matches!(else_schema.kind(), SchemaNodeKind::String { .. }));
}

#[test]
fn metadata_only_enum_wrapper_uses_terminal_enum_shape() {
    let ast = build_schema(&json!({
        "title": "Answer",
        "x-jsoncompat": {
            "kind": "declaration",
            "stable_id": "answer",
            "name": "Answer",
            "version": 1,
            "schema_ref": "#"
        },
        "enum": [42]
    }));

    let guard = ast.kind();
    let SchemaNodeKind::Enum(values) = guard else {
        panic!("expected enum schema, got {guard:?}");
    };

    assert_eq!(values, &vec![json!(42)]);
}

#[test]
fn resolves_local_refs_with_percent_encoded_pointer_tokens() {
    let ast = build_and_resolve_schema(&json!({
        "x foo": { "type": "string" },
        "$ref": "#/x%20foo"
    }))
    .unwrap();

    assert!(matches!(ast.kind(), SchemaNodeKind::String { .. }));
}

#[test]
fn preserves_dangling_then_target_without_if() {
    let ast = build_and_resolve_schema(&json!({
        "allOf": [
            { "$ref": "#/then" }
        ],
        "then": {
            "type": "string"
        }
    }))
    .unwrap();

    assert!(matches!(ast.kind(), SchemaNodeKind::String { .. }));
}

#[test]
fn preserves_if_target_without_then_or_else() {
    let ast = build_and_resolve_schema(&json!({
        "allOf": [
            { "$ref": "#/if" }
        ],
        "if": {
            "type": "string"
        }
    }))
    .unwrap();

    assert!(matches!(ast.kind(), SchemaNodeKind::String { .. }));
}

#[test]
fn preserves_not_false_target_when_referenced_from_then_branch() {
    let ast = build_and_resolve_schema(&json!({
        "if": false,
        "then": {
            "$ref": "#/not"
        },
        "not": false
    }))
    .unwrap();

    let guard = ast.kind();
    let SchemaNodeKind::IfThenElse {
        if_schema,
        then_schema: Some(then_schema),
        else_schema: None,
    } = guard
    else {
        panic!("expected normalized conditional schema, got {guard:?}");
    };

    assert!(matches!(
        if_schema.kind(),
        SchemaNodeKind::BoolSchema(false)
    ));
    assert!(matches!(
        then_schema.kind(),
        SchemaNodeKind::BoolSchema(false)
    ));

    let compiled = compile_ast(&ast);
    assert!(compiled.is_valid(&json!(null)));
    assert!(compiled.is_valid(&json!("value")));
}

#[test]
fn preserves_indexed_allof_ref_targets_when_deduping_equivalent_branches() {
    let ast = build_and_resolve_schema(&json!({
        "allOf": [
            {
                "$ref": "#/x/allOf/1/properties/value"
            }
        ],
        "x": {
            "allOf": [
                {
                    "type": "object",
                    "properties": {
                        "value": { "type": "string" }
                    }
                },
                {
                    "type": "object",
                    "properties": {
                        "value": { "type": "string" }
                    }
                }
            ]
        }
    }))
    .unwrap();

    assert!(matches!(ast.kind(), SchemaNodeKind::String { .. }));
}

#[test]
fn preserves_referenced_descendants_under_unsatisfiable_branches() {
    let ast = build_and_resolve_schema(&json!({
        "allOf": [
            {
                "$ref": "#/x/allOf/0/properties/value"
            }
        ],
        "x": {
            "allOf": [
                {
                    "type": "object",
                    "properties": {
                        "value": { "type": "string" }
                    },
                    "minProperties": 2,
                    "maxProperties": 1
                }
            ]
        }
    }))
    .unwrap();

    assert!(matches!(ast.kind(), SchemaNodeKind::String { .. }));
}

#[test]
fn preserves_referenced_defs_when_anyof_or_oneof_collapses_to_false() {
    for schema in [
        json!({
            "allOf": [
                { "$ref": "#/x/$defs/A" }
            ],
            "x": {
                "anyOf": [false],
                "$defs": {
                    "A": { "type": "string" }
                }
            }
        }),
        json!({
            "allOf": [
                { "$ref": "#/x/$defs/A" }
            ],
            "x": {
                "oneOf": [false],
                "$defs": {
                    "A": { "type": "string" }
                }
            }
        }),
    ] {
        let ast = build_and_resolve_schema(&schema).unwrap();
        assert!(matches!(ast.kind(), SchemaNodeKind::String { .. }));
    }
}

#[test]
fn preserves_referenced_anyof_branch_when_true_branch_is_present() {
    let ast = build_and_resolve_schema(&json!({
        "allOf": [
            { "$ref": "#/anyOf/0" }
        ],
        "anyOf": [
            true,
            { "type": "string" }
        ]
    }))
    .unwrap();

    assert!(matches!(
        ast.kind(),
        SchemaNodeKind::Any | SchemaNodeKind::BoolSchema(true)
    ));
}

#[test]
fn preserves_referenced_defs_when_false_schema_is_canonicalized() {
    let ast = build_and_resolve_schema(&json!({
        "x": {
            "allOf": [false],
            "$defs": {
                "A": { "type": "string" }
            }
        },
        "$ref": "#/x/$defs/A"
    }))
    .unwrap();

    assert!(matches!(ast.kind(), SchemaNodeKind::String { .. }));
}

#[test]
fn resolved_schema_contains_only_public_node_variants_for_recursive_local_refs() {
    let schema = ResolvedSchema::from_json(&json!({
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
        "allOf": [
            { "$ref": "#/$defs/Node" },
            {
                "if": {
                    "properties": {
                        "next": { "type": "object" }
                    },
                    "required": ["next"]
                },
                "then": {
                    "properties": {
                        "value": { "minimum": 0 }
                    }
                }
            }
        ]
    }))
    .unwrap();

    assert_resolved_graph_is_public(
        schema.root().unwrap(),
        &mut std::collections::HashSet::new(),
    );
}

#[test]
fn object_property_names_pattern_is_enforced_by_resolved_schema_evaluator() {
    let schema = ResolvedSchema::from_json(&json!({
        "propertyNames": {
            "pattern": "^a+$"
        }
    }))
    .unwrap();

    assert!(schema.is_valid(&json!({})).unwrap());
    assert!(schema.root().unwrap().accepts_value(&json!({})));
    assert!(schema.is_valid(&json!({ "a": {} })).unwrap());
    assert!(schema.root().unwrap().accepts_value(&json!({ "a": {} })));
    assert!(!schema.is_valid(&json!({ "9DsHx": {} })).unwrap());
    assert!(
        !schema
            .root()
            .unwrap()
            .accepts_value(&json!({ "9DsHx": {} }))
    );
}

#[test]
fn rejects_non_local_ref_with_explicit_unsupported_reference_error() {
    let schema = ResolvedSchema::from_json(&json!({
        "$ref": "https://example.com/schemas/other.json"
    }))
    .unwrap();
    let error = schema.root().unwrap_err();

    assert!(matches!(
        error,
        AstError::UnsupportedReference { ref_path }
            if ref_path == "https://example.com/schemas/other.json"
    ));
}

#[test]
fn rejects_anchor_and_dynamic_ref_keywords_with_explicit_unsupported_reference_error() {
    for raw in [
        json!({
            "$anchor": "node",
            "type": "string"
        }),
        json!({
            "$dynamicRef": "#node"
        }),
        json!({
            "$id": "https://example.com/schemas/node.json",
            "type": "string"
        }),
    ] {
        let schema = ResolvedSchema::from_json(&raw).unwrap();
        let error = schema.root().unwrap_err();
        assert!(matches!(error, AstError::UnsupportedReference { .. }));
    }
}

#[test]
fn raw_validation_does_not_force_canonicalization_or_resolution() {
    let schema = ResolvedSchema::from_json(&json!({
        "$id": "https://example.com/schemas/node.json",
        "type": "string"
    }))
    .unwrap();

    assert!(schema.is_valid(&json!("value")).unwrap());
    assert!(!schema.is_valid(&json!(123)).unwrap());

    let error = schema.root().unwrap_err();
    assert!(matches!(error, AstError::UnsupportedReference { .. }));
}

#[test]
fn rejects_missing_local_ref_with_explicit_unresolved_reference_error() {
    let schema = ResolvedSchema::from_json(&json!({
        "$ref": "#/$defs/Missing"
    }))
    .unwrap();
    let error = schema.root().unwrap_err();

    assert!(matches!(
        error,
        AstError::UnresolvedReference { ref_path }
            if ref_path == "#/$defs/Missing"
    ));
}

fn build_schema(raw: &Value) -> SchemaNode {
    ResolvedSchema::from_json(raw)
        .unwrap()
        .root()
        .unwrap()
        .clone()
}

fn compile_ast(ast: &SchemaNode) -> schema::JSONSchema {
    compile(&ast.to_json()).unwrap()
}

fn assert_resolved_graph_is_public(
    node: &SchemaNode,
    seen: &mut std::collections::HashSet<schema::ResolvedNodeId>,
) {
    if !seen.insert(node.id()) {
        return;
    }

    match node.kind() {
        SchemaNodeKind::BoolSchema(_)
        | SchemaNodeKind::Any
        | SchemaNodeKind::String { .. }
        | SchemaNodeKind::Number { .. }
        | SchemaNodeKind::Integer { .. }
        | SchemaNodeKind::Boolean { .. }
        | SchemaNodeKind::Null { .. }
        | SchemaNodeKind::Const(_)
        | SchemaNodeKind::Enum(_) => {}
        SchemaNodeKind::Object {
            properties,
            pattern_properties,
            additional,
            property_names,
            ..
        } => {
            for child in properties
                .values()
                .chain(pattern_properties.values())
                .chain(std::iter::once(additional))
                .chain(std::iter::once(property_names))
            {
                assert_resolved_graph_is_public(child, seen);
            }
        }
        SchemaNodeKind::Array {
            prefix_items,
            items,
            contains,
            ..
        } => {
            for child in prefix_items {
                assert_resolved_graph_is_public(child, seen);
            }
            assert_resolved_graph_is_public(items, seen);
            if let Some(contains) = contains {
                assert_resolved_graph_is_public(&contains.schema, seen);
            }
        }
        SchemaNodeKind::AllOf(children)
        | SchemaNodeKind::AnyOf(children)
        | SchemaNodeKind::OneOf(children) => {
            for child in children {
                assert_resolved_graph_is_public(child, seen);
            }
        }
        SchemaNodeKind::Not(child) => {
            assert_resolved_graph_is_public(child, seen);
        }
        SchemaNodeKind::IfThenElse {
            if_schema,
            then_schema,
            else_schema,
        } => {
            assert_resolved_graph_is_public(if_schema, seen);
            if let Some(child) = then_schema {
                assert_resolved_graph_is_public(child, seen);
            }
            if let Some(child) = else_schema {
                assert_resolved_graph_is_public(child, seen);
            }
        }
        other => panic!("unexpected unresolved node kind in public graph: {other:?}"),
    }
}
