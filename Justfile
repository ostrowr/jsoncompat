help:
  @just --list

check:
  @echo "[just] checking Rust code …"
  cargo fmt --all
  cargo clippy --workspace --all-features --all-targets -- -D warnings
  cargo check --workspace --all-features --all-targets --locked
  cargo test --workspace --all-features --all-targets --locked
  @echo "[just] checking Python pydantic fuzz goldens …"
  @cd codegen/tests/golden/pydantic_fuzz && \
    if [ -d .venv ]; then \
      env -u VIRTUAL_ENV UV_CACHE_DIR=.uv_cache UV_NO_MANAGED_PYTHON=1 UV_PYTHON_DOWNLOADS=never UV_OFFLINE=1 \
        uv run --offline --no-managed-python --no-sync pytest -q; \
    else \
      env -u VIRTUAL_ENV UV_CACHE_DIR=.uv_cache uv run pytest -q; \
    fi
  @echo "[just] checking TypeScript code …"
  pnpm --prefix web/jsoncompatdotcom run ci

# ---- Basic python smoke test ----

python-demo:
  uv run --reinstall-package jsoncompat examples/python/basic/demo.py

# ---- Basic javascript smoke test ----

wasm-demo:
  @command -v wasm-pack >/dev/null 2>&1 || (echo "error: wasm-pack not found (install with 'cargo install wasm-pack --locked')" >&2 && exit 1)
  @command -v python >/dev/null 2>&1 || (echo "error: python not found" >&2 && exit 1)
  @echo "[just] building wasm package for the Web target …"
  @wasm-pack build wasm --target web --release
  @echo "[just] serving example at http://localhost:8000/examples/wasm/demo.html …"
  @echo "Press Ctrl+C to stop."
  @python -m http.server 8000

release version="patch":
  @echo "[just] releasing {{version}} (dry run)"
  cargo release {{version}} --workspace
