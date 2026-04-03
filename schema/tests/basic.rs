use json_schema_ast as schema;

use schema::{
    SchemaNode, SchemaNodeKind, build_and_resolve_canonical_schema, build_canonical_schema_ast,
    canonicalize_schema, compile_canonical,
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
fn resolve_local_ref() {
    let raw = json!({
        "definitions": {"Int": {"type":"integer"}},
        "$ref": "#/definitions/Int"
    });
    let canonical = canonicalize_schema(&raw).unwrap();
    let mut ast = build_canonical_schema_ast(&canonical).unwrap();
    schema::resolve_refs(&mut ast, &canonical, &[]).unwrap();
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
    let canonical = canonicalize_schema(&raw).unwrap();
    let mut ast = build_canonical_schema_ast(&canonical).unwrap();
    schema::resolve_refs(&mut ast, &canonical, &[]).unwrap();
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
    let canonical = canonicalize_schema(&raw).unwrap();
    let mut ast = build_canonical_schema_ast(&canonical).unwrap();
    schema::resolve_refs(&mut ast, &canonical, &[]).unwrap();

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

fn build_schema(raw: &Value) -> SchemaNode {
    let schema = canonicalize_schema(raw).unwrap();
    build_and_resolve_canonical_schema(&schema).unwrap()
}

fn compile_ast(ast: &SchemaNode) -> schema::JSONSchema {
    let schema = canonicalize_schema(&ast.to_json()).unwrap();
    compile_canonical(&schema).unwrap()
}
