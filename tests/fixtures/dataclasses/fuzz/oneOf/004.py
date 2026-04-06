from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch0(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch1(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """false"""
    root: typing.Any = dc.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "oneOf": [
    true,
    true,
    false
  ]
}"""
    root: (GeneratedSchemaBranch0 | GeneratedSchemaBranch1 | GeneratedSchemaBranch2) = dc.jsoncompat_root_field()

GeneratedSchemaBranch0.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaBranch1.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchemaBranch2.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchema.__jsoncompat_root_annotation__ = (GeneratedSchemaBranch0 | GeneratedSchemaBranch1 | GeneratedSchemaBranch2)

JSONCOMPAT_MODEL = GeneratedSchema
