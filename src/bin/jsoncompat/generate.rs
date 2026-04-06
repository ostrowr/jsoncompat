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
