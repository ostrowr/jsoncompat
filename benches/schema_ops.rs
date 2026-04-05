use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use json_schema_ast::{JSONSchema, SchemaDocument, SchemaNode, compile};
use serde_json::Value;
use std::fs;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::time::Duration;

const FIXTURE_ROOT: &str = "benches/fixtures";

fn bench_compile(c: &mut Criterion) {
    let fixtures = load_bench_fixtures();
    let mut group = c.benchmark_group("compile");

    for fixture in &fixtures {
        group.bench_with_input(
            BenchmarkId::from_parameter(&fixture.name),
            &fixture.schema,
            |b, schema| {
                b.iter(|| {
                    black_box(compile(black_box(schema)).unwrap_or_else(|error| {
                        panic!("compile failed for {}: {error}", fixture.name)
                    }))
                });
            },
        );
    }

    group.finish();
}

fn bench_schema_document_root(c: &mut Criterion) {
    let fixtures = load_bench_fixtures();
    let mut group = c.benchmark_group("SchemaDocument::root");

    for fixture in &fixtures {
        group.bench_with_input(
            BenchmarkId::from_parameter(&fixture.name),
            &fixture.schema,
            |b, schema| {
                b.iter(|| {
                    let document =
                        SchemaDocument::from_json(black_box(schema)).unwrap_or_else(|error| {
                            panic!(
                                "SchemaDocument::from_json failed for {}: {error}",
                                fixture.name
                            )
                        });
                    let root = document
                        .root()
                        .unwrap_or_else(|error| {
                            panic!("SchemaDocument::root failed for {}: {error}", fixture.name)
                        })
                        .clone();
                    black_box(root)
                });
            },
        );
    }

    group.finish();
}

fn bench_validate(c: &mut Criterion) {
    let fixtures = load_bench_fixtures();
    let validators = fixtures
        .iter()
        .map(|fixture| ValidationFixture {
            name: fixture.name.clone(),
            validator: compile(&fixture.schema)
                .unwrap_or_else(|error| panic!("compile failed for {}: {error}", fixture.name)),
            instance: fixture.instance.clone(),
        })
        .collect::<Vec<_>>();
    let mut group = c.benchmark_group("validate");

    for fixture in &validators {
        group.bench_with_input(
            BenchmarkId::from_parameter(&fixture.name),
            fixture,
            |b, fixture| {
                b.iter(|| black_box(fixture.validator.is_valid(black_box(&fixture.instance))));
            },
        );
    }

    group.finish();
}

fn criterion_config() -> Criterion {
    Criterion::default()
        .sample_size(20)
        .warm_up_time(Duration::from_millis(100))
        .measurement_time(Duration::from_millis(300))
}

criterion_group! {
    name = benches;
    config = criterion_config();
    targets = bench_compile, bench_schema_document_root, bench_validate,
}
criterion_main!(benches);

#[derive(Debug)]
struct BenchFixture {
    name: String,
    schema: Value,
    instance: Value,
}

struct ValidationFixture {
    name: String,
    validator: JSONSchema,
    instance: Value,
}

fn load_bench_fixtures() -> Vec<BenchFixture> {
    let mut fixture_paths = fs::read_dir(FIXTURE_ROOT)
        .unwrap_or_else(|error| panic!("failed to list {FIXTURE_ROOT}: {error}"))
        .map(|entry| {
            entry
                .unwrap_or_else(|error| panic!("failed to read {FIXTURE_ROOT} entry: {error}"))
                .path()
        })
        .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    fixture_paths.sort();

    fixture_paths.into_iter().map(load_bench_fixture).collect()
}

fn load_bench_fixture(path: PathBuf) -> BenchFixture {
    let bytes = fs::read(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    let root: Value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()));
    let Some(name) = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(str::to_owned)
    else {
        panic!(
            "benchmark fixture path has no UTF-8 file stem: {}",
            path.display()
        );
    };
    let Some(schema) = root.get("schema").cloned() else {
        panic!("benchmark fixture {name} is missing a schema field");
    };
    let Some(instance) = root.get("instance").cloned() else {
        panic!("benchmark fixture {name} is missing an instance field");
    };

    assert_fixture_is_valid(&name, &schema, &instance, &path);

    BenchFixture {
        name,
        schema,
        instance,
    }
}

fn assert_fixture_is_valid(name: &str, schema: &Value, instance: &Value, path: &Path) {
    let document = SchemaDocument::from_json(schema).unwrap_or_else(|error| {
        panic!(
            "benchmark fixture {name} failed schema construction from {}: {error}",
            path.display()
        )
    });
    let ast: SchemaNode = document
        .root()
        .unwrap_or_else(|error| {
            panic!(
                "benchmark fixture {name} failed AST resolution from {}: {error}",
                path.display()
            )
        })
        .clone();
    let validator = compile(schema).unwrap_or_else(|error| {
        panic!(
            "benchmark fixture {name} failed validator compilation from {}: {error}",
            path.display()
        )
    });
    assert!(
        validator.is_valid(instance),
        "benchmark fixture {name} instance is invalid under {}",
        path.display()
    );
    black_box(ast);
}
