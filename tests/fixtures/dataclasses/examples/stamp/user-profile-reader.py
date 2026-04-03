from __future__ import annotations

from dataclasses import dataclass
import typing

from jsoncompat.codegen import dataclasses as jsoncompat_dataclasses


@dataclass(frozen=True, slots=True, kw_only=True)
class ExamplesStampUserProfileV1(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"v1\":{\"properties\":{\"age\":{\"minimum\":0,\"type\":\"integer\"},\"name\":{\"minLength\":1,\"type\":\"string\"}},\"required\":[\"name\",\"age\"],\"type\":\"object\",\"x-jsoncompat\":{\"kind\":\"declaration\",\"name\":\"ExamplesStampUserProfileV1\",\"schema_ref\":\"#/$defs/v1\",\"stable_id\":\"examples/stamp/user-profile\",\"version\":1}},\"v2\":{\"properties\":{\"age\":{\"minimum\":0,\"type\":\"integer\"},\"interests\":{\"type\":\"integer\"},\"name\":{\"minLength\":1,\"type\":\"string\"}},\"required\":[\"name\",\"age\",\"interests\"],\"type\":\"object\",\"x-jsoncompat\":{\"kind\":\"declaration\",\"name\":\"ExamplesStampUserProfileV2\",\"schema_ref\":\"#/$defs/v2\",\"stable_id\":\"examples/stamp/user-profile\",\"version\":2}}},\"properties\":{\"age\":{\"minimum\":0,\"type\":\"integer\"},\"name\":{\"minLength\":1,\"type\":\"string\"}},\"required\":[\"name\",\"age\"],\"type\":\"object\",\"x-jsoncompat\":{\"kind\":\"declaration\",\"name\":\"ExamplesStampUserProfileV1\",\"schema_ref\":\"#/$defs/v1\",\"stable_id\":\"examples/stamp/user-profile\",\"version\":1}}"
    age: int = jsoncompat_dataclasses.jsoncompat_field("age")
    name: str = jsoncompat_dataclasses.jsoncompat_field("name")
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class ExamplesStampUserProfileV2(jsoncompat_dataclasses.DataclassAdditionalModel[typing.Any]):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"v1\":{\"properties\":{\"age\":{\"minimum\":0,\"type\":\"integer\"},\"name\":{\"minLength\":1,\"type\":\"string\"}},\"required\":[\"name\",\"age\"],\"type\":\"object\",\"x-jsoncompat\":{\"kind\":\"declaration\",\"name\":\"ExamplesStampUserProfileV1\",\"schema_ref\":\"#/$defs/v1\",\"stable_id\":\"examples/stamp/user-profile\",\"version\":1}},\"v2\":{\"properties\":{\"age\":{\"minimum\":0,\"type\":\"integer\"},\"interests\":{\"type\":\"integer\"},\"name\":{\"minLength\":1,\"type\":\"string\"}},\"required\":[\"name\",\"age\",\"interests\"],\"type\":\"object\",\"x-jsoncompat\":{\"kind\":\"declaration\",\"name\":\"ExamplesStampUserProfileV2\",\"schema_ref\":\"#/$defs/v2\",\"stable_id\":\"examples/stamp/user-profile\",\"version\":2}}},\"properties\":{\"age\":{\"minimum\":0,\"type\":\"integer\"},\"interests\":{\"type\":\"integer\"},\"name\":{\"minLength\":1,\"type\":\"string\"}},\"required\":[\"name\",\"age\",\"interests\"],\"type\":\"object\",\"x-jsoncompat\":{\"kind\":\"declaration\",\"name\":\"ExamplesStampUserProfileV2\",\"schema_ref\":\"#/$defs/v2\",\"stable_id\":\"examples/stamp/user-profile\",\"version\":2}}"
    age: int = jsoncompat_dataclasses.jsoncompat_field("age")
    interests: int = jsoncompat_dataclasses.jsoncompat_field("interests")
    name: str = jsoncompat_dataclasses.jsoncompat_field("name")
    __jsoncompat_extra__: dict[str, typing.Any] = jsoncompat_dataclasses.jsoncompat_extra_field()

@dataclass(frozen=True, slots=True, kw_only=True)
class ExamplesStampUserProfileV2Reader(jsoncompat_dataclasses.ReaderDataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"v1\":{\"properties\":{\"age\":{\"minimum\":0,\"type\":\"integer\"},\"name\":{\"minLength\":1,\"type\":\"string\"}},\"required\":[\"name\",\"age\"],\"type\":\"object\",\"x-jsoncompat\":{\"kind\":\"declaration\",\"name\":\"ExamplesStampUserProfileV1\",\"schema_ref\":\"#/$defs/v1\",\"stable_id\":\"examples/stamp/user-profile\",\"version\":1}},\"v2\":{\"properties\":{\"age\":{\"minimum\":0,\"type\":\"integer\"},\"interests\":{\"type\":\"integer\"},\"name\":{\"minLength\":1,\"type\":\"string\"}},\"required\":[\"name\",\"age\",\"interests\"],\"type\":\"object\",\"x-jsoncompat\":{\"kind\":\"declaration\",\"name\":\"ExamplesStampUserProfileV2\",\"schema_ref\":\"#/$defs/v2\",\"stable_id\":\"examples/stamp/user-profile\",\"version\":2}}},\"additionalProperties\":false,\"properties\":{\"data\":{\"$ref\":\"#/$defs/v2\"},\"version\":{\"const\":2}},\"required\":[\"version\",\"data\"],\"type\":\"object\",\"x-jsoncompat\":{\"kind\":\"reader_variant\",\"name\":\"ExamplesStampUserProfileV2Reader\",\"payload_ref\":\"#/$defs/v2\",\"stable_id\":\"examples/stamp/user-profile\",\"version\":2}}"
    version: typing.Literal[2] = jsoncompat_dataclasses.jsoncompat_field("version")
    data: ExamplesStampUserProfileV2 = jsoncompat_dataclasses.jsoncompat_field("data")

@dataclass(frozen=True, slots=True, kw_only=True)
class ExamplesStampUserProfileV1Reader(jsoncompat_dataclasses.ReaderDataclassModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"v1\":{\"properties\":{\"age\":{\"minimum\":0,\"type\":\"integer\"},\"name\":{\"minLength\":1,\"type\":\"string\"}},\"required\":[\"name\",\"age\"],\"type\":\"object\",\"x-jsoncompat\":{\"kind\":\"declaration\",\"name\":\"ExamplesStampUserProfileV1\",\"schema_ref\":\"#/$defs/v1\",\"stable_id\":\"examples/stamp/user-profile\",\"version\":1}},\"v2\":{\"properties\":{\"age\":{\"minimum\":0,\"type\":\"integer\"},\"interests\":{\"type\":\"integer\"},\"name\":{\"minLength\":1,\"type\":\"string\"}},\"required\":[\"name\",\"age\",\"interests\"],\"type\":\"object\",\"x-jsoncompat\":{\"kind\":\"declaration\",\"name\":\"ExamplesStampUserProfileV2\",\"schema_ref\":\"#/$defs/v2\",\"stable_id\":\"examples/stamp/user-profile\",\"version\":2}}},\"additionalProperties\":false,\"properties\":{\"data\":{\"$ref\":\"#/$defs/v1\"},\"version\":{\"const\":1}},\"required\":[\"version\",\"data\"],\"type\":\"object\",\"x-jsoncompat\":{\"kind\":\"reader_variant\",\"name\":\"ExamplesStampUserProfileV1Reader\",\"payload_ref\":\"#/$defs/v1\",\"stable_id\":\"examples/stamp/user-profile\",\"version\":1}}"
    version: typing.Literal[1] = jsoncompat_dataclasses.jsoncompat_field("version")
    data: ExamplesStampUserProfileV1 = jsoncompat_dataclasses.jsoncompat_field("data")


@dataclass(frozen=True, slots=True, kw_only=True)
class ExamplesStampUserProfileReader(jsoncompat_dataclasses.ReaderDataclassRootModel):
    __jsoncompat_schema__: typing.ClassVar[str] = "{\"$defs\":{\"v1\":{\"properties\":{\"age\":{\"minimum\":0,\"type\":\"integer\"},\"name\":{\"minLength\":1,\"type\":\"string\"}},\"required\":[\"name\",\"age\"],\"type\":\"object\",\"x-jsoncompat\":{\"kind\":\"declaration\",\"name\":\"ExamplesStampUserProfileV1\",\"schema_ref\":\"#/$defs/v1\",\"stable_id\":\"examples/stamp/user-profile\",\"version\":1}},\"v2\":{\"properties\":{\"age\":{\"minimum\":0,\"type\":\"integer\"},\"interests\":{\"type\":\"integer\"},\"name\":{\"minLength\":1,\"type\":\"string\"}},\"required\":[\"name\",\"age\",\"interests\"],\"type\":\"object\",\"x-jsoncompat\":{\"kind\":\"declaration\",\"name\":\"ExamplesStampUserProfileV2\",\"schema_ref\":\"#/$defs/v2\",\"stable_id\":\"examples/stamp/user-profile\",\"version\":2}}},\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"oneOf\":[{\"additionalProperties\":false,\"properties\":{\"data\":{\"$ref\":\"#/$defs/v2\"},\"version\":{\"const\":2}},\"required\":[\"version\",\"data\"],\"type\":\"object\",\"x-jsoncompat\":{\"kind\":\"reader_variant\",\"name\":\"ExamplesStampUserProfileV2Reader\",\"payload_ref\":\"#/$defs/v2\",\"stable_id\":\"examples/stamp/user-profile\",\"version\":2}},{\"additionalProperties\":false,\"properties\":{\"data\":{\"$ref\":\"#/$defs/v1\"},\"version\":{\"const\":1}},\"required\":[\"version\",\"data\"],\"type\":\"object\",\"x-jsoncompat\":{\"kind\":\"reader_variant\",\"name\":\"ExamplesStampUserProfileV1Reader\",\"payload_ref\":\"#/$defs/v1\",\"stable_id\":\"examples/stamp/user-profile\",\"version\":1}}],\"title\":\"examples/stamp/user-profile reader\",\"x-jsoncompat\":{\"kind\":\"reader\",\"name\":\"ExamplesStampUserProfileReader\",\"stable_id\":\"examples/stamp/user-profile\"}}"
    root: (ExamplesStampUserProfileV1Reader | ExamplesStampUserProfileV2Reader) = jsoncompat_dataclasses.jsoncompat_root_field()

ExamplesStampUserProfileV1.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("age", "age", int),
    jsoncompat_dataclasses.jsoncompat_field_spec("name", "name", str),
    extra_annotation=dict[str, typing.Any],
)

ExamplesStampUserProfileV2.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("age", "age", int),
    jsoncompat_dataclasses.jsoncompat_field_spec("interests", "interests", int),
    jsoncompat_dataclasses.jsoncompat_field_spec("name", "name", str),
    extra_annotation=dict[str, typing.Any],
)

ExamplesStampUserProfileV2Reader.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("version", "version", typing.Literal[2]),
    jsoncompat_dataclasses.jsoncompat_field_spec("data", "data", ExamplesStampUserProfileV2),
)

ExamplesStampUserProfileV1Reader.__jsoncompat_object_spec__ = jsoncompat_dataclasses.jsoncompat_object_spec(
    jsoncompat_dataclasses.jsoncompat_field_spec("version", "version", typing.Literal[1]),
    jsoncompat_dataclasses.jsoncompat_field_spec("data", "data", ExamplesStampUserProfileV1),
)


ExamplesStampUserProfileReader.__jsoncompat_root_annotation__ = (ExamplesStampUserProfileV1Reader | ExamplesStampUserProfileV2Reader)

JSONCOMPAT_MODEL = ExamplesStampUserProfileReader
