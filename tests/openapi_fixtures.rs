use jsoncompat::{OpenApiDocument, check_openapi_compat};
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

datatest_stable::harness! {
    { test = fixture, root = "tests/fixtures/openapi_compat", pattern = r".*[/\\]expect\.json$" },
}

#[derive(Debug, Deserialize)]
struct Expectation {
    compatible: bool,
    #[serde(default)]
    surfaces: Vec<String>,
    #[serde(default)]
    expected_message: Option<String>,
}

fn fixture(expect_file: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let dir: PathBuf = expect_file.parent().unwrap().into();
    let old_raw: Value = serde_json::from_slice(&fs::read(dir.join("old.json"))?)?;
    let new_raw: Value = serde_json::from_slice(&fs::read(dir.join("new.json"))?)?;
    let expect: Expectation = serde_json::from_slice(&fs::read(expect_file)?)?;

    let old = OpenApiDocument::from_json(&old_raw)?;
    let new = OpenApiDocument::from_json(&new_raw)?;
    let report = check_openapi_compat(&old, &new)?;

    assert_eq!(
        report.is_compatible(),
        expect.compatible,
        "compatibility mismatch in {dir:?}: {report:?}"
    );
    let actual_surfaces = report
        .issues()
        .iter()
        .map(|issue| format!("{:?}", issue.surface))
        .collect::<Vec<_>>();
    assert_eq!(
        actual_surfaces, expect.surfaces,
        "issue surface mismatch in {dir:?}: {report:?}"
    );
    match (expect.expected_message.as_deref(), report.issues()) {
        (Some(expected_message), [issue]) => assert_eq!(
            issue.message, expected_message,
            "issue message mismatch in {dir:?}: {report:?}"
        ),
        (Some(_), issues) => {
            panic!("fixture {dir:?} expects exactly one explained incompatibility, got {issues:?}")
        }
        (None, []) if expect.compatible => {}
        (None, _) if !expect.compatible => {
            panic!("incompatible fixture {dir:?} must define `expected_message`")
        }
        (None, issues) => panic!("compatible fixture {dir:?} reported issues: {issues:?}"),
    }

    Ok(())
}
