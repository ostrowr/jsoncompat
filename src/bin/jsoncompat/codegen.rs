use anyhow::{Context, Result};
use clap::{Args, ValueEnum};
use std::path::{Path, PathBuf};

use crate::SchemaDoc;
use jsoncompat_codegen::generate_dataclass_module_from_document;

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
    /// Write readable Python models and a private generated companion.
    #[arg(short, long, value_name = "MODELS.py")]
    output: Option<PathBuf>,
}

pub(crate) fn cmd(args: CodegenArgs) -> Result<()> {
    let schema = SchemaDoc::load(&args.schema)?;
    let canonical_schema = schema
        .schema
        .canonical_schema_json()
        .with_context(|| format!("canonicalizing schema for {}", args.schema))?;

    match args.target {
        CodegenTarget::Dataclasses => {
            let module = generate_dataclass_module_from_document(&schema.schema)?;
            if let Some(output) = args.output {
                let stem = output
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .filter(|name| {
                        name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
                            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                    })
                    .context(
                        "Python output must have a simple module filename, such as models.py",
                    )?;
                anyhow::ensure!(
                    output.extension().is_some_and(|ext| ext == "py"),
                    "Python output must end in .py"
                );
                let implementation = module.private_file();
                let companion = format!("_{stem}_generated");
                write_atomic(
                    &output.with_file_name(format!("{companion}.py")),
                    &implementation,
                )?;
                write_atomic(&output, &module.public_file(&companion))?;
            } else {
                print!("{}", module.single_file());
            }
            Ok(())
        }
        CodegenTarget::Schema => {
            anyhow::ensure!(
                args.output.is_none(),
                "--output is supported with --target dataclasses"
            );
            print_json(canonical_schema, args.pretty)
        }
    }
}

fn write_atomic(destination: &Path, source: &str) -> Result<()> {
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(
        ".jsoncompat-{}-{:016x}.tmp",
        std::process::id(),
        rand::random::<u64>()
    ));
    let result = (|| {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(source.as_bytes())?;
        file.sync_all()?;
        std::fs::rename(&temporary, destination)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result.with_context(|| format!("writing {}", destination.display()))?;
    Ok(())
}

fn print_json(value: &serde_json::Value, pretty: bool) -> Result<()> {
    if pretty {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        println!("{}", serde_json::to_string(value)?);
    }
    Ok(())
}
