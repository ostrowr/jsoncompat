//! Application-owned borrowed views for the prepared Python model evaluator.
//!
//! These adapters expose Jiter trees, Python values, and generated model slots
//! without allocating an intermediate JSON tree. They contain no schema validator.
//!
//! Adapted from ostrowr/jsonschema's instance adapters at 3705ac7.
// MIT License
//
// Copyright (c) 2020-2026 Dmitry Dygalo
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use std::iter::FusedIterator;

use serde_json::{Map, Number, Value};

use pyo3::{
    Borrowed, ffi,
    prelude::*,
    types::{PyAny, PyBool, PyDict, PyFloat, PyInt, PyList, PyString, PyTuple},
};

/// A borrowed JSON value accepted by flag validation.
#[derive(Clone, Copy, Debug)]
pub(crate) struct InstanceRef<'a> {
    repr: InstanceRepr<'a>,
}

#[derive(Clone, Copy, Debug)]
enum InstanceRepr<'a> {
    Serde(&'a Value),
    Jiter(&'a jiter::JsonValue<'a>),
    Python(PythonValue<'a>),
    ProjectedPython(ProjectedPythonInstance<'a>),
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct InstanceIdentity {
    representation: usize,
    node: usize,
    value: usize,
}

#[derive(Clone, Copy)]
pub(crate) struct PythonValue<'a> {
    object: std::ptr::NonNull<ffi::PyObject>,
    py: Python<'a>,
}

impl std::fmt::Debug for PythonValue<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_tuple("PythonValue")
            .field(&self.object)
            .finish()
    }
}

impl<'a> PythonValue<'a> {
    fn from_bound<'py>(value: &'a Bound<'py, PyAny>) -> Self
    where
        'py: 'a,
    {
        Self {
            object: std::ptr::NonNull::new(value.as_ptr())
                .expect("Python objects are never represented by null pointers"),
            py: value.py(),
        }
    }

    unsafe fn from_ptr(py: Python<'a>, value: *mut ffi::PyObject) -> Self {
        Self {
            object: std::ptr::NonNull::new(value)
                .expect("borrowed Python container entries are never null"),
            py,
        }
    }

    fn borrowed(self) -> Borrowed<'a, 'a, PyAny> {
        // SAFETY: `object` remains owned by the Python value graph borrowed for
        // `'a`, and `py` proves the interpreter lock is held for that lifetime.
        unsafe { Borrowed::from_ptr(self.py, self.object.as_ptr()) }
    }
}

/// A Python value plus an application-defined projection node.
///
/// Projection providers use this compact value to expose non-JSON Python
/// objects as their logical JSON representation without allocating an
/// intermediate `serde_json::Value` tree.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ProjectedPythonValue<'a> {
    value: PythonValue<'a>,
    node: usize,
}

impl<'a> ProjectedPythonValue<'a> {
    /// Pair a borrowed Python value with a provider-defined projection node.
    ///
    /// # Panics
    ///
    /// Panics only if `value` contains a null pointer, which would violate
    /// `pyo3::Borrowed`'s validity invariant.
    #[must_use]
    pub(crate) fn new(node: usize, value: Borrowed<'a, 'a, PyAny>) -> Self {
        Self {
            value: PythonValue {
                object: std::ptr::NonNull::new(value.as_ptr())
                    .expect("Python objects are never represented by null pointers"),
                py: value.py(),
            },
            node,
        }
    }

    /// Return the provider-defined projection node.
    #[must_use]
    pub(crate) const fn node(self) -> usize {
        self.node
    }

    /// Borrow the underlying Python object.
    #[must_use]
    pub(crate) fn value(self) -> Borrowed<'a, 'a, PyAny> {
        self.value.borrowed()
    }
}

/// The logical JSON shape selected by a [`PythonInstanceProvider`].
#[derive(Clone, Copy, Debug)]
pub(crate) enum ProjectedPythonKind<'a> {
    /// Interpret the underlying object as an ordinary Python JSON value.
    Native(ProjectedPythonValue<'a>),
    /// Interpret the value through the provider's array operations.
    Array(ProjectedPythonValue<'a>),
    /// Interpret the value through the provider's object operations.
    Object(ProjectedPythonValue<'a>),
    /// The value has no valid JSON representation under this projection.
    Invalid,
}

/// Lazily projects application-defined Python objects into the JSON data model.
///
/// Implementations must return values borrowed from the root object graph or
/// from the provider itself. The validator never retains a projected value
/// beyond the root [`InstanceRef`] lifetime.
pub(crate) trait PythonInstanceProvider {
    /// Resolve a value to its logical JSON shape.
    fn project<'a>(&'a self, value: ProjectedPythonValue<'a>) -> ProjectedPythonKind<'a>;

    /// Return an array's logical length.
    fn array_len(&self, value: ProjectedPythonValue<'_>) -> usize;

    /// Return an array element and its projection node.
    fn array_get<'a>(
        &'a self,
        value: ProjectedPythonValue<'a>,
        index: usize,
    ) -> Option<ProjectedPythonValue<'a>>;

    /// Return an object's logical property count.
    fn object_len(&self, value: ProjectedPythonValue<'_>) -> usize;

    /// Return whether every logical object key is a string.
    ///
    /// Providers synthesizing keys from static metadata may use the default.
    fn object_keys_are_strings(&self, _value: ProjectedPythonValue<'_>) -> bool {
        true
    }

    /// Look up one logical object property.
    fn object_get<'a>(
        &'a self,
        value: ProjectedPythonValue<'a>,
        key: &str,
    ) -> Option<ProjectedPythonValue<'a>> {
        let mut state = [0, 0];
        while let Some((candidate, child)) = self.object_next(value, &mut state) {
            if candidate == key {
                return Some(child);
            }
        }
        None
    }

    /// Advance an object iterator.
    ///
    /// Both state words are initialized to zero and are exclusively owned by
    /// the iterator, so providers may interpret them however they choose.
    fn object_next<'a>(
        &'a self,
        value: ProjectedPythonValue<'a>,
        state: &mut [usize; 2],
    ) -> Option<(&'a str, ProjectedPythonValue<'a>)>;
}

#[derive(Clone, Copy)]
#[doc(hidden)]
pub(crate) struct ProjectedPythonInstance<'a> {
    provider: &'a dyn PythonInstanceProvider,
    value: ProjectedPythonValue<'a>,
}

impl std::fmt::Debug for ProjectedPythonInstance<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProjectedPythonInstance")
            .field("value", &self.value)
            .finish_non_exhaustive()
    }
}

impl<'a> InstanceRef<'a> {
    /// Borrow a `serde_json` value.
    #[must_use]
    pub(crate) const fn from_serde(value: &'a Value) -> Self {
        Self {
            repr: InstanceRepr::Serde(value),
        }
    }

    /// Borrow a jiter value.
    #[must_use]
    pub(crate) const fn from_jiter(value: &'a jiter::JsonValue<'a>) -> Self {
        Self {
            repr: InstanceRepr::Jiter(value),
        }
    }

    /// Borrow a Python JSON value while the interpreter lock is held.
    #[must_use]
    pub(crate) fn from_python<'py>(value: &'a Bound<'py, PyAny>) -> Self
    where
        'py: 'a,
    {
        Self {
            repr: InstanceRepr::Python(PythonValue::from_bound(value)),
        }
    }

    /// Borrow a Python object through an application-defined JSON projection.
    #[must_use]
    pub(crate) fn from_projected_python<'py>(
        value: &'a Bound<'py, PyAny>,
        node: usize,
        provider: &'a dyn PythonInstanceProvider,
    ) -> Self
    where
        'py: 'a,
    {
        Self {
            repr: InstanceRepr::ProjectedPython(ProjectedPythonInstance {
                provider,
                value: ProjectedPythonValue {
                    value: PythonValue::from_bound(value),
                    node,
                },
            }),
        }
    }

    fn from_python_value(value: PythonValue<'a>) -> Self {
        Self {
            repr: InstanceRepr::Python(value),
        }
    }

    /// Return the underlying serde value when this view was created from one.
    #[must_use]
    pub(crate) const fn as_serde(self) -> Option<&'a Value> {
        match self.repr {
            InstanceRepr::Serde(value) => Some(value),
            InstanceRepr::Jiter(_) => None,
            InstanceRepr::Python(_) => None,
            InstanceRepr::ProjectedPython(_) => None,
        }
    }

    #[must_use]
    pub(crate) fn is_null(self) -> bool {
        match self.repr {
            InstanceRepr::Serde(value) => value.is_null(),
            InstanceRepr::Jiter(value) => matches!(value, jiter::JsonValue::Null),
            InstanceRepr::Python(value) => value.borrowed().is_none(),
            InstanceRepr::ProjectedPython(value) => match value.provider.project(value.value) {
                ProjectedPythonKind::Native(value) => value.value.borrowed().is_none(),
                ProjectedPythonKind::Array(_)
                | ProjectedPythonKind::Object(_)
                | ProjectedPythonKind::Invalid => false,
            },
        }
    }

    #[must_use]
    pub(crate) fn is_boolean(self) -> bool {
        self.as_bool().is_some()
    }

    #[must_use]
    pub(crate) fn as_bool(self) -> Option<bool> {
        match self.repr {
            InstanceRepr::Serde(value) => value.as_bool(),
            InstanceRepr::Jiter(value) => match value {
                jiter::JsonValue::Bool(value) => Some(*value),
                _ => None,
            },
            InstanceRepr::Python(value) => value
                .borrowed()
                .cast::<PyBool>()
                .ok()
                .map(|value| value.is_true()),
            InstanceRepr::ProjectedPython(value) => match value.provider.project(value.value) {
                ProjectedPythonKind::Native(value) => value
                    .value
                    .borrowed()
                    .cast::<PyBool>()
                    .ok()
                    .map(|value| value.is_true()),
                ProjectedPythonKind::Array(_)
                | ProjectedPythonKind::Object(_)
                | ProjectedPythonKind::Invalid => None,
            },
        }
    }

    #[must_use]
    pub(crate) fn is_string(self) -> bool {
        self.as_str().is_some()
    }

    #[must_use]
    pub(crate) fn as_str(self) -> Option<&'a str> {
        match self.repr {
            InstanceRepr::Serde(value) => value.as_str(),
            InstanceRepr::Jiter(value) => match value {
                jiter::JsonValue::Str(value) => Some(value.as_ref()),
                _ => None,
            },
            InstanceRepr::Python(value) => python_string(value),
            InstanceRepr::ProjectedPython(value) => match value.provider.project(value.value) {
                ProjectedPythonKind::Native(value) => python_string(value.value),
                ProjectedPythonKind::Array(_)
                | ProjectedPythonKind::Object(_)
                | ProjectedPythonKind::Invalid => None,
            },
        }
    }

    #[must_use]
    pub(crate) fn is_number(self) -> bool {
        self.as_number().is_some()
    }

    #[must_use]
    pub(crate) fn as_number(self) -> Option<NumberRef<'a>> {
        match self.repr {
            InstanceRepr::Serde(value) => match value {
                Value::Number(value) => Some(NumberRef::Serde(value)),
                _ => None,
            },
            InstanceRepr::Jiter(value) => match value {
                jiter::JsonValue::Int(value) => Some(NumberRef::Integer(*value)),
                jiter::JsonValue::BigInt(value) => Some(NumberRef::BigInteger(value)),
                jiter::JsonValue::Float(value) => Some(NumberRef::Float(*value)),
                _ => None,
            },
            InstanceRepr::Python(value) => {
                let borrowed = value.borrowed();
                if borrowed.cast::<PyBool>().is_ok() {
                    None
                } else if borrowed.cast::<PyInt>().is_ok() {
                    Some(NumberRef::PythonInteger(value))
                } else {
                    borrowed
                        .cast::<PyFloat>()
                        .ok()
                        .and_then(|value| value.value().is_finite().then(|| value.value()))
                        .map(NumberRef::Float)
                }
            }
            InstanceRepr::ProjectedPython(value) => match value.provider.project(value.value) {
                ProjectedPythonKind::Native(value) => {
                    let borrowed = value.value.borrowed();
                    if borrowed.cast::<PyBool>().is_ok() {
                        None
                    } else if borrowed.cast::<PyInt>().is_ok() {
                        Some(NumberRef::PythonInteger(value.value))
                    } else {
                        borrowed
                            .cast::<PyFloat>()
                            .ok()
                            .and_then(|value| value.value().is_finite().then(|| value.value()))
                            .map(NumberRef::Float)
                    }
                }
                ProjectedPythonKind::Array(_)
                | ProjectedPythonKind::Object(_)
                | ProjectedPythonKind::Invalid => None,
            },
        }
    }

    #[must_use]
    pub(crate) fn as_array(self) -> Option<ArrayRef<'a>> {
        match self.repr {
            InstanceRepr::Serde(value) => match value {
                Value::Array(items) => Some(ArrayRef::Serde(items)),
                _ => None,
            },
            InstanceRepr::Jiter(value) => match value {
                jiter::JsonValue::Array(items) => Some(ArrayRef::Jiter(items)),
                _ => None,
            },
            InstanceRepr::Python(value) => {
                let borrowed = value.borrowed();
                if borrowed.cast::<PyList>().is_ok() {
                    Some(ArrayRef::PythonList(value))
                } else if borrowed.cast::<PyTuple>().is_ok() {
                    Some(ArrayRef::PythonTuple(value))
                } else {
                    None
                }
            }
            InstanceRepr::ProjectedPython(instance) => {
                match instance.provider.project(instance.value) {
                    ProjectedPythonKind::Native(value) => {
                        InstanceRef::from_python_value(value.value).as_array()
                    }
                    ProjectedPythonKind::Array(value) => {
                        Some(ArrayRef::Projected(ProjectedPythonInstance {
                            provider: instance.provider,
                            value,
                        }))
                    }
                    ProjectedPythonKind::Object(_) | ProjectedPythonKind::Invalid => None,
                }
            }
        }
    }

    #[must_use]
    pub(crate) fn as_object(self) -> Option<ObjectRef<'a>> {
        match self.repr {
            InstanceRepr::Serde(value) => match value {
                Value::Object(properties) => Some(ObjectRef::Serde(properties)),
                _ => None,
            },
            InstanceRepr::Jiter(value) => match value {
                jiter::JsonValue::Object(properties) => Some(ObjectRef::Jiter(properties)),
                _ => None,
            },
            InstanceRepr::Python(value) => value
                .borrowed()
                .cast::<PyDict>()
                .is_ok()
                .then_some(ObjectRef::Python(value)),
            InstanceRepr::ProjectedPython(instance) => {
                match instance.provider.project(instance.value) {
                    ProjectedPythonKind::Native(value) => {
                        InstanceRef::from_python_value(value.value).as_object()
                    }
                    ProjectedPythonKind::Object(value) => {
                        Some(ObjectRef::Projected(ProjectedPythonInstance {
                            provider: instance.provider,
                            value,
                        }))
                    }
                    ProjectedPythonKind::Array(_) | ProjectedPythonKind::Invalid => None,
                }
            }
        }
    }

    /// Stable identity of this node for recursive-schema cycle detection.
    #[must_use]
    pub(crate) fn identity(self) -> InstanceIdentity {
        match self.repr {
            InstanceRepr::Serde(value) => InstanceIdentity {
                representation: 0,
                node: 0,
                value: std::ptr::from_ref(value).cast::<()>() as usize,
            },
            InstanceRepr::Jiter(value) => InstanceIdentity {
                representation: 1,
                node: 0,
                value: std::ptr::from_ref(value).cast::<()>() as usize,
            },
            InstanceRepr::Python(value) => InstanceIdentity {
                representation: 2,
                node: 0,
                value: value.object.as_ptr().cast::<()>() as usize,
            },
            InstanceRepr::ProjectedPython(value) => InstanceIdentity {
                representation: std::ptr::from_ref::<dyn PythonInstanceProvider>(value.provider)
                    .cast::<()>() as usize,
                node: value.value.node,
                value: value.value.value.object.as_ptr().cast::<()>() as usize,
            },
        }
    }

    /// Materialize this view as a serde value.
    ///
    /// # Panics
    ///
    /// Panics if a borrowed number exceeds `serde_json`'s numeric range. Enable
    /// `arbitrary-precision`, or use [`Self::try_to_owned`] to handle this case.
    #[must_use]
    pub(crate) fn to_owned(self) -> Value {
        self.try_to_owned()
            .expect("borrowed number exceeds serde_json's range; enable arbitrary-precision")
    }

    /// Materialize this view using `serde_json`'s configured numeric range.
    ///
    /// Without `arbitrary-precision`, decimals may round and large numbers may
    /// fail conversion. Borrowed validation can still inspect native numbers.
    ///
    /// # Errors
    ///
    /// Returns an error when a borrowed number cannot be represented.
    pub(crate) fn try_to_owned(self) -> Result<Value, serde_json::Error> {
        match self.repr {
            InstanceRepr::Serde(value) => Ok(value.clone()),
            InstanceRepr::Jiter(value) => jiter_to_serde(value),
            InstanceRepr::Python(value) => python_to_serde(value),
            InstanceRepr::ProjectedPython(_) => borrowed_instance_to_serde(self),
        }
    }

    /// Return whether this view represents a value in the JSON data model.
    ///
    /// Serde and jiter values are JSON by construction. Python values require
    /// a recursive check for unsupported objects, non-string mapping keys,
    /// non-finite numbers, and container cycles.
    #[must_use]
    pub(crate) fn is_json(self) -> bool {
        if matches!(
            self.repr,
            InstanceRepr::Python(_) | InstanceRepr::ProjectedPython(_)
        ) {
            return python_value_is_json(self, &mut std::collections::HashSet::new());
        }
        true
    }

    pub(crate) fn equals(self, expected: &Value) -> bool {
        match expected {
            Value::Null => self.is_null(),
            Value::Bool(expected) => self.as_bool().is_some_and(|value| value == *expected),
            Value::String(expected) => self.as_str().is_some_and(|value| value == expected),
            _ => self
                .try_to_owned()
                .is_ok_and(|value| json_schema_ast::json_values_equal(&value, expected)),
        }
    }
}

impl<'a> From<&'a Value> for InstanceRef<'a> {
    fn from(value: &'a Value) -> Self {
        Self::from_serde(value)
    }
}

/// A borrowed JSON number.
#[derive(Clone, Copy, Debug)]
pub(crate) enum NumberRef<'a> {
    Serde(&'a Number),
    Integer(i64),
    BigInteger(&'a num_bigint::BigInt),
    PythonInteger(PythonValue<'a>),
    Float(f64),
}

impl NumberRef<'_> {
    #[must_use]
    pub(crate) fn as_i64(self) -> Option<i64> {
        match self {
            Self::Serde(value) => value.as_i64(),
            Self::Integer(value) => Some(value),
            Self::BigInteger(_) | Self::Float(_) => None,

            Self::PythonInteger(value) => value.borrowed().extract::<i64>().ok(),
        }
    }

    #[must_use]
    pub(crate) fn as_u64(self) -> Option<u64> {
        match self {
            Self::Serde(value) => value.as_u64(),
            Self::Integer(value) => u64::try_from(value).ok(),
            Self::BigInteger(_) | Self::Float(_) => None,

            Self::PythonInteger(value) => value.borrowed().extract::<u64>().ok(),
        }
    }

    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub(crate) fn as_f64(self) -> Option<f64> {
        match self {
            Self::Serde(value) => value.as_f64(),
            Self::Integer(value) => Some(value as f64),
            Self::BigInteger(value) => {
                use num_traits::ToPrimitive;
                value.to_f64()
            }
            Self::PythonInteger(value) => value.borrowed().extract::<f64>().ok(),
            Self::Float(value) => Some(value),
        }
    }

    #[must_use]
    pub(crate) fn is_integer(self) -> bool {
        match self {
            Self::Serde(value) => {
                value.is_i64()
                    || value.is_u64()
                    || value.as_f64().is_some_and(|value| value.fract() == 0.0)
            }
            Self::Integer(_) => true,
            Self::BigInteger(_) => true,
            Self::PythonInteger(_) => true,
            Self::Float(value) => value.fract() == 0.0,
        }
    }
}

/// A borrowed JSON array.
#[derive(Clone, Copy, Debug)]
pub(crate) enum ArrayRef<'a> {
    Serde(&'a [Value]),
    Jiter(&'a [jiter::JsonValue<'a>]),
    PythonList(PythonValue<'a>),
    PythonTuple(PythonValue<'a>),
    Projected(ProjectedPythonInstance<'a>),
}

impl<'a> ArrayRef<'a> {
    /// Returns the number of items in this array.
    ///
    /// # Panics
    ///
    /// Panics only if `CPython` reports a negative length for an exact `list` or
    /// `tuple`, which would violate the `CPython` C API contract.
    #[must_use]
    pub(crate) fn len(self) -> usize {
        match self {
            Self::Serde(items) => items.len(),
            Self::Jiter(items) => items.len(),
            Self::PythonList(value) => {
                // SAFETY: the variant is constructed only after an exact list cast.
                usize::try_from(unsafe { ffi::PyList_Size(value.object.as_ptr()) })
                    .expect("Python list lengths are non-negative")
            }
            Self::PythonTuple(value) => {
                // SAFETY: the variant is constructed only after an exact tuple cast.
                usize::try_from(unsafe { ffi::PyTuple_Size(value.object.as_ptr()) })
                    .expect("Python tuple lengths are non-negative")
            }
            Self::Projected(value) => value.provider.array_len(value.value),
        }
    }

    #[must_use]
    pub(crate) fn iter(self) -> ArrayIter<'a> {
        match self {
            Self::Serde(items) => ArrayIter::Serde(items.iter()),
            Self::Jiter(items) => ArrayIter::Jiter(items.iter()),
            Self::PythonList(value) => ArrayIter::Python {
                value,
                index: 0,
                len: self.len(),
                tuple: false,
            },
            Self::PythonTuple(value) => ArrayIter::Python {
                value,
                index: 0,
                len: self.len(),
                tuple: true,
            },
            Self::Projected(value) => ArrayIter::Projected {
                value,
                index: 0,
                len: self.len(),
            },
        }
    }
}

impl<'a> IntoIterator for ArrayRef<'a> {
    type Item = InstanceRef<'a>;
    type IntoIter = ArrayIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub(crate) enum ArrayIter<'a> {
    Serde(std::slice::Iter<'a, Value>),
    Jiter(std::slice::Iter<'a, jiter::JsonValue<'a>>),
    Python {
        value: PythonValue<'a>,
        index: usize,
        len: usize,
        tuple: bool,
    },
    Projected {
        value: ProjectedPythonInstance<'a>,
        index: usize,
        len: usize,
    },
}

impl<'a> Iterator for ArrayIter<'a> {
    type Item = InstanceRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Serde(items) => items.next().map(InstanceRef::from_serde),
            Self::Jiter(items) => items.next().map(InstanceRef::from_jiter),
            Self::Python {
                value,
                index,
                len,
                tuple,
            } => {
                if *index >= *len {
                    return None;
                }
                // SAFETY: the index is in bounds and the variant records the
                // concrete container type checked at construction.
                let python_index = ffi::Py_ssize_t::try_from(*index)
                    .expect("an in-bounds Python sequence index fits Py_ssize_t");
                let item = unsafe {
                    if *tuple {
                        ffi::PyTuple_GetItem(value.object.as_ptr(), python_index)
                    } else {
                        ffi::PyList_GetItem(value.object.as_ptr(), python_index)
                    }
                };
                *index += 1;
                // SAFETY: the container owns this borrowed child for `'a`.
                Some(InstanceRef::from_python_value(unsafe {
                    PythonValue::from_ptr(value.py, item)
                }))
            }
            Self::Projected { value, index, len } => {
                if *index >= *len {
                    return None;
                }
                let child = value.provider.array_get(value.value, *index)?;
                *index += 1;
                Some(InstanceRef {
                    repr: InstanceRepr::ProjectedPython(ProjectedPythonInstance {
                        provider: value.provider,
                        value: child,
                    }),
                })
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Serde(items) => items.size_hint(),
            Self::Jiter(items) => items.size_hint(),
            Self::Python { index, len, .. } => {
                let remaining = len.saturating_sub(*index);
                (remaining, Some(remaining))
            }
            Self::Projected { index, len, .. } => {
                let remaining = len.saturating_sub(*index);
                (remaining, Some(remaining))
            }
        }
    }
}

impl ExactSizeIterator for ArrayIter<'_> {}
impl FusedIterator for ArrayIter<'_> {}

/// A borrowed JSON object.
#[derive(Clone, Copy, Debug)]
pub(crate) enum ObjectRef<'a> {
    Serde(&'a Map<String, Value>),
    Jiter(&'a [(std::borrow::Cow<'a, str>, jiter::JsonValue<'a>)]),
    Python(PythonValue<'a>),
    Projected(ProjectedPythonInstance<'a>),
}

impl<'a> ObjectRef<'a> {
    /// Returns the number of properties in this object.
    ///
    /// # Panics
    ///
    /// Panics only if `CPython` reports a negative length for an exact `dict`,
    /// which would violate the `CPython` C API contract.
    #[must_use]
    pub(crate) fn len(self) -> usize {
        match self {
            Self::Serde(properties) => properties.len(),
            Self::Jiter(properties) => properties.len(),
            Self::Python(value) => {
                // SAFETY: the variant is created only after a dict cast.
                usize::try_from(unsafe { ffi::PyDict_Size(value.object.as_ptr()) })
                    .expect("Python dict lengths are non-negative")
            }
            Self::Projected(value) => value.provider.object_len(value.value),
        }
    }

    #[must_use]
    pub(crate) fn get(self, key: &str) -> Option<InstanceRef<'a>> {
        match self {
            Self::Serde(properties) => properties.get(key).map(InstanceRef::from_serde),
            Self::Jiter(properties) => properties
                .iter()
                .find(|(candidate, _)| candidate == key)
                .map(|(_, value)| InstanceRef::from_jiter(value)),
            Self::Python(_) => self
                .iter()
                .find_map(|(candidate, value)| (candidate == key).then_some(value)),
            Self::Projected(value) => {
                value
                    .provider
                    .object_get(value.value, key)
                    .map(|child| InstanceRef {
                        repr: InstanceRepr::ProjectedPython(ProjectedPythonInstance {
                            provider: value.provider,
                            value: child,
                        }),
                    })
            }
        }
    }

    #[must_use]
    pub(crate) fn contains_key(self, key: &str) -> bool {
        self.get(key).is_some()
    }

    #[must_use]
    pub(crate) fn iter(self) -> ObjectIter<'a> {
        match self {
            Self::Serde(properties) => ObjectIter::Serde(properties.iter()),
            Self::Jiter(properties) => ObjectIter::Jiter(properties.iter()),
            Self::Python(value) => ObjectIter::Python {
                value,
                position: 0,
                remaining: self.len(),
            },
            Self::Projected(value) => ObjectIter::Projected {
                value,
                state: [0, 0],
                remaining: self.len(),
            },
        }
    }
}

impl<'a> IntoIterator for ObjectRef<'a> {
    type Item = (&'a str, InstanceRef<'a>);
    type IntoIter = ObjectIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub(crate) enum ObjectIter<'a> {
    Serde(serde_json::map::Iter<'a>),
    Jiter(std::slice::Iter<'a, (std::borrow::Cow<'a, str>, jiter::JsonValue<'a>)>),
    Python {
        value: PythonValue<'a>,
        position: ffi::Py_ssize_t,
        remaining: usize,
    },
    Projected {
        value: ProjectedPythonInstance<'a>,
        state: [usize; 2],
        remaining: usize,
    },
}

impl<'a> Iterator for ObjectIter<'a> {
    type Item = (&'a str, InstanceRef<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Serde(properties) => properties
                .next()
                .map(|(key, value)| (key.as_str(), InstanceRef::from_serde(value))),
            Self::Jiter(properties) => properties
                .next()
                .map(|(key, value)| (key.as_ref(), InstanceRef::from_jiter(value))),
            Self::Python {
                value,
                position,
                remaining,
            } => {
                let mut key = std::ptr::null_mut();
                let mut child = std::ptr::null_mut();
                // SAFETY: the variant guarantees a valid dict pointer and
                // PyDict_Next initializes both borrowed output pointers on success.
                let present = unsafe {
                    ffi::PyDict_Next(
                        value.object.as_ptr(),
                        position,
                        &raw mut key,
                        &raw mut child,
                    )
                };
                if present == 0 {
                    *remaining = 0;
                    return None;
                }
                *remaining = remaining.saturating_sub(1);
                // SAFETY: successful PyDict_Next returns non-null borrowed
                // references kept alive by the dict for `'a`.
                let key = unsafe { PythonValue::from_ptr(value.py, key) };
                let child = unsafe { PythonValue::from_ptr(value.py, child) };
                Some((
                    python_string(key).unwrap_or(""),
                    InstanceRef::from_python_value(child),
                ))
            }
            Self::Projected {
                value,
                state,
                remaining,
            } => {
                let (key, child) = value.provider.object_next(value.value, state)?;
                *remaining = remaining.saturating_sub(1);
                Some((
                    key,
                    InstanceRef {
                        repr: InstanceRepr::ProjectedPython(ProjectedPythonInstance {
                            provider: value.provider,
                            value: child,
                        }),
                    },
                ))
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Serde(properties) => properties.size_hint(),
            Self::Jiter(properties) => properties.size_hint(),
            Self::Python { remaining, .. } => (*remaining, Some(*remaining)),
            Self::Projected { remaining, .. } => (*remaining, Some(*remaining)),
        }
    }
}

impl ExactSizeIterator for ObjectIter<'_> {}
impl FusedIterator for ObjectIter<'_> {}

fn python_value_is_json(
    instance: InstanceRef<'_>,
    active_containers: &mut std::collections::HashSet<InstanceIdentity>,
) -> bool {
    if instance.is_null() || instance.is_boolean() || instance.is_string() || instance.is_number() {
        return true;
    }
    if let Some(items) = instance.as_array() {
        let identity = instance.identity();
        if !active_containers.insert(identity) {
            return false;
        }
        let valid = items
            .iter()
            .all(|item| python_value_is_json(item, active_containers));
        active_containers.remove(&identity);
        return valid;
    }
    let identity = instance.identity();
    if !active_containers.insert(identity) {
        return false;
    }
    match instance.repr {
        InstanceRepr::Python(value) => {
            if value.borrowed().cast::<PyDict>().is_err() {
                active_containers.remove(&identity);
                return false;
            }
            let mut position = 0;
            let mut key = std::ptr::null_mut();
            let mut child = std::ptr::null_mut();
            loop {
                // SAFETY: `value` is a dict and PyDict_Next initializes borrowed
                // pointers whenever it returns non-zero.
                let present = unsafe {
                    ffi::PyDict_Next(
                        value.object.as_ptr(),
                        &raw mut position,
                        &raw mut key,
                        &raw mut child,
                    )
                };
                if present == 0 {
                    break;
                }
                // SAFETY: successful PyDict_Next returns non-null values owned by the dict.
                let key = unsafe { PythonValue::from_ptr(value.py, key) };
                if key.borrowed().cast::<PyString>().is_err() {
                    active_containers.remove(&identity);
                    return false;
                }
                // SAFETY: successful PyDict_Next returns a non-null value owned by the dict.
                let child = InstanceRef::from_python_value(unsafe {
                    PythonValue::from_ptr(value.py, child)
                });
                if !python_value_is_json(child, active_containers) {
                    active_containers.remove(&identity);
                    return false;
                }
            }
        }
        InstanceRepr::ProjectedPython(_) => {
            let Some(properties) = instance.as_object() else {
                active_containers.remove(&identity);
                return false;
            };
            if let ObjectRef::Projected(value) = properties
                && !value.provider.object_keys_are_strings(value.value)
            {
                active_containers.remove(&identity);
                return false;
            }
            for (_, child) in properties {
                if !python_value_is_json(child, active_containers) {
                    active_containers.remove(&identity);
                    return false;
                }
            }
        }
        _ => unreachable!("only Python representations require recursive JSON checks"),
    }
    active_containers.remove(&identity);
    true
}

fn python_string(value: PythonValue<'_>) -> Option<&str> {
    value.borrowed().cast::<PyString>().ok()?;
    let mut size = 0;
    // SAFETY: the cast above proves this is a Unicode object. CPython keeps the
    // UTF-8 cache alive for the lifetime of the object borrowed by `value`.
    let data = unsafe { ffi::PyUnicode_AsUTF8AndSize(value.object.as_ptr(), &raw mut size) };
    if data.is_null() {
        return None;
    }
    // SAFETY: CPython returned a valid UTF-8 buffer of `size` bytes.
    let size = usize::try_from(size).expect("Python Unicode lengths are non-negative");
    let bytes = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), size) };
    // SAFETY: PyUnicode_AsUTF8AndSize guarantees valid UTF-8.
    Some(unsafe { std::str::from_utf8_unchecked(bytes) })
}

fn python_to_serde(value: PythonValue<'_>) -> Result<Value, serde_json::Error> {
    let instance = InstanceRef::from_python_value(value);
    borrowed_instance_to_serde(instance)
}

fn borrowed_instance_to_serde(instance: InstanceRef<'_>) -> Result<Value, serde_json::Error> {
    Ok(if instance.is_null() {
        Value::Null
    } else if let Some(value) = instance.as_bool() {
        Value::Bool(value)
    } else if let Some(value) = instance.as_str() {
        Value::String(value.to_owned())
    } else if let Some(number) = instance.as_number() {
        match number {
            NumberRef::PythonInteger(value) => {
                if let Ok(value) = value.borrowed().extract::<i64>() {
                    Value::Number(Number::from(value))
                } else if let Ok(value) = value.borrowed().extract::<u64>() {
                    Value::Number(Number::from(value))
                } else {
                    let rendered = value
                        .borrowed()
                        .str()
                        .expect("Python integers always have a string representation");
                    serde_json::from_str(
                        rendered
                            .to_str()
                            .expect("Python integer strings are always valid UTF-8"),
                    )?
                }
            }
            NumberRef::Float(value) => Number::from_f64(value).map_or(Value::Null, Value::Number),
            NumberRef::Serde(value) => Value::Number(value.clone()),
            NumberRef::Integer(value) => Value::Number(Number::from(value)),
            NumberRef::BigInteger(value) => serde_json::from_str(&value.to_string())?,
        }
    } else if let Some(items) = instance.as_array() {
        Value::Array(
            items
                .iter()
                .map(InstanceRef::try_to_owned)
                .collect::<Result<_, _>>()?,
        )
    } else if let Some(properties) = instance.as_object() {
        Value::Object(
            properties
                .iter()
                .map(|(key, value)| value.try_to_owned().map(|value| (key.to_owned(), value)))
                .collect::<Result<_, _>>()?,
        )
    } else {
        Value::Null
    })
}

fn jiter_to_serde(value: &jiter::JsonValue<'_>) -> Result<Value, serde_json::Error> {
    Ok(match value {
        jiter::JsonValue::Null => Value::Null,
        jiter::JsonValue::Bool(value) => Value::Bool(*value),
        jiter::JsonValue::Int(value) => Value::Number(Number::from(*value)),
        jiter::JsonValue::BigInt(value) => serde_json::from_str(&value.to_string())?,
        jiter::JsonValue::Float(value) => {
            Value::Number(Number::from_f64(*value).expect("jiter JSON numbers are always finite"))
        }
        jiter::JsonValue::Str(value) => Value::String(value.to_string()),
        jiter::JsonValue::Array(items) => {
            Value::Array(items.iter().map(jiter_to_serde).collect::<Result<_, _>>()?)
        }
        jiter::JsonValue::Object(properties) => Value::Object(
            properties
                .iter()
                .map(|(key, value)| jiter_to_serde(value).map(|value| (key.to_string(), value)))
                .collect::<Result<_, _>>()?,
        ),
    })
}
