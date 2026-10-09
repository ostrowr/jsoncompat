//! Convert Python and parsed JSON values into immutable generated models.
use super::*;

impl ModelConverterPy {
    pub(super) fn convert(
        &self,
        py: Python<'_>,
        node_id: NodeId,
        value: &Bound<'_, PyAny>,
        union_selection: UnionSelection,
        remaining_depth: u16,
        active_containers: ActiveContainers<'_>,
    ) -> ConversionResult<Py<PyAny>> {
        if remaining_depth == 0 {
            return Err(ConversionFailure::Mismatch(ConversionMismatch::Depth));
        }
        let node = self.node(node_id);
        if union_selection != UnionSelection::FirstRepresentable
            && let Some(guard) = &self.leaf_guards[node_id.0]
        {
            let ConversionNode::Scalar { kind } = node else {
                unreachable!("prepared guards reference scalar converters")
            };
            // A wrong scalar type is a branch mismatch, not a raised Python
            // exception: an enclosing union must still try its other branches.
            let normalized = convert_scalar(py, *kind, value)?;
            if !guard.accepts_leaf(JsonInstanceRef::from_python(normalized.bind(py))) {
                return Err(ConversionFailure::Mismatch(ConversionMismatch::Literal));
            }
            return Ok(normalized);
        }
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

    pub(super) fn convert_node(
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

    pub(super) fn convert_direct(
        &self,
        py: Python<'_>,
        node_id: NodeId,
        value: &Bound<'_, PyAny>,
        remaining_depth: u16,
        json_shape: &mut JsonShape,
        active_containers: ActiveContainers<'_>,
    ) -> ConversionResult<Py<PyAny>> {
        if remaining_depth == 0 {
            return Err(ConversionFailure::Mismatch(ConversionMismatch::Depth));
        }
        let node = self.node(node_id);
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

    pub(super) fn convert_direct_node(
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
                if value.get_type().is(model_type.bind(py)) {
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

    pub(super) fn convert_missing_field_value(
        &self,
        py: Python<'_>,
        field: &FieldPlan,
    ) -> Result<Py<PyAny>, ConversionMismatch> {
        if field.presence.is_omittable() {
            return Ok(self.missing_sentinel.clone_ref(py));
        }
        Err(ConversionMismatch::MissingField(field.json_name.clone()))
    }

    pub(super) fn freeze_list(&self, py: Python<'_>, items: Vec<Py<PyAny>>) -> PyResult<Py<PyAny>> {
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
    pub(super) fn freeze_normalized_dict(
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
    pub(super) fn freeze_dict_pairs(
        &self,
        py: Python<'_>,
        items: Vec<FrozenDictPair>,
    ) -> PyResult<Py<PyAny>> {
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

    pub(super) fn frozen_dict_items<'py>(
        &self,
        value: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyTuple>> {
        self.frozen_dict_items_attribute
            .get(value)?
            .cast_into::<PyTuple>()
            .map_err(|_| {
                PyErr::new::<PyTypeError, _>("generated immutable mapping storage must be a tuple")
            })
    }

    pub(super) fn freeze_python_json_value(
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

    pub(super) fn freeze_jiter_json_value(
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

    pub(super) fn convert_list(
        &self,
        py: Python<'_>,
        item_node: NodeId,
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
    pub(super) fn convert_dict(
        &self,
        py: Python<'_>,
        value_node: NodeId,
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
    pub(super) fn convert_union(
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

    pub(super) fn node_matches_kind(
        &self,
        py: Python<'_>,
        node_id: NodeId,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<bool> {
        let node = self.node(node_id);
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

    pub(super) fn node_can_represent_python_value(
        &self,
        py: Python<'_>,
        node_id: NodeId,
        value: &Bound<'_, PyAny>,
        remaining_depth: u16,
    ) -> PyResult<bool> {
        if remaining_depth == 0 {
            return Ok(false);
        }
        let node = self.node(node_id);
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
    pub(super) fn convert_model(
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
    pub(super) fn convert_root_kwargs(
        &self,
        py: Python<'_>,
        model_type: &Py<PyType>,
        value_node: NodeId,
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
    pub(super) fn convert_model_kwargs(
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

    pub(super) fn convert_jiter(
        &self,
        py: Python<'_>,
        node_id: NodeId,
        value: &JiterJsonValue<'_>,
        union_selection: UnionSelection,
    ) -> ConversionResult<Py<PyAny>> {
        let node = self.node(node_id);
        if union_selection != UnionSelection::FirstRepresentable
            && let Some(guard) = &self.leaf_guards[node_id.0]
            && !guard.accepts_jiter_leaf(value)
        {
            return Err(ConversionFailure::Mismatch(ConversionMismatch::Literal));
        }
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

    pub(super) fn convert_jiter_union_value(
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
    pub(super) fn convert_jiter_model_value(
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

    pub(super) fn jiter_node_matches_kind(
        &self,
        node_id: NodeId,
        value: &JiterJsonValue<'_>,
    ) -> bool {
        let node = self.node(node_id);
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

    pub(super) fn jiter_node_can_represent_value(
        &self,
        node_id: NodeId,
        value: &JiterJsonValue<'_>,
        remaining_depth: u16,
    ) -> PyResult<bool> {
        if remaining_depth == 0 {
            return Ok(false);
        }
        let node = self.node(node_id);
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
}
