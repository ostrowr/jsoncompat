from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "maxProperties": 0,
  "type": "object"
}"""
    __jsoncompat_extra__: typing.Mapping[str, typing.Any] = dc.extra_field()

JSONCOMPAT_MODEL = GeneratedSchema
