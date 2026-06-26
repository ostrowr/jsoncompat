from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaA(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$ref": "#/definitions/B",
  "definitions": {
    "A": {
      "$ref": "#/definitions/B"
    },
    "B": {
      "type": "string"
    }
  }
}"""
    root: str = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaB(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "definitions": {
    "A": {
      "$ref": "#/definitions/B"
    },
    "B": {
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
  "$ref": "#/definitions/A",
  "definitions": {
    "A": {
      "$ref": "#/definitions/B"
    },
    "B": {
      "type": "string"
    }
  }
}"""
    root: GeneratedSchemaA = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
