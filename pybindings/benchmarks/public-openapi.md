# Public OpenAPI corpus audit

Tested **56 public specifications: 42 native OpenAPI 3.1 documents and 14 older-version controls**. After the fixes below, **16 complete, unmodified clients (624 operations)** pass all generation, import, and typing checks. Two separately recorded JSON/text operation slices add **175 operations**. These are local schema/client tests, not live service E2E tests.

The remaining 26 native 3.1 documents are rejected by current media/contract support or strict document validation. A successful corpus run is not a claim that all public APIs are supported. Each rejection records the first diagnostic; additional blockers may exist.

[Pinned sources and hashes](public-openapi-manifest.json) · [Results and diagnostics](public-openapi-results.json) · [Explicit runtime cases](public-openapi-samples.json) · [Slice cases](public-openapi-slice-samples.json)

## Defects fixed

- Generated requests, responses, aliases, nested definitions, and inline helpers now avoid existing component names. Explicit component identities stay intact; generated names receive deterministic numeric suffixes. This unblocked Unkey, LSProxy, Renegade, Luma, and Asterwise.
- HTTP metadata and model constructors share reserved Python field naming. Planetary Computer’s `collections` query parameter now generates a working `collections_` field and sends the original wire name.
- Response `content: {}` now means no declared body, matching omitted content. Request-body and parameter/header cardinality checks remain intact. Agent Protocol and JupyterHub advance past this defect, but still encounter other unsupported features.
- A single-reference `allOf` with annotation-only siblings keeps the referenced Python type and class identity. Unkey’s `App.git` is now `Omittable[AppGit]`, removing 29 redundant helpers. Sibling assertions, multiple branches, and resource scopes retain the conservative path and original validation.

## What passed

Every complete client preserves the operation count, imports every generated Python module, produces byte-identical output on repeat generation, and passes manifest hash checks. All prepared dispatch plans equal reflective preparation, with no HTTP annotation reflection during API import. Strict `ty 0.0.86` checks every generated file plus synchronous/asynchronous request, result, unwrap, and API-boundary consumers; negative consumers must fail.

File sizes include source, stubs, and generated metadata. Generation and source-import times are single observations on macOS ARM64 / Python 3.12.2; they are not controlled benchmarks.

| Complete raw client | Operations | Python files | Total MiB | Largest `.py` MiB | Generate s | API import s |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| [unkey](https://raw.githubusercontent.com/unkeyed/unkey/b5a373236aa5a7627a59104d4d2b9ffeb494c829/svc/api/openapi/openapi-generated.yaml) | 100 | 376 | 5.57 | 2.44 | 0.256 | 0.213 |
| [airflow_edge](https://raw.githubusercontent.com/apache/airflow/9fc4143efc5eba3b0436122ff7505bc516e8f80a/providers/edge3/src/airflow/providers/edge3/worker_api/v2-edge-generated.yaml) | 18 | 37 | 0.35 | 0.17 | 0.049 | 0.021 |
| [airflow_fab_auth](https://raw.githubusercontent.com/apache/airflow/9fc4143efc5eba3b0436122ff7505bc516e8f80a/providers/fab/src/airflow/providers/fab/auth_manager/api_fastapi/openapi/v2-fab-auth-manager-generated.yaml) | 14 | 27 | 0.29 | 0.17 | 0.050 | 0.018 |
| [airflow_keycloak_auth](https://raw.githubusercontent.com/apache/airflow/9fc4143efc5eba3b0436122ff7505bc516e8f80a/providers/keycloak/src/airflow/providers/keycloak/auth_manager/openapi/v2-keycloak-auth-manager-generated.yaml) | 6 | 19 | 0.09 | 0.04 | 0.044 | 0.007 |
| [gematik_push_gateway](https://raw.githubusercontent.com/gematik/gem-push-notifications-concept/e24377cb87111f294e0562e58fd6b55bba66d2e5/docs_sources/push_gateway_openapi.yaml) | 3 | 22 | 0.20 | 0.06 | 0.049 | 0.011 |
| [lsproxy](https://raw.githubusercontent.com/agentic-labs/lsproxy/0b7d96cacd8f5f782bf71fcef4b3bc4a347e8af2/openapi.json) | 8 | 36 | 0.21 | 0.06 | 0.044 | 0.011 |
| [mantrae](https://raw.githubusercontent.com/MizuchiLabs/mantrae/10e1ee319e7e2ac5fbab1db8bf14ba275c72c024/internal/api/server/openapi/openapi.json) | 84 | 179 | 1.89 | 0.10 | 0.145 | 0.083 |
| [prefect](https://raw.githubusercontent.com/PrefectHQ/prefect/302beb37eb79cd6ba37fcaa9fa002c4429b45702/docs/v3/api-ref/rest-api/server/schema.json) | 179 | 441 | 64.84 | 31.48 | 1.296 | 0.745 |
| [renegade](https://raw.githubusercontent.com/renegade-fi/renegade/744b3abd8e8814acd20e4520740d6f124a9c8c71/openapi.yaml) | 23 | 85 | 0.59 | 0.18 | 0.079 | 0.027 |
| [userlist_push](https://raw.githubusercontent.com/userlist/userlist-openapi/bad400a7b54737ce7398b04dfd2d2b21b89ad350/reference/push.v1.yaml) | 7 | 25 | 0.22 | 0.03 | 0.050 | 0.009 |
| [grafana_slo](https://raw.githubusercontent.com/grafana/slo-openapi-client/75fe08b09d871ab45cb4a5eeca91dbbe43c66672/openapi.yaml) | 10 | 42 | 0.26 | 0.12 | 0.048 | 0.014 |
| [finmind](https://raw.githubusercontent.com/FinMind/FinMind.github.io/master/openapi.yaml) | 4 | 14 | 0.06 | 0.01 | 0.040 | 0.004 |
| [typesafe](https://api.typesafe.ai/openapi.json) | 2 | 27 | 0.14 | 0.02 | 0.046 | 0.008 |
| [asterwise](https://api.asterwise.com/openapi.json) | 143 | 494 | 7.80 | 3.43 | 0.541 | 0.417 |
| [planetary_computer](https://planetarycomputer.microsoft.com/api/stac/v1/openapi.json) | 10 | 46 | 1.29 | 0.46 | 0.090 | 0.027 |
| [luma](https://raw.githubusercontent.com/lumalabs/lumaai-api/main/openapi.yaml) | 13 | 46 | 0.28 | 0.10 | 0.051 | 0.015 |

## Declared errors and non-object bodies

Across the complete clients, 267 inline/no-body examples were exercised: **99 `Ok`, 123 typed `Err`, and 45 rejected examples**. Native and ordinary value conversion agreed in every case. The rejected examples are disclosed separately: python-jsonschema 4.26.0 independently confirms that all 36 Unkey and nine Luma examples violate their original schemas. Unkey examples contain forbidden extra fields, omit required fields, or use null for a non-null field; Luma examples use `failure_reason: null` where the schema requires a string. No constraints were relaxed to accept them.

**61 explicit runtime cases pass** (30 on complete raw clients, 31 on operation slices). Synthetic bodies are labeled as synthetic in the manifests. Coverage includes:

- JSON string, boolean, integer, number, array, and explicit JSON null response roots; string/array request roots; nullable and discriminated union variants.
- Declared 400, 401, 403, 404, 409, 422, 500, and 503 responses, including `detail` fields accepting either strings or objects.
- Typed errors through `.result()`, `.unwrap()` raising with the same error, undeclared 503s, and malformed declared errors becoming `UnexpectedResponse` with bytes, text/JSON access, status, and duplicate headers intact.
- No parsing on buffered metadata access; decoding and failures cached across `.result()` and `.unwrap()`; invalid success JSON raises only when parsing is requested.
- Invalid primitive/array bodies, exact body-type inference, asynchronous inference, and expected-error narrowing. A bodyless 204 is recorded separately from a JSON `null` response.

Not every response has an inline example, so runtime coverage is not exhaustive. Static typing and dispatch equality cover every operation in every successfully generated client. The explicit cases fill important gaps in published examples.

## Explicit operation slices

Whole-spec results remain unchanged. The slice runner removes complete operations if any request or response advertises unsupported media. It preserves all retained operation values, path-level fields, components, and schema constraints, and records every exclusion in `projection.json`.

| Slice | Included / original operations | Excluded | Result |
| --- | ---: | ---: | --- |
| Meilisearch | 138 / 143 | 5 | Generation, imports, typing, runtime pass |
| AivisSpeech | 37 / 46 | 9 | Generation, imports, typing, runtime pass |

Meilisearch’s dictionary prose says `null` resets the setting, but its declared schema accepts only arrays. Both generated runtime validation and static typing reject `None`. The harness does not reinterpret prose as a schema change.

## Remaining whole-document blockers

These are first observed diagnostics, not an exhaustive classification of each document. Media/transport gaps and strict-parser rejections are kept separate from generator defects; inputs are not silently converted or repaired.

| Native 3.1 specification | First blocker |
| --- | --- |
| [polar](https://raw.githubusercontent.com/polarsource/polar/2a1cb969fb21e2c9811c094069351089736fb172/docs/openapi/2026-10.openapi.json) | `deepObject` query parameter declares `explode: false`. |
| [openresponses](https://raw.githubusercontent.com/openresponses/openresponses/7078a8f1aecd3d1cd41c9891e21c307fcda7f4af/public/openapi/openapi.json) | Discriminator without an adjacent supported composition keyword. |
| [agent_protocol](https://raw.githubusercontent.com/langchain-ai/agent-protocol/440a86ba097b4bc41e8da007b24e419ec07a3e35/openapi.json) | `text/event-stream`. |
| [landing_ai_ade](https://raw.githubusercontent.com/landing-ai/ade-cli/d1995ffef4e63a2a1c2774d9add93368889353c5/openapi.json) | `multipart/form-data`. |
| [airflow_simple_auth](https://raw.githubusercontent.com/apache/airflow/9fc4143efc5eba3b0436122ff7505bc516e8f80a/airflow-core/src/airflow/api_fastapi/auth/managers/simple/openapi/v2-simple-auth-manager-generated.yaml) | `application/x-www-form-urlencoded`. |
| [aivisspeech](https://raw.githubusercontent.com/Aivis-Project/AivisSpeech/77a3675f7b76865cfa0253ac30f41ef1dd822e27/openapi.json) | `text/html` (other operations also use unsupported media). |
| [chroma_core](https://raw.githubusercontent.com/chroma-core/chroma/5f30478c11ffdcf8cba1f872993c29d8f84ce0d9/docs/mintlify/sync.openapi.json) | Unresolved `SourceTypeFilter` schema reference. |
| [cohere_ai](https://raw.githubusercontent.com/cohere-ai/cohere-developer-experience/57596d8d2a8df5e3cdeb072cdc602c17805c72d4/cohere-openapi.yaml) | Empty `enum` in `TruncationStrategy`. |
| [marimo](https://raw.githubusercontent.com/marimo-team/marimo/feaf0b2464be52fb43005d8622708d0af89cf645/packages/openapi/api.yaml) | Non-string `info.version`. |
| [papr_memory](https://raw.githubusercontent.com/Papr-ai/memory-opensource/a0a816eb00f73b0819ef3fd7115bb6835cd18c39/openapi.json) | `multipart/form-data`. |
| [openai](https://raw.githubusercontent.com/openai/openai-openapi/77618e641a840768f1e58197ca53ddedffea6df9/openapi.json) | Discriminator attached to an array schema rather than its union. |
| [opensearch](https://github.com/opensearch-project/opensearch-api-specification/releases/download/v0.2.0/opensearch-openapi.yaml) | A 204 response declares nonempty content. |
| [intelligencex](https://raw.githubusercontent.com/IntelligenceX/SDK/867124360296531330b6fc8fabafaa78bfea4303/openapi.yaml) | An operation has no response declarations. |
| [airflow](https://raw.githubusercontent.com/apache/airflow/9fc4143efc5eba3b0436122ff7505bc516e8f80a/airflow-core/src/airflow/api_fastapi/core_api/openapi/v2-rest-api-generated.yaml) | `application/x-ndjson`. |
| [langflow](https://raw.githubusercontent.com/langflow-ai/langflow/504c02fc47e76087b82b0e7cbe4186e9cdd916d4/docs/openapi/openapi.json) | Duplicate path-template shape under different parameter names. |
| [meilisearch](https://raw.githubusercontent.com/meilisearch/documentation/0d96f60225eabdfed0721ab95e39b3605a764bed/assets/release-assets/meilisearch-openapi.json) | `application/x-ndjson`. |
| [mcp_registry](https://raw.githubusercontent.com/modelcontextprotocol/registry/11b7b0e274d518b6f449c5ee3518a6151d9848b9/docs/reference/api/openapi.yaml) | Unsupported top-level `$id` field. |
| [gravit_launcher](https://raw.githubusercontent.com/GravitLauncher/Launcher/eda714f1ebc9f8cd04a3ed7bf31940c8c1c9b23f/openapi.yaml) | Multipart media-type encoding. |
| [nomad](https://nomad-lab.eu/prod/v1/api/v1/openapi.json) | `application/zip`. |
| [materials_project](https://api.materialsproject.org/openapi.json) | Unknown security-scheme field `descriptions`. |
| [fossology](https://raw.githubusercontent.com/fossology/fossology/d94e7fa191c8a1f1d46abdf4b207896212c9ca3f/src/www/ui/api/documentation/openapi.yaml) | `text/csv`. |
| [optimade](https://raw.githubusercontent.com/Materials-Consortia/optimade-python-tools/main/openapi/openapi.json) | Unsupported media-type parameter. |
| [coolify](https://raw.githubusercontent.com/coollabsio/coolify/v4.x/openapi.yaml) | Unresolved `#/components/responses/403`. |
| [jupyterhub](https://raw.githubusercontent.com/jupyterhub/jupyterhub/main/docs/source/_static/rest-api.yml) | `allowReserved` / `allowEmptyValue` API generation support. |
| [typesense_cloud](https://cloud.typesense.org/api/v1/openapi.json) | `application/pdf`. |
| [openrailwaymap](https://raw.githubusercontent.com/hiddewie/OpenRailwayMap-vector/master/proxy/api/openapi.yaml) | An operation has no response declarations. |

The 14 version controls are `digitalocean`, `gematik_fachdienst`, `langchain_ai`, `langfuse`, `langfuse_organizations`, `linode`, `netlify`, `openfga`, `stripe`, `immich`, `qdrant`, `authentik`, `netbox`, `noc_titiler`. Thirteen use OpenAPI 3.0; Netlify uses Swagger 2.0. The generator continues to require OpenAPI 3.1.

## Readability and artifact findings

Operation modules put request/response declarations, constructor signatures, and source documentation before prepared implementation data. Direct resource/component imports work, all request classes have docstrings, and the audit found no wildcard imports or redundant `MISSING` re-exports. LSProxy’s component request/response bodies retain their names while HTTP envelopes receive suffixes; Unkey’s reference fields now offer useful autocomplete.

There is still visible polish to do: upstream FastAPI operation IDs create long names, stub re-export lines and slot lists can be long, and unconstrained booleans sometimes render as `Literal[False, True]`. These do not break typing, but the output is not uniformly compact.

**Prefect is the important size outlier: about 65 MiB total and a 31.5 MiB `operations/deployments.py`.** Inspection shows repeated large precompiled regular-expression tables, primarily timezone validation, across otherwise small validation programs. This is required validator data, not stored source schema JSON. It works and stays below the per-file limit that motivated splitting, but sharing compiled regex tables across programs is a concrete next optimization. Resource grouping can also collect many operations into `v1.py` or `v2.py`; this audit does not introduce file-budget options.

## Reproduce

Build release codegen and the native extension, then run from the repository root. Each output directory must be new. Source bytes are SHA-256 checked before YAML syntax normalization. Most URLs are immutable commits; a few providers publish mutable endpoints, whose hashes deliberately fail if the snapshot changes. Generated packages and downloaded specifications stay outside the repository.

```sh
cargo build --release --bin jsoncompat
just _build-python-release

env -u VIRTUAL_ENV uv run --no-config --project pybindings --all-extras --locked python \
  pybindings/check_public_openapi.py \
  pybindings/benchmarks/public-openapi-manifest.json /tmp/public-openapi

env -u VIRTUAL_ENV uv run --no-config --project pybindings --all-extras --locked python \
  pybindings/check_public_openapi_samples.py /tmp/public-openapi

env -u VIRTUAL_ENV uv run --no-config --project pybindings --all-extras --locked python \
  pybindings/check_public_openapi_slices.py \
  pybindings/benchmarks/public-openapi-manifest.json /tmp/public-openapi-slices

env -u VIRTUAL_ENV uv run --no-config --project pybindings --all-extras --locked \
  --with jsonschema==4.26.0 python pybindings/review_public_openapi_examples.py /tmp/public-openapi
```

`just check` runs the production regressions and offline corpus-helper tests. Public downloads and the full corpus audit remain opt-in. The result JSON records the release binary hash, harness hash, base revision, and dirty-tree flag; measurements were taken from the source changes accompanying this report.
