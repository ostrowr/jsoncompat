use json_schema_draft2020::compile;
use json_schema_fuzz::generate_value;
use jsoncompat::{build_and_resolve_schema, check_compat, Role};
use rand::{rngs::StdRng, SeedableRng};
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

// datatest‑stable macro generates one test per fixture directory.
datatest_stable::harness!(
    fixture,
    "tests/fixtures/backcompat",
    r".*[/\\]expect\.json$"
);

#[derive(Deserialize)]
struct Expectation {
    serializer: bool,
    deserializer: bool,
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
    let old_ast = build_and_resolve_schema(&old_raw)?;
    let new_ast = build_and_resolve_schema(&new_raw)?;

    // Core compat result
    let ser = check_compat(&old_ast, &new_ast, Role::Serializer);
    let de = check_compat(&old_ast, &new_ast, Role::Deserializer);

    if expect.allowed_failure {
        if ser == expect.serializer && de == expect.deserializer {
            panic!(
                "Previously-failing fixture now passes; remove allowed_failure from {:?}",
                dir
            );
        }
    } else {
        assert_eq!(ser, expect.serializer, "serializer mismatch in {:?}", dir);
        assert_eq!(
            de, expect.deserializer,
            "deserializer mismatch in {:?}",
            dir
        );
    }

    // Load examples if present
    let ex_path = dir.join("examples.json");
    if ex_path.exists() {
        let samples: SampleSets = serde_json::from_slice(&fs::read(&ex_path)?)?;
        let compiled_old = compile(&old_raw)?;
        let compiled_new = compile(&new_raw)?;

        for v in &samples.old_only {
            assert!(
                compiled_old.is_valid(v),
                "old_only invalid in {:?}: {:?}",
                dir,
                v
            );
            assert!(
                !compiled_new.is_valid(v),
                "old_only accepted by NEW in {:?}: {:?}",
                dir,
                v
            );
        }
        for v in &samples.new_only {
            assert!(
                compiled_new.is_valid(v),
                "new_only invalid in {:?}: {:?}",
                dir,
                v
            );
            assert!(
                !compiled_old.is_valid(v),
                "new_only accepted by OLD in {:?}: {:?}",
                dir,
                v
            );
        }
        for v in &samples.both {
            assert!(
                compiled_old.is_valid(v) && compiled_new.is_valid(v),
                "both sample invalid in {:?}: {:?}",
                dir,
                v
            );
        }
    }

    // Quick fuzz confirmation (10 samples each direction)
    let compiled_old = compile(&old_raw)?;
    let compiled_new = compile(&new_raw)?;
    let mut rng = StdRng::seed_from_u64(0xDEADBEEF + dir.to_string_lossy().len() as u64);

    for _ in 0..100 {
        let v_new = generate_value(&new_ast, &mut rng, 4);
        if expect.serializer {
            assert!(
                compiled_old.is_valid(&v_new),
                "serializer=true but OLD rejects generated value {:?} in {:?}",
                v_new,
                dir
            );
        }

        let v_old = generate_value(&old_ast, &mut rng, 4);
        if expect.deserializer {
            assert!(
                compiled_new.is_valid(&v_old),
                "deserializer=true but NEW rejects generated value {:?} in {:?}",
                v_old,
                dir
            );
        }
    }

    Ok(())
}
