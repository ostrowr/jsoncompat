from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch0(dc.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "additionalProperties": false,
  "properties": {
    "kind": {
      "const": "int"
    },
    "value": {
      "type": "integer"
    }
  },
  "required": [
    "kind",
    "value"
  ],
  "type": "object"
}"""
    kind: typing.Literal["int"] = dc.jsoncompat_field("kind")
    value: int = dc.jsoncompat_field("value")

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch1(dc.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "additionalProperties": false,
  "properties": {
    "kind": {
      "const": "str"
    },
    "value": {
      "type": "string"
    }
  },
  "required": [
    "kind",
    "value"
  ],
  "type": "object"
}"""
    kind: typing.Literal["str"] = dc.jsoncompat_field("kind")
    value: str = dc.jsoncompat_field("value")

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "oneOf": [
    {
      "additionalProperties": false,
      "properties": {
        "kind": {
          "const": "int"
        },
        "value": {
          "type": "integer"
        }
      },
      "required": [
        "kind",
        "value"
      ],
      "type": "object"
    },
    {
      "additionalProperties": false,
      "properties": {
        "kind": {
          "const": "str"
        },
        "value": {
          "type": "string"
        }
      },
      "required": [
        "kind",
        "value"
      ],
      "type": "object"
    }
  ]
}"""
    root: (GeneratedSchemaBranch0 | GeneratedSchemaBranch1) = dc.jsoncompat_root_field()

GeneratedSchemaBranch0.__jsoncompat_object_spec__ = dc.jsoncompat_object_spec(
    dc.jsoncompat_field_spec("kind", "kind", typing.Literal["int"]),
    dc.jsoncompat_field_spec("value", "value", int),
)

GeneratedSchemaBranch1.__jsoncompat_object_spec__ = dc.jsoncompat_object_spec(
    dc.jsoncompat_field_spec("kind", "kind", typing.Literal["str"]),
    dc.jsoncompat_field_spec("value", "value", str),
)

GeneratedSchema.__jsoncompat_root_annotation__ = (GeneratedSchemaBranch0 | GeneratedSchemaBranch1)

JSONCOMPAT_MODEL = GeneratedSchema
