"""Behavioral tests run against freshly generated models by openapi_client.rs."""

import asyncio
import inspect
import unittest
from concurrent.futures import ThreadPoolExecutor
from contextlib import contextmanager
from dataclasses import FrozenInstanceError, replace
from unittest.mock import patch
from urllib.parse import parse_qs, urlsplit

import other_petstore as other_api
import petstore as api
from jsoncompat import MISSING, ValidationError
from jsoncompat.http import (
    API,
    ClientResponse,
    ContractError,
    Cookie,
    Err,
    Header,
    HTTPStatusError,
    Ok,
    Path,
    Query,
    RawResponse,
    Request,
    RequestOptions,
    ResponseError,
    UnexpectedResponse,
    bind,
    bind_async,
    prepare,
)
from jsoncompat.http._parameters import decode_header, encode_parameter
from jsoncompat.http import _binding

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
        self.transports = []

    def __call__(self, request, /, *, transport=None):
        self.requests.append(request)
        self.transports.append(transport)
        return self.reply


@contextmanager
def operations(*specs):
    with (
        patch.object(api.PetstoreAPI, "__jsoncompat_http_plans__", None),
        patch.object(api.PetstoreAPI, "operations", return_value=specs),
    ):
        yield


class BindingTests(unittest.TestCase):
    def setUp(self):
        self.fetch = Recorder()
        self.client = bind(api.PetstoreAPI, fetch=self.fetch)

    def test_generated_dispatch_equals_reflective_contract_preparation(self):
        # Compare every field, header, selector, media type, and escaped path.
        # The existing wire/response tests exercise these same immutable plans.
        for spec in api.PetstoreAPI.operations():
            with self.subTest(operation=spec.endpoint.operation_id):
                emitted = api.PetstoreAPI.__jsoncompat_http_plans__[spec.request]
                reflected = _binding._prepare(spec, api.PetstoreAPI)
                self.assertEqual(emitted, reflected)
                self.assertIs(emitted.endpoint, spec.endpoint)

    def test_generated_registry_rejects_wrong_version_duplicates_and_foreign_api(self):
        plans = tuple(api.PetstoreAPI.__jsoncompat_http_plans__.values())
        foreign = tuple(other_api.PetstoreAPI.__jsoncompat_http_plans__.values())
        for version, entries in (
            (2, plans),
            (1, ()),
            (1, (plans[0], plans[0])),
            (1, (plans[0], foreign[0])),
        ):
            with (
                self.subTest(version=version, entries=entries),
                patch.object(api.PetstoreAPI, "__jsoncompat_http_plans__", None),
            ):
                with self.assertRaises(ContractError):
                    _binding.install_generated_api(api.PetstoreAPI, version, entries)
                self.assertIsNone(api.PetstoreAPI.__jsoncompat_http_plans__)

        # A subclass must never inherit the parent API's nominal registration.
        class OtherAPI(api.PetstoreAPI):
            pass

        with self.assertRaises(ContractError):
            _binding.install_generated_api(OtherAPI, 1, plans)
        self.assertNotIn("__jsoncompat_http_plans__", OtherAPI.__dict__)

    def test_result_identity_immutability_and_unwrap(self):
        result = self.client.request(api.GetPetRequest(pet_id="x")).result()
        self.assertIs(type(result), Ok)
        self.assertIs(result.unwrap(), result.value)
        self.assertIs(result.unwrap(), result.unwrap())
        with self.assertRaises(FrozenInstanceError):
            result.value = result.value
        raw = response(404, PROBLEM)
        self.fetch.reply = raw
        result = self.client.request(api.GetPetRequest(pet_id="x")).result()
        self.assertIs(type(result), Err)
        self.assertIs(result.response, raw)
        match result:
            case Err(error=api.GetPet404(body=problem)):
                self.assertEqual(problem.message, "No such pet")
            case _:
                self.fail("expected a typed 404")
        for _ in range(2):
            with self.assertRaises(HTTPStatusError) as caught:
                result.unwrap()
            self.assertIs(caught.exception.error, result.error)
            self.assertIs(caught.exception.response, raw)
            self.assertEqual(str(caught.exception), "HTTP status 404")
        self.assertEqual(len(self.fetch.requests), 2)
        with self.assertRaises(FrozenInstanceError):
            result.error = result.error

    def test_shared_models_and_missing(self):
        result = self.client.request(api.GetPetRequest(**{"pet_id": "p_123"})).unwrap()
        self.assertIs(type(result), api.GetPet200)
        self.assertIs(type(result.body), api.Pet)
        self.assertIs(result.body.nickname, MISSING)
        self.assertEqual(result.headers.etag, '"r1"')
        self.assertEqual(self.fetch.requests[-1].target, "/pets/p_123")

    def test_lazy_response_headers_and_cached_success(self):
        raw = response(headers=(("Set-Cookie", "a=1"), ("set-cookie", "b=2")))
        self.fetch.reply = raw
        with patch.object(
            api.GetPet200, "from_value", wraps=api.GetPet200.from_value
        ) as decode:
            pending = self.client.request(api.GetPetRequest(pet_id="x"))
            self.assertIs(type(pending), ClientResponse)
            self.assertIs(pending.raw, raw)
            self.assertIs(pending.content, raw.body)
            self.assertEqual(pending.status, 200)
            self.assertEqual(pending.headers["ETAG"], '"r1"')
            self.assertEqual(pending.headers.get_all("SET-cookie"), ("a=1", "b=2"))
            self.assertEqual(pending.headers["set-cookie"], "a=1")
            self.assertIsNone(pending.headers.get("missing"))
            self.assertEqual(pending.headers.get_all("missing"), ())
            self.assertEqual(len(pending.headers), 3)
            self.assertEqual(
                pending.headers,
                {
                    "Content-Type": "application/json",
                    "ETag": '"r1"',
                    "Set-Cookie": "a=1",
                },
            )
            self.assertEqual(
                list(pending.headers), ["Content-Type", "ETag", "Set-Cookie"]
            )
            self.assertIs(pending.headers.raw, raw.headers)
            with self.assertRaises(KeyError):
                pending.headers["missing"]
            with self.assertRaises(TypeError):
                pending.headers["ETag"] = "changed"
            with self.assertRaises(FrozenInstanceError):
                pending.raw = raw
            decode.assert_not_called()
            # A response can be shared across threads without decoding twice.
            with ThreadPoolExecutor(max_workers=4) as pool:
                results = list(pool.map(lambda _: pending.result(), range(12)))
            self.assertTrue(all(result is results[0] for result in results))
            self.assertIs(pending.unwrap(), results[0].value)
            self.assertEqual(decode.call_count, 1)
        self.assertEqual(len(self.fetch.requests), 1)

    def test_lazy_response_caches_success_contract_failure(self):
        raw = response(body=b"{}")
        self.fetch.reply = raw
        with patch.object(
            api.GetPet200, "from_value", wraps=api.GetPet200.from_value
        ) as decode:
            pending = self.client.request(api.GetPetRequest(pet_id="x"))
            self.assertEqual(pending.headers.get("etag"), '"r1"')
            decode.assert_not_called()
            failures = []
            for method in (pending.result, pending.unwrap, pending.result):
                with self.assertRaises(ResponseError) as caught:
                    method()
                failures.append(caught.exception)
            self.assertTrue(all(error is failures[0] for error in failures))
            self.assertIs(failures[0].response, raw)
            self.assertIsNotNone(failures[0].__cause__)
            self.assertEqual(decode.call_count, 1)

    def test_unexpected_errors_preserve_wire_data_and_decode_failure(self):
        cases = (
            (
                response(503, b'{"message":"Overloaded"}', (("Retry-After", "10"),)),
                "undeclared HTTP status",
            ),
            (
                RawResponse(
                    503, (("Content-Type", "text/html"),), b"<h1>Unavailable</h1>"
                ),
                "undeclared HTTP status",
            ),
            (response(404, b"not json"), "GET /pets/{petId}"),
            (response(404, b"{}"), "GET /pets/{petId}"),
            (response(404, b"\xff"), "GET /pets/{petId}"),
            (
                RawResponse(404, (("Content-Type", "text/plain"),), b"Not found"),
                "undeclared response media",
            ),
            (
                response(404, PROBLEM, (("Content-Type", "application/json"),)),
                "multiple Content-Type",
            ),
            (RawResponse(304, (), b"unexpected body"), "declares no body"),
        )
        for raw, reason in cases:
            with self.subTest(raw=raw):
                self.fetch.reply = raw
                pending = self.client.request(api.GetPetRequest(pet_id="x"))
                result = pending.result()
                match result:
                    case Err(error=UnexpectedResponse(status=status) as error):
                        self.assertEqual(status, raw.status)
                    case _:
                        self.fail("expected an unmodeled HTTP error")
                self.assertIs(result.response, raw)
                self.assertIs(error.response, raw)
                self.assertIs(error.body, raw.body)
                self.assertIs(error.headers.raw, raw.headers)
                self.assertIn(reason, str(error.decode_error))
                self.assertIsNotNone(error.decode_error.__cause__)
                self.assertIs(pending.result(), result)
                with self.assertRaises(HTTPStatusError) as caught:
                    pending.unwrap()
                self.assertIs(caught.exception.error, error)
                self.assertIs(caught.exception.response, raw)
                with self.assertRaises(FrozenInstanceError):
                    error.decode_error = error.decode_error

    def test_unexpected_body_helpers_are_explicit_and_do_not_require_json_media(self):
        for payload, expected in (
            (b"null", None),
            (b"42", 42),
            (b"[]", []),
            (b'{"x":true}', {"x": True}),
        ):
            raw = RawResponse(503, (), payload)
            result = prepare(api.PetstoreAPI, api.GetPetRequest(pet_id="x")).decode(raw)
            self.assertEqual(result.error.json(), expected)
            self.assertEqual(result.error.text(), payload.decode())
        for payload in (b"<html>Unavailable</html>", b"", b"\xff", b"NaN"):
            error = (
                prepare(api.PetstoreAPI, api.GetPetRequest(pet_id="x"))
                .decode(RawResponse(503, (), payload))
                .error
            )
            with self.assertRaises(ValueError):
                error.json()
            self.assertEqual(error.text(), payload.decode(errors="replace"))
        self.assertEqual(error.text("latin-1"), "NaN")
        error = (
            prepare(api.PetstoreAPI, api.GetPetRequest(pet_id="x"))
            .decode(RawResponse(503, (), b"caf\xe9"))
            .error
        )
        self.assertEqual(error.text("latin-1"), "café")
        with self.assertRaises(UnicodeDecodeError):
            error.text(errors="strict")

    def test_invalid_raw_contract_and_undeclared_success_still_raise(self):
        for raw in (
            response(True),
            response(99),
            response(600),
            response(503, "not bytes"),
            response(201),
        ):
            pending = ClientResponse(
                raw, prepare(api.PetstoreAPI, api.GetPetRequest(pet_id="x")).decode
            )
            with self.subTest(raw=raw), self.assertRaises(ResponseError) as caught:
                pending.result()
            self.assertIs(caught.exception.response, raw)
        self.fetch.reply = object()
        with self.assertRaises(TypeError):
            self.client.request(api.GetPetRequest(pet_id="x"))

    def test_invalid_ranged_and_default_errors_fall_back_without_retrying_selection(
        self,
    ):
        for raw in (response(503, b"{}"), response(418, PROBLEM)):
            self.fetch.reply = raw
            result = self.client.request(api.HealthRequest()).result()
            self.assertIsInstance(result.error, UnexpectedResponse)
            self.assertIs(result.response, raw)
        # A failed range must not fall through to the valid empty default.
        self.fetch.reply = RawResponse(503, (), b"")
        self.assertIsInstance(
            self.client.request(api.HealthRequest()).result().error, UnexpectedResponse
        )
        # Likewise, an exact success cannot fall through to the empty default.
        self.fetch.reply = RawResponse(200, (), b"")
        with self.assertRaises(ResponseError):
            self.client.request(api.HealthRequest()).result()

    def test_encoded_path_and_false_query(self):
        self.client.request(
            api.GetPetRequest(pet_id="a/b ?#%é", include_history=False)
        ).unwrap()
        self.assertEqual(
            self.fetch.requests[-1].target,
            "/pets/a%2Fb%20%3F%23%25%C3%A9?includeHistory=false",
        )

    def test_query_arrays_objects_cookies_headers(self):
        self.fetch.reply = response(
            body=b'{"items":[],"nextOffset":null}', headers=(("X-Total-Count", "0"),)
        )
        result = self.client.request(
            api.ListPetsRequest(
                limit=1,
                offset=0,
                tags=["a&b", "x y"],
                filter=api.PetFilter(adopted=False, species="cat"),
                locale="fr",
                x_request_id="trace",
            )
        ).unwrap()
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
        result = self.client.request(
            api.SearchPetsRequest(
                criteria=api.SearchCriteria(name="a&b"),
                fields=["name", "species"],
                x_flags=["a", "b"],
            )
        ).unwrap()
        wire = self.fetch.requests[-1]
        self.assertEqual(
            parse_qs(urlsplit(wire.target).query)["criteria"], ['{"name":"a&b"}']
        )
        self.assertIn("fields=name|species", wire.target)
        self.assertIn(("X-Flags", "a,b"), wire.headers)
        self.assertEqual(result.headers.x_selected_fields, ["name", "species"])

    def test_absent_and_explicit_null_body(self):
        self.client.request(api.UpdatePetRequest(pet_id="p_123")).unwrap()
        self.assertIsNone(self.fetch.requests[-1].body)
        self.assertNotIn("Content-Type", dict(self.fetch.requests[-1].headers))
        self.client.request(
            api.UpdatePetRequest(pet_id="p_123", body=api.PetPatch(nickname=None))
        ).unwrap()
        self.assertEqual(self.fetch.requests[-1].body, b'{"nickname":null}')
        self.fetch.reply = response(body=b"null")
        self.client.request(
            api.SetPetNoteRequest(
                pet_id="p_123",
                body=api.SetPetNoteBodyApplicationJson(
                    content_type="application/json", value=None
                ),
            )
        ).unwrap()
        self.assertEqual(self.fetch.requests[-1].body, b"null")

    def test_body_media_variants(self):
        self.fetch.reply = response(body=b'"hi"')
        self.client.request(
            api.SetPetNoteRequest(
                pet_id="p_123",
                body=api.SetPetNoteBodyTextPlain(
                    content_type="text/plain", value="héllo"
                ),
            )
        ).unwrap()
        self.assertEqual(self.fetch.requests[-1].body, "héllo".encode())
        self.assertIn(("Content-Type", "text/plain"), self.fetch.requests[-1].headers)

    def test_response_media_variants(self):
        self.fetch.reply = RawResponse(
            200, (("content-type", "Text/Plain; charset=UTF-8"),), "héllo".encode()
        )
        result = self.client.request(api.GetPetNoteRequest(pet_id="p_123")).unwrap()
        self.assertIs(type(result), api.GetPetNote200TextPlain)
        self.assertEqual(result.body, "héllo")
        self.fetch.reply = response(body=b"null")
        result = self.client.request(api.GetPetNoteRequest(pet_id="p_123")).unwrap()
        self.assertIs(type(result), api.GetPetNote200ApplicationJson)
        self.assertIsNone(result.body)

    def test_no_content_and_conditional_response(self):
        for input_model, status, expected in [
            (api.DeletePetRequest, 204, api.DeletePet204),
            (api.GetPetRequest, 304, api.GetPet304),
        ]:
            self.fetch.reply = RawResponse(status, (), b"")
            result = self.client.request(input_model(pet_id="p_123")).result()
            self.assertIs(type(result), Ok if status == 204 else Err)
            result = result.value if isinstance(result, Ok) else result.error
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
            result = self.client.request(api.HealthRequest()).result()
            self.assertIs(type(result), Ok if 200 <= status < 300 else Err)
            result = result.value if isinstance(result, Ok) else result.error
            self.assertIs(type(result), expected)
            self.assertEqual(result.status, status)
        self.fetch.reply = response(404, PROBLEM)
        self.assertIsInstance(
            self.client.request(api.GetPetRequest(pet_id="missing")).result().error,
            api.GetPet404,
        )

    def test_response_contract_errors_retain_raw_response(self):
        for raw in [
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
                    self.client.request(api.GetPetRequest(pet_id="p_123")).unwrap()
                self.assertIs(caught.exception.response, raw)
                self.assertIsNotNone(caught.exception.__cause__)
        self.fetch.reply = RawResponse(204, (), b"unexpected")
        with self.assertRaises(ResponseError):
            self.client.request(api.DeletePetRequest(pet_id="p_123")).unwrap()

    def test_raw_headers_preserve_duplicates(self):
        self.fetch.reply = response(
            headers=(("Set-Cookie", "a=1"), ("Set-Cookie", "b=2"))
        )
        result = self.client.request(api.GetPetRequest(pet_id="p_123")).unwrap()
        self.assertEqual(
            result.raw_headers[-2:], (("Set-Cookie", "a=1"), ("Set-Cookie", "b=2"))
        )

    def test_validation_precedes_network(self):
        bad = api.GetPetRequest(pet_id="", skip_validation=True)
        with self.assertRaises(ValidationError):
            self.client.request(bad).unwrap()
        with self.assertRaises(TypeError):
            self.client.request(object()).unwrap()
        with self.assertRaises(ValueError):
            self.client.request(
                api.GetPetRequest(pet_id="x", if_none_match="a\r\ninjected: true")
            ).unwrap()
        self.assertEqual(self.fetch.requests, [])

    def test_transport_exceptions_propagate(self):
        failure = TimeoutError("transport timeout")

        def fetch(request, /, *, transport=None):
            raise failure

        client = bind(api.PetstoreAPI, fetch=fetch)
        with self.assertRaises(TimeoutError) as caught:
            client.request(api.GetPetRequest(pet_id="x")).unwrap()
        self.assertIs(caught.exception, failure)

    def test_transport_options_are_forwarded_by_identity_for_one_call(self):
        model = api.GetPetRequest(pet_id="p_123")
        transport = object()  # The binder imposes no option class or schema.
        result = self.client.request(
            model,
            transport=transport,
            options=RequestOptions(extra_headers={"X-Trace-ID": "one"}),
        ).unwrap()
        self.assertIs(type(result), api.GetPet200)
        self.assertIs(self.fetch.transports[-1], transport)
        self.assertIn(("X-Trace-ID", "one"), self.fetch.requests[-1].headers)
        self.client.request(model).unwrap()
        self.assertIsNone(self.fetch.transports[-1])
        self.assertNotIn(("X-Trace-ID", "one"), self.fetch.requests[-1].headers)
        self.client.request(model, transport=None).unwrap()
        self.assertEqual(self.fetch.transports, [transport, None, None])
        with self.assertRaises(TypeError):
            self.client.request(
                other_api.GetPetRequest(pet_id="x"), transport=transport
            ).unwrap()
        invalid = api.GetPetRequest(pet_id="", skip_validation=True)
        with self.assertRaises(ValidationError):
            self.client.request(invalid, transport=transport).unwrap()
        self.assertEqual(len(self.fetch.transports), 3)

    def test_fetch_type_errors_propagate_without_retry(self):
        calls = []
        failure = TypeError("raised inside the fetcher")

        def fetch(request, /, *, transport=None):
            calls.append((request, transport))
            raise failure

        client = bind(api.PetstoreAPI, fetch=fetch)
        with self.assertRaises(TypeError) as caught:
            client.request(api.GetPetRequest(pet_id="x")).unwrap()
        self.assertIs(caught.exception, failure)
        self.assertEqual(len(calls), 1)
        self.assertIsNone(calls[0][1])

    def test_cached_plans_and_fetcher_isolation(self):
        other_fetch = Recorder(response(404, PROBLEM))
        with patch(
            "jsoncompat.http._binding.get_type_hints",
            side_effect=AssertionError("unexpected reflection"),
        ):
            other = bind(api.PetstoreAPI, fetch=other_fetch)
            self.assertIs(type(other), type(self.client))
            self.assertIsInstance(
                other.request(api.GetPetRequest(pet_id="x")).result().error,
                api.GetPet404,
            )
            self.assertIsInstance(
                self.client.request(api.GetPetRequest(pet_id="x")).unwrap(),
                api.GetPet200,
            )
        self.assertEqual(len(self.fetch.requests), 1)
        self.assertEqual(len(other_fetch.requests), 1)
        self.assertEqual(
            tuple(inspect.signature(self.client.request).parameters),
            ("request", "options", "transport"),
        )
        self.assertIs(
            inspect.signature(self.client.request).parameters["options"].kind,
            inspect.Parameter.KEYWORD_ONLY,
        )

    def test_security_and_servers_are_metadata(self):
        self.client.request(api.GetPetRequest(pet_id="x")).unwrap()
        endpoint = self.fetch.requests[-1].operation
        self.assertEqual(endpoint.security[0][0].scheme, "bearerAuth")
        self.assertEqual(endpoint.servers[0].variables, (("port", "8000"),))
        self.assertNotIn("Authorization", dict(self.fetch.requests[-1].headers))
        self.assertEqual(api.SECURITY_SCHEMES["bearerAuth"]["scheme"], "bearer")

    def test_first_bind_reuses_prepared_models(self):
        with (
            patch("zlib.decompress", side_effect=AssertionError("schema expansion")),
            patch(
                "jsoncompat.validator_for",
                side_effect=AssertionError("schema compilation"),
            ),
            patch(
                "jsoncompat.http._binding.get_type_hints",
                side_effect=AssertionError("annotation inspection"),
            ),
        ):
            client = bind(api.PetstoreAPI, fetch=self.fetch)
            result = client.request(api.GetPetRequest(pet_id="p_123")).unwrap()
        self.assertIsInstance(result, api.GetPet200)
        self.assertFalse(hasattr(client, "get_pet"))

    def test_static_path_encoding_and_dot_segment_rejection(self):
        spec = next(
            s for s in api.PetstoreAPI.operations() if s.request is api.GetPetRequest
        )
        spec = replace(
            spec, endpoint=replace(spec.endpoint, path="/préfix%20x/{petId}")
        )
        with operations(spec):
            client = bind(api.PetstoreAPI, fetch=self.fetch)
            client.request(api.GetPetRequest(pet_id="a/b")).unwrap()
            self.assertEqual(self.fetch.requests[-1].target, "/pr%C3%A9fix%20x/a%2Fb")
            for value in (".", ".."):
                with self.assertRaises(ValueError):
                    client.request(api.GetPetRequest(pet_id=value)).unwrap()

    def test_api_identity_and_registration_precede_network(self):
        self.assertEqual(api.PetstoreAPI.__name__, other_api.PetstoreAPI.__name__)
        self.assertIsNot(api.PetstoreAPI, other_api.PetstoreAPI)

        class Unregistered(
            Request[api.PetstoreAPI, api.GetPetSuccess, api.GetPetError]
        ):
            __slots__ = ()

        for request in (
            other_api.GetPetRequest(pet_id="x"),
            object.__new__(Unregistered),
            api.GetPetRequest,
            object(),
        ):
            with (
                self.subTest(request=request),
                self.assertRaisesRegex(TypeError, "not a registered request"),
            ):
                self.client.request(request).unwrap()
        self.assertEqual(self.fetch.requests, [])
        other = bind(other_api.PetstoreAPI, fetch=self.fetch)
        result = other.request(other_api.GetPetRequest(pet_id="x")).unwrap()
        self.assertIs(type(result), other_api.GetPet200)
        with self.assertRaises(TypeError):
            other.request(api.GetPetRequest(pet_id="x")).result().error
        self.assertEqual(len(self.fetch.requests), 1)

    def test_incomplete_api_contracts_fail_binding(self):
        for identity in (API, object, api.PetstoreAPI()):
            with self.subTest(identity=identity), self.assertRaises(ContractError):
                bind(identity, fetch=self.fetch)
        spec = next(
            s for s in api.PetstoreAPI.operations() if s.request is api.GetPetRequest
        )
        malformed = [
            (),
            (object(),),
            (spec, spec),
            (other_api.PetstoreAPI.operations()[0],),
            (replace(spec, success=api.GetPet404),),
            (replace(spec, success=spec.error, error=spec.success),),
            (replace(spec, endpoint=replace(spec.endpoint, path="/missing/{wrong}")),),
            (replace(spec, endpoint=replace(spec.endpoint, responses=())),),
        ]
        for specs in malformed:
            with (
                self.subTest(specs=specs),
                operations(*specs),
                self.assertRaises(ContractError),
            ):
                bind(api.PetstoreAPI, fetch=self.fetch)
        with self.assertRaises(TypeError):
            bind(fetch=self.fetch)

    def test_subclass_cannot_reuse_another_api_plan(self):
        class OtherAPI(api.PetstoreAPI):
            pass

        with self.assertRaisesRegex(ContractError, "must be a Request scoped to"):
            bind(OtherAPI, fetch=self.fetch)
        self.assertNotIn("__jsoncompat_http_plans__", OtherAPI.__dict__)
        self.assertEqual(self.fetch.requests, [])


class PreparationTests(unittest.TestCase):
    def setUp(self):
        self.fetch = Recorder()
        self.client = bind(api.PetstoreAPI, fetch=self.fetch)

    def test_options_snapshot_mutable_inputs(self):
        headers = {"X-Trace-ID": "original"}
        query = [["preview", "true"]]
        removed = ["Accept"]
        options = RequestOptions(
            extra_headers=headers, extra_query=query, remove_headers=removed
        )
        headers["X-Trace-ID"] = "changed"
        query[0][1] = "changed"
        removed.clear()
        call = prepare(api.PetstoreAPI, api.GetPetRequest(pet_id="x"), options=options)
        self.assertEqual(call.request.headers, (("X-Trace-ID", "original"),))
        self.assertEqual(call.request.target, "/pets/x?preview=true")
        with self.assertRaises(FrozenInstanceError):
            options.extra_headers = {}
        with self.assertRaises(FrozenInstanceError):
            call.request = call.request

    def test_headers_replace_case_insensitively_preserve_repetitions_and_do_not_leak(
        self,
    ):
        request = api.GetPetRequest(pet_id="x", if_none_match='"old"')
        options = RequestOptions(
            extra_headers=(
                ("if-none-match", '"new"'),
                ("X-Repeat", "a"),
                ("x-repeat", "b"),
            ),
            remove_headers=("ACCEPT", "IF-NONE-MATCH"),
        )
        result = self.client.request(request, options=options).unwrap()
        self.assertIs(type(result), api.GetPet200)
        self.assertEqual(self.fetch.requests[-1].headers, options.extra_headers)
        self.assertEqual(request.if_none_match, '"old"')
        self.client.request(request).unwrap()
        self.assertIn(("If-None-Match", '"old"'), self.fetch.requests[-1].headers)
        self.assertIn("Accept", dict(self.fetch.requests[-1].headers))
        self.assertNotIn("X-Repeat", dict(self.fetch.requests[-1].headers))

    def test_query_append_and_remove_preserve_original_encoding(self):
        request = api.ListPetsRequest(
            tags=["a&b", "x y"], offset=0, filter=api.PetFilter(species="cat")
        )
        original = prepare(api.PetstoreAPI, request).request.target
        call = prepare(
            api.PetstoreAPI,
            request,
            options=RequestOptions(
                extra_query=(("tags", "猫 +%/"), ("tags", ""), ("feature name", "a&b"))
            ),
        )
        self.assertEqual(
            call.request.target,
            original + "&tags=%E7%8C%AB%20%2B%25%2F&tags=&feature%20name=a%26b",
        )
        replaced = prepare(
            api.PetstoreAPI,
            request,
            options=RequestOptions(
                remove_query=("tags", "offset", "filter[species]"),
                extra_query={"tags": "new"},
            ),
        )
        self.assertEqual(replaced.request.target, "/pets?tags=new")
        case_sensitive = prepare(
            api.PetstoreAPI, request, options=RequestOptions(remove_query=("Tags",))
        )
        self.assertEqual(case_sensitive.request.target, original)
        empty = prepare(
            api.PetstoreAPI,
            api.GetPetRequest(pet_id="x", include_history=False),
            options=RequestOptions(remove_query=("includeHistory",)),
        )
        self.assertEqual(empty.request.target, "/pets/x")

    def test_transform_runs_last_and_keeps_the_selected_decoder(self):
        seen = []

        def transform(request):
            seen.append(request)
            return replace(
                request,
                method="PATCH",
                target="/custom?raw=%2f+ok",
                body=b'{"experimental":true}',
                headers=request.headers + (("Content-Type", "application/json"),),
            )

        options = RequestOptions(
            extra_headers={"X-Trace-ID": "trace"},
            extra_query={"preview": "true"},
            remove_headers=("Accept",),
            transform=transform,
        )
        model = api.GetPetRequest(pet_id="x")
        call = prepare(api.PetstoreAPI, model, options=options)
        self.assertEqual(len(seen), 1)
        self.assertEqual(seen[0].target, "/pets/x?preview=true")
        self.assertEqual(seen[0].headers, (("X-Trace-ID", "trace"),))
        self.assertEqual(self.fetch.requests, [])
        self.assertIs(call.api, api.PetstoreAPI)
        self.assertEqual(call.request.body, b'{"experimental":true}')
        self.assertEqual(call.request.target, "/custom?raw=%2f+ok")
        self.assertIs(
            type(call.decode(self.fetch(call.request)).unwrap()), api.GetPet200
        )
        self.assertIs(
            type(self.client.request(model, options=options).unwrap()), api.GetPet200
        )
        self.assertEqual(self.fetch.requests[0], self.fetch.requests[1])

    def test_identity_transform_preserves_parameter_encoding(self):
        model = api.SearchPetsRequest(
            criteria=api.SearchCriteria(name="a&b"),
            fields=["name", "species"],
            x_flags=["a", "b"],
        )
        original = prepare(api.PetstoreAPI, model)
        transformed = prepare(
            api.PetstoreAPI,
            model,
            options=RequestOptions(transform=lambda request: request),
        )
        self.assertEqual(transformed.request, original.request)
        self.assertIn("name|species", transformed.request.target)

    def test_prepared_decoder_validates_independently_and_preserves_raw_failures(
        self,
    ):
        call = prepare(api.PetstoreAPI, api.GetPetRequest(pet_id="x"))
        with patch.object(api.PetstoreAPI, "__jsoncompat_http_plans__", None):
            self.assertIs(type(call.decode(response()).unwrap()), api.GetPet200)
        self.assertIs(type(call.decode(response(404, PROBLEM)).error), api.GetPet404)
        for raw in (response(body=b"not json"), response(body=b"{}")):
            with self.assertRaises(ResponseError) as caught:
                call.decode(raw).unwrap()
            self.assertIs(caught.exception.response, raw)
        with self.assertRaises(TypeError):
            call.decode(b"not a RawResponse").unwrap()
        self.assertEqual(self.fetch.requests, [])

    def test_identity_and_model_validation_precede_transforms(self):
        def unexpected(request):
            self.fail("transform ran before validation")

        options = RequestOptions(transform=unexpected)
        for model, error in (
            (other_api.GetPetRequest(pet_id="x"), TypeError),
            (api.GetPetRequest(pet_id="", skip_validation=True), ValidationError),
            (api.GetPetRequest, TypeError),
        ):
            with self.assertRaises(error):
                prepare(api.PetstoreAPI, model, options=options)
        with self.assertRaises(ContractError):
            prepare(API, api.GetPetRequest(pet_id="x"), options=options)

    def test_invalid_options_fail_before_fetching(self):
        for kwargs in (
            {"extra_headers": {"X": 1}},
            {"extra_query": (("x", None),)},
            {"extra_headers": ["ab"]},
            {"remove_headers": "Accept"},
            {"remove_query": (1,)},
            {"transform": 1},
        ):
            with self.subTest(kwargs=kwargs), self.assertRaises(TypeError):
                RequestOptions(**kwargs)
        for options in (
            {},
            RequestOptions(extra_headers={"Bad Name": "x"}),
            RequestOptions(extra_headers={"X": "bad\r\nvalue"}),
            RequestOptions(extra_headers={"X": "猫"}),
            RequestOptions(remove_headers=("Bad Name",)),
        ):
            with (
                self.subTest(options=options),
                self.assertRaises((TypeError, ValueError)),
            ):
                self.client.request(
                    api.GetPetRequest(pet_id="x"), options=options
                ).unwrap()
        self.assertEqual(self.fetch.requests, [])

    def test_malformed_transforms_fail_before_fetching(self):
        transforms = [
            lambda request: None,
            lambda request: replace(
                request, operation=replace(request.operation, path="/other")
            ),
            lambda request: replace(request, method="GET\r\nX"),
            lambda request: replace(request, body="not bytes"),
            lambda request: replace(request, headers=(("X", "bad\nvalue"),)),
        ]
        for target in (
            "https://other.example/",
            "//other.example/",
            "/a#fragment",
            "/a b",
            "/猫",
            "/bad%",
            "/bad%2g",
            "/a/../b",
            "/bad\\path",
            "/bad\r\n",
        ):
            transforms.append(
                lambda request, target=target: replace(request, target=target)
            )
        for transform in transforms:
            with (
                self.subTest(transform=transform),
                self.assertRaises((TypeError, ValueError)),
            ):
                self.client.request(
                    api.GetPetRequest(pet_id="x"),
                    options=RequestOptions(transform=transform),
                ).unwrap()
        self.assertEqual(self.fetch.requests, [])

    def test_transform_exception_propagates(self):
        failure = RuntimeError("could not customize")

        def transform(request):
            raise failure

        with self.assertRaises(RuntimeError) as caught:
            self.client.request(
                api.GetPetRequest(pet_id="x"),
                options=RequestOptions(transform=transform),
            ).unwrap()
        self.assertIs(caught.exception, failure)
        self.assertEqual(self.fetch.requests, [])


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
    async def test_unexpected_error_is_decoded_only_on_demand(self):
        calls = []
        raw = response(404, b"{}")

        async def fetch(request, /, *, transport=None):
            calls.append(request)
            return raw

        client = bind_async(api.PetstoreAPI, fetch=fetch)
        with patch.object(
            api.GetPet404, "from_value", wraps=api.GetPet404.from_value
        ) as decode:
            pending = await client.request(api.GetPetRequest(pet_id="x"))
            self.assertIs(pending.raw, raw)
            self.assertEqual(pending.headers.get("content-type"), "application/json")
            decode.assert_not_called()
            result = pending.result()
            self.assertIsInstance(result.error, UnexpectedResponse)
            self.assertIs(pending.result(), result)
            with self.assertRaises(HTTPStatusError):
                pending.unwrap()
            self.assertEqual(decode.call_count, 1)
            self.assertEqual(len(calls), 1)

    async def test_concurrent_options_are_isolated_and_preparation_is_transport_independent(
        self,
    ):
        wires = []
        transports = []
        first, second = object(), object()
        gate = asyncio.Event()

        async def fetch(request, /, *, transport=None):
            wires.append(request)
            transports.append(transport)
            if len(wires) == 2:
                gate.set()
            await gate.wait()
            return response()

        client = bind_async(api.PetstoreAPI, fetch=fetch)
        model = api.GetPetRequest(pet_id="x")
        results = await asyncio.wait_for(
            asyncio.gather(
                client.request(
                    model,
                    options=RequestOptions(extra_headers={"X-Trace-ID": "one"}),
                    transport=first,
                ),
                client.request(
                    model,
                    options=RequestOptions(extra_headers={"X-Trace-ID": "two"}),
                    transport=second,
                ),
            ),
            timeout=5,
        )
        self.assertTrue(
            all(
                type(result.result()) is Ok and type(result.unwrap()) is api.GetPet200
                for result in results
            )
        )
        self.assertEqual(
            [dict(wire.headers)["X-Trace-ID"] for wire in wires], ["one", "two"]
        )
        self.assertIs(transports[0], first)
        self.assertIs(transports[1], second)
        (await client.request(model)).unwrap()
        self.assertIsNone(transports[-1])
        call = prepare(api.PetstoreAPI, model)
        self.assertIs(
            type(call.decode(await fetch(call.request)).unwrap()), api.GetPet200
        )
        self.assertNotIn("X-Trace-ID", dict(wires[-1].headers))
        with self.assertRaises(TypeError):
            (
                await client.request(
                    other_api.GetPetRequest(pet_id="x"),
                    options=RequestOptions(),
                    transport=first,
                )
            ).unwrap()
        self.assertEqual(len(wires), 4)

    async def test_awaited_dispatch_and_typed_response(self):
        requests = []

        async def fetch(request, /, *, transport=None):
            requests.append(request)
            await asyncio.sleep(0)
            return response()

        client = bind_async(api.PetstoreAPI, fetch=fetch)
        self.assertTrue(inspect.iscoroutinefunction(client.request))
        result = (await client.request(api.GetPetRequest(pet_id="x"))).unwrap()
        self.assertIs(type(result), api.GetPet200)
        self.assertEqual(len(requests), 1)
        with self.assertRaisesRegex(TypeError, "not a registered request"):
            (await client.request(other_api.GetPetRequest(pet_id="x"))).unwrap()
        self.assertEqual(len(requests), 1)

    async def test_cancellation_and_transport_failure(self):
        for failure in [
            asyncio.CancelledError(),
            TimeoutError("deadline"),
            TypeError("raised inside the fetcher"),
        ]:
            calls = []
            settings = object()

            async def fetch(request, /, *, transport=None):
                calls.append(transport)
                raise failure

            client = bind_async(api.PetstoreAPI, fetch=fetch)
            with self.assertRaises(type(failure)) as caught:
                (
                    await client.request(
                        api.GetPetRequest(pet_id="x"), transport=settings
                    )
                ).unwrap()
            self.assertIs(caught.exception, failure)
            self.assertEqual(calls, [settings])


if __name__ == "__main__":
    unittest.main()
