from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaPercentField(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "percent%field": {
      "type": "integer"
    },
    "slash/field": {
      "type": "integer"
    },
    "tilde~field": {
      "type": "integer"
    }
  },
  "type": "integer"
}"""
    root: int = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaSlashField(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "percent%field": {
      "type": "integer"
    },
    "slash/field": {
      "type": "integer"
    },
    "tilde~field": {
      "type": "integer"
    }
  },
  "type": "integer"
}"""
    root: int = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaTildeField(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "percent%field": {
      "type": "integer"
    },
    "slash/field": {
      "type": "integer"
    },
    "tilde~field": {
      "type": "integer"
    }
  },
  "type": "integer"
}"""
    root: int = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "percent%field": {
      "type": "integer"
    },
    "slash/field": {
      "type": "integer"
    },
    "tilde~field": {
      "type": "integer"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "percent": {
      "$ref": "#/$defs/percent%25field"
    },
    "slash": {
      "$ref": "#/$defs/slash~1field"
    },
    "tilde": {
      "$ref": "#/$defs/tilde~0field"
    }
  }
}"""
    root: typing.Any = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema

dc.bind_generated_models((
    (GeneratedSchemaPercentField, "root", int),
    (GeneratedSchemaSlashField, "root", int),
    (GeneratedSchemaTildeField, "root", int),
    (GeneratedSchema, "root", typing.Any),
))
