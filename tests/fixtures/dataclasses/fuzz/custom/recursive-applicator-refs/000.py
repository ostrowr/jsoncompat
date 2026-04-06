from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaTreeBranch1(jsoncompat_dataclasses.DataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"tree\":{\"anyOf\":[{\"enum\":[null]},{\"additionalProperties\":false,\"minProperties\":2,\"properties\":{\"left\":{\"$ref\":\"#/$defs/tree\"},\"right\":{\"$ref\":\"#/$defs/tree\"}},\"required\":[\"left\",\"right\"],\"type\":\"object\"}]}},\"additionalProperties\":false,\"minProperties\":2,\"properties\":{\"left\":{\"$ref\":\"#/$defs/tree\"},\"right\":{\"$ref\":\"#/$defs/tree\"}},\"required\":[\"left\",\"right\"],\"type\":\"object\"}"
    left: GeneratedSchemaTree = jsoncompat_dataclasses.jsoncompat_field("left")
    right: GeneratedSchemaTree = jsoncompat_dataclasses.jsoncompat_field("right")

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchemaTree(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"tree\":{\"anyOf\":[{\"enum\":[null]},{\"additionalProperties\":false,\"minProperties\":2,\"properties\":{\"left\":{\"$ref\":\"#/$defs/tree\"},\"right\":{\"$ref\":\"#/$defs/tree\"}},\"required\":[\"left\",\"right\"],\"type\":\"object\"}]}},\"anyOf\":[{\"enum\":[null]},{\"additionalProperties\":false,\"minProperties\":2,\"properties\":{\"left\":{\"$ref\":\"#/$defs/tree\"},\"right\":{\"$ref\":\"#/$defs/tree\"}},\"required\":[\"left\",\"right\"],\"type\":\"object\"}]}"
    root: (GeneratedSchemaTreeBranch1 | typing.Literal[None]) = jsoncompat_dataclasses.jsoncompat_root_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"tree\":{\"anyOf\":[{\"enum\":[null]},{\"additionalProperties\":false,\"minProperties\":2,\"properties\":{\"left\":{\"$ref\":\"#/$defs/tree\"},\"right\":{\"$ref\":\"#/$defs/tree\"}},\"required\":[\"left\",\"right\"],\"type\":\"object\"}]}},\"$ref\":\"#/$defs/tree\",\"$schema\":\"https://json-schema.org/draft/2020-12/schema\"}"
    root: GeneratedSchemaTree = jsoncompat_dataclasses.jsoncompat_root_field()

GeneratedSchemaTreeBranch1.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("left", "left", GeneratedSchemaTree),
    jsoncompat_dataclasses.jsoncompat_field_spec("right", "right", GeneratedSchemaTree),
)

GeneratedSchemaTree.__jsoncompat_root_annotation__ = (GeneratedSchemaTreeBranch1 | typing.Literal[None])

GeneratedSchema.__jsoncompat_root_annotation__ = GeneratedSchemaTree

JSONCOMPAT_MODEL = GeneratedSchema
