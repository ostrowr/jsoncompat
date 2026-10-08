from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN0(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$ref": "#/$defs/n1",
      "$schema": "https://json-schema.org/draft/2020-12/schema"
    },
    "n1": {
      "minLength": 0,
      "type": "string"
    }
  },
  "$ref": "#/$defs/n1",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}"""
    root: str = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN1(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$ref": "#/$defs/n1",
      "$schema": "https://json-schema.org/draft/2020-12/schema"
    },
    "n1": {
      "minLength": 0,
      "type": "string"
    }
  },
  "minLength": 0,
  "type": "string"
}"""
    root: str = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "a": {
      "$id": "http://example.com/ref/absref/foobar.json",
      "type": "number"
    },
    "b": {
      "$id": "http://example.com/absref/foobar.json",
      "type": "string"
    }
  },
  "$id": "http://example.com/ref/absref.json",
  "$ref": "/absref/foobar.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}"""
    root: GeneratedSchemaN0 = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
