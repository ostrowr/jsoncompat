use jsoncompat::{Role, SchemaDocument, check_compat};
use serde_json::json;

#[test]
fn numeric_extremes_remain_sound() {
    let witnesses = [
        json!(0),
        json!(1),
        json!(-1),
        json!(1.5),
        json!(9007199254740993_u64),
        json!(u64::MAX),
        json!(1e30),
        json!(1e100),
        json!(-1e100),
        json!(1e-100),
    ];
    let mut raws = vec![
        json!(true),
        json!(false),
        json!({"type":"integer"}),
        json!({"type":"number"}),
        json!({"type":"integer","minimum":0}),
        json!({"type":"integer","maximum":0}),
    ];
    for w in &witnesses {
        raws.push(json!({"const":w}));
        raws.push(json!({"type":"integer","enum":[w]}));
        raws.push(json!({"type":"number","enum":[w]}));
    }
    let schemas: Vec<_> = raws
        .iter()
        .map(|raw| SchemaDocument::from_json(raw).unwrap())
        .collect();
    for (s, source) in schemas.iter().enumerate() {
        for (t, target) in schemas.iter().enumerate() {
            if check_compat(target, source, Role::Serializer).unwrap() {
                for w in &witnesses {
                    assert!(
                        !source.is_valid(w).unwrap() || target.is_valid(w).unwrap(),
                        "{} <= {} rejects {w}",
                        raws[s],
                        raws[t]
                    );
                }
            }
        }
    }
}

#[test]
fn object_boolean_combinations_preserve_declared_witnesses() {
    let guards = [
        json!({"type":"object"}),
        json!({"type":"object","properties":{"x":{"type":"string"}}}),
        json!({"type":"object","properties":{"y":{"type":"string"}}}),
        json!({"type":"object","required":["x"]}),
        json!({"type":"object","required":["y"]}),
        json!({"type":"object","properties":{"x":false}}),
        json!({"type":"object","additionalProperties":false}),
    ];
    let base =
        json!({"type":"object","properties":{"x":true,"y":true},"additionalProperties":false});
    let mut raws = vec![base.clone()];
    for a in &guards {
        for form in [a.clone(), json!({"not":a})] {
            raws.push(json!({"allOf":[base,form]}));
        }
    }
    let atomic_count = raws.len();
    for a in &guards {
        for b in &guards {
            for form in [
                json!({"allOf":[a,b]}),
                json!({"anyOf":[a,b]}),
                json!({"oneOf":[a,b]}),
                json!({"if":a,"then":b,"else":false}),
                json!({"if":a,"then":false,"else":b}),
            ] {
                raws.push(json!({"allOf":[base,form]}));
            }
        }
    }
    let mut witnesses = vec![json!({})];
    for x in [json!(null), json!("s"), json!(0)] {
        witnesses.push(json!({"x":x}));
        witnesses.push(json!({"y":x}));
        for y in [json!(null), json!("s"), json!(0)] {
            witnesses.push(json!({"x":x,"y":y}));
        }
    }
    let schemas: Vec<_> = raws
        .iter()
        .map(|raw| SchemaDocument::from_json(raw).unwrap())
        .collect();
    let masks: Vec<Vec<_>> = schemas
        .iter()
        .map(|schema| {
            witnesses
                .iter()
                .map(|w| schema.is_valid(w).unwrap())
                .collect()
        })
        .collect();
    let mut failures = Vec::new();
    for (s, source) in schemas.iter().enumerate() {
        for (t, target) in schemas.iter().enumerate() {
            // Compare each composition with every atomic predicate in both
            // directions; avoid a quadratic product of compound expressions.
            if s >= atomic_count && t >= atomic_count {
                continue;
            }
            let Some(w) = witnesses
                .iter()
                .enumerate()
                .find_map(|(w, value)| (masks[s][w] && !masks[t][w]).then_some(value))
            else {
                continue;
            };
            for role in [Role::Serializer, Role::Deserializer] {
                let claimed = if role == Role::Serializer {
                    check_compat(target, source, role)
                } else {
                    check_compat(source, target, role)
                }
                .unwrap();
                if claimed {
                    failures.push(format!("{role:?}: {} <= {} rejects {w}", raws[s], raws[t]));
                    if failures.len() == 15 {
                        panic!("{}", failures.join("\n"));
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
