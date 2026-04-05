//! Resolved JSON Schema document and typed semantic IR.
//!
//! This module owns the canonicalized schema graph consumed by compatibility
//! checks and fuzz generation.  The parser still uses a private mutable graph
//! while resolving local `$ref`s, but the public `SchemaNodeKind` IR only
//! exposes post-resolution semantic variants with normalized constraint types.

use crate::canonicalize::{canonicalize_schema, json_type_name};
use crate::constraints::{
    ContainsConstraint, CountRange, IntegerBounds, NumberBound, NumberBounds, PatternConstraint,
    PatternProperty,
};
use crate::json_semantics::{integer_value_from_json, json_values_equal, numeric_values_equal};
use crate::schema_metadata::{is_schema_metadata_key, strip_schema_metadata};
use crate::{CompileError, JSONSchema, SchemaError, compile};
use percent_encoding::percent_decode_str;
use serde_json::{Map, Value};
use std::cell::OnceCell;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::Hash;
use std::num::NonZeroI64;
use std::rc::Rc;

type Result<T> = std::result::Result<T, AstError>;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AstError {
    /// The raw schema document failed dialect or keyword-shape validation.
    #[error(transparent)]
    Schema(#[from] SchemaError),
    /// The backend validator rejected the original raw schema document.
    #[error("failed to compile raw schema validator: {source}")]
    RawValidator {
        #[source]
        source: CompileError,
    },
    #[cfg(test)]
    #[error("failed to compile canonicalized schema validator: {source}")]
    CanonicalizedValidator {
        #[source]
        source: CompileError,
    },
    /// A local JSON Pointer `$ref` did not point to a schema node.
    #[error("local $ref '{ref_path}' does not resolve to a schema node in the current document")]
    UnresolvedReference { ref_path: String },
    /// A local `$ref` chain cycles without ever reaching a concrete schema.
    #[error("local $ref '{ref_path}' forms an alias-only cycle with no concrete schema node")]
    CyclicReferenceAlias { ref_path: String },
    /// Reference resolution is intentionally limited to same-document JSON Pointers.
    #[error(
        "unsupported reference '{ref_path}': only local JSON Pointer $ref targets of the form '#/...'"
    )]
    UnsupportedReference { ref_path: String },
}

/// Fully resolved JSON Schema document.
pub struct SchemaDocument {
    raw: Value,
    root: OnceCell<SchemaNode>,
    canonical: OnceCell<Value>,
    raw_validator: OnceCell<JSONSchema>,
    #[cfg(test)]
    canonical_validator: OnceCell<JSONSchema>,
}

impl SchemaDocument {
    /// Build a resolved schema document from raw JSON Schema.
    ///
    /// The resolved graph is built from the canonicalized schema so
    /// compatibility analysis and generation can consume a deterministic IR,
    /// while `is_valid()` intentionally validates against a backend compiled
    /// from the original raw schema document.
    pub fn from_json(raw: &Value) -> Result<Self> {
        let canonical = canonicalize_schema(raw)?.as_value().clone();
        let schema = Self {
            raw: raw.clone(),
            root: OnceCell::new(),
            canonical: OnceCell::new(),
            raw_validator: OnceCell::new(),
            #[cfg(test)]
            canonical_validator: OnceCell::new(),
        };
        schema
            .canonical
            .set(canonical)
            .expect("canonical schema cache should be initialized exactly once");
        Ok(schema)
    }

    /// Return the lazily built resolved root node.
    ///
    /// This is the low-level extension point for crates that need the resolved
    /// IR, such as `jsoncompat` and `json_schema_fuzz`. Validation callers
    /// should prefer [`SchemaDocument::is_valid`].
    pub fn root(&self) -> Result<&SchemaNode> {
        get_or_try_init(&self.root, || {
            let canonical = self.canonical_schema_json()?;
            let mut graph = MutableSchemaGraph::default();
            let root = build_schema_ast_from_value(canonical, &mut graph)?;
            let root = resolve_refs_internal(
                root,
                &mut graph,
                canonical,
                &mut Vec::new(),
                &mut HashMap::new(),
            )?;
            Ok(freeze_schema_node(root, &graph, &mut HashMap::new()))
        })
    }

    /// Return the canonicalized JSON Schema document used to build `root()`.
    pub fn canonical_schema_json(&self) -> Result<&Value> {
        get_or_try_init(&self.canonical, || {
            Ok(canonicalize_schema(&self.raw)?.as_value().clone())
        })
    }

    /// Validate one instance against the backend compiled from the original
    /// raw schema document.
    pub fn is_valid(&self, value: &Value) -> Result<bool> {
        let validator = get_or_try_init(&self.raw_validator, || {
            compile(&self.raw).map_err(|source| AstError::RawValidator { source })
        })?;
        Ok(validator.is_valid(value))
    }

    #[cfg(test)]
    pub(crate) fn is_valid_canonicalized(&self, value: &Value) -> Result<bool> {
        let canonical = self.canonical_schema_json()?;
        let validator = get_or_try_init(&self.canonical_validator, || {
            compile(canonical).map_err(|source| AstError::CanonicalizedValidator { source })
        })?;
        Ok(validator.is_valid(value))
    }
}

fn get_or_try_init<T>(cell: &OnceCell<T>, init: impl FnOnce() -> Result<T>) -> Result<&T> {
    if let Some(value) = cell.get() {
        return Ok(value);
    }

    let value = init()?;
    let _ = cell.set(value);

    Ok(cell
        .get()
        .expect("lazy schema field must be initialized before returning"))
}

impl fmt::Debug for SchemaDocument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SchemaDocument")
            .field("raw", &self.raw)
            .field("canonical", &self.canonical.get())
            .field("root", &self.root.get())
            .finish()
    }
}

/// Shared immutable representation of a resolved JSON Schema node.
///
/// Reference counting allows multiple parents to point to the same node, which
/// is required to faithfully model schemas containing recursive `$ref`s.
#[derive(Clone)]
pub struct SchemaNode(Rc<OnceCell<SchemaNodeKind>>);

/// Stable identity for one in-memory schema node within the lifetime of the AST.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(usize);

/// Public schema-build error name used by `SchemaDocument::from_json`.
pub type SchemaBuildError = AstError;

impl SchemaNode {
    /// Return the resolved semantic variant for this node.
    ///
    /// The returned value is part of the canonical IR. It intentionally omits
    /// parser-only states such as `$ref`.
    pub fn kind(&self) -> &SchemaNodeKind {
        self.0
            .get()
            .expect("resolved SchemaNode must be initialized before use")
    }

    fn ptr_id(&self) -> usize {
        Rc::as_ptr(&self.0) as usize
    }

    /// Return a stable identity for this in-memory node.
    ///
    /// Identities are only meaningful within one resolved schema graph and are
    /// primarily used for cycle guards.
    #[must_use]
    pub fn id(&self) -> NodeId {
        NodeId(self.ptr_id())
    }

    /// Check whether one instance is accepted by this canonicalized AST node.
    ///
    /// This is a low-level evaluator for resolved subgraphs used by
    /// compatibility and generation crates. User-visible validation should go
    /// through [`SchemaDocument::is_valid`], which uses the `jsonschema`
    /// backend compiled from the original raw schema document.
    #[must_use]
    pub fn accepts_value(&self, value: &Value) -> bool {
        self.accepts_value_inner(value, &mut HashSet::new())
    }

    fn accepts_value_inner(
        &self,
        value: &Value,
        active: &mut HashSet<RecursiveValidationFrame>,
    ) -> bool {
        let frame = RecursiveValidationFrame {
            schema_id: self.id(),
            value_address: std::ptr::from_ref(value) as usize,
        };
        if !active.insert(frame) {
            // Re-entering the same schema on the same JSON value is a
            // non-productive cycle (`A = anyOf(string, A)` on `[]`). Fail
            // closed here while still descending through child instance values
            // at distinct addresses.
            return false;
        }

        let is_valid = match self.kind() {
            SchemaNodeKind::BoolSchema(valid) => *valid,
            SchemaNodeKind::Any => true,
            SchemaNodeKind::String {
                length,
                pattern,
                enumeration,
                ..
            } => value.as_str().is_some_and(|string_value| {
                string_length_in_range(string_value, *length)
                    && pattern
                        .as_ref()
                        .is_none_or(|pattern| pattern.is_match(string_value))
                    && enum_contains_value(
                        enumeration.as_deref(),
                        &Value::String(string_value.to_owned()),
                    )
            }),
            SchemaNodeKind::Number {
                bounds,
                multiple_of,
                enumeration,
            } => value.as_f64().is_some_and(|number_value| {
                bounds.contains(number_value)
                    && value_is_multiple_of(
                        number_value,
                        multiple_of.as_ref().map(|multiple_of| multiple_of.as_f64()),
                    )
                    && enum_contains_numeric_value(enumeration.as_deref(), value)
            }),
            SchemaNodeKind::Integer {
                bounds,
                multiple_of,
                enumeration,
            } => integer_value_from_json(value).map_or_else(
                || {
                    value.as_f64().is_some_and(|number_value| {
                        number_value.fract() == 0.0
                            && bounds.as_number_bounds().contains(number_value)
                            && value_is_multiple_of(
                                number_value,
                                multiple_of.as_ref().map(|multiple_of| multiple_of.as_f64()),
                            )
                            && enum_contains_numeric_value(enumeration.as_deref(), value)
                    })
                },
                |integer_value| {
                    bounds.contains_i128(integer_value)
                        && integer_value_is_multiple_of(integer_value, multiple_of.as_ref())
                        && enum_contains_numeric_value(enumeration.as_deref(), value)
                },
            ),
            SchemaNodeKind::Boolean { enumeration } => value
                .as_bool()
                .is_some_and(|_| enum_contains_value(enumeration.as_deref(), value)),
            SchemaNodeKind::Null { enumeration } => {
                value.is_null() && enum_contains_value(enumeration.as_deref(), value)
            }
            SchemaNodeKind::Object {
                properties,
                pattern_properties,
                required,
                additional,
                property_names,
                property_count,
                dependent_required,
                enumeration,
            } => value.as_object().is_some_and(|object_value| {
                enum_contains_value(enumeration.as_deref(), value)
                    && property_count.contains(object_value.len())
                    && required.iter().all(|name| object_value.contains_key(name))
                    && dependent_required.iter().all(|(trigger, dependencies)| {
                        !object_value.contains_key(trigger)
                            || dependencies
                                .iter()
                                .all(|dependency| object_value.contains_key(dependency))
                    })
                    && object_value.iter().all(|(property_name, property_value)| {
                        let property_name_value = Value::String(property_name.clone());
                        property_names.accepts_value_inner(&property_name_value, active)
                            && object_property_is_valid(
                                properties,
                                pattern_properties,
                                additional,
                                property_name,
                                property_value,
                                active,
                            )
                    })
            }),
            SchemaNodeKind::Array {
                prefix_items,
                items,
                item_count,
                contains,
                unique_items,
                enumeration,
            } => value.as_array().is_some_and(|array_value| {
                enum_contains_value(enumeration.as_deref(), value)
                    && item_count.contains(array_value.len() as u64)
                    && (!unique_items || array_values_are_unique(array_value))
                    && array_value.iter().enumerate().all(|(index, item)| {
                        let item_schema = prefix_items.get(index).unwrap_or(items);
                        item_schema.accepts_value_inner(item, active)
                    })
                    && contains.as_ref().is_none_or(|contains| {
                        let matching_items = array_value
                            .iter()
                            .filter(|item| contains.schema.accepts_value_inner(item, active))
                            .count() as u64;
                        contains.count().contains(matching_items)
                    })
            }),
            SchemaNodeKind::AllOf(children) => children
                .iter()
                .all(|child| child.accepts_value_inner(value, active)),
            SchemaNodeKind::AnyOf(children) => children
                .iter()
                .any(|child| child.accepts_value_inner(value, active)),
            SchemaNodeKind::OneOf(children) => {
                children
                    .iter()
                    .filter(|child| child.accepts_value_inner(value, active))
                    .count()
                    == 1
            }
            SchemaNodeKind::Not(child) => !child.accepts_value_inner(value, active),
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => {
                if if_schema.accepts_value_inner(value, active) {
                    then_schema
                        .as_ref()
                        .is_none_or(|then_schema| then_schema.accepts_value_inner(value, active))
                } else {
                    else_schema
                        .as_ref()
                        .is_none_or(|else_schema| else_schema.accepts_value_inner(value, active))
                }
            }
            SchemaNodeKind::Const(expected) => json_values_equal(expected, value),
            SchemaNodeKind::Enum(values) => values
                .iter()
                .any(|expected| json_values_equal(expected, value)),
        };

        active.remove(&frame);
        is_valid
    }

    #[cfg(test)]
    pub(crate) fn has_cycle(&self) -> bool {
        fn visit(
            node: &SchemaNode,
            active: &mut HashSet<usize>,
            seen: &mut HashSet<usize>,
        ) -> bool {
            let id = node.ptr_id();
            if active.contains(&id) {
                return true;
            }
            if !seen.insert(id) {
                return false;
            }

            active.insert(id);
            let children = {
                use SchemaNodeKind::*;

                match node.kind() {
                    AllOf(children) | AnyOf(children) | OneOf(children) => children.clone(),
                    Not(child) => vec![child.clone()],
                    IfThenElse {
                        if_schema,
                        then_schema,
                        else_schema,
                    } => {
                        let mut children = vec![if_schema.clone()];
                        if let Some(child) = then_schema {
                            children.push(child.clone());
                        }
                        if let Some(child) = else_schema {
                            children.push(child.clone());
                        }
                        children
                    }
                    Object {
                        properties,
                        pattern_properties,
                        additional,
                        property_names,
                        ..
                    } => properties
                        .values()
                        .cloned()
                        .chain(
                            pattern_properties
                                .values()
                                .map(|pattern_property| pattern_property.schema.clone()),
                        )
                        .chain(std::iter::once(additional.clone()))
                        .chain(std::iter::once(property_names.clone()))
                        .collect(),
                    Array {
                        prefix_items,
                        items,
                        contains,
                        ..
                    } => prefix_items
                        .iter()
                        .cloned()
                        .chain(std::iter::once(items.clone()))
                        .chain(contains.iter().map(|contains| contains.schema.clone()))
                        .collect(),
                    BoolSchema(_)
                    | Any
                    | String { .. }
                    | Number { .. }
                    | Integer { .. }
                    | Boolean { .. }
                    | Null { .. }
                    | Const(_)
                    | Enum(_) => Vec::new(),
                }
            };

            let has_cycle = children.iter().any(|child| visit(child, active, seen));
            active.remove(&id);
            has_cycle
        }

        visit(self, &mut HashSet::new(), &mut HashSet::new())
    }

    /// Convert the AST node back into a *minimal* JSON representation.  This
    /// is **lossy** for complex scenarios but is sufficient for the validator
    /// tests and fuzz harness (which only relies on the subset of keywords we
    /// explicitly generate).
    #[cfg(test)]
    pub(crate) fn to_json(&self) -> Value {
        use SchemaNodeKind::*;

        match self.kind() {
            BoolSchema(b) => Value::Bool(*b),
            Any => Value::Object(serde_json::Map::new()),

            Enum(values) => {
                let mut obj = serde_json::Map::new();
                obj.insert("enum".into(), Value::Array(values.clone()));
                Value::Object(obj)
            }

            String {
                length,
                pattern,
                format,
                enumeration,
            } => {
                let mut obj = serde_json::Map::new();
                obj.insert("type".into(), Value::String("string".into()));
                if length.min() != 0 {
                    obj.insert("minLength".into(), Value::Number(length.min().into()));
                }
                if let Some(m) = length.max() {
                    obj.insert("maxLength".into(), Value::Number(m.into()));
                }
                if let Some(p) = pattern {
                    obj.insert("pattern".into(), Value::String(p.as_str().to_owned()));
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
                bounds,
                multiple_of,
                enumeration,
            } => {
                let mut obj = serde_json::Map::new();
                obj.insert("type".into(), Value::String("number".into()));
                write_number_bounds(&mut obj, *bounds);
                if let Some(mo) = multiple_of {
                    obj.insert("multipleOf".into(), Value::Number(mo.to_json_number()));
                }
                if let Some(e) = enumeration {
                    obj.insert("enum".into(), Value::Array(e.clone()));
                }
                Value::Object(obj)
            }

            Integer {
                bounds,
                multiple_of,
                enumeration,
            } => {
                let mut obj = serde_json::Map::new();
                obj.insert("type".into(), Value::String("integer".into()));
                write_integer_bounds(&mut obj, *bounds);
                if let Some(e) = enumeration {
                    obj.insert("enum".into(), Value::Array(e.clone()));
                }
                if let Some(mo) = multiple_of {
                    obj.insert("multipleOf".into(), Value::Number(mo.to_json_number()));
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
                prefix_items,
                items,
                item_count,
                contains,
                unique_items,
                enumeration,
            } => {
                let mut obj = serde_json::Map::new();
                obj.insert("type".into(), Value::String("array".into()));
                if !prefix_items.is_empty() {
                    obj.insert(
                        "prefixItems".into(),
                        Value::Array(prefix_items.iter().map(SchemaNode::to_json).collect()),
                    );
                }
                if !matches!(items.kind(), SchemaNodeKind::Any) {
                    obj.insert("items".into(), items.to_json());
                }
                if item_count.min() != 0 {
                    obj.insert("minItems".into(), Value::Number(item_count.min().into()));
                }
                if let Some(ma) = item_count.max() {
                    obj.insert("maxItems".into(), Value::Number(ma.into()));
                }
                if let Some(contains) = contains {
                    obj.insert("contains".into(), contains.schema.to_json());
                    if contains.count().min() != 1 {
                        obj.insert(
                            "minContains".into(),
                            Value::Number(contains.count().min().into()),
                        );
                    }
                    if let Some(max_contains) = contains.count().max() {
                        obj.insert("maxContains".into(), Value::Number(max_contains.into()));
                    }
                }
                if *unique_items {
                    obj.insert("uniqueItems".into(), Value::Bool(true));
                }
                if let Some(e) = enumeration {
                    obj.insert("enum".into(), Value::Array(e.clone()));
                }
                Value::Object(obj)
            }

            Object {
                properties,
                pattern_properties,
                required,
                additional,
                property_names,
                property_count,
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

                if !pattern_properties.is_empty() {
                    let mut props_map = serde_json::Map::new();
                    for (pattern, pattern_property) in pattern_properties {
                        props_map.insert(pattern.clone(), pattern_property.schema.to_json());
                    }
                    obj.insert("patternProperties".into(), Value::Object(props_map));
                }

                if !required.is_empty() {
                    let mut sorted: Vec<_> = required.iter().cloned().collect();
                    sorted.sort();
                    obj.insert(
                        "required".into(),
                        Value::Array(sorted.into_iter().map(Value::String).collect()),
                    );
                }

                match additional.kind() {
                    SchemaNodeKind::Any => {}
                    SchemaNodeKind::BoolSchema(b) => {
                        obj.insert("additionalProperties".into(), Value::Bool(*b));
                    }
                    _ => {
                        obj.insert("additionalProperties".into(), additional.to_json());
                    }
                }

                match property_names.kind() {
                    SchemaNodeKind::Any | SchemaNodeKind::BoolSchema(true) => {}
                    SchemaNodeKind::BoolSchema(b) => {
                        obj.insert("propertyNames".into(), Value::Bool(*b));
                    }
                    _ => {
                        obj.insert("propertyNames".into(), property_names.to_json());
                    }
                }

                if property_count.min() != required.len() {
                    obj.insert(
                        "minProperties".into(),
                        Value::Number(property_count.min().into()),
                    );
                }
                if let Some(mp) = property_count.max() {
                    obj.insert("maxProperties".into(), Value::Number(mp.into()));
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

            Const(v) => {
                let mut obj = serde_json::Map::new();
                obj.insert("const".into(), v.clone());
                Value::Object(obj)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct RecursiveValidationFrame {
    schema_id: NodeId,
    value_address: usize,
}

fn object_property_is_valid(
    properties: &HashMap<String, SchemaNode>,
    pattern_properties: &HashMap<String, PatternProperty<SchemaNode>>,
    additional: &SchemaNode,
    property_name: &str,
    property_value: &Value,
    active: &mut HashSet<RecursiveValidationFrame>,
) -> bool {
    let mut matched = false;

    if let Some(property_schema) = properties.get(property_name) {
        matched = true;
        if !property_schema.accepts_value_inner(property_value, active) {
            return false;
        }
    }

    for pattern_property in pattern_properties.values() {
        if !pattern_property.pattern.is_match(property_name) {
            continue;
        }
        matched = true;
        if !pattern_property
            .schema
            .accepts_value_inner(property_value, active)
        {
            return false;
        }
    }

    matched || additional.accepts_value_inner(property_value, active)
}

fn string_length_in_range(value: &str, length: CountRange<u64>) -> bool {
    let character_count = value.chars().count() as u64;
    length.contains(character_count)
}

#[cfg(test)]
fn write_number_bounds(obj: &mut Map<String, Value>, bounds: NumberBounds) {
    match bounds.lower() {
        NumberBound::Unbounded => {}
        NumberBound::Inclusive(value) => {
            obj.insert(
                "minimum".into(),
                Value::Number(
                    serde_json::Number::from_f64(value).expect("finite lower bound number"),
                ),
            );
        }
        NumberBound::Exclusive(value) => {
            obj.insert(
                "exclusiveMinimum".into(),
                Value::Number(
                    serde_json::Number::from_f64(value).expect("finite lower bound number"),
                ),
            );
        }
    }

    match bounds.upper() {
        NumberBound::Unbounded => {}
        NumberBound::Inclusive(value) => {
            obj.insert(
                "maximum".into(),
                Value::Number(
                    serde_json::Number::from_f64(value).expect("finite upper bound number"),
                ),
            );
        }
        NumberBound::Exclusive(value) => {
            obj.insert(
                "exclusiveMaximum".into(),
                Value::Number(
                    serde_json::Number::from_f64(value).expect("finite upper bound number"),
                ),
            );
        }
    }
}

#[cfg(test)]
fn write_integer_bounds(obj: &mut Map<String, Value>, bounds: IntegerBounds) {
    if let Some(value) = bounds.lower() {
        obj.insert("minimum".into(), Value::Number(value.into()));
    }

    if let Some(value) = bounds.upper() {
        obj.insert("maximum".into(), Value::Number(value.into()));
    }
}

fn enum_contains_value(enumeration: Option<&[Value]>, value: &Value) -> bool {
    enumeration.is_none_or(|enumeration| {
        enumeration
            .iter()
            .any(|expected| json_values_equal(expected, value))
    })
}

fn enum_contains_numeric_value(enumeration: Option<&[Value]>, value: &Value) -> bool {
    enumeration.is_none_or(|enumeration| {
        enumeration
            .iter()
            .any(|expected| numeric_values_equal(expected, value))
    })
}

fn value_is_multiple_of(value: f64, multiple_of: Option<f64>) -> bool {
    let Some(multiple_of) = multiple_of else {
        return true;
    };
    if multiple_of <= 0.0 {
        return false;
    }
    if let (Some(value), Some(multiple_of)) = (
        exact_positive_integer(value.abs()),
        exact_positive_integer(multiple_of),
    ) {
        return value % multiple_of == 0;
    }

    let ratio = value / multiple_of;
    (ratio - ratio.round()).abs() <= f64::EPSILON * ratio.abs().max(1.0) * 4.0
}

fn integer_value_is_multiple_of(value: i128, multiple_of: Option<&IntegerMultipleOf>) -> bool {
    let Some(multiple_of) = multiple_of else {
        return true;
    };
    let Some(divisor) = multiple_of.integer_divisor() else {
        return value_is_multiple_of(value as f64, Some(multiple_of.as_f64()));
    };
    value.rem_euclid(divisor) == 0
}

fn parse_integer_value(value: Option<&Value>) -> Option<i64> {
    value
        .and_then(integer_value_from_json)
        .and_then(|integer| i64::try_from(integer).ok())
}

fn parse_integer_multiple_of(value: Option<&Value>) -> Option<IntegerMultipleOf> {
    let value = value?;
    if let Some(integer) = parse_integer_value(Some(value)) {
        return IntegerMultipleOf::from_integer(integer);
    }

    IntegerMultipleOf::from_number(value.as_f64()?)
}

fn decimal_number_integer_divisor(value: f64) -> Option<i128> {
    if !value.is_finite() || value <= 0.0 {
        return None;
    }

    let text = value.to_string();
    let (mantissa, exponent) = if let Some((mantissa, exponent)) = text.split_once(['e', 'E']) {
        (mantissa, exponent.parse::<i32>().ok()?)
    } else {
        (text.as_str(), 0)
    };

    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    if whole.starts_with('-')
        || whole.starts_with('+')
        || !whole.chars().all(|character| character.is_ascii_digit())
        || !fraction.chars().all(|character| character.is_ascii_digit())
    {
        return None;
    }

    let mut numerator = parse_decimal_digits(whole)?;
    numerator = fraction.chars().try_fold(numerator, |numerator, digit| {
        numerator
            .checked_mul(10)?
            .checked_add(i128::from(digit.to_digit(10)?))
    })?;

    if numerator == 0 {
        return None;
    }

    let scale = i32::try_from(fraction.len()).ok()?.checked_sub(exponent)?;

    if scale <= 0 {
        return numerator.checked_mul(checked_pow10(scale.unsigned_abs())?);
    }

    let denominator = checked_pow10(scale.unsigned_abs())?;
    Some(numerator / gcd_i128(numerator, denominator))
}

fn parse_decimal_digits(value: &str) -> Option<i128> {
    if value.is_empty() {
        return Some(0);
    }

    value.chars().try_fold(0_i128, |accumulator, digit| {
        accumulator
            .checked_mul(10)?
            .checked_add(i128::from(digit.to_digit(10)?))
    })
}

fn checked_pow10(exponent: u32) -> Option<i128> {
    (0..exponent).try_fold(1_i128, |value, _| value.checked_mul(10))
}

fn gcd_i128(mut left: i128, mut right: i128) -> i128 {
    while right != 0 {
        let remainder = left.rem_euclid(right);
        left = right;
        right = remainder;
    }
    left
}

fn exact_positive_integer(value: f64) -> Option<u64> {
    if !value.is_finite() || value <= 0.0 || value.fract() != 0.0 || value > u64::MAX as f64 {
        return None;
    }

    let integer = value as u64;
    ((integer as f64) == value).then_some(integer)
}

fn array_values_are_unique(values: &[Value]) -> bool {
    values.iter().enumerate().all(|(index, value)| {
        values[..index]
            .iter()
            .all(|seen| !json_values_equal(seen, value))
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct MutableSchemaNode(usize);

#[derive(Debug, Default)]
struct MutableSchemaGraph {
    nodes: Vec<MutableSchemaNodeKind>,
}

impl MutableSchemaGraph {
    fn push(&mut self, kind: MutableSchemaNodeKind) -> MutableSchemaNode {
        let node = MutableSchemaNode(self.nodes.len());
        self.nodes.push(kind);
        node
    }

    fn bool_schema(&mut self, value: bool) -> MutableSchemaNode {
        self.push(MutableSchemaNodeKind::BoolSchema(value))
    }

    fn any(&mut self) -> MutableSchemaNode {
        self.push(MutableSchemaNodeKind::Any)
    }

    fn kind(&self, node: MutableSchemaNode) -> &MutableSchemaNodeKind {
        &self.nodes[node.0]
    }

    fn set_kind(&mut self, node: MutableSchemaNode, kind: MutableSchemaNodeKind) {
        self.nodes[node.0] = kind;
    }
}

trait SchemaNodeGraph: Sized {
    fn graph_ptr_id(&self) -> usize;

    fn with_graph_kind<R, F>(&self, read_kind: F) -> R
    where
        F: FnOnce(SchemaNodeKindView<'_, Self>) -> R;
}

#[derive(Clone, Copy)]
enum SchemaNodeKindView<'a, Node> {
    BoolSchema(bool),
    Any,
    String {
        length: CountRange<u64>,
        pattern: &'a Option<PatternConstraint>,
        format: &'a Option<String>,
        enumeration: &'a Option<Vec<Value>>,
    },
    Number {
        bounds: NumberBounds,
        multiple_of: &'a Option<NumberMultipleOf>,
        enumeration: &'a Option<Vec<Value>>,
    },
    Integer {
        bounds: IntegerBounds,
        multiple_of: &'a Option<IntegerMultipleOf>,
        enumeration: &'a Option<Vec<Value>>,
    },
    Boolean {
        enumeration: &'a Option<Vec<Value>>,
    },
    Null {
        enumeration: &'a Option<Vec<Value>>,
    },
    Object {
        properties: &'a HashMap<String, Node>,
        pattern_properties: &'a HashMap<String, PatternProperty<Node>>,
        required: &'a HashSet<String>,
        additional: &'a Node,
        property_names: &'a Node,
        property_count: CountRange<usize>,
        dependent_required: &'a HashMap<String, Vec<String>>,
        enumeration: &'a Option<Vec<Value>>,
    },
    Array {
        prefix_items: &'a [Node],
        items: &'a Node,
        item_count: CountRange<u64>,
        contains: Option<&'a ContainsConstraint<Node>>,
        unique_items: bool,
        enumeration: &'a Option<Vec<Value>>,
    },
    AllOf(&'a [Node]),
    AnyOf(&'a [Node]),
    OneOf(&'a [Node]),
    Not(&'a Node),
    IfThenElse {
        if_schema: &'a Node,
        then_schema: Option<&'a Node>,
        else_schema: Option<&'a Node>,
    },
    Const(&'a Value),
    Enum(&'a [Value]),
    Ref(&'a str),
}

impl SchemaNodeGraph for SchemaNode {
    fn graph_ptr_id(&self) -> usize {
        SchemaNode::ptr_id(self)
    }

    fn with_graph_kind<R, F>(&self, read_kind: F) -> R
    where
        F: FnOnce(SchemaNodeKindView<'_, Self>) -> R,
    {
        read_kind(self.kind().into())
    }
}

impl<'a> From<&'a MutableSchemaNodeKind> for SchemaNodeKindView<'a, MutableSchemaNode> {
    fn from(kind: &'a MutableSchemaNodeKind) -> Self {
        match kind {
            MutableSchemaNodeKind::BoolSchema(value) => Self::BoolSchema(*value),
            MutableSchemaNodeKind::Any => Self::Any,
            MutableSchemaNodeKind::String {
                length,
                pattern,
                format,
                enumeration,
            } => Self::String {
                length: *length,
                pattern,
                format,
                enumeration,
            },
            MutableSchemaNodeKind::Number {
                bounds,
                multiple_of,
                enumeration,
            } => Self::Number {
                bounds: *bounds,
                multiple_of,
                enumeration,
            },
            MutableSchemaNodeKind::Integer {
                bounds,
                multiple_of,
                enumeration,
            } => Self::Integer {
                bounds: *bounds,
                multiple_of,
                enumeration,
            },
            MutableSchemaNodeKind::Boolean { enumeration } => Self::Boolean { enumeration },
            MutableSchemaNodeKind::Null { enumeration } => Self::Null { enumeration },
            MutableSchemaNodeKind::Object {
                properties,
                pattern_properties,
                required,
                additional,
                property_names,
                property_count,
                dependent_required,
                enumeration,
            } => Self::Object {
                properties,
                pattern_properties,
                required,
                additional,
                property_names,
                property_count: *property_count,
                dependent_required,
                enumeration,
            },
            MutableSchemaNodeKind::Array {
                prefix_items,
                items,
                item_count,
                contains,
                unique_items,
                enumeration,
            } => Self::Array {
                prefix_items,
                items,
                item_count: *item_count,
                contains: contains.as_ref(),
                unique_items: *unique_items,
                enumeration,
            },
            MutableSchemaNodeKind::AllOf(children) => Self::AllOf(children),
            MutableSchemaNodeKind::AnyOf(children) => Self::AnyOf(children),
            MutableSchemaNodeKind::OneOf(children) => Self::OneOf(children),
            MutableSchemaNodeKind::Not(child) => Self::Not(child),
            MutableSchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => Self::IfThenElse {
                if_schema,
                then_schema: then_schema.as_ref(),
                else_schema: else_schema.as_ref(),
            },
            MutableSchemaNodeKind::Const(value) => Self::Const(value),
            MutableSchemaNodeKind::Enum(values) => Self::Enum(values),
            MutableSchemaNodeKind::Ref(ref_path) => Self::Ref(ref_path),
        }
    }
}

impl<'a, Node> From<&'a SchemaNodeKind<Node>> for SchemaNodeKindView<'a, Node> {
    fn from(kind: &'a SchemaNodeKind<Node>) -> Self {
        match kind {
            SchemaNodeKind::BoolSchema(value) => Self::BoolSchema(*value),
            SchemaNodeKind::Any => Self::Any,
            SchemaNodeKind::String {
                length,
                pattern,
                format,
                enumeration,
            } => Self::String {
                length: *length,
                pattern,
                format,
                enumeration,
            },
            SchemaNodeKind::Number {
                bounds,
                multiple_of,
                enumeration,
            } => Self::Number {
                bounds: *bounds,
                multiple_of,
                enumeration,
            },
            SchemaNodeKind::Integer {
                bounds,
                multiple_of,
                enumeration,
            } => Self::Integer {
                bounds: *bounds,
                multiple_of,
                enumeration,
            },
            SchemaNodeKind::Boolean { enumeration } => Self::Boolean { enumeration },
            SchemaNodeKind::Null { enumeration } => Self::Null { enumeration },
            SchemaNodeKind::Object {
                properties,
                pattern_properties,
                required,
                additional,
                property_names,
                property_count,
                dependent_required,
                enumeration,
            } => Self::Object {
                properties,
                pattern_properties,
                required,
                additional,
                property_names,
                property_count: *property_count,
                dependent_required,
                enumeration,
            },
            SchemaNodeKind::Array {
                prefix_items,
                items,
                item_count,
                contains,
                unique_items,
                enumeration,
            } => Self::Array {
                prefix_items,
                items,
                item_count: *item_count,
                contains: contains.as_ref(),
                unique_items: *unique_items,
                enumeration,
            },
            SchemaNodeKind::AllOf(children) => Self::AllOf(children),
            SchemaNodeKind::AnyOf(children) => Self::AnyOf(children),
            SchemaNodeKind::OneOf(children) => Self::OneOf(children),
            SchemaNodeKind::Not(child) => Self::Not(child),
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => Self::IfThenElse {
                if_schema,
                then_schema: then_schema.as_ref(),
                else_schema: else_schema.as_ref(),
            },
            SchemaNodeKind::Const(value) => Self::Const(value),
            SchemaNodeKind::Enum(values) => Self::Enum(values),
        }
    }
}

fn schema_node_graphs_are_equal<Node: SchemaNodeGraph>(
    left: &Node,
    right: &Node,
    seen: &mut HashSet<(usize, usize)>,
) -> bool {
    let key = (left.graph_ptr_id(), right.graph_ptr_id());
    if !seen.insert(key) {
        return true;
    }

    let mut children_are_equal =
        |left: &Node, right: &Node| schema_node_graphs_are_equal(left, right, seen);

    left.with_graph_kind(|left_kind| {
        right.with_graph_kind(|right_kind| {
            schema_node_kind_views_are_equal(left_kind, right_kind, &mut children_are_equal)
        })
    })
}

fn schema_node_kind_views_are_equal<Node>(
    left: SchemaNodeKindView<'_, Node>,
    right: SchemaNodeKindView<'_, Node>,
    children_are_equal: &mut impl FnMut(&Node, &Node) -> bool,
) -> bool {
    use SchemaNodeKindView::*;

    match (left, right) {
        (BoolSchema(left), BoolSchema(right)) => left == right,
        (Any, Any) => true,
        (Any, BoolSchema(true)) | (BoolSchema(true), Any) => true,
        (
            String {
                length: left_length,
                pattern: left_pattern,
                format: left_format,
                enumeration: left_enumeration,
            },
            String {
                length: right_length,
                pattern: right_pattern,
                format: right_format,
                enumeration: right_enumeration,
            },
        ) => {
            left_length == right_length
                && left_pattern == right_pattern
                && left_format == right_format
                && left_enumeration == right_enumeration
        }
        (
            Number {
                bounds: left_bounds,
                multiple_of: left_multiple_of,
                enumeration: left_enumeration,
            },
            Number {
                bounds: right_bounds,
                multiple_of: right_multiple_of,
                enumeration: right_enumeration,
            },
        ) => {
            left_bounds == right_bounds
                && left_multiple_of == right_multiple_of
                && left_enumeration == right_enumeration
        }
        (
            Integer {
                bounds: left_bounds,
                multiple_of: left_multiple_of,
                enumeration: left_enumeration,
            },
            Integer {
                bounds: right_bounds,
                multiple_of: right_multiple_of,
                enumeration: right_enumeration,
            },
        ) => {
            left_bounds == right_bounds
                && left_multiple_of == right_multiple_of
                && left_enumeration == right_enumeration
        }
        (
            Boolean {
                enumeration: left_enumeration,
            },
            Boolean {
                enumeration: right_enumeration,
            },
        )
        | (
            Null {
                enumeration: left_enumeration,
            },
            Null {
                enumeration: right_enumeration,
            },
        ) => left_enumeration == right_enumeration,
        (
            Object {
                properties: left_properties,
                pattern_properties: left_pattern_properties,
                required: left_required,
                additional: left_additional,
                property_names: left_property_names,
                property_count: left_property_count,
                dependent_required: left_dependent_required,
                enumeration: left_enumeration,
            },
            Object {
                properties: right_properties,
                pattern_properties: right_pattern_properties,
                required: right_required,
                additional: right_additional,
                property_names: right_property_names,
                property_count: right_property_count,
                dependent_required: right_dependent_required,
                enumeration: right_enumeration,
            },
        ) => {
            left_required == right_required
                && left_property_count == right_property_count
                && left_dependent_required == right_dependent_required
                && left_enumeration == right_enumeration
                && children_are_equal(left_additional, right_additional)
                && children_are_equal(left_property_names, right_property_names)
                && schema_node_maps_are_equal(left_properties, right_properties, children_are_equal)
                && pattern_property_maps_are_equal(
                    left_pattern_properties,
                    right_pattern_properties,
                    children_are_equal,
                )
        }
        (
            Array {
                prefix_items: left_prefix_items,
                items: left_items,
                item_count: left_item_count,
                contains: left_contains,
                unique_items: left_unique_items,
                enumeration: left_enumeration,
            },
            Array {
                prefix_items: right_prefix_items,
                items: right_items,
                item_count: right_item_count,
                contains: right_contains,
                unique_items: right_unique_items,
                enumeration: right_enumeration,
            },
        ) => {
            left_item_count == right_item_count
                && left_unique_items == right_unique_items
                && left_enumeration == right_enumeration
                && schema_node_slices_are_equal(
                    left_prefix_items,
                    right_prefix_items,
                    children_are_equal,
                )
                && children_are_equal(left_items, right_items)
                && array_contains_are_equal(left_contains, right_contains, children_are_equal)
        }
        (AllOf(left_children), AllOf(right_children))
        | (AnyOf(left_children), AnyOf(right_children))
        | (OneOf(left_children), OneOf(right_children)) => {
            schema_node_slices_are_equal(left_children, right_children, children_are_equal)
        }
        (Not(left_child), Not(right_child)) => children_are_equal(left_child, right_child),
        (
            IfThenElse {
                if_schema: left_if_schema,
                then_schema: left_then_schema,
                else_schema: left_else_schema,
            },
            IfThenElse {
                if_schema: right_if_schema,
                then_schema: right_then_schema,
                else_schema: right_else_schema,
            },
        ) => {
            children_are_equal(left_if_schema, right_if_schema)
                && optional_schema_nodes_are_equal(
                    left_then_schema,
                    right_then_schema,
                    children_are_equal,
                )
                && optional_schema_nodes_are_equal(
                    left_else_schema,
                    right_else_schema,
                    children_are_equal,
                )
        }
        (Const(left), Const(right)) => left == right,
        (Enum(left), Enum(right)) => left == right,
        (Ref(left), Ref(right)) => left == right,
        _ => false,
    }
}

fn pattern_property_maps_are_equal<Node>(
    left: &HashMap<String, PatternProperty<Node>>,
    right: &HashMap<String, PatternProperty<Node>>,
    children_are_equal: &mut impl FnMut(&Node, &Node) -> bool,
) -> bool {
    left.len() == right.len()
        && left.iter().all(|(pattern, left_property)| {
            right.get(pattern).is_some_and(|right_property| {
                left_property.pattern == right_property.pattern
                    && children_are_equal(&left_property.schema, &right_property.schema)
            })
        })
}

fn schema_node_maps_are_equal<Key, Node>(
    left: &HashMap<Key, Node>,
    right: &HashMap<Key, Node>,
    children_are_equal: &mut impl FnMut(&Node, &Node) -> bool,
) -> bool
where
    Key: Eq + Hash,
{
    left.len() == right.len()
        && left.iter().all(|(key, left_node)| {
            right
                .get(key)
                .is_some_and(|right_node| children_are_equal(left_node, right_node))
        })
}

fn schema_node_slices_are_equal<Node>(
    left: &[Node],
    right: &[Node],
    children_are_equal: &mut impl FnMut(&Node, &Node) -> bool,
) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right.iter())
            .all(|(left_node, right_node)| children_are_equal(left_node, right_node))
}

fn optional_schema_nodes_are_equal<Node>(
    left: Option<&Node>,
    right: Option<&Node>,
    children_are_equal: &mut impl FnMut(&Node, &Node) -> bool,
) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => children_are_equal(left, right),
        _ => false,
    }
}

fn array_contains_are_equal<Node>(
    left: Option<&ContainsConstraint<Node>>,
    right: Option<&ContainsConstraint<Node>>,
    children_are_equal: &mut impl FnMut(&Node, &Node) -> bool,
) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => {
            left.count() == right.count() && children_are_equal(&left.schema, &right.schema)
        }
        _ => false,
    }
}

fn freeze_schema_node(
    node: MutableSchemaNode,
    graph: &MutableSchemaGraph,
    cache: &mut HashMap<MutableSchemaNode, SchemaNode>,
) -> SchemaNode {
    if let Some(existing) = cache.get(&node) {
        return existing.clone();
    }

    let frozen = SchemaNode(Rc::new(OnceCell::new()));
    cache.insert(node, frozen.clone());

    let kind = freeze_schema_node_kind(graph.kind(node).clone(), graph, cache);
    frozen
        .0
        .set(kind)
        .expect("frozen SchemaNode must be initialized exactly once");
    frozen
}

fn freeze_schema_node_kind(
    kind: MutableSchemaNodeKind,
    graph: &MutableSchemaGraph,
    cache: &mut HashMap<MutableSchemaNode, SchemaNode>,
) -> SchemaNodeKind {
    match kind {
        MutableSchemaNodeKind::BoolSchema(value) => SchemaNodeKind::BoolSchema(value),
        MutableSchemaNodeKind::Any => SchemaNodeKind::Any,
        MutableSchemaNodeKind::String {
            length,
            pattern,
            format,
            enumeration,
        } => SchemaNodeKind::String {
            length,
            pattern,
            format,
            enumeration,
        },
        MutableSchemaNodeKind::Number {
            bounds,
            multiple_of,
            enumeration,
        } => SchemaNodeKind::Number {
            bounds,
            multiple_of,
            enumeration,
        },
        MutableSchemaNodeKind::Integer {
            bounds,
            multiple_of,
            enumeration,
        } => SchemaNodeKind::Integer {
            bounds,
            multiple_of,
            enumeration,
        },
        MutableSchemaNodeKind::Boolean { enumeration } => SchemaNodeKind::Boolean { enumeration },
        MutableSchemaNodeKind::Null { enumeration } => SchemaNodeKind::Null { enumeration },
        MutableSchemaNodeKind::Object {
            properties,
            pattern_properties,
            required,
            additional,
            property_names,
            property_count,
            dependent_required,
            enumeration,
        } => SchemaNodeKind::Object {
            properties: properties
                .into_iter()
                .map(|(name, child)| (name, freeze_schema_node(child, graph, cache)))
                .collect(),
            pattern_properties: pattern_properties
                .into_iter()
                .map(|(pattern, pattern_property)| {
                    (
                        pattern,
                        PatternProperty::new(
                            pattern_property.pattern,
                            freeze_schema_node(pattern_property.schema, graph, cache),
                        ),
                    )
                })
                .collect(),
            required,
            additional: freeze_schema_node(additional, graph, cache),
            property_names: freeze_schema_node(property_names, graph, cache),
            property_count,
            dependent_required,
            enumeration,
        },
        MutableSchemaNodeKind::Array {
            prefix_items,
            items,
            item_count,
            contains,
            unique_items,
            enumeration,
        } => SchemaNodeKind::Array {
            prefix_items: prefix_items
                .into_iter()
                .map(|child| freeze_schema_node(child, graph, cache))
                .collect(),
            items: freeze_schema_node(items, graph, cache),
            item_count,
            contains: contains.map(|contains| {
                ContainsConstraint::new(
                    freeze_schema_node(contains.schema, graph, cache),
                    contains.count(),
                )
            }),
            unique_items,
            enumeration,
        },
        MutableSchemaNodeKind::AllOf(children) => SchemaNodeKind::AllOf(
            children
                .iter()
                .map(|child| freeze_schema_node(*child, graph, cache))
                .collect(),
        ),
        MutableSchemaNodeKind::AnyOf(children) => SchemaNodeKind::AnyOf(
            children
                .iter()
                .map(|child| freeze_schema_node(*child, graph, cache))
                .collect(),
        ),
        MutableSchemaNodeKind::OneOf(children) => SchemaNodeKind::OneOf(
            children
                .iter()
                .map(|child| freeze_schema_node(*child, graph, cache))
                .collect(),
        ),
        MutableSchemaNodeKind::Not(child) => {
            SchemaNodeKind::Not(freeze_schema_node(child, graph, cache))
        }
        MutableSchemaNodeKind::IfThenElse {
            if_schema,
            then_schema,
            else_schema,
        } => SchemaNodeKind::IfThenElse {
            if_schema: freeze_schema_node(if_schema, graph, cache),
            then_schema: then_schema.map(|child| freeze_schema_node(child, graph, cache)),
            else_schema: else_schema.map(|child| freeze_schema_node(child, graph, cache)),
        },
        MutableSchemaNodeKind::Const(value) => SchemaNodeKind::Const(value),
        MutableSchemaNodeKind::Enum(values) => SchemaNodeKind::Enum(values),
        MutableSchemaNodeKind::Ref(_) => {
            unreachable!("parser-only schema node kind remained after reference resolution")
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

impl std::borrow::Borrow<SchemaNodeKind> for SchemaNode {
    fn borrow(&self) -> &SchemaNodeKind {
        self.kind()
    }
}

impl PartialEq for SchemaNode {
    fn eq(&self, other: &Self) -> bool {
        schema_node_graphs_are_equal(self, other, &mut HashSet::new())
    }
}

impl Eq for SchemaNode {}

/// Positive `multipleOf` constraint stored on integer schemas.
///
/// Integer-valued factors are preserved exactly. Fractional factors are stored as
/// finite positive `f64`s and projected to their implied integer divisor when
/// checking integer instances (`1.5` only admits multiples of `3`, `0.5` admits
/// all integers).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IntegerMultipleOf(IntegerMultipleOfKind);

#[derive(Debug, Clone, Copy, PartialEq)]
enum IntegerMultipleOfKind {
    Integer(NonZeroI64),
    Number(f64),
}

/// Positive `multipleOf` constraint stored on number schemas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumberMultipleOf(f64);

impl NumberMultipleOf {
    fn new(value: f64) -> Option<Self> {
        (value.is_finite() && value > 0.0).then_some(Self(value))
    }

    /// Return the positive finite divisor as `f64`.
    #[must_use]
    pub fn as_f64(self) -> f64 {
        self.0
    }

    /// Return true when this number divisor can be represented as a JSON
    /// integer without loss.
    #[must_use]
    pub fn is_integer_valued(self) -> bool {
        integer_value_from_json(&Value::Number(self.to_json_number())).is_some()
    }

    /// Return the implied integer divisor for integer-valued JSON instances.
    ///
    /// Fractional divisors such as `0.5` admit every integer and therefore
    /// return `Some(1)`.
    #[must_use]
    pub fn integer_divisor(self) -> Option<i128> {
        decimal_number_integer_divisor(self.0)
    }

    /// Return whether every integer multiple of `self` is also a multiple of
    /// `divisor`.
    #[must_use]
    pub fn integer_divisor_is_multiple_of(self, divisor: Self) -> Option<bool> {
        Some(
            self.integer_divisor()?
                .rem_euclid(divisor.integer_divisor()?)
                == 0,
        )
    }

    fn to_json_number(self) -> serde_json::Number {
        serde_json::Number::from_f64(self.0).expect("finite positive multipleOf")
    }
}

impl IntegerMultipleOf {
    fn from_integer(value: i64) -> Option<Self> {
        NonZeroI64::new(value)
            .filter(|value| value.get() > 0)
            .map(|value| Self(IntegerMultipleOfKind::Integer(value)))
    }

    fn from_number(value: f64) -> Option<Self> {
        (value.is_finite() && value > 0.0).then_some(Self(IntegerMultipleOfKind::Number(value)))
    }

    /// Return the positive divisor as `f64`.
    #[must_use]
    pub fn as_f64(self) -> f64 {
        match self.0 {
            IntegerMultipleOfKind::Integer(value) => value.get() as f64,
            IntegerMultipleOfKind::Number(value) => value,
        }
    }

    /// Return the implied integer divisor for integer instances.
    #[must_use]
    pub fn integer_divisor(self) -> Option<i128> {
        match self.0 {
            IntegerMultipleOfKind::Integer(value) => Some(i128::from(value.get())),
            IntegerMultipleOfKind::Number(value) => decimal_number_integer_divisor(value),
        }
    }

    /// Return whether every integer multiple of `self` is also a multiple of
    /// `divisor`.
    #[must_use]
    pub fn integer_divisor_is_multiple_of(self, divisor: Self) -> Option<bool> {
        Some(
            self.integer_divisor()?
                .rem_euclid(divisor.integer_divisor()?)
                == 0,
        )
    }

    /// Return whether every integer multiple of `self` is also a multiple of
    /// the number-schema divisor.
    #[must_use]
    pub fn integer_divisor_is_multiple_of_number(self, divisor: NumberMultipleOf) -> Option<bool> {
        Some(
            self.integer_divisor()?
                .rem_euclid(divisor.integer_divisor()?)
                == 0,
        )
    }

    /// Return the implied integer divisor when it fits in `i64`.
    #[must_use]
    pub fn integer_divisor_i64(self) -> Option<i64> {
        let divisor = self.integer_divisor()?;
        i64::try_from(divisor).ok()
    }

    #[cfg(test)]
    fn to_json_number(self) -> serde_json::Number {
        match self.0 {
            IntegerMultipleOfKind::Integer(value) => value.get().into(),
            IntegerMultipleOfKind::Number(value) => {
                serde_json::Number::from_f64(value).expect("finite positive multipleOf")
            }
        }
    }
}

/// A resolved, executable JSON Schema node.
///
/// This enum intentionally excludes parser-only states (`$ref`) and
/// keyword-fragment nodes (`type`, `required`, annotation keywords, and
/// similar) so successful resolution produces a graph with fewer impossible
/// states for downstream crates to reason about.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum SchemaNodeKind<Node = SchemaNode> {
    /// Boolean schema form: `true` accepts all values, `false` accepts none.
    BoolSchema(bool),
    /// Unconstrained schema object.
    Any,

    /// String schema with normalized length, pattern, format, and enum constraints.
    String {
        length: CountRange<u64>,
        pattern: Option<PatternConstraint>,
        format: Option<String>,
        enumeration: Option<Vec<Value>>,
    },
    /// Number schema with normalized bounds and `multipleOf`.
    Number {
        bounds: NumberBounds,
        multiple_of: Option<NumberMultipleOf>,
        enumeration: Option<Vec<Value>>,
    },
    /// Integer schema with exact integer bounds and divisor information.
    Integer {
        bounds: IntegerBounds,
        multiple_of: Option<IntegerMultipleOf>,
        enumeration: Option<Vec<Value>>,
    },
    /// Boolean schema with optional enum restriction.
    Boolean { enumeration: Option<Vec<Value>> },
    /// Null schema with optional enum restriction.
    Null { enumeration: Option<Vec<Value>> },

    /// Object schema with resolved property schemas and normalized count constraints.
    Object {
        properties: HashMap<String, Node>,
        pattern_properties: HashMap<String, PatternProperty<Node>>,
        required: HashSet<String>,
        additional: Node,
        property_names: Node,
        property_count: CountRange<usize>,
        dependent_required: HashMap<String, Vec<String>>,
        enumeration: Option<Vec<Value>>,
    },
    /// Array schema with tuple, tail-item, `contains`, and uniqueness constraints.
    Array {
        prefix_items: Vec<Node>,
        items: Node,
        item_count: CountRange<u64>,
        contains: Option<ContainsConstraint<Node>>,
        unique_items: bool,
        enumeration: Option<Vec<Value>>,
    },

    /// Conjunction of child schemas.
    AllOf(Vec<Node>),
    /// Disjunction of child schemas.
    AnyOf(Vec<Node>),
    /// Exact-one disjunction of child schemas.
    OneOf(Vec<Node>),
    /// Negation of a child schema.
    Not(Node),
    /// Conditional applicator with optional `then` and `else` branches.
    IfThenElse {
        if_schema: Node,
        then_schema: Option<Node>,
        else_schema: Option<Node>,
    },

    /// Single JSON value accepted under JSON Schema equality semantics.
    Const(Value),
    /// Finite set of JSON values accepted under JSON Schema equality semantics.
    Enum(Vec<Value>),
}

/// Private parser/resolver graph node that may still contain unresolved refs or
/// keyword-fragment variants before freezing into `SchemaNodeKind`.
#[derive(Debug, Clone)]
#[non_exhaustive]
enum MutableSchemaNodeKind<Node = MutableSchemaNode> {
    BoolSchema(bool),
    Any,

    String {
        length: CountRange<u64>,
        pattern: Option<PatternConstraint>,
        format: Option<String>,
        enumeration: Option<Vec<Value>>,
    },
    Number {
        bounds: NumberBounds,
        multiple_of: Option<NumberMultipleOf>,
        enumeration: Option<Vec<Value>>,
    },
    Integer {
        bounds: IntegerBounds,
        multiple_of: Option<IntegerMultipleOf>,
        enumeration: Option<Vec<Value>>,
    },
    Boolean {
        enumeration: Option<Vec<Value>>,
    },
    Null {
        enumeration: Option<Vec<Value>>,
    },

    Object {
        properties: HashMap<String, Node>,
        pattern_properties: HashMap<String, PatternProperty<Node>>,
        required: HashSet<String>,
        additional: Node,
        property_names: Node,
        property_count: CountRange<usize>,
        dependent_required: HashMap<String, Vec<String>>,
        enumeration: Option<Vec<Value>>,
    },
    Array {
        prefix_items: Vec<Node>,
        items: Node,
        item_count: CountRange<u64>,
        contains: Option<ContainsConstraint<Node>>,
        unique_items: bool,
        enumeration: Option<Vec<Value>>,
    },

    AllOf(Vec<Node>),
    AnyOf(Vec<Node>),
    OneOf(Vec<Node>),
    Not(Node),
    IfThenElse {
        if_schema: Node,
        then_schema: Option<Node>,
        else_schema: Option<Node>,
    },

    Const(Value),
    Enum(Vec<Value>),
    Ref(String),
}

#[cfg(test)]
pub(crate) fn build_and_resolve_schema(raw: &Value) -> Result<SchemaNode> {
    Ok(SchemaDocument::from_json(raw)?.root()?.clone())
}

fn build_schema_ast_from_value(
    raw: &Value,
    graph: &mut MutableSchemaGraph,
) -> Result<MutableSchemaNode> {
    if let Some(b) = raw.as_bool() {
        return Ok(graph.bool_schema(b));
    }
    let Some(obj) = raw.as_object() else {
        return Ok(graph.any());
    };

    match SchemaShape::classify(obj) {
        SchemaShape::Ref(ref_path) => Ok(parse_ref_schema(graph, ref_path)),
        SchemaShape::Enum(values) => Ok(parse_enum_schema(graph, values)),
        SchemaShape::UnsupportedReference(ref_path) => Err(AstError::UnsupportedReference {
            ref_path: ref_path.to_owned(),
        }),
        SchemaShape::Const(value) => Ok(parse_const_schema(graph, value)),
        SchemaShape::Conditional {
            if_schema,
            then_schema,
            else_schema,
        } => parse_conditional_schema(obj, if_schema, then_schema, else_schema, graph),
        SchemaShape::AllOf(subschemas) => parse_all_of_schema(obj, subschemas, graph),
        SchemaShape::AnyOf(subschemas) => parse_any_of_schema(obj, subschemas, graph),
        SchemaShape::OneOf(subschemas) => parse_one_of_schema(obj, subschemas, graph),
        SchemaShape::Not(schema) => parse_not_schema(obj, schema, graph),
        SchemaShape::String => parse_string_schema(obj, graph),
        SchemaShape::Number => parse_number_schema(obj, false, graph),
        SchemaShape::Integer => parse_number_schema(obj, true, graph),
        SchemaShape::Boolean => parse_boolean_schema(obj, graph),
        SchemaShape::Null => parse_null_schema(obj, graph),
        SchemaShape::Object => parse_object_schema(obj, graph),
        SchemaShape::Array => parse_array_schema(obj, graph),
        SchemaShape::TypeUnion(type_names) => parse_type_union_schema(obj, type_names, graph),
        SchemaShape::Any => Ok(graph.any()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SchemaShape<'a> {
    Ref(&'a str),
    UnsupportedReference(&'a str),
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

        if let Some(ref_path) = keywords.unsupported_ref_path {
            return Self::UnsupportedReference(ref_path);
        }
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
    unsupported_ref_path: Option<&'a str>,
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
                "$id" | "$anchor" | "$dynamicRef" | "$dynamicAnchor" => {
                    keywords.unsupported_ref_path = Some(value.as_str().unwrap_or(key));
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

fn parse_ref_schema(graph: &mut MutableSchemaGraph, ref_path: &str) -> MutableSchemaNode {
    graph.push(MutableSchemaNodeKind::Ref(ref_path.to_owned()))
}

fn parse_enum_schema(graph: &mut MutableSchemaGraph, values: &[Value]) -> MutableSchemaNode {
    graph.push(MutableSchemaNodeKind::Enum(values.to_vec()))
}

fn parse_const_schema(graph: &mut MutableSchemaGraph, value: &Value) -> MutableSchemaNode {
    graph.push(MutableSchemaNodeKind::Const(value.clone()))
}

fn parse_conditional_schema(
    obj: &Map<String, Value>,
    if_schema: Option<&Value>,
    then_schema: Option<&Value>,
    else_schema: Option<&Value>,
    graph: &mut MutableSchemaGraph,
) -> Result<MutableSchemaNode> {
    let Some(cond) = if_schema else {
        let mut base = obj.clone();
        base.remove("then");
        base.remove("else");
        return build_schema_ast_from_value(&Value::Object(base), graph);
    };

    let if_schema = build_schema_ast_from_value(cond, graph)?;
    let then_schema = then_schema
        .map(|schema| build_schema_ast_from_value(schema, graph))
        .transpose()?;
    let else_schema = else_schema
        .map(|schema| build_schema_ast_from_value(schema, graph))
        .transpose()?;

    let cond_node = graph.push(MutableSchemaNodeKind::IfThenElse {
        if_schema,
        then_schema,
        else_schema,
    });

    if let Some(base_schema) = parse_applicator_base_schema(obj, &["if", "then", "else"], graph)? {
        return Ok(graph.push(MutableSchemaNodeKind::AllOf(vec![base_schema, cond_node])));
    }

    Ok(cond_node)
}

fn parse_all_of_schema(
    obj: &Map<String, Value>,
    subschemas: &[Value],
    graph: &mut MutableSchemaGraph,
) -> Result<MutableSchemaNode> {
    let mut list = Vec::new();
    if let Some(base_schema) = parse_applicator_base_schema(obj, &["allOf"], graph)? {
        list.push(base_schema);
    }
    for schema in subschemas {
        list.push(build_schema_ast_from_value(schema, graph)?);
    }

    Ok(graph.push(MutableSchemaNodeKind::AllOf(list)))
}

fn parse_any_of_schema(
    obj: &Map<String, Value>,
    subschemas: &[Value],
    graph: &mut MutableSchemaGraph,
) -> Result<MutableSchemaNode> {
    let branches = subschemas
        .iter()
        .map(|schema| build_schema_ast_from_value(schema, graph))
        .collect::<Result<Vec<_>>>()?;
    let any_of = graph.push(MutableSchemaNodeKind::AnyOf(branches));

    if let Some(base_schema) = parse_applicator_base_schema(obj, &["anyOf"], graph)? {
        return Ok(graph.push(MutableSchemaNodeKind::AllOf(vec![base_schema, any_of])));
    }

    Ok(any_of)
}

fn parse_one_of_schema(
    obj: &Map<String, Value>,
    subschemas: &[Value],
    graph: &mut MutableSchemaGraph,
) -> Result<MutableSchemaNode> {
    let branches = subschemas
        .iter()
        .map(|schema| build_schema_ast_from_value(schema, graph))
        .collect::<Result<Vec<_>>>()?;
    let one_of = graph.push(MutableSchemaNodeKind::OneOf(branches));

    if let Some(base_schema) = parse_applicator_base_schema(obj, &["oneOf"], graph)? {
        return Ok(graph.push(MutableSchemaNodeKind::AllOf(vec![base_schema, one_of])));
    }

    Ok(one_of)
}

fn parse_not_schema(
    obj: &Map<String, Value>,
    schema: &Value,
    graph: &mut MutableSchemaGraph,
) -> Result<MutableSchemaNode> {
    let child = build_schema_ast_from_value(schema, graph)?;
    let not_node = graph.push(MutableSchemaNodeKind::Not(child));

    if let Some(base_schema) = parse_applicator_base_schema(obj, &["not"], graph)? {
        return Ok(graph.push(MutableSchemaNodeKind::AllOf(vec![base_schema, not_node])));
    }

    Ok(not_node)
}

fn parse_type_union_schema(
    obj: &Map<String, Value>,
    type_names: &[Value],
    graph: &mut MutableSchemaGraph,
) -> Result<MutableSchemaNode> {
    let mut variants = Vec::new();
    for type_name in type_names {
        if let Some(type_name) = type_name.as_str() {
            let mut typed_obj = obj.clone();
            typed_obj.insert("type".into(), Value::String(type_name.into()));
            variants.push(build_schema_ast_from_value(
                &Value::Object(typed_obj),
                graph,
            )?);
        }
    }

    if variants.len() == 1 {
        Ok(variants.remove(0))
    } else {
        Ok(graph.push(MutableSchemaNodeKind::AnyOf(variants)))
    }
}

fn parse_applicator_base_schema(
    obj: &Map<String, Value>,
    applicator_keys: &[&str],
    graph: &mut MutableSchemaGraph,
) -> Result<Option<MutableSchemaNode>> {
    let mut base = obj.clone();
    for key in applicator_keys {
        base.remove(*key);
    }
    strip_schema_metadata(&mut base);

    if base.is_empty() {
        Ok(None)
    } else {
        Ok(Some(build_schema_ast_from_value(
            &Value::Object(base),
            graph,
        )?))
    }
}

fn parse_string_schema(
    obj: &Map<String, Value>,
    graph: &mut MutableSchemaGraph,
) -> Result<MutableSchemaNode> {
    let Some(length) = CountRange::new(
        parse_u64_keyword(obj, "minLength")?.unwrap_or(0),
        parse_u64_keyword(obj, "maxLength")?,
    ) else {
        return Ok(graph.bool_schema(false));
    };
    let pattern = parse_string_keyword(obj, "pattern")?.map(PatternConstraint::new);
    let format = parse_string_keyword(obj, "format")?;
    let enumeration = parse_enum_keyword(obj)?;

    Ok(graph.push(MutableSchemaNodeKind::String {
        length,
        pattern,
        format,
        enumeration,
    }))
}

fn parse_number_schema(
    obj: &Map<String, Value>,
    integer: bool,
    graph: &mut MutableSchemaGraph,
) -> Result<MutableSchemaNode> {
    if integer {
        let Some(bounds) = IntegerBounds::from_json_schema_keywords(
            parse_i64_keyword(obj, "minimum")?,
            parse_i64_keyword(obj, "exclusiveMinimum")?,
            parse_i64_keyword(obj, "maximum")?,
            parse_i64_keyword(obj, "exclusiveMaximum")?,
        ) else {
            return Ok(graph.bool_schema(false));
        };

        let multiple_of =
            parse_integer_multiple_of_keyword(obj)?.or_else(|| IntegerMultipleOf::from_integer(1));

        return Ok(graph.push(MutableSchemaNodeKind::Integer {
            bounds,
            multiple_of,
            enumeration: parse_enum_keyword(obj)?,
        }));
    }

    let lower = if let Some(bound) = parse_f64_keyword(obj, "exclusiveMinimum")? {
        NumberBound::Exclusive(bound)
    } else if let Some(bound) = parse_f64_keyword(obj, "minimum")? {
        NumberBound::Inclusive(bound)
    } else {
        NumberBound::Unbounded
    };

    let upper = if let Some(bound) = parse_f64_keyword(obj, "exclusiveMaximum")? {
        NumberBound::Exclusive(bound)
    } else if let Some(bound) = parse_f64_keyword(obj, "maximum")? {
        NumberBound::Inclusive(bound)
    } else {
        NumberBound::Unbounded
    };
    let Some(bounds) = NumberBounds::new(lower, upper) else {
        return Ok(graph.bool_schema(false));
    };
    let enumeration = parse_enum_keyword(obj)?;

    let multiple_of = parse_number_multiple_of_keyword(obj)?;

    Ok(graph.push(MutableSchemaNodeKind::Number {
        bounds,
        multiple_of,
        enumeration,
    }))
}

fn parse_boolean_schema(
    obj: &Map<String, Value>,
    graph: &mut MutableSchemaGraph,
) -> Result<MutableSchemaNode> {
    Ok(graph.push(MutableSchemaNodeKind::Boolean {
        enumeration: parse_enum_keyword(obj)?,
    }))
}

fn parse_null_schema(
    obj: &Map<String, Value>,
    graph: &mut MutableSchemaGraph,
) -> Result<MutableSchemaNode> {
    Ok(graph.push(MutableSchemaNodeKind::Null {
        enumeration: parse_enum_keyword(obj)?,
    }))
}

fn parse_object_schema(
    obj: &Map<String, Value>,
    graph: &mut MutableSchemaGraph,
) -> Result<MutableSchemaNode> {
    let mut properties = HashMap::new();
    if let Some(value) = obj.get("properties") {
        let props = parse_object_keyword(value, "properties")?;
        for (k, v) in props {
            properties.insert(k.clone(), build_schema_ast_from_value(v, graph)?);
        }
    }
    let mut pattern_properties = HashMap::new();
    if let Some(value) = obj.get("patternProperties") {
        let props = parse_object_keyword(value, "patternProperties")?;
        for (pattern, schema) in props {
            pattern_properties.insert(
                pattern.clone(),
                PatternProperty::new(
                    PatternConstraint::new(pattern.clone()),
                    build_schema_ast_from_value(schema, graph)?,
                ),
            );
        }
    }
    let required = parse_string_set_keyword(obj, "required")?;

    for name in &required {
        if !properties.contains_key(name) {
            properties.insert(name.clone(), graph.any());
        }
    }

    let additional = match obj.get("additionalProperties") {
        None => graph.any(),
        Some(Value::Bool(b)) => graph.bool_schema(*b),
        Some(other) => build_schema_ast_from_value(other, graph)?,
    };

    let property_names = match obj.get("propertyNames") {
        None => graph.any(),
        Some(Value::Bool(true)) => graph.any(),
        Some(Value::Bool(false)) => graph.bool_schema(false),
        Some(other) => build_schema_ast_from_value(other, graph)?,
    };

    let Some(property_count) = CountRange::new(
        parse_usize_keyword(obj, "minProperties")?.unwrap_or(required.len()),
        parse_usize_keyword(obj, "maxProperties")?,
    ) else {
        return Ok(graph.bool_schema(false));
    };
    let dependent_required = parse_dependent_required_keyword(obj)?;
    let enumeration = parse_enum_keyword(obj)?;

    Ok(graph.push(MutableSchemaNodeKind::Object {
        properties,
        pattern_properties,
        required,
        additional,
        property_names,
        property_count,
        dependent_required,
        enumeration,
    }))
}

fn parse_array_schema(
    obj: &Map<String, Value>,
    graph: &mut MutableSchemaGraph,
) -> Result<MutableSchemaNode> {
    let mut prefix_items =
        parse_schema_array_keyword(obj.get("prefixItems"), "prefixItems", graph)?;
    let items_node = match obj.get("items") {
        None => graph.any(),
        Some(Value::Bool(true)) => graph.any(),
        Some(Value::Bool(false)) => graph.bool_schema(false),
        Some(Value::Array(arr)) => {
            prefix_items.extend(
                arr.iter()
                    .map(|schema| build_schema_ast_from_value(schema, graph))
                    .collect::<Result<Vec<_>>>()?,
            );
            graph.any()
        }
        Some(other) => build_schema_ast_from_value(other, graph)?,
    };
    let min_contains = parse_u64_keyword(obj, "minContains")?.unwrap_or(1);
    let max_contains = parse_u64_keyword(obj, "maxContains")?;
    let Some(contains_count) = CountRange::new(min_contains, max_contains) else {
        return Ok(graph.bool_schema(false));
    };
    let min_items = parse_u64_keyword(obj, "minItems")?.unwrap_or_else(|| {
        if obj.contains_key("contains") {
            contains_count.min()
        } else {
            0
        }
    });
    let Some(item_count) = CountRange::new(min_items, parse_u64_keyword(obj, "maxItems")?) else {
        return Ok(graph.bool_schema(false));
    };
    let Some(item_count) =
        cap_array_item_count_for_false_items(&prefix_items, items_node, item_count, graph)?
    else {
        return Ok(graph.bool_schema(false));
    };
    let unique_items = parse_bool_keyword(obj, "uniqueItems")?.unwrap_or(false);
    let enumeration = parse_enum_keyword(obj)?;

    let contains = obj
        .get("contains")
        .map(
            |contains| -> Result<ContainsConstraint<MutableSchemaNode>> {
                Ok(ContainsConstraint::new(
                    build_schema_ast_from_value(contains, graph)?,
                    contains_count,
                ))
            },
        )
        .transpose()?;

    Ok(graph.push(MutableSchemaNodeKind::Array {
        prefix_items,
        items: items_node,
        item_count,
        contains,
        unique_items,
        enumeration,
    }))
}

fn cap_array_item_count_for_false_items(
    prefix_items: &[MutableSchemaNode],
    items_node: MutableSchemaNode,
    item_count: CountRange<u64>,
    graph: &MutableSchemaGraph,
) -> Result<Option<CountRange<u64>>> {
    if !is_false_schema(items_node, graph) {
        return Ok(Some(item_count));
    }

    let tuple_max_items = u64::try_from(prefix_items.len()).map_err(|_| {
        AstError::Schema(SchemaError::IntegerKeywordOutOfRange {
            pointer: "#/prefixItems".to_owned(),
            keyword: "prefixItems".to_owned(),
        })
    })?;

    let max_items = item_count
        .max()
        .map_or(tuple_max_items, |max_items| max_items.min(tuple_max_items));
    Ok(CountRange::new(item_count.min(), Some(max_items)))
}

fn parse_schema_array_keyword(
    items: Option<&Value>,
    keyword: &str,
    graph: &mut MutableSchemaGraph,
) -> Result<Vec<MutableSchemaNode>> {
    let Some(items) = items else {
        return Ok(Vec::new());
    };
    let items = items
        .as_array()
        .ok_or_else(|| invalid_parser_keyword_type(keyword, "an array", items))?;

    items
        .iter()
        .map(|schema| build_schema_ast_from_value(schema, graph))
        .collect()
}

fn parse_enum_keyword(obj: &Map<String, Value>) -> Result<Option<Vec<Value>>> {
    obj.get("enum")
        .map(|value| {
            value
                .as_array()
                .cloned()
                .ok_or_else(|| invalid_parser_keyword_type("enum", "an array", value))
        })
        .transpose()
}

fn parse_string_keyword(obj: &Map<String, Value>, keyword: &str) -> Result<Option<String>> {
    obj.get(keyword)
        .map(|value| {
            value
                .as_str()
                .map(ToOwned::to_owned)
                .ok_or_else(|| invalid_parser_keyword_type(keyword, "a string", value))
        })
        .transpose()
}

fn parse_bool_keyword(obj: &Map<String, Value>, keyword: &str) -> Result<Option<bool>> {
    obj.get(keyword)
        .map(|value| {
            value
                .as_bool()
                .ok_or_else(|| invalid_parser_keyword_type(keyword, "a boolean", value))
        })
        .transpose()
}

fn parse_u64_keyword(obj: &Map<String, Value>, keyword: &str) -> Result<Option<u64>> {
    obj.get(keyword)
        .map(|value| {
            integer_value_from_json(value)
                .and_then(|integer| u64::try_from(integer).ok())
                .ok_or_else(|| {
                    invalid_parser_keyword_type(keyword, "a non-negative integer", value)
                })
        })
        .transpose()
}

fn parse_usize_keyword(obj: &Map<String, Value>, keyword: &str) -> Result<Option<usize>> {
    parse_u64_keyword(obj, keyword)?
        .map(|value| {
            usize::try_from(value).map_err(|_| {
                AstError::Schema(SchemaError::IntegerKeywordOutOfRange {
                    pointer: format!("#/{keyword}"),
                    keyword: keyword.to_owned(),
                })
            })
        })
        .transpose()
}

fn parse_f64_keyword(obj: &Map<String, Value>, keyword: &str) -> Result<Option<f64>> {
    obj.get(keyword)
        .map(|value| {
            let number = value
                .as_f64()
                .ok_or_else(|| invalid_parser_keyword_type(keyword, "a finite number", value))?;
            if !number.is_finite() {
                return Err(AstError::Schema(SchemaError::NonFiniteNumericKeyword {
                    pointer: format!("#/{keyword}"),
                    keyword: keyword.to_owned(),
                }));
            }
            Ok(number)
        })
        .transpose()
}

fn parse_i64_keyword(obj: &Map<String, Value>, keyword: &str) -> Result<Option<i64>> {
    obj.get(keyword)
        .map(|value| {
            parse_integer_value(Some(value)).ok_or_else(|| {
                invalid_parser_keyword_type(
                    keyword,
                    "an integer in the supported signed 64-bit range",
                    value,
                )
            })
        })
        .transpose()
}

fn parse_integer_multiple_of_keyword(
    obj: &Map<String, Value>,
) -> Result<Option<IntegerMultipleOf>> {
    let Some(value) = obj.get("multipleOf") else {
        return Ok(None);
    };
    let multiple_of = parse_integer_multiple_of(Some(value))
        .ok_or_else(|| invalid_parser_keyword_type("multipleOf", "a positive number", value))?;
    Ok(Some(multiple_of))
}

fn parse_number_multiple_of_keyword(obj: &Map<String, Value>) -> Result<Option<NumberMultipleOf>> {
    let Some(value) = obj.get("multipleOf") else {
        return Ok(None);
    };
    let multiple_of = value
        .as_f64()
        .and_then(NumberMultipleOf::new)
        .ok_or_else(|| invalid_parser_keyword_type("multipleOf", "a positive number", value))?;
    Ok(Some(multiple_of))
}

fn parse_object_keyword<'a>(value: &'a Value, keyword: &str) -> Result<&'a Map<String, Value>> {
    value
        .as_object()
        .ok_or_else(|| invalid_parser_keyword_type(keyword, "an object", value))
}

fn parse_string_set_keyword(obj: &Map<String, Value>, keyword: &str) -> Result<HashSet<String>> {
    let Some(value) = obj.get(keyword) else {
        return Ok(HashSet::new());
    };
    let items = value
        .as_array()
        .ok_or_else(|| invalid_parser_keyword_type(keyword, "an array of strings", value))?;
    let mut names = HashSet::new();
    for (index, item) in items.iter().enumerate() {
        let name = item
            .as_str()
            .ok_or_else(|| invalid_parser_keyword_entry_type(keyword, index, "a string", item))?;
        names.insert(name.to_owned());
    }
    Ok(names)
}

fn parse_dependent_required_keyword(
    obj: &Map<String, Value>,
) -> Result<HashMap<String, Vec<String>>> {
    let Some(value) = obj.get("dependentRequired") else {
        return Ok(HashMap::new());
    };
    let entries = parse_object_keyword(value, "dependentRequired")?;
    let mut dependent_required = HashMap::new();
    for (name, deps) in entries {
        let deps = deps.as_array().ok_or_else(|| {
            invalid_parser_keyword_type("dependentRequired", "an object of string arrays", deps)
        })?;
        let mut parsed_deps = Vec::with_capacity(deps.len());
        for (index, dep) in deps.iter().enumerate() {
            let dep = dep.as_str().ok_or_else(|| {
                invalid_parser_keyword_entry_type("dependentRequired", index, "a string", dep)
            })?;
            parsed_deps.push(dep.to_owned());
        }
        dependent_required.insert(name.clone(), parsed_deps);
    }
    Ok(dependent_required)
}

fn invalid_parser_keyword_type(
    keyword: &str,
    expected_type: &'static str,
    value: &Value,
) -> AstError {
    AstError::Schema(SchemaError::InvalidKeywordType {
        pointer: format!("#/{keyword}"),
        keyword: keyword.to_owned(),
        expected_type,
        actual_type: json_type_name(value),
    })
}

fn invalid_parser_keyword_entry_type(
    keyword: &str,
    index: usize,
    expected_type: &'static str,
    value: &Value,
) -> AstError {
    AstError::Schema(SchemaError::InvalidKeywordEntryType {
        pointer: format!("#/{keyword}"),
        keyword: keyword.to_owned(),
        index,
        expected_type,
        actual_type: json_type_name(value),
    })
}

fn resolve_refs_internal(
    node: MutableSchemaNode,
    graph: &mut MutableSchemaGraph,
    root_json: &Value,
    stack: &mut Vec<String>,
    cache: &mut HashMap<String, MutableSchemaNode>,
) -> Result<MutableSchemaNode> {
    match graph.kind(node).clone() {
        MutableSchemaNodeKind::Ref(path) => {
            if let Some(existing) = cache.get(&path).copied() {
                return resolve_cached_ref_alias(&path, existing, graph, stack, cache);
            }

            if !(path == "#" || path.starts_with("#/")) {
                return Err(AstError::UnsupportedReference { ref_path: path });
            }

            let mut current = root_json;
            if let Some(stripped) = path.strip_prefix("#/") {
                for token in stripped.split('/').map(decode_ref_token) {
                    current = resolve_json_pointer_child(current, &token).ok_or_else(|| {
                        AstError::UnresolvedReference {
                            ref_path: path.clone(),
                        }
                    })?;
                }
            }

            let resolved = build_schema_ast_from_value(current, graph)?;
            cache.insert(path.clone(), resolved);
            stack.push(path.clone());
            let resolved = resolve_refs_internal(resolved, graph, root_json, stack, cache)?;
            stack.pop();
            cache.insert(path, resolved);
            Ok(resolved)
        }
        MutableSchemaNodeKind::AllOf(children) => {
            let children = children
                .into_iter()
                .map(|child| resolve_refs_internal(child, graph, root_json, stack, cache))
                .collect::<Result<Vec<_>>>()?;
            graph.set_kind(node, MutableSchemaNodeKind::AllOf(children));
            normalize_resolved_node(node, graph);
            Ok(node)
        }
        MutableSchemaNodeKind::AnyOf(children) => {
            let children = children
                .into_iter()
                .map(|child| resolve_refs_internal(child, graph, root_json, stack, cache))
                .collect::<Result<Vec<_>>>()?;
            graph.set_kind(node, MutableSchemaNodeKind::AnyOf(children));
            normalize_resolved_node(node, graph);
            Ok(node)
        }
        MutableSchemaNodeKind::OneOf(children) => {
            let children = children
                .into_iter()
                .map(|child| resolve_refs_internal(child, graph, root_json, stack, cache))
                .collect::<Result<Vec<_>>>()?;
            graph.set_kind(node, MutableSchemaNodeKind::OneOf(children));
            normalize_resolved_node(node, graph);
            Ok(node)
        }
        MutableSchemaNodeKind::Not(child) => {
            let child = resolve_refs_internal(child, graph, root_json, stack, cache)?;
            graph.set_kind(node, MutableSchemaNodeKind::Not(child));
            normalize_resolved_node(node, graph);
            Ok(node)
        }
        MutableSchemaNodeKind::IfThenElse {
            if_schema,
            then_schema,
            else_schema,
        } => {
            let if_schema = resolve_refs_internal(if_schema, graph, root_json, stack, cache)?;
            let then_schema = then_schema
                .map(|child| resolve_refs_internal(child, graph, root_json, stack, cache))
                .transpose()?;
            let else_schema = else_schema
                .map(|child| resolve_refs_internal(child, graph, root_json, stack, cache))
                .transpose()?;
            graph.set_kind(
                node,
                MutableSchemaNodeKind::IfThenElse {
                    if_schema,
                    then_schema,
                    else_schema,
                },
            );
            normalize_resolved_node(node, graph);
            Ok(node)
        }
        MutableSchemaNodeKind::Object {
            mut properties,
            mut pattern_properties,
            required,
            mut additional,
            mut property_names,
            property_count,
            dependent_required,
            enumeration,
        } => {
            for child in properties.values_mut() {
                *child = resolve_refs_internal(*child, graph, root_json, stack, cache)?;
            }
            for pattern_property in pattern_properties.values_mut() {
                pattern_property.schema =
                    resolve_refs_internal(pattern_property.schema, graph, root_json, stack, cache)?;
            }
            additional = resolve_refs_internal(additional, graph, root_json, stack, cache)?;
            property_names = resolve_refs_internal(property_names, graph, root_json, stack, cache)?;
            graph.set_kind(
                node,
                MutableSchemaNodeKind::Object {
                    properties,
                    pattern_properties,
                    required,
                    additional,
                    property_names,
                    property_count,
                    dependent_required,
                    enumeration,
                },
            );
            Ok(node)
        }
        MutableSchemaNodeKind::Array {
            mut prefix_items,
            mut items,
            item_count,
            mut contains,
            unique_items,
            enumeration,
        } => {
            for child in &mut prefix_items {
                *child = resolve_refs_internal(*child, graph, root_json, stack, cache)?;
            }
            items = resolve_refs_internal(items, graph, root_json, stack, cache)?;
            if let Some(contains) = &mut contains {
                contains.schema =
                    resolve_refs_internal(contains.schema, graph, root_json, stack, cache)?;
            }
            let Some(item_count) =
                cap_array_item_count_for_false_items(&prefix_items, items, item_count, graph)?
            else {
                graph.set_kind(node, MutableSchemaNodeKind::BoolSchema(false));
                return Ok(node);
            };
            graph.set_kind(
                node,
                MutableSchemaNodeKind::Array {
                    prefix_items,
                    items,
                    item_count,
                    contains,
                    unique_items,
                    enumeration,
                },
            );
            Ok(node)
        }
        MutableSchemaNodeKind::BoolSchema(_)
        | MutableSchemaNodeKind::Any
        | MutableSchemaNodeKind::String { .. }
        | MutableSchemaNodeKind::Number { .. }
        | MutableSchemaNodeKind::Integer { .. }
        | MutableSchemaNodeKind::Boolean { .. }
        | MutableSchemaNodeKind::Null { .. }
        | MutableSchemaNodeKind::Const(_)
        | MutableSchemaNodeKind::Enum(_) => Ok(node),
    }
}

fn resolve_cached_ref_alias(
    ref_path: &str,
    cached_node: MutableSchemaNode,
    graph: &MutableSchemaGraph,
    stack: &[String],
    cache: &HashMap<String, MutableSchemaNode>,
) -> Result<MutableSchemaNode> {
    if !stack.iter().any(|active_path| active_path == ref_path) {
        return Ok(cached_node);
    }

    let mut current_path = ref_path.to_owned();
    let mut visited_paths = HashSet::new();

    while let Some(current_node) = cache.get(&current_path).cloned() {
        if !stack.iter().any(|active_path| active_path == &current_path) {
            return Ok(current_node);
        }

        let next_path = {
            match graph.kind(current_node) {
                MutableSchemaNodeKind::Ref(next_path) => Some(next_path.clone()),
                _ => None,
            }
        };
        let Some(next_path) = next_path else {
            return Ok(current_node);
        };

        if !visited_paths.insert(current_path.clone()) {
            // Every ref target observed in this active chain is still a
            // parser-only `Ref`, so this is an alias-only cycle
            // (`A -> B -> A` or `{"$ref":"#"}`), not productive recursion
            // through a concrete schema wrapper.
            return Err(AstError::CyclicReferenceAlias {
                ref_path: ref_path.to_owned(),
            });
        }

        current_path = next_path;
    }

    Ok(cached_node)
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

fn decode_ref_token(token: &str) -> String {
    let mut decoded = percent_decode_str(token).decode_utf8_lossy().into_owned();
    decoded = decoded.replace("~1", "/");
    decoded.replace("~0", "~")
}

fn normalize_resolved_node(node: MutableSchemaNode, graph: &mut MutableSchemaGraph) {
    let current = graph.kind(node).clone();
    let replacement = match current {
        MutableSchemaNodeKind::AllOf(mut children) => {
            if children.iter().any(|child| is_false_schema(*child, graph)) {
                Some(MutableSchemaNodeKind::BoolSchema(false))
            } else {
                children.retain(|child| !is_any_schema(*child, graph));
                collapse_applicator_children(&children, true, graph)
                    .or(Some(MutableSchemaNodeKind::AllOf(children)))
            }
        }
        MutableSchemaNodeKind::AnyOf(mut children) => {
            if children.iter().any(|child| is_any_schema(*child, graph)) {
                Some(MutableSchemaNodeKind::Any)
            } else {
                children.retain(|child| !is_false_schema(*child, graph));
                collapse_applicator_children(&children, false, graph)
                    .or(Some(MutableSchemaNodeKind::AnyOf(children)))
            }
        }
        MutableSchemaNodeKind::OneOf(mut children) => {
            let any_branch_count = children
                .iter()
                .filter(|child| is_any_schema(**child, graph))
                .count();
            if any_branch_count > 1 {
                Some(MutableSchemaNodeKind::BoolSchema(false))
            } else {
                children.retain(|child| {
                    !is_any_schema(*child, graph) && !is_false_schema(*child, graph)
                });
                // Keep duplicates: `oneOf: [A, A]` matches `A` twice, so it is
                // unsatisfiable and must not collapse to `A`.
                collapse_one_of_children(children, any_branch_count == 1, graph)
            }
        }
        MutableSchemaNodeKind::Not(sub) => {
            if is_false_schema(sub, graph) {
                Some(MutableSchemaNodeKind::Any)
            } else if is_any_schema(sub, graph) {
                Some(MutableSchemaNodeKind::BoolSchema(false))
            } else {
                Some(MutableSchemaNodeKind::Not(sub))
            }
        }
        MutableSchemaNodeKind::IfThenElse {
            if_schema,
            mut then_schema,
            mut else_schema,
        } => {
            if then_schema
                .as_ref()
                .is_some_and(|child| is_any_schema(*child, graph))
            {
                then_schema = None;
            }
            if else_schema
                .as_ref()
                .is_some_and(|child| is_any_schema(*child, graph))
            {
                else_schema = None;
            }
            if then_schema.is_none() && else_schema.is_none() {
                Some(MutableSchemaNodeKind::Any)
            } else {
                Some(MutableSchemaNodeKind::IfThenElse {
                    if_schema,
                    then_schema,
                    else_schema,
                })
            }
        }
        _ => None,
    };

    if let Some(kind) = replacement {
        graph.set_kind(node, kind);
    }
}

fn collapse_applicator_children(
    children: &[MutableSchemaNode],
    empty_is_any: bool,
    graph: &MutableSchemaGraph,
) -> Option<MutableSchemaNodeKind> {
    match children.len() {
        0 => Some(if empty_is_any {
            MutableSchemaNodeKind::Any
        } else {
            MutableSchemaNodeKind::BoolSchema(false)
        }),
        1 => Some(graph.kind(children[0]).clone()),
        _ => None,
    }
}

fn collapse_one_of_children(
    mut children: Vec<MutableSchemaNode>,
    has_always_valid_branch: bool,
    graph: &mut MutableSchemaGraph,
) -> Option<MutableSchemaNodeKind> {
    if !has_always_valid_branch {
        return collapse_applicator_children(&children, false, graph)
            .or(Some(MutableSchemaNodeKind::OneOf(children)));
    }

    match children.len() {
        0 => Some(MutableSchemaNodeKind::Any),
        1 => Some(MutableSchemaNodeKind::Not(children.remove(0))),
        _ => Some(MutableSchemaNodeKind::Not(
            graph.push(MutableSchemaNodeKind::AnyOf(children)),
        )),
    }
}

fn is_any_schema(node: MutableSchemaNode, graph: &MutableSchemaGraph) -> bool {
    matches!(
        graph.kind(node),
        MutableSchemaNodeKind::Any | MutableSchemaNodeKind::BoolSchema(true)
    )
}

fn is_false_schema(node: MutableSchemaNode, graph: &MutableSchemaGraph) -> bool {
    matches!(graph.kind(node), MutableSchemaNodeKind::BoolSchema(false))
}
