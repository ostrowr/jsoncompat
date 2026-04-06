use std::path::Path;

use anyhow::{Context, Result};
use clap::{Args, ValueEnum};
use serde_json::Value;

use crate::read_to_string;
use jsoncompat as backcompat;

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq)]
enum StampDisplayMode {
    Bundle,
    Writer,
    Reader,
    Manifest,
}

#[derive(Args)]
pub(crate) struct StampArgs {
    /// Path to the schema manifest.
    #[arg(long)]
    manifest: String,
    /// Stable schema identifier.
    #[arg(long)]
    id: String,
    /// Update the manifest file after stamping succeeds.
    #[arg(long)]
    write_manifest: bool,
    /// Display mode.
    #[arg(short, long, value_enum, default_value_t = StampDisplayMode::Bundle)]
    display: StampDisplayMode,
    /// Pretty-print output (multi-line).
    #[arg(short, long)]
    pretty: bool,
    /// Path to the current JSON Schema. Use '-' for STDIN.
    schema: String,
}

pub(crate) fn cmd(args: StampArgs) -> Result<()> {
    let manifest = load_stamp_manifest(&args.manifest)?;
    let schema = read_json(&args.schema)?;
    let result = backcompat::stamp_schema(&manifest, &args.id, schema)?;

    if args.write_manifest {
        backcompat::write_stamp_manifest_atomic(&args.manifest, &result.manifest)?;
    }

    match args.display {
        StampDisplayMode::Bundle => {
            let value = serde_json::to_value(&result.bundle)?;
            print_json(&value, args.pretty)
        }
        StampDisplayMode::Writer => print_json(&result.bundle.writer, args.pretty),
        StampDisplayMode::Reader => print_json(&result.bundle.reader, args.pretty),
        StampDisplayMode::Manifest => {
            let value = serde_json::to_value(&result.manifest)?;
            print_json(&value, args.pretty)
        }
    }
}

fn load_stamp_manifest(path: &str) -> Result<backcompat::StampManifest> {
    if !Path::new(path).exists() {
        return Ok(backcompat::StampManifest::empty());
    }

    let raw = read_to_string(path)?;
    serde_json::from_str(&raw).with_context(|| format!("parsing manifest file {path}"))
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
