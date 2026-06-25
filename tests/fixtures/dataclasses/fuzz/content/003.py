from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "contentEncoding": "base64",
  "contentMediaType": "application/json",
  "contentSchema": {
    "properties": {
      "foo": {
        "type": "string"
      }
    },
    "required": [
      "foo"
    ],
    "type": "object"
  }
}"""
    root: dc.JsonValue = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
