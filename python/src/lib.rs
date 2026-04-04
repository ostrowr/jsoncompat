use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use ::jsoncompat::{ResolvedSchema, Role, check_compat};
use json_schema_fuzz::{GenerateError, generate_value};

use serde_json::Value as JsonValue;

/// Parse a JSON string into a serde_json::Value, converting any error into a Python ValueError.
fn parse_json(s: &str) -> PyResult<JsonValue> {
    serde_json::from_str(s).map_err(|e| PyErr::new::<PyValueError, _>(format!("Invalid JSON: {e}")))
}

/// Map a string into the Role enum, raising ValueError on unknown input.
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

    let old_schema = ResolvedSchema::from_json(&old_raw)
        .map_err(|e| PyErr::new::<PyValueError, _>(format!("Invalid old schema: {e}")))?;
    let new_schema = ResolvedSchema::from_json(&new_raw)
        .map_err(|e| PyErr::new::<PyValueError, _>(format!("Invalid new schema: {e}")))?;

    let old_root = old_schema
        .root()
        .map_err(|e| PyErr::new::<PyValueError, _>(format!("Invalid old schema: {e}")))?;
    let new_root = new_schema
        .root()
        .map_err(|e| PyErr::new::<PyValueError, _>(format!("Invalid new schema: {e}")))?;

    Ok(check_compat(old_root, new_root, role_e))
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
    let raw = parse_json(schema_json)?;
    let schema = ResolvedSchema::from_json(&raw)
        .map_err(|e| PyErr::new::<PyValueError, _>(format!("Invalid schema: {e}")))?;

    let mut rng = rand::rng();
    let value = generate_value(&schema, &mut rng, depth).map_err(|error| match error {
        GenerateError::Schema(error) => {
            PyErr::new::<PyValueError, _>(format!("Invalid schema: {error}"))
        }
        GenerateError::ExhaustedAttempts { .. } => PyErr::new::<PyValueError, _>(error.to_string()),
        _ => PyErr::new::<PyValueError, _>(error.to_string()),
    })?;

    serde_json::to_string(&value).map_err(|e| {
        PyErr::new::<PyValueError, _>(format!("Failed to serialize generated value: {e}"))
    })
}

/// Python module definition
#[pymodule]
#[pyo3(name = "jsoncompat")]
fn jsoncompat(py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Ensure the random generator is initialised (auto‑initialize takes care of pyo3 env).
    m.add_function(wrap_pyfunction!(check_compat_py, m)?)?;
    m.add_function(wrap_pyfunction!(generate_value_py, m)?)?;

    // Expose the Role enum for convenience.
    let role_enum = PyModule::new(py, "Role")?;
    role_enum.add("SERIALIZER", "serializer")?;
    role_enum.add("DESERIALIZER", "deserializer")?;
    role_enum.add("BOTH", "both")?;
    m.add_submodule(&role_enum)?;

    Ok(())
}
