from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaItem(dc.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "additionalProperties": false,
  "properties": {
    "host": {
      "type": "string"
    },
    "port": {
      "maximum": 65535,
      "minimum": 0,
      "type": "integer"
    }
  },
  "required": [
    "host",
    "port"
  ],
  "type": "object"
}"""
    host: str = dc.field("host")
    port: int = dc.field("port")

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "servers": {
      "items": {
        "additionalProperties": false,
        "properties": {
          "host": {
            "type": "string"
          },
          "port": {
            "maximum": 65535,
            "minimum": 0,
            "type": "integer"
          }
        },
        "required": [
          "host",
          "port"
        ],
        "type": "object"
      },
      "type": "array"
    }
  },
  "required": [
    "servers"
  ],
  "type": "object"
}"""
    servers: list[GeneratedSchemaItem] = dc.field("servers")

JSONCOMPAT_MODEL = GeneratedSchema
