use serde_json::Value;
use std::error::Error;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct DataclassGeneratedValueRoundTripper {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
}

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
