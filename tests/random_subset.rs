use json_schema_backcompat::{build_and_resolve_schema, check_compat, Role};
use rand::{Rng, SeedableRng};
use serde_json::json;

#[test]
fn random_numeric_subset() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(123);
    for _ in 0..50 {
        let min1 = rng.gen_range(-100..0);
        let max1 = rng.gen_range(0..100);
        let min2 = rng.gen_range(-100..0);
        let max2 = rng.gen_range(0..100);

        let s1 = json!({"type":"integer","minimum":min1,"maximum":max1});
        let s2 = json!({"type":"integer","minimum":min2,"maximum":max2});

        let ast1 = build_and_resolve_schema(&s1).unwrap();
        let ast2 = build_and_resolve_schema(&s2).unwrap();

        let subset = min2 >= min1 && max2 <= max1;
        assert_eq!(check_compat(&ast1, &ast2, Role::Serializer), subset);
    }
}
