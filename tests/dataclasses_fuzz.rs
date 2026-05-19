#[path = "support/generated_value_harness.rs"]
mod generated_value_harness;
#[path = "support/python_env.rs"]
mod python_env;

use generated_value_harness::{
    FuzzSchemaCase, GeneratedValueRoundTripper, GeneratedValueRoundTripperFactory,
    run_generated_value_fixture,
};
use jsoncompat_codegen::generate_dataclass_models;
use serde_json::Value;
use std::error::Error;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

datatest_stable::harness! {
    { test = fixture, root = "tests/fixtures/fuzz", pattern = ".*\\.json$" },
}

fn fixture(file: &Path) -> Result<(), Box<dyn Error>> {
    run_generated_value_fixture(file, &DataclassGeneratedValueRoundTripperFactory)
}

struct DataclassGeneratedValueRoundTripperFactory;

impl GeneratedValueRoundTripperFactory for DataclassGeneratedValueRoundTripperFactory {
    type RoundTripper = DataclassGeneratedValueRoundTripper;

    fn build_round_tripper(
        &self,
        schema_case: &FuzzSchemaCase<'_>,
    ) -> Result<Option<Self::RoundTripper>, Box<dyn Error>> {
        let source = match generate_dataclass_models(schema_case.schema_json) {
            Ok(source) => source,
            Err(_) => return Ok(None),
        };
        let module_path = write_generated_module(schema_case, &source)?;
        Ok(Some(DataclassGeneratedValueRoundTripper::spawn(
            module_path,
        )?))
    }
}

struct DataclassGeneratedValueRoundTripper {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
}

impl DataclassGeneratedValueRoundTripper {
    fn spawn(module_path: PathBuf) -> Result<Self, Box<dyn Error>> {
        let mut command = python_env::python_command();
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

    fn read_stderr(&mut self) -> String {
        let mut stderr_text = String::new();
        if let Some(stderr) = self.child.stderr.as_mut() {
            let _ = std::io::Read::read_to_string(stderr, &mut stderr_text);
        }
        stderr_text
    }
}

impl GeneratedValueRoundTripper for DataclassGeneratedValueRoundTripper {
    fn round_trip(&mut self, candidate: &Value) -> Result<Value, String> {
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

    fn reject_invalid(&mut self, candidate: &Value) -> Result<(), String> {
        self.write_request("reject_invalid", candidate)?;
        let line = self.read_response()?;
        match line.as_str() {
            "rejected" => Ok(()),
            "accepted" => Err("generated dataclass accepted invalid fixture input".to_owned()),
            _ => Err(format!("unexpected invalid-input response: {line:?}")),
        }
    }
}

impl DataclassGeneratedValueRoundTripper {
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

fn write_generated_module(
    schema_case: &FuzzSchemaCase<'_>,
    source: &str,
) -> Result<PathBuf, Box<dyn Error>> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "jsoncompat-dataclasses-fuzz-{}-{}-{unique}",
        std::process::id(),
        python_identifier_fragment(schema_case.rel_path),
    ));
    fs::create_dir_all(&dir)?;
    let module_path = dir.join(format!(
        "schema_{}.py",
        python_identifier_fragment(&schema_case.index.to_string()),
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

for raw_line in sys.stdin:
    mode, raw_json = raw_line.split("\t", 1)
    candidate = json.loads(raw_json)
    try:
        model = reader_model.from_json(candidate)
        if mode == "reject_invalid":
            print("accepted", flush=True)
            continue
        emitted = model.to_json()
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
