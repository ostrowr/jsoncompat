from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "properties": {
    "id": {
      "type": "integer"
    }
  },
  "required": [],
  "type": "object"
}"""
    id: dc.Omittable[int] = dc.field("id", omittable=True)
    __jsoncompat_extra__: dict[str, typing.Any] = dc.extra_field()

GeneratedSchema.__jsoncompat_object_spec__ = dc.object_spec(
    dc.field_spec("id", "id", (int | dc.JsoncompatMissingType), omittable=True),
    extra_annotation=dict[str, typing.Any],
)

JSONCOMPAT_MODEL = GeneratedSchema
