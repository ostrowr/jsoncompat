from __future__ import annotations

import collections.abc
from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "additionalProperties": true,
  "properties": {
    "x": {
      "type": "string"
    }
  },
  "type": "object"
}"""
    x: dc.Omittable[str] = dc.field("x", omittable=True)
    __jsoncompat_extra__: collections.abc.Mapping[str, typing.Any] = dc.extra_field()

JSONCOMPAT_MODEL = GeneratedSchema

dc.bind_generated_models((
    (
        GeneratedSchema,
        "object",
        (
            ("x", "x", str, True),
        ),
        True,
        typing.Any,
    ),
))
