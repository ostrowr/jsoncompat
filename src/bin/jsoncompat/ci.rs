use crate::{RoleCli, read_to_string};
use anyhow::{Context, Result};
use console::{Alignment, pad_str};
use jsoncompat as backcompat;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum DisplayMode {
    Table,
    Json,
}
#[derive(clap::Args)]
pub(crate) struct CiArgs {
    /// Path to the old golden file.
    old: String,
    /// Path to the new golden file.
    new: String,
    #[arg(short, long, value_enum, default_value_t = DisplayMode::Table)]
    display: DisplayMode,
}
#[derive(Deserialize)]
struct GoldenEntry {
    mode: RoleCli,
    schema: Value,
    stable_id: String,
}
type GoldenFile = std::collections::BTreeMap<String, GoldenEntry>;
fn load_golden_file(path: &str) -> Result<GoldenFile> {
    serde_json::from_str(&read_to_string(path)?)
        .with_context(|| format!("parsing golden file {path}"))
}
#[derive(Debug, PartialEq, Serialize)]
enum Status {
    Ok,
    MissingOld,
    MissingNew,
    ModeChanged,
    Incompatible { example: Value },
    Unknown { reason: String },
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
    let entry = new.or(old).expect("at least one golden entry");
    let status = match (old, new) {
        (Some(old), Some(new)) => grade_schemas(old, new),
        (Some(_), None) => Status::MissingNew,
        (None, Some(_)) => Status::MissingOld,
        (None, None) => unreachable!(),
    };
    Grade {
        id: entry.stable_id.clone(),
        mode: old.unwrap_or(entry).mode,
        status,
    }
}
fn grade_schemas(old: &GoldenEntry, new: &GoldenEntry) -> Status {
    let analyze = || -> Result<Status, backcompat::CompatibilityError> {
        let old_schema = backcompat::SchemaDocument::from_json(&old.schema)?;
        let new_schema = backcompat::SchemaDocument::from_json(&new.schema)?;
        let verdict = backcompat::analyze_compat(&old_schema, &new_schema, old.mode.into())?;
        Ok(match verdict {
            backcompat::CompatibilityResult::Incompatible { counterexample, .. } => {
                Status::Incompatible {
                    example: counterexample,
                }
            }
            backcompat::CompatibilityResult::Unknown { reason } => Status::Unknown { reason },
            backcompat::CompatibilityResult::Compatible if old.mode != new.mode => {
                Status::ModeChanged
            }
            backcompat::CompatibilityResult::Compatible if old.schema == new.schema => {
                Status::Identical
            }
            backcompat::CompatibilityResult::Compatible => Status::Ok,
        })
    };
    analyze().unwrap_or(Status::Invalid)
}
fn print_grades_table(grades: &[Grade]) {
    let mut rows = vec![[
        "ID".into(),
        "Mode".into(),
        "Status".into(),
        "Counterexample / reason".into(),
    ]];
    for grade in grades {
        let (status, detail) = match &grade.status {
            Status::Ok => ("Ok", String::new()),
            Status::MissingOld => ("MissingOld", String::new()),
            Status::MissingNew => ("MissingNew", String::new()),
            Status::ModeChanged => ("ModeChanged", String::new()),
            Status::Incompatible { example } => ("Incompatible", example.to_string()),
            Status::Unknown { reason } => ("Unknown", reason.clone()),
            Status::Invalid => ("Invalid", String::new()),
            Status::Identical => ("Identical", String::new()),
        };
        rows.push([
            grade.id.clone(),
            format!("{:?}", grade.mode),
            status.into(),
            detail,
        ]);
    }
    let widths: [usize; 4] = std::array::from_fn(|column| {
        rows.iter()
            .map(|row| console::measure_text_width(&row[column]))
            .max()
            .unwrap_or(0)
    });
    for row in rows {
        println!(
            "{}",
            row.iter()
                .zip(widths)
                .map(|(cell, width)| pad_str(cell, width, Alignment::Left, None).into_owned())
                .collect::<Vec<_>>()
                .join("  ")
                .trim_end()
        );
    }
}
pub(crate) fn cmd(args: CiArgs) -> Result<()> {
    let old = load_golden_file(&args.old)?;
    let new = load_golden_file(&args.new)?;
    let ids: std::collections::BTreeSet<_> = old.keys().chain(new.keys()).collect();
    let grades: Vec<_> = ids
        .into_iter()
        .map(|id| grade_entry(old.get(id), new.get(id)))
        .collect();
    match args.display {
        DisplayMode::Table => print_grades_table(&grades),
        DisplayMode::Json => println!("{}", serde_json::to_string_pretty(&grades)?),
    }
    // JSON stdout always contains exactly one JSON document.
    if grades
        .iter()
        .any(|g| matches!(g.status, Status::Incompatible { .. } | Status::Invalid))
    {
        eprintln!("Found incompatible or invalid grades");
        std::process::exit(1);
    }
    if grades
        .iter()
        .any(|g| matches!(g.status, Status::Unknown { .. }))
    {
        eprintln!("Compatibility could not be established for every grade");
        std::process::exit(2);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn ci_command_accepts_identical_canonicalized_golden_files() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir();
        let old_path = dir.join(format!("jsoncompat-ci-old-{unique}.json"));
        let new_path = dir.join(format!("jsoncompat-ci-new-{unique}.json"));
        let golden = r##"{
  "example": {
    "mode": "serializer",
    "schema": {
      "$schema": "https://json-schema.org/draft/2020-12/schema#",
      "type": "integer",
      "minimum": 1
    },
    "stable_id": "example"
  }
}"##;

        fs::write(&old_path, golden).unwrap();
        fs::write(&new_path, golden).unwrap();

        let result = cmd(CiArgs {
            old: old_path.to_string_lossy().into_owned(),
            new: new_path.to_string_lossy().into_owned(),
            display: DisplayMode::Json,
        });

        fs::remove_file(old_path).unwrap();
        fs::remove_file(new_path).unwrap();
        result.unwrap();
    }

    #[test]
    fn ci_grade_reports_incompatible_when_unique_items_is_relaxed_for_serializer() {
        let old = GoldenEntry {
            mode: RoleCli::Serializer,
            schema: serde_json::json!({
                "type": "array",
                "uniqueItems": true
            }),
            stable_id: "example".to_owned(),
        };
        let new = GoldenEntry {
            mode: RoleCli::Serializer,
            schema: serde_json::json!({
                "type": "array",
                "uniqueItems": false
            }),
            stable_id: "example".to_owned(),
        };

        let grade = grade_entry(Some(&old), Some(&new));

        assert!(matches!(grade.status, Status::Incompatible { .. }));
    }

    #[test]
    fn ci_grade_marks_invalid_schemas_before_compatibility_checks() {
        let old = GoldenEntry {
            mode: RoleCli::Serializer,
            schema: serde_json::json!({
                "type": "string",
                "maxLength": "x"
            }),
            stable_id: "example".to_owned(),
        };
        let new = GoldenEntry {
            mode: RoleCli::Serializer,
            schema: serde_json::json!({
                "type": "string"
            }),
            stable_id: "example".to_owned(),
        };

        let grade = grade_entry(Some(&old), Some(&new));

        assert_eq!(grade.status, Status::Invalid);
    }

    #[test]
    fn ci_grade_marks_backend_invalid_schemas_invalid() {
        let old = GoldenEntry {
            mode: RoleCli::Serializer,
            schema: serde_json::json!({
                "type": "string",
                "deprecated": "eventually"
            }),
            stable_id: "example".to_owned(),
        };
        let new = GoldenEntry {
            mode: RoleCli::Serializer,
            schema: serde_json::json!({
                "type": "string"
            }),
            stable_id: "example".to_owned(),
        };

        let grade = grade_entry(Some(&old), Some(&new));

        assert_eq!(grade.status, Status::Invalid);
    }

    #[test]
    fn ci_grade_marks_backend_invalid_ref_bearing_schemas_invalid() {
        let old = GoldenEntry {
            mode: RoleCli::Serializer,
            schema: serde_json::json!({
                "$defs": {
                    "Value": { "type": "string" }
                },
                "$ref": "#/$defs/Value",
                "deprecated": "eventually"
            }),
            stable_id: "example".to_owned(),
        };
        let new = GoldenEntry {
            mode: RoleCli::Serializer,
            schema: serde_json::json!({
                "type": "string"
            }),
            stable_id: "example".to_owned(),
        };

        let grade = grade_entry(Some(&old), Some(&new));

        assert_eq!(grade.status, Status::Invalid);
    }

    #[test]
    fn ci_grade_accepts_identical_unevaluated_schemas_without_warnings() {
        let schema = serde_json::json!({
            "type": "object",
            "unevaluatedProperties": false
        });
        let old = GoldenEntry {
            mode: RoleCli::Serializer,
            schema: schema.clone(),
            stable_id: "example".to_owned(),
        };
        let new = GoldenEntry {
            mode: RoleCli::Serializer,
            schema,
            stable_id: "example".to_owned(),
        };

        let grade = grade_entry(Some(&old), Some(&new));

        assert_eq!(grade.status, Status::Identical);
    }
}
