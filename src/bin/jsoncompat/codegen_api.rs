//! Generate API identities, typed requests, and operation metadata for one shared client.
use anyhow::{Context, Result, ensure};
use jsoncompat_codegen::DataclassMetadata;
use jsoncompat_codegen::python_names::module_names;
use jsoncompat_openapi::{
    ApiMedia, ApiOperation, ApiParameter, OpenApiDocument, OpenApiOperationLowerer,
    ParameterEncoding, ParameterLocation, SchemaSerialization,
};
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

pub(super) struct ApiSource {
    pub imports: String,
    pub public: String,
    pub private: String,
}

pub(super) struct ApiPackage {
    pub schemas: Map<String, Value>,
    pub metadata: DataclassMetadata,
    pub owners: BTreeMap<String, String>,
    pub aliases: BTreeMap<String, (String, String)>,
    current_owner: String,
    name: String,
    operations: Vec<Operation>,
    security_schemes: Value,
    runtime_names: RuntimeNames,
    generated_names: BTreeSet<String>,
}

/// Ordinary schema/operation names are converted to UpperCamelCase. Explicit
/// component renames and schema declaration metadata can use private-looking
/// identifiers, so keep short aliases only when they do not collide. The
/// fallback prefix is reserved by the model generator.
struct RuntimeNames {
    http: &'static str,
    codegen: &'static str,
    type_form: &'static str,
}

fn collect_declaration_names(value: &Value, names: &mut BTreeSet<String>) {
    match value {
        Value::Object(object) => {
            if let Some(name) = object
                .get("x-jsoncompat")
                .and_then(|metadata| metadata.get("name"))
                .and_then(Value::as_str)
            {
                names.insert(name.to_owned());
            }
            for child in object.values() {
                collect_declaration_names(child, names);
            }
        }
        Value::Array(values) => {
            for child in values {
                collect_declaration_names(child, names);
            }
        }
        _ => {}
    }
}

impl RuntimeNames {
    fn for_document(raw: &Value, renames: &[String]) -> Self {
        fn declared_names<'a>(value: &'a Value, names: &mut BTreeSet<&'a str>) {
            match value {
                Value::Object(object) => {
                    if let Some(name) = object
                        .get("x-jsoncompat")
                        .and_then(|metadata| metadata.get("name"))
                        .and_then(Value::as_str)
                    {
                        names.insert(name);
                    }
                    for child in object.values() {
                        declared_names(child, names);
                    }
                }
                Value::Array(values) => {
                    for child in values {
                        declared_names(child, names);
                    }
                }
                _ => {}
            }
        }
        let mut names = BTreeSet::new();
        declared_names(raw, &mut names);
        let alias = |short, fallback| {
            if names.contains(short)
                || renames.iter().any(|rename| {
                    rename
                        .split_once('=')
                        .is_some_and(|(_, model)| model == short)
                })
            {
                fallback
            } else {
                short
            }
        };
        Self {
            http: alias("_http", "_jsoncompat_http"),
            codegen: alias("_http_codegen", "_jsoncompat_http_codegen"),
            type_form: alias("_TypeForm", "_jsoncompat_TypeForm"),
        }
    }
}
struct Operation {
    resource: String,
    input: String,
    response_alias: String,
    success_alias: String,
    error_alias: String,
    result_alias: String,
    responses: Vec<ResponseModel>,
    declarations: Vec<String>,
    path_template: Vec<(bool, String)>,
    contract: ApiOperation,
}
struct ResponseModel {
    status: String,
    model: String,
    media: Option<String>,
    codec: &'static str,
    headers: String,
}

impl Operation {
    fn alias_names(&self) -> impl Iterator<Item = &str> {
        [
            &self.response_alias,
            &self.success_alias,
            &self.error_alias,
            &self.result_alias,
        ]
        .into_iter()
        .map(String::as_str)
    }

    fn alias_source(&self, http: &str) -> String {
        let mut source = String::new();
        for (name, annotation) in [
            (
                &self.response_alias,
                self.responses
                    .iter()
                    .map(|r| r.model.as_str())
                    .collect::<Vec<_>>()
                    .join(" | "),
            ),
            (&self.success_alias, self.partition(true)),
            (&self.error_alias, self.partition(false)),
            (
                &self.result_alias,
                format!(
                    "{http}.Result[{}, {}]",
                    self.success_alias, self.error_alias
                ),
            ),
        ] {
            // Parenthesize unions so large response sets remain easy to scan.
            if name.len() + annotation.len() > 96 && annotation.contains(" | ") {
                writeln!(source, "type {name} = (").unwrap();
                for (i, part) in annotation.split(" | ").enumerate() {
                    writeln!(source, "    {}{part}", if i == 0 { "" } else { "| " }).unwrap();
                }
                writeln!(source, ")").unwrap();
            } else {
                writeln!(source, "type {name} = {annotation}").unwrap();
            }
        }
        source.push('\n');
        source
    }

    fn partition(&self, success: bool) -> String {
        // Resolve selectors once. Formatting and searching the same response
        // list for all 500 status codes made large registries expensive.
        let mut exact = [None; 500];
        let mut ranges = [None; 5];
        let mut fallback = None;
        for response in &self.responses {
            let selector = &response.status;
            match selector.as_bytes() {
                b"default" => fallback = Some(selector.as_str()),
                [class @ b'1'..=b'5', b'X', b'X'] => {
                    ranges[usize::from(class - b'1')] = Some(selector.as_str());
                }
                _ => {
                    let status: usize = selector.parse().expect("validated response status");
                    exact[status - 100] = Some(selector.as_str());
                }
            }
        }
        let selectors: BTreeSet<_> = (100..600)
            .filter(|status| (200..300).contains(status) == success)
            .filter_map(|status| {
                exact[status - 100]
                    .or(ranges[status / 100 - 1])
                    .or(fallback)
            })
            .collect();
        let mut models = BTreeSet::new();
        models.extend(
            self.responses
                .iter()
                .filter(|response| selectors.contains(response.status.as_str()))
                .map(|response| response.model.as_str()),
        );
        if models.is_empty() {
            "typing.Never".into()
        } else {
            models.into_iter().collect::<Vec<_>>().join(" | ")
        }
    }
}

impl ApiPackage {
    pub fn prepare(
        document: &OpenApiDocument,
        raw: &Value,
        name: &str,
        renames: &[String],
    ) -> Result<Self> {
        ensure!(
            identifier(name)
                && !name.starts_with('_')
                && name.chars().next().is_some_and(char::is_uppercase),
            "--api must be a public Python class name, such as Petstore"
        );
        let lowerer = OpenApiOperationLowerer::new(document)?;
        let runtime_names = RuntimeNames::for_document(raw, renames);
        let RuntimeNames { http, .. } = runtime_names;
        // Component names retain their public spelling. Generated HTTP wrappers
        // and aliases choose unused names, including when a component already
        // has the natural name of an operation's request or response.
        let mut generated_names = BTreeSet::new();
        for (component, _) in lowerer.component_schemas() {
            generated_names.insert(component.to_owned());
            generated_names.insert(super::codegen_package::class_name(component));
        }
        for rename in renames {
            if let Some((_, model)) = rename.split_once('=') {
                generated_names.insert(model.to_owned());
            }
        }
        collect_declaration_names(raw, &mut generated_names);
        generated_names.extend([
            format!("{name}API"),
            format!("{name}Client"),
            format!("{name}AsyncClient"),
            "SECURITY_SCHEMES".into(),
            "OPERATIONS".into(),
        ]);
        let mut package = Self {
            schemas: Map::new(),
            owners: BTreeMap::new(),
            aliases: BTreeMap::new(),
            current_owner: String::new(),
            metadata: DataclassMetadata {
                imports: format!("from jsoncompat import http as {http}\n"),
                base_imports: format!("from ._api import {name}API\n"),
                ..Default::default()
            },
            name: name.into(),
            operations: Vec::new(),
            security_schemes: raw
                .pointer("/components/securitySchemes")
                .cloned()
                .unwrap_or_else(|| json!({})),
            runtime_names,
            generated_names,
        };
        let definitions = lowerer
            .component_schemas()
            .map(|(name, schema)| (name.to_owned(), schema.clone()))
            .collect();
        let definitions = Value::Object(Map::from_iter([(
            "$defs".into(),
            Value::Object(definitions),
        )]));
        let contracts = lowerer
            .operation_keys()?
            .iter()
            .map(|key| {
                lowerer
                    .operation_contract(key)
                    .map(|c| c.expect("operation key exists"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let resources: BTreeSet<_> = contracts.iter().map(resource_name).collect();
        let resource_modules = module_names(resources.iter().map(String::as_str));
        let empty_headers = package.allocate_name("JsoncompatEmptyHeaders".into());
        let mut ids = BTreeSet::new();
        for contract in contracts {
            let key = &contract.key;
            let resource = format!("operations.{}", resource_modules[&resource_name(&contract)]);
            package.current_owner = resource.clone();
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
            let path_template = prepare_path_template(&key.path)?;
            ensure!(
                !contract
                    .parameters
                    .iter()
                    .any(|p| p.location == ParameterLocation::Header
                        && p.name.eq_ignore_ascii_case("cookie"))
                    || !contract
                        .parameters
                        .iter()
                        .any(|p| p.location == ParameterLocation::Cookie),
                "conflicting Cookie header and cookie parameter"
            );
            let prefix = super::codegen_package::class_name(operation_name);
            let input = package.allocate_name(format!("{prefix}Request"));
            let response_alias = package.allocate_name(format!("{prefix}Response"));
            let success_alias = package.allocate_name(format!("{prefix}Success"));
            let error_alias = package.allocate_name(format!("{prefix}Error"));
            let result_alias = package.allocate_name(format!("{prefix}Result"));
            let mut declarations = Vec::new();
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
                    parameter_metadata(parameter, false, &definitions, http)
                        .with_context(|| format!("{method_name} parameter {}", parameter.name))?,
                );
            }
            if let Some(body) = &contract.body {
                let body_schema = if body.content.len() == 1 {
                    let media = project_media(&body.content[0], &definitions, false)?;
                    metadata.insert("body".into(), media.body_metadata(http));
                    media.schema
                } else {
                    let mut variants = Vec::new();
                    let mut codecs = Vec::new();
                    for original in &body.content {
                        let media = project_media(original, &definitions, false)?;
                        let name = package.allocate_name(format!(
                            "{prefix}Body{}",
                            super::codegen_package::class_name(&media.content_type)
                        ));
                        codecs.push(media.body_metadata(http));
                        package.insert(&name, record(json!({"content_type":{"enum":[media.content_type]}, "value":media.schema}), &["content_type", "value"]))?;
                        declarations.push(name.clone());
                        variants.push(reference(&name));
                    }
                    metadata.insert(
                        "body".into(),
                        format!("{http}.Body(None, variants={})", tuple(&codecs)),
                    );
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
            let mut description = format!("{} {}", key.method, key.path);
            let summary = raw
                .get("paths")
                .and_then(|paths| paths.get(&key.path))
                .and_then(|path| path.get(key.method.to_ascii_lowercase()))
                .and_then(|operation| operation.get("summary"))
                .and_then(Value::as_str);
            if let Some(summary) = summary {
                description.push_str("\n\n");
                description.push_str(summary);
            }
            if let Some(text) = &contract.description
                && Some(text.as_str()) != summary
            {
                description.push_str("\n\n");
                description.push_str(text);
            }
            package
                .metadata
                .docstrings
                .insert(input.clone(), description);
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
                let headers_name = package.allocate_name(format!("{prefix}{status_name}Headers"));
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
                        parameter_metadata(header, true, &definitions, http).with_context(
                            || format!("{method_name} response header {}", header.name),
                        )?,
                    );
                }
                let headers_schema = json!({"type":"object", "properties":properties, "required":required, "additionalProperties":false});
                let headers_name = if response.headers.is_empty() {
                    package
                        .aliases
                        .insert(headers_name, (empty_headers.clone(), resource.clone()));
                    if !package.schemas.contains_key(&empty_headers) {
                        package.insert(&empty_headers, headers_schema)?;
                        package
                            .owners
                            .insert(empty_headers.clone(), "models._empty_headers".into());
                    }
                    empty_headers.clone()
                } else {
                    package.insert(&headers_name, headers_schema)?;
                    declarations.push(headers_name.clone());
                    headers_name
                };
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
                    if media.is_some() {
                        ensure!(
                            contract.key.method != "HEAD"
                                && !matches!(response.selector.as_str(), "204" | "304"),
                            "{} response {} must not declare content for API generation",
                            contract.key.method,
                            response.selector
                        );
                    }
                    let suffix = if response.content.len() > 1 {
                        super::codegen_package::class_name(&media.unwrap().media_type)
                    } else {
                        String::new()
                    };
                    let projected = media
                        .map(|m| project_media(m, &definitions, true))
                        .transpose()?;
                    let model = package.allocate_name(format!("{prefix}{status_name}{suffix}"));
                    let mut properties = json!({"status":status_schema,"body":projected.as_ref().map(|m| m.schema.clone()).unwrap_or_else(|| json!({"type":"null"})),"headers":reference(&headers_name),"raw_headers":{"type":"array","items":{"type":"array","prefixItems":[{"type":"string"},{"type":"string"}],"items":false,"minItems":2}}});
                    let mut required = vec!["status", "body", "headers", "raw_headers"];
                    if let Some(media) = &projected {
                        properties["content_type"] = json!({"enum":[media.content_type]});
                        required.push("content_type");
                    }
                    let mut schema = record(properties, &required);
                    schema["description"] = json!(response.description);
                    package.insert(&model, schema)?;
                    declarations.push(model.clone());
                    responses.push(ResponseModel {
                        status: response.selector.clone(),
                        model,
                        media: projected.as_ref().map(|m| m.content_type.clone()),
                        codec: projected.as_ref().map_or("json", |m| m.codec),
                        headers: headers_name.clone(),
                    });
                }
            }
            package.metadata.bases.insert(
                input.clone(),
                format!(
                    "{http}.Request[{name}API, {}, {}]",
                    py(&success_alias),
                    py(&error_alias)
                ),
            );
            declarations.push(input.clone());
            package.operations.push(Operation {
                resource,
                input,
                response_alias,
                success_alias,
                error_alias,
                result_alias,
                responses,
                declarations,
                path_template,
                contract,
            });
        }
        ensure!(
            !package.operations.is_empty(),
            "--api requires at least one path operation"
        );
        for operation in &package.operations {
            package
                .metadata
                .declaration_order
                .extend(operation.declarations.clone());
            package
                .metadata
                .before
                .insert(operation.input.clone(), operation.alias_source(http));
            package.metadata.extra_exports.insert(
                operation.input.clone(),
                operation.alias_names().map(str::to_owned).collect(),
            );
        }
        package.metadata.reserved_names = package.generated_names.clone();
        Ok(package)
    }
    fn allocate_name(&mut self, preferred: String) -> String {
        if self.generated_names.insert(preferred.clone()) {
            return preferred;
        }
        let mut suffix = 2;
        loop {
            let candidate = format!("{preferred}{suffix}");
            if self.generated_names.insert(candidate.clone()) {
                return candidate;
            }
            suffix += 1;
        }
    }
    fn insert(&mut self, name: &str, schema: Value) -> Result<()> {
        ensure!(
            self.schemas.insert(name.into(), schema).is_none(),
            "generated model names collide at {name}"
        );
        self.owners.insert(name.into(), self.current_owner.clone());
        Ok(())
    }
    pub fn alias_owner(&self, name: &str) -> Option<&str> {
        self.operations
            .iter()
            .find(|o| {
                [
                    o.response_alias.as_str(),
                    o.success_alias.as_str(),
                    o.error_alias.as_str(),
                    o.result_alias.as_str(),
                ]
                .contains(&name)
            })
            .map(|o| o.resource.as_str())
    }
    pub fn identity_source(&self) -> String {
        format!(
            "# Generated nominal API identity; importing it does not load models.\nfrom typing import final, override\nfrom jsoncompat import http as _http\n\n@final\nclass {}API(_http.API):\n    __slots__ = ()\n\n    @classmethod\n    @override\n    def operations(cls) -> tuple[_http.Operation, ...]:\n        from .api import OPERATIONS\n        return OPERATIONS\n",
            self.name
        )
    }
    pub fn sources(
        &self,
        exports: &BTreeMap<String, String>,
    ) -> Result<(BTreeMap<String, ApiSource>, Vec<String>)> {
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
            "# Generated operation registry. Dispatch is prepared by codegen and bound on import.\nfrom __future__ import annotations\n\nimport typing as _typing\n\nfrom typing_extensions import TypeAliasType as _TypeAliasType, TypeVar as _TypeVar\nfrom jsoncompat import JsonValue as _JsonValue\nfrom jsoncompat import http as _http\nfrom jsoncompat.codegen import http as _http_codegen\n\n",
        );
        writeln!(source, "from ._api import {0}API as {0}API", self.name)?;
        let mut files = BTreeMap::new();
        let mut resources: BTreeMap<&str, Vec<&Operation>> = BTreeMap::new();
        for operation in &self.operations {
            resources
                .entry(&operation.resource)
                .or_default()
                .push(operation);
        }
        for (module, operations) in &resources {
            let mut declarations = self.operation_source(operations, exports, module)?;
            for (alias, (target, owner)) in &self.aliases {
                if owner == module {
                    writeln!(
                        declarations.imports,
                        "from ..{} import {target} as {alias}",
                        exports[target]
                    )?;
                }
            }
            files.insert(format!("{}.py", module.replace('.', "/")), declarations);
            writeln!(source, "from .{module} import (")?;
            for operation in operations {
                for alias in operation.alias_names() {
                    writeln!(source, "    {alias} as {alias},")?;
                }
            }
            writeln!(
                source,
                "    OPERATIONS as _operations_{0},\n    _JSONCOMPAT_OPERATIONS as _plans_{0},\n)",
                module
                    .strip_prefix("operations.")
                    .expect("operation resource module")
            )?;
        }
        writeln!(
            source,
            "\n_TransportOptions = _TypeVar(\"_TransportOptions\", default=_typing.Never)\n{0}Client = _TypeAliasType(\n    \"{0}Client\",\n    _http.Client[{0}API, _TransportOptions],\n    type_params=(_TransportOptions,),\n)\n{0}AsyncClient = _TypeAliasType(\n    \"{0}AsyncClient\",\n    _http.AsyncClient[{0}API, _TransportOptions],\n    type_params=(_TransportOptions,),\n)",
            self.name
        )?;
        writeln!(
            source,
            "\nSECURITY_SCHEMES: _typing.Final[dict[str, _JsonValue]] = {}",
            python_value(&self.security_schemes)
        )?;
        writeln!(
            source,
            "\nOPERATIONS: _typing.Final[tuple[_http.Operation, ...]] = tuple(\n    _operation\n    for _group in (\n{}    )\n    for _operation in _group\n)\n",
            resources
                .keys()
                .map(|module| format!(
                    "        _operations_{},\n",
                    module
                        .strip_prefix("operations.")
                        .expect("operation resource module")
                ))
                .collect::<String>()
        )?;
        writeln!(
            source,
            "_http_codegen.install_api(\n    {}API,\n    1,\n    tuple(\n        _plan\n        for _group in (\n{}        )\n        for _plan in _group\n    ),\n)\n",
            self.name,
            resources
                .keys()
                .map(|module| format!(
                    "            _plans_{},\n",
                    module
                        .strip_prefix("operations.")
                        .expect("operation resource module")
                ))
                .collect::<String>()
        )?;
        writeln!(
            source,
            "__all__: tuple[str, ...] = (\n{})",
            public
                .iter()
                .map(|s| format!("    {},\n", py(s)))
                .collect::<String>()
        )?;
        files.insert(
            "api.py".into(),
            ApiSource {
                imports: String::new(),
                public: source,
                private: String::new(),
            },
        );
        files.insert(
            "_api.py".into(),
            ApiSource {
                imports: String::new(),
                public: self.identity_source(),
                private: String::new(),
            },
        );
        Ok((files, public))
    }

    fn operation_source(
        &self,
        operations: &[&Operation],
        exports: &BTreeMap<String, String>,
        module: &str,
    ) -> Result<ApiSource> {
        let RuntimeNames {
            http,
            codegen,
            type_form,
            ..
        } = self.runtime_names;
        let imports = format!(
            "import typing\nfrom typing_extensions import TypeForm as {type_form}\nfrom jsoncompat import http as {http}\nfrom jsoncompat.codegen import http as {codegen}\n",
        );
        let mut source = String::from("# --- Public operation metadata ---\n");
        let imported: BTreeSet<_> = operations
            .iter()
            .flat_map(|o| std::iter::once(&o.input).chain(o.responses.iter().map(|r| &r.model)))
            .collect();
        for name in &imported {
            if exports.get(*name).is_some_and(|owner| owner == module) {
                continue;
            }
            writeln!(
                source,
                "from ..{} import {name} as {name}",
                exports
                    .get(*name)
                    .with_context(|| format!("missing generated export {name}"))?
            )?;
        }
        writeln!(
            source,
            "\nOPERATIONS: typing.Final[tuple[{http}.Operation, ...]] = ("
        )?;
        for operation in operations {
            let contract = &operation.contract;
            let responses = operation
                .responses
                .iter()
                .map(
                    |ResponseModel {
                         status,
                         model,
                         media,
                         codec,
                         ..
                     }| {
                        format!(
                            "{http}.Response({}, {model}{}{})",
                            status
                                .parse::<u16>()
                                .map(|n| n.to_string())
                                .unwrap_or_else(|_| py(status)),
                            media
                                .as_ref()
                                .map(|m| format!(", {}", py(m)))
                                .unwrap_or_default(),
                            if *codec == "json"
                                && media
                                    .as_ref()
                                    .is_none_or(|m| m.starts_with("application/json")
                                        || m.split(';')
                                            .next()
                                            .is_some_and(|m| m.ends_with("+json")))
                            {
                                String::new()
                            } else {
                                format!(", codec={}", py(codec))
                            }
                        )
                    },
                )
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
                                "{http}.SecurityRequirement({}{})",
                                py(name),
                                if scopes.is_empty() {
                                    String::new()
                                } else {
                                    format!(", {}", tuple(&scopes))
                                }
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
                        "{http}.Server({}{})",
                        py(server["url"].as_str().context("server URL")?),
                        if variables.is_empty() {
                            String::new()
                        } else {
                            format!(", {}", tuple(&variables))
                        }
                    ))
                })
                .collect::<Result<Vec<_>>>()?;
            writeln!(
                source,
                "    {http}.operation(\n        {},\n        {type_form}({}),\n        {type_form}({}),\n        method={},\n        path={},\n        responses=(\n{}        ),",
                operation.input,
                operation.success_alias,
                operation.error_alias,
                py(&contract.key.method),
                py(&contract.key.path),
                responses
                    .iter()
                    .map(|response| format!("            {response},\n"))
                    .collect::<String>(),
            )?;
            if let Some(id) = &contract.operation_id {
                writeln!(source, "        operation_id={},", py(id))?;
            }
            if !security.is_empty() {
                writeln!(source, "        security={},", tuple(&security))?;
            }
            if !servers.is_empty() {
                writeln!(source, "        servers={},", tuple(&servers))?;
            }
            writeln!(source, "    ),")?;
        }
        writeln!(source, ")\n")?;
        let public = source;
        let mut source = String::new();
        writeln!(
            source,
            "# --- Prepared HTTP dispatch ---\n# Prevalidated dispatch; importing this module does not inspect annotations.\n_JSONCOMPAT_OPERATIONS: typing.Final[tuple[{codegen}.OperationPlan, ...]] = ("
        )?;
        for (index, operation) in operations.iter().enumerate() {
            let fields = self.prepared_fields(&operation.input);
            let responses = operation.responses.iter().enumerate().map(|(response_index, response)| {
                format!("{codegen}.ResponsePlan(OPERATIONS[{index}].endpoint.responses[{response_index}], {})", self.prepared_fields(&response.headers))
            }).collect::<Vec<_>>();
            // Preserve response order while deduplicating media, as dict.fromkeys
            // does in the reflective runtime used for manually declared APIs.
            let mut accepted = Vec::new();
            for response in &operation.responses {
                if let Some(media) = &response.media
                    && !accepted.contains(&media.as_str())
                {
                    accepted.push(media.as_str());
                }
            }
            let accept = if accepted.is_empty() {
                "None".into()
            } else {
                py(&accepted.join(", "))
            };
            let path = tuple(
                &operation
                    .path_template
                    .iter()
                    .map(|(parameter, value)| {
                        format!(
                            "({}, {})",
                            if *parameter { "True" } else { "False" },
                            py(value)
                        )
                    })
                    .collect::<Vec<_>>(),
            );
            writeln!(
                source,
                "    {codegen}.OperationPlan(OPERATIONS[{index}].endpoint, {}, {fields}, {}, {accept}, {path}),",
                operation.input,
                tuple(&responses)
            )?;
        }
        writeln!(source, ")\n")?;
        Ok(ApiSource {
            imports,
            public,
            private: source,
        })
    }

    fn prepared_fields(&self, model: &str) -> String {
        let codegen = self.runtime_names.codegen;
        tuple(
            &self.metadata.fields[model]
                .iter()
                .map(|(field, metadata)| format!("{codegen}.Field({}, {metadata})", py(field)))
                .collect::<Vec<_>>(),
        )
    }
}

/// Match urllib.parse.quote(..., safe="/:@!$&'()*+,;=%") on literal segments;
/// parameter names are kept verbatim for lookup after parameter serialization.
fn prepare_path_template(path: &str) -> Result<Vec<(bool, String)>> {
    let mut parts = Vec::new();
    let mut remaining = path;
    loop {
        let (literal, parameter) = remaining
            .split_once('{')
            .map_or((remaining, None), |(a, b)| (a, Some(b)));
        ensure!(!literal.contains('}'), "unbalanced path template");
        let mut escaped = String::new();
        for (index, byte) in literal.bytes().enumerate() {
            if byte == b'%' {
                ensure!(
                    literal
                        .as_bytes()
                        .get(index + 1..index + 3)
                        .is_some_and(|digits| digits.iter().all(u8::is_ascii_hexdigit)),
                    "invalid percent escape in endpoint path"
                );
            }
            if byte.is_ascii_alphanumeric() || b"-._~/:@!$&'()*+,;=%".contains(&byte) {
                escaped.push(char::from(byte));
            } else {
                write!(escaped, "%{byte:02X}")?;
            }
        }
        parts.push((false, escaped));
        let Some(parameter) = parameter else { break };
        let (name, rest) = parameter
            .split_once('}')
            .context("unbalanced path template")?;
        ensure!(
            !name.is_empty() && !name.contains('{'),
            "unbalanced path template"
        );
        parts.push((true, name.into()));
        remaining = rest;
    }
    Ok(parts)
}

fn resource_name(operation: &ApiOperation) -> String {
    operation
        .operation_id
        .as_deref()
        .and_then(|id| id.split_once('/').map(|(resource, _)| resource))
        .filter(|s| !s.is_empty())
        .or_else(|| {
            operation
                .key
                .path
                .split('/')
                .find(|s| !s.is_empty() && !s.starts_with('{'))
        })
        .unwrap_or("root")
        .to_owned()
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
fn python_value(value: &Value) -> String {
    match value {
        Value::Null => "None".into(),
        Value::Bool(value) => if *value { "True" } else { "False" }.into(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => py(value),
        Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(python_value)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Value::Object(values) => format!(
            "{{{}}}",
            values
                .iter()
                .map(|(key, value)| format!("{}: {}", py(key), python_value(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
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
    let value = if [
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
    };
    jsoncompat_codegen::python_names::field_name(&value)
}
struct MediaProjection {
    schema: Value,
    content_type: String,
    codec: &'static str,
    encoding: Vec<String>,
}
impl MediaProjection {
    fn body_metadata(&self, http: &str) -> String {
        if self.content_type == "application/json" && self.codec == "json" {
            return format!("{http}.Body()");
        }
        let mut args = vec![py(&self.content_type)];
        if self.codec != "json"
            || !(self.content_type.starts_with("application/json")
                || self
                    .content_type
                    .split(';')
                    .next()
                    .is_some_and(|m| m.ends_with("+json")))
        {
            args.push(format!("codec={}", py(self.codec)));
        }
        if !self.encoding.is_empty() {
            args.push(format!(
                "encoding={}",
                tuple(
                    &self
                        .encoding
                        .iter()
                        .map(|field| field.replace("$HTTP", http))
                        .collect::<Vec<_>>()
                )
            ));
        }
        format!("{http}.Body({})", args.join(", "))
    }
}

fn media_name(media: &ApiMedia) -> String {
    let mut result = media.media_type.clone();
    for (name, value) in &media.parameters {
        result.push_str(&format!(
            "; {name}=\"{}\"",
            value.replace('\\', "\\\\").replace('"', "\\\"")
        ));
    }
    result
}

fn string_schema(schema: &Value) -> bool {
    schema.get("type").and_then(Value::as_str) == Some("string")
        || schema
            .get("enum")
            .and_then(Value::as_array)
            .is_some_and(|v| v.iter().all(Value::is_string))
}
fn binary_schema(schema: &Value) -> Result<Value> {
    ensure!(
        schema == &Value::Bool(true)
            || schema
                .as_object()
                .is_some_and(
                    |object| object.iter().all(|(key, value)| match key.as_str() {
                        "type" => value == "string",
                        "format" => value == "binary",
                        "title" | "description" | "default" | "examples" | "example"
                        | "deprecated" | "readOnly" | "writeOnly" | "$comment" => true,
                        key if key.starts_with("x-") => true,
                        _ => false,
                    })
                ),
        "opaque binary media require an unconstrained schema or an unconstrained string with format: binary; string assertions cannot be interpreted as byte constraints"
    );
    let mut value = schema.as_object().cloned().unwrap_or_default();
    value.remove("format");
    value.insert("type".into(), json!("string"));
    value.insert("x-jsoncompat-http-binary".into(), json!(true));
    Ok(Value::Object(value))
}

fn binary_reference_schema<'a>(mut schema: &'a Value, definitions: &'a Value) -> Result<&'a Value> {
    let mut seen = BTreeSet::new();
    while let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        ensure!(seen.insert(reference), "recursive binary schema reference");
        ensure!(
            schema
                .as_object()
                .is_some_and(|object| object.keys().all(|key| matches!(
                    key.as_str(),
                    "$ref"
                        | "title"
                        | "description"
                        | "default"
                        | "example"
                        | "examples"
                        | "deprecated"
                        | "readOnly"
                        | "writeOnly"
                        | "$comment"
                ))),
            "binary schema references cannot have adjacent validation assertions"
        );
        schema = reference
            .strip_prefix('#')
            .and_then(|r| definitions.pointer(r))
            .context("unresolved binary schema reference")?;
    }
    Ok(schema)
}

fn project_binary_property(schema: &Value, definitions: &Value) -> Result<(Value, bool)> {
    let resolved = resolve_schema(schema, definitions)?;
    if resolved.get("format").and_then(Value::as_str) == Some("binary") {
        return Ok((
            binary_schema(binary_reference_schema(schema, definitions)?)?,
            true,
        ));
    }
    if resolved.get("type").and_then(Value::as_str) == Some("array")
        && let Some(items) = resolved.get("items")
    {
        let (item, binary) = project_binary_property(items, definitions)?;
        if binary {
            binary_reference_schema(schema, definitions)?;
            let mut projected = resolved.clone();
            projected["items"] = item;
            return Ok((projected, true));
        }
    }
    // Nullable file fields are common in FastAPI. A string-or-file union stays
    // a string, since format annotations alone do not distinguish those branches.
    if let Some(branches) = resolved.get("anyOf").and_then(Value::as_array)
        && branches.iter().all(|branch| {
            branch.get("type").and_then(Value::as_str) == Some("null")
                || branch.get("format").and_then(Value::as_str) == Some("binary")
        })
    {
        binary_reference_schema(schema, definitions)?;
        let branches = branches
            .iter()
            .map(|branch| project_binary_property(branch, definitions).map(|p| p.0))
            .collect::<Result<Vec<_>>>()?;
        let mut value = resolved.clone();
        value["anyOf"] = json!(branches);
        return Ok((value, true));
    }
    Ok((schema.clone(), false))
}

fn project_form_schema(
    schema: &Value,
    definitions: &Value,
    properties: &mut BTreeMap<String, (Value, bool)>,
    stack: &mut BTreeSet<String>,
    media: &ApiMedia,
) -> Result<Value> {
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        ensure!(
            stack.insert(reference.into()),
            "recursive form body schema is not supported"
        );
        let resolved = reference
            .strip_prefix('#')
            .and_then(|r| definitions.pointer(r))
            .context("unresolved form schema reference")?;
        let projected = project_form_schema(resolved, definitions, properties, stack, media)?;
        stack.remove(reference);
        let mut siblings = schema.as_object().expect("reference object").clone();
        siblings.remove("$ref");
        if siblings.is_empty() {
            return Ok(projected);
        }
        let siblings = project_form_schema(
            &Value::Object(siblings),
            definitions,
            properties,
            stack,
            media,
        )?;
        return Ok(json!({"allOf":[projected, siblings]}));
    }
    let mut projected = schema.clone();
    if let Some(branches) = schema.get("allOf").and_then(Value::as_array) {
        projected["allOf"] = json!(
            branches
                .iter()
                .map(|branch| project_form_schema(branch, definitions, properties, stack, media))
                .collect::<Result<Vec<_>>>()?
        );
    }
    if let Some(fields) = schema.get("properties").and_then(Value::as_object) {
        for (name, field) in fields {
            let content_type = media
                .encoding
                .as_ref()
                .and_then(|encodings| encodings.get(name))
                .filter(|encoding| !encoding.uses_parameter_encoding)
                .and_then(|encoding| encoding.content_type.as_deref())
                .filter(|_| media.media_type == "multipart/form-data");
            let structured = content_type.is_some_and(|value| {
                let base = value
                    .split(';')
                    .next()
                    .unwrap_or(value)
                    .trim()
                    .to_ascii_lowercase();
                base == "application/json"
                    || base.ends_with("+json")
                    || [
                        "application/yaml",
                        "text/yaml",
                        "application/x-yaml",
                        "application/vnd.oai.openapi",
                        "application/x-ndjson",
                        "application/ndjson",
                    ]
                    .contains(&base.as_str())
            });
            if content_type.is_some_and(|value| {
                matches!(
                    value
                        .split(';')
                        .next()
                        .unwrap_or(value)
                        .trim()
                        .to_ascii_lowercase()
                        .as_str(),
                    "application/x-ndjson" | "application/ndjson"
                )
            }) {
                ensure!(
                    string_schema(resolve_schema(field, definitions)?),
                    "NDJSON multipart parts require an explicit string schema; structured record parts need an explicit multipart representation"
                );
            }
            // A JSON/YAML media representation retains the schema's JSON type.
            // format:binary is an annotation, not permission to bypass its codec.
            let (value, binary) = if structured {
                (field.clone(), false)
            } else {
                project_binary_property(field, definitions)?
            };
            properties.insert(name.clone(), (value.clone(), binary));
            projected["properties"][name] = value;
        }
    }
    Ok(projected)
}

fn project_media(media: &ApiMedia, definitions: &Value, response: bool) -> Result<MediaProjection> {
    validate_media_parameters(&media.parameters)?;
    ensure!(
        response || !media.media_type.contains('*'),
        "request media selectors must name a concrete content type"
    );
    let resolved = resolve_schema(&media.schema, definitions)?;
    let mut schema = media.schema.clone();
    let mut encoding = Vec::new();
    let codec = if media.media_type == "multipart/form-data"
        || media.media_type == "application/x-www-form-urlencoded"
    {
        ensure!(!response, "form response bodies are not supported");
        ensure!(
            !media.parameters.contains_key("boundary"),
            "multipart boundaries are selected by the encoder; omit a fixed boundary parameter"
        );
        let mut properties = BTreeMap::new();
        schema = project_form_schema(
            &schema,
            definitions,
            &mut properties,
            &mut BTreeSet::new(),
            media,
        )?;
        ensure!(
            resolved.get("type").and_then(Value::as_str) == Some("object")
                || resolved.get("properties").is_some()
                || resolved.get("allOf").is_some(),
            "form media require an object schema"
        );
        for (name, (_, binary)) in properties {
            let mut args = vec![py(&name)];
            if binary {
                ensure!(
                    media.media_type == "multipart/form-data",
                    "binary form values require multipart/form-data"
                );
                args.push("binary=True".into());
            }
            if let Some(field) = media.encoding.as_ref().and_then(|fields| fields.get(&name)) {
                ensure!(
                    field.headers.is_empty()
                        || media.media_type == "application/x-www-form-urlencoded",
                    "custom multipart part headers require an explicit per-part input model and are not yet supported"
                );
                if let Some(content_type) = &field.content_type
                    && !field.uses_parameter_encoding
                    && media.media_type == "multipart/form-data"
                {
                    ensure!(
                        !content_type.contains(['*', ',', '\r', '\n']) && content_type.is_ascii(),
                        "multipart contentType must name one concrete ASCII media type"
                    );
                    args.push(format!("content_type={}", py(content_type)));
                }
                ensure!(
                    field.style.as_str() != "deepObject" || field.explode,
                    "deepObject requires explode=true"
                );
                ensure!(
                    !matches!(field.style.as_str(), "spaceDelimited" | "pipeDelimited")
                        || !field.explode,
                    "delimited form arrays require explode=false"
                );
                ensure!(
                    !binary || !field.uses_parameter_encoding,
                    "binary multipart fields cannot use parameter serialization controls"
                );
                if field.uses_parameter_encoding {
                    args.push("parameter_encoding=True".into());
                }
                if field.style.as_str() != "form" {
                    args.push(format!("style={}", py(field.style.as_str())));
                }
                if !field.explode {
                    args.push("explode=False".into());
                }
                if field.allow_reserved {
                    args.push("allow_reserved=True".into());
                }
            }
            encoding.push(format!("$HTTP.FormField({})", args.join(", ")));
        }
        if media.media_type == "multipart/form-data" {
            "multipart"
        } else {
            "urlencoded"
        }
    } else if media.media_type == "application/json" || media.media_type.ends_with("+json") {
        "json"
    } else if media.media_type == "application/x-ndjson" || media.media_type == "application/ndjson"
    {
        if string_schema(resolved) {
            "text"
        } else {
            schema = json!({"type":"array", "items":schema});
            "ndjson"
        }
    } else if [
        "application/yaml",
        "text/yaml",
        "application/x-yaml",
        "application/vnd.oai.openapi",
    ]
    .contains(&media.media_type.as_str())
    {
        "yaml"
    } else if resolved.get("format").and_then(Value::as_str) == Some("binary") {
        schema = binary_schema(binary_reference_schema(&media.schema, definitions)?)?;
        "binary"
    } else if media.media_type.starts_with("text/")
        || ["application/xml", "application/javascript"].contains(&media.media_type.as_str())
    {
        if matches!(
            resolved.get("type").and_then(Value::as_str),
            Some("boolean" | "integer" | "number" | "null")
        ) {
            // Scalar text uses the canonical JSON lexical forms for booleans,
            // numbers and null; string bodies remain unquoted UTF-8 text.
            "json"
        } else {
            ensure!(
                string_schema(resolved)
                    || resolved == &Value::Bool(true)
                    || resolved.as_object().is_some_and(Map::is_empty),
                "text media {} require a scalar schema",
                media.media_type
            );
            if !string_schema(resolved) {
                schema = json!({"type":"string"});
            }
            "text"
        }
    } else {
        schema = binary_schema(binary_reference_schema(&media.schema, definitions)?)
            .with_context(|| format!("unsupported structured media {}", media.media_type))?;
        "binary"
    };
    Ok(MediaProjection {
        schema,
        content_type: media_name(media),
        codec,
        encoding,
    })
}
fn parameter_metadata(
    parameter: &ApiParameter,
    response: bool,
    definitions: &Value,
    http: &str,
) -> Result<String> {
    ensure!(
        !parameter.name.is_empty() && !parameter.name.contains(['\r', '\n']),
        "invalid parameter name"
    );
    if parameter.location == ParameterLocation::Header {
        ensure!(
            parameter
                .name
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"!#$%&'*+.^_`|~-".contains(&c)),
            "invalid HTTP header name {}",
            parameter.name
        );
    }
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
                    if class != "Cookie" && style.as_str() != "simple" {
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
                    if *allow_reserved {
                        args.push("allow_reserved=True".into());
                    }
                    if *allow_empty_value {
                        args.push("allow_empty_value=True".into());
                    }
                    ensure!(
                        !matches!(style.as_str(), "spaceDelimited" | "pipeDelimited") || !explode,
                        "delimited query arrays require explode=false"
                    );
                    ensure!(
                        style.as_str() != "deepObject" || *explode,
                        "deepObject requires explode=true"
                    );
                    if style.as_str() != "form" {
                        args.push(format!("style={}", py(style.as_str())));
                    }
                    *explode
                }
                SchemaSerialization::Header { explode } => *explode,
            };
            let default_explode = matches!(
                parameter.location,
                ParameterLocation::Query | ParameterLocation::Cookie
            );
            if explode != default_explode {
                args.push(format!(
                    "explode={}",
                    if explode { "True" } else { "False" }
                ));
            }
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
                if kind != "string" {
                    args.push(format!("kind={}", py(kind)));
                }
                if array {
                    args.push("array=True".into());
                }
            }
        }
    }
    Ok(format!("{http}.{class}({})", args.join(", ")))
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
            .all(|(name, value)| !name.contains(['\r', '\n'])
                && !value.chars().any(char::is_control)),
        "invalid media parameter"
    );
    if let Some(charset) = parameters.get("charset") {
        ensure!(
            matches!(charset.to_ascii_lowercase().as_str(), "utf-8" | "utf8"),
            "API generation supports only UTF-8 text and JSON charsets"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_aliases_avoid_nested_schema_declaration_names() {
        let raw = json!({"components":{"schemas":{"Payload":{"$defs":{
            "Helper":{"x-jsoncompat":{"kind":"declaration", "name":"_http_codegen"}}
        }, "description":"_http and _api remain literal description text"}}}});
        let aliases = RuntimeNames::for_document(&raw, &[]);
        assert_eq!(aliases.codegen, "_jsoncompat_http_codegen");
        assert_eq!(aliases.http, "_http");
        assert_eq!(aliases.type_form, "_TypeForm");
    }
}
