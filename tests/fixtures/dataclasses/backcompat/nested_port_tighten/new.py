from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaItem(jsoncompat_dataclasses.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "additionalProperties": false,
  "properties": {
    "host": {
      "type": "string"
    },
    "port": {
      "maximum": 65535,
      "minimum": 1024,
      "type": "integer"
    }
  },
  "required": [
    "host",
    "port"
  ],
  "type": "object"
}"""
    host: str = jsoncompat_dataclasses.jsoncompat_field("host")
    port: int = jsoncompat_dataclasses.jsoncompat_field("port")

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassModel):
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
            "minimum": 1024,
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
    servers: list[GeneratedSchemaItem] = jsoncompat_dataclasses.jsoncompat_field("servers")

GeneratedSchemaItem.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("host", "host", str),
    jsoncompat_dataclasses.jsoncompat_field_spec("port", "port", int),
)

GeneratedSchema.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("servers", "servers", list[GeneratedSchemaItem]),
)

JSONCOMPAT_MODEL = GeneratedSchema
