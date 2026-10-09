//! Compiled conversion plans for generated Python dataclasses.
//!
//! Plans move repeated object-graph traversal into Rust while retaining the
//! Python runtime's existing type checks, missing-field factories, union
//! selection, and frozen-slot construction semantics.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap, HashSet};
#[cfg(all(Py_3_11, not(any(PyPy, GraalPy, Py_GIL_DISABLED))))]
use std::num::NonZeroUsize;
use std::ops::Deref;
use std::sync::Arc;

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
#[cfg(not(all(Py_3_11, not(any(PyPy, GraalPy, Py_GIL_DISABLED)))))]
use pyo3::{exceptions::PyAttributeError, types::PyModule};

use super::prepared_schema::{NodeExt, PreparedSchema};

mod input;
mod output;
mod prepared;
mod projection;
mod streaming;
pub(crate) use streaming::rounded_integer_float;

// Keep recursive conversion comfortably inside the smallest native thread
// stacks used by supported platforms. In particular, Windows debug builds can
// exhaust their stack before a 255-level guard is reached.
const MAX_MODEL_DEPTH: u16 = 64;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct NodeId(usize);

impl NodeId {
    fn parse(raw: usize, node_count: usize) -> PyResult<Self> {
        if raw < node_count {
            Ok(Self(raw))
        } else {
            Err(PyErr::new::<PyIndexError, _>(format!(
                "model converter node reference {raw} is out of bounds"
            )))
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BranchOrdinal(usize);

impl BranchOrdinal {
    fn parse(raw: usize, branch_count: usize) -> PyResult<Self> {
        if raw < branch_count {
            Ok(Self(raw))
        } else {
            Err(PyErr::new::<PyIndexError, _>(
                "discriminator branch ordinal is out of bounds",
            ))
        }
    }
}

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
    branch_ordinals_by_value: HashMap<DiscriminatorKey, BranchOrdinal>,
}

struct UnionPlan {
    branches: Box<[NodeId]>,
    discriminator: Option<DiscriminatorPlan>,
}

impl UnionPlan {
    fn new(branches: Vec<NodeId>, discriminator: Option<DiscriminatorPlan>) -> PyResult<Self> {
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
    fn discriminated_branch(&self, key: &DiscriminatorKey) -> Option<NodeId> {
        let ordinal = *self
            .discriminator
            .as_ref()?
            .branch_ordinals_by_value
            .get(key)?;
        Some(self.branches[ordinal.0])
    }
}

impl Deref for UnionPlan {
    type Target = [NodeId];

    fn deref(&self) -> &Self::Target {
        &self.branches
    }
}

impl<'a> IntoIterator for &'a UnionPlan {
    type Item = &'a NodeId;
    type IntoIter = std::slice::Iter<'a, NodeId>;

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

// The interpreter ABI selects the storage at build time. Both forms retain
// the exact owner and are constructed only after descriptor validation.
struct ValidatedSlot {
    owner: Py<PyType>,
    #[cfg(all(Py_3_11, not(any(PyPy, GraalPy, Py_GIL_DISABLED))))]
    offset: NonZeroUsize,
    #[cfg(not(all(Py_3_11, not(any(PyPy, GraalPy, Py_GIL_DISABLED)))))]
    descriptor: Py<PyAny>,
}

struct ModelAttribute {
    name: Py<PyString>,
    storage: ValidatedSlot,
}

impl ModelAttribute {
    fn compile(py: Python<'_>, model_type: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        let name = PyString::new(py, name);
        let descriptor = model_type.getattr("__dict__")?.get_item(&name)?;
        Self::compile_descriptor(model_type, &name, &descriptor)
    }

    fn compile_descriptor(
        model_type: &Bound<'_, PyType>,
        name: &Bound<'_, PyString>,
        descriptor: &Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        Ok(Self {
            name: name.clone().unbind(),
            storage: inspect_native_slot(model_type, name.to_str()?, descriptor)?,
        })
    }

    #[inline(always)]
    fn owner(&self) -> &Py<PyType> {
        &self.storage.owner
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

    #[cfg(all(Py_3_11, not(any(PyPy, GraalPy, Py_GIL_DISABLED))))]
    #[inline(always)]
    fn native_slot_ptr(
        &self,
        py: Python<'_>,
        object: *mut ffi::PyObject,
    ) -> Option<*mut *mut ffi::PyObject> {
        let slot = &self.storage;
        if unsafe { ffi::Py_TYPE(object) } != slot.owner.bind(py).as_ptr().cast() {
            return None;
        }
        // SAFETY: `ValidatedSlot` is only created after proving that the
        // named member descriptor belongs to `owner` and identifies an
        // aligned object-pointer slot within the concrete allocation.
        Some(unsafe {
            object
                .cast::<u8>()
                .add(slot.offset.get())
                .cast::<*mut ffi::PyObject>()
        })
    }

    #[cfg(not(all(Py_3_11, not(any(PyPy, GraalPy, Py_GIL_DISABLED)))))]
    #[inline(always)]
    fn native_slot_ptr(
        &self,
        _py: Python<'_>,
        _object: *mut ffi::PyObject,
    ) -> Option<*mut *mut ffi::PyObject> {
        None
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
        #[cfg(not(all(Py_3_11, not(any(PyPy, GraalPy, Py_GIL_DISABLED)))))]
        {
            self.storage
                .descriptor
                .bind(py)
                .call_method1("__get__", (instance, self.owner().bind(py)))
        }
        #[cfg(all(Py_3_11, not(any(PyPy, GraalPy, Py_GIL_DISABLED))))]
        instance.getattr(self.name.bind(py))
    }

    #[inline(always)]
    fn is_set(&self, py: Python<'_>, instance: &Bound<'_, PyAny>) -> PyResult<bool> {
        self.ensure_owner(py, instance.as_ptr())?;
        if let Some(value) = self.native_value_ptr_from_object(py, instance.as_ptr()) {
            return Ok(!value.is_null());
        }
        #[cfg(not(all(Py_3_11, not(any(PyPy, GraalPy, Py_GIL_DISABLED)))))]
        {
            match self
                .storage
                .descriptor
                .bind(py)
                .call_method1("__get__", (instance, self.owner().bind(py)))
            {
                Ok(_) => Ok(true),
                Err(error) if error.is_instance_of::<PyAttributeError>(py) => Ok(false),
                Err(error) => Err(error),
            }
        }
        #[cfg(all(Py_3_11, not(any(PyPy, GraalPy, Py_GIL_DISABLED))))]
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

        #[cfg(not(all(Py_3_11, not(any(PyPy, GraalPy, Py_GIL_DISABLED)))))]
        {
            self.storage
                .descriptor
                .bind(py)
                .call_method1("__set__", (instance, value.bind(py)))?;
            Ok(())
        }
        #[cfg(all(Py_3_11, not(any(PyPy, GraalPy, Py_GIL_DISABLED))))]
        unreachable!("native slot storage returned no native slot")
    }

    #[inline]
    fn set_owned(
        &self,
        py: Python<'_>,
        instance: &Bound<'_, PyAny>,
        value: Py<PyAny>,
    ) -> PyResult<()> {
        self.ensure_owner(py, instance.as_ptr())?;
        if let Some(slot) = self.native_slot_ptr(py, instance.as_ptr()) {
            // SAFETY: the validated slot belongs to this exact instance, as
            // in `set`. Transfer the owned reference instead of retaining it
            // only to release the caller's reference immediately afterward.
            unsafe {
                let previous = std::ptr::replace(slot, value.into_ptr());
                ffi::Py_XDECREF(previous);
            }
            return Ok(());
        }
        self.set(py, instance, &value)
    }

    fn traverse(&self, visit: &PyVisit<'_>) -> Result<(), PyTraverseError> {
        visit.call(&self.name)?;
        visit.call(self.owner())?;
        #[cfg(not(all(Py_3_11, not(any(PyPy, GraalPy, Py_GIL_DISABLED)))))]
        visit.call(&self.storage.descriptor)?;
        Ok(())
    }
}

struct FieldPlan {
    json_name: String,
    json_prefix: Vec<u8>,
    attribute: ModelAttribute,
    value_node: NodeId,
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
    by_py_name: Option<HashMap<String, FieldId>>,
    omittable: Vec<FieldId>,
}

impl ModelFields {
    fn new(mut fields: Vec<(FieldPlan, Option<String>)>) -> PyResult<Self> {
        fields.sort_unstable_by(|(left, _), (right, _)| left.json_name.cmp(&right.json_name));
        let mut serialized = Vec::with_capacity(fields.len());
        let mut by_json_name = HashMap::with_capacity(fields.len());
        // Most generated names are already valid Python identifiers. In that
        // case the JSON index is also the keyword index, with no extra strings
        // or hash table needed. Aliased models retain a separate full index.
        let mut by_py_name = fields
            .iter()
            .any(|(_, name)| name.is_some())
            .then(|| HashMap::with_capacity(fields.len()));
        let mut omittable = Vec::new();
        for (field, py_name) in fields {
            let id = FieldId(serialized.len());
            if by_json_name.insert(field.json_name.clone(), id).is_some() {
                return Err(PyErr::new::<PyValueError, _>(format!(
                    "duplicate generated JSON field name {:?}",
                    field.json_name
                )));
            }
            if let Some(index) = &mut by_py_name {
                let py_name = py_name.unwrap_or_else(|| field.json_name.clone());
                if index.insert(py_name, id).is_some() {
                    return Err(PyErr::new::<PyValueError, _>(
                        "duplicate generated Python field name",
                    ));
                }
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
            .as_ref()
            .unwrap_or(&self.by_json_name)
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
    value_node: NodeId,
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
    program: Arc<PreparedSchema>,
}

type SchemaValidatorRef<'a> = &'a PreparedSchema;

impl BranchSchema {
    fn is_valid_instance(&self, instance: JsonInstanceRef<'_>) -> PyResult<bool> {
        Ok(self.program.is_valid_instance(instance))
    }
    fn is_valid_python_value(&self, py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<bool> {
        validate_python_value(&self.program, py, value, false)
    }
}

pub(crate) fn validate_python_value(
    schema: SchemaValidatorRef<'_>,
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
    };
    Ok(canonical
        .as_ref()
        .map_or_else(|| validate(value), |value| validate(value.bind(py))))
}

enum ConversionNode {
    Scalar {
        kind: ScalarKind,
    },
    List {
        item: NodeId,
    },
    Dict {
        value: NodeId,
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
        value: NodeId,
        root_attribute: ModelAttribute,
    },
}

pub(crate) struct ModelConverterPlan {
    nodes: Vec<ConversionNode>,
    conversion_validates: Vec<bool>,
    leaf_guards: Vec<Option<super::prepared_schema::Node>>,
    object_new: Py<PyAny>,
    missing_sentinel: MissingSentinel,
    frozen_list_type: Py<PyType>,
    frozen_dict_type: Py<PyType>,
    frozen_dict_items_attribute: ModelAttribute,
}

pub(crate) struct ModelConverterPy {
    plan: Arc<ModelConverterPlan>,
    root: RootNode,
    output_overrides: Cell<bool>,
    checked_output: Cell<bool>,
}

#[derive(Clone, Copy)]
struct RootNode(NodeId);

pub(crate) struct RootedModelConverterPlan {
    plan: std::sync::Weak<ModelConverterPlan>,
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
        if !self.converter.conversion_validates[self.converter.root.0.0]
            && !validate_python_value(self.converter.schema()?, py, value, true)?
        {
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
        };
        if !is_valid {
            if let Some(failure) = self.converter.schema()?.explain_instance(projected) {
                return Err(super::validation_error::from_failure(py, failure));
            }
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
    #[inline(always)]
    fn node(&self, id: NodeId) -> &ConversionNode {
        self.nodes
            .get(id.0)
            .expect("validated model converter node id must remain in bounds")
    }

    #[inline(always)]
    fn projected_node_id(&self, raw: usize) -> NodeId {
        NodeId::parse(raw, self.nodes.len())
            .expect("projected node ids originate from a validated model converter plan")
    }

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
    union_branches: RefCell<HashMap<(NodeId, usize), NodeId>>,
}

impl ModelConverterPy {
    pub(crate) fn model_type(&self) -> PyResult<&Py<PyType>> {
        match self.node(self.root.0) {
            ConversionNode::Model { model_type, .. } | ConversionNode::Root { model_type, .. } => {
                Ok(model_type)
            }
            _ => unreachable!("rooted model converter must have a generated model root"),
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
            output_overrides: Cell::new(false),
            checked_output: Cell::new(false),
        })
    }
}

impl ModelConverterPy {
    pub(crate) fn validate_emitted_json(&self, payload: &str) -> PyResult<bool> {
        if self.schema()?.requires_exact_json_numbers() {
            let exact: serde_json::Value = serde_json::from_str(payload)
                .map_err(|error| PyValueError::new_err(error.to_string()))?;
            return Ok(self
                .schema()?
                .is_valid_instance_assuming_json(JsonInstanceRef::from_serde(&exact)));
        }
        if self.conversion_validates[self.root.0.0] && !self.output_overrides.get() {
            return Ok(true);
        }
        let parsed = JiterJsonValue::parse(payload.as_bytes(), false)
            .map_err(|error| PyValueError::new_err(error.to_string()))?;
        Ok(self
            .schema()?
            .is_valid_instance_assuming_json(JsonInstanceRef::from_jiter(&parsed)))
    }

    pub(crate) fn schema(&self) -> PyResult<SchemaValidatorRef<'_>> {
        match self.node(self.root.0) {
            ConversionNode::Model { branch_schema, .. }
            | ConversionNode::Root { branch_schema, .. } => Ok(&branch_schema.program),
            _ => unreachable!("rooted model converter must have a generated model root"),
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
        let node = self.node(self.root.0);
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
        Ok(self
            .schema()?
            .is_valid_instance(JsonInstanceRef::from_python(value)))
    }

    pub(crate) fn validate_json_value(
        &self,
        py: Python<'_>,
        value: &MaterializedJsonValue,
    ) -> PyResult<bool> {
        Ok(self
            .schema()?
            .is_valid_instance_assuming_json(JsonInstanceRef::from_python(value.0.bind(py))))
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
        if self.conversion_validates[self.root.0.0] {
            return match self.convert_jiter(
                py,
                self.root.0,
                value,
                UnionSelection::ValidateAmbiguousBranches,
            ) {
                Ok(instance) => Ok(Some(instance)),
                Err(ConversionFailure::Mismatch(_)) => Ok(None),
                Err(ConversionFailure::Raised(error)) => Err(error),
            };
        }
        let is_valid = self
            .schema()?
            .is_valid_instance_assuming_json(JsonInstanceRef::from_jiter(value));
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

    pub(crate) fn serialize_model_checked(
        &self,
        py: Python<'_>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<String> {
        // Ambiguous branch guards need not describe the complete root
        // language. Let the prepared general program validate emitted JSON
        // whenever the generator could not prove the complete inline path.
        self.checked_output
            .set(self.conversion_validates[self.root.0.0]);
        self.serialize_model_trusted(py, value)
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
        // SAFETY: the writer only appends ASCII JSON punctuation/numbers,
        // serde_json string output, and complete Rust str slices. Prepared
        // field prefixes are parsed as JSON strings when loaded. Every write
        // therefore preserves UTF-8, including rollback after a union miss.
        Ok(unsafe { String::from_utf8_unchecked(output) })
    }

    pub(crate) fn materialize_json_value(
        &self,
        py: Python<'_>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<MaterializedJsonValue> {
        self.to_python_value_node(
            py,
            self.root.0,
            value,
            MAX_MODEL_DEPTH,
            ActiveContainers::default(),
        )
        .map(MaterializedJsonValue)
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
    active_containers: ActiveContainers<'_>,
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
        return active_containers.with_pyresult(value, |active_containers| {
            let output = PyDict::new(py);
            for entry in input.items()? {
                let (key, item) = mapping_pair(&entry)?;
                let key = canonical_output_key(py, &key)?;
                output.set_item(
                    key,
                    copy_python_json_value(py, &item, remaining_depth - 1, active_containers)?,
                )?;
            }
            Ok(output.into_any().unbind())
        });
    }
    if let Ok(input) = value.cast::<PyList>() {
        return active_containers.with_pyresult(value, |active_containers| {
            let output = PyList::empty(py);
            for item in input {
                output.append(copy_python_json_value(
                    py,
                    &item,
                    remaining_depth - 1,
                    active_containers,
                )?)?;
            }
            Ok(output.into_any().unbind())
        });
    }
    if let Ok(input) = value.cast::<PyTuple>() {
        return active_containers.with_pyresult(value, |active_containers| {
            let output = PyList::empty(py);
            for item in input {
                output.append(copy_python_json_value(
                    py,
                    &item,
                    remaining_depth - 1,
                    active_containers,
                )?)?;
            }
            Ok(output.into_any().unbind())
        });
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
    if needs_json_escape(value.as_bytes()) {
        return serde_json::to_writer(&mut *output, value).map_err(json_serialization_error);
    }
    output.reserve(value.len() + 2);
    output.push(b'"');
    output.extend_from_slice(value.as_bytes());
    output.push(b'"');
    Ok(())
}

/// Test eight bytes at a time for quotes, backslashes, or ASCII controls. The
/// subtraction may flag an extra byte after a match, but can never miss one.
/// Escaped strings retain serde_json's established escaping implementation.
fn needs_json_escape(bytes: &[u8]) -> bool {
    const LOW: u64 = 0x0101_0101_0101_0101;
    const HIGH: u64 = 0x8080_8080_8080_8080;
    let (chunks, remainder) = bytes.as_chunks::<8>();
    for chunk in chunks {
        let word = u64::from_ne_bytes(*chunk);
        let quote = word ^ (LOW * u64::from(b'"'));
        let slash = word ^ (LOW * u64::from(b'\\'));
        if ((quote.wrapping_sub(LOW) & !quote)
            | (slash.wrapping_sub(LOW) & !slash)
            | (word.wrapping_sub(LOW * 0x20) & !word))
            & HIGH
            != 0
        {
            return true;
        }
    }
    remainder
        .iter()
        .any(|byte| *byte < 0x20 || matches!(byte, b'"' | b'\\'))
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
    prepared_plan: &[u8],
) -> PyResult<Arc<ModelConverterPlan>> {
    if !frozen_list_type.is_subclass_of::<PyTuple>()? {
        return Err(PyErr::new::<PyTypeError, _>(
            "generated immutable sequence type must be a tuple subclass",
        ));
    }
    let mut nodes = Vec::with_capacity(descriptors.len());
    let node_count = descriptors.len();
    let mut schemas = HashMap::new();
    for descriptor in descriptors {
        nodes.push(parse_node(py, &descriptor, node_count, &mut schemas)?);
    }
    let object_new = py.get_type::<PyAny>().getattr("__new__")?.unbind();
    let mut plan = ModelConverterPlan {
        conversion_validates: vec![false; nodes.len()],
        leaf_guards: vec![None; nodes.len()],
        nodes,
        object_new,
        missing_sentinel: MissingSentinel(missing_sentinel),
        frozen_list_type: frozen_list_type.clone().unbind(),
        frozen_dict_type: frozen_dict_type.clone().unbind(),
        frozen_dict_items_attribute: ModelAttribute::compile(py, frozen_dict_type, "_items")?,
    };
    plan.install_prepared_plan(prepared_plan)?;
    Ok(Arc::new(plan))
}

pub(crate) fn root_model_converter_plan(
    py: Python<'_>,
    plan: &Arc<ModelConverterPlan>,
    model_type: &Bound<'_, PyType>,
    root: usize,
) -> PyResult<RootedModelConverterPlan> {
    let root = NodeId::parse(root, plan.nodes.len())?;
    let root_model_type = match plan.node(root) {
        ConversionNode::Model { model_type, .. } | ConversionNode::Root { model_type, .. } => {
            model_type
        }
        _ => {
            return Err(PyErr::new::<PyTypeError, _>(
                "model converter root must be a generated model",
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
        plan: Arc::downgrade(plan),
        root: RootNode(root),
    })
}

#[cfg(all(Py_3_11, not(any(PyPy, GraalPy, Py_GIL_DISABLED))))]
fn inspect_native_slot(
    model_type: &Bound<'_, PyType>,
    name: &str,
    descriptor: &Bound<'_, PyAny>,
) -> PyResult<ValidatedSlot> {
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
        Ok(ValidatedSlot {
            owner: model_type.clone().unbind(),
            offset: NonZeroUsize::new(offset).ok_or_else(|| {
                PyErr::new::<PyTypeError, _>(format!(
                    "generated attribute {name:?} has a zero slot offset"
                ))
            })?,
        })
    }
}

#[cfg(not(all(Py_3_11, not(any(PyPy, GraalPy, Py_GIL_DISABLED)))))]
fn inspect_native_slot(
    model_type: &Bound<'_, PyType>,
    name: &str,
    descriptor: &Bound<'_, PyAny>,
) -> PyResult<ValidatedSlot> {
    let member_descriptor_type = PyModule::import(model_type.py(), "types")?
        .getattr("MemberDescriptorType")?
        .cast_into::<PyType>()?;
    let owner = descriptor.getattr("__objclass__")?;
    let descriptor_name = descriptor.getattr("__name__")?;
    if !descriptor.get_type().is(&member_descriptor_type)
        || !owner.is(model_type)
        || descriptor_name.cast::<PyString>()?.to_str()? != name
        || !descriptor.hasattr("__get__")?
        || !descriptor.hasattr("__set__")?
    {
        return Err(PyErr::new::<PyTypeError, _>(format!(
            "generated attribute {name:?} must be an owned data descriptor"
        )));
    }
    Ok(ValidatedSlot {
        owner: model_type.clone().unbind(),
        descriptor: descriptor.clone().unbind(),
    })
}

fn parse_node(
    py: Python<'_>,
    descriptor: &Bound<'_, PyAny>,
    node_count: usize,
    schemas: &mut HashMap<Vec<u8>, Arc<PreparedSchema>>,
) -> PyResult<ConversionNode> {
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
                item: NodeId::parse(descriptor.get_item(1)?.extract()?, node_count)?,
            })
        }
        "dict" => {
            require_arity(descriptor, 2, "dict node")?;
            Ok(ConversionNode::Dict {
                value: NodeId::parse(descriptor.get_item(1)?.extract()?, node_count)?,
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
        "union" => parse_union_node(descriptor, node_count),
        "model" => parse_model_node(py, descriptor, node_count, schemas),
        "root" => {
            require_arity(descriptor, 4, "root model node")?;
            let model_type = descriptor.get_item(1)?.cast_into::<PyType>()?.unbind();
            let branch_schema = load_branch_schema(&descriptor.get_item(3)?, schemas)?;
            Ok(ConversionNode::Root {
                branch_schema,
                root_attribute: ModelAttribute::compile(py, model_type.bind(py), "root")?,
                model_type,
                value: NodeId::parse(descriptor.get_item(2)?.extract()?, node_count)?,
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

fn parse_union_node(
    descriptor: &Bound<'_, PyTuple>,
    node_count: usize,
) -> PyResult<ConversionNode> {
    require_arity(descriptor, 4, "union node")?;
    let branches = descriptor.get_item(1)?.cast_into::<PyTuple>()?;
    let branches = branches
        .iter()
        .map(|branch| NodeId::parse(branch.extract::<usize>()?, node_count))
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
            let branch =
                BranchOrdinal::parse(entry.get_item(1)?.extract::<usize>()?, branches.len())?;
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

fn parse_model_node(
    py: Python<'_>,
    descriptor: &Bound<'_, PyTuple>,
    node_count: usize,
    schemas: &mut HashMap<Vec<u8>, Arc<PreparedSchema>>,
) -> PyResult<ConversionNode> {
    require_arity(descriptor, 5, "model node")?;
    let model_type = descriptor.get_item(1)?.cast_into::<PyType>()?;
    let branch_schema = load_branch_schema(&descriptor.get_item(4)?, schemas)?;
    let namespace = model_type.getattr("__dict__")?;
    let field_descriptors = descriptor.get_item(2)?.cast_into::<PyTuple>()?;
    let mut fields = Vec::with_capacity(field_descriptors.len());
    for field in field_descriptors.iter() {
        let field = field.cast_into::<PyTuple>()?;
        require_arity(&field, 4, "model field")?;
        let json_name = field.get_item(0)?.extract::<String>()?;
        let py_name_object = field.get_item(1)?.cast_into::<PyString>()?;
        let py_name = py_name_object.to_str()?;
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
        let alias = (py_name != json_name).then(|| py_name.to_owned());
        let attribute = ModelAttribute::compile_descriptor(
            &model_type,
            &py_name_object,
            &namespace.get_item(&py_name_object)?,
        )?;
        fields.push((
            FieldPlan {
                json_name,
                json_prefix: Vec::new(),
                attribute,
                value_node: NodeId::parse(field.get_item(2)?.extract()?, node_count)?,
                presence: if omittable.extract::<bool>()? {
                    FieldPresence::Omittable
                } else {
                    FieldPresence::Required
                },
            },
            alias,
        ));
    }
    let extra_value = descriptor.get_item(3)?;
    let extra_value = if extra_value.is_none() {
        None
    } else {
        Some(NodeId::parse(extra_value.extract::<usize>()?, node_count)?)
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

fn load_branch_schema(
    prepared: &Bound<'_, PyAny>,
    schemas: &mut HashMap<Vec<u8>, Arc<PreparedSchema>>,
) -> PyResult<BranchSchema> {
    let bytes = prepared.cast::<PyBytes>()?;
    if let Some(program) = schemas.get(bytes.as_bytes()) {
        return Ok(BranchSchema {
            program: Arc::clone(program),
        });
    }
    let program = Arc::new(PreparedSchema::load(bytes.as_bytes()).map_err(PyValueError::new_err)?);
    schemas.insert(bytes.as_bytes().to_vec(), Arc::clone(&program));
    Ok(BranchSchema { program })
}
