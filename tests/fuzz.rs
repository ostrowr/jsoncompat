//! Fuzzer tests.

#[path = "support/generated_value_harness.rs"]
mod generated_value_harness;

use generated_value_harness::{
    FuzzSchemaCase, GeneratedValueValidator, GeneratedValueValidatorFactory,
    run_generated_value_fixture,
};
use json_schema_ast::{JSONSchema, compile};
use serde_json::Value;
use std::error::Error;
use std::path::Path;

datatest_stable::harness!(fixture, "tests/fixtures/fuzz", ".*\\.json$");

fn fixture(file: &Path) -> Result<(), Box<dyn std::error::Error>> {
    run_generated_value_fixture(file, &CompiledSchemaValidatorFactory)
}

struct CompiledSchemaValidatorFactory;

impl GeneratedValueValidatorFactory for CompiledSchemaValidatorFactory {
    type Validator = CompiledSchemaValidator;

    fn build_validator(
        &self,
        schema_case: &FuzzSchemaCase<'_>,
    ) -> Result<Option<Self::Validator>, Box<dyn Error>> {
        Ok(Some(CompiledSchemaValidator {
            compiled: compile(schema_case.schema_json)?,
        }))
    }
}

struct CompiledSchemaValidator {
    compiled: JSONSchema,
}

impl GeneratedValueValidator for CompiledSchemaValidator {
    fn validate(&mut self, candidate: &Value) -> Result<(), String> {
        if self.compiled.is_valid(candidate) {
            Ok(())
        } else {
            Err("candidate was rejected by the secondary Rust schema compiler".to_owned())
        }
    }
}
