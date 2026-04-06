from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaA(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "a": {
      "type": "integer"
    },
    "b": {
      "$ref": "#/$defs/a"
    },
    "c": {
      "$ref": "#/$defs/b"
    }
  },
  "type": "integer"
}"""
    root: int = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaB(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "a": {
      "type": "integer"
    },
    "b": {
      "$ref": "#/$defs/a"
    },
    "c": {
      "$ref": "#/$defs/b"
    }
  },
  "$ref": "#/$defs/a"
}"""
    root: GeneratedSchemaA = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaC(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "a": {
      "type": "integer"
    },
    "b": {
      "$ref": "#/$defs/a"
    },
    "c": {
      "$ref": "#/$defs/b"
    }
  },
  "$ref": "#/$defs/b"
}"""
    root: GeneratedSchemaB = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "a": {
      "type": "integer"
    },
    "b": {
      "$ref": "#/$defs/a"
    },
    "c": {
      "$ref": "#/$defs/b"
    }
  },
  "$ref": "#/$defs/c",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}"""
    root: GeneratedSchemaC = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
