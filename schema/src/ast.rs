use anyhow::{Result, anyhow};
use percent_encoding::percent_decode_str;
use serde_json::Value;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::Rc;

/// Shared, interior-mutable representation of a JSON Schema node.  Using
/// reference counting allows multiple parents to point to the same node which
/// is required to faithfully model schemas containing recursive `$ref`s.
#[derive(Clone)]
pub struct SchemaNode(Rc<RefCell<SchemaNodeKind>>);

impl SchemaNode {
    pub fn new(kind: SchemaNodeKind) -> Self {
        Self(Rc::new(RefCell::new(kind)))
    }

    pub fn bool_schema(value: bool) -> Self {
        Self::new(SchemaNodeKind::BoolSchema(value))
    }

    pub fn any() -> Self {
        Self::new(SchemaNodeKind::Any)
    }

    pub fn borrow(&self) -> Ref<'_, SchemaNodeKind> {
        self.0.borrow()
    }

    pub fn borrow_mut(&self) -> RefMut<'_, SchemaNodeKind> {
        self.0.borrow_mut()
    }

    fn ptr_id(&self) -> usize {
        Rc::as_ptr(&self.0) as usize
    }

    pub fn ptr_eq(&self, other: &SchemaNode) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    /// Convert the AST node back into a *minimal* JSON representation.  This
    /// is **lossy** for complex scenarios but is sufficient for the validator
    /// tests and fuzz harness (which only relies on the subset of keywords we
    /// explicitly generate).
    pub fn to_json(&self) -> Value {
        use SchemaNodeKind::*;

        match &*self.borrow() {
            BoolSchema(b) => Value::Bool(*b),
            Any => Value::Object(serde_json::Map::new()),

            Enum(values) => {
                let mut obj = serde_json::Map::new();
                obj.insert("enum".into(), Value::Array(values.clone()));
                Value::Object(obj)
            }

            String {
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

            Number {
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
                if *exclusive_minimum
                    && let Some(m) = minimum {
                        obj.insert(
                            "exclusiveMinimum".into(),
                            Value::Number(serde_json::Number::from_f64(*m).unwrap()),
                        );
                    }
                if *exclusive_maximum
                    && let Some(m) = maximum {
                        obj.insert(
                            "exclusiveMaximum".into(),
                            Value::Number(serde_json::Number::from_f64(*m).unwrap()),
                        );
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

            Integer {
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
                if *exclusive_minimum
                    && let Some(m) = minimum {
                        obj.insert("exclusiveMinimum".into(), Value::Number((*m).into()));
                    }
                if *exclusive_maximum
                    && let Some(m) = maximum {
                        obj.insert("exclusiveMaximum".into(), Value::Number((*m).into()));
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

            Boolean { enumeration } => {
                let mut obj = serde_json::Map::new();
                obj.insert("type".into(), Value::String("boolean".into()));
                if let Some(e) = enumeration {
                    obj.insert("enum".into(), Value::Array(e.clone()));
                }
                Value::Object(obj)
            }

            Null { enumeration } => {
                let mut obj = serde_json::Map::new();
                obj.insert("type".into(), Value::String("null".into()));
                if let Some(e) = enumeration {
                    obj.insert("enum".into(), Value::Array(e.clone()));
                }
                Value::Object(obj)
            }

            AllOf(subs) => {
                let arr = subs.iter().map(|s| s.to_json()).collect();
                let mut obj = serde_json::Map::new();
                obj.insert("allOf".into(), Value::Array(arr));
                Value::Object(obj)
            }
            AnyOf(subs) => {
                let arr = subs.iter().map(|s| s.to_json()).collect();
                let mut obj = serde_json::Map::new();
                obj.insert("anyOf".into(), Value::Array(arr));
                Value::Object(obj)
            }
            OneOf(subs) => {
                let arr = subs.iter().map(|s| s.to_json()).collect();
                let mut obj = serde_json::Map::new();
                obj.insert("oneOf".into(), Value::Array(arr));
                Value::Object(obj)
            }
            Not(sub) => {
                let mut obj = serde_json::Map::new();
                obj.insert("not".into(), sub.to_json());
                Value::Object(obj)
            }
            IfThenElse {
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

            Array {
                items,
                min_items,
                max_items,
                contains,
                enumeration,
            } => {
                let mut obj = serde_json::Map::new();
                obj.insert("type".into(), Value::String("array".into()));
                if !matches!(&*items.borrow(), SchemaNodeKind::Any) {
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

            Object {
                properties,
                required,
                additional,
                property_names,
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
                    let mut sorted: Vec<_> = required.iter().cloned().collect();
                    sorted.sort();
                    obj.insert(
                        "required".into(),
                        Value::Array(sorted.into_iter().map(Value::String).collect()),
                    );
                }

                match &*additional.borrow() {
                    SchemaNodeKind::Any => {}
                    SchemaNodeKind::BoolSchema(b) => {
                        obj.insert("additionalProperties".into(), Value::Bool(*b));
                    }
                    _ => {
                        obj.insert("additionalProperties".into(), additional.to_json());
                    }
                }

                match &*property_names.borrow() {
                    SchemaNodeKind::Any | SchemaNodeKind::BoolSchema(true) => {}
                    SchemaNodeKind::BoolSchema(b) => {
                        obj.insert("propertyNames".into(), Value::Bool(*b));
                    }
                    _ => {
                        obj.insert("propertyNames".into(), property_names.to_json());
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

            Defs(map) => {
                let mut defs_obj = serde_json::Map::new();
                for (k, v) in map {
                    defs_obj.insert(k.clone(), v.to_json());
                }
                let mut obj = serde_json::Map::new();
                obj.insert("$defs".into(), Value::Object(defs_obj));
                Value::Object(obj)
            }

            Const(v) => {
                let mut obj = serde_json::Map::new();
                obj.insert("const".into(), v.clone());
                Value::Object(obj)
            }
            Type(t) => {
                let mut obj = serde_json::Map::new();
                obj.insert("type".into(), Value::String(t.clone()));
                Value::Object(obj)
            }
            Minimum(m) => {
                let mut obj = serde_json::Map::new();
                obj.insert(
                    "minimum".into(),
                    Value::Number(serde_json::Number::from_f64(*m).unwrap()),
                );
                Value::Object(obj)
            }
            Maximum(m) => {
                let mut obj = serde_json::Map::new();
                obj.insert(
                    "maximum".into(),
                    Value::Number(serde_json::Number::from_f64(*m).unwrap()),
                );
                Value::Object(obj)
            }
            Required(reqs) => {
                let mut sorted = reqs.clone();
                sorted.sort();
                let mut obj = serde_json::Map::new();
                obj.insert(
                    "required".into(),
                    Value::Array(sorted.into_iter().map(Value::String).collect()),
                );
                Value::Object(obj)
            }
            AdditionalProperties(schema) => {
                let mut obj = serde_json::Map::new();
                obj.insert("additionalProperties".into(), schema.to_json());
                Value::Object(obj)
            }

            Format(f) => {
                let mut obj = serde_json::Map::new();
                obj.insert("format".into(), Value::String(f.clone()));
                Value::Object(obj)
            }
            ContentEncoding(c) => {
                let mut obj = serde_json::Map::new();
                obj.insert("contentEncoding".into(), Value::String(c.clone()));
                Value::Object(obj)
            }
            ContentMediaType(c) => {
                let mut obj = serde_json::Map::new();
                obj.insert("contentMediaType".into(), Value::String(c.clone()));
                Value::Object(obj)
            }

            Title(t) => {
                let mut obj = serde_json::Map::new();
                obj.insert("title".into(), Value::String(t.clone()));
                Value::Object(obj)
            }
            Description(d) => {
                let mut obj = serde_json::Map::new();
                obj.insert("description".into(), Value::String(d.clone()));
                Value::Object(obj)
            }
            Default(def) => {
                let mut obj = serde_json::Map::new();
                obj.insert("default".into(), def.clone());
                Value::Object(obj)
            }
            Examples(ex) => {
                let mut obj = serde_json::Map::new();
                obj.insert("examples".into(), Value::Array(ex.clone()));
                Value::Object(obj)
            }
            ReadOnly(b) => {
                let mut obj = serde_json::Map::new();
                obj.insert("readOnly".into(), Value::Bool(*b));
                Value::Object(obj)
            }
            WriteOnly(b) => {
                let mut obj = serde_json::Map::new();
                obj.insert("writeOnly".into(), Value::Bool(*b));
                Value::Object(obj)
            }

            Ref(r) => {
                let mut obj = serde_json::Map::new();
                obj.insert("$ref".into(), Value::String(r.clone()));
                Value::Object(obj)
            }
        }
    }
}

impl fmt::Debug for SchemaNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SchemaNode")
            .field("id", &self.ptr_id())
            .finish()
    }
}

impl PartialEq for SchemaNode {
    fn eq(&self, other: &Self) -> bool {
        fn eq_inner(a: &SchemaNode, b: &SchemaNode, seen: &mut HashSet<(usize, usize)>) -> bool {
            use SchemaNodeKind::*;

            let key = (a.ptr_id(), b.ptr_id());
            if !seen.insert(key) {
                return true;
            }

            let a_kind = a.borrow();
            let b_kind = b.borrow();

            match (&*a_kind, &*b_kind) {
                (BoolSchema(ax), BoolSchema(bx)) => ax == bx,
                (Any, Any) => true,
                (
                    String {
                        min_length: ax,
                        max_length: ay,
                        pattern: ap,
                        enumeration: ae,
                    },
                    String {
                        min_length: bx,
                        max_length: by,
                        pattern: bp,
                        enumeration: be,
                    },
                ) => ax == bx && ay == by && ap == bp && ae == be,
                (
                    Number {
                        minimum: amin,
                        maximum: amax,
                        exclusive_minimum: aexmin,
                        exclusive_maximum: aexmax,
                        multiple_of: amul,
                        enumeration: aenum,
                    },
                    Number {
                        minimum: bmin,
                        maximum: bmax,
                        exclusive_minimum: bexmin,
                        exclusive_maximum: bexmax,
                        multiple_of: bmul,
                        enumeration: benum,
                    },
                ) => {
                    amin == bmin
                        && amax == bmax
                        && aexmin == bexmin
                        && aexmax == bexmax
                        && amul == bmul
                        && aenum == benum
                }
                (
                    Integer {
                        minimum: amin,
                        maximum: amax,
                        exclusive_minimum: aexmin,
                        exclusive_maximum: aexmax,
                        multiple_of: amul,
                        enumeration: aenum,
                    },
                    Integer {
                        minimum: bmin,
                        maximum: bmax,
                        exclusive_minimum: bexmin,
                        exclusive_maximum: bexmax,
                        multiple_of: bmul,
                        enumeration: benum,
                    },
                ) => {
                    amin == bmin
                        && amax == bmax
                        && aexmin == bexmin
                        && aexmax == bexmax
                        && amul == bmul
                        && aenum == benum
                }
                (Boolean { enumeration: ae }, Boolean { enumeration: be }) => ae == be,
                (Null { enumeration: ae }, Null { enumeration: be }) => ae == be,
                (
                    Object {
                        properties: aprops,
                        required: areq,
                        additional: aaddl,
                        property_names: apropnames,
                        min_properties: amin,
                        max_properties: amax,
                        dependent_required: adep,
                        enumeration: aenum,
                    },
                    Object {
                        properties: bprops,
                        required: breq,
                        additional: baddl,
                        property_names: bpropnames,
                        min_properties: bmin,
                        max_properties: bmax,
                        dependent_required: bdep,
                        enumeration: benum,
                    },
                ) => {
                    if areq != breq
                        || amin != bmin
                        || amax != bmax
                        || adep != bdep
                        || aenum != benum
                        || !eq_inner(apropnames, bpropnames, seen)
                        || aprops.len() != bprops.len()
                    {
                        return false;
                    }
                    for (k, aval) in aprops {
                        let Some(bval) = bprops.get(k) else {
                            return false;
                        };
                        if !eq_inner(aval, bval, seen) {
                            return false;
                        }
                    }
                    eq_inner(aaddl, baddl, seen)
                }
                (
                    Array {
                        items: aitems,
                        min_items: amin,
                        max_items: amax,
                        contains: acontains,
                        enumeration: aenum,
                    },
                    Array {
                        items: bitems,
                        min_items: bmin,
                        max_items: bmax,
                        contains: bcontains,
                        enumeration: benum,
                    },
                ) => {
                    if amin != bmin || amax != bmax || aenum != benum {
                        return false;
                    }
                    if !eq_inner(aitems, bitems, seen) {
                        return false;
                    }
                    match (acontains, bcontains) {
                        (None, None) => true,
                        (Some(av), Some(bv)) => eq_inner(av, bv, seen),
                        _ => false,
                    }
                }
                (Defs(a), Defs(b)) => {
                    if a.len() != b.len() {
                        return false;
                    }
                    for (k, aval) in a {
                        let Some(bval) = b.get(k) else {
                            return false;
                        };
                        if !eq_inner(aval, bval, seen) {
                            return false;
                        }
                    }
                    true
                }
                (AllOf(a), AllOf(b)) | (AnyOf(a), AnyOf(b)) | (OneOf(a), OneOf(b)) => {
                    if a.len() != b.len() {
                        return false;
                    }
                    for (av, bv) in a.iter().zip(b.iter()) {
                        if !eq_inner(av, bv, seen) {
                            return false;
                        }
                    }
                    true
                }
                (Not(a), Not(b)) => eq_inner(a, b, seen),
                (
                    IfThenElse {
                        if_schema: a_if,
                        then_schema: a_then,
                        else_schema: a_else,
                    },
                    IfThenElse {
                        if_schema: b_if,
                        then_schema: b_then,
                        else_schema: b_else,
                    },
                ) => {
                    if !eq_inner(a_if, b_if, seen) {
                        return false;
                    }
                    match (a_then, b_then) {
                        (None, None) => {}
                        (Some(av), Some(bv)) => {
                            if !eq_inner(av, bv, seen) {
                                return false;
                            }
                        }
                        _ => return false,
                    }
                    match (a_else, b_else) {
                        (None, None) => true,
                        (Some(av), Some(bv)) => eq_inner(av, bv, seen),
                        _ => false,
                    }
                }
                (Const(a), Const(b)) => a == b,
                (Enum(a), Enum(b)) => a == b,
                (Type(a), Type(b)) => a == b,
                (Minimum(a), Minimum(b)) => a == b,
                (Maximum(a), Maximum(b)) => a == b,
                (Required(a), Required(b)) => a == b,
                (AdditionalProperties(a), AdditionalProperties(b)) => eq_inner(a, b, seen),
                (Format(a), Format(b)) => a == b,
                (ContentEncoding(a), ContentEncoding(b)) => a == b,
                (ContentMediaType(a), ContentMediaType(b)) => a == b,
                (Title(a), Title(b)) => a == b,
                (Description(a), Description(b)) => a == b,
                (Default(a), Default(b)) => a == b,
                (Examples(a), Examples(b)) => a == b,
                (ReadOnly(a), ReadOnly(b)) => a == b,
                (WriteOnly(a), WriteOnly(b)) => a == b,
                (Ref(a), Ref(b)) => a == b,
                _ => false,
            }
        }

        eq_inner(self, other, &mut HashSet::new())
    }
}

impl Eq for SchemaNode {}

/// An internal Abstract Syntax Tree (AST) representing a fully-resolved JSON
/// Schema draft-2020-12 document.  The node types are deliberately *very*
/// close to the JSON Schema specification so that higher-level crates (e.g.
/// the back-compat checker or fuzz generator) can reason about schemas
/// without constantly reparsing raw JSON values.
#[derive(Debug, Clone)]
pub enum SchemaNodeKind {
    BoolSchema(bool),
    Any,

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

    Object {
        properties: HashMap<String, SchemaNode>,
        required: HashSet<String>,
        additional: SchemaNode,
        property_names: SchemaNode,
        min_properties: Option<usize>,
        max_properties: Option<usize>,
        dependent_required: HashMap<String, Vec<String>>,
        enumeration: Option<Vec<Value>>,
    },
    Array {
        items: SchemaNode,
        min_items: Option<u64>,
        max_items: Option<u64>,
        contains: Option<SchemaNode>,
        enumeration: Option<Vec<Value>>,
    },

    Defs(HashMap<String, SchemaNode>),

    AllOf(Vec<SchemaNode>),
    AnyOf(Vec<SchemaNode>),
    OneOf(Vec<SchemaNode>),
    Not(SchemaNode),
    IfThenElse {
        if_schema: SchemaNode,
        then_schema: Option<SchemaNode>,
        else_schema: Option<SchemaNode>,
    },

    Const(Value),
    Enum(Vec<Value>),
    Type(String),
    Minimum(f64),
    Maximum(f64),
    Required(Vec<String>),
    AdditionalProperties(SchemaNode),

    Format(String),
    ContentEncoding(String),
    ContentMediaType(String),

    Title(String),
    Description(String),
    Default(Value),
    Examples(Vec<Value>),
    ReadOnly(bool),
    WriteOnly(bool),

    Ref(String),
}

/// Build and fully resolve a schema node from raw JSON + a base URL.
pub fn build_and_resolve_schema(raw: &Value) -> Result<SchemaNode> {
    let mut root = build_schema_ast(raw)?;
    resolve_refs(&mut root, raw, &[])?;
    Ok(root)
}

/// Build the high-level AST without immediately resolving references.
pub fn build_schema_ast(raw: &Value) -> Result<SchemaNode> {
    if let Some(b) = raw.as_bool() {
        return Ok(SchemaNode::bool_schema(b));
    }
    if !raw.is_object() {
        return Ok(SchemaNode::any());
    }

    let obj = raw.as_object().unwrap();

    if let Some(Value::String(r)) = obj.get("$ref") {
        return Ok(SchemaNode::new(SchemaNodeKind::Ref(r.to_owned())));
    }

    if let Some(Value::Array(e)) = obj.get("enum") {
        return Ok(SchemaNode::new(SchemaNodeKind::Enum(e.clone())));
    }

    if let Some(c) = obj.get("const") {
        return Ok(SchemaNode::new(SchemaNodeKind::Const(c.clone())));
    }

    if obj.contains_key("if") || obj.contains_key("then") || obj.contains_key("else") {
        if let Some(cond) = obj.get("if") {
            let if_schema = build_schema_ast(cond)?;
            let then_schema = match obj.get("then") {
                Some(v) => Some(build_schema_ast(v)?),
                None => None,
            };
            let else_schema = match obj.get("else") {
                Some(v) => Some(build_schema_ast(v)?),
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
            let cond_node = SchemaNode::new(SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            });
            if !base.is_empty() {
                let subs = vec![build_schema_ast(&Value::Object(base))?, cond_node.clone()];
                return Ok(SchemaNode::new(SchemaNodeKind::AllOf(subs)));
            } else {
                return Ok(cond_node);
            }
        } else {
            let mut base = obj.clone();
            base.remove("then");
            base.remove("else");
            return build_schema_ast(&Value::Object(base));
        }
    }

    if let Some(Value::Array(subs)) = obj.get("allOf") {
        let mut list = Vec::new();
        if obj.len() > 1 {
            let mut base = obj.clone();
            base.remove("allOf");
            const META_KEYS: [&str; 4] = ["$schema", "$id", "$comment", "$defs"];
            for key in META_KEYS {
                base.remove(key);
            }
            if !base.is_empty() {
                list.push(build_schema_ast(&Value::Object(base))?);
            }
        }
        for s in subs {
            list.push(build_schema_ast(s)?);
        }
        return Ok(SchemaNode::new(SchemaNodeKind::AllOf(list)));
    }
    if let Some(Value::Array(subs)) = obj.get("anyOf") {
        let parsed = subs
            .iter()
            .map(build_schema_ast)
            .collect::<Result<Vec<_>>>()?;
        return Ok(SchemaNode::new(SchemaNodeKind::AnyOf(parsed)));
    }
    if let Some(Value::Array(subs)) = obj.get("oneOf") {
        let parsed = subs
            .iter()
            .map(build_schema_ast)
            .collect::<Result<Vec<_>>>()?;
        return Ok(SchemaNode::new(SchemaNodeKind::OneOf(parsed)));
    }
    if let Some(n) = obj.get("not") {
        return Ok(SchemaNode::new(SchemaNodeKind::Not(build_schema_ast(n)?)));
    }

    match obj.get("type") {
        Some(Value::String(t)) => match t.as_str() {
            "string" => parse_string_schema(obj),
            "number" => parse_number_schema(obj, false),
            "integer" => parse_number_schema(obj, true),
            "boolean" => parse_boolean_schema(obj),
            "null" => parse_null_schema(obj),
            "object" => parse_object_schema(obj),
            "array" => parse_array_schema(obj),
            _ => Ok(SchemaNode::any()),
        },
        Some(Value::Array(arr)) => {
            let mut variants = Vec::new();
            for t_val in arr {
                if let Some(t_str) = t_val.as_str() {
                    let mut cloned = obj.clone();
                    cloned.insert("type".into(), Value::String(t_str.into()));
                    variants.push(build_schema_ast(&Value::Object(cloned))?);
                }
            }
            if variants.len() == 1 {
                Ok(variants.remove(0))
            } else {
                Ok(SchemaNode::new(SchemaNodeKind::AnyOf(variants)))
            }
        }
        _ => {
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
                parse_string_schema(obj)
            } else {
                Ok(SchemaNode::any())
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

    Ok(SchemaNode::new(SchemaNodeKind::String {
        min_length,
        max_length,
        pattern,
        enumeration,
    }))
}

fn parse_number_schema(obj: &serde_json::Map<String, Value>, integer: bool) -> Result<SchemaNode> {
    let mut minimum = obj.get("minimum").and_then(|v| v.as_f64());
    let mut maximum = obj.get("maximum").and_then(|v| v.as_f64());

    let exclusive_minimum = if let Some(Value::Number(n)) = obj.get("exclusiveMinimum") {
        minimum = n.as_f64();
        true
    } else {
        false
    };

    let exclusive_maximum = if let Some(Value::Number(n)) = obj.get("exclusiveMaximum") {
        maximum = n.as_f64();
        true
    } else {
        false
    };
    let enumeration = obj.get("enum").and_then(|v| v.as_array()).cloned();

    let multiple_of = obj
        .get("multipleOf")
        .and_then(|v| v.as_f64())
        .filter(|m| *m > 0.0);

    if integer {
        let min_i = minimum.map(|m| m as i64);
        let max_i = maximum.map(|m| m as i64);
        Ok(SchemaNode::new(SchemaNodeKind::Integer {
            minimum: min_i,
            maximum: max_i,
            exclusive_minimum,
            exclusive_maximum,
            multiple_of,
            enumeration,
        }))
    } else {
        Ok(SchemaNode::new(SchemaNodeKind::Number {
            minimum,
            maximum,
            exclusive_minimum,
            exclusive_maximum,
            multiple_of,
            enumeration,
        }))
    }
}

fn parse_boolean_schema(obj: &serde_json::Map<String, Value>) -> Result<SchemaNode> {
    let enumeration = obj.get("enum").and_then(|v| v.as_array()).cloned();
    Ok(SchemaNode::new(SchemaNodeKind::Boolean { enumeration }))
}

fn parse_null_schema(obj: &serde_json::Map<String, Value>) -> Result<SchemaNode> {
    let enumeration = obj.get("enum").and_then(|v| v.as_array()).cloned();
    Ok(SchemaNode::new(SchemaNodeKind::Null { enumeration }))
}

fn parse_object_schema(obj: &serde_json::Map<String, Value>) -> Result<SchemaNode> {
    let mut properties = HashMap::new();
    if let Some(Value::Object(props)) = obj.get("properties") {
        for (k, v) in props {
            properties.insert(k.clone(), build_schema_ast(v)?);
        }
    }
    let required: HashSet<String> = obj
        .get("required")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_owned()))
                .collect()
        })
        .unwrap_or_default();

    for name in &required {
        if !properties.contains_key(name) {
            properties.insert(name.clone(), SchemaNode::any());
        }
    }

    let additional = match obj.get("additionalProperties") {
        None => SchemaNode::any(),
        Some(Value::Bool(b)) => SchemaNode::bool_schema(*b),
        Some(other) => build_schema_ast(other)?,
    };

    let property_names = match obj.get("propertyNames") {
        None => SchemaNode::any(),
        Some(Value::Bool(b)) => SchemaNode::bool_schema(*b),
        Some(other) => build_schema_ast(other)?,
    };

    let min_properties = obj
        .get("minProperties")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);
    let max_properties = obj
        .get("maxProperties")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);
    let dependent_required = obj
        .get("dependentRequired")
        .and_then(|v| v.as_object())
        .map(|map| {
            map.iter()
                .map(|(k, v)| {
                    let deps = v
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_owned()))
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    (k.clone(), deps)
                })
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();
    let enumeration = obj.get("enum").and_then(|v| v.as_array()).cloned();

    Ok(SchemaNode::new(SchemaNodeKind::Object {
        properties,
        required,
        additional,
        property_names,
        min_properties,
        max_properties,
        dependent_required,
        enumeration,
    }))
}

fn parse_array_schema(obj: &serde_json::Map<String, Value>) -> Result<SchemaNode> {
    let items_node = match obj.get("items") {
        None => SchemaNode::any(),
        Some(Value::Array(arr)) => {
            if arr.is_empty() {
                SchemaNode::any()
            } else if arr.len() == 1 {
                build_schema_ast(&arr[0])?
            } else {
                let subnodes = arr
                    .iter()
                    .map(build_schema_ast)
                    .collect::<Result<Vec<SchemaNode>>>()?;
                SchemaNode::new(SchemaNodeKind::AllOf(subnodes))
            }
        }
        Some(other) => build_schema_ast(other)?,
    };
    let min_items = obj.get("minItems").and_then(|v| v.as_u64());
    let max_items = obj.get("maxItems").and_then(|v| v.as_u64());
    let enumeration = obj.get("enum").and_then(|v| v.as_array()).cloned();

    let contains_node = match obj.get("contains") {
        None => None,
        Some(v) => Some(build_schema_ast(v)?),
    };

    Ok(SchemaNode::new(SchemaNodeKind::Array {
        items: items_node,
        min_items,
        max_items,
        contains: contains_node,
        enumeration,
    }))
}

/// Recursively resolves `SchemaNode::Ref` by looking up fragments in `root_json`.
pub fn resolve_refs(node: &mut SchemaNode, root_json: &Value, visited: &[String]) -> Result<()> {
    let mut stack = visited.to_vec();
    let mut cache: HashMap<String, SchemaNode> = HashMap::new();
    resolve_refs_internal(node, root_json, &mut stack, &mut cache)
}

fn resolve_refs_internal(
    node: &mut SchemaNode,
    root_json: &Value,
    stack: &mut Vec<String>,
    cache: &mut HashMap<String, SchemaNode>,
) -> Result<()> {
    let ref_path = {
        let guard = node.borrow();
        if let SchemaNodeKind::Ref(p) = &*guard {
            Some(p.clone())
        } else {
            None
        }
    };

    if let Some(path) = ref_path {
        if let Some(existing) = cache.get(&path) {
            *node = existing.clone();
            return Ok(());
        }

        if let Some(stripped) = path.strip_prefix("#/") {
            let parts: Vec<String> = stripped
                .split('/')
                .map(|token| {
                    let mut decoded = percent_decode_str(token).decode_utf8_lossy().into_owned();
                    decoded = decoded.replace("~1", "/");
                    decoded.replace("~0", "~")
                })
                .collect();
            let mut current = root_json;
            for p in &parts {
                if let Some(next) = current.get(p.as_str()) {
                    current = next;
                } else {
                    return Err(anyhow!("Unresolved reference: {}", path));
                }
            }
            let mut resolved = build_schema_ast(current)?;
            cache.insert(path.clone(), resolved.clone());
            stack.push(path.clone());
            resolve_refs_internal(&mut resolved, root_json, stack, cache)?;
            stack.pop();
            cache.insert(path.clone(), resolved.clone());
            *node = resolved;
        } else {
            *node.borrow_mut() = SchemaNodeKind::BoolSchema(true);
        }
        return Ok(());
    }

    if let SchemaNodeKind::AllOf(children) = &mut *node.borrow_mut() {
        for child in children.iter_mut() {
            resolve_refs_internal(child, root_json, stack, cache)?;
        }
        return Ok(());
    }
    if let SchemaNodeKind::AnyOf(children) = &mut *node.borrow_mut() {
        for child in children.iter_mut() {
            resolve_refs_internal(child, root_json, stack, cache)?;
        }
        return Ok(());
    }
    if let SchemaNodeKind::OneOf(children) = &mut *node.borrow_mut() {
        for child in children.iter_mut() {
            resolve_refs_internal(child, root_json, stack, cache)?;
        }
        return Ok(());
    }
    if let SchemaNodeKind::IfThenElse {
        if_schema,
        then_schema,
        else_schema,
    } = &mut *node.borrow_mut()
    {
        resolve_refs_internal(if_schema, root_json, stack, cache)?;
        if let Some(t) = then_schema {
            resolve_refs_internal(t, root_json, stack, cache)?;
        }
        if let Some(e) = else_schema {
            resolve_refs_internal(e, root_json, stack, cache)?;
        }
        return Ok(());
    }
    if let SchemaNodeKind::Not(sub) = &mut *node.borrow_mut() {
        resolve_refs_internal(sub, root_json, stack, cache)?;
        return Ok(());
    }
    if let SchemaNodeKind::Object {
        properties,
        additional,
        property_names,
        ..
    } = &mut *node.borrow_mut()
    {
        for child in properties.values_mut() {
            resolve_refs_internal(child, root_json, stack, cache)?;
        }
        resolve_refs_internal(additional, root_json, stack, cache)?;
        resolve_refs_internal(property_names, root_json, stack, cache)?;
        return Ok(());
    }
    if let SchemaNodeKind::Array {
        items, contains, ..
    } = &mut *node.borrow_mut()
    {
        resolve_refs_internal(items, root_json, stack, cache)?;
        if let Some(child) = contains {
            resolve_refs_internal(child, root_json, stack, cache)?;
        }
        return Ok(());
    }
    if let SchemaNodeKind::AdditionalProperties(schema) = &mut *node.borrow_mut() {
        resolve_refs_internal(schema, root_json, stack, cache)?;
        return Ok(());
    }
    if let SchemaNodeKind::Defs(map) = &mut *node.borrow_mut() {
        for child in map.values_mut() {
            resolve_refs_internal(child, root_json, stack, cache)?;
        }
        return Ok(());
    }

    Ok(())
}

/// Minimal check if an *instance* `val` is valid against `schema`.
///
/// This helper purposefully supports only the keyword subset that the fuzz
/// generator and back-compat checker rely on.  It is **not** a full JSON
/// Schema validator.
pub fn instance_is_valid_against(val: &Value, schema: &SchemaNode) -> bool {
    use SchemaNodeKind::*;

    match &*schema.borrow() {
        BoolSchema(false) => false,
        BoolSchema(true) | Any => true,

        Enum(e) => e.contains(val),

        AllOf(subs) => subs.iter().all(|s| instance_is_valid_against(val, s)),
        AnyOf(subs) => subs.iter().any(|s| instance_is_valid_against(val, s)),
        OneOf(subs) => {
            let mut count = 0;
            for s in subs {
                if instance_is_valid_against(val, s) {
                    count += 1;
                }
            }
            count == 1
        }
        Not(sub) => !instance_is_valid_against(val, sub),
        IfThenElse {
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

        String { enumeration, .. } => {
            if let Some(e) = enumeration
                && !e.contains(val) {
                    return false;
                }
            val.is_string()
        }
        Number { enumeration, .. } => {
            if let Some(e) = enumeration
                && !e.contains(val) {
                    return false;
                }
            val.is_number()
        }
        Integer { enumeration, .. } => {
            if let Some(e) = enumeration
                && !e.contains(val) {
                    return false;
                }
            val.as_i64().is_some()
        }
        Boolean { enumeration } => {
            if let Some(e) = enumeration
                && !e.contains(val) {
                    return false;
                }
            val.is_boolean()
        }
        Null { enumeration } => {
            if let Some(e) = enumeration
                && !e.contains(val) {
                    return false;
                }
            val.is_null()
        }
        Object { .. } | Array { .. } => true,

        _ => true,
    }
}
