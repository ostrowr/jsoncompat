//! Parse directly into generated objects when conversion proves the schema.
//!
//! Scalar guards run before field assignment. Unions and unconstrained values
//! can still use the existing borrowed-value converter for their subtree. This
//! avoids allocating a complete intermediate JSON tree for ordinary models.

use std::borrow::Cow;

use jiter::{Jiter, JiterError, Peek};

use super::*;

struct StreamValidation {
    selection: UnionSelection,
    exact: bool,
}

impl ModelConverterPy {
    pub(crate) fn can_validate_while_parsing(&self) -> bool {
        self.conversion_validates[self.root.0.0]
    }

    pub(crate) fn construct_stream(
        &self,
        py: Python<'_>,
        payload: &[u8],
        checked: bool,
    ) -> PyResult<Option<Py<PyAny>>> {
        let mut parser = Jiter::new(payload);
        let selection = if checked {
            UnionSelection::ValidateAmbiguousBranches
        } else {
            UnionSelection::FirstRepresentable
        };
        let mut validation = StreamValidation {
            selection,
            exact: false,
        };
        match self.stream_node(
            py,
            self.root.0,
            &mut parser,
            &mut validation,
            MAX_MODEL_DEPTH,
        ) {
            Ok(value) => {
                parser
                    .finish()
                    .map_err(|error| stream_error(error, &parser).into_pyerr())?;
                if checked && validation.exact {
                    return crate::construct_exact_model_json(py, payload, self);
                }
                Ok(Some(value))
            }
            Err(ConversionFailure::Mismatch(ConversionMismatch::Depth)) => {
                // Preserve the established decoder's depth semantics while
                // bounding the additional streaming converter's stack use.
                let value = JiterJsonValue::parse(payload, false)
                    .map_err(|error| jiter::map_json_error(payload, &error))?;
                if checked && rounded_integer_float(&value) {
                    crate::construct_exact_model_json(py, payload, self)
                } else if checked {
                    self.construct_jiter_checked(py, &value)
                } else {
                    self.construct_jiter_unchecked(py, &value).map(Some)
                }
            }
            Err(ConversionFailure::Mismatch(_)) if checked => Ok(None),
            Err(error) => Err(error.into_pyerr()),
        }
    }

    fn stream_node(
        &self,
        py: Python<'_>,
        node_id: NodeId,
        parser: &mut Jiter<'_>,
        validation: &mut StreamValidation,
        depth: u16,
    ) -> ConversionResult<Py<PyAny>> {
        if depth == 0 {
            return Err(ConversionFailure::Mismatch(ConversionMismatch::Depth));
        }
        let selection = validation.selection;
        let node = self.node(node_id);
        if let ConversionNode::Root {
            model_type,
            value,
            root_attribute,
            ..
        } = node
        {
            let value = self.stream_node(py, *value, parser, validation, depth - 1)?;
            let instance = allocate_model(py, model_type, &self.object_new)?;
            root_attribute.set_owned(py, &instance, value)?;
            return Ok(instance.unbind());
        }
        let peek = parser.peek().map_err(|error| stream_error(error, parser))?;
        match node {
            ConversionNode::Scalar {
                kind: ScalarKind::String,
            } if peek == Peek::String => {
                let text = parser.known_str().map_err(|error| {
                    ConversionFailure::Raised(PyValueError::new_err(error.to_string()))
                })?;
                if selection != UnionSelection::FirstRepresentable
                    && let Some(guard) = &self.leaf_guards[node_id.0]
                    && !guard.accepts_jiter_leaf(&JiterJsonValue::Str(Cow::Borrowed(text)))
                {
                    return Err(ConversionFailure::Mismatch(ConversionMismatch::Literal));
                }
                if text.len() >= 64 {
                    return Ok(crate::unicode::from_utf8(py, text)?.into_any().unbind());
                }
                Ok(jiter::cached_py_string(py, text).into_any().unbind())
            }
            ConversionNode::List { item } if peek == Peek::Array => {
                let mut values = Vec::new();
                let mut next = parser
                    .known_array()
                    .map_err(|error| stream_error(error, parser))?;
                while next.is_some() {
                    values.push(self.stream_node(py, *item, parser, validation, depth - 1)?);
                    next = parser
                        .array_step()
                        .map_err(|error| stream_error(error, parser))?;
                }
                self.freeze_list(py, values)
                    .map_err(ConversionFailure::Raised)
            }
            ConversionNode::Model {
                model_type,
                fields,
                extra,
                ..
            } if peek == Peek::Object => {
                let instance = allocate_model(py, model_type, &self.object_new)?;
                let mut extra_values = Vec::new();
                let mut extra_keys = HashSet::new();
                let mut count = 0;
                let mut key = parser.known_object().map_err(|error| {
                    ConversionFailure::Raised(PyValueError::new_err(error.to_string()))
                })?;
                while let Some(name) = key {
                    // The serializer emits fields in this order. Matching the
                    // next name directly also makes that common input order
                    // cheap; arbitrary order still uses the complete lookup.
                    let field = fields
                        .serialized
                        .get(count)
                        .filter(|field| field.json_name == name)
                        .or_else(|| fields.by_json_name(name));
                    if let Some(field) = field {
                        if field.attribute.is_set(py, &instance)? {
                            return Err(duplicate_key(name).into());
                        }
                        let value =
                            self.stream_node(py, field.value_node, parser, validation, depth - 1)?;
                        field.attribute.set_owned(py, &instance, value)?;
                        count += 1;
                    } else if let Some(extra) = extra {
                        if !extra_keys.insert(name.to_owned()) {
                            return Err(duplicate_key(name).into());
                        }
                        let name = jiter::cached_py_string(py, name).into_any().unbind();
                        let value =
                            self.stream_node(py, extra.value_node, parser, validation, depth - 1)?;
                        extra_values.push((name, value));
                    } else {
                        return Err(ConversionFailure::Mismatch(
                            ConversionMismatch::UnknownProperty(name.to_owned()),
                        ));
                    }
                    key = parser.next_key().map_err(|error| {
                        ConversionFailure::Raised(PyValueError::new_err(error.to_string()))
                    })?;
                }
                if count != fields.len() {
                    for field in fields {
                        if !field.attribute.is_set(py, &instance)? {
                            let value = self
                                .convert_missing_field_value(py, field)
                                .map_err(ConversionFailure::Mismatch)?;
                            field.attribute.set_owned(py, &instance, value)?;
                        }
                    }
                }
                if let Some(extra) = extra {
                    let value = self.freeze_dict_pairs(py, extra_values)?;
                    extra.attribute.set_owned(py, &instance, value)?;
                }
                Ok(instance.unbind())
            }
            ConversionNode::Dict { value } if peek == Peek::Object => {
                let mut values = Vec::new();
                let mut seen = HashSet::new();
                let mut key = parser.known_object().map_err(|error| {
                    ConversionFailure::Raised(PyValueError::new_err(error.to_string()))
                })?;
                while let Some(name) = key {
                    if !seen.insert(name.to_owned()) {
                        return Err(duplicate_key(name).into());
                    }
                    let name = jiter::cached_py_string(py, name).into_any().unbind();
                    let value = self.stream_node(py, *value, parser, validation, depth - 1)?;
                    values.push((name, value));
                    key = parser.next_key().map_err(|error| {
                        ConversionFailure::Raised(PyValueError::new_err(error.to_string()))
                    })?;
                }
                self.freeze_dict_pairs(py, values)
                    .map_err(ConversionFailure::Raised)
            }
            _ => {
                let value = parser
                    .known_value(peek)
                    .map_err(|error| stream_error(error, parser))?;
                if selection != UnionSelection::FirstRepresentable && rounded_integer_float(&value)
                {
                    validation.exact = true;
                    // Check exact membership after parsing the entire payload.
                    return self.convert_jiter(
                        py,
                        node_id,
                        &value,
                        UnionSelection::FirstRepresentable,
                    );
                }
                self.convert_jiter(py, node_id, &value, selection)
            }
        }
    }
}

fn stream_error(error: JiterError, parser: &Jiter<'_>) -> ConversionFailure {
    ConversionFailure::Raised(PyValueError::new_err(error.description(parser)))
}

/// A fractional JSON token can round onto an integer (including zero). Only
/// those parsed values need the exact-number retry for integer-shaped schemas;
/// ordinary integer tokens stay on the streaming path.
pub(crate) fn rounded_integer_float(value: &JiterJsonValue<'_>) -> bool {
    match value {
        JiterJsonValue::Float(value) => value.fract() == 0.0,
        JiterJsonValue::Array(values) => values.iter().any(rounded_integer_float),
        JiterJsonValue::Object(values) => {
            values.iter().any(|(_, value)| rounded_integer_float(value))
        }
        _ => false,
    }
}
