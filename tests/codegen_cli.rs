use serde_json::json;
use std::{fs, process::Command};

#[path = "support/python_env.rs"]
mod python_env;

#[test]
fn split_codegen_uses_stable_names_and_imports_before_models() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!("jsoncompat-split-{unique}"));
    let package = directory.join("models_package");
    fs::create_dir_all(&package).unwrap();
    fs::write(package.join("__init__.py"), "").unwrap();
    let schema = directory.join("schema.json");
    fs::write(
        &schema,
        json!({
            "title":"Node", "type":"object", "properties": {
                "value":{"type":"integer","minimum":0},
                "label":{"type":"string"},
                "children":{"type":"array","items":{"$ref":"#"}}
            }, "required":["value","children"], "additionalProperties": false
        })
        .to_string(),
    )
    .unwrap();
    let output = package.join("models.py");
    let generate = |path: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_jsoncompat"))
            .args(["codegen", "--target", "dataclasses", "--output"])
            .arg(path)
            .arg(&schema)
            .output()
            .unwrap()
    };
    let generated = generate(&output);
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    assert!(generated.stdout.is_empty());
    let source = fs::read_to_string(&output).unwrap();
    assert!(source.find("import bind_models").unwrap() < source.find("class Node(").unwrap());
    assert!(source.find("class Node(").unwrap() < source.find("\n_jsoncompat_bind(").unwrap());
    assert!(source.contains("from ._models_generated import bind_models"));
    let companion = package.join("_models_generated.py");
    let implementation = fs::read_to_string(&companion).unwrap();
    assert!(!source.contains("dc.install_model"));
    assert!(!source.contains("def _jsoncompat_init"));
    assert!(source.len() < 2000);
    assert!(generate(&output).status.success());
    assert_eq!(source, fs::read_to_string(&output).unwrap());
    assert_eq!(fs::read_dir(&package).unwrap().count(), 3);

    // Package-relative and top-level imports share one companion but must keep
    // distinct constructor namespaces, including self-referential annotations.
    let checked = python_env::python_command()
        .arg("-c")
        .arg(
            r#"
import dataclasses, inspect, json, pickle, sys, typing
sys.path.insert(0, sys.argv[1])
from models_package import models
sys.path.insert(0, sys.argv[1] + '/models_package')
import models as other
for module in (models, other):
    Node = module.Node
    instance = Node(value=1, children=[Node(value=2, children=[])])
    assert dataclasses.is_dataclass(instance)
    assert pickle.loads(pickle.dumps(instance)) == instance
    assert json.loads(instance.serialize()) == {'value':1,'children':[{'value':2,'children':[]}]}
    assert typing.get_args(typing.get_type_hints(Node.__init__)['children']) == (Node,)
    assert inspect.signature(Node).parameters['label'].default is module.dc.JSONCOMPAT_MISSING
    assert dataclasses.replace(instance, value=3).value == 3
assert models.Node.__init__ is not other.Node.__init__
"#,
        )
        .arg(&directory)
        .output()
        .unwrap();
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );

    // Schema changes replace the same companion without changing its import.
    let mut revised: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&schema).unwrap()).unwrap();
    revised["properties"]["value"]["minimum"] = json!(10);
    fs::write(&schema, revised.to_string()).unwrap();
    assert!(generate(&output).status.success());
    assert_eq!(source, fs::read_to_string(&output).unwrap());
    let revised_implementation = fs::read_to_string(&companion).unwrap();
    assert_ne!(implementation, revised_implementation);
    let companions = fs::read_dir(&package)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("_models_generated")
        })
        .count();
    assert_eq!(companions, 1);
    let checked = python_env::python_command()
        .arg("-c")
        .arg(
            r#"
import sys
sys.path.insert(0, sys.argv[1])
from models_package.models import Node
assert Node(value=10, children=[]).serialize() == '{"children":[],"value":10}'
try:
    Node(value=1, children=[])
except ValueError:
    pass
else:
    raise AssertionError("regenerated models retained the old constraint")
"#,
        )
        .arg(&directory)
        .output()
        .unwrap();
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );

    assert!(!generate(&package.join("123invalid.py")).status.success());
    // Unsupported input must fail before changing either published artifact.
    fs::write(&schema, r##"{"$dynamicRef":"#node"}"##).unwrap();
    assert!(!generate(&output).status.success());
    assert_eq!(source, fs::read_to_string(&output).unwrap());
    assert_eq!(
        revised_implementation,
        fs::read_to_string(&companion).unwrap()
    );
    fs::remove_dir_all(directory).unwrap();
}
