"""Build unique OpenAPI components and measure actual package imports.

Each class has its own constraints and source. Runs stop at a memory budget;
partial populations are reported, never extrapolated to 200,000 classes.
"""
from __future__ import annotations
import argparse
import compileall
import importlib
import hashlib
import json
import os
from pathlib import Path
import resource
import subprocess
import sys
import tempfile
import time
from benchmark_harness import measure, provenance

REPO = Path(__file__).resolve().parents[1]


def width(index: int, fields: int) -> int:
    return fields or (5, 20, 50, 100, 200)[index % 5]


def record(index: int, fields: int) -> tuple[dict, dict]:
    properties = {}
    value = {}
    for field in range(width(index, fields)):
        key = f"field{field:04}"
        properties[key] = {"type": "integer", "minimum": index + field} if field % 2 else {"type": "string", "minLength": 1 + index % 8}
        value[key] = index + field if field % 2 else "x" * (1 + index % 8)
    return dict(type="object", properties=properties, required=list(properties), additionalProperties=False), value


def build(args) -> dict:
    args.directory.mkdir(parents=True, exist_ok=True)
    schema = args.directory / "openapi.json"
    # Stream the input to avoid a second full schema tree in the benchmark driver.
    with schema.open("w") as stream:
        stream.write('{"openapi":"3.1.0","info":{"title":"Unique model scale","version":"1"},"paths":{},"components":{"schemas":{')
        for i in range(args.classes):
            if i:
                stream.write(",")
            stream.write(json.dumps(f"Model{i:07}") + ":" + json.dumps(record(i, args.fields)[0], separators=(",", ":")))
        stream.write("}}}")
    start = time.perf_counter()
    subprocess.run([str(args.cli.resolve()), "codegen", "--openapi", str(schema), "--output", str(args.directory / "generated"), "--models-per-module", str(args.module_size)], check=True, timeout=args.timeout)
    generated_seconds = time.perf_counter() - start
    compiler_peak = resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss / (1024 * 1024 if sys.platform == "darwin" else 1024)
    peer = args.directory / "pydantic_models"
    peer.mkdir(exist_ok=True)
    exports = {}
    start = time.perf_counter()
    for begin in range(0, args.classes, args.module_size):
        module = f"models_{begin // args.module_size:04}"
        lines = ["from pydantic import BaseModel, ConfigDict, Field"]
        for i in range(begin, min(args.classes, begin + args.module_size)):
            name = f"Model{i:07}"
            exports[name] = module
            lines += [f"class {name}(BaseModel):", "    model_config = ConfigDict(extra='forbid')"]
            for field in range(width(i, args.fields)):
                annotation, constraint = ("int", f"ge={i+field}") if field % 2 else ("str", f"min_length={1+i%8}")
                lines.append(f"    field{field:04}: {annotation} = Field({constraint})")
        (peer / f"{module}.py").write_text("\n".join(lines) + "\n")
    # Use the exact same lazy namespace policy for the Pydantic baseline.
    (peer / "__init__.py").write_text((args.directory / "generated/__init__.py").read_text())
    (peer / "_exports.py").write_text("MODELS = " + repr(exports) + "\n")
    peer_seconds = time.perf_counter() - start
    start = time.perf_counter()
    assert compileall.compile_dir(args.directory / "generated", quiet=2)
    assert compileall.compile_dir(peer, quiet=2)
    return dict(generated_build_seconds=generated_seconds, compiler_peak_rss_mib=compiler_peak, input_bytes=schema.stat().st_size, pydantic_source_seconds=peer_seconds,
                bytecode_seconds=time.perf_counter()-start,
                artifacts={kind: {"source_bytes":sum(p.stat().st_size for p in (args.directory/kind).rglob("*.py")),
                                  "bytecode_bytes":sum(p.stat().st_size for p in (args.directory/kind).rglob("*.pyc"))}
                           for kind in ("generated", "pydantic_models")})


def rss_mib() -> float:
    peak = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    return peak / (1024 * 1024 if sys.platform == "darwin" else 1024)


def worker(args) -> None:
    sys.path.insert(0, str(args.directory.resolve()))
    start = time.perf_counter()
    module = importlib.import_module(args.kind)
    package_seconds = time.perf_counter() - start
    start = time.perf_counter()
    model = getattr(module, "Model0000000")
    payload = json.dumps(record(0, args.fields)[1])
    def round_trip(cls, payload):
        return cls.deserialize(payload).serialize() if args.kind == "generated" else cls.model_validate_json(payload).model_dump_json()
    assert json.loads(round_trip(model, payload)) == json.loads(payload)
    first_seconds = time.perf_counter() - start
    start = time.perf_counter()
    loaded = 0
    for i in range(args.classes):
        getattr(module, f"Model{i:07}")
        loaded += 1
        if (i + 1) % args.module_size == 0 and rss_mib() >= args.memory_mib:
            break
    all_seconds = time.perf_counter() - start
    sample_ids = sorted({0, loaded // 2, loaded - 1})
    callbacks = {}
    for i in sample_ids:
        cls = getattr(module, f"Model{i:07}")
        value = record(i, args.fields)[1]
        payload = json.dumps(value, separators=(",", ":"))
        assert json.loads(round_trip(cls, payload)) == value
        for bad in ({}, dict(value, field0000=""), dict(value, unexpected=1)):
            try:
                round_trip(cls, json.dumps(bad))
            except (ValueError, TypeError):
                pass
            else:
                raise AssertionError("invalid benchmark input accepted")
        callbacks[str(i)] = lambda cls=cls, payload=payload: round_trip(cls, payload)
    print(json.dumps(dict(kind=args.kind, package_import_seconds=package_seconds,
          first_access_round_trip_seconds=first_seconds, population_import_seconds=all_seconds,
          loaded_classes=loaded, requested_classes=args.classes, complete=loaded == args.classes,
          peak_rss_mib=rss_mib(), round_trip=measure(callbacks, 2000, 7))), flush=True)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--classes", type=int, default=1000)
    parser.add_argument("--fields", type=int, default=0, help="0 cycles through 5,20,50,100,200 fields")
    parser.add_argument("--module-size", type=int, default=128)
    parser.add_argument("--memory-mib", type=int, default=2048)
    parser.add_argument("--timeout", type=int, default=3600)
    parser.add_argument("--directory", type=Path, default=REPO / "target/python-codegen/unique-imports")
    parser.add_argument("--cli", type=Path, default=REPO / "target/release/jsoncompat")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--build-only", action="store_true", help="build artifacts without runtime measurements")
    mode.add_argument("--reuse-build", action="store_true", help="measure an existing build with matching inputs and compiler")
    parser.add_argument("--worker", action="store_true", help=argparse.SUPPRESS)
    parser.add_argument("--kind", choices=["generated", "pydantic_models"], default="generated")
    args = parser.parse_args()
    if min(args.classes, args.module_size, args.memory_mib, args.timeout) <= 0 or not 0 <= args.fields <= 200:
        parser.error("positive limits and 0..200 fields are required")
    if args.worker:
        worker(args)
        return
    config = dict(classes=args.classes, fields=args.fields, module_size=args.module_size,
                  compiler_sha256=hashlib.sha256(args.cli.read_bytes()).hexdigest())
    build_path = args.directory / "build.json"
    if args.reuse_build:
        built = json.loads(build_path.read_text())
        if built["configuration"] != config:
            parser.error("existing build does not match the requested population or compiler")
    else:
        built = dict(configuration=config, provenance=provenance(REPO), timings=build(args))
        build_path.write_text(json.dumps(built, indent=2) + "\n")
    if args.build_only:
        print(json.dumps(built, indent=2))
        return
    report = dict(provenance=provenance(REPO), requested_classes=args.classes, fields=args.fields, build=built, runs=[])
    for bytecode in (True, False):
        for kind in ("generated", "pydantic_models"):
            command = [sys.executable, __file__, "--worker", "--directory", str(args.directory), "--classes", str(args.classes), "--fields", str(args.fields), "--module-size", str(args.module_size), "--memory-mib", str(args.memory_mib), "--kind", kind]
            with tempfile.TemporaryDirectory(prefix="jsoncompat-import-cache-") as cache:
                environment = dict(os.environ)
                if not bytecode:
                    environment["PYTHONPYCACHEPREFIX"] = cache
                result = subprocess.run(command, check=True, capture_output=True, text=True, env=environment, timeout=args.timeout)
            report["runs"].append(dict(bytecode=bytecode, **json.loads(result.stdout)))
    output = args.directory / "report.json"
    output.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
