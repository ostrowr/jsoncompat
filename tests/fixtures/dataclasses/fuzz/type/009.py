from __future__ import annotations

import collections.abc
from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaItem(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch1(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "minProperties": 0,
  "properties": {},
  "type": "object"
}"""
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": [
    "array",
    "object"
  ]
}"""
    root: (GeneratedSchemaBranch1 | collections.abc.Sequence[GeneratedSchemaItem]) = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
