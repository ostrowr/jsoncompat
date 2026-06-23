from __future__ import annotations

import collections.abc
from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassAdditionalModel[str]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "additionalProperties": false,
  "patternProperties": {
    "^x": {
      "type": "string"
    }
  },
  "type": "object"
}"""
    __jsoncompat_extra__: collections.abc.Mapping[str, str] = dc.extra_field()

JSONCOMPAT_MODEL = GeneratedSchema

dc.bind_generated_models((
    (
        GeneratedSchema,
        "object",
        (
        ),
        True,
        str,
    ),
))
