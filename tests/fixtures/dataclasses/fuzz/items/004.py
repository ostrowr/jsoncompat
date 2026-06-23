from __future__ import annotations

import collections.abc
from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": {
    "items": {
      "items": {
        "items": {
          "type": "number"
        },
        "type": "array"
      },
      "type": "array"
    },
    "type": "array"
  },
  "type": "array"
}"""
    root: collections.abc.Sequence[collections.abc.Sequence[collections.abc.Sequence[collections.abc.Sequence[float]]]] = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema

dc.bind_generated_models((
    (GeneratedSchema, "root", collections.abc.Sequence[collections.abc.Sequence[collections.abc.Sequence[collections.abc.Sequence[float]]]]),
))
