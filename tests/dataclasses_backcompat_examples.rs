#[path = "support/dataclass_round_trip.rs"]
mod dataclass_round_trip;
#[path = "support/python_env.rs"]
mod python_env;

use dataclass_round_trip::{DataclassGeneratedValueRoundTripper, write_generated_module};
use json_schema_ast::SchemaDocument;
use jsoncompat_codegen::generate_dataclass_models;
use serde::Deserialize;
use serde_json::Value;
use std::error::Error;
use std::fs;
use std::path::Path;

datatest_stable::harness! {
    { test = fixture, root = "tests/fixtures/backcompat", pattern = r".*[/\\]examples\.json$" },
}

#[derive(Deserialize, Default)]
struct SampleSets {
    #[serde(default)]
    old_only: Vec<Value>,
    #[serde(default)]
    new_only: Vec<Value>,
    #[serde(default)]
    both: Vec<Value>,
}

fn fixture(examples_file: &Path) -> Result<(), Box<dyn Error>> {
    let fixture_dir = examples_file
        .parent()
        .expect("backcompat examples have a fixture directory");
    let case_name = fixture_dir
        .file_name()
        .and_then(|name| name.to_str())
        .expect("utf-8 fixture directory name");
    let old_raw = read_json(&fixture_dir.join("old.json"))?;
    let new_raw = read_json(&fixture_dir.join("new.json"))?;
    let samples: SampleSets = serde_json::from_slice(&fs::read(examples_file)?)?;

    let old_schema = SchemaDocument::from_json(&old_raw)?;
    let new_schema = SchemaDocument::from_json(&new_raw)?;
    let mut old_round_tripper = build_round_tripper(case_name, "old", &old_raw)?;
    let mut new_round_tripper = build_round_tripper(case_name, "new", &new_raw)?;

    for sample in &samples.old_only {
        assert!(
            old_schema.is_valid(sample)?,
            "{case_name} old_only sample is invalid under the old schema: {sample:?}"
        );
        assert!(
            !new_schema.is_valid(sample)?,
            "{case_name} old_only sample unexpectedly matches the new schema: {sample:?}"
        );
        round_trip_valid(
            case_name,
            "old",
            &old_schema,
            old_round_tripper.as_mut(),
            sample,
        )?;
        reject_invalid(case_name, "new", new_round_tripper.as_mut(), sample);
    }

    for sample in &samples.new_only {
        assert!(
            new_schema.is_valid(sample)?,
            "{case_name} new_only sample is invalid under the new schema: {sample:?}"
        );
        assert!(
            !old_schema.is_valid(sample)?,
            "{case_name} new_only sample unexpectedly matches the old schema: {sample:?}"
        );
        round_trip_valid(
            case_name,
            "new",
            &new_schema,
            new_round_tripper.as_mut(),
            sample,
        )?;
        reject_invalid(case_name, "old", old_round_tripper.as_mut(), sample);
    }

    for sample in &samples.both {
        assert!(
            old_schema.is_valid(sample)? && new_schema.is_valid(sample)?,
            "{case_name} both sample must validate under both schemas: {sample:?}"
        );
        round_trip_valid(
            case_name,
            "old",
            &old_schema,
            old_round_tripper.as_mut(),
            sample,
        )?;
        round_trip_valid(
            case_name,
            "new",
            &new_schema,
            new_round_tripper.as_mut(),
            sample,
        )?;
    }

    Ok(())
}

fn build_round_tripper(
    case_name: &str,
    side: &str,
    schema: &Value,
) -> Result<Option<DataclassGeneratedValueRoundTripper>, Box<dyn Error>> {
    let source = match generate_dataclass_models(schema) {
        Ok(source) => source,
        Err(error) => {
            assert_codegen_error_snapshot(case_name, side, &error.to_string())?;
            return Ok(None);
        }
    };
    let module_path = write_generated_module("dataclasses-backcompat", case_name, side, &source)?;
    Ok(Some(DataclassGeneratedValueRoundTripper::spawn(
        module_path,
    )?))
}

fn assert_codegen_error_snapshot(
    case_name: &str,
    side: &str,
    error: &str,
) -> Result<(), Box<dyn Error>> {
    let snapshot_path = Path::new("tests/fixtures/dataclasses/backcompat")
        .join(case_name)
        .join(format!("{side}.error.txt"));
    let expected = fs::read_to_string(&snapshot_path).unwrap_or_else(|read_error| {
        panic!(
            "dataclass backcompat example skipped {case_name}/{side} without an explicit error snapshot {}: {read_error}",
            snapshot_path.display(),
        )
    });
    let actual = format!("{error}\n");
    assert_eq!(
        expected,
        actual,
        "dataclass backcompat example codegen error snapshot is stale for {case_name}/{side}: {}",
        snapshot_path.display(),
    );
    Ok(())
}

fn round_trip_valid(
    case_name: &str,
    side: &str,
    schema: &SchemaDocument,
    round_tripper: Option<&mut DataclassGeneratedValueRoundTripper>,
    sample: &Value,
) -> Result<(), Box<dyn Error>> {
    let Some(round_tripper) = round_tripper else {
        return Ok(());
    };
    let emitted = round_tripper
        .round_trip_value(sample)
        .unwrap_or_else(|message| {
            panic!(
                "{case_name}/{side} generated dataclass rejected a valid curated sample {sample:?}: {message}"
            )
        });
    assert!(
        schema.is_valid(&emitted)?,
        "{case_name}/{side} generated dataclass emitted invalid JSON {emitted:?} from sample {sample:?}",
    );
    Ok(())
}

fn reject_invalid(
    case_name: &str,
    side: &str,
    round_tripper: Option<&mut DataclassGeneratedValueRoundTripper>,
    sample: &Value,
) {
    let Some(round_tripper) = round_tripper else {
        return;
    };
    round_tripper
        .reject_invalid_value(sample)
        .unwrap_or_else(|message| {
            panic!(
                "{case_name}/{side} generated dataclass accepted a curated invalid sample {sample:?}: {message}"
            )
        });
}

fn read_json(path: &Path) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}
