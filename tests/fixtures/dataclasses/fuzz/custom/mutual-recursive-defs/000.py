from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaABranch1(jsoncompat_dataclasses.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"a\":{\"anyOf\":[{\"enum\":[null]},{\"additionalProperties\":false,\"minProperties\":1,\"properties\":{\"next\":{\"$ref\":\"#/$defs/b\"}},\"required\":[\"next\"],\"type\":\"object\"}]},\"b\":{\"anyOf\":[{\"enum\":[null]},{\"additionalProperties\":false,\"minProperties\":1,\"properties\":{\"next\":{\"$ref\":\"#/$defs/a\"}},\"required\":[\"next\"],\"type\":\"object\"}]}},\"additionalProperties\":false,\"minProperties\":1,\"properties\":{\"next\":{\"$ref\":\"#/$defs/b\"}},\"required\":[\"next\"],\"type\":\"object\"}"
    next: GeneratedSchemaB = jsoncompat_dataclasses.jsoncompat_field("next")

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaA(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"a\":{\"anyOf\":[{\"enum\":[null]},{\"additionalProperties\":false,\"minProperties\":1,\"properties\":{\"next\":{\"$ref\":\"#/$defs/b\"}},\"required\":[\"next\"],\"type\":\"object\"}]},\"b\":{\"anyOf\":[{\"enum\":[null]},{\"additionalProperties\":false,\"minProperties\":1,\"properties\":{\"next\":{\"$ref\":\"#/$defs/a\"}},\"required\":[\"next\"],\"type\":\"object\"}]}},\"anyOf\":[{\"enum\":[null]},{\"additionalProperties\":false,\"minProperties\":1,\"properties\":{\"next\":{\"$ref\":\"#/$defs/b\"}},\"required\":[\"next\"],\"type\":\"object\"}]}"
    root: (GeneratedSchemaABranch1 | typing.Literal[None]) = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaBBranch1(jsoncompat_dataclasses.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"a\":{\"anyOf\":[{\"enum\":[null]},{\"additionalProperties\":false,\"minProperties\":1,\"properties\":{\"next\":{\"$ref\":\"#/$defs/b\"}},\"required\":[\"next\"],\"type\":\"object\"}]},\"b\":{\"anyOf\":[{\"enum\":[null]},{\"additionalProperties\":false,\"minProperties\":1,\"properties\":{\"next\":{\"$ref\":\"#/$defs/a\"}},\"required\":[\"next\"],\"type\":\"object\"}]}},\"additionalProperties\":false,\"minProperties\":1,\"properties\":{\"next\":{\"$ref\":\"#/$defs/a\"}},\"required\":[\"next\"],\"type\":\"object\"}"
    next: GeneratedSchemaA = jsoncompat_dataclasses.jsoncompat_field("next")

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaB(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"a\":{\"anyOf\":[{\"enum\":[null]},{\"additionalProperties\":false,\"minProperties\":1,\"properties\":{\"next\":{\"$ref\":\"#/$defs/b\"}},\"required\":[\"next\"],\"type\":\"object\"}]},\"b\":{\"anyOf\":[{\"enum\":[null]},{\"additionalProperties\":false,\"minProperties\":1,\"properties\":{\"next\":{\"$ref\":\"#/$defs/a\"}},\"required\":[\"next\"],\"type\":\"object\"}]}},\"anyOf\":[{\"enum\":[null]},{\"additionalProperties\":false,\"minProperties\":1,\"properties\":{\"next\":{\"$ref\":\"#/$defs/a\"}},\"required\":[\"next\"],\"type\":\"object\"}]}"
    root: (GeneratedSchemaBBranch1 | typing.Literal[None]) = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"a\":{\"anyOf\":[{\"enum\":[null]},{\"additionalProperties\":false,\"minProperties\":1,\"properties\":{\"next\":{\"$ref\":\"#/$defs/b\"}},\"required\":[\"next\"],\"type\":\"object\"}]},\"b\":{\"anyOf\":[{\"enum\":[null]},{\"additionalProperties\":false,\"minProperties\":1,\"properties\":{\"next\":{\"$ref\":\"#/$defs/a\"}},\"required\":[\"next\"],\"type\":\"object\"}]}},\"$ref\":\"#/$defs/a\",\"$schema\":\"https://json-schema.org/draft/2020-12/schema\"}"
    root: GeneratedSchemaA = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchemaABranch1.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("next", "next", GeneratedSchemaB),
)

GeneratedSchemaA.__jsoncompat_root_annotation__ = (GeneratedSchemaABranch1 | typing.Literal[None])

GeneratedSchemaBBranch1.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("next", "next", GeneratedSchemaA),
)

GeneratedSchemaB.__jsoncompat_root_annotation__ = (GeneratedSchemaBBranch1 | typing.Literal[None])

GeneratedSchema.__jsoncompat_root_annotation__ = GeneratedSchemaA

JSONCOMPAT_MODEL = GeneratedSchema
