use anyhow::{Context, Result};
use clap::{Args, ValueEnum};

use crate::SchemaDoc;
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
    let schema = SchemaDoc::load(&args.schema)?;
    let canonical_schema = schema
        .schema
        .canonical_schema_json()
        .with_context(|| format!("canonicalizing schema for {}", args.schema))?;

    match args.target {
        CodegenTarget::Dataclasses => {
            let source = generate_dataclass_models(canonical_schema)?;
            print!("{source}");
            Ok(())
        }
        CodegenTarget::Schema => print_json(canonical_schema, args.pretty),
    }
}

fn print_json(value: &serde_json::Value, pretty: bool) -> Result<()> {
    if pretty {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        println!("{}", serde_json::to_string(value)?);
    }
    Ok(())
}
