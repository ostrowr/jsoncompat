//! Python/Jiter adapters for the portable prepared evaluator.
//! Fork-only validator types stay inside this unpublished crate.
use jsoncompat_codegen::prepared_schema as portable;
pub(crate) use portable::Node;
use portable::instance::{ArrayView, InstanceView, NumberView, ObjectView};
use portable::{JsonType, Rule, compare_integer, string_length_valid};
use serde_json::{Number, Value};
use std::cmp::Ordering;

pub(crate) struct PreparedSchema(portable::PreparedSchema);
impl std::ops::Deref for PreparedSchema {
    type Target = portable::PreparedSchema;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl PreparedSchema {
    pub(crate) fn load(bytes: &[u8]) -> Result<Self, String> {
        portable::PreparedSchema::load(bytes).map(Self)
    }
    pub(crate) fn is_valid_instance(&self, value: jsonschema::InstanceRef<'_>) -> bool {
        self.0.is_valid_view(View(value))
    }
    pub(crate) fn is_valid_instance_assuming_json(
        &self,
        value: jsonschema::InstanceRef<'_>,
    ) -> bool {
        self.0.is_valid_json_view(View(value))
    }
}

pub(crate) trait NodeExt {
    fn accepts_leaf(&self, value: jsonschema::InstanceRef<'_>) -> bool;
    fn accepts_jiter_leaf(&self, value: &jiter::JsonValue<'_>) -> bool;
}
impl NodeExt for Node {
    #[inline]
    fn accepts_leaf(&self, value: jsonschema::InstanceRef<'_>) -> bool {
        self.accepts_leaf_view(View(value))
    }
    /// The parser and writer already know the concrete scalar representation.
    /// Avoid re-dispatching through all supported instance backends per rule.
    #[inline]
    fn accepts_jiter_leaf(&self, value: &jiter::JsonValue<'_>) -> bool {
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
                    Rule::MultipleOf(_) => {
                        self.accepts_leaf(jsonschema::InstanceRef::from_jiter(value))
                    }
                    Rule::StringLength { .. } => true,
                    _ => unreachable!("leaf guards contain only scalar constraints"),
                })
            }
            _ => self.accepts_leaf(jsonschema::InstanceRef::from_jiter(value)),
        }
    }
}

#[derive(Clone, Copy)]
struct View<'a>(jsonschema::InstanceRef<'a>);
#[derive(Clone, Copy)]
struct NumberAdapter<'a>(jsonschema::NumberRef<'a>);
#[derive(Clone, Copy)]
struct ArrayAdapter<'a>(jsonschema::ArrayRef<'a>);
#[derive(Clone, Copy)]
struct ObjectAdapter<'a>(jsonschema::ObjectRef<'a>);

impl<'a> InstanceView<'a> for View<'a> {
    type Number = NumberAdapter<'a>;
    type Array = ArrayAdapter<'a>;
    type Object = ObjectAdapter<'a>;
    #[inline]
    fn is_json(self) -> bool {
        self.0.is_json()
    }
    #[inline]
    fn is_null(self) -> bool {
        self.0.is_null()
    }
    #[inline]
    fn is_boolean(self) -> bool {
        self.0.is_boolean()
    }
    #[inline]
    fn as_str(self) -> Option<&'a str> {
        self.0.as_str()
    }
    #[inline]
    fn as_number(self) -> Option<Self::Number> {
        self.0.as_number().map(NumberAdapter)
    }
    #[inline]
    fn as_array(self) -> Option<Self::Array> {
        self.0.as_array().map(ArrayAdapter)
    }
    #[inline]
    fn as_object(self) -> Option<Self::Object> {
        self.0.as_object().map(ObjectAdapter)
    }
    #[inline]
    fn equals(self, value: &Value) -> bool {
        self.0.equals(value)
    }
    fn to_owned(self) -> Value {
        self.0.to_owned()
    }
}
impl<'a> NumberView<'a> for NumberAdapter<'a> {
    #[inline]
    fn as_i64(self) -> Option<i64> {
        self.0.as_i64()
    }
    #[inline]
    fn as_u64(self) -> Option<u64> {
        self.0.as_u64()
    }
    #[inline]
    fn as_f64(self) -> Option<f64> {
        self.0.as_f64()
    }
    #[inline]
    fn is_integer(self) -> bool {
        self.0.is_integer()
    }
    #[inline]
    fn serde(self) -> Option<&'a Number> {
        match self.0 {
            jsonschema::NumberRef::Serde(value) => Some(value),
            _ => None,
        }
    }
    #[inline]
    fn big_integer(self) -> Option<&'a num_bigint::BigInt> {
        match self.0 {
            jsonschema::NumberRef::BigInteger(value) => Some(value),
            _ => None,
        }
    }
}
impl<'a> ArrayView<'a> for ArrayAdapter<'a> {
    type Item = View<'a>;
    #[inline]
    fn len(self) -> usize {
        self.0.len()
    }
    #[inline]
    fn iter(self) -> impl Iterator<Item = Self::Item> {
        self.0.iter().map(View)
    }
}
impl<'a> ObjectView<'a> for ObjectAdapter<'a> {
    type Item = View<'a>;
    #[inline]
    fn len(self) -> usize {
        self.0.len()
    }
    #[inline]
    fn contains_key(self, key: &str) -> bool {
        self.0.contains_key(key)
    }
    #[inline]
    fn iter(self) -> impl Iterator<Item = (&'a str, Self::Item)> {
        self.0.iter().map(|(key, value)| (key, View(value)))
    }
}
