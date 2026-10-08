//! Conservative proof that conversion entails a prepared schema's validation.

use serde::{Deserialize, Serialize};

use super::{
    ConversionNode, HashSet, JsonInstanceRef, ModelConverterPlan, NodeId, PreparedSchema, Python,
    ScalarKind,
};
use crate::prepared_schema::{JsonType, NodeId as SchemaNodeId, Rule};

impl ModelConverterPlan {
    pub(super) fn implies_prepared_schema(
        &self,
        py: Python<'_>,
        node_id: NodeId,
        schema: &PreparedSchema,
        schema_id: SchemaNodeId,
        active: &mut HashSet<(usize, usize)>,
    ) -> bool {
        if self.leaf_guards[node_id.0]
            .as_ref()
            .is_some_and(|guard| guard == schema.node(schema_id))
        {
            return true;
        }
        let pair = (node_id.0, schema_id.0);
        if !active.insert(pair) {
            return true;
        }
        let node = self.node(node_id);
        let result = match node {
            ConversionNode::Root { value, .. } => {
                self.implies_prepared_schema(py, *value, schema, schema_id, active)
            }
            ConversionNode::Union(branches) => branches
                .iter()
                .all(|branch| self.implies_prepared_schema(py, *branch, schema, schema_id, active)),
            _ => {
                let constraint = schema.node(schema_id);
                let type_valid = constraint.types.as_ref().is_none_or(|types| {
                    if let ConversionNode::Literal { values } = node {
                        return values.iter().all(|value| {
                            types.iter().any(|kind| {
                                kind.accepts(JsonInstanceRef::from_python(value.bind(py)))
                            })
                        });
                    }
                    types.iter().any(|kind| {
                        matches!(
                            (kind, node),
                            (
                                JsonType::String,
                                ConversionNode::Scalar {
                                    kind: ScalarKind::String
                                }
                            ) | (
                                JsonType::Integer | JsonType::Number,
                                ConversionNode::Scalar {
                                    kind: ScalarKind::Integer
                                }
                            ) | (
                                JsonType::Number,
                                ConversionNode::Scalar {
                                    kind: ScalarKind::Number
                                }
                            ) | (
                                JsonType::Boolean,
                                ConversionNode::Scalar {
                                    kind: ScalarKind::Boolean
                                }
                            ) | (
                                JsonType::Null,
                                ConversionNode::Scalar {
                                    kind: ScalarKind::Null
                                }
                            ) | (JsonType::Array, ConversionNode::List { .. })
                                | (
                                    JsonType::Object,
                                    ConversionNode::Model { .. } | ConversionNode::Dict { .. }
                                )
                        )
                    })
                });
                let choices_valid = constraint
                    .choices
                    .as_ref()
                    .is_none_or(|choices| match node {
                        ConversionNode::Literal { values } => values.iter().all(|value| {
                            choices.iter().any(|choice| {
                                JsonInstanceRef::from_python(value.bind(py)).equals(choice)
                            })
                        }),
                        _ => false,
                    });
                type_valid
                    && choices_valid
                    && constraint.rules.iter().all(|rule| match rule {
                        Rule::Ref(id) => {
                            self.implies_prepared_schema(py, node_id, schema, *id, active)
                        }
                        Rule::All(ids) => ids.iter().all(|id| {
                            self.implies_prepared_schema(py, node_id, schema, *id, active)
                        }),
                        Rule::Any(ids) => ids.iter().any(|id| {
                            self.implies_prepared_schema(py, node_id, schema, *id, active)
                        }),
                        Rule::One(ids) => ids.iter().any(|selected| {
                            self.implies_prepared_schema(py, node_id, schema, *selected, active)
                                && ids
                                    .iter()
                                    .filter(|other| other.0 != selected.0)
                                    .all(|other| {
                                        self.excludes_prepared_schema(
                                            py,
                                            node_id,
                                            schema,
                                            *other,
                                            &mut HashSet::new(),
                                        )
                                    })
                        }),
                        Rule::Object {
                            properties,
                            patterns,
                            required,
                            additional,
                        } if patterns.is_empty() => match node {
                            ConversionNode::Model { fields, extra, .. } => {
                                required.iter().all(|name| {
                                    fields
                                        .by_json_name(name)
                                        .is_some_and(|field| !field.presence.is_omittable())
                                }) && fields.iter().all(|field| {
                                    let constraint = properties
                                        .iter()
                                        .find(|(name, _)| name == &field.json_name)
                                        .map(|(_, node)| *node)
                                        .or(*additional);
                                    constraint.is_none_or(|constraint| {
                                        self.implies_prepared_schema(
                                            py,
                                            field.value_node,
                                            schema,
                                            constraint,
                                            active,
                                        )
                                    })
                                }) && extra.as_ref().is_none_or(|extra| {
                                    additional.is_none_or(|constraint| {
                                        self.implies_prepared_schema(
                                            py,
                                            extra.value_node,
                                            schema,
                                            constraint,
                                            active,
                                        )
                                    }) && properties
                                        .iter()
                                        .filter(|(name, _)| fields.by_json_name(name).is_none())
                                        .all(|(_, constraint)| {
                                            self.implies_prepared_schema(
                                                py,
                                                extra.value_node,
                                                schema,
                                                *constraint,
                                                active,
                                            )
                                        })
                                })
                            }
                            ConversionNode::Dict { value } => {
                                required.is_empty()
                                    && additional.is_none_or(|constraint| {
                                        self.implies_prepared_schema(
                                            py, *value, schema, constraint, active,
                                        )
                                    })
                                    && properties.iter().all(|(_, constraint)| {
                                        self.implies_prepared_schema(
                                            py,
                                            *value,
                                            schema,
                                            *constraint,
                                            active,
                                        )
                                    })
                            }
                            _ => false,
                        },
                        Rule::Array { prefix, items } => match node {
                            ConversionNode::List { item } => {
                                prefix.iter().all(|constraint| {
                                    self.implies_prepared_schema(
                                        py,
                                        *item,
                                        schema,
                                        *constraint,
                                        active,
                                    )
                                }) && items.is_none_or(|constraint| {
                                    self.implies_prepared_schema(
                                        py, *item, schema, constraint, active,
                                    )
                                })
                            }
                            _ => false,
                        },
                        _ => false,
                    })
            }
        };
        active.remove(&pair);
        result
    }
}

impl ModelConverterPlan {
    /// Give each constrained scalar field its own converter identity. Shared
    /// primitive nodes must not inherit a constraint from an unrelated field.
    pub(super) fn leaf_guard_patches(&self) -> Vec<GuardPatch> {
        use super::SchemaSource;
        let mut jobs = Vec::new();
        for (index, node) in self.nodes.iter().enumerate() {
            let (schema, fields, root) = match node {
                ConversionNode::Model {
                    branch_schema,
                    fields,
                    ..
                } => (&branch_schema.source, Some(fields), None),
                ConversionNode::Root {
                    branch_schema,
                    value,
                    ..
                } => (&branch_schema.source, None, Some(*value)),
                _ => continue,
            };
            let SchemaSource::Prepared(schema) = schema else {
                continue;
            };
            let constraint = schema.node(SchemaNodeId(0));
            if let Some(fields) = fields {
                for rule in &constraint.rules {
                    if let Rule::Object { properties, .. } = rule {
                        for (field_index, field) in fields.iter().enumerate() {
                            if let Some((_, id)) =
                                properties.iter().find(|(name, _)| name == &field.json_name)
                            {
                                let leaf = schema.node(*id);
                                if leaf.is_leaf()
                                    && !leaf.rules.is_empty()
                                    && matches!(self.node(field.value_node), ConversionNode::Scalar { kind } if !matches!(kind, ScalarKind::Any))
                                {
                                    jobs.push(GuardPatch {
                                        owner: index,
                                        field: Some(field_index),
                                        original: field.value_node.0,
                                        guard: leaf.clone(),
                                    });
                                }
                            }
                        }
                    }
                }
            } else if let Some(root) = root
                && constraint.is_leaf()
                && !constraint.rules.is_empty()
                && matches!(self.node(root), ConversionNode::Scalar { kind } if !matches!(kind, ScalarKind::Any))
            {
                jobs.push(GuardPatch {
                    owner: index,
                    field: None,
                    original: root.0,
                    guard: constraint.clone(),
                });
            }
        }
        jobs
    }

    pub(super) fn install_prepared_plan(
        &mut self,
        py: Python<'_>,
        bytes: Option<&[u8]>,
    ) -> super::PyResult<()> {
        let prepared = if let Some(bytes) = bytes {
            if self.nodes.iter().any(|node| matches!(node,
                ConversionNode::Model { branch_schema, .. } | ConversionNode::Root { branch_schema, .. }
                    if matches!(branch_schema.source, super::SchemaSource::Raw(_)))) {
                return Err(super::PyValueError::new_err("Prepared model binding requires prepared schemas for every model"));
            }
            let prepared: PreparedPlan = serde_json::from_slice(bytes).map_err(|error| {
                super::PyValueError::new_err(format!("Invalid prepared model plan: {error}"))
            })?;
            if prepared.version != 1
                || prepared.base_nodes != self.nodes.len()
                || prepared.conversion_validates.len() != self.nodes.len() + prepared.guards.len()
            {
                return Err(super::PyValueError::new_err(
                    "Incompatible prepared model plan; rebuild the module",
                ));
            }
            prepared
        } else {
            PreparedPlan {
                version: 1,
                base_nodes: self.nodes.len(),
                guards: self.leaf_guard_patches(),
                conversion_validates: Vec::new(),
                json_keys: self
                    .nodes
                    .iter()
                    .enumerate()
                    .filter_map(|(index, node)| {
                        let ConversionNode::Model { fields, .. } = node else {
                            return None;
                        };
                        Some((
                            index,
                            fields
                                .iter()
                                .map(|field| {
                                    let mut prefix = serde_json::to_string(&field.json_name)
                                        .expect("string serialization cannot fail");
                                    prefix.push(':');
                                    prefix
                                })
                                .collect(),
                        ))
                    })
                    .collect(),
            }
        };
        let mut keyed = HashSet::new();
        for (owner, prefixes) in &prepared.json_keys {
            let Some(ConversionNode::Model { fields, .. }) = self.nodes.get_mut(*owner) else {
                return Err(super::PyValueError::new_err(
                    "Invalid prepared JSON key owner",
                ));
            };
            if !keyed.insert(*owner) || prefixes.len() != fields.len() {
                return Err(super::PyValueError::new_err(
                    "Prepared JSON keys do not match model fields",
                ));
            }
            for (field, prefix) in fields.serialized.iter_mut().zip(prefixes) {
                let name = prefix
                    .strip_suffix(':')
                    .and_then(|name| serde_json::from_str::<String>(name).ok());
                if name.as_deref() != Some(field.json_name.as_str()) {
                    return Err(super::PyValueError::new_err(
                        "Invalid prepared JSON field key",
                    ));
                }
                field.json_prefix = prefix.as_bytes().to_vec();
            }
        }
        if self.nodes.iter().enumerate().any(|(index, node)| {
            matches!(node, ConversionNode::Model { .. }) && !keyed.contains(&index)
        }) {
            return Err(super::PyValueError::new_err(
                "Prepared model is missing JSON field keys",
            ));
        }
        for patch in &prepared.guards {
            let Some(ConversionNode::Scalar { kind }) = self.nodes.get(patch.original) else {
                return Err(super::PyValueError::new_err(
                    "Prepared guard must reference a scalar converter",
                ));
            };
            if !patch.guard.is_leaf() {
                return Err(super::PyValueError::new_err(
                    "Prepared scalar guard contains non-scalar rules",
                ));
            }
            let node = ConversionNode::Scalar { kind: *kind };
            let id = NodeId(self.nodes.len());
            match self.nodes.get_mut(patch.owner) {
                Some(ConversionNode::Model { fields, .. }) => {
                    let field = patch
                        .field
                        .and_then(|index| fields.serialized.get_mut(index))
                        .ok_or_else(|| {
                            super::PyValueError::new_err("Invalid prepared field guard")
                        })?;
                    if field.value_node.0 != patch.original {
                        return Err(super::PyValueError::new_err(
                            "Prepared field guard does not match model",
                        ));
                    }
                    field.value_node = id;
                }
                Some(ConversionNode::Root { value, .. })
                    if patch.field.is_none() && value.0 == patch.original =>
                {
                    *value = id
                }
                _ => return Err(super::PyValueError::new_err("Invalid prepared guard owner")),
            }
            self.nodes.push(node);
            self.leaf_guards.push(Some(patch.guard.clone()));
            self.conversion_validates.push(false);
        }
        if bytes.is_some() {
            self.conversion_validates
                .clone_from(&prepared.conversion_validates);
        } else {
            for index in 0..self.nodes.len() {
                let schema = match &self.nodes[index] {
                    ConversionNode::Model { branch_schema, .. }
                    | ConversionNode::Root { branch_schema, .. } => &branch_schema.source,
                    _ => continue,
                };
                if let super::SchemaSource::Prepared(schema) = schema {
                    self.conversion_validates[index] = self.implies_prepared_schema(
                        py,
                        NodeId(index),
                        schema,
                        SchemaNodeId(0),
                        &mut HashSet::new(),
                    );
                }
            }
        }
        self.prepared_plan = Some(PreparedPlan {
            conversion_validates: self.conversion_validates.clone(),
            ..prepared
        });
        Ok(())
    }

    pub(crate) fn prepared_plan_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(
            self.prepared_plan
                .as_ref()
                .expect("model plan prepared at construction"),
        )
    }

    /// Prove disjoint union alternatives from required literal fields. Failure
    /// to prove exclusion simply retains the general validator.
    fn excludes_prepared_schema(
        &self,
        py: Python<'_>,
        node_id: NodeId,
        schema: &PreparedSchema,
        schema_id: SchemaNodeId,
        active: &mut HashSet<(usize, usize)>,
    ) -> bool {
        let pair = (node_id.0, schema_id.0);
        if !active.insert(pair) {
            return false;
        }
        let node = self.node(node_id);
        let constraint = schema.node(schema_id);
        let result = match node {
            ConversionNode::Root { value, .. } => {
                self.excludes_prepared_schema(py, *value, schema, schema_id, active)
            }
            ConversionNode::Union(branches) => branches.iter().all(|branch| {
                self.excludes_prepared_schema(py, *branch, schema, schema_id, active)
            }),
            _ => {
                let literals_disjoint = if let ConversionNode::Literal { values } = node {
                    constraint.choices.as_ref().is_some_and(|choices| {
                        values.iter().all(|value| {
                            choices.iter().all(|choice| {
                                !JsonInstanceRef::from_python(value.bind(py)).equals(choice)
                            })
                        })
                    })
                } else {
                    false
                };
                literals_disjoint
                    || constraint.rules.iter().any(|rule| match rule {
                        Rule::False => true,
                        Rule::Ref(child) => {
                            self.excludes_prepared_schema(py, node_id, schema, *child, active)
                        }
                        Rule::All(children) => children.iter().any(|child| {
                            self.excludes_prepared_schema(py, node_id, schema, *child, active)
                        }),
                        Rule::Any(children) | Rule::One(children) => children.iter().all(|child| {
                            self.excludes_prepared_schema(py, node_id, schema, *child, active)
                        }),
                        Rule::Object { properties, .. } => match node {
                            ConversionNode::Model { fields, .. } => {
                                properties.iter().any(|(name, child)| {
                                    fields.by_json_name(name).is_some_and(|field| {
                                        !field.presence.is_omittable()
                                            && self.excludes_prepared_schema(
                                                py,
                                                field.value_node,
                                                schema,
                                                *child,
                                                active,
                                            )
                                    })
                                })
                            }
                            _ => false,
                        },
                        _ => false,
                    })
            }
        };
        active.remove(&pair);
        result
    }
}

/// Only portable decisions are persisted. Python classes and slot offsets are
/// rebound against the installed interpreter when the module is imported.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PreparedPlan {
    version: u32,
    base_nodes: usize,
    guards: Vec<GuardPatch>,
    conversion_validates: Vec<bool>,
    json_keys: Vec<(usize, Vec<String>)>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GuardPatch {
    owner: usize,
    field: Option<usize>,
    original: usize,
    guard: crate::prepared_schema::Node,
}
