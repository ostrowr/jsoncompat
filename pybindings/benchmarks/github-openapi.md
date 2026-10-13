# GitHub OpenAPI generation benchmark

The resource/component layout reduces the full client from **19,599 files to
1,047**, and from **86.80 MiB to 63.48 MiB**. Generation improves from **7.48 s to
5.42 s** median. Source API loading improves from **5.548 s to 2.066 s**; bytecode
loading improves from **2.407 s to 0.852 s**. Strict `ty` checks pass.

Peak generation memory increases from **0.95 GiB to 1.41 GiB**. API generation now
uses one shared compilation namespace, so more compiler state is live together.
This is a remaining cost; the approximately one-second generation target is
still unmet. Runtime imports remain directed and selective.

## Input and scope

- Official [GitHub REST API description](https://raw.githubusercontent.com/github/rest-api-description/58b1e0c00b39b9e46c24dc4a0ba6a2669c90e9c1/descriptions-next/api.github.com/api.github.com.json), revision
  `58b1e0c00b39b9e46c24dc4a0ba6a2669c90e9c1`.
- SHA-256: `0db1a3dee41ed680628a2a3381a114da7358ac8aa3f7b9276c9198608ca6a1fe`.
- Original: 13,131,466 bytes, 1,232 operations, 973 component schemas,
  816 paths, and 274 inbound webhooks.
- The untouched document still fails OpenAPI validation on two duplicate
  path-template shapes. It is not counted as successfully generated.
- The client projection retains **1,223 operations**. It excludes those two
  operations, seven operations with unsupported media, inbound webhooks, and
  documentation examples. Component constraints and retained operation bodies
  are not simplified.
- The Issues projection contains 61 operations and the dependency closure of
  `basic-error`; the component-only projection clears paths and selects all 973
  schemas. The harness saves every projection and exclusion.

These are release-build measurements on macOS ARM64 with Python 3.12.2 and
`ty 0.0.86 --error all --python-version 3.12`. All final runs were sequential,
after repository tests and builds finished. Generation uses fresh output
directories with ordinary warm filesystem caches; it includes file publication.
The table reports three-run medians. Imports run in fresh Python processes;
source runs disable bytecode and bytecode runs follow `compileall`. Import times
below measure model/API loading after importing the lightweight package root.
Root-import overhead, process wall time, memory, and individual samples are
recorded separately in the JSON results. `ty` times include its launcher.

## Measurements

| Case | Files, before → after | Classes, before → after | Generation median, before → after |
| --- | ---: | ---: | ---: |
| 973 components | 4,095 → **999** | 3,133 → **3,133** | 3.73 → **3.28 s** |
| 61-operation Issues client | 1,119 → **105** | 664 → **473** | 1.64 → **1.51 s** |
| 1,223-operation client | 19,599 → **1,047** | 12,351 → **9,124** | 7.48 → **5.42 s** |

| Case | Output | Largest Python file | Generation peak RSS | Source load | Bytecode load | ty |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 973 components | 25.79 MiB | 1.16 MiB | 0.70 GiB | 1.118 s | 0.260 s | 0.346 s |
| 61-operation Issues client | 3.36 MiB | 1.74 MiB | 0.40 GiB | 0.163 s | 0.080 s | 0.186 s |
| 1,223-operation client | 63.48 MiB | 3.55 MiB | 1.41 GiB | 2.066 s | 0.852 s | 1.117 s |

The full client's largest source is `operations/repos.py` (**3,718,810 bytes**).
Its files comprise 973 component modules, one shared empty-header module,
45 resource modules, 16 packed schema-data modules, and 12 support/manifest files.
Output sizes exclude bytecode caches. All generated files and representative
sync/async consumers type-check without diagnostics; both client negative
consumers still report the expected argument-type and return-type errors.

The preceding full-client primary `ty` run took 5.714 s, while separate warmed
interleaved repeats had a 2.236 s median. Those distinct measurements remain in
the baseline snapshot; they are not treated as interchangeable baselines for
the new 1.117 s primary run.

## Direct imports

| Import | Loaded model classes | Source | Bytecode |
| --- | ---: | ---: | ---: |
| `models.simple_user` | 1 | 0.021 s | 0.016 s |
| Issues-only `operations.issues` | 442 | 0.129 s | 0.045 s |
| Full-client `operations.issues` | 442 | 0.275 s | 0.058 s |

Previously, accessing `SimpleUser` loaded 2,679 classes. Importing the private
`issues_get` operation loaded 7,779 classes in the full client. The new public
resource import loads all 61 Issues operations and their dependencies: **442
classes**. It does not import unrelated resources or initialize the full API
registry. These are selective-import measurements, not selectively bound clients:
importing `GitHubAPI` or binding it still prepares the full registry, as documented.

## Implementation and correctness

- Each named component owns its nested declarations in `models/<component>.py`.
  Each resource owns requests, response envelopes, anonymous body models, aliases,
  and operation metadata in `operations/<resource>.py`. Resource names use the
  `operationId` namespace before `/`, or the first literal path segment as a
  fallback. No file-size budget changes these boundaries.
- Dependency edges are directed. Only actual strongly connected module cycles
  share a `_cycles/` implementation. Canonical helpers outside named components
  use `models/_shared.py`. Public convenience exports remain lazy, and `.pyi`
  files provide static exports without duplicating runtime import statements.
- A module binds its local prepared graph and links already prepared dependency
  nodes. Linking preserves model identities, scalar guards, branch validators,
  discriminator ordinals, JSON field prefixes, and slot ownership. Shared nodes
  in diamond-shaped dependencies are interned by origin. No schema compilation,
  reflection-based model construction, or first-request preparation is added.
- The **3,228 empty header classes become one closed-object class**. Old
  operation-specific header names remain aliases. Nonempty header records remain
  distinct, empty records still reject extra fields, and raw HTTP headers retain
  all names, order, and repeated values.
- Compressed source fragments are interned and packed into sixteen fixed hash
  buckets. They expand only on explicit schema introspection. The manifest tracks
  symbol ownership inside each bucket, removes unowned constants, repairs missing
  or modified files, preserves unmanaged files, and migrates earlier layouts.
- Resource rendering caches shared fragment identities and precomputes program
  names once. Pointer-parent lookups assign nested ownership without repeatedly
  scanning every declaration. The initial exploratory regression to 15.43 s was
  fixed before these recorded runs.

Independent AST-only comparisons cover **all 16,148 previous model names** across
the three packages. Model schema roots match and complete reachable prepared
validation graphs are bisimilar. The full client alone covers **148,805 node
pairs**, including all legacy empty-header aliases. Of its 12,351 previous model
names, **9,048 prepared programs remain byte-identical**.

Compilation namespaces and shared header definition names change, so this pass
does **not** claim byte-identical full schema-introspection text. The verifier
compares root constraints separately from build namespace definitions, then
checks every reachable validation rule, edge, constant, pattern, and diagnostic
reference. Only the explicitly shared empty-header names are normalized in
schema references, generated declaration metadata, and validator reference
pointers. Literal values are never normalized.

Validation: `just check` passed; final focused checks pass all **12 package
tests**, all **13 local client tests**, and clippy with warnings denied. Tests
cover direct import isolation, diamond dependencies, recursive identity and type
hints, canonicalized external components, empty-header aliases, raw repeated
headers, incremental repair/migration, and strict typing. All three adapters run
against the local test server. The opt-in public JSONPlaceholder test was not
rerun for this generator change; earlier live validation remains in the PR.

## Artifacts and reproduction

- [Current results](github-openapi-results.json): fingerprints, environment,
  individual measurements, cold-generation repeats, stage profiles, selective
  imports, typing diagnostics, and all three equivalence reports.
- [Previous layout](github-openapi-before-resource-layout.json).
- [Component equivalence](github-openapi-equivalence.json).
- Earlier optimization snapshots remain in this directory.

```sh
cargo build --release --bin jsoncompat
cargo rustc --release -p jsoncompat_py --lib -- \
  -C link-arg=-undefined -C link-arg=dynamic_lookup # macOS

uv run --no-config --project pybindings --all-extras --group benchmark --locked \
  python pybindings/bench_github_openapi.py \
  --output /tmp/github-resource-run --case client --repeats 3
```

Repeat with `--case models`, `--case issues`, and `--case original`. Use a fresh
output directory for each cold generation run; `--inspect-only` reuses artifacts
for import/typing measurements. On other platforms, build the Python extension
with `cargo build --release -p jsoncompat_py`.

```sh
uv run --no-config --project pybindings --all-extras --locked python \
  pybindings/bench_codegen_equivalence.py \
  --before /tmp/previous/github_client \
  --after /tmp/github-resource-run/github_client \
  --output /tmp/client-equivalence.json
```
