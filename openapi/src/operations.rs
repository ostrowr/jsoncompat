//! Typed operation contracts shared with protocol generation. Schema references
//! use `#/$defs/` and can be combined with component_schema_groups_with_schemas.
use super::*;

#[derive(Debug, Clone)]
pub struct ApiMedia {
    pub media_type: String,
    pub parameters: BTreeMap<String, String>,
    pub schema: Value,
}
impl ApiMedia {
    fn selector(&self) -> MediaTypeSelector {
        let (media_type, media_subtype) = self
            .media_type
            .split_once('/')
            .expect("validated media type");
        MediaTypeSelector {
            media_type: media_type.into(),
            media_subtype: media_subtype.into(),
        }
    }
    pub(super) fn envelope<'a>(&self, others: impl Iterator<Item = &'a Self>) -> Value {
        let selector = self.selector();
        let others: Vec<_> = others.map(Self::selector).collect();
        json!({"type":"object", "properties": {
            "content_type": selector.contract_schema(others.iter().filter(|other| other.is_more_specific_than(&selector))),
            "value": self.schema
        }, "required":["content_type", "value"], "additionalProperties":false})
    }
}

#[derive(Debug, Clone)]
pub enum ParameterEncoding {
    Schema(SchemaSerialization),
    Content {
        media_type: String,
        parameters: BTreeMap<String, String>,
    },
}

#[derive(Debug, Clone)]
pub struct ApiParameter {
    pub name: String,
    pub location: ParameterLocation,
    pub required: bool,
    pub schema: Value,
    pub encoding: ParameterEncoding,
}
impl ApiParameter {
    fn new(name: String, location: ParameterLocation, required: bool, value: FieldValue) -> Self {
        let (schema, encoding) = match value {
            FieldValue::Schema {
                schema,
                serialization,
            } => (schema, ParameterEncoding::Schema(serialization)),
            FieldValue::Content { media } => (
                media.schema,
                ParameterEncoding::Content {
                    media_type: media.media_type,
                    parameters: media.parameters,
                },
            ),
        };
        Self {
            name,
            location,
            required,
            schema,
            encoding,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ApiBody {
    pub required: bool,
    pub content: Vec<ApiMedia>,
}

#[derive(Debug, Clone)]
pub struct ApiResponse {
    /// An explicit three-digit status, an uppercase range such as 2XX, or default.
    pub selector: String,
    pub description: String,
    pub headers: Vec<ApiParameter>,
    pub content: Vec<ApiMedia>,
}

#[derive(Debug, Clone)]
pub struct ApiOperation {
    pub key: OperationKey,
    pub operation_id: Option<String>,
    pub description: Option<String>,
    pub parameters: Vec<ApiParameter>,
    pub body: Option<ApiBody>,
    pub responses: Vec<ApiResponse>,
    /// Effective security requirements and servers, including inherited values.
    /// Transport implementations choose credentials and server configuration.
    pub security: Value,
    pub servers: Value,
}

impl OpenApiOperationLowerer<'_> {
    pub fn operation_contract(
        &self,
        key: &OperationKey,
    ) -> Result<Option<ApiOperation>, OpenApiLoweringError> {
        let Some(resolved) = self.resolve_operation(key)? else {
            return Ok(None);
        };
        let operation = resolved.operation;
        let pointer = &resolved.pointer;
        let parameters = resolved
            .parameters
            .into_values()
            .map(|p| ApiParameter::new(p.name, p.location, p.required, p.value))
            .collect();
        let body = operation
            .get("requestBody")
            .map(|raw| {
                let pointer = pointer.child("requestBody");
                let raw = self.resolver.resolve_value(raw, &pointer)?;
                let body = raw
                    .as_object()
                    .ok_or_else(|| invalid_value(&pointer, "a request body object"))?;
                validate_request_body_fields(body, &pointer)?;
                let content = body
                    .get("content")
                    .ok_or_else(|| invalid_value(&pointer.child("content"), "a content object"))?;
                Ok::<_, OpenApiError>(ApiBody {
                    required: parse_optional_bool(body, "required", &pointer)?.unwrap_or(false),
                    content: parse_content_variants(
                        &self.resolver,
                        content,
                        &pointer.child("content"),
                        MediaTypeKeyKind::ConcreteOrRange,
                    )?,
                })
            })
            .transpose()?;
        let responses_pointer = pointer.child("responses");
        let responses = operation
            .get("responses")
            .and_then(Value::as_object)
            .ok_or_else(|| OpenApiError::MissingResponses {
                method: key.method.clone(),
                path: key.path.clone(),
            })?;
        let mut parsed = Vec::new();
        for (selector, raw) in responses {
            if selector.starts_with("x-") {
                continue;
            }
            let pointer = responses_pointer.child(selector);
            validate_response_status_selector(selector, &pointer)?;
            let response = self
                .resolver
                .resolve_value(raw, &pointer)?
                .as_object()
                .ok_or_else(|| invalid_value(&pointer, "a response object"))?;
            reject_unsupported_response_fields(response, &pointer)?;
            let description = response
                .get("description")
                .and_then(Value::as_str)
                .ok_or_else(|| invalid_value(&pointer.child("description"), "a string"))?
                .to_owned();
            let content = response
                .get("content")
                .map(|content| {
                    parse_content_variants(
                        &self.resolver,
                        content,
                        &pointer.child("content"),
                        MediaTypeKeyKind::ConcreteOrRange,
                    )
                })
                .transpose()?
                .unwrap_or_default();
            let mut headers = Vec::new();
            if let Some(raw) = response.get("headers") {
                let pointer = pointer.child("headers");
                validate_response_headers_document_shapes(raw, &pointer)?;
                for (name, raw) in raw.as_object().expect("validated headers") {
                    if name.eq_ignore_ascii_case("content-type") {
                        continue;
                    }
                    let field = lower_response_header_field(
                        &self.resolver,
                        name,
                        raw,
                        &pointer.child(name),
                    )?;
                    headers.push(ApiParameter::new(
                        field.name,
                        ParameterLocation::Header,
                        field.required,
                        field.value,
                    ));
                }
            }
            parsed.push(ApiResponse {
                selector: selector.clone(),
                description,
                headers,
                content,
            });
        }
        if parsed.is_empty() {
            return Err(invalid_value(&responses_pointer, "at least one response").into());
        }
        Ok(Some(ApiOperation {
            key: key.clone(),
            operation_id: operation
                .get("operationId")
                .and_then(Value::as_str)
                .map(str::to_owned),
            description: operation
                .get("description")
                .or_else(|| operation.get("summary"))
                .and_then(Value::as_str)
                .map(str::to_owned),
            parameters,
            body,
            responses: parsed,
            security: operation
                .get("security")
                .or_else(|| self.document.as_object().get("security"))
                .cloned()
                .unwrap_or_else(|| json!([])),
            servers: operation
                .get("servers")
                .or_else(|| resolved.path_item.get("servers"))
                .or_else(|| self.document.as_object().get("servers"))
                .cloned()
                .unwrap_or_else(|| json!([{"url":"/"}])),
        }))
    }
}
