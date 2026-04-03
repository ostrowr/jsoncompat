use anyhow::{Context, Result};
use clap::{Args, ValueEnum};
use serde_json::{Map, Value};

use crate::read_to_string;
use jsoncompat_codegen::generate_dataclass_models;

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq)]
enum CodegenTarget {
    Dataclasses,
    Schema,
}

#[derive(Args)]
pub(crate) struct CodegenArgs {
    /// Code generation target.
    #[arg(long, value_enum)]
    target: CodegenTarget,
    /// Pretty-print output (multi-line).
    #[arg(short, long)]
    pretty: bool,
    /// Path to a JSON Schema document. Use '-' for STDIN.
    schema: String,
}

pub(crate) fn cmd(args: CodegenArgs) -> Result<()> {
    let schema = read_json(&args.schema)?;

    match args.target {
        CodegenTarget::Dataclasses => {
            let source = generate_dataclass_models(&schema)?;
            print!("{source}");
            Ok(())
        }
        CodegenTarget::Schema => print_json(&canonicalize_json(&schema), args.pretty),
    }
}

fn read_json(path: &str) -> Result<Value> {
    let raw = read_to_string(path)?;
    serde_json::from_str(&raw).with_context(|| format!("parsing {path}"))
}

fn print_json(value: &Value, pretty: bool) -> Result<()> {
    if pretty {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        println!("{}", serde_json::to_string(value)?);
    }
    Ok(())
}

fn canonicalize_json(value: &Value) -> Value {
    match value {
        Value::Object(obj) => {
            let mut canonical = Map::new();
            let mut keys = obj.keys().collect::<Vec<_>>();
            keys.sort();
            for key in keys {
                let child = obj.get(key).expect("key comes from object");
                canonical.insert(key.clone(), canonicalize_json(child));
            }
            Value::Object(canonical)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonicalize_json).collect()),
        _ => value.clone(),
    }
}
