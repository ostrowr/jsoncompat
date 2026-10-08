use jsoncompat::{OpenApiCompatibilitySurface, OpenApiDocument, check_openapi_compat};
use jsoncompat_openapi::{OpenApiOperationLowerer, OperationKey};
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

datatest_stable::harness! {
    { test = fixture, root = "tests/fixtures/openapi_compat", pattern = r".*[/\\]expect\.json$" },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Expectation {
    compatible: bool,
    #[serde(default)]
    surfaces: Vec<String>,
    #[serde(default)]
    expected_message: Option<String>,
}

#[derive(Clone, Copy, Deserialize)]
enum Surface {
    Request,
    Response,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Example {
    method: String,
    path: String,
    surface: Surface,
    data: Value,
    old: bool,
    new: bool,
}

fn fixture(expect_file: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let dir: PathBuf = expect_file.parent().unwrap().into();
    let old_raw: Value = serde_json::from_slice(&fs::read(dir.join("old.json"))?)?;
    let new_raw: Value = serde_json::from_slice(&fs::read(dir.join("new.json"))?)?;
    let expect: Expectation = serde_json::from_slice(&fs::read(expect_file)?)?;

    let old = OpenApiDocument::from_json(&old_raw)?;
    let new = OpenApiDocument::from_json(&new_raw)?;
    let report = check_openapi_compat(&old, &new)?;

    // Audit labels against actual lowered-contract validation. Every reported
    // request/response incompatibility has a concrete, checked-in witness;
    // operation removals have a structural witness instead.
    let examples: Vec<Example> = serde_json::from_slice(&fs::read(dir.join("examples.json"))?)?;
    let old_lowerer = OpenApiOperationLowerer::new(&old)?;
    let new_lowerer = OpenApiOperationLowerer::new(&new)?;
    let mut witnessed_issues = Vec::new();
    for example in examples {
        let operation = OperationKey {
            method: example.method,
            path: example.path,
        };
        let old_operation = old_lowerer
            .lower_operation(&operation)?
            .expect("old example operation exists");
        let new_operation = new_lowerer
            .lower_operation(&operation)?
            .expect("new example operation exists");
        let (old_raw, new_raw, surface, breaks) = match example.surface {
            Surface::Request => (
                &old_operation.request,
                &new_operation.request,
                OpenApiCompatibilitySurface::Request,
                example.old && !example.new,
            ),
            Surface::Response => (
                &old_operation.response,
                &new_operation.response,
                OpenApiCompatibilitySurface::Response,
                example.new && !example.old,
            ),
        };
        assert_eq!(
            old.lowered_contract_document(old_raw)?
                .is_valid(&example.data)?,
            example.old,
            "old example label in {dir:?}: {}",
            example.data
        );
        assert_eq!(
            new.lowered_contract_document(new_raw)?
                .is_valid(&example.data)?,
            example.new,
            "new example label in {dir:?}: {}",
            example.data
        );
        if breaks {
            assert!(
                report
                    .issues()
                    .iter()
                    .any(|issue| issue.method == operation.method
                        && issue.path == operation.path
                        && issue.surface == surface),
                "unreported {surface:?} counterexample in {dir:?}: {}",
                example.data
            );
            witnessed_issues.push((operation, surface));
        }
    }
    for issue in report.issues() {
        let operation = OperationKey {
            method: issue.method.clone(),
            path: issue.path.clone(),
        };
        if issue.surface == OpenApiCompatibilitySurface::Operation {
            assert!(old_lowerer.lower_operation(&operation)?.is_some());
            assert!(new_lowerer.lower_operation(&operation)?.is_none());
        } else {
            assert!(
                witnessed_issues.contains(&(operation, issue.surface)),
                "incompatible fixture {dir:?} needs a concrete {:?} counterexample",
                issue.surface
            );
        }
    }

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
