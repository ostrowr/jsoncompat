//! Generate API identities, typed requests, and operation metadata for one shared client.
use anyhow::{Context, Result, ensure};
use jsoncompat_codegen::DataclassMetadata;
use jsoncompat_openapi::{
    ApiMedia, ApiOperation, ApiParameter, OpenApiDocument, OpenApiOperationLowerer,
    ParameterEncoding, ParameterLocation, SchemaSerialization,
};
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

pub(super) struct ApiPackage {
    pub schemas: Map<String, Value>,
    pub metadata: DataclassMetadata,
    name: String,
    operations: Vec<Operation>,
    security_schemes: Value,
}
struct Operation {
    input: String,
    response_alias: String,
    success_alias: String,
    error_alias: String,
    result_alias: String,
    responses: Vec<(String, String, Option<String>)>,
    contract: ApiOperation,
}

impl Operation {
    fn partition(&self, success: bool) -> String {
        let mut models = BTreeSet::new();
        for status in (100..600).filter(|status| (200..300).contains(status) == success) {
            let exact = status.to_string();
            let range = format!("{}XX", status / 100);
            let selector = if self.responses.iter().any(|(s, _, _)| *s == exact) {
                &exact
            } else if self.responses.iter().any(|(s, _, _)| *s == range) {
                &range
            } else {
                "default"
            };
            models.extend(
                self.responses
                    .iter()
                    .filter(|(s, _, _)| s == selector)
                    .map(|(_, name, _)| name.as_str()),
            );
        }
        if models.is_empty() {
            "_typing.Never".into()
        } else {
            models.into_iter().collect::<Vec<_>>().join(" | ")
        }
    }
}

impl ApiPackage {
    pub fn prepare(document: &OpenApiDocument, raw: &Value, name: &str) -> Result<Self> {
        ensure!(
            identifier(name)
                && !name.starts_with('_')
                && name.chars().next().is_some_and(char::is_uppercase),
            "--api must be a public Python class name, such as Petstore"
        );
        let lowerer = OpenApiOperationLowerer::new(document)?;
        let mut package = Self {
            schemas: Map::new(),
            metadata: DataclassMetadata {
                imports: "from jsoncompat import http as _jsoncompat_http\n".into(),
                base_imports: "from . import _api as _jsoncompat_api\nif typing.TYPE_CHECKING:\n    from . import api as _jsoncompat_api_types\n".into(),
                ..Default::default()
            },
            name: name.into(),
            operations: Vec::new(),
            security_schemes: raw
                .pointer("/components/securitySchemes")
                .cloned()
                .unwrap_or_else(|| json!({})),
        };
        let mut definitions = Map::new();
        for group in document
            .component_schema_groups(&BTreeSet::new(), std::num::NonZeroUsize::new(128).unwrap())?
        {
            definitions.extend(group.schemas);
        }
        let definitions = json!({"$defs":definitions});
        let mut ids = BTreeSet::new();
        for key in lowerer.operation_keys()? {
            let contract = lowerer
                .operation_contract(&key)?
                .expect("operation key exists");
            if let Some(id) = &contract.operation_id {
                ensure!(ids.insert(id.clone()), "duplicate operationId {id}");
            }
            let fallback = format!(
                "{}_{}",
                key.method.to_ascii_lowercase(),
                key.path.replace('{', "by_").replace('}', "")
            );
            let operation_name = contract.operation_id.as_deref().unwrap_or(&fallback);
            let method_name = snake_name(operation_name);
            ensure!(
                !key.path.starts_with("//")
                    && !key.path.contains(['?', '#', '\\'])
                    && !key.path.chars().any(char::is_control),
                "unsupported path template {}",
                key.path
            );
            let prefix = super::codegen_package::class_name(operation_name);
            let input = format!("{prefix}Request");
            let response_alias = format!("{prefix}Response");
            let success_alias = format!("{prefix}Success");
            let error_alias = format!("{prefix}Error");
            let result_alias = format!("{prefix}Result");
            let mut properties = Map::new();
            let mut required = Vec::new();
            let mut metadata = BTreeMap::new();
            let mut field_names = BTreeSet::new();
            let candidates: Vec<_> = contract
                .parameters
                .iter()
                .map(|p| field_name(&p.name))
                .collect();
            for (parameter, candidate) in contract.parameters.iter().zip(&candidates) {
                let field = if candidate == "body"
                    || candidates
                        .iter()
                        .filter(|other| *other == candidate)
                        .count()
                        > 1
                {
                    format!("{}_{}", parameter.location.field_name(), candidate)
                } else {
                    candidate.clone()
                };
                ensure!(
                    field_names.insert(field.clone()),
                    "parameter names collide in {input} at {field}"
                );
                properties.insert(field.clone(), parameter.schema.clone());
                if parameter.required {
                    required.push(field.clone());
                }
                metadata.insert(
                    field,
                    parameter_metadata(parameter, false, &definitions)
                        .with_context(|| format!("{method_name} parameter {}", parameter.name))?,
                );
            }
            if let Some(body) = &contract.body {
                let body_schema = if body.content.len() == 1 {
                    validate_media(&body.content[0], &definitions)?;
                    metadata.insert(
                        "body".into(),
                        format!("_jsoncompat_http.Body({})", py(&body.content[0].media_type)),
                    );
                    body.content[0].schema.clone()
                } else {
                    let mut variants = Vec::new();
                    for media in &body.content {
                        validate_media(media, &definitions)?;
                        let name = format!(
                            "{prefix}Body{}",
                            super::codegen_package::class_name(&media.media_type)
                        );
                        package.insert(&name, record(json!({"content_type":{"enum":[media.media_type]}, "value":media.schema}), &["content_type", "value"]))?;
                        variants.push(reference(&name));
                    }
                    metadata.insert("body".into(), "_jsoncompat_http.Body(None)".into());
                    json!({"anyOf":variants})
                };
                ensure!(
                    !properties.contains_key("body"),
                    "body field collision in {input}"
                );
                properties.insert("body".into(), body_schema);
                if body.required {
                    required.push("body".into());
                }
            }
            package.insert(&input, json!({"type":"object", "properties":properties, "required":required, "additionalProperties":false}))?;
            package.metadata.fields.insert(input.clone(), metadata);
            let explicit: Vec<_> = contract
                .responses
                .iter()
                .filter_map(|r| r.selector.parse::<u16>().ok())
                .collect();
            let ranges: Vec<_> = contract
                .responses
                .iter()
                .filter(|r| r.selector.ends_with("XX"))
                .map(|r| r.selector.as_bytes()[0] - b'0')
                .collect();
            let mut responses = Vec::new();
            for response in &contract.responses {
                let status_name = if response.selector == "default" {
                    "Default"
                } else {
                    &response.selector
                };
                let headers_name = format!("{prefix}{status_name}Headers");
                let mut properties = Map::new();
                let mut required = Vec::new();
                let mut metadata = BTreeMap::new();
                for header in &response.headers {
                    let field = field_name(&header.name);
                    ensure!(
                        !properties.contains_key(&field),
                        "response header names collide at {field}"
                    );
                    properties.insert(field.clone(), header.schema.clone());
                    if header.required {
                        required.push(field.clone());
                    }
                    metadata.insert(
                        field,
                        parameter_metadata(header, true, &definitions).with_context(|| {
                            format!("{method_name} response header {}", header.name)
                        })?,
                    );
                }
                package.insert(&headers_name, json!({"type":"object", "properties":properties, "required":required, "additionalProperties":false}))?;
                package
                    .metadata
                    .fields
                    .insert(headers_name.clone(), metadata);
                let status_schema = if let Ok(exact) = response.selector.parse::<u16>() {
                    json!({"enum":[exact]})
                } else {
                    let mut excluded = Vec::new();
                    if !explicit.is_empty() {
                        excluded.push(json!({"enum":explicit}));
                    }
                    let (minimum, maximum) = if response.selector == "default" {
                        excluded.extend(ranges.iter().map(|range| json!({"minimum":u16::from(*range)*100,"maximum":u16::from(*range)*100+99})));
                        (100, 599)
                    } else {
                        let minimum = u16::from(response.selector.as_bytes()[0] - b'0') * 100;
                        (minimum, minimum + 99)
                    };
                    let mut schema = json!({"type":"integer","minimum":minimum,"maximum":maximum});
                    if !excluded.is_empty() {
                        schema["not"] = json!({"anyOf":excluded});
                    }
                    schema
                };
                let media: Vec<_> = if response.content.is_empty() {
                    vec![None]
                } else {
                    response.content.iter().map(Some).collect()
                };
                for media in media {
                    if let Some(media) = media {
                        ensure!(
                            contract.key.method != "HEAD"
                                && !matches!(response.selector.as_str(), "204" | "304"),
                            "{} response {} must not declare content for API generation",
                            contract.key.method,
                            response.selector
                        );
                        validate_media(media, &definitions)?;
                    }
                    let suffix = if response.content.len() > 1 {
                        super::codegen_package::class_name(&media.unwrap().media_type)
                    } else {
                        String::new()
                    };
                    let model = format!("{prefix}{status_name}{suffix}");
                    let mut properties = json!({"status":status_schema,"body":media.map(|m| m.schema.clone()).unwrap_or_else(|| json!({"type":"null"})),"headers":reference(&headers_name),"raw_headers":{"type":"array","items":{"type":"array","prefixItems":[{"type":"string"},{"type":"string"}],"items":false,"minItems":2}}});
                    let mut required = vec!["status", "body", "headers", "raw_headers"];
                    if let Some(media) = media {
                        properties["content_type"] = json!({"enum":[media.media_type]});
                        required.push("content_type");
                    }
                    let mut schema = record(properties, &required);
                    schema["description"] = json!(response.description);
                    package.insert(&model, schema)?;
                    responses.push((
                        response.selector.clone(),
                        model,
                        media.map(|m| m.media_type.clone()),
                    ));
                }
            }
            package.metadata.bases.insert(
                input.clone(),
                format!(
                    "_jsoncompat_http.Request[_jsoncompat_api.{name}API, {}, {}]",
                    py(&format!("_jsoncompat_api_types.{success_alias}")),
                    py(&format!("_jsoncompat_api_types.{error_alias}"))
                ),
            );
            package.operations.push(Operation {
                input,
                response_alias,
                success_alias,
                error_alias,
                result_alias,
                responses,
                contract,
            });
        }
        ensure!(
            !package.operations.is_empty(),
            "--api requires at least one path operation"
        );
        Ok(package)
    }
    fn insert(&mut self, name: &str, schema: Value) -> Result<()> {
        ensure!(
            self.schemas.insert(name.into(), schema).is_none(),
            "generated model names collide at {name}"
        );
        Ok(())
    }
    pub fn identity_source(&self) -> String {
        format!(
            "# Generated nominal API identity; importing it does not load models.\nfrom typing import final\nfrom jsoncompat import http as _http\n\n@final\nclass {}API(_http.API):\n    __slots__ = ()\n\n    @classmethod\n    def operations(cls) -> tuple[_http.Operation, ...]:\n        from .api import OPERATIONS\n        return OPERATIONS\n",
            self.name
        )
    }
    pub fn source(&self, exports: &BTreeMap<String, String>) -> Result<(String, Vec<String>)> {
        let mut names: BTreeSet<String> = exports.keys().cloned().collect();
        let mut public = Vec::new();
        for name in std::iter::once(format!("{}API", self.name))
            .chain([
                format!("{}Client", self.name),
                format!("{}AsyncClient", self.name),
            ])
            .chain(std::iter::once("SECURITY_SCHEMES".into()))
            .chain(self.operations.iter().flat_map(|o| {
                [
                    o.response_alias.clone(),
                    o.success_alias.clone(),
                    o.error_alias.clone(),
                    o.result_alias.clone(),
                ]
            }))
        {
            ensure!(names.insert(name.clone()), "API export collides at {name}");
            public.push(name);
        }
        let mut source = String::from(
            "# Generated operation registry. Dispatch is prepared once on import.\nfrom __future__ import annotations\n\nimport json as _json\nimport typing as _typing\nfrom typing_extensions import TypeAliasType as _TypeAliasType, TypeForm as _TypeForm, TypeVar as _TypeVar\nfrom jsoncompat import JsonValue as _JsonValue\nfrom jsoncompat import http as _http\nfrom jsoncompat.http._binding import initialize_api as _initialize_api\n",
        );
        writeln!(source, "from ._api import {0}API as {0}API", self.name)?;
        writeln!(
            source,
            "\n_TransportOptions = _TypeVar(\"_TransportOptions\", default=_typing.Never)\n{0}Client = _TypeAliasType(\"{0}Client\", _http.Client[{0}API, _TransportOptions], type_params=(_TransportOptions,))\n{0}AsyncClient = _TypeAliasType(\"{0}AsyncClient\", _http.AsyncClient[{0}API, _TransportOptions], type_params=(_TransportOptions,))",
            self.name
        )?;
        let imported: BTreeSet<_> = self
            .operations
            .iter()
            .flat_map(|o| {
                std::iter::once(&o.input).chain(o.responses.iter().map(|(_, name, _)| name))
            })
            .collect();
        for name in imported {
            writeln!(
                source,
                "from .{} import {name} as {name}",
                exports
                    .get(name)
                    .with_context(|| format!("missing generated export {name}"))?
            )?;
        }
        writeln!(
            source,
            "\nSECURITY_SCHEMES: _typing.Final[dict[str, _JsonValue]] = _json.loads({})",
            py(&self.security_schemes.to_string())
        )?;
        for operation in &self.operations {
            writeln!(
                source,
                "\ntype {} = {}",
                operation.response_alias,
                operation
                    .responses
                    .iter()
                    .map(|(_, name, _)| name.as_str())
                    .collect::<Vec<_>>()
                    .join(" | ")
            )?;
        }
        for operation in &self.operations {
            writeln!(
                source,
                "type {} = {}",
                operation.success_alias,
                operation.partition(true)
            )?;
            writeln!(
                source,
                "type {} = {}",
                operation.error_alias,
                operation.partition(false)
            )?;
            writeln!(
                source,
                "type {} = _http.Result[{}, {}]",
                operation.result_alias, operation.success_alias, operation.error_alias
            )?;
        }
        writeln!(
            source,
            "\nOPERATIONS: _typing.Final[tuple[_http.Operation, ...]] = ("
        )?;
        for operation in &self.operations {
            let contract = &operation.contract;
            let responses = operation
                .responses
                .iter()
                .map(|(status, model, media)| {
                    format!(
                        "_http.Response({}, {model}, {})",
                        status
                            .parse::<u16>()
                            .map(|n| n.to_string())
                            .unwrap_or_else(|_| py(status)),
                        media
                            .as_ref()
                            .map(|m| py(m))
                            .unwrap_or_else(|| "None".into())
                    )
                })
                .collect::<Vec<_>>();
            let security = contract
                .security
                .as_array()
                .context("security requirements")?
                .iter()
                .map(|requirement| {
                    let items = requirement
                        .as_object()
                        .context("security requirement object")?
                        .iter()
                        .map(|(name, scopes)| {
                            let scopes = scopes
                                .as_array()
                                .context("security scopes")?
                                .iter()
                                .map(|s| s.as_str().map(py).context("security scope string"))
                                .collect::<Result<Vec<_>>>()?;
                            Ok(format!(
                                "_http.SecurityRequirement({}, {})",
                                py(name),
                                tuple(&scopes)
                            ))
                        })
                        .collect::<Result<Vec<_>>>()?;
                    Ok(tuple(&items))
                })
                .collect::<Result<Vec<_>>>()?;
            let servers = contract
                .servers
                .as_array()
                .context("servers array")?
                .iter()
                .map(|server| {
                    let variables = server
                        .get("variables")
                        .and_then(Value::as_object)
                        .into_iter()
                        .flatten()
                        .map(|(name, variable)| {
                            Ok(format!(
                                "({}, {})",
                                py(name),
                                py(variable["default"]
                                    .as_str()
                                    .context("server variable default")?)
                            ))
                        })
                        .collect::<Result<Vec<_>>>()?;
                    Ok(format!(
                        "_http.Server({}, {})",
                        py(server["url"].as_str().context("server URL")?),
                        tuple(&variables)
                    ))
                })
                .collect::<Result<Vec<_>>>()?;
            writeln!(
                source,
                "    _http.operation({}, _TypeForm({}), _TypeForm({}), method={}, path={}, responses={}, operation_id={}, security={}, servers={}),",
                operation.input,
                operation.success_alias,
                operation.error_alias,
                py(&contract.key.method),
                py(&contract.key.path),
                tuple(&responses),
                contract
                    .operation_id
                    .as_ref()
                    .map(|s| py(s))
                    .unwrap_or_else(|| "None".into()),
                tuple(&security),
                tuple(&servers)
            )?;
        }
        writeln!(source, ")\n")?;
        writeln!(source, "_initialize_api({}API)\n", self.name)?;
        writeln!(
            source,
            "__all__ = {}",
            tuple(&public.iter().map(|s| py(s)).collect::<Vec<_>>())
        )?;
        Ok((source, public))
    }
}

fn tuple(items: &[String]) -> String {
    if items.is_empty() {
        "()".into()
    } else {
        format!("({},)", items.join(", "))
    }
}
fn py(value: &str) -> String {
    serde_json::to_string(value).expect("string serialization")
}
fn reference(name: &str) -> Value {
    json!({"$ref":format!("#/$defs/{name}")})
}
fn record(properties: Value, required: &[&str]) -> Value {
    json!({"type":"object", "properties":properties,"required":required,"additionalProperties":false})
}
pub(super) fn identifier(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !KEYWORDS.contains(&name)
}
const KEYWORDS: &[&str] = &[
    "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class", "continue",
    "def", "del", "elif", "else", "except", "finally", "for", "from", "global", "if", "import",
    "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while",
    "with", "yield",
];
fn snake_name(name: &str) -> String {
    let mut result = String::new();
    let chars: Vec<_> = name.chars().collect();
    for (index, ch) in chars.iter().enumerate() {
        if !ch.is_ascii_alphanumeric() {
            if !result.ends_with('_') {
                result.push('_');
            }
            continue;
        }
        if ch.is_ascii_uppercase()
            && index > 0
            && (chars[index - 1].is_ascii_lowercase()
                || chars[index - 1].is_ascii_digit()
                || (chars[index - 1].is_ascii_uppercase()
                    && chars.get(index + 1).is_some_and(char::is_ascii_lowercase)))
            && !result.ends_with('_')
        {
            result.push('_');
        }
        result.push(ch.to_ascii_lowercase());
    }
    let mut result = result.trim_matches('_').to_owned();
    if result.is_empty() || result.starts_with(|c: char| c.is_ascii_digit()) {
        result.insert_str(0, "operation_");
    }
    if KEYWORDS.contains(&result.as_str()) {
        result.push('_');
    }
    result
}
fn field_name(name: &str) -> String {
    let value = snake_name(name);
    if [
        "skip_validation",
        "serialize",
        "deserialize",
        "to_value",
        "from_value",
        "root",
        "get_additional_property",
    ]
    .contains(&value.as_str())
    {
        format!("param_{value}")
    } else {
        value
    }
}
fn validate_media(media: &ApiMedia, definitions: &Value) -> Result<()> {
    validate_media_parameters(&media.parameters)?;
    ensure!(
        media.media_type == "application/json"
            || media.media_type == "text/plain"
            || (media.media_type.starts_with("application/")
                && media.media_type.ends_with("+json")),
        "API generation supports JSON and text/plain media types; unsupported {}",
        media.media_type
    );
    if media.media_type == "text/plain" {
        let schema = resolve_schema(&media.schema, definitions)?;
        ensure!(
            schema.get("type").and_then(Value::as_str) == Some("string")
                || schema
                    .get("enum")
                    .and_then(Value::as_array)
                    .is_some_and(|values| values.iter().all(Value::is_string)),
            "text/plain requires a string schema"
        );
    }
    Ok(())
}
fn parameter_metadata(
    parameter: &ApiParameter,
    response: bool,
    definitions: &Value,
) -> Result<String> {
    let class = match parameter.location {
        ParameterLocation::Path => "Path",
        ParameterLocation::Query => "Query",
        ParameterLocation::Header => "Header",
        ParameterLocation::Cookie => "Cookie",
    };
    let mut args = vec![py(&parameter.name)];
    match &parameter.encoding {
        ParameterEncoding::Content {
            media_type,
            parameters,
        } => {
            validate_media_parameters(parameters)?;
            ensure!(
                media_type == "application/json"
                    || (media_type.starts_with("application/") && media_type.ends_with("+json")),
                "parameter content must be JSON"
            );
            args.push(format!("content_type={}", py(media_type)));
        }
        ParameterEncoding::Schema(serialization) => {
            let explode = match serialization {
                SchemaSerialization::PathParameter { style, explode }
                | SchemaSerialization::CookieParameter { style, explode } => {
                    if class != "Cookie" {
                        args.push(format!("style={}", py(style.as_str())));
                    }
                    *explode
                }
                SchemaSerialization::QueryParameter {
                    style,
                    explode,
                    allow_reserved,
                    allow_empty_value,
                } => {
                    ensure!(
                        !allow_reserved && !allow_empty_value,
                        "allowReserved and allowEmptyValue are not supported by API generation"
                    );
                    ensure!(
                        !matches!(style.as_str(), "spaceDelimited" | "pipeDelimited") || !explode,
                        "delimited query arrays require explode=false"
                    );
                    args.push(format!("style={}", py(style.as_str())));
                    *explode
                }
                SchemaSerialization::Header { explode } => *explode,
            };
            args.push(format!(
                "explode={}",
                if explode { "True" } else { "False" }
            ));
            if response {
                let schema = resolve_schema(&parameter.schema, definitions)?;
                let array = schema.get("type").and_then(Value::as_str) == Some("array");
                let scalar = if array {
                    resolve_schema(
                        schema
                            .get("items")
                            .context("response header array needs items")?,
                        definitions,
                    )?
                } else {
                    schema
                };
                let kind = scalar.get("type").and_then(Value::as_str).context("response headers need an explicit scalar type, an array of scalars, or JSON content")?;
                ensure!(
                    ["string", "integer", "number", "boolean"].contains(&kind),
                    "unsupported response header schema type {kind}; use JSON content for structured headers"
                );
                args.push(format!("kind={}", py(kind)));
                args.push(format!("array={}", if array { "True" } else { "False" }));
            }
        }
    }
    Ok(format!("_jsoncompat_http.{class}({})", args.join(", ")))
}
fn resolve_schema<'a>(mut schema: &'a Value, definitions: &'a Value) -> Result<&'a Value> {
    let mut seen = BTreeSet::new();
    while let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        ensure!(
            seen.insert(reference),
            "recursive response header schema requires JSON content"
        );
        schema = reference
            .strip_prefix('#')
            .and_then(|r| definitions.pointer(r))
            .context("unresolved response header schema reference")?;
    }
    Ok(schema)
}

fn validate_media_parameters(parameters: &BTreeMap<String, String>) -> Result<()> {
    ensure!(
        parameters
            .iter()
            .all(|(name, value)| name.eq_ignore_ascii_case("charset")
                && matches!(value.to_ascii_lowercase().as_str(), "utf-8" | "utf8")),
        "API generation supports only an optional UTF-8 charset media parameter"
    );
    Ok(())
}
