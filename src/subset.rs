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

    fn under_conditional_branch(mut self, keyword: &'static str) -> Self {
        self.schema_path.prepend([keyword]);
        self.under(format!("conditional {keyword} branch"))
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

    // A few applicator shapes (notably `allOf` with an enum/const child, and
    // small bounded integer domains) have a finite syntactic upper bound even
    // though they are not represented as a typed `enumeration` in the IR.  If
    // every value in that upper bound is definitely accepted by the superset,
    // the whole subset language is covered.  Unlike the direct enum case
    // above, failure to prove this is not evidence of incompatibility: the
    // bound may include values rejected by another conjunct, so just fall
    // through to the structural checker.
    if let Some(values) = finite_schema_value_superset(sub)
        && values.iter().all(|value| {
            context.schema_definitely_rejects_value(sub, value)
                || context.superset_contains_value(sup, value)
        })
    {
        return SubschemaAnalysis::compatible();
    }

    // Broad schemas such as `{}` hit the `Any` arm below before the normal
    // superset-`anyOf` handler. Give obvious whole-type unions a chance first.
    if let SchemaNodeKind::AnyOf(branches) = sup.kind()
        && any_of_obvious_type_cover_contains(sub, branches)
    {
        return SubschemaAnalysis::compatible();
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
        (AllOf(subs), AllOf(sups)) => {
            // Intersections on both sides can be compared conjunct-wise: for
            // each required superset conjunct, it is enough that one subset
            // conjunct implies it. This avoids losing obvious equivalences when
            // generated schemas split `required` and `properties` across
            // parallel allOf wrappers.
            let is_subschema = sups.iter().all(|sup_conjunct| {
                subs.iter().any(|sub_conjunct| {
                    analyze_subschema_with_context(sub_conjunct, sup_conjunct, context, mode)
                        .is_subschema
                })
            });
            SubschemaAnalysis::from_check(is_subschema, mode, || {
                explain_superset_all_of_failure(sub, sups, context)
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
        (AllOf(_), OneOf(sups)) => {
            // Do not let the generic left-allOf shortcut hide a partitioned
            // oneOf on the right.  The partition proof reasons about the whole
            // intersection, which is exactly what is needed for split allOf
            // wrappers around tagged object branches.
            let is_subschema = one_of_type_partition_contains(sub, sups, context, mode);
            SubschemaAnalysis::from_check(is_subschema, mode, || explain_schema_kind_gap(sub, sup))
        }
        (
            AllOf(subs),
            Number {
                multiple_of: None,
                enumeration: None,
                ..
            },
        ) => {
            // Generated schemas often split `type: number` and numeric bounds
            // across parallel allOf conjuncts. A single conjunct may not imply
            // the target range, but the whole intersection can. Keep this
            // deliberately limited to plain number ranges on the right; other
            // numeric keywords (multipleOf/enums) still use the normal path.
            let is_subschema = split_allof_numeric_range_subset_of_number(sub, sup)
                || subs.iter().any(|schema| {
                    analyze_subschema_with_context(schema, sup, context, mode).is_subschema
                });
            SubschemaAnalysis::from_check(is_subschema, mode, || explain_schema_kind_gap(sub, sup))
        }
        (AllOf(subs), Integer { .. }) if integer_schema_is_plain_range(sup) => {
            // Split integer schemas need one extra guard beyond numeric
            // interval containment: a direct integer conjunct must force
            // integrality for the whole intersection.
            let is_subschema = split_allof_integer_range_subset_of_integer(sub, sup)
                || subs.iter().any(|schema| {
                    analyze_subschema_with_context(schema, sup, context, mode).is_subschema
                });
            SubschemaAnalysis::from_check(is_subschema, mode, || explain_schema_kind_gap(sub, sup))
        }
        (
            AllOf(subs),
            String {
                pattern: None,
                format: None,
                enumeration: None,
                ..
            },
        ) => {
            // Generated schemas often split `type: string` and min/maxLength
            // across parallel allOf conjuncts. A single conjunct may not imply
            // the target range, but the whole intersection can.
            let is_subschema = split_allof_string_length_subset_of_string(sub, sup)
                || subs.iter().any(|schema| {
                    analyze_subschema_with_context(schema, sup, context, mode).is_subschema
                });
            SubschemaAnalysis::from_check(is_subschema, mode, || explain_schema_kind_gap(sub, sup))
        }
        (AllOf(subs), Array { .. }) if array_schema_is_plain_count(sup) => {
            let is_subschema = split_allof_array_length_subset_of_array(sub, sup)
                || subs.iter().any(|schema| {
                    analyze_subschema_with_context(schema, sup, context, mode).is_subschema
                });
            SubschemaAnalysis::from_check(is_subschema, mode, || explain_schema_kind_gap(sub, sup))
        }
        (AllOf(subs), Object { .. }) if object_schema_is_plain_count(sup) => {
            let is_subschema = split_allof_object_count_subset_of_object(sub, sup)
                || subs.iter().any(|schema| {
                    analyze_subschema_with_context(schema, sup, context, mode).is_subschema
                });
            SubschemaAnalysis::from_check(is_subschema, mode, || explain_schema_kind_gap(sub, sup))
        }
        (
            AllOf(_),
            IfThenElse {
                if_schema: sup_if,
                then_schema: sup_then,
                else_schema: sup_else,
            },
        ) => {
            // Reason about the whole intersection against a conditional; a
            // single allOf conjunct often only carries the type or the `not`
            // guard, neither of which is sufficient on its own.
            let is_subschema = superset_conditional_contains(
                sub,
                sup,
                sup_if,
                sup_then.as_ref(),
                sup_else.as_ref(),
                context,
            );
            SubschemaAnalysis::from_check(is_subschema, mode, || explain_schema_kind_gap(sub, sup))
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
            let conditional_branches_fit_union = if let IfThenElse {
                then_schema: Some(then_branch),
                else_schema: Some(else_branch),
                ..
            } = sub.kind()
            {
                analyze_subschema_with_context(then_branch, sup, context, mode).is_subschema
                    && analyze_subschema_with_context(else_branch, sup, context, mode).is_subschema
            } else {
                false
            };
            let is_subschema = conditional_branches_fit_union
                || sups.iter().any(|branch| {
                    analyze_subschema_with_context(sub, branch, context, mode).is_subschema
                })
                || any_of_obvious_type_cover_contains(sub, sups)
                || finite_subset_values_fit_anyof(sub, sups, context);
            SubschemaAnalysis::from_check(is_subschema, mode, || {
                explain_superset_any_of_failure(sub, sups, context)
            })
        }
        (_, OneOf(sups)) => {
            // A common OpenAPI/JSON Schema shape uses `oneOf` as a tagged or
            // primitive-type union. It is sound to accept a subset when we can
            // prove it fits one branch and its possible JSON types cannot
            // overlap any of the other branches: then every subset instance
            // satisfies exactly one branch. Keep the proof deliberately
            // syntactic; harder overlapping oneOf cases remain conservative.
            let is_subschema = one_of_type_partition_contains(sub, sups, context, mode);
            SubschemaAnalysis::from_check(is_subschema, mode, || explain_schema_kind_gap(sub, sup))
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
            IfThenElse {
                if_schema: sub_if,
                then_schema: sub_then,
                else_schema: sub_else,
            },
            IfThenElse {
                if_schema: sup_if,
                then_schema: sup_then,
                else_schema: sup_else,
            },
        ) if sub_if == sup_if => {
            // Identical conditions partition the instance space in the same
            // way, so branch-wise implication is sufficient. Missing
            // branches are the JSON Schema `true` schema.
            let is_subschema = optional_conditional_branch_subsumed(
                sub_then.as_ref(),
                sup_then.as_ref(),
                context,
                mode,
            ) && optional_conditional_branch_subsumed(
                sub_else.as_ref(),
                sup_else.as_ref(),
                context,
                mode,
            );
            SubschemaAnalysis::from_check(is_subschema, mode, || {
                explain_identical_conditional_failure(
                    sub_then.as_ref(),
                    sup_then.as_ref(),
                    sub_else.as_ref(),
                    sup_else.as_ref(),
                    context,
                )
            })
        }
        (
            _,
            IfThenElse {
                if_schema: sup_if,
                then_schema: sup_then,
                else_schema: sup_else,
            },
        ) => {
            // A conditional on the right is satisfied when the subset fits
            // both branches, or when it is known to fall on just one side of
            // the guard. Keep the detailed proof in a helper so split allOf
            // wrappers can use the same whole-schema reasoning.
            let is_subschema = superset_conditional_contains(
                sub,
                sup,
                sup_if,
                sup_then.as_ref(),
                sup_else.as_ref(),
                context,
            );
            SubschemaAnalysis::from_check(is_subschema, mode, || explain_schema_kind_gap(sub, sup))
        }
        (
            IfThenElse {
                then_schema,
                else_schema,
                ..
            },
            _,
        ) => {
            // If both conditional branches are themselves subsets of the
            // target, the whole conditional is too. This deliberately ignores
            // the condition (a stronger proof obligation) and therefore stays
            // sound even for unsupported condition schemas.
            let is_subschema = conditional_branches_subsumed_by(
                then_schema.as_ref(),
                else_schema.as_ref(),
                sup,
                context,
                mode,
            );
            SubschemaAnalysis::from_check(is_subschema, mode, || {
                Some(SubschemaExplanation::new(
                    "conditional branches are not all contained by the comparison target",
                ))
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

        (Not(sub_negated), Not(sup_negated)) => {
            // Negation is contravariant: if the superset's inner schema is a
            // subset of the subset's inner schema (`sup_inner <= sub_inner`),
            // then `not(sub_inner)` is a subset of `not(sup_inner)`. This
            // catches common generated refinements such as `not: {type:
            // string}` being safely narrower than `not: {enum: ["x"]}`
            // without opening up arbitrary complement reasoning.
            let inner_implication = analyze_subschema_with_context(
                sup_negated,
                sub_negated,
                context,
                ExplanationMode::VerdictOnly,
            )
            .is_subschema;
            let disjoint_fallback = !inner_implication
                && schemas_definitely_disjoint_for_negation(sub, sup_negated, context);
            SubschemaAnalysis::from_check(inner_implication || disjoint_fallback, mode, || {
                Some(SubschemaExplanation::new(
                    "negated inner schema is not known to contain the comparison target's exclusion",
                ))
            })
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
            _ => {
                // To prove `sub <= not(S)` it is enough to prove that `sub`
                // and `S` are disjoint. Reuse the deliberately small
                // disjointness facts maintained for oneOf partitioning: JSON
                // type separation, simple numeric intervals, and finite
                // object discriminator values. This catches common schemas
                // such as `{type: string}` versus `{not: {type: number}}`
                // without treating arbitrary negation as implication.
                let disjoint = schemas_definitely_disjoint_for_negation(sub, sup_negated, context);
                SubschemaAnalysis::from_check(disjoint, mode, || explain_schema_kind_gap(sub, sup))
            }
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

fn optional_sup_conditional_branch_contains(
    sub: &SchemaNode,
    sup_branch: Option<&SchemaNode>,
    context: &mut SubschemaCheckContext,
    mode: ExplanationMode,
) -> bool {
    match sup_branch {
        None => true,
        Some(branch) => analyze_subschema_with_context(sub, branch, context, mode).is_subschema,
    }
}

fn optional_conditional_branch_subsumed(
    sub_branch: Option<&SchemaNode>,
    sup_branch: Option<&SchemaNode>,
    context: &mut SubschemaCheckContext,
    mode: ExplanationMode,
) -> bool {
    match (sub_branch, sup_branch) {
        (_, None) => true,
        (Some(sub), Some(sup)) => {
            analyze_subschema_with_context(sub, sup, context, mode).is_subschema
        }
        (None, Some(sup)) => schema_is_trivially_universal(sup),
    }
}

fn explain_identical_conditional_failure(
    sub_then: Option<&SchemaNode>,
    sup_then: Option<&SchemaNode>,
    sub_else: Option<&SchemaNode>,
    sup_else: Option<&SchemaNode>,
    context: &mut SubschemaCheckContext,
) -> Option<SubschemaExplanation> {
    if !optional_conditional_branch_subsumed(
        sub_then,
        sup_then,
        context,
        ExplanationMode::VerdictOnly,
    ) {
        return explain_optional_conditional_branch_failure(sub_then, sup_then, context)
            .map(|detail| detail.under_conditional_branch("then"));
    }
    if !optional_conditional_branch_subsumed(
        sub_else,
        sup_else,
        context,
        ExplanationMode::VerdictOnly,
    ) {
        return explain_optional_conditional_branch_failure(sub_else, sup_else, context)
            .map(|detail| detail.under_conditional_branch("else"));
    }
    None
}

fn explain_optional_conditional_branch_failure(
    sub_branch: Option<&SchemaNode>,
    sup_branch: Option<&SchemaNode>,
    context: &mut SubschemaCheckContext,
) -> Option<SubschemaExplanation> {
    match (sub_branch, sup_branch) {
        (Some(sub), Some(sup)) => explain_subschema_failure_with_context(sub, sup, context),
        (None, Some(_)) => Some(SubschemaExplanation::new(
            "unconstrained conditional branch is not contained by the comparison target branch",
        )),
        _ => None,
    }
}

fn conditional_branches_subsumed_by(
    then_branch: Option<&SchemaNode>,
    else_branch: Option<&SchemaNode>,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
    mode: ExplanationMode,
) -> bool {
    match (then_branch, else_branch) {
        (Some(then_branch), Some(else_branch)) => {
            analyze_subschema_with_context(then_branch, sup, context, mode).is_subschema
                && analyze_subschema_with_context(else_branch, sup, context, mode).is_subschema
        }
        // A missing branch is unconstrained on its side of the condition. We
        // only prove that case when the target is syntactically universal.
        _ => schema_is_trivially_universal(sup),
    }
}

fn superset_conditional_contains(
    sub: &SchemaNode,
    sup: &SchemaNode,
    sup_if: &SchemaNode,
    sup_then: Option<&SchemaNode>,
    sup_else: Option<&SchemaNode>,
    context: &mut SubschemaCheckContext,
) -> bool {
    let covered_by_then = optional_sup_conditional_branch_contains(
        sub,
        sup_then,
        context,
        ExplanationMode::VerdictOnly,
    );
    let covered_by_else = optional_sup_conditional_branch_contains(
        sub,
        sup_else,
        context,
        ExplanationMode::VerdictOnly,
    );
    let unconditional_branch_cover = covered_by_then && covered_by_else;

    let condition_always_true =
        analyze_subschema_with_context(sub, sup_if, context, ExplanationMode::VerdictOnly)
            .is_subschema;
    let sub_mask = possible_json_type_mask(sub);
    let condition_always_false = sub_mask == 0
        || schemas_definitely_disjoint_for_partition(sub, sub_mask, sup_if, context)
        || schema_definitely_excludes_schema(sub, sup_if, context);
    let guarded_branch_cover =
        (condition_always_true && covered_by_then) || (condition_always_false && covered_by_else);

    let branchwise_subset_cover = if let SchemaNodeKind::IfThenElse {
        then_schema,
        else_schema,
        ..
    } = sub.kind()
    {
        conditional_branches_subsumed_by(
            then_schema.as_ref(),
            else_schema.as_ref(),
            sup,
            context,
            ExplanationMode::VerdictOnly,
        )
    } else {
        false
    };

    unconditional_branch_cover || guarded_branch_cover || branchwise_subset_cover
}

const JSON_TYPE_NULL: u8 = 1 << 0;
const JSON_TYPE_BOOL: u8 = 1 << 1;
const JSON_TYPE_NUMBER: u8 = 1 << 2;
const JSON_TYPE_STRING: u8 = 1 << 3;
const JSON_TYPE_ARRAY: u8 = 1 << 4;
const JSON_TYPE_OBJECT: u8 = 1 << 5;
const JSON_TYPE_ALL: u8 = JSON_TYPE_NULL
    | JSON_TYPE_BOOL
    | JSON_TYPE_NUMBER
    | JSON_TYPE_STRING
    | JSON_TYPE_ARRAY
    | JSON_TYPE_OBJECT;

/// Prove a schema is covered by an `anyOf` whose branches obviously accept
/// whole JSON value types. This is intentionally much weaker than general
/// union reasoning: it only fires when every possible type of `sub` has a
/// branch that accepts *all* values of that type. That makes it useful for
/// generated `anyOf` type unions while staying sound for unsupported keywords.
fn any_of_obvious_type_cover_contains(sub: &SchemaNode, branches: &[SchemaNode]) -> bool {
    let mask = possible_json_type_mask(sub);
    if mask == 0 {
        return true;
    }

    for bit in [
        JSON_TYPE_NULL,
        JSON_TYPE_BOOL,
        JSON_TYPE_NUMBER,
        JSON_TYPE_STRING,
        JSON_TYPE_ARRAY,
        JSON_TYPE_OBJECT,
    ] {
        if mask & bit == 0 {
            continue;
        }
        if !branches
            .iter()
            .any(|branch| schema_obviously_accepts_json_type(branch, bit))
        {
            return false;
        }
    }
    true
}

/// A small syntactic recognizer for schemas that accept every value of one JSON
/// type. Returning false is always conservative; returning true must mean the
/// entire type is admitted by raw JSON Schema validation.
fn schema_obviously_accepts_json_type(schema: &SchemaNode, bit: u8) -> bool {
    fn inner(schema: &SchemaNode, bit: u8, active: &mut HashSet<NodeId>) -> bool {
        if !active.insert(schema.id()) {
            return false;
        }
        use SchemaNodeKind::*;
        let result = match schema.kind() {
            BoolSchema(true) | Any => true,
            BoolSchema(false) => false,
            // `null` has exactly one inhabitant, so a const-null schema admits
            // the whole null JSON type. Other const values are only a single
            // member of their type and cannot witness whole-type acceptance.
            Const(value) => bit == JSON_TYPE_NULL && value.is_null(),
            Enum(values) => match bit {
                JSON_TYPE_NULL => values.iter().any(|value| value.is_null()),
                JSON_TYPE_BOOL => {
                    values.iter().any(|value| value == &Value::Bool(false))
                        && values.iter().any(|value| value == &Value::Bool(true))
                }
                _ => false,
            },
            Not(child) if matches!(child.kind(), BoolSchema(false)) => true,
            AllOf(children) => children.iter().all(|child| inner(child, bit, active)),
            AnyOf(children) => children.iter().any(|child| inner(child, bit, active)),
            String {
                length,
                pattern,
                format,
                enumeration,
            } => {
                bit == JSON_TYPE_STRING
                    && length.min() == 0
                    && length.max().is_none()
                    && pattern.is_none()
                    && format.is_none()
                    && enumeration.is_none()
            }
            Number {
                bounds,
                multiple_of,
                enumeration,
            } => {
                bit == JSON_TYPE_NUMBER
                    && bounds.lower() == NumberBound::Unbounded
                    && bounds.upper() == NumberBound::Unbounded
                    && multiple_of.is_none()
                    && enumeration.is_none()
            }
            Boolean { enumeration } => bit == JSON_TYPE_BOOL && enumeration.is_none(),
            Null { enumeration } => bit == JSON_TYPE_NULL && enumeration.is_none(),
            Object {
                properties,
                pattern_properties,
                required,
                additional,
                property_names,
                property_count,
                dependent_required,
                enumeration,
            } => {
                bit == JSON_TYPE_OBJECT
                    && properties.is_empty()
                    && pattern_properties.is_empty()
                    && required.is_empty()
                    && property_count.min() == 0
                    && property_count.max().is_none()
                    && dependent_required.is_empty()
                    && enumeration.is_none()
                    && schema_is_trivially_universal(additional)
                    && (schema_is_trivially_universal(property_names)
                        || inner(property_names, JSON_TYPE_STRING, active))
            }
            Array {
                prefix_items,
                items,
                item_count,
                contains,
                unique_items,
                enumeration,
            } => {
                bit == JSON_TYPE_ARRAY
                    && prefix_items.is_empty()
                    && item_count.min() == 0
                    && item_count.max().is_none()
                    && contains.is_none()
                    && !*unique_items
                    && enumeration.is_none()
                    && schema_is_trivially_universal(items)
            }
            // Integer schemas do not accept every JSON number (fractional
            // numbers remain), and conditionals/oneOf require more exactness
            // than this helper promises.
            Integer { .. } | OneOf(_) | IfThenElse { .. } | Not(_) => false,
            _ => false,
        };
        active.remove(&schema.id());
        result
    }

    inner(schema, bit, &mut HashSet::new())
}

/// Prove a subset is accepted by a `oneOf` whose alternatives are separated
/// by JSON value type. This intentionally does not try to reason about
/// overlapping numeric ranges or object tags; it only uses an upper bound on
/// the set of JSON types a schema can accept. If that upper bound is disjoint
/// from every non-selected branch, a value that fits the selected branch can
/// match exactly one branch.
fn one_of_type_partition_contains(
    sub: &SchemaNode,
    branches: &[SchemaNode],
    context: &mut SubschemaCheckContext,
    mode: ExplanationMode,
) -> bool {
    let sub_mask = possible_json_type_mask(sub);
    if sub_mask == 0 {
        return true;
    }

    for (index, branch) in branches.iter().enumerate() {
        if !analyze_subschema_with_context(sub, branch, context, mode).is_subschema {
            continue;
        }

        let disjoint_from_others = branches.iter().enumerate().all(|(other_index, other)| {
            other_index == index
                || schemas_definitely_disjoint_for_partition(sub, sub_mask, other, context)
        });
        if disjoint_from_others {
            return true;
        }
    }

    finite_subset_values_fit_oneof_exactly(sub, branches, context)
}

/// Prove a finite-language subset is contained in an anyOf even when its
/// values are split across branches. We only rely on a finite upper bound for
/// the subset and definite positive membership in at least one branch, so
/// unsupported evaluators keep this conservative rather than unsound.
fn finite_subset_values_fit_anyof(
    sub: &SchemaNode,
    branches: &[SchemaNode],
    context: &mut SubschemaCheckContext,
) -> bool {
    let Some(values) = finite_schema_value_superset(sub) else {
        return false;
    };

    values.into_iter().all(|value| {
        context.schema_definitely_rejects_value(sub, &value)
            || branches
                .iter()
                .any(|branch| context.superset_contains_value(branch, &value))
    })
}

/// Prove a finite-language subset is contained in a oneOf even when its
/// values are split across multiple branches. `finite_schema_value_superset`
/// is an upper bound, so checking every candidate is conservative; for each
/// candidate that might be accepted by `sub`, require one branch that is
/// definitely accepting and all other branches definitely rejecting. This
/// avoids treating overlapping oneOfs as unions while handling common enum /
/// const partitions.
fn finite_subset_values_fit_oneof_exactly(
    sub: &SchemaNode,
    branches: &[SchemaNode],
    context: &mut SubschemaCheckContext,
) -> bool {
    let Some(values) = finite_schema_value_superset(sub) else {
        return false;
    };

    values.into_iter().all(|value| {
        if context.schema_definitely_rejects_value(sub, &value) {
            return true;
        }

        let mut accepting = 0usize;
        for branch in branches {
            if context.superset_contains_value(branch, &value) {
                accepting += 1;
                if accepting > 1 {
                    return false;
                }
            } else if !context.schema_definitely_rejects_value(branch, &value) {
                return false;
            }
        }
        accepting == 1
    })
}

/// Prove disjointness for a negated target, including simple applicator
/// structure in the negated schema.  This is a sufficient (not complete)
/// proof: an `anyOf`/`oneOf` is disjoint when every branch is disjoint, while
/// an `allOf` is disjoint when any conjunct is disjoint.  The symmetric rules
/// for applicators on the left make this useful for generated wrappers without
/// constructing temporary `not` nodes.
fn schemas_definitely_disjoint_for_negation(
    left: &SchemaNode,
    right: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    fn inner(
        left: &SchemaNode,
        right: &SchemaNode,
        context: &mut SubschemaCheckContext,
        active: &mut HashSet<(NodeId, NodeId)>,
    ) -> bool {
        if !active.insert((left.id(), right.id())) {
            return false;
        }

        use SchemaNodeKind::*;
        let result = match (left.kind(), right.kind()) {
            (BoolSchema(false), _) | (_, BoolSchema(false)) => true,
            (AnyOf(children), _) | (OneOf(children), _) => children
                .iter()
                .all(|child| inner(child, right, context, active)),
            (AllOf(children), _) => children
                .iter()
                .any(|child| inner(child, right, context, active)),
            (_, AnyOf(children)) | (_, OneOf(children)) => children
                .iter()
                .all(|child| inner(left, child, context, active)),
            (_, AllOf(children)) => children
                .iter()
                .any(|child| inner(left, child, context, active)),
            _ => {
                let left_mask = possible_json_type_mask(left);
                left_mask == 0
                    || schemas_definitely_disjoint_for_partition(left, left_mask, right, context)
            }
        };

        active.remove(&(left.id(), right.id()));
        result
    }

    inner(left, right, context, &mut HashSet::new())
}
/// Like `schemas_definitely_disjoint_for_oneof`, but also uses finite value
/// upper bounds with the concrete-value evaluator. This is only used while a
/// context is already available (for oneOf partition proofs): if every value
/// in a finite upper bound for either side is definitely rejected by the other
/// side, the languages cannot overlap. The upper-bound direction is important
/// here; unsupported schemas simply return `None` and keep the check
/// conservative.
fn schema_contains_explicit_not(schema: &SchemaNode) -> bool {
    fn inner(schema: &SchemaNode, active: &mut HashSet<NodeId>) -> bool {
        if !active.insert(schema.id()) {
            return false;
        }
        let result = match schema.kind() {
            SchemaNodeKind::Not(_) => true,
            SchemaNodeKind::AllOf(children)
            | SchemaNodeKind::AnyOf(children)
            | SchemaNodeKind::OneOf(children) => children.iter().any(|child| inner(child, active)),
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => {
                inner(if_schema, active)
                    || then_schema
                        .as_ref()
                        .is_some_and(|child| inner(child, active))
                    || else_schema
                        .as_ref()
                        .is_some_and(|child| inner(child, active))
            }
            _ => false,
        };
        active.remove(&schema.id());
        result
    }
    inner(schema, &mut HashSet::new())
}

fn schemas_definitely_disjoint_for_partition(
    sub: &SchemaNode,
    sub_mask: u8,
    other: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    fn base(
        sub: &SchemaNode,
        sub_mask: u8,
        other: &SchemaNode,
        context: &mut SubschemaCheckContext,
    ) -> bool {
        if schemas_definitely_disjoint_for_oneof(sub, sub_mask, other) {
            return true;
        }
        if let Some(values) = finite_schema_value_superset(sub)
            && values
                .iter()
                .all(|value| context.schema_definitely_rejects_value(other, value))
        {
            return true;
        }
        if let Some(values) = finite_schema_value_superset(other)
            && values
                .iter()
                .all(|value| context.schema_definitely_rejects_value(sub, value))
        {
            return true;
        }
        if finite_required_property_values_rejected_by_other(sub, other, context)
            || finite_required_property_values_rejected_by_other(other, sub, context)
        {
            return true;
        }
        if finite_required_array_item_values_rejected_by_other(sub, other, context)
            || finite_required_array_item_values_rejected_by_other(other, sub, context)
        {
            return true;
        }
        // Explicit complement guards are a common way generators spell the
        // "other" side of a partition (for example `allOf: [{not: A}, B]`).
        // Only invoke the recursive exclusion prover when a visible `not` is
        // present, both to keep this fast and to avoid re-entering the general
        // subset checker for ordinary branch pairs.
        if (schema_contains_explicit_not(sub)
            && schema_definitely_excludes_schema(sub, other, context))
            || (schema_contains_explicit_not(other)
                && schema_definitely_excludes_schema(other, sub, context))
        {
            return true;
        }
        false
    }

    fn inner(
        left: &SchemaNode,
        right: &SchemaNode,
        context: &mut SubschemaCheckContext,
        active: &mut HashSet<(NodeId, NodeId)>,
    ) -> bool {
        if !active.insert((left.id(), right.id())) {
            return false;
        }
        use SchemaNodeKind::*;
        // Try the whole pair first; split-allOf discriminator witnesses often
        // require constraints from multiple conjuncts.
        let result = if base(left, possible_json_type_mask(left), right, context) {
            true
        } else {
            match (left.kind(), right.kind()) {
                (BoolSchema(false), _) | (_, BoolSchema(false)) => true,
                (AnyOf(children), _) | (OneOf(children), _) => children
                    .iter()
                    .all(|child| inner(child, right, context, active)),
                (AllOf(children), _) => children
                    .iter()
                    .any(|child| inner(child, right, context, active)),
                (_, AnyOf(children)) | (_, OneOf(children)) => children
                    .iter()
                    .all(|child| inner(left, child, context, active)),
                (_, AllOf(children)) => children
                    .iter()
                    .any(|child| inner(left, child, context, active)),
                _ => false,
            }
        };
        active.remove(&(left.id(), right.id()));
        result
    }

    // Preserve the caller-provided mask for the common non-applicator fast path;
    // recursive applicator decomposition recomputes child masks as needed.
    if !matches!(
        sub.kind(),
        SchemaNodeKind::AnyOf(_) | SchemaNodeKind::OneOf(_) | SchemaNodeKind::AllOf(_)
    ) && !matches!(
        other.kind(),
        SchemaNodeKind::AnyOf(_) | SchemaNodeKind::OneOf(_) | SchemaNodeKind::AllOf(_)
    ) {
        return base(sub, sub_mask, other, context);
    }
    inner(sub, other, context, &mut HashSet::new())
}
/// Return true when `finite_side` forces a property into a finite set of
/// values, while `other` both guarantees that property exists and has a
/// syntactic property constraint that rejects every value in the set. This is
/// a context-aware sibling of the cheap discriminator checks: it lets the
/// concrete evaluator handle patterns/ranges on the property schema without
/// attempting general object implication.
fn finite_required_property_values_rejected_by_other(
    finite_side: &SchemaNode,
    other: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    for (name, values) in finite_required_property_value_bounds(finite_side) {
        if values.is_empty() {
            return true;
        }
        let object_only_overlap = (possible_json_type_mask(finite_side)
            & possible_json_type_mask(other)
            & !JSON_TYPE_OBJECT)
            == 0;
        if !(schema_guarantees_property_name(other, &name)
            || (object_only_overlap && schema_guarantees_property_name_for_objects(other, &name)))
        {
            continue;
        }
        if property_value_set_definitely_rejected(other, &name, &values, context) {
            return true;
        }
    }
    false
}

/// Does `schema` reject every object that contains `name` with a value from
/// `values`? The caller is responsible for proving the property is present.
/// We only use local property applicators (plus safe applicator composition),
/// so a false result simply means "unknown".
fn property_value_set_definitely_rejected(
    schema: &SchemaNode,
    name: &str,
    values: &[Value],
    context: &mut SubschemaCheckContext,
) -> bool {
    fn all_values_rejected(
        constraint: &SchemaNode,
        values: &[Value],
        context: &mut SubschemaCheckContext,
    ) -> bool {
        values
            .iter()
            .all(|value| context.schema_definitely_rejects_value(constraint, value))
    }

    fn inner(
        schema: &SchemaNode,
        name: &str,
        values: &[Value],
        context: &mut SubschemaCheckContext,
        active: &mut HashSet<NodeId>,
    ) -> bool {
        if !active.insert(schema.id()) {
            return false;
        }
        let result = match schema.kind() {
            SchemaNodeKind::BoolSchema(false) => true,
            SchemaNodeKind::BoolSchema(true) => false,
            SchemaNodeKind::Object {
                properties,
                pattern_properties,
                additional,
                property_names,
                ..
            } => {
                if string_literal_definitely_rejected(property_names, name) {
                    true
                } else {
                    let mut matched = false;
                    let direct_rejects = properties.get(name).is_some_and(|property_schema| {
                        matched = true;
                        all_values_rejected(property_schema, values, context)
                    });
                    if direct_rejects {
                        true
                    } else {
                        let mut pattern_rejects = false;
                        let mut unsupported_pattern = false;
                        for pattern_property in pattern_properties.values() {
                            if pattern_property.pattern.support() != PatternSupport::Supported {
                                // It may match this name, in which case `additionalProperties`
                                // would not apply. Keep that fallback conservative.
                                unsupported_pattern = true;
                                continue;
                            }
                            if pattern_property.pattern.is_match(name) {
                                matched = true;
                                if all_values_rejected(&pattern_property.schema, values, context) {
                                    pattern_rejects = true;
                                    break;
                                }
                            }
                        }
                        pattern_rejects
                            || (!matched
                                && !unsupported_pattern
                                && all_values_rejected(additional, values, context))
                    }
                }
            }
            SchemaNodeKind::AllOf(children) => children
                .iter()
                .any(|child| inner(child, name, values, context, active)),
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children) => {
                !children.is_empty()
                    && children
                        .iter()
                        .all(|child| inner(child, name, values, context, active))
            }
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), Some(else_schema)) => {
                    inner(then_schema, name, values, context, active)
                        && inner(else_schema, name, values, context, active)
                }
                (Some(then_schema), None)
                    if whole_json_types_accepted_mask(if_schema) & JSON_TYPE_OBJECT != 0 =>
                {
                    inner(then_schema, name, values, context, active)
                }
                (None, Some(else_schema))
                    if possible_json_type_mask(if_schema) & JSON_TYPE_OBJECT == 0 =>
                {
                    inner(else_schema, name, values, context, active)
                }
                _ => false,
            },
            _ => false,
        };
        active.remove(&schema.id());
        result
    }

    inner(schema, name, values, context, &mut HashSet::new())
}

/// Return true when `finite_side` forces some present tuple item into a finite
/// value set, while `other` guarantees the same position and locally rejects
/// every value in that set. This mirrors the property discriminator helper for
/// tuple-tagged unions.
fn finite_required_array_item_values_rejected_by_other(
    finite_side: &SchemaNode,
    other: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    const MAX_TRACKED_INDEX: u64 = 32;
    let Some(finite_len) = array_length_interval_bound(finite_side) else {
        return false;
    };
    let Some(other_len) = array_length_interval_bound(other) else {
        return false;
    };
    let shared = finite_len.lower.min(other_len.lower).min(MAX_TRACKED_INDEX);
    for index in 0..shared {
        let Ok(index_usize) = usize::try_from(index) else {
            break;
        };
        let Some(values) = finite_item_value_bound_at(finite_side, index_usize) else {
            continue;
        };
        if values.is_empty()
            || array_item_value_set_definitely_rejected(other, index_usize, &values, context)
        {
            return true;
        }
    }
    false
}

fn finite_item_value_bound_at(schema: &SchemaNode, index: usize) -> Option<Vec<Value>> {
    fn dedup_extend(out: &mut Vec<Value>, values: Vec<Value>) {
        for value in values {
            if !out.iter().any(|seen| json_values_equal(seen, &value)) {
                out.push(value);
            }
        }
    }

    fn inner(
        schema: &SchemaNode,
        index: usize,
        active: &mut HashSet<NodeId>,
    ) -> Option<Vec<Value>> {
        if !active.insert(schema.id()) {
            return None;
        }
        let result = match schema.kind() {
            SchemaNodeKind::BoolSchema(false) => Some(Vec::new()),
            SchemaNodeKind::Array {
                prefix_items,
                items,
                ..
            } => {
                let item_schema = prefix_items.get(index).unwrap_or(items);
                finite_schema_value_superset(item_schema)
            }
            SchemaNodeKind::AllOf(children) => children
                .iter()
                .find_map(|child| inner(child, index, active)),
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children) => {
                if children.is_empty() {
                    None
                } else {
                    let mut out = Vec::new();
                    let mut complete = true;
                    for child in children {
                        match inner(child, index, active) {
                            Some(values) => dedup_extend(&mut out, values),
                            None => {
                                complete = false;
                                break;
                            }
                        }
                    }
                    complete.then_some(out)
                }
            }
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), Some(else_schema)) => {
                    // Regardless of which side of the guard an array takes,
                    // its item must be covered by one of the two branch
                    // bounds. This is a union upper bound for discriminator values.
                    match (
                        inner(then_schema, index, active),
                        inner(else_schema, index, active),
                    ) {
                        (Some(mut then_values), Some(else_values)) => {
                            dedup_extend(&mut then_values, else_values);
                            Some(then_values)
                        }
                        _ => None,
                    }
                }
                (Some(then_schema), None)
                    if whole_json_types_accepted_mask(if_schema) & JSON_TYPE_ARRAY != 0 =>
                {
                    inner(then_schema, index, active)
                }
                (None, Some(else_schema))
                    if possible_json_type_mask(if_schema) & JSON_TYPE_ARRAY == 0 =>
                {
                    inner(else_schema, index, active)
                }
                _ => None,
            },
            SchemaNodeKind::Const(value) => value
                .as_array()
                .and_then(|array| array.get(index).cloned())
                .map(|value| vec![value]),
            SchemaNodeKind::Enum(values) => {
                let mut out = Vec::new();
                let mut complete = !values.is_empty();
                for value in values {
                    let Some(array) = value.as_array() else {
                        complete = false;
                        break;
                    };
                    let Some(item) = array.get(index) else {
                        complete = false;
                        break;
                    };
                    if !out.iter().any(|seen| json_values_equal(seen, item)) {
                        out.push(item.clone());
                    }
                }
                complete.then_some(out)
            }
            _ => None,
        };
        active.remove(&schema.id());
        result
    }
    inner(schema, index, &mut HashSet::new())
}

fn array_item_value_set_definitely_rejected(
    schema: &SchemaNode,
    index: usize,
    values: &[Value],
    context: &mut SubschemaCheckContext,
) -> bool {
    fn all_values_rejected(
        constraint: &SchemaNode,
        values: &[Value],
        context: &mut SubschemaCheckContext,
    ) -> bool {
        values
            .iter()
            .all(|value| context.schema_definitely_rejects_value(constraint, value))
    }

    fn inner(
        schema: &SchemaNode,
        index: usize,
        values: &[Value],
        context: &mut SubschemaCheckContext,
        active: &mut HashSet<NodeId>,
    ) -> bool {
        if !active.insert(schema.id()) {
            return false;
        }
        let result = match schema.kind() {
            SchemaNodeKind::BoolSchema(false) => true,
            SchemaNodeKind::BoolSchema(true) => false,
            SchemaNodeKind::Array {
                prefix_items,
                items,
                ..
            } => {
                let item_schema = prefix_items.get(index).unwrap_or(items);
                all_values_rejected(item_schema, values, context)
            }
            SchemaNodeKind::AllOf(children) => children
                .iter()
                .any(|child| inner(child, index, values, context, active)),
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children) => {
                !children.is_empty()
                    && children
                        .iter()
                        .all(|child| inner(child, index, values, context, active))
            }
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), Some(else_schema)) => {
                    inner(then_schema, index, values, context, active)
                        && inner(else_schema, index, values, context, active)
                }
                (Some(then_schema), None)
                    if whole_json_types_accepted_mask(if_schema) & JSON_TYPE_ARRAY != 0 =>
                {
                    inner(then_schema, index, values, context, active)
                }
                (None, Some(else_schema))
                    if possible_json_type_mask(if_schema) & JSON_TYPE_ARRAY == 0 =>
                {
                    inner(else_schema, index, values, context, active)
                }
                _ => false,
            },
            _ => false,
        };
        active.remove(&schema.id());
        result
    }
    inner(schema, index, values, context, &mut HashSet::new())
}

/// Return true when every value admitted by `schema` is known not to satisfy
/// `excluded`.  This is a small complement-aware helper for condition guards:
/// generated schemas often spell an else-only branch as `allOf: [{not: G}, ...]`.
/// We only use explicit `not` wrappers (plus applicator structure), and prove
/// the guard is contained by the negated schema with the regular subset
/// checker, so a `true` result is a sound disjointness fact.
fn schema_definitely_excludes_schema(
    schema: &SchemaNode,
    excluded: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    fn inner(
        schema: &SchemaNode,
        excluded: &SchemaNode,
        context: &mut SubschemaCheckContext,
        active: &mut HashSet<(NodeId, NodeId)>,
    ) -> bool {
        if !active.insert((schema.id(), excluded.id())) {
            return false;
        }

        use SchemaNodeKind::*;
        let result = match schema.kind() {
            BoolSchema(false) => true,
            AllOf(children) => children
                .iter()
                .any(|child| inner(child, excluded, context, active)),
            AnyOf(children) | OneOf(children) => children
                .iter()
                .all(|child| inner(child, excluded, context, active)),
            IfThenElse {
                then_schema,
                else_schema,
                ..
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), Some(else_schema)) => {
                    inner(then_schema, excluded, context, active)
                        && inner(else_schema, excluded, context, active)
                }
                // A missing conditional branch is the implicit `true` schema,
                // so it cannot by itself exclude anything.
                _ => false,
            },
            Not(negated) => {
                analyze_subschema_with_context(
                    excluded,
                    negated,
                    context,
                    ExplanationMode::VerdictOnly,
                )
                .is_subschema
            }
            _ => false,
        };

        active.remove(&(schema.id(), excluded.id()));
        result
    }

    inner(schema, excluded, context, &mut HashSet::new())
}

/// Return true when two schemas cannot share a value, using only cheap facts
/// needed by the `oneOf` partition proof. Type separation handles primitives;
/// for object-only overlap, also recognize the common discriminator pattern of
/// a required property constrained to disjoint finite value sets.
fn schemas_definitely_disjoint_for_oneof(
    sub: &SchemaNode,
    sub_mask: u8,
    other: &SchemaNode,
) -> bool {
    let other_mask = possible_json_type_mask(other);
    let overlap = sub_mask & other_mask;
    if overlap == 0 {
        return true;
    }
    if overlap == JSON_TYPE_NUMBER && numeric_intervals_are_disjoint(sub, other) {
        return true;
    }
    if overlap == JSON_TYPE_STRING && string_length_intervals_are_disjoint(sub, other) {
        return true;
    }
    if overlap == JSON_TYPE_ARRAY && array_length_intervals_are_disjoint(sub, other) {
        return true;
    }
    if overlap == JSON_TYPE_ARRAY && required_array_item_shapes_are_disjoint(sub, other) {
        return true;
    }
    if overlap == JSON_TYPE_OBJECT && object_property_count_intervals_are_disjoint(sub, other) {
        return true;
    }
    if overlap & !JSON_TYPE_OBJECT != 0 {
        return false;
    }
    required_property_values_are_disjoint(sub, other)
}

/// Cheap, sound disjointness facts that are useful outside the `oneOf`
/// partition proof as well (for example, tuple positions whose domains make
/// `uniqueItems` redundant). This intentionally exposes only the boolean
/// fact, not the type-mask machinery, so callers cannot accidentally rely on
/// the masks as exact languages.
pub(super) fn schemas_definitely_disjoint_by_shape(left: &SchemaNode, right: &SchemaNode) -> bool {
    let left_mask = possible_json_type_mask(left);
    left_mask == 0 || schemas_definitely_disjoint_for_oneof(left, left_mask, right)
}

#[derive(Clone, Copy, Debug)]
struct NumericIntervalBound {
    value: f64,
    inclusive: bool,
}

#[derive(Clone, Copy, Debug)]
struct NumericInterval {
    lower: Option<NumericIntervalBound>,
    upper: Option<NumericIntervalBound>,
    empty: bool,
}

impl NumericInterval {
    const fn unbounded() -> Self {
        Self {
            lower: None,
            upper: None,
            empty: false,
        }
    }

    const fn empty() -> Self {
        Self {
            lower: Some(NumericIntervalBound {
                value: 0.0,
                inclusive: false,
            }),
            upper: Some(NumericIntervalBound {
                value: 0.0,
                inclusive: false,
            }),
            empty: true,
        }
    }

    fn intersect(mut self, other: Self) -> Self {
        if self.empty || other.empty {
            self.empty = true;
            return self;
        }
        self.lower = tighter_lower(self.lower, other.lower);
        self.upper = tighter_upper(self.upper, other.upper);
        if interval_bounds_are_empty(self.lower, self.upper) {
            self.empty = true;
        }
        self
    }

    fn hull(self, other: Self) -> Option<Self> {
        if self.empty {
            return Some(other);
        }
        if other.empty {
            return Some(self);
        }
        Some(Self {
            lower: looser_lower(self.lower, other.lower)?,
            upper: looser_upper(self.upper, other.upper)?,
            empty: false,
        })
    }
}

fn tighter_lower(
    left: Option<NumericIntervalBound>,
    right: Option<NumericIntervalBound>,
) -> Option<NumericIntervalBound> {
    match (left, right) {
        (None, bound) | (bound, None) => bound,
        (Some(left), Some(right)) => match left.value.partial_cmp(&right.value) {
            Some(std::cmp::Ordering::Less) => Some(right),
            Some(std::cmp::Ordering::Greater) => Some(left),
            Some(std::cmp::Ordering::Equal) => Some(NumericIntervalBound {
                value: left.value,
                inclusive: left.inclusive && right.inclusive,
            }),
            None => None,
        },
    }
}

fn tighter_upper(
    left: Option<NumericIntervalBound>,
    right: Option<NumericIntervalBound>,
) -> Option<NumericIntervalBound> {
    match (left, right) {
        (None, bound) | (bound, None) => bound,
        (Some(left), Some(right)) => match left.value.partial_cmp(&right.value) {
            Some(std::cmp::Ordering::Less) => Some(left),
            Some(std::cmp::Ordering::Greater) => Some(right),
            Some(std::cmp::Ordering::Equal) => Some(NumericIntervalBound {
                value: left.value,
                inclusive: left.inclusive && right.inclusive,
            }),
            None => None,
        },
    }
}

fn looser_lower(
    left: Option<NumericIntervalBound>,
    right: Option<NumericIntervalBound>,
) -> Option<Option<NumericIntervalBound>> {
    match (left, right) {
        (None, _) | (_, None) => Some(None),
        (Some(left), Some(right)) => match left.value.partial_cmp(&right.value) {
            Some(std::cmp::Ordering::Less) => Some(Some(left)),
            Some(std::cmp::Ordering::Greater) => Some(Some(right)),
            Some(std::cmp::Ordering::Equal) => Some(Some(NumericIntervalBound {
                value: left.value,
                inclusive: left.inclusive || right.inclusive,
            })),
            None => None,
        },
    }
}

fn looser_upper(
    left: Option<NumericIntervalBound>,
    right: Option<NumericIntervalBound>,
) -> Option<Option<NumericIntervalBound>> {
    match (left, right) {
        (None, _) | (_, None) => Some(None),
        (Some(left), Some(right)) => match left.value.partial_cmp(&right.value) {
            Some(std::cmp::Ordering::Less) => Some(Some(right)),
            Some(std::cmp::Ordering::Greater) => Some(Some(left)),
            Some(std::cmp::Ordering::Equal) => Some(Some(NumericIntervalBound {
                value: left.value,
                inclusive: left.inclusive || right.inclusive,
            })),
            None => None,
        },
    }
}

fn interval_bounds_are_empty(
    lower: Option<NumericIntervalBound>,
    upper: Option<NumericIntervalBound>,
) -> bool {
    let (Some(lower), Some(upper)) = (lower, upper) else {
        return false;
    };
    match lower.value.partial_cmp(&upper.value) {
        Some(std::cmp::Ordering::Greater) => true,
        Some(std::cmp::Ordering::Equal) => !(lower.inclusive && upper.inclusive),
        _ => false,
    }
}

fn numeric_intervals_are_disjoint(left: &SchemaNode, right: &SchemaNode) -> bool {
    let Some(left_interval) = numeric_interval_bound(left) else {
        return false;
    };
    let Some(right_interval) = numeric_interval_bound(right) else {
        return false;
    };
    if left_interval.empty || right_interval.empty {
        return true;
    }
    interval_strictly_before(left_interval, right_interval)
        || interval_strictly_before(right_interval, left_interval)
}

/// Cheap string-length disjointness for partition proofs. Length bounds are
/// monotone under `allOf`, so intersecting syntactic bounds gives an
/// over-approximation of each schema's string language. If those intervals do
/// not overlap, the schemas cannot share a string value. Callers only use this
/// when string is the only overlapping JSON type, since min/maxLength do not
/// reject non-strings by themselves.
fn string_length_intervals_are_disjoint(left: &SchemaNode, right: &SchemaNode) -> bool {
    let Some(left_interval) = string_length_interval_bound(left) else {
        return false;
    };
    let Some(right_interval) = string_length_interval_bound(right) else {
        return false;
    };
    left_interval.empty
        || right_interval.empty
        || length_interval_strictly_before(left_interval, right_interval)
        || length_interval_strictly_before(right_interval, left_interval)
}

#[derive(Clone, Copy, Debug)]
struct LengthInterval {
    lower: u64,
    upper: Option<u64>,
    empty: bool,
}

impl LengthInterval {
    const fn unbounded() -> Self {
        Self {
            lower: 0,
            upper: None,
            empty: false,
        }
    }

    const fn empty() -> Self {
        Self {
            lower: 1,
            upper: Some(0),
            empty: true,
        }
    }

    fn intersect(mut self, other: Self) -> Self {
        if self.empty || other.empty {
            self.empty = true;
            return self;
        }
        self.lower = self.lower.max(other.lower);
        self.upper = match (self.upper, other.upper) {
            (None, bound) | (bound, None) => bound,
            (Some(left), Some(right)) => Some(left.min(right)),
        };
        if self.upper.is_some_and(|upper| self.lower > upper) {
            self.empty = true;
        }
        self
    }

    /// Return an interval containing both inputs. This is used for union
    /// applicators: a hull is deliberately imprecise, but remains a sound
    /// over-approximation for disjointness proofs.
    fn hull(self, other: Self) -> Self {
        if self.empty {
            return other;
        }
        if other.empty {
            return self;
        }
        Self {
            lower: self.lower.min(other.lower),
            upper: match (self.upper, other.upper) {
                (Some(left), Some(right)) => Some(left.max(right)),
                _ => None,
            },
            empty: false,
        }
    }
}

fn string_length_interval_bound(schema: &SchemaNode) -> Option<LengthInterval> {
    fn inner(schema: &SchemaNode, active: &mut HashSet<NodeId>) -> Option<LengthInterval> {
        if !active.insert(schema.id()) {
            return None;
        }
        let result = match schema.kind() {
            SchemaNodeKind::BoolSchema(false) => Some(LengthInterval {
                lower: 1,
                upper: Some(0),
                empty: true,
            }),
            SchemaNodeKind::String { length, .. } => Some(LengthInterval {
                lower: length.min(),
                upper: length.max(),
                empty: length.max().is_some_and(|upper| length.min() > upper),
            }),
            SchemaNodeKind::Const(value) => {
                if let Some(string) = value.as_str() {
                    let len = u64::try_from(string.chars().count()).ok()?;
                    Some(LengthInterval {
                        lower: len,
                        upper: Some(len),
                        empty: false,
                    })
                } else {
                    Some(LengthInterval::empty())
                }
            }
            SchemaNodeKind::Enum(values) => {
                let mut lower: Option<u64> = None;
                let mut upper: Option<u64> = None;
                for value in values {
                    if let Some(string) = value.as_str() {
                        let len = u64::try_from(string.chars().count()).ok()?;
                        lower = Some(lower.map_or(len, |current| current.min(len)));
                        upper = Some(upper.map_or(len, |current| current.max(len)));
                    }
                }
                match (lower, upper) {
                    (Some(lower), Some(upper)) => Some(LengthInterval {
                        lower,
                        upper: Some(upper),
                        empty: false,
                    }),
                    _ => Some(LengthInterval::empty()),
                }
            }
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), Some(else_schema)) => {
                    match (inner(then_schema, active), inner(else_schema, active)) {
                        (Some(then_interval), Some(else_interval)) => {
                            Some(then_interval.hull(else_interval))
                        }
                        _ => None,
                    }
                }
                (Some(then_schema), None)
                    if whole_json_types_accepted_mask(if_schema) & JSON_TYPE_STRING != 0 =>
                {
                    inner(then_schema, active)
                }
                (None, Some(else_schema))
                    if possible_json_type_mask(if_schema) & JSON_TYPE_STRING == 0 =>
                {
                    inner(else_schema, active)
                }
                _ => None,
            },
            SchemaNodeKind::AllOf(children) => {
                let mut interval = LengthInterval::unbounded();
                let mut saw_bound = false;
                for child in children {
                    if let Some(child_interval) = inner(child, active) {
                        interval = interval.intersect(child_interval);
                        saw_bound = true;
                    }
                }
                saw_bound.then_some(interval)
            }
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children) => {
                let mut hull: Option<LengthInterval> = None;
                let mut unknown_string_branch = false;
                for child in children {
                    if possible_json_type_mask(child) & JSON_TYPE_STRING == 0 {
                        continue;
                    }
                    match inner(child, active) {
                        Some(child_interval) => {
                            hull =
                                Some(hull.map_or(child_interval, |acc| acc.hull(child_interval)));
                        }
                        None => {
                            unknown_string_branch = true;
                            break;
                        }
                    }
                }
                if unknown_string_branch {
                    None
                } else {
                    Some(hull.unwrap_or_else(LengthInterval::empty))
                }
            }
            _ => None,
        };
        active.remove(&schema.id());
        result
    }

    inner(schema, &mut HashSet::new())
}

fn length_interval_strictly_before(left: LengthInterval, right: LengthInterval) -> bool {
    if left.empty || right.empty {
        return true;
    }
    left.upper.is_some_and(|upper| upper < right.lower)
}

/// Same interval proof as string lengths, for minItems/maxItems.  This is
/// only consulted when arrays are the sole overlapping JSON type.
fn array_length_intervals_are_disjoint(left: &SchemaNode, right: &SchemaNode) -> bool {
    let Some(left_interval) = array_length_interval_bound(left) else {
        return false;
    };
    let Some(right_interval) = array_length_interval_bound(right) else {
        return false;
    };
    left_interval.empty
        || right_interval.empty
        || length_interval_strictly_before(left_interval, right_interval)
        || length_interval_strictly_before(right_interval, left_interval)
}

fn array_length_interval_bound(schema: &SchemaNode) -> Option<LengthInterval> {
    fn inner(schema: &SchemaNode, active: &mut HashSet<NodeId>) -> Option<LengthInterval> {
        if !active.insert(schema.id()) {
            return None;
        }
        let result = match schema.kind() {
            SchemaNodeKind::BoolSchema(false) => Some(LengthInterval {
                lower: 1,
                upper: Some(0),
                empty: true,
            }),
            SchemaNodeKind::Array {
                prefix_items,
                items,
                item_count,
                contains,
                unique_items,
                ..
            } => {
                let effective_count = array::effective_item_count_for_unique_finite_domain(
                    prefix_items,
                    items,
                    *item_count,
                    *unique_items,
                )
                .unwrap_or(*item_count);
                let mut lower = effective_count.min();
                let mut upper = effective_count.max();
                let mut empty = upper.is_some_and(|upper| lower > upper);

                if let Some(contains) = contains {
                    let count = contains.count();
                    // At least N matching items implies at least N items total.
                    lower = lower.max(count.min());
                    // If every item necessarily matches (the common
                    // `contains: true` spelling), match-count bounds are also
                    // length bounds. Conversely, an impossible contains schema
                    // with a positive minimum makes the array language empty.
                    if schema_is_trivially_universal(&contains.schema) {
                        upper = match (upper, count.max()) {
                            (Some(a), Some(b)) => Some(a.min(b)),
                            (None, bound) | (bound, None) => bound,
                        };
                    } else if count.min() > 0
                        && schema_is_locally_empty_for_finite_enumeration(&contains.schema)
                    {
                        empty = true;
                    }
                    if upper.is_some_and(|upper| lower > upper) {
                        empty = true;
                    }
                }

                Some(LengthInterval {
                    lower,
                    upper,
                    empty,
                })
            }
            SchemaNodeKind::Const(value) => {
                if let Some(array) = value.as_array() {
                    let len = u64::try_from(array.len()).ok()?;
                    Some(LengthInterval {
                        lower: len,
                        upper: Some(len),
                        empty: false,
                    })
                } else {
                    Some(LengthInterval::empty())
                }
            }
            SchemaNodeKind::Enum(values) => {
                let mut lower: Option<u64> = None;
                let mut upper: Option<u64> = None;
                for value in values {
                    if let Some(array) = value.as_array() {
                        let len = u64::try_from(array.len()).ok()?;
                        lower = Some(lower.map_or(len, |current| current.min(len)));
                        upper = Some(upper.map_or(len, |current| current.max(len)));
                    }
                }
                match (lower, upper) {
                    (Some(lower), Some(upper)) => Some(LengthInterval {
                        lower,
                        upper: Some(upper),
                        empty: false,
                    }),
                    _ => Some(LengthInterval::empty()),
                }
            }
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), Some(else_schema)) => {
                    match (inner(then_schema, active), inner(else_schema, active)) {
                        (Some(then_interval), Some(else_interval)) => {
                            Some(then_interval.hull(else_interval))
                        }
                        _ => None,
                    }
                }
                (Some(then_schema), None)
                    if whole_json_types_accepted_mask(if_schema) & JSON_TYPE_ARRAY != 0 =>
                {
                    inner(then_schema, active)
                }
                (None, Some(else_schema))
                    if possible_json_type_mask(if_schema) & JSON_TYPE_ARRAY == 0 =>
                {
                    inner(else_schema, active)
                }
                _ => None,
            },
            SchemaNodeKind::AllOf(children) => {
                let mut interval = LengthInterval::unbounded();
                let mut saw_bound = false;
                for child in children {
                    if let Some(child_interval) = inner(child, active) {
                        interval = interval.intersect(child_interval);
                        saw_bound = true;
                    }
                }
                saw_bound.then_some(interval)
            }
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children) => {
                let mut hull: Option<LengthInterval> = None;
                let mut unknown_array_branch = false;
                for child in children {
                    if possible_json_type_mask(child) & JSON_TYPE_ARRAY == 0 {
                        continue;
                    }
                    match inner(child, active) {
                        Some(child_interval) => {
                            hull =
                                Some(hull.map_or(child_interval, |acc| acc.hull(child_interval)));
                        }
                        None => {
                            unknown_array_branch = true;
                            break;
                        }
                    }
                }
                if unknown_array_branch {
                    None
                } else {
                    Some(hull.unwrap_or_else(LengthInterval::empty))
                }
            }
            _ => None,
        };
        active.remove(&schema.id());
        result
    }

    inner(schema, &mut HashSet::new())
}

/// Same interval proof for minProperties/maxProperties, used only when objects
/// are the sole overlapping JSON type.
fn object_property_count_intervals_are_disjoint(left: &SchemaNode, right: &SchemaNode) -> bool {
    let Some(left_interval) = object_property_count_interval_bound(left) else {
        return false;
    };
    let Some(right_interval) = object_property_count_interval_bound(right) else {
        return false;
    };
    left_interval.empty
        || right_interval.empty
        || length_interval_strictly_before(left_interval, right_interval)
        || length_interval_strictly_before(right_interval, left_interval)
}

fn object_property_count_interval_bound(schema: &SchemaNode) -> Option<LengthInterval> {
    fn inner(schema: &SchemaNode, active: &mut HashSet<NodeId>) -> Option<LengthInterval> {
        if !active.insert(schema.id()) {
            return None;
        }
        let result = match schema.kind() {
            SchemaNodeKind::BoolSchema(false) => Some(LengthInterval {
                lower: 1,
                upper: Some(0),
                empty: true,
            }),
            SchemaNodeKind::Object {
                properties,
                pattern_properties,
                additional,
                property_names,
                property_count,
                ..
            } => {
                let syntactic_lower = u64::try_from(property_count.min()).ok()?;
                let implied_lower =
                    u64::try_from(guaranteed_property_name_closure(schema).len()).ok()?;
                let lower = syntactic_lower.max(implied_lower);
                let mut upper = property_count.max().map(u64::try_from).transpose().ok()?;

                // Closed objects with no pattern properties can only use the
                // explicitly declared names. This is a hard capacity even when
                // individual property schemas are broad or recursive.
                if pattern_properties.is_empty()
                    && matches!(additional.kind(), SchemaNodeKind::BoolSchema(false))
                {
                    let declared = u64::try_from(properties.len()).ok()?;
                    upper = Some(upper.map_or(declared, |current| current.min(declared)));
                }

                // A finite propertyNames language caps the number of distinct
                // keys regardless of additionalProperties/patternProperties.
                // `finite_schema_value_superset` is an upper bound, so counting
                // its string members remains a sound (possibly loose) capacity.
                if schema_has_obvious_finite_property_names(property_names)
                    && let Some(values) = finite_schema_value_superset(property_names)
                {
                    let mut names: Vec<&str> = Vec::new();
                    for value in &values {
                        if let Some(name) = value.as_str()
                            && !names.contains(&name)
                        {
                            names.push(name);
                        }
                    }
                    let finite = u64::try_from(names.len()).ok()?;
                    upper = Some(upper.map_or(finite, |current| current.min(finite)));
                }

                Some(LengthInterval {
                    lower,
                    upper,
                    empty: upper.is_some_and(|upper| lower > upper),
                })
            }
            SchemaNodeKind::Const(value) => {
                if let Some(object) = value.as_object() {
                    let len = u64::try_from(object.len()).ok()?;
                    Some(LengthInterval {
                        lower: len,
                        upper: Some(len),
                        empty: false,
                    })
                } else {
                    Some(LengthInterval::empty())
                }
            }
            SchemaNodeKind::Enum(values) => {
                let mut lower: Option<u64> = None;
                let mut upper: Option<u64> = None;
                for value in values {
                    if let Some(object) = value.as_object() {
                        let len = u64::try_from(object.len()).ok()?;
                        lower = Some(lower.map_or(len, |current| current.min(len)));
                        upper = Some(upper.map_or(len, |current| current.max(len)));
                    }
                }
                match (lower, upper) {
                    (Some(lower), Some(upper)) => Some(LengthInterval {
                        lower,
                        upper: Some(upper),
                        empty: false,
                    }),
                    _ => Some(LengthInterval::empty()),
                }
            }
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), Some(else_schema)) => {
                    match (inner(then_schema, active), inner(else_schema, active)) {
                        (Some(then_interval), Some(else_interval)) => {
                            Some(then_interval.hull(else_interval))
                        }
                        _ => None,
                    }
                }
                (Some(then_schema), None)
                    if whole_json_types_accepted_mask(if_schema) & JSON_TYPE_OBJECT != 0 =>
                {
                    inner(then_schema, active)
                }
                (None, Some(else_schema))
                    if possible_json_type_mask(if_schema) & JSON_TYPE_OBJECT == 0 =>
                {
                    inner(else_schema, active)
                }
                _ => None,
            },
            SchemaNodeKind::AllOf(children) => {
                let mut interval = LengthInterval::unbounded();
                let mut saw_bound = false;
                for child in children {
                    if let Some(child_interval) = inner(child, active) {
                        interval = interval.intersect(child_interval);
                        saw_bound = true;
                    }
                }
                if saw_bound {
                    if let Ok(implied_lower) =
                        u64::try_from(guaranteed_property_name_closure(schema).len())
                    {
                        interval.lower = interval.lower.max(implied_lower);
                        if interval.upper.is_some_and(|upper| interval.lower > upper) {
                            interval.empty = true;
                        }
                    }
                    Some(interval)
                } else {
                    None
                }
            }
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children) => {
                let mut hull: Option<LengthInterval> = None;
                let mut unknown_object_branch = false;
                for child in children {
                    if possible_json_type_mask(child) & JSON_TYPE_OBJECT == 0 {
                        continue;
                    }
                    match inner(child, active) {
                        Some(child_interval) => {
                            hull =
                                Some(hull.map_or(child_interval, |acc| acc.hull(child_interval)));
                        }
                        None => {
                            unknown_object_branch = true;
                            break;
                        }
                    }
                }
                if unknown_object_branch {
                    None
                } else {
                    Some(hull.unwrap_or_else(LengthInterval::empty))
                }
            }
            _ => None,
        };
        active.remove(&schema.id());
        result
    }

    inner(schema, &mut HashSet::new())
}

/// Prove a split `allOf` numeric intersection is contained by a plain number
/// range. This is intentionally narrower than general numeric implication:
/// callers only use it for a right-hand `Number` without `multipleOf` or
/// enumeration. The type-mask check is essential because JSON Schema numeric
/// bounds alone do not reject non-number instances.
fn split_allof_numeric_range_subset_of_number(sub: &SchemaNode, sup: &SchemaNode) -> bool {
    let sub_mask = possible_json_type_mask(sub);
    if sub_mask == 0 {
        return true;
    }
    if sub_mask & !JSON_TYPE_NUMBER != 0 {
        return false;
    }
    let Some(sub_interval) = numeric_interval_bound(sub) else {
        return false;
    };
    if sub_interval.empty {
        return true;
    }
    let Some(sup_interval) = numeric_interval_bound(sup) else {
        return false;
    };
    numeric_interval_contains(sup_interval, sub_interval)
}

fn split_allof_integer_range_subset_of_integer(sub: &SchemaNode, sup: &SchemaNode) -> bool {
    let sub_mask = possible_json_type_mask(sub);
    if sub_mask == 0 {
        return true;
    }
    if sub_mask & !JSON_TYPE_NUMBER != 0 {
        return false;
    }
    if !allof_has_direct_integer_conjunct(sub) {
        return false;
    }
    let Some(sub_interval) = numeric_interval_bound(sub) else {
        return false;
    };
    if sub_interval.empty {
        return true;
    }
    let Some(sup_interval) = numeric_interval_bound(sup) else {
        return false;
    };
    numeric_interval_contains(sup_interval, sub_interval)
}

fn allof_has_direct_integer_conjunct(schema: &SchemaNode) -> bool {
    match schema.kind() {
        SchemaNodeKind::AllOf(children) => children
            .iter()
            .any(|child| matches!(child.kind(), SchemaNodeKind::Integer { .. })),
        _ => false,
    }
}

fn split_allof_string_length_subset_of_string(sub: &SchemaNode, sup: &SchemaNode) -> bool {
    split_allof_length_subset_of_type(sub, sup, JSON_TYPE_STRING, string_length_interval_bound)
}

fn split_allof_array_length_subset_of_array(sub: &SchemaNode, sup: &SchemaNode) -> bool {
    split_allof_length_subset_of_type(sub, sup, JSON_TYPE_ARRAY, array_length_interval_bound)
}

fn split_allof_object_count_subset_of_object(sub: &SchemaNode, sup: &SchemaNode) -> bool {
    split_allof_length_subset_of_type(
        sub,
        sup,
        JSON_TYPE_OBJECT,
        object_property_count_interval_bound,
    )
}

fn split_allof_length_subset_of_type(
    sub: &SchemaNode,
    sup: &SchemaNode,
    type_bit: u8,
    interval_bound: fn(&SchemaNode) -> Option<LengthInterval>,
) -> bool {
    let sub_mask = possible_json_type_mask(sub);
    if sub_mask == 0 {
        return true;
    }
    if sub_mask & !type_bit != 0 {
        return false;
    }
    let Some(sub_interval) = interval_bound(sub) else {
        return false;
    };
    if sub_interval.empty {
        return true;
    }
    let Some(sup_interval) = interval_bound(sup) else {
        return false;
    };
    length_interval_contains(sup_interval, sub_interval)
}

fn length_interval_contains(outer: LengthInterval, inner: LengthInterval) -> bool {
    if inner.empty {
        return true;
    }
    if outer.empty {
        return false;
    }
    if outer.lower > inner.lower {
        return false;
    }
    match (outer.upper, inner.upper) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(outer_upper), Some(inner_upper)) => inner_upper <= outer_upper,
    }
}

fn array_schema_is_plain_count(schema: &SchemaNode) -> bool {
    match schema.kind() {
        SchemaNodeKind::Array {
            prefix_items,
            items,
            contains,
            unique_items,
            enumeration,
            ..
        } => {
            prefix_items.is_empty()
                && schema_is_trivially_universal(items)
                && contains.is_none()
                && !*unique_items
                && enumeration.is_none()
        }
        _ => false,
    }
}

fn object_schema_is_plain_count(schema: &SchemaNode) -> bool {
    match schema.kind() {
        SchemaNodeKind::Object {
            properties,
            pattern_properties,
            required,
            additional,
            property_names,
            dependent_required,
            enumeration,
            ..
        } => {
            properties.is_empty()
                && pattern_properties.is_empty()
                && required.is_empty()
                && schema_is_trivially_universal(additional)
                && schema_is_trivially_universal(property_names)
                && dependent_required.is_empty()
                && enumeration.is_none()
        }
        _ => false,
    }
}

fn integer_schema_is_plain_range(schema: &SchemaNode) -> bool {
    match schema.kind() {
        SchemaNodeKind::Integer {
            multiple_of,
            enumeration,
            ..
        } => {
            enumeration.is_none()
                && multiple_of
                    .as_ref()
                    .is_none_or(|multiple| multiple.integer_divisor() == Some(1))
        }
        _ => false,
    }
}

fn numeric_interval_contains(outer: NumericInterval, inner: NumericInterval) -> bool {
    if inner.empty {
        return true;
    }
    if outer.empty {
        return false;
    }
    lower_bound_contains(outer.lower, inner.lower) && upper_bound_contains(outer.upper, inner.upper)
}

fn lower_bound_contains(
    outer: Option<NumericIntervalBound>,
    inner: Option<NumericIntervalBound>,
) -> bool {
    match (outer, inner) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(outer), Some(inner)) => match inner.value.partial_cmp(&outer.value) {
            Some(std::cmp::Ordering::Greater) => true,
            Some(std::cmp::Ordering::Equal) => outer.inclusive || !inner.inclusive,
            _ => false,
        },
    }
}

fn upper_bound_contains(
    outer: Option<NumericIntervalBound>,
    inner: Option<NumericIntervalBound>,
) -> bool {
    match (outer, inner) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(outer), Some(inner)) => match inner.value.partial_cmp(&outer.value) {
            Some(std::cmp::Ordering::Less) => true,
            Some(std::cmp::Ordering::Equal) => outer.inclusive || !inner.inclusive,
            _ => false,
        },
    }
}

fn interval_strictly_before(left: NumericInterval, right: NumericInterval) -> bool {
    let (Some(left_upper), Some(right_lower)) = (left.upper, right.lower) else {
        return false;
    };
    match left_upper.value.partial_cmp(&right_lower.value) {
        Some(std::cmp::Ordering::Less) => true,
        Some(std::cmp::Ordering::Equal) => !(left_upper.inclusive && right_lower.inclusive),
        _ => false,
    }
}

fn numeric_interval_bound(schema: &SchemaNode) -> Option<NumericInterval> {
    fn from_number_bound(bound: NumberBound) -> Option<NumericIntervalBound> {
        match bound {
            NumberBound::Unbounded => None,
            NumberBound::Inclusive(value) => Some(NumericIntervalBound {
                value,
                inclusive: true,
            }),
            NumberBound::Exclusive(value) => Some(NumericIntervalBound {
                value,
                inclusive: false,
            }),
        }
    }

    fn integer_endpoint(value: i64) -> Option<f64> {
        const MAX_EXACT: i64 = 9_007_199_254_740_991;
        value
            .checked_abs()
            .is_some_and(|abs| abs <= MAX_EXACT)
            .then_some(value as f64)
    }

    // Convert JSON numeric literals only when doing so cannot silently widen or
    // narrow a large integer. The interval proof is allowed to be imprecise,
    // but not wrong: a rounded 2^63 literal could otherwise look equal to a
    // neighboring value and create a bogus disjointness fact.
    fn json_number_endpoint(value: &Value) -> Option<f64> {
        let number = value.as_number()?;
        const MAX_EXACT_U64: u64 = 9_007_199_254_740_991;
        if let Some(integer) = number.as_i64() {
            return integer_endpoint(integer);
        }
        if let Some(integer) = number.as_u64() {
            return (integer <= MAX_EXACT_U64).then_some(integer as f64);
        }
        let value = number.as_f64()?;
        value.is_finite().then_some(value)
    }

    // Return a hull for the numeric members of a literal set. Non-numeric
    // members are ignored (the caller separately checks type-mask overlap),
    // while an unrepresentable numeric member makes the hull unknown.
    fn literal_numeric_hull(values: &[Value]) -> Option<NumericInterval> {
        let mut hull: Option<NumericInterval> = None;
        let mut saw_numeric = false;
        for value in values {
            if !value.is_number() {
                continue;
            }
            saw_numeric = true;
            let endpoint = json_number_endpoint(value)?;
            let singleton = NumericInterval {
                lower: Some(NumericIntervalBound {
                    value: endpoint,
                    inclusive: true,
                }),
                upper: Some(NumericIntervalBound {
                    value: endpoint,
                    inclusive: true,
                }),
                empty: false,
            };
            hull = Some(match hull {
                Some(current) => current.hull(singleton)?,
                None => singleton,
            });
        }
        Some(if saw_numeric {
            hull.unwrap_or_else(NumericInterval::empty)
        } else {
            NumericInterval::empty()
        })
    }

    fn inner(schema: &SchemaNode, active: &mut HashSet<NodeId>) -> Option<NumericInterval> {
        if !active.insert(schema.id()) {
            return None;
        }
        let result = match schema.kind() {
            SchemaNodeKind::BoolSchema(false) => Some(NumericInterval::empty()),
            SchemaNodeKind::Const(value) => {
                if value.is_number() {
                    json_number_endpoint(value).map(|endpoint| NumericInterval {
                        lower: Some(NumericIntervalBound {
                            value: endpoint,
                            inclusive: true,
                        }),
                        upper: Some(NumericIntervalBound {
                            value: endpoint,
                            inclusive: true,
                        }),
                        empty: false,
                    })
                } else {
                    Some(NumericInterval::empty())
                }
            }
            SchemaNodeKind::Enum(values) => literal_numeric_hull(values),
            SchemaNodeKind::Number {
                bounds,
                enumeration,
                ..
            } => {
                let mut interval = NumericInterval {
                    lower: from_number_bound(bounds.lower()),
                    upper: from_number_bound(bounds.upper()),
                    empty: false,
                };
                if let Some(values) = enumeration
                    && let Some(enum_interval) = literal_numeric_hull(values)
                {
                    interval = interval.intersect(enum_interval);
                }
                Some(interval)
            }
            SchemaNodeKind::Integer {
                bounds,
                enumeration,
                ..
            } => {
                let lower = match bounds.lower() {
                    Some(value) => match integer_endpoint(value) {
                        Some(value) => Some(NumericIntervalBound {
                            value,
                            inclusive: true,
                        }),
                        None => {
                            active.remove(&schema.id());
                            return None;
                        }
                    },
                    None => None,
                };
                let upper = match bounds.upper() {
                    Some(value) => match integer_endpoint(value) {
                        Some(value) => Some(NumericIntervalBound {
                            value,
                            inclusive: true,
                        }),
                        None => {
                            active.remove(&schema.id());
                            return None;
                        }
                    },
                    None => None,
                };
                let mut interval = NumericInterval {
                    lower,
                    upper,
                    empty: false,
                };
                if let Some(values) = enumeration
                    && let Some(enum_interval) = literal_numeric_hull(values)
                {
                    interval = interval.intersect(enum_interval);
                }
                Some(interval)
            }
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), Some(else_schema)) => {
                    match (inner(then_schema, active), inner(else_schema, active)) {
                        (Some(then_interval), Some(else_interval)) => {
                            then_interval.hull(else_interval)
                        }
                        _ => None,
                    }
                }
                (Some(then_schema), None)
                    if whole_json_types_accepted_mask(if_schema) & JSON_TYPE_NUMBER != 0 =>
                {
                    inner(then_schema, active)
                }
                (None, Some(else_schema))
                    if possible_json_type_mask(if_schema) & JSON_TYPE_NUMBER == 0 =>
                {
                    inner(else_schema, active)
                }
                _ => None,
            },
            SchemaNodeKind::AllOf(children) => {
                let mut interval = NumericInterval::unbounded();
                let mut saw_numeric_bound = false;
                for child in children {
                    if let Some(child_interval) = inner(child, active) {
                        interval = interval.intersect(child_interval);
                        saw_numeric_bound = true;
                    }
                }
                saw_numeric_bound.then_some(interval)
            }
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children) => {
                let mut hull: Option<NumericInterval> = None;
                let mut unknown_numeric_branch = false;
                for child in children {
                    if possible_json_type_mask(child) & JSON_TYPE_NUMBER == 0 {
                        continue;
                    }
                    match inner(child, active) {
                        Some(child_interval) => {
                            hull = match hull {
                                Some(acc) => match acc.hull(child_interval) {
                                    Some(joined) => Some(joined),
                                    None => {
                                        unknown_numeric_branch = true;
                                        break;
                                    }
                                },
                                None => Some(child_interval),
                            };
                        }
                        None => {
                            unknown_numeric_branch = true;
                            break;
                        }
                    }
                }
                if unknown_numeric_branch {
                    None
                } else {
                    Some(hull.unwrap_or_else(NumericInterval::empty))
                }
            }
            _ => None,
        };
        active.remove(&schema.id());
        result
    }

    inner(schema, &mut HashSet::new())
}

/// Collect property names forced by direct `required` declarations and by
/// `dependentRequired` rules whose triggers are themselves forced. For an
/// `allOf`, requirements and dependency rules from different conjuncts can
/// interact, so this computes a small closure across all conjuncts. It is an
/// under-approximation: missing a name is conservative.
fn guaranteed_property_name_closure(schema: &SchemaNode) -> HashSet<String> {
    fn collect(
        schema: &SchemaNode,
        names: &mut HashSet<String>,
        rules: &mut Vec<(String, Vec<String>)>,
        active: &mut HashSet<NodeId>,
    ) {
        if !active.insert(schema.id()) {
            return;
        }
        match schema.kind() {
            SchemaNodeKind::Object {
                required,
                dependent_required,
                ..
            } => {
                names.extend(required.iter().cloned());
                rules.extend(
                    dependent_required
                        .iter()
                        .map(|(trigger, deps)| (trigger.clone(), deps.clone())),
                );
            }
            SchemaNodeKind::Const(value) => {
                if let Some(object) = value.as_object() {
                    names.extend(object.keys().cloned());
                }
            }
            SchemaNodeKind::Enum(values) if !values.is_empty() => {
                let mut iter = values.iter();
                if let Some(first) = iter.next().and_then(Value::as_object) {
                    let mut common: HashSet<String> = first.keys().cloned().collect();
                    for value in iter {
                        if let Some(object) = value.as_object() {
                            common.retain(|name| object.contains_key(name));
                        } else {
                            common.clear();
                            break;
                        }
                    }
                    names.extend(common);
                }
            }
            SchemaNodeKind::AllOf(children) => {
                for child in children {
                    collect(child, names, rules, active);
                }
            }
            _ => {}
        }
        active.remove(&schema.id());
    }

    let mut names = HashSet::new();
    let mut rules = Vec::new();
    collect(schema, &mut names, &mut rules, &mut HashSet::new());
    let mut changed = true;
    while changed {
        changed = false;
        for (trigger, deps) in &rules {
            if names.contains(trigger) {
                for dep in deps {
                    if names.insert(dep.clone()) {
                        changed = true;
                    }
                }
            }
        }
    }
    names
}

/// Return true when every object admitted by `schema` is known to contain
/// `name`. This is deliberately syntactic and mirrors the discriminator
/// helpers: unions only guarantee names common to all branches, while allOf can
/// inherit a guarantee from any conjunct.
fn schema_guarantees_property_name(schema: &SchemaNode, name: &str) -> bool {
    if guaranteed_property_name_closure(schema).contains(name) {
        return true;
    }

    fn inner(schema: &SchemaNode, name: &str, active: &mut HashSet<NodeId>) -> bool {
        if !active.insert(schema.id()) {
            return false;
        }
        let result = match schema.kind() {
            SchemaNodeKind::BoolSchema(false) => true,
            SchemaNodeKind::Object { required, .. } => {
                required.contains(name) || guaranteed_property_name_closure(schema).contains(name)
            }
            SchemaNodeKind::Const(value) => value
                .as_object()
                .is_some_and(|object| object.contains_key(name)),
            SchemaNodeKind::Enum(values) => {
                !values.is_empty()
                    && values.iter().all(|value| {
                        value
                            .as_object()
                            .is_some_and(|object| object.contains_key(name))
                    })
            }
            SchemaNodeKind::AllOf(children) => {
                guaranteed_property_name_closure(schema).contains(name)
                    || children.iter().any(|child| inner(child, name, active))
            }
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children) => {
                !children.is_empty() && children.iter().all(|child| inner(child, name, active))
            }
            SchemaNodeKind::IfThenElse {
                then_schema,
                else_schema,
                ..
            } => {
                match (then_schema.as_ref(), else_schema.as_ref()) {
                    (Some(then_schema), Some(else_schema)) => {
                        inner(then_schema, name, active) && inner(else_schema, name, active)
                    }
                    // A missing branch is unconstrained, so it cannot force a name.
                    _ => false,
                }
            }
            _ => false,
        };
        active.remove(&schema.id());
        result
    }
    inner(schema, name, &mut HashSet::new())
}

/// Like `schema_guarantees_property_name`, but scoped to object instances of
/// `schema`.  This is only safe for callers that have already ruled out
/// non-object overlap with the other side; conditional schemas with a missing
/// branch often accept scalars vacuously, so the global guarantee would be too
/// strong there.
fn schema_guarantees_property_name_for_objects(schema: &SchemaNode, name: &str) -> bool {
    fn inner(schema: &SchemaNode, name: &str, active: &mut HashSet<NodeId>) -> bool {
        if !active.insert(schema.id()) {
            return false;
        }
        let result = match schema.kind() {
            SchemaNodeKind::BoolSchema(false) => true,
            SchemaNodeKind::BoolSchema(true) => false,
            SchemaNodeKind::Object { required, .. } => {
                required.contains(name) || guaranteed_property_name_closure(schema).contains(name)
            }
            SchemaNodeKind::Const(value) => value
                .as_object()
                .is_none_or(|object| object.contains_key(name)),
            SchemaNodeKind::Enum(values) => {
                let mut saw_object = false;
                let mut ok = true;
                for value in values {
                    if let Some(object) = value.as_object() {
                        saw_object = true;
                        if !object.contains_key(name) {
                            ok = false;
                            break;
                        }
                    }
                }
                // No object literals means the object slice is empty.
                !saw_object || ok
            }
            SchemaNodeKind::AllOf(children) => {
                children.iter().any(|child| inner(child, name, active))
            }
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children) => {
                let mut saw_object_branch = false;
                let mut ok = true;
                for child in children {
                    if possible_json_type_mask(child) & JSON_TYPE_OBJECT == 0 {
                        continue;
                    }
                    saw_object_branch = true;
                    if !inner(child, name, active) {
                        ok = false;
                        break;
                    }
                }
                !saw_object_branch || ok
            }
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), Some(else_schema)) => {
                    inner(then_schema, name, active) && inner(else_schema, name, active)
                }
                (Some(then_schema), None)
                    if whole_json_types_accepted_mask(if_schema) & JSON_TYPE_OBJECT != 0 =>
                {
                    inner(then_schema, name, active)
                }
                (None, Some(else_schema))
                    if possible_json_type_mask(if_schema) & JSON_TYPE_OBJECT == 0 =>
                {
                    inner(else_schema, name, active)
                }
                _ => false,
            },
            _ => false,
        };
        active.remove(&schema.id());
        result
    }

    schema_guarantees_property_name(schema, name) || inner(schema, name, &mut HashSet::new())
}

/// Syntactic rejection check for a concrete property name against a
/// `propertyNames` schema. This intentionally recognizes only simple literal,
/// enum, length, and supported-pattern cases; returning false is conservative.
fn string_literal_definitely_rejected(schema: &SchemaNode, name: &str) -> bool {
    fn inner(schema: &SchemaNode, name: &str, active: &mut HashSet<NodeId>) -> bool {
        if !active.insert(schema.id()) {
            return false;
        }
        let name_value = Value::String(name.to_owned());
        let result = match schema.kind() {
            SchemaNodeKind::BoolSchema(false) => true,
            SchemaNodeKind::BoolSchema(true) | SchemaNodeKind::Any => false,
            SchemaNodeKind::Const(value) => !json_values_equal(value, &name_value),
            SchemaNodeKind::Enum(values) => !values
                .iter()
                .any(|value| json_values_equal(value, &name_value)),
            SchemaNodeKind::String {
                length,
                pattern,
                enumeration,
                ..
            } => {
                if enumeration.as_ref().is_some_and(|values| {
                    !values
                        .iter()
                        .any(|value| json_values_equal(value, &name_value))
                }) {
                    true
                } else {
                    let len = name.chars().count() as u64;
                    len < length.min()
                        || length.max().is_some_and(|max| len > max)
                        || pattern.as_ref().is_some_and(|pattern| {
                            pattern.support() == PatternSupport::Supported
                                && !pattern.is_match(name)
                        })
                }
            }
            SchemaNodeKind::AllOf(children) => {
                children.iter().any(|child| inner(child, name, active))
            }
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children) => {
                !children.is_empty() && children.iter().all(|child| inner(child, name, active))
            }
            _ => false,
        };
        active.remove(&schema.id());
        result
    }
    inner(schema, name, &mut HashSet::new())
}

/// Return true when `schema` rejects every object containing property `name`.
/// The common generated spelling is `properties: { name: false }`; applicator
/// propagation is included, but we avoid general negation/propertyNames
/// reasoning here to keep the fact obviously sound.
/// Return true when `schema` is known to accept every object containing `name`.
/// This lets `not schema` act as a syntactic "property is absent" guard.  The
/// recognizer is intentionally narrow: object constraints must be universal
/// for arbitrary extra names/values, and applicators are handled only in the
/// directions that preserve a universal-with-property fact.
fn schema_accepts_all_objects_with_property(schema: &SchemaNode, name: &str) -> bool {
    fn inner(schema: &SchemaNode, name: &str, active: &mut HashSet<NodeId>) -> bool {
        if !active.insert(schema.id()) {
            return false;
        }
        let result = match schema.kind() {
            SchemaNodeKind::Any | SchemaNodeKind::BoolSchema(true) => true,
            SchemaNodeKind::Object {
                properties,
                pattern_properties,
                required,
                additional,
                property_names,
                property_count,
                dependent_required,
                enumeration,
            } => {
                required.iter().all(|required_name| required_name == name)
                    && properties.keys().all(|property_name| property_name == name)
                    && properties
                        .get(name)
                        .is_none_or(schema_is_trivially_universal)
                    && pattern_properties.is_empty()
                    && property_count.min() <= 1
                    && property_count.max().is_none()
                    && dependent_required.is_empty()
                    && enumeration.is_none()
                    && schema_is_trivially_universal(additional)
                    && schema_is_trivially_universal(property_names)
            }
            // A union accepts all objects-with-name if any branch does.
            SchemaNodeKind::AnyOf(children) => {
                children.iter().any(|child| inner(child, name, active))
            }
            // An intersection does so only if every conjunct does.
            SchemaNodeKind::AllOf(children) => {
                !children.is_empty() && children.iter().all(|child| inner(child, name, active))
            }
            _ => false,
        };
        active.remove(&schema.id());
        result
    }
    inner(schema, name, &mut HashSet::new())
}

fn schema_forbids_property_name(schema: &SchemaNode, name: &str) -> bool {
    fn inner(schema: &SchemaNode, name: &str, active: &mut HashSet<NodeId>) -> bool {
        if !active.insert(schema.id()) {
            return false;
        }
        let result = match schema.kind() {
            SchemaNodeKind::BoolSchema(false) => true,
            SchemaNodeKind::Const(value) => !value
                .as_object()
                .is_some_and(|object| object.contains_key(name)),
            SchemaNodeKind::Enum(values) => values.iter().all(|value| {
                !value
                    .as_object()
                    .is_some_and(|object| object.contains_key(name))
            }),
            SchemaNodeKind::Object {
                properties,
                pattern_properties,
                additional,
                property_names,
                property_count,
                ..
            } => {
                property_count.max() == Some(0)
                    || property_count.max().is_some_and(|max| {
                        let guaranteed = guaranteed_property_name_closure(schema);
                        !guaranteed.contains(name) && guaranteed.len() >= max
                    })
                    || properties
                        .get(name)
                        .is_some_and(schema_is_locally_empty_for_finite_enumeration)
                    || pattern_properties.values().any(|pattern_property| {
                        pattern_property.pattern.support() == PatternSupport::Supported
                            && pattern_property.pattern.is_match(name)
                            && schema_is_locally_empty_for_finite_enumeration(
                                &pattern_property.schema,
                            )
                    })
                    || string_literal_definitely_rejected(property_names, name)
                    || (!properties.contains_key(name)
                        && pattern_properties.values().all(|pattern_property| {
                            pattern_property.pattern.support() == PatternSupport::Supported
                                && !pattern_property.pattern.is_match(name)
                        })
                        && schema_is_locally_empty_for_finite_enumeration(additional))
            }
            SchemaNodeKind::AllOf(children) => {
                children.iter().any(|child| inner(child, name, active))
            }
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children) => {
                !children.is_empty() && children.iter().all(|child| inner(child, name, active))
            }
            SchemaNodeKind::IfThenElse {
                then_schema,
                else_schema,
                ..
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), Some(else_schema)) => {
                    inner(then_schema, name, active) && inner(else_schema, name, active)
                }
                _ => false,
            },
            SchemaNodeKind::Not(child) => schema_accepts_all_objects_with_property(child, name),
            _ => false,
        };
        active.remove(&schema.id());
        result
    }
    inner(schema, name, &mut HashSet::new())
}

/// Object-scoped variant of `schema_forbids_property_name`.  It may return
/// true for schemas that also accept scalars, so callers must first establish
/// that only object values can overlap with the comparison side.
fn schema_forbids_property_name_for_objects(schema: &SchemaNode, name: &str) -> bool {
    fn inner(schema: &SchemaNode, name: &str, active: &mut HashSet<NodeId>) -> bool {
        if !active.insert(schema.id()) {
            return false;
        }
        let result = match schema.kind() {
            SchemaNodeKind::BoolSchema(false) => true,
            SchemaNodeKind::BoolSchema(true) => false,
            SchemaNodeKind::Const(value) => value
                .as_object()
                .is_none_or(|object| !object.contains_key(name)),
            SchemaNodeKind::Enum(values) => {
                let mut saw_object = false;
                let mut ok = true;
                for value in values {
                    if let Some(object) = value.as_object() {
                        saw_object = true;
                        if object.contains_key(name) {
                            ok = false;
                            break;
                        }
                    }
                }
                !saw_object || ok
            }
            SchemaNodeKind::Object {
                properties,
                pattern_properties,
                additional,
                property_names,
                property_count,
                ..
            } => {
                property_count.max() == Some(0)
                    || property_count.max().is_some_and(|max| {
                        let guaranteed = guaranteed_property_name_closure(schema);
                        !guaranteed.contains(name) && guaranteed.len() >= max
                    })
                    || properties
                        .get(name)
                        .is_some_and(schema_is_locally_empty_for_finite_enumeration)
                    || pattern_properties.values().any(|pattern_property| {
                        pattern_property.pattern.support() == PatternSupport::Supported
                            && pattern_property.pattern.is_match(name)
                            && schema_is_locally_empty_for_finite_enumeration(
                                &pattern_property.schema,
                            )
                    })
                    || string_literal_definitely_rejected(property_names, name)
                    || (!properties.contains_key(name)
                        && pattern_properties.values().all(|pattern_property| {
                            pattern_property.pattern.support() == PatternSupport::Supported
                                && !pattern_property.pattern.is_match(name)
                        })
                        && schema_is_locally_empty_for_finite_enumeration(additional))
            }
            SchemaNodeKind::AllOf(children) => {
                children.iter().any(|child| inner(child, name, active))
            }
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children) => {
                let mut saw_object_branch = false;
                let mut ok = true;
                for child in children {
                    if possible_json_type_mask(child) & JSON_TYPE_OBJECT == 0 {
                        continue;
                    }
                    saw_object_branch = true;
                    if !inner(child, name, active) {
                        ok = false;
                        break;
                    }
                }
                !saw_object_branch || ok
            }
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), Some(else_schema)) => {
                    inner(then_schema, name, active) && inner(else_schema, name, active)
                }
                (Some(then_schema), None)
                    if whole_json_types_accepted_mask(if_schema) & JSON_TYPE_OBJECT != 0 =>
                {
                    inner(then_schema, name, active)
                }
                (None, Some(else_schema))
                    if possible_json_type_mask(if_schema) & JSON_TYPE_OBJECT == 0 =>
                {
                    inner(else_schema, name, active)
                }
                _ => false,
            },
            _ => false,
        };
        active.remove(&schema.id());
        result
    }

    schema_forbids_property_name(schema, name) || inner(schema, name, &mut HashSet::new())
}

fn required_vs_forbidden_property_are_disjoint(left: &SchemaNode, right: &SchemaNode) -> bool {
    fn guaranteed_names(
        schema: &SchemaNode,
        out: &mut HashSet<String>,
        active: &mut HashSet<NodeId>,
    ) {
        if !active.insert(schema.id()) {
            return;
        }
        match schema.kind() {
            SchemaNodeKind::Object { required, .. } => out.extend(required.iter().cloned()),
            SchemaNodeKind::Const(value) => {
                if let Some(object) = value.as_object() {
                    out.extend(object.keys().cloned());
                }
            }
            SchemaNodeKind::Enum(values) if !values.is_empty() => {
                let mut iter = values.iter();
                if let Some(first) = iter.next().and_then(Value::as_object) {
                    let mut common: HashSet<String> = first.keys().cloned().collect();
                    for value in iter {
                        if let Some(object) = value.as_object() {
                            common.retain(|name| object.contains_key(name));
                        } else {
                            common.clear();
                            break;
                        }
                    }
                    out.extend(common);
                }
            }
            SchemaNodeKind::AllOf(children) => {
                for child in children {
                    guaranteed_names(child, out, active);
                }
            }
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children)
                if !children.is_empty() =>
            {
                let mut iter = children.iter();
                let mut common = HashSet::new();
                guaranteed_names(iter.next().expect("nonempty"), &mut common, active);
                for child in iter {
                    let mut child_names = HashSet::new();
                    guaranteed_names(child, &mut child_names, active);
                    common.retain(|name| child_names.contains(name));
                }
                out.extend(common);
            }
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), Some(else_schema)) => {
                    let mut common = HashSet::new();
                    guaranteed_names(then_schema, &mut common, active);
                    let mut else_names = HashSet::new();
                    guaranteed_names(else_schema, &mut else_names, active);
                    common.retain(|name| else_names.contains(name));
                    out.extend(common);
                }
                (Some(then_schema), None)
                    if whole_json_types_accepted_mask(if_schema) & JSON_TYPE_OBJECT != 0 =>
                {
                    guaranteed_names(then_schema, out, active);
                }
                (None, Some(else_schema))
                    if possible_json_type_mask(if_schema) & JSON_TYPE_OBJECT == 0 =>
                {
                    guaranteed_names(else_schema, out, active);
                }
                _ => {}
            },
            _ => {}
        }
        active.remove(&schema.id());
    }

    let object_only_overlap =
        (possible_json_type_mask(left) & possible_json_type_mask(right) & !JSON_TYPE_OBJECT) == 0;
    let guarantees = |schema: &SchemaNode, name: &str| {
        schema_guarantees_property_name(schema, name)
            || (object_only_overlap && schema_guarantees_property_name_for_objects(schema, name))
    };
    let forbids = |schema: &SchemaNode, name: &str| {
        schema_forbids_property_name(schema, name)
            || (object_only_overlap && schema_forbids_property_name_for_objects(schema, name))
    };

    let mut left_names = guaranteed_property_name_closure(left);
    guaranteed_names(left, &mut left_names, &mut HashSet::new());
    if left_names
        .iter()
        .any(|name| guarantees(left, name) && forbids(right, name))
    {
        return true;
    }
    let mut right_names = guaranteed_property_name_closure(right);
    guaranteed_names(right, &mut right_names, &mut HashSet::new());
    right_names
        .iter()
        .any(|name| guarantees(right, name) && forbids(left, name))
}

fn required_property_values_are_disjoint(left: &SchemaNode, right: &SchemaNode) -> bool {
    let left_values = finite_required_property_value_bounds(left);
    let right_values = finite_required_property_value_bounds(right);
    for (name, left_bound) in left_values {
        let Some(right_bound) = right_values.get(&name) else {
            continue;
        };
        if left_bound.is_empty() || right_bound.is_empty() {
            return true;
        }
        if left_bound.iter().all(|left_value| {
            right_bound
                .iter()
                .all(|right_value| !json_values_equal(left_value, right_value))
        }) {
            return true;
        }
    }
    required_vs_forbidden_property_are_disjoint(left, right)
        || required_property_schema_shapes_are_disjoint(left, right)
}

/// Return true when both object schemas force the same property and the forced
/// property's schemas have obviously disjoint primitive domains. This is a
/// deliberately smaller fact than full property-schema implication: it avoids
/// recursively invoking object disjointness (which can cycle through `$ref`s),
/// but still catches common tagged unions where the tag is separated by JSON
/// type or a simple numeric interval rather than by finite const/enum values.
fn required_property_schema_shapes_are_disjoint(left: &SchemaNode, right: &SchemaNode) -> bool {
    fn guaranteed_names(schema: &SchemaNode, active: &mut HashSet<NodeId>) -> HashSet<String> {
        if !active.insert(schema.id()) {
            return HashSet::new();
        }
        // Start with the dependency closure for this whole node. In an allOf,
        // a required trigger in one conjunct can activate a dependentRequired
        // rule in another; those implied names are just as guaranteed as
        // syntactic `required` names for discriminator-shape checks.
        let mut names = guaranteed_property_name_closure(schema);
        match schema.kind() {
            SchemaNodeKind::Object { .. } => {}
            SchemaNodeKind::AllOf(children) => {
                for child in children {
                    names.extend(guaranteed_names(child, active));
                }
            }
            // A union only guarantees names common to every branch. This is a
            // useful safe case, but avoid treating an empty union as universal.
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children)
                if !children.is_empty() =>
            {
                let mut iter = children.iter();
                let mut common = guaranteed_names(iter.next().expect("nonempty"), active);
                for child in iter {
                    let child_names = guaranteed_names(child, active);
                    common.retain(|name| child_names.contains(name));
                }
                names.extend(common);
            }
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), None)
                    if whole_json_types_accepted_mask(if_schema) & JSON_TYPE_OBJECT != 0 =>
                {
                    names.extend(guaranteed_names(then_schema, active));
                }
                (None, Some(else_schema))
                    if possible_json_type_mask(if_schema) & JSON_TYPE_OBJECT == 0 =>
                {
                    names.extend(guaranteed_names(else_schema, active));
                }
                (Some(then_schema), Some(else_schema)) => {
                    let mut common = guaranteed_names(then_schema, active);
                    let else_names = guaranteed_names(else_schema, active);
                    common.retain(|name| else_names.contains(name));
                    names.extend(common);
                }
                _ => {}
            },
            _ => {}
        }
        active.remove(&schema.id());
        names
    }

    fn collect_constraints_for_name<'a>(
        schema: &'a SchemaNode,
        name: &str,
        out: &mut Vec<&'a SchemaNode>,
        active: &mut HashSet<NodeId>,
    ) {
        if !active.insert(schema.id()) {
            return;
        }
        match schema.kind() {
            SchemaNodeKind::Object {
                properties,
                pattern_properties,
                additional,
                ..
            } => {
                let mut matched = false;
                if let Some(property_schema) = properties.get(name) {
                    matched = true;
                    out.push(property_schema);
                }
                let mut unsupported_pattern = false;
                for pattern_property in pattern_properties.values() {
                    if pattern_property.pattern.support() != PatternSupport::Supported {
                        unsupported_pattern = true;
                        continue;
                    }
                    if pattern_property.pattern.is_match(name) {
                        matched = true;
                        out.push(&pattern_property.schema);
                    }
                }
                if !matched && !unsupported_pattern {
                    out.push(additional);
                }
            }
            SchemaNodeKind::AllOf(children) => {
                for child in children {
                    collect_constraints_for_name(child, name, out, active);
                }
            }
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), None)
                    if whole_json_types_accepted_mask(if_schema) & JSON_TYPE_OBJECT != 0 =>
                {
                    collect_constraints_for_name(then_schema, name, out, active);
                }
                (None, Some(else_schema))
                    if possible_json_type_mask(if_schema) & JSON_TYPE_OBJECT == 0 =>
                {
                    collect_constraints_for_name(else_schema, name, out, active);
                }
                _ => {}
            },
            // For unions, a constraint applies to every accepted value only if
            // every branch contributes an obvious constraint for this name. We
            // do not try to merge those alternatives here; finite discriminator
            // extraction handles the common const/enum case separately.
            _ => {}
        }
        active.remove(&schema.id());
    }

    fn constraint_map(schema: &SchemaNode) -> HashMap<String, Vec<&SchemaNode>> {
        let names = guaranteed_names(schema, &mut HashSet::new());
        let mut result = HashMap::new();
        for name in names {
            let mut constraints = Vec::new();
            collect_constraints_for_name(schema, &name, &mut constraints, &mut HashSet::new());
            if !constraints.is_empty() {
                result.insert(name, constraints);
            }
        }
        result
    }

    let left_constraints = constraint_map(left);
    let right_constraints = constraint_map(right);

    left_constraints.iter().any(|(name, left_schemas)| {
        let Some(right_schemas) = right_constraints.get(name) else {
            return false;
        };
        left_schemas.iter().any(|left_schema| {
            right_schemas
                .iter()
                .any(|right_schema| primitive_domains_are_disjoint(left_schema, right_schema))
        })
    })
}

/// A deliberately small primitive-domain disjointness witness used inside
/// object and tuple discriminators. It never recurses into object/array shape
/// reasoning, which keeps `$ref` cycles from turning a cheap partition fact
/// into an unbounded proof search.
fn primitive_domains_are_disjoint(left: &SchemaNode, right: &SchemaNode) -> bool {
    if let (Some(left_values), Some(right_values)) = (
        finite_schema_value_superset(left),
        finite_schema_value_superset(right),
    ) && left_values.iter().all(|left_value| {
        right_values
            .iter()
            .all(|right_value| !json_values_equal(left_value, right_value))
    }) {
        return true;
    }

    let left_mask = possible_json_type_mask(left);
    let right_mask = possible_json_type_mask(right);
    let overlap = left_mask & right_mask;
    if overlap == 0 {
        return true;
    }
    (overlap == JSON_TYPE_NUMBER && numeric_intervals_are_disjoint(left, right))
        || (overlap == JSON_TYPE_STRING && string_length_intervals_are_disjoint(left, right))
        || (overlap == JSON_TYPE_ARRAY && array_length_intervals_are_disjoint(left, right))
        || (overlap == JSON_TYPE_OBJECT
            && object_property_count_intervals_are_disjoint(left, right))
}

/// Return true when two array schemas both force an item at the same tuple
/// position and the schemas applying to that item have disjoint primitive
/// domains. This catches tagged tuple unions without attempting general
/// item-subset reasoning.
fn required_array_item_shapes_are_disjoint(left: &SchemaNode, right: &SchemaNode) -> bool {
    const MAX_TRACKED_PREFIX: usize = 32;

    fn guaranteed_len(schema: &SchemaNode, active: &mut HashSet<NodeId>) -> usize {
        if !active.insert(schema.id()) {
            return 0;
        }
        let len = match schema.kind() {
            SchemaNodeKind::Array { item_count, .. } => {
                usize::try_from(item_count.min()).unwrap_or(usize::MAX)
            }
            SchemaNodeKind::AllOf(children) => children
                .iter()
                .map(|child| guaranteed_len(child, active))
                .max()
                .unwrap_or(0),
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children)
                if !children.is_empty() =>
            {
                children
                    .iter()
                    .map(|child| guaranteed_len(child, active))
                    .min()
                    .unwrap_or(0)
            }
            SchemaNodeKind::Const(value) => value.as_array().map_or(0, Vec::len),
            SchemaNodeKind::Enum(values) if !values.is_empty() => values
                .iter()
                .map(|value| value.as_array().map_or(0, Vec::len))
                .min()
                .unwrap_or(0),
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), None)
                    if whole_json_types_accepted_mask(if_schema) & JSON_TYPE_ARRAY != 0 =>
                {
                    guaranteed_len(then_schema, active)
                }
                (None, Some(else_schema))
                    if possible_json_type_mask(if_schema) & JSON_TYPE_ARRAY == 0 =>
                {
                    guaranteed_len(else_schema, active)
                }
                (Some(then_schema), Some(else_schema)) => {
                    guaranteed_len(then_schema, active).min(guaranteed_len(else_schema, active))
                }
                _ => 0,
            },
            _ => 0,
        };
        active.remove(&schema.id());
        len.min(MAX_TRACKED_PREFIX)
    }

    fn collect_constraints_at<'a>(
        schema: &'a SchemaNode,
        index: usize,
        out: &mut Vec<&'a SchemaNode>,
        active: &mut HashSet<NodeId>,
    ) {
        if !active.insert(schema.id()) {
            return;
        }
        match schema.kind() {
            SchemaNodeKind::Array {
                prefix_items,
                items,
                ..
            } => {
                if let Some(prefix) = prefix_items.get(index) {
                    out.push(prefix);
                } else {
                    out.push(items);
                }
            }
            SchemaNodeKind::AllOf(children) => {
                for child in children {
                    collect_constraints_at(child, index, out, active);
                }
            }
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), None)
                    if whole_json_types_accepted_mask(if_schema) & JSON_TYPE_ARRAY != 0 =>
                {
                    collect_constraints_at(then_schema, index, out, active);
                }
                (None, Some(else_schema))
                    if possible_json_type_mask(if_schema) & JSON_TYPE_ARRAY == 0 =>
                {
                    collect_constraints_at(else_schema, index, out, active);
                }
                // With two live branches, constraints are alternatives rather
                // than conjuncts; leave those to finite discriminator helpers.
                _ => {}
            },
            _ => {}
        }
        active.remove(&schema.id());
    }

    let shared_len =
        guaranteed_len(left, &mut HashSet::new()).min(guaranteed_len(right, &mut HashSet::new()));
    for index in 0..shared_len {
        let mut left_constraints = Vec::new();
        let mut right_constraints = Vec::new();
        collect_constraints_at(left, index, &mut left_constraints, &mut HashSet::new());
        collect_constraints_at(right, index, &mut right_constraints, &mut HashSet::new());
        if left_constraints.iter().any(|left_item| {
            right_constraints
                .iter()
                .any(|right_item| primitive_domains_are_disjoint(left_item, right_item))
        }) {
            return true;
        }
    }
    false
}

/// For each returned property name, every object accepted by `schema` has that
/// property and its value is contained in the returned finite upper bound.
/// Missing entries mean "unknown", not unconstrained. This is intentionally
/// small: direct object `required`+`properties` constraints and allOf
/// propagation cover the usual discriminator shape without attempting object
/// satisfiability.
fn finite_required_property_value_bounds(schema: &SchemaNode) -> HashMap<String, Vec<Value>> {
    fn intersect_value_bounds(left: &mut Vec<Value>, right: &[Value]) {
        left.retain(|left_value| {
            right
                .iter()
                .any(|right_value| json_values_equal(left_value, right_value))
        });
    }

    fn inner(schema: &SchemaNode, active: &mut HashSet<NodeId>) -> HashMap<String, Vec<Value>> {
        if !active.insert(schema.id()) {
            return HashMap::new();
        }
        let mut result = HashMap::new();
        match schema.kind() {
            SchemaNodeKind::Object { properties, .. } => {
                // Include names implied by dependentRequired closure, not just
                // syntactic `required`. If a forced name has a finite property
                // domain, it can serve as a discriminator too.
                for name in guaranteed_property_name_closure(schema) {
                    if let Some(property_schema) = properties.get(&name)
                        && let Some(values) = finite_schema_value_superset(property_schema)
                    {
                        result.insert(name, values);
                    }
                }
            }
            SchemaNodeKind::AllOf(children) => {
                for child in children {
                    for (name, mut values) in inner(child, active) {
                        match result.get_mut(&name) {
                            Some(existing) => intersect_value_bounds(existing, &values),
                            None => {
                                // Deduplicate for stable comparisons and to keep bounds small.
                                let mut deduped: Vec<Value> = Vec::new();
                                for value in values.drain(..) {
                                    if !deduped.iter().any(|seen| json_values_equal(seen, &value)) {
                                        deduped.push(value);
                                    }
                                }
                                result.insert(name, deduped);
                            }
                        }
                    }
                }
            }
            SchemaNodeKind::Const(value) => {
                if let Some(object) = value.as_object() {
                    for (name, property_value) in object {
                        result.insert(name.clone(), vec![property_value.clone()]);
                    }
                }
            }
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children) => {
                // A union guarantees a discriminator only when every branch
                // guarantees it.  The value bound is then the union of the
                // branch bounds.  This is useful for generated schemas that
                // wrap tagged objects in an anyOf/oneOf layer, and remains
                // conservative for empty or recursive unions.
                let mut child_iter = children.iter();
                if let Some(first_child) = child_iter.next() {
                    result = inner(first_child, active);
                    for child in child_iter {
                        let child_bounds = inner(child, active);
                        result.retain(|name, values| {
                            let Some(other_values) = child_bounds.get(name) else {
                                return false;
                            };
                            for value in other_values {
                                if !values.iter().any(|seen| json_values_equal(seen, value)) {
                                    values.push(value.clone());
                                }
                            }
                            true
                        });
                        if result.is_empty() {
                            break;
                        }
                    }
                }
            }
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), Some(else_schema)) => {
                    result = inner(then_schema, active);
                    let else_bounds = inner(else_schema, active);
                    result.retain(|name, values| {
                        let Some(other_values) = else_bounds.get(name) else {
                            return false;
                        };
                        for value in other_values {
                            if !values.iter().any(|seen| json_values_equal(seen, value)) {
                                values.push(value.clone());
                            }
                        }
                        true
                    });
                }
                (Some(then_schema), None)
                    if whole_json_types_accepted_mask(if_schema) & JSON_TYPE_OBJECT != 0 =>
                {
                    result = inner(then_schema, active);
                }
                (None, Some(else_schema))
                    if possible_json_type_mask(if_schema) & JSON_TYPE_OBJECT == 0 =>
                {
                    result = inner(else_schema, active);
                }
                _ => {}
            },
            SchemaNodeKind::Enum(values)
                if !values.is_empty() && values.iter().all(|value| value.as_object().is_some()) =>
            {
                // A property is guaranteed only if *every* enumerated value is
                // an object and has that property.  Mixed-type enums are common
                // in generated schemas; ignoring their non-object literals here
                // would make discriminator disjointness unsound.
                let first = values[0].as_object().expect("checked above");
                for (name, first_value) in first {
                    let mut bound = vec![first_value.clone()];
                    let mut guaranteed = true;
                    for object in values.iter().filter_map(Value::as_object) {
                        let Some(value) = object.get(name) else {
                            guaranteed = false;
                            break;
                        };
                        if !bound.iter().any(|seen| json_values_equal(seen, value)) {
                            bound.push(value.clone());
                        }
                    }
                    if guaranteed {
                        result.insert(name.clone(), bound);
                    }
                }
            }
            SchemaNodeKind::Enum(_) => {}
            _ => {}
        }
        active.remove(&schema.id());
        result
    }

    inner(schema, &mut HashSet::new())
}

/// Return the JSON types for which this schema is known to accept every value.
/// This is intentionally syntactic and conservative; it is used only to refine
/// complements and conditional guards.
fn whole_json_types_accepted_mask(schema: &SchemaNode) -> u8 {
    [
        JSON_TYPE_NULL,
        JSON_TYPE_BOOL,
        JSON_TYPE_NUMBER,
        JSON_TYPE_STRING,
        JSON_TYPE_ARRAY,
        JSON_TYPE_OBJECT,
    ]
    .into_iter()
    .fold(0, |mask, bit| {
        if schema_obviously_accepts_json_type(schema, bit) {
            mask | bit
        } else {
            mask
        }
    })
}

/// Upper bound on the JSON types that may satisfy `not schema`. If `schema`
/// accepts every value of a type, the complement cannot contain that type;
/// otherwise keep the type as possible.
fn complement_type_mask_upper_bound(schema: &SchemaNode) -> u8 {
    JSON_TYPE_ALL & !whole_json_types_accepted_mask(schema)
}

/// Return a sound upper bound on the JSON value types a schema may accept.
/// Disjoint upper bounds imply disjoint languages; overlapping bounds say
/// nothing. Applicators are handled with ordinary set algebra where it is
/// safe, and unknown cases fall back to all types.
fn possible_json_type_mask(schema: &SchemaNode) -> u8 {
    fn value_mask(value: &Value) -> u8 {
        match value {
            Value::Null => JSON_TYPE_NULL,
            Value::Bool(_) => JSON_TYPE_BOOL,
            Value::Number(_) => JSON_TYPE_NUMBER,
            Value::String(_) => JSON_TYPE_STRING,
            Value::Array(_) => JSON_TYPE_ARRAY,
            Value::Object(_) => JSON_TYPE_OBJECT,
        }
    }

    fn inner(schema: &SchemaNode, active: &mut HashSet<NodeId>) -> u8 {
        if !active.insert(schema.id()) {
            return JSON_TYPE_ALL;
        }
        let mask = match schema.kind() {
            SchemaNodeKind::BoolSchema(false) => 0,
            SchemaNodeKind::BoolSchema(true) | SchemaNodeKind::Any => JSON_TYPE_ALL,
            SchemaNodeKind::String { enumeration, .. } => {
                enumeration.as_ref().map_or(JSON_TYPE_STRING, |values| {
                    values.iter().fold(0, |m, v| m | value_mask(v))
                })
            }
            SchemaNodeKind::Number { enumeration, .. }
            | SchemaNodeKind::Integer { enumeration, .. } => {
                enumeration.as_ref().map_or(JSON_TYPE_NUMBER, |values| {
                    values.iter().fold(0, |m, v| m | value_mask(v))
                })
            }
            SchemaNodeKind::Boolean { enumeration } => {
                enumeration.as_ref().map_or(JSON_TYPE_BOOL, |values| {
                    values.iter().fold(0, |m, v| m | value_mask(v))
                })
            }
            SchemaNodeKind::Null { enumeration } => {
                enumeration.as_ref().map_or(JSON_TYPE_NULL, |values| {
                    values.iter().fold(0, |m, v| m | value_mask(v))
                })
            }
            SchemaNodeKind::Object { enumeration, .. } => {
                enumeration.as_ref().map_or(JSON_TYPE_OBJECT, |values| {
                    values.iter().fold(0, |m, v| m | value_mask(v))
                })
            }
            SchemaNodeKind::Array { enumeration, .. } => {
                enumeration.as_ref().map_or(JSON_TYPE_ARRAY, |values| {
                    values.iter().fold(0, |m, v| m | value_mask(v))
                })
            }
            SchemaNodeKind::Const(value) => value_mask(value),
            SchemaNodeKind::Enum(values) => values.iter().fold(0, |m, v| m | value_mask(v)),
            SchemaNodeKind::AllOf(children) => children
                .iter()
                .fold(JSON_TYPE_ALL, |m, child| m & inner(child, active)),
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children) => {
                children.iter().fold(0, |m, child| m | inner(child, active))
            }
            SchemaNodeKind::Not(child) => match child.kind() {
                SchemaNodeKind::BoolSchema(true) => 0,
                SchemaNodeKind::BoolSchema(false) => JSON_TYPE_ALL,
                _ => {
                    // If the negated schema accepts *every* value of a JSON
                    // type, then `not` excludes that entire type. This is a
                    // cheap upper-bound refinement (for example, `not: {type:
                    // string}` cannot accept strings) and remains conservative
                    // for partial constraints such as `not: {minLength: 2}`.
                    complement_type_mask_upper_bound(child)
                }
            },
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => {
                // A conditional accepts values from the guarded-then side or
                // from the negated-guard else side. Include cheap type facts
                // from the guard itself; this keeps schemas like
                // `if type=string, then true, else number` from looking like
                // they may accept every JSON type.
                let guard_mask = inner(if_schema, active);
                let not_guard_mask = complement_type_mask_upper_bound(if_schema);
                let then_mask = then_schema
                    .as_ref()
                    .map_or(JSON_TYPE_ALL, |child| inner(child, active));
                let else_mask = else_schema
                    .as_ref()
                    .map_or(JSON_TYPE_ALL, |child| inner(child, active));
                (guard_mask & then_mask) | (not_guard_mask & else_mask)
            }
            _ => JSON_TYPE_ALL,
        };
        active.remove(&schema.id());
        mask
    }

    inner(schema, &mut HashSet::new())
}

fn schema_is_trivially_universal(schema: &SchemaNode) -> bool {
    fn inner(schema: &SchemaNode, active: &mut HashSet<NodeId>) -> bool {
        if !active.insert(schema.id()) {
            return false;
        }
        let result = match schema.kind() {
            SchemaNodeKind::Any | SchemaNodeKind::BoolSchema(true) => true,
            SchemaNodeKind::Not(child) => matches!(child.kind(), SchemaNodeKind::BoolSchema(false)),
            SchemaNodeKind::AllOf(children) => children.iter().all(|child| inner(child, active)),
            SchemaNodeKind::AnyOf(children) => children.iter().any(|child| inner(child, active)),
            _ => false,
        };
        active.remove(&schema.id());
        result
    }
    inner(schema, &mut HashSet::new())
}

/// Return a finite (not necessarily minimal) superset of the JSON values that
/// can satisfy `schema`, when such a superset is syntactically obvious.
///
/// This is intentionally an *upper* bound, not an exact evaluator. Enum and
/// const keywords cap a language even when another conjunct uses an
/// unsupported regex or recursion, and an `allOf` language is capped by any
/// finite child. Callers use the result for pigeonhole/capacity arguments, so
/// over-approximating is safe while under-approximating would not be. Cycles
/// simply make the helper give up.
/// Non-recursive emptiness check used while constructing finite value supersets.
/// The full emptiness prover may recursively ask for another finite superset;
/// doing that from inside enumeration can re-enter recursive schemas with a
/// fresh visitation set. Keep this deliberately local.
fn schema_is_locally_empty_for_finite_enumeration(schema: &SchemaNode) -> bool {
    matches!(schema.kind(), SchemaNodeKind::BoolSchema(false))
        || matches!(schema.kind(), SchemaNodeKind::Enum(values) if values.is_empty())
        || matches!(constrained_enumeration(schema), Some(values) if values.is_empty())
}

fn schema_has_obvious_finite_property_names(schema: &SchemaNode) -> bool {
    fn inner(schema: &SchemaNode, active: &mut HashSet<NodeId>) -> bool {
        if !active.insert(schema.id()) {
            return false;
        }
        let result = match schema.kind() {
            SchemaNodeKind::Const(_) | SchemaNodeKind::Enum(_) => true,
            SchemaNodeKind::String {
                enumeration: Some(_),
                ..
            } => true,
            SchemaNodeKind::String { length, .. } if length.max() == Some(0) => true,
            SchemaNodeKind::AllOf(children) => children.iter().any(|child| inner(child, active)),
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children) => {
                children.iter().all(|child| inner(child, active))
            }
            _ => false,
        };
        active.remove(&schema.id());
        result
    }
    inner(schema, &mut HashSet::new())
}

fn finite_integer_multiple_number_values(
    bounds: NumberBounds,
    multiple_of: NumberMultipleOf,
) -> Option<Vec<Value>> {
    // A number schema with an integral multipleOf has only integer-valued
    // inhabitants. With finite endpoints that gives a finite domain; keep the
    // enumeration tiny and within exactly-representable f64 integers.
    if !multiple_of.is_integer_valued() || multiple_of.as_f64().fract() != 0.0 {
        return None;
    }
    let divisor = multiple_of.integer_divisor()?;
    if divisor <= 0 || divisor > i128::from(i64::MAX) {
        return None;
    }

    const MAX_EXACT_F64_INTEGER_LOCAL: f64 = 9_007_199_254_740_991.0;
    fn integer_floor(value: f64) -> Option<i128> {
        (value.is_finite() && value.abs() <= MAX_EXACT_F64_INTEGER_LOCAL)
            .then(|| value.floor() as i128)
    }
    fn integer_ceil(value: f64) -> Option<i128> {
        (value.is_finite() && value.abs() <= MAX_EXACT_F64_INTEGER_LOCAL)
            .then(|| value.ceil() as i128)
    }

    let lower = match bounds.lower() {
        NumberBound::Unbounded => return None,
        NumberBound::Inclusive(value) => integer_ceil(value)?,
        NumberBound::Exclusive(value) => {
            let ceil = integer_ceil(value)?;
            if value.fract() == 0.0 { ceil + 1 } else { ceil }
        }
    };
    let upper = match bounds.upper() {
        NumberBound::Unbounded => return None,
        NumberBound::Inclusive(value) => integer_floor(value)?,
        NumberBound::Exclusive(value) => {
            let floor = integer_floor(value)?;
            if value.fract() == 0.0 {
                floor - 1
            } else {
                floor
            }
        }
    };
    if upper < lower {
        return Some(Vec::new());
    }

    let rem = lower.rem_euclid(divisor);
    let first = if rem == 0 {
        lower
    } else {
        lower + (divisor - rem)
    };
    if first > upper {
        return Some(Vec::new());
    }
    let count = ((upper - first) / divisor) + 1;
    if count > 256 {
        return None;
    }
    let mut values = Vec::new();
    for offset in 0..count {
        let raw = first + offset * divisor;
        let as_i64 = i64::try_from(raw).ok()?;
        values.push(Value::Number(as_i64.into()));
    }
    Some(values)
}

pub(super) fn finite_schema_value_superset(schema: &SchemaNode) -> Option<Vec<Value>> {
    fn push_distinct(values: &mut Vec<Value>, value: Value) {
        if !values.iter().any(|seen| json_values_equal(seen, &value)) {
            values.push(value);
        }
    }

    fn collect_enum(values: &[Value]) -> Vec<Value> {
        let mut distinct = Vec::new();
        for value in values {
            push_distinct(&mut distinct, value.clone());
        }
        distinct
    }

    fn enumerate_object_values(
        keys: &[String],
        domains: &[Vec<Value>],
        required: &HashSet<String>,
        property_count: CountRange<usize>,
    ) -> Option<Vec<Value>> {
        let mut objects = Vec::new();
        let mut candidate_count = 0_usize;
        let subset_count = 1_usize << keys.len();
        for mask in 0..subset_count {
            let selected_len = mask.count_ones() as usize;
            if selected_len < property_count.min()
                || property_count.max().is_some_and(|max| selected_len > max)
            {
                continue;
            }
            if keys
                .iter()
                .enumerate()
                .any(|(index, key)| required.contains(key) && (mask & (1_usize << index)) == 0)
            {
                continue;
            }

            let mut partials = vec![serde_json::Map::new()];
            let mut impossible = false;
            for (index, key) in keys.iter().enumerate() {
                if (mask & (1_usize << index)) == 0 {
                    continue;
                }
                if domains[index].is_empty() {
                    impossible = true;
                    break;
                }
                let mut next = Vec::new();
                for partial in &partials {
                    for value in &domains[index] {
                        let mut extended = partial.clone();
                        extended.insert(key.clone(), value.clone());
                        next.push(extended);
                        candidate_count = candidate_count.saturating_add(1);
                        if candidate_count > 256 {
                            return None;
                        }
                    }
                }
                partials = next;
            }
            if impossible {
                continue;
            }
            for object in partials {
                push_distinct(&mut objects, Value::Object(object));
            }
        }
        Some(objects)
    }

    fn inner(schema: &SchemaNode, active: &mut HashSet<NodeId>) -> Option<Vec<Value>> {
        if !active.insert(schema.id()) {
            return None;
        }

        use SchemaNodeKind::*;
        let result = match schema.kind() {
            BoolSchema(false) => Some(Vec::new()),
            Const(value) => Some(vec![value.clone()]),
            Enum(values) => Some(collect_enum(values)),
            String {
                enumeration: Some(values),
                ..
            }
            | Number {
                enumeration: Some(values),
                ..
            }
            | Integer {
                enumeration: Some(values),
                ..
            }
            | Boolean {
                enumeration: Some(values),
            }
            | Null {
                enumeration: Some(values),
            }
            | Object {
                enumeration: Some(values),
                ..
            }
            | Array {
                enumeration: Some(values),
                ..
            } => Some(collect_enum(values)),
            String {
                length,
                enumeration: None,
                ..
            } if length.max() == Some(0) => {
                // At most one string has length zero. Other string keywords
                // (pattern/format) may reject it, but cannot introduce a
                // second value.
                Some(vec![Value::String(std::string::String::new())])
            }
            Integer {
                bounds,
                multiple_of,
                enumeration: None,
            } => {
                let (Some(lower), Some(upper)) = (bounds.lower(), bounds.upper()) else {
                    active.remove(&schema.id());
                    return None;
                };
                // Keep this helper cheap and avoid huge allocations. Giving
                // up on wider ranges is conservative.
                if upper < lower || upper.saturating_sub(lower) > 256 {
                    active.remove(&schema.id());
                    return None;
                }
                let divisor = multiple_of
                    .as_ref()
                    .and_then(|m| m.integer_divisor())
                    .filter(|divisor| *divisor > 0);
                let mut values = Vec::new();
                for i in lower..=upper {
                    if divisor.is_none_or(|d| i128::from(i).rem_euclid(d) == 0) {
                        values.push(Value::Number(i.into()));
                    }
                }
                Some(values)
            }
            Number {
                bounds,
                multiple_of,
                enumeration: None,
            } => {
                if let Some(multiple_of) = multiple_of
                    && let Some(values) =
                        finite_integer_multiple_number_values(*bounds, *multiple_of)
                {
                    Some(values)
                } else {
                    match (bounds.lower(), bounds.upper()) {
                        (NumberBound::Inclusive(lower), NumberBound::Inclusive(upper))
                            if lower.to_bits() == upper.to_bits() =>
                        {
                            serde_json::Number::from_f64(lower)
                                .map(Value::Number)
                                .map(|value| vec![value])
                        }
                        _ => None,
                    }
                }
            }
            // These primitive domains are genuinely finite even without an
            // explicit enum. Keeping them here lets uniqueItems reasoning see
            // boolean/null item schemas and propertyNames reject non-strings.
            Boolean { enumeration: None } => Some(vec![Value::Bool(false), Value::Bool(true)]),
            Null { enumeration: None } => Some(vec![Value::Null]),
            Object {
                properties,
                pattern_properties,
                required,
                additional,
                property_count,
                enumeration: None,
                ..
            } if pattern_properties.is_empty()
                && matches!(additional.kind(), SchemaNodeKind::BoolSchema(false)) =>
            {
                // A closed object with no pattern properties can mention only
                // its declared property names. When each declared property's
                // value language is finite, the whole object language is
                // finite too. This deliberately ignores propertyNames and
                // dependentRequired while constructing the upper bound; the
                // final exact-evaluator filter below (when available) tightens
                // those away, and leaving extra candidates is still sound.
                if properties.len() > 5 {
                    active.remove(&schema.id());
                    return None;
                }

                let mut keys = properties.keys().collect::<Vec<_>>();
                keys.sort();

                // Required undeclared names cannot be supplied in a closed
                // object, so the language is empty.
                if required.iter().any(|name| !properties.contains_key(name)) {
                    active.remove(&schema.id());
                    return Some(Vec::new());
                }

                if property_count.max().is_some_and(|max| required.len() > max)
                    || property_count.min() > properties.len()
                {
                    active.remove(&schema.id());
                    return Some(Vec::new());
                }

                let required_saturates_max = property_count
                    .max()
                    .is_some_and(|max| max == required.len());
                let mut domains = Vec::with_capacity(keys.len());
                for key in &keys {
                    if required_saturates_max && !required.contains(*key) {
                        // Optional names cannot appear when required names
                        // already fill maxProperties, so an unknown or
                        // recursive optional property schema does not make the
                        // object language infinite.
                        domains.push(Vec::new());
                        continue;
                    }
                    let Some(values) = inner(&properties[*key], active) else {
                        active.remove(&schema.id());
                        return None;
                    };
                    domains.push(values);
                }

                let key_strings = keys.iter().map(|key| (*key).clone()).collect::<Vec<_>>();
                let objects =
                    enumerate_object_values(&key_strings, &domains, required, *property_count);
                if objects.is_none() {
                    active.remove(&schema.id());
                }
                objects
            }
            Object {
                properties,
                pattern_properties,
                required,
                additional,
                property_names,
                property_count,
                enumeration: None,
                ..
            } if property_count.max() != Some(0)
                && schema_has_obvious_finite_property_names(property_names)
                && pattern_properties.values().all(|pattern_property| {
                    pattern_property.pattern.support() == PatternSupport::Supported
                }) =>
            {
                // `propertyNames` can make the key space finite even for an
                // otherwise-open object. If every possible key has at least
                // one finite applicable value constraint, enumerate a small
                // superset of object values. Unsupported patternProperties
                // are excluded by the guard above because they make it
                // impossible to know whether `additional` applies.
                let Some(name_values) = inner(property_names, active) else {
                    active.remove(&schema.id());
                    return None;
                };
                let mut keys: Vec<std::string::String> = Vec::new();
                for value in name_values {
                    if let Some(name) = value.as_str()
                        && !keys.iter().any(|seen| seen == name)
                    {
                        keys.push(name.to_owned());
                    }
                }
                keys.sort();
                if keys.len() > 5 {
                    active.remove(&schema.id());
                    return None;
                }

                // Required names outside the finite name superset cannot be
                // present in any valid object.
                if required
                    .iter()
                    .any(|name| !keys.iter().any(|key| key == name))
                {
                    active.remove(&schema.id());
                    return Some(Vec::new());
                }
                if property_count.max().is_some_and(|max| required.len() > max)
                    || property_count.min() > keys.len()
                {
                    active.remove(&schema.id());
                    return Some(Vec::new());
                }

                let required_saturates_max = property_count
                    .max()
                    .is_some_and(|max| max == required.len());
                let mut domains = Vec::with_capacity(keys.len());
                for key in &keys {
                    if required_saturates_max && !required.contains(key) {
                        domains.push(Vec::new());
                        continue;
                    }
                    let explicit_schema = properties.get(key);
                    let mut applicable: Vec<&SchemaNode> = Vec::new();
                    if let Some(schema) = explicit_schema {
                        applicable.push(schema);
                    }
                    let mut matched_pattern = false;
                    for pattern_property in pattern_properties.values() {
                        if pattern_property.pattern.is_match(key) {
                            matched_pattern = true;
                            applicable.push(&pattern_property.schema);
                        }
                    }
                    if explicit_schema.is_none() && !matched_pattern {
                        applicable.push(additional);
                    }

                    let mut finite_values = None;
                    for candidate_schema in applicable {
                        if let Some(values) = inner(candidate_schema, active) {
                            finite_values = Some(values);
                            break;
                        }
                    }
                    let Some(values) = finite_values else {
                        active.remove(&schema.id());
                        return None;
                    };
                    domains.push(values);
                }

                let objects = enumerate_object_values(&keys, &domains, required, *property_count);
                if objects.is_none() {
                    active.remove(&schema.id());
                }
                objects
            }
            Object {
                property_count,
                enumeration: None,
                ..
            } if property_count.max() == Some(0) => {
                // With maxProperties: 0 there is at most one object value: {}
                // (required/dependent/property schemas may still reject it).
                // Treating it as an upper bound is useful for uniqueItems
                // pigeonhole reasoning over arrays of empty objects.
                Some(vec![Value::Object(serde_json::Map::new())])
            }
            Array {
                prefix_items,
                items,
                item_count,
                enumeration: None,
                ..
            } => {
                let mut inferred_max = item_count.max();
                for (index, prefix_item) in prefix_items.iter().enumerate() {
                    if schema_is_locally_empty_for_finite_enumeration(prefix_item) {
                        let ceiling = u64::try_from(index).unwrap_or(u64::MAX);
                        inferred_max = Some(inferred_max.map_or(ceiling, |max| max.min(ceiling)));
                        break;
                    }
                }
                if schema_is_locally_empty_for_finite_enumeration(items) {
                    let ceiling = u64::try_from(prefix_items.len()).unwrap_or(u64::MAX);
                    inferred_max = Some(inferred_max.map_or(ceiling, |max| max.min(ceiling)));
                }
                let Some(max_items) = inferred_max else {
                    active.remove(&schema.id());
                    return None;
                };
                // Enumerate only very small bounded array languages. This is
                // a finite *superset*: contains/uniqueItems may reject some of
                // these candidates, but every valid array of these lengths is
                // represented when each positional item domain is finite.
                if max_items > 3 {
                    active.remove(&schema.id());
                    return None;
                }
                let min_items = item_count.min();
                let mut arrays: Vec<Value> = Vec::new();
                let mut candidate_count = 0_usize;
                for len_u64 in min_items..=max_items {
                    let len = match usize::try_from(len_u64) {
                        Ok(len) => len,
                        Err(_) => {
                            active.remove(&schema.id());
                            return None;
                        }
                    };
                    let mut choices: Vec<Vec<Value>> = Vec::new();
                    let mut impossible_length = false;
                    for index in 0..len {
                        let item_schema = prefix_items.get(index).unwrap_or(items);
                        let Some(values) = inner(item_schema, active) else {
                            active.remove(&schema.id());
                            return None;
                        };
                        if values.is_empty() {
                            impossible_length = true;
                            break;
                        }
                        choices.push(values);
                    }
                    if impossible_length {
                        continue;
                    }

                    let mut partials: Vec<Vec<Value>> = vec![Vec::new()];
                    for values in choices {
                        let mut next = Vec::new();
                        for partial in &partials {
                            for value in &values {
                                let mut extended = partial.clone();
                                extended.push(value.clone());
                                next.push(extended);
                                candidate_count = candidate_count.saturating_add(1);
                                if candidate_count > 256 {
                                    active.remove(&schema.id());
                                    return None;
                                }
                            }
                        }
                        partials = next;
                    }
                    for array in partials {
                        push_distinct(&mut arrays, Value::Array(array));
                    }
                }
                Some(arrays)
            }
            AnyOf(children) | OneOf(children) => {
                let mut union = Vec::new();
                let mut all_finite = true;
                for child in children {
                    let Some(child_values) = inner(child, active) else {
                        all_finite = false;
                        break;
                    };
                    for value in child_values {
                        push_distinct(&mut union, value);
                    }
                }
                if all_finite {
                    union.retain(|value| {
                        !children.iter().all(|child| {
                            !schema_may_under_accept_values(child) && !child.accepts_value(value)
                        })
                    });
                    Some(union)
                } else {
                    None
                }
            }
            AllOf(children) => {
                // The intersection is a subset of every child. Any finite
                // child therefore gives a sound finite upper bound; choose the
                // smallest one we can find to keep later bounds useful.
                let mut best: Option<Vec<Value>> = None;
                for child in children {
                    if let Some(child_values) = inner(child, active)
                        && best
                            .as_ref()
                            .is_none_or(|current| child_values.len() < current.len())
                    {
                        best = Some(child_values);
                    }
                }
                best.map(|mut values| {
                    // Tighten the bound by dropping candidates that another
                    // conjunct definitively rejects. This never removes a
                    // real inhabitant, but it often turns enum/intersection
                    // shapes into singleton (or empty) domains for
                    // cardinality reasoning.
                    values.retain(|value| {
                        !children.iter().any(|child| {
                            !schema_may_under_accept_values(child) && !child.accepts_value(value)
                        })
                    });
                    values
                })
            }
            IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => {
                let merge = |mut left: Vec<Value>, right: Vec<Value>| {
                    for value in right {
                        push_distinct(&mut left, value);
                    }
                    left
                };

                match if_schema.kind() {
                    // Constant conditions collapse to the selected branch. A
                    // missing selected branch is equivalent to `true`, so it is
                    // not finite.
                    BoolSchema(true) => then_schema
                        .as_ref()
                        .and_then(|branch| inner(branch, active)),
                    BoolSchema(false) => else_schema
                        .as_ref()
                        .and_then(|branch| inner(branch, active)),
                    _ if schema_is_locally_empty_for_finite_enumeration(if_schema) => else_schema
                        .as_ref()
                        .and_then(|branch| inner(branch, active)),
                    _ if schema_is_trivially_universal(if_schema) => then_schema
                        .as_ref()
                        .and_then(|branch| inner(branch, active)),
                    _ => {
                        let then_values = then_schema
                            .as_ref()
                            .and_then(|branch| inner(branch, active));
                        let else_values = else_schema
                            .as_ref()
                            .and_then(|branch| inner(branch, active));
                        match (then_values, else_values) {
                            (Some(union), Some(else_values)) => Some(merge(union, else_values)),
                            (None, Some(else_values)) if then_schema.is_none() => {
                                // Instances satisfying a finite `if` schema take
                                // the unconstrained then side; all other valid
                                // instances must come from the finite else side.
                                // The condition values are an upper bound, so
                                // unioning them is sound even when later filters
                                // cannot evaluate the condition exactly.
                                inner(if_schema, active)
                                    .map(|condition_values| merge(condition_values, else_values))
                            }
                            (Some(then_values), None) if else_schema.is_none() => {
                                // Symmetric special case: for `if: {not: S}`
                                // with no else branch, the unconstrained else
                                // side is bounded by S.  Avoid trying to reason
                                // about arbitrary complements; this syntactic
                                // shape is common after schema generation and
                                // keeps the bound obviously sound.
                                match if_schema.kind() {
                                    Not(negated) => inner(negated, active)
                                        .map(|else_side| merge(then_values, else_side)),
                                    _ => None,
                                }
                            }
                            _ => None,
                        }
                    }
                }
            }
            _ => None,
        };

        let result = result.map(|mut values| {
            // Drop candidates only when the internal evaluator cannot fail
            // closed for this schema. Over-acceptance merely leaves extra
            // candidates in the finite superset, which is harmless.
            if !schema_may_under_accept_values(schema) {
                values.retain(|value| schema.accepts_value(value));
            }
            values
        });
        active.remove(&schema.id());
        result
    }

    inner(schema, &mut HashSet::new())
}

pub(super) fn schema_may_under_accept_values(schema: &SchemaNode) -> bool {
    schema_acceptance_deviation(schema).may_under_accept
}

/// Cheap, sound proof that a schema accepts no values at all.
///
/// Besides literal `false`, normalized typed schemas with an enum restriction
/// are empty when every enum member is definitively rejected by the remaining
/// constraints. The enum is a hard cap on the raw language, so this remains
/// sound as long as the evaluator is not known to under-accept that schema.
pub(super) fn schema_definitely_rejects_all_values(schema: &SchemaNode) -> bool {
    if matches!(schema.kind(), SchemaNodeKind::BoolSchema(false)) {
        return true;
    }
    if matches!(schema.kind(), SchemaNodeKind::Enum(values) if values.is_empty()) {
        return true;
    }

    let Some(values) = constrained_enumeration(schema) else {
        if let Some(values) = finite_schema_value_superset(schema) {
            return values.is_empty()
                || (!schema_may_under_accept_values(schema)
                    && values.iter().all(|value| !schema.accepts_value(value)));
        }
        return false;
    };
    values.is_empty()
        || (!schema_may_under_accept_values(schema)
            && values.iter().all(|value| !schema.accepts_value(value)))
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
            let guaranteed_names_storage = (!sub_dependent_required.is_empty())
                .then(|| object::names_forced_by_required(sub_required, sub_dependent_required));
            let guaranteed_names = guaranteed_names_storage.as_ref().unwrap_or(sub_required);
            let Some(mut effective_sub_property_count) =
                object::effective_property_count_with_forced_names(
                    *sub_property_count,
                    guaranteed_names,
                )
            else {
                // Required/dependentRequired names already exceed maxProperties,
                // so this object branch has no inhabitants.
                return None;
            };
            if let Some(name_capacity) = object::finite_property_name_capacity(sub_property_names) {
                let capped_max = Some(
                    effective_sub_property_count
                        .max()
                        .map_or(name_capacity, |max| max.min(name_capacity)),
                );
                let capped_count = CountRange::new(effective_sub_property_count.min(), capped_max)?;
                effective_sub_property_count = capped_count;
            }

            if let Some(property) = sup_required.difference(guaranteed_names).next() {
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

            // With maxProperties: 0, no named property, pattern property,
            // additional property, propertyNames, or dependentRequired
            // constraint can be exercised by a subset instance. Check the
            // count range here (the main checker does it first), then stop
            // before producing a spurious per-property explanation.
            if effective_sub_property_count.max() == Some(0) {
                if !sup_property_count.contains_range(effective_sub_property_count) {
                    return Some(SubschemaExplanation::new(format!(
                        "object property count range {} is not contained by required range {}",
                        format_count_range(effective_sub_property_count),
                        format_count_range(*sup_property_count),
                    )));
                }
                return None;
            }

            let property_can_fit = |property: &str| {
                object::property_name_can_fit_with_dependencies(
                    property,
                    guaranteed_names,
                    effective_sub_property_count,
                    sub_dependent_required,
                )
            };

            for (property, sub_schema) in sub_properties {
                if !property_can_fit(property) {
                    continue;
                }
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
                if !property_can_fit(trigger) {
                    continue;
                }
                if let Some(dependency) = dependencies.iter().find(|dependency| {
                    !dependent_requirement_is_guaranteed(
                        trigger,
                        dependency,
                        guaranteed_names,
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
                if !property_can_fit(property) {
                    continue;
                }
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

            if !object::property_names_subsumed_with_count(
                sub_property_names,
                sup_property_names,
                guaranteed_names,
                effective_sub_property_count,
                context,
            ) {
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

            if !sup_property_count.contains_range(effective_sub_property_count) {
                return Some(SubschemaExplanation::new(format!(
                    "object property count range {} is not contained by required range {}",
                    format_count_range(effective_sub_property_count),
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
            if array::array_constraints_definitely_uninhabited(&array::ArrayConstraints {
                prefix_items: sub_prefix_items,
                items: sub_items,
                item_count: *sub_item_count,
                contains: sub_contains.as_ref(),
                unique_items: *sub_unique_items,
                enumeration: sub_enum.as_deref(),
            }) {
                return None;
            }

            let Some(effective_sub_item_count) =
                array::effective_item_count_for_unique_finite_domain(
                    sub_prefix_items,
                    sub_items,
                    *sub_item_count,
                    *sub_unique_items,
                )
            else {
                // The subset array branch is empty (for example, uniqueItems
                // with minItems above a finite item domain), so it is vacuously
                // contained.
                return None;
            };

            if sub_contains.as_ref().is_some_and(|contains| {
                array::contains_requirement_definitely_impossible(
                    contains,
                    effective_sub_item_count,
                    *sub_unique_items,
                ) || array::contains_requirement_impossible_for_unique_finite_items(
                    sub_prefix_items,
                    sub_items,
                    effective_sub_item_count,
                    contains,
                    *sub_unique_items,
                )
            }) {
                return None;
            }

            if !sup_item_count.contains_range(effective_sub_item_count) {
                return Some(SubschemaExplanation::new(format!(
                    "array length range {} is not contained by required range {}",
                    format_count_range(effective_sub_item_count),
                    format_count_range(*sup_item_count),
                )));
            }

            if *sup_unique_items
                && !*sub_unique_items
                && effective_sub_item_count
                    .max()
                    .is_none_or(|max_items| max_items > 1)
            {
                return Some(SubschemaExplanation::new(
                    "arrays may contain duplicate items, but the comparison target requires unique items",
                )
                .in_superset()
                .at_keyword("uniqueItems"));
            }

            let checked_prefix_len = sub_prefix_items.len().max(sup_prefix_items.len());
            for index in 0..checked_prefix_len {
                if !array_index_can_exist(effective_sub_item_count.max(), index) {
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

            if array_index_can_exist(effective_sub_item_count.max(), checked_prefix_len)
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
                        effective_sub_item_count.min(),
                        &sup_contains.schema,
                        sup_count.min(),
                        context,
                    )
                    || array::unique_finite_domain_guarantees_contains_at_least(
                        sub_prefix_items,
                        sub_items,
                        effective_sub_item_count.min(),
                        *sub_unique_items,
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
                        }) || effective_sub_item_count
                            .max()
                            .is_some_and(|sub_max_items| sub_max_items <= sup_max_contains)
                            || (*sub_unique_items
                                && array::finite_match_domain_size(&sup_contains.schema)
                                    .is_some_and(|domain_size| domain_size <= sup_max_contains))
                            || array::array_items_match_at_most(
                                sub_prefix_items,
                                sub_items,
                                effective_sub_item_count.max(),
                                *sub_unique_items,
                                &sup_contains.schema,
                                context,
                            )
                            .is_some_and(|max_matches| max_matches <= sup_max_contains);
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
    fn discriminator_disjoint_items_satisfy_zero_max_contains() {
        let old = resolve(json!({
            "type": "array",
            "contains": {
                "type": "object",
                "required": ["kind"],
                "properties": { "kind": { "const": "blocked" } }
            },
            "minContains": 0,
            "maxContains": 0
        }));
        let new = resolve(json!({
            "type": "array",
            "items": {
                "type": "object",
                "required": ["kind"],
                "properties": { "kind": { "const": "allowed" } }
            }
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn range_disjoint_items_satisfy_zero_max_contains() {
        let old = resolve(json!({
            "type": "array",
            "contains": { "type": "number", "minimum": 10 },
            "minContains": 0,
            "maxContains": 0
        }));
        let new = resolve(json!({
            "type": "array",
            "items": { "type": "number", "maximum": 0 }
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn type_disjoint_items_satisfy_zero_max_contains() {
        let old = resolve(json!({
            "type": "array",
            "contains": { "type": "number" },
            "minContains": 0,
            "maxContains": 0
        }));
        let new = resolve(json!({
            "type": "array",
            "items": { "type": "string" }
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn discriminator_disjoint_tuple_positions_make_unique_items_redundant() {
        let old = resolve(json!({
            "type": "array",
            "minItems": 2,
            "maxItems": 2,
            "uniqueItems": true
        }));
        let tagged = |tag: &str| {
            json!({
                "type": "object",
                "required": ["kind"],
                "properties": { "kind": { "const": tag } }
            })
        };
        let new = resolve(json!({
            "type": "array",
            "prefixItems": [tagged("left"), tagged("right")],
            "items": false,
            "minItems": 2,
            "maxItems": 2
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn disjoint_tuple_positions_make_unique_items_redundant() {
        let old = resolve(json!({
            "type": "array",
            "minItems": 2,
            "maxItems": 2,
            "uniqueItems": true
        }));
        let new = resolve(json!({
            "type": "array",
            "prefixItems": [
                { "type": "string" },
                { "type": "number" }
            ],
            "items": false,
            "minItems": 2,
            "maxItems": 2
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn unique_items_finite_item_domain_can_force_contains_lower_bound() {
        let old = resolve(json!({
            "type": "array",
            "contains": { "const": "a" },
            "minContains": 1
        }));
        let new = resolve(json!({
            "type": "array",
            "items": { "enum": ["a", "b"] },
            "uniqueItems": true,
            "minItems": 2
        }));

        assert!(is_subschema_of(&new, &old));
        assert!(explain_subschema_failure(&new, &old).is_none());
    }

    #[test]
    fn unique_items_finite_tail_domain_with_prefix_can_force_contains_lower_bound() {
        let old = resolve(json!({
            "type": "array",
            "contains": { "enum": [2, 3] },
            "minContains": 1
        }));
        let new = resolve(json!({
            "type": "array",
            "prefixItems": [{ "const": "header" }],
            "items": { "enum": [1, 2, 3] },
            "uniqueItems": true,
            "minItems": 3
        }));

        assert!(is_subschema_of(&new, &old));
        assert!(explain_subschema_failure(&new, &old).is_none());
    }

    #[test]
    fn unique_nested_array_domain_filters_contains_candidates() {
        let old = resolve(json!({
            "type": "array",
            "maxItems": 1
        }));
        let new = resolve(json!({
            "type": "array",
            "items": {
                "type": "array",
                "items": { "const": 0 },
                "maxItems": 1,
                "contains": { "const": 0 },
                "minContains": 1
            },
            "uniqueItems": true
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn unique_nested_tuple_item_domain_infers_false_tail_capacity() {
        let old = resolve(json!({
            "type": "array",
            "maxItems": 2
        }));
        let new = resolve(json!({
            "type": "array",
            "items": {
                "type": "array",
                "prefixItems": [{ "const": 0 }],
                "items": false
            },
            "uniqueItems": true
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn unique_nested_small_array_item_domain_has_finite_capacity() {
        let old = resolve(json!({
            "type": "array",
            "maxItems": 2
        }));
        let new = resolve(json!({
            "type": "array",
            "items": {
                "type": "array",
                "items": { "const": 0 },
                "maxItems": 1
            },
            "uniqueItems": true
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn unique_required_prefix_consumes_tail_match_for_max_contains() {
        let old = resolve(json!({
            "type": "array",
            "contains": { "const": "x" },
            "maxContains": 1
        }));
        let new = resolve(json!({
            "type": "array",
            "prefixItems": [{ "const": "x" }],
            "items": { "enum": ["x", "y"] },
            "uniqueItems": true,
            "minItems": 1
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn unique_required_prefix_consumes_tail_nonmatch_for_contains() {
        let old = resolve(json!({
            "type": "array",
            "contains": { "const": "y" },
            "minContains": 1
        }));
        let new = resolve(json!({
            "type": "array",
            "prefixItems": [{ "const": "x" }],
            "items": { "enum": ["x", "y"] },
            "uniqueItems": true,
            "minItems": 2
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn unique_items_finite_item_domain_does_not_force_contains_when_room_to_avoid() {
        let old = resolve(json!({
            "type": "array",
            "contains": { "const": "a" },
            "minContains": 1
        }));
        let new = resolve(json!({
            "type": "array",
            "items": { "enum": ["a", "b"] },
            "uniqueItems": true,
            "minItems": 1
        }));

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn unique_items_finite_item_domain_disjoint_contains_is_impossible() {
        let old = resolve(json!({
            "type": "array",
            "maxItems": 0
        }));
        let new = resolve(json!({
            "type": "array",
            "items": { "enum": [1, 2] },
            "uniqueItems": true,
            "contains": { "const": 3 },
            "minContains": 1
        }));

        assert!(is_subschema_of(&new, &old));
        assert!(explain_subschema_failure(&new, &old).is_none());
    }

    #[test]
    fn unique_items_inferred_max_can_make_broad_min_contains_impossible() {
        let old = resolve(json!({
            "type": "array",
            "maxItems": 0
        }));
        let new = resolve(json!({
            "type": "array",
            "items": { "enum": [1, 2] },
            "uniqueItems": true,
            "contains": true,
            "minContains": 3
        }));

        assert!(is_subschema_of(&new, &old));
        assert!(explain_subschema_failure(&new, &old).is_none());
    }

    #[test]
    fn unique_items_finite_contains_domain_can_make_min_contains_impossible() {
        let old = resolve(json!({
            "type": "array",
            "maxItems": 0
        }));
        let new = resolve(json!({
            "type": "array",
            "uniqueItems": true,
            "contains": { "enum": [1, 2] },
            "minContains": 3
        }));

        assert!(is_subschema_of(&new, &old));
        assert!(explain_subschema_failure(&new, &old).is_none());
    }

    #[test]
    fn unique_items_finite_item_domain_satisfies_max_items() {
        let old = resolve(json!({
            "type": "array",
            "maxItems": 2
        }));
        let new = resolve(json!({
            "type": "array",
            "items": { "enum": ["a", "b"] },
            "uniqueItems": true
        }));

        assert!(is_subschema_of(&new, &old));
        assert!(explain_subschema_failure(&new, &old).is_none());
    }

    #[test]
    fn unique_items_finite_tail_domain_with_prefix_satisfies_max_items() {
        let old = resolve(json!({
            "type": "array",
            "maxItems": 3
        }));
        let new = resolve(json!({
            "type": "array",
            "prefixItems": [{ "type": "string" }],
            "items": { "enum": [1, 2] },
            "uniqueItems": true
        }));

        assert!(is_subschema_of(&new, &old));
        assert!(explain_subschema_failure(&new, &old).is_none());
    }

    #[test]
    fn unique_items_finite_item_domain_can_make_min_items_impossible() {
        let old = resolve(json!({
            "type": "array",
            "maxItems": 0
        }));
        let new = resolve(json!({
            "type": "array",
            "items": { "enum": [1, 2] },
            "uniqueItems": true,
            "minItems": 3
        }));

        assert!(is_subschema_of(&new, &old));
        assert!(explain_subschema_failure(&new, &old).is_none());
    }

    #[test]
    fn unique_items_finite_tail_domain_with_prefix_can_make_min_items_impossible() {
        let old = resolve(json!({
            "type": "array",
            "maxItems": 0
        }));
        let new = resolve(json!({
            "type": "array",
            "prefixItems": [{ "type": "string" }],
            "items": { "enum": [1, 2] },
            "uniqueItems": true,
            "minItems": 4
        }));

        assert!(is_subschema_of(&new, &old));
        assert!(explain_subschema_failure(&new, &old).is_none());
    }

    #[test]
    fn unique_items_bounds_max_contains_for_finite_contains_domain() {
        let old = resolve(json!({
            "type": "array",
            "contains": { "enum": [1, 2] },
            "minContains": 0,
            "maxContains": 2
        }));
        let new = resolve(json!({
            "type": "array",
            "items": { "enum": [1, 2, 3, 4] },
            "uniqueItems": true
        }));

        assert!(is_subschema_of(&new, &old));
        assert!(explain_subschema_failure(&new, &old).is_none());
    }

    #[test]
    fn unique_items_does_not_bound_max_contains_past_finite_domain_size() {
        let old = resolve(json!({
            "type": "array",
            "contains": { "enum": [1, 2, 3] },
            "minContains": 0,
            "maxContains": 2
        }));
        let new = resolve(json!({
            "type": "array",
            "items": { "enum": [1, 2, 3, 4] },
            "uniqueItems": true
        }));

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn object_count_disjoint_oneof_can_partition_objects() {
        let sub = resolve(json!({
            "type": "object",
            "maxProperties": 1
        }));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "object", "maxProperties": 1 },
                { "type": "object", "minProperties": 2 }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn overlapping_object_count_oneof_stays_conservative() {
        let sub = resolve(json!({
            "type": "object",
            "maxProperties": 2
        }));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "object", "maxProperties": 2 },
                { "type": "object", "minProperties": 2 }
            ]
        }));

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn closed_declared_names_imply_object_count_partition() {
        let sub = resolve(json!({
            "type": "object",
            "properties": { "a": true, "b": true },
            "additionalProperties": false
        }));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "object", "properties": { "a": true, "b": true }, "additionalProperties": false },
                { "type": "object", "minProperties": 3 }
            ]
        }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn finite_property_names_imply_object_count_partition() {
        let sub = resolve(json!({
            "type": "object",
            "propertyNames": { "enum": ["a", "b"] }
        }));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "object", "propertyNames": { "enum": ["a", "b"] } },
                { "type": "object", "minProperties": 3 }
            ]
        }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn array_length_disjoint_oneof_can_partition_arrays() {
        let sub = resolve(json!({
            "type": "array",
            "maxItems": 1
        }));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "array", "maxItems": 1 },
                { "type": "array", "minItems": 2 }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn contains_minimum_implies_array_length_partition() {
        let sub = resolve(json!({
            "type": "array",
            "contains": {},
            "minContains": 2
        }));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "array", "contains": {}, "minContains": 2 },
                { "type": "array", "maxItems": 1 }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn universal_contains_maximum_implies_array_length_partition() {
        let sub = resolve(json!({
            "type": "array",
            "contains": {},
            "maxContains": 1
        }));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "array", "contains": {}, "maxContains": 1 },
                { "type": "array", "minItems": 2 }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn false_tail_implies_array_length_partition() {
        let sub = resolve(json!({
            "type": "array",
            "prefixItems": [{ "const": 1 }, { "const": 2 }],
            "items": false
        }));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "array", "prefixItems": [{ "const": 1 }, { "const": 2 }], "items": false },
                { "type": "array", "minItems": 3 }
            ]
        }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn unique_finite_items_imply_array_length_partition() {
        let sub = resolve(json!({
            "type": "array",
            "uniqueItems": true,
            "items": { "enum": [1, 2] }
        }));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "array", "uniqueItems": true, "items": { "enum": [1, 2] } },
                { "type": "array", "minItems": 3 }
            ]
        }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn tuple_item_type_disjoint_oneof_can_partition_arrays() {
        let sub = resolve(json!({
            "type": "array",
            "minItems": 1,
            "prefixItems": [{ "type": "string" }]
        }));
        let sup = resolve(json!({
            "oneOf": [
                {
                    "type": "array",
                    "minItems": 1,
                    "prefixItems": [{ "type": "string" }]
                },
                {
                    "type": "array",
                    "minItems": 1,
                    "prefixItems": [{ "type": "integer" }]
                }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn tuple_item_const_disjoint_oneof_can_partition_arrays() {
        let sub = resolve(json!({
            "type": "array",
            "minItems": 1,
            "prefixItems": [{ "const": "left" }]
        }));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "array", "minItems": 1, "prefixItems": [{ "const": "left" }] },
                { "type": "array", "minItems": 1, "prefixItems": [{ "const": "right" }] }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn finite_tuple_item_rejected_by_pattern_can_partition_oneof() {
        let sub = resolve(json!({
            "type": "array",
            "minItems": 1,
            "prefixItems": [{ "const": "ok" }]
        }));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "array", "minItems": 1, "prefixItems": [{ "const": "ok" }] },
                { "type": "array", "minItems": 1, "prefixItems": [{ "type": "string", "pattern": "^err$" }] }
            ]
        }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditional_tuple_item_shape_can_partition_oneof() {
        let conditional = json!({
            "if": { "type": "array" },
            "then": {
                "type": "array",
                "minItems": 1,
                "prefixItems": [{ "type": "string" }]
            }
        });
        let sub = resolve(conditional.clone());
        let sup = resolve(json!({
            "oneOf": [
                conditional,
                { "type": "array", "minItems": 1, "prefixItems": [{ "type": "integer" }] }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditional_tuple_item_rejection_can_partition_oneof() {
        let sub = resolve(json!({
            "type": "array",
            "minItems": 1,
            "prefixItems": [{ "const": "tag-a" }]
        }));
        let conditional_other = json!({
            "if": { "type": "array" },
            "then": {
                "type": "array",
                "minItems": 1,
                "prefixItems": [{ "const": "tag-b" }]
            }
        });
        let sup = resolve(json!({
            "oneOf": [
                { "type": "array", "minItems": 1, "prefixItems": [{ "const": "tag-a" }] },
                conditional_other
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditional_tuple_item_values_can_partition_oneof() {
        let conditional = json!({
            "if": { "type": "array" },
            "then": {
                "type": "array",
                "minItems": 1,
                "prefixItems": [{ "const": "tag-a" }]
            }
        });
        let sub = resolve(conditional.clone());
        let sup = resolve(json!({
            "oneOf": [
                conditional,
                {
                    "type": "array",
                    "minItems": 1,
                    "prefixItems": [{ "type": "string", "pattern": "^tag-b$" }]
                }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn optional_tuple_item_partition_stays_conservative() {
        let sub = resolve(json!({
            "type": "array",
            "prefixItems": [{ "type": "string" }]
        }));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "array", "prefixItems": [{ "type": "string" }] },
                { "type": "array", "prefixItems": [{ "type": "integer" }] }
            ]
        }));

        // The empty array matches both branches, so the tuple discriminator is
        // only sound when both sides force the discriminating position to exist.
        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn overlapping_array_length_oneof_stays_conservative() {
        let sub = resolve(json!({
            "type": "array",
            "maxItems": 2
        }));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "array", "maxItems": 2 },
                { "type": "array", "minItems": 2 }
            ]
        }));

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn literal_string_and_array_lengths_have_intervals() {
        let string_const = resolve(json!({ "const": "hé" }));
        let string_interval =
            string_length_interval_bound(&string_const).expect("string const interval");
        assert_eq!(string_interval.lower, 2);
        assert_eq!(string_interval.upper, Some(2));

        let string_enum = resolve(json!({ "enum": ["a", "abcd", 7] }));
        let string_interval =
            string_length_interval_bound(&string_enum).expect("string enum interval");
        assert_eq!(string_interval.lower, 1);
        assert_eq!(string_interval.upper, Some(4));

        let array_const = resolve(json!({ "const": [1, 2, 3] }));
        let array_interval =
            array_length_interval_bound(&array_const).expect("array const interval");
        assert_eq!(array_interval.lower, 3);
        assert_eq!(array_interval.upper, Some(3));

        let array_enum = resolve(json!({ "enum": [[], [1, 2], "skip"] }));
        let array_interval = array_length_interval_bound(&array_enum).expect("array enum interval");
        assert_eq!(array_interval.lower, 0);
        assert_eq!(array_interval.upper, Some(2));
    }

    #[test]
    fn string_length_disjoint_oneof_can_partition_strings() {
        let sub = resolve(json!({
            "type": "string",
            "maxLength": 2
        }));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "string", "maxLength": 2 },
                { "type": "string", "minLength": 3 }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn union_wrapped_string_length_partition_is_disjoint() {
        let sub = resolve(json!({
            "type": "string",
            "maxLength": 1
        }));
        let sup = resolve(json!({
            "oneOf": [
                {
                    "anyOf": [
                        { "type": "string", "maxLength": 1 },
                        { "type": "string", "minLength": 2, "maxLength": 2 }
                    ]
                },
                { "type": "string", "minLength": 3 }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn union_wrapped_numeric_range_partition_is_disjoint() {
        let sub = resolve(json!({
            "type": "number",
            "maximum": 1
        }));
        let sup = resolve(json!({
            "oneOf": [
                {
                    "anyOf": [
                        { "type": "number", "maximum": 1 },
                        { "type": "number", "minimum": 2, "maximum": 2 }
                    ]
                },
                { "type": "number", "minimum": 3 }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn overlapping_string_length_oneof_stays_conservative() {
        let sub = resolve(json!({
            "type": "string",
            "maxLength": 3
        }));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "string", "maxLength": 3 },
                { "type": "string", "minLength": 3 }
            ]
        }));

        // Length 3 strings hit both branches, so the oneOf is not a superset.
        assert!(!is_subschema_of(&sub, &sup));
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
    fn finite_enum_can_be_split_across_anyof_literals() {
        let sub = resolve(json!({ "enum": ["red", "blue"] }));
        let sup = resolve(json!({
            "anyOf": [
                { "const": "red" },
                { "const": "blue" }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn finite_anyof_split_requires_every_possible_value() {
        let sub = resolve(json!({ "enum": ["red", "blue"] }));
        let sup = resolve(json!({
            "anyOf": [
                { "const": "red" }
            ]
        }));

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn obvious_anyof_type_cover_accepts_unconstrained_schema() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "type": "null" },
                { "type": "boolean" },
                { "type": "number" },
                { "type": "string" },
                { "type": "array" },
                { "type": "object" }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn incomplete_anyof_type_cover_stays_conservative() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "type": "null" },
                { "type": "boolean" },
                { "type": "number" },
                { "type": "string" },
                { "type": "array" }
            ]
        }));

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn dependent_required_implied_finite_tag_can_partition_oneof() {
        let sub = resolve(json!({
            "type": "object",
            "required": ["trigger"],
            "dependentRequired": { "trigger": ["tag"] },
            "properties": { "tag": { "const": "left" } }
        }));
        let sup = resolve(json!({
            "oneOf": [
                {
                    "type": "object",
                    "required": ["trigger"],
                    "dependentRequired": { "trigger": ["tag"] },
                    "properties": { "tag": { "const": "left" } }
                },
                {
                    "type": "object",
                    "required": ["trigger"],
                    "dependentRequired": { "trigger": ["tag"] },
                    "properties": { "tag": { "const": "right" } }
                }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn dependent_required_implied_count_disjoints_small_object_branch() {
        let sub = resolve(json!({
            "type": "object",
            "required": ["a"],
            "dependentRequired": {"a": ["b"]}
        }));
        let sup = resolve(json!({
            "oneOf": [
                {"type": "object", "required": ["a"], "dependentRequired": {"a": ["b"]}},
                {"type": "object", "maxProperties": 1}
            ]
        }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn max_properties_zero_forbids_implied_required_name_partition() {
        let sub = resolve(json!({
            "type": "object",
            "required": ["trigger"],
            "dependentRequired": {"trigger": ["tag"]}
        }));
        let sup = resolve(json!({
            "oneOf": [
                {"type": "object", "required": ["trigger"], "dependentRequired": {"trigger": ["tag"]}},
                {"type": "object", "maxProperties": 0}
            ]
        }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn negated_required_property_can_partition_oneof() {
        let sub = resolve(json!({
            "type": "object",
            "required": ["present"]
        }));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "object", "required": ["present"] },
                { "type": "object", "not": { "required": ["present"] } }
            ]
        }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn split_allof_dependent_required_implied_tag_shape_can_partition_oneof() {
        let left = json!({"allOf": [
            {"type": "object", "required": ["trigger"]},
            {"type": "object", "dependentRequired": {"trigger": ["tag"]}},
            {"type": "object", "properties": {"tag": {"type": "string"}}}
        ]});
        let right = json!({"allOf": [
            {"type": "object", "required": ["trigger"]},
            {"type": "object", "dependentRequired": {"trigger": ["tag"]}},
            {"type": "object", "properties": {"tag": {"type": "number"}}}
        ]});
        let sub = resolve(left.clone());
        let sup = resolve(json!({"oneOf": [left, right]}));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn dependent_required_implied_tag_shape_can_partition_oneof() {
        let sub = resolve(json!({
            "type": "object",
            "required": ["trigger"],
            "dependentRequired": { "trigger": ["tag"] },
            "properties": { "tag": { "type": "string" } }
        }));
        let sup = resolve(json!({
            "oneOf": [
                {
                    "type": "object",
                    "required": ["trigger"],
                    "dependentRequired": { "trigger": ["tag"] },
                    "properties": { "tag": { "type": "string" } }
                },
                {
                    "type": "object",
                    "required": ["trigger"],
                    "dependentRequired": { "trigger": ["tag"] },
                    "properties": { "tag": { "type": "number" } }
                }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn dependent_required_implied_property_can_partition_oneof() {
        let sub = resolve(json!({
            "type": "object",
            "required": ["a"],
            "dependentRequired": { "a": ["b"] },
            "properties": { "a": true, "b": { "type": "string" } }
        }));
        let sup = resolve(json!({
            "oneOf": [
                {
                    "type": "object",
                    "required": ["a"],
                    "dependentRequired": { "a": ["b"] },
                    "properties": { "a": true, "b": { "type": "string" } }
                },
                { "type": "object", "properties": { "b": false } }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn optional_dependent_required_trigger_does_not_partition_oneof() {
        let sub = resolve(json!({
            "type": "object",
            "dependentRequired": { "a": ["b"] }
        }));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "object", "dependentRequired": { "a": ["b"] } },
                { "type": "object", "properties": { "b": false } }
            ]
        }));

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn required_property_forbidden_by_closed_other_branch_is_disjoint() {
        let sub = resolve(json!({
            "type": "object",
            "required": ["a"],
            "properties": { "a": { "type": "string" } },
            "additionalProperties": false
        }));
        let sup = resolve(json!({
            "oneOf": [
                {
                    "type": "object",
                    "required": ["a"],
                    "properties": { "a": { "type": "string" } },
                    "additionalProperties": false
                },
                {
                    "type": "object",
                    "required": ["b"],
                    "properties": { "b": { "type": "string" } },
                    "additionalProperties": false
                }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn required_property_forbidden_by_property_names_other_branch_is_disjoint() {
        let sub = resolve(json!({
            "type": "object",
            "required": ["a"],
            "properties": { "a": { "type": "string" } }
        }));
        let sup = resolve(json!({
            "oneOf": [
                {
                    "type": "object",
                    "required": ["a"],
                    "properties": { "a": { "type": "string" } }
                },
                {
                    "type": "object",
                    "required": ["b"],
                    "propertyNames": { "enum": ["b"] },
                    "properties": { "b": { "type": "string" } }
                }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn required_property_forbidden_in_other_oneof_branch_is_disjoint() {
        let sub = resolve(json!({
            "type": "object",
            "required": ["a"],
            "properties": {
                "a": { "type": "string" },
                "b": false
            }
        }));
        let sup = resolve(json!({
            "oneOf": [
                {
                    "type": "object",
                    "required": ["a"],
                    "properties": { "a": { "type": "string" }, "b": false }
                },
                {
                    "type": "object",
                    "required": ["b"],
                    "properties": { "b": { "type": "string" }, "a": false }
                }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditional_guard_narrows_possible_type_mask_for_oneof() {
        let sub = resolve(json!({
            "if": { "type": "string" },
            "then": true,
            "else": { "type": "number" }
        }));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "string" },
                { "type": "number" }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn complement_type_oneof_branch_is_disjoint_by_mask() {
        let sub = resolve(json!({ "type": "string", "minLength": 1 }));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "string" },
                { "not": { "type": "string" } }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn complement_null_oneof_branch_is_disjoint_by_mask() {
        let sub = resolve(json!({ "type": "string" }));
        let sup = resolve(json!({
            "oneOf": [
                { "not": { "const": null } },
                { "const": null }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn type_disjoint_oneof_can_be_a_superset_of_string_schema() {
        let sub = resolve(json!({
            "type": "string",
            "minLength": 2
        }));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "string" },
                { "type": "integer" },
                { "type": "null" }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn finite_enum_can_be_split_across_disjoint_oneof_literals() {
        let sub = resolve(json!({ "enum": ["red", "blue"] }));
        let sup = resolve(json!({
            "oneOf": [
                { "const": "red" },
                { "const": "blue" }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn finite_enum_overlapping_oneof_branch_stays_conservative() {
        let sub = resolve(json!({ "enum": ["red"] }));
        let sup = resolve(json!({
            "oneOf": [
                { "enum": ["red", "blue"] },
                { "const": "red" }
            ]
        }));

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn finite_disjoint_oneof_literals_can_partition_same_type() {
        let sub = resolve(json!({ "const": "red" }));
        let sup = resolve(json!({
            "oneOf": [
                { "const": "red" },
                { "const": "blue" }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditional_forbidden_property_can_partition_required_branch() {
        let sub = resolve(json!({
            "type": "object",
            "required": ["flag"]
        }));
        let conditional_other = json!({
            "if": { "type": "object" },
            "then": {
                "type": "object",
                "properties": { "flag": false }
            }
        });
        let sup = resolve(json!({
            "oneOf": [
                { "type": "object", "required": ["flag"] },
                conditional_other
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditional_object_property_rejection_can_partition_other_branch() {
        let sub = resolve(json!({
            "type": "object",
            "required": ["kind"],
            "properties": { "kind": { "const": "a" } }
        }));
        let conditional_other = json!({
            "if": { "type": "object" },
            "then": {
                "type": "object",
                "required": ["kind"],
                "properties": { "kind": { "const": "b" } }
            }
        });
        let sup = resolve(json!({
            "oneOf": [
                { "type": "object", "required": ["kind"], "properties": { "kind": { "const": "a" } } },
                conditional_other
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditional_object_property_shape_can_partition_other_branch() {
        let conditional = json!({
            "if": { "type": "object" },
            "then": {
                "type": "object",
                "required": ["kind"],
                "properties": { "kind": { "type": "string" } }
            }
        });
        let sub = resolve(conditional.clone());
        let sup = resolve(json!({
            "oneOf": [
                conditional,
                {
                    "type": "object",
                    "required": ["kind"],
                    "properties": { "kind": { "type": "integer" } }
                }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditional_then_only_object_tag_can_partition_other_branch() {
        let conditional = json!({
            "if": { "type": "object" },
            "then": {
                "type": "object",
                "required": ["kind"],
                "properties": { "kind": { "const": "a" } }
            }
        });
        let sub = resolve(conditional.clone());
        let sup = resolve(json!({
            "oneOf": [
                conditional,
                {
                    "type": "object",
                    "required": ["kind"],
                    "properties": { "kind": { "const": "b" } }
                }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditional_object_tag_values_can_partition_other_branch() {
        let conditional = json!({
            "if": { "type": "object", "required": ["kind"] },
            "then": {
                "type": "object",
                "required": ["kind"],
                "properties": { "kind": { "const": "a" } }
            },
            "else": {
                "type": "object",
                "required": ["kind"],
                "properties": { "kind": { "const": "b" } }
            }
        });
        let sub = resolve(conditional.clone());
        let sup = resolve(json!({
            "oneOf": [
                conditional,
                {
                    "type": "object",
                    "required": ["kind"],
                    "properties": { "kind": { "const": "c" } }
                }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditional_then_only_guard_all_strings_bounds_string_partition() {
        let conditional = json!({
            "if": { "type": "string" },
            "then": { "type": "string", "maxLength": 2 }
        });
        let sub = resolve(conditional.clone());
        let sup = resolve(json!({
            "oneOf": [
                conditional,
                { "type": "string", "minLength": 8 }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditional_else_only_impossible_string_guard_bounds_partition() {
        let conditional = json!({
            "if": { "type": "number" },
            "else": { "type": "string", "maxLength": 2 }
        });
        let sub = resolve(conditional.clone());
        let sup = resolve(json!({
            "oneOf": [
                conditional,
                { "type": "string", "minLength": 8 }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditional_string_branch_hull_can_partition_length() {
        let conditional = json!({
            "if": { "type": "string", "maxLength": 2 },
            "then": { "type": "string", "maxLength": 2 },
            "else": { "type": "string", "minLength": 4, "maxLength": 5 }
        });
        let sub = resolve(conditional.clone());
        let sup = resolve(json!({
            "oneOf": [
                conditional,
                { "type": "string", "minLength": 8 }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditional_array_branch_hull_can_partition_length() {
        let conditional = json!({
            "if": { "type": "array", "maxItems": 1 },
            "then": { "type": "array", "maxItems": 1 },
            "else": { "type": "array", "minItems": 3, "maxItems": 4 }
        });
        let sub = resolve(conditional.clone());
        let sup = resolve(json!({
            "oneOf": [
                conditional,
                { "type": "array", "minItems": 8 }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditional_object_branch_hull_can_partition_count() {
        let conditional = json!({
            "if": { "type": "object", "maxProperties": 1 },
            "then": { "type": "object", "maxProperties": 1 },
            "else": { "type": "object", "minProperties": 3, "maxProperties": 4 }
        });
        let sub = resolve(conditional.clone());
        let sup = resolve(json!({
            "oneOf": [
                conditional,
                { "type": "object", "minProperties": 8 }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditional_numeric_branch_hull_can_partition_range() {
        let conditional = json!({
            "if": { "type": "number", "maximum": 0 },
            "then": { "type": "number", "maximum": 0 },
            "else": { "type": "number", "minimum": 10, "maximum": 20 }
        });
        let sub = resolve(conditional.clone());
        let sup = resolve(json!({
            "oneOf": [
                conditional,
                { "type": "number", "minimum": 30 }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn numeric_literal_interval_can_partition_range_branch() {
        let sub = resolve(json!({ "const": 7 }));
        let sup = resolve(json!({
            "oneOf": [
                { "const": 7 },
                { "type": "number", "maximum": 0 }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn numeric_enum_interval_can_partition_range_branch() {
        let sub = resolve(json!({ "enum": [2, 3] }));
        let sup = resolve(json!({
            "oneOf": [
                { "enum": [2, 3] },
                { "type": "number", "minimum": 10 }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn numeric_range_disjoint_oneof_can_be_a_superset_of_bounded_integer_schema() {
        let sub = resolve(json!({
            "type": "integer",
            "minimum": 2,
            "maximum": 4
        }));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "integer", "minimum": 0, "maximum": 10 },
                { "type": "integer", "minimum": 11, "maximum": 20 }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn split_allof_numeric_range_can_imply_plain_number_range() {
        let sub = resolve(json!({
            "allOf": [
                { "type": "number", "minimum": 1 },
                { "type": "number", "exclusiveMaximum": 5 }
            ]
        }));
        let sup = resolve(json!({
            "type": "number",
            "minimum": 0,
            "maximum": 5
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn split_allof_integer_range_can_imply_plain_integer_range() {
        let sub = resolve(json!({
            "allOf": [
                { "type": "integer", "minimum": 2 },
                { "type": "integer", "maximum": 4 }
            ]
        }));
        let sup = resolve(json!({
            "type": "integer",
            "minimum": 1,
            "maximum": 5
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn split_allof_number_range_does_not_imply_integer_range() {
        let sub = resolve(json!({
            "allOf": [
                { "type": "number", "minimum": 2 },
                { "type": "number", "maximum": 4 }
            ]
        }));
        let sup = resolve(json!({
            "type": "integer",
            "minimum": 1,
            "maximum": 5
        }));

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn split_allof_numeric_bounds_without_type_stay_conservative() {
        let sub = resolve(json!({
            "allOf": [
                { "minimum": 1 },
                { "maximum": 3 }
            ]
        }));
        let sup = resolve(json!({
            "type": "number",
            "minimum": 0,
            "maximum": 5
        }));

        // Numeric keywords alone also admit strings/objects in JSON Schema.
        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn split_allof_string_length_can_imply_plain_string_range() {
        let sub = resolve(json!({
            "allOf": [
                { "type": "string", "minLength": 2 },
                { "type": "string", "maxLength": 4 }
            ]
        }));
        let sup = resolve(json!({
            "type": "string",
            "minLength": 1,
            "maxLength": 5
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn split_allof_array_length_can_imply_plain_array_range() {
        let sub = resolve(json!({
            "allOf": [
                { "type": "array", "minItems": 2 },
                { "type": "array", "maxItems": 4 }
            ]
        }));
        let sup = resolve(json!({
            "type": "array",
            "minItems": 1,
            "maxItems": 5
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn split_allof_object_count_can_imply_plain_object_range() {
        let sub = resolve(json!({
            "allOf": [
                { "type": "object", "minProperties": 2 },
                { "type": "object", "maxProperties": 4 }
            ]
        }));
        let sup = resolve(json!({
            "type": "object",
            "minProperties": 1,
            "maxProperties": 5
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn split_allof_length_bounds_without_type_stay_conservative() {
        let sub = resolve(json!({
            "allOf": [
                { "minLength": 2 },
                { "maxLength": 4 }
            ]
        }));
        let sup = resolve(json!({
            "type": "string",
            "minLength": 1,
            "maxLength": 5
        }));

        // Length keywords alone also admit non-strings in JSON Schema.
        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn discriminator_disjoint_oneof_can_be_a_superset_of_object_schema() {
        let sub = resolve(json!({
            "type": "object",
            "required": ["kind", "name"],
            "properties": {
                "kind": { "const": "cat" },
                "name": { "type": "string" }
            }
        }));
        let sup = resolve(json!({
            "oneOf": [
                {
                    "type": "object",
                    "required": ["kind"],
                    "properties": { "kind": { "const": "cat" } }
                },
                {
                    "type": "object",
                    "required": ["kind"],
                    "properties": { "kind": { "const": "dog" } }
                }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn pattern_properties_shape_can_partition_required_property() {
        let sub = resolve(json!({
            "type": "object",
            "required": ["tag"],
            "patternProperties": { "^tag$": { "type": "string" } }
        }));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "object", "required": ["tag"], "patternProperties": { "^tag$": { "type": "string" } } },
                { "type": "object", "required": ["tag"], "patternProperties": { "^tag$": { "type": "integer" } } }
            ]
        }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn branchwise_wrapped_range_partition_is_disjoint() {
        let sub = resolve(json!({ "type": "number", "maximum": 0 }));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "number", "maximum": 0 },
                { "anyOf": [
                    { "type": "number", "minimum": 1 },
                    { "type": "string" }
                ] }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn explicit_not_guard_can_partition_oneof() {
        let sub = resolve(json!({
            "type": "object",
            "required": ["kind"],
            "properties": { "kind": { "const": "cat" } }
        }));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "object", "required": ["kind"], "properties": { "kind": { "const": "cat" } } },
                { "allOf": [
                    { "not": { "type": "object", "required": ["kind"], "properties": { "kind": { "const": "cat" } } } },
                    { "type": "object" }
                ] }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn saturated_required_slots_forbid_extra_property_partition() {
        let sub = resolve(json!({
            "type": "object",
            "required": ["a"],
            "maxProperties": 1
        }));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "object", "required": ["a"], "maxProperties": 1 },
                { "type": "object", "required": ["b"], "maxProperties": 1 }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn enum_without_name_forbids_required_property_partition() {
        let sub = resolve(json!({
            "type": "object",
            "required": ["tag"],
            "properties": { "tag": { "type": "string" } }
        }));
        let sup = resolve(json!({
            "oneOf": [
                {
                    "type": "object",
                    "required": ["tag"],
                    "properties": { "tag": { "type": "string" } }
                },
                { "enum": [{}, {"other": 1}, null] }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn matching_false_pattern_forbids_required_property_partition() {
        let sub = resolve(json!({
            "type": "object",
            "required": ["x_tag"],
            "properties": { "x_tag": { "type": "string" } }
        }));
        let sup = resolve(json!({
            "oneOf": [
                {
                    "type": "object",
                    "required": ["x_tag"],
                    "properties": { "x_tag": { "type": "string" } }
                },
                {
                    "type": "object",
                    "patternProperties": { "^x_": false }
                }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn property_type_disjoint_oneof_can_partition_objects() {
        let sub = resolve(json!({
            "type": "object",
            "required": ["tag"],
            "properties": { "tag": { "type": "string" } }
        }));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "object", "required": ["tag"], "properties": { "tag": { "type": "string" } } },
                { "type": "object", "required": ["tag"], "properties": { "tag": { "type": "integer" } } }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn property_length_disjoint_oneof_can_partition_objects() {
        let sub = resolve(json!({
            "type": "object",
            "required": ["code"],
            "properties": { "code": { "type": "string", "maxLength": 2 } }
        }));
        let sup = resolve(json!({
            "oneOf": [
                {
                    "type": "object",
                    "required": ["code"],
                    "properties": { "code": { "type": "string", "maxLength": 2 } }
                },
                {
                    "type": "object",
                    "required": ["code"],
                    "properties": { "code": { "type": "string", "minLength": 3 } }
                }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn finite_property_value_rejected_by_pattern_can_partition_oneof() {
        let sub = resolve(json!({
            "type": "object",
            "required": ["code"],
            "properties": { "code": { "const": "ok" } }
        }));
        let sup = resolve(json!({
            "oneOf": [
                {
                    "type": "object",
                    "required": ["code"],
                    "properties": { "code": { "const": "ok" } }
                },
                {
                    "type": "object",
                    "required": ["code"],
                    "properties": { "code": { "type": "string", "pattern": "^err$" } }
                }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn overlapping_property_length_partition_stays_conservative() {
        let sub = resolve(json!({
            "type": "object",
            "required": ["code"],
            "properties": { "code": { "type": "string", "maxLength": 3 } }
        }));
        let sup = resolve(json!({
            "oneOf": [
                {
                    "type": "object",
                    "required": ["code"],
                    "properties": { "code": { "type": "string", "maxLength": 3 } }
                },
                {
                    "type": "object",
                    "required": ["code"],
                    "properties": { "code": { "type": "string", "minLength": 3 } }
                }
            ]
        }));

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn split_allof_overlapping_property_partition_stays_conservative() {
        let sub = resolve(json!({
            "allOf": [
                { "type": "object", "required": ["tag"] },
                { "type": "object", "properties": { "tag": { "type": "string" } } }
            ]
        }));
        let sup = resolve(json!({
            "oneOf": [
                { "allOf": [
                    { "type": "object", "required": ["tag"] },
                    { "type": "object", "properties": { "tag": { "type": "string" } } }
                ] },
                { "allOf": [
                    { "type": "object", "required": ["tag"] },
                    { "type": "object", "properties": { "tag": { "minLength": 1 } } }
                ] }
            ]
        }));

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn split_allof_required_property_type_disjoint_partition() {
        let sub = resolve(json!({
            "allOf": [
                { "type": "object", "required": ["tag"] },
                { "type": "object", "properties": { "tag": { "type": "string" } } }
            ]
        }));
        let sup = resolve(json!({
            "oneOf": [
                { "allOf": [
                    { "type": "object", "required": ["tag"] },
                    { "type": "object", "properties": { "tag": { "type": "string" } } }
                ] },
                { "allOf": [
                    { "type": "object", "required": ["tag"] },
                    { "type": "object", "properties": { "tag": { "type": "integer" } } }
                ] }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn union_wrapped_discriminators_are_still_disjoint_for_oneof() {
        let sub = resolve(json!({
            "type": "object",
            "required": ["kind"],
            "properties": { "kind": { "const": "cat" } }
        }));
        let sup = resolve(json!({
            "oneOf": [
                {
                    "anyOf": [
                        { "type": "object", "required": ["kind"], "properties": { "kind": { "const": "cat" } } },
                        { "type": "object", "required": ["kind"], "properties": { "kind": { "enum": ["cat"] }, "extra": { "type": "string" } } }
                    ]
                },
                { "type": "object", "required": ["kind"], "properties": { "kind": { "const": "dog" } } }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn mixed_object_enum_without_common_discriminator_stays_conservative() {
        let sub = resolve(json!({
            "enum": [
                { "kind": "cat" },
                { "shared": true }
            ]
        }));
        let sup = resolve(json!({
            "oneOf": [
                { "enum": [{ "kind": "cat" }, { "shared": true }] },
                { "enum": [{ "kind": "dog" }, { "shared": true }] }
            ]
        }));

        // The {"shared": true} value matches both branches, so discriminator
        // extraction must not ignore enum objects that lack the discriminator.
        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn type_disjoint_negation_accepts_string_subset() {
        let sub = resolve(json!({ "type": "string", "minLength": 1 }));
        let sup = resolve(json!({ "not": { "type": "number" } }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn finite_negated_values_rejected_by_subset_are_disjoint() {
        let sub = resolve(json!({ "type": "string", "minLength": 2 }));
        let sup = resolve(json!({ "not": { "const": "" } }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn finite_subset_values_can_be_disjoint_from_infinite_negated_schema() {
        let sub = resolve(json!({ "const": "" }));
        let sup = resolve(json!({ "not": { "type": "string", "minLength": 1 } }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn negated_anyof_disjoint_bounds_are_handled_branchwise() {
        let sub = resolve(json!({ "type": "integer", "maximum": 0 }));
        let sup = resolve(json!({
            "not": {
                "anyOf": [
                    { "type": "integer", "minimum": 1 },
                    { "type": "string" }
                ]
            }
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn negation_is_contravariant_for_proved_inner_subset() {
        let sub = resolve(json!({ "not": { "type": "string" } }));
        let sup = resolve(json!({ "not": { "enum": ["blocked"] } }));

        // enum["blocked"] is a subset of string, so complement(string) is a
        // subset of complement(enum["blocked"]).
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn negation_contravariance_does_not_reverse_unproved_inner_subset() {
        let sub = resolve(json!({ "not": { "enum": ["blocked"] } }));
        let sup = resolve(json!({ "not": { "type": "string" } }));

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn overlapping_negation_stays_conservative() {
        let sub = resolve(json!({ "type": "string" }));
        let sup = resolve(json!({ "not": { "type": "string", "minLength": 2 } }));

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn numeric_disjoint_negation_accepts_bounded_subset() {
        let sub = resolve(json!({ "type": "integer", "minimum": 0, "maximum": 4 }));
        let sup = resolve(json!({ "not": { "type": "number", "minimum": 5, "maximum": 10 } }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn discriminator_disjoint_negation_accepts_object_subset() {
        let sub = resolve(json!({
            "type": "object",
            "required": ["kind"],
            "properties": { "kind": { "const": "cat" } }
        }));
        let sup = resolve(json!({
            "not": {
                "type": "object",
                "required": ["kind"],
                "properties": { "kind": { "const": "dog" } }
            }
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn overlapping_oneof_branch_stays_conservative_for_nonfinite_schema() {
        let sub = resolve(json!({ "type": "string" }));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "string" },
                { "anyOf": [{ "type": "string" }, { "type": "integer" }] }
            ]
        }));

        // Every string would match both branches, so this is not a superset.
        assert!(!is_subschema_of(&sub, &sup));
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
    fn optional_false_prefix_item_implies_shorter_max_items() {
        let old = resolve(json!({
            "type": "array",
            "maxItems": 1
        }));
        let new = resolve(json!({
            "type": "array",
            "prefixItems": [
                { "const": "ok" },
                false,
                { "type": "string" }
            ]
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
    fn dependent_required_trigger_that_overflows_max_properties_is_vacuous() {
        let old = resolve(json!({
            "type": "object",
            "dependentRequired": {
                "a": ["c"]
            }
        }));
        let new = resolve(json!({
            "type": "object",
            "maxProperties": 1,
            "dependentRequired": {
                "a": ["b"]
            }
        }));

        // In the subset schema, an object containing `a` would also need `b`,
        // which cannot fit under maxProperties: 1. Therefore `a` is not a
        // realizable trigger for the superset dependency.
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn dependent_required_trigger_forcing_rejected_name_is_vacuous() {
        let old = resolve(json!({
            "type": "object",
            "dependentRequired": { "a": ["c"] }
        }));
        let new = resolve(json!({
            "type": "object",
            "propertyNames": { "pattern": "^a$" },
            "dependentRequired": { "a": ["b"] }
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn dependent_required_trigger_forcing_false_property_is_vacuous() {
        let old = resolve(json!({
            "type": "object",
            "dependentRequired": { "a": ["c"] }
        }));
        let new = resolve(json!({
            "type": "object",
            "properties": { "b": false },
            "dependentRequired": { "a": ["b"] }
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn dependent_required_trigger_forcing_dead_enum_property_is_vacuous() {
        let old = resolve(json!({
            "type": "object",
            "dependentRequired": { "a": ["c"] }
        }));
        let new = resolve(json!({
            "type": "object",
            "properties": {
                "b": { "type": "integer", "enum": ["not-an-integer"] }
            },
            "dependentRequired": { "a": ["b"] }
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

    #[test]
    fn impossible_required_false_property_is_vacuous() {
        let impossible = resolve(json!({
            "type": "object",
            "required": ["x"],
            "properties": {
                "x": false
            }
        }));
        let arbitrary_object = resolve(json!({
            "type": "object",
            "required": ["y"]
        }));

        assert!(is_subschema_of(&impossible, &arbitrary_object));
    }

    #[test]
    fn impossible_required_property_with_dead_enum_is_vacuous() {
        let impossible = resolve(json!({
            "type": "object",
            "required": ["x"],
            "properties": {
                "x": { "type": "integer", "enum": ["not-an-integer"] }
            }
        }));
        let arbitrary_object = resolve(json!({
            "type": "object",
            "required": ["y"]
        }));

        assert!(is_subschema_of(&impossible, &arbitrary_object));
    }

    #[test]
    fn impossible_required_name_rejected_by_property_names_is_vacuous() {
        let impossible = resolve(json!({
            "type": "object",
            "required": ["x"],
            "propertyNames": {
                "const": "y"
            }
        }));
        let arbitrary_object = resolve(json!({
            "type": "object",
            "required": ["z"]
        }));

        assert!(is_subschema_of(&impossible, &arbitrary_object));
    }

    #[test]
    fn impossible_min_properties_exceeds_closed_declared_names_is_vacuous() {
        let impossible = resolve(json!({
            "type": "object",
            "minProperties": 2,
            "properties": {
                "a": { "type": "string" }
            },
            "additionalProperties": false
        }));
        let arbitrary_object = resolve(json!({
            "type": "object",
            "required": ["z"]
        }));

        assert!(is_subschema_of(&impossible, &arbitrary_object));
    }

    #[test]
    fn impossible_min_properties_exceeds_finite_property_names_is_vacuous() {
        let impossible = resolve(json!({
            "type": "object",
            "minProperties": 2,
            "propertyNames": { "enum": ["only"] }
        }));
        let arbitrary_object = resolve(json!({
            "type": "object",
            "required": ["z"]
        }));

        assert!(is_subschema_of(&impossible, &arbitrary_object));
    }

    #[test]
    fn impossible_min_properties_excludes_declared_names_with_impossible_dependencies() {
        let impossible = resolve(json!({
            "type": "object",
            "minProperties": 2,
            "properties": {
                "a": true,
                "b": true
            },
            "dependentRequired": {
                "a": ["missing"],
                "b": ["missing"]
            },
            "additionalProperties": false
        }));
        let arbitrary_object = resolve(json!({
            "type": "object",
            "required": ["z"]
        }));

        assert!(is_subschema_of(&impossible, &arbitrary_object));
    }

    #[test]
    fn finite_property_names_avoid_global_additional_properties_requirement() {
        let sub = resolve(json!({
            "type": "object",
            "propertyNames": { "enum": ["a"] },
            "additionalProperties": { "type": "string" }
        }));
        let sup = resolve(json!({
            "type": "object",
            "properties": {
                "a": { "type": "string" }
            },
            "additionalProperties": false
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn dependent_required_from_required_property_guarantees_sup_required_name() {
        let sub = resolve(json!({
            "type": "object",
            "required": ["a"],
            "dependentRequired": { "a": ["b"] },
            "maxProperties": 2
        }));
        let sup = resolve(json!({
            "type": "object",
            "required": ["b"]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn dependent_required_from_required_property_counts_toward_min_properties() {
        let sub = resolve(json!({
            "type": "object",
            "required": ["a"],
            "dependentRequired": { "a": ["b"] }
        }));
        let sup = resolve(json!({
            "type": "object",
            "minProperties": 2
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn optional_dependent_required_does_not_imply_min_properties() {
        let sub = resolve(json!({
            "type": "object",
            "dependentRequired": { "a": ["b"] }
        }));
        let sup = resolve(json!({
            "type": "object",
            "minProperties": 2
        }));

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn impossible_dependent_required_overflows_property_budget_is_vacuous() {
        let impossible = resolve(json!({
            "type": "object",
            "required": ["a"],
            "maxProperties": 1,
            "dependentRequired": { "a": ["b"] }
        }));
        let arbitrary_object = resolve(json!({
            "type": "object",
            "required": ["z"]
        }));

        assert!(is_subschema_of(&impossible, &arbitrary_object));
    }

    #[test]
    fn impossible_required_false_prefix_item_is_vacuous() {
        let impossible = resolve(json!({
            "type": "array",
            "minItems": 1,
            "prefixItems": [false]
        }));
        let arbitrary_array = resolve(json!({
            "type": "array",
            "minItems": 2
        }));

        assert!(is_subschema_of(&impossible, &arbitrary_array));
    }

    #[test]
    fn impossible_required_prefix_item_with_dead_enum_is_vacuous() {
        let impossible = resolve(json!({
            "type": "array",
            "minItems": 1,
            "prefixItems": [{ "type": "string", "enum": [1] }]
        }));
        let arbitrary_array = resolve(json!({
            "type": "array",
            "minItems": 2
        }));

        assert!(is_subschema_of(&impossible, &arbitrary_array));
    }

    #[test]
    fn impossible_required_allof_finite_domain_item_is_vacuous() {
        let impossible = resolve(json!({
            "type": "array",
            "minItems": 1,
            "prefixItems": [{
                "allOf": [
                    { "enum": [1] },
                    { "type": "string" }
                ]
            }]
        }));
        let arbitrary_array = resolve(json!({
            "type": "array",
            "minItems": 2
        }));

        assert!(is_subschema_of(&impossible, &arbitrary_array));
    }

    #[test]
    fn impossible_required_false_tail_item_is_vacuous() {
        let impossible = resolve(json!({
            "type": "array",
            "minItems": 2,
            "prefixItems": [{ "type": "string" }],
            "items": false
        }));
        let arbitrary_array = resolve(json!({
            "type": "array",
            "minItems": 3
        }));

        assert!(is_subschema_of(&impossible, &arbitrary_array));
    }

    #[test]
    fn impossible_unique_tuple_duplicate_singletons_is_vacuous() {
        let impossible = resolve(json!({
            "type": "array",
            "uniqueItems": true,
            "minItems": 2,
            "prefixItems": [
                { "const": "same" },
                { "enum": ["same"] }
            ]
        }));
        let arbitrary_array = resolve(json!({
            "type": "array",
            "minItems": 10
        }));

        assert!(is_subschema_of(&impossible, &arbitrary_array));
    }

    #[test]
    fn impossible_unique_tuple_hall_subset_is_vacuous() {
        // The first three required positions have only two possible values.
        // A fourth, unrelated position makes the total union large enough that
        // a simple global pigeonhole check would miss the contradiction.
        let impossible = resolve(json!({
            "type": "array",
            "uniqueItems": true,
            "minItems": 4,
            "prefixItems": [
                { "enum": [1, 2] },
                { "enum": [1, 2] },
                { "enum": [1, 2] },
                { "enum": [3, 4, 5] }
            ]
        }));
        let arbitrary_array = resolve(json!({
            "type": "array",
            "minItems": 10
        }));

        assert!(is_subschema_of(&impossible, &arbitrary_array));
    }

    #[test]
    fn unique_prefix_tail_overlap_tightens_effective_max_items() {
        let subset = resolve(json!({
            "type": "array",
            "uniqueItems": true,
            "prefixItems": [{ "const": 1 }],
            "items": { "enum": [1, 2] }
        }));
        let superset = resolve(json!({
            "type": "array",
            "maxItems": 2
        }));

        assert!(is_subschema_of(&subset, &superset));
    }

    #[test]
    fn finite_item_domain_disjoint_from_contains_satisfies_max_contains_zero() {
        let subset = resolve(json!({
            "type": "array",
            "items": { "enum": [1, 2] }
        }));
        let superset = resolve(json!({
            "type": "array",
            "contains": { "const": 3 },
            "minContains": 0,
            "maxContains": 0
        }));

        assert!(is_subschema_of(&subset, &superset));
    }

    #[test]
    fn unique_boolean_items_have_finite_capacity() {
        let subset = resolve(json!({
            "type": "array",
            "uniqueItems": true,
            "items": { "type": "boolean" }
        }));
        let superset = resolve(json!({
            "type": "array",
            "maxItems": 2
        }));

        assert!(is_subschema_of(&subset, &superset));
    }

    #[test]
    fn unique_anyof_finite_item_domain_has_finite_capacity() {
        let subset = resolve(json!({
            "type": "array",
            "uniqueItems": true,
            "items": {
                "anyOf": [
                    { "const": "a" },
                    { "const": "b" }
                ]
            }
        }));
        let superset = resolve(json!({
            "type": "array",
            "maxItems": 2
        }));

        assert!(is_subschema_of(&subset, &superset));
    }

    #[test]
    fn unique_conditional_finite_item_domain_has_finite_capacity() {
        let subset = resolve(json!({
            "type": "array",
            "uniqueItems": true,
            "items": {
                "if": { "type": "string" },
                "then": { "enum": ["a"] },
                "else": { "enum": [1] }
            }
        }));
        let superset = resolve(json!({
            "type": "array",
            "maxItems": 2
        }));

        assert!(is_subschema_of(&subset, &superset));
    }

    #[test]
    fn unique_conditional_missing_then_finite_condition_has_capacity() {
        let subset = resolve(json!({
            "type": "array",
            "uniqueItems": true,
            "items": {
                "if": { "enum": [1, 2] },
                "else": { "const": 3 }
            }
        }));
        let superset = resolve(json!({
            "type": "array",
            "maxItems": 3
        }));

        assert!(is_subschema_of(&subset, &superset));
    }

    #[test]
    fn unique_conditional_missing_else_finite_negated_condition_has_capacity() {
        let subset = resolve(json!({
            "type": "array",
            "uniqueItems": true,
            "items": {
                "if": { "not": { "enum": [1, 2] } },
                "then": { "const": 3 }
            }
        }));
        let superset = resolve(json!({
            "type": "array",
            "maxItems": 3
        }));

        assert!(is_subschema_of(&subset, &superset));
    }

    #[test]
    fn unique_zero_sized_object_and_array_items_have_singleton_capacity() {
        let empty_objects = resolve(json!({
            "type": "array",
            "uniqueItems": true,
            "items": { "type": "object", "maxProperties": 0 }
        }));
        let at_most_one = resolve(json!({
            "type": "array",
            "maxItems": 1
        }));
        assert!(is_subschema_of(&empty_objects, &at_most_one));

        let empty_arrays = resolve(json!({
            "type": "array",
            "uniqueItems": true,
            "items": { "type": "array", "maxItems": 0 }
        }));
        assert!(is_subschema_of(&empty_arrays, &at_most_one));
    }

    #[test]
    fn unique_closed_object_item_domain_has_finite_capacity() {
        let subset = resolve(json!({
            "type": "array",
            "uniqueItems": true,
            "items": {
                "type": "object",
                "properties": {
                    "flag": { "type": "boolean" }
                },
                "additionalProperties": false
            }
        }));
        let superset = resolve(json!({
            "type": "array",
            "maxItems": 3
        }));

        // The only possible item objects are {}, {flag:false}, and {flag:true}.
        assert!(is_subschema_of(&subset, &superset));
    }

    #[test]
    fn unique_closed_object_ignores_impossible_recursive_optional_property() {
        let subset = resolve(json!({
            "$defs": {
                "Obj": {
                    "type": "object",
                    "properties": {
                        "flag": { "type": "boolean" },
                        "child": { "$ref": "#/$defs/Obj" }
                    },
                    "required": ["flag"],
                    "maxProperties": 1,
                    "additionalProperties": false
                }
            },
            "type": "array",
            "uniqueItems": true,
            "items": { "$ref": "#/$defs/Obj" }
        }));
        let superset = resolve(json!({
            "type": "array",
            "maxItems": 2
        }));

        assert!(is_subschema_of(&subset, &superset));
    }

    #[test]
    fn disjoint_tuple_positions_imply_unique_items() {
        let subset = resolve(json!({
            "type": "array",
            "prefixItems": [
                { "const": "id" },
                { "type": "integer", "minimum": 1, "maximum": 2 },
                { "const": true }
            ],
            "items": false
        }));
        let superset = resolve(json!({
            "type": "array",
            "prefixItems": [
                { "const": "id" },
                { "type": "integer", "minimum": 1, "maximum": 2 },
                { "const": true }
            ],
            "items": false,
            "uniqueItems": true
        }));

        assert!(is_subschema_of(&subset, &superset));
    }

    #[test]
    fn identical_conditionals_compare_branchwise() {
        let subset = resolve(json!({
            "if": { "type": "string" },
            "then": { "type": "string", "maxLength": 3 },
            "else": { "type": "integer", "minimum": 2, "maximum": 4 }
        }));
        let superset = resolve(json!({
            "if": { "type": "string" },
            "then": { "type": "string", "maxLength": 5 },
            "else": { "type": "integer", "minimum": 1, "maximum": 5 }
        }));

        assert!(is_subschema_of(&subset, &superset));
    }

    #[test]
    fn subset_implying_superset_condition_only_needs_then_branch() {
        let subset = resolve(json!({ "type": "string", "maxLength": 3 }));
        let superset = resolve(json!({
            "if": { "type": "string" },
            "then": { "type": "string", "maxLength": 5 },
            "else": { "type": "integer" }
        }));

        assert!(is_subschema_of(&subset, &superset));
    }

    #[test]
    fn subset_disjoint_from_superset_condition_only_needs_else_branch() {
        let subset = resolve(json!({ "type": "integer", "minimum": 2, "maximum": 4 }));
        let superset = resolve(json!({
            "if": { "type": "string" },
            "then": { "type": "string", "maxLength": 5 },
            "else": { "type": "integer", "minimum": 1, "maximum": 5 }
        }));

        assert!(is_subschema_of(&subset, &superset));
    }

    #[test]
    fn subset_with_negated_guard_only_needs_else_branch() {
        let subset = resolve(json!({
            "allOf": [
                { "type": "string" },
                { "not": { "const": "debug" } }
            ]
        }));
        let superset = resolve(json!({
            "if": { "const": "debug" },
            "then": { "type": "integer" },
            "else": { "type": "string" }
        }));

        assert!(is_subschema_of(&subset, &superset));
    }

    #[test]
    fn unrelated_negated_guard_does_not_force_else_branch() {
        let subset = resolve(json!({
            "allOf": [
                { "type": "string" },
                { "not": { "const": "release" } }
            ]
        }));
        let superset = resolve(json!({
            "if": { "const": "debug" },
            "then": { "type": "integer" },
            "else": { "type": "string" }
        }));

        assert!(!is_subschema_of(&subset, &superset));
    }

    #[test]
    fn subset_contained_by_both_superset_conditional_branches() {
        let subset = resolve(json!({
            "type": "string",
            "maxLength": 3
        }));
        let superset = resolve(json!({
            "if": { "type": "number" },
            "then": { "type": "string", "maxLength": 5 },
            "else": { "type": "string", "maxLength": 5 }
        }));

        assert!(is_subschema_of(&subset, &superset));
    }

    #[test]
    fn superset_conditional_requires_all_possible_branches() {
        let subset = resolve(json!({ "type": "string", "maxLength": 3 }));
        let superset = resolve(json!({
            "if": { "type": "number" },
            "then": { "type": "string", "maxLength": 5 },
            "else": { "type": "integer" }
        }));

        assert!(!is_subschema_of(&subset, &superset));
    }

    #[test]
    fn conditional_with_bounded_branches_subsets_common_target() {
        let subset = resolve(json!({
            "if": { "type": "string" },
            "then": { "enum": ["a", "b"] },
            "else": { "enum": [1, 2] }
        }));
        let superset = resolve(json!({
            "anyOf": [
                { "enum": ["a", "b"] },
                { "enum": [1, 2] }
            ]
        }));

        assert!(is_subschema_of(&subset, &superset));
    }

    #[test]
    fn conditional_string_branches_subset_common_string_target() {
        let subset = resolve(json!({
            "if": { "maxLength": 3 },
            "then": { "type": "string", "minLength": 1, "maxLength": 3 },
            "else": { "type": "string", "minLength": 4, "maxLength": 8 }
        }));
        let superset = resolve(json!({
            "type": "string",
            "maxLength": 8
        }));

        assert!(is_subschema_of(&subset, &superset));
    }

    #[test]
    fn conditional_branches_can_target_union_collectively() {
        let subset = resolve(json!({
            "if": { "type": "string" },
            "then": { "type": "string", "maxLength": 4 },
            "else": { "type": "integer", "minimum": 0, "maximum": 10 }
        }));
        let superset = resolve(json!({
            "anyOf": [
                { "type": "string", "maxLength": 8 },
                { "type": "integer", "minimum": 0, "maximum": 20 }
            ]
        }));

        assert!(is_subschema_of(&subset, &superset));
    }

    #[test]
    fn unique_small_integer_range_has_finite_capacity() {
        let subset = resolve(json!({
            "type": "array",
            "uniqueItems": true,
            "items": {
                "type": "integer",
                "minimum": 1,
                "maximum": 3
            }
        }));
        let superset = resolve(json!({
            "type": "array",
            "maxItems": 3
        }));

        assert!(is_subschema_of(&subset, &superset));
    }

    #[test]
    fn finite_allof_integer_domain_can_prove_enum_subset() {
        let subset = resolve(json!({
            "allOf": [
                {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 3
                },
                { "type": "integer", "multipleOf": 2 }
            ]
        }));
        let superset = resolve(json!({ "enum": [2] }));

        assert!(is_subschema_of(&subset, &superset));
    }

    #[test]
    fn finite_allof_domain_filters_rejected_candidates_for_capacity() {
        let subset = resolve(json!({
            "type": "array",
            "uniqueItems": true,
            "items": {
                "allOf": [
                    { "enum": [1, 2] },
                    { "not": { "const": 2 } }
                ]
            }
        }));
        let superset = resolve(json!({
            "type": "array",
            "maxItems": 1
        }));

        assert!(is_subschema_of(&subset, &superset));
    }

    #[test]
    fn zero_length_property_names_cap_object_capacity() {
        let impossible = resolve(json!({
            "type": "object",
            "minProperties": 2,
            "propertyNames": { "type": "string", "maxLength": 0 }
        }));
        let arbitrary_object = resolve(json!({
            "type": "object",
            "required": ["z"]
        }));

        assert!(is_subschema_of(&impossible, &arbitrary_object));
    }

    #[test]
    fn allof_finite_child_caps_property_names() {
        let impossible = resolve(json!({
            "type": "object",
            "minProperties": 3,
            "propertyNames": {
                "allOf": [
                    { "enum": ["a", "b"] },
                    { "type": "string" }
                ]
            }
        }));
        let arbitrary_object = resolve(json!({
            "type": "object",
            "required": ["z"]
        }));

        assert!(is_subschema_of(&impossible, &arbitrary_object));
    }

    #[test]
    fn recursive_trivial_universal_probe_does_not_overflow() {
        let schema = resolve(json!({
            "$defs": {
                "U": { "allOf": [true, { "$ref": "#/$defs/U" }] }
            },
            "$ref": "#/$defs/U"
        }));

        // Cyclic proofs deliberately give up rather than recursing forever.
        assert!(!schema_is_trivially_universal(&schema));
    }

    #[test]
    fn unique_integral_number_multiple_domain_has_finite_capacity() {
        let subset = resolve(json!({
            "type": "array",
            "uniqueItems": true,
            "items": {
                "type": "number",
                "minimum": 0,
                "maximum": 4,
                "multipleOf": 2
            }
        }));
        let superset = resolve(json!({
            "type": "array",
            "maxItems": 3
        }));

        assert!(is_subschema_of(&subset, &superset));
    }

    #[test]
    fn unique_finite_property_names_with_supported_patterns_has_capacity() {
        let subset = resolve(json!({
            "type": "array",
            "uniqueItems": true,
            "items": {
                "type": "object",
                "propertyNames": { "enum": ["x", "y"] },
                "patternProperties": {
                    "^x$": { "const": 1 }
                },
                "additionalProperties": { "const": 2 }
            }
        }));
        let superset = resolve(json!({
            "type": "array",
            "maxItems": 4
        }));

        assert!(is_subschema_of(&subset, &superset));
    }

    #[test]
    fn unique_open_object_finite_property_names_and_values_has_capacity() {
        let subset = resolve(json!({
            "type": "array",
            "uniqueItems": true,
            "items": {
                "type": "object",
                "propertyNames": { "enum": ["a"] },
                "additionalProperties": { "type": "boolean" }
            }
        }));
        let superset = resolve(json!({
            "type": "array",
            "maxItems": 3
        }));

        assert!(is_subschema_of(&subset, &superset));
    }

    #[test]
    fn impossible_false_contains_requirement_is_vacuous() {
        let impossible = resolve(json!({
            "type": "array",
            "contains": false,
            "minContains": 1
        }));
        let arbitrary_array = resolve(json!({
            "type": "array",
            "minItems": 10
        }));

        assert!(is_subschema_of(&impossible, &arbitrary_array));
    }

    #[test]
    fn impossible_contains_count_above_max_items_is_vacuous() {
        let impossible = resolve(json!({
            "type": "array",
            "maxItems": 1,
            "contains": { "type": "string" },
            "minContains": 2
        }));
        let arbitrary_array = resolve(json!({
            "type": "array",
            "minItems": 10
        }));

        assert!(is_subschema_of(&impossible, &arbitrary_array));
    }
}
