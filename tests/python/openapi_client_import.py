"""Fresh-process checks for import-time preparation and session-free aliases."""

import asyncio
import importlib
import sys
from typing import get_type_hints
from unittest.mock import patch

from jsoncompat.http import (
    ContractError,
    RawResponse,
    _binding,
    bind,
    bind_async,
    prepare,
)

mode = sys.argv[1]
petstore = importlib.import_module("petstore")
assert "petstore.api" not in sys.modules

calls = []


def fetch(request, /, *, transport=None):
    calls.append(request)
    return RawResponse(
        200,
        (("Content-Type", "application/json"), ("ETag", "r1")),
        b'{"id":"p_123","name":"Mochi","species":"cat","tags":[]}',
    )


async def async_fetch(request, /, *, transport=None):
    return fetch(request, transport=transport)


if mode == "failed":
    identity = importlib.import_module("petstore._api").PetstoreAPI
    request_model = _binding._request_model
    attempted = []

    def fail_later(model, api):
        attempted.append(model)
        if len(attempted) == 2:
            raise ContractError("invalid operation during import")
        return request_model(model, api)

    with patch.object(_binding, "_request_model", side_effect=fail_later):
        try:
            importlib.import_module("petstore.api")
        except ContractError:
            pass
        else:
            raise AssertionError("invalid registry imported successfully")
    assert len(attempted) == 2
    assert identity.__jsoncompat_http_plans__ is None
    assert "petstore.api" not in sys.modules

with (
    patch.object(_binding, "_prepare", side_effect=AssertionError("runtime preparation")),
    patch.object(_binding, "get_type_hints", side_effect=AssertionError("reflection")),
    patch.object(_binding.dataclasses, "fields", side_effect=AssertionError("field reflection")),
):
    if mode == "model":
        request_type = petstore.GetPetRequest
        assert "petstore.api" not in sys.modules
    elif mode == "identity":
        identity = importlib.import_module("petstore._api").PetstoreAPI
        assert "petstore.api" not in sys.modules
        # Binding the private identity loads the public precomputed registry.
        bind(identity, fetch=fetch)
    elif mode in ("PetstoreClient", "PetstoreAsyncClient"):
        getattr(petstore, mode)
    api = petstore.PetstoreAPI
    assert len(api.__jsoncompat_http_plans__) == len(api.operations())

assert not calls
assert "__jsoncompat_http_plans__" in get_type_hints(api)
assert not {"aiohttp", "httpx", "requests"} & sys.modules.keys()
if mode == "model":
    assert petstore.GetPetRequest is request_type

plans = api.__jsoncompat_http_plans__
try:
    plans[petstore.GetPetRequest] = None
except TypeError:
    pass
else:
    raise AssertionError("prepared plans are mutable")


async def exercise():
    with (
        patch.object(api, "operations", side_effect=AssertionError("registry reload")),
        patch.object(_binding, "_prepare", side_effect=AssertionError("repreparation")),
        patch.object(
            _binding, "get_type_hints", side_effect=AssertionError("reflection")
        ),
    ):
        request = petstore.GetPetRequest(pet_id="p_123")
        first = bind(api, fetch=fetch)
        second = bind_async(api, fetch=async_fetch)
        assert first._plans is plans
        assert second._plans is plans
        assert first.request(request).unwrap().body.name == "Mochi"
        assert (await second.request(request)).unwrap().body.name == "Mochi"
        call = prepare(api, request)
        assert call.decode(fetch(call.request)).unwrap().body.name == "Mochi"
        assert api.__jsoncompat_http_plans__ is plans


asyncio.run(exercise())
assert len(calls) == 3
