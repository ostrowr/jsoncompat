"""Check what reaches the socket, including escaping libraries may normalize."""

import asyncio
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from queue import Queue
from threading import Event, Thread
from unittest.mock import patch

import aiohttp
import httpx
import requests
from jsoncompat.http import RequestOptions, bind, bind_async, prepare
from petstore import GetPetRequest, PetPatch, PetstoreAPI, UpdatePetRequest
from transports import (
    AiohttpFetch,
    AiohttpOptions,
    AsyncHttpxFetch,
    HttpxFetch,
    HttpxOptions,
    RequestsFetch,
    RequestsOptions,
)

seen = Queue()
slow_requests = Queue()


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *args):
        pass

    def do_GET(self):
        self.respond()

    def do_PATCH(self):
        self.respond()

    def respond(self):
        if self.path == "/api/pets/slow":
            # The caller releases this handler after its timeout. The upper
            # bound also makes an ignored timeout fail instead of hanging.
            release = slow_requests.get(timeout=5)
            release.wait(timeout=5)
            self.close_connection = True
            return
        seen.put(
            (
                self.path,
                dict(self.headers),
                self.rfile.read(int(self.headers.get("Content-Length", "0"))),
            )
        )
        payload = b'{"id":"p","name":"Mochi","species":"cat","tags":[]}'
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        self.send_header("ETag", '"r1"')
        self.send_header("Set-Cookie", "a=1; Expires=Wed, 21 Oct 2026 07:28:00 GMT")
        self.send_header("Set-Cookie", "b=2")
        self.end_headers()
        self.wfile.write(payload)


GET = GetPetRequest(pet_id="a/b ?#%é", include_history=False, if_none_match='"r0"')
PATCH = UpdatePetRequest(pet_id="p", body=PetPatch(nickname=None))
OMITTED = UpdatePetRequest(pet_id="p")


def check_get(result):
    target, headers, body = seen.get(timeout=5)
    assert target == "/api/pets/a%2Fb%20%3F%23%25%C3%A9?includeHistory=false", target
    assert headers["If-None-Match"] == '"r0"'
    assert headers["Authorization"] == "Bearer demo"
    assert body == b""
    assert [
        pair[1] for pair in result.raw_headers if pair[0].lower() == "set-cookie"
    ] == ["a=1; Expires=Wed, 21 Oct 2026 07:28:00 GMT", "b=2"]


def check_patch(present):
    target, headers, body = seen.get(timeout=5)
    assert target == "/api/pets/p"
    if present:
        assert headers["Content-Type"] == "application/json"
        assert body == b'{"nickname":null}'
    else:
        assert "Content-Type" not in headers, headers
        assert body == b""


def prepared():
    return prepare(
        PetstoreAPI,
        GET,
        options=RequestOptions(
            extra_headers={"X-Trace-ID": "override"},
            remove_headers=("IF-NONE-MATCH",),
            remove_query=("includeHistory",),
            extra_query=(
                ("includeHistory", "true"),
                ("preview", "猫 +%"),
                ("preview", "two"),
            ),
        ),
    )


def check_override(result):
    target, headers, body = seen.get(timeout=5)
    assert (
        target
        == "/api/pets/a%2Fb%20%3F%23%25%C3%A9?includeHistory=true&preview=%E7%8C%AB%20%2B%25&preview=two"
    ), target
    assert "if-none-match" not in {key.lower() for key in headers}
    assert headers["X-Trace-ID"] == "override"
    assert headers["Authorization"] == "Bearer demo"  # Session defaults still apply.
    assert body == b"" and result.body.name == "Mochi"


def sync(client, fetch):
    check_get(client.request(GET))
    client.request(PATCH)
    check_patch(True)
    client.request(OMITTED)
    check_patch(False)
    call = prepared()
    raw = fetch(call.request)
    assert raw.status == 200 and raw.body
    check_override(call.decode(raw))
    check_get(client.request(GET))


async def asynchronous(client, fetch):
    check_get(await client.request(GET))
    await client.request(PATCH)
    check_patch(True)
    await client.request(OMITTED)
    check_patch(False)
    call = prepared()
    raw = await fetch(call.request)
    assert raw.status == 200 and raw.body
    check_override(call.decode(raw))
    check_get(await client.request(GET))


def sync_timeout(client, fetch, transport, exception):
    release = Event()
    slow_requests.put(release)
    default = (
        fetch.timeout if isinstance(fetch, RequestsFetch) else fetch.session.timeout
    )
    with patch.object(fetch.session, "request", wraps=fetch.session.request) as send:
        try:
            with unittest.TestCase().assertRaises(exception):
                client.request(GetPetRequest(pet_id="slow"), transport=transport)
        finally:
            release.set()
        check_get(client.request(GET))
        disabled = type(transport)(timeout=None)
        check_get(client.request(GET, transport=disabled))
        assert send.call_args_list[0].kwargs["timeout"] is transport.timeout
        assert send.call_args_list[1].kwargs["timeout"] == default
        assert send.call_args_list[2].kwargs["timeout"] is None
    assert (
        fetch.timeout if isinstance(fetch, RequestsFetch) else fetch.session.timeout
    ) == default


async def async_timeout(client, fetch, transport, exception):
    release = Event()
    slow_requests.put(release)
    default = fetch.session.timeout
    with patch.object(fetch.session, "request", wraps=fetch.session.request) as send:
        try:
            with unittest.TestCase().assertRaises(exception):
                await client.request(GetPetRequest(pet_id="slow"), transport=transport)
        finally:
            release.set()
        check_get(await client.request(GET))
        disabled = (
            AiohttpOptions(timeout=aiohttp.ClientTimeout(total=None))
            if isinstance(fetch, AiohttpFetch)
            else HttpxOptions(timeout=None)
        )
        check_get(await client.request(GET, transport=disabled))
        assert send.call_args_list[0].kwargs["timeout"] is transport.timeout
        assert send.call_args_list[1].kwargs["timeout"] == default
        assert send.call_args_list[2].kwargs["timeout"] is disabled.timeout
    assert fetch.session.timeout == default


async def async_checks(url):
    async with aiohttp.ClientSession(
        headers={"Authorization": "Bearer demo"}, timeout=aiohttp.ClientTimeout(total=5)
    ) as session:
        fetch = AiohttpFetch(session, url)
        await asynchronous(bind_async(PetstoreAPI, fetch=fetch), fetch)
        await async_timeout(
            bind_async(PetstoreAPI, fetch=fetch),
            fetch,
            AiohttpOptions(timeout=aiohttp.ClientTimeout(total=0.1)),
            TimeoutError,
        )
    async with httpx.AsyncClient(
        headers={"Authorization": "Bearer demo"}, timeout=5, trust_env=False
    ) as session:
        fetch = AsyncHttpxFetch(session, url)
        await asynchronous(bind_async(PetstoreAPI, fetch=fetch), fetch)
        await async_timeout(
            bind_async(PetstoreAPI, fetch=fetch),
            fetch,
            HttpxOptions(timeout=httpx.Timeout(5, read=0.1)),
            httpx.ReadTimeout,
        )


with ThreadingHTTPServer(("127.0.0.1", 0), Handler) as server:
    thread = Thread(target=server.serve_forever, daemon=True)
    thread.start()
    url = f"http://127.0.0.1:{server.server_port}/api"
    try:
        with requests.Session() as session:
            session.trust_env = False
            session.headers["Authorization"] = "Bearer demo"
            fetch = RequestsFetch(session, url, timeout=5)
            sync(bind(PetstoreAPI, fetch=fetch), fetch)
            sync_timeout(
                bind(PetstoreAPI, fetch=fetch),
                fetch,
                RequestsOptions(timeout=(5, 0.1)),
                requests.ReadTimeout,
            )
        with httpx.Client(
            headers={"Authorization": "Bearer demo"}, timeout=5, trust_env=False
        ) as session:
            fetch = HttpxFetch(session, url)
            sync(bind(PetstoreAPI, fetch=fetch), fetch)
            sync_timeout(
                bind(PetstoreAPI, fetch=fetch),
                fetch,
                HttpxOptions(timeout=httpx.Timeout(5, read=0.1)),
                httpx.ReadTimeout,
            )
        asyncio.run(async_checks(url))
    finally:
        server.shutdown()
        thread.join()
print(
    "all transports preserve wire data and apply per-call timeouts without changing defaults"
)
