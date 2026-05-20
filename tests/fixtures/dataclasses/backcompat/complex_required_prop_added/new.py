from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaConfig(dc.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "additionalProperties": false,
  "properties": {
    "enable": {
      "type": "boolean"
    }
  },
  "required": [
    "enable"
  ],
  "type": "object"
}"""
    enable: (typing.Literal[False] | typing.Literal[True]) = dc.field("enable")

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "config": {
      "additionalProperties": false,
      "properties": {
        "enable": {
          "type": "boolean"
        }
      },
      "required": [
        "enable"
      ],
      "type": "object"
    },
    "description": {
      "type": "string"
    },
    "id": {
      "type": "string"
    },
    "name": {
      "type": "string"
    }
  },
  "required": [
    "id",
    "name",
    "description"
  ],
  "type": "object"
}"""
    config: dc.Omittable[GeneratedSchemaConfig] = dc.field("config", omittable=True)
    description: str = dc.field("description")
    id: str = dc.field("id")
    name: str = dc.field("name")

JSONCOMPAT_MODEL = GeneratedSchema
