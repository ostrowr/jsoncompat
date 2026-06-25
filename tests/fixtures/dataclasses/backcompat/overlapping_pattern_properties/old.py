from __future__ import annotations

import collections.abc
from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@typing.final
@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassAdditionalModel[(int | str)]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "additionalProperties": false,
  "patternProperties": {
    "^x": {
      "type": "string"
    },
    "x$": {
      "type": "integer"
    }
  },
  "type": "object"
}"""
    __jsoncompat_extra__: collections.abc.Mapping[str, (int | str)] = dc.extra_field()

JSONCOMPAT_MODEL = GeneratedSchema
