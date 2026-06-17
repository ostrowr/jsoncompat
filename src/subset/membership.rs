//! Membership-oracle bookkeeping for subset proofs.
//!
//! The structural prover sometimes asks the schema evaluator about concrete
//! values.  Those answers are only sound in one polarity when the evaluator
//! intentionally fails closed (unsupported regexes, recursive re-entry) or can
//! over-accept (unsupported pattern properties under negation).  This module
//! keeps that polarity accounting in one place so proof rules do not have to
//! duplicate it.

use crate::SchemaNode;
use json_schema_ast::{NodeId, PatternSupport, SchemaNodeKind};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub(crate) struct SubschemaCheckContext {
    active_pairs: HashMap<(NodeId, NodeId), usize>,
    pub(super) acceptance_deviations: HashMap<NodeId, AcceptanceDeviation>,
    productive_depth: usize,
    pub(super) assume_subset_omits_undeclared_properties: bool,
}

impl SubschemaCheckContext {
    /// Context variant used when checking values emitted by serializers that
    /// omit undeclared object properties.
    pub(super) fn for_emitted_values() -> Self {
        Self {
            assume_subset_omits_undeclared_properties: true,
            ..Self::default()
        }
    }

    /// Run a nested proof in a productive (property/item) position.
    pub(super) fn with_productive_frame<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        self.productive_depth += 1;
        let result = f(self);
        self.productive_depth -= 1;
        result
    }

    /// If this pair is already on the goal stack, report whether re-entry is
    /// guarded by a productive edge and can be accepted coinductively.
    pub(super) fn recursion_reentry_is_guarded(&self, key: (NodeId, NodeId)) -> Option<bool> {
        self.active_pairs
            .get(&key)
            .map(|active_depth| self.productive_depth > *active_depth)
    }

    /// Register a goal-stack frame for the duration of `f`.
    ///
    /// Callers check `recursion_reentry_is_guarded` first; this helper owns the
    /// push/pop symmetry for non-recursive entries.
    pub(super) fn with_recursion_pair<T>(
        &mut self,
        key: (NodeId, NodeId),
        f: impl FnOnce(&mut Self) -> T,
    ) -> T {
        self.active_pairs.insert(key, self.productive_depth);
        let result = f(self);
        self.active_pairs.remove(&key);
        result
    }

    pub(super) fn superset_contains_value(&mut self, sup: &SchemaNode, value: &Value) -> bool {
        !self.schema_may_over_accept_values(sup) && sup.accepts_value(value)
    }

    pub(super) fn superset_contains_value_set(
        &mut self,
        sup: &SchemaNode,
        values: &[Value],
    ) -> bool {
        values
            .iter()
            .all(|value| self.superset_contains_value(sup, value))
    }

    /// Return true only when the internal evaluator can soundly prove that a
    /// concrete value is rejected by `schema`. This is the dual of
    /// `superset_contains_value`: under-acceptance would make a negative
    /// evaluator result unsafe, while over-acceptance is harmless for
    /// rejection.
    pub(super) fn schema_definitely_rejects_value(
        &mut self,
        schema: &SchemaNode,
        value: &Value,
    ) -> bool {
        !schema_acceptance_deviation_cached(schema, &mut self.acceptance_deviations)
            .may_under_accept
            && !schema.accepts_value(value)
    }

    /// Check a finite syntactic upper bound for `sub` against `sup`.
    ///
    /// Each candidate is either proven unreachable from `sub` (using only a
    /// sound negative evaluator result) or proven accepted by `sup` (using
    /// only a sound positive evaluator result).  Keeping this polarity-sensitive
    /// pattern here avoids subtle copy/paste mistakes in finite proof rules.
    pub(super) fn finite_upper_bound_fits_target(
        &mut self,
        sub: &SchemaNode,
        sup: &SchemaNode,
        values: &[Value],
    ) -> bool {
        values.iter().all(|value| {
            self.schema_definitely_rejects_value(sub, value)
                || self.superset_contains_value(sup, value)
        })
    }

    fn schema_may_over_accept_values(&mut self, schema: &SchemaNode) -> bool {
        schema_acceptance_deviation_cached(schema, &mut self.acceptance_deviations).may_over_accept
    }
}

/// Whether evaluator rejection for this schema may be a false negative.
pub(super) fn schema_may_under_accept_values(schema: &SchemaNode) -> bool {
    schema_acceptance_deviation(schema).may_under_accept
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct AcceptanceDeviation {
    may_under_accept: bool,
    may_over_accept: bool,
}

impl AcceptanceDeviation {
    const NONE: Self = Self {
        may_under_accept: false,
        may_over_accept: false,
    };
    const UNDER: Self = Self {
        may_under_accept: true,
        may_over_accept: false,
    };
    const BOTH: Self = Self {
        may_under_accept: true,
        may_over_accept: true,
    };

    fn combine(self, other: Self) -> Self {
        Self {
            may_under_accept: self.may_under_accept || other.may_under_accept,
            may_over_accept: self.may_over_accept || other.may_over_accept,
        }
    }

    fn inverted(self) -> Self {
        Self {
            may_under_accept: self.may_over_accept,
            may_over_accept: self.may_under_accept,
        }
    }

    fn is_exact(self) -> bool {
        !self.may_under_accept && !self.may_over_accept
    }
}

fn schema_acceptance_deviation(schema: &SchemaNode) -> AcceptanceDeviation {
    schema_acceptance_deviation_cached(schema, &mut HashMap::new())
}

fn schema_acceptance_deviation_cached(
    schema: &SchemaNode,
    memo: &mut HashMap<NodeId, AcceptanceDeviation>,
) -> AcceptanceDeviation {
    schema_acceptance_deviation_with_state(schema, memo, &mut HashSet::new())
}

fn schema_acceptance_deviation_with_state(
    schema: &SchemaNode,
    memo: &mut HashMap<NodeId, AcceptanceDeviation>,
    active: &mut HashSet<NodeId>,
) -> AcceptanceDeviation {
    if let Some(deviation) = memo.get(&schema.id()) {
        return *deviation;
    }
    if !active.insert(schema.id()) {
        // The evaluator fails closed on same-value recursive re-entry. That can
        // matter to anti-monotone parents such as `not`, but the recursive edge
        // itself does not directly make a positive membership check accept too
        // much.
        return AcceptanceDeviation::UNDER;
    }

    let deviation =
        match schema.kind() {
            SchemaNodeKind::String {
                pattern: Some(pattern),
                ..
            } if pattern.support() == PatternSupport::Unsupported => AcceptanceDeviation::UNDER,
            SchemaNodeKind::Object {
                properties,
                pattern_properties,
                additional,
                property_names,
                ..
            } => {
                let mut child_deviation = AcceptanceDeviation::NONE;
                for property in properties.values() {
                    child_deviation = child_deviation.combine(
                        schema_acceptance_deviation_with_state(property, memo, active),
                    );
                }
                for property in pattern_properties.values() {
                    child_deviation = child_deviation.combine(
                        schema_acceptance_deviation_with_state(&property.schema, memo, active),
                    );
                }
                child_deviation = child_deviation.combine(schema_acceptance_deviation_with_state(
                    additional, memo, active,
                ));
                child_deviation = child_deviation.combine(schema_acceptance_deviation_with_state(
                    property_names,
                    memo,
                    active,
                ));

                if pattern_properties
                    .values()
                    .any(|property| property.pattern.support() == PatternSupport::Unsupported)
                {
                    child_deviation.combine(AcceptanceDeviation::BOTH)
                } else {
                    child_deviation
                }
            }
            SchemaNodeKind::Array {
                prefix_items,
                items,
                contains,
                ..
            } => {
                let mut item_deviation = AcceptanceDeviation::NONE;
                for item in prefix_items {
                    item_deviation = item_deviation
                        .combine(schema_acceptance_deviation_with_state(item, memo, active));
                }
                item_deviation = item_deviation
                    .combine(schema_acceptance_deviation_with_state(items, memo, active));

                match contains {
                    None => item_deviation,
                    Some(contains) => {
                        let contains_deviation =
                            schema_acceptance_deviation_with_state(&contains.schema, memo, active);
                        let mut contains_effect = AcceptanceDeviation::NONE;
                        if contains.count().min() > 0 {
                            contains_effect.may_under_accept = contains_deviation.may_under_accept;
                            contains_effect.may_over_accept = contains_deviation.may_over_accept;
                        }
                        if contains.count().max().is_some() {
                            contains_effect.may_under_accept |= contains_deviation.may_over_accept;
                            contains_effect.may_over_accept |= contains_deviation.may_under_accept;
                        }
                        item_deviation.combine(contains_effect)
                    }
                }
            }
            SchemaNodeKind::AllOf(children) | SchemaNodeKind::AnyOf(children) => children
                .iter()
                .map(|child| schema_acceptance_deviation_with_state(child, memo, active))
                .fold(AcceptanceDeviation::NONE, AcceptanceDeviation::combine),
            SchemaNodeKind::OneOf(children) => {
                let child_deviation = children
                    .iter()
                    .map(|child| schema_acceptance_deviation_with_state(child, memo, active))
                    .fold(AcceptanceDeviation::NONE, AcceptanceDeviation::combine);
                if child_deviation.is_exact() {
                    AcceptanceDeviation::NONE
                } else {
                    AcceptanceDeviation::BOTH
                }
            }
            SchemaNodeKind::Not(child) => {
                schema_acceptance_deviation_with_state(child, memo, active).inverted()
            }
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => {
                let if_deviation = schema_acceptance_deviation_with_state(if_schema, memo, active);
                let branch_deviation =
                    then_schema
                        .as_ref()
                        .map(|schema| schema_acceptance_deviation_with_state(schema, memo, active))
                        .into_iter()
                        .chain(else_schema.as_ref().map(|schema| {
                            schema_acceptance_deviation_with_state(schema, memo, active)
                        }))
                        .fold(AcceptanceDeviation::NONE, AcceptanceDeviation::combine);

                if if_deviation.is_exact() {
                    branch_deviation
                } else {
                    branch_deviation.combine(AcceptanceDeviation::BOTH)
                }
            }
            _ => AcceptanceDeviation::NONE,
        };

    active.remove(&schema.id());
    memo.insert(schema.id(), deviation);
    deviation
}
