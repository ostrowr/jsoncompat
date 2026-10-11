//! Build a readable Python package with shared class identities and cached
//! prepared programs. Model compilation happens here, before application startup.
use super::codegen::write_atomic;
use anyhow::{Context, Result, bail, ensure};
use jsoncompat::{OpenApiDocument, SchemaDocument, SchemaOptions};
use jsoncompat_codegen::{DataclassMetadata, generate_dataclass_declarations_with_metadata};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    num::NonZeroUsize,
    path::{Path, PathBuf},
};

const MANIFEST: &str = ".jsoncompat-manifest.json";
#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: u32,
    compiler: String,
    modules: BTreeMap<String, String>,
    files: BTreeMap<String, String>,
    #[serde(default)]
    exports: BTreeMap<String, String>,
}
struct Staging(PathBuf);
impl Drop for Staging {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub(super) fn generate(
    raw: &Value,
    destination: &Path,
    components: &[String],
    renames: &[String],
    models_per_module: NonZeroUsize,
    options: &SchemaOptions,
    api: Option<&str>,
) -> Result<()> {
    let document = OpenApiDocument::from_json(raw)?;
    let api = api
        .map(|name| super::codegen_api::ApiPackage::prepare(&document, raw, name))
        .transpose()?;
    let empty_schemas = serde_json::Map::new();
    let empty_metadata = DataclassMetadata::default();
    let metadata = api.as_ref().map(|p| &p.metadata).unwrap_or(&empty_metadata);
    let groups = document.component_schema_groups_with_schemas(
        &components.iter().cloned().collect(),
        models_per_module,
        api.as_ref().map(|p| &p.schemas).unwrap_or(&empty_schemas),
    )?;
    ensure!(
        !groups.is_empty(),
        "OpenAPI document has no selected component schemas"
    );
    let mut names = BTreeMap::new();
    let mut used = BTreeSet::new();
    let mut overrides = BTreeMap::new();
    for rename in renames {
        let (component, name) = rename
            .split_once('=')
            .context("--rename must be COMPONENT=PythonName")?;
        ensure!(
            overrides
                .insert(component.to_owned(), name.to_owned())
                .is_none(),
            "duplicate rename for {component}"
        );
    }
    for name in groups.iter().flat_map(|group| group.schemas.keys()) {
        ensure!(
            !api.as_ref().is_some_and(|p| p.schemas.contains_key(name))
                || !overrides.contains_key(name),
            "--rename applies to component schemas, not generated operation model {name}"
        );
        let model = overrides.remove(name).unwrap_or_else(|| class_name(name));
        ensure!(
            used.insert(model.clone()),
            "component names collide at Python model {model}; use --rename COMPONENT=PythonName"
        );
        names.insert(name.clone(), model);
    }
    ensure!(
        overrides.is_empty(),
        "--rename references unselected components: {:?}",
        overrides.keys().collect::<Vec<_>>()
    );
    let previous: Manifest = if destination.join(MANIFEST).exists() {
        serde_json::from_slice(&fs::read(destination.join(MANIFEST))?)
            .context("reading generated-package manifest")?
    } else {
        Manifest::default()
    };
    ensure!(
        previous.version <= 2,
        "unsupported package manifest; use a new output directory"
    );
    ensure!(
        previous.files.keys().all(|name| safe_filename(name)),
        "package manifest contains an invalid output filename"
    );
    ensure!(
        previous
            .exports
            .keys()
            .all(|name| super::codegen_api::identifier(name)),
        "package manifest contains an invalid export name"
    );
    let compiler = digest(&fs::read(std::env::current_exe()?)?);
    let mut manifest = Manifest {
        version: 2,
        compiler,
        ..Default::default()
    };
    fs::create_dir_all(destination)?;
    let staging = Staging(destination.join(format!(
        ".jsoncompat-build-{}-{:016x}",
        std::process::id(),
        rand::random::<u64>()
    )));
    fs::create_dir(&staging.0)?;
    let mut exports = BTreeMap::new();
    let mut stub = String::from("# Generated public model exports.\n");
    let mut rebuilt = 0;
    for (index, mut group) in groups.into_iter().enumerate() {
        let module_name = format!("models_{index:04}");
        let companion = format!("_{module_name}_generated");
        let public_file = format!("{module_name}.py");
        let private_file = format!("{companion}.py");
        let declarations: BTreeMap<_, _> = group
            .schemas
            .keys()
            .map(|name| {
                (
                    format!("#/$defs/{}", name.replace('~', "~0").replace('/', "~1")),
                    names[name].clone(),
                )
            })
            .collect();
        for name in group.schemas.keys() {
            let model = &names[name];
            add_export(&mut exports, model, &module_name)?;
        }
        for (name, schema) in &mut group.schemas {
            if schema.is_boolean() {
                *schema = json!({"allOf":[schema.clone()]});
            }
            let pointer = format!("#/$defs/{}", name.replace('~', "~0").replace('/', "~1"));
            schema["x-jsoncompat"] = json!({"kind":"declaration", "stable_id":pointer, "name":names[name], "version":1, "schema_ref":pointer});
        }
        let selected: Vec<_> = declarations
            .keys()
            .map(|pointer| json!({"$ref":pointer}))
            .collect();
        let input = json!({"$schema":group.dialect, "$defs":group.schemas, "anyOf":selected});
        let input_digest = digest(&serde_json::to_vec(&json!([
            input,
            declarations,
            options,
            metadata.imports,
            metadata.base_imports,
            metadata.fields,
            metadata.bases
        ]))?);
        let cached = previous.compiler == manifest.compiler
            && previous.modules.get(&module_name) == Some(&input_digest)
            && [&public_file, &private_file].iter().all(|file| {
                previous.files.get(*file).is_some_and(|expected| {
                    fs::read(destination.join(file)).is_ok_and(|bytes| digest(&bytes) == *expected)
                })
            });
        let nested_exports = if cached {
            for file in [public_file, private_file] {
                manifest
                    .files
                    .insert(file.clone(), previous.files[&file].clone());
            }
            previous
                .exports
                .iter()
                .filter(|(_, module)| *module == &module_name)
                .map(|(name, _)| name.clone())
                .collect::<Vec<_>>()
        } else {
            let schema = SchemaDocument::from_json_with_options(&input, options)?;
            let module =
                generate_dataclass_declarations_with_metadata(&schema, &declarations, metadata)
                    .with_context(|| format!("generating {module_name}"))?;
            stage(
                &staging,
                &mut manifest,
                &public_file,
                &module.public_file(&companion),
            )?;
            stage(
                &staging,
                &mut manifest,
                &private_file,
                &module.private_file(),
            )?;
            rebuilt += 1;
            module.models
        };
        if api.is_some() {
            for name in nested_exports {
                add_export(&mut exports, &name, &module_name)?;
            }
        }
        manifest.modules.insert(module_name, input_digest);
    }
    let model_count = exports.len();
    if let Some(api) = api {
        let (source, public) = api.source(&exports)?;
        stage(&staging, &mut manifest, "api.py", &source)?;
        stage(&staging, &mut manifest, "_api.py", &api.identity_source())?;
        for name in public {
            exports.insert(name, "api".into());
        }
    }
    let mut routing =
        String::from("# Generated export locations; no model imports occur here.\nMODELS = {\n");
    for (model, module) in &exports {
        routing.push_str(&format!("    {model:?}: {module:?},\n"));
        stub.push_str(&format!("from .{module} import {model} as {model}\n"));
    }
    routing.push_str("}\n");
    stage(&staging, &mut manifest, "_exports.py", &routing)?;
    stage(&staging, &mut manifest, "__init__.pyi", &stub)?;
    stage(&staging, &mut manifest, "py.typed", "")?;
    manifest.exports = exports.clone();
    let public_names = exports
        .keys()
        .map(|name| format!("{name:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    let type_exports = stub
        .lines()
        .filter(|line| line.starts_with("from "))
        .map(|line| format!("    {line}\n"))
        .collect::<String>();
    stage(
        &staging,
        &mut manifest,
        "__init__.py",
        &format!(
            "{INITIALIZER}\nif _TYPE_CHECKING:\n{type_exports}\n__all__ = ({public_names},)\n"
        ),
    )?;
    // Finish every build before modifying the existing package. Unmanaged
    // files are never replaced. Deploy the resulting directory as one unit.
    for file in manifest.files.keys() {
        ensure!(
            !destination.join(file).exists() || previous.files.contains_key(file),
            "refusing to replace unmanaged package file {}",
            destination.join(file).display()
        );
    }
    for file in manifest.files.keys() {
        let source = staging.0.join(file);
        if source.exists() {
            let text = fs::read_to_string(&source)?;
            if !fs::read(destination.join(file)).is_ok_and(|bytes| bytes == text.as_bytes()) {
                write_atomic(&destination.join(file), &text)?;
            }
        }
    }
    for file in previous
        .files
        .keys()
        .filter(|file| !manifest.files.contains_key(*file))
    {
        match fs::remove_file(destination.join(file)) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    write_atomic(
        &destination.join(MANIFEST),
        &serde_json::to_string_pretty(&manifest)?,
    )?;
    eprintln!(
        "Generated {} models in {} modules ({} rebuilt) at {}",
        model_count,
        manifest.modules.len(),
        rebuilt,
        destination.display()
    );
    Ok(())
}
fn add_export(exports: &mut BTreeMap<String, String>, name: &str, module: &str) -> Result<()> {
    if let Some(previous) = exports.insert(name.into(), module.into()) {
        ensure!(
            previous == module,
            "generated model export {name} collides across modules; rename the component or operation"
        );
    }
    Ok(())
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn stage(directory: &Staging, manifest: &mut Manifest, name: &str, source: &str) -> Result<()> {
    if !safe_filename(name) {
        bail!("invalid generated filename {name}");
    }
    fs::write(directory.0.join(name), source)?;
    manifest
        .files
        .insert(name.into(), digest(source.as_bytes()));
    Ok(())
}
fn safe_filename(name: &str) -> bool {
    !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
        && (name.ends_with(".py") || name.ends_with(".pyi") || name == "py.typed")
}
pub(super) fn class_name(name: &str) -> String {
    let mut result = String::new();
    for word in name
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
    {
        let mut characters = word.chars();
        result.extend(characters.next().unwrap().to_uppercase());
        result.extend(characters);
    }
    if result.is_empty()
        || result.starts_with(|c: char| c.is_ascii_digit())
        || matches!(result.as_str(), "None" | "True" | "False")
    {
        result.insert_str(0, "Model");
    }
    result
}
const INITIALIZER: &str = r#""""Generated model package. Import a model by name; unrelated modules stay unloaded."""
from importlib import import_module as _import_module
from typing import TYPE_CHECKING as _TYPE_CHECKING
from ._exports import MODELS as _MODELS

def __getattr__(name: str):
    module = _MODELS.get(name)
    if module is None:
        raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
    value = getattr(_import_module(f".{module}", __name__), name)
    globals()[name] = value
    return value

def __dir__():
    return sorted(set(globals()) | _MODELS.keys())
"#;
