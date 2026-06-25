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
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "contains": {
    "type": "string"
  },
  "maxContains": 2,
  "maxItems": 3,
  "minContains": 2,
  "minItems": 2,
  "type": "array"
}"""
    root: collections.abc.Sequence[GeneratedSchemaItem] = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
