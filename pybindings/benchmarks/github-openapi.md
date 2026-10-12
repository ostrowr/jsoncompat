# GitHub OpenAPI scale experiment

GitHub exposes real limits that the synthetic fixtures do not: the model output
fits comfortably within individual Git file limits, but generation, total schema
storage, and import costs are substantial. A smaller complete API namespace is
practical. This is not evidence that the entire GitHub client works unchanged.

## Input and scope

Source: GitHub's official
[OpenAPI 3.1 description](https://github.com/github/rest-api-description/blob/58b1e0c00b39b9e46c24dc4a0ba6a2669c90e9c1/descriptions-next/api.github.com/api.github.com.json),
pinned at `58b1e0c00b39b9e46c24dc4a0ba6a2669c90e9c1`.
The 13,131,466-byte document has **1,232 operations, 973 component schemas,
816 paths, and 274 webhooks**. SHA-256:
`0db1a3dee41ed680628a2a3381a114da7358ac8aa3f7b9276c9198608ca6a1fe`.

Four cases keep failures separate from successful measurements:

- **Original:** fails validation before generation. Two attestation paths in
  each of `/orgs/{org}` and `/users/{username}` have the same template shape with
  different final parameter names (`attestation_id` and `subject_digest`).
- **Models:** sets `paths` to `{}` and generates all 973 unchanged component
  schemas, including their nested classes. This measures the entire model
  population, not a working full API client.
- **JSON client projection:** retains 1,223 operations and every component
  schema. It omits two duplicate-shape operations and seven operations with
  unsupported media, plus the 274 inbound webhooks and 533 documentation examples
  in `components.examples`. The latter collections are currently rejected by the
  outbound operation lowerer. It does not erase schema constraints or replace
  declared response bodies with `Any`.
- **Issues:** selects all 61 `issues/` operations from that projection and uses
  `--component basic-error` to select their transitive model dependencies rather
  than every unrelated component. It produces 664 model declarations in five
  binding groups, with unchanged schemas for the selected contracts.

The script saves each projected document and an itemized exclusion log. An
omitted operation is never counted as successfully supported.

## Measurements

| Metric | All 973 component schemas | 61-operation Issues client |
|---|---:|---:|
| Generated declarations | 3,133 | 664 |
| Generated files (including manifest) | 15,619 | 2,751 |
| Total output | 622.61 MiB | 10.79 MiB |
| Largest Python file | 778.4 KiB | 121.6 KiB |
| 95th percentile Python file | 185.4 KiB | 18.4 KiB |
| Cold generation | 455.80 s | 22.24 s |
| Generation peak RSS | 5.58 GiB | 0.47 GiB |
| Import all models / prepared API, source | 5.637 s | 0.399 s |
| Import all models / prepared API, bytecode | 0.950 s | 0.311 s |
| Lazy package import, bytecode | 2.33 ms | 2.53 ms |
| Import peak RSS, source / bytecode | 647.7 / 619.9 MiB | 50.5 / 44.2 MiB |
| Strict Pyright wall time | 50.52 s; 0 errors | 6.43 s; 0 errors |
| Pyright peak RSS | 3.87 GiB | 0.79 GiB |

The 1,223-operation projection did **not** complete within 900 seconds and
peaked at 7,477,182,464 bytes (6.96 GiB) RSS. No complete package, import timing,
or typing result is reported for it. That exploratory run used the preceding
split generator at `a28b29a2`, before the filename and type-import fixes; the
table above uses fresh runs of the final generator. The original document
fails validation in about 0.09 seconds, before any files are generated.

Measurements use an Apple M2 Max with 32 GiB RAM, macOS 26.6.2 arm64, Python
3.12.2, release Rust generator/native extension, Pyright 1.1.408, and Node
24.14.0 with its default heap limit. Generation
starts in an empty output directory and includes writing the manifest. File
totals exclude bytecode; maximum Python size includes `.py` and `.pyi` files.
The largest overall file is the manifest. These are local observations, not
performance thresholds: other generation/check processes ran concurrently.

Import samples use three fresh interpreters per mode. Source mode removes the
generated package's bytecode first and disables writes; bytecode mode runs
`compileall` first. Filesystem caches are not flushed. Import timings measure
the Python import itself; process wall time and peak RSS are recorded separately.
The model case accesses every exported model, loading every binding group. The
Issues case imports `GitHubAPI`, including every operation and its dispatch
preparation. Merely importing the lazy package root is reported separately.

Strict Pyright checks include **every generated Python file** and a typed
consumer, including the new model barrel. The Issues consumer verifies inferred
success responses for sync and async client aliases. A separate negative test
must reject a string `issue_number` and returning the response's string title
from a function annotated `int`.

Two additional public HTTPX calls decoded the current PR as `IssuesGet200` and
an absent issue as `IssuesGet404`. The latter remained an expected typed `Err`.
Binding the already imported API took about 5 microseconds in that single run;
this is a smoke observation, not a stable microbenchmark.

## Findings and layout

The initial large-package typing run found Pyright's tuple-literal inference
limit: unannotated `__all__` became `tuple[Unknown, ...]` in large barrels. Explicit
`tuple[str, ...]` annotations fix this; a large recursive API regression now
checks the whole generated package and its consumer in strict mode. Moving
annotation dependencies through the new group barrel also exhausted Pyright's
default 4 GiB heap on the full model population. Importing each referenced class
directly from its declaration module fixed that regression without increasing
the heap limit. These imports remain under `TYPE_CHECKING`; runtime forward
annotations still resolve in the shared group namespace.

The old `models_0000.py` name exposed the index of an internal binding group.
The public barrels are now **`models.py` and `api.py`**, with the same convenient
package-root imports. For example:

```text
petstore/
  __init__.py / __init__.pyi
  models.py / models.pyi
  api.py
  _operations/get_pet.py
  _models/shared/
    __init__.py
    pet.py
    _init_pet.py
    _binding.py
    _schema_pet_0000.py
    _program_pet_0000.py
```

Private binding groups use descriptive names; a single group with multiple
roots uses `shared`. Declaration and operation filenames use snake case, with digest
suffixes only for collisions and long names. Numbered chunks remain appropriate
for prepared data. Regeneration migrates earlier managed files, preserves
handwritten files, and refuses to traverse directory symlinks.

About **96% of the full model artifact is compressed source-schema storage**:
many classes retain the same connected definitions in their individual schema
payloads. In this snapshot, 6,168 schema chunk files have only 1,551 distinct contents:
625,390,408 bytes versus 47,943,743 bytes of unique chunk source. This is a
storage opportunity, not a measured optimized artifact. Splitting does not
deduplicate that data or eliminate compression work.
The large connected group also dominates model startup. Sharing source-schema
storage and reducing repeated preparation are the next performance targets;
the current split layout is not a claim that these costs are solved. Individual
model declarations and public export lists also remain whole, so there is no
universal maximum file-size guarantee for arbitrary inputs.

## Reproduce

The benchmark is opt-in, downloads only the pinned public specification, and
requires macOS or Linux for subprocess RSS measurements. Reserve several GiB of
disk and memory; each stage has a 900-second timeout. Failed stages retain their
stderr and timing instead of inventing downstream measurements.

```sh
cargo build --release --bin jsoncompat --locked
just _build-python-release

# Use the repository Python environment, which is Python 3.12 or later.
uv run --no-config --project pybindings --all-extras --locked python \
  pybindings/bench_github_openapi.py --case models --output /tmp/github-models-run
uv run --no-config --project pybindings --all-extras --locked python \
  pybindings/bench_github_openapi.py --case issues --output /tmp/github-issues-run
uv run --no-config --project pybindings --all-extras --locked python \
  pybindings/bench_github_openapi.py --case original --output /tmp/github-original-run
uv run --no-config --project pybindings --all-extras --locked python \
  pybindings/bench_github_openapi.py --case client --output /tmp/github-client-run
```

Use a fresh output directory for cold generation. `--prepare-only` saves inputs
and exclusions without generating. `--inspect-only` remeasures an existing
generated package without claiming a generation time. The checked-in [measurement snapshot](github-openapi-results.json) preserves
this run. New JSON reports record commands, generator digest, source revision,
working-tree state, interpreter,
wall time, peak RSS, file inventory, and Pyright diagnostics. Generated packages
and the upstream spec are deliberately not checked into this repository.
