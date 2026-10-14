# Public OpenAPI support follow-up

The previous audit's **26 rejected native OpenAPI 3.1 documents are not 26 missing features**. Their first blockers divide into **13 generation/transport gaps**, **nine invalid document shapes or references**, **three undefined serialization/discriminator arrangements**, and **one conflict with HTTP response semantics**. This classification uses the exact snapshots and SHA-256 hashes in the [corpus manifest](public-openapi-manifest.json); a newer upstream revision may differ.

The follow-up now passes **29 complete OpenAPI 3.1 specifications, 1,423 operations, and 3,171 imported modules**, up from 16 specifications and 624 operations. All 13 documents blocked by generation or transport gaps pass in full, without changing their source contracts. The remaining 13 native documents are rejected by the current validation policy; the proposed client-generation policy below distinguishes harmless metadata from missing wire contracts. Fourteen older-version controls remain outside the OpenAPI 3.1 boundary.

The [machine-readable results](public-openapi-support-results.json) record source hashes, individual outcomes, file sizes, and validation evidence. Each passing client has deterministic generation, valid manifest hashes, imports of every module, matching generated/reflected HTTP dispatch plans, zero HTTP annotation reflection at import, strict `ty` checks, and an intentionally invalid consumer that `ty` rejects. Source schemas and versions are never patched to obtain a pass.

The tables below classify the original first blockers. Enabling media support also exposed and fixed private-field name mangling and scalar `text/plain` bodies; those findings are included in the complete-spec rerun.

## Generation and transport gaps

OpenAPI permits request and response media beyond JSON. The [Media Type Object](https://spec.openapis.org/oas/v3.1.0.html#media-type-object) and [Encoding Object](https://spec.openapis.org/oas/v3.1.0.html#encoding-object) define the form and multipart contract; query flags come from the [Parameter Object](https://spec.openapis.org/oas/v3.1.2.html#parameter-object). Support must preserve the declared body schema and media parameters rather than changing the spec to fit a JSON-only client.

| Snapshot | First source location | Missing capability |
| --- | --- | --- |
| [agent_protocol](https://raw.githubusercontent.com/langchain-ai/agent-protocol/440a86ba097b4bc41e8da007b24e419ec07a3e35/openapi.json) | `#/paths/~1runs~1stream/post/responses/200/content/text~1event-stream` | Finite buffered event-stream text; an unbounded streaming API is not added. |
| [landing_ai_ade](https://raw.githubusercontent.com/landing-ai/ade-cli/d1995ffef4e63a2a1c2774d9add93368889353c5/openapi.json) | `#/paths/~1v2~1parse/post/requestBody/content/multipart~1form-data` | Multipart request parts, including binary files. |
| [airflow_simple_auth](https://raw.githubusercontent.com/apache/airflow/9fc4143efc5eba3b0436122ff7505bc516e8f80a/airflow-core/src/airflow/api_fastapi/auth/managers/simple/openapi/v2-simple-auth-manager-generated.yaml) | `#/paths/~1auth~1token/post/requestBody/content/application~1x-www-form-urlencoded` | URL-encoded object fields. |
| [aivisspeech](https://raw.githubusercontent.com/Aivis-Project/AivisSpeech/77a3675f7b76865cfa0253ac30f41ef1dd822e27/openapi.json) | `#/paths/~1/get/responses/200/content/text~1html` | General text and binary response bodies; more than one media type is affected. |
| [papr_memory](https://raw.githubusercontent.com/Papr-ai/memory-opensource/a0a816eb00f73b0819ef3fd7115bb6835cd18c39/openapi.json) | `#/paths/~1v1~1document~1v2/post/requestBody/content/multipart~1form-data` | Multipart request bodies and binary fields. |
| [airflow](https://raw.githubusercontent.com/apache/airflow/9fc4143efc5eba3b0436122ff7505bc516e8f80a/airflow-core/src/airflow/api_fastapi/core_api/openapi/v2-rest-api-generated.yaml) | `#/paths/~1api~1v2~1dags~1{dag_id}~1dagRuns~1{dag_run_id}~1wait/get/responses/200/content/application~1x-ndjson` | Newline-delimited JSON record streams. |
| [meilisearch](https://raw.githubusercontent.com/meilisearch/documentation/0d96f60225eabdfed0721ab95e39b3605a764bed/assets/release-assets/meilisearch-openapi.json) | `#/paths/~1batches~1stream/get/responses/200/content/application~1x-ndjson` | JSON record streams plus CSV bodies. |
| [gravit_launcher](https://raw.githubusercontent.com/GravitLauncher/Launcher/eda714f1ebc9f8cd04a3ed7bf31940c8c1c9b23f/openapi.yaml) | `#/paths/~1updates~1upload~1{variant}/post/requestBody/content/multipart~1form-data` | Per-part contentType (JSON secrets and binary image), plus multipart encoding metadata. |
| [nomad](https://nomad-lab.eu/prod/v1/api/v1/openapi.json) | `#/paths/~1entries~1raw~1query/post/responses/200/content/application~1zip` | Opaque binary download bodies. |
| [fossology](https://raw.githubusercontent.com/fossology/fossology/d94e7fa191c8a1f1d46abdf4b207896212c9ca3f/src/www/ui/api/documentation/openapi.yaml) | `#/paths/~1license~1export-csv/get/responses/200/content/text~1csv` | CSV/text and other binary media. |
| [optimade](https://raw.githubusercontent.com/Materials-Consortia/optimade-python-tools/main/openapi/openapi.json) | `#/paths/~1versions/get/responses/200/content/text~1csv; header=present` | Media parameters retained in requests and response selection. |
| [jupyterhub](https://raw.githubusercontent.com/jupyterhub/jupyterhub/main/docs/source/_static/rest-api.yml) | `#/paths/~1users/get/parameters/3` | Query allowReserved / allowEmptyValue serialization. |
| [typesense_cloud](https://cloud.typesense.org/api/v1/openapi.json) | `#/paths/~1invoices~1{id}~1pdf/get/responses/200/content/application~1pdf` | Opaque binary response bodies. |

Media support does not imply unbounded streaming: the existing adapters buffer a response before returning it, then parsing is lazy. An event stream can therefore only be consumed as a finite buffered response with this API. Caller-owned transport timeouts and cancellation still apply.

## Inputs rejected by current document and contract checks

| Snapshot | Source location and observed value | Why it remains rejected |
| --- | --- | --- |
| [polar](https://raw.githubusercontent.com/polarsource/polar/2a1cb969fb21e2c9811c094069351089736fb172/docs/openapi/2026-10.openapi.json) | `#/paths/~1v1~1benefits~1/get/parameters/8`: `style: deepObject` with `explode` omitted; its description explicitly shows `?metadata[key]=value`. | The parser applies the non-form default `explode: false`; the [parameter serialization table](https://spec.openapis.org/oas/v3.1.2.html#parameter-object) leaves that combination undefined. The source does not explicitly request false. Its schema also permits array-valued properties, whose deepObject encoding is undefined. |
| [openresponses](https://raw.githubusercontent.com/openresponses/openresponses/7078a8f1aecd3d1cd41c9891e21c307fcda7f4af/public/openapi/openapi.json) | `#/components/schemas/Annotation`: A single `$ref` to `UrlCitationBody` with `discriminator`, without a composition. | This is outside the defined [discriminator configurations](https://spec.openapis.org/oas/v3.1.2.html#conditions-for-using-the-discriminator-object). It is not the supported parent/child `allOf` inheritance form. Four other schemas have the same arrangement. |
| [chroma_core](https://raw.githubusercontent.com/chroma-core/chroma/5f30478c11ffdcf8cba1f872993c29d8f84ce0d9/docs/mintlify/sync.openapi.json) | `#/paths/~1api~1v1~1invocations/get/parameters/2/schema`: Reference to missing `#/components/schemas/SourceTypeFilter`. | A [schema reference](https://json-schema.org/draft/2020-12/json-schema-core#section-8.2.3.1) must resolve; no replacement schema can be inferred safely. |
| [cohere_ai](https://raw.githubusercontent.com/cohere-ai/cohere-developer-experience/57596d8d2a8df5e3cdeb072cdc602c17805c72d4/cohere-openapi.yaml) | `#/components/schemas/TruncationStrategy/oneOf`: `oneOf: []`. | The [applicator meta-schema](https://raw.githubusercontent.com/json-schema-org/json-schema-spec/2020-12/meta/applicator.json) requires one or more `oneOf` branches. The original report incorrectly called this an empty enum. Empty `enum` is valid in Draft 2020-12; this source actually has an empty `oneOf`. |
| [marimo](https://raw.githubusercontent.com/marimo-team/marimo/feaf0b2464be52fb43005d8622708d0af89cf645/packages/openapi/api.yaml) | `#/info/version`: Required field is absent; `info` contains only `title`. | [Info.version](https://spec.openapis.org/oas/v3.1.0.html#info-object) is required and must be a string; the current document validator requires it even though client generation does not use it. |
| [openai](https://raw.githubusercontent.com/openai/openai-openapi/77618e641a840768f1e58197ca53ddedffea6df9/openapi.json) | `#/components/schemas/BetaCodeInterpreterToolCall/properties/outputs/anyOf/0/discriminator`: Discriminator on an array; its `items` already has a union and discriminator. | The array-level discriminator is outside the defined [discriminator configurations](https://spec.openapis.org/oas/v3.1.2.html#conditions-for-using-the-discriminator-object). `CodeInterpreterToolCall` has the same redundant array-level declaration. |
| [opensearch](https://github.com/opensearch-project/opensearch-api-specification/releases/download/v0.2.0/opensearch-openapi.yaml) | `#/paths/~1_plugins~1_query~1_datasources~1{datasource_name}/delete/responses/204`: References `#/components/responses/query.datasource_delete___204`, which declares a JSON object body. | A 204 response has no content under [RFC 9110 section 15.3.5](https://www.rfc-editor.org/rfc/rfc9110.html#section-15.3.5). The current generator rejects the contradictory declaration; a future client projection can give HTTP semantics precedence and report that the body annotation was ignored. |
| [intelligencex](https://raw.githubusercontent.com/IntelligenceX/SDK/867124360296531330b6fc8fabafaa78bfea4303/openapi.yaml) | `#/paths/~1api~1item~1get/get/responses`: The field is absent (as it is on six other operations). | The [Responses Object](https://spec.openapis.org/oas/v3.1.0.html#responses-object) requires at least one response code. |
| [langflow](https://raw.githubusercontent.com/langflow-ai/langflow/504c02fc47e76087b82b0e7cbe4186e9cdd916d4/docs/openapi/openapi.json) | `#/paths/~1api~1v1~1monitor~1messages~1session~1{session_id}`: A second path template with the same shape but different parameter names. | The [Paths Object](https://spec.openapis.org/oas/v3.1.0.html#paths-object) forbids equivalent templated paths with different names. |
| [mcp_registry](https://raw.githubusercontent.com/modelcontextprotocol/registry/11b7b0e274d518b6f449c5ee3518a6151d9848b9/docs/reference/api/openapi.yaml) | `#/$id`: `$id` on the OpenAPI document root. | `$id` is not a field of the [OpenAPI Object](https://spec.openapis.org/oas/v3.1.0.html#openapi-object), whose extensions use `x-`. A Schema Object may contain `$id`; the OpenAPI document root is not itself a Schema Object. |
| [materials_project](https://api.materialsproject.org/openapi.json) | `#/components/securitySchemes/ApiKeyAuth/descriptions`: Unknown `descriptions` field. | The [Security Scheme Object](https://spec.openapis.org/oas/v3.1.0.html#security-scheme-object) uses singular `description`; arbitrary extensions must start with `x-`. |
| [coolify](https://raw.githubusercontent.com/coollabsio/coolify/v4.x/openapi.yaml) | `#/paths/~1applications~1{uuid}~1previews~1{pull_request_id}/patch/responses/403`: Reference to missing `#/components/responses/403`. | The [Reference Object](https://spec.openapis.org/oas/v3.1.0.html#reference-object) cannot supply the missing response status/body/header contract. |
| [openrailwaymap](https://raw.githubusercontent.com/hiddewie/OpenRailwayMap-vector/master/proxy/api/openapi.yaml) | `#/paths/~1api~1status/get/responses`: The field is absent. | The [Responses Object](https://spec.openapis.org/oas/v3.1.0.html#responses-object) requires at least one response code. |

The parser continues to validate unused components, so removing an unrelated operation does not hide an invalid component. Neither the corpus harness nor code generation automatically patches these sources. The older OpenAPI 3.0 / Swagger 2.0 controls are separate from these 26 and remain outside the OpenAPI 3.1 boundary.

## Generation safety vs document conformance

The following is a **proposed policy, not implemented behavior**. The audit counts above describe the current strict implementation. A client can safely tolerate some document defects while retaining schema validation and accurate Python types. Compatibility proofs and explicit document validation should continue to report malformed contracts; accepting a document for client generation need not certify OpenAPI conformance.

| Snapshot | Proposed client-generation policy | Boundary and remaining risk |
| --- | --- | --- |
| Marimo | Accept missing or malformed `info.version` as unusable descriptive metadata; omit it from generated descriptions. | No request or response contract depends on this field. The pinned source has a missing version, not a numeric version. |
| Materials Project | Ignore the unknown `descriptions` annotation with a diagnostic. | Retain and validate `type: apiKey`, `in: header`, and `name: X-API-KEY`; do not apply the same tolerance to missing security mechanics. |
| MCP Registry | Ignore `$id` on the OpenAPI document root with a diagnostic. | Its references are local JSON pointers. Do not reinterpret this stray field as a base URI; `$id` inside actual Schema Objects must retain its scope semantics. |
| OpenResponses | Ignore the five misplaced discriminator hints and validate each underlying `$ref` or composition normally. | These hints add no schema alternatives. Never invent mapped subtypes or weaken the referenced schema. |
| OpenAI | Ignore the two array-level discriminator hints; retain the discriminated unions inside `items`. | Payload validation and nested model selection still use the complete schema. |
| Langflow | Preserve the two path-alias pairs as separate operations. | Each pair has disjoint methods: PATCH uses `{old_session_id}`, DELETE uses `{session_id}`. There is no outbound method/path ambiguity. Conflicting contracts for the same normalized method and path need separate assessment. |
| IntelligenceX | Generate the seven operations with absent responses as explicitly lacking a typed response contract. | Buffered status, headers, and bytes remain useful. Do not infer a success schema or silently treat arbitrary 2xx bodies as validated. |
| OpenRailwayMap | Apply the same explicit raw-response treatment to `GET /api/status`. | The other operations can retain their declared response types. |
| OpenSearch | Generate the declared 204 response as bodyless, retaining status and headers, and diagnose the contradictory body annotation. | HTTP defines the absence of content. Do not manufacture the declared object or hide content received from a nonconforming transport. |
| Cohere | Generate operation-reachable schemas; the malformed `TruncationStrategy` component is unreferenced and need not block those operations. | Diagnose the skipped component. If explicitly requested as a generated model, reject it; neither an unconstrained value nor an inferred intended variant is justified. |
| Polar | For omitted `explode` on `deepObject`, consider a documented bracket-encoding convention with a diagnostic; preserve rejection of explicit false. | The source description confirms bracket notation, but `MetadataQuery` also allows array-valued properties and null. Flat scalar values have a usable contract; the other values still need defined serialization. This change alone would not support the entire declared input domain. |
| Chroma Core | Limit missing-schema failures to the affected reachable request contract. | Four optional query parameters reference missing `SourceTypeFilter` or `OrderBy` across two GET operations. Do not replace them with `Any`, guess enums, or silently drop supplied values; the nine unrelated operations can remain usable. |
| Coolify | Keep the one affected PATCH operation's request and resolved responses; treat the unresolved 403 response as unknown. | The missing reference is response-only. `UnexpectedResponse` can preserve that error's bytes, status, and headers without claiming a typed body. Do not fabricate the referenced schema. |

Discriminator hints must not change validation results under the [OpenAPI discriminator rules](https://spec.openapis.org/oas/v3.1.2.html#discriminator-object). Likewise, route-template restrictions primarily affect routing: a generated client already knows which operation and HTTP method the caller selected. A blanket rule that every nonconforming document is unsafe would reject useful clients unnecessarily.

Missing success contracts need deliberate runtime behavior. Currently, `UnexpectedResponse` handles non-2xx failures; an undeclared 2xx status raises `ResponseError` when parsing is requested, while buffered metadata and content remain accessible. Allowing absent response lists therefore also requires removing their parser/binder rejection and defining truthful aliases (for example, `Never` for the known success type), or explicitly introducing an untyped success envelope. It is not equivalent to inventing a default JSON response.

These proposals address the observed first blockers, not a promise that all 13 complete specifications will then generate. The source inspection found the Polar serialization limitation and Chroma's second missing schema without modifying any source documents. Further diagnostics must still be evaluated as they become reachable.

## Multipart metadata and compatibility proofs

The Rust operation contract now preserves an optional per-property encoding map. Each entry records the content type, normalized style/explode/allowReserved settings, whether those controls were explicitly set, and resolved part-header schemas. Explicit controls matter because OpenAPI gives them precedence over contentType. The Content-Type entry in a part-header map is ignored as required by the specification.

Code generation can consume that metadata. Compatibility lowering still rejects an explicit encoding map: comparing decoded value schemas alone would not prove that a changed multipart/form encoding accepts or emits the same wire representation. This retains the existing soundness boundary.

## Defects found after enabling media

Papr Memory's `ParsePointer` and Airflow's `TimeDelta` expose a `__type` property. Python mangled that generated attribute while slot and converter metadata expected its original spelling, preventing module import. Model fields beginning with `__` now receive the same `field_` prefix already used for special dunder names. The existing allocator assigns deterministic suffixes when that spelling collides with a real property. Wire keys remain unchanged. A regression checks private fields, special methods, collision cases, JSON round trips, HTTP query/header encoding, prepared dispatch, and strict `ty` typing.

## Media evidence

The [explicit media cases](public-openapi-media-samples.json) exercise real pinned operations using labeled synthetic wire payloads: finite SSE, HTML, parameterized CSV, PDF bytes, YAML, NDJSON, a URL-encoded body variant, and multipart JSON plus binary parts. They also check malformed successes and unexpected binary errors. Reading status/headers/content remains lazy; decoding results and failures are cached, and native/value paths agree. [Strict consumer checks](public-openapi-media-consumer.py) assert that PDF bodies are `bytes` and SSE/CSV bodies are `str`.

Automatic inline examples use the generated codec instead of assuming JSON from the media label. Original media keys and referenced response locations remain in source pointers even when generated Content-Type parameters are quoted or reordered. Binary examples without explicit wire bytes and wildcard examples without a concrete media type are recorded as uncovered, with reasons. NDJSON schema examples represent one record, including when that record is an array.

Run the media cases after the complete corpus audit:

```sh
env -u VIRTUAL_ENV uv run --no-config --project pybindings --all-extras --locked python \
  pybindings/check_public_openapi_samples.py /tmp/public-openapi \
  --samples pybindings/benchmarks/public-openapi-media-samples.json
```

The runner writes `explicit-samples.json`; retain or rename an earlier run's report before running another manifest against the same audit directory.

All **74 explicit cases** passed against the complete final clients: 13 media cases, 30 original raw-client cases, and 31 earlier focused cases rerun using the complete Meilisearch and AivisSpeech packages. Their three strict typing consumers also passed.

## Rejected upstream examples

The complete audit exercised 832 automatic cases (649 inline examples and 183 bodyless responses): 329 `Ok`, 450 declared `Err`, and 53 rejected payloads. An independent `python-jsonschema` 4.26.0 Draft 2020-12 check confirmed that all 53 rejected payloads violate their original source schemas: Unkey accounts for 36, Luma for nine, and Papr Memory for eight. The eight additional failures introduced by the newly supported documents are all `application/json` examples; they are schema/example inconsistencies, not media codec failures.

| Papr operation and response | Original example | Violated source constraint |
| --- | --- | --- |
| `GET /v1/user` 200; `POST /v1/user/batch` 200 | `#/components/schemas/UserListResponse/example` | `data[0]` lacks required `code` and `status`. |
| `POST /v1/memory` 200 and 207 | Each response's `content/application~1json/example` | `data[0]` lacks required `memoryId` and `objectId`; `content`, `id`, `metadata`, and `type` are forbidden additional properties. |
| `POST /v1/memory/search` 200 | `system_nodes_response`, `custom_schema_response`, and `mixed_nodes_response` | `data.memories[0]` lacks required `type`, `acl`, and `user_id`. |
| `POST /v1/schemas` 201 | Response `content/application~1json/example` | `data.relationship_types.PURCHASED` lacks required `label`. |

These are eight operation outcomes from seven distinct source example locations because `UserListResponse/example` is reused. Source examples and schemas were preserved unchanged.
