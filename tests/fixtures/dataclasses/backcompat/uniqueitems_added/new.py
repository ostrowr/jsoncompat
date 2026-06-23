from __future__ import annotations

import collections.abc
from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as dc


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(dc.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = """{
  "items": {
    "type": "integer"
  },
  "type": "array",
  "uniqueItems": true
}"""
    root: collections.abc.Sequence[int] = dc.root_field()

JSONCOMPAT_MODEL = GeneratedSchema
