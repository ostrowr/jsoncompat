"""Behavioral tests run against freshly generated models by openapi_client.rs."""

import asyncio
import inspect
import unittest
from typing import Protocol
from unittest.mock import patch
from urllib.parse import parse_qs, urlsplit

import petstore as api
from jsoncompat import MISSING, ValidationError
from jsoncompat.http import (
    ContractError,
    Cookie,
    Header,
    Path,
    Query,
    RawResponse,
    Response,
    ResponseError,
    bind,
    bind_async,
    endpoint,
)
from jsoncompat.http._parameters import decode_header, encode_parameter

PET = b'{"id":"p_123","name":"Mochi","species":"cat","tags":[]}'
PROBLEM = b'{"code":"missing","message":"No such pet"}'


def response(status=200, body=PET, headers=()):
    return RawResponse(
        status, (("Content-Type", "application/json"), ("ETag", '"r1"'), *headers), body
    )


class Recorder:
    def __init__(self, reply=None):
        self.reply = response() if reply is None else reply
        self.requests = []

    def __call__(self, request):
        self.requests.append(request)
        return self.reply


class BindingTests(unittest.TestCase):
    def setUp(self):
        self.fetch = Recorder()
        self.client = bind(api.Petstore, fetch=self.fetch)

    def test_shared_models_and_missing(self):
        result = self.client.get_pet(api.GetPetInput(**{"pet_id": "p_123"}))
        self.assertIs(type(result), api.GetPet200)
        self.assertIs(type(result.body), api.Pet)
        self.assertIs(result.body.nickname, MISSING)
        self.assertEqual(result.headers.etag, '"r1"')
        self.assertEqual(self.fetch.requests[-1].target, "/pets/p_123")

    def test_encoded_path_and_false_query(self):
        self.client.get_pet(api.GetPetInput(pet_id="a/b ?#%é", include_history=False))
        self.assertEqual(
            self.fetch.requests[-1].target,
            "/pets/a%2Fb%20%3F%23%25%C3%A9?includeHistory=false",
        )

    def test_query_arrays_objects_cookies_headers(self):
        self.fetch.reply = response(
            body=b'{"items":[],"nextOffset":null}', headers=(("X-Total-Count", "0"),)
        )
        result = self.client.list_pets(
            api.ListPetsInput(
                limit=1,
                offset=0,
                tags=["a&b", "x y"],
                filter=api.PetFilter(adopted=False, species="cat"),
                locale="fr",
                x_request_id="trace",
            )
        )
        wire = self.fetch.requests[-1]
        self.assertEqual(
            parse_qs(urlsplit(wire.target).query),
            {
                "limit": ["1"],
                "offset": ["0"],
                "tags": ["a&b", "x y"],
                "filter[adopted]": ["false"],
                "filter[species]": ["cat"],
            },
        )
        self.assertIn(("Cookie", "locale=fr"), wire.headers)
        self.assertIn(("X-Request-ID", "trace"), wire.headers)
        self.assertEqual(result.headers.x_total_count, 0)

    def test_content_parameters(self):
        self.fetch.reply = response(
            body=b"[]",
            headers=(("X-Selected-Fields", "name"), ("X-Selected-Fields", "species")),
        )
        result = self.client.search_pets(
            api.SearchPetsInput(
                criteria=api.SearchCriteria(name="a&b"),
                fields=["name", "species"],
                x_flags=["a", "b"],
            )
        )
        wire = self.fetch.requests[-1]
        self.assertEqual(
            parse_qs(urlsplit(wire.target).query)["criteria"], ['{"name":"a&b"}']
        )
        self.assertIn("fields=name|species", wire.target)
        self.assertIn(("X-Flags", "a,b"), wire.headers)
        self.assertEqual(result.headers.x_selected_fields, ["name", "species"])

    def test_absent_and_explicit_null_body(self):
        self.client.update_pet(api.UpdatePetInput(pet_id="p_123"))
        self.assertIsNone(self.fetch.requests[-1].body)
        self.assertNotIn("Content-Type", dict(self.fetch.requests[-1].headers))
        self.client.update_pet(
            api.UpdatePetInput(pet_id="p_123", body=api.PetPatch(nickname=None))
        )
        self.assertEqual(self.fetch.requests[-1].body, b'{"nickname":null}')
        self.fetch.reply = response(body=b"null")
        self.client.set_pet_note(
            api.SetPetNoteInput(
                pet_id="p_123",
                body=api.SetPetNoteBodyApplicationJson(
                    content_type="application/json", value=None
                ),
            )
        )
        self.assertEqual(self.fetch.requests[-1].body, b"null")

    def test_body_media_variants(self):
        self.fetch.reply = response(body=b'"hi"')
        self.client.set_pet_note(
            api.SetPetNoteInput(
                pet_id="p_123",
                body=api.SetPetNoteBodyTextPlain(
                    content_type="text/plain", value="héllo"
                ),
            )
        )
        self.assertEqual(self.fetch.requests[-1].body, "héllo".encode())
        self.assertIn(("Content-Type", "text/plain"), self.fetch.requests[-1].headers)

    def test_response_media_variants(self):
        self.fetch.reply = RawResponse(
            200, (("content-type", "Text/Plain; charset=UTF-8"),), "héllo".encode()
        )
        result = self.client.get_pet_note(api.GetPetNoteInput(pet_id="p_123"))
        self.assertIs(type(result), api.GetPetNote200TextPlain)
        self.assertEqual(result.body, "héllo")
        self.fetch.reply = response(body=b"null")
        result = self.client.get_pet_note(api.GetPetNoteInput(pet_id="p_123"))
        self.assertIs(type(result), api.GetPetNote200ApplicationJson)
        self.assertIsNone(result.body)

    def test_no_content_and_conditional_response(self):
        for method, input_model, status, expected in [
            ("delete_pet", api.DeletePetInput, 204, api.DeletePet204),
            ("get_pet", api.GetPetInput, 304, api.GetPet304),
        ]:
            self.fetch.reply = RawResponse(status, (), b"")
            result = getattr(self.client, method)(input_model(pet_id="p_123"))
            self.assertIs(type(result), expected)
            self.assertIsNone(result.body)

    def test_status_precedence_and_errors_are_values(self):
        for status, body, expected in [
            (200, b'{"status":"ok"}', api.Health200),
            (202, b'{"status":"starting"}', api.Health2XX),
            (503, PROBLEM, api.Health5XX),
            (418, b"", api.HealthDefault),
        ]:
            self.fetch.reply = response(status, body)
            result = self.client.health(api.HealthInput())
            self.assertIs(type(result), expected)
            self.assertEqual(result.status, status)
        self.fetch.reply = response(404, PROBLEM)
        self.assertIsInstance(
            self.client.get_pet(api.GetPetInput(pet_id="missing")), api.GetPet404
        )

    def test_response_contract_errors_retain_raw_response(self):
        for raw in [
            response(500, PROBLEM),
            response(body=b"not json"),
            response(body=b"{}"),
            RawResponse(200, (), PET),
            response(headers=(("content-type", "application/json"),)),
            RawResponse(200, (("content-type", "text/html"),), b"x"),
            response(headers=(("etag", '"duplicate"'),)),
            RawResponse(200, (("Content-Type", "application/json"),), PET),
            RawResponse(
                200,
                (
                    ("Content-Type", "application/json; charset=latin-1"),
                    ("ETag", '"r1"'),
                ),
                PET,
            ),
        ]:
            with self.subTest(raw=raw):
                self.fetch.reply = raw
                with self.assertRaises(ResponseError) as caught:
                    self.client.get_pet(api.GetPetInput(pet_id="p_123"))
                self.assertIs(caught.exception.response, raw)
                self.assertIsNotNone(caught.exception.__cause__)
        self.fetch.reply = RawResponse(204, (), b"unexpected")
        with self.assertRaises(ResponseError):
            self.client.delete_pet(api.DeletePetInput(pet_id="p_123"))

    def test_raw_headers_preserve_duplicates(self):
        self.fetch.reply = response(
            headers=(("Set-Cookie", "a=1"), ("Set-Cookie", "b=2"))
        )
        result = self.client.get_pet(api.GetPetInput(pet_id="p_123"))
        self.assertEqual(
            result.raw_headers[-2:], (("Set-Cookie", "a=1"), ("Set-Cookie", "b=2"))
        )

    def test_validation_precedes_network(self):
        bad = api.GetPetInput(pet_id="", skip_validation=True)
        with self.assertRaises(ValidationError):
            self.client.get_pet(bad)
        with self.assertRaises(TypeError):
            self.client.get_pet(api.DeletePetInput(pet_id="x"))
        with self.assertRaises(ValueError):
            self.client.get_pet(
                api.GetPetInput(pet_id="x", if_none_match="a\r\ninjected: true")
            )
        self.assertEqual(self.fetch.requests, [])

    def test_transport_exceptions_propagate(self):
        failure = TimeoutError("transport timeout")

        def fetch(request):
            raise failure

        client = bind(api.Petstore, fetch=fetch)
        with self.assertRaises(TimeoutError) as caught:
            client.get_pet(api.GetPetInput(pet_id="x"))
        self.assertIs(caught.exception, failure)

    def test_cached_methods_and_fetcher_isolation(self):
        other_fetch = Recorder(response(404, PROBLEM))
        with patch(
            "jsoncompat.http._binding.get_type_hints",
            side_effect=AssertionError("unexpected reflection"),
        ):
            other = bind(api.Petstore, fetch=other_fetch)
            self.assertIs(type(other), type(self.client))
            self.assertIsInstance(
                other.get_pet(api.GetPetInput(pet_id="x")), api.GetPet404
            )
            self.assertIsInstance(
                self.client.get_pet(api.GetPetInput(pet_id="x")), api.GetPet200
            )
        self.assertEqual(len(self.fetch.requests), 1)
        self.assertEqual(len(other_fetch.requests), 1)
        self.assertEqual(
            tuple(inspect.signature(self.client.get_pet).parameters), ("request",)
        )

    def test_security_and_servers_are_metadata(self):
        self.client.get_pet(api.GetPetInput(pet_id="x"))
        endpoint = self.fetch.requests[-1].operation
        self.assertEqual(endpoint.security[0][0].scheme, "bearerAuth")
        self.assertEqual(endpoint.servers[0].variables, (("port", "8000"),))
        self.assertNotIn("Authorization", dict(self.fetch.requests[-1].headers))
        self.assertEqual(api.SECURITY_SCHEMES["bearerAuth"]["scheme"], "bearer")

    def test_first_bind_reuses_prepared_models(self):
        class Fresh(api.Petstore, Protocol):
            pass

        with (
            patch("zlib.decompress", side_effect=AssertionError("schema expansion")),
            patch(
                "jsoncompat.validator_for",
                side_effect=AssertionError("schema compilation"),
            ),
        ):
            client = bind(Fresh, fetch=self.fetch)
            result = client.get_pet(api.GetPetInput(pet_id="p_123"))
        self.assertIsInstance(result, api.GetPet200)

    def test_static_path_encoding_and_dot_segment_rejection(self):
        class UnicodePath(Protocol):
            @endpoint(
                method="GET",
                path="/préfix%20x/{petId}",
                responses=(Response(200, api.GetPet200, "application/json"),),
            )
            def get(self, request: api.GetPetInput, /) -> api.GetPet200: ...

        client = bind(UnicodePath, fetch=self.fetch)
        client.get(api.GetPetInput(pet_id="a/b"))
        self.assertEqual(self.fetch.requests[-1].target, "/pr%C3%A9fix%20x/a%2Fb")
        for value in (".", ".."):
            with self.assertRaises(ValueError):
                client.get(api.GetPetInput(pet_id=value))

    def test_inherited_protocol_methods(self):
        class Derived(api.Petstore, Protocol):
            pass

        client = bind(Derived, fetch=self.fetch)
        self.assertIsInstance(
            client.get_pet(api.GetPetInput(pet_id="x")), api.GetPet200
        )

    def test_incomplete_protocols_fail_binding(self):
        class Empty(Protocol):
            pass

        class MissingMetadata(Protocol):
            def get(self, request: api.GetPetInput, /) -> api.GetPetResponse: ...

        class ExtraMember(api.Petstore, Protocol):
            name: str

        class WrongReturn(Protocol):
            @endpoint(
                method="GET",
                path="/pets/{petId}",
                responses=(Response(200, api.GetPet200, "application/json"),),
            )
            def get(self, request: api.GetPetInput, /) -> api.GetPet404: ...

        for protocol in [
            Empty,
            MissingMetadata,
            ExtraMember,
            WrongReturn,
            object,
            api.AsyncPetstore,
        ]:
            with self.subTest(protocol=protocol), self.assertRaises(ContractError):
                bind(protocol, fetch=self.fetch)


class ParameterTests(unittest.TestCase):
    def test_path_styles(self):
        cases = [
            (Path("id"), [1, 2], "1,2"),
            (Path("id", style="label", explode=True), [1, 2], ".1.2"),
            (Path("id", style="matrix", explode=True), [1, 2], ";id=1;id=2"),
            (
                Path("id", style="matrix", explode=True),
                {"a": "x y", "b": False},
                ";a=x%20y;b=false",
            ),
            (Path("id", explode=True), {"a": 1, "b": 2}, "a=1,b=2"),
            (Path("id"), {"a": 1, "b": 2}, "a,1,b,2"),
        ]
        for metadata, value, expected in cases:
            with self.subTest(metadata=metadata, value=value):
                self.assertEqual(encode_parameter(metadata, value), expected)

    def test_query_and_cookie_styles(self):
        cases = [
            (Query("q", explode=False), [1, 2], [("q", "1,2")]),
            (
                Query("q", style="spaceDelimited", explode=False),
                [1, 2],
                [("q", "1%202")],
            ),
            (Query("q", style="deepObject"), {"x": "a&b"}, [("q%5Bx%5D", "a%26b")]),
            (Query("q"), {"x": 1, "y": False}, [("x", "1"), ("y", "false")]),
            (Cookie("q", explode=False), [1, 2], [("q", "1,2")]),
        ]
        for metadata, value, expected in cases:
            self.assertEqual(encode_parameter(metadata, value), expected)

    def test_unrepresentable_values_are_rejected(self):
        for value in [None, [None], [[1]], {"x": {"y": 1}}]:
            with self.subTest(value=value), self.assertRaises(ValueError):
                encode_parameter(Query("q"), value)
        with self.assertRaises(ValueError):
            encode_parameter(Header("Bad Name"), "x")
        with self.assertRaises(ValueError):
            encode_parameter(Header("X-Value"), "x\r\n")
        self.assertEqual(
            encode_parameter(Query("q", content_type="application/json"), None),
            [("q", "null")],
        )

    def test_json_headers_escape_unicode_portably(self):
        metadata = Header("X-JSON", content_type="application/json")
        encoded = encode_parameter(metadata, {"text": "猫😀"})
        self.assertTrue(encoded.isascii())
        self.assertEqual(decode_header(metadata, [encoded]), {"text": "猫😀"})
        with self.assertRaisesRegex(ValueError, "ASCII"):
            encode_parameter(Header("X-Text"), "é")

    def test_typed_header_decoding(self):
        self.assertEqual(
            decode_header(
                Header("Set-Cookie", array=True),
                ["a=1; Expires=Wed, 21 Oct 2026 07:28:00 GMT", "b=2"],
            ),
            ["a=1; Expires=Wed, 21 Oct 2026 07:28:00 GMT", "b=2"],
        )
        self.assertEqual(decode_header(Header("x", kind="boolean"), ["false"]), False)
        self.assertEqual(
            decode_header(Header("x", kind="integer", array=True), ["1,2", "3"]),
            [1, 2, 3],
        )
        self.assertEqual(
            decode_header(Header("x", content_type="application/json"), ['{"a":1}']),
            {"a": 1},
        )
        for kind, value in [
            ("integer", "1.0"),
            ("integer", "+1"),
            ("boolean", "False"),
            ("number", "null"),
        ]:
            with self.assertRaises(ValueError):
                decode_header(Header("x", kind=kind), [value])


class AsyncTests(unittest.IsolatedAsyncioTestCase):
    async def test_awaited_dispatch_and_typed_response(self):
        requests = []

        async def fetch(request):
            requests.append(request)
            await asyncio.sleep(0)
            return response()

        client = bind_async(api.AsyncPetstore, fetch=fetch)
        self.assertTrue(inspect.iscoroutinefunction(client.get_pet))
        result = await client.get_pet(api.GetPetInput(pet_id="x"))
        self.assertIs(type(result), api.GetPet200)
        self.assertEqual(len(requests), 1)
        with self.assertRaises(ContractError):
            bind_async(api.Petstore, fetch=fetch)

    async def test_cancellation_and_transport_failure(self):
        for failure in [asyncio.CancelledError(), TimeoutError("deadline")]:

            async def fetch(request):
                raise failure

            client = bind_async(api.AsyncPetstore, fetch=fetch)
            with self.assertRaises(type(failure)) as caught:
                await client.get_pet(api.GetPetInput(pet_id="x"))
            self.assertIs(caught.exception, failure)


if __name__ == "__main__":
    unittest.main()
