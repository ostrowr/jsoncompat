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
    count: dc.Omittable[int] = dc.jsoncompat_field("count", omittable=True)
    flag: (typing.Literal[False] | typing.Literal[True]) = dc.jsoncompat_field("flag")

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
    description: dc.Omittable[str] = dc.jsoncompat_field("description", omittable=True)
    id: str = dc.jsoncompat_field("id")
    name: str = dc.jsoncompat_field("name")
    nested: dc.Omittable[GeneratedSchemaNested] = dc.jsoncompat_field("nested", omittable=True)
    tags: dc.Omittable[list[str]] = dc.jsoncompat_field("tags", omittable=True)

GeneratedSchemaNested.__jsoncompat_object_spec__ = dc.jsoncompat_object_spec(
    dc.jsoncompat_field_spec("count", "count", (int | dc.JsoncompatMissingType), omittable=True),
    dc.jsoncompat_field_spec("flag", "flag", (typing.Literal[False] | typing.Literal[True])),
)

GeneratedSchema.__jsoncompat_object_spec__ = dc.jsoncompat_object_spec(
    dc.jsoncompat_field_spec("description", "description", (str | dc.JsoncompatMissingType), omittable=True),
    dc.jsoncompat_field_spec("id", "id", str),
    dc.jsoncompat_field_spec("name", "name", str),
    dc.jsoncompat_field_spec("nested", "nested", (GeneratedSchemaNested | dc.JsoncompatMissingType), omittable=True),
    dc.jsoncompat_field_spec("tags", "tags", (list[str] | dc.JsoncompatMissingType), omittable=True),
)

JSONCOMPAT_MODEL = GeneratedSchema
