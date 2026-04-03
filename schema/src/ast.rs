use crate::SchemaError;
use crate::canonicalize::{CanonicalSchema, canonicalize_schema};
use crate::schema_metadata::{is_schema_metadata_key, strip_schema_metadata};
use percent_encoding::percent_decode_str;
use serde_json::{Map, Value};
use std::cell::{Ref, RefCell, RefMut};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::Rc;

type Result<T> = std::result::Result<T, AstError>;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AstError {
    #[error(transparent)]
    Schema(#[from] SchemaError),
    #[error("local $ref '{ref_path}' does not resolve to a schema node in the current document")]
    UnresolvedReference { ref_path: String },
}

/// Shared, interior-mutable representation of a JSON Schema node.  Using
/// reference counting allows multiple parents to point to the same node which
/// is required to faithfully model schemas containing recursive `$ref`s.
#[derive(Clone)]
pub struct SchemaNode(Rc<RefCell<SchemaNodeKind>>);

impl SchemaNode {
    pub(crate) fn new(kind: SchemaNodeKind) -> Self {
        Self(Rc::new(RefCell::new(kind)))
    }

    pub(crate) fn bool_schema(value: bool) -> Self {
        Self::new(SchemaNodeKind::BoolSchema(value))
    }

    pub(crate) fn any() -> Self {
        Self::new(SchemaNodeKind::Any)
    }

    pub fn borrow(&self) -> Ref<'_, SchemaNodeKind> {
        self.0.borrow()
    }

    pub(crate) fn borrow_mut(&self) -> RefMut<'_, SchemaNodeKind> {
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
                format,
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
                if let Some(f) = format {
                    obj.insert("format".into(), Value::String(f.clone()));
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
                if *exclusive_minimum && let Some(m) = minimum {
                    obj.insert(
                        "exclusiveMinimum".into(),
                        Value::Number(serde_json::Number::from_f64(*m).unwrap()),
                    );
                }
                if *exclusive_maximum && let Some(m) = maximum {
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
                if *exclusive_minimum && let Some(m) = minimum {
                    obj.insert("exclusiveMinimum".into(), Value::Number((*m).into()));
                }
                if *exclusive_maximum && let Some(m) = maximum {
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
                min_contains,
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
                if let Some(min_contains) = min_contains
                    && *min_contains != 1
                {
                    obj.insert("minContains".into(), Value::Number((*min_contains).into()));
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
                (Any, BoolSchema(true)) | (BoolSchema(true), Any) => true,
                (
                    String {
                        min_length: ax,
                        max_length: ay,
                        pattern: ap,
                        format: af,
                        enumeration: ae,
                    },
                    String {
                        min_length: bx,
                        max_length: by,
                        pattern: bp,
                        format: bf,
                        enumeration: be,
                    },
                ) => ax == bx && ay == by && ap == bp && af == bf && ae == be,
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
                        min_contains: amin_contains,
                        enumeration: aenum,
                    },
                    Array {
                        items: bitems,
                        min_items: bmin,
                        max_items: bmax,
                        contains: bcontains,
                        min_contains: bmin_contains,
                        enumeration: benum,
                    },
                ) => {
                    if amin != bmin
                        || amax != bmax
                        || amin_contains != bmin_contains
                        || aenum != benum
                    {
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
#[non_exhaustive]
pub enum SchemaNodeKind {
    BoolSchema(bool),
    Any,

    String {
        min_length: Option<u64>,
        max_length: Option<u64>,
        pattern: Option<String>,
        format: Option<String>,
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
        min_contains: Option<u64>,
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

/// Build and fully resolve a schema node from a JSON Schema document.
pub fn build_and_resolve_schema(raw: &Value) -> Result<SchemaNode> {
    let canonical = canonicalize_schema(raw)?;
    let mut root = build_schema_ast_from_canonical(&canonical)?;
    resolve_refs_from_canonical_root(&mut root, &canonical, &[])?;
    Ok(root)
}

fn build_schema_ast_from_canonical(raw: &CanonicalSchema) -> Result<SchemaNode> {
    build_schema_ast_from_value(raw.as_value())
}

fn build_schema_ast_from_value(raw: &Value) -> Result<SchemaNode> {
    if let Some(b) = raw.as_bool() {
        return Ok(SchemaNode::bool_schema(b));
    }
    let Some(obj) = raw.as_object() else {
        return Ok(SchemaNode::any());
    };

    match SchemaShape::classify(obj) {
        SchemaShape::Ref(ref_path) => Ok(parse_ref_schema(ref_path)),
        SchemaShape::Enum(values) => Ok(parse_enum_schema(values)),
        SchemaShape::Const(value) => Ok(parse_const_schema(value)),
        SchemaShape::Conditional {
            if_schema,
            then_schema,
            else_schema,
        } => parse_conditional_schema(obj, if_schema, then_schema, else_schema),
        SchemaShape::AllOf(subschemas) => parse_all_of_schema(obj, subschemas),
        SchemaShape::AnyOf(subschemas) => parse_any_of_schema(obj, subschemas),
        SchemaShape::OneOf(subschemas) => parse_one_of_schema(obj, subschemas),
        SchemaShape::Not(schema) => parse_not_schema(schema),
        SchemaShape::String => Ok(parse_string_schema(obj)),
        SchemaShape::Number => Ok(parse_number_schema(obj, false)),
        SchemaShape::Integer => Ok(parse_number_schema(obj, true)),
        SchemaShape::Boolean => Ok(parse_boolean_schema(obj)),
        SchemaShape::Null => Ok(parse_null_schema(obj)),
        SchemaShape::Object => parse_object_schema(obj),
        SchemaShape::Array => parse_array_schema(obj),
        SchemaShape::TypeUnion(type_names) => parse_type_union_schema(obj, type_names),
        SchemaShape::Any => Ok(SchemaNode::any()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SchemaShape<'a> {
    Ref(&'a str),
    Enum(&'a [Value]),
    Const(&'a Value),
    Conditional {
        if_schema: Option<&'a Value>,
        then_schema: Option<&'a Value>,
        else_schema: Option<&'a Value>,
    },
    AllOf(&'a [Value]),
    AnyOf(&'a [Value]),
    OneOf(&'a [Value]),
    Not(&'a Value),
    String,
    Number,
    Integer,
    Boolean,
    Null,
    Object,
    Array,
    TypeUnion(&'a [Value]),
    Any,
}

impl<'a> SchemaShape<'a> {
    #[must_use]
    fn classify(obj: &'a Map<String, Value>) -> Self {
        let keywords = SchemaKeywords::classify(obj);

        if let Some(ref_path) = keywords.ref_path {
            return Self::Ref(ref_path);
        }
        if let Some(values) = keywords.enum_values
            && keywords.has_only_one_non_metadata_keyword()
        {
            return Self::Enum(values);
        }
        if let Some(value) = keywords.const_value
            && keywords.has_only_one_non_metadata_keyword()
        {
            return Self::Const(value);
        }
        if keywords.flags.contains(SchemaKeywordFlags::CONDITIONAL) {
            return Self::Conditional {
                if_schema: keywords.if_schema,
                then_schema: keywords.then_schema,
                else_schema: keywords.else_schema,
            };
        }
        if let Some(subschemas) = keywords.all_of {
            return Self::AllOf(subschemas);
        }
        if let Some(subschemas) = keywords.any_of {
            return Self::AnyOf(subschemas);
        }
        if let Some(subschemas) = keywords.one_of {
            return Self::OneOf(subschemas);
        }
        if let Some(schema) = keywords.not_schema {
            return Self::Not(schema);
        }
        if let Some(shape) = keywords.type_shape {
            return shape;
        }
        if keywords.flags.contains(SchemaKeywordFlags::OBJECT)
            && keywords.values_are_all(Value::is_object)
        {
            return Self::Object;
        }
        if keywords.flags.contains(SchemaKeywordFlags::ARRAY)
            && keywords.values_are_all(Value::is_array)
        {
            return Self::Array;
        }
        if keywords.flags.contains(SchemaKeywordFlags::STRING)
            && keywords.values_are_all(Value::is_string)
        {
            return Self::String;
        }
        if keywords.flags.contains(SchemaKeywordFlags::NUMERIC) && keywords.values_are_all_numeric()
        {
            return Self::Number;
        }
        if let Some(values) = keywords.enum_values {
            return Self::Enum(values);
        }
        if let Some(value) = keywords.const_value {
            return Self::Const(value);
        }
        Self::Any
    }

    #[must_use]
    fn typed(type_name: &str) -> Self {
        match type_name {
            "string" => Self::String,
            "number" => Self::Number,
            "integer" => Self::Integer,
            "boolean" => Self::Boolean,
            "null" => Self::Null,
            "object" => Self::Object,
            "array" => Self::Array,
            _ => Self::Any,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct SchemaKeywordFlags {
    bits: u8,
}

impl SchemaKeywordFlags {
    const OBJECT: Self = Self { bits: 1 << 0 };
    const ARRAY: Self = Self { bits: 1 << 1 };
    const STRING: Self = Self { bits: 1 << 2 };
    const NUMERIC: Self = Self { bits: 1 << 3 };
    const CONDITIONAL: Self = Self { bits: 1 << 4 };

    #[must_use]
    const fn contains(self, flag: Self) -> bool {
        self.bits & flag.bits != 0
    }
}

impl std::ops::BitOrAssign for SchemaKeywordFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.bits |= rhs.bits;
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct SchemaKeywords<'a> {
    ref_path: Option<&'a str>,
    enum_values: Option<&'a [Value]>,
    const_value: Option<&'a Value>,
    if_schema: Option<&'a Value>,
    then_schema: Option<&'a Value>,
    else_schema: Option<&'a Value>,
    all_of: Option<&'a [Value]>,
    any_of: Option<&'a [Value]>,
    one_of: Option<&'a [Value]>,
    not_schema: Option<&'a Value>,
    type_shape: Option<SchemaShape<'a>>,
    flags: SchemaKeywordFlags,
    non_metadata_keyword_count: usize,
}

impl<'a> SchemaKeywords<'a> {
    #[must_use]
    fn classify(obj: &'a Map<String, Value>) -> Self {
        let mut keywords = Self::default();

        for (key, value) in obj {
            if !is_schema_metadata_key(key) {
                keywords.non_metadata_keyword_count += 1;
            }

            match key.as_str() {
                "$ref" => {
                    keywords.ref_path = value.as_str();
                }
                "enum" => {
                    keywords.enum_values = value.as_array().map(Vec::as_slice);
                }
                "const" => {
                    keywords.const_value = Some(value);
                }
                "if" => {
                    keywords.if_schema = Some(value);
                    keywords.flags |= SchemaKeywordFlags::CONDITIONAL;
                }
                "then" => {
                    keywords.then_schema = Some(value);
                    keywords.flags |= SchemaKeywordFlags::CONDITIONAL;
                }
                "else" => {
                    keywords.else_schema = Some(value);
                    keywords.flags |= SchemaKeywordFlags::CONDITIONAL;
                }
                "allOf" => {
                    keywords.all_of = value.as_array().map(Vec::as_slice);
                }
                "anyOf" => {
                    keywords.any_of = value.as_array().map(Vec::as_slice);
                }
                "oneOf" => {
                    keywords.one_of = value.as_array().map(Vec::as_slice);
                }
                "not" => {
                    keywords.not_schema = Some(value);
                }
                "type" => match value {
                    Value::String(type_name) => {
                        keywords.type_shape = Some(SchemaShape::typed(type_name));
                    }
                    Value::Array(type_names) => {
                        keywords.type_shape = Some(SchemaShape::TypeUnion(type_names.as_slice()));
                    }
                    _ => {}
                },
                "properties"
                | "patternProperties"
                | "minProperties"
                | "maxProperties"
                | "required"
                | "additionalProperties"
                | "propertyNames"
                | "dependentRequired"
                | "dependentSchemas"
                | "unevaluatedProperties" => {
                    keywords.flags |= SchemaKeywordFlags::OBJECT;
                }
                "items" | "prefixItems" | "contains" | "minItems" | "maxItems" | "minContains"
                | "maxContains" | "uniqueItems" | "unevaluatedItems" => {
                    keywords.flags |= SchemaKeywordFlags::ARRAY;
                }
                "minLength" | "maxLength" | "pattern" | "format" => {
                    keywords.flags |= SchemaKeywordFlags::STRING;
                }
                "minimum" | "maximum" | "exclusiveMinimum" | "exclusiveMaximum" | "multipleOf" => {
                    keywords.flags |= SchemaKeywordFlags::NUMERIC;
                }
                _ => {}
            }
        }

        keywords
    }

    #[must_use]
    const fn has_only_one_non_metadata_keyword(self) -> bool {
        self.non_metadata_keyword_count == 1
    }

    #[must_use]
    fn values_are_all_numeric(self) -> bool {
        (self.enum_values.is_some() || self.const_value.is_some())
            && self
                .enum_values
                .is_none_or(|values| values.iter().all(Value::is_number))
            && self.const_value.is_none_or(Value::is_number)
    }

    #[must_use]
    fn values_are_all(self, mut predicate: impl FnMut(&Value) -> bool) -> bool {
        self.enum_values
            .is_none_or(|values| values.iter().all(&mut predicate))
            && self.const_value.is_none_or(predicate)
    }
}

fn parse_ref_schema(ref_path: &str) -> SchemaNode {
    SchemaNode::new(SchemaNodeKind::Ref(ref_path.to_owned()))
}

fn parse_enum_schema(values: &[Value]) -> SchemaNode {
    SchemaNode::new(SchemaNodeKind::Enum(values.to_vec()))
}

fn parse_const_schema(value: &Value) -> SchemaNode {
    SchemaNode::new(SchemaNodeKind::Const(value.clone()))
}

fn parse_conditional_schema(
    obj: &Map<String, Value>,
    if_schema: Option<&Value>,
    then_schema: Option<&Value>,
    else_schema: Option<&Value>,
) -> Result<SchemaNode> {
    let Some(cond) = if_schema else {
        let mut base = obj.clone();
        base.remove("then");
        base.remove("else");
        return build_schema_ast_from_value(&Value::Object(base));
    };

    let if_schema = build_schema_ast_from_value(cond)?;
    let then_schema = then_schema.map(build_schema_ast_from_value).transpose()?;
    let else_schema = else_schema.map(build_schema_ast_from_value).transpose()?;

    let cond_node = SchemaNode::new(SchemaNodeKind::IfThenElse {
        if_schema,
        then_schema,
        else_schema,
    });

    if let Some(base_schema) = parse_applicator_base_schema(obj, &["if", "then", "else"])? {
        return Ok(SchemaNode::new(SchemaNodeKind::AllOf(vec![
            base_schema,
            cond_node,
        ])));
    }

    Ok(cond_node)
}

fn parse_all_of_schema(obj: &Map<String, Value>, subschemas: &[Value]) -> Result<SchemaNode> {
    let mut list = Vec::new();
    if let Some(base_schema) = parse_applicator_base_schema(obj, &["allOf"])? {
        list.push(base_schema);
    }
    for schema in subschemas {
        list.push(build_schema_ast_from_value(schema)?);
    }

    Ok(SchemaNode::new(SchemaNodeKind::AllOf(dedupe_schema_nodes(
        list,
    ))))
}

fn parse_any_of_schema(obj: &Map<String, Value>, subschemas: &[Value]) -> Result<SchemaNode> {
    let any_of = SchemaNode::new(SchemaNodeKind::AnyOf(dedupe_schema_nodes(
        subschemas
            .iter()
            .map(build_schema_ast_from_value)
            .collect::<Result<Vec<_>>>()?,
    )));

    if let Some(base_schema) = parse_applicator_base_schema(obj, &["anyOf"])? {
        return Ok(SchemaNode::new(SchemaNodeKind::AllOf(vec![
            base_schema,
            any_of,
        ])));
    }

    Ok(any_of)
}

fn parse_one_of_schema(obj: &Map<String, Value>, subschemas: &[Value]) -> Result<SchemaNode> {
    let one_of = SchemaNode::new(SchemaNodeKind::OneOf(
        subschemas
            .iter()
            .map(build_schema_ast_from_value)
            .collect::<Result<Vec<_>>>()?,
    ));

    if let Some(base_schema) = parse_applicator_base_schema(obj, &["oneOf"])? {
        return Ok(SchemaNode::new(SchemaNodeKind::AllOf(vec![
            base_schema,
            one_of,
        ])));
    }

    Ok(one_of)
}

fn parse_not_schema(schema: &Value) -> Result<SchemaNode> {
    Ok(SchemaNode::new(SchemaNodeKind::Not(
        build_schema_ast_from_value(schema)?,
    )))
}

fn parse_type_union_schema(obj: &Map<String, Value>, type_names: &[Value]) -> Result<SchemaNode> {
    let mut variants = Vec::new();
    for type_name in type_names {
        if let Some(type_name) = type_name.as_str() {
            let mut typed_obj = obj.clone();
            typed_obj.insert("type".into(), Value::String(type_name.into()));
            variants.push(build_schema_ast_from_value(&Value::Object(typed_obj))?);
        }
    }

    if variants.len() == 1 {
        Ok(variants.remove(0))
    } else {
        Ok(SchemaNode::new(SchemaNodeKind::AnyOf(dedupe_schema_nodes(
            variants,
        ))))
    }
}

fn parse_applicator_base_schema(
    obj: &Map<String, Value>,
    applicator_keys: &[&str],
) -> Result<Option<SchemaNode>> {
    let mut base = obj.clone();
    for key in applicator_keys {
        base.remove(*key);
    }
    strip_schema_metadata(&mut base);

    if base.is_empty() {
        Ok(None)
    } else {
        Ok(Some(build_schema_ast_from_value(&Value::Object(base))?))
    }
}

fn parse_string_schema(obj: &Map<String, Value>) -> SchemaNode {
    let min_length = Some(obj.get("minLength").and_then(|v| v.as_u64()).unwrap_or(0));
    let max_length = obj.get("maxLength").and_then(|v| v.as_u64());
    let pattern = obj
        .get("pattern")
        .and_then(|v| v.as_str())
        .map(|s| s.to_owned());
    let format = obj
        .get("format")
        .and_then(|v| v.as_str())
        .map(|s| s.to_owned());
    let enumeration = obj.get("enum").and_then(|v| v.as_array()).cloned();

    SchemaNode::new(SchemaNodeKind::String {
        min_length,
        max_length,
        pattern,
        format,
        enumeration,
    })
}

fn dedupe_schema_nodes(nodes: Vec<SchemaNode>) -> Vec<SchemaNode> {
    let mut unique = Vec::new();
    for node in nodes {
        if unique.iter().all(|existing| existing != &node) {
            unique.push(node);
        }
    }
    unique
}

fn parse_number_schema(obj: &Map<String, Value>, integer: bool) -> SchemaNode {
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
        .filter(|m| *m > 0.0)
        .or_else(|| integer.then_some(1.0));

    if integer {
        let min_i = minimum.map(|m| m as i64);
        let max_i = maximum.map(|m| m as i64);
        SchemaNode::new(SchemaNodeKind::Integer {
            minimum: min_i,
            maximum: max_i,
            exclusive_minimum,
            exclusive_maximum,
            multiple_of,
            enumeration,
        })
    } else {
        SchemaNode::new(SchemaNodeKind::Number {
            minimum,
            maximum,
            exclusive_minimum,
            exclusive_maximum,
            multiple_of,
            enumeration,
        })
    }
}

fn parse_boolean_schema(obj: &Map<String, Value>) -> SchemaNode {
    let enumeration = obj.get("enum").and_then(|v| v.as_array()).cloned();
    SchemaNode::new(SchemaNodeKind::Boolean { enumeration })
}

fn parse_null_schema(obj: &Map<String, Value>) -> SchemaNode {
    let enumeration = obj.get("enum").and_then(|v| v.as_array()).cloned();
    SchemaNode::new(SchemaNodeKind::Null { enumeration })
}

fn parse_object_schema(obj: &Map<String, Value>) -> Result<SchemaNode> {
    let mut properties = HashMap::new();
    if let Some(Value::Object(props)) = obj.get("properties") {
        for (k, v) in props {
            properties.insert(k.clone(), build_schema_ast_from_value(v)?);
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
        Some(other) => build_schema_ast_from_value(other)?,
    };

    let property_names = match obj.get("propertyNames") {
        None => SchemaNode::any(),
        Some(Value::Bool(true)) => SchemaNode::any(),
        Some(Value::Bool(false)) => SchemaNode::bool_schema(false),
        Some(other) => build_schema_ast_from_value(other)?,
    };

    let min_properties = obj
        .get("minProperties")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .or(Some(required.len()));
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

fn parse_array_schema(obj: &Map<String, Value>) -> Result<SchemaNode> {
    let items_node = match obj.get("items") {
        None => SchemaNode::any(),
        Some(Value::Bool(true)) => SchemaNode::any(),
        Some(Value::Bool(false)) => SchemaNode::bool_schema(false),
        Some(Value::Array(arr)) => {
            if arr.is_empty() {
                SchemaNode::any()
            } else if arr.len() == 1 {
                build_schema_ast_from_value(&arr[0])?
            } else {
                let subnodes = arr
                    .iter()
                    .map(build_schema_ast_from_value)
                    .collect::<Result<Vec<SchemaNode>>>()?;
                SchemaNode::new(SchemaNodeKind::AllOf(subnodes))
            }
        }
        Some(other) => build_schema_ast_from_value(other)?,
    };
    let min_items = obj.get("minItems").and_then(|v| v.as_u64()).or_else(|| {
        if obj.contains_key("contains") {
            obj.get("minContains")
                .and_then(|value| value.as_u64())
                .or(Some(1))
        } else {
            Some(0)
        }
    });
    let max_items = obj.get("maxItems").and_then(|v| v.as_u64());
    let enumeration = obj.get("enum").and_then(|v| v.as_array()).cloned();

    let contains_node = match obj.get("contains") {
        None => None,
        Some(v) => Some(build_schema_ast_from_value(v)?),
    };
    let min_contains = if contains_node.is_some() {
        obj.get("minContains")
            .and_then(|value| value.as_u64())
            .or(Some(1))
    } else {
        None
    };

    Ok(SchemaNode::new(SchemaNodeKind::Array {
        items: items_node,
        min_items,
        max_items,
        contains: contains_node,
        min_contains,
        enumeration,
    }))
}

fn resolve_refs_from_canonical_root(
    node: &mut SchemaNode,
    root_json: &CanonicalSchema,
    visited: &[String],
) -> Result<()> {
    let mut stack = visited.to_vec();
    let mut cache: HashMap<String, SchemaNode> = HashMap::new();
    resolve_refs_internal(node, root_json.as_value(), &mut stack, &mut cache)
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
                if let Some(next) = resolve_json_pointer_child(current, p) {
                    current = next;
                } else {
                    return Err(AstError::UnresolvedReference {
                        ref_path: path.clone(),
                    });
                }
            }
            let mut resolved = build_schema_ast_from_value(current)?;
            cache.insert(path.clone(), resolved.clone());
            stack.push(path.clone());
            resolve_refs_internal(&mut resolved, root_json, stack, cache)?;
            stack.pop();
            cache.insert(path.clone(), resolved.clone());
            *node = resolved;
        } else {
            *node.borrow_mut() = SchemaNodeKind::Any;
        }
        return Ok(());
    }

    if matches!(&*node.borrow(), SchemaNodeKind::AllOf(_)) {
        let mut children = {
            let guard = node.borrow();
            match &*guard {
                SchemaNodeKind::AllOf(children) => children.clone(),
                _ => unreachable!("node kind checked above"),
            }
        };
        for child in children.iter_mut() {
            resolve_refs_internal(child, root_json, stack, cache)?;
        }
        *node.borrow_mut() = SchemaNodeKind::AllOf(children);
        normalize_resolved_node(node);
        return Ok(());
    }
    if matches!(&*node.borrow(), SchemaNodeKind::AnyOf(_)) {
        let mut children = {
            let guard = node.borrow();
            match &*guard {
                SchemaNodeKind::AnyOf(children) => children.clone(),
                _ => unreachable!("node kind checked above"),
            }
        };
        for child in children.iter_mut() {
            resolve_refs_internal(child, root_json, stack, cache)?;
        }
        *node.borrow_mut() = SchemaNodeKind::AnyOf(children);
        normalize_resolved_node(node);
        return Ok(());
    }
    if matches!(&*node.borrow(), SchemaNodeKind::OneOf(_)) {
        let mut children = {
            let guard = node.borrow();
            match &*guard {
                SchemaNodeKind::OneOf(children) => children.clone(),
                _ => unreachable!("node kind checked above"),
            }
        };
        for child in children.iter_mut() {
            resolve_refs_internal(child, root_json, stack, cache)?;
        }
        *node.borrow_mut() = SchemaNodeKind::OneOf(children);
        normalize_resolved_node(node);
        return Ok(());
    }
    if matches!(&*node.borrow(), SchemaNodeKind::IfThenElse { .. }) {
        let (mut if_schema, mut then_schema, mut else_schema) = {
            let guard = node.borrow();
            match &*guard {
                SchemaNodeKind::IfThenElse {
                    if_schema,
                    then_schema,
                    else_schema,
                } => (if_schema.clone(), then_schema.clone(), else_schema.clone()),
                _ => unreachable!("node kind checked above"),
            }
        };

        resolve_refs_internal(&mut if_schema, root_json, stack, cache)?;
        if let Some(t) = &mut then_schema {
            resolve_refs_internal(t, root_json, stack, cache)?;
        }
        if let Some(e) = &mut else_schema {
            resolve_refs_internal(e, root_json, stack, cache)?;
        }
        *node.borrow_mut() = SchemaNodeKind::IfThenElse {
            if_schema,
            then_schema,
            else_schema,
        };
        normalize_resolved_node(node);
        return Ok(());
    }
    if matches!(&*node.borrow(), SchemaNodeKind::Not(_)) {
        let mut sub = {
            let guard = node.borrow();
            match &*guard {
                SchemaNodeKind::Not(sub) => sub.clone(),
                _ => unreachable!("node kind checked above"),
            }
        };
        resolve_refs_internal(&mut sub, root_json, stack, cache)?;
        *node.borrow_mut() = SchemaNodeKind::Not(sub);
        normalize_resolved_node(node);
        return Ok(());
    }
    if matches!(&*node.borrow(), SchemaNodeKind::Object { .. }) {
        let (
            mut properties,
            required,
            mut additional,
            mut property_names,
            min_properties,
            max_properties,
            dependent_required,
            enumeration,
        ) = {
            let guard = node.borrow();
            match &*guard {
                SchemaNodeKind::Object {
                    properties,
                    required,
                    additional,
                    property_names,
                    min_properties,
                    max_properties,
                    dependent_required,
                    enumeration,
                } => (
                    properties.clone(),
                    required.clone(),
                    additional.clone(),
                    property_names.clone(),
                    *min_properties,
                    *max_properties,
                    dependent_required.clone(),
                    enumeration.clone(),
                ),
                _ => unreachable!("node kind checked above"),
            }
        };

        for child in properties.values_mut() {
            resolve_refs_internal(child, root_json, stack, cache)?;
        }
        resolve_refs_internal(&mut additional, root_json, stack, cache)?;
        resolve_refs_internal(&mut property_names, root_json, stack, cache)?;
        *node.borrow_mut() = SchemaNodeKind::Object {
            properties,
            required,
            additional,
            property_names,
            min_properties,
            max_properties,
            dependent_required,
            enumeration,
        };
        return Ok(());
    }
    if matches!(&*node.borrow(), SchemaNodeKind::Array { .. }) {
        let (mut items, min_items, max_items, mut contains, min_contains, enumeration) = {
            let guard = node.borrow();
            match &*guard {
                SchemaNodeKind::Array {
                    items,
                    min_items,
                    max_items,
                    contains,
                    min_contains,
                    enumeration,
                } => (
                    items.clone(),
                    *min_items,
                    *max_items,
                    contains.clone(),
                    *min_contains,
                    enumeration.clone(),
                ),
                _ => unreachable!("node kind checked above"),
            }
        };

        resolve_refs_internal(&mut items, root_json, stack, cache)?;
        if let Some(child) = &mut contains {
            resolve_refs_internal(child, root_json, stack, cache)?;
        }
        *node.borrow_mut() = SchemaNodeKind::Array {
            items,
            min_items,
            max_items,
            contains,
            min_contains,
            enumeration,
        };
        return Ok(());
    }
    if matches!(&*node.borrow(), SchemaNodeKind::AdditionalProperties(_)) {
        let mut schema = {
            let guard = node.borrow();
            match &*guard {
                SchemaNodeKind::AdditionalProperties(schema) => schema.clone(),
                _ => unreachable!("node kind checked above"),
            }
        };
        resolve_refs_internal(&mut schema, root_json, stack, cache)?;
        *node.borrow_mut() = SchemaNodeKind::AdditionalProperties(schema);
        return Ok(());
    }
    if matches!(&*node.borrow(), SchemaNodeKind::Defs(_)) {
        let mut map = {
            let guard = node.borrow();
            match &*guard {
                SchemaNodeKind::Defs(map) => map.clone(),
                _ => unreachable!("node kind checked above"),
            }
        };
        for child in map.values_mut() {
            resolve_refs_internal(child, root_json, stack, cache)?;
        }
        *node.borrow_mut() = SchemaNodeKind::Defs(map);
        return Ok(());
    }

    Ok(())
}

fn resolve_json_pointer_child<'a>(current: &'a Value, token: &str) -> Option<&'a Value> {
    match current {
        Value::Object(object) => object.get(token),
        Value::Array(items) => token
            .parse::<usize>()
            .ok()
            .and_then(|index| items.get(index)),
        _ => None,
    }
}

fn normalize_resolved_node(node: &mut SchemaNode) {
    let current = node.borrow().clone();
    let replacement = match current {
        SchemaNodeKind::AllOf(mut children) => {
            if children.iter().any(is_false_schema) {
                Some(SchemaNodeKind::BoolSchema(false))
            } else {
                children.retain(|child| !is_any_schema(child));
                children = dedupe_schema_nodes(children);
                collapse_applicator_children(&children, true)
                    .or(Some(SchemaNodeKind::AllOf(children)))
            }
        }
        SchemaNodeKind::AnyOf(mut children) => {
            if children.iter().any(is_any_schema) {
                Some(SchemaNodeKind::Any)
            } else {
                children.retain(|child| !is_false_schema(child));
                children = dedupe_schema_nodes(children);
                collapse_applicator_children(&children, false)
                    .or(Some(SchemaNodeKind::AnyOf(children)))
            }
        }
        SchemaNodeKind::OneOf(children) => {
            collapse_applicator_children(&children, false).or(Some(SchemaNodeKind::OneOf(children)))
        }
        SchemaNodeKind::Not(sub) => {
            if is_false_schema(&sub) {
                Some(SchemaNodeKind::Any)
            } else if is_any_schema(&sub) {
                Some(SchemaNodeKind::BoolSchema(false))
            } else {
                Some(SchemaNodeKind::Not(sub))
            }
        }
        SchemaNodeKind::IfThenElse {
            if_schema,
            mut then_schema,
            mut else_schema,
        } => {
            if then_schema.as_ref().is_some_and(is_any_schema) {
                then_schema = None;
            }
            if else_schema.as_ref().is_some_and(is_any_schema) {
                else_schema = None;
            }
            if then_schema.is_none() && else_schema.is_none() {
                Some(SchemaNodeKind::Any)
            } else {
                Some(SchemaNodeKind::IfThenElse {
                    if_schema,
                    then_schema,
                    else_schema,
                })
            }
        }
        _ => None,
    };

    if let Some(kind) = replacement {
        *node.borrow_mut() = kind;
    }
}

fn collapse_applicator_children(
    children: &[SchemaNode],
    empty_is_any: bool,
) -> Option<SchemaNodeKind> {
    match children.len() {
        0 => Some(if empty_is_any {
            SchemaNodeKind::Any
        } else {
            SchemaNodeKind::BoolSchema(false)
        }),
        1 => Some(children[0].borrow().clone()),
        _ => None,
    }
}

fn is_any_schema(node: &SchemaNode) -> bool {
    matches!(
        &*node.borrow(),
        SchemaNodeKind::Any | SchemaNodeKind::BoolSchema(true)
    )
}

fn is_false_schema(node: &SchemaNode) -> bool {
    matches!(&*node.borrow(), SchemaNodeKind::BoolSchema(false))
}
