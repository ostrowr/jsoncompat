"""Measure checked HTTP model conversion without network latency.

After building the release CLI and extension, run:
    JSONCOMPAT_NATIVE_PROFILE=release python pybindings/bench_http_models.py /tmp/http-models

The output directory must be fresh. The generated API has nested JSON bodies,
typed response headers, and required/length/range constraints. Setup and semantic
checks are outside the timers; encoding and decoding include validation.
"""

from __future__ import annotations

import importlib
import json
import os
from pathlib import Path
import statistics
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]


def specification() -> dict[str, object]:
    batch = {"$ref": "#/components/schemas/Batch"}
    response = {
        "description": "A validated batch",
        "headers": {
            "X-Sequence": {
                "required": True,
                "schema": {"type": "integer", "minimum": 0},
            }
        },
        "content": {"application/json": {"schema": batch}},
    }
    return {
        "openapi": "3.1.0",
        "info": {"title": "HTTP model benchmark", "version": "1"},
        "paths": {
            "/batch": {
                "get": {"operationId": "getBatch", "responses": {"200": response}},
                "put": {
                    "operationId": "putBatch",
                    "requestBody": {
                        "required": True,
                        "content": {"application/json": {"schema": batch}},
                    },
                    "responses": {"200": response},
                },
            }
        },
        "components": {
            "schemas": {
                "Batch": {
                    "type": "object",
                    "properties": {
                        "items": {
                            "type": "array",
                            "items": {"$ref": "#/components/schemas/Item"},
                        }
                    },
                    "required": ["items"],
                    "additionalProperties": False,
                },
                "Item": {
                    "type": "object",
                    "properties": {
                        "id": {"type": "integer", "minimum": 0},
                        "name": {"type": "string", "minLength": 1},
                        "tags": {"type": "array", "items": {"type": "string"}},
                        "active": {"type": "boolean"},
                    },
                    "required": ["id", "name", "tags", "active"],
                    "additionalProperties": False,
                },
            }
        },
    }


def generate(directory: Path, generator: Path) -> None:
    directory.mkdir(parents=True, exist_ok=True)
    package = directory / "http_benchmark"
    if package.exists():
        raise ValueError("use a fresh benchmark output directory")
    spec = directory / "openapi.json"
    spec.write_text(json.dumps(specification()) + "\n")
    subprocess.run(
        [str(generator), "codegen", "--openapi", str(spec), "--api", "Benchmark",
         "--output", str(package)],
        check=True,
    )


def measure(directory: Path) -> dict[str, object]:
    from jsoncompat.http import RawResponse, bind, prepare

    sys.path.insert(0, str(directory))
    api = importlib.import_module("http_benchmark")
    get = api.GetBatchRequest()
    prepared = prepare(api.BenchmarkAPI, get)
    rows = []
    for count in (1, 100, 1_000, 10_000):
        value = {
            "items": [
                {"id": i, "name": f"item-{i}", "tags": ["first", "second"], "active": True}
                for i in range(count)
            ]
        }
        content = json.dumps(value, separators=(",", ":")).encode()
        batch = api.Batch.from_value(value)
        request = api.PutBatchRequest(body=batch)
        raw = RawResponse(
            200, (("Content-Type", "application/json"), ("X-Sequence", "7")), content
        )

        def fetch(encoded, /, *, transport=None):
            return raw

        client = bind(api.BenchmarkAPI, fetch=fetch)
        decoded = prepared.decode(raw).unwrap()
        assert decoded.body.to_value() == value
        assert decoded.headers.x_sequence == 7
        wire = prepare(api.BenchmarkAPI, request).request
        assert wire.body is not None and json.loads(wire.body) == value
        assert client.request(get).unwrap().body.to_value() == value

        cases = {
            "encode": lambda: prepare(api.BenchmarkAPI, request),
            "decode": lambda: prepared.decode(raw).unwrap(),
            "request_and_unwrap": lambda: client.request(get).unwrap(),
        }
        row: dict[str, object] = {"items": count, "body_bytes": len(content)}
        for name, operation in cases.items():
            # Calibrate each case to at least 50ms; record iterations for audit.
            iterations = 1
            while True:
                start = time.perf_counter()
                for _ in range(iterations):
                    operation()
                if time.perf_counter() - start >= 0.05:
                    break
                iterations *= 2
            samples = []
            for _ in range(7):
                start = time.perf_counter()
                for _ in range(iterations):
                    operation()
                samples.append((time.perf_counter() - start) / iterations)
            row[name] = {
                "iterations": iterations,
                "median_seconds": statistics.median(samples),
                "samples_seconds": samples,
            }
        rows.append(row)
        print(json.dumps(row), flush=True)
    return {"python": sys.version, "cases": rows}


if __name__ == "__main__":
    directory = Path(sys.argv[1]).resolve()
    os.environ.setdefault("JSONCOMPAT_NATIVE_PROFILE", "release")
    generate(directory, ROOT / "target/release/jsoncompat")
    results = measure(directory)
    (directory / "results.json").write_text(json.dumps(results, indent=2) + "\n")
