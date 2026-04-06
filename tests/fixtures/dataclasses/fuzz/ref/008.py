from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaIsString(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "is-string": {
      "type": "string"
    }
  },
  "type": "string"
}"""
    root: str = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "is-string": {
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "$ref": {
      "$ref": "#/$defs/is-string"
    }
  }
}"""
    _ref: dc.Omittable[GeneratedSchemaIsString] = dc.field("$ref", omittable=True)
    __jsoncompat_extra__: dict[str, typing.Any] = dc.extra_field()

GeneratedSchemaIsString.__jsoncompat_root_annotation__ = str

GeneratedSchema.__jsoncompat_object_spec__ = dc.object_spec(
    dc.field_spec("_ref", "$ref", (GeneratedSchemaIsString | dc.JsoncompatMissingType), omittable=True),
    extra_annotation=dict[str, typing.Any],
)

JSONCOMPAT_MODEL = GeneratedSchema
