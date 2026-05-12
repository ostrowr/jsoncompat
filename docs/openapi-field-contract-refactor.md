# OpenAPI Field Contract Refactor

## Motivation

`src/openapi.rs` currently reuses `Parameter` to lower both request parameters
and response headers into the synthetic schemas consumed by the compatibility
engine.

That reuse keeps the lowerer compact, but it makes one important state
representable that should not be:

- response headers are materialized as `Parameter { location: Header, ... }`;
- header lowering fills parameter-only fields such as `allow_reserved` and
  `allow_empty_value` with synthetic `false` values.

Those values are implementation artifacts, not OpenAPI response-header
semantics. The code is still understandable, but the boundary is slightly
dishonest and will get worse if the OpenAPI surface grows.

The OpenAPI 3.1 [Header Object](https://spec.openapis.org/oas/v3.1.2.html#header-object)
follows the Parameter Object shape only with header-specific restrictions:

- `allowEmptyValue` and `allowReserved` must not be used for headers;
- header `style`, when present, must be `"simple"`.

The refactor should align the internal model with those constraints rather than
preserving the current synthetic defaults.

## Goals

1. Remove the fake “response headers are parameters” representation.
2. Preserve the current request-parameter compatibility semantics.
3. Keep response-header compatibility semantics explicit and statically
   representable.
4. Avoid widening the public API or changing the document-level OpenAPI
   compatibility entrypoints.
5. Keep the lowered-schema comparison model intact: parse OpenAPI, lower into
   request/response envelope schemas, then reuse the ordinary compatibility
   checker.

## Non-goals

- Do not redesign the whole OpenAPI lowerer.
- Do not add OpenAPI 3.0 semantics.
- Do not change operation matching, media-type handling, or component-ref
  resolution.
- Do not solve original-document source pointers for OpenAPI explanations in
  this refactor.

## Proposed Internal Model

Keep `Parameter` as the parsed request-parameter representation.

Introduce a smaller private lowered representation for fields that appear inside
synthetic request/response envelope schemas. The stronger target shape should
encode both the field source and whether the source uses `schema`-based or
`content`-based serialization:

```rust
struct ContractField {
    name: String,
    required: bool,
    value: FieldValue,
}

enum FieldValue {
    Schema {
        schema: Value,
        serialization: SchemaSerialization,
    },
    Content {
        media_schema: Value,
    },
}

enum SchemaSerialization {
    PathParameter {
        style: String,
        explode: bool,
    },
    QueryParameter {
        style: String,
        explode: bool,
        allow_reserved: bool,
        allow_empty_value: bool,
    },
    Header {
        // Header style is implicitly/simple-only in OpenAPI 3.1.
        explode: bool,
    },
    CookieParameter {
        style: String,
        explode: bool,
    },
}
```

The key points are:

- a response header can never carry `allow_reserved` or `allow_empty_value`;
- non-query request parameters cannot carry query-only metadata either;
- `content`-based fields do not accidentally retain `style` / `explode`
  metadata that belongs to `schema`-based serialization;
- header style can be tightened further in implementation so only `"simple"`
  is representable, instead of carrying an arbitrary `String`.

## Lowering Changes

### Request parameters

1. Parse OpenAPI parameters into the existing `Parameter` type.
2. Convert each parsed `Parameter` into `ContractField`.
3. Lower parameter groups from `ContractField` values instead of directly from
   `Parameter`.

This keeps request parameter parsing and path/operation override logic exactly
where it is today. If the parser remains permissive while this refactor lands,
the design should explicitly document that only the lowering model is being
tightened in this step; the better end state is for illegal location-specific
metadata to be rejected earlier too.

### Response headers

1. Resolve each `Header Object | Reference Object` before constructing the
   lowered field.
2. Parse each OpenAPI response header directly into `ContractField`.
3. Use header-specific serialization rather than manufacturing a synthetic
   `Parameter`.
4. Preserve current header name normalization and requiredness semantics.

### Synthetic schema builder

Replace:

```rust
fn parameter_contract_schema(parameter: &Parameter) -> Value
```

with:

```rust
fn contract_field_schema(field: &ContractField) -> Value
```

The schema builder should emit serialization metadata according to `FieldValue`
and `SchemaSerialization` rather than assuming every field has parameter-only
metadata.

## Open Design Choice

There are two viable lowered-schema shapes:

### Option A: Semantically faithful shapes

Emit different synthetic metadata properties for parameters and headers:

- query parameters include `allow_reserved` and `allow_empty_value`;
- non-query parameters do not include query-only metadata;
- headers do not include them at all.

Pros:

- truthful to the OpenAPI model;
- makes impossible states structurally impossible in the lowered schema too;
- easier to reason about future additions.

Cons:

- response-header message paths and fixture expectations may change.

### Option B: Uniform compatibility envelope

Keep the current lowered JSON shape for both field kinds, but produce it from
typed `ContractField` input rather than fake `Parameter` values. Headers would
still serialize synthetic default metadata into the envelope.

Pros:

- minimal fixture churn;
- keeps lowered request/response field envelopes visually uniform.

Cons:

- retains semantic noise in the comparison schema;
- weaker payoff for the refactor.

## Recommendation

Choose **Option A** unless preserving exact synthetic response-header schema
shape is especially valuable.

The compatibility checker already handles heterogeneous envelope schemas well.
This codebase generally benefits more from truthful typed boundaries than from
uniformity that encodes non-domain values.

The implementation should also decide whether this refactor is the right place
to reject illegal OpenAPI input such as:

- response headers with `allowReserved` or `allowEmptyValue`;
- response headers with non-`"simple"` `style`;
- non-query parameters carrying query-only metadata.

Rejecting those during parsing would line up with the typed model. Deferring the
parser tightening is acceptable only if it is called out as a deliberately
separate follow-up.

## Implementation Sequence

1. Add `ContractField`, `FieldValue`, and `SchemaSerialization`.
2. Add a conversion/helper for request parameters:

   ```rust
   impl From<&Parameter> for ContractField
   ```

3. Extract response-header parsing into a small helper that returns
   `ContractField`.
4. Change `lower_parameter_group(...)` to work with lowered fields, or to map
   `Parameter -> ContractField` at the boundary.
5. Replace `parameter_contract_schema(...)` with
   `contract_field_schema(...)`.
6. Preserve header reference resolution, existing header requiredness, and
   header-name normalization.
7. Decide whether to tighten parser validation for illegal location-specific
   metadata in the same change or in a tracked follow-up.
8. Update fixture expectations only where Option A intentionally changes the
   lowered schema path/message shape.

## Test Plan

Add or tighten tests so the typed boundary is visible in behavior:

1. Request-parameter lowering still treats `allowEmptyValue` changes as
   compatibility-relevant.
2. Response-header lowering never emits parameter-only metadata fields.
3. Existing response-header schema broadening and requiredness tests remain
   green.
4. Existing query-parameter add/remove/requiredness tests remain green.
5. OpenAPI compatibility fixture messages remain pinned after the refactor.
6. A ref-based response-header case continues to lower correctly.
7. A `content`-based response-header case exercises the `schema` vs `content`
   split.

If Option A is chosen, add a focused assertion over the lowered response-header
schema shape or over an incompatibility message path that proves parameter-only
metadata is absent from the header envelope. Add at least one fixture-pinned
response-header incompatibility message so pointer/message drift is detected in
the normal fixture flow.

If parser tightening is included, add negative tests for:

- non-`"simple"` header styles;
- header `allowReserved` / `allowEmptyValue`;
- query-only metadata on non-query request parameters.

## Expected Outcome

After the refactor:

- request parameters remain modeled as request parameters;
- response headers are modeled as response headers;
- the lowered comparison schema stops carrying fake header metadata;
- the OpenAPI compatibility surface is easier to extend without repeating the
  current representational shortcut.
