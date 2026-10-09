//! Command-line interface for the `jsoncompat` crate.

use std::{
    fs,
    io::{self, Read},
    path::Path,
};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use jsoncompat as backcompat;

use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use json_schema_fuzz::{GenerateError, GenerationConfig, ValueGenerator};

#[path = "jsoncompat/ci.rs"]
mod ci;
#[path = "jsoncompat/codegen.rs"]
mod codegen;
#[path = "jsoncompat/codegen_package.rs"]
mod codegen_package;
#[path = "jsoncompat/compat.rs"]
mod compat;
#[path = "jsoncompat/demo.rs"]
mod demo;
#[path = "jsoncompat/generate.rs"]
mod generate;
#[path = "jsoncompat/lower_openapi.rs"]
mod lower_openapi;
#[path = "jsoncompat/stamp.rs"]
mod stamp;

/// In-memory representation of a parsed schema document.
#[derive(Debug)]
pub(crate) struct SchemaDoc {
    pub(crate) schema: backcompat::SchemaDocument,
}

impl SchemaDoc {
    pub(crate) fn load(path: &str) -> Result<Self> {
        let raw = read_to_string(path)?;
        let json: Value = serde_json::from_str(&raw).with_context(|| format!("parsing {path}"))?;

        let schema = backcompat::SchemaDocument::from_json(&json)
            .with_context(|| format!("building schema for {path}"))?;
        schema
            .root()
            .with_context(|| format!("resolving schema for {path}"))?;
        schema
            .validate_source_schema()
            .with_context(|| format!("validating schema for {path}"))?;

        Ok(Self { schema })
    }

    #[inline]
    pub(crate) fn gen_value<R: Rng>(
        &self,
        rng: &mut R,
        depth: u8,
    ) -> std::result::Result<Value, GenerateError> {
        ValueGenerator::generate(&self.schema, GenerationConfig::new(depth), rng)
    }
}

/// Read an entire file (or stdin) into a string.
pub(crate) fn read_to_string(path: &str) -> Result<String> {
    if path == "-" {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        Ok(buf)
    } else {
        fs::read_to_string(Path::new(path)).with_context(|| format!("reading {path}"))
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
    Generate(generate::GenerateArgs),
    /// Check raw JSON Schemas, or OpenAPI 3.1 contracts with --openapi.
    Compat(compat::CompatArgs),
    /// Check compatibility between two golden files.
    CI(ci::CiArgs),
    /// Stamp a schema into versioned writer/reader envelopes.
    Stamp(stamp::StampArgs),
    /// Generate code or normalized schema output from JSON Schema.
    Codegen(codegen::CodegenArgs),
    /// Print the lowered JSON Schema contracts for an OpenAPI 3.1 document.
    #[command(name = "lower-openapi")]
    LowerOpenApi(lower_openapi::LowerOpenApiArgs),
    /// Run a guided end-to-end demo of generate, compat, and ci.
    Demo(demo::DemoArgs),
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum RoleCli {
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
        Command::Generate(a) => generate::cmd(a),
        Command::Compat(a) => compat::cmd(a),
        Command::CI(a) => ci::cmd(a),
        Command::Stamp(a) => stamp::cmd(a),
        Command::Codegen(a) => codegen::cmd(a),
        Command::LowerOpenApi(a) => lower_openapi::cmd(a),
        Command::Demo(a) => demo::cmd(a),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};
    use serde_json::json;

    #[test]
    fn role_conversion() {
        let r: backcompat::Role = RoleCli::Serializer.into();
        assert!(matches!(r, backcompat::Role::Serializer));
    }

    #[test]
    fn gen_value_retries_until_raw_schema_accepts_the_candidate() {
        let raw = json!({
            "type": "integer",
            "minimum": 1
        });
        let schema = SchemaDoc {
            schema: backcompat::SchemaDocument::from_json(&raw).unwrap(),
        };
        let mut rng = StdRng::seed_from_u64(7);

        let value = schema.gen_value(&mut rng, 4).unwrap();

        assert!(
            schema.schema.is_valid(&value).unwrap(),
            "generated invalid value: {value}"
        );
    }

    #[test]
    fn gen_value_returns_unsatisfiable_for_false_schema() {
        let raw = json!(false);
        let schema = SchemaDoc {
            schema: backcompat::SchemaDocument::from_json(&raw).unwrap(),
        };
        let mut rng = StdRng::seed_from_u64(7);

        let error = schema.gen_value(&mut rng, 4).unwrap_err();

        assert!(matches!(error, GenerateError::Unsatisfiable));
    }
}
