"""Native HTTP conversion must accept, reject, and explain like the value path."""

import json
import sys
from unittest.mock import patch

sys.path.insert(0, sys.argv[1])
import http_native as api
from jsoncompat import deserialize_json_value
from jsoncompat.http import Ok, RawResponse, bind, prepare


class ValuePath:
    def __init__(self, runtime):
        self.runtime = runtime

    def __getattr__(self, name):
        return getattr(self.runtime, name)

    def _deserialize_json_field(self, fields, name, payload):
        return None

    def _serialize_json_field(self, instance, name):
        raise TypeError("reference value path")


def failure(error):
    return (
        type(error), str(error), getattr(error, "instance_path", None),
        getattr(error, "schema_path", None), getattr(error, "keyword", None),
        failure(error.__cause__) if error.__cause__ else None,
    )


def outcome(action):
    try:
        result = action()
        if isinstance(result, Ok):
            return type(result.value), result.value.to_value(skip_validation=True)
        return result
    except (ValueError, TypeError) as error:
        return failure(error)


cases = {
    "Integer": [b"0", b"-1", b"1.0", b"1.0000000000000000001", b"1.5",
                b"9007199254740993.0", b"123456789012345678901234567890", b"1e309"],
    "Number": [b"0.1", b"0.3", b"0.09999999999999999999999",
               b"0.30000000000000000000001", b"-0.0", b"NaN", b"Infinity"],
    "Bag": [b"{}", b'{"fixed":1,"extra":0.25}', b'{"fixed":1.5}',
            b'{"extra":1e309}', b'{"extra":1,"extra":2}', b'{"extra":"x"}'],
    "Union": [b'"hello"', b'""', b'{"kind":"list","items":[0,1,2]}',
              b'{"kind":"list","items":[-1]}', b'{"kind":"other","items":[]}'],
    "Array": [b"[]", b"[0.5,null,2]", b"[9007199254740993.0]", b"[NaN]", b'["x"]'],
    "Anything": [b"null", b'"h\xc3\xa9llo"', b'{"x":[1,true,null,{"y":2.5}]}',
                 b'{"x":1,"x":2}', b"1.0000000000000000001", b"1e309"],
}

for name, bodies in cases.items():
    request_type = getattr(api, f"{name}Request")
    response_type = getattr(api, f"{name}200")
    # All schemas admit the first body. Construct requests through conversion
    # so root wrappers and inline/union models need no special test handling.
    request = request_type.from_value({"body": deserialize_json_value(bodies[0])})
    for body in bodies + [b"invalid", bodies[0] + b' ,"status":200', bodies[0] + b" null"]:
        for count in ("3", "invalid"):
            raw = RawResponse(200, (("Content-Type", "application/json"), ("X-Count", count), ("X-Map", '{"z":1,"a":2}')), body)
            client = bind(api.NativeAPI, fetch=lambda request, transport=None: raw)
            actual = outcome(lambda: client.request(request).result())
            with patch.object(response_type, "__jsoncompat_runtime__", ValuePath(response_type.__jsoncompat_runtime__)):
                expected = outcome(lambda: client.request(request).result())
            assert actual == expected, (name, body, count, actual, expected)

    for body in bodies:
        try:
            request = request_type.from_value({"body": deserialize_json_value(body)}, skip_validation=True)
        except (ValueError, TypeError):
            continue  # Invalid JSON or unrepresentable shapes cannot form a request.
        actual = outcome(lambda: prepare(api.NativeAPI, request).request)
        with patch.object(request_type, "__jsoncompat_runtime__", ValuePath(request_type.__jsoncompat_runtime__)):
            expected = outcome(lambda: prepare(api.NativeAPI, request).request)
        assert actual == expected, (name, body, actual, expected)

request = api.IntegerRequest.from_value({"body": 1, "filter": {"z": 1, "a": 2}})
actual = prepare(api.NativeAPI, request).request
with patch.object(api.IntegerRequest, "__jsoncompat_runtime__", ValuePath(api.IntegerRequest.__jsoncompat_runtime__)):
    assert actual == prepare(api.NativeAPI, request).request
assert actual.target.endswith("?filter%5Bz%5D=1&filter%5Ba%5D=2"), actual
raw = RawResponse(200, (("Content-Type", "application/json"), ("X-Map", '{"z":1,"a":2}')), b"1")
client = bind(api.NativeAPI, fetch=lambda request, transport=None: raw)
assert list(client.request(request).unwrap().headers.x_map.to_value()) == ["z", "a"]

for content_type, value in (("application/json", {"fixed": 1, "extra": 2.5}), ("text/plain", "héllo")):
    request = api.VariantsRequest.from_value({"body": {"content_type": content_type, "value": value}})
    actual = prepare(api.NativeAPI, request).request
    with patch.object(api.VariantsRequest, "__jsoncompat_runtime__", ValuePath(api.VariantsRequest.__jsoncompat_runtime__)):
        assert actual == prepare(api.NativeAPI, request).request
    assert actual.body == (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode() if content_type == "application/json" else value.encode())

print("Native HTTP body semantics match the reference value path.")
