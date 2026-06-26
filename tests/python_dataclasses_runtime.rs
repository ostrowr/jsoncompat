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
assert gc.is_tracked(profile.tags)
assert is_dataclass(profile.attributes)
assert isinstance(profile.attributes.__jsoncompat_extra__, dc.FrozenDict)
assert isinstance(
    profile.attributes.__jsoncompat_extra__["scores"],
    dc.FrozenList,
)
assert gc.is_tracked(profile.attributes.__jsoncompat_extra__["scores"])
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

    lifetime_item = "".join(("jsoncompat", "-native-list-item"))
    lifetime_profile = Profile.from_value(
        {
            "name": "Ada",
            "tags": [lifetime_item],
            "attributes": {},
        },
        skip_validation=True,
    )
    stored_item = lifetime_profile.tags[0]
    item_refs = sys.getrefcount(stored_item)
    del lifetime_profile
    gc.collect()
    assert sys.getrefcount(stored_item) == item_refs - 1, (
        item_refs,
        sys.getrefcount(stored_item),
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

try:
    class CustomProfile(Profile):
        pass
except TypeError as error:
    assert "generated model Profile cannot be subclassed" in str(error)
else:
    raise AssertionError("a generated model subclass was representable")
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
fn generated_dataclass_native_slots_reject_foreign_descriptors() {
    let source = generate_dataclass_models(&json!({
        "title": "SlotSafety",
        "type": "object",
        "properties": {"value": {"type": "string"}},
        "required": ["value"],
        "additionalProperties": false,
    }))
    .expect("generate slot-safety dataclass");
    let module_path = write_temp_module("foreign_slot_descriptor", &source);

    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
import importlib.util
import sys

module_path = sys.argv[1]
spec = importlib.util.spec_from_file_location("slot_safety_models", module_path)
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = module
spec.loader.exec_module(module)


class ForeignSlots:
    __slots__ = tuple(f"slot_{index}" for index in range(100))


# Install a valid member descriptor whose offset belongs to a much larger,
# unrelated allocation before the generated runtime compiles its slot plan.
original_value_descriptor = module.SlotSafety.value
module.SlotSafety.value = ForeignSlots.slot_99
for _ in range(2):
    try:
        module.SlotSafety.from_value({"value": "safe"})
    except TypeError:
        pass
    else:
        raise AssertionError("foreign slot descriptor was used for native construction")

module.SlotSafety.value = original_value_descriptor
module.SlotSafety.from_value({"value": "safe"})

try:
    class SlotSafetySubclass(module.SlotSafety):
        __slots__ = ()
except TypeError as error:
    assert "generated model SlotSafety cannot be subclassed" in str(error), str(error)
else:
    raise AssertionError("a generated model subclass was representable")
"###,
    );
    command.arg(module_path);
    let output = command
        .output()
        .expect("run generated dataclass foreign-slot test");
    assert!(
        output.status.success(),
        "generated dataclass foreign-slot test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn generated_dataclass_native_slots_reject_property_replacement() {
    let source = generate_dataclass_models(&json!({
        "title": "PropertyReplacement",
        "type": "object",
        "properties": {"value": {"type": "string"}},
        "required": ["value"],
        "additionalProperties": false,
    }))
    .expect("generate property-replacement dataclass");
    let module_path = write_temp_module("property_slot_replacement", &source);

    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
import importlib.util
import sys

module_path = sys.argv[1]
spec = importlib.util.spec_from_file_location("property_slot_models", module_path)
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = module
spec.loader.exec_module(module)

setter_calls = []

def get_value(instance):
    return "property"

def set_value(instance, value):
    setter_calls.append((instance, value))

module.PropertyReplacement.value = property(get_value, set_value)
try:
    module.PropertyReplacement.from_value({"value": "unsafe"})
except TypeError as error:
    assert "must be an exact member descriptor" in str(error), str(error)
else:
    raise AssertionError("a replacement property was accepted as generated storage")

assert setter_calls == []
"###,
    );
    command.arg(module_path);
    let output = command
        .output()
        .expect("run generated dataclass property-replacement test");
    assert!(
        output.status.success(),
        "generated dataclass property-replacement test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn native_plan_descriptor_protocol_rejects_unrepresentable_states() {
    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
from jsoncompat import compile_model_runtimes
from jsoncompat.codegen import dataclasses as dc
from jsoncompat.codegen.dataclasses import (
    JSONCOMPAT_MISSING,
    FrozenDict,
    FrozenList,
    JsoncompatMissingType,
)


class Model:
    __slots__ = (
        "__jsoncompat_extra__",
        "root",
        "value",
        "other",
    )
    __jsoncompat_schema__ = "{}"


assert repr(JSONCOMPAT_MISSING) == "JSONCOMPAT_MISSING"
assert JsoncompatMissingType() is JSONCOMPAT_MISSING
assert JsoncompatMissingType.__new__(JsoncompatMissingType) is JSONCOMPAT_MISSING
for construct_missing in (lambda: object.__new__(JsoncompatMissingType),):
    try:
        construct_missing()
    except TypeError:
        pass
    else:
        raise AssertionError("constructed a second native missing sentinel")


def compile(descriptors):
    return compile_model_runtimes([], descriptors, FrozenList, FrozenDict)


def rejects(label, descriptors):
    try:
        compile(descriptors)
    except (IndexError, TypeError, ValueError):
        return
    raise AssertionError(f"accepted malformed native descriptor: {label}")


cases = (
    ("empty node", [()]),
    ("scalar trailing item", [("str", "trailing")]),
    ("short list", [("list",)]),
    ("external list item", [("list", 1)]),
    ("external dict value", [("dict", 1)]),
    ("external union branch", [("union", (1,), None, None)]),
    ("external root value", [("root", Model, 1)]),
    (
        "external model field value",
        [("model", Model, (("value", "value", 1, False),), None)],
    ),
    (
        "external additional-property value",
        [("model", Model, (), 1)],
    ),
    ("long root", [("str",), ("root", Model, 0, "trailing")]),
    ("empty literal", [("literal", ())]),
    ("unsupported literal object", [("literal", (object(),))]),
    ("empty union", [("union", (), None, None)]),
    (
        "discriminator without name",
        [("str",), ("union", (0,), None, (("x", 0),))],
    ),
    (
        "external discriminator target",
        [
            ("str",),
            ("any",),
            ("union", (0,), "kind", (("x", 1),)),
        ],
    ),
    (
        "duplicate discriminator value",
        [
            ("str",),
            ("int",),
            ("union", (0, 1), "kind", (("x", 0), ("x", 1))),
        ],
    ),
    (
        "long discriminator entry",
        [("str",), ("union", (0,), "kind", (("x", 0, "trailing"),))],
    ),
    (
        "long field",
        [
            ("str",),
            ("model", Model, (("value", "value", 0, False, "trailing"),), None),
        ],
    ),
    (
        "duplicate JSON field",
        [
            ("str",),
            (
                "model",
                Model,
                (("value", "value", 0, False), ("value", "other", 0, False)),
                None,
            ),
        ],
    ),
    (
        "duplicate Python field",
        [
            ("str",),
            (
                "model",
                Model,
                (("value", "value", 0, False), ("other", "value", 0, False)),
                None,
            ),
        ],
    ),
    (
        "old nullable field presence",
        [
            ("str",),
            (
                "model",
                Model,
                (("value", "value", 0, None),),
                None,
            ),
        ],
    ),
    (
        "reserved extra field",
        [
            ("str",),
            (
                "model",
                Model,
                (("value", "__jsoncompat_extra__", 0, False),),
                None,
            ),
        ],
    ),
)

for label, descriptors in cases:
    rejects(label, descriptors)

try:
    compile_model_runtimes([], [], FrozenList, FrozenDict, False)
except TypeError:
    pass
else:
    raise AssertionError("native runtime compiler accepted a caller-owned sentinel")

# The discriminator protocol stores an ordinal into the enclosing non-empty
# branch collection, rather than a global node id.
compile(
    [
        ("str",),
        ("int",),
        ("union", (0, 1), "kind", (("text", 0), ("number", 1))),
    ]
)

# The old protocol treated any non-None fourth item as the missing sentinel,
# so False could cause a present JSON false value to disappear on output. The
# fourth item is now only an omittable flag, and the canonical sentinel is
# supplied once for the whole plan.
required_runtime = compile_model_runtimes(
    [(Model, 1)],
    [("bool",), ("model", Model, (("value", "value", 0, False),), None)],
    FrozenList,
    FrozenDict,
)[0]
required_false = required_runtime.from_value({"value": False}, skip_validation=True)
assert required_runtime.to_value(required_false, skip_validation=True) == {"value": False}

dc.JSONCOMPAT_MISSING = False
optional_runtime = compile_model_runtimes(
    [(Model, 1)],
    [("bool",), ("model", Model, (("value", "value", 0, True),), None)],
    FrozenList,
    FrozenDict,
)[0]
optional_missing = optional_runtime.from_value({}, skip_validation=True)
assert optional_missing.value is JSONCOMPAT_MISSING
assert optional_runtime.to_value(optional_missing, skip_validation=True) == {}
assert optional_runtime.to_value(optional_missing) == {}
assert optional_runtime.serialize(optional_missing, skip_validation=True) == "{}"
assert optional_runtime.serialize(optional_missing) == "{}"
optional_false = optional_runtime.from_value({"value": False}, skip_validation=True)
assert optional_runtime.to_value(optional_false, skip_validation=True) == {"value": False}
assert optional_runtime.to_value(optional_false) == {"value": False}
assert optional_runtime.serialize(optional_false, skip_validation=True) == '{"value":false}'
assert optional_runtime.serialize(optional_false) == '{"value":false}'
"###,
    );
    let output = command
        .output()
        .expect("run native descriptor protocol invariant test");
    assert!(
        output.status.success(),
        "native descriptor protocol invariant test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn omittable_constructor_uses_only_the_native_missing_singleton() {
    let source = generate_dataclass_models(&json!({
        "title": "OptionalValue",
        "type": "object",
        "properties": {
            "value": {"type": ["boolean", "null"]}
        },
        "additionalProperties": false,
    }))
    .expect("generate omittable-value dataclass");
    let module_path = write_temp_module("native_missing_singleton", &source);

    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
import dataclasses
import copy
import importlib
import importlib.util
import pickle
import sys

import jsoncompat
from jsoncompat.codegen import dataclasses as dc

module_path = sys.argv[1]
canonical_missing = dc.JSONCOMPAT_MISSING
assert copy.copy(canonical_missing) is canonical_missing
assert copy.deepcopy(canonical_missing) is canonical_missing
assert pickle.loads(pickle.dumps(canonical_missing)) is canonical_missing
assert importlib.reload(jsoncompat).JSONCOMPAT_MISSING is canonical_missing
dc.JSONCOMPAT_MISSING = False
spec = importlib.util.spec_from_file_location("native_missing_models", module_path)
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = module
spec.loader.exec_module(module)

model_type = module.OptionalValue
assert dataclasses.fields(model_type)[0].default is canonical_missing

for skip_validation in (False, True):
    implicit = model_type(skip_validation=skip_validation)
    explicit = model_type(
        value=canonical_missing,
        skip_validation=skip_validation,
    )
    present_false = model_type(value=False, skip_validation=skip_validation)
    present_null = model_type(value=None, skip_validation=skip_validation)
    assert implicit.value is canonical_missing
    assert explicit.value is canonical_missing
    assert copy.deepcopy(implicit).value is canonical_missing
    assert pickle.loads(pickle.dumps(implicit)).value is canonical_missing
    assert dataclasses.asdict(implicit)["value"] is canonical_missing
    assert implicit.to_value(skip_validation=skip_validation) == {}
    assert explicit.to_value(skip_validation=skip_validation) == {}
    assert present_false.to_value(skip_validation=skip_validation) == {"value": False}
    assert present_null.to_value(skip_validation=skip_validation) == {"value": None}

    for from_value in (
        lambda: model_type.from_value(
            {"value": canonical_missing},
            skip_validation=skip_validation,
        ),
        lambda: model_type.deserialize(
            '{"value":"JSONCOMPAT_MISSING"}',
            skip_validation=skip_validation,
        ),
    ):
        try:
            from_value()
        except (TypeError, ValueError):
            pass
        else:
            raise AssertionError("wire construction accepted the missing sentinel")

# Rebinding the public convenience name before the generated module is even
# executed cannot change the runtime-owned identity. The captured singleton
# still means omitted; False remains present.
assert model_type(value=canonical_missing).to_value() == {}
assert model_type(value=dc.JSONCOMPAT_MISSING).to_value() == {"value": False}
"###,
    );
    command.arg(module_path);
    let output = command
        .output()
        .expect("run native missing singleton constructor test");
    assert!(
        output.status.success(),
        "native missing singleton constructor test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn native_unavailable_missing_singleton_is_unforgeable_and_reload_stable() {
    let package_root = write_isolated_jsoncompat_package("fallback_missing_singleton");
    let expected_init = package_root.join("jsoncompat").join("__init__.py");

    let mut command = python_env::python_command();
    command
        .env_remove("JSONCOMPAT_NATIVE_PROFILE")
        .env("PYTHONPATH", &package_root)
        .current_dir(&package_root)
        .arg("-B")
        .arg("-c")
        .arg(
            r###"
import copy
import importlib
import pathlib
import pickle
import sys

import jsoncompat


assert pathlib.Path(jsoncompat.__file__).resolve() == pathlib.Path(sys.argv[1]).resolve()
assert jsoncompat._native_symbols is None
canonical = jsoncompat.JSONCOMPAT_MISSING
missing_type = jsoncompat.JsoncompatMissingType
assert type(canonical) is missing_type
assert canonical is Ellipsis
assert missing_type is type(Ellipsis)

for label, construct in (
    ("object.__new__", lambda: object.__new__(missing_type)),
):
    try:
        construct()
    except TypeError:
        pass
    else:
        raise AssertionError(f"constructed a second fallback missing sentinel via {label}")

try:
    class ForgedMissing(missing_type):
        pass
except TypeError:
    pass
else:
    raise AssertionError("subclassed the fallback missing sentinel type")

assert missing_type() is canonical
assert missing_type.__new__(missing_type) is canonical
assert copy.copy(canonical) is canonical
assert copy.deepcopy(canonical) is canonical
assert pickle.loads(pickle.dumps(canonical)) is canonical

reloaded = importlib.reload(jsoncompat)
assert reloaded._native_symbols is None
assert reloaded.JsoncompatMissingType is missing_type
assert reloaded.JSONCOMPAT_MISSING is canonical
assert copy.copy(canonical) is canonical
assert copy.deepcopy(canonical) is canonical
assert pickle.loads(pickle.dumps(canonical)) is canonical
"###,
        )
        .arg(expected_init);
    let output = command
        .output()
        .expect("run isolated native-unavailable singleton test");
    assert!(
        output.status.success(),
        "isolated native-unavailable singleton test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn generated_huge_integer_literals_round_trip_in_checked_and_trusted_modes() {
    let huge = u64::MAX - 15;
    let source = generate_dataclass_models(&json!({
        "title": "HugeLiteral",
        "const": huge,
    }))
    .expect("generate huge integer literal dataclass");
    let module_path = write_temp_module("huge_integer_literal", &source);

    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
import importlib.util
import sys

module_path = sys.argv[1]
huge = int(sys.argv[2])
spec = importlib.util.spec_from_file_location("huge_literal_models", module_path)
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = module
spec.loader.exec_module(module)

model_type = module.JSONCOMPAT_MODEL
for skip_validation in (False, True):
    value = model_type.from_value(huge, skip_validation=skip_validation)
    assert value.root == huge
    assert value.to_value(skip_validation=skip_validation) == huge
    assert value.serialize(skip_validation=skip_validation) == str(huge)
    decoded = model_type.deserialize(str(huge), skip_validation=skip_validation)
    assert decoded.root == huge
"###,
    );
    command.arg(module_path).arg(huge.to_string());
    let output = command
        .output()
        .expect("run huge integer literal round-trip test");
    assert!(
        output.status.success(),
        "huge integer literal round-trip test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn generated_dataclass_native_slots_reject_same_owner_aliases() {
    let source = generate_dataclass_models(&json!({
        "title": "SlotAlias",
        "type": "object",
        "properties": {
            "left": {"type": "string"},
            "right": {"type": "string"}
        },
        "required": ["left", "right"],
        "additionalProperties": false,
    }))
    .expect("generate same-owner slot alias dataclass");
    let module_path = write_temp_module("same_owner_slot_alias", &source);

    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
import importlib.util
import sys

module_path = sys.argv[1]
spec = importlib.util.spec_from_file_location("same_owner_slot_alias_models", module_path)
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = module
spec.loader.exec_module(module)

module.SlotAlias.left = module.SlotAlias.right
try:
    module.SlotAlias.from_value({"left": "L", "right": "R"})
except TypeError as error:
    assert "aliases member descriptor" in str(error), str(error)
else:
    raise AssertionError("same-owner member descriptor alias was accepted")
"###,
    );
    command.arg(module_path);
    let output = command.output().expect("run same-owner slot alias test");
    assert!(
        output.status.success(),
        "same-owner slot alias test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn checked_output_revalidates_current_state_without_an_exposed_cache_marker() {
    let source = generate_dataclass_models(&json!({
        "title": "MutableEscapeHatch",
        "type": "object",
        "properties": {"value": {"type": "string", "minLength": 1}},
        "required": ["value"],
        "additionalProperties": false,
    }))
    .expect("generate checked-output dataclass");
    let module_path = write_temp_module("checked_output_revalidation", &source);

    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
import importlib.util
import sys

module_path = sys.argv[1]
spec = importlib.util.spec_from_file_location("checked_output_models", module_path)
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = module
spec.loader.exec_module(module)

model_type = module.MutableEscapeHatch
assert all(
    "_jsoncompat_validated" not in getattr(base, "__slots__", ())
    for base in model_type.__mro__
)

trusted_invalid = model_type.from_value({"value": ""}, skip_validation=True)
assert not hasattr(trusted_invalid, "_jsoncompat_validated")
assert trusted_invalid.to_value(skip_validation=True) == {"value": ""}
assert trusted_invalid.serialize(skip_validation=True) == '{"value":""}'
for checked_output in (trusted_invalid.to_value, trusted_invalid.serialize):
    try:
        checked_output()
    except ValueError:
        pass
    else:
        raise AssertionError("checked output trusted an unchecked invalid model")

checked = model_type.from_value({"value": "valid"})
assert checked.to_value() == {"value": "valid"}
assert checked.serialize() == '{"value":"valid"}'
assert not hasattr(checked, "_jsoncompat_validated")

# Frozen dataclasses deliberately expose Python's low-level escape hatch. A
# checked output operation must validate the current graph instead of trusting
# construction-time state that this mutation could stale.
object.__setattr__(checked, "value", "")
assert checked.to_value(skip_validation=True) == {"value": ""}
assert checked.serialize(skip_validation=True) == '{"value":""}'
for checked_output in (checked.to_value, checked.serialize):
    try:
        checked_output()
    except ValueError:
        pass
    else:
        raise AssertionError("checked output trusted stale construction state")
"###,
    );
    command.arg(module_path);
    let output = command
        .output()
        .expect("run checked-output revalidation regression");
    assert!(
        output.status.success(),
        "checked-output revalidation regression failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn output_materialization_rejects_non_json_mutations_before_validation() {
    let source = generate_dataclass_models(&json!({
        "title": "OutputShape",
        "type": "object",
        "properties": {
            "typed": {"type": "string"},
            "literal": {"const": "fixed"},
            "anything": {},
            "nested": {
                "type": "object",
                "properties": {"count": {"type": "integer"}},
                "required": ["count"],
                "additionalProperties": false
            }
        },
        "required": ["typed", "literal", "anything", "nested"],
        "additionalProperties": false,
    }))
    .expect("generate output-shape dataclass");
    let module_path = write_temp_module("output_shape_proof", &source);

    let any_source = generate_dataclass_models(&json!({
        "title": "AnythingRoot"
    }))
    .expect("generate unconstrained root dataclass");
    let any_module_path = write_temp_module("output_any_root_proof", &any_source);

    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
import importlib.util
import sys


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


module = load("output_shape_models", sys.argv[1])
any_module = load("output_any_root_models", sys.argv[2])
payload = {
    "typed": "value",
    "literal": "fixed",
    "anything": {"safe": [1, True, None]},
    "nested": {"count": 1},
}


def fresh():
    return module.OutputShape.from_value(payload)


def rejects_every_output(instance):
    for skip_validation in (False, True):
        for output in (instance.to_value, instance.serialize):
            try:
                output(skip_validation=skip_validation)
            except TypeError:
                pass
            else:
                raise AssertionError(
                    f"{output!r} leaked a non-JSON value with "
                    f"skip_validation={skip_validation}"
                )


for field_name in ("typed", "literal", "anything", "nested"):
    instance = fresh()
    object.__setattr__(instance, field_name, object())
    rejects_every_output(instance)

instance = fresh()
object.__setattr__(instance.nested, "count", object())
rejects_every_output(instance)

instance = fresh()
object.__setattr__(instance.anything, "root", object())
rejects_every_output(instance)

unconstrained = any_module.AnythingRoot.from_value({"safe": True})
object.__setattr__(unconstrained, "root", object())
rejects_every_output(unconstrained)


class Text(str):
    pass


class Number(int):
    pass


instance = fresh()
object.__setattr__(instance, "typed", Text("value"))
object.__setattr__(instance, "literal", Text("fixed"))
object.__setattr__(instance.anything, "root", {Text("key"): Text("value")})
object.__setattr__(instance.nested, "count", Number(1))
for skip_validation in (False, True):
    value = instance.to_value(skip_validation=skip_validation)
    assert type(value["typed"]) is str
    assert type(value["literal"]) is str
    assert type(next(iter(value["anything"]))) is str
    assert type(value["anything"]["key"]) is str
    assert type(value["nested"]["count"]) is int
    assert instance.serialize(skip_validation=skip_validation) == (
        '{"anything":{"key":"value"},"literal":"fixed",'
        '"nested":{"count":1},"typed":"value"}'
    )
"###,
    );
    command.arg(module_path).arg(any_module_path);
    let output = command
        .output()
        .expect("run output JSON materialization proof test");
    assert!(
        output.status.success(),
        "output JSON materialization proof test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn trusted_direct_json_matches_trusted_materialization_for_edge_values() {
    let source = generate_dataclass_models(&json!({
        "title": "TrustedEquivalence",
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "payload": {}
        },
        "required": ["name", "payload"],
        "additionalProperties": {}
    }))
    .expect("generate trusted-writer equivalence dataclass");
    let module_path = write_temp_module("trusted_writer_equivalence", &source);

    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
import importlib.util
import json
import sys

module_path = sys.argv[1]
spec = importlib.util.spec_from_file_location("trusted_writer_models", module_path)
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = module
spec.loader.exec_module(module)


class Text(str):
    pass


huge = 10**80
instance = module.TrustedEquivalence.from_value(
    {
        "name": "declared",
        "payload": {"huge": huge, "nested": [True, None]},
        "dup": 0,
    },
    skip_validation=True,
)

# Exercise canonical string keys, duplicate-key last-write behavior, an extra
# property colliding with a declared property, subclass scalar normalization,
# and integers larger than u64. These mutations model every state the direct
# writer must reconcile the same way as to_value().
object.__setattr__(
    instance.__jsoncompat_extra__,
    "_items",
    (
        ((Text("dup")), 1),
        ("dup", 2),
        ("name", Text("extra-wins")),
        ("huge", huge),
    ),
)

materialized = instance.to_value(skip_validation=True)
wire = instance.serialize(skip_validation=True)
assert json.loads(wire) == materialized
assert materialized["dup"] == 2
assert materialized["name"] == "extra-wins"
assert type(materialized["name"]) is str
assert materialized["huge"] == huge


def assert_trusted_rejects(value, exception):
    for output in (value.to_value, value.serialize):
        try:
            output(skip_validation=True)
        except exception:
            pass
        else:
            raise AssertionError(
                f"trusted output hid an overwritten invalid value: {output!r}"
            )


def with_duplicate_payload(first, second):
    value = module.TrustedEquivalence.from_value(
        {"name": "declared", "payload": {}},
        skip_validation=True,
    )
    object.__setattr__(
        value.payload.root,
        "_items",
        ((Text("duplicate"), first), ("duplicate", second)),
    )
    return value


# Python-dict materialization proves every value before a canonical-key
# collision overwrites it. The direct writer must not let its sorted pending
# map hide either an invalid scalar or an earlier cycle.
assert_trusted_rejects(with_duplicate_payload(object(), "valid"), TypeError)

overwritten_cycle = []
overwritten_cycle.append(overwritten_cycle)
assert_trusted_rejects(with_duplicate_payload(overwritten_cycle, "valid"), ValueError)

# Extra properties intentionally win wire-name collisions, but that precedence
# cannot erase proof of the displaced declared field.
invalid_declared = module.TrustedEquivalence.from_value(
    {"name": "declared", "payload": None},
    skip_validation=True,
)
object.__setattr__(invalid_declared, "name", object())
object.__setattr__(
    invalid_declared.__jsoncompat_extra__,
    "_items",
    (("name", "extra-wins"),),
)
assert_trusted_rejects(invalid_declared, TypeError)

cycle = []
cycle.append(cycle)
object.__setattr__(instance.payload, "root", cycle)
for output in (instance.to_value, instance.serialize):
    try:
        output(skip_validation=True)
    except ValueError:
        pass
    else:
        raise AssertionError(f"trusted output accepted a cycle: {output!r}")
"###,
    );
    command.arg(module_path);
    let output = command
        .output()
        .expect("run trusted direct JSON equivalence test");
    assert!(
        output.status.success(),
        "trusted direct JSON equivalence test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn trusted_and_checked_output_share_one_depth_budget() {
    let source = generate_dataclass_models(&json!({
        "title": "DepthBoundary"
    }))
    .expect("generate unconstrained depth-boundary dataclass");
    let module_path = write_temp_module("output_depth_boundary", &source);

    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
import importlib.util
import sys

module_path = sys.argv[1]
spec = importlib.util.spec_from_file_location("depth_boundary_models", module_path)
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = module
spec.loader.exec_module(module)


def nested_lists(depth):
    value = None
    for _ in range(depth):
        value = [value]
    return value


instance = module.DepthBoundary.from_value(None, skip_validation=True)
# MAX_MODEL_DEPTH is 64. The root and Any nodes consume the first two levels,
# leaving 62 nested containers as the last accepted value.
last_accepted_depth = 62
first_rejected_depth = 63
last_accepted = nested_lists(last_accepted_depth)
first_rejected = nested_lists(first_rejected_depth)

for skip_validation in (False, True):
    object.__setattr__(instance, "root", last_accepted)
    assert instance.to_value(skip_validation=skip_validation) == last_accepted
    assert instance.serialize(skip_validation=skip_validation) == (
        "[" * last_accepted_depth + "null" + "]" * last_accepted_depth
    )

    object.__setattr__(instance, "root", first_rejected)
    for output in (instance.to_value, instance.serialize):
        try:
            output(skip_validation=skip_validation)
        except ValueError:
            pass
        else:
            raise AssertionError(
                f"output exceeded the shared depth budget: {output!r}, "
                f"skip_validation={skip_validation}"
            )
"###,
    );
    command.arg(module_path);
    let output = command
        .output()
        .expect("run output depth-boundary equivalence test");
    assert!(
        output.status.success(),
        "output depth-boundary equivalence test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn generated_dataclasses_normalize_keys_after_scalar_canonicalization() {
    let source = generate_dataclass_models(&json!({
        "title": "CanonicalKeys",
        "type": "object",
        "properties": {"metadata": {}},
        "required": ["metadata"],
        "additionalProperties": {"type": "integer"},
    }))
    .expect("generate key-normalization dataclass");
    let module_path = write_temp_module("canonical_keys", &source);

    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
from collections.abc import Mapping
import importlib.util
import sys

module_path = sys.argv[1]
spec = importlib.util.spec_from_file_location("canonical_key_models", module_path)
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = module
spec.loader.exec_module(module)


class IdentityString(str):
    __hash__ = object.__hash__
    __eq__ = object.__eq__


def duplicate_x(first, second):
    result = {IdentityString("x"): first, IdentityString("x"): second}
    assert len(result) == 2
    return result


value = {"metadata": duplicate_x(1, 2), **duplicate_x(3, 4)}
model = module.CanonicalKeys.from_value(value)
expected = {"metadata": {"x": 2}, "x": 4}
assert model.to_value() == expected
wire = model.serialize()
assert module.CanonicalKeys.deserialize(wire).to_value() == expected


class DuplicateMapping(Mapping):
    def __getitem__(self, key):
        if key == "x":
            return 2
        raise KeyError(key)

    def __iter__(self):
        return iter(("x",))

    def __len__(self):
        return 1

    def items(self):
        return (("x", 1), ("x", 2))


for skip_validation in (False, True):
    raw = DuplicateMapping()
    try:
        module.CanonicalKeys.from_value(
            {"metadata": raw},
            skip_validation=skip_validation,
        )
    except (TypeError, ValueError):
        pass
    else:
        raise AssertionError("raw from_value accepted a non-JSON Mapping")

direct = DuplicateMapping()
stateful = module.CanonicalKeys(
    metadata=module.CanonicalKeysMetadata.from_value({}),
    __jsoncompat_extra__=direct,
    skip_validation=True,
)
assert stateful.to_value(skip_validation=True) == {"x": 2, "metadata": {}}
"###,
    );
    command.arg(module_path);
    let output = command
        .output()
        .expect("run generated dataclass key-normalization test");
    assert!(
        output.status.success(),
        "generated dataclass key-normalization test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn checked_construction_canonicalizes_valid_literal_subclasses() {
    let root_source = generate_dataclass_models(&json!({
        "title": "RootLiteral",
        "type": "string",
        "const": "x",
    }))
    .expect("generate root literal dataclass");
    let nested_source = generate_dataclass_models(&json!({
        "title": "LiteralEnvelope",
        "type": "object",
        "properties": {"kind": {"enum": ["x", "y"]}},
        "required": ["kind"],
        "additionalProperties": false,
    }))
    .expect("generate nested literal dataclass");
    let numeric_source = generate_dataclass_models(&json!({
        "title": "NumericLiteral",
        "enum": [1, 2.5],
    }))
    .expect("generate numeric literal dataclass");
    let boolean_source = generate_dataclass_models(&json!({
        "title": "BooleanLiteral",
        "const": true,
    }))
    .expect("generate boolean literal dataclass");
    let numeric_union_source = generate_dataclass_models(&json!({
        "title": "NumericUnion",
        "anyOf": [
            {"const": 1},
            {"type": "number", "minimum": 2}
        ],
    }))
    .expect("generate numeric literal union dataclass");
    let ambiguous_source = generate_dataclass_models(&json!({
        "title": "SignedValue",
        "oneOf": [
            {
                "title": "NonNegative",
                "type": "object",
                "properties": {"value": {"type": "integer", "minimum": 0}},
                "required": ["value"],
                "additionalProperties": false
            },
            {
                "title": "Negative",
                "type": "object",
                "properties": {"value": {"type": "integer", "maximum": -1}},
                "required": ["value"],
                "additionalProperties": false
            }
        ]
    }))
    .expect("generate ambiguous model union");
    let tuple_union_source = generate_dataclass_models(&json!({
        "title": "TupleUnion",
        "oneOf": [
            {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {"value": {"type": "integer", "minimum": 0}},
                    "required": ["value"],
                    "additionalProperties": false
                }
            },
            {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {"value": {"type": "integer", "maximum": -1}},
                    "required": ["value"],
                    "additionalProperties": false
                }
            }
        ]
    }))
    .expect("generate ambiguous tuple union");
    let root_module_path = write_temp_module("root_literal_subclass", &root_source);
    let nested_module_path = write_temp_module("nested_literal_subclass", &nested_source);
    let numeric_module_path = write_temp_module("numeric_literal_subclass", &numeric_source);
    let boolean_module_path = write_temp_module("boolean_literal", &boolean_source);
    let ambiguous_module_path = write_temp_module("ambiguous_model_union", &ambiguous_source);
    let numeric_union_module_path =
        write_temp_module("numeric_literal_union", &numeric_union_source);
    let tuple_union_module_path = write_temp_module("tuple_union", &tuple_union_source);

    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
import importlib.util
import sys


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


roots = load("root_literal_models", sys.argv[1])
nested = load("nested_literal_models", sys.argv[2])
numeric = load("numeric_literal_models", sys.argv[3])
boolean = load("boolean_literal_models", sys.argv[4])
ambiguous = load("ambiguous_union_models", sys.argv[5])
numeric_union = load("numeric_literal_union_models", sys.argv[6])
tuple_union = load("tuple_union_models", sys.argv[7])


class StringSubclass(str):
    pass


class IntegerSubclass(int):
    pass


class FloatSubclass(float):
    pass


class RaisingIntegerSubclass(int):
    __hash__ = int.__hash__

    def __eq__(self, other):
        raise RuntimeError("subclass equality must not run")


class RaisingTuple(tuple):
    def __iter__(self):
        raise RuntimeError("tuple subclass iteration must not run")


class RaisingList(list):
    def __iter__(self):
        raise RuntimeError("list subclass iteration must not run")


for skip_validation in (False, True):
    exact_root = roots.JSONCOMPAT_MODEL.from_value(
        "x",
        skip_validation=skip_validation,
    )
    assert exact_root.root == "x"
    assert type(exact_root.root) is str
    root = roots.JSONCOMPAT_MODEL.from_value(
        StringSubclass("x"),
        skip_validation=skip_validation,
    )
    assert root.root == "x"
    assert type(root.root) is str
    envelope = nested.LiteralEnvelope.from_value(
        {"kind": StringSubclass("x")},
        skip_validation=skip_validation,
    )
    assert envelope.kind == "x"
    assert type(envelope.kind) is str

    exact_integer = numeric.JSONCOMPAT_MODEL.from_value(
        1,
        skip_validation=skip_validation,
    )
    assert exact_integer.root == 1
    assert type(exact_integer.root) is int
    exact_float = numeric.JSONCOMPAT_MODEL.from_value(
        2.5,
        skip_validation=skip_validation,
    )
    assert exact_float.root == 2.5
    assert type(exact_float.root) is float
    integer = numeric.JSONCOMPAT_MODEL.from_value(
        IntegerSubclass(1),
        skip_validation=skip_validation,
    )
    assert integer.root == 1
    assert type(integer.root) is int
    float_value = numeric.JSONCOMPAT_MODEL.from_value(
        FloatSubclass(2.5),
        skip_validation=skip_validation,
    )
    assert float_value.root == 2.5
    assert type(float_value.root) is float
    json_equal_number = numeric.JSONCOMPAT_MODEL.from_value(
        FloatSubclass(1.0),
        skip_validation=skip_validation,
    )
    assert json_equal_number.root == 1
    assert type(json_equal_number.root) in (int, float)

    union_value = numeric_union.JSONCOMPAT_MODEL.from_value(
        RaisingIntegerSubclass(1),
        skip_validation=skip_validation,
    )
    assert union_value.root == 1
    assert type(union_value.root) is int

    boolean_value = boolean.JSONCOMPAT_MODEL.from_value(
        True,
        skip_validation=skip_validation,
    )
    assert boolean_value.root is True
    for non_boolean in (1, 1.0):
        try:
            boolean.JSONCOMPAT_MODEL.from_value(
                non_boolean,
                skip_validation=skip_validation,
            )
        except (TypeError, ValueError):
            pass
        else:
            raise AssertionError("boolean literal accepted a JSON number")

checked_negative = ambiguous.JSONCOMPAT_MODEL.from_value({"value": -1})
assert checked_negative.to_value() == {"value": -1}
assert type(checked_negative.root) is ambiguous.SignedValueBranch1
checked_positive = ambiguous.JSONCOMPAT_MODEL.from_value({"value": 1})
assert checked_positive.to_value() == {"value": 1}
assert type(checked_positive.root) is ambiguous.SignedValueBranch0
trusted_positive = ambiguous.JSONCOMPAT_MODEL.from_value(
    {"value": 1},
    skip_validation=True,
)
assert trusted_positive.to_value(skip_validation=True) == {"value": 1}

for Container in (RaisingTuple, RaisingList):
    checked_array = tuple_union.JSONCOMPAT_MODEL.from_value(
        Container(({"value": 1},))
    )
    assert type(checked_array.root[0]) is tuple_union.TupleUnionItem
    trusted_array = tuple_union.JSONCOMPAT_MODEL.from_value(
        Container(({"value": 1},)),
        skip_validation=True,
    )
    assert type(trusted_array.root[0]) is tuple_union.TupleUnionItem2
"###,
    );
    command
        .arg(root_module_path)
        .arg(nested_module_path)
        .arg(numeric_module_path)
        .arg(boolean_module_path)
        .arg(ambiguous_module_path)
        .arg(numeric_union_module_path)
        .arg(tuple_union_module_path);
    let output = command
        .output()
        .expect("run generated dataclass literal-subclass test");
    assert!(
        output.status.success(),
        "generated dataclass literal-subclass test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn checked_candidate_construction_preserves_control_flow_exceptions() {
    let source = generate_dataclass_models(&json!({
        "title": "InterruptEnvelope",
        "type": "object",
        "properties": {"metadata": {}},
        "required": ["metadata"],
        "additionalProperties": false,
    }))
    .expect("generate interrupt dataclass");
    let module_path = write_temp_module("candidate_keyboard_interrupt", &source);

    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
from collections.abc import Mapping
import importlib.util
import sys

module_path = sys.argv[1]
spec = importlib.util.spec_from_file_location("interrupt_models", module_path)
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = module
spec.loader.exec_module(module)


class RaisingMapping(Mapping):
    def __init__(self, exception):
        self.exception = exception
        self.items_calls = 0

    def __getitem__(self, key):
        raise KeyError(key)

    def __iter__(self):
        return iter(())

    def __len__(self):
        return 0

    def items(self):
        self.items_calls += 1
        raise self.exception("user sentinel")


for exception in (TypeError, ValueError, KeyboardInterrupt):
    checked = RaisingMapping(exception)
    try:
        module.InterruptEnvelope.from_value({"metadata": checked})
    except ValueError:
        pass
    else:
        raise AssertionError("checked construction accepted a non-JSON Mapping")
    assert checked.items_calls == 0

    unchecked = RaisingMapping(exception)
    try:
        module.InterruptEnvelope.from_value(
            {"metadata": unchecked},
            skip_validation=True,
        )
    except TypeError:
        pass
    else:
        raise AssertionError("unchecked construction accepted a non-JSON Mapping")
    assert unchecked.items_calls == 0
"###,
    );
    command.arg(module_path);
    let output = command
        .output()
        .expect("run generated dataclass control-flow exception test");
    assert!(
        output.status.success(),
        "generated dataclass control-flow exception test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn direct_union_construction_does_not_retry_hard_failures() {
    let source = generate_dataclass_models(&json!({
        "title": "DirectUnionEnvelope",
        "type": "object",
        "properties": {
            "payload": {
                "anyOf": [
                    {
                        "type": "array",
                        "items": {"type": "integer"}
                    },
                    {"type": "integer"}
                ]
            }
        },
        "required": ["payload"],
        "additionalProperties": false,
    }))
    .expect("generate direct union dataclass");
    let module_path = write_temp_module("direct_union_hard_failure", &source);

    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
import importlib.util
import sys
from collections.abc import Sequence

module_path = sys.argv[1]
spec = importlib.util.spec_from_file_location("direct_union_models", module_path)
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = module
spec.loader.exec_module(module)

class HostileSequence(Sequence):
    def __len__(self):
        return 1

    def __getitem__(self, index):
        raise RuntimeError("hard branch failure")


try:
    module.DirectUnionEnvelope(payload=HostileSequence(), skip_validation=True)
except RuntimeError as error:
    assert str(error) == "hard branch failure"
else:
    raise AssertionError("direct union construction swallowed a hard failure")

try:
    module.DirectUnionEnvelope(payload={}, skip_validation=True)
except TypeError as error:
    message = str(error)
    assert "does not match any generated model union branch" in message, message
    assert "expected sequence, got dict" in message, message
else:
    raise AssertionError("direct union construction accepted an unmatched value")
"###,
    );
    command.arg(module_path);
    let output = command
        .output()
        .expect("run direct union hard-failure test");
    assert!(
        output.status.success(),
        "direct union hard-failure test failed: {}",
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
fn native_plan_only_represents_string_keyed_json_mappings() {
    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
from collections.abc import Mapping, Sequence
import typing

from jsoncompat.codegen import dataclasses as dc


builder = dc._NativePlanBuilder({})
assert builder.add(Mapping[str, int]) == 0
assert builder.finish() == [("dict", 1), ("int",)]

builder = dc._NativePlanBuilder({})
assert builder.add(Sequence[int]) == 0
assert builder.finish() == [("list", 1), ("int",)]

builder = dc._NativePlanBuilder({})
assert builder.add(int | str) == 0
assert builder.finish() == [
    ("union", (1, 2), None, None),
    ("int",),
    ("str",),
]

# Generated `Literal[...] | Literal[...]` source evaluates to typing.Union,
# so this is part of the generated contract rather than a compatibility alias.
builder = dc._NativePlanBuilder({})
assert builder.add(typing.Literal["x"] | typing.Literal["y"]) == 0
assert builder.finish()[0][0] == "union"

for annotation, message in (
    (Mapping[int, str], "JSON mappings must have string keys"),
    (
        dc.JsoncompatMissingType,
        "JsoncompatMissingType is only valid in an omittable field",
    ),
):
    try:
        dc._NativePlanBuilder({}).add(annotation)
    except dc._NativePlanUnsupported as error:
        assert str(error) == message
    else:
        raise AssertionError(f"native plan accepted {annotation!r}")

for annotation in (
    list,
    dict,
    Sequence,
    Mapping,
    list[int],
    dict[str, int],
    typing.List[int],
    typing.Dict[str, int],
    typing.Sequence[int],
    typing.Mapping[str, int],
):
    try:
        dc._NativePlanBuilder({}).add(annotation)
    except dc._NativePlanUnsupported:
        pass
    else:
        raise AssertionError(f"native plan accepted fallback annotation {annotation!r}")
"###,
    );
    let output = command
        .output()
        .expect("run native plan JSON mapping invariant test");
    assert!(
        output.status.success(),
        "native plan JSON mapping invariant test failed: {}",
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
fn cyclic_values_reject_across_the_complete_graph_on_a_constrained_thread_stack() {
    let recursive_source = generate_dataclass_models(&json!({
        "title": "CycleEnvelope",
        "type": "object",
        "properties": {
            "children": {
                "type": "array",
                "items": {"$ref": "#"},
            },
            "payload": {},
        },
        "required": ["children", "payload"],
        "additionalProperties": false,
    }))
    .expect("generate recursive cycle-test dataclass");
    let recursive_module_path =
        write_temp_module("cyclic_recursive_small_stack", &recursive_source);

    let list_source = generate_dataclass_models(&json!({
        "title": "NestedList",
        "type": "array",
        "items": {
            "type": "array",
            "items": {},
        },
    }))
    .expect("generate typed-list cycle-test dataclass");
    let list_module_path = write_temp_module("cyclic_list_small_stack", &list_source);

    let mapping_source = generate_dataclass_models(&json!({
        "title": "OpenMapping",
        "type": "object",
        "additionalProperties": {},
    }))
    .expect("generate mapping cycle-test dataclass");
    let mapping_module_path = write_temp_module("cyclic_mapping_small_stack", &mapping_source);

    // Local references intentionally keep adjacent type unions intact during
    // canonicalization. Their object arm is the supported schema path which
    // emits an ordinary Mapping node rather than an object dataclass.
    let ordinary_mapping_source = generate_dataclass_models(&json!({
        "title": "MappingOrString",
        "$defs": {
            "label": {"type": "string"},
        },
        "type": ["object", "string"],
        "properties": {
            "label": {"$ref": "#/$defs/label"},
        },
    }))
    .expect("generate ordinary mapping cycle-test dataclass");
    assert!(
        ordinary_mapping_source
            .contains("root: (collections.abc.Mapping[str, typing.Any] | str) = dc.root_field()")
    );
    let ordinary_mapping_module_path = write_temp_module(
        "cyclic_ordinary_mapping_small_stack",
        &ordinary_mapping_source,
    );

    let any_source = generate_dataclass_models(&json!({
        "title": "AnyRoot",
    }))
    .expect("generate any-root cycle-test dataclass");
    let any_module_path = write_temp_module("cyclic_any_small_stack", &any_source);

    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
import importlib.util
import sys
import threading
import traceback
from collections.abc import Sequence

from jsoncompat.codegen.dataclasses import SerializationFormat


try:
    # 256 KiB remains deliberately constrained while leaving enough room for
    # CPython imports and per-thread native-plan initialization on every CI
    # platform. The cycle assertions below still require immediate detection
    # rather than eventual depth exhaustion.
    threading.stack_size(262144)
except (RuntimeError, ValueError):
    raise SystemExit(0)

EXPECTED = "cyclic containers are not JSON values"


def import_module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def expect_cycle(callback):
    try:
        callback()
    except ValueError as error:
        assert str(error) == EXPECTED, str(error)
    else:
        raise AssertionError("cyclic JSON value was accepted")


class CyclicSequence(Sequence):
    def __len__(self):
        return 1

    def __getitem__(self, index):
        if index == 0:
            return self
        raise IndexError(index)


def exercise_cycles():
    recursive_module = import_module("cyclic_recursive_models", sys.argv[1])
    list_module = import_module("cyclic_list_models", sys.argv[2])
    mapping_module = import_module("cyclic_mapping_models", sys.argv[3])
    ordinary_mapping_module = import_module(
        "cyclic_ordinary_mapping_models",
        sys.argv[4],
    )
    any_module = import_module("cyclic_any_models", sys.argv[5])

    recursive_model = recursive_module.JSONCOMPAT_MODEL
    list_model = list_module.JSONCOMPAT_MODEL
    mapping_model = mapping_module.JSONCOMPAT_MODEL
    ordinary_mapping_model = ordinary_mapping_module.JSONCOMPAT_MODEL
    any_model = any_module.JSONCOMPAT_MODEL

    # Module setup and runtime-plan compilation must not be covered by the
    # expected-error assertions below.
    recursive_model.from_value(
        {"children": [], "payload": None},
        skip_validation=True,
    )
    list_model(root=[[]], skip_validation=True)
    mapping_model(__jsoncompat_extra__={"ready": True}, skip_validation=True)
    ordinary_mapping_model(root={"ready": True}, skip_validation=True)

    typed_cycle = {"children": [], "payload": None}
    typed_cycle["children"].append(typed_cycle)
    mixed_cycle = {"children": [], "payload": None}
    mixed_cycle["payload"] = mixed_cycle
    direct_list_cycle = []
    direct_list_cycle.append(direct_list_cycle)
    direct_mapping_cycle = {}
    direct_mapping_cycle["self"] = direct_mapping_cycle

    for skip_validation in (False, True):
        expect_cycle(
            lambda skip_validation=skip_validation: recursive_model.from_value(
                typed_cycle,
                skip_validation=skip_validation,
            )
        )
        expect_cycle(
            lambda skip_validation=skip_validation: recursive_model.from_value(
                mixed_cycle,
                skip_validation=skip_validation,
            )
        )

    expect_cycle(
        lambda: list_model(root=direct_list_cycle, skip_validation=True)
    )
    expect_cycle(
        lambda: list_model(root=CyclicSequence(), skip_validation=True)
    )
    expect_cycle(
        lambda: mapping_model(
            __jsoncompat_extra__=direct_mapping_cycle,
            skip_validation=True,
        )
    )
    expect_cycle(
        lambda: ordinary_mapping_model(
            root=direct_mapping_cycle,
            skip_validation=True,
        )
    )

    shared_child = []
    shared_mapping = {"left": shared_child, "right": shared_child}
    shared = mapping_model(
        __jsoncompat_extra__=shared_mapping,
        skip_validation=True,
    )
    assert shared.to_value(skip_validation=True) == shared_mapping

    output_cycle = []
    output_cycle.append(output_cycle)
    cyclic_output = any_model.from_value(None, skip_validation=True)
    object.__setattr__(cyclic_output, "root", output_cycle)
    for output in (
        lambda: cyclic_output.to_value(skip_validation=True),
        lambda: cyclic_output.to_value(),
        lambda: cyclic_output.serialize(),
        lambda: cyclic_output.serialize(format=SerializationFormat.YAML),
        lambda: cyclic_output.serialize(format=SerializationFormat.MSGPACK),
    ):
        expect_cycle(output)

    shared_output = []
    acyclic_output = any_model.from_value(None, skip_validation=True)
    object.__setattr__(
        acyclic_output,
        "root",
        {"left": shared_output, "right": shared_output},
    )
    assert acyclic_output.to_value(skip_validation=True) == {
        "left": [],
        "right": [],
    }


thread_errors = []
original_excepthook = threading.excepthook


def capture_thread_error(args):
    thread_errors.append(
        "".join(
            traceback.format_exception(
                args.exc_type,
                args.exc_value,
                args.exc_traceback,
            )
        )
    )


threading.excepthook = capture_thread_error
try:
    thread = threading.Thread(target=exercise_cycles)
    thread.start()
    thread.join(10)
    assert not thread.is_alive(), "cyclic construction thread did not finish"
finally:
    threading.excepthook = original_excepthook

assert thread_errors == [], thread_errors
"###,
    );
    command
        .arg(recursive_module_path)
        .arg(list_module_path)
        .arg(mapping_module_path)
        .arg(ordinary_mapping_module_path)
        .arg(any_module_path);
    let output = command
        .output()
        .expect("run complete-graph cycle small-stack subprocess test");
    assert!(
        output.status.success(),
        "complete-graph cycle small-stack subprocess failed with {:?}: {}",
        output.status.code(),
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
    '{"sku":"abc","quantity":3,"metadata":{"x":1,"x":2}}',
    '{"sku":"abc","quantity":3,"metadata":{"a":1,"b":2,"c":3,"a":4}}',
    '{"sku":"abc","quantity":1e999}',
    '{"sku":"abc","quantity":NaN}',
)
for skip_validation in (False, True):
    for payload in invalid_json_payloads:
        try:
            model.deserialize(payload, skip_validation=skip_validation)
        except (TypeError, ValueError):
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
for skip_validation in (False, True):
    try:
        model.from_value(mapping_value, skip_validation=skip_validation)
    except (TypeError, ValueError):
        pass
    else:
        raise AssertionError("raw construction accepted a non-JSON Mapping")
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
fn reusable_python_runtimes_are_thread_local_and_safe_across_threads() {
    let source = generate_dataclass_models(&json!({
        "title": "ThreadedProfile",
        "type": "object",
        "properties": {"name": {"type": "string"}},
        "required": ["name"],
        "additionalProperties": false,
    }))
    .expect("generate threaded runtime dataclass");
    let module_path = write_temp_module("thread_local_runtime", &source);

    let mut command = python_env::python_command();
    command.arg("-B").arg("-c").arg(
        r###"
import importlib.util
import json
import sys
import threading

import jsoncompat


def import_models(name):
    spec = importlib.util.spec_from_file_location(name, sys.argv[1])
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


schema = '{"type":"object","properties":{"name":{"type":"string"}},"required":["name"],"additionalProperties":false}'
validator = jsoncompat.validator_for(schema)
generator = jsoncompat.generator_for(schema)
models = import_models("threaded_models_first_use")
model_type = models.JSONCOMPAT_MODEL

# Establish native state on thread A, then reuse every public object on B.
main_instance = model_type.from_value({"name": "main"})
assert validator.is_valid_json('{"name":"main"}')
assert validator.is_valid_value({"name": "main"})
assert validator.parse_json('{"name":"main"}')[0]
assert validator.serialize_json({"name": "main"}) == '{"name":"main"}'
assert validator.is_valid_json(generator.generate_value(2))

errors = []


def cross_thread_use():
    try:
        assert main_instance.to_value() == {"name": "main"}
        value = model_type.deserialize('{"name":"worker"}')
        assert value.serialize() == '{"name":"worker"}'
        assert validator.is_valid_value({"name": "worker"})
        assert validator.is_valid_json(generator.generate_value(2))
    except BaseException as error:
        errors.append(error)


thread = threading.Thread(target=cross_thread_use)
thread.start()
thread.join(10)
assert not thread.is_alive()
assert errors == [], errors

# A fresh generated module has no bound runtime. Several threads may race its
# first use; each must receive native state owned by that thread.
concurrent_models = import_models("threaded_models_concurrent_first_use")
concurrent_type = concurrent_models.JSONCOMPAT_MODEL
barrier = threading.Barrier(4)


def concurrent_first_use(index):
    try:
        barrier.wait()
        value = concurrent_type.from_value({"name": f"worker-{index}"})
        assert value.to_value() == {"name": f"worker-{index}"}
        assert validator.is_valid_json(value.serialize())
        assert validator.is_valid_json(generator.generate_value(2))
    except BaseException as error:
        errors.append(error)


threads = [threading.Thread(target=concurrent_first_use, args=(index,)) for index in range(4)]
for thread in threads:
    thread.start()
for thread in threads:
    thread.join(10)
assert all(not thread.is_alive() for thread in threads)
assert errors == [], errors
"###,
    );
    command.arg(module_path);
    let output = command
        .output()
        .expect("run thread-local reusable runtime test");
    assert!(
        output.status.success(),
        "thread-local reusable runtime test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn generated_dataclasses_allow_the_former_cache_name_as_a_json_property() {
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
assert model._jsoncompat_validated is False
assert not hasattr(model, "_jsoncompat_validated_")
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

fn write_isolated_jsoncompat_package(test_name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "jsoncompat-isolated-package-{test_name}-{}-{unique}",
        std::process::id(),
    ));
    let package = root.join("jsoncompat");
    fs::create_dir_all(&package).expect("create isolated jsoncompat package directory");
    let source = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("pybindings")
            .join("jsoncompat")
            .join("__init__.py"),
    )
    .expect("read jsoncompat package initializer");
    fs::write(package.join("__init__.py"), source)
        .expect("write isolated jsoncompat package initializer");
    root
}
