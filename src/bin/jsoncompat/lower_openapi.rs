use std::collections::BTreeMap;

use anyhow::{Context, Result};
use jsoncompat as backcompat;
use jsoncompat_openapi::lower_operations;
use serde::Serialize;
use serde_json::Value;

use crate::read_to_string;

#[derive(clap::Args)]
pub(crate) struct LowerOpenApiArgs {
    /// Path to the OpenAPI 3.1 JSON document.
    spec: String,
}

#[derive(Debug, Serialize)]
struct LoweredOpenApiDocument {
    operations: BTreeMap<String, LoweredOperationDocument>,
}

#[derive(Debug, Serialize)]
struct LoweredOperationDocument {
    request: Value,
    response: Value,
}

pub(crate) fn cmd(args: LowerOpenApiArgs) -> Result<()> {
    let document = load_document(&args.spec)?;
    let lowered = lower_document(&document)
        .with_context(|| format!("validating OpenAPI compatibility input for {}", args.spec))?;
    println!("{}", serde_json::to_string_pretty(&lowered)?);
    Ok(())
}

fn load_document(path: &str) -> Result<backcompat::OpenApiDocument> {
    let raw = read_to_string(path)?;
    let json: Value = serde_json::from_str(&raw).with_context(|| format!("parsing {path}"))?;
    backcompat::OpenApiDocument::from_json(&json)
        .with_context(|| format!("building OpenAPI document for {path}"))
}

fn lower_document(document: &backcompat::OpenApiDocument) -> Result<LoweredOpenApiDocument> {
    let mut operations = BTreeMap::new();
    for (key, operation) in lower_operations(document)? {
        operations.insert(
            format!("{} {}", key.method, key.path),
            LoweredOperationDocument {
                request: lowered_contract_source(document, &operation.request)?,
                response: lowered_contract_source(document, &operation.response)?,
            },
        );
    }

    Ok(LoweredOpenApiDocument { operations })
}

fn lowered_contract_source(
    document: &backcompat::OpenApiDocument,
    lowered_schema: &Value,
) -> Result<Value> {
    let schema = document.lowered_contract_document(lowered_schema)?;
    backcompat::validate_compatibility_input(&schema)?;
    Ok(schema.source_schema_json().clone())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use serde_json::json;

    use super::*;

    #[test]
    fn lower_document_prints_validated_operation_contracts_with_the_document_dialect() {
        let document = backcompat::OpenApiDocument::from_json(&json!({
            "openapi": "3.1.0",
            "jsonSchemaDialect": "https://json-schema.org/draft/2020-12/schema#",
            "info": { "title": "Pets", "version": "1.0.0" },
            "paths": {
                "/pets": {
                    "get": {
                        "responses": {
                            "200": {
                                "description": "ok",
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
        }))
        .unwrap();

        let lowered = serde_json::to_value(lower_document(&document).unwrap()).unwrap();
        let operation = &lowered["operations"]["GET /pets"];

        assert_eq!(
            operation["request"]["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert_eq!(
            operation["response"]["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert_eq!(
            operation["response"]["properties"]["body"]["properties"]["content_type"]["properties"]
                ["type"]["enum"][0],
            "application"
        );
        assert_eq!(
            operation["response"]["properties"]["body"]["properties"]["content_type"]["properties"]
                ["subtype"]["enum"][0],
            "json"
        );
    }

    #[test]
    fn lower_openapi_command_reports_unsupported_contract_surfaces_as_readiness_errors() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("jsoncompat-lower-openapi-{unique}.json"));
        fs::write(
            &path,
            r#"{
                "openapi":"3.1.0",
                "info":{"title":"Pets","version":"1.0.0"},
                "webhooks":{
                    "pet.created":{
                        "post":{"responses":{"200":{"description":"ok"}}}
                    }
                }
            }"#,
        )
        .unwrap();

        let error = cmd(LowerOpenApiArgs {
            spec: path.to_string_lossy().into_owned(),
        })
        .unwrap_err();

        fs::remove_file(path).unwrap();

        let message = format!("{error:#}");
        assert!(
            message.contains("validating OpenAPI compatibility input for"),
            "{message}"
        );
        assert!(message.contains("#/webhooks"), "{message}");
        assert!(
            message.contains("OpenAPI compatibility checks do not support webhooks"),
            "{message}"
        );
    }
}
