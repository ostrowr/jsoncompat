//! Physical source shards for a logical model group. Recursive groups still bind
//! together; moving their declarations and prepared bytes must not duplicate types.
use super::{
    ClassKind, ClassSpec, DataclassModule,
    types::{Annotation, Kind},
};
use crate::python_names::module_names;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
    sync::Arc,
};

/// Generated source files for one logical package model group.
/// File paths are relative to the public package, including shared schema data.
pub struct DataclassPackage {
    pub models: Vec<String>,
    pub files: BTreeMap<String, String>,
}

#[derive(Default)]
pub(super) struct Parts {
    pub imports: Vec<String>,
    pub references: Vec<BTreeSet<String>>,
    pub declarations: Vec<String>,
    pub initializers: Vec<String>,
    pub footer: String,
    pub sources: Vec<SchemaSource>,
    pub programs: Vec<Vec<u8>>,
    pub roots: Vec<usize>,
    pub descriptors: Vec<String>,
    pub plan: Vec<u8>,
}

pub(super) enum SchemaSource {
    Whole(Vec<u8>),
    Fragments(Vec<SchemaFragment>),
}

pub(super) enum SchemaFragment {
    Local(Vec<u8>),
    Shared(Arc<Vec<u8>>),
}

const PRIVATE_IMPORTS: &str = "# Generated implementation. Import the public model module instead.\nfrom __future__ import annotations\nimport typing\nfrom jsoncompat.codegen import dataclasses as dc\n\n";
// Prepared bytes can dwarf declarations, even for a single recursive model.
// Bound the prepared bytes stored alongside each initializer; overflow lives in
// data-only modules. These constants never decide whether neighboring models or
// operations share a source file.
const DATA_CHUNK_BYTES: usize = 64 * 1024;
const NODES_PER_MODULE: usize = 256;

impl DataclassModule {
    /// Render a package group with one combined declaration/initializer module per
    /// model. The layout is unconditional and named by model, not source size.
    /// Prepared data uses bounded byte-constant chunks, without runtime source
    /// compilation or resource files that require packaging configuration.
    pub fn package_files(&self, module: &str) -> BTreeMap<String, String> {
        let mut files = Files {
            prefix: module.replace('.', "/"),
            ..Default::default()
        };
        let filenames = module_names(self.models.iter().map(String::as_str));
        // These imports supply the namespace used by get_type_hints after the
        // declarations move to separate modules. They are intentionally retained.
        let mut public = format!(
            "# Imports below also resolve forward annotations.\n# pyright: reportUnusedImport=false\n{}",
            package_imports(&self.imports)
        );
        let mut imports = String::new();
        let mut programs = Vec::new();
        for (i, name) in self.models.iter().enumerate() {
            let stem = &filenames[name];
            let declaration_module = stem;
            // Forward annotations resolve against the public facade. The same
            // static imports make those names visible to type checkers.
            let references = &self.parts.references[i];
            let type_imports = if references.is_empty() {
                String::new()
            } else {
                format!(
                    "if typing.TYPE_CHECKING:\n{}",
                    references
                        .iter()
                        .map(|name| format!("    from .{} import {name}\n", filenames[name]))
                        .collect::<String>()
                )
            };
            let mut model_imports = package_imports(&self.parts.imports[i]);
            let runtime_import = "from jsoncompat.codegen import dataclasses as dc\n";
            // Empty requests can inherit only the HTTP base and therefore need
            // no runtime import in their declaration. Their initializer does.
            if !model_imports.contains(runtime_import) {
                model_imports.push_str(runtime_import);
            }
            model_imports.push_str("from builtins import bytes as _jsoncompat_bytes\n");
            let declaration = format!(
                "{}{type_imports}{}\n__all__ = ({name:?},)\n",
                model_imports, self.parts.declarations[i]
            );
            writeln!(
                public,
                "from .{declaration_module} import {name} as {name}\n{name}.__module__ = __name__"
            )
            .unwrap();

            let schema = files.schema(&self.parts.sources[i]);
            let program = files.blob(
                &format!("_program_{stem}"),
                &self.parts.programs[i],
                "_jsoncompat_program",
            );
            let initializer = self.parts.initializers[i]
                .replace(&format!("_jsoncompat_init_{i}("), "_jsoncompat_init(");
            files.add(declaration_module, format!("{declaration}\n# Prepared implementation; regenerated with the declaration above.\n{schema}{program}\n_jsoncompat_missing = dc.MissingType.MISSING\n\n{initializer}\ndef _jsoncompat_install(model: type[dc.DataclassModel], namespace: dict[str, typing.Any]) -> _jsoncompat_bytes:\n    dc.install_model(model, _jsoncompat_init, _jsoncompat_schema, namespace)\n    return _jsoncompat_program\n"));
            writeln!(
                imports,
                "from .{declaration_module} import _jsoncompat_install as _install_{name}"
            )
            .unwrap();
            programs.push(format!("_install_{name}(_models[{i}], _namespace)"));
        }
        // Large literal tuples exceed type-checker inference limits.
        public.push_str(
            &self
                .parts
                .footer
                .replace("__all__ =", "__all__: tuple[str, ...] ="),
        );
        writeln!(public, "from ._binding import bind_models as _jsoncompat_bind\n_jsoncompat_bind(({},), globals())\ndel _jsoncompat_bind", self.models.join(", ")).unwrap();
        files.add("__init__", public);

        let mut calls = String::new();
        for (i, nodes) in self.parts.descriptors.chunks(NODES_PER_MODULE).enumerate() {
            let shard = format!("_nodes_{i:04}");
            let descriptors = nodes
                .iter()
                .map(|s| format!("        {s},\n"))
                .collect::<String>();
            files.add(&shard, format!("{PRIVATE_IMPORTS}def descriptors(_models: tuple[type[dc.DataclassModel], ...], _programs: list[bytes], _descriptors: list[tuple[typing.Any, ...]]) -> None:\n    _descriptors.extend([\n{descriptors}    ])\n"));
            writeln!(imports, "from .{shard} import descriptors as _nodes_{i}").unwrap();
            writeln!(calls, "    _nodes_{i}(_models, _programs, _descriptors)").unwrap();
        }
        imports.push_str(&files.blob("_plan", &self.parts.plan, "_plan"));
        let roots = self
            .parts
            .roots
            .iter()
            .enumerate()
            .map(|(i, node)| format!("(_models[{i}], {node})"))
            .collect::<Vec<_>>()
            .join(", ");
        files.add("_binding", format!("{PRIVATE_IMPORTS}{imports}\n__all__ = (\"bind_models\",)\n\ndef bind_models(_models: tuple[type[dc.DataclassModel], ...], _namespace: dict[str, typing.Any]) -> None:\n    _programs = [{}]\n    _descriptors: list[tuple[typing.Any, ...]] = []\n{calls}    dc.bind_module(1, [{roots}], _descriptors, _plan)\n", programs.join(", ")));
        files.sources
    }
}

#[derive(Default)]
struct Files {
    prefix: String,
    sources: BTreeMap<String, String>,
    schema_chunks: BTreeMap<usize, Vec<String>>,
}
impl Files {
    fn add(&mut self, module: &str, source: String) {
        assert!(
            self.sources
                .insert(format!("{}/{module}.py", self.prefix), source)
                .is_none()
        );
    }
    fn blob(&mut self, module: &str, bytes: &[u8], alias: &str) -> String {
        let mut imports = String::new();
        let mut parts = Vec::new();
        for (i, chunk) in bytes.chunks(DATA_CHUNK_BYTES).enumerate() {
            if i == 0 {
                // Most programs fit beside their existing initializer. Avoid a
                // separate file and import just to hold one small constant.
                writeln!(imports, "{alias}_{i} = {}", super::render::bytes(chunk)).unwrap();
                parts.push(format!("{alias}_{i}"));
                continue;
            }
            let shard = format!("{module}_{i:04}");
            self.add(
                &shard,
                format!(
                    "# Generated prepared data.\nDATA = {}\n",
                    super::render::bytes(chunk)
                ),
            );
            writeln!(imports, "from .{shard} import DATA as {alias}_{i}").unwrap();
            parts.push(format!("{alias}_{i}"));
        }
        writeln!(imports, "{alias} = b''.join([{}])", parts.join(", ")).unwrap();
        imports
    }

    fn schema(&mut self, source: &SchemaSource) -> String {
        enum Chunk<'a> {
            Local(&'a [u8]),
            Shared(String),
        }
        let imported =
            |chunks: Vec<String>| chunks.into_iter().map(Chunk::Shared).collect::<Vec<_>>();
        let mut inline_budget = DATA_CHUNK_BYTES;
        let streams = match source {
            // Identical complete schemas can be shared by independent model
            // groups, whose lazy imports must retain the same bytes objects.
            SchemaSource::Whole(bytes) => vec![imported(self.schema_stream(bytes))],
            SchemaSource::Fragments(fragments) => fragments
                .iter()
                .map(|fragment| match fragment {
                    SchemaFragment::Local(bytes) if bytes.len() <= inline_budget => {
                        inline_budget -= bytes.len();
                        vec![Chunk::Local(bytes)]
                    }
                    SchemaFragment::Local(bytes) => imported(self.schema_stream(bytes)),
                    SchemaFragment::Shared(bytes) => {
                        let identity = Arc::as_ptr(bytes) as usize;
                        let chunks = if let Some(chunks) = self.schema_chunks.get(&identity) {
                            chunks.clone()
                        } else {
                            let chunks = self.schema_stream(bytes);
                            self.schema_chunks.insert(identity, chunks.clone());
                            chunks
                        };
                        imported(chunks)
                    }
                })
                .collect(),
        };
        let mut imports = String::new();
        let mut streams_literals = Vec::new();
        let mut next_alias = 0;
        for chunks in streams {
            let mut parts = Vec::new();
            for chunk in chunks {
                let alias = next_alias;
                next_alias += 1;
                match chunk {
                    Chunk::Shared(name) => writeln!(
                        imports,
                        "from ..._schemas.{name} import DATA as _jsoncompat_schema_{alias}"
                    ),
                    Chunk::Local(bytes) => {
                        writeln!(
                            imports,
                            "_jsoncompat_schema_{alias} = {}",
                            super::render::bytes(bytes)
                        )
                    }
                }
                .unwrap();
                parts.push(format!("_jsoncompat_schema_{alias},"));
            }
            streams_literals.push(format!("({})", parts.join(" ")));
        }
        match source {
            SchemaSource::Whole(_) => writeln!(
                imports,
                "_jsoncompat_schema: tuple[_jsoncompat_bytes, ...] = {}",
                streams_literals[0]
            ),
            SchemaSource::Fragments(_) => writeln!(
                imports,
                "_jsoncompat_schema = dc.SchemaSource(({},))",
                streams_literals.join(", ")
            ),
        }
        .unwrap();
        imports
    }

    fn schema_stream(&mut self, bytes: &[u8]) -> Vec<String> {
        bytes
            .chunks(DATA_CHUNK_BYTES)
            .map(|chunk| {
                // Shared fragments and identical complete schemas are content
                // addressed across independent binding groups as well as models.
                let name = format!("_schema_{:x}", Sha256::digest(chunk));
                self.sources
                    .entry(format!("_schemas/{name}.py"))
                    .or_insert_with(|| {
                        format!(
                            "# Generated shared schema data.\nDATA: bytes = {}\n",
                            super::render::bytes(chunk)
                        )
                    });
                name
            })
            .collect()
    }
}

// Declarations live two packages below the public API.
fn package_imports(imports: &str) -> String {
    imports.replace("from .", "from ...")
}

pub(super) fn references(class: &ClassSpec) -> BTreeSet<String> {
    fn visit(annotation: &Annotation, names: &mut BTreeSet<String>) {
        match &annotation.kind {
            Kind::Model(name) => {
                names.insert(name.clone());
            }
            Kind::Sequence(item) | Kind::Mapping(item) => visit(item, names),
            Kind::Union(branches) => {
                for branch in branches {
                    visit(branch, names);
                }
            }
            _ => {}
        }
    }
    let mut names = BTreeSet::new();
    match &class.kind {
        ClassKind::Root { annotation } => visit(annotation, &mut names),
        ClassKind::Object {
            fields,
            extra_annotation,
        } => {
            for field in fields {
                visit(&field.annotation, &mut names);
            }
            if let Some(extra) = extra_annotation {
                visit(extra, &mut names);
            }
        }
    }
    names.remove(&class.name);
    names
}
