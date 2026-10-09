//! OpenAPI document and component shape validation, before contract lowering.
use super::*;

pub(super) fn is_supported_openapi_31_version(version: &str) -> bool {
    version
        .strip_prefix(OPENAPI_31_PREFIX)
        .is_some_and(|patch| !patch.is_empty() && patch.bytes().all(|byte| byte.is_ascii_digit()))
}

pub(super) fn validate_info_fields(
    info: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    for field in info.keys() {
        if field.starts_with("x-")
            || matches!(
                field.as_str(),
                "title"
                    | "version"
                    | "summary"
                    | "description"
                    | "termsOfService"
                    | "contact"
                    | "license"
            )
        {
            continue;
        }

        return Err(invalid_value(
            &pointer.child(field),
            "a supported OpenAPI info field or specification extension beginning with 'x-'",
        ));
    }

    validate_optional_string_field(info, "summary", pointer)?;
    validate_optional_string_field(info, "description", pointer)?;
    validate_optional_url_reference_field(info, "termsOfService", pointer)?;
    validate_optional_contact_field(info, pointer)?;
    validate_optional_license_field(info, pointer)?;

    Ok(())
}

pub(super) fn validate_document_fields(object: &Map<String, Value>) -> Result<(), OpenApiError> {
    for field in object.keys() {
        if field.starts_with("x-")
            || matches!(
                field.as_str(),
                "openapi"
                    | "info"
                    | "jsonSchemaDialect"
                    | "paths"
                    | "components"
                    | "webhooks"
                    | "servers"
                    | "security"
                    | "tags"
                    | "externalDocs"
            )
        {
            continue;
        }

        let pointer = JsonPointer::root().child(field);
        return Err(invalid_value(
            &pointer,
            "a supported OpenAPI document field or specification extension beginning with 'x-'",
        ));
    }

    validate_optional_servers_field(object, &JsonPointer::root())?;
    validate_optional_security_field(object, &JsonPointer::root())?;
    validate_optional_tags_field(object, &JsonPointer::root())?;
    validate_optional_external_docs_field(object, &JsonPointer::root())?;
    validate_optional_webhooks_field(object, &JsonPointer::root())?;

    Ok(())
}

pub(super) fn validate_document_schema_dialect_field(
    object: &Map<String, Value>,
) -> Result<(), OpenApiError> {
    validate_optional_absolute_uri_field(object, "jsonSchemaDialect", &JsonPointer::root())
}

pub(super) fn validate_optional_webhooks_field(
    object: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    if let Some(webhooks) = object.get("webhooks")
        && !webhooks.is_object()
    {
        return Err(invalid_value(&pointer.child("webhooks"), "an object"));
    }
    Ok(())
}

pub(super) fn validate_contract_container_shapes(
    object: &Map<String, Value>,
) -> Result<(), OpenApiError> {
    if let Some(paths) = object.get("paths").and_then(Value::as_object) {
        validate_path_item_container_shapes(paths, &JsonPointer::root().child("paths"), true)?;
    }
    if let Some(webhooks) = object.get("webhooks").and_then(Value::as_object) {
        validate_path_item_container_shapes(
            webhooks,
            &JsonPointer::root().child("webhooks"),
            false,
        )?;
    }
    if let Some(components) = object.get("components").and_then(Value::as_object) {
        validate_component_document_fields(components, &JsonPointer::root().child("components"))?;
        validate_component_collection_container_shapes(
            components,
            &JsonPointer::root().child("components"),
        )?;
        validate_component_collection_document_shapes(
            components,
            &JsonPointer::root().child("components"),
        )?;
    }
    Ok(())
}

pub(super) fn validate_path_template_parameter_bindings(
    document: &Value,
    object: &Map<String, Value>,
) -> Result<(), OpenApiError> {
    let Some(paths) = object.get("paths").and_then(Value::as_object) else {
        return Ok(());
    };
    let paths_pointer = JsonPointer::root().child("paths");
    for (path, raw_path_item) in paths {
        if path.starts_with("x-") {
            continue;
        }
        let path_pointer = paths_pointer.child(path);
        let path_template_names = path_template_names(path, &path_pointer)?;
        let Some(path_item) = raw_path_item.as_object() else {
            continue;
        };
        if path_item.contains_key("$ref") {
            continue;
        }

        let path_parameters = collect_path_template_parameter_bindings(
            document,
            path_item.get("parameters"),
            &path_pointer.child("parameters"),
            &path_template_names,
        )?;
        for method in HTTP_METHODS {
            let Some(operation) = path_item.get(method).and_then(Value::as_object) else {
                continue;
            };
            let operation_pointer = path_pointer.child(method);
            let operation_parameters = collect_path_template_parameter_bindings(
                document,
                operation.get("parameters"),
                &operation_pointer.child("parameters"),
                &path_template_names,
            )?;
            require_document_path_template_parameters(
                &path_template_names,
                &path_parameters,
                &operation_parameters,
                &operation_pointer.child("parameters"),
            )?;
        }
    }
    Ok(())
}

pub(super) fn validate_locally_referenced_request_body_encoding_keys(
    document: &Value,
    object: &Map<String, Value>,
) -> Result<(), OpenApiError> {
    if let Some(paths) = object.get("paths").and_then(Value::as_object) {
        validate_path_item_request_body_encoding_keys(
            document,
            paths,
            &JsonPointer::root().child("paths"),
            true,
        )?;
    }
    if let Some(webhooks) = object.get("webhooks").and_then(Value::as_object) {
        validate_path_item_request_body_encoding_keys(
            document,
            webhooks,
            &JsonPointer::root().child("webhooks"),
            false,
        )?;
    }
    if let Some(request_bodies) = object
        .get("components")
        .and_then(Value::as_object)
        .and_then(|components| components.get("requestBodies"))
        .and_then(Value::as_object)
    {
        let pointer = JsonPointer::root()
            .child("components")
            .child("requestBodies");
        for (name, request_body) in request_bodies {
            validate_request_body_encoding_keys_against_locally_referenced_media_schema(
                document,
                request_body,
                &pointer.child(name),
            )?;
        }
    }
    Ok(())
}

pub(super) fn validate_path_item_request_body_encoding_keys(
    document: &Value,
    path_items: &Map<String, Value>,
    pointer: &JsonPointer,
    allow_extension_entries: bool,
) -> Result<(), OpenApiError> {
    for (path, raw_path_item) in path_items {
        if allow_extension_entries && path.starts_with("x-") {
            continue;
        }
        let Some(path_item) = raw_path_item.as_object() else {
            continue;
        };
        if path_item.contains_key("$ref") {
            continue;
        }
        let path_pointer = pointer.child(path);
        for method in HTTP_METHODS {
            let Some(operation) = path_item.get(method).and_then(Value::as_object) else {
                continue;
            };
            if let Some(request_body) = operation.get("requestBody") {
                validate_request_body_encoding_keys_against_locally_referenced_media_schema(
                    document,
                    request_body,
                    &path_pointer.child(method).child("requestBody"),
                )?;
            }
        }
    }
    Ok(())
}

pub(super) fn validate_request_body_encoding_keys_against_locally_referenced_media_schema(
    document: &Value,
    request_body: &Value,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let request_body = match resolve_reference_chain(document, request_body, pointer)? {
        ReferenceResolution::Value(request_body) => request_body,
        ReferenceResolution::ExternalReference { .. } => return Ok(()),
    };
    let Some(content) = request_body
        .as_object()
        .and_then(|request_body| request_body.get("content"))
        .and_then(Value::as_object)
    else {
        return Ok(());
    };

    for (media_type, media) in content {
        let Some(media) = media.as_object() else {
            continue;
        };
        validate_encoding_keys_against_locally_referenced_media_schema_properties(
            document,
            media,
            &pointer.child("content").child(media_type),
        )?;
    }
    Ok(())
}

pub(super) fn validate_encoding_keys_against_locally_referenced_media_schema_properties(
    document: &Value,
    media: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let Some(encoding) = media.get("encoding").and_then(Value::as_object) else {
        return Ok(());
    };
    let Some(schema) = media.get("schema") else {
        return Ok(());
    };
    let Some(schema_object) = schema.as_object() else {
        return Ok(());
    };
    if schema_object.len() != 1 || schema_object.get("$ref").and_then(Value::as_str).is_none() {
        return Ok(());
    }

    let schema_pointer = pointer.child("schema");
    let schema = match resolve_reference_chain(document, schema, &schema_pointer)? {
        ReferenceResolution::Value(schema) => schema,
        ReferenceResolution::ExternalReference { .. } => return Ok(()),
    };
    let Some(properties) = schema
        .as_object()
        .and_then(|schema| schema.get("properties"))
        .and_then(Value::as_object)
    else {
        return Ok(());
    };

    for name in encoding.keys() {
        if !properties.contains_key(name) {
            return Err(invalid_value(
                &pointer.child("encoding").child(name),
                "a property declared by the media type schema",
            ));
        }
    }

    Ok(())
}

#[derive(Debug, Default)]
pub(super) struct DocumentParameterBindings {
    path_names: BTreeSet<String>,
    has_external_reference: bool,
}

pub(super) fn collect_path_template_parameter_bindings(
    document: &Value,
    raw: Option<&Value>,
    pointer: &JsonPointer,
    path_template_names: &BTreeSet<String>,
) -> Result<DocumentParameterBindings, OpenApiError> {
    let Some(raw) = raw else {
        return Ok(DocumentParameterBindings::default());
    };
    let parameters = raw
        .as_array()
        .ok_or_else(|| invalid_value(pointer, "an array"))?;
    let mut bindings = DocumentParameterBindings::default();
    let mut identities = BTreeSet::new();
    for (index, raw_parameter) in parameters.iter().enumerate() {
        let parameter_pointer = pointer.child(index.to_string());
        let parameter = match resolve_reference_chain(document, raw_parameter, &parameter_pointer)?
        {
            ReferenceResolution::Value(parameter) => parameter,
            ReferenceResolution::ExternalReference { .. } => {
                bindings.has_external_reference = true;
                continue;
            }
        };
        let parameter = parameter
            .as_object()
            .ok_or_else(|| invalid_value(&parameter_pointer, "an object or local reference"))?;
        let name = parameter
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid_value(&parameter_pointer.child("name"), "a string"))?;
        let location = ParameterLocation::from_value(
            parameter
                .get("in")
                .and_then(Value::as_str)
                .ok_or_else(|| invalid_value(&parameter_pointer.child("in"), "a string"))?,
            &parameter_pointer.child("in"),
        )?;
        let identity = (location, parameter_identity_name(location, name));
        if !identities.insert(identity.clone()) {
            return Err(OpenApiError::DuplicateParameter {
                pointer: parameter_pointer.render(),
                location: identity.0.field_name().to_owned(),
                name: identity.1,
            });
        }
        if location == ParameterLocation::Path {
            if !path_template_names.contains(name) {
                return Err(invalid_value(
                    &parameter_pointer.child("name"),
                    "a template expression that appears in the path key",
                ));
            }
            bindings.path_names.insert(name.to_owned());
        }
    }
    Ok(bindings)
}

pub(super) fn require_document_path_template_parameters(
    path_template_names: &BTreeSet<String>,
    path_parameters: &DocumentParameterBindings,
    operation_parameters: &DocumentParameterBindings,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    if path_parameters.has_external_reference || operation_parameters.has_external_reference {
        return Ok(());
    }
    for template_name in path_template_names {
        if !path_parameters.path_names.contains(template_name)
            && !operation_parameters.path_names.contains(template_name)
        {
            return Err(invalid_value(
                pointer,
                "path parameters covering every template expression in the path key",
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_path_item_container_shapes(
    path_items: &Map<String, Value>,
    pointer: &JsonPointer,
    allow_extension_entries: bool,
) -> Result<(), OpenApiError> {
    for (path, raw_path_item) in path_items {
        if allow_extension_entries && path.starts_with("x-") {
            continue;
        }
        let path_pointer = pointer.child(path);
        let path_item = raw_path_item
            .as_object()
            .ok_or_else(|| invalid_value(&path_pointer, "an object or local reference"))?;
        validate_path_item_document_fields(path_item, &path_pointer)?;
        validate_parameter_array_document_shapes(
            path_item.get("parameters"),
            &path_pointer.child("parameters"),
        )?;
        for method in HTTP_METHODS {
            let Some(raw_operation) = path_item.get(method) else {
                continue;
            };
            let operation_pointer = path_pointer.child(method);
            let operation = raw_operation
                .as_object()
                .ok_or_else(|| invalid_value(&operation_pointer, "an object"))?;
            validate_operation_container_shapes(operation, &operation_pointer)?;
            validate_operation_response_document_shapes(operation, &operation_pointer)?;
        }
    }
    Ok(())
}

pub(super) fn validate_operation_container_shapes(
    operation: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    validate_operation_document_fields(operation, pointer)?;
    validate_parameter_array_document_shapes(
        operation.get("parameters"),
        &pointer.child("parameters"),
    )?;
    if operation
        .get("requestBody")
        .is_some_and(|request_body| !request_body.is_object())
    {
        return Err(invalid_value(
            &pointer.child("requestBody"),
            "an object or local reference",
        ));
    }
    if let Some(request_body) = operation.get("requestBody") {
        validate_request_body_document_shape(request_body, &pointer.child("requestBody"))?;
    }
    if operation
        .get("responses")
        .is_some_and(|responses| !responses.is_object())
    {
        return Err(invalid_value(&pointer.child("responses"), "an object"));
    }
    Ok(())
}

pub(super) fn validate_parameter_array_document_shapes(
    raw: Option<&Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let Some(raw) = raw else {
        return Ok(());
    };
    let parameters = raw
        .as_array()
        .ok_or_else(|| invalid_value(pointer, "an array"))?;
    for (index, parameter) in parameters.iter().enumerate() {
        validate_parameter_document_shape(parameter, &pointer.child(index.to_string()))?;
    }
    Ok(())
}

pub(super) fn validate_parameter_document_shape(
    parameter: &Value,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let parameter = parameter
        .as_object()
        .ok_or_else(|| invalid_value(pointer, "an object or local reference"))?;
    if let Some(reference) = parameter.get("$ref") {
        if !reference.is_string() {
            return Err(invalid_value(&pointer.child("$ref"), "a string"));
        }
        return validate_reference_object_fields(parameter, pointer);
    }

    validate_parameter_fields(parameter, pointer)?;
    parameter
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_value(&pointer.child("name"), "a string"))?;
    let location = ParameterLocation::from_value(
        parameter
            .get("in")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid_value(&pointer.child("in"), "a string"))?,
        &pointer.child("in"),
    )?;
    let required = parameter
        .get("required")
        .map(|value| {
            value
                .as_bool()
                .ok_or_else(|| invalid_value(&pointer.child("required"), "a boolean"))
        })
        .transpose()?
        .unwrap_or(false);
    if location == ParameterLocation::Path && !required {
        return Err(invalid_value(
            &pointer.child("required"),
            "true for path parameters",
        ));
    }
    reject_query_only_metadata_outside_query(location, parameter, pointer)?;
    match (parameter.get("schema"), parameter.get("content")) {
        (Some(schema), None) => {
            validate_schema_document_shape(schema, &pointer.child("schema"))?;
            validate_parameter_schema_serialization_fields(location, parameter, pointer)
        }
        (None, Some(content)) => {
            reject_content_serialization_fields(
                parameter,
                pointer,
                &[
                    "style",
                    "explode",
                    "allowReserved",
                    "allowEmptyValue",
                    "example",
                    "examples",
                ],
            )?;
            validate_single_content_document_shape(
                content,
                &pointer.child("content"),
                MediaTypeEncodingContext::NonRequestBody,
            )
        }
        _ => Err(invalid_value(
            pointer,
            "exactly one of `schema` or `content`",
        )),
    }
}

pub(super) fn validate_request_body_document_shape(
    request_body: &Value,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let request_body = request_body
        .as_object()
        .ok_or_else(|| invalid_value(pointer, "an object or local reference"))?;
    if let Some(reference) = request_body.get("$ref") {
        if !reference.is_string() {
            return Err(invalid_value(&pointer.child("$ref"), "a string"));
        }
        return validate_reference_object_fields(request_body, pointer);
    }
    validate_request_body_fields(request_body, pointer)?;
    if !request_body.get("content").is_some_and(Value::is_object) {
        return Err(invalid_value(&pointer.child("content"), "an object"));
    }
    validate_content_document_shapes(
        request_body
            .get("content")
            .expect("content existence was checked above"),
        &pointer.child("content"),
        MediaTypeEncodingContext::RequestBody,
    )?;
    Ok(())
}

pub(super) fn validate_operation_response_document_shapes(
    operation: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    if operation.contains_key("$ref") {
        return Err(invalid_value(
            &pointer.child("$ref"),
            "a supported OpenAPI operation field or specification extension beginning with 'x-'",
        ));
    }

    let responses_pointer = pointer.child("responses");
    let responses = operation
        .get("responses")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            invalid_value(
                &responses_pointer,
                "an object containing at least one response",
            )
        })?;
    if responses.keys().all(|status| status.starts_with("x-")) {
        return Err(invalid_value(
            &responses_pointer,
            "an object containing at least one response",
        ));
    }

    for (status, response) in responses {
        if status.starts_with("x-") {
            continue;
        }
        let response_pointer = responses_pointer.child(status);
        validate_response_status_selector(status, &response_pointer)?;
        validate_response_document_shape(response, &response_pointer)?;
    }
    Ok(())
}

pub(super) fn validate_response_document_shape(
    response: &Value,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let response = response
        .as_object()
        .ok_or_else(|| invalid_value(pointer, "an object or local reference"))?;
    if let Some(reference) = response.get("$ref") {
        if !reference.is_string() {
            return Err(invalid_value(&pointer.child("$ref"), "a string"));
        }
        return validate_reference_object_fields(response, pointer);
    }

    validate_response_document_fields(response, pointer)?;
    response
        .get("description")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_value(&pointer.child("description"), "a string"))?;
    if let Some(headers) = response.get("headers") {
        validate_response_headers_document_shapes(headers, &pointer.child("headers"))?;
    }
    if let Some(content) = response.get("content") {
        validate_content_document_shapes(
            content,
            &pointer.child("content"),
            MediaTypeEncodingContext::NonRequestBody,
        )?;
    }
    Ok(())
}

pub(super) fn validate_component_collection_container_shapes(
    components: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    for field in [
        "schemas",
        "parameters",
        "requestBodies",
        "responses",
        "headers",
        "securitySchemes",
        "examples",
        "links",
        "callbacks",
        "pathItems",
    ] {
        let Some(collection) = components.get(field) else {
            continue;
        };
        let collection = collection
            .as_object()
            .ok_or_else(|| invalid_value(&pointer.child(field), "an object"))?;
        validate_component_collection_names(collection, &pointer.child(field))?;
    }
    Ok(())
}

pub(super) fn validate_component_collection_document_shapes(
    components: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    if let Some(schemas) = components.get("schemas").and_then(Value::as_object) {
        for (name, schema) in schemas {
            validate_schema_document_shape(schema, &pointer.child("schemas").child(name))?;
        }
    }

    for (field, validate_entry) in [
        (
            "parameters",
            validate_parameter_document_shape
                as fn(&Value, &JsonPointer) -> Result<(), OpenApiError>,
        ),
        ("requestBodies", validate_request_body_document_shape),
        ("responses", validate_response_document_shape),
        ("headers", validate_header_document_shape),
        ("securitySchemes", validate_security_scheme_document_shape),
    ] {
        let Some(collection) = components.get(field).and_then(Value::as_object) else {
            continue;
        };
        for (name, entry) in collection {
            validate_entry(entry, &pointer.child(field).child(name))?;
        }
    }

    for field in ["examples", "links", "callbacks", "pathItems"] {
        if let Some(collection) = components.get(field) {
            validate_unsupported_component_collection(field, collection, &pointer.child(field))?;
        }
    }

    Ok(())
}

pub(super) fn validate_paths_object_shape(paths: &Map<String, Value>) -> Result<(), OpenApiError> {
    let paths_pointer = JsonPointer::root().child("paths");
    let mut templated_shapes = BTreeMap::new();
    for path in paths.keys() {
        if path.starts_with("x-") {
            continue;
        }
        let path_pointer = paths_pointer.child(path);
        if !path.starts_with('/') {
            return Err(invalid_value(
                &path_pointer,
                "a path template key beginning with '/' or a specification extension beginning with 'x-'",
            ));
        }
        let template_shape = normalized_path_template_shape(path, &path_pointer)?;
        if templated_shapes
            .insert(template_shape, path.clone())
            .is_some()
        {
            return Err(invalid_value(
                &path_pointer,
                "a path template shape that is not already declared with different parameter names",
            ));
        }
    }
    Ok(())
}

pub(super) fn normalized_path_template_shape(
    path: &str,
    pointer: &JsonPointer,
) -> Result<String, OpenApiError> {
    let mut normalized = String::with_capacity(path.len());
    let mut rest = path;
    while let Some(open_index) = rest.find('{') {
        normalized.push_str(&rest[..open_index]);
        let after_open = &rest[open_index + 1..];
        let Some(close_index) = after_open.find('}') else {
            return Err(invalid_value(
                pointer,
                "a path key with balanced non-empty template expressions",
            ));
        };
        let name = &after_open[..close_index];
        if name.is_empty() || name.contains('{') {
            return Err(invalid_value(
                pointer,
                "a path key with balanced non-empty template expressions",
            ));
        }
        normalized.push_str("{}");
        rest = &after_open[close_index + 1..];
    }
    if rest.contains('}') {
        return Err(invalid_value(
            pointer,
            "a path key with balanced non-empty template expressions",
        ));
    }
    normalized.push_str(rest);
    Ok(normalized)
}

pub(super) fn validate_path_item_document_fields(
    path_item: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    for (field, value) in path_item {
        if field.starts_with("x-")
            || matches!(
                field.as_str(),
                "parameters" | "servers" | "summary" | "description"
            )
            || HTTP_METHODS.contains(&field.as_str())
        {
            continue;
        }

        if field == "$ref" {
            if !value.is_string() {
                return Err(invalid_value(&pointer.child(field), "a string"));
            }
            continue;
        }

        return Err(invalid_value(
            &pointer.child(field),
            "a supported OpenAPI path item field or specification extension beginning with 'x-'",
        ));
    }

    validate_optional_servers_field(path_item, pointer)?;
    validate_optional_string_field(path_item, "summary", pointer)?;
    validate_optional_string_field(path_item, "description", pointer)?;

    Ok(())
}

pub(super) fn validate_path_item_fields(
    path_item: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    validate_path_item_document_fields(path_item, pointer)?;
    if path_item.contains_key("$ref") {
        return Err(unsupported_compatibility_feature(
            &pointer.child("$ref"),
            "path item references",
        ));
    }
    Ok(())
}

pub(super) fn validate_operation_document_fields(
    operation: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    for field in operation.keys() {
        if field.starts_with("x-")
            || matches!(
                field.as_str(),
                "parameters"
                    | "requestBody"
                    | "responses"
                    | "security"
                    | "servers"
                    | "tags"
                    | "summary"
                    | "description"
                    | "externalDocs"
                    | "operationId"
                    | "deprecated"
            )
        {
            continue;
        }

        let field_pointer = pointer.child(field);
        if field == "callbacks" {
            validate_callbacks_field(
                operation
                    .get(field)
                    .expect("field is present while iterating operation keys"),
                &field_pointer,
            )?;
            continue;
        }
        return Err(invalid_value(
            &field_pointer,
            "a supported OpenAPI operation field or specification extension beginning with 'x-'",
        ));
    }

    validate_optional_security_field(operation, pointer)?;
    validate_optional_servers_field(operation, pointer)?;
    validate_optional_string_array_field(operation, "tags", pointer)?;
    validate_optional_string_field(operation, "summary", pointer)?;
    validate_optional_string_field(operation, "description", pointer)?;
    validate_optional_external_docs_field(operation, pointer)?;
    validate_optional_string_field(operation, "operationId", pointer)?;
    validate_optional_bool_field(operation, "deprecated", pointer)?;

    Ok(())
}

pub(super) fn validate_operation_fields(
    operation: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    validate_operation_document_fields(operation, pointer)?;
    if operation.contains_key("callbacks") {
        return Err(unsupported_compatibility_feature(
            &pointer.child("callbacks"),
            "operation callbacks",
        ));
    }
    Ok(())
}

pub(super) fn validate_callbacks_field(
    callbacks: &Value,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let callbacks = callbacks
        .as_object()
        .ok_or_else(|| invalid_value(pointer, "an object"))?;
    for (name, callback) in callbacks {
        validate_callback_object_or_reference(callback, &pointer.child(name))?;
    }
    Ok(())
}

pub(super) fn validate_callback_object_or_reference(
    callback: &Value,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let callback = callback
        .as_object()
        .ok_or_else(|| invalid_value(pointer, "an OpenAPI Callback Object or Reference Object"))?;
    if let Some(reference) = callback.get("$ref") {
        if !reference.is_string() {
            return Err(invalid_value(&pointer.child("$ref"), "a string"));
        }
        return validate_reference_object_fields(callback, pointer);
    }
    validate_callback_key_expressions(callback, pointer)?;
    validate_path_item_container_shapes(callback, pointer, true)
}

pub(super) fn validate_callback_key_expressions(
    callback: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    for key in callback.keys() {
        if key.starts_with("x-") {
            continue;
        }
        if !is_valid_callback_key(key) {
            return Err(invalid_value(
                &pointer.child(key),
                "a callback key formed from a valid runtime expression or a URL template containing runtime expressions",
            ));
        }
    }
    Ok(())
}

pub(super) fn is_valid_callback_key(key: &str) -> bool {
    if is_valid_runtime_expression(key) {
        return true;
    }

    let mut normalized = String::with_capacity(key.len());
    let mut rest = key;
    let mut saw_expression = false;
    while let Some(open_index) = rest.find('{') {
        let before_open = &rest[..open_index];
        if before_open.contains('}') {
            return false;
        }
        normalized.push_str(before_open);

        let after_open = &rest[open_index + 1..];
        let Some(close_index) = after_open.find('}') else {
            return false;
        };
        let expression = &after_open[..close_index];
        if expression.contains('{') || !is_valid_runtime_expression(expression) {
            return false;
        }
        normalized.push('1');
        saw_expression = true;
        rest = &after_open[close_index + 1..];
    }
    if rest.contains('}') {
        return false;
    }
    normalized.push_str(rest);

    saw_expression && is_valid_reference(&normalized)
}

pub(super) fn is_valid_runtime_expression(expression: &str) -> bool {
    if matches!(expression, "$url" | "$method" | "$statusCode") {
        return true;
    }

    let Some(source) = expression
        .strip_prefix("$request.")
        .or_else(|| expression.strip_prefix("$response."))
    else {
        return false;
    };

    if let Some(token) = source.strip_prefix("header.") {
        return is_valid_header_token(token);
    }
    if source.starts_with("query.") || source.starts_with("path.") {
        return true;
    }
    source == "body"
        || source
            .strip_prefix("body#")
            .is_some_and(is_valid_runtime_json_pointer)
}

pub(super) fn is_valid_header_token(token: &str) -> bool {
    !token.is_empty()
        && token.chars().all(|character| {
            character.is_ascii_alphanumeric()
                || matches!(
                    character,
                    '!' | '#'
                        | '$'
                        | '%'
                        | '&'
                        | '\''
                        | '*'
                        | '+'
                        | '-'
                        | '.'
                        | '^'
                        | '_'
                        | '`'
                        | '|'
                        | '~'
                )
        })
}

pub(super) fn is_valid_runtime_json_pointer(pointer: &str) -> bool {
    if !pointer.is_empty() && !pointer.starts_with('/') {
        return false;
    }

    let mut characters = pointer.chars();
    while let Some(character) = characters.next() {
        if character == '~' && !matches!(characters.next(), Some('0' | '1')) {
            return false;
        }
    }

    true
}

pub(super) fn validate_component_fields(
    components: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    validate_component_document_fields(components, pointer)?;
    for field in ["examples", "links", "callbacks", "pathItems"] {
        if components.contains_key(field) {
            return Err(unsupported_compatibility_feature(
                &pointer.child(field),
                "this OpenAPI component collection",
            ));
        }
    }

    validate_optional_object_field(components, "securitySchemes", pointer)?;

    Ok(())
}

pub(super) fn validate_unsupported_component_collection(
    field: &str,
    collection: &Value,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let collection = collection
        .as_object()
        .ok_or_else(|| invalid_value(pointer, "an object"))?;
    match field {
        "examples" => {
            for (name, example) in collection {
                validate_example_or_reference_object(example, &pointer.child(name))?;
            }
            Ok(())
        }
        "links" => {
            for (name, link) in collection {
                validate_link_object_or_reference(link, &pointer.child(name))?;
            }
            Ok(())
        }
        "callbacks" => {
            for (name, callback) in collection {
                validate_callback_object_or_reference(callback, &pointer.child(name))?;
            }
            Ok(())
        }
        "pathItems" => validate_path_item_container_shapes(collection, pointer, false),
        _ => unreachable!("caller only passes unsupported component collections"),
    }
}

pub(super) fn validate_component_document_fields(
    components: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    for field in components.keys() {
        if field.starts_with("x-")
            || matches!(
                field.as_str(),
                "schemas"
                    | "parameters"
                    | "requestBodies"
                    | "responses"
                    | "headers"
                    | "securitySchemes"
                    | "examples"
                    | "links"
                    | "callbacks"
                    | "pathItems"
            )
        {
            continue;
        }

        return Err(invalid_value(
            &pointer.child(field),
            "an OpenAPI component collection or specification extension beginning with 'x-'",
        ));
    }

    Ok(())
}

pub(super) fn validate_optional_string_field(
    object: &Map<String, Value>,
    field: &str,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    if object.get(field).is_some_and(|value| !value.is_string()) {
        return Err(invalid_value(&pointer.child(field), "a string"));
    }

    Ok(())
}

pub(super) fn validate_optional_url_reference_field(
    object: &Map<String, Value>,
    field: &str,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    validate_optional_reference_field(object, field, pointer, "a valid URL reference")
}

pub(super) fn validate_optional_uri_reference_field(
    object: &Map<String, Value>,
    field: &str,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    validate_optional_reference_field(object, field, pointer, "a valid URI reference")
}

pub(super) fn validate_optional_reference_field(
    object: &Map<String, Value>,
    field: &str,
    pointer: &JsonPointer,
    expected: &'static str,
) -> Result<(), OpenApiError> {
    let Some(value) = object.get(field) else {
        return Ok(());
    };
    let Some(value) = value.as_str() else {
        return Err(invalid_value(&pointer.child(field), expected));
    };

    if is_valid_reference(value) {
        Ok(())
    } else {
        Err(invalid_value(&pointer.child(field), expected))
    }
}

pub(super) fn validate_optional_absolute_uri_field(
    object: &Map<String, Value>,
    field: &str,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let Some(value) = object.get(field) else {
        return Ok(());
    };
    let Some(value) = value.as_str() else {
        return Err(invalid_value(&pointer.child(field), "an absolute URI"));
    };

    if Url::parse(value).is_ok() {
        Ok(())
    } else {
        Err(invalid_value(&pointer.child(field), "an absolute URI"))
    }
}

pub(super) fn validate_optional_email_field(
    object: &Map<String, Value>,
    field: &str,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let Some(value) = object.get(field) else {
        return Ok(());
    };
    let Some(value) = value.as_str() else {
        return Err(invalid_value(
            &pointer.child(field),
            "a valid email address",
        ));
    };

    if EmailAddress::is_valid(value) {
        Ok(())
    } else {
        Err(invalid_value(
            &pointer.child(field),
            "a valid email address",
        ))
    }
}

pub(super) fn is_valid_reference(value: &str) -> bool {
    Url::parse(value).is_ok()
        || Url::parse("https://jsoncompat.invalid/")
            .expect("static URL base is valid")
            .join(value)
            .is_ok()
}

pub(super) fn validate_optional_bool_field(
    object: &Map<String, Value>,
    field: &str,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    if object.get(field).is_some_and(|value| !value.is_boolean()) {
        return Err(invalid_value(&pointer.child(field), "a boolean"));
    }

    Ok(())
}

pub(super) fn validate_optional_object_field(
    object: &Map<String, Value>,
    field: &str,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    if object.get(field).is_some_and(|value| !value.is_object()) {
        return Err(invalid_value(&pointer.child(field), "an object"));
    }

    Ok(())
}

pub(super) fn validate_optional_string_array_field(
    object: &Map<String, Value>,
    field: &str,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let Some(values) = object.get(field) else {
        return Ok(());
    };
    let array = values
        .as_array()
        .ok_or_else(|| invalid_value(&pointer.child(field), "an array of strings"))?;
    if let Some(index) = array.iter().position(|value| !value.is_string()) {
        return Err(invalid_value(
            &pointer.child(field).child(index.to_string()),
            "a string",
        ));
    }

    Ok(())
}

pub(super) fn validate_optional_contact_field(
    info: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let Some(contact) = info.get("contact") else {
        return Ok(());
    };
    let pointer = pointer.child("contact");
    let contact = contact
        .as_object()
        .ok_or_else(|| invalid_value(&pointer, "an object"))?;
    validate_object_fields(
        contact,
        &pointer,
        &["name", "url", "email"],
        "a supported OpenAPI contact field or specification extension beginning with 'x-'",
    )?;
    validate_optional_string_field(contact, "name", &pointer)?;
    validate_optional_url_reference_field(contact, "url", &pointer)?;
    validate_optional_email_field(contact, "email", &pointer)?;
    Ok(())
}

pub(super) fn validate_optional_license_field(
    info: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let Some(license) = info.get("license") else {
        return Ok(());
    };
    let pointer = pointer.child("license");
    let license = license
        .as_object()
        .ok_or_else(|| invalid_value(&pointer, "an object"))?;
    validate_object_fields(
        license,
        &pointer,
        &["name", "identifier", "url"],
        "a supported OpenAPI license field or specification extension beginning with 'x-'",
    )?;
    require_string_field(license, "name", &pointer)?;
    validate_optional_string_field(license, "identifier", &pointer)?;
    validate_optional_url_reference_field(license, "url", &pointer)?;
    if license.contains_key("identifier") && license.contains_key("url") {
        return Err(invalid_value(
            &pointer,
            "at most one of `identifier` or `url`",
        ));
    }
    Ok(())
}

pub(super) fn validate_optional_external_docs_field(
    object: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let Some(external_docs) = object.get("externalDocs") else {
        return Ok(());
    };
    let pointer = pointer.child("externalDocs");
    let external_docs = external_docs
        .as_object()
        .ok_or_else(|| invalid_value(&pointer, "an object"))?;
    validate_object_fields(
        external_docs,
        &pointer,
        &["description", "url"],
        "a supported OpenAPI external-docs field or specification extension beginning with 'x-'",
    )?;
    require_url_reference_field(external_docs, "url", &pointer)?;
    validate_optional_string_field(external_docs, "description", &pointer)?;
    Ok(())
}

pub(super) fn validate_optional_servers_field(
    object: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let Some(servers) = object.get("servers") else {
        return Ok(());
    };
    let servers = servers
        .as_array()
        .ok_or_else(|| invalid_value(&pointer.child("servers"), "an array"))?;
    for (index, server) in servers.iter().enumerate() {
        validate_server_object(server, &pointer.child("servers").child(index.to_string()))?;
    }
    Ok(())
}

pub(super) fn validate_server_object(
    server: &Value,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let server = server
        .as_object()
        .ok_or_else(|| invalid_value(pointer, "an object"))?;
    validate_object_fields(
        server,
        pointer,
        &["url", "description", "variables"],
        "a supported OpenAPI server field or specification extension beginning with 'x-'",
    )?;
    require_server_url_field(server, pointer)?;
    validate_optional_string_field(server, "description", pointer)?;
    let Some(variables) = server.get("variables") else {
        return Ok(());
    };
    let variables_pointer = pointer.child("variables");
    let variables = variables
        .as_object()
        .ok_or_else(|| invalid_value(&variables_pointer, "an object"))?;
    for (name, variable) in variables {
        validate_server_variable_object(variable, &variables_pointer.child(name))?;
    }
    Ok(())
}

pub(super) fn require_server_url_field(
    server: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let url = require_string_field_value(server, "url", pointer)?;
    if !url.contains('{') && !url.contains('}') {
        return if is_valid_reference(url) {
            Ok(())
        } else {
            Err(invalid_value(
                &pointer.child("url"),
                "a valid URL reference",
            ))
        };
    }
    if is_valid_server_url_template(url) {
        Ok(())
    } else {
        Err(invalid_value(
            &pointer.child("url"),
            "a valid URL reference or server URL template",
        ))
    }
}

pub(super) fn is_valid_server_url_template(url: &str) -> bool {
    let mut normalized = String::with_capacity(url.len());
    let mut rest = url;
    while let Some(open_index) = rest.find('{') {
        let before_open = &rest[..open_index];
        if before_open.contains('}') {
            return false;
        }
        normalized.push_str(before_open);

        let after_open = &rest[open_index + 1..];
        let Some(close_index) = after_open.find('}') else {
            return false;
        };
        let variable = &after_open[..close_index];
        if variable.is_empty() || variable.contains('{') {
            return false;
        }
        normalized.push('1');
        rest = &after_open[close_index + 1..];
    }
    if rest.contains('}') {
        return false;
    }
    normalized.push_str(rest);
    is_valid_reference(&normalized)
}

pub(super) fn validate_server_variable_object(
    variable: &Value,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let variable = variable
        .as_object()
        .ok_or_else(|| invalid_value(pointer, "an object"))?;
    validate_object_fields(
        variable,
        pointer,
        &["enum", "default", "description"],
        "a supported OpenAPI server-variable field or specification extension beginning with 'x-'",
    )?;
    require_string_field(variable, "default", pointer)?;
    validate_optional_string_field(variable, "description", pointer)?;
    let Some(values) = variable.get("enum") else {
        return Ok(());
    };
    let enum_pointer = pointer.child("enum");
    let values = values
        .as_array()
        .ok_or_else(|| invalid_value(&enum_pointer, "a non-empty array of strings"))?;
    if values.is_empty() {
        return Err(invalid_value(&enum_pointer, "a non-empty array of strings"));
    }
    if let Some(index) = values.iter().position(|value| !value.is_string()) {
        return Err(invalid_value(
            &enum_pointer.child(index.to_string()),
            "a string",
        ));
    }
    let default = require_string_field_value(variable, "default", pointer)?;
    if !values.iter().any(|value| value.as_str() == Some(default)) {
        return Err(invalid_value(
            &pointer.child("default"),
            "a value present in `enum`",
        ));
    }
    Ok(())
}

pub(super) fn validate_optional_tags_field(
    object: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let Some(tags) = object.get("tags") else {
        return Ok(());
    };
    let tags = tags
        .as_array()
        .ok_or_else(|| invalid_value(&pointer.child("tags"), "an array"))?;
    let mut names = BTreeSet::new();
    for (index, tag) in tags.iter().enumerate() {
        let tag_pointer = pointer.child("tags").child(index.to_string());
        let name = validate_tag_object(tag, &tag_pointer)?;
        if !names.insert(name.to_owned()) {
            return Err(invalid_value(
                &tag_pointer.child("name"),
                "a tag name that is unique within the OpenAPI document",
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_tag_object<'a>(
    tag: &'a Value,
    pointer: &JsonPointer,
) -> Result<&'a str, OpenApiError> {
    let tag = tag
        .as_object()
        .ok_or_else(|| invalid_value(pointer, "an object"))?;
    validate_object_fields(
        tag,
        pointer,
        &["name", "description", "externalDocs"],
        "a supported OpenAPI tag field or specification extension beginning with 'x-'",
    )?;
    let name = require_string_field_value(tag, "name", pointer)?;
    validate_optional_string_field(tag, "description", pointer)?;
    validate_optional_external_docs_field(tag, pointer)?;
    Ok(name)
}

pub(super) fn validate_optional_security_field(
    object: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let Some(security) = object.get("security") else {
        return Ok(());
    };
    let security = security
        .as_array()
        .ok_or_else(|| invalid_value(&pointer.child("security"), "an array"))?;
    for (index, requirement) in security.iter().enumerate() {
        let requirement_pointer = pointer.child("security").child(index.to_string());
        let requirement = requirement
            .as_object()
            .ok_or_else(|| invalid_value(&requirement_pointer, "an object"))?;
        for (scheme, scopes) in requirement {
            let scopes_pointer = requirement_pointer.child(scheme);
            let scopes = scopes
                .as_array()
                .ok_or_else(|| invalid_value(&scopes_pointer, "an array of strings"))?;
            if let Some(index) = scopes.iter().position(|scope| !scope.is_string()) {
                return Err(invalid_value(
                    &scopes_pointer.child(index.to_string()),
                    "a string",
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn validate_declared_security_requirement_names(
    document: &Map<String, Value>,
) -> Result<(), OpenApiError> {
    let declared = declared_security_scheme_names(document);
    validate_security_requirement_names(document, &JsonPointer::root(), &declared)?;
    validate_security_requirement_names_in_path_item_container(
        document.get("paths").and_then(Value::as_object),
        &JsonPointer::root().child("paths"),
        true,
        &declared,
    )?;
    validate_security_requirement_names_in_path_item_container(
        document.get("webhooks").and_then(Value::as_object),
        &JsonPointer::root().child("webhooks"),
        false,
        &declared,
    )?;
    validate_security_requirement_names_in_component_callbacks(document, &declared)?;
    validate_security_requirement_names_in_component_path_items(document, &declared)?;

    Ok(())
}

pub(super) fn validate_security_requirement_names_in_component_callbacks(
    document: &Map<String, Value>,
    declared: &BTreeSet<String>,
) -> Result<(), OpenApiError> {
    let Some(callbacks) = document
        .get("components")
        .and_then(Value::as_object)
        .and_then(|components| components.get("callbacks"))
        .and_then(Value::as_object)
    else {
        return Ok(());
    };
    let callbacks_pointer = JsonPointer::root().child("components").child("callbacks");
    for (name, callback) in callbacks {
        validate_security_requirement_names_in_callback(
            callback,
            &callbacks_pointer.child(name),
            declared,
        )?;
    }
    Ok(())
}

pub(super) fn validate_security_requirement_names_in_component_path_items(
    document: &Map<String, Value>,
    declared: &BTreeSet<String>,
) -> Result<(), OpenApiError> {
    let Some(path_items) = document
        .get("components")
        .and_then(Value::as_object)
        .and_then(|components| components.get("pathItems"))
        .and_then(Value::as_object)
    else {
        return Ok(());
    };

    validate_security_requirement_names_in_path_item_container(
        Some(path_items),
        &JsonPointer::root().child("components").child("pathItems"),
        false,
        declared,
    )
}

pub(super) fn validate_security_requirement_names_in_path_item_container(
    entries: Option<&Map<String, Value>>,
    pointer: &JsonPointer,
    allow_extension_entries: bool,
    declared: &BTreeSet<String>,
) -> Result<(), OpenApiError> {
    let Some(entries) = entries else {
        return Ok(());
    };
    for (entry_name, path_item) in entries {
        if allow_extension_entries && entry_name.starts_with("x-") {
            continue;
        }
        let Some(path_item) = path_item.as_object() else {
            continue;
        };
        let path_pointer = pointer.child(entry_name);
        for method in HTTP_METHODS {
            let Some(operation) = path_item.get(method).and_then(Value::as_object) else {
                continue;
            };
            validate_security_requirement_names_in_operation(
                operation,
                &path_pointer.child(method),
                declared,
            )?;
        }
    }
    Ok(())
}

pub(super) fn validate_security_requirement_names_in_operation(
    operation: &Map<String, Value>,
    pointer: &JsonPointer,
    declared: &BTreeSet<String>,
) -> Result<(), OpenApiError> {
    validate_security_requirement_names(operation, pointer, declared)?;
    let Some(callbacks) = operation.get("callbacks").and_then(Value::as_object) else {
        return Ok(());
    };
    let callbacks_pointer = pointer.child("callbacks");
    for (name, callback) in callbacks {
        validate_security_requirement_names_in_callback(
            callback,
            &callbacks_pointer.child(name),
            declared,
        )?;
    }
    Ok(())
}

pub(super) fn validate_security_requirement_names_in_callback(
    callback: &Value,
    pointer: &JsonPointer,
    declared: &BTreeSet<String>,
) -> Result<(), OpenApiError> {
    let Some(callback) = callback.as_object() else {
        return Ok(());
    };
    if callback.contains_key("$ref") {
        return Ok(());
    }
    validate_security_requirement_names_in_path_item_container(
        Some(callback),
        pointer,
        true,
        declared,
    )
}

pub(super) fn validate_security_requirement_names(
    object: &Map<String, Value>,
    pointer: &JsonPointer,
    declared: &BTreeSet<String>,
) -> Result<(), OpenApiError> {
    let Some(security) = object.get("security").and_then(Value::as_array) else {
        return Ok(());
    };
    for (index, requirement) in security.iter().enumerate() {
        let requirement = requirement.as_object().ok_or_else(|| {
            invalid_value(
                &pointer.child("security").child(index.to_string()),
                "an object",
            )
        })?;
        for scheme in requirement.keys() {
            if !declared.contains(scheme) {
                return Err(invalid_value(
                    &pointer
                        .child("security")
                        .child(index.to_string())
                        .child(scheme),
                    "the name of a declared components.securitySchemes entry",
                ));
            }
        }
    }
    Ok(())
}

#[derive(Debug, Default)]
pub(super) struct OperationIndex {
    ids: BTreeSet<String>,
    local_references: BTreeSet<String>,
}

pub(super) fn collect_operation_index(
    document: &Map<String, Value>,
) -> Result<OperationIndex, OpenApiError> {
    let mut operations = OperationIndex::default();
    collect_operations_in_path_item_container(
        document.get("paths").and_then(Value::as_object),
        &JsonPointer::root().child("paths"),
        true,
        &mut operations,
    )?;
    collect_operations_in_path_item_container(
        document.get("webhooks").and_then(Value::as_object),
        &JsonPointer::root().child("webhooks"),
        false,
        &mut operations,
    )?;
    collect_operations_in_component_callbacks(document, &mut operations)?;
    collect_operations_in_component_path_items(document, &mut operations)?;
    Ok(operations)
}

pub(super) fn validate_resolvable_link_targets(
    document: &Map<String, Value>,
    operations: &OperationIndex,
) -> Result<(), OpenApiError> {
    validate_link_targets_in_path_item_container(
        document.get("paths").and_then(Value::as_object),
        &JsonPointer::root().child("paths"),
        true,
        operations,
    )?;
    validate_link_targets_in_path_item_container(
        document.get("webhooks").and_then(Value::as_object),
        &JsonPointer::root().child("webhooks"),
        false,
        operations,
    )?;
    validate_link_targets_in_component_callbacks(document, operations)?;
    validate_link_targets_in_component_path_items(document, operations)?;
    validate_link_targets_in_component_responses(document, operations)?;
    validate_link_targets_in_component_links(document, operations)?;
    Ok(())
}

pub(super) fn validate_link_targets_in_component_callbacks(
    document: &Map<String, Value>,
    operations: &OperationIndex,
) -> Result<(), OpenApiError> {
    let Some(callbacks) = document
        .get("components")
        .and_then(Value::as_object)
        .and_then(|components| components.get("callbacks"))
        .and_then(Value::as_object)
    else {
        return Ok(());
    };
    let callbacks_pointer = JsonPointer::root().child("components").child("callbacks");
    for (name, callback) in callbacks {
        validate_link_targets_in_callback(callback, &callbacks_pointer.child(name), operations)?;
    }
    Ok(())
}

pub(super) fn validate_link_targets_in_component_path_items(
    document: &Map<String, Value>,
    operations: &OperationIndex,
) -> Result<(), OpenApiError> {
    let Some(path_items) = document
        .get("components")
        .and_then(Value::as_object)
        .and_then(|components| components.get("pathItems"))
        .and_then(Value::as_object)
    else {
        return Ok(());
    };

    validate_link_targets_in_path_item_container(
        Some(path_items),
        &JsonPointer::root().child("components").child("pathItems"),
        false,
        operations,
    )
}

pub(super) fn validate_link_targets_in_component_responses(
    document: &Map<String, Value>,
    operations: &OperationIndex,
) -> Result<(), OpenApiError> {
    let Some(responses) = document
        .get("components")
        .and_then(Value::as_object)
        .and_then(|components| components.get("responses"))
        .and_then(Value::as_object)
    else {
        return Ok(());
    };
    let responses_pointer = JsonPointer::root().child("components").child("responses");
    for (name, response) in responses {
        validate_link_targets_in_response(response, &responses_pointer.child(name), operations)?;
    }
    Ok(())
}

pub(super) fn validate_link_targets_in_component_links(
    document: &Map<String, Value>,
    operations: &OperationIndex,
) -> Result<(), OpenApiError> {
    let Some(links) = document
        .get("components")
        .and_then(Value::as_object)
        .and_then(|components| components.get("links"))
        .and_then(Value::as_object)
    else {
        return Ok(());
    };
    let links_pointer = JsonPointer::root().child("components").child("links");
    for (name, link) in links {
        validate_link_target(link, &links_pointer.child(name), operations)?;
    }
    Ok(())
}

pub(super) fn validate_link_targets_in_path_item_container(
    entries: Option<&Map<String, Value>>,
    pointer: &JsonPointer,
    allow_extension_entries: bool,
    operations: &OperationIndex,
) -> Result<(), OpenApiError> {
    let Some(entries) = entries else {
        return Ok(());
    };
    for (entry_name, path_item) in entries {
        if allow_extension_entries && entry_name.starts_with("x-") {
            continue;
        }
        let Some(path_item) = path_item.as_object() else {
            continue;
        };
        let path_pointer = pointer.child(entry_name);
        for method in HTTP_METHODS {
            let Some(operation) = path_item.get(method).and_then(Value::as_object) else {
                continue;
            };
            validate_link_targets_in_operation(operation, &path_pointer.child(method), operations)?;
        }
    }
    Ok(())
}

pub(super) fn validate_link_targets_in_operation(
    operation: &Map<String, Value>,
    pointer: &JsonPointer,
    operations: &OperationIndex,
) -> Result<(), OpenApiError> {
    if let Some(responses) = operation.get("responses").and_then(Value::as_object) {
        let responses_pointer = pointer.child("responses");
        for (status, response) in responses {
            if status.starts_with("x-") {
                continue;
            }
            validate_link_targets_in_response(
                response,
                &responses_pointer.child(status),
                operations,
            )?;
        }
    }

    let Some(callbacks) = operation.get("callbacks").and_then(Value::as_object) else {
        return Ok(());
    };
    let callbacks_pointer = pointer.child("callbacks");
    for (name, callback) in callbacks {
        validate_link_targets_in_callback(callback, &callbacks_pointer.child(name), operations)?;
    }
    Ok(())
}

pub(super) fn validate_link_targets_in_callback(
    callback: &Value,
    pointer: &JsonPointer,
    operations: &OperationIndex,
) -> Result<(), OpenApiError> {
    let Some(callback) = callback.as_object() else {
        return Ok(());
    };
    if callback.contains_key("$ref") {
        return Ok(());
    }
    validate_link_targets_in_path_item_container(Some(callback), pointer, true, operations)
}

pub(super) fn validate_link_targets_in_response(
    response: &Value,
    pointer: &JsonPointer,
    operations: &OperationIndex,
) -> Result<(), OpenApiError> {
    let Some(response) = response.as_object() else {
        return Ok(());
    };
    if response.contains_key("$ref") {
        return Ok(());
    }
    let Some(links) = response.get("links").and_then(Value::as_object) else {
        return Ok(());
    };
    let links_pointer = pointer.child("links");
    for (name, link) in links {
        validate_link_target(link, &links_pointer.child(name), operations)?;
    }
    Ok(())
}

pub(super) fn validate_link_target(
    link: &Value,
    pointer: &JsonPointer,
    operations: &OperationIndex,
) -> Result<(), OpenApiError> {
    let Some(link) = link.as_object() else {
        return Ok(());
    };
    if link.contains_key("$ref") {
        return Ok(());
    }
    if let Some(operation_id) = link.get("operationId").and_then(Value::as_str) {
        if operations.ids.contains(operation_id) {
            return Ok(());
        }
        return Err(invalid_value(
            &pointer.child("operationId"),
            "an existing OpenAPI operationId",
        ));
    }

    let Some(reference) = link.get("operationRef").and_then(Value::as_str) else {
        return Ok(());
    };
    match classify_operation_reference(reference) {
        OperationReference::External => Ok(()),
        OperationReference::InvalidLocal => Err(invalid_value(
            &pointer.child("operationRef"),
            "a local reference to an existing OpenAPI operation",
        )),
        OperationReference::Local(reference) => {
            if operations.local_references.contains(&reference) {
                Ok(())
            } else {
                Err(invalid_value(
                    &pointer.child("operationRef"),
                    "a local reference to an existing OpenAPI operation",
                ))
            }
        }
    }
}

pub(super) fn collect_operations_in_component_callbacks(
    document: &Map<String, Value>,
    operations: &mut OperationIndex,
) -> Result<(), OpenApiError> {
    let Some(callbacks) = document
        .get("components")
        .and_then(Value::as_object)
        .and_then(|components| components.get("callbacks"))
        .and_then(Value::as_object)
    else {
        return Ok(());
    };
    let callbacks_pointer = JsonPointer::root().child("components").child("callbacks");
    for (name, callback) in callbacks {
        collect_operations_in_callback(callback, &callbacks_pointer.child(name), operations)?;
    }
    Ok(())
}

pub(super) fn collect_operations_in_component_path_items(
    document: &Map<String, Value>,
    operations: &mut OperationIndex,
) -> Result<(), OpenApiError> {
    let Some(path_items) = document
        .get("components")
        .and_then(Value::as_object)
        .and_then(|components| components.get("pathItems"))
        .and_then(Value::as_object)
    else {
        return Ok(());
    };

    collect_operations_in_path_item_container(
        Some(path_items),
        &JsonPointer::root().child("components").child("pathItems"),
        false,
        operations,
    )
}

pub(super) fn collect_operations_in_path_item_container(
    entries: Option<&Map<String, Value>>,
    pointer: &JsonPointer,
    allow_extension_entries: bool,
    operations: &mut OperationIndex,
) -> Result<(), OpenApiError> {
    let Some(entries) = entries else {
        return Ok(());
    };
    for (entry_name, path_item) in entries {
        if allow_extension_entries && entry_name.starts_with("x-") {
            continue;
        }
        let Some(path_item) = path_item.as_object() else {
            continue;
        };
        let path_pointer = pointer.child(entry_name);
        for method in HTTP_METHODS {
            let Some(operation) = path_item.get(method).and_then(Value::as_object) else {
                continue;
            };
            collect_operations_in_operation(operation, &path_pointer.child(method), operations)?;
        }
    }
    Ok(())
}

pub(super) fn collect_operations_in_operation(
    operation: &Map<String, Value>,
    pointer: &JsonPointer,
    operations: &mut OperationIndex,
) -> Result<(), OpenApiError> {
    operations.local_references.insert(pointer.render());
    if let Some(operation_id) = operation.get("operationId").and_then(Value::as_str)
        && !operations.ids.insert(operation_id.to_owned())
    {
        return Err(OpenApiError::DuplicateOperationId {
            pointer: pointer.child("operationId").render(),
            operation_id: operation_id.to_owned(),
        });
    }

    let Some(callbacks) = operation.get("callbacks").and_then(Value::as_object) else {
        return Ok(());
    };
    let callbacks_pointer = pointer.child("callbacks");
    for (name, callback) in callbacks {
        collect_operations_in_callback(callback, &callbacks_pointer.child(name), operations)?;
    }
    Ok(())
}

pub(super) fn collect_operations_in_callback(
    callback: &Value,
    pointer: &JsonPointer,
    operations: &mut OperationIndex,
) -> Result<(), OpenApiError> {
    let Some(callback) = callback.as_object() else {
        return Ok(());
    };
    if callback.contains_key("$ref") {
        return Ok(());
    }
    collect_operations_in_path_item_container(Some(callback), pointer, true, operations)
}

pub(super) fn validate_security_scheme_object(
    scheme: &Value,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let scheme = scheme
        .as_object()
        .ok_or_else(|| invalid_value(pointer, "an object"))?;
    validate_object_fields(
        scheme,
        pointer,
        &[
            "type",
            "description",
            "name",
            "in",
            "scheme",
            "bearerFormat",
            "flows",
            "openIdConnectUrl",
        ],
        "a supported OpenAPI security-scheme field or specification extension beginning with 'x-'",
    )?;
    validate_optional_string_field(scheme, "description", pointer)?;
    let scheme_type = require_string_field_value(scheme, "type", pointer)?;
    match scheme_type {
        "apiKey" => {
            require_string_field(scheme, "name", pointer)?;
            require_string_enum_field(scheme, "in", pointer, &["query", "header", "cookie"])?;
            reject_present_fields(
                scheme,
                pointer,
                &["scheme", "bearerFormat", "flows", "openIdConnectUrl"],
                "absent for this security scheme type",
            )?;
        }
        "http" => {
            require_string_field(scheme, "scheme", pointer)?;
            validate_optional_string_field(scheme, "bearerFormat", pointer)?;
            reject_present_fields(
                scheme,
                pointer,
                &["name", "in", "flows", "openIdConnectUrl"],
                "absent for this security scheme type",
            )?;
        }
        "mutualTLS" => {
            reject_present_fields(
                scheme,
                pointer,
                &[
                    "name",
                    "in",
                    "scheme",
                    "bearerFormat",
                    "flows",
                    "openIdConnectUrl",
                ],
                "absent for this security scheme type",
            )?;
        }
        "oauth2" => {
            validate_oauth_flows_field(scheme, pointer)?;
            reject_present_fields(
                scheme,
                pointer,
                &["name", "in", "scheme", "bearerFormat", "openIdConnectUrl"],
                "absent for this security scheme type",
            )?;
        }
        "openIdConnect" => {
            require_url_reference_field(scheme, "openIdConnectUrl", pointer)?;
            reject_present_fields(
                scheme,
                pointer,
                &["name", "in", "scheme", "bearerFormat", "flows"],
                "absent for this security scheme type",
            )?;
        }
        _ => {
            return Err(invalid_value(
                &pointer.child("type"),
                "one of 'apiKey', 'http', 'mutualTLS', 'oauth2', or 'openIdConnect'",
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_security_scheme_document_shape(
    scheme: &Value,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let object = scheme
        .as_object()
        .ok_or_else(|| invalid_value(pointer, "an object or local reference"))?;
    if let Some(reference) = object.get("$ref") {
        if !reference.is_string() {
            return Err(invalid_value(&pointer.child("$ref"), "a string"));
        }
        return validate_reference_object_fields(object, pointer);
    }
    validate_security_scheme_object(scheme, pointer)
}

pub(super) fn validate_oauth_flows_field(
    scheme: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let flows_pointer = pointer.child("flows");
    let flows = scheme
        .get("flows")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid_value(&flows_pointer, "an object"))?;
    validate_object_fields(
        flows,
        &flows_pointer,
        &[
            "implicit",
            "password",
            "clientCredentials",
            "authorizationCode",
        ],
        "a supported OpenAPI OAuth flows field or specification extension beginning with 'x-'",
    )?;
    for (flow_name, flow) in flows {
        if flow_name.starts_with("x-") {
            continue;
        }
        validate_oauth_flow_object(flow_name, flow, &flows_pointer.child(flow_name))?;
    }
    Ok(())
}

pub(super) fn validate_oauth_flow_object(
    flow_name: &str,
    flow: &Value,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let flow = flow
        .as_object()
        .ok_or_else(|| invalid_value(pointer, "an object"))?;
    validate_object_fields(
        flow,
        pointer,
        &["authorizationUrl", "tokenUrl", "refreshUrl", "scopes"],
        "a supported OpenAPI OAuth flow field or specification extension beginning with 'x-'",
    )?;
    match flow_name {
        "implicit" => {
            require_url_reference_field(flow, "authorizationUrl", pointer)?;
            reject_present_fields(
                flow,
                pointer,
                &["tokenUrl"],
                "absent for this OAuth flow type",
            )?;
        }
        "password" | "clientCredentials" => {
            require_url_reference_field(flow, "tokenUrl", pointer)?;
            reject_present_fields(
                flow,
                pointer,
                &["authorizationUrl"],
                "absent for this OAuth flow type",
            )?;
        }
        "authorizationCode" => {
            require_url_reference_field(flow, "authorizationUrl", pointer)?;
            require_url_reference_field(flow, "tokenUrl", pointer)?;
        }
        _ => {}
    }
    validate_optional_url_reference_field(flow, "refreshUrl", pointer)?;
    validate_string_map_field(flow, "scopes", pointer)?;
    Ok(())
}

pub(super) fn validate_object_fields(
    object: &Map<String, Value>,
    pointer: &JsonPointer,
    allowed: &[&str],
    expected: &'static str,
) -> Result<(), OpenApiError> {
    for field in object.keys() {
        if field.starts_with("x-") || allowed.contains(&field.as_str()) {
            continue;
        }
        return Err(invalid_value(&pointer.child(field), expected));
    }
    Ok(())
}

pub(super) fn require_string_field(
    object: &Map<String, Value>,
    field: &str,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    if object.get(field).is_some_and(Value::is_string) {
        Ok(())
    } else {
        Err(invalid_value(&pointer.child(field), "a string"))
    }
}

pub(super) fn require_url_reference_field(
    object: &Map<String, Value>,
    field: &str,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let value = object
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_value(&pointer.child(field), "a valid URL reference"))?;

    if is_valid_reference(value) {
        Ok(())
    } else {
        Err(invalid_value(
            &pointer.child(field),
            "a valid URL reference",
        ))
    }
}

pub(super) fn require_string_field_value<'a>(
    object: &'a Map<String, Value>,
    field: &str,
    pointer: &JsonPointer,
) -> Result<&'a str, OpenApiError> {
    object
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_value(&pointer.child(field), "a string"))
}

pub(super) fn require_string_enum_field(
    object: &Map<String, Value>,
    field: &str,
    pointer: &JsonPointer,
    allowed: &[&str],
) -> Result<(), OpenApiError> {
    let value = require_string_field_value(object, field, pointer)?;
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(invalid_value(&pointer.child(field), "a supported value"))
    }
}

pub(super) fn reject_present_fields(
    object: &Map<String, Value>,
    pointer: &JsonPointer,
    fields: &[&str],
    expected: &'static str,
) -> Result<(), OpenApiError> {
    for field in fields {
        if object.contains_key(*field) {
            return Err(invalid_value(&pointer.child(*field), expected));
        }
    }
    Ok(())
}

pub(super) fn validate_string_map_field(
    object: &Map<String, Value>,
    field: &str,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let values = object
        .get(field)
        .and_then(Value::as_object)
        .ok_or_else(|| invalid_value(&pointer.child(field), "an object of strings"))?;
    if let Some((key, _)) = values.iter().find(|(_, value)| !value.is_string()) {
        return Err(invalid_value(&pointer.child(field).child(key), "a string"));
    }
    Ok(())
}

pub(super) fn validate_discriminator(
    schema: &Map<String, Value>,
    value: &Value,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    if !["oneOf", "anyOf", "allOf"]
        .iter()
        .any(|keyword| schema.contains_key(*keyword))
    {
        return Err(invalid_value(
            pointer,
            "a discriminator adjacent to `oneOf`, `anyOf`, or `allOf`",
        ));
    }
    let discriminator = value
        .as_object()
        .ok_or_else(|| invalid_value(pointer, "an object"))?;
    for field in discriminator.keys() {
        if field.starts_with("x-") || matches!(field.as_str(), "propertyName" | "mapping") {
            continue;
        }

        return Err(invalid_value(
            &pointer.child(field),
            "a supported OpenAPI discriminator field or specification extension beginning with 'x-'",
        ));
    }

    discriminator
        .get("propertyName")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_value(&pointer.child("propertyName"), "a string"))?;

    if let Some(mapping) = discriminator.get("mapping") {
        let mapping = mapping
            .as_object()
            .ok_or_else(|| invalid_value(&pointer.child("mapping"), "an object of strings"))?;
        if let Some((name, _)) = mapping.iter().find(|(_, target)| !target.is_string()) {
            return Err(invalid_value(
                &pointer.child("mapping").child(name),
                "a string",
            ));
        }
    }

    Ok(())
}

pub(super) fn validate_schema_document_shape(
    schema: &Value,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    validate_schema_document_shape_with_context(schema, pointer, SchemaLocation::NonProperty)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SchemaLocation {
    Property,
    NonProperty,
}

pub(super) fn validate_schema_document_shape_with_context(
    schema: &Value,
    pointer: &JsonPointer,
    location: SchemaLocation,
) -> Result<(), OpenApiError> {
    match schema {
        Value::Bool(_) => Ok(()),
        Value::Object(object) => {
            validate_optional_external_docs_field(object, pointer)?;
            if object.contains_key("xml") && location != SchemaLocation::Property {
                return Err(invalid_value(
                    &pointer.child("xml"),
                    "absent outside property schemas",
                ));
            }
            validate_optional_xml_field(object, pointer)?;
            if let Some(discriminator) = object.get("discriminator") {
                validate_discriminator(object, discriminator, &pointer.child("discriminator"))?;
            }
            for keyword in ["readOnly", "writeOnly"] {
                if object
                    .get(keyword)
                    .is_some_and(|annotation| !annotation.is_boolean())
                {
                    return Err(invalid_value(&pointer.child(keyword), "a boolean"));
                }
            }

            for keyword in SINGLE_SCHEMA_CHILD_KEYWORDS {
                if let Some(child) = object.get(keyword) {
                    validate_schema_document_shape_with_context(
                        child,
                        &pointer.child(keyword),
                        SchemaLocation::NonProperty,
                    )?;
                }
            }

            for keyword in ["$defs", "definitions", "dependentSchemas"] {
                if let Some(children) = object.get(keyword).and_then(Value::as_object) {
                    for (name, child) in children {
                        validate_schema_document_shape_with_context(
                            child,
                            &pointer.child(keyword).child(name),
                            SchemaLocation::NonProperty,
                        )?;
                    }
                }
            }

            for keyword in ["patternProperties", "properties"] {
                if let Some(children) = object.get(keyword).and_then(Value::as_object) {
                    for (name, child) in children {
                        validate_schema_document_shape_with_context(
                            child,
                            &pointer.child(keyword).child(name),
                            SchemaLocation::Property,
                        )?;
                    }
                }
            }

            for keyword in SCHEMA_ARRAY_CHILD_KEYWORDS {
                if let Some(children) = object.get(keyword).and_then(Value::as_array) {
                    for (index, child) in children.iter().enumerate() {
                        validate_schema_document_shape_with_context(
                            child,
                            &pointer.child(keyword).child(index.to_string()),
                            SchemaLocation::NonProperty,
                        )?;
                    }
                }
            }

            Ok(())
        }
        _ => Err(invalid_value(pointer, "an object or boolean schema")),
    }
}
