"""Stress prepared models with large schemas and multi-megabyte JSON values.

Only the JSON Schema subset explicitly implemented by the Pydantic peer builder
is used. Both implementations enforce all emitted constraints. The benchmark
screens invalid inputs before measuring fully checked JSON round trips.
"""

from __future__ import annotations

import argparse
import copy
import json
import os
import statistics
import subprocess
import sys
import time
from pathlib import Path
from typing import Annotated, Any

from bench_dataclasses_codegen import build_models
from benchmark_harness import measure, provenance
from benchmark_generated_models import load_generated_path
from pydantic import ConfigDict, Field, RootModel, create_model

REPO = Path(__file__).resolve().parents[1]


def record_schema(width: int) -> dict[str, Any]:
    properties = {
        f"field{index:04}": (
            {"type": "string", "minLength": 1}
            if index % 2 == 0
            else {"type": "integer", "minimum": 0}
        )
        for index in range(width)
    }
    return {
        "type": "object",
        "properties": properties,
        "required": list(properties),
        "additionalProperties": False,
    }


def record_value(width: int, row: int = 0) -> dict[str, Any]:
    return {
        f"field{index:04}": (
            f"row-{row}-field-{index}-" + "abc🐲" * 12
            if index % 2 == 0
            else row + index
        )
        for index in range(width)
    }


def workloads() -> list[tuple[str, dict[str, Any], Any, list[str | int], int]]:
    many = {f"section{index:04}": record_schema(12) for index in range(200)}
    return [
        (
            "wide_1000_fields",
            record_schema(1000),
            record_value(1000),
            ["field0998"],
            500,
        ),
        (
            "schema_201_classes",
            {
                "type": "object",
                "properties": many,
                "required": list(many),
                "additionalProperties": False,
            },
            {key: record_value(12, index) for index, key in enumerate(many)},
            ["section0199", "field0010"],
            150,
        ),
        (
            "values_12000_records",
            {"type": "array", "items": record_schema(12)},
            [record_value(12, index) for index in range(12000)],
            [11999, "field0010"],
            5,
        ),
    ]


def pydantic_model(schema: dict[str, Any]) -> type[Any]:
    def annotation(node: dict[str, Any], name: str) -> Any:
        kind = node["type"]
        if kind == "object":
            assert node["additionalProperties"] is False
            assert set(node["required"]) == set(node["properties"])
            fields = {
                key: (annotation(value, name + key), ...)
                for key, value in node["properties"].items()
            }
            return create_model(
                name,
                __config__=ConfigDict(strict=True, frozen=True, extra="forbid"),
                **fields,
            )
        if kind == "array":
            return list[annotation(node["items"], name + "Item")]
        if kind == "integer":
            return Annotated[int, Field(ge=node["minimum"])]
        if kind == "string":
            return Annotated[str, Field(min_length=node["minLength"])]
        raise ValueError(f"Unimplemented Pydantic peer schema: {node}")

    model = annotation(schema, "LargeModel")
    if schema["type"] == "object":
        return model
    return type(
        "LargeRoot",
        (RootModel[model],),
        {"model_config": ConfigDict(strict=True, frozen=True)},
    )


def probe(schema_path: Path, model_path: str, payload_path: Path) -> None:
    schema = json.loads(schema_path.read_text())
    payload = payload_path.read_text()
    start = time.perf_counter_ns()
    model = (
        pydantic_model(schema)
        if model_path == "pydantic"
        else load_generated_path(Path(model_path)).JSONCOMPAT_MODEL
    )
    loaded = time.perf_counter_ns()
    if model_path == "pydantic":
        model.model_validate_json(payload).model_dump_json(exclude_unset=True)
    else:
        model.deserialize(payload).serialize()
    used = time.perf_counter_ns()
    print(
        json.dumps(
            {
                "import_ms": (loaded - start) / 1e6,
                "first_roundtrip_ms": (used - loaded) / 1e6,
            }
        )
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, default=REPO / "target/release/jsoncompat")
    parser.add_argument("--repeats", type=int, default=9)
    parser.add_argument("--startup-repeats", type=int, default=5)
    parser.add_argument(
        "--output", type=Path, default=REPO / "target/python-codegen/large/benchmark.json"
    )
    parser.add_argument("--assert-target", action="store_true")
    parser.add_argument("--probe", nargs=3, metavar=("SCHEMA", "MODEL", "PAYLOAD"))
    args = parser.parse_args()
    if args.probe:
        probe(Path(args.probe[0]), args.probe[1], Path(args.probe[2]))
        return
    if os.environ.get("JSONCOMPAT_NATIVE_PROFILE") != "release":
        parser.error(
            "set JSONCOMPAT_NATIVE_PROFILE=release and build the release extension"
        )
    if not args.cli.is_file():
        parser.error("build the CLI first with cargo build, or pass --cli")
    if min(args.repeats, args.startup_repeats) <= 0:
        parser.error("repeat counts must be positive")
    directory = args.output.resolve().parent
    directory.mkdir(parents=True, exist_ok=True)
    report: dict[str, Any] = {
        **provenance(REPO),
        "cases": {},
    }
    met_target = True
    for name, schema, value, invalid_path, iterations in workloads():
        schema_path = directory / f"{name}.json"
        payload_path = directory / f"{name}_payload.json"
        destination = directory / f"prepared_{name}.py"
        schema_path.write_text(json.dumps(schema))
        payload = json.dumps(value, separators=(",", ":"), ensure_ascii=False)
        payload_path.write_text(payload)
        build = build_models(args.cli, schema_path, destination)
        build_ms = build["build_ms"]
        prepared = load_generated_path(destination).JSONCOMPAT_MODEL
        peer = pydantic_model(schema)
        # Invalid input at the last field checks complete traversal, not just
        # prefix acceptance. Both length and required-property checks apply.
        invalid = copy.deepcopy(value)
        parent = invalid
        for component in invalid_path[:-1]:
            parent = parent[component]
        parent[invalid_path[-1]] = ""
        for bad in (invalid, None, {}):
            wire = json.dumps(bad)
            for decode in (
                prepared.deserialize,
                peer.model_validate_json,
            ):
                try:
                    decode(wire)
                except (ValueError, TypeError):
                    pass
                else:
                    raise AssertionError(f"{name} accepted invalid input")
        callbacks = {
            "prepared_checked": lambda: prepared.deserialize(payload).serialize(),
            "pydantic": lambda: peer.model_validate_json(payload).model_dump_json(
                exclude_unset=True
            ),
        }
        for callback in callbacks.values():
            assert json.loads(callback()) == value
        timings = measure(callbacks, iterations, args.repeats)
        ratio = (
            timings["prepared_checked"]["median_us"] / timings["pydantic"]["median_us"]
        )
        startup: dict[str, list[dict[str, float]]] = {
            key: [] for key in ("prepared", "pydantic")
        }
        models = {
            "prepared": str(destination),
            "pydantic": "pydantic",
        }
        for repeat in range(args.startup_repeats):
            names = list(models)
            offset = repeat % len(names)
            for key in names[offset:] + names[:offset]:
                child = subprocess.run(
                    [
                        sys.executable,
                        str(Path(__file__).resolve()),
                        "--probe",
                        str(schema_path),
                        models[key],
                        str(payload_path),
                    ],
                    check=True,
                    capture_output=True,
                    text=True,
                )
                startup[key].append(json.loads(child.stdout))
        report["cases"][name] = {
            "schema_bytes": schema_path.stat().st_size,
            "payload_bytes": len(payload.encode()),
            **build,
            "iterations": iterations,
            "repeats": args.repeats,
            "timings": timings,
            "ratio": ratio,
            "startup": {
                key: {
                    "samples": values,
                    **{
                        metric: statistics.median(sample[metric] for sample in values)
                        for metric in ("import_ms", "first_roundtrip_ms")
                    },
                }
                for key, values in startup.items()
            },
        }
        print(
            f"{name}: schema={schema_path.stat().st_size:,}B, payload={len(payload.encode()):,}B, prepared/Pydantic={ratio:.3f}, build={build_ms:.1f}ms",
            flush=True,
        )
        for key, result in report["cases"][name]["startup"].items():
            print(
                f"  {key}: model import={result['import_ms']:.2f}ms, first round trip={result['first_roundtrip_ms']:.2f}ms",
                flush=True,
            )
        args.output.write_text(json.dumps(report, indent=2) + "\n")
        met_target &= ratio <= 0.70
    if args.assert_target and not met_target:
        raise SystemExit(
            "At least one large workload missed the 30% elapsed-time target; see the report."
        )


if __name__ == "__main__":
    main()
