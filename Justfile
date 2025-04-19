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

# -----------------------------------------------------------------------------
# Release artefact build recipes
# -----------------------------------------------------------------------------

_dist_dir := "dist"

# -----------------------------------------------------------------------------
# CLI binary ("jsoncompat") ----------------------------------------------------
# -----------------------------------------------------------------------------

# Build the CLI for every target listed in `_cli-targets` and copy the final
# binaries into `dist/cli/<target>/` so they can be uploaded straight to a
# GitHub release or similar.
cli-release:
 @set -euo pipefail; \
 echo "[just] building CLI binary (host platform) …"; \
 cargo build --locked --release --bin jsoncompat; \
 host_triple=$(rustc -vV | awk '/^host:/ {print $2}'); \
 out_dir="{{_dist_dir}}/cli/{{host_triple}}"; \
 mkdir -p "{{out_dir}}"; \
 bin_name="jsoncompat"; \
 case "{{host_triple}}" in *windows-*) bin_name="{{bin_name}}.exe" ;; esac; \
 cp "target/{{host_triple}}/release/{{bin_name}}" "{{out_dir}}/" || cp "target/release/{{bin_name}}" "{{out_dir}}/"

# -----------------------------------------------------------------------------
# Python wheels ----------------------------------------------------------------
# -----------------------------------------------------------------------------

# Build binary wheels for the Python bindings.  By default this uses maturin to
# build a wheel for the *current* platform.  If you need manylinux or
# cross‑compiled wheels, execute the recipe inside the appropriate build
# environment (e.g. via cibuildwheel or the manylinux docker images).
python-release:
 @command -v maturin >/dev/null 2>&1 || (echo "error: maturin not found (install with 'pip install maturin' or 'cargo install maturin --locked')" >&2 && exit 1)
 @set -euo pipefail; \
 echo "[just] building Python wheels …"; \
 mkdir -p "{{_dist_dir}}/python"; \
 maturin build -m python/Cargo.toml --release --strip --out "{{_dist_dir}}/python"

# -----------------------------------------------------------------------------
# JavaScript / WebAssembly package --------------------------------------------
# -----------------------------------------------------------------------------

# Build the WebAssembly bindings for all three wasm‑pack targets (bundler,
# nodejs and web).  Each build is written to `dist/js/<target>/`.
js-release:
 @command -v wasm-pack >/dev/null 2>&1 || (echo "error: wasm-pack not found (install with 'cargo install wasm-pack --locked')" >&2 && exit 1)
 @set -euo pipefail; \
 echo "[just] building wasm packages …"; \
 for target in bundler nodejs web; do \
  echo "  ⇒ $$target"; \
  wasm-pack build wasm --release --target "$$target" --out-dir "pkg-$$target"; \
  out_dir="{{_dist_dir}}/js/$$target"; \
  rm -rf "$$out_dir"; \
  mkdir -p "$$out_dir"; \
  cp -r "pkg-$$target"/* "$$out_dir/"; \
  rm -rf "pkg-$$target"; \
 done

# -----------------------------------------------------------------------------
# Composite release recipe -----------------------------------------------------
# -----------------------------------------------------------------------------

# Build everything (CLI binaries, Python wheels, JavaScript/WebAssembly
# packages).  The resulting artefacts live under the `dist/` directory.
release: cli-release python-release js-release
 @echo "[just] Release complete – artefacts are in the 'dist' directory."

