from __future__ import annotations

import collections.abc
from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassAdditionalModel[int]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "additionalProperties": false,
  "patternProperties": {
    "^\\\\cC$": {
      "type": "integer"
    }
  },
  "type": "object"
}"""
    __jsoncompat_extra__: collections.abc.Mapping[str, int] = dc.extra_field()

JSONCOMPAT_MODEL = GeneratedSchema
