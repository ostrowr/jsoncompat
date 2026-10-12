# GitHub OpenAPI scale experiment

Combining each declaration with its initializer removes **12,351 files** from the
full client: **31,950 → 19,599**. Reusing component-validation proofs and avoiding
repeated canonicalization reduces its three-run generation median from **9.28 to
7.48 seconds**. The models and Issues medians are **3.73 and 1.64 seconds**.
The approximately one-second target is still not met.

All model identities, public imports, schemas, and prepared validation programs
are preserved. No schema compilation moves to import or first use. The direct
import and model-count investigation below describes proposed follow-up work;
barrel removal and model deduplication are not implemented in these measurements.

## Input and scope

The input is GitHub's official
[OpenAPI 3.1 description](https://github.com/github/rest-api-description/blob/58b1e0c00b39b9e46c24dc4a0ba6a2669c90e9c1/descriptions-next/api.github.com/api.github.com.json),
pinned at `58b1e0c00b39b9e46c24dc4a0ba6a2669c90e9c1`.
The 13,131,466-byte document contains **1,232 operations, 973 component schemas,
816 paths, and 274 webhooks**. Its SHA-256 is
`0db1a3dee41ed680628a2a3381a114da7358ac8aa3f7b9276c9198608ca6a1fe`.

The projections are unchanged from the preceding benchmark:

- **Original:** fails validation because organization and user attestation paths
  have duplicate template shapes with different final parameter names
  (`attestation_id` and `subject_digest`).
- **Models:** sets `paths` to `{}` and generates all 973 unchanged component
  schemas, including nested classes. Other collections, including webhooks,
  remain in the input and are validated. This is a model package.
- **Issues:** selects all 61 `issues/` operations and their transitive model
  dependencies with `--component basic-error`.
- **JSON client projection:** retains 1,223 operations and every component schema.
  It omits two duplicate-shape operations and seven operations with unsupported
  media, plus the 274 inbound webhooks and 533 documentation examples in
  `components.examples`. Those collections are unsupported by the outbound
  operation lowerer. Declared schema constraints and response bodies remain
  intact.

Each projected document and its itemized exclusions are saved by the harness.
Omitted operations are not counted as supported.

## Measurements

Cold generation includes every output file and the manifest. These comparisons
use the preceding optimized build, with the same inputs and release configuration.

| Case | Previous median | Current median | Files, previous → current |
| --- | ---: | ---: | ---: |
| All 973 component schemas | 4.85 s | 3.73 s | 7,228 → 4,095 |
| 61-operation Issues client | 2.23 s | 1.64 s | 1,783 → 1,119 |
| 1,223-operation client projection | 9.28 s | 7.48 s | 31,950 → 19,599 |

The primary runs took **3.87 / 1.65 / 7.82 seconds**. Two further generations
into fresh directories give the medians above, with ranges **3.71–3.87 /
1.63–1.65 / 7.29–7.82 seconds**. Filesystem caches were not flushed.

For historical context, before the earlier compiler optimizations the models
and Issues cases took 436.95 and 23.23 seconds, and the full client timed out at
900 seconds. Before schema-source sharing, the model package occupied 622.61 MiB.

| Current artifacts and imports | Models | Issues client | Client projection |
| --- | ---: | ---: | ---: |
| Generated classes | 3,133 | 664 | 12,351 |
| Binding groups | 4 | 5 | 34 |
| Total output | 28.04 MiB | 4.41 MiB | 86.80 MiB |
| Generation peak RSS | 0.71 GiB | 0.38 GiB | 0.95 GiB |
| Largest Python file | 778.4 KiB | 121.6 KiB | 2,974.6 KiB |
| 95th percentile Python file | 24.4 KiB | 10.3 KiB | 9.6 KiB |
| Import all models / prepared API, source | 1.437 s | 0.287 s | 5.548 s |
| Import all models / prepared API, bytecode | 0.451 s | 0.143 s | 2.407 s |
| Lazy package import, bytecode | 2.87 ms | 2.74 ms | 10.98 ms |
| Import peak RSS, source / bytecode | 165.2 / 136.2 MiB | 46.0 / 39.5 MiB | 397.6 / 330.0 MiB |

The largest full-client Python file is the package `__init__.py`, at 2.90 MiB.
The file merge reduces module overhead, but preserves the payloads and model
population; total full-client output falls only from 89.04 to 86.80 MiB.
Full-client source import is 5.548 seconds versus the previous 5.432 seconds;
bytecode import improves from 2.696 to 2.407 seconds. Fewer files alone do not
remove the cost of constructing and binding thousands of classes.

**ty 0.0.86**, targeting Python 3.12 with every rule treated as an error, checks
every generated Python file and a typed consumer:

| Primary ty check | Models | Issues client | Client projection |
| --- | ---: | ---: | ---: |
| Wall time | 0.667 s | 0.188 s | 5.714 s |
| Peak RSS | 0.47 GiB | 0.11 GiB | 1.27 GiB |
| Diagnostics | 0 | 0 | 0 |

The first full-client typing run took 5.714 seconds. To check the apparent
regression, three subsequent old/new runs were interleaved on the same machine.
Those medians were **2.568 seconds before and 2.236 seconds after**, all with zero
diagnostics. Both sets are retained in the snapshot; warmed repeats are not a
replacement for the primary measurement. Paths are resolved before invoking ty.

Both clients also pass negative checks for an incorrect request argument and
an incorrectly annotated response return value. Positive consumers check
inferred request, response, and body types for sync and async clients.
Historical Pyright measurements remain in the preceding snapshots; the current
comparison uses ty throughout.

Measurements use an Apple M2 Max with 32 GiB RAM, macOS 26.6.2 arm64, Python
3.12.2, and a release Rust generator/native extension. The successful baseline
and current benchmark stages were run sequentially without competing repository
tests or generation jobs. Generation starts in an empty output directory. File
totals exclude bytecode; Python sizes include `.py` and `.pyi`. These are local
observations, not performance guarantees.

Imports use three fresh interpreters per mode; the table reports medians and the
largest measured RSS. Source mode removes generated bytecode and disables writes;
bytecode mode runs `compileall` first. Filesystem caches are not flushed. Timings
measure the import itself, with process wall time recorded separately. The model
case accesses every exported model; client cases import `GitHubAPI`, including
operation dispatch preparation. Lazy package-root imports are measured separately.

## What changed

1. Each model file contains its declaration and prepared initializer. Separate
   `_init_<model>.py` files are removed during regeneration. Prepared helpers use
   reserved names, and the runtime import is retained even for empty requests.
   Overflow data, shared schema chunks, conversion plans, and recursive binding
   groups retain their existing layout and behavior.
2. OpenAPI validation retains an immutable proof for component definitions that
   were fully exposed to the backend validator. Generated namespaces reuse it
   only when definitions and dependency meanings are unchanged. New wrappers
   still undergo backend validation. Changed definitions, resource/dynamic
   scopes, unsupported reference shapes, and any failed check use the original
   path and diagnostics. Instance validation always uses the complete original
   schema, never the reduced schema used to check new wrappers.
3. Canonicalization reuses members unchanged by a parent rewrite. A member is
   reused only at the same location, within the same reference and annotation
   context; changed members still receive the full recursive pass.

`just check` passes, including schema/fuzz/differential coverage, Python typing,
and website checks. Final targeted runs pass all 10 model-package tests, all 12
local generated-client tests, and clippy. The existing opt-in live internet test
was not rerun for this generator-only change. New regressions cover cache
fallbacks, recursive instance validation, thread-shareable OpenAPI documents,
legacy-file migration, empty requests, and user renames that resemble helpers
or the built-in `bytes` type.

An independent AST-only verifier compares all three packages against the
preceding compiler: **16,148 classes**, **21,607,182,092 reconstructed schema
bytes**, and every prepared validation program are byte-identical. The full
client alone covers **12,351 classes and 148,805 validation nodes**. The verifier
also checks graph equivalence and has self-tests that pass with and without
Python assertions enabled.

## Model count and direct imports

The full client contains **3,228 empty header classes**. These are the clearest
candidate for sharing a structural header model. Requests and named component
models should retain their identities; sharing other anonymous models needs to
account for constraints, HTTP annotations, and reference scope.

The current `api.py` eagerly prepares every operation. The package root and
`models.py` are lazy routers, but touching a model loads its entire undirected
reference group. Directly importing a private operation therefore still pulls
in many unrelated reverse dependents:

| Selective import (diagnostic only) | Loaded model classes | Source | Bytecode |
| --- | ---: | ---: | ---: |
| `SimpleUser` in models | 2,679 | 1.280 s | 0.358 s |
| `issues_get` in Issues | 609 | 0.228 s | 0.093 s |
| `issues_get` in full client | 7,779 | 2.978 s | 0.993 s |

The operation measurements bypass the public API registry. Binding the current
API subsequently loads and prepares the complete registry. They do not measure
a selectively bound client.

The recommended next layout makes `operations/issues_get.py` and
`models/issue.py` the public import paths, keeps the package initializer small,
and replaces undirected groups with directed dependencies: only actual recursive
cycles need to bind together. Operation imports would prepare their own dispatch
plans under the same explicit API identity, keeping later binding cheap.
Removing a barrel without changing these dependencies leaves the expensive
group loads in place. Sharing empty headers first would connect more operations
under the current grouping algorithm, so the two changes should be designed
together. This recommendation is not part of the implemented layout.

## Remaining generation costs

| Instrumented stage, primary run | Models | Issues client | Client projection |
| --- | ---: | ---: | ---: |
| Validate OpenAPI document | 1.09 s | 1.11 s | 1.29 s |
| Canonicalize / validate generated groups | 0.79 s | 0.06 s | 1.61 s |
| Prepare and render models | 1.04 s | 0.08 s | 1.95 s |
| Write staged package files | 0.38 s | 0.08 s | 1.81 s |

These stages exclude startup and some final publication work. Group
canonicalization, preparation, and writing 19,599 files remain substantial.
The next major reduction needs fewer generated classes and narrower dependency
groups, while retaining the existing correctness guarantees.

## Reproduce

The benchmark downloads only the pinned public specification and requires macOS
or Linux for subprocess RSS measurements. Each stage has a 900-second timeout;
failed stages retain their timing and stderr.

```sh
cargo build --release --bin jsoncompat --locked
just _build-python-release

uv run --no-config --project pybindings --all-extras --locked python \
  pybindings/bench_github_openapi.py --case models --output /tmp/github-models-run
uv run --no-config --project pybindings --all-extras --locked python \
  pybindings/bench_github_openapi.py --case issues --output /tmp/github-issues-run
uv run --no-config --project pybindings --all-extras --locked python \
  pybindings/bench_github_openapi.py --case original --output /tmp/github-original-run
uv run --no-config --project pybindings --all-extras --locked python \
  pybindings/bench_github_openapi.py --case client --output /tmp/github-client-run
```

Use a fresh directory for cold generation. `--prepare-only` saves inputs and
exclusions; `--inspect-only` remeasures an existing package without claiming a new
generation time. `--typecheck-only` checks an existing package with the pinned
version of ty, preserving generation and import measurements. Set
`JSONCOMPAT_CODEGEN_TIMING=1` for the stage breakdown.

Compare packages generated by the previous and current compiler:

```sh
uv run --no-config --project pybindings --locked python \
  pybindings/bench_codegen_equivalence.py \
  --before /tmp/previous-models/github_models \
  --after /tmp/github-models-run/github_models \
  --output /tmp/github-model-equivalence.json
```

The [current measurement snapshot](github-openapi-results.json),
[preceding layout baseline](github-openapi-before-merged-models.json),
[earlier generation baseline](github-openapi-before-fast-generation.json), and
[earlier storage baseline](github-openapi-before-dedup.json) preserve the results.
The [artifact equivalence report](github-openapi-equivalence.json) records the
compared generator fingerprints. Raw harness reports retain commands, environment,
source revision, file inventory, and typing diagnostics. Generated packages and
the upstream specification are not checked into the repository.
