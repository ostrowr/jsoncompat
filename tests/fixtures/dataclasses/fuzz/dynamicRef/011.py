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
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "else": {
        "$ref": "#/$defs/n1"
      },
      "if": {
        "$ref": "#/$defs/n7"
      },
      "then": {
        "$ref": "#/$defs/n9"
      }
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n10": {
      "$ref": "#/$defs/n11"
    },
    "n11": {
      "properties": {
        "list": {
          "$ref": "#/$defs/n12"
        }
      }
    },
    "n12": {
      "items": {
        "$ref": "#/$defs/n13"
      }
    },
    "n13": {
      "$ref": "#/$defs/n14"
    },
    "n14": {
      "type": "number"
    },
    "n2": {
      "$ref": "#/$defs/n3"
    },
    "n3": {
      "properties": {
        "list": {
          "$ref": "#/$defs/n4"
        }
      }
    },
    "n4": {
      "items": {
        "$ref": "#/$defs/n5"
      }
    },
    "n5": {
      "$ref": "#/$defs/n6"
    },
    "n6": {
      "minLength": 0,
      "type": "string"
    },
    "n7": {
      "properties": {
        "kindOfList": {
          "$ref": "#/$defs/n8"
        }
      },
      "required": [
        "kindOfList"
      ]
    },
    "n8": {
      "enum": [
        "numbers"
      ]
    },
    "n9": {
      "$ref": "#/$defs/n10"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "else": {
    "$ref": "#/$defs/n1"
  },
  "if": {
    "$ref": "#/$defs/n7"
  },
  "then": {
    "$ref": "#/$defs/n9"
  }
}"""
    root: typing.Any = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN1(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "else": {
        "$ref": "#/$defs/n1"
      },
      "if": {
        "$ref": "#/$defs/n7"
      },
      "then": {
        "$ref": "#/$defs/n9"
      }
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n10": {
      "$ref": "#/$defs/n11"
    },
    "n11": {
      "properties": {
        "list": {
          "$ref": "#/$defs/n12"
        }
      }
    },
    "n12": {
      "items": {
        "$ref": "#/$defs/n13"
      }
    },
    "n13": {
      "$ref": "#/$defs/n14"
    },
    "n14": {
      "type": "number"
    },
    "n2": {
      "$ref": "#/$defs/n3"
    },
    "n3": {
      "properties": {
        "list": {
          "$ref": "#/$defs/n4"
        }
      }
    },
    "n4": {
      "items": {
        "$ref": "#/$defs/n5"
      }
    },
    "n5": {
      "$ref": "#/$defs/n6"
    },
    "n6": {
      "minLength": 0,
      "type": "string"
    },
    "n7": {
      "properties": {
        "kindOfList": {
          "$ref": "#/$defs/n8"
        }
      },
      "required": [
        "kindOfList"
      ]
    },
    "n8": {
      "enum": [
        "numbers"
      ]
    },
    "n9": {
      "$ref": "#/$defs/n10"
    }
  },
  "$ref": "#/$defs/n2"
}"""
    root: GeneratedSchemaN2 = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN10(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "else": {
        "$ref": "#/$defs/n1"
      },
      "if": {
        "$ref": "#/$defs/n7"
      },
      "then": {
        "$ref": "#/$defs/n9"
      }
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n10": {
      "$ref": "#/$defs/n11"
    },
    "n11": {
      "properties": {
        "list": {
          "$ref": "#/$defs/n12"
        }
      }
    },
    "n12": {
      "items": {
        "$ref": "#/$defs/n13"
      }
    },
    "n13": {
      "$ref": "#/$defs/n14"
    },
    "n14": {
      "type": "number"
    },
    "n2": {
      "$ref": "#/$defs/n3"
    },
    "n3": {
      "properties": {
        "list": {
          "$ref": "#/$defs/n4"
        }
      }
    },
    "n4": {
      "items": {
        "$ref": "#/$defs/n5"
      }
    },
    "n5": {
      "$ref": "#/$defs/n6"
    },
    "n6": {
      "minLength": 0,
      "type": "string"
    },
    "n7": {
      "properties": {
        "kindOfList": {
          "$ref": "#/$defs/n8"
        }
      },
      "required": [
        "kindOfList"
      ]
    },
    "n8": {
      "enum": [
        "numbers"
      ]
    },
    "n9": {
      "$ref": "#/$defs/n10"
    }
  },
  "$ref": "#/$defs/n11"
}"""
    root: GeneratedSchemaN11 = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN11(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "else": {
        "$ref": "#/$defs/n1"
      },
      "if": {
        "$ref": "#/$defs/n7"
      },
      "then": {
        "$ref": "#/$defs/n9"
      }
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n10": {
      "$ref": "#/$defs/n11"
    },
    "n11": {
      "properties": {
        "list": {
          "$ref": "#/$defs/n12"
        }
      }
    },
    "n12": {
      "items": {
        "$ref": "#/$defs/n13"
      }
    },
    "n13": {
      "$ref": "#/$defs/n14"
    },
    "n14": {
      "type": "number"
    },
    "n2": {
      "$ref": "#/$defs/n3"
    },
    "n3": {
      "properties": {
        "list": {
          "$ref": "#/$defs/n4"
        }
      }
    },
    "n4": {
      "items": {
        "$ref": "#/$defs/n5"
      }
    },
    "n5": {
      "$ref": "#/$defs/n6"
    },
    "n6": {
      "minLength": 0,
      "type": "string"
    },
    "n7": {
      "properties": {
        "kindOfList": {
          "$ref": "#/$defs/n8"
        }
      },
      "required": [
        "kindOfList"
      ]
    },
    "n8": {
      "enum": [
        "numbers"
      ]
    },
    "n9": {
      "$ref": "#/$defs/n10"
    }
  },
  "properties": {
    "list": {
      "$ref": "#/$defs/n12"
    }
  }
}"""
    root: typing.Any = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN12(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "else": {
        "$ref": "#/$defs/n1"
      },
      "if": {
        "$ref": "#/$defs/n7"
      },
      "then": {
        "$ref": "#/$defs/n9"
      }
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n10": {
      "$ref": "#/$defs/n11"
    },
    "n11": {
      "properties": {
        "list": {
          "$ref": "#/$defs/n12"
        }
      }
    },
    "n12": {
      "items": {
        "$ref": "#/$defs/n13"
      }
    },
    "n13": {
      "$ref": "#/$defs/n14"
    },
    "n14": {
      "type": "number"
    },
    "n2": {
      "$ref": "#/$defs/n3"
    },
    "n3": {
      "properties": {
        "list": {
          "$ref": "#/$defs/n4"
        }
      }
    },
    "n4": {
      "items": {
        "$ref": "#/$defs/n5"
      }
    },
    "n5": {
      "$ref": "#/$defs/n6"
    },
    "n6": {
      "minLength": 0,
      "type": "string"
    },
    "n7": {
      "properties": {
        "kindOfList": {
          "$ref": "#/$defs/n8"
        }
      },
      "required": [
        "kindOfList"
      ]
    },
    "n8": {
      "enum": [
        "numbers"
      ]
    },
    "n9": {
      "$ref": "#/$defs/n10"
    }
  },
  "items": {
    "$ref": "#/$defs/n13"
  }
}"""
    root: typing.Any = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN13(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "else": {
        "$ref": "#/$defs/n1"
      },
      "if": {
        "$ref": "#/$defs/n7"
      },
      "then": {
        "$ref": "#/$defs/n9"
      }
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n10": {
      "$ref": "#/$defs/n11"
    },
    "n11": {
      "properties": {
        "list": {
          "$ref": "#/$defs/n12"
        }
      }
    },
    "n12": {
      "items": {
        "$ref": "#/$defs/n13"
      }
    },
    "n13": {
      "$ref": "#/$defs/n14"
    },
    "n14": {
      "type": "number"
    },
    "n2": {
      "$ref": "#/$defs/n3"
    },
    "n3": {
      "properties": {
        "list": {
          "$ref": "#/$defs/n4"
        }
      }
    },
    "n4": {
      "items": {
        "$ref": "#/$defs/n5"
      }
    },
    "n5": {
      "$ref": "#/$defs/n6"
    },
    "n6": {
      "minLength": 0,
      "type": "string"
    },
    "n7": {
      "properties": {
        "kindOfList": {
          "$ref": "#/$defs/n8"
        }
      },
      "required": [
        "kindOfList"
      ]
    },
    "n8": {
      "enum": [
        "numbers"
      ]
    },
    "n9": {
      "$ref": "#/$defs/n10"
    }
  },
  "$ref": "#/$defs/n14"
}"""
    root: float = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN14(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "else": {
        "$ref": "#/$defs/n1"
      },
      "if": {
        "$ref": "#/$defs/n7"
      },
      "then": {
        "$ref": "#/$defs/n9"
      }
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n10": {
      "$ref": "#/$defs/n11"
    },
    "n11": {
      "properties": {
        "list": {
          "$ref": "#/$defs/n12"
        }
      }
    },
    "n12": {
      "items": {
        "$ref": "#/$defs/n13"
      }
    },
    "n13": {
      "$ref": "#/$defs/n14"
    },
    "n14": {
      "type": "number"
    },
    "n2": {
      "$ref": "#/$defs/n3"
    },
    "n3": {
      "properties": {
        "list": {
          "$ref": "#/$defs/n4"
        }
      }
    },
    "n4": {
      "items": {
        "$ref": "#/$defs/n5"
      }
    },
    "n5": {
      "$ref": "#/$defs/n6"
    },
    "n6": {
      "minLength": 0,
      "type": "string"
    },
    "n7": {
      "properties": {
        "kindOfList": {
          "$ref": "#/$defs/n8"
        }
      },
      "required": [
        "kindOfList"
      ]
    },
    "n8": {
      "enum": [
        "numbers"
      ]
    },
    "n9": {
      "$ref": "#/$defs/n10"
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
      "else": {
        "$ref": "#/$defs/n1"
      },
      "if": {
        "$ref": "#/$defs/n7"
      },
      "then": {
        "$ref": "#/$defs/n9"
      }
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n10": {
      "$ref": "#/$defs/n11"
    },
    "n11": {
      "properties": {
        "list": {
          "$ref": "#/$defs/n12"
        }
      }
    },
    "n12": {
      "items": {
        "$ref": "#/$defs/n13"
      }
    },
    "n13": {
      "$ref": "#/$defs/n14"
    },
    "n14": {
      "type": "number"
    },
    "n2": {
      "$ref": "#/$defs/n3"
    },
    "n3": {
      "properties": {
        "list": {
          "$ref": "#/$defs/n4"
        }
      }
    },
    "n4": {
      "items": {
        "$ref": "#/$defs/n5"
      }
    },
    "n5": {
      "$ref": "#/$defs/n6"
    },
    "n6": {
      "minLength": 0,
      "type": "string"
    },
    "n7": {
      "properties": {
        "kindOfList": {
          "$ref": "#/$defs/n8"
        }
      },
      "required": [
        "kindOfList"
      ]
    },
    "n8": {
      "enum": [
        "numbers"
      ]
    },
    "n9": {
      "$ref": "#/$defs/n10"
    }
  },
  "$ref": "#/$defs/n3"
}"""
    root: GeneratedSchemaN3 = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN3(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "else": {
        "$ref": "#/$defs/n1"
      },
      "if": {
        "$ref": "#/$defs/n7"
      },
      "then": {
        "$ref": "#/$defs/n9"
      }
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n10": {
      "$ref": "#/$defs/n11"
    },
    "n11": {
      "properties": {
        "list": {
          "$ref": "#/$defs/n12"
        }
      }
    },
    "n12": {
      "items": {
        "$ref": "#/$defs/n13"
      }
    },
    "n13": {
      "$ref": "#/$defs/n14"
    },
    "n14": {
      "type": "number"
    },
    "n2": {
      "$ref": "#/$defs/n3"
    },
    "n3": {
      "properties": {
        "list": {
          "$ref": "#/$defs/n4"
        }
      }
    },
    "n4": {
      "items": {
        "$ref": "#/$defs/n5"
      }
    },
    "n5": {
      "$ref": "#/$defs/n6"
    },
    "n6": {
      "minLength": 0,
      "type": "string"
    },
    "n7": {
      "properties": {
        "kindOfList": {
          "$ref": "#/$defs/n8"
        }
      },
      "required": [
        "kindOfList"
      ]
    },
    "n8": {
      "enum": [
        "numbers"
      ]
    },
    "n9": {
      "$ref": "#/$defs/n10"
    }
  },
  "properties": {
    "list": {
      "$ref": "#/$defs/n4"
    }
  }
}"""
    root: typing.Any = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN4(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "else": {
        "$ref": "#/$defs/n1"
      },
      "if": {
        "$ref": "#/$defs/n7"
      },
      "then": {
        "$ref": "#/$defs/n9"
      }
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n10": {
      "$ref": "#/$defs/n11"
    },
    "n11": {
      "properties": {
        "list": {
          "$ref": "#/$defs/n12"
        }
      }
    },
    "n12": {
      "items": {
        "$ref": "#/$defs/n13"
      }
    },
    "n13": {
      "$ref": "#/$defs/n14"
    },
    "n14": {
      "type": "number"
    },
    "n2": {
      "$ref": "#/$defs/n3"
    },
    "n3": {
      "properties": {
        "list": {
          "$ref": "#/$defs/n4"
        }
      }
    },
    "n4": {
      "items": {
        "$ref": "#/$defs/n5"
      }
    },
    "n5": {
      "$ref": "#/$defs/n6"
    },
    "n6": {
      "minLength": 0,
      "type": "string"
    },
    "n7": {
      "properties": {
        "kindOfList": {
          "$ref": "#/$defs/n8"
        }
      },
      "required": [
        "kindOfList"
      ]
    },
    "n8": {
      "enum": [
        "numbers"
      ]
    },
    "n9": {
      "$ref": "#/$defs/n10"
    }
  },
  "items": {
    "$ref": "#/$defs/n5"
  }
}"""
    root: typing.Any = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN5(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "else": {
        "$ref": "#/$defs/n1"
      },
      "if": {
        "$ref": "#/$defs/n7"
      },
      "then": {
        "$ref": "#/$defs/n9"
      }
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n10": {
      "$ref": "#/$defs/n11"
    },
    "n11": {
      "properties": {
        "list": {
          "$ref": "#/$defs/n12"
        }
      }
    },
    "n12": {
      "items": {
        "$ref": "#/$defs/n13"
      }
    },
    "n13": {
      "$ref": "#/$defs/n14"
    },
    "n14": {
      "type": "number"
    },
    "n2": {
      "$ref": "#/$defs/n3"
    },
    "n3": {
      "properties": {
        "list": {
          "$ref": "#/$defs/n4"
        }
      }
    },
    "n4": {
      "items": {
        "$ref": "#/$defs/n5"
      }
    },
    "n5": {
      "$ref": "#/$defs/n6"
    },
    "n6": {
      "minLength": 0,
      "type": "string"
    },
    "n7": {
      "properties": {
        "kindOfList": {
          "$ref": "#/$defs/n8"
        }
      },
      "required": [
        "kindOfList"
      ]
    },
    "n8": {
      "enum": [
        "numbers"
      ]
    },
    "n9": {
      "$ref": "#/$defs/n10"
    }
  },
  "$ref": "#/$defs/n6"
}"""
    root: str = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN6(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "else": {
        "$ref": "#/$defs/n1"
      },
      "if": {
        "$ref": "#/$defs/n7"
      },
      "then": {
        "$ref": "#/$defs/n9"
      }
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n10": {
      "$ref": "#/$defs/n11"
    },
    "n11": {
      "properties": {
        "list": {
          "$ref": "#/$defs/n12"
        }
      }
    },
    "n12": {
      "items": {
        "$ref": "#/$defs/n13"
      }
    },
    "n13": {
      "$ref": "#/$defs/n14"
    },
    "n14": {
      "type": "number"
    },
    "n2": {
      "$ref": "#/$defs/n3"
    },
    "n3": {
      "properties": {
        "list": {
          "$ref": "#/$defs/n4"
        }
      }
    },
    "n4": {
      "items": {
        "$ref": "#/$defs/n5"
      }
    },
    "n5": {
      "$ref": "#/$defs/n6"
    },
    "n6": {
      "minLength": 0,
      "type": "string"
    },
    "n7": {
      "properties": {
        "kindOfList": {
          "$ref": "#/$defs/n8"
        }
      },
      "required": [
        "kindOfList"
      ]
    },
    "n8": {
      "enum": [
        "numbers"
      ]
    },
    "n9": {
      "$ref": "#/$defs/n10"
    }
  },
  "minLength": 0,
  "type": "string"
}"""
    root: str = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN7(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "else": {
        "$ref": "#/$defs/n1"
      },
      "if": {
        "$ref": "#/$defs/n7"
      },
      "then": {
        "$ref": "#/$defs/n9"
      }
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n10": {
      "$ref": "#/$defs/n11"
    },
    "n11": {
      "properties": {
        "list": {
          "$ref": "#/$defs/n12"
        }
      }
    },
    "n12": {
      "items": {
        "$ref": "#/$defs/n13"
      }
    },
    "n13": {
      "$ref": "#/$defs/n14"
    },
    "n14": {
      "type": "number"
    },
    "n2": {
      "$ref": "#/$defs/n3"
    },
    "n3": {
      "properties": {
        "list": {
          "$ref": "#/$defs/n4"
        }
      }
    },
    "n4": {
      "items": {
        "$ref": "#/$defs/n5"
      }
    },
    "n5": {
      "$ref": "#/$defs/n6"
    },
    "n6": {
      "minLength": 0,
      "type": "string"
    },
    "n7": {
      "properties": {
        "kindOfList": {
          "$ref": "#/$defs/n8"
        }
      },
      "required": [
        "kindOfList"
      ]
    },
    "n8": {
      "enum": [
        "numbers"
      ]
    },
    "n9": {
      "$ref": "#/$defs/n10"
    }
  },
  "properties": {
    "kindOfList": {
      "$ref": "#/$defs/n8"
    }
  },
  "required": [
    "kindOfList"
  ]
}"""
    root: typing.Any = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN8(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "else": {
        "$ref": "#/$defs/n1"
      },
      "if": {
        "$ref": "#/$defs/n7"
      },
      "then": {
        "$ref": "#/$defs/n9"
      }
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n10": {
      "$ref": "#/$defs/n11"
    },
    "n11": {
      "properties": {
        "list": {
          "$ref": "#/$defs/n12"
        }
      }
    },
    "n12": {
      "items": {
        "$ref": "#/$defs/n13"
      }
    },
    "n13": {
      "$ref": "#/$defs/n14"
    },
    "n14": {
      "type": "number"
    },
    "n2": {
      "$ref": "#/$defs/n3"
    },
    "n3": {
      "properties": {
        "list": {
          "$ref": "#/$defs/n4"
        }
      }
    },
    "n4": {
      "items": {
        "$ref": "#/$defs/n5"
      }
    },
    "n5": {
      "$ref": "#/$defs/n6"
    },
    "n6": {
      "minLength": 0,
      "type": "string"
    },
    "n7": {
      "properties": {
        "kindOfList": {
          "$ref": "#/$defs/n8"
        }
      },
      "required": [
        "kindOfList"
      ]
    },
    "n8": {
      "enum": [
        "numbers"
      ]
    },
    "n9": {
      "$ref": "#/$defs/n10"
    }
  },
  "enum": [
    "numbers"
  ]
}"""
    root: typing.Literal["numbers"] = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaN9(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "n0": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "else": {
        "$ref": "#/$defs/n1"
      },
      "if": {
        "$ref": "#/$defs/n7"
      },
      "then": {
        "$ref": "#/$defs/n9"
      }
    },
    "n1": {
      "$ref": "#/$defs/n2"
    },
    "n10": {
      "$ref": "#/$defs/n11"
    },
    "n11": {
      "properties": {
        "list": {
          "$ref": "#/$defs/n12"
        }
      }
    },
    "n12": {
      "items": {
        "$ref": "#/$defs/n13"
      }
    },
    "n13": {
      "$ref": "#/$defs/n14"
    },
    "n14": {
      "type": "number"
    },
    "n2": {
      "$ref": "#/$defs/n3"
    },
    "n3": {
      "properties": {
        "list": {
          "$ref": "#/$defs/n4"
        }
      }
    },
    "n4": {
      "items": {
        "$ref": "#/$defs/n5"
      }
    },
    "n5": {
      "$ref": "#/$defs/n6"
    },
    "n6": {
      "minLength": 0,
      "type": "string"
    },
    "n7": {
      "properties": {
        "kindOfList": {
          "$ref": "#/$defs/n8"
        }
      },
      "required": [
        "kindOfList"
      ]
    },
    "n8": {
      "enum": [
        "numbers"
      ]
    },
    "n9": {
      "$ref": "#/$defs/n10"
    }
  },
  "$ref": "#/$defs/n10"
}"""
    root: GeneratedSchemaN10 = dc.root_field()

@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$defs": {
    "genericList": {
      "$defs": {
        "defaultItemType": {
          "$comment": "Only needed to satisfy bookending requirement",
          "$dynamicAnchor": "itemType"
        }
      },
      "$id": "genericList",
      "properties": {
        "list": {
          "items": {
            "$dynamicRef": "#itemType"
          }
        }
      }
    },
    "numberList": {
      "$defs": {
        "itemType": {
          "$dynamicAnchor": "itemType",
          "type": "number"
        }
      },
      "$id": "numberList",
      "$ref": "genericList"
    },
    "stringList": {
      "$defs": {
        "itemType": {
          "$dynamicAnchor": "itemType",
          "type": "string"
        }
      },
      "$id": "stringList",
      "$ref": "genericList"
    }
  },
  "$id": "https://test.json-schema.org/dynamic-ref-with-multiple-paths/main",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "else": {
    "$ref": "stringList"
  },
  "if": {
    "properties": {
      "kindOfList": {
        "const": "numbers"
      }
    },
    "required": [
      "kindOfList"
    ]
  },
  "then": {
    "$ref": "numberList"
  }
}"""
    root: GeneratedSchemaN0 = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
