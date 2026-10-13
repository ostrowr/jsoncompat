//! Keep the opt-in corpus audit's format/provenance helpers under the local gate.

#[path = "support/python_env.rs"]
mod python_env;

#[test]
fn public_openapi_audit_helpers_preserve_source_semantics() {
    let output = python_env::python_command()
        .args(["-m", "unittest", "discover", "-s"])
        .arg(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("pybindings/tests"))
        .args(["-p", "test_public_openapi*.py"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
