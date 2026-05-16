//! Structural subset checks over the resolved schema IR.
//!
//! `is_subschema_of(sub, sup)` answers whether every instance accepted by
//! `sub` is also accepted by `sup`.  The checker is intentionally conservative
//! for hard cases such as regex implication and `oneOf` on the right-hand side.

use crate::{SchemaNode, json_pointer::JsonPointer};
use json_schema_ast::{
    CountRange, IntegerBounds, IntegerMultipleOf, NodeId, NumberBound, NumberBounds,
    NumberMultipleOf, SchemaNodeKind, json_values_equal,
};
use serde_json::Value;
use std::collections::HashSet;

mod array;
mod object;
mod scalar;

use scalar::{
    StringConstraints, check_enum_inclusion, integer_constraints_subsumed_by_number,
    string_constraints_subsumed,
};

#[derive(Default)]
pub(super) struct SubschemaCheckContext {
    active_pairs: HashSet<(NodeId, NodeId)>,
}

impl SubschemaCheckContext {
    pub(super) fn superset_contains_value(&mut self, sup: &SchemaNode, value: &Value) -> bool {
        sup.accepts_value(value)
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
    analyze_subschema_with_context(
        sub,
        sup,
        &mut SubschemaCheckContext::default(),
        ExplanationMode::Explain,
    )
    .explanation
}

pub(super) fn is_subschema_of_with_context(
    sub: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    analyze_subschema_with_context(sub, sup, context, ExplanationMode::VerdictOnly).is_subschema
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

    let recursion_key = (sub.id(), sup.id());
    if !context.active_pairs.insert(recursion_key) {
        return SubschemaAnalysis::compatible();
    }

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
                explain_any_of_to_any_of_failure(subs, sups, sup)
            })
        }
        (OneOf(subs), OneOf(sups)) => {
            let is_subschema = subs.iter().all(|branch| {
                analyze_subschema_with_context(branch, sup, context, mode).is_subschema
            });
            SubschemaAnalysis::from_check(is_subschema, mode, || {
                explain_one_of_to_one_of_failure(subs, sups)
            })
        }
        (AnyOf(subs), _) | (OneOf(subs), _) => {
            let is_subschema = subs.iter().all(|branch| {
                analyze_subschema_with_context(branch, sup, context, mode).is_subschema
            });
            SubschemaAnalysis::from_check(is_subschema, mode, || {
                explain_subset_union_failure(subs, sup)
            })
        }
        (AllOf(subs), _) => {
            let is_subschema = subs.iter().all(|schema| {
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
                explain_superset_any_of_failure(sub, sups)
            })
        }
        (_, OneOf(_)) => {
            SubschemaAnalysis::from_check(false, mode, || explain_schema_kind_gap(sub, sup))
        }
        (_, AllOf(sups)) => {
            let is_subschema = sups.iter().all(|schema| {
                analyze_subschema_with_context(sub, schema, context, mode).is_subschema
            });
            SubschemaAnalysis::from_check(is_subschema, mode, || None)
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
                !matches!(sup.kind(), Any | BoolSchema(true)),
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
            BoolSchema(false) => SubschemaAnalysis::from_check(
                matches!(sub.kind(), BoolSchema(true) | Any),
                mode,
                || explain_schema_kind_gap(sub, sup),
            ),
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
            || explain_type_constraint_failure(sub, sup),
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

fn explain_any_of_to_any_of_failure(
    subs: &[SchemaNode],
    sups: &[SchemaNode],
    sup: &SchemaNode,
) -> Option<SubschemaExplanation> {
    subs.iter().enumerate().find_map(|(index, branch)| {
        (!sups
            .iter()
            .any(|sup_branch| is_subschema_of(branch, sup_branch)))
        .then(|| {
            sups.get(index)
                .and_then(|sup_branch| explain_subschema_failure(branch, sup_branch))
                .or_else(|| explain_branch_against_union(branch, sups))
                .or_else(|| explain_subschema_failure(branch, sup))
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
) -> Option<SubschemaExplanation> {
    subs.iter().enumerate().find_map(|(index, branch)| {
        sups.get(index)
            .and_then(|sup_branch| explain_subschema_failure(branch, sup_branch))
            .map(|detail| detail.under_one_of_branch(index))
    })
}

fn explain_subset_union_failure(
    subs: &[SchemaNode],
    sup: &SchemaNode,
) -> Option<SubschemaExplanation> {
    subs.iter().enumerate().find_map(|(index, branch)| {
        (!is_subschema_of(branch, sup)).then(|| {
            explain_branch_against_sup(branch, sup)
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
) -> Option<SubschemaExplanation> {
    match sup.kind() {
        SchemaNodeKind::AnyOf(sups) => explain_branch_against_union(branch, sups),
        _ => explain_subschema_failure(branch, sup),
    }
}

fn explain_branch_against_union(
    branch: &SchemaNode,
    sups: &[SchemaNode],
) -> Option<SubschemaExplanation> {
    explain_superset_any_of_failure(branch, sups)
}

fn explain_superset_any_of_failure(
    sub: &SchemaNode,
    sups: &[SchemaNode],
) -> Option<SubschemaExplanation> {
    sups.iter()
        .enumerate()
        .find_map(|(index, branch)| {
            explain_subschema_failure(sub, branch)
                .map(|detail| detail.under_superset_any_of_branch(index))
        })
        .or_else(|| {
            Some(SubschemaExplanation::new(
                "value shape does not fit any previous anyOf branch",
            ))
        })
}

fn explain_type_constraint_failure(
    sub: &SchemaNode,
    sup: &SchemaNode,
) -> Option<SubschemaExplanation> {
    use SchemaNodeKind::*;

    match (sub.kind(), sup.kind()) {
        (
            String {
                length: sub_length,
                pattern: sub_pattern,
                format: sub_format,
                enumeration: sub_enum,
            },
            String {
                length: sup_length,
                pattern: sup_pattern,
                format: sup_format,
                enumeration: sup_enum,
            },
        ) => explain_string_constraints(
            StringConstraints {
                length: *sub_length,
                pattern: sub_pattern.as_ref(),
                format: sub_format.as_deref(),
                enumeration: sub_enum.as_deref(),
            },
            StringConstraints {
                length: *sup_length,
                pattern: sup_pattern.as_ref(),
                format: sup_format.as_deref(),
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
                dependent_required: _,
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
            if !sup_property_count.contains_range(*sub_property_count) {
                return Some(SubschemaExplanation::new(format!(
                    "object property count range {} is not contained by required range {}",
                    format_count_range(*sub_property_count),
                    format_count_range(*sup_property_count),
                )));
            }
            if let Some(detail) = explain_enumeration_gap(sub_enum.as_deref(), sup_enum.as_deref())
            {
                return Some(detail);
            }

            for (property, sub_schema) in sub_properties {
                if sup_properties.contains_key(property) {
                    continue;
                }
                if sup_pattern_properties.is_empty() && !is_subschema_of(sub_schema, sup_additional)
                {
                    return Some(SubschemaExplanation::new(format!(
                        "property '{property}' can appear with values the comparison target rejects",
                    ))
                    .at_property(property));
                }
            }

            let mut best_property_failure = None;
            for (property, sub_schema) in sub_properties {
                if let Some(sup_schema) = sup_properties.get(property)
                    && !is_subschema_of(sub_schema, sup_schema)
                {
                    let detail =
                        explain_subschema_failure(sub_schema, sup_schema).unwrap_or_else(|| {
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
                if !is_subschema_of(&sub_pattern_property.schema, sup_schema) {
                    return Some(SubschemaExplanation::new(format!(
                        "pattern property '{pattern}' can accept values the comparison target rejects",
                    ))
                    .at_pattern_property(pattern));
                }
            }

            if !is_subschema_of(sub_additional, sup_additional) {
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
                if !is_subschema_of(sub_additional, &sup_pattern_property.schema) {
                    return Some(SubschemaExplanation::new(format!(
                        "additional properties matching pattern '{pattern}' may violate the required pattern-property schema",
                    ))
                    .in_superset()
                    .at_pattern_property(pattern));
                }
            }

            if !is_subschema_of(sub_property_names, sup_property_names) {
                let detail = explain_subschema_failure(sub_property_names, sup_property_names)
                    .unwrap_or_else(|| {
                        SubschemaExplanation::new(
                            "property names are not contained by the comparison target",
                        )
                    });
                return Some(detail.under_property_names());
            }

            for (trigger, dependencies) in sup_dependent_required {
                if let Some(dependency) = dependencies
                    .iter()
                    .find(|dependency| !sub_required.contains(*dependency))
                {
                    return Some(SubschemaExplanation::new(format!(
                        "property '{trigger}' may appear without dependent property '{dependency}'",
                    ))
                    .in_superset()
                    .at_dependent_required(trigger));
                }
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
                if !is_subschema_of(sub_item, sup_item) {
                    let detail =
                        explain_subschema_failure(sub_item, sup_item).unwrap_or_else(|| {
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
                && !is_subschema_of(sub_items, sup_items)
            {
                let detail = explain_subschema_failure(sub_items, sup_items).unwrap_or_else(|| {
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
                            && is_subschema_of(&sub_contains.schema, &sup_contains.schema)
                    })
                    || (sub_item_count.min() >= sup_count.min()
                        && all_array_item_schemas_subsumed_by_for_explanation(
                            sub_prefix_items,
                            sub_items,
                            sub_item_count.max(),
                            &sup_contains.schema,
                        ));
                if !lower_bound_ok {
                    return Some(SubschemaExplanation::new(format!(
                        "array values do not guarantee at least {} item(s) matching the required contains schema",
                        sup_count.min(),
                    ))
                    .in_superset()
                    .at_keyword("contains"));
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
    if sup.format.is_some() && sub.format != sup.format {
        return Some(SubschemaExplanation::new(
            "string format does not preserve the comparison target's required format",
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
) -> bool {
    for (index, prefix_item) in prefix_items.iter().enumerate() {
        if !array_index_can_exist(max_items, index) {
            return true;
        }
        if !is_subschema_of(prefix_item, sup_schema) {
            return false;
        }
    }

    !array_index_can_exist(max_items, prefix_items.len()) || is_subschema_of(items, sup_schema)
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
                format: sub_format,
                enumeration: sub_enum,
            },
            String {
                length: sup_length,
                pattern: sup_pattern,
                format: sup_format,
                enumeration: sup_enum,
            },
        ) => string_constraints_subsumed(
            StringConstraints {
                length: *sub_length,
                pattern: sub_pattern.as_ref(),
                format: sub_format.as_deref(),
                enumeration: sub_enum.as_deref(),
            },
            StringConstraints {
                length: *sup_length,
                pattern: sup_pattern.as_ref(),
                format: sup_format.as_deref(),
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
    fn differing_string_formats_are_not_treated_as_subsumed() {
        let old = resolve(json!({
            "type": "string",
            "format": "email"
        }));
        let new = resolve(json!({
            "type": "string",
            "format": "uuid"
        }));

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn identical_string_language_constraints_remain_subsumed() {
        let old = resolve(json!({
            "type": "string",
            "pattern": "^a+$",
            "format": "email"
        }));
        let new = resolve(json!({
            "type": "string",
            "pattern": "^a+$",
            "format": "email",
            "minLength": 1
        }));

        assert!(is_subschema_of(&new, &old));
    }
}
