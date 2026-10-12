# GitHub OpenAPI scale experiment

Generated packages now share identical compressed source-schema chunks across
all models and binding groups. This experiment measures the storage and import
effects against the previous layout, while retaining the same schema semantics.
Generation remains expensive for large connected schemas. This is not evidence
that the entire GitHub client works unchanged.

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

The full model package shrank from **622.61 MiB to 71.13 MiB (88.6%)**.
Schema chunk storage alone fell from 596.42 MiB to 45.74 MiB. All **3,133
compressed schema streams remain byte-for-byte identical** to the baseline;
6,168 chunk references now share 1,551 source files and 1,551 in-memory byte
objects. The comparison imported both packages and reconstructed every stream,
including nested models, while excluding duplicate public aliases.

| Full model population | Before | Shared chunks |
|---|---:|---:|
| Total output | 622.61 MiB | 71.13 MiB |
| Files | 15,619 | 11,003 |
| Source import | 5.637 s | 1.979 s |
| Bytecode import | 0.950 s | 0.710 s |
| Bytecode import peak RSS | 619.9 MiB | 164.9 MiB |
| Strict Pyright | 50.52 s | 15.85 s |
| Cold generation | 455.80 s | 436.95 s |

The Issues client has mostly distinct small schema payloads. Its total output
increased by 22.4 KiB (0.2%) because digest-based imports cost slightly more than
the little duplication removed. Its prior size was 10.79 MiB. Sharing primarily
benefits models that repeat large connected definition sets.

Current measurements:

| Metric | All 973 component schemas | 61-operation Issues client |
| --- | ---: | ---: |
| Generated declarations | 3,133 | 664 |
| Generated files (including manifest) | 11,003 | 2,729 |
| Total output | 71.13 MiB | 10.81 MiB |
| Largest Python file | 778.4 KiB | 121.6 KiB |
| 95th percentile Python file | 69.7 KiB | 18.4 KiB |
| Cold generation | 436.95 s | 23.23 s |
| Generation peak RSS | 5.47 GiB | 0.49 GiB |
| Import all models / prepared API, source | 1.979 s | 0.382 s |
| Import all models / prepared API, bytecode | 0.710 s | 0.204 s |
| Lazy package import, bytecode | 2.42 ms | 2.33 ms |
| Import peak RSS, source / bytecode | 191.8 / 164.9 MiB | 50.8 / 44.8 MiB |
| Strict Pyright wall time | 15.85 s; 0 errors | 3.44 s; 0 errors |
| Pyright peak RSS | 2.41 GiB | 0.78 GiB |

The 1,223-operation projection again did **not** complete within 900 seconds
(900.28 s; 7,476,281,344 bytes / 6.96 GiB peak RSS). No complete package, import
timing, or typing result is reported for it. Unlike the earlier exploratory
projection at `a28b29a2`, this rerun uses the same generator as the successful
cases above. Two five-second stack samples were collected during this run to
investigate the bottleneck below. The untouched document still fails validation
before generation, in 0.075 seconds, on duplicate path-template shapes.

Measurements use an Apple M2 Max with 32 GiB RAM, macOS 26.6.2 arm64, Python
3.12.2, release Rust generator/native extension, Pyright 1.1.408, and Node
24.14.0 with its default heap limit. Generation
starts in an empty output directory and includes writing the manifest. File
totals exclude bytecode; maximum Python size includes `.py` and `.pyi` files.
The largest overall file is the manifest. These are local observations, not
performance thresholds. The rerun executes benchmark stages sequentially with no
other repository tests or generation jobs running. The earlier baseline had
concurrent generation/check activity, so timing differences should not be read
as controlled speedup guarantees; artifact sizes and schema equality are exact.

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

In the earlier run, two additional public HTTPX calls decoded the current PR as
`IssuesGet200` and an absent issue as `IssuesGet404`. The latter remained an
expected typed `Err`.
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
  _schemas/
    __init__.py
    _schema_<sha256>.py
  _models/shared/
    __init__.py
    pet.py
    _init_pet.py
    _binding.py
    _program_pet_0000.py
```

Private binding groups use descriptive names; a single group with multiple
roots uses `shared`. Declaration and operation filenames use snake case, with digest
suffixes only for collisions and long names. Numbered chunks remain appropriate
for prepared data. Regeneration migrates earlier managed files, preserves
handwritten files, and refuses to traverse directory symlinks.

The baseline stored 6,168 source-schema chunks, many repeating the same connected
definitions. Schema storage alone occupied 625,390,408 bytes, about **96% of the
full artifact**. The new `_schemas/` pool names each compressed chunk by its full
SHA-256 digest. Identical chunks share one source file and one imported `bytes`
object; each model retains a tuple of references in the original order. Only an
explicit `Model.__jsoncompat_schema__` access joins and decompresses its stream,
then caches the schema text on that class. Imports, constructors, validation, and
error reporting do not expand or copy the complete schema stream.

Manifest v5 tracks shared ownership: cached groups retain their chunks, damaged
chunks rebuild every owner, and a chunk disappears only when its last owner is
removed. Regeneration also removes the old per-model schema source files. Shared data
does not import unrelated model groups or merge their class identities. The
regression suite covers these properties, multi-chunk reconstruction, and lazy
introspection. Standalone JSON Schema output and previously generated byte
payloads remain supported by the runtime.

This change deduplicates stored data; it does not eliminate repeated schema
construction, compression, or validator preparation during generation. Those
remain separate performance targets. Individual model declarations and public
export lists also remain whole, so there is no universal maximum file-size
guarantee for arbitrary inputs.

## Generation bottleneck

Storage sharing happens after compilation, so it does not address the main
repeated work during generation. Two five-second samples of the full-client run,
taken several minutes apart, placed 4,014/4,031 and 4,049/4,076 main-thread stack
samples inside `PreparedModule::compile` (99.6% and 99.3%). These are observations
of that phase, not a whole-run allocation of generation time.

The current pipeline:

1. [Copies root definitions into reference-bearing class schemas](../../codegen/src/dataclasses.rs)
   and pretty-prints each as a separate JSON string.
2. [Parses each string back into JSON and compiles it independently](../../codegen/src/dataclasses/plan.rs).
3. [Constructs a fresh SchemaDocument](../../codegen/src/prepared_schema.rs),
   repeating normalization, reference resolution, and backend validator building
   before emitting each prepared validator. The samples include substantial
   time in canonicalization and source-schema validation.
4. [Compresses every complete class schema at level 9](../../codegen/src/dataclasses/render.rs),
   building the legacy single-file source string even for package output. The
   package layout only deduplicates chunks afterward.

The next optimization target is one validated, normalized schema context per
binding group, with model roots referring into it. Prepared nodes and schema
storage can then reuse that context instead of repeatedly processing whole
definition maps. Package output should also skip building unused legacy source.
Reference scopes, recursion, and validation semantics must
remain intact. A full phase timing breakdown is still needed before assigning
an expected speedup to that change.

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
generated package without claiming a generation time. The checked-in
[measurement snapshot](github-openapi-results.json) preserves this run, and the
[baseline snapshot](github-openapi-before-dedup.json) preserves the earlier layout.
New JSON reports record commands, generator digest, source revision,
working-tree state, interpreter,
wall time, peak RSS, file inventory, and Pyright diagnostics. Generated packages
and the upstream spec are deliberately not checked into this repository.
