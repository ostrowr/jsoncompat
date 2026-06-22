//! Compiled conversion plans for generated Python dataclasses.
//!
//! Plans move repeated object-graph traversal into Rust while retaining the
//! Python runtime's existing type checks, missing-field factories, union
//! selection, and frozen-slot construction semantics.

use std::collections::{HashMap, HashSet};

use jiter::JsonValue as JiterJsonValue;
use pyo3::exceptions::{PyIndexError, PyOverflowError, PyTypeError, PyValueError};
use pyo3::ffi;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBool, PyDict, PyFloat, PyInt, PyList, PyString, PyTuple, PyType};
use serde_json::{Map as JsonMap, Number as JsonNumber, Value as JsonValue};

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
    },
    Union {
        branches: Vec<usize>,
        discriminator: Option<DiscriminatorPlan>,
    },
    Model {
        model_type: Py<PyType>,
        fields: Vec<FieldPlan>,
        fields_by_json_name: HashMap<String, usize>,
        extra_value: Option<usize>,
        extra_py_name: Option<Py<PyString>>,
    },
    Root {
        model_type: Py<PyType>,
        value: usize,
        root_py_name: Py<PyString>,
    },
}

#[pyclass(name = "ModelConverter", module = "jsoncompat._native", unsendable)]
pub(crate) struct ModelConverterPy {
    nodes: Vec<ConversionNode>,
    root: usize,
    object_new: Py<PyAny>,
}

#[pymethods]
impl ModelConverterPy {
    fn construct(
        &self,
        py: Python<'_>,
        value: &Bound<'_, PyAny>,
        validated: bool,
    ) -> PyResult<Py<PyAny>> {
        self.convert(py, self.root, value, validated, MAX_MODEL_DEPTH)
    }
}

impl ModelConverterPy {
    pub(crate) fn construct_python(
        &self,
        py: Python<'_>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<Py<PyAny>> {
        self.convert(py, self.root, value, false, MAX_MODEL_DEPTH)
    }

    pub(crate) fn construct_jiter(
        &self,
        py: Python<'_>,
        value: &JiterJsonValue<'_>,
    ) -> PyResult<Py<PyAny>> {
        self.convert_jiter(py, self.root, value)
    }

    pub(crate) fn serialize_to_json(
        &self,
        py: Python<'_>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<JsonValue> {
        self.serialize_node(py, self.root, value, MAX_MODEL_DEPTH)
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
            } => convert_scalar(py, *kind, missing_sentinel.as_ref(), value, validated),
            ConversionNode::List { item } => {
                self.convert_list(py, *item, value, validated, remaining_depth)
            }
            ConversionNode::Dict {
                key,
                value: value_node,
            } => self.convert_dict(py, *key, *value_node, value, validated, remaining_depth),
            ConversionNode::Literal { values } => convert_literal(py, values, value, validated),
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
            } => {
                let converted =
                    self.convert(py, *value_node, value, validated, remaining_depth - 1)?;
                let instance = allocate_model(py, model_type, &self.object_new)?;
                set_model_attribute(py, &instance, root_py_name, &converted)?;
                Ok(instance.unbind())
            }
        }
    }

    fn convert_list(
        &self,
        py: Python<'_>,
        item_node: usize,
        value: &Bound<'_, PyAny>,
        validated: bool,
        remaining_depth: u16,
    ) -> PyResult<Py<PyAny>> {
        let mut converted: Vec<Py<PyAny>>;
        if let Ok(items) = value.cast::<PyList>() {
            converted = Vec::with_capacity(items.len());
            for item in items {
                converted.push(self.convert(
                    py,
                    item_node,
                    &item,
                    validated,
                    remaining_depth - 1,
                )?);
            }
        } else if validated {
            if let Ok(items) = value.cast::<PyTuple>() {
                converted = Vec::with_capacity(items.len());
                for item in items {
                    converted.push(self.convert(
                        py,
                        item_node,
                        &item,
                        validated,
                        remaining_depth - 1,
                    )?);
                }
            } else {
                return Err(expected_type("list", value)?);
            }
        } else {
            return Err(expected_type("list", value)?);
        }
        Ok(PyList::new(py, converted)?.into_any().unbind())
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
        let output = PyDict::new(py);
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
            ConversionNode::Literal { values } => literal_index(py, values, value)?.is_some(),
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
        let mut converted_fields =
            Vec::with_capacity(fields.len() + usize::from(extra_value.is_some()));
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
            converted_fields.push((&field.py_name, converted));
        }

        let extra = if let Some(extra_node) = extra_value {
            let output = PyDict::new(py);
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

        let instance = allocate_model(py, model_type, &self.object_new)?;
        for (name, converted) in converted_fields {
            set_model_attribute(py, &instance, name, &converted)?;
        }
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
            ConversionNode::Scalar { kind, .. } => convert_jiter_scalar_value(py, *kind, value),
            ConversionNode::Literal { values } => {
                let python_value = value.into_pyobject(py)?.unbind();
                convert_literal(py, values, python_value.bind(py), false)
            }
            ConversionNode::List { item } => {
                let JiterJsonValue::Array(items) = value else {
                    return Err(PyErr::new::<PyTypeError, _>("expected list"));
                };
                let mut converted = Vec::with_capacity(items.len());
                for item_value in items.iter() {
                    converted.push(self.convert_jiter(py, *item, item_value)?);
                }
                Ok(PyList::new(py, converted)?.into_any().unbind())
            }
            ConversionNode::Dict {
                key,
                value: value_node,
            } => {
                let JiterJsonValue::Object(entries) = value else {
                    return Err(PyErr::new::<PyTypeError, _>("expected dict"));
                };
                let output = PyDict::new(py);
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

        let matching = branches
            .iter()
            .copied()
            .filter(|branch| self.jiter_node_matches_kind(*branch, value))
            .collect::<Vec<_>>();
        if matching.len() == 1 {
            return self.convert_jiter(py, matching[0], value);
        }
        let candidates = if matching.is_empty() {
            branches
        } else {
            matching.as_slice()
        };
        for branch in candidates {
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
        let mut converted_fields: Vec<Option<Py<PyAny>>> =
            std::iter::repeat_with(|| None).take(fields.len()).collect();
        let extra_output = extra_value.map(|_| PyDict::new(py));
        let mut seen = HashSet::with_capacity(entries.len());

        for (key, item) in entries.iter() {
            let key_string = key.as_ref();
            if !seen.insert(key_string) {
                return Err(duplicate_key(key));
            }
            if let Some(field_index) = fields_by_json_name.get(key_string) {
                let field = &fields[*field_index];
                converted_fields[*field_index] =
                    Some(self.convert_jiter(py, field.value_node, item)?);
            } else if let (Some(extra_node), Some(output)) = (extra_value, extra_output.as_ref()) {
                output.set_item(key_string, self.convert_jiter(py, extra_node, item)?)?;
            }
        }

        let extra = extra_output.map(|output| output.into_any().unbind());
        let instance = allocate_model(py, model_type, &self.object_new)?;
        for (index, field) in fields.iter().enumerate() {
            let converted = match converted_fields[index].take() {
                Some(converted) => converted,
                None => field.missing_factory.bind(py).call0()?.unbind(),
            };
            set_model_attribute(py, &instance, &field.py_name, &converted)?;
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

    fn serialize_node(
        &self,
        py: Python<'_>,
        node_id: usize,
        value: &Bound<'_, PyAny>,
        remaining_depth: u16,
    ) -> PyResult<JsonValue> {
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
                serializable_scalar_json_value(value)
            }
            ConversionNode::List { item } => {
                let items = value
                    .cast::<PyList>()
                    .map_err(|_| expected_type("list", value).unwrap())?;
                items
                    .iter()
                    .map(|item_value| {
                        self.serialize_node(py, *item, &item_value, remaining_depth - 1)
                    })
                    .collect::<PyResult<Vec<_>>>()
                    .map(JsonValue::Array)
            }
            ConversionNode::Dict {
                key: _,
                value: value_node,
            } => {
                let input = value
                    .cast::<PyDict>()
                    .map_err(|_| expected_type("dict", value).unwrap())?;
                let mut output = JsonMap::with_capacity(input.len());
                for (key, item) in input {
                    let key = key.extract::<String>().map_err(|_| {
                        PyErr::new::<PyTypeError, _>("JSON object keys must be strings")
                    })?;
                    output.insert(
                        key,
                        self.serialize_node(py, *value_node, &item, remaining_depth - 1)?,
                    );
                }
                Ok(JsonValue::Object(output))
            }
            ConversionNode::Union { branches, .. } => {
                let mut first_error = None;
                for branch in branches {
                    if self.node_matches_model_value(py, *branch, value)? {
                        match self.serialize_node(py, *branch, value, remaining_depth - 1) {
                            Ok(serialized) => return Ok(serialized),
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
                ..
            } => {
                if !value.is_instance(model_type.bind(py))? {
                    let expected = model_type.bind(py).name()?.to_str()?.to_owned();
                    return Err(expected_type(&expected, value)?);
                }
                let mut output = JsonMap::with_capacity(fields.len());
                for field in fields {
                    let field_value = value.getattr(field.py_name.bind(py))?;
                    if field
                        .missing_sentinel
                        .as_ref()
                        .is_some_and(|sentinel| field_value.is(sentinel.bind(py)))
                    {
                        continue;
                    }
                    output.insert(
                        field.json_name.clone(),
                        self.serialize_node(
                            py,
                            field.value_node,
                            &field_value,
                            remaining_depth - 1,
                        )?,
                    );
                }
                if let (Some(extra_node), Some(extra_name)) = (extra_value, extra_py_name) {
                    let extra = value.getattr(extra_name.bind(py))?;
                    let extra = extra.cast::<PyDict>().map_err(|_| {
                        PyErr::new::<PyTypeError, _>(
                            "generated additional properties must be a dict",
                        )
                    })?;
                    for (key, item) in extra {
                        let key = key.extract::<String>().map_err(|_| {
                            PyErr::new::<PyTypeError, _>("JSON object keys must be strings")
                        })?;
                        output.insert(
                            key,
                            self.serialize_node(py, *extra_node, &item, remaining_depth - 1)?,
                        );
                    }
                }
                Ok(JsonValue::Object(output))
            }
            ConversionNode::Root {
                model_type,
                value: value_node,
                root_py_name,
            } => {
                if !value.is_instance(model_type.bind(py))? {
                    let expected = model_type.bind(py).name()?.to_str()?.to_owned();
                    return Err(expected_type(&expected, value)?);
                }
                let root = value.getattr(root_py_name.bind(py))?;
                self.serialize_node(py, *value_node, &root, remaining_depth - 1)
            }
        }
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
            ConversionNode::Literal { values } => literal_index(py, values, value)?.is_some(),
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

fn serializable_scalar_json_value(value: &Bound<'_, PyAny>) -> PyResult<JsonValue> {
    if value.is_none() {
        return Ok(JsonValue::Null);
    }
    if value.is_instance_of::<PyBool>() {
        return Ok(JsonValue::Bool(value.extract()?));
    }
    if value.is_instance_of::<PyInt>() {
        if let Ok(number) = value.extract::<i64>() {
            return Ok(JsonValue::Number(JsonNumber::from(number)));
        }
        if let Ok(number) = value.extract::<u64>() {
            return Ok(JsonValue::Number(JsonNumber::from(number)));
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
        return super::parse_json(value.str()?.to_str()?);
    }
    if value.is_instance_of::<PyString>() {
        return Ok(JsonValue::String(value.extract()?));
    }
    super::py_to_serializable_json_value(value)
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
    })
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
            Ok(ConversionNode::Literal {
                values: values.iter().map(Bound::unbind).collect(),
            })
        }
        "union" => parse_union_node(descriptor),
        "model" => parse_model_node(py, descriptor),
        "root" => {
            let model_type = descriptor.get_item(1)?.cast_into::<PyType>()?.unbind();
            Ok(ConversionNode::Root {
                model_type,
                value: descriptor.get_item(2)?.extract()?,
                root_py_name: PyString::new(py, "root").unbind(),
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
    let model_type = descriptor.get_item(1)?.cast_into::<PyType>()?.unbind();
    let field_descriptors = descriptor.get_item(2)?.cast_into::<PyTuple>()?;
    let mut fields = Vec::with_capacity(field_descriptors.len());
    let mut fields_by_json_name = HashMap::with_capacity(field_descriptors.len());
    for field in field_descriptors.iter() {
        let field = field.cast_into::<PyTuple>()?;
        let json_name = field.get_item(0)?.extract::<String>()?;
        fields_by_json_name.insert(json_name.clone(), fields.len());
        fields.push(FieldPlan {
            json_name,
            py_name: PyString::new(py, field.get_item(1)?.extract::<String>()?.as_str()).unbind(),
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
    Ok(ConversionNode::Model {
        model_type,
        fields,
        fields_by_json_name,
        extra_value,
        extra_py_name: extra_value.map(|_| PyString::new(py, "__jsoncompat_extra__").unbind()),
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
