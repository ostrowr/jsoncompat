//! Borrow generated model slots as JSON without allocating a value tree.
use super::*;

impl<'converter> ModelProjection<'converter> {
    pub(crate) fn instance<'a, 'py>(&'a self, value: &'a Bound<'py, PyAny>) -> JsonInstanceRef<'a>
    where
        'py: 'a,
    {
        JsonInstanceRef::from_projected_python(value, self.converter.root.0.0, self)
    }

    fn retain<'a>(&'a self, node: NodeId, value: Bound<'a, PyAny>) -> ProjectedPythonValue<'a> {
        let py = value.py();
        let pointer = value.as_ptr();
        self.retained.borrow_mut().push(value.unbind());
        // SAFETY: `retained` owns a reference to this object until the
        // projection is dropped, which is after every projected borrow.
        let borrowed: Borrowed<'a, 'a, PyAny> = unsafe { Borrowed::from_ptr(py, pointer) };
        ProjectedPythonValue::new(node.0, borrowed)
    }

    fn attribute<'a>(
        &'a self,
        value: ProjectedPythonValue<'a>,
        name: &Py<PyString>,
        node: NodeId,
    ) -> Option<ProjectedPythonValue<'a>> {
        let py = value.value().py();
        let attribute = value.value().getattr(name.bind(py)).ok()?;
        Some(self.retain(node, attribute))
    }

    fn model_attribute<'a>(
        &'a self,
        value: ProjectedPythonValue<'a>,
        attribute: &ModelAttribute,
        node: NodeId,
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
        node: NodeId,
        child: *mut ffi::PyObject,
    ) -> Option<ProjectedPythonValue<'a>> {
        // SAFETY: callers obtain `child` as a borrowed entry from a Python
        // container retained by `parent` for the full projection lifetime.
        let child: Borrowed<'a, 'a, PyAny> =
            unsafe { Borrowed::from_ptr_or_opt(parent.value().py(), child) }?;
        Some(ProjectedPythonValue::new(node.0, child))
    }

    fn resolve<'a>(
        &'a self,
        value: ProjectedPythonValue<'a>,
        remaining_depth: u16,
    ) -> ProjectedPythonKind<'a> {
        if remaining_depth == 0 {
            return ProjectedPythonKind::Invalid;
        }
        let node_id = self.converter.projected_node_id(value.node());
        let node = self.converter.node(node_id);
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
                let cache_key = (node_id, value.value().as_ptr() as usize);
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
                        ProjectedPythonValue::new(selected.0, value.value()),
                        remaining_depth - 1,
                    )
                } else {
                    ProjectedPythonKind::Invalid
                }
            }
            ConversionNode::Model { model_type, .. } => {
                let py = value.value().py();
                if value.value().get_type().is(model_type.bind(py)) {
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
                if !value.value().get_type().is(model_type.bind(py)) {
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
            self.converter.projected_node_id(value.node()),
        )?;
        storage.value().cast::<PyTuple>().ok()
    }

    fn extra_mapping<'a>(
        &'a self,
        value: ProjectedPythonValue<'a>,
        extra_attribute: &ModelAttribute,
    ) -> Option<Borrowed<'a, 'a, PyTuple>> {
        let extra = self.model_attribute(
            value,
            extra_attribute,
            self.converter.projected_node_id(value.node()),
        )?;
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
        child_node: NodeId,
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
        child_node: NodeId,
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
        match self
            .converter
            .node(self.converter.projected_node_id(value.node()))
        {
            ConversionNode::List { .. }
            | ConversionNode::Scalar {
                kind: ScalarKind::Any,
                ..
            } => {}
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
        let item = match self
            .converter
            .node(self.converter.projected_node_id(value.node()))
        {
            ConversionNode::List { item } => *item,
            ConversionNode::Scalar {
                kind: ScalarKind::Any,
                ..
            } => self.converter.projected_node_id(value.node()),
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
        match self
            .converter
            .node(self.converter.projected_node_id(value.node()))
        {
            ConversionNode::Scalar {
                kind: ScalarKind::Any,
                ..
            } => self
                .mapping_storage(value)
                .map_or(0, |dictionary| dictionary.len()),
            ConversionNode::Dict { .. } => self
                .mapping_storage(value)
                .map_or(0, |dictionary| dictionary.len()),
            ConversionNode::Model { fields, extra, .. } => {
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
        match self
            .converter
            .node(self.converter.projected_node_id(value.node()))
        {
            ConversionNode::Scalar {
                kind: ScalarKind::Any,
                ..
            } => self
                .mapping_storage(value)
                .is_some_and(|dictionary| Self::mapping_keys_are_strings(value, dictionary)),
            ConversionNode::Dict { .. } => self
                .mapping_storage(value)
                .is_some_and(|dictionary| Self::mapping_keys_are_strings(value, dictionary)),
            ConversionNode::Model {
                extra: Some(extra), ..
            } => self
                .extra_mapping(value, &extra.attribute)
                .is_some_and(|dictionary| Self::mapping_keys_are_strings(value, dictionary)),
            ConversionNode::Model { .. } => true,
            _ => false,
        }
    }

    fn object_get<'a>(
        &'a self,
        value: ProjectedPythonValue<'a>,
        key: &str,
    ) -> Option<ProjectedPythonValue<'a>> {
        match self
            .converter
            .node(self.converter.projected_node_id(value.node()))
        {
            ConversionNode::Scalar {
                kind: ScalarKind::Any,
                ..
            } => {
                let dictionary = self.mapping_storage(value)?;
                self.dict_get(
                    value,
                    dictionary,
                    key,
                    self.converter.projected_node_id(value.node()),
                )
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
        match self
            .converter
            .node(self.converter.projected_node_id(value.node()))
        {
            ConversionNode::Scalar {
                kind: ScalarKind::Any,
                ..
            } => {
                let dictionary = self.mapping_storage(value)?;
                self.dict_next(
                    value,
                    dictionary,
                    &mut state[0],
                    self.converter.projected_node_id(value.node()),
                )
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
