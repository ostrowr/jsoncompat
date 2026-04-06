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
        },
        "required": ["sku", "metadata"],
        "additionalProperties": {"type": "number"},
    });
    let source = generate_dataclass_models(&schema)?;
    let work_dir = write_typecheck_files(
        &source,
        r#"
# pyright: strict

from typing import assert_type

from generated_models import InventoryItem, InventoryItemMetadata, JSONCOMPAT_MODEL
from jsoncompat.codegen.dataclasses import JSONCOMPAT_MISSING, JsoncompatMissingType, Omittable


item = InventoryItem.from_json({
    "sku": "sku-123",
    "metadata": {"warehouse": "west"},
    "quantity": 10,
    "tags": ["fresh", "boxed"],
    "priority": 1,
})

assert_type(JSONCOMPAT_MODEL, type[InventoryItem])
assert_type(item, InventoryItem)
assert_type(item.sku, str)
assert_type(item.quantity, Omittable[int])
assert_type(item.metadata, InventoryItemMetadata)
assert_type(item.metadata.warehouse, str)
assert_type(item.tags, Omittable[list[str]])
assert_type(item.__jsoncompat_extra__, dict[str, float])
assert_type(
    item.get_additional_property("priority"),
    float | JsoncompatMissingType,
)

item_from_constructor = InventoryItem(
    sku="sku-456",
    metadata=InventoryItemMetadata(warehouse="east"),
    quantity=JSONCOMPAT_MISSING,
    tags=["dry"],
    __jsoncompat_extra__={"priority": 2.5},
)
assert_type(item_from_constructor, InventoryItem)
"#,
        r#"
# pyright: strict

from generated_models import InventoryItem, InventoryItemMetadata


InventoryItem(
    sku=123,
    metadata=InventoryItemMetadata(warehouse="east"),
)

InventoryItem(
    sku="sku-456",
    metadata=InventoryItemMetadata(warehouse=123),
)

InventoryItemMetadata(warehouse="east").get_additional_property("priority")
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
