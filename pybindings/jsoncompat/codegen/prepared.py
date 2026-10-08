"""Small loading helpers for modules emitted by :mod:`jsoncompat.codegen.build`."""

from __future__ import annotations

import copy
import dataclasses
from typing import Any, Literal, cast

from jsoncompat import bind_prepared_model_runtimes

from . import dataclasses as dc

PREPARED_VERSION = 1
type FieldKind = Literal["field", "classvar", "initvar"]
type FieldDefinition = tuple[str, str, object, FieldKind]


def finish_dataclass(
    model: type[dc.DataclassModel], definitions: tuple[FieldDefinition, ...]
) -> None:
    """Install dataclass introspection metadata without generating functions."""
    templates = dc.DataclassModel.__dataclass_fields__
    field_spec = dc._FieldSpec  # pyright: ignore[reportPrivateUsage]
    kinds = {
        "field": field_spec.__dataclass_fields__["json_name"]._field_type,
        "classvar": templates[dc.JSONCOMPAT_SCHEMA_FIELD]._field_type,
        "initvar": templates["skip_validation"]._field_type,
    }
    fields = dict(templates)
    for name, annotation, raw_field, kind in definitions:
        if not isinstance(raw_field, dataclasses.Field):
            raise TypeError("Prepared dataclass metadata requires dataclass fields")
        field = cast(dataclasses.Field[Any], raw_field)
        field.name = name
        field.type = annotation
        field._field_type = kinds[kind]
        if field.kw_only is dataclasses.MISSING:
            field.kw_only = kind != "classvar"
        fields[name] = field
    model.__dataclass_fields__ = fields
    params = copy.copy(getattr(field_spec, "__dataclass_params__"))
    for name, value in (("kw_only", True), ("slots", True), ("weakref_slot", False)):
        if hasattr(params, name):
            setattr(params, name, value)
    setattr(model, "__dataclass_params__", params)
    # CPython's generated initializer stores the inherited InitVar annotation
    # verbatim, and its return annotation is the value None, not a string.
    initializer = getattr(model, "__init__")
    initializer.__annotations__["skip_validation"] = templates["skip_validation"].type
    initializer.__annotations__["return"] = None


def bind_module(
    version: int,
    roots: list[tuple[type[dc.DataclassModel], int]],
    descriptors: list[tuple[Any, ...]],
    prepared_plan: bytes,
) -> None:
    """Bind process-local class/slot references in a precomputed model graph."""
    if version != PREPARED_VERSION:
        raise ImportError("Unsupported jsoncompat prepared module; rebuild it")
    runtimes = bind_prepared_model_runtimes(
        roots, descriptors, dc.FrozenList, dc.FrozenDict, prepared_plan
    )
    for (model, _), runtime in zip(roots, runtimes, strict=True):
        setattr(model, dc.JSONCOMPAT_RUNTIME_FIELD, runtime)
