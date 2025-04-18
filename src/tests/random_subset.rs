use json_schema_backcompat::{build_and_resolve_schema, check_compat, Role};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use serde_json::json;
use url::Url;

/// Generate simple numeric range schemas and assert subset logic is sound.
#[test]
fn random_numeric_range_subset() {
    let mut rng = StdRng::seed_from_u64(7);
    for _ in 0..100 {
        let a_min = rng.gen_range(-50..50);
        let a_max = rng.gen_range(a_min..=a_min + 50);
        let b_min = rng.gen_range(-50..50);
        let b_max = rng.gen_range(b_min..=b_min + 50);

        let schema_a = json!({"type":"integer","minimum":a_min,"maximum":a_max});
        let schema_b = json!({"type":"integer","minimum":b_min,"maximum":b_max});

        let base = Url::parse("file:///r.json").unwrap();
        let a_ast = build_and_resolve_schema(&schema_a, &base).unwrap();
        let b_ast = build_and_resolve_schema(&schema_b, &base).unwrap();

        let subset = a_min >= b_min && a_max <= b_max;
        assert_eq!(check_compat(&b_ast, &a_ast, Role::Serializer), subset);
    }
}
