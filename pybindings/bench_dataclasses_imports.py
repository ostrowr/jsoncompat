"""Measure retained independent classes with bounded memory and elapsed time.

Build 100-class shards first, then load fresh class identities from the cached
shard under distinct module names. This isolates class setup and retained memory;
it excludes a real package's dependency graph and unique-file disk I/O.
"""

from __future__ import annotations

import argparse
import gc
import importlib.util
import json
import os
from pathlib import Path
import platform
import py_compile
import resource
import sys
import time

from bench_dataclasses_codegen import build_models
from bench_dataclasses_codegen_large import record_schema, record_value

REPO = Path(__file__).resolve().parents[1]


def load(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def rss_mib() -> float:
    units = 1 if sys.platform == "darwin" else 1024
    return resource.getrusage(resource.RUSAGE_SELF).ru_maxrss * units / 1024**2


def build(args) -> None:
    args.directory.mkdir(parents=True, exist_ok=True)
    sys.path.insert(0, str(args.directory))
    names = [f"Record{index:04}" for index in range(args.shard_size)]
    def independent_schema(index: int) -> dict:
        schema = dict(record_schema(args.fields), title=names[index])
        if args.distinct_constraints:
            for field in schema["properties"].values():
                if field["type"] == "integer":
                    field["minimum"] = index
                else:
                    field["minLength"] = 1 + index % 32
        return schema

    schema = independent_schema(0)
    schema["$defs"] = {
        name: independent_schema(index) for index, name in enumerate(names) if index
    }
    source = args.directory / f"schema_{args.fields}.json"
    source.write_text(json.dumps(schema))
    generated = args.directory / f"generated_{args.fields}.py"
    metadata = build_models(args.cli, source, generated)
    peer = args.directory / f"pydantic_{args.fields}.py"
    lines = ["from pydantic import BaseModel, ConfigDict, Field"]
    for model_index, name in enumerate(names):
        lines += [f"class {name}(BaseModel):", '    model_config = ConfigDict(frozen=True, strict=True, extra="forbid")']
        for index in range(args.fields):
            minimum = model_index if args.distinct_constraints else 0
            minimum_length = 1 + model_index % 32 if args.distinct_constraints else 1
            annotation, constraint = ("str", f"min_length={minimum_length}") if index % 2 == 0 else ("int", f"ge={minimum}")
            lines.append(f"    field{index:04}: {annotation} = Field({constraint})")
    lines += [f"__all__ = {tuple(names)!r}"]
    peer.write_text("\n".join(lines) + "\n")
    py_compile.compile(str(peer), doraise=True)
    for kind, path in (("generated", generated), ("pydantic", peer)):
        module = load(path, f"check_{kind}_{args.fields}")
        models = [getattr(module, name) for name in module.__all__ if name != "JSONCOMPAT_MODEL"]
        assert len(models) == args.shard_size
        model = models[-1]
        decode = model.deserialize if kind == "generated" else model.model_validate_json
        payload = record_value(args.fields, args.shard_size - 1 if args.distinct_constraints else 0)
        instance = decode(json.dumps(payload))
        emitted = instance.serialize() if kind == "generated" else instance.model_dump_json()
        assert json.loads(emitted) == payload
        invalid = [{}, dict(payload, field0000=""), dict(payload, unexpected=1)]
        if args.distinct_constraints and args.shard_size > 1:
            if args.fields > 1:
                invalid.append(dict(payload, field0001=args.shard_size - 2))
            if (args.shard_size - 1) % 32:
                invalid.append(dict(payload, field0000="x"))
        for bad in invalid:
            try:
                decode(json.dumps(bad))
            except (TypeError, ValueError):
                pass
            else:
                raise AssertionError(f"{kind} accepted invalid input")
    metadata.update(fields=args.fields, distinct_constraints=args.distinct_constraints, classes_per_shard=args.shard_size, pydantic_source_bytes=peer.stat().st_size)
    (args.directory / f"build_{args.fields}.json").write_text(json.dumps(metadata, indent=2) + "\n")
    print(json.dumps(metadata), flush=True)


def worker(args) -> None:
    import pydantic
    from pydantic import BaseModel
    import jsoncompat.codegen.dataclasses  # Preload both runtimes equally.

    class Warmup(BaseModel):
        value: int

    sys.path.insert(0, str(args.directory))
    gc.collect()
    baseline = rss_mib()
    print(json.dumps(dict(event="baseline", kind=args.mode, fields=args.fields, distinct_constraints=args.distinct_constraints,
                          peak_rss_mib=baseline, python=platform.python_version(),
                          pydantic=pydantic.__version__, platform=platform.platform(),
                          gc_enabled=gc.isenabled())), flush=True)
    path = args.directory / f"{args.mode}_{args.fields}.py"
    modules = []
    start = time.perf_counter()
    setup_seconds = 0.0
    checkpoints = {1000, 5000, 10000, 20000, 50000, 100000, 200000}
    loaded = 0
    while loaded < args.classes:
        begin = time.perf_counter()
        module = load(path, f"scale_{args.mode}_{args.fields}_{len(modules)}")
        setup_seconds += time.perf_counter() - begin
        modules.append(module)
        loaded += args.shard_size
        peak = rss_mib()
        elapsed = time.perf_counter() - start
        stop = "memory_budget" if peak >= args.memory_mib else "time_budget" if elapsed >= args.seconds else None
        if loaded in checkpoints or stop or loaded >= args.classes:
            print(json.dumps(dict(event="measurement", kind=args.mode, fields=args.fields,
                                  classes=loaded, import_seconds=setup_seconds,
                                  elapsed_seconds=elapsed, peak_rss_mib=peak,
                                  added_peak_mib=peak-baseline, modules=len(modules), stop=stop)), flush=True)
        if stop:
            break
    identities = {
        id(getattr(module, name)) for module in modules for name in module.__all__
        if name != "JSONCOMPAT_MODEL"
    }
    assert len(identities) == loaded
    model = getattr(modules[-1], f"Record{args.shard_size-1:04}")
    payload = json.dumps(record_value(args.fields, args.shard_size - 1 if args.distinct_constraints else 0))
    begin = time.perf_counter()
    if args.mode == "pydantic":
        model.model_validate_json(payload).model_dump_json()
    else:
        model.deserialize(payload).serialize()
    print(json.dumps(dict(event="first_use", seconds=time.perf_counter()-begin)), flush=True)
    # Exclude interpreter teardown; every class stayed alive during measurement.
    os._exit(0)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["build", "generated", "pydantic"])
    parser.add_argument("--fields", type=int, required=True)
    parser.add_argument("--distinct-constraints", action="store_true", help="give each class distinct numeric/string bounds")
    parser.add_argument("--shard-size", type=int, default=100)
    parser.add_argument("--classes", type=int, default=200000)
    parser.add_argument("--memory-mib", type=int, default=2048)
    parser.add_argument("--seconds", type=int, default=180)
    parser.add_argument("--cli", type=Path, default=REPO / "target/release/jsoncompat")
    parser.add_argument("--directory", type=Path, default=REPO / "target/python-codegen/imports")
    args = parser.parse_args()
    args.directory = args.directory.resolve()
    if min(args.fields, args.shard_size, args.classes, args.memory_mib, args.seconds) < 1:
        parser.error("counts and budgets must be positive")
    if args.classes % args.shard_size:
        parser.error("class count must be a multiple of shard size")
    if os.environ.get("JSONCOMPAT_NATIVE_PROFILE") != "release":
        parser.error("set JSONCOMPAT_NATIVE_PROFILE=release and build the release extension")
    if args.mode == "build":
        build(args)
    else:
        worker(args)
