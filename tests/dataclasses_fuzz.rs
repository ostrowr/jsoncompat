#[path = "support/dataclass_round_trip.rs"]
mod dataclass_round_trip;
#[path = "support/generated_value_harness.rs"]
mod generated_value_harness;
#[path = "support/python_env.rs"]
mod python_env;

use dataclass_round_trip::{DataclassGeneratedValueRoundTripper, write_generated_module};
use generated_value_harness::{
    FuzzSchemaCase, GeneratedValueRoundTripper, GeneratedValueRoundTripperFactory,
    run_generated_value_fixture,
};
use jsoncompat_codegen::generate_dataclass_models;
use serde_json::Value;
use std::error::Error;
use std::fs;
use std::path::Path;

datatest_stable::harness! {
    { test = fixture, root = "tests/fixtures/fuzz", pattern = ".*\\.json$" },
}

fn fixture(file: &Path) -> Result<(), Box<dyn Error>> {
    run_generated_value_fixture(file, &DataclassGeneratedValueRoundTripperFactory)
}

struct DataclassGeneratedValueRoundTripperFactory;

impl GeneratedValueRoundTripperFactory for DataclassGeneratedValueRoundTripperFactory {
    type RoundTripper = DataclassGeneratedValueRoundTripper;

    fn build_round_tripper(
        &self,
        schema_case: &FuzzSchemaCase<'_>,
    ) -> Result<Option<Self::RoundTripper>, Box<dyn Error>> {
        let source = match generate_dataclass_models(schema_case.schema_json) {
            Ok(source) => source,
            Err(error) => {
                assert_codegen_error_snapshot(schema_case, &error.to_string())?;
                return Ok(None);
            }
        };
        let module_path = write_generated_module(
            "dataclasses-fuzz",
            schema_case.rel_path,
            &schema_case.index.to_string(),
            &source,
        )?;
        Ok(Some(DataclassGeneratedValueRoundTripper::spawn(
            module_path,
        )?))
    }
}

impl GeneratedValueRoundTripper for DataclassGeneratedValueRoundTripper {
    fn round_trip(&mut self, candidate: &Value) -> Result<Value, String> {
        self.round_trip_value(candidate)
    }

    fn reject_invalid(&mut self, candidate: &Value) -> Result<(), String> {
        self.reject_invalid_value(candidate)
    }
}

fn assert_codegen_error_snapshot(
    schema_case: &FuzzSchemaCase<'_>,
    error: &str,
) -> Result<(), Box<dyn Error>> {
    let fixture_relative = Path::new(schema_case.rel_path);
    let snapshot_path = Path::new("tests/fixtures/dataclasses/fuzz")
        .join(fixture_relative.with_extension(""))
        .join(format!("{:03}.error.txt", schema_case.index));
    let expected = fs::read_to_string(&snapshot_path).map_err(|read_error| {
        format!(
            "dataclass fuzz skipped schema #{} in {} without an explicit error snapshot {}: {read_error}",
            schema_case.index,
            schema_case.rel_path,
            snapshot_path.display(),
        )
    })?;
    let actual = format!("{error}\n");
    if expected != actual {
        return Err(format!(
            "dataclass fuzz codegen error snapshot is stale for schema #{} in {}: {}\n\nexpected:\n{}\nactual:\n{}",
            schema_case.index,
            schema_case.rel_path,
            snapshot_path.display(),
            expected,
            actual,
        )
        .into());
    }
    Ok(())
}
