from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "minItems": 1,
  "prefixItems": [
    {
      "type": "integer"
    }
  ],
  "type": "array"
}"""
    root: list[typing.Any] = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
