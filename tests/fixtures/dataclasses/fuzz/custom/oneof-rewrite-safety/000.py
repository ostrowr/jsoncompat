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
    kind: typing.Literal["int"] = dc.field("kind")
    value: int = dc.field("value")

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
    kind: typing.Literal["str"] = dc.field("kind")
    value: str = dc.field("value")

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
    root: (GeneratedSchemaBranch0 | GeneratedSchemaBranch1) = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
