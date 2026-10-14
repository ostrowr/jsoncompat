"""Compare two release-built worktrees with alternating fresh Python processes.

Both trees must contain a default-feature release extension. The candidate must
also have a CLI at target/debug/jsoncompat (or pass --cli). Generated artifacts,
payloads, interpreter, iteration counts, and GC policy are shared by both runs.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import statistics
import subprocess
import sys
import time
from typing import Any, Callable


def worker(repo: Path, artifacts: Path) -> dict[str, Any]:
    sys.path.insert(0, str(repo / "pybindings"))
    started = time.perf_counter_ns()
    import jsoncompat
    from benchmark_generated_models import load_generated_module, load_generated_path
    runtime_import_us = (time.perf_counter_ns() - started) / 1000
    started = time.perf_counter_ns()
    representative = load_generated_module("representative").JSONCOMPAT_MODEL
    model_import_us = (time.perf_counter_ns() - started) / 1000
    value = json.loads((artifacts / "representative.payload.json").read_text())
    wire = json.dumps(value, separators=(",", ":"), sort_keys=True)
    started = time.perf_counter_ns()
    first = representative.deserialize(wire).serialize()
    first_round_trip_us = (time.perf_counter_ns() - started) / 1000
    assert json.loads(first) == value
    assert jsoncompat.validator_for('{"const":0.10000000000000001}').is_valid_json("0.1"), (
        "benchmark both extensions with arbitrary precision disabled"
    )

    from benchmark_harness import measure
    from bench_dataclasses_scaling import build_payload

    cases = [("representative", representative, value, 10000),
             ("recursive_1365_nodes", load_generated_module("scaling").JSONCOMPAT_MODEL,
              build_payload(5, 4), 100)]
    for name, iterations in [("wide_1000_fields", 200), ("schema_201_classes", 60),
                             ("values_12000_records", 3), ("numbers_4096_decimals", 100)]:
        cases.append((name, load_generated_path(artifacts / f"{name}.py").JSONCOMPAT_MODEL,
                      json.loads((artifacts / f"{name}.payload.json").read_text()), iterations))
    timings: dict[str, float] = {}
    for name, model, value, iterations in cases:
        wire = json.dumps(value, separators=(",", ":"), sort_keys=True)
        assert json.loads(model.deserialize(wire).serialize()) == value
        timings[f"{name}/checked_round_trip"] = measure(
            {"run": lambda: model.deserialize(wire).serialize()}, iterations, 1,
            warmup=3)["run"]["median_us"]
        if name in ("representative", "recursive_1365_nodes", "numbers_4096_decimals"):
            schema = model.__jsoncompat_schema__
            validator = jsoncompat.validator_for(schema)
            callbacks: dict[str, Callable[[], object]] = {
                "validate_json": lambda: validator.is_valid_json(wire),
                "validate_value": lambda: validator.is_valid_value(value),
                "validate_borrowed": lambda: validator._is_valid_borrowed_value(value),  # pyright: ignore[reportPrivateUsage]
            }
            if name == "numbers_4096_decimals":
                accept_all = jsoncompat.validator_for("true")
                callbacks["parse_json_accept_all"] = lambda: accept_all.is_valid_json(wire)
            assert all(callback() for callback in callbacks.values())
            for operation, timing in measure(callbacks, iterations, 1, warmup=3).items():
                timings[f"{name}/{operation}"] = timing["median_us"]
            timings[f"{name}/compile_validator"] = measure(
                {"run": lambda: jsoncompat.validator_for(schema)}, 10, 1,
                warmup=1)["run"]["median_us"]

    native_file = sys.modules["jsoncompat._native"].__file__
    assert native_file is not None
    native = Path(native_file).resolve()
    if not native.is_relative_to(repo / "target" / "release"):
        raise RuntimeError(f"wrong native extension loaded: {native}")
    return {
        "timings_us": timings,
        "startup_us": {"runtime_import": runtime_import_us, "model_import": model_import_us,
                       "first_round_trip": first_round_trip_us},
        "native": str(native), "native_bytes": native.stat().st_size,
        "native_sha256": hashlib.sha256(native.read_bytes()).hexdigest(),
    }


def prepare(candidate: Path, artifacts: Path, cli: Path) -> None:
    sys.path.insert(0, str(candidate / "pybindings"))
    from bench_dataclasses_runtime import PAYLOAD
    from bench_dataclasses_codegen_large import workloads

    artifacts.mkdir(parents=True, exist_ok=True)
    (artifacts / "representative.payload.json").write_text(json.dumps(PAYLOAD))
    cases = workloads()
    cases.append(("numbers_4096_decimals", {"type": "array", "items": {"type": "number"}},
                  [index / 7 + 0.1 for index in range(4096)], [], 100))
    for name, schema, value, _, _ in cases:
        schema_file = artifacts / f"{name}.schema.json"
        schema_file.write_text(json.dumps(schema))
        (artifacts / f"{name}.payload.json").write_text(json.dumps(value))
        subprocess.run([str(cli), "codegen", "--target", "dataclasses", str(schema_file),
                        "--output", str(artifacts / f"{name}.py")],
                       check=True, capture_output=True, text=True)


def git(repo: Path, *args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=repo, text=True).strip()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", type=Path)
    parser.add_argument("--candidate", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--cli", type=Path)
    parser.add_argument("--repeats", type=int, default=7)
    parser.add_argument("--worker", type=Path, help=argparse.SUPPRESS)
    args = parser.parse_args()
    artifacts = args.output.resolve().parent / "artifacts"
    if args.worker:
        print(json.dumps(worker(args.worker.resolve(), artifacts)))
        return
    if args.baseline is None or args.repeats < 1:
        parser.error("--baseline and a positive --repeats are required")
    candidate = args.candidate.resolve()
    baseline = args.baseline.resolve()
    prepare(candidate, artifacts, (args.cli or candidate / "target/debug/jsoncompat").resolve())
    samples: dict[str, list[dict[str, Any]]] = {"fork": [], "upstream": []}
    trees = {"fork": baseline, "upstream": candidate}
    for repeat in range(args.repeats):
        order = ("fork", "upstream") if repeat % 2 == 0 else ("upstream", "fork")
        for name in order:
            repo = trees[name]
            environment = dict(os.environ, JSONCOMPAT_NATIVE_PROFILE="release", PYTHONHASHSEED="0",
                               CARGO_TARGET_DIR=str(repo / "target"), PYTHONPATH=str(repo / "pybindings"))
            result = subprocess.check_output(
                [sys.executable, str(Path(__file__).resolve()), "--worker", str(repo),
                 "--output", str(args.output.resolve())], cwd=repo, env=environment, text=True)
            samples[name].append(json.loads(result))
            print(f"round {repeat + 1}/{args.repeats}: {name} complete", flush=True)

    comparisons: dict[str, dict[str, dict[str, float]]] = {}
    for group in ("timings_us", "startup_us"):
        comparisons[group] = {}
        for case in samples["fork"][0][group]:
            medians = {name: statistics.median(run[group][case] for run in runs)
                       for name, runs in samples.items()}
            comparisons[group][case] = {**medians, "upstream_over_fork": medians["upstream"] / medians["fork"]}
    report = {
        "python": sys.version, "platform": platform.platform(), "repeats": args.repeats,
        "method": "alternating fresh processes; default precision; release; GC disabled during steady-state timing",
        "trees": {name: {"path": str(repo), "commit": git(repo, "rev-parse", "HEAD"),
                         "dirty": bool(git(repo, "status", "--porcelain"))} for name, repo in trees.items()},
        "comparisons": comparisons, "samples": samples,
    }
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    for case, result in comparisons["timings_us"].items():
        print(f"{case}: fork={result['fork']:.3f}us upstream={result['upstream']:.3f}us ratio={result['upstream_over_fork']:.3f}")


if __name__ == "__main__":
    main()
