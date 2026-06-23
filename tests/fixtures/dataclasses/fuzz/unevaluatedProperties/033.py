from __future__ import annotations

import collections.abc
from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaOneBranch2A(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaOneBranch2(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "one": {
      "properties": {
        "a": true
      }
    },
    "two": {
      "properties": {
        "x": true
      },
      "required": [
        "x"
      ]
    }
  },
  "minProperties": 0,
  "properties": {
    "a": true
  },
  "type": "object"
}"""
    a: dc.Omittable[GeneratedSchemaOneBranch2A] = dc.field("a", omittable=True)
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaOneItem(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaOne(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "one": {
      "properties": {
        "a": true
      }
    },
    "two": {
      "properties": {
        "x": true
      },
      "required": [
        "x"
      ]
    }
  },
  "properties": {
    "a": true
  }
}"""
    root: ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaOneBranch2 | collections.abc.Sequence[GeneratedSchemaOneItem] | float | str | None) = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaTwoBranch2X(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaTwoBranch2(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "one": {
      "properties": {
        "a": true
      }
    },
    "two": {
      "properties": {
        "x": true
      },
      "required": [
        "x"
      ]
    }
  },
  "minProperties": 1,
  "properties": {
    "x": true
  },
  "required": [
    "x"
  ],
  "type": "object"
}"""
    x: GeneratedSchemaTwoBranch2X = dc.field("x")
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaTwoItem(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaTwo(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "one": {
      "properties": {
        "a": true
      }
    },
    "two": {
      "properties": {
        "x": true
      },
      "required": [
        "x"
      ]
    }
  },
  "properties": {
    "x": true
  },
  "required": [
    "x"
  ]
}"""
    root: ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaTwoBranch2 | collections.abc.Sequence[GeneratedSchemaTwoItem] | float | str | None) = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "one": {
      "properties": {
        "a": true
      }
    },
    "two": {
      "properties": {
        "x": true
      },
      "required": [
        "x"
      ]
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "$ref": "#/$defs/one"
    },
    {
      "properties": {
        "b": true
      }
    },
    {
      "oneOf": [
        {
          "$ref": "#/$defs/two"
        },
        {
          "properties": {
            "y": true
          },
          "required": [
            "y"
          ]
        }
      ]
    }
  ],
  "unevaluatedProperties": false
}"""
    root: typing.Any = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema

dc.bind_generated_models((
    (GeneratedSchemaOneBranch2A, "root", typing.Any),
    (
        GeneratedSchemaOneBranch2,
        "object",
        (
            ("a", "a", GeneratedSchemaOneBranch2A, True),
        ),
        True,
        typing.Any,
    ),
    (GeneratedSchemaOneItem, "root", typing.Any),
    (GeneratedSchemaOne, "root", ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaOneBranch2 | collections.abc.Sequence[GeneratedSchemaOneItem] | float | str | None)),
    (GeneratedSchemaTwoBranch2X, "root", typing.Any),
    (
        GeneratedSchemaTwoBranch2,
        "object",
        (
            ("x", "x", GeneratedSchemaTwoBranch2X, False),
        ),
        True,
        typing.Any,
    ),
    (GeneratedSchemaTwoItem, "root", typing.Any),
    (GeneratedSchemaTwo, "root", ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaTwoBranch2 | collections.abc.Sequence[GeneratedSchemaTwoItem] | float | str | None)),
    (GeneratedSchema, "root", typing.Any),
))
