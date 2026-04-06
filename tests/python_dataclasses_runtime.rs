use jsoncompat::{StampManifest, stamp_schema};
use jsoncompat_codegen::generate_dataclass_models;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[path = "support/python_env.rs"]
mod python_env;

#[test]
fn packaged_dataclasses_runtime_helpers_construct_validate_and_guard_directional_models() {
    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
from dataclasses import dataclass
from typing import ClassVar, Literal

from jsoncompat.codegen.dataclasses import (
    DataclassAdditionalModel,
    DataclassModel,
    DataclassRootModel,
    JSONCOMPAT_MISSING,
    ReaderDataclassModel,
    ReaderDataclassRootModel,
    WriterDataclassModel,
    jsoncompat_extra_field,
    jsoncompat_field,
    jsoncompat_root_field,
)


@dataclass(frozen=True, slots=True, kw_only=True)
class Profile(DataclassAdditionalModel[str]):
    __jsoncompat_schema__: ClassVar[str] = '{"type":"object","properties":{"name":{"type":"string"},"age":{"type":"integer"}},"required":["name"],"additionalProperties":{"type":"string"}}'

    name: str = jsoncompat_field("name")
    age: int | None = jsoncompat_field("age", omittable=True)
    __jsoncompat_extra__: dict[str, str] = jsoncompat_extra_field()


profile = Profile.from_json({"name": "Ada", "nickname": "ace"})
assert profile.name == "Ada"
assert profile.age is JSONCOMPAT_MISSING
assert profile.__jsoncompat_extra__ == {"nickname": "ace"}
assert profile.get_additional_property("nickname") == "ace"
assert profile.get_additional_property("missing") is JSONCOMPAT_MISSING
assert profile.to_json() == {"name": "Ada", "nickname": "ace"}
assert profile.to_json_string() == '{"name":"Ada","nickname":"ace"}'

profile_with_age = Profile(name="Ada", age=37, __jsoncompat_extra__={"nickname": "ace"})
assert profile_with_age.to_json() == {
    "name": "Ada",
    "age": 37,
    "nickname": "ace",
}

for factory in (
    lambda: Profile(name=1),
    lambda: Profile.from_json({"name": 1}),
    lambda: Profile.from_json({"name": "Ada", "age": "37"}),
    lambda: Profile(name="Ada", __jsoncompat_extra__={"nickname": 1}),
):
    try:
        factory()
    except (TypeError, ValueError):
        pass
    else:
        raise AssertionError("invalid dataclass payload was accepted")


@dataclass(frozen=True, slots=True, kw_only=True)
class ProfileRoot(DataclassRootModel):
    __jsoncompat_schema__: ClassVar[str] = '{"type":"string","minLength":1}'

    root: str = jsoncompat_root_field()


assert ProfileRoot.from_json("ok").root == "ok"
assert ProfileRoot(root="ok").to_json() == "ok"
try:
    ProfileRoot(root="")
except ValueError:
    pass
else:
    raise AssertionError("invalid root dataclass was accepted")


@dataclass(frozen=True, slots=True, kw_only=True)
class ProfileWriter(WriterDataclassModel):
    __jsoncompat_schema__: ClassVar[str] = '{"type":"object","properties":{"version":{"const":1},"data":{"type":"object","properties":{"name":{"type":"string"}},"required":["name"],"additionalProperties":false}},"required":["version","data"],"additionalProperties":false}'

    version: Literal[1] = jsoncompat_field("version")
    data: Profile = jsoncompat_field("data")


@dataclass(frozen=True, slots=True, kw_only=True)
class ProfileReaderV1(ReaderDataclassModel):
    __jsoncompat_schema__: ClassVar[str] = '{"type":"object","properties":{"version":{"const":1},"data":{"type":"object","properties":{"name":{"type":"string"}},"required":["name"],"additionalProperties":{"type":"string"}}},"required":["version","data"],"additionalProperties":false}'

    version: Literal[1] = jsoncompat_field("version")
    data: Profile = jsoncompat_field("data")


@dataclass(frozen=True, slots=True, kw_only=True)
class ProfileReader(ReaderDataclassRootModel):
    __jsoncompat_schema__: ClassVar[str] = '{"oneOf":[{"type":"object","properties":{"version":{"const":1},"data":{"type":"object","properties":{"name":{"type":"string"}},"required":["name"],"additionalProperties":{"type":"string"}}},"required":["version","data"],"additionalProperties":false}]}'

    root: ProfileReaderV1 = jsoncompat_root_field()


writer = ProfileWriter(version=1, data=Profile(name="Ada"))
assert writer.to_json() == {"version": 1, "data": {"name": "Ada"}}
reader = ProfileReader.from_json({"version": 1, "data": {"name": "Ada"}})
assert reader.root.version == 1
assert reader.root.data.name == "Ada"

for forbidden in (
    lambda: ProfileWriter.from_json({"version": 1, "data": {"name": "Ada"}}),
    lambda: ProfileWriter.from_json_string('{"version":1,"data":{"name":"Ada"}}'),
    lambda: ProfileReaderV1(version=1, data=Profile(name="Ada")).to_json(),
    lambda: reader.to_json(),
    lambda: reader.to_json_string(),
):
    try:
        forbidden()
    except TypeError:
        pass
    else:
        raise AssertionError("directional dataclass guard did not fire")
"###,
    );
    let output = command.output().expect("run dataclass helper module test");
    assert!(
        output.status.success(),
        "dataclass helper module test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn generated_dataclasses_reject_invalid_json_values_for_plain_schemas() {
    let source = generate_dataclass_models(&json!({
        "title": "InventoryItem",
        "type": "object",
        "properties": {
            "sku": {"type": "string", "minLength": 1},
            "quantity": {"type": "integer", "minimum": 0},
            "tags": {"type": "array", "items": {"type": "string"}},
        },
        "required": ["sku", "quantity"],
        "additionalProperties": false,
    }))
    .expect("generate dataclasses from plain schema");
    let module_path = write_temp_module("invalid_payloads", &source);

    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
import importlib.util
import sys

module_path = sys.argv[1]
spec = importlib.util.spec_from_file_location("generated_models", module_path)
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = module
spec.loader.exec_module(module)

model = module.InventoryItem
valid = model.from_json({"sku": "abc", "quantity": 3, "tags": ["new"]})
assert valid.to_json() == {"sku": "abc", "quantity": 3, "tags": ["new"]}

invalid_values = [
    {"sku": "", "quantity": 3},
    {"sku": "abc", "quantity": -1},
    {"sku": "abc", "quantity": 3, "tags": ["new", 1]},
    {"sku": "abc", "quantity": 3, "extra": "nope"},
    "not-an-object",
]

for value in invalid_values:
    try:
        model.from_json(value)
    except (TypeError, ValueError):
        pass
    else:
        raise AssertionError(f"invalid payload was accepted: {value!r}")
"###,
    );
    command.arg(module_path);
    let output = command
        .output()
        .expect("run generated dataclass invalid payload test");
    assert!(
        output.status.success(),
        "generated dataclass invalid payload test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn generated_dataclasses_use_root_defs_for_stamped_payload_ref_collisions() {
    let stamped = stamp_schema(
        &StampManifest::empty(),
        "collision",
        json!({
            "type": "object",
            "properties": {
                "name": { "$ref": "#/$defs/v1" }
            },
            "required": ["name"],
            "additionalProperties": false,
            "$defs": {
                "v1": { "type": "string" }
            }
        }),
    )
    .expect("stamp schema with colliding payload $defs");
    let source = generate_dataclass_models(&stamped.bundle.writer)
        .expect("generate dataclasses from stamped collision schema");
    let module_path = write_temp_module("stamped_defs_collision", &source);

    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
import importlib.util
import sys

module_path = sys.argv[1]
spec = importlib.util.spec_from_file_location("collision_models", module_path)
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = module
spec.loader.exec_module(module)

payload = module.CollisionV1(name="Ada")
assert payload.to_json() == {"name": "Ada"}

writer = module.CollisionWriter(version=1, data=payload)
assert writer.to_json() == {"version": 1, "data": {"name": "Ada"}}
"###,
    );
    command.arg(module_path);
    let output = command
        .output()
        .expect("run generated dataclass stamped collision test");
    assert!(
        output.status.success(),
        "generated dataclass stamped collision test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write_temp_module(test_name: &str, source: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "jsoncompat-dataclass-runtime-{test_name}-{}-{unique}",
        std::process::id(),
    ));
    fs::create_dir_all(&dir).expect("create temporary test module directory");
    let module_path = dir.join("generated_models.py");
    fs::write(&module_path, source).expect("write temporary test module");
    module_path
}
