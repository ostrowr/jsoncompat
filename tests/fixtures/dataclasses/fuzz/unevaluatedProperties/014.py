from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "else": {
    "properties": {
      "baz": {
        "type": "string"
      }
    },
    "required": [
      "baz"
    ]
  },
  "if": {
    "properties": {
      "foo": {
        "const": "then"
      }
    },
    "required": [
      "foo"
    ]
  },
  "type": "object",
  "unevaluatedProperties": false
}"""
    __jsoncompat_extra__: dict[str, typing.Any] = dc.extra_field()

GeneratedSchema.__jsoncompat_object_spec__ = dc.object_spec(
    extra_annotation=dict[str, typing.Any],
)

JSONCOMPAT_MODEL = GeneratedSchema
