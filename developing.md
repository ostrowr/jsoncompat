# Developing jsoncompat

This guide is for contributors and maintainers. The user-facing entrypoint is [readme.md](readme.md); package READMEs should stay short, practical, and caller-facing.

## Local setup

Run [bootstrap.sh](bootstrap.sh) once to install the project dependencies, then use:

```bash
just check
```

That is the full local validation gate. It runs the Rust checks and tests, the web checks, and the production website build.

The full Rust/Python suite runs once with `--all-features`, including exact
numeric-lexeme fixtures. Focused numeric and prepared-Python tests also check the
default rounding and range limits. Build the Python extension with the same
precision feature as its Rust integration tests. CI runs these extra default-mode
checks only on Ubuntu/stable, without adding another matrix dimension or full
suite run.

`tests/check_cargo_features.py` checks the resolved feature metadata for every
crate from isolated consumers, with defaults and with `arbitrary-precision`.
It does not compile those consumers: the focused runtime tests cover numeric
behavior, while metadata detects accidental feature unification and missing
forwarding. CI exercises the default WebAssembly release exports. To additionally
check the opt-in WebAssembly build locally:

```bash
wasm-pack build wasm --release --target web --out-dir pkg-precise --features arbitrary-precision
node wasm/tests/verdicts.mjs --arbitrary-precision
```

Use the pnpm version declared in `package.json` when updating JavaScript
dependencies. Keep routine dependency refreshes within compatible version
ranges, preserve the exact Python benchmark pins, and commit the Cargo, pnpm,
and uv lockfiles. After a refresh, run `just check` plus the interactive
presentation's tests and build (`pnpm --filter jsoncompat-interactive test`
and `pnpm --filter jsoncompat-interactive build`).

The deck pins Slidev 52.14.1 and overrides its UnoCSS dependency to 66.6.7
because newer releases produced invalid CSS in the production build. Its
explicit Markdown 14 dependency satisfies the Markdown plugin's peer range.
Revisit these constraints only after the deck builds without CSS syntax warnings.

Useful related commands:

```bash
cargo run --bin jsoncompat -- demo --noninteractive
just bench
just bench-check
```

The benchmark fixtures under [benches/fixtures](benches/fixtures) are fixed on purpose so unrelated fuzz-fixture edits do not move the baseline.

## Repository layout

| Path | Package | Responsibility |
| --- | --- | --- |
| `schema/` | `json_schema_ast` | Draft 2020-12 dialect checks, canonicalization, AST construction, local `$ref` resolution, and raw-validator compilation |
| `src/` | `jsoncompat` | Compatibility checking and the CLI |
| `openapi/` | `jsoncompat_openapi` | OpenAPI document validation and lowering into synthetic request/response schemas |
| `fuzz/` | `json_schema_fuzz` | Schema-guided JSON value generation |
| `codegen/` | `jsoncompat_codegen` | Python source generation, portable validation programs, and conversion proofs |
| `pybindings/` | `jsoncompat_py` | PyO3 bindings |
| `wasm/` | `jsoncompat_wasm` | `wasm-bindgen` bindings |
| `web/` | website | Documentation site and interactive frontend |

Package READMEs stay user-facing on purpose. Deeper implementation notes live here so the crates' front doors remain short and practical.

## Architecture

`SchemaDocument::from_json()` stores the raw source JSON, canonicalizes it once, and preserves precise frontend errors. The raw `jsonschema` backend remains the source of truth for user-facing value validation through `SchemaDocument::is_valid()`.

Reference siblings are intersections. When canonicalization leaves an implicit
type union in place to preserve reference targets, the AST expands that union
without imposing a type on otherwise untyped keywords. `dependentSchemas`
lowers to conditional/intersection nodes in the AST, preserving the original
JSON Pointer targets. Regex membership lazily compiles a shared backend validator so
ECMA-262 character classes have the same meaning in static finite-value proofs
and raw validation.

Canonicalization preserves evaluation annotations throughout documents using
`unevaluatedItems` or `unevaluatedProperties`: it must not insert `items: true`,
invent declared properties from `required`, or discard successful `anyOf`/`if`
annotations. When annotation expansion exceeds the proof engine’s scope,
`analyze_compat` returns `Unknown` unless it finds a validated counterexample.

All crates use the same published upstream `jsonschema` 0.58.6 package from
crates.io, declared in the workspace dependencies. Python's schema validator
delegates numeric/equality assertions to the shared compiler to preserve
jsoncompat's semantics. Ordinary Python validator calls use
`serde_json::Value`; the private borrowed-value entrypoint checks the JSON
shape and materializes a value before calling upstream.

Upstream enables `serde_json/float_roundtrip` for accurate `f64` parsing, which is distinct from
`arbitrary_precision`. This dependency feature is unconditional even with
`default-features = false`; decimal-heavy raw JSON parsing can be slower.
Arbitrary precision remains opt-in. Sharing the backend version across Rust and
Python avoids including two versions of the validator in the Python extension.

Generated models keep their prepared evaluator and allocation-free Python,
Jiter, and model-slot views in the unpublished extension. These application
adapters live in `pybindings/src/instance.rs` and contain no forked schema
validator. They are independent of upstream's representation API and retain
fallible conversion for out-of-range numeric diagnostics.

To compare the migration with an unchanged fork worktree, build both Python
extensions with `just _build-python-release`, build the candidate CLI, and run:

```bash
JSONCOMPAT_NATIVE_PROFILE=release uv run --no-config --project pybindings --all-extras --group benchmark --locked python pybindings/bench_validator_backends.py --baseline /path/to/fork/worktree --output target/python-upstream/benchmark.json
```

The comparison alternates fresh processes using the same interpreter and
generated artifacts. It measures checked JSON round trips, validator calls,
compilation, import, and first use separately, and records each native library's
path and hash. Run it with other builds/tests idle. Both extensions must use
the same numeric feature setting; the default comparison keeps precision off.

The compatibility layer works over the resolved schema graph:

- `SchemaDocument::root()` resolves local `#` / `#/...` references into immutable `SchemaNode`s;
- `src/subset.rs` and `src/subset/*.rs` implement the structural subset checks;
- `check_compat(old, new, role)` turns serializer/deserializer compatibility into directional subset questions;
- `explain_compat_failure()` reports the first useful structural reason the subset check can identify.

The resolved IR is public because the compatibility checker and the fuzzer are separate crates, but the parser-only details stay private. Typed domains such as `IntegerBounds`, `NumberBounds`, `CountRange`, `ContainsConstraint`, and `PatternConstraint` keep impossible states out of the core model where practical.

### Subset checker internals

The subset checker is deliberately a one-sided prover: a `false` result means
"unknown or incompatible", not a proof of non-subset. Keep new rules in that
style unless they are backed by an exact evaluator. The root `src/subset.rs`
only exposes entry points and a module map; `dispatcher.rs` owns the ordered
recursive pipeline. Its phases are intentionally ordered as normalization and
vacuity checks, recursion bookkeeping, pre-kind structural covers, then the
concrete kind-pair dispatch.

Most sibling modules are conservative fact providers. `type_masks`,
`intervals`, `finite`, `enumeration`, `emptiness`, and `properties` compute
upper/lower facts where `None`/`false` means unknown. Higher-level modules such
as `predispatch`, `conditional`, `partitions`, `boolean`, and `disjoint` combine
those facts into proof shortcuts. `membership` owns evaluator probes and the
recursion/productivity guard; avoid calling raw validation negatively unless the
helper explicitly documents that under-acceptance is safe. `explainers` and
`explanation` mirror proof failures without changing verdict behavior.

When adding a rule, prefer a narrow helper in the fact module closest to the
semantic claim, then call it from a named dispatcher phase. Add both a positive
fixture and a near-miss negative fixture, especially for `oneOf`, negation,
conditionals, recursion, and finite-domain/cardinality arguments. If a rule
needs recursion, route it through `SubschemaCheckContext` rather than creating a
fresh visited set; that keeps productive recursion and explanation mode aligned.

For `json_schema_ast`, the user-facing validation surface is intentionally
smaller than the resolved IR surface:

- `SchemaDocument::from_json()` and `SchemaDocument::is_valid()` cover ordinary callers;
- `SchemaDocument::canonical_schema_json()` and `SchemaDocument::root()` exist for analyzers, the compatibility checker, and fuzzing;
- the structured bound and pattern types keep invalid internal states from leaking into downstream reasoning.

## Compatibility diagnostics

`analyze_compat()` distinguishes a proof (`Compatible`), a validated counterexample (`Incompatible`), and an incomplete proof/search (`Unknown`). `AnalysisOptions` bounds deterministic counterexample generation. `check_compat()` remains the one-sided boolean proof API; false alone does not establish a break.

OpenAPI reports preserve unknown operation surfaces. Removed operations are explicit structural breaks. The CLI, Python dictionaries, and WASM objects expose the same three statuses. `compat --json` and `ci --display json` keep stdout machine-readable. CLI exit codes are 0 for compatible, 1 for incompatible or invalid input, and 2 for unknown. There is no warning-only path that ignores assertion semantics.

## Canonicalization and debugging

To inspect the canonicalized schema document that backs compatibility checks and generation:

1. build a `SchemaDocument`;
2. compare the original JSON with `SchemaDocument::canonical_schema_json()`;
3. compile the canonical JSON with `json_schema_ast::compile()` if you need validator-level parity checks on representative values.

Canonicalization is intentionally an internal library facility, not a CLI subcommand.

## OpenAPI internals

Most users should read [openapi/README.md](openapi/README.md). This section records the implementation boundary.

`OpenApiDocument::from_json()` validates OpenAPI 3.1 document shape. `validate_openapi_compatibility_input()` performs the compatibility-readiness pass. `check_openapi_compat()` lowers each supported operation and reuses the ordinary JSON Schema checker.

The lowerer preserves:

- path, query, header, and cookie parameters;
- request body requiredness, media types, and schemas;
- response status/media/body/header variants;
- supported local component references;
- operation removal semantics.

Each operation becomes:

1. a request envelope with `path`, `query`, `headers`, `cookies`, and `body`;
2. a response envelope with `status`, `body`, and `headers`.

Requests are compared in the deserializer direction. Responses are compared in the serializer direction.

The lowerer rejects contract-bearing OpenAPI surfaces it cannot represent without approximation. That includes webhooks, path-item references, callbacks, response links, media-type encoding, remote references, unsupported component collections, media-type selector collisions after normalization, unsupported schema-reference scoping, and unsupported JSON Schema compatibility semantics inside OpenAPI contracts.

OpenAPI metadata that does not change the value-language contract is shape-checked but not lowered into the request or response envelopes.

## Test strategy

Key suites:

- `tests/backcompat.rs` covers hand-authored serializer/deserializer compatibility cases and fuzz-backed counterexample searches;
- `tests/compatibility.rs` checks small regression fixtures in both directions and all roles, including empty languages, with independently labeled raw/canonical/IR membership witnesses;
- `tests/compat_soundness.rs` keeps claimed compatibility aligned with witness spaces;
- `tests/compat_composition_soundness.rs` combines object guards with Boolean applicators and checks numeric extremes against raw-validator witnesses;
- `tests/openapi.rs`, `tests/openapi_fixtures.rs`, and `tests/openapi_soundness.rs` cover the OpenAPI lowering and reporting contract;
- `tests/fuzz.rs` runs JSON Schema Test Suite fixtures through parsing, generation, canonicalization parity, and evaluator checks;
- `tests/dataclasses_backcompat.rs`, `tests/dataclasses_fuzz.rs`, and `tests/dataclasses_stamp_backcompat.rs` keep generated Python models aligned with plain schemas, fuzz fixtures, and stamped writer/reader histories;
- `schema/src/canonicalize/integration_tests.rs` and `schema/src/roundtrip_tests.rs` cover canonicalization and AST round-tripping.

Compatibility fixtures should stay small, synthetic, and net new. Do not add internal or production schemas to the public repository.

When adding incompatible OpenAPI fixtures, keep the human-facing explanation precise as well as the verdict. The fixture contract should make it obvious which schema location broke compatibility.

Every backcompat fixture includes labeled examples and a concrete counterexample
for each incompatible direction. Fixture parsing rejects malformed labels and
missing examples instead of silently skipping them. The OpenAPI fixture runner
also validates `examples.json` against both lowered contracts and requires a
counterexample for every reported request/response incompatibility. Operation
removals are checked against the operation inventory. Generated samples
supplement these labels; generation alone can miss optional pattern properties.

The JSON Schema fixture runner audits imported example labels as well as custom
ones, including negative examples for empty schemas. Optional format-assertion
examples use an assertion-enabled validator for their labels; ordinary
Draft 2020-12 treats `format` as an annotation, with opt-in and vocabulary-selected assertion support. Unsupported
reference targets and dialects retain explicit skips. The suite is a regression
corpus, not an exhaustive proof over all JSON Schemas; conservative false
verdicts and the documented unsupported features remain possible.

The fixture audit corrected `required_to_optional`, `allof_relaxed`, and
`oneof_overlap` to exercise the transitions their names describe. The former
control-escape fallback fixture now expects serializer compatibility because
its pattern can be evaluated exactly. Historical `unsupported_*` fixture names
are retained as regression identifiers, not current support claims.

## Detailed compatibility surface

The end-user README intentionally keeps the feature summary short. The main implementation-facing rules that matter when extending support are:

- the [keyword support matrix](keyword-support.md) tracks parsing, validation, canonicalization, proof, and generation separately;
- scoped references are linked into a memoized local graph; no implicit network retrieval occurs;
- unevaluated constraints share evaluated-location bookkeeping and retain executable validation when structural expansion is too large or recursive;
- decimal multiples and wide bounds use exact rational assertions shared by membership checks and compatibility proofs; ordinary bounds retain a small-number fast path;
- `analyze_compat` distinguishes a proof, a validated counterexample, and an unknown result; the legacy boolean API remains conservative;
- deserializer compatibility can assume old producers omit optional undeclared properties; required and dependency-forced names must still be checked. Intersections, negation, conditionals, exact-one unions, and `contains` proofs use full validation semantics, and recursion keys distinguish those semantics;
- string-pattern reasoning is intentionally conservative when the checker cannot prove regex-language inclusion;
- generation may rely on retries for heuristic cases and distinguishes deterministic `Unsatisfiable` from `ExhaustedAttempts`.

For OpenAPI, the practical rules are:

- document validation catches malformed 3.1 shapes before lowering;
- compatibility readiness catches valid but unsupported lowerability surfaces;
- local component references are supported where the lowerer models them;
- media-type ranges, status ranges, and header identity are normalized directionally so serializer/deserializer compatibility stays meaningful.

## Website and package README boundaries

The repository has several public-facing READMEs:

- [readme.md](readme.md) is the general end-user entrypoint;
- [openapi/README.md](openapi/README.md) is the OpenAPI usage guide;
- [pybindings/README.md](pybindings/README.md), [wasm/README.md](wasm/README.md), [schema/README.md](schema/README.md), and [fuzz/README.md](fuzz/README.md) describe installable packages from a caller's perspective;
- [web/jsoncompatdotcom/README.md](web/jsoncompatdotcom/README.md) only covers running and validating the website locally.

Keep repo architecture, internal invariants, test design, fixture policy, and
maintenance workflow in this guide instead of duplicating them across those
entrypoints.

## Releases

`just release` dry-runs the patch-release flow.

PyPI and npm releases are triggered in CI by manually dispatching the `CI` workflow on a tag. Cargo publishing is still manual. `just check-packages` builds every publishable archive using Cargo’s staged registry. Publishing jobs require the Rust test matrix, archive verification, workflow linting, and their respective artifact builds to succeed. Merging to `main` deploys the website.

The Python wheel release jobs target CPython 3.12–3.15, including the
free-threaded 3.14t builds, on Linux (manylinux and musllinux), Windows, and
macOS. Linux and macOS also build 3.15t wheels. Windows installs a real 3.15
interpreter and its import library; 3.15t wheels are deferred because the
import-library generator currently supports only through 3.14.

The `python-sdist` job builds one source archive, installs it with pip in a
temporary virtual environment, and exercises the native APIs, typing files,
generated dataclasses, optional codecs, and stamped readers/writers. The test
runs outside the checkout with isolated Python imports and fresh native build
outputs. Only a tested archive is uploaded; the Python release job waits for
it and attests and publishes it alongside the wheels.

To run the same source-install check locally with Maturin installed:

```bash
maturin sdist --manifest-path pybindings/Cargo.toml --out target/sdist
python3 pybindings/tests/check_sdist.py target/sdist/*.tar.gz
```

Prepared validation programs use a generic borrowed-value interface in `codegen/src/prepared_schema/instance.rs`. The Python extension implements it for its parser and Python views. The published codegen crate has no Python-specific dependencies.


### Prepared runtime boundaries

`codegen/src/prepared_schema` owns portable validation, annotations, precompiled
patterns, and cold structured diagnostics. `pybindings/src/prepared_schema.rs`
adapts Python/Jiter values without exposing Python-specific types in published crates.
`pybindings/src/model_converter/{input,output,projection}.rs` separate construction,
serialization, and borrowed model views; binding/layout remains in the parent.
`openapi/src/validation.rs` validates document shapes before lowering, while
`schema_groups.rs` partitions component packages without breaking references.

Use `just check-packages` before release: it packages every published Rust crate
and builds the resulting dependency closure from archives. CI runs this gate
independently of workspace tests. Keep Python-specific instance adapters confined
to the unpublished extension.

Representative generated Python snapshots remain in source control; the full
fixture corpus is regenerated for E2E tests and benchmarks. Do not replace runtime
coverage with snapshots or label a repeated cached shard as a unique model package.
`just python-bench-imports` now measures unique OpenAPI-generated packages and
records both completed and memory-limited populations.
