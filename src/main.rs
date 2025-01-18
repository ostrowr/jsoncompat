use anyhow::{anyhow, Result};
use clap::Parser;
use rand::thread_rng;
use std::fs;
use std::path::PathBuf;
use url::Url;

use json_schema_backcompat::fuzz::fuzz_compat_check;
use json_schema_backcompat::{build_and_resolve_schema, check_compat, Role};

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    /// Path to the old schema
    #[arg(long)]
    old_schema: PathBuf,

    /// Path to the new schema
    #[arg(long)]
    new_schema: PathBuf,

    /// Role: serializer|deserializer|both
    #[arg(long, default_value = "both")]
    role: String,

    /// Number of fuzz samples to generate from each schema
    #[arg(long, default_value = "0")]
    fuzz: usize,

    /// Maximum recursion depth for fuzz generation
    #[arg(long, default_value = "3")]
    fuzz_depth: u8,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let old_raw = fs::read_to_string(&cli.old_schema)?;
    let new_raw = fs::read_to_string(&cli.new_schema)?;

    let old_json: serde_json::Value = serde_json::from_str(&old_raw)?;
    let new_json: serde_json::Value = serde_json::from_str(&new_raw)?;

    let old_base = Url::from_file_path(&cli.old_schema)
        .map_err(|_| anyhow!("Couldn't parse old_schema path into file URL"))?;
    let new_base = Url::from_file_path(&cli.new_schema)
        .map_err(|_| anyhow!("Couldn't parse new_schema path into file URL"))?;

    let old_ast = build_and_resolve_schema(&old_json, &old_base)?;
    let new_ast = build_and_resolve_schema(&new_json, &new_base)?;

    // Determine role
    let role = match cli.role.as_str() {
        "serializer" => Role::Serializer,
        "deserializer" => Role::Deserializer,
        "both" => Role::Both,
        other => return Err(anyhow!("Unrecognized role: {}", other)),
    };

    // 1. Run the structural/back-compat check
    let ok = check_compat(&old_ast, &new_ast, role);
    if ok {
        println!("No breaking changes (role={})", cli.role);
    } else {
        println!("Breaking changes detected (role={})", cli.role);
    }

    // 2. If fuzz requested, do random checks
    if cli.fuzz > 0 {
        println!(
            "Fuzzing with {} samples each (depth={})...",
            cli.fuzz, cli.fuzz_depth
        );
        let mut rng = thread_rng();
        fuzz_compat_check(&old_ast, &new_ast, cli.fuzz, cli.fuzz_depth, &mut rng);
    }

    Ok(())
}
