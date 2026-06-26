from __future__ import annotations

import collections.abc
from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaABranch2(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "A": {
      "unevaluatedProperties": false
    }
  },
  "minProperties": 0,
  "properties": {},
  "type": "object",
  "unevaluatedProperties": false
}"""
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaAItem(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaA(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "A": {
      "unevaluatedProperties": false
    }
  },
  "unevaluatedProperties": false
}"""
    root: ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaABranch2 | collections.abc.Sequence[GeneratedSchemaAItem] | float | str | None) = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "A": {
      "unevaluatedProperties": false
    }
  },
  "$ref": "#/$defs/A",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "prop1": {
      "type": "string"
    }
  }
}"""
    root: GeneratedSchemaA = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
