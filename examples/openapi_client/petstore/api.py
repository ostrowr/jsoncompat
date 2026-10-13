# Generated operation registry. Dispatch is prepared by codegen and bound on import.
from __future__ import annotations

import typing as _typing

from typing_extensions import TypeAliasType as _TypeAliasType, TypeVar as _TypeVar
from jsoncompat import JsonValue as _JsonValue
from jsoncompat import http as _http
from jsoncompat.codegen import http as _http_codegen

from ._api import PetstoreAPI as PetstoreAPI
from .operations.groups import (
    GetGroupPetsResponse as GetGroupPetsResponse,
    GetGroupPetsSuccess as GetGroupPetsSuccess,
    GetGroupPetsError as GetGroupPetsError,
    GetGroupPetsResult as GetGroupPetsResult,
    OPERATIONS as _operations_groups,
    _JSONCOMPAT_OPERATIONS as _plans_groups,
)
from .operations.health import (
    HealthResponse as HealthResponse,
    HealthSuccess as HealthSuccess,
    HealthError as HealthError,
    HealthResult as HealthResult,
    OPERATIONS as _operations_health,
    _JSONCOMPAT_OPERATIONS as _plans_health,
)
from .operations.pets import (
    DeletePetResponse as DeletePetResponse,
    DeletePetSuccess as DeletePetSuccess,
    DeletePetError as DeletePetError,
    DeletePetResult as DeletePetResult,
    ListPetsResponse as ListPetsResponse,
    ListPetsSuccess as ListPetsSuccess,
    ListPetsError as ListPetsError,
    ListPetsResult as ListPetsResult,
    GetPetResponse as GetPetResponse,
    GetPetSuccess as GetPetSuccess,
    GetPetError as GetPetError,
    GetPetResult as GetPetResult,
    GetPetNoteResponse as GetPetNoteResponse,
    GetPetNoteSuccess as GetPetNoteSuccess,
    GetPetNoteError as GetPetNoteError,
    GetPetNoteResult as GetPetNoteResult,
    UpdatePetResponse as UpdatePetResponse,
    UpdatePetSuccess as UpdatePetSuccess,
    UpdatePetError as UpdatePetError,
    UpdatePetResult as UpdatePetResult,
    CreatePetResponse as CreatePetResponse,
    CreatePetSuccess as CreatePetSuccess,
    CreatePetError as CreatePetError,
    CreatePetResult as CreatePetResult,
    SetPetNoteResponse as SetPetNoteResponse,
    SetPetNoteSuccess as SetPetNoteSuccess,
    SetPetNoteError as SetPetNoteError,
    SetPetNoteResult as SetPetNoteResult,
    OPERATIONS as _operations_pets,
    _JSONCOMPAT_OPERATIONS as _plans_pets,
)
from .operations.search import (
    SearchPetsResponse as SearchPetsResponse,
    SearchPetsSuccess as SearchPetsSuccess,
    SearchPetsError as SearchPetsError,
    SearchPetsResult as SearchPetsResult,
    OPERATIONS as _operations_search,
    _JSONCOMPAT_OPERATIONS as _plans_search,
)

_TransportOptions = _TypeVar("_TransportOptions", default=_typing.Never)
PetstoreClient = _TypeAliasType(
    "PetstoreClient",
    _http.Client[PetstoreAPI, _TransportOptions],
    type_params=(_TransportOptions,),
)
PetstoreAsyncClient = _TypeAliasType(
    "PetstoreAsyncClient",
    _http.AsyncClient[PetstoreAPI, _TransportOptions],
    type_params=(_TransportOptions,),
)

SECURITY_SCHEMES: _typing.Final[dict[str, _JsonValue]] = {"bearerAuth": {"scheme": "bearer", "type": "http"}}

OPERATIONS: _typing.Final[tuple[_http.Operation, ...]] = tuple(
    _operation
    for _group in (
        _operations_groups,
        _operations_health,
        _operations_pets,
        _operations_search,
    )
    for _operation in _group
)

_http_codegen.install_api(
    PetstoreAPI,
    1,
    tuple(
        _plan
        for _group in (
            _plans_groups,
            _plans_health,
            _plans_pets,
            _plans_search,
        )
        for _plan in _group
    ),
)

__all__: tuple[str, ...] = (
    "PetstoreAPI",
    "PetstoreClient",
    "PetstoreAsyncClient",
    "SECURITY_SCHEMES",
    "DeletePetResponse",
    "DeletePetSuccess",
    "DeletePetError",
    "DeletePetResult",
    "GetGroupPetsResponse",
    "GetGroupPetsSuccess",
    "GetGroupPetsError",
    "GetGroupPetsResult",
    "HealthResponse",
    "HealthSuccess",
    "HealthError",
    "HealthResult",
    "ListPetsResponse",
    "ListPetsSuccess",
    "ListPetsError",
    "ListPetsResult",
    "GetPetResponse",
    "GetPetSuccess",
    "GetPetError",
    "GetPetResult",
    "GetPetNoteResponse",
    "GetPetNoteSuccess",
    "GetPetNoteError",
    "GetPetNoteResult",
    "SearchPetsResponse",
    "SearchPetsSuccess",
    "SearchPetsError",
    "SearchPetsResult",
    "UpdatePetResponse",
    "UpdatePetSuccess",
    "UpdatePetError",
    "UpdatePetResult",
    "CreatePetResponse",
    "CreatePetSuccess",
    "CreatePetError",
    "CreatePetResult",
    "SetPetNoteResponse",
    "SetPetNoteSuccess",
    "SetPetNoteError",
    "SetPetNoteResult",
)
