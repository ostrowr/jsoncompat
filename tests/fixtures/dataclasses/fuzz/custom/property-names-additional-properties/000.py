from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassAdditionalModel[int]):
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
    id: int = jsoncompat_dataclasses.jsoncompat_field("id")
    __jsoncompat_extra__: dict[str, int] = jsoncompat_dataclasses.jsoncompat_extra_field()

GeneratedSchema.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("id", "id", int),
    extra_annotation=dict[str, int],
)

JSONCOMPAT_MODEL = GeneratedSchema
