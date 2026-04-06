//! Python bindings for the `jsoncompat` compatibility checker and value generator.
//!
//! The extension module exposes `check_compat`, reusable validators and
//! generators, and a `Role` constants module. Public functions accept JSON
//! schemas as strings and report invalid inputs or unsupported core-library
//! cases as `ValueError`.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use ::jsoncompat::{Role, SchemaDocument, check_compat};
use json_schema_fuzz::{GenerateError, GenerationConfig, ValueGenerator};

use serde_json::Value as JsonValue;

#[pyclass(name = "Validator", module = "jsoncompat._native", unsendable)]
struct ValidatorPy {
    schema: SchemaDocument,
}

#[pyclass(name = "Generator", module = "jsoncompat._native", unsendable)]
struct GeneratorPy {
    schema: SchemaDocument,
}

#[pymethods]
impl ValidatorPy {
    /// Check whether a JSON value satisfies this validator's schema.
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
    fn is_valid(&self, instance_json: &str) -> PyResult<bool> {
        let instance = parse_json(instance_json)?;
        self.schema
            .is_valid(&instance)
            .map_err(|e| PyErr::new::<PyValueError, _>(format!("Validation failed: {e}")))
    }
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

fn parse_schema(schema_json: &str) -> PyResult<SchemaDocument> {
    let raw = parse_json(schema_json)?;
    SchemaDocument::from_json(&raw)
        .map_err(|e| PyErr::new::<PyValueError, _>(format!("Invalid schema: {e}")))
}

fn validator_for_schema(schema_json: &str) -> PyResult<ValidatorPy> {
    let schema = parse_schema(schema_json)?;
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

    let old_schema = SchemaDocument::from_json(&old_raw)
        .map_err(|e| PyErr::new::<PyValueError, _>(format!("Invalid old schema: {e}")))?;
    let new_schema = SchemaDocument::from_json(&new_raw)
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
    validator_for_schema(schema_json)?.is_valid(instance_json)
}

/// Python module definition
#[pymodule]
#[pyo3(name = "_native")]
fn jsoncompat_native(py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(check_compat_py, m)?)?;
    m.add_function(wrap_pyfunction!(generate_value_py, m)?)?;
    m.add_function(wrap_pyfunction!(generator_for_py, m)?)?;
    m.add_function(wrap_pyfunction!(validator_for_py, m)?)?;
    m.add_function(wrap_pyfunction!(is_valid_py, m)?)?;
    m.add_class::<GeneratorPy>()?;
    m.add_class::<ValidatorPy>()?;

    let role_constants = PyModule::new(py, "Role")?;
    role_constants.add("SERIALIZER", "serializer")?;
    role_constants.add("DESERIALIZER", "deserializer")?;
    role_constants.add("BOTH", "both")?;
    m.add_submodule(&role_constants)?;

    Ok(())
}
