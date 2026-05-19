use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use jsoncompat_codegen::generate_dataclass_models;
use serde_json::Value;
use std::fs;
use std::hint::black_box;
use std::path::PathBuf;
use std::time::Duration;

const FIXTURE_ROOT: &str = "benches/fixtures";

fn bench_dataclass_codegen(c: &mut Criterion) {
    let fixtures = load_bench_fixtures();
    let mut group = c.benchmark_group("dataclasses_codegen");

    for fixture in &fixtures {
        group.bench_with_input(
            BenchmarkId::from_parameter(&fixture.name),
            &fixture.schema,
            |b, schema| {
                b.iter(|| {
                    black_box(generate_dataclass_models(black_box(schema)).unwrap_or_else(
                        |error| panic!("dataclass codegen failed for {}: {error}", fixture.name),
                    ))
                });
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
    targets = bench_dataclass_codegen,
}
criterion_main!(benches);

#[derive(Debug)]
struct BenchFixture {
    name: String,
    schema: Value,
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

    BenchFixture { name, schema }
}
