# GitHub OpenAPI generation benchmark

Full GitHub client generation is **3.400 → 2.485 seconds (27% faster)**
after profiling and removing repeated allocation/traversal work. Every generated
source, stub, and marker is byte-identical to the baseline. Schema validation,
constructor behavior, HTTP metadata, public types, and imports are unchanged.
The one-second full-client target remains unmet.

## Generation and output

| Case | Generation median, before → after | Output (unchanged) | Files | Model classes |
| --- | ---: | ---: | ---: | ---: |
| 973 components | 1.913 → **1.385 s** | 20.06 MiB | 982 | 3,133 |
| 61-operation Issues client | 1.035 → **0.798 s** | 2.51 MiB | 88 | 473 |
| 1,223-operation client | 3.400 → **2.485 s** | 47.38 MiB | 1,030 | 9,124 |

Generation uses three fresh output directories per case. Sizes include the
manifest and exclude bytecode caches. Peak full-client generation RSS across
those samples is **1.18 → 1.05 GiB**. The largest file remains
`operations/repos.py`: **3,144,842 bytes (3.00 MiB)**. Resource/component boundaries,
empty-header aliases, and the checked-in 23-file Petstore golden are unchanged.
Generated packages still contain prepared programs and no source schema JSON.

## Profile and changes

The original approximately 3.4-second profile spent about half its time in
schema processing, about one second preparing/rendering models, and the rest
in operation contracts, grouping, fingerprints, and publication. CPU sampling
identified uneven normalization batches, repeated deep copies, allocations for
unused traversal paths, and software SHA-256 hashing on ARM.

- Normalization workers take the next definition when ready. Large webhook
  schemas no longer collect in the last alphabetic batch. Results and errors
  are consumed in source order, preserving deterministic output and diagnostics.
- Plain optional-property objects finish normalization after their children and
  implicit constraints are normalized. This avoids copying an entire namespace
  through fixed-point passes; interacting constraints retain the general path.
- Source validity checks and canonicalization overlap for large native
  namespaces. Canonical errors retain precedence, and resolver-owned reference
  errors retain their existing deferred behavior. WASM keeps sequential execution.
- Prepared input trees move into their documents. Cached-definition comparisons
  borrow values, and validator inputs copy only definitions that still need
  checking. Model emission constructs fallback schema copies only when needed.
- Schema predicates walk child values without allocating JSON pointers.
  Default/disabled format options with no supplied resources skip a traversal
  that cannot modify the document. Actual dialect, keyword, reference, and
  instance validation still run through their existing checks.
- Response partitioning resolves exact/range/default selectors once instead of
  repeatedly formatting 500 status codes. ARM Unix builds enable the SHA library's
  existing hardware acceleration, retaining runtime CPU detection and its
  portable fallback. Digests remain SHA-256.

Primary full-client runs (wall-clock stages; internal work can overlap):

| Stage | Before | After |
| --- | ---: | ---: |
| OpenAPI validation and normalization | 0.753 s | 0.552 s |
| Generated namespace validation and normalization | 0.841 s | 0.317 s |
| Model declarations, prepared programs, and resource rendering | 1.013 s | 0.998 s |
| Stage model files | 0.129 s | 0.084 s |
| Render and stage operations/exports | 0.256 s | 0.116 s |

Model preparation/rendering remains the largest stage. Within it, preparing
validation and conversion programs takes **0.456 s**.
The other stages include declarations, serialization, resource graph projection,
HTTP contracts, grouping, fingerprints, and filesystem work. Process startup and
teardown are included in the headline timings. Further progress toward one
second requires reducing the remaining model/graph construction and schema
processing. No work moved to imports or first use, and no public option was added.

## Imports and typing

| Case | Source load, before → after | Bytecode load, before → after | Strict ty, after |
| --- | ---: | ---: | ---: |
| 973 components | 1.001 → 0.999 s | 0.244 → 0.241 s | 0.385 s |
| 61-operation Issues client | 0.139 → 0.142 s | 0.072 → 0.071 s | 0.152 s |
| 1,223-operation client | 1.685 → 1.701 s | 0.753 → 0.742 s | 0.790 s |

These are complete model/API loading times after the lightweight package root;
root import samples are recorded separately. Source runs have no generated
bytecode; bytecode runs follow `compileall`. Both variants use the same Python
runtime and identical generated code, so this change does not claim an import
speedup. Small timing differences reflect run-to-run variation.

| Direct import | Source, before → after | Bytecode, before → after |
| --- | ---: | ---: |
| 973 components | 0.020 → 0.019 s | 0.016 → 0.015 s |
| 61-operation Issues client | 0.111 → 0.112 s | 0.040 → 0.041 s |
| 1,223-operation client | 0.178 → 0.179 s | 0.050 → 0.048 s |

The component case imports `models.simple_user` (one class); both client cases
import `operations.issues` (442 classes). Importing `GitHubAPI` loads its complete
registry. Strict `ty 0.0.86 --error all --python-version 3.12` checks all generated
files and representative consumers. Negative consumers retain the expected
argument-type and return-type errors.

## Correctness

All **2,097 managed files** across the three cases are byte-identical before
and after, as are all three independent generations of each optimized package.
Every file digest is independently checked with Python's `hashlib.sha256`.
Only the manifest's compiler fingerprint differs. Independent AST-only
comparison also verifies **12,730 prepared programs byte-identical** and
graph-equivalent, plus **2,094 modules/stubs** with identical syntax.

`just check` passes in full: Rust/schema/fuzz suites, model and client regressions,
strict Python typing, clippy, and the WASM/website build. New regressions compare
optimized normalization with the general rewrite path, check concurrent error
precedence and deferred references, compare borrowed cache matching with the
original metadata-stripping behavior, and cover every schema-child family,
malformed containers, annotation payloads, and default/explicit format options.
Existing goldens and all local adapter examples pass without regeneration.
Public JSONPlaceholder tests were not rerun for this generator-only change.

## Method and input

Measurements use macOS ARM64, Python 3.12.2, and release builds. Baseline revision
`9165c10e5dcc65a3967a02aa4602854f8a98b3ce` already omits schema storage. Both
versions run sequentially after builds and tests, with ordinary warm filesystem
caches, three fresh generation directories, and five fresh import processes per
mode. Runtime bytecode is prepared before timing either variant. The earlier
3.39-second result and intermediate profiling trials are historical measurements;
use this fresh paired control to attribute the improvement.

- Official [GitHub REST description](https://raw.githubusercontent.com/github/rest-api-description/58b1e0c00b39b9e46c24dc4a0ba6a2669c90e9c1/descriptions-next/api.github.com/api.github.com.json), revision `58b1e0c00b39b9e46c24dc4a0ba6a2669c90e9c1`.
- SHA-256: `0db1a3dee41ed680628a2a3381a114da7358ac8aa3f7b9276c9198608ca6a1fe`.
- Original: 13,131,466 bytes, 1,232 operations, 973 components, and 274 webhooks.
- The untouched spec remains unsupported because of two duplicate path-template shapes.
- The full-client projection retains 1,223 operations: it omits those two duplicates,
  seven operations using unsupported media, inbound webhooks, and documentation
  examples. Retained schema constraints and response bodies are not simplified.
- The Issues projection retains 61 operations and their model dependencies.
  The component projection clears paths and selects all 973 schemas.

## Artifacts and reproduction

- [Before/after results](github-openapi-results.json): individual timings, compiler
  hashes, inventories, stage profiles, projections, typing, and equivalence reports.
- [Component equivalence](github-openapi-equivalence.json).
- [Previous schema-removal report](github-openapi-before-normalization.md) and
  [its measurements](github-openapi-before-normalization.json).

```sh
cargo build --release --bin jsoncompat
cargo rustc --release -p jsoncompat_py --lib -- \
  -C link-arg=-undefined -C link-arg=dynamic_lookup # macOS

JSONCOMPAT_CODEGEN_TIMING=1 \
uv run --no-config --project pybindings --all-extras --group benchmark --locked \
  python pybindings/bench_github_openapi.py \
  --output /tmp/github-optimized --case client --repeats 5
```

Use three fresh output directories for generation medians; `--repeats` controls
imports. Repeat with `--case models` and `--case issues`. Run the same harness
with the baseline release binary for a paired comparison. On other platforms,
build the Python extension with `cargo build --release -p jsoncompat_py`.

```sh
uv run --no-config --project pybindings --all-extras --locked python \
  pybindings/bench_codegen_equivalence.py \
  --before /tmp/github-before/github_client \
  --after /tmp/github-optimized/github_client \
  --output /tmp/client-equivalence.json
```
