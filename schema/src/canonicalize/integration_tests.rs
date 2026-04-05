use crate::canonicalize::{CanonicalSchema, CanonicalizeError, canonicalize_schema};
use crate::{AstError, JSONSchema, build_and_resolve_schema, compile, compile_canonical};
use rand::{Rng, RngExt, SeedableRng, rngs::StdRng};
use serde_json::{Map, Value, json};
use std::fs;
use std::path::Path;

const FUZZ_FIXTURE_ROOT: &str = "../tests/fixtures/fuzz";
const SEMANTIC_EQUIVALENCE_SAMPLES_PER_SCHEMA: usize = 64;
const MAX_SAFE_ADJACENT_INTEGER_MUTATION: i64 = 9_007_199_254_740_990;
const JSON_SCHEMA_DRAFT_2020_12: &str = "https://json-schema.org/draft/2020-12/schema";
const JSON_SCHEMA_DRAFT_2020_12_WITH_FRAGMENT: &str =
    "https://json-schema.org/draft/2020-12/schema#";

#[test]
fn canonicalize_every_fuzz_fixture_schema_is_idempotent_and_ast_equivalent()
-> Result<(), Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    collect_fixture_files(Path::new(FUZZ_FIXTURE_ROOT), &mut files)?;
    files.sort();

    for path in files {
        let bytes = fs::read(&path)?;
        let root: Value = serde_json::from_slice(&bytes)?;
        let schemas = collect_embedded_schemas(&root);
        let is_custom_fixture = is_custom_fixture_path(&path);

        for (index, schema_json) in schemas.iter().enumerate() {
            let canonical = match canonicalize_schema(schema_json) {
                Ok(canonical) => canonical,
                Err(error)
                    if !is_custom_fixture
                        && schema_declares_unsupported_schema_uri(schema_json) =>
                {
                    assert_unsupported_schema_uri_error(schema_json, &error, path.display(), index);
                    continue;
                }
                Err(error) => {
                    return Err(format!("{} schema #{index}: {error}", path.display()).into());
                }
            };
            let canonical_again = canonicalize_schema(canonical.as_value()).map_err(|error| {
                format!("{} schema #{index} recanonicalize: {error}", path.display())
            })?;
            assert_eq!(
                canonical,
                canonical_again,
                "canonicalization is not idempotent for {} schema #{}\nraw: {}\ncanonical: {}\nrecanonical: {}",
                path.display(),
                index,
                serde_json::to_string_pretty(schema_json)?,
                serde_json::to_string_pretty(canonical.as_value())?,
                serde_json::to_string_pretty(canonical_again.as_value())?,
            );

            let canonical_ast = match build_and_resolve_schema(canonical.as_value()) {
                Ok(ast) => ast,
                Err(
                    AstError::UnsupportedReference { .. } | AstError::UnresolvedReference { .. },
                ) => continue,
                Err(error) => {
                    return Err(format!(
                        "{} schema #{index} canonical AST: {error}",
                        path.display()
                    )
                    .into());
                }
            };
            let canonical_again_ast = build_and_resolve_schema(canonical_again.as_value())
                .map_err(|error| {
                    format!(
                        "{} schema #{index} recanonicalized AST: {error}",
                        path.display()
                    )
                })?;
            assert_eq!(
                canonical_ast,
                canonical_again_ast,
                "recanonicalization changed AST semantics for {} schema #{}\ncanonical: {}\nrecanonical: {}",
                path.display(),
                index,
                serde_json::to_string_pretty(canonical.as_value())?,
                serde_json::to_string_pretty(canonical_again.as_value())?,
            );
        }
    }

    Ok(())
}

#[test]
fn canonicalize_every_fuzz_fixture_schema_preserves_validation_semantics()
-> Result<(), Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    collect_fixture_files(Path::new(FUZZ_FIXTURE_ROOT), &mut files)?;
    files.sort();

    for path in files {
        let bytes = fs::read(&path)?;
        let root: Value = serde_json::from_slice(&bytes)?;
        let schemas = collect_embedded_schemas(&root);
        let is_custom_fixture = is_custom_fixture_path(&path);

        for (index, schema_json) in schemas.iter().enumerate() {
            let canonical = match canonicalize_schema(schema_json) {
                Ok(canonical) => canonical,
                Err(error)
                    if !is_custom_fixture
                        && schema_declares_unsupported_schema_uri(schema_json) =>
                {
                    assert_unsupported_schema_uri_error(schema_json, &error, path.display(), index);
                    continue;
                }
                Err(error) => {
                    return Err(format!("{} schema #{index}: {error}", path.display()).into());
                }
            };
            let raw_compiled = compile(schema_json).map_err(|error| {
                format!("{} schema #{index} raw compile: {error}", path.display())
            })?;
            let canonical_compiled = compile_canonical(&canonical).map_err(|error| {
                format!(
                    "{} schema #{index} canonical compile: {error}",
                    path.display()
                )
            })?;

            let mut stream_a_rng = StdRng::seed_from_u64(schema_seed(&path, index, 0));
            let mut stream_b_rng = StdRng::seed_from_u64(schema_seed(&path, index, 1));

            for sample_index in 0..SEMANTIC_EQUIVALENCE_SAMPLES_PER_SCHEMA {
                let from_stream_a = random_probe_candidate(&mut stream_a_rng, 6);
                assert_compiled_validators_agree(
                    &raw_compiled,
                    &canonical_compiled,
                    &path,
                    index,
                    &from_stream_a,
                    "generated from probe stream A",
                    sample_index,
                )?;
                for (mutation_index, candidate) in semantic_probe_candidates(&from_stream_a)
                    .into_iter()
                    .enumerate()
                {
                    assert_compiled_validators_agree(
                        &raw_compiled,
                        &canonical_compiled,
                        &path,
                        index,
                        &candidate,
                        &format!(
                            "mutation #{mutation_index} of value generated from probe stream A"
                        ),
                        sample_index,
                    )?;
                }

                let from_stream_b = random_probe_candidate(&mut stream_b_rng, 6);
                assert_compiled_validators_agree(
                    &raw_compiled,
                    &canonical_compiled,
                    &path,
                    index,
                    &from_stream_b,
                    "generated from probe stream B",
                    sample_index,
                )?;
                for (mutation_index, candidate) in semantic_probe_candidates(&from_stream_b)
                    .into_iter()
                    .enumerate()
                {
                    assert_compiled_validators_agree(
                        &raw_compiled,
                        &canonical_compiled,
                        &path,
                        index,
                        &candidate,
                        &format!(
                            "mutation #{mutation_index} of value generated from probe stream B"
                        ),
                        sample_index,
                    )?;
                }
            }
        }
    }

    Ok(())
}

#[test]
fn canonicalize_preserves_title_and_jsoncompat_metadata_but_strips_annotations() {
    let raw = json!({
        "type": "string",
        "title": "Display Name",
        "description": "Human-facing text",
        "default": "alice",
        "examples": ["alice", "bob"],
        "x-unknown": {"debug": true},
        "x-jsoncompat": {
            "kind": "declaration",
            "stable_id": "user-name",
            "name": "UserNameV1",
            "version": 1,
            "schema_ref": "#"
        }
    });

    let canonical = canonicalize_schema(&raw).unwrap();

    assert_eq!(
        canonical.as_value(),
        &json!({
            "minLength": 0,
            "title": "Display Name",
            "type": "string",
            "x-jsoncompat": {
                "kind": "declaration",
                "stable_id": "user-name",
                "name": "UserNameV1",
                "version": 1,
                "schema_ref": "#"
            }
        })
    );
}

#[test]
fn canonicalize_normalizes_schema_uri_and_removes_stale_keywords() {
    assert_canonicalizes_to(
        &json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema#",
            "type": "string",
            "then": { "type": "string" },
            "else": { "type": "number" },
            "minContains": 2,
            "maxContains": 4,
            "additionalItems": false
        }),
        &json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "minLength": 0,
            "type": "string"
        }),
    );
}

#[test]
fn canonicalize_preserves_identity_metadata_and_drops_defs_when_collapsing_to_false() {
    assert_canonicalizes_to(
        &json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema#",
            "$id": "https://example.com/root",
            "$anchor": "root",
            "$dynamicAnchor": "dynamic-root",
            "$defs": {
                "unused": { "type": "string" }
            },
            "x-jsoncompat": {
                "kind": "declaration",
                "stable_id": "root",
                "name": "Root",
                "version": 1,
                "schema_ref": "#"
            },
            "allOf": [false],
            "type": "string"
        }),
        &json!({
            "$anchor": "root",
            "$dynamicAnchor": "dynamic-root",
            "$id": "https://example.com/root",
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "not": true,
            "x-jsoncompat": {
                "kind": "declaration",
                "name": "Root",
                "schema_ref": "#",
                "stable_id": "root",
                "version": 1
            }
        }),
    );
}

#[test]
fn canonicalize_is_idempotent() {
    let raw = json!({
        "required": ["name", "name"],
        "properties": {
            "name": { "type": ["string"] }
        },
        "description": "ignored"
    });

    let first = canonicalize_schema(&raw).unwrap();
    let second = canonicalize_schema(first.as_value()).unwrap();

    assert_eq!(first, second);
}

#[test]
fn canonicalize_expands_implicit_object_constraints() {
    let raw = json!({
        "required": ["name"],
        "properties": {
            "name": { "const": "alice" }
        }
    });

    let canonical = canonicalize_schema(&raw).unwrap();

    assert_eq!(
        canonical.as_value(),
        &json!({
            "anyOf": [
                { "enum": [null] },
                { "enum": [false, true] },
                {
                    "minProperties": 1,
                    "properties": {
                        "name": { "enum": ["alice"] }
                    },
                    "required": ["name"],
                    "type": "object"
                },
                { "items": true, "minItems": 0, "type": "array" },
                { "minLength": 0, "type": "string" },
                { "type": "number" }
            ]
        })
    );
}

#[test]
fn canonicalize_preserves_local_ref_targets_in_object_keywords() {
    let raw = json!({
        "properties": {
            "foo": { "type": "integer" },
            "bar": { "$ref": "#/properties/foo" }
        }
    });

    let canonical = canonicalize_schema(&raw).unwrap();
    let object = canonical.as_value().as_object().unwrap();

    assert!(object.contains_key("properties"));
    assert!(!object.contains_key("anyOf"));
    assert_eq!(
        object["properties"]["bar"]["$ref"],
        json!("#/properties/foo")
    );
    assert_eq!(
        object["properties"]["foo"],
        json!({ "multipleOf": 1, "type": "integer" })
    );
}

#[test]
fn canonicalize_preserves_pruned_keyword_branches_when_local_refs_target_them() {
    let raw = json!({
        "type": "object",
        "prefixItems": [
            {
                "type": "string"
            }
        ],
        "$defs": {
            "Alias": {
                "$ref": "#/prefixItems/0"
            }
        },
        "properties": {
            "value": {
                "$ref": "#/$defs/Alias"
            }
        }
    });

    let canonical = canonicalize_schema(&raw).unwrap();
    assert_eq!(
        canonical.as_value(),
        &json!({
            "$defs": {
                "Alias": {
                    "$ref": "#/prefixItems/0"
                }
            },
            "minProperties": 0,
            "prefixItems": [
                {
                    "minLength": 0,
                    "type": "string"
                }
            ],
            "properties": {
                "value": {
                    "$ref": "#/$defs/Alias"
                }
            },
            "type": "object"
        })
    );

    build_and_resolve_schema(canonical.as_value()).unwrap();
}

#[test]
fn canonicalize_preserves_if_targets_without_then_or_else() {
    let canonical = assert_canonicalizes_to(
        &json!({
            "allOf": [
                { "$ref": "#/if" }
            ],
            "if": {
                "type": "string"
            }
        }),
        &json!({
            "allOf": [
                { "$ref": "#/if" }
            ],
            "if": {
                "minLength": 0,
                "type": "string"
            }
        }),
    );

    build_and_resolve_schema(canonical.as_value()).unwrap();
}

#[test]
fn canonicalize_preserves_not_false_targets() {
    let canonical = assert_canonicalizes_to(
        &json!({
            "if": false,
            "then": {
                "$ref": "#/not"
            },
            "not": false
        }),
        &json!({
            "if": false,
            "not": false,
            "then": {
                "$ref": "#/not"
            }
        }),
    );

    build_and_resolve_schema(canonical.as_value()).unwrap();
}

#[test]
fn canonicalize_keeps_duplicate_applicator_items_when_indexed_refs_target_them() {
    let canonical = canonicalize_schema(&json!({
        "allOf": [
            {
                "$ref": "#/x/allOf/1/properties/value"
            }
        ],
        "x": {
            "allOf": [
                {
                    "type": "object",
                    "properties": {
                        "value": { "type": "string" }
                    }
                },
                {
                    "type": "object",
                    "properties": {
                        "value": { "type": "string" }
                    }
                }
            ]
        }
    }))
    .unwrap();

    let all_of = canonical.as_value()["x"]["allOf"].as_array().unwrap();
    assert_eq!(all_of.len(), 2);
    assert_eq!(all_of[0], all_of[1]);
    build_and_resolve_schema(canonical.as_value()).unwrap();
}

#[test]
fn canonicalize_preserves_descendants_of_unsatisfiable_branches_when_locally_referenced() {
    let canonical = canonicalize_schema(&json!({
        "allOf": [
            {
                "$ref": "#/x/allOf/0/properties/value"
            }
        ],
        "x": {
            "allOf": [
                {
                    "type": "object",
                    "properties": {
                        "value": { "type": "string" }
                    },
                    "minProperties": 2,
                    "maxProperties": 1
                }
            ]
        }
    }))
    .unwrap();

    assert_eq!(
        canonical.as_value()["x"]["allOf"][0]["properties"]["value"],
        json!({
            "minLength": 0,
            "type": "string"
        })
    );
    build_and_resolve_schema(canonical.as_value()).unwrap();
}

#[test]
fn canonicalize_preserves_defs_when_false_anyof_or_oneof_is_referenced() {
    for raw in [
        json!({
            "allOf": [
                { "$ref": "#/x/$defs/A" }
            ],
            "x": {
                "anyOf": [false],
                "$defs": {
                    "A": { "type": "string" }
                }
            }
        }),
        json!({
            "allOf": [
                { "$ref": "#/x/$defs/A" }
            ],
            "x": {
                "oneOf": [false],
                "$defs": {
                    "A": { "type": "string" }
                }
            }
        }),
    ] {
        let canonical = canonicalize_schema(&raw).unwrap();
        assert_eq!(
            canonical.as_value()["x"]["$defs"]["A"],
            json!({
                "minLength": 0,
                "type": "string"
            })
        );
        assert_eq!(canonical.as_value()["x"]["not"], json!(true));
        build_and_resolve_schema(canonical.as_value()).unwrap();
    }
}

#[test]
fn canonicalize_lowers_boolean_and_null_types_to_enum() {
    assert_eq!(
        canonicalize_schema(&json!({ "type": "boolean" }))
            .unwrap()
            .into_value(),
        json!({ "enum": [false, true] })
    );
    assert_eq!(
        canonicalize_schema(&json!({ "type": "null" }))
            .unwrap()
            .into_value(),
        json!({ "enum": [null] })
    );
}

#[test]
fn canonicalize_intersects_const_with_enum() {
    assert_canonicalizes_to(
        &json!({
            "const": 1,
            "enum": [1, 2]
        }),
        &json!({
            "enum": [1]
        }),
    );

    assert_canonicalizes_to(
        &json!({
            "const": 1,
            "enum": [2]
        }),
        &json!({ "not": true }),
    );

    assert_canonicalizes_to(
        &json!({
            "const": 1.0,
            "enum": [1]
        }),
        &json!({
            "enum": [1.0]
        }),
    );
}

#[test]
fn canonicalize_accepts_float_form_integer_keyword_values() {
    assert_canonicalizes_to(
        &json!({
            "type": "string",
            "maxLength": 1.0
        }),
        &json!({
            "maxLength": 1,
            "minLength": 0,
            "type": "string"
        }),
    );
}

#[test]
fn canonicalize_normalizes_integer_bounds_and_multiple_of() {
    assert_eq!(
        canonicalize_schema(&json!({
            "type": "integer",
            "minimum": 1.2,
            "exclusiveMaximum": 5
        }))
        .unwrap()
        .into_value(),
        json!({
            "maximum": 4,
            "minimum": 2,
            "multipleOf": 1,
            "type": "integer"
        })
    );
}

#[test]
fn canonicalize_does_not_rewrite_oneof_integer_overlapping_integral_numeric_enum_to_anyof() {
    let raw = json!({
        "oneOf": [
            { "enum": [1.0] },
            { "type": "integer" }
        ]
    });

    let canonical = assert_canonicalizes_to(
        &raw,
        &json!({
            "oneOf": [
                { "enum": [1.0] },
                { "multipleOf": 1, "type": "integer" }
            ]
        }),
    );

    let raw_compiled = compile(&raw).unwrap();
    let canonical_compiled = compile_canonical(&canonical).unwrap();
    for value in [json!(1), json!(1.0), json!(2)] {
        assert_eq!(
            raw_compiled.is_valid(&value),
            canonical_compiled.is_valid(&value),
            "raw and canonical validators disagree for {value}"
        );
    }
}

#[test]
fn canonicalize_converts_integral_exclusive_integer_bounds_and_checks_equal_bound_multiple_of() {
    assert_canonicalizes_to(
        &json!({
            "type": "integer",
            "exclusiveMinimum": 1.0,
            "exclusiveMaximum": 5.0
        }),
        &json!({
            "maximum": 4,
            "minimum": 2,
            "multipleOf": 1,
            "type": "integer"
        }),
    );

    assert_canonicalizes_to(
        &json!({
            "type": "integer",
            "minimum": 3,
            "maximum": 3,
            "multipleOf": 2
        }),
        &json!({ "not": true }),
    );
}

#[test]
fn canonicalize_rejects_integer_bounds_above_i64_max() {
    let error = canonicalize_schema(&json!({
        "type": "integer",
        "minimum": 9223372036854775809_u64,
        "maximum": 9223372036854775809_u64,
        "multipleOf": 3
    }))
    .unwrap_err();

    assert!(matches!(
        error,
        CanonicalizeError::IntegerKeywordOutOfRange {
            pointer,
            ..
        } if pointer == "#/minimum" || pointer == "#/maximum"
    ));
}

#[test]
fn canonicalize_collapses_overflowed_exclusive_integer_bounds_to_unsatisfiable() {
    assert_canonicalizes_to(
        &json!({
            "type": "integer",
            "exclusiveMaximum": i64::MIN
        }),
        &json!({
            "not": true
        }),
    );

    assert_canonicalizes_to(
        &json!({
            "type": "integer",
            "exclusiveMinimum": i64::MAX
        }),
        &json!({
            "not": true
        }),
    );
}

#[test]
fn canonicalize_lowers_equal_numeric_bounds_to_enum_when_multiple_of_is_compatible() {
    assert_canonicalizes_to(
        &json!({
            "type": "integer",
            "minimum": 4,
            "maximum": 4,
            "multipleOf": 2
        }),
        &json!({
            "enum": [4]
        }),
    );

    assert_canonicalizes_to(
        &json!({
            "type": "number",
            "minimum": 1.5,
            "maximum": 1.5,
            "multipleOf": 0.5
        }),
        &json!({
            "enum": [1.5]
        }),
    );
}

#[test]
fn canonicalize_closes_dependent_required_and_synthesizes_property_schemas() {
    assert_canonicalizes_to(
        &json!({
            "type": "object",
            "required": ["foo"],
            "dependentRequired": {
                "foo": ["bar"],
                "bar": ["baz", "baz"]
            }
        }),
        &json!({
            "dependentRequired": {},
            "minProperties": 3,
            "properties": {
                "bar": true,
                "baz": true,
                "foo": true
            },
            "required": ["bar", "baz", "foo"],
            "type": "object"
        }),
    );
}

#[test]
fn canonicalize_collapses_single_type_arrays_and_prunes_irrelevant_keywords() {
    assert_canonicalizes_to(
        &json!({
            "type": ["string"],
            "minimum": 5,
            "minLength": 2,
            "properties": {
                "foo": true
            }
        }),
        &json!({
            "minLength": 2,
            "type": "string"
        }),
    );
}

#[test]
fn canonicalize_expands_type_unions_and_routes_keywords_to_matching_branches() {
    assert_canonicalizes_to(
        &json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema#",
            "$id": "https://example.com/schema",
            "$anchor": "root",
            "type": ["string", "number", "integer"],
            "minimum": 5,
            "minLength": 2,
            "properties": {
                "foo": true
            }
        }),
        &json!({
            "$anchor": "root",
            "$id": "https://example.com/schema",
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "anyOf": [
                { "minimum": 5, "multipleOf": 1, "type": "integer" },
                { "minimum": 5, "type": "number" },
                { "minLength": 2, "type": "string" }
            ]
        }),
    );
}

#[test]
fn canonicalize_wraps_existing_anyof_when_expanding_type_unions() {
    assert_canonicalizes_to(
        &json!({
            "type": ["string", "number"],
            "anyOf": [
                { "minLength": 1 },
                { "minimum": 0 }
            ]
        }),
        &json!({
            "allOf": [
                {
                    "anyOf": [
                        {
                            "anyOf": [
                                { "enum": [null] },
                                { "enum": [false, true] },
                                { "minProperties": 0, "properties": {}, "type": "object" },
                                { "items": true, "minItems": 0, "type": "array" },
                                { "minLength": 1, "type": "string" },
                                { "type": "number" }
                            ]
                        },
                        {
                            "anyOf": [
                                { "enum": [null] },
                                { "enum": [false, true] },
                                { "minProperties": 0, "properties": {}, "type": "object" },
                                { "items": true, "minItems": 0, "type": "array" },
                                { "minLength": 0, "type": "string" },
                                { "minimum": 0, "type": "number" }
                            ]
                        }
                    ]
                },
                {
                    "anyOf": [
                        { "type": "number" },
                        { "minLength": 0, "type": "string" }
                    ]
                }
            ]
        }),
    );
}

#[test]
fn canonicalize_keeps_type_unions_in_place_when_local_refs_are_present() {
    assert_canonicalizes_to(
        &json!({
            "type": ["object", "array"],
            "$ref": "#/$defs/foo",
            "$defs": {
                "foo": { "type": "string" }
            }
        }),
        &json!({
            "$defs": {
                "foo": { "minLength": 0, "type": "string" }
            },
            "$ref": "#/$defs/foo",
            "type": ["array", "object"]
        }),
    );
}

#[test]
fn canonicalize_sorts_and_deduplicates_required_and_enum_values() {
    assert_eq!(
        canonicalize_schema(&json!({
            "type": "object",
            "required": ["z", "a", "z"],
            "enum": [{ "z": 1, "a": 2 }, { "a": 2, "z": 1 }],
            "properties": {
                "z": true,
                "a": true
            }
        }))
        .unwrap()
        .into_value(),
        json!({
            "enum": [{ "a": 2, "z": 1 }],
            "properties": {
                "a": true,
                "z": true
            },
            "required": ["a", "z"]
        })
    );
}

#[test]
fn canonicalize_simplifies_unsatisfiable_array_contains_bounds() {
    assert_eq!(
        canonicalize_schema(&json!({
            "type": "array",
            "contains": { "const": 1 },
            "minContains": 3,
            "maxContains": 1
        }))
        .unwrap()
        .into_value(),
        json!({ "not": true })
    );
}

#[test]
fn canonicalize_expands_implicit_type_union_for_nested_schemas_too() {
    assert_eq!(
        canonicalize_schema(&json!({
            "properties": {
                "child": { "properties": { "name": { "type": "string" } } }
            }
        }))
        .unwrap()
        .into_value(),
        json!({
            "anyOf": [
                { "enum": [null] },
                { "enum": [false, true] },
                {
                    "minProperties": 0,
                    "properties": {
                        "child": {
                            "anyOf": [
                                { "enum": [null] },
                                { "enum": [false, true] },
                                {
                                    "minProperties": 0,
                                    "properties": {
                                        "name": { "minLength": 0, "type": "string" }
                                    },
                                    "type": "object"
                                },
                                { "items": true, "minItems": 0, "type": "array" },
                                { "minLength": 0, "type": "string" },
                                { "type": "number" }
                            ]
                        }
                    },
                    "type": "object"
                },
                { "items": true, "minItems": 0, "type": "array" },
                { "minLength": 0, "type": "string" },
                { "type": "number" }
            ]
        })
    );
}

#[test]
fn canonicalize_routes_nested_string_constraints_to_only_the_string_branch() {
    assert_eq!(
        canonicalize_schema(&json!({
            "type": "object",
            "properties": {
                "child": { "minLength": 2 }
            }
        }))
        .unwrap()
        .into_value(),
        json!({
            "minProperties": 0,
            "properties": {
                "child": {
                    "anyOf": [
                        { "enum": [null] },
                        { "enum": [false, true] },
                        { "minProperties": 0, "properties": {}, "type": "object" },
                        { "items": true, "minItems": 0, "type": "array" },
                        { "minLength": 2, "type": "string" },
                        { "type": "number" }
                    ]
                }
            },
            "type": "object"
        })
    );
}

#[test]
fn canonicalize_routes_nested_format_constraints_to_only_the_string_branch() {
    assert_eq!(
        canonicalize_schema(&json!({
            "type": "object",
            "properties": {
                "email": { "format": "email" }
            }
        }))
        .unwrap()
        .into_value(),
        json!({
            "minProperties": 0,
            "properties": {
                "email": {
                    "anyOf": [
                        { "enum": [null] },
                        { "enum": [false, true] },
                        { "minProperties": 0, "properties": {}, "type": "object" },
                        { "items": true, "minItems": 0, "type": "array" },
                        { "format": "email", "minLength": 0, "type": "string" },
                        { "type": "number" }
                    ]
                }
            },
            "type": "object"
        })
    );
}

#[test]
fn canonicalize_preserves_unknown_keywords_if_they_are_local_ref_targets() {
    let canonical = canonicalize_schema(&json!({
        "unknown-keyword": {
            "type": "integer",
            "description": "ignored"
        },
        "properties": {
            "value": { "$ref": "#/unknown-keyword" }
        }
    }))
    .unwrap();

    assert_eq!(
        canonical.as_value(),
        &json!({
            "properties": {
                "value": { "$ref": "#/unknown-keyword" }
            },
            "unknown-keyword": {
                "multipleOf": 1,
                "type": "integer"
            }
        })
    );
}

#[test]
fn canonicalize_only_preserves_unknown_keywords_that_are_ref_targets_or_target_ancestors() {
    let canonical = canonicalize_schema(&json!({
        "x-preserved": {
            "nested": {
                "type": "string",
                "description": "ignored"
            }
        },
        "x-removed": {
            "type": "integer"
        },
        "properties": {
            "value": { "$ref": "#/x-preserved/nested" }
        }
    }))
    .unwrap();

    assert_eq!(
        canonical.as_value(),
        &json!({
            "properties": {
                "value": { "$ref": "#/x-preserved/nested" }
            },
            "x-preserved": {
                "nested": {
                    "minLength": 0,
                    "type": "string"
                }
            }
        })
    );
}

#[test]
fn canonicalize_simplifies_allof_and_anyof_boolean_branches() {
    assert_eq!(
        canonicalize_schema(&json!({
            "allOf": [true, { "type": "string" }, { "type": "string" }]
        }))
        .unwrap()
        .into_value(),
        json!({
            "allOf": [
                { "minLength": 0, "type": "string" }
            ]
        })
    );

    assert_eq!(
        canonicalize_schema(&json!({
            "anyOf": [false, { "type": "string" }, { "type": "string" }]
        }))
        .unwrap()
        .into_value(),
        json!({
            "anyOf": [
                { "minLength": 0, "type": "string" }
            ]
        })
    );

    assert_eq!(
        canonicalize_schema(&json!({ "allOf": [false, { "type": "string" }] }))
            .unwrap()
            .into_value(),
        json!({ "not": true })
    );
}

#[test]
fn canonicalize_accepts_tuple_items_arrays() {
    assert_eq!(
        canonicalize_schema(&json!({
            "type": "array",
            "items": [
                { "type": "integer" },
                { "const": "done" }
            ]
        }))
        .unwrap()
        .into_value(),
        json!({
            "items": [
                { "multipleOf": 1, "type": "integer" },
                { "enum": ["done"] }
            ],
            "minItems": 0,
            "type": "array"
        })
    );
}

#[test]
fn canonicalize_clamps_array_and_object_size_defaults() {
    assert_canonicalizes_to(
        &json!({
            "type": "array",
            "contains": { "type": "string" },
            "minContains": 2,
            "maxContains": 4,
            "maxItems": 2
        }),
        &json!({
            "contains": {
                "minLength": 0,
                "type": "string"
            },
            "items": true,
            "maxContains": 2,
            "maxItems": 2,
            "minContains": 2,
            "minItems": 2,
            "type": "array"
        }),
    );

    assert_canonicalizes_to(
        &json!({
            "type": "object",
            "minProperties": 1,
            "required": ["foo", "bar"]
        }),
        &json!({
            "minProperties": 2,
            "properties": {
                "bar": true,
                "foo": true
            },
            "required": ["bar", "foo"],
            "type": "object"
        }),
    );
}

#[test]
fn canonicalize_keeps_output_order_deterministic_for_permuted_equivalent_schemas() {
    let left = canonicalize_schema(&json!({
        "required": ["z", "a", "z"],
        "properties": {
            "z": { "type": "integer" },
            "a": { "type": "string" }
        },
        "enum": [
            { "z": 1, "a": 2 },
            { "a": 2, "z": 1 }
        ],
        "type": ["object"]
    }))
    .unwrap();

    let right = canonicalize_schema(&json!({
        "type": ["object"],
        "enum": [
            { "a": 2, "z": 1 },
            { "z": 1, "a": 2 }
        ],
        "properties": {
            "a": { "type": "string" },
            "z": { "type": "integer" }
        },
        "required": ["a", "z"]
    }))
    .unwrap();

    assert_eq!(left, right);
    assert_eq!(
        serde_json::to_string(left.as_value()).unwrap(),
        r#"{"enum":[{"a":2,"z":1}],"properties":{"a":{"minLength":0,"type":"string"},"z":{"multipleOf":1,"type":"integer"}},"required":["a","z"]}"#
    );
}

#[test]
fn canonicalize_rejects_invalid_schema_shapes_with_precise_pointers() {
    assert_canonicalizes_to(
        &json!({ "type": "string" }),
        &json!({ "minLength": 0, "type": "string" }),
    );
    assert_canonicalize_error(
        &json!({
            "$schema": "https://json-schema.org/draft-07/schema#",
            "type": "string"
        }),
        "unsupported $schema URI at '#/$schema': expected 'https://json-schema.org/draft/2020-12/schema', got 'https://json-schema.org/draft-07/schema#'",
    );
    assert_canonicalize_error(
        &json!({ "$schema": 7 }),
        "keyword '$schema' at '#/$schema' must be a URI string, got number",
    );
    assert_canonicalize_error(
        &json!("not a schema"),
        "schema node at '#' must be an object or boolean schema, got string",
    );
    assert_canonicalize_error(
        &json!({
            "properties": {
                "foo": 1
            }
        }),
        "schema node at '#/properties/foo' must be an object or boolean schema, got number",
    );
    assert_canonicalize_error(
        &json!({ "required": [1] }),
        "entry 0 of 'required' at '#/required' must be a string, got number",
    );
    assert_canonicalize_error(
        &json!({
            "dependentRequired": {
                "foo": [1]
            }
        }),
        "entry 0 of 'dependentRequired' at '#/dependentRequired/foo' must be a string, got number",
    );
    assert_canonicalize_error(
        &json!({ "type": ["string", 1] }),
        "entry 1 of 'type' at '#/type' must be a string, got number",
    );
    assert_canonicalize_error(
        &json!({ "anyOf": {} }),
        "keyword 'anyOf' at '#/anyOf' must be an array, got object",
    );
    assert_canonicalize_error(
        &json!({ "maxLength": "x" }),
        "keyword 'maxLength' at '#/maxLength' must be a non-negative integer, got string",
    );
    assert_canonicalize_error(
        &json!({ "minimum": "x" }),
        "keyword 'minimum' at '#/minimum' must be a finite number, got string",
    );
    assert_canonicalize_error(
        &json!({ "minItems": "x" }),
        "keyword 'minItems' at '#/minItems' must be a non-negative integer, got string",
    );
    assert_canonicalize_error(
        &json!({ "multipleOf": 0 }),
        "keyword 'multipleOf' at '#/multipleOf' must be a positive number, got number",
    );
}

fn collect_fixture_files(
    dir: &Path,
    files: &mut Vec<std::path::PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_fixture_files(&path, files)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("json") {
            files.push(path);
        }
    }
    Ok(())
}

fn is_custom_fixture_path(path: &Path) -> bool {
    path.to_string_lossy()
        .replace('\\', "/")
        .contains("/custom/")
}

fn collect_embedded_schemas(root: &Value) -> Vec<Value> {
    match root {
        Value::Array(items) => items
            .iter()
            .filter_map(|item| item.get("schema").cloned())
            .collect(),
        schema => vec![schema.clone()],
    }
}

fn schema_declares_unsupported_schema_uri(schema: &Value) -> bool {
    match schema {
        Value::Object(object) => {
            if let Some(schema_uri) = object.get("$schema")
                && !matches!(
                    schema_uri.as_str(),
                    Some(JSON_SCHEMA_DRAFT_2020_12 | JSON_SCHEMA_DRAFT_2020_12_WITH_FRAGMENT)
                )
            {
                return true;
            }
            object.values().any(schema_declares_unsupported_schema_uri)
        }
        Value::Array(items) => items.iter().any(schema_declares_unsupported_schema_uri),
        _ => false,
    }
}

fn assert_unsupported_schema_uri_error(
    schema_json: &Value,
    error: &CanonicalizeError,
    path: impl std::fmt::Display,
    index: usize,
) {
    let expected_uri = schema_json
        .as_object()
        .and_then(|object| object.get("$schema"))
        .and_then(Value::as_str)
        .expect("unsupported-$schema fixtures should declare a root $schema URI");
    assert!(
        matches!(
            error,
            CanonicalizeError::UnsupportedSchemaDialect {
                pointer,
                expected_uri: JSON_SCHEMA_DRAFT_2020_12,
                actual_uri,
            } if pointer == "#/$schema" && actual_uri == expected_uri
        ),
        "unexpected unsupported-$schema error for {path} schema #{index}: {error}"
    );
}

fn assert_canonicalizes_to(raw: &Value, expected: &Value) -> CanonicalSchema {
    let canonical = canonicalize_schema(raw).unwrap();
    assert_eq!(canonical.as_value(), expected);
    assert_eq!(
        canonicalize_schema(canonical.as_value()).unwrap(),
        canonical,
        "expected test fixture is not itself canonical"
    );
    canonical
}

fn assert_canonicalize_error(raw: &Value, expected: &str) {
    let error = canonicalize_schema(raw).unwrap_err().to_string();
    assert_eq!(error, expected);
}

fn schema_seed(path: &Path, schema_index: usize, stream: u64) -> u64 {
    path.to_string_lossy()
        .bytes()
        .fold(0x5EED_5EED_u64 ^ stream, |state, byte| {
            state.wrapping_mul(1099511628211) ^ u64::from(byte)
        })
        ^ (schema_index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

fn random_probe_candidate(rng: &mut impl Rng, depth: u8) -> Value {
    if depth == 0 {
        return match rng.random_range(0..=4) {
            0 => Value::Null,
            1 => Value::Bool(rng.random_bool(0.5)),
            2 => json!(rng.random_range(-3..=3)),
            3 => json!(rng.random_range(-3.0..=3.0)),
            _ => Value::String("jsoncompat".to_owned()),
        };
    }

    match rng.random_range(0..=6) {
        0 => Value::Null,
        1 => Value::Bool(rng.random_bool(0.5)),
        2 => json!(rng.random_range(-10..=10)),
        3 => json!(rng.random_range(-10.0..=10.0)),
        4 => {
            let length = rng.random_range(0..=8);
            let value: String = (0..length)
                .map(|_| rng.sample(rand::distr::Alphanumeric) as char)
                .collect();
            Value::String(value)
        }
        5 => {
            let mut items = Vec::new();
            let length = rng.random_range(0..=4);
            for _ in 0..length {
                items.push(random_probe_candidate(rng, depth - 1));
            }
            Value::Array(items)
        }
        _ => {
            let mut object = Map::new();
            let length = rng.random_range(0..=4);
            for index in 0..length {
                object.insert(format!("k{index}"), random_probe_candidate(rng, depth - 1));
            }
            Value::Object(object)
        }
    }
}

fn assert_compiled_validators_agree(
    raw_compiled: &JSONSchema,
    canonical_compiled: &JSONSchema,
    path: &Path,
    schema_index: usize,
    candidate: &Value,
    source: &str,
    sample_index: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let raw_valid = raw_compiled.is_valid(candidate);
    let canonical_valid = canonical_compiled.is_valid(candidate);
    assert_eq!(
        raw_valid,
        canonical_valid,
        "canonicalized and raw schemas disagree for {} schema #{} sample #{} ({source})\ncandidate: {}",
        path.display(),
        schema_index,
        sample_index,
        serde_json::to_string_pretty(candidate)?,
    );
    Ok(())
}

fn semantic_probe_candidates(candidate: &Value) -> Vec<Value> {
    let mut probes = vec![
        Value::Null,
        Value::Bool(false),
        Value::Bool(true),
        json!(0),
        json!(1),
        json!(""),
        json!("jsoncompat"),
        Value::Array(Vec::new()),
        Value::Object(serde_json::Map::new()),
    ];

    match candidate {
        Value::Null => {
            probes.push(json!("null"));
        }
        Value::Bool(value) => {
            probes.push(Value::Bool(!value));
        }
        Value::Number(number) => {
            if let Some(integer) = number.as_i64()
                && integer.unsigned_abs() <= MAX_SAFE_ADJACENT_INTEGER_MUTATION as u64
            {
                if let Some(next) = integer.checked_add(1) {
                    probes.push(json!(next));
                }
                if let Some(previous) = integer.checked_sub(1) {
                    probes.push(json!(previous));
                }
            }
            if let Some(float) = number.as_f64()
                && float.is_finite()
                && float.abs() <= MAX_SAFE_ADJACENT_INTEGER_MUTATION as f64
            {
                probes.push(json!(float + 0.5));
            }
            probes.push(Value::String(number.to_string()));
        }
        Value::String(text) => {
            probes.push(json!(format!("{text}x")));
            probes.push(json!(
                text.chars()
                    .take(text.chars().count().saturating_sub(1))
                    .collect::<String>()
            ));
            probes.push(json!(text.len()));
        }
        Value::Array(items) => {
            let mut shorter = items.clone();
            shorter.pop();
            probes.push(Value::Array(shorter));

            let mut longer = items.clone();
            longer.push(Value::Null);
            probes.push(Value::Array(longer));

            let mut object = serde_json::Map::new();
            object.insert("items".to_owned(), Value::Array(items.clone()));
            probes.push(Value::Object(object));
        }
        Value::Object(object) => {
            if let Some(first_key) = object.keys().next() {
                let mut missing_key = object.clone();
                missing_key.remove(first_key);
                probes.push(Value::Object(missing_key));
            }

            let mut with_extra_key = object.clone();
            with_extra_key.insert("__jsoncompat_probe__".to_owned(), Value::Null);
            probes.push(Value::Object(with_extra_key));

            probes.push(Value::Array(object.values().cloned().collect()));
        }
    }

    probes
}
