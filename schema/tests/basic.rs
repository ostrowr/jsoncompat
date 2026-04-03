use json_schema_ast as schema;

use schema::{SchemaNode, SchemaNodeKind, build_and_resolve_schema, compile};
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
fn resolve_local_ref() {
    let raw = json!({
        "definitions": {"Int": {"type":"integer"}},
        "$ref": "#/definitions/Int"
    });
    let ast = build_and_resolve_schema(&raw).unwrap();
    assert!(matches!(&*ast.borrow(), SchemaNodeKind::Integer { .. }));
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
        let guard = ast.borrow();
        if let SchemaNodeKind::Object { properties, .. } = &*guard {
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

    let guard = ast.borrow();
    if let SchemaNodeKind::Object { properties, .. } = &*guard {
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
    let guard = ast.borrow();
    assert!(matches!(&*guard, SchemaNodeKind::AllOf(children) if children.len() == 1));
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

    let guard = ast.borrow();
    let SchemaNodeKind::Number {
        minimum,
        enumeration,
        ..
    } = &*guard
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

    let guard = ast.borrow();
    let SchemaNodeKind::Enum(values) = &*guard else {
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

    let guard = ast.borrow();
    let SchemaNodeKind::Enum(values) = &*guard else {
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

    let guard = ast.borrow();
    let SchemaNodeKind::Enum(values) = &*guard else {
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

    let guard = ast.borrow();
    let SchemaNodeKind::Enum(values) = &*guard else {
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

    let guard = ast.borrow();
    let SchemaNodeKind::String {
        pattern,
        enumeration,
        ..
    } = &*guard
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

    let guard = ast.borrow();
    let SchemaNodeKind::Object { properties, .. } = &*guard else {
        panic!("expected object schema, got {guard:?}");
    };

    let email = properties.get("email").expect("email property");
    let email_guard = email.borrow();
    let SchemaNodeKind::AnyOf(branches) = &*email_guard else {
        panic!("expected implicit union for email property, got {email_guard:?}");
    };

    assert!(branches.iter().any(|branch| {
        matches!(
            &*branch.borrow(),
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

    let guard = ast.borrow();
    let SchemaNodeKind::IfThenElse {
        then_schema,
        else_schema,
        ..
    } = &*guard
    else {
        panic!("expected conditional schema, got {guard:?}");
    };

    let then_schema = then_schema.as_ref().expect("then schema");
    assert!(matches!(
        &*then_schema.borrow(),
        SchemaNodeKind::AnyOf(branches)
            if branches
                .iter()
                .any(|branch| matches!(&*branch.borrow(), SchemaNodeKind::Number { .. }))
    ));
    let else_schema = else_schema.as_ref().expect("else schema");
    assert!(matches!(
        &*else_schema.borrow(),
        SchemaNodeKind::String { .. }
    ));
}

#[test]
fn metadata_only_enum_wrapper_uses_terminal_enum_shape() {
    let ast = build_schema(&json!({
        "$id": "https://example.com/enums/answer",
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

    let guard = ast.borrow();
    let SchemaNodeKind::Enum(values) = &*guard else {
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

    assert!(matches!(&*ast.borrow(), SchemaNodeKind::String { .. }));
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

    assert!(matches!(&*ast.borrow(), SchemaNodeKind::String { .. }));
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

    assert!(matches!(&*ast.borrow(), SchemaNodeKind::String { .. }));
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

    let guard = ast.borrow();
    let SchemaNodeKind::IfThenElse {
        if_schema,
        then_schema: Some(then_schema),
        else_schema: None,
    } = &*guard
    else {
        panic!("expected normalized conditional schema, got {guard:?}");
    };

    assert!(matches!(
        &*if_schema.borrow(),
        SchemaNodeKind::BoolSchema(false)
    ));
    assert!(matches!(
        &*then_schema.borrow(),
        SchemaNodeKind::BoolSchema(false)
    ));

    drop(guard);
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

    assert!(matches!(&*ast.borrow(), SchemaNodeKind::String { .. }));
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

    assert!(matches!(&*ast.borrow(), SchemaNodeKind::String { .. }));
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
        assert!(matches!(&*ast.borrow(), SchemaNodeKind::String { .. }));
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
        &*ast.borrow(),
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

    assert!(matches!(&*ast.borrow(), SchemaNodeKind::String { .. }));
}

fn build_schema(raw: &Value) -> SchemaNode {
    build_and_resolve_schema(raw).unwrap()
}

fn compile_ast(ast: &SchemaNode) -> schema::JSONSchema {
    compile(&ast.to_json()).unwrap()
}
