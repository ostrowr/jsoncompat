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
      "$ref": "#/$defs/n2"
    },
    "n2": {
      "items": {
        "$ref": "#/$defs/n3"
      },
      "minItems": 0,
      "type": "array"
    },
    "n3": {
      "$ref": "#/$defs/n4"
    },
    "n4": {
      "minLength": 0,
      "type": "string"
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
      "$ref": "#/$defs/n2"
    },
    "n2": {
      "items": {
        "$ref": "#/$defs/n3"
      },
      "minItems": 0,
      "type": "array"
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
class GeneratedSchemaN2(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$ref": "#/$defs/n1",
      "$schema": "https://json-schema.org/draft/2020-12/schema"
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n2": {
      "items": {
        "$ref": "#/$defs/n3"
      },
      "minItems": 0,
      "type": "array"
    },
    "n3": {
      "$ref": "#/$defs/n4"
    },
    "n4": {
      "minLength": 0,
      "type": "string"
    }
  },
  "items": {
    "$ref": "#/$defs/n3"
  },
  "minItems": 0,
  "type": "array"
}"""
    root: collections.abc.Sequence[GeneratedSchemaN3] = dc.root_field()

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
      "$ref": "#/$defs/n2"
    },
    "n2": {
      "items": {
        "$ref": "#/$defs/n3"
      },
      "minItems": 0,
      "type": "array"
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
      "$ref": "#/$defs/n1",
      "$schema": "https://json-schema.org/draft/2020-12/schema"
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n2": {
      "items": {
        "$ref": "#/$defs/n3"
      },
      "minItems": 0,
      "type": "array"
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
    "foo": {
      "$dynamicAnchor": "items",
      "type": "string"
    },
    "intermediate-scope": {
      "$id": "intermediate-scope",
      "$ref": "list"
    },
    "list": {
      "$defs": {
        "items": {
          "$comment": "This is only needed to satisfy the bookending requirement",
          "$dynamicAnchor": "items"
        }
      },
      "$id": "list",
      "items": {
        "$dynamicRef": "#items"
      },
      "type": "array"
    }
  },
  "$id": "https://test.json-schema.org/dynamic-resolution-with-intermediate-scopes/root",
  "$ref": "intermediate-scope",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}"""
    root: GeneratedSchemaN0 = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
