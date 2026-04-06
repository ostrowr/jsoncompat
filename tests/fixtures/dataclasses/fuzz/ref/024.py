from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


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

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$comment": "RFC 8141 §2.3.2",
  "$defs": {
    "bar": {
      "type": "string"
    }
  },
  "$id": "urn:example:weather?=op=map&lat=39.56&lon=-104.85&datetime=1969-07-21T02:56:15Z",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo": {
      "$ref": "#/$defs/bar"
    }
  }
}"""
    foo: dc.Omittable[GeneratedSchemaBar] = dc.field("foo", omittable=True)
    __jsoncompat_extra__: dict[str, typing.Any] = dc.extra_field()

JSONCOMPAT_MODEL = GeneratedSchema
