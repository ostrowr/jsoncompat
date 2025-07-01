use json_schema_ast as schema;

use schema::{build_and_resolve_schema, build_schema_ast, compile, SchemaNode};
use serde_json::json;

#[test]
fn roundtrip_compile_validate() {
    let raw = json!({"type":"string", "minLength":3});
    let ast = build_and_resolve_schema(&raw).unwrap();
    let compiled = compile(&ast.to_json()).unwrap();
    assert!(compiled.is_valid(&json!("abc")));
    assert!(!compiled.is_valid(&json!("ab")));
}

#[test]
fn resolve_local_ref() {
    let raw = json!({
        "definitions": {"Int": {"type":"integer"}},
        "$ref": "#/definitions/Int"
    });
    let mut ast = build_schema_ast(&raw).unwrap();
    schema::resolve_refs(&mut ast, &raw, &[]).unwrap();
    assert!(matches!(ast, SchemaNode::Integer { .. }));
}

#[test]
fn boolean_schemas() {
    let sample = json!({"k":1});
    for b in [true, false] {
        let raw = json!(b);
        let ast = build_and_resolve_schema(&raw).unwrap();
        let compiled = compile(&ast.to_json()).unwrap();
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
    let ast = build_and_resolve_schema(&raw).unwrap();
    let json = ast.to_json();
    let ast2 = build_and_resolve_schema(&json).unwrap();
    assert_eq!(ast, ast2);
}
