from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"allOf\":[{\"minimum\":0,\"multipleOf\":1,\"type\":\"integer\"},{\"anyOf\":[{\"enum\":[null]},{\"enum\":[false,true]},{\"minProperties\":0,\"properties\":{},\"type\":\"object\"},{\"items\":true,\"minItems\":0,\"type\":\"array\"},{\"minLength\":0,\"type\":\"string\"},{\"maximum\":10,\"type\":\"number\"}]}]}"
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchema.__jsoncompat_root_annotation__ = typing.Any

JSONCOMPAT_MODEL = GeneratedSchema
