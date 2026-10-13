use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};
#[path = "support/python_env.rs"]
mod python_env;
struct Work(PathBuf);
impl Work {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "jsoncompat-package-{}-{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        fs::create_dir(&path).unwrap();
        let work = Self(path);
        // The cache deliberately fingerprints its compiler. Keep this test's
        // compiler fixed even if another Cargo process rebuilds the checkout.
        fs::copy(env!("CARGO_BIN_EXE_jsoncompat"), work.compiler()).unwrap();
        work
    }
    fn compiler(&self) -> PathBuf {
        self.0
            .join(format!("compiler{}", std::env::consts::EXE_SUFFIX))
    }
    fn generate(&self, extra: &[&str]) -> Output {
        self.generate_with_group_size("2", extra)
    }
    fn generate_with_group_size(&self, size: &str, extra: &[&str]) -> Output {
        Command::new(self.compiler())
            .current_dir(&self.0)
            .args([
                "codegen",
                "--target",
                "dataclasses",
                "--openapi",
                "api.json",
                "--output",
                "models",
                "--models-per-module",
                size,
            ])
            .args(extra)
            .output()
            .unwrap()
    }
    fn write(&self, value: &Value) {
        fs::write(self.0.join("api.json"), value.to_string()).unwrap();
    }
}
impl Drop for Work {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn success(output: Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stderr).unwrap()
}
fn record(width: usize) -> Value {
    let properties: serde_json::Map<_, _> = (0..width)
        .map(|i| (format!("f{i:03}"), json!({"type":"integer","minimum":0})))
        .collect();
    json!({"type":"object","required":properties.keys().collect::<Vec<_>>(),"properties":properties,"additionalProperties":false})
}
fn api() -> Value {
    json!({"openapi":"3.1.0","info":{"title":"Package regression","version":"1"},"components":{"schemas":{
        "Customer":{"type":"object","properties":{"id":{"type":"integer","minimum":0},"orders":{"type":"array","items":{"$ref":"#/components/schemas/Order"}}},"required":["id","orders"],"additionalProperties":false},
        "Order":{"type":"object","properties":{"customer":{"$ref":"#/components/schemas/Customer"},"total":{"type":"integer","minimum":0}},"required":["customer","total"],"additionalProperties":false},
        "Wide":record(200), "Other":record(5), "Unused": {"type":"boolean"}
    }}})
}
fn contents(directory: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, out);
            } else if path
                .extension()
                .is_some_and(|ext| ext == "py" || ext == "pyi")
            {
                out.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(directory, directory, &mut out);
    out
}
fn declaration(name: &str) -> bool {
    name.starts_with("models/") && !name.rsplit('/').next().unwrap().starts_with('_')
}

#[test]
fn merged_implementations_preserve_user_declaration_names() {
    let work = Work::new();
    let mut spec = api();
    spec["components"]["schemas"]["Blob"] = json!({"type":"string"});
    work.write(&spec);
    success(work.generate_with_group_size(
        "128",
        &[
            "--rename",
            "Customer=_schema",
            "--rename",
            "Order=_program",
            "--rename",
            "Other=_schema_0",
            "--rename",
            "Unused=_program_0",
            "--rename",
            "Wide=_install",
            "--rename",
            "Blob=bytes",
        ],
    ));
    success(
        python_env::python_command()
            .arg("-c")
            .arg(
                r#"
import sys
sys.path.insert(0, sys.argv[1])
from models import _schema, _program, _schema_0, _program_0, _install, bytes
for model in (_schema, _program, _schema_0, _program_0, _install, bytes):
    assert isinstance(model, type), model
customer = _schema(id=1, orders=[])
order = _program(customer=customer, total=2)
assert type(_program.deserialize(order.serialize()).customer) is _schema
assert _program_0(root=True).serialize() == 'true'
assert bytes(root='text').serialize() == '"text"'
assert _install.deserialize(_install(**{f'f{i:03}': i for i in range(200)}).serialize()).f199 == 199
"#,
            )
            .arg(&work.0)
            .output()
            .unwrap(),
    );
    let typed = python_env::ty_command()
        .arg("--project")
        .arg(&work.0)
        .arg(work.0.join("models"))
        .output()
        .unwrap();
    assert!(
        typed.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&typed.stdout),
        String::from_utf8_lossy(&typed.stderr)
    );
}

#[test]
fn older_packages_migrate_without_removing_unmanaged_files() {
    for (previous_version, previous_file) in [
        (3, "models_0000.py"),
        (4, "_models/wide/_schema_wide_0000.py"),
        (5, "_models/wide/_init_wide.py"),
    ] {
        let work = Work::new();
        work.write(&api());
        let output = work.0.join("models");
        fs::create_dir_all(&output).unwrap();
        fs::create_dir_all(output.join(previous_file).parent().unwrap()).unwrap();
        fs::write(output.join(previous_file), "# previous generated data\n").unwrap();
        fs::write(output.join("notes.txt"), "keep me").unwrap();
        fs::write(
            output.join(".jsoncompat-manifest.json"),
            json!({
                "version": previous_version, "compiler": "previous compiler", "modules": {},
                "files": {previous_file: "previous digest"}
            })
            .to_string(),
        )
        .unwrap();
        success(work.generate(&[]));
        assert!(!output.join(previous_file).exists());
        assert!(output.join("models/__init__.py").exists());
        assert_eq!(
            fs::read_to_string(output.join("notes.txt")).unwrap(),
            "keep me"
        );
        fs::create_dir_all(output.join("_models/wide")).unwrap();
        fs::write(output.join("_models/wide/notes.txt"), "nested note").unwrap();
        success(work.generate(&["--component", "Order"]));
        assert_eq!(
            fs::read_to_string(output.join("_models/wide/notes.txt")).unwrap(),
            "nested note"
        );
        assert!(!output.join("_models/other").exists());
    }
}

#[test]
fn schema_chunks_share_storage_across_independent_groups_and_cache_owners() {
    let work = Work::new();
    let object = json!({"type":"object","properties":{"payload":record(3)},"required":["payload"]});
    work.write(&json!({"openapi":"3.1.0","info":{"title":"Shared schemas","version":"1"},"components":{"schemas":{
        "A":object, "B":object, "C":{"type":"string"}
    }}}));
    assert!(success(work.generate_with_group_size("1", &[])).contains("(3 rebuilt)"));
    let package = work.0.join("models");
    let before = contents(&package);
    let manifest: Value =
        serde_json::from_slice(&fs::read(package.join(".jsoncompat-manifest.json")).unwrap())
            .unwrap();
    assert_eq!(manifest["version"], 6);
    let a = manifest["module_files"]["_models.a"].as_array().unwrap();
    let b = manifest["module_files"]["_models.b"].as_array().unwrap();
    let shared = a
        .iter()
        .filter_map(Value::as_str)
        .filter(|name| name.starts_with("_data/") && b.contains(&json!(name)))
        .collect::<Vec<_>>();
    assert!(
        !shared.is_empty(),
        "independent groups must reuse identical chunks"
    );
    success(
        python_env::python_command()
            .arg("-c")
            .arg(
                r#"
import json, sys, zlib
from unittest.mock import patch
sys.path.insert(0, sys.argv[1])
def forbidden(*a, **k): raise AssertionError('schema introspection during model use')
with patch.object(zlib, 'decompress', forbidden):
    from models import A
    assert 'models.models.b' not in sys.modules
    value = {'payload': {'f000': 0, 'f001': 1, 'f002': 2}}
    first = A.from_value(value)
    from models import B
    second = B.from_value(value)
    assert type(first.payload) is not type(second.payload)
    assert json.loads(first.serialize()) == json.loads(second.serialize()) == value
    a = vars(type(first.payload))['__jsoncompat_schema__']._compressed
    b = vars(type(second.payload))['__jsoncompat_schema__']._compressed
    assert isinstance(a, tuple) and isinstance(b, tuple) and a
    assert len(a) == len(b) and all(x is y for x, y in zip(a, b))
with patch.object(zlib, 'decompress', wraps=zlib.decompress) as decompress:
    source = type(first.payload).__jsoncompat_schema__
    assert type(first.payload).__jsoncompat_schema__ is source
    assert decompress.call_count == 1
    assert type(second.payload).__jsoncompat_schema__ == source
    assert decompress.call_count == 2
assert json.loads(source)['properties']['f002']['minimum'] == 0
"#,
            )
            .arg(&work.0)
            .output()
            .unwrap(),
    );
    fs::remove_file(package.join(shared[0])).unwrap();
    assert!(success(work.generate_with_group_size("1", &[])).contains("(2 rebuilt)"));
    assert_eq!(contents(&package), before);
    fs::write(package.join(shared[0]), "# corrupted chunk\n").unwrap();
    assert!(success(work.generate_with_group_size("1", &[])).contains("(2 rebuilt)"));
    assert_eq!(contents(&package), before);
    fs::write(package.join("_data/notes.txt"), "caller owned").unwrap();
    assert!(
        success(work.generate_with_group_size("1", &["--component", "A"])).contains("(0 rebuilt)")
    );
    for file in &shared {
        let retained = fs::read_to_string(package.join(file)).unwrap();
        let old = String::from_utf8_lossy(&before[*file]);
        assert!(
            retained
                .lines()
                .all(|line| old.lines().any(|old| old == line)),
            "only unowned data may be pruned"
        );
    }
    assert!(!package.join("models/b.py").exists());
    success(work.generate_with_group_size("1", &["--component", "C"]));
    for file in &shared {
        assert!(!package.join(file).exists(), "last owner removed: {file}");
    }
    assert_eq!(
        fs::read_to_string(package.join("_data/notes.txt")).unwrap(),
        "caller owned"
    );
}

#[cfg(unix)]
#[test]
fn nested_output_does_not_follow_directory_symlinks() {
    let work = Work::new();
    work.write(&api());
    let output = work.0.join("models");
    let outside = work.0.join("unmanaged");
    fs::create_dir_all(&output).unwrap();
    fs::create_dir_all(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, output.join("models")).unwrap();
    let result = work.generate(&[]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("symlink"));
    assert!(fs::read_dir(outside).unwrap().next().is_none());
    assert!(!output.join("__init__.py").exists());
}

#[test]
fn publication_preserves_unchanged_files_and_checks_all_conflicts_before_writing() {
    let work = Work::new();
    work.write(&api());
    let output = work.0.join("models");
    fs::create_dir(&output).unwrap();
    fs::create_dir(output.join("models")).unwrap();
    let conflict = output.join("models/__init__.py");
    fs::write(&conflict, "# caller owned\n").unwrap();
    let rejected = work.generate(&[]);
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("unmanaged package file"));
    assert_eq!(
        contents(&output),
        BTreeMap::from([("models/__init__.py".into(), b"# caller owned\n".to_vec())]),
        "a conflict discovered late in preflight must leave every generated file unpublished"
    );
    assert_eq!(fs::read_dir(&output).unwrap().count(), 1);
    fs::remove_file(conflict).unwrap();
    success(work.generate(&[]));

    let unchanged = output.join("__init__.py");
    let modified = std::time::UNIX_EPOCH + std::time::Duration::from_secs(60);
    fs::OpenOptions::new()
        .write(true)
        .open(&unchanged)
        .unwrap()
        .set_modified(modified)
        .unwrap();
    let before = contents(&output);
    let corrupted = before
        .keys()
        .find(|name| name.ends_with("/wide.py"))
        .unwrap();
    fs::write(output.join(corrupted), "# corrupted prepared data\n").unwrap();
    assert!(success(work.generate(&[])).contains("(1 rebuilt)"));
    assert_eq!(contents(&output), before);
    assert_eq!(
        fs::metadata(unchanged).unwrap().modified().unwrap(),
        modified
    );
    assert!(fs::read_dir(&output).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".jsoncompat-build-")
    }));
}

#[test]
fn namespace_generation_preserves_true_false_and_constrained_component_roots() {
    let work = Work::new();
    work.write(&json!({
        "openapi":"3.1.0", "info":{"title":"Boolean roots", "version":"1"},
        "components":{"schemas":{
            "Allow":true,
            "Reject":false,
            "Count":{"type":"integer", "minimum":0}
        }}
    }));
    success(work.generate_with_group_size("128", &[]));
    success(
        python_env::python_command()
            .arg("-c")
            .arg(
                r#"
import json, sys
sys.path.insert(0, sys.argv[1])
from models import Allow, Reject, Count
for value in (None, False, True, 0, -1, 0.5, '', 'text', [], {}, {'key': [1]}):
    assert Allow.from_value(value).to_value() == value
    assert Allow.deserialize(json.dumps(value)).to_value() == value
    for convert, instance in ((Reject.from_value, value), (Reject.deserialize, json.dumps(value))):
        try: convert(instance)
        except ValueError: pass
        else: raise AssertionError(('false schema accepted', value))
assert Count.from_value(3).to_value() == 3
assert Count.deserialize('3').to_value() == 3
for value in (None, False, -1, 0.5, '3', [], {}):
    for convert, instance in ((Count.from_value, value), (Count.deserialize, json.dumps(value))):
        try: convert(instance)
        except ValueError: pass
        else: raise AssertionError(('integer constraint omitted', value))
allow_schema = {'anyOf': [
    {'enum': [None]}, {'enum': [False, True]},
    {'type': 'object', 'properties': {}, 'minProperties': 0},
    {'type': 'array', 'items': True, 'minItems': 0},
    {'type': 'string', 'minLength': 0}, {'type': 'number'}
]}
for model, expected in ((Allow, allow_schema), (Reject, {'not': True}), (Count, {'minimum': 0, 'multipleOf': 1, 'type': 'integer'})):
    source = json.loads(model.__jsoncompat_schema__)
    metadata = source.pop('x-jsoncompat')
    assert source == expected, (model, source)
    assert metadata == {
        'kind': 'declaration', 'stable_id': f'#/$defs/{model.__name__}',
        'name': model.__name__, 'version': 1, 'schema_ref': f'#/$defs/{model.__name__}'
    }, metadata
"#,
            )
            .arg(&work.0)
            .output()
            .unwrap(),
    );
}

#[test]
fn package_builds_preserve_identity_recursion_laziness_and_incremental_outputs() {
    let work = Work::new();
    let mut input = api();
    work.write(&input);
    assert!(success(work.generate(&[])).contains("5 models in 3 modules (3 rebuilt)"));
    let before = contents(&work.0.join("models"));
    assert!(
        !before.keys().any(|name| name.contains("/_init_")),
        "each declaration includes its own prepared initializer"
    );
    assert!(
        !before.keys().any(|name| name.contains("/_program_")),
        "small prepared programs belong in the existing model initializer"
    );
    assert!(success(work.generate(&[])).contains("(0 rebuilt)"));
    assert_eq!(contents(&work.0.join("models")), before);
    let output = python_env::python_command().arg("-c").arg(r#"
import builtins, compileall, dataclasses, inspect, json, sys, zlib
from unittest.mock import patch
from pathlib import Path
import jsoncompat
from jsoncompat.codegen import dataclasses as dc
sys.path.insert(0, sys.argv[1])
assert compileall.compile_dir(str(Path(sys.argv[1]) / 'models'), quiet=1)
def forbidden(*a, **k): raise AssertionError('runtime compilation or reflection')
with patch.object(builtins, 'compile', forbidden), patch.object(dataclasses, 'dataclass', forbidden), patch.object(inspect, 'get_annotations', forbidden), patch.object(jsoncompat, 'validator_for', forbidden), patch.object(zlib, 'decompress', forbidden):
    import models
    assert not any(name.startswith('models._models.') for name in sys.modules)
    from models.models import Order, Customer
    assert Order is models.Order and Customer is models.Customer
    value = {'customer': {'id': 7, 'orders': []}, 'total': 9}
    order = Order.deserialize(json.dumps(value))
    assert type(order.customer) is Customer
    assert type(Customer(id=8, orders=[order]).orders[0]) is Order
    assert json.loads(order.serialize()) == value
    assert 'Wide' not in vars(models)
    from models import Wide
    payload = {f'f{i:03}': i for i in range(200)}
    assert Wide.deserialize(json.dumps(payload)).to_value() == payload
    try: Wide.deserialize(json.dumps(dict(payload, f199=-1)))
    except ValueError: pass
    else: raise AssertionError('wide constraint was omitted')
# The recursive models retain shared streams, and explicit introspection alone
# expands them into the original complete schema, once per class.
order_source = vars(Order)['__jsoncompat_schema__']._compressed
customer_source = vars(Customer)['__jsoncompat_schema__']._compressed
assert isinstance(order_source, dc.SchemaSource)
assert isinstance(customer_source, dc.SchemaSource)
assert len(order_source.parts[1]) == len(customer_source.parts[1])
assert all(a is b for a, b in zip(order_source.parts[1], customer_source.parts[1]))
with patch.object(zlib, 'decompress', wraps=zlib.decompress) as decompress:
    source = Order.__jsoncompat_schema__
    assert Order.__jsoncompat_schema__ is source
    assert decompress.call_count == len(order_source.parts)
    source_schema = json.loads(source)
    assert source_schema['properties']['customer']['$ref'] == '#/$defs/Customer'
    assert source_schema['$defs']['Customer']['properties']['id']['minimum'] == 0
    assert source_schema['$defs']['Customer']['properties']['orders']['items']['$ref'] == '#/$defs/Order'
    assert source_schema['$defs']['Order']['properties']['total']['minimum'] == 0
assert len(models.__all__) == 5
assert set(models.__all__) <= set(dir(models))
try: models.NoSuchModel
except AttributeError: pass
else: raise AssertionError('unknown export')
"#).arg(&work.0).output().unwrap();
    success(output);
    input["components"]["schemas"]["Wide"]["properties"]["f199"]["minimum"] = json!(1);
    work.write(&input);
    assert!(success(work.generate(&[])).contains("(1 rebuilt)"));
    let after = contents(&work.0.join("models"));
    assert_eq!(
        before
            .keys()
            .filter(|name| !name.starts_with("_data/"))
            .collect::<Vec<_>>(),
        after
            .keys()
            .filter(|name| !name.starts_with("_data/"))
            .collect::<Vec<_>>(),
        "a constraint edit must not repack neighboring models"
    );
    let changed = before
        .iter()
        .filter(|(name, data)| after.get(*name) != Some(*data))
        .map(|(name, _)| name)
        .collect::<Vec<_>>();
    assert!(!changed.is_empty());
    assert!(
        changed.iter().all(|name| name.starts_with("_data/")
            || name.ends_with("/wide.py")
            || name.contains("_program_wide_")
            || name.as_str() == "models/wide.py"
            || name.contains("_plan_")),
        "only Wide's prepared data and the shared decision plan should change: {changed:?}"
    );
    fs::write(work.0.join("models/notes.txt"), "keep me").unwrap();
    success(work.generate(&["--component", "Order"]));
    let manifest: Value =
        serde_json::from_slice(&fs::read(work.0.join("models/.jsoncompat-manifest.json")).unwrap())
            .unwrap();
    assert_eq!(manifest["modules"].as_object().unwrap().len(), 1);
    assert_eq!(
        fs::read_to_string(work.0.join("models/notes.txt")).unwrap(),
        "keep me"
    );
    assert!(!work.0.join("models/_models/wide/__init__.py").exists());
}
#[test]
fn component_names_resources_and_failed_regeneration_are_explicit() {
    let work = Work::new();
    let document = json!({"openapi":"3.1.0","info":{"title":"Resources","version":"1"},"components":{"schemas":{
        "foo-bar":{"$ref":"https://example.test/record"}, "FooBar":{"type":"integer"}
    }}});
    work.write(&document);
    let collision = work.generate(&[]);
    assert!(!collision.status.success());
    assert!(String::from_utf8_lossy(&collision.stderr).contains("collide"));
    fs::write(
        work.0.join("options.json"),
        json!({"resources":{"https://example.test/record":record(5)}}).to_string(),
    )
    .unwrap();
    success(work.generate(&[
        "--rename",
        "foo-bar=RemoteRecord",
        "--schema-options",
        "options.json",
    ]));
    let before = contents(&work.0.join("models"));
    let invalid = work.generate(&["--component", "Absent"]);
    assert!(!invalid.status.success());
    assert_eq!(contents(&work.0.join("models")), before);
    let output = python_env::python_command().arg("-c").arg("import sys; sys.path.insert(0,sys.argv[1]); from models import RemoteRecord; assert RemoteRecord.deserialize('{\"f000\":0,\"f001\":1,\"f002\":2,\"f003\":3,\"f004\":4}').serialize()")
        .arg(&work.0).output().unwrap();
    success(output);
}

#[test]
fn split_model_artifacts_recover_missing_and_modified_chunks_and_remove_stale_ones() {
    let work = Work::new();
    let mut spec = api();
    spec["components"]["schemas"]["Wide"] = record(1200);
    // High-entropy constants exercise a large *single* prepared program, rather
    // than relying on repeated fields which compress extremely well.
    let mut state = 17_u64;
    let marker = (0..160_000)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (b'a' + (state % 26) as u8) as char
        })
        .collect::<String>();
    spec["components"]["schemas"]["Wide"]["properties"]["marker"] = json!({"const":marker});
    spec["components"]["schemas"]["Wide"]["required"]
        .as_array_mut()
        .unwrap()
        .push(json!("marker"));
    let mut payload: serde_json::Map<String, Value> =
        (0..1200).map(|i| (format!("f{i:03}"), json!(i))).collect();
    payload.insert("marker".into(), json!(marker));
    fs::write(
        work.0.join("payload.json"),
        serde_json::to_vec(&payload).unwrap(),
    )
    .unwrap();
    work.write(&spec);
    success(work.generate(&[]));
    let package = work.0.join("models");
    let before = contents(&package);
    let chunks = before
        .keys()
        .filter(|name| name.starts_with("_data/s_"))
        .collect::<Vec<_>>();
    assert!(!chunks.is_empty());
    assert!(
        before["models/wide.py"].len() > 160_000,
        "large model stays in its named component module"
    );
    fs::remove_file(package.join("models/wide.py")).unwrap();
    assert!(success(work.generate(&[])).contains("(1 rebuilt)"));
    assert_eq!(contents(&package), before);
    fs::write(package.join("models/wide.py"), "broken prepared data").unwrap();
    assert!(success(work.generate(&[])).contains("(1 rebuilt)"));
    assert_eq!(contents(&package), before);
    success(
        python_env::python_command()
            .arg("-c")
            .arg(
                r#"
import json, sys
from pathlib import Path
from typing import get_type_hints
sys.path.insert(0, sys.argv[1])
from models import Customer, Order, Wide
assert get_type_hints(Order)['customer'] is Customer
payload = json.loads(Path(sys.argv[1], 'payload.json').read_text())
assert Wide.from_value(payload).to_value() == payload
assert Wide.deserialize(Wide.from_value(payload).serialize()).to_value() == payload
chunks = vars(Wide)['__jsoncompat_schema__']._compressed
assert isinstance(chunks, tuple) and chunks
source = Wide.__jsoncompat_schema__
assert Wide.__jsoncompat_schema__ is source
assert payload['marker'] in json.dumps(json.loads(source)['properties']['marker'])
"#,
            )
            .arg(&work.0)
            .output()
            .unwrap(),
    );
    spec["components"]["schemas"]["Wide"] = record(5);
    work.write(&spec);
    success(work.generate(&[]));
    let after = contents(&package);
    assert!(after["models/wide.py"].len() < before["models/wide.py"].len());
    assert!(
        before
            .keys()
            .filter(|name| declaration(name))
            .all(|name| after.contains_key(name)),
        "shrinking a model must retain the declaration layout"
    );
    for removed in before.keys().filter(|name| !after.contains_key(*name)) {
        assert!(!package.join(removed).exists(), "stale artifact: {removed}");
    }
}

#[test]
fn named_model_files_handle_long_names_and_case_insensitive_filesystems() {
    let work = Work::new();
    let long_name = format!("Long{}", "Component".repeat(40));
    let spec = json!({"openapi":"3.1.0","info":{"title":"Names","version":"1"},"components":{"schemas":{
        "URL":{"type":"integer"}, "Url":{"type":"string"}, long_name.clone():{"type":"boolean"}
    }}});
    work.write(&spec);
    success(work.generate(&[]));
    let package = work.0.join("models");
    let files = contents(&package);
    let lower: std::collections::BTreeSet<_> =
        files.keys().map(|name| name.to_ascii_lowercase()).collect();
    assert_eq!(files.len(), lower.len());
    assert!(files.keys().all(|name| name.len() < 255));
    success(
        python_env::python_command()
            .arg("-c")
            .arg(
                r#"
import sys
sys.path.insert(0, sys.argv[1])
import models
assert models.URL.from_value(1).root == 1
assert models.Url.from_value('one').root == 'one'
assert getattr(models, sys.argv[2]).from_value(True).root is True
"#,
            )
            .arg(&work.0)
            .arg(long_name)
            .output()
            .unwrap(),
    );
}

#[test]
fn directed_imports_link_shared_dependencies_without_loading_reverse_users() {
    let work = Work::new();
    work.write(&json!({"openapi":"3.1.0","info":{"title":"Directed","version":"1"},"components":{"schemas":{
        "Leaf":{"type":"object","properties":{"n":{"type":"integer","minimum":2},"tag":{"type":"string","pattern":"^ok"}},"required":["n","tag"],"additionalProperties":false},
        "Left":{"type":"object","properties":{"leaf":{"$ref":"#/components/schemas/Leaf"}},"required":["leaf"],"additionalProperties":false},
        "Right":{"type":"object","properties":{"leaf":{"$ref":"#/components/schemas/Leaf"}},"required":["leaf"],"additionalProperties":false},
        "Parent":{"type":"object","properties":{"left":{"$ref":"#/components/schemas/Left"},"right":{"$ref":"#/components/schemas/Right"},"items":{"type":"array","items":{"$ref":"#/components/schemas/Leaf"}}},"required":["left","right","items"],"additionalProperties":false},
        "Unused":{"type":"object","properties":{"leaf":{"$ref":"#/components/schemas/Leaf"}},"required":["leaf"]}
    }}}));
    success(work.generate(&[]));
    success(python_env::python_command().arg("-c").arg(r#"
import builtins, compileall, dataclasses, gc, inspect, json, sys, zlib
from pathlib import Path
from typing import get_type_hints
from unittest.mock import patch
import jsoncompat
from jsoncompat.codegen import dataclasses as dc
sys.path.insert(0, sys.argv[1])
compileall.compile_dir(str(Path(sys.argv[1], 'models')), quiet=1)
def forbidden(*a, **k): raise AssertionError('runtime compilation')
with patch.object(builtins, 'compile', forbidden), patch.object(dataclasses, 'dataclass', forbidden), patch.object(inspect, 'get_annotations', forbidden), patch.object(jsoncompat, 'validator_for', forbidden), patch.object(zlib, 'decompress', forbidden):
    from models.models.leaf import Leaf
    assert 'models.models.parent' not in sys.modules
    from models.models.parent import Parent
    assert 'models.models.unused' not in sys.modules
    leaf = {'n': 3, 'tag': 'okay'}
    value = {'left': {'leaf': leaf}, 'right': {'leaf': leaf}, 'items': [leaf]}
    for parse, payload in ((Parent.from_value, value), (Parent.deserialize, json.dumps(value))):
        obj = parse(payload)
        assert type(obj.left.leaf) is type(obj.right.leaf) is type(obj.items[0]) is Leaf
        assert obj.to_value() == value and json.loads(obj.serialize()) == value
        gc.collect()
        assert obj.to_value() == value
    for invalid in ({'n': 1, 'tag': 'okay'}, {'n': 3, 'tag': 'wrong'}, {'n': True, 'tag': 'okay'}, {'n': 3, 'tag': 'okay', 'extra': 1}):
        bad = dict(value, right={'leaf': invalid})
        for parse, payload in ((Parent.from_value, bad), (Parent.deserialize, json.dumps(bad))):
            try: parse(payload)
            except (ValueError, TypeError): pass
            else: raise AssertionError(('dependency validation omitted', bad))
from models.models.left import Left
assert get_type_hints(Parent)['left'] is Left
assert get_type_hints(Left)['leaf'] is Leaf
"#).arg(&work.0).output().unwrap());
}

#[test]
fn external_component_nested_types_have_stable_ownership() {
    let work = Work::new();
    work.write(&json!({"openapi":"3.1.0","info":{"title":"External","version":"1"},"components":{"schemas":{
        "Remote":{"$ref":"https://example.test/remote"}, "Unrelated":{"type":"boolean"}
    }}}));
    fs::write(work.0.join("options.json"), json!({"resources":{"https://example.test/remote":{
        "type":"object","properties":{"child":{"type":"object","properties":{"n":{"type":"integer","minimum":0}},"required":["n"],"additionalProperties":false}},"required":["child"],"additionalProperties":false
    }}}).to_string()).unwrap();
    success(work.generate(&["--schema-options", "options.json"]));
    success(python_env::python_command().arg("-c").arg(r#"
import sys
from typing import get_type_hints
sys.path.insert(0, sys.argv[1])
from models.models.remote import Remote
value = Remote.deserialize('{"child":{"n":1}}')
assert value.to_value() == {'child': {'n': 1}}
assert get_type_hints(Remote)['root'] is type(value.root)
assert get_type_hints(type(value.root))['child'] is type(value.root.child)
assert type(value.root.child).__module__ in {'models.models.remote', 'models.models._shared'}
assert 'models.models.unrelated' not in sys.modules
try: Remote.deserialize('{"child":{"n":-1}}')
except ValueError: pass
else: raise AssertionError('external component constraint lost')
"#).arg(&work.0).output().unwrap());
}
