from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "const": 9007199254740992
}"""
    root: typing.Literal[9007199254740992] = dc.jsoncompat_root_field()

GeneratedSchema.__jsoncompat_root_annotation__ = typing.Literal[9007199254740992]

JSONCOMPAT_MODEL = GeneratedSchema
