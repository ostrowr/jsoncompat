//! Command‑line interface for the `jsoncompat` crate.

use std::{
    fs,
    io::{self, Read},
    path::Path,
};

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use json_schema_ast::{compile, JSONSchema};
use jsoncompat as backcompat;

use owo_colors::OwoColorize;
use rand::Rng;
use serde_json::Value;

/// In‑memory representation of a schema with everything we need in one place.
struct SchemaDoc {
    ast: backcompat::SchemaNode,
    validator: JSONSchema,
}

impl SchemaDoc {
    fn load(path: &str) -> Result<Self> {
        // Read JSON (stdin if `-`).
        let raw = read_to_string(path)?;
        let json: Value =
            serde_json::from_str(&raw).with_context(|| format!("parsing {}", path))?;

        // Build AST and a validator for fast membership checks.
        let ast = backcompat::build_and_resolve_schema(&json)
            .with_context(|| format!("building AST for {}", path))?;
        let validator =
            compile(&json).with_context(|| format!("compiling validator for {}", path))?;

        Ok(Self { ast, validator })
    }

    #[inline]
    fn is_valid(&self, v: &Value) -> bool {
        self.validator.is_valid(v)
    }

    fn gen_value<R: Rng>(&self, rng: &mut R, depth: u8) -> Value {
        json_schema_fuzz::generate_value(&self.ast, rng, depth)
    }
}

/// Read an entire file (or stdin) into a string.
fn read_to_string(path: &str) -> Result<String> {
    if path == "-" {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        Ok(buf)
    } else {
        fs::read_to_string(Path::new(path)).with_context(|| format!("reading {}", path))
    }
}

// -----------------------------------------------------------------------------
// Sampling logic shared by fuzzing and counterexample search
// -----------------------------------------------------------------------------

fn sample_incompat<R: Rng>(
    old: &SchemaDoc,
    new: &SchemaDoc,
    role: backcompat::Role,
    attempts: usize,
    depth: u8,
    rng: &mut R,
) -> Option<Value> {
    let mut try_once = |src: &SchemaDoc, dst: &SchemaDoc| -> Option<Value> {
        (0..attempts).find_map(|_| {
            let v = src.gen_value(rng, depth);
            (src.is_valid(&v) && !dst.is_valid(&v)).then_some(v)
        })
    };

    match role {
        backcompat::Role::Serializer => try_once(new, old),
        backcompat::Role::Deserializer => try_once(old, new),
        backcompat::Role::Both => try_once(new, old).or_else(|| try_once(old, new)),
    }
}

// -----------------------------------------------------------------------------
// CLI (clap)
// -----------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "jsoncompat",
    about = "Schema utility toolbox: generation & compatibility checks",
    author,
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate random JSON instances that satisfy a schema.
    Generate(GenerateArgs),
    /// Check backward‑compatibility between two schema revisions.
    Compat(CompatArgs),
}

#[derive(Args)]
struct GenerateArgs {
    /// Path to the JSON Schema. Use ‘-’ for STDIN.
    schema: String,
    /// How many instances to emit.
    #[arg(short, long, default_value_t = 1)]
    count: u32,
    /// Maximum recursion depth.
    #[arg(short, long, default_value_t = 8)]
    depth: u8,
    /// Pretty‑print output (multi‑line).
    #[arg(short, long)]
    pretty: bool,
}

#[derive(Args)]
struct CompatArgs {
    /// Path to the *old* schema.
    old: String,
    /// Path to the *new* schema.
    new: String,
    /// Compatibility role.
    #[arg(long, value_enum, default_value_t = RoleCli::Both)]
    role: RoleCli,
    /// Additional fuzzing attempts (0 disables fuzz).
    #[arg(short = 'f', long, value_name = "N", default_value_t = 0)]
    fuzz: u32,
    /// Depth used during fuzzing.
    #[arg(short, long, default_value_t = 8)]
    depth: u8,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum RoleCli {
    Serializer,
    Deserializer,
    Both,
}

impl From<RoleCli> for backcompat::Role {
    fn from(r: RoleCli) -> Self {
        match r {
            RoleCli::Serializer => backcompat::Role::Serializer,
            RoleCli::Deserializer => backcompat::Role::Deserializer,
            RoleCli::Both => backcompat::Role::Both,
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Generate(a) => cmd_generate(a),
        Command::Compat(a) => cmd_compat(a),
    }
}

fn cmd_generate(args: GenerateArgs) -> Result<()> {
    let schema = SchemaDoc::load(&args.schema)?;
    let mut rng = rand::thread_rng();

    for _ in 0..args.count {
        let v = schema.gen_value(&mut rng, args.depth);
        if args.pretty {
            println!("{}", serde_json::to_string_pretty(&v)?);
        } else {
            println!("{}", serde_json::to_string(&v)?);
        }
    }
    Ok(())
}

fn cmd_compat(args: CompatArgs) -> Result<()> {
    let old = SchemaDoc::load(&args.old)?;
    let new = SchemaDoc::load(&args.new)?;
    let role: backcompat::Role = args.role.into();

    // 1. Static analysis.
    let ok_static = backcompat::check_compat(&old.ast, &new.ast, role);

    // 2. Optional fuzzing (only if requested or static failed).
    let offender = if args.fuzz > 0 && !ok_static {
        let mut rng = rand::thread_rng();
        sample_incompat(&old, &new, role, args.fuzz as usize, args.depth, &mut rng)
    } else {
        None
    };

    if ok_static && offender.is_none() {
        eprintln!(
            "{} Schemas seem backward-compatible (role = {:?})",
            "✔".green(),
            role
        );
        return Ok(());
    }

    // Failure case.
    eprintln!(
        "{} Schemas are NOT backward-compatible (role = {:?})",
        "✘".red(),
        role
    );

    if let Some(ex) = offender {
        let pretty =
            serde_json::to_string_pretty(&ex).unwrap_or_else(|_| "<unserializable>".into());
        eprintln!("{} Counter-example:\n{}", "•".yellow(), pretty);
        let old_valid = old.is_valid(&ex);
        let new_valid = new.is_valid(&ex);
        eprintln!(
            "{} Old schema: {}",
            "•".yellow(),
            if old_valid { "accepts" } else { "rejects" }
        );
        eprintln!(
            "{} New schema: {}",
            "•".yellow(),
            if new_valid { "accepts" } else { "rejects" }
        );
    }

    std::process::exit(1);
}

// -----------------------------------------------------------------------------
// Compile‑time smoke test
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_conversion() {
        let r: backcompat::Role = RoleCli::Serializer.into();
        assert!(matches!(r, backcompat::Role::Serializer));
    }
}
