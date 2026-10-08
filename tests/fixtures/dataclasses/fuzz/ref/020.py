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
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minimum": 30,
      "properties": {
        "foo": {
          "$ref": "#/$defs/n1"
        }
      }
    },
    "n1": {
      "$ref": "#/$defs/n0"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "minimum": 30,
  "properties": {
    "foo": {
      "$ref": "#/$defs/n1"
    }
  }
}"""
    root: typing.Any = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN1(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minimum": 30,
      "properties": {
        "foo": {
          "$ref": "#/$defs/n1"
        }
      }
    },
    "n1": {
      "$ref": "#/$defs/n0"
    }
  },
  "$ref": "#/$defs/n0"
}"""
    root: GeneratedSchemaN0 = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$comment": "URIs do not have to have HTTP(s) schemes",
  "$id": "urn:uuid:deadbeef-1234-ffff-ffff-4321feebdaed",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "minimum": 30,
  "properties": {
    "foo": {
      "$ref": "urn:uuid:deadbeef-1234-ffff-ffff-4321feebdaed"
    }
  }
}"""
    root: GeneratedSchemaN0 = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
