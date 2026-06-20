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
import typing
from typing import ClassVar, Literal

from jsoncompat.codegen import SerializationFormat
from jsoncompat.codegen import dataclasses as dc
from jsoncompat.codegen.dataclasses import (
    DataclassAdditionalModel,
    DataclassModel,
    DataclassRootModel,
    JSONCOMPAT_MISSING,
    ReaderDataclassModel,
    ReaderDataclassRootModel,
    WriterDataclassModel,
    extra_field,
    field,
    root_field,
)


@dataclass(frozen=True, slots=True, kw_only=True)
class Profile(DataclassAdditionalModel[str]):
    __jsoncompat_schema__: ClassVar[str] = '{"type":"object","properties":{"name":{"type":"string","minLength":1},"age":{"type":"integer"}},"required":["name"],"additionalProperties":{"type":"string"}}'

    name: str = field("name")
    age: int | None = field("age", omittable=True)
    __jsoncompat_extra__: dict[str, str] = extra_field()


profile_hints = typing.get_type_hints(Profile)
assert profile_hints["__jsoncompat_extra__"] == dict[str, str]
assert not hasattr(Profile, "from_json")
assert not hasattr(Profile, "from_json_string")
assert not hasattr(Profile, "to_json")
assert not hasattr(Profile, "to_json_string")

profile = Profile.from_value({"name": "Ada", "nickname": "ace"})
assert profile.name == "Ada"
assert profile.age is JSONCOMPAT_MISSING
assert profile.__jsoncompat_extra__ == {"nickname": "ace"}
assert profile.get_additional_property("nickname") == "ace"
assert profile.get_additional_property("missing") is JSONCOMPAT_MISSING
assert profile.to_value() == {"name": "Ada", "nickname": "ace"}
assert profile.serialize() == '{"name":"Ada","nickname":"ace"}'

for format in SerializationFormat:
    encoded = profile.serialize(format=format)
    decoded = Profile.deserialize(encoded, format=format)
    assert decoded.to_value() == {"name": "Ada", "nickname": "ace"}

profile_with_age = Profile(name="Ada", age=37, __jsoncompat_extra__={"nickname": "ace"})
assert profile_with_age.to_value() == {
    "name": "Ada",
    "age": 37,
    "nickname": "ace",
}

for factory in (
    lambda: Profile(name=1),
    lambda: Profile.from_value({"name": 1}),
    lambda: Profile.from_value({"name": "Ada", "age": "37"}),
    lambda: Profile(name="Ada", __jsoncompat_extra__={"nickname": 1}),
):
    try:
        factory()
    except (TypeError, ValueError):
        pass
    else:
        raise AssertionError("invalid dataclass payload was accepted")


@dataclass(frozen=True, slots=True, kw_only=True)
class AuditContext(DataclassModel):
    __jsoncompat_schema__: ClassVar[str] = '{"type":"object","properties":{"tags":{"type":"object","additionalProperties":{"type":"string"}}},"required":["tags"],"additionalProperties":false}'

    tags: dict[str, str] = field("tags")


context = AuditContext(tags={"team": "schema"})
assert context.to_value() == {"tags": {"team": "schema"}}
assert AuditContext.from_value({"tags": {"team": "schema"}}).tags == {
    "team": "schema"
}

for factory in (
    lambda: AuditContext(tags="oops"),
    lambda: AuditContext(tags={1: "schema"}),
    lambda: AuditContext(tags={"team": 1}),
):
    try:
        factory()
    except (TypeError, ValueError):
        pass
    else:
        raise AssertionError("mapping annotations accepted an invalid value")


@dataclass(frozen=True, slots=True, kw_only=True)
class UnsupportedRuntimeAnnotation(DataclassModel):
    __jsoncompat_schema__: ClassVar[str] = '{"type":"object","properties":{"tags":{"type":"array","items":{"type":"string"}}},"required":["tags"],"additionalProperties":false}'

    tags: tuple[str, ...] = field("tags")


try:
    UnsupportedRuntimeAnnotation(tags=("schema",))
except TypeError as error:
    assert "unsupported runtime annotation" in str(error)
else:
    raise AssertionError("unsupported runtime annotations must fail loudly")


@dataclass(frozen=True, slots=True, kw_only=True)
class ProfileRoot(DataclassRootModel):
    __jsoncompat_schema__: ClassVar[str] = '{"type":"string","minLength":1}'

    root: str = root_field()


assert ProfileRoot.from_value("ok").root == "ok"
assert ProfileRoot(root="ok").to_value() == "ok"

trusted_root = ProfileRoot(root="", skip_validation=True)
assert trusted_root.to_value(skip_validation=True) == ""
assert ProfileRoot.from_value("", skip_validation=True).root == ""
assert ProfileRoot.deserialize('""', skip_validation=True).root == ""

try:
    trusted_root.to_value()
except ValueError:
    pass
else:
    raise AssertionError("checked serialization accepted a trusted invalid model")

try:
    ProfileRoot(root="")
except ValueError:
    pass
else:
    raise AssertionError("invalid root dataclass was accepted")


@dataclass(frozen=True, slots=True, kw_only=True)
class ProfileWriter(WriterDataclassModel):
    __jsoncompat_schema__: ClassVar[str] = '{"type":"object","properties":{"version":{"const":1},"data":{"type":"object","properties":{"name":{"type":"string","minLength":1}},"required":["name"],"additionalProperties":false}},"required":["version","data"],"additionalProperties":false}'

    version: Literal[1] = field("version")
    data: Profile = field("data")


@dataclass(frozen=True, slots=True, kw_only=True)
class ProfileReaderV1(ReaderDataclassModel):
    __jsoncompat_schema__: ClassVar[str] = '{"type":"object","properties":{"version":{"const":1},"data":{"type":"object","properties":{"name":{"type":"string","minLength":1}},"required":["name"],"additionalProperties":{"type":"string"}}},"required":["version","data"],"additionalProperties":false}'

    version: Literal[1] = field("version")
    data: Profile = field("data")


@dataclass(frozen=True, slots=True, kw_only=True)
class ProfileReaderV2(ReaderDataclassModel):
    __jsoncompat_schema__: ClassVar[str] = '{"type":"object","properties":{"version":{"const":2},"data":{"type":"object","properties":{"name":{"type":"string","minLength":1}},"required":["name"],"additionalProperties":{"type":"string"}}},"required":["version","data"],"additionalProperties":false}'

    version: Literal[2] = field("version")
    data: Profile = field("data")


@dataclass(frozen=True, slots=True, kw_only=True)
class ProfileReader(ReaderDataclassRootModel):
    __jsoncompat_schema__: ClassVar[str] = '{"oneOf":[{"type":"object","properties":{"version":{"const":1},"data":{"type":"object","properties":{"name":{"type":"string","minLength":1}},"required":["name"],"additionalProperties":{"type":"string"}}},"required":["version","data"],"additionalProperties":false},{"type":"object","properties":{"version":{"const":2},"data":{"type":"object","properties":{"name":{"type":"string","minLength":1}},"required":["name"],"additionalProperties":{"type":"string"}}},"required":["version","data"],"additionalProperties":false}]}'

    root: ProfileReaderV1 | ProfileReaderV2 = root_field()


writer = ProfileWriter(version=1, data=Profile(name="Ada"))
assert writer.to_value() == {"version": 1, "data": {"name": "Ada"}}
reader = ProfileReader.from_value({"version": 1, "data": {"name": "Ada"}})
assert reader.root.version == 1
assert reader.root.data.name == "Ada"

try:
    ProfileWriter(
        version=True,
        data=Profile(name="Ada"),
        skip_validation=True,
    )
except TypeError:
    pass
else:
    raise AssertionError("trusted construction conflated boolean and integer literals")

trusted_reader = ProfileReader.from_value(
    {"version": 1, "data": {"name": ""}},
    skip_validation=True,
)
assert isinstance(trusted_reader.root, ProfileReaderV1)
assert trusted_reader.root.data.name == ""


@dataclass(frozen=True, slots=True, kw_only=True)
class RecursiveNode(DataclassModel):
    __jsoncompat_schema__: ClassVar[str] = '{"$defs":{"node":{"type":"object","properties":{"value":{"type":"integer"},"next":{"anyOf":[{"$ref":"#/$defs/node"},{"type":"null"}]}},"required":["value","next"],"additionalProperties":false}},"$ref":"#/$defs/node"}'

    value: int = field("value")
    next: "RecursiveNode | None" = field("next")


recursive_value = None
for index in range(50):
    recursive_value = {"value": index, "next": recursive_value}

validator_calls = []
original_validator_for = dc._jsoncompat_validator_for


def tracking_validator_for(model_type):
    validator_calls.append(model_type)
    return original_validator_for(model_type)


dc._jsoncompat_validator_for = tracking_validator_for
try:
    checked_recursive = RecursiveNode.from_value(recursive_value)
    assert validator_calls == [RecursiveNode]

    validator_calls.clear()
    trusted_recursive = RecursiveNode.from_value(
        recursive_value,
        skip_validation=True,
    )
    assert validator_calls == []
finally:
    dc._jsoncompat_validator_for = original_validator_for

assert checked_recursive.to_value() == recursive_value
assert trusted_recursive.to_value(skip_validation=True) == recursive_value

try:
    RecursiveNode.from_value(
        {"value": 0, "next": "not-a-node"},
        skip_validation=True,
    )
except TypeError:
    pass
else:
    raise AssertionError("trusted union construction accepted an invalid shape")

try:
    ProfileReader.from_value({"version": 1, "data": {"name": ""}})
except ValueError:
    pass
else:
    raise AssertionError("checked discriminated reader accepted invalid data")

try:
    ProfileWriter(version=1, data={"name": "Ada"})
except TypeError:
    pass
else:
    raise AssertionError("constructor accepted raw nested JSON instead of a Profile")

for forbidden in (
    lambda: ProfileWriter.from_value({"version": 1, "data": {"name": "Ada"}}),
    lambda: ProfileWriter.deserialize('{"version":1,"data":{"name":"Ada"}}'),
    lambda: ProfileReaderV1(version=1, data=Profile(name="Ada")).to_value(),
    lambda: reader.to_value(),
    lambda: reader.serialize(),
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
fn serialization_formats_reject_values_outside_the_json_data_model() {
    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
import msgpack

from jsoncompat.codegen import SerializationFormat
from jsoncompat.codegen.serialization import deserialize_value, serialize_value


value = {"name": "Ada", "scores": [1, 2, 3]}
for format in SerializationFormat:
    encoded = serialize_value(value, format=format)
    assert deserialize_value(encoded, format=format) == value

cyclic = []
cyclic.append(cyclic)

invalid_inputs = (
    (ValueError, lambda: deserialize_value('{"name":"Ada","name":"Grace"}')),
    (ValueError, lambda: deserialize_value('{"score":NaN}')),
    (ValueError, lambda: deserialize_value('{"score":1e999}')),
    (TypeError, lambda: serialize_value({1: "not-json"})),
    (ValueError, lambda: serialize_value({"score": float("inf")})),
    (ValueError, lambda: serialize_value(cyclic)),
    (
        ValueError,
        lambda: deserialize_value(
            "name: Ada\nname: Grace\n",
            format=SerializationFormat.YAML,
        ),
    ),
    (
        TypeError,
        lambda: deserialize_value(
            "when: 2026-06-20\n",
            format=SerializationFormat.YAML,
        ),
    ),
    (
        TypeError,
        lambda: deserialize_value(
            msgpack.packb({"value": b"not-json"}, use_bin_type=True),
            format=SerializationFormat.MSGPACK,
        ),
    ),
    (
        ValueError,
        lambda: deserialize_value(
            msgpack.packb(
                {"value": msgpack.ExtType(1, b"not-json")},
                use_bin_type=True,
            ),
            format=SerializationFormat.MSGPACK,
        ),
    ),
)

for error_type, callback in invalid_inputs:
    try:
        callback()
    except error_type:
        pass
    else:
        raise AssertionError(f"{callback!r} accepted a non-JSON value")
"###,
    );
    let output = command.output().expect("run serialization format test");
    assert!(
        output.status.success(),
        "serialization format test failed: {}",
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
valid = model.from_value({"sku": "abc", "quantity": 3, "tags": ["new"]})
assert valid.to_value() == {"sku": "abc", "quantity": 3, "tags": ["new"]}

invalid_values = [
    {"sku": "", "quantity": 3},
    {"sku": "abc", "quantity": -1},
    {"sku": "abc", "quantity": 3, "tags": ["new", 1]},
    {"sku": "abc", "quantity": 3, "extra": "nope"},
    "not-an-object",
]

for value in invalid_values:
    try:
        model.from_value(value)
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
fn generated_dataclasses_normalize_integer_valued_json_numbers_into_python_ints() {
    let source = generate_dataclass_models(&json!({
        "title": "Counter",
        "type": "object",
        "properties": {
            "count": {"type": "integer"},
        },
        "required": ["count"],
        "additionalProperties": false,
    }))
    .expect("generate dataclasses from integer schema");
    let module_path = write_temp_module("integer_normalization", &source);

    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
import importlib.util
import sys

module_path = sys.argv[1]
spec = importlib.util.spec_from_file_location("integer_models", module_path)
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = module
spec.loader.exec_module(module)

counter = module.Counter.from_value({"count": 1.0})
assert counter.count == 1
assert isinstance(counter.count, int)
assert counter.to_value() == {"count": 1}

try:
    module.Counter(count=1.0)
except TypeError:
    pass
else:
    raise AssertionError("direct constructors must keep Python int fields strict")
"###,
    );
    command.arg(module_path);
    let output = command
        .output()
        .expect("run generated dataclass integer normalization test");
    assert!(
        output.status.success(),
        "generated dataclass integer normalization test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn generated_dataclasses_follow_legacy_definition_ref_chains() {
    let source = generate_dataclass_models(&json!({
        "definitions": {
            "A": {"$ref": "#/definitions/B"},
            "B": {"type": "string"},
        },
        "$ref": "#/definitions/A",
    }))
    .expect("generate dataclasses from legacy definitions refs");
    let module_path = write_temp_module("legacy_definition_refs", &source);

    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
import importlib.util
import sys

module_path = sys.argv[1]
spec = importlib.util.spec_from_file_location("legacy_definition_models", module_path)
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = module
spec.loader.exec_module(module)

value = module.JSONCOMPAT_MODEL.from_value("Ada")
assert value.to_value() == "Ada"

try:
    module.JSONCOMPAT_MODEL.from_value(42)
except ValueError:
    pass
else:
    raise AssertionError("legacy definition ref models accepted an invalid payload")
"###,
    );
    command.arg(module_path);
    let output = command
        .output()
        .expect("run generated dataclass legacy definitions ref test");
    assert!(
        output.status.success(),
        "generated dataclass legacy definitions ref test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn generated_dataclasses_keep_conditionally_evaluated_object_properties_constructible() {
    let source = generate_dataclass_models(&json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "if": {
            "patternProperties": {
                "foo": {"type": "string"},
            },
        },
        "unevaluatedProperties": false,
    }))
    .expect("generate dataclasses from conditional unevaluatedProperties schema");
    let module_path = write_temp_module("conditional_unevaluated_properties", &source);

    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
import importlib.util
import sys

module_path = sys.argv[1]
spec = importlib.util.spec_from_file_location("conditional_models", module_path)
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = module
spec.loader.exec_module(module)

model = module.GeneratedSchema.from_value({"foo": "a"})
assert model.to_value() == {"foo": "a"}

try:
    module.GeneratedSchema.from_value({"bar": "a"})
except ValueError:
    pass
else:
    raise AssertionError("unevaluatedProperties still needs to reject unmatched keys")
"###,
    );
    command.arg(module_path);
    let output = command
        .output()
        .expect("run generated dataclass conditional unevaluatedProperties test");
    assert!(
        output.status.success(),
        "generated dataclass conditional unevaluatedProperties test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn generated_dataclasses_keep_prefix_item_types_ergonomic_without_losing_schema_checks() {
    let source = generate_dataclass_models(&json!({
        "title": "TracePoint",
        "type": "object",
        "properties": {
            "coordinates": {
                "type": "array",
                "prefixItems": [
                    {"type": "string"},
                    {"type": "integer"},
                ],
                "items": false,
                "minItems": 2,
                "maxItems": 2,
            }
        },
        "required": ["coordinates"],
        "additionalProperties": false,
    }))
    .expect("generate dataclasses from prefixItems schema");
    let module_path = write_temp_module("prefix_items", &source);

    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
import importlib.util
import sys

module_path = sys.argv[1]
spec = importlib.util.spec_from_file_location("prefix_item_models", module_path)
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = module
spec.loader.exec_module(module)

point = module.TracePoint(coordinates=["aisle", 7])
assert point.to_value() == {"coordinates": ["aisle", 7]}
assert module.TracePoint.from_value({"coordinates": ["aisle", 7]}).to_value() == {
    "coordinates": ["aisle", 7]
}

for factory in (
    lambda: module.TracePoint(coordinates=[7, "aisle"]),
    lambda: module.TracePoint(coordinates=["aisle", 7, 9]),
    lambda: module.TracePoint.from_value({"coordinates": [7, "aisle"]}),
):
    try:
        factory()
    except (TypeError, ValueError):
        pass
    else:
        raise AssertionError("prefixItems schema invariant was not enforced")
"###,
    );
    command.arg(module_path);
    let output = command
        .output()
        .expect("run generated dataclass prefixItems test");
    assert!(
        output.status.success(),
        "generated dataclass prefixItems test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn generated_dataclasses_preserve_pattern_properties_even_when_additional_properties_are_closed() {
    let source = generate_dataclass_models(&json!({
        "title": "LabeledRecord",
        "type": "object",
        "properties": {
            "name": {"type": "string"},
        },
        "patternProperties": {
            "^x-": {"type": "integer"},
        },
        "required": ["name"],
        "additionalProperties": false,
    }))
    .expect("generate dataclasses from patternProperties schema");
    let module_path = write_temp_module("pattern_properties", &source);

    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
import importlib.util
import sys

module_path = sys.argv[1]
spec = importlib.util.spec_from_file_location("pattern_property_models", module_path)
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = module
spec.loader.exec_module(module)

record = module.LabeledRecord.from_value({"name": "Ada", "x-rank": 7})
assert record.__jsoncompat_extra__ == {"x-rank": 7}
assert record.to_value() == {"name": "Ada", "x-rank": 7}
"###,
    );
    command.arg(module_path);
    let output = command
        .output()
        .expect("run generated dataclass patternProperties test");
    assert!(
        output.status.success(),
        "generated dataclass patternProperties test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn python_api_exposes_reusable_schema_tools_and_deprecates_one_shot_helpers() {
    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
import json
import warnings

import jsoncompat

schema_json = '{"type":"object","properties":{"name":{"type":"string"}},"required":["name"],"additionalProperties":false}'
valid_json = '{"name":"Ada"}'
invalid_json = '{"name":1}'

validator = jsoncompat.validator_for(schema_json)
assert validator.is_valid_json(valid_json)
assert not validator.is_valid_json(invalid_json)
assert validator.is_valid_value({"name": "Ada"})
assert not validator.is_valid_value({"name": 1})

array_validator = jsoncompat.validator_for('{"type":"array","items":{"type":"integer"}}')
assert array_validator.is_valid_value((1, 2, 3))
assert not array_validator.is_valid_value((1, "two", 3))

integer_validator = jsoncompat.validator_for('{"type":"integer"}')
big_integer = 10 ** 80
assert integer_validator.is_valid_json(str(big_integer))
assert integer_validator.is_valid_value(big_integer)

exclusive_validator = jsoncompat.validator_for('{"exclusiveMaximum":9.727837981879871e+26}')
exclusive_boundary = 9.727837981879871e+26
assert not exclusive_validator.is_valid_json(json.dumps(exclusive_boundary))
assert not exclusive_validator.is_valid_value(exclusive_boundary)

generator = jsoncompat.generator_for(schema_json)
generated = generator.generate_value(3)
assert validator.is_valid_json(generated)

try:
    jsoncompat.validator_for('{"type": 1}')
except ValueError:
    pass
else:
    raise AssertionError("invalid schema was accepted")

try:
    validator.is_valid_json("{")
except ValueError:
    pass
else:
    raise AssertionError("invalid instance JSON was accepted")

try:
    validator.is_valid_value({"name": object()})
except TypeError:
    pass
else:
    raise AssertionError("non-JSON Python values were accepted")

try:
    validator.is_valid_value({1: "not-json"})
except TypeError:
    pass
else:
    raise AssertionError("non-string JSON object keys were accepted")

try:
    validator.is_valid_value({"name": float("inf")})
except ValueError:
    pass
else:
    raise AssertionError("non-finite JSON numbers were accepted")

try:
    jsoncompat.generator_for('{"type": 1}')
except ValueError:
    pass
else:
    raise AssertionError("invalid generator schema was accepted")

with warnings.catch_warnings(record=True) as caught:
    warnings.simplefilter("always", DeprecationWarning)
    assert jsoncompat.is_valid(schema_json, valid_json)

assert len(caught) == 1
assert issubclass(caught[0].category, DeprecationWarning)
assert "validator_for" in str(caught[0].message)

with warnings.catch_warnings(record=True) as caught:
    warnings.simplefilter("always", DeprecationWarning)
    assert validator.is_valid_json(jsoncompat.generate_value(schema_json, 3))

assert len(caught) == 1
assert issubclass(caught[0].category, DeprecationWarning)
assert "generator_for" in str(caught[0].message)
"###,
    );
    let output = command.output().expect("run Python validation API test");
    assert!(
        output.status.success(),
        "Python validation API test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn generated_dataclasses_for_checkout_demo_are_python_usable() {
    let source = generate_dataclass_models(&json!({
        "type": "object",
        "required": ["event", "customer", "items", "currency"],
        "properties": {
            "event": {
                "enum": ["checkout.completed", "checkout.failed"]
            },
            "customer": {
                "type": "object",
                "required": ["id", "email", "segment"],
                "properties": {
                    "id": { "type": "string" },
                    "email": { "type": "string", "format": "email" },
                    "segment": { "enum": ["self_serve", "startup", "enterprise"] },
                    "trialDaysRemaining": { "type": "integer", "minimum": 0, "maximum": 30 }
                },
                "additionalProperties": false
            },
            "items": {
                "type": "array",
                "minItems": 1,
                "maxItems": 3,
                "items": {
                    "type": "object",
                    "required": ["sku", "quantity", "unitPrice"],
                    "properties": {
                        "sku": { "enum": ["starter-seat", "team-seat", "audit-log"] },
                        "quantity": { "type": "integer", "minimum": 1, "maximum": 5 },
                        "unitPrice": { "type": "integer", "minimum": 0, "maximum": 500 }
                    },
                    "additionalProperties": false
                }
            },
            "currency": { "enum": ["USD", "EUR", "GBP"] },
            "couponCode": { "type": "string", "minLength": 4, "maxLength": 12 }
        },
        "additionalProperties": false
    }))
    .expect("generate dataclasses from checkout demo schema");
    assert!(!source.contains("__jsoncompat_object_spec__"));
    assert!(!source.contains("dc.object_spec("));
    assert!(!source.contains("dc.field_spec("));
    assert!(!source.contains("__jsoncompat_root_annotation__"));
    let module_path = write_temp_module("checkout_demo", &source);

    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
import importlib.util
import sys
import typing

from jsoncompat.codegen.dataclasses import JSONCOMPAT_MISSING

module_path = sys.argv[1]
spec = importlib.util.spec_from_file_location("checkout_demo_models", module_path)
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = module
spec.loader.exec_module(module)

model_hints = typing.get_type_hints(module.GeneratedSchema)
customer_hints = typing.get_type_hints(module.GeneratedSchemaCustomer)
item_hints = typing.get_type_hints(module.GeneratedSchemaItem)
assert model_hints["customer"] is module.GeneratedSchemaCustomer
assert model_hints["items"] == list[module.GeneratedSchemaItem]
assert customer_hints["id"] is str
assert item_hints["quantity"] is int

assert module.GeneratedSchema.__jsoncompat_schema__.startswith('{\n')
assert '"minProperties"' not in module.GeneratedSchema.__jsoncompat_schema__

customer = module.GeneratedSchemaCustomer(
    id="cus_123",
    email="ada@example.com",
    segment="enterprise",
    trialDaysRemaining=7,
)
item = module.GeneratedSchemaItem(
    sku="team-seat",
    quantity=2,
    unitPrice=120,
)
event = module.GeneratedSchema(
    event="checkout.completed",
    customer=customer,
    items=[item],
    currency="USD",
)
assert event.couponCode is JSONCOMPAT_MISSING
assert event.to_value() == {
    "event": "checkout.completed",
    "customer": {
        "id": "cus_123",
        "email": "ada@example.com",
        "segment": "enterprise",
        "trialDaysRemaining": 7,
    },
    "items": [
        {
            "sku": "team-seat",
            "quantity": 2,
            "unitPrice": 120,
        }
    ],
    "currency": "USD",
}

parsed = module.GeneratedSchema.from_value({
    "event": "checkout.completed",
    "customer": {
        "id": "cus_123",
        "email": "ada@example.com",
        "segment": "enterprise",
        "trialDaysRemaining": 7,
    },
    "items": [
        {
            "sku": "team-seat",
            "quantity": 2,
            "unitPrice": 120,
        }
    ],
    "currency": "USD",
})
assert parsed.couponCode is JSONCOMPAT_MISSING
assert parsed.customer.id == "cus_123"
assert parsed.items[0].sku == "team-seat"
"###,
    );
    command.arg(module_path);
    let output = command
        .output()
        .expect("run generated dataclass checkout demo test");
    assert!(
        output.status.success(),
        "generated dataclass checkout demo test failed: {}",
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
assert payload.to_value() == {"name": "Ada"}

writer = module.CollisionWriter(version=1, data=payload)
assert writer.to_value() == {"version": 1, "data": {"name": "Ada"}}
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
