use jsoncompat::{StampManifest, stamp_schema};
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
fn stamp_example_snapshots_are_up_to_date() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/stamp");
    let schema_v1 = read_json(root.join("schema-v1.json"));
    let schema_v2 = read_json(root.join("schema-v2.json"));

    let first = stamp_schema(&StampManifest::empty(), "user-profile", schema_v1).unwrap();
    let second = stamp_schema(&first.manifest, "user-profile", schema_v2).unwrap();

    let manifest = serde_json::to_value(&second.manifest).unwrap();
    let bundle = serde_json::to_value(&second.bundle).unwrap();
    let writer_dataclasses = generate_dataclass_models(&second.bundle.writer).unwrap();
    let reader_dataclasses = generate_dataclass_models(&second.bundle.reader).unwrap();

    assert_eq!(manifest, read_json(root.join("manifest.json")));
    assert_eq!(bundle, read_json(root.join("bundle.json")));
    assert_eq!(
        writer_dataclasses,
        read_text(root.join("writer.dataclasses.py"))
    );
    assert_eq!(
        reader_dataclasses,
        read_text(root.join("reader.dataclasses.py"))
    );

    assert_python_compiles(&root.join("writer.dataclasses.py"));
    assert_python_compiles(&root.join("reader.dataclasses.py"));
}

fn assert_python_compiles(path: &Path) {
    let mut command = python_env::python_command();
    command.args(["-B", "-m", "py_compile", path.to_str().unwrap()]);

    let py_compile = command.output().expect("run python py_compile");
    assert!(
        py_compile.status.success(),
        "generated dataclasses fixture {} did not compile: {}",
        path.display(),
        String::from_utf8_lossy(&py_compile.stderr)
    );
}
