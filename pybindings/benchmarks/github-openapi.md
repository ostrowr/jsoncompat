# GitHub OpenAPI scale experiment

Cold generation now takes **5.18 seconds for all GitHub models** and **2.21
seconds for the Issues client**, down from 436.95 and 23.23 seconds. The
**1,223-operation client projection completes in 9.24 seconds**; the previous
build timed out after 900 seconds. These measurements include writing every
output file and the manifest. The approximately one-second target is not met.

Compilation remains a build-time operation. This change does not defer schema
compilation, normalization, or regex compilation to import or first model use.
The original upstream document still requires the explicitly described
projection.

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

| Cold generation | Previous build | Current build | Generation peak RSS, previous / current |
| --- | ---: | ---: | ---: |
| All 973 component schemas | 436.95 s | 5.18 s | 5.47 / 0.67 GiB |
| 61-operation Issues client | 23.23 s | 2.21 s | 0.49 / 0.38 GiB |
| 1,223-operation client projection | Timeout at 900 s | 9.24 s | 6.96 / 0.91 GiB |

The table uses each primary harness run, whose package was then measured for
imports and typing. Two additional generations into fresh directories produced
three-run medians of **4.85 s / 2.23 s / 9.28 s** for models / Issues / the client
projection, with ranges **4.84–5.18 s / 2.21–2.25 s / 9.24–9.32 s**. These are
complete cold generations; filesystem caches were not flushed.

The model package also shrank from **71.13 MiB to 28.44 MiB**, and the Issues
package from **10.81 MiB to 4.51 MiB**. The earlier layout before any schema
sharing occupied 622.61 MiB for the models.

| Current artifact and import measurements | Models | Issues client | Client projection |
| --- | ---: | ---: | ---: |
| Generated declarations | 3,133 | 664 | 12,351 |
| Binding groups | 4 | 5 | 34 |
| Files, including manifest | 7,228 | 1,783 | 31,950 |
| Total output | 28.44 MiB | 4.51 MiB | 89.04 MiB |
| Largest Python file | 778.4 KiB | 121.6 KiB | 2,974.6 KiB |
| 95th percentile Python file | 14.5 KiB | 6.2 KiB | 5.0 KiB |
| Import all models / prepared API, source | 1.479 s | 0.287 s | 5.432 s |
| Import all models / prepared API, bytecode | 0.541 s | 0.158 s | 2.696 s |
| Lazy package import, bytecode | 2.34 ms | 2.34 ms | 9.62 ms |
| Import peak RSS, source / bytecode | 167.5 / 142.6 MiB | 47.0 / 40.5 MiB | 417.5 / 353.4 MiB |

Type checking now uses **ty 0.0.86 with every rule treated as an error** and a
Python 3.12 target. It checks every generated Python file and a typed consumer:

| ty check | Models | Issues client | Client projection |
| --- | ---: | ---: | ---: |
| Wall time | 0.775 s | 0.210 s | 3.027 s |
| Peak RSS | 0.42 GiB | 0.10 GiB | 1.21 GiB |
| Diagnostics | 0 | 0 | 0 |

Both clients also pass negative checks: a string `issue_number` is rejected, as
is returning the response's string title from a function declared to return
`int`. Positive consumers verify inferred response and body types for both
synchronous and asynchronous clients.

Earlier exploratory checks used Pyright 1.1.408 and Node 24.14.0. The complete
client exhausted Node's default **4 GiB heap** in both the positive and negative
checks (18.13 and 17.91 seconds); explicit **8 GiB** reruns also exhausted the heap
(35.63 and 34.51 seconds). The models and Issues package passed Pyright. Those
results remain separate in the snapshot as historical measurements. The switch
of type checker is not a generation optimization or a like-for-like typing
speed comparison.

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

The old compiler copied entire definition maps into each reference-bearing
model, pretty-printed them, parsed them again, and rebuilt a validator for every
class. It compressed those expanded schemas independently and rendered an unused
single-file implementation before producing package files.

The new pipeline:

1. Keeps local model schemas separate from shared definitions, reuses validated
   documents, and prepares common validation nodes and regexes once per group.
   It emits standalone prepared programs for each model with the original
   promoted-root recursion behavior.
2. Validates distinct OpenAPI schema roots together with every component,
   including unused components. Only ordinary references to known schema
   positions use this path. Resource scopes, dynamic references, data targets,
   and any validation failure retain the original validation path and diagnostics.
3. Renders package artifacts directly. Shared definition fragments are serialized
   and compressed once; their exact original schema text is reconstructed only
   for explicit `Model.__jsoncompat_schema__` access and cached on that class.
   Small local source and prepared payloads live in existing initializer modules;
   larger data and shared fragments remain separate chunks.
4. Publishes new generated subtrees with directory renames after checking every
   destination for conflicts. Incremental regeneration preserves caller-owned
   files, unchanged outputs, and shared-schema ownership.

Named model and operation modules remain stable. Public imports still use
`models.py`, `api.py`, or the package root. Shared compressed fragments live in
`_schemas/_schema_<sha256>.py`; private model groups retain one declaration and
initializer per model. Numbered modules store large prepared data, not arbitrary
collections of declarations. Individual declarations and export lists remain
whole, so there is no universal maximum file size for arbitrary inputs.

The regression suite checks reference scopes, recursive root identity and depth
limits, validation errors, exact introspection text, shared storage, import/use
without compilation or decompression, legacy artifacts, and safe publication.
A separate artifact comparison confirms byte-for-byte equality for all **3,133
expanded model schema texts** (4,455,281,588 bytes) and all **3,133 prepared
programs**, containing **60,993 validator nodes**, against the preceding
generator. The verifier also compares every reachable rule, ordered edge,
compiled regex, and reference diagnostic while allowing node IDs to differ. This
checks the prepared programs and exposed schema text, not the complete generated
Python source: generated declarations now hide runtime-only slots and model
binding from type checkers with `typing.TYPE_CHECKING` guards.

## Remaining generation costs

Instrumented stages explain why the largest case is still several seconds:

| Generation stage | Models | Issues client | Client projection |
| --- | ---: | ---: | ---: |
| Validate OpenAPI document | 1.62 s | 1.61 s | 1.71 s |
| Validate generated group schemas | 1.30 s | 0.07 s | 2.00 s |
| Prepare and render models | 1.09 s | 0.09 s | 2.08 s |
| Write staged package files | 0.57 s | 0.12 s | 2.37 s |

These stage timers exclude some startup and final cleanup work; complete process
wall times appear above. Schema validation and writing tens of thousands of files
now dominate the largest package. Further reductions toward one second require
reducing those costs while preserving validation and the stable split layout.

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
[previous generation baseline](github-openapi-before-fast-generation.json), and
[earlier storage baseline](github-openapi-before-dedup.json) preserve the results.
The [artifact equivalence report](github-openapi-equivalence.json) records the
compared generator fingerprints. Raw harness reports retain commands, environment,
source revision, file inventory, and typing diagnostics. Generated packages and
the upstream specification are not checked into the repository.
