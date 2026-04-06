from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaConfig(jsoncompat_dataclasses.DataclassModel):
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
    enable: (typing.Literal[False] | typing.Literal[True]) = jsoncompat_dataclasses.jsoncompat_field("enable")

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassModel):
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
    config: jsoncompat_dataclasses.Omittable[GeneratedSchemaConfig] = jsoncompat_dataclasses.jsoncompat_field("config", omittable=True)
    description: str = jsoncompat_dataclasses.jsoncompat_field("description")
    id: str = jsoncompat_dataclasses.jsoncompat_field("id")
    name: str = jsoncompat_dataclasses.jsoncompat_field("name")

GeneratedSchemaConfig.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("enable", "enable", (typing.Literal[False] | typing.Literal[True])),
)

GeneratedSchema.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("config", "config", (GeneratedSchemaConfig | jsoncompat_dataclasses.JsoncompatMissingType), omittable=True),
    jsoncompat_dataclasses.jsoncompat_field_spec("description", "description", str),
    jsoncompat_dataclasses.jsoncompat_field_spec("id", "id", str),
    jsoncompat_dataclasses.jsoncompat_field_spec("name", "name", str),
)

JSONCOMPAT_MODEL = GeneratedSchema
