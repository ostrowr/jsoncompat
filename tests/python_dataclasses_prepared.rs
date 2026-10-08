use json_schema_ast::SchemaDocument;
use jsoncompat_codegen::{generate_dataclass_models, generate_dataclass_module_from_document};
use serde_json::json;
use std::fs;

#[path = "support/python_env.rs"]
mod python_env;

#[test]
fn prepared_dataclasses_preserve_runtime_and_fixture_contracts() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "jsoncompat-prepared-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&directory).expect("create prepared test directory");
    let mut schemas = vec![
        (
            "constrained",
            json!({
                "title": "Constrained", "type": "object",
                "properties": {
                    "name": {"type": "string", "minLength": 1},
                    "count": {"type": "integer", "minimum": 0},
                    "optional": {"type": ["string", "null"]}
                },
                "required": ["name", "count"], "additionalProperties": false
            }),
        ),
        (
            "empty",
            json!({"title":"Empty", "type":"object", "additionalProperties":false}),
        ),
        (
            "extras",
            json!({
                "title": "Extras", "type": "object",
                "properties": {"name": {"type": "string", "minLength": 1}},
                "required": ["name"], "additionalProperties": {"type": "string"}
            }),
        ),
        (
            "regex",
            json!({"title": "Pattern", "type": "string", "pattern": "^(?=x+$)x{1,3}$"}),
        ),
        (
            "ambiguous",
            json!({
                "title": "Ambiguous", "oneOf": [
                    {"type": "object", "properties": {"value": {"type": "integer", "minimum": 0}}, "required": ["value"], "additionalProperties": false},
                    {"type": "object", "properties": {"value": {"type": "integer", "maximum": -1}}, "required": ["value"], "additionalProperties": false}
                ]
            }),
        ),
        (
            "nested_regex",
            json!({"title": "Patterns", "type": "object", "patternProperties": {"^x": {"type": "string", "pattern": "^[a-z]+$"}}, "additionalProperties": false}),
        ),
        (
            "escaped",
            json!({"type": "object", "properties": {"a\"b\\c\n🐲": {"type": "integer"}}, "required": ["a\"b\\c\n🐲"], "additionalProperties": false}),
        ),
    ];
    schemas.push(("duplicate_oneof", json!({"oneOf":[true,true,true]})));
    schemas.push((
        "duplicate_leaf_oneof",
        json!({"oneOf":[
            {"type":"string","minLength":1}, {"type":"string","minLength":1}, {"type":"integer"}
        ]}),
    ));
    for (name, pattern) in [
        ("word_boundary", r"\bcat\b"),
        ("non_boundary", r"\Bcat\B"),
        ("boundary_start", r"\b{start}cat"),
        ("boundary_end", r"cat\b{end}"),
        ("boundary_start_half", r"\b{start-half}cat"),
        ("boundary_end_half", r"cat\b{end-half}"),
        ("boundary_look", r"(?<=\b)cat(?=\b)"),
    ] {
        schemas.push((name, json!({"type":"string", "pattern":pattern})));
    }
    let costly_pattern = "(?=a+$)a+$";
    schemas.push((
        "budget_not",
        json!({"type":"string", "not":{"pattern":costly_pattern}}),
    ));
    schemas.push((
        "budget_if",
        json!({"type":"string", "if":{"pattern":costly_pattern}, "then":false}),
    ));
    schemas.push(("budget_keys", json!({"type":"object", "patternProperties":{costly_pattern:false}, "additionalProperties":true})));
    schemas.push(("wide", record_schema(1000)));
    schemas.push((
        "unicode_text",
        json!({"type":"string", "minLength":1, "maxLength":2_000_000}),
    ));
    schemas.push((
        "reserved",
        json!({"title":"Reserved", "type":"object", "properties": {
        "dc":{"type":"string"}, "self":{"type":"integer"}, "dict":{"type":"string"},
        "str":{"type":"string"}, "typing":{"type":"string"}
    }, "required":["dc","self","dict","str","typing"]}),
    ));
    schemas.push((
        "recursive",
        json!({"title":"Node", "type":"object", "properties": {
        "value":{"type":"integer","minimum":0},
        "children":{"type":"array","items":{"$ref":"#"}}
    }, "required":["value","children"], "additionalProperties":false}),
    ));
    let sections: serde_json::Map<String, serde_json::Value> = (0..200)
        .map(|index| (format!("section{index:04}"), record_schema(12)))
        .collect();
    schemas.push((
        "many",
        json!({
            "type": "object", "required": sections.keys().collect::<Vec<_>>(),
            "properties": sections, "additionalProperties": false,
        }),
    ));
    schemas.push(("independent", json!({"title":"Independent", "type":"object", "properties":{"value":{"type":"integer"}}, "required":["value"], "additionalProperties":false,
        "$defs": {"other":{"title":"Other", "type":"object", "properties":{"name":{"type":"string"}}, "required":["name"], "additionalProperties":false}}
    })));
    schemas.push((
        "records",
        json!({"type": "array", "items": record_schema(12)}),
    ));
    for (name, schema) in schemas {
        if name == "constrained" {
            let document = SchemaDocument::from_json(&schema).expect("schema");
            let module =
                generate_dataclass_module_from_document(&document).expect("generate parts");
            fs::write(
                directory.join("constrained_public.py"),
                module.public_file("_constrained_generated"),
            )
            .unwrap();
            fs::write(
                directory.join("_constrained_generated.py"),
                module.private_file(),
            )
            .unwrap();
        }
        let source = generate_dataclass_models(&schema).expect("generate test models");
        fs::write(directory.join(format!("{name}.py")), source).expect("write models");
    }
    let output = python_env::python_command()
        .arg("tests/support/python_prepared.py")
        .arg(&directory)
        .output()
        .expect("run prepared model tests");
    print!("{}", String::from_utf8_lossy(&output.stdout));
    assert!(
        output.status.success(),
        "prepared dataclass tests failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    fs::remove_dir_all(directory).expect("remove prepared test directory");
}

fn record_schema(width: usize) -> serde_json::Value {
    let properties: serde_json::Map<String, serde_json::Value> = (0..width)
        .map(|index| {
            (
                format!("field{index:04}"),
                if index % 2 == 0 {
                    json!({"type": "string", "minLength": 1})
                } else {
                    json!({"type": "integer", "minimum": 0})
                },
            )
        })
        .collect();
    json!({"type": "object", "required": properties.keys().collect::<Vec<_>>(), "properties": properties, "additionalProperties": false})
}
