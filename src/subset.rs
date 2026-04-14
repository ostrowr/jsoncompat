//! Structural subset checks over the resolved schema IR.
//!
//! `is_subschema_of(sub, sup)` answers whether every instance accepted by
//! `sub` is also accepted by `sup`.  The checker is intentionally conservative
//! for hard cases such as regex implication and `oneOf` on the right-hand side.

use crate::{SchemaNode, json_pointer::JsonPointer};
use json_schema_ast::{
    CountRange, IntegerBounds, IntegerMultipleOf, NodeId, NumberBound, NumberBounds,
    NumberMultipleOf, PatternSupport, SchemaNodeKind, json_values_equal,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

mod array;
mod object;
mod scalar;

use object::dependent_requirement_is_guaranteed;
use scalar::{
    StringConstraints, check_enum_inclusion, integer_constraints_subsumed_by_number,
    string_constraints_subsumed,
};

#[derive(Default)]
pub(super) struct SubschemaCheckContext {
    active_pairs: HashMap<(NodeId, NodeId), usize>,
    acceptance_deviations: HashMap<NodeId, AcceptanceDeviation>,
    productive_depth: usize,
    assume_subset_omits_undeclared_properties: bool,
}

impl SubschemaCheckContext {
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

    fn schema_may_over_accept_values(&mut self, schema: &SchemaNode) -> bool {
        schema_acceptance_deviation_cached(schema, &mut self.acceptance_deviations).may_over_accept
    }
}

/// Returns `true` if **every** instance that satisfies `sub` also satisfies
/// `sup`.
pub(crate) fn is_subschema_of(sub: &SchemaNode, sup: &SchemaNode) -> bool {
    is_subschema_of_with_context(sub, sup, &mut SubschemaCheckContext::default())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SubschemaExplanation {
    segments: Vec<String>,
    reason: String,
    schema_path: JsonPointer,
    schema_side: ExplanationSchemaSide,
}

impl SubschemaExplanation {
    fn new(reason: impl Into<String>) -> Self {
        Self {
            segments: Vec::new(),
            reason: reason.into(),
            schema_path: JsonPointer::root(),
            schema_side: ExplanationSchemaSide::Subset,
        }
    }

    fn under(mut self, segment: impl Into<String>) -> Self {
        self.segments.insert(0, segment.into());
        self
    }

    fn in_superset(mut self) -> Self {
        self.schema_side = ExplanationSchemaSide::Superset;
        self
    }

    fn at_keyword(mut self, keyword: impl Into<String>) -> Self {
        self.schema_path.push(keyword);
        self
    }

    fn at_dependent_required(mut self, trigger: &str) -> Self {
        self.schema_path.push("dependentRequired");
        self.schema_path.push(trigger);
        self
    }

    fn at_pattern_property(mut self, pattern: &str) -> Self {
        self.schema_path.push("patternProperties");
        self.schema_path.push(pattern);
        self
    }

    fn at_property(mut self, property: &str) -> Self {
        self.schema_path.push("properties");
        self.schema_path.push(property);
        self
    }

    fn under_property(mut self, property: &str) -> Self {
        self.schema_path.prepend(["properties", property]);
        self.under(format!("property '{property}'"))
    }

    fn under_property_names(mut self) -> Self {
        self.schema_path.prepend(["propertyNames"]);
        self.under("property names")
    }

    fn under_any_of_branch(mut self, index: usize) -> Self {
        self.schema_path.prepend(["anyOf", &index.to_string()]);
        self.under(format!("anyOf branch {}", index + 1))
    }

    fn under_subset_any_of_branch(mut self, index: usize) -> Self {
        if self.schema_side == ExplanationSchemaSide::Subset {
            self.schema_path.prepend(["anyOf", &index.to_string()]);
        }
        self.under(format!("anyOf branch {}", index + 1))
    }

    fn under_superset_any_of_branch(mut self, index: usize) -> Self {
        if self.schema_side == ExplanationSchemaSide::Superset {
            self.schema_path.prepend(["anyOf", &index.to_string()]);
        }
        self.under("closest previous anyOf branch")
    }

    fn under_superset_all_of_branch(mut self, index: usize) -> Self {
        if self.schema_side == ExplanationSchemaSide::Superset {
            self.schema_path.prepend(["allOf", &index.to_string()]);
        }
        self.under(format!("required allOf branch {}", index + 1))
    }

    fn under_one_of_branch(mut self, index: usize) -> Self {
        self.schema_path.prepend(["oneOf", &index.to_string()]);
        self.under(format!("oneOf branch {}", index + 1))
    }

    fn under_array_item(
        mut self,
        index: usize,
        subset_uses_prefix: bool,
        superset_uses_prefix: bool,
    ) -> Self {
        match self.schema_side {
            ExplanationSchemaSide::Subset if subset_uses_prefix => {
                self.schema_path
                    .prepend(["prefixItems", &index.to_string()]);
            }
            ExplanationSchemaSide::Superset if superset_uses_prefix => {
                self.schema_path
                    .prepend(["prefixItems", &index.to_string()]);
            }
            ExplanationSchemaSide::Subset | ExplanationSchemaSide::Superset => {
                self.schema_path.prepend(["items"]);
            }
        }
        self.under(format!("array item {}", index + 1))
    }

    fn under_array_items(mut self) -> Self {
        self.schema_path.prepend(["items"]);
        self.under("array items")
    }

    pub(crate) fn render(&self, subset_label: &str, superset_label: &str) -> String {
        let reason = if self.segments.is_empty() {
            self.reason.clone()
        } else {
            format!("{}: {}", self.segments.join(" -> "), self.reason)
        };
        let schema_label = match self.schema_side {
            ExplanationSchemaSide::Subset => subset_label,
            ExplanationSchemaSide::Superset => superset_label,
        };
        format!(
            "{schema_label} schema {}: {reason}",
            self.schema_path.render()
        )
    }

    fn depth(&self) -> usize {
        self.segments.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExplanationSchemaSide {
    Subset,
    Superset,
}

pub(crate) fn explain_subschema_failure(
    sub: &SchemaNode,
    sup: &SchemaNode,
) -> Option<SubschemaExplanation> {
    explain_subschema_failure_with_context(sub, sup, &mut SubschemaCheckContext::default())
}

pub(crate) fn explain_subschema_failure_emitted_values(
    sub: &SchemaNode,
    sup: &SchemaNode,
) -> Option<SubschemaExplanation> {
    explain_subschema_failure_with_context(
        sub,
        sup,
        &mut SubschemaCheckContext {
            assume_subset_omits_undeclared_properties: true,
            ..SubschemaCheckContext::default()
        },
    )
}

fn explain_subschema_failure_with_context(
    sub: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> Option<SubschemaExplanation> {
    analyze_subschema_with_context(sub, sup, context, ExplanationMode::Explain).explanation
}

/// Variant of [`is_subschema_of`] that models serializer output rather than
/// full JSON Schema validity for the subset side.
///
/// In this mode, object schemas on the subset side are treated as if
/// undeclared extra properties are never emitted, even when
/// `additionalProperties` would permit them.
pub(crate) fn is_subschema_of_emitted_values(sub: &SchemaNode, sup: &SchemaNode) -> bool {
    is_subschema_of_with_context(
        sub,
        sup,
        &mut SubschemaCheckContext {
            assume_subset_omits_undeclared_properties: true,
            ..SubschemaCheckContext::default()
        },
    )
}

pub(super) fn is_subschema_of_with_context(
    sub: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    analyze_subschema_with_context(sub, sup, context, ExplanationMode::VerdictOnly).is_subschema
}

pub(super) fn is_subschema_of_with_productive_context(
    sub: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    context.productive_depth += 1;
    let is_subschema = is_subschema_of_with_context(sub, sup, context);
    context.productive_depth -= 1;
    is_subschema
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExplanationMode {
    VerdictOnly,
    Explain,
}

#[derive(Debug)]
struct SubschemaAnalysis {
    is_subschema: bool,
    explanation: Option<SubschemaExplanation>,
}

impl SubschemaAnalysis {
    fn compatible() -> Self {
        Self {
            is_subschema: true,
            explanation: None,
        }
    }

    fn from_check(
        is_subschema: bool,
        mode: ExplanationMode,
        explanation: impl FnOnce() -> Option<SubschemaExplanation>,
    ) -> Self {
        if is_subschema {
            return Self::compatible();
        }
        Self {
            is_subschema: false,
            explanation: match mode {
                ExplanationMode::VerdictOnly => None,
                ExplanationMode::Explain => explanation(),
            },
        }
    }
}

fn analyze_subschema_with_context(
    sub: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
    mode: ExplanationMode,
) -> SubschemaAnalysis {
    if sub == sup {
        return SubschemaAnalysis::compatible();
    }

    if let Some(values) = constrained_enumeration(sub) {
        let live_values = if schema_may_under_accept_values(sub) {
            // The raw validator may still accept these literals even when the
            // internal evaluator intentionally fails closed.
            values.iter().collect::<Vec<_>>()
        } else {
            values
                .iter()
                .filter(|value| sub.accepts_value(value))
                .collect::<Vec<_>>()
        };
        let is_subschema = live_values
            .iter()
            .all(|value| context.superset_contains_value(sup, value));
        return SubschemaAnalysis::from_check(is_subschema, mode, || {
            Some(SubschemaExplanation::new(
                "enumerated values are not contained by the comparison target",
            ))
        });
    }

    let recursion_key = (sub.id(), sup.id());
    if let Some(active_depth) = context.active_pairs.get(&recursion_key) {
        // Productive recursion through object properties or array items can be
        // assumed coinductively. Same-value applicator cycles cannot: the raw
        // validator may distinguish `anyOf[self, T]` from `allOf[self, T]`.
        let is_guarded_reentry = context.productive_depth > *active_depth;
        return SubschemaAnalysis::from_check(is_guarded_reentry, mode, || {
            explain_schema_kind_gap(sub, sup)
        });
    }
    context
        .active_pairs
        .insert(recursion_key, context.productive_depth);

    use SchemaNodeKind::*;

    let analysis = match (sub.kind(), sup.kind()) {
        (BoolSchema(false), _) => SubschemaAnalysis::compatible(),
        (_, BoolSchema(true)) => SubschemaAnalysis::compatible(),
        (Any, Any) => SubschemaAnalysis::compatible(),
        (_, Any) => SubschemaAnalysis::compatible(),
        (Any, _) => {
            SubschemaAnalysis::from_check(false, mode, || explain_schema_kind_gap(sub, sup))
        }

        // Keep sub-combinator handlers before sup-combinator handlers so when
        // both sides are unions we reason branch-wise on `sub` first.
        (AnyOf(subs), AnyOf(sups)) => {
            let is_subschema = subs.iter().all(|branch| {
                analyze_subschema_with_context(branch, sup, context, mode).is_subschema
            });
            SubschemaAnalysis::from_check(is_subschema, mode, || {
                explain_any_of_to_any_of_failure(subs, sups, sup, context)
            })
        }
        (OneOf(subs), OneOf(sups)) => {
            let is_subschema = subs.iter().all(|branch| {
                analyze_subschema_with_context(branch, sup, context, mode).is_subschema
            });
            SubschemaAnalysis::from_check(is_subschema, mode, || {
                explain_one_of_to_one_of_failure(subs, sups, context)
            })
        }
        (AnyOf(subs), _) | (OneOf(subs), _) => {
            let is_subschema = subs.iter().all(|branch| {
                analyze_subschema_with_context(branch, sup, context, mode).is_subschema
            });
            SubschemaAnalysis::from_check(is_subschema, mode, || {
                explain_subset_union_failure(subs, sup, context)
            })
        }
        (AllOf(subs), _) => {
            let is_subschema = subs.iter().any(|schema| {
                analyze_subschema_with_context(schema, sup, context, mode).is_subschema
            });
            SubschemaAnalysis::from_check(is_subschema, mode, || explain_schema_kind_gap(sub, sup))
        }

        (Enum(sub_e), Enum(sup_e)) => SubschemaAnalysis::from_check(
            check_enum_inclusion(Some(sub_e), Some(sup_e)),
            mode,
            || {
                Some(SubschemaExplanation::new(
                    "enumerated values are not contained by the comparison target",
                ))
            },
        ),
        (Enum(sub_e), _) => SubschemaAnalysis::from_check(
            context.superset_contains_value_set(sup, sub_e),
            mode,
            || explain_schema_kind_gap(sub, sup),
        ),

        (Const(sub_value), Const(sup_value)) => {
            SubschemaAnalysis::from_check(json_values_equal(sub_value, sup_value), mode, || None)
        }
        (Const(sub_value), _) => SubschemaAnalysis::from_check(
            context.superset_contains_value(sup, sub_value),
            mode,
            || explain_schema_kind_gap(sub, sup),
        ),

        (_, AnyOf(sups)) => {
            let is_subschema = sups.iter().any(|branch| {
                analyze_subschema_with_context(sub, branch, context, mode).is_subschema
            });
            SubschemaAnalysis::from_check(is_subschema, mode, || {
                explain_superset_any_of_failure(sub, sups, context)
            })
        }
        (_, OneOf(_)) => {
            SubschemaAnalysis::from_check(false, mode, || explain_schema_kind_gap(sub, sup))
        }
        (_, AllOf(sups)) => {
            let is_subschema = sups.iter().all(|schema| {
                analyze_subschema_with_context(sub, schema, context, mode).is_subschema
            });
            SubschemaAnalysis::from_check(is_subschema, mode, || {
                explain_superset_all_of_failure(sub, sups, context)
            })
        }

        (
            Number {
                enumeration: Some(sub_enum),
                ..
            },
            Enum(_),
        ) => SubschemaAnalysis::from_check(
            context.superset_contains_value_set(sup, sub_enum),
            mode,
            || explain_schema_kind_gap(sub, sup),
        ),

        (_, Enum(_)) => {
            SubschemaAnalysis::from_check(false, mode, || explain_schema_kind_gap(sub, sup))
        }

        (Not(sub_negated), _) => match sub_negated.kind() {
            Any | BoolSchema(true) => SubschemaAnalysis::compatible(),
            BoolSchema(false) => SubschemaAnalysis::from_check(
                matches!(sup.kind(), Any | BoolSchema(true)),
                mode,
                || explain_schema_kind_gap(sub, sup),
            ),
            _ => SubschemaAnalysis::from_check(false, mode, || explain_schema_kind_gap(sub, sup)),
        },
        (_, Not(sup_negated)) => match sup_negated.kind() {
            Any | BoolSchema(true) => {
                SubschemaAnalysis::from_check(matches!(sub.kind(), BoolSchema(false)), mode, || {
                    explain_schema_kind_gap(sub, sup)
                })
            }
            BoolSchema(false) => SubschemaAnalysis::compatible(),
            _ => SubschemaAnalysis::from_check(false, mode, || explain_schema_kind_gap(sub, sup)),
        },

        (String { .. }, String { .. })
        | (Number { .. }, Number { .. })
        | (Integer { .. }, Integer { .. })
        | (Boolean { .. }, Boolean { .. })
        | (Null { .. }, Null { .. })
        | (Object { .. }, Object { .. })
        | (Array { .. }, Array { .. }) => SubschemaAnalysis::from_check(
            type_constraints_subsumed_with_context(sub, sup, context),
            mode,
            || explain_type_constraint_failure(sub, sup, context),
        ),

        (Integer { .. }, Number { .. }) => SubschemaAnalysis::from_check(
            integer_constraints_subsumed_by_number(sub, sup),
            mode,
            || explain_schema_kind_gap(sub, sup),
        ),
        (
            Number {
                enumeration: Some(sub_enum),
                ..
            },
            Integer { .. } | Const(_),
        ) => SubschemaAnalysis::from_check(
            context.superset_contains_value_set(sup, sub_enum),
            mode,
            || explain_schema_kind_gap(sub, sup),
        ),

        (_, Const(_)) => {
            SubschemaAnalysis::from_check(false, mode, || explain_schema_kind_gap(sub, sup))
        }

        _ => SubschemaAnalysis::from_check(false, mode, || explain_schema_kind_gap(sub, sup)),
    };

    context.active_pairs.remove(&recursion_key);
    analysis
}

fn constrained_enumeration(schema: &SchemaNode) -> Option<&[Value]> {
    match schema.kind() {
        SchemaNodeKind::String {
            enumeration: Some(values),
            ..
        }
        | SchemaNodeKind::Number {
            enumeration: Some(values),
            ..
        }
        | SchemaNodeKind::Integer {
            enumeration: Some(values),
            ..
        }
        | SchemaNodeKind::Boolean {
            enumeration: Some(values),
        }
        | SchemaNodeKind::Null {
            enumeration: Some(values),
        }
        | SchemaNodeKind::Object {
            enumeration: Some(values),
            ..
        }
        | SchemaNodeKind::Array {
            enumeration: Some(values),
            ..
        } => Some(values),
        _ => None,
    }
}

pub(super) fn schema_may_under_accept_values(schema: &SchemaNode) -> bool {
    schema_acceptance_deviation(schema).may_under_accept
}

#[derive(Debug, Clone, Copy, Default)]
struct AcceptanceDeviation {
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

fn explain_any_of_to_any_of_failure(
    subs: &[SchemaNode],
    sups: &[SchemaNode],
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> Option<SubschemaExplanation> {
    subs.iter().enumerate().find_map(|(index, branch)| {
        (!sups
            .iter()
            .any(|sup_branch| is_subschema_of_with_context(branch, sup_branch, context)))
        .then(|| {
            sups.get(index)
                .and_then(|sup_branch| {
                    explain_subschema_failure_with_context(branch, sup_branch, context)
                })
                .or_else(|| explain_branch_against_union(branch, sups, context))
                .or_else(|| explain_subschema_failure_with_context(branch, sup, context))
                .unwrap_or_else(|| {
                    SubschemaExplanation::new("union branch is not accepted by the previous schema")
                })
                .under_any_of_branch(index)
        })
    })
}

fn explain_one_of_to_one_of_failure(
    subs: &[SchemaNode],
    sups: &[SchemaNode],
    context: &mut SubschemaCheckContext,
) -> Option<SubschemaExplanation> {
    subs.iter().enumerate().find_map(|(index, branch)| {
        sups.get(index)
            .and_then(|sup_branch| {
                explain_subschema_failure_with_context(branch, sup_branch, context)
            })
            .map(|detail| detail.under_one_of_branch(index))
    })
}

fn explain_subset_union_failure(
    subs: &[SchemaNode],
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> Option<SubschemaExplanation> {
    subs.iter().enumerate().find_map(|(index, branch)| {
        (!is_subschema_of_with_context(branch, sup, context)).then(|| {
            explain_branch_against_sup(branch, sup, context)
                .unwrap_or_else(|| {
                    SubschemaExplanation::new("union branch is not accepted by the previous schema")
                })
                .under_subset_any_of_branch(index)
        })
    })
}

fn explain_branch_against_sup(
    branch: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> Option<SubschemaExplanation> {
    match sup.kind() {
        SchemaNodeKind::AnyOf(sups) => explain_branch_against_union(branch, sups, context),
        _ => explain_subschema_failure_with_context(branch, sup, context),
    }
}

fn explain_branch_against_union(
    branch: &SchemaNode,
    sups: &[SchemaNode],
    context: &mut SubschemaCheckContext,
) -> Option<SubschemaExplanation> {
    explain_superset_any_of_failure(branch, sups, context)
}

fn explain_superset_any_of_failure(
    sub: &SchemaNode,
    sups: &[SchemaNode],
    context: &mut SubschemaCheckContext,
) -> Option<SubschemaExplanation> {
    sups.iter()
        .enumerate()
        .find_map(|(index, branch)| {
            explain_subschema_failure_with_context(sub, branch, context)
                .map(|detail| detail.under_superset_any_of_branch(index))
        })
        .or_else(|| {
            Some(SubschemaExplanation::new(
                "value shape does not fit any previous anyOf branch",
            ))
        })
}

fn explain_superset_all_of_failure(
    sub: &SchemaNode,
    sups: &[SchemaNode],
    context: &mut SubschemaCheckContext,
) -> Option<SubschemaExplanation> {
    sups.iter().enumerate().find_map(|(index, branch)| {
        (!is_subschema_of_with_context(sub, branch, context)).then(|| {
            explain_subschema_failure_with_context(sub, branch, context)
                .unwrap_or_else(|| {
                    SubschemaExplanation::new(
                        "value shape does not satisfy one required allOf branch",
                    )
                })
                .under_superset_all_of_branch(index)
        })
    })
}

fn explain_type_constraint_failure(
    sub: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> Option<SubschemaExplanation> {
    use SchemaNodeKind::*;

    match (sub.kind(), sup.kind()) {
        (
            String {
                length: sub_length,
                pattern: sub_pattern,
                enumeration: sub_enum,
                ..
            },
            String {
                length: sup_length,
                pattern: sup_pattern,
                enumeration: sup_enum,
                ..
            },
        ) => explain_string_constraints(
            StringConstraints {
                length: *sub_length,
                pattern: sub_pattern.as_ref(),
                enumeration: sub_enum.as_deref(),
            },
            StringConstraints {
                length: *sup_length,
                pattern: sup_pattern.as_ref(),
                enumeration: sup_enum.as_deref(),
            },
        ),
        (
            Number {
                bounds: sub_bounds,
                multiple_of: sub_multiple_of,
                enumeration: sub_enum,
            },
            Number {
                bounds: sup_bounds,
                multiple_of: sup_multiple_of,
                enumeration: sup_enum,
            },
        ) => explain_number_constraints(
            *sub_bounds,
            sub_multiple_of.as_ref(),
            sub_enum.as_deref(),
            *sup_bounds,
            sup_multiple_of.as_ref(),
            sup_enum.as_deref(),
        ),
        (
            Integer {
                bounds: sub_bounds,
                multiple_of: sub_multiple_of,
                enumeration: sub_enum,
            },
            Integer {
                bounds: sup_bounds,
                multiple_of: sup_multiple_of,
                enumeration: sup_enum,
            },
        ) => explain_integer_constraints(
            *sub_bounds,
            sub_multiple_of.as_ref(),
            sub_enum.as_deref(),
            *sup_bounds,
            sup_multiple_of.as_ref(),
            sup_enum.as_deref(),
        ),
        (
            Boolean {
                enumeration: sub_enum,
            },
            Boolean {
                enumeration: sup_enum,
            },
        )
        | (
            Null {
                enumeration: sub_enum,
            },
            Null {
                enumeration: sup_enum,
            },
        ) => explain_enumeration_gap(sub_enum.as_deref(), sup_enum.as_deref()),
        (
            Object {
                properties: sub_properties,
                pattern_properties: sub_pattern_properties,
                required: sub_required,
                additional: sub_additional,
                property_names: sub_property_names,
                property_count: sub_property_count,
                dependent_required: sub_dependent_required,
                enumeration: sub_enum,
                ..
            },
            Object {
                properties: sup_properties,
                pattern_properties: sup_pattern_properties,
                required: sup_required,
                additional: sup_additional,
                property_names: sup_property_names,
                property_count: sup_property_count,
                dependent_required: sup_dependent_required,
                enumeration: sup_enum,
                ..
            },
        ) => {
            if let Some(property) = sup_required.difference(sub_required).next() {
                return Some(
                    SubschemaExplanation::new(format!(
                        "property '{property}' is no longer guaranteed to be present",
                    ))
                    .in_superset()
                    .at_keyword("required"),
                );
            }
            if let Some(detail) = explain_enumeration_gap(sub_enum.as_deref(), sup_enum.as_deref())
            {
                return Some(detail);
            }

            for (property, sub_schema) in sub_properties {
                if sup_properties.contains_key(property) {
                    continue;
                }
                if sup_pattern_properties.is_empty()
                    && !is_subschema_of_with_context(sub_schema, sup_additional, context)
                {
                    return Some(SubschemaExplanation::new(format!(
                        "property '{property}' can appear with values the comparison target rejects",
                    ))
                    .at_property(property));
                }
            }

            for (trigger, dependencies) in sup_dependent_required {
                if let Some(dependency) = dependencies.iter().find(|dependency| {
                    !dependent_requirement_is_guaranteed(
                        trigger,
                        dependency,
                        sub_required,
                        sub_dependent_required,
                    )
                }) {
                    return Some(
                        SubschemaExplanation::new(format!(
                            "property '{trigger}' may appear without dependent property '{dependency}'",
                        ))
                        .in_superset()
                        .at_dependent_required(trigger),
                    );
                }
            }

            for (property, sup_schema) in sup_properties {
                if sub_properties.contains_key(property) {
                    continue;
                }
                if !object::implicit_property_conjuncts_subsume_schema(
                    property,
                    sub_pattern_properties,
                    sub_additional,
                    sup_schema,
                ) {
                    return Some(
                        SubschemaExplanation::new(format!(
                            "property '{property}' can appear with values the comparison target rejects",
                        ))
                        .in_superset()
                        .at_property(property),
                    );
                }
            }

            let mut best_property_failure = None;
            for (property, sub_schema) in sub_properties {
                if let Some(sup_schema) = sup_properties.get(property)
                    && !is_subschema_of_with_context(sub_schema, sup_schema, context)
                {
                    let detail =
                        explain_subschema_failure_with_context(sub_schema, sup_schema, context)
                            .unwrap_or_else(|| {
                                SubschemaExplanation::new(
                                    "property schema widened beyond the previous contract",
                                )
                            });
                    let detail = detail.under_property(property);
                    let replace = best_property_failure
                        .as_ref()
                        .is_none_or(|best: &SubschemaExplanation| detail.depth() < best.depth());
                    if replace {
                        best_property_failure = Some(detail);
                    }
                }
            }

            if let Some(detail) = best_property_failure {
                return Some(detail);
            }

            for (pattern, sub_pattern_property) in sub_pattern_properties {
                let sup_schema = sup_pattern_properties
                    .get(pattern)
                    .map_or(sup_additional, |sup_pattern_property| {
                        &sup_pattern_property.schema
                    });
                if !is_subschema_of_with_context(&sub_pattern_property.schema, sup_schema, context)
                {
                    return Some(SubschemaExplanation::new(format!(
                        "pattern property '{pattern}' can accept values the comparison target rejects",
                    ))
                    .at_pattern_property(pattern));
                }

                for (sup_pattern, sup_pattern_property) in sup_pattern_properties {
                    if sup_pattern == pattern {
                        continue;
                    }
                    if !is_subschema_of_with_context(
                        &sub_pattern_property.schema,
                        &sup_pattern_property.schema,
                        context,
                    ) {
                        return Some(
                            SubschemaExplanation::new(format!(
                                "pattern property '{pattern}' may overlap comparison pattern '{sup_pattern}' with values the comparison target rejects",
                            ))
                            .in_superset()
                            .at_pattern_property(sup_pattern),
                        );
                    }
                }
            }

            if !is_subschema_of_with_context(sub_additional, sup_additional, context) {
                return Some(
                    SubschemaExplanation::new(
                        "additional properties can accept values the comparison target rejects",
                    )
                    .at_keyword("additionalProperties"),
                );
            }

            for (pattern, sup_pattern_property) in sup_pattern_properties {
                if sub_pattern_properties.contains_key(pattern) {
                    continue;
                }
                if !is_subschema_of_with_context(
                    sub_additional,
                    &sup_pattern_property.schema,
                    context,
                ) {
                    return Some(SubschemaExplanation::new(format!(
                        "additional properties matching pattern '{pattern}' may violate the required pattern-property schema",
                    ))
                    .in_superset()
                    .at_pattern_property(pattern));
                }
            }

            if !is_subschema_of_with_context(sub_property_names, sup_property_names, context) {
                let detail = explain_subschema_failure_with_context(
                    sub_property_names,
                    sup_property_names,
                    context,
                )
                .unwrap_or_else(|| {
                    SubschemaExplanation::new(
                        "property names are not contained by the comparison target",
                    )
                });
                return Some(detail.under_property_names());
            }

            if !sup_property_count.contains_range(*sub_property_count) {
                return Some(SubschemaExplanation::new(format!(
                    "object property count range {} is not contained by required range {}",
                    format_count_range(*sub_property_count),
                    format_count_range(*sup_property_count),
                )));
            }

            None
        }
        (
            Array {
                prefix_items: sub_prefix_items,
                items: sub_items,
                item_count: sub_item_count,
                contains: sub_contains,
                unique_items: sub_unique_items,
                enumeration: sub_enum,
            },
            Array {
                prefix_items: sup_prefix_items,
                items: sup_items,
                item_count: sup_item_count,
                contains: sup_contains,
                unique_items: sup_unique_items,
                enumeration: sup_enum,
            },
        ) => {
            if !sup_item_count.contains_range(*sub_item_count) {
                return Some(SubschemaExplanation::new(format!(
                    "array length range {} is not contained by required range {}",
                    format_count_range(*sub_item_count),
                    format_count_range(*sup_item_count),
                )));
            }

            if *sup_unique_items
                && !*sub_unique_items
                && sub_item_count.max().is_none_or(|max_items| max_items > 1)
            {
                return Some(SubschemaExplanation::new(
                    "arrays may contain duplicate items, but the comparison target requires unique items",
                )
                .in_superset()
                .at_keyword("uniqueItems"));
            }

            let checked_prefix_len = sub_prefix_items.len().max(sup_prefix_items.len());
            for index in 0..checked_prefix_len {
                if !array_index_can_exist(sub_item_count.max(), index) {
                    break;
                }

                let sub_item = sub_prefix_items.get(index).unwrap_or(sub_items);
                let sup_item = sup_prefix_items.get(index).unwrap_or(sup_items);
                if !is_subschema_of_with_context(sub_item, sup_item, context) {
                    let detail =
                        explain_subschema_failure_with_context(sub_item, sup_item, context)
                            .unwrap_or_else(|| {
                                SubschemaExplanation::new(
                                    "array item schema widened beyond the comparison target",
                                )
                            });
                    return Some(detail.under_array_item(
                        index,
                        index < sub_prefix_items.len(),
                        index < sup_prefix_items.len(),
                    ));
                }
            }

            if array_index_can_exist(sub_item_count.max(), checked_prefix_len)
                && !is_subschema_of_with_context(sub_items, sup_items, context)
            {
                let detail = explain_subschema_failure_with_context(sub_items, sup_items, context)
                    .unwrap_or_else(|| {
                        SubschemaExplanation::new(
                            "array item schema widened beyond the comparison target",
                        )
                    });
                return Some(detail.under_array_items());
            }

            if let Some(sup_contains) = sup_contains {
                let sup_count = sup_contains.count();
                let lower_bound_ok = sup_count.min() == 0
                    || sub_contains.as_ref().is_some_and(|sub_contains| {
                        sub_contains.count().min() >= sup_count.min()
                            && is_subschema_of_with_context(
                                &sub_contains.schema,
                                &sup_contains.schema,
                                context,
                            )
                    })
                    || guaranteed_array_item_matches_at_least_for_explanation(
                        sub_prefix_items,
                        sub_items,
                        sub_item_count.min(),
                        &sup_contains.schema,
                        sup_count.min(),
                        context,
                    );
                if !lower_bound_ok {
                    return Some(SubschemaExplanation::new(format!(
                        "array values do not guarantee at least {} item(s) matching the required contains schema",
                        sup_count.min(),
                    ))
                    .in_superset()
                    .at_keyword("contains"));
                }

                if let Some(sup_max_contains) = sup_count.max() {
                    let upper_bound_ok =
                        sub_contains.as_ref().is_some_and(|sub_contains| {
                            sub_contains.count().max().is_some_and(|sub_max_contains| {
                                sub_max_contains <= sup_max_contains
                            }) && is_subschema_of_with_context(
                                &sup_contains.schema,
                                &sub_contains.schema,
                                context,
                            )
                        }) || (sub_item_count
                            .max()
                            .is_some_and(|sub_max_items| sub_max_items <= sup_max_contains)
                            && all_array_item_schemas_subsumed_by_for_explanation(
                                sub_prefix_items,
                                sub_items,
                                sub_item_count.max(),
                                &sup_contains.schema,
                                context,
                            ));
                    if !upper_bound_ok {
                        return Some(SubschemaExplanation::new(format!(
                            "array values may contain more than {sup_max_contains} item(s) matching the comparison target's contains schema",
                        ))
                        .in_superset()
                        .at_keyword("contains"));
                    }
                }
            }

            if let Some(detail) = explain_enumeration_gap(sub_enum.as_deref(), sup_enum.as_deref())
            {
                return Some(detail);
            }

            Some(SubschemaExplanation::new(
                "array constraints are not contained by the comparison target",
            ))
        }
        _ => None,
    }
}

fn explain_schema_kind_gap(sub: &SchemaNode, sup: &SchemaNode) -> Option<SubschemaExplanation> {
    (schema_kind_name(sub.kind()) != schema_kind_name(sup.kind())).then(|| {
        SubschemaExplanation::new(format!(
            "new values may be {}, but the previous schema only accepted {}",
            schema_kind_name(sub.kind()),
            schema_kind_name(sup.kind()),
        ))
    })
}

fn explain_string_constraints(
    sub: StringConstraints<'_>,
    sup: StringConstraints<'_>,
) -> Option<SubschemaExplanation> {
    if !sup.length.contains_range(sub.length) {
        return Some(SubschemaExplanation::new(format!(
            "string length range {} is not contained by required range {}",
            format_count_range(sub.length),
            format_count_range(sup.length),
        )));
    }
    if sup.pattern.is_some() && sub.pattern != sup.pattern {
        return Some(SubschemaExplanation::new(
            "string pattern does not preserve the comparison target's required pattern",
        ));
    }
    explain_enumeration_gap(sub.enumeration, sup.enumeration)
}

fn explain_number_constraints(
    sub_bounds: NumberBounds,
    sub_multiple_of: Option<&NumberMultipleOf>,
    sub_enum: Option<&[Value]>,
    sup_bounds: NumberBounds,
    sup_multiple_of: Option<&NumberMultipleOf>,
    sup_enum: Option<&[Value]>,
) -> Option<SubschemaExplanation> {
    if !sup_bounds.contains_bounds(sub_bounds) {
        return Some(SubschemaExplanation::new(format!(
            "number bounds {} are not contained by required bounds {}",
            format_number_bounds(sub_bounds),
            format_number_bounds(sup_bounds),
        )));
    }
    if !number_multiple_of_constraints_subsumed(sub_multiple_of, sup_multiple_of) {
        return Some(SubschemaExplanation::new(format!(
            "number multipleOf {} is not at least as restrictive as required multipleOf {}",
            format_optional_number_multiple_of(sub_multiple_of),
            format_optional_number_multiple_of(sup_multiple_of),
        )));
    }
    explain_enumeration_gap(sub_enum, sup_enum)
}

fn explain_integer_constraints(
    sub_bounds: IntegerBounds,
    sub_multiple_of: Option<&IntegerMultipleOf>,
    sub_enum: Option<&[Value]>,
    sup_bounds: IntegerBounds,
    sup_multiple_of: Option<&IntegerMultipleOf>,
    sup_enum: Option<&[Value]>,
) -> Option<SubschemaExplanation> {
    if !sup_bounds.contains_bounds(sub_bounds) {
        return Some(SubschemaExplanation::new(format!(
            "integer bounds {} are not contained by required bounds {}",
            format_integer_bounds(sub_bounds),
            format_integer_bounds(sup_bounds),
        )));
    }
    if !integer_multiple_of_constraints_subsumed(sub_multiple_of, sup_multiple_of) {
        return Some(SubschemaExplanation::new(format!(
            "integer multipleOf {} is not at least as restrictive as required multipleOf {}",
            format_optional_integer_multiple_of(sub_multiple_of),
            format_optional_integer_multiple_of(sup_multiple_of),
        )));
    }
    explain_enumeration_gap(sub_enum, sup_enum)
}

fn explain_enumeration_gap(
    sub_enum: Option<&[Value]>,
    sup_enum: Option<&[Value]>,
) -> Option<SubschemaExplanation> {
    (!scalar::check_enum_inclusion(sub_enum, sup_enum)).then(|| {
        SubschemaExplanation::new("enumerated values are not contained by the comparison target")
    })
}

fn number_multiple_of_constraints_subsumed(
    sub_multiple_of: Option<&NumberMultipleOf>,
    sup_multiple_of: Option<&NumberMultipleOf>,
) -> bool {
    let Some(sup_multiple_of) = sup_multiple_of else {
        return true;
    };
    let Some(sub_multiple_of) = sub_multiple_of else {
        return false;
    };
    sub_multiple_of
        .integer_divisor_is_multiple_of(*sup_multiple_of)
        .unwrap_or(false)
}

fn integer_multiple_of_constraints_subsumed(
    sub_multiple_of: Option<&IntegerMultipleOf>,
    sup_multiple_of: Option<&IntegerMultipleOf>,
) -> bool {
    let Some(sup_multiple_of) = sup_multiple_of else {
        return true;
    };
    let Some(sub_multiple_of) = sub_multiple_of else {
        return false;
    };
    sub_multiple_of
        .integer_divisor_is_multiple_of(*sup_multiple_of)
        .unwrap_or(false)
}

fn format_count_range<T: std::fmt::Display + Copy + Ord>(range: CountRange<T>) -> String {
    match range.max() {
        Some(max) if max == range.min() => format!("{}", range.min()),
        Some(max) => format!("{}..={max}", range.min()),
        None => format!("{}..", range.min()),
    }
}

fn format_number_bounds(bounds: NumberBounds) -> String {
    format!(
        "{}{}, {}{}",
        number_lower_delimiter(bounds.lower()),
        format_number_bound_value(bounds.lower(), "-inf"),
        format_number_bound_value(bounds.upper(), "+inf"),
        number_upper_delimiter(bounds.upper()),
    )
}

fn number_lower_delimiter(bound: NumberBound) -> &'static str {
    match bound {
        NumberBound::Exclusive(_) => "(",
        NumberBound::Inclusive(_) | NumberBound::Unbounded => "[",
    }
}

fn number_upper_delimiter(bound: NumberBound) -> &'static str {
    match bound {
        NumberBound::Exclusive(_) => ")",
        NumberBound::Inclusive(_) | NumberBound::Unbounded => "]",
    }
}

fn format_number_bound_value(bound: NumberBound, unbounded: &str) -> String {
    match bound {
        NumberBound::Unbounded => unbounded.to_owned(),
        NumberBound::Inclusive(value) | NumberBound::Exclusive(value) => value.to_string(),
    }
}

fn format_integer_bounds(bounds: IntegerBounds) -> String {
    format!(
        "[{}, {}]",
        bounds
            .lower()
            .map_or("-inf".to_owned(), |value| value.to_string()),
        bounds
            .upper()
            .map_or("+inf".to_owned(), |value| value.to_string()),
    )
}

fn format_optional_number_multiple_of(multiple_of: Option<&NumberMultipleOf>) -> String {
    multiple_of.map_or_else(
        || "<none>".to_owned(),
        |multiple_of| multiple_of.as_f64().to_string(),
    )
}

fn format_optional_integer_multiple_of(multiple_of: Option<&IntegerMultipleOf>) -> String {
    multiple_of.map_or_else(
        || "<none>".to_owned(),
        |multiple_of| multiple_of.as_f64().to_string(),
    )
}

fn array_index_can_exist(max_items: Option<u64>, index: usize) -> bool {
    let Ok(index) = u64::try_from(index) else {
        return false;
    };
    max_items.is_none_or(|max_items| index < max_items)
}

fn all_array_item_schemas_subsumed_by_for_explanation(
    prefix_items: &[SchemaNode],
    items: &SchemaNode,
    max_items: Option<u64>,
    sup_schema: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    for (index, prefix_item) in prefix_items.iter().enumerate() {
        if !array_index_can_exist(max_items, index) {
            return true;
        }
        if !is_subschema_of_with_context(prefix_item, sup_schema, context) {
            return false;
        }
    }

    !array_index_can_exist(max_items, prefix_items.len())
        || is_subschema_of_with_context(items, sup_schema, context)
}

fn guaranteed_array_item_matches_at_least_for_explanation(
    prefix_items: &[SchemaNode],
    items: &SchemaNode,
    guaranteed_items: u64,
    sup_schema: &SchemaNode,
    required_matches: u64,
    context: &mut SubschemaCheckContext,
) -> bool {
    if required_matches == 0 {
        return true;
    }

    let guaranteed_prefix_items = prefix_items
        .len()
        .min(usize::try_from(guaranteed_items).unwrap_or(usize::MAX));
    let mut guaranteed_matches = 0_u64;
    for prefix_item in &prefix_items[..guaranteed_prefix_items] {
        if is_subschema_of_with_context(prefix_item, sup_schema, context) {
            guaranteed_matches += 1;
            if guaranteed_matches >= required_matches {
                return true;
            }
        }
    }

    let guaranteed_tail_items =
        guaranteed_items.saturating_sub(u64::try_from(guaranteed_prefix_items).unwrap_or(u64::MAX));
    guaranteed_tail_items > 0
        && is_subschema_of_with_context(items, sup_schema, context)
        && guaranteed_matches.saturating_add(guaranteed_tail_items) >= required_matches
}

fn schema_kind_name(kind: &SchemaNodeKind) -> &'static str {
    match kind {
        SchemaNodeKind::Any => "any value",
        SchemaNodeKind::BoolSchema(true) => "any value",
        SchemaNodeKind::BoolSchema(false) => "no value",
        SchemaNodeKind::String { .. } => "strings",
        SchemaNodeKind::Number { .. } => "numbers",
        SchemaNodeKind::Integer { .. } => "integers",
        SchemaNodeKind::Boolean { .. } => "booleans",
        SchemaNodeKind::Null { .. } => "null",
        SchemaNodeKind::Object { .. } => "objects",
        SchemaNodeKind::Array { .. } => "arrays",
        SchemaNodeKind::Enum(_) => "enumerated values",
        SchemaNodeKind::Const(_) => "a fixed value",
        SchemaNodeKind::AllOf(_) => "allOf-constrained values",
        SchemaNodeKind::AnyOf(_) => "anyOf-constrained values",
        SchemaNodeKind::OneOf(_) => "oneOf-constrained values",
        SchemaNodeKind::Not(_) => "negated-schema values",
        SchemaNodeKind::IfThenElse { .. } => "conditional-schema values",
        _ => "values accepted by another schema form",
    }
}

fn type_constraints_subsumed_with_context(
    sub: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    use SchemaNodeKind::*;

    match (sub.kind(), sup.kind()) {
        (
            String {
                length: sub_length,
                pattern: sub_pattern,
                enumeration: sub_enum,
                ..
            },
            String {
                length: sup_length,
                pattern: sup_pattern,
                enumeration: sup_enum,
                ..
            },
        ) => string_constraints_subsumed(
            StringConstraints {
                length: *sub_length,
                pattern: sub_pattern.as_ref(),
                enumeration: sub_enum.as_deref(),
            },
            StringConstraints {
                length: *sup_length,
                pattern: sup_pattern.as_ref(),
                enumeration: sup_enum.as_deref(),
            },
        ),

        (
            Number {
                bounds: sub_bounds,
                multiple_of: sub_multiple_of,
                enumeration: sub_enum,
            },
            Number {
                bounds: sup_bounds,
                multiple_of: sup_multiple_of,
                enumeration: sup_enum,
            },
        ) => scalar::number_constraints_subsumed(
            *sub_bounds,
            sub_multiple_of.as_ref(),
            sub_enum.as_deref(),
            *sup_bounds,
            sup_multiple_of.as_ref(),
            sup_enum.as_deref(),
        ),

        (
            Integer {
                bounds: sub_bounds,
                multiple_of: sub_multiple_of,
                enumeration: sub_enum,
            },
            Integer {
                bounds: sup_bounds,
                multiple_of: sup_multiple_of,
                enumeration: sup_enum,
            },
        ) => scalar::integer_constraints_subsumed(
            *sub_bounds,
            sub_multiple_of.as_ref(),
            sub_enum.as_deref(),
            *sup_bounds,
            sup_multiple_of.as_ref(),
            sup_enum.as_deref(),
        ),

        (
            Boolean {
                enumeration: sub_enum,
            },
            Boolean {
                enumeration: sup_enum,
            },
        )
        | (
            Null {
                enumeration: sub_enum,
            },
            Null {
                enumeration: sup_enum,
            },
        ) => check_enum_inclusion(sub_enum.as_deref(), sup_enum.as_deref()),

        (
            Object {
                properties: sub_properties,
                pattern_properties: sub_pattern_properties,
                required: sub_required,
                additional: sub_additional,
                property_names: sub_property_names,
                property_count: sub_property_count,
                dependent_required: _sub_dependent_required,
                enumeration: sub_enum,
            },
            Object {
                properties: sup_properties,
                pattern_properties: sup_pattern_properties,
                required: sup_required,
                additional: sup_additional,
                property_names: sup_property_names,
                property_count: sup_property_count,
                dependent_required: sup_dependent_required,
                enumeration: sup_enum,
            },
        ) => object::object_constraints_subsumed(
            object::ObjectConstraints {
                properties: sub_properties,
                pattern_properties: sub_pattern_properties,
                required: sub_required,
                additional: sub_additional,
                property_names: sub_property_names,
                property_count: *sub_property_count,
                dependent_required: _sub_dependent_required,
                enumeration: sub_enum.as_deref(),
            },
            object::ObjectConstraints {
                properties: sup_properties,
                pattern_properties: sup_pattern_properties,
                required: sup_required,
                additional: sup_additional,
                property_names: sup_property_names,
                property_count: *sup_property_count,
                dependent_required: sup_dependent_required,
                enumeration: sup_enum.as_deref(),
            },
            context,
        ),

        (
            Array {
                prefix_items: sub_prefix_items,
                items: sub_items,
                item_count: sub_item_count,
                contains: sub_contains,
                unique_items: sub_unique_items,
                enumeration: sub_enum,
            },
            Array {
                prefix_items: sup_prefix_items,
                items: sup_items,
                item_count: sup_item_count,
                contains: sup_contains,
                unique_items: sup_unique_items,
                enumeration: sup_enum,
            },
        ) => array::array_constraints_subsumed(
            array::ArrayConstraints {
                prefix_items: sub_prefix_items,
                items: sub_items,
                item_count: *sub_item_count,
                contains: sub_contains.as_ref(),
                unique_items: *sub_unique_items,
                enumeration: sub_enum.as_deref(),
            },
            array::ArrayConstraints {
                prefix_items: sup_prefix_items,
                items: sup_items,
                item_count: *sup_item_count,
                contains: sup_contains.as_ref(),
                unique_items: *sup_unique_items,
                enumeration: sup_enum.as_deref(),
            },
            context,
        ),

        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use json_schema_ast::SchemaDocument;
    use serde_json::json;

    fn resolve(raw: Value) -> SchemaNode {
        SchemaDocument::from_json(&raw)
            .unwrap()
            .root()
            .unwrap()
            .clone()
    }

    #[test]
    fn allof_tighten_subset() {
        let old = resolve(json!({
            "allOf": [
                {"type": "integer", "minimum": 0},
                {"maximum": 10}
            ]
        }));
        let new = resolve(json!({
            "type": "integer",
            "minimum": 0,
            "maximum": 5
        }));
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn allof_subset_can_be_proven_by_a_single_conjunct() {
        let old = resolve(json!({
            "type": "number",
            "minimum": 0
        }));
        let new = resolve(json!({
            "allOf": [
                { "type": "number", "minimum": 1, "maximum": 5 },
                { "type": "number", "maximum": 10 }
            ]
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn allof_subset_proof_can_ignore_an_unhelpful_sibling_conjunct() {
        let old = resolve(json!({
            "type": "object",
            "properties": {
                "id": { "type": "string" }
            },
            "required": ["id"],
            "additionalProperties": false
        }));
        let new = resolve(json!({
            "allOf": [
                {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" }
                    },
                    "required": ["id"],
                    "additionalProperties": false
                },
                {
                    "type": "object",
                    "properties": {
                        "trace": { "type": "string" }
                    }
                }
            ]
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn allof_subset_proof_does_not_invent_a_useful_conjunct() {
        let old = resolve(json!({ "type": "string" }));
        let new = resolve(json!({
            "allOf": [
                { "minLength": 1 },
                { "pattern": "^x$" }
            ]
        }));

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn not_false_behaves_like_any_schema_for_subset_checks() {
        let any = resolve(json!({ "not": false }));
        let string = resolve(json!({ "type": "string" }));

        assert!(is_subschema_of(&string, &any));
        assert!(!is_subschema_of(&any, &string));
    }

    #[test]
    fn exclusive_bounds_subset() {
        let old = resolve(json!({
            "minimum": 1,
            "exclusiveMinimum": 1
        }));
        let new = resolve(json!({
            "exclusiveMinimum": 1,
            "maximum": 3
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn enum_with_numeric_bound_is_not_subsumed_by_wider_enum() {
        let old = resolve(json!({
                "type": "number",
                "enum": [0, 1],
                "minimum": 1
        }));
        let new = resolve(json!({
                "enum": [0, 1]
        }));

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn constrained_number_enum_ignores_dead_literals_when_proving_subset() {
        let old = resolve(json!({
            "type": "number",
            "enum": [1]
        }));
        let new = resolve(json!({
            "type": "number",
            "enum": [0, 1],
            "minimum": 1
        }));

        assert!(is_subschema_of(&new, &old));
        assert!(is_subschema_of(&old, &new));
    }

    #[test]
    fn large_number_bounds_do_not_prune_live_enum_literals() {
        let raw = json!({
            "type": "number",
            "minimum": 9_007_199_254_740_993_i64,
            "enum": [9_007_199_254_740_993_i64]
        });
        let sub_document = SchemaDocument::from_json(&raw).unwrap();
        assert!(
            sub_document
                .is_valid(&json!(9_007_199_254_740_993_i64))
                .unwrap()
        );

        let sub = sub_document.root().unwrap().clone();
        let sup = resolve(json!({ "type": "integer", "enum": [0] }));

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn constrained_enum_with_unsupported_pattern_is_not_treated_as_empty() {
        let raw = json!({
            "type": "string",
            "pattern": "^\\cC$",
            "enum": ["\u{3}"]
        });
        let sub_document = SchemaDocument::from_json(&raw).unwrap();
        assert!(sub_document.is_valid(&json!("\u{3}")).unwrap());

        let sub = sub_document.root().unwrap().clone();
        let sup = resolve(json!({ "type": "integer" }));

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn nested_unsupported_patterns_keep_object_enum_literals_live() {
        let raw = json!({
            "type": "object",
            "properties": {
                "value": {
                    "type": "string",
                    "pattern": "^\\cC$"
                }
            },
            "required": ["value"],
            "additionalProperties": false,
            "enum": [{ "value": "\u{3}" }]
        });
        let sub_document = SchemaDocument::from_json(&raw).unwrap();
        assert!(sub_document.is_valid(&json!({ "value": "\u{3}" })).unwrap());

        let sub = sub_document.root().unwrap().clone();
        let sup = resolve(json!({
            "type": "object",
            "properties": {
                "value": { "type": "integer" }
            },
            "required": ["value"],
            "additionalProperties": false
        }));

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn nested_unsupported_patterns_keep_array_enum_literals_live() {
        let raw = json!({
            "type": "array",
            "prefixItems": [{
                "type": "string",
                "pattern": "^\\cC$"
            }],
            "items": false,
            "enum": [["\u{3}"]]
        });
        let sub_document = SchemaDocument::from_json(&raw).unwrap();
        assert!(sub_document.is_valid(&json!(["\u{3}"])).unwrap());

        let sub = sub_document.root().unwrap().clone();
        let sup = resolve(json!({
            "type": "array",
            "prefixItems": [{ "type": "integer" }],
            "items": false
        }));

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn unsupported_contains_patterns_keep_array_enum_literals_live() {
        let raw = json!({
            "type": "array",
            "contains": {
                "type": "string",
                "pattern": "^\\cC$"
            },
            "minContains": 1,
            "enum": [["\u{3}"]]
        });
        let sub_document = SchemaDocument::from_json(&raw).unwrap();
        assert!(sub_document.is_valid(&json!(["\u{3}"])).unwrap());

        let sub = sub_document.root().unwrap().clone();
        let sup = resolve(json!({
            "type": "array",
            "contains": { "type": "integer" },
            "minContains": 1
        }));

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn unsupported_pattern_property_matchers_keep_object_enum_literals_live() {
        let raw = json!({
            "type": "object",
            "patternProperties": {
                "^\\cC$": { "type": "integer" }
            },
            "additionalProperties": false,
            "enum": [{ "\u{3}": 1 }]
        });
        let sub_document = SchemaDocument::from_json(&raw).unwrap();
        assert!(sub_document.is_valid(&json!({ "\u{3}": 1 })).unwrap());

        let sub = sub_document.root().unwrap().clone();
        let sup = resolve(json!({
            "type": "object",
            "patternProperties": {
                "^\\cC$": { "type": "string" }
            },
            "additionalProperties": false
        }));

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn unsupported_superset_pattern_properties_do_not_overaccept_object_enum_literals() {
        let sub_document = SchemaDocument::from_json(&json!({
            "type": "object",
            "enum": [{ "\u{3}": "not an integer" }]
        }))
        .unwrap();
        let sup_document = SchemaDocument::from_json(&json!({
            "type": "object",
            "patternProperties": {
                "^\\cC$": { "type": "integer" }
            },
            "additionalProperties": true
        }))
        .unwrap();
        let witness = json!({ "\u{3}": "not an integer" });

        assert!(sub_document.is_valid(&witness).unwrap());
        assert!(!sup_document.is_valid(&witness).unwrap());

        let sub = sub_document.root().unwrap().clone();
        let sup = sup_document.root().unwrap().clone();
        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn negated_unsupported_patterns_do_not_overaccept_enum_literals() {
        let sub_document = SchemaDocument::from_json(&json!({
            "type": "string",
            "enum": ["\u{3}"]
        }))
        .unwrap();
        let sup_document = SchemaDocument::from_json(&json!({
            "not": {
                "type": "string",
                "pattern": "^\\cC$"
            }
        }))
        .unwrap();
        let witness = json!("\u{3}");

        assert!(sub_document.is_valid(&witness).unwrap());
        assert!(!sup_document.is_valid(&witness).unwrap());

        let sub = sub_document.root().unwrap().clone();
        let sup = sup_document.root().unwrap().clone();
        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn one_of_with_unsupported_patterns_does_not_overaccept_enum_literals() {
        let sub_document = SchemaDocument::from_json(&json!({
            "type": "string",
            "enum": ["\u{3}"]
        }))
        .unwrap();
        let sup_document = SchemaDocument::from_json(&json!({
            "oneOf": [
                {
                    "type": "string",
                    "pattern": "^\\cC$"
                },
                { "const": "\u{3}" }
            ]
        }))
        .unwrap();
        let witness = json!("\u{3}");

        assert!(sub_document.is_valid(&witness).unwrap());
        assert!(!sup_document.is_valid(&witness).unwrap());

        let sub = sub_document.root().unwrap().clone();
        let sup = sup_document.root().unwrap().clone();
        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditionals_with_unsupported_patterns_do_not_overaccept_enum_literals() {
        let sub_document = SchemaDocument::from_json(&json!({
            "type": "string",
            "enum": ["\u{3}"]
        }))
        .unwrap();
        let sup_document = SchemaDocument::from_json(&json!({
            "if": {
                "type": "string",
                "pattern": "^\\cC$"
            },
            "then": false,
            "else": true
        }))
        .unwrap();
        let witness = json!("\u{3}");

        assert!(sub_document.is_valid(&witness).unwrap());
        assert!(!sup_document.is_valid(&witness).unwrap());

        let sub = sub_document.root().unwrap().clone();
        let sup = sup_document.root().unwrap().clone();
        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn max_contains_with_unsupported_patterns_does_not_overaccept_enum_literals() {
        let sub_document = SchemaDocument::from_json(&json!({
            "type": "array",
            "enum": [["\u{3}"]]
        }))
        .unwrap();
        let sup_document = SchemaDocument::from_json(&json!({
            "type": "array",
            "contains": {
                "type": "string",
                "pattern": "^\\cC$"
            },
            "maxContains": 0
        }))
        .unwrap();
        let witness = json!(["\u{3}"]);

        assert!(sub_document.is_valid(&witness).unwrap());
        assert!(!sup_document.is_valid(&witness).unwrap());

        let sub = sub_document.root().unwrap().clone();
        let sup = sup_document.root().unwrap().clone();
        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn recursive_enum_schemas_do_not_recurse_forever_while_scanning_patterns() {
        let raw = json!({
            "$defs": {
                "node": {
                    "type": "object",
                    "properties": {
                        "next": { "$ref": "#/$defs/node" }
                    },
                    "additionalProperties": false,
                    "enum": [{}]
                }
            },
            "$ref": "#/$defs/node"
        });
        let sub_document = SchemaDocument::from_json(&raw).unwrap();
        assert!(sub_document.is_valid(&json!({})).unwrap());

        let sub = sub_document.root().unwrap().clone();
        let sup = resolve(json!({ "type": "integer" }));

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn recursive_allof_enum_literals_stay_live_when_raw_validation_accepts_them() {
        let raw = json!({
            "$defs": {
                "Value": {
                    "allOf": [
                        { "$ref": "#/$defs/Value" },
                        { "type": "string" }
                    ]
                }
            },
            "type": "object",
            "properties": {
                "value": { "$ref": "#/$defs/Value" }
            },
            "required": ["value"],
            "additionalProperties": false,
            "enum": [{ "value": "leaf" }]
        });
        let sub_document = SchemaDocument::from_json(&raw).unwrap();
        assert!(sub_document.is_valid(&json!({ "value": "leaf" })).unwrap());

        let sub = sub_document.root().unwrap().clone();
        let sup = resolve(json!({
            "type": "object",
            "properties": {
                "value": { "type": "integer" }
            },
            "required": ["value"],
            "additionalProperties": false
        }));

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn enum_inclusion_uses_json_schema_numeric_equality() {
        let old = resolve(json!({
            "enum": [1.0, 2]
        }));
        let new = resolve(json!({
            "enum": [1, 2.0]
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn integer_multiple_of_must_be_at_least_as_constrained_as_number_multiple_of() {
        let old = resolve(json!({
                "type": "number",
                "multipleOf": 2
        }));
        let new = resolve(json!({
                "type": "integer",
                "multipleOf": 3
        }));

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn huge_integer_multiple_of_ratio_must_be_exactly_divisible() {
        let old = resolve(json!({
                "type": "integer",
                "multipleOf": 3
        }));
        let new = resolve(json!({
                "type": "integer",
                "multipleOf": 9_007_199_254_740_994_f64
        }));

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn integer_schema_without_enum_is_not_subsumed_by_old_number_enum() {
        let old = resolve(json!({
                "type": "number",
                "enum": [1],
                "minimum": 1
        }));
        let new = resolve(json!({
                "type": "integer",
                "minimum": 1,
                "maximum": 2
        }));

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn enum_is_not_checked_by_serializing_recursive_sup_schema() {
        let sup = resolve(json!({
            "$defs": {
                "Node": {
                    "type": "object",
                    "properties": {
                        "next": { "$ref": "#/$defs/Node" }
                    }
                }
            },
            "$ref": "#/$defs/Node"
        }));
        let sub = resolve(json!({
            "enum": [1]
        }));

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn enum_values_are_checked_against_recursive_sup_schema_structurally() {
        let sup = resolve(json!({
            "$defs": {
                "Node": {
                    "type": "object",
                    "properties": {
                        "next": { "$ref": "#/$defs/Node" }
                    }
                }
            },
            "$ref": "#/$defs/Node"
        }));
        let sub = resolve(json!({
            "enum": [{ "next": {} }]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn const_values_are_checked_against_recursive_sup_schema_structurally() {
        let sup = resolve(json!({
            "$defs": {
                "Node": {
                    "type": "object",
                    "properties": {
                        "value": { "type": "integer" },
                        "next": { "$ref": "#/$defs/Node" }
                    },
                    "required": ["value"]
                }
            },
            "$ref": "#/$defs/Node"
        }));
        let sub = resolve(json!({
            "const": {
                "value": 1,
                "next": {
                    "value": 2
                }
            }
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn const_values_reject_non_productive_recursive_sup_anyof() {
        let sup = resolve(json!({
            "$defs": {
                "A": {
                    "anyOf": [
                        { "type": "integer" },
                        { "$ref": "#/$defs/A" }
                    ]
                }
            },
            "$ref": "#/$defs/A"
        }));
        let sub = resolve(json!({
            "const": "x"
        }));

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn same_value_recursive_anyof_is_not_subsumed_by_same_value_recursive_allof() {
        let sub = resolve(json!({
            "$defs": {
                "Node": {
                    "anyOf": [
                        { "$ref": "#/$defs/Node" },
                        { "type": "string" }
                    ]
                }
            },
            "$ref": "#/$defs/Node"
        }));
        let sup = resolve(json!({
            "$defs": {
                "Node": {
                    "allOf": [
                        { "$ref": "#/$defs/Node" },
                        { "type": "string" }
                    ]
                }
            },
            "$ref": "#/$defs/Node"
        }));

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn array_contains_lower_bound_must_hold_for_every_subset_instance() {
        let old = resolve(json!({
            "type": "array",
            "contains": { "type": "string" },
            "minContains": 2
        }));
        let new = resolve(json!({
            "type": "array",
            "items": { "type": "integer" },
            "minItems": 2
        }));

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn array_items_can_witness_contains_lower_bound_when_every_item_matches() {
        let old = resolve(json!({
            "type": "array",
            "contains": { "type": "string" },
            "minContains": 1
        }));
        let new = resolve(json!({
            "type": "array",
            "items": { "type": "string" },
            "minItems": 2
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn guaranteed_prefix_items_can_witness_contains_lower_bounds() {
        let old = resolve(json!({
            "type": "array",
            "contains": { "type": "integer" },
            "minContains": 1
        }));
        let new = resolve(json!({
            "type": "array",
            "prefixItems": [
                { "type": "integer" },
                { "type": "string" }
            ],
            "items": false,
            "minItems": 1
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn duplicate_oneof_branches_are_not_a_deserializer_superset_of_the_branch_schema() {
        let old = resolve(json!({
            "type": "string"
        }));
        let new = resolve(json!({
            "oneOf": [
                { "type": "string" },
                { "type": "string" }
            ]
        }));

        assert!(!is_subschema_of(&old, &new));
    }

    #[test]
    fn prefix_items_are_checked_positionally() {
        let old = resolve(json!({
            "type": "array",
            "prefixItems": [{ "type": "string" }],
            "minItems": 1
        }));
        let new = resolve(json!({
            "type": "array",
            "prefixItems": [{ "type": "integer" }],
            "minItems": 1
        }));

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn prefix_items_with_false_items_are_subsumed_by_max_items_cap() {
        let old = resolve(json!({
            "type": "array",
            "maxItems": 2
        }));
        let new = resolve(json!({
            "type": "array",
            "prefixItems": [
                { "const": 1 },
                { "const": 2 }
            ],
            "items": false
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn prefix_items_with_ref_false_items_are_subsumed_by_max_items_cap() {
        let old = resolve(json!({
            "type": "array",
            "maxItems": 1
        }));
        let new = resolve(json!({
            "$defs": {
                "Never": false
            },
            "type": "array",
            "prefixItems": [{ "const": 1 }],
            "items": {
                "$ref": "#/$defs/Never"
            }
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn single_item_tuples_with_false_items_satisfy_unique_items() {
        let old = resolve(json!({
            "type": "array",
            "uniqueItems": true
        }));
        let new = resolve(json!({
            "type": "array",
            "prefixItems": [{ "const": 1 }],
            "items": false
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn dependent_required_trigger_can_be_admitted_by_subset_pattern_properties() {
        let old = resolve(json!({
            "type": "object",
            "patternProperties": {
                "^x$": true
            },
            "dependentRequired": {
                "x": ["y"]
            },
            "additionalProperties": false
        }));
        let new = resolve(json!({
            "type": "object",
            "patternProperties": {
                "^x$": true
            },
            "additionalProperties": false
        }));

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn dependent_required_transitive_chain_preserves_a_required_dependency() {
        let old = resolve(json!({
            "type": "object",
            "properties": {
                "a": true,
                "c": true
            },
            "dependentRequired": {
                "a": ["c"]
            }
        }));
        let new = resolve(json!({
            "type": "object",
            "properties": {
                "a": true,
                "b": true,
                "c": true,
                "extra": { "type": "string" }
            },
            "dependentRequired": {
                "a": ["b"],
                "b": ["c"]
            }
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn dependent_required_trigger_forbidden_by_subset_property_names_is_vacuous() {
        let old = resolve(json!({
            "type": "object",
            "dependentRequired": {
                "x": ["y"]
            }
        }));
        let new = resolve(json!({
            "type": "object",
            "propertyNames": {
                "pattern": "^z$"
            }
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn dependent_required_trigger_with_unsupported_property_name_pattern_is_not_vacuous() {
        let old = resolve(json!({
            "type": "object",
            "dependentRequired": {
                "x": ["y"]
            }
        }));
        let new = resolve(json!({
            "type": "object",
            "propertyNames": {
                "pattern": "^(?=x$)x$"
            }
        }));

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn dependent_required_trigger_with_recursive_property_names_is_not_vacuous() {
        let old = resolve(json!({
            "type": "object",
            "dependentRequired": {
                "x": ["y"]
            }
        }));
        let raw = json!({
            "$defs": {
                "Name": {
                    "allOf": [
                        { "$ref": "#/$defs/Name" },
                        { "type": "string" }
                    ]
                }
            },
            "type": "object",
            "propertyNames": { "$ref": "#/$defs/Name" }
        });
        let new_document = SchemaDocument::from_json(&raw).unwrap();
        assert!(new_document.is_valid(&json!({ "x": 1 })).unwrap());
        let new = new_document.root().unwrap().clone();

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn dependent_required_trigger_admitted_by_unsupported_pattern_properties_is_not_vacuous() {
        let old = resolve(json!({
            "type": "object",
            "dependentRequired": {
                "x": ["y"]
            }
        }));
        let new = resolve(json!({
            "type": "object",
            "patternProperties": {
                "^(?=x$)x$": true
            },
            "additionalProperties": false
        }));

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn dependent_required_trigger_forbidden_by_matching_pattern_property_is_vacuous() {
        let old = resolve(json!({
            "type": "object",
            "dependentRequired": {
                "x": ["y"]
            }
        }));
        let new = resolve(json!({
            "type": "object",
            "properties": {
                "x": true
            },
            "patternProperties": {
                "^x$": false
            }
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn subset_pattern_properties_must_satisfy_matching_superset_properties() {
        let old = resolve(json!({
            "type": "object",
            "properties": {
                "x": false
            },
            "additionalProperties": true
        }));
        let new = resolve(json!({
            "type": "object",
            "patternProperties": {
                "^x$": true
            },
            "additionalProperties": false
        }));

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn unsupported_superset_pattern_properties_may_constrain_explicit_subset_properties() {
        let old = resolve(json!({
            "type": "object",
            "patternProperties": {
                "^(?=x$)x$": { "type": "integer" }
            },
            "additionalProperties": true
        }));
        let new = resolve(json!({
            "type": "object",
            "properties": {
                "x": true
            },
            "additionalProperties": false
        }));

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn overlapping_pattern_properties_must_preserve_every_superset_constraint() {
        let old = resolve(json!({
            "type": "object",
            "patternProperties": {
                "^x": { "type": "string" },
                "x$": { "type": "integer" }
            },
            "additionalProperties": false
        }));
        let new = resolve(json!({
            "type": "object",
            "patternProperties": {
                "^x": { "type": "string" }
            },
            "additionalProperties": false
        }));

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn closed_object_property_capacity_must_respect_implicit_max_properties() {
        let old = resolve(json!({
            "type": "object",
            "properties": {
                "x": true
            },
            "maxProperties": 1,
            "additionalProperties": false
        }));
        let new = resolve(json!({
            "type": "object",
            "properties": {
                "x": true
            },
            "maxProperties": 3,
            "additionalProperties": false
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn unsupported_subset_pattern_properties_cannot_fall_back_to_additional_properties() {
        let old = resolve(json!({
            "type": "object",
            "properties": {
                "x": { "type": "string" }
            },
            "additionalProperties": true
        }));
        let new = resolve(json!({
            "type": "object",
            "patternProperties": {
                "^\\cC$": { "type": "integer" }
            },
            "additionalProperties": false
        }));

        assert!(!is_subschema_of(&new, &old));
        assert_eq!(
            explain_subschema_failure(&new, &old)
                .expect("failure should be explainable")
                .render("new", "old"),
            "old schema #/properties/x: property 'x' can appear with values the comparison target rejects",
        );
    }

    #[test]
    fn subset_pattern_properties_do_not_shadow_matching_explicit_subset_properties() {
        let old = resolve(json!({
            "type": "object",
            "properties": {
                "x": { "type": "integer" }
            },
            "additionalProperties": true
        }));
        let new = resolve(json!({
            "type": "object",
            "properties": {
                "x": { "type": "integer" }
            },
            "patternProperties": {
                "^x$": true
            },
            "additionalProperties": false
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn subset_property_patterns_can_jointly_satisfy_a_matching_superset_property() {
        let old = resolve(json!({
            "type": "object",
            "properties": {
                "x": { "type": "integer" }
            },
            "additionalProperties": true
        }));
        let new = resolve(json!({
            "type": "object",
            "patternProperties": {
                "^x$": true,
                "^.$": { "type": "integer" }
            },
            "additionalProperties": false
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn subset_pattern_properties_can_tighten_matching_explicit_subset_properties() {
        let old = resolve(json!({
            "type": "object",
            "properties": {
                "x": { "type": "integer" }
            },
            "additionalProperties": true
        }));
        let new = resolve(json!({
            "type": "object",
            "properties": {
                "x": true
            },
            "patternProperties": {
                "^x$": { "type": "integer" }
            },
            "additionalProperties": false
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn subset_additional_properties_must_satisfy_unmatched_superset_properties() {
        let old = resolve(json!({
            "type": "object",
            "properties": {
                "x": false
            },
            "additionalProperties": true
        }));
        let new = resolve(json!({
            "type": "object",
            "additionalProperties": true
        }));

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn subset_pattern_properties_can_fall_back_to_superset_additional_properties() {
        let old = resolve(json!({
            "type": "object",
            "additionalProperties": true
        }));
        let new = resolve(json!({
            "type": "object",
            "patternProperties": {
                "^x$": {
                    "type": "integer"
                }
            }
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn subset_additional_properties_must_satisfy_unmatched_superset_pattern_properties() {
        let old = resolve(json!({
            "type": "object",
            "additionalProperties": true
        }));
        let new = resolve(json!({
            "type": "object",
            "patternProperties": {
                "^x$": {
                    "type": "integer"
                }
            }
        }));

        assert!(!is_subschema_of(&old, &new));
    }

    #[test]
    fn number_enum_with_float_form_integer_values_can_be_subsumed_by_integer_const() {
        let sub = resolve(json!({
            "type": "number",
            "minimum": 0,
            "enum": [9_007_199_254_740_994.0_f64]
        }));
        let sup = resolve(json!({
            "const": 9_007_199_254_740_994_i64
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn numeric_const_subset_uses_json_schema_number_equality() {
        let int_const = resolve(json!({
            "const": 1
        }));
        let float_const = resolve(json!({
            "const": 1.0
        }));

        assert!(is_subschema_of(&int_const, &float_const));
        assert!(is_subschema_of(&float_const, &int_const));
    }

    #[test]
    fn numeric_enum_subset_uses_json_schema_number_equality() {
        let int_enum = resolve(json!({
            "enum": [1]
        }));
        let float_enum = resolve(json!({
            "enum": [1.0]
        }));

        assert!(is_subschema_of(&int_enum, &float_enum));
        assert!(is_subschema_of(&float_enum, &int_enum));
    }

    #[test]
    fn differing_string_patterns_are_not_treated_as_subsumed() {
        let old = resolve(json!({
            "type": "string",
            "pattern": "^a+$"
        }));
        let new = resolve(json!({
            "type": "string",
            "pattern": "^b+$"
        }));

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn differing_string_formats_remain_subsumed_as_annotations() {
        let old = resolve(json!({
            "type": "string",
            "format": "email"
        }));
        let new = resolve(json!({
            "type": "string",
            "format": "uuid"
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn identical_string_language_constraints_remain_subsumed() {
        let old = resolve(json!({
            "type": "string",
            "pattern": "^a+$"
        }));
        let new = resolve(json!({
            "type": "string",
            "pattern": "^a+$",
            "minLength": 1
        }));

        assert!(is_subschema_of(&new, &old));
    }
}
