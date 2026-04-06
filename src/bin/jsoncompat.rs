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
#[path = "jsoncompat/compat.rs"]
mod compat;
#[path = "jsoncompat/demo.rs"]
mod demo;
#[path = "jsoncompat/generate.rs"]
mod generate;

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

        Ok(Self { schema })
    }

    #[inline]
    pub(crate) fn is_valid(&self, v: &Value) -> Result<bool> {
        Ok(self.schema.is_valid(v)?)
    }

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

// Sampling logic shared by fuzzing and counterexample search.
pub(crate) fn sample_incompat<R: Rng>(
    old: &SchemaDoc,
    new: &SchemaDoc,
    role: backcompat::Role,
    attempts: usize,
    depth: u8,
    rng: &mut R,
) -> Result<Option<Value>> {
    let mut try_once = |src: &SchemaDoc, dst: &SchemaDoc| -> Result<Option<Value>> {
        for _ in 0..attempts {
            let v = match src.gen_value(rng, depth) {
                Ok(value) => value,
                Err(GenerateError::Unsatisfiable | GenerateError::ExhaustedAttempts { .. }) => {
                    return Ok(None);
                }
                Err(error) => return Err(error.into()),
            };
            if src.is_valid(&v)? && !dst.is_valid(&v)? {
                return Ok(Some(v));
            }
        }
        Ok(None)
    };

    match role {
        backcompat::Role::Serializer => try_once(new, old),
        backcompat::Role::Deserializer => try_once(old, new),
        backcompat::Role::Both => try_once(new, old).and_then(|result| match result {
            Some(value) => Ok(Some(value)),
            None => try_once(old, new),
        }),
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
    /// Check backward-compatibility between two schema revisions.
    Compat(compat::CompatArgs),
    /// Check compatibility between two golden files.
    CI(ci::CiArgs),
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
        let schema = SchemaDoc {
            schema: backcompat::SchemaDocument::from_json(&json!({
                "type": "integer",
                "minimum": 1
            }))
            .unwrap(),
        };
        let mut rng = StdRng::seed_from_u64(7);

        let value = schema.gen_value(&mut rng, 4).unwrap();

        assert!(
            schema.is_valid(&value).unwrap(),
            "generated invalid value: {value}"
        );
    }

    #[test]
    fn gen_value_returns_unsatisfiable_for_false_schema() {
        let schema = SchemaDoc {
            schema: backcompat::SchemaDocument::from_json(&json!(false)).unwrap(),
        };
        let mut rng = StdRng::seed_from_u64(7);

        let error = schema.gen_value(&mut rng, 4).unwrap_err();

        assert!(matches!(error, GenerateError::Unsatisfiable));
    }

    #[test]
    fn sample_incompat_with_role_both_continues_after_exhausting_the_first_direction() {
        let old = SchemaDoc {
            schema: backcompat::SchemaDocument::from_json(&json!({})).unwrap(),
        };
        let new = SchemaDoc {
            schema: backcompat::SchemaDocument::from_json(&json!(false)).unwrap(),
        };
        let mut rng = StdRng::seed_from_u64(7);

        let offender = sample_incompat(&old, &new, backcompat::Role::Both, 3, 4, &mut rng).unwrap();

        let offender = offender.expect("expected a deserializer counterexample");
        assert!(
            old.is_valid(&offender).unwrap(),
            "old schema must accept {offender}"
        );
        assert!(
            !new.is_valid(&offender).unwrap(),
            "new schema must reject {offender}"
        );
    }
}
