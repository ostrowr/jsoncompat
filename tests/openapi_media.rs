use std::{fs, process::Command};
#[path = "support/python_env.rs"]
mod python_env;

#[test]
fn generated_http_media_preserve_wire_formats_and_validation() {
    let dir = std::env::temp_dir().join(format!(
        "jsoncompat-media-{}-{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    fs::create_dir(&dir).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_jsoncompat"))
        .args([
            "codegen",
            "--openapi",
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/openapi_media.json"
            ),
            "--api",
            "Media",
            "--output",
        ])
        .arg(dir.join("media"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::write(
        dir.join("check.py"),
        include_str!("support/openapi_media.py"),
    )
    .unwrap();
    let output = python_env::python_command()
        .arg("check.py")
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    fs::write(
        dir.join("consumer.py"),
        r#"
from typing import assert_type
from media import MediaClient, BinaryRequest, ScalarTextRequest, Binary200, JsonBinaryAnnotationRequest

def consume(client: MediaClient) -> None:
    binary = client.request(BinaryRequest(body=b"file")).unwrap()
    assert_type(binary, Binary200)
    assert_type(binary.body, bytes)
    scalar = client.request(ScalarTextRequest(body=3)).unwrap()
    assert_type(scalar.body, bool)
    text = client.request(JsonBinaryAnnotationRequest(body="hello")).unwrap()
    assert_type(text.body, str)
"#,
    )
    .unwrap();
    fs::write(
        dir.join("ty.toml"),
        "[src]\ninclude = [\"media\", \"consumer.py\"]\n",
    )
    .unwrap();
    let output = python_env::ty_command().current_dir(&dir).output().unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    fs::write(dir.join("invalid.py"), "from media import BinaryRequest\nBinaryRequest(body=\"text\")\nBinaryRequest(body=bytearray(b'file'))\n").unwrap();
    let invalid = python_env::ty_command()
        .current_dir(&dir)
        .arg("invalid.py")
        .output()
        .unwrap();
    assert!(!invalid.status.success());
    assert_eq!(
        String::from_utf8_lossy(&invalid.stdout)
            .matches("error[invalid-argument-type]")
            .count(),
        2,
        "{}",
        String::from_utf8_lossy(&invalid.stdout)
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn binary_projections_reject_adjacent_constraints_and_ambiguous_encodings() {
    let source: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/openapi_media.json")).unwrap();
    let dir = std::env::temp_dir().join(format!(
        "jsoncompat-media-reject-{}-{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    fs::create_dir(&dir).unwrap();
    let mut cases = Vec::new();
    let mut spec = source.clone();
    spec["components"]["schemas"]["File"] = serde_json::json!({"type":"string","format":"binary"});
    spec["paths"]["/binary"]["post"]["requestBody"]["content"]["application/octet-stream"]["schema"] =
        serde_json::json!({"$ref":"#/components/schemas/File","minLength":1});
    cases.push((spec, "adjacent validation assertions"));
    let mut spec = source.clone();
    spec["paths"]["/upload"]["post"]["requestBody"]["content"]["multipart/form-data"]["encoding"]
        ["file"]["contentType"] = serde_json::json!("image/*");
    cases.push((spec, "one concrete ASCII media type"));
    let mut spec = source;
    let content = &mut spec["paths"]["/upload"]["post"]["requestBody"]["content"];
    let value = content
        .as_object_mut()
        .unwrap()
        .remove("multipart/form-data")
        .unwrap();
    content["multipart/form-data; boundary=fixed"] = value;
    cases.push((spec, "omit a fixed boundary"));
    for (spec, expected) in cases {
        fs::write(dir.join("spec.json"), spec.to_string()).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_jsoncompat"))
            .args([
                "codegen",
                "--openapi",
                "spec.json",
                "--api",
                "Media",
                "--output",
                "media",
            ])
            .current_dir(&dir)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    fs::remove_dir_all(dir).unwrap();
}
