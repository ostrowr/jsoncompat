from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBool(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "bool": true
  },
  "$ref": "#/$defs/bool",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}"""
    root: GeneratedSchemaBool = dc.jsoncompat_root_field()

GeneratedSchemaBool.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchema.__jsoncompat_root_annotation__ = GeneratedSchemaBool

JSONCOMPAT_MODEL = GeneratedSchema
