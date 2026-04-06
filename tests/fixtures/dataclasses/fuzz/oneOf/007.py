from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch1(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{}"""
    root: typing.Any = dc.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "oneOf": [
    {
      "type": "number"
    },
    {}
  ]
}"""
    root: (GeneratedSchemaBranch1 | float) = dc.jsoncompat_root_field()

GeneratedSchemaBranch1.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchema.__jsoncompat_root_annotation__ = (GeneratedSchemaBranch1 | float)

JSONCOMPAT_MODEL = GeneratedSchema
