//! Command‑line interface for the `jsoncompat` crate.

use console::{Alignment, pad_str};
use owo_colors::OwoColorize;
use std::{
    fs,
    io::{self, Read},
    path::Path,
};

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use json_schema_ast::{JSONSchema, compile};
use jsoncompat as backcompat;

use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// In‑memory representation of a schema with a cached validator.
#[derive(Debug)]
struct SchemaDoc {
    ast: backcompat::SchemaNode,
    validator: JSONSchema,
}

impl SchemaDoc {
    fn load(path: &str) -> Result<Self> {
        // Read JSON (stdin if `-`).
        let raw = read_to_string(path)?;
        let json: Value = serde_json::from_str(&raw).with_context(|| format!("parsing {path}"))?;

        // Build AST and a validator for fast membership checks.
        let ast = backcompat::build_and_resolve_schema(&json)
            .with_context(|| format!("building AST for {path}"))?;
        let validator =
            compile(&json).with_context(|| format!("compiling validator for {path}"))?;

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
        fs::read_to_string(Path::new(path)).with_context(|| format!("reading {path}"))
    }
}

// Sampling logic shared by fuzzing and counterexample search
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
    /// Check compatibility between two golden files.
    CI(CiArgs),
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

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum DisplayMode {
    Table,
    Json,
}

#[derive(Args)]
struct CiArgs {
    /// Path to the *old* golden file.
    old: String,
    /// Path to the *new* golden file.
    new: String,
    /// Display mode.
    #[arg(short, long, value_enum, default_value_t = DisplayMode::Table)]
    display: DisplayMode,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
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
        Command::CI(a) => cmd_ci(a),
    }
}

fn cmd_generate(args: GenerateArgs) -> Result<()> {
    let schema = SchemaDoc::load(&args.schema)?;
    let mut rng = rand::rng();

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
        let mut rng = rand::rng();
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

#[derive(Deserialize)]
struct GoldenEntry {
    mode: RoleCli,
    schema: serde_json::Value,
    stable_id: String,
}

type GoldenFile = std::collections::HashMap<String, GoldenEntry>;

fn load_golden_file(path: &str) -> Result<GoldenFile> {
    let raw = read_to_string(path)?;
    let golden: GoldenFile =
        serde_json::from_str(&raw).with_context(|| format!("parsing golden file {path}"))?;

    Ok(golden)
}

#[derive(Debug, PartialEq, Serialize)]
enum Status {
    Ok,
    MissingOld,
    MissingNew,
    ModeChanged,
    Incompatible { example: Option<Value> },
    Invalid,
    Identical,
}

#[derive(Debug, PartialEq, Serialize)]
struct Grade {
    id: String,
    mode: RoleCli,
    status: Status,
}

fn grade_entry(old: Option<&GoldenEntry>, new: Option<&GoldenEntry>) -> Grade {
    match (old, new) {
        (Some(old), Some(new)) => {
            let (old_schema, new_schema) = (
                backcompat::build_and_resolve_schema(&old.schema),
                backcompat::build_and_resolve_schema(&new.schema),
            );
            match (old_schema, new_schema) {
                (Ok(old_schema), Ok(new_schema)) => {
                    if old.schema == new.schema {
                        return Grade {
                            id: new.stable_id.clone(),
                            mode: old.mode,
                            status: Status::Identical,
                        };
                    }
                    let ok = backcompat::check_compat(&old_schema, &new_schema, old.mode.into());
                    if !ok {
                        let old_validator = compile(&old.schema).unwrap();
                        let new_validator = compile(&new.schema).unwrap();
                        let mut rng = rand::rng();
                        let example = sample_incompat(
                            &SchemaDoc {
                                ast: old_schema,
                                validator: old_validator,
                            },
                            &SchemaDoc {
                                ast: new_schema,
                                validator: new_validator,
                            },
                            old.mode.into(),
                            100,
                            8,
                            &mut rng,
                        );
                        Grade {
                            id: new.stable_id.clone(),
                            mode: old.mode,
                            status: Status::Incompatible { example },
                        }
                    } else if old.mode != new.mode {
                        Grade {
                            id: new.stable_id.clone(),
                            mode: old.mode,
                            status: Status::ModeChanged,
                        }
                    } else {
                        Grade {
                            id: new.stable_id.clone(),
                            mode: old.mode,
                            status: Status::Ok,
                        }
                    }
                }
                _ => Grade {
                    id: new.stable_id.clone(),
                    mode: old.mode,
                    status: Status::Invalid,
                },
            }
        }
        (Some(old), None) => Grade {
            id: old.stable_id.clone(),
            mode: old.mode,
            status: Status::MissingNew,
        },
        (None, Some(new)) => Grade {
            id: new.stable_id.clone(),
            mode: new.mode,
            status: Status::MissingOld,
        },
        (None, None) => unreachable!(
            "grade_entry called with both old and new as None; this should never happen"
        ),
    }
}

fn print_grades_table(grades: &Vec<Grade>) -> Result<()> {
    // Table headers
    let header_id = "ID";
    let header_mode = "Mode";
    let header_status = "Status";
    let header_example = "Example";

    // Compute column widths
    let id_width = grades
        .iter()
        .map(|g| g.id.len())
        .max()
        .unwrap_or(2)
        .max(header_id.len());
    let mode_width = grades
        .iter()
        .map(|g| format!("{:?}", g.mode).len())
        .max()
        .unwrap_or(4)
        .max(header_mode.len());
    let status_width = grades
        .iter()
        .map(|g| match &g.status {
            Status::Ok => "Ok".len(),
            Status::MissingOld => "MissingOld".len(),
            Status::MissingNew => "MissingNew".len(),
            Status::ModeChanged => "ModeChanged".len(),
            Status::Incompatible { .. } => "Incompatible".len(),
            Status::Invalid => "Invalid".len(),
            Status::Identical => "Identical".len(),
        })
        .max()
        .unwrap_or(6)
        .max(header_status.len());
    let no_example = "Could not find example";
    let example_width = grades
        .iter()
        .map(|g| match &g.status {
            Status::Incompatible { example } => {
                if let Some(example) = example {
                    let s = example.to_string();
                    s.len()
                } else {
                    no_example.len()
                }
            }
            _ => "N/A".len(),
        })
        .max()
        .unwrap_or(7)
        .max(header_example.len());

    // Print header
    println!(
        "{}  {}  {}  {}",
        pad_str(
            &header_id.bold().to_string(),
            id_width,
            Alignment::Left,
            None
        ),
        pad_str(
            &header_mode.bold().to_string(),
            mode_width,
            Alignment::Left,
            None
        ),
        pad_str(
            &header_status.bold().to_string(),
            status_width,
            Alignment::Left,
            None
        ),
        pad_str(
            &header_example.bold().to_string(),
            example_width,
            Alignment::Left,
            None
        )
    );

    // Print separator
    println!(
        "{}  {}  {}  {}",
        pad_str("", id_width, Alignment::Left, Some("-")),
        pad_str("", mode_width, Alignment::Left, Some("-")),
        pad_str("", status_width, Alignment::Left, Some("-")),
        pad_str("", example_width, Alignment::Left, Some("-"))
    );

    // Print each grade
    for grade in grades {
        let (status_str, example_str) = match &grade.status {
            Status::Ok => ("Ok".green().to_string(), "N/A".to_string()),
            Status::MissingOld => ("MissingOld".yellow().to_string(), "N/A".to_string()),
            Status::MissingNew => ("MissingNew".yellow().to_string(), "N/A".to_string()),
            Status::ModeChanged => ("ModeChanged".yellow().to_string(), "N/A".to_string()),
            Status::Incompatible { example } => {
                let status = "Incompatible".red().to_string();
                let example_str = if let Some(example) = example {
                    example.to_string()
                } else {
                    no_example.to_string()
                };
                (status, example_str)
            }
            Status::Invalid => ("Invalid".red().to_string(), "N/A".to_string()),
            Status::Identical => ("Identical".green().to_string(), "N/A".to_string()),
        };

        let mode = grade.mode;
        let mode_str = format!("{mode:?}");

        println!(
            "{}  {}  {}  {}",
            pad_str(&grade.id, id_width, Alignment::Left, None),
            pad_str(
                &mode_str.cyan().to_string(),
                mode_width,
                Alignment::Left,
                None
            ),
            pad_str(&status_str, status_width, Alignment::Left, None),
            pad_str(
                &example_str.bright_black().to_string(),
                example_width,
                Alignment::Left,
                None
            )
        );
    }

    Ok(())
}

fn print_grades_json(grades: &Vec<Grade>) -> Result<()> {
    let json = serde_json::to_string_pretty(&grades)?;
    println!("{json}");
    Ok(())
}

fn print_grades(grades: &Vec<Grade>, display: DisplayMode) -> Result<()> {
    match display {
        DisplayMode::Table => print_grades_table(grades),
        DisplayMode::Json => print_grades_json(grades),
    }
}

fn cmd_ci(args: CiArgs) -> Result<()> {
    let old = load_golden_file(&args.old)?;
    let new = load_golden_file(&args.new)?;

    let all_ids = old
        .keys()
        .chain(new.keys())
        .collect::<std::collections::HashSet<_>>();

    let grades: Vec<Grade> = all_ids
        .iter()
        .map(|id| {
            let old_entry = old.get(*id);
            let new_entry = new.get(*id);
            grade_entry(old_entry, new_entry)
        })
        .collect();

    print_grades(&grades, args.display)?;

    if grades
        .iter()
        .any(|g| matches!(g.status, Status::Incompatible { .. } | Status::Invalid))
    {
        println!("\nError: Found incompatible or invalid grades");
        std::process::exit(1);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_conversion() {
        let r: backcompat::Role = RoleCli::Serializer.into();
        assert!(matches!(r, backcompat::Role::Serializer));
    }
}
