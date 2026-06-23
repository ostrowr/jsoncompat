from __future__ import annotations

import collections.abc
from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaOneBranch2B(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaOneBranch2(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "one": {
      "oneOf": [
        {
          "$ref": "#/$defs/two"
        },
        {
          "properties": {
            "b": true
          },
          "required": [
            "b"
          ]
        },
        {
          "patternProperties": {
            "x": true
          },
          "required": [
            "xx"
          ]
        },
        {
          "required": [
            "all"
          ],
          "unevaluatedProperties": true
        }
      ]
    },
    "two": {
      "oneOf": [
        {
          "properties": {
            "c": true
          },
          "required": [
            "c"
          ]
        },
        {
          "properties": {
            "d": true
          },
          "required": [
            "d"
          ]
        }
      ]
    }
  },
  "minProperties": 1,
  "properties": {
    "b": true
  },
  "required": [
    "b"
  ],
  "type": "object"
}"""
    b: GeneratedSchemaOneBranch2B = dc.field("b")
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaOneItem(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaOneBranch22Xx(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaOneBranch22(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "one": {
      "oneOf": [
        {
          "$ref": "#/$defs/two"
        },
        {
          "properties": {
            "b": true
          },
          "required": [
            "b"
          ]
        },
        {
          "patternProperties": {
            "x": true
          },
          "required": [
            "xx"
          ]
        },
        {
          "required": [
            "all"
          ],
          "unevaluatedProperties": true
        }
      ]
    },
    "two": {
      "oneOf": [
        {
          "properties": {
            "c": true
          },
          "required": [
            "c"
          ]
        },
        {
          "properties": {
            "d": true
          },
          "required": [
            "d"
          ]
        }
      ]
    }
  },
  "minProperties": 1,
  "patternProperties": {
    "x": true
  },
  "properties": {
    "xx": true
  },
  "required": [
    "xx"
  ],
  "type": "object"
}"""
    xx: GeneratedSchemaOneBranch22Xx = dc.field("xx")
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaOneItem2(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaOneBranch23All(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaOneBranch23(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "one": {
      "oneOf": [
        {
          "$ref": "#/$defs/two"
        },
        {
          "properties": {
            "b": true
          },
          "required": [
            "b"
          ]
        },
        {
          "patternProperties": {
            "x": true
          },
          "required": [
            "xx"
          ]
        },
        {
          "required": [
            "all"
          ],
          "unevaluatedProperties": true
        }
      ]
    },
    "two": {
      "oneOf": [
        {
          "properties": {
            "c": true
          },
          "required": [
            "c"
          ]
        },
        {
          "properties": {
            "d": true
          },
          "required": [
            "d"
          ]
        }
      ]
    }
  },
  "minProperties": 1,
  "properties": {
    "all": true
  },
  "required": [
    "all"
  ],
  "type": "object",
  "unevaluatedProperties": true
}"""
    all: GeneratedSchemaOneBranch23All = dc.field("all")
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaOneItem3(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaOne(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "one": {
      "oneOf": [
        {
          "$ref": "#/$defs/two"
        },
        {
          "properties": {
            "b": true
          },
          "required": [
            "b"
          ]
        },
        {
          "patternProperties": {
            "x": true
          },
          "required": [
            "xx"
          ]
        },
        {
          "required": [
            "all"
          ],
          "unevaluatedProperties": true
        }
      ]
    },
    "two": {
      "oneOf": [
        {
          "properties": {
            "c": true
          },
          "required": [
            "c"
          ]
        },
        {
          "properties": {
            "d": true
          },
          "required": [
            "d"
          ]
        }
      ]
    }
  },
  "oneOf": [
    {
      "$ref": "#/$defs/two"
    },
    {
      "properties": {
        "b": true
      },
      "required": [
        "b"
      ]
    },
    {
      "patternProperties": {
        "x": true
      },
      "required": [
        "xx"
      ]
    },
    {
      "required": [
        "all"
      ],
      "unevaluatedProperties": true
    }
  ]
}"""
    root: (((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaOneBranch2 | collections.abc.Sequence[GeneratedSchemaOneItem] | float | str | None) | ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaOneBranch22 | collections.abc.Sequence[GeneratedSchemaOneItem2] | float | str | None) | ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaOneBranch23 | collections.abc.Sequence[GeneratedSchemaOneItem3] | float | str | None) | GeneratedSchemaTwo) = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaTwoBranch2C(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaTwoBranch2(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "one": {
      "oneOf": [
        {
          "$ref": "#/$defs/two"
        },
        {
          "properties": {
            "b": true
          },
          "required": [
            "b"
          ]
        },
        {
          "patternProperties": {
            "x": true
          },
          "required": [
            "xx"
          ]
        },
        {
          "required": [
            "all"
          ],
          "unevaluatedProperties": true
        }
      ]
    },
    "two": {
      "oneOf": [
        {
          "properties": {
            "c": true
          },
          "required": [
            "c"
          ]
        },
        {
          "properties": {
            "d": true
          },
          "required": [
            "d"
          ]
        }
      ]
    }
  },
  "minProperties": 1,
  "properties": {
    "c": true
  },
  "required": [
    "c"
  ],
  "type": "object"
}"""
    c: GeneratedSchemaTwoBranch2C = dc.field("c")
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaTwoItem(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaTwoBranch22D(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaTwoBranch22(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "one": {
      "oneOf": [
        {
          "$ref": "#/$defs/two"
        },
        {
          "properties": {
            "b": true
          },
          "required": [
            "b"
          ]
        },
        {
          "patternProperties": {
            "x": true
          },
          "required": [
            "xx"
          ]
        },
        {
          "required": [
            "all"
          ],
          "unevaluatedProperties": true
        }
      ]
    },
    "two": {
      "oneOf": [
        {
          "properties": {
            "c": true
          },
          "required": [
            "c"
          ]
        },
        {
          "properties": {
            "d": true
          },
          "required": [
            "d"
          ]
        }
      ]
    }
  },
  "minProperties": 1,
  "properties": {
    "d": true
  },
  "required": [
    "d"
  ],
  "type": "object"
}"""
    d: GeneratedSchemaTwoBranch22D = dc.field("d")
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaTwoItem2(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaTwo(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "one": {
      "oneOf": [
        {
          "$ref": "#/$defs/two"
        },
        {
          "properties": {
            "b": true
          },
          "required": [
            "b"
          ]
        },
        {
          "patternProperties": {
            "x": true
          },
          "required": [
            "xx"
          ]
        },
        {
          "required": [
            "all"
          ],
          "unevaluatedProperties": true
        }
      ]
    },
    "two": {
      "oneOf": [
        {
          "properties": {
            "c": true
          },
          "required": [
            "c"
          ]
        },
        {
          "properties": {
            "d": true
          },
          "required": [
            "d"
          ]
        }
      ]
    }
  },
  "oneOf": [
    {
      "properties": {
        "c": true
      },
      "required": [
        "c"
      ]
    },
    {
      "properties": {
        "d": true
      },
      "required": [
        "d"
      ]
    }
  ]
}"""
    root: (((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaTwoBranch2 | collections.abc.Sequence[GeneratedSchemaTwoItem] | float | str | None) | ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaTwoBranch22 | collections.abc.Sequence[GeneratedSchemaTwoItem2] | float | str | None)) = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2A(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBranch2(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "one": {
      "oneOf": [
        {
          "$ref": "#/$defs/two"
        },
        {
          "properties": {
            "b": true
          },
          "required": [
            "b"
          ]
        },
        {
          "patternProperties": {
            "x": true
          },
          "required": [
            "xx"
          ]
        },
        {
          "required": [
            "all"
          ],
          "unevaluatedProperties": true
        }
      ]
    },
    "two": {
      "oneOf": [
        {
          "properties": {
            "c": true
          },
          "required": [
            "c"
          ]
        },
        {
          "properties": {
            "d": true
          },
          "required": [
            "d"
          ]
        }
      ]
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "minProperties": 1,
  "properties": {
    "a": true
  },
  "required": [
    "a"
  ],
  "type": "object",
  "unevaluatedProperties": false
}"""
    a: GeneratedSchemaBranch2A = dc.field("a")
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaItem(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """true"""
    root: typing.Any = dc.root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "one": {
      "oneOf": [
        {
          "$ref": "#/$defs/two"
        },
        {
          "properties": {
            "b": true
          },
          "required": [
            "b"
          ]
        },
        {
          "patternProperties": {
            "x": true
          },
          "required": [
            "xx"
          ]
        },
        {
          "required": [
            "all"
          ],
          "unevaluatedProperties": true
        }
      ]
    },
    "two": {
      "oneOf": [
        {
          "properties": {
            "c": true
          },
          "required": [
            "c"
          ]
        },
        {
          "properties": {
            "d": true
          },
          "required": [
            "d"
          ]
        }
      ]
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "oneOf": [
    {
      "$ref": "#/$defs/one"
    },
    {
      "properties": {
        "a": true
      },
      "required": [
        "a"
      ]
    }
  ],
  "unevaluatedProperties": false
}"""
    root: (((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | collections.abc.Sequence[GeneratedSchemaItem] | float | str | None) | GeneratedSchemaOne) = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema

dc.bind_generated_models((
    (GeneratedSchemaOneBranch2B, "root", typing.Any),
    (
        GeneratedSchemaOneBranch2,
        "object",
        (
            ("b", "b", GeneratedSchemaOneBranch2B, False),
        ),
        True,
        typing.Any,
    ),
    (GeneratedSchemaOneItem, "root", typing.Any),
    (GeneratedSchemaOneBranch22Xx, "root", typing.Any),
    (
        GeneratedSchemaOneBranch22,
        "object",
        (
            ("xx", "xx", GeneratedSchemaOneBranch22Xx, False),
        ),
        True,
        typing.Any,
    ),
    (GeneratedSchemaOneItem2, "root", typing.Any),
    (GeneratedSchemaOneBranch23All, "root", typing.Any),
    (
        GeneratedSchemaOneBranch23,
        "object",
        (
            ("all", "all", GeneratedSchemaOneBranch23All, False),
        ),
        True,
        typing.Any,
    ),
    (GeneratedSchemaOneItem3, "root", typing.Any),
    (GeneratedSchemaOne, "root", (((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaOneBranch2 | collections.abc.Sequence[GeneratedSchemaOneItem] | float | str | None) | ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaOneBranch22 | collections.abc.Sequence[GeneratedSchemaOneItem2] | float | str | None) | ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaOneBranch23 | collections.abc.Sequence[GeneratedSchemaOneItem3] | float | str | None) | GeneratedSchemaTwo)),
    (GeneratedSchemaTwoBranch2C, "root", typing.Any),
    (
        GeneratedSchemaTwoBranch2,
        "object",
        (
            ("c", "c", GeneratedSchemaTwoBranch2C, False),
        ),
        True,
        typing.Any,
    ),
    (GeneratedSchemaTwoItem, "root", typing.Any),
    (GeneratedSchemaTwoBranch22D, "root", typing.Any),
    (
        GeneratedSchemaTwoBranch22,
        "object",
        (
            ("d", "d", GeneratedSchemaTwoBranch22D, False),
        ),
        True,
        typing.Any,
    ),
    (GeneratedSchemaTwoItem2, "root", typing.Any),
    (GeneratedSchemaTwo, "root", (((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaTwoBranch2 | collections.abc.Sequence[GeneratedSchemaTwoItem] | float | str | None) | ((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaTwoBranch22 | collections.abc.Sequence[GeneratedSchemaTwoItem2] | float | str | None))),
    (GeneratedSchemaBranch2A, "root", typing.Any),
    (
        GeneratedSchemaBranch2,
        "object",
        (
            ("a", "a", GeneratedSchemaBranch2A, False),
        ),
        True,
        typing.Any,
    ),
    (GeneratedSchemaItem, "root", typing.Any),
    (GeneratedSchema, "root", (((typing.Literal[False] | typing.Literal[True]) | GeneratedSchemaBranch2 | collections.abc.Sequence[GeneratedSchemaItem] | float | str | None) | GeneratedSchemaOne)),
))
