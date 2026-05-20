from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "enum": [
    "foo\\nbar",
    "foo\\rbar"
  ]
}"""
    root: (typing.Literal["foo\nbar"] | typing.Literal["foo\rbar"]) = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
