use jsoncompat::{Role, SchemaDocument, check_compat};
use serde_json::{Value, json};

fn membership(schema: Value, examples: &[(Value, bool)]) {
    let document = SchemaDocument::from_json(&schema).unwrap();
    for (value, expected) in examples {
        assert_eq!(
            document.is_valid(value).unwrap(),
            *expected,
            "raw {schema} accepts {value}"
        );
        assert_eq!(
            document.root().unwrap().accepts_value(value),
            *expected,
            "IR {schema} accepts {value}"
        );
    }
}

#[test]
fn unevaluated_locations_follow_successful_branches() {
    membership(
        json!({"allOf":[{"properties":{"x":true}},{"properties":{"y":true}}],"unevaluatedProperties":false}),
        &[
            (json!({"x":1,"y":2}), true),
            (json!({"z":0}), false),
            (json!(0), true),
        ],
    );
    membership(
        json!({"anyOf":[{"properties":{"x":{"type":"string"}}},{"properties":{"y":true}}],"unevaluatedProperties":false}),
        &[
            (json!({"x":"s","y":2}), true),
            (json!({"x":0,"y":2}), false),
        ],
    );
    membership(
        json!({"if":{"properties":{"x":{"type":"string"}}},"unevaluatedProperties":false}),
        &[
            (json!({"x":"s"}), true),
            (json!({"x":0}), false),
            (json!({}), true),
        ],
    );
    membership(
        json!({"allOf":[{"properties":{"x":true},"unevaluatedProperties":{"type":"string"}}],"unevaluatedProperties":false}),
        &[(json!({"x":0,"y":"s"}), true), (json!({"y":0}), false)],
    );
    membership(
        json!({"prefixItems":[true],"contains":{"type":"string"},"minContains":0,"unevaluatedItems":false}),
        &[
            (json!([0, "a", "b"]), true),
            (json!([0, 1]), false),
            (json!([]), true),
        ],
    );
}

#[test]
fn resource_identifiers_and_dynamic_anchors_resolve() {
    membership(
        json!({"$id":"https://example.com/root","$defs":{"a":{"$id":"child","$anchor":"value","type":"string"}},"$ref":"child#value"}),
        &[(json!("yes"), true), (json!(0), false)],
    );
    membership(
        json!({"$id":"https://example.com/strict","$dynamicAnchor":"node","$ref":"tree","unevaluatedProperties":false,"$defs":{"tree":{"$id":"tree","$dynamicAnchor":"node","type":"object","properties":{"children":{"type":"array","items":{"$dynamicRef":"#node"}}}}}}),
        &[
            (json!({"children":[{}]}), true),
            (json!({"children":[{"extra":0}]}), false),
        ],
    );
}

#[test]
fn rational_multiple_of_and_large_bounds_are_supported() {
    let old = SchemaDocument::from_json(&json!({"type":"number","multipleOf":0.1})).unwrap();
    let new = SchemaDocument::from_json(&json!({"type":"number","multipleOf":0.3})).unwrap();
    assert!(check_compat(&old, &new, Role::Serializer).unwrap());
    assert!(!check_compat(&old, &new, Role::Deserializer).unwrap());
    membership(
        json!({"type":"integer","minimum":18446744073709551614_u64}),
        &[
            (json!(18446744073709551613_u64), false),
            (json!(u64::MAX), true),
        ],
    );
}

#[test]
fn supplied_resources_and_format_vocabularies() {
    use jsoncompat::{CompatibilityResult, SchemaOptions, analyze_compat};
    let mut options = SchemaOptions::default();
    options.resources.insert(
        "https://example.com/value".into(),
        json!({"$anchor":"s","type":"string"}),
    );
    for invalid in [json!(null), json!(1), json!("schema"), json!([])] {
        assert!(SchemaDocument::from_json_with_options(&invalid, &options).is_err());
    }
    let document = SchemaDocument::from_json_with_options(
        &json!({"$ref":"https://example.com/value#s"}),
        &options,
    )
    .unwrap();
    assert!(document.is_valid(&json!("yes")).unwrap());
    assert!(!document.root().unwrap().accepts_value(&json!(1)));
    options.resources.insert(
        "https://example.com/meta".into(),
        json!({"$vocabulary":{
            "https://json-schema.org/draft/2020-12/vocab/core":true,
            "https://json-schema.org/draft/2020-12/vocab/format-assertion":true
        }}),
    );
    let document = SchemaDocument::from_json_with_options(
        &json!({"$schema":"https://example.com/meta", "type":"string", "format":"email"}),
        &options,
    )
    .unwrap();
    for (value, expected) in [(json!("a@example.com"), true), (json!("invalid"), false)] {
        assert_eq!(document.is_valid(&value).unwrap(), expected);
        assert_eq!(document.root().unwrap().accepts_value(&value), expected);
    }
    let ordinary = SchemaDocument::from_json(&json!({"type":"string","format":"email"})).unwrap();
    assert!(ordinary.is_valid(&json!("invalid")).unwrap());
    assert!(matches!(
        analyze_compat(&ordinary, &document, Role::Deserializer).unwrap(),
        CompatibilityResult::Incompatible { .. }
    ));
}

#[test]
fn exact_decimals_do_not_accept_nearby_nonmultiples() {
    membership(
        json!({"type":"number","multipleOf":0.1}),
        &[
            (json!(0.3), true),
            (json!(0.30000000000000004_f64), false),
            (json!(1e-17), false),
        ],
    );
}

#[test]
fn imported_new_keyword_labels_agree_with_evaluator() {
    for name in [
        "unevaluatedProperties",
        "unevaluatedItems",
        "dynamicRef",
        "anchor",
        "multipleOf",
        "optional/dependencies-compatibility",
    ] {
        let path = format!("tests/fixtures/fuzz/{name}.json");
        let contents = std::fs::read_to_string(&path).unwrap();
        let groups: Vec<Value> = serde_json::from_str(&contents).unwrap();
        for group in groups {
            let document = match SchemaDocument::from_json(&group["schema"]) {
                Ok(document) => document,
                Err(
                    json_schema_ast::AstError::UnresolvedReference { .. }
                    | json_schema_ast::AstError::UnsupportedReference { .. },
                ) => continue,
                Err(error) => panic!("{path}: {}: {error}", group["description"]),
            };
            let root = match document.root() {
                Ok(root) => root,
                Err(
                    json_schema_ast::AstError::UnresolvedReference { .. }
                    | json_schema_ast::AstError::UnsupportedReference { .. },
                ) => continue,
                Err(error) => panic!("{path}: {}: {error}", group["description"]),
            };
            for test in group["tests"].as_array().unwrap() {
                let expected = test["valid"].as_bool().unwrap();
                assert_eq!(
                    document.is_valid(&test["data"]).unwrap(),
                    expected,
                    "raw {path}: {}: {}",
                    group["description"],
                    test["description"]
                );
                assert_eq!(
                    root.accepts_value(&test["data"]),
                    expected,
                    "IR {path}: {}: {}: {}",
                    group["description"],
                    test["description"],
                    test["data"]
                );
            }
        }
    }
}

#[test]
fn complex_annotation_proofs_are_unknown_and_still_validate() {
    use jsoncompat::{CompatibilityResult, analyze_compat};
    let properties: serde_json::Map<_, _> =
        (0..9).map(|i| (format!("p{i}"), json!(true))).collect();
    let branches: Vec<_> = properties
        .keys()
        .map(|key| json!({"properties":{key:true}}))
        .collect();
    let simple = SchemaDocument::from_json(
        &json!({"type":"object", "properties":properties, "additionalProperties":false}),
    )
    .unwrap();
    let complex = SchemaDocument::from_json(
        &json!({"type":"object", "anyOf":branches, "unevaluatedProperties":false}),
    )
    .unwrap();
    assert!(matches!(
        complex.root().unwrap().kind(),
        json_schema_ast::SchemaNodeKind::Validation(_)
    ));
    assert!(
        !complex
            .root()
            .unwrap()
            .accepts_value(&json!({"extra":true}))
    );
    assert!(complex.root().unwrap().accepts_value(&json!({"p8":true})));
    assert!(matches!(
        analyze_compat(&simple, &complex, Role::Both).unwrap(),
        CompatibilityResult::Unknown { .. }
    ));
    assert!(matches!(
        analyze_compat(&complex, &complex, Role::Both).unwrap(),
        CompatibilityResult::Compatible
    ));
}

#[test]
fn format_selection_is_scoped_and_unknown_required_vocabularies_fail() {
    use jsoncompat::SchemaOptions;
    let mut options = SchemaOptions::default();
    options.resources.insert(
        "https://example.com/assert".into(),
        json!({"$vocabulary":{
            "https://json-schema.org/draft/2020-12/vocab/core":true,
            "https://json-schema.org/draft/2020-12/vocab/format-assertion":true
        }}),
    );
    let schema = SchemaDocument::from_json_with_options(&json!({
        "$schema":"https://example.com/assert", "type":"object", "properties":{
            "asserted":{"format":"email"},
            "annotated":{"$id":"https://example.com/annotation","$schema":"https://json-schema.org/draft/2020-12/schema","format":"email"}
        }
    }),&options).unwrap();
    assert!(
        schema
            .is_valid(&json!({"asserted":"a@example.com","annotated":"invalid"}))
            .unwrap()
    );
    assert!(
        !schema
            .root()
            .unwrap()
            .accepts_value(&json!({"asserted":"invalid"}))
    );
    options
        .resources
        .get_mut("https://example.com/assert")
        .unwrap()["$vocabulary"]["https://example.com/unknown"] = json!(true);
    assert!(
        SchemaDocument::from_json_with_options(
            &json!({"$schema":"https://example.com/assert"}),
            &options
        )
        .is_err()
    );
}

#[test]
fn numeric_and_residual_generation_returns_valid_values() {
    use json_schema_fuzz::{GenerationConfig, ValueGenerator};
    use rand::{SeedableRng, rngs::StdRng};
    let mut rng = StdRng::seed_from_u64(202012);
    for raw in [
        json!({"type":"number","minimum":0.2,"maximum":0.4,"multipleOf":0.1}),
        json!({"type":"integer","minimum":u64::MAX-1,"maximum":u64::MAX}),
        json!({"$id":"https://example.com/tree","$dynamicAnchor":"node","type":"object","properties":{"children":{"type":"array","items":{"$dynamicRef":"#node"}}},"unevaluatedProperties":false}),
    ] {
        let schema = SchemaDocument::from_json(&raw).unwrap();
        for _ in 0..16 {
            let value =
                ValueGenerator::generate(&schema, GenerationConfig::new(4), &mut rng).unwrap();
            assert!(schema.is_valid(&value).unwrap(), "{raw}: {value}");
            assert!(
                schema.root().unwrap().accepts_value(&value),
                "{raw}: {value}"
            );
        }
    }
}

#[test]
fn supported_keywords_remain_sound_when_composed() {
    let schemas = vec![
        json!(true),
        json!(false),
        json!({"type":"number","multipleOf":0.1}),
        json!({"type":"number","multipleOf":0.3}),
        json!({"type":"integer","multipleOf":1.5}),
        json!({"type":"number","minimum":9007199254740992_u64}),
        json!({"type":"integer","exclusiveMinimum":18446744073709551614_u64}),
        json!({"type":"object","unevaluatedProperties":false}),
        json!({"properties":{"x":true},"unevaluatedProperties":false}),
        json!({"allOf":[{"properties":{"x":true}},{"properties":{"y":{"type":"string"}}}],"unevaluatedProperties":false}),
        json!({"anyOf":[{"properties":{"x":{"type":"string"}}},{"properties":{"y":{"type":"number"}}}],"unevaluatedProperties":false}),
        json!({"oneOf":[{"required":["x"],"properties":{"x":true}},{"required":["y"],"properties":{"y":true}}],"unevaluatedProperties":false}),
        json!({"if":{"properties":{"x":{"type":"string"}},"required":["x"]},"then":{"properties":{"y":true}},"else":{"properties":{"z":true}},"unevaluatedProperties":false}),
        json!({"not":{"properties":{"x":{"type":"number"}}},"unevaluatedProperties":false}),
        json!({"dependentSchemas":{"x":{"properties":{"y":true}}},"unevaluatedProperties":false}),
        json!({"dependencies":{"x":{"properties":{"x":true,"y":true}}},"unevaluatedProperties":false}),
        json!({"allOf":[{"properties":{"x":true},"unevaluatedProperties":{"type":"string"}}],"unevaluatedProperties":false}),
        json!({"contains":{"type":"string"},"minContains":0,"unevaluatedItems":false}),
        json!({"prefixItems":[true],"contains":{"type":"string"},"minContains":0,"unevaluatedItems":{"type":"number"}}),
        json!({"allOf":[{"contains":{"type":"string"}},{"contains":{"type":"number"}}],"unevaluatedItems":false}),
        json!({"if":{"prefixItems":[{"type":"string"}]},"then":{"prefixItems":[true,true]},"unevaluatedItems":false}),
        json!({"$id":"https://example.com/a","$defs":{"b":{"$anchor":"b","type":"number","multipleOf":0.1}},"$ref":"#b"}),
        json!({"not":{"type":"number","multipleOf":0.1}}),
    ];
    let atoms = [json!(0), json!(1), json!("s"), json!(null)];
    let mut values = vec![
        json!(true),
        json!(false),
        json!(0.1),
        json!(0.3),
        json!(0.30000000000000004_f64),
        json!(9007199254740993_u64),
        json!(u64::MAX),
    ];
    values.extend(atoms.iter().cloned());
    for code in 0..125 {
        let mut n = code;
        let mut object = serde_json::Map::new();
        for key in ["x", "y", "z"] {
            let choice = n % 5;
            n /= 5;
            if choice > 0 {
                object.insert(key.into(), atoms[choice - 1].clone());
            }
        }
        values.push(Value::Object(object));
    }
    values.push(json!([]));
    for a in &atoms {
        values.push(json!([a]));
        for b in &atoms {
            values.push(json!([a, b]));
            for c in &atoms {
                values.push(json!([a, b, c]));
            }
        }
    }
    let documents: Vec<_> = schemas
        .iter()
        .map(|schema| SchemaDocument::from_json(schema).unwrap())
        .collect();
    let memberships: Vec<Vec<bool>> = documents
        .iter()
        .enumerate()
        .map(|(index, schema)| {
            values
                .iter()
                .map(|value| {
                    let accepted = schema.is_valid(value).unwrap();
                    assert_eq!(
                        schema.root().unwrap().accepts_value(value),
                        accepted,
                        "{}: {value}",
                        schemas[index]
                    );
                    accepted
                })
                .collect()
        })
        .collect();
    for (a, old) in documents.iter().enumerate() {
        for (b, new) in documents.iter().enumerate() {
            if check_compat(old, new, Role::Serializer).unwrap() {
                for (i, value) in values.iter().enumerate() {
                    assert!(
                        !memberships[b][i] || memberships[a][i],
                        "false proof: {} contains {}: counterexample {value}",
                        schemas[a],
                        schemas[b]
                    );
                }
            }
        }
    }
}

#[test]
fn content_schema_is_an_annotation_but_can_define_a_referenced_resource() {
    membership(
        json!({"type":"string","contentSchema":{"$ref":"https://example.com/not-fetched"}}),
        &[(json!("opaque"), true)],
    );
    membership(
        json!({"$id":"https://example.com/root", "contentSchema":{"$id":"embedded", "$anchor":"value", "type":"string"}, "$ref":"embedded#value"}),
        &[(json!("yes"), true), (json!(0), false)],
    );
}

#[test]
fn generation_has_a_configurable_structural_work_budget() {
    use json_schema_fuzz::{GenerateError, GenerationConfig, ValueGenerator};
    use rand::{SeedableRng, rngs::StdRng};
    use std::num::NonZeroUsize;
    let schema = SchemaDocument::from_json(
        &json!({"type":"array","minItems":1,"maxItems":1,"items":{"const":"x"}}),
    )
    .unwrap();
    let config = GenerationConfig::new(4)
        .with_max_generation_attempts(NonZeroUsize::new(1).unwrap())
        .with_max_candidate_nodes(NonZeroUsize::new(1).unwrap());
    let mut rng = StdRng::seed_from_u64(4);
    assert!(matches!(
        ValueGenerator::generate(&schema, config, &mut rng),
        Err(GenerateError::ExhaustedAttempts { .. })
    ));
    let value = ValueGenerator::generate(&schema, GenerationConfig::new(4), &mut rng).unwrap();
    assert_eq!(value, json!(["x"]));
}
