//! Allocation-free validation for the common generated-schema subset.
//!
//! Eligibility is intentionally conservative. Any source keyword or numeric
//! case whose semantics are not represented exactly here returns to the full
//! Draft 2020-12 backend in `lib.rs`.

use std::collections::HashSet;

use jiter::JsonValue as JiterJsonValue;
use json_schema_ast::{NodeId, SchemaDocument, SchemaNode, SchemaNodeKind};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBool, PyDict, PyFloat, PyInt, PyList, PyString, PyTuple};
use serde_json::Value as JsonValue;

const MAX_EXACT_F64_INTEGER: u64 = 9_007_199_254_740_991;

#[derive(Clone, Copy, PartialEq, Eq)]
struct ValidationFrame {
    schema: NodeId,
    value: usize,
}

struct ValidationContext {
    active: Vec<ValidationFrame>,
    supported: bool,
}

struct PythonValidationContext {
    active_schemas: Vec<ValidationFrame>,
    active_containers: HashSet<usize>,
    supported: bool,
}

pub(crate) fn supports(schema: &SchemaDocument) -> bool {
    source_schema_supported(schema.source_schema_json())
        && schema
            .root()
            .is_ok_and(|root| node_supported(root, &mut HashSet::new()))
}

pub(crate) fn is_valid(schema: &SchemaDocument, value: &JiterJsonValue<'_>) -> Option<bool> {
    let root = schema.root().ok()?;
    let mut context = ValidationContext {
        active: Vec::new(),
        supported: true,
    };
    let valid = node_accepts(root, value, &mut context);
    context.supported.then_some(valid)
}

pub(crate) fn is_valid_python(
    schema: &SchemaDocument,
    value: &Bound<'_, PyAny>,
) -> PyResult<Option<bool>> {
    let root = schema.root().map_err(|error| {
        pyo3::exceptions::PyValueError::new_err(format!("Validation failed: {error}"))
    })?;
    let mut context = PythonValidationContext {
        active_schemas: Vec::new(),
        active_containers: HashSet::new(),
        supported: true,
    };
    let valid = node_accepts_python(root, value, &mut context)?;
    Ok(context.supported.then_some(valid))
}

fn source_schema_supported(schema: &JsonValue) -> bool {
    let JsonValue::Object(object) = schema else {
        return schema.is_boolean();
    };

    const TYPE_SPECIFIC_KEYWORDS: &[&str] = &[
        "properties",
        "required",
        "additionalProperties",
        "dependentRequired",
        "minProperties",
        "maxProperties",
        "items",
        "prefixItems",
        "minItems",
        "maxItems",
        "minLength",
        "maxLength",
        "minimum",
        "maximum",
        "exclusiveMinimum",
        "exclusiveMaximum",
    ];
    if !object.contains_key("type")
        && TYPE_SPECIFIC_KEYWORDS
            .iter()
            .any(|keyword| object.contains_key(*keyword))
    {
        return false;
    }
    if object.contains_key("$ref")
        && object.keys().any(|keyword| {
            !matches!(
                keyword.as_str(),
                "$ref"
                    | "$defs"
                    | "definitions"
                    | "$schema"
                    | "$id"
                    | "title"
                    | "description"
                    | "$comment"
                    | "default"
                    | "deprecated"
                    | "readOnly"
                    | "writeOnly"
                    | "examples"
            )
        })
    {
        return false;
    }

    object
        .iter()
        .all(|(keyword, value)| match keyword.as_str() {
            "$schema" | "$id" | "title" | "description" | "$comment" | "default" | "deprecated"
            | "readOnly" | "writeOnly" | "examples" => true,
            "$ref" => value
                .as_str()
                .is_some_and(|reference| reference.starts_with('#')),
            "$defs" | "definitions" | "properties" => value
                .as_object()
                .is_some_and(|children| children.values().all(source_schema_supported)),
            "allOf" | "anyOf" | "oneOf" | "prefixItems" => value
                .as_array()
                .is_some_and(|children| children.iter().all(source_schema_supported)),
            "items" | "additionalProperties" => source_schema_supported(value),
            "type" | "enum" | "const" | "required" | "dependentRequired" | "minProperties"
            | "maxProperties" | "minItems" | "maxItems" | "minLength" | "maxLength" | "minimum"
            | "maximum" | "exclusiveMinimum" | "exclusiveMaximum" => true,
            _ => false,
        })
}

fn node_supported(node: &SchemaNode, seen: &mut HashSet<NodeId>) -> bool {
    if !seen.insert(node.id()) {
        return true;
    }

    match node.kind() {
        SchemaNodeKind::BoolSchema(_) | SchemaNodeKind::Any => true,
        SchemaNodeKind::String {
            pattern,
            format,
            enumeration,
            ..
        } => {
            pattern.is_none()
                && format.is_none()
                && enumeration.as_deref().is_none_or(scalar_values_supported)
        }
        SchemaNodeKind::Number { enumeration, .. }
        | SchemaNodeKind::Integer { enumeration, .. }
        | SchemaNodeKind::Boolean { enumeration }
        | SchemaNodeKind::Null { enumeration } => {
            enumeration.as_deref().is_none_or(scalar_values_supported)
        }
        SchemaNodeKind::Object {
            properties,
            pattern_properties,
            additional,
            property_names,
            enumeration,
            ..
        } => {
            pattern_properties.is_empty()
                && property_name_schema_is_unconstrained(property_names)
                && enumeration.is_none()
                && properties.values().all(|child| node_supported(child, seen))
                && node_supported(additional, seen)
        }
        SchemaNodeKind::Array {
            prefix_items,
            items,
            contains,
            unique_items,
            enumeration,
            ..
        } => {
            contains.is_none()
                && !unique_items
                && enumeration.is_none()
                && prefix_items.iter().all(|child| node_supported(child, seen))
                && node_supported(items, seen)
        }
        SchemaNodeKind::AllOf(children)
        | SchemaNodeKind::AnyOf(children)
        | SchemaNodeKind::OneOf(children) => {
            children.iter().all(|child| node_supported(child, seen))
        }
        SchemaNodeKind::Not(child) => node_supported(child, seen),
        SchemaNodeKind::IfThenElse {
            if_schema,
            then_schema,
            else_schema,
        } => {
            node_supported(if_schema, seen)
                && then_schema
                    .as_ref()
                    .is_none_or(|child| node_supported(child, seen))
                && else_schema
                    .as_ref()
                    .is_none_or(|child| node_supported(child, seen))
        }
        SchemaNodeKind::Const(value) => scalar_value_supported(value),
        SchemaNodeKind::Enum(values) => scalar_values_supported(values),
        _ => false,
    }
}

fn property_name_schema_is_unconstrained(node: &SchemaNode) -> bool {
    matches!(
        node.kind(),
        SchemaNodeKind::Any | SchemaNodeKind::BoolSchema(true)
    )
}

fn scalar_values_supported(values: &[JsonValue]) -> bool {
    values.iter().all(scalar_value_supported)
}

fn scalar_value_supported(value: &JsonValue) -> bool {
    match value {
        JsonValue::Null | JsonValue::Bool(_) | JsonValue::String(_) => true,
        JsonValue::Number(number) => number.as_f64().is_some_and(|number| {
            number.is_finite() && number.abs() <= MAX_EXACT_F64_INTEGER as f64
        }),
        JsonValue::Array(_) | JsonValue::Object(_) => false,
    }
}

fn node_accepts(
    node: &SchemaNode,
    value: &JiterJsonValue<'_>,
    context: &mut ValidationContext,
) -> bool {
    let guarded = matches!(
        node.kind(),
        SchemaNodeKind::AllOf(_)
            | SchemaNodeKind::AnyOf(_)
            | SchemaNodeKind::OneOf(_)
            | SchemaNodeKind::Not(_)
            | SchemaNodeKind::IfThenElse { .. }
    );
    if guarded {
        let frame = ValidationFrame {
            schema: node.id(),
            value: std::ptr::from_ref(value) as usize,
        };
        if context.active.contains(&frame) {
            return false;
        }
        context.active.push(frame);
    }

    let valid = match node.kind() {
        SchemaNodeKind::BoolSchema(valid) => *valid,
        SchemaNodeKind::Any => true,
        SchemaNodeKind::String {
            length,
            enumeration,
            ..
        } => match value {
            JiterJsonValue::Str(value) => {
                length.contains(value.chars().count() as u64)
                    && enumeration.as_deref().is_none_or(|values| {
                        values
                            .iter()
                            .any(|expected| expected.as_str() == Some(value.as_ref()))
                    })
            }
            _ => false,
        },
        SchemaNodeKind::Number {
            bounds,
            multiple_of,
            enumeration,
        } => jiter_number(value, context).is_some_and(|number| {
            bounds.contains(number)
                && number_is_multiple_of(number, multiple_of.map(|divisor| divisor.as_f64()))
                && enumeration_accepts(enumeration.as_deref(), value, context)
        }),
        SchemaNodeKind::Integer {
            bounds,
            multiple_of,
            enumeration,
        } => jiter_integer(value, context).is_some_and(|integer| {
            bounds.contains_i128(integer)
                && multiple_of
                    .as_ref()
                    .is_none_or(|divisor| integer_is_multiple_of(integer, *divisor))
                && enumeration_accepts(enumeration.as_deref(), value, context)
        }),
        SchemaNodeKind::Boolean { enumeration } => {
            matches!(value, JiterJsonValue::Bool(_))
                && enumeration_accepts(enumeration.as_deref(), value, context)
        }
        SchemaNodeKind::Null { enumeration } => {
            matches!(value, JiterJsonValue::Null)
                && enumeration_accepts(enumeration.as_deref(), value, context)
        }
        SchemaNodeKind::Object {
            properties,
            required,
            additional,
            property_count,
            dependent_required,
            ..
        } => match value {
            JiterJsonValue::Object(entries) => {
                if !property_count.contains(entries.len()) {
                    false
                } else {
                    let mut required_found = 0;
                    let mut valid = true;
                    for (index, (name, item)) in entries.iter().enumerate() {
                        if entries[..index]
                            .iter()
                            .any(|(seen, _)| seen.as_ref() == name.as_ref())
                        {
                            valid = false;
                            break;
                        }
                        if required.contains(name.as_ref()) {
                            required_found += 1;
                        }
                        let child = properties.get(name.as_ref()).unwrap_or(additional);
                        if !node_accepts(child, item, context) {
                            valid = false;
                            break;
                        }
                    }
                    valid
                        && required_found == required.len()
                        && dependent_required.iter().all(|(trigger, dependencies)| {
                            !entries
                                .iter()
                                .any(|(name, _)| name.as_ref() == trigger.as_str())
                                || dependencies.iter().all(|dependency| {
                                    entries
                                        .iter()
                                        .any(|(name, _)| name.as_ref() == dependency.as_str())
                                })
                        })
                }
            }
            _ => false,
        },
        SchemaNodeKind::Array {
            prefix_items,
            items,
            item_count,
            ..
        } => match value {
            JiterJsonValue::Array(values) => {
                item_count.contains(values.len() as u64)
                    && values.iter().enumerate().all(|(index, item)| {
                        node_accepts(prefix_items.get(index).unwrap_or(items), item, context)
                    })
            }
            _ => false,
        },
        SchemaNodeKind::AllOf(children) => children
            .iter()
            .all(|child| node_accepts(child, value, context)),
        SchemaNodeKind::AnyOf(children) => children
            .iter()
            .any(|child| node_accepts(child, value, context)),
        SchemaNodeKind::OneOf(children) => match discriminated_child(children, value) {
            Some(Some(child)) => node_accepts(child, value, context),
            Some(None) => false,
            None => {
                children
                    .iter()
                    .filter(|child| node_accepts(child, value, context))
                    .count()
                    == 1
            }
        },
        SchemaNodeKind::Not(child) => !node_accepts(child, value, context),
        SchemaNodeKind::IfThenElse {
            if_schema,
            then_schema,
            else_schema,
        } => {
            if node_accepts(if_schema, value, context) {
                then_schema
                    .as_ref()
                    .is_none_or(|child| node_accepts(child, value, context))
            } else {
                else_schema
                    .as_ref()
                    .is_none_or(|child| node_accepts(child, value, context))
            }
        }
        SchemaNodeKind::Const(expected) => values_equal(value, expected, context),
        SchemaNodeKind::Enum(expected) => enumeration_accepts(Some(expected), value, context),
        _ => false,
    };

    if guarded {
        context.active.pop();
    }
    valid
}

fn node_accepts_python(
    node: &SchemaNode,
    value: &Bound<'_, PyAny>,
    context: &mut PythonValidationContext,
) -> PyResult<bool> {
    let guarded = matches!(
        node.kind(),
        SchemaNodeKind::AllOf(_)
            | SchemaNodeKind::AnyOf(_)
            | SchemaNodeKind::OneOf(_)
            | SchemaNodeKind::Not(_)
            | SchemaNodeKind::IfThenElse { .. }
    );
    if guarded {
        let frame = ValidationFrame {
            schema: node.id(),
            value: value.as_ptr() as usize,
        };
        if context.active_schemas.contains(&frame) {
            return Ok(false);
        }
        context.active_schemas.push(frame);
    }

    let valid = (|| -> PyResult<bool> {
        Ok(match node.kind() {
            SchemaNodeKind::BoolSchema(true) => python_value_is_json(value, context)?,
            SchemaNodeKind::BoolSchema(false) => false,
            SchemaNodeKind::Any => python_value_is_json(value, context)?,
            SchemaNodeKind::String {
                length,
                enumeration,
                ..
            } => {
                if let Ok(value) = value.cast::<PyString>() {
                    let value = value.to_str()?;
                    length.contains(value.chars().count() as u64)
                        && enumeration.as_deref().is_none_or(|values| {
                            values
                                .iter()
                                .any(|expected| expected.as_str() == Some(value))
                        })
                } else {
                    false
                }
            }
            SchemaNodeKind::Number {
                bounds,
                multiple_of,
                enumeration,
            } => {
                if let Some(number) = python_number(value, context)? {
                    bounds.contains(number)
                        && number_is_multiple_of(
                            number,
                            multiple_of.map(|divisor| divisor.as_f64()),
                        )
                        && python_enumeration_accepts(enumeration.as_deref(), value, context)?
                } else {
                    false
                }
            }
            SchemaNodeKind::Integer {
                bounds,
                multiple_of,
                enumeration,
            } => {
                if let Some(integer) = python_integer(value, context)? {
                    bounds.contains_i128(integer)
                        && multiple_of
                            .as_ref()
                            .is_none_or(|divisor| integer_is_multiple_of(integer, *divisor))
                        && python_enumeration_accepts(enumeration.as_deref(), value, context)?
                } else {
                    false
                }
            }
            SchemaNodeKind::Boolean { enumeration } => {
                value.is_instance_of::<PyBool>()
                    && python_enumeration_accepts(enumeration.as_deref(), value, context)?
            }
            SchemaNodeKind::Null { enumeration } => {
                value.is_none()
                    && python_enumeration_accepts(enumeration.as_deref(), value, context)?
            }
            SchemaNodeKind::Object {
                properties,
                required,
                additional,
                property_count,
                dependent_required,
                ..
            } => {
                if let Ok(input) = value.cast::<PyDict>() {
                    python_object_accepts(
                        input,
                        properties,
                        required,
                        additional,
                        *property_count,
                        dependent_required,
                        context,
                    )?
                } else {
                    false
                }
            }
            SchemaNodeKind::Array {
                prefix_items,
                items,
                item_count,
                ..
            } => {
                if let Ok(input) = value.cast::<PyList>() {
                    python_array_accepts(
                        value,
                        input.len(),
                        input.iter(),
                        prefix_items,
                        items,
                        *item_count,
                        context,
                    )?
                } else if let Ok(input) = value.cast::<PyTuple>() {
                    python_array_accepts(
                        value,
                        input.len(),
                        input.iter(),
                        prefix_items,
                        items,
                        *item_count,
                        context,
                    )?
                } else {
                    false
                }
            }
            SchemaNodeKind::AllOf(children) => {
                let mut valid = true;
                for child in children {
                    if !node_accepts_python(child, value, context)? {
                        valid = false;
                        break;
                    }
                }
                valid
            }
            SchemaNodeKind::AnyOf(children) => {
                let mut valid = false;
                for child in children {
                    if node_accepts_python(child, value, context)? {
                        valid = true;
                        break;
                    }
                }
                valid
            }
            SchemaNodeKind::OneOf(children) => match discriminated_python_child(children, value)? {
                Some(Some(child)) => node_accepts_python(child, value, context)?,
                Some(None) => false,
                None => {
                    let mut matches = 0;
                    for child in children {
                        if node_accepts_python(child, value, context)? {
                            matches += 1;
                            if matches > 1 {
                                break;
                            }
                        }
                    }
                    matches == 1
                }
            },
            SchemaNodeKind::Not(child) => !node_accepts_python(child, value, context)?,
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => {
                if node_accepts_python(if_schema, value, context)? {
                    if let Some(child) = then_schema {
                        node_accepts_python(child, value, context)?
                    } else {
                        true
                    }
                } else if let Some(child) = else_schema {
                    node_accepts_python(child, value, context)?
                } else {
                    true
                }
            }
            SchemaNodeKind::Const(expected) => python_values_equal(value, expected, context)?,
            SchemaNodeKind::Enum(expected) => {
                python_enumeration_accepts(Some(expected), value, context)?
            }
            _ => false,
        })
    })();

    if guarded {
        context.active_schemas.pop();
    }
    valid
}

fn python_object_accepts(
    input: &Bound<'_, PyDict>,
    properties: &std::collections::HashMap<String, SchemaNode>,
    required: &HashSet<String>,
    additional: &SchemaNode,
    property_count: json_schema_ast::CountRange<usize>,
    dependent_required: &std::collections::HashMap<String, Vec<String>>,
    context: &mut PythonValidationContext,
) -> PyResult<bool> {
    let container_id = input.as_ptr() as usize;
    if !context.active_containers.insert(container_id) {
        return Ok(false);
    }
    let result = (|| -> PyResult<bool> {
        if !property_count.contains(input.len()) {
            return Ok(false);
        }
        for required in required {
            if !input.contains(required)? {
                return Ok(false);
            }
        }
        for (trigger, dependencies) in dependent_required {
            if input.contains(trigger)? {
                for dependency in dependencies {
                    if !input.contains(dependency)? {
                        return Ok(false);
                    }
                }
            }
        }
        for (name, item) in input {
            let Ok(name) = name.cast::<PyString>() else {
                return Ok(false);
            };
            let name = name.to_str()?;
            let child = properties.get(name).unwrap_or(additional);
            if !node_accepts_python(child, &item, context)? {
                return Ok(false);
            }
        }
        Ok(true)
    })();
    context.active_containers.remove(&container_id);
    result
}

fn python_array_accepts<'py>(
    container: &Bound<'py, PyAny>,
    len: usize,
    values: impl Iterator<Item = Bound<'py, PyAny>>,
    prefix_items: &[SchemaNode],
    items: &SchemaNode,
    item_count: json_schema_ast::CountRange<u64>,
    context: &mut PythonValidationContext,
) -> PyResult<bool> {
    if !item_count.contains(len as u64) {
        return Ok(false);
    }
    let container_id = container.as_ptr() as usize;
    if !context.active_containers.insert(container_id) {
        return Ok(false);
    }
    let result = (|| -> PyResult<bool> {
        for (index, item) in values.enumerate() {
            if !node_accepts_python(prefix_items.get(index).unwrap_or(items), &item, context)? {
                return Ok(false);
            }
        }
        Ok(true)
    })();
    context.active_containers.remove(&container_id);
    result
}

fn python_value_is_json(
    value: &Bound<'_, PyAny>,
    context: &mut PythonValidationContext,
) -> PyResult<bool> {
    if value.is_none()
        || value.is_instance_of::<PyBool>()
        || value.is_instance_of::<PyInt>()
        || value.is_instance_of::<PyString>()
    {
        return Ok(true);
    }
    if value.is_instance_of::<PyFloat>() {
        return Ok(value.extract::<f64>()?.is_finite());
    }
    if let Ok(values) = value.cast::<PyList>() {
        return python_json_array(value, values.iter(), context);
    }
    if let Ok(values) = value.cast::<PyTuple>() {
        return python_json_array(value, values.iter(), context);
    }
    if let Ok(input) = value.cast::<PyDict>() {
        let container_id = value.as_ptr() as usize;
        if !context.active_containers.insert(container_id) {
            return Ok(false);
        }
        let result = (|| -> PyResult<bool> {
            for (name, item) in input {
                if !name.is_instance_of::<PyString>() || !python_value_is_json(&item, context)? {
                    return Ok(false);
                }
            }
            Ok(true)
        })();
        context.active_containers.remove(&container_id);
        return result;
    }
    Ok(false)
}

fn python_json_array<'py>(
    container: &Bound<'py, PyAny>,
    values: impl Iterator<Item = Bound<'py, PyAny>>,
    context: &mut PythonValidationContext,
) -> PyResult<bool> {
    let container_id = container.as_ptr() as usize;
    if !context.active_containers.insert(container_id) {
        return Ok(false);
    }
    let result = (|| -> PyResult<bool> {
        for item in values {
            if !python_value_is_json(&item, context)? {
                return Ok(false);
            }
        }
        Ok(true)
    })();
    context.active_containers.remove(&container_id);
    result
}

fn python_number(
    value: &Bound<'_, PyAny>,
    context: &mut PythonValidationContext,
) -> PyResult<Option<f64>> {
    if value.is_instance_of::<PyBool>() {
        return Ok(None);
    }
    if value.is_instance_of::<PyInt>() {
        let Ok(integer) = value.extract::<i128>() else {
            context.supported = false;
            return Ok(None);
        };
        if integer.unsigned_abs() > u128::from(MAX_EXACT_F64_INTEGER) {
            context.supported = false;
            return Ok(None);
        }
        return Ok(Some(integer as f64));
    }
    if value.is_instance_of::<PyFloat>() {
        let number = value.extract::<f64>()?;
        return Ok(number.is_finite().then_some(number));
    }
    Ok(None)
}

fn python_integer(
    value: &Bound<'_, PyAny>,
    context: &mut PythonValidationContext,
) -> PyResult<Option<i128>> {
    if value.is_instance_of::<PyBool>() {
        return Ok(None);
    }
    if value.is_instance_of::<PyInt>() {
        return match value.extract::<i128>() {
            Ok(integer) => Ok(Some(integer)),
            Err(_) => {
                context.supported = false;
                Ok(None)
            }
        };
    }
    if value.is_instance_of::<PyFloat>() {
        let number = value.extract::<f64>()?;
        if number.is_finite()
            && number.fract() == 0.0
            && number.abs() <= MAX_EXACT_F64_INTEGER as f64
        {
            return Ok(Some(number as i128));
        }
        if number.is_finite() && number.fract() == 0.0 {
            context.supported = false;
        }
    }
    Ok(None)
}

fn python_enumeration_accepts(
    enumeration: Option<&[JsonValue]>,
    value: &Bound<'_, PyAny>,
    context: &mut PythonValidationContext,
) -> PyResult<bool> {
    let Some(enumeration) = enumeration else {
        return Ok(true);
    };
    for expected in enumeration {
        if python_values_equal(value, expected, context)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn python_values_equal(
    actual: &Bound<'_, PyAny>,
    expected: &JsonValue,
    context: &mut PythonValidationContext,
) -> PyResult<bool> {
    Ok(match expected {
        JsonValue::Null => actual.is_none(),
        JsonValue::Bool(expected) => {
            actual.is_instance_of::<PyBool>() && actual.extract::<bool>()? == *expected
        }
        JsonValue::String(expected) => actual
            .cast::<PyString>()
            .is_ok_and(|actual| actual.to_str().is_ok_and(|actual| actual == expected)),
        JsonValue::Number(expected) => {
            let Some(actual) = python_number(actual, context)? else {
                return Ok(false);
            };
            expected.as_f64().is_some_and(|expected| actual == expected)
        }
        JsonValue::Array(_) | JsonValue::Object(_) => false,
    })
}

fn discriminated_python_child<'a>(
    children: &'a [SchemaNode],
    value: &Bound<'_, PyAny>,
) -> PyResult<Option<Option<&'a SchemaNode>>> {
    let Ok(input) = value.cast::<PyDict>() else {
        return Ok(None);
    };
    let first_properties = match children.first().map(SchemaNode::kind) {
        Some(SchemaNodeKind::Object { properties, .. }) => properties,
        _ => return Ok(None),
    };

    for (name, first_property) in first_properties {
        if !matches!(
            first_property.kind(),
            SchemaNodeKind::Const(JsonValue::String(_))
        ) {
            continue;
        }
        let Some(actual) = input.get_item(name)? else {
            continue;
        };
        let Ok(actual) = actual.cast::<PyString>() else {
            continue;
        };
        let actual = actual.to_str()?;

        let mut matched = None;
        for child in children {
            let SchemaNodeKind::Object { properties, .. } = child.kind() else {
                return Ok(None);
            };
            let Some(property) = properties.get(name) else {
                return Ok(None);
            };
            let SchemaNodeKind::Const(JsonValue::String(expected)) = property.kind() else {
                return Ok(None);
            };
            if expected == actual {
                if matched.is_some() {
                    return Ok(None);
                }
                matched = Some(child);
            }
        }
        return Ok(Some(matched));
    }
    Ok(None)
}

fn discriminated_child<'a>(
    children: &'a [SchemaNode],
    value: &JiterJsonValue<'_>,
) -> Option<Option<&'a SchemaNode>> {
    let JiterJsonValue::Object(entries) = value else {
        return None;
    };
    let first_properties = match children.first()?.kind() {
        SchemaNodeKind::Object { properties, .. } => properties,
        _ => return None,
    };

    for (name, actual_value) in entries.iter() {
        let JiterJsonValue::Str(actual) = actual_value else {
            continue;
        };
        let Some(first_property) = first_properties.get(name.as_ref()) else {
            continue;
        };
        if !matches!(
            first_property.kind(),
            SchemaNodeKind::Const(JsonValue::String(_))
        ) {
            continue;
        }

        let mut matched = None;
        for child in children {
            let SchemaNodeKind::Object { properties, .. } = child.kind() else {
                return None;
            };
            let property = properties.get(name.as_ref())?;
            let SchemaNodeKind::Const(JsonValue::String(expected)) = property.kind() else {
                return None;
            };
            if expected == actual.as_ref() {
                if matched.is_some() {
                    return None;
                }
                matched = Some(child);
            }
        }
        return Some(matched);
    }
    None
}

fn jiter_number(value: &JiterJsonValue<'_>, context: &mut ValidationContext) -> Option<f64> {
    match value {
        JiterJsonValue::Int(value) if value.unsigned_abs() <= MAX_EXACT_F64_INTEGER => {
            Some(*value as f64)
        }
        JiterJsonValue::Int(_) | JiterJsonValue::BigInt(_) => {
            context.supported = false;
            None
        }
        JiterJsonValue::Float(value) => Some(*value),
        _ => None,
    }
}

fn jiter_integer(value: &JiterJsonValue<'_>, context: &mut ValidationContext) -> Option<i128> {
    match value {
        JiterJsonValue::Int(value) => Some(i128::from(*value)),
        JiterJsonValue::Float(value)
            if value.fract() == 0.0 && value.abs() <= MAX_EXACT_F64_INTEGER as f64 =>
        {
            Some(*value as i128)
        }
        JiterJsonValue::Float(value) if value.fract() == 0.0 => {
            context.supported = false;
            None
        }
        JiterJsonValue::BigInt(_) => {
            context.supported = false;
            None
        }
        _ => None,
    }
}

fn number_is_multiple_of(value: f64, divisor: Option<f64>) -> bool {
    let Some(divisor) = divisor else {
        return true;
    };
    let ratio = value / divisor;
    (ratio - ratio.round()).abs() <= f64::EPSILON * ratio.abs().max(1.0) * 4.0
}

fn integer_is_multiple_of(value: i128, divisor: json_schema_ast::IntegerMultipleOf) -> bool {
    divisor.integer_divisor().map_or_else(
        || number_is_multiple_of(value as f64, Some(divisor.as_f64())),
        |divisor| value.rem_euclid(divisor) == 0,
    )
}

fn enumeration_accepts(
    enumeration: Option<&[JsonValue]>,
    value: &JiterJsonValue<'_>,
    context: &mut ValidationContext,
) -> bool {
    enumeration.is_none_or(|values| {
        values
            .iter()
            .any(|expected| values_equal(value, expected, context))
    })
}

fn values_equal(
    actual: &JiterJsonValue<'_>,
    expected: &JsonValue,
    context: &mut ValidationContext,
) -> bool {
    match (actual, expected) {
        (JiterJsonValue::Null, JsonValue::Null) => true,
        (JiterJsonValue::Bool(actual), JsonValue::Bool(expected)) => actual == expected,
        (JiterJsonValue::Str(actual), JsonValue::String(expected)) => actual.as_ref() == expected,
        (JiterJsonValue::Int(actual), JsonValue::Number(expected)) => {
            expected
                .as_i64()
                .is_some_and(|expected| *actual == expected)
                || expected.as_u64().is_some_and(|expected| {
                    u64::try_from(*actual).is_ok_and(|actual| actual == expected)
                })
                || expected
                    .as_f64()
                    .is_some_and(|expected| *actual as f64 == expected)
        }
        (JiterJsonValue::Float(actual), JsonValue::Number(expected)) => expected
            .as_f64()
            .is_some_and(|expected| *actual == expected),
        (JiterJsonValue::BigInt(_), JsonValue::Number(_)) => {
            context.supported = false;
            false
        }
        _ => false,
    }
}
