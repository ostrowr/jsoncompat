use anyhow::Result;

use crate::SchemaDoc;

#[derive(clap::Args)]
pub(crate) struct GenerateArgs {
    /// Path to the JSON Schema. Use '-' for STDIN.
    schema: String,
    /// How many instances to emit.
    #[arg(short, long, default_value_t = 1)]
    count: u32,
    /// Maximum recursion depth.
    #[arg(short, long, default_value_t = 8)]
    depth: u8,
    /// Pretty-print output (multi-line).
    #[arg(short, long)]
    pretty: bool,
}

pub(crate) fn cmd(args: GenerateArgs) -> Result<()> {
    let schema = SchemaDoc::load(&args.schema)?;
    let mut rng = rand::rng();

    for _ in 0..args.count {
        let v = schema.gen_value(&mut rng, args.depth)?;
        if args.pretty {
            println!("{}", serde_json::to_string_pretty(&v)?);
        } else {
            println!("{}", serde_json::to_string(&v)?);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn generate_command_rejects_invalid_schemas_before_emitting_values() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("jsoncompat-generate-invalid-{unique}.json"));
        fs::write(&path, r#"{"type":"string","maxLength":"x"}"#).unwrap();

        let error = cmd(GenerateArgs {
            schema: path.to_string_lossy().into_owned(),
            count: 1,
            depth: 8,
            pretty: false,
        })
        .unwrap_err();

        fs::remove_file(path).unwrap();

        let message = format!("{error:#}");
        assert!(message.contains("building schema"), "{message}");
        assert!(message.contains("maxLength"), "{message}");
    }
}
