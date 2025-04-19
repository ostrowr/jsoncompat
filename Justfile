# Developer convenience tasks -------------------------------------------------

# ---- Basic python smoke test ----

python-demo:
  @command -v maturin >/dev/null 2>&1 || (echo "error: maturin not found (install with 'pip install maturin' or 'cargo install maturin --locked')" >&2 && exit 1)
  @echo "[just] building Python extension via maturin …"
  maturin develop -q -m python/Cargo.toml
  @echo "[just] running Python demo …"
  @python examples/python/demo.py

# ---- Basic javascript smoke test ----

wasm-demo:
  @command -v wasm-pack >/dev/null 2>&1 || (echo "error: wasm-pack not found (install with 'cargo install wasm-pack --locked')" >&2 && exit 1)
  @command -v python >/dev/null 2>&1 || (echo "error: python not found" >&2 && exit 1)
  @echo "[just] building wasm package for the Web target …"
  @wasm-pack build wasm --target web --release
  @echo "[just] serving example at http://localhost:8000/examples/wasm/demo.html …"
  @echo "Press Ctrl+C to stop."
  @python -m http.server 8000

default:
  @just --list
