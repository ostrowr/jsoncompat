use crate::{
    CompatibilityError, Role, SchemaDocument, check_compat, explain_compat_failure,
    json_pointer::JsonPointer,
};
use json_schema_ast::SchemaBuildError;
use percent_encoding::percent_decode_str;
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};

const OPENAPI_31_PREFIX: &str = "3.1.";
const COMPONENT_SCHEMA_REF_PREFIX: &str = "#/components/schemas/";
const SUPPORTED_REF_PREFIX: &str = "#/";

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OpenApiError {
    #[error("OpenAPI document root must be an object")]
    InvalidDocumentRoot,
    #[error("OpenAPI document must declare an `openapi` version string")]
    MissingVersion,
    #[error("unsupported OpenAPI version '{actual}': expected a 3.1.x document")]
    UnsupportedVersion { actual: String },
    #[error("OpenAPI value at '{pointer}' must be {expected}")]
    InvalidValue {
        pointer: String,
        expected: &'static str,
    },
    #[error("unsupported OpenAPI reference '{reference}' at '{pointer}'")]
    UnsupportedReference { pointer: String, reference: String },
    #[error("OpenAPI reference '{reference}' at '{pointer}' did not resolve")]
    UnresolvedReference { pointer: String, reference: String },
    #[error("OpenAPI reference chain at '{pointer}' forms a cycle through '{reference}'")]
    CyclicReference { pointer: String, reference: String },
    #[error("OpenAPI operation '{method} {path}' is missing a responses object")]
    MissingResponses { method: String, path: String },
    #[error("duplicate OpenAPI parameter '{location}:{name}' in '{pointer}'")]
    DuplicateParameter {
        pointer: String,
        location: String,
        name: String,
    },
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OpenApiCompatibilityError {
    #[error(transparent)]
    OpenApi(#[from] OpenApiError),
    #[error(transparent)]
    Schema(#[from] SchemaBuildError),
    #[error(transparent)]
    Compatibility(#[from] CompatibilityError),
}

#[derive(Debug, Clone)]
pub struct OpenApiDocument {
    raw: Value,
}

impl OpenApiDocument {
    pub fn from_json(raw: &Value) -> Result<Self, OpenApiError> {
        let object = raw.as_object().ok_or(OpenApiError::InvalidDocumentRoot)?;
        let version = object
            .get("openapi")
            .and_then(Value::as_str)
            .ok_or(OpenApiError::MissingVersion)?;
        if !version.starts_with(OPENAPI_31_PREFIX) {
            return Err(OpenApiError::UnsupportedVersion {
                actual: version.to_owned(),
            });
        }
        if let Some(paths) = object.get("paths")
            && !paths.is_object()
        {
            return Err(invalid_value(
                &JsonPointer::root().child("paths"),
                "an object",
            ));
        }
        if let Some(components) = object.get("components")
            && !components.is_object()
        {
            return Err(invalid_value(
                &JsonPointer::root().child("components"),
                "an object",
            ));
        }
        Ok(Self { raw: raw.clone() })
    }

    fn as_object(&self) -> &Map<String, Value> {
        self.raw
            .as_object()
            .expect("OpenApiDocument validates its root object at construction")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OpenApiCompatibilitySurface {
    Operation,
    Request,
    Response,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenApiCompatibilityIssue {
    pub method: String,
    pub path: String,
    pub surface: OpenApiCompatibilitySurface,
    pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OpenApiCompatibilityReport {
    issues: Vec<OpenApiCompatibilityIssue>,
}

impl OpenApiCompatibilityReport {
    #[must_use]
    pub fn is_compatible(&self) -> bool {
        self.issues.is_empty()
    }

    #[must_use]
    pub fn issues(&self) -> &[OpenApiCompatibilityIssue] {
        &self.issues
    }

    fn push(
        &mut self,
        operation: &OperationKey,
        surface: OpenApiCompatibilitySurface,
        message: impl Into<String>,
    ) {
        self.issues.push(OpenApiCompatibilityIssue {
            method: operation.method.clone(),
            path: operation.path.clone(),
            surface,
            message: message.into(),
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct OperationKey {
    method: String,
    path: String,
}

#[derive(Debug)]
struct LoweredOperation {
    request: SchemaDocument,
    response: SchemaDocument,
}

#[derive(Debug, Clone)]
struct Parameter {
    name: String,
    location: ParameterLocation,
    required: bool,
    value: FieldValue,
}

#[derive(Debug, Clone)]
struct ContractField {
    name: String,
    required: bool,
    value: FieldValue,
}

#[derive(Debug, Clone)]
enum FieldValue {
    Schema {
        schema: Value,
        serialization: SchemaSerialization,
    },
    Content {
        media_schema: Value,
    },
}

#[derive(Debug, Clone)]
enum SchemaSerialization {
    PathParameter {
        style: String,
        explode: bool,
    },
    QueryParameter {
        style: String,
        explode: bool,
        allow_reserved: bool,
        allow_empty_value: bool,
    },
    Header {
        explode: bool,
    },
    CookieParameter {
        style: String,
        explode: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ParameterLocation {
    Path,
    Query,
    Header,
    Cookie,
}

impl ParameterLocation {
    fn from_value(value: &str, pointer: &JsonPointer) -> Result<Self, OpenApiError> {
        match value {
            "path" => Ok(Self::Path),
            "query" => Ok(Self::Query),
            "header" => Ok(Self::Header),
            "cookie" => Ok(Self::Cookie),
            _ => Err(OpenApiError::InvalidValue {
                pointer: pointer.render(),
                expected: "one of 'path', 'query', 'header', or 'cookie'",
            }),
        }
    }

    const fn field_name(self) -> &'static str {
        match self {
            Self::Path => "path",
            Self::Query => "query",
            Self::Header => "headers",
            Self::Cookie => "cookies",
        }
    }

    const fn default_style(self) -> &'static str {
        match self {
            Self::Path | Self::Header => "simple",
            Self::Query | Self::Cookie => "form",
        }
    }
}

impl From<&Parameter> for ContractField {
    fn from(parameter: &Parameter) -> Self {
        Self {
            name: parameter_identity_name(parameter.location, &parameter.name),
            required: parameter.required,
            value: parameter.value.clone(),
        }
    }
}

pub fn check_openapi_compat(
    old: &OpenApiDocument,
    new: &OpenApiDocument,
) -> Result<OpenApiCompatibilityReport, OpenApiCompatibilityError> {
    let old_operations = lower_operations(old)?;
    let new_operations = lower_operations(new)?;
    let mut report = OpenApiCompatibilityReport::default();

    for (key, old_operation) in &old_operations {
        let Some(new_operation) = new_operations.get(key) else {
            report.push(
                key,
                OpenApiCompatibilitySurface::Operation,
                "operation was removed",
            );
            continue;
        };

        if !check_compat(
            &old_operation.request,
            &new_operation.request,
            Role::Deserializer,
        )? {
            let detail = explain_compat_failure(
                &old_operation.request,
                &new_operation.request,
                Role::Deserializer,
            )?;
            report.push(
                key,
                OpenApiCompatibilitySurface::Request,
                detail.unwrap_or_else(|| "request contract became incompatible".to_owned()),
            );
        }
        if !check_compat(
            &old_operation.response,
            &new_operation.response,
            Role::Serializer,
        )? {
            let detail = explain_compat_failure(
                &old_operation.response,
                &new_operation.response,
                Role::Serializer,
            )?;
            report.push(
                key,
                OpenApiCompatibilitySurface::Response,
                detail.unwrap_or_else(|| "response contract became incompatible".to_owned()),
            );
        }
    }

    Ok(report)
}

fn lower_operations(
    document: &OpenApiDocument,
) -> Result<BTreeMap<OperationKey, LoweredOperation>, OpenApiCompatibilityError> {
    let resolver = Resolver::new(document)?;
    let mut operations = BTreeMap::new();
    let Some(paths) = document.as_object().get("paths") else {
        return Ok(operations);
    };
    let paths_pointer = JsonPointer::root().child("paths");
    let paths = paths
        .as_object()
        .ok_or_else(|| invalid_value(&paths_pointer, "an object"))?;

    for (path, path_item) in paths {
        let path_pointer = paths_pointer.child(path);
        let path_item = resolver.resolve_value(path_item, &path_pointer)?;
        let path_item = path_item
            .as_object()
            .ok_or_else(|| invalid_value(&path_pointer, "an object or local reference"))?;
        let path_parameters = collect_parameters(
            &resolver,
            path_item.get("parameters"),
            &path_pointer.child("parameters"),
        )?;

        for method in HTTP_METHODS {
            let Some(operation_value) = path_item.get(method) else {
                continue;
            };
            let operation_pointer = path_pointer.child(method);
            let operation_value = resolver.resolve_value(operation_value, &operation_pointer)?;
            let operation = operation_value
                .as_object()
                .ok_or_else(|| invalid_value(&operation_pointer, "an object or local reference"))?;
            let operation_parameters = collect_parameters(
                &resolver,
                operation.get("parameters"),
                &operation_pointer.child("parameters"),
            )?;
            let parameters = merge_parameters(path_parameters.clone(), operation_parameters);
            let request_schema =
                lower_request_schema(&resolver, operation, &operation_pointer, &parameters)?;
            let response_schema =
                lower_response_schema(&resolver, operation, &operation_pointer, method, path)?;
            operations.insert(
                OperationKey {
                    method: method.to_ascii_uppercase(),
                    path: path.clone(),
                },
                LoweredOperation {
                    request: SchemaDocument::from_json(&request_schema)?,
                    response: SchemaDocument::from_json(&response_schema)?,
                },
            );
        }
    }

    Ok(operations)
}

const HTTP_METHODS: [&str; 8] = [
    "get", "put", "post", "delete", "options", "head", "patch", "trace",
];

fn collect_parameters(
    resolver: &Resolver<'_>,
    raw: Option<&Value>,
    pointer: &JsonPointer,
) -> Result<BTreeMap<(ParameterLocation, String), Parameter>, OpenApiError> {
    let Some(raw) = raw else {
        return Ok(BTreeMap::new());
    };
    let raw = raw
        .as_array()
        .ok_or_else(|| invalid_value(pointer, "an array"))?;
    let mut parameters = BTreeMap::new();
    for (index, raw_parameter) in raw.iter().enumerate() {
        let parameter_pointer = pointer.child(index.to_string());
        let parameter = parse_parameter(resolver, raw_parameter, &parameter_pointer)?;
        let identity = (
            parameter.location,
            parameter_identity_name(parameter.location, &parameter.name),
        );
        if parameters.insert(identity.clone(), parameter).is_some() {
            return Err(OpenApiError::DuplicateParameter {
                pointer: parameter_pointer.render(),
                location: identity.0.field_name().to_owned(),
                name: identity.1,
            });
        }
    }
    Ok(parameters)
}

fn merge_parameters(
    mut path_parameters: BTreeMap<(ParameterLocation, String), Parameter>,
    operation_parameters: BTreeMap<(ParameterLocation, String), Parameter>,
) -> BTreeMap<(ParameterLocation, String), Parameter> {
    for (identity, parameter) in operation_parameters {
        path_parameters.insert(identity, parameter);
    }
    path_parameters
}

fn parse_parameter(
    resolver: &Resolver<'_>,
    raw: &Value,
    pointer: &JsonPointer,
) -> Result<Parameter, OpenApiError> {
    let raw = resolver.resolve_value(raw, pointer)?;
    let object = raw
        .as_object()
        .ok_or_else(|| invalid_value(pointer, "an object or local reference"))?;
    let name = object
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_value(&pointer.child("name"), "a string"))?
        .to_owned();
    let location = ParameterLocation::from_value(
        object
            .get("in")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid_value(&pointer.child("in"), "a string"))?,
        &pointer.child("in"),
    )?;
    let required = object
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

    reject_query_only_metadata_outside_query(location, object, pointer)?;
    let value = lower_field_value(
        resolver,
        object.get("schema"),
        object.get("content"),
        pointer,
        |schema| parameter_schema_value(location, object, pointer, schema),
    )?;

    Ok(Parameter {
        name,
        location,
        required,
        value,
    })
}

fn reject_query_only_metadata_outside_query(
    location: ParameterLocation,
    object: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    if location == ParameterLocation::Query {
        return Ok(());
    }
    for keyword in ["allowReserved", "allowEmptyValue"] {
        if object.contains_key(keyword) {
            return Err(invalid_value(
                &pointer.child(keyword),
                "a query parameter field",
            ));
        }
    }
    Ok(())
}

fn parameter_schema_value(
    location: ParameterLocation,
    object: &Map<String, Value>,
    pointer: &JsonPointer,
    schema: Value,
) -> Result<FieldValue, OpenApiError> {
    let style = parse_optional_string(object, "style", pointer)?
        .unwrap_or_else(|| location.default_style().to_owned());
    let explode = parse_optional_bool(object, "explode", pointer)?.unwrap_or(style == "form");
    let serialization = match location {
        ParameterLocation::Path => SchemaSerialization::PathParameter { style, explode },
        ParameterLocation::Query => SchemaSerialization::QueryParameter {
            style,
            explode,
            allow_reserved: parse_optional_bool(object, "allowReserved", pointer)?.unwrap_or(false),
            allow_empty_value: parse_optional_bool(object, "allowEmptyValue", pointer)?
                .unwrap_or(false),
        },
        ParameterLocation::Header => {
            if style != "simple" {
                return Err(invalid_value(
                    &pointer.child("style"),
                    "'simple' for header parameters",
                ));
            }
            SchemaSerialization::Header { explode }
        }
        ParameterLocation::Cookie => SchemaSerialization::CookieParameter { style, explode },
    };
    Ok(FieldValue::Schema {
        schema,
        serialization,
    })
}

fn parse_optional_string(
    object: &Map<String, Value>,
    key: &str,
    pointer: &JsonPointer,
) -> Result<Option<String>, OpenApiError> {
    object
        .get(key)
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| invalid_value(&pointer.child(key), "a string"))
        })
        .transpose()
}

fn parse_optional_bool(
    object: &Map<String, Value>,
    key: &str,
    pointer: &JsonPointer,
) -> Result<Option<bool>, OpenApiError> {
    object
        .get(key)
        .map(|value| {
            value
                .as_bool()
                .ok_or_else(|| invalid_value(&pointer.child(key), "a boolean"))
        })
        .transpose()
}

fn lower_request_schema(
    resolver: &Resolver<'_>,
    operation: &Map<String, Value>,
    pointer: &JsonPointer,
    parameters: &BTreeMap<(ParameterLocation, String), Parameter>,
) -> Result<Value, OpenApiError> {
    let mut properties = Map::new();
    for location in [
        ParameterLocation::Path,
        ParameterLocation::Query,
        ParameterLocation::Header,
        ParameterLocation::Cookie,
    ] {
        properties.insert(
            location.field_name().to_owned(),
            lower_parameter_group(
                parameters
                    .values()
                    .filter(|parameter| parameter.location == location),
            ),
        );
    }
    properties.insert(
        "body".to_owned(),
        lower_request_body(
            resolver,
            operation.get("requestBody"),
            &pointer.child("requestBody"),
        )?,
    );

    attach_schema_defs(
        resolver,
        json!({
            "type": "object",
            "properties": properties,
            "required": ["path", "query", "headers", "cookies", "body"],
            "additionalProperties": false
        }),
    )
}

fn lower_parameter_group<'a>(parameters: impl Iterator<Item = &'a Parameter>) -> Value {
    let mut properties = Map::new();
    let mut required = Vec::new();
    for parameter in parameters {
        let field = ContractField::from(parameter);
        if field.required {
            let key = field.name.clone();
            required.push(Value::String(key.clone()));
            properties.insert(key, contract_field_schema(&field));
        } else {
            properties.insert(field.name.clone(), contract_field_schema(&field));
        }
    }
    required.sort_by(|left, right| left.as_str().cmp(&right.as_str()));
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false
    })
}

fn contract_field_schema(field: &ContractField) -> Value {
    let mut properties = Map::new();
    let mut required = Vec::new();
    match &field.value {
        FieldValue::Schema {
            schema,
            serialization,
        } => {
            properties.insert("value".to_owned(), schema.clone());
            required.push(Value::String("value".to_owned()));
            add_schema_serialization_properties(serialization, &mut properties, &mut required);
        }
        FieldValue::Content { media_schema } => {
            properties.insert("value".to_owned(), media_schema.clone());
            required.push(Value::String("value".to_owned()));
        }
    }
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false
    })
}

fn add_schema_serialization_properties(
    serialization: &SchemaSerialization,
    properties: &mut Map<String, Value>,
    required: &mut Vec<Value>,
) {
    match serialization {
        SchemaSerialization::PathParameter { style, explode }
        | SchemaSerialization::CookieParameter { style, explode } => {
            add_enum_property(properties, required, "style", Value::String(style.clone()));
            add_enum_property(properties, required, "explode", Value::Bool(*explode));
        }
        SchemaSerialization::QueryParameter {
            style,
            explode,
            allow_reserved,
            allow_empty_value,
        } => {
            add_enum_property(properties, required, "style", Value::String(style.clone()));
            add_enum_property(properties, required, "explode", Value::Bool(*explode));
            add_enum_property(
                properties,
                required,
                "allow_reserved",
                Value::Bool(*allow_reserved),
            );
            add_enum_property(
                properties,
                required,
                "allow_empty_value",
                Value::Bool(*allow_empty_value),
            );
        }
        SchemaSerialization::Header { explode } => {
            add_enum_property(properties, required, "explode", Value::Bool(*explode));
        }
    }
}

fn add_enum_property(
    properties: &mut Map<String, Value>,
    required: &mut Vec<Value>,
    name: &str,
    value: Value,
) {
    properties.insert(name.to_owned(), json!({ "enum": [value] }));
    required.push(Value::String(name.to_owned()));
}

fn lower_request_body(
    resolver: &Resolver<'_>,
    raw: Option<&Value>,
    pointer: &JsonPointer,
) -> Result<Value, OpenApiError> {
    let Some(raw) = raw else {
        return Ok(json!({ "type": "null" }));
    };
    let raw = resolver.resolve_value(raw, pointer)?;
    let object = raw
        .as_object()
        .ok_or_else(|| invalid_value(pointer, "an object or local reference"))?;
    let required = object
        .get("required")
        .map(|value| {
            value
                .as_bool()
                .ok_or_else(|| invalid_value(&pointer.child("required"), "a boolean"))
        })
        .transpose()?
        .unwrap_or(false);
    let content = object.get("content").ok_or_else(|| {
        invalid_value(
            &pointer.child("content"),
            "an object containing at least one media type",
        )
    })?;
    let variants = lower_content_variants(resolver, content, &pointer.child("content"))?;
    if required {
        Ok(any_of(variants))
    } else {
        Ok(any_of(
            std::iter::once(json!({ "type": "null" }))
                .chain(variants)
                .collect(),
        ))
    }
}

fn lower_response_schema(
    resolver: &Resolver<'_>,
    operation: &Map<String, Value>,
    pointer: &JsonPointer,
    method: &str,
    path: &str,
) -> Result<Value, OpenApiError> {
    let responses_pointer = pointer.child("responses");
    let responses = operation
        .get("responses")
        .and_then(Value::as_object)
        .ok_or_else(|| OpenApiError::MissingResponses {
            method: method.to_ascii_uppercase(),
            path: path.to_owned(),
        })?;
    if responses.is_empty() {
        return Err(invalid_value(
            &responses_pointer,
            "an object containing at least one response",
        ));
    }
    let mut variants = Vec::new();
    for (status, raw_response) in responses {
        let response_pointer = responses_pointer.child(status);
        let raw_response = resolver.resolve_value(raw_response, &response_pointer)?;
        let response = raw_response
            .as_object()
            .ok_or_else(|| invalid_value(&response_pointer, "an object or local reference"))?;
        let body = lower_response_body(
            resolver,
            response.get("content"),
            &response_pointer.child("content"),
        )?;
        let headers = lower_response_headers(
            resolver,
            response.get("headers"),
            &response_pointer.child("headers"),
        )?;
        variants.push(json!({
            "type": "object",
            "properties": {
                "status": { "enum": [status] },
                "body": body,
                "headers": headers
            },
            "required": ["status", "body", "headers"],
            "additionalProperties": false
        }));
    }

    attach_schema_defs(resolver, any_of(variants))
}

fn lower_response_body(
    resolver: &Resolver<'_>,
    content: Option<&Value>,
    pointer: &JsonPointer,
) -> Result<Value, OpenApiError> {
    let Some(content) = content else {
        return Ok(json!({ "type": "null" }));
    };
    Ok(any_of(lower_content_variants(resolver, content, pointer)?))
}

fn lower_content_variants(
    resolver: &Resolver<'_>,
    content: &Value,
    pointer: &JsonPointer,
) -> Result<Vec<Value>, OpenApiError> {
    let content = content
        .as_object()
        .ok_or_else(|| invalid_value(pointer, "an object"))?;
    if content.is_empty() {
        return Err(invalid_value(
            pointer,
            "an object containing at least one media type",
        ));
    }
    let mut variants = Vec::with_capacity(content.len());
    for (media_type, raw_media) in content {
        let media_pointer = pointer.child(media_type);
        let media = resolver.resolve_value(raw_media, &media_pointer)?;
        let media = media
            .as_object()
            .ok_or_else(|| invalid_value(&media_pointer, "an object or local reference"))?;
        let schema = media
            .get("schema")
            .map(|schema| rewrite_schema_refs(schema, &media_pointer.child("schema")))
            .transpose()?
            .unwrap_or(Value::Bool(true));
        variants.push(json!({
            "type": "object",
            "properties": {
                "content_type": { "enum": [media_type] },
                "value": schema
            },
            "required": ["content_type", "value"],
            "additionalProperties": false
        }));
    }
    Ok(variants)
}

fn lower_response_headers(
    resolver: &Resolver<'_>,
    raw: Option<&Value>,
    pointer: &JsonPointer,
) -> Result<Value, OpenApiError> {
    let Some(raw) = raw else {
        return Ok(json!({
            "type": "object",
            "properties": {},
            "required": [],
            "additionalProperties": false
        }));
    };
    let headers = raw
        .as_object()
        .ok_or_else(|| invalid_value(pointer, "an object"))?;
    let mut properties = Map::new();
    let mut required = Vec::new();
    for (name, raw_header) in headers {
        let header_pointer = pointer.child(name);
        let field = lower_response_header_field(resolver, name, raw_header, &header_pointer)?;
        if field.required {
            required.push(Value::String(field.name.clone()));
        }
        properties.insert(field.name.clone(), contract_field_schema(&field));
    }
    required.sort_by(|left, right| left.as_str().cmp(&right.as_str()));

    Ok(json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false
    }))
}

fn lower_response_header_field(
    resolver: &Resolver<'_>,
    name: &str,
    raw_header: &Value,
    pointer: &JsonPointer,
) -> Result<ContractField, OpenApiError> {
    let raw_header = resolver.resolve_value(raw_header, pointer)?;
    let header = raw_header
        .as_object()
        .ok_or_else(|| invalid_value(pointer, "an object or local reference"))?;
    if header.contains_key("allowReserved") {
        return Err(invalid_value(
            &pointer.child("allowReserved"),
            "not present for response headers",
        ));
    }
    if header.contains_key("allowEmptyValue") {
        return Err(invalid_value(
            &pointer.child("allowEmptyValue"),
            "not present for response headers",
        ));
    }
    let required = parse_optional_bool(header, "required", pointer)?.unwrap_or(false);
    let value = lower_field_value(
        resolver,
        header.get("schema"),
        header.get("content"),
        pointer,
        |schema| header_schema_value(header, pointer, schema),
    )?;
    Ok(ContractField {
        name: name.to_ascii_lowercase(),
        required,
        value,
    })
}

fn header_schema_value(
    header: &Map<String, Value>,
    pointer: &JsonPointer,
    schema: Value,
) -> Result<FieldValue, OpenApiError> {
    let style =
        parse_optional_string(header, "style", pointer)?.unwrap_or_else(|| "simple".to_owned());
    if style != "simple" {
        return Err(invalid_value(
            &pointer.child("style"),
            "'simple' for response headers",
        ));
    }
    Ok(FieldValue::Schema {
        schema,
        serialization: SchemaSerialization::Header {
            explode: parse_optional_bool(header, "explode", pointer)?.unwrap_or(false),
        },
    })
}

fn lower_field_value(
    resolver: &Resolver<'_>,
    schema: Option<&Value>,
    content: Option<&Value>,
    pointer: &JsonPointer,
    schema_value: impl FnOnce(Value) -> Result<FieldValue, OpenApiError>,
) -> Result<FieldValue, OpenApiError> {
    match (schema, content) {
        (Some(schema), None) => {
            schema_value(rewrite_schema_refs(schema, &pointer.child("schema"))?)
        }
        (None, Some(content)) => Ok(FieldValue::Content {
            media_schema: any_of(lower_content_variants(
                resolver,
                content,
                &pointer.child("content"),
            )?),
        }),
        (Some(_), Some(_)) => Err(invalid_value(
            pointer,
            "exactly one of `schema` or `content`",
        )),
        (None, None) => Err(invalid_value(
            pointer,
            "exactly one of `schema` or `content`",
        )),
    }
}

fn any_of(mut variants: Vec<Value>) -> Value {
    match variants.len() {
        0 => Value::Bool(false),
        1 => variants.pop().expect("single variant should exist"),
        _ => json!({ "anyOf": variants }),
    }
}

fn attach_schema_defs(resolver: &Resolver<'_>, mut schema: Value) -> Result<Value, OpenApiError> {
    let defs = resolver.component_schema_defs_for(&schema)?;
    if defs.is_empty() {
        return Ok(schema);
    }
    let object = schema.as_object_mut().ok_or_else(|| {
        invalid_value(
            &JsonPointer::root(),
            "an object schema when component schemas exist",
        )
    })?;
    object.insert("$defs".to_owned(), Value::Object(defs));
    Ok(schema)
}

fn parameter_identity_name(location: ParameterLocation, name: &str) -> String {
    if location == ParameterLocation::Header {
        name.to_ascii_lowercase()
    } else {
        name.to_owned()
    }
}

struct Resolver<'a> {
    document: &'a OpenApiDocument,
    component_schema_defs: BTreeMap<String, ComponentSchemaDef>,
}

struct ComponentSchemaDef {
    schema: Value,
    dependencies: Vec<String>,
}

impl<'a> Resolver<'a> {
    fn new(document: &'a OpenApiDocument) -> Result<Self, OpenApiError> {
        Ok(Self {
            document,
            component_schema_defs: load_component_schema_defs(document)?,
        })
    }

    fn resolve_value<'b>(
        &'b self,
        value: &'b Value,
        pointer: &JsonPointer,
    ) -> Result<&'b Value, OpenApiError> {
        let mut current = value;
        let mut visited_references = BTreeSet::new();
        loop {
            let Some(reference) = current
                .as_object()
                .and_then(|object| object.get("$ref"))
                .and_then(Value::as_str)
            else {
                return Ok(current);
            };
            if !visited_references.insert(reference.to_owned()) {
                return Err(OpenApiError::CyclicReference {
                    pointer: pointer.render(),
                    reference: reference.to_owned(),
                });
            }
            current = self.resolve_reference(reference, pointer)?;
        }
    }

    fn resolve_reference<'b>(
        &'b self,
        reference: &str,
        pointer: &JsonPointer,
    ) -> Result<&'b Value, OpenApiError> {
        if reference != "#" && !reference.starts_with(SUPPORTED_REF_PREFIX) {
            return Err(OpenApiError::UnsupportedReference {
                pointer: pointer.render(),
                reference: reference.to_owned(),
            });
        }
        lookup_pointer(&self.document.raw, reference).ok_or_else(|| {
            OpenApiError::UnresolvedReference {
                pointer: pointer.render(),
                reference: reference.to_owned(),
            }
        })
    }

    fn component_schema_defs_for(
        &self,
        schema: &Value,
    ) -> Result<Map<String, Value>, OpenApiError> {
        let mut pending = collect_component_schema_names(schema);
        let mut visited = BTreeSet::new();
        let mut defs = Map::new();
        while let Some(name) = pending.pop() {
            if !visited.insert(name.clone()) {
                continue;
            }
            let Some(component) = self.component_schema_defs.get(&name) else {
                continue;
            };
            pending.extend(component.dependencies.iter().cloned());
            defs.insert(name, component.schema.clone());
        }
        Ok(defs)
    }
}

fn load_component_schema_defs(
    document: &OpenApiDocument,
) -> Result<BTreeMap<String, ComponentSchemaDef>, OpenApiError> {
    let Some(components) = document.as_object().get("components") else {
        return Ok(BTreeMap::new());
    };
    let components = components
        .as_object()
        .ok_or_else(|| invalid_value(&JsonPointer::root().child("components"), "an object"))?;
    let Some(schemas) = components.get("schemas") else {
        return Ok(BTreeMap::new());
    };
    let schemas = schemas.as_object().ok_or_else(|| {
        invalid_value(
            &JsonPointer::root().child("components").child("schemas"),
            "an object",
        )
    })?;
    let mut defs = BTreeMap::new();
    for (name, schema) in schemas {
        let schema = rewrite_schema_refs(
            schema,
            &JsonPointer::root()
                .child("components")
                .child("schemas")
                .child(name),
        )?;
        let dependencies = collect_component_schema_names(&schema);
        defs.insert(
            name.clone(),
            ComponentSchemaDef {
                schema,
                dependencies,
            },
        );
    }
    Ok(defs)
}

fn collect_component_schema_names(schema: &Value) -> Vec<String> {
    let mut names = BTreeSet::new();
    collect_component_schema_names_into(schema, &mut names);
    names.into_iter().collect()
}

fn collect_component_schema_names_into(schema: &Value, names: &mut BTreeSet<String>) {
    match schema {
        Value::Object(object) => {
            if let Some(reference) = object.get("$ref").and_then(Value::as_str)
                && let Some(name) = component_schema_name_from_defs_reference(reference)
            {
                names.insert(name);
            }
            for value in object.values() {
                collect_component_schema_names_into(value, names);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_component_schema_names_into(item, names);
            }
        }
        _ => {}
    }
}

fn component_schema_name_from_defs_reference(reference: &str) -> Option<String> {
    let component_path = reference.strip_prefix("#/$defs/")?;
    let encoded_name = component_path.split('/').next()?;
    let decoded = percent_decode_str(encoded_name).decode_utf8().ok()?;
    Some(decoded.replace("~1", "/").replace("~0", "~"))
}

fn rewrite_schema_refs(schema: &Value, pointer: &JsonPointer) -> Result<Value, OpenApiError> {
    match schema {
        Value::Object(object) => {
            let mut rewritten = Map::new();
            for (key, value) in object {
                let child_pointer = pointer.child(key);
                let rewritten_value = match key.as_str() {
                    "$ref" => {
                        let reference = value
                            .as_str()
                            .ok_or_else(|| invalid_value(&child_pointer, "a string"))?;
                        Value::String(rewrite_component_schema_reference(
                            reference,
                            &child_pointer,
                        )?)
                    }
                    "additionalProperties"
                    | "contains"
                    | "contentSchema"
                    | "else"
                    | "if"
                    | "items"
                    | "not"
                    | "propertyNames"
                    | "then"
                    | "unevaluatedItems"
                    | "unevaluatedProperties" => rewrite_schema_refs(value, &child_pointer)?,
                    "$defs" | "definitions" | "dependentSchemas" | "patternProperties"
                    | "properties" => rewrite_schema_map_refs(value, &child_pointer)?,
                    "allOf" | "anyOf" | "oneOf" | "prefixItems" => {
                        rewrite_schema_array_refs(value, &child_pointer)?
                    }
                    _ => value.clone(),
                };
                rewritten.insert(key.clone(), rewritten_value);
            }
            Ok(Value::Object(rewritten))
        }
        Value::Bool(_) => Ok(schema.clone()),
        _ => Err(invalid_value(pointer, "an object or boolean schema")),
    }
}

fn rewrite_schema_map_refs(value: &Value, pointer: &JsonPointer) -> Result<Value, OpenApiError> {
    let object = value
        .as_object()
        .ok_or_else(|| invalid_value(pointer, "an object of schemas"))?;
    object
        .iter()
        .map(|(key, schema)| {
            rewrite_schema_refs(schema, &pointer.child(key)).map(|schema| (key.clone(), schema))
        })
        .collect::<Result<Map<_, _>, _>>()
        .map(Value::Object)
}

fn rewrite_schema_array_refs(value: &Value, pointer: &JsonPointer) -> Result<Value, OpenApiError> {
    let items = value
        .as_array()
        .ok_or_else(|| invalid_value(pointer, "an array of schemas"))?;
    items
        .iter()
        .enumerate()
        .map(|(index, schema)| rewrite_schema_refs(schema, &pointer.child(index.to_string())))
        .collect::<Result<Vec<_>, _>>()
        .map(Value::Array)
}

fn rewrite_component_schema_reference(
    reference: &str,
    pointer: &JsonPointer,
) -> Result<String, OpenApiError> {
    let Some(component_path) = reference.strip_prefix(COMPONENT_SCHEMA_REF_PREFIX) else {
        return Err(OpenApiError::UnsupportedReference {
            pointer: pointer.render(),
            reference: reference.to_owned(),
        });
    };
    Ok(format!("#/$defs/{component_path}"))
}

fn lookup_pointer<'a>(root: &'a Value, reference: &str) -> Option<&'a Value> {
    if reference == "#" {
        return Some(root);
    }
    let pointer = reference.strip_prefix("#/")?;
    let mut current = root;
    for token in pointer.split('/') {
        let decoded = percent_decode_str(token).decode_utf8().ok()?;
        let token = decoded.replace("~1", "/").replace("~0", "~");
        current = match current {
            Value::Object(object) => object.get(&token)?,
            Value::Array(items) => items.get(token.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(current)
}

fn invalid_value(pointer: &JsonPointer, expected: &'static str) -> OpenApiError {
    OpenApiError::InvalidValue {
        pointer: pointer.render(),
        expected,
    }
}

#[cfg(test)]
mod tests {
    use super::{lookup_pointer, rewrite_component_schema_reference, rewrite_schema_refs};
    use crate::json_pointer::JsonPointer;
    use serde_json::json;

    #[test]
    fn pointer_lookup_handles_escaped_component_names() {
        let root = json!({
            "components": {
                "schemas": {
                    "Pet/Record": { "type": "object" }
                }
            }
        });

        assert!(lookup_pointer(&root, "#/components/schemas/Pet~1Record").is_some());
    }

    #[test]
    fn component_schema_refs_lower_into_defs_refs() {
        let lowered = rewrite_component_schema_reference(
            "#/components/schemas/Pet~1Record/properties/id",
            &JsonPointer::root().child("$ref"),
        )
        .unwrap();

        assert_eq!(lowered, "#/$defs/Pet~1Record/properties/id");
    }

    #[test]
    fn property_named_ref_is_not_treated_as_a_schema_reference_keyword() {
        let lowered = rewrite_schema_refs(
            &json!({
                "type": "object",
                "properties": {
                    "$ref": {
                        "anyOf": [
                            { "type": "string" },
                            { "type": "null" }
                        ]
                    },
                    "pet": {
                        "$ref": "#/components/schemas/Pet"
                    }
                }
            }),
            &JsonPointer::root()
                .child("components")
                .child("schemas")
                .child("RefSchema-Input"),
        )
        .unwrap();

        assert_eq!(lowered["properties"]["$ref"]["anyOf"][0]["type"], "string");
        assert_eq!(lowered["properties"]["pet"]["$ref"], "#/$defs/Pet");
    }
}
