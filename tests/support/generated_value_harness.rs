use json_schema_ast::{AstError, SchemaDocument, SchemaError, SchemaNodeKind};
use json_schema_fuzz::{GenerateError, GenerationConfig, ValueGenerator};
use rand::{SeedableRng, rngs::StdRng};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs;
use std::path::Path;

const FUZZ_FIXTURE_ROOT: &str = "tests/fixtures/fuzz";
const GENERATED_VALUE_ITERATIONS: usize = 1000;

pub struct FuzzSchemaCase<'a> {
    #[allow(dead_code)]
    pub rel_path: &'a str,
    #[allow(dead_code)]
    pub index: usize,
    pub schema_json: &'a Value,
}

pub trait GeneratedValueRoundTripper {
    fn round_trip(&mut self, candidate: &Value) -> Result<Value, String>;
}

pub trait GeneratedValueRoundTripperFactory {
    type RoundTripper: GeneratedValueRoundTripper;

    fn build_round_tripper(
        &self,
        schema_case: &FuzzSchemaCase<'_>,
    ) -> Result<Option<Self::RoundTripper>, Box<dyn Error>>;
}

pub fn run_generated_value_fixture<Factory>(
    file: &Path,
    round_tripper_factory: &Factory,
) -> Result<(), Box<dyn Error>>
where
    Factory: GeneratedValueRoundTripperFactory,
{
    let bytes = fs::read(file)?;
    let root: Value = serde_json::from_slice(&bytes)?;
    let schemas = collect_embedded_schemas(&root);

    let rel_path = file.strip_prefix(FUZZ_FIXTURE_ROOT).unwrap_or(file);
    let rel_str = rel_path.to_string_lossy().replace('\\', "/");
    let validate_fixture_tests = rel_str.starts_with("custom/");

    let seed = 0xBADBABE + file.to_string_lossy().len() as u64;
    let mut rng = StdRng::seed_from_u64(seed);

    let whitelist = load_whitelist();
    let allowed = whitelist.get::<str>(rel_str.as_ref());

    for (index, schema_json) in schemas.iter().enumerate() {
        if schema_json == &Value::Bool(false) {
            continue;
        }

        let schema_case = FuzzSchemaCase {
            rel_path: &rel_str,
            index,
            schema_json,
        };
        let schema = match SchemaDocument::from_json(schema_json) {
            Ok(schema) => schema,
            Err(AstError::Schema(SchemaError::UnsupportedSchemaDialect { .. }))
                if !validate_fixture_tests =>
            {
                continue;
            }
            Err(AstError::UnsupportedReference { .. } | AstError::UnresolvedReference { .. })
                if !validate_fixture_tests =>
            {
                continue;
            }
            Err(error) => return Err(error.into()),
        };
        let root = match schema.root() {
            Ok(root) => root,
            Err(AstError::UnsupportedReference { .. } | AstError::UnresolvedReference { .. })
                if !validate_fixture_tests =>
            {
                continue;
            }
            Err(error) => return Err(error.into()),
        };
        if matches!(root.kind(), SchemaNodeKind::BoolSchema(false)) {
            continue;
        }
        let mut generated_round_tripper =
            round_tripper_factory.build_round_tripper(&schema_case)?;
        let is_whitelisted = allowed.map(|set| set.contains(&index)).unwrap_or(false);
        let generation_config = GenerationConfig::new(6);

        let mut success = true;
        for _ in 0..GENERATED_VALUE_ITERATIONS {
            let candidate = match ValueGenerator::generate(&schema, generation_config, &mut rng) {
                Ok(candidate) => candidate,
                Err(GenerateError::ExhaustedAttempts { .. }) => {
                    if !is_whitelisted {
                        panic!(
                            "{}",
                            format!(
                                "Failed to generate a valid instance for schema #{index} in {rel_str}\n\nSchema:\n{}",
                                serde_json::to_string_pretty(schema_json)?,
                            ),
                        );
                    }
                    success = false;
                    break;
                }
                Err(GenerateError::Schema(error)) => return Err(error.into()),
                Err(error) => return Err(error.into()),
            };
            if !schema.is_valid(&candidate)? {
                if !is_whitelisted {
                    panic!(
                        "{}",
                        format_validation_failure(
                            &rel_str,
                            index,
                            schema_json,
                            &candidate,
                            "generator returned a value rejected by the raw schema validator",
                        )?,
                    );
                }
                success = false;
                break;
            }

            if let Some(round_tripper) = generated_round_tripper.as_mut() {
                let emitted = match round_tripper.round_trip(&candidate) {
                    Ok(emitted) => emitted,
                    Err(message) => {
                        panic!(
                            "{}",
                            format_validation_failure(
                                &rel_str,
                                index,
                                schema_json,
                                &candidate,
                                &message,
                            )?,
                        );
                    }
                };
                if !schema.is_valid(&emitted)? {
                    panic!(
                        "{}",
                        format_round_trip_failure(
                            &rel_str,
                            index,
                            schema_json,
                            &candidate,
                            &emitted,
                            "generated dataclass emitted a value rejected by the raw schema validator",
                        )?,
                    );
                }
            }
        }

        match (success, is_whitelisted) {
            (true, false) | (false, true) => {}
            (true, true) => {
                panic!(
                    "Whitelisted failure now passes; please remove entry for schema #{index} in {rel_str}"
                );
            }
            (false, false) => {
                panic!("Should have panicked above, but didn't: schema #{index} in {rel_str}");
            }
        }
    }

    Ok(())
}

fn collect_embedded_schemas(root: &Value) -> Vec<Value> {
    match root {
        Value::Array(items) => items
            .iter()
            .filter_map(|item| item.get("schema").cloned())
            .collect(),
        schema => vec![schema.clone()],
    }
}

fn format_validation_failure(
    rel_path: &str,
    schema_index: usize,
    schema_json: &Value,
    candidate: &Value,
    message: &str,
) -> Result<String, Box<dyn Error>> {
    Ok(format!(
        "Generated validator rejected schema #{schema_index} in {rel_path}\n\n{message}\n\nSchema:\n{}\n\nInstance:\n{}",
        serde_json::to_string_pretty(schema_json)?,
        serde_json::to_string_pretty(candidate)?,
    ))
}

fn format_round_trip_failure(
    rel_path: &str,
    schema_index: usize,
    schema_json: &Value,
    candidate: &Value,
    emitted: &Value,
    message: &str,
) -> Result<String, Box<dyn Error>> {
    Ok(format!(
        "Generated dataclass round-trip rejected schema #{schema_index} in {rel_path}\n\n{message}\n\nSchema:\n{}\n\nInput instance:\n{}\n\nEmitted instance:\n{}",
        serde_json::to_string_pretty(schema_json)?,
        serde_json::to_string_pretty(candidate)?,
        serde_json::to_string_pretty(emitted)?,
    ))
}

fn load_whitelist() -> HashMap<String, HashSet<usize>> {
    let mut map: HashMap<String, HashSet<usize>> = HashMap::new();
    map.insert("anyOf.json".to_string(), [4].iter().cloned().collect());
    map.insert("allOf.json".to_string(), [4, 5].iter().cloned().collect());
    map.insert(
        "oneOf.json".to_string(),
        [2, 4, 5].iter().cloned().collect(),
    );
    map.insert("not.json".to_string(), [4, 5, 8].iter().cloned().collect());
    map.insert(
        "unevaluatedItems.json".to_string(),
        [12, 18].iter().cloned().collect(),
    );
    map.insert(
        "unevaluatedProperties.json".to_string(),
        [12, 15].iter().cloned().collect(),
    );
    map.insert(
        "anchor.json".to_string(),
        [0, 1, 2, 3].iter().cloned().collect(),
    );
    map.insert(
        "optional/anchor.json".to_string(),
        [0].iter().cloned().collect(),
    );
    map.insert(
        "optional/unknownKeyword.json".to_string(),
        [0].iter().cloned().collect(),
    );
    map.insert(
        "optional/id.json".to_string(),
        [0].iter().cloned().collect(),
    );
    map.insert(
        "dynamicRef.json".to_string(),
        [2, 3, 4, 5, 6, 7, 8, 13, 14, 15, 16, 17, 20]
            .iter()
            .cloned()
            .collect(),
    );
    map.insert("optional/dynamicRef.json".to_string(), (1..30).collect());
    map.insert(
        "ref.json".to_string(),
        [6, 10, 11, 17, 19, 27, 28, 29, 30, 31]
            .iter()
            .cloned()
            .collect(),
    );
    map.insert("vocabulary.json".to_string(), [0].iter().cloned().collect());
    map.insert(
        "refRemote.json".to_string(),
        [0, 1, 2, 3, 4, 8, 9, 11, 12, 13, 14]
            .iter()
            .cloned()
            .collect(),
    );
    map.insert(
        "optional/cross-draft.json".to_string(),
        [0].iter().cloned().collect(),
    );
    map.insert("defs.json".to_string(), [0].iter().cloned().collect());
    map
}
