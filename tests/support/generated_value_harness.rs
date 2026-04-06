use json_schema_ast::{AstError, SchemaDocument, SchemaError};
use json_schema_fuzz::{GenerateError, GenerationConfig, ValueGenerator};
use rand::{SeedableRng, rngs::StdRng};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs;
use std::path::Path;

const FUZZ_FIXTURE_ROOT: &str = "tests/fixtures/fuzz";
const GENERATED_VALUE_ITERATIONS: usize = 1000;
const JSON_SCHEMA_DRAFT_2020_12: &str = "https://json-schema.org/draft/2020-12/schema";
const JSON_SCHEMA_DRAFT_2020_12_WITH_FRAGMENT: &str =
    "https://json-schema.org/draft/2020-12/schema#";

pub struct FuzzSchemaCase<'a> {
    #[allow(dead_code)]
    pub rel_path: &'a str,
    #[allow(dead_code)]
    pub index: usize,
    pub schema_json: &'a Value,
}

pub trait GeneratedValueValidator {
    fn validate(&mut self, candidate: &Value) -> Result<(), String>;
}

pub trait GeneratedValueValidatorFactory {
    type Validator: GeneratedValueValidator;

    fn build_validator(
        &self,
        schema_case: &FuzzSchemaCase<'_>,
    ) -> Result<Option<Self::Validator>, Box<dyn Error>>;
}

pub fn run_generated_value_fixture<Factory>(
    file: &Path,
    validator_factory: &Factory,
) -> Result<(), Box<dyn Error>>
where
    Factory: GeneratedValueValidatorFactory,
{
    let bytes = fs::read(file)?;
    let root: Value = serde_json::from_slice(&bytes)?;
    let schemas = collect_embedded_schemas(&root);

    let rel_path = file.strip_prefix(FUZZ_FIXTURE_ROOT).unwrap_or(file);
    let rel_str = rel_path.to_string_lossy().replace('\\', "/");

    let seed = 0xBADBABE + file.to_string_lossy().len() as u64;
    let mut rng = StdRng::seed_from_u64(seed);

    let whitelist = load_whitelist();
    let allowed = whitelist.get::<str>(rel_str.as_ref());

    'schemas: for (index, schema_json) in schemas.iter().enumerate() {
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
            Err(error)
                if !rel_str.starts_with("custom/")
                    && schema_declares_unsupported_schema_uri(schema_json) =>
            {
                assert!(
                    matches!(
                        error,
                        AstError::Schema(
                            SchemaError::UnsupportedSchemaDialect {
                                ref pointer,
                                expected_uri: JSON_SCHEMA_DRAFT_2020_12,
                                ..
                            }
                        ) if pointer == "#/$schema"
                    ),
                    "unexpected unsupported-$schema error for {rel_str} schema #{index}: {error}"
                );
                continue;
            }
            Err(AstError::UnsupportedReference { .. } | AstError::UnresolvedReference { .. })
                if !rel_str.starts_with("custom/") =>
            {
                continue;
            }
            Err(error) => return Err(error.into()),
        };
        let mut generated_validator = validator_factory.build_validator(&schema_case)?;
        let is_whitelisted = allowed.map(|set| set.contains(&index)).unwrap_or(false);

        let mut success = true;
        for _ in 0..GENERATED_VALUE_ITERATIONS {
            let candidate = match ValueGenerator::generate(
                &schema,
                GenerationConfig::new(6),
                &mut rng,
            ) {
                Ok(candidate) => candidate,
                Err(GenerateError::Unsatisfiable | GenerateError::ExhaustedAttempts { .. }) => {
                    if !is_whitelisted {
                        panic!(
                            "Failed to generate a valid instance for schema #{index} in {rel_str}\n\nSchema:\n{}",
                            serde_json::to_string_pretty(schema_json)?,
                        );
                    }
                    success = false;
                    break;
                }
                Err(GenerateError::Schema(
                    AstError::UnsupportedReference { .. } | AstError::UnresolvedReference { .. },
                )) if !rel_str.starts_with("custom/") => {
                    continue 'schemas;
                }
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
                            "fuzzer generated a value rejected by the Rust schema compiler",
                        )?,
                    );
                }
                success = false;
                break;
            }

            if let Some(validator) = generated_validator.as_mut()
                && let Err(message) = validator.validate(&candidate)
            {
                panic!(
                    "{}",
                    format_validation_failure(&rel_str, index, schema_json, &candidate, &message,)?,
                );
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

fn schema_declares_unsupported_schema_uri(schema: &Value) -> bool {
    match schema {
        Value::Object(object) => {
            if let Some(schema_uri) = object.get("$schema")
                && !matches!(
                    schema_uri.as_str(),
                    Some(JSON_SCHEMA_DRAFT_2020_12 | JSON_SCHEMA_DRAFT_2020_12_WITH_FRAGMENT)
                )
            {
                return true;
            }
            object.values().any(schema_declares_unsupported_schema_uri)
        }
        Value::Array(items) => items.iter().any(schema_declares_unsupported_schema_uri),
        _ => false,
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
        "optional/cross-draft.json".to_string(),
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
