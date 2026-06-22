from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaP(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """false"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "properties": {
    "p": false
  },
  "type": "object"
}"""
    p: dc.Omittable[GeneratedSchemaP] = dc.field("p", omittable=True)
    __jsoncompat_extra__: typing.Mapping[str, typing.Any] = dc.extra_field()

JSONCOMPAT_MODEL = GeneratedSchema
