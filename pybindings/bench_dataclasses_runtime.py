from __future__ import annotations

import argparse
import json
import time
from dataclasses import dataclass
from typing import Any, Callable, ClassVar, Literal

from jsoncompat import validator_for
from jsoncompat.codegen import dataclasses as dc


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


def clear_runtime_caches() -> None:
    getattr(dc._jsoncompat_validator_for, "cache_clear")()
    getattr(dc._jsoncompat_type_hints_for, "cache_clear")()
    getattr(dc._jsoncompat_object_spec_for, "cache_clear")()
    getattr(dc._jsoncompat_root_annotation_for, "cache_clear")()


def infer_all_specs() -> None:
    dc._jsoncompat_object_spec_for(BenchCustomer)
    dc._jsoncompat_object_spec_for(BenchItem)
    dc._jsoncompat_object_spec_for(BenchEvent)


def cold_spec_inference() -> None:
    clear_runtime_caches()
    infer_all_specs()


def cached_spec_lookup() -> None:
    infer_all_specs()


def from_json() -> BenchEvent:
    return BenchEvent.from_json(PAYLOAD)


def to_json(instance: BenchEvent) -> object:
    return instance.to_json()


def bench(name: str, iterations: int, callback: Callable[[], Any]) -> None:
    start = time.perf_counter()
    for _ in range(iterations):
        callback()
    elapsed = time.perf_counter() - start
    print(
        f"{name:24} iterations={iterations:>8} "
        f"total={elapsed:.6f}s per_iter={elapsed / iterations * 1_000_000:.2f}us"
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--iterations", type=int, default=10_000)
    args = parser.parse_args()

    clear_runtime_caches()
    infer_all_specs()
    instance = from_json()
    validator = validator_for(BenchEvent.__jsoncompat_schema__)
    assert instance.to_json() == PAYLOAD
    assert validator.is_valid(PAYLOAD_JSON)

    bench("cold spec inference", args.iterations, cold_spec_inference)
    clear_runtime_caches()
    infer_all_specs()
    from_json()
    bench("cached spec lookup", args.iterations, cached_spec_lookup)
    bench("validator.is_valid", args.iterations, lambda: validator.is_valid(PAYLOAD_JSON))
    bench("from_json", args.iterations, from_json)
    bench("to_json", args.iterations, lambda: to_json(instance))


if __name__ == "__main__":
    main()
