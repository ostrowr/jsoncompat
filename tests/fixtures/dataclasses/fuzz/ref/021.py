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
  "$comment": "URIs do not have to have HTTP(s) schemes",
  "$defs": {
    "bar": {
      "type": "string"
    }
  },
  "$id": "urn:uuid:deadbeef-1234-00ff-ff00-4321feebdaed",
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
