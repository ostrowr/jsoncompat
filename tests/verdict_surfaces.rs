use jsoncompat::{OpenApiCompatibilityIssueKind, OpenApiDocument, check_openapi_compat};
use serde_json::{Value, json};
use std::{fs, process::Command};
fn equivalent_pair() -> (Value, Value) {
    let properties: serde_json::Map<_, _> =
        (0..9).map(|i| (format!("p{i}"), json!(true))).collect();
    let branches: Vec<_> = properties
        .keys()
        .map(|name| json!({"properties":{name:true}}))
        .collect();
    (
        json!({"type":"object","properties":properties,"additionalProperties":false}),
        json!({"type":"object","anyOf":branches,"unevaluatedProperties":false}),
    )
}
fn openapi(schema: Value) -> Value {
    json!({"openapi":"3.1.0","info":{"title":"Verdict regression","version":"1"},"paths":{"/example":{"get":{"responses":{"200":{"description":"ok","content":{"application/json":{"schema":schema}}}}}}}})
}
#[test]
fn unknown_survives_openapi_cli_and_golden_file_grading() {
    let (old, new) = equivalent_pair();
    let documents = (openapi(old.clone()), openapi(new.clone()));
    let report = check_openapi_compat(
        &OpenApiDocument::from_json(&documents.0).unwrap(),
        &OpenApiDocument::from_json(&documents.1).unwrap(),
    )
    .unwrap();
    assert!(report.is_unknown());
    assert!(!report.is_incompatible());
    assert!(matches!(
        report.issues()[0].kind,
        OpenApiCompatibilityIssueKind::Unknown { .. }
    ));
    let path = std::env::temp_dir().join(format!(
        "jsoncompat-verdict-{}-{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    fs::create_dir(&path).unwrap();
    for (name, value) in [
        ("old.json", old.clone()),
        ("new.json", new.clone()),
        ("old-api.json", documents.0),
        ("new-api.json", documents.1),
        (
            "old-golden.json",
            json!({"model":{"mode":"serializer","stable_id":"model","schema":old}}),
        ),
        (
            "new-golden.json",
            json!({"model":{"mode":"serializer","stable_id":"model","schema":new}}),
        ),
    ] {
        fs::write(path.join(name), serde_json::to_vec(&value).unwrap()).unwrap();
    }
    for args in [
        vec!["compat", "old.json", "new.json", "--json"],
        vec![
            "compat",
            "old-api.json",
            "new-api.json",
            "--openapi",
            "--json",
        ],
        vec![
            "ci",
            "old-golden.json",
            "new-golden.json",
            "--display",
            "json",
        ],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_jsoncompat"))
            .args(&args)
            .current_dir(&path)
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(2),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let result: Value =
            serde_json::from_slice(&output.stdout).expect("stdout must be one JSON document");
        if args[0] == "ci" {
            assert!(result[0]["status"].get("Unknown").is_some());
        } else {
            assert_eq!(result["status"], "unknown");
        }
    }
    let output = Command::new(env!("CARGO_BIN_EXE_jsoncompat"))
        .args(["codegen", "--target", "dataclasses", "old-api.json"])
        .current_dir(&path)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("OpenAPI document"));
    fs::remove_dir_all(path).unwrap();
}
