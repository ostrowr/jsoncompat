//! Python bindings for the `jsoncompat` compatibility checker and value generator.
//!
//! The extension module exposes `check_compat`, reusable validators and
//! generators, and a `Role` constants module. Public functions accept JSON
//! strings and report invalid inputs or hard unsupported core-library cases as
//! `ValueError`.

mod model_converter;

use std::collections::HashSet;
use std::rc::Rc;

use jiter::{JsonValue as JiterJsonValue, PythonParse, StringCacheMode, map_json_error};
use pyo3::exceptions::{PyRuntimeError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::pyclass::{PyTraverseError, PyVisit};
use pyo3::sync::PyOnceLock;
use pyo3::types::{
    PyAny, PyBool, PyBytes, PyDict, PyFloat, PyInt, PyList, PyMapping, PyString, PyTuple, PyType,
};

use ::jsoncompat::{Role, SchemaDocument, check_compat, validate_compatibility_input};
use json_schema_fuzz::{GenerateError, GenerationConfig, ValueGenerator};
use jsonschema::InstanceRef as JSONInstanceRef;

use serde_json::{Map as JsonMap, Number as JsonNumber, Value as JsonValue};

use model_converter::{
    CandidateConstruction, MaterializedJsonValue, ModelConverterPlan, ModelConverterPy,
    RootedModelConverterPlan, compile_model_converter_plan, root_model_converter_plan,
};

fn validated_schema(raw: &JsonValue) -> Result<SchemaDocument, String> {
    let schema = SchemaDocument::from_json(raw).map_err(|error| error.to_string())?;
    schema.root().map_err(|error| error.to_string())?;
    schema
        .validate_source_schema()
        .map_err(|error| error.to_string())?;
    Ok(schema)
}

fn compatibility_schema(raw: &JsonValue) -> Result<SchemaDocument, String> {
    let schema = SchemaDocument::from_json(raw).map_err(|error| error.to_string())?;
    validate_compatibility_input(&schema).map_err(|error| error.to_string())?;
    Ok(schema)
}

fn validate_value_for_schema(schema: &SchemaDocument, instance: &JsonValue) -> PyResult<bool> {
    schema
        .is_valid(instance)
        .map_err(|e| PyErr::new::<PyValueError, _>(format!("Validation failed: {e}")))
}

fn py_to_json_value(value: &Bound<'_, PyAny>) -> PyResult<JsonValue> {
    py_to_json_value_inner(value, &mut HashSet::new())
}

fn py_to_serializable_json_value(value: &Bound<'_, PyAny>) -> PyResult<JsonValue> {
    py_to_json_value_inner(value, &mut HashSet::new())
}

fn py_to_json_value_inner(
    value: &Bound<'_, PyAny>,
    active_containers: &mut HashSet<usize>,
) -> PyResult<JsonValue> {
    if value.is_none() {
        return Ok(JsonValue::Null);
    }
    if value.is_instance_of::<PyBool>() {
        return Ok(JsonValue::Bool(value.extract::<bool>()?));
    }
    if value.is_instance_of::<PyInt>() {
        return py_int_to_json_value(value);
    }
    if value.is_instance_of::<PyFloat>() {
        let number = value.extract::<f64>()?;
        if !number.is_finite() {
            return Err(PyErr::new::<PyValueError, _>("JSON numbers must be finite"));
        }
        return Ok(JsonValue::Number(
            JsonNumber::from_f64(number).expect("finite f64 values are JSON numbers"),
        ));
    }
    if value.is_instance_of::<PyString>() {
        return Ok(JsonValue::String(value.extract::<String>()?));
    }
    if let Ok(list) = value.cast::<PyList>() {
        let container_id = enter_python_container(value, active_containers)?;
        let result = list
            .iter()
            .map(|item| py_to_json_value_inner(&item, active_containers))
            .collect::<PyResult<Vec<_>>>()
            .map(JsonValue::Array);
        active_containers.remove(&container_id);
        return result;
    }
    if let Ok(dict) = value.cast::<PyDict>() {
        let container_id = enter_python_container(value, active_containers)?;
        let mut object = JsonMap::with_capacity(dict.len());
        for (key, item) in dict {
            if !key.is_instance_of::<PyString>() {
                active_containers.remove(&container_id);
                return Err(PyErr::new::<PyTypeError, _>(
                    "JSON object keys must be strings",
                ));
            }
            let key = match key.extract::<String>() {
                Ok(key) => key,
                Err(error) => {
                    active_containers.remove(&container_id);
                    return Err(error);
                }
            };
            let item = match py_to_json_value_inner(&item, active_containers) {
                Ok(item) => item,
                Err(error) => {
                    active_containers.remove(&container_id);
                    return Err(error);
                }
            };
            object.insert(key, item);
        }
        active_containers.remove(&container_id);
        return Ok(JsonValue::Object(object));
    }
    if let Ok(mapping) = value.cast::<PyMapping>() {
        let container_id = enter_python_container(value, active_containers)?;
        let mut object = JsonMap::with_capacity(mapping.len()?);
        for entry in mapping.items()? {
            let pair = entry.cast::<PyTuple>().map_err(|_| {
                PyErr::new::<PyTypeError, _>("mapping items must be key-value pairs")
            })?;
            if pair.len() != 2 {
                active_containers.remove(&container_id);
                return Err(PyErr::new::<PyTypeError, _>(
                    "mapping items must be key-value pairs",
                ));
            }
            let key = pair.get_item(0)?;
            let item = pair.get_item(1)?;
            if !key.is_instance_of::<PyString>() {
                active_containers.remove(&container_id);
                return Err(PyErr::new::<PyTypeError, _>(
                    "JSON object keys must be strings",
                ));
            }
            let key = match key.extract::<String>() {
                Ok(key) => key,
                Err(error) => {
                    active_containers.remove(&container_id);
                    return Err(error);
                }
            };
            let item = match py_to_json_value_inner(&item, active_containers) {
                Ok(item) => item,
                Err(error) => {
                    active_containers.remove(&container_id);
                    return Err(error);
                }
            };
            object.insert(key, item);
        }
        active_containers.remove(&container_id);
        return Ok(JsonValue::Object(object));
    }
    if let Ok(tuple) = value.cast::<PyTuple>() {
        let container_id = enter_python_container(value, active_containers)?;
        let result = tuple
            .iter()
            .map(|item| py_to_json_value_inner(&item, active_containers))
            .collect::<PyResult<Vec<_>>>()
            .map(JsonValue::Array);
        active_containers.remove(&container_id);
        return result;
    }

    Err(PyErr::new::<PyTypeError, _>(format!(
        "expected a JSON-compatible value, got {}",
        value.get_type().name()?
    )))
}

fn enter_python_container(
    value: &Bound<'_, PyAny>,
    active_containers: &mut HashSet<usize>,
) -> PyResult<usize> {
    let container_id = value.as_ptr() as usize;
    if !active_containers.insert(container_id) {
        return Err(PyErr::new::<PyValueError, _>(
            "cyclic containers are not JSON values",
        ));
    }
    Ok(container_id)
}

fn ensure_finite_python_json_numbers(value: &Bound<'_, PyAny>) -> PyResult<()> {
    if value.is_instance_of::<PyFloat>() {
        if !value.extract::<f64>()?.is_finite() {
            return Err(PyErr::new::<PyValueError, _>("JSON numbers must be finite"));
        }
        return Ok(());
    }
    if let Ok(list) = value.cast::<PyList>() {
        for item in list {
            ensure_finite_python_json_numbers(&item)?;
        }
        return Ok(());
    }
    if let Ok(dict) = value.cast::<PyDict>() {
        for (_, item) in dict {
            ensure_finite_python_json_numbers(&item)?;
        }
        return Ok(());
    }
    if let Ok(mapping) = value.cast::<PyMapping>() {
        for entry in mapping.items()? {
            let pair = entry.cast::<PyTuple>().map_err(|_| {
                PyErr::new::<PyTypeError, _>("mapping items must be key-value pairs")
            })?;
            if pair.len() != 2 {
                return Err(PyErr::new::<PyTypeError, _>(
                    "mapping items must be key-value pairs",
                ));
            }
            ensure_finite_python_json_numbers(&pair.get_item(1)?)?;
        }
        return Ok(());
    }
    if let Ok(tuple) = value.cast::<PyTuple>() {
        for item in tuple {
            ensure_finite_python_json_numbers(&item)?;
        }
    }
    Ok(())
}

fn py_int_to_json_value(value: &Bound<'_, PyAny>) -> PyResult<JsonValue> {
    if let Ok(number) = value.extract::<i64>() {
        return Ok(JsonValue::Number(JsonNumber::from(number)));
    }
    if let Ok(number) = value.extract::<u64>() {
        return Ok(JsonValue::Number(JsonNumber::from(number)));
    }
    let rendered = value
        .py()
        .get_type::<PyInt>()
        .getattr("__repr__")?
        .call1((value,))?
        .extract::<String>()?;
    parse_json(&rendered)
}

#[pyclass(name = "Validator", module = "jsoncompat._native", unsendable)]
struct ValidatorPy {
    schema: Rc<SchemaDocument>,
}

#[pyclass(name = "JsoncompatMissingType", module = "jsoncompat._native", frozen)]
struct JsoncompatMissingPy;

static JSONCOMPAT_MISSING_SINGLETON: PyOnceLock<Py<JsoncompatMissingPy>> = PyOnceLock::new();

fn typed_jsoncompat_missing(py: Python<'_>) -> PyResult<Py<JsoncompatMissingPy>> {
    JSONCOMPAT_MISSING_SINGLETON
        .get_or_try_init(py, || Py::new(py, JsoncompatMissingPy))
        .map(|missing| missing.clone_ref(py))
}

fn jsoncompat_missing(py: Python<'_>) -> PyResult<Py<PyAny>> {
    typed_jsoncompat_missing(py).map(Py::into_any)
}

#[pyclass(name = "_ModelPlan", module = "jsoncompat._native", unsendable)]
struct ModelPlanPy {
    plan: Option<Rc<ModelConverterPlan>>,
}

enum ModelRuntimeState {
    Live {
        plan_owner: Py<ModelPlanPy>,
        rooted_plan: RootedModelConverterPlan,
    },
    Cleared,
}

#[pyclass(name = "ModelRuntime", module = "jsoncompat._native", unsendable)]
struct ModelRuntimePy {
    state: ModelRuntimeState,
}

#[pyclass(name = "Generator", module = "jsoncompat._native", unsendable)]
struct GeneratorPy {
    schema: SchemaDocument,
}

#[pymethods]
impl JsoncompatMissingPy {
    #[new]
    fn new(py: Python<'_>) -> PyResult<Py<Self>> {
        typed_jsoncompat_missing(py)
    }

    fn __repr__(&self) -> &'static str {
        "JSONCOMPAT_MISSING"
    }

    fn __copy__(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        jsoncompat_missing(py)
    }

    fn __deepcopy__(&self, py: Python<'_>, _memo: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        jsoncompat_missing(py)
    }

    fn __reduce__(&self) -> &'static str {
        "JSONCOMPAT_MISSING"
    }
}

#[pymethods]
impl ValidatorPy {
    /// Check whether a JSON string satisfies this validator's schema.
    ///
    /// Parameters
    /// ----------
    /// instance_json : str
    ///     JSON string of the candidate value.
    ///
    /// Returns
    /// -------
    /// bool
    ///     `True` if the value satisfies the schema, `False` otherwise.
    fn is_valid_json(&self, instance_json: &str) -> PyResult<bool> {
        let instance = parse_json(instance_json)?;
        self.validate_instance_assuming_json(JSONInstanceRef::from_serde(&instance))
    }

    /// Check whether a Python JSON-compatible value satisfies this validator's schema.
    fn is_valid_value(&self, instance: &Bound<'_, PyAny>) -> PyResult<bool> {
        let instance = py_to_json_value(instance)?;
        self.validate_instance_assuming_json(JSONInstanceRef::from_serde(&instance))
    }

    /// Check a Python JSON value in place without allocating a serde value tree.
    fn _is_valid_borrowed_value(&self, instance: &Bound<'_, PyAny>) -> PyResult<bool> {
        self.validate_instance(JSONInstanceRef::from_python(instance))
    }

    /// Parse JSON once, validate it, and return the parsed Python value.
    fn parse_json(
        &self,
        py: Python<'_>,
        payload: &Bound<'_, PyAny>,
    ) -> PyResult<(bool, Py<PyAny>)> {
        parse_and_validate_json_to_python(&self.schema, py, payload)
    }

    /// Validate and serialize a Python JSON-compatible value in one traversal.
    fn serialize_json(&self, instance: &Bound<'_, PyAny>) -> PyResult<Option<String>> {
        let instance = py_to_serializable_json_value(instance)?;
        if self.validate_instance_assuming_json(JSONInstanceRef::from_serde(&instance))? {
            serialize_json_value(&instance).map(Some)
        } else {
            Ok(None)
        }
    }
}

impl ValidatorPy {
    fn validate_instance(&self, instance: JSONInstanceRef<'_>) -> PyResult<bool> {
        let result = self.schema.is_valid_instance(instance);
        result.map_err(validation_error)
    }

    fn validate_instance_assuming_json(&self, instance: JSONInstanceRef<'_>) -> PyResult<bool> {
        let result = self.schema.is_valid_instance_assuming_json(instance);
        result.map_err(validation_error)
    }
}

#[pymethods]
impl ModelPlanPy {
    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        if let Some(plan) = &self.plan {
            plan.traverse(&visit)?;
        }
        Ok(())
    }

    fn __clear__(&mut self) {
        self.plan = None;
    }
}

#[pymethods]
impl ModelRuntimePy {
    /// Construct a generated model from its keyword-only dataclass interface.
    #[pyo3(signature = (kwargs, *, skip_validation=false))]
    fn construct_kwargs(
        &self,
        py: Python<'_>,
        kwargs: &Bound<'_, PyDict>,
        skip_validation: bool,
    ) -> PyResult<Py<PyAny>> {
        let converter = self.converter(py)?;
        let candidate = converter.construct_kwargs_candidate(py, kwargs)?;
        let converted = if skip_validation {
            Some(candidate.finish_unchecked())
        } else {
            candidate.validate(py)?
        };
        Self::require_valid(&converter, py, converted)
    }

    /// Construct a generated model from an already-decoded JSON value.
    #[allow(clippy::wrong_self_convention)]
    #[pyo3(signature = (value, *, skip_validation=false))]
    fn from_value(
        &self,
        py: Python<'_>,
        value: &Bound<'_, PyAny>,
        skip_validation: bool,
    ) -> PyResult<Py<PyAny>> {
        let converter = self.converter(py)?;
        let converted = if skip_validation {
            converter.construct_unchecked(py, value).map(Some)
        } else {
            construct_model_value(py, value, &converter)
        }?;
        Self::require_valid(&converter, py, converted)
    }

    /// Parse JSON and construct a generated model in one native pass.
    #[pyo3(signature = (payload, *, skip_validation=false))]
    fn deserialize(
        &self,
        py: Python<'_>,
        payload: &Bound<'_, PyAny>,
        skip_validation: bool,
    ) -> PyResult<Py<PyAny>> {
        let converter = self.converter(py)?;
        let converted = if let Ok(text) = payload.cast::<PyString>() {
            if skip_validation {
                construct_model_json_bytes_unchecked(py, text.to_str()?.as_bytes(), &converter)?
            } else {
                construct_model_json_bytes_checked(py, text.to_str()?.as_bytes(), &converter)?
            }
        } else if let Ok(bytes) = payload.cast::<PyBytes>() {
            if skip_validation {
                construct_model_json_bytes_unchecked(py, bytes.as_bytes(), &converter)?
            } else {
                construct_model_json_bytes_checked(py, bytes.as_bytes(), &converter)?
            }
        } else {
            return Err(PyErr::new::<PyTypeError, _>(
                "JSON payloads must be str or bytes",
            ));
        };
        Self::require_valid(&converter, py, converted)
    }

    /// Materialize a generated model as a mutable Python JSON value.
    #[pyo3(signature = (instance, *, skip_validation=false))]
    fn to_value(
        &self,
        py: Python<'_>,
        instance: &Bound<'_, PyAny>,
        skip_validation: bool,
    ) -> PyResult<Py<PyAny>> {
        let converter = self.converter(py)?;
        Self::ensure_model_instance(&converter, py, instance)?;
        let value = model_to_value(py, instance, &converter)?;
        if !skip_validation && !converter.validate_json_value(py, &value)? {
            return Self::require_valid(&converter, py, None);
        }
        Ok(value.into_py())
    }

    /// Serialize a generated model directly from its immutable slots.
    #[pyo3(signature = (instance, *, skip_validation=false))]
    fn serialize(
        &self,
        py: Python<'_>,
        instance: &Bound<'_, PyAny>,
        skip_validation: bool,
    ) -> PyResult<String> {
        let converter = self.converter(py)?;
        Self::ensure_model_instance(&converter, py, instance)?;
        if skip_validation {
            return converter.serialize_model_trusted(py, instance);
        }
        let value = model_to_value(py, instance, &converter)?;
        if !converter.validate_json_value(py, &value)? {
            return Self::require_valid(&converter, py, None);
        }
        serialize_model(py, &value, &converter)
    }

    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        match &self.state {
            ModelRuntimeState::Live { plan_owner, .. } => visit.call(plan_owner),
            ModelRuntimeState::Cleared => Ok(()),
        }
    }

    fn __clear__(&mut self) {
        self.state = ModelRuntimeState::Cleared;
    }
}

impl ModelRuntimePy {
    fn converter(&self, _py: Python<'_>) -> PyResult<ModelConverterPy> {
        let ModelRuntimeState::Live { rooted_plan, .. } = &self.state else {
            return Err(PyErr::new::<PyRuntimeError, _>(
                "generated model runtime has been cleared",
            ));
        };
        rooted_plan.upgrade().ok_or_else(|| {
            PyErr::new::<PyRuntimeError, _>("generated model runtime has been cleared")
        })
    }

    fn ensure_model_instance(
        converter: &ModelConverterPy,
        py: Python<'_>,
        instance: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        let model_type = converter.model_type()?;
        if instance.get_type().is(model_type.bind(py)) {
            Ok(())
        } else {
            Err(PyErr::new::<PyTypeError, _>(format!(
                "expected {}, got {}",
                model_type.bind(py).name()?,
                instance.get_type().name()?,
            )))
        }
    }

    fn require_valid<T>(
        converter: &ModelConverterPy,
        py: Python<'_>,
        value: Option<T>,
    ) -> PyResult<T> {
        value.ok_or_else(|| {
            let model_name = converter
                .model_type()
                .and_then(|model_type| model_type.bind(py).name())
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|_| "generated model".to_owned());
            PyErr::new::<PyValueError, _>(format!("value does not satisfy {model_name} schema"))
        })
    }
}

fn construct_model_value(
    py: Python<'_>,
    instance: &Bound<'_, PyAny>,
    converter: &ModelConverterPy,
) -> PyResult<Option<Py<PyAny>>> {
    // Conversion necessarily traverses the complete input graph to reject
    // cycles and values outside the JSON data model. Once that succeeds for
    // ordinary dict/list input, schema validation can borrow the original
    // graph without repeating the shape-only traversal.
    let candidate = converter.construct_candidate(py, instance)?;
    let candidate = match candidate {
        CandidateConstruction::Constructed(candidate) => candidate,
        CandidateConstruction::Mismatch(conversion_error) => {
            let raw_is_valid = converter.is_valid_raw_python(instance)?;
            if raw_is_valid {
                return Err(PyErr::new::<PyRuntimeError, _>(format!(
                    "schema-valid value cannot be represented by its generated model: {conversion_error}"
                )));
            }
            return Ok(None);
        }
    };
    candidate.validate(py, instance)
}

fn model_to_value(
    py: Python<'_>,
    instance: &Bound<'_, PyAny>,
    converter: &ModelConverterPy,
) -> PyResult<MaterializedJsonValue> {
    converter.materialize_json_value(py, instance)
}

fn serialize_model(
    py: Python<'_>,
    value: &MaterializedJsonValue,
    converter: &ModelConverterPy,
) -> PyResult<String> {
    converter.serialize_json_value(py, value)
}

fn construct_model_json_bytes_unchecked(
    py: Python<'_>,
    payload: &[u8],
    converter: &ModelConverterPy,
) -> PyResult<Option<Py<PyAny>>> {
    let parsed =
        JiterJsonValue::parse(payload, false).map_err(|error| map_json_error(payload, &error))?;
    converter.construct_jiter_unchecked(py, &parsed).map(Some)
}

fn construct_model_json_bytes_checked(
    py: Python<'_>,
    payload: &[u8],
    converter: &ModelConverterPy,
) -> PyResult<Option<Py<PyAny>>> {
    let parsed =
        JiterJsonValue::parse(payload, false).map_err(|error| map_json_error(payload, &error))?;
    // Jiter has already enforced JSON scalar syntax and finite numbers; the
    // model converter that immediately follows rejects duplicate keys at every
    // object node. Avoid repeating those shape checks here.
    converter.construct_jiter_checked(py, &parsed)
}

fn validation_error(error: impl std::fmt::Display) -> PyErr {
    PyErr::new::<PyValueError, _>(format!("Validation failed: {error}"))
}

#[pymethods]
impl GeneratorPy {
    /// Generate a JSON value intended to satisfy this generator's schema.
    ///
    /// Parameters
    /// ----------
    /// depth : int, optional
    ///     Recursion depth limit (default 5).
    ///
    /// Returns
    /// -------
    /// str
    ///     A JSON string representing a randomly generated value that should satisfy the schema.
    #[pyo3(signature = (depth=5))]
    fn generate_value(&self, depth: u8) -> PyResult<String> {
        generate_value_for_schema(&self.schema, depth)
    }
}

/// Parse a JSON string into a serde_json::Value, converting any error into a Python ValueError.
fn parse_json(s: &str) -> PyResult<JsonValue> {
    serde_json::from_str(s).map_err(|e| PyErr::new::<PyValueError, _>(format!("Invalid JSON: {e}")))
}

fn parse_json_to_python<'py>(
    py: Python<'py>,
    payload: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    if let Ok(text) = payload.cast::<PyString>() {
        return parse_json_bytes_to_python(py, text.to_str()?.as_bytes());
    }
    if let Ok(bytes) = payload.cast::<PyBytes>() {
        return parse_json_bytes_to_python(py, bytes.as_bytes());
    }
    Err(PyErr::new::<PyTypeError, _>(
        "JSON payloads must be str or bytes",
    ))
}

fn parse_json_bytes_to_python<'py>(py: Python<'py>, payload: &[u8]) -> PyResult<Bound<'py, PyAny>> {
    let parser = PythonParse {
        allow_inf_nan: false,
        cache_mode: StringCacheMode::Keys,
        catch_duplicate_keys: true,
        ..PythonParse::default()
    };
    parser
        .python_parse(py, payload)
        .map_err(|error| map_json_error(payload, &error))
}

fn parse_and_validate_json_to_python(
    schema: &SchemaDocument,
    py: Python<'_>,
    payload: &Bound<'_, PyAny>,
) -> PyResult<(bool, Py<PyAny>)> {
    if let Ok(text) = payload.cast::<PyString>() {
        return parse_and_validate_json_bytes(schema, py, text.to_str()?.as_bytes());
    }
    if let Ok(bytes) = payload.cast::<PyBytes>() {
        return parse_and_validate_json_bytes(schema, py, bytes.as_bytes());
    }
    Err(PyErr::new::<PyTypeError, _>(
        "JSON payloads must be str or bytes",
    ))
}

fn parse_and_validate_json_bytes(
    schema: &SchemaDocument,
    py: Python<'_>,
    payload: &[u8],
) -> PyResult<(bool, Py<PyAny>)> {
    let parsed =
        JiterJsonValue::parse(payload, false).map_err(|error| map_json_error(payload, &error))?;
    let instance = jiter_to_serde_json(&parsed)?;
    let is_valid = validate_value_for_schema(schema, &instance)?;
    Ok((is_valid, parsed.into_pyobject(py)?.unbind()))
}

fn jiter_to_serde_json(value: &JiterJsonValue<'_>) -> PyResult<JsonValue> {
    match value {
        JiterJsonValue::Null => Ok(JsonValue::Null),
        JiterJsonValue::Bool(value) => Ok(JsonValue::Bool(*value)),
        JiterJsonValue::Int(value) => Ok(JsonValue::Number(JsonNumber::from(*value))),
        JiterJsonValue::BigInt(value) => parse_json(&value.to_string()),
        JiterJsonValue::Float(value) => JsonNumber::from_f64(*value)
            .map(JsonValue::Number)
            .ok_or_else(|| PyErr::new::<PyValueError, _>("JSON numbers must be finite")),
        JiterJsonValue::Str(value) => Ok(JsonValue::String(value.to_string())),
        JiterJsonValue::Array(values) => values
            .iter()
            .map(jiter_to_serde_json)
            .collect::<PyResult<Vec<_>>>()
            .map(JsonValue::Array),
        JiterJsonValue::Object(entries) => {
            let mut object = JsonMap::with_capacity(entries.len());
            for (key, value) in entries.iter() {
                if object
                    .insert(key.to_string(), jiter_to_serde_json(value)?)
                    .is_some()
                {
                    return Err(PyErr::new::<PyValueError, _>(format!(
                        "duplicate key: `{key}`"
                    )));
                }
            }
            Ok(JsonValue::Object(object))
        }
    }
}

fn serialize_json_value(value: &JsonValue) -> PyResult<String> {
    serde_json::to_string(value).map_err(|error| {
        PyErr::new::<PyValueError, _>(format!("JSON serialization failed: {error}"))
    })
}

fn parse_schema(schema_json: &str) -> PyResult<SchemaDocument> {
    let raw = parse_json(schema_json)?;
    validated_schema(&raw)
        .map_err(|e| PyErr::new::<PyValueError, _>(format!("Invalid schema: {e}")))
}

fn validator_for_schema(schema_json: &str) -> PyResult<ValidatorPy> {
    let schema = Rc::new(parse_schema(schema_json)?);
    Ok(ValidatorPy { schema })
}

fn generator_for_schema(schema_json: &str) -> PyResult<GeneratorPy> {
    let schema = parse_schema(schema_json)?;
    Ok(GeneratorPy { schema })
}

fn generate_value_for_schema(schema: &SchemaDocument, depth: u8) -> PyResult<String> {
    let mut rng = rand::rng();
    let value = ValueGenerator::generate(schema, GenerationConfig::new(depth), &mut rng).map_err(
        |error| match error {
            GenerateError::Schema(error) => {
                PyErr::new::<PyValueError, _>(format!("Invalid schema: {error}"))
            }
            GenerateError::Unsatisfiable => PyErr::new::<PyValueError, _>(error.to_string()),
            GenerateError::ExhaustedAttempts { .. } => {
                PyErr::new::<PyValueError, _>(error.to_string())
            }
            _ => PyErr::new::<PyValueError, _>(error.to_string()),
        },
    )?;

    serde_json::to_string(&value).map_err(|e| {
        PyErr::new::<PyValueError, _>(format!("Failed to serialize generated value: {e}"))
    })
}

/// Map a string into the Rust role enum, raising ValueError on unknown input.
fn parse_role(role: &str) -> PyResult<Role> {
    match role.to_ascii_lowercase().as_str() {
        "serializer" => Ok(Role::Serializer),
        "deserializer" => Ok(Role::Deserializer),
        "both" => Ok(Role::Both),
        _ => Err(PyErr::new::<PyValueError, _>(
            "role must be one of 'serializer', 'deserializer', or 'both'",
        )),
    }
}

/// Check whether `new_schema_json` is compatible with `old_schema_json` under the given role.
///
/// Parameters
/// ----------
/// old_schema_json : str
///     JSON string representing the original schema.
/// new_schema_json : str
///     JSON string representing the updated schema.
/// role : str, optional
///     One of "serializer", "deserializer" or "both" (default).
///
/// Returns
/// -------
/// bool
///     `True` if the change is considered compatible, `False` otherwise.
#[pyfunction]
#[pyo3(signature = (old_schema_json, new_schema_json, role="both"), name = "check_compat")]
fn check_compat_py(old_schema_json: &str, new_schema_json: &str, role: &str) -> PyResult<bool> {
    let role_e = parse_role(role)?;

    let old_raw = parse_json(old_schema_json)?;
    let new_raw = parse_json(new_schema_json)?;

    let old_schema = compatibility_schema(&old_raw)
        .map_err(|e| PyErr::new::<PyValueError, _>(format!("Invalid old schema: {e}")))?;
    let new_schema = compatibility_schema(&new_raw)
        .map_err(|e| PyErr::new::<PyValueError, _>(format!("Invalid new schema: {e}")))?;

    check_compat(&old_schema, &new_schema, role_e)
        .map_err(|e| PyErr::new::<PyValueError, _>(format!("Compatibility check failed: {e}")))
}

/// Generate a JSON value intended to satisfy the provided schema.
///
/// Parameters
/// ----------
/// schema_json : str
///     JSON string of the schema the output should conform to.
/// depth : int, optional
///     Recursion depth limit (default 5).
///
/// Returns
/// -------
/// str
///     A JSON string representing a randomly generated value that should satisfy the schema.
#[pyfunction]
#[pyo3(signature = (schema_json, depth=5), name = "generate_value")]
fn generate_value_py(schema_json: &str, depth: u8) -> PyResult<String> {
    generator_for_schema(schema_json)?.generate_value(depth)
}

/// Build a reusable generator for a JSON Schema.
///
/// Parameters
/// ----------
/// schema_json : str
///     JSON string of the schema to generate values for.
///
/// Returns
/// -------
/// Generator
///     A reusable generator that parses the schema once.
#[pyfunction]
#[pyo3(signature = (schema_json), name = "generator_for")]
fn generator_for_py(schema_json: &str) -> PyResult<GeneratorPy> {
    generator_for_schema(schema_json)
}

/// Build a reusable validator for a JSON Schema.
///
/// Parameters
/// ----------
/// schema_json : str
///     JSON string of the schema to validate against.
///
/// Returns
/// -------
/// Validator
///     A reusable validator that parses the schema once.
#[pyfunction]
#[pyo3(signature = (schema_json), name = "validator_for")]
fn validator_for_py(schema_json: &str) -> PyResult<ValidatorPy> {
    validator_for_schema(schema_json)
}

/// Compile one shared conversion plan and a rooted runtime for every generated dataclass.
#[pyfunction]
#[pyo3(
    signature = (model_roots, descriptors, frozen_list_type, frozen_dict_type),
    name = "compile_model_runtimes"
)]
fn compile_model_runtimes_py(
    py: Python<'_>,
    model_roots: &Bound<'_, PyList>,
    descriptors: &Bound<'_, PyList>,
    frozen_list_type: &Bound<'_, PyType>,
    frozen_dict_type: &Bound<'_, PyType>,
) -> PyResult<Vec<Py<ModelRuntimePy>>> {
    let missing_sentinel = jsoncompat_missing(py)?;
    let plan = compile_model_converter_plan(
        py,
        descriptors,
        frozen_list_type,
        frozen_dict_type,
        missing_sentinel,
    )?;
    let plan_owner = Py::new(
        py,
        ModelPlanPy {
            plan: Some(Rc::clone(&plan)),
        },
    )?;
    let mut runtimes = Vec::with_capacity(model_roots.len());
    for model_root in model_roots {
        let model_root = model_root.cast_into::<PyTuple>().map_err(|_| {
            PyErr::new::<PyTypeError, _>("model roots must be (model type, root node) pairs")
        })?;
        if model_root.len() != 2 {
            return Err(PyErr::new::<PyTypeError, _>(
                "model roots must be (model type, root node) pairs",
            ));
        }
        let model_type = model_root.get_item(0)?.cast_into::<PyType>()?;
        let root = model_root.get_item(1)?.extract::<usize>()?;
        let rooted_plan = root_model_converter_plan(py, &plan, &model_type, root)?;
        runtimes.push(Py::new(
            py,
            ModelRuntimePy {
                state: ModelRuntimeState::Live {
                    plan_owner: plan_owner.clone_ref(py),
                    rooted_plan,
                },
            },
        )?);
    }
    Ok(runtimes)
}

/// Parse a JSON string or byte sequence directly into Python JSON values.
#[pyfunction]
#[pyo3(signature = (payload), name = "deserialize_json")]
fn deserialize_json_py(py: Python<'_>, payload: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    let parsed = parse_json_to_python(py, payload)?;
    ensure_finite_python_json_numbers(&parsed)?;
    Ok(parsed.unbind())
}

/// Serialize a Python JSON-compatible value using the native JSON encoder.
#[pyfunction]
#[pyo3(signature = (value), name = "serialize_json")]
fn serialize_json_py(value: &Bound<'_, PyAny>) -> PyResult<String> {
    serialize_json_value(&py_to_serializable_json_value(value)?)
}

/// Check whether a JSON value satisfies a schema.
///
/// Parameters
/// ----------
/// schema_json : str
///     JSON string of the schema to validate against.
/// instance_json : str
///     JSON string of the candidate value.
///
/// Returns
/// -------
/// bool
///     `True` if the value satisfies the schema, `False` otherwise.
#[pyfunction]
#[pyo3(signature = (schema_json, instance_json), name = "is_valid")]
fn is_valid_py(schema_json: &str, instance_json: &str) -> PyResult<bool> {
    validator_for_schema(schema_json)?.is_valid_json(instance_json)
}

/// Python module definition
#[pymodule]
#[pyo3(name = "_native")]
fn jsoncompat_native(py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(check_compat_py, m)?)?;
    m.add_function(wrap_pyfunction!(generate_value_py, m)?)?;
    m.add_function(wrap_pyfunction!(generator_for_py, m)?)?;
    m.add_function(wrap_pyfunction!(validator_for_py, m)?)?;
    m.add_function(wrap_pyfunction!(compile_model_runtimes_py, m)?)?;
    m.add_function(wrap_pyfunction!(deserialize_json_py, m)?)?;
    m.add_function(wrap_pyfunction!(serialize_json_py, m)?)?;
    m.add_function(wrap_pyfunction!(is_valid_py, m)?)?;
    m.add_class::<GeneratorPy>()?;
    m.add_class::<JsoncompatMissingPy>()?;
    m.add_class::<ModelRuntimePy>()?;
    m.add_class::<ValidatorPy>()?;
    m.add("JSONCOMPAT_MISSING", jsoncompat_missing(py)?)?;

    let role_constants = PyModule::new(py, "Role")?;
    role_constants.add("SERIALIZER", "serializer")?;
    role_constants.add("DESERIALIZER", "deserializer")?;
    role_constants.add("BOTH", "both")?;
    m.add_submodule(&role_constants)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{compatibility_schema, validated_schema};
    use serde_json::json;

    #[test]
    fn compatibility_schema_validation_accepts_unmodeled_keywords_for_modeled_comparison() {
        compatibility_schema(&json!({
            "type": "object",
            "dependentSchemas": {
                "kind": { "required": ["detail"] }
            }
        }))
        .expect("compatibility bindings should accept warning-only schema keywords");
    }

    #[test]
    fn generation_schema_validation_rejects_backend_invalid_ref_bearing_schemas_up_front() {
        let error = validated_schema(&json!({
            "$defs": {
                "Value": { "type": "string" }
            },
            "$ref": "#/$defs/Value",
            "deprecated": "eventually"
        }))
        .expect_err("generation bindings must validate raw ref-bearing schemas before work");

        assert!(
            error.contains("schema failed Draft 2020-12 validator compilation"),
            "unexpected error: {error}"
        );
    }
}
