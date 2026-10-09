#[path = "support/dataclass_corpus.rs"]
mod corpus;
#[path = "support/python_env.rs"]
mod python_env;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::Stdio,
};

#[test]
fn dataclass_snapshots_are_up_to_date_for_all_sample_schemas() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"));
    let snapshots = repo.join("tests/fixtures/dataclasses");
    let destination = std::env::temp_dir().join(format!(
        "jsoncompat-corpus-{}-{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    let update = std::env::var_os("JSONCOMPAT_UPDATE_DATACLASSES_FIXTURES").is_some();
    let paths = corpus::generate(&destination);
    let mut expected = BTreeSet::new();
    for path in &paths {
        let relative = path.strip_prefix(&destination).unwrap();
        // Keep readable representative model graphs and stamped roles. All
        // other models still receive syntax, runtime and differential checks.
        let golden = relative.starts_with("benchmarks")
            || relative.starts_with("examples")
            || path.extension().is_some_and(|ext| ext == "txt");
        if !golden {
            continue;
        }
        let snapshot = snapshots.join(relative);
        let source = fs::read_to_string(path).unwrap();
        if update {
            fs::create_dir_all(snapshot.parent().unwrap()).unwrap();
            fs::write(&snapshot, &source).unwrap();
        }
        assert_eq!(
            fs::read_to_string(&snapshot)
                .unwrap_or_else(|error| panic!(
                    "{}: {error}; run just regen-dataclasses-fixtures",
                    snapshot.display()
                ))
                .replace("\r\n", "\n"),
            source.replace("\r\n", "\n"),
            "stale golden {}; run just regen-dataclasses-fixtures",
            snapshot.display()
        );
        expected.insert(snapshot);
    }
    let mut pending = vec![snapshots];
    while let Some(path) = pending.pop() {
        if path.is_dir() {
            pending.extend(
                fs::read_dir(path)
                    .unwrap()
                    .map(|entry| entry.unwrap().path()),
            );
        } else if path
            .extension()
            .is_some_and(|ext| ext == "py" || ext == "txt")
            && !expected.contains(&path)
        {
            if update {
                fs::remove_file(path).unwrap();
            } else {
                panic!(
                    "stale golden {}; run just regen-dataclasses-fixtures",
                    path.display()
                );
            }
        }
    }
    let python_paths: Vec<&PathBuf> = paths
        .iter()
        .filter(|path| path.extension().is_some_and(|ext| ext == "py"))
        .collect();
    assert!(
        python_paths.len() > 500,
        "full corpus must still be generated"
    );
    let mut child = python_env::python_command().args(["-B", "-c", "import ast,json,pathlib,sys\nfor p in json.load(sys.stdin):\n ast.parse(pathlib.Path(p).read_text(encoding='utf-8'), filename=p)"]).stdin(Stdio::piped()).spawn().unwrap();
    serde_json::to_writer(child.stdin.take().unwrap(), &python_paths).unwrap();
    assert!(child.wait().unwrap().success());
    fs::remove_dir_all(destination).unwrap();
}
