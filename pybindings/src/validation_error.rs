use jsoncompat_codegen::prepared_schema::{ValidationFailure, ValidationFailureKind};
use pyo3::{exceptions::PyValueError, prelude::*};
pyo3::create_exception!(jsoncompat, ValidationError, PyValueError);

pub(crate) fn from_failure(py: Python<'_>, failure: ValidationFailure) -> PyErr {
    let error = ValidationError::new_err(format!(
        "{} at {} ({})",
        failure.message,
        if failure.instance_path.is_empty() {
            "/"
        } else {
            &failure.instance_path
        },
        failure.schema_path
    ));
    let kind = match failure.kind {
        ValidationFailureKind::Constraint => "constraint",
        ValidationFailureKind::ResourceLimit => "resource_limit",
        ValidationFailureKind::NonJson => "non_json",
    };
    let value = error.value(py);
    let result = (|| -> PyResult<()> {
        value.setattr("kind", kind)?;
        value.setattr("instance_path", failure.instance_path)?;
        value.setattr("schema_path", failure.schema_path)?;
        value.setattr("keyword", failure.keyword)?;
        Ok(())
    })();
    result.err().unwrap_or(error)
}
