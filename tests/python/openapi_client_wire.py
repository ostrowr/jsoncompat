"""Check what reaches the socket, including escaping libraries may normalize."""

import asyncio
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from queue import Queue
from threading import Thread

import aiohttp
import httpx
import requests
from jsoncompat.http import RequestOptions, bind, bind_async, prepare
from petstore import GetPetRequest, PetPatch, PetstoreAPI, UpdatePetRequest
from transports import AiohttpFetch, AsyncHttpxFetch, HttpxFetch, RequestsFetch

seen = Queue()


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *args):
        pass

    def do_GET(self):
        self.respond()

    def do_PATCH(self):
        self.respond()

    def respond(self):
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


async def async_checks(url):
    async with aiohttp.ClientSession(
        headers={"Authorization": "Bearer demo"}, timeout=aiohttp.ClientTimeout(total=5)
    ) as session:
        fetch = AiohttpFetch(session, url)
        await asynchronous(bind_async(PetstoreAPI, fetch=fetch), fetch)
    async with httpx.AsyncClient(
        headers={"Authorization": "Bearer demo"}, timeout=5, trust_env=False
    ) as session:
        fetch = AsyncHttpxFetch(session, url)
        await asynchronous(bind_async(PetstoreAPI, fetch=fetch), fetch)


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
        with httpx.Client(
            headers={"Authorization": "Bearer demo"}, timeout=5, trust_env=False
        ) as session:
            fetch = HttpxFetch(session, url)
            sync(bind(PetstoreAPI, fetch=fetch), fetch)
        asyncio.run(async_checks(url))
    finally:
        server.shutdown()
        thread.join()
print(
    "all transports preserve encoded URLs, body omission, and repeated response headers"
)
