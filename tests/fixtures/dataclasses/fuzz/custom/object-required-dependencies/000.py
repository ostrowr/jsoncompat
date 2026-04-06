from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "dependentRequired": {
    "name": [
      "email"
    ]
  },
  "minProperties": 2,
  "properties": {
    "email": {
      "type": "string"
    },
    "kind": {
      "const": "user"
    },
    "name": {
      "type": "string"
    }
  },
  "required": [
    "kind",
    "email"
  ],
  "type": "object"
}"""
    email: str = dc.jsoncompat_field("email")
    kind: typing.Literal["user"] = dc.jsoncompat_field("kind")
    name: dc.Omittable[str] = dc.jsoncompat_field("name", omittable=True)

GeneratedSchema.__jsoncompat_object_spec__ = dc.jsoncompat_object_spec(
    dc.jsoncompat_field_spec("email", "email", str),
    dc.jsoncompat_field_spec("kind", "kind", typing.Literal["user"]),
    dc.jsoncompat_field_spec("name", "name", (str | dc.JsoncompatMissingType), omittable=True),
)

JSONCOMPAT_MODEL = GeneratedSchema
