use serde_json::Value;
use std::error::Error;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

#[allow(dead_code)]
pub struct DataclassGeneratedValueRoundTripper {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
}

#[allow(dead_code)]
pub struct StampedDataclassRoundTripper {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
}

#[allow(dead_code)]
impl DataclassGeneratedValueRoundTripper {
    pub fn spawn(module_path: PathBuf) -> Result<Self, Box<dyn Error>> {
        let mut command = crate::python_env::python_command();
        command
            .args(["-B", "-c", DATACLASS_VALIDATOR_DRIVER])
            .arg(module_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn()?;

        let stdin = child.stdin.take().expect("piped validator stdin");
        let stdout = BufReader::new(child.stdout.take().expect("piped validator stdout"));

        Ok(Self {
            child,
            stdin: Some(stdin),
            stdout,
        })
    }

    pub fn round_trip_value(&mut self, candidate: &Value) -> Result<Value, String> {
        self.write_request("round_trip", candidate)?;
        let line = self.read_response()?;
        match line.strip_prefix("err\t") {
            Some(message) => Err(message.to_owned()),
            None => {
                let emitted = line
                    .strip_prefix("ok\t")
                    .ok_or_else(|| format!("unexpected validator response: {line:?}"))?;
                serde_json::from_str(emitted)
                    .map_err(|error| format!("validator returned invalid JSON: {error}"))
            }
        }
    }

    pub fn reject_invalid_value(&mut self, candidate: &Value) -> Result<(), String> {
        self.write_request("reject_invalid", candidate)?;
        let line = self.read_response()?;
        match line.as_str() {
            "rejected" => Ok(()),
            "accepted" => Err("generated dataclass accepted invalid fixture input".to_owned()),
            _ => Err(format!("unexpected invalid-input response: {line:?}")),
        }
    }

    fn read_stderr(&mut self) -> String {
        let mut stderr_text = String::new();
        if let Some(stderr) = self.child.stderr.as_mut() {
            let _ = std::io::Read::read_to_string(stderr, &mut stderr_text);
        }
        stderr_text
    }

    fn write_request(&mut self, mode: &str, candidate: &Value) -> Result<(), String> {
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| "dataclass validator stdin is closed".to_owned())?;
        stdin
            .write_all(mode.as_bytes())
            .map_err(|error| error.to_string())?;
        stdin.write_all(b"\t").map_err(|error| error.to_string())?;
        serde_json::to_writer(&mut *stdin, candidate).map_err(|error| error.to_string())?;
        stdin.write_all(b"\n").map_err(|error| error.to_string())?;
        stdin.flush().map_err(|error| error.to_string())
    }

    fn read_response(&mut self) -> Result<String, String> {
        let mut line = String::new();
        let bytes_read = self
            .stdout
            .read_line(&mut line)
            .map_err(|error| error.to_string())?;
        if bytes_read == 0 {
            let stderr_text = self.read_stderr();
            return Err(if stderr_text.is_empty() {
                "dataclass validator exited without a response".to_owned()
            } else {
                stderr_text
            });
        }
        Ok(line.trim_end().to_owned())
    }
}

impl Drop for DataclassGeneratedValueRoundTripper {
    fn drop(&mut self) {
        drop(self.stdin.take());
        let _ = self.child.wait();
    }
}

#[allow(dead_code)]
impl StampedDataclassRoundTripper {
    pub fn spawn(
        writer_module_path: PathBuf,
        reader_module_path: PathBuf,
    ) -> Result<Self, Box<dyn Error>> {
        let mut command = crate::python_env::python_command();
        command
            .args(["-B", "-c", STAMPED_DATACLASS_VALIDATOR_DRIVER])
            .arg(writer_module_path)
            .arg(reader_module_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn()?;

        let stdin = child.stdin.take().expect("piped stamped validator stdin");
        let stdout = BufReader::new(child.stdout.take().expect("piped stamped validator stdout"));

        Ok(Self {
            child,
            stdin: Some(stdin),
            stdout,
        })
    }

    pub fn round_trip_writer_payload(&mut self, payload: &Value) -> Result<Value, String> {
        self.write_request("writer_round_trip", payload)?;
        self.read_json_response()
    }

    pub fn reject_writer_payload(&mut self, payload: &Value) -> Result<(), String> {
        self.write_request("writer_reject", payload)?;
        self.read_rejection_response("stamped writer accepted invalid payload")
    }

    pub fn round_trip_reader_envelope(&mut self, envelope: &Value) -> Result<Value, String> {
        self.write_request("reader_round_trip", envelope)?;
        self.read_json_response()
    }

    pub fn reject_reader_envelope(&mut self, envelope: &Value) -> Result<(), String> {
        self.write_request("reader_reject", envelope)?;
        self.read_rejection_response("stamped reader accepted invalid envelope")
    }

    fn read_stderr(&mut self) -> String {
        let mut stderr_text = String::new();
        if let Some(stderr) = self.child.stderr.as_mut() {
            let _ = std::io::Read::read_to_string(stderr, &mut stderr_text);
        }
        stderr_text
    }

    fn write_request(&mut self, mode: &str, candidate: &Value) -> Result<(), String> {
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| "stamped dataclass validator stdin is closed".to_owned())?;
        stdin
            .write_all(mode.as_bytes())
            .map_err(|error| error.to_string())?;
        stdin.write_all(b"\t").map_err(|error| error.to_string())?;
        serde_json::to_writer(&mut *stdin, candidate).map_err(|error| error.to_string())?;
        stdin.write_all(b"\n").map_err(|error| error.to_string())?;
        stdin.flush().map_err(|error| error.to_string())
    }

    fn read_json_response(&mut self) -> Result<Value, String> {
        let line = self.read_response()?;
        match line.strip_prefix("err\t") {
            Some(message) => Err(message.to_owned()),
            None => {
                let emitted = line
                    .strip_prefix("ok\t")
                    .ok_or_else(|| format!("unexpected stamped validator response: {line:?}"))?;
                serde_json::from_str(emitted)
                    .map_err(|error| format!("stamped validator returned invalid JSON: {error}"))
            }
        }
    }

    fn read_rejection_response(&mut self, accepted_message: &str) -> Result<(), String> {
        let line = self.read_response()?;
        match line.as_str() {
            "rejected" => Ok(()),
            "accepted" => Err(accepted_message.to_owned()),
            _ => Err(format!(
                "unexpected stamped invalid-input response: {line:?}"
            )),
        }
    }

    fn read_response(&mut self) -> Result<String, String> {
        let mut line = String::new();
        let bytes_read = self
            .stdout
            .read_line(&mut line)
            .map_err(|error| error.to_string())?;
        if bytes_read == 0 {
            let stderr_text = self.read_stderr();
            return Err(if stderr_text.is_empty() {
                "stamped dataclass validator exited without a response".to_owned()
            } else {
                stderr_text
            });
        }
        Ok(line.trim_end().to_owned())
    }
}

impl Drop for StampedDataclassRoundTripper {
    fn drop(&mut self) {
        drop(self.stdin.take());
        let _ = self.child.wait();
    }
}

pub fn write_generated_module(
    temp_prefix: &str,
    namespace: &str,
    module_name: &str,
    source: &str,
) -> Result<PathBuf, Box<dyn Error>> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "jsoncompat-{temp_prefix}-{}-{}-{unique}",
        std::process::id(),
        python_identifier_fragment(namespace),
    ));
    fs::create_dir_all(&dir)?;
    let module_path = dir.join(format!(
        "schema_{}.py",
        python_identifier_fragment(module_name),
    ));
    fs::write(&module_path, source)?;
    Ok(module_path)
}

fn python_identifier_fragment(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

#[allow(dead_code)]
const DATACLASS_VALIDATOR_DRIVER: &str = r#"
import importlib.util
import json
import sys

module_path = sys.argv[1]
spec = importlib.util.spec_from_file_location("generated_models", module_path)
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = module
spec.loader.exec_module(module)
reader_model = module.JSONCOMPAT_MODEL

def json_equivalent(left, right):
    if isinstance(left, dict) and isinstance(right, dict):
        return left.keys() == right.keys() and all(
            json_equivalent(left[key], right[key]) for key in left
        )
    if isinstance(left, list) and isinstance(right, list):
        return len(left) == len(right) and all(
            json_equivalent(left_item, right_item)
            for left_item, right_item in zip(left, right)
        )
    if isinstance(left, bool) or isinstance(right, bool):
        return isinstance(left, bool) and isinstance(right, bool) and left == right
    if isinstance(left, (int, float)) and isinstance(right, (int, float)):
        return left == right
    return type(left) is type(right) and left == right

for raw_line in sys.stdin:
    mode, raw_json = raw_line.split("\t", 1)
    candidate = json.loads(raw_json)
    try:
        model = reader_model.from_json(candidate)
        if mode == "reject_invalid":
            print("accepted", flush=True)
            continue
        emitted = model.to_json()
        candidate_json = json.dumps(candidate, separators=(",", ":"), sort_keys=True)
        emitted_json = json.dumps(emitted, separators=(",", ":"), sort_keys=True)
        if not json_equivalent(candidate, emitted):
            print(
                "err\tgenerated dataclass changed parsed JSON during round-trip: "
                + f"{candidate_json} -> {emitted_json}",
                flush=True,
            )
            continue
    except Exception as error:
        if mode == "reject_invalid":
            print("rejected", flush=True)
        else:
            print(f"err\t{type(error).__name__}: {error}", flush=True)
    else:
        print(
            "ok\t" + json.dumps(emitted, separators=(",", ":"), sort_keys=True),
            flush=True,
        )
"#;

#[allow(dead_code)]
const STAMPED_DATACLASS_VALIDATOR_DRIVER: &str = r###"
import importlib.util
import json
import sys


def load_module(module_name, module_path):
    spec = importlib.util.spec_from_file_location(module_name, module_path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def resolve_local_ref(schema, ref):
    if ref == "#":
        return schema
    if not ref.startswith("#/"):
        raise AssertionError(f"unsupported stamped payload ref {ref!r}")
    current = schema
    for raw_segment in ref[2:].split("/"):
        segment = raw_segment.replace("~1", "/").replace("~0", "~")
        current = current[segment]
    return current


def json_equivalent(left, right):
    if isinstance(left, dict) and isinstance(right, dict):
        return left.keys() == right.keys() and all(
            json_equivalent(left[key], right[key]) for key in left
        )
    if isinstance(left, list) and isinstance(right, list):
        return len(left) == len(right) and all(
            json_equivalent(left_item, right_item)
            for left_item, right_item in zip(left, right)
        )
    if isinstance(left, bool) or isinstance(right, bool):
        return isinstance(left, bool) and isinstance(right, bool) and left == right
    if isinstance(left, (int, float)) and isinstance(right, (int, float)):
        return left == right
    return type(left) is type(right) and left == right


writer_module = load_module("generated_writer_models", sys.argv[1])
reader_module = load_module("generated_reader_models", sys.argv[2])
writer_model = writer_module.JSONCOMPAT_MODEL
reader_model = reader_module.JSONCOMPAT_MODEL
writer_schema = json.loads(writer_model.__jsoncompat_schema__)
writer_metadata = writer_schema["x-jsoncompat"]
writer_version = writer_metadata["version"]
writer_payload_schema = resolve_local_ref(writer_schema, writer_metadata["payload_ref"])
writer_payload_name = writer_payload_schema["x-jsoncompat"]["name"]
writer_payload_model = getattr(writer_module, writer_payload_name)


def writer_from_payload(payload):
    data = writer_payload_model.from_json(payload)
    return writer_model(version=writer_version, data=data)


def reader_envelope_from_model(model):
    return {
        "version": model.root.version,
        "data": model.root.data.to_json(),
    }


for raw_line in sys.stdin:
    mode, raw_json = raw_line.split("\t", 1)
    candidate = json.loads(raw_json)
    try:
        if mode == "writer_round_trip":
            emitted = writer_from_payload(candidate).to_json()
            if emitted["version"] != writer_version or not json_equivalent(
                candidate, emitted["data"]
            ):
                print(
                    "err\tgenerated stamped writer changed payload during round-trip: "
                    + f"{candidate!r} -> {emitted!r}",
                    flush=True,
                )
                continue
        elif mode == "writer_reject":
            writer_from_payload(candidate)
            print("accepted", flush=True)
            continue
        elif mode == "reader_round_trip":
            emitted = reader_envelope_from_model(reader_model.from_json(candidate))
            if not json_equivalent(candidate, emitted):
                print(
                    "err\tgenerated stamped reader changed envelope during round-trip: "
                    + f"{candidate!r} -> {emitted!r}",
                    flush=True,
                )
                continue
        elif mode == "reader_reject":
            reader_model.from_json(candidate)
            print("accepted", flush=True)
            continue
        else:
            raise AssertionError(f"unknown stamped dataclass mode {mode!r}")
    except Exception as error:
        if mode.endswith("_reject"):
            print("rejected", flush=True)
        else:
            print(f"err\t{type(error).__name__}: {error}", flush=True)
    else:
        print(
            "ok\t" + json.dumps(emitted, separators=(",", ":"), sort_keys=True),
            flush=True,
        )
"###;
