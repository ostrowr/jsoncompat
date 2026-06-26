from __future__ import annotations

import collections.abc
from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "minProperties": 0,
  "prefixItems": [
    {
      "enum": [
        "foo"
      ]
    }
  ],
  "properties": {},
  "type": "object",
  "unevaluatedItems": false
}"""
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch22(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "minProperties": 0,
  "prefixItems": [
    {
      "enum": [
        "foo"
      ]
    }
  ],
  "properties": {},
  "type": "object",
  "unevaluatedItems": false
}"""
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "oneOf": [
    {
      "prefixItems": [
        true,
        {
          "const": "bar"
        }
      ]
    },
    {
      "prefixItems": [
        true,
        {
          "const": "baz"
        }
      ]
    }
  ],
  "prefixItems": [
    {
      "const": "foo"
    }
  ],
  "unevaluatedItems": false
}"""
    root: (((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | collections.abc.Sequence[typing.Any] | float | str | None) | ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch22 | collections.abc.Sequence[typing.Any] | float | str | None)) = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
