//! Portable validation programs prepared before Python application startup.
//!
//! The wire format contains node indices, never native addresses or Python
//! objects. Loading checks its version and every edge without compiling schemas.
//! Unsupported vocabulary is rejected by the builder, never silently ignored.

use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

pub mod instance;
use instance::{ArrayView, InstanceRef, InstanceView, NumberView, ObjectView};
use json_schema_ast::Decimal;
use num_cmp::NumCmp;
use num_traits::{FromPrimitive, ToPrimitive};
use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};

mod annotations;
mod diagnostics;
pub use diagnostics::{ValidationFailure, ValidationFailureKind};
mod pattern;
use pattern::Pattern;

const VERSION: u32 = 4;
const MAX_DEPTH: usize = 512;

#[derive(Clone, Copy, PartialEq, Deserialize, Serialize)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, PartialEq, Deserialize, Serialize)]
pub struct PatternId(usize);

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedSchema {
    version: u32,
    nodes: Vec<Node>,
    patterns: Vec<Arc<Pattern>>,
    pattern_sources: Vec<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    exact_json_numbers: bool,
}

/// A compiled pattern shared by all generated programs that reference it.
/// The identifier covers the source and portable automata, including both byte orders.
pub struct SharedPattern {
    identifier: String,
    source: String,
    pattern: Arc<Pattern>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PatternData {
    source: String,
    pattern: Arc<Pattern>,
}

impl SharedPattern {
    pub fn load(bytes: &[u8]) -> Result<Self, String> {
        let data: PatternData = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        Ok(Self {
            identifier: format!("{:x}", Sha256::digest(bytes)),
            source: data.source,
            pattern: data.pattern,
        })
    }

    pub fn identifier(&self) -> &str {
        &self.identifier
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SharedProgram {
    version: u32,
    nodes: Vec<Node>,
    pattern_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    exact_json_numbers: bool,
}

/// Build-time pool. Source identity avoids serializing a large automaton more
/// than once; content identities keep references stable as other models change.
#[derive(Default)]
pub(crate) struct PatternPool {
    identifiers: HashMap<String, String>,
    pub(crate) data: BTreeMap<String, Vec<u8>>,
}

impl PatternPool {
    pub(crate) fn program(
        &mut self,
        schema: &PreparedSchema,
    ) -> Result<(Vec<u8>, Vec<String>), String> {
        if schema.patterns.is_empty() {
            return schema.to_bytes().map(|bytes| (bytes, Vec::new()));
        }
        let mut ids = Vec::new();
        for (source, pattern) in schema.pattern_sources.iter().zip(&schema.patterns) {
            let id = if let Some(id) = self.identifiers.get(source) {
                id.clone()
            } else {
                let data = serde_json::to_vec(&PatternData {
                    source: source.clone(),
                    pattern: Arc::clone(pattern),
                })
                .map_err(|e| e.to_string())?;
                let id = format!("{:x}", Sha256::digest(&data));
                self.data.insert(id.clone(), data);
                self.identifiers.insert(source.clone(), id.clone());
                id
            };
            ids.push(id);
        }
        let bytes = serde_json::to_vec(&SharedProgram {
            version: VERSION,
            nodes: schema.nodes.clone(),
            pattern_ids: ids.clone(),
            exact_json_numbers: schema.exact_json_numbers,
        })
        .map_err(|e| e.to_string())?;
        Ok((bytes, ids))
    }
}

#[derive(Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub types: Option<Vec<JsonType>>,
    pub choices: Option<Vec<Value>>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub constant: bool,
    pub rules: Vec<Rule>,
}

#[derive(Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JsonType {
    Null,
    Boolean,
    Integer,
    Number,
    String,
    Array,
    Object,
}

impl JsonType {
    #[inline]
    pub fn accepts<'a>(self, value: impl InstanceView<'a>) -> bool {
        match self {
            Self::Null => value.is_null(),
            Self::Boolean => value.is_boolean(),
            Self::Integer => value
                .as_number()
                .is_some_and(|number| match number.serde() {
                    Some(number) if !number.is_i64() && !number.is_u64() => {
                        exact_instance_number(value).is_some_and(|value| value.is_integer())
                    }
                    _ => number.is_integer(),
                }),
            Self::Number => value.is_number(),
            Self::String => value.is_string(),
            Self::Array => value.is_array(),
            Self::Object => value.is_object(),
        }
    }
}

#[derive(Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Rule {
    False,
    Ref {
        node: NodeId,
        pointer: String,
    },
    All(Vec<NodeId>),
    Any(Vec<NodeId>),
    One(Vec<NodeId>),
    Not(NodeId),
    If {
        condition: NodeId,
        then_node: Option<NodeId>,
        else_node: Option<NodeId>,
    },
    Object {
        properties: Vec<(String, NodeId)>,
        patterns: Vec<(PatternId, NodeId)>,
        required: Vec<String>,
        additional: Option<NodeId>,
    },
    PropertyNames(NodeId),
    Dependencies(Vec<(String, Vec<String>)>),
    DependentSchemas(Vec<(String, NodeId)>),
    Array {
        prefix: Vec<NodeId>,
        items: Option<NodeId>,
    },
    Contains {
        node: NodeId,
        min: u64,
        max: Option<u64>,
    },
    Unique,
    MultipleOf(Decimal),
    UnevaluatedProperties(NodeId),
    UnevaluatedItems(NodeId),
    Pattern(PatternId),
    StringLength {
        min: u64,
        max: Option<u64>,
    },
    ArrayLength {
        min: u64,
        max: Option<u64>,
    },
    ObjectLength {
        min: u64,
        max: Option<u64>,
    },
    Bound {
        value: Number,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        exact: Option<Decimal>,
        lower: bool,
        exclusive: bool,
    },
}

impl Node {
    pub fn is_leaf(&self) -> bool {
        self.rules.iter().all(|rule| {
            matches!(
                rule,
                Rule::False | Rule::StringLength { .. } | Rule::Bound { .. } | Rule::MultipleOf(_)
            )
        })
    }

    #[inline]
    pub fn accepts_leaf_view<'a>(&self, value: impl InstanceView<'a>) -> bool {
        self.types
            .as_ref()
            .is_none_or(|types| types.iter().any(|kind| kind.accepts(value)))
            && self
                .choices
                .as_ref()
                .is_none_or(|choices| choices.iter().any(|choice| value.equals(choice)))
            && self.rules.iter().all(|rule| match rule {
                Rule::False => false,
                Rule::StringLength { min, max } => value
                    .as_str()
                    .is_none_or(|value| string_length_valid(value, *min, *max)),
                Rule::Bound {
                    value: limit,
                    exact,
                    lower,
                    exclusive,
                } => value.as_number().is_none_or(|_| {
                    compare_bound(value, limit, exact.as_ref()).is_some_and(|order| match order {
                        Ordering::Equal => !exclusive,
                        Ordering::Greater => *lower,
                        Ordering::Less => !lower,
                    })
                }),
                Rule::MultipleOf(divisor) => multiple_of(value, divisor),
                _ => unreachable!("leaf guards contain only scalar constraints"),
            })
    }
}

fn multiple_of<'a>(value: impl InstanceView<'a>, divisor: &Decimal) -> bool {
    value.as_number().is_none_or(|number| {
        // Keep machine-sized integer divisibility exact and allocation-free.
        if divisor.is_integer()
            && let Some(divisor) = divisor.to_u64().filter(|value| *value != 0)
        {
            if let Some(number) = number.as_i64() {
                return i128::from(number) % i128::from(divisor) == 0;
            }
            if let Some(number) = number.as_u64() {
                return number % divisor == 0;
            }
        }
        exact_instance_number(value).is_some_and(|number| number.is_multiple_of(divisor))
    })
}

impl Rule {
    /// Rewrite every graph edge, including edges under annotation applicators.
    fn map_nodes(&mut self, mut map: impl FnMut(NodeId) -> NodeId) {
        match self {
            Self::Ref { node, .. }
            | Self::Not(node)
            | Self::PropertyNames(node)
            | Self::Contains { node, .. }
            | Self::UnevaluatedProperties(node)
            | Self::UnevaluatedItems(node) => *node = map(*node),
            Self::All(nodes) | Self::Any(nodes) | Self::One(nodes) => {
                nodes.iter_mut().for_each(|node| *node = map(*node));
            }
            Self::If {
                condition,
                then_node,
                else_node,
            } => {
                *condition = map(*condition);
                for node in then_node.iter_mut().chain(else_node.iter_mut()) {
                    *node = map(*node);
                }
            }
            Self::Object {
                properties,
                patterns,
                additional,
                ..
            } => {
                for (_, node) in properties {
                    *node = map(*node);
                }
                for (_, node) in patterns {
                    *node = map(*node);
                }
                if let Some(node) = additional {
                    *node = map(*node);
                }
            }
            Self::DependentSchemas(entries) => {
                for (_, node) in entries {
                    *node = map(*node);
                }
            }
            Self::Array { prefix, items } => {
                for node in prefix.iter_mut().chain(items.iter_mut()) {
                    *node = map(*node);
                }
            }
            Self::False
            | Self::Dependencies(_)
            | Self::Unique
            | Self::MultipleOf(_)
            | Self::Pattern(_)
            | Self::StringLength { .. }
            | Self::ArrayLength { .. }
            | Self::ObjectLength { .. }
            | Self::Bound { .. } => {}
        }
    }

    fn edges_valid(&self, node_count: usize) -> bool {
        let valid = |node: NodeId| node.0 < node_count;
        match self {
            Self::Ref { node, .. }
            | Self::Not(node)
            | Self::PropertyNames(node)
            | Self::Contains { node, .. }
            | Self::UnevaluatedProperties(node)
            | Self::UnevaluatedItems(node) => valid(*node),
            Self::All(nodes) | Self::Any(nodes) | Self::One(nodes) => {
                nodes.iter().copied().all(valid)
            }
            Self::If {
                condition,
                then_node,
                else_node,
            } => std::iter::once(*condition)
                .chain(*then_node)
                .chain(*else_node)
                .all(valid),
            Self::Object {
                properties,
                patterns,
                additional,
                ..
            } => properties
                .iter()
                .map(|(_, node)| *node)
                .chain(patterns.iter().map(|(_, node)| *node))
                .chain(*additional)
                .all(valid),
            Self::DependentSchemas(entries) => entries.iter().all(|(_, node)| valid(*node)),
            Self::Array { prefix, items } => prefix.iter().copied().chain(*items).all(valid),
            _ => true,
        }
    }
}

impl PreparedSchema {
    #[inline]
    pub fn node(&self, id: NodeId) -> &Node {
        &self.nodes[id.0]
    }
    /// Whether JSON must retain numeric lexemes until schema validation.
    pub fn requires_exact_json_numbers(&self) -> bool {
        self.exact_json_numbers
    }

    pub fn compile(schema: &Value) -> Result<Self, String> {
        // Validate at build time using the existing source-of-truth backend.
        let document = json_schema_ast::SchemaDocument::from_json(schema)
            .map_err(|error| error.to_string())?;
        document.root().map_err(|error| error.to_string())?;
        document
            .validate_source_schema()
            .map_err(|error| error.to_string())?;
        let linked = document
            .linked_schema_json()
            .map_err(|error| error.to_string())?;
        let schema = linked.as_ref();
        let mut builder = Builder {
            root: schema,
            max_nodes: Some(100_000),
            nodes: Vec::new(),
            paths: HashMap::new(),
            leaves: HashMap::new(),
            patterns: Vec::new(),
            pattern_ids: HashMap::new(),
        };
        builder.add(schema, "#")?;
        let exact_json_numbers = requires_exact_numbers(&builder.nodes);
        Ok(Self {
            version: VERSION,
            exact_json_numbers,
            nodes: builder.nodes,
            pattern_sources: {
                let mut sources = vec![String::new(); builder.patterns.len()];
                for (source, id) in builder.pattern_ids {
                    sources[id.0] = source;
                }
                sources
            },
            patterns: builder.patterns,
        })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(self).map_err(|error| error.to_string())
    }

    pub fn load(bytes: &[u8]) -> Result<Self, String> {
        let program: Self = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
        program.checked()
    }

    /// Link already-loaded automata without parsing or compiling a regex again.
    pub fn load_shared(bytes: &[u8], patterns: &[&SharedPattern]) -> Result<Self, String> {
        let program: SharedProgram = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        if program.pattern_ids.len() != patterns.len()
            || program
                .pattern_ids
                .iter()
                .zip(patterns)
                .any(|(id, pattern)| id != pattern.identifier())
        {
            return Err("prepared schema pattern identity mismatch; regenerate models".into());
        }
        Self {
            version: program.version,
            nodes: program.nodes,
            exact_json_numbers: program.exact_json_numbers,
            patterns: patterns
                .iter()
                .map(|pattern| Arc::clone(&pattern.pattern))
                .collect(),
            pattern_sources: patterns
                .iter()
                .map(|pattern| pattern.source.clone())
                .collect(),
        }
        .checked()
    }

    fn checked(self) -> Result<Self, String> {
        let program = self;
        if program.version != VERSION
            || program.nodes.is_empty()
            || program.pattern_sources.len() != program.patterns.len()
        {
            return Err(
                "unsupported or empty prepared schema; rebuild the generated module".into(),
            );
        }
        for node in &program.nodes {
            for rule in &node.rules {
                if !rule.edges_valid(program.nodes.len()) {
                    return Err("prepared schema contains an invalid node reference".into());
                }
                let patterns_valid = match rule {
                    Rule::Pattern(id) => id.0 < program.patterns.len(),
                    Rule::Object { patterns, .. } => {
                        patterns.iter().all(|(id, _)| id.0 < program.patterns.len())
                    }
                    _ => true,
                };
                if !patterns_valid {
                    return Err("prepared schema contains an invalid pattern reference".into());
                }
                if let Rule::Object { properties, .. } = rule
                    && properties.windows(2).any(|pair| pair[0].0 >= pair[1].0)
                {
                    return Err("prepared object properties must be sorted and unique".into());
                }
            }
        }
        Ok(program)
    }

    pub fn is_valid_view<'a>(&self, value: impl InstanceView<'a>) -> bool {
        value.is_json() && self.is_valid_json_view(value)
    }

    pub fn is_valid_json_view<'a>(&self, value: impl InstanceView<'a>) -> bool {
        let mut context = Evaluation::default();
        let valid = self.accepts(NodeId(0), value, 0, &mut context);
        valid && !context.incomplete
    }

    fn pattern_matches(&self, id: PatternId, text: &str, context: &mut Evaluation) -> bool {
        match self.patterns[id.0].is_match(text) {
            Some(matched) => matched,
            None => {
                // A resource limit must not turn into a successful negation,
                // conditional branch, or additional-property fallback.
                context.incomplete = true;
                false
            }
        }
    }

    fn accepts<'a>(
        &self,
        id: NodeId,
        value: impl InstanceView<'a>,
        depth: usize,
        context: &mut Evaluation,
    ) -> bool {
        let identity = (id.0, context.instance_depth);
        if context.active.contains(&identity) {
            return true;
        }
        if depth >= MAX_DEPTH {
            context.incomplete = true;
            return false;
        }
        let node = &self.nodes[id.0];
        if node
            .types
            .as_ref()
            .is_some_and(|types| !types.iter().any(|kind| kind.accepts(value)))
        {
            return false;
        }
        if node
            .choices
            .as_ref()
            .is_some_and(|choices| !choices.iter().any(|choice| value.equals(choice)))
        {
            return false;
        }
        context.active.push(identity);
        let valid = node
            .rules
            .iter()
            .filter(|rule| {
                !matches!(
                    rule,
                    Rule::UnevaluatedProperties(_) | Rule::UnevaluatedItems(_)
                )
            })
            .all(|rule| self.accepts_rule(rule, value, depth + 1, context))
            && node.rules.iter().all(|rule| match rule {
                Rule::UnevaluatedProperties(child) => value.as_object().is_none_or(|object| {
                    object.iter().all(|(key, item)| {
                        self.evaluates(
                            id,
                            value,
                            annotations::Location::Property(key),
                            depth + 1,
                            context,
                            &mut Vec::new(),
                            false,
                        ) || self.accepts_child(*child, item, depth + 1, context)
                    })
                }),
                Rule::UnevaluatedItems(child) => value.as_array().is_none_or(|array| {
                    array.iter().enumerate().all(|(index, item)| {
                        self.evaluates(
                            id,
                            value,
                            annotations::Location::Item(index),
                            depth + 1,
                            context,
                            &mut Vec::new(),
                            false,
                        ) || self.accepts_child(*child, item, depth + 1, context)
                    })
                }),
                _ => true,
            });
        context.active.pop();
        valid
    }

    fn accepts_child<'a>(
        &self,
        node: NodeId,
        value: impl InstanceView<'a>,
        depth: usize,
        context: &mut Evaluation,
    ) -> bool {
        context.instance_depth += 1;
        let valid = self.accepts(node, value, depth, context);
        context.instance_depth -= 1;
        valid
    }

    fn accepts_rule<'a>(
        &self,
        rule: &Rule,
        value: impl InstanceView<'a>,
        depth: usize,
        context: &mut Evaluation,
    ) -> bool {
        match rule {
            Rule::False => false,
            Rule::Ref { node, .. } => self.accepts(*node, value, depth, context),
            Rule::All(nodes) => nodes
                .iter()
                .all(|node| self.accepts(*node, value, depth, context)),
            Rule::Any(nodes) => nodes
                .iter()
                .any(|node| self.accepts(*node, value, depth, context)),
            Rule::One(nodes) => {
                let mut matched = false;
                for node in nodes {
                    if self.accepts(*node, value, depth, context) {
                        if matched {
                            return false;
                        }
                        matched = true;
                    }
                }
                matched
            }
            Rule::Not(node) => !self.accepts(*node, value, depth, context),
            Rule::If {
                condition,
                then_node,
                else_node,
            } => {
                let selected = if self.accepts(*condition, value, depth, context) {
                    then_node
                } else {
                    else_node
                };
                selected.is_none_or(|node| self.accepts(node, value, depth, context))
            }
            Rule::Object {
                properties,
                patterns,
                required,
                additional,
            } => {
                let Some(object) = value.as_object() else {
                    return true;
                };
                if required.iter().any(|name| !object.contains_key(name)) {
                    return false;
                }
                object.iter().all(|(name, value)| {
                    let mut matched = false;
                    if let Ok(index) =
                        properties.binary_search_by(|(candidate, _)| candidate.as_str().cmp(name))
                    {
                        matched = true;
                        if !self.accepts_child(properties[index].1, value, depth, context) {
                            return false;
                        }
                    }
                    for (pattern, node) in patterns {
                        if self.pattern_matches(*pattern, name, context) {
                            matched = true;
                            if !self.accepts_child(*node, value, depth, context) {
                                return false;
                            }
                        }
                    }
                    matched
                        || additional
                            .is_none_or(|node| self.accepts_child(node, value, depth, context))
                })
            }
            Rule::Pattern(id) => value
                .as_str()
                .is_none_or(|value| self.pattern_matches(*id, value, context)),
            Rule::PropertyNames(node) => value.as_object().is_none_or(|object| {
                object.keys().all(|key| {
                    self.accepts_child(
                        *node,
                        InstanceRef::from_serde(&Value::String(key.to_owned())),
                        depth,
                        context,
                    )
                })
            }),
            Rule::Dependencies(entries) => value.as_object().is_none_or(|object| {
                entries.iter().all(|(key, required)| {
                    !object.contains_key(key)
                        || required.iter().all(|name| object.contains_key(name))
                })
            }),
            Rule::DependentSchemas(entries) => value.as_object().is_none_or(|object| {
                entries.iter().all(|(key, node)| {
                    !object.contains_key(key) || self.accepts(*node, value, depth, context)
                })
            }),
            Rule::Array { prefix, items } => value.as_array().is_none_or(|array| {
                array.iter().enumerate().all(|(index, value)| {
                    prefix
                        .get(index)
                        .copied()
                        .or(*items)
                        .is_none_or(|node| self.accepts_child(node, value, depth, context))
                })
            }),
            Rule::Contains { node, min, max } => value.as_array().is_none_or(|array| {
                let count = array
                    .iter()
                    .filter(|value| self.accepts_child(*node, *value, depth, context))
                    .count() as u64;
                count >= *min && max.is_none_or(|max| count <= max)
            }),
            Rule::UnevaluatedProperties(_) | Rule::UnevaluatedItems(_) => {
                unreachable!("handled at the containing node")
            }
            Rule::MultipleOf(divisor) => multiple_of(value, divisor),
            Rule::Unique => value.as_array().is_none_or(|array| {
                let mut seen = Vec::with_capacity(array.len());
                for value in array.iter() {
                    if seen.iter().any(|previous| value.equals(previous)) {
                        return false;
                    }
                    seen.push(value.to_owned());
                }
                true
            }),
            Rule::StringLength { min, max } => value
                .as_str()
                .is_none_or(|value| string_length_valid(value, *min, *max)),
            Rule::ArrayLength { min, max } => value
                .as_array()
                .is_none_or(|value| length_valid(value.len(), *min, *max)),
            Rule::ObjectLength { min, max } => value
                .as_object()
                .is_none_or(|value| length_valid(value.len(), *min, *max)),
            Rule::Bound {
                value: limit,
                exact,
                lower,
                exclusive,
            } => value.as_number().is_none_or(|_| {
                let comparison = compare_bound(value, limit, exact.as_ref());
                comparison.is_some_and(|order| {
                    if order == Ordering::Equal {
                        !exclusive
                    } else if *lower {
                        order == Ordering::Greater
                    } else {
                        order == Ordering::Less
                    }
                })
            }),
        }
    }
}

#[derive(Default)]
struct Evaluation {
    active: Vec<(usize, usize)>,
    instance_depth: usize,
    incomplete: bool,
}

pub fn string_length_valid(value: &str, min: u64, max: Option<u64>) -> bool {
    if max.is_none() && min <= 1 {
        return min == 0 || !value.is_empty();
    }
    length_valid(value.chars().count(), min, max)
}

fn length_valid(length: usize, min: u64, max: Option<u64>) -> bool {
    let length = length as u64;
    length >= min && max.is_none_or(|max| length <= max)
}

fn needs_exact_json_number(value: &Value) -> bool {
    match value {
        Value::Number(number) => number.as_i64().is_none() && number.as_u64().is_none(),
        Value::Array(values) => values.iter().any(needs_exact_json_number),
        Value::Object(values) => values.values().any(needs_exact_json_number),
        _ => false,
    }
}

// Persist exact large/decimal boundaries in the build artifact. Ordinary
// machine-sized integer constraints keep their allocation-free fast path.
fn exact_decimal(text: &str) -> Option<Decimal> {
    Decimal::try_from(text.to_owned()).ok()
}

fn compare_bound<'a>(
    value: impl InstanceView<'a>,
    limit: &Number,
    exact: Option<&Decimal>,
) -> Option<Ordering> {
    let number = value.as_number()?;
    let Some(exact) = exact else {
        return compare_number(number, limit);
    };
    let number = exact_instance_number(value)?;
    number.partial_cmp(exact)
}

fn exact_instance_number<'a>(value: impl InstanceView<'a>) -> Option<Decimal> {
    let number = value.as_number()?;
    Some(if let Some(value) = number.as_i64() {
        Decimal::from_i64(value)
    } else if let Some(value) = number.as_u64() {
        Decimal::from_u64(value)
    } else {
        match number.big_integer() {
            Some(value) => exact_decimal(&value.to_string())?,
            // Use the decimal representation of a Python float, just as the
            // public value validator does; preserve raw JSON decimal lexemes.
            _ => exact_decimal(&value.to_owned().to_string())?,
        }
    })
}

fn compare_number<'a>(value: impl NumberView<'a>, limit: &Number) -> Option<Ordering> {
    if let Some(number) = value.serde()
        && !number.is_i64()
        && !number.is_u64()
    {
        return exact_decimal(&number.to_string())?
            .partial_cmp(&exact_decimal(&limit.to_string())?);
    }
    macro_rules! compare {
        ($value:expr) => {{
            let value = $value;
            if let Some(limit) = limit.as_i64() {
                value.num_cmp(limit)
            } else if let Some(limit) = limit.as_u64() {
                value.num_cmp(limit)
            } else {
                limit.as_f64().and_then(|limit| value.num_cmp(limit))
            }
        }};
    }
    if let Some(value) = value.big_integer() {
        if let Some(value) = value.to_i64() {
            return compare!(value);
        }
        if let Some(value) = value.to_u64() {
            return compare!(value);
        }
        let limit = limit.as_f64()?;
        let integer_limit = num_bigint::BigInt::from_f64(limit)?;
        return match value.cmp(&integer_limit) {
            Ordering::Equal if limit.fract() > 0.0 => Some(Ordering::Less),
            Ordering::Equal if limit.fract() < 0.0 => Some(Ordering::Greater),
            order => Some(order),
        };
    }
    if let Some(value) = value.as_i64() {
        compare!(value)
    } else if let Some(value) = value.as_u64() {
        compare!(value)
    } else {
        value.as_f64().and_then(|value| compare!(value))
    }
}

pub fn compare_integer(value: i64, limit: &Number) -> Option<Ordering> {
    if let Some(limit) = limit.as_i64() {
        Some(value.cmp(&limit))
    } else if let Some(limit) = limit.as_u64() {
        value.num_cmp(limit)
    } else {
        limit.as_f64().and_then(|limit| value.num_cmp(limit))
    }
}

struct Builder<'a> {
    root: &'a Value,
    max_nodes: Option<usize>,
    nodes: Vec<Node>,
    paths: HashMap<String, NodeId>,
    leaves: HashMap<Vec<u8>, NodeId>,
    patterns: Vec<Arc<Pattern>>,
    pattern_ids: HashMap<String, PatternId>,
}

impl Builder<'_> {
    fn pattern(&mut self, pattern: &str) -> Result<PatternId, String> {
        if let Some(id) = self.pattern_ids.get(pattern) {
            return Ok(*id);
        }
        let id = PatternId(self.patterns.len());
        self.patterns.push(Arc::new(Pattern::compile(pattern)?));
        self.pattern_ids.insert(pattern.to_owned(), id);
        Ok(id)
    }

    fn add(&mut self, schema: &Value, path: &str) -> Result<NodeId, String> {
        if let Some(id) = self.paths.get(path) {
            return Ok(*id);
        }
        if self
            .max_nodes
            .is_some_and(|limit| self.nodes.len() >= limit)
        {
            return Err("prepared schema exceeds 100,000 nodes".into());
        }
        let id = NodeId(self.nodes.len());
        self.paths.insert(path.to_owned(), id);
        self.nodes.push(Node {
            types: None,
            choices: None,
            constant: false,
            rules: Vec::new(),
        });
        let node = self.node(schema, path)?;
        // Leaf checks have no outgoing edges, so no recursive reference can
        // have observed this provisional slot. Intern only completed leaves
        // and only when no child allocations followed the placeholder.
        if node.is_leaf() && id.0 + 1 == self.nodes.len() {
            let key = serde_json::to_vec(&node).map_err(|error| error.to_string())?;
            if let Some(existing) = self.leaves.get(&key).copied() {
                self.nodes.pop();
                self.paths.insert(path.to_owned(), existing);
                return Ok(existing);
            }
            self.leaves.insert(key, id);
        }
        self.nodes[id.0] = node;
        Ok(id)
    }

    fn child(&mut self, schema: &Value, path: &str, key: &str) -> Result<NodeId, String> {
        self.add(
            schema,
            &format!("{path}/{}", key.replace('~', "~0").replace('/', "~1")),
        )
    }

    fn node(&mut self, schema: &Value, path: &str) -> Result<Node, String> {
        let mut node = Node {
            types: None,
            choices: None,
            constant: false,
            rules: Vec::new(),
        };
        match schema {
            Value::Bool(true) => return Ok(node),
            Value::Bool(false) => {
                node.rules.push(Rule::False);
                return Ok(node);
            }
            _ => {}
        }
        let object = schema
            .as_object()
            .ok_or_else(|| format!("{path}: expected a schema"))?;
        for key in object.keys() {
            if matches!(
                key.as_str(),
                "$dynamicRef" | "$recursiveRef" | "$vocabulary" | "x-jsoncompat-format-assertion"
            ) || (key == "$id" && path != "#")
            {
                return Err(format!(
                    "{path}/{key}: not supported by generated dataclasses"
                ));
            }
        }
        if let Some(types) = object.get("type") {
            node.types = Some(match types {
                Value::Array(values) => values
                    .iter()
                    .map(|value| {
                        serde_json::from_value(value.clone()).map_err(|error| error.to_string())
                    })
                    .collect::<Result<_, _>>()?,
                value => {
                    vec![serde_json::from_value(value.clone()).map_err(|error| error.to_string())?]
                }
            });
        }
        if let Some(choices) = object.get("enum") {
            node.choices = Some(choices.as_array().ok_or("invalid enum")?.clone());
        }
        if let Some(value) = object.get("const") {
            node.constant = true;
            if node.choices.as_ref().is_some_and(|choices| {
                !choices
                    .iter()
                    .any(|choice| InstanceRef::from_serde(value).equals(choice))
            }) {
                node.rules.push(Rule::False);
            }
            node.choices = Some(vec![value.clone()]);
        }
        if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
            let pointer = reference
                .strip_prefix('#')
                .filter(|pointer| pointer.is_empty() || pointer.starts_with('/'))
                .ok_or_else(|| {
                    format!("{path}/$ref: only local JSON Pointer references can be prepared")
                })?;
            let pointer = percent_encoding::percent_decode_str(pointer)
                .decode_utf8()
                .map_err(|error| error.to_string())?;
            // The immutable document outlives the builder, so following an
            // edge need not clone the referenced subtree (or its definitions).
            let root = self.root;
            let target = root
                .pointer(&pointer)
                .ok_or_else(|| format!("unresolved reference {reference}"))?;
            node.rules.push(Rule::Ref {
                node: self.add(target, &format!("#{pointer}"))?,
                pointer: pointer.into_owned(),
            });
        }
        for key in ["allOf", "anyOf", "oneOf"] {
            if let Some(values) = object.get(key).and_then(Value::as_array) {
                let children = values
                    .iter()
                    .enumerate()
                    .map(|(index, value)| {
                        self.child(value, &format!("{path}/{key}"), &index.to_string())
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                node.rules.push(match key {
                    "allOf" => Rule::All(children),
                    "anyOf" => Rule::Any(children),
                    _ => Rule::One(children),
                });
            }
        }
        if let Some(value) = object.get("not") {
            node.rules.push(Rule::Not(self.child(value, path, "not")?));
        }
        if let Some(value) = object.get("if") {
            node.rules.push(Rule::If {
                condition: self.child(value, path, "if")?,
                then_node: object
                    .get("then")
                    .map(|value| self.child(value, path, "then"))
                    .transpose()?,
                else_node: object
                    .get("else")
                    .map(|value| self.child(value, path, "else"))
                    .transpose()?,
            });
        }
        if object.contains_key("properties")
            || object.contains_key("patternProperties")
            || object.contains_key("required")
            || object.contains_key("additionalProperties")
        {
            let mut properties = Vec::new();
            let mut patterns = Vec::new();
            if let Some(values) = object.get("properties").and_then(Value::as_object) {
                for (key, value) in values {
                    properties.push((
                        key.clone(),
                        self.child(value, &format!("{path}/properties"), key)?,
                    ));
                }
            }
            properties.sort_unstable_by(|a, b| a.0.cmp(&b.0));
            if let Some(values) = object.get("patternProperties").and_then(Value::as_object) {
                for (pattern, value) in values {
                    patterns.push((
                        self.pattern(pattern)?,
                        self.child(value, &format!("{path}/patternProperties"), pattern)?,
                    ));
                }
            }
            let required = object
                .get("required")
                .map(|value| {
                    serde_json::from_value(value.clone()).map_err(|error| error.to_string())
                })
                .transpose()?
                .unwrap_or_default();
            let additional = object
                .get("additionalProperties")
                .map(|value| self.child(value, path, "additionalProperties"))
                .transpose()?;
            node.rules.push(Rule::Object {
                properties,
                patterns,
                required,
                additional,
            });
        }
        if let Some(pattern) = object.get("pattern").and_then(Value::as_str) {
            node.rules.push(Rule::Pattern(self.pattern(pattern)?));
        }
        if let Some(value) = object.get("propertyNames") {
            node.rules.push(Rule::PropertyNames(self.child(
                value,
                path,
                "propertyNames",
            )?));
        }
        if let Some(value) = object.get("dependentRequired") {
            let entries: std::collections::BTreeMap<String, Vec<String>> =
                serde_json::from_value(value.clone()).map_err(|error| error.to_string())?;
            node.rules
                .push(Rule::Dependencies(entries.into_iter().collect()));
        }
        if let Some(values) = object.get("dependencies").and_then(Value::as_object) {
            let mut required = Vec::new();
            let mut schemas = Vec::new();
            for (key, value) in values {
                if value.is_array() {
                    required.push((
                        key.clone(),
                        serde_json::from_value(value.clone()).map_err(|error| error.to_string())?,
                    ));
                } else {
                    schemas.push((
                        key.clone(),
                        self.child(value, &format!("{path}/dependencies"), key)?,
                    ));
                }
            }
            if !required.is_empty() {
                node.rules.push(Rule::Dependencies(required));
            }
            if !schemas.is_empty() {
                node.rules.push(Rule::DependentSchemas(schemas));
            }
        }
        if let Some(values) = object.get("dependentSchemas").and_then(Value::as_object) {
            let entries = values
                .iter()
                .map(|(key, value)| {
                    Ok((
                        key.clone(),
                        self.child(value, &format!("{path}/dependentSchemas"), key)?,
                    ))
                })
                .collect::<Result<_, String>>()?;
            node.rules.push(Rule::DependentSchemas(entries));
        }
        if object.contains_key("prefixItems") || object.contains_key("items") {
            let mut prefix = Vec::new();
            if let Some(values) = object.get("prefixItems").and_then(Value::as_array) {
                for (index, value) in values.iter().enumerate() {
                    prefix.push(self.child(
                        value,
                        &format!("{path}/prefixItems"),
                        &index.to_string(),
                    )?);
                }
            }
            let items = object
                .get("items")
                .map(|value| self.child(value, path, "items"))
                .transpose()?;
            node.rules.push(Rule::Array { prefix, items });
        }
        if let Some(value) = object.get("contains") {
            node.rules.push(Rule::Contains {
                node: self.child(value, path, "contains")?,
                min: count(object.get("minContains"))?.unwrap_or(1),
                max: count(object.get("maxContains"))?,
            });
        }
        for (keyword, properties) in [("unevaluatedProperties", true), ("unevaluatedItems", false)]
        {
            if let Some(value) = object.get(keyword) {
                let child = self.child(value, path, keyword)?;
                node.rules.push(if properties {
                    Rule::UnevaluatedProperties(child)
                } else {
                    Rule::UnevaluatedItems(child)
                });
            }
        }
        if let Some(Value::Number(value)) = object.get("multipleOf") {
            // Canonical integer schemas may spell the implicit integer
            // lattice as multipleOf: 1. It adds neither validation nor an
            // annotation, so omit it from the runtime program.
            if !(value.as_u64() == Some(1) && node.types.as_deref() == Some(&[JsonType::Integer])) {
                node.rules.push(Rule::MultipleOf(
                    exact_decimal(&value.to_string()).ok_or("unsupported multipleOf exponent")?,
                ));
            }
        }
        if object.get("uniqueItems") == Some(&Value::Bool(true)) {
            node.rules.push(Rule::Unique);
        }
        for (min_key, max_key) in [
            ("minLength", "maxLength"),
            ("minItems", "maxItems"),
            ("minProperties", "maxProperties"),
        ] {
            if object.contains_key(min_key) || object.contains_key(max_key) {
                let min = count(object.get(min_key))?.unwrap_or(0);
                let max = count(object.get(max_key))?;
                if min == 0 && max.is_none() {
                    continue;
                }
                node.rules.push(match min_key {
                    "minLength" => Rule::StringLength { min, max },
                    "minItems" => Rule::ArrayLength { min, max },
                    _ => Rule::ObjectLength { min, max },
                });
            }
        }
        for (key, lower, exclusive) in [
            ("minimum", true, false),
            ("maximum", false, false),
            ("exclusiveMinimum", true, true),
            ("exclusiveMaximum", false, true),
        ] {
            if let Some(Value::Number(value)) = object.get(key) {
                node.rules.push(Rule::Bound {
                    value: value.clone(),
                    exact: if value.as_i64().is_none() && value.as_u64().is_none() {
                        exact_decimal(&value.to_string())
                    } else {
                        None
                    },
                    lower,
                    exclusive,
                });
            }
        }
        Ok(node)
    }
}

fn count(value: Option<&Value>) -> Result<Option<u64>, String> {
    value
        .map(|value| {
            value
                .as_u64()
                .or_else(|| value.as_f64().and_then(|number| number.to_u64()))
                .ok_or_else(|| "count bound cannot be represented as u64".into())
        })
        .transpose()
}

fn requires_exact_numbers(nodes: &[Node]) -> bool {
    nodes.iter().any(|node| {
        node.rules.iter().any(|rule| {
            matches!(rule, Rule::Bound { exact: Some(_), .. })
                || matches!(rule, Rule::MultipleOf(divisor) if !divisor.is_integer())
        }) || node
            .choices
            .as_ref()
            .is_some_and(|choices| choices.iter().any(needs_exact_json_number))
    })
}

/// Build the shared definition graph once, then emit standalone programs for
/// individual model roots. This deliberately accepts only reference scopes
/// whose meaning is unchanged when a model is promoted to a document root and
/// receives the document's definition maps. Other schemas use `compile`.
pub(crate) struct PreparedSchemaContext<'a> {
    builder: Builder<'a>,
    schemas: HashMap<String, &'a Value>,
    model_roots: HashMap<String, NodeId>,
}

impl<'a> PreparedSchemaContext<'a> {
    pub(crate) fn new(
        document: &'a json_schema_ast::SchemaDocument,
    ) -> Result<Option<Self>, String> {
        Self::with_root(document, document.source_schema_json())
    }

    /// Package declarations already use this validated document's canonical
    /// graph. Reuse it without constructing and validating a second document.
    pub(crate) fn new_canonical(
        document: &'a json_schema_ast::SchemaDocument,
    ) -> Result<Option<Self>, String> {
        let Ok(root) = document.canonical_schema_json() else {
            return Ok(None);
        };
        Self::with_root(document, root)
    }

    fn with_root(
        document: &'a json_schema_ast::SchemaDocument,
        root: &'a Value,
    ) -> Result<Option<Self>, String> {
        let mut schemas = HashMap::new();
        let mut references = Vec::new();
        if !shared_schema_positions(root, "#", &mut schemas, &mut references)
            || references
                .iter()
                .any(|pointer| !schemas.contains_key(pointer))
        {
            return Ok(None);
        }
        // Keep exactly the build-time checks performed by standalone compile,
        // but perform them once for the complete shared document.
        if document.root().is_err() || document.validate_source_schema().is_err() {
            // A combined document can exceed an analysis budget even when its
            // individual models do not. Decline the optimization; standalone
            // compilation remains responsible for accepting or rejecting each
            // model, including validating every materialized source schema.
            return Ok(None);
        }
        Ok(Some(Self {
            builder: Builder {
                root,
                // The existing limit is per emitted program, not per package.
                max_nodes: None,
                nodes: Vec::new(),
                paths: HashMap::new(),
                leaves: HashMap::new(),
                patterns: Vec::new(),
                pattern_ids: HashMap::new(),
            },
            schemas,
            model_roots: HashMap::new(),
        }))
    }

    pub(crate) fn compile_at(&mut self, pointer: &str) -> Result<PreparedSchema, String> {
        let schema = self
            .schemas
            .get(pointer)
            .copied()
            .ok_or_else(|| format!("{pointer}: expected a validated schema position"))?;
        let root = if let Some(root) = self.model_roots.get(pointer) {
            *root
        } else {
            // The standalone model root and its inline children must remain
            // distinct from the same schema reached through a definition ref.
            // Merging them changes cycle detection and depth-budget behavior.
            // A non-pointer namespace cannot collide with local ref targets.
            let model_path = format!("model:{pointer}");
            let root = self.builder.add(schema, &model_path).map_err(|error| {
                error
                    .strip_prefix(&model_path)
                    .map_or_else(|| error.clone(), |suffix| format!("#{suffix}"))
            })?;
            self.model_roots.insert(pointer.to_owned(), root);
            root
        };
        self.extract(root)
    }

    fn extract(&self, root: NodeId) -> Result<PreparedSchema, String> {
        enum Dependency {
            Node(NodeId),
            Pattern(PatternId),
        }
        let mut pending = vec![Dependency::Node(root)];
        let mut ids = HashMap::new();
        let mut nodes = Vec::new();
        let mut pattern_ids = HashMap::new();
        let mut patterns = Vec::new();
        let mut pattern_sources = Vec::new();
        let mut source_patterns = vec![""; self.builder.patterns.len()];
        for (source, id) in &self.builder.pattern_ids {
            source_patterns[id.0] = source.as_str();
        }
        // Match the standalone compiler's depth-first allocation order. Besides
        // keeping generated artifacts stable, visiting regexes between their
        // preceding properties and own child schema preserves pattern IDs.
        while let Some(dependency) = pending.pop() {
            match dependency {
                Dependency::Pattern(old) => {
                    pattern_ids.entry(old.0).or_insert_with(|| {
                        let new = PatternId(patterns.len());
                        patterns.push(self.builder.patterns[old.0].clone());
                        pattern_sources.push(source_patterns[old.0].to_owned());
                        new
                    });
                }
                Dependency::Node(old) => {
                    if ids.contains_key(&old.0) {
                        continue;
                    }
                    if nodes.len() >= 100_000 {
                        return Err("prepared schema exceeds 100,000 nodes".into());
                    }
                    ids.insert(old.0, NodeId(nodes.len()));
                    let mut node = self.builder.nodes[old.0].clone();
                    let mut children = Vec::new();
                    for rule in &mut node.rules {
                        match rule {
                            Rule::Object {
                                properties,
                                patterns,
                                additional,
                                ..
                            } => {
                                children.extend(
                                    properties.iter().map(|(_, child)| Dependency::Node(*child)),
                                );
                                for (pattern, child) in patterns {
                                    children.push(Dependency::Pattern(*pattern));
                                    children.push(Dependency::Node(*child));
                                }
                                children.extend(additional.map(Dependency::Node));
                            }
                            Rule::Pattern(pattern) => children.push(Dependency::Pattern(*pattern)),
                            _ => rule.map_nodes(|child| {
                                children.push(Dependency::Node(child));
                                child
                            }),
                        }
                    }
                    nodes.push(node);
                    pending.extend(children.into_iter().rev());
                }
            }
        }
        for node in &mut nodes {
            for rule in &mut node.rules {
                rule.map_nodes(|old| ids[&old.0]);
                match rule {
                    Rule::Pattern(id) => *id = pattern_ids[&id.0],
                    Rule::Object { patterns, .. } => {
                        for (id, _) in patterns {
                            *id = pattern_ids[&id.0];
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(PreparedSchema {
            version: VERSION,
            exact_json_numbers: requires_exact_numbers(&nodes),
            nodes,
            patterns,
            pattern_sources,
        })
    }
}

/// Only visit actual schema positions. Annotation payloads can themselves
/// contain objects spelled `$ref`, `$id`, or `properties` without being schemas.
fn shared_schema_positions<'a>(
    schema: &'a Value,
    path: &str,
    schemas: &mut HashMap<String, &'a Value>,
    references: &mut Vec<String>,
) -> bool {
    schemas.insert(path.to_owned(), schema);
    let Some(object) = schema.as_object() else {
        return schema.is_boolean();
    };
    if [
        "$id",
        "$anchor",
        "$dynamicAnchor",
        "$dynamicRef",
        "$recursiveAnchor",
        "$recursiveRef",
        "$vocabulary",
        "x-jsoncompat-format-assertion",
    ]
    .iter()
    .any(|key| object.contains_key(*key))
    {
        return false;
    }
    if let Some(reference) = object.get("$ref") {
        let Some(reference) = reference.as_str().and_then(|value| value.strip_prefix('#')) else {
            return false;
        };
        let Ok(pointer) = percent_encoding::percent_decode_str(reference).decode_utf8() else {
            return false;
        };
        if !(pointer.starts_with("/$defs/") || pointer.starts_with("/definitions/")) {
            return false;
        }
        references.push(format!("#{pointer}"));
    }
    let mut child = |key: &str, value: &'a Value| {
        shared_schema_positions(value, &format!("{path}/{key}"), schemas, references)
    };
    for (key, value) in object {
        if json_schema_ast::is_single_schema_child_keyword(key) {
            if !child(key, value) {
                return false;
            }
        } else if json_schema_ast::is_schema_map_child_keyword(key) || key == "dependencies" {
            if let Some(values) = value.as_object() {
                for (name, value) in values {
                    if key == "dependencies" && value.is_array() {
                        continue;
                    }
                    let name = name.replace('~', "~0").replace('/', "~1");
                    if !child(&format!("{key}/{name}"), value) {
                        return false;
                    }
                }
            }
        } else if json_schema_ast::is_schema_array_child_keyword(key)
            && let Some(values) = value.as_array()
        {
            for (index, value) in values.iter().enumerate() {
                if !child(&format!("{key}/{index}"), value) {
                    return false;
                }
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_patterns_preserve_validation_diagnostics_and_native_identity() {
        let schemas = [
            serde_json::json!({"type":"object", "patternProperties":{"^x":{"type":"string", "pattern":"^(?=.{2,})[a-z]+$"}}, "additionalProperties":false}),
            serde_json::json!({"type":"array", "items":{"type":"string", "pattern":"^(?=.{2,})[a-z]+$"}}),
        ];
        let mut pool = PatternPool::default();
        let compiled = schemas
            .iter()
            .map(|schema| PreparedSchema::compile(schema).unwrap())
            .collect::<Vec<_>>();
        let programs = compiled
            .iter()
            .map(|schema| pool.program(schema).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(pool.data.len(), 2);
        let patterns = pool
            .data
            .iter()
            .map(|(id, data)| (id, SharedPattern::load(data).unwrap()))
            .collect::<BTreeMap<_, _>>();
        let values = serde_json::json!([null, {}, {"x":"ab"}, {"x":"A"}, {"x":3}, {"y":"ab"}, [], ["ab"], ["a"], ["A"], [3]]);
        for (original, (bytes, ids)) in compiled.iter().zip(&programs) {
            let handles = ids.iter().map(|id| &patterns[id]).collect::<Vec<_>>();
            let shared = PreparedSchema::load_shared(bytes, &handles).unwrap();
            for (pattern, handle) in shared.patterns.iter().zip(&handles) {
                assert!(Arc::ptr_eq(pattern, &handle.pattern));
            }
            for value in values.as_array().unwrap() {
                assert_eq!(
                    shared.is_valid_view(InstanceRef::from_serde(value)),
                    original.is_valid_view(InstanceRef::from_serde(value))
                );
                assert_eq!(shared.explain(value), original.explain(value));
            }
            assert!(PreparedSchema::load_shared(bytes, &[]).is_err());
            let wrong = SharedPattern::load(
                &serde_json::to_vec(&PatternData {
                    source: "different".into(),
                    pattern: Arc::clone(&handles[0].pattern),
                })
                .unwrap(),
            )
            .unwrap();
            let mut swapped = handles;
            swapped[0] = &wrong;
            assert!(
                PreparedSchema::load_shared(bytes, &swapped)
                    .err()
                    .unwrap()
                    .contains("identity mismatch")
            );
        }
        // Identifiers do not depend on the order programs enter the pool.
        let mut reversed = PatternPool::default();
        for schema in compiled.iter().rev() {
            reversed.program(schema).unwrap();
        }
        assert_eq!(pool.data, reversed.data);
        let (bytes, ids) = pool
            .program(&PreparedSchema::compile(&serde_json::json!({"type":"null"})).unwrap())
            .unwrap();
        assert!(ids.is_empty());
        assert!(
            PreparedSchema::load(&bytes)
                .unwrap()
                .is_valid_view(InstanceRef::from_serde(&Value::Null))
        );
    }

    #[test]
    fn shared_patterns_reject_corrupt_automata_and_program_edges() {
        let mut pool = PatternPool::default();
        let schema =
            PreparedSchema::compile(&serde_json::json!({"type":"string", "pattern":"^[a-z]+$"}))
                .unwrap();
        let (bytes, ids) = pool.program(&schema).unwrap();
        let data = &pool.data[&ids[0]];
        let handle = SharedPattern::load(data).unwrap();
        let mut corrupt: Value = serde_json::from_slice(data).unwrap();
        corrupt["pattern"]["dfa"]["little"] = serde_json::json!([0]);
        corrupt["pattern"]["dfa"]["big"] = serde_json::json!([0]);
        assert!(SharedPattern::load(&serde_json::to_vec(&corrupt).unwrap()).is_err());
        let mut program: Value = serde_json::from_slice(&bytes).unwrap();
        program["nodes"][0]["rules"][0]["pattern"] = serde_json::json!(99);
        assert!(
            PreparedSchema::load_shared(&serde_json::to_vec(&program).unwrap(), &[&handle])
                .is_err()
        );
        program["version"] = serde_json::json!(0);
        assert!(
            PreparedSchema::load_shared(&serde_json::to_vec(&program).unwrap(), &[&handle])
                .is_err()
        );
    }

    fn standalone_at(root: &Value, pointer: &str) -> Value {
        let mut schema = root
            .pointer(pointer.strip_prefix('#').unwrap())
            .unwrap()
            .clone();
        if let Some(object) = schema.as_object_mut() {
            for keyword in ["$defs", "definitions"] {
                if let Some(definitions) = root.get(keyword).and_then(Value::as_object) {
                    let local = object
                        .entry(keyword)
                        .or_insert_with(|| Value::Object(Default::default()));
                    local.as_object_mut().unwrap().extend(definitions.clone());
                }
            }
        }
        schema
    }

    #[test]
    fn shared_context_preserves_validation_and_diagnostics_for_every_root() {
        let schema = serde_json::json!({
            "$defs": {
                "Node": {"type": "object", "required": ["name"], "properties": {
                    "name": {"type": "string", "pattern": "^(?=.{2,})[a-z]+$"},
                    "next": {"anyOf": [{"type": "null"}, {"$ref": "#/$defs/Node"}]}
                }, "additionalProperties": false},
                "Number": {"type": "number", "minimum": 0.2, "maximum": 3.5, "multipleOf": 0.1},
                "Choice": {"oneOf": [{"const": 1}, {"enum": [2, 3]}, {"$ref": "#/$defs/Node"}]},
                "Array": {"type": "array", "prefixItems": [{"$ref": "#/$defs/Number"}],
                    "items": {"$ref": "#/$defs/Choice"}, "contains": {"type": "number"},
                    "minContains": 1, "maxContains": 2, "uniqueItems": true,
                    "minItems": 1, "maxItems": 4},
                "Annotations": {"type": "object", "allOf": [{"properties": {"a": {"type": "integer"}}}],
                    "anyOf": [{"properties": {"b": {"type": "boolean"}}}, {"required": ["a"]}],
                    "unevaluatedProperties": false},
                "Conditions": {"type": "object", "if": {"required": ["a"]},
                    "then": {"required": ["b"]}, "else": {"not": {"required": ["b"]}},
                    "dependentSchemas": {"a": {"properties": {"b": {"const": true}}}},
                    "dependentRequired": {"b": ["a"]}, "minProperties": 0, "maxProperties": 3},
                "Names": {"type": "object", "propertyNames": {"pattern": "^[a-z]+$"},
                    "patternProperties": {"^x": {"type": "integer"}},
                    "additionalProperties": {"type": "boolean"}},
                "Escaped/a~b": {"$ref": "#/%24defs/Number"},
                "Legacy": {"$ref": "#/definitions/Text"},
                "UnevaluatedArray": {"allOf": [{"prefixItems": [{"const": 1}]}], "unevaluatedItems": false}
            },
            "definitions": {"Text": {"type": "string", "minLength": 2, "maxLength": 3}},
            "type": "object"
        });
        let document = json_schema_ast::SchemaDocument::from_json(&schema).unwrap();
        let mut context = PreparedSchemaContext::new(&document).unwrap().unwrap();
        let values = serde_json::json!([
            null, false, true, 0, 0.1, 0.2, 0.3, 1, 2, 3, 3.5, 3.6, 4, "", "a", "ab", "abcd",
            [], [1], [0.3, 1, 2], [1, 1], [1, 2, 3], [1, "a"], {},
            {"a": 1}, {"b": true}, {"a": 1, "b": true}, {"a": 1, "b": false}, {"c": 1},
            {"x": 1}, {"x": false}, {"X": true}, {"name": "ab"}, {"name": "a"},
            {"name": "ab", "next": {"name": "cd", "next": null}},
            {"name": "ab", "next": {"name": "C"}}, {"name": "ab", "extra": false}
        ]);
        let mut pointers = context.schemas.keys().cloned().collect::<Vec<_>>();
        pointers.sort();
        for pointer in pointers {
            let standalone = standalone_at(&schema, &pointer);
            let expected = PreparedSchema::compile(&standalone).unwrap();
            let backend = json_schema_ast::compile(&standalone).unwrap();
            let shared = context.compile_at(&pointer).unwrap();
            assert_eq!(
                expected.to_bytes().unwrap(),
                shared.to_bytes().unwrap(),
                "{pointer}: standalone program layout"
            );
            let loaded = PreparedSchema::load(&shared.to_bytes().unwrap()).unwrap();
            assert_eq!(
                expected.requires_exact_json_numbers(),
                loaded.requires_exact_json_numbers(),
                "{pointer}"
            );
            for value in values.as_array().unwrap() {
                assert_eq!(
                    backend.is_valid(value),
                    loaded.is_valid_json_view(InstanceRef::from_serde(value)),
                    "{pointer}: {value}"
                );
                assert_eq!(
                    expected.is_valid_json_view(InstanceRef::from_serde(value)),
                    loaded.is_valid_json_view(InstanceRef::from_serde(value)),
                    "{pointer}: {value}"
                );
                assert_eq!(
                    expected.explain(value),
                    loaded.explain(value),
                    "{pointer}: {value}"
                );
            }
        }
        let nodes = context.builder.nodes.len();
        let patterns = context.builder.patterns.len();
        context.compile_at("#/$defs/Node").unwrap();
        assert_eq!(context.builder.nodes.len(), nodes);
        assert_eq!(context.builder.patterns.len(), patterns);
        assert!(patterns > 0);
    }

    #[test]
    fn shared_context_preserves_recursive_root_identity_and_depth_limits() {
        std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(|| {
            let schema = serde_json::json!({"$defs": {
                "Self": {"$ref": "#/$defs/Self", "type": "object", "properties": {
                    "next": {"$ref": "#/$defs/Self"}, "flag": {"type": "boolean"}
                }},
                "Indirect": {"allOf": [{"$ref": "#/$defs/Other"}]},
                "Other": {"anyOf": [{"$ref": "#/$defs/Indirect"}, {"type": "null"}],
                    "properties": {"next": {"$ref": "#/$defs/Indirect"}}},
                "Array": {"type": "array", "items": {"$ref": "#/$defs/Array"}}
            }});
            let document = json_schema_ast::SchemaDocument::from_json(&schema).unwrap();
            let mut context = PreparedSchemaContext::new(&document).unwrap().unwrap();
            for pointer in ["#/$defs/Self", "#/$defs/Indirect", "#/$defs/Other", "#/$defs/Array"] {
                let expected = PreparedSchema::compile(&standalone_at(&schema, pointer)).unwrap();
                let shared = context.compile_at(pointer).unwrap();
                if pointer == "#/$defs/Self" {
                    assert!(matches!(shared.nodes[0].rules[0], Rule::Ref { node, .. } if node.0 != 0));
                }
                for length in [0, 1, 2, 126, 127, 128, 254, 255, 256, 257, 300] {
                    for base in [serde_json::json!({}), serde_json::json!({"flag": 1}), serde_json::json!([])] {
                        let mut value = base;
                        for _ in 0..length {
                            value = if pointer == "#/$defs/Array" { serde_json::json!([value]) } else { serde_json::json!({"next": value}) };
                        }
                        assert_eq!(expected.is_valid_json_view(InstanceRef::from_serde(&value)), shared.is_valid_json_view(InstanceRef::from_serde(&value)), "{pointer} at depth {length}");
                        assert_eq!(expected.explain(&value), shared.explain(&value), "{pointer} at depth {length}");
                    }
                }
            }
        }).unwrap().join().unwrap();
    }

    #[test]
    fn shared_context_preserves_recursive_unevaluated_annotations() {
        let schema = serde_json::json!({"$defs": {"Closed": {"type": "object",
            "allOf": [{"properties": {"a": {"type": "integer"}, "next": {"$ref": "#/$defs/Closed"}}}],
            "unevaluatedProperties": false}}});
        let document = json_schema_ast::SchemaDocument::from_json(&schema).unwrap();
        let mut context = PreparedSchemaContext::new(&document).unwrap().unwrap();
        let expected = PreparedSchema::compile(&standalone_at(&schema, "#/$defs/Closed")).unwrap();
        let shared = context.compile_at("#/$defs/Closed").unwrap();
        for value in serde_json::json!([{}, {"a": 1}, {"b": 1}, {"next": {"a": 2}}, {"next": {"b": 1}}, {"next": {"next": {"a": false}}}]).as_array().unwrap() {
            assert_eq!(expected.is_valid_json_view(InstanceRef::from_serde(value)), shared.is_valid_json_view(InstanceRef::from_serde(value)), "{value}");
            assert_eq!(expected.explain(value), shared.explain(value), "{value}");
        }
    }

    #[test]
    fn canonical_context_borrows_the_existing_graph_and_preserves_model_validation() {
        let schema = serde_json::json!({
            "$defs": {
                "Number": {"type":"integer", "minimum":0, "exclusiveMaximum":10},
                "Node": {"type":"object", "required":["value"], "properties": {
                    "value":{"$ref":"#/$defs/Number"},
                    "next":{"anyOf":[{"type":"null"},{"$ref":"#/$defs/Node"}]}
                }, "additionalProperties":false},
                "Shape": {"type":"object", "anyOf":[
                    {"properties":{"name":{"type":"string", "pattern":"^a"}}},
                    {"properties":{"count":{"$ref":"#/$defs/Number"}}}
                ], "unevaluatedProperties":false}
            },
            "type":"object", "properties":{"node":{"$ref":"#/$defs/Node"}}
        });
        let document = json_schema_ast::SchemaDocument::from_json(&schema).unwrap();
        let canonical = document.canonical_schema_json().unwrap();
        assert_ne!(
            canonical, &schema,
            "exercise canonical rather than raw input"
        );
        let mut context = PreparedSchemaContext::new_canonical(&document)
            .unwrap()
            .unwrap();
        assert!(std::ptr::eq(context.builder.root, canonical));
        let values = serde_json::json!([
            null, false, 0, 1, 1.5, 9, 10, "a", {}, {"name":"ab"}, {"name":"bad"},
            {"count":3}, {"count":-1}, {"extra":true}, {"value":1},
            {"value":1,"next":{"value":2,"next":null}}, {"value":1,"next":{"value":10}}
        ]);
        let mut pointers = context.schemas.keys().cloned().collect::<Vec<_>>();
        pointers.sort();
        for pointer in pointers {
            let expected = PreparedSchema::compile(&standalone_at(canonical, &pointer)).unwrap();
            let shared = context.compile_at(&pointer).unwrap();
            let loaded = PreparedSchema::load(&shared.to_bytes().unwrap()).unwrap();
            for value in values.as_array().unwrap() {
                assert_eq!(
                    expected.is_valid_json_view(InstanceRef::from_serde(value)),
                    loaded.is_valid_json_view(InstanceRef::from_serde(value)),
                    "{pointer}: {value}"
                );
                assert_eq!(
                    expected.explain(value),
                    loaded.explain(value),
                    "{pointer}: {value}"
                );
            }
        }
    }

    #[test]
    fn shared_context_falls_back_when_promoting_a_root_can_change_references() {
        for schema in [
            serde_json::json!({"type": "object", "properties": {"next": {"$ref": "#"}}}),
            serde_json::json!({"properties": {"a": {"type": "integer"}, "b": {"$ref": "#/properties/a"}}}),
            serde_json::json!({"$id": "https://example.com/schema", "type": "string"}),
            serde_json::json!({"$defs": {"A": {"$anchor": "A", "type": "string"}}, "$ref": "#A"}),
            serde_json::json!({"$defs": {"A": {"$id": "https://example.com/child", "type": "string"}}, "$ref": "#/$defs/A"}),
            serde_json::json!({"$defs": {"A": {"const": {"schema": {"type": "integer"}}}}, "$ref": "#/$defs/A/const/schema"}),
        ] {
            let document = json_schema_ast::SchemaDocument::from_json(&schema).unwrap();
            assert!(
                PreparedSchemaContext::new(&document).unwrap().is_none(),
                "{schema}"
            );
        }
    }

    #[test]
    fn shared_context_treats_annotation_payloads_as_data_and_rejects_non_schema_positions() {
        let schema = serde_json::json!({"$defs": {"A": {"const": {"$id": "data", "$ref": "#"},
            "examples": [{"$dynamicRef": "data"}]}}, "type": "object"});
        let document = json_schema_ast::SchemaDocument::from_json(&schema).unwrap();
        let mut context = PreparedSchemaContext::new(&document).unwrap().unwrap();
        assert!(context.compile_at("#/$defs/A/const").is_err());
        let model = context.compile_at("#/$defs/A").unwrap();
        assert!(model.is_valid_json_view(InstanceRef::from_serde(
            &serde_json::json!({"$id": "data", "$ref": "#"})
        )));
    }

    #[test]
    fn exhausted_depth_cannot_accept_negations_or_conditionals() {
        std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(|| {
                // Every reference resolves to true, so both enclosing schemas
                // reject null whether the reference chain fits the budget or not.
                for length in [4, MAX_DEPTH + 4] {
                    let nodes: Vec<_> = (0..=length)
                        .map(|index| Node {
                            types: None,
                            choices: None,
                            constant: false,
                            rules: if index < length {
                                vec![Rule::Ref {
                                    node: NodeId(index + 1),
                                    pointer: String::new(),
                                }]
                            } else {
                                Vec::new()
                            },
                        })
                        .chain(std::iter::once(Node {
                            types: None,
                            choices: None,
                            constant: false,
                            rules: vec![Rule::False],
                        }))
                        .collect();
                    let mut program = PreparedSchema {
                        version: VERSION,
                        nodes,
                        patterns: Vec::new(),
                        pattern_sources: Vec::new(),
                        exact_json_numbers: false,
                    };
                    for rule in [
                        Rule::Not(NodeId(1)),
                        Rule::If {
                            condition: NodeId(1),
                            then_node: Some(NodeId(length + 1)),
                            else_node: None,
                        },
                    ] {
                        program.nodes[0].rules = vec![rule];
                        assert!(!program.is_valid_json_view(InstanceRef::from_serde(&Value::Null)));
                        assert_eq!(
                            program.explain(&Value::Null).unwrap().kind,
                            if length >= MAX_DEPTH {
                                ValidationFailureKind::ResourceLimit
                            } else {
                                ValidationFailureKind::Constraint
                            }
                        );
                    }
                }
            })
            .expect("start schema-depth regression")
            .join()
            .expect("schema-depth regression");
    }
}
