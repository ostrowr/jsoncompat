from __future__ import annotations

import collections.abc
from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaCustomer(dc.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "customer": {
      "additionalProperties": false,
      "properties": {
        "email": {
          "type": "string"
        },
        "id": {
          "type": "string"
        },
        "segment": {
          "enum": [
            "self_serve",
            "startup",
            "enterprise"
          ]
        },
        "trialDaysRemaining": {
          "type": "integer"
        }
      },
      "required": [
        "id",
        "email",
        "segment"
      ],
      "type": "object"
    },
    "item": {
      "additionalProperties": false,
      "properties": {
        "quantity": {
          "type": "integer"
        },
        "sku": {
          "enum": [
            "starter-seat",
            "team-seat",
            "audit-log"
          ]
        },
        "unitPrice": {
          "type": "integer"
        }
      },
      "required": [
        "sku",
        "quantity",
        "unitPrice"
      ],
      "type": "object"
    }
  },
  "additionalProperties": false,
  "properties": {
    "email": {
      "type": "string"
    },
    "id": {
      "type": "string"
    },
    "segment": {
      "enum": [
        "self_serve",
        "startup",
        "enterprise"
      ]
    },
    "trialDaysRemaining": {
      "type": "integer"
    }
  },
  "required": [
    "id",
    "email",
    "segment"
  ],
  "type": "object"
}"""
    email: str = dc.field("email")
    id: str = dc.field("id")
    segment: (typing.Literal["enterprise"] | typing.Literal["self_serve"] | typing.Literal["startup"]) = dc.field("segment")
    trialDaysRemaining: dc.Omittable[int] = dc.field("trialDaysRemaining", omittable=True)

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaItem(dc.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "customer": {
      "additionalProperties": false,
      "properties": {
        "email": {
          "type": "string"
        },
        "id": {
          "type": "string"
        },
        "segment": {
          "enum": [
            "self_serve",
            "startup",
            "enterprise"
          ]
        },
        "trialDaysRemaining": {
          "type": "integer"
        }
      },
      "required": [
        "id",
        "email",
        "segment"
      ],
      "type": "object"
    },
    "item": {
      "additionalProperties": false,
      "properties": {
        "quantity": {
          "type": "integer"
        },
        "sku": {
          "enum": [
            "starter-seat",
            "team-seat",
            "audit-log"
          ]
        },
        "unitPrice": {
          "type": "integer"
        }
      },
      "required": [
        "sku",
        "quantity",
        "unitPrice"
      ],
      "type": "object"
    }
  },
  "additionalProperties": false,
  "properties": {
    "quantity": {
      "type": "integer"
    },
    "sku": {
      "enum": [
        "starter-seat",
        "team-seat",
        "audit-log"
      ]
    },
    "unitPrice": {
      "type": "integer"
    }
  },
  "required": [
    "sku",
    "quantity",
    "unitPrice"
  ],
  "type": "object"
}"""
    quantity: int = dc.field("quantity")
    sku: (typing.Literal["audit-log"] | typing.Literal["starter-seat"] | typing.Literal["team-seat"]) = dc.field("sku")
    unitPrice: int = dc.field("unitPrice")

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassAdditionalModel[str]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "customer": {
      "additionalProperties": false,
      "properties": {
        "email": {
          "type": "string"
        },
        "id": {
          "type": "string"
        },
        "segment": {
          "enum": [
            "self_serve",
            "startup",
            "enterprise"
          ]
        },
        "trialDaysRemaining": {
          "type": "integer"
        }
      },
      "required": [
        "id",
        "email",
        "segment"
      ],
      "type": "object"
    },
    "item": {
      "additionalProperties": false,
      "properties": {
        "quantity": {
          "type": "integer"
        },
        "sku": {
          "enum": [
            "starter-seat",
            "team-seat",
            "audit-log"
          ]
        },
        "unitPrice": {
          "type": "integer"
        }
      },
      "required": [
        "sku",
        "quantity",
        "unitPrice"
      ],
      "type": "object"
    }
  },
  "additionalProperties": {
    "type": "string"
  },
  "properties": {
    "couponCode": {
      "type": [
        "string",
        "null"
      ]
    },
    "currency": {
      "enum": [
        "USD",
        "EUR",
        "GBP"
      ]
    },
    "customer": {
      "$ref": "#/$defs/customer"
    },
    "event": {
      "enum": [
        "checkout.completed",
        "checkout.failed"
      ]
    },
    "items": {
      "items": {
        "$ref": "#/$defs/item"
      },
      "type": "array"
    }
  },
  "required": [
    "event",
    "customer",
    "items",
    "currency"
  ],
  "type": "object"
}"""
    couponCode: dc.Omittable[str | None] = dc.field("couponCode", omittable=True)
    currency: (typing.Literal["EUR"] | typing.Literal["GBP"] | typing.Literal["USD"]) = dc.field("currency")
    customer: GeneratedSchemaCustomer = dc.field("customer")
    event: (typing.Literal["checkout.completed"] | typing.Literal["checkout.failed"]) = dc.field("event")
    items: collections.abc.Sequence[GeneratedSchemaItem] = dc.field("items")
    __jsoncompat_extra__: collections.abc.Mapping[str, str] = dc.extra_field()

JSONCOMPAT_MODEL = GeneratedSchema

dc.bind_generated_models((
    (
        GeneratedSchemaCustomer,
        "object",
        (
            ("email", "email", str, False),
            ("id", "id", str, False),
            ("segment", "segment", (typing.Literal["enterprise"] | typing.Literal["self_serve"] | typing.Literal["startup"]), False),
            ("trialDaysRemaining", "trialDaysRemaining", int, True),
        ),
        False,
        None,
    ),
    (
        GeneratedSchemaItem,
        "object",
        (
            ("quantity", "quantity", int, False),
            ("sku", "sku", (typing.Literal["audit-log"] | typing.Literal["starter-seat"] | typing.Literal["team-seat"]), False),
            ("unitPrice", "unitPrice", int, False),
        ),
        False,
        None,
    ),
    (
        GeneratedSchema,
        "object",
        (
            ("couponCode", "couponCode", (str | None), True),
            ("currency", "currency", (typing.Literal["EUR"] | typing.Literal["GBP"] | typing.Literal["USD"]), False),
            ("customer", "customer", GeneratedSchemaCustomer, False),
            ("event", "event", (typing.Literal["checkout.completed"] | typing.Literal["checkout.failed"]), False),
            ("items", "items", collections.abc.Sequence[GeneratedSchemaItem], False),
        ),
        True,
        str,
    ),
))
