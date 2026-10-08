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
      "$ref": "#/$defs/n1",
      "$schema": "https://json-schema.org/draft/2020-12/schema"
    },
    "n1": {
      "items": {
        "$ref": "#/$defs/n2"
      },
      "minItems": 0,
      "type": "array"
    },
    "n2": {
      "$ref": "#/$defs/n3"
    },
    "n3": {
      "type": "number"
    }
  },
  "$ref": "#/$defs/n1",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}"""
    root: GeneratedSchemaN1 = dc.root_field()

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
      "items": {
        "$ref": "#/$defs/n2"
      },
      "minItems": 0,
      "type": "array"
    },
    "n2": {
      "$ref": "#/$defs/n3"
    },
    "n3": {
      "type": "number"
    }
  },
  "items": {
    "$ref": "#/$defs/n2"
  },
  "minItems": 0,
  "type": "array"
}"""
    root: collections.abc.Sequence[GeneratedSchemaN2] = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN2(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$ref": "#/$defs/n1",
      "$schema": "https://json-schema.org/draft/2020-12/schema"
    },
    "n1": {
      "items": {
        "$ref": "#/$defs/n2"
      },
      "minItems": 0,
      "type": "array"
    },
    "n2": {
      "$ref": "#/$defs/n3"
    },
    "n3": {
      "type": "number"
    }
  },
  "$ref": "#/$defs/n3"
}"""
    root: float = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN3(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$ref": "#/$defs/n1",
      "$schema": "https://json-schema.org/draft/2020-12/schema"
    },
    "n1": {
      "items": {
        "$ref": "#/$defs/n2"
      },
      "minItems": 0,
      "type": "array"
    },
    "n2": {
      "$ref": "#/$defs/n3"
    },
    "n3": {
      "type": "number"
    }
  },
  "type": "number"
}"""
    root: float = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "foo": {
      "$dynamicAnchor": "items",
      "type": "string"
    },
    "list": {
      "$defs": {
        "items": {
          "$comment": "This is only needed to satisfy the bookending requirement",
          "$dynamicAnchor": "items",
          "type": "number"
        }
      },
      "$id": "list",
      "items": {
        "$dynamicRef": "#/$defs/items"
      },
      "type": "array"
    }
  },
  "$id": "https://test.json-schema.org/dynamicRef-without-anchor/root",
  "$ref": "list",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}"""
    root: GeneratedSchemaN0 = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
