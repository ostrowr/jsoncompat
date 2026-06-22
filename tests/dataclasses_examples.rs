use jsoncompat_codegen::generate_dataclass_models;
use serde_json::Value;
use std::fs;
use std::path::Path;

#[path = "support/python_env.rs"]
mod python_env;

fn read_json(path: impl AsRef<Path>) -> Value {
    let bytes = fs::read(path).expect("read json file");
    serde_json::from_slice(&bytes).expect("parse json file")
}

fn read_text(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path).expect("read text file")
}

#[test]
fn plain_schema_example_snapshot_is_up_to_date() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/dataclasses");
    let schema = read_json(root.join("schema.json"));
    let generated = generate_dataclass_models(&schema).unwrap();

    assert_eq!(
        normalized_newlines(&generated),
        normalized_newlines(&read_text(root.join("models.py")))
    );

    assert_python_compiles(&root.join("models.py"));
    assert_python_compiles(&root.join("demo.py"));
}

#[test]
fn plain_schema_python_example_exercises_generated_model_lifecycle() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/dataclasses");

    let mut command = python_env::python_command();
    command
        .env_remove("PYTHONSAFEPATH")
        .args(["-B", root.join("demo.py").to_str().unwrap()]);
    let demo = command
        .output()
        .expect("run canonical plain-schema Python example");
    assert!(
        demo.status.success(),
        "canonical plain-schema Python example failed: {}",
        String::from_utf8_lossy(&demo.stderr)
    );
    let stdout = String::from_utf8(demo.stdout).unwrap();
    assert_eq!(
        normalized_newlines(&stdout),
        concat!(
            "Python value: order-123: 3 units for Ada\n",
            "JSON: order-123: 3 units for Ada\n",
            "YAML: order-123: 3 units for Ada\n",
            "MessagePack: order-123: 3 units for Ada\n",
            "Omitted and null notes remain distinct\n",
            "Trusted path matches checked path\n",
            "Invalid input rejected\n",
        )
    );
}

fn normalized_newlines(contents: &str) -> String {
    contents.replace("\r\n", "\n")
}

fn assert_python_compiles(path: &Path) {
    let mut command = python_env::python_command();
    command.args(["-B", "-m", "py_compile", path.to_str().unwrap()]);

    let py_compile = command.output().expect("run python py_compile");
    assert!(
        py_compile.status.success(),
        "generated dataclasses example {} did not compile: {}",
        path.display(),
        String::from_utf8_lossy(&py_compile.stderr)
    );
}
