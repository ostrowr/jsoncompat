use crate::error::CodegenError;
use crate::model::{
    AdditionalProperties, FieldSpec, ModelGraph, ModelRole, ModelSpec, SchemaType, StringFormat,
};
use crate::strings::sanitize_field_name;
use crate::{build_model_graph, ModelGenerator};
use json_schema_ast::SchemaNode;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

#[derive(Debug, Clone)]
pub struct PydanticOptions {
    pub root_model_name: String,
    pub serializer_suffix: String,
    pub deserializer_suffix: String,
    pub base_module: Option<String>,
    pub allow_non_object_inputs: bool,
}

impl Default for PydanticOptions {
    fn default() -> Self {
        Self {
            root_model_name: "Model".to_string(),
            serializer_suffix: "Serializer".to_string(),
            deserializer_suffix: "Deserializer".to_string(),
            base_module: None,
            allow_non_object_inputs: false,
        }
    }
}

impl PydanticOptions {
    pub fn with_root_model_name(mut self, name: impl Into<String>) -> Self {
        self.root_model_name = name.into();
        self
    }

    pub fn with_base_module(mut self, module: impl Into<String>) -> Self {
        self.base_module = Some(module.into());
        self
    }

    pub fn with_allow_non_object_inputs(mut self, allow: bool) -> Self {
        self.allow_non_object_inputs = allow;
        self
    }
}

#[derive(Debug, Clone)]
pub struct PydanticGenerator {
    options: PydanticOptions,
}

pub fn generate_model(
    schema: &SchemaNode,
    role: ModelRole,
    options: PydanticOptions,
) -> Result<String, CodegenError> {
    let generator = PydanticGenerator::new(options);
    generator.generate_model(schema, role)
}

impl PydanticGenerator {
    pub fn new(options: PydanticOptions) -> Self {
        Self { options }
    }

    pub fn generate(&self, graph: &ModelGraph, role: ModelRole) -> Result<String, CodegenError> {
        let (ordered_models, needs_rebuild) = topo_sort_models(&graph.models);
        let mut ctx = PyContext::new(&self.options);

        let mut out = CodeWriter::new();

        ctx.imports.add("pydantic", "ConfigDict");
        ctx.imports.add("pydantic", "Field");

        let needs_model_validator = graph
            .models
            .values()
            .any(|model| model.min_properties.is_some() || model.max_properties.is_some());
        if needs_model_validator {
            ctx.imports.add("pydantic", "model_validator");
        }

        if let Some(base_module) = &self.options.base_module {
            ctx.imports.add(base_module, "SerializerBase");
            ctx.imports.add(base_module, "DeserializerBase");
        } else {
            ctx.imports.add("pydantic", "BaseModel");
            match role {
                ModelRole::Serializer => emit_serializer_base(&mut out),
                ModelRole::Deserializer => emit_deserializer_base(&mut out),
            }
        }

        for model_name in ordered_models {
            let model =
                graph
                    .models
                    .get(&model_name)
                    .ok_or_else(|| CodegenError::InvalidSchema {
                        location: crate::SchemaPath::root(),
                        message: format!("missing model {model_name}"),
                    })?;
            emit_model(
                &mut out,
                &mut ctx,
                model,
                role,
                model.name == graph.root && self.options.allow_non_object_inputs,
            )?;
        }

        if needs_rebuild {
            out.push_empty();
            for model_name in graph.models.keys() {
                let class_name = ctx.class_name(model_name, role);
                out.push_line(&format!("{class_name}.model_rebuild()"));
            }
        }

        let mut rendered = String::new();
        rendered.push_str(&ctx.imports.render());
        rendered.push_str(&out.finish());
        Ok(rendered)
    }
}

impl ModelGenerator for PydanticGenerator {
    fn generate_model(&self, schema: &SchemaNode, role: ModelRole) -> Result<String, CodegenError> {
        let graph = build_model_graph(schema, &self.options.root_model_name)?;
        self.generate(&graph, role)
    }
}

/// Source for the reusable Pydantic base classes used by generated models.
pub fn base_module() -> String {
    let mut imports = ImportSet::new();
    imports.add("pydantic", "BaseModel");
    imports.add("pydantic", "ConfigDict");

    let mut out = CodeWriter::new();
    emit_serializer_base(&mut out);
    emit_deserializer_base(&mut out);

    let mut rendered = String::new();
    rendered.push_str(&imports.render());
    rendered.push_str(&out.finish());
    rendered
}

struct PyContext {
    imports: ImportSet,
    options: PydanticOptions,
}

impl PyContext {
    fn new(options: &PydanticOptions) -> Self {
        Self {
            imports: ImportSet::new(),
            options: options.clone(),
        }
    }

    fn class_name(&self, model: &str, role: ModelRole) -> String {
        match role {
            ModelRole::Serializer => format!("{model}{}", self.options.serializer_suffix),
            ModelRole::Deserializer => format!("{model}{}", self.options.deserializer_suffix),
        }
    }

    fn require_pydantic_undefined(&mut self) {
        self.imports.add("pydantic_core", "PydanticUndefined");
    }

    fn type_expr(&mut self, schema: &SchemaType, role: ModelRole) -> Result<String, CodegenError> {
        match schema {
            SchemaType::Any => {
                self.imports.add("typing", "Any");
                Ok("Any".to_string())
            }
            SchemaType::Bool => Ok("bool".to_string()),
            SchemaType::String(constraints) => {
                let base = match constraints.format {
                    Some(StringFormat::DateTime) => {
                        self.imports.add("datetime", "datetime");
                        "datetime".to_string()
                    }
                    Some(StringFormat::Date) => {
                        self.imports.add("datetime", "date");
                        "date".to_string()
                    }
                    Some(StringFormat::Time) => {
                        self.imports.add("datetime", "time");
                        "time".to_string()
                    }
                    Some(StringFormat::Uuid) => {
                        self.imports.add("uuid", "UUID");
                        "UUID".to_string()
                    }
                    Some(StringFormat::Email) => {
                        self.imports.add("pydantic", "EmailStr");
                        "EmailStr".to_string()
                    }
                    Some(StringFormat::Uri) => {
                        self.imports.add("pydantic", "AnyUrl");
                        "AnyUrl".to_string()
                    }
                    None => "str".to_string(),
                };
                Ok(self.maybe_annotated(base, constraint_args_for_string(constraints)?))
            }
            SchemaType::Integer(constraints) => {
                Ok(self
                    .maybe_annotated("int".to_string(), constraint_args_for_number(constraints)?))
            }
            SchemaType::Number(constraints) => Ok(self.maybe_annotated(
                "float".to_string(),
                constraint_args_for_number(constraints)?,
            )),
            SchemaType::Null => Ok("None".to_string()),
            SchemaType::Array { items, constraints } => {
                let inner = self.type_expr(items, role)?;
                let base = format!("list[{inner}]");
                Ok(self.maybe_annotated(base, constraint_args_for_array(constraints)?))
            }
            SchemaType::Map {
                values,
                constraints,
            } => {
                let inner = self.type_expr(values, role)?;
                let base = format!("dict[str, {inner}]");
                Ok(self.maybe_annotated(base, constraint_args_for_object(constraints)?))
            }
            SchemaType::Object(name) => Ok(self.class_name(name, role)),
            SchemaType::Literal(values) => {
                self.imports.add("typing", "Literal");
                let rendered = values
                    .iter()
                    .map(literal_value)
                    .collect::<Result<Vec<_>, CodegenError>>()?
                    .join(", ");
                Ok(format!("Literal[{rendered}]"))
            }
            SchemaType::Union(variants) => {
                let rendered = variants
                    .iter()
                    .map(|variant| self.type_expr(variant, role))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(" | ");
                Ok(rendered)
            }
            SchemaType::Nullable(inner) => {
                let inner = self.type_expr(inner, role)?;
                Ok(format!("{inner} | None"))
            }
        }
    }

    fn maybe_annotated(&mut self, base: String, args: Vec<FieldArg>) -> String {
        if args.is_empty() {
            return base;
        }
        self.imports.add("typing", "Annotated");
        self.imports.add("pydantic", "Field");
        let rendered = render_field_args(&args);
        format!("Annotated[{base}, Field({rendered})]")
    }
}

struct PythonFieldNames {
    mapping: HashMap<String, String>,
}

impl PythonFieldNames {
    fn new(fields: &[FieldSpec]) -> Self {
        let mut used = HashSet::new();
        let mut mapping = HashMap::new();

        let mut names: Vec<&String> = fields.iter().map(|f| &f.name).collect();
        names.sort();

        for name in names {
            let mut candidate = sanitize_field_name(name);
            if is_reserved_field_name(&candidate) {
                candidate = format!("field_{candidate}");
            }
            let base = candidate.clone();
            let mut idx = 2;
            while used.contains(&candidate) {
                candidate = format!("{base}_{idx}");
                idx += 1;
            }
            used.insert(candidate.clone());
            mapping.insert(name.clone(), candidate);
        }

        Self { mapping }
    }

    fn field_name<'a>(&'a self, json_name: &'a str) -> &'a str {
        self.mapping
            .get(json_name)
            .map(|s| s.as_str())
            .unwrap_or(json_name)
    }

    fn needs_alias(&self, json_name: &str) -> bool {
        self.field_name(json_name) != json_name
    }
}

fn emit_serializer_base(out: &mut CodeWriter) {
    out.push_line("class SerializerBase(BaseModel):");
    out.indent();
    out.push_line("model_config = ConfigDict(");
    out.indent();
    out.push_line("validate_by_alias=True,");
    out.push_line("validate_by_name=True,");
    out.push_line("serialize_by_alias=True,");
    out.dedent();
    out.push_line(")");
    out.push_empty();
    out.push_line("def model_dump(self, **kwargs):");
    out.indent();
    out.push_line("kwargs.setdefault(\"exclude_unset\", True)");
    out.push_line("return super().model_dump(**kwargs)");
    out.dedent();
    out.push_empty();
    out.push_line("def model_dump_json(self, **kwargs):");
    out.indent();
    out.push_line("kwargs.setdefault(\"exclude_unset\", True)");
    out.push_line("return super().model_dump_json(**kwargs)");
    out.dedent();
    out.dedent();
    out.push_empty();
}

fn emit_deserializer_base(out: &mut CodeWriter) {
    out.push_line("class DeserializerBase(BaseModel):");
    out.indent();
    out.push_line("model_config = ConfigDict(");
    out.indent();
    out.push_line("validate_by_alias=True,");
    out.push_line("validate_by_name=True,");
    out.push_line("serialize_by_alias=True,");
    out.dedent();
    out.push_line(")");
    out.dedent();
    out.push_empty();
}

fn emit_model(
    out: &mut CodeWriter,
    ctx: &mut PyContext,
    model: &ModelSpec,
    role: ModelRole,
    force_allow_non_objects: bool,
) -> Result<(), CodegenError> {
    let class_name = ctx.class_name(&model.name, role);
    let base = match role {
        ModelRole::Serializer => "SerializerBase",
        ModelRole::Deserializer => "DeserializerBase",
    };

    out.push_line(&format!("class {class_name}({base}):"));
    out.indent();

    if let Some(doc) = model.description.as_ref().or(model.title.as_ref()) {
        out.push_line(&format!("\"\"\"{}\"\"\"", escape_docstring(doc)));
    }

    let extra_value = match &model.additional_properties {
        AdditionalProperties::Allow => "allow",
        AdditionalProperties::Forbid => "forbid",
        AdditionalProperties::Typed(_) => "allow",
    };
    out.push_line(&format!(
        "model_config = ConfigDict(extra=\"{extra_value}\")"
    ));

    if let AdditionalProperties::Typed(schema) = &model.additional_properties {
        let extra_type = ctx.type_expr(schema, role)?;
        out.push_line(&format!("__pydantic_extra__: dict[str, {extra_type}]"));
    }

    let field_names = PythonFieldNames::new(&model.fields);
    let mut fields: Vec<&FieldSpec> = model.fields.iter().collect();
    fields.sort_by(|a, b| a.name.cmp(&b.name));

    for field in fields {
        if !field.include_in_role(role) {
            continue;
        }
        emit_field(out, ctx, field, &field_names, role)?;
    }

    let allow_non_objects = model.allow_non_objects || force_allow_non_objects;

    if allow_non_objects {
        ctx.imports.add("pydantic", "model_validator");
        emit_non_object_bypass(out)?;
    }

    if model.min_properties.is_some() || model.max_properties.is_some() {
        emit_property_validator(out, model)?;
    }

    out.dedent();
    out.push_empty();
    Ok(())
}

fn emit_field(
    out: &mut CodeWriter,
    ctx: &mut PyContext,
    field: &FieldSpec,
    names: &PythonFieldNames,
    role: ModelRole,
) -> Result<(), CodegenError> {
    let field_name = names.field_name(&field.name);
    let type_expr = ctx.type_expr(&field.schema, role)?;
    let required = field.required_for_role(role);

    let mut args = Vec::new();
    if names.needs_alias(&field.name) {
        args.push(FieldArg::new("alias", python_string_literal(&field.name)?));
    }
    if let Some(title) = &field.title {
        args.push(FieldArg::new("title", python_string_literal(title)?));
    }
    if let Some(desc) = &field.description {
        args.push(FieldArg::new("description", python_string_literal(desc)?));
    }

    if required {
        if args.is_empty() {
            out.push_line(&format!("{field_name}: {type_expr}"));
            return Ok(());
        }
        let rendered = render_field_args(&args);
        out.push_line(&format!("{field_name}: {type_expr} = Field({rendered})"));
        return Ok(());
    }

    if let Some(default) = field.default_for_role(role) {
        match default_expression(default)? {
            DefaultExpr::Value(expr) => {
                args.push(FieldArg::new("default", expr));
            }
            DefaultExpr::Factory(factory) => {
                args.push(FieldArg::new("default_factory", factory));
            }
        }
    } else {
        ctx.require_pydantic_undefined();
        args.push(FieldArg::new(
            "default_factory",
            "lambda: PydanticUndefined".to_string(),
        ));
    }

    let rendered = render_field_args(&args);
    out.push_line(&format!("{field_name}: {type_expr} = Field({rendered})"));
    Ok(())
}

fn emit_property_validator(out: &mut CodeWriter, model: &ModelSpec) -> Result<(), CodegenError> {
    out.push_empty();
    out.push_line("@model_validator(mode=\"after\")");
    out.push_line("def _check_properties(self):");
    out.indent();
    out.push_line("if getattr(self, \"_jsonschema_codegen_skip_object_checks\", False):");
    out.indent();
    out.push_line("return self");
    out.dedent();
    out.push_line("count = len(self.model_fields_set)");
    out.push_line("extra = getattr(self, \"__pydantic_extra__\", None)");
    out.push_line("if extra:");
    out.indent();
    out.push_line("count += len(extra)");
    out.dedent();

    if let Some(min_props) = model.min_properties {
        out.push_line(&format!("if count < {min_props}:",));
        out.indent();
        out.push_line(&format!(
            "raise ValueError(\"expected at least {min_props} properties\")"
        ));
        out.dedent();
    }

    if let Some(max_props) = model.max_properties {
        out.push_line(&format!("if count > {max_props}:",));
        out.indent();
        out.push_line(&format!(
            "raise ValueError(\"expected at most {max_props} properties\")"
        ));
        out.dedent();
    }

    out.push_line("return self");
    out.dedent();
    Ok(())
}

fn emit_non_object_bypass(out: &mut CodeWriter) -> Result<(), CodegenError> {
    out.push_empty();
    out.push_line("@model_validator(mode=\"wrap\")");
    out.push_line("def _allow_non_objects(cls, value, handler):");
    out.indent();
    out.push_line("if not isinstance(value, dict):");
    out.indent();
    out.push_line("inst = cls.model_construct()");
    out.push_line("setattr(inst, \"_jsonschema_codegen_skip_object_checks\", True)");
    out.push_line("return inst");
    out.dedent();
    out.push_line("return handler(value)");
    out.dedent();
    Ok(())
}

#[derive(Debug, Clone)]
struct FieldArg {
    key: &'static str,
    value: String,
}

impl FieldArg {
    fn new(key: &'static str, value: String) -> Self {
        Self { key, value }
    }
}

fn render_field_args(args: &[FieldArg]) -> String {
    args.iter()
        .map(|arg| format!("{}={}", arg.key, arg.value))
        .collect::<Vec<_>>()
        .join(", ")
}

fn constraint_args_for_string(
    constraints: &crate::model::StringConstraints,
) -> Result<Vec<FieldArg>, CodegenError> {
    let mut args = Vec::new();
    if let Some(min) = constraints.min_length {
        args.push(FieldArg::new("min_length", min.to_string()));
    }
    if let Some(max) = constraints.max_length {
        args.push(FieldArg::new("max_length", max.to_string()));
    }
    if let Some(pattern) = &constraints.pattern {
        args.push(FieldArg::new("pattern", python_string_literal(pattern)?));
    }
    Ok(args)
}

fn constraint_args_for_number(
    constraints: &crate::model::NumberConstraints,
) -> Result<Vec<FieldArg>, CodegenError> {
    let mut args = Vec::new();
    if let Some(min) = constraints.minimum {
        let key = if constraints.exclusive_minimum {
            "gt"
        } else {
            "ge"
        };
        args.push(FieldArg::new(key, float_literal(min)));
    }
    if let Some(max) = constraints.maximum {
        let key = if constraints.exclusive_maximum {
            "lt"
        } else {
            "le"
        };
        args.push(FieldArg::new(key, float_literal(max)));
    }
    if let Some(multiple_of) = constraints.multiple_of {
        args.push(FieldArg::new("multiple_of", float_literal(multiple_of)));
    }
    Ok(args)
}

fn constraint_args_for_array(
    constraints: &crate::model::ArrayConstraints,
) -> Result<Vec<FieldArg>, CodegenError> {
    let mut args = Vec::new();
    if let Some(min) = constraints.min_items {
        args.push(FieldArg::new("min_length", min.to_string()));
    }
    if let Some(max) = constraints.max_items {
        args.push(FieldArg::new("max_length", max.to_string()));
    }
    Ok(args)
}

fn constraint_args_for_object(
    constraints: &crate::model::ObjectConstraints,
) -> Result<Vec<FieldArg>, CodegenError> {
    let mut args = Vec::new();
    if let Some(min) = constraints.min_properties {
        args.push(FieldArg::new("min_length", min.to_string()));
    }
    if let Some(max) = constraints.max_properties {
        args.push(FieldArg::new("max_length", max.to_string()));
    }
    Ok(args)
}

fn literal_value(value: &crate::model::LiteralValue) -> Result<String, CodegenError> {
    match value {
        crate::model::LiteralValue::Null => Ok("None".to_string()),
        crate::model::LiteralValue::Bool(v) => Ok(if *v { "True" } else { "False" }.to_string()),
        crate::model::LiteralValue::String(s) => python_string_literal(s),
        crate::model::LiteralValue::Number(n) => Ok(n.to_string()),
    }
}

enum DefaultExpr {
    Value(String),
    Factory(String),
}

fn default_expression(value: &Value) -> Result<DefaultExpr, CodegenError> {
    match value {
        Value::Array(_) | Value::Object(_) => Ok(DefaultExpr::Factory(format!(
            "lambda: {}",
            python_literal(value)?
        ))),
        _ => Ok(DefaultExpr::Value(python_literal(value)?)),
    }
}

fn python_literal(value: &Value) -> Result<String, CodegenError> {
    match value {
        Value::Null => Ok("None".to_string()),
        Value::Bool(v) => Ok(if *v { "True" } else { "False" }.to_string()),
        Value::Number(n) => Ok(n.to_string()),
        Value::String(s) => python_string_literal(s),
        Value::Array(values) => {
            let rendered = values
                .iter()
                .map(python_literal)
                .collect::<Result<Vec<_>, _>>()?
                .join(", ");
            Ok(format!("[{rendered}]"))
        }
        Value::Object(map) => {
            let mut entries = Vec::new();
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for key in keys {
                let value = map.get(key).expect("key must exist");
                let key_literal = python_string_literal(key)?;
                let val = python_literal(value)?;
                entries.push(format!("{key_literal}: {val}"));
            }
            Ok(format!("{{{}}}", entries.join(", ")))
        }
    }
}

fn python_string_literal(value: &str) -> Result<String, CodegenError> {
    serde_json::to_string(value).map_err(|e| CodegenError::InvalidSchema {
        location: crate::SchemaPath::root(),
        message: format!("failed to serialize string literal: {e}"),
    })
}

fn float_literal(value: f64) -> String {
    let mut s = value.to_string();
    if !s.contains('.') && !s.contains('e') && !s.contains('E') {
        s.push_str(".0");
    }
    s
}

fn escape_docstring(value: &str) -> String {
    value.replace("\"\"\"", "\\\"\\\"\\\"")
}

fn is_reserved_field_name(name: &str) -> bool {
    name.starts_with("__")
        || matches!(
            name,
            "model_config"
                | "model_fields"
                | "model_fields_set"
                | "model_dump"
                | "model_dump_json"
                | "model_validate"
                | "model_copy"
                | "model_rebuild"
                | "model_json_schema"
                | "__pydantic_extra__"
        )
}

#[derive(Default)]
struct ImportSet {
    modules: BTreeMap<String, BTreeSet<String>>,
}

impl ImportSet {
    fn new() -> Self {
        Self::default()
    }

    fn add(&mut self, module: &str, name: &str) {
        self.modules
            .entry(module.to_string())
            .or_default()
            .insert(name.to_string());
    }

    fn render(&self) -> String {
        let mut out = String::new();
        out.push_str("from __future__ import annotations\n\n");

        let mut grouped: BTreeMap<u8, Vec<(&String, &BTreeSet<String>)>> = BTreeMap::new();
        for (module, names) in &self.modules {
            grouped
                .entry(module_group(module))
                .or_default()
                .push((module, names));
        }

        for (group_idx, mut group) in grouped {
            group.sort_by(|a, b| a.0.cmp(b.0));
            let has_items = !group.is_empty();
            for (module, names) in &group {
                let mut sorted: Vec<_> = names.iter().collect();
                sorted.sort();
                let joined = sorted
                    .iter()
                    .map(|name| name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&format!("from {module} import {joined}\n"));
            }
            if group_idx == 0 && has_items {
                out.push('\n');
            }
        }

        out.push('\n');
        out
    }
}

fn module_group(module: &str) -> u8 {
    match module {
        "typing" | "datetime" | "uuid" => 0,
        _ => 1,
    }
}

struct CodeWriter {
    lines: Vec<String>,
    indent: usize,
}

impl CodeWriter {
    fn new() -> Self {
        Self {
            lines: Vec::new(),
            indent: 0,
        }
    }

    fn push_line(&mut self, line: &str) {
        if line.is_empty() {
            self.lines.push(String::new());
            return;
        }
        let indent = "    ".repeat(self.indent);
        self.lines.push(format!("{indent}{line}"));
    }

    fn push_empty(&mut self) {
        self.lines.push(String::new());
    }

    fn indent(&mut self) {
        self.indent += 1;
    }

    fn dedent(&mut self) {
        self.indent = self.indent.saturating_sub(1);
    }

    fn finish(self) -> String {
        let mut out = self.lines.join("\n");
        out.push('\n');
        out
    }
}

fn topo_sort_models(models: &BTreeMap<String, ModelSpec>) -> (Vec<String>, bool) {
    let mut incoming: HashMap<String, usize> = HashMap::new();
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();

    for (name, model) in models {
        let deps = collect_model_deps(model);
        incoming.insert(name.clone(), deps.len());
        for dep in deps {
            adjacency.entry(dep).or_default().push(name.clone());
        }
    }

    let mut queue: VecDeque<String> = incoming
        .iter()
        .filter(|(_, &count)| count == 0)
        .map(|(name, _)| name.clone())
        .collect();
    let mut ordered = Vec::new();

    while let Some(name) = queue.pop_front() {
        ordered.push(name.clone());
        if let Some(children) = adjacency.get(&name) {
            for child in children {
                if let Some(count) = incoming.get_mut(child) {
                    *count -= 1;
                    if *count == 0 {
                        queue.push_back(child.clone());
                    }
                }
            }
        }
    }

    if ordered.len() == models.len() {
        return (ordered, false);
    }

    let mut fallback: Vec<String> = models.keys().cloned().collect();
    fallback.sort();
    (fallback, true)
}

fn collect_model_deps(model: &ModelSpec) -> BTreeSet<String> {
    let mut deps = BTreeSet::new();
    for field in &model.fields {
        collect_schema_deps(&field.schema, &mut deps);
    }
    if let AdditionalProperties::Typed(schema) = &model.additional_properties {
        collect_schema_deps(schema, &mut deps);
    }
    deps
}

fn collect_schema_deps(schema: &SchemaType, deps: &mut BTreeSet<String>) {
    match schema {
        SchemaType::Object(name) => {
            deps.insert(name.clone());
        }
        SchemaType::Array { items, .. } => collect_schema_deps(items, deps),
        SchemaType::Map { values, .. } => collect_schema_deps(values, deps),
        SchemaType::Union(variants) => {
            for variant in variants {
                collect_schema_deps(variant, deps);
            }
        }
        SchemaType::Nullable(inner) => collect_schema_deps(inner, deps),
        _ => {}
    }
}
