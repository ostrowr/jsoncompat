use jsoncompat_openapi::{
    OpenApiDocument, OpenApiOperationLowerer, ParameterEncoding, ParameterStyle,
};
use serde_json::{Value, json};

fn spec(media: Value) -> Value {
    json!({
        "openapi": "3.1.0", "info": {"title": "Uploads", "version": "1"},
        "paths": {"/upload": {"post": {
            "requestBody": {"required": true, "content": {"multipart/form-data": media}},
            "responses": {"204": {"description": "Uploaded"}}
        }}},
        "components": {"headers": {"RequestId": {
            "required": true, "schema": {"type": "string", "minLength": 1}
        }}}
    })
}

#[test]
fn operation_contract_preserves_multipart_encoding_and_header_constraints() {
    let raw = spec(json!({
        "schema": {"type": "object", "properties": {
            "secrets": {"type": "object"}, "file": {"type": "string"},
            "tags": {"type": "array", "items": {"type": "string"}}
        }},
        "encoding": {
            "secrets": {"contentType": "application/json"},
            "file": {"contentType": "image/png", "headers": {
                "X-Request-Id": {"$ref": "#/components/headers/RequestId"},
                "Content-Type": {"schema": {"type": "integer"}}
            }},
            "tags": {"style": "pipeDelimited", "explode": false, "allowReserved": true}
        }
    }));
    let document = OpenApiDocument::from_json(&raw).unwrap();
    let lowerer = OpenApiOperationLowerer::new(&document).unwrap();
    let key = lowerer.operation_keys().unwrap().pop_first().unwrap();
    let contract = lowerer.operation_contract(&key).unwrap().unwrap();
    let body = contract.body.unwrap();
    assert!(body.required);
    let encoding = body.content[0].encoding.as_ref().unwrap();
    let secrets = &encoding["secrets"];
    assert_eq!(secrets.content_type.as_deref(), Some("application/json"));
    assert_eq!(secrets.style, ParameterStyle::Form);
    assert!(secrets.explode);
    assert!(!secrets.allow_reserved);
    assert!(!secrets.uses_parameter_encoding);
    let file = &encoding["file"];
    assert_eq!(file.content_type.as_deref(), Some("image/png"));
    assert_eq!(file.headers.len(), 1);
    assert_eq!(file.headers[0].name, "x-request-id");
    assert!(file.headers[0].required);
    assert_eq!(file.headers[0].schema["minLength"], 1);
    assert!(matches!(
        file.headers[0].encoding,
        ParameterEncoding::Schema(_)
    ));
    let tags = &encoding["tags"];
    assert_eq!(tags.style, ParameterStyle::PipeDelimited);
    assert!(!tags.explode);
    assert!(tags.allow_reserved);
    assert!(tags.uses_parameter_encoding);

    // This is codegen metadata, not a proof that two encodings have equal bytes.
    let error = lowerer.lower_operation(&key).unwrap_err().to_string();
    assert!(error.contains("media-type encoding"), "{error}");
}

#[test]
fn explicit_encoding_controls_remain_distinct_from_defaults() {
    for control in [
        json!({}),
        json!({"style": "form"}),
        json!({"explode": true}),
        json!({"allowReserved": false}),
    ] {
        let raw = spec(json!({
            "schema": {"type": "object", "properties": {"payload": {"type": "object"}}},
            "encoding": {"payload": control}
        }));
        let document = OpenApiDocument::from_json(&raw).unwrap();
        let lowerer = OpenApiOperationLowerer::new(&document).unwrap();
        let key = lowerer.operation_keys().unwrap().pop_first().unwrap();
        let body = lowerer
            .operation_contract(&key)
            .unwrap()
            .unwrap()
            .body
            .unwrap();
        let entry = &body.content[0].encoding.as_ref().unwrap()["payload"];
        assert_eq!(entry.uses_parameter_encoding, control != json!({}));
        assert_eq!(entry.style, ParameterStyle::Form);
        assert!(entry.explode);
        assert!(!entry.allow_reserved);
    }
}

#[test]
fn preserving_encodings_does_not_accept_invalid_or_undefined_metadata() {
    for encoding in [
        json!({"absent": {}}),
        json!({"payload": {"explode": "true"}}),
    ] {
        let error = OpenApiDocument::from_json(&spec(json!({
            "schema": {"type": "object", "properties": {"payload": {"type": "object"}}},
            "encoding": encoding
        })))
        .unwrap_err()
        .to_string();
        assert!(error.contains("encoding"), "{error}");
    }
    let raw = spec(json!({
        "schema": {"type": "object", "properties": {"payload": {"type": "object"}}},
        "encoding": {"payload": {"style": "deepObject", "explode": false}}
    }));
    let document = OpenApiDocument::from_json(&raw).unwrap();
    let lowerer = OpenApiOperationLowerer::new(&document).unwrap();
    let key = lowerer.operation_keys().unwrap().pop_first().unwrap();
    let error = lowerer.operation_contract(&key).unwrap_err().to_string();
    assert!(error.contains("explode"), "{error}");
}
