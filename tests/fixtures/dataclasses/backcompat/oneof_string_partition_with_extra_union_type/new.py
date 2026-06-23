from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "oneOf": [
    {
      "type": "string"
    },
    {
      "anyOf": [
        {
          "minLength": 1,
          "type": "string"
        },
        {
          "type": "number"
        }
      ]
    }
  ]
}"""
    root: ((float | str) | str) = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
