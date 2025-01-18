use anyhow::{anyhow, Result};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use url::Url;
pub mod fuzz;

/// Our internal AST for a JSON Schema after we parse references.
/// Derived PartialEq so we can do short-circuit checks (exact structural equality).
#[derive(Debug, Clone, PartialEq)]
pub enum SchemaNode {
    BoolSchema(bool),
    Any,

    // JSON primitive types
    String {
        min_length: Option<u64>,
        max_length: Option<u64>,
        pattern: Option<String>,
        enumeration: Option<Vec<Value>>,
    },
    Number {
        minimum: Option<f64>,
        maximum: Option<f64>,
        exclusive_minimum: bool,
        exclusive_maximum: bool,
        enumeration: Option<Vec<Value>>,
    },
    Integer {
        minimum: Option<i64>,
        maximum: Option<i64>,
        exclusive_minimum: bool,
        exclusive_maximum: bool,
        enumeration: Option<Vec<Value>>,
    },
    Boolean {
        enumeration: Option<Vec<Value>>,
    },
    Null {
        enumeration: Option<Vec<Value>>,
    },

    // Object
    Object {
        properties: HashMap<String, SchemaNode>,
        required: HashSet<String>,
        additional: Box<SchemaNode>,
        enumeration: Option<Vec<Value>>,
    },
    // Array
    Array {
        items: Box<SchemaNode>,
        min_items: Option<u64>,
        max_items: Option<u64>,
        enumeration: Option<Vec<Value>>,
    },

    // Definitions
    Defs(HashMap<String, SchemaNode>),

    // Applicators
    AllOf(Vec<SchemaNode>),
    AnyOf(Vec<SchemaNode>),
    OneOf(Vec<SchemaNode>),
    Not(Box<SchemaNode>),

    // Validation Keywords
    Const(Value),
    Enum(Vec<Value>),
    Type(String),
    Minimum(f64),
    Maximum(f64),
    Required(Vec<String>),
    AdditionalProperties(Box<SchemaNode>),

    // Format and Content
    Format(String),
    ContentEncoding(String),
    ContentMediaType(String),

    // Annotations
    Title(String),
    Description(String),
    Default(Value),
    Examples(Vec<Value>),
    ReadOnly(bool),
    WriteOnly(bool),

    // $ref placeholder
    Ref(String),
}

/// For convenience, define roles
#[derive(Debug, Copy, Clone)]
pub enum Role {
    Serializer,
    Deserializer,
    Both,
}

/// Build and fully resolve a schema node from raw JSON + a base URL.
pub fn build_and_resolve_schema(raw: &Value, base: &Url) -> Result<SchemaNode> {
    let mut root = build_schema_ast(raw)?;
    resolve_refs(&mut root, raw, base, &[])?;
    Ok(root)
}

/// Build the high-level AST without immediately resolving references.
pub fn build_schema_ast(raw: &Value) -> Result<SchemaNode> {
    // If the entire schema is a bool => true|false
    if let Some(b) = raw.as_bool() {
        return Ok(SchemaNode::BoolSchema(b));
    }
    if !raw.is_object() {
        // Not object/boolean => treat as Any
        return Ok(SchemaNode::Any);
    }

    let obj = raw.as_object().unwrap();

    // $ref
    if let Some(Value::String(r)) = obj.get("$ref") {
        return Ok(SchemaNode::Ref(r.to_owned()));
    }

    // enum
    if let Some(Value::Array(e)) = obj.get("enum") {
        return Ok(SchemaNode::Enum(e.clone()));
    }

    // allOf, anyOf, oneOf, not
    if let Some(Value::Array(subs)) = obj.get("allOf") {
        return Ok(SchemaNode::AllOf(
            subs.iter().map(build_schema_ast).collect::<Result<_>>()?,
        ));
    }
    if let Some(Value::Array(subs)) = obj.get("anyOf") {
        return Ok(SchemaNode::AnyOf(
            subs.iter().map(build_schema_ast).collect::<Result<_>>()?,
        ));
    }
    if let Some(Value::Array(subs)) = obj.get("oneOf") {
        return Ok(SchemaNode::OneOf(
            subs.iter().map(build_schema_ast).collect::<Result<_>>()?,
        ));
    }
    if let Some(n) = obj.get("not") {
        return Ok(SchemaNode::Not(Box::new(build_schema_ast(n)?)));
    }

    // type
    let maybe_type = obj.get("type");
    match maybe_type {
        Some(Value::String(t)) => match t.as_str() {
            "string" => parse_string_schema(obj),
            "number" => parse_number_schema(obj, false),
            "integer" => parse_number_schema(obj, true),
            "boolean" => parse_boolean_schema(obj),
            "null" => parse_null_schema(obj),
            "object" => parse_object_schema(obj),
            "array" => parse_array_schema(obj),
            _ => Ok(SchemaNode::Any),
        },
        Some(Value::Array(arr)) => {
            // treat "type": [...multiple...] as an AnyOf
            let mut variants = Vec::new();
            for t_val in arr {
                if let Some(t_str) = t_val.as_str() {
                    let mut cloned = obj.clone();
                    cloned.insert("type".into(), Value::String(t_str.into()));
                    let s = build_schema_ast(&Value::Object(cloned))?;
                    variants.push(s);
                }
            }
            if variants.len() == 1 {
                Ok(variants.remove(0))
            } else {
                Ok(SchemaNode::AnyOf(variants))
            }
        }
        _ => {
            // If no "type" but "properties" => object
            if obj.contains_key("properties") {
                parse_object_schema(obj)
            } else if obj.contains_key("items") {
                parse_array_schema(obj)
            } else {
                Ok(SchemaNode::Any)
            }
        }
    }
}

fn parse_string_schema(obj: &serde_json::Map<String, Value>) -> Result<SchemaNode> {
    let min_length = obj.get("minLength").and_then(|v| v.as_u64());
    let max_length = obj.get("maxLength").and_then(|v| v.as_u64());
    let pattern = obj
        .get("pattern")
        .and_then(|v| v.as_str())
        .map(|s| s.to_owned());
    let enumeration = obj.get("enum").and_then(|v| v.as_array()).cloned();
    Ok(SchemaNode::String {
        min_length,
        max_length,
        pattern,
        enumeration,
    })
}
fn parse_number_schema(obj: &serde_json::Map<String, Value>, integer: bool) -> Result<SchemaNode> {
    let minimum = obj.get("minimum").and_then(|v| v.as_f64());
    let maximum = obj.get("maximum").and_then(|v| v.as_f64());
    let exclusive_minimum = obj
        .get("exclusiveMinimum")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let exclusive_maximum = obj
        .get("exclusiveMaximum")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let enumeration = obj.get("enum").and_then(|v| v.as_array()).cloned();

    if integer {
        let min_i = minimum.map(|m| m as i64);
        let max_i = maximum.map(|m| m as i64);
        Ok(SchemaNode::Integer {
            minimum: min_i,
            maximum: max_i,
            exclusive_minimum,
            exclusive_maximum,
            enumeration,
        })
    } else {
        Ok(SchemaNode::Number {
            minimum,
            maximum,
            exclusive_minimum,
            exclusive_maximum,
            enumeration,
        })
    }
}
fn parse_boolean_schema(obj: &serde_json::Map<String, Value>) -> Result<SchemaNode> {
    let enumeration = obj
        .get("enum")
        .and_then(|v| v.as_array())
        .map(|v| v.clone());
    Ok(SchemaNode::Boolean { enumeration })
}
fn parse_null_schema(obj: &serde_json::Map<String, Value>) -> Result<SchemaNode> {
    let enumeration = obj
        .get("enum")
        .and_then(|v| v.as_array())
        .map(|v| v.clone());
    Ok(SchemaNode::Null { enumeration })
}

fn parse_object_schema(obj: &serde_json::Map<String, Value>) -> Result<SchemaNode> {
    use std::collections::{HashMap, HashSet};
    let mut properties = HashMap::new();
    let mut required = HashSet::new();

    if let Some(Value::Object(props)) = obj.get("properties") {
        for (k, v) in props {
            let parsed = build_schema_ast(v)?;
            properties.insert(k.clone(), parsed);
        }
    }
    if let Some(Value::Array(reqs)) = obj.get("required") {
        for r in reqs {
            if let Some(s) = r.as_str() {
                required.insert(s.to_owned());
            }
        }
    }
    let additional = match obj.get("additionalProperties") {
        None => SchemaNode::Any,
        Some(Value::Bool(b)) => SchemaNode::BoolSchema(*b),
        Some(other) => build_schema_ast(other)?,
    };
    let enumeration = obj
        .get("enum")
        .and_then(|v| v.as_array())
        .map(|v| v.clone());
    Ok(SchemaNode::Object {
        properties,
        required,
        additional: Box::new(additional),
        enumeration,
    })
}
fn parse_array_schema(obj: &serde_json::Map<String, Value>) -> Result<SchemaNode> {
    let items_node = match obj.get("items") {
        None => SchemaNode::Any,
        Some(Value::Array(arr)) => {
            // "tuple" form => approximate with allOf
            if arr.is_empty() {
                SchemaNode::Any
            } else if arr.len() == 1 {
                build_schema_ast(&arr[0])?
            } else {
                let subnodes = arr
                    .iter()
                    .map(build_schema_ast)
                    .collect::<Result<Vec<SchemaNode>>>()?;
                SchemaNode::AllOf(subnodes)
            }
        }
        Some(other) => build_schema_ast(other)?,
    };
    let min_items = obj.get("minItems").and_then(|v| v.as_u64());
    let max_items = obj.get("maxItems").and_then(|v| v.as_u64());
    let enumeration = obj
        .get("enum")
        .and_then(|v| v.as_array())
        .map(|v| v.clone());
    Ok(SchemaNode::Array {
        items: Box::new(items_node),
        min_items,
        max_items,
        enumeration,
    })
}

/// Recursively resolves `SchemaNode::Ref` by looking up fragments in `root_json`.
pub fn resolve_refs(
    node: &mut SchemaNode,
    root_json: &Value,
    base: &Url,
    visited: &[String],
) -> Result<()> {
    match node {
        SchemaNode::Ref(r) => {
            // detect cycles
            if visited.contains(r) {
                return Err(anyhow!("Circular reference detected: {}", r));
            }
            let mut new_visited = visited.to_vec();
            new_visited.push(r.clone());

            // parse as URL or relative
            let url = match Url::parse(r) {
                Ok(u) => {
                    if u.scheme().is_empty() || u.scheme() == "file" {
                        base.join(r)?
                    } else {
                        u
                    }
                }
                Err(_) => base.join(r)?,
            };
            let fragment = url.fragment().unwrap_or("");
            // For simplicity, assume same doc
            let pointed_value = if fragment.is_empty() {
                root_json
            } else {
                let pointer = if fragment.starts_with('/') {
                    fragment.to_owned()
                } else {
                    format!("/{}", fragment)
                };
                root_json
                    .pointer(&pointer)
                    .ok_or_else(|| anyhow!("Invalid fragment: {}", fragment))?
            };

            let mut resolved = build_schema_ast(pointed_value)?;
            resolve_refs(&mut resolved, root_json, base, &new_visited)?;
            *node = resolved;
        }
        SchemaNode::AllOf(subs) => {
            for s in subs {
                resolve_refs(s, root_json, base, visited)?;
            }
        }
        SchemaNode::AnyOf(subs) => {
            for s in subs {
                resolve_refs(s, root_json, base, visited)?;
            }
        }
        SchemaNode::OneOf(subs) => {
            for s in subs {
                resolve_refs(s, root_json, base, visited)?;
            }
        }
        SchemaNode::Not(sub_schema) => {
            resolve_refs(sub_schema, root_json, base, visited)?;
        }
        SchemaNode::Object {
            properties,
            additional,
            ..
        } => {
            for v in properties.values_mut() {
                resolve_refs(v, root_json, base, visited)?;
            }
            resolve_refs(additional, root_json, base, visited)?;
        }
        SchemaNode::Array { items, .. } => {
            resolve_refs(items, root_json, base, visited)?;
        }
        // others have no references
        _ => {}
    }
    Ok(())
}

/// Main "subschema" check. Is every instance of `sub` also valid under `sup`?
///
/// We skip the full code here for brevity, but it's the same from earlier examples,
/// including the short-circuit `if sub == sup { return true; }`.
pub fn is_subschema_of(sub: &SchemaNode, sup: &SchemaNode) -> bool {
    // If identical, short-circuit
    if sub == sup {
        return true;
    }

    match (sub, sup) {
        // sub = false => empty set => subset of anything
        (SchemaNode::BoolSchema(false), _) => true,

        // sup = false => sup is empty => sub must also be empty
        (_, SchemaNode::BoolSchema(false)) => matches!(sub, SchemaNode::BoolSchema(false)),

        // sup = true => universal => sub is trivially a subset
        (_, SchemaNode::BoolSchema(true)) => true,

        // sub=Any, sup=Any => equal
        (SchemaNode::Any, SchemaNode::Any) => true,
        (SchemaNode::Any, SchemaNode::BoolSchema(true)) => true,
        (SchemaNode::Any, _) => false,

        // Both enumerations => check that sub's enum values are in sup's
        (SchemaNode::Enum(e1), SchemaNode::Enum(e2)) => e1.iter().all(|v| e2.contains(v)),
        // sub=Enum => each enumerated value must be valid under sup
        (SchemaNode::Enum(e_sub), sup_other) => e_sub
            .iter()
            .all(|val| instance_is_valid_against(val, sup_other)),
        // sup=Enum => sub must not accept anything outside sup's enumerations
        (sub_other, SchemaNode::Enum(e_sup)) => match sub_other {
            SchemaNode::Enum(e_sub) => e_sub.iter().all(|v| e_sup.contains(v)),
            _ => false,
        },

        // allOf
        (SchemaNode::AllOf(subs), sup_schema) => {
            subs.iter().all(|s| is_subschema_of(s, sup_schema))
        }
        (sub_schema, SchemaNode::AllOf(sups)) => {
            sups.iter().all(|s| is_subschema_of(sub_schema, s))
        }

        // anyOf
        (SchemaNode::AnyOf(subs), sup_schema) => {
            // sub is valid if it satisfies *some* branch => to be a subset, *all* branches must be subsets
            subs.iter()
                .all(|branch| is_subschema_of(branch, sup_schema))
        }
        (sub_schema, SchemaNode::AnyOf(sups)) => {
            // sup = union => sub must be subset of at least one
            sups.iter()
                .any(|branch| is_subschema_of(sub_schema, branch))
        }

        // oneOf => treat similarly to anyOf for subset checks
        (SchemaNode::OneOf(subs), sup_schema) => subs
            .iter()
            .all(|branch| is_subschema_of(branch, sup_schema)),
        (sub_schema, SchemaNode::OneOf(sups)) => sups
            .iter()
            .any(|branch| is_subschema_of(sub_schema, branch)),

        // Not
        (SchemaNode::Not(sub_node), sup_schema) => {
            // sub = everything that doesn't match sub_node
            // partial approach
            match &**sub_node {
                SchemaNode::Any | SchemaNode::BoolSchema(true) => true, // empty => subset
                SchemaNode::BoolSchema(false) => {
                    matches!(sup_schema, SchemaNode::BoolSchema(true) | SchemaNode::Any)
                } // everything => need sup=all
                _ => false,
            }
        }
        (sub_schema, SchemaNode::Not(sup_node)) => {
            match &**sup_node {
                SchemaNode::Any | SchemaNode::BoolSchema(true) => {
                    // sup=empty => sub must be empty => sub=false
                    matches!(sub_schema, SchemaNode::BoolSchema(false))
                }
                SchemaNode::BoolSchema(false) => {
                    // sup=all => sub must be all => sub=true or Any
                    matches!(sub_schema, SchemaNode::BoolSchema(true) | SchemaNode::Any)
                }
                _ => false,
            }
        }

        // matching types => check constraints
        (SchemaNode::String { .. }, SchemaNode::String { .. })
        | (SchemaNode::Number { .. }, SchemaNode::Number { .. })
        | (SchemaNode::Integer { .. }, SchemaNode::Integer { .. })
        | (SchemaNode::Boolean { .. }, SchemaNode::Boolean { .. })
        | (SchemaNode::Null { .. }, SchemaNode::Null { .. })
        | (SchemaNode::Object { .. }, SchemaNode::Object { .. })
        | (SchemaNode::Array { .. }, SchemaNode::Array { .. }) => {
            type_constraints_subsumed(sub, sup)
        }

        // mismatch => false
        _ => false,
    }
}

/// Minimal check if an instance is valid under a schema node (used for enumerations).
fn instance_is_valid_against(val: &Value, schema: &SchemaNode) -> bool {
    match schema {
        SchemaNode::BoolSchema(false) => false,
        SchemaNode::BoolSchema(true) => true,
        SchemaNode::Any => true,

        SchemaNode::Enum(e) => e.contains(val),

        SchemaNode::AllOf(subs) => subs.iter().all(|s| instance_is_valid_against(val, s)),
        SchemaNode::AnyOf(subs) => subs.iter().any(|s| instance_is_valid_against(val, s)),
        SchemaNode::OneOf(subs) => {
            let mut count = 0;
            for s in subs {
                if instance_is_valid_against(val, s) {
                    count += 1;
                }
                if count > 1 {
                    return false;
                }
            }
            count == 1
        }
        SchemaNode::Not(sub_schema) => !instance_is_valid_against(val, sub_schema),

        SchemaNode::String { enumeration, .. } => {
            if let Some(e) = enumeration {
                if !e.contains(val) {
                    return false;
                }
            }
            val.is_string()
        }
        SchemaNode::Number { enumeration, .. } => {
            if let Some(e) = enumeration {
                if !e.contains(val) {
                    return false;
                }
            }
            val.is_number()
        }
        SchemaNode::Integer { enumeration, .. } => {
            if let Some(e) = enumeration {
                if !e.contains(val) {
                    return false;
                }
            }
            // is_i64() or is_u64() might matter, we approximate
            val.is_i64() || val.is_u64()
        }
        SchemaNode::Boolean { enumeration } => {
            if let Some(e) = enumeration {
                if !e.contains(val) {
                    return false;
                }
            }
            val.is_boolean()
        }
        SchemaNode::Null { enumeration } => {
            if let Some(e) = enumeration {
                if !e.contains(val) {
                    return false;
                }
            }
            val.is_null()
        }
        SchemaNode::Object { enumeration, .. } => {
            if let Some(e) = enumeration {
                if !e.contains(val) {
                    return false;
                }
            }
            val.is_object()
        }
        SchemaNode::Array { enumeration, .. } => {
            if let Some(e) = enumeration {
                if !e.contains(val) {
                    return false;
                }
            }
            val.is_array()
        }
        SchemaNode::Ref(_) => false, // Should be resolved

        // Handle new SchemaNode variants
        SchemaNode::Defs(_) => false,
        SchemaNode::Const(_) => false,
        SchemaNode::Type(_) => false,
        SchemaNode::Minimum(_) => false,
        SchemaNode::Maximum(_) => false,
        SchemaNode::Required(_) => false,
        SchemaNode::AdditionalProperties(_) => false,
        SchemaNode::Format(_) => false,
        SchemaNode::ContentEncoding(_) => false,
        SchemaNode::ContentMediaType(_) => false,
        SchemaNode::Title(_) => false,
        SchemaNode::Description(_) => false,
        SchemaNode::Default(_) => false,
        SchemaNode::Examples(_) => false,
        SchemaNode::ReadOnly(_) => false,
        SchemaNode::WriteOnly(_) => false,
    }
}

/// Check "sub" and "sup" for more detailed constraints. (Object, Array, or typed constraints.)
///
/// The code below includes a fix to handle `additionalProperties`.
fn type_constraints_subsumed(sub: &SchemaNode, sup: &SchemaNode) -> bool {
    use check_int_inclusion;
    use check_numeric_inclusion;
    use is_subschema_of;

    match (sub, sup) {
        // Strings
        (
            SchemaNode::String {
                min_length: smin,
                max_length: smax,
                enumeration: s_enum,
                ..
            },
            SchemaNode::String {
                min_length: pmin,
                max_length: pmax,
                enumeration: p_enum,
                ..
            },
        ) => {
            if let Some(pm) = pmin {
                if smin.unwrap_or(0) < *pm {
                    return false;
                }
            }
            if let Some(px) = pmax {
                if smax.unwrap_or(u64::MAX) > *px {
                    return false;
                }
            }
            // sub's enum must be subset of sup's enum
            if let (Some(se), Some(pe)) = (s_enum, p_enum) {
                if !se.iter().all(|v| pe.contains(v)) {
                    return false;
                }
            }
            true
        }

        // Numbers
        (
            SchemaNode::Number {
                minimum: smin,
                maximum: smax,
                exclusive_minimum: sexmin,
                exclusive_maximum: sexmax,
                enumeration: s_en,
            },
            SchemaNode::Number {
                minimum: pmin,
                maximum: pmax,
                exclusive_minimum: pexmin,
                exclusive_maximum: pexmax,
                enumeration: p_en,
            },
        ) => {
            if !check_numeric_inclusion(*smin, *sexmin, *pmin, *pexmin, true) {
                return false;
            }
            if !check_numeric_inclusion(*smax, *sexmax, *pmax, *pexmax, false) {
                return false;
            }
            if let (Some(se), Some(pe)) = (s_en, p_en) {
                if !se.iter().all(|v| pe.contains(v)) {
                    return false;
                }
            }
            true
        }

        // Integers
        (
            SchemaNode::Integer {
                minimum: smin,
                maximum: smax,
                exclusive_minimum: sexmin,
                exclusive_maximum: sexmax,
                enumeration: s_en,
            },
            SchemaNode::Integer {
                minimum: pmin,
                maximum: pmax,
                exclusive_minimum: pexmin,
                exclusive_maximum: pexmax,
                enumeration: p_en,
            },
        ) => {
            if !check_int_inclusion(*smin, *sexmin, *pmin, *pexmin, true) {
                return false;
            }
            if !check_int_inclusion(*smax, *sexmax, *pmax, *pexmax, false) {
                return false;
            }
            if let (Some(se), Some(pe)) = (s_en, p_en) {
                if !se.iter().all(|v| pe.contains(v)) {
                    return false;
                }
            }
            true
        }

        // Boolean
        (SchemaNode::Boolean { enumeration: s_e }, SchemaNode::Boolean { enumeration: p_e }) => {
            if let (Some(se), Some(pe)) = (s_e, p_e) {
                if !se.iter().all(|v| pe.contains(v)) {
                    return false;
                }
            }
            true
        }

        // Null
        (SchemaNode::Null { enumeration: s_e }, SchemaNode::Null { enumeration: p_e }) => {
            if let (Some(se), Some(pe)) = (s_e, p_e) {
                if !se.iter().all(|v| pe.contains(v)) {
                    return false;
                }
            }
            true
        }

        // Object
        (
            SchemaNode::Object {
                properties: sprops,
                required: sreq,
                additional: s_addl,
                enumeration: s_en,
            },
            SchemaNode::Object {
                properties: pprops,
                required: preq,
                additional: p_addl,
                enumeration: p_en,
            },
        ) => {
            // If both have enumerations, sub's must be subset
            if let (Some(se), Some(pe)) = (s_en, p_en) {
                if !se.iter().all(|v| pe.contains(v)) {
                    return false;
                }
            }
            // sub's known properties must be subsets of sup's
            for (k, ssub) in sprops {
                if let Some(psub) = pprops.get(k) {
                    if !is_subschema_of(ssub, psub) {
                        return false;
                    }
                } else {
                    // sup doesn't define property => must rely on sup's additionalProps
                    if !is_subschema_of(ssub, p_addl) {
                        return false;
                    }
                }
            }
            // sup's required => must also be required by sub
            for r in preq {
                if !sreq.contains(r) {
                    return false;
                }
            }
            // Additionally, sub's additionalProperties must also be subset of sup's
            // otherwise sub might accept unknown props that sup doesn't.
            if !is_subschema_of(s_addl, p_addl) {
                return false;
            }
            true
        }

        // Array
        (
            SchemaNode::Array {
                items: sitems,
                min_items: smin,
                max_items: smax,
                enumeration: s_en,
            },
            SchemaNode::Array {
                items: pitems,
                min_items: pmin,
                max_items: pmax,
                enumeration: p_en,
            },
        ) => {
            if let Some(pm) = pmin {
                if smin.unwrap_or(0) < *pm {
                    return false;
                }
            }
            if let Some(pm) = pmax {
                if smax.unwrap_or(u64::MAX) > *pm {
                    return false;
                }
            }
            if !is_subschema_of(sitems, pitems) {
                return false;
            }
            if let (Some(se), Some(pe)) = (s_en, p_en) {
                if !se.iter().all(|v| pe.contains(v)) {
                    return false;
                }
            }
            true
        }

        // Handle new SchemaNode variants
        (SchemaNode::Defs(_), _) => false,
        (SchemaNode::Const(_), _) => false,
        (SchemaNode::Type(_), _) => false,
        (SchemaNode::Minimum(_), _) => false,
        (SchemaNode::Maximum(_), _) => false,
        (SchemaNode::Required(_), _) => false,
        (SchemaNode::AdditionalProperties(_), _) => false,
        (SchemaNode::Format(_), _) => false,
        (SchemaNode::ContentEncoding(_), _) => false,
        (SchemaNode::ContentMediaType(_), _) => false,
        (SchemaNode::Title(_), _) => false,
        (SchemaNode::Description(_), _) => false,
        (SchemaNode::Default(_), _) => false,
        (SchemaNode::Examples(_), _) => false,
        (SchemaNode::ReadOnly(_), _) => false,
        (SchemaNode::WriteOnly(_), _) => false,

        _ => false,
    }
}

/// A helper for numeric ranges (floats).
pub fn check_numeric_inclusion(
    s_val: Option<f64>,
    s_excl: bool,
    p_val: Option<f64>,
    p_excl: bool,
    is_min: bool,
) -> bool {
    // p_val=None => sup has no bound => sub is narrower => ok
    if p_val.is_none() {
        return true;
    }
    let supv = p_val.unwrap();
    let subv = s_val.unwrap_or(if is_min { f64::MIN } else { f64::MAX });

    if is_min {
        // sup demands x >= supv (or x > supv)
        if p_excl {
            // sup => x > supv
            if s_excl {
                // sub => x > subv => sub is narrower if subv >= supv
                return subv >= supv;
            } else {
                // sub => x >= subv => narrower if subv > supv
                return subv > supv;
            }
        } else {
            // sup => x >= supv
            if s_excl {
                // sub => x > subv => narrower if subv >= supv
                return subv >= supv;
            } else {
                // sub => x >= subv
                return subv >= supv;
            }
        }
    } else {
        // is_max
        // sup => x <= supv (or x < supv)
        if p_excl {
            // x < supv
            if s_excl {
                // sub => x < subv => narrower if subv <= supv
                return subv <= supv;
            } else {
                // sub => x <= subv => narrower if subv < supv
                return subv < supv;
            }
        } else {
            // x <= supv
            if s_excl {
                // sub => x < subv => narrower if subv <= supv
                return subv <= supv;
            } else {
                // sub => x <= subv
                return subv <= supv;
            }
        }
    }
}

/// A helper for integer ranges.
pub fn check_int_inclusion(
    s_val: Option<i64>,
    s_excl: bool,
    p_val: Option<i64>,
    p_excl: bool,
    is_min: bool,
) -> bool {
    if p_val.is_none() {
        return true;
    }
    let supv = p_val.unwrap();
    let subv = s_val.unwrap_or(if is_min { i64::MIN } else { i64::MAX });

    if is_min {
        if p_excl {
            // sup => x > supv
            if s_excl {
                return subv >= supv;
            } else {
                return subv > supv;
            }
        } else {
            // sup => x >= supv
            if s_excl {
                return subv >= supv;
            } else {
                return subv >= supv;
            }
        }
    } else {
        // is_max
        if p_excl {
            // sup => x < supv
            if s_excl {
                return subv <= supv;
            } else {
                return subv < supv;
            }
        } else {
            // sup => x <= supv
            if s_excl {
                return subv <= supv;
            } else {
                return subv <= supv;
            }
        }
    }
}

/// Convenience function to check old vs new schema with a role:
///
/// - Role::Serializer => new ⊆ old
/// - Role::Deserializer => old ⊆ new
/// - Role::Both => (new ⊆ old) and (old ⊆ new)
///
/// Returns Ok(true) if no break, Ok(false) if there's a break.
pub fn check_compat(old: &SchemaNode, new: &SchemaNode, role: Role) -> bool {
    match role {
        Role::Serializer => {
            // Breaking if there's a JSON that new accepts but old doesn't => new ⊆ old
            is_subschema_of(new, old)
        }
        Role::Deserializer => {
            // Breaking if there's a JSON that old accepts but new doesn't => old ⊆ new
            is_subschema_of(old, new)
        }
        Role::Both => {
            // Must hold in both directions
            is_subschema_of(new, old) && is_subschema_of(old, new)
        }
    }
}
