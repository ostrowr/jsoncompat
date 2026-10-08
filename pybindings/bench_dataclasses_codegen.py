"""Compare generated models and strict Pydantic peers, including codegen and startup.

Run after building the release extension. Each timed round trip validates JSON
input and emits JSON with omitted fields preserved. Jsoncompat also validates
current model state on output. Build time and cold startup are measured apart
from steady state. Rotating execution order reduces systematic timing bias.
"""

from __future__ import annotations

import argparse
import ast
import gc
import json
import os
import platform
import statistics
import subprocess
import sys
import time
from pathlib import Path
from typing import Any, Callable

import bench_dataclasses_runtime as small
import bench_dataclasses_scaling as large
import pydantic
from bench_dataclasses_startup import PYDANTIC_PEER_SOURCE
from benchmark_generated_models import MODEL_ROOT, load_generated_path
import py_compile

REPO = Path(__file__).resolve().parents[1]


def build_models(cli: Path, schema: Path, destination: Path) -> dict[str, float | int]:
    """Time the production split-file build, including ordinary Python bytecode."""
    start = time.perf_counter_ns()
    subprocess.run([
        str(cli.resolve()), "codegen", "--target", "dataclasses", str(schema),
        "--output", str(destination),
    ], check=True, capture_output=True, text=True)
    source = destination.read_text()
    companion_name = next(
        node.module for node in ast.walk(ast.parse(source))
        if isinstance(node, ast.ImportFrom)
        and any(alias.name == "bind_models" for alias in node.names)
    )
    companion = destination.with_name(companion_name + ".py")
    for path in (destination, companion):
        py_compile.compile(str(path), doraise=True)
    return {
        "build_ms": (time.perf_counter_ns() - start) / 1_000_000,
        "public_source_bytes": destination.stat().st_size,
        "implementation_source_bytes": companion.stat().st_size,
    }


def measure(
    callbacks: dict[str, Callable[[], object]], iterations: int, repeats: int
) -> dict[str, Any]:
    names = list(callbacks)
    samples: dict[str, list[float]] = {name: [] for name in names}
    for callback in callbacks.values():
        for _ in range(50):
            callback()
    gc_was_enabled = gc.isenabled()
    gc.disable()
    try:
        for repeat in range(repeats):
            offset = repeat % len(names)
            for name in names[offset:] + names[:offset]:
                callback = callbacks[name]
                start = time.perf_counter_ns()
                for _ in range(iterations):
                    callback()
                samples[name].append(
                    (time.perf_counter_ns() - start) / iterations / 1000
                )
    finally:
        if gc_was_enabled:
            gc.enable()
    return {
        name: {"median_us": statistics.median(values), "samples_us": values}
        for name, values in samples.items()
    }


def startup(prepared: Path, repeats: int) -> dict[str, Any]:
    payload = json.dumps(small.PAYLOAD, separators=(",", ":"))
    cases = {
        "baseline": "pass",
        "runtime_import": "import jsoncompat.codegen.dataclasses",
        "prepared_import": "import prepared_representative",
        "prepared_first_roundtrip": f"import prepared_representative\nprepared_representative.JSONCOMPAT_MODEL.deserialize({payload!r}).serialize()",
        "pydantic_import": PYDANTIC_PEER_SOURCE,
        "pydantic_first_roundtrip": PYDANTIC_PEER_SOURCE
        + f"\nPydanticEvent.model_validate_json({payload!r}).model_dump_json(exclude_unset=True)",
    }
    environment = dict(os.environ)
    environment["PYTHONPATH"] = os.pathsep.join(
        (str(MODEL_ROOT), str(prepared), str(REPO / "pybindings"))
    )
    environment["PYTHONHASHSEED"] = "0"
    samples: dict[str, list[float]] = {name: [] for name in cases}
    names = list(cases)
    for repeat in range(repeats):
        offset = repeat % len(names)
        for name in names[offset:] + names[:offset]:
            start = time.perf_counter_ns()
            subprocess.run(
                [sys.executable, "-c", cases[name]],
                env=environment,
                check=True,
                capture_output=True,
            )
            samples[name].append((time.perf_counter_ns() - start) / 1_000_000)
    return {
        name: {"median_ms": statistics.median(values), "samples_ms": values}
        for name, values in samples.items()
    }


def model_startup(prepared: Path, repeats: int) -> dict[str, Any]:
    """Separate model loading and the first round trip after runtime import."""
    payload = json.dumps(small.PAYLOAD, separators=(",", ":"))
    cases = {
        "prepared": (
            "import jsoncompat.codegen.dataclasses",
            "import prepared_representative",
            f"prepared_representative.JSONCOMPAT_MODEL.deserialize({payload!r}).serialize()",
        ),
        "pydantic": (
            "from pydantic import BaseModel\nclass Warmup(BaseModel):\n    value: int",
            PYDANTIC_PEER_SOURCE,
            f"PydanticEvent.model_validate_json({payload!r}).model_dump_json(exclude_unset=True)",
        ),
    }
    environment = dict(os.environ)
    environment["PYTHONPATH"] = os.pathsep.join(
        (str(MODEL_ROOT), str(prepared), str(REPO / "pybindings"))
    )
    samples: dict[str, list[dict[str, float]]] = {name: [] for name in cases}
    names = list(cases)
    for repeat in range(repeats):
        offset = repeat % len(names)
        for name in names[offset:] + names[:offset]:
            runtime, model, first_use = cases[name]
            source = f"""import json, time
{runtime}
start = time.perf_counter_ns()
{model}
loaded = time.perf_counter_ns()
{first_use}
used = time.perf_counter_ns()
print(json.dumps(dict(import_us=(loaded-start)/1000, first_roundtrip_us=(used-loaded)/1000)))
"""
            process = subprocess.run(
                [sys.executable, "-c", source],
                env=environment,
                check=True,
                capture_output=True,
                text=True,
            )
            samples[name].append(json.loads(process.stdout))
    return {
        name: {
            "samples": values,
            **{
                key: statistics.median(value[key] for value in values)
                for key in ("import_us", "first_roundtrip_us")
            },
        }
        for name, values in samples.items()
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, default=REPO / "target/release/jsoncompat")
    parser.add_argument("--iterations", type=int, default=10000)
    parser.add_argument("--tree-iterations", type=int, default=200)
    parser.add_argument("--repeats", type=int, default=9)
    parser.add_argument("--startup-repeats", type=int, default=15)
    parser.add_argument("--depth", type=int, default=5)
    parser.add_argument("--fanout", type=int, default=4)
    parser.add_argument(
        "--output", type=Path, default=REPO / "target/python-codegen/benchmark.json"
    )
    parser.add_argument(
        "--assert-target",
        action="store_true",
        help="fail unless BOTH prepared round trips take at most 70%% of Pydantic's time",
    )
    args = parser.parse_args()
    if (
        min(
            args.iterations,
            args.tree_iterations,
            args.repeats,
            args.startup_repeats,
            args.fanout,
        )
        <= 0
        or args.depth < 0
    ):
        parser.error(
            "iteration counts and fanout must be positive; depth must be nonnegative"
        )
    if os.environ.get("JSONCOMPAT_NATIVE_PROFILE") != "release":
        parser.error(
            "set JSONCOMPAT_NATIVE_PROFILE=release and build the release extension first"
        )
    output_dir = args.output.resolve().parent
    output_dir.mkdir(parents=True, exist_ok=True)
    report: dict[str, Any] = {
        "python": platform.python_version(),
        "pydantic": pydantic.__version__,
        "platform": platform.platform(),
        "native_profile": "release",
        "target": "prepared/Pydantic elapsed time <= 0.70 on each case",
        "cases": {},
    }
    met_target = True
    for name, peer, value, iterations in [
        (
            "representative",
            small.PydanticEvent,
            small.PAYLOAD,
            args.iterations,
        ),
        (
            "scaling",
            large.PydanticTree,
            large.build_payload(args.depth, args.fanout),
            args.tree_iterations,
        ),
    ]:
        destination = output_dir / f"prepared_{name}.py"
        build = build_models(args.cli, REPO / "pybindings/benchmark_schemas" / f"{name}.json", destination)
        build_ms = build["build_ms"]
        prepared = load_generated_path(destination).JSONCOMPAT_MODEL
        payload = json.dumps(value, separators=(",", ":"), sort_keys=True)
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
        report["cases"][name] = {
            **build,
            "payload_bytes": len(payload.encode()),
            "iterations": iterations,
            "repeats": args.repeats,
            "timings": timings,
            "ratio": ratio,
        }
        if name == "scaling":
            report["cases"][name].update(
                depth=args.depth,
                fanout=args.fanout,
                nodes=large.node_count(args.depth, args.fanout),
            )
        print(
            f"{name}: prepared={timings['prepared_checked']['median_us']:.3f}us, Pydantic={timings['pydantic']['median_us']:.3f}us, ratio={ratio:.3f}, build={build_ms:.1f}ms",
            flush=True,
        )
        met_target &= ratio <= 0.70
    report["startup"] = startup(output_dir, args.startup_repeats)
    for name, data in report["startup"].items():
        print(f"{name}: {data['median_ms']:.2f}ms (fresh process)")
    report["model_startup"] = model_startup(output_dir, args.startup_repeats)
    for name, data in report["model_startup"].items():
        print(
            f"{name}: model import={data['import_us']:.1f}us, first round trip={data['first_roundtrip_us']:.1f}us (runtime preloaded)"
        )
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    if args.assert_target and not met_target:
        raise SystemExit(
            "The 30% elapsed-time target was not met on both workloads; see the report."
        )


if __name__ == "__main__":
    main()
