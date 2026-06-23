from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaFalse(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """false"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaTrue(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "false": false,
    "true": true
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "false": {
      "$dynamicRef": "#/$defs/false"
    },
    "true": {
      "$dynamicRef": "#/$defs/true"
    }
  }
}"""
    root: typing.Any = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema

dc.bind_generated_models((
    (GeneratedSchemaFalse, "root", typing.Any),
    (GeneratedSchemaTrue, "root", typing.Any),
    (GeneratedSchema, "root", typing.Any),
))
