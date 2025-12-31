from typing import Any, ClassVar

from jsonschema_rs import validator_for
from pydantic import BaseModel, ConfigDict, RootModel, model_validator

def _json_equal(candidate, expected):
    if isinstance(expected, bool):
        return isinstance(candidate, bool) and candidate is expected
    if expected is None:
        return candidate is None
    if isinstance(expected, (int, float)) and not isinstance(expected, bool):
        return isinstance(candidate, (int, float)) and not isinstance(candidate, bool) and candidate == expected
    if isinstance(expected, list):
        return isinstance(candidate, list) and len(candidate) == len(expected) and all(_json_equal(c, e) for c, e in zip(candidate, expected))
    if isinstance(expected, dict):
        return isinstance(candidate, dict) and candidate.keys() == expected.keys() and all(_json_equal(candidate[k], v) for k, v in expected.items())
    return candidate == expected

def _validate_literal(value, allowed):
    if any(_json_equal(value, expected) for expected in allowed):
        return value
    raise ValueError("value does not match literal constraint")

class SerializerBase(BaseModel):
    __json_schema__: ClassVar[str | None] = None
    _validate_formats: ClassVar[bool] = False
    _jsonschema_validator: ClassVar[object | None] = None

    @classmethod
    def _get_jsonschema_validator(cls):
        if cls.__json_schema__ is None:
            raise TypeError(f"{cls.__name__} is missing __json_schema__")
        validator = cls._jsonschema_validator
        if validator is None:
            validator = validator_for(cls.__json_schema__, validate_formats=cls._validate_formats)
            cls._jsonschema_validator = validator
        return validator

    @model_validator(mode="before")
    @classmethod
    def _validate_jsonschema(cls, value):
        cls._get_jsonschema_validator().validate(value)
        return value

    model_config = ConfigDict(
        validate_by_alias=True,
        validate_by_name=True,
        serialize_by_alias=True,
    )

    def model_dump(self, **kwargs):
        kwargs.setdefault("exclude_unset", True)
        return super().model_dump(**kwargs)

    def model_dump_json(self, **kwargs):
        kwargs.setdefault("exclude_unset", True)
        return super().model_dump_json(**kwargs)

class DeserializerBase(BaseModel):
    __json_schema__: ClassVar[str | None] = None
    _validate_formats: ClassVar[bool] = False
    _jsonschema_validator: ClassVar[object | None] = None

    @classmethod
    def _get_jsonschema_validator(cls):
        if cls.__json_schema__ is None:
            raise TypeError(f"{cls.__name__} is missing __json_schema__")
        validator = cls._jsonschema_validator
        if validator is None:
            validator = validator_for(cls.__json_schema__, validate_formats=cls._validate_formats)
            cls._jsonschema_validator = validator
        return validator

    @model_validator(mode="before")
    @classmethod
    def _validate_jsonschema(cls, value):
        cls._get_jsonschema_validator().validate(value)
        return value

    model_config = ConfigDict(
        validate_by_alias=True,
        validate_by_name=True,
        serialize_by_alias=True,
    )

class SerializerRootModel(RootModel[Any]):
    __json_schema__: ClassVar[str | None] = None
    _validate_formats: ClassVar[bool] = False
    _jsonschema_validator: ClassVar[object | None] = None

    @classmethod
    def _get_jsonschema_validator(cls):
        if cls.__json_schema__ is None:
            raise TypeError(f"{cls.__name__} is missing __json_schema__")
        validator = cls._jsonschema_validator
        if validator is None:
            validator = validator_for(cls.__json_schema__, validate_formats=cls._validate_formats)
            cls._jsonschema_validator = validator
        return validator

    @model_validator(mode="before")
    @classmethod
    def _validate_jsonschema(cls, value):
        cls._get_jsonschema_validator().validate(value)
        return value

    model_config = ConfigDict(
        validate_by_alias=True,
        validate_by_name=True,
        serialize_by_alias=True,
    )

    def model_dump(self, **kwargs):
        kwargs.setdefault("exclude_unset", True)
        return super().model_dump(**kwargs)

    def model_dump_json(self, **kwargs):
        kwargs.setdefault("exclude_unset", True)
        return super().model_dump_json(**kwargs)

class DeserializerRootModel(RootModel[Any]):
    __json_schema__: ClassVar[str | None] = None
    _validate_formats: ClassVar[bool] = False
    _jsonschema_validator: ClassVar[object | None] = None

    @classmethod
    def _get_jsonschema_validator(cls):
        if cls.__json_schema__ is None:
            raise TypeError(f"{cls.__name__} is missing __json_schema__")
        validator = cls._jsonschema_validator
        if validator is None:
            validator = validator_for(cls.__json_schema__, validate_formats=cls._validate_formats)
            cls._jsonschema_validator = validator
        return validator

    @model_validator(mode="before")
    @classmethod
    def _validate_jsonschema(cls, value):
        cls._get_jsonschema_validator().validate(value)
        return value

    model_config = ConfigDict(
        validate_by_alias=True,
        validate_by_name=True,
        serialize_by_alias=True,
    )

