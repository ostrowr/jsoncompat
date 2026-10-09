# JSON Schema keyword support

The target dialect is Draft 2020-12, including OpenAPI 3.1 Schema Objects.
Keyword coverage and complete inclusion proofs are different promises. Validation
checks an instance; compatibility must prove a statement about every instance.
`analyze_compat` returns `Compatible` only after an inclusion proof,
`Incompatible` only with a validated counterexample, and `Unknown` when neither
is established. `check_compat` preserves its boolean API: false means no proof.

| Keywords | Parsing and validation | Canonicalization and compatibility | Generation |
| --- | --- | --- | --- |
| `type`, `enum`, `const` | Supported, with JSON numeric equality | Type and finite-domain proofs | Supported |
| `minimum`, `maximum`, `exclusiveMinimum`, `exclusiveMaximum`, `multipleOf` | Exact decimal assertions, including unsigned 64-bit bounds | Small-number fast path; rational bounds and divisibility outside it | Validated numeric seeds and bounded generation |
| `minLength`, `maxLength`, `pattern` | Supported; backend-compatible regex matching | Length proofs; conservative regex inclusion | Heuristic regex generation and validation |
| `properties`, `patternProperties`, `additionalProperties`, `required`, `propertyNames`, `minProperties`, `maxProperties` | Supported | Structural, finite-domain, and dependency proofs | Heuristic object generation and validation |
| `dependentRequired`, `dependentSchemas` | Supported | Required-name closure and conditional schema application | Heuristic generation and validation |
| `prefixItems`, `items`, `contains`, `minContains`, `maxContains`, `minItems`, `maxItems`, `uniqueItems` | Supported | Tuple, cardinality, finite-domain, and uniqueness proofs | Includes intersecting tuple constraints |
| `allOf`, `anyOf`, `oneOf`, `not`, `if`, `then`, `else` | Supported | Conservative Boolean and conditional proofs | Heuristic generation and validation |
| `unevaluatedProperties`, `unevaluatedItems` | Supported, including successful-branch annotations and `contains` indices | Direct closures use ordinary object/array nodes; composed closures carry evaluated locations. Recursive or large expansions retain executable validation constraints and may yield `Unknown` | Proposes values from the structural graph or a relaxed graph; validates every result |
| `$ref`, `$id`, `$anchor`, `$dynamicRef`, `$dynamicAnchor`, `$defs` | Embedded and explicitly supplied resources; relative URI resolution and dynamic scope | Resource instances become local graph references; expansion is bounded | Uses the resolved graph |
| `$schema`, `$vocabulary` | Standard dialects, plus caller-supplied Draft 2020-12 metaschemas through `SchemaOptions` | Required unknown vocabularies are rejected. Vocabulary-selected format assertions are retained | Follows the document's validation mode |
| `format` | Annotation by default. Assertions through the format-assertion vocabulary or `SchemaOptions.assert_formats` | Asserted formats retain exact membership checks; general format-language inclusion can be unknown | Format hints plus validation; no completeness guarantee |
| `contentEncoding`, `contentMediaType`, `contentSchema` | Annotations, without automatic decoding or embedded-content validation | No change to the containing instance language | No obligation to generate decodable content |
| `title`, `description`, `default`, `examples`, `deprecated`, `readOnly`, `writeOnly`, `$comment` | Annotation shapes validated | No instance assertion; identity/code-generation metadata is preserved where needed | Defaults do not manufacture required values |

Legacy `definitions` remains a reference container and `dependencies` retains
property/schema assertions, without contributing modern evaluated-location annotations. `additionalItems` is not an assertion in Draft
2020-12. Older dialects, tuple-form `items`, and OpenAPI 3.0 `nullable` are not
reinterpreted as Draft 2020-12 keywords.

## Resources and formats

```rust
use jsoncompat::{SchemaDocument, SchemaOptions};
use serde_json::json;

let mut options = SchemaOptions::default();
options.resources.insert(
    "https://example.com/value".into(),
    json!({"type": "string", "format": "email"}),
);
options.assert_formats = Some(true);
let document = SchemaDocument::from_json_with_options(
    &json!({"$ref": "https://example.com/value"}),
    &options,
).unwrap();
```

No network fetches are performed. Supply referenced resources explicitly or
embed them under `$defs`. Missing resources and conflicting identifiers produce
errors. Custom dialect resources must declare Draft 2020-12 core and known
vocabularies; unknown required vocabularies fail. Assertion mode rejects unknown
format names rather than silently ignoring them. Nested resources select their
own dialect. Options-based construction materializes supplied resources into the
source document. Canonical output uses the reserved extension
`x-jsoncompat-format-assertion` to retain format assertions across serialization.

## Practical limits

JSON input preserves arbitrary-precision decimal numbers. Numeric assertions
compare their decimal values exactly. Python floats have already been rounded;
the value API validates their serialized decimal representation. A wire decimal
can be valid yet round across a bound during Python conversion: deserialization
accepts the original value, and checked serialization rejects an invalid rounded
value. It cannot recover digits lost before the API was called.

Unevaluated-location expansion is capped at 128 cases and a depth of 64.
Resource/dynamic-scope expansion is capped at 4,096 graph instances. General
regex inclusion and recursive annotation reasoning remain conservative. Value
generation limits structural work to 16,384 node visits per candidate by default
(configurable with `GenerationConfig::with_max_candidate_nodes`). Generation and counterexample search are bounded heuristics, so `Unknown` is an
expected result, not evidence of incompatibility.

OpenAPI contract lowering supports numeric and unevaluated assertions, content
annotations, and local component references. It still rejects resource-scoping
keywords (`$id`, `$anchor`, `$dynamicRef`, `$dynamicAnchor`) inside contracts;
relocating an independently scoped schema into a request/response envelope needs
additional reference handling. Raw SchemaDocument support is broader than this
OpenAPI lowering surface.

The fixtures audit labels independently, compare raw/canonical/IR membership,
exercise both compatibility directions, and cross-check claimed inclusion
against composed witness spaces. They are a regression corpus, not a proof of
exhaustiveness over every possible schema.

Generated Python dataclasses resolve embedded resource identifiers, anchors, and
dynamic scopes during code generation. Inline guards and the prepared general
validator share no runtime schema compiler. Asserted formats/custom vocabularies,
unavailable resources, and unsupported regex operations fail during generation;
they are never silently converted to annotations. Standard `format` remains an
annotation. See [the Python runtime details](pybindings/README.md).

The differential E2E suite additionally compares each generated fixture and an
adversarial schema-composition matrix against validation of the original schema.
It tests string/bytes decoding, Python-value construction, and checked encoding
separately, with inline guards enabled and forcibly removed. Test-only poisoned
programs verify that both paths are actually exercised. Hand-labeled witnesses,
seeded mutations, exhaustive bounded regex strings, deep invalid model mutations,
large schemas, and large values supplement the existing fixture corpus.
