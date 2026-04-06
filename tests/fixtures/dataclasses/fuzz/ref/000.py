from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "foo": {
      "$ref": "#"
    }
  }
}"""
    foo: jsoncompat_dataclasses.Omittable[GeneratedSchema] = jsoncompat_dataclasses.jsoncompat_field("foo", omittable=True)

GeneratedSchema.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("foo", "foo", (GeneratedSchema | jsoncompat_dataclasses.JsoncompatMissingType), omittable=True),
)

JSONCOMPAT_MODEL = GeneratedSchema
