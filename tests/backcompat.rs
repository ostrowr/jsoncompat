use json_schema_ast::SchemaDocument;
use json_schema_fuzz::{GenerationConfig, ValueGenerator};
use jsoncompat::{Role, check_compat, explain_compat_failure};
use rand::{SeedableRng, rngs::StdRng};
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

// datatest‑stable macro generates one test per fixture directory.
datatest_stable::harness! {
    { test = fixture, root = "tests/fixtures/backcompat", pattern = r".*[/\\]expect\.json$" },
}

#[derive(Deserialize)]
struct Expectation {
    serializer: bool,
    deserializer: bool,
    #[serde(default)]
    expected_serializer_message: Option<String>,
    #[serde(default)]
    expected_deserializer_message: Option<String>,
    #[serde(default)]
    allowed_failure: bool,
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

fn fixture(expect_file: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let dir: PathBuf = expect_file.parent().unwrap().into();
    let old_raw: Value = serde_json::from_slice(&fs::read(dir.join("old.json"))?)?;
    let new_raw: Value = serde_json::from_slice(&fs::read(dir.join("new.json"))?)?;
    let expect: Expectation = serde_json::from_slice(&fs::read(expect_file)?)?;

    // Build ASTs
    let old_schema = SchemaDocument::from_json(&old_raw)?;
    let new_schema = SchemaDocument::from_json(&new_raw)?;

    // Core compat result
    let ser = check_compat(&old_schema, &new_schema, Role::Serializer)?;
    let de = check_compat(&old_schema, &new_schema, Role::Deserializer)?;
    let ser_message = explain_compat_failure(&old_schema, &new_schema, Role::Serializer)?;
    let de_message = explain_compat_failure(&old_schema, &new_schema, Role::Deserializer)?;

    if expect.allowed_failure {
        if ser == expect.serializer && de == expect.deserializer {
            panic!("Previously-failing fixture now passes; remove allowed_failure from {dir:?}");
        }
    } else {
        assert_eq!(ser, expect.serializer, "serializer mismatch in {dir:?}");
        assert_eq!(de, expect.deserializer, "deserializer mismatch in {dir:?}");
    }
    assert_expected_message(
        dir.as_path(),
        "serializer",
        ser,
        expect.expected_serializer_message.as_deref(),
        ser_message.as_deref(),
    );
    assert_expected_message(
        dir.as_path(),
        "deserializer",
        de,
        expect.expected_deserializer_message.as_deref(),
        de_message.as_deref(),
    );

    // Load examples if present
    let ex_path = dir.join("examples.json");
    if ex_path.exists() {
        let samples: SampleSets = serde_json::from_slice(&fs::read(&ex_path)?)?;
        for v in &samples.old_only {
            assert!(
                old_schema.is_valid(v)?,
                "old_only invalid in {dir:?}: {v:?}"
            );
            assert!(
                !new_schema.is_valid(v)?,
                "old_only accepted by NEW in {dir:?}: {v:?}"
            );
        }
        for v in &samples.new_only {
            assert!(
                new_schema.is_valid(v)?,
                "new_only invalid in {dir:?}: {v:?}"
            );
            assert!(
                !old_schema.is_valid(v)?,
                "new_only accepted by OLD in {dir:?}: {v:?}"
            );
        }
        for v in &samples.both {
            assert!(
                old_schema.is_valid(v)? && new_schema.is_valid(v)?,
                "both sample invalid in {dir:?}: {v:?}"
            );
        }
    }

    // Quick fuzz confirmation (10 samples each direction)
    let mut rng = StdRng::seed_from_u64(0xDEADBEEF + dir.to_string_lossy().len() as u64);
    let config = GenerationConfig::new(4);

    for _ in 0..100 {
        let v_new = ValueGenerator::generate(&new_schema, config, &mut rng)?;
        if expect.serializer {
            assert!(
                old_schema.is_valid(&v_new)?,
                "serializer=true but OLD rejects generated value {v_new:?} in {dir:?}"
            );
        }

        let v_old = ValueGenerator::generate(&old_schema, config, &mut rng)?;
        if expect.deserializer {
            assert!(
                new_schema.is_valid(&v_old)?,
                "deserializer=true but NEW rejects generated value {v_old:?} in {dir:?}"
            );
        }
    }

    Ok(())
}

fn assert_expected_message(
    dir: &Path,
    role: &str,
    compatible: bool,
    expected: Option<&str>,
    actual: Option<&str>,
) {
    match (compatible, expected, actual) {
        (true, None, None) => {}
        (true, Some(expected), _) => panic!(
            "compatible {role} fixture {dir:?} must not define a message, found {expected:?}"
        ),
        (true, None, Some(actual)) => {
            panic!("compatible {role} fixture {dir:?} produced an unexpected message {actual:?}")
        }
        (false, Some(expected), Some(actual)) => {
            assert_eq!(actual, expected, "{role} issue message mismatch in {dir:?}")
        }
        (false, None, Some(actual)) => panic!(
            "incompatible {role} fixture {dir:?} must define its expected message; actual {actual:?}"
        ),
        (false, Some(expected), None) => panic!(
            "incompatible {role} fixture {dir:?} expected {expected:?}, but no message was produced"
        ),
        (false, None, None) => panic!(
            "incompatible {role} fixture {dir:?} must define its expected message, but no message was produced"
        ),
    }
}
