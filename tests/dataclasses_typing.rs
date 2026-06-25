#[path = "support/python_env.rs"]
mod python_env;

use jsoncompat_codegen::generate_dataclass_models;
use serde_json::json;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
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
        },
        "required": ["sku", "metadata"],
        "additionalProperties": {"type": "number"},
        "$defs": {
            "warehouseCode": {
                "type": "string",
            }
        }
    });
    let source = generate_dataclass_models(&schema)?;
    let work_dir = write_typecheck_files(
        &source,
        r#"
# pyright: strict

from typing import Mapping, Sequence, assert_type

from generated_models import InventoryItem, InventoryItemMetadata, JSONCOMPAT_MODEL
from jsoncompat import JsonValue
from jsoncompat.codegen import SerializationFormat
from jsoncompat.codegen.dataclasses import JSONCOMPAT_MISSING, JsoncompatMissingType, Omittable


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
assert_type(item.warehouseCode, Omittable[str])
assert_type(item.coordinates, Omittable[Sequence[int | str]])
assert_type(item.__jsoncompat_extra__, Mapping[str, float])
assert_type(
    item.get_additional_property("priority"),
    float | JsoncompatMissingType,
)
assert_type(JsoncompatMissingType(), JsoncompatMissingType)

item_from_constructor = InventoryItem(
    sku="sku-456",
    metadata=InventoryItemMetadata(warehouse="east"),
    quantity=JSONCOMPAT_MISSING,
    tags=["dry"],
    warehouseCode="WH-123",
    coordinates=["aisle", 7],
    __jsoncompat_extra__={"priority": 2.5},
    skip_validation=True,
)
assert_type(item_from_constructor, InventoryItem)
assert_type(
    InventoryItem.from_value(item.to_value(), skip_validation=True),
    InventoryItem,
)
"#,
        r#"
# pyright: strict

from generated_models import InventoryItem, InventoryItemMetadata
from jsoncompat.codegen.dataclasses import JsoncompatMissingType


class ForgedMissing(JsoncompatMissingType):
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
"#,
    )?;

    let valid_output = run_pyright(&work_dir, "valid_usage.py")?;
    assert!(
        valid_output.status.success(),
        "pyright rejected valid generated dataclass usage:\n{}\n{}",
        String::from_utf8_lossy(&valid_output.stdout),
        String::from_utf8_lossy(&valid_output.stderr),
    );

    let invalid_output = run_pyright(&work_dir, "invalid_usage.py")?;
    assert!(
        !invalid_output.status.success(),
        "pyright accepted invalid generated dataclass usage"
    );
    let invalid_stdout = String::from_utf8_lossy(&invalid_output.stdout);
    assert!(
        invalid_stdout.contains("sku") && invalid_stdout.contains("str"),
        "pyright failure did not report the sku type mismatch:\n{invalid_stdout}",
    );
    assert!(
        invalid_stdout.contains("warehouse") && invalid_stdout.contains("str"),
        "pyright failure did not report the nested warehouse type mismatch:\n{invalid_stdout}",
    );
    assert!(
        invalid_stdout.contains("JsoncompatMissingType")
            && invalid_stdout.contains("marked final")
            && invalid_stdout.contains("cannot be subclassed"),
        "pyright failure did not reject subclassing the final missing singleton type:\n{invalid_stdout}",
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
# pyright: strict

from typing import Mapping, assert_type

from generated_models import LabeledRecord
from jsoncompat.codegen.dataclasses import JsoncompatMissingType


record = LabeledRecord.from_value({"name": "Ada", "x-rank": 7})
assert_type(record.__jsoncompat_extra__, Mapping[str, int])
assert_type(
    record.get_additional_property("x-rank"),
    int | JsoncompatMissingType,
)

constructed = LabeledRecord(name="Ada", __jsoncompat_extra__={"x-rank": 7})
assert_type(constructed, LabeledRecord)
"#,
        r#"
# pyright: strict

from generated_models import LabeledRecord


LabeledRecord(name="Ada", __jsoncompat_extra__={"x-rank": "high"})
"#,
    )?;

    let valid_output = run_pyright(&work_dir, "valid_usage.py")?;
    assert!(
        valid_output.status.success(),
        "pyright rejected valid patterned extras usage:\n{}\n{}",
        String::from_utf8_lossy(&valid_output.stdout),
        String::from_utf8_lossy(&valid_output.stderr),
    );

    let invalid_output = run_pyright(&work_dir, "invalid_usage.py")?;
    assert!(
        !invalid_output.status.success(),
        "pyright accepted invalid patterned extras usage"
    );
    let invalid_stdout = String::from_utf8_lossy(&invalid_output.stdout);
    assert!(
        invalid_stdout.contains("__jsoncompat_extra__")
            && invalid_stdout.contains("Mapping[str, int]"),
        "pyright failure did not report the patterned extra value mismatch:\n{invalid_stdout}",
    );

    Ok(())
}

fn write_typecheck_files(
    source: &str,
    valid_usage: &str,
    invalid_usage: &str,
) -> Result<PathBuf, Box<dyn Error>> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "jsoncompat-dataclass-typing-{}-{unique}",
        std::process::id(),
    ));
    fs::create_dir_all(&dir)?;
    fs::write(dir.join("generated_models.py"), source)?;
    fs::write(dir.join("valid_usage.py"), valid_usage)?;
    fs::write(dir.join("invalid_usage.py"), invalid_usage)?;
    Ok(dir)
}

fn run_pyright(work_dir: &Path, file_name: &str) -> Result<std::process::Output, Box<dyn Error>> {
    let mut command = python_env::pyright_command();
    command
        .arg("--pythonversion")
        .arg("3.12")
        .arg(file_name)
        .current_dir(work_dir);
    Ok(command.output()?)
}
