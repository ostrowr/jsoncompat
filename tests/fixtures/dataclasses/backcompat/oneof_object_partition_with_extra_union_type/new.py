from __future__ import annotations

import collections.abc
from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch0(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "type": "object"
}"""
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch02(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "minProperties": 1,
  "type": "object"
}"""
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "oneOf": [
    {
      "type": "object"
    },
    {
      "anyOf": [
        {
          "minProperties": 1,
          "type": "object"
        },
        {
          "type": "string"
        }
      ]
    }
  ]
}"""
    root: ((GeneratedSchemaBranch02 | str) | GeneratedSchemaBranch0) = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
