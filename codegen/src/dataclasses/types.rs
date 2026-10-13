//! Typed model descriptions shared by source rendering and runtime preparation.

use serde_json::Value;
use std::{cmp::Ordering, fmt, ops::Deref};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Any,
    String,
    Bytes,
    Integer,
    Number,
    Boolean,
    Null,
    Literal(Value),
    Sequence(Box<Annotation>),
    Mapping(Box<Annotation>),
    Model(String),
    Union(Vec<Annotation>),
}

#[derive(Clone, Debug, Eq)]
pub(super) struct Annotation {
    source: String,
    pub kind: Kind,
}

impl Annotation {
    fn new(source: impl Into<String>, kind: Kind) -> Self {
        Self {
            source: source.into(),
            kind,
        }
    }
    pub fn any() -> Self {
        Self::new("typing.Any", Kind::Any)
    }
    pub fn json() -> Self {
        Self::new("dc.JsonValue", Kind::Any)
    }
    pub fn string() -> Self {
        Self::new("str", Kind::String)
    }
    pub fn bytes() -> Self {
        Self::new("builtins.bytes", Kind::Bytes)
    }
    pub fn integer() -> Self {
        Self::new("int", Kind::Integer)
    }
    pub fn number() -> Self {
        Self::new("float", Kind::Number)
    }
    pub fn boolean() -> Self {
        Self::new("bool", Kind::Boolean)
    }
    pub fn null() -> Self {
        Self::new("None", Kind::Null)
    }
    pub fn model(name: String) -> Self {
        Self::new(name.clone(), Kind::Model(name))
    }
    pub fn literal(value: Value) -> Self {
        Self::new(
            format!("typing.Literal[{}]", super::python_json_literal(&value)),
            Kind::Literal(value),
        )
    }
    pub fn sequence(item: Self) -> Self {
        Self::new(
            format!("collections.abc.Sequence[{item}]"),
            Kind::Sequence(Box::new(item)),
        )
    }
    pub fn mapping(value: Self) -> Self {
        Self::new(
            format!("collections.abc.Mapping[str, {value}]"),
            Kind::Mapping(Box::new(value)),
        )
    }
    pub fn union(branches: &[Self]) -> Self {
        let mut unique = Vec::new();
        for branch in branches {
            if let Kind::Union(children) = &branch.kind {
                unique.extend(children.iter().cloned());
            } else {
                unique.push(branch.clone());
            }
        }
        unique.sort();
        unique.dedup();
        if unique.iter().any(|a| a.source == "typing.Any") {
            return Self::any();
        }
        unique.sort_by(|a, b| match (a.source == "None", b.source == "None") {
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            _ => a.cmp(b),
        });
        match unique.len() {
            0 => Self::any(),
            1 => unique.pop().expect("one branch"),
            _ => {
                let sources = union_sources(&unique);
                let source = if sources.len() == 1 {
                    sources[0].clone()
                } else {
                    format!("({})", sources.join(" | "))
                };
                Self::new(source, Kind::Union(unique))
            }
        }
    }
}
impl Deref for Annotation {
    type Target = str;
    fn deref(&self) -> &str {
        &self.source
    }
}
impl fmt::Display for Annotation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.source)
    }
}
impl PartialEq for Annotation {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
    }
}
impl Ord for Annotation {
    fn cmp(&self, other: &Self) -> Ordering {
        self.source.cmp(&other.source)
    }
}
impl PartialOrd for Annotation {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// Literal alternatives share one typing.Literal declaration. Keep the original
// branch graph intact: validation and discriminator planning still see every
// distinct literal, including Python's otherwise equal True/1 values.
fn union_sources(branches: &[Annotation]) -> Vec<String> {
    let literals: Vec<_> = branches
        .iter()
        .filter_map(|branch| match &branch.kind {
            Kind::Literal(value) => Some(super::python_json_literal(value)),
            _ => None,
        })
        .collect();
    let mut result = Vec::new();
    let mut emitted = false;
    for branch in branches {
        if matches!(branch.kind, Kind::Literal(_)) && literals.len() > 1 {
            if !emitted {
                result.push(format!("typing.Literal[{}]", literals.join(", ")));
                emitted = true;
            }
        } else {
            result.push(branch.to_string());
        }
    }
    result
}
