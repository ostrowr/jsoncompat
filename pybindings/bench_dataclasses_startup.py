"""Benchmark cold startup boundaries for generated dataclasses and Pydantic v2.

Each sample starts a fresh interpreter. The generated jsoncompat module is the
checked-in representative benchmark fixture, while the Pydantic peer has the
same model graph and strict/frozen configuration as the steady-state benchmark.
"""

import argparse
import importlib.metadata
import os
import platform
import statistics
import subprocess
import sys
import time
from pathlib import Path
from typing import Final


REPO_ROOT: Final = Path(__file__).resolve().parents[1]
MODEL_ROOT: Final = REPO_ROOT / "tests" / "fixtures" / "dataclasses" / "benchmarks"

PAYLOAD: Final = {
    "event": "checkout.completed",
    "customer": {
        "id": "cus_123",
        "email": "ada@example.com",
        "segment": "enterprise",
        "trialDaysRemaining": 7,
    },
    "items": [{"sku": "team-seat", "quantity": 2, "unitPrice": 120}],
    "currency": "USD",
    "traceId": "trace_123",
}

PYDANTIC_PEER_SOURCE: Final = """
from typing import Literal
from pydantic import BaseModel, ConfigDict, Field

class PydanticCustomer(BaseModel):
    model_config = ConfigDict(extra="forbid", frozen=True, strict=True)
    email: str
    id: str
    segment: Literal["enterprise", "self_serve", "startup"]
    trialDaysRemaining: int = 0

class PydanticItem(BaseModel):
    model_config = ConfigDict(extra="forbid", frozen=True, strict=True)
    quantity: int
    sku: Literal["audit-log", "starter-seat", "team-seat"]
    unitPrice: int

class PydanticEvent(BaseModel):
    model_config = ConfigDict(extra="allow", frozen=True, strict=True)
    __pydantic_extra__: dict[str, str] = Field(init=False)
    couponCode: str | None = None
    currency: Literal["EUR", "GBP", "USD"]
    customer: PydanticCustomer
    event: Literal["checkout.completed", "checkout.failed"]
    items: list[PydanticItem]
"""

PAYLOAD_SOURCE: Final = repr(PAYLOAD)
SCENARIO_SOURCE: Final = {
    "baseline": "pass",
    "jsoncompat-runtime-import": "import jsoncompat.codegen.dataclasses",
    "jsoncompat-model-import": "import representative",
    "jsoncompat-first-trusted": (
        "import representative\n"
        f"representative.JSONCOMPAT_MODEL.from_value({PAYLOAD_SOURCE}, "
        "skip_validation=True)"
    ),
    "jsoncompat-first-checked": (
        "import representative\n"
        f"representative.JSONCOMPAT_MODEL.from_value({PAYLOAD_SOURCE})"
    ),
    "pydantic-model-import": PYDANTIC_PEER_SOURCE,
    "pydantic-first-checked": (
        f"{PYDANTIC_PEER_SOURCE}\nPydanticEvent.model_validate({PAYLOAD_SOURCE})"
    ),
}
SCENARIOS: Final = tuple(SCENARIO_SOURCE)

LABELS: Final = {
    "baseline": "empty benchmark process",
    "jsoncompat-runtime-import": "jsoncompat runtime import",
    "jsoncompat-model-import": "jsoncompat generated model import",
    "jsoncompat-first-trusted": "jsoncompat first trusted use",
    "jsoncompat-first-checked": "jsoncompat first checked use",
    "pydantic-model-import": "pydantic equivalent model import",
    "pydantic-first-checked": "pydantic first checked use",
}


def positive_int(raw_value: str) -> int:
    value = int(raw_value)
    if value < 1:
        raise argparse.ArgumentTypeError("value must be at least 1")
    return value


def _child_environment() -> dict[str, str]:
    environment = dict(os.environ)
    existing_pythonpath = environment.get("PYTHONPATH")
    model_path = str(MODEL_ROOT)
    environment["PYTHONPATH"] = (
        model_path
        if not existing_pythonpath
        else os.pathsep.join((model_path, existing_pythonpath))
    )
    environment["PYTHONHASHSEED"] = "0"
    return environment


def _sample(scenario: str, environment: dict[str, str]) -> int:
    command = (sys.executable, "-c", SCENARIO_SOURCE[scenario])
    start = time.perf_counter_ns()
    completed = subprocess.run(
        command,
        cwd=REPO_ROOT,
        env=environment,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        check=False,
        text=True,
    )
    elapsed = time.perf_counter_ns() - start
    if completed.returncode != 0:
        raise RuntimeError(
            f"startup worker {scenario!r} failed with exit code "
            f"{completed.returncode}:\n{completed.stderr}"
        )
    return elapsed


def _median_ms(samples: list[int]) -> float:
    return statistics.median(samples) / 1_000_000


def _benchmark(repeats: int) -> None:
    environment = _child_environment()
    for scenario in SCENARIOS:
        _sample(scenario, environment)

    samples: dict[str, list[int]] = {scenario: [] for scenario in SCENARIOS}
    for repeat in range(repeats):
        offset = repeat % len(SCENARIOS)
        ordered = SCENARIOS[offset:] + SCENARIOS[:offset]
        for scenario in ordered:
            samples[scenario].append(_sample(scenario, environment))

    print(
        f"Python {platform.python_version()}, "
        f"Pydantic {importlib.metadata.version('pydantic')}, repeats={repeats}"
    )
    print("Fresh interpreter per sample; delta is paired with the empty worker.")
    for scenario in SCENARIOS:
        median_ms = _median_ms(samples[scenario])
        best_ms = min(samples[scenario]) / 1_000_000
        delta_ms = (
            statistics.median(
                sample - baseline
                for sample, baseline in zip(
                    samples[scenario], samples["baseline"], strict=True
                )
            )
            / 1_000_000
        )
        print(
            f"{LABELS[scenario]:36} "
            f"median={median_ms:8.3f}ms "
            f"delta={delta_ms:+8.3f}ms "
            f"best={best_ms:8.3f}ms"
        )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repeats", type=positive_int, default=25)
    args = parser.parse_args()
    _benchmark(args.repeats)


if __name__ == "__main__":
    main()
