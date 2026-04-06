from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2(dc.DataclassAdditionalModel[float]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": {
    "type": "number"
  },
  "minProperties": 0,
  "properties": {},
  "propertyNames": {
    "anyOf": [
      {
        "enum": [
          null
        ]
      },
      {
        "enum": [
          false,
          true
        ]
      },
      {
        "minProperties": 0,
        "properties": {},
        "type": "object"
      },
      {
        "items": true,
        "minItems": 0,
        "type": "array"
      },
      {
        "maxLength": 5,
        "minLength": 0,
        "type": "string"
      },
      {
        "type": "number"
      }
    ]
  },
  "type": "object"
}"""
    __jsoncompat_extra__: dict[str, float] = dc.extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaItem(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": {
    "type": "number"
  },
  "propertyNames": {
    "maxLength": 5
  }
}"""
    root: ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | float | list[GeneratedSchemaItem] | str | None) = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
