use anyhow::{anyhow, Result};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// An internal Abstract Syntax Tree (AST) representing a fully‑resolved JSON
/// Schema draft‑2020‑12 document.  The node types are deliberately *very*
/// close to the JSON Schema specification so that higher‑level crates (e.g.
/// the back‑compat checker or fuzz generator) can reason about schemas
/// without constantly reparsing raw JSON values.
/// TODO: need roundtrip tests
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
        multiple_of: Option<f64>,
        enumeration: Option<Vec<Value>>,
    },
    Integer {
        minimum: Option<i64>,
        maximum: Option<i64>,
        exclusive_minimum: bool,
        exclusive_maximum: bool,
        multiple_of: Option<f64>,
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

        // Validation keywords for objects
        min_properties: Option<usize>,
        max_properties: Option<usize>,
        dependent_required: std::collections::HashMap<String, Vec<String>>,

        enumeration: Option<Vec<Value>>,
    },
    // Array
    Array {
        items: Box<SchemaNode>,
        min_items: Option<u64>,
        max_items: Option<u64>,
        contains: Option<Box<SchemaNode>>,
        enumeration: Option<Vec<Value>>,
    },

    // Definitions
    Defs(HashMap<String, SchemaNode>),

    // Applicators
    AllOf(Vec<SchemaNode>),
    AnyOf(Vec<SchemaNode>),
    OneOf(Vec<SchemaNode>),
    Not(Box<SchemaNode>),
    IfThenElse {
        if_schema: Box<SchemaNode>,
        then_schema: Option<Box<SchemaNode>>,
        else_schema: Option<Box<SchemaNode>>,
    },

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

impl SchemaNode {
    /// Convert the AST node back into a *minimal* JSON representation.  This
    /// is **lossy** for complex scenarios but is sufficient for the validator
    /// tests and fuzz harness (which only relies on the subset of keywords we
    /// explicitly generate).
    pub fn to_json(&self) -> Value {
        match self {
            SchemaNode::BoolSchema(b) => Value::Bool(*b),
            SchemaNode::Any => Value::Object(Default::default()),

            SchemaNode::Enum(values) => {
                let mut obj = serde_json::Map::new();
                obj.insert("enum".into(), Value::Array(values.clone()));
                Value::Object(obj)
            }

            SchemaNode::String {
                min_length,
                max_length,
                pattern,
                enumeration,
            } => {
                let mut obj = serde_json::Map::new();
                obj.insert("type".into(), Value::String("string".into()));
                if let Some(m) = min_length {
                    obj.insert("minLength".into(), Value::Number((*m).into()));
                }
                if let Some(m) = max_length {
                    obj.insert("maxLength".into(), Value::Number((*m).into()));
                }
                if let Some(p) = pattern {
                    obj.insert("pattern".into(), Value::String(p.clone()));
                }
                if let Some(e) = enumeration {
                    obj.insert("enum".into(), Value::Array(e.clone()));
                }
                Value::Object(obj)
            }

            SchemaNode::Number {
                minimum,
                maximum,
                exclusive_minimum,
                exclusive_maximum,
                multiple_of,
                enumeration,
            } => {
                let mut obj = serde_json::Map::new();
                obj.insert("type".into(), Value::String("number".into()));
                if let Some(m) = minimum {
                    obj.insert(
                        "minimum".into(),
                        Value::Number(serde_json::Number::from_f64(*m).unwrap()),
                    );
                }
                if let Some(m) = maximum {
                    obj.insert(
                        "maximum".into(),
                        Value::Number(serde_json::Number::from_f64(*m).unwrap()),
                    );
                }
                if *exclusive_minimum {
                    if let Some(m) = minimum {
                        obj.insert(
                            "exclusiveMinimum".into(),
                            Value::Number(serde_json::Number::from_f64(*m).unwrap()),
                        );
                    }
                }
                if *exclusive_maximum {
                    if let Some(m) = maximum {
                        obj.insert(
                            "exclusiveMaximum".into(),
                            Value::Number(serde_json::Number::from_f64(*m).unwrap()),
                        );
                    }
                }
                if let Some(mo) = multiple_of {
                    obj.insert(
                        "multipleOf".into(),
                        Value::Number(serde_json::Number::from_f64(*mo).unwrap()),
                    );
                }
                if let Some(e) = enumeration {
                    obj.insert("enum".into(), Value::Array(e.clone()));
                }
                Value::Object(obj)
            }

            SchemaNode::Integer {
                minimum,
                maximum,
                exclusive_minimum,
                exclusive_maximum,
                multiple_of,
                enumeration,
            } => {
                let mut obj = serde_json::Map::new();
                obj.insert("type".into(), Value::String("integer".into()));
                if let Some(m) = minimum {
                    obj.insert("minimum".into(), Value::Number((*m).into()));
                }
                if let Some(m) = maximum {
                    obj.insert("maximum".into(), Value::Number((*m).into()));
                }
                if *exclusive_minimum {
                    if let Some(m) = minimum {
                        obj.insert("exclusiveMinimum".into(), Value::Number((*m).into()));
                    }
                }
                if *exclusive_maximum {
                    if let Some(m) = maximum {
                        obj.insert("exclusiveMaximum".into(), Value::Number((*m).into()));
                    }
                }
                if let Some(e) = enumeration {
                    obj.insert("enum".into(), Value::Array(e.clone()));
                }
                if let Some(mo) = multiple_of {
                    obj.insert(
                        "multipleOf".into(),
                        Value::Number(serde_json::Number::from_f64(*mo).unwrap()),
                    );
                }
                Value::Object(obj)
            }

            SchemaNode::Boolean { enumeration } => {
                let mut obj = serde_json::Map::new();
                obj.insert("type".into(), Value::String("boolean".into()));
                if let Some(e) = enumeration {
                    obj.insert("enum".into(), Value::Array(e.clone()));
                }
                Value::Object(obj)
            }

            SchemaNode::Null { enumeration } => {
                let mut obj = serde_json::Map::new();
                obj.insert("type".into(), Value::String("null".into()));
                if let Some(e) = enumeration {
                    obj.insert("enum".into(), Value::Array(e.clone()));
                }
                Value::Object(obj)
            }

            // Composite/applier keywords ----------------------------------
            SchemaNode::AllOf(subs) => {
                let arr = subs.iter().map(|s| s.to_json()).collect();
                let mut obj = serde_json::Map::new();
                obj.insert("allOf".into(), Value::Array(arr));
                Value::Object(obj)
            }
            SchemaNode::AnyOf(subs) => {
                let arr = subs.iter().map(|s| s.to_json()).collect();
                let mut obj = serde_json::Map::new();
                obj.insert("anyOf".into(), Value::Array(arr));
                Value::Object(obj)
            }
            SchemaNode::OneOf(subs) => {
                let arr = subs.iter().map(|s| s.to_json()).collect();
                let mut obj = serde_json::Map::new();
                obj.insert("oneOf".into(), Value::Array(arr));
                Value::Object(obj)
            }
            SchemaNode::Not(sub) => {
                let mut obj = serde_json::Map::new();
                obj.insert("not".into(), sub.to_json());
                Value::Object(obj)
            }
            SchemaNode::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => {
                let mut obj = serde_json::Map::new();
                obj.insert("if".into(), if_schema.to_json());
                if let Some(t) = then_schema {
                    obj.insert("then".into(), t.to_json());
                }
                if let Some(e) = else_schema {
                    obj.insert("else".into(), e.to_json());
                }
                Value::Object(obj)
            }

            // Array -------------------------------------------------------
            SchemaNode::Array {
                items,
                min_items,
                max_items,
                contains,
                enumeration,
            } => {
                let mut obj = serde_json::Map::new();
                obj.insert("type".into(), Value::String("array".into()));
                // `items` – only include if not SchemaNode::Any to mimic
                // original minimal style yet remain loss‑less.
                if !matches!(**items, SchemaNode::Any) {
                    obj.insert("items".into(), items.to_json());
                }
                if let Some(mi) = min_items {
                    obj.insert("minItems".into(), Value::Number((*mi).into()));
                }
                if let Some(ma) = max_items {
                    obj.insert("maxItems".into(), Value::Number((*ma).into()));
                }
                if let Some(c) = contains {
                    obj.insert("contains".into(), c.to_json());
                }
                if let Some(e) = enumeration {
                    obj.insert("enum".into(), Value::Array(e.clone()));
                }
                Value::Object(obj)
            }

            // Object ------------------------------------------------------
            SchemaNode::Object {
                properties,
                required,
                additional,
                min_properties,
                max_properties,
                dependent_required,
                enumeration,
            } => {
                let mut obj = serde_json::Map::new();
                obj.insert("type".into(), Value::String("object".into()));

                if !properties.is_empty() {
                    let mut props_map = serde_json::Map::new();
                    for (k, v) in properties {
                        props_map.insert(k.clone(), v.to_json());
                    }
                    obj.insert("properties".into(), Value::Object(props_map));
                }

                if !required.is_empty() {
                    // Sort to ensure deterministic output (helps with tests)
                    let mut sorted: Vec<_> = required.iter().cloned().collect();
                    sorted.sort();
                    obj.insert(
                        "required".into(),
                        Value::Array(sorted.into_iter().map(Value::String).collect()),
                    );
                }

                // additionalProperties
                if !matches!(**additional, SchemaNode::Any) {
                    match &**additional {
                        SchemaNode::BoolSchema(b) => {
                            obj.insert("additionalProperties".into(), Value::Bool(*b));
                        }
                        other => {
                            obj.insert("additionalProperties".into(), other.to_json());
                        }
                    }
                }

                if let Some(mp) = min_properties {
                    obj.insert("minProperties".into(), Value::Number((*mp).into()));
                }
                if let Some(mp) = max_properties {
                    obj.insert("maxProperties".into(), Value::Number((*mp).into()));
                }

                if !dependent_required.is_empty() {
                    let mut dr_map = serde_json::Map::new();
                    for (k, v) in dependent_required {
                        dr_map.insert(
                            k.clone(),
                            Value::Array(v.iter().cloned().map(Value::String).collect()),
                        );
                    }
                    obj.insert("dependentRequired".into(), Value::Object(dr_map));
                }

                if let Some(e) = enumeration {
                    obj.insert("enum".into(), Value::Array(e.clone()));
                }

                Value::Object(obj)
            }

            // Definitions (rarely used in resolved AST) -------------------
            SchemaNode::Defs(map) => {
                let mut defs_obj = serde_json::Map::new();
                for (k, v) in map {
                    defs_obj.insert(k.clone(), v.to_json());
                }
                let mut obj = serde_json::Map::new();
                obj.insert("$defs".into(), Value::Object(defs_obj));
                Value::Object(obj)
            }

            // Simple keyword wrappers -------------------------------------
            SchemaNode::Const(v) => {
                let mut obj = serde_json::Map::new();
                obj.insert("const".into(), v.clone());
                Value::Object(obj)
            }
            SchemaNode::Type(t) => {
                let mut obj = serde_json::Map::new();
                obj.insert("type".into(), Value::String(t.clone()));
                Value::Object(obj)
            }
            SchemaNode::Minimum(m) => {
                let mut obj = serde_json::Map::new();
                obj.insert(
                    "minimum".into(),
                    Value::Number(serde_json::Number::from_f64(*m).unwrap()),
                );
                Value::Object(obj)
            }
            SchemaNode::Maximum(m) => {
                let mut obj = serde_json::Map::new();
                obj.insert(
                    "maximum".into(),
                    Value::Number(serde_json::Number::from_f64(*m).unwrap()),
                );
                Value::Object(obj)
            }
            SchemaNode::Required(reqs) => {
                let mut sorted = reqs.clone();
                sorted.sort();
                let mut obj = serde_json::Map::new();
                obj.insert(
                    "required".into(),
                    Value::Array(sorted.into_iter().map(Value::String).collect()),
                );
                Value::Object(obj)
            }
            SchemaNode::AdditionalProperties(schema) => {
                let mut obj = serde_json::Map::new();
                obj.insert("additionalProperties".into(), schema.to_json());
                Value::Object(obj)
            }

            // Format & content --------------------------------------------
            SchemaNode::Format(f) => {
                let mut obj = serde_json::Map::new();
                obj.insert("format".into(), Value::String(f.clone()));
                Value::Object(obj)
            }
            SchemaNode::ContentEncoding(c) => {
                let mut obj = serde_json::Map::new();
                obj.insert("contentEncoding".into(), Value::String(c.clone()));
                Value::Object(obj)
            }
            SchemaNode::ContentMediaType(c) => {
                let mut obj = serde_json::Map::new();
                obj.insert("contentMediaType".into(), Value::String(c.clone()));
                Value::Object(obj)
            }

            // Annotations --------------------------------------------------
            SchemaNode::Title(t) => {
                let mut obj = serde_json::Map::new();
                obj.insert("title".into(), Value::String(t.clone()));
                Value::Object(obj)
            }
            SchemaNode::Description(d) => {
                let mut obj = serde_json::Map::new();
                obj.insert("description".into(), Value::String(d.clone()));
                Value::Object(obj)
            }
            SchemaNode::Default(def) => {
                let mut obj = serde_json::Map::new();
                obj.insert("default".into(), def.clone());
                Value::Object(obj)
            }
            SchemaNode::Examples(ex) => {
                let mut obj = serde_json::Map::new();
                obj.insert("examples".into(), Value::Array(ex.clone()));
                Value::Object(obj)
            }
            SchemaNode::ReadOnly(b) => {
                let mut obj = serde_json::Map::new();
                obj.insert("readOnly".into(), Value::Bool(*b));
                Value::Object(obj)
            }
            SchemaNode::WriteOnly(b) => {
                let mut obj = serde_json::Map::new();
                obj.insert("writeOnly".into(), Value::Bool(*b));
                Value::Object(obj)
            }

            // $ref ---------------------------------------------------------
            SchemaNode::Ref(r) => {
                let mut obj = serde_json::Map::new();
                obj.insert("$ref".into(), Value::String(r.clone()));
                Value::Object(obj)
            }
        }
    }
}

/// Build and fully resolve a schema node from raw JSON + a base URL.
pub fn build_and_resolve_schema(raw: &Value) -> Result<SchemaNode> {
    // For local‑only references we don’t need a real base URI.
    let mut root = build_schema_ast(raw)?;
    resolve_refs(&mut root, raw, &[])?;
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

    // const
    if let Some(c) = obj.get("const") {
        return Ok(SchemaNode::Const(c.clone()));
    }

    // if / then / else ---------------------------------------------------
    if obj.contains_key("if") || obj.contains_key("then") || obj.contains_key("else") {
        if let Some(cond) = obj.get("if") {
            let if_schema = Box::new(build_schema_ast(cond)?);
            let then_schema = match obj.get("then") {
                Some(v) => Some(Box::new(build_schema_ast(v)?)),
                None => None,
            };
            let else_schema = match obj.get("else") {
                Some(v) => Some(Box::new(build_schema_ast(v)?)),
                None => None,
            };
            let mut base = obj.clone();
            base.remove("if");
            base.remove("then");
            base.remove("else");
            const META_KEYS: [&str; 4] = ["$schema", "$id", "$comment", "$defs"];
            for key in META_KEYS {
                base.remove(key);
            }
            let cond_node = SchemaNode::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            };
            if !base.is_empty() {
                let subs = vec![build_schema_ast(&Value::Object(base))?, cond_node];
                return Ok(SchemaNode::AllOf(subs));
            } else {
                return Ok(cond_node);
            }
        } else {
            // then/else without if are ignored
            let mut base = obj.clone();
            base.remove("then");
            base.remove("else");
            return build_schema_ast(&Value::Object(base));
        }
    }

    // ------------------------------------------------------------------
    // allOf, anyOf, oneOf, not
    // ------------------------------------------------------------------
    // According to the specification, keywords *in addition* to `allOf`
    // combine through intersection.  Therefore, if the current object has
    // both `allOf` and other validation keywords, we treat the *base* object
    // (with `allOf` removed) as an implicit subschema that participates in
    // the conjunction.
    if let Some(Value::Array(subs)) = obj.get("allOf") {
        let mut list = Vec::new();

        // Extract the base‑schema (everything except the `allOf` keyword) –
        // but only if there are additional constraints present.
        if obj.len() > 1 {
            let mut base = obj.clone();
            base.remove("allOf");

            // Strip recognised meta‑schema keywords that do not impose
            // validation constraints – most notably `$schema` but also
            // `$id`, `$comment`, etc.  If, after removal, no keywords
            // remain, there is no point in adding an empty subschema.
            const META_KEYS: [&str; 4] = ["$schema", "$id", "$comment", "$defs"];
            for key in META_KEYS {
                base.remove(key);
            }

            if !base.is_empty() {
                list.push(build_schema_ast(&Value::Object(base))?);
            }
        }

        // Parse each subschema from the `allOf` array.
        for s in subs {
            list.push(build_schema_ast(s)?);
        }

        return Ok(SchemaNode::AllOf(list));
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
            if obj.contains_key("properties")
                || obj.contains_key("minProperties")
                || obj.contains_key("maxProperties")
                || obj.contains_key("required")
            {
                parse_object_schema(obj)
            } else if obj.contains_key("items") {
                parse_array_schema(obj)
            } else if obj.contains_key("minLength")
                || obj.contains_key("maxLength")
                || obj.contains_key("pattern")
            {
                // String‑related keywords without explicit "type"
                parse_string_schema(obj)
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
    // Start with the basic inclusive bounds.
    let mut minimum = obj.get("minimum").and_then(|v| v.as_f64());
    let mut maximum = obj.get("maximum").and_then(|v| v.as_f64());

    // Exclusive minimum (numeric only in 2020‑12) -------------------------
    let exclusive_minimum = if let Some(serde_json::Value::Number(n)) = obj.get("exclusiveMinimum")
    {
        minimum = n.as_f64();
        true
    } else {
        false
    };

    // Exclusive maximum (numeric only in 2020‑12) -------------------------
    let exclusive_maximum = if let Some(serde_json::Value::Number(n)) = obj.get("exclusiveMaximum")
    {
        maximum = n.as_f64();
        true
    } else {
        false
    };
    let enumeration = obj.get("enum").and_then(|v| v.as_array()).cloned();

    // multipleOf ---------------------------------------------------------
    let multiple_of = obj
        .get("multipleOf")
        .and_then(|v| v.as_f64())
        .filter(|m| *m > 0.0);

    if integer {
        let min_i = minimum.map(|m| m as i64);
        let max_i = maximum.map(|m| m as i64);
        Ok(SchemaNode::Integer {
            minimum: min_i,
            maximum: max_i,
            exclusive_minimum,
            exclusive_maximum,
            multiple_of,
            enumeration,
        })
    } else {
        Ok(SchemaNode::Number {
            minimum,
            maximum,
            exclusive_minimum,
            exclusive_maximum,
            multiple_of,
            enumeration,
        })
    }
}

fn parse_boolean_schema(obj: &serde_json::Map<String, Value>) -> Result<SchemaNode> {
    let enumeration = obj.get("enum").and_then(|v| v.as_array()).cloned();
    Ok(SchemaNode::Boolean { enumeration })
}

fn parse_null_schema(obj: &serde_json::Map<String, Value>) -> Result<SchemaNode> {
    let enumeration = obj.get("enum").and_then(|v| v.as_array()).cloned();
    Ok(SchemaNode::Null { enumeration })
}

fn parse_object_schema(obj: &serde_json::Map<String, Value>) -> Result<SchemaNode> {
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
                // If this required field is not in properties, add it with SchemaNode::Any
                if !properties.contains_key(s) {
                    properties.insert(s.to_owned(), SchemaNode::Any);
                }
            }
        }
    }
    let additional = match obj.get("additionalProperties") {
        None => SchemaNode::Any,
        Some(Value::Bool(b)) => SchemaNode::BoolSchema(*b),
        Some(other) => build_schema_ast(other)?,
    };
    let enumeration = obj.get("enum").and_then(|v| v.as_array()).cloned();

    let dependent_required = obj
        .get("dependentRequired")
        .and_then(|v| v.as_object())
        .map(|m| {
            m.iter()
                .filter_map(|(k, val)| {
                    val.as_array().map(|arr| {
                        (
                            k.clone(),
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_owned()))
                                .collect::<Vec<_>>(),
                        )
                    })
                })
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();

    let min_properties = obj
        .get("minProperties")
        .and_then(|v| v.as_u64().and_then(|n| usize::try_from(n).ok()));
    let max_properties = obj
        .get("maxProperties")
        .and_then(|v| v.as_u64().and_then(|n| usize::try_from(n).ok()));

    Ok(SchemaNode::Object {
        properties,
        required,
        additional: Box::new(additional),
        min_properties,
        max_properties,
        dependent_required,
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
    let enumeration = obj.get("enum").and_then(|v| v.as_array()).cloned();

    // contains -----------------------------------------------------------
    let contains_node = match obj.get("contains") {
        None => None,
        Some(v) => Some(Box::new(build_schema_ast(v)?)),
    };

    Ok(SchemaNode::Array {
        items: Box::new(items_node),
        min_items,
        max_items,
        contains: contains_node,
        enumeration,
    })
}

/// Recursively resolves `SchemaNode::Ref` by looking up fragments in `root_json`.
pub fn resolve_refs(node: &mut SchemaNode, root_json: &Value, visited: &[String]) -> Result<()> {
    match node {
        SchemaNode::Ref(r) => {
            // detect cycles
            if visited.contains(r) {
                return Err(anyhow!("Circular reference detected: {}", r));
            }

            // For now, handle only local fragment refs (starting with "#/")
            if let Some(stripped) = r.strip_prefix("#/") {
                // Split JSON Pointer *after* the leading "#/" into its path
                // components and **unescape** each token according to
                // RFC 6901: first percent‑decode (the pointer may be embedded
                // in a URI fragment) and then replace the JSON Pointer escape
                // sequences `~1` → `/` and `~0` → `~`.

                fn decode_pointer_token(token: &str) -> String {
                    // 1. Percent‑decode anything that the URI fragment may
                    //    have escaped (e.g. `%25` for `%`).
                    let mut decoded = percent_encoding::percent_decode_str(token)
                        .decode_utf8_lossy()
                        .into_owned();

                    // 2. Replace JSON Pointer escape sequences.  The order is
                    //    significant: we must replace `~1` **before** `~0` so
                    //    that a sequence like `~01` is interpreted correctly
                    //    (`~0` followed by `1`).  See RFC 6901 § 4.
                    decoded = decoded.replace("~1", "/");
                    decoded.replace("~0", "~")
                }

                let parts: Vec<String> = stripped.split('/').map(decode_pointer_token).collect();
                let mut current = root_json;
                for p in parts.iter() {
                    if let Some(next) = current.get(p.as_str()) {
                        current = next;
                    } else {
                        return Err(anyhow!("Unresolved reference: {}", r));
                    }
                }
                let mut resolved = build_schema_ast(current)?;
                // Recursively resolve inside the resolved node as well
                resolve_refs(
                    &mut resolved,
                    root_json,
                    &[visited, std::slice::from_ref(r)].concat(),
                )?;
                *node = resolved;
            } else {
                // For the purposes of fuzz‑generation we ignore external or
                // unsupported `$ref`s and replace them with the permissive
                // `true` schema so that validation still passes.
                *node = SchemaNode::BoolSchema(true);
            }
        }
        SchemaNode::AllOf(subs) | SchemaNode::AnyOf(subs) | SchemaNode::OneOf(subs) => {
            for s in subs {
                resolve_refs(s, root_json, visited)?;
            }
        }
        SchemaNode::IfThenElse {
            if_schema,
            then_schema,
            else_schema,
        } => {
            resolve_refs(if_schema, root_json, visited)?;
            if let Some(t) = then_schema {
                resolve_refs(t, root_json, visited)?;
            }
            if let Some(e) = else_schema {
                resolve_refs(e, root_json, visited)?;
            }
        }
        SchemaNode::Not(sub_schema) => {
            resolve_refs(sub_schema, root_json, visited)?;
        }
        SchemaNode::Object {
            properties,
            additional,
            ..
        } => {
            for v in properties.values_mut() {
                resolve_refs(v, root_json, visited)?;
            }
            resolve_refs(additional, root_json, visited)?;
        }
        SchemaNode::Array { items, .. } => {
            resolve_refs(items, root_json, visited)?;
        }
        // primitives / annotations – no nested schemas
        _ => {}
    }
    Ok(())
}

/// Minimal check if an *instance* `val` is valid against `schema`.
///
/// This helper purposefully supports only the keyword subset that the fuzz
/// generator and back‑compat checker rely on.  It is **not** a full JSON
/// Schema validator – for that, use the `compile()` + `is_valid()` helpers
/// exposed by the parent crate which wrap the proven `jsonschema` crate.
pub fn instance_is_valid_against(val: &Value, schema: &SchemaNode) -> bool {
    match schema {
        SchemaNode::BoolSchema(false) => false,
        SchemaNode::BoolSchema(true) | SchemaNode::Any => true,

        SchemaNode::Enum(e) => e.contains(val),

        SchemaNode::AllOf(subs) => subs.iter().all(|s| instance_is_valid_against(val, s)),
        SchemaNode::AnyOf(subs) => subs.iter().any(|s| instance_is_valid_against(val, s)),
        SchemaNode::OneOf(subs) => {
            let mut count = 0;
            for s in subs {
                if instance_is_valid_against(val, s) {
                    count += 1;
                }
            }
            count == 1
        }
        SchemaNode::Not(sub) => !instance_is_valid_against(val, sub),
        SchemaNode::IfThenElse {
            if_schema,
            then_schema,
            else_schema,
        } => {
            if instance_is_valid_against(val, if_schema) {
                if let Some(t) = then_schema {
                    instance_is_valid_against(val, t)
                } else {
                    true
                }
            } else if let Some(e) = else_schema {
                instance_is_valid_against(val, e)
            } else {
                true
            }
        }

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
            val.as_i64().is_some()
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
        SchemaNode::Object { .. } | SchemaNode::Array { .. } => {
            // Very naïve – treat any object/array as valid unless an enum specifies otherwise.
            true
        }

        // The remaining variants are annotations or placeholders – they do not
        // restrict the instance space in this minimal implementation.
        _ => true,
    }
}
