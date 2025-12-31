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
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct NumberConstraints {
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub exclusive_minimum: bool,
    pub exclusive_maximum: bool,
    pub multiple_of: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ArrayConstraints {
    pub min_items: Option<u64>,
    pub max_items: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ObjectConstraints {
    pub min_properties: Option<usize>,
    pub max_properties: Option<usize>,
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
            SchemaType::Literal(values) => values.iter().any(|v| matches!(v, LiteralValue::Null)),
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
}

impl LiteralValue {
    pub fn matches_value(&self, value: &Value) -> bool {
        match (self, value) {
            (LiteralValue::Null, Value::Null) => true,
            (LiteralValue::Bool(a), Value::Bool(b)) => a == b,
            (LiteralValue::String(a), Value::String(b)) => a == b,
            (LiteralValue::Number(a), Value::Number(b)) => a == b,
            _ => false,
        }
    }
}
