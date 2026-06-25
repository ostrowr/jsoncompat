from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBar(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "bar": {
      "type": "string"
    }
  },
  "type": "string"
}"""
    root: str = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$comment": "RFC 8141 §2.2",
  "$defs": {
    "bar": {
      "type": "string"
    }
  },
  "$id": "urn:example:1/406/47452/2",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo": {
      "$ref": "#/$defs/bar"
    }
  }
}"""
    root: typing.Any = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
