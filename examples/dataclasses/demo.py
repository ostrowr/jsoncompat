# /// script
# requires-python = ">=3.12"
# dependencies = ["jsoncompat[msgpack,yaml]"]
# [tool.uv.sources]
# jsoncompat = { path = "../../pybindings", editable = true }
# ///
"""Canonical generated-dataclass example for an ordinary JSON Schema."""

from jsoncompat import JsonValue
from jsoncompat.codegen import SerializationFormat
from jsoncompat.codegen.dataclasses import JSONCOMPAT_MISSING

from models import Customer, Order, OrderItem


def describe(order: Order) -> str:
    units = sum(item.quantity for item in order.items)
    return f"{order.id}: {units} units for {order.customer.name}"


def main() -> None:
    # The generated model and every nested model validate on construction.
    order = Order(
        id="order-123",
        customer=Customer(name="Ada", email="ada@example.com"),
        items=[
            OrderItem(sku="keyboard", quantity=1, unitPriceCents=12500),
            OrderItem(sku="switch", quantity=2, unitPriceCents=75),
        ],
        status="paid",
    )
    assert order.note is JSONCOMPAT_MISSING

    # Use to_value()/from_value() at Python JSON-value boundaries.
    value: JsonValue = order.to_value()
    assert value == {
        "id": "order-123",
        "customer": {"name": "Ada", "email": "ada@example.com"},
        "items": [
            {"sku": "keyboard", "quantity": 1, "unitPriceCents": 12500},
            {"sku": "switch", "quantity": 2, "unitPriceCents": 75},
        ],
        "status": "paid",
    }
    assert describe(Order.from_value(value)) == "order-123: 3 units for Ada"
    print("Python value:", describe(order))

    # One ordinary generated model supports both serialization directions.
    json_wire: str = order.serialize()
    yaml_wire: str = order.serialize(format=SerializationFormat.YAML)
    msgpack_wire: bytes = order.serialize(format=SerializationFormat.MSGPACK)

    json_order = Order.deserialize(json_wire)
    yaml_order = Order.deserialize(yaml_wire, format=SerializationFormat.YAML)
    msgpack_order = Order.deserialize(
        msgpack_wire,
        format=SerializationFormat.MSGPACK,
    )
    print("JSON:", describe(json_order))
    print("YAML:", describe(yaml_order))
    print("MessagePack:", describe(msgpack_order))

    # Omitted and explicit null remain distinct.
    assert json_order.note is JSONCOMPAT_MISSING
    null_value: JsonValue = {
        "id": "order-124",
        "customer": {"name": "Grace", "email": "grace@example.com"},
        "items": [
            {"sku": "compiler", "quantity": 1, "unitPriceCents": 0},
        ],
        "status": "pending",
        "note": None,
    }
    assert Order.from_value(null_value).note is None
    print("Omitted and null notes remain distinct")

    # Trusted callers can explicitly skip only the JSON Schema check.
    trusted = Order.from_value(value, skip_validation=True)
    assert trusted.to_value(skip_validation=True) == value
    print("Trusted path matches checked path")

    # Checked boundaries reject schema-invalid values.
    invalid_value: JsonValue = {
        "id": "order-125",
        "customer": {"name": "Ada", "email": "ada@example.com"},
        "items": [
            {"sku": "keyboard", "quantity": 0, "unitPriceCents": 12500},
        ],
        "status": "paid",
    }
    try:
        Order.from_value(invalid_value)
    except ValueError:
        print("Invalid input rejected")
    else:
        raise AssertionError("checked model accepted an invalid order")


if __name__ == "__main__":
    main()
