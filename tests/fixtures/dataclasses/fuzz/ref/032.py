from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaFoo(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "foo": {
      "type": "number"
    }
  },
  "type": "number"
}"""
    root: float = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "foo": {
      "type": "number"
    }
  },
  "$id": "file:///folder/file.json",
  "$ref": "#/$defs/foo",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}"""
    root: GeneratedSchemaFoo = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchemaFoo.__jsoncompat_root_annotation__ = float

GeneratedSchema.__jsoncompat_root_annotation__ = GeneratedSchemaFoo

JSONCOMPAT_MODEL = GeneratedSchema
