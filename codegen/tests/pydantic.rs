use json_schema_ast::build_and_resolve_schema;
use json_schema_codegen::{pydantic, ModelRole, PydanticOptions};
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

    let _schema = build_and_resolve_schema(&schema_json).expect("schema build failed");
    let options = PydanticOptions::default()
        .with_root_model_name("User")
        .with_base_module("json_schema_codegen_base");

    let serializer_code =
        pydantic::generate_model_from_value(&schema_json, ModelRole::Serializer, options.clone())
            .expect("serializer codegen failed");
    let deserializer_code =
        pydantic::generate_model_from_value(&schema_json, ModelRole::Deserializer, options)
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

    let options = PydanticOptions::default().with_root_model_name("Metadata");

    build_and_resolve_schema(&schema_json).expect("schema build failed");

    let code = pydantic::generate_model_from_value(&schema_json, ModelRole::Serializer, options)
        .expect("codegen failed");

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

    build_and_resolve_schema(&schema_json).expect("schema build failed");
    let code = pydantic::generate_model_from_value(
        &schema_json,
        ModelRole::Serializer,
        PydanticOptions::default().with_root_model_name("Config"),
    )
    .expect("enum object should be supported");

    assert!(
        code.contains("_validate_literal"),
        "expected literal helper to be emitted for object enum"
    );
}

#[test]
fn object_keywords_without_type_allow_non_objects() {
    let schema_json = json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "minProperties": 1
    });

    build_and_resolve_schema(&schema_json).expect("schema build failed");
    let code = pydantic::generate_model_from_value(
        &schema_json,
        ModelRole::Serializer,
        PydanticOptions::default().with_root_model_name("Root"),
    )
    .expect("codegen failed");
    assert!(
        !code.contains("_allow_non_objects"),
        "non-object bypass validator should not be emitted"
    );
}

#[test]
fn root_model_generated_for_primitives() {
    let schema_json = json!({
        "type": "string",
        "minLength": 3
    });

    build_and_resolve_schema(&schema_json).expect("schema build failed");
    let code = pydantic::generate_model_from_value(
        &schema_json,
        ModelRole::Serializer,
        PydanticOptions::default().with_root_model_name("Root"),
    )
    .expect("codegen failed");

    assert!(code.contains("class RootSerializer(SerializerRootModel):"));
    assert!(code.contains("root: Annotated[str, Field(min_length=3)]"));
}
