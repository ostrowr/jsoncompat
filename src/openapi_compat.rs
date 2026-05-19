//! Compatibility reporting over lowered OpenAPI request/response contracts.

use crate::{
    CompatibilityError, Role, check_compat, explain_compat_failure, validate_compatibility_input,
};
use jsoncompat_openapi::{
    LoweredOperation, OpenApiDocument, OpenApiLoweringError, OperationKey, lower_operations,
};
use std::collections::BTreeMap;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OpenApiCompatibilityError {
    #[error(transparent)]
    Lowering(#[from] OpenApiLoweringError),
    #[error(transparent)]
    Compatibility(#[from] CompatibilityError),
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

struct OpenApiCompatibilityInput<'a> {
    document: &'a OpenApiDocument,
    operations: BTreeMap<OperationKey, PreparedOperation>,
}

struct PreparedOperation {
    lowered: LoweredOperation,
    request: crate::SchemaDocument,
    response: crate::SchemaDocument,
}

impl<'a> OpenApiCompatibilityInput<'a> {
    fn new(document: &'a OpenApiDocument) -> Result<Self, OpenApiCompatibilityError> {
        let mut operations = BTreeMap::new();
        for (key, lowered) in lower_operations(document)? {
            let request = prepare_lowered_contract(document, &lowered.request)?;
            let response = prepare_lowered_contract(document, &lowered.response)?;
            operations.insert(
                key,
                PreparedOperation {
                    lowered,
                    request,
                    response,
                },
            );
        }

        Ok(Self {
            document,
            operations,
        })
    }
}

/// Return whether an OpenAPI document is fully supported by the compatibility layer.
///
/// [`OpenApiDocument::from_json`] validates OpenAPI document shape. This helper
/// finishes the compatibility-specific validation pass: it resolves and lowers
/// every operation contract, then verifies that the generated request and
/// response schemas stay inside the raw JSON Schema compatibility subset.
pub fn validate_openapi_compatibility_input(
    document: &OpenApiDocument,
) -> Result<(), OpenApiCompatibilityError> {
    OpenApiCompatibilityInput::new(document).map(|_| ())
}

pub fn check_openapi_compat(
    old: &OpenApiDocument,
    new: &OpenApiDocument,
) -> Result<OpenApiCompatibilityReport, OpenApiCompatibilityError> {
    let old = OpenApiCompatibilityInput::new(old)?;
    let new = OpenApiCompatibilityInput::new(new)?;
    let mut report = OpenApiCompatibilityReport::default();
    let dialects_differ = !old.document.uses_same_schema_dialect_as(new.document)?;

    for (key, old_operation) in &old.operations {
        let Some(new_operation) = new.operations.get(key) else {
            report.push(
                key,
                OpenApiCompatibilitySurface::Operation,
                "operation was removed",
            );
            continue;
        };
        if (old_operation.lowered.request != new_operation.lowered.request || dialects_differ)
            && !check_compat(
                &old_operation.request,
                &new_operation.request,
                Role::Deserializer,
            )?
        {
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

        if (old_operation.lowered.response != new_operation.lowered.response || dialects_differ)
            && !check_compat(
                &old_operation.response,
                &new_operation.response,
                Role::Serializer,
            )?
        {
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

fn prepare_lowered_contract(
    document: &OpenApiDocument,
    schema: &serde_json::Value,
) -> Result<crate::SchemaDocument, OpenApiCompatibilityError> {
    let schema = document.lowered_contract_document(schema)?;
    validate_compatibility_input(&schema)?;
    Ok(schema)
}
