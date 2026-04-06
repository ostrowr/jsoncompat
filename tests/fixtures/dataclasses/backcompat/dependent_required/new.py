from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"dependentRequired\":{\"credit_card\":[\"billing_address\"]},\"minProperties\":0,\"properties\":{\"billing_address\":{\"minLength\":0,\"type\":\"string\"},\"credit_card\":{\"type\":\"number\"}},\"type\":\"object\"}"
    billing_address: (str | jsoncompat_dataclasses.JsoncompatMissingType) = jsoncompat_dataclasses.jsoncompat_field("billing_address", omittable=True)
    credit_card: (float | jsoncompat_dataclasses.JsoncompatMissingType) = jsoncompat_dataclasses.jsoncompat_field("credit_card", omittable=True)
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

GeneratedSchema.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("billing_address", "billing_address", (str | jsoncompat_dataclasses.JsoncompatMissingType), omittable=True),
    jsoncompat_dataclasses.jsoncompat_field_spec("credit_card", "credit_card", (float | jsoncompat_dataclasses.JsoncompatMissingType), omittable=True),
    extra_annotation=dict[str, typing.Any],
)

JSONCOMPAT_MODEL = GeneratedSchema
