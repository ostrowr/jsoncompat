#[path = "support/dataclass_round_trip.rs"]
mod dataclass_round_trip;
#[path = "support/python_env.rs"]
mod python_env;

use dataclass_round_trip::{StampedDataclassRoundTripper, write_generated_module};
use json_schema_ast::SchemaDocument;
use json_schema_fuzz::{GenerationConfig, ValueGenerator};
use jsoncompat::{StampManifest, StampResult, stamp_schema};
use jsoncompat_codegen::generate_dataclass_models;
use rand::{SeedableRng, rngs::StdRng};
use serde::Deserialize;
use serde_json::{Value, json};
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

struct StampedModels {
    result: StampResult,
    writer_schema: SchemaDocument,
    reader_schema: SchemaDocument,
    round_tripper: StampedDataclassRoundTripper,
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
    let mut stamped = build_stamped_models(case_name, old_raw, new_raw)?;

    let examples_file = fixture_dir.join("examples.json");
    if examples_file.exists() {
        let samples: SampleSets = serde_json::from_slice(&fs::read(examples_file)?)?;
        exercise_curated_samples(case_name, &new_schema, &mut stamped, &samples)?;
    }

    let mut rng = StdRng::seed_from_u64(0x57A4_4D50 + case_name.len() as u64);
    exercise_generated_payloads(
        case_name,
        "old",
        &old_schema,
        &new_schema,
        &mut stamped,
        &mut rng,
    )?;
    exercise_generated_payloads(
        case_name,
        "new",
        &new_schema,
        &new_schema,
        &mut stamped,
        &mut rng,
    )?;
    exercise_historical_reader_payloads(case_name, &mut stamped, &mut rng)?;

    Ok(())
}

fn build_stamped_models(
    case_name: &str,
    old_raw: Value,
    new_raw: Value,
) -> Result<StampedModels, Box<dyn Error>> {
    let stable_id = format!("tests/backcompat/{case_name}");
    let first = stamp_schema(&StampManifest::empty(), &stable_id, old_raw)?;
    let result = stamp_schema(&first.manifest, &stable_id, new_raw)?;
    let writer_source = generate_dataclass_models(&result.bundle.writer)?;
    let reader_source = generate_dataclass_models(&result.bundle.reader)?;
    let writer_module_path = write_generated_module(
        "dataclasses-stamp-backcompat",
        case_name,
        "writer",
        &writer_source,
    )?;
    let reader_module_path = write_generated_module(
        "dataclasses-stamp-backcompat",
        case_name,
        "reader",
        &reader_source,
    )?;

    Ok(StampedModels {
        writer_schema: SchemaDocument::from_json(&result.bundle.writer)?,
        reader_schema: SchemaDocument::from_json(&result.bundle.reader)?,
        round_tripper: StampedDataclassRoundTripper::spawn(writer_module_path, reader_module_path)?,
        result,
    })
}

fn exercise_curated_samples(
    case_name: &str,
    latest_schema: &SchemaDocument,
    stamped: &mut StampedModels,
    samples: &SampleSets,
) -> Result<(), Box<dyn Error>> {
    for sample in samples
        .old_only
        .iter()
        .chain(&samples.new_only)
        .chain(&samples.both)
    {
        exercise_latest_writer_sample(case_name, latest_schema, stamped, sample)?;
        exercise_reader_sample_against_all_versions(case_name, stamped, sample)?;
    }
    Ok(())
}

fn exercise_generated_payloads(
    case_name: &str,
    source_side: &str,
    source_schema: &SchemaDocument,
    latest_schema: &SchemaDocument,
    stamped: &mut StampedModels,
    rng: &mut StdRng,
) -> Result<(), Box<dyn Error>> {
    let config = GenerationConfig::new(4);
    for _ in 0..GENERATED_VALUE_ITERATIONS {
        let sample = ValueGenerator::generate(source_schema, config, rng)?;
        assert!(
            source_schema.is_valid(&sample)?,
            "{case_name}/{source_side} generator emitted an invalid payload: {sample:?}"
        );
        exercise_latest_writer_sample(case_name, latest_schema, stamped, &sample)?;
        exercise_reader_sample_against_all_versions(case_name, stamped, &sample)?;
    }
    Ok(())
}

fn exercise_historical_reader_payloads(
    case_name: &str,
    stamped: &mut StampedModels,
    rng: &mut StdRng,
) -> Result<(), Box<dyn Error>> {
    let config = GenerationConfig::new(4);
    let versions = stamped.result.bundle.versions.clone();
    for version in versions {
        let version_schema = SchemaDocument::from_json(&version.schema)?;
        for _ in 0..GENERATED_VALUE_ITERATIONS {
            let sample = ValueGenerator::generate(&version_schema, config, rng)?;
            assert!(
                version_schema.is_valid(&sample)?,
                "{case_name}/v{} generator emitted an invalid payload: {sample:?}",
                version.version
            );
            round_trip_reader_valid(case_name, stamped, version.version, &sample)?;
        }
    }
    Ok(())
}

fn exercise_latest_writer_sample(
    case_name: &str,
    latest_schema: &SchemaDocument,
    stamped: &mut StampedModels,
    sample: &Value,
) -> Result<(), Box<dyn Error>> {
    if latest_schema.is_valid(sample)? {
        round_trip_writer_valid(case_name, stamped, sample)?;
    } else {
        reject_writer_invalid(case_name, stamped, sample);
    }
    Ok(())
}

fn exercise_reader_sample_against_all_versions(
    case_name: &str,
    stamped: &mut StampedModels,
    sample: &Value,
) -> Result<(), Box<dyn Error>> {
    let versions = stamped.result.bundle.versions.clone();
    for version in versions {
        let version_schema = SchemaDocument::from_json(&version.schema)?;
        if version_schema.is_valid(sample)? {
            round_trip_reader_valid(case_name, stamped, version.version, sample)?;
        } else {
            reject_reader_invalid(case_name, stamped, version.version, sample);
        }
    }
    Ok(())
}

fn round_trip_writer_valid(
    case_name: &str,
    stamped: &mut StampedModels,
    payload: &Value,
) -> Result<(), Box<dyn Error>> {
    let emitted = stamped
        .round_tripper
        .round_trip_writer_payload(payload)
        .unwrap_or_else(|message| {
            panic!(
                "{case_name} stamped writer rejected a valid latest payload {payload:?}: {message}"
            )
        });
    assert!(
        stamped.writer_schema.is_valid(&emitted)?,
        "{case_name} stamped writer emitted invalid JSON {emitted:?} from payload {payload:?}"
    );
    Ok(())
}

fn reject_writer_invalid(case_name: &str, stamped: &mut StampedModels, payload: &Value) {
    stamped
        .round_tripper
        .reject_writer_payload(payload)
        .unwrap_or_else(|message| {
            panic!(
                "{case_name} stamped writer accepted a payload rejected by the latest schema {payload:?}: {message}"
            )
        });
}

fn round_trip_reader_valid(
    case_name: &str,
    stamped: &mut StampedModels,
    version: u32,
    payload: &Value,
) -> Result<(), Box<dyn Error>> {
    let envelope = envelope(version, payload);
    let emitted = stamped
        .round_tripper
        .round_trip_reader_envelope(&envelope)
        .unwrap_or_else(|message| {
            panic!(
                "{case_name} stamped reader rejected valid v{version} envelope {envelope:?}: {message}"
            )
        });
    assert!(
        stamped.reader_schema.is_valid(&emitted)?,
        "{case_name} stamped reader emitted invalid JSON {emitted:?} from envelope {envelope:?}"
    );
    Ok(())
}

fn reject_reader_invalid(
    case_name: &str,
    stamped: &mut StampedModels,
    version: u32,
    payload: &Value,
) {
    let envelope = envelope(version, payload);
    stamped
        .round_tripper
        .reject_reader_envelope(&envelope)
        .unwrap_or_else(|message| {
            panic!(
                "{case_name} stamped reader accepted invalid v{version} envelope {envelope:?}: {message}"
            )
        });
}

fn envelope(version: u32, payload: &Value) -> Value {
    json!({
        "version": version,
        "data": payload,
    })
}

fn read_json(path: &Path) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}
