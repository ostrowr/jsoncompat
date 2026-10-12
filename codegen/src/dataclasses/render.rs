//! Public declarations first; precomputed implementation in a separable footer.

use super::plan::{ConversionNode, PreparedModule, ScalarKind};
use super::{
    ClassKind, ClassSchema, ClassSpec, DataclassError, EXTRA_FIELD_NAME, omittable_annotation,
    python_json_literal, python_string_literal, render_class_base,
};
use std::collections::BTreeMap;
use std::fmt::Write;
use std::sync::Arc;

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
            "{}{}\n# --- Generated implementation: regenerate this file to change it. ---\n\n{}\n_jsoncompat_bind({}, globals())\n",
            self.imports,
            self.declarations,
            self.implementation,
            self.model_tuple()
        )
    }
    pub fn public_file(&self, companion: &str) -> String {
        format!(
            "{}if not typing.TYPE_CHECKING:\n    if __package__ not in (None, \"\"):\n        from .{companion} import bind_models as _jsoncompat_bind\n    else:\n        from {companion} import bind_models as _jsoncompat_bind\n\n{}\nif not typing.TYPE_CHECKING:\n    _jsoncompat_bind({}, globals())\n    del _jsoncompat_bind\n",
            self.imports,
            self.declarations,
            self.model_tuple()
        )
    }
    pub fn private_file(&self) -> String {
        format!(
            "# Generated implementation. Import the public model module instead.\nfrom __future__ import annotations\n\nimport typing\nfrom jsoncompat.codegen import dataclasses as dc\n\n__all__ = (\"bind_models\",)\n\n{}\nbind_models = _jsoncompat_bind\n",
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
    expression: String,
    default: Option<&'static str>,
}
fn fields(class: &ClassSpec) -> Vec<Field> {
    match &class.kind {
        ClassKind::Root { annotation } => vec![Field {
            name: "root".into(),
            annotation: annotation.to_string(),
            expression: "dc.root_field()".into(),
            default: None,
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
                    expression: format!(
                        "dc.field({}{})",
                        python_string_literal(&field.json_name),
                        if field.required {
                            ""
                        } else {
                            ", default=dc.MISSING"
                        }
                    ),
                    default: if field.required {
                        None
                    } else {
                        Some("_jsoncompat_missing")
                    },
                })
                .collect::<Vec<_>>();
            if let Some(annotation) = extra_annotation {
                result.push(Field {
                    name: EXTRA_FIELD_NAME.into(),
                    annotation: format!("collections.abc.Mapping[str, {annotation}]"),
                    expression: "dc.extra_field(default_factory=dict)".into(),
                    default: Some("dc.EXTRA_DEFAULT"),
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
    let mut out = String::from("b'");
    for byte in bytes {
        match byte {
            b'\'' => out.push_str("\\'"),
            b'\\' => out.push_str("\\\\"),
            32..=126 => out.push(*byte as char),
            _ => write!(out, "\\x{byte:02x}").expect("string write"),
        }
    }
    out.push('\'');
    out
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
        "# Generated by jsoncompat. Edit the schema and regenerate this module.\nfrom __future__ import annotations\n\nimport collections.abc\nimport typing\n\nfrom jsoncompat.codegen import dataclasses as dc\n\n",
    );
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
    let mut implementation = if classes
        .iter()
        .flat_map(fields)
        .any(|field| field.default == Some("_jsoncompat_missing"))
    {
        String::from("_jsoncompat_missing = dc.MissingType.MISSING\n\n")
    } else {
        String::new()
    };
    for (index, class) in classes.iter().enumerate() {
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
        let initializer_start = implementation.len();
        writeln!(
            declarations,
            "\n@typing.final\nclass {}({}):",
            class.name,
            metadata
                .bases
                .get(&class.name)
                .cloned()
                .unwrap_or_else(|| render_class_base(class))
        )
        .expect("string write");
        for field in &fields {
            let annotation = metadata
                .fields
                .get(&class.name)
                .and_then(|fields| fields.get(&field.name))
                .map(|extra| format!("typing.Annotated[{}, {extra}]", field.annotation))
                .unwrap_or_else(|| field.annotation.clone());
            writeln!(
                declarations,
                "    {}: {} = {}",
                field.name, annotation, field.expression
            )
            .expect("string write");
        }
        writeln!(
            declarations,
            "\n    if not typing.TYPE_CHECKING:\n        __slots__ = {}\n",
            tuple(fields.iter().map(|f| python_string_literal(&f.name)))
        )
        .expect("string write");
        let mut parameters = vec![
            "self: dc.DataclassModel".into(),
            "*".into(),
            "skip_validation: bool = False".into(),
        ];
        parameters.extend(fields.iter().map(|field| {
            format!(
                "{}: {}{}",
                field.name,
                "typing.Any",
                field.default.map(|d| format!(" = {d}")).unwrap_or_default()
            )
        }));
        writeln!(
            implementation,
            "def _jsoncompat_init_{index}({}) -> None:\n    self.__post_init__(skip_validation)\n",
            parameters.join(", ")
        )
        .expect("string write");
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
        parts.imports.push(class_imports);
        parts.references.push(super::package::references(class));
        parts.declarations.push(declaration.to_owned());
        parts
            .initializers
            .push(implementation[initializer_start..].to_owned());
    }
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
        .push_str("def _jsoncompat_bind(_models: tuple[type[dc.DataclassModel], ...], _namespace: dict[str, typing.Any]) -> None:\n");
    writeln!(
        implementation,
        "    {} = _models",
        tuple(classes.iter().map(|c| c.name.clone()))
    )
    .expect("string write");
    finish_stage("declarations");
    let programs = classes
        .iter()
        .zip(&prepared.graph.schemas)
        .map(|(class, schema)| {
            schema
                .to_bytes()
                .map(|program| {
                    let literal = if standalone {
                        bytes(&program)
                    } else {
                        String::new()
                    };
                    parts.programs.push(program);
                    literal
                })
                .map_err(|error| super::invalid_schema(class.name.clone(), error))
        })
        .collect::<Result<Vec<_>, _>>()?;
    finish_stage("program serialization");
    let mut source_renderer = SourceRenderer::default();
    for (index, class) in classes.iter().enumerate() {
        // Source introspection is independent of the native validation program.
        // Package sources share independently compressed definition blocks, so
        // a large resource graph is serialized and compressed only once.
        let source = if standalone {
            let source = compress(class.schema.pretty_json()?.as_bytes());
            writeln!(
                implementation,
                "    dc.install_model({}, _jsoncompat_init_{index}, {}, _namespace)",
                class.name,
                bytes(&source)
            )
            .expect("string write");
            super::package::SchemaSource::Whole(source)
        } else {
            source_renderer.render(&class.schema)?
        };
        parts.sources.push(source);
    }
    finish_stage("schema sources");
    let roots = classes
        .iter()
        .zip(&prepared.roots)
        .map(|(class, id)| format!("({}, {})", class.name, id.0))
        .collect::<Vec<_>>()
        .join(", ");
    let render_descriptor =
        |node: &ConversionNode, models: &[String], programs: &[String]| match node {
            ConversionNode::Scalar { kind } => format!(
                "({:?},)",
                match kind {
                    ScalarKind::Any => "any",
                    ScalarKind::String => "str",
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
            ConversionNode::List { item } => format!("('list', {})", item.0),
            ConversionNode::Dict { value } => format!("('dict', {})", value.0),
            ConversionNode::Root { model, value } => {
                format!(
                    "('root', {}, {}, {})",
                    models[*model], value.0, programs[*model]
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
                    f.value_node.0,
                    if f.presence.is_omittable() {
                        "True"
                    } else {
                        "False"
                    }
                ))),
                extra
                    .as_ref()
                    .map(|e| e.value_node.0.to_string())
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
                    tuple(union.branches.iter().map(|id| id.0.to_string()))
                )
            }
        };
    let models = classes.iter().map(|c| c.name.clone()).collect::<Vec<_>>();
    let descriptors = if standalone {
        prepared
            .graph
            .nodes
            .iter()
            .map(|node| render_descriptor(node, &models, &programs))
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
        .map(|node| render_descriptor(node, &model_references, &program_references))
        .collect();
    parts.roots = prepared.roots.iter().map(|id| id.0).collect();
    let plan = serde_json::to_vec(&prepared.decisions).expect("serializable model plan");
    if standalone {
        writeln!(
            implementation,
            "    dc.bind_module(1, [{roots}], [\n        {descriptors}\n    ], {})\n",
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
    finish_stage("conversion descriptors");
    Ok(DataclassModule {
        parts,
        imports,
        declarations,
        implementation,
        models: classes.iter().map(|c| c.name.clone()).collect(),
    })
}

fn compress(source: &[u8]) -> Vec<u8> {
    miniz_oxide::deflate::compress_to_vec_zlib(source, 9)
}

/// Retain byte-for-byte schema introspection while sharing the expensive root
/// definition blocks. Each fragment is an independent compressed stream; only
/// explicit introspection expands and concatenates them.
#[derive(Default)]
struct SourceRenderer {
    definitions: BTreeMap<(usize, &'static str), Arc<Vec<u8>>>,
}
impl SourceRenderer {
    fn render(
        &mut self,
        schema: &ClassSchema,
    ) -> Result<super::package::SchemaSource, DataclassError> {
        let (Some(definitions), Some(object)) = (&schema.definitions, schema.value.as_object())
        else {
            return Ok(super::package::SchemaSource::Whole(compress(
                schema.pretty_json()?.as_bytes(),
            )));
        };
        let shared = [
            ("$defs", definitions.defs()),
            ("definitions", definitions.legacy_defs()),
        ];
        let mut keys = object
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        for (keyword, definitions) in shared {
            if definitions.is_some_and(|definitions| !definitions.is_empty()) {
                keys.insert(keyword);
            }
        }
        let mut fragments = Vec::new();
        let mut local = String::from("{\n");
        for (index, key) in keys.into_iter().enumerate() {
            if index > 0 {
                local.push_str(",\n");
            }
            let root = shared
                .iter()
                .find_map(|(keyword, root)| (*keyword == key).then_some((*keyword, *root)))
                .and_then(|(keyword, root)| {
                    root.filter(|root| !root.is_empty())
                        .map(|root| (keyword, root))
                });
            if let Some((keyword, root)) = root {
                // Root definitions have always overridden local definitions.
                // Extra local definitions are unusual, but retain them exactly.
                let extra_local = object
                    .get(key)
                    .and_then(serde_json::Value::as_object)
                    .is_some_and(|local| local.keys().any(|key| !root.contains_key(key)));
                if extra_local {
                    let mut merged = object
                        .get(key)
                        .and_then(serde_json::Value::as_object)
                        .cloned()
                        .unwrap_or_default();
                    merged.extend(root.iter().map(|(key, value)| (key.clone(), value.clone())));
                    local.push_str(&source_member(key, &merged)?);
                } else {
                    if !local.is_empty() {
                        fragments.push(super::package::SchemaFragment::Local(compress(
                            local.as_bytes(),
                        )));
                        local.clear();
                    }
                    let identity = (Arc::as_ptr(definitions) as usize, keyword);
                    let block = if let Some(block) = self.definitions.get(&identity) {
                        Arc::clone(block)
                    } else {
                        let block = Arc::new(compress(source_member(key, root)?.as_bytes()));
                        self.definitions.insert(identity, Arc::clone(&block));
                        block
                    };
                    fragments.push(super::package::SchemaFragment::Shared(block));
                }
            } else {
                local.push_str(&source_member(key, &object[key])?);
            }
        }
        local.push_str("\n}");
        fragments.push(super::package::SchemaFragment::Local(compress(
            local.as_bytes(),
        )));
        Ok(super::package::SchemaSource::Fragments(fragments))
    }
}

fn source_member(value_key: &str, value: &impl serde::Serialize) -> Result<String, DataclassError> {
    let key = serde_json::to_string(value_key).expect("serializable JSON key");
    let value = serde_json::to_string_pretty(value).map_err(|error| {
        super::invalid_schema("#".into(), format!("schema is not JSON: {error}"))
    })?;
    Ok(format!("  {key}: {}", value.replace('\n', "\n  ")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataclasses::{
        RootDefinitions,
        package::{SchemaFragment, SchemaSource},
    };
    use serde_json::json;

    fn expand(source: &SchemaSource) -> String {
        let streams: Vec<&[u8]> = match source {
            SchemaSource::Whole(bytes) => vec![bytes],
            SchemaSource::Fragments(parts) => parts
                .iter()
                .map(|part| match part {
                    SchemaFragment::Local(bytes) => bytes.as_slice(),
                    SchemaFragment::Shared(bytes) => bytes.as_slice(),
                })
                .collect(),
        };
        streams
            .into_iter()
            .map(|stream| {
                String::from_utf8(miniz_oxide::inflate::decompress_to_vec_zlib(stream).unwrap())
                    .unwrap()
            })
            .collect()
    }

    #[test]
    fn shared_schema_fragments_preserve_exact_pretty_source_and_root_precedence() {
        let root = Arc::new(RootDefinitions {
            root: json!({
                "$defs": {"Shared": {"type": "string", "enum": ["雪", "quoted\"\\value", "line\nbreak"]}},
                "definitions": {"Legacy": {"type": "integer", "minimum": -2}}
            }),
        });
        let mut renderer = SourceRenderer::default();
        for value in [
            json!({"$ref": "#/$defs/Shared", "description": "simple"}),
            json!({"$defs": {"Shared": false}, "$ref": "#/$defs/Shared"}),
            json!({"$defs": {"Local": {"type":"boolean"}, "Shared": false}, "$ref": "#/$defs/Local"}),
            json!({"definitions": {"Legacy": false, "Extra": {"const": null}}, "$ref": "#/definitions/Extra"}),
            json!({"type": "object", "properties": {"a": {"$ref": "#/$defs/Shared"}}, "required": ["a"]}),
        ] {
            let schema = ClassSchema {
                value,
                definitions: Some(Arc::clone(&root)),
                context: None,
                pointer: "#".into(),
            };
            assert_eq!(
                expand(&renderer.render(&schema).unwrap()),
                schema.pretty_json().unwrap()
            );
        }
        assert_eq!(renderer.definitions.len(), 2);
    }

    #[test]
    fn shared_schema_fragments_reuse_compressed_definition_objects() {
        let root = Arc::new(RootDefinitions {
            root: json!({"$defs": {"Shared": {"type": "string"}}}),
        });
        let mut renderer = SourceRenderer::default();
        let sources = ["first", "second"].map(|description| {
            renderer
                .render(&ClassSchema {
                    value: json!({"$ref": "#/$defs/Shared", "description": description}),
                    definitions: Some(Arc::clone(&root)),
                    context: None,
                    pointer: "#".into(),
                })
                .unwrap()
        });
        let [SchemaSource::Fragments(a), SchemaSource::Fragments(b)] = sources else {
            panic!("expected shared schema fragments");
        };
        let (SchemaFragment::Shared(a_shared), SchemaFragment::Shared(b_shared)) = (&a[1], &b[1])
        else {
            panic!("expected shared definition streams");
        };
        assert!(Arc::ptr_eq(a_shared, b_shared));
        let (SchemaFragment::Local(a_local), SchemaFragment::Local(b_local)) =
            (a.last().unwrap(), b.last().unwrap())
        else {
            panic!("expected local schema suffixes");
        };
        assert_ne!(a_local, b_local);
    }

    #[test]
    fn unshared_schema_sources_keep_legacy_stream_layout() {
        let mut renderer = SourceRenderer::default();
        for value in [
            json!(true),
            json!(false),
            json!({}),
            json!({"type": "array", "items": {"type": "string"}}),
        ] {
            let schema = ClassSchema::standalone(value);
            let source = renderer.render(&schema).unwrap();
            assert!(matches!(source, SchemaSource::Whole(_)));
            assert_eq!(expand(&source), schema.pretty_json().unwrap());
        }
    }
}
