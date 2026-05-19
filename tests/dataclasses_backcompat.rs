#[path = "support/dataclass_round_trip.rs"]
mod dataclass_round_trip;
#[path = "support/python_env.rs"]
mod python_env;

use dataclass_round_trip::{DataclassGeneratedValueRoundTripper, write_generated_module};
use json_schema_ast::SchemaDocument;
use json_schema_fuzz::{GenerationConfig, ValueGenerator};
use jsoncompat_codegen::generate_dataclass_models;
use rand::{SeedableRng, rngs::StdRng};
use serde::Deserialize;
use serde_json::Value;
use std::error::Error;
use std::fs;
use std::path::Path;

datatest_stable::harness! {
    { test = fixture, root = "tests/fixtures/backcompat", pattern = r".*[/\\]expect\.json$" },
}

const GENERATED_VALUE_ITERATIONS: usize = 100;

#[derive(Deserialize, Default)]
struct SampleSets {
    #[serde(default)]
    old_only: Vec<Value>,
    #[serde(default)]
    new_only: Vec<Value>,
    #[serde(default)]
    both: Vec<Value>,
}

fn fixture(expect_file: &Path) -> Result<(), Box<dyn Error>> {
    let fixture_dir = expect_file
        .parent()
        .expect("backcompat fixtures have a fixture directory");
    let case_name = fixture_dir
        .file_name()
        .and_then(|name| name.to_str())
        .expect("utf-8 fixture directory name");
    let old_raw = read_json(&fixture_dir.join("old.json"))?;
    let new_raw = read_json(&fixture_dir.join("new.json"))?;
    let old_schema = SchemaDocument::from_json(&old_raw)?;
    let new_schema = SchemaDocument::from_json(&new_raw)?;
    let mut old_round_tripper = build_round_tripper(case_name, "old", &old_raw)?;
    let mut new_round_tripper = build_round_tripper(case_name, "new", &new_raw)?;

    let examples_file = fixture_dir.join("examples.json");
    if examples_file.exists() {
        let samples: SampleSets = serde_json::from_slice(&fs::read(examples_file)?)?;
        round_trip_curated_samples(
            case_name,
            &old_schema,
            &new_schema,
            old_round_tripper.as_mut(),
            new_round_tripper.as_mut(),
            &samples,
        )?;
    }

    let mut rng = StdRng::seed_from_u64(0xDADA_CAFE + case_name.len() as u64);
    round_trip_generated_samples(
        case_name,
        "old",
        &old_schema,
        old_round_tripper.as_mut(),
        "new",
        &new_schema,
        new_round_tripper.as_mut(),
        &mut rng,
    )?;
    round_trip_generated_samples(
        case_name,
        "new",
        &new_schema,
        new_round_tripper.as_mut(),
        "old",
        &old_schema,
        old_round_tripper.as_mut(),
        &mut rng,
    )?;

    Ok(())
}

fn round_trip_curated_samples(
    case_name: &str,
    old_schema: &SchemaDocument,
    new_schema: &SchemaDocument,
    mut old_round_tripper: Option<&mut DataclassGeneratedValueRoundTripper>,
    mut new_round_tripper: Option<&mut DataclassGeneratedValueRoundTripper>,
    samples: &SampleSets,
) -> Result<(), Box<dyn Error>> {
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
            old_schema,
            old_round_tripper.as_deref_mut(),
            sample,
        )?;
        reject_invalid(case_name, "new", new_round_tripper.as_deref_mut(), sample);
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
            new_schema,
            new_round_tripper.as_deref_mut(),
            sample,
        )?;
        reject_invalid(case_name, "old", old_round_tripper.as_deref_mut(), sample);
    }

    for sample in &samples.both {
        assert!(
            old_schema.is_valid(sample)? && new_schema.is_valid(sample)?,
            "{case_name} both sample must validate under both schemas: {sample:?}"
        );
        round_trip_valid(
            case_name,
            "old",
            old_schema,
            old_round_tripper.as_deref_mut(),
            sample,
        )?;
        round_trip_valid(
            case_name,
            "new",
            new_schema,
            new_round_tripper.as_deref_mut(),
            sample,
        )?;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn round_trip_generated_samples(
    case_name: &str,
    source_side: &str,
    source_schema: &SchemaDocument,
    mut source_round_tripper: Option<&mut DataclassGeneratedValueRoundTripper>,
    counterpart_side: &str,
    counterpart_schema: &SchemaDocument,
    mut counterpart_round_tripper: Option<&mut DataclassGeneratedValueRoundTripper>,
    rng: &mut StdRng,
) -> Result<(), Box<dyn Error>> {
    let config = GenerationConfig::new(4);
    for _ in 0..GENERATED_VALUE_ITERATIONS {
        let sample = ValueGenerator::generate(source_schema, config, rng)?;
        assert!(
            source_schema.is_valid(&sample)?,
            "{case_name}/{source_side} generator emitted an invalid sample: {sample:?}"
        );
        round_trip_valid(
            case_name,
            source_side,
            source_schema,
            source_round_tripper.as_deref_mut(),
            &sample,
        )?;

        if counterpart_schema.is_valid(&sample)? {
            round_trip_valid(
                case_name,
                counterpart_side,
                counterpart_schema,
                counterpart_round_tripper.as_deref_mut(),
                &sample,
            )?;
        } else {
            reject_invalid(
                case_name,
                counterpart_side,
                counterpart_round_tripper.as_deref_mut(),
                &sample,
            );
        }
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
                "{case_name}/{side} generated dataclass rejected a valid sample {sample:?}: {message}"
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
                "{case_name}/{side} generated dataclass accepted an invalid sample {sample:?}: {message}"
            )
        });
}

fn read_json(path: &Path) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}
