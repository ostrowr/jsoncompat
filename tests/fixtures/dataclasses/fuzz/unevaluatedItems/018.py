from __future__ import annotations

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
      "$ref": "#/$defs/n2",
      "minItems": 0,
      "prefixItems": [
        {
          "$ref": "#/$defs/n6"
        }
      ],
      "type": "array",
      "unevaluatedItems": {
        "$ref": "#/$defs/n5"
      }
    },
    "n2": {
      "prefixItems": [
        {
          "$ref": "#/$defs/n3"
        },
        {
          "$ref": "#/$defs/n4"
        }
      ]
    },
    "n3": true,
    "n4": {
      "minLength": 0,
      "type": "string"
    },
    "n5": false,
    "n6": {
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
      "$ref": "#/$defs/n2",
      "minItems": 0,
      "prefixItems": [
        {
          "$ref": "#/$defs/n6"
        }
      ],
      "type": "array",
      "unevaluatedItems": {
        "$ref": "#/$defs/n5"
      }
    },
    "n2": {
      "prefixItems": [
        {
          "$ref": "#/$defs/n3"
        },
        {
          "$ref": "#/$defs/n4"
        }
      ]
    },
    "n3": true,
    "n4": {
      "minLength": 0,
      "type": "string"
    },
    "n5": false,
    "n6": {
      "minLength": 0,
      "type": "string"
    }
  },
  "$ref": "#/$defs/n2",
  "minItems": 0,
  "prefixItems": [
    {
      "$ref": "#/$defs/n6"
    }
  ],
  "type": "array",
  "unevaluatedItems": {
    "$ref": "#/$defs/n5"
  }
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
      "$ref": "#/$defs/n2",
      "minItems": 0,
      "prefixItems": [
        {
          "$ref": "#/$defs/n6"
        }
      ],
      "type": "array",
      "unevaluatedItems": {
        "$ref": "#/$defs/n5"
      }
    },
    "n2": {
      "prefixItems": [
        {
          "$ref": "#/$defs/n3"
        },
        {
          "$ref": "#/$defs/n4"
        }
      ]
    },
    "n3": true,
    "n4": {
      "minLength": 0,
      "type": "string"
    },
    "n5": false,
    "n6": {
      "minLength": 0,
      "type": "string"
    }
  },
  "prefixItems": [
    {
      "$ref": "#/$defs/n3"
    },
    {
      "$ref": "#/$defs/n4"
    }
  ]
}"""
    root: typing.Any = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN3(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

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
      "$ref": "#/$defs/n2",
      "minItems": 0,
      "prefixItems": [
        {
          "$ref": "#/$defs/n6"
        }
      ],
      "type": "array",
      "unevaluatedItems": {
        "$ref": "#/$defs/n5"
      }
    },
    "n2": {
      "prefixItems": [
        {
          "$ref": "#/$defs/n3"
        },
        {
          "$ref": "#/$defs/n4"
        }
      ]
    },
    "n3": true,
    "n4": {
      "minLength": 0,
      "type": "string"
    },
    "n5": false,
    "n6": {
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
class GeneratedSchemaN5(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """false"""
    root: typing.Any = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN6(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$ref": "#/$defs/n1",
      "$schema": "https://json-schema.org/draft/2020-12/schema"
    },
    "n1": {
      "$ref": "#/$defs/n2",
      "minItems": 0,
      "prefixItems": [
        {
          "$ref": "#/$defs/n6"
        }
      ],
      "type": "array",
      "unevaluatedItems": {
        "$ref": "#/$defs/n5"
      }
    },
    "n2": {
      "prefixItems": [
        {
          "$ref": "#/$defs/n3"
        },
        {
          "$ref": "#/$defs/n4"
        }
      ]
    },
    "n3": true,
    "n4": {
      "minLength": 0,
      "type": "string"
    },
    "n5": false,
    "n6": {
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
    "baseSchema": {
      "$comment": "unevaluatedItems comes first so it's more likely to catch bugs with implementations that are sensitive to keyword ordering",
      "$defs": {
        "defaultAddons": {
          "$comment": "Needed to satisfy the bookending requirement",
          "$dynamicAnchor": "addons"
        }
      },
      "$dynamicRef": "#addons",
      "$id": "./baseSchema",
      "prefixItems": [
        {
          "type": "string"
        }
      ],
      "type": "array",
      "unevaluatedItems": false
    },
    "derived": {
      "$dynamicAnchor": "addons",
      "prefixItems": [
        true,
        {
          "type": "string"
        }
      ]
    }
  },
  "$id": "https://example.com/unevaluated-items-with-dynamic-ref/derived",
  "$ref": "./baseSchema",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}"""
    root: GeneratedSchemaN0 = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
