use crate::error::CodegenError;
use crate::model::{
    AdditionalProperties, FieldSpec, ModelGraph, ModelRole, ModelSpec, SchemaType, StringFormat,
};
use crate::strings::sanitize_field_name;
use crate::{build_model_graph, ModelGenerator, SchemaPath};
use json_schema_ast::{build_and_resolve_schema, SchemaNode};
use serde_json::Value;
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct PydanticOptions {
    pub root_model_name: String,
    pub serializer_suffix: String,
    pub deserializer_suffix: String,
    pub base_module: Option<String>,
    pub header_comment: Option<String>,
}

impl Default for PydanticOptions {
    fn default() -> Self {
        Self {
            root_model_name: "Model".to_string(),
            serializer_suffix: "Serializer".to_string(),
            deserializer_suffix: "Deserializer".to_string(),
            base_module: None,
            header_comment: None,
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

    pub fn with_header_comment(mut self, comment: impl Into<String>) -> Self {
        self.header_comment = Some(comment.into());
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
    generate_model_from_value(&schema.to_json(), role, options)
}

pub fn generate_model_from_value(
    schema_json: &Value,
    role: ModelRole,
    options: PydanticOptions,
) -> Result<String, CodegenError> {
    let generator = PydanticGenerator::new(options);
    generator.generate_from_value(schema_json, role)
}

impl PydanticGenerator {
    pub fn new(options: PydanticOptions) -> Self {
        Self { options }
    }

    pub fn generate_from_value(
        &self,
        schema_json: &Value,
        role: ModelRole,
    ) -> Result<String, CodegenError> {
        let schema =
            build_and_resolve_schema(schema_json).map_err(|e| CodegenError::InvalidSchema {
                location: crate::SchemaPath::root(),
                message: e.to_string(),
            })?;
        let graph = build_model_graph(&schema, &self.options.root_model_name)?;
        self.generate(&graph, schema_json, role)
    }

    pub fn generate(
        &self,
        graph: &ModelGraph,
        schema_json: &Value,
        role: ModelRole,
    ) -> Result<String, CodegenError> {
        let (ordered_models, needs_rebuild) = topo_sort_models(&graph.models);
        let mut ctx = PyContext::new(&self.options, schema_json);
        let needs_root_wrapper = !matches!(
            &graph.root_type,
            SchemaType::Object(name) if name == &graph.root_name
        );

        let mut out = CodeWriter::new();

        ctx.imports.add("pydantic", "ConfigDict");
        ctx.imports.add("pydantic", "Field");
        ctx.imports.add("pydantic", "model_validator");
        ctx.imports.add("typing", "ClassVar");
        ctx.validate_formats = should_validate_formats(schema_json);

        if let Some(base_module) = &self.options.base_module {
            ctx.imports.add(base_module, "SerializerBase");
            ctx.imports.add(base_module, "DeserializerBase");
            if needs_root_wrapper {
                ctx.imports.add(base_module, "SerializerRootModel");
                ctx.imports.add(base_module, "DeserializerRootModel");
            }
        } else {
            ctx.imports.add("pydantic", "BaseModel");
            ctx.imports.add("jsonschema_rs", "validator_for");
            if needs_root_wrapper {
                ctx.imports.add("pydantic", "RootModel");
                ctx.imports.add("typing", "Any");
            }
            emit_literal_helpers(&mut out);
            match role {
                ModelRole::Serializer => emit_serializer_base(&mut out),
                ModelRole::Deserializer => emit_deserializer_base(&mut out),
            }
            if needs_root_wrapper {
                match role {
                    ModelRole::Serializer => emit_serializer_root_base(&mut out),
                    ModelRole::Deserializer => emit_deserializer_root_base(&mut out),
                }
            }
        }

        emit_schema_helpers(&mut out, &mut ctx)?;

        for model_name in ordered_models {
            let model =
                graph
                    .models
                    .get(&model_name)
                    .ok_or_else(|| CodegenError::InvalidSchema {
                        location: crate::SchemaPath::root(),
                        message: format!("missing model {model_name}"),
                    })?;
            emit_model(&mut out, &mut ctx, model, role)?;
        }

        if needs_root_wrapper {
            emit_root_model(&mut out, &mut ctx, role, &graph.root_name, &graph.root_type)?;
        }

        if needs_rebuild {
            out.push_empty();
            for model_name in graph.models.keys() {
                let class_name = ctx.class_name(model_name, role);
                out.push_line(&format!("{class_name}.model_rebuild()"));
            }
            if needs_root_wrapper {
                let class_name = ctx.class_name(&graph.root_name, role);
                out.push_line(&format!("{class_name}.model_rebuild()"));
            }
        }

        let mut rendered = String::new();
        if let Some(comment) = &self.options.header_comment {
            rendered.push_str("\"\"\"\n");
            rendered.push_str(comment);
            if !comment.ends_with('\n') {
                rendered.push('\n');
            }
            rendered.push_str("\"\"\"\n\n");
        }
        rendered.push_str(&ctx.imports.render());
        rendered.push_str(&out.finish());
        Ok(rendered)
    }
}

impl ModelGenerator for PydanticGenerator {
    fn generate_model(&self, schema: &SchemaNode, role: ModelRole) -> Result<String, CodegenError> {
        self.generate_from_value(&schema.to_json(), role)
    }
}

/// Source for the reusable Pydantic base classes used by generated models.
pub fn base_module() -> String {
    let mut imports = ImportSet::new();
    imports.add("pydantic", "BaseModel");
    imports.add("pydantic", "ConfigDict");
    imports.add("pydantic", "model_validator");
    imports.add("pydantic", "RootModel");
    imports.add("jsonschema_rs", "validator_for");
    imports.add("typing", "Any");
    imports.add("typing", "ClassVar");

    let mut out = CodeWriter::new();
    emit_literal_helpers(&mut out);
    emit_serializer_base(&mut out);
    emit_deserializer_base(&mut out);
    emit_serializer_root_base(&mut out);
    emit_deserializer_root_base(&mut out);

    let mut rendered = String::new();
    rendered.push_str(&imports.render());
    rendered.push_str(&out.finish());
    rendered
}

struct TypeExpr {
    expr: String,
    field_args: Vec<FieldArg>,
    validators: Vec<String>,
}

struct PyContext {
    imports: ImportSet,
    options: PydanticOptions,
    root_schema: Value,
    validate_formats: bool,
}

impl PyContext {
    fn new(options: &PydanticOptions, root_schema: &Value) -> Self {
        Self {
            imports: ImportSet::new(),
            options: options.clone(),
            root_schema: root_schema.clone(),
            validate_formats: false,
        }
    }

    fn class_name(&self, model: &str, role: ModelRole) -> String {
        match role {
            ModelRole::Serializer => format!("{model}{}", self.options.serializer_suffix),
            ModelRole::Deserializer => format!("{model}{}", self.options.deserializer_suffix),
        }
    }

    fn type_expr(
        &mut self,
        schema: &SchemaType,
        role: ModelRole,
    ) -> Result<TypeExpr, CodegenError> {
        match schema {
            SchemaType::Any => {
                self.imports.add("typing", "Any");
                Ok(TypeExpr {
                    expr: "Any".to_string(),
                    field_args: Vec::new(),
                    validators: Vec::new(),
                })
            }
            SchemaType::Bool => Ok(TypeExpr {
                expr: "bool".to_string(),
                field_args: Vec::new(),
                validators: Vec::new(),
            }),
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
                let field_args = constraint_args_for_string(constraints)?;
                if constraints.type_enforced {
                    return Ok(TypeExpr {
                        expr: base,
                        field_args,
                        validators: Vec::new(),
                    });
                }
                self.imports.add("typing", "Any");
                self.imports.add("pydantic", "TypeAdapter");
                self.imports.add("pydantic", "ConfigDict");
                self.imports
                    .add("pydantic.functional_validators", "BeforeValidator");
                let adapter_type = self.apply_metadata(base, &field_args, &[]);
                let validator = format!("BeforeValidator(lambda v, _adapter=TypeAdapter({adapter_type}, config=ConfigDict(strict=True)): v if not isinstance(v, str) else _adapter.validate_python(v))");
                Ok(TypeExpr {
                    expr: "Any".to_string(),
                    field_args: Vec::new(),
                    validators: vec![validator],
                })
            }
            SchemaType::Integer(constraints) => {
                let (field_args, _) = integer_field_args(constraints)?;
                if constraints.type_enforced {
                    let expr = match role {
                        ModelRole::Serializer => "int | float".to_string(),
                        ModelRole::Deserializer => "int".to_string(),
                    };
                    return Ok(TypeExpr {
                        expr,
                        field_args,
                        validators: Vec::new(),
                    });
                }
                self.imports.add("typing", "Any");
                self.imports.add("pydantic", "TypeAdapter");
                self.imports.add("pydantic", "ConfigDict");
                self.imports
                    .add("pydantic.functional_validators", "BeforeValidator");
                let adapter_type = self.apply_metadata("int".to_string(), &field_args, &[]);
                let validator = format!("BeforeValidator(lambda v, _adapter=TypeAdapter({adapter_type}, config=ConfigDict(strict=True)): v if isinstance(v, bool) or not isinstance(v, int) else _adapter.validate_python(v))");
                Ok(TypeExpr {
                    expr: "Any".to_string(),
                    field_args: Vec::new(),
                    validators: vec![validator],
                })
            }
            SchemaType::Number(constraints) => {
                let field_args = constraint_args_for_number(constraints)?;
                if constraints.type_enforced {
                    return Ok(TypeExpr {
                        expr: "float".to_string(),
                        field_args,
                        validators: Vec::new(),
                    });
                }
                self.imports.add("typing", "Any");
                self.imports.add("pydantic", "TypeAdapter");
                self.imports.add("pydantic", "ConfigDict");
                self.imports
                    .add("pydantic.functional_validators", "BeforeValidator");
                let adapter_type = self.apply_metadata("float".to_string(), &field_args, &[]);
                let validator = format!("BeforeValidator(lambda v, _adapter=TypeAdapter({adapter_type}, config=ConfigDict(strict=True)): v if isinstance(v, bool) or not isinstance(v, (int, float)) else _adapter.validate_python(v))");
                Ok(TypeExpr {
                    expr: "Any".to_string(),
                    field_args: Vec::new(),
                    validators: vec![validator],
                })
            }
            SchemaType::Null => Ok(TypeExpr {
                expr: "None".to_string(),
                field_args: Vec::new(),
                validators: Vec::new(),
            }),
            SchemaType::Array { items, constraints } => {
                let inner = self.type_expr(items, role)?;
                let inner_expr =
                    self.apply_metadata(inner.expr, &inner.field_args, &inner.validators);
                let base = format!("list[{inner_expr}]");
                let field_args = constraint_args_for_array(constraints)?;
                if constraints.type_enforced {
                    return Ok(TypeExpr {
                        expr: base,
                        field_args,
                        validators: Vec::new(),
                    });
                }
                self.imports.add("typing", "Any");
                self.imports.add("pydantic", "TypeAdapter");
                self.imports.add("pydantic", "ConfigDict");
                self.imports
                    .add("pydantic.functional_validators", "BeforeValidator");
                let adapter_type = self.apply_metadata(base, &field_args, &[]);
                let validator = format!("BeforeValidator(lambda v, _adapter=TypeAdapter({adapter_type}, config=ConfigDict(strict=True)): v if not isinstance(v, list) else _adapter.validate_python(v))");
                Ok(TypeExpr {
                    expr: "Any".to_string(),
                    field_args: Vec::new(),
                    validators: vec![validator],
                })
            }
            SchemaType::Map {
                values,
                constraints,
            } => {
                let inner = self.type_expr(values, role)?;
                let inner_expr =
                    self.apply_metadata(inner.expr, &inner.field_args, &inner.validators);
                let base = format!("dict[str, {inner_expr}]");
                let field_args = constraint_args_for_object(constraints)?;
                if constraints.type_enforced {
                    return Ok(TypeExpr {
                        expr: base,
                        field_args,
                        validators: Vec::new(),
                    });
                }
                self.imports.add("typing", "Any");
                self.imports.add("pydantic", "TypeAdapter");
                self.imports.add("pydantic", "ConfigDict");
                self.imports
                    .add("pydantic.functional_validators", "BeforeValidator");
                let adapter_type = self.apply_metadata(base, &field_args, &[]);
                let validator = format!("BeforeValidator(lambda v, _adapter=TypeAdapter({adapter_type}, config=ConfigDict(strict=True)): v if not isinstance(v, dict) else _adapter.validate_python(v))");
                Ok(TypeExpr {
                    expr: "Any".to_string(),
                    field_args: Vec::new(),
                    validators: vec![validator],
                })
            }
            SchemaType::Object(name) => Ok(TypeExpr {
                expr: self.class_name(name, role),
                field_args: Vec::new(),
                validators: Vec::new(),
            }),
            SchemaType::Literal(values) => {
                let mut literal_values = Vec::new();
                let mut literal_types = Vec::new();
                let mut all_simple = true;
                for value in values {
                    literal_values.push(python_literal(&value.to_value())?);
                    match literal_value(value)? {
                        Some(rendered) => literal_types.push(rendered),
                        None => all_simple = false,
                    }
                }

                self.imports
                    .add("pydantic.functional_validators", "BeforeValidator");
                if let Some(base) = &self.options.base_module {
                    self.imports.add(base, "_validate_literal");
                }
                let allowed_literal = literal_values.join(", ");
                let validator = format!(
                    "BeforeValidator(lambda v, _allowed=[{allowed_literal}]: _validate_literal(v, _allowed))"
                );

                let expr = if all_simple && !literal_types.is_empty() {
                    self.imports.add("typing", "Literal");
                    format!("Literal[{}]", literal_types.join(", "))
                } else {
                    self.imports.add("typing", "Any");
                    "Any".to_string()
                };

                Ok(TypeExpr {
                    expr,
                    field_args: Vec::new(),
                    validators: vec![validator],
                })
            }
            SchemaType::Union(variants) => {
                let rendered = variants
                    .iter()
                    .map(|variant| self.type_expr(variant, role))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .map(|t| self.apply_metadata(t.expr, &t.field_args, &t.validators))
                    .collect::<Vec<_>>()
                    .join(" | ");
                Ok(TypeExpr {
                    expr: rendered,
                    field_args: Vec::new(),
                    validators: Vec::new(),
                })
            }
            SchemaType::Nullable(inner) => {
                let inner = self.type_expr(inner, role)?;
                let applied = self.apply_metadata(inner.expr, &inner.field_args, &inner.validators);
                Ok(TypeExpr {
                    expr: format!("{applied} | None"),
                    field_args: Vec::new(),
                    validators: Vec::new(),
                })
            }
        }
    }

    fn apply_metadata(
        &mut self,
        base: String,
        args: &[FieldArg],
        _validators: &[String],
    ) -> String {
        let filtered_args: Vec<&FieldArg> = args
            .iter()
            .filter(|arg| matches!(arg.key, "alias" | "default" | "default_factory"))
            .collect();
        if filtered_args.is_empty() {
            return base;
        }
        self.imports.add("typing", "Annotated");
        let mut meta = Vec::new();
        if !filtered_args.is_empty() {
            self.imports.add("pydantic", "Field");
            meta.push(format!("Field({})", render_field_args(&filtered_args)));
        }
        let rendered = meta.join(", ");
        format!("Annotated[{base}, {rendered}]")
    }
}

fn schema_for_path(ctx: &PyContext, path: &SchemaPath) -> Value {
    if path.as_segments().is_empty() {
        return ctx.root_schema.clone();
    }
    if schema_at_path(&ctx.root_schema, path).is_none() {
        return ctx.root_schema.clone();
    }
    let mut defs = serde_json::Map::new();
    if let Value::Object(root) = &ctx.root_schema {
        if let Some(Value::Object(existing_defs)) = root.get("$defs") {
            defs.extend(existing_defs.clone());
        }
        if let Some(Value::Object(existing_defs)) = root.get("definitions") {
            defs.entry("definitions".to_string())
                .or_insert_with(|| Value::Object(existing_defs.clone()));
        }
    }
    defs.insert("__root__".to_string(), ctx.root_schema.clone());

    let mut schema = serde_json::Map::new();
    if let Value::Object(root) = &ctx.root_schema {
        for (key, value) in root {
            if key == "$ref" || key == "$id" {
                continue;
            }
            if key.starts_with('$') && key != "$defs" {
                schema.insert(key.clone(), value.clone());
            }
            if key == "definitions" {
                schema.insert(key.clone(), value.clone());
            }
        }
    }
    schema.insert("$defs".to_string(), Value::Object(defs));
    let suffix = path
        .to_string()
        .strip_prefix('#')
        .map(|s| s.to_string())
        .unwrap_or_else(|| path.to_string());
    schema.insert(
        "$ref".to_string(),
        Value::String(format!("#/$defs/__root__{suffix}")),
    );
    Value::Object(schema)
}

fn schema_at_path<'a>(schema: &'a Value, path: &SchemaPath) -> Option<&'a Value> {
    let mut current = schema;
    for segment in path.as_segments() {
        match current {
            Value::Object(map) => match map.get(segment) {
                Some(next) => current = next,
                None => return None,
            },
            Value::Array(items) => {
                let idx = segment.parse::<usize>().ok()?;
                current = items.get(idx)?;
            }
            _ => return None,
        }
    }
    Some(current)
}

fn should_validate_formats(schema: &Value) -> bool {
    if let Value::Object(obj) = schema {
        if let Some(Value::Object(vocab)) = obj.get("$vocabulary") {
            if vocab
                .get("https://json-schema.org/draft/2020-12/vocab/format-assertion")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
                || vocab
                    .get("https://json-schema.org/draft/2019-09/vocab/format-assertion")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            {
                return true;
            }
        }
        if let Some(Value::String(uri)) = obj.get("$schema") {
            if uri.contains("format-assertion") {
                return true;
            }
        }
    }
    false
}

fn emit_schema_helpers(out: &mut CodeWriter, ctx: &mut PyContext) -> Result<(), CodegenError> {
    let rendered_schema = schema_string_literal(&ctx.root_schema)?;
    out.push_line(&format!("_JSON_SCHEMA = {rendered_schema}"));
    out.push_empty();
    out.push_line(&format!(
        "_VALIDATE_FORMATS = {}",
        if ctx.validate_formats {
            "True"
        } else {
            "False"
        }
    ));
    out.push_empty();
    Ok(())
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

fn emit_literal_helpers(out: &mut CodeWriter) {
    out.push_line("def _json_equal(candidate, expected):");
    out.indent();
    out.push_line("if isinstance(expected, bool):");
    out.indent();
    out.push_line("return isinstance(candidate, bool) and candidate is expected");
    out.dedent();
    out.push_line("if expected is None:");
    out.indent();
    out.push_line("return candidate is None");
    out.dedent();
    out.push_line("if isinstance(expected, (int, float)) and not isinstance(expected, bool):");
    out.indent();
    out.push_line(
        "return isinstance(candidate, (int, float)) and not isinstance(candidate, bool) and candidate == expected",
    );
    out.dedent();
    out.push_line("if isinstance(expected, list):");
    out.indent();
    out.push_line(
        "return isinstance(candidate, list) and len(candidate) == len(expected) and all(_json_equal(c, e) for c, e in zip(candidate, expected))",
    );
    out.dedent();
    out.push_line("if isinstance(expected, dict):");
    out.indent();
    out.push_line(
        "return isinstance(candidate, dict) and candidate.keys() == expected.keys() and all(_json_equal(candidate[k], v) for k, v in expected.items())",
    );
    out.dedent();
    out.push_line("return candidate == expected");
    out.dedent();
    out.push_empty();

    out.push_line("def _validate_literal(value, allowed):");
    out.indent();
    out.push_line("if any(_json_equal(value, expected) for expected in allowed):");
    out.indent();
    out.push_line("return value");
    out.dedent();
    out.push_line("raise ValueError(\"value does not match literal constraint\")");
    out.dedent();
    out.push_empty();
}

fn emit_serializer_base(out: &mut CodeWriter) {
    out.push_line("class SerializerBase(BaseModel):");
    out.indent();
    out.push_line("__json_schema__: ClassVar[str | None] = None");
    out.push_line("_validate_formats: ClassVar[bool] = False");
    out.push_line("_jsonschema_validator: ClassVar[object | None] = None");
    out.push_empty();
    out.push_line("@classmethod");
    out.push_line("def _get_jsonschema_validator(cls):");
    out.indent();
    out.push_line("if cls.__json_schema__ is None:");
    out.indent();
    out.push_line("raise TypeError(f\"{cls.__name__} is missing __json_schema__\")");
    out.dedent();
    out.push_line("validator = cls._jsonschema_validator");
    out.push_line("if validator is None:");
    out.indent();
    out.push_line(
        "validator = validator_for(cls.__json_schema__, validate_formats=cls._validate_formats)",
    );
    out.push_line("cls._jsonschema_validator = validator");
    out.dedent();
    out.push_line("return validator");
    out.dedent();
    out.push_empty();
    out.push_line("@model_validator(mode=\"before\")");
    out.push_line("@classmethod");
    out.push_line("def _validate_jsonschema(cls, value):");
    out.indent();
    out.push_line("cls._get_jsonschema_validator().validate(value)");
    out.push_line("return value");
    out.dedent();
    out.push_empty();
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
    out.push_line("__json_schema__: ClassVar[str | None] = None");
    out.push_line("_validate_formats: ClassVar[bool] = False");
    out.push_line("_jsonschema_validator: ClassVar[object | None] = None");
    out.push_empty();
    out.push_line("@classmethod");
    out.push_line("def _get_jsonschema_validator(cls):");
    out.indent();
    out.push_line("if cls.__json_schema__ is None:");
    out.indent();
    out.push_line("raise TypeError(f\"{cls.__name__} is missing __json_schema__\")");
    out.dedent();
    out.push_line("validator = cls._jsonschema_validator");
    out.push_line("if validator is None:");
    out.indent();
    out.push_line(
        "validator = validator_for(cls.__json_schema__, validate_formats=cls._validate_formats)",
    );
    out.push_line("cls._jsonschema_validator = validator");
    out.dedent();
    out.push_line("return validator");
    out.dedent();
    out.push_empty();
    out.push_line("@model_validator(mode=\"before\")");
    out.push_line("@classmethod");
    out.push_line("def _validate_jsonschema(cls, value):");
    out.indent();
    out.push_line("cls._get_jsonschema_validator().validate(value)");
    out.push_line("return value");
    out.dedent();
    out.push_empty();
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

fn emit_serializer_root_base(out: &mut CodeWriter) {
    out.push_line("class SerializerRootModel(RootModel[Any]):");
    out.indent();
    out.push_line("__json_schema__: ClassVar[str | None] = None");
    out.push_line("_validate_formats: ClassVar[bool] = False");
    out.push_line("_jsonschema_validator: ClassVar[object | None] = None");
    out.push_empty();
    out.push_line("@classmethod");
    out.push_line("def _get_jsonschema_validator(cls):");
    out.indent();
    out.push_line("if cls.__json_schema__ is None:");
    out.indent();
    out.push_line("raise TypeError(f\"{cls.__name__} is missing __json_schema__\")");
    out.dedent();
    out.push_line("validator = cls._jsonschema_validator");
    out.push_line("if validator is None:");
    out.indent();
    out.push_line(
        "validator = validator_for(cls.__json_schema__, validate_formats=cls._validate_formats)",
    );
    out.push_line("cls._jsonschema_validator = validator");
    out.dedent();
    out.push_line("return validator");
    out.dedent();
    out.push_empty();
    out.push_line("@model_validator(mode=\"before\")");
    out.push_line("@classmethod");
    out.push_line("def _validate_jsonschema(cls, value):");
    out.indent();
    out.push_line("cls._get_jsonschema_validator().validate(value)");
    out.push_line("return value");
    out.dedent();
    out.push_empty();
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

fn emit_deserializer_root_base(out: &mut CodeWriter) {
    out.push_line("class DeserializerRootModel(RootModel[Any]):");
    out.indent();
    out.push_line("__json_schema__: ClassVar[str | None] = None");
    out.push_line("_validate_formats: ClassVar[bool] = False");
    out.push_line("_jsonschema_validator: ClassVar[object | None] = None");
    out.push_empty();
    out.push_line("@classmethod");
    out.push_line("def _get_jsonschema_validator(cls):");
    out.indent();
    out.push_line("if cls.__json_schema__ is None:");
    out.indent();
    out.push_line("raise TypeError(f\"{cls.__name__} is missing __json_schema__\")");
    out.dedent();
    out.push_line("validator = cls._jsonschema_validator");
    out.push_line("if validator is None:");
    out.indent();
    out.push_line(
        "validator = validator_for(cls.__json_schema__, validate_formats=cls._validate_formats)",
    );
    out.push_line("cls._jsonschema_validator = validator");
    out.dedent();
    out.push_line("return validator");
    out.dedent();
    out.push_empty();
    out.push_line("@model_validator(mode=\"before\")");
    out.push_line("@classmethod");
    out.push_line("def _validate_jsonschema(cls, value):");
    out.indent();
    out.push_line("cls._get_jsonschema_validator().validate(value)");
    out.push_line("return value");
    out.dedent();
    out.push_empty();
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

    out.push_line("_validate_formats = _VALIDATE_FORMATS");
    if model.schema_path.as_segments().is_empty() {
        out.push_line("__json_schema__ = _JSON_SCHEMA");
    } else {
        let schema_for_model = schema_for_path(ctx, &model.schema_path);
        let rendered_schema = schema_string_literal(&schema_for_model)?;
        out.push_line(&format!("__json_schema__ = {rendered_schema}"));
    }

    if !model.requires_object {
        ctx.imports.add("pydantic_core", "core_schema");
        out.push_empty();
        out.push_line("@classmethod");
        out.push_line("def __get_pydantic_core_schema__(cls, source, handler):");
        out.indent();
        out.push_line("model_schema = handler(source)");
        out.push_line(
            "non_object_schema = core_schema.no_info_plain_validator_function(lambda v: v)",
        );
        out.push_line(
            "return core_schema.tagged_union_schema({True: model_schema, False: non_object_schema}, discriminator=lambda v: isinstance(v, dict))",
        );
        out.dedent();
    }

    let needs_custom_extra = !model.pattern_properties.is_empty()
        || model.property_name_max.is_some()
        || !matches!(model.additional_properties, AdditionalProperties::Allow);
    let extra_value = if needs_custom_extra {
        "allow"
    } else {
        match &model.additional_properties {
            AdditionalProperties::Allow => "allow",
            AdditionalProperties::Forbid => "forbid",
            AdditionalProperties::Typed(_) => "allow",
        }
    };
    out.push_line(&format!(
        "model_config = ConfigDict(extra=\"{extra_value}\")"
    ));

    let field_names = PythonFieldNames::new(&model.fields);
    let mut fields: Vec<&FieldSpec> = model.fields.iter().collect();
    fields.sort_by(|a, b| a.name.cmp(&b.name));

    for field in fields {
        if !field.include_in_role(role) {
            continue;
        }
        emit_field(out, ctx, field, &field_names, role)?;
    }

    out.dedent();
    out.push_empty();
    Ok(())
}

fn emit_root_model(
    out: &mut CodeWriter,
    ctx: &mut PyContext,
    role: ModelRole,
    root_name: &str,
    schema: &SchemaType,
) -> Result<(), CodegenError> {
    let class_name = ctx.class_name(root_name, role);
    let base = match role {
        ModelRole::Serializer => "SerializerRootModel",
        ModelRole::Deserializer => "DeserializerRootModel",
    };
    let type_expr = ctx.type_expr(schema, role)?;
    let rendered_type =
        ctx.apply_metadata(type_expr.expr, &type_expr.field_args, &type_expr.validators);

    out.push_line(&format!("class {class_name}({base}):"));
    out.indent();
    out.push_line("_validate_formats = _VALIDATE_FORMATS");
    out.push_line("__json_schema__ = _JSON_SCHEMA");
    out.push_line(&format!("root: {rendered_type}"));
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

    let mut args = type_expr.field_args;
    if names.needs_alias(&field.name) {
        args.push(FieldArg::new("alias", python_string_literal(&field.name)?));
    }
    if let Some(title) = &field.title {
        args.push(FieldArg::new("title", python_string_literal(title)?));
    }
    if let Some(desc) = &field.description {
        args.push(FieldArg::new("description", python_string_literal(desc)?));
    }

    let mut rendered_type = type_expr.expr;
    if !required && !field.schema.allows_null() {
        rendered_type = format!("{rendered_type} | None");
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
    } else if !required {
        args.push(FieldArg::new("default", "None".to_string()));
    }

    let annotated = ctx.apply_metadata(rendered_type, &args, &type_expr.validators);
    out.push_line(&format!("{field_name}: {annotated}"));
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

fn render_field_args(args: &[&FieldArg]) -> String {
    args.iter()
        .map(|arg| format!("{}={}", arg.key, arg.value))
        .collect::<Vec<_>>()
        .join(", ")
}

fn normalize_pattern(pattern: &str) -> String {
    // Translate ECMA-style control escapes (\cX) into literal control characters
    // so Python's regex engine can compile them.
    let mut out = String::with_capacity(pattern.len());
    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && matches!(chars.peek(), Some('c')) {
            // consume 'c'
            let _ = chars.next();
            if let Some(ctrl) = chars.next() {
                if ctrl.is_ascii_alphabetic() {
                    let upper = ctrl.to_ascii_uppercase();
                    let code = (upper as u8) ^ 0x40;
                    out.push(char::from(code));
                    continue;
                } else {
                    out.push('\\');
                    out.push('c');
                    out.push(ctrl);
                    continue;
                }
            } else {
                out.push('\\');
                out.push('c');
                break;
            }
        }
        out.push(c);
    }
    out
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
        let normalized = normalize_pattern(pattern);
        args.push(FieldArg::new(
            "pattern",
            python_string_literal(&normalized)?,
        ));
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

fn integer_field_args(
    constraints: &crate::model::NumberConstraints,
) -> Result<(Vec<FieldArg>, Option<f64>), CodegenError> {
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
    let mut fractional_multiple = None;
    if let Some(multiple_of) = constraints.multiple_of {
        if multiple_of.fract() == 0.0 {
            args.push(FieldArg::new("multiple_of", float_literal(multiple_of)));
        } else {
            fractional_multiple = Some(multiple_of);
        }
    }
    Ok((args, fractional_multiple))
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

fn literal_value(value: &crate::model::LiteralValue) -> Result<Option<String>, CodegenError> {
    let rendered = match value {
        crate::model::LiteralValue::Null => "None".to_string(),
        crate::model::LiteralValue::Bool(v) => {
            if *v {
                "True".to_string()
            } else {
                "False".to_string()
            }
        }
        crate::model::LiteralValue::String(s) => python_string_literal(s)?,
        crate::model::LiteralValue::Number(n) => n.to_string(),
        crate::model::LiteralValue::Json(_) => return Ok(None),
    };
    Ok(Some(rendered))
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

fn schema_string_literal(schema: &Value) -> Result<String, CodegenError> {
    let pretty = serde_json::to_string_pretty(schema).map_err(|e| CodegenError::InvalidSchema {
        location: crate::SchemaPath::root(),
        message: format!("failed to serialize schema: {e}"),
    })?;
    Ok(format!("r\"\"\"\n{pretty}\n\"\"\""))
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
                | "_jsonschema_validator"
                | "_get_jsonschema_validator"
                | "_validate_jsonschema"
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

    let mut queue: BinaryHeap<Reverse<String>> = incoming
        .iter()
        .filter(|(_, &count)| count == 0)
        .map(|(name, _)| Reverse(name.clone()))
        .collect();
    let mut ordered = Vec::new();

    while let Some(Reverse(name)) = queue.pop() {
        ordered.push(name.clone());
        if let Some(children) = adjacency.get(&name) {
            let mut sorted = children.clone();
            sorted.sort();
            for child in sorted {
                if let Some(count) = incoming.get_mut(&child) {
                    *count -= 1;
                    if *count == 0 {
                        queue.push(Reverse(child));
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
