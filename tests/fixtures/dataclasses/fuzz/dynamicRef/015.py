from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$id\":\"http://localhost:1234/draft2020-12/strict-extendible-allof-defs-first.json\",\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"allOf\":[{\"$ref\":\"extendible-dynamic-ref.json\"},{\"$defs\":{\"elements\":{\"$dynamicAnchor\":\"elements\",\"anyOf\":[{\"enum\":[null]},{\"enum\":[false,true]},{\"additionalProperties\":false,\"minProperties\":1,\"properties\":{\"a\":true},\"required\":[\"a\"],\"type\":\"object\"},{\"items\":true,\"minItems\":0,\"type\":\"array\"},{\"minLength\":0,\"type\":\"string\"},{\"type\":\"number\"}]}},\"anyOf\":[{\"enum\":[null]},{\"enum\":[false,true]},{\"minProperties\":0,\"properties\":{},\"type\":\"object\"},{\"items\":true,\"minItems\":0,\"type\":\"array\"},{\"minLength\":0,\"type\":\"string\"},{\"type\":\"number\"}]}]}"
    root: typing.Any = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchema.__jsoncompat_root_annotation__ = typing.Any

JSONCOMPAT_MODEL = GeneratedSchema
