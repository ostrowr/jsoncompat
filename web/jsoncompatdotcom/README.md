# jsoncompat.com

Source for the public `jsoncompat` website and interactive documentation.

## Run locally

```bash
pnpm install
pnpm start
```

## Validate changes

```bash
pnpm check
pnpm test
pnpm build
```

`pnpm build` also emits the Slidev presentation into `public/deck` so it is served from `/deck/`.

## More detail

- [Public docs](https://jsoncompat.com)
- [Repository README](../../readme.md)
- [Developer guide](../../developing.md) for repository-wide validation and architecture notes


The website builds WASM from the same checkout before Vite runs. On a Node-only
host (including Workers Builds), `scripts/build-wasm.sh` provisions Rust 1.99.0
and wasm-pack 0.15.0 under `target/web-build-tools`; it does not change shell
profiles. Existing matching installations are reused. Cache `target` and the
Rust package caches to speed up subsequent builds. The first build requires
network access to the official Rust installer and crates.io.
