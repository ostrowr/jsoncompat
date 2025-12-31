use crate::error::SchemaPath;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct ModelGraph {
    pub root_name: String,
    pub root_type: SchemaType,
    pub models: BTreeMap<String, ModelSpec>,
}

#[derive(Debug, Clone)]
pub struct ModelSpec {
    pub name: String,
    pub fields: Vec<FieldSpec>,
    pub additional_properties: AdditionalProperties,
    pub min_properties: Option<usize>,
    pub max_properties: Option<usize>,
    pub pattern_properties: Vec<String>,
    pub property_name_max: Option<usize>,
    pub has_all_of: bool,
    pub requires_object: bool,
    pub description: Option<String>,
    pub title: Option<String>,
    pub schema_path: SchemaPath,
}

#[derive(Debug, Clone)]
pub struct FieldSpec {
    pub name: String,
    pub schema: SchemaType,
    pub required: bool,
    pub default: Option<Value>,
    pub description: Option<String>,
    pub title: Option<String>,
    pub read_only: bool,
    pub write_only: bool,
}

impl FieldSpec {
    pub fn include_in_role(&self, role: ModelRole) -> bool {
        match role {
            ModelRole::Serializer => !self.read_only,
            ModelRole::Deserializer => true,
        }
    }

    pub fn required_for_role(&self, role: ModelRole) -> bool {
        match role {
            ModelRole::Serializer => self.required && !self.read_only,
            ModelRole::Deserializer => self.required && !self.read_only,
        }
    }

    pub fn default_for_role(&self, role: ModelRole) -> Option<&Value> {
        match role {
            ModelRole::Serializer => None,
            ModelRole::Deserializer => {
                if self.required_for_role(role) {
                    None
                } else {
                    self.default.as_ref()
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelRole {
    Serializer,
    Deserializer,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AdditionalProperties {
    Allow,
    Forbid,
    Typed(Box<SchemaType>),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct StringConstraints {
    pub min_length: Option<u64>,
    pub max_length: Option<u64>,
    pub pattern: Option<String>,
    pub format: Option<StringFormat>,
    pub type_enforced: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct NumberConstraints {
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub exclusive_minimum: bool,
    pub exclusive_maximum: bool,
    pub multiple_of: Option<f64>,
    pub type_enforced: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ArrayConstraints {
    pub min_items: Option<u64>,
    pub max_items: Option<u64>,
    pub type_enforced: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ObjectConstraints {
    pub min_properties: Option<usize>,
    pub max_properties: Option<usize>,
    pub type_enforced: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StringFormat {
    DateTime,
    Date,
    Time,
    Uuid,
    Email,
    Uri,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SchemaType {
    Any,
    Bool,
    String(StringConstraints),
    Integer(NumberConstraints),
    Number(NumberConstraints),
    Null,
    Array {
        items: Box<SchemaType>,
        constraints: ArrayConstraints,
    },
    Map {
        values: Box<SchemaType>,
        constraints: ObjectConstraints,
    },
    Object(String),
    Literal(Vec<LiteralValue>),
    Union(Vec<SchemaType>),
    Nullable(Box<SchemaType>),
}

impl SchemaType {
    pub fn allows_null(&self) -> bool {
        match self {
            SchemaType::Null => true,
            SchemaType::Nullable(_) => true,
            SchemaType::Union(variants) => variants.iter().any(|v| v.allows_null()),
            SchemaType::Literal(values) => values.iter().any(|v| v.is_null()),
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LiteralValue {
    Null,
    Bool(bool),
    String(String),
    Number(serde_json::Number),
    Json(Value),
}

impl LiteralValue {
    pub fn is_null(&self) -> bool {
        matches!(self, LiteralValue::Null)
    }

    pub fn is_simple(&self) -> bool {
        matches!(
            self,
            LiteralValue::Null
                | LiteralValue::Bool(_)
                | LiteralValue::String(_)
                | LiteralValue::Number(_)
        )
    }

    pub fn to_value(&self) -> Value {
        match self {
            LiteralValue::Null => Value::Null,
            LiteralValue::Bool(v) => Value::Bool(*v),
            LiteralValue::String(v) => Value::String(v.clone()),
            LiteralValue::Number(v) => Value::Number(v.clone()),
            LiteralValue::Json(v) => v.clone(),
        }
    }

    pub fn matches_value(&self, value: &Value) -> bool {
        json_values_equal(&self.to_value(), value)
    }
}

fn json_values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Number(x), Value::Number(y)) => match (x.as_f64(), y.as_f64()) {
            (Some(lhs), Some(rhs)) => (lhs - rhs).abs() < f64::EPSILON,
            _ => x == y,
        },
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Array(xs), Value::Array(ys)) => {
            xs.len() == ys.len() && xs.iter().zip(ys).all(|(x, y)| json_values_equal(x, y))
        }
        (Value::Object(xs), Value::Object(ys)) => {
            if xs.len() != ys.len() {
                return false;
            }
            xs.keys().all(|k| ys.contains_key(k))
                && xs
                    .iter()
                    .all(|(k, v)| ys.get(k).is_some_and(|other| json_values_equal(v, other)))
        }
        _ => false,
    }
}
