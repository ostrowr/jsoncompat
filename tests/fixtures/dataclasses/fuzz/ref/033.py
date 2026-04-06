from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaFoo(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "foo": {
      "type": "number"
    }
  },
  "type": "number"
}"""
    root: float = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "foo": {
      "type": "number"
    }
  },
  "$id": "file:///c:/folder/file.json",
  "$ref": "#/$defs/foo",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}"""
    root: GeneratedSchemaFoo = dc.root_field()

GeneratedSchemaFoo.__jsoncompat_root_annotation__ = float

GeneratedSchema.__jsoncompat_root_annotation__ = GeneratedSchemaFoo

JSONCOMPAT_MODEL = GeneratedSchema
