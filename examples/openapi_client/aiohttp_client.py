"""Run an aiohttp session with its own timeout, credentials, and connection pool."""

import asyncio

import aiohttp
from exercise import exercise_async
from jsoncompat.http import bind_async
from petstore import AsyncPetstore
from server import serve
from transports import AiohttpFetch


async def main() -> None:
    with serve() as base_url:
        async with aiohttp.ClientSession(
            headers={"Authorization": "Bearer example-token"},
            timeout=aiohttp.ClientTimeout(total=10),
        ) as session:
            client = bind_async(AsyncPetstore, fetch=AiohttpFetch(session, base_url))
            await exercise_async(client)
    print("aiohttp: all ten operations passed")


if __name__ == "__main__":
    asyncio.run(main())
