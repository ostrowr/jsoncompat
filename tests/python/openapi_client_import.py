"""Fresh-process checks for import-time preparation and session-free aliases."""

import asyncio
import importlib
import sys
from pathlib import Path
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

if mode == "legacy":
    # Previously generated packages had no import-time initialization hook.
    source = Path(petstore.__file__).with_name("api.py")
    source.write_text(source.read_text().replace("_initialize_api(PetstoreAPI)\n", ""))

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
    prepare_operation = _binding._prepare
    attempted = []

    def fail_later(spec, api):
        attempted.append(spec)
        if len(attempted) == 2:
            raise ContractError("invalid operation during import")
        return prepare_operation(spec, api)

    with patch.object(_binding, "_prepare", side_effect=fail_later):
        try:
            importlib.import_module("petstore.api")
        except ContractError:
            pass
        else:
            raise AssertionError("invalid registry imported successfully")
    assert len(attempted) == 2
    assert identity.__jsoncompat_http_plans__ is None
    assert "petstore.api" not in sys.modules

with patch.object(_binding, "_prepare", wraps=_binding._prepare) as build:
    if mode == "model":
        request_type = petstore.GetPetRequest
        assert "petstore.api" not in sys.modules
        assert build.call_count == 0
    elif mode == "identity":
        identity = importlib.import_module("petstore._api").PetstoreAPI
        assert "petstore.api" not in sys.modules
        # Internal/older identities still prepare safely when first bound.
        bind(identity, fetch=fetch)
    elif mode in ("PetstoreClient", "PetstoreAsyncClient"):
        getattr(petstore, mode)
    api = petstore.PetstoreAPI
    if mode == "legacy":
        assert build.call_count == 0
        bind(api, fetch=fetch)
    assert build.call_count == len(api.operations())
    assert len(api.__jsoncompat_http_plans__) == build.call_count

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
