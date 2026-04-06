from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "properties": {
    "credit_card": {
      "type": "number"
    }
  },
  "type": "object"
}"""
    credit_card: dc.Omittable[float] = dc.jsoncompat_field("credit_card", omittable=True)
    __jsoncompat_extra__: dict[str, typing.Any] = dc.jsoncompat_extra_field()

GeneratedSchema.__jsoncompat_object_spec__ = dc.jsoncompat_object_spec(
    dc.jsoncompat_field_spec("credit_card", "credit_card", (float | dc.JsoncompatMissingType), omittable=True),
    extra_annotation=dict[str, typing.Any],
)

JSONCOMPAT_MODEL = GeneratedSchema
