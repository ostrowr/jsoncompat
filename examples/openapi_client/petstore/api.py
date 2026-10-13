# Generated operation registry. Dispatch is prepared by codegen and bound on import.
from __future__ import annotations

import json as _json
import typing as _typing
from typing_extensions import TypeAliasType as _TypeAliasType, TypeVar as _TypeVar
from jsoncompat import JsonValue as _JsonValue
from jsoncompat import http as _http
from jsoncompat.http._binding import install_generated_api as _install_generated_api
from ._api import PetstoreAPI as PetstoreAPI

_TransportOptions = _TypeVar("_TransportOptions", default=_typing.Never)
PetstoreClient = _TypeAliasType("PetstoreClient", _http.Client[PetstoreAPI, _TransportOptions], type_params=(_TransportOptions,))
PetstoreAsyncClient = _TypeAliasType("PetstoreAsyncClient", _http.AsyncClient[PetstoreAPI, _TransportOptions], type_params=(_TransportOptions,))
from .operations.groups import *
from .operations.groups import OPERATIONS as _operations_operations_groups
from .operations.groups import _JSONCOMPAT_OPERATIONS as _plans_operations_groups
from .operations.health import *
from .operations.health import OPERATIONS as _operations_operations_health
from .operations.health import _JSONCOMPAT_OPERATIONS as _plans_operations_health
from .operations.pets import *
from .operations.pets import OPERATIONS as _operations_operations_pets
from .operations.pets import _JSONCOMPAT_OPERATIONS as _plans_operations_pets
from .operations.search import *
from .operations.search import OPERATIONS as _operations_operations_search
from .operations.search import _JSONCOMPAT_OPERATIONS as _plans_operations_search

SECURITY_SCHEMES: _typing.Final[dict[str, _JsonValue]] = _typing.cast(dict[str, _JsonValue], _json.loads("{\"bearerAuth\":{\"scheme\":\"bearer\",\"type\":\"http\"}}"))

OPERATIONS: _typing.Final[tuple[_http.Operation, ...]] = tuple(_operation for _group in (_operations_operations_groups, _operations_operations_health, _operations_operations_pets, _operations_operations_search,) for _operation in _group)

_install_generated_api(PetstoreAPI, 1, tuple(_plan for _group in (_plans_operations_groups, _plans_operations_health, _plans_operations_pets, _plans_operations_search,) for _plan in _group))

__all__: tuple[str, ...] = ("PetstoreAPI", "PetstoreClient", "PetstoreAsyncClient", "SECURITY_SCHEMES", "DeletePetResponse", "DeletePetSuccess", "DeletePetError", "DeletePetResult", "GetGroupPetsResponse", "GetGroupPetsSuccess", "GetGroupPetsError", "GetGroupPetsResult", "HealthResponse", "HealthSuccess", "HealthError", "HealthResult", "ListPetsResponse", "ListPetsSuccess", "ListPetsError", "ListPetsResult", "GetPetResponse", "GetPetSuccess", "GetPetError", "GetPetResult", "GetPetNoteResponse", "GetPetNoteSuccess", "GetPetNoteError", "GetPetNoteResult", "SearchPetsResponse", "SearchPetsSuccess", "SearchPetsError", "SearchPetsResult", "UpdatePetResponse", "UpdatePetSuccess", "UpdatePetError", "UpdatePetResult", "CreatePetResponse", "CreatePetSuccess", "CreatePetError", "CreatePetResult", "SetPetNoteResponse", "SetPetNoteSuccess", "SetPetNoteError", "SetPetNoteResult",)
