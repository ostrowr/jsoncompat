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
    fn generate_package(
        &self,
        spec: &Value,
        protocol: &str,
        package: &str,
        extra: &[&str],
    ) -> Output {
        fs::write(self.0.join("api.json"), spec.to_string()).unwrap();
        Command::new(env!("CARGO_BIN_EXE_jsoncompat"))
            .args([
                "codegen",
                "--openapi",
                "api.json",
                "--protocol",
                protocol,
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
fn generated_protocols_encode_decode_and_bind_without_generated_clients() {
    let work = Work::new();
    success(work.generate(&example(), &[]));
    assert!(work.0.join("petstore/py.typed").is_file());
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
fn protocols_preserve_static_types_and_structural_implementations() {
    let work = Work::new();
    success(work.generate(&example(), &[]));
    copy_python_examples(&work.0);
    fs::write(work.0.join("pyrightconfig.json"),json!({"pythonVersion":"3.12","typeCheckingMode":"strict","extraPaths":[Path::new(env!("CARGO_MANIFEST_DIR")).join("pybindings")],"include":["valid.py","petstore","transports.py","exercise.py","server.py","aiohttp_client.py","httpx_client.py","requests_client.py"]}).to_string()).unwrap();
    fs::write(
        work.0.join("valid.py"),
        r#"
from typing import Protocol, assert_type
from petstore import Petstore, AsyncPetstore, GetPetInput, GetPetResponse, GetPet200, ListPetsInput
from jsoncompat.http import bind, bind_async, EncodedRequest, RawResponse
from jsoncompat import MISSING

def fetch(request: EncodedRequest, /) -> RawResponse:
    raise NotImplementedError
async def async_fetch(request: EncodedRequest, /) -> RawResponse:
    raise NotImplementedError
client = bind(Petstore, fetch=fetch)
assert_type(client, Petstore)
assert_type(client.get_pet(GetPetInput(pet_id="a")), GetPetResponse)
request=ListPetsInput()
if request.limit is not MISSING:
    assert_type(request.limit, int)
async def example() -> GetPetResponse:
    client=bind_async(AsyncPetstore, fetch=async_fetch)
    assert_type(client, AsyncPetstore)
    return await client.get_pet(GetPetInput(pet_id="a"))
class ReadPet(Protocol):
    def get_pet(self, request: GetPetInput, /) -> GetPetResponse: ...
class Fake:
    def get_pet(self, input: GetPetInput, /) -> GetPetResponse:
        raise NotImplementedError
reader: ReadPet = Fake()
response=reader.get_pet(GetPetInput(pet_id="a"))
if isinstance(response, GetPet200):
    assert_type(response.body.name, str)
"#,
    )
    .unwrap();
    success(
        python_env::pyright_command_with_group("http-examples")
            .current_dir(&work.0)
            .output()
            .unwrap(),
    );
    fs::write(
        work.0.join("invalid.py"),
        r#"
from petstore import Petstore, GetPetInput, ListPetsInput
from jsoncompat.http import bind, EncodedRequest, RawResponse
async def fetch(request: EncodedRequest, /) -> RawResponse: raise NotImplementedError
client=bind(Petstore,fetch=fetch)
client.get_pet(ListPetsInput())
client.get_pet(GetPetInput(pet_id=123))
client.unknown_operation()
"#,
    )
    .unwrap();
    let output = python_env::pyright_command()
        .current_dir(&work.0)
        .arg("invalid.py")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let diagnostics = String::from_utf8_lossy(&output.stdout);
    for expected in [
        "Coroutine",
        "ListPetsInput",
        "Literal[123]",
        "unknown_operation",
    ] {
        assert!(diagnostics.contains(expected), "{diagnostics}");
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
from petstore import Petstore, PingInput, PingDefault, Outcome
from jsoncompat.http import bind, RawResponse
def fetch(request): return RawResponse(418, (("Content-Type","application/json"),), b'{"ok":false}')
result=bind(Petstore,fetch=fetch).ping(PingInput())
assert isinstance(result,PingDefault) and isinstance(result.body,Outcome)
assert result.status == 418 and result.body.ok is False
"#,
            )
            .output()
            .unwrap(),
    );
    fs::write(work.0.join("petstore/unmanaged.txt"), "keep").unwrap();
    let before = fs::read(work.0.join("petstore/protocols.py")).unwrap();
    success(work.generate(&spec, &["--rename", "Result=Outcome"]));
    assert_eq!(
        fs::read(work.0.join("petstore/protocols.py")).unwrap(),
        before
    );
    assert_eq!(
        fs::read_to_string(work.0.join("petstore/unmanaged.txt")).unwrap(),
        "keep"
    );
}
