[![jsoncompat logo](web/jsoncompatdotcom/public/logo192.png)](https://jsoncompat.com)

# jsoncompat

[![crates.io](https://img.shields.io/crates/v/jsoncompat)](https://crates.io/crates/jsoncompat) [![docs.rs](https://docs.rs/jsoncompat/badge.svg)](https://docs.rs/jsoncompat) [![PyPI](https://img.shields.io/pypi/v/jsoncompat.svg)](https://pypi.org/project/jsoncompat/) [![npm](https://img.shields.io/npm/v/jsoncompat.svg)](https://www.npmjs.com/package/jsoncompat) [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Check compatibility of evolving JSON schemas.

jsoncompat currently supports JSON Schema Draft 2020-12 only. If a schema declares `$schema`, it
must be `https://json-schema.org/draft/2020-12/schema` with an optional trailing `#`.

> [!WARNING]
> Docs and examples at [jsoncompat.com](https://jsoncompat.com)
>
> This is alpha software. Not all incompatible changes are detected, and there may be false positives. Contributions are welcome!

Imagine you have an API that returns some JSON data, or JSON that you're storing in a database or file. You need to ensure that new code can read old data and that old code can read new data.

It's difficult to version JSON schemas in a traditional sense, because they can break in two directions:

1. If a schema is used by the party generating the data, or "serializer", then a change to the schema that can break clients using an older version of the schema should be considered "breaking." For example, removing a required property from a serializer schema should be considered a breaking change for a schema with the serializer role.

More formally, consider a serializer schema $S_A$ which is changed to $S_B$. This change should be considered breaking if there exists some JSON value that is valid against $S_B$ but invalid against $S_A$.

As a concrete example, if you're a webserver that returns JSON data with the following schema:

```json
{
  "type": "object",
  "properties": {
    "id": { "type": "integer" },
    "name": { "type": "string" }
  },
  "required": ["id", "name"]
}
```

and you make `name` optional:

```json
{
  "type": "object",
  "properties": {
    "id": { "type": "integer" },
    "name": { "type": "string" }
  },
  "required": ["id"]
}
```

then you've made a breaking change for any client that is using the old schema.

We assume that the serializer will not write additional properties that are not in the schema, even if additionalProperties is true. This allows us to consider a change to the schema that adds an optional property of some type not to be a breaking change.

1. If a schema is used by a party receiving the data, or "deserializer", then a change to the schema that might fail to deserialize existing data should be considered "breaking." For example, adding a required property to a deserializer should be considered a breaking change.

More formally, consider a deserializer schema $S_A$ which is changed to $S_B$. This change should be considered breaking if there exists some JSON value that is valid against $S_A$ but invalid against $S_B$.

As a concrete example, imagine that you've been writing code that saves JSON data to a database with the following schema:

```json
{
  "type": "object",
  "properties": {
    "id": { "type": "integer" },
    "name": { "type": "string" }
  },
  "required": ["id"]
}
```

and you make `name` required, attempting to load that data into memory by deserializing it with the following schema:

```json
{
  "type": "object",
  "properties": {
    "id": { "type": "integer" },
    "name": { "type": "string" }
  },
  "required": ["id", "name"]
}
```

you'll be unable to deserialize any data that doesn't have a `name` property, which is a breaking change for the `deserializer` role.

If a schema is used by both a serializer and a deserializer, then a change to the schema that can break either should be considered "breaking."

## Support checklist

Legend:

- ✅: the backcompat checker has an explicit subset rule, or the fuzzer has direct
  schema-guided candidate construction for that keyword.
- 🟡: the feature is parsed/evaluated, but the checker is conservative or the fuzzer is
  heuristic/retry-based for that keyword.
- ⚪: the keyword is accepted or stripped, but it is not represented in the resolved IR and does
  not contribute checker/fuzzer semantics.
- ⛔: schema construction fails with a typed error before the checker/fuzzer runs.

Important global caveats:

- `jsoncompat::check_compat` is a structural subset checker over the resolved AST. It can return
  false negatives for conservative subset proofs and false positives for string-language
  constraints that are currently ignored when both sides are open string schemas.
- For serializer compatibility, the checker assumes serializers do not emit undeclared extra
  properties, even when `additionalProperties: true`.
- `json_schema_fuzz::generate_value` walks the resolved AST heuristically, then validates every
  candidate with the raw `jsonschema` backend. If no valid candidate is found within the retry
  budget, it returns `GenerateError::ExhaustedAttempts`.
- Boolean schema documents `true` and `false` are supported directly. `false` has no valid
  instances, so generation returns `ExhaustedAttempts` if no branch can satisfy it.
- Unknown extension keywords are preserved only when needed to keep local ref targets addressable;
  otherwise they are ignored by both the checker and the fuzzer.

### Scalar schemas

| Keyword | Compat | Fuzz | Notes |
| --- | :-: | :-: | --- |
| `type` | ✅ | ✅ | Supports `null`, `boolean`, `object`, `array`, `string`, `number`, `integer`, and unions of those primitive types. |
| `enum` | ✅ | ✅ | Numeric equality follows JSON Schema semantics, so `1` and `1.0` are treated as equal. |
| `const` | ✅ | ✅ | Same semantic numeric equality as `enum`. |
| `minimum` | ✅ | ✅ | Integer bounds are normalized exactly where possible; number bounds use `f64`. |
| `maximum` | ✅ | ✅ | Integer bounds are normalized exactly where possible; number bounds use `f64`. |
| `exclusiveMinimum` | ✅ | 🟡 | Integer exclusives are canonicalized into inclusive integer bounds; number generation may rely on validator retries around open lower bounds. |
| `exclusiveMaximum` | ✅ | 🟡 | Integer exclusives are canonicalized into inclusive integer bounds; number generation may rely on validator retries around open upper bounds. |
| `multipleOf` | ✅ | ✅ | Exact integer divisibility is used when both sides are integer-valued. Fractional integer divisors are preserved and projected to their implied integer divisor for integer instances; non-integer numeric checks use `f64` ratios. |
| `minLength` | ✅ | 🟡 | The generic string generator respects this bound, but the `format` and `pattern` paths may ignore it and rely on validator retries. |
| `maxLength` | ✅ | 🟡 | The generic string generator respects this bound, but the `format` and `pattern` paths may ignore it and rely on validator retries. |
| `pattern` | 🟡 | 🟡 | `ResolvedNode::accepts_value()` enforces regexes for finite-value checks, but `check_compat` does not prove regex-language inclusion between open string schemas. Regex generation is best-effort and does not guarantee coverage for complex constructs. |
| `format` | 🟡 | 🟡 | The checker does not prove format-language inclusion between open string schemas. The fuzzer only synthesizes `date`, `date-time`, `time`, `email`, `idn-email`, `uri`, `iri`, `uri-reference`, `iri-reference`, `uuid`, `ipv4`, `ipv6`, `hostname`, and `idn-hostname`; unknown formats fall back to generic strings. |
| `contentEncoding` | ⚪ | ⚪ | Canonicalized for syntax, then stripped from the semantic IR. |
| `contentMediaType` | ⚪ | ⚪ | Canonicalized for syntax, then stripped from the semantic IR. |
| `contentSchema` | ⚪ | ⚪ | Nested schemas are canonicalized and ref-checked, but content decoding/validation is not modeled in the semantic IR. |

### Object schemas

| Keyword | Compat | Fuzz | Notes |
| --- | :-: | :-: | --- |
| `properties` | ✅ | ✅ | Property schemas are compared recursively and generated directly. |
| `required` | ✅ | ✅ | Missing required properties are synthesized before generation returns a candidate. |
| `additionalProperties` | ✅ | ✅ | Checker support is subject to the serializer assumption above; the fuzzer generates additional keys only when this schema is not `false`. |
| `patternProperties` | 🟡 | 🟡 | Exact same regex keys are compared structurally, and property values are checked against every matching pattern schema. If the superset has no `patternProperties`, subset patterns fall back to `additionalProperties`; otherwise the checker does not prove regex-language inclusion between different patterns. The fuzzer does not synthesize property names from regexes. |
| `propertyNames` | 🟡 | 🟡 | Checker support depends on subset reasoning for the property-name schema itself, so regex/format caveats still apply. The fuzzer can generate keys from string/enum schemas and otherwise falls back to random names plus acceptance checks. |
| `minProperties` | ✅ | ✅ | Canonicalization raises `minProperties` to at least the number of required keys. |
| `maxProperties` | ✅ | ✅ | Checked structurally and used as a hard upper bound during generation. |
| `dependentRequired` | ✅ | 🟡 | The checker accounts for trigger keys admitted by `properties`, `patternProperties`, or `additionalProperties`. The fuzzer does not proactively synthesize dependencies, but invalid candidates are rejected by the final validator pass. |
| `dependentSchemas` | ⚪ | ⚪ | Nested schemas are canonicalized for dialect/ref validation, but `ResolvedNodeKind::Object` does not store `dependentSchemas`, so checker/fuzzer semantics are absent. |
| `unevaluatedProperties` | ⚪ | ⚪ | Canonicalized recursively but not represented with evaluated-location bookkeeping in the resolved IR. |
| `dependencies` | ⚪ | ⚪ | Legacy keyword accepted by the parser but not represented in the resolved IR; use `dependentRequired` / `dependentSchemas` for Draft 2020-12 semantics. |

### Array schemas

| Keyword | Compat | Fuzz | Notes |
| --- | :-: | :-: | --- |
| `items` | ✅ | ✅ | Schema-form `items` is compared recursively and used for tail-item generation. Legacy tuple-form `items: [...]` is parsed by appending entries into `prefixItems`; use `prefixItems` for explicit Draft 2020-12 tuple schemas. |
| `prefixItems` | ✅ | ✅ | Compared positionally and generated positionally. |
| `additionalItems` | ⚪ | ⚪ | Legacy keyword not represented in the resolved array node, so it does not affect compatibility or generation. |
| `minItems` | ✅ | ✅ | Defaults to `minContains` (or `1`) when `contains` is present, otherwise `0`. |
| `maxItems` | ✅ | ✅ | Checked structurally and used as a hard upper bound during generation. |
| `contains` | ✅ | 🟡 | Checker reasoning handles item-match count constraints structurally. The fuzzer tries to force or avoid matching items, but this is heuristic and may rely on retries. |
| `minContains` | ✅ | 🟡 | Participates in structural count reasoning for `contains`; the fuzzer uses best-effort witness construction plus validator retries. |
| `maxContains` | ✅ | 🟡 | Participates in structural count reasoning for `contains`; the fuzzer uses best-effort witness avoidance plus validator retries. |
| `uniqueItems` | ✅ | 🟡 | The checker handles the common sound cases; the fuzzer retries and then uses a synthetic fallback object for unconstrained item schemas if duplicates persist. |
| `unevaluatedItems` | ⚪ | ⚪ | Canonicalized recursively but not represented with evaluated-location bookkeeping in the resolved IR. |

### Applicators and conditionals

| Keyword | Compat | Fuzz | Notes |
| --- | :-: | :-: | --- |
| `allOf` | 🟡 | 🟡 | Canonicalization simplifies trivial boolean branches. The checker is branch-local and can miss proofs such as `allOf[A, B] <= A` when only one branch is inspected. The fuzzer merges object branches heuristically and otherwise picks one non-trivial branch. |
| `anyOf` | 🟡 | 🟡 | The checker requires every `sub` branch to fit `sup`, and for `sup anyOf` it looks for one branch containing all of `sub`; that is conservative and can miss valid disjunctive proofs. The fuzzer samples one branch and retries. |
| `oneOf` | 🟡 | 🟡 | Canonicalization rewrites provably disjoint `oneOf` branches to `anyOf`. Outside that case, checker reasoning is conservative and does not symbolically prove exact-one exclusivity, and the fuzzer uses retry-based witness counting. |
| `not` | 🟡 | 🟡 | The checker only handles shallow boolean/`Any` cases symbolically; other `not` proofs fall back to conservative `false`. The fuzzer generates type-mismatch and fixed candidate witnesses, then retries against the full schema. |
| `if` | 🟡 | 🟡 | `ResolvedNode::accepts_value()` evaluates the condition and the fuzzer samples branches, but the subset checker has no dedicated symbolic rule for conditional implication beyond exact-node equality and finite `const`/`enum` probes. |
| `then` | 🟡 | 🟡 | Stored and evaluated only as part of `if` / `then` / `else`; checker reasoning inherits the same conditional caveat as `if`. |
| `else` | 🟡 | 🟡 | Stored and evaluated only as part of `if` / `then` / `else`; checker reasoning inherits the same conditional caveat as `if`. |

### Document shape, references, and metadata

| Keyword | Compat | Fuzz | Notes |
| --- | :-: | :-: | --- |
| `$ref` | ✅ | 🟡 | Same-document refs to `"#"` and `"#/..."` are supported, including recursive graphs. Pure alias cycles and non-local refs are rejected with typed resolver errors. Generation is depth-limited. |
| `$anchor` | ⛔ | ⛔ | Plain-name fragment anchors are rejected; only JSON Pointer refs are supported today. |
| `$dynamicRef` | ⛔ | ⛔ | Dynamic refs are rejected during schema resolution. |
| `$dynamicAnchor` | ⛔ | ⛔ | Dynamic anchors are rejected during schema resolution. |
| `$id` | ⛔ | ⛔ | Resource-scope identifiers are rejected because reference resolution is currently same-document only. |
| `$defs` | ✅ | 🟡 | Acts as a local ref target container; nested schemas are canonicalized and resolved, but `$defs` is not a semantic node by itself. |
| `definitions` | ✅ | 🟡 | Legacy ref target container with the same caveat as `$defs`. |
| `title` | ⚪ | ⚪ | Preserved as metadata in canonical JSON, but not used by checker/fuzzer semantics. |
| `$comment` | ⚪ | ⚪ | Stripped or preserved only as metadata; no compatibility or generation semantics. |
| `description` | ⚪ | ⚪ | Stripped or preserved only as metadata; no compatibility or generation semantics. |
| `default` | ⚪ | ⚪ | Stripped or preserved only as metadata; default values are not synthesized or used for compatibility. |
| `examples` | ⚪ | ⚪ | Stripped or preserved only as metadata; examples do not guide fuzzing. |
| `deprecated` | ⚪ | ⚪ | Annotation only; ignored by checker and fuzzer semantics. |
| `readOnly` | ⚪ | ⚪ | Annotation only; ignored by checker and fuzzer semantics. |
| `writeOnly` | ⚪ | ⚪ | Annotation only; ignored by checker and fuzzer semantics. |
| `$vocabulary` | ⚪ | ⚪ | Annotation only; no dialect negotiation semantics are modeled. |

## Rust workspace architecture

The Rust code is split into five crates and one CLI binary. The website under `web/` is a separate frontend and is not part of the compatibility engine.

| Path | Package | Responsibility |
| --- | --- | --- |
| `schema/` | `json_schema_ast` | Draft 2020-12 dialect checks, schema canonicalization, AST construction, local `$ref` resolution, and direct validator compilation via `jsonschema` |
| `src/` | `jsoncompat` | Backward-compatibility checking over resolved ASTs, plus the `jsoncompat` CLI in `src/bin/jsoncompat.rs` |
| `fuzz/` | `json_schema_fuzz` | Schema-guided JSON instance generation and random schema generation for fuzz tests |
| `python/` | `jsoncompat_py` | PyO3 bindings that expose `check_compat` and `generate_value` |
| `wasm/` | `jsoncompat_wasm` | `wasm-bindgen` bindings that expose `check_compat` and `generate_value` to JavaScript |

The two core schema APIs are:

- `json_schema_ast::compile(&raw_schema)`, which compiles the original schema document directly with the `jsonschema` validator backend. Before compilation, this crate rejects any `$schema` declaration other than Draft 2020-12 (`https://json-schema.org/draft/2020-12/schema`, with an optional trailing `#`).
- `json_schema_ast::ResolvedSchema::from_json(&raw_schema)`, which eagerly stores the original raw schema JSON and lazily materializes derived views on first use:
  - `ResolvedSchema::is_valid(&value) -> Result<bool, SchemaBuildError>` lazily compiles a validator from the original raw schema document.
  - `ResolvedSchema::is_valid_canonicalized(&value) -> Result<bool, SchemaBuildError>` lazily canonicalizes the schema, compiles a validator from the canonicalized JSON, and exists to test/debug whether canonicalization preserved semantics.
  - `ResolvedSchema::root() -> Result<&ResolvedNode, SchemaBuildError>` lazily canonicalizes the schema, resolves local refs, and exposes the immutable canonicalized graph used by compatibility analysis and fuzzing/codegen.
  - `ResolvedSchema::raw_schema_json()` returns the original raw schema document.
  - `ResolvedSchema::canonical_schema_json() -> Result<&Value, SchemaBuildError>` lazily returns the canonicalized schema document as JSON so humans can inspect the rewrite directly.
    `ResolvedNodeKind` only contains post-resolution semantic variants; parser-only `$ref` and declaration metadata variants are private implementation details.

At a high level, the runtime flow is:

```mermaid
flowchart TD
    raw["Raw JSON Schema document"]
    compile_raw["json_schema_ast::compile(raw)\nlazily compile raw validator"]
    validate_raw["ResolvedSchema::is_valid(value)\nvalidate against raw schema"]
    canonicalize["schema/src/canonicalize.rs\nlazily canonicalize syntax and fill implicit constraints"]
    canonical_json["ResolvedSchema::canonical_schema_json()\ndebug canonical JSON output"]
    compile_canonical["compile(canonicalized)\nparity/debug validator"]
    validate_canonical["ResolvedSchema::is_valid_canonicalized(value)\nvalidate canonicalized schema"]
    parse["schema/src/ast.rs\nparse canonical JSON into a private mutable graph"]
    resolve["schema/src/ast.rs\nresolve local # / #/... refs and freeze to ResolvedNodeKind"]
    compat["src/subset.rs\njsoncompat::check_compat(old_root, new_root, role)"]
    fuzz["fuzz/src/lib.rs\njson_schema_fuzz::generate_value(&ResolvedSchema)"]

    raw --> compile_raw --> validate_raw
    raw --> canonicalize --> parse --> resolve
    canonicalize --> canonical_json
    canonicalize --> compile_canonical --> validate_canonical
    resolve --> compat
    resolve --> fuzz
```

### What each crate owns

- `json_schema_ast` is the schema frontend and resolved IR crate. `schema/src/ast.rs` stores the raw input schema immediately, and lazily asks `schema/src/canonicalize.rs` for a deterministic canonical form only when a caller requests `root()`, `canonical_schema_json()`, or `is_valid_canonicalized()`. The resolved graph is then built by parsing that canonical JSON into a private mutable parse graph, resolving local recursive references, and freezing it into `ResolvedNodeKind`. Unsupported or non-productive refs fail with typed resolver errors. `ResolvedSchema::is_valid()` is intentionally backed by the validator compiled from the original raw schema. `ResolvedSchema::raw_schema_json()` preserves the original input schema for debugging and comparisons. `ResolvedNode::accepts_value()` is the internal evaluator for canonicalized AST subgraphs used by compatibility/fuzzing heuristics. `ResolvedNode::to_json()` remains a debug/test helper and should not be used as a production semantic bridge.
- `jsoncompat` is the static compatibility checker. `src/lib.rs` defines `Role` and `check_compat`, and `src/subset.rs` implements the actual inclusion relation (`sub ⊆ sup`) over `ResolvedNode`. The checker now uses `ResolvedNode::accepts_value()` for finite-value membership checks and keeps a cycle guard for recursive subset proofs.
- `json_schema_fuzz` is the value-generation engine. Its public APIs take `ResolvedSchema` and only return values accepted by `ResolvedSchema::is_valid()`; if the internal candidate generator cannot find such a value within its retry budget, generation returns a typed `GenerateError::ExhaustedAttempts`. Internally it walks the canonicalized `ResolvedNode` graph and uses `ResolvedNode::accepts_value()` only as a pruning heuristic for recursive subgraphs.
- `jsoncompat_py` and `jsoncompat_wasm` are thin adapters. They parse JSON strings, call the Rust core crates, and map Rust errors/results into Python or JavaScript types.

### Test strategy

- `tests/backcompat.rs` checks expected serializer/deserializer compatibility outcomes for hand-authored old/new schema pairs, then fuzzes each direction to look for concrete counterexamples.
- `tests/fuzz.rs` runs the JSON-Schema-Test-Suite fixture corpus through `ResolvedSchema::from_json`, asks `json_schema_fuzz::ValueGenerator` for raw-valid examples, and asserts that `ResolvedSchema::is_valid_canonicalized()` returns the same answer on those instances. Fixtures that rely on unsupported reference-resource features are skipped when schema build returns a typed resolver error, and known generation gaps are tracked by explicit `GenerateError::ExhaustedAttempts` whitelist entries.
- `schema/src/canonicalize/integration_tests.rs` and `schema/src/roundtrip_tests.rs` test canonicalization and AST round-tripping directly.

## Debugging canonicalization

To inspect the canonicalized schema document that backs compatibility checks and generation:

```bash
jsoncompat canonicalize schema.json --pretty
```

For programmatic checks, compare `ResolvedSchema::raw_schema_json()` with `ResolvedSchema::canonical_schema_json()`, and compare `ResolvedSchema::is_valid(value)` with `ResolvedSchema::is_valid_canonicalized(value)` on representative instances.

## Development

Requirements:

Run [bootstrap.sh](bootstrap.sh) to install the necessary dependencies.

Run tests:

```bash
just check
```

Run the performance benchmark harnesses:

```bash
just bench
```

The schema operation benchmarks use a fixed handpicked corpus under
[benches/fixtures](benches/fixtures) so broad fuzz fixture changes do not move
the benchmark baseline.

For a fast smoke check of the benchmark binary:

```bash
just bench-check
```

See the [Justfile](Justfile) for more commands

## Releasing

`just release` will dry-run the release process for a patch release.

Right now, releases to PyPI and npm are done in CI via manual dispatch of the `CI` workflow
on a tag. Releases to cargo are done manually for now.

Merging to main will trigger a release of the website.
