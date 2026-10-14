"""Exercise explicit sample bodies against unchanged public generated clients.

    pybindings/.venv/bin/python pybindings/check_public_openapi_samples.py /tmp/audit

The companion manifest labels every synthesized payload. These are schema tests,
not live API calls; a rejected expected-valid sample is a test failure. Run the
main public corpus audit first, retaining its generated packages and source.json.
"""
from __future__ import annotations

import argparse
from collections import Counter
import importlib
import json
import os
from pathlib import Path
import subprocess
import sys
from typing import Any

from check_public_openapi import TY_VERSION, run

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SAMPLES = ROOT / "pybindings/benchmarks/public-openapi-samples.json"


def check_case(directory: Path, case: dict[str, Any]) -> dict[str, Any]:
    from jsoncompat.http import Body, ClientResponse, Err, HTTPStatusError, Ok, RawResponse, ResponseError, UnexpectedResponse

    spec = case["spec"]
    sys.path.insert(0, str(directory / spec))
    api = importlib.import_module(f"corpus_{spec}.api").CorpusAPI
    operation = next(operation for operation in api.operations()
                     if operation.endpoint.method == case["method"] and operation.endpoint.path == case["path"])
    plan = api.__jsoncompat_http_plans__[operation.request]

    class ValuePathRuntime:
        def __init__(self, runtime: object):
            self.runtime = runtime

        def __getattr__(self, name: str) -> Any:
            return getattr(self.runtime, name)

        def _serialize_json_field(self, *args: object) -> None:
            raise TypeError("audit: force ordinary value conversion")

        def _deserialize_json_field(self, *args: object) -> None:
            return None

    if case["direction"] == "request":
        body_field = next(field.name for field in plan.fields if isinstance(field.metadata, Body))
        fields = {**case.get("parameters", {}), body_field: case["body"]}
        if case.get("expect") == "request_error":
            try:
                operation.request.from_value(fields)
            except (ValueError, TypeError):
                return {"id": case["id"], "status": "passed", "direction": "request",
                        "outcome": "request_error", "body_shape": type(case["body"]).__name__}
            raise AssertionError("request accepted a body excluded by the declared schema")
        request = operation.request.from_value(fields)
        native = plan.encode(request)
        original = operation.request.__jsoncompat_runtime__
        try:
            operation.request.__jsoncompat_runtime__ = ValuePathRuntime(original)
            ordinary = plan.encode(request)
        finally:
            operation.request.__jsoncompat_runtime__ = original
        assert native == ordinary, "native/value request encoding differs"
        assert native.body is not None
        assert json.loads(native.body) == case["body"], "request body changed"
        return {"id": case["id"], "status": "passed", "direction": "request",
                "body_shape": type(case["body"]).__name__, "native_value_equivalent": True}

    status = case["status"]
    media = case.get("media_type", "application/json")
    body = case["raw_body"].encode() if "raw_body" in case else (
        b"" if media is None else json.dumps(case["body"], ensure_ascii=False).encode()
    )
    headers = (() if media is None else (("Content-Type", media),)) + (
        ("X-Corpus-Sample", case["id"]), ("Set-Cookie", "first=1"), ("Set-Cookie", "second=2"),
    )
    raw = RawResponse(status, headers, body)
    calls = 0

    def decode(response: RawResponse):
        nonlocal calls
        calls += 1
        return plan.decode(response)

    envelope = ClientResponse(raw, decode)
    assert envelope.status == status and envelope.content == body
    assert envelope.headers["x-corpus-sample"] == case["id"]
    assert envelope.headers.get_all("set-cookie") == ("first=1", "second=2")
    assert calls == 0, "reading buffered metadata parsed the body"
    expected = case.get("expect", "declared")
    if expected != "declared":
        def observation() -> object:
            try:
                observed = plan.decode(raw)
                item = observed.value if isinstance(observed, Ok) else observed.error
                if isinstance(item, UnexpectedResponse):
                    return ("unexpected", str(item.decode_error), str(item.decode_error.__cause__))
                return (type(observed).__name__, type(item).__name__, item.to_value())
            except ResponseError as error:
                return ("raises", str(error), str(error.__cause__))

        native_observation = observation()
        original_runtimes = {response.model: response.model.__jsoncompat_runtime__ for response in operation.endpoint.responses}
        try:
            for model, runtime in original_runtimes.items():
                model.__jsoncompat_runtime__ = ValuePathRuntime(runtime)
            ordinary_observation = observation()
        finally:
            for model, runtime in original_runtimes.items():
                model.__jsoncompat_runtime__ = runtime
        assert native_observation == ordinary_observation, "native/value diagnostics differ"
    if expected == "parse_error":
        try:
            envelope.result()
        except ResponseError as first:
            assert first.response is raw
            try:
                envelope.unwrap()
            except ResponseError as second:
                assert second is first
            else:
                raise AssertionError("unwrap accepted invalid success response")
        else:
            raise AssertionError("result accepted invalid success response")
        assert calls == 1, "parse failure was not cached"
        return {"id": case["id"], "status": "passed", "direction": "response", "outcome": expected,
                "lazy_metadata": True, "cached_parse_failure": True, "native_value_equivalent": True}

    result = envelope.result()
    assert envelope.result() is result and calls == 1
    value = result.value if isinstance(result, Ok) else result.error
    if expected == "unexpected":
        assert isinstance(result, Err) and isinstance(value, UnexpectedResponse)
        assert value.status == status and value.body == body and value.text() == body.decode()
        assert value.headers["x-corpus-sample"] == case["id"]
        assert value.decode_error.response is raw
        if "body" in case:
            assert value.json() == case["body"]
    else:
        assert not isinstance(value, UnexpectedResponse), getattr(value, "decode_error", None)
        decoded = value.to_value()
        assert decoded["body"] == case["body"], "response body changed"
        assert decoded["status"] == status
        assert decoded["raw_headers"] == [list(pair) for pair in headers], "response headers changed"
        assert isinstance(result, Ok) if 200 <= status < 300 else isinstance(result, Err)
        # All declared schemas remain active for both native and ordinary decoding.
        model = type(value)
        original = model.__jsoncompat_runtime__
        try:
            model.__jsoncompat_runtime__ = ValuePathRuntime(original)
            other = plan.decode(raw)
        finally:
            model.__jsoncompat_runtime__ = original
        other_value = other.value if isinstance(other, Ok) else other.error
        assert type(other_value) is model and other_value.to_value() == value.to_value()
    if isinstance(result, Ok):
        assert envelope.unwrap() is value
    else:
        try:
            envelope.unwrap()
        except HTTPStatusError as error:
            assert error.error is value and error.response is raw
        else:
            raise AssertionError("error unwrap did not raise")
    assert calls == 1, "unwrap parsed a second time"
    return {"id": case["id"], "status": "passed", "direction": "response", "http_status": status,
            "body_shape": type(case.get("body")).__name__, "outcome": expected,
            "lazy_metadata": True, "cached_result": True, "native_value_equivalent": True}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("audit", type=Path)
    parser.add_argument("--samples", type=Path, default=DEFAULT_SAMPLES)
    args = parser.parse_args()
    audit = args.audit.resolve()
    sample_manifest = json.loads(args.samples.read_text())
    cases = sample_manifest["cases"]
    env = dict(os.environ)
    env["PYTHONPATH"] = str(ROOT / "pybindings")
    env["JSONCOMPAT_NATIVE_PROFILE"] = "release"
    results = []
    for case in cases:
        completed = subprocess.run([sys.executable, __file__, "_case", str(audit), json.dumps(case)],
                                   capture_output=True, text=True, env=env, timeout=60)
        if completed.returncode:
            result = {"id": case["id"], "status": "failed", "diagnostic": completed.stderr}
        else:
            result = json.loads(completed.stdout)
        results.append(result)
        print(f"{case['id']}: {result['status']}", flush=True)
    typing_checks = []
    for index, consumer in enumerate(sample_manifest.get("typing_consumers", [])):
        paths = [str(audit / spec) for spec in consumer["specs"]]
        command = ["uv", "run", "--no-config", "--project", str(ROOT / "pybindings"), "--all-extras", "--locked",
                   "--with", f"ty=={TY_VERSION}", "ty", "check", "--project", str(audit), "--python-version", "3.12",
                   "--extra-search-path", str(ROOT / "pybindings"), "--error", "all", "--output-format", "gitlab"]
        for path in paths:
            command += ["--extra-search-path", path]
        command.append(str((args.samples.parent / consumer["path"]).resolve()))
        check = run(audit, f"explicit-consumer-{index}", command, 60, env)
        typing_checks.append(check)
        print(f"consumer {consumer['path']}: {'passed' if check['exit_code'] == 0 else 'failed'}", flush=True)
    output = {"samples_manifest": str(args.samples.resolve()), "counts": dict(Counter(result["status"] for result in results)),
              "cases": results, "typing_consumers": typing_checks}
    (audit / "explicit-samples.json").write_text(json.dumps(output, indent=2) + "\n")
    if any(result["status"] != "passed" for result in results) or any(check["exit_code"] != 0 for check in typing_checks):
        raise SystemExit(1)


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "_case":
        print(json.dumps(check_case(Path(sys.argv[2]), json.loads(sys.argv[3]))))
    else:
        main()
