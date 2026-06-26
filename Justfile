help:
  @just --list

check:
  @echo "[just] checking Rust code …"
  cargo fmt --all
  @echo "[just] building Python extension …"
  if [ "$(uname)" = "Darwin" ]; then \
    cargo rustc -p jsoncompat_py --lib -- -C link-arg=-undefined -C link-arg=dynamic_lookup; \
  else \
    cargo build -p jsoncompat_py; \
  fi
  cargo clippy --workspace --all-features --all-targets --locked -- -D warnings
  cargo test --workspace --exclude jsoncompat_py --all-features --locked
  @echo "[just] checking Python code …"
  env -u VIRTUAL_ENV -u UV_DEFAULT_INDEX -u UV_INDEX -u UV_INDEX_URL -u UV_EXTRA_INDEX_URL uv run --no-config --project pybindings --all-extras --group benchmark --locked --with pyright==1.1.408 pyright
  @echo "[just] checking TypeScript code …"
  pnpm --prefix web/jsoncompatdotcom run ci
  pnpm --prefix web/jsoncompatdotcom run build

regen-dataclasses-fixtures:
  JSONCOMPAT_UPDATE_DATACLASSES_FIXTURES=1 cargo test --test dataclasses_fixtures -- --exact dataclass_snapshots_are_up_to_date_for_all_sample_schemas

bench:
  @echo "[just] running Rust benchmarks …"
  cargo bench --workspace --exclude jsoncompat_py --all-features --bench '*' --locked

bench-check:
  @echo "[just] smoke-checking Rust benchmarks …"
  cargo bench --workspace --exclude jsoncompat_py --all-features --bench '*' --locked -- --test

python_bench_command := "env -u VIRTUAL_ENV -u UV_DEFAULT_INDEX -u UV_INDEX -u UV_INDEX_URL -u UV_EXTRA_INDEX_URL PYTHONHASHSEED=0 JSONCOMPAT_NATIVE_PROFILE=release uv run --no-config --project pybindings --all-extras --group benchmark --locked python"

_build-python-release:
  @echo "[just] building release Python extension for representative timings …"
  if [ "$(uname)" = "Darwin" ]; then \
    cargo rustc --release -p jsoncompat_py --lib -- -C link-arg=-undefined -C link-arg=dynamic_lookup; \
  else \
    cargo build --release -p jsoncompat_py; \
  fi

[private]
_verify-dataclass-fixtures:
  @echo "[just] verifying checked-in generated dataclass fixtures are current …"
  cargo test --test dataclasses_fixtures -- --exact dataclass_snapshots_are_up_to_date_for_all_sample_schemas

[private]
_python-bench-provenance profile:
  @mkdir -p target/python-benchmark
  @git status -sb > "target/python-benchmark/provenance-{{profile}}.txt"
  @git rev-parse HEAD >> "target/python-benchmark/provenance-{{profile}}.txt"
  @cat "target/python-benchmark/provenance-{{profile}}.txt"

[private]
_python-bench-runtime iterations repeats profile: _verify-dataclass-fixtures
  @echo "[just] benchmarking generated Python dataclasses against Pydantic v2 …"
  @mkdir -p target/python-benchmark
  {{python_bench_command}} pybindings/bench_dataclasses_runtime.py --iterations {{iterations}} --repeats {{repeats}} > "target/python-benchmark/runtime-{{profile}}.txt"
  @cat "target/python-benchmark/runtime-{{profile}}.txt"

[private]
_python-bench-startup repeats profile: _verify-dataclass-fixtures
  @echo "[just] benchmarking cold generated-model startup against Pydantic v2 …"
  @mkdir -p target/python-benchmark
  {{python_bench_command}} pybindings/bench_dataclasses_startup.py --repeats {{repeats}} > "target/python-benchmark/startup-{{profile}}.txt"
  @cat "target/python-benchmark/startup-{{profile}}.txt"

[private]
_python-bench-scale depth fanout iterations repeats profile: _verify-dataclass-fixtures
  @echo "[just] benchmarking a large recursive dataclass graph against Pydantic v2 …"
  @mkdir -p target/python-benchmark
  {{python_bench_command}} pybindings/bench_dataclasses_scaling.py --depth {{depth}} --fanout {{fanout}} --iterations {{iterations}} --repeats {{repeats}} > "target/python-benchmark/scaling-{{profile}}.txt"
  @cat "target/python-benchmark/scaling-{{profile}}.txt"

[private]
_python-bench-scale-profile depth fanout iterations repeats profile: _verify-dataclass-fixtures
  @echo "[just] profiling checked and trusted recursive JSON deserialization …"
  @mkdir -p target/python-benchmark
  {{python_bench_command}} pybindings/bench_dataclasses_scaling.py --depth {{depth}} --fanout {{fanout}} --iterations {{iterations}} --repeats {{repeats}} --profile > "target/python-benchmark/profile-{{profile}}.txt"
  @cat "target/python-benchmark/profile-{{profile}}.txt"

[private]
_python-bench-fixtures iterations repeats profile: _verify-dataclass-fixtures
  @echo "[just] benchmarking fixture JSON -> generated dataclass -> JSON; comparing semantically equivalent Pydantic v2 models …"
  {{python_bench_command}} pybindings/bench_fixture_models.py --iterations {{iterations}} --repeats {{repeats}} --profile "{{profile}}" --reuse-models --results "target/python-fixture-benchmark/results-{{profile}}.json"

[private]
_python-bench-fixtures-limited iterations repeats limit profile: _verify-dataclass-fixtures
  @echo "[just] smoke-benchmarking fixture JSON -> generated dataclass -> JSON for the first {{limit}} schemas …"
  {{python_bench_command}} pybindings/bench_fixture_models.py --iterations {{iterations}} --repeats {{repeats}} --limit {{limit}} --profile "{{profile}}" --reuse-models --results "target/python-fixture-benchmark/results-{{profile}}.json"

# Benchmark the representative small generated-model graph.
python-bench iterations="10000" repeats="5": _build-python-release (_python-bench-provenance "manual") (_python-bench-runtime iterations repeats "manual")

# Benchmark fresh-interpreter import and first-use costs.
python-bench-startup repeats="25": _build-python-release (_python-bench-provenance "startup") (_python-bench-startup repeats "startup")

# Benchmark the recursive graph at a caller-selected size.
python-bench-scale depth="5" fanout="4" iterations="100" repeats="5": _build-python-release (_python-bench-provenance "manual") (_python-bench-scale depth fanout iterations repeats "manual")

# Benchmark JSON -> generated dataclass -> JSON for every supported fixture and compare semantically equivalent Pydantic peers.
python-bench-fixtures iterations="200" repeats="5": _build-python-release (_python-bench-provenance "manual") (_python-bench-fixtures iterations repeats "manual")

# Quickly smoke-test all four Python benchmark surfaces; fixture E2E timings and ratios cover the first 50 schemas.
python-bench-quick: _build-python-release (_python-bench-provenance "quick") (_python-bench-startup "10" "quick") (_python-bench-runtime "2000" "3" "quick") (_python-bench-scale "4" "3" "25" "3" "quick") (_python-bench-fixtures-limited "25" "3" "50" "quick")

# Run the standard repeatable suite, including every fixture E2E round trip and semantically equivalent Pydantic comparison.
python-bench-standard: _build-python-release (_python-bench-provenance "standard") (_python-bench-startup "25" "standard") (_python-bench-runtime "10000" "5" "standard") (_python-bench-scale "5" "4" "100" "5" "standard") (_python-bench-fixtures "200" "5" "standard")

# Run a high-sample suite over every fixture E2E round trip, with Pydantic comparisons, for release performance reports.
python-bench-full: _build-python-release (_python-bench-provenance "full") (_python-bench-startup "100" "full") (_python-bench-runtime "50000" "10" "full") (_python-bench-scale "6" "4" "100" "10" "full") (_python-bench-fixtures "1000" "10" "full")

# Profile checked and trusted deserialization on the standard recursive graph.
python-bench-profile depth="5" fanout="4" iterations="100" repeats="5": _build-python-release (_python-bench-provenance "profile") (_python-bench-scale-profile depth fanout iterations repeats "standard")

# ---- Basic python smoke test ----

python-demo:
  env -u VIRTUAL_ENV uv run --reinstall-package jsoncompat examples/python/basic/demo.py

# ---- Basic javascript smoke test ----

wasm-demo:
  @command -v wasm-pack >/dev/null 2>&1 || (echo "error: wasm-pack not found (install with 'cargo install wasm-pack --locked')" >&2 && exit 1)
  @command -v uv >/dev/null 2>&1 || (echo "error: uv not found" >&2 && exit 1)
  @echo "[just] building wasm package for the Web target …"
  @wasm-pack build wasm --target web --release
  @echo "[just] serving example at http://localhost:8000/examples/wasm/demo.html …"
  @echo "Press Ctrl+C to stop."
  @env -u VIRTUAL_ENV uv run python -m http.server 8000

release version="patch":
  @echo "[just] releasing {{version}} (dry run)"
  cargo release {{version}} --workspace
