from __future__ import annotations

import collections.abc
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
      "anyOf": [
        {
          "$ref": "#/$defs/n1"
        },
        {
          "$ref": "#/$defs/n3"
        }
      ]
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n2": {
      "enum": [
        {
          "$anchor": "my_anchor",
          "type": "null"
        }
      ]
    },
    "n3": {
      "$ref": "#/$defs/n4"
    },
    "n4": {
      "minLength": 0,
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "anyOf": [
    {
      "$ref": "#/$defs/n1"
    },
    {
      "$ref": "#/$defs/n3"
    }
  ]
}"""
    root: (GeneratedSchemaN1 | GeneratedSchemaN3) = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN1(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "anyOf": [
        {
          "$ref": "#/$defs/n1"
        },
        {
          "$ref": "#/$defs/n3"
        }
      ]
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n2": {
      "enum": [
        {
          "$anchor": "my_anchor",
          "type": "null"
        }
      ]
    },
    "n3": {
      "$ref": "#/$defs/n4"
    },
    "n4": {
      "minLength": 0,
      "type": "string"
    }
  },
  "$ref": "#/$defs/n2"
}"""
    root: GeneratedSchemaN2 = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN2(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "anyOf": [
        {
          "$ref": "#/$defs/n1"
        },
        {
          "$ref": "#/$defs/n3"
        }
      ]
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n2": {
      "enum": [
        {
          "$anchor": "my_anchor",
          "type": "null"
        }
      ]
    },
    "n3": {
      "$ref": "#/$defs/n4"
    },
    "n4": {
      "minLength": 0,
      "type": "string"
    }
  },
  "enum": [
    {
      "$anchor": "my_anchor",
      "type": "null"
    }
  ]
}"""
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN3(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "anyOf": [
        {
          "$ref": "#/$defs/n1"
        },
        {
          "$ref": "#/$defs/n3"
        }
      ]
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n2": {
      "enum": [
        {
          "$anchor": "my_anchor",
          "type": "null"
        }
      ]
    },
    "n3": {
      "$ref": "#/$defs/n4"
    },
    "n4": {
      "minLength": 0,
      "type": "string"
    }
  },
  "$ref": "#/$defs/n4"
}"""
    root: str = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN4(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "anyOf": [
        {
          "$ref": "#/$defs/n1"
        },
        {
          "$ref": "#/$defs/n3"
        }
      ]
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n2": {
      "enum": [
        {
          "$anchor": "my_anchor",
          "type": "null"
        }
      ]
    },
    "n3": {
      "$ref": "#/$defs/n4"
    },
    "n4": {
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
    "anchor_in_enum": {
      "enum": [
        {
          "$anchor": "my_anchor",
          "type": "null"
        }
      ]
    },
    "real_identifier_in_schema": {
      "$anchor": "my_anchor",
      "type": "string"
    },
    "zzz_anchor_in_const": {
      "const": {
        "$anchor": "my_anchor",
        "type": "null"
      }
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "anyOf": [
    {
      "$ref": "#/$defs/anchor_in_enum"
    },
    {
      "$ref": "#my_anchor"
    }
  ]
}"""
    root: GeneratedSchemaN0 = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
