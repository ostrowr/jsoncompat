//! Fuzzer tests.

use json_schema_ast::compile;
use json_schema_fuzz::{generate_value, satisfiability, Satisfiability};
use jsoncompat::build_and_resolve_schema;
use rand::{rngs::StdRng, SeedableRng};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::thread;
use tiny_http::{Response, Server, StatusCode};

const N_ITERATIONS: usize = 1000;

/// Load the temporary whitelist that allows individual failures to be marked
/// as expected while we iteratively improve the fuzzer.
///
/// ```jsonc
/// {
///   "recursive.json": [0, 3, 4]      // skip specific schema indices
/// }
/// ```
fn load_whitelist() -> HashMap<String, HashSet<usize>> {
    let map: HashMap<String, HashSet<usize>> = HashMap::new();

    map
}

fn remote_root() -> &'static PathBuf {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        fs::canonicalize("tests/fixtures").expect("missing tests/fixtures directory")
    })
}

fn start_fixture_server() {
    static START: OnceLock<()> = OnceLock::new();
    START.get_or_init(|| {
        let server = Server::http("127.0.0.1:0").expect("failed to start fixture server");
        let addr_str = server.server_addr().to_string();
        let port: u16 = addr_str
            .rsplit(':')
            .next()
            .and_then(|p| p.parse().ok())
            .unwrap_or(1234);
        std::env::set_var("JSONCOMPAT_FIXTURE_PORT", port.to_string());
        thread::spawn(move || {
            for req in server.incoming_requests() {
                let path = req.url().trim_start_matches('/');
                if let Some(body) = remote_body(path) {
                    let response = Response::from_data(body).with_status_code(StatusCode(200));
                    let _ = req.respond(response);
                } else {
                    let _ =
                        req.respond(Response::from_string("").with_status_code(StatusCode(404)));
                }
            }
        });
    });
}

fn remote_body(path: &str) -> Option<Vec<u8>> {
    let root = remote_root();
    let requested = root.join(path.trim_start_matches('/'));
    if let Ok(canon) = requested.canonicalize() {
        if !canon.starts_with(root) {
            return None;
        }
        return fs::read(canon).ok();
    }
    None
}

// -------------------------------------------------------------------------
// Test harness: one test per JSON file under fixtures/fuzz
// -------------------------------------------------------------------------

datatest_stable::harness!(fixture, "tests/fixtures/fuzz", ".*\\.json$");

fn fixture(file: &Path) -> Result<(), Box<dyn std::error::Error>> {
    start_fixture_server();

    let bytes = fs::read(file)?;
    let root: Value = serde_json::from_slice(&bytes)?;

    // Collect all schemas contained in the file.  For the official test‑suite
    // this is typically an array of objects each with a `schema` member.
    let mut schemas = Vec::new();
    match &root {
        Value::Array(groups) => {
            for item in groups {
                if let Some(s) = item.get("schema") {
                    schemas.push(s.clone());
                }
            }
        }
        v => {
            // Fallback: treat the entire document as a single schema.
            schemas.push(v.clone());
        }
    }

    // Deterministic RNG per file for reproducibility.
    let seed = 0xBADBABE + file.to_string_lossy().len() as u64;
    let mut rng = StdRng::seed_from_u64(seed);

    // Whitelist lookup key – path relative to the fixtures root.
    let rel_path = file.strip_prefix("tests/fixtures/fuzz").unwrap_or(file);
    let rel_str = rel_path.to_string_lossy().replace('\\', "/");

    let whitelist = load_whitelist();
    let allowed = whitelist.get::<str>(rel_str.as_ref());

    for (idx, schema_json) in schemas.iter().enumerate() {
        // Skip `false` schemas – they have an empty instance set by design.
        if schema_json == &Value::Bool(false) {
            continue;
        }

        let ast = build_and_resolve_schema(schema_json)?;

        if matches!(satisfiability(&ast), Satisfiability::Never) {
            continue;
        }

        let compiled = compile(&ast.to_json())?;

        let is_whitelisted = allowed.map(|set| set.contains(&idx)).unwrap_or(false);

        let mut success = true;
        for _ in 0..N_ITERATIONS {
            let candidate = generate_value(&ast, &mut rng, 6);
            if !compiled.is_valid(&candidate) {
                if !allowed.map(|set| set.contains(&idx)).unwrap_or(false) {
                    panic!(
                            "{}", &format!(
                                "Failed to generate a valid instance for schema #{idx} in {}\n\nSchema:\n{}\n\nInstance:\n{}",
                                rel_str,
                                serde_json::to_string_pretty(&schema_json)?,
                                serde_json::to_string_pretty(&candidate)?
                            )
                        );
                }
                success = false;
                break;
            }
        }

        match (success, is_whitelisted) {
            (true, false) => { /* success as expected */ }
            (true, true) => {
                // This schema was previously whitelisted but now passes – flag it.
                panic!(
                    "Whitelisted failure now passes; please remove entry for schema #{idx} in {rel_str}"
                );
            }
            (false, true) => {
                // Allowed failure – proceed.
            }
            (false, false) => {
                panic!("Should have panicked above, but didn't: schema #{idx} in {rel_str}");
            }
        }
    }

    Ok(())
}
