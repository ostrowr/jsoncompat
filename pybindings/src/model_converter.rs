//! Compiled conversion plans for generated Python dataclasses.
//!
//! Plans move repeated object-graph traversal into Rust while retaining the
//! Python runtime's existing type checks, missing-field factories, union
//! selection, and frozen-slot construction semantics.

use std::cell::{OnceCell, RefCell};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::num::NonZeroUsize;
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
    String,
    Integer,
    Number,
    Boolean,
    Null,
}

struct DiscriminatorPlan {
    json_name: String,
    branch_ordinals_by_value: HashMap<DiscriminatorKey, usize>,
}

struct UnionPlan {
    branches: Box<[usize]>,
    discriminator: Option<DiscriminatorPlan>,
}

impl UnionPlan {
    fn new(branches: Vec<usize>, discriminator: Option<DiscriminatorPlan>) -> PyResult<Self> {
        if branches.is_empty() {
            return Err(PyErr::new::<PyValueError, _>(
                "native union nodes must contain at least one branch",
            ));
        }
        let mut unique = HashSet::with_capacity(branches.len());
        if !branches.iter().all(|branch| unique.insert(*branch)) {
            return Err(PyErr::new::<PyValueError, _>(
                "native union branches must be unique",
            ));
        }
        if let Some(discriminator) = &discriminator
            && discriminator
                .branch_ordinals_by_value
                .values()
                .any(|branch| *branch >= branches.len())
        {
            return Err(PyErr::new::<PyIndexError, _>(
                "discriminator branch ordinal is out of bounds",
            ));
        }
        Ok(Self {
            branches: branches.into_boxed_slice(),
            discriminator,
        })
    }

    #[inline]
    fn discriminator_name(&self) -> Option<&str> {
        self.discriminator
            .as_ref()
            .map(|discriminator| discriminator.json_name.as_str())
    }

    #[inline]
    fn discriminated_branch(&self, key: &DiscriminatorKey) -> Option<usize> {
        let ordinal = *self
            .discriminator
            .as_ref()?
            .branch_ordinals_by_value
            .get(key)?;
        Some(self.branches[ordinal])
    }
}

impl Deref for UnionPlan {
    type Target = [usize];

    fn deref(&self) -> &Self::Target {
        &self.branches
    }
}

impl<'a> IntoIterator for &'a UnionPlan {
    type Item = &'a usize;
    type IntoIter = std::slice::Iter<'a, usize>;

    fn into_iter(self) -> Self::IntoIter {
        self.branches.iter()
    }
}

#[derive(Eq, Hash, PartialEq)]
enum DiscriminatorKey {
    Null,
    Boolean(bool),
    Integer(i64),
    String(String),
}

struct ValidatedNativeSlot {
    owner: Py<PyType>,
    offset: NonZeroUsize,
}

enum SlotInspection {
    Native(ValidatedNativeSlot),
    #[cfg(not(all(Py_3_11, not(any(PyPy, GraalPy, Py_GIL_DISABLED)))))]
    Portable,
}

enum AttributeStorage {
    NativeSlot(ValidatedNativeSlot),
    #[cfg(not(all(Py_3_11, not(any(PyPy, GraalPy, Py_GIL_DISABLED)))))]
    GenericDescriptor {
        owner: Py<PyType>,
    },
}

struct ModelAttribute {
    name: Py<PyString>,
    storage: AttributeStorage,
}

impl ModelAttribute {
    fn compile(py: Python<'_>, model_type: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        let storage = match inspect_native_slot(model_type, name)? {
            SlotInspection::Native(slot) => AttributeStorage::NativeSlot(slot),
            #[cfg(not(all(Py_3_11, not(any(PyPy, GraalPy, Py_GIL_DISABLED)))))]
            SlotInspection::Portable => AttributeStorage::GenericDescriptor {
                owner: model_type.clone().unbind(),
            },
        };
        Ok(Self {
            name: PyString::new(py, name).unbind(),
            storage,
        })
    }

    #[inline(always)]
    fn owner(&self) -> &Py<PyType> {
        match &self.storage {
            AttributeStorage::NativeSlot(slot) => &slot.owner,
            #[cfg(not(all(Py_3_11, not(any(PyPy, GraalPy, Py_GIL_DISABLED)))))]
            AttributeStorage::GenericDescriptor { owner } => owner,
        }
    }

    #[inline(always)]
    fn ensure_owner(&self, py: Python<'_>, object: *mut ffi::PyObject) -> PyResult<()> {
        if unsafe { ffi::Py_TYPE(object) } == self.owner().bind(py).as_ptr().cast() {
            Ok(())
        } else {
            Err(PyErr::new::<PyTypeError, _>(
                "generated model attribute used with an unexpected owner type",
            ))
        }
    }

    #[inline(always)]
    fn native_slot_ptr(
        &self,
        py: Python<'_>,
        object: *mut ffi::PyObject,
    ) -> Option<*mut *mut ffi::PyObject> {
        #[cfg(all(Py_3_11, not(any(PyPy, GraalPy, Py_GIL_DISABLED))))]
        let AttributeStorage::NativeSlot(slot) = &self.storage;
        #[cfg(not(all(Py_3_11, not(any(PyPy, GraalPy, Py_GIL_DISABLED)))))]
        let slot = match &self.storage {
            AttributeStorage::NativeSlot(slot) => slot,
            AttributeStorage::GenericDescriptor { .. } => return None,
        };
        if unsafe { ffi::Py_TYPE(object) } != slot.owner.bind(py).as_ptr().cast() {
            return None;
        }
        // SAFETY: `ValidatedNativeSlot` is only created after proving that the
        // named member descriptor belongs to `owner` and identifies an
        // aligned object-pointer slot within the concrete allocation.
        Some(unsafe {
            object
                .cast::<u8>()
                .add(slot.offset.get())
                .cast::<*mut ffi::PyObject>()
        })
    }

    #[inline(always)]
    fn native_value_ptr_from_object(
        &self,
        py: Python<'_>,
        object: *mut ffi::PyObject,
    ) -> Option<*mut ffi::PyObject> {
        let slot = self.native_slot_ptr(py, object)?;
        Some(unsafe { *slot })
    }

    #[inline(always)]
    fn get<'py>(&self, instance: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        let py = instance.py();
        self.ensure_owner(py, instance.as_ptr())?;
        if let Some(value) = self.native_value_ptr_from_object(py, instance.as_ptr())
            && !value.is_null()
        {
            // SAFETY: the instance owns the slot reference for the returned
            // bound object's lifetime.
            return Ok(unsafe { Bound::from_borrowed_ptr(py, value) });
        }
        instance.getattr(self.name.bind(py))
    }

    #[inline(always)]
    fn is_set(&self, py: Python<'_>, instance: &Bound<'_, PyAny>) -> PyResult<bool> {
        self.ensure_owner(py, instance.as_ptr())?;
        if let Some(value) = self.native_value_ptr_from_object(py, instance.as_ptr()) {
            return Ok(!value.is_null());
        }
        instance.hasattr(self.name.bind(py))
    }

    #[inline(always)]
    fn set(&self, py: Python<'_>, instance: &Bound<'_, PyAny>, value: &Py<PyAny>) -> PyResult<()> {
        self.ensure_owner(py, instance.as_ptr())?;
        if let Some(slot) = self.native_slot_ptr(py, instance.as_ptr()) {
            // SAFETY: `native_slot_ptr` proves this is the named owned
            // object-pointer slot for the exact allocation. Retain the new
            // value before replacing and releasing the previous reference.
            let value = value.bind(py).as_ptr();
            unsafe {
                ffi::Py_INCREF(value);
                let previous = std::ptr::replace(slot, value);
                ffi::Py_XDECREF(previous);
            }
            return Ok(());
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

    fn traverse(&self, visit: &PyVisit<'_>) -> Result<(), PyTraverseError> {
        visit.call(&self.name)?;
        visit.call(self.owner())?;
        Ok(())
    }
}

struct FieldPlan {
    json_name: String,
    attribute: ModelAttribute,
    value_node: usize,
    presence: FieldPresence,
}

#[derive(Clone, Copy)]
enum FieldPresence {
    Required,
    Omittable,
}

impl FieldPresence {
    fn is_omittable(self) -> bool {
        matches!(self, Self::Omittable)
    }
}

#[derive(Clone, Copy)]
struct FieldId(usize);

struct ModelFields {
    serialized: Vec<FieldPlan>,
    by_json_name: HashMap<String, FieldId>,
    by_py_name: HashMap<String, FieldId>,
    omittable: Vec<FieldId>,
}

impl ModelFields {
    fn new(mut fields: Vec<(FieldPlan, String)>) -> PyResult<Self> {
        fields.sort_unstable_by(|(left, _), (right, _)| left.json_name.cmp(&right.json_name));
        let mut serialized = Vec::with_capacity(fields.len());
        let mut by_json_name = HashMap::with_capacity(fields.len());
        let mut by_py_name = HashMap::with_capacity(fields.len());
        let mut omittable = Vec::new();
        for (field, py_name) in fields {
            let id = FieldId(serialized.len());
            if by_json_name.insert(field.json_name.clone(), id).is_some() {
                return Err(PyErr::new::<PyValueError, _>(format!(
                    "duplicate generated JSON field name {:?}",
                    field.json_name
                )));
            }
            if by_py_name.insert(py_name.clone(), id).is_some() {
                return Err(PyErr::new::<PyValueError, _>(format!(
                    "duplicate generated Python field name {py_name:?}"
                )));
            }
            if field.presence.is_omittable() {
                omittable.push(id);
            }
            serialized.push(field);
        }
        Ok(Self {
            serialized,
            by_json_name,
            by_py_name,
            omittable,
        })
    }

    #[inline]
    fn by_json_name(&self, name: &str) -> Option<&FieldPlan> {
        self.by_json_name
            .get(name)
            .map(|field| &self.serialized[field.0])
    }

    #[inline]
    fn by_py_name(&self, name: &str) -> Option<&FieldPlan> {
        self.by_py_name
            .get(name)
            .map(|field| &self.serialized[field.0])
    }

    #[inline]
    fn get(&self, field: FieldId) -> &FieldPlan {
        &self.serialized[field.0]
    }
}

impl Deref for ModelFields {
    type Target = [FieldPlan];

    fn deref(&self) -> &Self::Target {
        &self.serialized
    }
}

impl<'a> IntoIterator for &'a ModelFields {
    type Item = &'a FieldPlan;
    type IntoIter = std::slice::Iter<'a, FieldPlan>;

    fn into_iter(self) -> Self::IntoIter {
        self.serialized.iter()
    }
}

struct ExtraPropertiesPlan {
    value_node: usize,
    attribute: ModelAttribute,
}

struct MissingSentinel(Py<PyAny>);

impl MissingSentinel {
    fn bind<'py>(&self, py: Python<'py>) -> &Bound<'py, PyAny> {
        self.0.bind(py)
    }

    fn clone_ref(&self, py: Python<'_>) -> Py<PyAny> {
        self.0.clone_ref(py)
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum UnionSelection {
    FirstRepresentable,
    ValidateAmbiguousBranches,
}

pub(crate) enum ConversionMismatch {
    ExpectedType {
        expected: String,
        actual: Option<String>,
    },
    JsonObjectKey,
    Literal,
    Depth,
    UnknownProperty(String),
    MissingField(String),
    NoUnionBranch {
        first: Option<Box<ConversionMismatch>>,
    },
}

impl std::fmt::Display for ConversionMismatch {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExpectedType {
                expected,
                actual: Some(actual),
            } => write!(formatter, "expected {expected}, got {actual}"),
            Self::ExpectedType {
                expected,
                actual: None,
            } => write!(formatter, "expected {expected}"),
            Self::JsonObjectKey => formatter.write_str("JSON object keys must be strings"),
            Self::Literal => formatter.write_str("value does not match the generated literal"),
            Self::Depth => {
                formatter.write_str("generated model conversion exceeds the maximum nesting depth")
            }
            Self::UnknownProperty(property) => {
                write!(
                    formatter,
                    "generated model cannot represent property {property:?}"
                )
            }
            Self::MissingField(field) => write!(formatter, "missing required field {field}"),
            Self::NoUnionBranch { first: Some(first) } => write!(
                formatter,
                "value does not match any generated model union branch: {first}"
            ),
            Self::NoUnionBranch { first: None } => {
                formatter.write_str("value does not match any generated model union branch")
            }
        }
    }
}

impl ConversionMismatch {
    fn into_pyerr(self) -> PyErr {
        match self {
            Self::Depth => PyErr::new::<PyValueError, _>(self.to_string()),
            _ => PyErr::new::<PyTypeError, _>(self.to_string()),
        }
    }
}

enum ConversionFailure {
    Mismatch(ConversionMismatch),
    Raised(PyErr),
}

impl ConversionFailure {
    #[inline]
    fn into_pyerr(self) -> PyErr {
        match self {
            Self::Mismatch(mismatch) => mismatch.into_pyerr(),
            Self::Raised(error) => error,
        }
    }
}

impl From<PyErr> for ConversionFailure {
    fn from(error: PyErr) -> Self {
        Self::Raised(error)
    }
}

type ConversionResult<T> = Result<T, ConversionFailure>;

#[inline]
fn branch_attempt<T>(
    result: ConversionResult<T>,
    first_mismatch: &mut Option<ConversionMismatch>,
) -> ConversionResult<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(ConversionFailure::Mismatch(error)) => {
            if first_mismatch.is_none() {
                *first_mismatch = Some(error);
            }
            Ok(None)
        }
        Err(ConversionFailure::Raised(error)) => Err(ConversionFailure::Raised(error)),
    }
}

fn no_union_branch_matched(first_mismatch: Option<ConversionMismatch>) -> ConversionFailure {
    ConversionFailure::Mismatch(ConversionMismatch::NoUnionBranch {
        first: first_mismatch.map(Box::new),
    })
}

#[derive(Clone, Copy, Default)]
struct ActiveContainers<'a>(Option<&'a ActiveContainer<'a>>);

struct ActiveContainer<'a> {
    identity: *mut ffi::PyObject,
    parent: ActiveContainers<'a>,
}

impl ActiveContainers<'_> {
    fn with<T>(
        self,
        value: &Bound<'_, PyAny>,
        operation: impl FnOnce(ActiveContainers<'_>) -> ConversionResult<T>,
    ) -> ConversionResult<T> {
        let identity = value.as_ptr();
        let mut ancestor = self.0;
        while let Some(container) = ancestor {
            if container.identity == identity {
                return Err(ConversionFailure::Raised(PyErr::new::<PyValueError, _>(
                    "cyclic containers are not JSON values",
                )));
            }
            ancestor = container.parent.0;
        }
        let container = ActiveContainer {
            identity,
            parent: self,
        };
        operation(ActiveContainers(Some(&container)))
    }

    fn with_pyresult<T>(
        self,
        value: &Bound<'_, PyAny>,
        operation: impl FnOnce(ActiveContainers<'_>) -> PyResult<T>,
    ) -> PyResult<T> {
        let identity = value.as_ptr();
        let mut ancestor = self.0;
        while let Some(container) = ancestor {
            if container.identity == identity {
                return Err(PyErr::new::<PyValueError, _>(
                    "cyclic containers are not JSON values",
                ));
            }
            ancestor = container.parent.0;
        }
        let container = ActiveContainer {
            identity,
            parent: self,
        };
        operation(ActiveContainers(Some(&container)))
    }
}

pub(crate) enum CandidateConstruction<'converter> {
    Constructed(PythonCandidate<'converter>),
    Mismatch(ConversionMismatch),
}

struct UnvalidatedModel(Py<PyAny>);

pub(crate) struct MaterializedJsonValue(Py<PyAny>);

impl MaterializedJsonValue {
    pub(crate) fn into_py(self) -> Py<PyAny> {
        self.0
    }
}

pub(crate) struct PythonCandidate<'converter> {
    converter: &'converter ModelConverterPy,
    model: UnvalidatedModel,
}

pub(crate) struct KwargsCandidate<'converter> {
    converter: &'converter ModelConverterPy,
    model: UnvalidatedModel,
    json_shape: JsonShape,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum JsonShape {
    Proven,
    NeedsValidation,
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

    fn is_valid_python_value(&self, py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<bool> {
        validate_python_value(self.compiled()?, py, value, false)
    }
}

pub(crate) fn validate_python_value(
    schema: &SchemaDocument,
    py: Python<'_>,
    value: &Bound<'_, PyAny>,
    assume_json: bool,
) -> PyResult<bool> {
    let canonical = canonical_python_scalar(py, value)?;
    let validate = |value: &Bound<'_, PyAny>| {
        let instance = JsonInstanceRef::from_python(value);
        if assume_json {
            schema.is_valid_instance_assuming_json(instance)
        } else {
            schema.is_valid_instance(instance)
        }
        .map_err(super::validation_error)
    };
    canonical
        .as_ref()
        .map_or_else(|| validate(value), |value| validate(value.bind(py)))
}

enum ConversionNode {
    Scalar {
        kind: ScalarKind,
    },
    List {
        item: usize,
    },
    Dict {
        value: usize,
    },
    Literal {
        values: Vec<Py<PyAny>>,
    },
    Union(UnionPlan),
    Model {
        model_type: Py<PyType>,
        branch_schema: BranchSchema,
        fields: ModelFields,
        extra: Option<ExtraPropertiesPlan>,
    },
    Root {
        model_type: Py<PyType>,
        branch_schema: BranchSchema,
        value: usize,
        root_attribute: ModelAttribute,
    },
}

pub(crate) struct ModelConverterPlan {
    nodes: Vec<ConversionNode>,
    object_new: Py<PyAny>,
    missing_sentinel: MissingSentinel,
    frozen_list_type: Py<PyType>,
    frozen_dict_type: Py<PyType>,
    frozen_dict_items_attribute: ModelAttribute,
}

pub(crate) struct ModelConverterPy {
    plan: Rc<ModelConverterPlan>,
    root: RootNode,
}

#[derive(Clone, Copy)]
struct RootNode(usize);

pub(crate) struct RootedModelConverterPlan {
    plan: std::rc::Weak<ModelConverterPlan>,
    root: RootNode,
}

impl Deref for ModelConverterPy {
    type Target = ModelConverterPlan;

    fn deref(&self) -> &Self::Target {
        &self.plan
    }
}

impl PythonCandidate<'_> {
    pub(crate) fn validate(
        self,
        py: Python<'_>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<Option<Py<PyAny>>> {
        if !validate_python_value(self.converter.schema()?, py, value, true)? {
            return Ok(None);
        }
        Ok(Some(self.model.finish()))
    }
}

impl KwargsCandidate<'_> {
    pub(crate) fn finish_unchecked(self) -> Py<PyAny> {
        self.model.finish()
    }

    pub(crate) fn validate(self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        let projection = self.converter.projection();
        let projected = projection.instance(self.model.0.bind(py));
        let is_valid = match self.json_shape {
            JsonShape::Proven => self
                .converter
                .schema()?
                .is_valid_instance_assuming_json(projected),
            JsonShape::NeedsValidation => self.converter.schema()?.is_valid_instance(projected),
        }
        .map_err(super::validation_error)?;
        if !is_valid {
            return Ok(None);
        }
        Ok(Some(self.model.finish()))
    }
}

impl UnvalidatedModel {
    fn finish(self) -> Py<PyAny> {
        self.0
    }
}

type FrozenDictPair = (Py<PyAny>, Py<PyAny>);

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
            return converter.freeze_dict_pairs(py, self.entries);
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

    fn finish(self) -> Vec<FrozenDictPair> {
        self.entries
    }
}

impl ModelConverterPlan {
    pub(crate) fn traverse(&self, visit: &PyVisit<'_>) -> Result<(), PyTraverseError> {
        visit.call(&self.object_new)?;
        visit.call(&self.missing_sentinel.0)?;
        visit.call(&self.frozen_list_type)?;
        visit.call(&self.frozen_dict_type)?;
        self.frozen_dict_items_attribute.traverse(visit)?;
        for node in &self.nodes {
            match node {
                ConversionNode::Scalar { .. } => {}
                ConversionNode::Literal { values, .. } => {
                    for value in values {
                        visit.call(value)?;
                    }
                }
                ConversionNode::Model {
                    model_type,
                    fields,
                    extra,
                    ..
                } => {
                    visit.call(model_type)?;
                    for field in fields {
                        field.attribute.traverse(visit)?;
                    }
                    if let Some(extra) = extra {
                        extra.attribute.traverse(visit)?;
                    }
                }
                ConversionNode::Root {
                    model_type,
                    root_attribute,
                    ..
                } => {
                    visit.call(model_type)?;
                    root_attribute.traverse(visit)?;
                }
                ConversionNode::List { .. }
                | ConversionNode::Dict { .. }
                | ConversionNode::Union(_) => {}
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
    pub(crate) fn model_type(&self) -> PyResult<&Py<PyType>> {
        match self.nodes.get(self.root.0) {
            Some(ConversionNode::Model { model_type, .. })
            | Some(ConversionNode::Root { model_type, .. }) => Ok(model_type),
            _ => Err(PyErr::new::<PyIndexError, _>(
                "model converter root is not a generated model",
            )),
        }
    }

    pub(crate) fn construct_unchecked(
        &self,
        py: Python<'_>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<Py<PyAny>> {
        let active_containers = ActiveContainers::default();
        let instance = self
            .convert(
                py,
                self.root.0,
                value,
                UnionSelection::FirstRepresentable,
                MAX_MODEL_DEPTH,
                active_containers,
            )
            .map_err(ConversionFailure::into_pyerr)?;
        Ok(UnvalidatedModel(instance).finish())
    }

    pub(crate) fn construct_candidate(
        &self,
        py: Python<'_>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<CandidateConstruction<'_>> {
        let active_containers = ActiveContainers::default();
        match self.convert(
            py,
            self.root.0,
            value,
            UnionSelection::ValidateAmbiguousBranches,
            MAX_MODEL_DEPTH,
            active_containers,
        ) {
            Ok(instance) => Ok(CandidateConstruction::Constructed(PythonCandidate {
                converter: self,
                model: UnvalidatedModel(instance),
            })),
            Err(ConversionFailure::Mismatch(error)) => Ok(CandidateConstruction::Mismatch(error)),
            Err(ConversionFailure::Raised(error)) => Err(error),
        }
    }
}

impl RootedModelConverterPlan {
    pub(crate) fn upgrade(&self) -> Option<ModelConverterPy> {
        self.plan.upgrade().map(|plan| ModelConverterPy {
            plan,
            root: self.root,
        })
    }
}

impl ModelConverterPy {
    pub(crate) fn schema(&self) -> PyResult<&SchemaDocument> {
        match self.nodes.get(self.root.0) {
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

    pub(crate) fn construct_kwargs_candidate(
        &self,
        py: Python<'_>,
        kwargs: &Bound<'_, PyDict>,
    ) -> PyResult<KwargsCandidate<'_>> {
        let mut json_shape = JsonShape::Proven;
        let active_containers = ActiveContainers::default();
        let node = self.nodes.get(self.root.0).ok_or_else(|| {
            PyErr::new::<PyIndexError, _>(format!(
                "model converter root node {} is missing",
                self.root.0
            ))
        })?;
        match node {
            ConversionNode::Model {
                model_type,
                fields,
                extra,
                ..
            } => self
                .convert_model_kwargs(
                    py,
                    model_type,
                    fields,
                    extra.as_ref(),
                    kwargs,
                    &mut json_shape,
                    active_containers,
                )
                .map(|instance| KwargsCandidate {
                    converter: self,
                    model: UnvalidatedModel(instance),
                    json_shape,
                }),
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
                    &mut json_shape,
                    active_containers,
                )
                .map(|instance| KwargsCandidate {
                    converter: self,
                    model: UnvalidatedModel(instance),
                    json_shape,
                }),
            _ => Err(PyErr::new::<PyTypeError, _>(
                "model converter root must be a generated model",
            )),
        }
    }

    pub(crate) fn is_valid_raw_python(&self, value: &Bound<'_, PyAny>) -> PyResult<bool> {
        self.schema()?
            .is_valid_instance(JsonInstanceRef::from_python(value))
            .map_err(super::validation_error)
    }

    pub(crate) fn validate_json_value(
        &self,
        py: Python<'_>,
        value: &MaterializedJsonValue,
    ) -> PyResult<bool> {
        self.schema()?
            .is_valid_instance_assuming_json(JsonInstanceRef::from_python(value.0.bind(py)))
            .map_err(super::validation_error)
    }

    pub(crate) fn construct_jiter_unchecked(
        &self,
        py: Python<'_>,
        value: &JiterJsonValue<'_>,
    ) -> PyResult<Py<PyAny>> {
        let instance = self
            .convert_jiter(py, self.root.0, value, UnionSelection::FirstRepresentable)
            .map_err(ConversionFailure::into_pyerr)?;
        Ok(UnvalidatedModel(instance).finish())
    }

    pub(crate) fn construct_jiter_checked(
        &self,
        py: Python<'_>,
        value: &JiterJsonValue<'_>,
    ) -> PyResult<Option<Py<PyAny>>> {
        let is_valid = self
            .schema()?
            .is_valid_instance_assuming_json(JsonInstanceRef::from_jiter(value))
            .map_err(super::validation_error)?;
        if !is_valid {
            return Ok(None);
        }
        let instance = self
            .convert_jiter(
                py,
                self.root.0,
                value,
                UnionSelection::ValidateAmbiguousBranches,
            )
            .map_err(ConversionFailure::into_pyerr)?;
        Ok(Some(UnvalidatedModel(instance).finish()))
    }

    pub(crate) fn serialize_json_value(
        &self,
        py: Python<'_>,
        value: &MaterializedJsonValue,
    ) -> PyResult<String> {
        let mut output = Vec::with_capacity(256);
        write_serializable_json_value(&mut output, value.0.bind(py), MAX_MODEL_DEPTH)?;
        String::from_utf8(output).map_err(|error| {
            PyErr::new::<PyValueError, _>(format!(
                "JSON serialization produced invalid UTF-8: {error}"
            ))
        })
    }

    pub(crate) fn serialize_model_trusted(
        &self,
        py: Python<'_>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<String> {
        let mut output = Vec::with_capacity(256);
        self.write_json_node(
            py,
            self.root.0,
            value,
            MAX_MODEL_DEPTH,
            ActiveContainers::default(),
            &mut output,
        )?;
        String::from_utf8(output).map_err(|error| {
            PyErr::new::<PyValueError, _>(format!(
                "JSON serialization produced invalid UTF-8: {error}"
            ))
        })
    }

    pub(crate) fn materialize_json_value(
        &self,
        py: Python<'_>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<MaterializedJsonValue> {
        self.to_python_value_node(py, self.root.0, value, MAX_MODEL_DEPTH)
            .map(MaterializedJsonValue)
    }

    fn convert(
        &self,
        py: Python<'_>,
        node_id: usize,
        value: &Bound<'_, PyAny>,
        union_selection: UnionSelection,
        remaining_depth: u16,
        active_containers: ActiveContainers<'_>,
    ) -> ConversionResult<Py<PyAny>> {
        if remaining_depth == 0 {
            return Err(ConversionFailure::Mismatch(ConversionMismatch::Depth));
        }
        let node = self.nodes.get(node_id).ok_or_else(|| {
            PyErr::new::<PyIndexError, _>(format!("model converter node {node_id} is missing"))
        })?;
        if matches!(
            node,
            ConversionNode::List { .. }
                | ConversionNode::Dict { .. }
                | ConversionNode::Model { .. }
        ) {
            return active_containers.with(value, |active_containers| {
                self.convert_node(
                    py,
                    node,
                    value,
                    union_selection,
                    remaining_depth,
                    active_containers,
                )
            });
        }
        self.convert_node(
            py,
            node,
            value,
            union_selection,
            remaining_depth,
            active_containers,
        )
    }

    fn convert_node(
        &self,
        py: Python<'_>,
        node: &ConversionNode,
        value: &Bound<'_, PyAny>,
        union_selection: UnionSelection,
        remaining_depth: u16,
        active_containers: ActiveContainers<'_>,
    ) -> ConversionResult<Py<PyAny>> {
        match node {
            ConversionNode::Scalar { kind } => {
                if matches!(kind, ScalarKind::Any) {
                    self.freeze_python_json_value(py, value, remaining_depth, active_containers)
                } else {
                    convert_scalar(py, *kind, value)
                }
            }
            ConversionNode::List { item } => self.convert_list(
                py,
                *item,
                value,
                union_selection,
                remaining_depth,
                active_containers,
            ),
            ConversionNode::Dict { value: value_node } => self.convert_dict(
                py,
                *value_node,
                value,
                union_selection,
                remaining_depth,
                active_containers,
            ),
            ConversionNode::Literal { values, .. } => convert_literal(py, values, value),
            ConversionNode::Union(plan) => self.convert_union(
                py,
                plan,
                value,
                union_selection,
                remaining_depth,
                active_containers,
            ),
            ConversionNode::Model {
                model_type,
                fields,
                extra,
                ..
            } => self.convert_model(
                py,
                model_type,
                fields,
                extra.as_ref(),
                value,
                union_selection,
                remaining_depth,
                active_containers,
            ),
            ConversionNode::Root {
                model_type,
                value: value_node,
                root_attribute,
                ..
            } => {
                let converted = self.convert(
                    py,
                    *value_node,
                    value,
                    union_selection,
                    remaining_depth - 1,
                    active_containers,
                )?;
                let instance = allocate_model(py, model_type, &self.object_new)?;
                root_attribute.set(py, &instance, &converted)?;
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
        json_shape: &mut JsonShape,
        active_containers: ActiveContainers<'_>,
    ) -> ConversionResult<Py<PyAny>> {
        if remaining_depth == 0 {
            return Err(ConversionFailure::Mismatch(ConversionMismatch::Depth));
        }
        let node = self.nodes.get(node_id).ok_or_else(|| {
            PyErr::new::<PyIndexError, _>(format!("model converter node {node_id} is missing"))
        })?;
        if matches!(
            node,
            ConversionNode::List { .. } | ConversionNode::Dict { .. }
        ) {
            return active_containers.with(value, |active_containers| {
                self.convert_direct_node(
                    py,
                    node,
                    value,
                    remaining_depth,
                    json_shape,
                    active_containers,
                )
            });
        }
        self.convert_direct_node(
            py,
            node,
            value,
            remaining_depth,
            json_shape,
            active_containers,
        )
    }

    fn convert_direct_node(
        &self,
        py: Python<'_>,
        node: &ConversionNode,
        value: &Bound<'_, PyAny>,
        remaining_depth: u16,
        json_shape: &mut JsonShape,
        active_containers: ActiveContainers<'_>,
    ) -> ConversionResult<Py<PyAny>> {
        match node {
            ConversionNode::Scalar { kind } => {
                if matches!(kind, ScalarKind::Any) {
                    self.freeze_python_json_value(py, value, remaining_depth, active_containers)
                } else {
                    convert_direct_scalar(py, *kind, value)
                }
            }
            ConversionNode::List { item } => {
                if value.is_instance(self.frozen_dict_type.bind(py))? {
                    return Err(ConversionFailure::Mismatch(expected_type_mismatch(
                        "sequence", value,
                    )?));
                }
                let is_builtin_sequence =
                    value.cast::<PyList>().is_ok() || value.cast::<PyTuple>().is_ok();
                if !is_builtin_sequence
                    && (value.is_instance_of::<PyString>()
                        || value.is_instance_of::<PyBytes>()
                        || value.cast::<PyMapping>().is_ok()
                        || value.cast::<PySequence>().is_err())
                {
                    return Err(ConversionFailure::Mismatch(expected_type_mismatch(
                        "sequence", value,
                    )?));
                }
                let mut output = Vec::new();
                if let Ok(input) = value.cast::<PyList>() {
                    for item_value in input {
                        output.push(self.convert_direct(
                            py,
                            *item,
                            &item_value,
                            remaining_depth - 1,
                            json_shape,
                            active_containers,
                        )?);
                    }
                } else if let Ok(input) = value.cast::<PyTuple>() {
                    for item_value in input {
                        output.push(self.convert_direct(
                            py,
                            *item,
                            &item_value,
                            remaining_depth - 1,
                            json_shape,
                            active_containers,
                        )?);
                    }
                } else {
                    let Ok(input) = value.cast::<PySequence>() else {
                        return Err(ConversionFailure::Mismatch(expected_type_mismatch(
                            "sequence", value,
                        )?));
                    };
                    for item_value in input.try_iter()? {
                        output.push(self.convert_direct(
                            py,
                            *item,
                            &item_value?,
                            remaining_depth - 1,
                            json_shape,
                            active_containers,
                        )?);
                    }
                }
                self.freeze_list(py, output)
                    .map_err(ConversionFailure::Raised)
            }
            ConversionNode::Dict { value: value_node } => {
                if value.cast::<PyDict>().is_err() && value.cast::<PyMapping>().is_err() {
                    return Err(ConversionFailure::Mismatch(expected_type_mismatch(
                        "mapping", value,
                    )?));
                }
                if let Ok(input) = value.cast::<PyDict>() {
                    let mut output = PythonFrozenDictBuilder::with_capacity(input.len());
                    for (key_value, item_value) in input {
                        let converted_key = canonical_json_object_key(py, &key_value)?;
                        let converted_value = self.convert_direct(
                            py,
                            *value_node,
                            &item_value,
                            remaining_depth - 1,
                            json_shape,
                            active_containers,
                        )?;
                        output.push(&key_value, converted_key, converted_value);
                    }
                    return output.finish(py, self).map_err(ConversionFailure::Raised);
                }

                let Ok(input) = value.cast::<PyMapping>() else {
                    return Err(ConversionFailure::Mismatch(expected_type_mismatch(
                        "mapping", value,
                    )?));
                };
                let output = NormalizingFrozenDictBuilder::new(py);
                for entry in input.items()? {
                    let (key_value, item_value) = mapping_pair(&entry)?;
                    let converted_key = canonical_json_object_key(py, &key_value)?;
                    let converted_value = self.convert_direct(
                        py,
                        *value_node,
                        &item_value,
                        remaining_depth - 1,
                        json_shape,
                        active_containers,
                    )?;
                    output.insert(py, converted_key, converted_value)?;
                }
                output.finish(py, self).map_err(ConversionFailure::Raised)
            }
            ConversionNode::Literal { values, .. } => convert_literal(py, values, value),
            ConversionNode::Union(plan) => {
                let mut first_error = None;
                for branch in plan {
                    if let Some(converted) = branch_attempt(
                        self.convert_direct(
                            py,
                            *branch,
                            value,
                            remaining_depth - 1,
                            json_shape,
                            active_containers,
                        ),
                        &mut first_error,
                    )? {
                        return Ok(converted);
                    }
                }
                Err(no_union_branch_matched(first_error))
            }
            ConversionNode::Model { model_type, .. } | ConversionNode::Root { model_type, .. } => {
                if value.is_instance(model_type.bind(py))? {
                    *json_shape = JsonShape::NeedsValidation;
                    Ok(value.clone().unbind())
                } else {
                    let expected = model_type.bind(py).name()?.to_str()?.to_owned();
                    Err(ConversionFailure::Mismatch(expected_type_mismatch(
                        &expected, value,
                    )?))
                }
            }
        }
    }

    fn convert_missing_field_value(
        &self,
        py: Python<'_>,
        field: &FieldPlan,
    ) -> Result<Py<PyAny>, ConversionMismatch> {
        if field.presence.is_omittable() {
            return Ok(self.missing_sentinel.clone_ref(py));
        }
        Err(ConversionMismatch::MissingField(field.json_name.clone()))
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
        self.freeze_dict_pairs(
            py,
            items
                .into_iter()
                .map(|(key, value)| (key.unbind(), value.unbind()))
                .collect(),
        )
    }

    #[inline]
    fn freeze_dict_pairs(&self, py: Python<'_>, items: Vec<FrozenDictPair>) -> PyResult<Py<PyAny>> {
        let mut pairs = Vec::with_capacity(items.len());
        for (key, value) in items {
            pairs.push(PyTuple::new(py, [key, value])?.into_any().unbind());
        }
        let frozen_items = PyTuple::new(py, pairs)?.into_any().unbind();
        let instance = allocate_model(py, &self.frozen_dict_type, &self.object_new)?;
        self.frozen_dict_items_attribute
            .set(py, &instance, &frozen_items)?;
        Ok(instance.unbind())
    }

    fn frozen_dict_items<'py>(&self, value: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyTuple>> {
        self.frozen_dict_items_attribute
            .get(value)?
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
        active_containers: ActiveContainers<'_>,
    ) -> ConversionResult<Py<PyAny>> {
        if remaining_depth == 0 {
            return Err(ConversionFailure::Mismatch(ConversionMismatch::Depth));
        }
        if let Ok(items) = value.cast::<PyList>() {
            return active_containers.with(value, |active_containers| {
                let output = items
                    .iter()
                    .map(|item| {
                        self.freeze_python_json_value(
                            py,
                            &item,
                            remaining_depth - 1,
                            active_containers,
                        )
                    })
                    .collect::<ConversionResult<Vec<_>>>()?;
                self.freeze_list(py, output)
                    .map_err(ConversionFailure::Raised)
            });
        }
        if let Ok(properties) = value.cast::<PyDict>() {
            return active_containers.with(value, |active_containers| {
                let mut output = PythonFrozenDictBuilder::with_capacity(properties.len());
                for (source_key, item) in properties {
                    let key = canonical_json_object_key(py, &source_key)?;
                    let value = self.freeze_python_json_value(
                        py,
                        &item,
                        remaining_depth - 1,
                        active_containers,
                    )?;
                    output.push(&source_key, key, value);
                }
                output.finish(py, self).map_err(ConversionFailure::Raised)
            });
        }
        if let Ok(items) = value.cast::<PyTuple>() {
            return active_containers.with(value, |active_containers| {
                let output = items
                    .iter()
                    .map(|item| {
                        self.freeze_python_json_value(
                            py,
                            &item,
                            remaining_depth - 1,
                            active_containers,
                        )
                    })
                    .collect::<ConversionResult<Vec<_>>>()?;
                self.freeze_list(py, output)
                    .map_err(ConversionFailure::Raised)
            });
        }
        if let Some(value) = canonical_python_scalar(py, value)? {
            return Ok(value);
        }
        Err(ConversionFailure::Mismatch(expected_type_mismatch(
            "a JSON-compatible value",
            value,
        )?))
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
                self.freeze_dict_pairs(py, output.finish())
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
        union_selection: UnionSelection,
        remaining_depth: u16,
        active_containers: ActiveContainers<'_>,
    ) -> ConversionResult<Py<PyAny>> {
        if value.is_instance(self.frozen_dict_type.bind(py))? {
            return Err(ConversionFailure::Mismatch(expected_type_mismatch(
                "sequence", value,
            )?));
        }
        if value.cast::<PyList>().is_err() && value.cast::<PyTuple>().is_err() {
            return Err(ConversionFailure::Mismatch(expected_type_mismatch(
                "list", value,
            )?));
        }
        let mut output = Vec::new();
        if let Ok(items) = value.cast::<PyList>() {
            for item in items {
                let converted = self.convert(
                    py,
                    item_node,
                    &item,
                    union_selection,
                    remaining_depth - 1,
                    active_containers,
                )?;
                output.push(converted);
            }
        } else if let Ok(items) = value.cast::<PyTuple>() {
            for item in items {
                let converted = self.convert(
                    py,
                    item_node,
                    &item,
                    union_selection,
                    remaining_depth - 1,
                    active_containers,
                )?;
                output.push(converted);
            }
        }
        self.freeze_list(py, output)
            .map_err(ConversionFailure::Raised)
    }

    #[allow(clippy::too_many_arguments)]
    fn convert_dict(
        &self,
        py: Python<'_>,
        value_node: usize,
        value: &Bound<'_, PyAny>,
        union_selection: UnionSelection,
        remaining_depth: u16,
        active_containers: ActiveContainers<'_>,
    ) -> ConversionResult<Py<PyAny>> {
        if let Ok(input) = value.cast::<PyDict>() {
            let mut output = PythonFrozenDictBuilder::with_capacity(input.len());
            for (key, item) in input {
                let converted_key = canonical_json_object_key(py, &key)?;
                let converted_value = self.convert(
                    py,
                    value_node,
                    &item,
                    union_selection,
                    remaining_depth - 1,
                    active_containers,
                )?;
                output.push(&key, converted_key, converted_value);
            }
            return output.finish(py, self).map_err(ConversionFailure::Raised);
        }

        Err(ConversionFailure::Mismatch(expected_type_mismatch(
            "dict", value,
        )?))
    }

    #[allow(clippy::too_many_arguments)]
    fn convert_union(
        &self,
        py: Python<'_>,
        plan: &UnionPlan,
        value: &Bound<'_, PyAny>,
        union_selection: UnionSelection,
        remaining_depth: u16,
        active_containers: ActiveContainers<'_>,
    ) -> ConversionResult<Py<PyAny>> {
        if let (Some(discriminator_name), Ok(object)) =
            (plan.discriminator_name(), value.cast::<PyDict>())
            && let Some(tag) = object.get_item(discriminator_name)?
            && let Some(tag) = python_discriminator_key(&tag)
            && let Some(branch) = plan.discriminated_branch(&tag)
        {
            return self.convert(
                py,
                branch,
                value,
                union_selection,
                remaining_depth - 1,
                active_containers,
            );
        }

        let mut matching_branches = Vec::new();
        for branch in plan {
            if self.node_matches_kind(py, *branch, value)? {
                matching_branches.push(*branch);
            }
        }
        if matching_branches.len() == 1 {
            return self.convert(
                py,
                matching_branches[0],
                value,
                union_selection,
                remaining_depth - 1,
                active_containers,
            );
        }

        let mut first_error = None;
        let candidate_branches = if matching_branches.is_empty() {
            &plan[..]
        } else {
            matching_branches.as_slice()
        };
        for branch in candidate_branches {
            let converted = branch_attempt(
                self.convert(
                    py,
                    *branch,
                    value,
                    union_selection,
                    remaining_depth - 1,
                    active_containers,
                ),
                &mut first_error,
            )?;
            let Some(converted) = converted else {
                continue;
            };
            if matching_branches.len() > 1
                && union_selection == UnionSelection::ValidateAmbiguousBranches
                && !self.node_can_represent_python_value(py, *branch, value, remaining_depth - 1)?
            {
                continue;
            }
            return Ok(converted);
        }
        Err(no_union_branch_matched(first_error))
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
                ScalarKind::String => value.is_instance_of::<PyString>(),
                ScalarKind::Integer => {
                    (value.is_instance_of::<PyInt>() && !value.is_instance_of::<PyBool>())
                        || (value.is_instance_of::<PyFloat>()
                            && value.extract::<f64>()?.fract() == 0.0)
                }
                ScalarKind::Number => {
                    !value.is_instance_of::<PyBool>()
                        && (value.is_instance_of::<PyInt>()
                            || (value.is_instance_of::<PyFloat>()
                                && value.extract::<f64>()?.is_finite()))
                }
                ScalarKind::Boolean => value.is_instance_of::<PyBool>(),
                ScalarKind::Null => value.is_none(),
            },
            ConversionNode::List { .. } => is_python_json_array(value),
            ConversionNode::Dict { .. } | ConversionNode::Model { .. } => {
                value.is_instance_of::<PyDict>()
            }
            ConversionNode::Literal { values, .. } => {
                matching_literal_index(py, values, value)?.is_some()
            }
            ConversionNode::Union(plan) => {
                let mut matches = false;
                for branch in plan {
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
                if let Ok(values) = value.cast::<PyList>() {
                    for item_value in values {
                        if !self.node_can_represent_python_value(
                            py,
                            *item,
                            &item_value,
                            remaining_depth - 1,
                        )? {
                            return Ok(false);
                        }
                    }
                    return Ok(true);
                }
                let Ok(values) = value.cast::<PyTuple>() else {
                    return Ok(false);
                };
                for item_value in values {
                    if !self.node_can_represent_python_value(
                        py,
                        *item,
                        &item_value,
                        remaining_depth - 1,
                    )? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            ConversionNode::Dict { value: value_node } => {
                let Ok(values) = value.cast::<PyDict>() else {
                    return Ok(false);
                };
                for (key_value, item_value) in values {
                    if !key_value.is_instance_of::<PyString>()
                        || !self.node_can_represent_python_value(
                            py,
                            *value_node,
                            &item_value,
                            remaining_depth - 1,
                        )?
                    {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            ConversionNode::Union(plan) => {
                for branch in plan {
                    if self.node_can_represent_python_value(
                        py,
                        *branch,
                        value,
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
                extra,
                ..
            } => {
                let Ok(values) = value.cast::<PyDict>() else {
                    return Ok(false);
                };
                if !branch_schema.is_valid_instance(JsonInstanceRef::from_python(value))? {
                    return Ok(false);
                }
                for (key, item_value) in values {
                    let Ok(key) = key.extract::<String>() else {
                        return Ok(false);
                    };
                    let child = fields
                        .by_json_name(&key)
                        .map(|field| field.value_node)
                        .or_else(|| extra.as_ref().map(|extra| extra.value_node));
                    let Some(child) = child else {
                        return Ok(false);
                    };
                    if !self.node_can_represent_python_value(
                        py,
                        child,
                        &item_value,
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
                if !branch_schema.is_valid_python_value(py, value)? {
                    return Ok(false);
                }
                self.node_can_represent_python_value(py, *child, value, remaining_depth - 1)
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn convert_model(
        &self,
        py: Python<'_>,
        model_type: &Py<PyType>,
        fields: &ModelFields,
        extra_plan: Option<&ExtraPropertiesPlan>,
        value: &Bound<'_, PyAny>,
        union_selection: UnionSelection,
        remaining_depth: u16,
        active_containers: ActiveContainers<'_>,
    ) -> ConversionResult<Py<PyAny>> {
        let Ok(input) = value.cast::<PyDict>() else {
            return Err(ConversionFailure::Mismatch(expected_type_mismatch(
                "JSON object",
                value,
            )?));
        };
        let instance = allocate_model(py, model_type, &self.object_new)?;
        let mut extra_output =
            extra_plan.map(|_| PythonFrozenDictBuilder::with_capacity(input.len()));
        let mut present_fields = 0;
        let mut normalization_required = false;

        for (key, item) in input {
            let key = key
                .cast::<PyString>()
                .map_err(|_| ConversionFailure::Mismatch(ConversionMismatch::JsonObjectKey))?;
            normalization_required |= !key.is_exact_instance_of::<PyString>();
            let key_string = key.to_str()?;
            if let Some(field) = fields.by_json_name(key_string) {
                let already_present =
                    normalization_required && field.attribute.is_set(py, &instance)?;
                let converted = self.convert(
                    py,
                    field.value_node,
                    &item,
                    union_selection,
                    remaining_depth - 1,
                    active_containers,
                )?;
                field.attribute.set(py, &instance, &converted)?;
                if !already_present {
                    present_fields += 1;
                }
            } else if let (Some(extra), Some(output)) = (extra_plan, extra_output.as_mut()) {
                let converted = self.convert(
                    py,
                    extra.value_node,
                    &item,
                    union_selection,
                    remaining_depth - 1,
                    active_containers,
                )?;
                output.push(
                    key.as_any(),
                    PyString::new(py, key_string).into_any().unbind(),
                    converted,
                );
            } else {
                return Err(ConversionFailure::Mismatch(
                    ConversionMismatch::UnknownProperty(key_string.to_owned()),
                ));
            }
        }

        if present_fields != fields.len() {
            for field in fields {
                if !field.attribute.is_set(py, &instance)? {
                    let converted = self
                        .convert_missing_field_value(py, field)
                        .map_err(ConversionFailure::Mismatch)?;
                    field.attribute.set(py, &instance, &converted)?;
                }
            }
        }

        let extra_value = extra_output
            .map(|output| output.finish(py, self))
            .transpose()
            .map_err(ConversionFailure::Raised)?;

        if let (Some(plan), Some(extra_value)) = (extra_plan, extra_value.as_ref()) {
            plan.attribute.set(py, &instance, extra_value)?;
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
        json_shape: &mut JsonShape,
        active_containers: ActiveContainers<'_>,
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
        let converted = self
            .convert_direct(
                py,
                value_node,
                &raw,
                MAX_MODEL_DEPTH - 1,
                json_shape,
                active_containers,
            )
            .map_err(ConversionFailure::into_pyerr)?;
        let instance = allocate_model(py, model_type, &self.object_new)?;
        root_attribute.set(py, &instance, &converted)?;
        Ok(instance.unbind())
    }

    #[allow(clippy::too_many_arguments)]
    fn convert_model_kwargs(
        &self,
        py: Python<'_>,
        model_type: &Py<PyType>,
        fields: &ModelFields,
        extra: Option<&ExtraPropertiesPlan>,
        kwargs: &Bound<'_, PyDict>,
        json_shape: &mut JsonShape,
        active_containers: ActiveContainers<'_>,
    ) -> PyResult<Py<PyAny>> {
        let instance = allocate_model(py, model_type, &self.object_new)?;
        let mut extra_input = None;
        let mut present_fields = 0;

        for (key, value) in kwargs {
            let key = key.extract::<String>().map_err(|_| {
                PyErr::new::<PyTypeError, _>("generated model keyword names must be strings")
            })?;
            if key == "__jsoncompat_extra__" {
                if extra.is_none() {
                    return Err(unexpected_keyword(py, model_type, &key));
                }
                extra_input = Some(value);
                continue;
            }
            let Some(field) = fields.by_py_name(&key) else {
                return Err(unexpected_keyword(py, model_type, &key));
            };
            if field.presence.is_omittable() && value.is(self.missing_sentinel.bind(py)) {
                field
                    .attribute
                    .set(py, &instance, &self.missing_sentinel.0)?;
                present_fields += 1;
                continue;
            }
            let converted = self
                .convert_direct(
                    py,
                    field.value_node,
                    &value,
                    MAX_MODEL_DEPTH - 1,
                    json_shape,
                    active_containers,
                )
                .map_err(ConversionFailure::into_pyerr)?;
            field.attribute.set(py, &instance, &converted)?;
            present_fields += 1;
        }

        if present_fields != fields.len() {
            for field in fields {
                if !field.attribute.is_set(py, &instance)? {
                    let converted = self
                        .convert_missing_field_value(py, field)
                        .map_err(ConversionMismatch::into_pyerr)?;
                    field.attribute.set(py, &instance, &converted)?;
                }
            }
        }

        if let Some(extra_plan) = extra {
            let extra_value = if let Some(extra_input) = extra_input {
                active_containers
                    .with(&extra_input, |active_containers| {
                        if let Ok(extra_input) = extra_input.cast::<PyDict>() {
                            let mut output =
                                PythonFrozenDictBuilder::with_capacity(extra_input.len());
                            for (key, value) in extra_input {
                                let key_string = key.extract::<String>().map_err(|_| {
                                    PyErr::new::<PyTypeError, _>(
                                        "JSON object keys must be strings",
                                    )
                                })?;
                                if fields.by_json_name(&key_string).is_some() {
                                    return Err(ConversionFailure::Raised(PyErr::new::<
                                        PyValueError,
                                        _,
                                    >(
                                        format!(
                                            "additional property {key_string:?} collides with a declared field"
                                        ),
                                    )));
                                }
                                let converted = self.convert_direct(
                                    py,
                                    extra_plan.value_node,
                                    &value,
                                    MAX_MODEL_DEPTH - 1,
                                    json_shape,
                                    active_containers,
                                )?;
                                output.push(
                                    &key,
                                    PyString::new(py, &key_string).into_any().unbind(),
                                    converted,
                                );
                            }
                            output.finish(py, self).map_err(ConversionFailure::Raised)
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
                                    PyErr::new::<PyTypeError, _>(
                                        "JSON object keys must be strings",
                                    )
                                })?;
                                if fields.by_json_name(&key_string).is_some() {
                                    return Err(ConversionFailure::Raised(PyErr::new::<
                                        PyValueError,
                                        _,
                                    >(
                                        format!(
                                            "additional property {key_string:?} collides with a declared field"
                                        ),
                                    )));
                                }
                                let converted = self.convert_direct(
                                    py,
                                    extra_plan.value_node,
                                    &value,
                                    MAX_MODEL_DEPTH - 1,
                                    json_shape,
                                    active_containers,
                                )?;
                                output.insert(
                                    py,
                                    PyString::new(py, &key_string).into_any().unbind(),
                                    converted,
                                )?;
                            }
                            output.finish(py, self).map_err(ConversionFailure::Raised)
                        }
                    })
                    .map_err(ConversionFailure::into_pyerr)?
            } else {
                self.freeze_dict_pairs(py, Vec::new())?
            };
            extra_plan.attribute.set(py, &instance, &extra_value)?;
        }

        Ok(instance.unbind())
    }

    fn convert_jiter(
        &self,
        py: Python<'_>,
        node_id: usize,
        value: &JiterJsonValue<'_>,
        union_selection: UnionSelection,
    ) -> ConversionResult<Py<PyAny>> {
        let node = self.nodes.get(node_id).ok_or_else(|| {
            PyErr::new::<PyIndexError, _>(format!("model converter node {node_id} is missing"))
        })?;
        match node {
            ConversionNode::Scalar { kind, .. } => {
                if matches!(kind, ScalarKind::Any) {
                    self.freeze_jiter_json_value(py, value, MAX_MODEL_DEPTH)
                        .map_err(ConversionFailure::Raised)
                } else {
                    convert_jiter_scalar_value(py, *kind, value)
                }
            }
            ConversionNode::Literal { values } => {
                let python_value = value.into_pyobject(py)?.unbind();
                convert_literal(py, values, python_value.bind(py))
            }
            ConversionNode::List { item } => {
                let JiterJsonValue::Array(items) = value else {
                    return Err(ConversionFailure::Mismatch(
                        ConversionMismatch::ExpectedType {
                            expected: "list".to_owned(),
                            actual: None,
                        },
                    ));
                };
                let mut converted = Vec::with_capacity(items.len());
                for item_value in items.iter() {
                    converted.push(self.convert_jiter(py, *item, item_value, union_selection)?);
                }
                self.freeze_list(py, converted)
                    .map_err(ConversionFailure::Raised)
            }
            ConversionNode::Dict { value: value_node } => {
                let JiterJsonValue::Object(entries) = value else {
                    return Err(ConversionFailure::Mismatch(
                        ConversionMismatch::ExpectedType {
                            expected: "dict".to_owned(),
                            actual: None,
                        },
                    ));
                };
                let mut output = JiterFrozenDictBuilder::with_capacity(entries.len());
                for (key_value, item) in entries.iter() {
                    let converted_key = PyString::new(py, key_value.as_ref()).into_any().unbind();
                    let converted_value =
                        self.convert_jiter(py, *value_node, item, union_selection)?;
                    output.push(key_value.as_ref(), converted_key, converted_value)?;
                }
                self.freeze_dict_pairs(py, output.finish())
                    .map_err(ConversionFailure::Raised)
            }
            ConversionNode::Union(plan) => {
                self.convert_jiter_union_value(py, plan, value, union_selection)
            }
            ConversionNode::Model {
                model_type,
                fields,
                extra,
                ..
            } => self.convert_jiter_model_value(
                py,
                model_type,
                fields,
                extra.as_ref(),
                value,
                union_selection,
            ),
            ConversionNode::Root {
                model_type,
                value: value_node,
                root_attribute,
                ..
            } => {
                let converted = self.convert_jiter(py, *value_node, value, union_selection)?;
                let instance = allocate_model(py, model_type, &self.object_new)?;
                root_attribute.set(py, &instance, &converted)?;
                Ok(instance.unbind())
            }
        }
    }

    fn convert_jiter_union_value(
        &self,
        py: Python<'_>,
        plan: &UnionPlan,
        value: &JiterJsonValue<'_>,
        union_selection: UnionSelection,
    ) -> ConversionResult<Py<PyAny>> {
        if let (Some(discriminator_name), JiterJsonValue::Object(entries)) =
            (plan.discriminator_name(), value)
            && let Some((_, tag)) = entries
                .iter()
                .find(|(key, _)| key.as_ref() == discriminator_name)
            && let Some(tag) = jiter_discriminator_key(tag)
            && let Some(branch) = plan.discriminated_branch(&tag)
        {
            return self.convert_jiter(py, branch, value, union_selection);
        }

        let mut matching_branches = Vec::new();
        for branch in plan {
            if self.jiter_node_matches_kind(*branch, value) {
                matching_branches.push(*branch);
            }
        }
        if matching_branches.len() == 1 {
            return self.convert_jiter(py, matching_branches[0], value, union_selection);
        }
        let mut first_error = None;
        let candidate_branches = if matching_branches.is_empty() {
            &plan[..]
        } else {
            matching_branches.as_slice()
        };
        for branch in candidate_branches {
            if matching_branches.len() > 1
                && union_selection == UnionSelection::ValidateAmbiguousBranches
                && !self.jiter_node_can_represent_value(*branch, value, MAX_MODEL_DEPTH)?
            {
                continue;
            }
            if let Some(converted) = branch_attempt(
                self.convert_jiter(py, *branch, value, union_selection),
                &mut first_error,
            )? {
                return Ok(converted);
            }
        }
        Err(no_union_branch_matched(first_error))
    }

    #[allow(clippy::too_many_arguments)]
    fn convert_jiter_model_value(
        &self,
        py: Python<'_>,
        model_type: &Py<PyType>,
        fields: &ModelFields,
        extra_plan: Option<&ExtraPropertiesPlan>,
        value: &JiterJsonValue<'_>,
        union_selection: UnionSelection,
    ) -> ConversionResult<Py<PyAny>> {
        let JiterJsonValue::Object(entries) = value else {
            return Err(ConversionFailure::Mismatch(
                ConversionMismatch::ExpectedType {
                    expected: "JSON object".to_owned(),
                    actual: None,
                },
            ));
        };
        let instance = allocate_model(py, model_type, &self.object_new)?;
        let mut extra_output =
            extra_plan.map(|_| JiterFrozenDictBuilder::with_capacity(entries.len()));
        let mut present_fields = 0;

        for (key, item) in entries.iter() {
            let key_string = key.as_ref();
            if let Some(field) = fields.by_json_name(key_string) {
                if field.attribute.is_set(py, &instance)? {
                    return Err(ConversionFailure::Raised(duplicate_key(key)));
                }
                let converted = self.convert_jiter(py, field.value_node, item, union_selection)?;
                field.attribute.set(py, &instance, &converted)?;
                present_fields += 1;
            } else if let (Some(extra), Some(output)) = (extra_plan, extra_output.as_mut()) {
                output.push(
                    key_string,
                    PyString::new(py, key_string).into_any().unbind(),
                    self.convert_jiter(py, extra.value_node, item, union_selection)?,
                )?;
            } else {
                return Err(ConversionFailure::Mismatch(
                    ConversionMismatch::UnknownProperty(key_string.to_owned()),
                ));
            }
        }

        let extra_value = extra_output
            .map(|output| self.freeze_dict_pairs(py, output.finish()))
            .transpose()?;
        if present_fields != fields.len() {
            for field in fields {
                if !field.attribute.is_set(py, &instance)? {
                    let converted = self
                        .convert_missing_field_value(py, field)
                        .map_err(ConversionFailure::Mismatch)?;
                    field.attribute.set(py, &instance, &converted)?;
                }
            }
        }
        if let (Some(plan), Some(extra_value)) = (extra_plan, extra_value.as_ref()) {
            plan.attribute.set(py, &instance, extra_value)?;
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
            ConversionNode::Union(plan) => plan
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
                    if !self.jiter_node_can_represent_value(*item, value, remaining_depth - 1)? {
                        return Ok(false);
                    }
                }
                true
            }
            ConversionNode::Dict { value: value_node } => {
                let JiterJsonValue::Object(values) = value else {
                    return Ok(false);
                };
                for (_, item_value) in values.iter() {
                    if !self.jiter_node_can_represent_value(
                        *value_node,
                        item_value,
                        remaining_depth - 1,
                    )? {
                        return Ok(false);
                    }
                }
                true
            }
            ConversionNode::Union(plan) => {
                let mut represents = false;
                for branch in plan {
                    if self.jiter_node_can_represent_value(*branch, value, remaining_depth - 1)? {
                        represents = true;
                        break;
                    }
                }
                represents
            }
            ConversionNode::Model {
                branch_schema,
                fields,
                extra,
                ..
            } => {
                let JiterJsonValue::Object(values) = value else {
                    return Ok(false);
                };
                if !branch_schema.is_valid_instance(JsonInstanceRef::from_jiter(value))? {
                    return Ok(false);
                }
                for (key, item_value) in values.iter() {
                    let child = fields
                        .by_json_name(key.as_ref())
                        .map(|field| field.value_node)
                        .or_else(|| extra.as_ref().map(|extra| extra.value_node));
                    let Some(child) = child else {
                        return Ok(false);
                    };
                    if !self.jiter_node_can_represent_value(
                        child,
                        item_value,
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
                if !branch_schema.is_valid_instance(JsonInstanceRef::from_jiter(value))? {
                    return Ok(false);
                }
                self.jiter_node_can_represent_value(*child, value, remaining_depth - 1)?
            }
        })
    }

    fn normalize_output_leaf(
        &self,
        py: Python<'_>,
        node: &ConversionNode,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<Option<Py<PyAny>>> {
        match node {
            ConversionNode::Scalar {
                kind: ScalarKind::Any,
            } => canonical_python_scalar(py, value),
            ConversionNode::Scalar { kind } => convert_scalar(py, *kind, value)
                .map(Some)
                .map_err(ConversionFailure::into_pyerr),
            ConversionNode::Literal { values } => convert_literal(py, values, value)
                .map(Some)
                .map_err(ConversionFailure::into_pyerr),
            _ => Ok(None),
        }
    }

    fn write_output_leaf(
        &self,
        py: Python<'_>,
        node: &ConversionNode,
        value: &Bound<'_, PyAny>,
        remaining_depth: u16,
        output: &mut Vec<u8>,
    ) -> PyResult<()> {
        let exact_scalar = match node {
            ConversionNode::Scalar {
                kind: ScalarKind::Any,
            } => is_exact_python_json_scalar(value),
            ConversionNode::Scalar {
                kind: ScalarKind::String,
            } => value.is_exact_instance_of::<PyString>(),
            ConversionNode::Scalar {
                kind: ScalarKind::Integer,
            } => value.is_exact_instance_of::<PyInt>(),
            ConversionNode::Scalar {
                kind: ScalarKind::Number,
            } => value.is_exact_instance_of::<PyInt>() || value.is_exact_instance_of::<PyFloat>(),
            ConversionNode::Scalar {
                kind: ScalarKind::Boolean,
            } => value.is_exact_instance_of::<PyBool>(),
            ConversionNode::Scalar {
                kind: ScalarKind::Null,
            } => value.is_none(),
            ConversionNode::Literal { values } => {
                let index = matching_literal_index(py, values, value)?
                    .ok_or_else(|| ConversionMismatch::Literal.into_pyerr())?;
                return write_serializable_json_value(
                    output,
                    values[index].bind(py),
                    remaining_depth,
                );
            }
            _ => false,
        };
        if exact_scalar {
            return write_serializable_json_value(output, value, remaining_depth);
        }
        let normalized = self
            .normalize_output_leaf(py, node, value)?
            .expect("scalar nodes always normalize to one leaf");
        write_serializable_json_value(output, normalized.bind(py), remaining_depth)
    }

    #[inline]
    #[allow(clippy::too_many_arguments)]
    fn insert_pending_json_entry<'py>(
        &self,
        py: Python<'py>,
        entries: &mut BTreeMap<String, (usize, Bound<'py, PyAny>)>,
        key: String,
        value_node: usize,
        value: Bound<'py, PyAny>,
        remaining_depth: u16,
        active_containers: ActiveContainers<'_>,
    ) -> PyResult<()> {
        let Some((displaced_node, displaced_value)) = entries.insert(key, (value_node, value))
        else {
            return Ok(());
        };

        // Materialization proves values before assigning them into a Python
        // dict, so a later canonical-key collision cannot hide an invalid
        // earlier value. Preserve that invariant without taxing the unique-key
        // path: only displaced values are written to a throwaway buffer.
        let mut scratch = Vec::new();
        self.write_json_node(
            py,
            displaced_node,
            &displaced_value,
            remaining_depth,
            active_containers,
            &mut scratch,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn write_any_json_node<'py>(
        &self,
        py: Python<'py>,
        node_id: usize,
        value: &Bound<'py, PyAny>,
        remaining_depth: u16,
        active_containers: ActiveContainers<'_>,
        output: &mut Vec<u8>,
    ) -> PyResult<()> {
        if let Some(scalar) = canonical_python_scalar(py, value)? {
            return write_serializable_json_value(output, scalar.bind(py), remaining_depth);
        }
        if let Ok(values) = value.cast::<PyMapping>() {
            let mut entries = BTreeMap::new();
            for entry in values.items()? {
                let (key, value) = mapping_pair(&entry)?;
                self.insert_pending_json_entry(
                    py,
                    &mut entries,
                    canonical_output_key(py, &key)?,
                    node_id,
                    value,
                    remaining_depth - 1,
                    active_containers,
                )?;
            }
            output.push(b'{');
            for (index, (key, (value_node, value))) in entries.into_iter().enumerate() {
                if index != 0 {
                    output.push(b',');
                }
                write_json_string(output, &key)?;
                output.push(b':');
                self.write_json_node(
                    py,
                    value_node,
                    &value,
                    remaining_depth - 1,
                    active_containers,
                    output,
                )?;
            }
            output.push(b'}');
            return Ok(());
        }
        if let Ok(values) = value.cast::<PyList>() {
            output.push(b'[');
            for (index, value) in values.iter().enumerate() {
                if index != 0 {
                    output.push(b',');
                }
                self.write_json_node(
                    py,
                    node_id,
                    &value,
                    remaining_depth - 1,
                    active_containers,
                    output,
                )?;
            }
            output.push(b']');
            return Ok(());
        }
        if let Ok(values) = value.cast::<PyTuple>() {
            output.push(b'[');
            for (index, value) in values.iter().enumerate() {
                if index != 0 {
                    output.push(b',');
                }
                self.write_json_node(
                    py,
                    node_id,
                    &value,
                    remaining_depth - 1,
                    active_containers,
                    output,
                )?;
            }
            output.push(b']');
            return Ok(());
        }
        Err(PyErr::new::<PyTypeError, _>(format!(
            "expected JSON value, got {}",
            value.get_type().name()?
        )))
    }

    fn write_json_node<'py>(
        &self,
        py: Python<'py>,
        node_id: usize,
        value: &Bound<'py, PyAny>,
        remaining_depth: u16,
        active_containers: ActiveContainers<'_>,
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
        if matches!(
            node,
            ConversionNode::Scalar {
                kind: ScalarKind::Any
            } | ConversionNode::List { .. }
                | ConversionNode::Dict { .. }
                | ConversionNode::Model { .. }
                | ConversionNode::Root { .. }
        ) {
            return active_containers.with_pyresult(value, |active_containers| {
                self.write_json_node_inner(
                    py,
                    node_id,
                    node,
                    value,
                    remaining_depth,
                    active_containers,
                    output,
                )
            });
        }
        self.write_json_node_inner(
            py,
            node_id,
            node,
            value,
            remaining_depth,
            active_containers,
            output,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn write_json_node_inner<'py>(
        &self,
        py: Python<'py>,
        node_id: usize,
        node: &ConversionNode,
        value: &Bound<'py, PyAny>,
        remaining_depth: u16,
        active_containers: ActiveContainers<'_>,
        output: &mut Vec<u8>,
    ) -> PyResult<()> {
        match node {
            ConversionNode::Scalar {
                kind: ScalarKind::Any,
            } => self.write_any_json_node(
                py,
                node_id,
                value,
                remaining_depth,
                active_containers,
                output,
            ),
            ConversionNode::Scalar { .. } | ConversionNode::Literal { .. } => {
                self.write_output_leaf(py, node, value, remaining_depth, output)
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
                    self.write_json_node(
                        py,
                        *item,
                        &item_value,
                        remaining_depth - 1,
                        active_containers,
                        output,
                    )?;
                }
                output.push(b']');
                Ok(())
            }
            ConversionNode::Dict { value: value_node } => {
                let input = self.frozen_dict_items(value)?;
                let mut entries = BTreeMap::new();
                for entry in input {
                    let (key, item) = mapping_pair(&entry)?;
                    self.insert_pending_json_entry(
                        py,
                        &mut entries,
                        canonical_output_key(py, &key)?,
                        *value_node,
                        item,
                        remaining_depth - 1,
                        active_containers,
                    )?;
                }
                output.push(b'{');
                for (index, (key, (item_node, item))) in entries.into_iter().enumerate() {
                    if index != 0 {
                        output.push(b',');
                    }
                    write_json_string(output, &key)?;
                    output.push(b':');
                    self.write_json_node(
                        py,
                        item_node,
                        &item,
                        remaining_depth - 1,
                        active_containers,
                        output,
                    )?;
                }
                output.push(b'}');
                Ok(())
            }
            ConversionNode::Union(plan) => {
                let checkpoint = output.len();
                let mut first_error = None;
                for branch in plan {
                    if self.node_matches_model_value(py, *branch, value)? {
                        match self.write_json_node(
                            py,
                            *branch,
                            value,
                            remaining_depth - 1,
                            active_containers,
                            output,
                        ) {
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
                extra,
                ..
            } => {
                if !value.is_instance(model_type.bind(py))? {
                    let expected = model_type.bind(py).name()?.to_str()?.to_owned();
                    return Err(expected_type(&expected, value)?);
                }

                if extra.is_none() {
                    output.push(b'{');
                    let mut first = true;
                    for field in fields {
                        let field_value = field.attribute.get(value)?;
                        if field.presence.is_omittable()
                            && field_value.is(self.missing_sentinel.bind(py))
                        {
                            continue;
                        }
                        if first {
                            first = false;
                        } else {
                            output.push(b',');
                        }
                        write_json_string(output, &field.json_name)?;
                        output.push(b':');
                        self.write_json_node(
                            py,
                            field.value_node,
                            &field_value,
                            remaining_depth - 1,
                            active_containers,
                            output,
                        )?;
                    }
                    output.push(b'}');
                    return Ok(());
                }

                let mut entries = BTreeMap::new();
                for field in fields {
                    let field_value = field.attribute.get(value)?;
                    if field.presence.is_omittable()
                        && field_value.is(self.missing_sentinel.bind(py))
                    {
                        continue;
                    }
                    self.insert_pending_json_entry(
                        py,
                        &mut entries,
                        field.json_name.clone(),
                        field.value_node,
                        field_value,
                        remaining_depth - 1,
                        active_containers,
                    )?;
                }
                if let Some(extra_plan) = extra {
                    let extra_value = extra_plan.attribute.get(value)?;
                    let extra_items = self.frozen_dict_items(&extra_value)?;
                    for entry in extra_items {
                        let (key, item) = mapping_pair(&entry)?;
                        self.insert_pending_json_entry(
                            py,
                            &mut entries,
                            canonical_output_key(py, &key)?,
                            extra_plan.value_node,
                            item,
                            remaining_depth - 1,
                            active_containers,
                        )?;
                    }
                }

                output.push(b'{');
                for (index, (key, (value_node, value))) in entries.into_iter().enumerate() {
                    if index != 0 {
                        output.push(b',');
                    }
                    write_json_string(output, &key)?;
                    output.push(b':');
                    self.write_json_node(
                        py,
                        value_node,
                        &value,
                        remaining_depth - 1,
                        active_containers,
                        output,
                    )?;
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
                let root = root_attribute.get(value)?;
                self.write_json_node(
                    py,
                    *value_node,
                    &root,
                    remaining_depth - 1,
                    active_containers,
                    output,
                )
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
            } => copy_python_json_value(py, value, remaining_depth),
            ConversionNode::Scalar { .. } | ConversionNode::Literal { .. } => Ok(self
                .normalize_output_leaf(py, node, value)?
                .expect("scalar and literal nodes always normalize to one leaf")),
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
            ConversionNode::Dict { value: value_node } => {
                let input = self.frozen_dict_items(value)?;
                let output = PyDict::new(py);
                for entry in input {
                    let (key, item) = mapping_pair(&entry)?;
                    output.set_item(
                        canonical_output_key(py, &key)?,
                        self.to_python_value_node(py, *value_node, &item, remaining_depth - 1)?,
                    )?;
                }
                Ok(output.into_any().unbind())
            }
            ConversionNode::Union(plan) => {
                let mut first_error = None;
                for branch in plan {
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
                extra,
                ..
            } => {
                if !value.is_instance(model_type.bind(py))? {
                    let expected = model_type.bind(py).name()?.to_str()?.to_owned();
                    return Err(expected_type(&expected, value)?);
                }
                let output = PyDict::new(py);
                for field in fields {
                    let field_value = field.attribute.get(value)?;
                    if field.presence.is_omittable()
                        && field_value.is(self.missing_sentinel.bind(py))
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
                if let Some(extra_plan) = extra {
                    let extra_value = extra_plan.attribute.get(value)?;
                    let extra_items = self.frozen_dict_items(&extra_value)?;
                    for entry in extra_items {
                        let (key, item) = mapping_pair(&entry)?;
                        output.set_item(
                            canonical_output_key(py, &key)?,
                            self.to_python_value_node(
                                py,
                                extra_plan.value_node,
                                &item,
                                remaining_depth - 1,
                            )?,
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
                let root = root_attribute.get(value)?;
                self.to_python_value_node(py, *value_node, &root, remaining_depth - 1)
            }
        }
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
            ConversionNode::Literal { values, .. } => {
                matching_literal_index(py, values, value)?.is_some()
            }
            ConversionNode::Union(plan) => {
                let mut matches = false;
                for branch in plan {
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
}

impl<'converter> ModelProjection<'converter> {
    pub(crate) fn instance<'a, 'py>(&'a self, value: &'a Bound<'py, PyAny>) -> JsonInstanceRef<'a>
    where
        'py: 'a,
    {
        JsonInstanceRef::from_projected_python(value, self.converter.root.0, self)
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
        attribute: &ModelAttribute,
        node: usize,
    ) -> Option<ProjectedPythonValue<'a>> {
        let py = value.value().py();
        attribute.ensure_owner(py, value.value().as_ptr()).ok()?;
        if let Some(child) = attribute.native_value_ptr_from_object(py, value.value().as_ptr()) {
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
            ConversionNode::Union(plan) => {
                let py = value.value().py();
                let cache_key = (value.node(), value.value().as_ptr() as usize);
                let selected = self.union_branches.borrow().get(&cache_key).copied();
                let selected = selected.or_else(|| {
                    let bound = value.value().to_owned();
                    let mut first_match = None;
                    let mut match_count = 0;
                    for branch in plan {
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
                    for branch in plan {
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
                            .write_json_node(
                                py,
                                *branch,
                                &bound,
                                remaining_depth - 1,
                                ActiveContainers::default(),
                                &mut scratch,
                            )
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
                self.model_attribute(value, root_attribute, *value_node)
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
            &self.converter.frozen_dict_items_attribute,
            value.node(),
        )?;
        storage.value().cast::<PyTuple>().ok()
    }

    fn extra_mapping<'a>(
        &'a self,
        value: ProjectedPythonValue<'a>,
        extra_attribute: &ModelAttribute,
    ) -> Option<Borrowed<'a, 'a, PyTuple>> {
        let extra = self.model_attribute(value, extra_attribute, value.node())?;
        self.mapping_storage(extra)
    }

    fn field_value<'a>(
        &'a self,
        value: ProjectedPythonValue<'a>,
        field: &FieldPlan,
    ) -> Option<ProjectedPythonValue<'a>> {
        let child = self.model_attribute(value, &field.attribute, field.value_node)?;
        if field.presence.is_omittable()
            && child
                .value()
                .is(self.converter.missing_sentinel.bind(child.value().py()))
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
            Some(ConversionNode::Model { fields, extra, .. }) => {
                if extra.is_none() {
                    return fields.len() - fields.omittable.len()
                        + fields
                            .omittable
                            .iter()
                            .filter(|field| self.field_value(value, fields.get(**field)).is_some())
                            .count();
                }
                let extra_mapping = extra
                    .as_ref()
                    .and_then(|extra| self.extra_mapping(value, &extra.attribute));
                let mut len = extra_mapping.map_or(0, |dictionary| dictionary.len());
                for field in fields {
                    if extra_mapping.is_some_and(|dictionary| {
                        Self::mapping_contains(value, dictionary, field.json_name.as_str())
                    }) {
                        continue;
                    }
                    if self.field_value(value, field).is_some() {
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
                extra: Some(extra), ..
            }) => self
                .extra_mapping(value, &extra.attribute)
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
            ConversionNode::Model { fields, extra, .. } => {
                if let Some(extra_plan) = extra {
                    let extra = self.extra_mapping(value, &extra_plan.attribute)?;
                    if let Some(child) = self.dict_get(value, extra, key, extra_plan.value_node) {
                        return Some(child);
                    }
                }
                let field = fields.by_json_name(key)?;
                self.field_value(value, field)
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
            ConversionNode::Model { fields, extra, .. } => {
                let extra_mapping = extra
                    .as_ref()
                    .and_then(|extra| self.extra_mapping(value, &extra.attribute));
                while state[0] < fields.len() {
                    let field_index = state[0];
                    state[0] += 1;
                    let field = &fields[field_index];
                    if extra_mapping.is_some_and(|dictionary| {
                        Self::mapping_contains(value, dictionary, field.json_name.as_str())
                    }) {
                        continue;
                    }
                    if let Some(child) = self.field_value(value, field) {
                        return Some((field.json_name.as_str(), child));
                    }
                }
                match (extra_mapping, extra) {
                    (Some(extra_mapping), Some(extra_plan)) => {
                        self.dict_next(value, extra_mapping, &mut state[1], extra_plan.value_node)
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
) -> ConversionResult<Py<PyAny>> {
    let valid = match kind {
        ScalarKind::Any => true,
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
        return Err(ConversionFailure::Mismatch(
            ConversionMismatch::ExpectedType {
                expected: scalar_name(kind).to_owned(),
                actual: None,
            },
        ));
    }

    let python_value = if matches!(kind, ScalarKind::Integer) {
        if let JiterJsonValue::Float(number) = value {
            number
                .into_pyobject(py)
                .expect("f64 conversion to Python is infallible")
                .call_method0("__int__")?
                .unbind()
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

#[inline]
fn is_python_json_array(value: &Bound<'_, PyAny>) -> bool {
    value.is_instance_of::<PyList>() || value.is_instance_of::<PyTuple>()
}

#[inline]
fn is_exact_python_json_scalar(value: &Bound<'_, PyAny>) -> bool {
    value.is_none()
        || value.is_exact_instance_of::<PyBool>()
        || value.is_exact_instance_of::<PyInt>()
        || value.is_exact_instance_of::<PyFloat>()
        || value.is_exact_instance_of::<PyString>()
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

#[inline]
fn canonical_json_object_key(
    py: Python<'_>,
    value: &Bound<'_, PyAny>,
) -> ConversionResult<Py<PyAny>> {
    convert_scalar(py, ScalarKind::String, value)
}

fn canonical_output_key(py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<String> {
    canonical_json_object_key(py, value)
        .map_err(ConversionFailure::into_pyerr)?
        .extract(py)
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
    if let Some(scalar) = canonical_python_scalar(py, value)? {
        return Ok(scalar);
    }
    if let Ok(input) = value.cast::<PyMapping>() {
        let output = PyDict::new(py);
        for entry in input.items()? {
            let (key, item) = mapping_pair(&entry)?;
            let key = canonical_output_key(py, &key)?;
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
    Err(PyErr::new::<PyTypeError, _>(format!(
        "expected JSON value, got {}",
        value.get_type().name()?
    )))
}

fn write_serializable_json_value(
    output: &mut Vec<u8>,
    value: &Bound<'_, PyAny>,
    remaining_depth: u16,
) -> PyResult<()> {
    if remaining_depth == 0 {
        return Err(PyErr::new::<PyValueError, _>(
            "generated model serialization exceeds the maximum nesting depth",
        ));
    }
    if value.is_none() {
        output.extend_from_slice(b"null");
        return Ok(());
    }
    if value.is_exact_instance_of::<PyBool>() {
        output.extend_from_slice(if value.extract()? { b"true" } else { b"false" });
        return Ok(());
    }
    if value.is_exact_instance_of::<PyInt>() {
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
    if value.is_exact_instance_of::<PyFloat>() {
        let number = value.extract::<f64>()?;
        if !number.is_finite() {
            return Err(PyErr::new::<PyValueError, _>("JSON numbers must be finite"));
        }
        return serde_json::to_writer(&mut *output, &number).map_err(json_serialization_error);
    }
    if let Ok(value) = value.cast_exact::<PyString>() {
        return write_json_string(output, value.to_str()?);
    }
    if let Ok(values) = value.cast::<PyList>() {
        output.push(b'[');
        for (index, value) in values.iter().enumerate() {
            if index != 0 {
                output.push(b',');
            }
            write_serializable_json_value(output, &value, remaining_depth - 1)?;
        }
        output.push(b']');
        return Ok(());
    }
    if let Ok(values) = value.cast::<PyTuple>() {
        output.push(b'[');
        for (index, value) in values.iter().enumerate() {
            if index != 0 {
                output.push(b',');
            }
            write_serializable_json_value(output, &value, remaining_depth - 1)?;
        }
        output.push(b']');
        return Ok(());
    }
    if let Ok(values) = value.cast::<PyDict>() {
        let mut entries = Vec::with_capacity(values.len());
        for (key, value) in values {
            let key = key
                .cast::<PyString>()
                .map_err(|_| PyErr::new::<PyTypeError, _>("JSON object keys must be strings"))?
                .to_str()?
                .to_owned();
            entries.push((key, value));
        }
        entries.sort_unstable_by(|left, right| left.0.cmp(&right.0));
        output.push(b'{');
        for (index, (key, value)) in entries.into_iter().enumerate() {
            if index != 0 {
                output.push(b',');
            }
            write_json_string(output, &key)?;
            output.push(b':');
            write_serializable_json_value(output, &value, remaining_depth - 1)?;
        }
        output.push(b'}');
        return Ok(());
    }
    Err(PyErr::new::<PyTypeError, _>(format!(
        "expected JSON value, got {}",
        value.get_type().name()?
    )))
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
    value: &Bound<'_, PyAny>,
) -> ConversionResult<Py<PyAny>> {
    let valid = match kind {
        ScalarKind::Any => true,
        ScalarKind::String => value.is_instance_of::<PyString>(),
        ScalarKind::Integer => {
            (value.is_instance_of::<PyInt>() && !value.is_instance_of::<PyBool>())
                || (value.is_instance_of::<PyFloat>() && {
                    let number = value.extract::<f64>()?;
                    number.is_finite() && number.fract() == 0.0
                })
        }
        ScalarKind::Number => {
            !value.is_instance_of::<PyBool>()
                && (value.is_instance_of::<PyInt>() || value.is_instance_of::<PyFloat>())
        }
        ScalarKind::Boolean => value.is_instance_of::<PyBool>(),
        ScalarKind::Null => value.is_none(),
    };
    if !valid {
        return Err(ConversionFailure::Mismatch(expected_type_mismatch(
            scalar_name(kind),
            value,
        )?));
    }

    if matches!(kind, ScalarKind::Integer) && value.is_instance_of::<PyFloat>() {
        return Ok(py
            .get_type::<PyFloat>()
            .getattr("__int__")?
            .call1((value,))?
            .unbind());
    }
    canonical_python_scalar(py, value)?.ok_or_else(|| {
        ConversionFailure::Mismatch(ConversionMismatch::ExpectedType {
            expected: scalar_name(kind).to_owned(),
            actual: None,
        })
    })
}

fn convert_direct_scalar(
    py: Python<'_>,
    kind: ScalarKind,
    value: &Bound<'_, PyAny>,
) -> ConversionResult<Py<PyAny>> {
    if matches!(kind, ScalarKind::Integer)
        && (value.is_instance_of::<PyBool>() || !value.is_instance_of::<PyInt>())
    {
        return Err(ConversionFailure::Mismatch(expected_type_mismatch(
            "int", value,
        )?));
    }
    convert_scalar(py, kind, value)
}

fn scalar_name(kind: ScalarKind) -> &'static str {
    match kind {
        ScalarKind::Any => "JSON value",
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
    if let Some(index) = matching_literal_index(py, values, value)? {
        Ok(values[index].clone_ref(py))
    } else {
        Err(ConversionFailure::Mismatch(ConversionMismatch::Literal))
    }
}

fn matching_literal_index(
    py: Python<'_>,
    values: &[Py<PyAny>],
    value: &Bound<'_, PyAny>,
) -> PyResult<Option<usize>> {
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
    Ok(index)
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

fn expected_type_mismatch(
    expected: &str,
    value: &Bound<'_, PyAny>,
) -> PyResult<ConversionMismatch> {
    Ok(ConversionMismatch::ExpectedType {
        expected: expected.to_owned(),
        actual: Some(value.get_type().name()?.to_string_lossy().into_owned()),
    })
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
    missing_sentinel: Py<PyAny>,
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
    let object_new = py.get_type::<PyAny>().getattr("__new__")?.unbind();
    Ok(Rc::new(ModelConverterPlan {
        nodes,
        object_new,
        missing_sentinel: MissingSentinel(missing_sentinel),
        frozen_list_type: frozen_list_type.clone().unbind(),
        frozen_dict_type: frozen_dict_type.clone().unbind(),
        frozen_dict_items_attribute: ModelAttribute::compile(py, frozen_dict_type, "_items")?,
    }))
}

pub(crate) fn root_model_converter_plan(
    py: Python<'_>,
    plan: &Rc<ModelConverterPlan>,
    model_type: &Bound<'_, PyType>,
    root: usize,
) -> PyResult<RootedModelConverterPlan> {
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
    Ok(RootedModelConverterPlan {
        plan: Rc::downgrade(plan),
        root: RootNode(root),
    })
}

#[cfg(all(Py_3_11, not(any(PyPy, GraalPy, Py_GIL_DISABLED))))]
fn inspect_native_slot(model_type: &Bound<'_, PyType>, name: &str) -> PyResult<SlotInspection> {
    let descriptor = model_type.getattr(name)?;
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
            return Err(PyErr::new::<PyTypeError, _>(format!(
                "generated attribute {name:?} must be an exact member descriptor"
            )));
        }
        let descriptor = descriptor.as_ptr().cast::<ffi::PyMemberDescrObject>();
        let owner = (*descriptor).d_common.d_type;
        let member = (*descriptor).d_member;
        let concrete_type = model_type.as_ptr().cast::<ffi::PyTypeObject>();
        if owner.is_null() || owner != concrete_type || member.is_null() || (*member).name.is_null()
        {
            return Err(PyErr::new::<PyTypeError, _>(format!(
                "generated attribute {name:?} has an invalid member descriptor owner"
            )));
        }
        let member_name = std::ffi::CStr::from_ptr((*member).name);
        if member_name.to_bytes() != name.as_bytes() {
            return Err(PyErr::new::<PyTypeError, _>(format!(
                "generated attribute {name:?} aliases member descriptor {:?}",
                member_name.to_string_lossy()
            )));
        }
        if (*member).type_code != ffi::Py_T_OBJECT_EX {
            return Err(PyErr::new::<PyTypeError, _>(format!(
                "generated attribute {name:?} is not an object slot"
            )));
        }
        let offset = usize::try_from((*member).offset).map_err(|_| {
            PyErr::new::<PyTypeError, _>(format!(
                "generated attribute {name:?} has a negative slot offset"
            ))
        })?;
        let basicsize = usize::try_from((*concrete_type).tp_basicsize).map_err(|_| {
            PyErr::new::<PyTypeError, _>("generated model has an invalid allocation size")
        })?;
        let pointer_size = std::mem::size_of::<*mut ffi::PyObject>();
        if offset < std::mem::size_of::<ffi::PyObject>()
            || offset % std::mem::align_of::<*mut ffi::PyObject>() != 0
            || offset
                .checked_add(pointer_size)
                .is_none_or(|end| end > basicsize)
        {
            return Err(PyErr::new::<PyTypeError, _>(format!(
                "generated attribute {name:?} has an invalid slot offset"
            )));
        }
        Ok(SlotInspection::Native(ValidatedNativeSlot {
            owner: model_type.clone().unbind(),
            offset: NonZeroUsize::new(offset).ok_or_else(|| {
                PyErr::new::<PyTypeError, _>(format!(
                    "generated attribute {name:?} has a zero slot offset"
                ))
            })?,
        }))
    }
}

#[cfg(not(all(Py_3_11, not(any(PyPy, GraalPy, Py_GIL_DISABLED)))))]
fn inspect_native_slot(_model_type: &Bound<'_, PyType>, _name: &str) -> PyResult<SlotInspection> {
    Ok(SlotInspection::Portable)
}

fn parse_node(py: Python<'_>, descriptor: &Bound<'_, PyAny>) -> PyResult<ConversionNode> {
    let descriptor = descriptor.cast::<PyTuple>()?;
    if descriptor.is_empty() {
        return Err(PyErr::new::<PyValueError, _>(
            "native model node descriptors cannot be empty",
        ));
    }
    let tag = descriptor.get_item(0)?.extract::<String>()?;
    match tag.as_str() {
        "any" => parse_scalar_node(descriptor, ScalarKind::Any),
        "str" => parse_scalar_node(descriptor, ScalarKind::String),
        "int" => parse_scalar_node(descriptor, ScalarKind::Integer),
        "float" => parse_scalar_node(descriptor, ScalarKind::Number),
        "bool" => parse_scalar_node(descriptor, ScalarKind::Boolean),
        "null" => parse_scalar_node(descriptor, ScalarKind::Null),
        "list" => {
            require_arity(descriptor, 2, "list node")?;
            Ok(ConversionNode::List {
                item: descriptor.get_item(1)?.extract()?,
            })
        }
        "dict" => {
            require_arity(descriptor, 2, "dict node")?;
            Ok(ConversionNode::Dict {
                value: descriptor.get_item(1)?.extract()?,
            })
        }
        "literal" => {
            require_arity(descriptor, 2, "literal node")?;
            let values = descriptor.get_item(1)?;
            let values = values.cast::<PyTuple>()?;
            if values.is_empty() {
                return Err(PyErr::new::<PyValueError, _>(
                    "native literal nodes must contain at least one value",
                ));
            }
            let values = values
                .iter()
                .map(|value| {
                    validate_literal_payload(&value)?;
                    Ok(value.unbind())
                })
                .collect::<PyResult<Vec<_>>>()?;
            Ok(ConversionNode::Literal { values })
        }
        "union" => parse_union_node(descriptor),
        "model" => parse_model_node(py, descriptor),
        "root" => {
            require_arity(descriptor, 3, "root model node")?;
            let model_type = descriptor.get_item(1)?.cast_into::<PyType>()?.unbind();
            let branch_schema = schema_for_model(model_type.bind(py))?;
            Ok(ConversionNode::Root {
                branch_schema,
                root_attribute: ModelAttribute::compile(py, model_type.bind(py), "root")?,
                model_type,
                value: descriptor.get_item(2)?.extract()?,
            })
        }
        _ => Err(PyErr::new::<PyValueError, _>(format!(
            "unknown model converter node kind {tag:?}"
        ))),
    }
}

fn require_arity(descriptor: &Bound<'_, PyTuple>, expected: usize, context: &str) -> PyResult<()> {
    if descriptor.len() == expected {
        Ok(())
    } else {
        Err(PyErr::new::<PyValueError, _>(format!(
            "{context} must contain exactly {expected} items, got {}",
            descriptor.len()
        )))
    }
}

fn parse_scalar_node(
    descriptor: &Bound<'_, PyTuple>,
    kind: ScalarKind,
) -> PyResult<ConversionNode> {
    require_arity(descriptor, 1, "scalar node")?;
    Ok(scalar_node(kind))
}

fn validate_literal_payload(value: &Bound<'_, PyAny>) -> PyResult<()> {
    let supported = value.is_none()
        || value.is_exact_instance_of::<PyBool>()
        || value.is_exact_instance_of::<PyInt>()
        || value.cast_exact::<PyFloat>().is_ok_and(|value| {
            value
                .extract::<f64>()
                .is_ok_and(|number| number.is_finite())
        })
        || value.is_exact_instance_of::<PyString>();
    if supported {
        Ok(())
    } else {
        Err(PyErr::new::<PyTypeError, _>(
            "native literal values must be null, bool, int, finite float, or str",
        ))
    }
}

fn scalar_node(kind: ScalarKind) -> ConversionNode {
    ConversionNode::Scalar { kind }
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
    require_arity(descriptor, 4, "union node")?;
    let branches = descriptor.get_item(1)?.cast_into::<PyTuple>()?;
    let branches = branches
        .iter()
        .map(|branch| branch.extract::<usize>())
        .collect::<PyResult<Vec<_>>>()?;
    let discriminator_name = descriptor.get_item(2)?;
    let discriminator = if discriminator_name.is_none() {
        if !descriptor.get_item(3)?.is_none() {
            return Err(PyErr::new::<PyValueError, _>(
                "union discriminator entries require a discriminator name",
            ));
        }
        None
    } else {
        let entries = descriptor.get_item(3)?.cast_into::<PyTuple>()?;
        if entries.is_empty() {
            return Err(PyErr::new::<PyValueError, _>(
                "native discriminator plans must contain at least one entry",
            ));
        }
        let mut branches_by_value = HashMap::with_capacity(entries.len());
        for entry in entries {
            let entry = entry.cast_into::<PyTuple>()?;
            require_arity(&entry, 2, "discriminator entry")?;
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
            branch_ordinals_by_value: branches_by_value,
        })
    };
    Ok(ConversionNode::Union(UnionPlan::new(
        branches,
        discriminator,
    )?))
}

fn parse_model_node(py: Python<'_>, descriptor: &Bound<'_, PyTuple>) -> PyResult<ConversionNode> {
    require_arity(descriptor, 4, "model node")?;
    let model_type = descriptor.get_item(1)?.cast_into::<PyType>()?;
    let branch_schema = schema_for_model(&model_type)?;
    let field_descriptors = descriptor.get_item(2)?.cast_into::<PyTuple>()?;
    let mut fields = Vec::with_capacity(field_descriptors.len());
    for field in field_descriptors.iter() {
        let field = field.cast_into::<PyTuple>()?;
        require_arity(&field, 4, "model field")?;
        let json_name = field.get_item(0)?.extract::<String>()?;
        let py_name = field.get_item(1)?.extract::<String>()?;
        if py_name == "__jsoncompat_extra__" {
            return Err(PyErr::new::<PyValueError, _>(format!(
                "generated Python field name {py_name:?} is reserved by the model runtime"
            )));
        }
        let omittable = field.get_item(3)?;
        if !omittable.is_exact_instance_of::<PyBool>() {
            return Err(PyErr::new::<PyTypeError, _>(
                "model field omittable flag must be bool",
            ));
        }
        fields.push((
            FieldPlan {
                json_name,
                attribute: ModelAttribute::compile(py, &model_type, &py_name)?,
                value_node: field.get_item(2)?.extract()?,
                presence: if omittable.extract::<bool>()? {
                    FieldPresence::Omittable
                } else {
                    FieldPresence::Required
                },
            },
            py_name,
        ));
    }
    let extra_value = descriptor.get_item(3)?;
    let extra_value = if extra_value.is_none() {
        None
    } else {
        Some(extra_value.extract::<usize>()?)
    };
    let fields = ModelFields::new(fields)?;
    let extra = extra_value
        .map(|value_node| {
            Ok::<_, PyErr>(ExtraPropertiesPlan {
                value_node,
                attribute: ModelAttribute::compile(py, &model_type, "__jsoncompat_extra__")?,
            })
        })
        .transpose()?;
    Ok(ConversionNode::Model {
        model_type: model_type.unbind(),
        branch_schema,
        fields,
        extra,
    })
}

fn schema_for_model(model_type: &Bound<'_, PyType>) -> PyResult<BranchSchema> {
    let schema = model_type
        .getattr("__jsoncompat_schema__")?
        .extract::<String>()?;
    let schema = serde_json::from_str::<serde_json::Value>(&schema).map_err(|error| {
        PyErr::new::<PyValueError, _>(format!("generated model schema is not valid JSON: {error}"))
    })?;
    Ok(BranchSchema {
        raw: schema,
        compiled: OnceCell::new(),
    })
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
            ConversionNode::Dict { value } => check(*value)?,
            ConversionNode::Union(plan) => {
                for branch in plan {
                    check(*branch)?;
                }
            }
            ConversionNode::Model { fields, extra, .. } => {
                for field in fields {
                    check(field.value_node)?;
                }
                if let Some(extra) = extra {
                    check(extra.value_node)?;
                }
            }
            ConversionNode::Root { value, .. } => check(*value)?,
        }
    }
    Ok(())
}
