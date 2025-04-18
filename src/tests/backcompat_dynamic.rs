//! Programmatically generate 50+ schema‑change pairs that exercise the
//! back‑compat checker along two independent dimensions (string length and
//! numeric ranges).  Expectations are derived analytically: tightening a
//! constraint is compatible for the *serializer* (new ⊆ old) and breaking for
//! the *deserializer*.

use json_schema_backcompat::{build_and_resolve_schema, check_compat, Role};
use json_schema_draft2020::{compile, SchemaNode};
use json_schema_fuzz::generate_value;
use rand::{rngs::StdRng, SeedableRng};
use serde_json::json;
use url::Url;

fn to_ast(v: &serde_json::Value) -> SchemaNode {
    build_and_resolve_schema(v, &Url::parse("file:///gen.json").unwrap()).unwrap()
}

#[test]
fn programmatic_tightening_cases() {
    let mut rng = StdRng::seed_from_u64(55);

    // 25 string‑length tightenings + 25 numeric‑minimum tightenings = 50
    for i in 0..25 {
        // -------- STRING minLength tightening --------
        let old = json!({"type":"string","minLength": i});
        let new = json!({"type":"string","minLength": i+5});

        run_case(&old, &new, true, false, &mut rng);

        // -------- NUMBER minimum tightening ----------
        let old_num = json!({"type":"number","minimum": i as i64});
        let new_num = json!({"type":"number","minimum": (i+10) as i64});

        run_case(&old_num, &new_num, true, false, &mut rng);
    }
}

fn run_case(old_raw: &serde_json::Value,
            new_raw: &serde_json::Value,
            expect_ser: bool,
            expect_de: bool,
            rng: &mut StdRng) {
    let old_ast = to_ast(old_raw);
    let new_ast = to_ast(new_raw);

    let ser = check_compat(&old_ast, &new_ast, Role::Serializer);
    let de  = check_compat(&old_ast, &new_ast, Role::Deserializer);
    assert_eq!(ser, expect_ser, "serializer expectation failed for {old_raw} -> {new_raw}");
    assert_eq!(de,  expect_de,  "deserializer expectation failed");

    // Sample‑based confirmation (10 each direction for speed)
    let comp_old = compile(&old_ast.to_json()).unwrap();
    let comp_new = compile(&new_ast.to_json()).unwrap();

    for _ in 0..10 {
        let v_new = generate_value(&new_ast, rng, 4);
        if expect_ser { assert!(comp_old.is_valid(&v_new)); }
        else if !comp_old.is_valid(&v_new) { break; }

        let v_old = generate_value(&old_ast, rng, 4);
        if expect_de { assert!(comp_new.is_valid(&v_old)); }
        else if !comp_new.is_valid(&v_old) { break; }
    }
}
