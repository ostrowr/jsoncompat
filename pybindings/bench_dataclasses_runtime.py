"""Benchmark generated dataclasses and an equivalent strict Pydantic v2 graph.

Pydantic dump methods do not re-run model validation, so compare them primarily
with jsoncompat's trusted serialization paths. All comparisons use the same
already-valid nested payload; coercion and error-reporting semantics differ.
"""

from __future__ import annotations

import argparse
import gc
import json
import platform
import statistics
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable, ClassVar, Literal

import pydantic
from pydantic import BaseModel, ConfigDict, Field

from jsoncompat import validator_for
from jsoncompat.codegen import SerializationFormat
from jsoncompat.codegen import dataclasses as dc


STAMP_EXAMPLE_DIR = Path(__file__).resolve().parents[1] / "examples" / "stamp"
sys.path.insert(0, str(STAMP_EXAMPLE_DIR))
from reader_models import UserProfileReader  # noqa: E402


@dataclass(frozen=True, slots=True, kw_only=True)
class BenchCustomer(dc.DataclassModel):
    __jsoncompat_schema__: ClassVar[str] = """
{
  "type": "object",
  "required": ["id", "email", "segment"],
  "properties": {
    "id": { "type": "string" },
    "email": { "type": "string" },
    "segment": { "enum": ["self_serve", "startup", "enterprise"] },
    "trialDaysRemaining": { "type": "integer" }
  },
  "additionalProperties": false
}
"""

    email: str = dc.field("email")
    id: str = dc.field("id")
    segment: Literal["enterprise", "self_serve", "startup"] = dc.field("segment")
    trialDaysRemaining: dc.Omittable[int] = dc.field(
        "trialDaysRemaining", omittable=True
    )


@dataclass(frozen=True, slots=True, kw_only=True)
class BenchItem(dc.DataclassModel):
    __jsoncompat_schema__: ClassVar[str] = """
{
  "type": "object",
  "required": ["sku", "quantity", "unitPrice"],
  "properties": {
    "sku": { "enum": ["starter-seat", "team-seat", "audit-log"] },
    "quantity": { "type": "integer" },
    "unitPrice": { "type": "integer" }
  },
  "additionalProperties": false
}
"""

    quantity: int = dc.field("quantity")
    sku: Literal["audit-log", "starter-seat", "team-seat"] = dc.field("sku")
    unitPrice: int = dc.field("unitPrice")


@dataclass(frozen=True, slots=True, kw_only=True)
class BenchEvent(dc.DataclassAdditionalModel[str]):
    __jsoncompat_schema__: ClassVar[str] = """
{
  "type": "object",
  "required": ["event", "customer", "items", "currency"],
  "properties": {
    "event": { "enum": ["checkout.completed", "checkout.failed"] },
    "customer": { "$ref": "#/$defs/customer" },
    "items": {
      "type": "array",
      "items": { "$ref": "#/$defs/item" }
    },
    "currency": { "enum": ["USD", "EUR", "GBP"] },
    "couponCode": { "type": ["string", "null"] }
  },
  "additionalProperties": { "type": "string" },
  "$defs": {
    "customer": {
      "type": "object",
      "required": ["id", "email", "segment"],
      "properties": {
        "id": { "type": "string" },
        "email": { "type": "string" },
        "segment": { "enum": ["self_serve", "startup", "enterprise"] },
        "trialDaysRemaining": { "type": "integer" }
      },
      "additionalProperties": false
    },
    "item": {
      "type": "object",
      "required": ["sku", "quantity", "unitPrice"],
      "properties": {
        "sku": { "enum": ["starter-seat", "team-seat", "audit-log"] },
        "quantity": { "type": "integer" },
        "unitPrice": { "type": "integer" }
      },
      "additionalProperties": false
    }
  }
}
"""

    couponCode: dc.Omittable[str | None] = dc.field("couponCode", omittable=True)
    currency: Literal["EUR", "GBP", "USD"] = dc.field("currency")
    customer: BenchCustomer = dc.field("customer")
    event: Literal["checkout.completed", "checkout.failed"] = dc.field("event")
    items: list[BenchItem] = dc.field("items")
    __jsoncompat_extra__: dict[str, str] = dc.extra_field()


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


PAYLOAD = {
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
STAMPED_VALUE = {
    "version": 2,
    "data": {
        "name": "Ada",
        "age": 37,
        "interests": 3,
    },
}
STAMPED_JSON = json.dumps(STAMPED_VALUE, separators=(",", ":"), sort_keys=True)


def clear_runtime_caches() -> None:
    getattr(dc._jsoncompat_validator_for, "cache_clear")()
    getattr(dc._jsoncompat_type_hints_for, "cache_clear")()
    getattr(dc._jsoncompat_object_spec_for, "cache_clear")()
    getattr(dc._jsoncompat_root_annotation_for, "cache_clear")()
    getattr(dc._jsoncompat_discriminator_plans_for, "cache_clear")()
    getattr(dc._jsoncompat_dataclass_fields_for_type, "cache_clear")()


def infer_all_specs() -> None:
    dc._jsoncompat_object_spec_for(BenchCustomer)
    dc._jsoncompat_object_spec_for(BenchItem)
    dc._jsoncompat_object_spec_for(BenchEvent)


def cold_spec_inference() -> None:
    clear_runtime_caches()
    infer_all_specs()


def cached_spec_lookup() -> None:
    infer_all_specs()


def cold_validator_compile() -> None:
    getattr(dc._jsoncompat_validator_for, "cache_clear")()
    dc._jsoncompat_validator_for(BenchEvent)


def cached_validator_lookup() -> None:
    dc._jsoncompat_validator_for(BenchEvent)


def cold_first_from_value() -> BenchEvent:
    clear_runtime_caches()
    return BenchEvent.from_value(PAYLOAD)


def from_value() -> BenchEvent:
    return BenchEvent.from_value(PAYLOAD)


def from_value_trusted() -> BenchEvent:
    return BenchEvent.from_value(PAYLOAD, skip_validation=True)


def to_value(instance: BenchEvent) -> object:
    return instance.to_value()


def to_value_trusted(instance: BenchEvent) -> object:
    return instance.to_value(skip_validation=True)


def pydantic_from_value() -> PydanticEvent:
    return PydanticEvent.model_validate(PAYLOAD)


def pydantic_from_json() -> PydanticEvent:
    return PydanticEvent.model_validate_json(PAYLOAD_JSON)


def pydantic_to_value(instance: PydanticEvent) -> dict[str, Any]:
    return instance.model_dump(mode="json", exclude_unset=True)


def pydantic_to_json(instance: PydanticEvent) -> str:
    return instance.model_dump_json(exclude_unset=True)


def stdlib_json_loads() -> Any:
    return json.loads(PAYLOAD_JSON)


def stdlib_json_dumps() -> str:
    return json.dumps(PAYLOAD, separators=(",", ":"), sort_keys=True)


def stamped_from_value() -> UserProfileReader:
    return UserProfileReader.from_value(STAMPED_VALUE)


def stamped_from_value_trusted() -> UserProfileReader:
    return UserProfileReader.from_value(STAMPED_VALUE, skip_validation=True)


def stamped_deserialize() -> UserProfileReader:
    return UserProfileReader.deserialize(STAMPED_JSON)


def stamped_deserialize_trusted() -> UserProfileReader:
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
        f"{name:28} iterations={iterations:>8} "
        f"median={median:.2f}us best={best:.2f}us"
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

    clear_runtime_caches()
    infer_all_specs()
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

    run("cold spec inference", cold_spec_inference)
    clear_runtime_caches()
    infer_all_specs()
    from_value()
    run("cached spec lookup", cached_spec_lookup)
    run("cold validator compile", cold_validator_compile)
    run("cached validator lookup", cached_validator_lookup)
    run("cold first from_value", cold_first_from_value)
    clear_runtime_caches()
    infer_all_specs()
    from_value()
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
