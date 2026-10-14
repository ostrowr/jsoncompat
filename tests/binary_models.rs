//! Lossless Python bytes inside HTTP models, with a JSON-valued base64 projection.
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
};
struct Work(PathBuf);
impl Work {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "jsoncompat-binary-{}-{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for Work {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[path = "support/python_env.rs"]
mod python_env;

#[test]
fn opaque_binary_models_preserve_bytes_and_json_roundtrips() {
    let directory = Work::new();
    let binary = json!({"type":"string", "x-jsoncompat-http-binary":true});
    let schema = json!({
        "type":"object",
        "properties":{
            "body":binary, "optional":binary,
            "items":{"type":"array", "items":binary},
            "union":{"anyOf":[binary,{"type":"null"}]},
            "label":{"type":"string","minLength":2}
        },
        "required":["body","items","union","label"],
        "additionalProperties":false
    });
    let source = jsoncompat_codegen::generate_dataclass_models(&schema).unwrap();
    assert!(source.contains("body: builtins.bytes"));
    assert!(source.contains("Sequence[builtins.bytes]"));
    fs::write(directory.path().join("binary_models.py"), source).unwrap();
    fs::write(
        directory.path().join("root_binary.py"),
        jsoncompat_codegen::generate_dataclass_models(&binary).unwrap(),
    )
    .unwrap();
    let script = r#"
import base64
import json
from binary_models import JSONCOMPAT_MODEL as Model
from root_binary import JSONCOMPAT_MODEL as Root
from jsoncompat import MISSING

def rejects(call):
    try:
        call()
    except (ValueError, TypeError):
        return
    raise AssertionError('invalid value accepted')

payload = bytes(range(256)) + b'\x00\xff\r\n'
kwargs = dict(body=payload, items=[payload, b''], union=payload, label='ok')
model = Model(**kwargs)
assert type(model.body) is bytes and type(model.items[0]) is bytes
assert model.optional is MISSING
value = model.to_value()
assert value['body'] == base64.b64encode(payload).decode('ascii')
assert value['items'][1] == ''
assert Model.from_value(value) == model
assert Model.deserialize(model.serialize()) == model
assert Model.deserialize(model.serialize().encode()) == model
for skip in (False, True):
    assert Model(**kwargs, skip_validation=skip) == model
    assert Model.from_value(value, skip_validation=skip) == model
    assert Model.deserialize(model.serialize(), skip_validation=skip) == model
    assert model.to_value(skip_validation=skip) == value
    assert json.loads(model.serialize(skip_validation=skip)) == value
assert Model(body=b'', items=[], union=None, label='ok').union is None
assert Root(root=payload).root == payload
assert Root.from_value(base64.b64encode(payload).decode()).root == payload
assert Root.deserialize(Root(root=payload).serialize()).root == payload
for bad in ('abc', 12, bytearray(payload), None):
    rejects(lambda: Model(**(kwargs | {'body':bad})))
    rejects(lambda: Root(root=bad))
for bad in ('@@@@', 'YQ', 'YR==', 'YQ===', 'YQ==\n', 'é'):
    rejects(lambda: Model.from_value(value | {'body':bad}))
    rejects(lambda: Model.deserialize(json.dumps(value | {'body':bad})))
    rejects(lambda: Root.from_value(bad))
    rejects(lambda: Root.deserialize(json.dumps(bad)))
rejects(lambda: Model(**(kwargs | {'label':'x'})))
rejects(lambda: Model.from_value(value | {'label':'x'}))
# A trusted construction still cannot bypass validation on later serialization.
invalid = Model(**(kwargs | {'label':'x'}), skip_validation=True)
rejects(invalid.to_value)
rejects(invalid.serialize)
# Ordinary JSON values remain JSON-only. The bytes mapping is local to fields
# explicitly selected as opaque HTTP binary by the generator.
from jsoncompat import serialize_json_value
rejects(lambda: serialize_json_value(payload))
"#;
    let output = python_env::python_command()
        .args(["-c", script])
        .current_dir(directory.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    fs::write(
        directory.path().join("consumer.py"),
        r#"
from binary_models import GeneratedSchema
from collections.abc import Sequence
model = GeneratedSchema(body=b'abc', items=[b''], union=None, label='ok')
a: bytes = model.body
b: Sequence[bytes] = model.items
c: bytes | None = model.union
"#,
    )
    .unwrap();
    let directory_path = directory.path().canonicalize().unwrap();
    let output = python_env::ty_command()
        .arg("--extra-search-path")
        .arg(&directory_path)
        .arg(directory_path.join("consumer.py"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    fs::write(directory.path().join("bad_consumer.py"), "from binary_models import GeneratedSchema\nGeneratedSchema(body='abc', items=[], union=None, label='ok')\n").unwrap();
    let output = python_env::ty_command()
        .arg("--extra-search-path")
        .arg(&directory_path)
        .arg(directory_path.join("bad_consumer.py"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("invalid-argument-type"));
}

#[test]
fn binary_projection_does_not_reinterpret_string_constraints() {
    for (keyword, assertion) in [
        ("minLength", json!(1)),
        ("pattern", json!("a")),
        ("maxLength", json!(2)),
        ("enum", json!(["a"])),
    ] {
        let mut schema = json!({"type":"string", "x-jsoncompat-http-binary":true});
        schema[keyword] = assertion;
        assert!(
            jsoncompat_codegen::generate_dataclass_models(&schema).is_err(),
            "{schema}"
        );
    }
}
