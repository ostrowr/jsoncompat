from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class GeneratedSchema(jsoncompat_dataclasses.DataclassAdditionalModel[int]):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"additionalProperties\":{\"multipleOf\":1,\"type\":\"integer\"},\"minProperties\":1,\"properties\":{\"id\":{\"multipleOf\":1,\"type\":\"integer\"}},\"propertyNames\":{\"anyOf\":[{\"enum\":[null]},{\"enum\":[false,true]},{\"minProperties\":0,\"properties\":{},\"type\":\"object\"},{\"items\":true,\"minItems\":0,\"type\":\"array\"},{\"minLength\":0,\"pattern\":\"^[a-z]+$\",\"type\":\"string\"},{\"type\":\"number\"}]},\"required\":[\"id\"],\"type\":\"object\"}"
    id: int = jsoncompat_dataclasses.jsoncompat_field("id")
    __jsoncompat_extra__: dict[str, int] = jsoncompat_dataclasses.jsoncompat_extra_field()

GeneratedSchema.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("id", "id", int),
    extra_annotation=dict[str, int],
)

JSONCOMPAT_MODEL = GeneratedSchema
