//! Wire keys must not become Python mangled names or replace special methods.
use serde_json::{Value, json};
use std::{fs, path::PathBuf, process::Command};
#[path = "support/python_env.rs"]
mod python_env;

struct Work(PathBuf);
impl Work {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "jsoncompat-private-fields-{}-{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        fs::create_dir(&path).unwrap();
        Self(path.canonicalize().unwrap())
    }
}
impl Drop for Work {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn success(output: std::process::Output) {
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn private_and_special_wire_keys_preserve_runtime_and_static_interfaces() {
    let work = Work::new();
    let names = [
        "__type",
        "__type_",
        "__type-",
        "field___type",
        "field___type2",
        "__init__",
        "__new__",
        "__slots__",
        "__dict__",
        "__weakref__",
        "__class__",
        "__annotations__",
        "__module__",
        "__getattribute__",
        "__setattr__",
        "__jsoncompat_extra__",
    ];
    let properties: serde_json::Map<String, Value> = names
        .iter()
        .map(|name| ((*name).into(), json!({"type":"string"})))
        .collect();
    let schema = json!({"type":"object", "properties":properties, "required":names, "additionalProperties":false});
    let raw = json!({"openapi":"3.1.0", "info":{"title":"Private fields", "version":"1"},
        "components":{"schemas":{"Pointer":schema}},
        "paths":{"/echo":{"post":{"operationId":"echo", "parameters":[
            {"name":"__type", "in":"query", "required":true, "schema":{"type":"string"}},
            {"name":"__init__", "in":"query", "required":true, "schema":{"type":"string"}}
        ], "requestBody":{"required":true,"content":{"application/json":{"schema":{"$ref":"#/components/schemas/Pointer"}}}},
        "responses":{"200":{"description":"Echo", "headers":{"__slots__":{"required":true,"schema":{"type":"string"}}}, "content":{"application/json":{"schema":{"$ref":"#/components/schemas/Pointer"}}}}}}}}
    });
    fs::write(work.0.join("source.json"), raw.to_string()).unwrap();
    success(
        Command::new(env!("CARGO_BIN_EXE_jsoncompat"))
            .current_dir(&work.0)
            .args([
                "codegen",
                "--openapi",
                "source.json",
                "--api",
                "PrivateFields",
                "--output",
                "private_fields",
            ])
            .output()
            .unwrap(),
    );
    let values: serde_json::Map<String, Value> = names
        .iter()
        .enumerate()
        .map(|(index, name)| ((*name).into(), json!(format!("value{index}"))))
        .collect();
    fs::write(work.0.join("value.json"), json!(values).to_string()).unwrap();
    fs::write(work.0.join("check.py"), r#"
import json
from pathlib import Path
from typing import assert_type
from urllib.parse import parse_qs, urlsplit
from private_fields import PrivateFieldsAPI
from private_fields.models.pointer import Pointer
from private_fields.operations.echo import EchoRequest, Echo200
from jsoncompat.http import EncodedRequest, RawResponse, bind
from jsoncompat.http._binding import _prepare

wire = json.loads(Path('value.json').read_text())
model = Pointer.from_value(wire)
assert model.to_value() == wire
assert Pointer.deserialize(model.serialize()).to_value() == wire
assert_type(model.field___type, str)
assert_type(model.field___init__, str)
assert_type(model.field___slots__, str)
assert model.field___type == wire['__type']
# The mangled spelling and the original special methods remain untouched.
assert not hasattr(model, '_Pointer__type')
assert callable(model.__init__)
assert model.__class__ is Pointer
assert len(Pointer.__slots__) == len(set(Pointer.__slots__))
request = EchoRequest(type='query', init='constructor', body=model)
assert_type(request.type, str)
operation, = PrivateFieldsAPI.operations()
plans = PrivateFieldsAPI.__jsoncompat_http_plans__
assert plans is not None
assert plans[EchoRequest] == _prepare(operation, PrivateFieldsAPI)
def fetch(request: EncodedRequest, /, *, transport: None = None) -> RawResponse:
    assert parse_qs(urlsplit(request.target).query) == {'__type': ['query'], '__init__': ['constructor']}
    assert request.body is not None
    assert json.loads(request.body) == wire
    return RawResponse(200, (('Content-Type', 'application/json'), ('__slots__', 'header')), json.dumps(wire).encode())
response = bind(PrivateFieldsAPI, fetch=fetch).request(request).unwrap()
assert_type(response, Echo200)
assert_type(response.headers.slots, str)
assert response.headers.slots == 'header'
assert response.body.to_value() == wire
"#).unwrap();
    success(
        python_env::python_command()
            .current_dir(&work.0)
            .arg("check.py")
            .output()
            .unwrap(),
    );
    success(
        python_env::ty_command()
            .current_dir(&work.0)
            .args(["private_fields", "check.py"])
            .output()
            .unwrap(),
    );
}
