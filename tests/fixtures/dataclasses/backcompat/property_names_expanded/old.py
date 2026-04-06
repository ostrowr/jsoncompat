from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaFlags(dc.DataclassAdditionalModel[(typing.Literal[False] | typing.Literal[True])]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "additionalProperties": {
    "type": "boolean"
  },
  "propertyNames": {
    "enum": [
      "a"
    ]
  },
  "type": "object"
}"""
    __jsoncompat_extra__: dict[str, (typing.Literal[False] | typing.Literal[True])] = dc.jsoncompat_extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "flags": {
      "additionalProperties": {
        "type": "boolean"
      },
      "propertyNames": {
        "enum": [
          "a"
        ]
      },
      "type": "object"
    }
  },
  "required": [
    "flags"
  ],
  "type": "object"
}"""
    flags: GeneratedSchemaFlags = dc.jsoncompat_field("flags")
    __jsoncompat_extra__: dict[str, typing.Any] = dc.jsoncompat_extra_field()

GeneratedSchemaFlags.__jsoncompat_object_spec__ = dc.jsoncompat_object_spec(
    extra_annotation=dict[str, (typing.Literal[False] | typing.Literal[True])],
)

GeneratedSchema.__jsoncompat_object_spec__ = dc.jsoncompat_object_spec(
    dc.jsoncompat_field_spec("flags", "flags", GeneratedSchemaFlags),
    extra_annotation=dict[str, typing.Any],
)

JSONCOMPAT_MODEL = GeneratedSchema
