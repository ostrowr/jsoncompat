"""Benchmark codegen-emitted dataclasses and an equivalent Pydantic v2 graph.

Pydantic dump methods do not re-run model validation, so compare them primarily
with jsoncompat's trusted serialization paths. All comparisons use the same
already-valid nested payload; coercion and error-reporting semantics differ. The
jsoncompat models are generated from ``benchmark_schemas/representative.json``
and their checked-in module is guarded by the dataclass snapshot test.
"""

from __future__ import annotations

import argparse
import gc
import json
import platform
import statistics
import time
from pathlib import Path
from typing import Any, Callable, Literal

import pydantic
from pydantic import BaseModel, ConfigDict, Field

from jsoncompat import JsonValue, validator_for
from jsoncompat.codegen import SerializationFormat

from benchmark_generated_models import (
    generated_dataclass,
    load_generated_module,
    load_generated_path,
)


STAMP_EXAMPLE_DIR = Path(__file__).resolve().parents[1] / "examples" / "stamp"
_STAMP_MODELS = load_generated_path(STAMP_EXAMPLE_DIR / "reader_models.py")
UserProfileReader = generated_dataclass(_STAMP_MODELS, "UserProfileReader")

_GENERATED_MODELS = load_generated_module("representative")
BenchCustomer = generated_dataclass(_GENERATED_MODELS, "GeneratedSchemaCustomer")
BenchItem = generated_dataclass(_GENERATED_MODELS, "GeneratedSchemaItem")
BenchEvent = generated_dataclass(_GENERATED_MODELS, "JSONCOMPAT_MODEL")


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

    __pydantic_extra__: dict[str, str] = Field(  # pyright: ignore[reportIncompatibleVariableOverride]
        init=False
    )
    couponCode: str | None = None
    currency: Literal["EUR", "GBP", "USD"]
    customer: PydanticCustomer
    event: Literal["checkout.completed", "checkout.failed"]
    items: list[PydanticItem]


PAYLOAD: JsonValue = {
    "event": "checkout.completed",
    "customer": {
        "id": "cus_123",
        "email": "ada@example.com",
        "segment": "enterprise",
        "trialDaysRemaining": 7,
    },
    "items": [
        {
            "sku": "team-seat",
            "quantity": 2,
            "unitPrice": 120,
        }
    ],
    "currency": "USD",
    "traceId": "trace_123",
}
PAYLOAD_JSON = json.dumps(PAYLOAD, separators=(",", ":"), sort_keys=True)
DIRECT_CUSTOMER = BenchCustomer(
    email="ada@example.com",
    id="cus_123",
    segment="enterprise",
    trialDaysRemaining=7,
)
DIRECT_ITEM = BenchItem(
    quantity=2,
    sku="team-seat",
    unitPrice=120,
)
PYDANTIC_DIRECT_CUSTOMER = PydanticCustomer(
    email="ada@example.com",
    id="cus_123",
    segment="enterprise",
    trialDaysRemaining=7,
)
PYDANTIC_DIRECT_ITEM = PydanticItem(
    quantity=2,
    sku="team-seat",
    unitPrice=120,
)
STAMPED_VALUE: JsonValue = {
    "version": 2,
    "data": {
        "name": "Ada",
        "age": 37,
        "interests": 3,
    },
}
STAMPED_JSON = json.dumps(STAMPED_VALUE, separators=(",", ":"), sort_keys=True)


def from_value() -> Any:
    return BenchEvent.from_value(PAYLOAD)


def from_value_trusted() -> Any:
    return BenchEvent.from_value(PAYLOAD, skip_validation=True)


def to_value(instance: Any) -> object:
    return instance.to_value()


def to_value_trusted(instance: Any) -> object:
    return instance.to_value(skip_validation=True)


def pydantic_from_value() -> PydanticEvent:
    return PydanticEvent.model_validate(PAYLOAD)


def pydantic_from_json() -> PydanticEvent:
    return PydanticEvent.model_validate_json(PAYLOAD_JSON)


def direct_leaf() -> Any:
    return BenchItem(quantity=2, sku="team-seat", unitPrice=120)


def direct_leaf_trusted() -> Any:
    return BenchItem(
        quantity=2,
        sku="team-seat",
        unitPrice=120,
        skip_validation=True,
    )


def pydantic_direct_leaf() -> PydanticItem:
    return PydanticItem(quantity=2, sku="team-seat", unitPrice=120)


def direct_model() -> Any:
    return BenchEvent(
        currency="USD",
        customer=DIRECT_CUSTOMER,
        event="checkout.completed",
        items=[DIRECT_ITEM],
        __jsoncompat_extra__={"traceId": "trace_123"},
    )


def direct_model_trusted() -> Any:
    return BenchEvent(
        currency="USD",
        customer=DIRECT_CUSTOMER,
        event="checkout.completed",
        items=[DIRECT_ITEM],
        __jsoncompat_extra__={"traceId": "trace_123"},
        skip_validation=True,
    )


def pydantic_direct_model() -> PydanticEvent:
    return PydanticEvent(
        currency="USD",
        customer=PYDANTIC_DIRECT_CUSTOMER,
        event="checkout.completed",
        items=[PYDANTIC_DIRECT_ITEM],
        **{"traceId": "trace_123"},  # pyright: ignore[reportCallIssue]
    )


def pydantic_to_value(instance: PydanticEvent) -> dict[str, Any]:
    return instance.model_dump(mode="json", exclude_unset=True)


def pydantic_to_json(instance: PydanticEvent) -> str:
    return instance.model_dump_json(exclude_unset=True)


def stdlib_json_loads() -> Any:
    return json.loads(PAYLOAD_JSON)


def stdlib_json_dumps() -> str:
    return json.dumps(PAYLOAD, separators=(",", ":"), sort_keys=True)


def stamped_from_value() -> Any:
    return UserProfileReader.from_value(STAMPED_VALUE)


def stamped_from_value_trusted() -> Any:
    return UserProfileReader.from_value(STAMPED_VALUE, skip_validation=True)


def stamped_deserialize() -> Any:
    return UserProfileReader.deserialize(STAMPED_JSON)


def stamped_deserialize_trusted() -> Any:
    return UserProfileReader.deserialize(STAMPED_JSON, skip_validation=True)


def bench(
    name: str,
    iterations: int,
    repeats: int,
    callback: Callable[[], Any],
) -> None:
    for _ in range(min(iterations, 100)):
        callback()

    samples: list[float] = []
    gc_was_enabled = gc.isenabled()
    gc.disable()
    try:
        for _ in range(repeats):
            start = time.perf_counter()
            for _ in range(iterations):
                callback()
            samples.append(time.perf_counter() - start)
    finally:
        if gc_was_enabled:
            gc.enable()

    median = statistics.median(samples) / iterations * 1_000_000
    best = min(samples) / iterations * 1_000_000
    print(
        f"{name:28} iterations={iterations:>8} median={median:.2f}us best={best:.2f}us"
    )


def positive_int(raw_value: str) -> int:
    value = int(raw_value)
    if value < 1:
        raise argparse.ArgumentTypeError("value must be at least 1")
    return value


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--iterations", type=positive_int, default=10_000)
    parser.add_argument("--repeats", type=positive_int, default=5)
    args = parser.parse_args()

    def run(name: str, callback: Callable[[], Any]) -> None:
        bench(name, args.iterations, args.repeats, callback)

    instance = from_value()
    pydantic_instance = pydantic_from_value()
    json_wire = instance.serialize()
    yaml_wire = instance.serialize(format=SerializationFormat.YAML)
    msgpack_wire = instance.serialize(format=SerializationFormat.MSGPACK)
    validator = validator_for(BenchEvent.__jsoncompat_schema__)
    assert instance.to_value() == PAYLOAD
    assert validator.is_valid_json(PAYLOAD_JSON)
    assert validator.is_valid_value(PAYLOAD)
    assert pydantic_to_value(pydantic_instance) == PAYLOAD
    assert json.loads(pydantic_to_json(pydantic_instance)) == PAYLOAD

    print(f"Python {platform.python_version()}, Pydantic {pydantic.__version__}")

    run(
        "validator.is_valid_json",
        lambda: validator.is_valid_json(PAYLOAD_JSON),
    )
    run(
        "validator.is_valid_value",
        lambda: validator.is_valid_value(PAYLOAD),
    )
    run("stdlib json.loads", stdlib_json_loads)
    run("stdlib json.dumps", stdlib_json_dumps)
    run("from_value checked", from_value)
    run("pydantic model_validate", pydantic_from_value)
    run("from_value trusted", from_value_trusted)
    run("direct leaf Model(...)", direct_leaf)
    run("direct leaf trusted", direct_leaf_trusted)
    run("pydantic direct leaf", pydantic_direct_leaf)
    run("direct nested Model(...)", direct_model)
    run("direct nested trusted", direct_model_trusted)
    run("pydantic direct nested", pydantic_direct_model)
    run("to_value checked", lambda: to_value(instance))
    run(
        "pydantic model_dump",
        lambda: pydantic_to_value(pydantic_instance),
    )
    run("to_value trusted", lambda: to_value_trusted(instance))
    run("serialize JSON checked", instance.serialize)
    run(
        "serialize JSON trusted",
        lambda: instance.serialize(skip_validation=True),
    )
    run(
        "pydantic model_dump_json",
        lambda: pydantic_to_json(pydantic_instance),
    )
    run(
        "deserialize JSON checked",
        lambda: BenchEvent.deserialize(json_wire),
    )
    run(
        "deserialize JSON trusted",
        lambda: BenchEvent.deserialize(json_wire, skip_validation=True),
    )
    run("pydantic model_validate_json", pydantic_from_json)
    run(
        "serialize YAML checked",
        lambda: instance.serialize(format=SerializationFormat.YAML),
    )
    run(
        "deserialize YAML checked",
        lambda: BenchEvent.deserialize(
            yaml_wire,
            format=SerializationFormat.YAML,
        ),
    )
    run(
        "serialize msgpack checked",
        lambda: instance.serialize(format=SerializationFormat.MSGPACK),
    )
    run(
        "deserialize msgpack checked",
        lambda: BenchEvent.deserialize(
            msgpack_wire,
            format=SerializationFormat.MSGPACK,
        ),
    )
    run("stamped value checked", stamped_from_value)
    run("stamped value trusted", stamped_from_value_trusted)
    run("stamped JSON checked", stamped_deserialize)
    run("stamped JSON trusted", stamped_deserialize_trusted)


if __name__ == "__main__":
    main()
