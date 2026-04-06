from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaNested(jsoncompat_dataclasses.DataclassModel):
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
    count: (int | jsoncompat_dataclasses.JsoncompatMissingType) = jsoncompat_dataclasses.jsoncompat_field("count", omittable=True)
    flag: (typing.Literal[False] | typing.Literal[True]) = jsoncompat_dataclasses.jsoncompat_field("flag")

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassModel):
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
    description: (str | jsoncompat_dataclasses.JsoncompatMissingType) = jsoncompat_dataclasses.jsoncompat_field("description", omittable=True)
    id: str = jsoncompat_dataclasses.jsoncompat_field("id")
    name: str = jsoncompat_dataclasses.jsoncompat_field("name")
    nested: (GeneratedSchemaNested | jsoncompat_dataclasses.JsoncompatMissingType) = jsoncompat_dataclasses.jsoncompat_field("nested", omittable=True)
    tags: (list[str] | jsoncompat_dataclasses.JsoncompatMissingType) = jsoncompat_dataclasses.jsoncompat_field("tags", omittable=True)

GeneratedSchemaNested.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("count", "count", (int | jsoncompat_dataclasses.JsoncompatMissingType), omittable=True),
    jsoncompat_dataclasses.jsoncompat_field_spec("flag", "flag", (typing.Literal[False] | typing.Literal[True])),
)

GeneratedSchema.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("description", "description", (str | jsoncompat_dataclasses.JsoncompatMissingType), omittable=True),
    jsoncompat_dataclasses.jsoncompat_field_spec("id", "id", str),
    jsoncompat_dataclasses.jsoncompat_field_spec("name", "name", str),
    jsoncompat_dataclasses.jsoncompat_field_spec("nested", "nested", (GeneratedSchemaNested | jsoncompat_dataclasses.JsoncompatMissingType), omittable=True),
    jsoncompat_dataclasses.jsoncompat_field_spec("tags", "tags", (list[str] | jsoncompat_dataclasses.JsoncompatMissingType), omittable=True),
)

JSONCOMPAT_MODEL = GeneratedSchema
