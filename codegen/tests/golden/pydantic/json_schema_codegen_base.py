from __future__ import annotations

from typing import Any

from pydantic import BaseModel, ConfigDict, RootModel

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
    model_config = ConfigDict(
        strict=True,
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
    model_config = ConfigDict(
        strict=True,
        validate_by_alias=True,
        validate_by_name=True,
        serialize_by_alias=True,
    )

class SerializerRootModel(RootModel[Any]):
    model_config = ConfigDict(
        strict=True,
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
    model_config = ConfigDict(
        strict=True,
        validate_by_alias=True,
        validate_by_name=True,
        serialize_by_alias=True,
    )

