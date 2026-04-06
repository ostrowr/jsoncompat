from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaFlags(jsoncompat_dataclasses.DataclassAdditionalModel[(typing.Literal[False] | typing.Literal[True])]):
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
    __jsoncompat_extra__: dict[str, (typing.Literal[False] | typing.Literal[True])] = jsoncompat_dataclasses.jsoncompat_extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
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
    flags: GeneratedSchemaFlags = jsoncompat_dataclasses.jsoncompat_field("flags")
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

GeneratedSchemaFlags.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    extra_annotation=dict[str, (typing.Literal[False] | typing.Literal[True])],
)

GeneratedSchema.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("flags", "flags", GeneratedSchemaFlags),
    extra_annotation=dict[str, typing.Any],
)

JSONCOMPAT_MODEL = GeneratedSchema
