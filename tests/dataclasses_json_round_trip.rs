#[path = "support/python_env.rs"]
mod python_env;

use jsoncompat_codegen::generate_dataclass_models;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{SystemTime, UNIX_EPOCH};

const FIXTURE_ROOT: &str = "tests/fixtures";
const DATACLASS_SNAPSHOT_ROOT: &str = "tests/fixtures/dataclasses";
const SAMPLE_CACHE: &str = "pybindings/bench_fixture_samples.json";
const DRIVER: &str = include_str!("support/dataclasses_json_round_trip.py");

#[derive(Serialize)]
struct Batch {
    cases: Vec<GeneratedCase>,
}

#[derive(Serialize)]
struct GeneratedCase {
    case_id: String,
    module_path: PathBuf,
    source_path: PathBuf,
    schema_index: Option<usize>,
    expected_schema_digest: Option<String>,
    candidates: Vec<Value>,
    runtime_unsupported: Option<String>,
    unsatisfiable: Option<String>,
}

#[derive(Deserialize)]
struct DriverSummary {
    generated_cases: usize,
    candidates: usize,
    checked_round_trips: usize,
    trusted_round_trips: usize,
    runtime_unsupported: usize,
    unsatisfiable: usize,
}

#[derive(Deserialize)]
struct CachedSample {
    schema_digest: String,
    value: Value,
}

#[derive(Deserialize)]
struct UnsatisfiableFixture {
    reason: String,
    schema_digest: String,
}

struct FixtureCase {
    case_id: String,
    snapshot_base: PathBuf,
    source_path: PathBuf,
    schema_index: Option<usize>,
    schema: Value,
    candidates: Vec<Value>,
}

struct TempDir(PathBuf);

impl TempDir {
    fn create() -> Result<Self, Box<dyn Error>> {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path = std::env::temp_dir().join(format!(
            "jsoncompat-dataclasses-json-round-trip-{}-{unique}",
            std::process::id(),
        ));
        fs::create_dir_all(&path)?;
        Ok(Self(path))
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn every_known_json_schema_fixture_round_trips_through_generated_dataclasses()
-> Result<(), Box<dyn Error>> {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture_cases = collect_fixture_cases(repo_root)?;
    assert!(
        !fixture_cases.is_empty(),
        "fixture corpus must not be empty"
    );

    let temp_dir = TempDir::create()?;
    let mut generated_cases = Vec::new();
    let mut unsupported_count = 0;
    let mut runtime_unsupported = read_runtime_unsupported(repo_root)?;
    let mut unsatisfiable = read_unsatisfiable(repo_root)?;
    let mut sample_cache = read_sample_cache(repo_root)?;
    let mut case_ids = BTreeSet::new();

    for (index, fixture_case) in fixture_cases.iter().enumerate() {
        assert!(
            case_ids.insert(fixture_case.case_id.clone()),
            "duplicate fixture case id {}",
            fixture_case.case_id,
        );

        match generate_dataclass_models(&fixture_case.schema) {
            Ok(source) => {
                assert_generated_snapshot(repo_root, fixture_case)?;
                let runtime_error = runtime_unsupported.remove(&fixture_case.case_id);
                let unsatisfiable_fixture = unsatisfiable.remove(&fixture_case.case_id);
                assert!(
                    runtime_error.is_none() || unsatisfiable_fixture.is_none(),
                    "fixture {} cannot be both runtime-unsupported and unsatisfiable",
                    fixture_case.case_id,
                );
                let mut candidates = fixture_case.candidates.clone();
                let mut expected_schema_digest = None;
                if runtime_error.is_none() && unsatisfiable_fixture.is_none() {
                    let cached = sample_cache.remove(&fixture_case.case_id).ok_or_else(|| {
                        format!(
                            "runtime-supported satisfiable fixture {} has no checked-in sample in {SAMPLE_CACHE}",
                            fixture_case.case_id,
                        )
                    })?;
                    expected_schema_digest = Some(cached.schema_digest);
                    if !candidates.contains(&cached.value) {
                        candidates.push(cached.value);
                    }
                    assert!(
                        !candidates.is_empty(),
                        "runtime-supported satisfiable fixture {} has no round-trip candidate",
                        fixture_case.case_id,
                    );
                } else {
                    assert!(
                        !sample_cache.contains_key(&fixture_case.case_id),
                        "fixture {} is classified as unsupported but still has a checked-in sample",
                        fixture_case.case_id,
                    );
                }
                if let Some(classification) = &unsatisfiable_fixture {
                    assert!(
                        candidates.is_empty(),
                        "unsatisfiable fixture {} declares a valid candidate",
                        fixture_case.case_id,
                    );
                    assert!(
                        !classification.reason.trim().is_empty(),
                        "unsatisfiable fixture {} has an empty reason",
                        fixture_case.case_id,
                    );
                    expected_schema_digest = Some(classification.schema_digest.clone());
                }
                let module_path = temp_dir.0.join(format!("fixture_{index:04}.py"));
                fs::write(&module_path, source)?;
                generated_cases.push(GeneratedCase {
                    case_id: fixture_case.case_id.clone(),
                    module_path,
                    source_path: fixture_case.source_path.clone(),
                    schema_index: fixture_case.schema_index,
                    expected_schema_digest,
                    candidates,
                    runtime_unsupported: runtime_error,
                    unsatisfiable: unsatisfiable_fixture.map(|entry| entry.reason),
                });
            }
            Err(error) => {
                assert!(
                    !runtime_unsupported.contains_key(&fixture_case.case_id),
                    "fixture {} is classified as both codegen-unsupported and runtime-unsupported",
                    fixture_case.case_id,
                );
                assert!(
                    !unsatisfiable.contains_key(&fixture_case.case_id),
                    "fixture {} is classified as both codegen-unsupported and unsatisfiable",
                    fixture_case.case_id,
                );
                assert!(
                    !sample_cache.contains_key(&fixture_case.case_id),
                    "codegen-unsupported fixture {} still has a checked-in runtime sample",
                    fixture_case.case_id,
                );
                unsupported_count += 1;
                assert_codegen_error_snapshot(repo_root, fixture_case, &error.to_string())?;
            }
        }
    }

    assert_eq!(
        generated_cases.len() + unsupported_count,
        fixture_cases.len(),
        "every fixture schema must be generated or explicitly classified as unsupported",
    );
    assert!(
        runtime_unsupported.is_empty(),
        "runtime-unsupported classifications do not identify generated fixture cases: {:?}",
        runtime_unsupported.keys().collect::<Vec<_>>(),
    );
    assert!(
        unsatisfiable.is_empty(),
        "unsatisfiable classifications do not identify generated fixture cases: {:?}",
        unsatisfiable.keys().collect::<Vec<_>>(),
    );
    assert!(
        sample_cache.is_empty(),
        "checked-in samples do not identify runtime-supported satisfiable generated fixtures: {:?}",
        sample_cache.keys().collect::<Vec<_>>(),
    );

    let candidate_count = generated_cases
        .iter()
        .filter(|case| case.runtime_unsupported.is_none() && case.unsatisfiable.is_none())
        .map(|case| case.candidates.len())
        .sum::<usize>();
    let runtime_unsupported_count = generated_cases
        .iter()
        .filter(|case| case.runtime_unsupported.is_some())
        .count();
    let unsatisfiable_count = generated_cases
        .iter()
        .filter(|case| case.unsatisfiable.is_some())
        .count();
    let expected_generated_count = generated_cases.len();
    let mut child = python_env::python_command();
    child
        .args(["-B", "-c", DRIVER])
        .env_remove("PYTHONSAFEPATH")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = child.spawn()?;
    serde_json::to_writer(
        child.stdin.as_mut().expect("piped Python stdin"),
        &Batch {
            cases: generated_cases,
        },
    )?;
    child.stdin.take().expect("piped Python stdin").flush()?;

    let output = child.wait_with_output()?;
    assert!(
        output.status.success(),
        "batched generated-dataclass JSON round-trip failed\n\nstdout:\n{}\n\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let summary: DriverSummary = serde_json::from_slice(&output.stdout).map_err(|error| {
        format!(
            "round-trip driver returned an invalid summary: {error}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        )
    })?;
    assert_eq!(summary.generated_cases, expected_generated_count);
    assert_eq!(summary.candidates, candidate_count);
    assert_eq!(summary.checked_round_trips, candidate_count);
    assert_eq!(summary.trusted_round_trips, candidate_count);
    assert_eq!(summary.runtime_unsupported, runtime_unsupported_count);
    assert_eq!(summary.unsatisfiable, unsatisfiable_count);

    eprintln!(
        "generated {expected_generated_count}/{} fixture schemas; codegen-unsupported: {unsupported_count}; runtime-unsupported: {runtime_unsupported_count}; unsatisfiable: {unsatisfiable_count}; checked and trusted JSON round-trips: {candidate_count} each",
        fixture_cases.len(),
    );
    Ok(())
}

fn read_unsatisfiable(
    repo_root: &Path,
) -> Result<BTreeMap<String, UnsatisfiableFixture>, Box<dyn Error>> {
    Ok(serde_json::from_slice(&fs::read(
        repo_root
            .join(DATACLASS_SNAPSHOT_ROOT)
            .join("unsatisfiable.json"),
    )?)?)
}

fn read_sample_cache(repo_root: &Path) -> Result<BTreeMap<String, CachedSample>, Box<dyn Error>> {
    Ok(serde_json::from_slice(&fs::read(
        repo_root.join(SAMPLE_CACHE),
    )?)?)
}

fn read_runtime_unsupported(repo_root: &Path) -> Result<BTreeMap<String, String>, Box<dyn Error>> {
    let path = repo_root
        .join(DATACLASS_SNAPSHOT_ROOT)
        .join("runtime_unsupported.json");
    read_nonempty_string_manifest(&path, "runtime-unsupported")
}

fn read_nonempty_string_manifest(
    path: &Path,
    name: &str,
) -> Result<BTreeMap<String, String>, Box<dyn Error>> {
    let classifications: BTreeMap<String, String> = serde_json::from_slice(&fs::read(path)?)?;
    assert!(
        classifications
            .values()
            .all(|reason| !reason.trim().is_empty()),
        "{name} classifications in {} must include a non-empty reason",
        path.display(),
    );
    Ok(classifications)
}

fn collect_fixture_cases(repo_root: &Path) -> Result<Vec<FixtureCase>, Box<dyn Error>> {
    let fixture_root = repo_root.join(FIXTURE_ROOT);
    let mut cases = collect_backcompat_cases(&fixture_root.join("backcompat"))?;
    cases.extend(collect_fuzz_cases(&fixture_root.join("fuzz"))?);
    cases.sort_by(|left, right| left.case_id.cmp(&right.case_id));
    Ok(cases)
}

fn collect_backcompat_cases(root: &Path) -> Result<Vec<FixtureCase>, Box<dyn Error>> {
    let mut cases = Vec::new();
    for fixture_dir in sorted_dirs(root)? {
        let case_name = utf8_file_name(&fixture_dir)?;
        let examples = read_optional_json(&fixture_dir.join("examples.json"))?;
        for side in ["old", "new"] {
            let relative_base = Path::new("backcompat").join(&case_name).join(side);
            let source_path = fixture_dir.join(format!("{side}.json"));
            cases.push(FixtureCase {
                case_id: relative_base.to_string_lossy().replace('\\', "/"),
                snapshot_base: relative_base,
                schema: read_json(&source_path)?,
                source_path,
                schema_index: None,
                candidates: backcompat_candidates(examples.as_ref(), side)?,
            });
        }
    }
    Ok(cases)
}

fn backcompat_candidates(
    examples: Option<&Value>,
    side: &str,
) -> Result<Vec<Value>, Box<dyn Error>> {
    let Some(examples) = examples else {
        return Ok(Vec::new());
    };
    let object = examples
        .as_object()
        .ok_or("backcompat examples must be a JSON object")?;
    let mut candidates = Vec::new();
    for key in ["both".to_owned(), format!("{side}_only")] {
        let Some(values) = object.get(&key) else {
            continue;
        };
        let values = values
            .as_array()
            .ok_or_else(|| format!("backcompat examples field {key:?} must be an array"))?;
        candidates.extend(values.iter().cloned());
    }
    Ok(candidates)
}

fn collect_fuzz_cases(root: &Path) -> Result<Vec<FixtureCase>, Box<dyn Error>> {
    let mut cases = Vec::new();
    for fixture_path in sorted_json_files(root)? {
        let relative_path = fixture_path.strip_prefix(root)?;
        let snapshot_dir = Path::new("fuzz").join(relative_path.with_extension(""));
        let root_value = read_json(&fixture_path)?;
        for (index, (schema, candidates)) in
            embedded_fuzz_schemas(&root_value).into_iter().enumerate()
        {
            let snapshot_base = snapshot_dir.join(format!("{index:03}"));
            cases.push(FixtureCase {
                case_id: snapshot_base.to_string_lossy().replace('\\', "/"),
                snapshot_base,
                source_path: fixture_path.clone(),
                schema_index: Some(index),
                schema,
                candidates,
            });
        }
    }
    Ok(cases)
}

fn embedded_fuzz_schemas(root: &Value) -> Vec<(Value, Vec<Value>)> {
    let Value::Array(items) = root else {
        return vec![(root.clone(), Vec::new())];
    };
    items
        .iter()
        .filter_map(|item| {
            let schema = item.get("schema")?.clone();
            let candidates = item
                .get("tests")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter(|test| test.get("valid").and_then(Value::as_bool) == Some(true))
                .filter_map(|test| test.get("data").cloned())
                .collect();
            Some((schema, candidates))
        })
        .collect()
}

fn assert_generated_snapshot(
    repo_root: &Path,
    fixture_case: &FixtureCase,
) -> Result<(), Box<dyn Error>> {
    let python_path = snapshot_path(repo_root, &fixture_case.snapshot_base, "py");
    let error_path = snapshot_path(repo_root, &fixture_case.snapshot_base, "error.txt");
    if !python_path.is_file() || error_path.exists() {
        return Err(format!(
            "generated fixture {} must have exactly one Python snapshot at {}; run `just regen-dataclasses-fixtures`",
            fixture_case.case_id,
            python_path.display(),
        )
        .into());
    }
    Ok(())
}

fn assert_codegen_error_snapshot(
    repo_root: &Path,
    fixture_case: &FixtureCase,
    error: &str,
) -> Result<(), Box<dyn Error>> {
    let error_path = snapshot_path(repo_root, &fixture_case.snapshot_base, "error.txt");
    let python_path = snapshot_path(repo_root, &fixture_case.snapshot_base, "py");
    if python_path.exists() {
        return Err(format!(
            "unsupported fixture {} still has a generated Python snapshot {}; run `just regen-dataclasses-fixtures`",
            fixture_case.case_id,
            python_path.display(),
        )
        .into());
    }
    let expected = fs::read_to_string(&error_path).map_err(|read_error| {
        format!(
            "fixture {} failed code generation without an explicit classification at {}: {read_error}",
            fixture_case.case_id,
            error_path.display(),
        )
    })?;
    let actual = format!("{error}\n");
    if normalized_newlines(&expected) != normalized_newlines(&actual) {
        return Err(format!(
            "codegen error classification is stale for {} at {}\n\nexpected:\n{}\nactual:\n{}",
            fixture_case.case_id,
            error_path.display(),
            expected,
            actual,
        )
        .into());
    }
    Ok(())
}

fn snapshot_path(repo_root: &Path, relative_base: &Path, extension: &str) -> PathBuf {
    repo_root
        .join(DATACLASS_SNAPSHOT_ROOT)
        .join(relative_base)
        .with_extension(extension)
}

fn normalized_newlines(contents: &str) -> String {
    contents.replace("\r\n", "\n")
}

fn read_json(path: &Path) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn read_optional_json(path: &Path) -> Result<Option<Value>, Box<dyn Error>> {
    if path.is_file() {
        Ok(Some(read_json(path)?))
    } else {
        Ok(None)
    }
}

fn sorted_dirs(root: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut paths = fs::read_dir(root)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.retain(|path| path.is_dir());
    paths.sort();
    Ok(paths)
}

fn sorted_json_files(root: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(path) = pending.pop() {
        if path.is_dir() {
            pending.extend(
                fs::read_dir(path)?
                    .map(|entry| entry.map(|entry| entry.path()))
                    .collect::<Result<Vec<_>, _>>()?,
            );
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("json") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn utf8_file_name(path: &Path) -> Result<String, Box<dyn Error>> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .ok_or_else(|| format!("fixture path is not valid UTF-8: {}", path.display()).into())
}
