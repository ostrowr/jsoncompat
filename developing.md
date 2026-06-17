# Developing jsoncompat

This guide is for contributors and maintainers. The user-facing entrypoint is [readme.md](readme.md); package READMEs should stay short, practical, and caller-facing.

## Local setup

Run [bootstrap.sh](bootstrap.sh) once to install the project dependencies, then use:

```bash
just check
```

That is the full local validation gate. It runs the Rust checks and tests, the web checks, and the production website build.

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
| `python/` | `jsoncompat_py` | PyO3 bindings |
| `wasm/` | `jsoncompat_wasm` | `wasm-bindgen` bindings |
| `web/` | website | Documentation site and interactive frontend |

Package READMEs stay user-facing on purpose. Deeper implementation notes live here so the crates' front doors remain short and practical.

## Architecture

`SchemaDocument::from_json()` stores the raw source JSON, canonicalizes it once, and preserves precise frontend errors. The raw `jsonschema` backend remains the source of truth for user-facing value validation through `SchemaDocument::is_valid()`.

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

`validate_compatibility_input()` only rejects inputs that cannot participate in a sound comparison. Warning-only gaps are exposed separately through `compatibility_warnings()`.

The split is deliberate:

- warning-only raw JSON Schema keywords are valid inputs whose semantics are not yet modeled by the subset checker;
- hard errors remain hard errors for unsupported reference scoping, non-integral `number.multipleOf`, unsafe floating-point number-bound precision, malformed schemas, and other cases that would make a static verdict unsafe.

`jsoncompat compat` prints warnings before the verdict for raw schemas; `jsoncompat compat --openapi` selects the separate OpenAPI contract path explicitly. `jsoncompat ci` keeps the warning text in its output without turning that grade into `Invalid`.

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
- `tests/compat_soundness.rs` keeps claimed compatibility aligned with witness spaces;
- `tests/openapi.rs`, `tests/openapi_fixtures.rs`, and `tests/openapi_soundness.rs` cover the OpenAPI lowering and reporting contract;
- `tests/fuzz.rs` runs JSON Schema Test Suite fixtures through parsing, generation, canonicalization parity, and evaluator checks;
- `tests/dataclasses_backcompat.rs`, `tests/dataclasses_fuzz.rs`, and `tests/dataclasses_stamp_backcompat.rs` keep generated Python models aligned with plain schemas, fuzz fixtures, and stamped writer/reader histories;
- `schema/src/canonicalize/integration_tests.rs` and `schema/src/roundtrip_tests.rs` cover canonicalization and AST round-tripping.

Compatibility fixtures should stay small, synthetic, and net new. Do not add internal or production schemas to the public repository.

When adding incompatible OpenAPI fixtures, keep the human-facing explanation precise as well as the verdict. The fixture contract should make it obvious which schema location broke compatibility.

## Detailed compatibility surface

The end-user README intentionally keeps the feature summary short. The main implementation-facing rules that matter when extending support are:

- raw JSON Schema warnings currently cover `additionalItems`, `contentEncoding`, `contentMediaType`, `contentSchema`, `dependencies`, `dependentSchemas`, `unevaluatedItems`, and `unevaluatedProperties`;
- hard compatibility errors currently cover `$id`, `$anchor`, `$dynamicRef`, `$dynamicAnchor`, unsupported non-local references, non-integral `number.multipleOf`, and number-schema bounds outside the adjacent-integer-safe `f64` range `[-9007199254740991, 9007199254740991]`;
- serializer compatibility assumes producers do not emit undeclared extra properties, even when `additionalProperties: true`;
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
- [python/README.md](python/README.md), [wasm/README.md](wasm/README.md), [schema/README.md](schema/README.md), and [fuzz/README.md](fuzz/README.md) describe installable packages from a caller's perspective;
- [web/jsoncompatdotcom/README.md](web/jsoncompatdotcom/README.md) only covers running and validating the website locally.

Keep repo architecture, internal invariants, test design, fixture policy, and
maintenance workflow in this guide instead of duplicating them across those
entrypoints.

## Releases

`just release` dry-runs the patch-release flow.

PyPI and npm releases are triggered in CI by manually dispatching the `CI` workflow on a tag. Cargo publishing is still manual. Merging to `main` deploys the website.
