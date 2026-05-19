use jsoncompat::{OpenApiDocument, check_openapi_compat};
use jsoncompat_openapi::{OpenApiOperationLowerer, OperationKey};
use serde_json::{Value, json};

#[test]
fn claimed_openapi_compatibility_survives_lowered_contract_witnesses() {
    let documents = [
        (
            "baseline",
            spec(
                json!([]),
                None,
                response_schema(json!({
                    "type": "object",
                    "properties": {
                        "id": { "type": "integer" }
                    },
                    "required": ["id"],
                    "additionalProperties": false
                })),
            ),
        ),
        (
            "optional_query_limit",
            spec(
                json!([{
                    "name": "limit",
                    "in": "query",
                    "schema": { "type": "integer" }
                }]),
                None,
                response_schema(json!({
                    "type": "object",
                    "properties": {
                        "id": { "type": "integer" }
                    },
                    "required": ["id"],
                    "additionalProperties": false
                })),
            ),
        ),
        (
            "required_query_limit",
            spec(
                json!([{
                    "name": "limit",
                    "in": "query",
                    "required": true,
                    "schema": { "type": "integer" }
                }]),
                None,
                response_schema(json!({
                    "type": "object",
                    "properties": {
                        "id": { "type": "integer" }
                    },
                    "required": ["id"],
                    "additionalProperties": false
                })),
            ),
        ),
        (
            "optional_request_body",
            spec(
                json!([]),
                Some(json!({
                    "content": {
                        "application/json": {
                            "schema": {
                                "type": "object",
                                "properties": {
                                    "id": { "type": "integer" }
                                },
                                "required": ["id"],
                                "additionalProperties": false
                            }
                        }
                    }
                })),
                response_schema(json!({
                    "type": "object",
                    "properties": {
                        "id": { "type": "integer" }
                    },
                    "required": ["id"],
                    "additionalProperties": false
                })),
            ),
        ),
        (
            "required_request_body",
            spec(
                json!([]),
                Some(json!({
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": {
                                "type": "object",
                                "properties": {
                                    "id": { "type": "integer" }
                                },
                                "required": ["id"],
                                "additionalProperties": false
                            }
                        }
                    }
                })),
                response_schema(json!({
                    "type": "object",
                    "properties": {
                        "id": { "type": "integer" }
                    },
                    "required": ["id"],
                    "additionalProperties": false
                })),
            ),
        ),
        (
            "response_id_string_or_integer",
            spec(
                json!([]),
                None,
                response_schema(json!({
                    "type": "object",
                    "properties": {
                        "id": { "type": ["integer", "string"] }
                    },
                    "required": ["id"],
                    "additionalProperties": false
                })),
            ),
        ),
    ]
    .into_iter()
    .map(|(name, raw)| {
        (
            name,
            OpenApiDocument::from_json(&raw).expect("soundness OpenAPI document should build"),
        )
    })
    .collect::<Vec<_>>();

    let request_witnesses = [
        request(json!({}), Value::Null),
        request(json!({}), json_body(json!({ "id": 1 }))),
        request(query_limit(json!(1)), Value::Null),
        request(query_limit(json!(1)), json_body(json!({ "id": 1 }))),
        request(query_limit(json!("x")), Value::Null),
    ];
    let response_witnesses = [
        response(json!({ "id": 1 })),
        response(json!({ "id": "one" })),
        response(json!({ "id": 1, "extra": true })),
    ];
    let operation = OperationKey {
        method: "GET".to_owned(),
        path: "/pets".to_owned(),
    };

    for (old_name, old) in &documents {
        for (new_name, new) in &documents {
            let report =
                check_openapi_compat(old, new).expect("OpenAPI soundness corpus should compare");
            if !report.is_compatible() {
                continue;
            }

            let old_lowerer = OpenApiOperationLowerer::new(old).expect("old lowerer should build");
            let new_lowerer = OpenApiOperationLowerer::new(new).expect("new lowerer should build");
            let old_operation = old_lowerer
                .lower_operation(&operation)
                .expect("old operation should lower")
                .expect("old operation should exist");
            let new_operation = new_lowerer
                .lower_operation(&operation)
                .expect("new operation should lower")
                .expect("new operation should exist");

            let old_request = old
                .lowered_contract_document(&old_operation.request)
                .expect("old lowered request should build");
            let new_request = new
                .lowered_contract_document(&new_operation.request)
                .expect("new lowered request should build");
            assert_witness_inclusion(
                old_name,
                new_name,
                "request",
                &old_request,
                &new_request,
                &request_witnesses,
            );

            let old_response = old
                .lowered_contract_document(&old_operation.response)
                .expect("old lowered response should build");
            let new_response = new
                .lowered_contract_document(&new_operation.response)
                .expect("new lowered response should build");
            assert_witness_inclusion(
                new_name,
                old_name,
                "response",
                &new_response,
                &old_response,
                &response_witnesses,
            );
        }
    }
}

#[test]
fn claimed_openapi_media_header_and_status_compatibility_survives_lowered_contract_witnesses() {
    let documents = [
        (
            "json_200",
            body_surface_spec(
                request_body_for("application/json"),
                json!({
                    "200": response_with(
                        "application/json",
                        json!({ "type": "string" }),
                        json!({})
                    )
                }),
            ),
        ),
        (
            "application_range_request",
            body_surface_spec(
                request_body_for("application/*"),
                json!({
                    "200": response_with(
                        "application/json",
                        json!({ "type": "string" }),
                        json!({})
                    )
                }),
            ),
        ),
        (
            "global_range_request",
            body_surface_spec(
                request_body_for("*/*"),
                json!({
                    "200": response_with(
                        "application/json",
                        json!({ "type": "string" }),
                        json!({})
                    )
                }),
            ),
        ),
        (
            "parameterized_json_request",
            body_surface_spec(
                request_body_for("application/json; charset=utf-8"),
                json!({
                    "200": response_with(
                        "application/json",
                        json!({ "type": "string" }),
                        json!({})
                    )
                }),
            ),
        ),
        (
            "global_range_request_with_application_and_exact_json_overrides",
            body_surface_spec(
                json!({
                    "required": true,
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
                }),
                json!({
                    "200": response_with(
                        "application/json",
                        json!({ "type": "string" }),
                        json!({})
                    )
                }),
            ),
        ),
        (
            "application_range_request_with_exact_json_integer",
            body_surface_spec(
                json!({
                    "required": true,
                    "content": {
                        "application/*": {
                            "schema": { "type": "string" }
                        },
                        "application/json": {
                            "schema": { "type": "integer" }
                        }
                    }
                }),
                json!({
                    "200": response_with(
                        "application/json",
                        json!({ "type": "string" }),
                        json!({})
                    )
                }),
            ),
        ),
        (
            "parameterized_application_range_request_with_exact_json_integer",
            body_surface_spec(
                json!({
                    "required": true,
                    "content": {
                        "application/*; q=0.5": {
                            "schema": { "type": "string" }
                        },
                        "application/json; charset=utf-8": {
                            "schema": { "type": "integer" }
                        }
                    }
                }),
                json!({
                    "200": response_with(
                        "application/json",
                        json!({ "type": "string" }),
                        json!({})
                    )
                }),
            ),
        ),
        (
            "application_range_response",
            body_surface_spec(
                request_body_for("application/json"),
                json!({
                    "200": response_with(
                        "application/*",
                        json!({ "type": "string" }),
                        json!({})
                    )
                }),
            ),
        ),
        (
            "global_range_response",
            body_surface_spec(
                request_body_for("application/json"),
                json!({
                    "200": response_with(
                        "*/*",
                        json!({ "type": "string" }),
                        json!({})
                    )
                }),
            ),
        ),
        (
            "parameterized_json_response",
            body_surface_spec(
                request_body_for("application/json"),
                json!({
                    "200": response_with(
                        "application/json; charset=utf-8",
                        json!({ "type": "string" }),
                        json!({})
                    )
                }),
            ),
        ),
        (
            "global_range_response_with_application_and_exact_json_overrides",
            body_surface_spec(
                request_body_for("application/json"),
                json!({
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
                }),
            ),
        ),
        (
            "application_range_response_with_exact_json_integer",
            body_surface_spec(
                request_body_for("application/json"),
                json!({
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
                }),
            ),
        ),
        (
            "parameterized_application_range_response_with_exact_json_integer",
            body_surface_spec(
                request_body_for("application/json"),
                json!({
                    "200": {
                        "description": "ok",
                        "content": {
                            "application/*; q=0.5": {
                                "schema": { "type": "string" }
                            },
                            "application/json; charset=utf-8": {
                                "schema": { "type": "integer" }
                            }
                        }
                    }
                }),
            ),
        ),
        (
            "optional_trace_header",
            body_surface_spec(
                request_body_for("application/json"),
                json!({
                    "200": response_with(
                        "application/json",
                        json!({ "type": "string" }),
                        json!({
                            "x-trace-id": {
                                "schema": { "type": "string" }
                            }
                        })
                    )
                }),
            ),
        ),
        (
            "required_trace_header",
            body_surface_spec(
                request_body_for("application/json"),
                json!({
                    "200": response_with(
                        "application/json",
                        json!({ "type": "string" }),
                        json!({
                            "x-trace-id": {
                                "required": true,
                                "schema": { "type": "string" }
                            }
                        })
                    )
                }),
            ),
        ),
        (
            "range_2xx",
            body_surface_spec(
                request_body_for("application/json"),
                json!({
                    "2XX": response_with(
                        "application/json",
                        json!({ "type": "string" }),
                        json!({})
                    )
                }),
            ),
        ),
        (
            "default_response",
            body_surface_spec(
                request_body_for("application/json"),
                json!({
                    "default": response_with(
                        "application/json",
                        json!({ "type": "string" }),
                        json!({})
                    )
                }),
            ),
        ),
        (
            "default_plus_2xx",
            body_surface_spec(
                request_body_for("application/json"),
                json!({
                    "default": response_with(
                        "application/json",
                        json!({ "type": "string" }),
                        json!({})
                    ),
                    "2XX": response_with(
                        "application/json",
                        json!({ "type": "integer" }),
                        json!({})
                    )
                }),
            ),
        ),
        (
            "default_plus_2xx_plus_200",
            body_surface_spec(
                request_body_for("application/json"),
                json!({
                    "default": response_with(
                        "application/json",
                        json!({ "type": "string" }),
                        json!({})
                    ),
                    "2XX": response_with(
                        "application/json",
                        json!({ "type": "integer" }),
                        json!({})
                    ),
                    "200": response_with(
                        "application/json",
                        json!({ "type": "boolean" }),
                        json!({})
                    )
                }),
            ),
        ),
        (
            "range_media_override_response",
            body_surface_spec(
                request_body_for("application/json"),
                json!({
                    "2XX": {
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
                }),
            ),
        ),
    ]
    .into_iter()
    .map(|(name, raw)| {
        (
            name,
            OpenApiDocument::from_json(&raw).expect("soundness OpenAPI document should build"),
        )
    })
    .collect::<Vec<_>>();

    let request_witnesses = [
        request(json!({}), json_body(json!("pet"))),
        request(json!({}), json_body(json!(1))),
        request(json!({}), typed_body("application", "xml", json!("pet"))),
        request(json!({}), typed_body("text", "plain", json!("pet"))),
    ];
    let response_witnesses = [
        typed_response("200", json_body(json!("ok")), json!({})),
        typed_response(
            "200",
            json_body(json!("ok")),
            json!({
                "x-trace-id": {
                    "value": "trace",
                    "explode": false
                }
            }),
        ),
        typed_response(
            "200",
            typed_body("application", "xml", json!("ok")),
            json!({}),
        ),
        typed_response("200", typed_body("text", "plain", json!("ok")), json!({})),
        typed_response("200", json_body(json!(1)), json!({})),
        typed_response("200", json_body(json!(true)), json!({})),
        typed_response("201", json_body(json!("ok")), json!({})),
        typed_response("201", json_body(json!(1)), json!({})),
        typed_response("404", json_body(json!("ok")), json!({})),
    ];
    let operation = OperationKey {
        method: "GET".to_owned(),
        path: "/pets".to_owned(),
    };

    for (old_name, old) in &documents {
        for (new_name, new) in &documents {
            let report =
                check_openapi_compat(old, new).expect("OpenAPI soundness corpus should compare");
            if !report.is_compatible() {
                continue;
            }

            let old_lowerer = OpenApiOperationLowerer::new(old).expect("old lowerer should build");
            let new_lowerer = OpenApiOperationLowerer::new(new).expect("new lowerer should build");
            let old_operation = old_lowerer
                .lower_operation(&operation)
                .expect("old operation should lower")
                .expect("old operation should exist");
            let new_operation = new_lowerer
                .lower_operation(&operation)
                .expect("new operation should lower")
                .expect("new operation should exist");

            let old_request = old
                .lowered_contract_document(&old_operation.request)
                .expect("old lowered request should build");
            let new_request = new
                .lowered_contract_document(&new_operation.request)
                .expect("new lowered request should build");
            assert_witness_inclusion(
                old_name,
                new_name,
                "request",
                &old_request,
                &new_request,
                &request_witnesses,
            );

            let old_response = old
                .lowered_contract_document(&old_operation.response)
                .expect("old lowered response should build");
            let new_response = new
                .lowered_contract_document(&new_operation.response)
                .expect("new lowered response should build");
            assert_witness_inclusion(
                new_name,
                old_name,
                "response",
                &new_response,
                &old_response,
                &response_witnesses,
            );
        }
    }
}

#[test]
fn claimed_openapi_query_serialization_compatibility_survives_lowered_contract_witnesses() {
    let documents = [
        (
            "query_form_defaults",
            spec(
                json!([{
                    "name": "filter",
                    "in": "query",
                    "schema": { "type": "string" }
                }]),
                None,
                response_schema(json!({ "type": "string" })),
            ),
        ),
        (
            "query_form_no_explode",
            spec(
                json!([{
                    "name": "filter",
                    "in": "query",
                    "style": "form",
                    "explode": false,
                    "schema": { "type": "string" }
                }]),
                None,
                response_schema(json!({ "type": "string" })),
            ),
        ),
        (
            "query_pipe_delimited",
            spec(
                json!([{
                    "name": "filter",
                    "in": "query",
                    "style": "pipeDelimited",
                    "explode": false,
                    "schema": { "type": "string" }
                }]),
                None,
                response_schema(json!({ "type": "string" })),
            ),
        ),
        (
            "query_space_delimited",
            spec(
                json!([{
                    "name": "filter",
                    "in": "query",
                    "style": "spaceDelimited",
                    "explode": false,
                    "schema": { "type": "string" }
                }]),
                None,
                response_schema(json!({ "type": "string" })),
            ),
        ),
        (
            "query_deep_object",
            spec(
                json!([{
                    "name": "filter",
                    "in": "query",
                    "style": "deepObject",
                    "explode": true,
                    "schema": { "type": "string" }
                }]),
                None,
                response_schema(json!({ "type": "string" })),
            ),
        ),
        (
            "query_allow_reserved",
            spec(
                json!([{
                    "name": "filter",
                    "in": "query",
                    "allowReserved": true,
                    "schema": { "type": "string" }
                }]),
                None,
                response_schema(json!({ "type": "string" })),
            ),
        ),
        (
            "query_allow_empty_value",
            spec(
                json!([{
                    "name": "filter",
                    "in": "query",
                    "allowEmptyValue": true,
                    "schema": { "type": "string" }
                }]),
                None,
                response_schema(json!({ "type": "string" })),
            ),
        ),
    ]
    .into_iter()
    .map(|(name, raw)| {
        (
            name,
            OpenApiDocument::from_json(&raw).expect("query soundness OpenAPI should build"),
        )
    })
    .collect::<Vec<_>>();

    let request_witnesses = [
        request(json!({}), Value::Null),
        request(
            query_filter_with_serialization("pet", "form", true, false, false),
            Value::Null,
        ),
        request(
            query_filter_with_serialization("pet", "form", false, false, false),
            Value::Null,
        ),
        request(
            query_filter_with_serialization("pet", "pipeDelimited", false, false, false),
            Value::Null,
        ),
        request(
            query_filter_with_serialization("pet", "spaceDelimited", false, false, false),
            Value::Null,
        ),
        request(
            query_filter_with_serialization("pet", "deepObject", true, false, false),
            Value::Null,
        ),
        request(
            query_filter_with_serialization("pet", "form", true, true, false),
            Value::Null,
        ),
        request(
            query_filter_with_serialization("pet", "form", true, false, true),
            Value::Null,
        ),
    ];
    let response_witnesses = [response(json!("ok"))];
    let operation = OperationKey {
        method: "GET".to_owned(),
        path: "/pets".to_owned(),
    };

    for (old_name, old) in &documents {
        for (new_name, new) in &documents {
            let report =
                check_openapi_compat(old, new).expect("OpenAPI soundness corpus should compare");
            if !report.is_compatible() {
                continue;
            }

            let old_lowerer = OpenApiOperationLowerer::new(old).expect("old lowerer should build");
            let new_lowerer = OpenApiOperationLowerer::new(new).expect("new lowerer should build");
            let old_operation = old_lowerer
                .lower_operation(&operation)
                .expect("old operation should lower")
                .expect("old operation should exist");
            let new_operation = new_lowerer
                .lower_operation(&operation)
                .expect("new operation should lower")
                .expect("new operation should exist");

            let old_request = old
                .lowered_contract_document(&old_operation.request)
                .expect("old lowered request should build");
            let new_request = new
                .lowered_contract_document(&new_operation.request)
                .expect("new lowered request should build");
            assert_witness_inclusion(
                old_name,
                new_name,
                "request",
                &old_request,
                &new_request,
                &request_witnesses,
            );

            let old_response = old
                .lowered_contract_document(&old_operation.response)
                .expect("old lowered response should build");
            let new_response = new
                .lowered_contract_document(&new_operation.response)
                .expect("new lowered response should build");
            assert_witness_inclusion(
                new_name,
                old_name,
                "response",
                &new_response,
                &old_response,
                &response_witnesses,
            );
        }
    }
}

#[test]
fn claimed_openapi_component_body_and_response_refs_survive_lowered_contract_witnesses() {
    let documents = [
        (
            "required_integer_components",
            component_body_response_spec(
                json!({
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": {
                                "type": "object",
                                "properties": {
                                    "id": { "type": "integer" }
                                },
                                "required": ["id"],
                                "additionalProperties": false
                            }
                        }
                    }
                }),
                response_schema(json!({
                    "type": "object",
                    "properties": {
                        "id": { "type": "integer" }
                    },
                    "required": ["id"],
                    "additionalProperties": false
                })),
            ),
        ),
        (
            "optional_integer_components",
            component_body_response_spec(
                json!({
                    "content": {
                        "application/json": {
                            "schema": {
                                "type": "object",
                                "properties": {
                                    "id": { "type": "integer" }
                                },
                                "required": ["id"],
                                "additionalProperties": false
                            }
                        }
                    }
                }),
                response_schema(json!({
                    "type": "object",
                    "properties": {
                        "id": { "type": "integer" }
                    },
                    "required": ["id"],
                    "additionalProperties": false
                })),
            ),
        ),
        (
            "request_id_string_or_integer_components",
            component_body_response_spec(
                json!({
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": {
                                "type": "object",
                                "properties": {
                                    "id": { "type": ["integer", "string"] }
                                },
                                "required": ["id"],
                                "additionalProperties": false
                            }
                        }
                    }
                }),
                response_schema(json!({
                    "type": "object",
                    "properties": {
                        "id": { "type": "integer" }
                    },
                    "required": ["id"],
                    "additionalProperties": false
                })),
            ),
        ),
        (
            "response_id_string_or_integer_components",
            component_body_response_spec(
                json!({
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": {
                                "type": "object",
                                "properties": {
                                    "id": { "type": "integer" }
                                },
                                "required": ["id"],
                                "additionalProperties": false
                            }
                        }
                    }
                }),
                response_schema(json!({
                    "type": "object",
                    "properties": {
                        "id": { "type": ["integer", "string"] }
                    },
                    "required": ["id"],
                    "additionalProperties": false
                })),
            ),
        ),
    ]
    .into_iter()
    .map(|(name, raw)| {
        (
            name,
            OpenApiDocument::from_json(&raw)
                .expect("component-ref soundness OpenAPI document should build"),
        )
    })
    .collect::<Vec<_>>();

    let request_witnesses = [
        request(json!({}), Value::Null),
        request(json!({}), json_body(json!({ "id": 1 }))),
        request(json!({}), json_body(json!({ "id": "one" }))),
        request(json!({}), json_body(json!({ "id": 1, "extra": true }))),
    ];
    let response_witnesses = [
        typed_response("200", json_body(json!({ "id": 1 })), json!({})),
        typed_response("200", json_body(json!({ "id": "one" })), json!({})),
        typed_response(
            "200",
            json_body(json!({ "id": 1, "extra": true })),
            json!({}),
        ),
    ];
    let operation = OperationKey {
        method: "POST".to_owned(),
        path: "/pets".to_owned(),
    };

    for (old_name, old) in &documents {
        for (new_name, new) in &documents {
            let report =
                check_openapi_compat(old, new).expect("OpenAPI soundness corpus should compare");
            if !report.is_compatible() {
                continue;
            }

            let old_lowerer = OpenApiOperationLowerer::new(old).expect("old lowerer should build");
            let new_lowerer = OpenApiOperationLowerer::new(new).expect("new lowerer should build");
            let old_operation = old_lowerer
                .lower_operation(&operation)
                .expect("old operation should lower")
                .expect("old operation should exist");
            let new_operation = new_lowerer
                .lower_operation(&operation)
                .expect("new operation should lower")
                .expect("new operation should exist");

            let old_request = old
                .lowered_contract_document(&old_operation.request)
                .expect("old lowered request should build");
            let new_request = new
                .lowered_contract_document(&new_operation.request)
                .expect("new lowered request should build");
            assert_witness_inclusion(
                old_name,
                new_name,
                "request",
                &old_request,
                &new_request,
                &request_witnesses,
            );

            let old_response = old
                .lowered_contract_document(&old_operation.response)
                .expect("old lowered response should build");
            let new_response = new
                .lowered_contract_document(&new_operation.response)
                .expect("new lowered response should build");
            assert_witness_inclusion(
                new_name,
                old_name,
                "response",
                &new_response,
                &old_response,
                &response_witnesses,
            );
        }
    }
}

#[test]
fn claimed_openapi_recursive_component_refs_survive_lowered_contract_witnesses() {
    let documents = [
        (
            "recursive_node_without_label",
            recursive_component_body_response_spec(json!({
                "type": "object",
                "properties": {
                    "next": { "$ref": "#/components/schemas/Node" }
                },
                "additionalProperties": false
            })),
        ),
        (
            "recursive_node_with_optional_label",
            recursive_component_body_response_spec(json!({
                "type": "object",
                "properties": {
                    "label": { "type": "string" },
                    "next": { "$ref": "#/components/schemas/Node" }
                },
                "additionalProperties": false
            })),
        ),
        (
            "recursive_node_with_required_label",
            recursive_component_body_response_spec(json!({
                "type": "object",
                "properties": {
                    "label": { "type": "string" },
                    "next": { "$ref": "#/components/schemas/Node" }
                },
                "required": ["label"],
                "additionalProperties": false
            })),
        ),
    ]
    .into_iter()
    .map(|(name, raw)| {
        (
            name,
            OpenApiDocument::from_json(&raw)
                .expect("recursive component-ref soundness OpenAPI document should build"),
        )
    })
    .collect::<Vec<_>>();

    let request_witnesses = [
        request(json!({}), json_body(json!({}))),
        request(json!({}), json_body(json!({ "label": "root" }))),
        request(json!({}), json_body(json!({ "next": {} }))),
        request(
            json!({}),
            json_body(json!({
                "label": "root",
                "next": {
                    "label": "child"
                }
            })),
        ),
        request(
            json!({}),
            json_body(json!({
                "next": {
                    "label": "child"
                }
            })),
        ),
    ];
    let response_witnesses = [
        typed_response("200", json_body(json!({})), json!({})),
        typed_response("200", json_body(json!({ "label": "root" })), json!({})),
        typed_response("200", json_body(json!({ "next": {} })), json!({})),
        typed_response(
            "200",
            json_body(json!({
                "label": "root",
                "next": {
                    "label": "child"
                }
            })),
            json!({}),
        ),
        typed_response(
            "200",
            json_body(json!({
                "next": {
                    "label": "child"
                }
            })),
            json!({}),
        ),
    ];
    let operation = OperationKey {
        method: "POST".to_owned(),
        path: "/nodes".to_owned(),
    };

    for (old_name, old) in &documents {
        for (new_name, new) in &documents {
            let report =
                check_openapi_compat(old, new).expect("OpenAPI soundness corpus should compare");
            if !report.is_compatible() {
                continue;
            }

            let old_lowerer = OpenApiOperationLowerer::new(old).expect("old lowerer should build");
            let new_lowerer = OpenApiOperationLowerer::new(new).expect("new lowerer should build");
            let old_operation = old_lowerer
                .lower_operation(&operation)
                .expect("old operation should lower")
                .expect("old operation should exist");
            let new_operation = new_lowerer
                .lower_operation(&operation)
                .expect("new operation should lower")
                .expect("new operation should exist");

            let old_request = old
                .lowered_contract_document(&old_operation.request)
                .expect("old lowered request should build");
            let new_request = new
                .lowered_contract_document(&new_operation.request)
                .expect("new lowered request should build");
            assert_witness_inclusion(
                old_name,
                new_name,
                "request",
                &old_request,
                &new_request,
                &request_witnesses,
            );

            let old_response = old
                .lowered_contract_document(&old_operation.response)
                .expect("old lowered response should build");
            let new_response = new
                .lowered_contract_document(&new_operation.response)
                .expect("new lowered response should build");
            assert_witness_inclusion(
                new_name,
                old_name,
                "response",
                &new_response,
                &old_response,
                &response_witnesses,
            );
        }
    }
}

#[test]
fn claimed_openapi_path_header_and_cookie_serialization_survives_lowered_contract_witnesses() {
    let documents = [
        (
            "parameter_serialization_defaults",
            parameter_serialization_spec(
                json!({
                    "name": "petId",
                    "in": "path",
                    "required": true,
                    "schema": { "type": "string" }
                }),
                json!({
                    "name": "trace",
                    "in": "header",
                    "schema": { "type": "string" }
                }),
                json!({
                    "name": "session",
                    "in": "cookie",
                    "schema": { "type": "string" }
                }),
            ),
        ),
        (
            "matrix_path_parameter",
            parameter_serialization_spec(
                json!({
                    "name": "petId",
                    "in": "path",
                    "required": true,
                    "style": "matrix",
                    "explode": true,
                    "schema": { "type": "string" }
                }),
                json!({
                    "name": "trace",
                    "in": "header",
                    "schema": { "type": "string" }
                }),
                json!({
                    "name": "session",
                    "in": "cookie",
                    "schema": { "type": "string" }
                }),
            ),
        ),
        (
            "label_path_parameter",
            parameter_serialization_spec(
                json!({
                    "name": "petId",
                    "in": "path",
                    "required": true,
                    "style": "label",
                    "explode": false,
                    "schema": { "type": "string" }
                }),
                json!({
                    "name": "trace",
                    "in": "header",
                    "schema": { "type": "string" }
                }),
                json!({
                    "name": "session",
                    "in": "cookie",
                    "schema": { "type": "string" }
                }),
            ),
        ),
        (
            "exploded_header_parameter",
            parameter_serialization_spec(
                json!({
                    "name": "petId",
                    "in": "path",
                    "required": true,
                    "schema": { "type": "string" }
                }),
                json!({
                    "name": "trace",
                    "in": "header",
                    "explode": true,
                    "schema": { "type": "string" }
                }),
                json!({
                    "name": "session",
                    "in": "cookie",
                    "schema": { "type": "string" }
                }),
            ),
        ),
        (
            "compact_cookie_parameter",
            parameter_serialization_spec(
                json!({
                    "name": "petId",
                    "in": "path",
                    "required": true,
                    "schema": { "type": "string" }
                }),
                json!({
                    "name": "trace",
                    "in": "header",
                    "schema": { "type": "string" }
                }),
                json!({
                    "name": "session",
                    "in": "cookie",
                    "style": "form",
                    "explode": false,
                    "schema": { "type": "string" }
                }),
            ),
        ),
    ]
    .into_iter()
    .map(|(name, raw)| {
        (
            name,
            OpenApiDocument::from_json(&raw)
                .expect("non-query parameter soundness OpenAPI document should build"),
        )
    })
    .collect::<Vec<_>>();

    let request_witnesses = [
        request_with_parts(
            path_pet_id("pet-1", "simple", false),
            json!({}),
            json!({}),
            json!({}),
            Value::Null,
        ),
        request_with_parts(
            path_pet_id("pet-1", "matrix", true),
            json!({}),
            json!({}),
            json!({}),
            Value::Null,
        ),
        request_with_parts(
            path_pet_id("pet-1", "label", false),
            json!({}),
            json!({}),
            json!({}),
            Value::Null,
        ),
        request_with_parts(
            path_pet_id("pet-1", "simple", false),
            json!({}),
            header_trace("trace-1", false),
            json!({}),
            Value::Null,
        ),
        request_with_parts(
            path_pet_id("pet-1", "simple", false),
            json!({}),
            header_trace("trace-1", true),
            json!({}),
            Value::Null,
        ),
        request_with_parts(
            path_pet_id("pet-1", "simple", false),
            json!({}),
            json!({}),
            cookie_session("session-1", "form", true),
            Value::Null,
        ),
        request_with_parts(
            path_pet_id("pet-1", "simple", false),
            json!({}),
            json!({}),
            cookie_session("session-1", "form", false),
            Value::Null,
        ),
    ];
    let response_witnesses = [response(json!("ok"))];
    let operation = OperationKey {
        method: "GET".to_owned(),
        path: "/pets/{petId}".to_owned(),
    };

    for (old_name, old) in &documents {
        for (new_name, new) in &documents {
            let report =
                check_openapi_compat(old, new).expect("OpenAPI soundness corpus should compare");
            if !report.is_compatible() {
                continue;
            }

            let old_lowerer = OpenApiOperationLowerer::new(old).expect("old lowerer should build");
            let new_lowerer = OpenApiOperationLowerer::new(new).expect("new lowerer should build");
            let old_operation = old_lowerer
                .lower_operation(&operation)
                .expect("old operation should lower")
                .expect("old operation should exist");
            let new_operation = new_lowerer
                .lower_operation(&operation)
                .expect("new operation should lower")
                .expect("new operation should exist");

            let old_request = old
                .lowered_contract_document(&old_operation.request)
                .expect("old lowered request should build");
            let new_request = new
                .lowered_contract_document(&new_operation.request)
                .expect("new lowered request should build");
            assert_witness_inclusion(
                old_name,
                new_name,
                "request",
                &old_request,
                &new_request,
                &request_witnesses,
            );

            let old_response = old
                .lowered_contract_document(&old_operation.response)
                .expect("old lowered response should build");
            let new_response = new
                .lowered_contract_document(&new_operation.response)
                .expect("new lowered response should build");
            assert_witness_inclusion(
                new_name,
                old_name,
                "response",
                &new_response,
                &old_response,
                &response_witnesses,
            );
        }
    }
}

#[test]
fn claimed_openapi_content_backed_fields_survive_lowered_contract_witnesses() {
    let documents = [
        (
            "content_fields_integer",
            content_backed_field_spec(
                json!({
                    "application/json": {
                        "schema": { "type": "integer" }
                    }
                }),
                json!({
                    "application/json": {
                        "schema": { "type": "integer" }
                    }
                }),
            ),
        ),
        (
            "content_fields_string_or_integer",
            content_backed_field_spec(
                json!({
                    "application/json": {
                        "schema": { "type": ["integer", "string"] }
                    }
                }),
                json!({
                    "application/json": {
                        "schema": { "type": ["integer", "string"] }
                    }
                }),
            ),
        ),
    ]
    .into_iter()
    .map(|(name, raw)| {
        (
            name,
            OpenApiDocument::from_json(&raw)
                .expect("content-backed soundness OpenAPI document should build"),
        )
    })
    .collect::<Vec<_>>();

    let request_witnesses = [
        request_with_parts(
            json!({}),
            content_filter(json_body(json!(1))),
            json!({}),
            json!({}),
            Value::Null,
        ),
        request_with_parts(
            json!({}),
            content_filter(json_body(json!("one"))),
            json!({}),
            json!({}),
            Value::Null,
        ),
    ];
    let response_witnesses = [
        typed_response(
            "200",
            json_body(json!("ok")),
            content_trace(json_body(json!(1))),
        ),
        typed_response(
            "200",
            json_body(json!("ok")),
            content_trace(json_body(json!("one"))),
        ),
    ];
    let operation = OperationKey {
        method: "GET".to_owned(),
        path: "/pets".to_owned(),
    };

    for (old_name, old) in &documents {
        for (new_name, new) in &documents {
            let report =
                check_openapi_compat(old, new).expect("OpenAPI soundness corpus should compare");
            if !report.is_compatible() {
                continue;
            }

            let old_lowerer = OpenApiOperationLowerer::new(old).expect("old lowerer should build");
            let new_lowerer = OpenApiOperationLowerer::new(new).expect("new lowerer should build");
            let old_operation = old_lowerer
                .lower_operation(&operation)
                .expect("old operation should lower")
                .expect("old operation should exist");
            let new_operation = new_lowerer
                .lower_operation(&operation)
                .expect("new operation should lower")
                .expect("new operation should exist");

            let old_request = old
                .lowered_contract_document(&old_operation.request)
                .expect("old lowered request should build");
            let new_request = new
                .lowered_contract_document(&new_operation.request)
                .expect("new lowered request should build");
            assert_witness_inclusion(
                old_name,
                new_name,
                "request",
                &old_request,
                &new_request,
                &request_witnesses,
            );

            let old_response = old
                .lowered_contract_document(&old_operation.response)
                .expect("old lowered response should build");
            let new_response = new
                .lowered_contract_document(&new_operation.response)
                .expect("new lowered response should build");
            assert_witness_inclusion(
                new_name,
                old_name,
                "response",
                &new_response,
                &old_response,
                &response_witnesses,
            );
        }
    }
}

#[test]
fn claimed_openapi_operation_parameter_overrides_survive_lowered_contract_witnesses() {
    let documents = [
        (
            "path_optional_query_limit",
            path_item_parameter_override_spec(
                json!({
                    "name": "limit",
                    "in": "query",
                    "schema": { "type": "integer" }
                }),
                None,
            ),
        ),
        (
            "operation_optional_query_limit",
            path_item_parameter_override_spec(
                json!({
                    "name": "limit",
                    "in": "query",
                    "required": true,
                    "schema": { "type": "integer" }
                }),
                Some(json!({
                    "name": "limit",
                    "in": "query",
                    "schema": { "type": "integer" }
                })),
            ),
        ),
        (
            "operation_required_query_limit",
            path_item_parameter_override_spec(
                json!({
                    "name": "limit",
                    "in": "query",
                    "schema": { "type": "integer" }
                }),
                Some(json!({
                    "name": "limit",
                    "in": "query",
                    "required": true,
                    "schema": { "type": "integer" }
                })),
            ),
        ),
        (
            "operation_string_or_integer_query_limit",
            path_item_parameter_override_spec(
                json!({
                    "name": "limit",
                    "in": "query",
                    "schema": { "type": "integer" }
                }),
                Some(json!({
                    "name": "limit",
                    "in": "query",
                    "schema": { "type": ["integer", "string"] }
                })),
            ),
        ),
    ]
    .into_iter()
    .map(|(name, raw)| {
        (
            name,
            OpenApiDocument::from_json(&raw)
                .expect("parameter-override soundness OpenAPI document should build"),
        )
    })
    .collect::<Vec<_>>();

    let request_witnesses = [
        request(json!({}), Value::Null),
        request(query_limit(json!(1)), Value::Null),
        request(query_limit(json!("one")), Value::Null),
    ];
    let response_witnesses = [response(json!("ok"))];
    let operation = OperationKey {
        method: "GET".to_owned(),
        path: "/pets".to_owned(),
    };

    for (old_name, old) in &documents {
        for (new_name, new) in &documents {
            let report =
                check_openapi_compat(old, new).expect("OpenAPI soundness corpus should compare");
            if !report.is_compatible() {
                continue;
            }

            let old_lowerer = OpenApiOperationLowerer::new(old).expect("old lowerer should build");
            let new_lowerer = OpenApiOperationLowerer::new(new).expect("new lowerer should build");
            let old_operation = old_lowerer
                .lower_operation(&operation)
                .expect("old operation should lower")
                .expect("old operation should exist");
            let new_operation = new_lowerer
                .lower_operation(&operation)
                .expect("new operation should lower")
                .expect("new operation should exist");

            let old_request = old
                .lowered_contract_document(&old_operation.request)
                .expect("old lowered request should build");
            let new_request = new
                .lowered_contract_document(&new_operation.request)
                .expect("new lowered request should build");
            assert_witness_inclusion(
                old_name,
                new_name,
                "request",
                &old_request,
                &new_request,
                &request_witnesses,
            );

            let old_response = old
                .lowered_contract_document(&old_operation.response)
                .expect("old lowered response should build");
            let new_response = new
                .lowered_contract_document(&new_operation.response)
                .expect("new lowered response should build");
            assert_witness_inclusion(
                new_name,
                old_name,
                "response",
                &new_response,
                &old_response,
                &response_witnesses,
            );
        }
    }
}

#[test]
fn claimed_openapi_component_parameter_and_header_refs_survive_lowered_contract_witnesses() {
    let documents = [
        (
            "component_integer_fields",
            component_parameter_header_spec(
                json!({ "type": "integer" }),
                json!({ "type": "integer" }),
            ),
        ),
        (
            "component_request_string_or_integer",
            component_parameter_header_spec(
                json!({ "type": ["integer", "string"] }),
                json!({ "type": "integer" }),
            ),
        ),
        (
            "component_response_string_or_integer",
            component_parameter_header_spec(
                json!({ "type": "integer" }),
                json!({ "type": ["integer", "string"] }),
            ),
        ),
    ]
    .into_iter()
    .map(|(name, raw)| {
        (
            name,
            OpenApiDocument::from_json(&raw)
                .expect("component parameter/header soundness OpenAPI document should build"),
        )
    })
    .collect::<Vec<_>>();

    let request_witnesses = [
        request(json!({}), Value::Null),
        request(query_limit(json!(1)), Value::Null),
        request(query_limit(json!("one")), Value::Null),
    ];
    let response_witnesses = [
        typed_response("200", json_body(json!("ok")), json!({})),
        typed_response(
            "200",
            json_body(json!("ok")),
            json!({
                "x-trace-id": {
                    "value": 1,
                    "explode": false
                }
            }),
        ),
        typed_response(
            "200",
            json_body(json!("ok")),
            json!({
                "x-trace-id": {
                    "value": "one",
                    "explode": false
                }
            }),
        ),
    ];
    let operation = OperationKey {
        method: "GET".to_owned(),
        path: "/pets".to_owned(),
    };

    for (old_name, old) in &documents {
        for (new_name, new) in &documents {
            let report =
                check_openapi_compat(old, new).expect("OpenAPI soundness corpus should compare");
            if !report.is_compatible() {
                continue;
            }

            let old_lowerer = OpenApiOperationLowerer::new(old).expect("old lowerer should build");
            let new_lowerer = OpenApiOperationLowerer::new(new).expect("new lowerer should build");
            let old_operation = old_lowerer
                .lower_operation(&operation)
                .expect("old operation should lower")
                .expect("old operation should exist");
            let new_operation = new_lowerer
                .lower_operation(&operation)
                .expect("new operation should lower")
                .expect("new operation should exist");

            let old_request = old
                .lowered_contract_document(&old_operation.request)
                .expect("old lowered request should build");
            let new_request = new
                .lowered_contract_document(&new_operation.request)
                .expect("new lowered request should build");
            assert_witness_inclusion(
                old_name,
                new_name,
                "request",
                &old_request,
                &new_request,
                &request_witnesses,
            );

            let old_response = old
                .lowered_contract_document(&old_operation.response)
                .expect("old lowered response should build");
            let new_response = new
                .lowered_contract_document(&new_operation.response)
                .expect("new lowered response should build");
            assert_witness_inclusion(
                new_name,
                old_name,
                "response",
                &new_response,
                &old_response,
                &response_witnesses,
            );
        }
    }
}

fn spec(parameters: Value, request_body: Option<Value>, response: Value) -> Value {
    let mut operation = serde_json::Map::new();
    operation.insert("parameters".to_owned(), parameters);
    operation.insert("responses".to_owned(), json!({ "200": response }));
    if let Some(request_body) = request_body {
        operation.insert("requestBody".to_owned(), request_body);
    }

    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "paths": {
            "/pets": {
                "get": Value::Object(operation)
            }
        }
    })
}

fn component_parameter_header_spec(parameter_schema: Value, header_schema: Value) -> Value {
    json!({
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
                    "schema": parameter_schema
                }
            },
            "headers": {
                "Trace": {
                    "schema": header_schema
                }
            }
        },
        "paths": {
            "/pets": {
                "get": {
                    "parameters": [
                        { "$ref": "#/components/parameters/Limit" }
                    ],
                    "responses": {
                        "200": {
                            "description": "ok",
                            "headers": {
                                "X-Trace-Id": {
                                    "$ref": "#/components/headers/Trace"
                                }
                            },
                            "content": {
                                "application/json": {
                                    "schema": { "type": "string" }
                                }
                            }
                        }
                    }
                }
            }
        }
    })
}

fn path_item_parameter_override_spec(
    path_parameter: Value,
    operation_parameter: Option<Value>,
) -> Value {
    let mut operation = serde_json::Map::new();
    if let Some(operation_parameter) = operation_parameter {
        operation.insert("parameters".to_owned(), json!([operation_parameter]));
    }
    operation.insert(
        "responses".to_owned(),
        json!({ "200": response_schema(json!({ "type": "string" })) }),
    );

    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "paths": {
            "/pets": {
                "parameters": [path_parameter],
                "get": Value::Object(operation)
            }
        }
    })
}

fn body_surface_spec(request_body: Value, responses: Value) -> Value {
    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "paths": {
            "/pets": {
                "get": {
                    "requestBody": request_body,
                    "responses": responses
                }
            }
        }
    })
}

fn component_body_response_spec(request_body: Value, response: Value) -> Value {
    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "components": {
            "requestBodies": {
                "PetBody": request_body
            },
            "responses": {
                "PetResponse": response
            }
        },
        "paths": {
            "/pets": {
                "post": {
                    "requestBody": {
                        "$ref": "#/components/requestBodies/PetBody"
                    },
                    "responses": {
                        "200": {
                            "$ref": "#/components/responses/PetResponse"
                        }
                    }
                }
            }
        }
    })
}

fn recursive_component_body_response_spec(node_schema: Value) -> Value {
    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Nodes",
            "version": "1.0.0"
        },
        "components": {
            "schemas": {
                "Node": node_schema
            }
        },
        "paths": {
            "/nodes": {
                "post": {
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/Node"
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": response_schema(json!({
                            "$ref": "#/components/schemas/Node"
                        }))
                    }
                }
            }
        }
    })
}

fn parameter_serialization_spec(
    path_parameter: Value,
    header_parameter: Value,
    cookie_parameter: Value,
) -> Value {
    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Pets",
            "version": "1.0.0"
        },
        "paths": {
            "/pets/{petId}": {
                "get": {
                    "parameters": [
                        path_parameter,
                        header_parameter,
                        cookie_parameter
                    ],
                    "responses": {
                        "200": response_schema(json!({ "type": "string" }))
                    }
                }
            }
        }
    })
}

fn content_backed_field_spec(parameter_content: Value, header_content: Value) -> Value {
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
                        "name": "filter",
                        "in": "query",
                        "content": parameter_content
                    }],
                    "responses": {
                        "200": {
                            "description": "ok",
                            "headers": {
                                "trace": {
                                    "content": header_content
                                }
                            },
                            "content": {
                                "application/json": {
                                    "schema": { "type": "string" }
                                }
                            }
                        }
                    }
                }
            }
        }
    })
}

fn request_body_for(media_type: &str) -> Value {
    json!({
        "required": true,
        "content": {
            media_type: {
                "schema": { "type": "string" }
            }
        }
    })
}

fn response_with(media_type: &str, schema: Value, headers: Value) -> Value {
    json!({
        "description": "ok",
        "headers": headers,
        "content": {
            media_type: {
                "schema": schema
            }
        }
    })
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

fn query_limit(value: Value) -> Value {
    json!({
        "limit": {
            "value": value,
            "style": "form",
            "explode": true,
            "allow_reserved": false,
            "allow_empty_value": false
        }
    })
}

fn query_filter_with_serialization(
    value: &str,
    style: &str,
    explode: bool,
    allow_reserved: bool,
    allow_empty_value: bool,
) -> Value {
    json!({
        "filter": {
            "value": value,
            "style": style,
            "explode": explode,
            "allow_reserved": allow_reserved,
            "allow_empty_value": allow_empty_value
        }
    })
}

fn path_pet_id(value: &str, style: &str, explode: bool) -> Value {
    json!({
        "petId": {
            "value": value,
            "style": style,
            "explode": explode
        }
    })
}

fn header_trace(value: &str, explode: bool) -> Value {
    json!({
        "trace": {
            "value": value,
            "explode": explode
        }
    })
}

fn cookie_session(value: &str, style: &str, explode: bool) -> Value {
    json!({
        "session": {
            "value": value,
            "style": style,
            "explode": explode
        }
    })
}

fn content_filter(body: Value) -> Value {
    json!({
        "filter": {
            "value": body
        }
    })
}

fn content_trace(body: Value) -> Value {
    json!({
        "trace": {
            "value": body
        }
    })
}

fn request(query: Value, body: Value) -> Value {
    json!({
        "path": {},
        "query": query,
        "headers": {},
        "cookies": {},
        "body": body
    })
}

fn request_with_parts(
    path: Value,
    query: Value,
    headers: Value,
    cookies: Value,
    body: Value,
) -> Value {
    json!({
        "path": path,
        "query": query,
        "headers": headers,
        "cookies": cookies,
        "body": body
    })
}

fn response(value: Value) -> Value {
    typed_response("200", json_body(value), json!({}))
}

fn json_body(value: Value) -> Value {
    typed_body("application", "json", value)
}

fn typed_body(media_type: &str, subtype: &str, value: Value) -> Value {
    json!({
        "content_type": {
            "type": media_type,
            "subtype": subtype
        },
        "value": value
    })
}

fn typed_response(status: &str, body: Value, headers: Value) -> Value {
    json!({
        "status": status,
        "body": body,
        "headers": headers
    })
}

fn assert_witness_inclusion(
    source_name: &str,
    target_name: &str,
    surface: &str,
    source: &jsoncompat::SchemaDocument,
    target: &jsoncompat::SchemaDocument,
    witnesses: &[Value],
) {
    for witness in witnesses {
        if source
            .is_valid(witness)
            .expect("source lowered contract should validate witnesses")
        {
            assert!(
                target
                    .is_valid(witness)
                    .expect("target lowered contract should validate witnesses"),
                "OpenAPI compatibility claimed {surface} {source_name} ⊆ {target_name}, but target rejected witness {witness}",
            );
        }
    }
}
