//! Track successful applicator annotations only for unevaluated* checks.
//!
//! The common path needs no annotation allocation. Each query asks whether a
//! property or item was evaluated at this instance location; child-instance
//! annotations never leak into the parent.

use super::{Evaluation, InstanceRef, MAX_DEPTH, NodeId, PreparedSchema, Rule};

#[derive(Clone, Copy)]
pub(super) enum Location<'a> {
    Property(&'a str),
    Item(usize),
}

impl PreparedSchema {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn evaluates(
        &self,
        id: NodeId,
        value: InstanceRef<'_>,
        location: Location<'_>,
        depth: usize,
        context: &mut Evaluation,
        active: &mut Vec<usize>,
        include_unevaluated: bool,
    ) -> bool {
        if depth >= MAX_DEPTH {
            context.incomplete = true;
            return false;
        }
        if active.contains(&id.0) {
            return false;
        }
        active.push(id.0);
        let evaluated = self.nodes[id.0].rules.iter().any(|rule| match rule {
            Rule::Ref(child) => {
                self.evaluates(*child, value, location, depth + 1, context, active, true)
            }
            Rule::All(children) => children.iter().any(|child| {
                self.evaluates(*child, value, location, depth + 1, context, active, true)
            }),
            Rule::Any(children) | Rule::One(children) => children.iter().any(|child| {
                self.accepts(*child, value, depth + 1, context)
                    && self.evaluates(*child, value, location, depth + 1, context, active, true)
            }),
            Rule::If {
                condition,
                then_node,
                else_node,
            } => {
                if self.accepts(*condition, value, depth + 1, context) {
                    self.evaluates(
                        *condition,
                        value,
                        location,
                        depth + 1,
                        context,
                        active,
                        true,
                    ) || then_node.is_some_and(|child| {
                        self.evaluates(child, value, location, depth + 1, context, active, true)
                    })
                } else {
                    else_node.is_some_and(|child| {
                        self.evaluates(child, value, location, depth + 1, context, active, true)
                    })
                }
            }
            Rule::DependentSchemas(entries) => value.as_object().is_some_and(|object| {
                entries.iter().any(|(key, child)| {
                    object.contains_key(key)
                        && self.evaluates(*child, value, location, depth + 1, context, active, true)
                })
            }),
            Rule::Object {
                properties,
                patterns,
                additional,
                ..
            } => match location {
                Location::Property(key) => {
                    additional.is_some()
                        || properties
                            .binary_search_by(|(candidate, _)| candidate.as_str().cmp(key))
                            .is_ok()
                        || patterns
                            .iter()
                            .any(|(pattern, _)| self.pattern_matches(*pattern, key, context))
                }
                _ => false,
            },
            Rule::Array { prefix, items } => {
                matches!(location, Location::Item(index) if index < prefix.len() || items.is_some())
            }
            Rule::Contains { node, .. } => match location {
                Location::Item(index) => value
                    .as_array()
                    .and_then(|array| array.iter().nth(index))
                    .is_some_and(|item| self.accepts_child(*node, item, depth + 1, context)),
                _ => false,
            },
            Rule::UnevaluatedProperties(_) => {
                include_unevaluated && matches!(location, Location::Property(_))
            }
            Rule::UnevaluatedItems(_) => {
                include_unevaluated && matches!(location, Location::Item(_))
            }
            _ => false,
        });
        active.pop();
        evaluated
    }
}
