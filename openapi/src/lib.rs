//! OpenAPI 3.1 document validation and lowering into JSON Schema envelopes.

mod json_pointer;

use email_address::EmailAddress;
use json_pointer::JsonPointer;
use json_schema_ast::{
    SCHEMA_ARRAY_CHILD_KEYWORDS, SCHEMA_MAP_CHILD_KEYWORDS, SINGLE_SCHEMA_CHILD_KEYWORDS,
    SchemaBuildError, SchemaDocument, is_schema_array_child_keyword, is_schema_map_child_keyword,
    is_single_schema_child_keyword,
};
use mime::Mime;
use percent_encoding::percent_decode_str;
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};
use url::Url;

const OPENAPI_31_PREFIX: &str = "3.1.";
const COMPONENT_SCHEMA_REF_PREFIX: &str = "#/components/schemas/";
const SUPPORTED_REF_PREFIX: &str = "#/";
const JSON_SCHEMA_DRAFT_2020_12: &str = "https://json-schema.org/draft/2020-12/schema";
const JSON_SCHEMA_DRAFT_2020_12_WITH_FRAGMENT: &str =
    "https://json-schema.org/draft/2020-12/schema#";
const OPENAPI_31_SCHEMA_OBJECT_DIALECT: &str = "https://spec.openapis.org/oas/3.1/dialect/base";
const SUPPORTED_SCHEMA_DIALECTS: &str = "https://json-schema.org/draft/2020-12/schema or https://spec.openapis.org/oas/3.1/dialect/base";
const MAX_EXACT_F64_INTEGER: f64 = 9_007_199_254_740_991.0;

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
    #[error("OpenAPI compatibility checks do not support {feature} at '{pointer}' yet")]
    UnsupportedCompatibilityFeature {
        pointer: String,
        feature: &'static str,
    },
    #[error("unsupported OpenAPI reference '{reference}' at '{pointer}'")]
    UnsupportedReference { pointer: String, reference: String },
    #[error("OpenAPI reference '{reference}' at '{pointer}' did not resolve")]
    UnresolvedReference { pointer: String, reference: String },
    #[error("OpenAPI reference chain at '{pointer}' forms a cycle through '{reference}'")]
    CyclicReference { pointer: String, reference: String },
    #[error(
        "unsupported OpenAPI jsonSchemaDialect '{actual}' at '{pointer}': expected '{expected}'"
    )]
    UnsupportedSchemaDialect {
        pointer: String,
        expected: &'static str,
        actual: String,
    },
    #[error("OpenAPI operation '{method} {path}' is missing a responses object")]
    MissingResponses { method: String, path: String },
    #[error("duplicate OpenAPI parameter '{location}:{name}' in '{pointer}'")]
    DuplicateParameter {
        pointer: String,
        location: String,
        name: String,
    },
    #[error("duplicate OpenAPI response header '{name}' in '{pointer}'")]
    DuplicateResponseHeader { pointer: String, name: String },
    #[error(
        "OpenAPI media types '{previous}' and '{current}' collapse to the same compatibility selector '{selector}' at '{pointer}'"
    )]
    DuplicateNormalizedMediaType {
        pointer: String,
        previous: String,
        current: String,
        selector: String,
    },
    #[error("duplicate OpenAPI operationId '{operation_id}' in '{pointer}'")]
    DuplicateOperationId {
        pointer: String,
        operation_id: String,
    },
    #[error("OpenAPI schema at '{pointer}' is invalid: {source}")]
    InvalidSchema {
        pointer: String,
        #[source]
        source: SchemaBuildError,
    },
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OpenApiLoweringError {
    #[error(transparent)]
    OpenApi(#[from] OpenApiError),
    #[error(transparent)]
    Schema(#[from] SchemaBuildError),
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
            .ok_or(OpenApiError::MissingVersion)?
            .as_str()
            .ok_or_else(|| invalid_value(&JsonPointer::root().child("openapi"), "a string"))?;
        if !is_supported_openapi_31_version(version) {
            return Err(OpenApiError::UnsupportedVersion {
                actual: version.to_owned(),
            });
        }
        let info = object
            .get("info")
            .and_then(Value::as_object)
            .ok_or_else(|| invalid_value(&JsonPointer::root().child("info"), "an object"))?;
        for field in ["title", "version"] {
            if !info.get(field).is_some_and(Value::is_string) {
                return Err(invalid_value(
                    &JsonPointer::root().child("info").child(field),
                    "a string",
                ));
            }
        }
        validate_info_fields(info, &JsonPointer::root().child("info"))?;
        validate_document_fields(object)?;
        validate_document_schema_dialect_field(object)?;
        if !["paths", "components", "webhooks"]
            .iter()
            .any(|field| object.contains_key(*field))
        {
            return Err(invalid_value(
                &JsonPointer::root(),
                "at least one of 'paths', 'components', or 'webhooks'",
            ));
        }
        if let Some(paths) = object.get("paths")
            && !paths.is_object()
        {
            return Err(invalid_value(
                &JsonPointer::root().child("paths"),
                "an object",
            ));
        }
        if let Some(paths) = object.get("paths").and_then(Value::as_object) {
            validate_paths_object_shape(paths)?;
        }
        if let Some(components) = object.get("components")
            && !components.is_object()
        {
            return Err(invalid_value(
                &JsonPointer::root().child("components"),
                "an object",
            ));
        }
        validate_contract_container_shapes(object)?;
        validate_path_template_parameter_bindings(raw, object)?;
        validate_locally_referenced_request_body_encoding_keys(raw, object)?;
        let operations = collect_operation_index(object)?;
        validate_resolvable_link_targets(object, &operations)?;
        validate_declared_security_requirement_names(object)?;
        let document = Self { raw: raw.clone() };
        document.validate_document_schema_validity()?;
        Ok(document)
    }

    fn as_object(&self) -> &Map<String, Value> {
        self.raw
            .as_object()
            .expect("OpenApiDocument validates its root object at construction")
    }

    fn schema_document(&self, mut schema: Value) -> Result<SchemaDocument, OpenApiLoweringError> {
        let schema_object = schema.as_object_mut().ok_or_else(|| {
            invalid_value(
                &JsonPointer::root(),
                "an object schema when lowering an OpenAPI contract",
            )
        })?;
        let schema_dialect = self.supported_schema_dialect()?;
        schema_object.insert(
            "$schema".to_owned(),
            Value::String(schema_dialect.uri().to_owned()),
        );
        Ok(SchemaDocument::from_json(&schema)?)
    }

    pub fn lowered_contract_document(
        &self,
        schema: &Value,
    ) -> Result<SchemaDocument, OpenApiLoweringError> {
        self.schema_document(schema.clone())
    }

    pub fn uses_same_schema_dialect_as(&self, other: &Self) -> Result<bool, OpenApiLoweringError> {
        Ok(self.supported_schema_dialect()? == other.supported_schema_dialect()?)
    }

    fn supported_schema_dialect(&self) -> Result<OpenApiSchemaDialect, OpenApiError> {
        OpenApiSchemaDialect::for_lowering(self.as_object())
    }

    fn validate_document_schema_validity(&self) -> Result<(), OpenApiError> {
        match self.supported_schema_dialect() {
            Ok(_) => {}
            Err(OpenApiError::UnsupportedSchemaDialect { .. }) => return Ok(()),
            Err(error) => return Err(error),
        }

        let defs = load_component_schema_defs_for_validation(self)?;
        self.validate_component_schema_defs(&defs)?;
        self.validate_response_component_schema_roots(&defs)?;
        self.validate_parameter_component_schema_roots(&defs)?;
        self.validate_request_body_component_schema_roots(&defs)?;
        self.validate_header_component_schema_roots(&defs)?;
        self.validate_inline_contract_schema_roots(&defs)?;
        Ok(())
    }

    fn validate_component_schema_defs(
        &self,
        defs: &BTreeMap<String, ComponentSchemaDef>,
    ) -> Result<(), OpenApiError> {
        if defs.is_empty() {
            return Ok(());
        }

        match self.validate_component_schema_selection(defs, defs.keys().map(String::as_str)) {
            Ok(ComponentSchemaValidation::Validated) => return Ok(()),
            Ok(ComponentSchemaValidation::Deferred) => {}
            Err(aggregate_source) => {
                for name in defs.keys() {
                    if let Err(source) = self
                        .validate_component_schema_selection(defs, std::iter::once(name.as_str()))
                    {
                        return Err(OpenApiError::InvalidSchema {
                            pointer: JsonPointer::root()
                                .child("components")
                                .child("schemas")
                                .child(name)
                                .render(),
                            source,
                        });
                    }
                }

                return Err(OpenApiError::InvalidSchema {
                    pointer: JsonPointer::root()
                        .child("components")
                        .child("schemas")
                        .render(),
                    source: aggregate_source,
                });
            }
        }

        for name in defs.keys() {
            if let Err(source) =
                self.validate_component_schema_selection(defs, std::iter::once(name.as_str()))
            {
                return Err(OpenApiError::InvalidSchema {
                    pointer: JsonPointer::root()
                        .child("components")
                        .child("schemas")
                        .child(name)
                        .render(),
                    source,
                });
            }
        }

        Ok(())
    }

    fn validate_component_schema_selection<'a>(
        &self,
        defs: &BTreeMap<String, ComponentSchemaDef>,
        names: impl Iterator<Item = &'a str>,
    ) -> Result<ComponentSchemaValidation, SchemaBuildError> {
        let component_refs = names
            .map(|name| {
                json!({
                    "$ref": JsonPointer::root()
                        .child("$defs")
                        .child(name)
                        .render()
                })
            })
            .collect::<Vec<_>>();
        let selection_schema = json!({ "anyOf": component_refs.clone() });
        let defs = component_schema_defs_for_schema(defs, &selection_schema);
        let validation_schema = json!({
            "$defs": defs,
            "anyOf": component_refs
        });
        let should_defer_reference_validation =
            schema_uses_later_lowering_reference_features(&validation_schema);
        if should_defer_reference_validation {
            let backend_schema = self
                .schema_document(strip_deferred_schema_references_for_validation(
                    &validation_schema,
                    DeferredReferenceValidation::Backend,
                ))
                .map_err(|error| match error {
                    OpenApiLoweringError::Schema(source) => source,
                    OpenApiLoweringError::OpenApi(error) => {
                        panic!(
                            "supported schema dialect unexpectedly failed during validation: {error}"
                        )
                    }
                })?;
            backend_schema.validate_source_schema()?;
            let schema = self
                .schema_document(strip_deferred_schema_references_for_validation(
                    &validation_schema,
                    DeferredReferenceValidation::ResolvedAst,
                ))
                .map_err(|error| match error {
                    OpenApiLoweringError::Schema(source) => source,
                    OpenApiLoweringError::OpenApi(error) => {
                        panic!(
                            "supported schema dialect unexpectedly failed during validation: {error}"
                        )
                    }
                })?;
            schema.root()?;
            schema.validate_source_schema()?;
            return Ok(ComponentSchemaValidation::Deferred);
        }
        let schema = self
            .schema_document(validation_schema)
            .map_err(|error| match error {
                OpenApiLoweringError::Schema(source) => source,
                OpenApiLoweringError::OpenApi(error) => {
                    panic!(
                        "supported schema dialect unexpectedly failed during validation: {error}"
                    )
                }
            })?;
        schema.root()?;
        schema.validate_source_schema()?;
        Ok(ComponentSchemaValidation::Validated)
    }

    fn validate_response_component_schema_roots(
        &self,
        defs: &BTreeMap<String, ComponentSchemaDef>,
    ) -> Result<(), OpenApiError> {
        let Some(responses) = self
            .as_object()
            .get("components")
            .and_then(Value::as_object)
            .and_then(|components| components.get("responses"))
            .and_then(Value::as_object)
        else {
            return Ok(());
        };
        let responses_pointer = JsonPointer::root().child("components").child("responses");
        for (name, raw_response) in responses {
            let response_pointer = responses_pointer.child(name);
            let response =
                match resolve_reference_chain(&self.raw, raw_response, &response_pointer)? {
                    ReferenceResolution::Value(response) => response,
                    ReferenceResolution::ExternalReference { .. } => continue,
                };
            let Some(response) = response.as_object() else {
                continue;
            };
            if let Some(content) = response.get("content").and_then(Value::as_object) {
                self.validate_content_schema_roots(
                    content,
                    &response_pointer.child("content"),
                    defs,
                )?;
            }
            if let Some(headers) = response.get("headers").and_then(Value::as_object) {
                self.validate_header_map_schema_roots(
                    headers,
                    &response_pointer.child("headers"),
                    defs,
                )?;
            }
        }
        Ok(())
    }

    fn validate_parameter_component_schema_roots(
        &self,
        defs: &BTreeMap<String, ComponentSchemaDef>,
    ) -> Result<(), OpenApiError> {
        let Some(parameters) = self
            .as_object()
            .get("components")
            .and_then(Value::as_object)
            .and_then(|components| components.get("parameters"))
            .and_then(Value::as_object)
        else {
            return Ok(());
        };
        let parameters_pointer = JsonPointer::root().child("components").child("parameters");
        for (name, raw_parameter) in parameters {
            let parameter_pointer = parameters_pointer.child(name);
            let parameter =
                match resolve_reference_chain(&self.raw, raw_parameter, &parameter_pointer)? {
                    ReferenceResolution::Value(parameter) => parameter,
                    ReferenceResolution::ExternalReference { .. } => continue,
                };
            let Some(parameter) = parameter.as_object() else {
                continue;
            };
            self.validate_field_schema_roots(parameter, &parameter_pointer, defs)?;
        }
        Ok(())
    }

    fn validate_request_body_component_schema_roots(
        &self,
        defs: &BTreeMap<String, ComponentSchemaDef>,
    ) -> Result<(), OpenApiError> {
        let Some(request_bodies) = self
            .as_object()
            .get("components")
            .and_then(Value::as_object)
            .and_then(|components| components.get("requestBodies"))
            .and_then(Value::as_object)
        else {
            return Ok(());
        };
        let request_bodies_pointer = JsonPointer::root()
            .child("components")
            .child("requestBodies");
        for (name, raw_body) in request_bodies {
            let body_pointer = request_bodies_pointer.child(name);
            let body = match resolve_reference_chain(&self.raw, raw_body, &body_pointer)? {
                ReferenceResolution::Value(body) => body,
                ReferenceResolution::ExternalReference { .. } => continue,
            };
            let Some(content) = body
                .as_object()
                .and_then(|body| body.get("content"))
                .and_then(Value::as_object)
            else {
                continue;
            };
            self.validate_content_schema_roots(content, &body_pointer.child("content"), defs)?;
        }
        Ok(())
    }

    fn validate_header_component_schema_roots(
        &self,
        defs: &BTreeMap<String, ComponentSchemaDef>,
    ) -> Result<(), OpenApiError> {
        let Some(headers) = self
            .as_object()
            .get("components")
            .and_then(Value::as_object)
            .and_then(|components| components.get("headers"))
            .and_then(Value::as_object)
        else {
            return Ok(());
        };
        self.validate_header_map_schema_roots(
            headers,
            &JsonPointer::root().child("components").child("headers"),
            defs,
        )
    }

    fn validate_inline_contract_schema_roots(
        &self,
        defs: &BTreeMap<String, ComponentSchemaDef>,
    ) -> Result<(), OpenApiError> {
        self.validate_path_item_container_schema_roots(
            self.as_object().get("paths").and_then(Value::as_object),
            &JsonPointer::root().child("paths"),
            defs,
            true,
        )?;
        self.validate_path_item_container_schema_roots(
            self.as_object().get("webhooks").and_then(Value::as_object),
            &JsonPointer::root().child("webhooks"),
            defs,
            false,
        )?;
        self.validate_component_callback_schema_roots(defs)?;
        self.validate_component_path_item_schema_roots(defs)
    }

    fn validate_path_item_container_schema_roots(
        &self,
        path_items: Option<&Map<String, Value>>,
        pointer: &JsonPointer,
        defs: &BTreeMap<String, ComponentSchemaDef>,
        allow_extension_entries: bool,
    ) -> Result<(), OpenApiError> {
        let Some(path_items) = path_items else {
            return Ok(());
        };

        for (path, path_item) in path_items {
            if allow_extension_entries && path.starts_with("x-") {
                continue;
            }
            let path_pointer = pointer.child(path);
            let Some(path_item) = path_item.as_object() else {
                continue;
            };
            if path_item.contains_key("$ref") {
                continue;
            }

            self.validate_parameter_array_schema_roots(
                path_item.get("parameters"),
                &path_pointer.child("parameters"),
                defs,
            )?;

            for method in HTTP_METHODS {
                let Some(operation) = path_item.get(method).and_then(Value::as_object) else {
                    continue;
                };
                let operation_pointer = path_pointer.child(method);
                self.validate_parameter_array_schema_roots(
                    operation.get("parameters"),
                    &operation_pointer.child("parameters"),
                    defs,
                )?;
                self.validate_request_body_schema_roots(
                    operation.get("requestBody"),
                    &operation_pointer.child("requestBody"),
                    defs,
                )?;
                self.validate_response_map_schema_roots(
                    operation.get("responses"),
                    &operation_pointer.child("responses"),
                    defs,
                )?;
                self.validate_callbacks_schema_roots(
                    operation.get("callbacks"),
                    &operation_pointer.child("callbacks"),
                    defs,
                )?;
            }
        }

        Ok(())
    }

    fn validate_component_callback_schema_roots(
        &self,
        defs: &BTreeMap<String, ComponentSchemaDef>,
    ) -> Result<(), OpenApiError> {
        self.validate_callbacks_schema_roots(
            self.as_object()
                .get("components")
                .and_then(Value::as_object)
                .and_then(|components| components.get("callbacks")),
            &JsonPointer::root().child("components").child("callbacks"),
            defs,
        )
    }

    fn validate_component_path_item_schema_roots(
        &self,
        defs: &BTreeMap<String, ComponentSchemaDef>,
    ) -> Result<(), OpenApiError> {
        self.validate_path_item_container_schema_roots(
            self.as_object()
                .get("components")
                .and_then(Value::as_object)
                .and_then(|components| components.get("pathItems"))
                .and_then(Value::as_object),
            &JsonPointer::root().child("components").child("pathItems"),
            defs,
            false,
        )
    }

    fn validate_callbacks_schema_roots(
        &self,
        raw_callbacks: Option<&Value>,
        pointer: &JsonPointer,
        defs: &BTreeMap<String, ComponentSchemaDef>,
    ) -> Result<(), OpenApiError> {
        let Some(callbacks) = raw_callbacks.and_then(Value::as_object) else {
            return Ok(());
        };

        for (name, raw_callback) in callbacks {
            let callback_pointer = pointer.child(name);
            let Some(callback) = raw_callback.as_object() else {
                continue;
            };
            if callback.contains_key("$ref") {
                continue;
            }
            self.validate_path_item_container_schema_roots(
                Some(callback),
                &callback_pointer,
                defs,
                true,
            )?;
        }

        Ok(())
    }

    fn validate_parameter_array_schema_roots(
        &self,
        raw_parameters: Option<&Value>,
        pointer: &JsonPointer,
        defs: &BTreeMap<String, ComponentSchemaDef>,
    ) -> Result<(), OpenApiError> {
        let Some(parameters) = raw_parameters.and_then(Value::as_array) else {
            return Ok(());
        };

        for (index, raw_parameter) in parameters.iter().enumerate() {
            let parameter_pointer = pointer.child(index.to_string());
            let parameter =
                match resolve_reference_chain(&self.raw, raw_parameter, &parameter_pointer)? {
                    ReferenceResolution::Value(parameter) => parameter,
                    ReferenceResolution::ExternalReference { .. } => continue,
                };
            let Some(parameter) = parameter.as_object() else {
                continue;
            };
            self.validate_field_schema_roots(parameter, &parameter_pointer, defs)?;
        }

        Ok(())
    }

    fn validate_request_body_schema_roots(
        &self,
        raw_body: Option<&Value>,
        pointer: &JsonPointer,
        defs: &BTreeMap<String, ComponentSchemaDef>,
    ) -> Result<(), OpenApiError> {
        let Some(raw_body) = raw_body else {
            return Ok(());
        };
        let body = match resolve_reference_chain(&self.raw, raw_body, pointer)? {
            ReferenceResolution::Value(body) => body,
            ReferenceResolution::ExternalReference { .. } => return Ok(()),
        };
        let Some(content) = body
            .as_object()
            .and_then(|body| body.get("content"))
            .and_then(Value::as_object)
        else {
            return Ok(());
        };
        self.validate_content_schema_roots(content, &pointer.child("content"), defs)
    }

    fn validate_response_map_schema_roots(
        &self,
        raw_responses: Option<&Value>,
        pointer: &JsonPointer,
        defs: &BTreeMap<String, ComponentSchemaDef>,
    ) -> Result<(), OpenApiError> {
        let Some(responses) = raw_responses.and_then(Value::as_object) else {
            return Ok(());
        };

        for (status, raw_response) in responses {
            if status.starts_with("x-") {
                continue;
            }
            let response_pointer = pointer.child(status);
            let response =
                match resolve_reference_chain(&self.raw, raw_response, &response_pointer)? {
                    ReferenceResolution::Value(response) => response,
                    ReferenceResolution::ExternalReference { .. } => continue,
                };
            let Some(response) = response.as_object() else {
                continue;
            };
            if let Some(content) = response.get("content").and_then(Value::as_object) {
                self.validate_content_schema_roots(
                    content,
                    &response_pointer.child("content"),
                    defs,
                )?;
            }
            if let Some(headers) = response.get("headers").and_then(Value::as_object) {
                self.validate_header_map_schema_roots(
                    headers,
                    &response_pointer.child("headers"),
                    defs,
                )?;
            }
        }

        Ok(())
    }

    fn validate_header_map_schema_roots(
        &self,
        headers: &Map<String, Value>,
        pointer: &JsonPointer,
        defs: &BTreeMap<String, ComponentSchemaDef>,
    ) -> Result<(), OpenApiError> {
        for (name, raw_header) in headers {
            let header_pointer = pointer.child(name);
            let header = match resolve_reference_chain(&self.raw, raw_header, &header_pointer)? {
                ReferenceResolution::Value(header) => header,
                ReferenceResolution::ExternalReference { .. } => continue,
            };
            let Some(header) = header.as_object() else {
                continue;
            };
            self.validate_field_schema_roots(header, &header_pointer, defs)?;
        }
        Ok(())
    }

    fn validate_field_schema_roots(
        &self,
        field: &Map<String, Value>,
        pointer: &JsonPointer,
        defs: &BTreeMap<String, ComponentSchemaDef>,
    ) -> Result<(), OpenApiError> {
        if let Some(schema) = field.get("schema") {
            self.validate_document_schema_root(schema, &pointer.child("schema"), defs)?;
        }
        if let Some(content) = field.get("content").and_then(Value::as_object) {
            self.validate_content_schema_roots(content, &pointer.child("content"), defs)?;
        }
        Ok(())
    }

    fn validate_content_schema_roots(
        &self,
        content: &Map<String, Value>,
        pointer: &JsonPointer,
        defs: &BTreeMap<String, ComponentSchemaDef>,
    ) -> Result<(), OpenApiError> {
        for (media_type, media) in content {
            let Some(schema) = media.as_object().and_then(|media| media.get("schema")) else {
                continue;
            };
            self.validate_document_schema_root(
                schema,
                &pointer.child(media_type).child("schema"),
                defs,
            )?;
        }
        Ok(())
    }

    fn validate_document_schema_root(
        &self,
        schema: &Value,
        pointer: &JsonPointer,
        defs: &BTreeMap<String, ComponentSchemaDef>,
    ) -> Result<(), OpenApiError> {
        let schema = rewrite_schema_refs_for_validation(schema, pointer)?;
        let defs = component_schema_defs_for_schema(defs, &schema);
        let validation_schema = json!({
            "$defs": defs,
            "type": "object",
            "properties": {
                "value": schema
            },
            "required": ["value"],
            "additionalProperties": false
        });
        let should_defer_reference_validation =
            schema_uses_later_lowering_reference_features(&validation_schema);
        if should_defer_reference_validation {
            let backend_schema = self
                .schema_document(strip_deferred_schema_references_for_validation(
                    &validation_schema,
                    DeferredReferenceValidation::Backend,
                ))
                .map_err(|error| match error {
                    OpenApiLoweringError::Schema(source) => OpenApiError::InvalidSchema {
                        pointer: pointer.render(),
                        source,
                    },
                    OpenApiLoweringError::OpenApi(error) => error,
                })?;
            backend_schema.validate_source_schema().map_err(|source| {
                OpenApiError::InvalidSchema {
                    pointer: pointer.render(),
                    source,
                }
            })?;
            let schema = self
                .schema_document(strip_deferred_schema_references_for_validation(
                    &validation_schema,
                    DeferredReferenceValidation::ResolvedAst,
                ))
                .map_err(|error| match error {
                    OpenApiLoweringError::Schema(source) => OpenApiError::InvalidSchema {
                        pointer: pointer.render(),
                        source,
                    },
                    OpenApiLoweringError::OpenApi(error) => error,
                })?;
            return schema
                .root()
                .and_then(|_| schema.validate_source_schema())
                .map_err(|source| OpenApiError::InvalidSchema {
                    pointer: pointer.render(),
                    source,
                });
        }
        let schema = self
            .schema_document(validation_schema)
            .map_err(|error| match error {
                OpenApiLoweringError::Schema(source) => OpenApiError::InvalidSchema {
                    pointer: pointer.render(),
                    source,
                },
                OpenApiLoweringError::OpenApi(error) => error,
            })?;
        schema
            .root()
            .and_then(|_| schema.validate_source_schema())
            .map_err(|source| OpenApiError::InvalidSchema {
                pointer: pointer.render(),
                source,
            })
    }
}

fn is_supported_openapi_31_version(version: &str) -> bool {
    version
        .strip_prefix(OPENAPI_31_PREFIX)
        .is_some_and(|patch| !patch.is_empty() && patch.bytes().all(|byte| byte.is_ascii_digit()))
}

fn validate_info_fields(
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

fn validate_document_fields(object: &Map<String, Value>) -> Result<(), OpenApiError> {
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

fn validate_document_schema_dialect_field(object: &Map<String, Value>) -> Result<(), OpenApiError> {
    validate_optional_absolute_uri_field(object, "jsonSchemaDialect", &JsonPointer::root())
}

fn validate_optional_webhooks_field(
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

fn validate_contract_container_shapes(object: &Map<String, Value>) -> Result<(), OpenApiError> {
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

fn validate_path_template_parameter_bindings(
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

fn validate_locally_referenced_request_body_encoding_keys(
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

fn validate_path_item_request_body_encoding_keys(
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

fn validate_request_body_encoding_keys_against_locally_referenced_media_schema(
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

fn validate_encoding_keys_against_locally_referenced_media_schema_properties(
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
struct DocumentParameterBindings {
    path_names: BTreeSet<String>,
    has_external_reference: bool,
}

fn collect_path_template_parameter_bindings(
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

fn require_document_path_template_parameters(
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

fn validate_path_item_container_shapes(
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

fn validate_operation_container_shapes(
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

fn validate_parameter_array_document_shapes(
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

fn validate_parameter_document_shape(
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

fn validate_request_body_document_shape(
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

fn validate_operation_response_document_shapes(
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

fn validate_response_document_shape(
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

fn validate_component_collection_container_shapes(
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

fn validate_component_collection_document_shapes(
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

fn validate_paths_object_shape(paths: &Map<String, Value>) -> Result<(), OpenApiError> {
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

fn normalized_path_template_shape(
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

fn validate_path_item_document_fields(
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

fn validate_path_item_fields(
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

fn validate_operation_document_fields(
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

fn validate_operation_fields(
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

fn validate_callbacks_field(callbacks: &Value, pointer: &JsonPointer) -> Result<(), OpenApiError> {
    let callbacks = callbacks
        .as_object()
        .ok_or_else(|| invalid_value(pointer, "an object"))?;
    for (name, callback) in callbacks {
        validate_callback_object_or_reference(callback, &pointer.child(name))?;
    }
    Ok(())
}

fn validate_callback_object_or_reference(
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

fn validate_callback_key_expressions(
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

fn is_valid_callback_key(key: &str) -> bool {
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

fn is_valid_runtime_expression(expression: &str) -> bool {
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

fn is_valid_header_token(token: &str) -> bool {
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

fn is_valid_runtime_json_pointer(pointer: &str) -> bool {
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

fn validate_component_fields(
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

fn validate_unsupported_component_collection(
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

fn validate_component_document_fields(
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

fn validate_optional_string_field(
    object: &Map<String, Value>,
    field: &str,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    if object.get(field).is_some_and(|value| !value.is_string()) {
        return Err(invalid_value(&pointer.child(field), "a string"));
    }

    Ok(())
}

fn validate_optional_url_reference_field(
    object: &Map<String, Value>,
    field: &str,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    validate_optional_reference_field(object, field, pointer, "a valid URL reference")
}

fn validate_optional_uri_reference_field(
    object: &Map<String, Value>,
    field: &str,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    validate_optional_reference_field(object, field, pointer, "a valid URI reference")
}

fn validate_optional_reference_field(
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

fn validate_optional_absolute_uri_field(
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

fn validate_optional_email_field(
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

fn is_valid_reference(value: &str) -> bool {
    Url::parse(value).is_ok()
        || Url::parse("https://jsoncompat.invalid/")
            .expect("static URL base is valid")
            .join(value)
            .is_ok()
}

fn validate_optional_bool_field(
    object: &Map<String, Value>,
    field: &str,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    if object.get(field).is_some_and(|value| !value.is_boolean()) {
        return Err(invalid_value(&pointer.child(field), "a boolean"));
    }

    Ok(())
}

fn validate_optional_object_field(
    object: &Map<String, Value>,
    field: &str,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    if object.get(field).is_some_and(|value| !value.is_object()) {
        return Err(invalid_value(&pointer.child(field), "an object"));
    }

    Ok(())
}

fn validate_optional_string_array_field(
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

fn validate_optional_contact_field(
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

fn validate_optional_license_field(
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

fn validate_optional_external_docs_field(
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

fn validate_optional_servers_field(
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

fn validate_server_object(server: &Value, pointer: &JsonPointer) -> Result<(), OpenApiError> {
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

fn require_server_url_field(
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

fn is_valid_server_url_template(url: &str) -> bool {
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

fn validate_server_variable_object(
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

fn validate_optional_tags_field(
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

fn validate_tag_object<'a>(tag: &'a Value, pointer: &JsonPointer) -> Result<&'a str, OpenApiError> {
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

fn validate_optional_security_field(
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

fn validate_declared_security_requirement_names(
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

fn validate_security_requirement_names_in_component_callbacks(
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

fn validate_security_requirement_names_in_component_path_items(
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

fn validate_security_requirement_names_in_path_item_container(
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

fn validate_security_requirement_names_in_operation(
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

fn validate_security_requirement_names_in_callback(
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

fn validate_security_requirement_names(
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
struct OperationIndex {
    ids: BTreeSet<String>,
    local_references: BTreeSet<String>,
}

fn collect_operation_index(document: &Map<String, Value>) -> Result<OperationIndex, OpenApiError> {
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

fn validate_resolvable_link_targets(
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

fn validate_link_targets_in_component_callbacks(
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

fn validate_link_targets_in_component_path_items(
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

fn validate_link_targets_in_component_responses(
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

fn validate_link_targets_in_component_links(
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

fn validate_link_targets_in_path_item_container(
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

fn validate_link_targets_in_operation(
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

fn validate_link_targets_in_callback(
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

fn validate_link_targets_in_response(
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

fn validate_link_target(
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

fn collect_operations_in_component_callbacks(
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

fn collect_operations_in_component_path_items(
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

fn collect_operations_in_path_item_container(
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

fn collect_operations_in_operation(
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

fn collect_operations_in_callback(
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

fn validate_security_scheme_object(
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

fn validate_security_scheme_document_shape(
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

fn validate_oauth_flows_field(
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

fn validate_oauth_flow_object(
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

fn validate_object_fields(
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

fn require_string_field(
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

fn require_url_reference_field(
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

fn require_string_field_value<'a>(
    object: &'a Map<String, Value>,
    field: &str,
    pointer: &JsonPointer,
) -> Result<&'a str, OpenApiError> {
    object
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_value(&pointer.child(field), "a string"))
}

fn require_string_enum_field(
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

fn reject_present_fields(
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

fn validate_string_map_field(
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

fn validate_discriminator(
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

fn validate_schema_document_shape(
    schema: &Value,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    validate_schema_document_shape_with_context(schema, pointer, SchemaLocation::NonProperty)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SchemaLocation {
    Property,
    NonProperty,
}

fn validate_schema_document_shape_with_context(
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpenApiSchemaDialect {
    JsonSchemaDraft202012,
    OpenApi31SchemaObject,
}

impl OpenApiSchemaDialect {
    fn for_lowering(object: &Map<String, Value>) -> Result<Self, OpenApiError> {
        let pointer = JsonPointer::root().child("jsonSchemaDialect");
        let Some(raw_dialect) = object.get("jsonSchemaDialect") else {
            return Ok(Self::OpenApi31SchemaObject);
        };
        let dialect = raw_dialect
            .as_str()
            .ok_or_else(|| invalid_value(&pointer, "a string"))?;
        match dialect {
            JSON_SCHEMA_DRAFT_2020_12 | JSON_SCHEMA_DRAFT_2020_12_WITH_FRAGMENT => {
                Ok(Self::JsonSchemaDraft202012)
            }
            OPENAPI_31_SCHEMA_OBJECT_DIALECT => Ok(Self::OpenApi31SchemaObject),
            _ => Err(OpenApiError::UnsupportedSchemaDialect {
                pointer: pointer.render(),
                expected: SUPPORTED_SCHEMA_DIALECTS,
                actual: dialect.to_owned(),
            }),
        }
    }

    const fn uri(self) -> &'static str {
        match self {
            Self::JsonSchemaDraft202012 => JSON_SCHEMA_DRAFT_2020_12,
            Self::OpenApi31SchemaObject => OPENAPI_31_SCHEMA_OBJECT_DIALECT,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OperationKey {
    pub method: String,
    pub path: String,
}

#[derive(Debug)]
pub struct LoweredOperation {
    pub request: Value,
    pub response: Value,
}

pub struct OpenApiOperationLowerer<'a> {
    document: &'a OpenApiDocument,
    resolver: Resolver<'a>,
}

impl<'a> OpenApiOperationLowerer<'a> {
    pub fn new(document: &'a OpenApiDocument) -> Result<Self, OpenApiLoweringError> {
        reject_unsupported_document_contract_surfaces(document)?;
        document.supported_schema_dialect()?;
        let resolver = Resolver::new(document)?;
        Ok(Self { document, resolver })
    }

    pub fn operation_keys(&self) -> Result<BTreeSet<OperationKey>, OpenApiLoweringError> {
        let mut operations = BTreeSet::new();
        let paths_pointer = JsonPointer::root().child("paths");
        let Some(paths) = self.document.as_object().get("paths") else {
            return Ok(operations);
        };
        let paths = paths
            .as_object()
            .expect("OpenApiDocument validates paths as an object");

        for (path, path_item) in paths {
            let path_pointer = paths_pointer.child(path);
            if path.starts_with("x-") {
                continue;
            }
            if !path.starts_with('/') {
                return Err(invalid_value(
                    &path_pointer,
                    "a path template key beginning with '/' or a specification extension beginning with 'x-'",
                )
                .into());
            }
            let path_template_names = path_template_names(path, &path_pointer)?;
            reject_unsupported_path_item_reference(path_item, &path_pointer)?;
            let path_item = self.resolver.resolve_value(path_item, &path_pointer)?;
            let path_item = path_item
                .as_object()
                .ok_or_else(|| invalid_value(&path_pointer, "an object or local reference"))?;
            validate_path_item_fields(path_item, &path_pointer)?;
            collect_parameters(
                &self.resolver,
                path_item.get("parameters"),
                &path_pointer.child("parameters"),
                &path_template_names,
            )?;

            for method in HTTP_METHODS {
                let Some(operation_value) = path_item.get(method) else {
                    continue;
                };
                let operation_pointer = path_pointer.child(method);
                let operation = operation_value
                    .as_object()
                    .ok_or_else(|| invalid_value(&operation_pointer, "an object"))?;
                validate_operation_fields(operation, &operation_pointer)?;
                operations.insert(OperationKey {
                    method: method.to_ascii_uppercase(),
                    path: path.clone(),
                });
            }
        }

        Ok(operations)
    }

    pub fn lower_operation(
        &self,
        key: &OperationKey,
    ) -> Result<Option<LoweredOperation>, OpenApiLoweringError> {
        let paths_pointer = JsonPointer::root().child("paths");
        let Some(paths) = self.document.as_object().get("paths") else {
            return Ok(None);
        };
        let paths = paths
            .as_object()
            .expect("OpenApiDocument validates paths as an object");
        let Some(path_item) = paths.get(&key.path) else {
            return Ok(None);
        };
        let path_pointer = paths_pointer.child(&key.path);
        let method = key.method.to_ascii_lowercase();
        if !HTTP_METHODS.contains(&method.as_str()) {
            return Ok(None);
        }

        let path_template_names = path_template_names(&key.path, &path_pointer)?;
        reject_unsupported_path_item_reference(path_item, &path_pointer)?;
        let path_item = self.resolver.resolve_value(path_item, &path_pointer)?;
        let path_item = path_item
            .as_object()
            .ok_or_else(|| invalid_value(&path_pointer, "an object or local reference"))?;
        validate_path_item_fields(path_item, &path_pointer)?;
        let path_parameters = collect_parameters(
            &self.resolver,
            path_item.get("parameters"),
            &path_pointer.child("parameters"),
            &path_template_names,
        )?;
        let Some(operation_value) = path_item.get(&method) else {
            return Ok(None);
        };
        let operation_pointer = path_pointer.child(&method);
        let operation = operation_value
            .as_object()
            .ok_or_else(|| invalid_value(&operation_pointer, "an object"))?;
        validate_operation_fields(operation, &operation_pointer)?;
        let operation_parameters = collect_parameters(
            &self.resolver,
            operation.get("parameters"),
            &operation_pointer.child("parameters"),
            &path_template_names,
        )?;
        let parameters = merge_parameters(path_parameters, operation_parameters);
        require_path_template_parameters(
            &path_template_names,
            &parameters,
            &operation_pointer.child("parameters"),
        )?;

        Ok(Some(LoweredOperation {
            request: lower_request_schema(
                &self.resolver,
                operation,
                &operation_pointer,
                &parameters,
            )?,
            response: lower_response_schema(
                &self.resolver,
                operation,
                &operation_pointer,
                &method,
                &key.path,
            )?,
        }))
    }
}

fn reject_unsupported_document_contract_surfaces(
    document: &OpenApiDocument,
) -> Result<(), OpenApiError> {
    if document.as_object().contains_key("webhooks") {
        return Err(unsupported_compatibility_feature(
            &JsonPointer::root().child("webhooks"),
            "webhooks",
        ));
    }
    Ok(())
}

fn reject_unsupported_path_item_reference(
    path_item: &Value,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    if path_item
        .as_object()
        .is_some_and(|path_item| path_item.contains_key("$ref"))
    {
        return Err(unsupported_compatibility_feature(
            &pointer.child("$ref"),
            "path item references",
        ));
    }
    Ok(())
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
        style: ParameterStyle,
        explode: bool,
    },
    QueryParameter {
        style: ParameterStyle,
        explode: bool,
        allow_reserved: bool,
        allow_empty_value: bool,
    },
    Header {
        explode: bool,
    },
    CookieParameter {
        style: ParameterStyle,
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

    const fn default_style(self) -> ParameterStyle {
        match self {
            Self::Path | Self::Header => ParameterStyle::Simple,
            Self::Query | Self::Cookie => ParameterStyle::Form,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParameterStyle {
    Matrix,
    Label,
    Simple,
    Form,
    SpaceDelimited,
    PipeDelimited,
    DeepObject,
}

impl ParameterStyle {
    fn from_value(
        location: ParameterLocation,
        value: &str,
        pointer: &JsonPointer,
    ) -> Result<Self, OpenApiError> {
        let style = match value {
            "matrix" => Self::Matrix,
            "label" => Self::Label,
            "simple" => Self::Simple,
            "form" => Self::Form,
            "spaceDelimited" => Self::SpaceDelimited,
            "pipeDelimited" => Self::PipeDelimited,
            "deepObject" => Self::DeepObject,
            _ => {
                return Err(invalid_value(
                    pointer,
                    "a supported OpenAPI parameter style",
                ));
            }
        };

        if style.supports(location) {
            Ok(style)
        } else {
            Err(invalid_value(
                pointer,
                match location {
                    ParameterLocation::Path => "'matrix', 'label', or 'simple' for path parameters",
                    ParameterLocation::Query => {
                        "'form', 'spaceDelimited', 'pipeDelimited', or 'deepObject' for query parameters"
                    }
                    ParameterLocation::Header => "'simple' for header parameters",
                    ParameterLocation::Cookie => "'form' for cookie parameters",
                },
            ))
        }
    }

    const fn supports(self, location: ParameterLocation) -> bool {
        matches!(
            (self, location),
            (
                Self::Matrix | Self::Label | Self::Simple,
                ParameterLocation::Path
            ) | (Self::Simple, ParameterLocation::Header)
                | (
                    Self::Form | Self::SpaceDelimited | Self::PipeDelimited | Self::DeepObject,
                    ParameterLocation::Query
                )
                | (Self::Form, ParameterLocation::Cookie)
        )
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Matrix => "matrix",
            Self::Label => "label",
            Self::Simple => "simple",
            Self::Form => "form",
            Self::SpaceDelimited => "spaceDelimited",
            Self::PipeDelimited => "pipeDelimited",
            Self::DeepObject => "deepObject",
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

pub fn lower_operations(
    document: &OpenApiDocument,
) -> Result<BTreeMap<OperationKey, LoweredOperation>, OpenApiLoweringError> {
    let lowerer = OpenApiOperationLowerer::new(document)?;
    let mut operations = BTreeMap::new();
    for key in lowerer.operation_keys()? {
        let operation = lowerer
            .lower_operation(&key)?
            .expect("operation keys must resolve back to operations");
        operations.insert(key, operation);
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
    path_template_names: &BTreeSet<String>,
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
        let Some(parameter) = parse_parameter(resolver, raw_parameter, &parameter_pointer)? else {
            continue;
        };
        if parameter.location == ParameterLocation::Path
            && !path_template_names.contains(&parameter.name)
        {
            return Err(invalid_value(
                &parameter_pointer.child("name"),
                "a template expression that appears in the path key",
            ));
        }
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

fn path_template_names(
    path: &str,
    pointer: &JsonPointer,
) -> Result<BTreeSet<String>, OpenApiError> {
    let mut names = BTreeSet::new();
    let mut rest = path;
    while let Some(open_index) = rest.find('{') {
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
        names.insert(name.to_owned());
        rest = &after_open[close_index + 1..];
    }
    if rest.contains('}') {
        return Err(invalid_value(
            pointer,
            "a path key with balanced non-empty template expressions",
        ));
    }
    Ok(names)
}

fn require_path_template_parameters(
    path_template_names: &BTreeSet<String>,
    parameters: &BTreeMap<(ParameterLocation, String), Parameter>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    for template_name in path_template_names {
        let identity = (ParameterLocation::Path, template_name.clone());
        if !parameters.contains_key(&identity) {
            return Err(invalid_value(
                pointer,
                "path parameters covering every template expression in the path key",
            ));
        }
    }
    Ok(())
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
) -> Result<Option<Parameter>, OpenApiError> {
    let raw = resolver.resolve_value(raw, pointer)?;
    let object = raw
        .as_object()
        .ok_or_else(|| invalid_value(pointer, "an object or local reference"))?;
    validate_parameter_fields(object, pointer)?;
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
    let ignored_header = is_ignored_header_parameter(location, &name);
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
    reject_content_serialization_fields(
        object,
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
    let value = lower_field_value(
        resolver,
        object.get("schema"),
        object.get("content"),
        pointer,
        |schema| parameter_schema_value(location, object, pointer, schema),
    )?;

    if ignored_header {
        return Ok(None);
    }

    Ok(Some(Parameter {
        name,
        location,
        required,
        value,
    }))
}

fn validate_parameter_fields(
    parameter: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    for field in parameter.keys() {
        if field.starts_with("x-")
            || matches!(
                field.as_str(),
                "name"
                    | "in"
                    | "required"
                    | "schema"
                    | "content"
                    | "style"
                    | "explode"
                    | "allowReserved"
                    | "allowEmptyValue"
                    | "description"
                    | "deprecated"
                    | "example"
                    | "examples"
            )
        {
            continue;
        }

        return Err(invalid_value(
            &pointer.child(field),
            "a supported OpenAPI parameter field or specification extension beginning with 'x-'",
        ));
    }

    validate_optional_string_field(parameter, "description", pointer)?;
    validate_optional_bool_field(parameter, "deprecated", pointer)?;
    validate_example_metadata_fields(parameter, pointer)?;

    Ok(())
}

fn is_ignored_header_parameter(location: ParameterLocation, name: &str) -> bool {
    location == ParameterLocation::Header
        && matches!(
            name.to_ascii_lowercase().as_str(),
            "accept" | "content-type" | "authorization"
        )
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

fn validate_parameter_schema_serialization_fields(
    location: ParameterLocation,
    object: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let style = parse_optional_string(object, "style", pointer)?
        .map(|style| ParameterStyle::from_value(location, &style, &pointer.child("style")))
        .transpose()?;
    let explode = parse_optional_bool(object, "explode", pointer)?;
    if let Some(style) = style {
        validate_deep_object_explode(style, explode, pointer)?;
    }
    parse_optional_bool(object, "allowReserved", pointer)?;
    parse_optional_bool(object, "allowEmptyValue", pointer)?;
    Ok(())
}

fn parameter_schema_value(
    location: ParameterLocation,
    object: &Map<String, Value>,
    pointer: &JsonPointer,
    schema: Value,
) -> Result<FieldValue, OpenApiError> {
    let style = match parse_optional_string(object, "style", pointer)? {
        Some(style) => ParameterStyle::from_value(location, &style, &pointer.child("style"))?,
        None => location.default_style(),
    };
    let explode = parse_optional_bool(object, "explode", pointer)?;
    validate_deep_object_explode(style, explode, pointer)?;
    let explode = explode.unwrap_or(style == ParameterStyle::Form);
    let serialization = match location {
        ParameterLocation::Path => SchemaSerialization::PathParameter { style, explode },
        ParameterLocation::Query => SchemaSerialization::QueryParameter {
            style,
            explode,
            allow_reserved: parse_optional_bool(object, "allowReserved", pointer)?.unwrap_or(false),
            allow_empty_value: parse_optional_bool(object, "allowEmptyValue", pointer)?
                .unwrap_or(false),
        },
        ParameterLocation::Header => SchemaSerialization::Header { explode },
        ParameterLocation::Cookie => SchemaSerialization::CookieParameter { style, explode },
    };
    Ok(FieldValue::Schema {
        schema,
        serialization,
    })
}

fn validate_deep_object_explode(
    style: ParameterStyle,
    explode: Option<bool>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    if style == ParameterStyle::DeepObject && explode != Some(true) {
        return Err(invalid_value(
            &pointer.child("explode"),
            "true when query parameter style is 'deepObject'",
        ));
    }
    Ok(())
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

fn reject_content_serialization_fields(
    object: &Map<String, Value>,
    pointer: &JsonPointer,
    fields: &[&str],
) -> Result<(), OpenApiError> {
    if !object.contains_key("content") {
        return Ok(());
    }
    for field in fields {
        if object.contains_key(*field) {
            return Err(invalid_value(
                &pointer.child(*field),
                "absent when `content` is present",
            ));
        }
    }
    Ok(())
}

fn validate_example_metadata_fields(
    object: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    if object.contains_key("example") && object.contains_key("examples") {
        return Err(invalid_value(
            &pointer.child("examples"),
            "absent when `example` is present",
        ));
    }
    let Some(examples) = object.get("examples") else {
        return Ok(());
    };
    let examples_pointer = pointer.child("examples");
    let examples = examples
        .as_object()
        .ok_or_else(|| invalid_value(&examples_pointer, "an object"))?;
    for (name, example) in examples {
        validate_example_or_reference_object(example, &examples_pointer.child(name))?;
    }
    Ok(())
}

fn validate_example_or_reference_object(
    example: &Value,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let example = example
        .as_object()
        .ok_or_else(|| invalid_value(pointer, "an OpenAPI Example Object or Reference Object"))?;

    if let Some(reference) = example.get("$ref") {
        if !reference.is_string() {
            return Err(invalid_value(&pointer.child("$ref"), "a string"));
        }
        return validate_reference_object_fields(example, pointer);
    }

    validate_object_fields(
        example,
        pointer,
        &["summary", "description", "value", "externalValue"],
        "a supported OpenAPI example field or specification extension beginning with 'x-'",
    )?;
    validate_optional_string_field(example, "summary", pointer)?;
    validate_optional_string_field(example, "description", pointer)?;
    validate_optional_uri_reference_field(example, "externalValue", pointer)?;
    if example.contains_key("value") && example.contains_key("externalValue") {
        return Err(invalid_value(
            pointer,
            "at most one of `value` or `externalValue`",
        ));
    }
    Ok(())
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
            add_enum_property(
                properties,
                required,
                "style",
                Value::String(style.as_str().to_owned()),
            );
            add_enum_property(properties, required, "explode", Value::Bool(*explode));
        }
        SchemaSerialization::QueryParameter {
            style,
            explode,
            allow_reserved,
            allow_empty_value,
        } => {
            add_enum_property(
                properties,
                required,
                "style",
                Value::String(style.as_str().to_owned()),
            );
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
    validate_request_body_fields(object, pointer)?;
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
    let variants = lower_content_variants(
        resolver,
        content,
        &pointer.child("content"),
        MediaTypeKeyKind::ConcreteOrRange,
    )?;
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

fn validate_request_body_fields(
    body: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    for field in body.keys() {
        if field.starts_with("x-")
            || matches!(field.as_str(), "required" | "content" | "description")
        {
            continue;
        }

        return Err(invalid_value(
            &pointer.child(field),
            "a supported OpenAPI request-body field or specification extension beginning with 'x-'",
        ));
    }

    validate_optional_string_field(body, "description", pointer)?;

    Ok(())
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
    let explicit_status_codes = explicit_response_status_codes(responses);
    let ranged_status_classes = ranged_response_status_classes(responses);
    let mut variants = Vec::new();
    for (status, raw_response) in responses {
        if status.starts_with("x-") {
            continue;
        }
        let response_pointer = responses_pointer.child(status);
        validate_response_status_selector(status, &response_pointer)?;
        let statuses =
            lowered_response_statuses(status, &explicit_status_codes, &ranged_status_classes)
                .expect("document validation already rejects invalid response status selectors");
        let raw_response = resolver.resolve_value(raw_response, &response_pointer)?;
        let response = raw_response
            .as_object()
            .ok_or_else(|| invalid_value(&response_pointer, "an object or local reference"))?;
        reject_unsupported_response_fields(response, &response_pointer)?;
        response
            .get("description")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid_value(&response_pointer.child("description"), "a string"))?;
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
                "status": { "enum": statuses },
                "body": body,
                "headers": headers
            },
            "required": ["status", "body", "headers"],
            "additionalProperties": false
        }));
    }
    if variants.is_empty() {
        return Err(invalid_value(
            &responses_pointer,
            "an object containing at least one response",
        ));
    }

    attach_schema_defs(resolver, any_of(variants))
}

fn validate_response_document_fields(
    response: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    for field in response.keys() {
        if field.starts_with("x-")
            || matches!(field.as_str(), "description" | "headers" | "content")
        {
            continue;
        }

        let field_pointer = pointer.child(field);
        if field == "links" {
            validate_response_links_field(
                response
                    .get(field)
                    .expect("field is present while iterating response keys"),
                &field_pointer,
            )?;
            continue;
        }
        return Err(invalid_value(
            &field_pointer,
            "a supported OpenAPI response field or specification extension beginning with 'x-'",
        ));
    }

    Ok(())
}

fn reject_unsupported_response_fields(
    response: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    validate_response_document_fields(response, pointer)?;
    if response.contains_key("links") {
        return Err(unsupported_compatibility_feature(
            &pointer.child("links"),
            "response links",
        ));
    }
    Ok(())
}

fn validate_response_links_field(links: &Value, pointer: &JsonPointer) -> Result<(), OpenApiError> {
    let links = links
        .as_object()
        .ok_or_else(|| invalid_value(pointer, "an object"))?;
    validate_component_collection_names(links, pointer)?;
    for (name, link) in links {
        validate_link_object_or_reference(link, &pointer.child(name))?;
    }
    Ok(())
}

fn validate_link_object_or_reference(
    link: &Value,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let link = link
        .as_object()
        .ok_or_else(|| invalid_value(pointer, "an OpenAPI Link Object or Reference Object"))?;
    if let Some(reference) = link.get("$ref") {
        if !reference.is_string() {
            return Err(invalid_value(&pointer.child("$ref"), "a string"));
        }
        return validate_reference_object_fields(link, pointer);
    }

    validate_object_fields(
        link,
        pointer,
        &[
            "operationRef",
            "operationId",
            "parameters",
            "requestBody",
            "description",
            "server",
        ],
        "a supported OpenAPI link field or specification extension beginning with 'x-'",
    )?;
    validate_optional_uri_reference_field(link, "operationRef", pointer)?;
    validate_optional_string_field(link, "operationId", pointer)?;
    validate_optional_string_field(link, "description", pointer)?;
    validate_optional_object_field(link, "parameters", pointer)?;
    if let Some(server) = link.get("server") {
        validate_server_object(server, &pointer.child("server"))?;
    }
    match (
        link.contains_key("operationRef"),
        link.contains_key("operationId"),
    ) {
        (true, false) | (false, true) => Ok(()),
        _ => Err(invalid_value(
            pointer,
            "exactly one of `operationRef` or `operationId`",
        )),
    }
}

fn validate_response_headers_document_shapes(
    headers: &Value,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let headers = headers
        .as_object()
        .ok_or_else(|| invalid_value(pointer, "an object"))?;
    let mut canonical_names = BTreeSet::new();
    for (name, header) in headers {
        if !name.eq_ignore_ascii_case("content-type") {
            let canonical_name = name.to_ascii_lowercase();
            if !canonical_names.insert(canonical_name.clone()) {
                return Err(OpenApiError::DuplicateResponseHeader {
                    pointer: pointer.child(name).render(),
                    name: canonical_name,
                });
            }
        }
        validate_header_document_shape(header, &pointer.child(name))?;
    }
    Ok(())
}

fn validate_header_document_shape(
    header: &Value,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let header = header
        .as_object()
        .ok_or_else(|| invalid_value(pointer, "an object or local reference"))?;
    if let Some(reference) = header.get("$ref") {
        if !reference.is_string() {
            return Err(invalid_value(&pointer.child("$ref"), "a string"));
        }
        return validate_reference_object_fields(header, pointer);
    }

    validate_header_fields(header, pointer)?;
    validate_optional_bool_field(header, "required", pointer)?;
    validate_optional_string_field(header, "style", pointer)?;
    validate_optional_bool_field(header, "explode", pointer)?;
    validate_optional_bool_field(header, "allowReserved", pointer)?;
    validate_optional_bool_field(header, "allowEmptyValue", pointer)?;
    reject_response_header_query_only_fields(header, pointer)?;
    match (header.get("schema"), header.get("content")) {
        (Some(schema), None) => {
            validate_schema_document_shape(schema, &pointer.child("schema"))?;
            validate_response_header_schema_serialization_fields(header, pointer)
        }
        (None, Some(content)) => {
            reject_content_serialization_fields(
                header,
                pointer,
                &["style", "explode", "example", "examples"],
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

fn validate_content_document_shapes(
    content: &Value,
    pointer: &JsonPointer,
    encoding_context: MediaTypeEncodingContext,
) -> Result<(), OpenApiError> {
    let content = content
        .as_object()
        .ok_or_else(|| invalid_value(pointer, "an object"))?;
    for (media_type, media) in content {
        let selector = MediaTypeSelector::parse(
            media_type,
            &pointer.child(media_type),
            MediaTypeKeyKind::ConcreteOrRange,
        )?;
        validate_media_type_document_shape(
            media,
            &pointer.child(media_type),
            &selector,
            encoding_context,
        )?;
    }
    Ok(())
}

fn validate_single_content_document_shape(
    content: &Value,
    pointer: &JsonPointer,
    encoding_context: MediaTypeEncodingContext,
) -> Result<(), OpenApiError> {
    let content = content
        .as_object()
        .ok_or_else(|| invalid_value(pointer, "an object containing exactly one media type"))?;
    if content.len() != 1 {
        return Err(invalid_value(
            pointer,
            "an object containing exactly one media type",
        ));
    }
    for (media_type, media) in content {
        let selector = MediaTypeSelector::parse(
            media_type,
            &pointer.child(media_type),
            MediaTypeKeyKind::ConcreteOnly,
        )?;
        validate_media_type_document_shape(
            media,
            &pointer.child(media_type),
            &selector,
            encoding_context,
        )?;
    }
    Ok(())
}

fn validate_media_type_document_shape(
    media: &Value,
    pointer: &JsonPointer,
    selector: &MediaTypeSelector,
    encoding_context: MediaTypeEncodingContext,
) -> Result<(), OpenApiError> {
    let media = media
        .as_object()
        .ok_or_else(|| invalid_value(pointer, "an object"))?;
    validate_media_type_fields(media, pointer)?;
    if let Some(schema) = media.get("schema") {
        validate_schema_document_shape(schema, &pointer.child("schema"))?;
    }
    if let Some(encoding) = media.get("encoding") {
        encoding_context.validate_encoding(selector, &pointer.child("encoding"))?;
        validate_media_type_encoding_document_shapes(encoding, &pointer.child("encoding"))?;
        validate_encoding_keys_against_inline_properties(media, pointer)?;
    }
    Ok(())
}

fn validate_encoding_keys_against_inline_properties(
    media: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let Some(encoding) = media.get("encoding").and_then(Value::as_object) else {
        return Ok(());
    };
    let Some(properties) = media
        .get("schema")
        .and_then(Value::as_object)
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

fn validate_media_type_encoding_document_shapes(
    encoding: &Value,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let encoding = encoding
        .as_object()
        .ok_or_else(|| invalid_value(pointer, "an object"))?;
    for (name, raw_encoding) in encoding {
        validate_encoding_object_document_shape(raw_encoding, &pointer.child(name))?;
    }
    Ok(())
}

fn validate_encoding_object_document_shape(
    encoding: &Value,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let encoding = encoding
        .as_object()
        .ok_or_else(|| invalid_value(pointer, "an OpenAPI Encoding Object"))?;
    validate_object_fields(
        encoding,
        pointer,
        &[
            "contentType",
            "headers",
            "style",
            "explode",
            "allowReserved",
        ],
        "a supported OpenAPI encoding field or specification extension beginning with 'x-'",
    )?;
    validate_optional_encoding_content_type_field(encoding, pointer)?;
    validate_optional_encoding_style_field(encoding, pointer)?;
    validate_optional_bool_field(encoding, "explode", pointer)?;
    validate_optional_bool_field(encoding, "allowReserved", pointer)?;
    if let Some(headers) = encoding.get("headers") {
        validate_response_headers_document_shapes(headers, &pointer.child("headers"))?;
    }
    Ok(())
}

fn validate_optional_encoding_content_type_field(
    encoding: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let Some(content_type) = encoding.get("contentType") else {
        return Ok(());
    };
    let content_type_pointer = pointer.child("contentType");
    let Some(content_type) = content_type.as_str() else {
        return Err(invalid_value(
            &content_type_pointer,
            "a comma-separated list of valid concrete media types or OpenAPI media-type ranges",
        ));
    };

    for media_type in content_type.split(',').map(str::trim) {
        if media_type.is_empty()
            || MediaTypeSelector::parse(
                media_type,
                &content_type_pointer,
                MediaTypeKeyKind::ConcreteOrRange,
            )
            .is_err()
        {
            return Err(invalid_value(
                &content_type_pointer,
                "a comma-separated list of valid concrete media types or OpenAPI media-type ranges",
            ));
        }
    }

    Ok(())
}

fn validate_optional_encoding_style_field(
    encoding: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let Some(style) = encoding.get("style") else {
        return Ok(());
    };
    let style_pointer = pointer.child("style");
    let Some(style) = style.as_str() else {
        return Err(invalid_value(
            &style_pointer,
            "one of 'form', 'spaceDelimited', 'pipeDelimited', or 'deepObject'",
        ));
    };

    if matches!(
        style,
        "form" | "spaceDelimited" | "pipeDelimited" | "deepObject"
    ) {
        Ok(())
    } else {
        Err(invalid_value(
            &style_pointer,
            "one of 'form', 'spaceDelimited', 'pipeDelimited', or 'deepObject'",
        ))
    }
}

fn explicit_response_status_codes(responses: &Map<String, Value>) -> BTreeSet<u16> {
    responses
        .keys()
        .filter_map(|status| parse_explicit_response_status(status))
        .collect()
}

fn ranged_response_status_classes(responses: &Map<String, Value>) -> BTreeSet<u16> {
    responses
        .keys()
        .filter_map(|status| parse_response_status_range(status))
        .collect()
}

fn lowered_response_statuses(
    status: &str,
    explicit_status_codes: &BTreeSet<u16>,
    ranged_status_classes: &BTreeSet<u16>,
) -> Option<Vec<Value>> {
    if status == "default" {
        return Some(
            standard_http_status_codes()
                .filter(|code| {
                    !explicit_status_codes.contains(code)
                        && !ranged_status_classes.contains(&(code / 100))
                })
                .map(response_status_value)
                .collect(),
        );
    }

    if let Some(status_class) = parse_response_status_range(status) {
        return Some(
            standard_http_status_codes()
                .filter(|code| code / 100 == status_class && !explicit_status_codes.contains(code))
                .map(response_status_value)
                .collect(),
        );
    }

    parse_explicit_response_status(status).map(|code| vec![response_status_value(code)])
}

fn standard_http_status_codes() -> impl Iterator<Item = u16> {
    100..=599
}

fn is_standard_http_status_code(code: u16) -> bool {
    (100..=599).contains(&code)
}

fn response_status_value(code: u16) -> Value {
    Value::String(format!("{code:03}"))
}

fn parse_explicit_response_status(status: &str) -> Option<u16> {
    if status.len() != 3 || !status.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let code = status.parse::<u16>().ok()?;
    is_standard_http_status_code(code).then_some(code)
}

fn parse_response_status_range(status: &str) -> Option<u16> {
    match status {
        "1XX" => Some(1),
        "2XX" => Some(2),
        "3XX" => Some(3),
        "4XX" => Some(4),
        "5XX" => Some(5),
        _ => None,
    }
}

fn validate_response_status_selector(
    status: &str,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    if status == "default"
        || parse_response_status_range(status).is_some()
        || parse_explicit_response_status(status).is_some()
    {
        return Ok(());
    }

    Err(invalid_value(
        pointer,
        "a response status code from `100` through `599`, one of `1XX` through `5XX`, or `default`",
    ))
}

fn lower_response_body(
    resolver: &Resolver<'_>,
    content: Option<&Value>,
    pointer: &JsonPointer,
) -> Result<Value, OpenApiError> {
    let Some(content) = content else {
        return Ok(json!({ "type": "null" }));
    };
    Ok(any_of(lower_content_variants(
        resolver,
        content,
        pointer,
        MediaTypeKeyKind::ConcreteOrRange,
    )?))
}

fn lower_content_variants(
    resolver: &Resolver<'_>,
    content: &Value,
    pointer: &JsonPointer,
    media_type_key_kind: MediaTypeKeyKind,
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
    let media_entries = content
        .iter()
        .map(|(media_type, raw_media)| {
            let media_pointer = pointer.child(media_type);
            MediaTypeSelector::parse(media_type, &media_pointer, media_type_key_kind)
                .map(|selector| (media_type, raw_media, selector))
        })
        .collect::<Result<Vec<_>, _>>()?;
    reject_duplicate_normalized_media_types(&media_entries, pointer)?;
    let mut variants = Vec::with_capacity(content.len());
    for (media_type, raw_media, media_type_selector) in &media_entries {
        let media_pointer = pointer.child(*media_type);
        let media_type_schema = media_type_selector.contract_schema(
            media_entries
                .iter()
                .map(|(_, _, selector)| selector)
                .filter(|candidate| candidate.is_more_specific_than(media_type_selector)),
        );
        let media = raw_media
            .as_object()
            .ok_or_else(|| invalid_value(&media_pointer, "an object"))?;
        validate_media_type_fields(media, &media_pointer)?;
        if let Some(encoding) = media.get("encoding") {
            validate_media_type_encoding_field(
                resolver,
                media,
                encoding,
                &media_pointer,
                &media_pointer.child("encoding"),
            )?;
            return Err(unsupported_compatibility_feature(
                &media_pointer.child("encoding"),
                "media-type encoding",
            ));
        }
        let schema = media
            .get("schema")
            .map(|schema| rewrite_schema_refs_for_lowering(schema, &media_pointer.child("schema")))
            .transpose()?
            .unwrap_or(Value::Bool(true));
        variants.push(json!({
            "type": "object",
            "properties": {
                "content_type": media_type_schema,
                "value": schema
            },
            "required": ["content_type", "value"],
            "additionalProperties": false
        }));
    }
    Ok(variants)
}

fn reject_duplicate_normalized_media_types(
    media_entries: &[(&String, &Value, MediaTypeSelector)],
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let mut selectors = BTreeMap::new();
    for (media_type, _, selector) in media_entries {
        let key = (selector.media_type.clone(), selector.media_subtype.clone());
        if let Some(previous) = selectors.insert(key, (*media_type).clone()) {
            return Err(OpenApiError::DuplicateNormalizedMediaType {
                pointer: pointer.child(*media_type).render(),
                previous,
                current: (*media_type).clone(),
                selector: selector.as_str(),
            });
        }
    }
    Ok(())
}

#[derive(Clone)]
struct MediaTypeSelector {
    media_type: String,
    media_subtype: String,
}

#[derive(Clone, Copy)]
enum MediaTypeKeyKind {
    ConcreteOnly,
    ConcreteOrRange,
}

#[derive(Clone, Copy)]
enum MediaTypeEncodingContext {
    RequestBody,
    NonRequestBody,
}

impl MediaTypeSelector {
    fn parse(
        media_type: &str,
        pointer: &JsonPointer,
        key_kind: MediaTypeKeyKind,
    ) -> Result<Self, OpenApiError> {
        let parsed = media_type
            .parse::<Mime>()
            .map_err(|_| invalid_value(pointer, key_kind.expected_description()))?;
        let media_type_kind = parsed.type_().as_str();
        let media_subtype_kind = parsed.subtype().as_str();
        if media_type_kind == "*" && media_subtype_kind != "*" {
            return Err(invalid_value(pointer, key_kind.expected_description()));
        }
        if matches!(key_kind, MediaTypeKeyKind::ConcreteOnly)
            && (media_type_kind == "*" || media_subtype_kind == "*")
        {
            return Err(invalid_value(pointer, key_kind.expected_description()));
        }

        Ok(Self {
            media_type: media_type_kind.to_owned(),
            media_subtype: media_subtype_kind.to_owned(),
        })
    }

    fn contract_schema<'a>(&self, more_specific: impl Iterator<Item = &'a Self>) -> Value {
        let base = self.base_contract_schema();
        let exclusions = more_specific
            .map(Self::base_contract_schema)
            .map(|schema| json!({ "not": schema }))
            .collect::<Vec<_>>();
        if exclusions.is_empty() {
            return base;
        }

        json!({
            "allOf": std::iter::once(base).chain(exclusions).collect::<Vec<_>>()
        })
    }

    fn base_contract_schema(&self) -> Value {
        json!({
        "type": "object",
        "properties": {
            "type": media_type_component_schema(&self.media_type),
            "subtype": media_type_component_schema(&self.media_subtype)
        },
        "required": ["type", "subtype"],
        "additionalProperties": false
        })
    }

    fn as_str(&self) -> String {
        format!("{}/{}", self.media_type, self.media_subtype)
    }

    fn supports_request_body_encoding(&self) -> bool {
        self.media_type == "multipart"
            || (self.media_type == "application" && self.media_subtype == "x-www-form-urlencoded")
    }

    fn is_more_specific_than(&self, other: &Self) -> bool {
        match (
            self.media_type.as_str(),
            self.media_subtype.as_str(),
            other.media_type.as_str(),
            other.media_subtype.as_str(),
        ) {
            (_, _, "*", "*") => !(self.media_type == "*" && self.media_subtype == "*"),
            (media_type, subtype, other_type, "*") => media_type == other_type && subtype != "*",
            _ => false,
        }
    }
}

impl MediaTypeEncodingContext {
    fn validate_encoding(
        self,
        selector: &MediaTypeSelector,
        pointer: &JsonPointer,
    ) -> Result<(), OpenApiError> {
        if matches!(self, Self::RequestBody) && selector.supports_request_body_encoding() {
            return Ok(());
        }

        Err(invalid_value(
            pointer,
            "request-body content with media type `multipart/*` or `application/x-www-form-urlencoded`",
        ))
    }
}

impl MediaTypeKeyKind {
    const fn expected_description(self) -> &'static str {
        match self {
            Self::ConcreteOnly => "a valid concrete media type",
            Self::ConcreteOrRange => "a valid concrete media type or OpenAPI media-type range",
        }
    }
}

fn media_type_component_schema(component: &str) -> Value {
    if component == "*" {
        json!({ "type": "string" })
    } else {
        json!({ "enum": [component] })
    }
}

fn validate_media_type_fields(
    media: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    for field in media.keys() {
        if field.starts_with("x-")
            || matches!(
                field.as_str(),
                "schema" | "encoding" | "example" | "examples"
            )
        {
            continue;
        }

        return Err(invalid_value(
            &pointer.child(field),
            "a supported OpenAPI media-type field or specification extension beginning with 'x-'",
        ));
    }

    validate_example_metadata_fields(media, pointer)?;

    Ok(())
}

fn validate_media_type_encoding_field(
    resolver: &Resolver<'_>,
    media: &Map<String, Value>,
    encoding: &Value,
    media_pointer: &JsonPointer,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    validate_media_type_encoding_document_shapes(encoding, pointer)?;
    validate_encoding_keys_against_supported_media_schema_refs(
        resolver,
        media,
        media_pointer,
        pointer,
    )?;
    let encoding = encoding
        .as_object()
        .expect("document-shape validation above requires encoding objects");
    for (name, raw_encoding) in encoding {
        validate_encoding_object(resolver, raw_encoding, &pointer.child(name))?;
    }
    Ok(())
}

fn validate_encoding_keys_against_supported_media_schema_refs(
    resolver: &Resolver<'_>,
    media: &Map<String, Value>,
    media_pointer: &JsonPointer,
    encoding_pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let Some(encoding) = media.get("encoding").and_then(Value::as_object) else {
        return Ok(());
    };
    let Some(schema) = media.get("schema") else {
        return Ok(());
    };
    let schema_pointer = media_pointer.child("schema");
    let Some(properties) =
        directly_declared_media_schema_properties(resolver, schema, &schema_pointer)?
    else {
        return Ok(());
    };

    for name in encoding.keys() {
        if !properties.contains(name) {
            return Err(invalid_value(
                &encoding_pointer.child(name),
                "a property declared by the media type schema",
            ));
        }
    }

    Ok(())
}

fn directly_declared_media_schema_properties(
    resolver: &Resolver<'_>,
    schema: &Value,
    pointer: &JsonPointer,
) -> Result<Option<BTreeSet<String>>, OpenApiError> {
    let Some(schema_object) = schema.as_object() else {
        return Ok(None);
    };
    if let Some(properties) = schema_object.get("properties").and_then(Value::as_object) {
        return Ok(Some(properties.keys().cloned().collect()));
    }
    if schema_object.len() == 1 && schema_object.get("$ref").and_then(Value::as_str).is_some() {
        let resolved = resolver.resolve_value(schema, pointer)?;
        let Some(resolved) = resolved.as_object() else {
            return Ok(None);
        };
        if let Some(properties) = resolved.get("properties").and_then(Value::as_object) {
            return Ok(Some(properties.keys().cloned().collect()));
        }
    }
    Ok(None)
}

fn validate_encoding_object(
    resolver: &Resolver<'_>,
    encoding: &Value,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    validate_encoding_object_document_shape(encoding, pointer)?;
    let encoding = encoding
        .as_object()
        .expect("document-shape validation above requires encoding objects");
    let Some(headers) = encoding.get("headers") else {
        return Ok(());
    };
    let headers_pointer = pointer.child("headers");
    let headers = headers
        .as_object()
        .ok_or_else(|| invalid_value(&headers_pointer, "an object"))?;
    for (name, raw_header) in headers {
        lower_response_header_field(resolver, name, raw_header, &headers_pointer.child(name))?;
    }
    Ok(())
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
    let mut canonical_names = BTreeSet::new();
    for (name, raw_header) in headers {
        if name.eq_ignore_ascii_case("content-type") {
            continue;
        }
        let canonical_name = name.to_ascii_lowercase();
        if !canonical_names.insert(canonical_name.clone()) {
            return Err(OpenApiError::DuplicateResponseHeader {
                pointer: pointer.child(name).render(),
                name: canonical_name,
            });
        }
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
    reject_response_header_query_only_fields(header, pointer)?;
    validate_header_fields(header, pointer)?;
    reject_content_serialization_fields(
        header,
        pointer,
        &["style", "explode", "example", "examples"],
    )?;
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

fn reject_response_header_query_only_fields(
    header: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    for field in ["allowReserved", "allowEmptyValue"] {
        if header.contains_key(field) {
            return Err(invalid_value(
                &pointer.child(field),
                "not present for response headers",
            ));
        }
    }
    Ok(())
}

fn validate_header_fields(
    header: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    for field in header.keys() {
        if field.starts_with("x-")
            || matches!(
                field.as_str(),
                "required"
                    | "schema"
                    | "content"
                    | "style"
                    | "explode"
                    | "allowReserved"
                    | "allowEmptyValue"
                    | "description"
                    | "deprecated"
                    | "example"
                    | "examples"
            )
        {
            continue;
        }

        return Err(invalid_value(
            &pointer.child(field),
            "a supported OpenAPI response-header field or specification extension beginning with 'x-'",
        ));
    }

    validate_optional_string_field(header, "description", pointer)?;
    validate_optional_bool_field(header, "deprecated", pointer)?;
    validate_example_metadata_fields(header, pointer)?;

    Ok(())
}

fn header_schema_value(
    header: &Map<String, Value>,
    pointer: &JsonPointer,
    schema: Value,
) -> Result<FieldValue, OpenApiError> {
    validate_response_header_schema_serialization_fields(header, pointer)?;
    Ok(FieldValue::Schema {
        schema,
        serialization: SchemaSerialization::Header {
            explode: parse_optional_bool(header, "explode", pointer)?.unwrap_or(false),
        },
    })
}

fn validate_response_header_schema_serialization_fields(
    header: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let style =
        parse_optional_string(header, "style", pointer)?.unwrap_or_else(|| "simple".to_owned());
    if style != "simple" {
        return Err(invalid_value(
            &pointer.child("style"),
            "'simple' for response headers",
        ));
    }
    parse_optional_bool(header, "explode", pointer)?;
    Ok(())
}

fn lower_field_value(
    resolver: &Resolver<'_>,
    schema: Option<&Value>,
    content: Option<&Value>,
    pointer: &JsonPointer,
    schema_value: impl FnOnce(Value) -> Result<FieldValue, OpenApiError>,
) -> Result<FieldValue, OpenApiError> {
    match (schema, content) {
        (Some(schema), None) => schema_value(rewrite_schema_refs_for_lowering(
            schema,
            &pointer.child("schema"),
        )?),
        (None, Some(content)) => Ok(FieldValue::Content {
            media_schema: lower_single_content_variant(
                resolver,
                content,
                &pointer.child("content"),
            )?,
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

fn lower_single_content_variant(
    resolver: &Resolver<'_>,
    content: &Value,
    pointer: &JsonPointer,
) -> Result<Value, OpenApiError> {
    let content = content
        .as_object()
        .ok_or_else(|| invalid_value(pointer, "an object containing exactly one media type"))?;
    if content.len() != 1 {
        return Err(invalid_value(
            pointer,
            "an object containing exactly one media type",
        ));
    }
    Ok(any_of(lower_content_variants(
        resolver,
        &Value::Object(content.clone()),
        pointer,
        MediaTypeKeyKind::ConcreteOnly,
    )?))
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

enum ComponentSchemaValidation {
    Validated,
    Deferred,
}

enum ReferenceResolution<'a> {
    Value(&'a Value),
    ExternalReference { reference: String },
}

fn declared_security_scheme_names(document: &Map<String, Value>) -> BTreeSet<String> {
    document
        .get("components")
        .and_then(Value::as_object)
        .and_then(|components| components.get("securitySchemes"))
        .and_then(Value::as_object)
        .map(|schemes| schemes.keys().cloned().collect())
        .unwrap_or_default()
}

impl<'a> Resolver<'a> {
    fn new(document: &'a OpenApiDocument) -> Result<Self, OpenApiLoweringError> {
        let resolver = Self {
            document,
            component_schema_defs: load_component_schema_defs(document)?,
        };
        resolver.validate_supported_components()?;
        Ok(resolver)
    }

    fn resolve_value<'b>(
        &'b self,
        value: &'b Value,
        pointer: &JsonPointer,
    ) -> Result<&'b Value, OpenApiError> {
        match resolve_reference_chain(&self.document.raw, value, pointer)? {
            ReferenceResolution::Value(value) => Ok(value),
            ReferenceResolution::ExternalReference { reference } => {
                Err(OpenApiError::UnsupportedReference {
                    pointer: pointer.render(),
                    reference,
                })
            }
        }
    }

    fn component_schema_defs_for(
        &self,
        schema: &Value,
    ) -> Result<Map<String, Value>, OpenApiError> {
        Ok(component_schema_defs_for_schema(
            &self.component_schema_defs,
            schema,
        ))
    }

    fn validate_supported_components(&self) -> Result<(), OpenApiLoweringError> {
        let Some(components) = self.document.as_object().get("components") else {
            return Ok(());
        };
        let components_pointer = JsonPointer::root().child("components");
        let components = components
            .as_object()
            .ok_or_else(|| invalid_value(&components_pointer, "an object"))?;
        validate_component_fields(components, &components_pointer)?;

        self.validate_parameter_components(
            component_collection(components, "parameters", &components_pointer)?,
            &components_pointer.child("parameters"),
        )?;
        self.validate_request_body_components(
            component_collection(components, "requestBodies", &components_pointer)?,
            &components_pointer.child("requestBodies"),
        )?;
        self.validate_response_components(
            component_collection(components, "responses", &components_pointer)?,
            &components_pointer.child("responses"),
        )?;
        self.validate_header_components(
            component_collection(components, "headers", &components_pointer)?,
            &components_pointer.child("headers"),
        )?;
        self.validate_security_scheme_components(
            component_collection(components, "securitySchemes", &components_pointer)?,
            &components_pointer.child("securitySchemes"),
        )?;

        Ok(())
    }

    fn validate_parameter_components(
        &self,
        components: Option<&Map<String, Value>>,
        pointer: &JsonPointer,
    ) -> Result<(), OpenApiLoweringError> {
        let Some(components) = components else {
            return Ok(());
        };
        for (name, raw_parameter) in components {
            let component_pointer = pointer.child(name);
            let Some(parameter) = parse_parameter(self, raw_parameter, &component_pointer)? else {
                continue;
            };
            self.validate_schema_value(contract_field_schema(&ContractField::from(&parameter)))?;
        }
        Ok(())
    }

    fn validate_request_body_components(
        &self,
        components: Option<&Map<String, Value>>,
        pointer: &JsonPointer,
    ) -> Result<(), OpenApiLoweringError> {
        let Some(components) = components else {
            return Ok(());
        };
        for (name, raw_body) in components {
            let body_pointer = pointer.child(name);
            let body = lower_request_body(self, Some(raw_body), &body_pointer)?;
            self.validate_schema_value(body)?;
        }
        Ok(())
    }

    fn validate_response_components(
        &self,
        components: Option<&Map<String, Value>>,
        pointer: &JsonPointer,
    ) -> Result<(), OpenApiLoweringError> {
        let Some(components) = components else {
            return Ok(());
        };
        for (name, raw_response) in components {
            let response_pointer = pointer.child(name);
            let raw_response = self.resolve_value(raw_response, &response_pointer)?;
            let response = raw_response
                .as_object()
                .ok_or_else(|| invalid_value(&response_pointer, "an object or local reference"))?;
            reject_unsupported_response_fields(response, &response_pointer)?;
            response
                .get("description")
                .and_then(Value::as_str)
                .ok_or_else(|| invalid_value(&response_pointer.child("description"), "a string"))?;
            let body = lower_response_body(
                self,
                response.get("content"),
                &response_pointer.child("content"),
            )?;
            let headers = lower_response_headers(
                self,
                response.get("headers"),
                &response_pointer.child("headers"),
            )?;
            self.validate_schema_value(body)?;
            self.validate_schema_value(headers)?;
        }
        Ok(())
    }

    fn validate_header_components(
        &self,
        components: Option<&Map<String, Value>>,
        pointer: &JsonPointer,
    ) -> Result<(), OpenApiLoweringError> {
        let Some(components) = components else {
            return Ok(());
        };
        for (name, raw_header) in components {
            let component_pointer = pointer.child(name);
            let field = lower_response_header_field(self, name, raw_header, &component_pointer)?;
            self.validate_schema_value(contract_field_schema(&field))?;
        }
        Ok(())
    }

    fn validate_security_scheme_components(
        &self,
        components: Option<&Map<String, Value>>,
        pointer: &JsonPointer,
    ) -> Result<(), OpenApiLoweringError> {
        let Some(components) = components else {
            return Ok(());
        };
        for (name, raw_scheme) in components {
            let scheme_pointer = pointer.child(name);
            let scheme = self.resolve_value(raw_scheme, &scheme_pointer)?;
            validate_security_scheme_object(scheme, &scheme_pointer)?;
        }
        Ok(())
    }

    fn validate_schema_value(&self, schema: Value) -> Result<(), OpenApiLoweringError> {
        let schema = attach_schema_defs(
            self,
            json!({
                "type": "object",
                "properties": {
                    "value": schema
                },
                "required": ["value"],
                "additionalProperties": false
            }),
        )?;
        let schema = self.document.schema_document(schema)?;
        schema.root()?;
        schema.validate_source_schema()?;
        Ok(())
    }
}

fn component_schema_defs_for_schema(
    available_defs: &BTreeMap<String, ComponentSchemaDef>,
    schema: &Value,
) -> Map<String, Value> {
    let mut pending = collect_component_schema_names(schema);
    let mut visited = BTreeSet::new();
    let mut defs = Map::new();
    while let Some(name) = pending.pop() {
        if !visited.insert(name.clone()) {
            continue;
        }
        let Some(component) = available_defs.get(&name) else {
            continue;
        };
        pending.extend(component.dependencies.iter().cloned());
        defs.insert(name, component.schema.clone());
    }
    defs
}

fn resolve_reference_chain<'a>(
    document: &'a Value,
    value: &'a Value,
    pointer: &JsonPointer,
) -> Result<ReferenceResolution<'a>, OpenApiError> {
    let mut current = value;
    let mut visited_references = BTreeSet::new();
    loop {
        let Some(object) = current.as_object() else {
            return Ok(ReferenceResolution::Value(current));
        };
        let Some(reference) = object.get("$ref") else {
            return Ok(ReferenceResolution::Value(current));
        };
        let reference = reference
            .as_str()
            .ok_or_else(|| invalid_value(&pointer.child("$ref"), "a string"))?;
        validate_reference_object_fields(object, pointer)?;
        if reference != "#" && !reference.starts_with(SUPPORTED_REF_PREFIX) {
            return Ok(ReferenceResolution::ExternalReference {
                reference: reference.to_owned(),
            });
        }
        if !visited_references.insert(reference.to_owned()) {
            return Err(OpenApiError::CyclicReference {
                pointer: pointer.render(),
                reference: reference.to_owned(),
            });
        }
        current = lookup_pointer(document, reference).ok_or_else(|| {
            OpenApiError::UnresolvedReference {
                pointer: pointer.render(),
                reference: reference.to_owned(),
            }
        })?;
    }
}

fn component_collection<'a>(
    components: &'a Map<String, Value>,
    field: &str,
    pointer: &JsonPointer,
) -> Result<Option<&'a Map<String, Value>>, OpenApiError> {
    components
        .get(field)
        .map(|value| {
            value
                .as_object()
                .ok_or_else(|| invalid_value(&pointer.child(field), "an object"))
        })
        .transpose()
}

fn validate_component_collection_names(
    collection: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    for name in collection.keys() {
        if !is_valid_component_name(name) {
            return Err(invalid_value(
                &pointer.child(name),
                "a component name matching `^[a-zA-Z0-9._-]+$`",
            ));
        }
    }
    Ok(())
}

fn is_valid_component_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn validate_reference_object_fields(
    reference: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    validate_optional_uri_reference_field(reference, "$ref", pointer)?;
    for (field, value) in reference {
        match field.as_str() {
            "$ref" => {}
            "summary" | "description" if !value.is_string() => {
                return Err(invalid_value(&pointer.child(field), "a string"));
            }
            _ => {}
        }
    }
    Ok(())
}

fn load_component_schema_defs(
    document: &OpenApiDocument,
) -> Result<BTreeMap<String, ComponentSchemaDef>, OpenApiError> {
    load_component_schema_defs_with(document, rewrite_schema_refs_for_lowering)
}

fn load_component_schema_defs_for_validation(
    document: &OpenApiDocument,
) -> Result<BTreeMap<String, ComponentSchemaDef>, OpenApiError> {
    load_component_schema_defs_with(document, rewrite_schema_refs_for_validation)
}

fn load_component_schema_defs_with(
    document: &OpenApiDocument,
    rewrite: fn(&Value, &JsonPointer) -> Result<Value, OpenApiError>,
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
        let schema = rewrite(
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

            for keyword in SINGLE_SCHEMA_CHILD_KEYWORDS {
                if let Some(child) = object.get(keyword) {
                    collect_component_schema_names_into(child, names);
                }
            }

            for keyword in SCHEMA_MAP_CHILD_KEYWORDS {
                if let Some(children) = object.get(keyword).and_then(Value::as_object) {
                    for child in children.values() {
                        collect_component_schema_names_into(child, names);
                    }
                }
            }

            for keyword in SCHEMA_ARRAY_CHILD_KEYWORDS {
                if let Some(children) = object.get(keyword).and_then(Value::as_array) {
                    for child in children {
                        collect_component_schema_names_into(child, names);
                    }
                }
            }
        }
        Value::Array(_) => {}
        _ => {}
    }
}

fn component_schema_name_from_defs_reference(reference: &str) -> Option<String> {
    let component_path = reference.strip_prefix("#/$defs/")?;
    let encoded_name = component_path.split('/').next()?;
    let decoded = percent_decode_str(encoded_name).decode_utf8().ok()?;
    Some(decoded.replace("~1", "/").replace("~0", "~"))
}

fn schema_uses_later_lowering_reference_features(schema: &Value) -> bool {
    match schema {
        Value::Object(object) => {
            if object.keys().any(|key| {
                matches!(
                    key.as_str(),
                    "$id" | "$anchor" | "$dynamicRef" | "$dynamicAnchor"
                )
            }) {
                return true;
            }
            if let Some(reference) = object.get("$ref").and_then(Value::as_str)
                && !reference.starts_with("#/$defs/")
            {
                return true;
            }

            for keyword in SINGLE_SCHEMA_CHILD_KEYWORDS {
                if object
                    .get(keyword)
                    .is_some_and(schema_uses_later_lowering_reference_features)
                {
                    return true;
                }
            }

            for keyword in SCHEMA_MAP_CHILD_KEYWORDS {
                if object
                    .get(keyword)
                    .and_then(Value::as_object)
                    .is_some_and(|children| {
                        children
                            .values()
                            .any(schema_uses_later_lowering_reference_features)
                    })
                {
                    return true;
                }
            }

            for keyword in SCHEMA_ARRAY_CHILD_KEYWORDS {
                if object
                    .get(keyword)
                    .and_then(Value::as_array)
                    .is_some_and(|children| {
                        children
                            .iter()
                            .any(schema_uses_later_lowering_reference_features)
                    })
                {
                    return true;
                }
            }

            false
        }
        Value::Array(_) | Value::Bool(_) | Value::Null | Value::Number(_) | Value::String(_) => {
            false
        }
    }
}

#[derive(Clone, Copy)]
enum DeferredReferenceValidation {
    Backend,
    ResolvedAst,
}

fn strip_deferred_schema_references_for_validation(
    schema: &Value,
    validation: DeferredReferenceValidation,
) -> Value {
    match schema {
        Value::Object(object) => {
            let mut stripped = Map::new();
            for (key, value) in object {
                let stripped_value = match key.as_str() {
                    "$ref" => {
                        if value
                            .as_str()
                            .is_some_and(|reference| reference.starts_with("#/$defs/"))
                        {
                            Some(value.clone())
                        } else {
                            None
                        }
                    }
                    "$id" | "$anchor" | "$dynamicRef" | "$dynamicAnchor" => match validation {
                        DeferredReferenceValidation::Backend => Some(value.clone()),
                        DeferredReferenceValidation::ResolvedAst => None,
                    },
                    key if is_single_schema_child_keyword(key) => Some(
                        strip_deferred_schema_references_for_validation(value, validation),
                    ),
                    key if is_schema_map_child_keyword(key) => {
                        Some(strip_schema_map_deferred_references(value, validation))
                    }
                    key if is_schema_array_child_keyword(key) => {
                        Some(strip_schema_array_deferred_references(value, validation))
                    }
                    _ => Some(value.clone()),
                };
                if let Some(stripped_value) = stripped_value {
                    stripped.insert(key.clone(), stripped_value);
                }
            }
            Value::Object(stripped)
        }
        _ => schema.clone(),
    }
}

fn strip_schema_map_deferred_references(
    value: &Value,
    validation: DeferredReferenceValidation,
) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .iter()
                .map(|(key, schema)| {
                    (
                        key.clone(),
                        strip_deferred_schema_references_for_validation(schema, validation),
                    )
                })
                .collect(),
        ),
        _ => value.clone(),
    }
}

fn strip_schema_array_deferred_references(
    value: &Value,
    validation: DeferredReferenceValidation,
) -> Value {
    match value {
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|schema| strip_deferred_schema_references_for_validation(schema, validation))
                .collect(),
        ),
        _ => value.clone(),
    }
}

#[derive(Clone, Copy)]
enum SchemaReferenceRewrite {
    Validation,
    Lowering,
}

impl SchemaReferenceRewrite {
    fn validate_schema_object(
        self,
        object: &Map<String, Value>,
        pointer: &JsonPointer,
    ) -> Result<(), OpenApiError> {
        if matches!(self, Self::Lowering) {
            reject_unsafe_number_bounds_in_schema_object(object, pointer)?;
        }
        validate_optional_external_docs_field(object, pointer)?;
        validate_optional_xml_field(object, pointer)
    }

    fn reject_keyword_if_needed(
        self,
        keyword: &str,
        pointer: &JsonPointer,
    ) -> Result<(), OpenApiError> {
        if !matches!(self, Self::Lowering) {
            return Ok(());
        }
        let Some(feature) = unsupported_lowering_schema_keyword_feature(keyword) else {
            return Ok(());
        };
        Err(unsupported_compatibility_feature(pointer, feature))
    }

    fn rewrite_reference(
        self,
        reference: &str,
        pointer: &JsonPointer,
    ) -> Result<String, OpenApiError> {
        match self {
            Self::Validation => Ok(rewrite_component_schema_reference_for_validation(reference)),
            Self::Lowering => rewrite_component_schema_reference_for_lowering(reference, pointer),
        }
    }
}

fn rewrite_schema_refs_for_validation(
    schema: &Value,
    pointer: &JsonPointer,
) -> Result<Value, OpenApiError> {
    rewrite_schema_refs(schema, pointer, SchemaReferenceRewrite::Validation)
}

fn rewrite_schema_refs_for_lowering(
    schema: &Value,
    pointer: &JsonPointer,
) -> Result<Value, OpenApiError> {
    rewrite_schema_refs(schema, pointer, SchemaReferenceRewrite::Lowering)
}

fn rewrite_schema_refs(
    schema: &Value,
    pointer: &JsonPointer,
    rewrite: SchemaReferenceRewrite,
) -> Result<Value, OpenApiError> {
    match schema {
        Value::Object(object) => {
            rewrite.validate_schema_object(object, pointer)?;
            let mut rewritten = Map::new();
            for (key, value) in object {
                let child_pointer = pointer.child(key);
                rewrite.reject_keyword_if_needed(key, &child_pointer)?;
                let rewritten_value = match key.as_str() {
                    "$ref" => {
                        let reference = value
                            .as_str()
                            .ok_or_else(|| invalid_value(&child_pointer, "a string"))?;
                        Value::String(rewrite.rewrite_reference(reference, &child_pointer)?)
                    }
                    "discriminator" => {
                        validate_discriminator(object, value, &child_pointer)?;
                        value.clone()
                    }
                    "readOnly" | "writeOnly" => {
                        if !value.is_boolean() {
                            return Err(invalid_value(&child_pointer, "a boolean"));
                        }
                        value.clone()
                    }
                    key if is_single_schema_child_keyword(key) => {
                        rewrite_schema_refs(value, &child_pointer, rewrite)?
                    }
                    key if is_schema_map_child_keyword(key) => {
                        rewrite_schema_map_refs(value, &child_pointer, rewrite)?
                    }
                    key if is_schema_array_child_keyword(key) => {
                        rewrite_schema_array_refs(value, &child_pointer, rewrite)?
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

fn unsupported_lowering_schema_keyword_feature(keyword: &str) -> Option<&'static str> {
    match keyword {
        "$id" => Some("JSON Schema keyword '$id'"),
        "$anchor" => Some("JSON Schema keyword '$anchor'"),
        "$dynamicRef" => Some("JSON Schema keyword '$dynamicRef'"),
        "$dynamicAnchor" => Some("JSON Schema keyword '$dynamicAnchor'"),
        "additionalItems" => Some("JSON Schema keyword 'additionalItems'"),
        "contentEncoding" => Some("JSON Schema keyword 'contentEncoding'"),
        "contentMediaType" => Some("JSON Schema keyword 'contentMediaType'"),
        "contentSchema" => Some("JSON Schema keyword 'contentSchema'"),
        "dependencies" => Some("JSON Schema keyword 'dependencies'"),
        "dependentSchemas" => Some("JSON Schema keyword 'dependentSchemas'"),
        "unevaluatedItems" => Some("JSON Schema keyword 'unevaluatedItems'"),
        "unevaluatedProperties" => Some("JSON Schema keyword 'unevaluatedProperties'"),
        _ => None,
    }
}

fn validate_optional_xml_field(
    schema: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    let Some(xml) = schema.get("xml") else {
        return Ok(());
    };
    let xml_pointer = pointer.child("xml");
    let xml = xml
        .as_object()
        .ok_or_else(|| invalid_value(&xml_pointer, "an object"))?;
    validate_object_fields(
        xml,
        &xml_pointer,
        &["name", "namespace", "prefix", "attribute", "wrapped"],
        "a supported OpenAPI XML field or specification extension beginning with 'x-'",
    )?;
    validate_optional_string_field(xml, "name", &xml_pointer)?;
    validate_optional_absolute_uri_field(xml, "namespace", &xml_pointer)?;
    validate_optional_string_field(xml, "prefix", &xml_pointer)?;
    validate_optional_bool_field(xml, "attribute", &xml_pointer)?;
    validate_optional_bool_field(xml, "wrapped", &xml_pointer)?;
    if xml.contains_key("wrapped") && schema_object_explicitly_excludes_array(schema) {
        return Err(invalid_value(
            &xml_pointer.child("wrapped"),
            "absent unless the sibling schema `type` includes `\"array\"`",
        ));
    }
    Ok(())
}

fn schema_object_explicitly_excludes_array(schema: &Map<String, Value>) -> bool {
    match schema.get("type") {
        Some(Value::String(schema_type)) => schema_type != "array",
        Some(Value::Array(schema_types)) => {
            schema_types.iter().all(Value::is_string)
                && !schema_types
                    .iter()
                    .any(|schema_type| schema_type.as_str() == Some("array"))
        }
        _ => false,
    }
}

fn reject_unsafe_number_bounds_in_schema_object(
    object: &Map<String, Value>,
    pointer: &JsonPointer,
) -> Result<(), OpenApiError> {
    if schema_object_has_integer_only_numeric_domain(object) {
        return Ok(());
    }

    for keyword in ["minimum", "maximum", "exclusiveMinimum", "exclusiveMaximum"] {
        let Some(value) = object.get(keyword) else {
            continue;
        };
        if !number_bound_is_outside_exact_f64_integer_range(value) {
            continue;
        }
        return Err(unsupported_compatibility_feature(
            &pointer.child(keyword),
            "JSON Schema number bounds outside the exact f64 integer range [-9007199254740991, 9007199254740991]",
        ));
    }

    Ok(())
}

fn schema_object_has_integer_only_numeric_domain(object: &Map<String, Value>) -> bool {
    match object.get("type") {
        Some(Value::String(schema_type)) => schema_type == "integer",
        Some(Value::Array(schema_types)) => {
            let mut has_integer = false;
            for schema_type in schema_types {
                let Some(schema_type) = schema_type.as_str() else {
                    return false;
                };
                match schema_type {
                    "integer" => has_integer = true,
                    "number" => return false,
                    _ => {}
                }
            }
            has_integer
        }
        _ => false,
    }
}

fn number_bound_is_outside_exact_f64_integer_range(value: &Value) -> bool {
    value
        .as_f64()
        .is_some_and(|value| value.is_finite() && value.abs() > MAX_EXACT_F64_INTEGER)
}

fn rewrite_schema_map_refs(
    value: &Value,
    pointer: &JsonPointer,
    rewrite: SchemaReferenceRewrite,
) -> Result<Value, OpenApiError> {
    let object = value
        .as_object()
        .ok_or_else(|| invalid_value(pointer, "an object of schemas"))?;
    object
        .iter()
        .map(|(key, schema)| {
            rewrite_schema_refs(schema, &pointer.child(key), rewrite)
                .map(|schema| (key.clone(), schema))
        })
        .collect::<Result<Map<_, _>, _>>()
        .map(Value::Object)
}

fn rewrite_schema_array_refs(
    value: &Value,
    pointer: &JsonPointer,
    rewrite: SchemaReferenceRewrite,
) -> Result<Value, OpenApiError> {
    let items = value
        .as_array()
        .ok_or_else(|| invalid_value(pointer, "an array of schemas"))?;
    items
        .iter()
        .enumerate()
        .map(|(index, schema)| {
            rewrite_schema_refs(schema, &pointer.child(index.to_string()), rewrite)
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Value::Array)
}

fn rewrite_component_schema_reference_for_validation(reference: &str) -> String {
    reference
        .strip_prefix(COMPONENT_SCHEMA_REF_PREFIX)
        .map_or_else(
            || reference.to_owned(),
            |component_path| format!("#/$defs/{component_path}"),
        )
}

fn rewrite_component_schema_reference_for_lowering(
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

enum OperationReference {
    External,
    InvalidLocal,
    Local(String),
}

fn classify_operation_reference(reference: &str) -> OperationReference {
    if !reference.starts_with('#') {
        return OperationReference::External;
    }

    match parse_local_reference(reference) {
        Some(pointer) => OperationReference::Local(pointer.render()),
        None => OperationReference::InvalidLocal,
    }
}

fn parse_local_reference(reference: &str) -> Option<JsonPointer> {
    if reference == "#" {
        return Some(JsonPointer::root());
    }

    let pointer = reference.strip_prefix("#/")?;
    let mut parsed = JsonPointer::root();
    for token in pointer.split('/') {
        parsed = parsed.child(decode_json_pointer_token(token)?);
    }
    Some(parsed)
}

fn decode_json_pointer_token(token: &str) -> Option<String> {
    let decoded = percent_decode_str(token).decode_utf8().ok()?;
    let mut unescaped = String::with_capacity(decoded.len());
    let mut characters = decoded.chars();
    while let Some(character) = characters.next() {
        if character != '~' {
            unescaped.push(character);
            continue;
        }
        match characters.next() {
            Some('0') => unescaped.push('~'),
            Some('1') => unescaped.push('/'),
            _ => return None,
        }
    }
    Some(unescaped)
}

fn lookup_pointer<'a>(root: &'a Value, reference: &str) -> Option<&'a Value> {
    let pointer = parse_local_reference(reference)?;
    let mut current = root;
    for token in pointer.tokens() {
        current = match current {
            Value::Object(object) => object.get(token)?,
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

fn unsupported_compatibility_feature(pointer: &JsonPointer, feature: &'static str) -> OpenApiError {
    OpenApiError::UnsupportedCompatibilityFeature {
        pointer: pointer.render(),
        feature,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        OpenApiDocument, collect_component_schema_names, lookup_pointer, lower_operations,
        rewrite_component_schema_reference_for_lowering, rewrite_schema_refs_for_lowering,
    };
    use crate::json_pointer::JsonPointer;
    use serde_json::{Value, json};

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
    fn pointer_lookup_rejects_invalid_escape_sequences() {
        let root = json!({
            "components": {
                "schemas": {
                    "Pet~Record": { "type": "object" }
                }
            }
        });

        assert!(lookup_pointer(&root, "#/components/schemas/Pet~2Record").is_none());
    }

    #[test]
    fn component_schema_refs_lower_into_defs_refs() {
        let lowered = rewrite_component_schema_reference_for_lowering(
            "#/components/schemas/Pet~1Record/properties/id",
            &JsonPointer::root().child("$ref"),
        )
        .unwrap();

        assert_eq!(lowered, "#/$defs/Pet~1Record/properties/id");
    }

    #[test]
    fn property_named_ref_is_not_treated_as_a_schema_reference_keyword() {
        let lowered = rewrite_schema_refs_for_lowering(
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

    #[test]
    fn component_dependency_collection_ignores_ref_shaped_const_payload_data() {
        let dependencies = collect_component_schema_names(&json!({
            "type": "object",
            "properties": {
                "payload": {
                    "const": {
                        "$ref": "#/$defs/NotASchemaDependency"
                    }
                },
                "pet": {
                    "$ref": "#/$defs/Pet"
                }
            }
        }));

        assert_eq!(dependencies, vec!["Pet"]);
    }

    #[test]
    fn component_dependency_collection_follows_later_lowering_schema_keywords() {
        let dependencies = collect_component_schema_names(&json!({
            "type": "object",
            "contentSchema": { "$ref": "#/$defs/Content" },
            "dependentSchemas": {
                "kind": { "$ref": "#/$defs/Dependency" }
            },
            "unevaluatedItems": { "$ref": "#/$defs/Items" },
            "unevaluatedProperties": { "$ref": "#/$defs/Properties" }
        }));

        assert_eq!(
            dependencies,
            vec!["Content", "Dependency", "Items", "Properties"]
        );
    }

    #[test]
    fn lowered_contract_schemas_inherit_the_openapi_document_dialect() {
        let document = OpenApiDocument::from_json(&json!({
            "openapi": "3.1.0",
            "jsonSchemaDialect": "https://json-schema.org/draft/2020-12/schema#",
            "info": {
                "title": "Pets",
                "version": "1.0.0"
            },
            "paths": {
                "/pets": {
                    "get": {
                        "responses": {
                            "200": {
                                "description": "ok"
                            }
                        }
                    }
                }
            }
        }))
        .unwrap();

        let operations = lower_operations(&document).unwrap();
        let operation = operations.values().next().unwrap();

        for schema in [&operation.request, &operation.response] {
            let schema = document.lowered_contract_document(schema).unwrap();
            assert_eq!(
                schema
                    .canonical_schema_json()
                    .unwrap()
                    .get("$schema")
                    .and_then(Value::as_str),
                Some("https://json-schema.org/draft/2020-12/schema"),
            );
        }
    }

    #[test]
    fn lowered_contracts_include_only_referenced_component_defs_and_transitive_dependencies() {
        let document = OpenApiDocument::from_json(&json!({
            "openapi": "3.1.0",
            "info": {
                "title": "Pets",
                "version": "1.0.0"
            },
            "paths": {
                "/pets": {
                    "get": {
                        "responses": {
                            "200": {
                                "description": "ok",
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "$ref": "#/components/schemas/Pet"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "components": {
                "schemas": {
                    "Pet": {
                        "type": "object",
                        "properties": {
                            "owner": {
                                "$ref": "#/components/schemas/Owner"
                            }
                        },
                        "required": ["owner"],
                        "additionalProperties": false
                    },
                    "Owner": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" }
                        },
                        "required": ["name"],
                        "additionalProperties": false
                    },
                    "Unused": {
                        "type": "string"
                    }
                }
            }
        }))
        .unwrap();

        let operations = lower_operations(&document).unwrap();
        let response = &operations
            .values()
            .next()
            .expect("operation should lower")
            .response;
        let defs = response["$defs"]
            .as_object()
            .expect("referenced component schemas should lower into $defs");

        assert!(defs.contains_key("Pet"));
        assert!(defs.contains_key("Owner"));
        assert!(!defs.contains_key("Unused"));
    }
}
