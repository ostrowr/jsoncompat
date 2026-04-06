from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "enum": [
    1
  ],
  "properties": {
    "x": {
      "type": "string"
    }
  }
}"""
    x: dc.Omittable[str] = dc.jsoncompat_field("x", omittable=True)
    __jsoncompat_extra__: dict[str, typing.Any] = dc.jsoncompat_extra_field()

GeneratedSchema.__jsoncompat_object_spec__ = dc.jsoncompat_object_spec(
    dc.jsoncompat_field_spec("x", "x", (str | dc.JsoncompatMissingType), omittable=True),
    extra_annotation=dict[str, typing.Any],
)

JSONCOMPAT_MODEL = GeneratedSchema
