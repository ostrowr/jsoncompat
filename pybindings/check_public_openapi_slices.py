"""Audit explicitly derived JSON operation slices of two pinned public APIs.

    pybindings/.venv/bin/python pybindings/check_public_openapi_slices.py \
        pybindings/benchmarks/public-openapi-manifest.json /tmp/public-slices

Run after building the release CLI and native extension. The original full-spec
outcomes remain separate. This secondary audit removes whole operations that
advertise unsupported media; it never rewrites a retained schema or drops one
media alternative from an operation. Every exclusion and original source hash
is saved in projection.json. The generated slices, packages and logs stay in
the output directory. No service API calls are made.
"""
from __future__ import annotations

import argparse
import copy
import json
import os
from pathlib import Path
import subprocess
import sys
from typing import Any

from check_public_openapi import METHODS, ROOT, TY_VERSION, digest, load_yaml, resolve, run, save, source_bytes

SPEC_IDS = ("meilisearch", "aivisspeech")
SAMPLES = ROOT / "pybindings/benchmarks/public-openapi-slice-samples.json"
TYPING = ROOT / "pybindings/benchmarks/public_openapi_slices"


def media_exclusions(document: dict[str, Any], location: str, content: dict[str, Any]) -> list[dict[str, str]]:
    excluded = []
    for media, value in content.items():
        parts = media.split(";")
        base = parts[0].strip().lower()
        for parameter in parts[1:]:
            name, _, parameter_value = parameter.partition("=")
            if name.strip().lower() != "charset" or parameter_value.strip().strip('"').lower() not in {"utf-8", "utf8"}:
                excluded.append({"location": location, "media_type": media, "reason": "unsupported media parameter"})
        if base == "application/json" or (base.startswith("application/") and base.endswith("+json")):
            continue
        if base == "text/plain":
            schema = resolve(document, value.get("schema", {}))
            if isinstance(schema, dict) and (
                schema.get("type") == "string"
                or (isinstance(schema.get("enum"), list) and all(isinstance(item, str) for item in schema["enum"]))
            ):
                continue
            reason = "text/plain schema does not declare a string"
        else:
            reason = "unsupported media type"
        excluded.append({"location": location, "media_type": media, "reason": reason})
    return excluded


def project_document(document: dict[str, Any]) -> tuple[dict[str, Any], list[dict[str, Any]], list[dict[str, Any]]]:
    projected = copy.deepcopy(document)
    paths = {}
    included = []
    excluded = []
    for path, path_item in document["paths"].items():
        if "$ref" in path_item:
            raise ValueError("operation slicing requires inline path items")
        selected = {key: copy.deepcopy(value) for key, value in path_item.items() if key not in METHODS}
        for method, operation in path_item.items():
            if method not in METHODS:
                continue
            body = resolve(document, operation.get("requestBody", {}))
            reasons = media_exclusions(document, "request", body.get("content", {}))
            for status, response in operation["responses"].items():
                if str(status).startswith("x-"):
                    continue
                response = resolve(document, response)
                reasons.extend(media_exclusions(document, "response " + str(status), response.get("content", {})))
            identity = {"method": method.upper(), "path": path, "operation_id": operation.get("operationId")}
            if reasons:
                excluded.append({**identity, "reasons": reasons})
            else:
                selected[method] = copy.deepcopy(operation)
                included.append(identity)
        if any(key in METHODS for key in selected):
            paths[path] = selected
    projected["paths"] = paths
    return projected, included, excluded


def build_slices(manifest: Path, output: Path) -> Path:
    source = json.loads(manifest.read_text())
    if source.get("version") != 1 or not isinstance(source.get("specs"), list):
        raise ValueError("expected version-1 public corpus manifest")
    entries = {}
    for entry in source["specs"]:
        if entry["id"] in entries:
            raise ValueError("duplicate public corpus id: " + entry["id"])
        entries[entry["id"]] = entry
    output.mkdir(parents=True, exist_ok=True)
    projections = []
    for identifier in SPEC_IDS:
        entry = entries[identifier]
        raw = source_bytes(entry, manifest, output)
        if digest(raw) != entry["sha256"]:
            raise ValueError("source SHA-256 mismatch: " + identifier)
        try:
            document = json.loads(raw)
        except (json.JSONDecodeError, UnicodeDecodeError):
            document = load_yaml(raw)
        if not isinstance(document, dict):
            raise ValueError("OpenAPI document must be an object")
        projected, included, excluded = project_document(document)
        name = identifier + "_json_projection"
        path = output / (name + ".json")
        save(path, projected)
        projections.append({
            "id": name, "path": path.name, "sha256": digest(path.read_bytes()),
            "projection_of": identifier, "source_url": entry["source_url"],
            "source_sha256": entry["sha256"], "revision": entry.get("revision"),
            "notes": "Secondary operation slice. source_url/source_sha256 identify the full original; path/sha256 identify the local slice. Reproduce with check_public_openapi_slices.py; downloading source_url alone does not recreate this slice.",
            "transformation": "Remove complete operations with unsupported request or response media; preserve all retained operations, their path-level fields, every component/schema, and every other document field unchanged as JSON values.",
            "original_operations": len(included) + len(excluded), "operations": len(included),
            "included_operations": included, "excluded_operations": excluded,
        })
    save(output / "projection.json", {"version": 1, "projections": projections})
    target = output / "manifest.json"
    save(target, {"version": 1, "specs": projections})
    return target


def check_negative_typing(audit: Path) -> None:
    invalid = audit / "slice_invalid_consumer.py"
    invalid.write_bytes((TYPING / "invalid_consumer.py").read_bytes())
    command = ["uv", "run", "--no-config", "--project", str(ROOT / "pybindings"), "--all-extras", "--locked",
               "--with", "ty==" + TY_VERSION, "ty", "check", "--project", str(audit),
               "--python-version", "3.12", "--extra-search-path", str(ROOT / "pybindings"), "--error", "all", "--output-format", "gitlab"]
    for identifier in SPEC_IDS:
        command.extend(["--extra-search-path", str(audit / (identifier + "_json_projection"))])
    environment = dict(os.environ)
    negative = run(audit, "slice-typecheck-negative", [*command, str(invalid)], 60, environment)
    diagnostics = json.loads((audit / "slice-typecheck-negative.stdout").read_text())
    expected = {9: "invalid-argument-type", 10: "invalid-argument-type", 11: "invalid-argument-type",
                12: "invalid-argument-type", 13: "invalid-assignment", 14: "invalid-assignment"}
    actual = {item["location"]["positions"]["begin"]["line"]: item["check_name"] for item in diagnostics}
    passed = negative["exit_code"] == 1 and len(diagnostics) == len(expected) and actual == expected
    save(audit / "slice-typing.json", {"passed": passed, "negative": negative})
    if not passed:
        raise RuntimeError("slice typing checks failed; see slice-typing.json")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", type=Path)
    parser.add_argument("output", type=Path, help="separate directory for derived slices and their audit")
    arguments = parser.parse_args()
    output = arguments.output.resolve()
    manifest = build_slices(arguments.manifest.resolve(), output)
    audit = output / "audit"
    subprocess.run([sys.executable, str(ROOT / "pybindings/check_public_openapi.py"), str(manifest), str(audit)], check=True)
    results = json.loads((audit / "aggregate.json").read_text())
    if results["counts"] != {"passed": len(SPEC_IDS)}:
        raise RuntimeError("operation-slice audit failed; see audit/aggregate.json")
    subprocess.run([sys.executable, str(ROOT / "pybindings/check_public_openapi_samples.py"), str(audit), "--samples", str(SAMPLES)], check=True)
    check_negative_typing(audit)


if __name__ == "__main__":
    main()
