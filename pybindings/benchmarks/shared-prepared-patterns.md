# Shared prepared regex tables

Prefect's generated client fell from **64.54 MiB to 8.94 MiB** of Python source and stubs. Cached full-client import fell from **391 ms to 108 ms**, and peak process RSS during that import fell from **189.75 MiB to 62.09 MiB**. Validation still uses exactly the same precompiled automata.

## What changed

The timezone expression was copied into many independent validation programs. Each copy contained two portable DFA byte orders and was loaded into a separate native automaton. Generation also deep-cloned these automata while projecting model programs.

Generated programs now refer to content identifiers. A private `_patterns/p_<sha256>.py` module stores each distinct compiled expression once, and models import only the patterns they use. The Python module cache shares loaded handles across generation groups; native programs share immutable `Arc` references. Unrelated patterns are not imported with a model. There is no process-wide registry, compression, first-validation compilation, schema JSON storage, or new generation option.

The identifier covers both byte orders and the regex source retained for diagnostics. A program checks its expected identifiers while loading. Corrupt automata, mismatched handles, invalid node/pattern references, and unsupported versions fail at import. Incremental generation retains a shared table until its last consumer disappears.

## Prefect: complete spec, 179 operations

| Measurement | Before | After |
|---|---:|---:|
| Fresh generation, median | 1.383 s | 1.177 s |
| Source and stub bytes | 67,675,921 | 9,370,268 |
| Python files, including stubs | 441 | 444 |
| `operations/deployments.py` | 33,004,817 bytes | 325,825 bytes |
| Largest Python file overall | 33,004,817 bytes | 3,630,818 bytes |
| Full API import from source | 819 ms | 304 ms |
| Full API import from bytecode | 391 ms | 108 ms |
| Source-import peak RSS | 230.69 MiB | 80.61 MiB |
| Bytecode-import peak RSS | 189.75 MiB | 62.09 MiB |
| First timezone-validated JSON decode after bytecode import | 33 µs | 33 µs |

There are two pattern modules and one empty package initializer. The remaining 3.63 MB file contains the single timezone automaton; operation and component files contain references, not copies. All 179 operations and 442 executable Python modules passed the corpus audit, including 43 inline/no-body runtime cases, ordinary/native conversion parity, dispatch-plan equivalence, deterministic generation, and strict `ty` with negative consumers. No HTTP annotation reflection occurred at import. The benchmark separately checks valid and invalid timezone values on first use.

## GitHub: existing 1,223-operation projection

The same documented JSON/text projection as the [GitHub benchmark](github-openapi.md) was used; this does not claim support for GitHub's entire unmodified source document.

| Measurement | Before | After |
|---|---:|---:|
| Fresh generation, median | 2.516 s | 2.527 s |
| Source and stub bytes | 43,962,901 | 43,643,152 |
| Python files, including stubs | 1028 | 1042 |
| Full API source import, sequential runs | 1281 ms | 1436 ms |
| Full API bytecode import, sequential runs | 258 ms | 262 ms |
| Full API source import, alternating before/after processes | 1371 ms | 1355 ms |
| Full API bytecode import, alternating before/after processes | 289 ms | 270 ms |

GitHub uses 13 distinct patterns and gains 14 files. Its automata were comparatively small: storage improves by only 0.7%, and generation and import remain broadly similar. Source imports were noisy under other local activity; the alternating probe does not reproduce the slower sequential result. No broad GitHub speedup is claimed.

All 1,223 operations and 1,040 executable modules passed the same corpus checks, including 781 inline/no-body runtime cases with no native/value differential failures, prepared/reflected dispatch equivalence, and strict `ty`. These are local generated-client checks, not live API calls; uncovered response variants remain uncovered.

## Validation and provenance

- `just check` passed (Rust formatting, clippy, workspace tests, Python typing, site/WASM build).
- Focused tests cover shared native identity, lookarounds, pattern properties, diagnostic equality, corruption rejection, selective imports, handle lifetime, incremental table retention/removal, and standalone/split recursive modules.
- Regenerated and verified dataclass/example/Petstore goldens; their output remained unchanged.
- Five samples per ordinary benchmark case; seven alternating pairs per GitHub import phase. Generation uses fresh directories. Import timing includes the generated API, Python runtime imports, and native initialization. Source imports disable bytecode writes; bytecode is prepared ahead of timing.
- Baseline: `185ae30f081c7ceec4b41aaf6db254083cce7dbe`, built in an independent detached worktree. Generator and native-library hashes, input hashes, all timing samples, and audit summaries are in [the measurement data](shared-prepared-patterns.json). An earlier measurement attempt detected a concurrently replaced baseline binary and was discarded.
- macOS ARM64; Python 3.12.2; release generator/native extension; ty 0.0.86. Measurements are local process timings, not filesystem-cold or universal latency guarantees.

## Reproduce

Build both revisions in independent worktrees, with no shared target-directory symlink:

```bash
cargo build --release --bin jsoncompat
just _build-python-release
```

Download the pinned Prefect source from the [corpus manifest](public-openapi-manifest.json), or reuse its SHA-verified `prefect/source.json`. Prepare GitHub's `client.json` with `bench_github_openapi.py --case client --prepare-only`. Then run from this checkout, using a fresh output directory each time:

```bash
pybindings/.venv/bin/python pybindings/bench_prepared_patterns.py \
  --repo /path/to/baseline --spec /path/to/prefect/source.json --output /tmp/prefect-before
pybindings/.venv/bin/python pybindings/bench_prepared_patterns.py \
  --spec /path/to/prefect/source.json --output /tmp/prefect-after
```

Repeat with GitHub's projected `client.json`. The script checks the first Prefect timezone decode, accepted timezones, and rejection of an invalid timezone after every import. Use `check_public_openapi.py` for the complete operation/module/typing audit.
