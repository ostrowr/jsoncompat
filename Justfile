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
  cargo clippy --workspace --all-features --all-targets -- -D warnings
  cargo check --workspace --all-features --all-targets --locked
  cargo test --workspace --all-features --all-targets --locked
  @echo "[just] checking Python code …"
  env -u VIRTUAL_ENV uv run --project pybindings --with pyright==1.1.408 pyright
  @echo "[just] checking TypeScript code …"
  pnpm --prefix web/jsoncompatdotcom run ci
  pnpm --prefix web/jsoncompatdotcom run build

regen-dataclasses-fixtures:
  JSONCOMPAT_UPDATE_DATACLASSES_FIXTURES=1 cargo test --test dataclasses_fixtures -- --exact dataclass_snapshots_are_up_to_date_for_all_sample_schemas

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
