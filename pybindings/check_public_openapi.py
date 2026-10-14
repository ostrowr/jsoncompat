"""Audit unmodified, pinned public OpenAPI documents against Python codegen.

Run after building the release CLI and native extension::

    pybindings/.venv/bin/python pybindings/check_public_openapi.py manifest.json /tmp/audit

Manifest format: {"version": 1, "specs": [{"id": "petstore", "path": "petstore.json",
"source_url": "https://...", "sha256": "..."}]}. Paths are relative to the
manifest. If a snapshot is absent, download its pinned URL to the output cache
and verify the hash before use. Extra provenance fields are retained. JSON is passed through unchanged;
YAML is parsed safely and serialized as JSON without schema transformations.
Snapshots, generated packages, and subprocess logs stay in the output directory.
An unsupported document is a recorded rejection, never a successful client test.
No API calls are made. Runtime checks use examples already present in each spec.
"""
from __future__ import annotations

import argparse
import ast
from collections import Counter
from decimal import Decimal
import hashlib
import importlib
import json
import math
import os
from pathlib import Path
import platform
import re
import signal
import subprocess
import sys
import time
from typing import Any
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
METHODS = frozenset({"get", "put", "post", "delete", "options", "head", "patch", "trace"})
TY_VERSION = "0.0.86"


def load_yaml(data: bytes) -> Any:
    """Use YAML 1.2 core scalar rules instead of PyYAML's YAML 1.1 defaults.

    OpenAPI dates and words such as ``on``/``no`` are JSON strings. YAML 1.1's
    timestamp, yes/no booleans, octal and sexagesimal conversions corrupt them.
    The loader remains SafeLoader: no Python object constructors are enabled.
    """
    import yaml

    class CoreLoader(yaml.SafeLoader):
        pass

    replaced = {"tag:yaml.org,2002:" + kind for kind in ("bool", "int", "float", "timestamp")}
    CoreLoader.yaml_implicit_resolvers = {
        key: [(tag, pattern) for tag, pattern in resolvers if tag not in replaced]
        for key, resolvers in yaml.SafeLoader.yaml_implicit_resolvers.items()
    }
    CoreLoader.add_implicit_resolver("tag:yaml.org,2002:bool", re.compile(r"^(?:true|True|TRUE|false|False|FALSE)$"), list("tTfF"))
    CoreLoader.add_implicit_resolver("tag:yaml.org,2002:int", re.compile(r"^[-+]?(?:[0-9]+|0o[0-7]+|0x[0-9a-fA-F]+)$"), list("-+0123456789"))
    CoreLoader.add_implicit_resolver("tag:yaml.org,2002:float", re.compile(
        r"^(?:[-+]?(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][-+]?[0-9]+)?|[-+]?\.(?:inf|Inf|INF)|\.(?:nan|NaN|NAN))$"
    ), list("-+0123456789."))

    def integer(loader: Any, node: Any) -> int:
        value = loader.construct_scalar(node)
        unsigned = value.lstrip("+-")
        return int(value, 8 if unsigned.startswith("0o") else 16 if unsigned.startswith("0x") else 10)

    def floating(loader: Any, node: Any) -> float:
        value = loader.construct_scalar(node)
        number = yaml.SafeLoader.construct_yaml_float(loader, node)
        if not math.isfinite(number):
            raise ValueError("non-finite YAML numbers cannot be normalized to JSON")
        if Decimal(value) != Decimal(str(number)):
            raise ValueError(f"YAML number cannot round-trip through JSON without precision loss: {value}")
        return number

    CoreLoader.add_constructor("tag:yaml.org,2002:int", integer)
    CoreLoader.add_constructor("tag:yaml.org,2002:float", floating)
    return yaml.load(data, Loader=CoreLoader)


def save(path: Path, value: object) -> None:
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n")


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def pointer_segment(value: object) -> str:
    return str(value).replace("~", "~0").replace("/", "~1")


def source_bytes(entry: dict[str, Any], manifest: Path, output: Path) -> bytes:
    source_path = Path(entry.get("path", "sources/" + entry["id"] + ".spec"))
    if not source_path.is_absolute():
        source_path = manifest.parent / source_path
    if source_path.exists():
        return source_path.read_bytes()
    cache = output / "sources" / (entry["id"] + ".spec")
    if cache.exists():
        return cache.read_bytes()
    request = urllib.request.Request(entry["source_url"], headers={"User-Agent": "jsoncompat-public-openapi-audit"})
    with urllib.request.urlopen(request, timeout=60) as response:
        data = response.read(64 * 1024 * 1024 + 1)
    if len(data) > 64 * 1024 * 1024:
        raise ValueError("source document exceeds the audit's 64 MiB download limit")
    if digest(data) != entry["sha256"]:
        raise ValueError("download SHA-256 does not match the pinned manifest")
    cache.parent.mkdir(parents=True, exist_ok=True)
    cache.write_bytes(data)
    return data


def resolve(document: dict[str, Any], value: Any) -> Any:
    """Resolve local references for inspection; never mutate or lower the input."""
    seen: set[str] = set()
    while isinstance(value, dict) and "$ref" in value:
        reference = value["$ref"]
        if not isinstance(reference, str) or not reference.startswith("#/") or reference in seen:
            return value
        seen.add(reference)
        target: Any = document
        try:
            for part in reference[2:].split("/"):
                target = target[part.replace("~1", "/").replace("~0", "~")]
        except (KeyError, TypeError):
            return value
        # Siblings matter for JSON Schema; inspection must not silently drop them.
        if len(value) > 1 and isinstance(target, dict):
            value = {**target, **{key: item for key, item in value.items() if key != "$ref"}}
        else:
            value = target
    return value


def shape(document: dict[str, Any], schema: Any) -> str:
    schema = resolve(document, schema)
    if isinstance(schema, bool):
        return "any" if schema else "never"
    if not isinstance(schema, dict):
        return "unspecified"
    kind = schema.get("type")
    if isinstance(kind, list):
        return " | ".join(sorted(kind))
    if isinstance(kind, str):
        return kind
    for key in ("oneOf", "anyOf", "allOf"):
        if key in schema:
            return key
    return "object" if "properties" in schema else "unspecified"


def spec_inventory(document: dict[str, Any]) -> dict[str, Any]:
    responses: Counter[str] = Counter()
    bodies: Counter[str] = Counter()
    media_types: Counter[str] = Counter()
    operation_count = 0
    for path_item in document.get("paths", {}).values():
        path_item = resolve(document, path_item)
        if not isinstance(path_item, dict):
            continue
        for method, operation in path_item.items():
            if method not in METHODS or not isinstance(operation, dict):
                continue
            operation_count += 1
            for status, response in operation.get("responses", {}).items():
                response = resolve(document, response)
                responses[str(status)] += 1
                if isinstance(response, dict):
                    for media, content in response.get("content", {}).items():
                        media_types[media] += 1
                        if isinstance(content, dict):
                            bodies[shape(document, content.get("schema", {}))] += 1
    schemas = document.get("components", {}).get("schemas", {})
    return {
        "openapi": document.get("openapi", document.get("swagger")),
        "operations": operation_count,
        "component_schemas": len(schemas),
        "component_root_shapes": dict(Counter(shape(document, schema) for schema in schemas.values())),
        "response_statuses": dict(sorted(responses.items())),
        "response_root_shapes": dict(sorted(bodies.items())),
        "media_types": dict(sorted(media_types.items())),
    }


def run(directory: Path, name: str, command: list[str], timeout: float, env: dict[str, str]) -> dict[str, Any]:
    start = time.perf_counter()
    with (directory / f"{name}.stdout").open("w") as out, (directory / f"{name}.stderr").open("w") as err:
        process = subprocess.Popen(command, stdout=out, stderr=err, env=env, start_new_session=True)
        try:
            code: int | str = process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait()
            code = "timeout"
    result = {"command": command, "seconds": time.perf_counter() - start, "exit_code": code}
    if code != 0:
        result["diagnostic"] = (directory / f"{name}.stderr").read_text()[-12000:] or (directory / f"{name}.stdout").read_text()[-12000:]
    save(directory / f"{name}.json", result)
    return result


def rejection_kind(diagnostic: str) -> str:
    """Conservative labels retain full diagnostics for subsequent human review."""
    lower = diagnostic.lower()
    if "openapi 3.1" in lower or "unsupported openapi version" in lower:
        return "unsupported_version"
    if any(term in lower for term in ("unsupported media", "media type", "multipart/form-data", "application/octet-stream", "application/x-www-form-urlencoded")):
        return "unsupported_media_or_contract"
    if "unsupported" in lower or "not supported" in lower:
        return "unsupported_feature"
    return "generation_rejected_needs_review"


def package_inventory(package: Path) -> dict[str, Any]:
    files = {str(path.relative_to(package)): path.read_bytes() for path in sorted(package.rglob("*"))
             if path.is_file() and "__pycache__" not in path.parts}
    manifest = json.loads(files[".jsoncompat-manifest.json"])
    for name, expected in manifest["files"].items():
        assert name in files and digest(files[name]) == expected, f"manifest hash mismatch: {name}"
    classes = 0
    request_classes = 0
    documented_requests = 0
    root_classes = 0
    wildcard_imports: list[str] = []
    missing_aliases: list[str] = []
    public_long_lines: list[dict[str, Any]] = []
    names: list[str] = []
    for name, data in files.items():
        if not name.endswith((".py", ".pyi")):
            continue
        source = data.decode()
        tree = ast.parse(source, filename=name)
        compile(source, name, "exec")
        for node in tree.body:
            if isinstance(node, ast.ImportFrom) and any(alias.name == "*" for alias in node.names):
                wildcard_imports.append(name)
            if isinstance(node, (ast.Assign, ast.AnnAssign)):
                targets = node.targets if isinstance(node, ast.Assign) else [node.target]
                if any(isinstance(target, ast.Name) and target.id in {"MISSING", "JSONCOMPAT_MISSING", "_jsoncompat_missing"} for target in targets):
                    missing_aliases.append(name)
            if isinstance(node, ast.ClassDef):
                classes += 1
                names.append(node.name)
                root_classes += any(isinstance(child, ast.AnnAssign) and isinstance(child.target, ast.Name)
                                    and child.target.id == "value" for child in node.body)
                if any(isinstance(base, ast.Subscript) and isinstance(base.value, ast.Attribute)
                       and base.value.attr == "Request" for base in node.bases):
                    request_classes += 1
                    documented_requests += bool(ast.get_docstring(node))
        for number, line in enumerate(source.splitlines(), 1):
            if line.startswith(("_jsoncompat_program_", "dc.bind_module", "_JSONCOMPAT_")):
                break
            if len(line) > 120:
                public_long_lines.append({"file": name, "line": number, "characters": len(line), "prefix": line[:140]})
    sizes = sorted(((len(data), name) for name, data in files.items()), reverse=True)
    return {
        "files": len(files), "bytes": sum(len(data) for data in files.values()),
        "python_files": sum(name.endswith((".py", ".pyi")) for name in files),
        "classes": classes, "root_wrapper_candidates": root_classes,
        "request_classes": request_classes, "documented_requests": documented_requests,
        "largest_files": sizes[:10], "manifest_hashes_valid": True,
        "wildcard_imports": wildcard_imports, "redundant_missing_aliases": missing_aliases,
        "public_lines_over_120": len(public_long_lines), "long_line_examples": public_long_lines[:20],
        "longest_class_names": sorted(names, key=len, reverse=True)[:10],
    }


def example_values(document: dict[str, Any], owner: Any, pointer: str, seen: frozenset[str] = frozenset()) -> list[tuple[str, Any]]:
    if not isinstance(owner, dict):
        return []
    result: list[tuple[str, Any]] = []
    reference = owner.get("$ref")
    if isinstance(reference, str) and reference.startswith("#/") and reference not in seen:
        target = resolve(document, {"$ref": reference})
        if target != owner:
            result.extend(example_values(document, target, reference, seen | {reference}))
    if "example" in owner:
        result.append((pointer + "/example", owner["example"]))
    examples = owner.get("examples", [])
    if isinstance(examples, list):
        result.extend((f"{pointer}/examples/{index}", value) for index, value in enumerate(examples))
    elif isinstance(examples, dict):
        for name, value in examples.items():
            value = resolve(document, value)
            if isinstance(value, dict) and "value" in value:
                result.append((f"{pointer}/examples/{pointer_segment(name)}/value", value["value"]))
    if "schema" in owner:
        result.extend(example_values(document, owner["schema"], pointer + "/schema", seen))
    # Sample at most four examples per media variant, deterministically.
    return result[:4]


def response_status(label: int | str, declarations: set[int | str]) -> int | None:
    if isinstance(label, int):
        return label
    candidates = range(599, 99, -1) if label == "default" else range(int(label[0]) * 100 + 99, int(label[0]) * 100 - 1, -1)
    for status in candidates:
        if status in declarations:
            continue
        if label == "default" and f"{status // 100}XX" in declarations:
            continue
        return status
    return None


def runtime_worker(directory: Path, package_name: str) -> None:
    """Runs in an isolated process so one package cannot contaminate another."""
    from unittest.mock import patch
    from jsoncompat.http import ClientResponse, Err, HTTPStatusError, Ok, RawResponse, UnexpectedResponse, _binding

    document = json.loads((directory / "source.json").read_text())
    started = time.perf_counter()
    with patch.object(_binding, "get_type_hints", wraps=_binding.get_type_hints) as reflection:
        api_module = importlib.import_module(package_name + ".api")
        api = api_module.CorpusAPI
        reflection_calls = reflection.call_count
    import_seconds = time.perf_counter() - started
    operations = api.operations()
    assert len(operations) == spec_inventory(document)["operations"], "generation lost operations"
    assert reflection_calls == 0, "generated dispatch performed import-time reflection"
    imported_modules = 0
    for path in sorted((directory / package_name).rglob("*.py")):
        relative = path.relative_to(directory).with_suffix("")
        parts = relative.parts[:-1] if relative.name == "__init__" else relative.parts
        importlib.import_module(".".join(parts))
        imported_modules += 1
    consumers = ["from typing import assert_type", "from jsoncompat.http import ClientResponse, Result",
                 f"from {package_name}.api import CorpusClient, CorpusAsyncClient", ""]
    samples: list[dict[str, Any]] = []
    uncovered: list[dict[str, Any]] = []
    failures: list[dict[str, Any]] = []

    class ValuePathRuntime:
        def __init__(self, runtime: object):
            self.runtime = runtime

        def __getattr__(self, name: str) -> Any:
            return getattr(self.runtime, name)

        def _deserialize_json_field(self, *args: object) -> None:
            return None

    def observed(plan: Any, raw: RawResponse) -> tuple[object, ...]:
        try:
            result = plan.decode(raw)
            if isinstance(result, Ok):
                return ("Ok", result.value.__class__.__name__, result.value.to_value())
            if isinstance(result.error, UnexpectedResponse):
                return ("UnexpectedResponse", str(result.error.decode_error), str(result.error.decode_error.__cause__))
            return ("Err", result.error.__class__.__name__, result.error.to_value())
        except Exception as error:
            return ("raised", type(error).__name__, str(error), str(error.__cause__))

    for index, operation in enumerate(operations):
        plan = api.__jsoncompat_http_plans__[operation.request]
        assert plan.endpoint is operation.endpoint
        assert plan == _binding._prepare(operation, api), operation.endpoint
        request = operation.request
        success = operation.success
        error = operation.error
        consumers += [
            f"from {request.__module__} import {request.__name__} as Request{index}",
            f"from {success.__module__} import {success.__name__} as Success{index}",
            f"from {error.__module__} import {error.__name__} as Error{index}",
            f"def check_{index}(client: CorpusClient, request: Request{index}) -> Success{index}:",
            "    response = client.request(request)",
            f"    assert_type(response, ClientResponse[Success{index}, Error{index}])",
            f"    assert_type(response.result(), Result[Success{index}, Error{index}])",
            "    return response.unwrap()", "",
            f"async def check_async_{index}(client: CorpusAsyncClient, request: Request{index}) -> Success{index}:",
            "    return (await client.request(request)).unwrap()", "",
        ]
        endpoint = operation.endpoint
        original = resolve(document, document["paths"][endpoint.path])[endpoint.method.lower()]
        declarations = {response.status for response in endpoint.responses}
        for response in endpoint.responses:
            raw_definition = resolve(document, original["responses"][str(response.status)])
            pointer = f"#/paths/{pointer_segment(endpoint.path)}/{endpoint.method.lower()}/responses/{response.status}"
            candidates = [(pointer, None)] if response.content_type is None else example_values(
                document, raw_definition.get("content", {}).get(response.content_type, {}),
                pointer + "/content/" + pointer_segment(response.content_type),
            )
            status = response_status(response.status, declarations)
            identity = {"method": endpoint.method, "path": endpoint.path, "declared_status": response.status,
                        "media_type": response.content_type, "model": response.model.__name__}
            if not candidates or status is None:
                uncovered.append({**identity, "reason": "no inline example" if not candidates else "fully shadowed response"})
                continue
            for origin, value in candidates:
                headers = () if response.content_type is None else (("Content-Type", response.content_type),)
                body = b"" if response.content_type is None else (
                    str(value).encode() if response.content_type == "text/plain" else json.dumps(value, ensure_ascii=False).encode()
                )
                raw = RawResponse(status, headers, body)
                model = response.model
                original_runtime = model.__jsoncompat_runtime__
                native = observed(plan, raw)
                try:
                    model.__jsoncompat_runtime__ = ValuePathRuntime(original_runtime)
                    baseline = observed(plan, raw)
                finally:
                    model.__jsoncompat_runtime__ = original_runtime
                sample = {**identity, "origin": origin, "actual_status": status, "body_shape": type(value).__name__,
                          "native_value_equivalent": native == baseline, "outcome": native[0]}
                if native != baseline:
                    failures.append({**sample, "reason": "native/value decoder disagreement", "native": repr(native), "value_path": repr(baseline)})
                if native[0] in {"raised", "UnexpectedResponse"}:
                    sample["diagnostic"] = repr(native)
                    # Invalid upstream examples are evidence, not generated-client failures.
                    sample["outcome"] = "example_rejected_needs_review"
                else:
                    envelope = ClientResponse(raw, plan.decode)
                    assert envelope.status == status and envelope.content == body
                    assert envelope.result() is envelope.result(), "result was not cached"
                    result = envelope.result()
                    if 200 <= status < 300:
                        assert isinstance(result, Ok) and isinstance(envelope.unwrap(), model)
                    else:
                        assert isinstance(result, Err) and isinstance(result.error, model)
                        try:
                            envelope.unwrap()
                        except HTTPStatusError as exc:
                            assert exc.error is result.error and exc.response is raw
                        else:
                            raise AssertionError("non-success unwrap did not raise")
                samples.append(sample)
        # Undeclared non-success statuses must preserve wire data even without examples.
        unknown = response_status("default", declarations | {"default"}) if "default" not in declarations else None
        if unknown is not None and unknown >= 300:
            raw = RawResponse(unknown, (("Content-Type", "application/json"), ("X-Audit", "retained")), b'{"message":"audit failure"}')
            result = ClientResponse(raw, plan.decode).result()
            assert isinstance(result, Err) and isinstance(result.error, UnexpectedResponse)
            assert result.error.status == unknown and result.error.body == raw.body
            assert result.error.headers["x-audit"] == "retained"
            assert result.error.json() == {"message": "audit failure"}
    (directory / "consumer.py").write_text("\n".join(consumers))
    if operations:
        request = operations[0].request
        (directory / "invalid_consumer.py").write_text(
            f"from {package_name}.api import CorpusClient\nfrom {request.__module__} import {request.__name__}\n"
            f"def invalid(client: CorpusClient, request: {request.__name__}) -> int:\n    return client.request(request)\n"
        )
    save(directory / "runtime-report.json", {
        "operations": len(operations), "import_seconds": import_seconds, "import_reflection_calls": reflection_calls,
        "all_generated_modules_imported": imported_modules,
        "dispatch_plans_equivalent": True, "examples": samples, "uncovered_responses": uncovered,
        "differential_failures": failures,
    })
    if failures:
        raise AssertionError(f"{len(failures)} native/value disagreements")


def audit(entry: dict[str, Any], manifest: Path, output: Path, generator: Path, timeout: float) -> dict[str, Any]:
    identifier = entry["id"]
    if not isinstance(identifier, str) or not re.fullmatch(r"[a-z][a-z0-9_]*", identifier):
        raise ValueError(f"invalid corpus id: {identifier!r}")
    directory = output / identifier
    directory.mkdir(parents=True, exist_ok=False)
    result: dict[str, Any] = {"id": identifier, "source": entry, "status": "incomplete"}
    def finish(status: str) -> dict[str, Any]:
        result["status"] = status
        save(directory / "audit.json", result)
        return result
    raw = source_bytes(entry, manifest, output)
    if digest(raw) != entry["sha256"]:
        result["diagnostic"] = "source SHA-256 mismatch"
        return finish("source_integrity_failure")
    result["source_bytes"] = len(raw)
    try:
        document = json.loads(raw)
        source = raw
        result["format_normalization"] = None
    except (json.JSONDecodeError, UnicodeDecodeError):
        try:
            document = load_yaml(raw)
            source = json.dumps(document, ensure_ascii=False, allow_nan=False).encode()
            result["format_normalization"] = "YAML 1.2 core scalar rules to JSON; no schema changes"
        except Exception as error:
            result["diagnostic"] = f"{type(error).__name__}: {error}"
            return finish("source_parse_failure")
    if not isinstance(document, dict):
        result["diagnostic"] = "OpenAPI root is not an object"
        return finish("source_parse_failure")
    source_path = directory / "source.json"
    source_path.write_bytes(source)
    result["normalized_sha256"] = digest(source)
    result["spec_inventory"] = spec_inventory(document)
    env = dict(os.environ)
    env["PYTHONPATH"] = os.pathsep.join([str(ROOT / "pybindings"), str(directory)])
    env["JSONCOMPAT_NATIVE_PROFILE"] = "release"
    env["PYTHONDONTWRITEBYTECODE"] = "1"
    for key in ("VIRTUAL_ENV", "UV_DEFAULT_INDEX", "UV_INDEX", "UV_INDEX_URL", "UV_EXTRA_INDEX_URL"):
        env.pop(key, None)
    package_name = "corpus_" + identifier
    package = directory / package_name
    command = [str(generator), "codegen", "--openapi", str(source_path), "--api", "Corpus", "--output", str(package)]
    generation = run(directory, "generation", command, timeout, env)
    result["generation"] = generation
    if generation["exit_code"] != 0:
        if generation["exit_code"] == "timeout":
            return finish("generation_timeout")
        version = result["spec_inventory"]["openapi"]
        if isinstance(version, str) and not version.startswith("3.1."):
            return finish("unsupported_version")
        return finish(rejection_kind(generation["diagnostic"]))
    try:
        result["generated_inventory"] = package_inventory(package)
    except Exception as error:
        result["diagnostic"] = f"{type(error).__name__}: {error}"
        return finish("generated_source_failure")
    # Generate independently, at the same package basename, so module names match.
    duplicate = directory / "repeat" / package_name
    duplicate.parent.mkdir()
    repeated = run(directory, "generation-repeat", [*command[:-1], str(duplicate)], timeout, env)
    result["repeat_generation"] = repeated
    if repeated["exit_code"] != 0:
        return finish("repeat_generation_failure")
    files = lambda path: {str(item.relative_to(path)): digest(item.read_bytes()) for item in path.rglob("*") if item.is_file() and "__pycache__" not in item.parts}
    first_files, second_files = files(package), files(duplicate)
    result["deterministic"] = first_files == second_files
    if not result["deterministic"]:
        result["changed_files"] = sorted(name for name in first_files.keys() | second_files.keys() if first_files.get(name) != second_files.get(name))
        return finish("nondeterministic_generation")
    runtime = run(directory, "runtime", [sys.executable, str(Path(__file__).resolve()), "_runtime", str(directory), package_name], timeout, env)
    result["runtime"] = runtime
    if (directory / "runtime-report.json").exists():
        result["runtime_report"] = json.loads((directory / "runtime-report.json").read_text())
    if runtime["exit_code"] != 0:
        return finish("runtime_failure")
    launcher = ["uv", "run", "--no-config", "--project", str(ROOT / "pybindings"), "--all-extras", "--locked", "--with", f"ty=={TY_VERSION}", "ty", "check",
                "--project", str(directory), "--python-version", "3.12", "--extra-search-path", str(ROOT / "pybindings"),
                "--extra-search-path", str(directory), "--error", "all", "--output-format", "gitlab"]
    result["typecheck"] = run(directory, "typecheck", [*launcher, str(package), str(directory / "consumer.py")], timeout, env)
    if result["typecheck"]["exit_code"] != 0:
        return finish("typecheck_failure")
    invalid = directory / "invalid_consumer.py"
    if invalid.exists():
        result["typecheck_negative"] = run(directory, "typecheck-negative", [*launcher, str(invalid)], timeout, env)
        try:
            diagnostics = json.loads((directory / "typecheck-negative.stdout").read_text())
            negative_passed = result["typecheck_negative"]["exit_code"] == 1 and {item["check_name"] for item in diagnostics} == {"invalid-return-type"}
        except (ValueError, TypeError, KeyError):
            negative_passed = False
        result["typecheck_negative"]["expected_error_detected"] = negative_passed
        if not negative_passed:
            return finish("negative_typecheck_failure")
    return finish("passed")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", type=Path)
    parser.add_argument("output", type=Path, help="fresh audit directory")
    parser.add_argument("--spec", help="audit one manifest id")
    parser.add_argument("--timeout", type=float, default=60, help="seconds per subprocess")
    parser.add_argument("--generator", type=Path, default=ROOT / "target/release/jsoncompat")
    args = parser.parse_args()
    if args.timeout <= 0:
        parser.error("timeout must be positive")
    manifest = args.manifest.resolve()
    source = json.loads(manifest.read_text())
    if source.get("version") != 1 or not isinstance(source.get("specs"), list):
        parser.error("expected version-1 corpus manifest")
    entries = [entry for entry in source["specs"] if args.spec is None or entry["id"] == args.spec]
    if not entries or len({entry["id"] for entry in entries}) != len(entries):
        parser.error("empty selection or duplicate corpus ids")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    save(output / "environment.json", {
        "platform": platform.platform(), "python": sys.version, "ty": TY_VERSION,
        "generator_sha256": digest(args.generator.read_bytes()), "manifest_sha256": digest(manifest.read_bytes()),
        "revision": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "working_tree_dirty": bool(subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT, text=True).strip()),
        "harness_sha256": digest(Path(__file__).read_bytes()),
    })
    results: list[dict[str, Any]] = []
    for entry in entries:
        try:
            result = audit(entry, manifest, output, args.generator.resolve(), args.timeout)
        except Exception as error:
            result = {"id": entry["id"], "source": entry, "status": "audit_failure", "diagnostic": f"{type(error).__name__}: {error}"}
            directory = output / entry["id"]
            if directory.is_dir():
                save(directory / "audit-failure.json", result)
        results.append(result)
        save(output / "aggregate.json", {"counts": dict(Counter(item["status"] for item in results)), "specs": results})
        print(f"{entry['id']}: {result['status']}", flush=True)
    # Unsupported contracts are audit findings; broken verification is a failure.
    findings = {"passed", "unsupported_version", "unsupported_feature",
                "unsupported_media_or_contract", "generation_rejected_needs_review"}
    if any(result["status"] not in findings for result in results):
        raise SystemExit(1)


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "_runtime":
        runtime_worker(Path(sys.argv[2]), sys.argv[3])
    else:
        main()
