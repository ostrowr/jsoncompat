from __future__ import annotations

import collections.abc
from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaItem(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaItem2(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "oneOf": [
    {
      "type": "array"
    },
    {
      "anyOf": [
        {
          "minItems": 1,
          "type": "array"
        },
        {
          "type": "string"
        }
      ]
    }
  ]
}"""
    root: ((collections.abc.Sequence[GeneratedSchemaItem2] | str) | collections.abc.Sequence[GeneratedSchemaItem]) = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema

dc.bind_generated_models((
    (GeneratedSchemaItem, "root", typing.Any),
    (GeneratedSchemaItem2, "root", typing.Any),
    (GeneratedSchema, "root", ((collections.abc.Sequence[GeneratedSchemaItem2] | str) | collections.abc.Sequence[GeneratedSchemaItem])),
))
