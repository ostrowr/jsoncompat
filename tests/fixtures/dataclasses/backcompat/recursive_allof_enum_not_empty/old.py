from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "additionalProperties": false,
  "properties": {
    "value": {
      "type": "integer"
    }
  },
  "required": [
    "value"
  ],
  "type": "object"
}"""
    value: int = dc.field("value")

JSONCOMPAT_MODEL = GeneratedSchema
