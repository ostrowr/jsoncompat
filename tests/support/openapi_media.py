from __future__ import annotations

import asyncio
import base64
from email import policy
from email.parser import BytesParser
from typing import get_args, get_type_hints
from urllib.parse import parse_qs

import media
from jsoncompat.http import RawResponse, ResponseError, UnexpectedResponse, prepare
from jsoncompat.http._binding import _prepare, initialize_api


def body_type(request):
    return get_args(get_type_hints(request, include_extras=True)["body"])[0]


def decode(call, content_type, body, status=200):
    return call.decode(RawResponse(status, (("Content-Type", content_type),), body))


def fails(f, kind=ValueError):
    try:
        f()
    except kind:
        return
    raise AssertionError("expected failure")


api = media.MediaAPI
plans = initialize_api(api)
for spec in api.operations():
    assert plans[spec.request] == _prepare(spec, api)

text = prepare(api, media.TextRequest(body="a,b\n"))
assert text.request.body == b"a,b\n"
assert dict(text.request.headers)["Content-Type"] == 'text/csv; header="present"'
assert (
    decode(text, 'text/html; charset="UTF-8"', b"<h1>Hello</h1>").unwrap().body
    == "<h1>Hello</h1>"
)
fails(lambda: media.TextRequest(body="x"))
fails(lambda: decode(text, "text/html", b"x"), ResponseError)
fails(lambda: decode(text, "text/html; charset=iso-8859-1", b"hello"), ResponseError)
assert decode(text, "text/plain", b"bad input", 400).error.body == "bad input"
assert isinstance(decode(text, "text/plain", b"x", 400).error, UnexpectedResponse)

events = prepare(api, media.EventsRequest())
assert (
    decode(events, "text/event-stream", b'data: {"ok":true}\n\n').unwrap().body
    == 'data: {"ok":true}\n\n'
)

binary = prepare(api, media.BinaryRequest(body=b"\x00\xff\x80abc"))
assert binary.request.body == b"\x00\xff\x80abc"
assert decode(binary, "application/pdf", b"\xff\x00PDF").unwrap().body == b"\xff\x00PDF"
assert (
    media.BinaryRequest(body=b"\x00\xff").to_value()["body"]
    == base64.b64encode(b"\x00\xff").decode()
)
fails(lambda: media.BinaryRequest(body="not bytes"), TypeError)

# Structured media retain JSON schema types even with a binary format annotation.
for request_type, content_type in [
    (media.JsonBinaryAnnotationRequest, "application/json"),
    (media.VendorBinaryAnnotationRequest, "application/vnd.test+json"),
]:
    request = prepare(api, request_type(body="hello"))
    assert request.request.body == b'"hello"'
    assert decode(request, content_type, b'"world"').unwrap().body == "world"
    fails(lambda: request_type(body=b"raw bytes"), TypeError)
    fails(lambda: request_type(body="x"))
    fails(lambda: decode(request, content_type, b"world"), ResponseError)
yaml_string = prepare(api, media.YamlBinaryAnnotationRequest(body="true"))
assert (
    decode(yaml_string, "application/yaml", yaml_string.request.body).unwrap().body
    == "true"
)
ndjson_string = prepare(api, media.NdjsonBinaryAnnotationRequest(body="hello\n"))
assert ndjson_string.request.body == b"hello\n"
assert (
    decode(ndjson_string, "application/x-ndjson", b"world\n").unwrap().body == "world\n"
)

ndjson_model = body_type(media.NdjsonRequest)
ndjson_item = get_args(ndjson_model)[0]
ndjson = prepare(api, media.NdjsonRequest(body=[ndjson_item(id=1), ndjson_item(id=2)]))
assert ndjson.request.body == b'{"id":1}\n{"id":2}\n'
assert [
    item.id
    for item in decode(ndjson, "application/x-ndjson", b'{"id":1}\r\n{"id":2}\n')
    .unwrap()
    .body
] == [1, 2]
fails(lambda: decode(ndjson, "application/x-ndjson", b'{"id":"bad"}\n'), ResponseError)
fails(lambda: decode(ndjson, "application/x-ndjson", b'{"id":1}\n\n'), ResponseError)
raw_ndjson = prepare(api, media.NdjsonTextRequest())
assert (
    decode(raw_ndjson, "application/x-ndjson", b"arbitrary raw text\n").unwrap().body
    == "arbitrary raw text\n"
)

form_model = body_type(media.FormRequest)
form = prepare(
    api,
    media.FormRequest(
        body=form_model(
            name="hello + & 世界",
            tags=["a,b", "c"],
            query=get_type_hints(form_model)["query"].from_value({"nested": 2}),
        )
    ),
)
assert parse_qs(form.request.body.decode()) == {
    "name": ["hello + & 世界"],
    "tags": ["a,b", "c"],
    "nested": ["2"],
}

upload_model = body_type(media.UploadRequest)
options_model = get_type_hints(upload_model)["options"]
upload = prepare(
    api,
    media.UploadRequest(
        body=upload_model(
            file=b"\x00\xffimage", options=options_model(quality=2), tags=["a,b", "x|y"]
        )
    ),
)
content_type = dict(upload.request.headers)["Content-Type"]
message = BytesParser(policy=policy.default).parsebytes(
    ("Content-Type: " + content_type + "\r\n\r\n").encode() + upload.request.body
)
parts = {
    part.get_param("name", header="content-disposition"): part
    for part in message.iter_parts()
}
assert set(parts) == {"file", "options", "tags"}
assert parts["file"].get_payload(decode=True) == b"\x00\xffimage"
assert parts["file"].get_content_type() == "image/png"
assert parts["file"].get_filename() == "file"
assert parts["options"].get_payload(decode=True) == b'{"quality":2}'
assert parts["options"].get_content_type() == "application/json"
assert parts["tags"].get_payload(decode=True) == b"a,b|x|y"
fails(lambda: options_model(quality=0))

yaml_model = body_type(media.YamlRequest)
yaml = prepare(api, media.YamlRequest(body=yaml_model(id=42)))
assert b"id: 42" in yaml.request.body
assert decode(yaml, "application/yaml", b"id: 43\n").unwrap().body.id == 43
fails(lambda: decode(yaml, "application/yaml", b"id: wrong\n"), ResponseError)
fails(lambda: decode(yaml, "application/yaml", b"id: [broken\n"), ResponseError)

parameters = prepare(api, media.ParametersRequest(q="a:/?#[]", blank=""))
assert parameters.request.target == "/parameters?blank=&q=a:/?%23%5B%5D"
wildcard = prepare(api, media.WildcardRequest())
assert decode(wildcard, "application/json", b"3").unwrap().body == 3
assert decode(wildcard, "image/png", b"\xff\x00").unwrap().body == b"\xff\x00"
params = prepare(api, media.MediaParamsRequest())
assert (
    decode(params, "text/csv; header=present; charset=UTF-8", b"a,b").unwrap().body
    == "a,b"
)
fails(lambda: decode(params, "text/csv; header=absent", b"a,b"), ResponseError)

from jsoncompat.http import Body, Response

assert Body("text/plain").codec == "text"
assert Response(200, media.Text200, "text/plain").codec == "text"
scalar_text = prepare(api, media.ScalarTextRequest(body=3))
assert scalar_text.request.body == b"3"
assert decode(scalar_text, "text/plain", b"true").unwrap().body is True
fails(lambda: media.ScalarTextRequest(body=1))
fails(lambda: decode(scalar_text, "text/plain", b'"true"'), ResponseError)
array_records = prepare(api, media.ArrayRecordsRequest(body=[[1, 2], [3]]))
assert array_records.request.body == b"[1,2]\n[3]\n"
assert list(
    decode(array_records, "application/x-ndjson", b"[1,2]\n").unwrap().body[0]
) == [1, 2]
fails(lambda: media.ArrayRecordsRequest(body=[[1, 2, 3]]))
fails(
    lambda: decode(ndjson, "application/x-ndjson", b'{"id":1}\v{"id":2}'), ResponseError
)
fails(lambda: decode(wildcard, "*/*", b"bytes"), ResponseError)
fails(lambda: media.SiblingFormRequest.from_value({"body": {"name": "alone"}}))
sibling_form = prepare(
    api, media.SiblingFormRequest.from_value({"body": {"name": "ok", "extra": "yes"}})
)
assert parse_qs(sibling_form.request.body.decode()) == {
    "name": ["ok"],
    "extra": ["yes"],
}
multiple_files = prepare(
    api,
    media.UploadRequest(
        body=upload_model(
            file=b"a",
            options=options_model(quality=1),
            tags=[],
            files=[b"\xff", b"\x00"],
            optionalFile=None,
        )
    ),
)
message = BytesParser(policy=policy.default).parsebytes(
    (
        "Content-Type: "
        + dict(multiple_files.request.headers)["Content-Type"]
        + "\r\n\r\n"
    ).encode()
    + multiple_files.request.body
)
files = [
    part.get_payload(decode=True)
    for part in message.iter_parts()
    if part.get_param("name", header="content-disposition") == "files"
]
assert files == [b"\xff", b"\x00"]

structured_parts = prepare(
    api,
    media.UploadRequest(
        body=upload_model(
            file=b"raw",
            options=options_model(quality=1),
            tags=[],
            jsonText="true",
            yamlText="true",
        )
    ),
)
message = BytesParser(policy=policy.default).parsebytes(
    (
        "Content-Type: "
        + dict(structured_parts.request.headers)["Content-Type"]
        + "\r\n\r\n"
    ).encode()
    + structured_parts.request.body
)
parts = {
    part.get_param("name", header="content-disposition"): part
    for part in message.iter_parts()
}
assert parts["file"].get_payload(decode=True) == b"raw"
assert parts["jsonText"].get_payload(decode=True) == b'"true"'
assert parts["jsonText"].get_filename() is None
from jsoncompat.codegen.serialization import deserialize_value, SerializationFormat

assert (
    deserialize_value(
        parts["yamlText"].get_payload(decode=True), format=SerializationFormat.YAML
    )
    == "true"
)

print("media codecs, validation, reflection, and binary models passed")

# Exercise the exact byte bodies through each caller-owned local transport.
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from threading import Thread
import aiohttp
import httpx
import requests
from jsoncompat.http import bind, bind_async
from jsoncompat.http.aiohttp import AiohttpFetch
from jsoncompat.http.httpx import HttpxFetch, AsyncHttpxFetch
from jsoncompat.http.requests import RequestsFetch

received = []


class Handler(BaseHTTPRequestHandler):
    def do_POST(self):
        body = self.rfile.read(int(self.headers.get("Content-Length", "0")))
        received.append((self.path, self.headers.get("Content-Type"), body))
        if self.path == "/binary":
            media_type, response = "application/pdf", body
        elif self.path == "/ndjson":
            media_type, response = "application/x-ndjson", body
        else:
            media_type, response = "application/json", b"null"
        self.send_response(200)
        self.send_header("Content-Type", media_type)
        self.send_header("Content-Length", str(len(response)))
        self.end_headers()
        self.wfile.write(response)

    def log_message(self, *args):
        pass


server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
thread = Thread(target=server.serve_forever, daemon=True)
thread.start()
base_url = f"http://127.0.0.1:{server.server_port}"
requests_to_send = [
    media.BinaryRequest(body=b"\x00\xff\x80abc"),
    media.FormRequest(
        body=form_model(
            name="a + 世界",
            tags=["a&b"],
            query=get_type_hints(form_model)["query"].from_value({"k": 2}),
        )
    ),
    media.UploadRequest(
        body=upload_model(
            file=b"\xff\x00image", options=options_model(quality=2), tags=["a|b", "c"]
        )
    ),
    media.NdjsonRequest(body=[ndjson_item(id=9)]),
    media.ParametersRequest(q="a:/?#[]", blank=""),
]


def check_exchange(index, response):
    # Metadata/content are available while no model has been parsed.
    assert response.status == 200
    assert isinstance(response.content, bytes)
    value = response.unwrap()
    if index == 0:
        assert value.body == b"\x00\xff\x80abc"
    elif index == 3:
        assert value.body[0].id == 9
    else:
        assert value.body is None


try:
    for session, adapter in [
        (requests.Session(), RequestsFetch),
        (httpx.Client(), HttpxFetch),
    ]:
        with session:
            client = bind(api, fetch=adapter(session, base_url))
            for i, req in enumerate(requests_to_send):
                check_exchange(i, client.request(req))

    async def run_async():
        async with aiohttp.ClientSession() as session:
            client = bind_async(api, fetch=AiohttpFetch(session, base_url))
            for i, req in enumerate(requests_to_send):
                check_exchange(i, await client.request(req))
        async with httpx.AsyncClient() as session:
            client = bind_async(api, fetch=AsyncHttpxFetch(session, base_url))
            for i, req in enumerate(requests_to_send):
                check_exchange(i, await client.request(req))

    asyncio.run(run_async())
finally:
    server.shutdown()
    server.server_close()
    thread.join()
assert len(received) == 20
for offset in range(0, 20, 5):
    batch = received[offset : offset + 5]
    assert batch[0][2] == b"\x00\xff\x80abc"
    assert parse_qs(batch[1][2].decode()) == {
        "name": ["a + 世界"],
        "tags": ["a&b"],
        "k": ["2"],
    }
    message = BytesParser(policy=policy.default).parsebytes(
        ("Content-Type: " + batch[2][1] + "\r\n\r\n").encode() + batch[2][2]
    )
    parts = list(message.iter_parts())
    assert any(part.get_payload(decode=True) == b"\xff\x00image" for part in parts)
    assert batch[3][2] == b'{"id":9}\n'
    assert batch[4][0] == "/parameters?blank=&q=a:/?%23%5B%5D"
print("requests, httpx sync/async, and aiohttp local wire exchanges passed")
