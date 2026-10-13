"""Run an aiohttp session with its own timeout, credentials, and connection pool."""

import asyncio

import aiohttp
from exercise import exercise_async
from jsoncompat.http import Err, UnexpectedResponse, bind_async
from jsoncompat.http.aiohttp import AiohttpFetch, AiohttpOptions
from petstore.api import PetstoreAPI
from petstore.operations.pets import GetPet404, GetPetRequest
from server import serve


async def main() -> None:
    with serve() as base_url:
        async with aiohttp.ClientSession(
            headers={"Authorization": "Bearer example-token"},
            timeout=aiohttp.ClientTimeout(total=10),
        ) as session:
            client = bind_async(PetstoreAPI, fetch=AiohttpFetch(session, base_url))
            response = await client.request(
                GetPetRequest(pet_id="p_123"),
                transport=AiohttpOptions(timeout=aiohttp.ClientTimeout(total=2)),
            )
            print(response.status, response.headers.get("ETag"))  # No model parsing.
            pet = response.unwrap()  # GetPet200; parsed and cached now.
            print(pet.body.name, pet.headers.etag)

            for pet_id in ("unknown", "unavailable"):
                response = await client.request(GetPetRequest(pet_id=pet_id))
                match response.result():
                    case Err(error=GetPet404(body=problem)):
                        print("Not found:", problem.message)
                    case Err(error=UnexpectedResponse(status=503) as error):
                        print(
                            "Unavailable:",
                            error.text(),
                            error.headers.get("Retry-After"),
                        )
                    case result:
                        print(result.unwrap().body.name)
            await exercise_async(client)
    print("aiohttp: all ten operations passed")


if __name__ == "__main__":
    asyncio.run(main())
