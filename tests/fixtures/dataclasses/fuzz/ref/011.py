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
      "minProperties": 2,
      "properties": {
        "meta": {
          "$ref": "#/$defs/n1"
        },
        "nodes": {
          "$ref": "#/$defs/n2"
        }
      },
      "required": [
        "meta",
        "nodes"
      ],
      "type": "object"
    },
    "n1": {
      "minLength": 0,
      "type": "string"
    },
    "n10": {
      "type": "number"
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
      "minProperties": 1,
      "properties": {
        "subtree": {
          "$ref": "#/$defs/n5"
        },
        "value": {
          "$ref": "#/$defs/n10"
        }
      },
      "required": [
        "value"
      ],
      "type": "object"
    },
    "n5": {
      "$ref": "#/$defs/n6"
    },
    "n6": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minProperties": 2,
      "properties": {
        "meta": {
          "$ref": "#/$defs/n7"
        },
        "nodes": {
          "$ref": "#/$defs/n8"
        }
      },
      "required": [
        "meta",
        "nodes"
      ],
      "type": "object"
    },
    "n7": {
      "minLength": 0,
      "type": "string"
    },
    "n8": {
      "items": {
        "$ref": "#/$defs/n9"
      },
      "minItems": 0,
      "type": "array"
    },
    "n9": {
      "$ref": "#/$defs/n4"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "minProperties": 2,
  "properties": {
    "meta": {
      "$ref": "#/$defs/n1"
    },
    "nodes": {
      "$ref": "#/$defs/n2"
    }
  },
  "required": [
    "meta",
    "nodes"
  ],
  "type": "object"
}"""
    meta: str = dc.field("meta")
    nodes: GeneratedSchemaN2 = dc.field("nodes")
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN1(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minProperties": 2,
      "properties": {
        "meta": {
          "$ref": "#/$defs/n1"
        },
        "nodes": {
          "$ref": "#/$defs/n2"
        }
      },
      "required": [
        "meta",
        "nodes"
      ],
      "type": "object"
    },
    "n1": {
      "minLength": 0,
      "type": "string"
    },
    "n10": {
      "type": "number"
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
      "minProperties": 1,
      "properties": {
        "subtree": {
          "$ref": "#/$defs/n5"
        },
        "value": {
          "$ref": "#/$defs/n10"
        }
      },
      "required": [
        "value"
      ],
      "type": "object"
    },
    "n5": {
      "$ref": "#/$defs/n6"
    },
    "n6": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minProperties": 2,
      "properties": {
        "meta": {
          "$ref": "#/$defs/n7"
        },
        "nodes": {
          "$ref": "#/$defs/n8"
        }
      },
      "required": [
        "meta",
        "nodes"
      ],
      "type": "object"
    },
    "n7": {
      "minLength": 0,
      "type": "string"
    },
    "n8": {
      "items": {
        "$ref": "#/$defs/n9"
      },
      "minItems": 0,
      "type": "array"
    },
    "n9": {
      "$ref": "#/$defs/n4"
    }
  },
  "minLength": 0,
  "type": "string"
}"""
    root: str = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN10(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minProperties": 2,
      "properties": {
        "meta": {
          "$ref": "#/$defs/n1"
        },
        "nodes": {
          "$ref": "#/$defs/n2"
        }
      },
      "required": [
        "meta",
        "nodes"
      ],
      "type": "object"
    },
    "n1": {
      "minLength": 0,
      "type": "string"
    },
    "n10": {
      "type": "number"
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
      "minProperties": 1,
      "properties": {
        "subtree": {
          "$ref": "#/$defs/n5"
        },
        "value": {
          "$ref": "#/$defs/n10"
        }
      },
      "required": [
        "value"
      ],
      "type": "object"
    },
    "n5": {
      "$ref": "#/$defs/n6"
    },
    "n6": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minProperties": 2,
      "properties": {
        "meta": {
          "$ref": "#/$defs/n7"
        },
        "nodes": {
          "$ref": "#/$defs/n8"
        }
      },
      "required": [
        "meta",
        "nodes"
      ],
      "type": "object"
    },
    "n7": {
      "minLength": 0,
      "type": "string"
    },
    "n8": {
      "items": {
        "$ref": "#/$defs/n9"
      },
      "minItems": 0,
      "type": "array"
    },
    "n9": {
      "$ref": "#/$defs/n4"
    }
  },
  "type": "number"
}"""
    root: float = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN2(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minProperties": 2,
      "properties": {
        "meta": {
          "$ref": "#/$defs/n1"
        },
        "nodes": {
          "$ref": "#/$defs/n2"
        }
      },
      "required": [
        "meta",
        "nodes"
      ],
      "type": "object"
    },
    "n1": {
      "minLength": 0,
      "type": "string"
    },
    "n10": {
      "type": "number"
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
      "minProperties": 1,
      "properties": {
        "subtree": {
          "$ref": "#/$defs/n5"
        },
        "value": {
          "$ref": "#/$defs/n10"
        }
      },
      "required": [
        "value"
      ],
      "type": "object"
    },
    "n5": {
      "$ref": "#/$defs/n6"
    },
    "n6": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minProperties": 2,
      "properties": {
        "meta": {
          "$ref": "#/$defs/n7"
        },
        "nodes": {
          "$ref": "#/$defs/n8"
        }
      },
      "required": [
        "meta",
        "nodes"
      ],
      "type": "object"
    },
    "n7": {
      "minLength": 0,
      "type": "string"
    },
    "n8": {
      "items": {
        "$ref": "#/$defs/n9"
      },
      "minItems": 0,
      "type": "array"
    },
    "n9": {
      "$ref": "#/$defs/n4"
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
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minProperties": 2,
      "properties": {
        "meta": {
          "$ref": "#/$defs/n1"
        },
        "nodes": {
          "$ref": "#/$defs/n2"
        }
      },
      "required": [
        "meta",
        "nodes"
      ],
      "type": "object"
    },
    "n1": {
      "minLength": 0,
      "type": "string"
    },
    "n10": {
      "type": "number"
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
      "minProperties": 1,
      "properties": {
        "subtree": {
          "$ref": "#/$defs/n5"
        },
        "value": {
          "$ref": "#/$defs/n10"
        }
      },
      "required": [
        "value"
      ],
      "type": "object"
    },
    "n5": {
      "$ref": "#/$defs/n6"
    },
    "n6": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minProperties": 2,
      "properties": {
        "meta": {
          "$ref": "#/$defs/n7"
        },
        "nodes": {
          "$ref": "#/$defs/n8"
        }
      },
      "required": [
        "meta",
        "nodes"
      ],
      "type": "object"
    },
    "n7": {
      "minLength": 0,
      "type": "string"
    },
    "n8": {
      "items": {
        "$ref": "#/$defs/n9"
      },
      "minItems": 0,
      "type": "array"
    },
    "n9": {
      "$ref": "#/$defs/n4"
    }
  },
  "$ref": "#/$defs/n4"
}"""
    root: GeneratedSchemaN4 = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN4(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minProperties": 2,
      "properties": {
        "meta": {
          "$ref": "#/$defs/n1"
        },
        "nodes": {
          "$ref": "#/$defs/n2"
        }
      },
      "required": [
        "meta",
        "nodes"
      ],
      "type": "object"
    },
    "n1": {
      "minLength": 0,
      "type": "string"
    },
    "n10": {
      "type": "number"
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
      "minProperties": 1,
      "properties": {
        "subtree": {
          "$ref": "#/$defs/n5"
        },
        "value": {
          "$ref": "#/$defs/n10"
        }
      },
      "required": [
        "value"
      ],
      "type": "object"
    },
    "n5": {
      "$ref": "#/$defs/n6"
    },
    "n6": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minProperties": 2,
      "properties": {
        "meta": {
          "$ref": "#/$defs/n7"
        },
        "nodes": {
          "$ref": "#/$defs/n8"
        }
      },
      "required": [
        "meta",
        "nodes"
      ],
      "type": "object"
    },
    "n7": {
      "minLength": 0,
      "type": "string"
    },
    "n8": {
      "items": {
        "$ref": "#/$defs/n9"
      },
      "minItems": 0,
      "type": "array"
    },
    "n9": {
      "$ref": "#/$defs/n4"
    }
  },
  "minProperties": 1,
  "properties": {
    "subtree": {
      "$ref": "#/$defs/n5"
    },
    "value": {
      "$ref": "#/$defs/n10"
    }
  },
  "required": [
    "value"
  ],
  "type": "object"
}"""
    subtree: dc.Omittable[GeneratedSchemaN5] = dc.field("subtree", omittable=True)
    value: float = dc.field("value")
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN5(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minProperties": 2,
      "properties": {
        "meta": {
          "$ref": "#/$defs/n1"
        },
        "nodes": {
          "$ref": "#/$defs/n2"
        }
      },
      "required": [
        "meta",
        "nodes"
      ],
      "type": "object"
    },
    "n1": {
      "minLength": 0,
      "type": "string"
    },
    "n10": {
      "type": "number"
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
      "minProperties": 1,
      "properties": {
        "subtree": {
          "$ref": "#/$defs/n5"
        },
        "value": {
          "$ref": "#/$defs/n10"
        }
      },
      "required": [
        "value"
      ],
      "type": "object"
    },
    "n5": {
      "$ref": "#/$defs/n6"
    },
    "n6": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minProperties": 2,
      "properties": {
        "meta": {
          "$ref": "#/$defs/n7"
        },
        "nodes": {
          "$ref": "#/$defs/n8"
        }
      },
      "required": [
        "meta",
        "nodes"
      ],
      "type": "object"
    },
    "n7": {
      "minLength": 0,
      "type": "string"
    },
    "n8": {
      "items": {
        "$ref": "#/$defs/n9"
      },
      "minItems": 0,
      "type": "array"
    },
    "n9": {
      "$ref": "#/$defs/n4"
    }
  },
  "$ref": "#/$defs/n6"
}"""
    root: GeneratedSchemaN6 = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN6(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minProperties": 2,
      "properties": {
        "meta": {
          "$ref": "#/$defs/n1"
        },
        "nodes": {
          "$ref": "#/$defs/n2"
        }
      },
      "required": [
        "meta",
        "nodes"
      ],
      "type": "object"
    },
    "n1": {
      "minLength": 0,
      "type": "string"
    },
    "n10": {
      "type": "number"
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
      "minProperties": 1,
      "properties": {
        "subtree": {
          "$ref": "#/$defs/n5"
        },
        "value": {
          "$ref": "#/$defs/n10"
        }
      },
      "required": [
        "value"
      ],
      "type": "object"
    },
    "n5": {
      "$ref": "#/$defs/n6"
    },
    "n6": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minProperties": 2,
      "properties": {
        "meta": {
          "$ref": "#/$defs/n7"
        },
        "nodes": {
          "$ref": "#/$defs/n8"
        }
      },
      "required": [
        "meta",
        "nodes"
      ],
      "type": "object"
    },
    "n7": {
      "minLength": 0,
      "type": "string"
    },
    "n8": {
      "items": {
        "$ref": "#/$defs/n9"
      },
      "minItems": 0,
      "type": "array"
    },
    "n9": {
      "$ref": "#/$defs/n4"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "minProperties": 2,
  "properties": {
    "meta": {
      "$ref": "#/$defs/n7"
    },
    "nodes": {
      "$ref": "#/$defs/n8"
    }
  },
  "required": [
    "meta",
    "nodes"
  ],
  "type": "object"
}"""
    meta: str = dc.field("meta")
    nodes: GeneratedSchemaN8 = dc.field("nodes")
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN7(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minProperties": 2,
      "properties": {
        "meta": {
          "$ref": "#/$defs/n1"
        },
        "nodes": {
          "$ref": "#/$defs/n2"
        }
      },
      "required": [
        "meta",
        "nodes"
      ],
      "type": "object"
    },
    "n1": {
      "minLength": 0,
      "type": "string"
    },
    "n10": {
      "type": "number"
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
      "minProperties": 1,
      "properties": {
        "subtree": {
          "$ref": "#/$defs/n5"
        },
        "value": {
          "$ref": "#/$defs/n10"
        }
      },
      "required": [
        "value"
      ],
      "type": "object"
    },
    "n5": {
      "$ref": "#/$defs/n6"
    },
    "n6": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minProperties": 2,
      "properties": {
        "meta": {
          "$ref": "#/$defs/n7"
        },
        "nodes": {
          "$ref": "#/$defs/n8"
        }
      },
      "required": [
        "meta",
        "nodes"
      ],
      "type": "object"
    },
    "n7": {
      "minLength": 0,
      "type": "string"
    },
    "n8": {
      "items": {
        "$ref": "#/$defs/n9"
      },
      "minItems": 0,
      "type": "array"
    },
    "n9": {
      "$ref": "#/$defs/n4"
    }
  },
  "minLength": 0,
  "type": "string"
}"""
    root: str = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN8(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minProperties": 2,
      "properties": {
        "meta": {
          "$ref": "#/$defs/n1"
        },
        "nodes": {
          "$ref": "#/$defs/n2"
        }
      },
      "required": [
        "meta",
        "nodes"
      ],
      "type": "object"
    },
    "n1": {
      "minLength": 0,
      "type": "string"
    },
    "n10": {
      "type": "number"
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
      "minProperties": 1,
      "properties": {
        "subtree": {
          "$ref": "#/$defs/n5"
        },
        "value": {
          "$ref": "#/$defs/n10"
        }
      },
      "required": [
        "value"
      ],
      "type": "object"
    },
    "n5": {
      "$ref": "#/$defs/n6"
    },
    "n6": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minProperties": 2,
      "properties": {
        "meta": {
          "$ref": "#/$defs/n7"
        },
        "nodes": {
          "$ref": "#/$defs/n8"
        }
      },
      "required": [
        "meta",
        "nodes"
      ],
      "type": "object"
    },
    "n7": {
      "minLength": 0,
      "type": "string"
    },
    "n8": {
      "items": {
        "$ref": "#/$defs/n9"
      },
      "minItems": 0,
      "type": "array"
    },
    "n9": {
      "$ref": "#/$defs/n4"
    }
  },
  "items": {
    "$ref": "#/$defs/n9"
  },
  "minItems": 0,
  "type": "array"
}"""
    root: collections.abc.Sequence[GeneratedSchemaN9] = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN9(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minProperties": 2,
      "properties": {
        "meta": {
          "$ref": "#/$defs/n1"
        },
        "nodes": {
          "$ref": "#/$defs/n2"
        }
      },
      "required": [
        "meta",
        "nodes"
      ],
      "type": "object"
    },
    "n1": {
      "minLength": 0,
      "type": "string"
    },
    "n10": {
      "type": "number"
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
      "minProperties": 1,
      "properties": {
        "subtree": {
          "$ref": "#/$defs/n5"
        },
        "value": {
          "$ref": "#/$defs/n10"
        }
      },
      "required": [
        "value"
      ],
      "type": "object"
    },
    "n5": {
      "$ref": "#/$defs/n6"
    },
    "n6": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minProperties": 2,
      "properties": {
        "meta": {
          "$ref": "#/$defs/n7"
        },
        "nodes": {
          "$ref": "#/$defs/n8"
        }
      },
      "required": [
        "meta",
        "nodes"
      ],
      "type": "object"
    },
    "n7": {
      "minLength": 0,
      "type": "string"
    },
    "n8": {
      "items": {
        "$ref": "#/$defs/n9"
      },
      "minItems": 0,
      "type": "array"
    },
    "n9": {
      "$ref": "#/$defs/n4"
    }
  },
  "$ref": "#/$defs/n4"
}"""
    root: GeneratedSchemaN4 = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "node": {
      "$id": "http://localhost:1234/draft2020-12/node",
      "description": "node",
      "properties": {
        "subtree": {
          "$ref": "tree"
        },
        "value": {
          "type": "number"
        }
      },
      "required": [
        "value"
      ],
      "type": "object"
    }
  },
  "$id": "http://localhost:1234/draft2020-12/tree",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "tree of nodes",
  "properties": {
    "meta": {
      "type": "string"
    },
    "nodes": {
      "items": {
        "$ref": "node"
      },
      "type": "array"
    }
  },
  "required": [
    "meta",
    "nodes"
  ],
  "type": "object"
}"""
    root: GeneratedSchemaN0 = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
