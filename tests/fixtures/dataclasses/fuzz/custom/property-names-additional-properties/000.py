from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassAdditionalModel[int]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": {
    "type": "integer"
  },
  "properties": {
    "id": {
      "type": "integer"
    }
  },
  "propertyNames": {
    "pattern": "^[a-z]+$"
  },
  "required": [
    "id"
  ],
  "type": "object"
}"""
    id: int = dc.jsoncompat_field("id")
    __jsoncompat_extra__: dict[str, int] = dc.jsoncompat_extra_field()

GeneratedSchema.__jsoncompat_object_spec__ = dc.jsoncompat_object_spec(
    dc.jsoncompat_field_spec("id", "id", int),
    extra_annotation=dict[str, int],
)

JSONCOMPAT_MODEL = GeneratedSchema
