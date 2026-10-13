"""Inspect unexpected errors without losing their status, headers, or body.

Run with: uv run --project pybindings --extra aiohttp python examples/openapi_client/unexpected_responses.py
The same functions also run through requests and both HTTPX adapters.
"""

import asyncio

from jsoncompat.http import (
    ClientResponse,
    Err,
    HTTPStatusError,
    ResponseError,
    UnexpectedResponse,
)
from petstore.api import PetstoreAPI, PetstoreAsyncClient, PetstoreClient
from petstore.operations.pets import GetPetError, GetPetRequest, GetPetSuccess


def inspect_error(
    response: ClientResponse[GetPetSuccess, GetPetError],
) -> UnexpectedResponse:
    # request() has downloaded the bytes and released the connection, but has
    # not parsed a model. Ordinary properties never trigger model decoding.
    print("Before decoding:", response.status, response.headers.get("Content-Type"))
    assert response.content

    result = response.result()  # Attempt contract decoding here, once.
    match result:
        case Err(error=UnexpectedResponse() as error):
            print("HTTP status:", error.status)
            print("Retry-After:", error.headers.get("Retry-After"))
            print("Raw bytes:", error.body)
            print("Text:", error.text())
            try:
                details = error.json()  # Explicit JSON parsing; no schema assumed.
            except ValueError:
                print("The body is not JSON; use its text or bytes instead.")
            else:
                print("JSON details:", details)
            print("Contract diagnostic:", error.decode_error)
            assert error.response is response.raw
            assert error.body is response.content
            assert response.result() is result  # Cached; no new decoding.
            return error
        case _:
            raise AssertionError("this demo expects an unexpected error response")


def check_error(pet_id: str, error: UnexpectedResponse) -> None:
    # These assertions make the examples integration tests for every adapter.
    assert error.headers.get_all("X-Trace") == ("first", "second")
    if pet_id == "unavailable":
        assert error.status == 503
        assert error.headers.get("retry-after") == "10"
        assert error.json() == {"message": "Try again later"}
    elif pet_id == "gateway-error":
        assert error.status == 502
        assert error.headers["content-type"] == "text/html; charset=utf-8"
        assert error.body == b"<h1>Bad gateway</h1>"
        assert error.text() == "<h1>Bad gateway</h1>"
        try:
            error.json()
        except ValueError:
            pass
        else:
            raise AssertionError("HTML must not be accepted as JSON")
    else:
        # This is valid JSON, but it is not the declared GetPet404 body.
        assert pet_id == "broken-error" and error.status == 404
        assert error.json() == {"detail": "The pet was removed"}
        assert error.decode_error.__cause__ is not None


def inspect_unwrap_failure(
    response: ClientResponse[GetPetSuccess, GetPetError],
) -> None:
    try:
        response.unwrap()  # Decode now and raise HTTPStatusError for a non-2xx.
    except HTTPStatusError as exc:
        # Raw status and bytes are accessible without narrowing exc.error.
        print("Unwrap failed:", exc.response.status, exc.response.body)
        assert exc.response is response.raw
        assert isinstance(exc.error, UnexpectedResponse)
        assert exc.error.headers.get("Retry-After") == "10"
    else:
        raise AssertionError("an undeclared 503 must fail unwrap()")


def inspect_invalid_success(
    response: ClientResponse[GetPetSuccess, GetPetError],
) -> None:
    # Even an invalid success can be inspected before choosing to parse it.
    assert response.status == 200
    assert response.headers.get("etag") == '"rev-1"'
    assert response.content == b"{}"
    try:
        response.result()
    except ResponseError as exc:
        print("Invalid success body:", exc.response.body)
        assert exc.response is response.raw
    else:
        raise AssertionError("an invalid success must not produce Ok")


def exercise_unexpected(client: PetstoreClient) -> None:
    for pet_id in ("unavailable", "gateway-error", "broken-error"):
        response = client.request(GetPetRequest(pet_id=pet_id))
        check_error(pet_id, inspect_error(response))
    inspect_unwrap_failure(client.request(GetPetRequest(pet_id="unavailable")))
    inspect_invalid_success(client.request(GetPetRequest(pet_id="broken-success")))


async def exercise_unexpected_async(client: PetstoreAsyncClient) -> None:
    for pet_id in ("unavailable", "gateway-error", "broken-error"):
        response = await client.request(GetPetRequest(pet_id=pet_id))
        check_error(pet_id, inspect_error(response))
    inspect_unwrap_failure(await client.request(GetPetRequest(pet_id="unavailable")))
    inspect_invalid_success(
        await client.request(GetPetRequest(pet_id="broken-success"))
    )


async def main() -> None:
    # Keep this module usable by the requests/HTTPX examples without aiohttp.
    import aiohttp
    from jsoncompat.http import bind_async
    from jsoncompat.http.aiohttp import AiohttpFetch
    from server import serve

    with serve() as base_url:
        async with aiohttp.ClientSession(
            headers={"Authorization": "Bearer example-token"},
            timeout=aiohttp.ClientTimeout(total=10),
        ) as session:
            client = bind_async(PetstoreAPI, fetch=AiohttpFetch(session, base_url))
            await exercise_unexpected_async(client)
    print("unexpected responses: JSON, HTML, schema failures, and unwrap passed")


if __name__ == "__main__":
    asyncio.run(main())
