#!/usr/bin/env bash
# Build the website against this checkout, including on Node-only build hosts.
set -euo pipefail
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.." && pwd)
tools_dir="$repo/target/web-build-tools"
export PATH="$tools_dir/bin:$PATH"
rust_version=1.99.0
wasm_pack_version=0.15.0

if ! command -v rustup >/dev/null; then
    # Keep bootstrapped tools inside build output; do not edit shell profiles or
    # replace a developer's existing Rust installation.
    export CARGO_HOME="$tools_dir/cargo"
    export RUSTUP_HOME="$tools_dir/rustup"
    export PATH="$CARGO_HOME/bin:$PATH"
    if ! command -v rustup >/dev/null; then
        mkdir -p "$tools_dir"
        curl --proto '=https' --tlsv1.2 --fail --silent --show-error --location \
            https://sh.rustup.rs --output "$tools_dir/rustup-init.sh"
        sh "$tools_dir/rustup-init.sh" -y --profile minimal \
            --default-toolchain "$rust_version" --no-modify-path
    fi
fi

# Existing matching toolchains need no update or network access for installation.
if [[ $(rustc --version) != "rustc $rust_version "* ]]; then
    rustup toolchain install "$rust_version" --profile minimal
    export RUSTUP_TOOLCHAIN="$rust_version"
fi
rustup target add wasm32-unknown-unknown
if ! command -v wasm-pack >/dev/null || [[ $(wasm-pack --version) != "wasm-pack $wasm_pack_version" ]]; then
    cargo install wasm-pack --version "$wasm_pack_version" --locked --root "$tools_dir"
fi
exec wasm-pack build "$repo/wasm" --release --target web --out-dir pkg-web
