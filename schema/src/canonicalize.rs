/*
The canonical representation implemented here is heavily inspired by Juan Cruz
Viotti's `jsonschema canonicalize` format:

https://github.com/sourcemeta/jsonschema/blob/67f5ebf04430d655383dab4f75758f65ab28b1ca/docs/canonicalize.markdown

This implementation intentionally deviates in a few places to preserve metadata
and expose schema structure in the ways `jsoncompat` needs for static
compatibility checks.
*/

use crate::schema_metadata::{
    JSONCOMPAT_METADATA_KEY, PRESERVED_SCHEMA_METADATA_KEYS, TERMINAL_SCHEMA_METADATA_KEYS,
    is_schema_metadata_key,
};
use percent_encoding::percent_decode_str;
use serde_json::{Map, Number, Value};
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::ops::{BitOr, BitOrAssign};

type Result<T> = std::result::Result<T, CanonicalizeError>;

const JSON_SCHEMA_DRAFT_2020_12: &str = "https://json-schema.org/draft/2020-12/schema";
const JSON_SCHEMA_DRAFT_2020_12_WITH_FRAGMENT: &str =
    "https://json-schema.org/draft/2020-12/schema#";
const MAX_PASSES: usize = 64;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CanonicalizeError {
    #[error("schema node at '{pointer}' must be an object or boolean schema, got {actual_type}")]
    InvalidSchemaNodeType {
        pointer: String,
        actual_type: &'static str,
    },
    #[error("keyword '{keyword}' at '{pointer}' must be {expected_type}, got {actual_type}")]
    InvalidKeywordType {
        pointer: String,
        keyword: String,
        expected_type: &'static str,
        actual_type: &'static str,
    },
    #[error(
        "entry {index} of '{keyword}' at '{pointer}' must be {expected_type}, got {actual_type}"
    )]
    InvalidKeywordEntryType {
        pointer: String,
        keyword: String,
        index: usize,
        expected_type: &'static str,
        actual_type: &'static str,
    },
    #[error(
        "unsupported $schema URI at '{pointer}': expected '{expected_uri}', got '{actual_uri}'"
    )]
    UnsupportedSchemaDialect {
        pointer: String,
        expected_uri: &'static str,
        actual_uri: String,
    },
    #[error("numeric keyword '{keyword}' at '{pointer}' must be finite")]
    NonFiniteNumericKeyword { pointer: String, keyword: String },
    #[error(
        "integer keyword '{keyword}' at '{pointer}' is outside the supported signed 64-bit range"
    )]
    IntegerKeywordOutOfRange { pointer: String, keyword: String },
    #[error("canonicalization at '{pointer}' did not converge after {passes} rewrite passes")]
    RewriteDidNotConverge { pointer: String, passes: usize },
    #[error("internal canonicalizer error at '{pointer}': rewrite produced {actual_type}")]
    InvalidRewriteOutput {
        pointer: String,
        actual_type: &'static str,
    },
}

#[derive(Debug, Clone, Copy)]
struct CanonicalizationOptions<'a> {
    local_ref_targets: &'a BTreeSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PrimitiveType {
    Null,
    Boolean,
    Object,
    Array,
    String,
    Number,
    Integer,
}

#[derive(Debug, Clone, PartialEq)]
enum NormalizedIntegerBound {
    Inclusive(Value),
    Unsatisfiable,
}

impl PrimitiveType {
    #[must_use]
    const fn bit(self) -> u8 {
        match self {
            Self::Null => 1 << 0,
            Self::Boolean => 1 << 1,
            Self::Object => 1 << 2,
            Self::Array => 1 << 3,
            Self::String => 1 << 4,
            Self::Number => 1 << 5,
            Self::Integer => 1 << 6,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct PrimitiveTypeSet {
    bits: u8,
}

impl PrimitiveTypeSet {
    #[must_use]
    const fn empty() -> Self {
        Self { bits: 0 }
    }

    #[must_use]
    const fn is_empty(self) -> bool {
        self.bits == 0
    }

    #[must_use]
    const fn intersects(self, other: Self) -> bool {
        self.bits & other.bits != 0
    }

    #[must_use]
    fn contains(self, primitive_type: PrimitiveType) -> bool {
        self.bits & primitive_type.bit() != 0
    }
}

impl From<PrimitiveType> for PrimitiveTypeSet {
    fn from(value: PrimitiveType) -> Self {
        Self { bits: value.bit() }
    }
}

impl BitOr for PrimitiveTypeSet {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            bits: self.bits | rhs.bits,
        }
    }
}

impl BitOrAssign for PrimitiveTypeSet {
    fn bitor_assign(&mut self, rhs: Self) {
        self.bits |= rhs.bits;
    }
}

impl BitOrAssign<PrimitiveType> for PrimitiveTypeSet {
    fn bitor_assign(&mut self, rhs: PrimitiveType) {
        self.bits |= rhs.bit();
    }
}

/// Canonicalized JSON Schema document.
///
/// This wrapper intentionally keeps the canonical representation opaque so
/// callers have to construct it through `canonicalize_schema`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CanonicalSchema {
    value: Value,
}

impl CanonicalSchema {
    pub(crate) fn as_value(&self) -> &Value {
        &self.value
    }

    #[cfg(test)]
    pub(crate) fn into_value(self) -> Value {
        self.value
    }
}

impl AsRef<Value> for CanonicalSchema {
    fn as_ref(&self) -> &Value {
        self.as_value()
    }
}

/// Canonicalize a JSON Schema document by surfacing implicit constraints,
/// lowering syntax sugar into explicit forms, preserving declaration metadata
/// used by downstream codegen, and applying a deterministic key/value ordering.
pub(crate) fn canonicalize_schema(schema: &Value) -> Result<CanonicalSchema> {
    validate_schema_dialects(schema)?;

    let local_ref_targets = collect_local_schema_refs(schema);
    let options = CanonicalizationOptions {
        local_ref_targets: &local_ref_targets,
    };

    Ok(CanonicalSchema {
        value: canonicalize_schema_value(schema, "#", options)?,
    })
}

pub(crate) fn validate_schema_dialects(schema: &Value) -> Result<()> {
    let local_ref_targets = collect_local_schema_refs(schema);
    validate_schema_dialects_at_pointer(schema, "#", &local_ref_targets)
}

fn validate_schema_dialects_at_pointer(
    schema: &Value,
    pointer: &str,
    local_ref_targets: &BTreeSet<String>,
) -> Result<()> {
    match schema {
        Value::Object(object) => {
            if let Some(schema_uri) = object.get("$schema") {
                canonicalize_schema_uri(schema_uri, &join_pointer(pointer, "$schema"))?;
            }
            for (key, value) in object {
                if should_strip_keyword(key) {
                    continue;
                }

                let child_pointer = join_pointer(pointer, key);
                if is_known_keyword(key) {
                    validate_known_keyword_dialects(key, value, &child_pointer, local_ref_targets)?;
                } else if preserve_unknown_keyword_at_pointer(&child_pointer, local_ref_targets) {
                    validate_unknown_keyword_dialects(value, &child_pointer, local_ref_targets)?;
                }
            }
        }
        Value::Array(_) | Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }

    Ok(())
}

fn validate_known_keyword_dialects(
    key: &str,
    value: &Value,
    pointer: &str,
    local_ref_targets: &BTreeSet<String>,
) -> Result<()> {
    match key {
        "$schema" => Ok(()),
        "not"
        | "if"
        | "then"
        | "else"
        | "contains"
        | "additionalProperties"
        | "propertyNames"
        | "unevaluatedItems"
        | "unevaluatedProperties"
        | "contentSchema" => validate_schema_dialects_at_pointer(value, pointer, local_ref_targets),
        "items" => match value {
            Value::Array(items) => {
                for (index, item) in items.iter().enumerate() {
                    validate_schema_dialects_at_pointer(
                        item,
                        &join_pointer(pointer, &index.to_string()),
                        local_ref_targets,
                    )?;
                }
                Ok(())
            }
            _ => validate_schema_dialects_at_pointer(value, pointer, local_ref_targets),
        },
        "allOf" | "anyOf" | "oneOf" | "prefixItems" => {
            if let Value::Array(items) = value {
                for (index, item) in items.iter().enumerate() {
                    validate_schema_dialects_at_pointer(
                        item,
                        &join_pointer(pointer, &index.to_string()),
                        local_ref_targets,
                    )?;
                }
            }
            Ok(())
        }
        "properties" | "patternProperties" | "$defs" | "definitions" | "dependentSchemas" => {
            if let Value::Object(entries) = value {
                for (name, child) in entries {
                    validate_schema_dialects_at_pointer(
                        child,
                        &join_pointer(pointer, &escape_pointer_token(name)),
                        local_ref_targets,
                    )?;
                }
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn validate_unknown_keyword_dialects(
    value: &Value,
    pointer: &str,
    local_ref_targets: &BTreeSet<String>,
) -> Result<()> {
    match value {
        Value::Object(_) | Value::Bool(_) => {
            validate_schema_dialects_at_pointer(value, pointer, local_ref_targets)
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                validate_unknown_keyword_dialects(
                    item,
                    &join_pointer(pointer, &index.to_string()),
                    local_ref_targets,
                )?;
            }
            Ok(())
        }
        Value::Null | Value::Number(_) | Value::String(_) => Ok(()),
    }
}

/// Return a recursively key-sorted clone of arbitrary JSON data.
pub(crate) fn canonicalize_json(value: &Value) -> Value {
    match value {
        Value::Object(obj) => Value::Object(sorted_object(
            obj.iter()
                .map(|(key, value)| (key.clone(), canonicalize_json(value)))
                .collect(),
        )),
        Value::Array(items) => Value::Array(items.iter().map(canonicalize_json).collect()),
        _ => value.clone(),
    }
}

fn canonicalize_schema_value(
    schema: &Value,
    pointer: &str,
    options: CanonicalizationOptions<'_>,
) -> Result<Value> {
    match schema {
        Value::Bool(_) => Ok(schema.clone()),
        Value::Object(obj) => canonicalize_schema_object(obj, pointer, options),
        _ => Err(CanonicalizeError::InvalidSchemaNodeType {
            pointer: pointer.to_owned(),
            actual_type: json_type_name(schema),
        }),
    }
}

fn canonicalize_schema_object(
    obj: &Map<String, Value>,
    pointer: &str,
    options: CanonicalizationOptions<'_>,
) -> Result<Value> {
    let mut current = canonicalize_schema_members(obj, pointer, options)?;

    for _ in 0..MAX_PASSES {
        let next = rewrite_schema_object(&current, pointer, options)?;
        if next == current {
            return Ok(match next {
                Value::Object(obj) if obj.is_empty() => Value::Bool(true),
                other => other,
            });
        }
        current = match next {
            Value::Object(obj) => canonicalize_schema_members(&obj, pointer, options)?,
            Value::Bool(_) => next,
            _ => {
                return Err(CanonicalizeError::InvalidRewriteOutput {
                    pointer: pointer.to_owned(),
                    actual_type: json_type_name(&next),
                });
            }
        };
    }

    Err(CanonicalizeError::RewriteDidNotConverge {
        pointer: pointer.to_owned(),
        passes: MAX_PASSES,
    })
}

fn canonicalize_schema_members(
    obj: &Map<String, Value>,
    pointer: &str,
    options: CanonicalizationOptions<'_>,
) -> Result<Value> {
    let mut canonical = Map::new();
    for (key, value) in obj {
        if should_strip_keyword(key) {
            continue;
        }

        if !is_known_keyword(key) {
            let pointer = join_pointer(pointer, key);
            if !preserve_unknown_keyword_at_pointer(&pointer, options.local_ref_targets) {
                continue;
            }

            canonical.insert(
                key.clone(),
                canonicalize_unknown_keyword_value(value, &pointer, options)?,
            );
            continue;
        }

        let pointer = join_pointer(pointer, key);
        let value = canonicalize_keyword_value(key, value, &pointer, options)?;
        canonical.insert(key.clone(), value);
    }

    Ok(Value::Object(sorted_object(canonical)))
}

fn canonicalize_keyword_value(
    key: &str,
    value: &Value,
    pointer: &str,
    options: CanonicalizationOptions<'_>,
) -> Result<Value> {
    match key {
        "not"
        | "if"
        | "then"
        | "else"
        | "contains"
        | "additionalProperties"
        | "propertyNames"
        | "unevaluatedItems"
        | "unevaluatedProperties"
        | "contentSchema" => canonicalize_schema_value(value, pointer, options),
        "items" => match value {
            Value::Array(items) => items
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    canonicalize_schema_value(
                        item,
                        &join_pointer(pointer, &index.to_string()),
                        options,
                    )
                })
                .collect::<Result<Vec<_>>>()
                .map(Value::Array),
            _ => canonicalize_schema_value(value, pointer, options),
        },
        "allOf" | "anyOf" | "oneOf" | "prefixItems" => {
            let items = value
                .as_array()
                .ok_or_else(|| invalid_keyword_type(pointer, key, "an array", value))?;
            items
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    canonicalize_schema_value(
                        item,
                        &join_pointer(pointer, &index.to_string()),
                        options,
                    )
                })
                .collect::<Result<Vec<_>>>()
                .map(Value::Array)
        }
        "properties" | "patternProperties" | "$defs" | "definitions" | "dependentSchemas" => {
            let entries = value
                .as_object()
                .ok_or_else(|| invalid_keyword_type(pointer, key, "an object", value))?;
            let mut canonical = Map::new();
            for (name, child) in entries {
                canonical.insert(
                    name.clone(),
                    canonicalize_schema_value(
                        child,
                        &join_pointer(pointer, &escape_pointer_token(name)),
                        options,
                    )?,
                );
            }
            Ok(Value::Object(sorted_object(canonical)))
        }
        "required" => {
            let items = value.as_array().ok_or_else(|| {
                invalid_keyword_type(pointer, "required", "an array of strings", value)
            })?;
            let mut names = BTreeSet::new();
            for (index, item) in items.iter().enumerate() {
                let Some(name) = item.as_str() else {
                    return Err(invalid_keyword_entry_type(
                        pointer, "required", index, "a string", item,
                    ));
                };
                names.insert(name.to_owned());
            }
            Ok(Value::Array(names.into_iter().map(Value::String).collect()))
        }
        "dependentRequired" => {
            let entries = value.as_object().ok_or_else(|| {
                invalid_keyword_type(pointer, "dependentRequired", "an object", value)
            })?;
            let mut canonical = Map::new();
            for (name, deps) in entries {
                let deps = deps.as_array().ok_or_else(|| {
                    invalid_keyword_type(
                        &join_pointer(pointer, &escape_pointer_token(name)),
                        "dependentRequired",
                        "an array of strings",
                        deps,
                    )
                })?;
                let mut sorted_deps = BTreeSet::new();
                let dep_pointer = join_pointer(pointer, &escape_pointer_token(name));
                for (index, dep) in deps.iter().enumerate() {
                    let Some(dep) = dep.as_str() else {
                        return Err(invalid_keyword_entry_type(
                            &dep_pointer,
                            "dependentRequired",
                            index,
                            "a string",
                            dep,
                        ));
                    };
                    sorted_deps.insert(dep.to_owned());
                }
                canonical.insert(
                    name.clone(),
                    Value::Array(sorted_deps.into_iter().map(Value::String).collect()),
                );
            }
            Ok(Value::Object(sorted_object(canonical)))
        }
        "enum" => {
            let items = value
                .as_array()
                .ok_or_else(|| invalid_keyword_type(pointer, "enum", "an array", value))?;
            Ok(Value::Array(sorted_unique_json(items)))
        }
        "type" => match value {
            Value::String(_) => Ok(value.clone()),
            Value::Array(items) => {
                let mut unique = BTreeSet::new();
                for (index, item) in items.iter().enumerate() {
                    let Some(type_name) = item.as_str() else {
                        return Err(invalid_keyword_entry_type(
                            pointer, "type", index, "a string", item,
                        ));
                    };
                    unique.insert(type_name.to_owned());
                }
                Ok(Value::Array(
                    unique.into_iter().map(Value::String).collect(),
                ))
            }
            _ => Err(invalid_keyword_type(
                pointer,
                "type",
                "a string or an array of strings",
                value,
            )),
        },
        "$schema" => canonicalize_schema_uri(value, pointer),
        "const" | "default" | JSONCOMPAT_METADATA_KEY => Ok(canonicalize_json(value)),
        "examples" => {
            let items = value
                .as_array()
                .ok_or_else(|| invalid_keyword_type(pointer, "examples", "an array", value))?;
            Ok(Value::Array(items.iter().map(canonicalize_json).collect()))
        }
        _ => Ok(canonicalize_json(value)),
    }
}

fn canonicalize_unknown_keyword_value(
    value: &Value,
    pointer: &str,
    options: CanonicalizationOptions<'_>,
) -> Result<Value> {
    match value {
        Value::Object(_) | Value::Bool(_) => canonicalize_schema_value(value, pointer, options),
        Value::Array(items) => items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                canonicalize_unknown_keyword_value(
                    item,
                    &join_pointer(pointer, &index.to_string()),
                    options,
                )
            })
            .collect::<Result<Vec<_>>>()
            .map(Value::Array),
        _ => Ok(canonicalize_json(value)),
    }
}

fn rewrite_schema_object(
    schema: &Value,
    pointer: &str,
    options: CanonicalizationOptions<'_>,
) -> Result<Value> {
    let Value::Object(source) = schema else {
        return Ok(schema.clone());
    };
    let mut obj = source.clone();

    normalize_schema_uri(&mut obj);
    remove_not_false(&mut obj, pointer, options.local_ref_targets);
    remove_empty_conditionals(&mut obj, pointer, options.local_ref_targets);
    remove_without_dependencies(&mut obj, pointer, options.local_ref_targets);
    dedupe_required(&mut obj, pointer)?;
    dedupe_enum(&mut obj);
    dedupe_schema_array(&mut obj, "allOf", pointer, options.local_ref_targets);
    dedupe_schema_array(&mut obj, "anyOf", pointer, options.local_ref_targets);
    simplify_allof(&mut obj, pointer, options.local_ref_targets);
    if let Some(result) = simplify_anyof(&mut obj, pointer, options.local_ref_targets) {
        return Ok(result);
    }
    if let Some(result) = simplify_oneof(&mut obj, pointer, options.local_ref_targets) {
        return Ok(result);
    }
    fold_required_dependencies(&mut obj, pointer)?;
    infer_required_properties(&mut obj, pointer)?;
    if let Some(result) = normalize_numeric_bounds(&mut obj, pointer)? {
        return Ok(result);
    }
    normalize_single_type_arrays(&mut obj, pointer)?;
    normalize_type_specific_keywords(&mut obj, pointer, options.local_ref_targets);
    lower_const_with_type(&mut obj);
    lower_enum_with_type(&mut obj);
    if let Some(result) = lower_equal_bounds_to_enum(&mut obj, pointer)? {
        return Ok(result);
    }
    if let Some(result) = lower_const_to_enum(&mut obj) {
        return Ok(result);
    }
    lower_boolean_and_null_types(&mut obj);
    insert_implicit_type_union(&mut obj, pointer, options.local_ref_targets);
    lower_type_array_to_any_of(&mut obj, pointer)?;
    if let Some(result) = rewrite_unsatisfiable_object(&obj, pointer, options.local_ref_targets) {
        return Ok(result);
    }
    fill_implicit_constraints(&mut obj);

    let schema = Value::Object(sorted_object(obj));
    if let Value::Object(obj) = &schema
        && obj.is_empty()
    {
        return Ok(Value::Bool(true));
    }

    Ok(schema)
}

fn normalize_schema_uri(schema: &mut Map<String, Value>) {
    let Some(Value::String(uri)) = schema.get_mut("$schema") else {
        return;
    };

    if uri == JSON_SCHEMA_DRAFT_2020_12_WITH_FRAGMENT {
        uri.pop();
    }
}

fn canonicalize_schema_uri(value: &Value, pointer: &str) -> Result<Value> {
    let Some(uri) = value.as_str() else {
        return Err(invalid_keyword_type(
            pointer,
            "$schema",
            "a URI string",
            value,
        ));
    };

    match uri {
        JSON_SCHEMA_DRAFT_2020_12 | JSON_SCHEMA_DRAFT_2020_12_WITH_FRAGMENT => {
            Ok(Value::String(JSON_SCHEMA_DRAFT_2020_12.to_owned()))
        }
        _ => Err(CanonicalizeError::UnsupportedSchemaDialect {
            pointer: pointer.to_owned(),
            expected_uri: JSON_SCHEMA_DRAFT_2020_12,
            actual_uri: uri.to_owned(),
        }),
    }
}

fn remove_not_false(
    schema: &mut Map<String, Value>,
    pointer: &str,
    local_ref_targets: &BTreeSet<String>,
) {
    if schema.get("not") == Some(&Value::Bool(false))
        && !preserve_unknown_keyword_at_pointer(&join_pointer(pointer, "not"), local_ref_targets)
    {
        schema.remove("not");
    }
}

fn remove_empty_conditionals(
    schema: &mut Map<String, Value>,
    pointer: &str,
    local_ref_targets: &BTreeSet<String>,
) {
    if matches!(schema.get("then"), Some(Value::Bool(true))) {
        schema.remove("then");
    }
    if matches!(schema.get("else"), Some(Value::Bool(true))) {
        schema.remove("else");
    }
    if let Some(if_schema) = schema.get("if")
        && !schema.contains_key("then")
        && !schema.contains_key("else")
        && !schema_contains_resource_identifier(if_schema)
        && !preserve_unknown_keyword_at_pointer(&join_pointer(pointer, "if"), local_ref_targets)
    {
        schema.remove("if");
    }
}

fn remove_without_dependencies(
    schema: &mut Map<String, Value>,
    pointer: &str,
    local_ref_targets: &BTreeSet<String>,
) {
    if !schema.contains_key("if") {
        if schema.get("then").is_some_and(|then_schema| {
            !schema_contains_resource_identifier(then_schema)
                && !preserve_unknown_keyword_at_pointer(
                    &join_pointer(pointer, "then"),
                    local_ref_targets,
                )
        }) {
            schema.remove("then");
        }
        if schema.get("else").is_some_and(|else_schema| {
            !schema_contains_resource_identifier(else_schema)
                && !preserve_unknown_keyword_at_pointer(
                    &join_pointer(pointer, "else"),
                    local_ref_targets,
                )
        }) {
            schema.remove("else");
        }
    }
    if !schema.contains_key("contains") {
        schema.remove("minContains");
        schema.remove("maxContains");
    }
    if !schema.contains_key("contentEncoding") {
        schema.remove("contentMediaType");
    }
    if !schema.contains_key("contentMediaType") {
        schema.remove("contentSchema");
    }
    if !matches!(schema.get("items"), Some(Value::Array(_))) {
        schema.remove("additionalItems");
    }
}

fn dedupe_required(schema: &mut Map<String, Value>, pointer: &str) -> Result<()> {
    let Some(Value::Array(required)) = schema.get("required") else {
        return Ok(());
    };
    let mut names = BTreeSet::new();
    let required_pointer = join_pointer(pointer, "required");
    for (index, name) in required.iter().enumerate() {
        let Some(name) = name.as_str() else {
            return Err(invalid_keyword_entry_type(
                &required_pointer,
                "required",
                index,
                "a string",
                name,
            ));
        };
        names.insert(name.to_owned());
    }
    schema.insert(
        "required".to_owned(),
        Value::Array(names.into_iter().map(Value::String).collect()),
    );
    Ok(())
}

fn dedupe_enum(schema: &mut Map<String, Value>) {
    let Some(Value::Array(values)) = schema.get("enum") else {
        return;
    };
    schema.insert("enum".to_owned(), Value::Array(sorted_unique_json(values)));
}

fn dedupe_schema_array(
    schema: &mut Map<String, Value>,
    keyword: &str,
    pointer: &str,
    local_ref_targets: &BTreeSet<String>,
) {
    let Some(Value::Array(values)) = schema.get(keyword) else {
        return;
    };

    if preserve_unknown_keyword_at_pointer(&join_pointer(pointer, keyword), local_ref_targets) {
        return;
    }

    let mut seen = BTreeSet::new();
    let mut unique = Vec::new();
    for value in values {
        let key = serde_json::to_string(&canonicalize_json(value))
            .expect("serializing canonical JSON value cannot fail");
        if seen.insert(key) {
            unique.push(value.clone());
        }
    }
    schema.insert(keyword.to_owned(), Value::Array(unique));
}

fn simplify_allof(
    schema: &mut Map<String, Value>,
    pointer: &str,
    local_ref_targets: &BTreeSet<String>,
) {
    let Some(Value::Array(branches)) = schema.get_mut("allOf") else {
        return;
    };

    if preserve_unknown_keyword_at_pointer(&join_pointer(pointer, "allOf"), local_ref_targets) {
        return;
    }

    if branches.iter().any(|branch| branch == &Value::Bool(false)) {
        let preserved = if preserve_unknown_keyword_at_pointer(pointer, local_ref_targets) {
            preserved_meta(schema)
        } else {
            preserved_terminal_meta(schema)
        };
        schema.clear();
        schema.extend(preserved);
        schema.insert("not".to_owned(), Value::Bool(true));
        return;
    }

    branches.retain(|branch| branch != &Value::Bool(true));
    if branches.is_empty() {
        schema.remove("allOf");
    }
}

fn simplify_anyof(
    schema: &mut Map<String, Value>,
    pointer: &str,
    local_ref_targets: &BTreeSet<String>,
) -> Option<Value> {
    let Some(Value::Array(branches)) = schema.get_mut("anyOf") else {
        return None;
    };

    if preserve_unknown_keyword_at_pointer(&join_pointer(pointer, "anyOf"), local_ref_targets) {
        return None;
    }

    if branches.iter().any(|branch| branch == &Value::Bool(true)) {
        schema.remove("anyOf");
        return None;
    }

    branches.retain(|branch| branch != &Value::Bool(false));
    if branches.is_empty() || (branches.len() == 1 && branches[0] == Value::Bool(false)) {
        let mut obj = if preserve_unknown_keyword_at_pointer(pointer, local_ref_targets) {
            preserved_meta(schema)
        } else {
            preserved_terminal_meta(schema)
        };
        obj.insert("not".to_owned(), Value::Bool(true));
        return Some(Value::Object(sorted_object(obj)));
    }
    if branches.is_empty() {
        schema.remove("anyOf");
    }
    None
}

fn simplify_oneof(
    schema: &mut Map<String, Value>,
    pointer: &str,
    local_ref_targets: &BTreeSet<String>,
) -> Option<Value> {
    let Some(Value::Array(branches)) = schema.get("oneOf") else {
        return None;
    };

    if preserve_unknown_keyword_at_pointer(&join_pointer(pointer, "oneOf"), local_ref_targets) {
        return None;
    }

    if branches.len() == 1 && branches[0] == Value::Bool(false) {
        let mut obj = if preserve_unknown_keyword_at_pointer(pointer, local_ref_targets) {
            preserved_meta(schema)
        } else {
            preserved_terminal_meta(schema)
        };
        obj.insert("not".to_owned(), Value::Bool(true));
        return Some(Value::Object(sorted_object(obj)));
    }

    if can_rewrite_oneof_to_anyof(branches) {
        let branches = branches.clone();
        schema.remove("oneOf");
        schema.insert("anyOf".to_owned(), Value::Array(branches));
    }
    None
}

#[must_use]
fn can_rewrite_oneof_to_anyof(branches: &[Value]) -> bool {
    let mut seen_types = PrimitiveTypeSet::empty();
    if branches.len() < 2 {
        return false;
    }

    for branch in branches {
        let Some(types) = branch_type_set(branch) else {
            return false;
        };
        if types.is_empty() {
            return false;
        }
        if seen_types.intersects(types) {
            return false;
        }
        seen_types |= types;
    }

    true
}

#[must_use]
fn branch_type_set(branch: &Value) -> Option<PrimitiveTypeSet> {
    match branch {
        Value::Bool(true) => None,
        Value::Bool(false) => Some(PrimitiveTypeSet::empty()),
        Value::Object(obj) => {
            if let Some(Value::String(type_name)) = obj.get("type") {
                return primitive_type_set(type_name.as_str());
            }
            if let Some(Value::Array(values)) = obj.get("enum") {
                let mut types = PrimitiveTypeSet::empty();
                for value in values {
                    types |= value_type_name(value)?;
                }
                return Some(types);
            }
            None
        }
        _ => None,
    }
}

#[must_use]
fn primitive_type_set(type_name: &str) -> Option<PrimitiveTypeSet> {
    match type_name {
        "null" => Some(PrimitiveType::Null.into()),
        "boolean" => Some(PrimitiveType::Boolean.into()),
        "object" => Some(PrimitiveType::Object.into()),
        "array" => Some(PrimitiveType::Array.into()),
        "string" => Some(PrimitiveType::String.into()),
        "number" => Some(
            PrimitiveTypeSet::from(PrimitiveType::Number)
                | PrimitiveTypeSet::from(PrimitiveType::Integer),
        ),
        "integer" => Some(PrimitiveType::Integer.into()),
        _ => None,
    }
}

#[must_use]
fn value_type_name(value: &Value) -> Option<PrimitiveType> {
    match value {
        Value::Null => Some(PrimitiveType::Null),
        Value::Bool(_) => Some(PrimitiveType::Boolean),
        Value::Object(_) => Some(PrimitiveType::Object),
        Value::Array(_) => Some(PrimitiveType::Array),
        Value::String(_) => Some(PrimitiveType::String),
        Value::Number(number) if number.is_i64() || number.is_u64() => Some(PrimitiveType::Integer),
        // This is a bit unfortunate because it relies on f64 semantics, but JSON
        // Schema's "integer" type intentionally accepts values like 1.0.
        Value::Number(number) if number.as_f64().is_some_and(|value| value.fract() == 0.0) => {
            Some(PrimitiveType::Integer)
        }
        Value::Number(_) => Some(PrimitiveType::Number),
    }
}

fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Object(_) => "object",
        Value::Array(_) => "array",
        Value::String(_) => "string",
        Value::Number(_) => "number",
    }
}

fn invalid_keyword_type(
    pointer: &str,
    keyword: &str,
    expected_type: &'static str,
    value: &Value,
) -> CanonicalizeError {
    CanonicalizeError::InvalidKeywordType {
        pointer: pointer.to_owned(),
        keyword: keyword.to_owned(),
        expected_type,
        actual_type: json_type_name(value),
    }
}

fn invalid_keyword_entry_type(
    pointer: &str,
    keyword: &str,
    index: usize,
    expected_type: &'static str,
    value: &Value,
) -> CanonicalizeError {
    CanonicalizeError::InvalidKeywordEntryType {
        pointer: pointer.to_owned(),
        keyword: keyword.to_owned(),
        index,
        expected_type,
        actual_type: json_type_name(value),
    }
}

fn last_pointer_token(pointer: &str) -> String {
    pointer.rsplit('/').next().unwrap_or(pointer).to_owned()
}

fn fold_required_dependencies(schema: &mut Map<String, Value>, pointer: &str) -> Result<()> {
    let Some(Value::Object(dependent_required)) = schema.get("dependentRequired") else {
        return Ok(());
    };
    let Some(Value::Array(required_values)) = schema.get("required") else {
        return Ok(());
    };

    let required_pointer = join_pointer(pointer, "required");
    let mut required = required_values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            value.as_str().map(str::to_owned).ok_or_else(|| {
                invalid_keyword_entry_type(&required_pointer, "required", index, "a string", value)
            })
        })
        .collect::<Result<BTreeSet<_>>>()?;

    let mut remaining = dependent_required.clone();
    loop {
        let mut changed = false;
        let triggers = required.iter().cloned().collect::<Vec<_>>();
        for trigger in triggers {
            let Some(Value::Array(deps)) = remaining.remove(&trigger) else {
                continue;
            };
            let trigger_pointer = join_pointer(
                &join_pointer(pointer, "dependentRequired"),
                &escape_pointer_token(&trigger),
            );
            for (index, dep) in deps.iter().enumerate() {
                let dep = dep.as_str().ok_or_else(|| {
                    invalid_keyword_entry_type(
                        &trigger_pointer,
                        "dependentRequired",
                        index,
                        "a string",
                        dep,
                    )
                })?;
                changed |= required.insert(dep.to_owned());
            }
        }
        if !changed {
            break;
        }
    }

    schema.insert(
        "required".to_owned(),
        Value::Array(required.into_iter().map(Value::String).collect()),
    );
    schema.insert(
        "dependentRequired".to_owned(),
        Value::Object(sorted_object(remaining)),
    );
    Ok(())
}

fn infer_required_properties(schema: &mut Map<String, Value>, pointer: &str) -> Result<()> {
    if schema.contains_key("additionalProperties") {
        return Ok(());
    }

    let Some(Value::Array(required)) = schema.get("required") else {
        return Ok(());
    };

    let mut properties = schema
        .get("properties")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let required_pointer = join_pointer(pointer, "required");
    for (index, name) in required.iter().enumerate() {
        let Some(name) = name.as_str() else {
            return Err(invalid_keyword_entry_type(
                &required_pointer,
                "required",
                index,
                "a string",
                name,
            ));
        };
        properties
            .entry(name.to_owned())
            .or_insert(Value::Bool(true));
    }

    if !properties.is_empty() {
        schema.insert(
            "properties".to_owned(),
            Value::Object(sorted_object(properties)),
        );
    }
    Ok(())
}

fn normalize_numeric_bounds(
    schema: &mut Map<String, Value>,
    pointer: &str,
) -> Result<Option<Value>> {
    if let (Some(maximum), Some(exclusive_maximum)) =
        (schema.get("maximum"), schema.get("exclusiveMaximum"))
        && let (Some(maximum), Some(exclusive_maximum)) =
            (maximum.as_f64(), exclusive_maximum.as_f64())
    {
        if maximum < exclusive_maximum {
            schema.remove("exclusiveMaximum");
        } else {
            schema.remove("maximum");
        }
    }

    if let (Some(minimum), Some(exclusive_minimum)) =
        (schema.get("minimum"), schema.get("exclusiveMinimum"))
        && let (Some(minimum), Some(exclusive_minimum)) =
            (minimum.as_f64(), exclusive_minimum.as_f64())
    {
        if exclusive_minimum < minimum {
            schema.remove("exclusiveMinimum");
        } else {
            schema.remove("minimum");
        }
    }

    if schema.get("type") == Some(&Value::String("integer".to_owned())) {
        if let Some(bound) = schema.get("maximum").cloned()
            && let Some(bound) = integer_floor(&bound, &join_pointer(pointer, "maximum"))?
        {
            schema.insert("maximum".to_owned(), bound);
        }
        if let Some(bound) = schema.get("minimum").cloned()
            && let Some(bound) = integer_ceil(&bound, &join_pointer(pointer, "minimum"))?
        {
            schema.insert("minimum".to_owned(), bound);
        }
        if !schema.contains_key("maximum")
            && let Some(bound) = schema.get("exclusiveMaximum").cloned()
            && let Some(bound) =
                integer_exclusive_maximum(&bound, &join_pointer(pointer, "exclusiveMaximum"))?
        {
            schema.remove("exclusiveMaximum");
            match bound {
                NormalizedIntegerBound::Inclusive(bound) => {
                    schema.insert("maximum".to_owned(), bound);
                }
                NormalizedIntegerBound::Unsatisfiable => {
                    return Ok(Some(Value::Object(unsatisfiable_object(schema))));
                }
            }
        }
        if !schema.contains_key("minimum")
            && let Some(bound) = schema.get("exclusiveMinimum").cloned()
            && let Some(bound) =
                integer_exclusive_minimum(&bound, &join_pointer(pointer, "exclusiveMinimum"))?
        {
            schema.remove("exclusiveMinimum");
            match bound {
                NormalizedIntegerBound::Inclusive(bound) => {
                    schema.insert("minimum".to_owned(), bound);
                }
                NormalizedIntegerBound::Unsatisfiable => {
                    return Ok(Some(Value::Object(unsatisfiable_object(schema))));
                }
            }
        }
    }

    Ok(None)
}

fn integer_floor(value: &Value, pointer: &str) -> Result<Option<Value>> {
    let Some(number) = value.as_number() else {
        return Ok(None);
    };
    if let Some(value) = number.as_i64() {
        return Ok(Some(Value::Number(Number::from(value))));
    }
    if let Some(value) = number.as_u64() {
        return Ok(Some(Value::Number(Number::from(checked_i64_from_u64(
            value, pointer,
        )?))));
    }
    let Some(value) = number.as_f64() else {
        return Err(CanonicalizeError::NonFiniteNumericKeyword {
            pointer: pointer.to_owned(),
            keyword: last_pointer_token(pointer),
        });
    };
    Ok(Some(number_from_f64(value.floor(), pointer)?))
}

fn integer_ceil(value: &Value, pointer: &str) -> Result<Option<Value>> {
    let Some(number) = value.as_number() else {
        return Ok(None);
    };
    if let Some(value) = number.as_i64() {
        return Ok(Some(Value::Number(Number::from(value))));
    }
    if let Some(value) = number.as_u64() {
        return Ok(Some(Value::Number(Number::from(checked_i64_from_u64(
            value, pointer,
        )?))));
    }
    let Some(value) = number.as_f64() else {
        return Err(CanonicalizeError::NonFiniteNumericKeyword {
            pointer: pointer.to_owned(),
            keyword: last_pointer_token(pointer),
        });
    };
    Ok(Some(number_from_f64(value.ceil(), pointer)?))
}

fn integer_exclusive_maximum(
    value: &Value,
    pointer: &str,
) -> Result<Option<NormalizedIntegerBound>> {
    let Some(number) = value.as_number() else {
        return Ok(None);
    };
    if let Some(value) = number.as_i64() {
        return Ok(Some(
            value
                .checked_sub(1)
                .map(|value| NormalizedIntegerBound::Inclusive(Value::Number(Number::from(value))))
                .unwrap_or(NormalizedIntegerBound::Unsatisfiable),
        ));
    }
    if let Some(value) = number.as_u64() {
        return if value == 0 {
            Ok(Some(NormalizedIntegerBound::Inclusive(Value::Number(
                Number::from(-1_i64),
            ))))
        } else {
            let maximum = (value - 1).min(i64::MAX as u64);
            Ok(Some(NormalizedIntegerBound::Inclusive(Value::Number(
                Number::from(maximum),
            ))))
        };
    }
    let Some(value) = number.as_f64() else {
        return Err(CanonicalizeError::NonFiniteNumericKeyword {
            pointer: pointer.to_owned(),
            keyword: last_pointer_token(pointer),
        });
    };
    let mut normalized = value.floor();
    if value.fract() == 0.0 {
        normalized -= 1.0;
    }
    if normalized < i64::MIN as f64 {
        return Ok(Some(NormalizedIntegerBound::Unsatisfiable));
    }
    if normalized > i64::MAX as f64 {
        return Ok(Some(NormalizedIntegerBound::Inclusive(Value::Number(
            Number::from(i64::MAX),
        ))));
    }
    Ok(Some(NormalizedIntegerBound::Inclusive(number_from_f64(
        normalized, pointer,
    )?)))
}

fn integer_exclusive_minimum(
    value: &Value,
    pointer: &str,
) -> Result<Option<NormalizedIntegerBound>> {
    let Some(number) = value.as_number() else {
        return Ok(None);
    };
    if let Some(value) = number.as_i64() {
        return Ok(Some(
            value
                .checked_add(1)
                .map(|value| NormalizedIntegerBound::Inclusive(Value::Number(Number::from(value))))
                .unwrap_or(NormalizedIntegerBound::Unsatisfiable),
        ));
    }
    if let Some(value) = number.as_u64() {
        if value >= i64::MAX as u64 {
            return Ok(Some(NormalizedIntegerBound::Unsatisfiable));
        }
        return Ok(Some(NormalizedIntegerBound::Inclusive(Value::Number(
            Number::from(value + 1),
        ))));
    }
    let Some(value) = number.as_f64() else {
        return Err(CanonicalizeError::NonFiniteNumericKeyword {
            pointer: pointer.to_owned(),
            keyword: last_pointer_token(pointer),
        });
    };
    let mut normalized = value.ceil();
    if value.fract() == 0.0 {
        normalized += 1.0;
    }
    if normalized > i64::MAX as f64 {
        return Ok(Some(NormalizedIntegerBound::Unsatisfiable));
    }
    if normalized < i64::MIN as f64 {
        return Ok(Some(NormalizedIntegerBound::Inclusive(Value::Number(
            Number::from(i64::MIN),
        ))));
    }
    Ok(Some(NormalizedIntegerBound::Inclusive(number_from_f64(
        normalized, pointer,
    )?)))
}

fn number_from_f64(value: f64, pointer: &str) -> Result<Value> {
    if value.fract() == 0.0 && value >= i64::MIN as f64 && value <= i64::MAX as f64 {
        return Ok(Value::Number(Number::from(value as i64)));
    }

    let Some(number) = Number::from_f64(value) else {
        return Err(CanonicalizeError::NonFiniteNumericKeyword {
            pointer: pointer.to_owned(),
            keyword: last_pointer_token(pointer),
        });
    };
    Ok(Value::Number(number))
}

fn normalize_single_type_arrays(schema: &mut Map<String, Value>, pointer: &str) -> Result<()> {
    let Some(Value::Array(types)) = schema.get("type") else {
        return Ok(());
    };
    if types.len() != 1 {
        return Ok(());
    }
    let Some(type_name) = types[0].as_str() else {
        return Err(invalid_keyword_entry_type(
            &join_pointer(pointer, "type"),
            "type",
            0,
            "a string",
            &types[0],
        ));
    };
    schema.insert("type".to_owned(), Value::String(type_name.to_owned()));
    Ok(())
}

fn normalize_type_specific_keywords(
    schema: &mut Map<String, Value>,
    pointer: &str,
    local_ref_targets: &BTreeSet<String>,
) {
    let Some(type_name) = schema.get("type").and_then(Value::as_str) else {
        return;
    };

    let allowed = match type_name {
        "string" => Some(
            [
                "type",
                "enum",
                "const",
                "minLength",
                "maxLength",
                "pattern",
                "format",
            ]
            .as_slice(),
        ),
        "number" | "integer" => Some(
            [
                "type",
                "enum",
                "const",
                "minimum",
                "maximum",
                "exclusiveMinimum",
                "exclusiveMaximum",
                "multipleOf",
            ]
            .as_slice(),
        ),
        "object" => Some(
            [
                "type",
                "enum",
                "const",
                "properties",
                "required",
                "additionalProperties",
                "patternProperties",
                "propertyNames",
                "minProperties",
                "maxProperties",
                "dependentRequired",
                "dependentSchemas",
                "unevaluatedProperties",
            ]
            .as_slice(),
        ),
        "array" => Some(
            [
                "type",
                "enum",
                "const",
                "items",
                "prefixItems",
                "contains",
                "minItems",
                "maxItems",
                "minContains",
                "maxContains",
                "uniqueItems",
                "unevaluatedItems",
            ]
            .as_slice(),
        ),
        "boolean" | "null" => Some(["type", "enum", "const"].as_slice()),
        _ => None,
    };
    let Some(allowed) = allowed else {
        return;
    };

    schema.retain(|key, _| {
        is_schema_metadata_key(key)
            || allowed.contains(&key.as_str())
            || preserve_unknown_keyword_at_pointer(&join_pointer(pointer, key), local_ref_targets)
            || matches!(
                key.as_str(),
                "allOf"
                    | "anyOf"
                    | "oneOf"
                    | "not"
                    | "if"
                    | "then"
                    | "else"
                    | "$ref"
                    | "$dynamicRef"
            )
    });
}

fn lower_const_with_type(schema: &mut Map<String, Value>) {
    let Some(const_value) = schema.get("const") else {
        return;
    };
    let Some(type_name) = schema.get("type") else {
        return;
    };
    if value_matches_type(const_value, type_name) {
        schema.remove("type");
    }
}

fn lower_enum_with_type(schema: &mut Map<String, Value>) {
    let Some(Value::Array(values)) = schema.get("enum") else {
        return;
    };
    let Some(type_name) = schema.get("type") else {
        return;
    };
    if values
        .iter()
        .all(|value| value_matches_type(value, type_name))
    {
        schema.remove("type");
    }
}

fn lower_equal_bounds_to_enum(
    schema: &mut Map<String, Value>,
    pointer: &str,
) -> Result<Option<Value>> {
    let Some(Value::String(type_name)) = schema.get("type") else {
        return Ok(None);
    };
    if type_name != "integer" && type_name != "number" {
        return Ok(None);
    }
    let (Some(minimum), Some(maximum)) = (schema.get("minimum"), schema.get("maximum")) else {
        return Ok(None);
    };
    if minimum != maximum {
        return Ok(None);
    }
    if let Some(multiple_of) = schema.get("multipleOf")
        && !value_is_multiple_of(
            minimum,
            multiple_of,
            type_name,
            &join_pointer(pointer, "minimum"),
            &join_pointer(pointer, "multipleOf"),
        )?
    {
        return Ok(Some(Value::Object(unsatisfiable_object(schema))));
    }
    schema.insert(
        "enum".to_owned(),
        Value::Array(vec![canonicalize_json(minimum)]),
    );
    schema.remove("type");
    schema.remove("minimum");
    schema.remove("maximum");
    schema.remove("multipleOf");
    Ok(None)
}

fn lower_const_to_enum(schema: &mut Map<String, Value>) -> Option<Value> {
    let value = schema.remove("const")?;
    let value = canonicalize_json(&value);

    if let Some(Value::Array(values)) = schema.get("enum") {
        if !values.contains(&value) {
            return Some(Value::Object(unsatisfiable_object(schema)));
        }
        schema.insert("enum".to_owned(), Value::Array(vec![value]));
        return None;
    }

    schema.insert("enum".to_owned(), Value::Array(vec![value]));
    None
}

fn lower_boolean_and_null_types(schema: &mut Map<String, Value>) {
    if schema.contains_key("enum") || schema.contains_key("const") {
        return;
    }
    match schema.get("type") {
        Some(Value::String(type_name)) if type_name == "boolean" => {
            schema.remove("type");
            schema.insert(
                "enum".to_owned(),
                Value::Array(vec![Value::Bool(false), Value::Bool(true)]),
            );
        }
        Some(Value::String(type_name)) if type_name == "null" => {
            schema.remove("type");
            schema.insert("enum".to_owned(), Value::Array(vec![Value::Null]));
        }
        _ => {}
    }
}

fn insert_implicit_type_union(
    schema: &mut Map<String, Value>,
    pointer: &str,
    local_ref_targets: &BTreeSet<String>,
) {
    if schema.is_empty()
        || pointer_is_strict_ancestor_of_local_ref_target(pointer, local_ref_targets)
    {
        return;
    }
    if schema.contains_key("type")
        || schema.contains_key("enum")
        || schema.contains_key("const")
        || schema.contains_key("$ref")
        || schema.contains_key("$dynamicRef")
        || schema.contains_key("allOf")
        || schema.contains_key("anyOf")
        || schema.contains_key("oneOf")
        || schema.contains_key("not")
        || schema.contains_key("if")
        || schema.contains_key("then")
        || schema.contains_key("else")
        || contains_local_schema_refs(schema)
    {
        return;
    }

    schema.insert(
        "type".to_owned(),
        Value::Array(vec![
            Value::String("null".to_owned()),
            Value::String("boolean".to_owned()),
            Value::String("object".to_owned()),
            Value::String("array".to_owned()),
            Value::String("string".to_owned()),
            Value::String("number".to_owned()),
        ]),
    );
}

fn pointer_is_strict_ancestor_of_local_ref_target(
    pointer: &str,
    local_ref_targets: &BTreeSet<String>,
) -> bool {
    let prefix = format!("{pointer}/");
    local_ref_targets
        .iter()
        .any(|reference| reference.starts_with(&prefix))
}

fn lower_type_array_to_any_of(schema: &mut Map<String, Value>, pointer: &str) -> Result<()> {
    let Some(Value::Array(types)) = schema.get("type").cloned() else {
        return Ok(());
    };
    if types.len() <= 1 {
        return Ok(());
    }
    if contains_local_schema_refs(schema) {
        return Ok(());
    }

    let mut preserved = preserved_meta(schema);
    let mut branch_context = Map::new();
    for (key, value) in schema.iter() {
        if key == "type" || key == "anyOf" || is_schema_metadata_key(key) {
            continue;
        }
        branch_context.insert(key.clone(), value.clone());
    }

    let mut branches = Vec::new();
    let type_pointer = join_pointer(pointer, "type");
    for (index, type_name) in types.iter().enumerate() {
        let Some(type_name) = type_name.as_str() else {
            return Err(invalid_keyword_entry_type(
                &type_pointer,
                "type",
                index,
                "a string",
                type_name,
            ));
        };
        let mut branch = branch_context.clone();
        branch.insert("type".to_owned(), Value::String(type_name.to_owned()));
        branches.push(Value::Object(sorted_object(branch)));
    }

    if let Some(existing_any_of) = schema.get("anyOf").cloned() {
        let mut first = branch_context;
        first.insert("anyOf".to_owned(), existing_any_of);
        let mut second = Map::new();
        second.insert("anyOf".to_owned(), Value::Array(branches));
        preserved.insert(
            "allOf".to_owned(),
            Value::Array(vec![
                Value::Object(sorted_object(first)),
                Value::Object(sorted_object(second)),
            ]),
        );
    } else {
        preserved.insert("anyOf".to_owned(), Value::Array(branches));
    }

    *schema = sorted_object(preserved);
    Ok(())
}

fn contains_local_schema_refs(schema: &Map<String, Value>) -> bool {
    schema
        .iter()
        .any(|(key, value)| value_contains_local_schema_refs(key, value))
}

fn value_contains_local_schema_refs(key: &str, value: &Value) -> bool {
    if matches!(key, "$ref" | "$dynamicRef")
        && let Some(reference) = value.as_str()
    {
        return reference.starts_with('#');
    }

    match value {
        Value::Object(obj) => contains_local_schema_refs(obj),
        Value::Array(items) => items
            .iter()
            .any(|item| value_contains_local_schema_refs("", item)),
        _ => false,
    }
}

fn collect_local_schema_refs(schema: &Value) -> BTreeSet<String> {
    let mut refs = BTreeSet::new();
    collect_local_schema_refs_in_value("", schema, &mut refs);
    refs
}

fn collect_local_schema_refs_in_value(key: &str, value: &Value, refs: &mut BTreeSet<String>) {
    if matches!(key, "$ref" | "$dynamicRef")
        && let Some(reference) = value.as_str()
        && let Some(reference) = normalize_local_ref_target(reference)
    {
        refs.insert(reference);
    }

    match value {
        Value::Object(obj) => {
            for (key, child) in obj {
                collect_local_schema_refs_in_value(key, child, refs);
            }
        }
        Value::Array(items) => {
            for child in items {
                collect_local_schema_refs_in_value("", child, refs);
            }
        }
        _ => {}
    }
}

fn preserve_unknown_keyword_at_pointer(
    pointer: &str,
    local_ref_targets: &BTreeSet<String>,
) -> bool {
    let prefix = format!("{pointer}/");
    local_ref_targets
        .iter()
        .any(|reference| reference == pointer || reference.starts_with(&prefix))
}

fn normalize_local_ref_target(reference: &str) -> Option<String> {
    if reference == "#" {
        return Some(reference.to_owned());
    }

    let stripped = reference.strip_prefix("#/")?;
    let mut normalized = String::from("#");
    for token in stripped.split('/') {
        let mut decoded = percent_decode_str(token).decode_utf8_lossy().into_owned();
        decoded = decoded.replace("~1", "/");
        decoded = decoded.replace("~0", "~");
        normalized.push('/');
        normalized.push_str(&escape_pointer_token(&decoded));
    }
    Some(normalized)
}

fn schema_contains_resource_identifier(schema: &Value) -> bool {
    match schema {
        Value::Object(obj) => {
            obj.contains_key("$id")
                || obj.contains_key("$anchor")
                || obj.contains_key("$dynamicAnchor")
                || obj.values().any(schema_contains_resource_identifier)
        }
        Value::Array(items) => items.iter().any(schema_contains_resource_identifier),
        _ => false,
    }
}

fn rewrite_unsatisfiable_object(
    schema: &Map<String, Value>,
    pointer: &str,
    local_ref_targets: &BTreeSet<String>,
) -> Option<Value> {
    if preserve_unknown_keyword_at_pointer(pointer, local_ref_targets) {
        return None;
    }

    let type_name = schema.get("type").and_then(Value::as_str);
    match type_name {
        Some("string") => {
            let min_length = schema.get("minLength").and_then(Value::as_u64).unwrap_or(0);
            if let Some(max_length) = schema.get("maxLength").and_then(Value::as_u64)
                && min_length > max_length
            {
                return Some(Value::Object(unsatisfiable_object(schema)));
            }
        }
        Some("object") => {
            let required_count = schema
                .get("required")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0);
            let min_properties = schema
                .get("minProperties")
                .and_then(Value::as_u64)
                .unwrap_or(required_count as u64);
            if let Some(max_properties) = schema.get("maxProperties").and_then(Value::as_u64)
                && min_properties > max_properties
            {
                return Some(Value::Object(unsatisfiable_object(schema)));
            }
        }
        Some("array") => {
            let min_items = schema.get("minItems").and_then(Value::as_u64).unwrap_or(0);
            if let Some(max_items) = schema.get("maxItems").and_then(Value::as_u64)
                && min_items > max_items
            {
                return Some(Value::Object(unsatisfiable_object(schema)));
            }
            if let (Some(min_contains), Some(max_contains)) = (
                schema.get("minContains").and_then(Value::as_u64),
                schema.get("maxContains").and_then(Value::as_u64),
            ) && min_contains > max_contains
            {
                return Some(Value::Object(unsatisfiable_object(schema)));
            }
            if let (Some(min_contains), Some(max_items)) = (
                schema.get("minContains").and_then(Value::as_u64),
                schema.get("maxItems").and_then(Value::as_u64),
            ) && min_contains > max_items
            {
                return Some(Value::Object(unsatisfiable_object(schema)));
            }
        }
        Some("integer") => {
            let min = schema.get("minimum").and_then(Value::as_i64);
            let max = schema.get("maximum").and_then(Value::as_i64);
            if let (Some(min), Some(max)) = (min, max)
                && min > max
            {
                return Some(Value::Object(unsatisfiable_object(schema)));
            }
        }
        Some("number") => {
            let min = schema.get("minimum").and_then(Value::as_f64);
            let max = schema.get("maximum").and_then(Value::as_f64);
            if let (Some(min), Some(max)) = (min, max)
                && min > max
            {
                return Some(Value::Object(unsatisfiable_object(schema)));
            }
        }
        _ => {}
    }
    None
}

fn fill_implicit_constraints(schema: &mut Map<String, Value>) {
    match schema.get("type").and_then(Value::as_str) {
        Some("object") => {
            if !schema.contains_key("properties") {
                schema.insert("properties".to_owned(), Value::Object(Map::new()));
            }
            if !schema.contains_key("minProperties") {
                let min_properties = schema
                    .get("required")
                    .and_then(Value::as_array)
                    .map(Vec::len)
                    .unwrap_or(0);
                schema.insert(
                    "minProperties".to_owned(),
                    Value::Number(Number::from(min_properties as u64)),
                );
            } else if let (Some(Value::Number(min_properties)), Some(Value::Array(required_values))) =
                (schema.get("minProperties").cloned(), schema.get("required"))
                && let Some(min_properties) = min_properties.as_u64()
                && (required_values.len() as u64) > min_properties
            {
                schema.insert(
                    "minProperties".to_owned(),
                    Value::Number(Number::from(required_values.len() as u64)),
                );
            }
        }
        Some("array") => {
            if !schema.contains_key("items") {
                schema.insert("items".to_owned(), Value::Bool(true));
            }
            if !schema.contains_key("minItems") {
                let min_items = if schema.contains_key("contains") {
                    schema
                        .get("minContains")
                        .and_then(Value::as_u64)
                        .unwrap_or(1)
                } else {
                    0
                };
                schema.insert(
                    "minItems".to_owned(),
                    Value::Number(Number::from(min_items)),
                );
            }
            if let (Some(max_contains), Some(max_items)) = (
                schema.get("maxContains").and_then(Value::as_u64),
                schema.get("maxItems").and_then(Value::as_u64),
            ) && max_contains > max_items
            {
                schema.insert(
                    "maxContains".to_owned(),
                    Value::Number(Number::from(max_items)),
                );
            }
        }
        Some("string") => {
            schema
                .entry("minLength".to_owned())
                .or_insert_with(|| Value::Number(Number::from(0_u64)));
        }
        Some("integer") => {
            schema
                .entry("multipleOf".to_owned())
                .or_insert_with(|| Value::Number(Number::from(1_u64)));
        }
        _ => {}
    }
}

fn value_matches_type(value: &Value, type_value: &Value) -> bool {
    match type_value {
        Value::String(type_name) => primitive_type_set(type_name).is_some_and(|types| {
            value_type_name(value).is_some_and(|value_type| types.contains(value_type))
        }),
        Value::Array(items) => items.iter().any(|item| value_matches_type(value, item)),
        _ => false,
    }
}

fn value_is_multiple_of(
    value: &Value,
    multiple_of: &Value,
    type_name: &str,
    value_pointer: &str,
    multiple_of_pointer: &str,
) -> Result<bool> {
    match type_name {
        "integer" => {
            let value = integer_value_from_json(value, value_pointer)?;
            let multiple_of = integer_value_from_json(multiple_of, multiple_of_pointer)?;
            Ok(multiple_of != 0 && value % multiple_of == 0)
        }
        "number" => {
            let Some(value) = value.as_f64() else {
                return Err(CanonicalizeError::NonFiniteNumericKeyword {
                    pointer: value_pointer.to_owned(),
                    keyword: last_pointer_token(value_pointer),
                });
            };
            let Some(multiple_of) = multiple_of.as_f64() else {
                return Err(CanonicalizeError::NonFiniteNumericKeyword {
                    pointer: multiple_of_pointer.to_owned(),
                    keyword: last_pointer_token(multiple_of_pointer),
                });
            };
            if multiple_of == 0.0 {
                return Ok(false);
            }
            let quotient = value / multiple_of;
            Ok((quotient - quotient.round()).abs() <= f64::EPSILON * quotient.abs().max(1.0) * 4.0)
        }
        _ => Ok(true),
    }
}

fn integer_value_from_json(value: &Value, pointer: &str) -> Result<i64> {
    let Some(number) = value.as_number() else {
        return Err(CanonicalizeError::IntegerKeywordOutOfRange {
            pointer: pointer.to_owned(),
            keyword: last_pointer_token(pointer),
        });
    };
    if let Some(value) = number.as_i64() {
        return Ok(value);
    }
    if let Some(value) = number.as_u64() {
        return checked_i64_from_u64(value, pointer);
    }
    Err(CanonicalizeError::IntegerKeywordOutOfRange {
        pointer: pointer.to_owned(),
        keyword: last_pointer_token(pointer),
    })
}

fn checked_i64_from_u64(value: u64, pointer: &str) -> Result<i64> {
    i64::try_from(value).map_err(|_| CanonicalizeError::IntegerKeywordOutOfRange {
        pointer: pointer.to_owned(),
        keyword: last_pointer_token(pointer),
    })
}

fn preserved_meta(schema: &Map<String, Value>) -> Map<String, Value> {
    let mut out = Map::new();
    for key in PRESERVED_SCHEMA_METADATA_KEYS {
        if let Some(value) = schema.get(key) {
            out.insert(key.to_owned(), value.clone());
        }
    }
    sorted_object(out)
}

fn unsatisfiable_object(schema: &Map<String, Value>) -> Map<String, Value> {
    let mut out = preserved_terminal_meta(schema);
    out.insert("not".to_owned(), Value::Bool(true));
    sorted_object(out)
}

fn preserved_terminal_meta(schema: &Map<String, Value>) -> Map<String, Value> {
    let mut out = Map::new();
    for key in TERMINAL_SCHEMA_METADATA_KEYS {
        if let Some(value) = schema.get(key) {
            out.insert(key.to_owned(), value.clone());
        }
    }
    sorted_object(out)
}

fn should_strip_keyword(key: &str) -> bool {
    matches!(
        key,
        "$comment"
            | "description"
            | "default"
            | "examples"
            | "deprecated"
            | "readOnly"
            | "writeOnly"
            | "contentEncoding"
            | "contentMediaType"
            | "contentSchema"
    )
}

fn is_known_keyword(key: &str) -> bool {
    matches!(
        key,
        "$schema"
            | "$id"
            | "$anchor"
            | "$dynamicAnchor"
            | "$ref"
            | "$dynamicRef"
            | "$defs"
            | "$vocabulary"
            | "definitions"
            | "title"
            | "type"
            | "enum"
            | "const"
            | "allOf"
            | "anyOf"
            | "oneOf"
            | "not"
            | "if"
            | "then"
            | "else"
            | "properties"
            | "patternProperties"
            | "required"
            | "additionalProperties"
            | "propertyNames"
            | "minProperties"
            | "maxProperties"
            | "dependentRequired"
            | "dependentSchemas"
            | "unevaluatedProperties"
            | "items"
            | "prefixItems"
            | "contains"
            | "minItems"
            | "maxItems"
            | "uniqueItems"
            | "minContains"
            | "maxContains"
            | "unevaluatedItems"
            | "minLength"
            | "maxLength"
            | "pattern"
            | "format"
            | "minimum"
            | "maximum"
            | "exclusiveMinimum"
            | "exclusiveMaximum"
            | "multipleOf"
            | "dependencies"
            | "additionalItems"
            | JSONCOMPAT_METADATA_KEY
    )
}

fn sorted_unique_json(values: &[Value]) -> Vec<Value> {
    let mut unique = Vec::new();
    for value in values.iter().map(canonicalize_json) {
        if !unique.iter().any(|existing| existing == &value) {
            unique.push(value);
        }
    }
    unique.sort_by(compare_json);
    unique
}

fn compare_json(left: &Value, right: &Value) -> Ordering {
    let left = serde_json::to_string(left).expect("JSON serialization cannot fail");
    let right = serde_json::to_string(right).expect("JSON serialization cannot fail");
    left.cmp(&right)
}

fn sorted_object(entries: Map<String, Value>) -> Map<String, Value> {
    let mut items = entries.into_iter().collect::<Vec<_>>();
    items.sort_by(|(left, _), (right, _)| left.cmp(right));
    items.into_iter().collect()
}

fn join_pointer(base: &str, token: &str) -> String {
    if base == "#" {
        format!("#/{}", escape_pointer_token(token))
    } else {
        format!("{base}/{}", escape_pointer_token(token))
    }
}

fn escape_pointer_token(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

#[cfg(test)]
mod tests {
    use super::{PrimitiveType, PrimitiveTypeSet, primitive_type_set, value_type_name};
    use serde_json::json;

    #[test]
    fn primitive_type_set_detects_integer_number_overlap() {
        let integer = primitive_type_set("integer").expect("integer type set");
        let number = primitive_type_set("number").expect("number type set");
        let string = primitive_type_set("string").expect("string type set");

        assert!(number.intersects(integer));
        assert!(integer.intersects(number));
        assert!(!string.intersects(integer));
        assert!(!integer.intersects(string));
    }

    #[test]
    fn primitive_type_set_is_a_compact_union() {
        let mut types = PrimitiveTypeSet::from(PrimitiveType::String);
        types |= PrimitiveTypeSet::from(PrimitiveType::Boolean);

        assert!(types.intersects(PrimitiveType::String.into()));
        assert!(types.intersects(PrimitiveType::Boolean.into()));
        assert!(!types.intersects(PrimitiveType::Null.into()));
    }

    #[test]
    fn value_type_name_treats_integral_json_numbers_as_integer() {
        assert_eq!(value_type_name(&json!(1.0)), Some(PrimitiveType::Integer));
        assert_eq!(value_type_name(&json!(1.5)), Some(PrimitiveType::Number));
    }
}

#[cfg(test)]
mod integration_tests;
