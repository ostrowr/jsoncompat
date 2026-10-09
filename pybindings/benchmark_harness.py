"""Shared timing and environment reporting for all generated-model benchmarks."""
from __future__ import annotations
import gc
import os
import platform
import statistics
import subprocess
import time
from pathlib import Path
from typing import Callable, TypedDict


class Timing(TypedDict):
    median_us: float
    samples_us: list[float]


def measure(callbacks: dict[str, Callable[[], object]], iterations: int, repeats: int,
            *, warmup: int = 50) -> dict[str, Timing]:
    if not callbacks or min(iterations, repeats) <= 0:
        raise ValueError("callbacks and positive iteration/repeat counts are required")
    for callback in callbacks.values():
        for _ in range(min(iterations, warmup)):
            callback()
    names = list(callbacks)
    samples: dict[str, list[float]] = {name: [] for name in names}
    enabled = gc.isenabled()
    gc.disable()
    try:
        for repeat in range(repeats):
            offset = repeat % len(names)
            for name in names[offset:] + names[:offset]:
                start = time.perf_counter_ns()
                callback = callbacks[name]
                for _ in range(iterations):
                    callback()
                samples[name].append((time.perf_counter_ns() - start) / iterations / 1000)
    finally:
        if enabled:
            gc.enable()
    return {name: {"median_us": statistics.median(values), "samples_us": values}
            for name, values in samples.items()}


def provenance(repo: Path) -> dict[str, object]:
    import pydantic
    def git(*args: str) -> str:
        return subprocess.check_output(["git", *args], cwd=repo, text=True).strip()
    return dict(native_profile=os.environ.get("JSONCOMPAT_NATIVE_PROFILE", "debug"), python=platform.python_version(), pydantic=pydantic.__version__,
                platform=platform.platform(), revision=git("rev-parse", "HEAD"),
                dirty=bool(git("status", "--porcelain", "--untracked-files=no")),
                timer="perf_counter_ns", gc_during_round_trip=False,
                startup="fresh interpreter; filesystem cache state unspecified")
