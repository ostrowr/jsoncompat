use crate::{JSONCOMPAT_METADATA_KEY, JsoncompatMetadata};
use json_schema_ast::canonicalize_json;
use serde_json::{Map, Value};
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

const DATACLASS_ADDITIONAL_MODEL_CLASS: &str = "DataclassAdditionalModel";
const DATACLASS_MODEL_CLASS: &str = "DataclassModel";
const DATACLASS_ROOT_MODEL_CLASS: &str = "DataclassRootModel";
const READER_MODEL_CLASS: &str = "ReaderDataclassModel";
const READER_ROOT_MODEL_CLASS: &str = "ReaderDataclassRootModel";
const WRITER_MODEL_CLASS: &str = "WriterDataclassModel";
const EXTRA_FIELD_NAME: &str = "__jsoncompat_extra__";
const MISSING_TYPE_NAME: &str = "JsoncompatMissingType";
const DATACLASSES_RUNTIME_MODULE: &str = "jsoncompat_dataclasses";

#[derive(Debug, thiserror::Error)]
pub enum DataclassError {
    #[error("invalid schema at '{pointer}': {message}")]
    InvalidSchema { pointer: String, message: String },
    #[error("unsupported $ref '{ref_value}' at '{pointer}'")]
    UnsupportedRef { pointer: String, ref_value: String },
    #[error("missing {metadata_key} declaration metadata at '{pointer}'")]
    MissingDeclarationMetadata {
        pointer: String,
        metadata_key: &'static str,
    },
    #[error("duplicate generated declaration '{name}'")]
    DuplicateDeclaration { name: String },
}

#[derive(Debug, Clone)]
struct FieldSpec {
    json_name: String,
    py_name: String,
    annotation: String,
    required: bool,
}

#[derive(Debug, Clone)]
enum ClassKind {
    Object {
        fields: Vec<FieldSpec>,
        extra_annotation: Option<String>,
    },
    Root {
        annotation: String,
    },
}

#[derive(Debug, Clone)]
struct ClassSpec {
    name: String,
    schema_json: String,
    base_class: &'static str,
    kind: ClassKind,
}

#[derive(Debug)]
struct DataclassModuleBuilder {
    classes: Vec<ClassSpec>,
    class_names: BTreeMap<String, String>,
    named_refs: BTreeMap<String, String>,
    root_defs: Option<Map<String, Value>>,
    inline_names: BTreeMap<String, String>,
    inline_name_counters: BTreeMap<String, u32>,
}

impl DataclassModuleBuilder {
    fn new(named_refs: BTreeMap<String, String>, root_defs: Option<Map<String, Value>>) -> Self {
        let class_names = [
            DATACLASS_ADDITIONAL_MODEL_CLASS,
            DATACLASS_MODEL_CLASS,
            DATACLASS_ROOT_MODEL_CLASS,
            READER_MODEL_CLASS,
            READER_ROOT_MODEL_CLASS,
            WRITER_MODEL_CLASS,
        ]
        .into_iter()
        .map(|name| (name.to_owned(), "#/generated-preamble".to_owned()))
        .collect();

        Self {
            classes: Vec::new(),
            class_names,
            named_refs,
            root_defs,
            inline_names: BTreeMap::new(),
            inline_name_counters: BTreeMap::new(),
        }
    }

    fn reserve_class_name(&mut self, name: &str, pointer: &str) -> Result<bool, DataclassError> {
        match self.class_names.entry(name.to_owned()) {
            Entry::Vacant(entry) => {
                entry.insert(pointer.to_owned());
                Ok(true)
            }
            Entry::Occupied(entry) => {
                if entry.get() == pointer {
                    Ok(false)
                } else {
                    Err(DataclassError::DuplicateDeclaration {
                        name: name.to_owned(),
                    })
                }
            }
        }
    }

    fn register_named_declaration(
        &mut self,
        schema: &Value,
        pointer: &str,
    ) -> Result<String, DataclassError> {
        let name = self.named_refs.get(pointer).cloned().ok_or_else(|| {
            invalid_schema(
                pointer.to_owned(),
                "schema declaration name was not collected",
            )
        })?;
        self.emit_declaration_schema_class(&name, schema, pointer, DATACLASS_MODEL_CLASS)?;
        Ok(name)
    }

    fn emit_declaration_schema_class(
        &mut self,
        name: &str,
        schema: &Value,
        pointer: &str,
        base_class: &'static str,
    ) -> Result<(), DataclassError> {
        match schema {
            Value::Bool(_) => self.register_class(
                ClassSpec {
                    name: name.to_owned(),
                    schema_json: self.schema_json(schema)?,
                    base_class: DATACLASS_ROOT_MODEL_CLASS,
                    kind: ClassKind::Root {
                        annotation: typing_symbol("Any"),
                    },
                },
                pointer,
            ),
            Value::Object(obj) => self.emit_declaration_class(name, obj, pointer, base_class),
            _ => Err(invalid_schema(
                pointer.to_owned(),
                "schema declarations must be objects or booleans",
            )),
        }
    }

    fn emit_declaration_class(
        &mut self,
        name: &str,
        obj: &Map<String, Value>,
        pointer: &str,
        base_class: &'static str,
    ) -> Result<(), DataclassError> {
        if !self.reserve_class_name(name, pointer)? {
            return Ok(());
        }

        emit_nested_defs(self, obj, pointer)?;

        let kind = if is_object_schema(obj) {
            ClassKind::Object {
                fields: parse_object_fields(self, obj, pointer, name)?,
                extra_annotation: parse_extra_annotation(self, obj, pointer, name)?,
            }
        } else {
            ClassKind::Root {
                annotation: self.schema_annotation(obj, pointer, name)?,
            }
        };

        self.classes.push(ClassSpec {
            name: name.to_owned(),
            schema_json: self.schema_json(&Value::Object(obj.clone()))?,
            base_class: if matches!(kind, ClassKind::Root { .. }) {
                DATACLASS_ROOT_MODEL_CLASS
            } else {
                base_class
            },
            kind,
        });
        Ok(())
    }

    fn register_class(
        &mut self,
        class_spec: ClassSpec,
        pointer: &str,
    ) -> Result<(), DataclassError> {
        if !self.reserve_class_name(&class_spec.name, pointer)? {
            return Ok(());
        }
        self.classes.push(class_spec);
        Ok(())
    }

    fn emit_inline_object_class(
        &mut self,
        obj: &Map<String, Value>,
        pointer: &str,
        scope_name: &str,
        hint_name: &str,
    ) -> Result<String, DataclassError> {
        if let Some(name) = self.named_refs.get(pointer).cloned() {
            self.emit_declaration_class(&name, obj, pointer, DATACLASS_MODEL_CLASS)?;
            return Ok(name);
        }
        if let Some(name) = self.inline_names.get(pointer).cloned() {
            return Ok(name);
        }

        let candidate = self.next_inline_name(&format!("{scope_name}{}", pascal_case(hint_name)));
        self.emit_declaration_class(&candidate, obj, pointer, DATACLASS_MODEL_CLASS)?;
        self.inline_names
            .insert(pointer.to_owned(), candidate.clone());
        Ok(candidate)
    }

    fn emit_inline_root_class(
        &mut self,
        schema: &Value,
        pointer: &str,
        scope_name: &str,
        hint_name: &str,
    ) -> Result<String, DataclassError> {
        if let Some(name) = self.named_refs.get(pointer).cloned() {
            self.emit_declaration_schema_class(&name, schema, pointer, DATACLASS_MODEL_CLASS)?;
            return Ok(name);
        }
        if let Some(name) = self.inline_names.get(pointer).cloned() {
            return Ok(name);
        }

        let candidate = self.next_inline_name(&format!("{scope_name}{}", pascal_case(hint_name)));
        self.emit_declaration_schema_class(&candidate, schema, pointer, DATACLASS_MODEL_CLASS)?;
        self.inline_names
            .insert(pointer.to_owned(), candidate.clone());
        Ok(candidate)
    }

    fn next_inline_name(&mut self, base_name: &str) -> String {
        let count = self
            .inline_name_counters
            .entry(base_name.to_owned())
            .or_insert(0);
        *count += 1;
        if *count == 1 {
            base_name.to_owned()
        } else {
            format!("{base_name}{count}")
        }
    }

    fn schema_json(&self, schema: &Value) -> Result<String, DataclassError> {
        let schema = match (schema, self.root_defs.as_ref()) {
            (Value::Object(obj), Some(root_defs)) => Value::Object(schema_object_with_root_defs(
                obj,
                &Map::from_iter([("$defs".to_owned(), Value::Object(root_defs.clone()))]),
            )?),
            _ => schema.clone(),
        };
        Ok(canonical_schema_literal(&schema))
    }

    fn schema_annotation(
        &mut self,
        obj: &Map<String, Value>,
        pointer: &str,
        scope_name: &str,
    ) -> Result<String, DataclassError> {
        if let Some(ref_value) = obj.get("$ref") {
            let ref_value = ref_value.as_str().ok_or_else(|| {
                invalid_schema(join_pointer(pointer, "$ref"), "$ref must be a string")
            })?;
            return resolve_local_ref_name(&self.named_refs, ref_value, pointer);
        }

        if obj.contains_key("oneOf") {
            return self.union_annotation(obj, pointer, scope_name, "oneOf");
        }
        if obj.contains_key("anyOf") {
            return self.union_annotation(obj, pointer, scope_name, "anyOf");
        }

        if is_object_schema(obj) {
            return self.emit_inline_object_class(obj, pointer, scope_name, "Value");
        }

        if obj.contains_key("prefixItems") {
            return Ok(parse_type_annotation(obj, pointer)?.unwrap_or_else(|| typing_symbol("Any")));
        }

        if let Some(items) = obj.get("items") {
            let item_annotation =
                self.inline_annotation(items, &join_pointer(pointer, "items"), scope_name, "Item")?;
            return Ok(format!("list[{item_annotation}]"));
        }

        if let Some(literal) = parse_literal_annotation(obj) {
            return Ok(literal);
        }

        if let Some(type_annotation) = parse_type_annotation(obj, pointer)? {
            return Ok(type_annotation);
        }

        Ok(typing_symbol("Any"))
    }

    fn inline_annotation(
        &mut self,
        schema: &Value,
        pointer: &str,
        scope_name: &str,
        hint_name: &str,
    ) -> Result<String, DataclassError> {
        match schema {
            Value::Bool(_) => self.emit_inline_root_class(schema, pointer, scope_name, hint_name),
            Value::Object(obj) => {
                if let Some(name) = self.named_refs.get(pointer).cloned() {
                    self.emit_declaration_class(&name, obj, pointer, DATACLASS_MODEL_CLASS)?;
                    return Ok(name);
                }
                if obj.contains_key("oneOf") || obj.contains_key("anyOf") {
                    return self.emit_inline_root_class(schema, pointer, scope_name, hint_name);
                }
                if is_object_schema(obj) {
                    return self.emit_inline_object_class(obj, pointer, scope_name, hint_name);
                }
                if obj.get("$ref").is_some() {
                    return self.schema_annotation(obj, pointer, scope_name);
                }
                if obj.get("items").is_some() {
                    return self.schema_annotation(obj, pointer, scope_name);
                }
                if parse_literal_annotation(obj).is_some() {
                    return self.schema_annotation(obj, pointer, scope_name);
                }
                if parse_type_annotation(obj, pointer)?.is_some() {
                    return self.schema_annotation(obj, pointer, scope_name);
                }
                self.emit_inline_root_class(schema, pointer, scope_name, hint_name)
            }
            _ => Err(invalid_schema(
                pointer.to_owned(),
                "subschemas must be objects or booleans",
            )),
        }
    }

    fn union_annotation(
        &mut self,
        obj: &Map<String, Value>,
        pointer: &str,
        scope_name: &str,
        keyword: &str,
    ) -> Result<String, DataclassError> {
        let branches = obj.get(keyword).and_then(Value::as_array).ok_or_else(|| {
            invalid_schema(
                join_pointer(pointer, keyword),
                format!("{keyword} must be an array"),
            )
        })?;
        if branches.is_empty() {
            return Err(invalid_schema(
                join_pointer(pointer, keyword),
                format!("{keyword} must contain at least one branch"),
            ));
        }

        let context = union_branch_context(obj, keyword);
        let mut annotations = Vec::new();
        for (index, branch) in branches.iter().enumerate() {
            let branch_pointer = join_pointer(&join_pointer(pointer, keyword), &index.to_string());
            let merged_branch = merge_union_branch_schema(branch, &context, &branch_pointer)?;
            annotations.push(self.inline_annotation(
                &merged_branch,
                &branch_pointer,
                scope_name,
                &format!("Branch{index}"),
            )?);
        }

        Ok(union_annotation(&annotations))
    }
}

pub fn generate_dataclass_models(schema: &Value) -> Result<String, DataclassError> {
    render_dataclass_module(schema)
}

fn render_dataclass_module(schema: &Value) -> Result<String, DataclassError> {
    let named_refs = collect_named_refs(schema)?;
    let root_name = named_refs.get("#").cloned().ok_or_else(|| {
        invalid_schema(
            "#".to_owned(),
            "root schema declaration name was not collected",
        )
    })?;

    let root_metadata = parse_optional_metadata(schema, "#")?;
    let root_defs = schema
        .as_object()
        .and_then(|obj| obj.get("$defs"))
        .and_then(Value::as_object)
        .cloned();
    let mut builder = DataclassModuleBuilder::new(named_refs, root_defs);
    match &root_metadata {
        Some(JsoncompatMetadata::Writer { .. }) | Some(JsoncompatMetadata::Reader { .. }) => {
            emit_root_defs(&mut builder, schema)?;
            builder.reserve_class_name(&root_name, "#")?;
        }
        Some(JsoncompatMetadata::ReaderVariant { .. }) => {
            return Err(invalid_schema(
                join_pointer("#", JSONCOMPAT_METADATA_KEY),
                "reader_variant metadata is only valid on oneOf branches",
            ));
        }
        Some(JsoncompatMetadata::Declaration { .. }) | None => {
            builder.register_named_declaration(schema, "#")?;
        }
    }

    let mut output = String::new();
    output.push_str("from __future__ import annotations\n\n");
    output.push_str("from dataclasses import dataclass\n");
    output.push_str("import typing\n\n");
    output.push_str("from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses\n\n\n");

    for class_spec in &builder.classes {
        render_class_spec(&mut output, class_spec);
        output.push('\n');
    }

    if let Some(metadata) = &root_metadata {
        match metadata {
            JsoncompatMetadata::Writer { .. } => {
                render_writer_class(&mut output, expect_schema_object(schema, "#")?)?;
                output.push('\n');
            }
            JsoncompatMetadata::Reader { .. } => {
                render_reader_variants(&mut output, expect_schema_object(schema, "#")?)?;
                output.push('\n');
                render_reader_root_class(&mut output, expect_schema_object(schema, "#")?)?;
                output.push('\n');
            }
            JsoncompatMetadata::Declaration { .. } => {}
            JsoncompatMetadata::ReaderVariant { .. } => unreachable!(),
        }
    }

    for class_spec in &builder.classes {
        render_class_runtime_spec(&mut output, class_spec);
        output.push('\n');
    }

    if let Some(metadata) = &root_metadata {
        match metadata {
            JsoncompatMetadata::Writer { .. } => {
                render_writer_runtime_spec(&mut output, expect_schema_object(schema, "#")?)?;
                output.push('\n');
            }
            JsoncompatMetadata::Reader { .. } => {
                render_reader_variant_runtime_specs(
                    &mut output,
                    expect_schema_object(schema, "#")?,
                )?;
                output.push('\n');
                render_reader_root_runtime_spec(&mut output, expect_schema_object(schema, "#")?)?;
                output.push('\n');
            }
            JsoncompatMetadata::Declaration { .. } => {}
            JsoncompatMetadata::ReaderVariant { .. } => unreachable!(),
        }
    }

    writeln!(&mut output, "JSONCOMPAT_MODEL = {root_name}").expect("writing to String cannot fail");

    Ok(output)
}

fn render_class_spec(output: &mut String, class_spec: &ClassSpec) {
    writeln!(output, "@dataclass(frozen=True, slots=True, kw_only=True)")
        .expect("writing to String cannot fail");
    writeln!(
        output,
        "class {}({}):",
        class_spec.name,
        render_class_base(class_spec),
    )
    .expect("writing to String cannot fail");
    writeln!(
        output,
        "    __jsoncompat_schema__: typing.ClassVar[str] = {}",
        python_string_literal(&class_spec.schema_json)
    )
    .expect("writing to String cannot fail");

    match &class_spec.kind {
        ClassKind::Object {
            fields,
            extra_annotation,
        } => {
            if fields.is_empty() && extra_annotation.is_none() {
                writeln!(output, "    pass").expect("writing to String cannot fail");
                return;
            }
            for field in fields {
                let annotation = if field.required {
                    field.annotation.clone()
                } else {
                    format!(
                        "({} | {})",
                        field.annotation,
                        runtime_dataclass_symbol(MISSING_TYPE_NAME),
                    )
                };
                if field.required {
                    writeln!(
                        output,
                        "    {}: {} = {}.jsoncompat_field({})",
                        field.py_name,
                        annotation,
                        DATACLASSES_RUNTIME_MODULE,
                        python_string_literal(&field.json_name)
                    )
                    .expect("writing to String cannot fail");
                } else {
                    writeln!(
                        output,
                        "    {}: {} = {}.jsoncompat_field({}, omittable=True)",
                        field.py_name,
                        annotation,
                        DATACLASSES_RUNTIME_MODULE,
                        python_string_literal(&field.json_name)
                    )
                    .expect("writing to String cannot fail");
                }
            }
            if let Some(extra_annotation) = extra_annotation {
                writeln!(
                    output,
                    "    {EXTRA_FIELD_NAME}: dict[str, {extra_annotation}] = {}.jsoncompat_extra_field()",
                    DATACLASSES_RUNTIME_MODULE,
                )
                .expect("writing to String cannot fail");
            }
        }
        ClassKind::Root { annotation } => {
            writeln!(
                output,
                "    root: {annotation} = {}.jsoncompat_root_field()",
                DATACLASSES_RUNTIME_MODULE,
            )
            .expect("writing to String cannot fail");
        }
    }
}

fn render_writer_class(
    output: &mut String,
    writer: &Map<String, Value>,
) -> Result<(), DataclassError> {
    let metadata = parse_metadata(writer, "#")?;
    let JsoncompatMetadata::Writer {
        name,
        version,
        payload_ref,
        ..
    } = metadata
    else {
        return Err(invalid_schema(
            join_pointer("#", JSONCOMPAT_METADATA_KEY),
            "writer schema must have writer metadata",
        ));
    };
    let payload_type = resolve_schema_ref_name(writer, &payload_ref, "#")?;

    writeln!(output, "@dataclass(frozen=True, slots=True, kw_only=True)")
        .expect("writing to String cannot fail");
    writeln!(
        output,
        "class {name}({}):",
        runtime_dataclass_symbol(WRITER_MODEL_CLASS),
    )
    .expect("writing to String cannot fail");
    writeln!(
        output,
        "    __jsoncompat_schema__: typing.ClassVar[str] = {}",
        python_string_literal(&canonical_schema_literal(&Value::Object(writer.clone())))
    )
    .expect("writing to String cannot fail");
    writeln!(
        output,
        "    version: typing.Literal[{version}] = {}.jsoncompat_field(\"version\")",
        DATACLASSES_RUNTIME_MODULE,
    )
    .expect("writing to String cannot fail");
    writeln!(
        output,
        "    data: {payload_type} = {}.jsoncompat_field(\"data\")",
        DATACLASSES_RUNTIME_MODULE,
    )
    .expect("writing to String cannot fail");
    Ok(())
}

fn render_reader_variants(
    output: &mut String,
    reader: &Map<String, Value>,
) -> Result<(), DataclassError> {
    let branches = reader
        .get("oneOf")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid_schema("#/oneOf".to_owned(), "oneOf must be an array"))?;

    for (index, branch) in branches.iter().enumerate() {
        let pointer = format!("#/oneOf/{index}");
        let branch = expect_schema_object(branch, &pointer)?;
        let metadata = parse_metadata(branch, &pointer)?;
        let JsoncompatMetadata::ReaderVariant {
            name,
            version,
            payload_ref,
            ..
        } = metadata
        else {
            return Err(invalid_schema(
                join_pointer(&pointer, JSONCOMPAT_METADATA_KEY),
                "reader branch must have reader_variant metadata",
            ));
        };
        let payload_type = resolve_schema_ref_name(reader, &payload_ref, &pointer)?;

        writeln!(output, "@dataclass(frozen=True, slots=True, kw_only=True)")
            .expect("writing to String cannot fail");
        writeln!(
            output,
            "class {name}({}):",
            runtime_dataclass_symbol(READER_MODEL_CLASS),
        )
        .expect("writing to String cannot fail");
        writeln!(
            output,
            "    __jsoncompat_schema__: typing.ClassVar[str] = {}",
            python_string_literal(&canonical_schema_literal(&Value::Object(
                schema_object_with_root_defs(branch, reader)?
            )))
        )
        .expect("writing to String cannot fail");
        writeln!(
            output,
            "    version: typing.Literal[{version}] = {}.jsoncompat_field(\"version\")",
            DATACLASSES_RUNTIME_MODULE,
        )
        .expect("writing to String cannot fail");
        writeln!(
            output,
            "    data: {payload_type} = {}.jsoncompat_field(\"data\")\n",
            DATACLASSES_RUNTIME_MODULE,
        )
        .expect("writing to String cannot fail");
    }

    Ok(())
}

fn schema_object_with_root_defs(
    schema: &Map<String, Value>,
    root: &Map<String, Value>,
) -> Result<Map<String, Value>, DataclassError> {
    let mut schema = schema.clone();
    let Some(root_defs) = root.get("$defs") else {
        return Ok(schema);
    };
    let root_defs = root_defs
        .as_object()
        .ok_or_else(|| invalid_schema("#/$defs".to_owned(), "$defs must be an object"))?;

    let mut defs = schema
        .remove("$defs")
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();
    for (key, value) in root_defs {
        defs.entry(key.clone()).or_insert_with(|| value.clone());
    }
    schema.insert("$defs".to_owned(), Value::Object(defs));
    Ok(schema)
}

fn render_reader_root_class(
    output: &mut String,
    reader: &Map<String, Value>,
) -> Result<(), DataclassError> {
    let metadata = parse_metadata(reader, "#")?;
    let JsoncompatMetadata::Reader { name, .. } = metadata else {
        return Err(invalid_schema(
            join_pointer("#", JSONCOMPAT_METADATA_KEY),
            "reader schema must have reader metadata",
        ));
    };

    let branches = reader
        .get("oneOf")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid_schema("#/oneOf".to_owned(), "oneOf must be an array"))?;
    let mut variant_names = Vec::new();
    for (index, branch) in branches.iter().enumerate() {
        let pointer = format!("#/oneOf/{index}");
        variant_names.push(metadata_name(
            expect_schema_object(branch, &pointer)?,
            &pointer,
        )?);
    }

    writeln!(output, "@dataclass(frozen=True, slots=True, kw_only=True)")
        .expect("writing to String cannot fail");
    writeln!(
        output,
        "class {name}({}):",
        runtime_dataclass_symbol(READER_ROOT_MODEL_CLASS),
    )
    .expect("writing to String cannot fail");
    writeln!(
        output,
        "    __jsoncompat_schema__: typing.ClassVar[str] = {}",
        python_string_literal(&canonical_schema_literal(&Value::Object(reader.clone())))
    )
    .expect("writing to String cannot fail");
    writeln!(
        output,
        "    root: {} = {}.jsoncompat_root_field()",
        union_annotation(&variant_names),
        DATACLASSES_RUNTIME_MODULE,
    )
    .expect("writing to String cannot fail");
    Ok(())
}

fn render_class_runtime_spec(output: &mut String, class_spec: &ClassSpec) {
    match &class_spec.kind {
        ClassKind::Object {
            fields,
            extra_annotation,
        } => {
            writeln!(
                output,
                "{}.__jsoncompat_object_spec__ = {}.jsoncompat_object_spec(",
                class_spec.name, DATACLASSES_RUNTIME_MODULE,
            )
            .expect("writing to String cannot fail");
            for field in fields {
                let annotation = if field.required {
                    field.annotation.clone()
                } else {
                    format!(
                        "({} | {})",
                        field.annotation,
                        runtime_dataclass_symbol(MISSING_TYPE_NAME),
                    )
                };
                if field.required {
                    writeln!(
                        output,
                        "    {}.jsoncompat_field_spec({}, {}, {}),",
                        DATACLASSES_RUNTIME_MODULE,
                        python_string_literal(&field.py_name),
                        python_string_literal(&field.json_name),
                        annotation,
                    )
                    .expect("writing to String cannot fail");
                } else {
                    writeln!(
                        output,
                        "    {}.jsoncompat_field_spec({}, {}, {}, omittable=True),",
                        DATACLASSES_RUNTIME_MODULE,
                        python_string_literal(&field.py_name),
                        python_string_literal(&field.json_name),
                        annotation,
                    )
                    .expect("writing to String cannot fail");
                }
            }
            if let Some(extra_annotation) = extra_annotation {
                writeln!(
                    output,
                    "    extra_annotation=dict[str, {extra_annotation}],"
                )
                .expect("writing to String cannot fail");
            }
            writeln!(output, ")").expect("writing to String cannot fail");
        }
        ClassKind::Root { annotation } => {
            writeln!(
                output,
                "{}.__jsoncompat_root_annotation__ = {annotation}",
                class_spec.name
            )
            .expect("writing to String cannot fail");
        }
    }
}

fn render_writer_runtime_spec(
    output: &mut String,
    writer: &Map<String, Value>,
) -> Result<(), DataclassError> {
    let metadata = parse_metadata(writer, "#")?;
    let JsoncompatMetadata::Writer {
        name,
        version,
        payload_ref,
        ..
    } = metadata
    else {
        return Err(invalid_schema(
            join_pointer("#", JSONCOMPAT_METADATA_KEY),
            "writer schema must have writer metadata",
        ));
    };
    let payload_type = resolve_schema_ref_name(writer, &payload_ref, "#")?;

    writeln!(
        output,
        "{name}.__jsoncompat_object_spec__ = {}.jsoncompat_object_spec(",
        DATACLASSES_RUNTIME_MODULE,
    )
    .expect("writing to String cannot fail");
    writeln!(
        output,
        "    {}.jsoncompat_field_spec(\"version\", \"version\", typing.Literal[{version}]),",
        DATACLASSES_RUNTIME_MODULE,
    )
    .expect("writing to String cannot fail");
    writeln!(
        output,
        "    {}.jsoncompat_field_spec(\"data\", \"data\", {payload_type}),",
        DATACLASSES_RUNTIME_MODULE,
    )
    .expect("writing to String cannot fail");
    writeln!(output, ")").expect("writing to String cannot fail");
    Ok(())
}

fn render_reader_variant_runtime_specs(
    output: &mut String,
    reader: &Map<String, Value>,
) -> Result<(), DataclassError> {
    let branches = reader
        .get("oneOf")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid_schema("#/oneOf".to_owned(), "oneOf must be an array"))?;

    for (index, branch) in branches.iter().enumerate() {
        let pointer = format!("#/oneOf/{index}");
        let branch = expect_schema_object(branch, &pointer)?;
        let metadata = parse_metadata(branch, &pointer)?;
        let JsoncompatMetadata::ReaderVariant {
            name,
            version,
            payload_ref,
            ..
        } = metadata
        else {
            return Err(invalid_schema(
                join_pointer(&pointer, JSONCOMPAT_METADATA_KEY),
                "reader branch must have reader_variant metadata",
            ));
        };
        let payload_type = resolve_schema_ref_name(reader, &payload_ref, &pointer)?;

        writeln!(
            output,
            "{name}.__jsoncompat_object_spec__ = {}.jsoncompat_object_spec(",
            DATACLASSES_RUNTIME_MODULE,
        )
        .expect("writing to String cannot fail");
        writeln!(
            output,
            "    {}.jsoncompat_field_spec(\"version\", \"version\", typing.Literal[{version}]),",
            DATACLASSES_RUNTIME_MODULE,
        )
        .expect("writing to String cannot fail");
        writeln!(
            output,
            "    {}.jsoncompat_field_spec(\"data\", \"data\", {payload_type}),",
            DATACLASSES_RUNTIME_MODULE,
        )
        .expect("writing to String cannot fail");
        writeln!(output, ")\n").expect("writing to String cannot fail");
    }

    Ok(())
}

fn render_reader_root_runtime_spec(
    output: &mut String,
    reader: &Map<String, Value>,
) -> Result<(), DataclassError> {
    let metadata = parse_metadata(reader, "#")?;
    let JsoncompatMetadata::Reader { name, .. } = metadata else {
        return Err(invalid_schema(
            join_pointer("#", JSONCOMPAT_METADATA_KEY),
            "reader schema must have reader metadata",
        ));
    };

    let branches = reader
        .get("oneOf")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid_schema("#/oneOf".to_owned(), "oneOf must be an array"))?;
    let mut variant_names = Vec::new();
    for (index, branch) in branches.iter().enumerate() {
        let pointer = format!("#/oneOf/{index}");
        variant_names.push(metadata_name(
            expect_schema_object(branch, &pointer)?,
            &pointer,
        )?);
    }

    writeln!(
        output,
        "{name}.__jsoncompat_root_annotation__ = {}",
        union_annotation(&variant_names)
    )
    .expect("writing to String cannot fail");
    Ok(())
}

fn emit_nested_defs(
    builder: &mut DataclassModuleBuilder,
    obj: &Map<String, Value>,
    pointer: &str,
) -> Result<(), DataclassError> {
    let Some(defs) = obj.get("$defs") else {
        return Ok(());
    };
    let defs = defs
        .as_object()
        .ok_or_else(|| invalid_schema(join_pointer(pointer, "$defs"), "$defs must be an object"))?;
    for (def_key, schema) in defs {
        builder.register_named_declaration(
            schema,
            &join_pointer(
                &join_pointer(pointer, "$defs"),
                &escape_pointer_token(def_key),
            ),
        )?;
    }
    Ok(())
}

fn emit_root_defs(
    builder: &mut DataclassModuleBuilder,
    schema: &Value,
) -> Result<(), DataclassError> {
    let root = expect_schema_object(schema, "#")?;
    emit_nested_defs(builder, root, "#")
}

fn parse_object_fields(
    builder: &mut DataclassModuleBuilder,
    obj: &Map<String, Value>,
    pointer: &str,
    scope_name: &str,
) -> Result<Vec<FieldSpec>, DataclassError> {
    let required = parse_required_fields(obj, pointer)?;
    let mut fields = Vec::new();
    let mut used_py_names = BTreeSet::from([EXTRA_FIELD_NAME.to_owned()]);
    let properties = match obj.get("properties") {
        None => Map::new(),
        Some(properties) => properties.as_object().cloned().ok_or_else(|| {
            invalid_schema(
                join_pointer(pointer, "properties"),
                "properties must be an object",
            )
        })?,
    };

    for (property_name, schema) in &properties {
        let property_pointer = join_pointer(
            &join_pointer(pointer, "properties"),
            &escape_pointer_token(property_name),
        );
        let py_name = unique_name(&python_field_name(property_name), &used_py_names);
        used_py_names.insert(py_name.clone());
        fields.push(FieldSpec {
            json_name: property_name.clone(),
            py_name,
            annotation: builder.inline_annotation(
                schema,
                &property_pointer,
                scope_name,
                property_name,
            )?,
            required: required.contains(property_name),
        });
    }

    let fallback_annotation =
        required_property_fallback_annotation(builder, obj, pointer, scope_name)?;
    for property_name in required {
        if properties.contains_key(&property_name) {
            continue;
        }
        let py_name = unique_name(&python_field_name(&property_name), &used_py_names);
        used_py_names.insert(py_name.clone());
        fields.push(FieldSpec {
            json_name: property_name.clone(),
            py_name,
            annotation: fallback_annotation.clone(),
            required: true,
        });
    }

    Ok(fields)
}

fn parse_extra_annotation(
    builder: &mut DataclassModuleBuilder,
    obj: &Map<String, Value>,
    pointer: &str,
    scope_name: &str,
) -> Result<Option<String>, DataclassError> {
    let Some(additional_properties) = obj.get("additionalProperties") else {
        return Ok(Some(typing_symbol("Any")));
    };

    match additional_properties {
        Value::Bool(true) => Ok(Some(typing_symbol("Any"))),
        Value::Bool(false) => Ok(None),
        schema => Ok(Some(builder.inline_annotation(
            schema,
            &join_pointer(pointer, "additionalProperties"),
            scope_name,
            "ExtraValue",
        )?)),
    }
}

fn required_property_fallback_annotation(
    builder: &mut DataclassModuleBuilder,
    obj: &Map<String, Value>,
    pointer: &str,
    scope_name: &str,
) -> Result<String, DataclassError> {
    match obj.get("additionalProperties") {
        None | Some(Value::Bool(true)) => Ok(typing_symbol("Any")),
        Some(Value::Bool(false)) => Ok(typing_symbol("Any")),
        Some(schema) => builder.inline_annotation(
            schema,
            &join_pointer(pointer, "additionalProperties"),
            scope_name,
            "ExtraValue",
        ),
    }
}

fn parse_required_fields(
    obj: &Map<String, Value>,
    pointer: &str,
) -> Result<BTreeSet<String>, DataclassError> {
    let Some(required) = obj.get("required") else {
        return Ok(BTreeSet::new());
    };
    let required = required.as_array().ok_or_else(|| {
        invalid_schema(
            join_pointer(pointer, "required"),
            "required must be an array",
        )
    })?;
    let mut result = BTreeSet::new();
    for (index, item) in required.iter().enumerate() {
        let Some(field_name) = item.as_str() else {
            return Err(invalid_schema(
                join_pointer(&join_pointer(pointer, "required"), &index.to_string()),
                "required entries must be strings",
            ));
        };
        result.insert(field_name.to_owned());
    }
    Ok(result)
}

fn parse_literal_annotation(obj: &Map<String, Value>) -> Option<String> {
    if let Some(value) = obj.get("const") {
        return Some(python_literal_annotation(value).unwrap_or_else(|| typing_symbol("Any")));
    }
    let values = obj.get("enum")?.as_array()?;
    if values.is_empty() {
        return Some(typing_symbol("Any"));
    }
    if values
        .iter()
        .any(|value| python_literal_annotation(value).is_none())
    {
        return Some(typing_symbol("Any"));
    }
    Some(union_annotation(
        values
            .iter()
            .map(|value| {
                python_literal_annotation(value)
                    .expect("unsupported enum values are filtered above")
            })
            .collect::<Vec<_>>()
            .as_slice(),
    ))
}

fn parse_type_annotation(
    obj: &Map<String, Value>,
    pointer: &str,
) -> Result<Option<String>, DataclassError> {
    let Some(type_value) = obj.get("type") else {
        return Ok(None);
    };

    match type_value {
        Value::String(type_name) => Ok(Some(single_type_annotation(type_name, pointer)?)),
        Value::Array(type_names) => {
            if type_names.is_empty() {
                return Err(invalid_schema(
                    join_pointer(pointer, "type"),
                    "type array must not be empty",
                ));
            }
            let mut annotations = Vec::new();
            for (index, type_name) in type_names.iter().enumerate() {
                let Some(type_name) = type_name.as_str() else {
                    return Err(invalid_schema(
                        join_pointer(&join_pointer(pointer, "type"), &index.to_string()),
                        "type array entries must be strings",
                    ));
                };
                annotations.push(single_type_annotation(type_name, pointer)?);
            }
            Ok(Some(union_annotation(&annotations)))
        }
        _ => Err(invalid_schema(
            join_pointer(pointer, "type"),
            "type must be a string or an array of strings",
        )),
    }
}

fn single_type_annotation(type_name: &str, pointer: &str) -> Result<String, DataclassError> {
    match type_name {
        "string" => Ok("str".to_owned()),
        "integer" => Ok("int".to_owned()),
        "number" => Ok("float".to_owned()),
        "boolean" => Ok("bool".to_owned()),
        "null" => Ok("None".to_owned()),
        "array" => Ok(format!("list[{}]", typing_symbol("Any"))),
        "object" => Ok(format!("dict[str, {}]", typing_symbol("Any"))),
        _ => Err(invalid_schema(
            join_pointer(pointer, "type"),
            format!("unsupported JSON Schema type '{type_name}'"),
        )),
    }
}

fn collect_named_refs(schema: &Value) -> Result<BTreeMap<String, String>, DataclassError> {
    let mut refs = BTreeMap::new();
    let mut used_names = BTreeSet::from([
        DATACLASS_MODEL_CLASS.to_owned(),
        DATACLASS_ROOT_MODEL_CLASS.to_owned(),
        EXTRA_FIELD_NAME.to_owned(),
        MISSING_TYPE_NAME.to_owned(),
        READER_MODEL_CLASS.to_owned(),
        READER_ROOT_MODEL_CLASS.to_owned(),
        WRITER_MODEL_CLASS.to_owned(),
    ]);

    let root_name = root_schema_name(schema, "#", &used_names)?;
    reserve_named_ref(&mut refs, &mut used_names, "#", &root_name)?;
    let scope_name = refs
        .get("#")
        .cloned()
        .expect("root declaration name inserted above");
    collect_schema_refs(schema, "#", &scope_name, &mut refs, &mut used_names)?;
    Ok(refs)
}

fn collect_schema_refs(
    schema: &Value,
    pointer: &str,
    scope_name: &str,
    refs: &mut BTreeMap<String, String>,
    used_names: &mut BTreeSet<String>,
) -> Result<(), DataclassError> {
    let Value::Object(obj) = schema else {
        return Ok(());
    };

    if let Some(defs) = obj.get("$defs") {
        let defs = defs.as_object().ok_or_else(|| {
            invalid_schema(join_pointer(pointer, "$defs"), "$defs must be an object")
        })?;
        for (def_key, schema) in defs {
            let def_pointer = join_pointer(
                &join_pointer(pointer, "$defs"),
                &escape_pointer_token(def_key),
            );
            let def_name =
                declaration_ref_name(schema, &def_pointer, scope_name, def_key, used_names)?;
            reserve_named_ref(refs, used_names, &def_pointer, &def_name)?;
            collect_schema_refs(schema, &def_pointer, &def_name, refs, used_names)?;
        }
    }

    if let Some(properties) = obj.get("properties") {
        let properties = properties.as_object().ok_or_else(|| {
            invalid_schema(
                join_pointer(pointer, "properties"),
                "properties must be an object",
            )
        })?;
        for (property_name, schema) in properties {
            collect_schema_refs(
                schema,
                &join_pointer(
                    &join_pointer(pointer, "properties"),
                    &escape_pointer_token(property_name),
                ),
                scope_name,
                refs,
                used_names,
            )?;
        }
    }

    if let Some(items) = obj.get("items") {
        collect_schema_refs(
            items,
            &join_pointer(pointer, "items"),
            scope_name,
            refs,
            used_names,
        )?;
    }

    if let Some(additional_properties) = obj.get("additionalProperties") {
        collect_schema_refs(
            additional_properties,
            &join_pointer(pointer, "additionalProperties"),
            scope_name,
            refs,
            used_names,
        )?;
    }

    for keyword in ["oneOf", "anyOf"] {
        let Some(branches) = obj.get(keyword) else {
            continue;
        };
        let branches = branches.as_array().ok_or_else(|| {
            invalid_schema(
                join_pointer(pointer, keyword),
                format!("{keyword} must be an array"),
            )
        })?;
        for (index, branch) in branches.iter().enumerate() {
            collect_schema_refs(
                branch,
                &join_pointer(&join_pointer(pointer, keyword), &index.to_string()),
                scope_name,
                refs,
                used_names,
            )?;
        }
    }

    Ok(())
}

fn reserve_named_ref(
    refs: &mut BTreeMap<String, String>,
    used_names: &mut BTreeSet<String>,
    pointer: &str,
    name: &str,
) -> Result<(), DataclassError> {
    if used_names.insert(name.to_owned()) {
        refs.insert(pointer.to_owned(), name.to_owned());
        Ok(())
    } else {
        Err(DataclassError::DuplicateDeclaration {
            name: name.to_owned(),
        })
    }
}

fn root_schema_name(
    schema: &Value,
    pointer: &str,
    used_names: &BTreeSet<String>,
) -> Result<String, DataclassError> {
    match parse_optional_metadata(schema, pointer)? {
        Some(JsoncompatMetadata::Declaration { name, .. })
        | Some(JsoncompatMetadata::Writer { name, .. })
        | Some(JsoncompatMetadata::Reader { name, .. })
        | Some(JsoncompatMetadata::ReaderVariant { name, .. }) => Ok(name),
        None => declaration_ref_name(schema, pointer, "Generated", "Schema", used_names),
    }
}

fn declaration_ref_name(
    schema: &Value,
    pointer: &str,
    scope_name: &str,
    fallback_hint: &str,
    used_names: &BTreeSet<String>,
) -> Result<String, DataclassError> {
    if let Some(name) = metadata_name_if_any(schema, pointer)? {
        return Ok(name);
    }
    if let Some(title) = schema.get("title").and_then(Value::as_str) {
        let candidate = unique_name(&pascal_case(title), used_names);
        return Ok(candidate);
    }
    Ok(unique_name(
        &format!("{scope_name}{}", pascal_case(fallback_hint)),
        used_names,
    ))
}

fn resolve_schema_ref_name(
    root: &Map<String, Value>,
    ref_value: &str,
    pointer: &str,
) -> Result<String, DataclassError> {
    let target = resolve_ref_pointer(root, ref_value, pointer)?;
    metadata_name(expect_schema_object(target, ref_value)?, ref_value)
}

fn resolve_local_ref_name(
    refs: &BTreeMap<String, String>,
    ref_value: &str,
    pointer: &str,
) -> Result<String, DataclassError> {
    if !ref_value.starts_with('#') {
        return Err(DataclassError::UnsupportedRef {
            pointer: join_pointer(pointer, "$ref"),
            ref_value: ref_value.to_owned(),
        });
    }
    refs.get(ref_value)
        .cloned()
        .ok_or_else(|| DataclassError::UnsupportedRef {
            pointer: join_pointer(pointer, "$ref"),
            ref_value: ref_value.to_owned(),
        })
}

fn resolve_ref_pointer<'a>(
    root: &'a Map<String, Value>,
    ref_value: &str,
    pointer: &str,
) -> Result<&'a Value, DataclassError> {
    if !ref_value.starts_with("#/$defs/") {
        return Err(DataclassError::UnsupportedRef {
            pointer: join_pointer(pointer, "$ref"),
            ref_value: ref_value.to_owned(),
        });
    }

    let mut current = root
        .get("$defs")
        .ok_or_else(|| DataclassError::UnsupportedRef {
            pointer: join_pointer(pointer, "$ref"),
            ref_value: ref_value.to_owned(),
        })?;

    for segment in ref_value
        .trim_start_matches("#/$defs/")
        .split('/')
        .map(unescape_pointer_token)
    {
        let Some(object) = current.as_object() else {
            return Err(DataclassError::UnsupportedRef {
                pointer: join_pointer(pointer, "$ref"),
                ref_value: ref_value.to_owned(),
            });
        };
        current = object
            .get(&segment)
            .ok_or_else(|| DataclassError::UnsupportedRef {
                pointer: join_pointer(pointer, "$ref"),
                ref_value: ref_value.to_owned(),
            })?;
    }

    Ok(current)
}

fn parse_metadata(
    schema: &Map<String, Value>,
    pointer: &str,
) -> Result<JsoncompatMetadata, DataclassError> {
    let Some(metadata) = schema.get(JSONCOMPAT_METADATA_KEY) else {
        return Err(DataclassError::MissingDeclarationMetadata {
            pointer: pointer.to_owned(),
            metadata_key: JSONCOMPAT_METADATA_KEY,
        });
    };
    serde_json::from_value(metadata.clone()).map_err(|error| {
        invalid_schema(
            join_pointer(pointer, JSONCOMPAT_METADATA_KEY),
            format!("invalid metadata: {error}"),
        )
    })
}

fn parse_optional_metadata(
    schema: &Value,
    pointer: &str,
) -> Result<Option<JsoncompatMetadata>, DataclassError> {
    let Some(obj) = schema.as_object() else {
        return Ok(None);
    };
    let Some(metadata) = obj.get(JSONCOMPAT_METADATA_KEY) else {
        return Ok(None);
    };
    serde_json::from_value(metadata.clone())
        .map(Some)
        .map_err(|error| {
            invalid_schema(
                join_pointer(pointer, JSONCOMPAT_METADATA_KEY),
                format!("invalid metadata: {error}"),
            )
        })
}

fn metadata_name(schema: &Map<String, Value>, pointer: &str) -> Result<String, DataclassError> {
    match parse_metadata(schema, pointer)? {
        JsoncompatMetadata::Declaration { name, .. }
        | JsoncompatMetadata::Writer { name, .. }
        | JsoncompatMetadata::Reader { name, .. }
        | JsoncompatMetadata::ReaderVariant { name, .. } => Ok(name),
    }
}

fn metadata_name_if_any(schema: &Value, pointer: &str) -> Result<Option<String>, DataclassError> {
    Ok(
        parse_optional_metadata(schema, pointer)?.map(|metadata| match metadata {
            JsoncompatMetadata::Declaration { name, .. }
            | JsoncompatMetadata::Writer { name, .. }
            | JsoncompatMetadata::Reader { name, .. }
            | JsoncompatMetadata::ReaderVariant { name, .. } => name,
        }),
    )
}

fn expect_schema_object<'a>(
    schema: &'a Value,
    pointer: &str,
) -> Result<&'a Map<String, Value>, DataclassError> {
    schema
        .as_object()
        .ok_or_else(|| invalid_schema(pointer.to_owned(), "schema must be an object"))
}

fn union_branch_context(schema: &Map<String, Value>, keyword: &str) -> Map<String, Value> {
    let mut context = schema.clone();
    context.remove(keyword);
    context
}

fn merge_union_branch_schema(
    branch: &Value,
    context: &Map<String, Value>,
    pointer: &str,
) -> Result<Value, DataclassError> {
    if context.is_empty() {
        return Ok(branch.clone());
    }
    match branch {
        Value::Bool(false) => Ok(Value::Bool(false)),
        Value::Bool(true) => Ok(Value::Object(context.clone())),
        Value::Object(branch_obj) => {
            let mut merged = context.clone();
            for (key, value) in branch_obj {
                if key == "$defs" {
                    let mut defs = merged
                        .remove("$defs")
                        .and_then(|existing| existing.as_object().cloned())
                        .unwrap_or_default();
                    let branch_defs = value.as_object().ok_or_else(|| {
                        invalid_schema(join_pointer(pointer, "$defs"), "$defs must be an object")
                    })?;
                    for (def_key, def_value) in branch_defs {
                        defs.insert(def_key.clone(), def_value.clone());
                    }
                    merged.insert("$defs".to_owned(), Value::Object(defs));
                } else {
                    merged.insert(key.clone(), value.clone());
                }
            }
            Ok(Value::Object(merged))
        }
        _ => Err(invalid_schema(
            pointer.to_owned(),
            "oneOf/anyOf branches must be objects or booleans",
        )),
    }
}

fn is_object_schema(obj: &Map<String, Value>) -> bool {
    if obj.contains_key("properties") || obj.contains_key("required") {
        return true;
    }
    matches!(obj.get("type"), Some(Value::String(type_name)) if type_name == "object")
}

fn union_annotation(annotations: &[String]) -> String {
    let mut unique = BTreeSet::new();
    for annotation in annotations {
        unique.insert(annotation.clone());
    }
    match unique.len() {
        0 => typing_symbol("Any"),
        1 => unique.into_iter().next().expect("len checked above"),
        _ => format!("({})", unique.into_iter().collect::<Vec<_>>().join(" | ")),
    }
}

fn canonical_schema_literal(schema: &Value) -> String {
    serde_json::to_string(&canonicalize_json(schema)).expect("canonical schema is valid JSON")
}

fn render_class_base(class_spec: &ClassSpec) -> String {
    match (&class_spec.kind, class_spec.base_class) {
        (
            ClassKind::Object {
                extra_annotation: Some(extra_annotation),
                ..
            },
            DATACLASS_MODEL_CLASS,
        ) => format!(
            "{}[{extra_annotation}]",
            runtime_dataclass_symbol(DATACLASS_ADDITIONAL_MODEL_CLASS),
        ),
        _ => runtime_dataclass_symbol(class_spec.base_class),
    }
}

fn runtime_dataclass_symbol(name: &str) -> String {
    format!("{DATACLASSES_RUNTIME_MODULE}.{name}")
}

fn typing_symbol(name: &str) -> String {
    format!("typing.{name}")
}

fn python_string_literal(value: &str) -> String {
    serde_json::to_string(value).expect("Python string literal source is valid JSON")
}

fn python_json_literal(value: &Value) -> String {
    match value {
        Value::Null => "None".to_owned(),
        Value::Bool(true) => "True".to_owned(),
        Value::Bool(false) => "False".to_owned(),
        Value::Number(number) => number.to_string(),
        Value::String(text) => python_string_literal(text),
        Value::Array(_) | Value::Object(_) => typing_symbol("Any"),
    }
}

fn python_literal_annotation(value: &Value) -> Option<String> {
    match value {
        Value::Null | Value::Bool(_) | Value::String(_) => {
            Some(format!("typing.Literal[{}]", python_json_literal(value)))
        }
        Value::Number(number) if number.to_string().starts_with('-') => {
            if number.is_i64() {
                Some("int".to_owned())
            } else {
                Some("float".to_owned())
            }
        }
        Value::Number(_) => Some(format!("typing.Literal[{}]", python_json_literal(value))),
        Value::Array(_) | Value::Object(_) => None,
    }
}

fn python_field_name(json_name: &str) -> String {
    let mut output = String::new();
    for (index, ch) in json_name.chars().enumerate() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            if index == 0 && ch.is_ascii_digit() {
                output.push('_');
            }
            output.push(ch);
        } else {
            output.push('_');
        }
    }
    if output.is_empty() {
        output.push_str("field_");
    }
    if python_keyword_or_reserved(&output) {
        output.push('_');
    }
    output
}

fn python_keyword_or_reserved(name: &str) -> bool {
    matches!(
        name,
        "False"
            | "None"
            | "True"
            | "and"
            | "as"
            | "assert"
            | "async"
            | "await"
            | "break"
            | "class"
            | "continue"
            | "def"
            | "del"
            | "elif"
            | "else"
            | "except"
            | "finally"
            | "for"
            | "from"
            | "global"
            | "if"
            | "import"
            | "in"
            | "is"
            | "lambda"
            | "nonlocal"
            | "not"
            | "or"
            | "pass"
            | "raise"
            | "return"
            | "try"
            | "while"
            | "with"
            | "yield"
            | "from_json"
            | "from_json_string"
            | "to_json"
            | "to_json_string"
            | "root"
            | EXTRA_FIELD_NAME
    )
}

fn unique_name(base_name: &str, used_names: &BTreeSet<String>) -> String {
    let base_name = if base_name.is_empty() {
        "GeneratedSchema".to_owned()
    } else {
        base_name.to_owned()
    };

    if !used_names.contains(&base_name) {
        return base_name;
    }

    let mut counter = 2;
    loop {
        let candidate = format!("{base_name}{counter}");
        if !used_names.contains(&candidate) {
            return candidate;
        }
        counter += 1;
    }
}

fn pascal_case(value: &str) -> String {
    let mut result = String::new();
    let mut uppercase_next = true;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            if uppercase_next {
                result.push(ch.to_ascii_uppercase());
            } else {
                result.push(ch);
            }
            uppercase_next = false;
        } else {
            uppercase_next = true;
        }
    }
    if result.is_empty() {
        result.push_str("GeneratedSchema");
    }
    if result
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_digit())
    {
        result.insert(0, 'T');
    }
    result
}

fn join_pointer(base: &str, token: &str) -> String {
    if base == "#" {
        format!("#/{token}")
    } else {
        format!("{base}/{token}")
    }
}

fn escape_pointer_token(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

fn unescape_pointer_token(token: &str) -> String {
    token.replace("~1", "/").replace("~0", "~")
}

fn invalid_schema(pointer: String, message: impl Into<String>) -> DataclassError {
    DataclassError::InvalidSchema {
        pointer,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn generate_dataclass_models_emits_plain_dataclass_root_alias() {
        let schema = json!({
            "title": "user profile",
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            },
            "required": ["name"],
            "additionalProperties": false
        });

        let source = generate_dataclass_models(&schema).unwrap();

        assert!(source.contains("class UserProfile(jsoncompat_dataclasses.DataclassModel):"));
        assert!(source.contains("name: str = jsoncompat_dataclasses.jsoncompat_field(\"name\")"));
        assert!(source.contains("JSONCOMPAT_MODEL = UserProfile"));
    }

    #[test]
    fn generate_dataclass_models_emits_writer_class_from_metadata() {
        let schema = json!({
            "type": "object",
            "properties": {
                "version": { "const": 1 },
                "data": { "$ref": "#/$defs/v1" }
            },
            "required": ["version", "data"],
            "additionalProperties": false,
            "$defs": {
                "v1": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" }
                    },
                    "required": ["name"],
                    "additionalProperties": false,
                    "x-jsoncompat": {
                        "kind": "declaration",
                        "stable_id": "user-profile",
                        "name": "UserProfileV1",
                        "version": 1,
                        "schema_ref": "#/$defs/v1"
                    }
                }
            },
            "x-jsoncompat": {
                "kind": "writer",
                "stable_id": "user-profile",
                "name": "UserProfileWriter",
                "version": 1,
                "payload_ref": "#/$defs/v1"
            }
        });

        let source = generate_dataclass_models(&schema).unwrap();

        assert!(source.contains("class UserProfileV1(jsoncompat_dataclasses.DataclassModel):"));
        assert!(
            source
                .contains("class UserProfileWriter(jsoncompat_dataclasses.WriterDataclassModel):")
        );
        assert!(source.contains("JSONCOMPAT_MODEL = UserProfileWriter"));
    }
}
