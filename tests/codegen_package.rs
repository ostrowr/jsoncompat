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
                "2",
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
    fs::read_dir(directory)
        .unwrap()
        .filter_map(|entry| {
            let path = entry.unwrap().path();
            (path
                .extension()
                .is_some_and(|ext| ext == "py" || ext == "pyi"))
            .then(|| {
                (
                    path.file_name().unwrap().to_string_lossy().into_owned(),
                    fs::read(path).unwrap(),
                )
            })
        })
        .collect()
}
#[test]
fn package_builds_preserve_identity_recursion_laziness_and_incremental_outputs() {
    let work = Work::new();
    let mut input = api();
    work.write(&input);
    assert!(success(work.generate(&[])).contains("5 models in 3 modules (3 rebuilt)"));
    let before = contents(&work.0.join("models"));
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
    assert not any(name.startswith('models.models_') for name in sys.modules)
    from models import Order, Customer
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
        before.keys().collect::<Vec<_>>(),
        after.keys().collect::<Vec<_>>(),
        "a constraint edit must not repack neighboring models"
    );
    let changed = before
        .iter()
        .filter(|(name, data)| after.get(*name) != Some(*data))
        .map(|(name, _)| name)
        .collect::<Vec<_>>();
    assert!(!changed.is_empty());
    assert!(
        changed.iter().all(|name| name.contains("_schema_Wide_")
            || name.contains("_program_Wide_")
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
    assert!(!work.0.join("models/models_0002.py").exists());
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
        .filter(|name| name.contains("_program_Wide_"))
        .collect::<Vec<_>>();
    assert!(
        chunks.len() > 1,
        "a single large prepared model must span data files"
    );
    for name in &chunks {
        assert!(before[*name].len() < 4 * 64 * 1024 + 128, "{name}");
    }
    fs::remove_file(package.join(chunks[0])).unwrap();
    assert!(success(work.generate(&[])).contains("(1 rebuilt)"));
    assert_eq!(contents(&package), before);
    fs::write(package.join(chunks[1]), "broken prepared data").unwrap();
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
    assert!(chunks.iter().any(|name| !after.contains_key(*name)));
    assert!(
        before
            .keys()
            .filter(|name| name.contains("_type_"))
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
