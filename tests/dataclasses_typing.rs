#[path = "support/python_env.rs"]
mod python_env;

use json_schema_ast::SchemaDocument;
use jsoncompat_codegen::{generate_dataclass_models, generate_dataclass_module_from_document};
use serde_json::json;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn generated_dataclasses_typecheck_and_expose_precise_field_types() -> Result<(), Box<dyn Error>> {
    let schema = json!({
        "title": "InventoryItem",
        "type": "object",
        "properties": {
            "sku": {"type": "string"},
            "quantity": {"type": "integer"},
            "metadata": {
                "type": "object",
                "properties": {
                    "warehouse": {"type": "string"},
                },
                "required": ["warehouse"],
                "additionalProperties": false,
            },
            "tags": {
                "type": "array",
                "items": {"type": "string"},
            },
            "tagGroups": {
                "type": "array",
                "items": {
                    "type": "array",
                    "items": {"type": "string"},
                },
            },
            "warehouseCode": {
                "$ref": "#/$defs/warehouseCode",
            },
            "coordinates": {
                "type": "array",
                "prefixItems": [
                    {"type": "string"},
                    {"type": "integer"},
                ],
                "items": false,
            },
            "fixedCoordinates": {
                "type": "array",
                "prefixItems": [
                    {"type": "string"},
                    {"type": "integer"},
                ],
                "items": false,
                "minItems": 2,
            },
        },
        "required": ["sku", "metadata"],
        "additionalProperties": {"type": "number"},
        "$defs": {
            "warehouseCode": {
                "type": "string",
            }
        }
    });
    let module = generate_dataclass_module_from_document(&SchemaDocument::from_json(&schema)?)?;
    let source = module.public_file("_generated_impl");
    let work_dir = write_typecheck_files(
        &source,
        r#"

from typing import Literal, Mapping, Sequence, assert_type

from generated_models import InventoryItem, InventoryItemMetadata, JSONCOMPAT_MODEL
from jsoncompat import JsonValue
from jsoncompat.codegen import SerializationFormat
from jsoncompat.codegen.dataclasses import FrozenList, MISSING, MissingType, Omittable


item = InventoryItem.from_value({
    "sku": "sku-123",
    "metadata": {"warehouse": "west"},
    "quantity": 10,
    "tags": ["fresh", "boxed"],
    "priority": 1,
})

assert_type(JSONCOMPAT_MODEL, type[InventoryItem])
assert_type(item, InventoryItem)
assert_type(item.to_value(), JsonValue)
assert_type(item.serialize(), str)
assert_type(item.serialize(format=SerializationFormat.YAML), str)
assert_type(item.serialize(format=SerializationFormat.MSGPACK), bytes)
assert_type(InventoryItem.deserialize(item.serialize()), InventoryItem)
assert_type(item.sku, str)
assert_type(item.quantity, Omittable[int])
assert_type(item.metadata, InventoryItemMetadata)
assert_type(item.metadata.warehouse, str)
assert_type(item.tags, Omittable[Sequence[str]])
assert_type(item.tagGroups, Omittable[Sequence[Sequence[str]]])
assert_type(item.warehouseCode, Omittable[str])
assert_type(item.coordinates, Omittable[Sequence[int | str]])
assert_type(item.fixedCoordinates, Omittable[tuple[str, int]])
if item.fixedCoordinates is not MISSING:
    assert_type(item.fixedCoordinates[0], str)
    assert_type(item.fixedCoordinates[1], int)
assert_type(item.__jsoncompat_extra__, Mapping[str, int | float])
priority = item.get_additional_property("priority")
assert_type(priority, int | float | MissingType)
if priority is not MISSING:
    assert_type(priority, int | float)
    if isinstance(priority, float):
        assert_type(priority.hex(), str)
assert_type(MISSING, Literal[MissingType.MISSING])

minimal = InventoryItem(sku="minimal", metadata=InventoryItemMetadata(warehouse="east"))
assert_type(minimal, InventoryItem)

item_from_constructor = InventoryItem(
    sku="sku-456",
    metadata=InventoryItemMetadata(warehouse="east"),
    quantity=MISSING,
    tags=["dry"],
    tagGroups=[["list"], ("tuple",), FrozenList(["frozen"])],
    warehouseCode="WH-123",
    coordinates=["aisle", 7],
    fixedCoordinates=("aisle", 7),
    __jsoncompat_extra__={"priority": 2.5},
    skip_validation=True,
)
assert_type(item_from_constructor, InventoryItem)
assert_type(
    InventoryItem(sku="tuple", metadata=item.metadata, tags=("dry",)),
    InventoryItem,
)
assert_type(
    InventoryItem(sku="frozen", metadata=item.metadata, tags=FrozenList(["dry"])),
    InventoryItem,
)


def migrate_tags(values: Sequence[str]) -> InventoryItem:
    return InventoryItem(sku="sequence", metadata=item.metadata, tags=list(values))


def migrate_tag_groups(values: Sequence[Sequence[str]]) -> InventoryItem:
    return InventoryItem(
        sku="groups",
        metadata=item.metadata,
        tagGroups=tuple(tuple(group) for group in values),
    )


assert_type(migrate_tags(("migrated",)), InventoryItem)
assert_type(migrate_tag_groups((("migrated",),)), InventoryItem)
assert_type(
    InventoryItem.from_value(item.to_value(), skip_validation=True),
    InventoryItem,
)
"#,
        r#"

from typing import Sequence

from generated_models import InventoryItem, InventoryItemMetadata
from jsoncompat.codegen.dataclasses import MissingType


class ForgedMissing(MissingType):
    pass


InventoryItem(
    sku=123,
    metadata=InventoryItemMetadata(warehouse="east"),
)

InventoryItem(
    sku="sku-456",
    metadata=InventoryItemMetadata(warehouse=123),
)

InventoryItemMetadata(warehouse="east").get_additional_property("priority")

InventoryItem(
    sku="sku-456",
    metadata=InventoryItemMetadata(warehouse="east"),
    coordinates=[{}],
)

InventoryItem(
    sku="sku-456",
    metadata=InventoryItemMetadata(warehouse="east"),
    fixedCoordinates=(7, "aisle"),
)

InventoryItem(
    sku="sku-456",
    metadata=InventoryItemMetadata(warehouse="east"),
    fixedCoordinates=("aisle",),
)

InventoryItem(
    sku="sku-456",
    metadata=InventoryItemMetadata(warehouse="east"),
    skip_validation="yes",
)

InventoryItem.from_value(
    {"sku": "sku-456", "metadata": {"warehouse": "east"}},
    skip_validation="yes",
)

InventoryItem(
    sku="sku-456",
    metadata=InventoryItemMetadata(warehouse="east"),
).serialize(format="json")


def invalid_array_inputs(
    string_array: str,
    bytes_array: bytes,
    broad_strings: Sequence[str],
    broad_integers: Sequence[int],
    broad_inner: Sequence[str],
) -> None:
    metadata = InventoryItemMetadata(warehouse="east")
    InventoryItem(sku="str", metadata=metadata, tags=string_array)
    InventoryItem(sku="bytes", metadata=metadata, coordinates=bytes_array)
    InventoryItem(sku="strings", metadata=metadata, tags=broad_strings)
    InventoryItem(sku="integers", metadata=metadata, coordinates=broad_integers)
    InventoryItem(sku="nested", metadata=metadata, tagGroups=[broad_inner])
"#,
    )?;

    fs::write(work_dir.join("_generated_impl.py"), module.private_file())?;
    let generated_output = run_ty(&work_dir, "generated_models.py")?;
    assert!(
        generated_output.status.success(),
        "{}",
        String::from_utf8_lossy(&generated_output.stdout)
    );
    let valid_output = run_ty(&work_dir, "valid_usage.py")?;
    assert!(
        valid_output.status.success(),
        "ty rejected valid generated dataclass usage:\n{}\n{}",
        String::from_utf8_lossy(&valid_output.stdout),
        String::from_utf8_lossy(&valid_output.stderr),
    );

    let invalid_output = run_ty(&work_dir, "invalid_usage.py")?;
    assert!(
        !invalid_output.status.success(),
        "ty accepted invalid generated dataclass usage"
    );
    let invalid_stdout = String::from_utf8_lossy(&invalid_output.stdout);
    assert!(
        invalid_stdout.contains("sku") && invalid_stdout.contains("str"),
        "ty failure did not report the sku type mismatch:\n{invalid_stdout}",
    );
    assert!(
        invalid_stdout.contains("warehouse") && invalid_stdout.contains("str"),
        "ty failure did not report the nested warehouse type mismatch:\n{invalid_stdout}",
    );
    assert!(
        invalid_stdout.contains("MissingType")
            && invalid_stdout.contains("subclass-of-final-class"),
        "ty failure did not reject subclassing the final missing singleton type:\n{invalid_stdout}",
    );
    for rejected in [
        "coordinates",
        "fixedCoordinates",
        "skip_validation",
        "get_additional_property",
        "no-matching-overload",
        "string_array",
        "bytes_array",
        "broad_strings",
        "broad_integers",
        "broad_inner",
    ] {
        assert!(
            invalid_stdout.contains(rejected),
            "ty did not reject {rejected}:\n{invalid_stdout}"
        );
    }

    fs::write(
        work_dir.join("invalid_number_usage.py"),
        r#"
from generated_models import InventoryItem
from jsoncompat.codegen.dataclasses import MISSING


def number_hex(item: InventoryItem) -> None:
    priority = item.get_additional_property("priority")
    if priority is not MISSING:
        priority.hex()
"#,
    )?;
    let invalid_number_output = run_ty(&work_dir, "invalid_number_usage.py")?;
    assert!(
        !invalid_number_output.status.success(),
        "ty accepted a float-only method on a stored number"
    );
    let invalid_number_stdout = String::from_utf8_lossy(&invalid_number_output.stdout);
    assert!(
        invalid_number_stdout.contains("unresolved-attribute")
            && invalid_number_stdout.contains("hex"),
        "ty did not reject the float-only method:\n{invalid_number_stdout}"
    );

    Ok(())
}

#[test]
fn deserialization_input_types_follow_the_selected_format() -> Result<(), Box<dyn Error>> {
    let schema = json!({
        "title": "Message",
        "type": "object",
        "properties": {"value": {"type": "string"}},
        "required": ["value"],
        "additionalProperties": false,
    });
    let module = generate_dataclass_module_from_document(&SchemaDocument::from_json(&schema)?)?;
    let source = module.public_file("_generated_impl");
    let work_dir = write_typecheck_files(
        &source,
        r#"
from typing import Literal, assert_type

from generated_models import Message
from jsoncompat import JsonValue
from jsoncompat.codegen import SerializationFormat
from jsoncompat.codegen.serialization import deserialize_value


json_text = '{"value":"hello"}'
yaml_text = "value: hello"
msgpack_bytes = b"\x81\xa5value\xa5hello"

assert_type(Message.deserialize(json_text), Message)
assert_type(
    Message.deserialize(json_text.encode(), format=SerializationFormat.JSON),
    Message,
)
assert_type(Message.deserialize(yaml_text, format=SerializationFormat.YAML), Message)
assert_type(
    Message.deserialize(yaml_text.encode(), format=SerializationFormat.YAML),
    Message,
)
assert_type(
    Message.deserialize(msgpack_bytes, format=SerializationFormat.MSGPACK),
    Message,
)

assert_type(deserialize_value(json_text), JsonValue)
assert_type(
    deserialize_value(json_text.encode(), format=SerializationFormat.JSON),
    JsonValue,
)
assert_type(deserialize_value(yaml_text, format=SerializationFormat.YAML), JsonValue)
assert_type(
    deserialize_value(yaml_text.encode(), format=SerializationFormat.YAML),
    JsonValue,
)
assert_type(
    deserialize_value(msgpack_bytes, format=SerializationFormat.MSGPACK),
    JsonValue,
)


def decode_text(
    payload: str,
    format: Literal[SerializationFormat.JSON, SerializationFormat.YAML],
) -> None:
    assert_type(Message.deserialize(payload, format=format), Message)
    assert_type(deserialize_value(payload, format=format), JsonValue)


def decode_bytes(payload: bytes, format: SerializationFormat) -> None:
    assert_type(Message.deserialize(payload, format=format), Message)
    assert_type(deserialize_value(payload, format=format), JsonValue)
"#,
        r#"
from generated_models import Message
from jsoncompat.codegen import SerializationFormat

Message.deserialize('{"value":"hello"}', format=SerializationFormat.MSGPACK)
"#,
    )?;
    fs::write(work_dir.join("_generated_impl.py"), module.private_file())?;
    for (file_name, usage) in [
        (
            "invalid_value_msgpack_text.py",
            r#"
from jsoncompat.codegen import SerializationFormat
from jsoncompat.codegen.serialization import deserialize_value

deserialize_value("{}", format=SerializationFormat.MSGPACK)
"#,
        ),
        (
            "invalid_model_dynamic_text.py",
            r#"
from generated_models import Message
from jsoncompat.codegen import SerializationFormat


def decode(format: SerializationFormat) -> None:
    Message.deserialize('{"value":"hello"}', format=format)
"#,
        ),
        (
            "invalid_value_dynamic_text.py",
            r#"
from jsoncompat.codegen import SerializationFormat
from jsoncompat.codegen.serialization import deserialize_value


def decode(format: SerializationFormat) -> None:
    deserialize_value("{}", format=format)
"#,
        ),
    ] {
        fs::write(work_dir.join(file_name), usage)?;
    }

    let valid_output = run_ty(&work_dir, "valid_usage.py")?;
    assert!(
        valid_output.status.success(),
        "ty rejected valid deserialization formats:\n{}\n{}",
        String::from_utf8_lossy(&valid_output.stdout),
        String::from_utf8_lossy(&valid_output.stderr),
    );
    for file_name in [
        "invalid_usage.py",
        "invalid_value_msgpack_text.py",
        "invalid_model_dynamic_text.py",
        "invalid_value_dynamic_text.py",
    ] {
        let invalid_output = run_ty(&work_dir, file_name)?;
        assert!(
            !invalid_output.status.success(),
            "ty accepted text for a possibly binary format in {file_name}"
        );
        let invalid_stdout = String::from_utf8_lossy(&invalid_output.stdout);
        assert!(
            invalid_stdout.contains("no-matching-overload"),
            "ty did not report the format input mismatch in {file_name}:\n{invalid_stdout}"
        );
    }

    Ok(())
}

#[test]
fn unconstrained_json_values_have_precise_immutable_types() -> Result<(), Box<dyn Error>> {
    let module =
        generate_dataclass_module_from_document(&SchemaDocument::from_json(&json!(true))?)?;
    let source = module.public_file("_generated_impl");
    assert!(!source.contains("typing.Any"));
    assert!(source.contains("root: dc.FrozenJsonValue = dc.root_field()"));
    assert!(source.contains("root: (dc.JsonValue | dc.FrozenJsonValue),"));
    let work_dir = write_typecheck_files(
        &source,
        r#"
from typing import assert_type

from generated_models import GeneratedSchema
from jsoncompat import JsonValue
from jsoncompat.codegen.dataclasses import FrozenDict, FrozenJsonValue, FrozenList

raw: JsonValue = {"nested": [1, "a", None]}
model = GeneratedSchema(root=raw)
assert_type(model.root, FrozenJsonValue)
reused = GeneratedSchema(root=model.root)
assert_type(reused, GeneratedSchema)

def size(value: FrozenJsonValue) -> int:
    if isinstance(value, (FrozenList, FrozenDict)):
        return len(value)
    return 1
"#,
        r#"
from generated_models import GeneratedSchema
from jsoncompat.codegen.dataclasses import FrozenDict, FrozenJsonValue, FrozenList

GeneratedSchema(root=object())
GeneratedSchema(root=b"opaque")
GeneratedSchema(root={1: "non-string key"})

def mutate(value: FrozenJsonValue) -> None:
    if isinstance(value, FrozenList):
        value.append(1)
    elif isinstance(value, FrozenDict):
        value["a"] = 1
"#,
    )?;
    fs::write(work_dir.join("_generated_impl.py"), module.private_file())?;
    let valid_output = run_ty(&work_dir, "valid_usage.py")?;
    assert!(
        valid_output.status.success(),
        "ty rejected valid immutable JSON usage:\n{}\n{}",
        String::from_utf8_lossy(&valid_output.stdout),
        String::from_utf8_lossy(&valid_output.stderr),
    );
    let invalid_output = run_ty(&work_dir, "invalid_usage.py")?;
    assert!(!invalid_output.status.success());
    let diagnostics = String::from_utf8_lossy(&invalid_output.stdout);
    for rejected in ["root", "object", "opaque", "append", "FrozenDict"] {
        assert!(
            diagnostics.contains(rejected),
            "ty did not reject {rejected}:\n{diagnostics}"
        );
    }
    Ok(())
}

#[test]
fn omitted_fields_narrow_in_both_branches_without_losing_null() -> Result<(), Box<dyn Error>> {
    let source = generate_dataclass_models(&json!({
        "title": "Preferences",
        "type": "object",
        "properties": {
            "note": {"type": ["string", "null"]},
            "enabled": {"type": "boolean"},
            "tags": {"type": "array", "items": {"type": "string"}},
        },
        "additionalProperties": {"type": "integer"},
    }))?;
    let work_dir = write_typecheck_files(
        &source,
        r#"
from collections.abc import Sequence
from typing import Literal, assert_type

from generated_models import Preferences
from jsoncompat import MISSING, MissingType, is_missing
from jsoncompat.codegen import dataclasses as dc


def describe(preferences: Preferences) -> str:
    if preferences.note is MISSING:
        assert_type(preferences.note, Literal[MissingType.MISSING])
        return "omitted"
    elif preferences.note is None:
        return "null"
    else:
        assert_type(preferences.note, str)
        return preferences.note.upper()


def use_present_values(preferences: Preferences) -> None:
    if preferences.note is not MISSING and preferences.note is not None:
        assert_type(preferences.note, str)
        print(preferences.note.upper())
    if preferences.enabled is not dc.MISSING:
        assert_type(preferences.enabled, bool)
    else:
        assert_type(preferences.enabled, Literal[MissingType.MISSING])
    if preferences.tags is not MISSING:
        assert_type(preferences.tags, Sequence[str])
        print(",".join(preferences.tags))
    priority = preferences.get_additional_property("priority")
    if priority is not MISSING:
        assert_type(priority, int)
        print(priority + 1)


def require_present[T](value: dc.Omittable[T]) -> T:
    if value is MISSING:
        raise ValueError("value was omitted")
    return value


def nullable_value(preferences: Preferences) -> None:
    assert_type(require_present(preferences.note), str | None)
    assert_type(require_present(preferences.enabled), bool)


def helper_is_still_available(preferences: Preferences) -> str | None:
    if is_missing(preferences.note):
        assert_type(preferences.note, MissingType)
        return None
    assert_type(preferences.note, str | None)
    if not dc.is_missing(preferences.enabled):
        assert_type(preferences.enabled, bool)
    return preferences.note


def match_missing(preferences: Preferences) -> str:
    match preferences.note:
        case dc.MISSING:
            return "omitted"
        case None:
            return "null"
        case note:
            assert_type(note, str)
            return note.upper()
"#,
        r#"
from generated_models import Preferences
from jsoncompat import MISSING


def invalid_usage(preferences: Preferences) -> None:
    if preferences.enabled is MISSING:
        print(preferences.enabled + 1)
    if preferences.note is not MISSING:
        print(preferences.note.upper())  # Still nullable.
"#,
    )?;
    let valid_output = run_ty(&work_dir, "valid_usage.py")?;
    assert!(
        valid_output.status.success(),
        "ty rejected omitted-field narrowing:\n{}\n{}",
        String::from_utf8_lossy(&valid_output.stdout),
        String::from_utf8_lossy(&valid_output.stderr),
    );
    let invalid_output = run_ty(&work_dir, "invalid_usage.py")?;
    assert!(!invalid_output.status.success());
    let diagnostics = String::from_utf8_lossy(&invalid_output.stdout);
    assert!(
        diagnostics.contains("MissingType")
            && diagnostics.contains("upper")
            && diagnostics.contains("None"),
        "ty failed to preserve missing and null branches:\n{diagnostics}",
    );
    Ok(())
}

#[test]
fn patterned_additional_properties_keep_precise_extra_types() -> Result<(), Box<dyn Error>> {
    let schema = json!({
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
    });
    let source = generate_dataclass_models(&schema)?;
    let work_dir = write_typecheck_files(
        &source,
        r#"

from typing import Mapping, assert_type

from generated_models import LabeledRecord
from jsoncompat.codegen.dataclasses import MissingType


record = LabeledRecord.from_value({"name": "Ada", "x-rank": 7})
assert_type(record.__jsoncompat_extra__, Mapping[str, int])
assert_type(
    record.get_additional_property("x-rank"),
    int | MissingType,
)

constructed = LabeledRecord(name="Ada", __jsoncompat_extra__={"x-rank": 7})
assert_type(constructed, LabeledRecord)
"#,
        r#"

from generated_models import LabeledRecord


LabeledRecord(name="Ada", __jsoncompat_extra__={"x-rank": "high"})
"#,
    )?;

    let valid_output = run_ty(&work_dir, "valid_usage.py")?;
    assert!(
        valid_output.status.success(),
        "ty rejected valid patterned extras usage:\n{}\n{}",
        String::from_utf8_lossy(&valid_output.stdout),
        String::from_utf8_lossy(&valid_output.stderr),
    );

    let invalid_output = run_ty(&work_dir, "invalid_usage.py")?;
    assert!(
        !invalid_output.status.success(),
        "ty accepted invalid patterned extras usage"
    );
    let invalid_stdout = String::from_utf8_lossy(&invalid_output.stdout);
    assert!(
        invalid_stdout.contains("__jsoncompat_extra__")
            && invalid_stdout.contains("Mapping[str, int]"),
        "ty failure did not report the patterned extra value mismatch:\n{invalid_stdout}",
    );

    Ok(())
}

fn write_typecheck_files(
    source: &str,
    valid_usage: &str,
    invalid_usage: &str,
) -> Result<PathBuf, Box<dyn Error>> {
    // Concurrent tests can observe the same clock tick, even through as_nanos().
    static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);
    let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "jsoncompat-dataclass-typing-{}-{unique}-{sequence}",
        std::process::id(),
    ));
    fs::create_dir(&dir)?;
    fs::write(dir.join("generated_models.py"), source)?;
    fs::write(dir.join("valid_usage.py"), valid_usage)?;
    fs::write(dir.join("invalid_usage.py"), invalid_usage)?;
    Ok(dir)
}

fn run_ty(work_dir: &Path, file_name: &str) -> Result<std::process::Output, Box<dyn Error>> {
    let mut command = python_env::ty_command();
    command.arg(file_name).current_dir(work_dir);
    Ok(command.output()?)
}
