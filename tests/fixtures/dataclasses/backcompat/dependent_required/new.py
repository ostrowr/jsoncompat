from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "dependentRequired": {
    "credit_card": [
      "billing_address"
    ]
  },
  "properties": {
    "billing_address": {
      "type": "string"
    },
    "credit_card": {
      "type": "number"
    }
  },
  "type": "object"
}"""
    billing_address: dc.Omittable[str] = dc.field("billing_address", omittable=True)
    credit_card: dc.Omittable[float] = dc.field("credit_card", omittable=True)
    __jsoncompat_extra__: dict[str, typing.Any] = dc.extra_field()

GeneratedSchema.__jsoncompat_object_spec__ = dc.object_spec(
    dc.field_spec("billing_address", "billing_address", (str | dc.JsoncompatMissingType), omittable=True),
    dc.field_spec("credit_card", "credit_card", (float | dc.JsoncompatMissingType), omittable=True),
    extra_annotation=dict[str, typing.Any],
)

JSONCOMPAT_MODEL = GeneratedSchema
