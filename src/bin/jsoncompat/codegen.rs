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
    #[arg(long, value_enum, default_value = "dataclasses")]
    target: CodegenTarget,
    /// Pretty-print output (multi-line).
    #[arg(short, long)]
    pretty: bool,
    /// Path to a JSON Schema document. Use '-' for STDIN.
    schema: String,
    /// Write readable Python models and a private generated companion.
    #[arg(short, long, value_name = "MODELS.py")]
    output: Option<PathBuf>,
    /// Generate a Python package from OpenAPI component schemas.
    #[arg(long, requires = "output")]
    openapi: bool,
    /// Select component schemas (dependencies are included). Defaults to all.
    #[arg(long = "component", requires = "openapi")]
    components: Vec<String>,
    /// Rename a component: COMPONENT=PythonName.
    #[arg(long, requires = "openapi")]
    rename: Vec<String>,
    /// Target module size; connected references stay together even above it.
    #[arg(long, default_value = "128")]
    models_per_module: std::num::NonZeroUsize,
    /// JSON SchemaOptions: offline resources and optional format assertions.
    #[arg(long, value_name = "OPTIONS.json")]
    schema_options: Option<PathBuf>,
}

pub(crate) fn cmd(args: CodegenArgs) -> Result<()> {
    let raw: serde_json::Value = serde_json::from_str(&crate::read_to_string(&args.schema)?)
        .with_context(|| format!("parsing {}", args.schema))?;
    let options: jsoncompat::SchemaOptions = args
        .schema_options
        .as_ref()
        .map(|path| {
            std::fs::read(path)
                .map_err(anyhow::Error::from)
                .and_then(|bytes| serde_json::from_slice(&bytes).map_err(Into::into))
        })
        .transpose()?
        .unwrap_or_default();
    if args.openapi {
        anyhow::ensure!(
            args.target == CodegenTarget::Dataclasses,
            "--openapi requires --target dataclasses"
        );
        return super::codegen_package::generate(
            &raw,
            args.output
                .as_deref()
                .context("--openapi requires --output PACKAGE_DIR")?,
            &args.components,
            &args.rename,
            args.models_per_module,
            &options,
        );
    }
    anyhow::ensure!(
        !super::compat::looks_like_openapi_document(&raw),
        "{} is an OpenAPI document, not a JSON Schema; pass --openapi --output PACKAGE_DIR to generate component models",
        args.schema
    );
    let schema = SchemaDoc {
        schema: jsoncompat::SchemaDocument::from_json_with_options(&raw, &options)?,
    };
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

pub(super) fn write_atomic(destination: &Path, source: &str) -> Result<()> {
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
