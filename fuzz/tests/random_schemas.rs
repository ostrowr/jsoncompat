use json_schema_fuzz::{generate_value, random_schema};
use json_schema_draft2020::{build_and_resolve_schema, compile};
use rand::{rngs::StdRng, SeedableRng};
use url::Url;

#[test]
fn random_schema_and_values_are_valid() {
    let mut rng = StdRng::seed_from_u64(1234);
    for _ in 0..50 {
        let raw = random_schema(&mut rng, 3);
        let base = Url::parse("file:///fuzzschema.json").unwrap();
        let ast = build_and_resolve_schema(&raw, &base).unwrap();
        let compiled = compile(&ast.to_json()).unwrap();

        for _ in 0..100 {
            let v = generate_value(&ast, &mut rng, 4);
            assert!(compiled.is_valid(&v), "generated value not valid: {v:?} for schema {raw}");
        }
    }
}
