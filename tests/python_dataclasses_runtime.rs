use jsoncompat::{StampManifest, stamp_schema};
use jsoncompat_codegen::generate_dataclass_models;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[path = "support/python_env.rs"]
mod python_env;

#[test]
fn generated_dataclasses_expose_one_native_runtime_contract() {
    let source = generate_dataclass_models(&json!({
        "title": "Profile",
        "type": "object",
        "properties": {
            "name": {"type": "string", "minLength": 1},
            "tags": {"type": "array", "items": {"type": "string"}},
            "attributes": {
                "type": "object",
                "additionalProperties": {
                    "type": "array",
                    "items": {"type": "integer"}
                }
            }
        },
        "required": ["name", "tags", "attributes"],
        "additionalProperties": {"type": "string"}
    }))
    .expect("generate object dataclass");
    let root_source = generate_dataclass_models(&json!({
        "title": "HugeInteger",
        "type": "integer"
    }))
    .expect("generate root dataclass");
    let ownership_source = generate_dataclass_models(&json!({
        "title": "AuditContext",
        "type": "object",
        "additionalProperties": {"type": "string"}
    }))
    .expect("generate ownership dataclass");
    let stamped = stamp_schema(
        &StampManifest::empty(),
        "profile",
        json!({
            "type": "object",
            "properties": {
                "name": {"type": "string", "minLength": 1}
            },
            "required": ["name"],
            "additionalProperties": false
        }),
    )
    .expect("stamp directional schemas");
    let writer_source =
        generate_dataclass_models(&stamped.bundle.writer).expect("generate writer dataclasses");
    let reader_source =
        generate_dataclass_models(&stamped.bundle.reader).expect("generate reader dataclasses");

    let module_path = write_temp_module("native_contract", &source);
    let root_module_path = write_temp_module("native_contract_root", &root_source);
    let ownership_module_path = write_temp_module("native_contract_ownership", &ownership_source);
    let writer_module_path = write_temp_module("native_contract_writer", &writer_source);
    let reader_module_path = write_temp_module("native_contract_reader", &reader_source);

    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
from dataclasses import FrozenInstanceError, is_dataclass
import gc
import importlib.util
import sys

from jsoncompat.codegen import SerializationFormat
from jsoncompat.codegen import dataclasses as dc


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


models = load("native_contract_models", sys.argv[1])
roots = load("native_contract_roots", sys.argv[2])
ownerships = load("native_contract_ownership", sys.argv[3])
writers = load("native_contract_writers", sys.argv[4])
readers = load("native_contract_readers", sys.argv[5])

Profile = models.Profile
ProfileAttributes = models.ProfileAttributes
value = {
    "name": "Ada",
    "tags": ["schema", "runtime"],
    "attributes": {"scores": [1, 2]},
    "nickname": "ace",
}
attributes = ProfileAttributes.from_value({"scores": [1, 2]})
profile = Profile(
    name="Ada",
    tags=["schema", "runtime"],
    attributes=attributes,
    __jsoncompat_extra__={"nickname": "ace"},
)
assert is_dataclass(profile)
assert profile.to_value() == value

for skip_validation in (False, True):
    parsed_value = Profile.from_value(value, skip_validation=skip_validation)
    parsed_json = Profile.deserialize(
        '{"name":"Ada","tags":["schema","runtime"],'
        '"attributes":{"scores":[1,2]},"nickname":"ace"}',
        skip_validation=skip_validation,
    )
    assert parsed_value.to_value(skip_validation=True) == value
    assert parsed_json.to_value(skip_validation=True) == value
    assert Profile.deserialize(
        profile.serialize(skip_validation=skip_validation),
        skip_validation=skip_validation,
    ).to_value(skip_validation=True) == value

for format in SerializationFormat:
    encoded = profile.serialize(format=format)
    assert Profile.deserialize(encoded, format=format).to_value() == value

assert isinstance(profile.tags, dc.FrozenList)
assert is_dataclass(profile.attributes)
assert isinstance(profile.attributes.__jsoncompat_extra__, dc.FrozenDict)
assert isinstance(
    profile.attributes.__jsoncompat_extra__["scores"],
    dc.FrozenList,
)
assert isinstance(profile.__jsoncompat_extra__, dc.FrozenDict)
for mutation in (
    lambda: setattr(profile, "name", "Grace"),
    lambda: profile.tags.append("mutable"),
    lambda: profile.attributes.__jsoncompat_extra__.__setitem__("scores", []),
    lambda: profile.attributes.__jsoncompat_extra__["scores"].append(3),
    lambda: profile.__jsoncompat_extra__.__setitem__("nickname", "changed"),
):
    try:
        mutation()
    except (AttributeError, FrozenInstanceError, TypeError):
        pass
    else:
        raise AssertionError("generated object graph remained mutable")

# Native tuple construction and frozen-slot writes must transfer exactly one
# reference for each FrozenDict entry and release it with the model graph.
if hasattr(sys, "getrefcount"):
    lifetime_key = "".join(("jsoncompat", "-native-key"))
    lifetime_value = "".join(("jsoncompat", "-native-value"))
    lifetime_input = {lifetime_key: lifetime_value}
    lifetime_context = ownerships.AuditContext.from_value(
        lifetime_input,
        skip_validation=True,
    )
    stored_key = next(iter(lifetime_context.__jsoncompat_extra__))
    stored_value = lifetime_context.__jsoncompat_extra__[stored_key]
    key_refs = sys.getrefcount(stored_key)
    value_refs = sys.getrefcount(stored_value)
    del lifetime_context
    gc.collect()
    assert sys.getrefcount(stored_key) == key_refs - 1, (
        key_refs,
        sys.getrefcount(stored_key),
    )
    assert sys.getrefcount(stored_value) == value_refs - 1, (
        value_refs,
        sys.getrefcount(stored_value),
    )

try:
    Profile(
        name="",
        tags=[],
        attributes=ProfileAttributes.from_value({}),
        __jsoncompat_extra__={},
    )
except ValueError:
    pass
else:
    raise AssertionError("checked construction accepted invalid data")

trusted = Profile(
    name="",
    tags=[],
    attributes=ProfileAttributes.from_value({}),
    __jsoncompat_extra__={},
    skip_validation=True,
)
assert trusted.to_value(skip_validation=True)["name"] == ""
try:
    trusted.serialize()
except ValueError:
    pass
else:
    raise AssertionError("checked serialization trusted unchecked data")

HugeInteger = roots.JSONCOMPAT_MODEL
huge = 10**80
root = HugeInteger(root=huge)
assert is_dataclass(root)
assert root.to_value() == huge
assert HugeInteger.from_value(huge).root == huge
assert HugeInteger.deserialize(str(huge)).root == huge
assert HugeInteger.deserialize(root.serialize()).root == huge

payload = writers.ProfileV1(name="Ada")
writer = writers.ProfileWriter(version=1, data=payload)
wire = writer.serialize()
reader = readers.ProfileReader.deserialize(wire)
assert reader.root.version == 1
assert reader.root.data.name == "Ada"

for forbidden in (
    lambda: writers.ProfileWriter.deserialize(wire),
    lambda: reader.serialize(),
):
    try:
        forbidden()
    except TypeError:
        pass
    else:
        raise AssertionError("reader/writer direction guard did not fire")

class CustomProfile(Profile):
    pass

for unbound in (
    lambda: CustomProfile(
        name="Ada",
        tags=[],
        attributes=ProfileAttributes.from_value({}),
        __jsoncompat_extra__={},
    ),
    lambda: CustomProfile.deserialize(
        '{"name":"Ada","tags":[],"attributes":{}}'
    ),
):
    try:
        unbound()
    except TypeError as error:
        assert "must be an unmodified generated frozen dataclass" in str(error)
    else:
        raise AssertionError("an unbound custom subclass used generated runtime state")
"###,
    );
    command
        .arg(module_path)
        .arg(root_module_path)
        .arg(ownership_module_path)
        .arg(writer_module_path)
        .arg(reader_module_path);
    let output = command
        .output()
        .expect("run generated native runtime contract");
    assert!(
        output.status.success(),
        "generated native runtime contract failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn generated_module_lazily_compiles_one_shared_plan_for_every_recursive_model_root() {
    let source = generate_dataclass_models(&json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$ref": "#/$defs/a",
        "$defs": {
            "a": {
                "anyOf": [
                    {"type": "null"},
                    {
                        "type": "object",
                        "properties": {"next": {"$ref": "#/$defs/b"}},
                        "required": ["next"],
                        "additionalProperties": false
                    }
                ]
            },
            "b": {
                "anyOf": [
                    {"type": "null"},
                    {
                        "type": "object",
                        "properties": {"next": {"$ref": "#/$defs/a"}},
                        "required": ["next"],
                        "additionalProperties": false
                    }
                ]
            }
        }
    }))
    .expect("generate mutually recursive dataclasses");
    let module_path = write_temp_module("shared_recursive_plan", &source);

    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
import importlib.util
import sys

from jsoncompat.codegen import dataclasses as dc


compile_calls = []
native_compile = dc.compile_model_runtimes


def record_compile(model_roots, descriptors, frozen_list_type, frozen_dict_type):
    compile_calls.append(
        (tuple(model_type.__name__ for model_type, _ in model_roots), len(descriptors))
    )
    return native_compile(
        model_roots,
        descriptors,
        frozen_list_type,
        frozen_dict_type,
    )


dc.compile_model_runtimes = record_compile
try:
    spec = importlib.util.spec_from_file_location("shared_recursive_models", sys.argv[1])
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    assert compile_calls == []
    del sys.modules[spec.name]
    module.GeneratedSchema.from_value(
        {"next": {"next": None}},
        skip_validation=True,
    )
finally:
    dc.compile_model_runtimes = native_compile

assert len(compile_calls) == 1
root_names, descriptor_count = compile_calls[0]
assert set(root_names) == {
    "GeneratedSchemaABranch1",
    "GeneratedSchemaA",
    "GeneratedSchemaBBranch1",
    "GeneratedSchemaB",
    "GeneratedSchema",
}
assert len(root_names) == 5
assert descriptor_count >= len(root_names)

cases = (
    (module.GeneratedSchemaABranch1, {"next": None}),
    (module.GeneratedSchemaA, None),
    (module.GeneratedSchemaBBranch1, {"next": None}),
    (module.GeneratedSchemaB, None),
    (module.GeneratedSchema, {"next": {"next": None}}),
)
for model_type, value in cases:
    instance = model_type.from_value(value)
    assert instance.to_value() == value
    assert model_type.deserialize(instance.serialize()).to_value() == value
"###,
    );
    command.arg(module_path);
    let output = command
        .output()
        .expect("run shared recursive conversion plan test");
    assert!(
        output.status.success(),
        "shared recursive conversion plan test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn trusted_generated_model_use_does_not_compile_its_schema() {
    let source = generate_dataclass_models(&json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://test.json-schema.org/lazy-schema/root",
        "title": "LazySchema",
        "type": "array",
        "items": { "$dynamicRef": "#items" },
        "$defs": {
            "foo": {
                "$dynamicAnchor": "items",
                "type": "string"
            }
        }
    }))
    .expect("generate a schema unsupported by the runtime validator");
    let module_path = write_temp_module("lazy_schema", &source);

    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
import importlib.util
import sys


spec = importlib.util.spec_from_file_location("lazy_schema_models", sys.argv[1])
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = module
spec.loader.exec_module(module)

model_type = module.JSONCOMPAT_MODEL
assert "__jsoncompat_runtime__" not in model_type.__dict__
trusted = model_type.from_value(["first", "second"], skip_validation=True)
assert trusted.to_value(skip_validation=True) == ["first", "second"]

try:
    model_type.from_value(["first", "second"])
except ValueError as error:
    assert "unsupported reference" in str(error)
else:
    raise AssertionError("checked use did not compile the unsupported schema")
"###,
    );
    command.arg(module_path);
    let output = command
        .output()
        .expect("run lazy generated schema compilation test");
    assert!(
        output.status.success(),
        "lazy generated schema compilation test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn generated_module_runtime_cycles_are_collectable() {
    let source = generate_dataclass_models(&json!({
        "title": "Envelope",
        "type": "object",
        "$defs": {
            "payload": {
                "type": "object",
                "properties": {
                    "name": {"type": "string"}
                },
                "required": ["name"],
                "additionalProperties": false
            }
        },
        "properties": {
            "payload": {"$ref": "#/$defs/payload"},
            "history": {
                "type": "array",
                "items": {"$ref": "#/$defs/payload"}
            }
        },
        "required": ["payload", "history"],
        "additionalProperties": {
            "anyOf": [
                {"$ref": "#/$defs/payload"},
                {"type": "null"}
            ]
        }
    }))
    .expect("generate multi-class dataclasses for GC regression");
    assert!(source.contains("collections.abc.Sequence["));
    assert!(!source.contains("typing.Sequence["));
    let module_path = write_temp_module("collectable_runtime_cycle", &source);

    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
import gc
import importlib.util
import sys
import weakref

from jsoncompat.codegen import dataclasses as dc


def import_weakrefs(index):
    module_name = f"collectable_runtime_cycle_{index}"
    spec = importlib.util.spec_from_file_location(module_name, sys.argv[1])
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    module.JSONCOMPAT_MODEL.from_value(
        {"payload": {"name": "Ada"}, "history": []},
        skip_validation=True,
    )
    classes = tuple(dict.fromkeys(
        value
        for value in vars(module).values()
        if isinstance(value, type)
        and issubclass(value, dc.DataclassModel)
        and value.__module__ == module_name
    ))
    assert len(classes) >= 2
    for model_type in classes:
        runtime = model_type.__dict__["__jsoncompat_runtime__"]
        assert gc.is_tracked(runtime)
        runtime_referents = gc.get_referents(runtime)
        assert set(classes).intersection(runtime_referents) == {model_type}
        plan_referents = tuple(
            referent
            for referent in runtime_referents
            if type(referent).__module__ == "jsoncompat._native"
            and type(referent).__name__ == "_ModelPlan"
        )
        assert len(plan_referents) == 1
        assert set(classes).issubset(gc.get_referents(plan_referents[0]))
    module_ref = weakref.ref(module)
    class_refs = tuple(weakref.ref(model_type) for model_type in classes)
    del sys.modules[module_name]
    return module_ref, class_refs


for index in range(25):
    module_ref, class_refs = import_weakrefs(index)
    gc.collect()
    assert module_ref() is None
    alive = tuple(class_ref() for class_ref in class_refs if class_ref() is not None)
    assert not alive, tuple(
        (
            model_type,
            tuple(
                (type(referrer).__name__, repr(referrer)[:300])
                for referrer in gc.get_referrers(model_type)
            ),
        )
        for model_type in alive
    )
"###,
    );
    command.arg(module_path);
    let output = command
        .output()
        .expect("run generated runtime GC regression");
    assert!(
        output.status.success(),
        "generated runtime GC regression failed: {}",
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

large_integer = 10**80
assert deserialize_value(serialize_value({"value": large_integer})) == {
    "value": large_integer,
}
assert deserialize_value(b'{"name":"Ada"}') == {"name": "Ada"}

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
            "metadata": {},
            "uniqueValues": {"type": "array", "uniqueItems": true},
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
from types import MappingProxyType

from jsoncompat.codegen import dataclasses as dc

module_path = sys.argv[1]
spec = importlib.util.spec_from_file_location("generated_models", module_path)
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = module
spec.loader.exec_module(module)

model = module.InventoryItem
valid = model.from_value({"sku": "abc", "quantity": 3, "tags": ["new"]})
assert valid.to_value() == {"sku": "abc", "quantity": 3, "tags": ["new"]}
omitted = model(sku="abc", quantity=3)
assert omitted.to_value() == {"sku": "abc", "quantity": 3}

for skip_validation in (False, True):
    try:
        model.from_value(
            {
                "sku": "abc",
                "quantity": 3,
                "tags": dc.JSONCOMPAT_MISSING,
            },
            skip_validation=skip_validation,
        )
    except (TypeError, ValueError):
        pass
    else:
        raise AssertionError(
            "a present JSONCOMPAT_MISSING sentinel was treated as an absent field"
        )

large_integer = 10**80
from_json = model.deserialize(
    f'{{"sku":"abc","quantity":{large_integer}}}',
)
assert from_json.quantity == large_integer
assert model.deserialize(
    from_json.serialize(),
).quantity == large_integer
assert model.deserialize(b'{"sku":"abc","quantity":3}').quantity == 3

invalid_json_payloads = (
    '{"sku":"abc","sku":"duplicate","quantity":3}',
    '{"sku":"abc","quantity":1e999}',
    '{"sku":"abc","quantity":NaN}',
)
for payload in invalid_json_payloads:
    try:
        model.deserialize(payload)
    except ValueError:
        pass
    else:
        raise AssertionError(f"invalid JSON payload was accepted: {payload!r}")

invalid_values = [
    {"quantity": 3},
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

cyclic = []
cyclic.append(cyclic)
non_json_values = [
    {"sku": "abc", "quantity": 3, "metadata": object()},
    {"sku": "abc", "quantity": 3, "metadata": {1: "not-json"}},
    {"sku": "abc", "quantity": 3, "metadata": float("inf")},
    {"sku": "abc", "quantity": 3, "metadata": cyclic},
    {"sku": "abc", "quantity": 3, "uniqueValues": cyclic},
]

for value in non_json_values:
    for skip_validation in (False, True):
        try:
            model.from_value(value, skip_validation=skip_validation)
        except (TypeError, ValueError):
            pass
        else:
            raise AssertionError(f"non-JSON payload was accepted: {value!r}")

mapping_value = {
    "sku": "abc",
    "quantity": 3,
    "metadata": MappingProxyType({"source": "proxy"}),
}
try:
    model.from_value(mapping_value)
except ValueError:
    pass
else:
    raise AssertionError("checked construction accepted a non-JSON Mapping")
assert model.from_value(
    mapping_value,
    skip_validation=True,
).to_value(skip_validation=True)["metadata"] == {"source": "proxy"}
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
fn generated_fields_cannot_shadow_dataclass_runtime_internals() {
    let source = generate_dataclass_models(&json!({
        "title": "RuntimeNames",
        "type": "object",
        "properties": {
            "__annotations__": {"type": "string"},
            "__init__": {"type": "integer"},
            "__jsoncompat_schema__": {"type": "string"},
            "__new__": {"type": "string"},
            "__post_init__": {"type": "string"},
            "__slots__": {"type": "string"},
            "get_additional_property": {"type": "string"},
        },
        "required": [
            "__annotations__",
            "__init__",
            "__jsoncompat_schema__",
            "__new__",
            "__post_init__",
            "__slots__",
            "get_additional_property",
        ],
        "additionalProperties": {"type": "string"},
    }))
    .expect("generate dataclass with runtime-colliding wire names");
    let module_path = write_temp_module("runtime_name_collisions", &source);

    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
import importlib.util
import json
import sys

module_path = sys.argv[1]
spec = importlib.util.spec_from_file_location("runtime_name_models", module_path)
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = module
spec.loader.exec_module(module)

value = {
    "__annotations__": "annotations",
    "__init__": 7,
    "__jsoncompat_schema__": "schema",
    "__new__": "new",
    "__post_init__": "post",
    "__slots__": "slots",
    "get_additional_property": "field",
    "extra": "value",
}
model = module.RuntimeNames.deserialize(json.dumps(value))
assert model.to_value() == value
assert json.loads(model.serialize()) == value
assert model.field___post_init__ == "post"
assert model.field___jsoncompat_schema__ == "schema"
assert model.field___init__ == 7
assert model.get_additional_property_ == "field"
assert model.get_additional_property("extra") == "value"
"###,
    );
    command.arg(module_path);
    let output = command
        .output()
        .expect("run generated dataclass runtime-name collision test");
    assert!(
        output.status.success(),
        "generated runtime-name collision test failed: {}",
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
fn generated_dataclasses_combine_successful_any_of_property_annotations() {
    let source = generate_dataclass_models(&json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "anyOf": [
            {
                "properties": {
                    "foo": {"type": "string"},
                },
            },
            {
                "properties": {
                    "bar": {"type": "integer"},
                },
            },
        ],
        "unevaluatedProperties": false,
    }))
    .expect("generate dataclasses from anyOf unevaluatedProperties schema");
    let module_path = write_temp_module("any_of_unevaluated_properties", &source);

    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
import importlib.util
import json
import sys

module_path = sys.argv[1]
spec = importlib.util.spec_from_file_location("any_of_models", module_path)
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = module
spec.loader.exec_module(module)

expected = {"foo": "x", "bar": 1}
from_value = module.GeneratedSchema.from_value(expected)
from_json = module.GeneratedSchema.deserialize('{"foo":"x","bar":1}')
assert from_value.to_value() == expected
assert from_json.to_value() == expected
assert json.loads(from_value.serialize()) == expected
assert json.loads(from_json.serialize()) == expected

for invalid in (
    lambda: module.GeneratedSchema.from_value({"baz": True}),
    lambda: module.GeneratedSchema.deserialize('{"baz":true}'),
):
    try:
        invalid()
    except (TypeError, ValueError):
        pass
    else:
        raise AssertionError("unevaluatedProperties accepted an unannotated key")
"###,
    );
    command.arg(module_path);
    let output = command
        .output()
        .expect("run generated dataclass anyOf unevaluatedProperties test");
    assert!(
        output.status.success(),
        "generated dataclass anyOf unevaluatedProperties test failed: {}",
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
fn generated_dataclasses_keep_validation_cache_separate_from_json_properties() {
    let source = generate_dataclass_models(&json!({
        "title": "cache collision",
        "type": "object",
        "properties": {
            "_jsoncompat_validated": { "type": "boolean" }
        },
        "required": ["_jsoncompat_validated"],
        "additionalProperties": false
    }))
    .expect("generate dataclasses with validation-cache property collision");
    let module_path = write_temp_module("validation_cache_collision", &source);

    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
import importlib.util
import sys

module_path = sys.argv[1]
spec = importlib.util.spec_from_file_location("validation_cache_collision", module_path)
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = module
spec.loader.exec_module(module)

model = module.CacheCollision.from_value({"_jsoncompat_validated": False})
assert model._jsoncompat_validated_ is False
assert model._jsoncompat_validated is True
assert model.to_value() == {"_jsoncompat_validated": False}
assert model.serialize() == '{"_jsoncompat_validated":false}'
"###,
    );
    command.arg(module_path);
    let output = command
        .output()
        .expect("run validation-cache collision test");
    assert!(
        output.status.success(),
        "validation-cache collision test failed: {}",
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
import collections.abc
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
assert model_hints["items"] == collections.abc.Sequence[module.GeneratedSchemaItem]
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
