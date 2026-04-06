from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "not": {
    "$comment": "this subschema must still produce annotations internally, even though the 'not' will ultimately discard them",
    "anyOf": [
      true,
      {
        "properties": {
          "foo": true
        }
      }
    ],
    "unevaluatedProperties": false
  }
}"""
    root: typing.Any = dc.jsoncompat_root_field()

GeneratedSchema.__jsoncompat_root_annotation__ = typing.Any

JSONCOMPAT_MODEL = GeneratedSchema
