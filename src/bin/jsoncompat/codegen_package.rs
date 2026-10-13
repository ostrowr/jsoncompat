//! Build a readable Python package with shared class identities and cached
//! prepared programs. Model compilation happens here, before application startup.
use super::codegen::write_atomic;
use anyhow::{Context, Result, bail, ensure};
use jsoncompat::{OpenApiDocument, SchemaDocument, SchemaOptions};
use jsoncompat_codegen::{
    DataclassMetadata, generate_dataclass_resource_package_with_metadata,
    python_names::module_names,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    num::NonZeroUsize,
    path::{Path, PathBuf},
    time::Instant,
};

const MANIFEST: &str = ".jsoncompat-manifest.json";
#[derive(Default, Deserialize, Serialize)]
struct Manifest {
    version: u32,
    compiler: String,
    modules: BTreeMap<String, String>,
    #[serde(default)]
    module_files: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    module_exports: BTreeMap<String, BTreeMap<String, String>>,
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

struct Timings(Option<(Instant, Instant)>);
impl Timings {
    fn new() -> Self {
        Self(std::env::var_os("JSONCOMPAT_CODEGEN_TIMING").map(|_| {
            let now = Instant::now();
            (now, now)
        }))
    }

    fn mark(&mut self, stage: &str) {
        if let Some((started, previous)) = &mut self.0 {
            let now = Instant::now();
            eprintln!(
                "Codegen timing: {stage}: {:.6}s ({:.6}s total)",
                now.duration_since(*previous).as_secs_f64(),
                now.duration_since(*started).as_secs_f64(),
            );
            *previous = now;
        }
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
    let mut timings = Timings::new();
    let document = OpenApiDocument::from_json(raw)?;
    timings.mark("validate OpenAPI document");
    let api = api
        .map(|name| super::codegen_api::ApiPackage::prepare(&document, raw, name))
        .transpose()?;
    timings.mark("prepare operation contracts");
    let empty_schemas = serde_json::Map::new();
    let empty_metadata = DataclassMetadata::default();
    let metadata = api.as_ref().map(|p| &p.metadata).unwrap_or(&empty_metadata);
    let groups = document.component_schema_groups_with_schemas(
        &components.iter().cloned().collect(),
        if api.is_some() {
            NonZeroUsize::new(usize::MAX).unwrap()
        } else {
            models_per_module
        },
        api.as_ref().map(|p| &p.schemas).unwrap_or(&empty_schemas),
    )?;
    timings.mark("group component schemas");
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
        previous.version <= 7,
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
    for file in previous.files.keys() {
        check_parents(destination, file)?;
    }
    let compiler = digest(&fs::read(std::env::current_exe()?)?);
    let mut manifest = Manifest {
        version: 7,
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
    let mut resource_sources = BTreeMap::new();
    for directory in ["models", "operations", "_cycles"] {
        stage(
            &staging,
            &mut manifest,
            &format!("{directory}/__init__.py"),
            "# Generated package; import modules directly.\n",
        )?;
    }
    let component_modules = module_names(
        names
            .iter()
            .filter(|(schema, _)| {
                !api.as_ref()
                    .is_some_and(|p| p.schemas.contains_key(*schema))
            })
            .map(|(_, name)| name.as_str()),
    );
    let owners: BTreeMap<_, _> = names
        .iter()
        .map(|(schema, name)| {
            let module = api
                .as_ref()
                .and_then(|p| p.owners.get(schema))
                .cloned()
                .unwrap_or_else(|| format!("models.{}", component_modules[name]));
            (name.clone(), module)
        })
        .collect();
    let group_names = module_names(
        groups
            .iter()
            .map(|g| names[g.schemas.keys().next().unwrap()].as_str()),
    );
    let one_group = groups.len() == 1;
    timings.mark("prepare output and compiler fingerprint");
    for mut group in groups {
        let anchor = &names[group.schemas.keys().next().unwrap()];
        let group_name = if one_group && group.schemas.len() > 1 {
            "shared"
        } else {
            &group_names[anchor]
        };
        let module_name = format!("_models.{group_name}");
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
            add_export(&mut exports, model, &owners[model])?;
        }
        for (name, schema) in &mut group.schemas {
            if schema.is_boolean() {
                *schema = json!({"allOf":[schema.clone()]});
            }
            let pointer = format!("#/$defs/{}", name.replace('~', "~0").replace('/', "~1"));
            schema["x-jsoncompat"] = json!({"kind":"declaration", "stable_id":pointer, "name":names[name], "version":1, "schema_ref":pointer});
        }
        // This root is only a namespace for the selected declarations. Keep
        // every definition reachable without analyzing a union of unrelated
        // models; the namespace itself is never emitted as a model.
        let selected: serde_json::Map<_, _> = declarations
            .iter()
            .map(|(pointer, name)| (name.clone(), json!({"$ref":pointer})))
            .collect();
        let input = Value::Object(serde_json::Map::from_iter([
            ("$schema".into(), Value::String(group.dialect)),
            ("$defs".into(), Value::Object(group.schemas)),
            ("type".into(), Value::String("object".into())),
            ("properties".into(), Value::Object(selected)),
        ]));
        let input_digest = digest(&serde_json::to_vec(&(
            &input,
            &declarations,
            options,
            &metadata.imports,
            &metadata.base_imports,
            &metadata.fields,
            &metadata.bases,
            declarations
                .values()
                .map(|name| (name, &owners[name]))
                .collect::<BTreeMap<_, _>>(),
        ))?);
        let cached_files = previous.module_files.get(&module_name);
        let cached = previous.version == 7
            && previous.compiler == manifest.compiler
            && previous.modules.get(&module_name) == Some(&input_digest)
            && cached_files.is_some_and(|files| {
                !files.is_empty()
                    && files.iter().all(|file| {
                        previous.files.get(file).is_some_and(|expected| {
                            fs::read(destination.join(file))
                                .is_ok_and(|bytes| digest(&bytes) == *expected)
                        })
                    })
            });
        timings.mark(&format!("{module_name}: prepare group and check cache"));
        let nested_exports = if cached {
            let files = cached_files.unwrap();
            manifest
                .module_files
                .insert(module_name.clone(), files.clone());
            for file in files {
                manifest
                    .files
                    .insert(file.clone(), previous.files[file].clone());
            }
            previous.module_exports[&module_name].clone()
        } else {
            let schema = SchemaDocument::from_json_with_options_and_cache(
                &input,
                options,
                document.schema_validation_cache(),
            )?;
            timings.mark(&format!("{module_name}: validate schema document"));
            let module = generate_dataclass_resource_package_with_metadata(
                &schema,
                &declarations,
                metadata,
                &owners,
            )
            .with_context(|| format!("generating {module_name}"))?;
            timings.mark(&format!(
                "{module_name}: generate and render prepared models"
            ));
            let files = module.files;
            manifest
                .module_files
                .insert(module_name.clone(), files.keys().cloned().collect());
            for (file, source) in files {
                if api.is_some() && file.starts_with("operations/") {
                    resource_sources.insert(file, source);
                } else {
                    stage(&staging, &mut manifest, &file, &source)?;
                }
            }
            timings.mark(&format!("{module_name}: stage package files"));
            rebuilt += 1;
            module.exports
        };
        if api.is_some() {
            for (name, owner) in &nested_exports {
                add_export(&mut exports, name, owner)?;
            }
        }
        manifest
            .module_exports
            .insert(module_name.clone(), nested_exports);
        manifest.modules.insert(module_name, input_digest);
    }
    let model_count = exports.len();
    if let Some(api) = &api {
        for (alias, (_, owner)) in &api.aliases {
            add_export(&mut exports, alias, owner)?;
        }
    }
    let model_exports = exports.clone();
    let model_stub = model_exports
        .iter()
        .map(|(name, module)| format!("from .{module} import {name} as {name}\n"))
        .collect::<String>();
    let model_names = model_exports
        .keys()
        .map(|s| format!("{s:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    let model_initializer = INITIALIZER
        .replace("from ._exports", "from .._exports")
        .replace("EXPORTS as _MODELS", "MODELS as _MODELS");
    let model_initializer = model_initializer.replace("f\".{module}\"", "f\"..{module}\"");
    stage(
        &staging,
        &mut manifest,
        "models/__init__.py",
        &format!("{model_initializer}\n__all__: tuple[str, ...] = ({model_names},)\n"),
    )?;
    stage(
        &staging,
        &mut manifest,
        "models/__init__.pyi",
        &model_stub.replace("from .", "from .."),
    )?;
    if let Some(api) = api {
        let (files, public) = api.sources(&exports)?;
        for (file, source) in files {
            let source = if file.starts_with("operations/") {
                let declarations = match resource_sources.remove(&file) {
                    Some(source) => source,
                    None => fs::read_to_string(destination.join(&file))?,
                };
                format!(
                    "{}\n\n{source}",
                    declarations
                        .split("# --- Operation metadata ---")
                        .next()
                        .unwrap()
                        .trim_end()
                )
            } else {
                source
            };
            stage(&staging, &mut manifest, &file, &source)?;
        }
        for name in public {
            let owner = api.alias_owner(&name).unwrap_or("api");
            exports.insert(name, owner.into());
        }
    }
    let mut routing =
        String::from("# Generated export locations; no model imports occur here.\nMODELS = {\n");
    for (model, module) in &model_exports {
        routing.push_str(&format!("    {model:?}: {module:?},\n"));
    }
    routing.push_str("}\nEXPORTS = {**MODELS,\n");
    for (model, module) in &exports {
        stub.push_str(&format!("from .{module} import {model} as {model}\n"));
        if !model_exports.contains_key(model) {
            routing.push_str(&format!("    {model:?}: {module:?},\n"));
        }
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
    stage(
        &staging,
        &mut manifest,
        "__init__.py",
        &format!("{INITIALIZER}\n__all__: tuple[str, ...] = ({public_names},)\n"),
    )?;
    let manifest_source = serde_json::to_string_pretty(&manifest)?;
    timings.mark("render and stage package exports");
    // Finish every build before modifying the existing package. Unmanaged
    // files are never replaced. Deploy the resulting directory as one unit.
    for file in manifest.files.keys() {
        check_parents(destination, file)?;
        ensure!(
            !destination.join(file).exists() || previous.files.contains_key(file),
            "refusing to replace unmanaged package file {}",
            destination.join(file).display()
        );
    }
    timings.mark("check publication paths");
    publish_staged_tree(&staging.0, destination, Path::new(""), &manifest)?;
    timings.mark("publish package files");
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
    // Only remove empty generated directories; preserve notes and bytecode.
    let mut obsolete_dirs = BTreeSet::new();
    for file in previous
        .files
        .keys()
        .filter(|file| !manifest.files.contains_key(*file))
    {
        let mut parent = Path::new(file).parent();
        while let Some(path) = parent.filter(|p| !p.as_os_str().is_empty()) {
            obsolete_dirs.insert(path.to_path_buf());
            parent = path.parent();
        }
    }
    for directory in obsolete_dirs.iter().rev() {
        match fs::remove_dir(destination.join(directory)) {
            Ok(()) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::DirectoryNotEmpty
                ) => {}
            Err(error) => return Err(error.into()),
        }
    }
    write_atomic(&destination.join(MANIFEST), &manifest_source)?;
    timings.mark("remove obsolete files and publish manifest");
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

fn publish_staged_tree(
    staging: &Path,
    destination: &Path,
    relative: &Path,
    manifest: &Manifest,
) -> Result<()> {
    for entry in fs::read_dir(staging.join(relative))? {
        let entry = entry?;
        let relative = relative.join(entry.file_name());
        let source = entry.path();
        let target = destination.join(&relative);
        if entry.file_type()?.is_dir() && target.try_exists()? {
            // Existing directories may contain cached models or caller-owned
            // notes. Merge into them; preflight has checked every managed path.
            publish_staged_tree(staging, destination, &relative, manifest)?;
            continue;
        }
        if entry.file_type()?.is_file() {
            let name = relative.to_string_lossy().replace('\\', "/");
            let expected = &manifest.files[&name];
            if fs::read(&target).is_ok_and(|bytes| digest(&bytes) == *expected) {
                continue;
            }
        }
        // Staging lives on the destination filesystem. Cold subtrees move as
        // one unit; changed files in existing trees are atomically replaced.
        // The final manifest is flushed, and a later build checks its digests
        // to repair any missing or damaged files after an interrupted build.
        fs::rename(&source, &target).with_context(|| format!("writing {}", target.display()))?;
    }
    Ok(())
}

fn stage(directory: &Staging, manifest: &mut Manifest, name: &str, source: &str) -> Result<()> {
    if !safe_filename(name) {
        bail!("invalid generated filename {name}");
    }
    let target = directory.0.join(name);
    fs::create_dir_all(target.parent().unwrap())?;
    fs::write(target, source)?;
    manifest
        .files
        .insert(name.into(), digest(source.as_bytes()));
    Ok(())
}
fn safe_filename(name: &str) -> bool {
    let parts = name.split('/').collect::<Vec<_>>();
    parts.iter().all(|part| {
        !part.is_empty()
            && !part.starts_with('.')
            && part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
    }) && (name.ends_with(".py") || name.ends_with(".pyi") || name == "py.typed")
}
fn check_parents(destination: &Path, file: &str) -> Result<()> {
    let mut parent = destination.to_path_buf();
    let path = Path::new(file);
    for part in path.parent().into_iter().flat_map(Path::components) {
        parent.push(part);
        match fs::symlink_metadata(&parent) {
            Ok(metadata) => ensure!(
                metadata.is_dir() && !metadata.file_type().is_symlink(),
                "refusing to write generated files through non-directory or symlink {}",
                parent.display()
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
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
from ._exports import EXPORTS as _MODELS

def __getattr__(name: str):
    module = _MODELS.get(name)
    if module is None:
        raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
    value = getattr(_import_module(f".{module}", __package__), name)
    globals()[name] = value
    return value

def __dir__():
    return sorted(set(globals()) | _MODELS.keys())
"#;
