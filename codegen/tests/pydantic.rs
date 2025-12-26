use json_schema_codegen::{generate_pydantic, PydanticOptions};
use serde_json::json;

#[test]
fn pydantic_defaults_and_aliases() {
    let schema = json!({
        "type": "object",
        "properties": {
            "id": { "type": "integer" },
            "display-name": { "type": "string", "minLength": 1 },
            "age": { "type": "integer", "default": 42 },
            "nickname": { "type": "string" }
        },
        "required": ["id"]
    });

    let code =
        generate_pydantic(&schema, "User", PydanticOptions::default()).expect("codegen failed");

    assert!(code.contains("class UserSerializer"));
    assert!(code.contains("class UserDeserializer"));
    assert!(code.contains("alias=\"display-name\""));
    assert!(code.contains("kwargs.setdefault(\"exclude_unset\", True)"));

    let serializer_pos = code.find("class UserSerializer").unwrap();
    let deserializer_pos = code.find("class UserDeserializer").unwrap();
    let default_positions: Vec<_> = code.match_indices("default=42").collect();
    assert_eq!(default_positions.len(), 1);
    assert!(default_positions[0].0 > deserializer_pos);
    assert!(deserializer_pos > serializer_pos);
}

#[test]
fn pydantic_typed_additional_properties() {
    let schema = json!({
        "type": "object",
        "additionalProperties": { "type": "string" }
    });

    let code =
        generate_pydantic(&schema, "Metadata", PydanticOptions::default()).expect("codegen failed");

    assert!(code.contains("__pydantic_extra__: dict[str, str]"));
    assert!(code.contains("model_config = ConfigDict(extra=\"allow\")"));
}

#[test]
fn rejects_enum_objects() {
    let schema = json!({
        "type": "object",
        "properties": {
            "config": {
                "enum": [ { "mode": "fast" } ]
            }
        }
    });

    let err = generate_pydantic(&schema, "Config", PydanticOptions::default())
        .expect_err("expected enum object to be rejected");

    let message = err.to_string();
    assert!(message.contains("unsupported enum/const value"));
}
