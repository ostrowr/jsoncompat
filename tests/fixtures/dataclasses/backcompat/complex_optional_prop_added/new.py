from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaNested(dc.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "additionalProperties": false,
  "properties": {
    "count": {
      "minimum": 0,
      "type": "integer"
    },
    "flag": {
      "type": "boolean"
    }
  },
  "required": [
    "flag"
  ],
  "type": "object"
}"""
    count: dc.Omittable[int] = dc.field("count", omittable=True)
    flag: (typing.Literal[False] | typing.Literal[True]) = dc.field("flag")

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "description": {
      "type": "string"
    },
    "id": {
      "type": "string"
    },
    "name": {
      "type": "string"
    },
    "nested": {
      "additionalProperties": false,
      "properties": {
        "count": {
          "minimum": 0,
          "type": "integer"
        },
        "flag": {
          "type": "boolean"
        }
      },
      "required": [
        "flag"
      ],
      "type": "object"
    },
    "tags": {
      "items": {
        "type": "string"
      },
      "type": "array"
    }
  },
  "required": [
    "id",
    "name"
  ],
  "type": "object"
}"""
    description: dc.Omittable[str] = dc.field("description", omittable=True)
    id: str = dc.field("id")
    name: str = dc.field("name")
    nested: dc.Omittable[GeneratedSchemaNested] = dc.field("nested", omittable=True)
    tags: dc.Omittable[list[str]] = dc.field("tags", omittable=True)

JSONCOMPAT_MODEL = GeneratedSchema
