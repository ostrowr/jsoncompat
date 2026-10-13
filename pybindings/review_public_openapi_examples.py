"""Independently validate examples rejected by the public OpenAPI corpus audit.

    uv run --no-config --project pybindings --all-extras --locked \
        --with jsonschema==4.26.0 python pybindings/review_public_openapi_examples.py /tmp/audit

This is an opt-in audit dependency, not a jsoncompat runtime dependency. Checks
use original request/response schemas and references, without codegen's lowering.
Every rejected inline example must also fail python-jsonschema validation; an
example that independently validates is reported as a possible codegen defect.
"""
from __future__ import annotations

import argparse
from collections import Counter
from importlib.metadata import version
import json
from pathlib import Path
from typing import Any

from check_public_openapi import save


def at(document: dict[str, Any], pointer: str) -> Any:
    if not pointer.startswith("#/"):
        raise ValueError("expected a local source JSON pointer")
    value: Any = document
    for part in pointer[2:].split("/"):
        token = part.replace("~1", "/").replace("~0", "~")
        value = value[int(token)] if isinstance(value, list) else value[token]
    return value


def resolve(document: dict[str, Any], value: dict[str, Any]) -> dict[str, Any]:
    seen: set[str] = set()
    while "$ref" in value:
        reference = value["$ref"]
        if reference in seen:
            raise ValueError("cyclic OpenAPI response reference")
        seen.add(reference)
        value = {**at(document, reference), **{key: item for key, item in value.items() if key != "$ref"}}
    return value


def main() -> None:
    from jsonschema import Draft202012Validator

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("audit", type=Path)
    args = parser.parse_args()
    directory = args.audit.resolve()
    audit = json.loads((directory / "aggregate.json").read_text())
    rows = []
    for entry in audit["specs"]:
        report = entry.get("runtime_report", {})
        rejected = [sample for sample in report.get("examples", [])
                    if sample["outcome"] == "example_rejected_needs_review"]
        if not rejected:
            continue
        document = json.loads((directory / entry["id"] / "source.json").read_text())
        for sample in rejected:
            value = at(document, sample["origin"])
            path_item = resolve(document, document["paths"][sample["path"]])
            operation = path_item[sample["method"].lower()]
            response = resolve(document, operation["responses"][str(sample["declared_status"])])
            schema = response["content"][sample["media_type"]]["schema"]
            # Retain the original document as reference context. OpenAPI fields
            # are annotations/unknown keywords to the JSON Schema validator.
            validator = Draft202012Validator({**document, "allOf": [schema]})
            errors = list(validator.iter_errors(value))
            rows.append({
                "spec": entry["id"], "method": sample["method"], "path": sample["path"],
                "status": sample["actual_status"], "origin": sample["origin"],
                "independently_invalid": bool(errors),
                "errors": [{"instance_path": list(error.absolute_path),
                            "schema_path": list(error.absolute_schema_path), "message": error.message}
                           for error in errors],
            })
    result = {"validator": "python-jsonschema Draft202012Validator", "validator_version": version("jsonschema"),
              "counts": dict(Counter(row["spec"] for row in rows)), "examples": rows}
    save(directory / "upstream-example-review.json", result)
    print(json.dumps(result["counts"], sort_keys=True))
    if any(not row["independently_invalid"] for row in rows):
        raise SystemExit("some rejected examples satisfy the original schema; inspect upstream-example-review.json")


if __name__ == "__main__":
    main()
