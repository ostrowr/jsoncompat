from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch0(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """false"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "oneOf": [
    false,
    {
      "type": "boolean"
    }
  ]
}"""
    root: ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch0) = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
