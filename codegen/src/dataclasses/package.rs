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
};

#[derive(Default)]
pub(super) struct Parts {
    pub imports: Vec<String>,
    pub references: Vec<BTreeSet<String>>,
    pub declarations: Vec<String>,
    pub initializers: Vec<String>,
    pub footer: String,
    pub sources: Vec<Vec<u8>>,
    pub programs: Vec<Vec<u8>>,
    pub roots: Vec<usize>,
    pub descriptors: Vec<String>,
    pub plan: Vec<u8>,
}

const PRIVATE_IMPORTS: &str = "# Generated implementation. Import the public model module instead.\nfrom __future__ import annotations\nimport typing\nfrom jsoncompat.codegen import dataclasses as dc\n\n";
// Prepared bytes can dwarf declarations, even for a single recursive model.
// Keep their storage separate from semantic module boundaries. These constants
// never decide whether neighboring models or operations share a source file.
const DATA_CHUNK_BYTES: usize = 64 * 1024;
const NODES_PER_MODULE: usize = 256;

impl DataclassModule {
    /// Render a package group with one declaration and initializer module per
    /// model. The layout is unconditional and named by model, not source size.
    /// Prepared data uses separate byte-constant chunks, without runtime source
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
            files.add(
                declaration_module,
                format!(
                    "{}{type_imports}{}\n__all__ = ({name:?},)\n",
                    package_imports(&self.parts.imports[i]),
                    self.parts.declarations[i]
                ),
            );
            writeln!(
                public,
                "from .{declaration_module} import {name} as {name}\n{name}.__module__ = __name__"
            )
            .unwrap();

            let initializer_module = format!("_init_{stem}");
            let schema = files.schema(&self.parts.sources[i]);
            let program = files.blob(
                &format!("_program_{stem}"),
                &self.parts.programs[i],
                "_program",
            );
            let initializer = self.parts.initializers[i]
                .replace(&format!("_jsoncompat_init_{i}("), "_jsoncompat_init(");
            files.add(&initializer_module, format!("{PRIVATE_IMPORTS}{schema}{program}\n_jsoncompat_missing = dc.MissingType.MISSING\n\n{initializer}\ndef install(model: type[dc.DataclassModel], namespace: dict[str, typing.Any]) -> bytes:\n    dc.install_model(model, _jsoncompat_init, _schema, namespace)\n    return _program\n"));
            writeln!(
                imports,
                "from .{initializer_module} import install as _install_{name}"
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

    fn schema(&mut self, bytes: &[u8]) -> String {
        let mut imports = String::new();
        let mut parts = Vec::new();
        for (i, chunk) in bytes.chunks(DATA_CHUNK_BYTES).enumerate() {
            // The digest covers the complete compressed chunk. Shared files
            // live at package scope, so independent binding groups reuse them
            // without importing each other's models or changing class identity.
            let name = format!("_schema_{:x}", Sha256::digest(chunk));
            self.sources
                .entry(format!("_schemas/{name}.py"))
                .or_insert_with(|| {
                    format!(
                        "# Generated shared schema data.\nDATA: bytes = {}\n",
                        super::render::bytes(chunk)
                    )
                });
            writeln!(
                imports,
                "from ..._schemas.{name} import DATA as _schema_{i}"
            )
            .unwrap();
            parts.push(format!("_schema_{i},"));
        }
        // Keep chunk identities shared in memory as well as on disk. Joining
        // belongs to explicit schema introspection, never model import.
        writeln!(
            imports,
            "_schema: tuple[bytes, ...] = ({})",
            parts.join(" ")
        )
        .unwrap();
        imports
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
