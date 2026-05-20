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
    email: str = dc.field("email")
    kind: typing.Literal["user"] = dc.field("kind")
    name: dc.Omittable[str] = dc.field("name", omittable=True)

JSONCOMPAT_MODEL = GeneratedSchema
