use jsoncompat::{
    OpenApiCompatibilitySurface, OpenApiDocument, check_openapi_compat,
    validate_openapi_compatibility_input,
};
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
fn adding_an_operation_is_compatible() {
    let old = spec(get_operation());
    let new = json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "paths": {
            "/pets": {
                "get": get_operation(),
                "post": {
                    "responses": {
                        "201": response_schema(json!({ "type": "object" }))
                    }
                }
            }
        }
    });

    assert!(report(old, new).is_compatible());
}

#[test]
fn openapi_documents_reject_invalid_added_operations_before_compatibility() {
    let invalid = json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "paths": {
            "/pets": {
                "get": get_operation(),
                "post": {
                    "responses": {
                        "201": response_schema(json!({ "type": 42 }))
                    }
                }
            }
        }
    });

    let error = OpenApiDocument::from_json(&invalid)
        .expect_err("invalid added operations must fail document validation")
        .to_string();
    assert!(
        error.contains("#/paths/~1pets/post/responses/201/content/application~1json/schema"),
        "{error}"
    );
    assert!(error.contains("type"), "{error}");
}

#[test]
fn compatibility_rejects_unsupported_features_in_added_operations() {
    let old = spec(get_operation());
    let new = json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "paths": {
            "/pets": {
                "get": get_operation(),
                "post": {
                    "responses": {
                        "201": response_schema(json!({
                            "type": "number",
                            "multipleOf": 0.5
                        }))
                    }
                }
            }
        }
    });

    let error = compat_error(old, new);
    assert!(
        error.contains("non-integral number multipleOf constraints are not supported"),
        "{error}"
    );
}

#[test]
fn openapi_documents_reject_unresolved_refs_in_added_operations_before_compatibility() {
    let invalid = json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "paths": {
            "/pets": {
                "get": get_operation(),
                "post": {
                    "responses": {
                        "201": response_schema(json!({
                            "$ref": "#/components/schemas/Missing"
                        }))
                    }
                }
            }
        }
    });

    let error = OpenApiDocument::from_json(&invalid)
        .expect_err("unresolved inline refs must fail document validation")
        .to_string();
    assert!(
        error.contains("#/paths/~1pets/post/responses/201/content/application~1json/schema"),
        "{error}"
    );
    assert!(error.contains("Missing"), "{error}");
    assert!(error.contains("does not resolve"), "{error}");
}

#[test]
fn openapi_documents_reject_invalid_unchanged_operations_before_compatibility() {
    let invalid = spec(json!({
        "responses": {
            "200": response_schema(json!({ "type": 42 }))
        }
    }));

    let error = OpenApiDocument::from_json(&invalid)
        .expect_err("invalid unchanged operations must fail document validation")
        .to_string();
    assert!(
        error.contains("#/paths/~1pets/get/responses/200/content/application~1json/schema"),
        "{error}"
    );
    assert!(error.contains("type"), "{error}");
}

#[test]
fn compatibility_rejects_unsupported_features_in_unchanged_operations() {
    let invalid = spec(json!({
        "responses": {
            "200": response_schema(json!({
                "type": "number",
                "multipleOf": 0.5
            }))
        }
    }));

    let error = compat_error(invalid.clone(), invalid);
    assert!(
        error.contains("non-integral number multipleOf constraints are not supported"),
        "{error}"
    );
}

#[test]
fn openapi_compatibility_input_validation_rejects_unsupported_operation_surfaces() {
    let document = document(spec(json!({
        "callbacks": {},
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    })));

    let error = validate_openapi_compatibility_input(&document)
        .expect_err("callbacks must fail before comparison")
        .to_string();

    assert!(error.contains("#/paths/~1pets/get/callbacks"), "{error}");
    assert!(
        error.contains("OpenAPI compatibility checks do not support operation callbacks"),
        "{error}"
    );
}

#[test]
fn openapi_documents_reject_invalid_removed_operations_before_compatibility() {
    let invalid = json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "paths": {
            "/pets": {
                "get": get_operation(),
                "post": {
                    "responses": {
                        "201": response_schema(json!({ "type": 42 }))
                    }
                }
            }
        }
    });

    let error = OpenApiDocument::from_json(&invalid)
        .expect_err("invalid removed operations must fail document validation")
        .to_string();
    assert!(
        error.contains("#/paths/~1pets/post/responses/201/content/application~1json/schema"),
        "{error}"
    );
    assert!(error.contains("type"), "{error}");
}

#[test]
fn openapi_documents_require_info_and_one_top_level_contract_surface() {
    let missing_info = json!({
        "openapi": "3.1.0",
        "paths": {}
    });
    let info_error = OpenApiDocument::from_json(&missing_info)
        .expect_err("OpenAPI document without info must fail")
        .to_string();
    assert!(info_error.contains("#/info"), "{info_error}");

    let missing_surface = json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        }
    });
    let surface_error = OpenApiDocument::from_json(&missing_surface)
        .expect_err("OpenAPI document without any top-level contract surface must fail")
        .to_string();
    assert!(
        surface_error.contains("paths")
            && surface_error.contains("components")
            && surface_error.contains("webhooks"),
        "{surface_error}"
    );
}

#[test]
fn openapi_documents_accept_components_without_paths() {
    let document = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "components": {
            "schemas": {
                "Pet": { "type": "object" }
            }
        }
    }))
    .expect("component-only OpenAPI documents are valid 3.1 inputs");

    validate_openapi_compatibility_input(&document)
        .expect("component-only OpenAPI documents lower to an empty operation set");
}

#[test]
fn openapi_documents_reject_malformed_31_version_strings() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.foo",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "paths": {}
    }))
    .expect_err("malformed 3.1-ish version strings must fail")
    .to_string();

    assert!(
        error.contains("unsupported OpenAPI version '3.1.foo'"),
        "{error}"
    );
}

#[test]
fn openapi_documents_reject_non_string_version_fields_with_a_precise_pointer() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": 31,
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {}
    }))
    .expect_err("OpenAPI version values must be strings")
    .to_string();

    assert!(error.contains("#/openapi"), "{error}");
    assert!(error.contains("a string"), "{error}");
}

#[test]
fn openapi_documents_accept_numeric_31_patch_versions() {
    OpenApiDocument::from_json(&json!({
        "openapi": "3.1.12",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "paths": {}
    }))
    .expect("numeric 3.1 patch versions should stay within the supported feature line");
}

#[test]
fn openapi_lowering_rejects_webhooks_until_they_are_compared() {
    let document = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "paths": {},
        "webhooks": {
            "petAdded": {}
        }
    }))
    .expect("webhook-bearing OpenAPI documents are valid before lowerability checks");
    let error = validate_openapi_compatibility_input(&document)
        .expect_err("webhooks must not be silently ignored by lowering")
        .to_string();

    assert!(error.contains("#/webhooks"), "{error}");
    assert!(
        error.contains("OpenAPI compatibility checks do not support webhooks"),
        "{error}"
    );
}

#[test]
fn openapi_documents_validate_webhook_response_shapes_before_reporting_webhooks_as_unsupported() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "webhooks": {
            "petAdded": {
                "post": {
                    "responses": {
                        "200": {
                            "content": {
                                "application/json": {
                                    "schema": { "type": "object" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }))
    .expect_err("invalid webhook responses must fail before lowerability checks")
    .to_string();

    assert!(
        error.contains("#/webhooks/petAdded/post/responses/200/description"),
        "{error}"
    );
    assert!(
        !error.contains("compatibility checks do not support webhooks"),
        "{error}"
    );
}

#[test]
fn openapi_documents_validate_webhook_request_body_shapes_before_reporting_webhooks_as_unsupported()
{
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "webhooks": {
            "petAdded": {
                "post": {
                    "requestBody": {
                        "description": "payload"
                    },
                    "responses": {
                        "200": response_schema(json!({ "type": "object" }))
                    }
                }
            }
        }
    }))
    .expect_err("invalid webhook request bodies must fail before lowerability checks")
    .to_string();

    assert!(
        error.contains("#/webhooks/petAdded/post/requestBody/content"),
        "{error}"
    );
    assert!(
        !error.contains("compatibility checks do not support webhooks"),
        "{error}"
    );
}

#[test]
fn openapi_documents_validate_webhook_parameter_shapes_before_reporting_webhooks_as_unsupported() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "webhooks": {
            "petAdded": {
                "post": {
                    "parameters": [{
                        "name": "trace",
                        "in": "header"
                    }],
                    "responses": {
                        "200": response_schema(json!({ "type": "object" }))
                    }
                }
            }
        }
    }))
    .expect_err("invalid webhook parameters must fail before lowerability checks")
    .to_string();

    assert!(
        error.contains("#/webhooks/petAdded/post/parameters/0"),
        "{error}"
    );
    assert!(
        error.contains("exactly one of `schema` or `content`"),
        "{error}"
    );
    assert!(
        !error.contains("compatibility checks do not support webhooks"),
        "{error}"
    );
}

#[test]
fn openapi_documents_validate_webhook_operation_ids_before_reporting_webhooks_as_unsupported() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "paths": {
            "/pets": {
                "get": {
                    "operationId": "sharedId",
                    "responses": {
                        "200": response_schema(json!({ "type": "object" }))
                    }
                }
            }
        },
        "webhooks": {
            "petAdded": {
                "post": {
                    "operationId": "sharedId",
                    "responses": {
                        "200": response_schema(json!({ "type": "object" }))
                    }
                }
            }
        }
    }))
    .expect_err("duplicate webhook operationIds must fail before lowerability checks")
    .to_string();

    assert!(
        error.contains("duplicate OpenAPI operationId 'sharedId'"),
        "{error}"
    );
    assert!(
        error.contains("#/webhooks/petAdded/post/operationId"),
        "{error}"
    );
}

#[test]
fn openapi_documents_validate_webhook_security_before_reporting_webhooks_as_unsupported() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "webhooks": {
            "petAdded": {
                "post": {
                    "security": [{ "MissingAuth": [] }],
                    "responses": {
                        "200": response_schema(json!({ "type": "object" }))
                    }
                }
            }
        },
        "components": {
            "securitySchemes": {
                "Bearer": {
                    "type": "http",
                    "scheme": "bearer"
                }
            }
        }
    }))
    .expect_err("invalid webhook security must fail before lowerability checks")
    .to_string();

    assert!(
        error.contains("#/webhooks/petAdded/post/security/0/MissingAuth")
            && error.contains("declared components.securitySchemes"),
        "{error}"
    );
}

#[test]
fn openapi_documents_validate_webhooks_container_shape_before_lowering() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "webhooks": []
    }))
    .expect_err("webhooks must still be a valid OpenAPI map")
    .to_string();

    assert!(error.contains("#/webhooks"), "{error}");
    assert!(error.contains("an object"), "{error}");
}

#[test]
fn openapi_documents_validate_webhook_entries_even_when_names_start_with_x_dash() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "webhooks": {
            "x-pet-added": []
        }
    }))
    .expect_err("webhook entry names beginning with x- are still webhook entries")
    .to_string();

    assert!(error.contains("#/webhooks/x-pet-added"), "{error}");
    assert!(error.contains("an object"), "{error}");
}

#[test]
fn openapi_documents_validate_contract_container_shapes_before_lowering() {
    for (document, pointer_fragment) in [
        (
            json!({
                "openapi": "3.1.0",
                "info": {
                    "title": "Pets",
                    "version": "1.0.0"
                },
                "paths": {
                    "/pets": []
                }
            }),
            "#/paths/~1pets",
        ),
        (
            json!({
                "openapi": "3.1.0",
                "info": {
                    "title": "Pets",
                    "version": "1.0.0"
                },
                "paths": {
                    "/pets": {
                        "parameters": {}
                    }
                }
            }),
            "#/paths/~1pets/parameters",
        ),
        (
            json!({
                "openapi": "3.1.0",
                "info": {
                    "title": "Pets",
                    "version": "1.0.0"
                },
                "paths": {
                    "/pets": {
                        "get": []
                    }
                }
            }),
            "#/paths/~1pets/get",
        ),
        (
            json!({
                "openapi": "3.1.0",
                "info": {
                    "title": "Pets",
                    "version": "1.0.0"
                },
                "paths": {
                    "/pets": {
                        "get": {
                            "responses": []
                        }
                    }
                }
            }),
            "#/paths/~1pets/get/responses",
        ),
        (
            json!({
                "openapi": "3.1.0",
                "info": {
                    "title": "Pets",
                    "version": "1.0.0"
                },
                "paths": {
                    "/pets": {
                        "get": {
                            "responses": {
                                "200": []
                            }
                        }
                    }
                }
            }),
            "#/paths/~1pets/get/responses/200",
        ),
        (
            json!({
                "openapi": "3.1.0",
                "info": {
                    "title": "Pets",
                    "version": "1.0.0"
                },
                "paths": {
                    "/pets": {
                        "get": {
                            "requestBody": {},
                            "responses": {
                                "200": response_schema(json!({ "type": "object" }))
                            }
                        }
                    }
                }
            }),
            "#/paths/~1pets/get/requestBody/content",
        ),
        (
            json!({
                "openapi": "3.1.0",
                "info": {
                    "title": "Pets",
                    "version": "1.0.0"
                },
                "paths": {
                    "/pets": {
                        "get": {
                            "parameters": [{
                                "name": "limit",
                                "in": "query"
                            }],
                            "responses": {
                                "200": response_schema(json!({ "type": "object" }))
                            }
                        }
                    }
                }
            }),
            "#/paths/~1pets/get/parameters/0",
        ),
        (
            json!({
                "openapi": "3.1.0",
                "info": {
                    "title": "Pets",
                    "version": "1.0.0"
                },
                "webhooks": {
                    "petAdded": []
                }
            }),
            "#/webhooks/petAdded",
        ),
        (
            json!({
                "openapi": "3.1.0",
                "info": {
                    "title": "Pets",
                    "version": "1.0.0"
                },
                "components": {
                    "callbacks": []
                }
            }),
            "#/components/callbacks",
        ),
    ] {
        let error = OpenApiDocument::from_json(&document)
            .expect_err("invalid OpenAPI contract containers must fail during document validation")
            .to_string();
        assert!(
            error.contains(pointer_fragment),
            "{pointer_fragment}: {error}"
        );
    }
}

#[test]
fn openapi_documents_accept_security_requirements_outside_the_contract_model() {
    OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "paths": {},
        "security": [{ "bearerAuth": [] }],
        "components": {
            "securitySchemes": {
                "bearerAuth": {
                    "type": "http",
                    "scheme": "bearer"
                }
            }
        }
    }))
    .expect("document-level security metadata is valid even though it is not lowered");
}

#[test]
fn openapi_documents_validate_root_metadata_shapes_before_lowering() {
    for (field, value) in [
        ("servers", json!("https://api.example.com")),
        ("tags", json!("pets")),
        ("externalDocs", json!("https://docs.example.com")),
    ] {
        let error = OpenApiDocument::from_json(&json!({
            "openapi": "3.1.0",
            "info": {
                "title": "Pets",
                "version": "1.0.0"
            },
            "paths": {},
            (field): value
        }))
        .expect_err("invalid root metadata shapes must fail at document construction")
        .to_string();

        assert!(error.contains(&format!("#/{field}")), "{field}: {error}");
        assert!(
            error.contains("an array") || error.contains("an object"),
            "{field}: {error}"
        );
    }
}

#[test]
fn openapi_documents_validate_nested_non_contract_metadata_shapes_before_lowering() {
    for (field, value, pointer_fragment) in [
        (
            "info",
            json!({
                "title": "Pets",
                "version": "1.0.0",
                "contact": { "name": true }
            }),
            "#/info/contact/name",
        ),
        (
            "info",
            json!({
                "title": "Pets",
                "version": "1.0.0",
                "license": { "identifier": "Apache-2.0" }
            }),
            "#/info/license/name",
        ),
        (
            "info",
            json!({
                "title": "Pets",
                "version": "1.0.0",
                "license": {
                    "name": "Apache 2.0",
                    "identifier": "Apache-2.0",
                    "url": "https://www.apache.org/licenses/LICENSE-2.0.html"
                }
            }),
            "#/info/license",
        ),
        ("externalDocs", json!({}), "#/externalDocs/url"),
        ("servers", json!([{}]), "#/servers/0/url"),
        (
            "servers",
            json!([{
                "url": "https://api.example.com",
                "variables": {
                    "env": {
                        "enum": ["prod"]
                    }
                }
            }]),
            "#/servers/0/variables/env/default",
        ),
        (
            "servers",
            json!([{
                "url": "https://api.example.com",
                "variables": {
                    "env": {
                        "enum": ["prod"],
                        "default": "dev"
                    }
                }
            }]),
            "#/servers/0/variables/env/default",
        ),
        ("tags", json!([{}]), "#/tags/0/name"),
        ("security", json!(["bearerAuth"]), "#/security/0"),
        (
            "security",
            json!([{ "bearerAuth": [42] }]),
            "#/security/0/bearerAuth/0",
        ),
    ] {
        let mut document = json!({
            "openapi": "3.1.0",
            "info": {
                "title": "Pets",
                "version": "1.0.0"
            },
            "paths": {}
        });
        document
            .as_object_mut()
            .expect("test OpenAPI document is an object")
            .insert(field.to_owned(), value);

        let error = OpenApiDocument::from_json(&document)
            .expect_err("invalid accepted metadata shapes must fail at document construction")
            .to_string();
        assert!(error.contains(pointer_fragment), "{field}: {error}");
    }
}

#[test]
fn openapi_documents_accept_common_info_metadata_before_lowering() {
    for (field, value) in [
        ("summary", json!("Pets")),
        ("description", json!("Pet API")),
        ("termsOfService", json!("https://example.com/terms")),
        ("termsOfService", json!("../terms")),
        ("contact", json!({ "name": "API Team" })),
        ("contact", json!({ "url": "../team" })),
        ("contact", json!({ "email": "api@example.com" })),
        ("license", json!({ "name": "MIT" })),
        (
            "license",
            json!({ "name": "MIT", "url": "../licenses/mit" }),
        ),
    ] {
        OpenApiDocument::from_json(&json!({
            "openapi": "3.1.0",
            "info": {
                "title": "Pets",
                "version": "1.0.0",
                (field): value
            },
            "paths": {}
        }))
        .expect("common info metadata is valid even though it is not lowered");
    }
}

#[test]
fn openapi_documents_reject_invalid_info_url_metadata_before_lowering() {
    for (field, value, pointer_fragment) in [
        ("termsOfService", json!("http://["), "#/info/termsOfService"),
        (
            "contact",
            json!({ "url": "http://[" }),
            "#/info/contact/url",
        ),
        (
            "license",
            json!({ "name": "MIT", "url": "http://[" }),
            "#/info/license/url",
        ),
    ] {
        let error = OpenApiDocument::from_json(&json!({
            "openapi": "3.1.0",
            "info": {
                "title": "Pets",
                "version": "1.0.0",
                (field): value
            },
            "paths": {}
        }))
        .expect_err("invalid info URL metadata must fail during document validation")
        .to_string();

        assert!(error.contains(pointer_fragment), "{field}: {error}");
        assert!(error.contains("a valid URL reference"), "{field}: {error}");
    }
}

#[test]
fn openapi_documents_reject_invalid_contact_emails_before_lowering() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0",
            "contact": {
                "email": "not-an-email"
            }
        },
        "paths": {}
    }))
    .expect_err("contact email metadata must be a valid email address")
    .to_string();

    assert!(error.contains("#/info/contact/email"), "{error}");
    assert!(error.contains("a valid email address"), "{error}");
}

#[test]
fn openapi_documents_reject_invalid_external_docs_urls_before_lowering() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "paths": {},
        "externalDocs": {
            "url": "http://["
        }
    }))
    .expect_err("invalid external-docs URLs must fail during document validation")
    .to_string();

    assert!(error.contains("#/externalDocs/url"), "{error}");
    assert!(error.contains("a valid URL reference"), "{error}");
}

#[test]
fn openapi_documents_reject_invalid_plain_server_urls_before_lowering() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "paths": {},
        "servers": [{
            "url": "http://["
        }]
    }))
    .expect_err("plain server URLs must be valid URL references")
    .to_string();

    assert!(error.contains("#/servers/0/url"), "{error}");
    assert!(error.contains("a valid URL reference"), "{error}");
}

#[test]
fn openapi_documents_accept_balanced_templated_server_urls_before_lowering() {
    OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "paths": {},
        "servers": [{
            "url": "https://{region}.example.com:{port}/{basePath}",
            "variables": {
                "region": { "default": "us-west" },
                "port": { "default": "443" },
                "basePath": { "default": "v1" }
            }
        }]
    }))
    .expect("balanced templated server URLs remain valid OpenAPI metadata");
}

#[test]
fn openapi_documents_reject_malformed_templated_server_urls_before_lowering() {
    for url in [
        "https://{region.example.com",
        "https://region}.example.com",
        "https://{}.example.com",
        "https://{{region}}.example.com",
    ] {
        let error = OpenApiDocument::from_json(&json!({
            "openapi": "3.1.0",
            "info": {
                "title": "Pets",
                "version": "1.0.0"
            },
            "paths": {},
            "servers": [{
                "url": url
            }]
        }))
        .expect_err("malformed templated server URLs must fail during validation")
        .to_string();

        assert!(error.contains("#/servers/0/url"), "{url}: {error}");
        assert!(
            error.contains("a valid URL reference or server URL template"),
            "{url}: {error}"
        );
    }
}

#[test]
fn openapi_documents_reject_duplicate_tag_names_before_lowering() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "paths": {},
        "tags": [
            { "name": "pets" },
            { "name": "pets" }
        ]
    }))
    .expect_err("OpenAPI root tag names must be unique")
    .to_string();

    assert!(error.contains("#/tags/1/name"), "{error}");
    assert!(error.contains("unique"), "{error}");
}

#[test]
fn openapi_documents_reject_unknown_non_extension_root_fields() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "paths": {},
        "vendorMetadata": { "owner": "pets" }
    }))
    .expect_err("unknown non-extension root fields must fail at document construction")
    .to_string();

    assert!(error.contains("#/vendorMetadata"), "{error}");
    assert!(error.contains("specification extension"), "{error}");
}

#[test]
fn openapi_lowering_rejects_unsupported_document_schema_dialects() {
    let document = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "jsonSchemaDialect": "https://json-schema.org/draft-07/schema#",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "paths": {}
    }))
    .expect("unsupported-but-valid schema dialect declarations build before lowering");
    let error = validate_openapi_compatibility_input(&document)
        .expect_err("unsupported document-level schema dialects must fail before comparison")
        .to_string();

    assert!(error.contains("jsonSchemaDialect"), "{error}");
    assert!(error.contains("draft-07"), "{error}");
}

#[test]
fn openapi_documents_reject_invalid_document_schema_dialect_uris_before_lowering() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "jsonSchemaDialect": "http://[",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "paths": {}
    }))
    .expect_err("jsonSchemaDialect must be a syntactically valid absolute URI")
    .to_string();

    assert!(error.contains("#/jsonSchemaDialect"), "{error}");
    assert!(error.contains("an absolute URI"), "{error}");
}

#[test]
fn compatibility_accepts_security_components_outside_the_contract_model() {
    let report = report(
        json!({
            "openapi": "3.1.0",
            "info": { "title": "Pets", "version": "1.0.0" },
            "paths": {
                "/pets": {
                    "get": get_operation()
                }
            },
            "components": {
                "securitySchemes": {
                    "Bearer": {
                        "type": "http",
                        "scheme": "bearer"
                    }
                }
            }
        }),
        spec(get_operation()),
    );

    assert!(report.is_compatible());
}

#[test]
fn compatibility_accepts_referenced_security_components_outside_the_contract_model() {
    let report = report(
        json!({
            "openapi": "3.1.0",
            "info": { "title": "Pets", "version": "1.0.0" },
            "paths": {
                "/pets": {
                    "get": get_operation()
                }
            },
            "components": {
                "securitySchemes": {
                    "Bearer": {
                        "$ref": "#/components/securitySchemes/BearerImpl"
                    },
                    "BearerImpl": {
                        "type": "http",
                        "scheme": "bearer"
                    }
                }
            }
        }),
        spec(get_operation()),
    );

    assert!(report.is_compatible());
}

#[test]
fn openapi_documents_reject_root_security_requirements_for_unknown_schemes() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "paths": {},
        "security": [
            {
                "MissingAuth": []
            }
        ]
    }))
    .expect_err("security requirements must name declared security schemes")
    .to_string();

    assert!(
        error.contains("#/security/0/MissingAuth")
            && error.contains("declared components.securitySchemes"),
        "{error}"
    );
}

#[test]
fn openapi_documents_reject_operation_security_requirements_for_unknown_schemes() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "get": {
                    "security": [
                        {
                            "MissingAuth": []
                        }
                    ],
                    "responses": {
                        "200": response_schema(json!({ "type": "object" }))
                    }
                }
            }
        },
        "components": {
            "securitySchemes": {
                "Bearer": {
                    "type": "http",
                    "scheme": "bearer"
                }
            }
        }
    }))
    .expect_err("security requirements must name declared security schemes")
    .to_string();

    assert!(
        error.contains("#/paths/~1pets/get/security/0/MissingAuth")
            && error.contains("declared components.securitySchemes"),
        "{error}"
    );
}

#[test]
fn openapi_documents_validate_security_scheme_shapes_before_lowering() {
    for (scheme, pointer_fragment) in [
        (
            json!({ "type": "apiKey", "in": "header" }),
            "#/components/securitySchemes/Auth/name",
        ),
        (
            json!({ "type": "apiKey", "name": "api-key", "in": "body" }),
            "#/components/securitySchemes/Auth/in",
        ),
        (
            json!({ "type": "http" }),
            "#/components/securitySchemes/Auth/scheme",
        ),
        (
            json!({ "type": "http", "scheme": "bearer", "flows": {} }),
            "#/components/securitySchemes/Auth/flows",
        ),
        (
            json!({ "type": "oauth2", "flows": { "implicit": { "scopes": {} } } }),
            "#/components/securitySchemes/Auth/flows/implicit/authorizationUrl",
        ),
        (
            json!({ "type": "oauth2", "flows": { "implicit": { "authorizationUrl": "http://[", "scopes": {} } } }),
            "#/components/securitySchemes/Auth/flows/implicit/authorizationUrl",
        ),
        (
            json!({ "type": "oauth2", "flows": { "implicit": { "authorizationUrl": "https://example.com/authorize", "tokenUrl": "https://example.com/token", "scopes": {} } } }),
            "#/components/securitySchemes/Auth/flows/implicit/tokenUrl",
        ),
        (
            json!({ "type": "oauth2", "flows": { "password": { "tokenUrl": "http://[", "scopes": {} } } }),
            "#/components/securitySchemes/Auth/flows/password/tokenUrl",
        ),
        (
            json!({ "type": "oauth2", "flows": { "password": { "authorizationUrl": "https://example.com/authorize", "tokenUrl": "https://example.com/token", "scopes": {} } } }),
            "#/components/securitySchemes/Auth/flows/password/authorizationUrl",
        ),
        (
            json!({ "type": "oauth2", "flows": { "password": { "tokenUrl": "https://example.com/token", "refreshUrl": "http://[", "scopes": {} } } }),
            "#/components/securitySchemes/Auth/flows/password/refreshUrl",
        ),
        (
            json!({ "type": "oauth2", "flows": { "clientCredentials": { "authorizationUrl": "https://example.com/authorize", "tokenUrl": "https://example.com/token", "scopes": {} } } }),
            "#/components/securitySchemes/Auth/flows/clientCredentials/authorizationUrl",
        ),
        (
            json!({ "type": "oauth2", "flows": { "password": { "tokenUrl": "https://example.com/token", "scopes": { "read": 42 } } } }),
            "#/components/securitySchemes/Auth/flows/password/scopes/read",
        ),
        (
            json!({ "type": "openIdConnect" }),
            "#/components/securitySchemes/Auth/openIdConnectUrl",
        ),
        (
            json!({ "type": "openIdConnect", "openIdConnectUrl": "http://[" }),
            "#/components/securitySchemes/Auth/openIdConnectUrl",
        ),
        (
            json!({ "type": "bogus" }),
            "#/components/securitySchemes/Auth/type",
        ),
    ] {
        let error = OpenApiDocument::from_json(&json!({
            "openapi": "3.1.0",
            "info": {
                "title": "Pets",
                "version": "1.0.0"
            },
            "paths": {},
            "components": {
                "securitySchemes": {
                    "Auth": scheme
                }
            }
        }))
        .expect_err("invalid security scheme shapes must fail during document validation")
        .to_string();

        assert!(error.contains(pointer_fragment), "{error}");
    }
}

#[test]
fn compatibility_rejects_invalid_unused_component_collections() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "get": get_operation()
            }
        },
        "components": {
            "parameters": []
        }
    }))
    .expect_err("invalid component collection containers must fail during document validation")
    .to_string();

    assert!(
        error.contains("#/components/parameters") && error.contains("an object"),
        "{error}"
    );
}

#[test]
fn openapi_documents_reject_unknown_non_extension_component_fields() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {},
        "components": {
            "vendorMetadata": {}
        }
    }))
    .expect_err("unknown component fields are invalid OpenAPI")
    .to_string();

    assert!(error.contains("#/components/vendorMetadata"), "{error}");
    assert!(error.contains("specification extension"), "{error}");
}

#[test]
fn openapi_documents_validate_callback_security_before_reporting_callbacks_as_unsupported() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "post": {
                    "callbacks": {
                        "petCreated": {
                            "{$request.body#/callbackUrl}": {
                                "post": {
                                    "security": [{ "MissingAuth": [] }],
                                    "responses": {
                                        "200": response_schema(json!({ "type": "object" }))
                                    }
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": response_schema(json!({ "type": "object" }))
                    }
                }
            }
        },
        "components": {
            "securitySchemes": {
                "BearerAuth": {
                    "type": "http",
                    "scheme": "bearer"
                }
            }
        }
    }))
    .expect_err("invalid callback security must fail before lowerability checks")
    .to_string();

    assert!(
        error.contains(
            "#/paths/~1pets/post/callbacks/petCreated/{$request.body#~1callbackUrl}/post/security/0/MissingAuth"
        ) && error.contains("declared components.securitySchemes"),
        "{error}"
    );
    assert!(!error.contains("operation callbacks"), "{error}");
}

#[test]
fn openapi_documents_validate_component_callback_security_before_reporting_callbacks_as_unsupported()
 {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {},
        "components": {
            "callbacks": {
                "PetCreated": {
                    "{$request.body#/callbackUrl}": {
                        "post": {
                            "security": [{ "MissingAuth": [] }],
                            "responses": {
                                "200": response_schema(json!({ "type": "object" }))
                            }
                        }
                    }
                }
            },
            "securitySchemes": {
                "BearerAuth": {
                    "type": "http",
                    "scheme": "bearer"
                }
            }
        }
    }))
    .expect_err("invalid component callback security must fail before lowerability checks")
    .to_string();

    assert!(
        error.contains(
            "#/components/callbacks/PetCreated/{$request.body#~1callbackUrl}/post/security/0/MissingAuth"
        ) && error.contains("declared components.securitySchemes"),
        "{error}"
    );
    assert!(!error.contains("component collection"), "{error}");
}

#[test]
fn openapi_documents_validate_component_path_item_security_before_reporting_path_items_as_unsupported()
 {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {},
        "components": {
            "pathItems": {
                "Pets": {
                    "post": {
                        "security": [{ "MissingAuth": [] }],
                        "responses": {
                            "200": response_schema(json!({ "type": "object" }))
                        }
                    }
                }
            },
            "securitySchemes": {
                "BearerAuth": {
                    "type": "http",
                    "scheme": "bearer"
                }
            }
        }
    }))
    .expect_err("invalid component path-item security must fail before lowerability checks")
    .to_string();

    assert!(
        error.contains("#/components/pathItems/Pets/post/security/0/MissingAuth")
            && error.contains("declared components.securitySchemes"),
        "{error}"
    );
    assert!(!error.contains("component collection"), "{error}");
}

#[test]
fn openapi_documents_reject_backend_invalid_callback_schemas_before_reporting_callbacks_as_unsupported()
 {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "post": {
                    "callbacks": {
                        "petCreated": {
                            "{$request.body#/callbackUrl}": {
                                "post": {
                                    "requestBody": {
                                        "content": {
                                            "application/json": {
                                                "schema": {
                                                    "type": "string",
                                                    "deprecated": "eventually"
                                                }
                                            }
                                        }
                                    },
                                    "responses": {
                                        "200": response_schema(json!({ "type": "object" }))
                                    }
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": response_schema(json!({ "type": "object" }))
                    }
                }
            }
        }
    }))
    .expect_err("callback schemas must validate before unsupported-surface handling")
    .to_string();

    assert!(
        error.contains(
            "#/paths/~1pets/post/callbacks/petCreated/{$request.body#~1callbackUrl}/post/requestBody/content/application~1json/schema"
        ) && error.contains("failed to compile raw schema validator"),
        "{error}"
    );
    assert!(
        !error.contains("OpenAPI compatibility checks do not support operation callbacks"),
        "{error}"
    );
}

#[test]
fn openapi_documents_reject_backend_invalid_component_callback_schemas_before_reporting_callbacks_as_unsupported()
 {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {},
        "components": {
            "callbacks": {
                "PetCreated": {
                    "{$request.body#/callbackUrl}": {
                        "post": {
                            "requestBody": {
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "type": "string",
                                            "deprecated": "eventually"
                                        }
                                    }
                                }
                            },
                            "responses": {
                                "200": response_schema(json!({ "type": "object" }))
                            }
                        }
                    }
                }
            }
        }
    }))
    .expect_err("component callback schemas must validate before lowerability checks")
    .to_string();

    assert!(
        error.contains(
            "#/components/callbacks/PetCreated/{$request.body#~1callbackUrl}/post/requestBody/content/application~1json/schema"
        ) && error.contains("failed to compile raw schema validator"),
        "{error}"
    );
    assert!(
        !error.contains("OpenAPI compatibility checks do not support component collection"),
        "{error}"
    );
}

#[test]
fn openapi_documents_reject_backend_invalid_component_path_item_schemas_before_reporting_path_items_as_unsupported()
 {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {},
        "components": {
            "pathItems": {
                "Pets": {
                    "post": {
                        "requestBody": {
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "string",
                                        "deprecated": "eventually"
                                    }
                                }
                            }
                        },
                        "responses": {
                            "200": response_schema(json!({ "type": "object" }))
                        }
                    }
                }
            }
        }
    }))
    .expect_err("component path-item schemas must validate before lowerability checks")
    .to_string();

    assert!(
        error.contains(
            "#/components/pathItems/Pets/post/requestBody/content/application~1json/schema"
        ) && error.contains("failed to compile raw schema validator"),
        "{error}"
    );
    assert!(
        !error.contains("OpenAPI compatibility checks do not support component collection"),
        "{error}"
    );
}

#[test]
fn openapi_lowering_rejects_valid_but_unsupported_component_collections() {
    let document = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {},
        "components": {
            "examples": {}
        }
    }))
    .expect("valid unsupported component collections build before lowerability checks");
    let error = validate_openapi_compatibility_input(&document)
        .expect_err("unsupported component collections must fail before comparison")
        .to_string();

    assert!(error.contains("#/components/examples"), "{error}");
    assert!(error.contains("component collection"), "{error}");
}

#[test]
fn openapi_documents_validate_unsupported_component_collection_entries_before_lowering() {
    for (collection, value, pointer_fragment) in [
        (
            "examples",
            json!({
                "Broken": []
            }),
            "#/components/examples/Broken",
        ),
        (
            "links",
            json!({
                "Broken": []
            }),
            "#/components/links/Broken",
        ),
        (
            "callbacks",
            json!({
                "Broken": []
            }),
            "#/components/callbacks/Broken",
        ),
        (
            "pathItems",
            json!({
                "Broken": []
            }),
            "#/components/pathItems/Broken",
        ),
        (
            "pathItems",
            json!({
                "x-Broken": []
            }),
            "#/components/pathItems/x-Broken",
        ),
    ] {
        let error = OpenApiDocument::from_json(&json!({
            "openapi": "3.1.0",
            "info": { "title": "Pets", "version": "1.0.0" },
            "paths": {},
            "components": {
                (collection): value
            }
        }))
        .expect_err("invalid unsupported component entries must fail as invalid OpenAPI")
        .to_string();

        assert!(
            error.contains(pointer_fragment),
            "{pointer_fragment}: {error}"
        );
        assert!(!error.contains("component collection"), "{error}");
    }
}

#[test]
fn openapi_documents_reject_component_names_outside_the_openapi_pattern_before_lowering() {
    for (collection, value) in [("schemas", json!(true)), ("parameters", json!({}))] {
        let error = OpenApiDocument::from_json(&json!({
            "openapi": "3.1.0",
            "info": { "title": "Pets", "version": "1.0.0" },
            "paths": {
                "/pets": {
                    "get": get_operation()
                }
            },
            "components": {
                (collection): {
                    "bad name": value
                }
            }
        }))
        .expect_err("invalid component names must fail during document validation")
        .to_string();

        assert!(
            error.contains(&format!("#/components/{collection}/bad name"))
                && error.contains("^[a-zA-Z0-9._-]+$"),
            "{collection}: {error}"
        );
    }
}

#[test]
fn openapi_documents_reject_invalid_unused_component_schemas_before_lowering() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "get": get_operation()
            }
        },
        "components": {
            "schemas": {
                "Broken": {
                    "type": 42
                }
            }
        }
    }))
    .expect_err("invalid component schemas must fail during OpenAPI document validation")
    .to_string();

    assert!(error.contains("#/components/schemas/Broken"), "{error}");
    assert!(error.contains("type"), "{error}");
}

#[test]
fn openapi_documents_reject_backend_invalid_unused_component_schemas_before_lowering() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "get": get_operation()
            }
        },
        "components": {
            "schemas": {
                "Broken": {
                    "type": "string",
                    "deprecated": "eventually"
                }
            }
        }
    }))
    .expect_err("backend-invalid component schemas must fail during document validation")
    .to_string();

    assert!(
        error.contains("#/components/schemas/Broken")
            && error.contains("OpenAPI schema")
            && error.contains("failed to compile raw schema validator"),
        "{error}"
    );
}

#[test]
fn deferred_component_schema_refs_do_not_hide_invalid_sibling_component_schemas() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "get": get_operation()
            }
        },
        "components": {
            "schemas": {
                "Broken": {
                    "type": "string",
                    "deprecated": "eventually"
                },
                "Deferred": {
                    "$ref": "https://example.com/schemas/deferred.json"
                }
            }
        }
    }))
    .expect_err("deferred refs must not mask invalid sibling component schemas")
    .to_string();

    assert!(
        error.contains("#/components/schemas/Broken")
            && error.contains("failed to compile raw schema validator"),
        "{error}"
    );
}

#[test]
fn deferred_component_schema_refs_do_not_hide_invalid_sibling_branches() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "get": get_operation()
            }
        },
        "components": {
            "schemas": {
                "Mixed": {
                    "anyOf": [
                        { "$ref": "https://example.com/schemas/deferred.json" },
                        {
                            "type": "string",
                            "deprecated": "eventually"
                        }
                    ]
                }
            }
        }
    }))
    .expect_err("deferred refs must not mask invalid sibling branches")
    .to_string();

    assert!(
        error.contains("#/components/schemas/Mixed")
            && error.contains("failed to compile raw schema validator"),
        "{error}"
    );
}

#[test]
fn deferred_component_schema_reference_metadata_still_validates_its_own_shape() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "get": get_operation()
            }
        },
        "components": {
            "schemas": {
                "Broken": {
                    "$id": 42,
                    "type": "string"
                }
            }
        }
    }))
    .expect_err("invalid deferred reference metadata must still fail document validation")
    .to_string();

    assert!(
        error.contains("#/components/schemas/Broken")
            && error.contains("keyword '$id'")
            && error.contains("must be a string"),
        "{error}"
    );
}

#[test]
fn openapi_documents_reject_backend_invalid_unused_response_component_schemas_before_lowering() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "get": get_operation()
            }
        },
        "components": {
            "responses": {
                "Broken": {
                    "description": "broken",
                    "content": {
                        "application/json": {
                            "schema": {
                                "type": "string",
                                "deprecated": "eventually"
                            }
                        }
                    }
                }
            }
        }
    }))
    .expect_err("response component schemas must validate during document construction")
    .to_string();

    assert!(
        error.contains("#/components/responses/Broken/content/application~1json/schema")
            && error.contains("failed to compile raw schema validator"),
        "{error}"
    );
}

#[test]
fn openapi_documents_reject_backend_invalid_unused_request_body_component_schemas_before_lowering()
{
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "get": get_operation()
            }
        },
        "components": {
            "requestBodies": {
                "Broken": {
                    "content": {
                        "application/json": {
                            "schema": {
                                "type": "string",
                                "deprecated": "eventually"
                            }
                        }
                    }
                }
            }
        }
    }))
    .expect_err("request-body component schemas must validate during document construction")
    .to_string();

    assert!(
        error.contains("#/components/requestBodies/Broken/content/application~1json/schema")
            && error.contains("failed to compile raw schema validator"),
        "{error}"
    );
}

#[test]
fn deferred_request_body_component_refs_do_not_hide_invalid_sibling_branches() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "get": get_operation()
            }
        },
        "components": {
            "requestBodies": {
                "Mixed": {
                    "content": {
                        "application/json": {
                            "schema": {
                                "anyOf": [
                                    { "$ref": "https://example.com/schemas/deferred.json" },
                                    {
                                        "type": "string",
                                        "deprecated": "eventually"
                                    }
                                ]
                            }
                        }
                    }
                }
            }
        }
    }))
    .expect_err("deferred request-body refs must not mask invalid sibling branches")
    .to_string();

    assert!(
        error.contains("#/components/requestBodies/Mixed/content/application~1json/schema")
            && error.contains("failed to compile raw schema validator"),
        "{error}"
    );
}

#[test]
fn openapi_documents_reject_backend_invalid_unused_parameter_component_schemas_before_lowering() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "get": get_operation()
            }
        },
        "components": {
            "parameters": {
                "Broken": {
                    "name": "limit",
                    "in": "query",
                    "schema": {
                        "type": "string",
                        "deprecated": "eventually"
                    }
                }
            }
        }
    }))
    .expect_err("parameter component schemas must validate during document construction")
    .to_string();

    assert!(
        error.contains("#/components/parameters/Broken/schema")
            && error.contains("failed to compile raw schema validator"),
        "{error}"
    );
}

#[test]
fn openapi_documents_reject_backend_invalid_unused_header_component_schemas_before_lowering() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "get": get_operation()
            }
        },
        "components": {
            "headers": {
                "Broken": {
                    "schema": {
                        "type": "string",
                        "deprecated": "eventually"
                    }
                }
            }
        }
    }))
    .expect_err("header component schemas must validate during document construction")
    .to_string();

    assert!(
        error.contains("#/components/headers/Broken/schema")
            && error.contains("failed to compile raw schema validator"),
        "{error}"
    );
}

#[test]
fn openapi_documents_reject_backend_invalid_inline_parameter_schemas_before_lowering() {
    let error = OpenApiDocument::from_json(&spec(json!({
        "parameters": [
            {
                "name": "limit",
                "in": "query",
                "schema": {
                    "type": "string",
                    "deprecated": "eventually"
                }
            }
        ],
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    })))
    .expect_err("inline parameter schemas must validate during document construction")
    .to_string();

    assert!(
        error.contains("#/paths/~1pets/get/parameters/0/schema")
            && error.contains("failed to compile raw schema validator"),
        "{error}"
    );
}

#[test]
fn openapi_documents_reject_backend_invalid_inline_request_body_schemas_before_lowering() {
    let error = OpenApiDocument::from_json(&spec(json!({
        "requestBody": {
            "content": {
                "application/json": {
                    "schema": {
                        "type": "string",
                        "deprecated": "eventually"
                    }
                }
            }
        },
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    })))
    .expect_err("inline request-body schemas must validate during document construction")
    .to_string();

    assert!(
        error.contains("#/paths/~1pets/get/requestBody/content/application~1json/schema")
            && error.contains("failed to compile raw schema validator"),
        "{error}"
    );
}

#[test]
fn openapi_documents_reject_backend_invalid_inline_response_header_schemas_before_lowering() {
    let error = OpenApiDocument::from_json(&spec(json!({
        "responses": {
            "200": {
                "description": "ok",
                "headers": {
                    "X-Trace-Id": {
                        "schema": {
                            "type": "string",
                            "deprecated": "eventually"
                        }
                    }
                }
            }
        }
    })))
    .expect_err("inline response-header schemas must validate during document construction")
    .to_string();

    assert!(
        error.contains("#/paths/~1pets/get/responses/200/headers/X-Trace-Id/schema")
            && error.contains("failed to compile raw schema validator"),
        "{error}"
    );
}

#[test]
fn openapi_documents_reject_backend_invalid_webhook_schemas_before_reporting_webhooks_as_unsupported()
 {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "webhooks": {
            "pet.created": {
                "post": {
                    "requestBody": {
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "string",
                                    "deprecated": "eventually"
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": response_schema(json!({ "type": "object" }))
                    }
                }
            }
        }
    }))
    .expect_err("webhook schemas must validate before unsupported-surface handling")
    .to_string();

    assert!(
        error.contains("#/webhooks/pet.created/post/requestBody/content/application~1json/schema")
            && error.contains("failed to compile raw schema validator"),
        "{error}"
    );
    assert!(
        !error.contains("OpenAPI compatibility checks do not support webhooks"),
        "{error}"
    );
}

#[test]
fn openapi_documents_keep_valid_but_unsupported_component_schemas_for_lowering_validation() {
    for (keyword, schema) in [
        (
            "dependentSchemas",
            json!({
                "type": "object",
                "dependentSchemas": {
                    "kind": { "required": ["detail"] }
                }
            }),
        ),
        (
            "dependencies",
            json!({
                "type": "object",
                "dependencies": {
                    "kind": ["detail"]
                }
            }),
        ),
        (
            "additionalItems",
            json!({
                "additionalItems": false
            }),
        ),
    ] {
        let document = OpenApiDocument::from_json(&json!({
            "openapi": "3.1.0",
            "info": { "title": "Pets", "version": "1.0.0" },
            "paths": {
                "/pets": {
                    "get": get_operation()
                }
            },
            "components": {
                "schemas": {
                    "Deferred": schema
                }
            }
        }))
        .expect("valid-but-unsupported component schemas should build before lowering");

        let error = validate_openapi_compatibility_input(&document)
            .expect_err("unsupported component schema keywords must fail during lowering readiness")
            .to_string();

        assert!(
            error.contains(&format!("#/components/schemas/Deferred/{keyword}"))
                && error.contains(&format!(
                    "OpenAPI compatibility checks do not support JSON Schema keyword '{keyword}'"
                )),
            "{keyword}: {error}"
        );
    }
}

#[test]
fn openapi_documents_keep_valid_but_unsupported_component_schema_reference_keywords_for_lowering_validation()
 {
    let document = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "get": get_operation()
            }
        },
        "components": {
            "schemas": {
                "Deferred": {
                    "$id": "https://example.com/schemas/deferred.json",
                    "type": "string"
                }
            }
        }
    }))
    .expect("valid-but-unsupported reference metadata should build before lowering");

    let error = validate_openapi_compatibility_input(&document)
        .expect_err("unsupported component schema reference metadata must fail during lowering")
        .to_string();

    assert!(
        error.contains("#/components/schemas/Deferred/$id")
            && error
                .contains("OpenAPI compatibility checks do not support JSON Schema keyword '$id'"),
        "{error}"
    );
}

#[test]
fn openapi_documents_keep_valid_component_refs_inside_later_lowering_schema_keywords() {
    for (keyword, deferred_schema) in [
        (
            "contentSchema",
            json!({
                "type": "string",
                "contentSchema": { "$ref": "#/components/schemas/Shared" }
            }),
        ),
        (
            "dependentSchemas",
            json!({
                "type": "object",
                "dependentSchemas": {
                    "kind": { "$ref": "#/components/schemas/Shared" }
                }
            }),
        ),
        (
            "unevaluatedItems",
            json!({
                "type": "array",
                "unevaluatedItems": { "$ref": "#/components/schemas/Shared" }
            }),
        ),
        (
            "unevaluatedProperties",
            json!({
                "type": "object",
                "unevaluatedProperties": { "$ref": "#/components/schemas/Shared" }
            }),
        ),
    ] {
        let document = OpenApiDocument::from_json(&json!({
            "openapi": "3.1.0",
            "info": { "title": "Pets", "version": "1.0.0" },
            "paths": {
                "/pets": {
                    "get": get_operation()
                }
            },
            "components": {
                "schemas": {
                    "Shared": { "type": "string" },
                    "Deferred": deferred_schema
                }
            }
        }))
        .unwrap_or_else(|error| {
            panic!(
                "valid refs under later-lowering keyword {keyword} should survive document validation: {error}"
            )
        });

        let error = validate_openapi_compatibility_input(&document)
            .expect_err("later-lowering schema keywords must fail during compatibility readiness")
            .to_string();

        assert!(
            error.contains(&format!("#/components/schemas/Deferred/{keyword}"))
                && error.contains(&format!(
                    "OpenAPI compatibility checks do not support JSON Schema keyword '{keyword}'"
                )),
            "{keyword}: {error}"
        );
    }
}

#[test]
fn openapi_documents_reject_unresolved_refs_in_unused_component_schemas_before_lowering() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "get": get_operation()
            }
        },
        "components": {
            "schemas": {
                "Broken": {
                    "$ref": "#/components/schemas/Missing"
                }
            }
        }
    }))
    .expect_err("component schema references must resolve during document validation")
    .to_string();

    assert!(error.contains("#/components/schemas/Broken"), "{error}");
    assert!(error.contains("Missing"), "{error}");
    assert!(error.contains("does not resolve"), "{error}");
}

#[test]
fn openapi_documents_reject_path_parameters_that_do_not_match_the_path_template_before_lowering() {
    let missing_parameter = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets/{petId}": {
                "get": {
                    "responses": {
                        "200": response_schema(json!({ "type": "object" }))
                    }
                }
            }
        }
    }))
    .expect_err("missing path-template parameters must fail during OpenAPI validation")
    .to_string();
    assert!(
        missing_parameter.contains("#/paths/~1pets~1{petId}/get/parameters")
            && missing_parameter.contains("every template expression"),
        "{missing_parameter}"
    );

    let stray_parameter = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets/{petId}": {
                "get": {
                    "parameters": [{
                        "name": "other",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }],
                    "responses": {
                        "200": response_schema(json!({ "type": "object" }))
                    }
                }
            }
        }
    }))
    .expect_err("stray path-template parameters must fail during OpenAPI validation")
    .to_string();
    assert!(
        stray_parameter.contains("#/paths/~1pets~1{petId}/get/parameters/0/name")
            && stray_parameter.contains("template expression"),
        "{stray_parameter}"
    );
}

#[test]
fn openapi_documents_validate_local_path_parameter_refs_against_the_template_before_lowering() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "components": {
            "parameters": {
                "OtherPathParameter": {
                    "name": "other",
                    "in": "path",
                    "required": true,
                    "schema": { "type": "string" }
                }
            }
        },
        "paths": {
            "/pets/{petId}": {
                "get": {
                    "parameters": [{
                        "$ref": "#/components/parameters/OtherPathParameter"
                    }],
                    "responses": {
                        "200": response_schema(json!({ "type": "object" }))
                    }
                }
            }
        }
    }))
    .expect_err("local referenced path parameters must be validated before lowering")
    .to_string();

    assert!(
        error.contains("#/paths/~1pets~1{petId}/get/parameters/0/name")
            && error.contains("template expression"),
        "{error}"
    );
}

#[test]
fn openapi_documents_reject_invalid_path_template_keys_before_lowering() {
    let missing_slash = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "pets": {
                "get": {
                    "responses": {
                        "200": response_schema(json!({ "type": "object" }))
                    }
                }
            }
        }
    }))
    .expect_err("path keys without a leading slash are invalid OpenAPI")
    .to_string();
    assert!(
        missing_slash.contains("#/paths/pets") && missing_slash.contains("beginning with '/'"),
        "{missing_slash}"
    );

    let unbalanced_template = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets/{petId": {
                "get": {
                    "responses": {
                        "200": response_schema(json!({ "type": "object" }))
                    }
                }
            }
        }
    }))
    .expect_err("unbalanced path templates are invalid OpenAPI")
    .to_string();
    assert!(
        unbalanced_template.contains("#/paths/~1pets~1{petId")
            && unbalanced_template.contains("balanced non-empty template expressions"),
        "{unbalanced_template}"
    );
}

#[test]
fn openapi_documents_reject_duplicate_templated_path_shapes_before_lowering() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets/{petId}": {},
            "/pets/{name}": {}
        }
    }))
    .expect_err("equivalent templated path shapes are invalid OpenAPI")
    .to_string();

    assert!(
        error.contains("#/paths/~1pets~1{name}") || error.contains("#/paths/~1pets~1{petId}"),
        "{error}"
    );
    assert!(
        error.contains("path template shape") && error.contains("different parameter names"),
        "{error}"
    );
}

#[test]
fn compatibility_rejects_callbacks_until_they_are_compared() {
    let error = compat_error(
        spec(json!({
            "callbacks": {
                "petCreated": {
                    "{$request.body#/callbackUrl}": {
                        "post": {
                            "responses": {
                                "200": response_schema(json!({ "type": "object" }))
                            }
                        }
                    }
                }
            },
            "responses": {
                "200": response_schema(json!({ "type": "object" }))
            }
        })),
        spec(get_operation()),
    );

    assert!(error.contains("#/paths/~1pets/get/callbacks"), "{error}");
    assert!(
        error.contains("OpenAPI compatibility checks do not support operation callbacks"),
        "{error}"
    );
}

#[test]
fn openapi_documents_reject_invalid_callbacks_before_lowering() {
    for (callbacks, pointer_fragment) in [
        (json!([]), "#/paths/~1pets/get/callbacks"),
        (
            json!({
                "petCreated": []
            }),
            "#/paths/~1pets/get/callbacks/petCreated",
        ),
        (
            json!({
                "petCreated": {
                    "{$request.body#/callbackUrl}": []
                }
            }),
            "#/paths/~1pets/get/callbacks/petCreated/{$request.body#~1callbackUrl}",
        ),
    ] {
        let error = OpenApiDocument::from_json(&json!({
            "openapi": "3.1.0",
            "info": { "title": "Pets", "version": "1.0.0" },
            "paths": {
                "/pets": {
                    "get": {
                        "callbacks": callbacks,
                        "responses": {
                            "200": response_schema(json!({ "type": "object" }))
                        }
                    }
                }
            }
        }))
        .expect_err("invalid callbacks must fail during OpenAPI document validation")
        .to_string();

        assert!(
            error.contains(pointer_fragment),
            "{pointer_fragment}: {error}"
        );
        assert!(!error.contains("operation callbacks"), "{error}");
    }
}

#[test]
fn openapi_documents_reject_invalid_callback_key_expressions_before_lowering() {
    for key in [
        "{not-a-runtime-expression}",
        "{$request.body#/bad~2escape}",
        "https://callbacks.example.test/{$request.body#/id",
    ] {
        let error = OpenApiDocument::from_json(&json!({
            "openapi": "3.1.0",
            "info": { "title": "Pets", "version": "1.0.0" },
            "paths": {
                "/pets": {
                    "post": {
                        "callbacks": {
                            "petCreated": {
                                (key): {
                                    "post": {
                                        "responses": {
                                            "200": response_schema(json!({ "type": "object" }))
                                        }
                                    }
                                }
                            }
                        },
                        "responses": {
                            "200": response_schema(json!({ "type": "object" }))
                        }
                    }
                }
            }
        }))
        .expect_err("invalid callback runtime expressions must fail during document validation")
        .to_string();

        assert!(
            error.contains("#/paths/~1pets/post/callbacks/petCreated/")
                && error.contains("callback key"),
            "{key}: {error}"
        );
        assert!(!error.contains("operation callbacks"), "{key}: {error}");
    }
}

#[test]
fn compatibility_accepts_operation_security_outside_the_contract_model() {
    let report = report(
        json!({
            "openapi": "3.1.0",
            "info": { "title": "Pets", "version": "1.0.0" },
            "paths": {
                "/pets": {
                    "get": {
                        "security": [{ "bearerAuth": [] }],
                        "responses": {
                            "200": response_schema(json!({ "type": "object" }))
                        }
                    }
                }
            },
            "components": {
                "securitySchemes": {
                    "bearerAuth": {
                        "type": "http",
                        "scheme": "bearer"
                    }
                }
            }
        }),
        spec(get_operation()),
    );

    assert!(report.is_compatible());
}

#[test]
fn compatibility_accepts_common_path_item_metadata_before_lowering() {
    let report = report(
        json!({
            "openapi": "3.1.0",
            "info": { "title": "Pets", "version": "1.0.0" },
            "paths": {
                "/pets": {
                    "servers": [{ "url": "https://api.example.com" }],
                    "summary": "Pets",
                    "description": "Pet collection",
                    "get": get_operation()
                }
            }
        }),
        spec(get_operation()),
    );

    assert!(report.is_compatible());
}

#[test]
fn compatibility_accepts_common_operation_metadata_before_lowering() {
    let report = report(
        json!({
            "openapi": "3.1.0",
            "info": { "title": "Pets", "version": "1.0.0" },
            "paths": {
                "/pets": {
                    "get": {
                        "servers": [{ "url": "https://api.example.com" }],
                        "security": [{ "bearerAuth": [] }],
                        "tags": ["pets"],
                        "summary": "List pets",
                        "description": "Returns pets",
                        "externalDocs": { "url": "https://docs.example.com" },
                        "operationId": "listPets",
                        "deprecated": false,
                        "responses": {
                            "200": response_schema(json!({ "type": "object" }))
                        }
                    }
                }
            },
            "components": {
                "securitySchemes": {
                    "bearerAuth": {
                        "type": "http",
                        "scheme": "bearer"
                    }
                }
            }
        }),
        spec(get_operation()),
    );

    assert!(report.is_compatible());
}

#[test]
fn openapi_documents_reject_duplicate_operation_ids() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "get": {
                    "operationId": "listPets",
                    "responses": {
                        "200": response_schema(json!({ "type": "object" }))
                    }
                },
                "post": {
                    "operationId": "listPets",
                    "responses": {
                        "201": response_schema(json!({ "type": "object" }))
                    }
                }
            }
        }
    }))
    .expect_err("duplicate operationIds are invalid OpenAPI")
    .to_string();

    assert!(
        error.contains("duplicate OpenAPI operationId 'listPets'"),
        "{error}"
    );
    assert!(
        error.contains("#/paths/~1pets/post/operationId")
            || error.contains("#/paths/~1pets/get/operationId"),
        "{error}"
    );
}

#[test]
fn openapi_documents_reject_duplicate_callback_operation_ids() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "post": {
                    "operationId": "createPet",
                    "callbacks": {
                        "petCreated": {
                            "{$request.body#/callbackUrl}": {
                                "post": {
                                    "operationId": "createPet",
                                    "responses": {
                                        "200": response_schema(json!({ "type": "object" }))
                                    }
                                }
                            }
                        }
                    },
                    "responses": {
                        "201": response_schema(json!({ "type": "object" }))
                    }
                }
            }
        }
    }))
    .expect_err("callback operationIds must participate in global OpenAPI uniqueness")
    .to_string();

    assert!(
        error.contains("duplicate OpenAPI operationId 'createPet'"),
        "{error}"
    );
    assert!(
        error.contains(
            "#/paths/~1pets/post/callbacks/petCreated/{$request.body#~1callbackUrl}/post/operationId"
        ),
        "{error}"
    );
}

#[test]
fn openapi_documents_reject_duplicate_component_callback_operation_ids() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "post": {
                    "operationId": "createPet",
                    "responses": {
                        "201": response_schema(json!({ "type": "object" }))
                    }
                }
            }
        },
        "components": {
            "callbacks": {
                "PetCreated": {
                    "{$request.body#/callbackUrl}": {
                        "post": {
                            "operationId": "createPet",
                            "responses": {
                                "200": response_schema(json!({ "type": "object" }))
                            }
                        }
                    }
                }
            }
        }
    }))
    .expect_err("component callback operationIds must fail before lowerability checks")
    .to_string();

    assert!(
        error.contains("duplicate OpenAPI operationId 'createPet'"),
        "{error}"
    );
    assert!(
        error.contains(
            "#/components/callbacks/PetCreated/{$request.body#~1callbackUrl}/post/operationId"
        ),
        "{error}"
    );
    assert!(!error.contains("component collection"), "{error}");
}

#[test]
fn openapi_documents_reject_duplicate_component_path_item_operation_ids() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "post": {
                    "operationId": "createPet",
                    "responses": {
                        "201": response_schema(json!({ "type": "object" }))
                    }
                }
            }
        },
        "components": {
            "pathItems": {
                "Pets": {
                    "get": {
                        "operationId": "createPet",
                        "responses": {
                            "200": response_schema(json!({ "type": "object" }))
                        }
                    }
                }
            }
        }
    }))
    .expect_err("component path-item operationIds must fail before lowerability checks")
    .to_string();

    assert!(
        error.contains("duplicate OpenAPI operationId 'createPet'"),
        "{error}"
    );
    assert!(
        error.contains("#/components/pathItems/Pets/get/operationId"),
        "{error}"
    );
    assert!(!error.contains("component collection"), "{error}");
}

#[test]
fn compatibility_rejects_operation_reference_objects_as_invalid_openapi() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "get": {
                    "$ref": "#/components/pathItems/PetsGet"
                }
            }
        }
    }))
    .expect_err("operation slots only accept Operation Objects")
    .to_string();

    assert!(error.contains("#/paths/~1pets/get/$ref"), "{error}");
    assert!(
        error.contains("supported OpenAPI operation field"),
        "{error}"
    );
}

#[test]
fn compatibility_rejects_media_type_encoding_until_it_is_compared() {
    let error = compat_error(
        spec(json!({
            "requestBody": {
                "required": true,
                "content": {
                    "multipart/form-data": {
                        "schema": {
                            "type": "object",
                            "properties": {
                                "file": { "type": "string" }
                            }
                        },
                        "encoding": {
                            "file": {
                                "contentType": "application/octet-stream"
                            }
                        }
                    }
                }
            },
            "responses": {
                "200": response_schema(json!({ "type": "object" }))
            }
        })),
        spec(get_operation()),
    );

    assert!(
        error.contains("#/paths/~1pets/get/requestBody/content/multipart~1form-data/encoding"),
        "{error}"
    );
    assert!(
        error.contains("OpenAPI compatibility checks do not support media-type encoding"),
        "{error}"
    );
}

#[test]
fn openapi_documents_reject_media_type_reference_objects_before_lowering() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "post": {
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "$ref": "#/components/mediaTypes/PetPayload"
                            }
                        }
                    },
                    "responses": {
                        "200": response_schema(json!({ "type": "object" }))
                    }
                }
            }
        }
    }))
    .expect_err("media-type content entries only accept Media Type Objects")
    .to_string();

    assert!(
        error.contains("#/paths/~1pets/post/requestBody/content/application~1json/$ref"),
        "{error}"
    );
    assert!(
        error.contains("supported OpenAPI media-type field"),
        "{error}"
    );
}

#[test]
fn openapi_documents_reject_invalid_media_type_encoding_before_lowering() {
    for (encoding, pointer_fragment) in [
        (
            json!([]),
            "#/paths/~1pets/get/requestBody/content/multipart~1form-data/encoding",
        ),
        (
            json!({
                "file": []
            }),
            "#/paths/~1pets/get/requestBody/content/multipart~1form-data/encoding/file",
        ),
        (
            json!({
                "file": {
                    "explode": "sometimes"
                }
            }),
            "#/paths/~1pets/get/requestBody/content/multipart~1form-data/encoding/file/explode",
        ),
        (
            json!({
                "file": {
                    "style": "bogus"
                }
            }),
            "#/paths/~1pets/get/requestBody/content/multipart~1form-data/encoding/file/style",
        ),
        (
            json!({
                "file": {
                    "contentType": "*/json"
                }
            }),
            "#/paths/~1pets/get/requestBody/content/multipart~1form-data/encoding/file/contentType",
        ),
        (
            json!({
                "file": {
                    "headers": {
                        "X-Trace": []
                    }
                }
            }),
            "#/paths/~1pets/get/requestBody/content/multipart~1form-data/encoding/file/headers/X-Trace",
        ),
    ] {
        let error = OpenApiDocument::from_json(&json!({
            "openapi": "3.1.0",
            "info": { "title": "Pets", "version": "1.0.0" },
            "paths": {
                "/pets": {
                    "get": {
                        "requestBody": {
                            "required": true,
                            "content": {
                                "multipart/form-data": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "file": { "type": "string" }
                                        }
                                    },
                                    "encoding": encoding
                                }
                            }
                        },
                        "responses": {
                            "200": response_schema(json!({ "type": "object" }))
                        }
                    }
                }
            }
        }))
        .expect_err("invalid media-type encoding must fail during document validation")
        .to_string();

        assert!(
            error.contains(pointer_fragment),
            "{pointer_fragment}: {error}"
        );
        assert!(!error.contains("media-type encoding"), "{error}");
    }
}

#[test]
fn openapi_documents_reject_encoding_keys_missing_from_inline_media_schemas() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "get": {
                    "requestBody": {
                        "required": true,
                        "content": {
                            "multipart/form-data": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "file": { "type": "string" }
                                    }
                                },
                                "encoding": {
                                    "missing": {}
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": response_schema(json!({ "type": "object" }))
                    }
                }
            }
        }
    }))
    .expect_err("encoding entries must name inline media-schema properties")
    .to_string();

    assert!(
        error.contains(
            "#/paths/~1pets/get/requestBody/content/multipart~1form-data/encoding/missing"
        ) && error.contains("a property declared by the media type schema"),
        "{error}"
    );
}

#[test]
fn openapi_documents_reject_encoding_keys_missing_from_referenced_media_schemas_before_lowering() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "components": {
            "schemas": {
                "Upload": {
                    "type": "object",
                    "properties": {
                        "file": { "type": "string" }
                    }
                }
            }
        },
        "paths": {
            "/pets": {
                "get": {
                    "requestBody": {
                        "required": true,
                        "content": {
                            "multipart/form-data": {
                                "schema": {
                                    "$ref": "#/components/schemas/Upload"
                                },
                                "encoding": {
                                    "missing": {}
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": response_schema(json!({ "type": "object" }))
                    }
                }
            }
        }
    }))
    .expect_err("encoding entries must name referenced media-schema properties during validation")
    .to_string();

    assert!(
        error.contains(
            "#/paths/~1pets/get/requestBody/content/multipart~1form-data/encoding/missing"
        ) && error.contains("a property declared by the media type schema"),
        "{error}"
    );
}

#[test]
fn openapi_documents_reject_component_request_body_encoding_keys_missing_from_referenced_media_schemas_before_lowering()
 {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "components": {
            "schemas": {
                "Upload": {
                    "type": "object",
                    "properties": {
                        "file": { "type": "string" }
                    }
                }
            },
            "requestBodies": {
                "UploadBody": {
                    "content": {
                        "multipart/form-data": {
                            "schema": {
                                "$ref": "#/components/schemas/Upload"
                            },
                            "encoding": {
                                "missing": {}
                            }
                        }
                    }
                }
            }
        },
        "paths": {
            "/pets": {
                "get": {
                    "responses": {
                        "200": response_schema(json!({ "type": "object" }))
                    }
                }
            }
        }
    }))
    .expect_err("component request bodies must validate referenced media-schema properties")
    .to_string();

    assert!(
        error.contains(
            "#/components/requestBodies/UploadBody/content/multipart~1form-data/encoding/missing"
        ) && error.contains("a property declared by the media type schema"),
        "{error}"
    );
}

#[test]
fn openapi_documents_reject_media_type_encoding_outside_request_body_form_content() {
    for (document, pointer_fragment) in [
        (
            json!({
                "openapi": "3.1.0",
                "info": { "title": "Pets", "version": "1.0.0" },
                "paths": {
                    "/pets": {
                        "get": {
                            "responses": {
                                "200": {
                                    "description": "Ok",
                                    "content": {
                                        "application/json": {
                                            "encoding": {
                                                "payload": {}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }),
            "#/paths/~1pets/get/responses/200/content/application~1json/encoding",
        ),
        (
            json!({
                "openapi": "3.1.0",
                "info": { "title": "Pets", "version": "1.0.0" },
                "paths": {
                    "/pets": {
                        "get": {
                            "parameters": [{
                                "name": "filter",
                                "in": "query",
                                "content": {
                                    "application/json": {
                                        "encoding": {
                                            "payload": {}
                                        }
                                    }
                                }
                            }],
                            "responses": {
                                "200": response_schema(json!({ "type": "object" }))
                            }
                        }
                    }
                }
            }),
            "#/paths/~1pets/get/parameters/0/content/application~1json/encoding",
        ),
        (
            json!({
                "openapi": "3.1.0",
                "info": { "title": "Pets", "version": "1.0.0" },
                "paths": {
                    "/pets": {
                        "get": {
                            "responses": {
                                "200": {
                                    "description": "Ok",
                                    "headers": {
                                        "X-Payload": {
                                            "content": {
                                                "application/json": {
                                                    "encoding": {
                                                        "payload": {}
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
            }),
            "#/paths/~1pets/get/responses/200/headers/X-Payload/content/application~1json/encoding",
        ),
        (
            json!({
                "openapi": "3.1.0",
                "info": { "title": "Pets", "version": "1.0.0" },
                "paths": {
                    "/pets": {
                        "post": {
                            "requestBody": {
                                "required": true,
                                "content": {
                                    "application/json": {
                                        "encoding": {
                                            "payload": {}
                                        }
                                    }
                                }
                            },
                            "responses": {
                                "200": response_schema(json!({ "type": "object" }))
                            }
                        }
                    }
                }
            }),
            "#/paths/~1pets/post/requestBody/content/application~1json/encoding",
        ),
    ] {
        let error = OpenApiDocument::from_json(&document)
            .expect_err("invalid encoding placement must fail during document validation")
            .to_string();

        assert!(
            error.contains(pointer_fragment),
            "{pointer_fragment}: {error}"
        );
        assert!(
            error.contains(
                "request-body content with media type `multipart/*` or `application/x-www-form-urlencoded`"
            ),
            "{error}"
        );
        assert!(!error.contains("media-type encoding"), "{error}");
    }
}

#[test]
fn compatibility_accepts_parameter_metadata_outside_the_contract_model() {
    for (field, value) in [
        ("description", json!("Cursor")),
        ("deprecated", json!(true)),
        ("example", json!("abc")),
        ("examples", json!({ "default": { "value": "abc" } })),
    ] {
        let old = spec(json!({
            "parameters": [{
                "name": "cursor",
                "in": "query",
                "schema": { "type": "string" },
                (field): value
            }],
            "responses": {
                "200": response_schema(json!({ "type": "object" }))
            }
        }));
        let new = spec(json!({
            "parameters": [{
                "name": "cursor",
                "in": "query",
                "schema": { "type": "string" }
            }],
            "responses": {
                "200": response_schema(json!({ "type": "object" }))
            }
        }));

        assert!(report(old, new).is_compatible(), "{field}");
    }
}

#[test]
fn openapi_documents_reject_parameter_example_and_examples_together() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "get": {
                    "parameters": [{
                        "name": "cursor",
                        "in": "query",
                        "schema": { "type": "string" },
                        "example": "abc",
                        "examples": { "default": { "value": "abc" } }
                    }],
                    "responses": {
                        "200": response_schema(json!({ "type": "object" }))
                    }
                }
            }
        }
    }))
    .expect_err("parameter example metadata conflicts must fail during document validation")
    .to_string();

    assert!(
        error.contains("#/paths/~1pets/get/parameters/0/examples")
            && error.contains("absent when `example` is present"),
        "{error}"
    );
}

#[test]
fn compatibility_accepts_request_body_metadata_outside_the_contract_model() {
    let old = spec(json!({
        "requestBody": {
            "description": "Create a pet",
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

    assert!(report(old, new).is_compatible());
}

#[test]
fn compatibility_rejects_unsupported_response_links_before_lowering() {
    let error = compat_error(
        spec(json!({
            "operationId": "getPet",
            "responses": {
                "200": {
                    "description": "ok",
                    "links": {
                        "self": {
                            "operationId": "getPet"
                        }
                    }
                }
            }
        })),
        spec(get_operation()),
    );

    assert!(
        error.contains("#/paths/~1pets/get/responses/200/links"),
        "{error}"
    );
    assert!(error.contains("response links"), "{error}");
}

#[test]
fn openapi_documents_reject_invalid_response_links_before_lowering() {
    for (links, pointer_fragment) in [
        (json!([]), "#/paths/~1pets/get/responses/200/links"),
        (
            json!({
                "self": []
            }),
            "#/paths/~1pets/get/responses/200/links/self",
        ),
        (
            json!({
                "self": {
                    "description": "missing target"
                }
            }),
            "#/paths/~1pets/get/responses/200/links/self",
        ),
        (
            json!({
                "self": {
                    "operationRef": "#/paths/~1pets/get",
                    "operationId": "listPets"
                }
            }),
            "#/paths/~1pets/get/responses/200/links/self",
        ),
        (
            json!({
                "bad name": {
                    "operationId": "getPet"
                }
            }),
            "#/paths/~1pets/get/responses/200/links/bad name",
        ),
    ] {
        let error = OpenApiDocument::from_json(&json!({
            "openapi": "3.1.0",
            "info": { "title": "Pets", "version": "1.0.0" },
            "paths": {
                "/pets": {
                    "get": {
                        "responses": {
                            "200": {
                                "description": "ok",
                                "links": links
                            }
                        }
                    }
                }
            }
        }))
        .expect_err("invalid response links must fail during document validation")
        .to_string();

        assert!(
            error.contains(pointer_fragment),
            "{pointer_fragment}: {error}"
        );
        assert!(!error.contains("response links"), "{error}");
        if pointer_fragment.ends_with("/bad name") {
            assert!(error.contains("^[a-zA-Z0-9._-]+$"), "{error}");
        }
    }
}

#[test]
fn openapi_documents_reject_response_links_to_missing_operation_ids_before_lowering() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "get": {
                    "operationId": "listPets",
                    "responses": {
                        "200": {
                            "description": "ok",
                            "links": {
                                "details": {
                                    "operationId": "getMissingPet"
                                }
                            }
                        }
                    }
                }
            }
        }
    }))
    .expect_err("response-link operationIds must resolve during document validation")
    .to_string();

    assert!(
        error.contains("#/paths/~1pets/get/responses/200/links/details/operationId")
            && error.contains("an existing OpenAPI operationId"),
        "{error}"
    );
    assert!(!error.contains("response links"), "{error}");
}

#[test]
fn openapi_documents_reject_component_links_to_missing_operation_ids_before_lowering() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "get": {
                    "operationId": "listPets",
                    "responses": {
                        "200": response_schema(json!({ "type": "object" }))
                    }
                }
            }
        },
        "components": {
            "links": {
                "details": {
                    "operationId": "getMissingPet"
                }
            }
        }
    }))
    .expect_err("component Link Object operationIds must resolve during document validation")
    .to_string();

    assert!(
        error.contains("#/components/links/details/operationId")
            && error.contains("an existing OpenAPI operationId"),
        "{error}"
    );
    assert!(!error.contains("component collection"), "{error}");
}

#[test]
fn openapi_documents_accept_local_link_operation_refs_to_operations_before_lowering() {
    OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/users/{id}": {
                "get": {
                    "parameters": [{
                        "name": "id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }],
                    "responses": {
                        "200": {
                            "description": "ok",
                            "links": {
                                "self": {
                                    "operationRef": "#/paths/~1users~1%7Bid%7D/get"
                                }
                            }
                        }
                    }
                }
            }
        }
    }))
    .expect("local link operationRefs should resolve through canonical JSON Pointer decoding");
}

#[test]
fn openapi_documents_reject_local_link_operation_refs_to_non_operations_before_lowering() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "get": {
                    "responses": {
                        "200": {
                            "description": "ok",
                            "links": {
                                "self": {
                                    "operationRef": "#/paths/~1pets/get/responses/200"
                                }
                            }
                        }
                    }
                }
            }
        }
    }))
    .expect_err("local link operationRefs must resolve to operation objects")
    .to_string();

    assert!(
        error.contains("#/paths/~1pets/get/responses/200/links/self/operationRef")
            && error.contains("a local reference to an existing OpenAPI operation"),
        "{error}"
    );
    assert!(!error.contains("response links"), "{error}");
}

#[test]
fn openapi_documents_reject_malformed_local_link_operation_refs_before_lowering() {
    for operation_ref in ["#/paths/~2pets/get", "#not-a-json-pointer"] {
        let error = OpenApiDocument::from_json(&json!({
            "openapi": "3.1.0",
            "info": { "title": "Pets", "version": "1.0.0" },
            "paths": {
                "/pets": {
                    "get": {
                        "responses": {
                            "200": {
                                "description": "ok",
                                "links": {
                                    "self": {
                                        "operationRef": operation_ref
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }))
        .expect_err("fragment-only link operationRefs must resolve to local operations")
        .to_string();

        assert!(
            error.contains("#/paths/~1pets/get/responses/200/links/self/operationRef")
                && error.contains("a local reference to an existing OpenAPI operation"),
            "{operation_ref}: {error}"
        );
        assert!(!error.contains("response links"), "{error}");
    }
}

#[test]
fn openapi_documents_reject_invalid_link_operation_refs_before_lowering() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "get": {
                    "responses": {
                        "200": {
                            "description": "ok",
                            "links": {
                                "self": {
                                    "operationRef": "http://["
                                }
                            }
                        }
                    }
                }
            }
        }
    }))
    .expect_err("link operationRef must be a syntactically valid URI reference")
    .to_string();

    assert!(
        error.contains("#/paths/~1pets/get/responses/200/links/self/operationRef"),
        "{error}"
    );
    assert!(error.contains("a valid URI reference"), "{error}");
}

#[test]
fn compatibility_rejects_path_item_refs_as_unsupported_before_lowering() {
    let error = compat_error(
        json!({
            "openapi": "3.1.0",
            "info": { "title": "Pets", "version": "1.0.0" },
            "paths": {
                "/pets": {
                    "$ref": "#/components/pathItems/Pets"
                }
            },
            "components": {}
        }),
        spec(get_operation()),
    );

    assert!(error.contains("#/paths/~1pets/$ref"), "{error}");
    assert!(error.contains("path item references"), "{error}");
}

#[test]
fn compatibility_rejects_path_item_refs_with_siblings_as_unsupported_before_lowering() {
    let error = compat_error(
        json!({
            "openapi": "3.1.0",
            "info": { "title": "Pets", "version": "1.0.0" },
            "paths": {
                "/pets": {
                    "$ref": "#/components/pathItems/Pets",
                    "get": get_operation()
                }
            }
        }),
        spec(get_operation()),
    );

    assert!(error.contains("#/paths/~1pets/$ref"), "{error}");
    assert!(error.contains("path item references"), "{error}");
}

#[test]
fn openapi_documents_validate_path_item_ref_siblings_before_lowering() {
    for (path_item, pointer_fragment) in [
        (
            json!({
                "$ref": "#/components/pathItems/Pets",
                "servers": "https://api.example.com"
            }),
            "#/paths/~1pets/servers",
        ),
        (
            json!({
                "$ref": "#/components/pathItems/Pets",
                "get": []
            }),
            "#/paths/~1pets/get",
        ),
    ] {
        let error = OpenApiDocument::from_json(&json!({
            "openapi": "3.1.0",
            "info": { "title": "Pets", "version": "1.0.0" },
            "paths": {
                "/pets": path_item
            }
        }))
        .expect_err("path-item reference siblings must still be valid OpenAPI")
        .to_string();

        assert!(
            error.contains(pointer_fragment),
            "{pointer_fragment}: {error}"
        );
    }
}

#[test]
fn compatibility_rejects_invalid_path_item_ref_shapes_before_lowering() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "$ref": 42
            }
        }
    }))
    .expect_err("malformed path-item refs must fail document-shape validation")
    .to_string();

    assert!(error.contains("#/paths/~1pets/$ref"), "{error}");
    assert!(error.contains("a string"), "{error}");
}

#[test]
fn compatibility_accepts_response_header_metadata_outside_the_contract_model() {
    let old = spec(json!({
        "responses": {
            "200": {
                "description": "ok",
                "headers": {
                    "X-Trace-Id": {
                        "description": "Trace identifier",
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

    assert!(report(old, new).is_compatible());
}

#[test]
fn openapi_documents_reject_response_header_example_and_examples_together() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "get": {
                    "responses": {
                        "200": {
                            "description": "ok",
                            "headers": {
                                "X-Trace-Id": {
                                    "schema": { "type": "string" },
                                    "example": "abc",
                                    "examples": {
                                        "default": {
                                            "value": "abc"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }))
    .expect_err("response-header example metadata conflicts must fail during document validation")
    .to_string();

    assert!(
        error.contains("#/paths/~1pets/get/responses/200/headers/X-Trace-Id/examples")
            && error.contains("absent when `example` is present"),
        "{error}"
    );
}

#[test]
fn compatibility_accepts_media_type_examples_outside_the_contract_model() {
    for field in ["example", "examples"] {
        let old = spec(json!({
            "requestBody": {
                "content": {
                    "application/json": {
                        "schema": { "type": "object" },
                        (field): if field == "example" {
                            json!({ "id": 1 })
                        } else {
                            json!({ "sample": { "value": { "id": 1 } } })
                        }
                    }
                }
            },
            "responses": {
                "200": response_schema(json!({ "type": "object" }))
            }
        }));
        let new = spec(json!({
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

        assert!(report(old, new).is_compatible(), "{field}");
    }
}

#[test]
fn openapi_documents_reject_media_type_example_and_examples_together() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "get": {
                    "requestBody": {
                        "content": {
                            "application/json": {
                                "schema": { "type": "object" },
                                "example": { "id": 1 },
                                "examples": {
                                    "sample": {
                                        "value": { "id": 1 }
                                    }
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": response_schema(json!({ "type": "object" }))
                    }
                }
            }
        }
    }))
    .expect_err("media-type example metadata conflicts must fail during document validation")
    .to_string();

    assert!(
        error.contains("#/paths/~1pets/get/requestBody/content/application~1json/examples")
            && error.contains("absent when `example` is present"),
        "{error}"
    );
}

#[test]
fn openapi_documents_reject_invalid_media_type_example_entries_before_lowering() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "get": {
                    "requestBody": {
                        "content": {
                            "application/json": {
                                "schema": { "type": "object" },
                                "examples": {
                                    "broken": 42
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": response_schema(json!({ "type": "object" }))
                    }
                }
            }
        }
    }))
    .expect_err("invalid media-type example entries must fail during document validation")
    .to_string();

    assert!(
        error.contains("#/paths/~1pets/get/requestBody/content/application~1json/examples/broken")
            && error.contains("Example Object or Reference Object"),
        "{error}"
    );
}

#[test]
fn openapi_documents_reject_example_value_and_external_value_together() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "get": {
                    "requestBody": {
                        "content": {
                            "application/json": {
                                "schema": { "type": "object" },
                                "examples": {
                                    "sample": {
                                        "value": { "id": 1 },
                                        "externalValue": "https://example.com/pets/1.json"
                                    }
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": response_schema(json!({ "type": "object" }))
                    }
                }
            }
        }
    }))
    .expect_err("invalid example payload conflicts must fail during document validation")
    .to_string();

    assert!(
        error.contains("#/paths/~1pets/get/requestBody/content/application~1json/examples/sample")
            && error.contains("at most one of `value` or `externalValue`"),
        "{error}"
    );
}

#[test]
fn openapi_documents_reject_invalid_example_external_value_uris_before_lowering() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "get": {
                    "requestBody": {
                        "content": {
                            "application/json": {
                                "schema": { "type": "object" },
                                "examples": {
                                    "sample": {
                                        "externalValue": "http://["
                                    }
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": response_schema(json!({ "type": "object" }))
                    }
                }
            }
        }
    }))
    .expect_err("example externalValue must be a syntactically valid URI reference")
    .to_string();

    assert!(
        error.contains(
            "#/paths/~1pets/get/requestBody/content/application~1json/examples/sample/externalValue"
        ),
        "{error}"
    );
    assert!(error.contains("a valid URI reference"), "{error}");
}

#[test]
fn openapi_documents_reject_invalid_body_content_media_type_keys_before_lowering() {
    for (contract, pointer_fragment) in [
        (
            json!({
                "requestBody": {
                    "content": {
                        "*/json": {
                            "schema": { "type": "string" }
                        }
                    }
                },
                "responses": {
                    "204": { "description": "ok" }
                }
            }),
            "#/paths/~1pets/get/requestBody/content/*~1json",
        ),
        (
            json!({
                "responses": {
                    "200": {
                        "description": "ok",
                        "content": {
                            "*/json": {
                                "schema": { "type": "string" }
                            }
                        }
                    }
                }
            }),
            "#/paths/~1pets/get/responses/200/content/*~1json",
        ),
    ] {
        let error = OpenApiDocument::from_json(&spec(contract))
            .expect_err("body content keys must be valid OpenAPI media types during validation")
            .to_string();

        assert!(
            error.contains(pointer_fragment),
            "{pointer_fragment}: {error}"
        );
        assert!(
            error.contains("a valid concrete media type or OpenAPI media-type range"),
            "{error}"
        );
    }
}

#[test]
fn compatibility_rejects_request_media_types_that_collapse_to_the_same_selector() {
    let spec = spec(json!({
        "requestBody": {
            "required": true,
            "content": {
                "application/json": {
                    "schema": { "type": "integer" }
                },
                "application/json; charset=utf-8": {
                    "schema": { "type": "string" }
                }
            }
        },
        "responses": {
            "204": { "description": "ok" }
        }
    }));

    let error = compat_error(spec.clone(), spec);

    assert!(
        error.contains("#/paths/~1pets/get/requestBody/content/application~1json; charset=utf-8"),
        "{error}"
    );
    assert!(
        error.contains("collapse to the same compatibility selector 'application/json'"),
        "{error}"
    );
}

#[test]
fn compatibility_rejects_response_media_types_that_collapse_to_the_same_selector() {
    let spec = spec(json!({
        "responses": {
            "200": {
                "description": "ok",
                "content": {
                    "application/json": {
                        "schema": { "type": "integer" }
                    },
                    "Application/JSON": {
                        "schema": { "type": "string" }
                    }
                }
            }
        }
    }));

    let error = compat_error(spec.clone(), spec);

    assert!(
        error.contains("#/paths/~1pets/get/responses/200/content/Application~1JSON")
            || error.contains("#/paths/~1pets/get/responses/200/content/application~1json"),
        "{error}"
    );
    assert!(
        error.contains("collapse to the same compatibility selector 'application/json'"),
        "{error}"
    );
}

#[test]
fn widening_a_request_media_type_range_is_compatible() {
    let old = spec(json!({
        "requestBody": {
            "required": true,
            "content": {
                "image/png": {
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
                "image/*": {
                    "schema": { "type": "object" }
                }
            }
        },
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    }));

    assert!(report(old, new).is_compatible());
}

#[test]
fn widening_a_request_media_type_range_to_a_global_wildcard_is_compatible() {
    let old = spec(json!({
        "requestBody": {
            "required": true,
            "content": {
                "image/*": {
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
                "*/*": {
                    "schema": { "type": "object" }
                }
            }
        },
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    }));

    assert!(report(old, new).is_compatible());
}

#[test]
fn narrowing_a_request_media_type_range_is_incompatible() {
    let old = spec(json!({
        "requestBody": {
            "required": true,
            "content": {
                "image/*": {
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
                "image/png": {
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
fn overriding_a_request_media_type_range_with_a_narrower_exact_schema_is_incompatible() {
    let old = spec(json!({
        "requestBody": {
            "required": true,
            "content": {
                "application/*": {
                    "schema": { "type": "string" }
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
                "application/*": {
                    "schema": { "type": "string" }
                },
                "application/json": {
                    "schema": { "type": "integer" }
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
fn widening_a_response_media_type_range_is_incompatible() {
    let old = spec(json!({
        "responses": {
            "200": {
                "description": "ok",
                "content": {
                    "image/png": {
                        "schema": { "type": "object" }
                    }
                }
            }
        }
    }));
    let new = spec(json!({
        "responses": {
            "200": {
                "description": "ok",
                "content": {
                    "image/*": {
                        "schema": { "type": "object" }
                    }
                }
            }
        }
    }));

    assert!(!report(old, new).is_compatible());
}

#[test]
fn widening_a_response_media_type_range_to_a_global_wildcard_is_incompatible() {
    let old = spec(json!({
        "responses": {
            "200": {
                "description": "ok",
                "content": {
                    "image/*": {
                        "schema": { "type": "object" }
                    }
                }
            }
        }
    }));
    let new = spec(json!({
        "responses": {
            "200": {
                "description": "ok",
                "content": {
                    "*/*": {
                        "schema": { "type": "object" }
                    }
                }
            }
        }
    }));

    assert!(!report(old, new).is_compatible());
}

#[test]
fn narrowing_a_response_media_type_range_is_compatible() {
    let old = spec(json!({
        "responses": {
            "200": {
                "description": "ok",
                "content": {
                    "image/*": {
                        "schema": { "type": "object" }
                    }
                }
            }
        }
    }));
    let new = spec(json!({
        "responses": {
            "200": {
                "description": "ok",
                "content": {
                    "image/png": {
                        "schema": { "type": "object" }
                    }
                }
            }
        }
    }));

    assert!(report(old, new).is_compatible());
}

#[test]
fn removing_an_exact_response_media_type_can_be_incompatible_when_global_and_type_ranges_take_over()
{
    let old = spec(json!({
        "responses": {
            "200": {
                "description": "ok",
                "content": {
                    "*/*": {
                        "schema": { "type": "string" }
                    },
                    "application/*": {
                        "schema": { "type": "boolean" }
                    },
                    "application/json": {
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
                "content": {
                    "*/*": {
                        "schema": { "type": "string" }
                    },
                    "application/*": {
                        "schema": { "type": "boolean" }
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
fn removing_an_exact_response_media_type_can_be_incompatible_when_a_range_takes_over() {
    let old = spec(json!({
        "responses": {
            "200": {
                "description": "ok",
                "content": {
                    "application/*": {
                        "schema": { "type": "string" }
                    },
                    "application/json": {
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
                "content": {
                    "application/*": {
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
fn media_type_type_and_subtype_casing_do_not_change_compatibility() {
    let old = spec(json!({
        "responses": {
            "200": {
                "description": "ok",
                "content": {
                    "Application/JSON": {
                        "schema": { "type": "object" }
                    }
                }
            }
        }
    }));
    let new = spec(json!({
        "responses": {
            "200": {
                "description": "ok",
                "content": {
                    "application/json": {
                        "schema": { "type": "object" }
                    }
                }
            }
        }
    }));

    assert!(report(old, new).is_compatible());
}

#[test]
fn media_type_parameters_do_not_change_compatibility() {
    let old = spec(json!({
        "responses": {
            "200": {
                "description": "ok",
                "content": {
                    "text/plain; charset=utf-8": {
                        "schema": { "type": "object" }
                    }
                }
            }
        }
    }));
    let new = spec(json!({
        "responses": {
            "200": {
                "description": "ok",
                "content": {
                    "text/plain; charset=us-ascii": {
                        "schema": { "type": "object" }
                    }
                }
            }
        }
    }));

    assert!(report(old, new).is_compatible());
}

#[test]
fn compatibility_rejects_schema_keywords_without_openapi_compatibility_semantics() {
    for keyword in [
        "additionalItems",
        "contentEncoding",
        "contentMediaType",
        "contentSchema",
        "dependencies",
        "dependentSchemas",
        "unevaluatedItems",
        "unevaluatedProperties",
    ] {
        let schema = match keyword {
            "additionalItems" => json!({ (keyword): false }),
            "contentEncoding" => json!({ "type": "string", (keyword): "base64" }),
            "contentMediaType" => {
                json!({ "type": "string", (keyword): "application/json" })
            }
            "contentSchema" => json!({ "type": "string", (keyword): { "type": "object" } }),
            "dependencies" => {
                json!({ "type": "object", (keyword): { "kind": ["detail"] } })
            }
            "dependentSchemas" => {
                json!({ "type": "object", (keyword): { "kind": { "type": "string" } } })
            }
            "unevaluatedItems" => json!({ "type": "array", (keyword): false }),
            "unevaluatedProperties" => json!({ "type": "object", (keyword): false }),
            _ => unreachable!("covered keyword cases"),
        };
        let error = compat_error(
            spec(json!({
                "requestBody": {
                    "content": {
                        "application/json": {
                            "schema": schema
                        }
                    }
                },
                "responses": {
                    "200": response_schema(json!({ "type": "object" }))
                }
            })),
            spec(get_operation()),
        );

        assert!(error.contains(keyword), "{keyword}: {error}");
        assert!(
            error.contains("OpenAPI compatibility checks do not support JSON Schema keyword"),
            "{keyword}: {error}"
        );
        assert!(
            error.contains(&format!(
                "#/paths/~1pets/get/requestBody/content/application~1json/schema/{keyword}"
            )),
            "{keyword}: {error}"
        );
    }
}

#[test]
fn compatibility_rejects_schema_reference_keywords_without_openapi_compatibility_semantics() {
    for (keyword, value) in [
        ("$id", json!("https://example.com/schemas/request.json")),
        ("$anchor", json!("request")),
        ("$dynamicRef", json!("#")),
        ("$dynamicAnchor", json!("request")),
    ] {
        let error = compat_error(
            spec(json!({
                "requestBody": {
                    "content": {
                        "application/json": {
                            "schema": {
                                "type": "string",
                                (keyword): value
                            }
                        }
                    }
                },
                "responses": {
                    "200": response_schema(json!({ "type": "object" }))
                }
            })),
            spec(get_operation()),
        );

        assert!(error.contains(keyword), "{keyword}: {error}");
        assert!(
            error.contains("OpenAPI compatibility checks do not support JSON Schema keyword"),
            "{keyword}: {error}"
        );
        assert!(
            error.contains(&format!(
                "#/paths/~1pets/get/requestBody/content/application~1json/schema/{keyword}"
            )),
            "{keyword}: {error}"
        );
    }
}

#[test]
fn compatibility_rejects_number_bounds_outside_the_exact_f64_integer_range() {
    let error = compat_error(
        spec(json!({
            "requestBody": {
                "content": {
                    "application/json": {
                        "schema": {
                            "type": "number",
                            "maximum": 9_007_199_254_740_992_i64
                        }
                    }
                }
            },
            "responses": {
                "200": response_schema(json!({ "type": "object" }))
            }
        })),
        spec(get_operation()),
    );

    assert!(
        error.contains(
            "JSON Schema number bounds outside the exact f64 integer range [-9007199254740991, 9007199254740991]"
        ),
        "{error}"
    );
    assert!(
        error.contains("#/paths/~1pets/get/requestBody/content/application~1json/schema/maximum"),
        "{error}"
    );
}

#[test]
fn compatibility_keeps_openapi_format_annotations_outside_the_contract_model() {
    let old = spec(json!({
        "responses": {
            "200": response_schema(json!({
                "type": "string",
                "format": "email"
            }))
        }
    }));
    let new = spec(json!({
        "responses": {
            "200": response_schema(json!({
                "type": "string",
                "format": "uuid"
            }))
        }
    }));

    assert!(report(old, new).is_compatible());
}

#[test]
fn compatibility_keeps_const_payload_schema_like_keys_as_data() {
    let contract = spec(json!({
        "requestBody": {
            "required": true,
            "content": {
                "application/json": {
                    "schema": {
                        "const": {
                            "$ref": "literal",
                            "contentEncoding": "literal",
                            "dependentSchemas": "literal"
                        }
                    }
                }
            }
        },
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    }));

    assert!(report(contract.clone(), contract).is_compatible());
}

#[test]
fn compatibility_accepts_read_write_annotations_outside_the_contract_model() {
    let annotated = spec(json!({
        "requestBody": {
            "content": {
                "application/json": {
                    "schema": {
                        "type": "object",
                        "properties": {
                            "server_id": {
                                "type": "string",
                                "readOnly": true
                            },
                            "secret": {
                                "type": "string",
                                "writeOnly": true
                            }
                        }
                    }
                }
            }
        },
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    }));

    assert!(report(annotated.clone(), annotated).is_compatible());
}

#[test]
fn openapi_documents_reject_invalid_read_write_annotation_shapes_before_lowering() {
    for keyword in ["readOnly", "writeOnly"] {
        let error = OpenApiDocument::from_json(&spec(json!({
            "requestBody": {
                "content": {
                    "application/json": {
                        "schema": {
                            "type": "string",
                            (keyword): "yes"
                        }
                    }
                }
            },
            "responses": {
                "200": response_schema(json!({ "type": "object" }))
            }
        })))
        .expect_err("schema read/write annotations must be boolean during document validation")
        .to_string();

        assert!(error.contains(keyword), "{keyword}: {error}");
        assert!(error.contains("a boolean"), "{keyword}: {error}");
    }
}

#[test]
fn compatibility_accepts_schema_external_docs_and_xml_metadata_outside_the_contract_model() {
    let annotated = spec(json!({
        "requestBody": {
            "content": {
                "application/json": {
                    "schema": {
                        "type": "object",
                        "externalDocs": {
                            "url": "https://docs.example.com/schema"
                        },
                        "properties": {
                            "name": {
                                "type": "string",
                                "xml": {
                                    "name": "display-name",
                                    "namespace": "https://example.com/schema",
                                    "prefix": "sample",
                                    "attribute": false
                                }
                            }
                        }
                    }
                }
            }
        },
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    }));

    assert!(report(annotated.clone(), annotated).is_compatible());
}

#[test]
fn openapi_documents_reject_invalid_schema_external_docs_before_lowering() {
    let error = OpenApiDocument::from_json(&spec(json!({
        "requestBody": {
            "content": {
                "application/json": {
                    "schema": {
                        "type": "string",
                        "externalDocs": {
                            "url": 42
                        }
                    }
                }
            }
        },
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    })))
    .expect_err("schema externalDocs metadata must validate before lowering")
    .to_string();

    assert!(
        error.contains(
            "#/paths/~1pets/get/requestBody/content/application~1json/schema/externalDocs/url"
        ) && error.contains("a valid URL reference"),
        "{error}"
    );
}

#[test]
fn openapi_documents_validate_component_schema_metadata_before_lowering() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "components": {
            "schemas": {
                "Broken": {
                    "type": "string",
                    "externalDocs": {
                        "url": 42
                    }
                }
            }
        }
    }))
    .expect_err("component schema metadata must validate before lowering")
    .to_string();

    assert!(
        error.contains("#/components/schemas/Broken/externalDocs/url")
            && error.contains("a valid URL reference"),
        "{error}"
    );
}

#[test]
fn openapi_documents_reject_invalid_schema_xml_before_lowering() {
    let error = OpenApiDocument::from_json(&spec(json!({
        "requestBody": {
            "content": {
                "application/json": {
                    "schema": {
                        "type": "object",
                        "properties": {
                            "name": {
                                "type": "string",
                                "xml": {
                                    "attribute": "yes"
                                }
                            }
                        }
                    }
                }
            }
        },
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    })))
    .expect_err("schema xml metadata must validate before lowering")
    .to_string();

    assert!(
        error.contains(
            "#/paths/~1pets/get/requestBody/content/application~1json/schema/properties/name/xml/attribute"
        ) && error.contains("a boolean"),
        "{error}"
    );
}

#[test]
fn openapi_documents_reject_xml_on_non_property_schemas_before_lowering() {
    let error = OpenApiDocument::from_json(&spec(json!({
        "requestBody": {
            "content": {
                "application/json": {
                    "schema": {
                        "type": "string",
                        "xml": {
                            "attribute": false
                        }
                    }
                }
            }
        },
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    })))
    .expect_err("xml metadata is only valid on property schemas")
    .to_string();

    assert!(
        error.contains("#/paths/~1pets/get/requestBody/content/application~1json/schema/xml")
            && error.contains("absent outside property schemas"),
        "{error}"
    );
}

#[test]
fn openapi_documents_reject_wrapped_xml_on_explicit_non_array_schemas_before_lowering() {
    let error = OpenApiDocument::from_json(&spec(json!({
        "requestBody": {
            "content": {
                "application/json": {
                    "schema": {
                        "type": "object",
                        "properties": {
                            "name": {
                                "type": "string",
                                "xml": {
                                    "wrapped": true
                                }
                            }
                        }
                    }
                }
            }
        },
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    })))
    .expect_err("wrapped xml metadata must match the sibling schema type")
    .to_string();

    assert!(
        error.contains(
            "#/paths/~1pets/get/requestBody/content/application~1json/schema/properties/name/xml/wrapped"
        ) && error.contains("absent unless the sibling schema `type` includes `\"array\"`"),
        "{error}"
    );
}

#[test]
fn openapi_documents_reject_relative_schema_xml_namespaces_before_lowering() {
    let error = OpenApiDocument::from_json(&spec(json!({
        "requestBody": {
            "content": {
                "application/json": {
                    "schema": {
                        "type": "object",
                        "properties": {
                            "name": {
                                "type": "string",
                                "xml": {
                                    "namespace": "../schema"
                                }
                            }
                        }
                    }
                }
            }
        },
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    })))
    .expect_err("schema xml namespace values must validate before lowering")
    .to_string();

    assert!(
        error.contains(
            "#/paths/~1pets/get/requestBody/content/application~1json/schema/properties/name/xml/namespace"
        ) && error.contains("an absolute URI"),
        "{error}"
    );
}

#[test]
fn compatibility_accepts_valid_discriminator_metadata_outside_the_contract_model() {
    let with_discriminator = spec(json!({
        "requestBody": {
            "content": {
                "application/json": {
                    "schema": {
                        "required": ["kind"],
                        "oneOf": [
                            {
                                "type": "object",
                                "properties": {
                                    "kind": { "const": "cat" }
                                },
                                "required": ["kind"]
                            },
                            {
                                "type": "object",
                                "properties": {
                                    "kind": { "const": "dog" }
                                },
                                "required": ["kind"]
                            }
                        ],
                        "discriminator": {
                            "propertyName": "kind",
                            "mapping": {
                                "cat": "#/components/schemas/Cat",
                                "dog": "#/components/schemas/Dog"
                            }
                        }
                    }
                }
            }
        },
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    }));

    assert!(report(with_discriminator.clone(), with_discriminator).is_compatible());
}

#[test]
fn openapi_documents_reject_invalid_discriminator_metadata_before_lowering() {
    let error = OpenApiDocument::from_json(&spec(json!({
        "requestBody": {
            "content": {
                "application/json": {
                    "schema": {
                        "type": "object",
                        "required": ["kind"],
                        "anyOf": [
                            {
                                "type": "object"
                            }
                        ],
                        "discriminator": {
                            "propertyName": "kind",
                            "mapping": {
                                "cat": 42
                            }
                        }
                    }
                }
            }
        },
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    })))
    .expect_err("schema discriminator metadata must validate before lowering")
    .to_string();

    assert!(
        error.contains("discriminator/mapping/cat") && error.contains("a string"),
        "{error}"
    );
}

#[test]
fn compatibility_accepts_discriminators_without_locally_required_properties() {
    let with_discriminator = spec(json!({
        "requestBody": {
            "content": {
                "application/json": {
                    "schema": {
                        "type": "object",
                        "anyOf": [
                            {
                                "type": "object"
                            }
                        ],
                        "discriminator": {
                            "propertyName": "kind"
                        }
                    }
                }
            }
        },
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    }));

    assert!(report(with_discriminator.clone(), with_discriminator).is_compatible());
}

#[test]
fn openapi_documents_reject_discriminators_without_composite_keywords() {
    let error = OpenApiDocument::from_json(&spec(json!({
        "requestBody": {
            "content": {
                "application/json": {
                    "schema": {
                        "type": "object",
                        "discriminator": {
                            "propertyName": "kind"
                        }
                    }
                }
            }
        },
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    })))
    .expect_err("schema discriminators require a neighboring composite keyword")
    .to_string();

    assert!(
        error.contains(
            "#/paths/~1pets/get/requestBody/content/application~1json/schema/discriminator"
        ) && error.contains("adjacent to `oneOf`, `anyOf`, or `allOf`"),
        "{error}"
    );
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
fn operation_level_parameters_override_path_item_parameters() {
    let old = json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "paths": {
            "/pets": {
                "parameters": [{
                    "name": "limit",
                    "in": "query",
                    "schema": { "type": "integer" }
                }],
                "get": get_operation()
            }
        }
    });
    let new = json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "paths": {
            "/pets": {
                "parameters": [{
                    "name": "limit",
                    "in": "query",
                    "schema": { "type": "integer" }
                }],
                "get": {
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
                }
            }
        }
    });

    let report = report(old, new);

    assert!(!report.is_compatible());
    assert_eq!(
        issue_surfaces(&report),
        vec![OpenApiCompatibilitySurface::Request]
    );
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
fn adding_a_required_cookie_parameter_is_incompatible() {
    let old = spec(get_operation());
    let new = spec(json!({
        "parameters": [{
            "name": "session",
            "in": "cookie",
            "required": true,
            "schema": { "type": "string" }
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
fn adding_an_optional_cookie_parameter_is_compatible() {
    let old = spec(get_operation());
    let new = spec(json!({
        "parameters": [{
            "name": "session",
            "in": "cookie",
            "schema": { "type": "string" }
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
fn narrowing_a_path_parameter_schema_is_incompatible() {
    let old = json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "paths": {
            "/pets/{petId}": {
                "get": {
                    "parameters": [{
                        "name": "petId",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }],
                    "responses": {
                        "200": response_schema(json!({ "type": "object" }))
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
        "paths": {
            "/pets/{petId}": {
                "get": {
                    "parameters": [{
                        "name": "petId",
                        "in": "path",
                        "required": true,
                        "schema": {
                            "type": "string",
                            "enum": ["cat"]
                        }
                    }],
                    "responses": {
                        "200": response_schema(json!({ "type": "object" }))
                    }
                }
            }
        }
    });

    let report = report(old, new);

    assert!(!report.is_compatible());
    assert_eq!(
        issue_surfaces(&report),
        vec![OpenApiCompatibilitySurface::Request]
    );
}

#[test]
fn header_parameter_names_compare_case_insensitively() {
    let old = spec(json!({
        "parameters": [{
            "name": "X-Request-Id",
            "in": "header",
            "schema": { "type": "string" }
        }],
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    }));
    let new = spec(json!({
        "parameters": [{
            "name": "x-request-id",
            "in": "header",
            "schema": { "type": "string" }
        }],
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    }));

    assert!(report(old, new).is_compatible());
}

#[test]
fn openapi_documents_reject_case_insensitive_header_parameter_duplicates_before_lowering() {
    let error = OpenApiDocument::from_json(&spec(json!({
        "parameters": [
            {
                "name": "X-Request-Id",
                "in": "header",
                "schema": { "type": "string" }
            },
            {
                "name": "x-request-id",
                "in": "header",
                "schema": { "type": "string" }
            }
        ],
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    })))
    .expect_err("duplicate operation parameters must fail during OpenAPI validation")
    .to_string();

    assert!(error.contains("duplicate OpenAPI parameter"), "{error}");
    assert!(error.contains("headers:x-request-id"), "{error}");
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
fn making_a_request_body_optional_is_compatible() {
    let old = spec(json!({
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
    let new = spec(json!({
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

    assert!(report(old, new).is_compatible());
}

#[test]
fn adding_an_optional_request_body_is_compatible() {
    let old = spec(json!({
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    }));
    let new = spec(json!({
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

    assert!(report(old, new).is_compatible());
}

#[test]
fn adding_a_required_request_body_is_incompatible() {
    let old = spec(json!({
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
fn removing_a_request_body_is_incompatible() {
    let old = spec(json!({
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
    let new = spec(json!({
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
fn adding_a_supported_request_media_type_is_compatible() {
    let old = spec(json!({
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
    let new = spec(json!({
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

    assert!(report(old, new).is_compatible());
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
fn removing_an_optional_response_header_is_compatible_for_serialized_responses() {
    let old = spec(json!({
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
    let new = spec(json!({
        "responses": {
            "200": {
                "description": "ok"
            }
        }
    }));

    assert!(report(old, new).is_compatible());
}

#[test]
fn adding_an_optional_response_header_is_incompatible_for_serialized_responses() {
    let old = spec(json!({
        "responses": {
            "200": {
                "description": "ok"
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
fn removing_a_required_response_header_is_incompatible_for_serialized_responses() {
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
                "description": "ok"
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
fn making_a_required_response_header_optional_is_incompatible_for_serialized_responses() {
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
fn response_header_names_compare_case_insensitively() {
    let old = spec(json!({
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
    let new = spec(json!({
        "responses": {
            "200": {
                "description": "ok",
                "headers": {
                    "x-trace-id": {
                        "schema": { "type": "string" }
                    }
                }
            }
        }
    }));

    assert!(report(old, new).is_compatible());
}

#[test]
fn openapi_documents_reject_case_insensitive_response_header_duplicates_before_lowering() {
    let error = OpenApiDocument::from_json(&spec(json!({
        "responses": {
            "200": {
                "description": "ok",
                "headers": {
                    "X-Trace-Id": {
                        "schema": { "type": "string" }
                    },
                    "x-trace-id": {
                        "schema": { "type": "string" }
                    }
                }
            }
        }
    })))
    .expect_err("duplicate response headers must fail during OpenAPI validation")
    .to_string();

    assert!(
        error.contains("duplicate OpenAPI response header"),
        "{error}"
    );
    assert!(error.contains("x-trace-id"), "{error}");
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
fn enabling_allow_reserved_on_a_query_parameter_is_incompatible() {
    let old = spec(json!({
        "parameters": [{
            "name": "filter",
            "in": "query",
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
            "allowReserved": true,
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
fn changing_query_parameter_style_is_incompatible() {
    let old = spec(json!({
        "parameters": [{
            "name": "filter",
            "in": "query",
            "style": "form",
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
            "style": "pipeDelimited",
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
fn changing_query_parameter_explode_is_incompatible() {
    let old = spec(json!({
        "parameters": [{
            "name": "filter",
            "in": "query",
            "explode": true,
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
            "explode": false,
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
fn parameters_reject_multiple_content_media_types() {
    let error = OpenApiDocument::from_json(&spec(json!({
        "parameters": [{
            "name": "filter",
            "in": "query",
            "content": {
                "application/json": {
                    "schema": { "type": "string" }
                },
                "text/plain": {
                    "schema": { "type": "string" }
                }
            }
        }],
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    })))
    .expect_err("parameter content cardinality is invalid OpenAPI")
    .to_string();

    assert!(
        error.contains("content") && error.contains("exactly one media type"),
        "{error}"
    );
}

#[test]
fn openapi_documents_validate_parameter_content_shapes_before_lowering() {
    for (content, pointer_fragment) in [
        (json!([]), "#/paths/~1pets/get/parameters/0/content"),
        (
            json!({
                "application/json": []
            }),
            "#/paths/~1pets/get/parameters/0/content/application~1json",
        ),
    ] {
        let error = OpenApiDocument::from_json(&spec(json!({
            "parameters": [{
                "name": "filter",
                "in": "query",
                "content": content
            }],
            "responses": {
                "200": response_schema(json!({ "type": "object" }))
            }
        })))
        .expect_err("parameter content shapes must fail during document validation")
        .to_string();

        assert!(
            error.contains(pointer_fragment),
            "{pointer_fragment}: {error}"
        );
    }
}

#[test]
fn openapi_documents_reject_deep_object_parameters_without_explicit_explode_true_before_lowering() {
    let error = OpenApiDocument::from_json(&spec(json!({
        "parameters": [{
            "name": "filter",
            "in": "query",
            "style": "deepObject",
            "schema": { "type": "object" }
        }],
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    })))
    .expect_err("deepObject query parameters must fail during document validation")
    .to_string();

    assert!(
        error.contains("#/paths/~1pets/get/parameters/0/explode") && error.contains("deepObject"),
        "{error}"
    );
}

#[test]
fn openapi_documents_reject_parameter_content_media_type_ranges_before_lowering() {
    let error = OpenApiDocument::from_json(&spec(json!({
        "parameters": [{
            "name": "filter",
            "in": "query",
            "content": {
                "application/*": {
                    "schema": { "type": "string" }
                }
            }
        }],
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    })))
    .expect_err("parameter content requires a concrete media type")
    .to_string();

    assert!(
        error.contains("#/paths/~1pets/get/parameters/0/content/application~1*")
            && error.contains("a valid concrete media type"),
        "{error}"
    );
}

#[test]
fn content_backed_parameters_reject_schema_serialization_fields() {
    for (field, value) in [
        ("style", json!("form")),
        ("explode", json!(true)),
        ("allowReserved", json!(true)),
        ("allowEmptyValue", json!(true)),
        ("example", json!("sample")),
        ("examples", json!({ "sample": { "value": "sample" } })),
    ] {
        let error = OpenApiDocument::from_json(&spec(json!({
            "parameters": [{
                "name": "filter",
                "in": "query",
                (field): value,
                "content": {
                    "application/json": {
                        "schema": { "type": "string" }
                    }
                }
            }],
            "responses": {
                "200": response_schema(json!({ "type": "object" }))
            }
        })))
        .expect_err("content-backed parameter serialization fields are invalid OpenAPI")
        .to_string();

        assert!(error.contains(field), "{field}: {error}");
        assert!(error.contains("content"), "{field}: {error}");
    }
}

#[test]
fn ignored_reserved_header_parameters_do_not_affect_compatibility() {
    let old = spec(json!({
        "parameters": [{
            "name": "Accept",
            "in": "header",
            "schema": { "type": "string" }
        }, {
            "name": "Content-Type",
            "in": "header",
            "required": true,
            "schema": { "type": "string" }
        }, {
            "name": "Authorization",
            "in": "header",
            "schema": { "type": "string" }
        }],
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    }));
    let new = spec(get_operation());

    assert!(report(old, new).is_compatible());
}

#[test]
fn ignored_reserved_header_parameters_still_require_valid_shapes() {
    let error = OpenApiDocument::from_json(&spec(json!({
        "parameters": [{
            "name": "Accept",
            "in": "header"
        }],
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    })))
    .expect_err("ignored reserved header parameters still need valid parameter shapes")
    .to_string();

    assert!(
        error.contains("#/paths/~1pets/get/parameters/0")
            && error.contains("exactly one of `schema` or `content`"),
        "{error}"
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
fn response_content_type_headers_do_not_affect_compatibility() {
    let old = spec(json!({
        "responses": {
            "200": {
                "description": "ok",
                "headers": {
                    "Content-Type": {
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
                    "content-type": {
                        "schema": { "type": ["integer", "string"] }
                    }
                }
            }
        }
    }));

    assert!(report(old, new).is_compatible());
}

#[test]
fn duplicate_response_content_type_headers_are_ignored_for_compatibility() {
    let old = spec(json!({
        "responses": {
            "200": {
                "description": "ok",
                "headers": {
                    "Content-Type": {
                        "schema": { "type": "integer" }
                    },
                    "content-type": {
                        "schema": { "type": "string" }
                    }
                }
            }
        }
    }));
    let new = spec(json!({
        "responses": {
            "200": {
                "description": "ok"
            }
        }
    }));

    assert!(report(old, new).is_compatible());
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
fn response_headers_reject_multiple_content_media_types() {
    let error = OpenApiDocument::from_json(&spec(json!({
        "responses": {
            "200": {
                "description": "ok",
                "headers": {
                    "X-Meta": {
                        "content": {
                            "application/json": {
                                "schema": { "type": "object" }
                            },
                            "text/plain": {
                                "schema": { "type": "string" }
                            }
                        }
                    }
                }
            }
        }
    })))
    .expect_err("response-header content cardinality is invalid OpenAPI")
    .to_string();

    assert!(
        error.contains("content") && error.contains("exactly one media type"),
        "{error}"
    );
}

#[test]
fn openapi_documents_reject_response_header_content_media_type_ranges_before_lowering() {
    let error = OpenApiDocument::from_json(&spec(json!({
        "responses": {
            "200": {
                "description": "ok",
                "headers": {
                    "X-Meta": {
                        "content": {
                            "application/*": {
                                "schema": { "type": "string" }
                            }
                        }
                    }
                }
            }
        }
    })))
    .expect_err("response-header content requires a concrete media type")
    .to_string();

    assert!(
        error.contains("#/paths/~1pets/get/responses/200/headers/X-Meta/content/application~1*")
            && error.contains("a valid concrete media type"),
        "{error}"
    );
}

#[test]
fn content_backed_response_headers_reject_schema_serialization_fields() {
    for (field, value) in [
        ("style", json!("simple")),
        ("explode", json!(true)),
        ("allowReserved", json!(true)),
        ("allowEmptyValue", json!(true)),
        ("example", json!("sample")),
        ("examples", json!({ "sample": { "value": "sample" } })),
    ] {
        let error = OpenApiDocument::from_json(&spec(json!({
            "responses": {
                "200": {
                    "description": "ok",
                    "headers": {
                        "X-Meta": {
                            (field): value,
                            "content": {
                                "application/json": {
                                    "schema": { "type": "string" }
                                }
                            }
                        }
                    }
                }
            }
        })))
        .expect_err("content-backed response-header serialization fields are invalid OpenAPI")
        .to_string();

        assert!(error.contains(field), "{field}: {error}");
    }
}

#[test]
fn response_headers_reject_allow_reserved() {
    let error = OpenApiDocument::from_json(&spec(json!({
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
    })))
    .expect_err("response-header allowReserved is invalid OpenAPI")
    .to_string();

    assert!(
        error.contains("allowReserved") && error.contains("not present for response headers"),
        "{error}"
    );
}

#[test]
fn response_headers_reject_non_simple_style() {
    let error = OpenApiDocument::from_json(&spec(json!({
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
    })))
    .expect_err("response-header non-simple style is invalid OpenAPI")
    .to_string();

    assert!(
        error.contains("style") && error.contains("'simple' for response headers"),
        "{error}"
    );
}

#[test]
fn non_query_parameters_reject_query_only_metadata() {
    let error = OpenApiDocument::from_json(&spec(json!({
        "parameters": [{
            "name": "X-Bad",
            "in": "header",
            "allowEmptyValue": true,
            "schema": { "type": "string" }
        }],
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    })))
    .expect_err("query-only parameter metadata is invalid outside query params")
    .to_string();

    assert!(
        error.contains("allowEmptyValue") && error.contains("a query parameter field"),
        "{error}"
    );
}

#[test]
fn parameters_reject_styles_that_are_invalid_for_their_location() {
    let query_error = OpenApiDocument::from_json(&spec(json!({
        "parameters": [{
            "name": "filter",
            "in": "query",
            "style": "simple",
            "schema": { "type": "string" }
        }],
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    })))
    .expect_err("query parameter style must match the OpenAPI location")
    .to_string();
    assert!(
        query_error.contains("#/paths/~1pets/get/parameters/0/style")
            && query_error.contains("query parameters"),
        "{query_error}"
    );

    let cookie_error = OpenApiDocument::from_json(&spec(json!({
        "parameters": [{
            "name": "session",
            "in": "cookie",
            "style": "pipeDelimited",
            "schema": { "type": "string" }
        }],
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    })))
    .expect_err("cookie parameter style must match the OpenAPI location")
    .to_string();
    assert!(
        cookie_error.contains("#/paths/~1pets/get/parameters/0/style")
            && cookie_error.contains("cookie parameters"),
        "{cookie_error}"
    );
}

#[test]
fn openapi_documents_reject_deep_object_parameters_with_explode_false_before_lowering() {
    let error = OpenApiDocument::from_json(&spec(json!({
        "parameters": [{
            "name": "filter",
            "in": "query",
            "style": "deepObject",
            "explode": false,
            "schema": { "type": "object" }
        }],
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    })))
    .expect_err(
        "deepObject query parameters with explode=false must fail during document validation",
    )
    .to_string();

    assert!(
        error.contains("#/paths/~1pets/get/parameters/0/explode") && error.contains("deepObject"),
        "{error}"
    );
}

#[test]
fn expanding_default_with_an_equivalent_explicit_response_is_compatible() {
    let old = spec(json!({
        "responses": {
            "default": response_schema(json!({ "type": "object" }))
        }
    }));
    let new = spec(json!({
        "responses": {
            "default": response_schema(json!({ "type": "object" })),
            "404": response_schema(json!({ "type": "object" }))
        }
    }));

    assert!(report(old, new).is_compatible());
}

#[test]
fn splitting_default_into_a_range_with_the_same_schema_is_compatible() {
    let old = spec(json!({
        "responses": {
            "default": response_schema(json!({ "type": "object" }))
        }
    }));
    let new = spec(json!({
        "responses": {
            "default": response_schema(json!({ "type": "object" })),
            "2XX": response_schema(json!({ "type": "object" }))
        }
    }));

    assert!(report(old, new).is_compatible());
}

#[test]
fn splitting_default_into_a_range_and_explicit_status_with_the_same_schema_is_compatible() {
    let old = spec(json!({
        "responses": {
            "default": response_schema(json!({ "type": "object" }))
        }
    }));
    let new = spec(json!({
        "responses": {
            "default": response_schema(json!({ "type": "object" })),
            "2XX": response_schema(json!({ "type": "object" })),
            "200": response_schema(json!({ "type": "object" }))
        }
    }));

    assert!(report(old, new).is_compatible());
}

#[test]
fn fully_shadowed_default_response_is_compatible_with_itself() {
    let spec = spec(json!({
        "responses": {
            "1XX": response_schema(json!({ "type": "object" })),
            "2XX": response_schema(json!({ "type": "object" })),
            "3XX": response_schema(json!({ "type": "object" })),
            "4XX": response_schema(json!({ "type": "object" })),
            "5XX": response_schema(json!({ "type": "object" })),
            "default": response_schema(json!({ "type": "object" }))
        }
    }));

    assert!(report(spec.clone(), spec).is_compatible());
}

#[test]
fn changing_a_fully_shadowed_default_response_is_compatible() {
    let old = spec(json!({
        "responses": {
            "1XX": response_schema(json!({ "type": "object" })),
            "2XX": response_schema(json!({ "type": "object" })),
            "3XX": response_schema(json!({ "type": "object" })),
            "4XX": response_schema(json!({ "type": "object" })),
            "5XX": response_schema(json!({ "type": "object" })),
            "default": response_schema(json!({
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
            "1XX": response_schema(json!({ "type": "object" })),
            "2XX": response_schema(json!({ "type": "object" })),
            "3XX": response_schema(json!({ "type": "object" })),
            "4XX": response_schema(json!({ "type": "object" })),
            "5XX": response_schema(json!({ "type": "object" })),
            "default": response_schema(json!({
                "type": "object",
                "properties": {
                    "status": { "type": ["string", "null"] }
                },
                "required": ["status"]
            }))
        }
    }));

    assert!(report(old, new).is_compatible());
}

#[test]
fn splitting_default_into_a_range_with_a_broader_schema_is_incompatible() {
    let old = spec(json!({
        "responses": {
            "default": response_schema(json!({
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
            "default": response_schema(json!({
                "type": "object",
                "properties": {
                    "status": { "type": "string" }
                },
                "required": ["status"]
            })),
            "2XX": response_schema(json!({
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
}

#[test]
fn overriding_a_default_and_range_covered_status_with_a_broader_explicit_response_is_incompatible()
{
    let old = spec(json!({
        "responses": {
            "default": response_schema(json!({
                "type": "object",
                "properties": {
                    "status": { "type": "string" }
                },
                "required": ["status"]
            })),
            "2XX": response_schema(json!({
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
            "default": response_schema(json!({
                "type": "object",
                "properties": {
                    "status": { "type": "string" }
                },
                "required": ["status"]
            })),
            "2XX": response_schema(json!({
                "type": "object",
                "properties": {
                    "status": { "type": "string" }
                },
                "required": ["status"]
            })),
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
}

#[test]
fn response_specification_extensions_do_not_affect_compatibility() {
    let old = spec(json!({
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    }));
    let new = spec(json!({
        "responses": {
            "200": response_schema(json!({ "type": "object" })),
            "x-owner": {
                "team": "platform"
            }
        }
    }));

    assert!(report(old, new).is_compatible());
}

#[test]
fn responses_objects_with_only_extensions_are_rejected() {
    let error = OpenApiDocument::from_json(&spec(json!({
        "responses": {
            "x-owner": {
                "team": "platform"
            }
        }
    })))
    .expect_err("responses maps with only extensions are invalid OpenAPI")
    .to_string();

    assert!(
        error.contains("responses") && error.contains("at least one response"),
        "{error}"
    );
}

#[test]
fn response_status_selectors_must_be_openapi_status_patterns() {
    for status in ["700", "2xx"] {
        let error = OpenApiDocument::from_json(&spec(json!({
            "responses": {
                (status): response_schema(json!({ "type": "object" }))
            }
        })))
        .expect_err("invalid response status selectors are invalid OpenAPI")
        .to_string();

        assert!(error.contains(status), "{error}");
        assert!(
            error.contains("response status code from `100` through `599`"),
            "{error}"
        );
    }
}

#[test]
fn response_objects_require_descriptions() {
    let error = OpenApiDocument::from_json(&spec(json!({
        "responses": {
            "200": {
                "content": {
                    "application/json": {
                        "schema": { "type": "object" }
                    }
                }
            }
        }
    })))
    .expect_err("inline response descriptions are required during document validation")
    .to_string();

    assert!(
        error.contains("#/paths/~1pets/get/responses/200/description"),
        "{error}"
    );
    assert!(error.contains("a string"), "{error}");
}

#[test]
fn expanding_a_response_range_with_an_equivalent_explicit_response_is_compatible() {
    let old = spec(json!({
        "responses": {
            "2XX": response_schema(json!({ "type": "object" }))
        }
    }));
    let new = spec(json!({
        "responses": {
            "2XX": response_schema(json!({ "type": "object" })),
            "200": response_schema(json!({ "type": "object" }))
        }
    }));

    assert!(report(old, new).is_compatible());
}

#[test]
fn splitting_a_response_range_into_explicit_statuses_with_the_same_schema_is_compatible() {
    let old = spec(json!({
        "responses": {
            "2XX": response_schema(json!({ "type": "object" }))
        }
    }));
    let new = spec(json!({
        "responses": {
            "2XX": response_schema(json!({ "type": "object" })),
            "200": response_schema(json!({ "type": "object" })),
            "201": response_schema(json!({ "type": "object" }))
        }
    }));

    assert!(report(old, new).is_compatible());
}

#[test]
fn overriding_a_response_range_with_a_broader_explicit_response_is_incompatible() {
    let old = spec(json!({
        "responses": {
            "2XX": response_schema(json!({
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
            "2XX": response_schema(json!({
                "type": "object",
                "properties": {
                    "status": { "type": "string" }
                },
                "required": ["status"]
            })),
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
}

#[test]
fn adding_a_response_status_is_incompatible() {
    let old = spec(json!({
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    }));
    let new = spec(json!({
        "responses": {
            "200": response_schema(json!({ "type": "object" })),
            "201": response_schema(json!({ "type": "object" }))
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
fn removing_a_response_status_is_compatible_for_serialized_responses() {
    let old = spec(json!({
        "responses": {
            "200": response_schema(json!({ "type": "object" })),
            "201": response_schema(json!({ "type": "object" }))
        }
    }));
    let new = spec(json!({
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    }));

    assert!(report(old, new).is_compatible());
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
fn removing_a_response_media_type_is_compatible_for_serialized_responses() {
    let old = spec(json!({
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
    let new = spec(json!({
        "responses": {
            "200": response_schema(json!({ "type": "object" }))
        }
    }));

    assert!(report(old, new).is_compatible());
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
fn referenced_path_parameters_can_be_overridden_while_referenced_headers_still_compare() {
    let old = json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "components": {
            "parameters": {
                "Trace": {
                    "name": "trace",
                    "in": "query",
                    "schema": { "type": "string" }
                }
            },
            "headers": {
                "RateLimit": {
                    "schema": { "type": "integer" }
                }
            }
        },
        "paths": {
            "/pets": {
                "parameters": [{ "$ref": "#/components/parameters/Trace" }],
                "get": {
                    "parameters": [{
                        "name": "trace",
                        "in": "query",
                        "required": true,
                        "schema": { "type": "string" }
                    }],
                    "responses": {
                        "200": {
                            "description": "ok",
                            "headers": {
                                "X-Rate-Limit": { "$ref": "#/components/headers/RateLimit" }
                            }
                        }
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
                "Trace": {
                    "name": "trace",
                    "in": "query",
                    "schema": { "type": "string" }
                }
            },
            "headers": {
                "RateLimit": {
                    "schema": { "type": ["integer", "string"] }
                }
            }
        },
        "paths": {
            "/pets": {
                "parameters": [{ "$ref": "#/components/parameters/Trace" }],
                "get": {
                    "parameters": [{
                        "name": "trace",
                        "in": "query",
                        "required": true,
                        "schema": { "type": "string" }
                    }],
                    "responses": {
                        "200": {
                            "description": "ok",
                            "headers": {
                                "X-Rate-Limit": { "$ref": "#/components/headers/RateLimit" }
                            }
                        }
                    }
                }
            }
        }
    });

    let report = report(old, new);

    assert!(!report.is_compatible());
    assert_eq!(
        issue_surfaces(&report),
        vec![OpenApiCompatibilitySurface::Response]
    );
    assert_eq!(
        report.issues()[0].message,
        "new schema #/properties/headers/properties/x-rate-limit/properties/value/anyOf/1: property 'headers' -> property 'x-rate-limit' -> property 'value' -> anyOf branch 2: new values may be strings, but the previous schema only accepted integers"
    );
}

#[test]
fn recursive_component_schema_refs_are_lowered_before_compatibility_checks() {
    let old = json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Nodes",
            "version": "1.0.0"
        },
        "components": {
            "schemas": {
                "Node": {
                    "type": "object",
                    "properties": {
                        "next": { "$ref": "#/components/schemas/Node" }
                    },
                    "additionalProperties": false
                }
            }
        },
        "paths": {
            "/nodes": {
                "get": {
                    "responses": {
                        "200": response_schema(json!({
                            "$ref": "#/components/schemas/Node"
                        }))
                    }
                }
            }
        }
    });
    let new = json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Nodes",
            "version": "1.0.0"
        },
        "components": {
            "schemas": {
                "Node": {
                    "type": "object",
                    "properties": {
                        "label": { "type": "string" },
                        "next": { "$ref": "#/components/schemas/Node" }
                    },
                    "additionalProperties": false
                }
            }
        },
        "paths": {
            "/nodes": {
                "get": {
                    "responses": {
                        "200": response_schema(json!({
                            "$ref": "#/components/schemas/Node"
                        }))
                    }
                }
            }
        }
    });

    let report = report(old, new);

    assert!(!report.is_compatible());
    assert_eq!(
        issue_surfaces(&report),
        vec![OpenApiCompatibilitySurface::Response]
    );
    assert_eq!(
        report.issues()[0].message,
        "new schema #/properties/body/properties/value/properties/label: property 'body' -> property 'value': property 'label' can appear with values the comparison target rejects"
    );
}

#[test]
fn openapi_reference_objects_accept_metadata_and_ignore_other_siblings() {
    let document = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "get": {
                    "parameters": [{
                        "$ref": "#/components/parameters/Limit",
                        "summary": "Limit parameter",
                        "description": "Ignored reference-object metadata should stay valid",
                        "required": true
                    }],
                    "responses": {
                        "200": response_schema(json!({ "type": "object" }))
                    }
                }
            }
        },
        "components": {
            "parameters": {
                "Limit": {
                    "name": "limit",
                    "in": "query",
                    "schema": { "type": "integer" }
                }
            }
        }
    }))
    .expect("OpenAPI 3.1 Reference Objects may carry metadata and ignored extra fields");

    validate_openapi_compatibility_input(&document)
        .expect("ignored Reference Object extras must not block lowering");
}

#[test]
fn openapi_reference_objects_require_string_summary_and_description() {
    for (field, value) in [("summary", json!(42)), ("description", json!(true))] {
        let error = OpenApiDocument::from_json(&json!({
            "openapi": "3.1.0",
            "info": { "title": "Pets", "version": "1.0.0" },
            "paths": {
                "/pets": {
                    "get": {
                        "parameters": [{
                            "$ref": "#/components/parameters/Limit",
                            (field): value
                        }],
                        "responses": {
                            "200": response_schema(json!({ "type": "object" }))
                        }
                    }
                }
            },
            "components": {
                "parameters": {
                    "Limit": {
                        "name": "limit",
                        "in": "query",
                        "schema": { "type": "integer" }
                    }
                }
            }
        }))
        .expect_err("Reference Object summary/description must stay strings")
        .to_string();

        assert!(
            error.contains(&format!("#/paths/~1pets/get/parameters/0/{field}"))
                && error.contains("a string"),
            "{field}: {error}"
        );
    }
}

#[test]
fn openapi_reference_objects_reject_invalid_reference_uris_before_lowering() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "get": {
                    "parameters": [{
                        "$ref": "http://["
                    }],
                    "responses": {
                        "200": response_schema(json!({ "type": "object" }))
                    }
                }
            }
        }
    }))
    .expect_err("OpenAPI Reference Object `$ref` values must be valid URI references")
    .to_string();

    assert!(
        error.contains("#/paths/~1pets/get/parameters/0/$ref"),
        "{error}"
    );
    assert!(error.contains("a valid URI reference"), "{error}");
}

#[test]
fn openapi_reference_objects_reject_remote_references() {
    let error = compat_error(
        json!({
            "openapi": "3.1.0",
            "info": { "title": "Pets", "version": "1.0.0" },
            "paths": {
                "/pets": {
                    "get": {
                        "parameters": [{
                            "$ref": "https://example.com/parameters.json#/Limit"
                        }],
                        "responses": {
                            "200": response_schema(json!({ "type": "object" }))
                        }
                    }
                }
            }
        }),
        spec(get_operation()),
    );

    assert!(
        error.contains("unsupported OpenAPI reference")
            && error.contains("https://example.com/parameters.json#/Limit"),
        "{error}"
    );
}

#[test]
fn openapi_reference_objects_reject_unresolved_local_references() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "get": {
                    "parameters": [{
                        "$ref": "#/components/parameters/Missing"
                    }],
                    "responses": {
                        "200": response_schema(json!({ "type": "object" }))
                    }
                }
            }
        }
    }))
    .expect_err("locally referenced path parameters must resolve during validation")
    .to_string();

    assert!(
        error.contains("did not resolve") && error.contains("#/components/parameters/Missing"),
        "{error}"
    );
}

#[test]
fn openapi_reference_objects_reject_cycles() {
    let error = OpenApiDocument::from_json(&json!({
        "openapi": "3.1.0",
        "info": { "title": "Pets", "version": "1.0.0" },
        "components": {
            "parameters": {
                "Limit": { "$ref": "#/components/parameters/LimitAlias" },
                "LimitAlias": { "$ref": "#/components/parameters/Limit" }
            },
        },
        "paths": {
            "/pets": {
                "get": {
                    "parameters": [{
                        "$ref": "#/components/parameters/Limit"
                    }],
                    "responses": {
                        "200": response_schema(json!({ "type": "object" }))
                    }
                }
            }
        }
    }))
    .expect_err("cyclic locally referenced path parameters must fail during validation")
    .to_string();

    assert!(
        error.contains("forms a cycle") && error.contains("#/components/parameters/Limit"),
        "{error}"
    );
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
                            "payload": {
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
                            "payload": {
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
    assert!(message.contains("property 'payload'"), "{message}");
    assert!(message.contains("objects"), "{message}");
    assert!(message.contains("strings"), "{message}");
}
