use std::{
    env, fs,
    io::{self, IsTerminal, Write},
    path::PathBuf,
    process,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use serde_json::Value;

#[derive(clap::Args)]
pub(crate) struct DemoArgs {
    /// Run without pausing between steps.
    #[arg(short = 'n', long)]
    noninteractive: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExpectedStatus {
    Ok,
    Fail,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DemoOutputFormat {
    Text,
    Json,
}

struct DemoTempDir {
    path: PathBuf,
}

impl DemoTempDir {
    fn create() -> Result<Self> {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path = env::temp_dir().join(format!("jsoncompat-demo-{}-{unique}", process::id()));
        fs::create_dir(&path).with_context(|| format!("creating {}", path.display()))?;
        Ok(Self { path })
    }

    fn join(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }
}

impl Drop for DemoTempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

struct Demo {
    temp_dir: DemoTempDir,
    interactive: bool,
    step_index: u8,
    exe_path: PathBuf,
}

impl Demo {
    fn create(noninteractive: bool) -> Result<Self> {
        Ok(Self {
            temp_dir: DemoTempDir::create()?,
            interactive: !noninteractive,
            step_index: 0,
            exe_path: env::current_exe().context("finding current jsoncompat executable")?,
        })
    }

    fn run(&mut self) -> Result<()> {
        println!("{}", "jsoncompat CLI demo".bright_blue());
        println!(
            "{} {}",
            "Temporary fixtures:".bright_black(),
            self.temp_dir.path.display()
        );

        self.create_fixtures()?;
        self.run_demo()?;

        println!("\n{}", "✔ demo completed successfully".green());
        Ok(())
    }

    fn create_fixtures(&self) -> Result<()> {
        self.write_fixture(
            "compat-old.json",
            r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "integer",
  "minimum": 0,
  "maximum": 10
}
"#,
        )?;
        self.write_fixture(
            "compat-new.json",
            r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "integer",
  "minimum": 2,
  "maximum": 8
}
"#,
        )?;
        self.write_fixture(
            "incompat-old.json",
            r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "string",
  "minLength": 2
}
"#,
        )?;
        self.write_fixture(
            "incompat-new.json",
            r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "integer",
  "minimum": 1
}
"#,
        )?;
        self.write_fixture(
            "sat-schema.json",
            r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "id": { "type": "integer", "minimum": 1 },
    "tags": {
      "type": "array",
      "items": { "type": "string", "minLength": 1 },
      "contains": { "pattern": "^(?:prod|dev)$" },
      "minContains": 1,
      "minItems": 1,
      "maxItems": 3,
      "uniqueItems": true
    }
  },
  "required": ["id", "tags"],
  "additionalProperties": false
}
"#,
        )?;
        self.write_fixture(
            "unsat-schema.json",
            r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "array",
  "minItems": 2,
  "maxItems": 1
}
"#,
        )?;
        self.write_fixture(
            "golden-old.json",
            r#"{
  "compatible_field": {
    "mode": "serializer",
    "stable_id": "compatible_field",
    "schema": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "type": "integer",
      "minimum": 0
    }
  },
  "incompatible_field": {
    "mode": "serializer",
    "stable_id": "incompatible_field",
    "schema": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "type": "string",
      "minLength": 1
    }
  }
}
"#,
        )?;
        self.write_fixture(
            "golden-compatible.json",
            r#"{
  "compatible_field": {
    "mode": "serializer",
    "stable_id": "compatible_field",
    "schema": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "type": "integer",
      "minimum": 2
    }
  },
  "incompatible_field": {
    "mode": "serializer",
    "stable_id": "incompatible_field",
    "schema": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "type": "string",
      "minLength": 3
    }
  }
}
"#,
        )?;
        self.write_fixture(
            "golden-incompatible.json",
            r#"{
  "compatible_field": {
    "mode": "serializer",
    "stable_id": "compatible_field",
    "schema": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "type": "integer",
      "minimum": 2
    }
  },
  "incompatible_field": {
    "mode": "serializer",
    "stable_id": "incompatible_field",
    "schema": {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "type": "integer",
      "minimum": 1
    }
  }
}
"#,
        )
    }

    fn write_fixture(&self, name: &str, contents: &str) -> Result<()> {
        let path = self.temp_dir.join(name);
        fs::write(&path, contents).with_context(|| format!("writing {}", path.display()))
    }

    fn run_demo(&mut self) -> Result<()> {
        self.section(
            "Generate valid sample data",
            "emit raw-valid JSON instances",
        )?;
        self.show_schema("Satisfiable schema", "sat-schema.json")?;
        self.run_jsoncompat(
            ExpectedStatus::Ok,
            DemoOutputFormat::Json,
            &[
                "generate".to_owned(),
                self.fixture_arg("sat-schema.json"),
                "--count".to_owned(),
                "3".to_owned(),
                "--depth".to_owned(),
                "6".to_owned(),
                "--pretty".to_owned(),
            ],
        )?;

        self.section("Reject unsatisfiable schemas", "generation must fail")?;
        self.show_schema("Unsatisfiable schema", "unsat-schema.json")?;
        self.run_jsoncompat(
            ExpectedStatus::Fail,
            DemoOutputFormat::Text,
            &[
                "generate".to_owned(),
                self.fixture_arg("unsat-schema.json"),
                "--count".to_owned(),
                "1".to_owned(),
                "--depth".to_owned(),
                "4".to_owned(),
            ],
        )?;

        self.section("Compatible serializer change", "static check should pass")?;
        self.show_schema("Old schema", "compat-old.json")?;
        self.show_schema("New schema", "compat-new.json")?;
        self.run_jsoncompat(
            ExpectedStatus::Ok,
            DemoOutputFormat::Text,
            &[
                "compat".to_owned(),
                self.fixture_arg("compat-old.json"),
                self.fixture_arg("compat-new.json"),
                "--role".to_owned(),
                "serializer".to_owned(),
            ],
        )?;

        self.section(
            "Incompatible serializer change",
            "fuzzed counterexample must fail",
        )?;
        self.show_schema("Old schema", "incompat-old.json")?;
        self.show_schema("New schema", "incompat-new.json")?;
        self.run_jsoncompat(
            ExpectedStatus::Fail,
            DemoOutputFormat::Text,
            &[
                "compat".to_owned(),
                self.fixture_arg("incompat-old.json"),
                self.fixture_arg("incompat-new.json"),
                "--role".to_owned(),
                "serializer".to_owned(),
                "--fuzz".to_owned(),
                "64".to_owned(),
                "--depth".to_owned(),
                "6".to_owned(),
            ],
        )?;

        self.section(
            "CI grading in JSON mode",
            "compatible golden set should pass",
        )?;
        self.show_schema("Old golden file", "golden-old.json")?;
        self.show_schema("New golden file", "golden-compatible.json")?;
        self.run_jsoncompat(
            ExpectedStatus::Ok,
            DemoOutputFormat::Json,
            &[
                "ci".to_owned(),
                self.fixture_arg("golden-old.json"),
                self.fixture_arg("golden-compatible.json"),
                "--display".to_owned(),
                "json".to_owned(),
            ],
        )?;

        self.section(
            "CI grading in table mode",
            "incompatible golden set must fail",
        )?;
        self.show_schema("Old golden file", "golden-old.json")?;
        self.show_schema("New golden file", "golden-incompatible.json")?;
        self.run_jsoncompat(
            ExpectedStatus::Fail,
            DemoOutputFormat::Text,
            &[
                "ci".to_owned(),
                self.fixture_arg("golden-old.json"),
                self.fixture_arg("golden-incompatible.json"),
                "--display".to_owned(),
                "table".to_owned(),
            ],
        )
    }

    fn section(&mut self, title: &str, expected: &str) -> Result<()> {
        self.step_index += 1;
        println!(
            "\n{}",
            format!("[{:02}] {title}", self.step_index).magenta()
        );
        println!("{} {}", "Expected:".bright_black(), expected);

        if !self.interactive {
            return Ok(());
        }
        if !io::stdin().is_terminal() {
            anyhow::bail!("interactive demo requires a TTY; rerun with --noninteractive");
        }

        print!(
            "{} ",
            "Press Enter to continue (q + Enter to quit):".bright_blue()
        );
        io::stdout().flush()?;
        let mut reply = String::new();
        io::stdin().read_line(&mut reply)?;
        if matches!(reply.trim(), "q" | "Q") {
            println!("{}", "Demo stopped before running this section.".yellow());
            process::exit(0);
        }
        Ok(())
    }

    fn show_schema(&self, title: &str, name: &str) -> Result<()> {
        self.subsection(title);
        let path = self.temp_dir.join(name);
        let raw =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let json: Value =
            serde_json::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
        println!("{}", serde_json::to_string_pretty(&json)?);
        Ok(())
    }

    fn run_jsoncompat(
        &self,
        expected_status: ExpectedStatus,
        output_format: DemoOutputFormat,
        args: &[String],
    ) -> Result<()> {
        self.subsection("Command");
        self.log_command(args);

        let output = process::Command::new(&self.exe_path)
            .args(args)
            .output()
            .with_context(|| format!("running {}", self.exe_path.display()))?;
        let succeeded = output.status.success();
        let combined_output = String::from_utf8_lossy(&output.stdout).into_owned()
            + &String::from_utf8_lossy(&output.stderr);

        match (expected_status, succeeded) {
            (ExpectedStatus::Ok, true) => {
                self.show_output(output_format, &combined_output)?;
                println!("{}", "✔ command succeeded".green());
                Ok(())
            }
            (ExpectedStatus::Ok, false) => {
                print!("{combined_output}");
                anyhow::bail!(
                    "expected command to succeed, but it exited with {}",
                    output.status
                );
            }
            (ExpectedStatus::Fail, true) => {
                print!("{combined_output}");
                anyhow::bail!("expected command to fail, but it succeeded");
            }
            (ExpectedStatus::Fail, false) => {
                self.show_output(output_format, &combined_output)?;
                println!("{}", "✔ expected failure observed".yellow());
                Ok(())
            }
        }
    }

    fn show_output(&self, output_format: DemoOutputFormat, output: &str) -> Result<()> {
        self.subsection("Output");
        if output_format == DemoOutputFormat::Json {
            match serde_json::from_str::<Value>(output) {
                Ok(value) => {
                    println!("{}", serde_json::to_string_pretty(&value)?);
                    return Ok(());
                }
                Err(_) => {
                    // Some demo commands emit multiple JSON documents. Print those
                    // streams as-is instead of inventing a wrapper array.
                }
            }
        }
        print!("{output}");
        Ok(())
    }

    fn subsection(&self, title: &str) {
        println!("\n{}", format!("-- {title} --").bright_blue());
    }

    fn log_command(&self, args: &[String]) {
        print!("{}", "$".cyan());
        print!(" jsoncompat");
        for arg in args {
            print!(" {}", shell_quote(arg));
        }
        println!();
    }

    fn fixture_arg(&self, name: &str) -> String {
        self.temp_dir.join(name).display().to_string()
    }
}

fn shell_quote(value: &str) -> String {
    if value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '-' | '_' | ':' | '='))
    {
        value.to_owned()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

pub(crate) fn cmd(args: DemoArgs) -> Result<()> {
    Demo::create(args.noninteractive)?.run()
}
