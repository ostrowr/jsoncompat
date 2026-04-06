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
    enable: (typing.Literal[False] | typing.Literal[True]) = dc.jsoncompat_field("enable")

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
    "id": {
      "type": "string"
    },
    "name": {
      "type": "string"
    }
  },
  "required": [
    "id",
    "name"
  ],
  "type": "object"
}"""
    config: dc.Omittable[GeneratedSchemaConfig] = dc.jsoncompat_field("config", omittable=True)
    id: str = dc.jsoncompat_field("id")
    name: str = dc.jsoncompat_field("name")

GeneratedSchemaConfig.__jsoncompat_object_spec__ = dc.jsoncompat_object_spec(
    dc.jsoncompat_field_spec("enable", "enable", (typing.Literal[False] | typing.Literal[True])),
)

GeneratedSchema.__jsoncompat_object_spec__ = dc.jsoncompat_object_spec(
    dc.jsoncompat_field_spec("config", "config", (GeneratedSchemaConfig | dc.JsoncompatMissingType), omittable=True),
    dc.jsoncompat_field_spec("id", "id", str),
    dc.jsoncompat_field_spec("name", "name", str),
)

JSONCOMPAT_MODEL = GeneratedSchema
