//! Borrowed input boundary for prepared programs.
//!
//! Backends implement these statically dispatched views without allocating a
//! second JSON tree. The published crate knows only serde; Python and parser
//! adapters belong to the embedding application.
use num_bigint::BigInt;
use serde_json::{Map, Number, Value};

pub trait InstanceView<'a>: Copy {
    type Number: NumberView<'a>;
    type Array: ArrayView<'a, Item = Self>;
    type Object: ObjectView<'a, Item = Self>;
    fn is_json(self) -> bool;
    fn is_null(self) -> bool;
    fn is_boolean(self) -> bool;
    fn as_str(self) -> Option<&'a str>;
    fn as_number(self) -> Option<Self::Number>;
    fn as_array(self) -> Option<Self::Array>;
    fn as_object(self) -> Option<Self::Object>;
    fn equals(self, expected: &Value) -> bool;
    fn to_owned(self) -> Value;
    fn is_string(self) -> bool {
        self.as_str().is_some()
    }
    fn is_number(self) -> bool {
        self.as_number().is_some()
    }
    fn is_array(self) -> bool {
        self.as_array().is_some()
    }
    fn is_object(self) -> bool {
        self.as_object().is_some()
    }
}

pub trait NumberView<'a>: Copy {
    fn as_i64(self) -> Option<i64>;
    fn as_u64(self) -> Option<u64>;
    fn as_f64(self) -> Option<f64>;
    fn is_integer(self) -> bool;
    fn serde(self) -> Option<&'a Number>;
    fn big_integer(self) -> Option<&'a BigInt> {
        None
    }
}

pub trait ArrayView<'a>: Copy {
    type Item: InstanceView<'a>;
    fn len(self) -> usize;
    fn is_empty(self) -> bool {
        self.len() == 0
    }
    fn iter(self) -> impl Iterator<Item = Self::Item>;
}
pub trait ObjectView<'a>: Copy {
    type Item: InstanceView<'a>;
    fn len(self) -> usize;
    fn is_empty(self) -> bool {
        self.len() == 0
    }
    fn contains_key(self, key: &str) -> bool;
    fn iter(self) -> impl Iterator<Item = (&'a str, Self::Item)>;
    fn keys(self) -> impl Iterator<Item = &'a str> {
        self.iter().map(|(key, _)| key)
    }
}

#[derive(Clone, Copy)]
pub struct InstanceRef<'a>(&'a Value);
impl<'a> InstanceRef<'a> {
    pub fn from_serde(value: &'a Value) -> Self {
        Self(value)
    }
}
impl<'a> InstanceView<'a> for InstanceRef<'a> {
    type Number = &'a Number;
    type Array = &'a [Value];
    type Object = &'a Map<String, Value>;
    fn is_json(self) -> bool {
        true
    }
    fn is_null(self) -> bool {
        self.0.is_null()
    }
    fn is_boolean(self) -> bool {
        self.0.is_boolean()
    }
    fn as_str(self) -> Option<&'a str> {
        self.0.as_str()
    }
    fn as_number(self) -> Option<Self::Number> {
        self.0.as_number()
    }
    fn as_array(self) -> Option<Self::Array> {
        self.0.as_array().map(Vec::as_slice)
    }
    fn as_object(self) -> Option<Self::Object> {
        self.0.as_object()
    }
    fn equals(self, expected: &Value) -> bool {
        json_schema_ast::json_values_equal(self.0, expected)
    }
    fn to_owned(self) -> Value {
        self.0.clone()
    }
}
impl<'a> NumberView<'a> for &'a Number {
    fn as_i64(self) -> Option<i64> {
        Number::as_i64(self)
    }
    fn as_u64(self) -> Option<u64> {
        Number::as_u64(self)
    }
    fn as_f64(self) -> Option<f64> {
        Number::as_f64(self)
    }
    fn is_integer(self) -> bool {
        self.is_i64() || self.is_u64() || self.as_f64().is_some_and(|value| value.fract() == 0.0)
    }
    fn serde(self) -> Option<&'a Number> {
        Some(self)
    }
}
impl<'a> ArrayView<'a> for &'a [Value] {
    type Item = InstanceRef<'a>;
    fn len(self) -> usize {
        <[Value]>::len(self)
    }
    fn iter(self) -> impl Iterator<Item = Self::Item> {
        <[Value]>::iter(self).map(InstanceRef)
    }
}
impl<'a> ObjectView<'a> for &'a Map<String, Value> {
    type Item = InstanceRef<'a>;
    fn len(self) -> usize {
        Map::len(self)
    }
    fn contains_key(self, key: &str) -> bool {
        Map::contains_key(self, key)
    }
    fn iter(self) -> impl Iterator<Item = (&'a str, Self::Item)> {
        Map::iter(self).map(|(key, value)| (key.as_str(), InstanceRef(value)))
    }
}
