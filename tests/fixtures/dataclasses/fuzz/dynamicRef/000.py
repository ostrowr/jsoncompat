from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaFoo(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "foo": {
      "$dynamicAnchor": "items",
      "type": "string"
    }
  },
  "$dynamicAnchor": "items",
  "type": "string"
}"""
    root: str = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaItem(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "foo": {
      "$dynamicAnchor": "items",
      "type": "string"
    }
  },
  "$dynamicRef": "#items"
}"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "foo": {
      "$dynamicAnchor": "items",
      "type": "string"
    }
  },
  "$id": "https://test.json-schema.org/dynamicRef-dynamicAnchor-same-schema/root",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": {
    "$dynamicRef": "#items"
  },
  "type": "array"
}"""
    root: list[GeneratedSchemaItem] = dc.root_field()

GeneratedSchemaFoo.__jsoncompat_root_annotation__ = str

GeneratedSchemaItem.__jsoncompat_root_annotation__ = typing.Any

GeneratedSchema.__jsoncompat_root_annotation__ = list[GeneratedSchemaItem]

JSONCOMPAT_MODEL = GeneratedSchema
