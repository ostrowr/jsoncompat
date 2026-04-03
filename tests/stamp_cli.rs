use serde_json::Value;
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

#[path = "support/python_env.rs"]
mod python_env;

fn temp_dir(name: &str) -> std::path::PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    let path =
        std::env::temp_dir().join(format!("jsoncompat-{name}-{}-{unique}", std::process::id()));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

#[test]
fn stamp_initializes_manifest_and_codegen_lowers_reader_schema_from_stdin() {
    let dir = temp_dir("stamp-init");
    let manifest_path = dir.join("manifest.json");
    let schema_path = dir.join("schema.json");
    fs::write(
        &schema_path,
        r#"{"type":"object","properties":{"name":{"type":"string"}},"required":["name"]}"#,
    )
    .expect("write schema");

    let stamp_output = Command::new(env!("CARGO_BIN_EXE_jsoncompat"))
        .args([
            "stamp",
            "--manifest",
            manifest_path.to_str().expect("utf-8 path"),
            "--id",
            "user-profile",
            "--write-manifest",
            "--display",
            "reader",
            "--pretty",
            schema_path.to_str().expect("utf-8 path"),
        ])
        .output()
        .expect("run jsoncompat stamp");

    assert!(
        stamp_output.status.success(),
        "stamp failed: {}",
        String::from_utf8_lossy(&stamp_output.stderr)
    );
    assert!(manifest_path.exists());

    let reader: Value =
        serde_json::from_slice(&stamp_output.stdout).expect("parse stamp reader stdout");
    assert_eq!(
        reader["x-jsoncompat"],
        serde_json::json!({
            "kind": "reader",
            "stable_id": "user-profile",
            "name": "UserProfileReader"
        })
    );
    assert_eq!(
        reader["$defs"]["v1"]["x-jsoncompat"],
        serde_json::json!({
            "kind": "declaration",
            "stable_id": "user-profile",
            "name": "UserProfileV1",
            "version": 1,
            "schema_ref": "#/$defs/v1"
        })
    );

    let mut codegen = Command::new(env!("CARGO_BIN_EXE_jsoncompat"))
        .args(["codegen", "--target", "schema", "--pretty", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn jsoncompat codegen");

    codegen
        .stdin
        .as_mut()
        .expect("stdin pipe")
        .write_all(&stamp_output.stdout)
        .expect("write reader schema to stdin");

    let codegen_output = codegen.wait_with_output().expect("wait for codegen");
    assert!(
        codegen_output.status.success(),
        "codegen failed: {}",
        String::from_utf8_lossy(&codegen_output.stderr)
    );

    let normalized: Value =
        serde_json::from_slice(&codegen_output.stdout).expect("parse normalized reader stdout");
    assert_eq!(normalized, reader);
}

#[test]
fn codegen_dataclasses_accepts_plain_schema_from_stdin() {
    let dir = temp_dir("dataclasses-plain");
    let module_path = dir.join("plain_models.py");

    let mut codegen = Command::new(env!("CARGO_BIN_EXE_jsoncompat"))
        .args(["codegen", "--target", "dataclasses", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn jsoncompat codegen");

    codegen
        .stdin
        .as_mut()
        .expect("stdin pipe")
        .write_all(
            br#"{"title":"user profile","type":"object","properties":{"name":{"type":"string","minLength":1},"age":{"type":"integer"}},"required":["name"],"additionalProperties":{"type":"string"}}"#,
        )
        .expect("write plain schema to stdin");

    let output = codegen.wait_with_output().expect("wait for codegen");
    assert!(
        output.status.success(),
        "codegen failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::write(&module_path, &output.stdout).expect("write generated module");

    let validation = python_env::python_command()
        .args([
            "-B",
            "-c",
            r#"
import importlib.util
import sys

from jsoncompat.codegen.dataclasses import JSONCOMPAT_MISSING

spec = importlib.util.spec_from_file_location("plain_models", sys.argv[1])
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = module
spec.loader.exec_module(module)

model = module.JSONCOMPAT_MODEL
user = model.from_json({"name": "Ada", "nickname": "ace"})
assert user.name == "Ada"
assert user.age is JSONCOMPAT_MISSING
assert user.__jsoncompat_extra__ == {"nickname": "ace"}
assert user.to_json() == {"name": "Ada", "nickname": "ace"}

try:
    model.from_json({"name": ""})
except ValueError:
    pass
else:
    raise AssertionError("expected invalid payload to be rejected")
"#,
        ])
        .arg(module_path.to_str().expect("utf-8 path"))
        .output()
        .expect("run generated dataclasses validation");
    assert!(
        validation.status.success(),
        "generated dataclasses validation failed: {}",
        String::from_utf8_lossy(&validation.stderr)
    );
}

#[test]
fn codegen_dataclasses_generates_directional_models_from_stamped_schemas() {
    let dir = temp_dir("dataclasses-strict");
    let manifest_path = dir.join("manifest.json");
    let schema_path = dir.join("schema.json");
    let writer_module_path = dir.join("generated_writer.py");
    let reader_module_path = dir.join("generated_reader.py");
    fs::write(
        &schema_path,
        r#"{"type":"object","properties":{"name":{"type":"string","minLength":1}},"required":["name"],"additionalProperties":false}"#,
    )
    .expect("write schema");

    let writer_schema = Command::new(env!("CARGO_BIN_EXE_jsoncompat"))
        .args([
            "stamp",
            "--manifest",
            manifest_path.to_str().expect("utf-8 path"),
            "--id",
            "user-profile",
            "--display",
            "writer",
            schema_path.to_str().expect("utf-8 path"),
        ])
        .output()
        .expect("run jsoncompat stamp writer");
    assert!(
        writer_schema.status.success(),
        "stamp writer failed: {}",
        String::from_utf8_lossy(&writer_schema.stderr)
    );

    let reader_schema = Command::new(env!("CARGO_BIN_EXE_jsoncompat"))
        .args([
            "stamp",
            "--manifest",
            manifest_path.to_str().expect("utf-8 path"),
            "--id",
            "user-profile",
            "--display",
            "reader",
            schema_path.to_str().expect("utf-8 path"),
        ])
        .output()
        .expect("run jsoncompat stamp reader");
    assert!(
        reader_schema.status.success(),
        "stamp reader failed: {}",
        String::from_utf8_lossy(&reader_schema.stderr)
    );

    let mut writer_codegen = Command::new(env!("CARGO_BIN_EXE_jsoncompat"))
        .args(["codegen", "--target", "dataclasses", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn writer codegen");
    writer_codegen
        .stdin
        .as_mut()
        .expect("stdin pipe")
        .write_all(&writer_schema.stdout)
        .expect("write writer schema to stdin");

    let writer_codegen_output = writer_codegen
        .wait_with_output()
        .expect("wait for writer codegen");
    assert!(
        writer_codegen_output.status.success(),
        "writer codegen failed: {}",
        String::from_utf8_lossy(&writer_codegen_output.stderr)
    );
    fs::write(&writer_module_path, &writer_codegen_output.stdout)
        .expect("write generated writer module");

    let mut reader_codegen = Command::new(env!("CARGO_BIN_EXE_jsoncompat"))
        .args(["codegen", "--target", "dataclasses", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn reader codegen");
    reader_codegen
        .stdin
        .as_mut()
        .expect("stdin pipe")
        .write_all(&reader_schema.stdout)
        .expect("write reader schema to stdin");

    let reader_codegen_output = reader_codegen
        .wait_with_output()
        .expect("wait for reader codegen");
    assert!(
        reader_codegen_output.status.success(),
        "reader codegen failed: {}",
        String::from_utf8_lossy(&reader_codegen_output.stderr)
    );
    fs::write(&reader_module_path, &reader_codegen_output.stdout)
        .expect("write generated reader module");

    let validation = python_env::python_command()
        .arg("-B")
        .arg("-c")
        .arg(
            r#"
import importlib.util
import sys

writer_spec = importlib.util.spec_from_file_location("generated_writer", sys.argv[1])
writer_module = importlib.util.module_from_spec(writer_spec)
assert writer_spec.loader is not None
sys.modules[writer_spec.name] = writer_module
writer_spec.loader.exec_module(writer_module)

reader_spec = importlib.util.spec_from_file_location("generated_reader", sys.argv[2])
reader_module = importlib.util.module_from_spec(reader_spec)
assert reader_spec.loader is not None
sys.modules[reader_spec.name] = reader_module
reader_spec.loader.exec_module(reader_module)

writer = writer_module.UserProfileWriter(version=1, data=writer_module.UserProfileV1(name="Ada"))
assert writer.to_json() == {"version": 1, "data": {"name": "Ada"}}

reader = reader_module.UserProfileReader.from_json({"version": 1, "data": {"name": "Ada"}})
assert reader.root.version == 1
assert reader.root.data.name == "Ada"

for forbidden in (
    lambda: writer_module.UserProfileWriter.from_json({"version": 1, "data": {"name": "Ada"}}),
    lambda: writer_module.UserProfileWriter.from_json_string('{"version":1,"data":{"name":"Ada"}}'),
    lambda: reader.root.to_json(),
    lambda: reader.to_json(),
    lambda: reader.to_json_string(),
):
    try:
        forbidden()
    except TypeError:
        pass
    else:
        raise AssertionError("directional reader/writer method guard did not fire")

for payload in (
    {"version": 1, "data": {"name": 1}},
    {"version": 1, "data": {"name": "Ada", "extra": "nope"}},
    {"version": 1, "data": {"name": ""}},
    {"data": {"name": "Ada"}},
):
    try:
        writer_module.UserProfileWriter.from_json(payload)
    except TypeError:
        pass
    else:
        raise AssertionError(f"writer deserialization should be forbidden: {payload!r}")

for factory in (
    lambda: writer_module.UserProfileWriter(version=1, data=writer_module.UserProfileV1(name="")),
    lambda: writer_module.UserProfileWriter(version=1, data=writer_module.UserProfileV1(name="Ada", __jsoncompat_extra__={"extra": "nope"})),
):
    try:
        factory()
    except (TypeError, ValueError):
        pass
    else:
        raise AssertionError("invalid writer instance should be rejected")
"#,
        )
        .arg(writer_module_path.to_str().expect("utf-8 writer path"))
        .arg(reader_module_path.to_str().expect("utf-8 reader path"))
        .output()
        .expect("run generated dataclasses validation");
    assert!(
        validation.status.success(),
        "generated dataclasses validation failed: {}",
        String::from_utf8_lossy(&validation.stderr)
    );
}
