use jsoncompat::{StampManifest, stamp_schema};
use jsoncompat_codegen::generate_dataclass_models;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[path = "support/python_env.rs"]
mod python_env;

const UPDATE_ENV: &str = "JSONCOMPAT_UPDATE_DATACLASSES_FIXTURES";
const SNAPSHOT_ROOT: &str = "tests/fixtures/dataclasses";

#[derive(Debug, Clone)]
enum SnapshotKind {
    Python,
    Error,
}

#[derive(Debug, Clone)]
struct Snapshot {
    kind: SnapshotKind,
    contents: String,
}

#[test]
fn dataclass_snapshots_are_up_to_date_for_all_sample_schemas() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let update = std::env::var_os(UPDATE_ENV).is_some();

    let mut expected_paths = BTreeSet::new();

    snapshot_backcompat_fixtures(repo_root, update, &mut expected_paths);
    snapshot_fuzz_fixtures(repo_root, update, &mut expected_paths);
    snapshot_stamp_example(repo_root, update, &mut expected_paths);

    prune_or_validate_stale_snapshots(repo_root, update, &expected_paths);
}

fn snapshot_backcompat_fixtures(
    repo_root: &Path,
    update: bool,
    expected_paths: &mut BTreeSet<PathBuf>,
) {
    let fixture_root = repo_root.join("tests/fixtures/backcompat");
    for fixture_dir in sorted_dirs(&fixture_root) {
        let case_name = fixture_dir
            .file_name()
            .and_then(|name| name.to_str())
            .expect("utf-8 fixture directory name");
        assert_snapshot(
            repo_root,
            Path::new("backcompat").join(case_name).join("old"),
            render_schema_snapshot(&read_json(fixture_dir.join("old.json"))),
            update,
            expected_paths,
        );
        assert_snapshot(
            repo_root,
            Path::new("backcompat").join(case_name).join("new"),
            render_schema_snapshot(&read_json(fixture_dir.join("new.json"))),
            update,
            expected_paths,
        );
    }
}

fn snapshot_fuzz_fixtures(repo_root: &Path, update: bool, expected_paths: &mut BTreeSet<PathBuf>) {
    let fixture_root = repo_root.join("tests/fixtures/fuzz");
    for schema_file in sorted_json_files(&fixture_root) {
        let relative = schema_file
            .strip_prefix(&fixture_root)
            .expect("fixture path is under fuzz root");
        let schema_doc = read_json(&schema_file);
        let schemas = collect_embedded_schemas(&schema_doc);

        let snapshot_dir = Path::new("fuzz").join(relative).with_extension("");
        for (index, schema) in schemas.into_iter().enumerate() {
            assert_snapshot(
                repo_root,
                snapshot_dir.join(format!("{index:03}")),
                render_schema_snapshot(&schema),
                update,
                expected_paths,
            );
        }
    }
}

fn snapshot_stamp_example(repo_root: &Path, update: bool, expected_paths: &mut BTreeSet<PathBuf>) {
    let example_root = repo_root.join("examples/stamp");
    let schema_v1 = read_json(example_root.join("schema-v1.json"));
    let schema_v2 = read_json(example_root.join("schema-v2.json"));
    let result = stamp_schema(
        &StampManifest::empty(),
        "examples/stamp/user-profile",
        schema_v1,
    )
    .and_then(|first| stamp_schema(&first.manifest, "examples/stamp/user-profile", schema_v2))
    .expect("stamp example schemas");
    assert_snapshot(
        repo_root,
        Path::new("examples/stamp/user-profile-writer").to_path_buf(),
        render_schema_snapshot(&result.bundle.writer),
        update,
        expected_paths,
    );
    assert_snapshot(
        repo_root,
        Path::new("examples/stamp/user-profile-reader").to_path_buf(),
        render_schema_snapshot(&result.bundle.reader),
        update,
        expected_paths,
    );
}

fn render_schema_snapshot(schema: &Value) -> Snapshot {
    match generate_dataclass_models(schema) {
        Ok(source) => Snapshot {
            kind: SnapshotKind::Python,
            contents: source,
        },
        Err(error) => Snapshot {
            kind: SnapshotKind::Error,
            contents: format!("{error}\n"),
        },
    }
}

fn assert_snapshot(
    repo_root: &Path,
    relative_base: PathBuf,
    snapshot: Snapshot,
    update: bool,
    expected_paths: &mut BTreeSet<PathBuf>,
) {
    let snapshot_path = repo_root
        .join(SNAPSHOT_ROOT)
        .join(&relative_base)
        .with_extension(snapshot.extension());
    let stale_path = repo_root
        .join(SNAPSHOT_ROOT)
        .join(&relative_base)
        .with_extension(snapshot.stale_extension());
    expected_paths.insert(snapshot_path.clone());

    if update {
        if let Some(parent) = snapshot_path.parent() {
            fs::create_dir_all(parent).expect("create snapshot directory");
        }
        fs::write(&snapshot_path, snapshot.contents.as_bytes()).expect("write snapshot fixture");
        if stale_path.exists() {
            fs::remove_file(&stale_path).expect("remove stale snapshot fixture");
        }
    }

    let current = fs::read_to_string(&snapshot_path).unwrap_or_else(|error| {
        panic!(
            "missing dataclass snapshot {}: {error}. Run `just regen-dataclasses-fixtures`.",
            snapshot_path.display()
        )
    });
    assert_eq!(
        current,
        snapshot.contents,
        "dataclass snapshot is stale: {}. Run `just regen-dataclasses-fixtures`.",
        snapshot_path.display()
    );

    if matches!(snapshot.kind, SnapshotKind::Python) {
        assert_python_syntax(&snapshot_path);
    }
}

impl Snapshot {
    fn extension(&self) -> &'static str {
        match self.kind {
            SnapshotKind::Python => "py",
            SnapshotKind::Error => "error.txt",
        }
    }

    fn stale_extension(&self) -> &'static str {
        match self.kind {
            SnapshotKind::Python => "error.txt",
            SnapshotKind::Error => "py",
        }
    }
}

fn prune_or_validate_stale_snapshots(
    repo_root: &Path,
    update: bool,
    expected_paths: &BTreeSet<PathBuf>,
) {
    let snapshot_root = repo_root.join(SNAPSHOT_ROOT);
    if !snapshot_root.exists() {
        if update {
            fs::create_dir_all(&snapshot_root).expect("create snapshot root");
            return;
        }
        panic!(
            "missing dataclass snapshot root {}. Run `just regen-dataclasses-fixtures`.",
            snapshot_root.display()
        );
    }

    for file in sorted_files_recursive(&snapshot_root) {
        let ext = file.extension().and_then(|ext| ext.to_str());
        let file_name = file
            .file_name()
            .and_then(|name| name.to_str())
            .expect("utf-8 snapshot filename");
        let is_snapshot = matches!(ext, Some("py")) || file_name.ends_with(".error.txt");
        if !is_snapshot || expected_paths.contains(&file) {
            continue;
        }

        if update {
            fs::remove_file(&file).expect("remove stale generated snapshot");
        } else {
            panic!(
                "stale dataclass snapshot {}. Run `just regen-dataclasses-fixtures`.",
                file.display()
            );
        }
    }
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

fn sorted_dirs(root: &Path) -> Vec<PathBuf> {
    let mut dirs = fs::read_dir(root)
        .unwrap_or_else(|error| panic!("read dir {}: {error}", root.display()))
        .map(|entry| entry.expect("read dir entry").path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    dirs.sort();
    dirs
}

fn sorted_json_files(root: &Path) -> Vec<PathBuf> {
    sorted_files_recursive(root)
        .into_iter()
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect()
}

fn sorted_files_recursive(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(path) = stack.pop() {
        if path.is_dir() {
            for entry in fs::read_dir(&path)
                .unwrap_or_else(|error| panic!("read dir {}: {error}", path.display()))
            {
                stack.push(entry.expect("read dir entry").path());
            }
        } else {
            files.push(path);
        }
    }
    files.sort();
    files
}

fn read_json(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(
        &fs::read(path.as_ref())
            .unwrap_or_else(|error| panic!("read json {}: {error}", path.as_ref().display())),
    )
    .unwrap_or_else(|error| panic!("parse json {}: {error}", path.as_ref().display()))
}

fn assert_python_syntax(path: &Path) {
    let status = python_env::python_command()
        .arg("-B")
        .arg("-c")
        .arg(
            "import ast, pathlib, sys; ast.parse(pathlib.Path(sys.argv[1]).read_text(encoding='utf-8'), filename=sys.argv[1])",
        )
        .arg(path)
        .status()
        .expect("run python syntax check");
    assert!(
        status.success(),
        "generated dataclass fixture {} did not compile",
        path.display()
    );
}
