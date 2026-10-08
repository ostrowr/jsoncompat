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
      "$ref": "#/$defs/n1",
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minProperties": 0,
      "properties": {
        "foo": {
          "$ref": "#/$defs/n7"
        }
      },
      "type": "object"
    },
    "n1": {
      "minProperties": 0,
      "properties": {
        "bar": {
          "$ref": "#/$defs/n2"
        }
      },
      "type": "object"
    },
    "n2": {
      "$ref": "#/$defs/n3"
    },
    "n3": {
      "minProperties": 0,
      "properties": {
        "baz": {
          "$ref": "#/$defs/n4"
        }
      },
      "type": "object"
    },
    "n4": {
      "$ref": "#/$defs/n5"
    },
    "n5": {
      "minProperties": 0,
      "properties": {
        "bar": {
          "$ref": "#/$defs/n6"
        }
      },
      "type": "object"
    },
    "n6": {
      "$ref": "#/$defs/n3"
    },
    "n7": {
      "enum": [
        "pass"
      ]
    }
  },
  "$ref": "#/$defs/n1",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "minProperties": 0,
  "properties": {
    "foo": {
      "$ref": "#/$defs/n7"
    }
  },
  "type": "object"
}"""
    foo: dc.Omittable[typing.Literal["pass"]] = dc.field("foo", omittable=True)
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN1(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$ref": "#/$defs/n1",
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minProperties": 0,
      "properties": {
        "foo": {
          "$ref": "#/$defs/n7"
        }
      },
      "type": "object"
    },
    "n1": {
      "minProperties": 0,
      "properties": {
        "bar": {
          "$ref": "#/$defs/n2"
        }
      },
      "type": "object"
    },
    "n2": {
      "$ref": "#/$defs/n3"
    },
    "n3": {
      "minProperties": 0,
      "properties": {
        "baz": {
          "$ref": "#/$defs/n4"
        }
      },
      "type": "object"
    },
    "n4": {
      "$ref": "#/$defs/n5"
    },
    "n5": {
      "minProperties": 0,
      "properties": {
        "bar": {
          "$ref": "#/$defs/n6"
        }
      },
      "type": "object"
    },
    "n6": {
      "$ref": "#/$defs/n3"
    },
    "n7": {
      "enum": [
        "pass"
      ]
    }
  },
  "minProperties": 0,
  "properties": {
    "bar": {
      "$ref": "#/$defs/n2"
    }
  },
  "type": "object"
}"""
    bar: dc.Omittable[GeneratedSchemaN2] = dc.field("bar", omittable=True)
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN2(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$ref": "#/$defs/n1",
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minProperties": 0,
      "properties": {
        "foo": {
          "$ref": "#/$defs/n7"
        }
      },
      "type": "object"
    },
    "n1": {
      "minProperties": 0,
      "properties": {
        "bar": {
          "$ref": "#/$defs/n2"
        }
      },
      "type": "object"
    },
    "n2": {
      "$ref": "#/$defs/n3"
    },
    "n3": {
      "minProperties": 0,
      "properties": {
        "baz": {
          "$ref": "#/$defs/n4"
        }
      },
      "type": "object"
    },
    "n4": {
      "$ref": "#/$defs/n5"
    },
    "n5": {
      "minProperties": 0,
      "properties": {
        "bar": {
          "$ref": "#/$defs/n6"
        }
      },
      "type": "object"
    },
    "n6": {
      "$ref": "#/$defs/n3"
    },
    "n7": {
      "enum": [
        "pass"
      ]
    }
  },
  "$ref": "#/$defs/n3"
}"""
    root: GeneratedSchemaN3 = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN3(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$ref": "#/$defs/n1",
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minProperties": 0,
      "properties": {
        "foo": {
          "$ref": "#/$defs/n7"
        }
      },
      "type": "object"
    },
    "n1": {
      "minProperties": 0,
      "properties": {
        "bar": {
          "$ref": "#/$defs/n2"
        }
      },
      "type": "object"
    },
    "n2": {
      "$ref": "#/$defs/n3"
    },
    "n3": {
      "minProperties": 0,
      "properties": {
        "baz": {
          "$ref": "#/$defs/n4"
        }
      },
      "type": "object"
    },
    "n4": {
      "$ref": "#/$defs/n5"
    },
    "n5": {
      "minProperties": 0,
      "properties": {
        "bar": {
          "$ref": "#/$defs/n6"
        }
      },
      "type": "object"
    },
    "n6": {
      "$ref": "#/$defs/n3"
    },
    "n7": {
      "enum": [
        "pass"
      ]
    }
  },
  "minProperties": 0,
  "properties": {
    "baz": {
      "$ref": "#/$defs/n4"
    }
  },
  "type": "object"
}"""
    baz: dc.Omittable[GeneratedSchemaN4] = dc.field("baz", omittable=True)
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN4(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$ref": "#/$defs/n1",
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minProperties": 0,
      "properties": {
        "foo": {
          "$ref": "#/$defs/n7"
        }
      },
      "type": "object"
    },
    "n1": {
      "minProperties": 0,
      "properties": {
        "bar": {
          "$ref": "#/$defs/n2"
        }
      },
      "type": "object"
    },
    "n2": {
      "$ref": "#/$defs/n3"
    },
    "n3": {
      "minProperties": 0,
      "properties": {
        "baz": {
          "$ref": "#/$defs/n4"
        }
      },
      "type": "object"
    },
    "n4": {
      "$ref": "#/$defs/n5"
    },
    "n5": {
      "minProperties": 0,
      "properties": {
        "bar": {
          "$ref": "#/$defs/n6"
        }
      },
      "type": "object"
    },
    "n6": {
      "$ref": "#/$defs/n3"
    },
    "n7": {
      "enum": [
        "pass"
      ]
    }
  },
  "$ref": "#/$defs/n5"
}"""
    root: GeneratedSchemaN5 = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN5(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$ref": "#/$defs/n1",
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minProperties": 0,
      "properties": {
        "foo": {
          "$ref": "#/$defs/n7"
        }
      },
      "type": "object"
    },
    "n1": {
      "minProperties": 0,
      "properties": {
        "bar": {
          "$ref": "#/$defs/n2"
        }
      },
      "type": "object"
    },
    "n2": {
      "$ref": "#/$defs/n3"
    },
    "n3": {
      "minProperties": 0,
      "properties": {
        "baz": {
          "$ref": "#/$defs/n4"
        }
      },
      "type": "object"
    },
    "n4": {
      "$ref": "#/$defs/n5"
    },
    "n5": {
      "minProperties": 0,
      "properties": {
        "bar": {
          "$ref": "#/$defs/n6"
        }
      },
      "type": "object"
    },
    "n6": {
      "$ref": "#/$defs/n3"
    },
    "n7": {
      "enum": [
        "pass"
      ]
    }
  },
  "minProperties": 0,
  "properties": {
    "bar": {
      "$ref": "#/$defs/n6"
    }
  },
  "type": "object"
}"""
    bar: dc.Omittable[GeneratedSchemaN6] = dc.field("bar", omittable=True)
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN6(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$ref": "#/$defs/n1",
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minProperties": 0,
      "properties": {
        "foo": {
          "$ref": "#/$defs/n7"
        }
      },
      "type": "object"
    },
    "n1": {
      "minProperties": 0,
      "properties": {
        "bar": {
          "$ref": "#/$defs/n2"
        }
      },
      "type": "object"
    },
    "n2": {
      "$ref": "#/$defs/n3"
    },
    "n3": {
      "minProperties": 0,
      "properties": {
        "baz": {
          "$ref": "#/$defs/n4"
        }
      },
      "type": "object"
    },
    "n4": {
      "$ref": "#/$defs/n5"
    },
    "n5": {
      "minProperties": 0,
      "properties": {
        "bar": {
          "$ref": "#/$defs/n6"
        }
      },
      "type": "object"
    },
    "n6": {
      "$ref": "#/$defs/n3"
    },
    "n7": {
      "enum": [
        "pass"
      ]
    }
  },
  "$ref": "#/$defs/n3"
}"""
    root: GeneratedSchemaN3 = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN7(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$ref": "#/$defs/n1",
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "minProperties": 0,
      "properties": {
        "foo": {
          "$ref": "#/$defs/n7"
        }
      },
      "type": "object"
    },
    "n1": {
      "minProperties": 0,
      "properties": {
        "bar": {
          "$ref": "#/$defs/n2"
        }
      },
      "type": "object"
    },
    "n2": {
      "$ref": "#/$defs/n3"
    },
    "n3": {
      "minProperties": 0,
      "properties": {
        "baz": {
          "$ref": "#/$defs/n4"
        }
      },
      "type": "object"
    },
    "n4": {
      "$ref": "#/$defs/n5"
    },
    "n5": {
      "minProperties": 0,
      "properties": {
        "bar": {
          "$ref": "#/$defs/n6"
        }
      },
      "type": "object"
    },
    "n6": {
      "$ref": "#/$defs/n3"
    },
    "n7": {
      "enum": [
        "pass"
      ]
    }
  },
  "enum": [
    "pass"
  ]
}"""
    root: typing.Literal["pass"] = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "bar": {
      "$id": "bar",
      "properties": {
        "baz": {
          "$dynamicRef": "extended#meta"
        }
      },
      "type": "object"
    },
    "extended": {
      "$anchor": "meta",
      "$id": "extended",
      "properties": {
        "bar": {
          "$ref": "bar"
        }
      },
      "type": "object"
    }
  },
  "$dynamicAnchor": "meta",
  "$id": "https://test.json-schema.org/relative-dynamic-reference-without-bookend/root",
  "$ref": "extended",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo": {
      "const": "pass"
    }
  },
  "type": "object"
}"""
    root: GeneratedSchemaN0 = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
