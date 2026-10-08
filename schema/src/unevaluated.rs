//! Eliminate unevaluated constraints by partitioning successful applicator
//! branches and carrying their evaluated locations. Expansion is bounded;
//! recursive or very wide annotation dependencies retain an exact validator.
use serde_json::{Map, Value, json};
use std::collections::BTreeSet;

const MAX_CASES: usize = 128;

#[derive(Clone, Default)]
struct Locations {
    properties: BTreeSet<String>,
    patterns: BTreeSet<String>,
    all_properties: bool,
    prefix: usize,
    all_items: bool,
    contains: Vec<Value>,
}

#[derive(Clone)]
struct Case {
    guard: Value,
    locations: Locations,
}

impl Locations {
    fn merge(&mut self, other: &Self) {
        self.properties.extend(other.properties.iter().cloned());
        self.patterns.extend(other.patterns.iter().cloned());
        self.all_properties |= other.all_properties;
        self.prefix = self.prefix.max(other.prefix);
        self.all_items |= other.all_items;
        self.contains.extend(other.contains.iter().cloned());
    }
}

pub(crate) fn present(schema: &Value) -> bool {
    schema.get("unevaluatedProperties").is_some()
        || schema.get("unevaluatedItems").is_some()
        || crate::references::children(schema)
            .iter()
            .any(|(suffix, child)| suffix != "contentSchema" && present(child))
}

pub(crate) fn lower(schema: &Value) -> Option<Value> {
    transform(schema, schema, 0)
}

fn transform(schema: &Value, root: &Value, depth: usize) -> Option<Value> {
    if depth > 64 {
        return None;
    }
    let Some(object) = schema.as_object() else {
        return Some(schema.clone());
    };
    let mut result = schema.clone();
    for (suffix, child) in crate::references::children(schema) {
        if suffix == "contentSchema" {
            continue;
        }
        *result.pointer_mut(&format!("/{suffix}"))? = transform(child, root, depth + 1)?;
    }
    if !object.contains_key("unevaluatedProperties") && !object.contains_key("unevaluatedItems") {
        return Some(result);
    }
    if [
        "$ref",
        "allOf",
        "anyOf",
        "oneOf",
        "if",
        "dependentSchemas",
        "dependencies",
    ]
    .iter()
    .all(|keyword| !object.contains_key(*keyword))
        && !(object.contains_key("unevaluatedItems") && object.contains_key("contains"))
    {
        let result = result.as_object_mut()?;
        if let Some(value) = result.remove("unevaluatedProperties") {
            result.entry("additionalProperties").or_insert(value);
        }
        if let Some(value) = result.remove("unevaluatedItems") {
            result.entry("items").or_insert(value);
        }
        return Some(Value::Object(result.clone()));
    }
    let cases = cases(schema, root, &mut BTreeSet::new(), depth + 1)?;
    let mut branches = Vec::new();
    for case in cases {
        let mut base = result.as_object()?.clone();
        let properties = base.remove("unevaluatedProperties");
        let items = base.remove("unevaluatedItems");
        let mut assertions = vec![Value::Object(base), case.guard];
        if let Some(unevaluated) = properties
            && !case.locations.all_properties
        {
            let properties: Map<_, _> = case
                .locations
                .properties
                .into_iter()
                .map(|name| (name, json!(true)))
                .collect();
            let patterns: Map<_, _> = case
                .locations
                .patterns
                .into_iter()
                .map(|name| (name, json!(true)))
                .collect();
            assertions.push(json!({"properties":properties, "patternProperties":patterns, "additionalProperties":unevaluated}));
        }
        if let Some(unevaluated) = items
            && !case.locations.all_items
        {
            let mut choices = case.locations.contains;
            choices.push(unevaluated);
            let mut closure = json!({"items":disjunction(choices)});
            if case.locations.prefix > 0 {
                closure["prefixItems"] = json!(vec![true; case.locations.prefix]);
            }
            assertions.push(closure);
        }
        branches.push(conjunction(assertions));
    }
    Some(disjunction(branches))
}

fn cases(
    schema: &Value,
    root: &Value,
    active: &mut BTreeSet<String>,
    depth: usize,
) -> Option<Vec<Case>> {
    if depth > 64 {
        return None;
    }
    let Some(object) = schema.as_object() else {
        return Some(vec![Case {
            guard: schema.clone(),
            locations: Locations::default(),
        }]);
    };
    let mut locations = Locations::default();
    if let Some(properties) = object.get("properties").and_then(Value::as_object) {
        locations.properties.extend(properties.keys().cloned());
    }
    if let Some(patterns) = object.get("patternProperties").and_then(Value::as_object) {
        locations.patterns.extend(patterns.keys().cloned());
    }
    locations.all_properties = object.contains_key("additionalProperties");
    locations.prefix = object
        .get("prefixItems")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    locations.all_items = object.contains_key("items");
    if let Some(contains) = object.get("contains") {
        locations
            .contains
            .push(transform(contains, root, depth + 1)?);
    }
    let mut result = vec![Case {
        guard: json!(true),
        locations,
    }];
    if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
        if !active.insert(reference.to_owned()) {
            return None;
        }
        let target = root.pointer(reference.strip_prefix('#')?)?;
        let children = successful_cases(target, root, active, depth + 1)?;
        active.remove(reference);
        result = product(result, children)?;
    }
    for keyword in ["allOf", "anyOf", "oneOf"] {
        if let Some(branches) = object.get(keyword).and_then(Value::as_array) {
            let mut alternatives = if keyword == "oneOf" {
                Vec::new()
            } else {
                vec![Case {
                    guard: json!(true),
                    locations: Locations::default(),
                }]
            };
            for (index, branch) in branches.iter().enumerate() {
                let mut successful = successful_cases(branch, root, active, depth + 1)?;
                for case in &mut successful {
                    case.guard = conjunction(vec![
                        transform(branch, root, depth + 1)?,
                        case.guard.clone(),
                    ]);
                }
                if keyword == "oneOf" {
                    for case in &mut successful {
                        let mut guards = vec![case.guard.clone()];
                        for (other_index, other) in branches.iter().enumerate() {
                            if other_index != index {
                                guards.push(json!({"not":transform(other, root, depth + 1)?}));
                            }
                        }
                        case.guard = conjunction(guards);
                    }
                    alternatives.extend(successful);
                } else {
                    if keyword == "anyOf" {
                        successful.push(Case {
                            guard: json!({"not":transform(branch, root, depth + 1)?}),
                            locations: Locations::default(),
                        });
                    }
                    alternatives = product(alternatives, successful)?;
                }
                if alternatives.len() > MAX_CASES {
                    return None;
                }
            }
            result = product(result, alternatives)?;
        }
    }
    if let Some(condition) = object.get("if") {
        let mut yes = successful_cases(condition, root, active, depth + 1)?;
        for case in &mut yes {
            case.guard = conjunction(vec![
                transform(condition, root, depth + 1)?,
                case.guard.clone(),
            ]);
        }
        if let Some(then) = object.get("then") {
            yes = product(yes, successful_cases(then, root, active, depth + 1)?)?;
        }
        let mut no = object
            .get("else")
            .map(|otherwise| successful_cases(otherwise, root, active, depth + 1))
            .unwrap_or_else(|| {
                Some(vec![Case {
                    guard: json!(true),
                    locations: Locations::default(),
                }])
            })?;
        for case in &mut no {
            case.guard = conjunction(vec![
                json!({"not":transform(condition, root, depth + 1)?}),
                case.guard.clone(),
            ]);
        }
        yes.extend(no);
        result = product(result, yes)?;
    }
    for dependencies in ["dependentSchemas"]
        .into_iter()
        .filter_map(|keyword| object.get(keyword).and_then(Value::as_object))
    {
        for (name, dependency) in dependencies {
            if dependency.is_array() {
                continue;
            }
            let trigger = json!({"type":"object", "required":[name]});
            let mut yes = successful_cases(dependency, root, active, depth + 1)?;
            for case in &mut yes {
                case.guard = conjunction(vec![trigger.clone(), case.guard.clone()]);
            }
            yes.push(Case {
                guard: json!({"not":trigger}),
                locations: Locations::default(),
            });
            result = product(result, yes)?;
        }
    }
    // A successful nested unevaluated constraint evaluates every remaining
    // location. The current schema's own constraint must see the pre-closure
    // locations, so only callers collecting a child's annotations apply this.
    Some(result)
}

fn successful_cases(
    schema: &Value,
    root: &Value,
    active: &mut BTreeSet<String>,
    depth: usize,
) -> Option<Vec<Case>> {
    let mut cases = cases(schema, root, active, depth)?;
    for case in &mut cases {
        case.locations.all_properties |= schema.get("unevaluatedProperties").is_some();
        case.locations.all_items |= schema.get("unevaluatedItems").is_some();
    }
    Some(cases)
}

fn product(left: Vec<Case>, right: Vec<Case>) -> Option<Vec<Case>> {
    if left.len().checked_mul(right.len())? > MAX_CASES {
        return None;
    }
    Some(
        left.into_iter()
            .flat_map(|left| {
                right.iter().map(move |right| {
                    let mut locations = left.locations.clone();
                    locations.merge(&right.locations);
                    Case {
                        guard: conjunction(vec![left.guard.clone(), right.guard.clone()]),
                        locations,
                    }
                })
            })
            .collect(),
    )
}

fn conjunction(mut schemas: Vec<Value>) -> Value {
    schemas.retain(|schema| schema != &json!(true));
    match schemas.len() {
        0 => json!(true),
        1 => schemas.remove(0),
        _ => json!({"allOf":schemas}),
    }
}

fn disjunction(mut schemas: Vec<Value>) -> Value {
    match schemas.len() {
        0 => json!(false),
        1 => schemas.remove(0),
        _ => json!({"anyOf":schemas}),
    }
}
