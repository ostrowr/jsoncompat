use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};
#[path = "support/python_env.rs"]
mod python_env;

struct Work(PathBuf);
impl Work {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "jsoncompat-http-{}-{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn generate(&self, spec: &Value, extra: &[&str]) -> Output {
        self.generate_package(spec, "Petstore", "petstore", extra)
    }
    fn generate_package(&self, spec: &Value, api: &str, package: &str, extra: &[&str]) -> Output {
        fs::write(self.0.join("api.json"), spec.to_string()).unwrap();
        Command::new(env!("CARGO_BIN_EXE_jsoncompat"))
            .args([
                "codegen",
                "--openapi",
                "api.json",
                "--api",
                api,
                "--output",
                package,
            ])
            .args(extra)
            .current_dir(&self.0)
            .output()
            .unwrap()
    }
}
impl Drop for Work {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn example() -> Value {
    serde_json::from_str(include_str!("../examples/openapi_client/openapi.json")).unwrap()
}
fn success(output: Output) {
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
fn copy_python_examples(destination: &Path) {
    for entry in
        fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/openapi_client")).unwrap()
    {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|extension| extension == "py") {
            fs::copy(&path, destination.join(path.file_name().unwrap())).unwrap();
        }
    }
}

fn jsonplaceholder_example() -> Work {
    let work = Work::new();
    let spec = serde_json::from_str(include_str!(
        "../examples/openapi_client/jsonplaceholder.json"
    ))
    .unwrap();
    success(work.generate_package(&spec, "JsonPlaceholder", "jsonplaceholder", &[]));
    copy_python_examples(&work.0);
    work
}

#[test]
fn jsonplaceholder_spec_and_live_test_preserve_static_types() {
    let work = jsonplaceholder_example();
    fs::write(
        work.0.join("pyrightconfig.json"),
        json!({
            "pythonVersion": "3.12",
            "typeCheckingMode": "strict",
            "extraPaths": [Path::new(env!("CARGO_MANIFEST_DIR")).join("pybindings")],
            "include": ["jsonplaceholder", "live_jsonplaceholder.py"]
        })
        .to_string(),
    )
    .unwrap();
    success(
        python_env::pyright_command()
            .current_dir(&work.0)
            .output()
            .unwrap(),
    );
}

#[test]
#[ignore = "makes 24 HTTPS requests to JSONPlaceholder; run just python-http-live"]
fn jsonplaceholder_live_all_transports() {
    let work = jsonplaceholder_example();
    let status = python_env::python_command()
        .current_dir(&work.0)
        .arg("live_jsonplaceholder.py")
        .status()
        .unwrap();
    assert!(status.success(), "JSONPlaceholder live smoke test failed");
}

#[test]
fn generated_requests_encode_decode_and_bind_without_generated_clients() {
    let work = Work::new();
    success(work.generate(&example(), &[]));
    assert!(work.0.join("petstore/py.typed").is_file());
    success(work.generate_package(&example(), "Petstore", "other_petstore", &[]));
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/python/openapi_client_contract.py"),
        work.0.join("test_contract.py"),
    )
    .unwrap();
    success(
        python_env::python_command()
            .current_dir(&work.0)
            .arg("test_contract.py")
            .arg("-v")
            .output()
            .unwrap(),
    );
}
#[test]
fn all_example_transports_work_against_the_local_server() {
    let work = Work::new();
    success(work.generate(&example(), &[]));
    copy_python_examples(&work.0);
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/python/openapi_client_wire.py"),
        work.0.join("wire.py"),
    )
    .unwrap();
    for file in [
        "aiohttp_client.py",
        "httpx_client.py",
        "requests_client.py",
        "prepared_client.py",
        "wire.py",
    ] {
        success(
            python_env::python_command()
                .current_dir(&work.0)
                .arg(file)
                .output()
                .unwrap(),
        );
    }
}
#[test]
fn api_boundaries_preserve_static_types_and_structural_implementations() {
    let work = Work::new();
    success(work.generate(&example(), &["--models-per-module", "1"]));
    success(work.generate_package(&example(), "Petstore", "other_petstore", &[]));
    copy_python_examples(&work.0);
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/python/openapi_client_transport_typing.py"),
        work.0.join("transport_typing.py"),
    )
    .unwrap();
    fs::write(work.0.join("pyrightconfig.json"),json!({"pythonVersion":"3.12","typeCheckingMode":"strict","extraPaths":[Path::new(env!("CARGO_MANIFEST_DIR")).join("pybindings")],"include":["valid.py","transport_typing.py","petstore","other_petstore","exercise.py","server.py","aiohttp_client.py","httpx_client.py","requests_client.py","prepared_client.py"]}).to_string()).unwrap();
    fs::write(
        work.0.join("valid.py"),
        r#"
from typing import Never, assert_type
from petstore import PetstoreAPI, GetPetRequest, GetPetResult, GetPet200, GetPetError, GetPet304, ListPetsRequest
from jsoncompat.http import API, AsyncClient, Client, PreparedRequest, Request, RequestOptions, Result, Ok, Err, bind, bind_async, prepare, EncodedRequest, RawResponse
from jsoncompat import MISSING

def fetch(request: EncodedRequest, /, *, transport: None = None) -> RawResponse:
    raise NotImplementedError
async def async_fetch(request: EncodedRequest, /, *, transport: None = None) -> RawResponse:
    raise NotImplementedError
client: Client[PetstoreAPI, Never] = bind(PetstoreAPI, fetch=fetch)
assert_type(client, Client[PetstoreAPI, Never])
assert_type(client.request(GetPetRequest(pet_id="a")), GetPetResult)
options = RequestOptions(extra_headers={"X-Trace-ID": "trace"}, extra_query=(("preview", "true"),))
assert_type(client.request(GetPetRequest(pet_id="a"), options=options), GetPetResult)
call = prepare(PetstoreAPI, GetPetRequest(pet_id="a"), options=options)
assert_type(call, PreparedRequest[PetstoreAPI, GetPetResult])
assert_type(call.request, EncodedRequest)
assert_type(call.api, type[PetstoreAPI])
assert_type(call.decode(fetch(call.request)), GetPetResult)
request=ListPetsRequest()
if request.limit is not MISSING:
    assert_type(request.limit, int)
async def example() -> GetPetResult:
    client: AsyncClient[PetstoreAPI, Never] = bind_async(PetstoreAPI, fetch=async_fetch)
    assert_type(client, AsyncClient[PetstoreAPI, Never])
    assert_type(await client.request(GetPetRequest(pet_id="a"), options=options), GetPetResult)
    return call.decode(await async_fetch(call.request))
class Fake:
    def request[S, E](self, request: Request[PetstoreAPI, S, E], /, *, options: RequestOptions | None = None, transport: Never | None = None) -> Result[S, E]:
        raise NotImplementedError
reader: Client[PetstoreAPI, Never] = Fake()
response=reader.request(GetPetRequest(pet_id="a")).unwrap()
assert_type(response, GetPet200)
assert_type(response.body.name, str)
def execute[A: API, S, E](client: Client[A, Never], request: Request[A, S, E]) -> Result[S, E]:
    return client.request(request)
assert_type(execute(client, GetPetRequest(pet_id="a")), GetPetResult)

assert_type(call.decode(fetch(call.request)).unwrap(), GetPet200)
async def unwrap_async() -> GetPet200:
    client: AsyncClient[PetstoreAPI, Never] = bind_async(PetstoreAPI, fetch=async_fetch)
    return (await client.request(GetPetRequest(pet_id="a"))).unwrap()
def inspect_result(result: GetPetResult) -> None:
    assert_type(result.unwrap(), GetPet200)
    if isinstance(result, Ok):
        assert_type(result.value, GetPet200)
    else:
        assert_type(result.error, GetPetError)
        assert_type(result.response, RawResponse)
    match result:
        case Ok(value=pet):
            assert_type(pet, GetPet200)
        case Err(error=error):
            assert_type(error, GetPetError)
            if isinstance(error, GetPet304):
                assert_type(error.body, None)
"#,
    ).unwrap();
    success(
        python_env::pyright_command()
            .current_dir(&work.0)
            .output()
            .unwrap(),
    );
    let invalid = r#"
from typing import Never
from petstore import PetstoreAPI, GetPetRequest, GetPetResult, DeletePet204
from other_petstore import PetstoreAPI as OtherAPI, GetPetRequest as OtherRequest
from jsoncompat.http import API, AsyncClient, Client, PreparedRequest, Request, RequestOptions, Response, Result, bind, operation, prepare
from typing_extensions import TypeForm
from valid import async_fetch, call, client, execute, fetch
from transport_typing import fetch as typed_fetch, async_fetch as typed_async_fetch
from jsoncompat.http.aiohttp import AiohttpOptions
from jsoncompat.http.httpx import HttpxOptions
from jsoncompat.http.requests import RequestsOptions
from jsoncompat.http import EncodedRequest, RawResponse, Fetch, AsyncFetch, bind_async

bind(PetstoreAPI, fetch=async_fetch)  # must fail
client.request(GetPetRequest(pet_id=123))  # must fail
client.request(OtherRequest(pet_id="x"))  # must fail
execute(client, OtherRequest(pet_id="x"))  # must fail
widened: Client[API, Never] = client  # must fail
wrong: Client[OtherAPI, Never] = client  # must fail
bare: Client = client  # must fail
bare_request: Request = GetPetRequest(pet_id="x")  # must fail
wrong_response: DeletePet204 = client.request(GetPetRequest(pet_id="x"))  # must fail
wrong_unwrap: DeletePet204 = client.request(GetPetRequest(pet_id="x")).unwrap()  # must fail
client.request(GetPetRequest(pet_id="x")).body  # must fail
bare_result: Result = client.request(GetPetRequest(pet_id="x"))  # must fail
bind(fetch=fetch)  # must fail
bind(object, fetch=fetch)  # must fail
client.request(GetPetRequest)  # must fail
client.get_pet(GetPetRequest(pet_id="x"))  # must fail
operation(GetPetRequest, TypeForm(DeletePet204), TypeForm(DeletePet204), method="DELETE", path="/pets/{petId}", responses=(Response(204, DeletePet204),))  # must fail
prepare(PetstoreAPI, OtherRequest(pet_id="x"))  # must fail
prepare(OtherAPI, GetPetRequest(pet_id="x"))  # must fail
prepare(GetPetRequest(pet_id="x"))  # must fail
prepared_other: PreparedRequest[OtherAPI, GetPetResult] = call  # must fail
prepared_widened: PreparedRequest[API, GetPetResult] = call  # must fail
prepared_wrong_response: PreparedRequest[PetstoreAPI, DeletePet204] = call  # must fail
prepared_bare: PreparedRequest = call  # must fail
decoded_wrong: DeletePet204 = call.decode(fetch(call.request))  # must fail
call.decode(b"invalid")  # must fail
call.request = call.request  # must fail
RequestOptions(extra_headers={"X-Trace-ID": 123})  # must fail
RequestOptions(extra_query=(("preview", True),))  # must fail
RequestOptions(transform=lambda request: "invalid")  # must fail
RequestOptions(transform=async_fetch)  # must fail
client.request(GetPetRequest(pet_id="x"), options={"extra_headers": {}})  # must fail
client.request(GetPetRequest(pet_id="x"), RequestOptions())  # must fail
async def wrong_async(client: AsyncClient[PetstoreAPI, Never]) -> None:
    await client.request(OtherRequest(pet_id="x"))  # must fail
    await client.request(GetPetRequest(pet_id="x"), transport=HttpxOptions(timeout=2))  # must fail

typed_client = bind(PetstoreAPI, fetch=typed_fetch)
typed_client.request(GetPetRequest(pet_id="x"), transport=RequestsOptions(timeout=2))  # must fail
typed_client.request(GetPetRequest(pet_id="x"), transport={"timeout": 2})  # must fail
client.request(GetPetRequest(pet_id="x"), transport=HttpxOptions(timeout=2))  # must fail
missing_options: Client[PetstoreAPI] = typed_client  # must fail
wrong_options: Client[PetstoreAPI, AiohttpOptions] = typed_client  # must fail
all_options: Client[PetstoreAPI, object] = typed_client  # must fail
wide_api: Client[API, HttpxOptions] = typed_client  # must fail
bare_fetch: Fetch = typed_fetch  # must fail
bare_async_fetch: AsyncFetch = typed_async_fetch  # must fail

def no_keyword(request: EncodedRequest, /) -> RawResponse:
    raise NotImplementedError
def required_option(request: EncodedRequest, /, *, transport: HttpxOptions) -> RawResponse:
    raise NotImplementedError
bind(PetstoreAPI, fetch=no_keyword)  # must fail
bind(PetstoreAPI, fetch=required_option)  # must fail
bind_async(PetstoreAPI, fetch=typed_fetch)  # must fail
async def invalid_options() -> None:
    typed_async = bind_async(PetstoreAPI, fetch=typed_async_fetch)
    await typed_async.request(GetPetRequest(pet_id="x"), transport=HttpxOptions(timeout=2))  # must fail
    await typed_async.request(OtherRequest(pet_id="x"))  # must fail
    missing: AsyncClient[PetstoreAPI] = typed_async  # must fail
"#;
    fs::write(work.0.join("invalid.py"), invalid).unwrap();
    let output = python_env::pyright_command()
        .current_dir(&work.0)
        .arg("--outputjson")
        .arg("invalid.py")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let diagnostics: Value = serde_json::from_slice(&output.stdout).unwrap();
    for (line, source) in invalid
        .lines()
        .enumerate()
        .filter(|(_, source)| source.ends_with("# must fail"))
    {
        assert!(
            diagnostics["generalDiagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|diagnostic| diagnostic["severity"] == "error"
                    && diagnostic["rule"] != "reportUnusedVariable"
                    && diagnostic["range"]["start"]["line"] == line),
            "missing expected type error on {source}: {diagnostics}"
        );
    }
}

#[test]
fn generation_rejects_unsupported_or_ambiguous_contracts_before_writing() {
    let mutations: Vec<(&str, Value, &str)> = vec![
        (
            "/paths/~1pets/post/requestBody/content",
            json!({"multipart/form-data":{"schema":{"type":"object"}}}),
            "unsupported multipart/form-data",
        ),
        (
            "/paths/~1pets/get/parameters/0/allowReserved",
            json!(true),
            "allowReserved",
        ),
        (
            "/paths/~1pets/get/operationId",
            json!("getPet"),
            "duplicate OpenAPI operationId",
        ),
        (
            "/paths/~1pets/get/responses/200/headers/X-Total-Count/schema",
            json!({"type":"object"}),
            "unsupported response header",
        ),
    ];
    for (pointer, value, message) in mutations {
        let work = Work::new();
        let mut spec = example();
        if pointer.ends_with("allowReserved") {
            spec["paths"]["/pets"]["get"]["parameters"][0]["allowReserved"] = value;
        } else {
            *spec.pointer_mut(pointer).unwrap() = value;
        }
        let output = work.generate(&spec, &[]);
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(message),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!work.0.join("petstore/__init__.py").exists());
    }
}
#[test]
fn inline_only_specs_and_parameter_overrides_are_supported() {
    let work = Work::new();
    let spec = json!({"openapi":"3.1.0","info":{"title":"Inline","version":"1"},"paths":{"/items/{id}":{"parameters":[{"name":"id","in":"path","required":true,"schema":{"type":"integer"}},{"name":"id","in":"query","schema":{"type":"integer"}}],"get":{"parameters":[{"name":"id","in":"query","required":true,"schema":{"type":"string"}}],"responses":{"200":{"description":"found","content":{"application/problem+json":{"schema":{"type":"object","properties":{"ok":{"type":"boolean"}},"required":["ok"],"additionalProperties":false}}}}}}}}});
    success(work.generate(&spec, &[]));
    let source = fs::read_to_string(work.0.join("petstore/models_0000.py")).unwrap();
    assert!(source.contains("path_id: typing.Annotated[int"));
    assert!(source.contains("query_id: typing.Annotated[str"));
    assert!(source.contains("application/problem+json"));
    let exports_before = fs::read_to_string(work.0.join("petstore/__init__.pyi")).unwrap();
    assert!(exports_before.contains("import GetItemsById200Body as GetItemsById200Body"));
    let document = jsoncompat::OpenApiDocument::from_json(&spec).unwrap();
    let lowerer = jsoncompat_openapi::OpenApiOperationLowerer::new(&document).unwrap();
    let key = lowerer.operation_keys().unwrap().pop_first().unwrap();
    let lowered = lowerer.lower_operation(&key).unwrap().unwrap();
    assert_eq!(
        lowered.response["properties"]["body"]["properties"]["content_type"]["properties"]["subtype"]
            ["enum"],
        json!(["problem+json"])
    );
    let output = work.generate(&spec, &[]);
    success(output);
    assert_eq!(
        fs::read_to_string(work.0.join("petstore/__init__.pyi")).unwrap(),
        exports_before
    );
}

#[test]
fn default_only_responses_and_component_renames_preserve_contracts() {
    let work = Work::new();
    let spec = json!({"openapi":"3.1.0","info":{"title":"Default","version":"1"},"paths":{"/ping":{"get":{"operationId":"ping","responses":{"default":{"description":"result","content":{"application/json":{"schema":{"$ref":"#/components/schemas/Result"}}}}}}}},"components":{"schemas":{"Result":{"type":"object","properties":{"ok":{"type":"boolean"}},"required":["ok"],"additionalProperties":false}}}});
    success(work.generate(&spec, &["--rename", "Result=Outcome"]));
    let source = fs::read_to_string(work.0.join("petstore/models_0000.py")).unwrap();
    assert!(source.contains("body: Outcome"));
    success(
        python_env::python_command()
            .current_dir(&work.0)
            .arg("-c")
            .arg(
                r#"
from petstore import PetstoreAPI, PingRequest, PingDefault, Outcome
from jsoncompat.http import bind, RawResponse
def fetch(request, /, *, transport=None): return RawResponse(418, (("Content-Type","application/json"),), b'{"ok":false}')
result=bind(PetstoreAPI,fetch=fetch).request(PingRequest())
from jsoncompat.http import Err
assert isinstance(result, Err)
result=result.error
assert isinstance(result,PingDefault) and isinstance(result.body,Outcome)
assert result.status == 418 and result.body.ok is False
"#,
            )
            .output()
            .unwrap(),
    );
    fs::write(work.0.join("petstore/unmanaged.txt"), "keep").unwrap();
    let before = fs::read(work.0.join("petstore/api.py")).unwrap();
    success(work.generate(&spec, &["--rename", "Result=Outcome"]));
    assert_eq!(fs::read(work.0.join("petstore/api.py")).unwrap(), before);
    assert_eq!(
        fs::read_to_string(work.0.join("petstore/unmanaged.txt")).unwrap(),
        "keep"
    );
}

#[test]
fn result_partitions_cover_exact_ranges_defaults_and_never() {
    let work = Work::new();
    let mut paths = serde_json::Map::new();
    for (id, statuses) in [
        ("onlySuccess", vec!["200"]),
        ("onlyError", vec!["404"]),
        ("fallback", vec!["200", "404", "default"]),
        ("ranges", vec!["200", "2XX", "5XX", "default"]),
        (
            "covered",
            vec!["1XX", "2XX", "3XX", "4XX", "5XX", "default"],
        ),
        ("defaultOnly", vec!["default"]),
    ] {
        let responses: serde_json::Map<_, _> = statuses
            .into_iter()
            .map(|status| (status.to_owned(), json!({"description": "no content"})))
            .collect();
        paths.insert(
            format!("/{id}"),
            json!({"get": {"operationId": id, "responses": responses}}),
        );
    }
    let spec = json!({"openapi":"3.1.0", "info":{"title":"Results", "version":"1"}, "paths":paths});
    success(work.generate(&spec, &[]));
    fs::write(
        work.0.join("results.py"),
        r#"
from typing import Never, assert_type
import petstore as api
from jsoncompat.http import Client, Err, HTTPStatusError, Ok, RawResponse, ResponseError, bind

assert api.OnlySuccessError.__value__ is Never
assert api.OnlyErrorSuccess.__value__ is Never
assert api.DefaultOnlySuccess.__value__ is api.DefaultOnlyDefault
assert api.DefaultOnlyError.__value__ is api.DefaultOnlyDefault

def typing_cases(client: Client[api.PetstoreAPI, Never]) -> None:
    assert_type(client.request(api.OnlySuccessRequest()).unwrap(), api.OnlySuccess200)
    # Put a Never-returning expression last so it cannot hide later type checks.
    assert_type(client.request(api.OnlyErrorRequest()).unwrap(), Never)

raw = RawResponse(200, (), b"")
def fetch(*args: object, transport: None = None) -> RawResponse:
    return raw

client: Client[api.PetstoreAPI, Never] = bind(api.PetstoreAPI, fetch=fetch)
for prefix, selectors in (
    ("OnlySuccess", {"200"}),
    ("OnlyError", {"404"}),
    ("Fallback", {"200", "404", "default"}),
    ("Ranges", {"200", "2XX", "5XX", "default"}),
    ("Covered", {"1XX", "2XX", "3XX", "4XX", "5XX", "default"}),
    ("DefaultOnly", {"default"}),
):
    request = getattr(api, prefix + "Request")()
    for status in range(100, 600):
        raw = RawResponse(status, (), b"")
        exact, ranged = str(status), f"{status // 100}XX"
        selector = exact if exact in selectors else ranged if ranged in selectors else "default"
        if selector not in selectors:
            try:
                client.request(request)
            except ResponseError as error:
                assert error.response is raw
            else:
                raise AssertionError((prefix, status, "undeclared response accepted"))
            continue
        expected = getattr(api, prefix + ("Default" if selector == "default" else selector))
        result = client.request(request)
        if 200 <= status < 300:
            assert isinstance(result, Ok) and type(result.value) is expected
            assert result.unwrap() is result.value
        else:
            assert isinstance(result, Err) and type(result.error) is expected
            assert result.response is raw
            try:
                result.unwrap()
            except HTTPStatusError as error:
                assert error.error is result.error and error.response is raw
            else:
                raise AssertionError("non-success unwrap returned")
"#,
    )
    .unwrap();
    success(
        python_env::python_command()
            .current_dir(&work.0)
            .arg("results.py")
            .output()
            .unwrap(),
    );
    // Runtime enumeration intentionally uses reflection; type-check only the
    // statically declared examples and generated packages separately.
    let source = fs::read_to_string(work.0.join("results.py")).unwrap();
    let typing = source
        .split("raw = RawResponse")
        .next()
        .unwrap()
        .replace("from dataclasses import replace\n", "")
        .replace(
            "Client, Err, HTTPStatusError, Ok, RawResponse, ResponseError, bind",
            "Client",
        )
        .replace("from jsoncompat.http._binding import _prepare_api\n", "");
    fs::write(work.0.join("typing_results.py"), typing).unwrap();
    fs::write(work.0.join("pyrightconfig.json"), json!({"pythonVersion":"3.12", "typeCheckingMode":"strict", "extraPaths":[Path::new(env!("CARGO_MANIFEST_DIR")).join("pybindings")], "include":["typing_results.py", "petstore"]}).to_string()).unwrap();
    success(
        python_env::pyright_command()
            .current_dir(&work.0)
            .output()
            .unwrap(),
    );
}
