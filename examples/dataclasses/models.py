from __future__ import annotations

import collections.abc
from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class Customer(dc.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "customer": {
      "additionalProperties": false,
      "properties": {
        "email": {
          "minLength": 3,
          "type": "string"
        },
        "name": {
          "minLength": 1,
          "type": "string"
        }
      },
      "required": [
        "name",
        "email"
      ],
      "title": "Customer",
      "type": "object"
    },
    "item": {
      "additionalProperties": false,
      "properties": {
        "quantity": {
          "minimum": 1,
          "type": "integer"
        },
        "sku": {
          "minLength": 1,
          "type": "string"
        },
        "unitPriceCents": {
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "sku",
        "quantity",
        "unitPriceCents"
      ],
      "title": "OrderItem",
      "type": "object"
    }
  },
  "additionalProperties": false,
  "properties": {
    "email": {
      "minLength": 3,
      "type": "string"
    },
    "name": {
      "minLength": 1,
      "type": "string"
    }
  },
  "required": [
    "name",
    "email"
  ],
  "title": "Customer",
  "type": "object"
}"""
    email: str = dc.field("email")
    name: str = dc.field("name")

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class OrderItem(dc.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "customer": {
      "additionalProperties": false,
      "properties": {
        "email": {
          "minLength": 3,
          "type": "string"
        },
        "name": {
          "minLength": 1,
          "type": "string"
        }
      },
      "required": [
        "name",
        "email"
      ],
      "title": "Customer",
      "type": "object"
    },
    "item": {
      "additionalProperties": false,
      "properties": {
        "quantity": {
          "minimum": 1,
          "type": "integer"
        },
        "sku": {
          "minLength": 1,
          "type": "string"
        },
        "unitPriceCents": {
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "sku",
        "quantity",
        "unitPriceCents"
      ],
      "title": "OrderItem",
      "type": "object"
    }
  },
  "additionalProperties": false,
  "properties": {
    "quantity": {
      "minimum": 1,
      "type": "integer"
    },
    "sku": {
      "minLength": 1,
      "type": "string"
    },
    "unitPriceCents": {
      "minimum": 0,
      "type": "integer"
    }
  },
  "required": [
    "sku",
    "quantity",
    "unitPriceCents"
  ],
  "title": "OrderItem",
  "type": "object"
}"""
    quantity: int = dc.field("quantity")
    sku: str = dc.field("sku")
    unitPriceCents: int = dc.field("unitPriceCents")

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class Order(dc.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "customer": {
      "additionalProperties": false,
      "properties": {
        "email": {
          "minLength": 3,
          "type": "string"
        },
        "name": {
          "minLength": 1,
          "type": "string"
        }
      },
      "required": [
        "name",
        "email"
      ],
      "title": "Customer",
      "type": "object"
    },
    "item": {
      "additionalProperties": false,
      "properties": {
        "quantity": {
          "minimum": 1,
          "type": "integer"
        },
        "sku": {
          "minLength": 1,
          "type": "string"
        },
        "unitPriceCents": {
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "sku",
        "quantity",
        "unitPriceCents"
      ],
      "title": "OrderItem",
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "customer": {
      "$ref": "#/$defs/customer"
    },
    "id": {
      "minLength": 1,
      "type": "string"
    },
    "items": {
      "items": {
        "$ref": "#/$defs/item"
      },
      "minItems": 1,
      "type": "array"
    },
    "note": {
      "type": [
        "string",
        "null"
      ]
    },
    "status": {
      "enum": [
        "pending",
        "paid"
      ]
    }
  },
  "required": [
    "id",
    "customer",
    "items",
    "status"
  ],
  "title": "Order",
  "type": "object"
}"""
    customer: Customer = dc.field("customer")
    id: str = dc.field("id")
    items: collections.abc.Sequence[OrderItem] = dc.field("items")
    note: dc.Omittable[str | None] = dc.field("note", omittable=True)
    status: (typing.Literal["paid"] | typing.Literal["pending"]) = dc.field("status")

JSONCOMPAT_MODEL = Order
