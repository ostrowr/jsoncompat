#[path = "support/dataclass_corpus.rs"]
mod corpus;
use jsoncompat_codegen::generate_dataclass_models;
use serde::Deserialize;
use serde_json::Value;
use std::fs;

#[path = "support/python_env.rs"]
mod python_env;

#[derive(Deserialize)]
struct Case {
    name: String,
    schema: Value,
}

#[test]
fn optimized_and_general_models_match_independent_validation() {
    let output = python_env::python_command()
        .arg("tests/support/dataclass_differential_cases.py")
        .output()
        .expect("produce differential cases");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let cases: Vec<Case> = serde_json::from_slice(&output.stdout).expect("case manifest");
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "jsoncompat-differential-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&directory).expect("create test directory");
    fs::write(directory.join("cases.json"), output.stdout).expect("write manifest");
    for case in cases {
        let source = generate_dataclass_models(&case.schema)
            .unwrap_or_else(|error| panic!("generate {}: {error}", case.name));
        fs::write(directory.join(format!("{}.py", case.name)), source).expect("write model");
    }
    corpus::generate(&directory.join("fixtures"));
    let output = python_env::python_command()
        .arg("tests/support/python_dataclasses_differential.py")
        .env("JSONCOMPAT_TEST_CLI", env!("CARGO_BIN_EXE_jsoncompat"))
        .arg(&directory)
        .output()
        .expect("run differential tests");
    print!("{}", String::from_utf8_lossy(&output.stdout));
    assert!(
        output.status.success(),
        "differential tests failed; artifacts at {}:\n{}\n{}",
        directory.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    fs::remove_dir_all(directory).expect("remove differential artifacts");
}

#[test]
fn asserted_formats_are_never_silently_dropped_by_codegen() {
    let options = json_schema_ast::SchemaOptions {
        assert_formats: Some(true),
        ..Default::default()
    };
    let schema = serde_json::json!({"type":"string", "format":"email"});
    let document =
        json_schema_ast::SchemaDocument::from_json_with_options(&schema, &options).unwrap();
    assert!(
        document
            .is_valid(&serde_json::json!("ok@example.com"))
            .unwrap()
    );
    assert!(!document.is_valid(&serde_json::json!("invalid")).unwrap());
    let error = jsoncompat_codegen::generate_dataclass_module_from_document(&document)
        .err()
        .expect("unsupported asserted formats must fail at generation");
    assert!(error.to_string().contains("format-assertion"), "{error}");
}
