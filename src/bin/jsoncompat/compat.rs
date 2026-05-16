use anyhow::{Context, Result, bail};
use owo_colors::OwoColorize;
use serde_json::Value;

use crate::{RoleCli, SchemaDoc, read_to_string, sample_incompat};
use jsoncompat as backcompat;

#[derive(clap::Args)]
pub(crate) struct CompatArgs {
    /// Path to the *old* JSON Schema or OpenAPI 3.1 document.
    old: String,
    /// Path to the *new* JSON Schema or OpenAPI 3.1 document.
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
    let old = CompatInput::load(&args.old)?;
    let new = CompatInput::load(&args.new)?;
    let role: backcompat::Role = args.role.into();

    match (old, new) {
        (CompatInput::Schema(old), CompatInput::Schema(new)) => {
            compat_schemas(old, new, role, args.fuzz, args.depth)
        }
        (CompatInput::OpenApi(old), CompatInput::OpenApi(new)) => {
            if args.role != RoleCli::Both {
                bail!(
                    "--role is only available for raw JSON Schema inputs; OpenAPI comparisons check request and response compatibility together"
                );
            }
            if args.fuzz > 0 {
                bail!("--fuzz is only available for raw JSON Schema inputs");
            }
            compat_openapi(old, new)
        }
        _ => bail!("compat inputs must both be raw JSON Schemas or both be OpenAPI 3.1 documents"),
    }
}

enum CompatInput {
    Schema(SchemaDoc),
    OpenApi(backcompat::OpenApiDocument),
}

impl CompatInput {
    fn load(path: &str) -> Result<Self> {
        let raw = read_to_string(path)?;
        let json: Value = serde_json::from_str(&raw).with_context(|| format!("parsing {path}"))?;
        if looks_like_openapi_document(&json) {
            let document = backcompat::OpenApiDocument::from_json(&json)
                .with_context(|| format!("building OpenAPI document for {path}"))?;
            return Ok(Self::OpenApi(document));
        }

        let schema = backcompat::SchemaDocument::from_json(&json)
            .with_context(|| format!("building schema for {path}"))?;
        Ok(Self::Schema(SchemaDoc { schema }))
    }
}

fn looks_like_openapi_document(json: &Value) -> bool {
    let Some(object) = json.as_object() else {
        return false;
    };
    object.contains_key("openapi")
}

fn compat_schemas(
    old: SchemaDoc,
    new: SchemaDoc,
    role: backcompat::Role,
    fuzz: u32,
    depth: u8,
) -> Result<()> {
    let ok_static = backcompat::check_compat(&old.schema, &new.schema, role)?;
    let offender = if fuzz > 0 && !ok_static {
        let mut rng = rand::rng();
        sample_incompat(&old, &new, role, fuzz as usize, depth, &mut rng)?
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
    if let Some(detail) = backcompat::explain_compat_failure(&old.schema, &new.schema, role)? {
        eprintln!("{} {}", "•".yellow(), detail);
    }

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

fn compat_openapi(
    old: backcompat::OpenApiDocument,
    new: backcompat::OpenApiDocument,
) -> Result<()> {
    let report = backcompat::check_openapi_compat(&old, &new)?;
    if report.is_compatible() {
        eprintln!("{} OpenAPI documents seem backward-compatible", "✔".green());
        return Ok(());
    }

    eprintln!(
        "{} OpenAPI documents are NOT backward-compatible",
        "✘".red()
    );
    for issue in report.issues() {
        eprintln!(
            "{} {} {} {:?}: {}",
            "•".yellow(),
            issue.method,
            issue.path,
            issue.surface,
            issue.message
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

    #[test]
    fn compat_command_accepts_identical_openapi_documents() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir();
        let old_path = dir.join(format!("jsoncompat-openapi-old-{unique}.json"));
        let new_path = dir.join(format!("jsoncompat-openapi-new-{unique}.json"));
        let openapi = r#"{
  "openapi": "3.1.0",
  "info": { "title": "Pets", "version": "1.0.0" },
  "paths": {
    "/pets": {
      "get": {
        "responses": {
          "200": {
            "description": "ok",
            "content": {
              "application/json": {
                "schema": { "type": "object" }
              }
            }
          }
        }
      }
    }
  }
}"#;

        fs::write(&old_path, openapi).unwrap();
        fs::write(&new_path, openapi).unwrap();

        let result = cmd(CompatArgs {
            old: old_path.to_string_lossy().into_owned(),
            new: new_path.to_string_lossy().into_owned(),
            role: RoleCli::Both,
            fuzz: 0,
            depth: 8,
        });

        fs::remove_file(old_path).unwrap();
        fs::remove_file(new_path).unwrap();
        result.unwrap();
    }

    #[test]
    fn malformed_openapi_inputs_do_not_fall_back_to_raw_schema_mode() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "jsoncompat-schema-with-openapi-annotation-{unique}.json"
        ));
        fs::write(&path, r#"{"openapi":"3.1.0","type":"string"}"#).unwrap();

        let error = match CompatInput::load(&path.to_string_lossy()) {
            Err(error) => error,
            Ok(_) => panic!("top-level `openapi` must route through OpenAPI validation"),
        };

        fs::remove_file(path).unwrap();
        let message = format!("{error:#}");
        assert!(message.contains("building OpenAPI document"), "{message}");
        assert!(message.contains("#/info"), "{message}");
    }

    #[test]
    fn compat_command_rejects_openapi_role_flags() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir();
        let old_path = dir.join(format!("jsoncompat-openapi-role-old-{unique}.json"));
        let new_path = dir.join(format!("jsoncompat-openapi-role-new-{unique}.json"));
        let openapi = r#"{
  "openapi": "3.1.0",
  "info": { "title": "Pets", "version": "1.0.0" },
  "paths": {
    "/pets": {
      "get": {
        "responses": {
          "200": { "description": "ok" }
        }
      }
    }
  }
}"#;

        fs::write(&old_path, openapi).unwrap();
        fs::write(&new_path, openapi).unwrap();

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

        assert!(
            error.to_string().contains("--role is only available"),
            "unexpected error: {error:#}"
        );
    }

    #[test]
    fn compat_command_rejects_unsupported_openapi_versions_before_schema_fallback() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir();
        let old_path = dir.join(format!("jsoncompat-openapi-30-old-{unique}.json"));
        let new_path = dir.join(format!("jsoncompat-openapi-30-new-{unique}.json"));
        let openapi = r#"{
  "openapi": "3.0.3",
  "info": { "title": "Pets", "version": "1.0.0" },
  "paths": {}
}"#;

        fs::write(&old_path, openapi).unwrap();
        fs::write(&new_path, openapi).unwrap();

        let error = cmd(CompatArgs {
            old: old_path.to_string_lossy().into_owned(),
            new: new_path.to_string_lossy().into_owned(),
            role: RoleCli::Both,
            fuzz: 0,
            depth: 8,
        })
        .unwrap_err();

        fs::remove_file(old_path).unwrap();
        fs::remove_file(new_path).unwrap();

        let message = format!("{error:#}");
        assert!(message.contains("building OpenAPI document"), "{message}");
        assert!(
            message.contains("unsupported OpenAPI version '3.0.3'"),
            "{message}"
        );
    }

    #[test]
    fn schema_compat_explanation_describes_a_property_type_widening() {
        let old = backcompat::SchemaDocument::from_json(&serde_json::json!({
            "type": "object",
            "properties": {
                "preamble": { "type": "string" }
            }
        }))
        .unwrap();
        let new = backcompat::SchemaDocument::from_json(&serde_json::json!({
            "type": "object",
            "properties": {
                "preamble": { "type": ["string", "object"] }
            }
        }))
        .unwrap();

        let detail = backcompat::explain_compat_failure(&old, &new, backcompat::Role::Serializer)
            .unwrap()
            .unwrap();

        assert!(
            detail.contains("new schema #/properties/preamble"),
            "{detail}"
        );
        assert!(detail.contains("property 'preamble'"), "{detail}");
    }
}
