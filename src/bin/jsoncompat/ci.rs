use anyhow::{Context, Result};
use console::{Alignment, pad_str};
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{RoleCli, SchemaDoc, read_to_string, sample_incompat};
use jsoncompat as backcompat;

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum DisplayMode {
    Table,
    Json,
}

#[derive(clap::Args)]
pub(crate) struct CiArgs {
    /// Path to the *old* golden file.
    old: String,
    /// Path to the *new* golden file.
    new: String,
    /// Display mode.
    #[arg(short, long, value_enum, default_value_t = DisplayMode::Table)]
    display: DisplayMode,
}

#[derive(Deserialize)]
struct RawGoldenEntry {
    mode: RoleCli,
    schema: serde_json::Value,
    stable_id: String,
}

struct GoldenEntry {
    mode: RoleCli,
    schema: Value,
    stable_id: String,
}

type GoldenFile = std::collections::HashMap<String, GoldenEntry>;

fn load_golden_file(path: &str) -> Result<GoldenFile> {
    let raw = read_to_string(path)?;
    let golden: std::collections::HashMap<String, RawGoldenEntry> =
        serde_json::from_str(&raw).with_context(|| format!("parsing golden file {path}"))?;

    golden
        .into_iter()
        .map(|(id, entry)| {
            Ok((
                id,
                GoldenEntry {
                    mode: entry.mode,
                    schema: entry.schema,
                    stable_id: entry.stable_id,
                },
            ))
        })
        .collect()
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
                backcompat::SchemaDocument::from_json(&old.schema),
                backcompat::SchemaDocument::from_json(&new.schema),
            );
            match (old_schema, new_schema) {
                (Ok(old_schema), Ok(new_schema)) => {
                    if old.mode == new.mode && old.schema == new.schema {
                        return Grade {
                            id: new.stable_id.clone(),
                            mode: old.mode,
                            status: Status::Identical,
                        };
                    }
                    let Ok(ok) =
                        backcompat::check_compat(&old_schema, &new_schema, old.mode.into())
                    else {
                        return Grade {
                            id: new.stable_id.clone(),
                            mode: old.mode,
                            status: Status::Invalid,
                        };
                    };
                    if !ok {
                        let mut rng = rand::rng();
                        let example = match sample_incompat(
                            &SchemaDoc {
                                raw: old.schema.clone(),
                                schema: old_schema,
                            },
                            &SchemaDoc {
                                raw: new.schema.clone(),
                                schema: new_schema,
                            },
                            old.mode.into(),
                            100,
                            8,
                            &mut rng,
                        ) {
                            Ok(example) => example,
                            Err(_) => {
                                return Grade {
                                    id: new.stable_id.clone(),
                                    mode: old.mode,
                                    status: Status::Invalid,
                                };
                            }
                        };
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
    let header_id = "ID";
    let header_mode = "Mode";
    let header_status = "Status";
    let header_example = "Example";

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

    println!(
        "{}  {}  {}  {}",
        pad_str("", id_width, Alignment::Left, Some("-")),
        pad_str("", mode_width, Alignment::Left, Some("-")),
        pad_str("", status_width, Alignment::Left, Some("-")),
        pad_str("", example_width, Alignment::Left, Some("-"))
    );

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

pub(crate) fn cmd(args: CiArgs) -> Result<()> {
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
}
