"""Measure fresh generation and source/bytecode imports for a pinned local spec.

Run against each checkout with its own release generator and native extension.
This benchmark performs no API calls and records hashes of every measured input.
"""
from __future__ import annotations

import argparse
import compileall
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import statistics
import subprocess
import sys
import time


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--spec", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--repeats", type=int, default=5)
    args = parser.parse_args()
    if args.repeats < 1:
        parser.error("--repeats must be positive")
    repo, spec, output = args.repo.resolve(), args.spec.resolve(), args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    generator = repo / "target/release/jsoncompat"
    env = dict(os.environ, PYTHONPATH=str(repo / "pybindings"), JSONCOMPAT_NATIVE_PROFILE="release")
    env.pop("CARGO_TARGET_DIR", None)
    generation = []
    for repeat in range(args.repeats):
        destination = output / str(repeat) / "generated"
        start = time.perf_counter()
        result = subprocess.run([str(generator), "codegen", "--openapi", str(spec), "--api", "Bench", "--output", str(destination)], env=env, capture_output=True, text=True, timeout=300)
        generation.append(time.perf_counter() - start)
        (output / f"generation-{repeat}.log").write_text(result.stderr)
        result.check_returncode()
    package = output / "0/generated"
    files = sorted(path for path in package.rglob("*") if path.suffix in {".py", ".pyi"})
    sizes = {str(path.relative_to(package)): path.stat().st_size for path in files}
    worker = '''import importlib,json,resource,sys,time
sys.path.insert(0, sys.argv[1])
t=time.perf_counter()
module=importlib.import_module('generated')
api=module.BenchAPI
elapsed=time.perf_counter()-t
first=None
if 'RRuleSchedule' in module.__all__:
    model=module.RRuleSchedule
    t=time.perf_counter()
    value=model.deserialize('{"rrule":"FREQ=DAILY","timezone":"America/New_York"}')
    assert value.timezone == 'America/New_York'
    first=time.perf_counter()-t
    for zone in ['America/New_York','Europe/London','UTC']:
        assert model(rrule='FREQ=DAILY',timezone=zone).timezone == zone
    try: model(rrule='FREQ=DAILY',timezone='Not_A_Zone')
    except ValueError: pass
    else: raise AssertionError('invalid timezone accepted')
import jsoncompat
print(json.dumps(dict(import_seconds=elapsed,first_validation_seconds=first,peak_rss_bytes=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss*(1 if sys.platform=='darwin' else 1024),native=jsoncompat._native_symbols.__file__)))
'''
    imports: dict[str, list[dict]] = {}
    for phase in ("source", "bytecode"):
        for cache in package.rglob("__pycache__"):
            shutil.rmtree(cache)
        if phase == "bytecode":
            assert compileall.compile_dir(package, quiet=2)
        imports[phase] = []
        for repeat in range(args.repeats):
            result = subprocess.run([sys.executable, "-B", "-c", worker, str(package.parent)], env=env, capture_output=True, text=True, timeout=120)
            (output / f"import-{phase}-{repeat}.log").write_text(result.stderr)
            result.check_returncode()
            imports[phase].append(json.loads(result.stdout))
    native = Path(imports["source"][0]["native"])
    report = {
        "platform": platform.platform(), "python": sys.version,
        "revision": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repo, text=True).strip(),
        "dirty": bool(subprocess.check_output(["git", "status", "--porcelain"], cwd=repo, text=True).strip()),
        "spec_sha256": sha256(spec), "generator_sha256": sha256(generator), "native_sha256": sha256(native),
        "repeats": args.repeats, "generation_seconds": generation,
        "generation_median_seconds": statistics.median(generation),
        "python_files": len(files), "python_bytes": sum(sizes.values()),
        "largest_files": sorted(sizes.items(), key=lambda item: item[1], reverse=True)[:5],
        "shared_patterns": sum(path.read_text().count("dc.load_pattern(") for path in files),
        "imports": imports,
        "import_median_seconds": {phase: statistics.median(sample["import_seconds"] for sample in samples) for phase, samples in imports.items()},
        "peak_rss_median_bytes": {phase: statistics.median(sample["peak_rss_bytes"] for sample in samples) for phase, samples in imports.items()},
    }
    (output / "results.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({key:value for key,value in report.items() if key not in {"imports", "generation_seconds"}}, indent=2))


if __name__ == "__main__":
    main()
