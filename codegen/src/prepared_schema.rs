//! Portable validation programs prepared before Python application startup.
//!
//! The wire format contains node indices, never native addresses or Python
//! objects. Loading checks its version and every edge without compiling schemas.
//! Unsupported vocabulary is rejected by the builder, never silently ignored.

use std::cmp::Ordering;
use std::collections::HashMap;

use jsonschema::{InstanceRef, NumberRef};
use num_cmp::NumCmp;
use num_traits::{FromPrimitive, ToPrimitive};
use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};

mod annotations;
mod pattern;
use pattern::Pattern;

const VERSION: u32 = 1;
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
    patterns: Vec<Pattern>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    exact_json_numbers: bool,
}

#[derive(Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub types: Option<Vec<JsonType>>,
    pub choices: Option<Vec<Value>>,
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
    pub fn accepts(self, value: InstanceRef<'_>) -> bool {
        match self {
            Self::Null => value.is_null(),
            Self::Boolean => value.is_boolean(),
            Self::Integer => value.as_number().is_some_and(NumberRef::is_integer),
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
    Ref(NodeId),
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
    MultipleOf(f64),
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
        exact: Option<fraction::BigFraction>,
        lower: bool,
        exclusive: bool,
    },
}

impl Node {
    /// The parser and writer already know the concrete scalar representation.
    /// Avoid re-dispatching through all supported instance backends per rule.
    #[inline]
    pub fn accepts_jiter_leaf(&self, value: &jiter::JsonValue<'_>) -> bool {
        match value {
            jiter::JsonValue::Str(text) => {
                self.types
                    .as_ref()
                    .is_none_or(|types| types.iter().any(|kind| matches!(kind, JsonType::String)))
                    && self.choices.as_ref().is_none_or(|choices| {
                        choices
                            .iter()
                            .any(|choice| choice.as_str() == Some(text.as_ref()))
                    })
                    && self.rules.iter().all(|rule| match rule {
                        Rule::False => false,
                        Rule::StringLength { min, max } => string_length_valid(text, *min, *max),
                        Rule::Bound { .. } | Rule::MultipleOf(_) => true,
                        _ => unreachable!("leaf guards contain only scalar constraints"),
                    })
            }
            jiter::JsonValue::Int(number) => {
                self.types.as_ref().is_none_or(|types| {
                    types
                        .iter()
                        .any(|kind| matches!(kind, JsonType::Integer | JsonType::Number))
                }) && self.choices.as_ref().is_none_or(|choices| {
                    choices.iter().any(|choice| {
                        choice.as_number().is_some_and(|choice| {
                            compare_integer(*number, choice) == Some(Ordering::Equal)
                        })
                    })
                }) && self.rules.iter().all(|rule| match rule {
                    Rule::False => false,
                    Rule::Bound {
                        value,
                        exact,
                        lower,
                        exclusive,
                    } => exact
                        .as_ref()
                        .map_or_else(
                            || compare_integer(*number, value),
                            |limit| fraction::BigFraction::from(*number).partial_cmp(limit),
                        )
                        .is_some_and(|order| match order {
                            Ordering::Equal => !exclusive,
                            Ordering::Greater => *lower,
                            Ordering::Less => !lower,
                        }),
                    Rule::MultipleOf(divisor) => {
                        multiple_of(InstanceRef::from_jiter(value), *divisor)
                    }
                    Rule::StringLength { .. } => true,
                    _ => unreachable!("leaf guards contain only scalar constraints"),
                })
            }
            _ => self.accepts_leaf(InstanceRef::from_jiter(value)),
        }
    }

    pub fn is_leaf(&self) -> bool {
        self.rules.iter().all(|rule| {
            matches!(
                rule,
                Rule::False | Rule::StringLength { .. } | Rule::Bound { .. } | Rule::MultipleOf(_)
            )
        })
    }

    #[inline]
    pub fn accepts_leaf(&self, value: InstanceRef<'_>) -> bool {
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
                Rule::MultipleOf(divisor) => multiple_of(value, *divisor),
                _ => unreachable!("leaf guards contain only scalar constraints"),
            })
    }
}

fn multiple_of(value: InstanceRef<'_>, divisor: f64) -> bool {
    value.as_number().is_none_or(|number| {
        let Some(number) = number.as_f64() else {
            return false;
        };
        if divisor.fract() == 0.0 {
            return number.fract() == 0.0 && number % divisor == 0.0;
        }
        if number == 0.0 {
            return true;
        }
        number >= divisor
            && (fraction::BigFraction::from(number) / fraction::BigFraction::from(divisor))
                .denom()
                .is_none_or(num_traits::One::is_one)
    })
}

impl Rule {
    fn edges(&self) -> Vec<NodeId> {
        match self {
            Self::Ref(node)
            | Self::Not(node)
            | Self::PropertyNames(node)
            | Self::Contains { node, .. }
            | Self::UnevaluatedProperties(node)
            | Self::UnevaluatedItems(node) => vec![*node],
            Self::All(nodes) | Self::Any(nodes) | Self::One(nodes) => nodes.clone(),
            Self::If {
                condition,
                then_node,
                else_node,
            } => std::iter::once(*condition)
                .chain(*then_node)
                .chain(*else_node)
                .collect(),
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
                .collect(),
            Self::DependentSchemas(entries) => entries.iter().map(|(_, node)| *node).collect(),
            Self::Array { prefix, items } => prefix.iter().copied().chain(*items).collect(),
            _ => Vec::new(),
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
        jsonschema::draft202012::options()
            .build(document.source_schema_json())
            .map_err(|error| {
                format!("schema failed Draft 2020-12 validator compilation: {error}")
            })?;
        let mut builder = Builder {
            root: schema,
            nodes: Vec::new(),
            paths: HashMap::new(),
            leaves: HashMap::new(),
            patterns: Vec::new(),
            pattern_ids: HashMap::new(),
        };
        builder.add(schema, "#")?;
        let exact_json_numbers = builder.nodes.iter().any(|node| {
            node.rules
                .iter()
                .any(|rule| matches!(rule, Rule::Bound { exact: Some(_), .. }))
                || node
                    .choices
                    .as_ref()
                    .is_some_and(|choices| choices.iter().any(needs_exact_json_number))
        });
        Ok(Self {
            version: VERSION,
            exact_json_numbers,
            nodes: builder.nodes,
            patterns: builder.patterns,
        })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(self).map_err(|error| error.to_string())
    }

    pub fn load(bytes: &[u8]) -> Result<Self, String> {
        let program: Self = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
        if program.version != VERSION || program.nodes.is_empty() {
            return Err(
                "unsupported or empty prepared schema; rebuild the generated module".into(),
            );
        }
        for node in &program.nodes {
            for rule in &node.rules {
                if rule
                    .edges()
                    .iter()
                    .any(|edge| edge.0 >= program.nodes.len())
                {
                    return Err("prepared schema contains an invalid node reference".into());
                }
                let pattern_ids = match rule {
                    Rule::Pattern(id) => vec![*id],
                    Rule::Object { patterns, .. } => patterns.iter().map(|(id, _)| *id).collect(),
                    _ => Vec::new(),
                };
                if pattern_ids.iter().any(|id| id.0 >= program.patterns.len()) {
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

    pub fn is_valid_instance(&self, value: InstanceRef<'_>) -> bool {
        value.is_json() && self.is_valid_instance_assuming_json(value)
    }

    pub fn is_valid_instance_assuming_json(&self, value: InstanceRef<'_>) -> bool {
        self.accepts(NodeId(0), value, 0, &mut Evaluation::default())
    }

    fn accepts(
        &self,
        id: NodeId,
        value: InstanceRef<'_>,
        depth: usize,
        context: &mut Evaluation,
    ) -> bool {
        let identity = (id.0, context.instance_depth);
        if context.active.contains(&identity) {
            return true;
        }
        if depth >= MAX_DEPTH {
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

    fn accepts_child(
        &self,
        node: NodeId,
        value: InstanceRef<'_>,
        depth: usize,
        context: &mut Evaluation,
    ) -> bool {
        context.instance_depth += 1;
        let valid = self.accepts(node, value, depth, context);
        context.instance_depth -= 1;
        valid
    }

    fn accepts_rule(
        &self,
        rule: &Rule,
        value: InstanceRef<'_>,
        depth: usize,
        context: &mut Evaluation,
    ) -> bool {
        match rule {
            Rule::False => false,
            Rule::Ref(node) => self.accepts(*node, value, depth, context),
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
                        if self.patterns[pattern.0].is_match(name) {
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
                .is_none_or(|value| self.patterns[id.0].is_match(value)),
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
            Rule::MultipleOf(divisor) => multiple_of(value, *divisor),
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
}

fn string_length_valid(value: &str, min: u64, max: Option<u64>) -> bool {
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
fn exact_decimal(text: &str) -> Option<fraction::BigFraction> {
    if let Some((mantissa, exponent)) = text.split_once(['e', 'E']) {
        let exponent: i32 = exponent.parse().ok()?;
        if exponent.unsigned_abs() > 10_000 {
            return None;
        }
        let mantissa: fraction::BigFraction = mantissa.parse().ok()?;
        let power = fraction::BigFraction::from(
            num_bigint::BigUint::from(10_u8).pow(exponent.unsigned_abs()),
        );
        Some(if exponent < 0 {
            mantissa / power
        } else {
            mantissa * power
        })
    } else {
        text.parse().ok()
    }
}

fn compare_bound(
    value: InstanceRef<'_>,
    limit: &Number,
    exact: Option<&fraction::BigFraction>,
) -> Option<Ordering> {
    let number = value.as_number()?;
    let Some(exact) = exact else {
        return compare_number(number, limit);
    };
    let number = if let Some(value) = number.as_i64() {
        fraction::BigFraction::from(value)
    } else if let Some(value) = number.as_u64() {
        fraction::BigFraction::from(value)
    } else {
        match number {
            NumberRef::BigInteger(value) => fraction::BigFraction::from(value.clone()),
            NumberRef::Float(value) => fraction::BigFraction::from(value),
            // Preserve arbitrary Python integers and serde decimal numbers;
            // converting these through f64 loses bits at exclusive boundaries.
            _ => exact_decimal(&value.to_owned().to_string())?,
        }
    };
    number.partial_cmp(exact)
}

fn compare_number(value: NumberRef<'_>, limit: &Number) -> Option<Ordering> {
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
    if let NumberRef::BigInteger(value) = value {
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

fn compare_integer(value: i64, limit: &Number) -> Option<Ordering> {
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
    nodes: Vec<Node>,
    paths: HashMap<String, NodeId>,
    leaves: HashMap<Vec<u8>, NodeId>,
    patterns: Vec<Pattern>,
    pattern_ids: HashMap<String, PatternId>,
}

impl Builder<'_> {
    fn pattern(&mut self, pattern: &str) -> Result<PatternId, String> {
        if let Some(id) = self.pattern_ids.get(pattern) {
            return Ok(*id);
        }
        let id = PatternId(self.patterns.len());
        self.patterns.push(Pattern::compile(pattern)?);
        self.pattern_ids.insert(pattern.to_owned(), id);
        Ok(id)
    }

    fn add(&mut self, schema: &Value, path: &str) -> Result<NodeId, String> {
        if let Some(id) = self.paths.get(path) {
            return Ok(*id);
        }
        if self.nodes.len() >= 100_000 {
            return Err("prepared schema exceeds 100,000 nodes".into());
        }
        let id = NodeId(self.nodes.len());
        self.paths.insert(path.to_owned(), id);
        self.nodes.push(Node {
            types: None,
            choices: None,
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
                "$dynamicRef" | "$recursiveRef" | "$vocabulary"
            ) || (key == "$id" && path != "#")
            {
                return Err(format!(
                    "{path}/{key}: not supported by the ahead-of-time validator yet; use the original generated module"
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
            let target = self
                .root
                .pointer(&pointer)
                .ok_or_else(|| format!("unresolved reference {reference}"))?
                .clone();
            node.rules
                .push(Rule::Ref(self.add(&target, &format!("#{pointer}"))?));
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
        if let Some(value) = object.get("multipleOf").and_then(Value::as_f64) {
            node.rules.push(Rule::MultipleOf(value));
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
