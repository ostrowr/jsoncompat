use criterion::{Criterion, black_box, criterion_group, criterion_main};
use json_schema_ast::{ResolvedSchema, build_and_resolve_schema};
use json_schema_fuzz::ValueGenerator;
use jsoncompat::is_subschema_of;
use rand::SeedableRng;
use rand::rngs::StdRng;
use serde_json::{Value, json};

fn bench_generate_value_with_raw_validation(c: &mut Criterion) {
    let schema = ResolvedSchema::from_json(&json!({
        "allOf": [
            {
                "type": "object",
                "properties": {
                    "kind": { "enum": ["a", "b", "c"] },
                    "payload": {
                        "anyOf": [
                            {
                                "type": "object",
                                "properties": {
                                    "count": { "type": "integer", "minimum": 0, "maximum": 100 },
                                    "enabled": { "type": "boolean" }
                                },
                                "required": ["count"]
                            },
                            {
                                "type": "object",
                                "properties": {
                                    "label": { "type": "string", "pattern": "^[a-z]{3}$" }
                                },
                                "required": ["label"]
                            }
                        ]
                    }
                },
                "required": ["kind", "payload"]
            },
            {
                "type": "object",
                "propertyNames": { "pattern": "^[a-z]+$" },
                "minProperties": 2
            }
        ]
    }))
    .unwrap();

    let _ = schema.root().unwrap();
    let _ = schema.is_valid(&json!({})).unwrap();

    c.bench_function("generate_value/raw_validated", |b| {
        let mut generator = ValueGenerator::new();
        let mut rng = StdRng::seed_from_u64(42);
        b.iter(|| {
            black_box(generator.generate_value(
                black_box(&schema),
                black_box(&mut rng),
                black_box(6),
            ))
            .unwrap()
        });
    });
}

fn bench_is_subschema_of_with_cached_sup_validator(c: &mut Criterion) {
    let enum_branches = (0..64)
        .map(|value| json!({ "enum": [{ "kind": "entry", "value": value }] }))
        .collect::<Vec<Value>>();

    let sub = build_and_resolve_schema(&json!({ "anyOf": enum_branches })).unwrap();
    let sup = build_and_resolve_schema(&json!({
        "type": "object",
        "properties": {
            "kind": { "const": "entry" },
            "value": { "type": "integer", "minimum": 0, "maximum": 100 }
        },
        "required": ["kind", "value"],
        "additionalProperties": false
    }))
    .unwrap();

    c.bench_function("is_subschema_of/cached_sup_validator", |b| {
        b.iter(|| black_box(is_subschema_of(black_box(&sub), black_box(&sup))));
    });
}

criterion_group!(
    benches,
    bench_generate_value_with_raw_validation,
    bench_is_subschema_of_with_cached_sup_validator
);
criterion_main!(benches);
