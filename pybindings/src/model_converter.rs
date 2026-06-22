//! Compiled conversion plans for generated Python dataclasses.
//!
//! Plans move repeated object-graph traversal into Rust while retaining the
//! Python runtime's existing type checks, missing-field factories, union
//! selection, and frozen-slot construction semantics.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use jiter::JsonValue as JiterJsonValue;
use jsonschema::{
    InstanceRef as JsonInstanceRef, ProjectedPythonKind, ProjectedPythonValue,
    PythonInstanceProvider,
};
use pyo3::Borrowed;
use pyo3::exceptions::{PyIndexError, PyOverflowError, PyTypeError, PyValueError};
use pyo3::ffi;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBool, PyDict, PyFloat, PyInt, PyList, PyString, PyTuple, PyType};

const MAX_MODEL_DEPTH: u16 = 255;

#[derive(Clone, Copy)]
enum ScalarKind {
    Any,
    Missing,
    String,
    Integer,
    Number,
    Boolean,
    Null,
}

struct DiscriminatorPlan {
    json_name: String,
    branches_by_value: HashMap<String, usize>,
}

struct FieldPlan {
    json_name: String,
    py_name: Py<PyString>,
    slot_offset: Option<isize>,
    value_node: usize,
    missing_factory: Py<PyAny>,
    missing_sentinel: Option<Py<PyAny>>,
}

enum ConversionNode {
    Scalar {
        kind: ScalarKind,
        missing_sentinel: Option<Py<PyAny>>,
    },
    List {
        item: usize,
    },
    Dict {
        key: usize,
        value: usize,
    },
    Literal {
        values: Vec<Py<PyAny>>,
        string_indices: HashMap<String, usize>,
    },
    Union {
        branches: Vec<usize>,
        discriminator: Option<DiscriminatorPlan>,
    },
    Model {
        model_type: Py<PyType>,
        fields: Vec<FieldPlan>,
        fields_by_json_name: HashMap<String, usize>,
        serialized_fields: Vec<usize>,
        required_field_count: usize,
        omittable_fields: Vec<usize>,
        extra_value: Option<usize>,
        extra_py_name: Option<Py<PyString>>,
        extra_slot_offset: Option<isize>,
    },
    Root {
        model_type: Py<PyType>,
        value: usize,
        root_py_name: Py<PyString>,
        root_slot_offset: Option<isize>,
    },
}

#[pyclass(name = "ModelConverter", module = "jsoncompat._native", unsendable)]
pub(crate) struct ModelConverterPy {
    nodes: Vec<ConversionNode>,
    root: usize,
    object_new: Py<PyAny>,
    frozen_list_type: Py<PyType>,
    frozen_dict_type: Py<PyType>,
    validated_py_name: Py<PyString>,
}

pub(crate) struct ModelProjection<'a> {
    converter: &'a ModelConverterPy,
    retained: RefCell<Vec<Py<PyAny>>>,
    union_branches: RefCell<HashMap<(usize, usize), usize>>,
}

#[pymethods]
impl ModelConverterPy {
    fn construct(
        &self,
        py: Python<'_>,
        value: &Bound<'_, PyAny>,
        validated: bool,
    ) -> PyResult<Py<PyAny>> {
        let instance = self.convert(py, self.root, value, validated, MAX_MODEL_DEPTH)?;
        self.finalize(py, instance, validated)
    }
}

impl ModelConverterPy {
    pub(crate) fn projection(&self) -> ModelProjection<'_> {
        ModelProjection {
            converter: self,
            retained: RefCell::new(Vec::new()),
            union_branches: RefCell::new(HashMap::new()),
        }
    }

    pub(crate) fn construct_python_unvalidated(
        &self,
        py: Python<'_>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<Py<PyAny>> {
        self.convert(py, self.root, value, false, MAX_MODEL_DEPTH)
    }

    pub(crate) fn mark_validated(
        &self,
        py: Python<'_>,
        instance: Py<PyAny>,
    ) -> PyResult<Py<PyAny>> {
        self.finalize(py, instance, true)
    }

    pub(crate) fn construct_jiter(
        &self,
        py: Python<'_>,
        value: &JiterJsonValue<'_>,
        validated: bool,
    ) -> PyResult<Py<PyAny>> {
        let instance = self.convert_jiter(py, self.root, value)?;
        self.finalize(py, instance, validated)
    }

    fn finalize(
        &self,
        py: Python<'_>,
        instance: Py<PyAny>,
        validated: bool,
    ) -> PyResult<Py<PyAny>> {
        let validated = PyBool::new(py, validated).to_owned().into_any().unbind();
        set_model_attribute(py, instance.bind(py), &self.validated_py_name, &validated)?;
        Ok(instance)
    }

    pub(crate) fn serialize_to_json_string(
        &self,
        py: Python<'_>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<String> {
        let mut output = Vec::with_capacity(256);
        self.write_json_node(py, self.root, value, MAX_MODEL_DEPTH, &mut output)?;
        String::from_utf8(output).map_err(|error| {
            PyErr::new::<PyValueError, _>(format!(
                "JSON serialization produced invalid UTF-8: {error}"
            ))
        })
    }

    pub(crate) fn to_python_value(
        &self,
        py: Python<'_>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<Py<PyAny>> {
        self.to_python_value_node(py, self.root, value, MAX_MODEL_DEPTH)
    }

    fn convert(
        &self,
        py: Python<'_>,
        node_id: usize,
        value: &Bound<'_, PyAny>,
        validated: bool,
        remaining_depth: u16,
    ) -> PyResult<Py<PyAny>> {
        if remaining_depth == 0 {
            return Err(PyErr::new::<PyValueError, _>(
                "generated model conversion exceeds the maximum nesting depth",
            ));
        }
        let node = self.nodes.get(node_id).ok_or_else(|| {
            PyErr::new::<PyIndexError, _>(format!("model converter node {node_id} is missing"))
        })?;
        match node {
            ConversionNode::Scalar {
                kind,
                missing_sentinel,
            } => {
                if matches!(kind, ScalarKind::Any) {
                    self.freeze_python_json_value(py, value, remaining_depth)
                } else {
                    convert_scalar(py, *kind, missing_sentinel.as_ref(), value, validated)
                }
            }
            ConversionNode::List { item } => {
                self.convert_list(py, *item, value, validated, remaining_depth)
            }
            ConversionNode::Dict {
                key,
                value: value_node,
            } => self.convert_dict(py, *key, *value_node, value, validated, remaining_depth),
            ConversionNode::Literal { values, .. } => convert_literal(py, values, value, validated),
            ConversionNode::Union {
                branches,
                discriminator,
            } => self.convert_union(
                py,
                branches,
                discriminator.as_ref(),
                value,
                validated,
                remaining_depth,
            ),
            ConversionNode::Model {
                model_type,
                fields,
                fields_by_json_name,
                extra_value,
                extra_py_name,
                ..
            } => self.convert_model(
                py,
                model_type,
                fields,
                fields_by_json_name,
                *extra_value,
                extra_py_name.as_ref(),
                value,
                validated,
                remaining_depth,
            ),
            ConversionNode::Root {
                model_type,
                value: value_node,
                root_py_name,
                ..
            } => {
                let converted =
                    self.convert(py, *value_node, value, validated, remaining_depth - 1)?;
                let instance = allocate_model(py, model_type, &self.object_new)?;
                set_model_attribute(py, &instance, root_py_name, &converted)?;
                Ok(instance.unbind())
            }
        }
    }

    fn new_frozen_list<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        self.frozen_list_type
            .bind(py)
            .call0()?
            .cast_into::<PyList>()
            .map_err(|_| PyErr::new::<PyTypeError, _>("frozen list type must inherit from list"))
    }

    fn new_frozen_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        self.frozen_dict_type
            .bind(py)
            .call0()?
            .cast_into::<PyDict>()
            .map_err(|_| PyErr::new::<PyTypeError, _>("frozen dict type must inherit from dict"))
    }

    fn freeze_python_json_value(
        &self,
        py: Python<'_>,
        value: &Bound<'_, PyAny>,
        remaining_depth: u16,
    ) -> PyResult<Py<PyAny>> {
        self.freeze_python_json_value_inner(py, value, remaining_depth, &mut HashSet::new())
    }

    fn freeze_python_json_value_inner(
        &self,
        py: Python<'_>,
        value: &Bound<'_, PyAny>,
        remaining_depth: u16,
        active_containers: &mut HashSet<usize>,
    ) -> PyResult<Py<PyAny>> {
        if remaining_depth == 0 {
            return Err(PyErr::new::<PyValueError, _>(
                "generated model conversion exceeds the maximum nesting depth",
            ));
        }
        if let Ok(items) = value.cast::<PyList>() {
            let identity = value.as_ptr() as usize;
            if !active_containers.insert(identity) {
                return Err(PyErr::new::<PyValueError, _>(
                    "cyclic containers are not JSON values",
                ));
            }
            let output = self.new_frozen_list(py)?;
            let result = items.iter().try_for_each(|item| {
                output.append(self.freeze_python_json_value_inner(
                    py,
                    &item,
                    remaining_depth - 1,
                    active_containers,
                )?)
            });
            active_containers.remove(&identity);
            result?;
            return Ok(output.into_any().unbind());
        }
        if let Ok(items) = value.cast::<PyTuple>() {
            let identity = value.as_ptr() as usize;
            if !active_containers.insert(identity) {
                return Err(PyErr::new::<PyValueError, _>(
                    "cyclic containers are not JSON values",
                ));
            }
            let output = self.new_frozen_list(py)?;
            let result = items.iter().try_for_each(|item| {
                output.append(self.freeze_python_json_value_inner(
                    py,
                    &item,
                    remaining_depth - 1,
                    active_containers,
                )?)
            });
            active_containers.remove(&identity);
            result?;
            return Ok(output.into_any().unbind());
        }
        if let Ok(properties) = value.cast::<PyDict>() {
            let identity = value.as_ptr() as usize;
            if !active_containers.insert(identity) {
                return Err(PyErr::new::<PyValueError, _>(
                    "cyclic containers are not JSON values",
                ));
            }
            let output = self.new_frozen_dict(py)?;
            let result = properties.iter().try_for_each(|(key, item)| {
                if key.cast::<PyString>().is_err() {
                    return Err(PyErr::new::<PyTypeError, _>(
                        "JSON object keys must be strings",
                    ));
                }
                output.set_item(
                    key,
                    self.freeze_python_json_value_inner(
                        py,
                        &item,
                        remaining_depth - 1,
                        active_containers,
                    )?,
                )
            });
            active_containers.remove(&identity);
            result?;
            return Ok(output.into_any().unbind());
        }
        if value.is_none()
            || value.is_instance_of::<PyBool>()
            || value.is_instance_of::<PyInt>()
            || value.is_instance_of::<PyString>()
        {
            return Ok(value.clone().unbind());
        }
        if value.is_instance_of::<PyFloat>() {
            if value.extract::<f64>()?.is_finite() {
                return Ok(value.clone().unbind());
            }
            return Err(PyErr::new::<PyValueError, _>("JSON numbers must be finite"));
        }
        Err(PyErr::new::<PyTypeError, _>(format!(
            "expected a JSON-compatible value, got {}",
            value.get_type().name()?
        )))
    }

    fn convert_list(
        &self,
        py: Python<'_>,
        item_node: usize,
        value: &Bound<'_, PyAny>,
        validated: bool,
        remaining_depth: u16,
    ) -> PyResult<Py<PyAny>> {
        let output = self.new_frozen_list(py)?;
        if let Ok(items) = value.cast::<PyList>() {
            for item in items {
                let converted =
                    self.convert(py, item_node, &item, validated, remaining_depth - 1)?;
                output.append(converted)?;
            }
        } else if validated {
            if let Ok(items) = value.cast::<PyTuple>() {
                for item in items {
                    let converted =
                        self.convert(py, item_node, &item, validated, remaining_depth - 1)?;
                    output.append(converted)?;
                }
            } else {
                return Err(expected_type("list", value)?);
            }
        } else {
            return Err(expected_type("list", value)?);
        }
        Ok(output.into_any().unbind())
    }

    fn convert_dict(
        &self,
        py: Python<'_>,
        key_node: usize,
        value_node: usize,
        value: &Bound<'_, PyAny>,
        validated: bool,
        remaining_depth: u16,
    ) -> PyResult<Py<PyAny>> {
        let input = value
            .cast::<PyDict>()
            .map_err(|_| expected_type("dict", value).unwrap())?;
        let output = self.new_frozen_dict(py)?;
        for (key, item) in input {
            let converted_key = self.convert(py, key_node, &key, validated, remaining_depth - 1)?;
            let converted_value =
                self.convert(py, value_node, &item, validated, remaining_depth - 1)?;
            output.set_item(converted_key, converted_value)?;
        }
        Ok(output.into_any().unbind())
    }

    fn convert_union(
        &self,
        py: Python<'_>,
        branches: &[usize],
        discriminator: Option<&DiscriminatorPlan>,
        value: &Bound<'_, PyAny>,
        validated: bool,
        remaining_depth: u16,
    ) -> PyResult<Py<PyAny>> {
        if let (Some(plan), Ok(object)) = (discriminator, value.cast::<PyDict>())
            && let Some(tag) = object.get_item(&plan.json_name)?
            && let Ok(tag) = tag.extract::<String>()
            && let Some(branch) = plan.branches_by_value.get(&tag)
        {
            return self.convert(py, *branch, value, validated, remaining_depth - 1);
        }

        let mut matching_branch = None;
        let mut matching_count = 0;
        for branch in branches {
            if self.node_matches_kind(py, *branch, value)? {
                matching_branch = Some(*branch);
                matching_count += 1;
            }
        }
        if matching_count == 1 {
            return self.convert(
                py,
                matching_branch.expect("one matching branch must be present"),
                value,
                validated,
                remaining_depth - 1,
            );
        }

        for branch in branches {
            if matching_count > 0 && !self.node_matches_kind(py, *branch, value)? {
                continue;
            }
            if let Ok(converted) = self.convert(py, *branch, value, validated, remaining_depth - 1)
            {
                return Ok(converted);
            }
        }
        Err(PyErr::new::<PyTypeError, _>(
            "value does not match any generated model union branch",
        ))
    }

    fn node_matches_kind(
        &self,
        py: Python<'_>,
        node_id: usize,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<bool> {
        let node = self.nodes.get(node_id).ok_or_else(|| {
            PyErr::new::<PyIndexError, _>(format!("model converter node {node_id} is missing"))
        })?;
        Ok(match node {
            ConversionNode::Scalar { kind, .. } => match kind {
                ScalarKind::Any => true,
                ScalarKind::Missing => false,
                ScalarKind::String => value.is_instance_of::<PyString>(),
                ScalarKind::Integer => {
                    (value.is_instance_of::<PyInt>() && !value.is_instance_of::<PyBool>())
                        || (value.is_instance_of::<PyFloat>()
                            && value.extract::<f64>()?.fract() == 0.0)
                }
                ScalarKind::Number => {
                    !value.is_instance_of::<PyBool>()
                        && (value.is_instance_of::<PyInt>() || value.is_instance_of::<PyFloat>())
                }
                ScalarKind::Boolean => value.is_instance_of::<PyBool>(),
                ScalarKind::Null => value.is_none(),
            },
            ConversionNode::List { .. } => value.is_instance_of::<PyList>(),
            ConversionNode::Dict { .. } | ConversionNode::Model { .. } => {
                value.is_instance_of::<PyDict>()
            }
            ConversionNode::Literal { values, .. } => literal_index(py, values, value)?.is_some(),
            ConversionNode::Union { branches, .. } => {
                let mut matches = false;
                for branch in branches {
                    if self.node_matches_kind(py, *branch, value)? {
                        matches = true;
                        break;
                    }
                }
                matches
            }
            ConversionNode::Root { value: child, .. } => {
                self.node_matches_kind(py, *child, value)?
            }
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn convert_model(
        &self,
        py: Python<'_>,
        model_type: &Py<PyType>,
        fields: &[FieldPlan],
        fields_by_json_name: &HashMap<String, usize>,
        extra_value: Option<usize>,
        extra_py_name: Option<&Py<PyString>>,
        value: &Bound<'_, PyAny>,
        validated: bool,
        remaining_depth: u16,
    ) -> PyResult<Py<PyAny>> {
        let input = value
            .cast::<PyDict>()
            .map_err(|_| expected_type("JSON object", value).unwrap())?;
        let instance = allocate_model(py, model_type, &self.object_new)?;
        for field in fields {
            let converted = if let Some(field_value) = input.get_item(&field.json_name)? {
                self.convert(
                    py,
                    field.value_node,
                    &field_value,
                    validated,
                    remaining_depth - 1,
                )?
            } else {
                field.missing_factory.bind(py).call0()?.unbind()
            };
            set_model_attribute(py, &instance, &field.py_name, &converted)?;
        }

        let extra = if let Some(extra_node) = extra_value {
            let output = self.new_frozen_dict(py)?;
            for (key, item) in input {
                let key_string = key.extract::<String>().map_err(|_| {
                    PyErr::new::<PyTypeError, _>("JSON object keys must be strings")
                })?;
                if !fields_by_json_name.contains_key(&key_string) {
                    let converted =
                        self.convert(py, extra_node, &item, validated, remaining_depth - 1)?;
                    output.set_item(key, converted)?;
                }
            }
            Some(output.into_any().unbind())
        } else {
            None
        };

        if let (Some(name), Some(extra)) = (extra_py_name, extra.as_ref()) {
            set_model_attribute(py, &instance, name, extra)?;
        }
        Ok(instance.unbind())
    }

    fn convert_jiter(
        &self,
        py: Python<'_>,
        node_id: usize,
        value: &JiterJsonValue<'_>,
    ) -> PyResult<Py<PyAny>> {
        let node = self.nodes.get(node_id).ok_or_else(|| {
            PyErr::new::<PyIndexError, _>(format!("model converter node {node_id} is missing"))
        })?;
        match node {
            ConversionNode::Scalar { kind, .. } => {
                let converted = convert_jiter_scalar_value(py, *kind, value)?;
                if matches!(kind, ScalarKind::Any) {
                    self.freeze_python_json_value(py, converted.bind(py), MAX_MODEL_DEPTH)
                } else {
                    Ok(converted)
                }
            }
            ConversionNode::Literal {
                values,
                string_indices,
            } => {
                if let JiterJsonValue::Str(value) = value
                    && let Some(index) = string_indices.get(value.as_ref())
                {
                    return Ok(values[*index].clone_ref(py));
                }
                let python_value = value.into_pyobject(py)?.unbind();
                convert_literal(py, values, python_value.bind(py), false)
            }
            ConversionNode::List { item } => {
                let JiterJsonValue::Array(items) = value else {
                    return Err(PyErr::new::<PyTypeError, _>("expected list"));
                };
                let converted = self.new_frozen_list(py)?;
                for item_value in items.iter() {
                    converted.append(self.convert_jiter(py, *item, item_value)?)?;
                }
                Ok(converted.into_any().unbind())
            }
            ConversionNode::Dict {
                key,
                value: value_node,
            } => {
                let JiterJsonValue::Object(entries) = value else {
                    return Err(PyErr::new::<PyTypeError, _>("expected dict"));
                };
                let output = self.new_frozen_dict(py)?;
                let mut seen = HashSet::with_capacity(entries.len());
                for (key_value, item) in entries.iter() {
                    if !seen.insert(key_value.as_ref()) {
                        return Err(duplicate_key(key_value));
                    }
                    let jiter_key = JiterJsonValue::Str(key_value.clone());
                    let converted_key = self.convert_jiter(py, *key, &jiter_key)?;
                    let converted_value = self.convert_jiter(py, *value_node, item)?;
                    output.set_item(converted_key, converted_value)?;
                }
                Ok(output.into_any().unbind())
            }
            ConversionNode::Union {
                branches,
                discriminator,
            } => self.convert_jiter_union_value(py, branches, discriminator.as_ref(), value),
            ConversionNode::Model {
                model_type,
                fields,
                fields_by_json_name,
                extra_value,
                extra_py_name,
                ..
            } => self.convert_jiter_model_value(
                py,
                model_type,
                fields,
                fields_by_json_name,
                *extra_value,
                extra_py_name.as_ref(),
                value,
            ),
            ConversionNode::Root {
                model_type,
                value: value_node,
                root_py_name,
                ..
            } => {
                let converted = self.convert_jiter(py, *value_node, value)?;
                let instance = allocate_model(py, model_type, &self.object_new)?;
                set_model_attribute(py, &instance, root_py_name, &converted)?;
                Ok(instance.unbind())
            }
        }
    }

    fn convert_jiter_union_value(
        &self,
        py: Python<'_>,
        branches: &[usize],
        discriminator: Option<&DiscriminatorPlan>,
        value: &JiterJsonValue<'_>,
    ) -> PyResult<Py<PyAny>> {
        if let (Some(plan), JiterJsonValue::Object(entries)) = (discriminator, value)
            && let Some((_, JiterJsonValue::Str(tag))) = entries
                .iter()
                .find(|(key, _)| key.as_ref() == plan.json_name)
            && let Some(branch) = plan.branches_by_value.get(tag.as_ref())
        {
            return self.convert_jiter(py, *branch, value);
        }

        let mut matching_count = 0;
        let mut sole_match = None;
        for branch in branches {
            if self.jiter_node_matches_kind(*branch, value) {
                matching_count += 1;
                sole_match = Some(*branch);
            }
        }
        if matching_count == 1 {
            return self.convert_jiter(
                py,
                sole_match.expect("one matching branch records its node"),
                value,
            );
        }
        for branch in branches {
            if matching_count != 0 && !self.jiter_node_matches_kind(*branch, value) {
                continue;
            }
            if let Ok(converted) = self.convert_jiter(py, *branch, value) {
                return Ok(converted);
            }
        }
        Err(PyErr::new::<PyTypeError, _>(
            "value does not match any generated model union branch",
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn convert_jiter_model_value(
        &self,
        py: Python<'_>,
        model_type: &Py<PyType>,
        fields: &[FieldPlan],
        fields_by_json_name: &HashMap<String, usize>,
        extra_value: Option<usize>,
        extra_py_name: Option<&Py<PyString>>,
        value: &JiterJsonValue<'_>,
    ) -> PyResult<Py<PyAny>> {
        let JiterJsonValue::Object(entries) = value else {
            return Err(PyErr::new::<PyTypeError, _>("expected JSON object"));
        };
        let instance = allocate_model(py, model_type, &self.object_new)?;
        let extra_output = match extra_value {
            Some(_) => Some(self.new_frozen_dict(py)?),
            None => None,
        };
        let mut dropped_keys = None;
        let mut present_fields = 0;

        for (key, item) in entries.iter() {
            let key_string = key.as_ref();
            if let Some(field_index) = fields_by_json_name.get(key_string) {
                let field = &fields[*field_index];
                if model_attribute_is_set(
                    py,
                    &instance,
                    model_type,
                    &field.py_name,
                    field.slot_offset,
                )? {
                    return Err(duplicate_key(key));
                }
                let converted = self.convert_jiter(py, field.value_node, item)?;
                set_model_attribute(py, &instance, &field.py_name, &converted)?;
                present_fields += 1;
            } else if let (Some(extra_node), Some(output)) = (extra_value, extra_output.as_ref()) {
                if output.contains(key_string)? {
                    return Err(duplicate_key(key));
                }
                output.set_item(key_string, self.convert_jiter(py, extra_node, item)?)?;
            } else if !dropped_keys
                .get_or_insert_with(HashSet::new)
                .insert(key_string)
            {
                return Err(duplicate_key(key));
            }
        }

        let extra = extra_output.map(|output| output.into_any().unbind());
        if present_fields != fields.len() {
            for field in fields {
                if !model_attribute_is_set(
                    py,
                    &instance,
                    model_type,
                    &field.py_name,
                    field.slot_offset,
                )? {
                    let converted = field.missing_factory.bind(py).call0()?.unbind();
                    set_model_attribute(py, &instance, &field.py_name, &converted)?;
                }
            }
        }
        if let (Some(name), Some(extra)) = (extra_py_name, extra.as_ref()) {
            set_model_attribute(py, &instance, name, extra)?;
        }
        Ok(instance.unbind())
    }

    fn jiter_node_matches_kind(&self, node_id: usize, value: &JiterJsonValue<'_>) -> bool {
        let Some(node) = self.nodes.get(node_id) else {
            return false;
        };
        match node {
            ConversionNode::Scalar { kind, .. } => match kind {
                ScalarKind::Any => true,
                ScalarKind::Missing => false,
                ScalarKind::String => matches!(value, JiterJsonValue::Str(_)),
                ScalarKind::Integer => match value {
                    JiterJsonValue::Int(_) | JiterJsonValue::BigInt(_) => true,
                    JiterJsonValue::Float(value) => value.fract() == 0.0,
                    _ => false,
                },
                ScalarKind::Number => matches!(
                    value,
                    JiterJsonValue::Int(_) | JiterJsonValue::BigInt(_) | JiterJsonValue::Float(_)
                ),
                ScalarKind::Boolean => matches!(value, JiterJsonValue::Bool(_)),
                ScalarKind::Null => matches!(value, JiterJsonValue::Null),
            },
            ConversionNode::List { .. } => matches!(value, JiterJsonValue::Array(_)),
            ConversionNode::Dict { .. } | ConversionNode::Model { .. } => {
                matches!(value, JiterJsonValue::Object(_))
            }
            ConversionNode::Literal { .. } => true,
            ConversionNode::Union { branches, .. } => branches
                .iter()
                .any(|branch| self.jiter_node_matches_kind(*branch, value)),
            ConversionNode::Root { value: child, .. } => {
                self.jiter_node_matches_kind(*child, value)
            }
        }
    }

    fn write_json_node<'py>(
        &self,
        py: Python<'py>,
        node_id: usize,
        value: &Bound<'py, PyAny>,
        remaining_depth: u16,
        output: &mut Vec<u8>,
    ) -> PyResult<()> {
        if remaining_depth == 0 {
            return Err(PyErr::new::<PyValueError, _>(
                "generated model serialization exceeds the maximum nesting depth",
            ));
        }
        let node = self.nodes.get(node_id).ok_or_else(|| {
            PyErr::new::<PyIndexError, _>(format!("model converter node {node_id} is missing"))
        })?;
        match node {
            ConversionNode::Scalar { .. } | ConversionNode::Literal { .. } => {
                write_serializable_json_value(output, value)
            }
            ConversionNode::List { item } => {
                let items = value
                    .cast::<PyList>()
                    .map_err(|_| expected_type("list", value).unwrap())?;
                output.push(b'[');
                for (index, item_value) in items.iter().enumerate() {
                    if index != 0 {
                        output.push(b',');
                    }
                    self.write_json_node(py, *item, &item_value, remaining_depth - 1, output)?;
                }
                output.push(b']');
                Ok(())
            }
            ConversionNode::Dict {
                key: _,
                value: value_node,
            } => {
                let input = value
                    .cast::<PyDict>()
                    .map_err(|_| expected_type("dict", value).unwrap())?;
                let mut entries = Vec::with_capacity(input.len());
                for (key, item) in input {
                    let key = key.extract::<String>().map_err(|_| {
                        PyErr::new::<PyTypeError, _>("JSON object keys must be strings")
                    })?;
                    entries.push((key, item));
                }
                entries.sort_unstable_by(|left, right| left.0.cmp(&right.0));
                output.push(b'{');
                for (index, (key, item)) in entries.into_iter().enumerate() {
                    if index != 0 {
                        output.push(b',');
                    }
                    write_json_string(output, &key)?;
                    output.push(b':');
                    self.write_json_node(py, *value_node, &item, remaining_depth - 1, output)?;
                }
                output.push(b'}');
                Ok(())
            }
            ConversionNode::Union { branches, .. } => {
                let checkpoint = output.len();
                let mut first_error = None;
                for branch in branches {
                    if self.node_matches_model_value(py, *branch, value)? {
                        match self.write_json_node(py, *branch, value, remaining_depth - 1, output)
                        {
                            Ok(()) => return Ok(()),
                            Err(error) if first_error.is_none() => {
                                output.truncate(checkpoint);
                                first_error = Some(error);
                            }
                            Err(_) => output.truncate(checkpoint),
                        }
                    }
                }
                if let Some(error) = first_error {
                    return Err(error);
                }
                Err(PyErr::new::<PyTypeError, _>(
                    "value does not match any generated model union branch",
                ))
            }
            ConversionNode::Model {
                model_type,
                fields,
                serialized_fields,
                extra_value,
                extra_py_name,
                extra_slot_offset,
                ..
            } => {
                if !value.is_instance(model_type.bind(py))? {
                    let expected = model_type.bind(py).name()?.to_str()?.to_owned();
                    return Err(expected_type(&expected, value)?);
                }

                let mut field_entries = Vec::with_capacity(fields.len());
                for field_index in serialized_fields {
                    let field = &fields[*field_index];
                    let field_value = model_attribute_bound(
                        value,
                        model_type,
                        &field.py_name,
                        field.slot_offset,
                    )?;
                    if field
                        .missing_sentinel
                        .as_ref()
                        .is_some_and(|sentinel| field_value.is(sentinel.bind(py)))
                    {
                        continue;
                    }
                    field_entries.push((field, field_value));
                }

                output.push(b'{');
                let mut first = true;
                if let (Some(extra_node), Some(extra_name)) = (extra_value, extra_py_name) {
                    let extra =
                        model_attribute_bound(value, model_type, extra_name, *extra_slot_offset)?;
                    let extra = extra.cast::<PyDict>().map_err(|_| {
                        PyErr::new::<PyTypeError, _>(
                            "generated additional properties must be a dict",
                        )
                    })?;
                    let mut extra_entries = Vec::with_capacity(extra.len());
                    for (key, item) in extra {
                        let key = key.extract::<String>().map_err(|_| {
                            PyErr::new::<PyTypeError, _>("JSON object keys must be strings")
                        })?;
                        extra_entries.push((key, item));
                    }
                    extra_entries.sort_unstable_by(|left, right| left.0.cmp(&right.0));

                    // Merge the sorted schema fields and additional properties.
                    // Additional properties win collisions, matching the
                    // materializing JsonMap encoder without allocating a map.
                    let mut field_index = 0;
                    let mut extra_index = 0;
                    while field_index < field_entries.len() || extra_index < extra_entries.len() {
                        match (
                            field_entries.get(field_index),
                            extra_entries.get(extra_index),
                        ) {
                            (Some((field, field_value)), Some((extra_key, extra_value))) => {
                                match field.json_name.as_str().cmp(extra_key.as_str()) {
                                    std::cmp::Ordering::Less => {
                                        self.write_json_object_entry(
                                            py,
                                            &field.json_name,
                                            field.value_node,
                                            field_value,
                                            remaining_depth - 1,
                                            output,
                                            &mut first,
                                        )?;
                                        field_index += 1;
                                    }
                                    std::cmp::Ordering::Equal => {
                                        self.write_json_object_entry(
                                            py,
                                            extra_key,
                                            *extra_node,
                                            extra_value,
                                            remaining_depth - 1,
                                            output,
                                            &mut first,
                                        )?;
                                        field_index += 1;
                                        extra_index += 1;
                                    }
                                    std::cmp::Ordering::Greater => {
                                        self.write_json_object_entry(
                                            py,
                                            extra_key,
                                            *extra_node,
                                            extra_value,
                                            remaining_depth - 1,
                                            output,
                                            &mut first,
                                        )?;
                                        extra_index += 1;
                                    }
                                }
                            }
                            (Some((field, field_value)), None) => {
                                self.write_json_object_entry(
                                    py,
                                    &field.json_name,
                                    field.value_node,
                                    field_value,
                                    remaining_depth - 1,
                                    output,
                                    &mut first,
                                )?;
                                field_index += 1;
                            }
                            (None, Some((extra_key, extra_value))) => {
                                self.write_json_object_entry(
                                    py,
                                    extra_key,
                                    *extra_node,
                                    extra_value,
                                    remaining_depth - 1,
                                    output,
                                    &mut first,
                                )?;
                                extra_index += 1;
                            }
                            (None, None) => break,
                        }
                    }
                } else {
                    for (field, field_value) in field_entries {
                        self.write_json_object_entry(
                            py,
                            &field.json_name,
                            field.value_node,
                            &field_value,
                            remaining_depth - 1,
                            output,
                            &mut first,
                        )?;
                    }
                }
                output.push(b'}');
                Ok(())
            }
            ConversionNode::Root {
                model_type,
                value: value_node,
                root_py_name,
                root_slot_offset,
            } => {
                if !value.is_instance(model_type.bind(py))? {
                    let expected = model_type.bind(py).name()?.to_str()?.to_owned();
                    return Err(expected_type(&expected, value)?);
                }
                let root =
                    model_attribute_bound(value, model_type, root_py_name, *root_slot_offset)?;
                self.write_json_node(py, *value_node, &root, remaining_depth - 1, output)
            }
        }
    }

    fn to_python_value_node(
        &self,
        py: Python<'_>,
        node_id: usize,
        value: &Bound<'_, PyAny>,
        remaining_depth: u16,
    ) -> PyResult<Py<PyAny>> {
        if remaining_depth == 0 {
            return Err(PyErr::new::<PyValueError, _>(
                "generated model serialization exceeds the maximum nesting depth",
            ));
        }
        let node = self.nodes.get(node_id).ok_or_else(|| {
            PyErr::new::<PyIndexError, _>(format!("model converter node {node_id} is missing"))
        })?;
        match node {
            ConversionNode::Scalar {
                kind: ScalarKind::Any,
                ..
            } => copy_python_json_value(py, value, remaining_depth - 1),
            ConversionNode::Scalar { .. } | ConversionNode::Literal { .. } => {
                Ok(value.clone().unbind())
            }
            ConversionNode::List { item } => {
                let input = value
                    .cast::<PyList>()
                    .map_err(|_| expected_type("list", value).unwrap())?;
                let output = PyList::empty(py);
                for item_value in input {
                    output.append(self.to_python_value_node(
                        py,
                        *item,
                        &item_value,
                        remaining_depth - 1,
                    )?)?;
                }
                Ok(output.into_any().unbind())
            }
            ConversionNode::Dict {
                key: _,
                value: value_node,
            } => {
                let input = value
                    .cast::<PyDict>()
                    .map_err(|_| expected_type("dict", value).unwrap())?;
                let output = PyDict::new(py);
                for (key, item) in input {
                    if !key.is_instance_of::<PyString>() {
                        return Err(PyErr::new::<PyTypeError, _>(
                            "JSON object keys must be strings",
                        ));
                    }
                    output.set_item(
                        key,
                        self.to_python_value_node(py, *value_node, &item, remaining_depth - 1)?,
                    )?;
                }
                Ok(output.into_any().unbind())
            }
            ConversionNode::Union { branches, .. } => {
                let mut first_error = None;
                for branch in branches {
                    if self.node_matches_model_value(py, *branch, value)? {
                        match self.to_python_value_node(py, *branch, value, remaining_depth - 1) {
                            Ok(converted) => return Ok(converted),
                            Err(error) if first_error.is_none() => first_error = Some(error),
                            Err(_) => {}
                        }
                    }
                }
                if let Some(error) = first_error {
                    return Err(error);
                }
                Err(PyErr::new::<PyTypeError, _>(
                    "value does not match any generated model union branch",
                ))
            }
            ConversionNode::Model {
                model_type,
                fields,
                extra_value,
                extra_py_name,
                extra_slot_offset,
                ..
            } => {
                if !value.is_instance(model_type.bind(py))? {
                    let expected = model_type.bind(py).name()?.to_str()?.to_owned();
                    return Err(expected_type(&expected, value)?);
                }
                let output = PyDict::new(py);
                for field in fields {
                    let field_value = model_attribute_bound(
                        value,
                        model_type,
                        &field.py_name,
                        field.slot_offset,
                    )?;
                    if field
                        .missing_sentinel
                        .as_ref()
                        .is_some_and(|sentinel| field_value.is(sentinel.bind(py)))
                    {
                        continue;
                    }
                    output.set_item(
                        field.json_name.as_str(),
                        self.to_python_value_node(
                            py,
                            field.value_node,
                            &field_value,
                            remaining_depth - 1,
                        )?,
                    )?;
                }
                if let (Some(extra_node), Some(extra_name)) = (extra_value, extra_py_name) {
                    let extra =
                        model_attribute_bound(value, model_type, extra_name, *extra_slot_offset)?;
                    let extra = extra.cast::<PyDict>().map_err(|_| {
                        PyErr::new::<PyTypeError, _>(
                            "generated additional properties must be a dict",
                        )
                    })?;
                    for (key, item) in extra {
                        if !key.is_instance_of::<PyString>() {
                            return Err(PyErr::new::<PyTypeError, _>(
                                "JSON object keys must be strings",
                            ));
                        }
                        output.set_item(
                            key,
                            self.to_python_value_node(py, *extra_node, &item, remaining_depth - 1)?,
                        )?;
                    }
                }
                Ok(output.into_any().unbind())
            }
            ConversionNode::Root {
                model_type,
                value: value_node,
                root_py_name,
                root_slot_offset,
            } => {
                if !value.is_instance(model_type.bind(py))? {
                    let expected = model_type.bind(py).name()?.to_str()?.to_owned();
                    return Err(expected_type(&expected, value)?);
                }
                let root =
                    model_attribute_bound(value, model_type, root_py_name, *root_slot_offset)?;
                self.to_python_value_node(py, *value_node, &root, remaining_depth - 1)
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn write_json_object_entry<'py>(
        &self,
        py: Python<'py>,
        key: &str,
        value_node: usize,
        value: &Bound<'py, PyAny>,
        remaining_depth: u16,
        output: &mut Vec<u8>,
        first: &mut bool,
    ) -> PyResult<()> {
        if *first {
            *first = false;
        } else {
            output.push(b',');
        }
        write_json_string(output, key)?;
        output.push(b':');
        self.write_json_node(py, value_node, value, remaining_depth, output)
    }

    fn node_matches_model_value(
        &self,
        py: Python<'_>,
        node_id: usize,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<bool> {
        let Some(node) = self.nodes.get(node_id) else {
            return Ok(false);
        };
        Ok(match node {
            ConversionNode::Scalar { kind, .. } => match kind {
                ScalarKind::Any => true,
                ScalarKind::Missing => self.node_value_is_missing(py, node_id, value),
                ScalarKind::String => value.is_instance_of::<PyString>(),
                ScalarKind::Integer => {
                    value.is_instance_of::<PyInt>() && !value.is_instance_of::<PyBool>()
                }
                ScalarKind::Number => {
                    !value.is_instance_of::<PyBool>()
                        && (value.is_instance_of::<PyInt>() || value.is_instance_of::<PyFloat>())
                }
                ScalarKind::Boolean => value.is_instance_of::<PyBool>(),
                ScalarKind::Null => value.is_none(),
            },
            ConversionNode::List { .. } => value.is_instance_of::<PyList>(),
            ConversionNode::Dict { .. } => value.is_instance_of::<PyDict>(),
            ConversionNode::Literal { values, .. } => literal_index(py, values, value)?.is_some(),
            ConversionNode::Union { branches, .. } => {
                let mut matches = false;
                for branch in branches {
                    if self.node_matches_model_value(py, *branch, value)? {
                        matches = true;
                        break;
                    }
                }
                matches
            }
            ConversionNode::Model { model_type, .. } | ConversionNode::Root { model_type, .. } => {
                value.is_instance(model_type.bind(py))?
            }
        })
    }

    fn node_value_is_missing(
        &self,
        py: Python<'_>,
        node_id: usize,
        value: &Bound<'_, PyAny>,
    ) -> bool {
        match self.nodes.get(node_id) {
            Some(ConversionNode::Scalar {
                kind: ScalarKind::Missing,
                missing_sentinel: Some(sentinel),
            }) => value.is(sentinel.bind(py)),
            Some(ConversionNode::Union { branches, .. }) => branches
                .iter()
                .any(|branch| self.node_value_is_missing(py, *branch, value)),
            _ => false,
        }
    }
}

impl<'converter> ModelProjection<'converter> {
    pub(crate) fn instance<'a, 'py>(&'a self, value: &'a Bound<'py, PyAny>) -> JsonInstanceRef<'a>
    where
        'py: 'a,
    {
        JsonInstanceRef::from_projected_python(value, self.converter.root, self)
    }

    fn retain<'a>(&'a self, node: usize, value: Bound<'a, PyAny>) -> ProjectedPythonValue<'a> {
        let py = value.py();
        let pointer = value.as_ptr();
        self.retained.borrow_mut().push(value.unbind());
        // SAFETY: `retained` owns a reference to this object until the
        // projection is dropped, which is after every projected borrow.
        let borrowed: Borrowed<'a, 'a, PyAny> = unsafe { Borrowed::from_ptr(py, pointer) };
        ProjectedPythonValue::new(node, borrowed)
    }

    fn attribute<'a>(
        &'a self,
        value: ProjectedPythonValue<'a>,
        name: &Py<PyString>,
        node: usize,
    ) -> Option<ProjectedPythonValue<'a>> {
        let py = value.value().py();
        let attribute = value.value().getattr(name.bind(py)).ok()?;
        Some(self.retain(node, attribute))
    }

    fn model_attribute<'a>(
        &'a self,
        value: ProjectedPythonValue<'a>,
        model_type: &Py<PyType>,
        name: &Py<PyString>,
        slot_offset: Option<isize>,
        node: usize,
    ) -> Option<ProjectedPythonValue<'a>> {
        let py = value.value().py();
        let object = value.value().as_ptr();
        if let Some(slot_offset) = slot_offset {
            // SAFETY: the fast path is restricted to the exact class whose
            // member descriptor supplied `slot_offset`; that descriptor was
            // checked to contain an object pointer. The instance owns the
            // borrowed slot value for the projection lifetime.
            if unsafe { ffi::Py_TYPE(object) } == model_type.bind(py).as_ptr().cast() {
                let slot = unsafe {
                    object
                        .cast::<u8>()
                        .offset(slot_offset)
                        .cast::<*mut ffi::PyObject>()
                };
                let child = unsafe { *slot };
                return self.child_from_ptr(value, node, child);
            }
        }
        self.attribute(value, name, node)
    }

    fn child_from_ptr<'a>(
        &'a self,
        parent: ProjectedPythonValue<'a>,
        node: usize,
        child: *mut ffi::PyObject,
    ) -> Option<ProjectedPythonValue<'a>> {
        // SAFETY: callers obtain `child` as a borrowed entry from a Python
        // container retained by `parent` for the full projection lifetime.
        let child: Borrowed<'a, 'a, PyAny> =
            unsafe { Borrowed::from_ptr_or_opt(parent.value().py(), child) }?;
        Some(ProjectedPythonValue::new(node, child))
    }

    fn resolve<'a>(
        &'a self,
        value: ProjectedPythonValue<'a>,
        remaining_depth: u16,
    ) -> ProjectedPythonKind<'a> {
        if remaining_depth == 0 {
            return ProjectedPythonKind::Invalid;
        }
        let Some(node) = self.converter.nodes.get(value.node()) else {
            return ProjectedPythonKind::Invalid;
        };
        match node {
            ConversionNode::Scalar { .. } | ConversionNode::Literal { .. } => {
                ProjectedPythonKind::Native(value)
            }
            ConversionNode::List { .. } => {
                if value.value().cast::<PyList>().is_ok() {
                    ProjectedPythonKind::Array(value)
                } else {
                    ProjectedPythonKind::Invalid
                }
            }
            ConversionNode::Dict { .. } => {
                if value.value().cast::<PyDict>().is_ok() {
                    ProjectedPythonKind::Object(value)
                } else {
                    ProjectedPythonKind::Invalid
                }
            }
            ConversionNode::Union { branches, .. } => {
                let py = value.value().py();
                let cache_key = (value.node(), value.value().as_ptr() as usize);
                let selected = self.union_branches.borrow().get(&cache_key).copied();
                let selected = selected.or_else(|| {
                    let bound = value.value().to_owned();
                    let mut first_match = None;
                    let mut match_count = 0;
                    for branch in branches {
                        if self
                            .converter
                            .node_matches_model_value(py, *branch, &bound)
                            .unwrap_or(false)
                        {
                            first_match.get_or_insert(*branch);
                            match_count += 1;
                        }
                    }
                    if match_count == 1 {
                        let branch = first_match.expect("one matching branch was recorded");
                        return Some(branch);
                    }
                    for branch in branches {
                        if !self
                            .converter
                            .node_matches_model_value(py, *branch, &bound)
                            .unwrap_or(false)
                        {
                            continue;
                        }
                        let mut scratch = Vec::new();
                        if self
                            .converter
                            .write_json_node(py, *branch, &bound, remaining_depth - 1, &mut scratch)
                            .is_ok()
                        {
                            self.union_branches.borrow_mut().insert(cache_key, *branch);
                            return Some(*branch);
                        }
                    }
                    None
                });
                if let Some(selected) = selected {
                    self.resolve(
                        ProjectedPythonValue::new(selected, value.value()),
                        remaining_depth - 1,
                    )
                } else {
                    ProjectedPythonKind::Invalid
                }
            }
            ConversionNode::Model { model_type, .. } => {
                let py = value.value().py();
                if value
                    .value()
                    .is_instance(model_type.bind(py))
                    .unwrap_or(false)
                {
                    ProjectedPythonKind::Object(value)
                } else {
                    ProjectedPythonKind::Invalid
                }
            }
            ConversionNode::Root {
                model_type,
                value: value_node,
                root_py_name,
                root_slot_offset,
            } => {
                let py = value.value().py();
                if !value
                    .value()
                    .is_instance(model_type.bind(py))
                    .unwrap_or(false)
                {
                    return ProjectedPythonKind::Invalid;
                }
                self.model_attribute(
                    value,
                    model_type,
                    root_py_name,
                    *root_slot_offset,
                    *value_node,
                )
                .map_or(ProjectedPythonKind::Invalid, |root| {
                    self.resolve(root, remaining_depth - 1)
                })
            }
        }
    }

    fn normalized<'a>(&'a self, value: ProjectedPythonValue<'a>) -> ProjectedPythonValue<'a> {
        match self.resolve(value, MAX_MODEL_DEPTH) {
            ProjectedPythonKind::Native(value)
            | ProjectedPythonKind::Array(value)
            | ProjectedPythonKind::Object(value) => value,
            ProjectedPythonKind::Invalid => value,
        }
    }

    fn extra_dict<'a>(
        &'a self,
        value: ProjectedPythonValue<'a>,
        model_type: &Py<PyType>,
        extra_py_name: &Py<PyString>,
        extra_slot_offset: Option<isize>,
    ) -> Option<Borrowed<'a, 'a, PyDict>> {
        self.model_attribute(
            value,
            model_type,
            extra_py_name,
            extra_slot_offset,
            value.node(),
        )?
        .value()
        .cast::<PyDict>()
        .ok()
    }

    fn field_value<'a>(
        &'a self,
        value: ProjectedPythonValue<'a>,
        model_type: &Py<PyType>,
        field: &FieldPlan,
    ) -> Option<ProjectedPythonValue<'a>> {
        let child = self.model_attribute(
            value,
            model_type,
            &field.py_name,
            field.slot_offset,
            field.value_node,
        )?;
        if field
            .missing_sentinel
            .as_ref()
            .is_some_and(|sentinel| child.value().is(sentinel.bind(child.value().py())))
        {
            None
        } else {
            Some(self.normalized(child))
        }
    }

    fn dict_get<'a>(
        &'a self,
        dictionary: Borrowed<'a, 'a, PyDict>,
        key: &str,
        child_node: usize,
    ) -> Option<ProjectedPythonValue<'a>> {
        let child = dictionary.get_item(key).ok()??;
        let child = self.retain(child_node, child);
        Some(self.normalized(child))
    }

    fn dict_next<'a>(
        &'a self,
        value: ProjectedPythonValue<'a>,
        dictionary: Borrowed<'a, 'a, PyDict>,
        position: &mut usize,
        child_node: usize,
    ) -> Option<(&'a str, ProjectedPythonValue<'a>)> {
        let mut python_position = ffi::Py_ssize_t::try_from(*position).ok()?;
        let mut key = std::ptr::null_mut();
        let mut child = std::ptr::null_mut();
        // SAFETY: `dictionary` proves the input is a live dict. Successful
        // PyDict_Next calls return borrowed entries owned by that dict.
        if unsafe {
            ffi::PyDict_Next(
                dictionary.as_ptr(),
                &raw mut python_position,
                &raw mut key,
                &raw mut child,
            )
        } == 0
        {
            return None;
        }
        *position = usize::try_from(python_position).ok()?;
        // SAFETY: both pointers are non-null borrowed dict entries.
        let key: Borrowed<'a, 'a, PyAny> = unsafe { Borrowed::from_ptr(value.value().py(), key) };
        let key = borrowed_python_string(key)?;
        let child = self.child_from_ptr(value, child_node, child)?;
        Some((key, self.normalized(child)))
    }

    fn dict_keys_are_strings(dictionary: Borrowed<'_, '_, PyDict>) -> bool {
        dictionary
            .iter()
            .all(|(key, _)| key.cast::<PyString>().is_ok())
    }
}

impl PythonInstanceProvider for ModelProjection<'_> {
    fn project<'a>(&'a self, value: ProjectedPythonValue<'a>) -> ProjectedPythonKind<'a> {
        self.resolve(value, MAX_MODEL_DEPTH)
    }

    fn array_len(&self, value: ProjectedPythonValue<'_>) -> usize {
        let Some(ConversionNode::List { .. }) = self.converter.nodes.get(value.node()) else {
            return 0;
        };
        value
            .value()
            .cast::<PyList>()
            .map_or(0, |items| items.len())
    }

    fn array_get<'a>(
        &'a self,
        value: ProjectedPythonValue<'a>,
        index: usize,
    ) -> Option<ProjectedPythonValue<'a>> {
        let ConversionNode::List { item } = self.converter.nodes.get(value.node())? else {
            return None;
        };
        let items = value.value().cast::<PyList>().ok()?;
        if index >= items.len() {
            return None;
        }
        let index = ffi::Py_ssize_t::try_from(index).ok()?;
        // SAFETY: the type and bounds checks above prove this returns a
        // borrowed non-null list entry owned by `value`.
        let child = unsafe { ffi::PyList_GetItem(items.as_ptr(), index) };
        let child = self.child_from_ptr(value, *item, child)?;
        Some(self.normalized(child))
    }

    fn object_len(&self, value: ProjectedPythonValue<'_>) -> usize {
        match self.converter.nodes.get(value.node()) {
            Some(ConversionNode::Dict { .. }) => value
                .value()
                .cast::<PyDict>()
                .map_or(0, |dictionary| dictionary.len()),
            Some(ConversionNode::Model {
                model_type,
                fields,
                serialized_fields,
                required_field_count,
                omittable_fields,
                extra_py_name,
                extra_slot_offset,
                ..
            }) => {
                if extra_py_name.is_none() {
                    return *required_field_count
                        + omittable_fields
                            .iter()
                            .filter(|field_index| {
                                self.field_value(value, model_type, &fields[**field_index])
                                    .is_some()
                            })
                            .count();
                }
                let extra = extra_py_name
                    .as_ref()
                    .and_then(|name| self.extra_dict(value, model_type, name, *extra_slot_offset));
                let mut len = extra.map_or(0, |dictionary| dictionary.len());
                for field_index in serialized_fields {
                    let field = &fields[*field_index];
                    if extra.is_some_and(|dictionary| {
                        dictionary
                            .get_item(field.json_name.as_str())
                            .is_ok_and(|value| value.is_some())
                    }) {
                        continue;
                    }
                    if self.field_value(value, model_type, field).is_some() {
                        len += 1;
                    }
                }
                len
            }
            _ => 0,
        }
    }

    fn object_keys_are_strings(&self, value: ProjectedPythonValue<'_>) -> bool {
        match self.converter.nodes.get(value.node()) {
            Some(ConversionNode::Dict { .. }) => value
                .value()
                .cast::<PyDict>()
                .is_ok_and(Self::dict_keys_are_strings),
            Some(ConversionNode::Model {
                model_type,
                extra_py_name: Some(extra_py_name),
                extra_slot_offset,
                ..
            }) => self
                .extra_dict(value, model_type, extra_py_name, *extra_slot_offset)
                .is_some_and(Self::dict_keys_are_strings),
            Some(ConversionNode::Model { .. }) => true,
            _ => false,
        }
    }

    fn object_get<'a>(
        &'a self,
        value: ProjectedPythonValue<'a>,
        key: &str,
    ) -> Option<ProjectedPythonValue<'a>> {
        match self.converter.nodes.get(value.node())? {
            ConversionNode::Dict {
                value: child_node, ..
            } => {
                let dictionary = value.value().cast::<PyDict>().ok()?;
                self.dict_get(dictionary, key, *child_node)
            }
            ConversionNode::Model {
                model_type,
                fields,
                fields_by_json_name,
                extra_value,
                extra_py_name,
                extra_slot_offset,
                ..
            } => {
                if let (Some(extra_node), Some(extra_name)) = (extra_value, extra_py_name) {
                    let extra =
                        self.extra_dict(value, model_type, extra_name, *extra_slot_offset)?;
                    if let Some(child) = self.dict_get(extra, key, *extra_node) {
                        return Some(child);
                    }
                }
                let field = fields.get(*fields_by_json_name.get(key)?)?;
                self.field_value(value, model_type, field)
            }
            _ => None,
        }
    }

    fn object_next<'a>(
        &'a self,
        value: ProjectedPythonValue<'a>,
        state: &mut [usize; 2],
    ) -> Option<(&'a str, ProjectedPythonValue<'a>)> {
        match self.converter.nodes.get(value.node())? {
            ConversionNode::Dict {
                value: child_node, ..
            } => {
                let dictionary = value.value().cast::<PyDict>().ok()?;
                self.dict_next(value, dictionary, &mut state[0], *child_node)
            }
            ConversionNode::Model {
                model_type,
                fields,
                serialized_fields,
                extra_value,
                extra_py_name,
                extra_slot_offset,
                ..
            } => {
                let extra = extra_py_name
                    .as_ref()
                    .and_then(|name| self.extra_dict(value, model_type, name, *extra_slot_offset));
                while state[0] < serialized_fields.len() {
                    let field_index = serialized_fields[state[0]];
                    state[0] += 1;
                    let field = &fields[field_index];
                    if extra.is_some_and(|dictionary| {
                        dictionary
                            .get_item(field.json_name.as_str())
                            .is_ok_and(|value| value.is_some())
                    }) {
                        continue;
                    }
                    if let Some(child) = self.field_value(value, model_type, field) {
                        return Some((field.json_name.as_str(), child));
                    }
                }
                match (extra, extra_value) {
                    (Some(extra), Some(extra_node)) => {
                        self.dict_next(value, extra, &mut state[1], *extra_node)
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

fn convert_jiter_scalar_value(
    py: Python<'_>,
    kind: ScalarKind,
    value: &JiterJsonValue<'_>,
) -> PyResult<Py<PyAny>> {
    let valid = match kind {
        ScalarKind::Any => true,
        ScalarKind::Missing => false,
        ScalarKind::String => matches!(value, JiterJsonValue::Str(_)),
        ScalarKind::Integer => match value {
            JiterJsonValue::Int(_) | JiterJsonValue::BigInt(_) => true,
            JiterJsonValue::Float(value) => value.fract() == 0.0,
            _ => false,
        },
        ScalarKind::Number => matches!(
            value,
            JiterJsonValue::Int(_) | JiterJsonValue::BigInt(_) | JiterJsonValue::Float(_)
        ),
        ScalarKind::Boolean => matches!(value, JiterJsonValue::Bool(_)),
        ScalarKind::Null => matches!(value, JiterJsonValue::Null),
    };
    if !valid {
        return Err(PyErr::new::<PyTypeError, _>(format!(
            "expected {}",
            scalar_name(kind)
        )));
    }

    let python_value = if matches!(kind, ScalarKind::Integer) {
        if let JiterJsonValue::Float(number) = value {
            number.into_pyobject(py)?.call_method0("__int__")?.unbind()
        } else {
            value.into_pyobject(py)?.unbind()
        }
    } else {
        value.into_pyobject(py)?.unbind()
    };
    Ok(python_value)
}

fn duplicate_key(key: impl std::fmt::Display) -> PyErr {
    PyErr::new::<PyValueError, _>(format!("duplicate key: `{key}`"))
}

fn copy_python_json_value(
    py: Python<'_>,
    value: &Bound<'_, PyAny>,
    remaining_depth: u16,
) -> PyResult<Py<PyAny>> {
    if remaining_depth == 0 {
        return Err(PyErr::new::<PyValueError, _>(
            "generated model serialization exceeds the maximum nesting depth",
        ));
    }
    if let Ok(input) = value.cast::<PyList>() {
        let output = PyList::empty(py);
        for item in input {
            output.append(copy_python_json_value(py, &item, remaining_depth - 1)?)?;
        }
        return Ok(output.into_any().unbind());
    }
    if let Ok(input) = value.cast::<PyDict>() {
        let output = PyDict::new(py);
        for (key, item) in input {
            output.set_item(key, copy_python_json_value(py, &item, remaining_depth - 1)?)?;
        }
        return Ok(output.into_any().unbind());
    }
    if value.hasattr("jsoncompat_to_value_unchecked")? {
        return Ok(value
            .call_method0("jsoncompat_to_value_unchecked")?
            .unbind());
    }
    Ok(value.clone().unbind())
}

fn write_serializable_json_value(output: &mut Vec<u8>, value: &Bound<'_, PyAny>) -> PyResult<()> {
    if value.is_none() {
        output.extend_from_slice(b"null");
        return Ok(());
    }
    if value.is_instance_of::<PyBool>() {
        output.extend_from_slice(if value.extract()? { b"true" } else { b"false" });
        return Ok(());
    }
    if value.is_instance_of::<PyInt>() {
        if let Ok(number) = value.extract::<i64>() {
            return serde_json::to_writer(&mut *output, &number).map_err(json_serialization_error);
        }
        if let Ok(number) = value.extract::<u64>() {
            return serde_json::to_writer(&mut *output, &number).map_err(json_serialization_error);
        }
        return Err(PyErr::new::<PyOverflowError, _>(
            "JSON integer cannot be serialized losslessly by the native encoder",
        ));
    }
    if value.is_instance_of::<PyFloat>() {
        let number = value.extract::<f64>()?;
        if !number.is_finite() {
            return Err(PyErr::new::<PyValueError, _>("JSON numbers must be finite"));
        }
        let number = super::parse_json(value.str()?.to_str()?)?;
        return serde_json::to_writer(&mut *output, &number).map_err(json_serialization_error);
    }
    if let Ok(value) = value.cast::<PyString>() {
        return write_json_string(output, value.to_str()?);
    }

    let value = super::py_to_serializable_json_value(value)?;
    serde_json::to_writer(&mut *output, &value).map_err(json_serialization_error)
}

fn write_json_string(output: &mut Vec<u8>, value: &str) -> PyResult<()> {
    serde_json::to_writer(&mut *output, value).map_err(json_serialization_error)
}

fn borrowed_python_string<'a>(value: Borrowed<'a, 'a, PyAny>) -> Option<&'a str> {
    value.cast::<PyString>().ok()?;
    let mut size = 0;
    // SAFETY: the cast above proves this is Unicode. CPython owns its cached
    // UTF-8 buffer for at least the lifetime represented by `value`.
    let data = unsafe { ffi::PyUnicode_AsUTF8AndSize(value.as_ptr(), &raw mut size) };
    if data.is_null() {
        return None;
    }
    let size = usize::try_from(size).ok()?;
    // SAFETY: PyUnicode_AsUTF8AndSize returns valid UTF-8 of exactly `size` bytes.
    let bytes = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), size) };
    Some(unsafe { std::str::from_utf8_unchecked(bytes) })
}

fn json_serialization_error(error: serde_json::Error) -> PyErr {
    PyErr::new::<PyValueError, _>(format!("JSON serialization failed: {error}"))
}

fn convert_scalar(
    py: Python<'_>,
    kind: ScalarKind,
    missing_sentinel: Option<&Py<PyAny>>,
    value: &Bound<'_, PyAny>,
    validated: bool,
) -> PyResult<Py<PyAny>> {
    if validated {
        if matches!(kind, ScalarKind::Integer) && value.is_instance_of::<PyFloat>() {
            return Ok(value.call_method0("__int__")?.unbind());
        }
        return Ok(value.clone().unbind());
    }

    let valid = match kind {
        ScalarKind::Any => true,
        ScalarKind::Missing => missing_sentinel.is_some_and(|sentinel| value.is(sentinel.bind(py))),
        ScalarKind::String => value.is_instance_of::<PyString>(),
        ScalarKind::Integer => {
            if value.is_instance_of::<PyInt>() && !value.is_instance_of::<PyBool>() {
                return Ok(value.clone().unbind());
            }
            if value.is_instance_of::<PyFloat>() && value.extract::<f64>()?.fract() == 0.0 {
                return Ok(value.call_method0("__int__")?.unbind());
            }
            false
        }
        ScalarKind::Number => {
            !value.is_instance_of::<PyBool>()
                && (value.is_instance_of::<PyInt>() || value.is_instance_of::<PyFloat>())
        }
        ScalarKind::Boolean => value.is_instance_of::<PyBool>(),
        ScalarKind::Null => value.is_none(),
    };
    if valid {
        Ok(value.clone().unbind())
    } else {
        Err(expected_type(scalar_name(kind), value)?)
    }
}

fn scalar_name(kind: ScalarKind) -> &'static str {
    match kind {
        ScalarKind::Any => "JSON value",
        ScalarKind::Missing => "JSONCOMPAT_MISSING",
        ScalarKind::String => "str",
        ScalarKind::Integer => "int",
        ScalarKind::Number => "number",
        ScalarKind::Boolean => "bool",
        ScalarKind::Null => "null",
    }
}

fn convert_literal(
    py: Python<'_>,
    values: &[Py<PyAny>],
    value: &Bound<'_, PyAny>,
    validated: bool,
) -> PyResult<Py<PyAny>> {
    if validated {
        return Ok(value.clone().unbind());
    }
    if let Some(index) = literal_index(py, values, value)? {
        Ok(values[index].clone_ref(py))
    } else {
        Err(PyErr::new::<PyTypeError, _>(
            "value does not match the generated literal",
        ))
    }
}

fn literal_index(
    py: Python<'_>,
    values: &[Py<PyAny>],
    value: &Bound<'_, PyAny>,
) -> PyResult<Option<usize>> {
    for (index, literal) in values.iter().enumerate() {
        let literal = literal.bind(py);
        let either_bool = literal.is_instance_of::<PyBool>() || value.is_instance_of::<PyBool>();
        let both_numbers = !either_bool
            && (literal.is_instance_of::<PyInt>() || literal.is_instance_of::<PyFloat>())
            && (value.is_instance_of::<PyInt>() || value.is_instance_of::<PyFloat>());
        let same_type = literal.get_type().is(value.get_type());
        if (both_numbers || same_type) && literal.eq(value)? {
            return Ok(Some(index));
        }
    }
    Ok(None)
}

fn model_attribute_bound<'py>(
    value: &Bound<'py, PyAny>,
    model_type: &Py<PyType>,
    name: &Py<PyString>,
    slot_offset: Option<isize>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = value.py();
    if let Some(slot_offset) = slot_offset {
        let object = value.as_ptr();
        // SAFETY: the exact-type check ties the offset to the member descriptor
        // compiled for this object layout. The slot contains an owned Python
        // object pointer, and from_borrowed_ptr creates the returned owned view.
        if unsafe { ffi::Py_TYPE(object) } == model_type.bind(py).as_ptr().cast() {
            let slot = unsafe {
                object
                    .cast::<u8>()
                    .offset(slot_offset)
                    .cast::<*mut ffi::PyObject>()
            };
            let child = unsafe { *slot };
            if !child.is_null() {
                return Ok(unsafe { Bound::from_borrowed_ptr(py, child) });
            }
        }
    }
    value.getattr(name.bind(py))
}

fn model_attribute_is_set(
    py: Python<'_>,
    value: &Bound<'_, PyAny>,
    model_type: &Py<PyType>,
    name: &Py<PyString>,
    slot_offset: Option<isize>,
) -> PyResult<bool> {
    if let Some(slot_offset) = slot_offset {
        let object = value.as_ptr();
        // SAFETY: this is the same exact-type and descriptor-derived-offset
        // invariant used by `model_attribute_bound`; only pointer nullness is
        // inspected here.
        if unsafe { ffi::Py_TYPE(object) } == model_type.bind(py).as_ptr().cast() {
            let slot = unsafe {
                object
                    .cast::<u8>()
                    .offset(slot_offset)
                    .cast::<*mut ffi::PyObject>()
            };
            return Ok(!unsafe { *slot }.is_null());
        }
    }
    value.hasattr(name.bind(py))
}

fn expected_type(expected: &str, value: &Bound<'_, PyAny>) -> PyResult<PyErr> {
    Ok(PyErr::new::<PyTypeError, _>(format!(
        "expected {expected}, got {}",
        value.get_type().name()?
    )))
}

fn allocate_model<'py>(
    py: Python<'py>,
    model_type: &Py<PyType>,
    object_new: &Py<PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    object_new.bind(py).call1((model_type.bind(py),))
}

fn set_model_attribute(
    py: Python<'_>,
    instance: &Bound<'_, PyAny>,
    name: &Py<PyString>,
    value: &Py<PyAny>,
) -> PyResult<()> {
    // Frozen dataclasses intentionally reject `PyObject_SetAttr`; calling the
    // generic implementation is equivalent to `object.__setattr__` and writes
    // the generated slot descriptor during construction.
    let result = unsafe {
        ffi::PyObject_GenericSetAttr(
            instance.as_ptr(),
            name.bind(py).as_ptr(),
            value.bind(py).as_ptr(),
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(PyErr::fetch(py))
    }
}

pub(crate) fn compile_model_converter(
    py: Python<'_>,
    descriptors: &Bound<'_, PyList>,
    root: usize,
    frozen_list_type: &Bound<'_, PyType>,
    frozen_dict_type: &Bound<'_, PyType>,
) -> PyResult<ModelConverterPy> {
    let mut nodes = Vec::with_capacity(descriptors.len());
    for descriptor in descriptors {
        nodes.push(parse_node(py, &descriptor)?);
    }
    if root >= nodes.len() {
        return Err(PyErr::new::<PyIndexError, _>(
            "model converter root node is out of bounds",
        ));
    }
    validate_references(&nodes)?;
    let object_new = py.get_type::<PyAny>().getattr("__new__")?.unbind();
    Ok(ModelConverterPy {
        nodes,
        root,
        object_new,
        frozen_list_type: frozen_list_type.clone().unbind(),
        frozen_dict_type: frozen_dict_type.clone().unbind(),
        validated_py_name: PyString::new(py, "_jsoncompat_validated").unbind(),
    })
}

#[cfg(all(Py_3_11, not(any(PyPy, GraalPy))))]
fn python_slot_offset(model_type: &Bound<'_, PyType>, name: &str) -> Option<isize> {
    let descriptor = model_type.getattr(name).ok()?;
    // SAFETY: exact member descriptors use the public CPython
    // PyMemberDescrObject/PyMemberDef layout. We validate the descriptor type,
    // member pointer, and object-valued type code before retaining the offset.
    unsafe {
        if ffi::Py_IS_TYPE(
            descriptor.as_ptr(),
            std::ptr::addr_of_mut!(ffi::PyMemberDescr_Type),
        ) == 0
        {
            return None;
        }
        let descriptor = descriptor.as_ptr().cast::<ffi::PyMemberDescrObject>();
        let member = (*descriptor).d_member;
        if member.is_null() || (*member).name.is_null() {
            return None;
        }
        if (*member).type_code != ffi::Py_T_OBJECT_EX {
            return None;
        }
        Some((*member).offset)
    }
}

#[cfg(not(all(Py_3_11, not(any(PyPy, GraalPy)))))]
fn python_slot_offset(_model_type: &Bound<'_, PyType>, _name: &str) -> Option<isize> {
    None
}

fn parse_node(py: Python<'_>, descriptor: &Bound<'_, PyAny>) -> PyResult<ConversionNode> {
    let descriptor = descriptor.cast::<PyTuple>()?;
    let tag = descriptor.get_item(0)?.extract::<String>()?;
    match tag.as_str() {
        "any" => Ok(scalar_node(ScalarKind::Any)),
        "missing" => Ok(ConversionNode::Scalar {
            kind: ScalarKind::Missing,
            missing_sentinel: Some(descriptor.get_item(1)?.unbind()),
        }),
        "str" => Ok(scalar_node(ScalarKind::String)),
        "int" => Ok(scalar_node(ScalarKind::Integer)),
        "float" => Ok(scalar_node(ScalarKind::Number)),
        "bool" => Ok(scalar_node(ScalarKind::Boolean)),
        "null" => Ok(scalar_node(ScalarKind::Null)),
        "list" => Ok(ConversionNode::List {
            item: descriptor.get_item(1)?.extract()?,
        }),
        "dict" => Ok(ConversionNode::Dict {
            key: descriptor.get_item(1)?.extract()?,
            value: descriptor.get_item(2)?.extract()?,
        }),
        "literal" => {
            let values = descriptor.get_item(1)?;
            let values = values.cast::<PyTuple>()?;
            let values = values.iter().map(Bound::unbind).collect::<Vec<_>>();
            let string_indices = values
                .iter()
                .enumerate()
                .filter_map(|(index, value)| {
                    value
                        .bind(py)
                        .cast::<PyString>()
                        .ok()
                        .and_then(|value| value.to_str().ok())
                        .map(|value| (value.to_owned(), index))
                })
                .collect();
            Ok(ConversionNode::Literal {
                values,
                string_indices,
            })
        }
        "union" => parse_union_node(descriptor),
        "model" => parse_model_node(py, descriptor),
        "root" => {
            let model_type = descriptor.get_item(1)?.cast_into::<PyType>()?.unbind();
            let root_slot_offset = python_slot_offset(model_type.bind(py), "root");
            Ok(ConversionNode::Root {
                model_type,
                value: descriptor.get_item(2)?.extract()?,
                root_py_name: PyString::new(py, "root").unbind(),
                root_slot_offset,
            })
        }
        _ => Err(PyErr::new::<PyValueError, _>(format!(
            "unknown model converter node kind {tag:?}"
        ))),
    }
}

fn scalar_node(kind: ScalarKind) -> ConversionNode {
    ConversionNode::Scalar {
        kind,
        missing_sentinel: None,
    }
}

fn parse_union_node(descriptor: &Bound<'_, PyTuple>) -> PyResult<ConversionNode> {
    let branches = descriptor.get_item(1)?.cast_into::<PyTuple>()?;
    let branches = branches
        .iter()
        .map(|branch| branch.extract::<usize>())
        .collect::<PyResult<Vec<_>>>()?;
    let discriminator_name = descriptor.get_item(2)?;
    let discriminator = if discriminator_name.is_none() {
        None
    } else {
        let mapping = descriptor.get_item(3)?.cast_into::<PyDict>()?;
        let mut branches_by_value = HashMap::with_capacity(mapping.len());
        for (value, branch) in mapping {
            branches_by_value.insert(value.extract::<String>()?, branch.extract::<usize>()?);
        }
        Some(DiscriminatorPlan {
            json_name: discriminator_name.extract()?,
            branches_by_value,
        })
    };
    Ok(ConversionNode::Union {
        branches,
        discriminator,
    })
}

fn parse_model_node(py: Python<'_>, descriptor: &Bound<'_, PyTuple>) -> PyResult<ConversionNode> {
    let model_type = descriptor.get_item(1)?.cast_into::<PyType>()?;
    let field_descriptors = descriptor.get_item(2)?.cast_into::<PyTuple>()?;
    let mut fields = Vec::with_capacity(field_descriptors.len());
    let mut fields_by_json_name = HashMap::with_capacity(field_descriptors.len());
    for field in field_descriptors.iter() {
        let field = field.cast_into::<PyTuple>()?;
        let json_name = field.get_item(0)?.extract::<String>()?;
        let py_name = field.get_item(1)?.extract::<String>()?;
        fields_by_json_name.insert(json_name.clone(), fields.len());
        fields.push(FieldPlan {
            json_name,
            slot_offset: python_slot_offset(&model_type, &py_name),
            py_name: PyString::new(py, &py_name).unbind(),
            value_node: field.get_item(2)?.extract()?,
            missing_factory: field.get_item(3)?.unbind(),
            missing_sentinel: {
                let sentinel = field.get_item(4)?;
                if sentinel.is_none() {
                    None
                } else {
                    Some(sentinel.unbind())
                }
            },
        });
    }
    let extra_value = descriptor.get_item(3)?;
    let extra_value = if extra_value.is_none() {
        None
    } else {
        Some(extra_value.extract::<usize>()?)
    };
    let mut serialized_fields = (0..fields.len()).collect::<Vec<_>>();
    serialized_fields
        .sort_unstable_by(|left, right| fields[*left].json_name.cmp(&fields[*right].json_name));
    let omittable_fields = fields
        .iter()
        .enumerate()
        .filter_map(|(index, field)| field.missing_sentinel.as_ref().map(|_| index))
        .collect::<Vec<_>>();
    let required_field_count = fields.len() - omittable_fields.len();
    let extra_py_name = extra_value.map(|_| PyString::new(py, "__jsoncompat_extra__").unbind());
    let extra_slot_offset = extra_py_name
        .as_ref()
        .and_then(|_| python_slot_offset(&model_type, "__jsoncompat_extra__"));
    Ok(ConversionNode::Model {
        model_type: model_type.unbind(),
        fields,
        fields_by_json_name,
        serialized_fields,
        required_field_count,
        omittable_fields,
        extra_value,
        extra_py_name,
        extra_slot_offset,
    })
}

fn validate_references(nodes: &[ConversionNode]) -> PyResult<()> {
    let len = nodes.len();
    let check = |reference: usize| {
        if reference < len {
            Ok(())
        } else {
            Err(PyErr::new::<PyIndexError, _>(format!(
                "model converter node reference {reference} is out of bounds"
            )))
        }
    };
    for node in nodes {
        match node {
            ConversionNode::Scalar { .. } | ConversionNode::Literal { .. } => {}
            ConversionNode::List { item } => check(*item)?,
            ConversionNode::Dict { key, value } => {
                check(*key)?;
                check(*value)?;
            }
            ConversionNode::Union {
                branches,
                discriminator,
            } => {
                for branch in branches {
                    check(*branch)?;
                }
                if let Some(discriminator) = discriminator {
                    for branch in discriminator.branches_by_value.values() {
                        check(*branch)?;
                    }
                }
            }
            ConversionNode::Model {
                fields,
                extra_value,
                ..
            } => {
                for field in fields {
                    check(field.value_node)?;
                }
                if let Some(extra_value) = extra_value {
                    check(*extra_value)?;
                }
            }
            ConversionNode::Root { value, .. } => check(*value)?,
        }
    }
    Ok(())
}
