# `jsoncompat_openapi`

`jsoncompat_openapi` owns the OpenAPI-specific part of `jsoncompat`:

- validating the supported OpenAPI 3.1 document shape before any compatibility work begins;
- lowering supported OpenAPI contracts into ordinary JSON Schema documents that the core checker already understands.

The crate intentionally stops at that boundary. It does not decide whether a contract change is backward-compatible. The root `jsoncompat` crate compares the lowered request and response schemas with its normal subset checker.

## What validation means

`OpenApiDocument::from_json` accepts JSON OpenAPI 3.1 documents and validates document-shape invariants without deciding whether the document is lowerable by `jsoncompat`. The root crate's `validate_openapi_compatibility_input` helper then performs the full compatibility-readiness pass before any comparison begins.

The constructor verifies, before lowering:

- the document root is an object;
- `openapi` exists and starts with `3.1.`;
- `info.title` and `info.version` are strings;
- common `info` metadata is shape-checked even though it does not affect the compatibility verdict;
- at least one of `paths`, `components`, or `webhooks` exists;
- `paths`, when present, is an object;
- `components`, when present, is an object;
- `webhooks`, when present, is an object;
- `jsonSchemaDialect`, when present, is an absolute URI;
- common document-level metadata, server, security, and tag containers are shape-checked;
- document-level tag names are unique, as required by OpenAPI 3.1;
- path-item and webhook entry containers are objects before any lowerability decision is made;
- path-item parameter arrays plus operation `parameters`, `requestBody`, and `responses` containers have the right structural type before lowering;
- inline parameter entries require the core OpenAPI contract shape (`name`, `in`, path-parameter requiredness, and exactly one of `schema` or `content`);
- inline parameter serialization metadata is checked against the parameter location before lowering, and content-backed parameters reject schema-only serialization/example fields while requiring exactly one concrete media type;
- path templates line up with inline and locally resolvable path parameters before lowering, and locally resolvable parameter arrays reject duplicate parameter identities before lowering;
- request-body media-type encoding keys must name directly declared inline or locally referenced media-schema properties before lowering;
- inline request bodies must expose `content`, and inline response maps must use valid OpenAPI status selectors plus at least one real response whose inline Response Object has a `description`;
- response-header objects reject query-only fields and case-insensitive duplicate names up front; content-backed headers reject schema-only serialization/example fields and require exactly one concrete media type;
- parameter, request-body, response, response-header, security-scheme, media-type, encoding, callback, response-link, and example metadata objects are shape-checked before lowerability is considered; Link Object `operationId` targets must resolve to an existing operation in the document, and local Link Object `operationRef` fragments must resolve to an existing Operation Object, `encoding` is only valid on request-body `multipart/*` or `application/x-www-form-urlencoded` media types, and encoding keys must name directly declared inline media-schema properties before lowering;
- schema metadata such as `externalDocs`, `xml`, `discriminator`, `readOnly`, and `writeOnly` is shape-checked everywhere those Schema Objects already appear in the document traversal, including `components.schemas`, before lowerability is considered; `xml` metadata is rejected outside property schemas;
- supported-dialect component schema documents, schema roots under component parameters, request bodies, responses, and headers, and inline contract schemas under `paths`, `webhooks`, callbacks, and component `pathItems`, are validated before lowering whenever that can be done without resolving reference features intentionally owned by the later lowering phase; malformed inputs keep pinpoint OpenAPI-source diagnostics, while valid-but-unsupported reference or compatibility features still defer to compatibility-readiness validation;
- `components` only names OpenAPI component collections or `x-*` extensions;
- component collections that OpenAPI models as maps are objects, their keys satisfy OpenAPI's component-name pattern, and structurally malformed contract entries are rejected before any later valid-but-unmodeled surface error;
- `paths` keys use valid OpenAPI path-template syntax, and equivalent templated paths such as `/pets/{id}` versus `/pets/{name}` are rejected up front;
- operation identifiers are globally unique across paths, webhooks, callback operations, and component path-item operations;
- root-, path-operation-, webhook-operation-, callback-operation-, and component-path-item-operation security requirements name declared `components.securitySchemes` entries;

Lowering performs the remaining contract-aware validation before producing any schemas:

- document-level `jsonSchemaDialect` values are restricted to the dialects the lowered JSON Schema layer explicitly supports;
- contract-bearing document fields that are valid OpenAPI but not yet modeled, such as `webhooks`, are rejected instead of being silently ignored;
- every supported local reference resolves, and reference cycles are rejected;
- unsupported remote references fail immediately;
- media-type map keys that collapse to the same compatibility selector after jsoncompat's parameter/casing normalization are rejected instead of approximated;
- unused contract components are still validated, so malformed specs do not pass merely because the malformed object is never referenced;
- valid-but-unsupported callbacks, response links, component collections, and media-type encoding blocks are rejected only after their document shape has already been validated;
- unsupported OpenAPI contract surfaces fail explicitly instead of being approximated.

This means `jsoncompat compat old.json new.json` fails invalid or unsupported OpenAPI inputs while validating each file individually, before it attempts compatibility comparison.

## The lowering contract

Lowering is a semantics-preserving translation from the supported OpenAPI contract surface into synthetic JSON Schema envelopes.

Each path operation becomes two JSON Schema documents:

1. a request envelope schema;
2. a response envelope schema.

The compatibility layer then compares:

- requests in the deserializer direction, because servers need to keep accepting requests they previously accepted;
- responses in the serializer direction, because clients need to keep accepting responses the server may now emit.

### Request envelopes

A lowered request schema is an object with some or all of:

- `path`
- `query`
- `headers`
- `cookies`
- `body`

Parameters are grouped by location and lowered into object properties. The translation preserves:

- parameter identity;
- requiredness;
- schema-vs-content form;
- serialization controls that materially affect the contract:
  - `style`
  - `explode`
  - `allowReserved`
  - `allowEmptyValue`

Path parameters are required both by OpenAPI and by the lowered object schema. Operation-level parameters override path-item parameters with the same OpenAPI identity.

Request bodies lower to a tagged contract over the accepted media variants and their schemas. Required request bodies remain required in the lowered request envelope.

### Response envelopes

A lowered response schema is an object with:

- `status`
- `body`
- `headers`

Responses preserve the OpenAPI selector that made them reachable:

- exact status codes such as `200`;
- response classes such as `2XX`;
- `default`.

Media-type variants are represented explicitly. Concrete media types and OpenAPI media-type ranges such as `image/*` and `*/*` lower into structured type/subtype contracts so widening and narrowing stay directional in the ordinary subset checker. When exact and ranged entries overlap, the lowering preserves OpenAPI's most-specific-match rule instead of treating the entries as a plain union. Media-type parameters are accepted but normalized away for compatibility, matching the current checker scope. Adding a new response media variant still expands what the server may serialize, so the root compatibility checker can correctly treat that as a serializer-side risk.

Response headers lower into object properties with their requiredness and schema/content shape preserved.
Header identities are canonicalized case-insensitively, matching HTTP semantics, and duplicate
case-insensitive response header names are rejected before lowering.

### References and component schemas

The lowerer supports local OpenAPI references through the comparison surface:

- parameters;
- request bodies;
- responses;
- headers;
- schema references rooted at `#/components/schemas/...`.

Referenced component schemas are copied into the lowered JSON Schema `$defs` set only when needed, including their transitive schema dependencies. That keeps generated envelopes smaller without weakening validation: every component collection is still checked up front.

### Dialects

Lowered request and response schemas inherit the document-level schema dialect:

- `https://json-schema.org/draft/2020-12/schema`
- `https://spec.openapis.org/oas/3.1/dialect/base`

The lowerer does not attempt to reinterpret OpenAPI 3.0-era schema behavior. For example, `nullable` is not modeled as a shortcut for a JSON Schema union.

## Compatibility-neutral surfaces

The loader accepts and shape-checks common OpenAPI fields that do not participate in jsoncompat's value-language verdict:

- document-level `servers`, `security`, `tags`, and `externalDocs`;
- `info.summary`, `info.description`, `info.termsOfService`, `info.contact`, and `info.license`, with contact email strings validated as email addresses;
- path-item `servers`, `summary`, and `description`;
- operation `security`, `servers`, `tags`, `summary`, `description`, `externalDocs`, `operationId`, and `deprecated`;
- `components.securitySchemes`.
- parameter `description`, `deprecated`, `example`, and `examples`;
- request-body `description`;
- response-header `description`, `deprecated`, `example`, and `examples`;
- media-type `example` and `examples`.
- Reference Object `$ref` URI references, `summary`, and `description`, plus any other sibling properties that OpenAPI 3.1 says consumers should ignore.
- schema `externalDocs` and property-schema `xml` metadata, which are shape-checked as OpenAPI metadata and remain outside the value-language verdict. URL-valued OpenAPI metadata is validated as URL references, including balanced server URL templates; URI-valued metadata such as `jsonSchemaDialect`, example `externalValue`, and link `operationRef` is validated as URI syntax requires, with local `operationRef` fragments additionally resolved against in-document operations; XML namespaces must be absolute URIs, and `xml.wrapped` is rejected when an explicit sibling schema `type` excludes `array`.
- schema `discriminator` objects, which are shape-checked as OpenAPI metadata, including the requirement that they be adjacent to `oneOf`, `anyOf`, or `allOf`, but not lowered into the value-language contract.
- schema `readOnly` and `writeOnly` annotations, which are shape-checked as booleans and remain outside the value-language verdict.

Those fields remain present in the source document, but they are not lowered into request or response envelopes.

## Explicitly unsupported surfaces

The crate rejects OpenAPI fields that could affect compatibility but are not yet modeled. That includes, among others:

- document-level `webhooks`;
- path-item `$ref` entries;
- operation `callbacks`;
- response `links`;
- unsupported component collections such as `examples`, `links`, `callbacks`, and `pathItems`;
- media-type `encoding`;
- content maps whose media-type keys collapse to the same compatibility selector after normalization;
- schema keywords whose compatibility semantics are not yet modeled here: JSON Schema `$id`, `$anchor`, `$dynamicRef`, `$dynamicAnchor`, `additionalItems`, `contentEncoding`, `contentMediaType`, `contentSchema`, `dependencies`, `dependentSchemas`, `unevaluatedItems`, and `unevaluatedProperties`;
- number-schema bounds outside the adjacent-integer-safe `f64` range `[-9007199254740991, 9007199254740991]`, because the compatibility layer refuses rounded subset proofs there;
- remote references.

Those rejections are deliberate. The library would rather fail early than silently claim compatibility while skipping a contract surface it does not yet understand.

## Why this crate exists

Keeping validation and lowering in a dedicated crate creates a strict abstraction boundary:

- `jsoncompat_openapi` understands OpenAPI documents and produces JSON Schema envelopes;
- `json_schema_ast` understands JSON Schema documents and builds the canonical schema graph;
- `jsoncompat` understands compatibility over already-validated schema documents.

That separation keeps the OpenAPI-specific code narrow, makes fail-fast validation easier to audit, and prevents OpenAPI concerns from leaking into the subset engine.
