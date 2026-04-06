from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "enum": [
    "red",
    "blue",
    "green"
  ],
  "type": "string"
}"""
    root: (typing.Literal["blue"] | typing.Literal["green"] | typing.Literal["red"]) = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchema.__jsoncompat_root_annotation__ = (typing.Literal["blue"] | typing.Literal["green"] | typing.Literal["red"])

JSONCOMPAT_MODEL = GeneratedSchema
