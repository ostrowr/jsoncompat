from __future__ import annotations

import collections.abc
from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch0(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "type": "object"
}"""
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch02P(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch02(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "required": [
    "p"
  ],
  "type": "object"
}"""
    p: GeneratedSchemaBranch02P = dc.field("p")
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "oneOf": [
    {
      "type": "object"
    },
    {
      "anyOf": [
        {
          "required": [
            "p"
          ],
          "type": "object"
        },
        {
          "type": "string"
        }
      ]
    }
  ]
}"""
    root: ((GeneratedSchemaBranch02 | str) | GeneratedSchemaBranch0) = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema

dc.bind_generated_models((
    (
        GeneratedSchemaBranch0,
        "object",
        (
        ),
        True,
        typing.Any,
    ),
    (GeneratedSchemaBranch02P, "root", typing.Any),
    (
        GeneratedSchemaBranch02,
        "object",
        (
            ("p", "p", GeneratedSchemaBranch02P, False),
        ),
        True,
        typing.Any,
    ),
    (GeneratedSchema, "root", ((GeneratedSchemaBranch02 | str) | GeneratedSchemaBranch0)),
))
