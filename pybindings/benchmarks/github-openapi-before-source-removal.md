# GitHub OpenAPI generation benchmark

Profiling and removing repeated work reduces full-client generation from
**5.42 s to 3.95 s** median, about **27% faster**. Peak generation memory falls
from **1.41 GiB to 1.26 GiB**. The package still contains **1,047 files** and
**9,124 model classes**. The approximately one-second target remains unmet.

Fast compression trades some disk space for build speed: output grows from
**63.48 MiB to 67.65 MiB**. Full API loading remains about **2.07 s** from source
and **0.90 s** from bytecode. Strict `ty` checks pass in **1.15 s**. All model
schema text, prepared validator bytes, and generated conversion code are unchanged.

## Where the time goes

The previous release binary was sampled on macOS with `sample` at 1 ms intervals.
Its hot stacks included deep `serde_json`/BTreeMap clones, recursive canonicalization,
deflate compression, and model rendering. Separate opt-in stage timings identify
the phases; final benchmarks below ran without CPU sampling.

| Full-client stage | Before | After |
| --- | ---: | ---: |
| Validate and normalize the OpenAPI document | 1.149 s | 0.777 s |
| Validate and normalize the generated model namespace | 1.468 s | 0.859 s |
| Prepare models and render their Python code | 1.780 s | 1.373 s |
| Stage source files, metadata, and exports | 0.485 s | 0.467 s |
| Contracts, grouping, cache keys, and publication | 0.413 s | 0.361 s |
| Total measured pipeline | 5.295 s | 3.837 s |

These are the primary run's stage timings, not medians. Input loading, process
startup, and final teardown account for the remaining wall time. Filesystem
publication itself takes about a millisecond; writing 1,047 files is not the main
bottleneck. Model preparation still takes **0.489 s** within the render stage,
while schema-source serialization/compression falls from **0.606 s to 0.240 s**.
The largest remaining target is schema processing, at roughly **1.64 s** combined.
No work has been deferred to imports or first use.

Changes made from this profile:

- Move already-normalized trees between rewrite passes, consume canonical results,
  and prepare options in one owned tree. Infer missing required properties in place.
  Sort JSON maps in place instead of rebuilding them, and visit legacy definitions once.
- Cache successful scalar-schema normalization using exact input bytes, only when
  local reference targets cannot make the location significant. Failures are never
  cached. Large top-level namespaces use at most eight bounded worker chunks on
  native targets, with ordered joins preserving the first error. WASM stays sequential.
- Reuse compressed local schema fragments and use deflate level 1 for package
  introspection data. Standalone generated-model compression is unchanged; expanded
  schema text remains exact. This saves around 0.35 s at the cost of 4.18 MiB.
- Index prepared guard patches and JSON keys by owner once, instead of scanning the
  whole conversion plan for each module. Preserve their original order.
- Write hexadecimal byte escapes directly, serialize cache-key inputs by reference,
  and append resource metadata before staging each new resource file once.

The stable resource/component layout, direct dependency imports, recursive class
identity, sixteen schema-data buckets, and shared empty-header aliases remain intact.

## Input and scope

- Official [GitHub REST API description](https://raw.githubusercontent.com/github/rest-api-description/58b1e0c00b39b9e46c24dc4a0ba6a2669c90e9c1/descriptions-next/api.github.com/api.github.com.json), revision
  `58b1e0c00b39b9e46c24dc4a0ba6a2669c90e9c1`.
- SHA-256: `0db1a3dee41ed680628a2a3381a114da7358ac8aa3f7b9276c9198608ca6a1fe`.
- Original: 13,131,466 bytes, 1,232 operations, 973 component schemas,
  816 paths, and 274 inbound webhooks.
- The untouched document still fails validation on two duplicate path-template
  shapes. It is not counted as successfully generated.
- The client projection retains **1,223 operations**. It excludes those two
  operations, seven operations with unsupported media, inbound webhooks, and
  documentation examples. Component constraints and retained operation bodies
  are not simplified.
- The Issues projection contains 61 operations and the dependency closure of
  `basic-error`; the component-only projection clears paths and selects all 973
  schemas. The harness saves every projection and exclusion.

These are release-build measurements on macOS ARM64 with Python 3.12.2 and
`ty 0.0.86 --error all --python-version 3.12`. Final runs were sequential, after
tests and builds finished. Generation uses fresh output directories and ordinary
warm filesystem caches, including publication. Generation medians cover three
runs. Imports use fresh Python processes: source runs disable bytecode, bytecode
runs follow `compileall`. Import timings below measure model/API loading after
the lightweight package root; root-import overhead and process wall time are
recorded separately. Typing times include the launcher.

## Measurements

| Case | Generation median, before → after | Files | Output | Generation peak RSS | Source load | Bytecode load | ty |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 973 components | 3.28 → **2.18 s** | 999 | 28.49 MiB | 0.68 GiB | 1.139 s | 0.272 s | 0.410 s |
| 61-operation Issues client | 1.51 → **1.10 s** | 105 | 3.51 MiB | 0.38 GiB | 0.168 s | 0.080 s | 0.183 s |
| 1,223-operation client | 5.42 → **3.95 s** | 1,047 | 67.65 MiB | 1.26 GiB | 2.070 s | 0.899 s | 1.154 s |

The full client's largest Python file is now `_data/s_6.py`, **4,197,964 bytes**
(4.00 MiB); the largest resource module remains `operations/repos.py`,
**3,718,810 bytes**. The 1,047 files comprise 973 components, one empty-header
module, 45 resources, 16 data buckets, and 12 support/manifest files. Sizes exclude
bytecode caches. All generated files and representative sync/async consumers pass
strict typing; both client negative consumers retain their two expected errors.

The earlier layout change reduced the client from 19,599 files / 86.80 MiB and
12,351 classes to the current 1,047-file layout. This profiling pass retains those
boundaries and all alias names. Baseline snapshots keep earlier typing and import
measurements separate rather than substituting warmed repeats for primary runs.

## Direct imports

| Import | Loaded model classes | Source | Bytecode |
| --- | ---: | ---: | ---: |
| `models.simple_user` | 1 | 0.024 s | 0.016 s |
| Issues-only `operations.issues` | 442 | 0.136 s | 0.046 s |
| Full-client `operations.issues` | 442 | 0.310 s | 0.057 s |

The full-client Issues resource loads all 61 Issues operations and their dependencies,
**442 classes**, without loading unrelated resources. These are selective imports,
not selectively bound clients: importing `GitHubAPI` still prepares the full registry.

## Correctness and checked-in example

Independent AST-only comparisons cover all **12,730 model classes** across the
three packages. Complete reconstructed schema text and all **12,730 prepared
validator programs** are byte-identical, and the verifier compares their reachable
graphs too. **2,094 Python modules/stubs** have identical syntax after excluding
schema-storage imports and compressed-schema assignments. This includes constructors,
HTTP metadata, conversion descriptors, and binding plans. Alias and export maps
also match. No header-name normalization is needed for this pass.

The stronger resource-to-resource verifier checks shared schema fragments once,
without executing generated code or materializing every repeated namespace.
Self-tests reject deliberate schema, program, and conversion-code changes and
pass under ordinary and optimized Python. Earlier layout-migration comparisons
remain supported.

`just check` passed, including all **12 package tests**, **14 local client tests**,
canonicalization fixtures and fuzzing, strict typing, clippy, and the WASM/website
build. An additional traversal regression and final verifier checks passed afterward.
The checked-in [Petstore client](../../examples/openapi_client/petstore/) contains
**40 files / 254 KiB**. Its golden test compares every source, stub, and `py.typed`
file, including additions/removals. Refresh it with `just regen-openapi-client`.
The committed client was also run through aiohttp, requests, and sync/async HTTPX
against the local server, including expected and unexpected errors. The opt-in
public JSONPlaceholder test was not rerun for this generator change.

## Artifacts and reproduction

- [Results at this revision](github-openapi-before-source-removal.json): fingerprints, individual samples,
  repeats, stage profiles, selective imports, typing, and three equivalence reports.
- [Before this profiling pass](github-openapi-before-profiled-generation.json).
- [Before the resource layout](github-openapi-before-resource-layout.json).
- [Recorded equivalence comparisons](github-openapi-before-source-removal.json).

```sh
cargo build --release --bin jsoncompat
cargo rustc --release -p jsoncompat_py --lib -- \
  -C link-arg=-undefined -C link-arg=dynamic_lookup # macOS

JSONCOMPAT_CODEGEN_TIMING=1 \
uv run --no-config --project pybindings --all-extras --group benchmark --locked \
  python pybindings/bench_github_openapi.py \
  --output /tmp/github-profile-run --case client --repeats 3
```

Repeat with `--case models`, `--case issues`, and `--case original`. Use fresh output
directories for generation repeats; `--inspect-only` reuses packages for import and
typing measurements. On other platforms, build the extension with
`cargo build --release -p jsoncompat_py`.

```sh
uv run --no-config --project pybindings --all-extras --locked python \
  pybindings/bench_codegen_equivalence.py \
  --before /tmp/previous/github_client \
  --after /tmp/github-profile-run/github_client \
  --output /tmp/client-equivalence.json
```
