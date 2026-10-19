//! Public declarations first; precomputed implementation in a separable footer.

use super::plan::{ConversionNode, PreparedModule, ScalarKind};
use super::{
    ClassKind, ClassSpec, DataclassError, EXTRA_FIELD_NAME, omittable_annotation,
    python_json_literal, python_string_literal, render_class_base,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

pub(super) const GENERATED_FORMAT_VERSION: u8 = 2;

/// Trusted Python metadata for generated declarations. Keys are public class
/// and Python field names. Annotation and base overrides do not change validation plans.
#[derive(Default)]
pub struct DataclassMetadata {
    pub imports: String,
    /// Additional imports needed only by classes with replacement bases.
    pub base_imports: String,
    pub fields: BTreeMap<String, BTreeMap<String, String>>,
    /// Trusted replacement base expressions for generated object declarations.
    pub bases: BTreeMap<String, String>,
    /// Public declarations listed first, in the requested reading order.
    pub declaration_order: Vec<String>,
    /// Trusted source emitted immediately before/after a public declaration.
    pub before: BTreeMap<String, String>,
    pub after: BTreeMap<String, String>,
    /// Human-facing descriptions; encoded as Python string literals.
    pub docstrings: BTreeMap<String, String>,
    /// Public names emitted with a class (for resource facade re-exports).
    pub extra_exports: BTreeMap<String, Vec<String>>,
    /// Names emitted beside models which generated inline helpers must avoid.
    pub reserved_names: BTreeSet<String>,
}

/// Declarations and prepared implementation for one logical model group.
/// Render as a single module, a public/private pair, or package shards; the
/// public facade passes shared classes to the binder after declaring them.
pub struct DataclassModule {
    pub(super) parts: super::package::Parts,
    pub imports: String,
    pub declarations: String,
    pub implementation: String,
    pub models: Vec<String>,
}
impl DataclassModule {
    pub fn single_file(&self) -> String {
        format!(
            "{}{}\n# --- Generated implementation: regenerate this file to change it. ---\n\n{}\n_jsoncompat_bind({})\n",
            self.imports,
            self.declarations,
            self.implementation,
            self.model_tuple()
        )
    }
    pub fn public_file(&self, companion: &str) -> String {
        format!(
            "{}if not typing.TYPE_CHECKING:\n    if __package__ not in (None, \"\"):\n        from .{companion} import bind_models as _jsoncompat_bind\n    else:\n        from {companion} import bind_models as _jsoncompat_bind\n\n{}\nif not typing.TYPE_CHECKING:\n    _jsoncompat_bind({})\n    del _jsoncompat_bind\n",
            self.imports,
            self.declarations,
            self.model_tuple()
        )
    }
    pub fn private_file(&self) -> String {
        format!(
            "# Generated implementation. Import the public model module instead.\nfrom __future__ import annotations\n\nfrom jsoncompat.codegen import dataclasses as dc\n\n__all__ = (\"bind_models\",)\n\n{}\nbind_models = _jsoncompat_bind\n",
            self.implementation
        )
    }
    fn model_tuple(&self) -> String {
        format!("({},)", self.models.join(", "))
    }
}

struct Field {
    name: String,
    annotation: String,
    input_annotation: String,
    expression: String,
    default: Option<&'static str>,
    description: Option<String>,
}
fn fields(class: &ClassSpec) -> Vec<Field> {
    match &class.kind {
        ClassKind::Root { annotation } => vec![Field {
            name: "root".into(),
            annotation: annotation.to_string(),
            input_annotation: annotation.input_source().to_owned(),
            expression: "dc.root_field()".into(),
            default: None,
            description: None,
        }],
        ClassKind::Object {
            fields,
            extra_annotation,
        } => {
            let mut result = fields
                .iter()
                .map(|field| Field {
                    name: field.py_name.clone(),
                    annotation: if field.required {
                        field.annotation.to_string()
                    } else {
                        omittable_annotation(&field.annotation)
                    },
                    input_annotation: if field.required {
                        field.annotation.input_source().to_owned()
                    } else {
                        omittable_annotation(field.annotation.input_source())
                    },
                    expression: format!(
                        "dc.field({}{})",
                        python_string_literal(&field.json_name),
                        if field.required {
                            ""
                        } else {
                            ", default=dc.MISSING"
                        }
                    ),
                    description: field.description.clone(),
                    default: if field.required {
                        None
                    } else {
                        Some("dc.MissingType.MISSING")
                    },
                })
                .collect::<Vec<_>>();
            if let Some(annotation) = extra_annotation {
                result.push(Field {
                    name: EXTRA_FIELD_NAME.into(),
                    annotation: format!("collections.abc.Mapping[str, {annotation}]"),
                    input_annotation: format!(
                        "collections.abc.Mapping[str, {}]",
                        annotation.input_source()
                    ),
                    expression: "dc.extra_field(default_factory=dict)".into(),
                    default: Some("dc.EXTRA_DEFAULT"),
                    description: None,
                });
            }
            result
        }
    }
}
fn tuple(items: impl IntoIterator<Item = String>) -> String {
    let items = items.into_iter().collect::<Vec<_>>();
    if items.is_empty() {
        "()".into()
    } else {
        format!("({},)", items.join(", "))
    }
}
pub(super) fn bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() + 3);
    out.push_str("b'");
    for byte in bytes {
        match byte {
            b'\'' => out.push_str("\\'"),
            b'\\' => out.push_str("\\\\"),
            32..=126 => out.push(*byte as char),
            _ => {
                out.push_str("\\x");
                out.push(HEX[(byte >> 4) as usize] as char);
                out.push(HEX[(byte & 15) as usize] as char);
            }
        }
    }
    out.push('\'');
    out
}

/// Format annotations structurally without importing a Python formatter at
/// generation time. String literals are opaque, including commas and pipes.
fn annotation_parts(source: &str, separator: char) -> Vec<&str> {
    let mut depth = 0;
    let mut quote = None;
    let mut escaped = false;
    let mut start = 0;
    let mut parts = Vec::new();
    for (index, ch) in source.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if let Some(end) = quote {
            if ch == '\\' {
                escaped = true;
            } else if ch == end {
                quote = None;
            }
            continue;
        }
        match ch {
            '\'' | '"' => quote = Some(ch),
            '[' | '(' | '{' => depth += 1,
            ']' | ')' | '}' => depth -= 1,
            _ if depth == 0 && ch == separator => {
                parts.push(source[start..index].trim());
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(source[start..].trim());
    parts
}

fn wrap_annotation(source: &str, column: usize, indent: usize) -> String {
    if column + source.len() <= 92 {
        return source.to_owned();
    }
    let padding = " ".repeat(indent);
    let inner_padding = " ".repeat(indent + 4);
    if let Some(inner) = source.strip_prefix('(').and_then(|s| s.strip_suffix(')')) {
        let parts = annotation_parts(inner, '|');
        if parts.len() > 1 {
            let lines = parts
                .iter()
                .enumerate()
                .map(|(index, part)| {
                    let prefix = if index == 0 { "" } else { "| " };
                    format!(
                        "{inner_padding}{prefix}{}",
                        wrap_annotation(part, indent + 4 + prefix.len(), indent + 4)
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            return format!("(\n{lines}\n{padding})");
        }
    }
    if source.ends_with(']')
        && let Some(open) = source.find('[')
    {
        let head = &source[..open];
        // Only generic type expressions; never split literal string data.
        if head
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '.')
        {
            let lines = annotation_parts(&source[open + 1..source.len() - 1], ',')
                .iter()
                .map(|part| {
                    format!(
                        "{inner_padding}{},",
                        wrap_annotation(part, indent + 4, indent + 4)
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            return format!("{head}[\n{lines}\n{padding}]");
        }
    }
    source.to_owned()
}

fn write_docstring(output: &mut String, description: &str) {
    let escaped = description
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\r', "\\r")
        .replace('\0', "\\x00");
    let lines: Vec<_> = escaped.lines().collect();
    if lines.len() == 1 {
        writeln!(output, "    \"\"\"{}\"\"\"\n", lines[0]).expect("string write");
    } else {
        output.push_str("    \"\"\"");
        for (index, line) in lines.iter().enumerate() {
            if index > 0 && !line.is_empty() {
                output.push_str("    ");
            }
            writeln!(output, "{line}").expect("string write");
        }
        output.push_str("    \"\"\"\n\n");
    }
}

pub(super) fn module(classes: &[ClassSpec], root: &str) -> Result<DataclassModule, DataclassError> {
    module_with_metadata(classes, root, &DataclassMetadata::default())
}

pub(super) fn module_with_metadata(
    classes: &[ClassSpec],
    root: &str,
    metadata: &DataclassMetadata,
) -> Result<DataclassModule, DataclassError> {
    render_module(classes, root, metadata, None)
}

pub(super) fn module_with_metadata_for_package(
    classes: &[ClassSpec],
    root: &str,
    metadata: &DataclassMetadata,
    document: &json_schema_ast::SchemaDocument,
) -> Result<DataclassModule, DataclassError> {
    render_module(classes, root, metadata, Some(document))
}

fn render_module(
    classes: &[ClassSpec],
    root: &str,
    metadata: &DataclassMetadata,
    package_document: Option<&json_schema_ast::SchemaDocument>,
) -> Result<DataclassModule, DataclassError> {
    let standalone = package_document.is_none();
    let timing = std::env::var_os("JSONCOMPAT_CODEGEN_TIMING").is_some();
    let mut stage_started = std::time::Instant::now();
    let mut finish_stage = |stage: &str| {
        if timing {
            eprintln!(
                "codegen model-group {root} ({}) {stage}: {:.3}s",
                classes.len(),
                stage_started.elapsed().as_secs_f64()
            );
        }
        stage_started = std::time::Instant::now();
    };
    let prepared = match package_document {
        Some(document) => PreparedModule::compile_for_package(classes, document),
        None => PreparedModule::compile(classes),
    }?;
    finish_stage("prepare");
    let mut parts = super::package::Parts::default();
    let mut imports = String::from(
        "# Generated by jsoncompat. Edit the schema and regenerate this module.\n# Constructors declare typed signatures; the native runtime constructs and validates slots.\nfrom __future__ import annotations\n\nimport collections.abc\nimport typing\n\nfrom jsoncompat.codegen import dataclasses as dc\n\n",
    );
    if prepared.graph.nodes.iter().any(|node| {
        matches!(
            node,
            ConversionNode::Scalar {
                kind: ScalarKind::Bytes
            }
        )
    }) {
        imports.push_str("import builtins\n\n");
    }
    let mut declarations = String::new();
    let has_bases = classes
        .iter()
        .any(|class| metadata.bases.contains_key(&class.name));
    if has_bases
        || classes.iter().any(|class| {
            metadata
                .fields
                .get(&class.name)
                .is_some_and(|fields| !fields.is_empty())
        })
    {
        imports.push_str(&metadata.imports);
    }
    if has_bases {
        imports.push_str(&metadata.base_imports);
    }
    let mut implementation = String::new();
    for class in classes {
        let fields = fields(class);
        if let Some(metadata) = metadata.fields.get(&class.name) {
            for name in metadata.keys() {
                if !fields.iter().any(|field| field.name == *name) {
                    return Err(super::invalid_schema(
                        class.name.clone(),
                        format!("HTTP annotation references missing field {name}"),
                    ));
                }
            }
        }
        let declaration_start = declarations.len();
        if let Some(prefix) = metadata.before.get(&class.name) {
            declarations.push_str(prefix);
        }
        let base = metadata
            .bases
            .get(&class.name)
            .cloned()
            .unwrap_or_else(|| render_class_base(class));
        if class.name.len() + base.len() + 9 > 92 {
            writeln!(
                declarations,
                "\n@typing.final\nclass {}(\n    {},\n):",
                class.name,
                wrap_annotation(&base, 4, 4)
            )
            .expect("string write");
        } else {
            writeln!(
                declarations,
                "\n@typing.final\nclass {}({base}):",
                class.name
            )
            .expect("string write");
        }
        if let Some(description) = metadata
            .docstrings
            .get(&class.name)
            .map(String::as_str)
            .or(class.description.as_deref())
            .filter(|description| !description.is_empty())
        {
            write_docstring(&mut declarations, description);
        }
        for field in &fields {
            if let Some(description) = &field.description {
                for line in description.lines() {
                    let safe: String = line
                        .chars()
                        .flat_map(|ch| {
                            if ch.is_control() {
                                ch.escape_default().collect::<Vec<_>>()
                            } else {
                                vec![ch]
                            }
                        })
                        .collect();
                    writeln!(declarations, "    # {safe}").expect("string write");
                }
            }
            let annotation = metadata
                .fields
                .get(&class.name)
                .and_then(|fields| fields.get(&field.name))
                .map(|extra| format!("typing.Annotated[{}, {extra}]", field.annotation))
                .unwrap_or_else(|| field.annotation.clone());
            let prefix = format!("    {}: ", field.name);
            let annotation = wrap_annotation(&annotation, prefix.len(), 4);
            writeln!(declarations, "{prefix}{annotation} = {}", field.expression)
                .expect("string write");
        }
        writeln!(
            declarations,
            "\n    if not typing.TYPE_CHECKING:\n        __slots__ = {}\n",
            tuple(fields.iter().map(|f| python_string_literal(&f.name)))
        )
        .expect("string write");
        declarations.push_str("    def __init__(\n        self,\n        *,\n        skip_validation: bool = False,\n");
        for field in &fields {
            let annotation = metadata
                .fields
                .get(&class.name)
                .and_then(|fields| fields.get(&field.name))
                .map(|extra| format!("typing.Annotated[{}, {extra}]", field.input_annotation))
                .unwrap_or_else(|| field.input_annotation.clone());
            let prefix = format!("        {}: ", field.name);
            let annotation = wrap_annotation(&annotation, prefix.len(), 8);
            writeln!(
                declarations,
                "{prefix}{annotation}{},",
                field
                    .default
                    .map(|default| format!(" = {default}"))
                    .unwrap_or_default()
            )
            .expect("string write");
        }
        declarations.push_str("    ) -> None:\n        self.__post_init__(skip_validation)\n\n");
        if let Some(suffix) = metadata.after.get(&class.name) {
            declarations.push_str(suffix);
        }
        let declaration = &declarations[declaration_start..];
        let mut class_imports = imports.clone();
        if !declaration.contains("collections.abc.") {
            class_imports = class_imports.replace("import collections.abc\n", "");
        }
        if !declaration.contains("dc.") {
            class_imports =
                class_imports.replace("from jsoncompat.codegen import dataclasses as dc\n", "");
        }
        if !metadata.bases.contains_key(&class.name) {
            class_imports = class_imports.replace(&metadata.base_imports, "");
            if !metadata
                .fields
                .get(&class.name)
                .is_some_and(|fields| !fields.is_empty())
            {
                class_imports = class_imports.replace(&metadata.imports, "");
            }
        }
        parts.extra_exports.push(
            metadata
                .extra_exports
                .get(&class.name)
                .cloned()
                .unwrap_or_default(),
        );
        parts.imports.push(class_imports);
        parts.references.push(super::package::references(class));
        parts.declarations.push(declaration.to_owned());
    }
    let mut order: Vec<usize> = (0..classes.len()).collect();
    let positions: BTreeMap<_, _> = metadata
        .declaration_order
        .iter()
        .enumerate()
        .map(|(index, name)| (name.as_str(), index))
        .collect();
    order.sort_by_key(|index| {
        positions
            .get(classes[*index].name.as_str())
            .copied()
            .unwrap_or(usize::MAX)
    });
    declarations = order
        .iter()
        .map(|index| parts.declarations[*index].as_str())
        .collect();
    parts.declaration_order = order;
    if has_bases && !declarations.contains("dc.") {
        imports = imports.replace("from jsoncompat.codegen import dataclasses as dc\n", "");
    }
    let footer_start = declarations.len();
    writeln!(declarations, "JSONCOMPAT_MODEL = {root}\n").expect("string write");
    writeln!(
        declarations,
        "__all__ = {}",
        tuple(
            classes
                .iter()
                .map(|c| python_string_literal(&c.name))
                .chain(std::iter::once(python_string_literal("JSONCOMPAT_MODEL")))
        )
    )
    .expect("string write");
    parts.footer = declarations[footer_start..].to_owned();
    implementation
        .push_str("def _jsoncompat_bind(_models: tuple[type[dc.DataclassModel], ...]) -> None:\n");
    writeln!(
        implementation,
        "    {} = _models",
        tuple(classes.iter().map(|c| c.name.clone()))
    )
    .expect("string write");
    finish_stage("declarations");
    let mut pattern_pool = crate::prepared_schema::PatternPool::default();
    let programs = classes
        .iter()
        .zip(&prepared.graph.schemas)
        .map(|(class, schema)| {
            pattern_pool
                .program(schema)
                .map(|(program_bytes, patterns)| {
                    let literal = if standalone {
                        program(&bytes(&program_bytes), &patterns)
                    } else {
                        String::new()
                    };
                    parts.programs.push(program_bytes);
                    parts.program_patterns.push(patterns);
                    literal
                })
                .map_err(|error| super::invalid_schema(class.name.clone(), error))
        })
        .collect::<Result<Vec<_>, _>>()?;
    parts.patterns = pattern_pool.data;
    if standalone && !parts.patterns.is_empty() {
        let shared = patterns(&parts.patterns)
            .replace("from jsoncompat.codegen import dataclasses as dc\n", "");
        // Bind-time local constants disappear after model installation; native
        // graphs retain shared automata through reference-counted handles.
        let shared = shared
            .lines()
            .map(|line| format!("    {line}\n"))
            .collect::<String>();
        implementation.push_str(&shared);
    }
    finish_stage("program serialization");
    let roots = classes
        .iter()
        .zip(&prepared.roots)
        .map(|(class, id)| format!("({}, {})", class.name, id.0))
        .collect::<Vec<_>>()
        .join(", ");
    let models = classes.iter().map(|c| c.name.clone()).collect::<Vec<_>>();
    let descriptors = if standalone {
        prepared
            .graph
            .nodes
            .iter()
            .map(|node| descriptor(node, &models, &programs, |id| id.0))
            .collect::<Vec<_>>()
            .join(",\n        ")
    } else {
        String::new()
    };
    let model_references = (0..classes.len())
        .map(|i| format!("_models[{i}]"))
        .collect::<Vec<_>>();
    let program_references = (0..classes.len())
        .map(|i| format!("_programs[{i}]"))
        .collect::<Vec<_>>();
    parts.descriptors = prepared
        .graph
        .nodes
        .iter()
        .map(|node| descriptor(node, &model_references, &program_references, |id| id.0))
        .collect();
    parts.roots = prepared.roots.iter().map(|id| id.0).collect();
    let plan = serde_json::to_vec(&prepared.decisions).expect("serializable model plan");
    if standalone {
        writeln!(
            implementation,
            "    dc.bind_module({GENERATED_FORMAT_VERSION}, [{roots}], [\n        {descriptors}\n    ], {})\n",
            bytes(&plan)
        )
        .expect("string write");
    } else {
        implementation.clear();
    }
    if !classes
        .iter()
        .flat_map(fields)
        .any(|field| field.annotation.contains("collections.abc."))
    {
        imports = imports.replace("import collections.abc\n", "");
    }
    parts.plan = plan;
    parts.prepared = Some(prepared);
    parts.pointers = classes.iter().map(|c| c.schema.pointer.clone()).collect();
    finish_stage("conversion descriptors");
    Ok(DataclassModule {
        parts,
        imports,
        declarations,
        implementation,
        models: classes.iter().map(|c| c.name.clone()).collect(),
    })
}

pub(super) fn descriptor(
    node: &ConversionNode,
    models: &[String],
    programs: &[String],
    index: impl Fn(super::plan::NodeId) -> usize,
) -> String {
    match node {
        ConversionNode::Scalar { kind } => format!(
            "({:?},)",
            match kind {
                ScalarKind::Any => "any",
                ScalarKind::String => "str",
                ScalarKind::Bytes => "bytes",
                ScalarKind::Integer => "int",
                ScalarKind::Number => "float",
                ScalarKind::Boolean => "bool",
                ScalarKind::Null => "null",
            }
        ),
        ConversionNode::Literal { values } => format!(
            "('literal', {})",
            tuple(values.iter().map(python_json_literal))
        ),
        ConversionNode::List { item } => format!("('list', {})", index(*item)),
        ConversionNode::Tuple { items } => format!(
            "('tuple', {})",
            tuple(items.iter().map(|item| index(*item).to_string()))
        ),
        ConversionNode::Dict { value } => format!("('dict', {})", index(*value)),
        ConversionNode::Root { model, value } => {
            format!(
                "('root', {}, {}, {})",
                models[*model],
                index(*value),
                programs[*model]
            )
        }
        ConversionNode::Model {
            model,
            fields,
            extra,
        } => format!(
            "('model', {}, {}, {}, {})",
            models[*model],
            tuple(fields.iter().map(|f| format!(
                "({}, {}, {}, {})",
                python_string_literal(&f.json_name),
                python_string_literal(&f.py_name),
                index(f.value_node),
                if f.presence.is_omittable() {
                    "True"
                } else {
                    "False"
                }
            ))),
            extra
                .as_ref()
                .map(|e| index(e.value_node).to_string())
                .unwrap_or_else(|| "None".into()),
            programs[*model]
        ),
        ConversionNode::Union(union) => {
            let (name, entries) = union
                .discriminator
                .as_ref()
                .map(|(name, entries)| {
                    (
                        python_string_literal(name),
                        tuple(entries.iter().map(|(value, index)| {
                            format!("({}, {index})", python_json_literal(value))
                        })),
                    )
                })
                .unwrap_or_else(|| ("None".into(), "None".into()));
            format!(
                "('union', {}, {name}, {entries})",
                tuple(union.branches.iter().map(|id| index(*id).to_string()))
            )
        }
    }
}

/// Local shared handles for a standalone generated module.
pub(super) fn patterns(patterns: &BTreeMap<String, Vec<u8>>) -> String {
    let mut source = String::from(
        "# Shared precompiled regular expressions. Do not edit.\nfrom jsoncompat.codegen import dataclasses as dc\n",
    );
    for (id, data) in patterns {
        writeln!(
            source,
            "_jsoncompat_pattern_{id} = dc.load_pattern({})",
            bytes(data)
        )
        .unwrap();
    }
    source
}

pub(super) fn program(bytes: &str, patterns: &[String]) -> String {
    if patterns.is_empty() {
        return bytes.to_owned();
    }
    format!(
        "({bytes}, ({},))",
        patterns
            .iter()
            .map(|id| format!("_jsoncompat_pattern_{id}"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(super) fn pattern_imports(ids: impl IntoIterator<Item = String>, prefix: &str) -> String {
    let mut imports = String::new();
    for id in ids {
        writeln!(
            imports,
            "from {prefix}_patterns.p_{id} import PATTERN as _jsoncompat_pattern_{id}"
        )
        .unwrap();
    }
    imports
}

pub(super) fn pattern_file(data: &[u8]) -> String {
    format!(
        "# Shared precompiled regular expression. Do not edit.\nfrom jsoncompat.codegen import dataclasses as dc\n\nPATTERN = dc.load_pattern({})\n",
        bytes(data)
    )
}
