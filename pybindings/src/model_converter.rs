//! Compiled conversion plans for generated Python dataclasses.
//!
//! Plans move repeated object-graph traversal into Rust while retaining the
//! Python runtime's existing type checks, missing-field factories, union
//! selection, and frozen-slot construction semantics.

use std::cell::{OnceCell, RefCell};
use std::collections::{HashMap, HashSet};
use std::num::NonZeroIsize;
use std::ops::Deref;
use std::rc::Rc;

use ::jsoncompat::SchemaDocument;
use jiter::JsonValue as JiterJsonValue;
use jsonschema::{
    InstanceRef as JsonInstanceRef, ProjectedPythonKind, ProjectedPythonValue,
    PythonInstanceProvider,
};
use pyo3::Borrowed;
use pyo3::exceptions::{PyIndexError, PyTypeError, PyValueError};
use pyo3::ffi;
use pyo3::prelude::*;
use pyo3::pyclass::{PyTraverseError, PyVisit};
use pyo3::types::{
    PyAny, PyBool, PyBytes, PyDict, PyFloat, PyInt, PyList, PyMapping, PySequence, PyString,
    PyTuple, PyType,
};

const MAX_MODEL_DEPTH: u16 = 255;

#[derive(Clone, Copy)]
enum ScalarKind {
    Any,
    Missing,
    String,
    Integer,
    Number,
    Boolean,
    Null,
}

struct DiscriminatorPlan {
    json_name: String,
    branches_by_value: HashMap<DiscriminatorKey, usize>,
}

#[derive(Eq, Hash, PartialEq)]
enum DiscriminatorKey {
    Null,
    Boolean(bool),
    Integer(i64),
    String(String),
}

#[derive(Clone, Copy)]
struct ValidatedSlotOffset(NonZeroIsize);

#[derive(Clone, Copy)]
enum AttributeStorage {
    NativeSlot(ValidatedSlotOffset),
    GenericDescriptor,
}

struct ModelAttribute {
    name: Py<PyString>,
    storage: AttributeStorage,
}

impl ModelAttribute {
    fn compile(py: Python<'_>, model_type: &Bound<'_, PyType>, name: &str) -> Self {
        Self {
            name: PyString::new(py, name).unbind(),
            storage: validated_slot_offset(model_type, name)
                .map(AttributeStorage::NativeSlot)
                .unwrap_or(AttributeStorage::GenericDescriptor),
        }
    }

    #[inline(always)]
    fn native_value_ptr(
        &self,
        py: Python<'_>,
        instance: &Bound<'_, PyAny>,
        model_type: &Py<PyType>,
    ) -> Option<*mut ffi::PyObject> {
        self.native_value_ptr_from_object(py, instance.as_ptr(), model_type)
    }

    #[inline(always)]
    fn native_value_ptr_from_object(
        &self,
        py: Python<'_>,
        object: *mut ffi::PyObject,
        model_type: &Py<PyType>,
    ) -> Option<*mut ffi::PyObject> {
        let AttributeStorage::NativeSlot(offset) = self.storage else {
            return None;
        };
        if unsafe { ffi::Py_TYPE(object) } != model_type.bind(py).as_ptr().cast() {
            return None;
        }
        // SAFETY: `ValidatedSlotOffset` is only created after proving that the
        // named member descriptor belongs to `model_type` (or one of its
        // bases) and identifies an aligned object-pointer slot within the
        // concrete allocation.
        let slot = unsafe {
            object
                .cast::<u8>()
                .offset(offset.0.get())
                .cast::<*mut ffi::PyObject>()
        };
        Some(unsafe { *slot })
    }

    #[inline(always)]
    fn get<'py>(
        &self,
        instance: &Bound<'py, PyAny>,
        model_type: &Py<PyType>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = instance.py();
        if let Some(value) = self.native_value_ptr(py, instance, model_type)
            && !value.is_null()
        {
            // SAFETY: the instance owns the slot reference for the returned
            // bound object's lifetime.
            return Ok(unsafe { Bound::from_borrowed_ptr(py, value) });
        }
        instance.getattr(self.name.bind(py))
    }

    #[inline(always)]
    fn is_set(
        &self,
        py: Python<'_>,
        instance: &Bound<'_, PyAny>,
        model_type: &Py<PyType>,
    ) -> PyResult<bool> {
        if let Some(value) = self.native_value_ptr(py, instance, model_type) {
            return Ok(!value.is_null());
        }
        instance.hasattr(self.name.bind(py))
    }

    #[inline(always)]
    fn set(
        &self,
        py: Python<'_>,
        instance: &Bound<'_, PyAny>,
        model_type: &Py<PyType>,
        value: &Py<PyAny>,
    ) -> PyResult<()> {
        if let AttributeStorage::NativeSlot(offset) = self.storage {
            let object = instance.as_ptr();
            if unsafe { ffi::Py_TYPE(object) } == model_type.bind(py).as_ptr().cast() {
                // SAFETY: `ValidatedSlotOffset` proves this is the named owned
                // object-pointer slot for the exact allocation. Retain the new
                // value before replacing and releasing the previous reference.
                let slot = unsafe {
                    object
                        .cast::<u8>()
                        .offset(offset.0.get())
                        .cast::<*mut ffi::PyObject>()
                };
                let value = value.bind(py).as_ptr();
                unsafe {
                    ffi::Py_INCREF(value);
                    let previous = std::ptr::replace(slot, value);
                    ffi::Py_XDECREF(previous);
                }
                return Ok(());
            }
        }

        // Frozen dataclasses intentionally reject `PyObject_SetAttr`; calling
        // the generic implementation is the portable descriptor fallback.
        let result = unsafe {
            ffi::PyObject_GenericSetAttr(
                instance.as_ptr(),
                self.name.bind(py).as_ptr(),
                value.bind(py).as_ptr(),
            )
        };
        if result == 0 {
            Ok(())
        } else {
            Err(PyErr::fetch(py))
        }
    }
}

struct FieldPlan {
    json_name: String,
    attribute: ModelAttribute,
    value_node: usize,
    missing_sentinel: Option<Py<PyAny>>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum UnionSelection {
    FirstRepresentable,
    ValidateAmbiguousBranches,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum MappingPolicy {
    Normalize,
    RejectWithoutTraversal,
}

enum ConversionFailure {
    Mismatch(PyErr),
    Raised(PyErr),
}

impl ConversionFailure {
    #[inline]
    fn into_pyerr(self) -> PyErr {
        match self {
            Self::Mismatch(error) | Self::Raised(error) => error,
        }
    }
}

impl From<PyErr> for ConversionFailure {
    fn from(error: PyErr) -> Self {
        Self::Raised(error)
    }
}

type ConversionResult<T> = Result<T, ConversionFailure>;

struct PythonConversionState {
    union_selection: UnionSelection,
    mapping_policy: MappingPolicy,
    ambiguous_union: bool,
}

pub(crate) struct ConstructedModelCandidate {
    pub(crate) instance: Py<PyAny>,
    pub(crate) ambiguous_union: bool,
}

pub(crate) enum CandidateConstruction {
    Constructed(ConstructedModelCandidate),
    Mismatch(PyErr),
}

struct JiterConversionState {
    union_selection: UnionSelection,
}

struct BranchSchema {
    raw: serde_json::Value,
    compiled: OnceCell<SchemaDocument>,
}

impl BranchSchema {
    fn compiled(&self) -> PyResult<&SchemaDocument> {
        if self.compiled.get().is_none() {
            let compiled = super::validated_schema(&self.raw).map_err(|error| {
                PyErr::new::<PyValueError, _>(format!("Invalid schema: {error}"))
            })?;
            self.compiled.set(compiled).map_err(|_| {
                PyErr::new::<PyValueError, _>("generated model schema initialized recursively")
            })?;
        }
        Ok(self
            .compiled
            .get()
            .expect("branch schema was initialized immediately above"))
    }

    fn is_valid_instance(&self, instance: JsonInstanceRef<'_>) -> PyResult<bool> {
        self.compiled()?
            .is_valid_instance(instance)
            .map_err(super::validation_error)
    }
}

enum ConversionNode {
    Scalar {
        kind: ScalarKind,
        missing_sentinel: Option<Py<PyAny>>,
    },
    List {
        item: usize,
    },
    Dict {
        key: usize,
        value: usize,
    },
    Literal {
        values: Vec<Py<PyAny>>,
        string_indices: HashMap<String, usize>,
    },
    Union {
        branches: Vec<usize>,
        discriminator: Option<DiscriminatorPlan>,
    },
    Model {
        model_type: Py<PyType>,
        validated_attribute: ModelAttribute,
        branch_schema: BranchSchema,
        prevalidated_schema: Option<serde_json::Value>,
        fields: Vec<FieldPlan>,
        fields_by_json_name: HashMap<String, usize>,
        fields_by_py_name: HashMap<String, usize>,
        serialized_fields: Vec<usize>,
        required_field_count: usize,
        omittable_fields: Vec<usize>,
        extra_value: Option<usize>,
        extra_attribute: Option<ModelAttribute>,
    },
    Root {
        model_type: Py<PyType>,
        validated_attribute: ModelAttribute,
        branch_schema: BranchSchema,
        prevalidated_schema: Option<serde_json::Value>,
        value: usize,
        root_attribute: ModelAttribute,
    },
}

pub(crate) struct ModelConverterPlan {
    nodes: Vec<ConversionNode>,
    has_prevalidated_schemas: bool,
    object_new: Py<PyAny>,
    frozen_list_type: Py<PyType>,
    frozen_dict_type: Py<PyType>,
    frozen_dict_items_attribute: ModelAttribute,
}

pub(crate) struct ModelConverterPy {
    plan: Rc<ModelConverterPlan>,
    root: usize,
}

impl Deref for ModelConverterPy {
    type Target = ModelConverterPlan;

    fn deref(&self) -> &Self::Target {
        &self.plan
    }
}

type FrozenDictPair = (Py<PyAny>, Py<PyAny>);

struct KnownUniqueFrozenDictEntries(Vec<FrozenDictPair>);

struct KnownUniqueFrozenDictBuilder {
    entries: Vec<FrozenDictPair>,
}

impl KnownUniqueFrozenDictBuilder {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
        }
    }

    #[inline]
    fn push(&mut self, key: Py<PyAny>, value: Py<PyAny>) {
        self.entries.push((key, value));
    }

    fn finish(self) -> KnownUniqueFrozenDictEntries {
        KnownUniqueFrozenDictEntries(self.entries)
    }
}

struct PythonFrozenDictBuilder {
    entries: Vec<FrozenDictPair>,
    requires_normalization: bool,
}

impl PythonFrozenDictBuilder {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
            requires_normalization: false,
        }
    }

    #[inline]
    fn push(&mut self, source_key: &Bound<'_, PyAny>, canonical_key: Py<PyAny>, value: Py<PyAny>) {
        self.requires_normalization |= !source_key.is_exact_instance_of::<PyString>();
        self.entries.push((canonical_key, value));
    }

    #[inline]
    fn finish(self, py: Python<'_>, converter: &ModelConverterPy) -> PyResult<Py<PyAny>> {
        if !self.requires_normalization {
            return converter
                .freeze_known_unique_dict(py, KnownUniqueFrozenDictEntries(self.entries));
        }

        let normalized = PyDict::new(py);
        for (key, value) in self.entries {
            normalized.set_item(key, value)?;
        }
        converter.freeze_normalized_dict(py, &normalized)
    }
}

struct NormalizingFrozenDictBuilder {
    entries: Py<PyDict>,
}

impl NormalizingFrozenDictBuilder {
    fn new(py: Python<'_>) -> Self {
        Self {
            entries: PyDict::new(py).unbind(),
        }
    }

    #[inline]
    fn insert(&self, py: Python<'_>, key: Py<PyAny>, value: Py<PyAny>) -> PyResult<()> {
        self.entries.bind(py).set_item(key, value)
    }

    #[inline]
    fn finish(self, py: Python<'_>, converter: &ModelConverterPy) -> PyResult<Py<PyAny>> {
        converter.freeze_normalized_dict(py, self.entries.bind(py))
    }
}

struct JiterFrozenDictBuilder<'a> {
    entries: Vec<FrozenDictPair>,
    seen: JiterSeenKeys<'a>,
}

enum JiterSeenKeys<'a> {
    Empty,
    One(&'a str),
    Two(&'a str, &'a str),
    Many(HashSet<&'a str>),
}

impl<'a> JiterSeenKeys<'a> {
    #[inline]
    fn insert(&mut self, key: &'a str, capacity: usize) -> bool {
        match self {
            Self::Empty => {
                *self = Self::One(key);
                true
            }
            Self::One(first) => {
                if *first == key {
                    return false;
                }
                *self = Self::Two(first, key);
                true
            }
            Self::Two(first, second) => {
                if *first == key || *second == key {
                    return false;
                }
                let mut seen = HashSet::with_capacity(capacity);
                seen.insert(*first);
                seen.insert(*second);
                seen.insert(key);
                *self = Self::Many(seen);
                true
            }
            Self::Many(seen) => seen.insert(key),
        }
    }
}

impl<'a> JiterFrozenDictBuilder<'a> {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
            seen: JiterSeenKeys::Empty,
        }
    }

    #[inline]
    fn push(
        &mut self,
        source_key: &'a str,
        canonical_key: Py<PyAny>,
        value: Py<PyAny>,
    ) -> PyResult<()> {
        if !self.seen.insert(source_key, self.entries.capacity()) {
            return Err(duplicate_key(source_key));
        }
        self.entries.push((canonical_key, value));
        Ok(())
    }

    fn finish(self) -> KnownUniqueFrozenDictEntries {
        KnownUniqueFrozenDictEntries(self.entries)
    }
}

impl ModelConverterPlan {
    pub(crate) fn traverse(&self, visit: &PyVisit<'_>) -> Result<(), PyTraverseError> {
        visit.call(&self.object_new)?;
        visit.call(&self.frozen_list_type)?;
        visit.call(&self.frozen_dict_type)?;
        visit.call(&self.frozen_dict_items_attribute.name)?;
        for node in &self.nodes {
            match node {
                ConversionNode::Scalar {
                    missing_sentinel, ..
                } => visit.call(missing_sentinel)?,
                ConversionNode::Literal { values, .. } => {
                    for value in values {
                        visit.call(value)?;
                    }
                }
                ConversionNode::Model {
                    model_type,
                    validated_attribute,
                    fields,
                    extra_attribute,
                    ..
                } => {
                    visit.call(model_type)?;
                    visit.call(&validated_attribute.name)?;
                    for field in fields {
                        visit.call(&field.attribute.name)?;
                        visit.call(&field.missing_sentinel)?;
                    }
                    if let Some(attribute) = extra_attribute {
                        visit.call(&attribute.name)?;
                    }
                }
                ConversionNode::Root {
                    model_type,
                    validated_attribute,
                    root_attribute,
                    ..
                } => {
                    visit.call(model_type)?;
                    visit.call(&validated_attribute.name)?;
                    visit.call(&root_attribute.name)?;
                }
                ConversionNode::List { .. }
                | ConversionNode::Dict { .. }
                | ConversionNode::Union { .. } => {}
            }
        }
        Ok(())
    }
}

pub(crate) struct ModelProjection<'a> {
    converter: &'a ModelConverterPy,
    retained: RefCell<Vec<Py<PyAny>>>,
    union_branches: RefCell<HashMap<(usize, usize), usize>>,
}

impl ModelConverterPy {
    pub(crate) fn construct(
        &self,
        py: Python<'_>,
        value: &Bound<'_, PyAny>,
        validated: bool,
    ) -> PyResult<Py<PyAny>> {
        let mut state = PythonConversionState {
            union_selection: if validated {
                UnionSelection::ValidateAmbiguousBranches
            } else {
                UnionSelection::FirstRepresentable
            },
            mapping_policy: MappingPolicy::Normalize,
            ambiguous_union: false,
        };
        let instance = self
            .convert(py, self.root, value, &mut state, MAX_MODEL_DEPTH)
            .map_err(ConversionFailure::into_pyerr)?;
        self.finalize(py, instance, validated)
    }

    pub(crate) fn construct_candidate(
        &self,
        py: Python<'_>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<CandidateConstruction> {
        let mut state = PythonConversionState {
            union_selection: UnionSelection::FirstRepresentable,
            mapping_policy: MappingPolicy::RejectWithoutTraversal,
            ambiguous_union: false,
        };
        match self.convert(py, self.root, value, &mut state, MAX_MODEL_DEPTH) {
            Ok(instance) => Ok(CandidateConstruction::Constructed(
                ConstructedModelCandidate {
                    instance,
                    ambiguous_union: state.ambiguous_union,
                },
            )),
            Err(ConversionFailure::Mismatch(error)) => Ok(CandidateConstruction::Mismatch(error)),
            Err(ConversionFailure::Raised(error)) => Err(error),
        }
    }
}

pub(crate) fn model_converter_for_validated_root(
    plan: Rc<ModelConverterPlan>,
    root: usize,
) -> ModelConverterPy {
    ModelConverterPy { plan, root }
}

impl ModelConverterPy {
    pub(crate) fn schema(&self) -> PyResult<&SchemaDocument> {
        match self.nodes.get(self.root) {
            Some(ConversionNode::Model { branch_schema, .. })
            | Some(ConversionNode::Root { branch_schema, .. }) => branch_schema.compiled(),
            Some(_) => Err(PyErr::new::<PyTypeError, _>(
                "model converter root must be a generated model",
            )),
            None => Err(PyErr::new::<PyIndexError, _>(
                "model converter root node is out of bounds",
            )),
        }
    }

    pub(crate) fn projection(&self) -> ModelProjection<'_> {
        ModelProjection {
            converter: self,
            retained: RefCell::new(Vec::new()),
            union_branches: RefCell::new(HashMap::new()),
        }
    }

    pub(crate) fn construct_kwargs_unvalidated(
        &self,
        py: Python<'_>,
        kwargs: &Bound<'_, PyDict>,
    ) -> PyResult<(Py<PyAny>, bool)> {
        let mut json_proven = true;
        let node = self.nodes.get(self.root).ok_or_else(|| {
            PyErr::new::<PyIndexError, _>(format!(
                "model converter root node {} is missing",
                self.root
            ))
        })?;
        match node {
            ConversionNode::Model {
                model_type,
                fields,
                fields_by_json_name,
                fields_by_py_name,
                extra_value,
                extra_attribute,
                ..
            } => self
                .convert_model_kwargs(
                    py,
                    model_type,
                    fields,
                    fields_by_json_name,
                    fields_by_py_name,
                    *extra_value,
                    extra_attribute.as_ref(),
                    kwargs,
                    &mut json_proven,
                )
                .map(|instance| (instance, json_proven)),
            ConversionNode::Root {
                model_type,
                value,
                root_attribute,
                ..
            } => self
                .convert_root_kwargs(
                    py,
                    model_type,
                    *value,
                    root_attribute,
                    kwargs,
                    &mut json_proven,
                )
                .map(|instance| (instance, json_proven)),
            _ => Err(PyErr::new::<PyTypeError, _>(
                "model converter root must be a generated model",
            )),
        }
    }

    pub(crate) fn mark_validated(
        &self,
        py: Python<'_>,
        instance: Py<PyAny>,
    ) -> PyResult<Py<PyAny>> {
        self.finalize(py, instance, true)
    }

    pub(crate) fn construct_jiter(
        &self,
        py: Python<'_>,
        value: &JiterJsonValue<'_>,
        validated: bool,
    ) -> PyResult<Py<PyAny>> {
        let mut state = JiterConversionState {
            union_selection: if validated {
                UnionSelection::ValidateAmbiguousBranches
            } else {
                UnionSelection::FirstRepresentable
            },
        };
        let instance = self.convert_jiter(py, self.root, value, &mut state)?;
        self.finalize(py, instance, validated)
    }

    pub(crate) fn finalize(
        &self,
        py: Python<'_>,
        instance: Py<PyAny>,
        validated: bool,
    ) -> PyResult<Py<PyAny>> {
        let (model_type, validated_attribute) = match self.nodes.get(self.root) {
            Some(ConversionNode::Model {
                model_type,
                validated_attribute,
                ..
            })
            | Some(ConversionNode::Root {
                model_type,
                validated_attribute,
                ..
            }) => (model_type, validated_attribute),
            Some(_) => {
                return Err(PyErr::new::<PyTypeError, _>(
                    "model converter root must be a generated model",
                ));
            }
            None => {
                return Err(PyErr::new::<PyIndexError, _>(
                    "model converter root node is out of bounds",
                ));
            }
        };
        let validated = PyBool::new(py, validated).to_owned().into_any().unbind();
        validated_attribute.set(py, instance.bind(py), model_type, &validated)?;
        Ok(instance)
    }

    pub(crate) fn serialize_to_json_string(
        &self,
        py: Python<'_>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<String> {
        let mut output = Vec::with_capacity(256);
        self.write_json_node(py, self.root, value, MAX_MODEL_DEPTH, &mut output)?;
        String::from_utf8(output).map_err(|error| {
            PyErr::new::<PyValueError, _>(format!(
                "JSON serialization produced invalid UTF-8: {error}"
            ))
        })
    }

    pub(crate) fn to_python_value(
        &self,
        py: Python<'_>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<Py<PyAny>> {
        self.to_python_value_node(py, self.root, value, MAX_MODEL_DEPTH)
    }

    fn convert(
        &self,
        py: Python<'_>,
        node_id: usize,
        value: &Bound<'_, PyAny>,
        state: &mut PythonConversionState,
        remaining_depth: u16,
    ) -> ConversionResult<Py<PyAny>> {
        if remaining_depth == 0 {
            return Err(ConversionFailure::Mismatch(PyErr::new::<PyValueError, _>(
                "generated model conversion exceeds the maximum nesting depth",
            )));
        }
        let node = self.nodes.get(node_id).ok_or_else(|| {
            PyErr::new::<PyIndexError, _>(format!("model converter node {node_id} is missing"))
        })?;
        match node {
            ConversionNode::Scalar {
                kind,
                missing_sentinel,
            } => {
                if matches!(kind, ScalarKind::Any) {
                    self.freeze_python_json_value_for_conversion(
                        py,
                        value,
                        remaining_depth,
                        state.mapping_policy,
                    )
                } else {
                    convert_scalar(py, *kind, missing_sentinel.as_ref(), value)
                }
            }
            ConversionNode::List { item } => {
                self.convert_list(py, *item, value, state, remaining_depth)
            }
            ConversionNode::Dict {
                key,
                value: value_node,
            } => self.convert_dict(py, *key, *value_node, value, state, remaining_depth),
            ConversionNode::Literal { values, .. } => convert_literal(py, values, value),
            ConversionNode::Union {
                branches,
                discriminator,
            } => self.convert_union(
                py,
                branches,
                discriminator.as_ref(),
                value,
                state,
                remaining_depth,
            ),
            ConversionNode::Model {
                model_type,
                fields,
                fields_by_json_name,
                extra_value,
                extra_attribute,
                ..
            } => self.convert_model(
                py,
                model_type,
                fields,
                fields_by_json_name,
                *extra_value,
                extra_attribute.as_ref(),
                value,
                state,
                remaining_depth,
            ),
            ConversionNode::Root {
                model_type,
                value: value_node,
                root_attribute,
                ..
            } => {
                let converted = self.convert(py, *value_node, value, state, remaining_depth - 1)?;
                let instance = allocate_model(py, model_type, &self.object_new)?;
                root_attribute.set(py, &instance, model_type, &converted)?;
                Ok(instance.unbind())
            }
        }
    }

    fn convert_direct(
        &self,
        py: Python<'_>,
        node_id: usize,
        value: &Bound<'_, PyAny>,
        remaining_depth: u16,
        json_proven: &mut bool,
    ) -> PyResult<Py<PyAny>> {
        if remaining_depth == 0 {
            return Err(PyErr::new::<PyValueError, _>(
                "generated model conversion exceeds the maximum nesting depth",
            ));
        }
        let node = self.nodes.get(node_id).ok_or_else(|| {
            PyErr::new::<PyIndexError, _>(format!("model converter node {node_id} is missing"))
        })?;
        match node {
            ConversionNode::Scalar {
                kind,
                missing_sentinel,
            } => {
                if matches!(kind, ScalarKind::Any) {
                    self.freeze_python_json_value(py, value, remaining_depth)
                } else {
                    convert_direct_scalar(py, *kind, missing_sentinel.as_ref(), value)
                }
            }
            ConversionNode::List { item } => {
                if value.is_instance(self.frozen_dict_type.bind(py))? {
                    return Err(expected_type("sequence", value)?);
                }
                let mut output = Vec::new();
                if let Ok(input) = value.cast::<PyList>() {
                    for item_value in input {
                        output.push(self.convert_direct(
                            py,
                            *item,
                            &item_value,
                            remaining_depth - 1,
                            json_proven,
                        )?);
                    }
                } else if let Ok(input) = value.cast::<PyTuple>() {
                    for item_value in input {
                        output.push(self.convert_direct(
                            py,
                            *item,
                            &item_value,
                            remaining_depth - 1,
                            json_proven,
                        )?);
                    }
                } else {
                    if value.is_instance_of::<PyString>()
                        || value.is_instance_of::<PyBytes>()
                        || value.cast::<PyMapping>().is_ok()
                    {
                        return Err(expected_type("sequence", value)?);
                    }
                    let input = value
                        .cast::<PySequence>()
                        .map_err(|_| expected_type("sequence", value).unwrap())?;
                    for item_value in input.try_iter()? {
                        output.push(self.convert_direct(
                            py,
                            *item,
                            &item_value?,
                            remaining_depth - 1,
                            json_proven,
                        )?);
                    }
                }
                self.freeze_list(py, output)
            }
            ConversionNode::Dict {
                key,
                value: value_node,
            } => {
                if let Ok(input) = value.cast::<PyDict>() {
                    let mut output = PythonFrozenDictBuilder::with_capacity(input.len());
                    for (key_value, item_value) in input {
                        let converted_key = self.convert_direct(
                            py,
                            *key,
                            &key_value,
                            remaining_depth - 1,
                            json_proven,
                        )?;
                        let converted_value = self.convert_direct(
                            py,
                            *value_node,
                            &item_value,
                            remaining_depth - 1,
                            json_proven,
                        )?;
                        output.push(&key_value, converted_key, converted_value);
                    }
                    return output.finish(py, self);
                }

                let input = value
                    .cast::<PyMapping>()
                    .map_err(|_| expected_type("mapping", value).unwrap())?;
                let output = NormalizingFrozenDictBuilder::new(py);
                for entry in input.items()? {
                    let (key_value, item_value) = mapping_pair(&entry)?;
                    let converted_key = self.convert_direct(
                        py,
                        *key,
                        &key_value,
                        remaining_depth - 1,
                        json_proven,
                    )?;
                    let converted_value = self.convert_direct(
                        py,
                        *value_node,
                        &item_value,
                        remaining_depth - 1,
                        json_proven,
                    )?;
                    output.insert(py, converted_key, converted_value)?;
                }
                output.finish(py, self)
            }
            ConversionNode::Literal { values, .. } => {
                convert_literal(py, values, value).map_err(ConversionFailure::into_pyerr)
            }
            ConversionNode::Union { branches, .. } => {
                let mut first_error = None;
                for branch in branches {
                    match self.convert_direct(py, *branch, value, remaining_depth - 1, json_proven)
                    {
                        Ok(converted) => return Ok(converted),
                        Err(error) if first_error.is_none() => first_error = Some(error),
                        Err(_) => {}
                    }
                }
                Err(first_error.unwrap_or_else(|| {
                    PyErr::new::<PyTypeError, _>(
                        "value does not match any generated model union branch",
                    )
                }))
            }
            ConversionNode::Model {
                model_type,
                validated_attribute,
                ..
            }
            | ConversionNode::Root {
                model_type,
                validated_attribute,
                ..
            } => {
                if value.is_instance(model_type.bind(py))? {
                    if !validated_attribute
                        .get(value, model_type)
                        .and_then(|validated| validated.extract::<bool>())
                        .unwrap_or(false)
                    {
                        *json_proven = false;
                    }
                    Ok(value.clone().unbind())
                } else {
                    let expected = model_type.bind(py).name()?.to_str()?.to_owned();
                    Err(expected_type(&expected, value)?)
                }
            }
        }
    }

    fn convert_missing_field_value(
        &self,
        py: Python<'_>,
        field: &FieldPlan,
    ) -> PyResult<Py<PyAny>> {
        if let Some(sentinel) = &field.missing_sentinel {
            return Ok(sentinel.clone_ref(py));
        }
        Err(PyErr::new::<PyTypeError, _>(format!(
            "missing required field {}",
            field.attribute.name.bind(py).to_str()?,
        )))
    }

    fn freeze_list(&self, py: Python<'_>, items: Vec<Py<PyAny>>) -> PyResult<Py<PyAny>> {
        #[cfg(all(Py_3_11, not(any(PyPy, GraalPy, Py_GIL_DISABLED))))]
        {
            let item_count = ffi::Py_ssize_t::try_from(items.len()).map_err(|_| {
                PyErr::new::<PyValueError, _>("generated immutable sequence is too large")
            })?;
            let list_type = self
                .frozen_list_type
                .bind(py)
                .as_ptr()
                .cast::<ffi::PyTypeObject>();
            // SAFETY: plan compilation verifies this is a tuple subtype. This
            // is CPython's tuple-subtype construction sequence: allocate the
            // final variable-sized object, transfer each owned item reference,
            // then ensure the fully initialized tuple is GC-tracked.
            if let Some(allocate) = unsafe { (*list_type).tp_alloc } {
                let instance = unsafe { allocate(list_type, item_count) };
                if instance.is_null() {
                    return Err(PyErr::fetch(py));
                }
                for (index, item) in items.into_iter().enumerate() {
                    let index = ffi::Py_ssize_t::try_from(index)
                        .expect("item index fits the previously converted tuple length");
                    unsafe { ffi::PyTuple_SET_ITEM(instance, index, item.into_ptr()) };
                }
                if unsafe { ffi::PyObject_GC_IsTracked(instance) } == 0 {
                    unsafe { ffi::PyObject_GC_Track(instance.cast()) };
                }
                return Ok(unsafe { Bound::from_owned_ptr(py, instance) }.unbind());
            }
        }

        let items = PyList::new(py, items)?;
        Ok(self.frozen_list_type.bind(py).call1((items,))?.unbind())
    }

    #[inline]
    fn freeze_normalized_dict(
        &self,
        py: Python<'_>,
        items: &Bound<'_, PyDict>,
    ) -> PyResult<Py<PyAny>> {
        let mut output = KnownUniqueFrozenDictBuilder::with_capacity(items.len());
        for (key, value) in items {
            output.push(key.unbind(), value.unbind());
        }
        self.freeze_known_unique_dict(py, output.finish())
    }

    #[inline]
    fn freeze_known_unique_dict(
        &self,
        py: Python<'_>,
        items: KnownUniqueFrozenDictEntries,
    ) -> PyResult<Py<PyAny>> {
        let mut pairs = Vec::with_capacity(items.0.len());
        for (key, value) in items.0 {
            pairs.push(PyTuple::new(py, [key, value])?.into_any().unbind());
        }
        let frozen_items = PyTuple::new(py, pairs)?.into_any().unbind();
        let instance = allocate_model(py, &self.frozen_dict_type, &self.object_new)?;
        self.frozen_dict_items_attribute.set(
            py,
            &instance,
            &self.frozen_dict_type,
            &frozen_items,
        )?;
        Ok(instance.unbind())
    }

    fn frozen_dict_items<'py>(&self, value: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyTuple>> {
        self.frozen_dict_items_attribute
            .get(value, &self.frozen_dict_type)?
            .cast_into::<PyTuple>()
            .map_err(|_| {
                PyErr::new::<PyTypeError, _>("generated immutable mapping storage must be a tuple")
            })
    }

    fn freeze_python_json_value(
        &self,
        py: Python<'_>,
        value: &Bound<'_, PyAny>,
        remaining_depth: u16,
    ) -> PyResult<Py<PyAny>> {
        self.freeze_python_json_value_for_conversion(
            py,
            value,
            remaining_depth,
            MappingPolicy::Normalize,
        )
        .map_err(ConversionFailure::into_pyerr)
    }

    fn freeze_python_json_value_for_conversion(
        &self,
        py: Python<'_>,
        value: &Bound<'_, PyAny>,
        remaining_depth: u16,
        mapping_policy: MappingPolicy,
    ) -> ConversionResult<Py<PyAny>> {
        self.freeze_python_json_value_inner(
            py,
            value,
            remaining_depth,
            mapping_policy,
            &mut HashSet::new(),
        )
    }

    fn freeze_python_json_value_inner(
        &self,
        py: Python<'_>,
        value: &Bound<'_, PyAny>,
        remaining_depth: u16,
        mapping_policy: MappingPolicy,
        active_containers: &mut HashSet<usize>,
    ) -> ConversionResult<Py<PyAny>> {
        if remaining_depth == 0 {
            return Err(ConversionFailure::Mismatch(PyErr::new::<PyValueError, _>(
                "generated model conversion exceeds the maximum nesting depth",
            )));
        }
        if let Ok(items) = value.cast::<PyList>() {
            let identity = value.as_ptr() as usize;
            if !active_containers.insert(identity) {
                return Err(ConversionFailure::Mismatch(PyErr::new::<PyValueError, _>(
                    "cyclic containers are not JSON values",
                )));
            }
            let result = items
                .iter()
                .map(|item| {
                    self.freeze_python_json_value_inner(
                        py,
                        &item,
                        remaining_depth - 1,
                        mapping_policy,
                        active_containers,
                    )
                })
                .collect::<ConversionResult<Vec<_>>>();
            active_containers.remove(&identity);
            return Ok(self.freeze_list(py, result?)?);
        }
        if let Ok(properties) = value.cast::<PyDict>() {
            let identity = value.as_ptr() as usize;
            if !active_containers.insert(identity) {
                return Err(ConversionFailure::Mismatch(PyErr::new::<PyValueError, _>(
                    "cyclic containers are not JSON values",
                )));
            }
            let mut output = PythonFrozenDictBuilder::with_capacity(properties.len());
            let result: ConversionResult<()> =
                properties.iter().try_for_each(|(source_key, item)| {
                    let key = source_key.extract::<String>().map_err(|_| {
                        ConversionFailure::Mismatch(PyErr::new::<PyTypeError, _>(
                            "JSON object keys must be strings",
                        ))
                    })?;
                    let value = self.freeze_python_json_value_inner(
                        py,
                        &item,
                        remaining_depth - 1,
                        mapping_policy,
                        active_containers,
                    )?;
                    output.push(
                        &source_key,
                        PyString::new(py, &key).into_any().unbind(),
                        value,
                    );
                    Ok(())
                });
            active_containers.remove(&identity);
            result?;
            return Ok(output.finish(py, self)?);
        }
        if let Ok(properties) = value.cast::<PyMapping>() {
            if mapping_policy == MappingPolicy::RejectWithoutTraversal {
                return Err(ConversionFailure::Mismatch(PyErr::new::<PyTypeError, _>(
                    "general Mapping values are not JSON values",
                )));
            }
            let identity = value.as_ptr() as usize;
            if !active_containers.insert(identity) {
                return Err(ConversionFailure::Mismatch(PyErr::new::<PyValueError, _>(
                    "cyclic containers are not JSON values",
                )));
            }
            let output = NormalizingFrozenDictBuilder::new(py);
            let result = (|| -> ConversionResult<()> {
                for entry in properties.items()? {
                    let (key, item) = mapping_pair(&entry)?;
                    let key = key.extract::<String>().map_err(|_| {
                        ConversionFailure::Mismatch(PyErr::new::<PyTypeError, _>(
                            "JSON object keys must be strings",
                        ))
                    })?;
                    let value = self.freeze_python_json_value_inner(
                        py,
                        &item,
                        remaining_depth - 1,
                        mapping_policy,
                        active_containers,
                    )?;
                    output.insert(py, PyString::new(py, &key).into_any().unbind(), value)?;
                }
                Ok(())
            })();
            active_containers.remove(&identity);
            result?;
            return Ok(output.finish(py, self)?);
        }
        if let Ok(items) = value.cast::<PyTuple>() {
            let identity = value.as_ptr() as usize;
            if !active_containers.insert(identity) {
                return Err(ConversionFailure::Mismatch(PyErr::new::<PyValueError, _>(
                    "cyclic containers are not JSON values",
                )));
            }
            let result = items
                .iter()
                .map(|item| {
                    self.freeze_python_json_value_inner(
                        py,
                        &item,
                        remaining_depth - 1,
                        mapping_policy,
                        active_containers,
                    )
                })
                .collect::<ConversionResult<Vec<_>>>();
            active_containers.remove(&identity);
            return Ok(self.freeze_list(py, result?)?);
        }
        if let Some(value) = canonical_python_scalar(py, value)? {
            return Ok(value);
        }
        Err(ConversionFailure::Mismatch(PyErr::new::<PyTypeError, _>(
            format!(
                "expected a JSON-compatible value, got {}",
                value.get_type().name()?
            ),
        )))
    }

    fn freeze_jiter_json_value(
        &self,
        py: Python<'_>,
        value: &JiterJsonValue<'_>,
        remaining_depth: u16,
    ) -> PyResult<Py<PyAny>> {
        if remaining_depth == 0 {
            return Err(PyErr::new::<PyValueError, _>(
                "generated model conversion exceeds the maximum nesting depth",
            ));
        }
        match value {
            JiterJsonValue::Array(items) => {
                let mut output = Vec::with_capacity(items.len());
                for item in items.iter() {
                    output.push(self.freeze_jiter_json_value(py, item, remaining_depth - 1)?);
                }
                self.freeze_list(py, output)
            }
            JiterJsonValue::Object(entries) => {
                let mut output = JiterFrozenDictBuilder::with_capacity(entries.len());
                for (key, item) in entries.iter() {
                    output.push(
                        key.as_ref(),
                        PyString::new(py, key.as_ref()).into_any().unbind(),
                        self.freeze_jiter_json_value(py, item, remaining_depth - 1)?,
                    )?;
                }
                self.freeze_known_unique_dict(py, output.finish())
            }
            JiterJsonValue::Null
            | JiterJsonValue::Bool(_)
            | JiterJsonValue::Int(_)
            | JiterJsonValue::BigInt(_)
            | JiterJsonValue::Float(_)
            | JiterJsonValue::Str(_) => Ok(value.into_pyobject(py)?.unbind()),
        }
    }

    fn convert_list(
        &self,
        py: Python<'_>,
        item_node: usize,
        value: &Bound<'_, PyAny>,
        state: &mut PythonConversionState,
        remaining_depth: u16,
    ) -> ConversionResult<Py<PyAny>> {
        if value.is_instance(self.frozen_dict_type.bind(py))? {
            return Err(ConversionFailure::Mismatch(expected_type(
                "sequence", value,
            )?));
        }
        let mut output = Vec::new();
        if let Ok(items) = value.cast::<PyList>() {
            for item in items {
                let converted = self.convert(py, item_node, &item, state, remaining_depth - 1)?;
                output.push(converted);
            }
        } else if let Ok(items) = value.cast::<PyTuple>() {
            for item in items {
                let converted = self.convert(py, item_node, &item, state, remaining_depth - 1)?;
                output.push(converted);
            }
        } else {
            return Err(ConversionFailure::Mismatch(expected_type("list", value)?));
        }
        self.freeze_list(py, output)
            .map_err(ConversionFailure::Raised)
    }

    #[allow(clippy::too_many_arguments)]
    fn convert_dict(
        &self,
        py: Python<'_>,
        key_node: usize,
        value_node: usize,
        value: &Bound<'_, PyAny>,
        state: &mut PythonConversionState,
        remaining_depth: u16,
    ) -> ConversionResult<Py<PyAny>> {
        if let Ok(input) = value.cast::<PyDict>() {
            let mut output = PythonFrozenDictBuilder::with_capacity(input.len());
            for (key, item) in input {
                let converted_key = self.convert(py, key_node, &key, state, remaining_depth - 1)?;
                let converted_value =
                    self.convert(py, value_node, &item, state, remaining_depth - 1)?;
                output.push(&key, converted_key, converted_value);
            }
            return output.finish(py, self).map_err(ConversionFailure::Raised);
        }

        // General Mapping implementations can yield duplicate or stateful
        // entries. Preserve their normalization semantics through a temporary
        // dict; exact dicts above construct final immutable storage directly.
        let Ok(input) = value.cast::<PyMapping>() else {
            return Err(ConversionFailure::Mismatch(expected_type(
                "mapping", value,
            )?));
        };
        if state.mapping_policy == MappingPolicy::RejectWithoutTraversal {
            return Err(ConversionFailure::Mismatch(PyErr::new::<PyTypeError, _>(
                "general Mapping values are not JSON values",
            )));
        }
        let output = NormalizingFrozenDictBuilder::new(py);
        for entry in input.items()? {
            let (key, item) = mapping_pair(&entry)?;
            let converted_key = self.convert(py, key_node, &key, state, remaining_depth - 1)?;
            let converted_value =
                self.convert(py, value_node, &item, state, remaining_depth - 1)?;
            output.insert(py, converted_key, converted_value)?;
        }
        output.finish(py, self).map_err(ConversionFailure::Raised)
    }

    #[allow(clippy::too_many_arguments)]
    fn convert_union(
        &self,
        py: Python<'_>,
        branches: &[usize],
        discriminator: Option<&DiscriminatorPlan>,
        value: &Bound<'_, PyAny>,
        state: &mut PythonConversionState,
        remaining_depth: u16,
    ) -> ConversionResult<Py<PyAny>> {
        if let (Some(plan), Ok(object)) = (discriminator, value.cast::<PyDict>())
            && let Some(tag) = object.get_item(&plan.json_name)?
            && let Some(tag) = python_discriminator_key(&tag)
            && let Some(branch) = plan.branches_by_value.get(&tag)
        {
            return self.convert(py, *branch, value, state, remaining_depth - 1);
        }

        let mut matching_branch = None;
        let mut matching_count = 0;
        for branch in branches {
            if self.node_matches_kind(py, *branch, value)? {
                matching_branch = Some(*branch);
                matching_count += 1;
            }
        }
        if matching_count == 1 {
            return self.convert(
                py,
                matching_branch.expect("one matching branch must be present"),
                value,
                state,
                remaining_depth - 1,
            );
        }
        if matching_count > 1 {
            state.ambiguous_union = true;
        }

        let mut first_error = None;
        let mut schema_rejected = Vec::new();
        let mut schema_accepted = false;
        for branch in branches {
            if matching_count > 0 && !self.node_matches_kind(py, *branch, value)? {
                continue;
            }
            if matching_count > 1
                && !self.node_can_represent_python_value(
                    py,
                    *branch,
                    value,
                    false,
                    remaining_depth - 1,
                )?
            {
                continue;
            }
            if matching_count > 1
                && state.union_selection == UnionSelection::ValidateAmbiguousBranches
                && !self.node_can_represent_python_value(
                    py,
                    *branch,
                    value,
                    true,
                    remaining_depth - 1,
                )?
            {
                schema_rejected.push(*branch);
                continue;
            }
            if matching_count > 1
                && state.union_selection == UnionSelection::ValidateAmbiguousBranches
            {
                schema_accepted = true;
            }
            match self.convert(py, *branch, value, state, remaining_depth - 1) {
                Ok(converted) => return Ok(converted),
                Err(ConversionFailure::Mismatch(error)) if first_error.is_none() => {
                    first_error = Some(error)
                }
                Err(ConversionFailure::Mismatch(_)) => {}
                Err(ConversionFailure::Raised(error)) => {
                    return Err(ConversionFailure::Raised(error));
                }
            }
        }
        if !schema_accepted {
            for branch in schema_rejected {
                match self.convert(py, branch, value, state, remaining_depth - 1) {
                    Ok(converted) => return Ok(converted),
                    Err(ConversionFailure::Mismatch(error)) if first_error.is_none() => {
                        first_error = Some(error)
                    }
                    Err(ConversionFailure::Mismatch(_)) => {}
                    Err(ConversionFailure::Raised(error)) => {
                        return Err(ConversionFailure::Raised(error));
                    }
                }
            }
        }
        Err(ConversionFailure::Mismatch(first_error.unwrap_or_else(
            || {
                PyErr::new::<PyTypeError, _>(
                    "value does not match any generated model union branch",
                )
            },
        )))
    }

    fn node_matches_kind(
        &self,
        py: Python<'_>,
        node_id: usize,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<bool> {
        let node = self.nodes.get(node_id).ok_or_else(|| {
            PyErr::new::<PyIndexError, _>(format!("model converter node {node_id} is missing"))
        })?;
        Ok(match node {
            ConversionNode::Scalar { kind, .. } => match kind {
                ScalarKind::Any => true,
                ScalarKind::Missing => false,
                ScalarKind::String => value.is_instance_of::<PyString>(),
                ScalarKind::Integer => {
                    (value.is_instance_of::<PyInt>() && !value.is_instance_of::<PyBool>())
                        || (value.is_instance_of::<PyFloat>()
                            && value.extract::<f64>()?.fract() == 0.0)
                }
                ScalarKind::Number => {
                    !value.is_instance_of::<PyBool>()
                        && (value.is_instance_of::<PyInt>() || value.is_instance_of::<PyFloat>())
                }
                ScalarKind::Boolean => value.is_instance_of::<PyBool>(),
                ScalarKind::Null => value.is_none(),
            },
            ConversionNode::List { .. } => value.is_instance_of::<PyList>(),
            ConversionNode::Dict { .. } | ConversionNode::Model { .. } => {
                value.is_instance_of::<PyDict>()
            }
            ConversionNode::Literal { values, .. } => literal_index(py, values, value)?.is_some(),
            ConversionNode::Union { branches, .. } => {
                let mut matches = false;
                for branch in branches {
                    if self.node_matches_kind(py, *branch, value)? {
                        matches = true;
                        break;
                    }
                }
                matches
            }
            ConversionNode::Root { value: child, .. } => {
                self.node_matches_kind(py, *child, value)?
            }
        })
    }

    fn node_can_represent_python_value(
        &self,
        py: Python<'_>,
        node_id: usize,
        value: &Bound<'_, PyAny>,
        validate_model_schema: bool,
        remaining_depth: u16,
    ) -> PyResult<bool> {
        if remaining_depth == 0 {
            return Ok(false);
        }
        let Some(node) = self.nodes.get(node_id) else {
            return Ok(false);
        };
        match node {
            ConversionNode::Scalar { .. } | ConversionNode::Literal { .. } => {
                self.node_matches_kind(py, node_id, value)
            }
            ConversionNode::List { item } => {
                let Ok(values) = value.cast::<PyList>() else {
                    return Ok(false);
                };
                for item_value in values {
                    if !self.node_can_represent_python_value(
                        py,
                        *item,
                        &item_value,
                        validate_model_schema,
                        remaining_depth - 1,
                    )? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            ConversionNode::Dict {
                key,
                value: value_node,
            } => {
                let Ok(values) = value.cast::<PyDict>() else {
                    return Ok(false);
                };
                for (key_value, item_value) in values {
                    if !self.node_can_represent_python_value(
                        py,
                        *key,
                        &key_value,
                        validate_model_schema,
                        remaining_depth - 1,
                    )? || !self.node_can_represent_python_value(
                        py,
                        *value_node,
                        &item_value,
                        validate_model_schema,
                        remaining_depth - 1,
                    )? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            ConversionNode::Union { branches, .. } => {
                for branch in branches {
                    if self.node_can_represent_python_value(
                        py,
                        *branch,
                        value,
                        validate_model_schema,
                        remaining_depth - 1,
                    )? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            ConversionNode::Model {
                branch_schema,
                fields,
                fields_by_json_name,
                extra_value,
                ..
            } => {
                let Ok(values) = value.cast::<PyDict>() else {
                    return Ok(false);
                };
                if validate_model_schema
                    && !branch_schema.is_valid_instance(JsonInstanceRef::from_python(value))?
                {
                    return Ok(false);
                }
                for (key, item_value) in values {
                    let Ok(key) = key.extract::<String>() else {
                        return Ok(false);
                    };
                    let child = fields_by_json_name
                        .get(&key)
                        .map(|index| fields[*index].value_node)
                        .or(*extra_value);
                    let Some(child) = child else {
                        return Ok(false);
                    };
                    if !self.node_can_represent_python_value(
                        py,
                        child,
                        &item_value,
                        validate_model_schema,
                        remaining_depth - 1,
                    )? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            ConversionNode::Root {
                branch_schema,
                value: child,
                ..
            } => {
                if validate_model_schema
                    && !branch_schema.is_valid_instance(JsonInstanceRef::from_python(value))?
                {
                    return Ok(false);
                }
                self.node_can_represent_python_value(
                    py,
                    *child,
                    value,
                    validate_model_schema,
                    remaining_depth - 1,
                )
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn convert_model(
        &self,
        py: Python<'_>,
        model_type: &Py<PyType>,
        fields: &[FieldPlan],
        fields_by_json_name: &HashMap<String, usize>,
        extra_value: Option<usize>,
        extra_attribute: Option<&ModelAttribute>,
        value: &Bound<'_, PyAny>,
        state: &mut PythonConversionState,
        remaining_depth: u16,
    ) -> ConversionResult<Py<PyAny>> {
        let Ok(input) = value.cast::<PyDict>() else {
            return Err(ConversionFailure::Mismatch(expected_type(
                "JSON object",
                value,
            )?));
        };
        let instance = allocate_model(py, model_type, &self.object_new)?;
        let mut extra_output =
            extra_value.map(|_| PythonFrozenDictBuilder::with_capacity(input.len()));
        let mut present_fields = 0;
        let mut normalization_required = false;

        for (key, item) in input {
            let key = key.cast::<PyString>().map_err(|_| {
                ConversionFailure::Mismatch(PyErr::new::<PyTypeError, _>(
                    "JSON object keys must be strings",
                ))
            })?;
            normalization_required |= !key.is_exact_instance_of::<PyString>();
            let key_string = key.to_str()?;
            if let Some(field_index) = fields_by_json_name.get(key_string) {
                let field = &fields[*field_index];
                let already_present =
                    normalization_required && field.attribute.is_set(py, &instance, model_type)?;
                let converted =
                    self.convert(py, field.value_node, &item, state, remaining_depth - 1)?;
                field.attribute.set(py, &instance, model_type, &converted)?;
                if !already_present {
                    present_fields += 1;
                }
            } else if let (Some(extra_node), Some(output)) = (extra_value, extra_output.as_mut()) {
                let converted = self.convert(py, extra_node, &item, state, remaining_depth - 1)?;
                output.push(
                    key.as_any(),
                    PyString::new(py, key_string).into_any().unbind(),
                    converted,
                );
            } else {
                return Err(ConversionFailure::Mismatch(PyErr::new::<PyTypeError, _>(
                    format!("generated model cannot represent property {key_string:?}"),
                )));
            }
        }

        if present_fields != fields.len() {
            for field in fields {
                if !field.attribute.is_set(py, &instance, model_type)? {
                    let converted = self
                        .convert_missing_field_value(py, field)
                        .map_err(ConversionFailure::Mismatch)?;
                    field.attribute.set(py, &instance, model_type, &converted)?;
                }
            }
        }

        let extra = extra_output
            .map(|output| output.finish(py, self))
            .transpose()
            .map_err(ConversionFailure::Raised)?;

        if let (Some(attribute), Some(extra)) = (extra_attribute, extra.as_ref()) {
            attribute.set(py, &instance, model_type, extra)?;
        }
        Ok(instance.unbind())
    }

    #[allow(clippy::too_many_arguments)]
    fn convert_root_kwargs(
        &self,
        py: Python<'_>,
        model_type: &Py<PyType>,
        value_node: usize,
        root_attribute: &ModelAttribute,
        kwargs: &Bound<'_, PyDict>,
        json_proven: &mut bool,
    ) -> PyResult<Py<PyAny>> {
        let model_name = model_type.bind(py).name()?.to_str()?.to_owned();
        if kwargs.len() != 1 {
            if kwargs.get_item("root")?.is_none() {
                return Err(PyErr::new::<PyTypeError, _>(format!(
                    "{model_name} is missing required field root"
                )));
            }
            let unexpected = kwargs
                .keys()
                .iter()
                .find_map(|key| {
                    let key = key.extract::<String>().ok()?;
                    (key != "root").then_some(key)
                })
                .unwrap_or_else(|| "<unknown>".to_owned());
            return Err(unexpected_keyword(py, model_type, &unexpected));
        }
        let raw = kwargs.get_item("root")?.ok_or_else(|| {
            PyErr::new::<PyTypeError, _>(format!("{model_name} is missing required field root"))
        })?;
        let converted =
            self.convert_direct(py, value_node, &raw, MAX_MODEL_DEPTH - 1, json_proven)?;
        let instance = allocate_model(py, model_type, &self.object_new)?;
        root_attribute.set(py, &instance, model_type, &converted)?;
        Ok(instance.unbind())
    }

    #[allow(clippy::too_many_arguments)]
    fn convert_model_kwargs(
        &self,
        py: Python<'_>,
        model_type: &Py<PyType>,
        fields: &[FieldPlan],
        fields_by_json_name: &HashMap<String, usize>,
        fields_by_py_name: &HashMap<String, usize>,
        extra_value: Option<usize>,
        extra_attribute: Option<&ModelAttribute>,
        kwargs: &Bound<'_, PyDict>,
        json_proven: &mut bool,
    ) -> PyResult<Py<PyAny>> {
        let instance = allocate_model(py, model_type, &self.object_new)?;
        let mut extra_input = None;
        let mut present_fields = 0;

        for (key, value) in kwargs {
            let key = key.extract::<String>().map_err(|_| {
                PyErr::new::<PyTypeError, _>("generated model keyword names must be strings")
            })?;
            if key == "__jsoncompat_extra__" {
                if extra_value.is_none() {
                    return Err(unexpected_keyword(py, model_type, &key));
                }
                extra_input = Some(value);
                continue;
            }
            let Some(field_index) = fields_by_py_name.get(&key) else {
                return Err(unexpected_keyword(py, model_type, &key));
            };
            let field = &fields[*field_index];
            let converted = self.convert_direct(
                py,
                field.value_node,
                &value,
                MAX_MODEL_DEPTH - 1,
                json_proven,
            )?;
            field.attribute.set(py, &instance, model_type, &converted)?;
            present_fields += 1;
        }

        if present_fields != fields.len() {
            for field in fields {
                if !field.attribute.is_set(py, &instance, model_type)? {
                    let converted = self.convert_missing_field_value(py, field)?;
                    field.attribute.set(py, &instance, model_type, &converted)?;
                }
            }
        }

        if let (Some(extra_node), Some(extra_attribute)) = (extra_value, extra_attribute) {
            let extra = if let Some(extra_input) = extra_input {
                if let Ok(extra_input) = extra_input.cast::<PyDict>() {
                    let mut output = PythonFrozenDictBuilder::with_capacity(extra_input.len());
                    for (key, value) in extra_input {
                        let key_string = key.extract::<String>().map_err(|_| {
                            PyErr::new::<PyTypeError, _>("JSON object keys must be strings")
                        })?;
                        if fields_by_json_name.contains_key(&key_string) {
                            return Err(PyErr::new::<PyValueError, _>(format!(
                                "additional property {key_string:?} collides with a declared field"
                            )));
                        }
                        let converted = self.convert_direct(
                            py,
                            extra_node,
                            &value,
                            MAX_MODEL_DEPTH - 1,
                            json_proven,
                        )?;
                        output.push(
                            &key,
                            PyString::new(py, &key_string).into_any().unbind(),
                            converted,
                        );
                    }
                    output.finish(py, self)?
                } else {
                    let extra_input = extra_input.cast::<PyMapping>().map_err(|_| {
                        PyErr::new::<PyTypeError, _>(
                            "generated additional properties must be a mapping",
                        )
                    })?;
                    let output = NormalizingFrozenDictBuilder::new(py);
                    for entry in extra_input.items()? {
                        let (key, value) = mapping_pair(&entry)?;
                        let key_string = key.extract::<String>().map_err(|_| {
                            PyErr::new::<PyTypeError, _>("JSON object keys must be strings")
                        })?;
                        if fields_by_json_name.contains_key(&key_string) {
                            return Err(PyErr::new::<PyValueError, _>(format!(
                                "additional property {key_string:?} collides with a declared field"
                            )));
                        }
                        let converted = self.convert_direct(
                            py,
                            extra_node,
                            &value,
                            MAX_MODEL_DEPTH - 1,
                            json_proven,
                        )?;
                        output.insert(
                            py,
                            PyString::new(py, &key_string).into_any().unbind(),
                            converted,
                        )?;
                    }
                    output.finish(py, self)?
                }
            } else {
                self.freeze_known_unique_dict(
                    py,
                    KnownUniqueFrozenDictBuilder::with_capacity(0).finish(),
                )?
            };
            extra_attribute.set(py, &instance, model_type, &extra)?;
        }

        Ok(instance.unbind())
    }

    fn convert_jiter(
        &self,
        py: Python<'_>,
        node_id: usize,
        value: &JiterJsonValue<'_>,
        state: &mut JiterConversionState,
    ) -> PyResult<Py<PyAny>> {
        let node = self.nodes.get(node_id).ok_or_else(|| {
            PyErr::new::<PyIndexError, _>(format!("model converter node {node_id} is missing"))
        })?;
        match node {
            ConversionNode::Scalar { kind, .. } => {
                if matches!(kind, ScalarKind::Any) {
                    self.freeze_jiter_json_value(py, value, MAX_MODEL_DEPTH)
                } else {
                    convert_jiter_scalar_value(py, *kind, value)
                }
            }
            ConversionNode::Literal {
                values,
                string_indices,
            } => {
                if let JiterJsonValue::Str(value) = value
                    && let Some(index) = string_indices.get(value.as_ref())
                {
                    return Ok(values[*index].clone_ref(py));
                }
                let python_value = value.into_pyobject(py)?.unbind();
                convert_literal(py, values, python_value.bind(py))
                    .map_err(ConversionFailure::into_pyerr)
            }
            ConversionNode::List { item } => {
                let JiterJsonValue::Array(items) = value else {
                    return Err(PyErr::new::<PyTypeError, _>("expected list"));
                };
                let mut converted = Vec::with_capacity(items.len());
                for item_value in items.iter() {
                    converted.push(self.convert_jiter(py, *item, item_value, state)?);
                }
                self.freeze_list(py, converted)
            }
            ConversionNode::Dict {
                key,
                value: value_node,
            } => {
                let JiterJsonValue::Object(entries) = value else {
                    return Err(PyErr::new::<PyTypeError, _>("expected dict"));
                };
                let mut output = JiterFrozenDictBuilder::with_capacity(entries.len());
                for (key_value, item) in entries.iter() {
                    let jiter_key = JiterJsonValue::Str(key_value.clone());
                    let converted_key = self.convert_jiter(py, *key, &jiter_key, state)?;
                    let converted_value = self.convert_jiter(py, *value_node, item, state)?;
                    output.push(key_value.as_ref(), converted_key, converted_value)?;
                }
                self.freeze_known_unique_dict(py, output.finish())
            }
            ConversionNode::Union {
                branches,
                discriminator,
            } => self.convert_jiter_union_value(py, branches, discriminator.as_ref(), value, state),
            ConversionNode::Model {
                model_type,
                fields,
                fields_by_json_name,
                extra_value,
                extra_attribute,
                ..
            } => self.convert_jiter_model_value(
                py,
                model_type,
                fields,
                fields_by_json_name,
                *extra_value,
                extra_attribute.as_ref(),
                value,
                state,
            ),
            ConversionNode::Root {
                model_type,
                value: value_node,
                root_attribute,
                ..
            } => {
                let converted = self.convert_jiter(py, *value_node, value, state)?;
                let instance = allocate_model(py, model_type, &self.object_new)?;
                root_attribute.set(py, &instance, model_type, &converted)?;
                Ok(instance.unbind())
            }
        }
    }

    fn convert_jiter_union_value(
        &self,
        py: Python<'_>,
        branches: &[usize],
        discriminator: Option<&DiscriminatorPlan>,
        value: &JiterJsonValue<'_>,
        state: &mut JiterConversionState,
    ) -> PyResult<Py<PyAny>> {
        if let (Some(plan), JiterJsonValue::Object(entries)) = (discriminator, value)
            && let Some((_, tag)) = entries
                .iter()
                .find(|(key, _)| key.as_ref() == plan.json_name)
            && let Some(tag) = jiter_discriminator_key(tag)
            && let Some(branch) = plan.branches_by_value.get(&tag)
        {
            return self.convert_jiter(py, *branch, value, state);
        }

        let mut matching_count = 0;
        let mut sole_match = None;
        for branch in branches {
            if self.jiter_node_matches_kind(*branch, value) {
                matching_count += 1;
                sole_match = Some(*branch);
            }
        }
        if matching_count == 1 {
            return self.convert_jiter(
                py,
                sole_match.expect("one matching branch records its node"),
                value,
                state,
            );
        }
        let mut first_error = None;
        let mut schema_rejected = Vec::new();
        let mut schema_accepted = false;
        for branch in branches {
            if matching_count != 0 && !self.jiter_node_matches_kind(*branch, value) {
                continue;
            }
            if matching_count > 1
                && !self.jiter_node_can_represent_value(*branch, value, false, MAX_MODEL_DEPTH)?
            {
                continue;
            }
            if matching_count > 1
                && state.union_selection == UnionSelection::ValidateAmbiguousBranches
                && !self.jiter_node_can_represent_value(*branch, value, true, MAX_MODEL_DEPTH)?
            {
                schema_rejected.push(*branch);
                continue;
            }
            if matching_count > 1
                && state.union_selection == UnionSelection::ValidateAmbiguousBranches
            {
                schema_accepted = true;
            }
            match self.convert_jiter(py, *branch, value, state) {
                Ok(converted) => return Ok(converted),
                Err(error) if first_error.is_none() => first_error = Some(error),
                Err(_) => {}
            }
        }
        if !schema_accepted {
            for branch in schema_rejected {
                match self.convert_jiter(py, branch, value, state) {
                    Ok(converted) => return Ok(converted),
                    Err(error) if first_error.is_none() => first_error = Some(error),
                    Err(_) => {}
                }
            }
        }
        Err(first_error.unwrap_or_else(|| {
            PyErr::new::<PyTypeError, _>("value does not match any generated model union branch")
        }))
    }

    #[allow(clippy::too_many_arguments)]
    fn convert_jiter_model_value(
        &self,
        py: Python<'_>,
        model_type: &Py<PyType>,
        fields: &[FieldPlan],
        fields_by_json_name: &HashMap<String, usize>,
        extra_value: Option<usize>,
        extra_attribute: Option<&ModelAttribute>,
        value: &JiterJsonValue<'_>,
        state: &mut JiterConversionState,
    ) -> PyResult<Py<PyAny>> {
        let JiterJsonValue::Object(entries) = value else {
            return Err(PyErr::new::<PyTypeError, _>("expected JSON object"));
        };
        let instance = allocate_model(py, model_type, &self.object_new)?;
        let mut extra_output =
            extra_value.map(|_| JiterFrozenDictBuilder::with_capacity(entries.len()));
        let mut present_fields = 0;

        for (key, item) in entries.iter() {
            let key_string = key.as_ref();
            if let Some(field_index) = fields_by_json_name.get(key_string) {
                let field = &fields[*field_index];
                if field.attribute.is_set(py, &instance, model_type)? {
                    return Err(duplicate_key(key));
                }
                let converted = self.convert_jiter(py, field.value_node, item, state)?;
                field.attribute.set(py, &instance, model_type, &converted)?;
                present_fields += 1;
            } else if let (Some(extra_node), Some(output)) = (extra_value, extra_output.as_mut()) {
                output.push(
                    key_string,
                    PyString::new(py, key_string).into_any().unbind(),
                    self.convert_jiter(py, extra_node, item, state)?,
                )?;
            } else {
                return Err(PyErr::new::<PyTypeError, _>(format!(
                    "generated model cannot represent property {key_string:?}"
                )));
            }
        }

        let extra = extra_output
            .map(|output| self.freeze_known_unique_dict(py, output.finish()))
            .transpose()?;
        if present_fields != fields.len() {
            for field in fields {
                if !field.attribute.is_set(py, &instance, model_type)? {
                    let converted = self.convert_missing_field_value(py, field)?;
                    field.attribute.set(py, &instance, model_type, &converted)?;
                }
            }
        }
        if let (Some(attribute), Some(extra)) = (extra_attribute, extra.as_ref()) {
            attribute.set(py, &instance, model_type, extra)?;
        }
        Ok(instance.unbind())
    }

    fn jiter_node_matches_kind(&self, node_id: usize, value: &JiterJsonValue<'_>) -> bool {
        let Some(node) = self.nodes.get(node_id) else {
            return false;
        };
        match node {
            ConversionNode::Scalar { kind, .. } => match kind {
                ScalarKind::Any => true,
                ScalarKind::Missing => false,
                ScalarKind::String => matches!(value, JiterJsonValue::Str(_)),
                ScalarKind::Integer => match value {
                    JiterJsonValue::Int(_) | JiterJsonValue::BigInt(_) => true,
                    JiterJsonValue::Float(value) => value.fract() == 0.0,
                    _ => false,
                },
                ScalarKind::Number => matches!(
                    value,
                    JiterJsonValue::Int(_) | JiterJsonValue::BigInt(_) | JiterJsonValue::Float(_)
                ),
                ScalarKind::Boolean => matches!(value, JiterJsonValue::Bool(_)),
                ScalarKind::Null => matches!(value, JiterJsonValue::Null),
            },
            ConversionNode::List { .. } => matches!(value, JiterJsonValue::Array(_)),
            ConversionNode::Dict { .. } | ConversionNode::Model { .. } => {
                matches!(value, JiterJsonValue::Object(_))
            }
            ConversionNode::Literal { .. } => true,
            ConversionNode::Union { branches, .. } => branches
                .iter()
                .any(|branch| self.jiter_node_matches_kind(*branch, value)),
            ConversionNode::Root { value: child, .. } => {
                self.jiter_node_matches_kind(*child, value)
            }
        }
    }

    fn jiter_node_can_represent_value(
        &self,
        node_id: usize,
        value: &JiterJsonValue<'_>,
        validate_model_schema: bool,
        remaining_depth: u16,
    ) -> PyResult<bool> {
        if remaining_depth == 0 {
            return Ok(false);
        }
        let Some(node) = self.nodes.get(node_id) else {
            return Ok(false);
        };
        Ok(match node {
            ConversionNode::Scalar { .. } | ConversionNode::Literal { .. } => {
                self.jiter_node_matches_kind(node_id, value)
            }
            ConversionNode::List { item } => {
                let JiterJsonValue::Array(values) = value else {
                    return Ok(false);
                };
                for value in values.iter() {
                    if !self.jiter_node_can_represent_value(
                        *item,
                        value,
                        validate_model_schema,
                        remaining_depth - 1,
                    )? {
                        return Ok(false);
                    }
                }
                true
            }
            ConversionNode::Dict {
                key,
                value: value_node,
            } => {
                let JiterJsonValue::Object(values) = value else {
                    return Ok(false);
                };
                for (key_value, item_value) in values.iter() {
                    if !self.jiter_node_can_represent_value(
                        *key,
                        &JiterJsonValue::Str(key_value.clone()),
                        validate_model_schema,
                        remaining_depth - 1,
                    )? || !self.jiter_node_can_represent_value(
                        *value_node,
                        item_value,
                        validate_model_schema,
                        remaining_depth - 1,
                    )? {
                        return Ok(false);
                    }
                }
                true
            }
            ConversionNode::Union { branches, .. } => {
                let mut represents = false;
                for branch in branches {
                    if self.jiter_node_can_represent_value(
                        *branch,
                        value,
                        validate_model_schema,
                        remaining_depth - 1,
                    )? {
                        represents = true;
                        break;
                    }
                }
                represents
            }
            ConversionNode::Model {
                branch_schema,
                fields,
                fields_by_json_name,
                extra_value,
                ..
            } => {
                let JiterJsonValue::Object(values) = value else {
                    return Ok(false);
                };
                if validate_model_schema
                    && !branch_schema.is_valid_instance(JsonInstanceRef::from_jiter(value))?
                {
                    return Ok(false);
                }
                for (key, item_value) in values.iter() {
                    let child = fields_by_json_name
                        .get(key.as_ref())
                        .map(|index| fields[*index].value_node)
                        .or(*extra_value);
                    let Some(child) = child else {
                        return Ok(false);
                    };
                    if !self.jiter_node_can_represent_value(
                        child,
                        item_value,
                        validate_model_schema,
                        remaining_depth - 1,
                    )? {
                        return Ok(false);
                    }
                }
                true
            }
            ConversionNode::Root {
                branch_schema,
                value: child,
                ..
            } => {
                if validate_model_schema
                    && !branch_schema.is_valid_instance(JsonInstanceRef::from_jiter(value))?
                {
                    return Ok(false);
                }
                self.jiter_node_can_represent_value(
                    *child,
                    value,
                    validate_model_schema,
                    remaining_depth - 1,
                )?
            }
        })
    }

    fn write_json_node<'py>(
        &self,
        py: Python<'py>,
        node_id: usize,
        value: &Bound<'py, PyAny>,
        remaining_depth: u16,
        output: &mut Vec<u8>,
    ) -> PyResult<()> {
        if remaining_depth == 0 {
            return Err(PyErr::new::<PyValueError, _>(
                "generated model serialization exceeds the maximum nesting depth",
            ));
        }
        let node = self.nodes.get(node_id).ok_or_else(|| {
            PyErr::new::<PyIndexError, _>(format!("model converter node {node_id} is missing"))
        })?;
        match node {
            ConversionNode::Scalar { .. } | ConversionNode::Literal { .. } => {
                write_serializable_json_value(output, value)
            }
            ConversionNode::List { item } => {
                let items = value
                    .cast::<PyTuple>()
                    .map_err(|_| expected_type("immutable sequence", value).unwrap())?;
                output.push(b'[');
                for (index, item_value) in items.iter().enumerate() {
                    if index != 0 {
                        output.push(b',');
                    }
                    self.write_json_node(py, *item, &item_value, remaining_depth - 1, output)?;
                }
                output.push(b']');
                Ok(())
            }
            ConversionNode::Dict {
                key: _,
                value: value_node,
            } => {
                let input = self.frozen_dict_items(value)?;
                let mut entries = Vec::with_capacity(input.len());
                for entry in input {
                    let (key, item) = mapping_pair(&entry)?;
                    let key = key.extract::<String>().map_err(|_| {
                        PyErr::new::<PyTypeError, _>("JSON object keys must be strings")
                    })?;
                    entries.push((key, item));
                }
                entries.sort_unstable_by(|left, right| left.0.cmp(&right.0));
                output.push(b'{');
                for (index, (key, item)) in entries.into_iter().enumerate() {
                    if index != 0 {
                        output.push(b',');
                    }
                    write_json_string(output, &key)?;
                    output.push(b':');
                    self.write_json_node(py, *value_node, &item, remaining_depth - 1, output)?;
                }
                output.push(b'}');
                Ok(())
            }
            ConversionNode::Union { branches, .. } => {
                let checkpoint = output.len();
                let mut first_error = None;
                for branch in branches {
                    if self.node_matches_model_value(py, *branch, value)? {
                        match self.write_json_node(py, *branch, value, remaining_depth - 1, output)
                        {
                            Ok(()) => return Ok(()),
                            Err(error) if first_error.is_none() => {
                                output.truncate(checkpoint);
                                first_error = Some(error);
                            }
                            Err(_) => output.truncate(checkpoint),
                        }
                    }
                }
                if let Some(error) = first_error {
                    return Err(error);
                }
                Err(PyErr::new::<PyTypeError, _>(
                    "value does not match any generated model union branch",
                ))
            }
            ConversionNode::Model {
                model_type,
                fields,
                serialized_fields,
                extra_value,
                extra_attribute,
                ..
            } => {
                if !value.is_instance(model_type.bind(py))? {
                    let expected = model_type.bind(py).name()?.to_str()?.to_owned();
                    return Err(expected_type(&expected, value)?);
                }

                let mut field_entries = Vec::with_capacity(fields.len());
                for field_index in serialized_fields {
                    let field = &fields[*field_index];
                    let field_value = field.attribute.get(value, model_type)?;
                    if field
                        .missing_sentinel
                        .as_ref()
                        .is_some_and(|sentinel| field_value.is(sentinel.bind(py)))
                    {
                        continue;
                    }
                    field_entries.push((field, field_value));
                }

                output.push(b'{');
                let mut first = true;
                if let (Some(extra_node), Some(extra_attribute)) = (extra_value, extra_attribute) {
                    let extra = extra_attribute.get(value, model_type)?;
                    let extra = self.frozen_dict_items(&extra)?;
                    let mut extra_entries = Vec::with_capacity(extra.len());
                    for entry in extra {
                        let (key, item) = mapping_pair(&entry)?;
                        let key = key.extract::<String>().map_err(|_| {
                            PyErr::new::<PyTypeError, _>("JSON object keys must be strings")
                        })?;
                        extra_entries.push((key, item));
                    }
                    extra_entries.sort_unstable_by(|left, right| left.0.cmp(&right.0));

                    // Merge the sorted schema fields and additional properties.
                    // Additional properties win collisions, matching the
                    // materializing JsonMap encoder without allocating a map.
                    let mut field_index = 0;
                    let mut extra_index = 0;
                    while field_index < field_entries.len() || extra_index < extra_entries.len() {
                        match (
                            field_entries.get(field_index),
                            extra_entries.get(extra_index),
                        ) {
                            (Some((field, field_value)), Some((extra_key, extra_value))) => {
                                match field.json_name.as_str().cmp(extra_key.as_str()) {
                                    std::cmp::Ordering::Less => {
                                        self.write_json_object_entry(
                                            py,
                                            &field.json_name,
                                            field.value_node,
                                            field_value,
                                            remaining_depth - 1,
                                            output,
                                            &mut first,
                                        )?;
                                        field_index += 1;
                                    }
                                    std::cmp::Ordering::Equal => {
                                        self.write_json_object_entry(
                                            py,
                                            extra_key,
                                            *extra_node,
                                            extra_value,
                                            remaining_depth - 1,
                                            output,
                                            &mut first,
                                        )?;
                                        field_index += 1;
                                        extra_index += 1;
                                    }
                                    std::cmp::Ordering::Greater => {
                                        self.write_json_object_entry(
                                            py,
                                            extra_key,
                                            *extra_node,
                                            extra_value,
                                            remaining_depth - 1,
                                            output,
                                            &mut first,
                                        )?;
                                        extra_index += 1;
                                    }
                                }
                            }
                            (Some((field, field_value)), None) => {
                                self.write_json_object_entry(
                                    py,
                                    &field.json_name,
                                    field.value_node,
                                    field_value,
                                    remaining_depth - 1,
                                    output,
                                    &mut first,
                                )?;
                                field_index += 1;
                            }
                            (None, Some((extra_key, extra_value))) => {
                                self.write_json_object_entry(
                                    py,
                                    extra_key,
                                    *extra_node,
                                    extra_value,
                                    remaining_depth - 1,
                                    output,
                                    &mut first,
                                )?;
                                extra_index += 1;
                            }
                            (None, None) => break,
                        }
                    }
                } else {
                    for (field, field_value) in field_entries {
                        self.write_json_object_entry(
                            py,
                            &field.json_name,
                            field.value_node,
                            &field_value,
                            remaining_depth - 1,
                            output,
                            &mut first,
                        )?;
                    }
                }
                output.push(b'}');
                Ok(())
            }
            ConversionNode::Root {
                model_type,
                value: value_node,
                root_attribute,
                ..
            } => {
                if !value.is_instance(model_type.bind(py))? {
                    let expected = model_type.bind(py).name()?.to_str()?.to_owned();
                    return Err(expected_type(&expected, value)?);
                }
                let root = root_attribute.get(value, model_type)?;
                self.write_json_node(py, *value_node, &root, remaining_depth - 1, output)
            }
        }
    }

    fn to_python_value_node(
        &self,
        py: Python<'_>,
        node_id: usize,
        value: &Bound<'_, PyAny>,
        remaining_depth: u16,
    ) -> PyResult<Py<PyAny>> {
        if remaining_depth == 0 {
            return Err(PyErr::new::<PyValueError, _>(
                "generated model serialization exceeds the maximum nesting depth",
            ));
        }
        let node = self.nodes.get(node_id).ok_or_else(|| {
            PyErr::new::<PyIndexError, _>(format!("model converter node {node_id} is missing"))
        })?;
        match node {
            ConversionNode::Scalar {
                kind: ScalarKind::Any,
                ..
            } => copy_python_json_value(py, value, remaining_depth - 1),
            ConversionNode::Scalar { .. } | ConversionNode::Literal { .. } => {
                Ok(value.clone().unbind())
            }
            ConversionNode::List { item } => {
                let input = value
                    .cast::<PyTuple>()
                    .map_err(|_| expected_type("immutable sequence", value).unwrap())?;
                let output = PyList::empty(py);
                for item_value in input {
                    output.append(self.to_python_value_node(
                        py,
                        *item,
                        &item_value,
                        remaining_depth - 1,
                    )?)?;
                }
                Ok(output.into_any().unbind())
            }
            ConversionNode::Dict {
                key: _,
                value: value_node,
            } => {
                let input = self.frozen_dict_items(value)?;
                let output = PyDict::new(py);
                for entry in input {
                    let (key, item) = mapping_pair(&entry)?;
                    if !key.is_instance_of::<PyString>() {
                        return Err(PyErr::new::<PyTypeError, _>(
                            "JSON object keys must be strings",
                        ));
                    }
                    output.set_item(
                        key,
                        self.to_python_value_node(py, *value_node, &item, remaining_depth - 1)?,
                    )?;
                }
                Ok(output.into_any().unbind())
            }
            ConversionNode::Union { branches, .. } => {
                let mut first_error = None;
                for branch in branches {
                    if self.node_matches_model_value(py, *branch, value)? {
                        match self.to_python_value_node(py, *branch, value, remaining_depth - 1) {
                            Ok(converted) => return Ok(converted),
                            Err(error) if first_error.is_none() => first_error = Some(error),
                            Err(_) => {}
                        }
                    }
                }
                if let Some(error) = first_error {
                    return Err(error);
                }
                Err(PyErr::new::<PyTypeError, _>(
                    "value does not match any generated model union branch",
                ))
            }
            ConversionNode::Model {
                model_type,
                fields,
                extra_value,
                extra_attribute,
                ..
            } => {
                if !value.is_instance(model_type.bind(py))? {
                    let expected = model_type.bind(py).name()?.to_str()?.to_owned();
                    return Err(expected_type(&expected, value)?);
                }
                let output = PyDict::new(py);
                for field in fields {
                    let field_value = field.attribute.get(value, model_type)?;
                    if field
                        .missing_sentinel
                        .as_ref()
                        .is_some_and(|sentinel| field_value.is(sentinel.bind(py)))
                    {
                        continue;
                    }
                    output.set_item(
                        field.json_name.as_str(),
                        self.to_python_value_node(
                            py,
                            field.value_node,
                            &field_value,
                            remaining_depth - 1,
                        )?,
                    )?;
                }
                if let (Some(extra_node), Some(extra_attribute)) = (extra_value, extra_attribute) {
                    let extra = extra_attribute.get(value, model_type)?;
                    let extra = self.frozen_dict_items(&extra)?;
                    for entry in extra {
                        let (key, item) = mapping_pair(&entry)?;
                        if !key.is_instance_of::<PyString>() {
                            return Err(PyErr::new::<PyTypeError, _>(
                                "JSON object keys must be strings",
                            ));
                        }
                        output.set_item(
                            key,
                            self.to_python_value_node(py, *extra_node, &item, remaining_depth - 1)?,
                        )?;
                    }
                }
                Ok(output.into_any().unbind())
            }
            ConversionNode::Root {
                model_type,
                value: value_node,
                root_attribute,
                ..
            } => {
                if !value.is_instance(model_type.bind(py))? {
                    let expected = model_type.bind(py).name()?.to_str()?.to_owned();
                    return Err(expected_type(&expected, value)?);
                }
                let root = root_attribute.get(value, model_type)?;
                self.to_python_value_node(py, *value_node, &root, remaining_depth - 1)
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn write_json_object_entry<'py>(
        &self,
        py: Python<'py>,
        key: &str,
        value_node: usize,
        value: &Bound<'py, PyAny>,
        remaining_depth: u16,
        output: &mut Vec<u8>,
        first: &mut bool,
    ) -> PyResult<()> {
        if *first {
            *first = false;
        } else {
            output.push(b',');
        }
        write_json_string(output, key)?;
        output.push(b':');
        self.write_json_node(py, value_node, value, remaining_depth, output)
    }

    fn node_matches_model_value(
        &self,
        py: Python<'_>,
        node_id: usize,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<bool> {
        let Some(node) = self.nodes.get(node_id) else {
            return Ok(false);
        };
        Ok(match node {
            ConversionNode::Scalar { kind, .. } => match kind {
                ScalarKind::Any => true,
                ScalarKind::Missing => self.node_value_is_missing(py, node_id, value),
                ScalarKind::String => value.is_instance_of::<PyString>(),
                ScalarKind::Integer => {
                    value.is_instance_of::<PyInt>() && !value.is_instance_of::<PyBool>()
                }
                ScalarKind::Number => {
                    !value.is_instance_of::<PyBool>()
                        && (value.is_instance_of::<PyInt>() || value.is_instance_of::<PyFloat>())
                }
                ScalarKind::Boolean => value.is_instance_of::<PyBool>(),
                ScalarKind::Null => value.is_none(),
            },
            ConversionNode::List { .. } => {
                value.is_instance_of::<PyTuple>()
                    && !value.is_instance(self.frozen_dict_type.bind(py))?
            }
            ConversionNode::Dict { .. } => value.is_instance(self.frozen_dict_type.bind(py))?,
            ConversionNode::Literal { values, .. } => literal_index(py, values, value)?.is_some(),
            ConversionNode::Union { branches, .. } => {
                let mut matches = false;
                for branch in branches {
                    if self.node_matches_model_value(py, *branch, value)? {
                        matches = true;
                        break;
                    }
                }
                matches
            }
            ConversionNode::Model { model_type, .. } | ConversionNode::Root { model_type, .. } => {
                value.is_instance(model_type.bind(py))?
            }
        })
    }

    fn node_value_is_missing(
        &self,
        py: Python<'_>,
        node_id: usize,
        value: &Bound<'_, PyAny>,
    ) -> bool {
        match self.nodes.get(node_id) {
            Some(ConversionNode::Scalar {
                kind: ScalarKind::Missing,
                missing_sentinel: Some(sentinel),
            }) => value.is(sentinel.bind(py)),
            Some(ConversionNode::Union { branches, .. }) => branches
                .iter()
                .any(|branch| self.node_value_is_missing(py, *branch, value)),
            _ => false,
        }
    }
}

impl<'converter> ModelProjection<'converter> {
    pub(crate) fn instance<'a, 'py>(&'a self, value: &'a Bound<'py, PyAny>) -> JsonInstanceRef<'a>
    where
        'py: 'a,
    {
        JsonInstanceRef::from_projected_python(value, self.converter.root, self)
    }

    fn retain<'a>(&'a self, node: usize, value: Bound<'a, PyAny>) -> ProjectedPythonValue<'a> {
        let py = value.py();
        let pointer = value.as_ptr();
        self.retained.borrow_mut().push(value.unbind());
        // SAFETY: `retained` owns a reference to this object until the
        // projection is dropped, which is after every projected borrow.
        let borrowed: Borrowed<'a, 'a, PyAny> = unsafe { Borrowed::from_ptr(py, pointer) };
        ProjectedPythonValue::new(node, borrowed)
    }

    fn attribute<'a>(
        &'a self,
        value: ProjectedPythonValue<'a>,
        name: &Py<PyString>,
        node: usize,
    ) -> Option<ProjectedPythonValue<'a>> {
        let py = value.value().py();
        let attribute = value.value().getattr(name.bind(py)).ok()?;
        Some(self.retain(node, attribute))
    }

    fn model_attribute<'a>(
        &'a self,
        value: ProjectedPythonValue<'a>,
        model_type: &Py<PyType>,
        attribute: &ModelAttribute,
        node: usize,
    ) -> Option<ProjectedPythonValue<'a>> {
        let py = value.value().py();
        if let Some(child) =
            attribute.native_value_ptr_from_object(py, value.value().as_ptr(), model_type)
        {
            return self.child_from_ptr(value, node, child);
        }
        self.attribute(value, &attribute.name, node)
    }

    fn child_from_ptr<'a>(
        &'a self,
        parent: ProjectedPythonValue<'a>,
        node: usize,
        child: *mut ffi::PyObject,
    ) -> Option<ProjectedPythonValue<'a>> {
        // SAFETY: callers obtain `child` as a borrowed entry from a Python
        // container retained by `parent` for the full projection lifetime.
        let child: Borrowed<'a, 'a, PyAny> =
            unsafe { Borrowed::from_ptr_or_opt(parent.value().py(), child) }?;
        Some(ProjectedPythonValue::new(node, child))
    }

    fn resolve<'a>(
        &'a self,
        value: ProjectedPythonValue<'a>,
        remaining_depth: u16,
    ) -> ProjectedPythonKind<'a> {
        if remaining_depth == 0 {
            return ProjectedPythonKind::Invalid;
        }
        let Some(node) = self.converter.nodes.get(value.node()) else {
            return ProjectedPythonKind::Invalid;
        };
        match node {
            ConversionNode::Scalar {
                kind: ScalarKind::Any,
                ..
            } => {
                let py = value.value().py();
                if value
                    .value()
                    .is_instance(self.converter.frozen_dict_type.bind(py))
                    .unwrap_or(false)
                {
                    ProjectedPythonKind::Object(value)
                } else if value.value().cast::<PyTuple>().is_ok() {
                    ProjectedPythonKind::Array(value)
                } else {
                    ProjectedPythonKind::Native(value)
                }
            }
            ConversionNode::Scalar { .. } | ConversionNode::Literal { .. } => {
                ProjectedPythonKind::Native(value)
            }
            ConversionNode::List { .. } => {
                let py = value.value().py();
                if value.value().cast::<PyTuple>().is_ok()
                    && !value
                        .value()
                        .is_instance(self.converter.frozen_dict_type.bind(py))
                        .unwrap_or(false)
                {
                    ProjectedPythonKind::Array(value)
                } else {
                    ProjectedPythonKind::Invalid
                }
            }
            ConversionNode::Dict { .. } => {
                let py = value.value().py();
                if value
                    .value()
                    .is_instance(self.converter.frozen_dict_type.bind(py))
                    .unwrap_or(false)
                {
                    ProjectedPythonKind::Object(value)
                } else {
                    ProjectedPythonKind::Invalid
                }
            }
            ConversionNode::Union { branches, .. } => {
                let py = value.value().py();
                let cache_key = (value.node(), value.value().as_ptr() as usize);
                let selected = self.union_branches.borrow().get(&cache_key).copied();
                let selected = selected.or_else(|| {
                    let bound = value.value().to_owned();
                    let mut first_match = None;
                    let mut match_count = 0;
                    for branch in branches {
                        if self
                            .converter
                            .node_matches_model_value(py, *branch, &bound)
                            .unwrap_or(false)
                        {
                            first_match.get_or_insert(*branch);
                            match_count += 1;
                        }
                    }
                    if match_count == 1 {
                        let branch = first_match.expect("one matching branch was recorded");
                        return Some(branch);
                    }
                    for branch in branches {
                        if !self
                            .converter
                            .node_matches_model_value(py, *branch, &bound)
                            .unwrap_or(false)
                        {
                            continue;
                        }
                        let mut scratch = Vec::new();
                        if self
                            .converter
                            .write_json_node(py, *branch, &bound, remaining_depth - 1, &mut scratch)
                            .is_ok()
                        {
                            self.union_branches.borrow_mut().insert(cache_key, *branch);
                            return Some(*branch);
                        }
                    }
                    None
                });
                if let Some(selected) = selected {
                    self.resolve(
                        ProjectedPythonValue::new(selected, value.value()),
                        remaining_depth - 1,
                    )
                } else {
                    ProjectedPythonKind::Invalid
                }
            }
            ConversionNode::Model { model_type, .. } => {
                let py = value.value().py();
                if value
                    .value()
                    .is_instance(model_type.bind(py))
                    .unwrap_or(false)
                {
                    ProjectedPythonKind::Object(value)
                } else {
                    ProjectedPythonKind::Invalid
                }
            }
            ConversionNode::Root {
                model_type,
                value: value_node,
                root_attribute,
                ..
            } => {
                let py = value.value().py();
                if !value
                    .value()
                    .is_instance(model_type.bind(py))
                    .unwrap_or(false)
                {
                    return ProjectedPythonKind::Invalid;
                }
                self.model_attribute(value, model_type, root_attribute, *value_node)
                    .map_or(ProjectedPythonKind::Invalid, |root| {
                        self.resolve(root, remaining_depth - 1)
                    })
            }
        }
    }

    fn normalized<'a>(&'a self, value: ProjectedPythonValue<'a>) -> ProjectedPythonValue<'a> {
        match self.resolve(value, MAX_MODEL_DEPTH) {
            ProjectedPythonKind::Native(value)
            | ProjectedPythonKind::Array(value)
            | ProjectedPythonKind::Object(value) => value,
            ProjectedPythonKind::Invalid => value,
        }
    }

    fn mapping_storage<'a>(
        &'a self,
        value: ProjectedPythonValue<'a>,
    ) -> Option<Borrowed<'a, 'a, PyTuple>> {
        let storage = self.model_attribute(
            value,
            &self.converter.frozen_dict_type,
            &self.converter.frozen_dict_items_attribute,
            value.node(),
        )?;
        storage.value().cast::<PyTuple>().ok()
    }

    fn extra_mapping<'a>(
        &'a self,
        value: ProjectedPythonValue<'a>,
        model_type: &Py<PyType>,
        extra_attribute: &ModelAttribute,
    ) -> Option<Borrowed<'a, 'a, PyTuple>> {
        let extra = self.model_attribute(value, model_type, extra_attribute, value.node())?;
        self.mapping_storage(extra)
    }

    fn field_value<'a>(
        &'a self,
        value: ProjectedPythonValue<'a>,
        model_type: &Py<PyType>,
        field: &FieldPlan,
    ) -> Option<ProjectedPythonValue<'a>> {
        let child = self.model_attribute(value, model_type, &field.attribute, field.value_node)?;
        if field
            .missing_sentinel
            .as_ref()
            .is_some_and(|sentinel| child.value().is(sentinel.bind(child.value().py())))
        {
            None
        } else {
            Some(self.normalized(child))
        }
    }

    fn dict_get<'a>(
        &'a self,
        value: ProjectedPythonValue<'a>,
        dictionary: Borrowed<'a, 'a, PyTuple>,
        key: &str,
        child_node: usize,
    ) -> Option<ProjectedPythonValue<'a>> {
        for index in 0..dictionary.len() {
            let (candidate, child) = Self::mapping_entry(value, dictionary, index)?;
            if candidate == key {
                let child = self.child_from_ptr(value, child_node, child)?;
                return Some(self.normalized(child));
            }
        }
        None
    }

    fn dict_next<'a>(
        &'a self,
        value: ProjectedPythonValue<'a>,
        dictionary: Borrowed<'a, 'a, PyTuple>,
        position: &mut usize,
        child_node: usize,
    ) -> Option<(&'a str, ProjectedPythonValue<'a>)> {
        if *position >= dictionary.len() {
            return None;
        }
        let (key, child) = Self::mapping_entry(value, dictionary, *position)?;
        *position += 1;
        let child = self.child_from_ptr(value, child_node, child)?;
        Some((key, self.normalized(child)))
    }

    fn mapping_entry<'a>(
        value: ProjectedPythonValue<'a>,
        dictionary: Borrowed<'a, 'a, PyTuple>,
        index: usize,
    ) -> Option<(&'a str, *mut ffi::PyObject)> {
        let index = ffi::Py_ssize_t::try_from(index).ok()?;
        // SAFETY: `dictionary` is a live tuple and callers bounds-check index.
        let entry = unsafe { ffi::PyTuple_GetItem(dictionary.as_ptr(), index) };
        if entry.is_null() || unsafe { ffi::PyTuple_Check(entry) } == 0 {
            return None;
        }
        if unsafe { ffi::PyTuple_Size(entry) } != 2 {
            return None;
        }
        // SAFETY: the pair length check proves both borrowed entries exist.
        let key = unsafe { ffi::PyTuple_GetItem(entry, 0) };
        let child = unsafe { ffi::PyTuple_GetItem(entry, 1) };
        let key: Borrowed<'a, 'a, PyAny> =
            unsafe { Borrowed::from_ptr_or_opt(value.value().py(), key) }?;
        Some((borrowed_python_string(key)?, child))
    }

    fn mapping_contains(
        value: ProjectedPythonValue<'_>,
        dictionary: Borrowed<'_, '_, PyTuple>,
        key: &str,
    ) -> bool {
        (0..dictionary.len()).any(|index| {
            Self::mapping_entry(value, dictionary, index)
                .is_some_and(|(candidate, _)| candidate == key)
        })
    }

    fn mapping_keys_are_strings(
        value: ProjectedPythonValue<'_>,
        dictionary: Borrowed<'_, '_, PyTuple>,
    ) -> bool {
        (0..dictionary.len()).all(|index| Self::mapping_entry(value, dictionary, index).is_some())
    }
}

impl PythonInstanceProvider for ModelProjection<'_> {
    fn has_prevalidated_schemas(&self) -> bool {
        self.converter.has_prevalidated_schemas
    }

    fn prevalidated_schema<'a>(
        &'a self,
        value: ProjectedPythonValue<'a>,
    ) -> Option<&'a serde_json::Value> {
        let (model_type, validated_attribute, schema) =
            match self.converter.nodes.get(value.node())? {
                ConversionNode::Model {
                    model_type,
                    validated_attribute,
                    prevalidated_schema,
                    ..
                }
                | ConversionNode::Root {
                    model_type,
                    validated_attribute,
                    prevalidated_schema,
                    ..
                } => (
                    model_type,
                    validated_attribute,
                    prevalidated_schema.as_ref()?,
                ),
                _ => return None,
            };
        let py = value.value().py();
        if unsafe { ffi::Py_TYPE(value.value().as_ptr()) } != model_type.bind(py).as_ptr().cast() {
            return None;
        }
        validated_attribute
            .get(&value.value().to_owned(), model_type)
            .ok()
            .and_then(|validated| validated.extract::<bool>().ok())
            .unwrap_or(false)
            .then_some(schema)
    }

    fn project<'a>(&'a self, value: ProjectedPythonValue<'a>) -> ProjectedPythonKind<'a> {
        self.resolve(value, MAX_MODEL_DEPTH)
    }

    fn array_len(&self, value: ProjectedPythonValue<'_>) -> usize {
        match self.converter.nodes.get(value.node()) {
            Some(ConversionNode::List { .. })
            | Some(ConversionNode::Scalar {
                kind: ScalarKind::Any,
                ..
            }) => {}
            _ => return 0,
        }
        value
            .value()
            .cast::<PyTuple>()
            .map_or(0, |items| items.len())
    }

    fn array_get<'a>(
        &'a self,
        value: ProjectedPythonValue<'a>,
        index: usize,
    ) -> Option<ProjectedPythonValue<'a>> {
        let item = match self.converter.nodes.get(value.node())? {
            ConversionNode::List { item } => *item,
            ConversionNode::Scalar {
                kind: ScalarKind::Any,
                ..
            } => value.node(),
            _ => return None,
        };
        let items = value.value().cast::<PyTuple>().ok()?;
        if index >= items.len() {
            return None;
        }
        let index = ffi::Py_ssize_t::try_from(index).ok()?;
        // SAFETY: the type and bounds checks above prove this returns a
        // borrowed non-null tuple entry owned by `value`.
        let child = unsafe { ffi::PyTuple_GetItem(items.as_ptr(), index) };
        let child = self.child_from_ptr(value, item, child)?;
        Some(self.normalized(child))
    }

    fn object_len(&self, value: ProjectedPythonValue<'_>) -> usize {
        match self.converter.nodes.get(value.node()) {
            Some(ConversionNode::Scalar {
                kind: ScalarKind::Any,
                ..
            }) => self
                .mapping_storage(value)
                .map_or(0, |dictionary| dictionary.len()),
            Some(ConversionNode::Dict { .. }) => self
                .mapping_storage(value)
                .map_or(0, |dictionary| dictionary.len()),
            Some(ConversionNode::Model {
                model_type,
                fields,
                serialized_fields,
                required_field_count,
                omittable_fields,
                extra_attribute,
                ..
            }) => {
                if extra_attribute.is_none() {
                    return *required_field_count
                        + omittable_fields
                            .iter()
                            .filter(|field_index| {
                                self.field_value(value, model_type, &fields[**field_index])
                                    .is_some()
                            })
                            .count();
                }
                let extra = extra_attribute
                    .as_ref()
                    .and_then(|attribute| self.extra_mapping(value, model_type, attribute));
                let mut len = extra.map_or(0, |dictionary| dictionary.len());
                for field_index in serialized_fields {
                    let field = &fields[*field_index];
                    if extra.is_some_and(|dictionary| {
                        Self::mapping_contains(value, dictionary, field.json_name.as_str())
                    }) {
                        continue;
                    }
                    if self.field_value(value, model_type, field).is_some() {
                        len += 1;
                    }
                }
                len
            }
            _ => 0,
        }
    }

    fn object_keys_are_strings(&self, value: ProjectedPythonValue<'_>) -> bool {
        match self.converter.nodes.get(value.node()) {
            Some(ConversionNode::Scalar {
                kind: ScalarKind::Any,
                ..
            }) => self
                .mapping_storage(value)
                .is_some_and(|dictionary| Self::mapping_keys_are_strings(value, dictionary)),
            Some(ConversionNode::Dict { .. }) => self
                .mapping_storage(value)
                .is_some_and(|dictionary| Self::mapping_keys_are_strings(value, dictionary)),
            Some(ConversionNode::Model {
                model_type,
                extra_attribute: Some(extra_attribute),
                ..
            }) => self
                .extra_mapping(value, model_type, extra_attribute)
                .is_some_and(|dictionary| Self::mapping_keys_are_strings(value, dictionary)),
            Some(ConversionNode::Model { .. }) => true,
            _ => false,
        }
    }

    fn object_get<'a>(
        &'a self,
        value: ProjectedPythonValue<'a>,
        key: &str,
    ) -> Option<ProjectedPythonValue<'a>> {
        match self.converter.nodes.get(value.node())? {
            ConversionNode::Scalar {
                kind: ScalarKind::Any,
                ..
            } => {
                let dictionary = self.mapping_storage(value)?;
                self.dict_get(value, dictionary, key, value.node())
            }
            ConversionNode::Dict {
                value: child_node, ..
            } => {
                let dictionary = self.mapping_storage(value)?;
                self.dict_get(value, dictionary, key, *child_node)
            }
            ConversionNode::Model {
                model_type,
                fields,
                fields_by_json_name,
                extra_value,
                extra_attribute,
                ..
            } => {
                if let (Some(extra_node), Some(extra_attribute)) = (extra_value, extra_attribute) {
                    let extra = self.extra_mapping(value, model_type, extra_attribute)?;
                    if let Some(child) = self.dict_get(value, extra, key, *extra_node) {
                        return Some(child);
                    }
                }
                let field = fields.get(*fields_by_json_name.get(key)?)?;
                self.field_value(value, model_type, field)
            }
            _ => None,
        }
    }

    fn object_next<'a>(
        &'a self,
        value: ProjectedPythonValue<'a>,
        state: &mut [usize; 2],
    ) -> Option<(&'a str, ProjectedPythonValue<'a>)> {
        match self.converter.nodes.get(value.node())? {
            ConversionNode::Scalar {
                kind: ScalarKind::Any,
                ..
            } => {
                let dictionary = self.mapping_storage(value)?;
                self.dict_next(value, dictionary, &mut state[0], value.node())
            }
            ConversionNode::Dict {
                value: child_node, ..
            } => {
                let dictionary = self.mapping_storage(value)?;
                self.dict_next(value, dictionary, &mut state[0], *child_node)
            }
            ConversionNode::Model {
                model_type,
                fields,
                serialized_fields,
                extra_value,
                extra_attribute,
                ..
            } => {
                let extra = extra_attribute
                    .as_ref()
                    .and_then(|attribute| self.extra_mapping(value, model_type, attribute));
                while state[0] < serialized_fields.len() {
                    let field_index = serialized_fields[state[0]];
                    state[0] += 1;
                    let field = &fields[field_index];
                    if extra.is_some_and(|dictionary| {
                        Self::mapping_contains(value, dictionary, field.json_name.as_str())
                    }) {
                        continue;
                    }
                    if let Some(child) = self.field_value(value, model_type, field) {
                        return Some((field.json_name.as_str(), child));
                    }
                }
                match (extra, extra_value) {
                    (Some(extra), Some(extra_node)) => {
                        self.dict_next(value, extra, &mut state[1], *extra_node)
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

fn convert_jiter_scalar_value(
    py: Python<'_>,
    kind: ScalarKind,
    value: &JiterJsonValue<'_>,
) -> PyResult<Py<PyAny>> {
    let valid = match kind {
        ScalarKind::Any => true,
        ScalarKind::Missing => false,
        ScalarKind::String => matches!(value, JiterJsonValue::Str(_)),
        ScalarKind::Integer => match value {
            JiterJsonValue::Int(_) | JiterJsonValue::BigInt(_) => true,
            JiterJsonValue::Float(value) => value.fract() == 0.0,
            _ => false,
        },
        ScalarKind::Number => matches!(
            value,
            JiterJsonValue::Int(_) | JiterJsonValue::BigInt(_) | JiterJsonValue::Float(_)
        ),
        ScalarKind::Boolean => matches!(value, JiterJsonValue::Bool(_)),
        ScalarKind::Null => matches!(value, JiterJsonValue::Null),
    };
    if !valid {
        return Err(PyErr::new::<PyTypeError, _>(format!(
            "expected {}",
            scalar_name(kind)
        )));
    }

    let python_value = if matches!(kind, ScalarKind::Integer) {
        if let JiterJsonValue::Float(number) = value {
            number.into_pyobject(py)?.call_method0("__int__")?.unbind()
        } else {
            value.into_pyobject(py)?.unbind()
        }
    } else {
        value.into_pyobject(py)?.unbind()
    };
    Ok(python_value)
}

fn mapping_pair<'py>(
    entry: &Bound<'py, PyAny>,
) -> PyResult<(Bound<'py, PyAny>, Bound<'py, PyAny>)> {
    let pair = entry
        .cast::<PyTuple>()
        .map_err(|_| PyErr::new::<PyTypeError, _>("mapping items must be key-value pairs"))?;
    if pair.len() != 2 {
        return Err(PyErr::new::<PyTypeError, _>(
            "mapping items must be key-value pairs",
        ));
    }
    Ok((pair.get_item(0)?, pair.get_item(1)?))
}

fn canonical_python_scalar(
    py: Python<'_>,
    value: &Bound<'_, PyAny>,
) -> PyResult<Option<Py<PyAny>>> {
    if value.is_none()
        || value.is_exact_instance_of::<PyBool>()
        || value.is_exact_instance_of::<PyInt>()
        || value.is_exact_instance_of::<PyString>()
    {
        return Ok(Some(value.clone().unbind()));
    }
    if value.is_exact_instance_of::<PyFloat>() {
        let number = value.extract::<f64>()?;
        if !number.is_finite() {
            return Err(PyErr::new::<PyValueError, _>("JSON numbers must be finite"));
        }
        return Ok(Some(value.clone().unbind()));
    }
    if value.is_instance_of::<PyInt>() {
        return Ok(Some(
            py.get_type::<PyInt>()
                .getattr("__int__")?
                .call1((value,))?
                .unbind(),
        ));
    }
    if value.is_instance_of::<PyFloat>() {
        let number = value.extract::<f64>()?;
        if !number.is_finite() {
            return Err(PyErr::new::<PyValueError, _>("JSON numbers must be finite"));
        }
        return Ok(Some(
            py.get_type::<PyFloat>()
                .getattr("__float__")?
                .call1((value,))?
                .unbind(),
        ));
    }
    if value.is_instance_of::<PyString>() {
        return Ok(Some(
            py.get_type::<PyString>()
                .getattr("__str__")?
                .call1((value,))?
                .unbind(),
        ));
    }
    Ok(None)
}

fn duplicate_key(key: impl std::fmt::Display) -> PyErr {
    PyErr::new::<PyValueError, _>(format!("duplicate key: `{key}`"))
}

fn copy_python_json_value(
    py: Python<'_>,
    value: &Bound<'_, PyAny>,
    remaining_depth: u16,
) -> PyResult<Py<PyAny>> {
    if remaining_depth == 0 {
        return Err(PyErr::new::<PyValueError, _>(
            "generated model serialization exceeds the maximum nesting depth",
        ));
    }
    if let Ok(input) = value.cast::<PyMapping>() {
        let output = PyDict::new(py);
        for entry in input.items()? {
            let (key, item) = mapping_pair(&entry)?;
            let key = key
                .extract::<String>()
                .map_err(|_| PyErr::new::<PyTypeError, _>("JSON object keys must be strings"))?;
            output.set_item(key, copy_python_json_value(py, &item, remaining_depth - 1)?)?;
        }
        return Ok(output.into_any().unbind());
    }
    if let Ok(input) = value.cast::<PyList>() {
        let output = PyList::empty(py);
        for item in input {
            output.append(copy_python_json_value(py, &item, remaining_depth - 1)?)?;
        }
        return Ok(output.into_any().unbind());
    }
    if let Ok(input) = value.cast::<PyTuple>() {
        let output = PyList::empty(py);
        for item in input {
            output.append(copy_python_json_value(py, &item, remaining_depth - 1)?)?;
        }
        return Ok(output.into_any().unbind());
    }
    Ok(value.clone().unbind())
}

fn write_serializable_json_value(output: &mut Vec<u8>, value: &Bound<'_, PyAny>) -> PyResult<()> {
    if value.is_none() {
        output.extend_from_slice(b"null");
        return Ok(());
    }
    if value.is_instance_of::<PyBool>() {
        output.extend_from_slice(if value.extract()? { b"true" } else { b"false" });
        return Ok(());
    }
    if value.is_instance_of::<PyInt>() {
        if let Ok(number) = value.extract::<i64>() {
            return serde_json::to_writer(&mut *output, &number).map_err(json_serialization_error);
        }
        if let Ok(number) = value.extract::<u64>() {
            return serde_json::to_writer(&mut *output, &number).map_err(json_serialization_error);
        }
        let rendered = value
            .py()
            .get_type::<PyInt>()
            .getattr("__repr__")?
            .call1((value,))?;
        output.extend_from_slice(rendered.cast::<PyString>()?.to_str()?.as_bytes());
        return Ok(());
    }
    if value.is_instance_of::<PyFloat>() {
        let number = value.extract::<f64>()?;
        if !number.is_finite() {
            return Err(PyErr::new::<PyValueError, _>("JSON numbers must be finite"));
        }
        return serde_json::to_writer(&mut *output, &number).map_err(json_serialization_error);
    }
    if let Ok(value) = value.cast::<PyString>() {
        return write_json_string(output, value.to_str()?);
    }

    let value = super::py_to_serializable_json_value(value)?;
    serde_json::to_writer(&mut *output, &value).map_err(json_serialization_error)
}

fn write_json_string(output: &mut Vec<u8>, value: &str) -> PyResult<()> {
    serde_json::to_writer(&mut *output, value).map_err(json_serialization_error)
}

fn borrowed_python_string<'a>(value: Borrowed<'a, 'a, PyAny>) -> Option<&'a str> {
    value.cast::<PyString>().ok()?;
    let mut size = 0;
    // SAFETY: the cast above proves this is Unicode. CPython owns its cached
    // UTF-8 buffer for at least the lifetime represented by `value`.
    let data = unsafe { ffi::PyUnicode_AsUTF8AndSize(value.as_ptr(), &raw mut size) };
    if data.is_null() {
        return None;
    }
    let size = usize::try_from(size).ok()?;
    // SAFETY: PyUnicode_AsUTF8AndSize returns valid UTF-8 of exactly `size` bytes.
    let bytes = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), size) };
    Some(unsafe { std::str::from_utf8_unchecked(bytes) })
}

fn json_serialization_error(error: serde_json::Error) -> PyErr {
    PyErr::new::<PyValueError, _>(format!("JSON serialization failed: {error}"))
}

fn convert_scalar(
    py: Python<'_>,
    kind: ScalarKind,
    missing_sentinel: Option<&Py<PyAny>>,
    value: &Bound<'_, PyAny>,
) -> ConversionResult<Py<PyAny>> {
    let valid = match kind {
        ScalarKind::Any => true,
        ScalarKind::Missing => missing_sentinel.is_some_and(|sentinel| value.is(sentinel.bind(py))),
        ScalarKind::String => value.is_instance_of::<PyString>(),
        ScalarKind::Integer => {
            (value.is_instance_of::<PyInt>() && !value.is_instance_of::<PyBool>())
                || (value.is_instance_of::<PyFloat>()
                    && value.extract::<f64>()?.is_finite()
                    && value.extract::<f64>()?.fract() == 0.0)
        }
        ScalarKind::Number => {
            !value.is_instance_of::<PyBool>()
                && (value.is_instance_of::<PyInt>()
                    || (value.is_instance_of::<PyFloat>() && value.extract::<f64>()?.is_finite()))
        }
        ScalarKind::Boolean => value.is_instance_of::<PyBool>(),
        ScalarKind::Null => value.is_none(),
    };
    if !valid {
        return Err(ConversionFailure::Mismatch(expected_type(
            scalar_name(kind),
            value,
        )?));
    }

    let converted = match kind {
        ScalarKind::String if value.is_exact_instance_of::<PyString>() => value.clone().unbind(),
        ScalarKind::String => py
            .get_type::<PyString>()
            .getattr("__str__")?
            .call1((value,))?
            .unbind(),
        ScalarKind::Integer if value.is_exact_instance_of::<PyInt>() => value.clone().unbind(),
        ScalarKind::Integer if value.is_instance_of::<PyFloat>() => py
            .get_type::<PyFloat>()
            .getattr("__int__")?
            .call1((value,))?
            .unbind(),
        ScalarKind::Integer if value.is_instance_of::<PyInt>() => py
            .get_type::<PyInt>()
            .getattr("__int__")?
            .call1((value,))?
            .unbind(),
        ScalarKind::Number
            if value.is_exact_instance_of::<PyInt>() || value.is_exact_instance_of::<PyFloat>() =>
        {
            value.clone().unbind()
        }
        ScalarKind::Number if value.is_instance_of::<PyInt>() => py
            .get_type::<PyInt>()
            .getattr("__int__")?
            .call1((value,))?
            .unbind(),
        ScalarKind::Number if value.is_instance_of::<PyFloat>() => py
            .get_type::<PyFloat>()
            .getattr("__float__")?
            .call1((value,))?
            .unbind(),
        ScalarKind::Any | ScalarKind::Missing | ScalarKind::Boolean | ScalarKind::Null => {
            value.clone().unbind()
        }
        ScalarKind::Integer | ScalarKind::Number => {
            return Err(ConversionFailure::Mismatch(expected_type(
                scalar_name(kind),
                value,
            )?));
        }
    };
    Ok(converted)
}

fn convert_direct_scalar(
    py: Python<'_>,
    kind: ScalarKind,
    missing_sentinel: Option<&Py<PyAny>>,
    value: &Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    if matches!(kind, ScalarKind::Integer)
        && (value.is_instance_of::<PyBool>() || !value.is_instance_of::<PyInt>())
    {
        return Err(expected_type("int", value)?);
    }
    if matches!(kind, ScalarKind::Number)
        && value.is_instance_of::<PyFloat>()
        && !value.extract::<f64>()?.is_finite()
    {
        return Err(PyErr::new::<PyValueError, _>("JSON numbers must be finite"));
    }
    convert_scalar(py, kind, missing_sentinel, value).map_err(ConversionFailure::into_pyerr)
}

fn scalar_name(kind: ScalarKind) -> &'static str {
    match kind {
        ScalarKind::Any => "JSON value",
        ScalarKind::Missing => "JSONCOMPAT_MISSING",
        ScalarKind::String => "str",
        ScalarKind::Integer => "int",
        ScalarKind::Number => "number",
        ScalarKind::Boolean => "bool",
        ScalarKind::Null => "null",
    }
}

fn convert_literal(
    py: Python<'_>,
    values: &[Py<PyAny>],
    value: &Bound<'_, PyAny>,
) -> ConversionResult<Py<PyAny>> {
    let index = if value.is_none()
        || value.is_exact_instance_of::<PyBool>()
        || value.is_exact_instance_of::<PyInt>()
        || value.is_exact_instance_of::<PyFloat>()
        || value.is_exact_instance_of::<PyString>()
    {
        literal_index(py, values, value)?
    } else if let Some(canonical) = canonical_python_scalar(py, value)? {
        literal_index(py, values, canonical.bind(py))?
    } else {
        literal_index(py, values, value)?
    };
    if let Some(index) = index {
        Ok(values[index].clone_ref(py))
    } else {
        Err(ConversionFailure::Mismatch(PyErr::new::<PyTypeError, _>(
            "value does not match the generated literal",
        )))
    }
}

fn literal_index(
    py: Python<'_>,
    values: &[Py<PyAny>],
    value: &Bound<'_, PyAny>,
) -> PyResult<Option<usize>> {
    for (index, literal) in values.iter().enumerate() {
        let literal = literal.bind(py);
        let either_bool = literal.is_instance_of::<PyBool>() || value.is_instance_of::<PyBool>();
        let both_numbers = !either_bool
            && (literal.is_instance_of::<PyInt>() || literal.is_instance_of::<PyFloat>())
            && (value.is_instance_of::<PyInt>() || value.is_instance_of::<PyFloat>());
        let same_type = literal.get_type().is(value.get_type());
        if (both_numbers || same_type) && literal.eq(value)? {
            return Ok(Some(index));
        }
    }
    Ok(None)
}

fn expected_type(expected: &str, value: &Bound<'_, PyAny>) -> PyResult<PyErr> {
    Ok(PyErr::new::<PyTypeError, _>(format!(
        "expected {expected}, got {}",
        value.get_type().name()?
    )))
}

fn unexpected_keyword(py: Python<'_>, model_type: &Py<PyType>, keyword: &str) -> PyErr {
    let model_name = model_type
        .bind(py)
        .name()
        .ok()
        .and_then(|name| name.to_str().ok().map(str::to_owned))
        .unwrap_or_else(|| "generated model".to_owned());
    PyErr::new::<PyTypeError, _>(format!(
        "{model_name}.__init__() got an unexpected keyword argument '{keyword}'"
    ))
}

fn allocate_model<'py>(
    py: Python<'py>,
    model_type: &Py<PyType>,
    object_new: &Py<PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    #[cfg(all(Py_3_11, not(any(PyPy, GraalPy, Py_GIL_DISABLED))))]
    {
        let model_type = model_type.bind(py).as_ptr().cast::<ffi::PyTypeObject>();
        let object_type = std::ptr::addr_of_mut!(ffi::PyBaseObject_Type);
        // SAFETY: generated model types are validated heap types. Calling
        // their allocator is the allocation step performed by
        // `object.__new__(model_type)`, without its Python call overhead.
        let uses_object_new = unsafe {
            (*model_type).tp_itemsize == 0
                && (*model_type).tp_new.map(|function| function as usize)
                    == (*object_type).tp_new.map(|function| function as usize)
        };
        if uses_object_new && let Some(allocate) = unsafe { (*model_type).tp_alloc } {
            let instance = unsafe { allocate(model_type, 0) };
            return unsafe { Bound::from_owned_ptr_or_err(py, instance) };
        }
    }
    object_new.bind(py).call1((model_type.bind(py),))
}

pub(crate) fn compile_model_converter_plan(
    py: Python<'_>,
    descriptors: &Bound<'_, PyList>,
    frozen_list_type: &Bound<'_, PyType>,
    frozen_dict_type: &Bound<'_, PyType>,
) -> PyResult<Rc<ModelConverterPlan>> {
    if !frozen_list_type.is_subclass_of::<PyTuple>()? {
        return Err(PyErr::new::<PyTypeError, _>(
            "generated immutable sequence type must be a tuple subclass",
        ));
    }
    let mut nodes = Vec::with_capacity(descriptors.len());
    for descriptor in descriptors {
        nodes.push(parse_node(py, &descriptor)?);
    }
    validate_references(&nodes)?;
    let has_prevalidated_schemas = nodes.iter().any(|node| match node {
        ConversionNode::Model {
            prevalidated_schema,
            ..
        }
        | ConversionNode::Root {
            prevalidated_schema,
            ..
        } => prevalidated_schema.is_some(),
        _ => false,
    });
    let object_new = py.get_type::<PyAny>().getattr("__new__")?.unbind();
    Ok(Rc::new(ModelConverterPlan {
        nodes,
        has_prevalidated_schemas,
        object_new,
        frozen_list_type: frozen_list_type.clone().unbind(),
        frozen_dict_type: frozen_dict_type.clone().unbind(),
        frozen_dict_items_attribute: ModelAttribute::compile(py, frozen_dict_type, "_items"),
    }))
}

pub(crate) fn model_converter_for_root(
    py: Python<'_>,
    plan: Rc<ModelConverterPlan>,
    model_type: &Bound<'_, PyType>,
    root: usize,
) -> PyResult<ModelConverterPy> {
    let root_model_type = match plan.nodes.get(root) {
        Some(ConversionNode::Model { model_type, .. })
        | Some(ConversionNode::Root { model_type, .. }) => model_type,
        Some(_) => {
            return Err(PyErr::new::<PyTypeError, _>(
                "model converter root must be a generated model",
            ));
        }
        None => {
            return Err(PyErr::new::<PyIndexError, _>(
                "model converter root node is out of bounds",
            ));
        }
    };
    if !root_model_type.bind(py).is(model_type) {
        return Err(PyErr::new::<PyTypeError, _>(format!(
            "model converter root does not describe {}",
            model_type.name()?,
        )));
    }
    Ok(ModelConverterPy { plan, root })
}

#[cfg(all(Py_3_11, not(any(PyPy, GraalPy, Py_GIL_DISABLED))))]
fn validated_slot_offset(
    model_type: &Bound<'_, PyType>,
    name: &str,
) -> Option<ValidatedSlotOffset> {
    let descriptor = model_type.getattr(name).ok()?;
    // SAFETY: exact member descriptors use the public CPython
    // PyMemberDescrObject/PyMemberDef layout. We validate that the descriptor
    // belongs to this concrete layout, names the requested member, and covers
    // one aligned object-pointer slot within the allocation before retaining
    // its offset.
    unsafe {
        if ffi::Py_IS_TYPE(
            descriptor.as_ptr(),
            std::ptr::addr_of_mut!(ffi::PyMemberDescr_Type),
        ) == 0
        {
            return None;
        }
        let descriptor = descriptor.as_ptr().cast::<ffi::PyMemberDescrObject>();
        let owner = (*descriptor).d_common.d_type;
        let member = (*descriptor).d_member;
        let concrete_type = model_type.as_ptr().cast::<ffi::PyTypeObject>();
        if owner.is_null()
            || ffi::PyType_IsSubtype(concrete_type, owner) == 0
            || member.is_null()
            || (*member).name.is_null()
        {
            return None;
        }
        let member_name = std::ffi::CStr::from_ptr((*member).name);
        if member_name.to_bytes() != name.as_bytes() || (*member).type_code != ffi::Py_T_OBJECT_EX {
            return None;
        }
        let offset = usize::try_from((*member).offset).ok()?;
        let basicsize = usize::try_from((*concrete_type).tp_basicsize).ok()?;
        let pointer_size = std::mem::size_of::<*mut ffi::PyObject>();
        if offset < std::mem::size_of::<ffi::PyObject>()
            || offset % std::mem::align_of::<*mut ffi::PyObject>() != 0
            || offset.checked_add(pointer_size)? > basicsize
        {
            return None;
        }
        NonZeroIsize::new(isize::try_from(offset).ok()?).map(ValidatedSlotOffset)
    }
}

#[cfg(not(all(Py_3_11, not(any(PyPy, GraalPy, Py_GIL_DISABLED)))))]
fn validated_slot_offset(
    _model_type: &Bound<'_, PyType>,
    _name: &str,
) -> Option<ValidatedSlotOffset> {
    None
}

fn parse_node(py: Python<'_>, descriptor: &Bound<'_, PyAny>) -> PyResult<ConversionNode> {
    let descriptor = descriptor.cast::<PyTuple>()?;
    let tag = descriptor.get_item(0)?.extract::<String>()?;
    match tag.as_str() {
        "any" => Ok(scalar_node(ScalarKind::Any)),
        "missing" => Ok(ConversionNode::Scalar {
            kind: ScalarKind::Missing,
            missing_sentinel: Some(descriptor.get_item(1)?.unbind()),
        }),
        "str" => Ok(scalar_node(ScalarKind::String)),
        "int" => Ok(scalar_node(ScalarKind::Integer)),
        "float" => Ok(scalar_node(ScalarKind::Number)),
        "bool" => Ok(scalar_node(ScalarKind::Boolean)),
        "null" => Ok(scalar_node(ScalarKind::Null)),
        "list" => Ok(ConversionNode::List {
            item: descriptor.get_item(1)?.extract()?,
        }),
        "dict" => Ok(ConversionNode::Dict {
            key: descriptor.get_item(1)?.extract()?,
            value: descriptor.get_item(2)?.extract()?,
        }),
        "literal" => {
            let values = descriptor.get_item(1)?;
            let values = values.cast::<PyTuple>()?;
            let values = values.iter().map(Bound::unbind).collect::<Vec<_>>();
            let string_indices = values
                .iter()
                .enumerate()
                .filter_map(|(index, value)| {
                    value
                        .bind(py)
                        .cast::<PyString>()
                        .ok()
                        .and_then(|value| value.to_str().ok())
                        .map(|value| (value.to_owned(), index))
                })
                .collect();
            Ok(ConversionNode::Literal {
                values,
                string_indices,
            })
        }
        "union" => parse_union_node(descriptor),
        "model" => parse_model_node(py, descriptor),
        "root" => {
            let model_type = descriptor.get_item(1)?.cast_into::<PyType>()?.unbind();
            let (prevalidated_schema, branch_schema) = schemas_for_model(model_type.bind(py))?;
            Ok(ConversionNode::Root {
                branch_schema,
                prevalidated_schema,
                validated_attribute: ModelAttribute::compile(
                    py,
                    model_type.bind(py),
                    "_jsoncompat_validated",
                ),
                root_attribute: ModelAttribute::compile(py, model_type.bind(py), "root"),
                model_type,
                value: descriptor.get_item(2)?.extract()?,
            })
        }
        _ => Err(PyErr::new::<PyValueError, _>(format!(
            "unknown model converter node kind {tag:?}"
        ))),
    }
}

fn scalar_node(kind: ScalarKind) -> ConversionNode {
    ConversionNode::Scalar {
        kind,
        missing_sentinel: None,
    }
}

fn python_discriminator_key(value: &Bound<'_, PyAny>) -> Option<DiscriminatorKey> {
    if value.is_none() {
        return Some(DiscriminatorKey::Null);
    }
    if value.is_instance_of::<PyBool>() {
        return value.extract::<bool>().ok().map(DiscriminatorKey::Boolean);
    }
    if value.is_instance_of::<PyInt>() {
        return value.extract::<i64>().ok().map(DiscriminatorKey::Integer);
    }
    value
        .cast::<PyString>()
        .ok()
        .and_then(|value| value.to_str().ok())
        .map(|value| DiscriminatorKey::String(value.to_owned()))
}

fn jiter_discriminator_key(value: &JiterJsonValue<'_>) -> Option<DiscriminatorKey> {
    match value {
        JiterJsonValue::Null => Some(DiscriminatorKey::Null),
        JiterJsonValue::Bool(value) => Some(DiscriminatorKey::Boolean(*value)),
        JiterJsonValue::Int(value) => Some(DiscriminatorKey::Integer(*value)),
        JiterJsonValue::Str(value) => Some(DiscriminatorKey::String(value.as_ref().to_owned())),
        JiterJsonValue::BigInt(_)
        | JiterJsonValue::Float(_)
        | JiterJsonValue::Array(_)
        | JiterJsonValue::Object(_) => None,
    }
}

fn parse_union_node(descriptor: &Bound<'_, PyTuple>) -> PyResult<ConversionNode> {
    let branches = descriptor.get_item(1)?.cast_into::<PyTuple>()?;
    let branches = branches
        .iter()
        .map(|branch| branch.extract::<usize>())
        .collect::<PyResult<Vec<_>>>()?;
    let discriminator_name = descriptor.get_item(2)?;
    let discriminator = if discriminator_name.is_none() {
        None
    } else {
        let entries = descriptor.get_item(3)?.cast_into::<PyTuple>()?;
        let mut branches_by_value = HashMap::with_capacity(entries.len());
        for entry in entries {
            let entry = entry.cast_into::<PyTuple>()?;
            let value = entry.get_item(0)?;
            let value = python_discriminator_key(&value).ok_or_else(|| {
                PyErr::new::<PyTypeError, _>(
                    "native discriminator values must be null, bool, i64, or str",
                )
            })?;
            let branch = entry.get_item(1)?.extract::<usize>()?;
            if branches_by_value.insert(value, branch).is_some() {
                return Err(PyErr::new::<PyValueError, _>(
                    "native discriminator values must be unique",
                ));
            }
        }
        Some(DiscriminatorPlan {
            json_name: discriminator_name.extract()?,
            branches_by_value,
        })
    };
    Ok(ConversionNode::Union {
        branches,
        discriminator,
    })
}

fn parse_model_node(py: Python<'_>, descriptor: &Bound<'_, PyTuple>) -> PyResult<ConversionNode> {
    let model_type = descriptor.get_item(1)?.cast_into::<PyType>()?;
    let (prevalidated_schema, branch_schema) = schemas_for_model(&model_type)?;
    let field_descriptors = descriptor.get_item(2)?.cast_into::<PyTuple>()?;
    let mut fields = Vec::with_capacity(field_descriptors.len());
    let mut fields_by_json_name = HashMap::with_capacity(field_descriptors.len());
    let mut fields_by_py_name = HashMap::with_capacity(field_descriptors.len());
    for field in field_descriptors.iter() {
        let field = field.cast_into::<PyTuple>()?;
        let json_name = field.get_item(0)?.extract::<String>()?;
        let py_name = field.get_item(1)?.extract::<String>()?;
        fields_by_json_name.insert(json_name.clone(), fields.len());
        fields_by_py_name.insert(py_name.clone(), fields.len());
        fields.push(FieldPlan {
            json_name,
            attribute: ModelAttribute::compile(py, &model_type, &py_name),
            value_node: field.get_item(2)?.extract()?,
            missing_sentinel: {
                let sentinel = field.get_item(3)?;
                if sentinel.is_none() {
                    None
                } else {
                    Some(sentinel.unbind())
                }
            },
        });
    }
    let extra_value = descriptor.get_item(3)?;
    let extra_value = if extra_value.is_none() {
        None
    } else {
        Some(extra_value.extract::<usize>()?)
    };
    let mut serialized_fields = (0..fields.len()).collect::<Vec<_>>();
    serialized_fields
        .sort_unstable_by(|left, right| fields[*left].json_name.cmp(&fields[*right].json_name));
    let omittable_fields = fields
        .iter()
        .enumerate()
        .filter_map(|(index, field)| field.missing_sentinel.as_ref().map(|_| index))
        .collect::<Vec<_>>();
    let required_field_count = fields.len() - omittable_fields.len();
    let extra_attribute =
        extra_value.map(|_| ModelAttribute::compile(py, &model_type, "__jsoncompat_extra__"));
    let validated_attribute = ModelAttribute::compile(py, &model_type, "_jsoncompat_validated");
    Ok(ConversionNode::Model {
        model_type: model_type.unbind(),
        validated_attribute,
        branch_schema,
        prevalidated_schema,
        fields,
        fields_by_json_name,
        fields_by_py_name,
        serialized_fields,
        required_field_count,
        omittable_fields,
        extra_value,
        extra_attribute,
    })
}

fn schemas_for_model(
    model_type: &Bound<'_, PyType>,
) -> PyResult<(Option<serde_json::Value>, BranchSchema)> {
    let schema = model_type
        .getattr("__jsoncompat_schema__")?
        .extract::<String>()?;
    let schema = serde_json::from_str::<serde_json::Value>(&schema).map_err(|error| {
        PyErr::new::<PyValueError, _>(format!("generated model schema is not valid JSON: {error}"))
    })?;
    let prevalidated_schema = schema_is_context_independent(&schema).then(|| schema.clone());
    Ok((
        prevalidated_schema,
        BranchSchema {
            raw: schema,
            compiled: OnceCell::new(),
        },
    ))
}

fn schema_is_context_independent(schema: &serde_json::Value) -> bool {
    match schema {
        serde_json::Value::Array(values) => values.iter().all(schema_is_context_independent),
        serde_json::Value::Object(values) => values
            .iter()
            .all(|(key, value)| !key.starts_with('$') && schema_is_context_independent(value)),
        serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::String(_) => true,
    }
}

fn validate_references(nodes: &[ConversionNode]) -> PyResult<()> {
    let len = nodes.len();
    let check = |reference: usize| {
        if reference < len {
            Ok(())
        } else {
            Err(PyErr::new::<PyIndexError, _>(format!(
                "model converter node reference {reference} is out of bounds"
            )))
        }
    };
    for node in nodes {
        match node {
            ConversionNode::Scalar { .. } | ConversionNode::Literal { .. } => {}
            ConversionNode::List { item } => check(*item)?,
            ConversionNode::Dict { key, value } => {
                check(*key)?;
                check(*value)?;
            }
            ConversionNode::Union {
                branches,
                discriminator,
            } => {
                for branch in branches {
                    check(*branch)?;
                }
                if let Some(discriminator) = discriminator {
                    for branch in discriminator.branches_by_value.values() {
                        check(*branch)?;
                    }
                }
            }
            ConversionNode::Model {
                fields,
                extra_value,
                ..
            } => {
                for field in fields {
                    check(field.value_node)?;
                }
                if let Some(extra_value) = extra_value {
                    check(*extra_value)?;
                }
            }
            ConversionNode::Root { value, .. } => check(*value)?,
        }
    }
    Ok(())
}
