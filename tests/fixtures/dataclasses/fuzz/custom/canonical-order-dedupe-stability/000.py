from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class ColorEnum(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"enum\":[\"red\",\"blue\",\"red\"],\"title\":\"color enum\",\"type\":\"string\"}"
    root: (typing.Literal["blue"] | typing.Literal["red"]) = jsoncompat_dataclasses.jsoncompat_root_field()

ColorEnum.__jsoncompat_root_annotation__ = (typing.Literal["blue"] | typing.Literal["red"])

JSONCOMPAT_MODEL = ColorEnum
