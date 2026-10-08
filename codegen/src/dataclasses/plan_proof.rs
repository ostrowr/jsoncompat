//! Build-time proof that conversion checks imply schema validation.
use super::plan::{ConversionNode, ModelConverterPlan, NodeId, ScalarKind};
use crate::prepared_schema::{JsonType, NodeId as SchemaNodeId, PreparedSchema, Rule};
use jsonschema::InstanceRef as JsonInstanceRef;
use std::collections::HashSet;

impl ModelConverterPlan {
    pub(super) fn implies_prepared_schema(
        &self,
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
                self.implies_prepared_schema(*value, schema, schema_id, active)
            }
            ConversionNode::Union(branches) => branches
                .iter()
                .all(|branch| self.implies_prepared_schema(*branch, schema, schema_id, active)),
            _ => {
                let constraint = schema.node(schema_id);
                let type_valid = constraint.types.as_ref().is_none_or(|types| {
                    if let ConversionNode::Literal { values } = node {
                        return values.iter().all(|value| {
                            types
                                .iter()
                                .any(|kind| kind.accepts(JsonInstanceRef::from_serde(value)))
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
                            choices
                                .iter()
                                .any(|choice| JsonInstanceRef::from_serde(value).equals(choice))
                        }),
                        _ => false,
                    });
                type_valid
                    && choices_valid
                    && constraint.rules.iter().all(|rule| match rule {
                        Rule::Ref(id) => self.implies_prepared_schema(node_id, schema, *id, active),
                        Rule::All(ids) => ids
                            .iter()
                            .all(|id| self.implies_prepared_schema(node_id, schema, *id, active)),
                        Rule::Any(ids) => ids
                            .iter()
                            .any(|id| self.implies_prepared_schema(node_id, schema, *id, active)),
                        // Equal checks may share a node, but oneOf counts
                        // branch occurrences, including repeated schemas.
                        Rule::One(ids) => {
                            ids.iter().enumerate().any(|(selected_index, selected)| {
                                self.implies_prepared_schema(node_id, schema, *selected, active)
                                    && ids
                                        .iter()
                                        .enumerate()
                                        .filter(|(index, _)| *index != selected_index)
                                        .all(|(_, other)| {
                                            self.excludes_prepared_schema(
                                                node_id,
                                                schema,
                                                *other,
                                                &mut HashSet::new(),
                                            )
                                        })
                            })
                        }
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
                                            field.value_node,
                                            schema,
                                            constraint,
                                            active,
                                        )
                                    })
                                }) && extra.as_ref().is_none_or(|extra| {
                                    additional.is_none_or(|constraint| {
                                        self.implies_prepared_schema(
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
                                            *value, schema, constraint, active,
                                        )
                                    })
                                    && properties.iter().all(|(_, constraint)| {
                                        self.implies_prepared_schema(
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
                                    self.implies_prepared_schema(*item, schema, *constraint, active)
                                }) && items.is_none_or(|constraint| {
                                    self.implies_prepared_schema(*item, schema, constraint, active)
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
    /// Prove disjoint union alternatives from required literal fields. Failure
    /// to prove exclusion simply retains the general validator.
    fn excludes_prepared_schema(
        &self,
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
                self.excludes_prepared_schema(*value, schema, schema_id, active)
            }
            ConversionNode::Union(branches) => branches
                .iter()
                .all(|branch| self.excludes_prepared_schema(*branch, schema, schema_id, active)),
            _ => {
                let literals_disjoint = if let ConversionNode::Literal { values } = node {
                    constraint.choices.as_ref().is_some_and(|choices| {
                        values.iter().all(|value| {
                            choices
                                .iter()
                                .all(|choice| !JsonInstanceRef::from_serde(value).equals(choice))
                        })
                    })
                } else {
                    false
                };
                literals_disjoint
                    || constraint.rules.iter().any(|rule| match rule {
                        Rule::False => true,
                        Rule::Ref(child) => {
                            self.excludes_prepared_schema(node_id, schema, *child, active)
                        }
                        Rule::All(children) => children.iter().any(|child| {
                            self.excludes_prepared_schema(node_id, schema, *child, active)
                        }),
                        Rule::Any(children) | Rule::One(children) => children.iter().all(|child| {
                            self.excludes_prepared_schema(node_id, schema, *child, active)
                        }),
                        Rule::Object { properties, .. } => match node {
                            ConversionNode::Model { fields, .. } => {
                                properties.iter().any(|(name, child)| {
                                    fields.by_json_name(name).is_some_and(|field| {
                                        !field.presence.is_omittable()
                                            && self.excludes_prepared_schema(
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
