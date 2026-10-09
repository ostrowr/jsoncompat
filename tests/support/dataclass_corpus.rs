//! Generate the entire fixture corpus without checking hundreds of duplicate
//! Python implementations into git. Error snapshots are still reviewed.
use jsoncompat::{StampManifest, stamp_schema};
use jsoncompat_codegen::generate_dataclass_models;
use serde_json::Value;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

pub fn generate(destination: &Path) -> BTreeSet<PathBuf> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut output = BTreeSet::new();
    let mut emit = |relative: PathBuf, schema: Value| {
        let (extension, source) = match generate_dataclass_models(&schema) {
            Ok(source) => ("py", source),
            Err(error) => ("error.txt", format!("{error}\n")),
        };
        let path = destination.join(relative.with_extension(extension));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, source).unwrap();
        output.insert(path);
    };
    let backcompat = repo.join("tests/fixtures/backcompat");
    for path in files(&backcompat) {
        if !matches!(
            path.file_name().and_then(|s| s.to_str()),
            Some("old.json" | "new.json")
        ) {
            continue;
        }
        emit(
            Path::new("backcompat").join(path.strip_prefix(&backcompat).unwrap()),
            read(&path),
        );
    }
    let fuzz = repo.join("tests/fixtures/fuzz");
    for path in files(&fuzz)
        .into_iter()
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
    {
        let document = read(&path);
        let schemas = match document {
            Value::Array(items) => items
                .into_iter()
                .filter_map(|mut item| item.get_mut("schema").map(Value::take))
                .collect(),
            schema => vec![schema],
        };
        let relative = Path::new("fuzz")
            .join(path.strip_prefix(&fuzz).unwrap())
            .with_extension("");
        for (index, schema) in schemas.into_iter().enumerate() {
            emit(relative.join(format!("{index:03}")), schema);
        }
    }
    let benchmarks = repo.join("pybindings/benchmark_schemas");
    for path in files(&benchmarks)
        .into_iter()
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
    {
        emit(
            Path::new("benchmarks").join(path.strip_prefix(&benchmarks).unwrap()),
            read(&path),
        );
    }
    let example = repo.join("examples/stamp");
    let first = stamp_schema(
        &StampManifest::empty(),
        "examples/stamp/user-profile",
        read(&example.join("schema-v1.json")),
    )
    .unwrap();
    let second = stamp_schema(
        &first.manifest,
        "examples/stamp/user-profile",
        read(&example.join("schema-v2.json")),
    )
    .unwrap();
    emit(
        "examples/stamp/user-profile-writer".into(),
        second.bundle.writer,
    );
    emit(
        "examples/stamp/user-profile-reader".into(),
        second.bundle.reader,
    );
    output
}
fn read(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn files(root: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            result.extend(files(&path));
        } else {
            result.push(path);
        }
    }
    result.sort();
    result
}
