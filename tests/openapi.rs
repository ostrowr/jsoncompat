use jsoncompat::{OpenApiCompatibilitySurface, OpenApiDocument, check_openapi_compat};
use serde_json::{Value, json};

fn document(raw: Value) -> OpenApiDocument {
    OpenApiDocument::from_json(&raw).expect("OpenAPI document should parse")
}

fn report(old: Value, new: Value) -> jsoncompat::OpenApiCompatibilityReport {
    check_openapi_compat(&document(old), &document(new))
        .expect("OpenAPI compatibility check should run")
}

fn compat_error(old: Value, new: Value) -> String {
    check_openapi_compat(&document(old), &document(new))
        .expect_err("OpenAPI compatibility check should fail")
        .to_string()
}

fn response_schema(schema: Value) -> Value {
    json!({
        "description": "ok",
        "content": {
            "application/json": {
                "schema": schema
            }
        }
    })
}

fn spec(operation: Value) -> Value {
    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "paths": {
            "/pets": {
                "get": operation
            }
        }
    })
}

fn get_operation() -> Value {
    json!({
        "responses": {
            "200": response_schema(json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer" }
                },
                "required": ["id"]
            }))
        }
    })
}

fn issue_surfaces(
    report: &jsoncompat::OpenApiCompatibilityReport,
) -> Vec<OpenApiCompatibilitySurface> {
    report.issues().iter().map(|issue| issue.surface).collect()
}

#[test]
fn identical_openapi_documents_are_compatible() {
    let spec = spec(get_operation());
    let report = report(spec.clone(), spec);

    assert!(report.is_compatible());
    assert!(report.issues().is_empty());
}

#[test]
fn adding_a_required_query_parameter_is_incompatible() {
    let old = spec(get_operation());
    let new = spec(json!({
        "parameters": [{
            "name": "limit",
            "in": "query",
            "required": true,
            "schema": { "type": "integer" }
        }],
        "responses": {
            "200": response_schema(json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer" }
                },
                "required": ["id"]
            }))
        }
    }));

    let report = report(old, new);

    assert!(!report.is_compatible());
    assert_eq!(
        issue_surfaces(&report),
        vec![OpenApiCompatibilitySurface::Request]
    );
}

#[test]
fn adding_an_optional_query_parameter_is_compatible() {
    let old = spec(get_operation());
    let new = spec(json!({
        "parameters": [{
            "name": "limit",
            "in": "query",
            "schema": { "type": "integer" }
        }],
        "responses": {
            "200": response_schema(json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer" }
                },
                "required": ["id"]
            }))
        }
    }));

    assert!(report(old, new).is_compatible());
}

#[test]
fn removing_a_query_parameter_is_incompatible() {
    let old = spec(json!({
        "parameters": [{
            "name": "limit",
            "in": "query",
            "schema": { "type": "integer" }
        }],
        "responses": {
            "200": response_schema(json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer" }
                },
                "required": ["id"]
            }))
        }
    }));
    let new = spec(get_operation());

    let report = report(old, new);

    assert!(!report.is_compatible());
    assert_eq!(
        issue_surfaces(&report),
        vec![OpenApiCompatibilitySurface::Request]
    );
}

#[test]
fn making_a_request_body_required_is_incompatible() {
    let old = spec(json!({
        "requestBody": {
            "content": {
                "application/json": {
                    "schema": { "type": "object" }
                }
            }
        },
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    }));
    let new = spec(json!({
        "requestBody": {
            "required": true,
            "content": {
                "application/json": {
                    "schema": { "type": "object" }
                }
            }
        },
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    }));

    let report = report(old, new);

    assert!(!report.is_compatible());
    assert_eq!(
        issue_surfaces(&report),
        vec![OpenApiCompatibilitySurface::Request]
    );
}

#[test]
fn removing_a_supported_request_media_type_is_incompatible() {
    let old = spec(json!({
        "requestBody": {
            "required": true,
            "content": {
                "application/json": {
                    "schema": { "type": "object" }
                },
                "application/problem+json": {
                    "schema": { "type": "object" }
                }
            }
        },
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    }));
    let new = spec(json!({
        "requestBody": {
            "required": true,
            "content": {
                "application/json": {
                    "schema": { "type": "object" }
                }
            }
        },
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    }));

    assert!(!report(old, new).is_compatible());
}

#[test]
fn broadening_a_response_body_schema_is_incompatible() {
    let old = spec(json!({
        "responses": {
            "200": response_schema(json!({
                "type": "object",
                "properties": {
                    "status": { "type": "string" }
                },
                "required": ["status"]
            }))
        }
    }));
    let new = spec(json!({
        "responses": {
            "200": response_schema(json!({
                "type": "object",
                "properties": {
                    "status": { "type": ["string", "null"] }
                },
                "required": ["status"]
            }))
        }
    }));

    let report = report(old, new);

    assert!(!report.is_compatible());
    assert_eq!(
        issue_surfaces(&report),
        vec![OpenApiCompatibilitySurface::Response]
    );
    assert!(
        report.issues()[0].message.contains("property 'status'"),
        "unexpected explanation: {:?}",
        report.issues()[0]
    );
}

#[test]
fn broadening_a_response_header_schema_is_incompatible() {
    let old = spec(json!({
        "responses": {
            "200": {
                "description": "ok",
                "headers": {
                    "X-Rate-Limit": {
                        "schema": { "type": "integer" }
                    }
                }
            }
        }
    }));
    let new = spec(json!({
        "responses": {
            "200": {
                "description": "ok",
                "headers": {
                    "x-rate-limit": {
                        "schema": { "type": ["integer", "string"] }
                    }
                }
            }
        }
    }));

    let report = report(old, new);

    assert!(!report.is_compatible());
    assert_eq!(
        issue_surfaces(&report),
        vec![OpenApiCompatibilitySurface::Response]
    );
    assert!(
        report.issues()[0]
            .message
            .contains("properties/headers/properties/x-rate-limit"),
        "unexpected explanation: {:?}",
        report.issues()[0]
    );
}

#[test]
fn making_a_required_response_header_optional_is_incompatible() {
    let old = spec(json!({
        "responses": {
            "200": {
                "description": "ok",
                "headers": {
                    "X-Trace-Id": {
                        "required": true,
                        "schema": { "type": "string" }
                    }
                }
            }
        }
    }));
    let new = spec(json!({
        "responses": {
            "200": {
                "description": "ok",
                "headers": {
                    "X-Trace-Id": {
                        "schema": { "type": "string" }
                    }
                }
            }
        }
    }));

    let report = report(old, new);

    assert!(!report.is_compatible());
    assert_eq!(
        issue_surfaces(&report),
        vec![OpenApiCompatibilitySurface::Response]
    );
}

#[test]
fn removing_allow_empty_value_from_a_query_parameter_is_incompatible() {
    let old = spec(json!({
        "parameters": [{
            "name": "filter",
            "in": "query",
            "allowEmptyValue": true,
            "schema": { "type": "string" }
        }],
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    }));
    let new = spec(json!({
        "parameters": [{
            "name": "filter",
            "in": "query",
            "schema": { "type": "string" }
        }],
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    }));

    let report = report(old, new);

    assert!(!report.is_compatible());
    assert_eq!(
        issue_surfaces(&report),
        vec![OpenApiCompatibilitySurface::Request]
    );
}

#[test]
fn response_header_refs_are_resolved_before_lowering() {
    let spec = json!({
        "openapi": "3.1.0",
        "info": { "title": "Headers", "version": "1.0.0" },
        "components": {
            "headers": {
                "TraceId": {
                    "required": true,
                    "schema": { "type": "string" }
                }
            }
        },
        "paths": {
            "/pets": {
                "get": {
                    "responses": {
                        "200": {
                            "description": "ok",
                            "headers": {
                                "X-Trace-Id": { "$ref": "#/components/headers/TraceId" }
                            }
                        }
                    }
                }
            }
        }
    });

    assert!(report(spec.clone(), spec).is_compatible());
}

#[test]
fn content_based_response_headers_remain_compatible_when_unchanged() {
    let spec = json!({
        "openapi": "3.1.0",
        "info": { "title": "Headers", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "get": {
                    "responses": {
                        "200": {
                            "description": "ok",
                            "headers": {
                                "X-Meta": {
                                    "content": {
                                        "application/json": {
                                            "schema": {
                                                "type": "object",
                                                "properties": {
                                                    "id": { "type": "integer" }
                                                },
                                                "required": ["id"]
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    assert!(report(spec.clone(), spec).is_compatible());
}

#[test]
fn response_headers_reject_allow_reserved() {
    let error = compat_error(
        spec(json!({
            "responses": {
                "200": {
                    "description": "ok",
                    "headers": {
                        "X-Bad": {
                            "allowReserved": true,
                            "schema": { "type": "string" }
                        }
                    }
                }
            }
        })),
        spec(get_operation()),
    );

    assert!(
        error.contains("allowReserved") && error.contains("not present for response headers"),
        "{error}"
    );
}

#[test]
fn response_headers_reject_non_simple_style() {
    let error = compat_error(
        spec(json!({
            "responses": {
                "200": {
                    "description": "ok",
                    "headers": {
                        "X-Bad": {
                            "style": "form",
                            "schema": { "type": "string" }
                        }
                    }
                }
            }
        })),
        spec(get_operation()),
    );

    assert!(
        error.contains("style") && error.contains("'simple' for response headers"),
        "{error}"
    );
}

#[test]
fn non_query_parameters_reject_query_only_metadata() {
    let error = compat_error(
        spec(json!({
            "parameters": [{
                "name": "X-Bad",
                "in": "header",
                "allowEmptyValue": true,
                "schema": { "type": "string" }
            }],
            "responses": {
                "200": response_schema(json!({ "type": "object" }))
            }
        })),
        spec(get_operation()),
    );

    assert!(
        error.contains("allowEmptyValue") && error.contains("a query parameter field"),
        "{error}"
    );
}

#[test]
fn adding_a_response_media_type_is_incompatible() {
    let old = spec(json!({
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    }));
    let new = spec(json!({
        "responses": {
            "200": {
                "description": "ok",
                "content": {
                    "application/json": {
                        "schema": { "type": "object" }
                    },
                    "application/problem+json": {
                        "schema": { "type": "object" }
                    }
                }
            }
        }
    }));

    assert!(!report(old, new).is_compatible());
}

#[test]
fn removing_an_operation_is_incompatible() {
    let old = spec(get_operation());
    let new = json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "paths": {}
    });

    let report = report(old, new);

    assert!(!report.is_compatible());
    assert_eq!(
        issue_surfaces(&report),
        vec![OpenApiCompatibilitySurface::Operation]
    );
}

#[test]
fn local_component_refs_are_lowered_before_compatibility_checks() {
    let old = json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "components": {
            "parameters": {
                "Limit": {
                    "name": "limit",
                    "in": "query",
                    "schema": { "type": "integer" }
                },
                "LimitAlias": {
                    "$ref": "#/components/parameters/Limit"
                }
            },
            "schemas": {
                "Pet": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "integer" }
                    },
                    "required": ["id"]
                }
            },
            "responses": {
                "PetResponse": {
                    "description": "ok",
                    "content": {
                        "application/json": {
                            "schema": { "$ref": "#/components/schemas/Pet" }
                        }
                    }
                }
            }
        },
        "paths": {
            "/pets": {
                "get": {
                    "parameters": [{ "$ref": "#/components/parameters/LimitAlias" }],
                    "responses": {
                        "200": { "$ref": "#/components/responses/PetResponse" }
                    }
                }
            }
        }
    });
    let new = json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "components": {
            "parameters": {
                "Limit": {
                    "name": "limit",
                    "in": "query",
                    "required": true,
                    "schema": { "type": "integer" }
                },
                "LimitAlias": {
                    "$ref": "#/components/parameters/Limit"
                }
            },
            "schemas": {
                "Pet": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "integer" }
                    },
                    "required": ["id"]
                }
            },
            "responses": {
                "PetResponse": {
                    "description": "ok",
                    "content": {
                        "application/json": {
                            "schema": { "$ref": "#/components/schemas/Pet" }
                        }
                    }
                }
            }
        },
        "paths": {
            "/pets": {
                "get": {
                    "parameters": [{ "$ref": "#/components/parameters/LimitAlias" }],
                    "responses": {
                        "200": { "$ref": "#/components/responses/PetResponse" }
                    }
                }
            }
        }
    });

    assert!(!report(old, new).is_compatible());
}

#[test]
fn component_schema_properties_named_ref_are_accepted() {
    let spec = json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Refs",
            "version": "1.0.0"
        },
        "components": {
            "schemas": {
                "RefSchema-Input": {
                    "type": "object",
                    "properties": {
                        "$ref": {
                            "anyOf": [
                                { "type": "string" },
                                { "type": "null" }
                            ]
                        }
                    }
                }
            }
        },
        "paths": {
            "/refs": {
                "post": {
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/RefSchema-Input"
                                }
                            }
                        }
                    },
                    "responses": {
                        "204": { "description": "ok" }
                    }
                }
            }
        }
    });

    assert!(report(spec.clone(), spec).is_compatible());
}

#[test]
fn nested_oneof_anyof_response_changes_get_a_property_level_explanation() {
    let old = spec(json!({
        "responses": {
            "200": response_schema(json!({
                "oneOf": [
                    {
                        "type": "object",
                        "properties": {
                            "kind": { "const": "single" },
                            "preamble": {
                                "anyOf": [
                                    { "type": "string" },
                                    { "type": "null" }
                                ]
                            }
                        },
                        "required": ["kind"]
                    },
                    {
                        "type": "object",
                        "properties": {
                            "kind": { "const": "multi" }
                        },
                        "required": ["kind"]
                    }
                ]
            }))
        }
    }));
    let new = spec(json!({
        "responses": {
            "200": response_schema(json!({
                "oneOf": [
                    {
                        "type": "object",
                        "properties": {
                            "kind": { "const": "single" },
                            "preamble": {
                                "anyOf": [
                                    { "type": "object" },
                                    { "type": "null" }
                                ]
                            }
                        },
                        "required": ["kind"]
                    },
                    {
                        "type": "object",
                        "properties": {
                            "kind": { "const": "multi" }
                        },
                        "required": ["kind"]
                    }
                ]
            }))
        }
    }));

    let report = report(old, new);
    let message = &report.issues()[0].message;

    assert!(!report.is_compatible());
    assert!(message.contains("schema #/properties/body"), "{message}");
    assert!(message.contains("oneOf branch 1"), "{message}");
    assert!(message.contains("property 'preamble'"), "{message}");
    assert!(message.contains("objects"), "{message}");
    assert!(message.contains("strings"), "{message}");
}
