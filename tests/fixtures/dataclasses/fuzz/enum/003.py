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
    bar: typing.Literal["bar"] = dc.jsoncompat_field("bar")
    foo: dc.Omittable[typing.Literal["foo"]] = dc.jsoncompat_field("foo", omittable=True)
    __jsoncompat_extra__: dict[str, typing.Any] = dc.jsoncompat_extra_field()

GeneratedSchema.__jsoncompat_object_spec__ = dc.jsoncompat_object_spec(
    dc.jsoncompat_field_spec("bar", "bar", typing.Literal["bar"]),
    dc.jsoncompat_field_spec("foo", "foo", (typing.Literal["foo"] | dc.JsoncompatMissingType), omittable=True),
    extra_annotation=dict[str, typing.Any],
)

JSONCOMPAT_MODEL = GeneratedSchema
