//! OpenAPI 3.1 document validation and lowering into JSON Schema envelopes.

mod json_pointer;

mod validation;
use validation::*;
mod schema_groups;
pub use schema_groups::ComponentSchemaGroup;

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
    contract_fields_object_schema(parameters.map(ContractField::from))
}

fn contract_fields_object_schema(fields: impl IntoIterator<Item = ContractField>) -> Value {
    let mut properties = Map::new();
    let mut required = Vec::new();
    for field in fields {
        if field.required {
            required.push(Value::String(field.name.clone()));
        }
        properties.insert(field.name.clone(), contract_field_schema(&field));
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
        return Ok(contract_fields_object_schema(Vec::new()));
    };
    let headers = raw
        .as_object()
        .ok_or_else(|| invalid_value(pointer, "an object"))?;
    let mut fields = Vec::new();
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
        fields.push(field);
    }

    Ok(contract_fields_object_schema(fields))
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
