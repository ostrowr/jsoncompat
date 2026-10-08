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
      "$ref": "#/$defs/n2",
      "minProperties": 0,
      "properties": {
        "foo": {
          "$ref": "#/$defs/n5"
        }
      },
      "type": "object",
      "unevaluatedProperties": {
        "$ref": "#/$defs/n4"
      }
    },
    "n2": {
      "properties": {
        "bar": {
          "$ref": "#/$defs/n3"
        }
      }
    },
    "n3": {
      "minLength": 0,
      "type": "string"
    },
    "n4": false,
    "n5": {
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
class GeneratedSchemaN1(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$ref": "#/$defs/n1",
      "$schema": "https://json-schema.org/draft/2020-12/schema"
    },
    "n1": {
      "$ref": "#/$defs/n2",
      "minProperties": 0,
      "properties": {
        "foo": {
          "$ref": "#/$defs/n5"
        }
      },
      "type": "object",
      "unevaluatedProperties": {
        "$ref": "#/$defs/n4"
      }
    },
    "n2": {
      "properties": {
        "bar": {
          "$ref": "#/$defs/n3"
        }
      }
    },
    "n3": {
      "minLength": 0,
      "type": "string"
    },
    "n4": false,
    "n5": {
      "minLength": 0,
      "type": "string"
    }
  },
  "$ref": "#/$defs/n2",
  "minProperties": 0,
  "properties": {
    "foo": {
      "$ref": "#/$defs/n5"
    }
  },
  "type": "object",
  "unevaluatedProperties": {
    "$ref": "#/$defs/n4"
  }
}"""
    foo: dc.Omittable[str] = dc.field("foo", omittable=True)
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

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
      "minProperties": 0,
      "properties": {
        "foo": {
          "$ref": "#/$defs/n5"
        }
      },
      "type": "object",
      "unevaluatedProperties": {
        "$ref": "#/$defs/n4"
      }
    },
    "n2": {
      "properties": {
        "bar": {
          "$ref": "#/$defs/n3"
        }
      }
    },
    "n3": {
      "minLength": 0,
      "type": "string"
    },
    "n4": false,
    "n5": {
      "minLength": 0,
      "type": "string"
    }
  },
  "properties": {
    "bar": {
      "$ref": "#/$defs/n3"
    }
  }
}"""
    root: typing.Any = dc.root_field()

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
      "$ref": "#/$defs/n2",
      "minProperties": 0,
      "properties": {
        "foo": {
          "$ref": "#/$defs/n5"
        }
      },
      "type": "object",
      "unevaluatedProperties": {
        "$ref": "#/$defs/n4"
      }
    },
    "n2": {
      "properties": {
        "bar": {
          "$ref": "#/$defs/n3"
        }
      }
    },
    "n3": {
      "minLength": 0,
      "type": "string"
    },
    "n4": false,
    "n5": {
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
class GeneratedSchemaN4(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """false"""
    root: typing.Any = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN5(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$ref": "#/$defs/n1",
      "$schema": "https://json-schema.org/draft/2020-12/schema"
    },
    "n1": {
      "$ref": "#/$defs/n2",
      "minProperties": 0,
      "properties": {
        "foo": {
          "$ref": "#/$defs/n5"
        }
      },
      "type": "object",
      "unevaluatedProperties": {
        "$ref": "#/$defs/n4"
      }
    },
    "n2": {
      "properties": {
        "bar": {
          "$ref": "#/$defs/n3"
        }
      }
    },
    "n3": {
      "minLength": 0,
      "type": "string"
    },
    "n4": false,
    "n5": {
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
      "$comment": "unevaluatedProperties comes first so it's more likely to catch bugs with implementations that are sensitive to keyword ordering",
      "$defs": {
        "defaultAddons": {
          "$comment": "Needed to satisfy the bookending requirement",
          "$dynamicAnchor": "addons"
        }
      },
      "$dynamicRef": "#addons",
      "$id": "./baseSchema",
      "properties": {
        "foo": {
          "type": "string"
        }
      },
      "type": "object",
      "unevaluatedProperties": false
    },
    "derived": {
      "$dynamicAnchor": "addons",
      "properties": {
        "bar": {
          "type": "string"
        }
      }
    }
  },
  "$id": "https://example.com/unevaluated-properties-with-dynamic-ref/derived",
  "$ref": "./baseSchema",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}"""
    root: GeneratedSchemaN0 = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
