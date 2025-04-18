use json_schema_draft2020 as schema;

use schema::{build_and_resolve_schema, build_schema_ast, compile, SchemaNode};
use serde_json::json;
use url::Url;

#[test]
fn roundtrip_compile_validate() {
    let raw = json!({"type":"string", "minLength":3});
    let base = Url::parse("file:///test.json").unwrap();
    let ast = build_and_resolve_schema(&raw, &base).unwrap();
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
    let base = Url::parse("file:///ref.json").unwrap();
    let mut ast = build_schema_ast(&raw).unwrap();
    schema::resolve_refs(&mut ast, &raw, &base, &[]).unwrap();
    assert!(matches!(ast, SchemaNode::Integer { .. }));
}

#[test]
fn boolean_schemas() {
    let base = Url::parse("file:///x.json").unwrap();
    let sample = json!({"k":1});
    for b in [true, false] {
        let raw = json!(b);
        let ast = build_and_resolve_schema(&raw, &base).unwrap();
        let compiled = compile(&ast.to_json()).unwrap();
        assert_eq!(compiled.is_valid(&sample), b);
    }
}
