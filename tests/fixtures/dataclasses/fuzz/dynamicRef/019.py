from __future__ import annotations

import collections.abc
from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN0(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minProperties": 0,
      "properties": {
        "bar-item": {
          "$ref": "#/$defs/n1"
        }
      },
      "type": "object"
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n2": {
      "minProperties": 0,
      "properties": {
        "content": {
          "$ref": "#/$defs/n3"
        }
      },
      "type": "object"
    },
    "n3": {
      "$ref": "#/$defs/n4"
    },
    "n4": {
      "multipleOf": 1,
      "type": "integer"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "minProperties": 0,
  "properties": {
    "bar-item": {
      "$ref": "#/$defs/n1"
    }
  },
  "type": "object"
}"""
    bar_item: dc.Omittable[GeneratedSchemaN1] = dc.field("bar-item", omittable=True)
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN1(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minProperties": 0,
      "properties": {
        "bar-item": {
          "$ref": "#/$defs/n1"
        }
      },
      "type": "object"
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n2": {
      "minProperties": 0,
      "properties": {
        "content": {
          "$ref": "#/$defs/n3"
        }
      },
      "type": "object"
    },
    "n3": {
      "$ref": "#/$defs/n4"
    },
    "n4": {
      "multipleOf": 1,
      "type": "integer"
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
      "minProperties": 0,
      "properties": {
        "bar-item": {
          "$ref": "#/$defs/n1"
        }
      },
      "type": "object"
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n2": {
      "minProperties": 0,
      "properties": {
        "content": {
          "$ref": "#/$defs/n3"
        }
      },
      "type": "object"
    },
    "n3": {
      "$ref": "#/$defs/n4"
    },
    "n4": {
      "multipleOf": 1,
      "type": "integer"
    }
  },
  "minProperties": 0,
  "properties": {
    "content": {
      "$ref": "#/$defs/n3"
    }
  },
  "type": "object"
}"""
    content: dc.Omittable[GeneratedSchemaN3] = dc.field("content", omittable=True)
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN3(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minProperties": 0,
      "properties": {
        "bar-item": {
          "$ref": "#/$defs/n1"
        }
      },
      "type": "object"
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n2": {
      "minProperties": 0,
      "properties": {
        "content": {
          "$ref": "#/$defs/n3"
        }
      },
      "type": "object"
    },
    "n3": {
      "$ref": "#/$defs/n4"
    },
    "n4": {
      "multipleOf": 1,
      "type": "integer"
    }
  },
  "$ref": "#/$defs/n4"
}"""
    root: int = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN4(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minProperties": 0,
      "properties": {
        "bar-item": {
          "$ref": "#/$defs/n1"
        }
      },
      "type": "object"
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n2": {
      "minProperties": 0,
      "properties": {
        "content": {
          "$ref": "#/$defs/n3"
        }
      },
      "type": "object"
    },
    "n3": {
      "$ref": "#/$defs/n4"
    },
    "n4": {
      "multipleOf": 1,
      "type": "integer"
    }
  },
  "multipleOf": 1,
  "type": "integer"
}"""
    root: int = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "bar": {
      "$defs": {
        "content": {
          "$dynamicAnchor": "content",
          "type": "string"
        },
        "item": {
          "$defs": {
            "defaultContent": {
              "$dynamicAnchor": "content",
              "type": "integer"
            }
          },
          "$id": "item",
          "properties": {
            "content": {
              "$dynamicRef": "#content"
            }
          },
          "type": "object"
        }
      },
      "$id": "bar",
      "items": {
        "$ref": "item"
      },
      "type": "array"
    }
  },
  "$id": "https://test.json-schema.org/dynamic-ref-skips-intermediate-resource/main",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "bar-item": {
      "$ref": "item"
    }
  },
  "type": "object"
}"""
    root: GeneratedSchemaN0 = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
