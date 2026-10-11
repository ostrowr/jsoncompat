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
            "include": ["jsonplaceholder", "live_jsonplaceholder.py", "transports.py"]
        })
        .to_string(),
    )
    .unwrap();
    success(
        python_env::pyright_command_with_group("http-examples")
            .current_dir(&work.0)
            .output()
            .unwrap(),
    );
}

#[test]
#[ignore = "makes 24 HTTPS requests to JSONPlaceholder; run just python-http-live"]
fn jsonplaceholder_live_all_transports() {
    let work = jsonplaceholder_example();
    let status = python_env::python_command_with_group("http-examples")
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
            python_env::python_command_with_group("http-examples")
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
    fs::write(work.0.join("pyrightconfig.json"),json!({"pythonVersion":"3.12","typeCheckingMode":"strict","extraPaths":[Path::new(env!("CARGO_MANIFEST_DIR")).join("pybindings")],"include":["valid.py","petstore","other_petstore","transports.py","exercise.py","server.py","aiohttp_client.py","httpx_client.py","requests_client.py","prepared_client.py"]}).to_string()).unwrap();
    fs::write(
        work.0.join("valid.py"),
        r#"
from typing import assert_type
from petstore import PetstoreAPI, GetPetRequest, GetPetResponse, GetPet200, ListPetsRequest
from jsoncompat.http import API, AsyncClient, Client, PreparedRequest, Request, RequestOptions, bind, bind_async, prepare, EncodedRequest, RawResponse
from jsoncompat import MISSING

def fetch(request: EncodedRequest, /) -> RawResponse:
    raise NotImplementedError
async def async_fetch(request: EncodedRequest, /) -> RawResponse:
    raise NotImplementedError
client = bind(PetstoreAPI, fetch=fetch)
assert_type(client, Client[PetstoreAPI])
assert_type(client.request(GetPetRequest(pet_id="a")), GetPetResponse)
options = RequestOptions(extra_headers={"X-Trace-ID": "trace"}, extra_query=(("preview", "true"),))
assert_type(client.request(GetPetRequest(pet_id="a"), options=options), GetPetResponse)
call = prepare(PetstoreAPI, GetPetRequest(pet_id="a"), options=options)
assert_type(call, PreparedRequest[PetstoreAPI, GetPetResponse])
assert_type(call.request, EncodedRequest)
assert_type(call.api, type[PetstoreAPI])
assert_type(call.decode(fetch(call.request)), GetPetResponse)
request=ListPetsRequest()
if request.limit is not MISSING:
    assert_type(request.limit, int)
async def example() -> GetPetResponse:
    client=bind_async(PetstoreAPI, fetch=async_fetch)
    assert_type(client, AsyncClient[PetstoreAPI])
    assert_type(await client.request(GetPetRequest(pet_id="a"), options=options), GetPetResponse)
    return call.decode(await async_fetch(call.request))
class Fake:
    def request[R](self, request: Request[PetstoreAPI, R], /, *, options: RequestOptions | None = None) -> R:
        raise NotImplementedError
reader: Client[PetstoreAPI] = Fake()
response=reader.request(GetPetRequest(pet_id="a"))
if isinstance(response, GetPet200):
    assert_type(response.body.name, str)
def execute[A: API, R](client: Client[A], request: Request[A, R]) -> R:
    return client.request(request)
assert_type(execute(client, GetPetRequest(pet_id="a")), GetPetResponse)
"#,
    ).unwrap();
    success(
        python_env::pyright_command_with_group("http-examples")
            .current_dir(&work.0)
            .output()
            .unwrap(),
    );
    let invalid = r#"
from petstore import PetstoreAPI, GetPetRequest, GetPetResponse, DeletePet204
from other_petstore import PetstoreAPI as OtherAPI, GetPetRequest as OtherRequest
from jsoncompat.http import API, AsyncClient, Client, PreparedRequest, Request, RequestOptions, Response, bind, operation, prepare
from typing_extensions import TypeForm
from valid import async_fetch, call, client, execute, fetch

bind(PetstoreAPI, fetch=async_fetch)  # must fail
client.request(GetPetRequest(pet_id=123))  # must fail
client.request(OtherRequest(pet_id="x"))  # must fail
execute(client, OtherRequest(pet_id="x"))  # must fail
widened: Client[API] = client  # must fail
wrong: Client[OtherAPI] = client  # must fail
bare: Client = client  # must fail
bare_request: Request = GetPetRequest(pet_id="x")  # must fail
wrong_response: DeletePet204 = client.request(GetPetRequest(pet_id="x"))  # must fail
bind(fetch=fetch)  # must fail
bind(object, fetch=fetch)  # must fail
client.request(GetPetRequest)  # must fail
client.get_pet(GetPetRequest(pet_id="x"))  # must fail
operation(GetPetRequest, TypeForm(DeletePet204), method="DELETE", path="/pets/{petId}", responses=(Response(204, DeletePet204),))  # must fail
prepare(PetstoreAPI, OtherRequest(pet_id="x"))  # must fail
prepare(OtherAPI, GetPetRequest(pet_id="x"))  # must fail
prepare(GetPetRequest(pet_id="x"))  # must fail
prepared_other: PreparedRequest[OtherAPI, GetPetResponse] = call  # must fail
prepared_widened: PreparedRequest[API, GetPetResponse] = call  # must fail
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
async def wrong_async(client: AsyncClient[PetstoreAPI]) -> None:
    await client.request(OtherRequest(pet_id="x"))  # must fail
"#;
    fs::write(work.0.join("invalid.py"), invalid).unwrap();
    let output = python_env::pyright_command_with_group("http-examples")
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
def fetch(request): return RawResponse(418, (("Content-Type","application/json"),), b'{"ok":false}')
result=bind(PetstoreAPI,fetch=fetch).request(PingRequest())
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
