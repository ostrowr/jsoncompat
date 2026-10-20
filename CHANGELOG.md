# Changelog

User-visible changes across jsoncompat's Rust, CLI, Python, and WebAssembly
packages. Historical entries are reconstructed from the repository's `v*` tags
and their source changes, including prereleases. Dates follow the release tags;
a tag does not imply that every package was published to every registry.

## 0.6.0 (Unreleased)

These are the prepared release notes for 0.6.0. The version bump and publication
are still pending. Installation examples in the documentation target that
upcoming release.

[Changes since 0.5.0](https://github.com/ostrowr/jsoncompat/compare/v0.5.0..main)

### Added

- Generate typed Python HTTP clients from OpenAPI 3.1 with
  `jsoncompat codegen --openapi openapi.json --api Petstore --output petstore`.
  A generated request selects its endpoint and response types through
  `client.request(GetPetRequest(...))`. Generated synchronous and asynchronous
  client protocols keep API boundaries and per-call transport options typed.
- Optional requests, HTTPX sync/async, and aiohttp adapters. Sessions remain
  caller-owned; custom fetchers can implement the same small transport protocol.
  Request overrides cover headers, query parameters, and other escape hatches.
- Buffered responses with explicit lazy parsing. `request()` downloads the body;
  `.status`, `.headers`, and `.content` are available immediately. `.unwrap()`
  parses a typed success or raises; `.result()` returns `Ok` or `Err`. Both share
  the same cached decoding outcome. Undeclared or malformed non-2xx responses
  preserve their status, headers, and bytes in `UnexpectedResponse`; malformed
  or undeclared successes raise `ResponseError` when parsing is requested.
- URL-encoded and multipart request bodies, binary uploads/downloads, text,
  CSV/HTML, NDJSON, optional YAML, finite event-stream text, media parameters,
  and additional query serialization controls. These adapters buffer bodies;
  they do not provide unbounded streaming. JSON media retains JSON semantics,
  including when a schema has a `format: binary` annotation.
- A checked-in generated Petstore client, runnable adapter examples, live
  JSONPlaceholder tests, and a pinned public-spec audit. The audit passes 29
  complete OpenAPI 3.1 documents; 13 still fail the current document/contract
  checks. Proposed tolerance for malformed metadata is not implemented here.

### Changed and fixed

- Generated packages use named component and resource modules, readable typed
  constructors, selective imports, and HTTP dispatch plans prepared by codegen.
  Schema preparation reuses validated definitions and avoids repeated cloning
  and normalization. Generated artifacts retain prepared programs, without
  storing or compressing copies of source schema JSON.
- Compiled regex tables are shared across generated modules and native models.
  The recorded complete Prefect benchmark reduced source/stubs from 64.54 MiB
  to 8.94 MiB and cached full-client import from 391 ms to 108 ms. These are local
  before/after measurements within the 0.6.0 development cycle, not comparisons
  against 0.5.0; see the [benchmark report](pybindings/benchmarks/shared-prepared-patterns.md).
- Regeneration invalidates stale timestamp-based Python bytecode, including
  external `PYTHONPYCACHEPREFIX` caches. Unchanged generated package files retain
  their timestamps. Python fields beginning with `__` receive safe names while
  preserving their original JSON keys.
- Rust and Python share upstream `jsonschema` 0.58.6; building the Python package
  no longer requires a Git fork of that validator. Remote schema fetching stays
  disabled, and internationalized hostname/email format support is retained.
- Nested-intersection value generation avoids repeatedly restarting exhausted
  searches within one candidate. Local checks use `ty`, and regressions cover
  Windows fixture line endings and concurrent typing-test directories.

### Breaking changes

| Change | Effect on existing users |
| --- | --- |
| Arbitrary precision is off by default. | Default Rust, CLI, Python, and WebAssembly builds use ordinary `serde_json` numeric limits. Decimal rounding, underflow, or out-of-range parse errors can change validation, generated constraints, and compatibility verdicts. Enable the `arbitrary-precision` build feature when exact JSON numeric text is required. |
| The omission sentinel API is renamed. | Replace `JSONCOMPAT_MISSING` with `MISSING` and `JsoncompatMissingType` with `MissingType`. The old exports are removed. `MissingType` is now a singleton enum; `bool(MISSING)` raises `TypeError`. |
| Generated model artifacts changed. | Regenerate 0.5.0 output with the matching 0.6.0 generator/runtime. Old private binding calls and prepared programs are not a supported interchange format. `Model.__jsoncompat_schema__` is removed; retain source schemas separately. |
| Generated names and OpenAPI package paths changed. | Numeric modules such as `models_0000.py` are replaced by named modules such as `models/order.py`. Private-looking fields and newly reserved names can receive different Python spellings. Update affected imports and attribute access. `--models-per-module` now controls internal compilation batches for model-only generation, not public file layout. |

### Migration from 0.5.0

#### 1. Choose your numeric precision before regenerating

If bounded integers and ordinary floating-point behavior are sufficient, the
default build needs no extra setting. If your schemas or data rely on exact
decimals, very large numbers, or extreme exponents, opt in explicitly. For
example, default parsing can round `0.10000000000000001` to `0.1`, underflow
`1e-400` to zero, and reject `1e400`. Signed/unsigned 64-bit integers remain exact.

For Rust, after 0.6.0 is published:

```toml
jsoncompat = { version = "0.6.0", features = ["arbitrary-precision"] }
```

The same feature is available on `json_schema_ast`, `json_schema_fuzz`,
`jsoncompat_codegen`, and `jsoncompat_openapi`. Build the CLI with it too when
using exact schemas:

```bash
cargo install jsoncompat --version 0.6.0 --locked --features arbitrary-precision
```

Python wheels and the default npm/WebAssembly build have precision disabled.
Build the desired binding from the matching release source to enable it:

```bash
# Python: build an exact-precision wheel, then install that wheel.
maturin build --release --manifest-path pybindings/Cargo.toml --features arbitrary-precision

# WebAssembly: use this generated package in place of the default npm build.
wasm-pack build wasm --target web --release --features arbitrary-precision
```

This is a build feature, not a Python extra or runtime option. Use the same
intended numeric policy for generation and runtime validation. Already-rounded
Python floats or JavaScript Numbers cannot recover their original decimal
digits. Cargo features are additive: another dependency can enable arbitrary
precision even when your dependency uses `default-features = false`.

#### 2. Regenerate and deploy the complete output

Upgrade the generator and Python runtime together, then rerun the generation
commands against your original schemas:

```bash
# A standalone schema: deploy models.py and _models_generated.py together.
jsoncompat codegen --target dataclasses schema.json --output models.py

# An existing OpenAPI model package: regenerate the complete directory.
jsoncompat codegen --openapi openapi.json --output my_models
```

Keep generated support files, shared pattern modules, manifests, and `py.typed`
with the package. Update direct imports from numbered modules, for example
`from my_models.models_0000 import Order` becomes
`from my_models.models.order import Order`. Use the names emitted for your schema;
the package's public exports remain available. Regeneration migrates managed
files to the new layout. Check renamed Python fields: a JSON key `__type` now
maps to `field___type`, and `builtins` is now reserved and becomes `builtins_`
(with suffixes if names collide). Their JSON wire names do not change.
If you explicitly named a generated class `builtins`, choose a different class
name; that name is now rejected rather than automatically suffixed.

Replace uses of `Model.__jsoncompat_schema__` with your retained source schema.
For explicit validation, pass the relevant JSON Schema text to
`jsoncompat.validator_for(...)`. An OpenAPI document is a build input containing
multiple contracts, not itself the JSON Schema of each generated model.

#### 3. Update missing-value imports and checks

Import the sentinel from jsoncompat, rather than from a generated model module:

```python
from jsoncompat import MISSING, MissingType, is_missing
```

Use `value is MISSING` / `value is not MISSING`, or `is_missing(value)`, instead
of truthiness. Identity checks narrow optional field types in both branches.
For example, with the [generated Petstore models](examples/openapi_client/petstore/):

```python
from jsoncompat import MISSING
from petstore.models.pet_patch import PetPatch

patch = PetPatch(**{"name": "Mochi"})
assert patch.nickname is MISSING
assert "nickname" not in patch.to_value()

cleared = PetPatch(nickname=None)
assert cleared.to_value() == {"nickname": None}
```

Missing optional fields stay omitted on serialization; explicit `None` stays
JSON `null` where allowed. Missing required fields still fail construction.
Pass `MISSING` to a constructor or `dataclasses.replace` to omit a field again.

#### 4. Adopt HTTP clients where useful

Existing model-only code does not need an HTTP adapter. To add a generated
client, use `--api`, install the chosen transport extra, and bind a session you
own. Start with the [requests, HTTPX, and aiohttp examples](examples/openapi_client/README.md),
including per-call timeouts and expected/unexpected errors. Call `.unwrap()` for
the usual typed success path, or `.result()` to handle declared error variants.
Reading response metadata or bytes does not trigger model parsing.

## [0.5.0] - 2026-10-10

- Added three-way compatibility results: `Compatible`, `Incompatible` with a validated counterexample, and `Unknown`. The CLI exposes structured JSON results and exit code 2 for unknown cases; boolean `check_compat` still returns false when inclusion is not proved.
- Expanded JSON Schema support for exact decimal constraints, unevaluated properties/items, embedded and explicitly supplied resources, anchors, dynamic references, and vocabulary-controlled format assertions. Compatibility proofs and regression fixtures were strengthened across these features.
- Moved generated Python model validation, regex compilation, and conversion preparation into code generation. Import and first use no longer compile schemas; generated modules can use a private companion file or a lazy OpenAPI component package.
- Added structured generated-model validation diagnostics and broader differential tests against the original schemas. Regenerate older models when upgrading to the prepared runtime.
- Restored release packaging and archive validation across the published Rust crates and Python source distribution.

## [0.4.3] - 2026-10-08

- Added Python source distributions containing the local Rust workspace and Python sources, with installation tests from the archive.
- Added free-threaded Python 3.15 wheel builds on Linux and macOS, and provisioned Python 3.15 explicitly for Windows wheel builds. Windows 3.15t wheels remained unsupported.

## [0.4.2] - 2026-08-04

- Added generated-model `from_value()` / `to_value()` and `deserialize()` / `serialize()` APIs, with JSON and optional YAML/MessagePack codecs.
- Moved generated-model construction and conversion into a shared native runtime, added `skip_validation=True`, and stored nested collections as deeply immutable sequences and mappings.
- Expanded Python wheel coverage to Python 3.14, Python 3.15, and free-threaded Python 3.14 where supported.

## [0.4.1] - 2026-06-18

- Fixed Python wheel uploads after the package directory moved to `pybindings`, and made missing wheel artifacts fail CI explicitly.

## [0.4.0] - 2026-06-17

- Added OpenAPI 3.1 contract comparison for operations, parameters, request bodies, response statuses, media types, and response headers, including supported local component references.
- Added schema stamping with versioned writer/reader envelopes and persistent schema history, plus frozen, slotted Python dataclass generation.
- Added reusable Python validators and generators, along with more detailed compatibility explanations and unsupported-feature diagnostics.
- Changed deserializer compatibility to consider values old serializers could emit, rather than every value accepted by the old schema. In particular, serializers are assumed not to emit undeclared optional properties merely because `additionalProperties` allows them.

## [0.3.1] - 2026-04-05

- Added `jsoncompat demo`, including a noninteractive mode, and refreshed CLI installation and usage examples.
- Removed the experimental `canonicalize` CLI subcommand; Rust callers can inspect `SchemaDocument::canonical_schema_json()` instead.
- Updated release automation for standalone CLI binaries across supported platforms.

## [0.3.0] - 2026-04-05

- Refactored the Rust API around `SchemaDocument`, deterministic canonicalization, an immutable resolved schema graph, and typed schema errors.
- Enforced Draft 2020-12 dialect and keyword-shape checks, and strengthened compatibility handling for numeric constraints, tuples, dependencies, conditionals, and recursive references.
- Improved regex- and format-guided example generation, with generated candidates checked against the original schema and bounded retry failures reported explicitly.

## [0.2.6] - 2026-02-03

- Fixed compatibility comparison ordering for schemas containing combinators on both sides, including enums within unions and `oneOf` comparisons against `anyOf` / `allOf`.

## [0.2.5] - 2026-02-03

- Migrated Rust crates to edition 2024 and updated the affected Rust APIs and build configuration.
- Updated macOS release runners and npm trusted publishing, and enabled generation of Windows Python import libraries.

## [0.2.4] - 2026-02-03

- Added `propertyNames` handling to schema parsing, reference resolution, compatibility checks, and generated example values.
- This version has a Git tag but no GitHub release record.

## [0.2.2] - 2025-09-22

- Promoted the Python typing fix from `0.2.2-alpha.1` to the stable release. The adjacent tag trees differ only in package versions and lockfile entries.

## [0.2.2-alpha.1] - 2025-09-22

- Corrected the Python type-stub filename from `jsoncompat_py.pyi` to `jsoncompat.pyi` so it matches the importable module.

## [0.2.1] - 2025-09-18

- Added circular and recursive local-reference support across the schema graph, compatibility checking, and example generation.
- Added a recursive model to the Pydantic golden-schema example.

## [0.2.0] - 2025-07-03

- Fixed WebAssembly example generation for object-property counts by using platform-sized bounds rather than converting an unbounded `u64` maximum to `usize`.
- Updated the Python, WebAssembly, and website fuzzer examples to demonstrate object schemas.

## [0.1.8-alpha.9] - 2025-07-03 (prerelease)

- Changed WebAssembly publication to the web-target package instead of the bundler-target package and configured npm registry authentication.
- Updated the website’s jsoncompat dependency. No compatibility-engine or value-generator changes.

## [0.1.6-alpha.6] - 2025-07-03 (prerelease)

- Aligned the Python module initializer and examples with the `jsoncompat` name.
- Set up Node.js 22 for npm publication.

## [0.1.6-alpha.5] - 2025-07-03 (prerelease)

- Supplied npm authentication to the publishing step. No runtime changes.

## [0.1.6-alpha.4] - 2025-07-03 (prerelease)

- Configured the Python distribution and extension module as `jsoncompat`, with a new Maturin `pyproject.toml` and updated installation instructions.
- Renamed the npm package to `jsoncompat` during publication.

## [0.1.6-alpha.3] - 2025-07-03 (prerelease)

- Adjusted the npm publish directory argument and removed the checked-in generated WebAssembly package manifest. No runtime changes.

## [0.1.6-alpha.2] - 2025-07-03 (prerelease)

- Corrected the downloaded WebAssembly package path used by the npm publishing workflow. No runtime changes.

## [0.1.6-alpha.1] - 2025-07-03 (prerelease)

- Added `jsoncompat ci OLD NEW` to compare schema golden files, report per-schema results as a table or JSON, and fail for invalid or incompatible schemas.
- Added a pull-request example comparing golden files with the merge base, package READMEs, and website usage documentation.

## [0.1.5] - 2025-07-01

- Added a complete Pydantic schema-golden example with a registration decorator, generated golden file, and pytest checks; ran both Python examples in CI.
- Renamed the Python type stub to match the `jsoncompat_py` module and added a developer bootstrap script.

## [0.1.5-alpha.3] - 2025-07-01 (prerelease)

- Ran wheel builds from the Python crate and collected artifacts from `python/dist`.
- Normalized Windows path separators in the fuzz-test whitelist.

## [0.1.5-alpha.2] - 2025-07-01 (prerelease)

- Built multiple Python interpreter wheels within each platform job, restored skipping existing PyPI uploads, and removed the Cargo publishing CI job.

## [0.1.5-alpha.1] - 2025-07-01 (prerelease)

- Added a workspace-wide `just release` dry-run command and updated workspace release versions. No runtime changes.

## [0.1.4] - 2025-07-01

- Aligned workspace crate versions and internal dependencies, marked the WebAssembly crate as npm-only, and added a Cargo publishing workflow.

## [0.1.3] - 2025-07-01

- Updated the root crate version and lockfile to 0.1.3. No behavior changes relative to v0.1.2-a.

## [0.1.2-a] - 2025-07-01 (prerelease)

- Removed explicit PyPI and npm token environment settings from the release workflow. No library or CLI changes.

## [0.1.2] - 2025-07-01 (prerelease)

- Updated the Python wheel workflow to cover Python 3.11–3.13 across Linux, macOS, and Windows, with Linux builds focused on x86-64 and ARM64.
- CI-only changes relative to v0.1.1; GitHub marked this release as a prerelease with the warning “pre-release; do not use.”

## [0.1.1] - 2025-06-30 (prerelease)

- Preserved `if`/`then`/`else` in the schema AST and round trips, resolved references within conditional branches, and generated values from those branches.
- Added schema round-trip coverage and improved the website examples and deployment tooling.
- GitHub marked this release as a prerelease with the warning “pre-release; do not use.”

## [0.1.0] - 2025-04-20

- Initial tagged release: Rust APIs and CLI commands for serializer/deserializer schema compatibility, random JSON generation, and sampled counterexamples.
- Included Python and WebAssembly bindings, example programs, and the browser compatibility checker and value generator.

[0.5.0]: https://github.com/ostrowr/jsoncompat/compare/v0.4.3..v0.5.0
[0.4.3]: https://github.com/ostrowr/jsoncompat/compare/v0.4.2..v0.4.3
[0.4.2]: https://github.com/ostrowr/jsoncompat/compare/v0.4.1..v0.4.2
[0.4.1]: https://github.com/ostrowr/jsoncompat/compare/v0.4.0..v0.4.1
[0.4.0]: https://github.com/ostrowr/jsoncompat/compare/v0.3.1..v0.4.0
[0.3.1]: https://github.com/ostrowr/jsoncompat/compare/v0.3.0..v0.3.1
[0.3.0]: https://github.com/ostrowr/jsoncompat/compare/v0.2.6..v0.3.0
[0.2.6]: https://github.com/ostrowr/jsoncompat/compare/v0.2.5..v0.2.6
[0.2.5]: https://github.com/ostrowr/jsoncompat/compare/v0.2.4..v0.2.5
[0.2.4]: https://github.com/ostrowr/jsoncompat/compare/v0.2.2..v0.2.4
[0.2.2]: https://github.com/ostrowr/jsoncompat/compare/v0.2.2-alpha.1..v0.2.2
[0.2.2-alpha.1]: https://github.com/ostrowr/jsoncompat/compare/v0.2.1..v0.2.2-alpha.1
[0.2.1]: https://github.com/ostrowr/jsoncompat/compare/v0.2.0..v0.2.1
[0.2.0]: https://github.com/ostrowr/jsoncompat/compare/v0.1.8-alpha.9..v0.2.0
[0.1.8-alpha.9]: https://github.com/ostrowr/jsoncompat/compare/v0.1.6-alpha.6..v0.1.8-alpha.9
[0.1.6-alpha.6]: https://github.com/ostrowr/jsoncompat/compare/v0.1.6-alpha.5..v0.1.6-alpha.6
[0.1.6-alpha.5]: https://github.com/ostrowr/jsoncompat/compare/v0.1.6-alpha.4..v0.1.6-alpha.5
[0.1.6-alpha.4]: https://github.com/ostrowr/jsoncompat/compare/v0.1.6-alpha.3..v0.1.6-alpha.4
[0.1.6-alpha.3]: https://github.com/ostrowr/jsoncompat/compare/v0.1.6-alpha.2..v0.1.6-alpha.3
[0.1.6-alpha.2]: https://github.com/ostrowr/jsoncompat/compare/v0.1.6-alpha.1..v0.1.6-alpha.2
[0.1.6-alpha.1]: https://github.com/ostrowr/jsoncompat/compare/v0.1.5..v0.1.6-alpha.1
[0.1.5]: https://github.com/ostrowr/jsoncompat/compare/v0.1.5-alpha.3..v0.1.5
[0.1.5-alpha.3]: https://github.com/ostrowr/jsoncompat/compare/v0.1.5-alpha.2..v0.1.5-alpha.3
[0.1.5-alpha.2]: https://github.com/ostrowr/jsoncompat/compare/v0.1.5-alpha.1..v0.1.5-alpha.2
[0.1.5-alpha.1]: https://github.com/ostrowr/jsoncompat/compare/v0.1.4..v0.1.5-alpha.1
[0.1.4]: https://github.com/ostrowr/jsoncompat/compare/v0.1.3..v0.1.4
[0.1.3]: https://github.com/ostrowr/jsoncompat/compare/v0.1.2-a..v0.1.3
[0.1.2-a]: https://github.com/ostrowr/jsoncompat/compare/v0.1.2..v0.1.2-a
[0.1.2]: https://github.com/ostrowr/jsoncompat/compare/v0.1.1..v0.1.2
[0.1.1]: https://github.com/ostrowr/jsoncompat/compare/v0.1.0..v0.1.1
[0.1.0]: https://github.com/ostrowr/jsoncompat/commits/v0.1.0
