//! Build a readable Python package with shared class identities and cached
//! prepared programs. Model compilation happens here, before application startup.
use super::codegen::write_atomic;
use anyhow::{Context, Result, bail, ensure};
use jsoncompat::{OpenApiDocument, SchemaDocument, SchemaOptions};
use jsoncompat_codegen::generate_dataclass_declarations;
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
) -> Result<()> {
    let document = OpenApiDocument::from_json(raw)?;
    let groups = document
        .component_schema_groups(&components.iter().cloned().collect(), models_per_module)?;
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
        previous.version <= 1,
        "unsupported package manifest; use a new output directory"
    );
    ensure!(
        previous.files.keys().all(|name| safe_filename(name)),
        "package manifest contains an invalid output filename"
    );
    let compiler = digest(&fs::read(std::env::current_exe()?)?);
    let mut manifest = Manifest {
        version: 1,
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
            exports.insert(model.clone(), module_name.clone());
            stub.push_str(&format!("from .{module_name} import {model} as {model}\n"));
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
        let input_digest = digest(&serde_json::to_vec(&json!([input, declarations, options]))?);
        let cached = previous.compiler == manifest.compiler
            && previous.modules.get(&module_name) == Some(&input_digest)
            && [&public_file, &private_file].iter().all(|file| {
                previous.files.get(*file).is_some_and(|expected| {
                    fs::read(destination.join(file)).is_ok_and(|bytes| digest(&bytes) == *expected)
                })
            });
        if cached {
            for file in [public_file, private_file] {
                manifest
                    .files
                    .insert(file.clone(), previous.files[&file].clone());
            }
        } else {
            let schema = SchemaDocument::from_json_with_options(&input, options)?;
            let module = generate_dataclass_declarations(&schema, &declarations)
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
        }
        manifest.modules.insert(module_name, input_digest);
    }
    let mut routing =
        String::from("# Generated export locations; no model imports occur here.\nMODELS = {\n");
    for (model, module) in &exports {
        routing.push_str(&format!("    {model:?}: {module:?},\n"));
    }
    routing.push_str("}\n");
    stage(&staging, &mut manifest, "_exports.py", &routing)?;
    stage(&staging, &mut manifest, "__init__.pyi", &stub)?;
    stage(&staging, &mut manifest, "__init__.py", INITIALIZER)?;
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
        exports.len(),
        manifest.modules.len(),
        rebuilt,
        destination.display()
    );
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
        && (name.ends_with(".py") || name.ends_with(".pyi"))
}
fn class_name(name: &str) -> String {
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
from ._exports import MODELS as _MODELS

__all__ = tuple(_MODELS)

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
