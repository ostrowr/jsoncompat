from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "items": false,
  "minItems": 1,
  "prefixItems": [
    {
      "type": "integer"
    },
    {
      "type": "string"
    }
  ],
  "type": "array"
}"""
    root: list[(int | str)] = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
