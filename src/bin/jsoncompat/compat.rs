use anyhow::Result;
use owo_colors::OwoColorize;

use crate::{RoleCli, SchemaDoc, sample_incompat};
use jsoncompat as backcompat;

#[derive(clap::Args)]
pub(crate) struct CompatArgs {
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

pub(crate) fn cmd(args: CompatArgs) -> Result<()> {
    let old = SchemaDoc::load(&args.old)?;
    let new = SchemaDoc::load(&args.new)?;
    let role: backcompat::Role = args.role.into();

    let ok_static = backcompat::check_compat(&old.schema, &new.schema, role)?;

    let offender = if args.fuzz > 0 && !ok_static {
        let mut rng = rand::rng();
        sample_incompat(&old, &new, role, args.fuzz as usize, args.depth, &mut rng)?
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

    eprintln!(
        "{} Schemas are NOT backward-compatible (role = {:?})",
        "✘".red(),
        role
    );

    if let Some(ex) = offender {
        let pretty =
            serde_json::to_string_pretty(&ex).unwrap_or_else(|_| "<unserializable>".into());
        eprintln!("{} Counter-example:\n{}", "•".yellow(), pretty);
        let old_valid = old.is_valid(&ex)?;
        let new_valid = new.is_valid(&ex)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn compat_command_rejects_invalid_old_schema_before_reporting_a_verdict() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir();
        let old_path = dir.join(format!("jsoncompat-invalid-old-{unique}.json"));
        let new_path = dir.join(format!("jsoncompat-invalid-new-{unique}.json"));

        fs::write(&old_path, r#"{"type":"string","maxLength":"x"}"#).unwrap();
        fs::write(&new_path, r#"{"type":"string"}"#).unwrap();

        let error = cmd(CompatArgs {
            old: old_path.to_string_lossy().into_owned(),
            new: new_path.to_string_lossy().into_owned(),
            role: RoleCli::Serializer,
            fuzz: 0,
            depth: 8,
        })
        .unwrap_err();

        fs::remove_file(old_path).unwrap();
        fs::remove_file(new_path).unwrap();

        let message = format!("{error:#}");
        assert!(
            message.contains("building schema"),
            "unexpected error: {message}"
        );
        assert!(
            message.contains("keyword 'maxLength' at '#/maxLength' must be a non-negative integer"),
            "unexpected error: {message}"
        );
    }
}
