use json_schema_ast::build_and_resolve_schema;
use json_schema_codegen::{build_model_graph, pydantic, ModelRole, PydanticOptions};
use serde_json::json;

#[test]
fn pydantic_defaults_and_aliases() {
    let schema_json = json!({
        "type": "object",
        "properties": {
            "id": { "type": "integer" },
            "display-name": { "type": "string", "minLength": 1 },
            "age": { "type": "integer", "default": 42 },
            "nickname": { "type": "string" }
        },
        "required": ["id"]
    });

    let schema = build_and_resolve_schema(&schema_json).expect("schema build failed");
    let options = PydanticOptions::default()
        .with_root_model_name("User")
        .with_base_module("json_schema_codegen_base");

    let serializer_code = pydantic::generate_model(&schema, ModelRole::Serializer, options.clone())
        .expect("serializer codegen failed");
    let deserializer_code = pydantic::generate_model(&schema, ModelRole::Deserializer, options)
        .expect("deserializer codegen failed");

    assert!(serializer_code.contains("json_schema_codegen_base"));
    assert!(serializer_code.contains("SerializerBase"));
    assert!(deserializer_code.contains("json_schema_codegen_base"));
    assert!(deserializer_code.contains("DeserializerBase"));
    assert!(serializer_code.contains("class UserSerializer"));
    assert!(deserializer_code.contains("class UserDeserializer"));
    assert!(serializer_code.contains("alias=\"display-name\""));
    assert!(deserializer_code.contains("alias=\"display-name\""));
    assert!(!serializer_code.contains("default=42"));
    assert!(deserializer_code.contains("default=42"));
}

#[test]
fn pydantic_typed_additional_properties() {
    let schema_json = json!({
        "type": "object",
        "additionalProperties": { "type": "string" }
    });

    let schema = build_and_resolve_schema(&schema_json).expect("schema build failed");
    let options = PydanticOptions::default().with_root_model_name("Metadata");

    let code =
        pydantic::generate_model(&schema, ModelRole::Serializer, options).expect("codegen failed");

    assert!(code.contains("__pydantic_extra__: dict[str, str]"));
    assert!(code.contains("model_config = ConfigDict(extra=\"allow\")"));
}

#[test]
fn rejects_enum_objects() {
    let schema_json = json!({
        "type": "object",
        "properties": {
            "config": {
                "enum": [ { "mode": "fast" } ]
            }
        }
    });

    let schema = build_and_resolve_schema(&schema_json).expect("schema build failed");
    let err = pydantic::generate_model(
        &schema,
        ModelRole::Serializer,
        PydanticOptions::default().with_root_model_name("Config"),
    )
    .expect_err("expected enum object to be rejected");

    let message = err.to_string();
    assert!(message.contains("unsupported enum/const value"));
}

#[test]
fn object_keywords_without_type_allow_non_objects() {
    let schema_json = json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "minProperties": 1
    });

    let schema = build_and_resolve_schema(&schema_json).expect("schema build failed");
    let graph = build_model_graph(&schema, "Root").expect("graph build failed");
    let root = graph
        .models
        .get(&graph.root)
        .expect("root model should exist");
    assert!(
        root.allow_non_objects,
        "expected non-object inputs to be allowed"
    );
}
