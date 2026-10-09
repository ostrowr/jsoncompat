//! Serialize and materialize immutable model slots with optional inline checks.
use super::*;

impl ModelConverterPy {
    pub(super) fn normalize_output_leaf(
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

    pub(super) fn write_output_leaf(
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
    pub(super) fn insert_pending_json_entry<'py>(
        &self,
        py: Python<'py>,
        entries: &mut BTreeMap<String, (NodeId, Bound<'py, PyAny>)>,
        key: String,
        value_node: NodeId,
        value: Bound<'py, PyAny>,
        remaining_depth: u16,
        active_containers: ActiveContainers<'_>,
    ) -> PyResult<()> {
        let Some((displaced_node, displaced_value)) = entries.insert(key, (value_node, value))
        else {
            return Ok(());
        };
        self.output_overrides.set(true);

        // Materialization proves values before assigning them into a Python
        // dict, so a later canonical-key collision cannot hide an invalid
        // earlier value. Preserve that invariant without taxing the unique-key
        // path: only displaced values are written to a throwaway buffer.
        let mut scratch = Vec::new();
        // Schema constraints apply to the final emitted object. A displaced
        // field still needs structural checks, but its scalar constraints no
        // longer describe the value that will be serialized under this key.
        let checked_output = self.checked_output.replace(false);
        let result = self.write_json_node(
            py,
            displaced_node,
            &displaced_value,
            remaining_depth,
            active_containers,
            &mut scratch,
        );
        self.checked_output.set(checked_output);
        result
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn write_any_json_node<'py>(
        &self,
        py: Python<'py>,
        node_id: NodeId,
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

    pub(super) fn write_json_node<'py>(
        &self,
        py: Python<'py>,
        node_id: NodeId,
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
        let node = self.node(node_id);
        // Exact builtins already have canonical JSON representations.
        // Check their constraints on the borrowed scalar and emit it
        // without constructing another owned Python reference.
        if let ConversionNode::Scalar { kind } = node {
            let guard = self
                .checked_output
                .get()
                .then(|| self.leaf_guards[node_id.0].as_ref())
                .flatten();
            if matches!(kind, ScalarKind::String)
                && let Ok(text) = value.cast_exact::<PyString>()
            {
                let text = text.to_str()?;
                let scalar = JiterJsonValue::Str(std::borrow::Cow::Borrowed(text));
                if guard.is_some_and(|guard| !guard.accepts_jiter_leaf(&scalar)) {
                    return Err(PyValueError::new_err(
                        "Instance does not conform to the JSON schema",
                    ));
                }
                return write_json_string(output, text);
            }
            if matches!(kind, ScalarKind::Integer | ScalarKind::Number)
                && value.is_exact_instance_of::<PyInt>()
                && let Ok(number) = value.extract::<i64>()
            {
                let scalar = JiterJsonValue::Int(number);
                if guard.is_some_and(|guard| !guard.accepts_jiter_leaf(&scalar)) {
                    return Err(PyValueError::new_err(
                        "Instance does not conform to the JSON schema",
                    ));
                }
                return serde_json::to_writer(&mut *output, &number)
                    .map_err(json_serialization_error);
            }
        }
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
    pub(super) fn write_json_node_inner<'py>(
        &self,
        py: Python<'py>,
        node_id: NodeId,
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
                if self.checked_output.get()
                    && let Some(guard) = &self.leaf_guards[node_id.0]
                {
                    let normalized = self
                        .normalize_output_leaf(py, node, value)?
                        .expect("guarded leaf");
                    if !guard.accepts_leaf(JsonInstanceRef::from_python(normalized.bind(py))) {
                        return Err(PyValueError::new_err(
                            "Instance does not conform to the JSON schema",
                        ));
                    }
                    return write_serializable_json_value(
                        output,
                        normalized.bind(py),
                        remaining_depth,
                    );
                }
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
                if !value.get_type().is(model_type.bind(py)) {
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
                        output.extend_from_slice(&field.json_prefix);
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
                if !value.get_type().is(model_type.bind(py)) {
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

    pub(super) fn to_python_value_node(
        &self,
        py: Python<'_>,
        node_id: NodeId,
        value: &Bound<'_, PyAny>,
        remaining_depth: u16,
        active_containers: ActiveContainers<'_>,
    ) -> PyResult<Py<PyAny>> {
        if remaining_depth == 0 {
            return Err(PyErr::new::<PyValueError, _>(
                "generated model serialization exceeds the maximum nesting depth",
            ));
        }
        let node = self.node(node_id);
        if matches!(
            node,
            ConversionNode::List { .. }
                | ConversionNode::Dict { .. }
                | ConversionNode::Model { .. }
                | ConversionNode::Root { .. }
        ) {
            return active_containers.with_pyresult(value, |active_containers| {
                self.to_python_value_node_inner(py, node, value, remaining_depth, active_containers)
            });
        }
        self.to_python_value_node_inner(py, node, value, remaining_depth, active_containers)
    }

    pub(super) fn to_python_value_node_inner(
        &self,
        py: Python<'_>,
        node: &ConversionNode,
        value: &Bound<'_, PyAny>,
        remaining_depth: u16,
        active_containers: ActiveContainers<'_>,
    ) -> PyResult<Py<PyAny>> {
        match node {
            ConversionNode::Scalar {
                kind: ScalarKind::Any,
                ..
            } => copy_python_json_value(py, value, remaining_depth, active_containers),
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
                        active_containers,
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
                        self.to_python_value_node(
                            py,
                            *value_node,
                            &item,
                            remaining_depth - 1,
                            active_containers,
                        )?,
                    )?;
                }
                Ok(output.into_any().unbind())
            }
            ConversionNode::Union(plan) => {
                let mut first_error = None;
                for branch in plan {
                    if self.node_matches_model_value(py, *branch, value)? {
                        match self.to_python_value_node(
                            py,
                            *branch,
                            value,
                            remaining_depth - 1,
                            active_containers,
                        ) {
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
                if !value.get_type().is(model_type.bind(py)) {
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
                            active_containers,
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
                                active_containers,
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
                if !value.get_type().is(model_type.bind(py)) {
                    let expected = model_type.bind(py).name()?.to_str()?.to_owned();
                    return Err(expected_type(&expected, value)?);
                }
                let root = root_attribute.get(value)?;
                self.to_python_value_node(
                    py,
                    *value_node,
                    &root,
                    remaining_depth - 1,
                    active_containers,
                )
            }
        }
    }

    pub(super) fn node_matches_model_value(
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
                value.get_type().is(model_type.bind(py))
            }
        })
    }
}
