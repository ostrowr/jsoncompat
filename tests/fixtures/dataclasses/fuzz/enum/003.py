from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "bar": {
      "enum": [
        "bar"
      ]
    },
    "foo": {
      "enum": [
        "foo"
      ]
    }
  },
  "required": [
    "bar"
  ],
  "type": "object"
}"""
    bar: typing.Literal["bar"] = dc.field("bar")
    foo: dc.Omittable[typing.Literal["foo"]] = dc.field("foo", omittable=True)
    __jsoncompat_extra__: dict[str, typing.Any] = dc.extra_field()

JSONCOMPAT_MODEL = GeneratedSchema
