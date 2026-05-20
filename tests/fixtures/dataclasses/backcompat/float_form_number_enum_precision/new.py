from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "const": 9007199254740994
}"""
    root: typing.Literal[9007199254740994] = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
