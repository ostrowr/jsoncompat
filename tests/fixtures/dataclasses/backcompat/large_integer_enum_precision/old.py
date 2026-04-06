from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"enum\":[9007199254740993,9007199254740995],\"minimum\":0}"
    root: (typing.Literal[9007199254740993] | typing.Literal[9007199254740995]) = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchema.__jsoncompat_root_annotation__ = (typing.Literal[9007199254740993] | typing.Literal[9007199254740995])

JSONCOMPAT_MODEL = GeneratedSchema
