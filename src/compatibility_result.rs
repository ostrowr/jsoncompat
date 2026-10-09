//! Three-way public verdicts. Only a validated counterexample proves a failure;
//! an unsuccessful structural proof remains unknown.
use crate::{CompatibilityError, Role, SchemaDocument, check_compat};
use json_schema_ast::{NodeId, SchemaNode, SchemaNodeKind};
use json_schema_fuzz::{GenerationConfig, ValueGenerator};
use rand::{SeedableRng, rngs::StdRng};
use serde_json::{Value, json};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CompatibilityResult {
    Compatible,
    Incompatible {
        direction: Role,
        counterexample: Value,
    },
    Unknown {
        reason: String,
    },
}

/// Prove compatibility or find a concrete counterexample. Failure to do either
/// is explicitly unknown, including unsupported structural expansions.
pub fn analyze_compat(
    old: &SchemaDocument,
    new: &SchemaDocument,
    role: Role,
) -> Result<CompatibilityResult, CompatibilityError> {
    if check_compat(old, new, role)? {
        return Ok(CompatibilityResult::Compatible);
    }
    let mut rng = StdRng::seed_from_u64(0x434f4d504154);
    for direction in [Role::Serializer, Role::Deserializer] {
        if role != Role::Both && direction != role {
            continue;
        }
        let (source, target) = if direction == Role::Serializer {
            (new, old)
        } else {
            (old, new)
        };
        let mut candidates = vec![
            json!(null),
            json!(false),
            json!(true),
            json!(0),
            json!(1),
            json!(-1),
            json!(0.5),
            json!(""),
            json!("a"),
            json!([]),
            json!({}),
        ];
        literals(source.source_schema_json(), &mut candidates);
        literals(target.source_schema_json(), &mut candidates);
        // Check cheap witnesses first, and generate only until one disproves
        // inclusion. Common failures need no randomized generation at all.
        let generated = (0..16).filter_map(|_| {
            ValueGenerator::generate(source, GenerationConfig::new(4), &mut rng).ok()
        });
        for value in candidates.into_iter().chain(generated) {
            if source.is_valid(&value)?
                && !target.is_valid(&value)?
                && (direction == Role::Serializer
                    || emitted(source.root()?, &value, &mut HashSet::new()))
            {
                return Ok(CompatibilityResult::Incompatible {
                    direction,
                    counterexample: value,
                });
            }
        }
    }
    Ok(CompatibilityResult::Unknown {
        reason:
            "the structural prover could not establish inclusion, and no counterexample was found"
                .into(),
    })
}

fn literals(schema: &Value, candidates: &mut Vec<Value>) {
    let Some(object) = schema.as_object() else {
        return;
    };
    if let Some(value) = object.get("const") {
        candidates.push(value.clone());
    }
    if let Some(values) = object.get("enum").and_then(Value::as_array) {
        candidates.extend(values.iter().cloned());
    }
    for key in [
        "minimum",
        "maximum",
        "exclusiveMinimum",
        "exclusiveMaximum",
        "multipleOf",
    ] {
        if let Some(value) = object.get(key) {
            candidates.push(value.clone());
        }
    }
    // Deliberately traverse only schema positions, never JSON payloads in const.
    for key in json_schema_ast::SINGLE_SCHEMA_CHILD_KEYWORDS {
        if let Some(child) = object.get(key) {
            literals(child, candidates);
        }
    }
    for key in json_schema_ast::SCHEMA_MAP_CHILD_KEYWORDS {
        if let Some(children) = object.get(key).and_then(Value::as_object) {
            for child in children.values() {
                literals(child, candidates);
            }
        }
    }
    for key in json_schema_ast::SCHEMA_ARRAY_CHILD_KEYWORDS {
        if let Some(children) = object.get(key).and_then(Value::as_array) {
            for child in children {
                literals(child, candidates);
            }
        }
    }
}

fn emitted(schema: &SchemaNode, value: &Value, active: &mut HashSet<(NodeId, usize)>) -> bool {
    let key = (schema.id(), std::ptr::from_ref(value) as usize);
    if !active.insert(key) {
        return false;
    }
    let result = match (schema.kind(), value) {
        (
            SchemaNodeKind::Object {
                properties,
                required,
                dependent_required,
                ..
            },
            Value::Object(values),
        ) => values.iter().all(|(name, value)| {
            if let Some(child) = properties.get(name) {
                emitted(child, value, active)
            } else {
                required.contains(name)
                    || dependent_required.iter().any(|(trigger, names)| {
                        values.contains_key(trigger) && names.contains(name)
                    })
            }
        }),
        (
            SchemaNodeKind::Array {
                prefix_items,
                items,
                contains: None,
                ..
            },
            Value::Array(values),
        ) => values
            .iter()
            .enumerate()
            .all(|(index, value)| emitted(prefix_items.get(index).unwrap_or(items), value, active)),
        (SchemaNodeKind::AnyOf(branches), _) => branches
            .iter()
            .any(|branch| branch.accepts_value(value) && emitted(branch, value, active)),
        // Other applicators use full validation semantics in the prover too.
        _ => true,
    };
    active.remove(&key);
    result
}
