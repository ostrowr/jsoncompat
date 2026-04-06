from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class ExamplesStampUserProfileV2(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"v2\":{\"minProperties\":3,\"properties\":{\"age\":{\"minimum\":0,\"multipleOf\":1,\"type\":\"integer\"},\"interests\":{\"multipleOf\":1,\"type\":\"integer\"},\"name\":{\"minLength\":1,\"type\":\"string\"}},\"required\":[\"age\",\"interests\",\"name\"],\"type\":\"object\",\"x-jsoncompat\":{\"kind\":\"declaration\",\"name\":\"ExamplesStampUserProfileV2\",\"schema_ref\":\"#/$defs/v2\",\"stable_id\":\"examples/stamp/user-profile\",\"version\":2}}},\"minProperties\":3,\"properties\":{\"age\":{\"minimum\":0,\"multipleOf\":1,\"type\":\"integer\"},\"interests\":{\"multipleOf\":1,\"type\":\"integer\"},\"name\":{\"minLength\":1,\"type\":\"string\"}},\"required\":[\"age\",\"interests\",\"name\"],\"type\":\"object\",\"x-jsoncompat\":{\"kind\":\"declaration\",\"name\":\"ExamplesStampUserProfileV2\",\"schema_ref\":\"#/$defs/v2\",\"stable_id\":\"examples/stamp/user-profile\",\"version\":2}}"
    age: int = jsoncompat_dataclasses.jsoncompat_field("age")
    interests: int = jsoncompat_dataclasses.jsoncompat_field("interests")
    name: str = jsoncompat_dataclasses.jsoncompat_field("name")
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class ExamplesStampUserProfileWriter(jsoncompat_dataclasses.WriterDataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"v2\":{\"minProperties\":3,\"properties\":{\"age\":{\"minimum\":0,\"multipleOf\":1,\"type\":\"integer\"},\"interests\":{\"multipleOf\":1,\"type\":\"integer\"},\"name\":{\"minLength\":1,\"type\":\"string\"}},\"required\":[\"age\",\"interests\",\"name\"],\"type\":\"object\",\"x-jsoncompat\":{\"kind\":\"declaration\",\"name\":\"ExamplesStampUserProfileV2\",\"schema_ref\":\"#/$defs/v2\",\"stable_id\":\"examples/stamp/user-profile\",\"version\":2}}},\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"additionalProperties\":false,\"minProperties\":2,\"properties\":{\"data\":{\"$ref\":\"#/$defs/v2\"},\"version\":{\"enum\":[2]}},\"required\":[\"data\",\"version\"],\"title\":\"examples/stamp/user-profile writer v2\",\"type\":\"object\",\"x-jsoncompat\":{\"kind\":\"writer\",\"name\":\"ExamplesStampUserProfileWriter\",\"payload_ref\":\"#/$defs/v2\",\"stable_id\":\"examples/stamp/user-profile\",\"version\":2}}"
    version: typing.Literal[2] = jsoncompat_dataclasses.jsoncompat_field("version")
    data: ExamplesStampUserProfileV2 = jsoncompat_dataclasses.jsoncompat_field("data")

ExamplesStampUserProfileV2.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("age", "age", int),
    jsoncompat_dataclasses.jsoncompat_field_spec("interests", "interests", int),
    jsoncompat_dataclasses.jsoncompat_field_spec("name", "name", str),
    extra_annotation=dict[str, typing.Any],
)

ExamplesStampUserProfileWriter.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("version", "version", typing.Literal[2]),
    jsoncompat_dataclasses.jsoncompat_field_spec("data", "data", ExamplesStampUserProfileV2),
)

JSONCOMPAT_MODEL = ExamplesStampUserProfileWriter
