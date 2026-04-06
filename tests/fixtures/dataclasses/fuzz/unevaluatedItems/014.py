from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "else": {
    "prefixItems": [
      true,
      true,
      true,
      {
        "const": "else"
      }
    ]
  },
  "if": {
    "prefixItems": [
      true,
      {
        "const": "bar"
      }
    ]
  },
  "prefixItems": [
    {
      "const": "foo"
    }
  ],
  "then": {
    "prefixItems": [
      true,
      true,
      {
        "const": "then"
      }
    ]
  },
  "unevaluatedItems": false
}"""
    root: typing.Any = dc.jsoncompat_root_field()

GeneratedSchema.__jsoncompat_root_annotation__ = typing.Any

JSONCOMPAT_MODEL = GeneratedSchema
