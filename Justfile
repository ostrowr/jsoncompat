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
  cargo test --workspace --all-features --locked
  @echo "[just] checking Python code …"
  env -u VIRTUAL_ENV -u UV_DEFAULT_INDEX -u UV_INDEX -u UV_INDEX_URL -u UV_EXTRA_INDEX_URL uv run --no-config --project pybindings --all-extras --group benchmark --locked --with pyright==1.1.408 pyright
  @echo "[just] checking TypeScript code …"
  pnpm --prefix web/jsoncompatdotcom run ci
  pnpm --prefix web/jsoncompatdotcom run build

regen-dataclasses-fixtures:
  JSONCOMPAT_UPDATE_DATACLASSES_FIXTURES=1 cargo test --test dataclasses_fixtures -- --exact dataclass_snapshots_are_up_to_date_for_all_sample_schemas

bench:
  @echo "[just] running Rust benchmarks …"
  cargo bench --workspace --all-features --bench '*' --locked

bench-check:
  @echo "[just] smoke-checking Rust benchmarks …"
  cargo bench --workspace --all-features --bench '*' --locked -- --test

_build-python-release:
  @echo "[just] building release Python extension for representative timings …"
  if [ "$(uname)" = "Darwin" ]; then \
    cargo rustc --release -p jsoncompat_py --lib -- -C link-arg=-undefined -C link-arg=dynamic_lookup; \
  else \
    cargo build --release -p jsoncompat_py; \
  fi

python-bench iterations="10000" repeats="5": _build-python-release
  @echo "[just] benchmarking generated Python dataclasses against Pydantic v2 …"
  env -u VIRTUAL_ENV -u UV_DEFAULT_INDEX -u UV_INDEX -u UV_INDEX_URL -u UV_EXTRA_INDEX_URL JSONCOMPAT_NATIVE_PROFILE=release uv run --no-config --project pybindings --all-extras --group benchmark --locked python pybindings/bench_dataclasses_runtime.py --iterations {{iterations}} --repeats {{repeats}}

python-bench-scale depth="5" fanout="4" iterations="100" repeats="5": _build-python-release
  @echo "[just] benchmarking a large recursive dataclass graph against Pydantic v2 …"
  env -u VIRTUAL_ENV -u UV_DEFAULT_INDEX -u UV_INDEX -u UV_INDEX_URL -u UV_EXTRA_INDEX_URL JSONCOMPAT_NATIVE_PROFILE=release uv run --no-config --project pybindings --all-extras --group benchmark --locked python pybindings/bench_dataclasses_scaling.py --depth {{depth}} --fanout {{fanout}} --iterations {{iterations}} --repeats {{repeats}}

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
