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
    number_constraints_subsumed_by_integer, string_constraints_subsumed,
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

/// Peel semantically-neutral singleton applicators.  `allOf`, `anyOf`, and
/// `oneOf` with exactly one child are all equivalent to that child; keeping
/// them wrapped tends to hide simple negation and type facts from the structural
/// prover.  Stop on cycles defensively (recursive refs can preserve wrappers).
fn unwrap_singleton_applicators(mut node: &SchemaNode) -> &SchemaNode {
    let mut seen = HashSet::new();
    loop {
        if !seen.insert(node.id()) {
            return node;
        }
        match node.kind() {
            SchemaNodeKind::AllOf(children)
            | SchemaNodeKind::AnyOf(children)
            | SchemaNodeKind::OneOf(children)
                if children.len() == 1 =>
            {
                node = &children[0];
            }
            _ => return node,
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

    let normalized_sub = unwrap_singleton_applicators(sub);
    let normalized_sup = unwrap_singleton_applicators(sup);
    if normalized_sub.id() != sub.id() || normalized_sup.id() != sup.id() {
        return analyze_subschema_with_context(normalized_sub, normalized_sup, context, mode);
    }

    // A zero possible-type mask is a syntactic proof that the subset language
    // is empty (for example, an allOf intersection of disjoint JSON types).
    // The mask helper is an upper bound, so returning compatible here is safe
    // and avoids falling into branch-wise applicator explanations for
    // impossible schemas.
    if possible_json_type_mask(sub) == 0 {
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

    // Split allOf integer ranges can be finite even when no individual
    // conjunct is finite (e.g. `integer & minimum & maximum`). Enumerate a
    // small outward-rounded upper bound and require every live candidate to be
    // accepted by the target.
    if let Some(values) = finite_split_allof_integer_values(sub)
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

    // A few local array contradictions (impossible contains, uniqueItems over
    // too-small finite domains, repeated required singleton positions) make an
    // array branch empty.  Check them before JSON-type dispatch so they remain
    // vacuous even against a different target type; the matcher emptiness
    // helper has its own cycle guard for recursive schemas.
    if array_schema_is_locally_impossible(sub) {
        context.active_pairs.remove(&recursion_key);
        return SubschemaAnalysis::compatible();
    }

    // Likewise, a few local object contradictions (required false property,
    // impossible propertyNames capacity, dependent-required overflow) make the
    // subset empty even when the target has a different JSON type.
    if object_schema_is_locally_impossible(sub) {
        context.active_pairs.remove(&recursion_key);
        return SubschemaAnalysis::compatible();
    }

    // Normalize the two degenerate XOR shapes that show up surprisingly often
    // in generated schemas. `oneOf: [A, A]` is empty, while
    // `oneOf: [false, A]` is exactly `A` (and symmetrically for the false arm).
    // Keeping this before the general oneOf handlers avoids treating an empty
    // xor as a union of its branches.
    if let SchemaNodeKind::OneOf(branches) = sub.kind() {
        if one_of_pair_is_syntactically_empty(branches) {
            context.active_pairs.remove(&recursion_key);
            return SubschemaAnalysis::compatible();
        }
        if let Some(live) = one_of_trivial_live_branch(branches) {
            context.active_pairs.remove(&recursion_key);
            return if let Some(live) = live {
                analyze_subschema_with_context(live, sup, context, mode)
            } else {
                SubschemaAnalysis::compatible()
            };
        }
        if let Some(live) = one_of_pair_single_live_branch(branches) {
            let result = analyze_subschema_with_context(live, sup, context, mode);
            context.active_pairs.remove(&recursion_key);
            return result;
        }
    }
    if let SchemaNodeKind::OneOf(branches) = sup.kind() {
        if let Some(Some(live)) = one_of_trivial_live_branch(branches) {
            let result = analyze_subschema_with_context(sub, live, context, mode);
            context.active_pairs.remove(&recursion_key);
            return result;
        }
        // An empty oneOf on the right only contains an empty subset; leave that
        // to the ordinary machinery unless the subset was handled above.
        if let Some(live) = one_of_pair_single_live_branch(branches) {
            let result = analyze_subschema_with_context(sub, live, context, mode);
            context.active_pairs.remove(&recursion_key);
            return result;
        }
    }

    // JSON Schema negation is involutive. Peel syntactic double negation early,
    // before union/conditional special cases can obscure the positive inner
    // schema. This is purely a normalization step, not general complement
    // reasoning.
    if let SchemaNodeKind::Not(outer) = sub.kind()
        && let SchemaNodeKind::Not(inner) = unwrap_singleton_applicators(outer).kind()
    {
        let result = analyze_subschema_with_context(inner, sup, context, mode);
        context.active_pairs.remove(&recursion_key);
        return result;
    }
    if let SchemaNodeKind::Not(outer) = sup.kind()
        && let SchemaNodeKind::Not(inner) = unwrap_singleton_applicators(outer).kind()
    {
        let result = analyze_subschema_with_context(sub, inner, context, mode);
        context.active_pairs.remove(&recursion_key);
        return result;
    }

    // Collapse conditionals whose guard is syntactically constant.  The
    // normal conditional prover is intentionally conservative because most
    // guards partition the instance space.  For `if: true`/`if: false`,
    // however, JSON Schema reduces exactly to the selected branch (or `true`
    // when that branch is absent).  Generated schemas often leave this shape
    // behind after simplifying a guard, and treating it structurally avoids
    // losing straightforward enum/range implications inside the live branch.
    if let SchemaNodeKind::IfThenElse {
        if_schema,
        then_schema,
        else_schema,
    } = sup.kind()
        && let Some(live) =
            constant_guard_conditional_branch(if_schema, then_schema.as_ref(), else_schema.as_ref())
    {
        match live {
            LiveConditionalBranch::Universal => {
                context.active_pairs.remove(&recursion_key);
                return SubschemaAnalysis::compatible();
            }
            LiveConditionalBranch::Schema(branch) if branch.id() != sup.id() => {
                let result = analyze_subschema_with_context(sub, branch, context, mode);
                context.active_pairs.remove(&recursion_key);
                return result;
            }
            LiveConditionalBranch::Schema(_) => {}
        }
    }

    if let SchemaNodeKind::IfThenElse {
        if_schema,
        then_schema,
        else_schema,
    } = sub.kind()
        && let Some(live) =
            constant_guard_conditional_branch(if_schema, then_schema.as_ref(), else_schema.as_ref())
    {
        match live {
            LiveConditionalBranch::Universal => {
                let is_subschema = schema_is_trivially_universal(sup);
                context.active_pairs.remove(&recursion_key);
                return SubschemaAnalysis::from_check(is_subschema, mode, || {
                    explain_schema_kind_gap(sub, sup)
                });
            }
            LiveConditionalBranch::Schema(branch) if branch.id() != sub.id() => {
                let result = analyze_subschema_with_context(branch, sup, context, mode);
                context.active_pairs.remove(&recursion_key);
                return result;
            }
            LiveConditionalBranch::Schema(_) => {}
        }
    }

    // The complement of a syntactically empty schema is universal.  This
    // catches common normalized contradictions such as `not(allOf[type:string,
    // type:array])` before the ordinary negation arm asks for contravariant
    // implication facts.
    if let SchemaNodeKind::Not(excluded) = sup.kind()
        && schema_is_locally_impossible_for_negation(unwrap_singleton_applicators(excluded))
    {
        context.active_pairs.remove(&recursion_key);
        return SubschemaAnalysis::compatible();
    }

    // Push the same constant-guard conditional normalization through a
    // negated target.  `not(if true then A else B)` is just `not A`, so a
    // disjointness proof against the live branch is sufficient.  Do not
    // synthesize a temporary `Not` node; the existing negated-target
    // disjointness prover already has the right conservative structure.
    if let SchemaNodeKind::Not(excluded) = sup.kind()
        && let SchemaNodeKind::IfThenElse {
            if_schema,
            then_schema,
            else_schema,
        } = unwrap_singleton_applicators(excluded).kind()
        && let Some(live) =
            constant_guard_conditional_branch(if_schema, then_schema.as_ref(), else_schema.as_ref())
    {
        match live {
            LiveConditionalBranch::Schema(branch)
                if schemas_definitely_disjoint_for_negation(sub, branch, context) =>
            {
                context.active_pairs.remove(&recursion_key);
                return SubschemaAnalysis::compatible();
            }
            LiveConditionalBranch::Universal
                if schema_is_locally_empty_for_finite_enumeration(sub) =>
            {
                context.active_pairs.remove(&recursion_key);
                return SubschemaAnalysis::compatible();
            }
            _ => {}
        }
    }

    // A conditional can be syntactically universal even when it is not the
    // literal `true` schema: if every value satisfying the guard is accepted
    // by the `then` branch, and the `else` branch is unconstrained (or absent),
    // the conditional accepts all instances. Recognize this before the main
    // structural match so such generated guards behave like a true superset.
    if let SchemaNodeKind::IfThenElse {
        if_schema,
        then_schema,
        else_schema,
    } = sup.kind()
        && conditional_is_known_universal(
            if_schema,
            then_schema.as_ref(),
            else_schema.as_ref(),
            context,
        )
    {
        context.active_pairs.remove(&recursion_key);
        return SubschemaAnalysis::compatible();
    }

    // A union containing both `A` and `not B` is universal whenever B <= A.
    // This complement-cover pattern is common in generated partition schemas
    // and is safe to recognize before the ordinary branch-wise AnyOf logic.
    if let SchemaNodeKind::AnyOf(branches) = sup.kind()
        && (any_of_contains_known_universal_branch(branches, context)
            || any_of_complement_cover_is_universal(branches, context)
            || any_of_complement_union_cover_is_universal(branches, context)
            || any_of_property_presence_cover_is_universal(branches)
            || any_of_single_property_name_partition_is_universal(branches)
            || any_of_required_property_value_partition_is_universal(branches)
            || any_of_prefix_item_partition_is_universal(branches)
            || any_of_items_contains_partition_is_universal(branches)
            || any_of_integer_partition_cover_is_universal(branches)
            || any_of_numeric_range_cover_is_universal(branches)
            || any_of_string_length_cover_is_universal(branches)
            || any_of_array_count_cover_is_universal(branches)
            || any_of_object_count_cover_is_universal(branches))
    {
        context.active_pairs.remove(&recursion_key);
        return SubschemaAnalysis::compatible();
    }

    // A two-arm `oneOf` of a schema and its complement is universal.
    if let SchemaNodeKind::OneOf(branches) = sup.kind()
        && one_of_complement_pair_is_universal(branches, context)
    {
        context.active_pairs.remove(&recursion_key);
        return SubschemaAnalysis::compatible();
    }

    // A `oneOf` with one universal arm behaves like the complement of the
    // other arm.  Use local disjointness to prove a subset lands on exactly
    // the universal side.
    if let SchemaNodeKind::OneOf(branches) = sup.kind()
        && one_of_universal_arm_contains_subset(sub, branches, context)
    {
        context.active_pairs.remove(&recursion_key);
        return SubschemaAnalysis::compatible();
    }

    // Likewise, `oneOf: [A, B, not(anyOf[A, B])]` is universal when the
    // positive siblings are known to be an exact disjoint cover of the union.
    if let SchemaNodeKind::OneOf(branches) = sup.kind()
        && one_of_complement_union_partition_is_universal(branches, context)
    {
        context.active_pairs.remove(&recursion_key);
        return SubschemaAnalysis::compatible();
    }

    // The same finite complement partition can be spelled with `oneOf` when
    // the finite side is split into mutually exclusive sibling branches.
    if let SchemaNodeKind::OneOf(branches) = sup.kind()
        && one_of_finite_complement_partition_is_universal(branches, context)
    {
        context.active_pairs.remove(&recursion_key);
        return SubschemaAnalysis::compatible();
    }

    // A two-arm xor of complements is contained in the union of its excluded
    // sides; prove each excluded side against the target directly.
    if let SchemaNodeKind::OneOf(branches) = sub.kind()
        && complement_only_oneof_subset_of(branches, sup, context)
    {
        context.active_pairs.remove(&recursion_key);
        return SubschemaAnalysis::compatible();
    }

    // A two-arm xor with comparable positive arms is a set difference.
    // If every excluded target arm is either inside the removed side or
    // disjoint from the retained side, the difference fits the negation.
    if let SchemaNodeKind::OneOf(branches) = sub.kind()
        && let SchemaNodeKind::Not(excluded) = sup.kind()
        && comparable_oneof_difference_subset_of_negation(branches, excluded, context)
    {
        context.active_pairs.remove(&recursion_key);
        return SubschemaAnalysis::compatible();
    }

    // A mixed xor `oneOf[A, not B]` collapses to `not(A ∪ B)` when A and B
    // are disjoint. In that case it is certainly contained by `not T` for
    // any T known to fit inside either excluded side. This catches compact
    // encodings of "everything except these two disjoint regions" without
    // constructing a synthetic union node.
    if let SchemaNodeKind::OneOf(branches) = sub.kind()
        && mixed_oneof_disjoint_complement_subset_of_target(branches, sup, context)
    {
        context.active_pairs.remove(&recursion_key);
        return SubschemaAnalysis::compatible();
    }

    // Dual De Morgan shape: the complement of an intersection of explicit
    // complements is the union of their positive inners. If each positive
    // inner is contained by the target, the whole negated intersection is too.
    if let SchemaNodeKind::Not(excluded_intersection) = sub.kind()
        && let SchemaNodeKind::AllOf(conjuncts) =
            unwrap_singleton_applicators(excluded_intersection).kind()
        && conjuncts
            .iter()
            .all(|conjunct| matches!(conjunct.kind(), SchemaNodeKind::Not(_)))
        && conjuncts.iter().all(|conjunct| {
            if let SchemaNodeKind::Not(inner) = conjunct.kind() {
                analyze_subschema_with_context(inner, sup, context, ExplanationMode::VerdictOnly)
                    .is_subschema
            } else {
                false
            }
        })
    {
        context.active_pairs.remove(&recursion_key);
        return SubschemaAnalysis::compatible();
    }

    // If the target mixed xor rejects only a finite comparable gap, it is
    // enough for a negated subset to exclude each gap value concretely.
    if let SchemaNodeKind::Not(excluded) = sub.kind()
        && negated_schema_excludes_mixed_finite_gap(
            unwrap_singleton_applicators(excluded),
            sup,
            context,
        )
    {
        context.active_pairs.remove(&recursion_key);
        return SubschemaAnalysis::compatible();
    }

    // Complemented xor normalization for `not(oneOf[A, not B])` in the
    // conservative comparable/disjoint cases handled by the helper.
    if let SchemaNodeKind::Not(excluded_xor) = sub.kind()
        && let SchemaNodeKind::OneOf(children) = unwrap_singleton_applicators(excluded_xor).kind()
        && (negated_oneof_complement_pair_subset_of(children, sup, context)
            || negated_complement_pair_subset_of_mixed_difference(children, sup, context)
            || negated_xor_covers_mixed_comparable_gap(children, sup, context))
    {
        context.active_pairs.remove(&recursion_key);
        return SubschemaAnalysis::compatible();
    }

    // For a two-arm xor, its complement accepts values that satisfy both
    // arms (or neither arm).  Prove either side directly before falling back
    // to more specialized complement-arm identities.
    if let SchemaNodeKind::Not(excluded_xor) = sup.kind()
        && let SchemaNodeKind::OneOf(children) = unwrap_singleton_applicators(excluded_xor).kind()
        && negated_two_arm_oneof_contains(sub, children, context)
    {
        context.active_pairs.remove(&recursion_key);
        return SubschemaAnalysis::compatible();
    }

    // The complement of `oneOf[not A, B]` contains both A and B when
    // A and B are definitely disjoint: outside the overlap, the xor is false
    // exactly on those two regions.  This is a common spelling of a two-way
    // partition with one arm negated.
    if let SchemaNodeKind::Not(excluded_xor) = sup.kind()
        && let SchemaNodeKind::OneOf(children) = unwrap_singleton_applicators(excluded_xor).kind()
        && negated_oneof_complement_pair_contains(sub, children, context)
    {
        context.active_pairs.remove(&recursion_key);
        return SubschemaAnalysis::compatible();
    }

    // Untyped array/object count assertions canonicalize the same way as
    // string lengths: all other JSON types plus one count half-line.
    if let SchemaNodeKind::Not(excluded_union) = sub.kind()
        && let SchemaNodeKind::AnyOf(children) = unwrap_singleton_applicators(excluded_union).kind()
        && (negated_untyped_array_count_halfline_subset_of(children, sup)
            || negated_untyped_object_count_halfline_subset_of(children, sup))
    {
        context.active_pairs.remove(&recursion_key);
        return SubschemaAnalysis::compatible();
    }

    // Analogous canonicalization for untyped string length assertions:
    // all non-strings are accepted by the positive union, so its negation is
    // the opposite string-length half-line.
    if let SchemaNodeKind::Not(excluded_union) = sub.kind()
        && let SchemaNodeKind::AnyOf(children) = unwrap_singleton_applicators(excluded_union).kind()
        && negated_untyped_string_length_halfline_subset_of(children, sup)
    {
        context.active_pairs.remove(&recursion_key);
        return SubschemaAnalysis::compatible();
    }

    // A negated canonicalized untyped numeric assertion has the shape
    // `not(anyOf[<all non-number types>, <numeric half-line>])`.  Once every
    // non-number type is covered, the complement is a numeric half-line with
    // the endpoint flipped; compare that interval directly to numeric targets.
    if let SchemaNodeKind::Not(excluded_union) = sub.kind()
        && let SchemaNodeKind::AnyOf(children) = unwrap_singleton_applicators(excluded_union).kind()
        && negated_untyped_numeric_halfline_subset_of(children, sup)
    {
        context.active_pairs.remove(&recursion_key);
        return SubschemaAnalysis::compatible();
    }

    // One-sided De Morgan normalization: `not(anyOf[..., not A, ...])`
    // implies `A`. Do this before the main match because finite/enum targets
    // have early arms that would otherwise hide the negated-subset case.
    if let SchemaNodeKind::Not(excluded_union) = sub.kind()
        && let SchemaNodeKind::AnyOf(children) = unwrap_singleton_applicators(excluded_union).kind()
        && (negated_union_type_remainder_subset_of(children, sup)
            || negated_anyof_finite_complement_arm_subset_of(children, sup, context)
            || negated_union_subset_of_mixed_difference(children, sup, context)
            || children.iter().any(|child| {
                if let SchemaNodeKind::Not(required) = child.kind() {
                    analyze_subschema_with_context(
                        required,
                        sup,
                        context,
                        ExplanationMode::VerdictOnly,
                    )
                    .is_subschema
                } else {
                    false
                }
            }))
    {
        context.active_pairs.remove(&recursion_key);
        return SubschemaAnalysis::compatible();
    }

    // A very common xor spelling for "objects without property p" is
    // `oneOf: [{type: object}, {type: object, required: [p]}]`.  The generic
    // oneOf rule below checks each arm independently, which is too strong: the
    // second arm is precisely removed by the xor. Recognize the narrow
    // presence-partition form before falling back to branch-wise reasoning.
    if let SchemaNodeKind::OneOf(children) = sub.kind()
        && (oneof_object_absence_partition_subset_of(children, sup)
            || oneof_array_empty_partition_subset_of(children, sup)
            || oneof_string_empty_partition_subset_of(children, sup)
            || oneof_object_empty_partition_subset_of(children, sup))
    {
        context.active_pairs.remove(&recursion_key);
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
                // First try the whole intersection: split range/count helpers
                // can combine facts spread across conjuncts. Avoid immediately
                // recursing into another allOf wrapper; the conjunct-wise
                // fallback below handles nested intersections without cycles.
                (!matches!(sup_conjunct.kind(), SchemaNodeKind::AllOf(_))
                    && analyze_subschema_with_context(sub, sup_conjunct, context, mode)
                        .is_subschema)
                    || subs.iter().any(|sub_conjunct| {
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
        (AllOf(_), Not(excluded)) => {
            // An intersection can imply a negated target even when no single
            // conjunct does (the De Morgan shape `not A && not B` is the
            // canonical example). Ask the disjointness prover about the whole
            // intersection before falling back to the generic one-conjunct
            // shortcut below.
            let is_subschema = schemas_definitely_disjoint_for_negation(sub, excluded, context);
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
                if_schema,
                then_schema,
                else_schema,
            } = sub.kind()
            {
                conditional_branches_subsumed_by(
                    if_schema,
                    then_schema.as_ref(),
                    else_schema.as_ref(),
                    sup,
                    context,
                    ExplanationMode::VerdictOnly,
                )
            } else {
                false
            };
            let negated_allof_fit_union = if let Not(inner) = sub.kind()
                && let AllOf(conjuncts) = inner.kind()
            {
                negated_allof_covered_by_anyof(conjuncts, sups, context)
            } else {
                false
            };
            let negated_finite_gap_fit_union = if let Not(excluded) = sub.kind() {
                negated_exclusion_covered_by_anyof_finite_gap(excluded, sups, context)
            } else {
                false
            };
            let is_subschema = conditional_branches_fit_union
                || negated_allof_fit_union
                || negated_finite_gap_fit_union
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
        ) => {
            // Equivalent conditions partition the instance space in the same
            // way, so branch-wise implication is sufficient. Node equality is
            // a fast path; otherwise use mutual conservative implication for
            // common independently-resolved guards (e.g. identical enums).
            let guards_equivalent = sub_if == sup_if
                || schemas_obviously_equivalent(sub_if, sup_if)
                || (analyze_subschema_with_context(
                    sub_if,
                    sup_if,
                    context,
                    ExplanationMode::VerdictOnly,
                )
                .is_subschema
                    && analyze_subschema_with_context(
                        sup_if,
                        sub_if,
                        context,
                        ExplanationMode::VerdictOnly,
                    )
                    .is_subschema);
            let is_subschema = if guards_equivalent {
                same_guard_conditional_branches_subsumed(
                    sub_if,
                    sub_then.as_ref(),
                    sub_else.as_ref(),
                    sup_then.as_ref(),
                    sup_else.as_ref(),
                    context,
                    mode,
                )
            } else {
                superset_conditional_contains(
                    sub,
                    sup,
                    sup_if,
                    sup_then.as_ref(),
                    sup_else.as_ref(),
                    context,
                )
            };
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
                if_schema,
                then_schema,
                else_schema,
            },
            _,
        ) => {
            // If both live conditional branches are themselves subsets of the
            // target, the whole conditional is too. Branches whose guard side
            // is syntactically impossible can be ignored; this is common in
            // generated schemas that use a type guard with an incompatible
            // branch as a way to spell a singleton/else case.
            let negated_target_cover = if let SchemaNodeKind::Not(excluded) = sup.kind() {
                schemas_definitely_disjoint_for_negation(sub, excluded, context)
            } else {
                false
            };
            let is_subschema = negated_target_cover
                || conditional_branches_subsumed_by(
                    if_schema,
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
            let split_inner_implication = if let AllOf(conjuncts) = sup_negated.kind() {
                conjuncts.iter().any(|conjunct| {
                    analyze_subschema_with_context(
                        conjunct,
                        sub_negated,
                        context,
                        ExplanationMode::VerdictOnly,
                    )
                    .is_subschema
                })
            } else {
                false
            };
            let inner_implication = split_inner_implication
                || analyze_subschema_with_context(
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
        (Not(sub_negated), _) => match unwrap_singleton_applicators(sub_negated).kind() {
            Not(inner) => {
                // Double negation is exactly the inner schema in JSON Schema.
                // Peel one layer so finite/enumerated inners can use the usual
                // subset machinery (notably unions on the right).
                let is_subschema =
                    analyze_subschema_with_context(inner, sup, context, mode).is_subschema;
                SubschemaAnalysis::from_check(is_subschema, mode, || {
                    explain_schema_kind_gap(sub, sup)
                })
            }
            Any | BoolSchema(true) => SubschemaAnalysis::compatible(),
            BoolSchema(false) => SubschemaAnalysis::from_check(
                matches!(sup.kind(), Any | BoolSchema(true)),
                mode,
                || explain_schema_kind_gap(sub, sup),
            ),
            _ => SubschemaAnalysis::from_check(false, mode, || explain_schema_kind_gap(sub, sup)),
        },
        (_, Not(sup_negated)) => match unwrap_singleton_applicators(sup_negated).kind() {
            Not(inner) => {
                // `not(not T)` on the right is just T; delegate to the normal
                // positive subset proof rather than requiring complement facts.
                let is_subschema =
                    analyze_subschema_with_context(sub, inner, context, mode).is_subschema;
                SubschemaAnalysis::from_check(is_subschema, mode, || {
                    explain_schema_kind_gap(sub, sup)
                })
            }
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
                let excluded = unwrap_singleton_applicators(sup_negated);
                let disjoint = schemas_definitely_disjoint_for_negation(sub, sup_negated, context)
                    || schema_disjoint_from_conditional(sub, excluded, context);
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

        (Number { .. }, Integer { .. }) => SubschemaAnalysis::from_check(
            finite_integer_number_values(sub)
                .is_some_and(|values| context.superset_contains_value_set(sup, &values))
                || number_constraints_subsumed_by_integer(sub, sup),
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

enum LiveConditionalBranch<'a> {
    /// The selected branch is absent, which is the JSON Schema `true` schema.
    Universal,
    Schema(&'a SchemaNode),
}

/// Return the only live branch for a conditional with a syntactically constant
/// guard.  Keep this deliberately narrow: recognizing literal/unconstrained
/// true and literal false is enough for common normalized schemas, while
/// avoiding general emptiness/complement reasoning in a normalization helper.
fn constant_guard_conditional_branch<'a>(
    if_schema: &SchemaNode,
    then_branch: Option<&'a SchemaNode>,
    else_branch: Option<&'a SchemaNode>,
) -> Option<LiveConditionalBranch<'a>> {
    let selected = if schema_is_trivially_universal(if_schema) {
        then_branch
    } else if matches!(if_schema.kind(), SchemaNodeKind::BoolSchema(false)) {
        else_branch
    } else {
        return None;
    };

    Some(match selected {
        Some(branch) => LiveConditionalBranch::Schema(branch),
        None => LiveConditionalBranch::Universal,
    })
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

/// Prove a finite conditional branch is contained by `sup` after restricting it
/// to the side of the guard on which the branch can actually run.  The finite
/// value list is an upper bound for the branch language; values that are
/// definitely rejected by the branch, definitely rejected by the guard (then
/// side), or definitely accepted by the guard (else side) can be ignored.
fn finite_conditional_branch_values_fit_target(
    if_schema: &SchemaNode,
    branch: &SchemaNode,
    sup: &SchemaNode,
    then_side: bool,
    context: &mut SubschemaCheckContext,
) -> bool {
    let Some(values) = finite_schema_value_superset(branch) else {
        return false;
    };
    values.iter().all(|value| {
        if context.schema_definitely_rejects_value(branch, value) {
            return true;
        }
        let can_reach_side = if then_side {
            !context.schema_definitely_rejects_value(if_schema, value)
        } else {
            !context.superset_contains_value(if_schema, value)
        };
        !can_reach_side || context.superset_contains_value(sup, value)
    })
}

/// Dual finite proof for the `then` side: when the guard itself has a finite
/// value superset, enumerate that instead of the branch.  This catches cases
/// like `if: {enum:[1,"a"]}, then: {type: integer}` where the branch is not
/// finite on its own, but the reachable intersection is.  Values that the
/// guard or branch evaluator can soundly reject are ignored; any uncertain
/// value is kept, making the check conservative.
fn finite_conditional_then_guard_values_fit_target(
    if_schema: &SchemaNode,
    then_branch: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    let Some(values) = finite_schema_value_superset(if_schema) else {
        return false;
    };
    values.iter().all(|value| {
        if context.schema_definitely_rejects_value(if_schema, value)
            || context.schema_definitely_rejects_value(then_branch, value)
        {
            return true;
        }
        context.superset_contains_value(sup, value)
    })
}

/// Enumerate a finite guard domain to prove an implicit `then` branch fits a target.
/// This is the missing-branch analogue of `finite_conditional_then_guard_values_fit_target`.
fn finite_guard_values_fit_target(
    if_schema: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    let Some(values) = finite_schema_value_superset(if_schema) else {
        return false;
    };
    values.iter().all(|value| {
        if context.schema_definitely_rejects_value(if_schema, value) {
            return true;
        }
        context.superset_contains_value(sup, value)
    })
}

/// If the guard is `not A`, its else-side domain is contained in `A`.
/// Proving `A <= sup` is therefore enough for any else branch (including an
/// implicit/universal one) to fit the target.
fn negated_guard_complement_subsumed_by_target(
    if_schema: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    let SchemaNodeKind::Not(negated) = if_schema.kind() else {
        return false;
    };
    analyze_subschema_with_context(negated, sup, context, ExplanationMode::VerdictOnly).is_subschema
}

/// Finite fallback for an implicit/universal else side of a negated guard.
/// This is useful when `A <= sup` is too hard structurally but `A` has a small
/// enumerable superset.
fn finite_negated_guard_complement_values_fit_target(
    if_schema: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    let SchemaNodeKind::Not(negated) = if_schema.kind() else {
        return false;
    };
    let Some(values) = finite_schema_value_superset(negated) else {
        return false;
    };
    values.iter().all(|value| {
        if context.schema_definitely_rejects_value(negated, value)
            || context.superset_contains_value(if_schema, value)
        {
            return true;
        }
        context.superset_contains_value(sup, value)
    })
}

/// Finite fallback for an explicit else branch under a negated guard; enumerate
/// the complement side (`A` in `not A`) and filter by the branch.
fn finite_conditional_else_negated_guard_values_fit_target(
    if_schema: &SchemaNode,
    else_branch: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    let SchemaNodeKind::Not(negated) = if_schema.kind() else {
        return false;
    };
    let Some(values) = finite_schema_value_superset(negated) else {
        return false;
    };
    values.iter().all(|value| {
        if context.schema_definitely_rejects_value(negated, value)
            || context.schema_definitely_rejects_value(else_branch, value)
            || context.superset_contains_value(if_schema, value)
        {
            return true;
        }
        context.superset_contains_value(sup, value)
    })
}

fn conditional_branches_subsumed_by(
    if_schema: &SchemaNode,
    then_branch: Option<&SchemaNode>,
    else_branch: Option<&SchemaNode>,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
    mode: ExplanationMode,
) -> bool {
    fn then_branch_is_vacuous(
        if_schema: &SchemaNode,
        then_branch: &SchemaNode,
        context: &mut SubschemaCheckContext,
    ) -> bool {
        // The then side admits only values satisfying both the guard and the
        // branch.  Any sound disjointness fact for those two schemas makes the
        // side empty; false is merely conservative.
        let then_mask = possible_json_type_mask(then_branch);
        then_mask == 0
            || schemas_definitely_disjoint_for_partition(then_branch, then_mask, if_schema, context)
            || schema_definitely_excludes_schema(then_branch, if_schema, context)
            || schema_definitely_excludes_schema(if_schema, then_branch, context)
    }

    fn else_branch_is_vacuous(
        if_schema: &SchemaNode,
        else_branch: &SchemaNode,
        context: &mut SubschemaCheckContext,
    ) -> bool {
        // The else side admits only values that fail the guard.  If the branch
        // itself is a subset of the guard, no value can take that side.
        analyze_subschema_with_context(
            else_branch,
            if_schema,
            context,
            ExplanationMode::VerdictOnly,
        )
        .is_subschema
    }

    match (then_branch, else_branch) {
        (Some(then_branch), Some(else_branch)) => {
            (analyze_subschema_with_context(then_branch, sup, context, mode).is_subschema
                || then_branch_is_vacuous(if_schema, then_branch, context)
                || analyze_subschema_with_context(
                    if_schema,
                    sup,
                    context,
                    ExplanationMode::VerdictOnly,
                )
                .is_subschema
                || finite_conditional_branch_values_fit_target(
                    if_schema,
                    then_branch,
                    sup,
                    true,
                    context,
                )
                || finite_conditional_then_guard_values_fit_target(
                    if_schema,
                    then_branch,
                    sup,
                    context,
                ))
                && (analyze_subschema_with_context(else_branch, sup, context, mode).is_subschema
                    || else_branch_is_vacuous(if_schema, else_branch, context)
                    || branch_covers_guard_complement(if_schema, sup, context)
                    || negated_guard_complement_subsumed_by_target(if_schema, sup, context)
                    || finite_conditional_branch_values_fit_target(
                        if_schema,
                        else_branch,
                        sup,
                        false,
                        context,
                    )
                    || finite_conditional_else_negated_guard_values_fit_target(
                        if_schema,
                        else_branch,
                        sup,
                        context,
                    ))
        }
        (None, Some(else_branch)) => {
            // With no `then`, every value satisfying the guard is accepted on
            // that side.  It is therefore enough for the guard language itself
            // to be contained by the target, plus the usual proof for the
            // explicit else side.
            analyze_subschema_with_context(if_schema, sup, context, mode).is_subschema
                && (analyze_subschema_with_context(else_branch, sup, context, mode).is_subschema
                    || else_branch_is_vacuous(if_schema, else_branch, context)
                    || branch_covers_guard_complement(if_schema, sup, context)
                    || negated_guard_complement_subsumed_by_target(if_schema, sup, context)
                    || finite_conditional_branch_values_fit_target(
                        if_schema,
                        else_branch,
                        sup,
                        false,
                        context,
                    )
                    || finite_conditional_else_negated_guard_values_fit_target(
                        if_schema,
                        else_branch,
                        sup,
                        context,
                    ))
        }
        (Some(then_branch), None) => {
            // With no `else`, values failing the guard are unconstrained.  The
            // cheap sound special case is a universal guard, which leaves no
            // else side at all (or a universal target, as before).
            (schema_is_trivially_universal(sup)
                || schema_is_trivially_universal(if_schema)
                || branch_covers_guard_complement(if_schema, sup, context)
                || negated_guard_complement_subsumed_by_target(if_schema, sup, context)
                || finite_negated_guard_complement_values_fit_target(if_schema, sup, context))
                && (analyze_subschema_with_context(then_branch, sup, context, mode).is_subschema
                    || then_branch_is_vacuous(if_schema, then_branch, context)
                    || analyze_subschema_with_context(
                        if_schema,
                        sup,
                        context,
                        ExplanationMode::VerdictOnly,
                    )
                    .is_subschema
                    || finite_conditional_branch_values_fit_target(
                        if_schema,
                        then_branch,
                        sup,
                        true,
                        context,
                    )
                    || finite_conditional_then_guard_values_fit_target(
                        if_schema,
                        then_branch,
                        sup,
                        context,
                    ))
        }
        (None, None) => schema_is_trivially_universal(sup),
    }
}

fn schemas_obviously_equivalent(a: &SchemaNode, b: &SchemaNode) -> bool {
    use SchemaNodeKind::*;
    match (a.kind(), b.kind()) {
        (Const(x), Const(y)) => json_values_equal(x, y),
        (Const(x), Enum(ys)) | (Enum(ys), Const(x)) => {
            ys.len() == 1 && ys.iter().any(|y| json_values_equal(x, y))
        }
        (Enum(xs), Enum(ys)) => {
            xs.len() == ys.len()
                && xs
                    .iter()
                    .all(|x| ys.iter().any(|y| json_values_equal(x, y)))
        }
        (BoolSchema(x), BoolSchema(y)) => x == y,
        (Any, Any) => true,
        (Null { enumeration: None }, Null { enumeration: None }) => true,
        (Boolean { enumeration: None }, Boolean { enumeration: None }) => true,
        (
            String {
                enumeration: None,
                length: al,
                pattern: ap,
                format: af,
            },
            String {
                enumeration: None,
                length: bl,
                pattern: bp,
                format: bf,
            },
        ) => al == bl && ap == bp && af == bf,
        (
            Number {
                enumeration: None,
                bounds: ab,
                multiple_of: am,
            },
            Number {
                enumeration: None,
                bounds: bb,
                multiple_of: bm,
            },
        ) => ab == bb && am == bm,
        (
            Integer {
                enumeration: None,
                bounds: ab,
                multiple_of: am,
            },
            Integer {
                enumeration: None,
                bounds: bb,
                multiple_of: bm,
            },
        ) => ab == bb && am == bm,
        _ => false,
    }
}

/// Compare two conditionals with an identical guard, taking account of
/// branches that are impossible on their guard side. Missing branches are the
/// JSON Schema `true` schema. For the guarded side, it is enough for the
/// superset branch to cover the guard; for the else side, a recognized
/// complement-cover branch can cover every value outside the guard.
fn same_guard_conditional_branches_subsumed(
    guard: &SchemaNode,
    sub_then: Option<&SchemaNode>,
    sub_else: Option<&SchemaNode>,
    sup_then: Option<&SchemaNode>,
    sup_else: Option<&SchemaNode>,
    context: &mut SubschemaCheckContext,
    mode: ExplanationMode,
) -> bool {
    let then_empty = |branch: &SchemaNode, context: &mut SubschemaCheckContext| {
        let mask = possible_json_type_mask(branch);
        mask == 0
            || schemas_definitely_disjoint_for_partition(branch, mask, guard, context)
            || schema_definitely_excludes_schema(branch, guard, context)
            || schema_definitely_excludes_schema(guard, branch, context)
    };
    let else_empty = |branch: &SchemaNode, context: &mut SubschemaCheckContext| {
        analyze_subschema_with_context(branch, guard, context, ExplanationMode::VerdictOnly)
            .is_subschema
    };

    let then_ok = match (sub_then, sup_then) {
        (_, None) => true,
        (Some(sub_branch), Some(sup_branch)) => {
            then_empty(sub_branch, context)
                || analyze_subschema_with_context(sub_branch, sup_branch, context, mode)
                    .is_subschema
                || analyze_subschema_with_context(
                    guard,
                    sup_branch,
                    context,
                    ExplanationMode::VerdictOnly,
                )
                .is_subschema
                || finite_conditional_branch_values_fit_target(
                    guard, sub_branch, sup_branch, true, context,
                )
                || finite_conditional_then_guard_values_fit_target(
                    guard, sub_branch, sup_branch, context,
                )
        }
        (None, Some(sup_branch)) => {
            analyze_subschema_with_context(guard, sup_branch, context, ExplanationMode::VerdictOnly)
                .is_subschema
                || finite_guard_values_fit_target(guard, sup_branch, context)
        }
    };
    if !then_ok {
        return false;
    }

    match (sub_else, sup_else) {
        (_, None) => true,
        (Some(sub_branch), Some(sup_branch)) => {
            else_empty(sub_branch, context)
                || analyze_subschema_with_context(sub_branch, sup_branch, context, mode)
                    .is_subschema
                || branch_covers_guard_complement(guard, sup_branch, context)
                || finite_conditional_branch_values_fit_target(
                    guard, sub_branch, sup_branch, false, context,
                )
                || finite_conditional_else_negated_guard_values_fit_target(
                    guard, sub_branch, sup_branch, context,
                )
        }
        (None, Some(sup_branch)) => {
            branch_covers_guard_complement(guard, sup_branch, context)
                || negated_guard_complement_subsumed_by_target(guard, sup_branch, context)
                || finite_negated_guard_complement_values_fit_target(guard, sup_branch, context)
        }
    }
}

/// Return true for the small, sound class of conditionals that accept every
/// instance. Missing conditional branches are JSON Schema `true`; for an
/// explicit `then` branch, proving `if <= then` makes the guarded side
/// vacuous as a restriction. We deliberately only treat the else side as
/// covered when it is absent or syntactically universal, avoiding general
/// complement reasoning here.
fn conditional_is_known_universal(
    if_schema: &SchemaNode,
    then_branch: Option<&SchemaNode>,
    else_branch: Option<&SchemaNode>,
    context: &mut SubschemaCheckContext,
) -> bool {
    let then_covers_guard = match then_branch {
        None => true,
        Some(then_branch) => {
            schema_is_trivially_universal(then_branch)
                || analyze_subschema_with_context(
                    if_schema,
                    then_branch,
                    context,
                    ExplanationMode::VerdictOnly,
                )
                .is_subschema
        }
    };
    if !then_covers_guard {
        return false;
    }

    match else_branch {
        None => true,
        Some(else_branch) => branch_covers_guard_complement(if_schema, else_branch, context),
    }
}

/// Prove that a conditional else branch accepts every value outside `if_schema`.
/// A branch `not E` has that property whenever `E <= if_schema`: anything that
/// misses the guard cannot be in `E`, so it satisfies `not E`.  This also works
/// through an anyOf containing such a complement branch.  Keep the recognizer
/// deliberately small; failure is conservative.
fn branch_covers_guard_complement(
    if_schema: &SchemaNode,
    branch: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    if schema_is_trivially_universal(branch) {
        return true;
    }
    match branch.kind() {
        SchemaNodeKind::Not(excluded) => {
            analyze_subschema_with_context(
                excluded,
                if_schema,
                context,
                ExplanationMode::VerdictOnly,
            )
            .is_subschema
        }
        SchemaNodeKind::AnyOf(children) => children
            .iter()
            .any(|child| branch_covers_guard_complement(if_schema, child, context)),
        _ => false,
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
        if_schema,
        then_schema,
        else_schema,
    } = sub.kind()
    {
        conditional_branches_subsumed_by(
            if_schema,
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

fn u64_intervals_cover_nonnegative(intervals: &[(u64, Option<u64>)]) -> bool {
    let mut reach: Option<u128> = None;
    for (min, max) in intervals {
        if *min == 0 {
            match max {
                None => return true,
                Some(upper) => {
                    let upper = u128::from(*upper);
                    reach = Some(reach.map_or(upper, |old| old.max(upper)));
                }
            }
        }
    }
    let Some(mut reach) = reach else {
        return false;
    };
    loop {
        let mut advanced = false;
        for (min, max) in intervals {
            if u128::from(*min) > reach + 1 {
                continue;
            }
            match max {
                None => return true,
                Some(upper) => {
                    let upper = u128::from(*upper);
                    if upper > reach {
                        reach = upper;
                        advanced = true;
                    }
                }
            }
        }
        if !advanced {
            return false;
        }
    }
}

fn usize_intervals_cover_nonnegative(intervals: &[(usize, Option<usize>)]) -> bool {
    let mut reach: Option<u128> = None;
    for (min, max) in intervals {
        if *min == 0 {
            match max {
                None => return true,
                Some(upper) => {
                    let upper = *upper as u128;
                    reach = Some(reach.map_or(upper, |old| old.max(upper)));
                }
            }
        }
    }
    let Some(mut reach) = reach else {
        return false;
    };
    loop {
        let mut advanced = false;
        for (min, max) in intervals {
            if (*min as u128) > reach + 1 {
                continue;
            }
            match max {
                None => return true,
                Some(upper) => {
                    let upper = *upper as u128;
                    if upper > reach {
                        reach = upper;
                        advanced = true;
                    }
                }
            }
        }
        if !advanced {
            return false;
        }
    }
}

/// Recognize `not integer` plus a pair of plain integer range arms that cover
/// the integer lattice.  The complement arm covers fractional numbers and all
/// non-numbers; the range arms cover every integer.
fn any_of_integer_partition_cover_is_universal(branches: &[SchemaNode]) -> bool {
    fn integer_divisor_is_one(multiple_of: &Option<IntegerMultipleOf>) -> bool {
        multiple_of
            .as_ref()
            .and_then(|m| m.integer_divisor())
            .is_none_or(|divisor| divisor == 1)
    }

    fn plain_integer_bounds(schema: &SchemaNode) -> Option<IntegerBounds> {
        match schema.kind() {
            SchemaNodeKind::Integer {
                bounds,
                multiple_of,
                enumeration,
            } if enumeration.is_none() && integer_divisor_is_one(multiple_of) => Some(*bounds),
            _ => None,
        }
    }

    fn is_unbounded_plain_integer(schema: &SchemaNode) -> bool {
        plain_integer_bounds(schema)
            .is_some_and(|bounds| bounds.lower().is_none() && bounds.upper().is_none())
    }

    fn collect(
        schema: &SchemaNode,
        has_noninteger: &mut bool,
        intervals: &mut Vec<IntegerBounds>,
        active: &mut HashSet<NodeId>,
    ) {
        if !active.insert(schema.id()) {
            return;
        }
        match schema.kind() {
            SchemaNodeKind::AnyOf(children) => {
                for child in children {
                    collect(child, has_noninteger, intervals, active);
                }
            }
            SchemaNodeKind::Not(inner) if is_unbounded_plain_integer(inner) => {
                *has_noninteger = true;
            }
            _ => {
                if let Some(bounds) = plain_integer_bounds(schema) {
                    intervals.push(bounds);
                }
            }
        }
        active.remove(&schema.id());
    }

    let mut has_noninteger = false;
    let mut intervals = Vec::new();
    let mut active = HashSet::new();
    for branch in branches {
        collect(branch, &mut has_noninteger, &mut intervals, &mut active);
    }
    if !has_noninteger {
        return false;
    }

    // Merge the integer intervals, allowing finite bridge arms between the
    // unbounded-low and unbounded-high sides.  Endpoints are inclusive in the
    // normalized IR, so the next uncovered integer after `reach` is
    // `reach + 1`.
    let mut reach: Option<i128> = None;
    for bounds in &intervals {
        if bounds.lower().is_none() {
            match bounds.upper() {
                None => return true,
                Some(upper) => {
                    let upper = i128::from(upper);
                    reach = Some(reach.map_or(upper, |old| old.max(upper)));
                }
            }
        }
    }
    let Some(mut reach) = reach else {
        return false;
    };

    loop {
        let mut advanced = false;
        for bounds in &intervals {
            let Some(lower) = bounds.lower() else {
                continue;
            };
            if i128::from(lower) > reach + 1 {
                continue;
            }
            match bounds.upper() {
                None => return true,
                Some(upper) => {
                    let upper = i128::from(upper);
                    if upper > reach {
                        reach = upper;
                        advanced = true;
                    }
                }
            }
        }
        if !advanced {
            return false;
        }
    }
}

/// Recognize an `anyOf` whose plain numeric branches cover the real number
/// line, while sibling applicability arms cover non-numbers.  We only use
/// unconstrained number intervals (no multipleOf/enum), and require an
/// unbounded-low arm plus an unbounded-high arm whose endpoints touch/overlap.
fn any_of_numeric_range_cover_is_universal(branches: &[SchemaNode]) -> bool {
    for bit in [
        JSON_TYPE_NULL,
        JSON_TYPE_BOOL,
        JSON_TYPE_STRING,
        JSON_TYPE_OBJECT,
        JSON_TYPE_ARRAY,
    ] {
        if !branches
            .iter()
            .any(|branch| schema_obviously_accepts_json_type(branch, bit))
        {
            return false;
        }
    }

    fn collect_intervals(
        schema: &SchemaNode,
        out: &mut Vec<NumberBounds>,
        active: &mut HashSet<NodeId>,
    ) {
        if !active.insert(schema.id()) {
            return;
        }
        match schema.kind() {
            SchemaNodeKind::AnyOf(children) => {
                for child in children {
                    collect_intervals(child, out, active);
                }
            }
            SchemaNodeKind::Number {
                bounds,
                multiple_of,
                enumeration,
            } if multiple_of.is_none() && enumeration.is_none() => out.push(*bounds),
            _ => {}
        }
        active.remove(&schema.id());
    }

    fn endpoints_touch_or_overlap(upper: NumberBound, lower: NumberBound) -> bool {
        match (upper, lower) {
            (NumberBound::Unbounded, _) | (_, NumberBound::Unbounded) => true,
            (NumberBound::Inclusive(u), NumberBound::Inclusive(l)) => u >= l,
            (NumberBound::Inclusive(u), NumberBound::Exclusive(l)) => u >= l,
            (NumberBound::Exclusive(u), NumberBound::Inclusive(l)) => u >= l,
            (NumberBound::Exclusive(u), NumberBound::Exclusive(l)) => u > l,
        }
    }

    fn farther_upper(a: NumberBound, b: NumberBound) -> NumberBound {
        match (a, b) {
            (NumberBound::Unbounded, _) | (_, NumberBound::Unbounded) => NumberBound::Unbounded,
            (NumberBound::Inclusive(x), NumberBound::Inclusive(y)) => {
                if y > x {
                    b
                } else {
                    a
                }
            }
            (NumberBound::Inclusive(x), NumberBound::Exclusive(y)) => {
                if y > x {
                    b
                } else {
                    a
                }
            }
            (NumberBound::Exclusive(x), NumberBound::Inclusive(y)) => {
                if y >= x {
                    b
                } else {
                    a
                }
            }
            (NumberBound::Exclusive(x), NumberBound::Exclusive(y)) => {
                if y > x {
                    b
                } else {
                    a
                }
            }
        }
    }

    let mut intervals = Vec::new();
    let mut active = HashSet::new();
    for branch in branches {
        collect_intervals(branch, &mut intervals, &mut active);
    }

    let mut reach: Option<NumberBound> = None;
    for interval in &intervals {
        if matches!(interval.lower(), NumberBound::Unbounded) {
            let upper = interval.upper();
            if matches!(upper, NumberBound::Unbounded) {
                return true;
            }
            reach = Some(reach.map_or(upper, |old| farther_upper(old, upper)));
        }
    }
    let Some(mut reach) = reach else {
        return false;
    };
    loop {
        let mut advanced = false;
        for interval in &intervals {
            if !endpoints_touch_or_overlap(reach, interval.lower()) {
                continue;
            }
            let upper = interval.upper();
            if matches!(upper, NumberBound::Unbounded) {
                return true;
            }
            let next = farther_upper(reach, upper);
            if next != reach {
                reach = next;
                advanced = true;
            }
        }
        if !advanced {
            return false;
        }
    }
}

/// Recognize an `anyOf` whose string branches cover the length line, while
/// sibling applicability arms cover non-strings.  This is the string analogue
/// of the count covers (`maxLength: n` vs `minLength: n+1`).
fn any_of_string_length_cover_is_universal(branches: &[SchemaNode]) -> bool {
    for bit in [
        JSON_TYPE_NULL,
        JSON_TYPE_BOOL,
        JSON_TYPE_NUMBER,
        JSON_TYPE_OBJECT,
        JSON_TYPE_ARRAY,
    ] {
        if !branches
            .iter()
            .any(|branch| schema_obviously_accepts_json_type(branch, bit))
        {
            return false;
        }
    }

    fn collect_intervals(
        schema: &SchemaNode,
        out: &mut Vec<(u64, Option<u64>)>,
        active: &mut HashSet<NodeId>,
    ) {
        if !active.insert(schema.id()) {
            return;
        }
        match schema.kind() {
            SchemaNodeKind::AnyOf(children) => {
                for child in children {
                    collect_intervals(child, out, active);
                }
            }
            SchemaNodeKind::String {
                length,
                pattern,
                format,
                enumeration,
            } if pattern.is_none() && format.is_none() && enumeration.is_none() => {
                out.push((length.min(), length.max()));
            }
            _ => {}
        }
        active.remove(&schema.id());
    }

    let mut intervals = Vec::new();
    let mut active = HashSet::new();
    for branch in branches {
        collect_intervals(branch, &mut intervals, &mut active);
    }
    u64_intervals_cover_nonnegative(&intervals)
}

/// Recognize an `anyOf` whose array branches cover the item-count line, while
/// sibling applicability arms cover non-arrays.  This is the array analogue of
/// the object property-count cover (`maxItems: n` vs `minItems: n+1`).
fn any_of_array_count_cover_is_universal(branches: &[SchemaNode]) -> bool {
    for bit in [
        JSON_TYPE_NULL,
        JSON_TYPE_BOOL,
        JSON_TYPE_NUMBER,
        JSON_TYPE_STRING,
        JSON_TYPE_OBJECT,
    ] {
        if !branches
            .iter()
            .any(|branch| schema_obviously_accepts_json_type(branch, bit))
        {
            return false;
        }
    }

    fn collect_intervals(
        schema: &SchemaNode,
        out: &mut Vec<(u64, Option<u64>)>,
        active: &mut HashSet<NodeId>,
    ) {
        if !active.insert(schema.id()) {
            return;
        }
        match schema.kind() {
            SchemaNodeKind::AnyOf(children) => {
                for child in children {
                    collect_intervals(child, out, active);
                }
            }
            SchemaNodeKind::Array { item_count, .. } if array_schema_is_plain_count(schema) => {
                out.push((item_count.min(), item_count.max()));
            }
            _ => {}
        }
        active.remove(&schema.id());
    }

    let mut intervals = Vec::new();
    let mut active = HashSet::new();
    for branch in branches {
        collect_intervals(branch, &mut intervals, &mut active);
    }
    u64_intervals_cover_nonnegative(&intervals)
}

/// Recognize an `anyOf` whose object branches cover the property-count line,
/// while sibling applicability arms cover all non-objects.  This catches
/// spellings such as `{minProperties: 1} | {maxProperties: 0}` after parser
/// normalization, and a slightly more general empty-object arm paired with a
/// plain non-empty-count arm.
fn any_of_object_count_cover_is_universal(branches: &[SchemaNode]) -> bool {
    for bit in [
        JSON_TYPE_NULL,
        JSON_TYPE_BOOL,
        JSON_TYPE_NUMBER,
        JSON_TYPE_STRING,
        JSON_TYPE_ARRAY,
    ] {
        if !branches
            .iter()
            .any(|branch| schema_obviously_accepts_json_type(branch, bit))
        {
            return false;
        }
    }

    fn collect_intervals(
        schema: &SchemaNode,
        out: &mut Vec<(usize, Option<usize>)>,
        active: &mut HashSet<NodeId>,
    ) {
        if !active.insert(schema.id()) {
            return;
        }
        match schema.kind() {
            SchemaNodeKind::AnyOf(children) => {
                for child in children {
                    collect_intervals(child, out, active);
                }
            }
            SchemaNodeKind::Object { property_count, .. }
                if object_schema_is_plain_count(schema) =>
            {
                out.push((property_count.min(), property_count.max()));
            }
            // Many applicator-only object schemas (for example `properties`)
            // accept the empty object even though they are not plain count
            // ranges.  Record just that singleton fact; it is enough to pair
            // with a `minProperties: 1` arm without assuming anything about
            // their non-empty behavior.
            SchemaNodeKind::Object {
                required,
                property_count,
                enumeration,
                ..
            } if required.is_empty() && enumeration.is_none() && property_count.min() == 0 => {
                out.push((0, Some(0)));
            }
            _ => {}
        }
        active.remove(&schema.id());
    }

    let mut intervals = Vec::new();
    let mut active = HashSet::new();
    for branch in branches {
        collect_intervals(branch, &mut intervals, &mut active);
    }

    usize_intervals_cover_nonnegative(&intervals)
}

/// Recognize the common object-applicator partition
/// `oneOf: [{type:object}, {type:object, minProperties:1}]`, whose
/// language is exactly the empty object.  The partition arms must be otherwise
/// unconstrained; targets are accepted only when the empty object is obviously
/// valid syntactically.
fn oneof_object_empty_partition_subset_of(branches: &[SchemaNode], sup: &SchemaNode) -> bool {
    if branches.len() != 2 {
        return false;
    }

    fn object_with_min(schema: &SchemaNode, min: usize) -> bool {
        match schema.kind() {
            // This recognizer is used to reduce an exact `oneOf` difference.
            // Every union/intersection arm must therefore denote the same
            // language; finding only one matching child would silently discard
            // values contributed by its siblings.
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::AllOf(children) => {
                !children.is_empty() && children.iter().all(|c| object_with_min(c, min))
            }
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
                properties.is_empty()
                    && pattern_properties.is_empty()
                    && required.is_empty()
                    && schema_is_trivially_universal(additional)
                    && schema_is_trivially_universal(property_names)
                    && property_count.min() == min
                    && property_count.max().is_none()
                    && dependent_required.is_empty()
                    && enumeration.is_none()
            }
            _ => false,
        }
    }

    fn accepts_empty_object(schema: &SchemaNode) -> bool {
        match schema.kind() {
            SchemaNodeKind::Any | SchemaNodeKind::BoolSchema(true) => true,
            SchemaNodeKind::AnyOf(children) => children.iter().any(accepts_empty_object),
            SchemaNodeKind::AllOf(children) => children.iter().all(accepts_empty_object),
            SchemaNodeKind::IfThenElse {
                if_schema,
                else_schema,
                ..
            } => {
                if possible_json_type_mask(if_schema) & JSON_TYPE_OBJECT == 0 {
                    else_schema.as_ref().is_none_or(accepts_empty_object)
                } else {
                    false
                }
            }
            SchemaNodeKind::Object {
                required,
                property_count,
                enumeration,
                ..
            } => required.is_empty() && property_count.min() == 0 && enumeration.is_none(),
            _ => false,
        }
    }

    ((object_with_min(&branches[0], 0) && object_with_min(&branches[1], 1))
        || (object_with_min(&branches[1], 0) && object_with_min(&branches[0], 1)))
        && accepts_empty_object(sup)
}

/// Recognize `oneOf: [{type:string}, {type:string, minLength:1}]`, whose
/// language is exactly the empty string.  Pattern/format/enum constraints are
/// deliberately excluded so we never assume a regex accepts "".
fn oneof_string_empty_partition_subset_of(branches: &[SchemaNode], sup: &SchemaNode) -> bool {
    if branches.len() != 2 {
        return false;
    }

    fn string_with_min(schema: &SchemaNode, min: u64) -> bool {
        match schema.kind() {
            // Exact XOR reduction cannot ignore values from sibling branches.
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::AllOf(children) => {
                !children.is_empty() && children.iter().all(|c| string_with_min(c, min))
            }
            SchemaNodeKind::String {
                length,
                pattern,
                format,
                enumeration,
            } => {
                length.min() == min
                    && length.max().is_none()
                    && pattern.is_none()
                    && format.is_none()
                    && enumeration.is_none()
            }
            _ => false,
        }
    }

    fn accepts_empty_string(schema: &SchemaNode) -> bool {
        match schema.kind() {
            SchemaNodeKind::Any | SchemaNodeKind::BoolSchema(true) => true,
            SchemaNodeKind::AnyOf(children) => children.iter().any(accepts_empty_string),
            SchemaNodeKind::AllOf(children) => children.iter().all(accepts_empty_string),
            SchemaNodeKind::IfThenElse {
                if_schema,
                else_schema,
                ..
            } => {
                if possible_json_type_mask(if_schema) & JSON_TYPE_STRING == 0 {
                    else_schema.as_ref().is_none_or(accepts_empty_string)
                } else {
                    false
                }
            }
            SchemaNodeKind::String {
                length,
                pattern,
                format,
                enumeration,
            } => {
                length.min() == 0 && pattern.is_none() && format.is_none() && enumeration.is_none()
            }
            _ => false,
        }
    }

    ((string_with_min(&branches[0], 0) && string_with_min(&branches[1], 1))
        || (string_with_min(&branches[1], 0) && string_with_min(&branches[0], 1)))
        && accepts_empty_string(sup)
}

/// Recognize the exact two-arm xor partition `all arrays` XOR `nonempty arrays`.
/// Its language is just the empty array.  Keep the recognizer deliberately
/// syntactic: the nonempty arm may only impose `minItems: 1`, and the target
/// must accept the empty array for structural reasons.
fn oneof_array_empty_partition_subset_of(branches: &[SchemaNode], sup: &SchemaNode) -> bool {
    if branches.len() != 2 {
        return false;
    }

    fn all_arrays(schema: &SchemaNode) -> bool {
        match schema.kind() {
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::AllOf(children) => {
                !children.is_empty() && children.iter().all(all_arrays)
            }
            SchemaNodeKind::Array {
                prefix_items,
                items,
                item_count,
                contains,
                unique_items,
                enumeration,
            } => {
                prefix_items.is_empty()
                    && schema_is_trivially_universal(items)
                    && item_count.min() == 0
                    && item_count.max().is_none()
                    && contains.is_none()
                    && !*unique_items
                    && enumeration.is_none()
            }
            _ => false,
        }
    }

    fn nonempty_arrays(schema: &SchemaNode) -> bool {
        match schema.kind() {
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::AllOf(children) => {
                !children.is_empty() && children.iter().all(nonempty_arrays)
            }
            SchemaNodeKind::Array {
                prefix_items,
                items,
                item_count,
                contains,
                unique_items,
                enumeration,
            } => {
                prefix_items.is_empty()
                    && schema_is_trivially_universal(items)
                    && item_count.min() == 1
                    && item_count.max().is_none()
                    && contains.is_none()
                    && !*unique_items
                    && enumeration.is_none()
            }
            _ => false,
        }
    }

    fn accepts_empty_array(schema: &SchemaNode) -> bool {
        match schema.kind() {
            SchemaNodeKind::Any | SchemaNodeKind::BoolSchema(true) => true,
            SchemaNodeKind::AnyOf(children) => children.iter().any(accepts_empty_array),
            SchemaNodeKind::AllOf(children) => children.iter().all(accepts_empty_array),
            SchemaNodeKind::IfThenElse {
                if_schema,
                else_schema,
                ..
            } => {
                if possible_json_type_mask(if_schema) & JSON_TYPE_ARRAY == 0 {
                    else_schema.as_ref().is_none_or(accepts_empty_array)
                } else {
                    false
                }
            }
            SchemaNodeKind::Array {
                item_count,
                contains,
                enumeration,
                ..
            } => {
                item_count.min() == 0
                    && contains.as_ref().is_none_or(|c| c.count().min() == 0)
                    && enumeration.is_none()
            }
            _ => false,
        }
    }

    ((all_arrays(&branches[0]) && nonempty_arrays(&branches[1]))
        || (all_arrays(&branches[1]) && nonempty_arrays(&branches[0])))
        && accepts_empty_array(sup)
}

/// Recognize the exact two-arm xor partition `all objects` XOR `objects with p`.
/// Its language is exactly the set of objects where `p` is absent.  This is
/// intentionally narrower than general oneOf difference reasoning: both arms
/// must be unconstrained apart from the single presence requirement, and the
/// target must accept every object missing that same property.
fn oneof_object_absence_partition_subset_of(branches: &[SchemaNode], sup: &SchemaNode) -> bool {
    if branches.len() != 2 {
        return false;
    }

    fn accepts_all_objects(schema: &SchemaNode) -> bool {
        match schema.kind() {
            // The broad XOR arm must be exactly the object domain. Requiring
            // every child to have that language prevents an unrelated sibling
            // from leaking scalars into the reduced difference.
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::AllOf(children) => {
                !children.is_empty() && children.iter().all(accepts_all_objects)
            }
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
                properties.is_empty()
                    && pattern_properties.is_empty()
                    && required.is_empty()
                    && schema_is_trivially_universal(additional)
                    && schema_is_trivially_universal(property_names)
                    && property_count.min() == 0
                    && property_count.max().is_none()
                    && dependent_required.is_empty()
                    && enumeration.is_none()
            }
            _ => false,
        }
    }

    fn presence_name(schema: &SchemaNode) -> Option<&str> {
        match schema.kind() {
            SchemaNodeKind::AnyOf(children) if !children.is_empty() => {
                let name = presence_name(&children[0])?;
                children[1..]
                    .iter()
                    .all(|child| presence_name(child) == Some(name))
                    .then_some(name)
            }
            SchemaNodeKind::AllOf(children) => {
                // Be conservative for split wrappers: exactly one conjunct may
                // provide the presence partition, and every sibling must accept
                // all objects so it cannot narrow the arm.
                let mut found = None;
                for child in children {
                    if let Some(name) = presence_name(child) {
                        if found.replace(name).is_some() {
                            return None;
                        }
                    } else if !accepts_all_objects(child) {
                        return None;
                    }
                }
                found
            }
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
                if required.len() != 1 {
                    return None;
                }
                let name = required.iter().next()?.as_str();
                let only_mentions_name = properties
                    .iter()
                    .all(|(key, value)| key == name && schema_is_trivially_universal(value));
                if only_mentions_name
                    && pattern_properties.is_empty()
                    && schema_is_trivially_universal(additional)
                    && schema_is_trivially_universal(property_names)
                    && property_count.min() <= 1
                    && property_count.max().is_none()
                    && dependent_required
                        .values()
                        .all(|deps| deps.iter().all(|dep| dep == name))
                    && enumeration.is_none()
                {
                    Some(name)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn accepts_all_without(schema: &SchemaNode, name: &str) -> bool {
        match schema.kind() {
            SchemaNodeKind::Any | SchemaNodeKind::BoolSchema(true) => true,
            SchemaNodeKind::AnyOf(children) => {
                children.iter().any(|c| accepts_all_without(c, name))
            }
            SchemaNodeKind::AllOf(children) => {
                children.iter().all(|c| accepts_all_without(c, name))
            }
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema: _,
                else_schema,
            } => {
                // If the guard cannot match objects, every object takes the else
                // branch (or an implicit true branch).
                if possible_json_type_mask(if_schema) & JSON_TYPE_OBJECT == 0 {
                    else_schema
                        .as_ref()
                        .is_none_or(|branch| accepts_all_without(branch, name))
                } else {
                    false
                }
            }
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
                required.is_empty()
                    && properties.keys().all(|key| key == name)
                    && pattern_properties.is_empty()
                    && schema_is_trivially_universal(additional)
                    && schema_is_trivially_universal(property_names)
                    && property_count.min() == 0
                    && property_count.max().is_none()
                    && dependent_required.keys().all(|key| key == name)
                    && enumeration.is_none()
            }
            _ => false,
        }
    }

    let (broad, presence) = (&branches[0], &branches[1]);
    let name = if accepts_all_objects(broad) {
        presence_name(presence)
    } else if accepts_all_objects(presence) {
        presence_name(broad)
    } else {
        None
    };
    name.is_some_and(|name| accepts_all_without(sup, name))
}

/// ```text
/// anyOf: [ { required: [p] }, { properties: { p: S } } ]
/// ```
///
/// JSON Schema object keywords are vacuous for non-objects; after parsing they
/// typically appear as applicability unions (non-object arms plus one object
/// arm).  For objects, the first branch accepts every object where `p` is
/// present, while the second accepts every object where `p` is absent.  The
/// property schema `S` may reject some present values, but those are already
/// covered by the presence branch.  Keep this recognizer deliberately narrow:
/// the presence arm may only require/constrain `p` (or have dependentRequired
/// rules whose dependencies are already satisfied by `p`), and the absence arm
/// may mention only `p` in `properties`/dependentRequired triggers with
/// otherwise-universal object constraints.
fn any_of_property_presence_cover_is_universal(branches: &[SchemaNode]) -> bool {
    fn object_arm_accepts_all_with_property(schema: &SchemaNode, name: &str) -> bool {
        match schema.kind() {
            SchemaNodeKind::AnyOf(children) => children
                .iter()
                .any(|child| object_arm_accepts_all_with_property(child, name)),
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
                // For every object that contains `name`, no other property
                // may be required or constrained.  Dependent-required rules are
                // okay only when their entire dependency set is already
                // satisfied by the presence of `name`.
                required.iter().all(|required_name| required_name == name)
                    && properties
                        .iter()
                        .all(|(key, value)| key == name && schema_is_trivially_universal(value))
                    && pattern_properties.is_empty()
                    && schema_is_trivially_universal(additional)
                    && schema_is_trivially_universal(property_names)
                    && property_count.min() <= 1
                    && property_count.max().is_none()
                    && dependent_required
                        .values()
                        .all(|deps| deps.iter().all(|dep| dep == name))
                    && enumeration.is_none()
            }
            _ => false,
        }
    }

    fn object_arm_accepts_all_without_property(schema: &SchemaNode, name: &str) -> bool {
        match schema.kind() {
            SchemaNodeKind::AnyOf(children) => children
                .iter()
                .any(|child| object_arm_accepts_all_without_property(child, name)),
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
                required.is_empty()
                    && properties.keys().all(|key| key == name)
                    && pattern_properties.is_empty()
                    && schema_is_trivially_universal(additional)
                    && schema_is_trivially_universal(property_names)
                    && property_count.min() == 0
                    && property_count.max().is_none()
                    && dependent_required.keys().all(|key| key == name)
                    && enumeration.is_none()
            }
            _ => false,
        }
    }

    // The presence split only covers objects.  Ensure every other JSON type is
    // wholly accepted by at least one sibling (usually the parser-created
    // applicability arms).
    for bit in [
        JSON_TYPE_NULL,
        JSON_TYPE_BOOL,
        JSON_TYPE_NUMBER,
        JSON_TYPE_STRING,
        JSON_TYPE_ARRAY,
    ] {
        if !branches
            .iter()
            .any(|branch| schema_obviously_accepts_json_type(branch, bit))
        {
            return false;
        }
    }

    let mut candidate_names = HashSet::new();
    fn collect_required_singletons(
        schema: &SchemaNode,
        out: &mut HashSet<String>,
        active: &mut HashSet<NodeId>,
    ) {
        if !active.insert(schema.id()) {
            return;
        }
        match schema.kind() {
            SchemaNodeKind::AnyOf(children) => {
                for child in children {
                    collect_required_singletons(child, out, active);
                }
            }
            SchemaNodeKind::Object {
                required,
                properties,
                dependent_required,
                ..
            } => {
                if required.len() == 1
                    && let Some(name) = required.iter().next()
                {
                    out.insert(name.clone());
                }
                out.extend(properties.keys().cloned());
                for deps in dependent_required.values() {
                    out.extend(deps.iter().cloned());
                }
            }
            _ => {}
        }
        active.remove(&schema.id());
    }
    let mut active = HashSet::new();
    for branch in branches {
        collect_required_singletons(branch, &mut candidate_names, &mut active);
    }

    candidate_names.into_iter().any(|name| {
        branches
            .iter()
            .any(|branch| object_arm_accepts_all_with_property(branch, &name))
            && branches
                .iter()
                .any(|branch| object_arm_accepts_all_without_property(branch, &name))
    })
}

/// Recognize a narrow propertyNames/count partition for objects with at most
/// one property, plus a sibling count arm for objects with two or more
/// properties.  For a one-key object, `propertyNames: P` and
/// `propertyNames: { not: P }` are complementary; the empty object satisfies
/// both propertyNames arms, and the high-count arm covers the remaining object
/// cardinalities.  As with the other object-universal recognizers, require
/// explicit sibling coverage for every non-object JSON type.
fn any_of_single_property_name_partition_is_universal(branches: &[SchemaNode]) -> bool {
    for bit in [
        JSON_TYPE_NULL,
        JSON_TYPE_BOOL,
        JSON_TYPE_NUMBER,
        JSON_TYPE_STRING,
        JSON_TYPE_ARRAY,
    ] {
        if !branches
            .iter()
            .any(|branch| schema_obviously_accepts_json_type(branch, bit))
        {
            return false;
        }
    }

    let mut has_high_count_arm = false;
    let mut positive: Vec<&SchemaNode> = Vec::new();
    let mut negative: Vec<&SchemaNode> = Vec::new();

    fn collect<'a>(
        schema: &'a SchemaNode,
        high: &mut bool,
        positive: &mut Vec<&'a SchemaNode>,
        negative: &mut Vec<&'a SchemaNode>,
        active: &mut HashSet<NodeId>,
    ) {
        if !active.insert(schema.id()) {
            return;
        }
        match schema.kind() {
            SchemaNodeKind::AnyOf(children) => {
                for child in children {
                    collect(child, high, positive, negative, active);
                }
            }
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
                if object_schema_is_plain_count(schema)
                    && property_count.max().is_none()
                    && property_count.min() <= 2
                {
                    *high = true;
                }

                // A low-count propertyNames arm must impose no constraints
                // other than the name predicate and an upper cardinality bound
                // that includes singleton objects.  Empty objects satisfy any
                // propertyNames predicate vacuously.
                let low_shape = properties.is_empty()
                    && pattern_properties.is_empty()
                    && required.is_empty()
                    && schema_is_trivially_universal(additional)
                    && dependent_required.is_empty()
                    && enumeration.is_none()
                    && property_count.min() == 0
                    && property_count.max().is_none_or(|max| max >= 1);
                if low_shape && !schema_is_trivially_universal(property_names) {
                    match property_names.kind() {
                        SchemaNodeKind::Not(inner) => negative.push(inner),
                        _ => positive.push(property_names),
                    }
                }
            }
            _ => {}
        }
        active.remove(&schema.id());
    }

    let mut active = HashSet::new();
    for branch in branches {
        collect(
            branch,
            &mut has_high_count_arm,
            &mut positive,
            &mut negative,
            &mut active,
        );
    }
    if !has_high_count_arm {
        return false;
    }

    positive.iter().any(|pos| {
        negative
            .iter()
            .any(|neg| schemas_obviously_equivalent(pos, neg))
    })
}

/// Recognize a three-way property-value partition:
///
/// * objects without property `p` (spelled as `not { required: [p] }`),
/// * objects with `p` whose value satisfies `S`, and
/// * objects with `p` whose value satisfies `not S`.
///
/// The object arms are kept deliberately plain so they accept arbitrary extra
/// properties.  Non-object coverage is checked independently, because parser
/// applicability expansion often supplies it via sibling arms.
fn any_of_required_property_value_partition_is_universal(branches: &[SchemaNode]) -> bool {
    for bit in [
        JSON_TYPE_NULL,
        JSON_TYPE_BOOL,
        JSON_TYPE_NUMBER,
        JSON_TYPE_STRING,
        JSON_TYPE_ARRAY,
    ] {
        if !branches
            .iter()
            .any(|branch| schema_obviously_accepts_json_type(branch, bit))
        {
            return false;
        }
    }

    fn plain_presence_object_name(schema: &SchemaNode) -> Option<String> {
        let SchemaNodeKind::Object {
            properties,
            pattern_properties,
            required,
            additional,
            property_names,
            property_count,
            dependent_required,
            enumeration,
        } = schema.kind()
        else {
            return None;
        };
        if required.len() != 1
            || !pattern_properties.is_empty()
            || !schema_is_trivially_universal(additional)
            || !schema_is_trivially_universal(property_names)
            || property_count.min() > 1
            || property_count.max().is_some()
            || !dependent_required.is_empty()
            || enumeration.is_some()
        {
            return None;
        }
        let name = required.iter().next()?.clone();
        if properties
            .iter()
            .all(|(key, value)| key == &name && schema_is_trivially_universal(value))
        {
            Some(name)
        } else {
            None
        }
    }

    fn presence_test_name(schema: &SchemaNode) -> Option<String> {
        match schema.kind() {
            SchemaNodeKind::Object { .. } => plain_presence_object_name(schema),
            SchemaNodeKind::AnyOf(children) => {
                let mut object_name: Option<String> = None;
                let mut saw_object = false;
                for child in children {
                    if possible_json_type_mask(child) & JSON_TYPE_OBJECT == 0 {
                        continue;
                    }
                    saw_object = true;
                    let name = plain_presence_object_name(child)?;
                    if object_name.as_ref().is_some_and(|old| old != &name) {
                        return None;
                    }
                    object_name = Some(name);
                }
                saw_object.then_some(object_name).flatten()
            }
            _ => None,
        }
    }

    fn collect_absence_names(
        schema: &SchemaNode,
        out: &mut HashSet<String>,
        active: &mut HashSet<NodeId>,
    ) {
        if !active.insert(schema.id()) {
            return;
        }
        match schema.kind() {
            SchemaNodeKind::AnyOf(children) => {
                for child in children {
                    collect_absence_names(child, out, active);
                }
            }
            SchemaNodeKind::Not(inner) => {
                if let Some(name) = presence_test_name(inner) {
                    out.insert(name);
                }
            }
            _ => {}
        }
        active.remove(&schema.id());
    }

    fn collect_value_arms<'a>(
        schema: &'a SchemaNode,
        out: &mut Vec<(String, bool, &'a SchemaNode)>,
        active: &mut HashSet<NodeId>,
    ) {
        if !active.insert(schema.id()) {
            return;
        }
        match schema.kind() {
            SchemaNodeKind::AnyOf(children) => {
                for child in children {
                    collect_value_arms(child, out, active);
                }
            }
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
                if required.len() != 1
                    || properties.len() != 1
                    || !pattern_properties.is_empty()
                    || !schema_is_trivially_universal(additional)
                    || !schema_is_trivially_universal(property_names)
                    || property_count.min() > 1
                    || property_count.max().is_some()
                    || !dependent_required.is_empty()
                    || enumeration.is_some()
                {
                    return;
                }
                let name = required.iter().next().expect("len checked");
                let Some(value_schema) = properties.get(name) else {
                    return;
                };
                match value_schema.kind() {
                    SchemaNodeKind::Not(inner) => out.push((name.clone(), false, inner)),
                    _ => out.push((name.clone(), true, value_schema)),
                }
            }
            _ => {}
        }
        active.remove(&schema.id());
    }

    let mut absence_names = HashSet::new();
    let mut value_arms = Vec::new();
    let mut absence_active = HashSet::new();
    let mut value_active = HashSet::new();
    for branch in branches {
        collect_absence_names(branch, &mut absence_names, &mut absence_active);
        collect_value_arms(branch, &mut value_arms, &mut value_active);
    }

    for name in absence_names {
        let positives: Vec<_> = value_arms
            .iter()
            .filter(|(arm_name, positive, _)| arm_name == &name && *positive)
            .map(|(_, _, schema)| *schema)
            .collect();
        let negatives: Vec<_> = value_arms
            .iter()
            .filter(|(arm_name, positive, _)| arm_name == &name && !*positive)
            .map(|(_, _, schema)| *schema)
            .collect();
        if positives.iter().any(|pos| {
            negatives
                .iter()
                .any(|neg| schemas_obviously_equivalent(pos, neg))
        }) {
            return true;
        }
    }
    false
}

/// Recognize complementary constraints on a single tuple position.  A
/// `prefixItems[i]` constraint is vacuous for arrays shorter than `i + 1`, so
/// two otherwise-plain array arms with predicates `P` and `not P` at the same
/// position cover every array; parser applicability arms cover non-arrays.
fn any_of_prefix_item_partition_is_universal(branches: &[SchemaNode]) -> bool {
    for bit in [
        JSON_TYPE_NULL,
        JSON_TYPE_BOOL,
        JSON_TYPE_NUMBER,
        JSON_TYPE_STRING,
        JSON_TYPE_OBJECT,
    ] {
        if !branches
            .iter()
            .any(|branch| schema_obviously_accepts_json_type(branch, bit))
        {
            return false;
        }
    }

    let mut positives: Vec<(usize, &SchemaNode)> = Vec::new();
    let mut negatives: Vec<(usize, &SchemaNode)> = Vec::new();

    fn collect<'a>(
        schema: &'a SchemaNode,
        positives: &mut Vec<(usize, &'a SchemaNode)>,
        negatives: &mut Vec<(usize, &'a SchemaNode)>,
        active: &mut HashSet<NodeId>,
    ) {
        if !active.insert(schema.id()) {
            return;
        }
        match schema.kind() {
            SchemaNodeKind::AnyOf(children) => {
                for child in children {
                    collect(child, positives, negatives, active);
                }
            }
            SchemaNodeKind::Array {
                prefix_items,
                items,
                item_count,
                contains,
                unique_items,
                enumeration,
            } => {
                if !schema_is_trivially_universal(items)
                    || item_count.min() != 0
                    || item_count.max().is_some()
                    || contains.is_some()
                    || *unique_items
                    || enumeration.is_some()
                {
                    return;
                }
                for (idx, item_schema) in prefix_items.iter().enumerate() {
                    // All other tuple positions must be unconstrained; this
                    // keeps the arm a pure predicate on one optional slot.
                    if prefix_items.iter().enumerate().any(|(other_idx, other)| {
                        other_idx != idx && !schema_is_trivially_universal(other)
                    }) {
                        continue;
                    }
                    match item_schema.kind() {
                        SchemaNodeKind::Not(inner) => negatives.push((idx, inner)),
                        _ if !schema_is_trivially_universal(item_schema) => {
                            positives.push((idx, item_schema));
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        active.remove(&schema.id());
    }

    let mut active = HashSet::new();
    for branch in branches {
        collect(branch, &mut positives, &mut negatives, &mut active);
    }
    positives.iter().any(|(pos_idx, pos)| {
        negatives
            .iter()
            .any(|(neg_idx, neg)| pos_idx == neg_idx && schemas_obviously_equivalent(pos, neg))
    })
}

/// Recognize the array tautology "all items satisfy P, or some item does not
/// satisfy P".  The `items` arm covers the empty array; the `contains: not P`
/// arm covers every non-empty counterexample.  Keep both array arms otherwise
/// unconstrained so this remains a pure partition fact.
fn any_of_items_contains_partition_is_universal(branches: &[SchemaNode]) -> bool {
    for bit in [
        JSON_TYPE_NULL,
        JSON_TYPE_BOOL,
        JSON_TYPE_NUMBER,
        JSON_TYPE_STRING,
        JSON_TYPE_OBJECT,
    ] {
        if !branches
            .iter()
            .any(|branch| schema_obviously_accepts_json_type(branch, bit))
        {
            return false;
        }
    }

    let mut all_item_preds: Vec<&SchemaNode> = Vec::new();
    let mut all_not_preds: Vec<&SchemaNode> = Vec::new();
    let mut some_preds: Vec<&SchemaNode> = Vec::new();
    let mut some_not_preds: Vec<&SchemaNode> = Vec::new();

    fn collect<'a>(
        schema: &'a SchemaNode,
        all_item_preds: &mut Vec<&'a SchemaNode>,
        all_not_preds: &mut Vec<&'a SchemaNode>,
        some_preds: &mut Vec<&'a SchemaNode>,
        some_not_preds: &mut Vec<&'a SchemaNode>,
        active: &mut HashSet<NodeId>,
    ) {
        if !active.insert(schema.id()) {
            return;
        }
        match schema.kind() {
            SchemaNodeKind::AnyOf(children) => {
                for child in children {
                    collect(
                        child,
                        all_item_preds,
                        all_not_preds,
                        some_preds,
                        some_not_preds,
                        active,
                    );
                }
            }
            SchemaNodeKind::Array {
                prefix_items,
                items,
                item_count,
                contains,
                unique_items,
                enumeration,
            } => {
                if !prefix_items.is_empty() || *unique_items || enumeration.is_some() {
                    return;
                }
                match contains {
                    None => {
                        if item_count.min() == 0
                            && item_count.max().is_none()
                            && !schema_is_trivially_universal(items)
                        {
                            match items.kind() {
                                SchemaNodeKind::Not(inner) => all_not_preds.push(inner),
                                _ => all_item_preds.push(items),
                            }
                        }
                    }
                    Some(contains) => {
                        if schema_is_trivially_universal(items)
                            && item_count.min() <= 1
                            && item_count.max().is_none()
                            && contains.count().min() <= 1
                            && contains.count().max().is_none()
                        {
                            match contains.schema.kind() {
                                SchemaNodeKind::Not(inner) => some_not_preds.push(inner),
                                _ if !schema_is_trivially_universal(&contains.schema) => {
                                    some_preds.push(&contains.schema)
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        active.remove(&schema.id());
    }

    let mut active = HashSet::new();
    for branch in branches {
        collect(
            branch,
            &mut all_item_preds,
            &mut all_not_preds,
            &mut some_preds,
            &mut some_not_preds,
            &mut active,
        );
    }
    all_item_preds.iter().any(|all_pred| {
        some_not_preds
            .iter()
            .any(|not_pred| schemas_obviously_equivalent(all_pred, not_pred))
    }) || all_not_preds.iter().any(|not_pred| {
        some_preds
            .iter()
            .any(|some_pred| schemas_obviously_equivalent(not_pred, some_pred))
    })
}

/// For `oneOf[true, X]` (or the symmetric spelling), membership is exactly
/// `not X`.  Prove a subset fits this xor when it is locally disjoint from the
/// non-universal arm, including a small conditional-disjointness case.
fn one_of_universal_arm_contains_subset(
    sub: &SchemaNode,
    branches: &[SchemaNode],
    context: &mut SubschemaCheckContext,
) -> bool {
    if branches.len() != 2 {
        return false;
    }
    for universal_idx in 0..2 {
        let universal = unwrap_singleton_applicators(&branches[universal_idx]);
        if !schema_is_trivially_universal(universal) {
            continue;
        }
        let excluded = unwrap_singleton_applicators(&branches[1 - universal_idx]);
        if schemas_definitely_disjoint_for_negation(sub, excluded, context)
            || schema_disjoint_from_conditional(sub, excluded, context)
        {
            return true;
        }
    }
    false
}

/// Conservative disjointness for a schema against a conditional.  If every
/// subset value is known to take one side of the guard, it suffices to prove it
/// disjoint from that side's branch. Missing branches are `true`, so they cannot
/// witness disjointness.
fn schema_disjoint_from_conditional(
    sub: &SchemaNode,
    conditional: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    let SchemaNodeKind::IfThenElse {
        if_schema,
        then_schema,
        else_schema,
    } = conditional.kind()
    else {
        return false;
    };

    let sub_implies_guard =
        analyze_subschema_with_context(sub, if_schema, context, ExplanationMode::VerdictOnly)
            .is_subschema;
    if sub_implies_guard {
        return then_schema.as_ref().is_some_and(|then_branch| {
            schemas_definitely_disjoint_for_negation(sub, then_branch, context)
        });
    }

    if schemas_definitely_disjoint_for_negation(sub, if_schema, context) {
        return else_schema.as_ref().is_some_and(|else_branch| {
            schemas_definitely_disjoint_for_negation(sub, else_branch, context)
        });
    }

    false
}

/// An `anyOf` is universal as soon as one branch is known universal.  The
/// generic trivial-universal recognizer intentionally stays syntactic; this
/// wrapper also recognizes the small conditional tautologies proved by
/// `conditional_is_known_universal` (for example `if: A, then: A`).
fn any_of_contains_known_universal_branch(
    branches: &[SchemaNode],
    context: &mut SubschemaCheckContext,
) -> bool {
    branches.iter().any(|branch| {
        let branch = unwrap_singleton_applicators(branch);
        if schema_is_trivially_universal(branch) {
            return true;
        }
        if let SchemaNodeKind::IfThenElse {
            if_schema,
            then_schema,
            else_schema,
        } = branch.kind()
        {
            return conditional_is_known_universal(
                if_schema,
                then_schema.as_ref(),
                else_schema.as_ref(),
                context,
            );
        }
        false
    })
}

/// Return true when an `anyOf` is syntactically universal because it contains
/// a branch `A` plus a complement branch `not B` with `B <= A`.
///
/// For every instance, either it satisfies `B` (and therefore `A`) or it does
/// not satisfy `B` (and therefore satisfies `not B`).  The inner implication
/// is delegated to the ordinary conservative subset prover; failure simply
/// means we do not recognize the cover.
fn any_of_complement_cover_is_universal(
    branches: &[SchemaNode],
    context: &mut SubschemaCheckContext,
) -> bool {
    for (not_index, branch) in branches.iter().enumerate() {
        let SchemaNodeKind::Not(excluded) = branch.kind() else {
            continue;
        };
        for (cover_index, cover) in branches.iter().enumerate() {
            if cover_index == not_index {
                continue;
            }
            if analyze_subschema_with_context(
                excluded,
                cover,
                context,
                ExplanationMode::VerdictOnly,
            )
            .is_subschema
            {
                return true;
            }
        }

        // The positive side of the partition may itself be split across
        // several finite branches rather than appearing as one syntactic
        // schema. If `B` has a finite upper bound and every live candidate of
        // `B` is accepted by some sibling branch, then those siblings cover B;
        // together with `not B` the union is universal. We deliberately use an
        // upper bound plus definite concrete-value checks, so unsupported
        // keywords only make this fail closed.
        if let Some(values) = finite_schema_value_superset(excluded)
            && values.iter().all(|value| {
                context.schema_definitely_rejects_value(excluded, value)
                    || branches.iter().enumerate().any(|(cover_index, cover)| {
                        cover_index != not_index && context.superset_contains_value(cover, value)
                    })
            })
        {
            return true;
        }
    }
    false
}

/// Prove an `anyOf` is universal when it contains the complement of a
/// syntactic union plus siblings covering every union arm.  Overlap among the
/// positive siblings is harmless for `anyOf`; for any instance, either it is
/// outside the excluded union (so the complement branch accepts it) or it is
/// inside one union arm that is contained by a sibling.
fn any_of_complement_union_cover_is_universal(
    branches: &[SchemaNode],
    context: &mut SubschemaCheckContext,
) -> bool {
    for (not_index, branch) in branches.iter().enumerate() {
        let SchemaNodeKind::Not(excluded_union) = branch.kind() else {
            continue;
        };
        let union_children = match excluded_union.kind() {
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children) => children,
            _ => continue,
        };

        if union_children.iter().all(|child| {
            branches.iter().enumerate().any(|(index, cover)| {
                index != not_index
                    && analyze_subschema_with_context(
                        child,
                        cover,
                        context,
                        ExplanationMode::VerdictOnly,
                    )
                    .is_subschema
            })
        }) {
            return true;
        }
    }
    false
}

/// Prove a subset relation for a `oneOf` made only of complements.
/// With two or more complement branches, any accepted value must be excluded
/// by at least one inner schema (otherwise it would match every complement
/// branch, not exactly one). Therefore the whole oneOf is contained in the
/// union of the excluded inners. If each inner is contained by the target, the
/// complement-only oneOf is contained as well.
fn complement_only_oneof_subset_of(
    branches: &[SchemaNode],
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    if branches.len() < 2 {
        return false;
    }
    let mut inners = Vec::with_capacity(branches.len());
    for branch in branches {
        let SchemaNodeKind::Not(inner) = branch.kind() else {
            return false;
        };
        inners.push(inner);
    }
    // For a branch `not A_j` to be the *only* matching complement, the
    // instance must satisfy every other inner `A_i`.  Thus it is enough that,
    // for each omitted position j, at least one of the remaining inners is
    // known to be contained by the target.  With two branches this reduces to
    // the familiar requirement that both inners imply the target; with three
    // or more branches, two target-contained inners already cover every xor
    // arm.  Keep the proof at the level of whole inners so we do not assume
    // anything about intersections we cannot model.
    let contained: Vec<bool> = inners
        .iter()
        .map(|inner| {
            analyze_subschema_with_context(inner, sup, context, ExplanationMode::VerdictOnly)
                .is_subschema
        })
        .collect();
    (0..inners.len()).all(|omitted| {
        if contained
            .iter()
            .enumerate()
            .any(|(index, ok)| index != omitted && *ok)
        {
            return true;
        }

        // The xor arm for `not A_omitted` also vanishes when two of the
        // required remaining inners are definitely disjoint.  This catches
        // empty complement xors such as not-string/not-number/not-boolean
        // without needing a general intersection solver.
        for (left_index, &left) in inners.iter().enumerate() {
            if left_index == omitted {
                continue;
            }
            let left_mask = possible_json_type_mask(left);
            if left_mask == 0 {
                return true;
            }
            for (right_index, &right) in inners.iter().enumerate().skip(left_index + 1) {
                if right_index == omitted {
                    continue;
                }
                if possible_json_type_mask(right) == 0
                    || schemas_definitely_disjoint_for_partition(left, left_mask, right, context)
                {
                    return true;
                }
            }
        }
        false
    })
}

/// Try the mixed-xor disjoint reduction against a target directly, or against
/// one negated arm of a target union.
/// Prove `oneOf[A, B] <= not T` when one positive arm is contained in the
/// other.  Then the xor is the larger arm with the smaller arm removed.
fn comparable_oneof_difference_subset_of_negation(
    branches: &[SchemaNode],
    excluded: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    if branches.len() != 2 {
        return false;
    }

    for (small_idx, large_idx) in [(0_usize, 1_usize), (1, 0)] {
        let small = unwrap_singleton_applicators(&branches[small_idx]);
        let large = unwrap_singleton_applicators(&branches[large_idx]);
        if matches!(small.kind(), SchemaNodeKind::Not(_))
            || matches!(large.kind(), SchemaNodeKind::Not(_))
        {
            continue;
        }
        if !analyze_subschema_with_context(small, large, context, ExplanationMode::VerdictOnly)
            .is_subschema
        {
            continue;
        }

        let excluded = unwrap_singleton_applicators(excluded);
        let mut pieces: Vec<&SchemaNode> = Vec::new();
        match excluded.kind() {
            SchemaNodeKind::AnyOf(children) => pieces.extend(children.iter()),
            _ => pieces.push(excluded),
        }

        let mut ok = true;
        for piece in pieces {
            let piece = unwrap_singleton_applicators(piece);
            let inside_removed =
                analyze_subschema_with_context(piece, small, context, ExplanationMode::VerdictOnly)
                    .is_subschema;
            if inside_removed {
                continue;
            }
            if !schemas_definitely_disjoint_for_negation(piece, large, context) {
                ok = false;
                break;
            }
        }
        if ok {
            return true;
        }
    }
    false
}

fn mixed_oneof_disjoint_complement_subset_of_target(
    branches: &[SchemaNode],
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    fn inner(
        branches: &[SchemaNode],
        sup: &SchemaNode,
        context: &mut SubschemaCheckContext,
        active: &mut HashSet<NodeId>,
    ) -> bool {
        let sup = unwrap_singleton_applicators(sup);
        if !active.insert(sup.id()) {
            return false;
        }
        let result = match sup.kind() {
            SchemaNodeKind::BoolSchema(true) | SchemaNodeKind::Any => true,
            SchemaNodeKind::Not(excluded) => {
                mixed_oneof_disjoint_complement_subset_of_negation(branches, excluded, context)
            }
            // It is enough to fit one union arm.
            SchemaNodeKind::AnyOf(children) => {
                if mixed_oneof_disjoint_complement_subset_of_union_target(
                    branches, children, context,
                ) {
                    true
                } else {
                    let mut ok = false;
                    for child in children {
                        if inner(branches, child, context, active) {
                            ok = true;
                            break;
                        }
                    }
                    ok
                }
            }
            // To fit an intersection, fit every conjunct.
            SchemaNodeKind::AllOf(children) => {
                let mut ok = true;
                for child in children {
                    if !inner(branches, child, context, active) {
                        ok = false;
                        break;
                    }
                }
                ok
            }
            SchemaNodeKind::OneOf(target_children) => {
                mixed_oneof_disjoint_complement_subset_of_mixed_target(
                    branches,
                    target_children,
                    context,
                )
            }
            _ => false,
        };
        active.remove(&sup.id());
        result
    }

    inner(branches, sup, context, &mut HashSet::new())
}

/// Prove a disjoint mixed xor fits a union containing a negated finite arm.
///
/// For subset `not(A ∪ B)`, a target union with an arm `not D` can only reject
/// values from D that are not accepted by another union arm.  Enumerate a
/// finite over-approximation of D and require each such remaining value to be
/// covered by A or B.
fn mixed_oneof_disjoint_complement_subset_of_union_target(
    sub_branches: &[SchemaNode],
    target_children: &[SchemaNode],
    context: &mut SubschemaCheckContext,
) -> bool {
    if sub_branches.len() != 2 || target_children.is_empty() {
        return false;
    }

    for sub_neg_idx in 0..2 {
        let sub_pos_idx = 1 - sub_neg_idx;
        let SchemaNodeKind::Not(sub_b_raw) = sub_branches[sub_neg_idx].kind() else {
            continue;
        };
        let sub_a = unwrap_singleton_applicators(&sub_branches[sub_pos_idx]);
        let sub_b = unwrap_singleton_applicators(sub_b_raw);
        if !schemas_definitely_disjoint_for_negation(sub_a, sub_b, context) {
            continue;
        }

        for (neg_index, child) in target_children.iter().enumerate() {
            let SchemaNodeKind::Not(d_raw) = unwrap_singleton_applicators(child).kind() else {
                continue;
            };
            let d = unwrap_singleton_applicators(d_raw);
            let Some(values) = finite_schema_value_superset(d) else {
                continue;
            };
            let mut covered = true;
            'values: for value in values {
                for (index, other) in target_children.iter().enumerate() {
                    if index == neg_index {
                        continue;
                    }
                    if context.superset_contains_value(other, &value) {
                        continue 'values;
                    }
                }
                if !(context.superset_contains_value(sub_a, &value)
                    || context.superset_contains_value(sub_b, &value))
                {
                    covered = false;
                    break;
                }
            }
            if covered {
                return true;
            }
        }
    }
    false
}

/// Prove a disjoint mixed xor is contained by a comparable mixed-xor target.
///
/// For disjoint A/B, the subset is `not(A ∪ B)`.  A target `oneOf[C, not D]`
/// with C <= D is `C ∪ not D`, i.e. it rejects only `D \ C`.  If D has a
/// finite over-approximation and every value not definitely in C is covered by
/// A or B, then anything outside A ∪ B must satisfy the target.
fn mixed_oneof_disjoint_complement_subset_of_mixed_target(
    sub_branches: &[SchemaNode],
    target_branches: &[SchemaNode],
    context: &mut SubschemaCheckContext,
) -> bool {
    if sub_branches.len() != 2 || target_branches.len() != 2 {
        return false;
    }

    for sub_neg_idx in 0..2 {
        let sub_pos_idx = 1 - sub_neg_idx;
        let SchemaNodeKind::Not(sub_b_raw) = sub_branches[sub_neg_idx].kind() else {
            continue;
        };
        let sub_a = unwrap_singleton_applicators(&sub_branches[sub_pos_idx]);
        let sub_b = unwrap_singleton_applicators(sub_b_raw);
        if !schemas_definitely_disjoint_for_negation(sub_a, sub_b, context) {
            continue;
        }

        for tgt_neg_idx in 0..2 {
            let tgt_pos_idx = 1 - tgt_neg_idx;
            let SchemaNodeKind::Not(tgt_d_raw) = target_branches[tgt_neg_idx].kind() else {
                continue;
            };
            let tgt_c = unwrap_singleton_applicators(&target_branches[tgt_pos_idx]);
            let tgt_d = unwrap_singleton_applicators(tgt_d_raw);
            if !analyze_subschema_with_context(tgt_c, tgt_d, context, ExplanationMode::VerdictOnly)
                .is_subschema
            {
                continue;
            }
            let Some(values) = finite_schema_value_superset(tgt_d) else {
                continue;
            };
            let mut covered = true;
            for value in values {
                if context.superset_contains_value(tgt_c, &value) {
                    continue;
                }
                if !(context.superset_contains_value(sub_a, &value)
                    || context.superset_contains_value(sub_b, &value))
                {
                    covered = false;
                    break;
                }
            }
            if covered {
                return true;
            }
        }
    }
    false
}

/// Prove `oneOf[A, not B] <= not T` for the disjoint A/B case.
///
/// If A and B are disjoint, the xor accepts exactly values outside both A and
/// B.  Therefore it is contained by `not T` whenever T is contained by A or B.
fn mixed_oneof_disjoint_complement_subset_of_negation(
    branches: &[SchemaNode],
    excluded: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    if branches.len() != 2 {
        return false;
    }

    for neg_idx in 0..2 {
        let pos_idx = 1 - neg_idx;
        let SchemaNodeKind::Not(inner_b) = branches[neg_idx].kind() else {
            continue;
        };
        let a = unwrap_singleton_applicators(&branches[pos_idx]);
        let b = unwrap_singleton_applicators(inner_b);
        if !schemas_definitely_disjoint_for_negation(a, b, context) {
            continue;
        }
        let excluded = unwrap_singleton_applicators(excluded);
        if analyze_subschema_with_context(excluded, a, context, ExplanationMode::VerdictOnly)
            .is_subschema
            || analyze_subschema_with_context(excluded, b, context, ExplanationMode::VerdictOnly)
                .is_subschema
        {
            return true;
        }
    }
    false
}

/// Prove a subset relation for the complement of a two-arm xor where one arm
/// is itself a complement: `not(oneOf[A, not B])`.
///
/// Semantically this is `(A && !B) || (!A && B)`, i.e. the symmetric
/// difference of `A` and `B`, so it is always contained in `A || B`. We first
/// try that general union bound, then fall back to smaller comparable/disjoint
/// normal forms. In each case we discharge the remaining containment(s) with
/// the ordinary prover. This catches generated xor/complement partitions without treating
/// arbitrary negated oneOf as a union.
fn negated_oneof_complement_pair_subset_of(
    branches: &[SchemaNode],
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    if branches.len() != 2 {
        return false;
    }

    for not_index in 0..2 {
        let SchemaNodeKind::Not(inner_b) = branches[not_index].kind() else {
            continue;
        };
        let a = &branches[1 - not_index];
        let b = inner_b;

        // In full generality this negated xor is the symmetric difference
        // `A xor B`, hence it is always contained in `A || B`. If both sides
        // independently imply the target, no relationship between A and B is
        // needed.
        let a_le_sup =
            analyze_subschema_with_context(a, sup, context, ExplanationMode::VerdictOnly)
                .is_subschema;
        let b_le_sup =
            analyze_subschema_with_context(b, sup, context, ExplanationMode::VerdictOnly)
                .is_subschema;
        if a_le_sup && b_le_sup {
            return true;
        }

        // If both sides are finite, evaluate the exact symmetric-difference
        // candidates concretely. Unknown evaluator results fail closed.
        if let (Some(mut a_values), Some(b_values)) = (
            finite_schema_value_superset(a),
            finite_schema_value_superset(b),
        ) {
            for value in b_values {
                if !a_values
                    .iter()
                    .any(|existing| json_values_equal(existing, &value))
                {
                    a_values.push(value);
                }
            }
            let mut all_known = true;
            let mut all_live_in_sup = true;
            for value in &a_values {
                let a_accepts = context.superset_contains_value(a, value);
                let a_rejects = context.schema_definitely_rejects_value(a, value);
                let b_accepts = context.superset_contains_value(b, value);
                let b_rejects = context.schema_definitely_rejects_value(b, value);
                if (!a_accepts && !a_rejects) || (!b_accepts && !b_rejects) {
                    all_known = false;
                    break;
                }
                if a_accepts != b_accepts && !context.superset_contains_value(sup, value) {
                    all_live_in_sup = false;
                    break;
                }
            }
            if all_known && all_live_in_sup {
                return true;
            }
        }

        let a_le_b = analyze_subschema_with_context(a, b, context, ExplanationMode::VerdictOnly)
            .is_subschema;
        if a_le_b
            && analyze_subschema_with_context(b, sup, context, ExplanationMode::VerdictOnly)
                .is_subschema
        {
            return true;
        }

        let b_le_a = analyze_subschema_with_context(b, a, context, ExplanationMode::VerdictOnly)
            .is_subschema;
        if b_le_a
            && analyze_subschema_with_context(a, sup, context, ExplanationMode::VerdictOnly)
                .is_subschema
        {
            return true;
        }

        let a_mask = possible_json_type_mask(a);
        if a_mask == 0 {
            if analyze_subschema_with_context(b, sup, context, ExplanationMode::VerdictOnly)
                .is_subschema
            {
                return true;
            }
        } else if schemas_definitely_disjoint_for_partition(a, a_mask, b, context)
            && analyze_subschema_with_context(a, sup, context, ExplanationMode::VerdictOnly)
                .is_subschema
            && analyze_subschema_with_context(b, sup, context, ExplanationMode::VerdictOnly)
                .is_subschema
        {
            return true;
        }
    }
    false
}

/// For `not(oneOf[P, Q])`, prove the xor excludes an infinite comparable gap
/// of a mixed-xor target. If the gap is `Domain \ Kept`, one xor arm covering
/// all of Domain and the other intersecting Domain only inside Kept suffices.
fn negated_xor_covers_mixed_comparable_gap(
    branches: &[SchemaNode],
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    fn finite_intersection_subset_of(
        finite_side: &SchemaNode,
        domain: &SchemaNode,
        kept: &SchemaNode,
        context: &mut SubschemaCheckContext,
    ) -> bool {
        let Some(values) = finite_schema_value_superset(finite_side) else {
            return false;
        };
        for value in values {
            if context.schema_definitely_rejects_value(finite_side, &value)
                || context.schema_definitely_rejects_value(domain, &value)
            {
                continue;
            }
            // If this value may lie in both sides, it must definitely be kept.
            if !context.superset_contains_value(kept, &value) {
                return false;
            }
        }
        true
    }

    if branches.len() != 2 {
        return false;
    }
    let p = unwrap_singleton_applicators(&branches[0]);
    let q = unwrap_singleton_applicators(&branches[1]);
    let SchemaNodeKind::OneOf(target_branches) = unwrap_singleton_applicators(sup).kind() else {
        return false;
    };
    if target_branches.len() != 2 {
        return false;
    }
    for neg_index in 0..2 {
        let neg_arm = unwrap_singleton_applicators(&target_branches[neg_index]);
        let SchemaNodeKind::Not(d_raw) = neg_arm.kind() else {
            continue;
        };
        let d = unwrap_singleton_applicators(d_raw);
        let c = unwrap_singleton_applicators(&target_branches[1 - neg_index]);
        for (domain, kept) in [(c, d), (d, c)] {
            if !analyze_subschema_with_context(kept, domain, context, ExplanationMode::VerdictOnly)
                .is_subschema
            {
                continue;
            }
            if analyze_subschema_with_context(domain, q, context, ExplanationMode::VerdictOnly)
                .is_subschema
                && finite_intersection_subset_of(p, domain, kept, context)
            {
                return true;
            }
            if analyze_subschema_with_context(domain, p, context, ExplanationMode::VerdictOnly)
                .is_subschema
                && finite_intersection_subset_of(q, domain, kept, context)
            {
                return true;
            }
        }
    }
    false
}

/// General finite-gap check for `not X <= oneOf[C, not D]` when C and D are
/// comparable. The mixed xor rejects exactly the finite difference between
/// the larger and smaller side; prove X definitely accepts every such value.
fn negated_schema_excludes_mixed_finite_gap(
    excluded: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    fn gap_rejected_by_negation(
        domain: &SchemaNode,
        kept: &SchemaNode,
        excluded: &SchemaNode,
        context: &mut SubschemaCheckContext,
    ) -> bool {
        let Some(values) = finite_schema_value_superset(domain) else {
            return false;
        };
        for value in values {
            if context.superset_contains_value(kept, &value)
                || context.schema_definitely_rejects_value(domain, &value)
            {
                continue;
            }
            if !context.superset_contains_value(excluded, &value) {
                return false;
            }
        }
        true
    }

    let SchemaNodeKind::OneOf(target_branches) = unwrap_singleton_applicators(sup).kind() else {
        return false;
    };
    if target_branches.len() != 2 {
        return false;
    }
    for neg_index in 0..2 {
        let neg_arm = unwrap_singleton_applicators(&target_branches[neg_index]);
        let SchemaNodeKind::Not(d_raw) = neg_arm.kind() else {
            continue;
        };
        let d = unwrap_singleton_applicators(d_raw);
        let c = unwrap_singleton_applicators(&target_branches[1 - neg_index]);
        if analyze_subschema_with_context(d, c, context, ExplanationMode::VerdictOnly).is_subschema
            && gap_rejected_by_negation(c, d, excluded, context)
        {
            return true;
        }
        if analyze_subschema_with_context(c, d, context, ExplanationMode::VerdictOnly).is_subschema
            && gap_rejected_by_negation(d, c, excluded, context)
        {
            return true;
        }
    }
    false
}

/// Prove `not(anyOf[arms...]) <= oneOf[C, not D]` in the finite remainder
/// case. If D <= C, the target rejects exactly C \ D; every such finite
/// candidate must be covered by one of the excluded union arms.
fn negated_union_subset_of_mixed_difference(
    arms: &[SchemaNode],
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    fn gap_covered(
        domain: &SchemaNode,
        kept: &SchemaNode,
        arms: &[SchemaNode],
        context: &mut SubschemaCheckContext,
    ) -> bool {
        let Some(values) = finite_schema_value_superset(domain) else {
            return false;
        };
        'values: for value in values {
            if context.superset_contains_value(kept, &value)
                || context.schema_definitely_rejects_value(domain, &value)
            {
                continue;
            }
            for arm in arms {
                if context.superset_contains_value(arm, &value) {
                    continue 'values;
                }
            }
            return false;
        }
        true
    }

    if arms.is_empty() {
        return false;
    }
    let SchemaNodeKind::OneOf(target_branches) = unwrap_singleton_applicators(sup).kind() else {
        return false;
    };
    if target_branches.len() != 2 {
        return false;
    }
    for neg_index in 0..2 {
        let neg_arm = unwrap_singleton_applicators(&target_branches[neg_index]);
        let SchemaNodeKind::Not(d_raw) = neg_arm.kind() else {
            continue;
        };
        let d = unwrap_singleton_applicators(d_raw);
        let c = unwrap_singleton_applicators(&target_branches[1 - neg_index]);
        // If D <= C, the xor rejects C \ D. If C <= D, it rejects D \ C.
        if analyze_subschema_with_context(d, c, context, ExplanationMode::VerdictOnly).is_subschema
            && gap_covered(c, d, arms, context)
        {
            return true;
        }
        if analyze_subschema_with_context(c, d, context, ExplanationMode::VerdictOnly).is_subschema
            && gap_covered(d, c, arms, context)
        {
            return true;
        }
    }
    false
}

/// Handle `not(oneOf[not A, not B]) <= oneOf[C, not D]` for the common
/// finite-difference case.  When A and B are disjoint, the left side is
/// `not(A || B)`.  When D is contained in C, the mixed-xor target is
/// `D || not C`, i.e. it rejects only `C \ D`.  Enumerating a finite
/// over-approximation of C and requiring every possible remainder value to be
/// covered by A or B is a conservative proof of containment.
fn negated_complement_pair_subset_of_mixed_difference(
    branches: &[SchemaNode],
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    if branches.len() != 2 {
        return false;
    }

    let left0 = unwrap_singleton_applicators(&branches[0]);
    let left1 = unwrap_singleton_applicators(&branches[1]);
    // Two shapes reduce to the complement of a disjoint union: a negated xor
    // of disjoint positive arms, and a negated xor of two complemented arms
    // whose inners are disjoint (`xor(!A, !B) == A || B`).
    let mut candidates = vec![(left0, left1)];
    if let (SchemaNodeKind::Not(a_raw), SchemaNodeKind::Not(b_raw)) = (left0.kind(), left1.kind()) {
        candidates.push((
            unwrap_singleton_applicators(a_raw),
            unwrap_singleton_applicators(b_raw),
        ));
    }

    let SchemaNodeKind::OneOf(target_branches) = unwrap_singleton_applicators(sup).kind() else {
        return false;
    };
    if target_branches.len() != 2 {
        return false;
    }

    for (a, b) in candidates {
        if !schemas_definitely_disjoint_for_negation(a, b, context) {
            continue;
        }

        for neg_index in 0..2 {
            let neg_arm = unwrap_singleton_applicators(&target_branches[neg_index]);
            let SchemaNodeKind::Not(d_raw) = neg_arm.kind() else {
                continue;
            };
            let d = unwrap_singleton_applicators(d_raw);
            let c = unwrap_singleton_applicators(&target_branches[1 - neg_index]);

            // Restrict to the simple identity `oneOf[C, not D] == D || not C`.
            if !analyze_subschema_with_context(d, c, context, ExplanationMode::VerdictOnly)
                .is_subschema
            {
                continue;
            }
            let Some(values) = finite_schema_value_superset(c) else {
                continue;
            };

            let mut ok = true;
            for value in values {
                if context.superset_contains_value(d, &value) {
                    continue;
                }
                if context.schema_definitely_rejects_value(c, &value) {
                    continue;
                }
                if !(context.superset_contains_value(a, &value)
                    || context.superset_contains_value(b, &value))
                {
                    ok = false;
                    break;
                }
            }
            if ok {
                return true;
            }
        }
    }
    false
}

/// Prove the generic two-arm identity for a complemented xor: values accepted
/// by both arms, or by neither arm, are accepted by `not(oneOf[A, B])`.
fn negated_two_arm_oneof_contains(
    sub: &SchemaNode,
    branches: &[SchemaNode],
    context: &mut SubschemaCheckContext,
) -> bool {
    if branches.len() != 2 {
        return false;
    }
    let a = &branches[0];
    let b = &branches[1];

    let in_a =
        analyze_subschema_with_context(sub, a, context, ExplanationMode::VerdictOnly).is_subschema;
    let in_b =
        analyze_subschema_with_context(sub, b, context, ExplanationMode::VerdictOnly).is_subschema;
    if in_a && in_b {
        return true;
    }

    // The "neither" side is also safe when we can prove disjointness from both
    // arms.  Keep this as a fallback because disjointness may recurse through
    // finite/value reasoning and is a little more expensive than implication.
    !in_a
        && !in_b
        && schemas_definitely_disjoint_for_negation(sub, a, context)
        && schemas_definitely_disjoint_for_negation(sub, b, context)
}

/// Prove a subset is contained by `not(oneOf[not A, B])` in the simple
/// disjoint case.  If A and B cannot overlap, the complemented xor is exactly
/// `A ∪ B`; therefore any schema known to fit either side is accepted.
fn negated_oneof_complement_pair_contains(
    sub: &SchemaNode,
    branches: &[SchemaNode],
    context: &mut SubschemaCheckContext,
) -> bool {
    if branches.len() != 2 {
        return false;
    }

    for neg_idx in 0..2 {
        let pos_idx = 1 - neg_idx;
        let SchemaNodeKind::Not(a) = branches[neg_idx].kind() else {
            continue;
        };
        let a = unwrap_singleton_applicators(a);
        let b = &branches[pos_idx];
        if !schemas_definitely_disjoint_for_negation(a, b, context) {
            continue;
        }
        if analyze_subschema_with_context(sub, a, context, ExplanationMode::VerdictOnly)
            .is_subschema
            || analyze_subschema_with_context(sub, b, context, ExplanationMode::VerdictOnly)
                .is_subschema
        {
            return true;
        }
    }
    false
}

/// Return true for the exact degenerate xor `oneOf: [A, A]`.
///
/// This is intentionally syntactic (plus the tiny equivalence fast path) so it
/// cannot turn overlapping-but-not-identical branches into an empty language.
fn one_of_pair_is_syntactically_empty(branches: &[SchemaNode]) -> bool {
    if branches.len() != 2 {
        return false;
    }
    (branches[0] == branches[1] || schemas_obviously_equivalent(&branches[0], &branches[1]))
        || (schema_is_locally_empty_for_finite_enumeration(&branches[0])
            && schema_is_locally_empty_for_finite_enumeration(&branches[1]))
}

/// If all but one branch of a oneOf are locally empty, the xor is exactly the
/// remaining branch; if every branch is empty, the xor is empty.  Returns
/// `Some(None)` for the all-empty case and `Some(Some(branch))` for one live
/// branch. Wider live sets are left untouched.
fn one_of_trivial_live_branch(branches: &[SchemaNode]) -> Option<Option<&SchemaNode>> {
    if branches.is_empty() {
        return Some(None);
    }
    let mut live: Option<&SchemaNode> = None;
    for branch in branches {
        if schema_is_locally_empty_for_finite_enumeration(branch) {
            continue;
        }
        if live.is_some() {
            return None;
        }
        live = Some(branch);
    }
    Some(live)
}

/// For a two-arm xor with one definitely empty arm, return the live arm.
///
/// Restrict this to local, non-recursive emptiness facts. A false arm has no
/// effect on xor parity, so `oneOf(false, A)` is exactly `A`.
fn one_of_pair_single_live_branch(branches: &[SchemaNode]) -> Option<&SchemaNode> {
    if branches.len() != 2 {
        return None;
    }
    let left_empty = schema_is_locally_empty_for_finite_enumeration(&branches[0]);
    let right_empty = schema_is_locally_empty_for_finite_enumeration(&branches[1]);
    match (left_empty, right_empty) {
        (true, false) => Some(&branches[1]),
        (false, true) => Some(&branches[0]),
        // If both are empty the xor is empty; the caller's empty-pair fast path
        // handles syntactically equal false arms, and otherwise falling through
        // remains conservative.
        _ => None,
    }
}

/// Prove the exact two-arm identity `oneOf: [A, not A]` (up to conservative
/// mutual-subset equivalence) is universal.  This catches infinite partitions
/// such as strings vs non-strings without treating arbitrary `oneOf` as a
/// union: with exactly two arms, equivalence of the positive arm and the
/// negated arm's exclusion guarantees every value matches exactly one arm.
fn one_of_complement_pair_is_universal(
    branches: &[SchemaNode],
    context: &mut SubschemaCheckContext,
) -> bool {
    if branches.len() != 2 {
        return false;
    }

    for not_index in 0..2 {
        let SchemaNodeKind::Not(excluded) = branches[not_index].kind() else {
            continue;
        };
        let positive = &branches[1 - not_index];
        let positive_implies_excluded = analyze_subschema_with_context(
            positive,
            excluded,
            context,
            ExplanationMode::VerdictOnly,
        )
        .is_subschema;
        if !positive_implies_excluded {
            continue;
        }
        let excluded_implies_positive = analyze_subschema_with_context(
            excluded,
            positive,
            context,
            ExplanationMode::VerdictOnly,
        )
        .is_subschema;
        if excluded_implies_positive {
            return true;
        }
    }
    false
}

/// Prove a `oneOf` is universal when it consists of disjoint positive
/// siblings plus the complement of their union.  This recognizes generated
/// partitions such as `[string, number, not(anyOf[string, number])]` without
/// assuming arbitrary `oneOf` behaves like `anyOf`.
fn one_of_complement_union_partition_is_universal(
    branches: &[SchemaNode],
    context: &mut SubschemaCheckContext,
) -> bool {
    for (not_index, branch) in branches.iter().enumerate() {
        let SchemaNodeKind::Not(excluded_union) = branch.kind() else {
            continue;
        };
        let union_children = match excluded_union.kind() {
            SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children) => children,
            _ => continue,
        };

        let sibling_indices: Vec<usize> = (0..branches.len())
            .filter(|&index| index != not_index)
            .collect();
        if sibling_indices.is_empty() {
            continue;
        }

        // Outside the excluded union, only the complement branch may match.
        if !sibling_indices.iter().all(|&index| {
            analyze_subschema_with_context(
                &branches[index],
                excluded_union,
                context,
                ExplanationMode::VerdictOnly,
            )
            .is_subschema
        }) {
            continue;
        }

        // Inside the union, at most one positive sibling may match.
        let pairwise_disjoint = sibling_indices
            .iter()
            .enumerate()
            .all(|(pos, &left_index)| {
                sibling_indices[pos + 1..].iter().all(|&right_index| {
                    let left = &branches[left_index];
                    let right = &branches[right_index];
                    let mask = possible_json_type_mask(left);
                    mask == 0
                        || schemas_definitely_disjoint_for_partition(left, mask, right, context)
                })
            });
        if !pairwise_disjoint {
            continue;
        }

        // Every arm of the excluded union must flow into some positive
        // sibling, giving coverage of the inside region.
        if union_children.iter().all(|child| {
            sibling_indices.iter().any(|&index| {
                analyze_subschema_with_context(
                    child,
                    &branches[index],
                    context,
                    ExplanationMode::VerdictOnly,
                )
                .is_subschema
            })
        }) {
            return true;
        }
    }
    false
}

/// Prove a finite complement partition written as `oneOf` is universal.
///
/// For a branch `not B`, require every sibling to be a subset of B, then use a
/// finite upper bound for B to prove each live B-candidate is accepted by
/// exactly one sibling (one definite accept, definite rejection by the rest).
/// Values outside B then satisfy only the complement branch, so the `oneOf`
/// accepts every instance.  All checks are conservative: unsupported concrete
/// evaluation or subset facts simply make the proof fail.
fn one_of_finite_complement_partition_is_universal(
    branches: &[SchemaNode],
    context: &mut SubschemaCheckContext,
) -> bool {
    for (not_index, branch) in branches.iter().enumerate() {
        let SchemaNodeKind::Not(excluded) = branch.kind() else {
            continue;
        };
        let Some(values) = finite_schema_value_superset(excluded) else {
            continue;
        };

        let sibling_indices: Vec<usize> = (0..branches.len())
            .filter(|&index| index != not_index)
            .collect();
        if sibling_indices.is_empty() {
            continue;
        }

        // Keep complement branch exclusive: no sibling may accept a value
        // outside B. Delegate that implication to the conservative subset
        // prover rather than trying to reason from concrete samples.
        if !sibling_indices.iter().all(|&index| {
            analyze_subschema_with_context(
                &branches[index],
                excluded,
                context,
                ExplanationMode::VerdictOnly,
            )
            .is_subschema
        }) {
            continue;
        }

        let partitions_finite_side = values.iter().all(|value| {
            if context.schema_definitely_rejects_value(excluded, value) {
                return true;
            }

            let mut definite_accepts = 0usize;
            for &index in &sibling_indices {
                let sibling = &branches[index];
                if context.superset_contains_value(sibling, value) {
                    definite_accepts += 1;
                    if definite_accepts > 1 {
                        return false;
                    }
                } else if !context.schema_definitely_rejects_value(sibling, value) {
                    // Unknown membership could hide either a coverage gap or
                    // an overlap, so do not claim an exact oneOf partition.
                    return false;
                }
            }
            definite_accepts == 1
        });

        if partitions_finite_side {
            return true;
        }
    }
    false
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

// Whole-type recognizers used by the lightweight union/complement shortcuts below.

// If a negated union explicitly contains whole-type arms for every type except
// a small remainder, then the negation can only produce values in that
// remainder.  When the target accepts each remaining type wholesale, this is a
// sound subset proof.  This is especially useful for parsed type-specific
// assertions without an explicit `type` (e.g. `{ "maximum": 1 }` lowers to a
// union of all non-number types plus the bounded-number arm; negating it
// forces a number).

fn complement_u64_count_halfline(range: CountRange<u64>) -> Option<CountRange<u64>> {
    match (range.min(), range.max()) {
        (0, None) => None,
        (min, None) => min
            .checked_sub(1)
            .and_then(|max| CountRange::new(0, Some(max))),
        (0, Some(max)) => max.checked_add(1).map(CountRange::unbounded_from),
        (_, Some(_)) => None,
    }
}

fn complement_usize_count_halfline(range: CountRange<usize>) -> Option<CountRange<usize>> {
    match (range.min(), range.max()) {
        (0, None) => None, // complement is empty; callers handle as vacuous true
        (min, None) => min
            .checked_sub(1)
            .and_then(|max| CountRange::new(0, Some(max))),
        (0, Some(max)) => max.checked_add(1).map(CountRange::unbounded_from),
        (_, Some(_)) => None,
    }
}

/// Recognize negated untyped `minItems`/`maxItems` half-lines.
fn negated_untyped_array_count_halfline_subset_of(
    branches: &[SchemaNode],
    sup: &SchemaNode,
) -> bool {
    for bit in [
        JSON_TYPE_NULL,
        JSON_TYPE_BOOL,
        JSON_TYPE_NUMBER,
        JSON_TYPE_STRING,
        JSON_TYPE_OBJECT,
    ] {
        if !branches
            .iter()
            .any(|b| schema_obviously_accepts_json_type(b, bit))
        {
            return false;
        }
    }
    if !array_schema_is_plain_count(sup) {
        return false;
    }
    let SchemaNodeKind::Array {
        item_count: sup_count,
        ..
    } = sup.kind()
    else {
        return false;
    };
    for branch in branches {
        if !array_schema_is_plain_count(branch) {
            continue;
        }
        let SchemaNodeKind::Array { item_count, .. } = branch.kind() else {
            continue;
        };
        if item_count.min() == 0 && item_count.max().is_none() {
            return true; // excluded union is universal for arrays too; complement empty
        }
        if let Some(complement) = complement_u64_count_halfline(*item_count)
            && sup_count.contains_range(complement)
        {
            return true;
        }
    }
    false
}

/// Recognize negated untyped `minProperties`/`maxProperties` half-lines.
fn negated_untyped_object_count_halfline_subset_of(
    branches: &[SchemaNode],
    sup: &SchemaNode,
) -> bool {
    for bit in [
        JSON_TYPE_NULL,
        JSON_TYPE_BOOL,
        JSON_TYPE_NUMBER,
        JSON_TYPE_STRING,
        JSON_TYPE_ARRAY,
    ] {
        if !branches
            .iter()
            .any(|b| schema_obviously_accepts_json_type(b, bit))
        {
            return false;
        }
    }
    if !object_schema_is_plain_count(sup) {
        return false;
    }
    let SchemaNodeKind::Object {
        property_count: sup_count,
        ..
    } = sup.kind()
    else {
        return false;
    };
    for branch in branches {
        if !object_schema_is_plain_count(branch) {
            continue;
        }
        let SchemaNodeKind::Object { property_count, .. } = branch.kind() else {
            continue;
        };
        if property_count.min() == 0 && property_count.max().is_none() {
            return true;
        }
        if let Some(complement) = complement_usize_count_halfline(*property_count)
            && sup_count.contains_range(complement)
        {
            return true;
        }
    }
    false
}

/// Recognize the complement of a canonicalized untyped string length half-line.
fn negated_untyped_string_length_halfline_subset_of(
    branches: &[SchemaNode],
    sup: &SchemaNode,
) -> bool {
    for bit in [
        JSON_TYPE_NULL,
        JSON_TYPE_BOOL,
        JSON_TYPE_NUMBER,
        JSON_TYPE_ARRAY,
        JSON_TYPE_OBJECT,
    ] {
        if !branches
            .iter()
            .any(|branch| schema_obviously_accepts_json_type(branch, bit))
        {
            return false;
        }
    }

    let SchemaNodeKind::String {
        length: sup_length,
        pattern: None,
        format: None,
        enumeration: None,
    } = sup.kind()
    else {
        return false;
    };

    for branch in branches {
        let SchemaNodeKind::String {
            length,
            pattern: None,
            format: None,
            enumeration: None,
        } = branch.kind()
        else {
            continue;
        };

        let complement = match (length.min(), length.max()) {
            // Excluding strings of length >= min leaves 0..min-1.  min=0 is
            // the universal string branch, whose complement is empty (vacuous).
            (0, None) => return true,
            (min, None) => CountRange::new(0_u64, min.checked_sub(1)),
            // Excluding strings of length <= max leaves max+1..
            (0, Some(max)) => max.checked_add(1).map(CountRange::unbounded_from),
            // A bounded middle interval has a two-sided complement; skip it.
            (_, Some(_)) => None,
        };
        if let Some(complement) = complement
            && sup_length.contains_range(complement)
        {
            return true;
        }
    }
    false
}

/// Recognize the complement of a canonicalized untyped numeric half-line.
///
/// A raw schema like `{ "minimum": 0 }` canonicalizes to an anyOf that accepts
/// every non-number type plus a bounded `type:number` branch. Negating that
/// union leaves only the opposite numeric half-line. We require explicit whole
/// non-number coverage and a plain numeric branch (no enum/multipleOf), so the
/// derived interval is an over-approximation of the negated language even if
/// there are additional numeric branches in the union.
fn negated_untyped_numeric_halfline_subset_of(branches: &[SchemaNode], sup: &SchemaNode) -> bool {
    // All non-number JSON types must be swallowed by the excluded union.
    for bit in [
        JSON_TYPE_NULL,
        JSON_TYPE_BOOL,
        JSON_TYPE_STRING,
        JSON_TYPE_ARRAY,
        JSON_TYPE_OBJECT,
    ] {
        if !branches
            .iter()
            .any(|branch| schema_obviously_accepts_json_type(branch, bit))
        {
            return false;
        }
    }

    let SchemaNodeKind::Number {
        multiple_of: None,
        enumeration: None,
        ..
    } = sup.kind()
    else {
        return false;
    };
    let Some(sup_interval) = numeric_interval_bound(sup) else {
        return false;
    };

    for branch in branches {
        let SchemaNodeKind::Number {
            bounds,
            multiple_of: None,
            enumeration: None,
        } = branch.kind()
        else {
            continue;
        };
        let excluded = NumericInterval {
            lower: match bounds.lower() {
                NumberBound::Unbounded => None,
                NumberBound::Inclusive(value) => Some(NumericIntervalBound {
                    value,
                    inclusive: true,
                }),
                NumberBound::Exclusive(value) => Some(NumericIntervalBound {
                    value,
                    inclusive: false,
                }),
            },
            upper: match bounds.upper() {
                NumberBound::Unbounded => None,
                NumberBound::Inclusive(value) => Some(NumericIntervalBound {
                    value,
                    inclusive: true,
                }),
                NumberBound::Exclusive(value) => Some(NumericIntervalBound {
                    value,
                    inclusive: false,
                }),
            },
            empty: false,
        };

        // Only a half-line has a single-interval complement.
        let complement = match (excluded.lower, excluded.upper) {
            (Some(lower), None) => NumericInterval {
                lower: None,
                upper: Some(NumericIntervalBound {
                    value: lower.value,
                    inclusive: !lower.inclusive,
                }),
                empty: false,
            },
            (None, Some(upper)) => NumericInterval {
                lower: Some(NumericIntervalBound {
                    value: upper.value,
                    inclusive: !upper.inclusive,
                }),
                upper: None,
                empty: false,
            },
            _ => continue,
        };
        if numeric_interval_contains(sup_interval, complement) {
            return true;
        }
    }
    false
}

fn negated_union_type_remainder_subset_of(branches: &[SchemaNode], sup: &SchemaNode) -> bool {
    let mut remainder = JSON_TYPE_ALL;
    for bit in [
        JSON_TYPE_NULL,
        JSON_TYPE_BOOL,
        JSON_TYPE_NUMBER,
        JSON_TYPE_STRING,
        JSON_TYPE_ARRAY,
        JSON_TYPE_OBJECT,
    ] {
        if branches
            .iter()
            .any(|branch| schema_obviously_accepts_json_type(branch, bit))
        {
            remainder &= !bit;
        }
    }
    if remainder == 0 {
        return true;
    }
    [
        JSON_TYPE_NULL,
        JSON_TYPE_BOOL,
        JSON_TYPE_NUMBER,
        JSON_TYPE_STRING,
        JSON_TYPE_ARRAY,
        JSON_TYPE_OBJECT,
    ]
    .into_iter()
    .filter(|bit| remainder & *bit != 0)
    .all(|bit| schema_obviously_accepts_json_type(sup, bit))
}

/// Prove a schema is covered by an `anyOf` whose branches obviously accept
/// whole JSON value types. This is intentionally much weaker than general
/// union reasoning: it only fires when every possible type of `sub` has a
/// branch that accepts *all* values of that type.
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
            // If a schema cannot possibly accept a JSON type, its negation
            // accepts that entire type. `possible_json_type_mask` is an upper
            // bound, so a missing bit is a sound whole-type fact.
            Not(child) if possible_json_type_mask(child) & bit == 0 => true,
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

        // `possible_json_type_mask` is a sound upper bound on accepted JSON
        // types.  If two upper bounds are disjoint, the languages themselves
        // are disjoint, regardless of any applicator structure inside either
        // side.  This cheap check is especially useful when proving
        // `S <= not(anyOf[T...])`: each excluded arm can often be ruled out by
        // type alone before we descend into contains/items/property details.
        let left_mask = possible_json_type_mask(left);
        let right_mask = possible_json_type_mask(right);
        if left_mask == 0 || right_mask == 0 || (left_mask & right_mask) == 0 {
            active.remove(&(left.id(), right.id()));
            return true;
        }

        // If either side has a finite upper bound, concrete-value rejection can
        // prove disjointness even when the other side is an explicit
        // complement/intersection (for example `allOf` of `not const` versus
        // an `enum`).  Because the finite set is only an upper bound, each
        // candidate must either be definitely rejected by its own schema or by
        // the opposite schema.
        if let Some(values) = finite_schema_value_superset(left)
            && values.iter().all(|value| {
                context.schema_definitely_rejects_value(left, value)
                    || context.schema_definitely_rejects_value(right, value)
            })
        {
            active.remove(&(left.id(), right.id()));
            return true;
        }
        if let Some(values) = finite_schema_value_superset(right)
            && values.iter().all(|value| {
                context.schema_definitely_rejects_value(right, value)
                    || context.schema_definitely_rejects_value(left, value)
            })
        {
            active.remove(&(left.id(), right.id()));
            return true;
        }

        use SchemaNodeKind::*;
        let result = match (left.kind(), right.kind()) {
            (BoolSchema(false), _) | (_, BoolSchema(false)) => true,
            // Distribute the cheap disjointness proof across an intersection
            // versus a union: every union arm must be ruled out, and any
            // single conjunct can rule out a given arm.  The generic AllOf
            // and AnyOf cases below are intentionally simpler; this mixed
            // shape is common after spelling De Morgan complements explicitly.
            (AllOf(conjuncts), AnyOf(branches)) | (AllOf(conjuncts), OneOf(branches)) => {
                branches.iter().all(|branch| {
                    conjuncts
                        .iter()
                        .any(|conjunct| inner(conjunct, branch, context, active))
                })
            }
            (AnyOf(branches), AllOf(conjuncts)) | (OneOf(branches), AllOf(conjuncts)) => {
                branches.iter().all(|branch| {
                    conjuncts
                        .iter()
                        .any(|conjunct| inner(branch, conjunct, context, active))
                })
            }
            // A complement is disjoint from any schema proved to be contained
            // by the complemented language. This is the small implication
            // needed for De Morgan-shaped schemas such as
            // `allOf: [{not: A}, {not: B}]` versus `not: {anyOf: [A, B]}`:
            // when checking disjointness with one branch `A`, the explicit
            // `not: A` conjunct is enough. Keep this as an implication proof
            // rather than attempting general complement algebra.
            (Not(excluded), _) => {
                analyze_subschema_with_context(
                    right,
                    excluded,
                    context,
                    ExplanationMode::VerdictOnly,
                )
                .is_subschema
            }
            (_, Not(excluded)) => {
                analyze_subschema_with_context(
                    left,
                    excluded,
                    context,
                    ExplanationMode::VerdictOnly,
                )
                .is_subschema
            }
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

/// Sufficient De Morgan cover for `not(allOf: [A, B, ...])` against an
/// `anyOf` target.  The complement of an intersection is the union of the
/// complements; for each conjunct `A`, it is enough to find a target branch
/// `not B` with `B <= A` (contravariance of negation).  This deliberately
/// avoids constructing synthetic schema nodes and stays conservative for
/// non-negated target branches.
fn negated_allof_covered_by_anyof(
    conjuncts: &[SchemaNode],
    branches: &[SchemaNode],
    context: &mut SubschemaCheckContext,
) -> bool {
    conjuncts.iter().all(|conjunct| {
        branches.iter().any(|branch| match branch.kind() {
            SchemaNodeKind::Any | SchemaNodeKind::BoolSchema(true) => true,
            SchemaNodeKind::Not(excluded) => {
                analyze_subschema_with_context(
                    excluded,
                    conjunct,
                    context,
                    ExplanationMode::VerdictOnly,
                )
                .is_subschema
            }
            _ => false,
        })
    })
}
/// Prove `not(anyOf([...])) <= target` when one excluded-union arm is a
/// complement of a finite schema. If an arm is `not A`, any value surviving
/// the outer negation must be in `A` and must avoid every sibling arm. Enumerate
/// an upper bound for `A`; each candidate must either be impossible for `A`, be
/// definitely consumed by a sibling arm, or be definitely accepted by target.
fn negated_anyof_finite_complement_arm_subset_of(
    children: &[SchemaNode],
    target: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    children.iter().enumerate().any(|(neg_index, child)| {
        let SchemaNodeKind::Not(core) = child.kind() else {
            return false;
        };
        let Some(values) = finite_schema_value_superset(core) else {
            return false;
        };

        values.iter().all(|value| {
            context.schema_definitely_rejects_value(core, value)
                || children.iter().enumerate().any(|(index, sibling)| {
                    index != neg_index && context.superset_contains_value(sibling, value)
                })
                || context.superset_contains_value(target, value)
        })
    })
}

/// Cover `not A` by an `anyOf` that contains a coarser finite complement
/// `not B` plus explicit branches for the finite gap `B \ A`.
///
/// For any value accepted by `not A`, either it is outside `B` (and therefore
/// accepted by the `not B` branch) or it is one of the finite candidates in an
/// upper bound for `B`.  For those candidates, require either definite
/// acceptance by `A` (so they are irrelevant to `not A`) or definite acceptance
/// by some union branch.  This handles generated partitions such as
/// `not {const: 1}` being covered by `anyOf: [not {enum:[1,2]}, {const:2}]`.
fn negated_exclusion_covered_by_anyof_finite_gap(
    excluded: &SchemaNode,
    branches: &[SchemaNode],
    context: &mut SubschemaCheckContext,
) -> bool {
    branches.iter().any(|branch| {
        let SchemaNodeKind::Not(coarser_excluded) = branch.kind() else {
            return false;
        };
        let Some(values) = finite_schema_value_superset(coarser_excluded) else {
            return false;
        };

        values.iter().all(|value| {
            context.schema_definitely_rejects_value(coarser_excluded, value)
                || context.superset_contains_value(excluded, value)
                || branches
                    .iter()
                    .any(|cover| context.superset_contains_value(cover, value))
        })
    })
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
        let overlap = possible_json_type_mask(sub) & possible_json_type_mask(other);
        // Property/tuple witnesses only prove disjointness for the object/array
        // portions of the languages.  If the same pair can also overlap on a
        // scalar (or on the other container kind), treating that local witness
        // as whole-schema disjointness would make oneOf partitions unsound.
        if overlap != 0
            && overlap & !JSON_TYPE_OBJECT == 0
            && (finite_required_property_values_rejected_by_other(sub, other, context)
                || finite_required_property_values_rejected_by_other(other, sub, context))
        {
            return true;
        }
        if overlap != 0
            && overlap & !JSON_TYPE_ARRAY == 0
            && (finite_required_array_item_values_rejected_by_other(sub, other, context)
                || finite_required_array_item_values_rejected_by_other(other, sub, context))
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
    if overlap == JSON_TYPE_NUMBER
        && (numeric_intervals_are_disjoint(sub, other) || integer_lattices_are_disjoint(sub, other))
    {
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

/// Prove disjointness for integer(-constrained) schemas whose `multipleOf`
/// constraints share no common multiple inside their overlapping finite
/// integer interval. Integer `multipleOf` constraints are all anchored at zero,
/// so their intersection is exactly the multiples of lcm(d1, d2). This is a
/// deliberately narrow syntactic check; returning false is conservative.
fn integer_lattices_are_disjoint(left: &SchemaNode, right: &SchemaNode) -> bool {
    #[derive(Clone, Copy)]
    struct Summary {
        lower: Option<i64>,
        upper: Option<i64>,
        divisor: i128,
    }

    fn gcd(mut a: i128, mut b: i128) -> i128 {
        while b != 0 {
            let r = a.rem_euclid(b);
            a = b;
            b = r;
        }
        a.abs()
    }

    fn lcm(a: i128, b: i128) -> Option<i128> {
        if a <= 0 || b <= 0 {
            return None;
        }
        let g = gcd(a, b);
        a.checked_div(g)?.checked_mul(b)
    }

    fn intersect(left: Summary, right: Summary) -> Option<Summary> {
        let lower = match (left.lower, right.lower) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (Some(a), None) | (None, Some(a)) => Some(a),
            (None, None) => None,
        };
        let upper = match (left.upper, right.upper) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(a), None) | (None, Some(a)) => Some(a),
            (None, None) => None,
        };
        Some(Summary {
            lower,
            upper,
            divisor: lcm(left.divisor, right.divisor)?,
        })
    }

    fn summarize(schema: &SchemaNode, active: &mut HashSet<NodeId>) -> Option<Summary> {
        if !active.insert(schema.id()) {
            return None;
        }
        let result = match schema.kind() {
            SchemaNodeKind::Integer {
                bounds,
                multiple_of,
                ..
            } => {
                let divisor = multiple_of
                    .as_ref()
                    .and_then(|multiple| multiple.integer_divisor())
                    .unwrap_or(1);
                (divisor > 0).then_some(Summary {
                    lower: bounds.lower(),
                    upper: bounds.upper(),
                    divisor,
                })
            }
            SchemaNodeKind::AllOf(children) => {
                let mut summary: Option<Summary> = None;
                for child in children {
                    if let Some(child_summary) = summarize(child, active) {
                        summary = Some(match summary {
                            Some(current) => match intersect(current, child_summary) {
                                Some(joined) => joined,
                                None => {
                                    active.remove(&schema.id());
                                    return None;
                                }
                            },
                            None => child_summary,
                        });
                    }
                }
                summary
            }
            _ => None,
        };
        active.remove(&schema.id());
        result
    }

    fn ceil_div(n: i128, d: i128) -> i128 {
        debug_assert!(d > 0);
        let q = n.div_euclid(d);
        let r = n.rem_euclid(d);
        if r == 0 { q } else { q + 1 }
    }

    let Some(left) = summarize(left, &mut HashSet::new()) else {
        return false;
    };
    let Some(right) = summarize(right, &mut HashSet::new()) else {
        return false;
    };
    let Some(combined) = intersect(left, right) else {
        return false;
    };
    let lower = combined.lower.map(i128::from);
    let upper = combined.upper.map(i128::from);
    let (Some(lower), Some(upper)) = (lower, upper) else {
        // Any half-infinite interval contains some multiple of a positive lcm.
        return false;
    };
    if lower > upper {
        return true;
    }
    let first_multiple = ceil_div(lower, combined.divisor).saturating_mul(combined.divisor);
    first_multiple > upper
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
                    let usable_declared = properties
                        .keys()
                        .filter(|name| !schema_forbids_property_name_for_objects(schema, name))
                        .count();
                    let declared = u64::try_from(usable_declared).ok()?;
                    upper = Some(upper.map_or(declared, |current| current.min(declared)));
                }

                // A finite propertyNames language caps the number of distinct
                // keys regardless of additionalProperties/patternProperties.
                // `finite_schema_value_superset` is an upper bound, so counting
                // its string members remains a sound (possibly loose) capacity.
                if let Some(names) = finite_property_name_strings_superset(property_names) {
                    // Some members of a finite propertyNames language may be
                    // syntactically impossible anyway (for example a declared
                    // property with schema `false`, or a name rejected by a
                    // matching false patternProperty). Dropping names that
                    // this very object schema forbids gives a tighter capacity
                    // while remaining an upper bound: every usable key must be
                    // in the finite name language and must not be individually
                    // forbidden by the object constraints.
                    let usable = names
                        .iter()
                        .filter(|name| !schema_forbids_property_name_for_objects(schema, name))
                        .count();
                    let finite = u64::try_from(usable).ok()?;
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

/// Return the exact small set of integer JSON values accepted by a `type:
/// number` schema when that finiteness is syntactically obvious.  We accept
/// explicit integer enums and inclusive singleton integer bounds.  In
/// particular, do *not* infer integrality from numeric `multipleOf`: validators
/// commonly use floating tolerances around multiples, so that would be unsafe.
/// Callers use this to bridge normalized numeric singletons into integer/const
/// targets without general arithmetic reasoning.
fn finite_integer_number_values(schema: &SchemaNode) -> Option<Vec<Value>> {
    let SchemaNodeKind::Number {
        multiple_of: _,
        enumeration,
        ..
    } = schema.kind()
    else {
        return None;
    };
    if let Some(values) = enumeration {
        if values.iter().all(|value| value.as_i64().is_some()) {
            return Some(values.clone());
        }
        return None;
    }

    let interval = numeric_interval_bound(schema)?;
    if interval.empty {
        return Some(Vec::new());
    }
    let (Some(lower), Some(upper)) = (interval.lower, interval.upper) else {
        return None;
    };
    if !lower.value.is_finite() || !upper.value.is_finite() {
        return None;
    }
    const MAX_EXACT: f64 = 9_007_199_254_740_991.0;
    if lower.value < -MAX_EXACT || upper.value > MAX_EXACT {
        return None;
    }

    // Do not infer integrality from `multipleOf` on a number schema: both the
    // JSON Schema validator and our evaluator allow tiny floating tolerances
    // around numeric multiples.  Only an inclusive singleton bound (or an
    // explicit enum handled above) is an exact finite integer language.
    let integrality_forced = lower.inclusive
        && upper.inclusive
        && lower.value == upper.value
        && lower.value.fract() == 0.0;
    if !integrality_forced {
        return None;
    }

    let mut start = lower.value.ceil();
    if !lower.inclusive && start == lower.value {
        start += 1.0;
    }
    let mut end = upper.value.floor();
    if !upper.inclusive && end == upper.value {
        end -= 1.0;
    }
    if start > end {
        return Some(Vec::new());
    }
    if start < i64::MIN as f64 || end > i64::MAX as f64 {
        return None;
    }
    let start_i = start as i64;
    let end_i = end as i64;
    if end_i.checked_sub(start_i).is_none_or(|span| span > 2048) {
        return None;
    }

    let mut values = Vec::new();
    for n in start_i..=end_i {
        let value = Value::Number(n.into());
        if schema.accepts_value(&value) {
            values.push(value);
        }
    }
    Some(values)
}

/// Return a small finite integer upper bound for split `allOf` integer ranges.
/// This is deliberately an over-approximation: endpoints are rounded outward,
/// then callers filter candidates with the exact/rejection evaluator. The
/// direct-integer-conjunct guard is what makes it safe to enumerate integers
/// rather than arbitrary numbers.
fn finite_split_allof_integer_values(schema: &SchemaNode) -> Option<Vec<Value>> {
    if !matches!(schema.kind(), SchemaNodeKind::AllOf(_))
        || !allof_has_direct_integer_conjunct(schema)
        || possible_json_type_mask(schema) & !JSON_TYPE_NUMBER != 0
    {
        return None;
    }
    let interval = numeric_interval_bound(schema)?;
    if interval.empty {
        return Some(Vec::new());
    }
    let (Some(lower), Some(upper)) = (interval.lower, interval.upper) else {
        return None;
    };
    if !lower.value.is_finite() || !upper.value.is_finite() {
        return None;
    }
    const MAX_EXACT: f64 = 9_007_199_254_740_991.0;
    if lower.value < -MAX_EXACT || upper.value > MAX_EXACT {
        return None;
    }
    // Round outward; this may include one extra integer at an exclusive bound,
    // which is fine for an upper bound.
    let start_f = lower.value.floor();
    let end_f = upper.value.ceil();
    if start_f < i64::MIN as f64 || end_f > i64::MAX as f64 || start_f > end_f {
        return None;
    }
    let start = start_f as i64;
    let end = end_f as i64;
    if end.checked_sub(start).is_none_or(|span| span > 256) {
        return None;
    }
    Some((start..=end).map(|n| Value::Number(n.into())).collect())
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
            SchemaNodeKind::IfThenElse {
                then_schema,
                else_schema,
                ..
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), Some(else_schema)) => {
                    inner(then_schema, name, active) && inner(else_schema, name, active)
                }
                // The missing branch is implicit `true`; if the present branch
                // accepts every object containing the property, the whole
                // conditional does too regardless of the guard outcome.
                (Some(branch), None) | (None, Some(branch)) => inner(branch, name, active),
                (None, None) => true,
            },
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
            SchemaNodeKind::IfThenElse {
                then_schema,
                else_schema,
                ..
            } => match (then_schema.as_ref(), else_schema.as_ref()) {
                (Some(then_schema), Some(else_schema)) => {
                    inner(then_schema, active) && inner(else_schema, active)
                }
                (Some(branch), None) | (None, Some(branch)) => inner(branch, active),
                (None, None) => true,
            },
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
fn array_schema_is_locally_impossible(schema: &SchemaNode) -> bool {
    let SchemaNodeKind::Array {
        prefix_items,
        items,
        item_count,
        contains,
        unique_items,
        enumeration,
    } = schema.kind()
    else {
        return false;
    };
    let constraints = array::ArrayConstraints {
        prefix_items,
        items,
        item_count: *item_count,
        contains: contains.as_ref(),
        unique_items: *unique_items,
        enumeration: enumeration.as_deref(),
    };
    if array::array_constraints_definitely_uninhabited(&constraints) {
        return true;
    }
    let Some(effective_count) = array::effective_item_count_for_unique_finite_domain(
        prefix_items,
        items,
        *item_count,
        *unique_items,
    ) else {
        return true;
    };
    if let Some(contains) = contains.as_ref() {
        let matcher_disjoint_from_every_position = contains.count().min() > 0
            && prefix_items
                .iter()
                .all(|item| schemas_definitely_disjoint_by_shape(item, &contains.schema))
            && schemas_definitely_disjoint_by_shape(items, &contains.schema);
        array::contains_requirement_definitely_impossible(contains, effective_count, *unique_items)
            || array::contains_requirement_impossible_for_unique_finite_items(
                prefix_items,
                items,
                effective_count,
                contains,
                *unique_items,
            )
            || matcher_disjoint_from_every_position
            || array_contains_requirement_is_locally_impossible(schema)
    } else {
        false
    }
}

fn object_schema_is_locally_impossible(schema: &SchemaNode) -> bool {
    let SchemaNodeKind::Object {
        properties,
        pattern_properties,
        required,
        additional,
        property_names,
        property_count,
        dependent_required,
        enumeration,
        ..
    } = schema.kind()
    else {
        return false;
    };
    object::object_constraints_definitely_uninhabited(&object::ObjectConstraints {
        properties,
        pattern_properties,
        required,
        additional,
        property_names,
        property_count: *property_count,
        dependent_required,
        enumeration: enumeration.as_deref(),
    })
}

fn array_contains_requirement_is_locally_impossible(schema: &SchemaNode) -> bool {
    let SchemaNodeKind::Array {
        contains: Some(contains),
        item_count,
        ..
    } = schema.kind()
    else {
        return false;
    };
    let required = contains.count().min();
    if required == 0 {
        return false;
    }
    if item_count
        .max()
        .is_some_and(|max_items| required > max_items)
    {
        return true;
    }
    schema_is_locally_empty_for_finite_enumeration(&contains.schema)
}

/// Detect a split `allOf` object contradiction where a property is guaranteed
/// present and two conjuncts give it disjoint direct `properties` schemas.
/// This stays deliberately syntactic; pattern/additionalProperties interactions
/// are left to the existing object-local prover.
fn split_allof_object_property_is_impossible(schema: &SchemaNode) -> bool {
    let SchemaNodeKind::AllOf(children) = schema.kind() else {
        return false;
    };
    let mask = possible_json_type_mask(schema);
    if mask == 0 || mask & !JSON_TYPE_OBJECT != 0 {
        return false;
    }

    let mut required = HashSet::<String>::new();
    let mut constraints: HashMap<String, Vec<&SchemaNode>> = HashMap::new();
    for child in children {
        let child = unwrap_singleton_applicators(child);
        if let SchemaNodeKind::Object {
            properties,
            required: child_required,
            ..
        } = child.kind()
        {
            required.extend(child_required.iter().cloned());
            for (name, property_schema) in properties {
                constraints
                    .entry(name.clone())
                    .or_default()
                    .push(property_schema);
            }
        }
    }

    for name in required {
        let Some(property_constraints) = constraints.get(&name) else {
            continue;
        };
        for i in 0..property_constraints.len() {
            for j in (i + 1)..property_constraints.len() {
                if schemas_definitely_disjoint_by_shape(
                    property_constraints[i],
                    property_constraints[j],
                ) {
                    return true;
                }
            }
        }
    }
    false
}

/// Detect a split `allOf` array contradiction where one conjunct constrains
/// every item to a homogeneous schema and another requires a `contains` match
/// that is disjoint from that homogeneous item schema.  This is intentionally
/// narrow: the whole intersection must be array-only, and the homogeneous
/// conjunct must have no prefix tuple holes.
fn split_allof_array_contains_is_impossible(schema: &SchemaNode) -> bool {
    let SchemaNodeKind::AllOf(children) = schema.kind() else {
        return false;
    };
    let mask = possible_json_type_mask(schema);
    if mask == 0 || mask & !JSON_TYPE_ARRAY != 0 {
        return false;
    }

    let mut homogeneous_items: Vec<&SchemaNode> = Vec::new();
    let mut required_contains: Vec<&SchemaNode> = Vec::new();
    for child in children {
        let child = unwrap_singleton_applicators(child);
        if let SchemaNodeKind::Array {
            prefix_items,
            items,
            contains,
            ..
        } = child.kind()
        {
            if prefix_items.is_empty() {
                homogeneous_items.push(items);
            }
            if let Some(contains) = contains.as_ref()
                && contains.count().min() > 0
            {
                required_contains.push(&contains.schema);
            }
        }
    }

    homogeneous_items.iter().any(|items| {
        required_contains
            .iter()
            .any(|matcher| schemas_definitely_disjoint_by_shape(items, matcher))
    })
}

/// Detect a split `allOf` array length/count contradiction.
fn split_allof_array_length_is_impossible(schema: &SchemaNode) -> bool {
    let SchemaNodeKind::AllOf(_) = schema.kind() else {
        return false;
    };
    let mask = possible_json_type_mask(schema);
    if mask == 0 || mask & !JSON_TYPE_ARRAY != 0 {
        return false;
    }
    array_length_interval_bound(schema).is_some_and(|interval| interval.empty)
}

/// Detect a split `allOf` object property-count contradiction.
fn split_allof_object_count_is_impossible(schema: &SchemaNode) -> bool {
    let SchemaNodeKind::AllOf(_) = schema.kind() else {
        return false;
    };
    let mask = possible_json_type_mask(schema);
    if mask == 0 || mask & !JSON_TYPE_OBJECT != 0 {
        return false;
    }
    object_property_count_interval_bound(schema).is_some_and(|interval| interval.empty)
}

/// Detect a split `allOf` numeric-range contradiction.  This only fires when
/// the whole intersection is numeric-only and the existing interval extractor
/// can prove the intersection empty, so lattice/multipleOf cases remain with
/// the more precise numeric provers.
fn split_allof_numeric_range_is_impossible(schema: &SchemaNode) -> bool {
    let SchemaNodeKind::AllOf(_) = schema.kind() else {
        return false;
    };
    let mask = possible_json_type_mask(schema);
    if mask == 0 || mask & !JSON_TYPE_NUMBER != 0 {
        return false;
    }
    numeric_interval_bound(schema).is_some_and(|interval| interval.empty)
}

/// Detect the simplest split `allOf` string-length contradiction.  The normal
/// string interval reasoning is used for pairwise disjointness, but negation
/// normalization needs an explicit emptiness witness for schemas such as
/// `allOf: [{minLength: 3}, {maxLength: 1}]`.  Keep this syntactic and
/// string-only so it cannot accidentally classify a mixed-type intersection as
/// empty.
fn split_allof_string_length_is_impossible(schema: &SchemaNode) -> bool {
    let SchemaNodeKind::AllOf(children) = schema.kind() else {
        return false;
    };
    let mask = possible_json_type_mask(schema);
    if mask == 0 || mask & !JSON_TYPE_STRING != 0 {
        return false;
    }

    let mut lower = 0_u64;
    let mut upper: Option<u64> = None;
    let mut saw_string_bound = false;
    for child in children {
        let child = unwrap_singleton_applicators(child);
        let SchemaNodeKind::String { length, .. } = child.kind() else {
            continue;
        };
        saw_string_bound = true;
        lower = lower.max(length.min());
        if let Some(child_upper) = length.max() {
            upper = Some(upper.map_or(child_upper, |current| current.min(child_upper)));
        }
    }

    saw_string_bound && upper.is_some_and(|max_len| lower > max_len)
}

/// A deliberately small emptiness predicate for the inner side of a negation.
///
/// `not(false)`-like targets are universal, but the finite-enumeration
/// emptiness helper intentionally does not look at the richer array/object
/// contradiction checks (unique finite domains, impossible `contains`, closed
/// object/property-name conflicts). Those checks are already used to prove an
/// impossible schema is a subset of an arbitrary typed target; expose the same
/// fact to negation normalization without turning this into a general recursive
/// emptiness prover. The applicator recursion below only follows directions
/// that preserve emptiness, and is cycle guarded.
fn schema_is_locally_impossible_for_negation(schema: &SchemaNode) -> bool {
    fn inner(schema: &SchemaNode, active: &mut HashSet<NodeId>) -> bool {
        if !active.insert(schema.id()) {
            return false;
        }
        let normalized = unwrap_singleton_applicators(schema);
        if normalized.id() != schema.id() {
            let result = inner(normalized, active);
            active.remove(&schema.id());
            return result;
        }

        let result = schema_is_locally_empty_for_finite_enumeration(schema)
            || split_allof_array_contains_is_impossible(schema)
            || split_allof_object_property_is_impossible(schema)
            || split_allof_array_length_is_impossible(schema)
            || split_allof_object_count_is_impossible(schema)
            || split_allof_numeric_range_is_impossible(schema)
            || split_allof_string_length_is_impossible(schema)
            || array_schema_is_locally_impossible(schema)
            || object_schema_is_locally_impossible(schema)
            || match schema.kind() {
                SchemaNodeKind::AllOf(children) => {
                    children.iter().any(|child| inner(child, active))
                }
                SchemaNodeKind::AnyOf(children) => {
                    !children.is_empty() && children.iter().all(|child| inner(child, active))
                }
                SchemaNodeKind::OneOf(children) => {
                    (!children.is_empty() && children.iter().all(|child| inner(child, active)))
                        || (children.len() == 2
                            && schemas_obviously_equivalent(&children[0], &children[1]))
                }
                SchemaNodeKind::IfThenElse {
                    if_schema,
                    then_schema,
                    else_schema,
                } => {
                    // If both explicit branches are empty, no guard outcome can
                    // admit a value.  For a locally constant guard, it is also
                    // enough for the selected explicit branch to be empty;
                    // a missing selected branch is implicit `true`, so it is
                    // deliberately not treated as empty.
                    match (then_schema.as_ref(), else_schema.as_ref()) {
                        (Some(then_branch), Some(else_branch))
                            if inner(then_branch, active) && inner(else_branch, active) =>
                        {
                            true
                        }
                        (Some(then_branch), _) if schema_is_trivially_universal(if_schema) => {
                            inner(then_branch, active)
                        }
                        (_, Some(else_branch)) if inner(if_schema, active) => {
                            inner(else_branch, active)
                        }
                        _ => false,
                    }
                }
                _ => false,
            };
        active.remove(&schema.id());
        result
    }

    inner(schema, &mut HashSet::new())
}

fn schema_is_locally_empty_for_finite_enumeration(schema: &SchemaNode) -> bool {
    fn inner(schema: &SchemaNode, active: &mut HashSet<NodeId>) -> bool {
        if !active.insert(schema.id()) {
            // Recursive wrappers are not a local emptiness witness by themselves.
            return false;
        }
        let result = if matches!(schema.kind(), SchemaNodeKind::BoolSchema(false))
            || matches!(schema.kind(), SchemaNodeKind::Enum(values) if values.is_empty())
            || possible_json_type_mask(schema) == 0
            || matches!(constrained_enumeration(schema), Some(values) if values.is_empty())
        {
            true
        } else if let Some(values) = constrained_enumeration(schema)
            && !schema_may_under_accept_values(schema)
            && values.iter().all(|value| !schema.accepts_value(value))
        {
            true
        } else {
            match schema.kind() {
                SchemaNodeKind::AllOf(children) => {
                    children.iter().any(|child| inner(child, active))
                }
                SchemaNodeKind::AnyOf(children) | SchemaNodeKind::OneOf(children)
                    if children.len() == 1 =>
                {
                    inner(&children[0], active)
                }
                _ => false,
            }
        };
        active.remove(&schema.id());
        result
    }

    inner(schema, &mut HashSet::new())
}
/// Return a finite upper bound for the *string* values accepted by `schema`.
///
/// This is deliberately narrower than `finite_schema_value_superset`: when a
/// schema is used as `propertyNames`, only string instances are ever tested.
/// A conditional such as `{if: {type: "string"}, then: {enum: [...]}}` is
/// infinite as a whole (non-strings are unconstrained), but has a finite string
/// language. Keeping this helper string-scoped lets object cardinality use
/// that fact without making the global finite-language prover unsound.
pub(super) fn finite_property_name_strings_superset(schema: &SchemaNode) -> Option<Vec<String>> {
    fn push(values: &mut Vec<String>, value: String) {
        if !values.iter().any(|seen| seen == &value) {
            values.push(value);
        }
    }

    fn strings_from_values(values: &[Value]) -> Vec<String> {
        let mut out = Vec::new();
        for value in values {
            if let Some(s) = value.as_str() {
                push(&mut out, s.to_owned());
            }
        }
        out
    }

    fn merge(mut left: Vec<String>, right: Vec<String>) -> Vec<String> {
        for value in right {
            push(&mut left, value);
        }
        left
    }

    fn inner(
        schema: &SchemaNode,
        active: &mut HashSet<NodeId>,
    ) -> Option<Vec<std::string::String>> {
        if !active.insert(schema.id()) {
            return None;
        }
        macro_rules! try_opt {
            ($expr:expr) => {
                match $expr {
                    Some(value) => value,
                    None => {
                        active.remove(&schema.id());
                        return None;
                    }
                }
            };
        }

        use SchemaNodeKind::*;
        let result = match schema.kind() {
            BoolSchema(false) => Some(Vec::new()),
            _ if possible_json_type_mask(schema) & JSON_TYPE_STRING == 0 => Some(Vec::new()),
            Const(value) => Some(value.as_str().map_or_else(Vec::new, |s| vec![s.to_owned()])),
            Enum(values) => Some(strings_from_values(values)),
            String {
                enumeration: Some(values),
                ..
            } => Some(strings_from_values(values)),
            String {
                length,
                enumeration: None,
                ..
            } if length.max() == Some(0) => Some(vec![std::string::String::new()]),
            AllOf(children) => {
                let mut best: Option<Vec<std::string::String>> = None;
                for child in children {
                    if let Some(values) = inner(child, active)
                        && best
                            .as_ref()
                            .is_none_or(|current| values.len() < current.len())
                    {
                        best = Some(values);
                    }
                }
                best.map(|mut values| {
                    values.retain(|name| {
                        let value = Value::String(name.clone());
                        !children.iter().any(|child| {
                            !schema_may_under_accept_values(child) && !child.accepts_value(&value)
                        })
                    });
                    values
                })
            }
            AnyOf(children) | OneOf(children) => {
                let mut union = Vec::new();
                for child in children {
                    let Some(values) = inner(child, active) else {
                        active.remove(&schema.id());
                        return None;
                    };
                    union = merge(union, values);
                }
                Some(union)
            }
            IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => {
                let if_all_strings =
                    whole_json_types_accepted_mask(if_schema) & JSON_TYPE_STRING != 0;
                let if_no_strings = possible_json_type_mask(if_schema) & JSON_TYPE_STRING == 0;
                if if_all_strings {
                    then_schema
                        .as_ref()
                        .and_then(|branch| inner(branch, active))
                } else if if_no_strings {
                    else_schema
                        .as_ref()
                        .and_then(|branch| inner(branch, active))
                } else if let (Some(then_branch), Some(else_branch)) =
                    (then_schema.as_ref(), else_schema.as_ref())
                {
                    let then_values = try_opt!(inner(then_branch, active));
                    let else_values = try_opt!(inner(else_branch, active));
                    Some(merge(then_values, else_values))
                } else if then_schema.is_none() {
                    let else_values = try_opt!(else_schema.as_ref().and_then(|b| inner(b, active)));
                    let condition_values = try_opt!(inner(if_schema, active));
                    Some(merge(condition_values, else_values))
                } else if else_schema.is_none() {
                    let then_values = try_opt!(then_schema.as_ref().and_then(|b| inner(b, active)));
                    if let Not(negated) = if_schema.kind() {
                        let else_side = try_opt!(inner(negated, active));
                        Some(merge(then_values, else_side))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        };

        let result = result.map(|mut values| {
            if !schema_may_under_accept_values(schema) {
                values.retain(|name| schema.accepts_value(&Value::String(name.clone())));
            }
            values
        });
        active.remove(&schema.id());
        result
    }

    inner(schema, &mut HashSet::new())
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
                enumeration: None,
                ..
            } => match (bounds.lower(), bounds.upper()) {
                (NumberBound::Inclusive(lower), NumberBound::Inclusive(upper))
                    if lower.to_bits() == upper.to_bits() =>
                {
                    serde_json::Number::from_f64(lower)
                        .map(Value::Number)
                        .map(|value| vec![value])
                }
                _ => None,
            },
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
                let Some(mut keys) = finite_property_name_strings_superset(property_names) else {
                    active.remove(&schema.id());
                    return None;
                };
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
            OneOf(children)
                if children.len() > 1
                    && children
                        .iter()
                        .all(|child| matches!(child.kind(), SchemaNodeKind::Not(_))) =>
            {
                // A `oneOf` made solely of complements has no inhabitants
                // outside the union of the excluded regions: a value outside
                // every excluded region matches every complement branch, not
                // exactly one of them. When each excluded region has a finite
                // upper bound, their union is therefore a finite upper bound
                // for the whole xor. This catches common generated shapes
                // such as `oneOf: [not const A, not enum [A, B]]`, whose true
                // language is the finite symmetric difference, without
                // pretending arbitrary negation is finite.
                let mut union = Vec::new();
                let mut all_finite = true;
                for child in children {
                    let SchemaNodeKind::Not(excluded) = child.kind() else {
                        all_finite = false;
                        break;
                    };
                    let Some(values) = inner(excluded, active) else {
                        all_finite = false;
                        break;
                    };
                    for value in values {
                        push_distinct(&mut union, value);
                        if union.len() > 256 {
                            all_finite = false;
                            break;
                        }
                    }
                    if !all_finite {
                        break;
                    }
                }
                if all_finite { Some(union) } else { None }
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
                // Split integer bounds across conjuncts can be finite even when
                // no single child is finite; use that as an initial upper bound
                // and still prefer any smaller finite child below.
                let mut best: Option<Vec<Value>> = finite_split_allof_integer_values(schema);
                // The intersection is a subset of every child. Any finite
                // child therefore gives a sound finite upper bound; choose the
                // smallest one we can find to keep later bounds useful.
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
    fn singleton_number_bounds_can_be_integer_subset() {
        let old = resolve(json!({ "type": "integer", "multipleOf": 2 }));
        let new = resolve(json!({ "type": "number", "minimum": 4, "maximum": 4 }));
        let fractional = resolve(json!({ "type": "number", "minimum": 4.5, "maximum": 4.5 }));

        assert!(is_subschema_of(&new, &old));
        assert!(!is_subschema_of(&fractional, &old));
    }

    #[test]
    fn singleton_number_bounds_empty_after_multiple_of_is_vacuous() {
        let old = resolve(json!({ "type": "integer", "const": 7 }));
        let new = resolve(json!({
            "type": "number",
            "minimum": 4,
            "maximum": 4,
            "multipleOf": 3
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn integral_number_multiple_of_alone_is_not_treated_as_integer() {
        let old = resolve(json!({ "type": "integer" }));
        let new = resolve(json!({ "type": "number", "multipleOf": 1 }));

        // The validator permits tiny floating-point deviations for multipleOf,
        // so values such as 1.0000000000000002 can satisfy the number schema
        // without satisfying the integer schema.
        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn number_enum_can_be_subsumed_by_integer_schema() {
        let old =
            resolve(json!({ "type": "integer", "minimum": 2, "maximum": 8, "multipleOf": 2 }));
        let new = resolve(json!({
            "type": "number",
            "enum": [2, 4.0, 8],
            "minimum": 0,
            "maximum": 10
        }));

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn number_enum_fractional_member_blocks_integer_subset() {
        let old = resolve(json!({ "type": "integer" }));
        let new = resolve(json!({ "type": "number", "enum": [2, 2.5] }));

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
    fn wrapped_integer_lattice_gap_can_partition_oneof() {
        let sub = resolve(json!({
            "allOf": [
                { "type": "integer", "minimum": 2, "maximum": 4 },
                { "type": "integer", "multipleOf": 2 }
            ]
        }));
        let sup = resolve(json!({
            "oneOf": [
                {
                    "allOf": [
                        { "type": "integer", "minimum": 2, "maximum": 4 },
                        { "type": "integer", "multipleOf": 2 }
                    ]
                },
                {
                    "allOf": [
                        { "type": "integer", "minimum": 1, "maximum": 5 },
                        { "type": "integer", "multipleOf": 3 }
                    ]
                }
            ]
        }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn property_disjointness_does_not_ignore_shared_non_objects() {
        let branch_a = json!({
            "type": ["object", "null"],
            "required": ["kind"],
            "properties": { "kind": { "const": "a" } }
        });
        let branch_b = json!({
            "type": ["object", "null"],
            "required": ["kind"],
            "properties": { "kind": { "const": "b" } }
        });
        let sub = resolve(branch_a.clone());
        let sup = resolve(json!({ "oneOf": [branch_a, branch_b] }));
        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn tuple_disjointness_does_not_ignore_shared_non_arrays() {
        let branch_a = json!({
            "type": ["array", "null"],
            "minItems": 1,
            "prefixItems": [{ "const": "a" }]
        });
        let branch_b = json!({
            "type": ["array", "null"],
            "minItems": 1,
            "prefixItems": [{ "const": "b" }]
        });
        let sub = resolve(branch_a.clone());
        let sup = resolve(json!({ "oneOf": [branch_a, branch_b] }));
        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditional_property_disjointness_keeps_shared_non_objects() {
        let branch_a = json!({
            "if": { "type": "object" },
            "then": {
                "required": ["kind"],
                "properties": { "kind": { "const": "a" } }
            }
        });
        let branch_b = json!({
            "if": { "type": "object" },
            "then": {
                "required": ["kind"],
                "properties": { "kind": { "const": "b" } }
            }
        });
        let sub = resolve(branch_a.clone());
        let sup = resolve(json!({ "oneOf": [branch_a, branch_b] }));
        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditional_tuple_disjointness_keeps_shared_non_arrays() {
        let branch_a = json!({
            "if": { "type": "array" },
            "then": {
                "minItems": 1,
                "prefixItems": [{ "const": "a" }]
            }
        });
        let branch_b = json!({
            "if": { "type": "array" },
            "then": {
                "minItems": 1,
                "prefixItems": [{ "const": "b" }]
            }
        });
        let sub = resolve(branch_a.clone());
        let sup = resolve(json!({ "oneOf": [branch_a, branch_b] }));
        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn integral_number_lattice_gap_is_not_used_for_number_oneof() {
        let sub = resolve(json!({
            "type": "number",
            "minimum": 2,
            "maximum": 4,
            "multipleOf": 2
        }));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "number", "minimum": 2, "maximum": 4, "multipleOf": 2 },
                { "type": "number", "minimum": 1, "maximum": 5, "multipleOf": 3 }
            ]
        }));
        // Numeric multipleOf uses an epsilon tolerance, so near-multiple floats
        // can overlap even when the integer lattice projection has a gap.
        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn integer_lattice_gap_can_partition_oneof() {
        let sub = resolve(json!({
            "type": "integer",
            "minimum": 2,
            "maximum": 4,
            "multipleOf": 2
        }));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "integer", "minimum": 2, "maximum": 4, "multipleOf": 2 },
                { "type": "integer", "minimum": 1, "maximum": 5, "multipleOf": 3 }
            ]
        }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn forbidden_closed_declared_property_tightens_count_partition() {
        let sub = resolve(json!({
            "type": "object",
            "properties": { "a": true, "b": false },
            "additionalProperties": false
        }));
        let sup = resolve(json!({
            "oneOf": [
                {
                    "type": "object",
                    "properties": { "a": true, "b": false },
                    "additionalProperties": false
                },
                { "type": "object", "minProperties": 2 }
            ]
        }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn forbidden_finite_property_names_tighten_object_count_partition() {
        let sub = resolve(json!({
            "type": "object",
            "propertyNames": { "enum": ["a", "b"] },
            "properties": { "b": false }
        }));
        let sup = resolve(json!({
            "oneOf": [
                {
                    "type": "object",
                    "propertyNames": { "enum": ["a", "b"] },
                    "properties": { "b": false }
                },
                { "type": "object", "minProperties": 2 }
            ]
        }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditional_finite_property_names_imply_object_count_partition() {
        let names = json!({
            "if": { "type": "string" },
            "then": { "enum": ["a", "b"] }
        });
        let sub = resolve(json!({
            "type": "object",
            "propertyNames": names.clone()
        }));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "object", "propertyNames": names },
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
    fn anyof_complement_cover_accepts_unconstrained_schema() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "type": "integer" },
                { "not": { "const": 1 } }
            ]
        }));

        // `const 1` is contained by `integer`, so integer ∪ not(1) is all JSON.
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn anyof_complement_of_union_cover_accepts_unconstrained_schema() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "type": "string" },
                { "type": "number" },
                { "not": { "anyOf": [
                    { "type": "string" },
                    { "type": "number" }
                ] } }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn anyof_finite_split_complement_cover_accepts_unconstrained_schema() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "not": { "enum": [1, 2, "x"] } },
                { "const": 1 },
                { "const": 2 },
                { "const": "x" }
            ]
        }));

        // The finite excluded enum is covered by several sibling branches;
        // together with its complement this is universal.
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn anyof_split_integer_range_complement_cover_accepts_unconstrained_schema() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "not": { "allOf": [
                    { "type": "integer" },
                    { "minimum": 0 },
                    { "maximum": 2 }
                ] } },
                { "const": 0 },
                { "const": 1 },
                { "const": 2 }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn oneof_complement_of_disjoint_union_accepts_unconstrained_schema() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "string" },
                { "type": "number" },
                { "not": { "anyOf": [
                    { "type": "string" },
                    { "type": "number" }
                ] } }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn oneof_complement_of_overlapping_union_is_not_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "integer" },
                { "type": "number" },
                { "not": { "anyOf": [
                    { "type": "integer" },
                    { "type": "number" }
                ] } }
            ]
        }));

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn oneof_infinite_complement_pair_accepts_unconstrained_schema() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "string" },
                { "not": { "type": "string" } }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn oneof_non_equivalent_complement_pair_stays_conservative() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "integer" },
                { "not": { "type": "number" } }
            ]
        }));

        // Fractional numbers match neither arm, so this is not universal.
        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn oneof_finite_complement_partition_accepts_unconstrained_schema() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "oneOf": [
                { "not": { "enum": [1, 2] } },
                { "const": 1 },
                { "const": 2 }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn oneof_finite_complement_partition_rejects_overlapping_cover() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "oneOf": [
                { "not": { "enum": [1, 2] } },
                { "enum": [1, 2] },
                { "const": 1 }
            ]
        }));

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn split_allof_small_integer_range_uses_finite_conditional_target() {
        let sub = resolve(json!({
            "allOf": [
                { "type": "integer", "minimum": 0 },
                { "type": "integer", "maximum": 1 }
            ]
        }));
        let sup = resolve(json!({
            "if": { "enum": [1, "a", null] },
            "then": { "type": "number" },
            "else": true
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditional_disjoint_from_negated_const_target() {
        let sub = resolve(json!({
            "if": { "enum": [1, "a", null] },
            "then": { "type": "number" },
            "else": true
        }));
        let sup = resolve(json!({ "not": { "const": null } }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn identical_type_guard_prunes_both_vacuous_then_sides() {
        let sub =
            resolve(json!({ "if": {"type":"integer"}, "then": {"type":"null"}, "else": true }));
        let sup =
            resolve(json!({ "if": {"type":"integer"}, "then": {"type":"object"}, "else": true }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn identical_conditional_guard_prunes_vacuous_then_side() {
        let sub = resolve(json!({
            "if": { "enum": ["a", "b"] },
            "then": { "type": "integer" },
            "else": { "type": "object" }
        }));
        let sup = resolve(json!({
            "if": { "enum": ["a", "b"] },
            "then": { "type": "object" },
            "else": true
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditional_guard_covered_then_with_universal_else_is_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "if": { "enum": ["a", "b"] },
            "then": { "type": "string" },
            "else": true
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn oneof_universal_arm_excludes_disjoint_conditional() {
        let sub = resolve(json!({ "type": "integer", "multipleOf": 2 }));
        let sup = resolve(json!({
            "oneOf": [
                true,
                {
                    "if": { "type": "integer", "multipleOf": 1 },
                    "then": { "type": "array", "items": { "type": "boolean" } },
                    "else": { "type": "number", "minimum": 0, "maximum": 3 }
                }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn anyof_conditional_tautology_branch_is_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "if": { "const": 1 }, "then": { "const": 1 } },
                { "type": "boolean" }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditional_else_complement_of_guard_is_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "if": { "type": "string" },
            "then": true,
            "else": { "not": { "const": "blocked-string" } }
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditional_else_complement_must_be_guard_subset() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "if": { "type": "string" },
            "then": true,
            "else": { "not": { "const": 1 } }
        }));

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditional_then_enum_filtered_by_guard_fits_target() {
        let sub = resolve(json!({
            "if": { "type": "integer" },
            "then": { "enum": [1, "a"] },
            "else": false
        }));
        let sup = resolve(json!({ "type": "integer" }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditional_finite_guard_filters_infinite_then_branch() {
        let sub = resolve(json!({
            "if": { "enum": [1, "a"] },
            "then": { "type": "integer" },
            "else": false
        }));
        let sup = resolve(json!({ "const": 1 }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditional_negated_type_guard_false_then_is_inner_type() {
        let sub = resolve(json!({
            "if": { "not": { "type": "string" } },
            "then": false,
            "else": true
        }));
        let sup = resolve(json!({ "type": "string" }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditional_negated_finite_guard_with_universal_else_fits_target() {
        let sub = resolve(json!({
            "if": { "not": { "const": 1 } },
            "then": { "type": "integer" },
            "else": true
        }));
        let sup = resolve(json!({ "type": "integer" }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditional_negated_finite_guard_filters_infinite_else_branch() {
        let sub = resolve(json!({
            "if": { "not": { "const": 1 } },
            "then": false,
            "else": { "type": "integer" }
        }));
        let sup = resolve(json!({ "const": 1 }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditional_else_enum_filtered_by_guard_fits_target() {
        let sub = resolve(json!({
            "if": { "type": "integer" },
            "then": { "type": "string" },
            "else": { "enum": [1, "a"] }
        }));
        let sup = resolve(json!({ "type": "string" }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn same_guard_conditional_filters_then_by_finite_guard() {
        let sub = resolve(json!({
            "if": { "enum": [1, "a"] },
            "then": { "type": "integer" },
            "else": true
        }));
        let sup = resolve(json!({
            "if": { "enum": [1, "a"] },
            "then": { "const": 1 },
            "else": true
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn same_guard_conditional_filters_else_by_guard() {
        let sub = resolve(json!({
            "if": { "const": 1 },
            "then": true,
            "else": { "enum": [1, "a"] }
        }));
        let sup = resolve(json!({
            "if": { "const": 1 },
            "then": true,
            "else": { "const": "a" }
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn same_guard_conditional_missing_then_uses_finite_guard() {
        let sub = resolve(json!({
            "if": { "allOf": [{ "enum": [1, 2] }, { "const": 1 }] },
            "else": false
        }));
        let sup = resolve(json!({
            "if": { "allOf": [{ "enum": [1, 2] }, { "const": 1 }] },
            "then": { "const": 1 },
            "else": false
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn same_guard_conditional_missing_else_uses_negated_finite_guard() {
        let sub = resolve(json!({
            "if": { "not": { "enum": [1, "a"] } },
            "then": false
        }));
        let sup = resolve(json!({
            "if": { "not": { "enum": [1, "a"] } },
            "then": false,
            "else": { "enum": [1, "a"] }
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditional_with_false_then_is_subset_of_guard_complement() {
        let sub = resolve(json!({
            "if": { "type": "null" },
            "then": false,
            "else": true
        }));
        let sup = resolve(json!({ "not": { "type": "null" } }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditional_missing_else_uses_target_complement_cover() {
        let sub = resolve(json!({
            "if": { "type": "string" },
            "then": { "not": { "const": "reserved" } }
        }));
        let sup = resolve(json!({ "not": { "const": "reserved" } }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditional_explicit_else_uses_target_complement_cover() {
        let sub = resolve(json!({
            "if": { "type": "string" },
            "then": { "not": { "const": "reserved" } },
            "else": { "const": 1 }
        }));
        let sup = resolve(json!({ "not": { "const": "reserved" } }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditional_with_false_else_is_subset_of_guard_target() {
        let sub = resolve(json!({
            "if": { "type": "null" },
            "then": { "not": { "const": 1 } },
            "else": false
        }));
        let sup = resolve(json!({ "type": "null" }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn allof_to_allof_uses_whole_intersection_for_conjunct() {
        let sub = resolve(json!({
            "allOf": [
                { "type": "integer" },
                { "minimum": 1 },
                { "maximum": 3 }
            ]
        }));
        let sup = resolve(json!({
            "allOf": [
                { "type": "integer", "minimum": 0, "maximum": 5 }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn same_guard_explicit_then_uses_guard_cover() {
        let sub = resolve(json!({
            "if": { "enum": ["a", "b"] },
            "then": true,
            "else": false
        }));
        let sup = resolve(json!({
            "if": { "enum": ["a", "b"] },
            "then": { "type": "string" },
            "else": false
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn same_guard_explicit_else_uses_complement_cover() {
        let sub = resolve(json!({
            "if": { "type": "string" },
            "then": { "maxLength": 5 },
            "else": { "not": { "const": 1 } }
        }));
        let sup = resolve(json!({
            "if": { "type": "string" },
            "then": { "maxLength": 10 },
            "else": { "not": { "const": "reserved" } }
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn same_guard_missing_else_accepts_complement_cover() {
        let sub = resolve(json!({
            "if": { "type": "string" },
            "then": { "maxLength": 5 }
        }));
        let sup = resolve(json!({
            "if": { "type": "string" },
            "then": { "maxLength": 10 },
            "else": { "not": { "const": "reserved" } }
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn same_guard_missing_else_rejects_incomplete_complement_cover() {
        let sub = resolve(json!({
            "if": { "type": "string" },
            "then": { "maxLength": 5 }
        }));
        let sup = resolve(json!({
            "if": { "type": "string" },
            "then": { "maxLength": 10 },
            "else": { "not": { "const": 1 } }
        }));

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditional_guard_not_covered_then_is_not_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "if": { "enum": ["a", "b"] },
            "then": { "type": "number" },
            "else": true
        }));

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn incomplete_anyof_complement_cover_stays_conservative() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "type": "string" },
                { "not": { "const": 1 } }
            ]
        }));

        assert!(!is_subschema_of(&sub, &sup));
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
    fn negated_conditional_required_property_can_partition_branch() {
        let sub = resolve(json!({
            "type": "object",
            "required": ["flag"]
        }));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "object", "required": ["flag"] },
                { "not": {
                    "if": { "type": "object" },
                    "then": { "required": ["flag"] }
                }}
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
    fn allof_of_negations_implies_negated_anyof() {
        let sub = resolve(json!({
            "allOf": [
                { "not": { "type": "string" } },
                { "not": { "type": "integer" } }
            ]
        }));
        let sup = resolve(json!({
            "not": {
                "anyOf": [
                    { "type": "string" },
                    { "type": "integer" }
                ]
            }
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn negated_allof_of_complements_is_covered_by_positive_union() {
        let sub = resolve(json!({
            "not": { "allOf": [
                { "not": { "const": 2 } },
                { "not": { "enum": [2, 3] } }
            ] }
        }));
        let sup = resolve(json!({ "anyOf": [
            { "enum": [2, 3] },
            { "enum": ["a", "b"] }
        ] }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn negated_union_with_finite_complement_arm_filters_sibling_values() {
        let sub = resolve(json!({
            "not": {
                "anyOf": [
                    { "const": 1 },
                    { "not": { "enum": [1, 2] } }
                ]
            }
        }));
        let sup = resolve(json!({ "anyOf": [
            { "enum": [2, 3] },
            { "enum": ["a", "b"] }
        ] }));

        // The subset simplifies to the singleton value 2.
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn negated_union_with_complement_arm_implies_positive_arm() {
        let sub = resolve(json!({
            "not": {
                "anyOf": [
                    { "not": { "const": "ok" } },
                    { "const": "bad" }
                ]
            }
        }));
        let sup = resolve(json!({ "enum": ["ok", "other"] }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn negated_oneof_complement_pair_disjoint_reduces_to_union() {
        let sub = resolve(json!({
            "not": { "oneOf": [
                { "const": false },
                { "not": { "type": "integer" } }
            ] }
        }));
        let sup = resolve(json!({ "anyOf": [
            { "const": false },
            { "type": "integer" }
        ] }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn negated_oneof_complement_pair_finite_cancellation() {
        let sub = resolve(json!({
            "not": { "oneOf": [
                { "not": { "enum": [null, false] } },
                { "type": "null" }
            ] }
        }));
        let sup = resolve(json!({ "const": false }));

        // The xor complement reduces to the finite symmetric difference
        // between {null,false} and {null}, i.e. just false.
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn negated_oneof_complement_pair_overlap_uses_union_bound() {
        let sub = resolve(json!({
            "not": { "oneOf": [
                { "type": "string" },
                { "not": { "enum": [1, "a", null] } }
            ] }
        }));
        let sup = resolve(json!({ "anyOf": [
            { "type": "string" },
            { "enum": [1, "a", null] }
        ] }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn negated_oneof_complement_pair_comparable_reduces_to_larger_side() {
        let sub = resolve(json!({
            "not": { "oneOf": [
                { "enum": [1, 2] },
                { "not": { "type": "integer" } }
            ] }
        }));
        let sup = resolve(json!({ "type": "integer" }));

        // The positive enum arm is contained in the integer arm, so the
        // complement of the xor can only contain integers.
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn negated_singleton_covered_by_finite_complement_gap_union() {
        let sub = resolve(json!({ "not": { "const": 1 } }));
        let sup = resolve(json!({
            "anyOf": [
                { "not": { "enum": [1, 2] } },
                { "const": 2 }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn allof_of_finite_negations_implies_negated_enum() {
        let sub = resolve(json!({
            "allOf": [
                { "not": { "const": 1 } },
                { "not": { "const": 2 } }
            ]
        }));
        let sup = resolve(json!({ "not": { "enum": [1, 2] } }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn negated_allof_is_covered_by_anyof_of_negated_applicability_conjuncts() {
        // `minLength` without an explicit type is normalized as an
        // applicability union (non-strings plus constrained strings).  The
        // De Morgan cover still holds after that expansion.
        let sub = resolve(json!({
            "not": {
                "allOf": [
                    { "type": "string" },
                    { "minLength": 2 }
                ]
            }
        }));
        let sup = resolve(json!({
            "anyOf": [
                { "not": { "type": "string" } },
                { "not": { "minLength": 2 } }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn double_negation_on_left_delegates_to_inner_subset() {
        let sub = resolve(json!({ "not": { "not": { "enum": [1, "a"] } } }));
        let sup = resolve(json!({ "anyOf": [
            { "enum": [1, "a"] },
            { "const": 2 }
        ] }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn double_negation_on_right_delegates_to_inner_subset() {
        let sub = resolve(json!({ "enum": [1, "a"] }));
        let sup = resolve(json!({ "not": { "not": { "anyOf": [
            { "const": 1 },
            { "const": "a" },
            { "const": 2 }
        ] } } }));

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
    fn impossible_min_properties_exceeds_conditional_finite_property_names_is_vacuous() {
        let names = json!({
            "if": { "type": "string" },
            "then": { "enum": ["only"] }
        });
        let impossible = resolve(json!({
            "type": "object",
            "minProperties": 2,
            "propertyNames": names
        }));
        let arbitrary_object = resolve(json!({
            "type": "object",
            "required": ["z"]
        }));

        assert!(is_subschema_of(&impossible, &arbitrary_object));
    }

    #[test]
    fn conditional_missing_then_finite_property_names_bound_count() {
        let names = json!({
            "if": { "enum": ["a"] },
            "else": { "enum": ["b"] }
        });
        let sub = resolve(json!({
            "type": "object",
            "propertyNames": names.clone()
        }));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "object", "propertyNames": names },
                { "type": "object", "minProperties": 3 }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn conditional_missing_else_negated_finite_property_names_bound_count() {
        let names = json!({
            "if": { "not": { "enum": ["a"] } },
            "then": { "enum": ["b"] }
        });
        let sub = resolve(json!({
            "type": "object",
            "propertyNames": names.clone()
        }));
        let sup = resolve(json!({
            "oneOf": [
                { "type": "object", "propertyNames": names },
                { "type": "object", "minProperties": 3 }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
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
    fn conditional_missing_then_uses_guard_and_else_cover() {
        let subset = resolve(json!({
            "if": { "type": "integer" },
            "else": { "enum": [false, null] }
        }));
        let superset = resolve(json!({
            "anyOf": [
                { "type": "integer" },
                { "enum": [false, null] }
            ]
        }));

        assert!(is_subschema_of(&subset, &superset));
    }

    #[test]
    fn conditional_missing_then_requires_guard_cover() {
        let subset = resolve(json!({
            "if": { "type": "integer" },
            "else": { "const": false }
        }));
        let superset = resolve(json!({ "enum": [false] }));

        assert!(!is_subschema_of(&subset, &superset));
    }

    #[test]
    fn conditional_vacuous_then_branch_can_use_else_subset() {
        let subset = resolve(json!({
            "if": { "type": "boolean" },
            "then": { "anyOf": [{ "type": "object" }] },
            "else": { "const": 1 }
        }));
        let superset = resolve(json!({ "type": "integer" }));

        assert!(is_subschema_of(&subset, &superset));
    }

    #[test]
    fn conditional_vacuous_then_detects_negated_guard_literal() {
        let subset = resolve(json!({
            "if": { "not": { "const": 1 } },
            "then": { "const": 1 },
            "else": { "const": 1 }
        }));
        let superset = resolve(json!({ "const": 1 }));

        // The then side is `not const(1) && const(1)`, so it is empty even
        // though both schemas have the same broad numeric type mask.
        assert!(is_subschema_of(&subset, &superset));
    }

    #[test]
    fn conditional_vacuous_else_branch_can_use_then_subset() {
        let subset = resolve(json!({
            "if": { "type": "integer" },
            "then": { "const": 1 },
            "else": { "type": "integer" }
        }));
        let superset = resolve(json!({ "const": 1 }));

        assert!(is_subschema_of(&subset, &superset));
    }

    #[test]
    fn conditional_overlapping_else_branch_stays_live() {
        let subset = resolve(json!({
            "if": { "type": "integer" },
            "then": { "const": 1 },
            "else": { "type": "string" }
        }));
        let superset = resolve(json!({ "const": 1 }));

        assert!(!is_subschema_of(&subset, &superset));
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
    fn unique_split_allof_integer_range_has_finite_capacity() {
        let subset = resolve(json!({
            "type": "array",
            "uniqueItems": true,
            "items": {
                "allOf": [
                    { "type": "integer", "minimum": 0 },
                    { "type": "integer", "maximum": 1 }
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
    fn unique_number_multiple_domain_is_not_treated_as_finite_capacity() {
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

        // Near-multiple floating values make this domain infinite under the
        // validator's epsilon semantics, so do not cap uniqueItems by the
        // projected integer multiples.
        assert!(!is_subschema_of(&subset, &superset));
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

    #[test]
    fn anyof_required_or_property_schema_is_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "required": ["tag"] },
                { "properties": { "tag": { "const": "ok" } } }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn property_presence_cover_with_extra_absence_constraint_is_not_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "required": ["tag"] },
                { "properties": { "tag": { "const": "ok" }, "other": { "const": 1 } } }
            ]
        }));

        // An object like {"other": 2} has no tag and fails the second branch.
        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn explicit_non_object_complement_with_presence_split_is_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "not": { "type": "object" } },
                { "type": "object", "required": ["tag"] },
                { "type": "object", "properties": { "tag": { "const": "ok" } } }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn required_or_same_trigger_dependent_required_is_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "required": ["tag"] },
                { "dependentRequired": { "tag": ["other"] } }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn required_or_other_trigger_dependent_required_is_not_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "required": ["tag"] },
                { "dependentRequired": { "other": ["missing"] } }
            ]
        }));

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn anyof_object_count_threshold_cover_is_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "maxProperties": 0 },
                { "minProperties": 1 }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn anyof_empty_object_arm_plus_nonempty_count_cover_is_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "properties": { "tag": { "const": "ok" } } },
                { "minProperties": 1 }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn anyof_object_count_bridge_intervals_are_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "not": { "type": "object" } },
                { "type": "object", "maxProperties": 0 },
                { "type": "object", "minProperties": 1, "maxProperties": 2 },
                { "type": "object", "minProperties": 3 }
            ]
        }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn anyof_object_count_threshold_gap_is_not_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "maxProperties": 1 },
                { "minProperties": 3 }
            ]
        }));

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn property_absence_and_dependent_target_presence_cover_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "properties": { "tag": { "const": "ok" } } },
                { "dependentRequired": { "other": ["tag"] } }
            ]
        }));

        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn dependent_target_presence_cover_with_extra_dependency_is_not_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "properties": { "tag": { "const": "ok" } } },
                { "dependentRequired": { "other": ["tag", "extra"] } }
            ]
        }));

        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn anyof_single_property_name_partition_with_count_gap_is_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "minProperties": 2 },
                { "maxProperties": 1, "propertyNames": { "enum": ["a"] } },
                { "maxProperties": 1, "propertyNames": { "not": { "enum": ["a"] } } }
            ]
        }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn property_name_partition_without_high_count_gap_is_not_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "maxProperties": 1, "propertyNames": { "enum": ["a"] } },
                { "maxProperties": 1, "propertyNames": { "not": { "enum": ["a"] } } }
            ]
        }));
        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn anyof_required_property_value_partition_is_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "not": { "required": ["a"] } },
                { "required": ["a"], "properties": { "a": { "enum": [1] } } },
                { "required": ["a"], "properties": { "a": { "not": { "enum": [1] } } } }
            ]
        }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn property_value_partition_with_extra_required_name_is_not_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "not": { "required": ["a"] } },
                { "required": ["a", "b"], "properties": { "a": { "enum": [1] } } },
                { "required": ["a"], "properties": { "a": { "not": { "enum": [1] } } } }
            ]
        }));
        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn anyof_prefix_item_complement_partition_is_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "prefixItems": [ { "enum": [1] } ] },
                { "prefixItems": [ { "not": { "enum": [1] } } ] }
            ]
        }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn prefix_item_partition_with_tail_bound_is_not_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "maxItems": 1, "prefixItems": [ { "enum": [1] } ] },
                { "maxItems": 1, "prefixItems": [ { "not": { "enum": [1] } } ] }
            ]
        }));
        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn anyof_items_or_contains_complement_is_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "items": { "enum": [1] } },
                { "contains": { "not": { "enum": [1] } } }
            ]
        }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn items_contains_partition_with_max_contains_is_not_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "items": { "enum": [1] } },
                { "contains": { "not": { "enum": [1] } }, "maxContains": 1 }
            ]
        }));
        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn anyof_not_items_or_contains_positive_is_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "items": { "not": { "enum": [1] } } },
                { "contains": { "enum": [1] } }
            ]
        }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn anyof_array_count_threshold_cover_is_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "maxItems": 1 },
                { "minItems": 2 }
            ]
        }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn anyof_array_count_bridge_intervals_are_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "not": { "type": "array" } },
                { "type": "array", "maxItems": 0 },
                { "type": "array", "minItems": 1, "maxItems": 2 },
                { "type": "array", "minItems": 3 }
            ]
        }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn anyof_array_count_gap_is_not_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "maxItems": 1 },
                { "minItems": 3 }
            ]
        }));
        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn anyof_string_length_threshold_cover_is_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "maxLength": 1 },
                { "minLength": 2 }
            ]
        }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn anyof_string_length_bridge_intervals_are_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "not": { "type": "string" } },
                { "type": "string", "maxLength": 1 },
                { "type": "string", "minLength": 2, "maxLength": 3 },
                { "type": "string", "minLength": 4 }
            ]
        }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn anyof_string_length_gap_is_not_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "maxLength": 1 },
                { "minLength": 3 }
            ]
        }));
        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn anyof_numeric_touching_range_cover_is_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "maximum": 1 },
                { "exclusiveMinimum": 1 }
            ]
        }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn anyof_numeric_bridge_intervals_are_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "not": { "type": "number" } },
                { "type": "number", "maximum": 0 },
                { "type": "number", "minimum": 0, "maximum": 2 },
                { "type": "number", "minimum": 2 }
            ]
        }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn anyof_numeric_bridge_open_point_gap_is_not_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "not": { "type": "number" } },
                { "type": "number", "exclusiveMaximum": 0 },
                { "type": "number", "exclusiveMinimum": 0, "maximum": 2 },
                { "type": "number", "minimum": 2 }
            ]
        }));
        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn anyof_numeric_open_point_gap_is_not_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "exclusiveMaximum": 1 },
                { "exclusiveMinimum": 1 }
            ]
        }));
        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn anyof_integer_lattice_with_noninteger_complement_is_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "not": { "type": "integer" } },
                { "type": "integer", "maximum": 1 },
                { "type": "integer", "minimum": 2 }
            ]
        }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn anyof_integer_lattice_bridge_intervals_cover_gap() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "not": { "type": "integer" } },
                { "type": "integer", "maximum": 0 },
                { "type": "integer", "minimum": 1, "maximum": 4 },
                { "type": "integer", "minimum": 5 }
            ]
        }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn anyof_integer_lattice_gap_with_noninteger_complement_is_not_universal() {
        let sub = resolve(json!({}));
        let sup = resolve(json!({
            "anyOf": [
                { "not": { "type": "integer" } },
                { "type": "integer", "maximum": 1 },
                { "type": "integer", "minimum": 3 }
            ]
        }));
        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn finite_oneof_of_complements_can_prove_xor_subset() {
        // The new schema accepts only []: it is the symmetric difference of
        // { } and { [], {} }.  The old schema accepts arrays (and 3), so the
        // finite xor is a safe subset even though neither complement branch is
        // finite by itself.
        let old = resolve(json!({
            "oneOf": [
                { "not": { "const": 3 } },
                { "not": { "type": "array" } }
            ]
        }));
        let new = resolve(json!({
            "oneOf": [
                { "not": { "const": {} } },
                { "not": { "enum": [[], {}] } }
            ]
        }));
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn impossible_type_intersection_is_subset_of_anything() {
        let sub = resolve(json!({
            "allOf": [
                { "type": "string" },
                { "type": "number" }
            ]
        }));
        let sup = resolve(json!({ "const": 42 }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn disjoint_complement_xor_collapses_to_union_subset() {
        let old = resolve(json!({
            "anyOf": [
                { "type": "string" },
                { "type": "number" }
            ]
        }));
        let new = resolve(json!({
            "oneOf": [
                { "not": { "type": "string" } },
                { "not": { "type": "number" } }
            ]
        }));
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn overlapping_complement_xor_does_not_collapse_to_union() {
        // Excluded regions overlap (integers are numbers), so the xor is the
        // non-integer-number gap rather than their union. Do not prove it as a
        // subset of integers.
        let old = resolve(json!({ "type": "integer" }));
        let new = resolve(json!({
            "oneOf": [
                { "not": { "type": "integer" } },
                { "not": { "type": "number" } }
            ]
        }));
        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn three_way_complement_xor_needs_only_two_covering_inners() {
        // In a three-way xor of complements, each accepted arm satisfies the
        // other two excluded regions.  A and B are both contained by `old`, so
        // every arm is covered even though C also admits arrays.
        let old = resolve(json!({ "type": ["string", "number", "boolean"] }));
        let new = resolve(json!({
            "oneOf": [
                { "not": { "type": ["string", "number"] } },
                { "not": { "type": ["number", "boolean"] } },
                { "not": { "type": ["boolean", "array"] } }
            ]
        }));
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn negated_singleton_untyped_numeric_assertion_forces_number() {
        let old = resolve(json!({ "type": "number" }));
        let new = resolve(json!({ "not": { "oneOf": [ { "maximum": 1 } ] } }));
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn negated_singleton_untyped_string_assertion_forces_string() {
        let old = resolve(json!({ "type": "string" }));
        let new = resolve(json!({ "not": { "oneOf": [ { "maxLength": 1 } ] } }));
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn disjoint_three_way_complement_xor_is_empty() {
        let old = resolve(json!({ "const": 42 }));
        let new = resolve(json!({
            "oneOf": [
                { "not": { "type": "string" } },
                { "not": { "type": "number" } },
                { "not": { "type": "boolean" } }
            ]
        }));
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn three_way_complement_xor_with_one_covering_inner_stays_conservative() {
        let old = resolve(json!({ "type": "number" }));
        let new = resolve(json!({
            "oneOf": [
                { "not": { "type": "number" } },
                { "not": { "type": ["number", "array"] } },
                { "not": { "type": ["number", "string"] } }
            ]
        }));
        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn overlapping_complement_xor_is_still_subset_of_excluded_union() {
        // Symmetric difference of integers and numbers is the non-integer
        // numbers, which is still contained by the broader number schema.
        let old = resolve(json!({ "type": "number" }));
        let new = resolve(json!({
            "oneOf": [
                { "not": { "type": "integer" } },
                { "not": { "type": "number" } }
            ]
        }));
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn duplicate_two_arm_oneof_is_empty() {
        let old = resolve(json!(false));
        let new = resolve(json!({ "oneOf": [ { "type": "string" }, { "type": "string" } ] }));
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn false_arm_two_way_oneof_normalizes_to_live_branch() {
        let old = resolve(json!({ "type": "string" }));
        let new = resolve(json!({ "oneOf": [ false, { "type": "string", "maxLength": 3 } ] }));
        assert!(is_subschema_of(&new, &old));

        let wrapped_old = resolve(json!({ "oneOf": [ { "type": "string" }, false ] }));
        assert!(is_subschema_of(&new, &wrapped_old));
    }

    #[test]
    fn oneof_with_multiple_empty_arms_reduces_to_live_branch() {
        let old = resolve(json!({ "type": "integer" }));
        let new = resolve(
            json!({ "oneOf": [ false, { "enum": [] }, { "type": "integer", "minimum": 0 } ] }),
        );
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn negated_impossible_intersection_is_universal_superset() {
        let old =
            resolve(json!({ "not": { "allOf": [ { "type": "array" }, { "type": "string" } ] } }));
        let new = resolve(json!({ "anyOf": [ { "maxLength": 1 }, { "not": false } ] }));
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn negated_oneof_complement_disjoint_pair_contains_positive_side() {
        let old = resolve(json!({
            "not": { "oneOf": [ { "not": { "type": "null" } }, { "type": "number" } ] }
        }));
        let new = resolve(json!({ "type": "number" }));
        assert!(is_subschema_of(&new, &old));

        let null_side = resolve(json!({ "type": "null" }));
        assert!(is_subschema_of(&null_side, &old));
    }

    #[test]
    fn negated_two_arm_oneof_contains_intersection_side() {
        let old = resolve(json!({ "not": { "oneOf": [ { "minLength": 1 }, { "maximum": 1 } ] } }));
        let new = resolve(json!({ "oneOf": [ { "type": "object" }, { "enum": ["a", "aa"] } ] }));
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn negated_two_arm_oneof_contains_neither_side() {
        let old =
            resolve(json!({ "not": { "oneOf": [ { "type": "string" }, { "type": "number" } ] } }));
        let new = resolve(json!({ "type": "object" }));
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn negated_untyped_minimum_becomes_number_upper_halfline() {
        let old = resolve(json!({ "type": "number", "exclusiveMaximum": 0 }));
        let new = resolve(json!({ "not": { "minimum": 0 } }));
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn negated_untyped_maximum_becomes_number_lower_halfline() {
        let old = resolve(json!({ "type": "number", "exclusiveMinimum": 1 }));
        let new = resolve(json!({ "not": { "maximum": 1 } }));
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn negated_untyped_minlength_becomes_string_upper_count() {
        let old = resolve(json!({ "type": "string", "maxLength": 0 }));
        let new = resolve(json!({ "not": { "minLength": 1 } }));
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn negated_untyped_maxlength_becomes_string_lower_count() {
        let old = resolve(json!({ "type": "string", "minLength": 2 }));
        let new = resolve(json!({ "not": { "maxLength": 1 } }));
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn negated_untyped_minitems_becomes_array_upper_count() {
        let old = resolve(json!({ "type": "array", "maxItems": 1 }));
        let new = resolve(json!({ "not": { "minItems": 2 } }));
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn negated_untyped_maxitems_becomes_array_lower_count() {
        let old = resolve(json!({ "type": "array", "minItems": 3 }));
        let new = resolve(json!({ "not": { "maxItems": 2 } }));
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn negated_untyped_minproperties_becomes_object_upper_count() {
        let old = resolve(json!({ "type": "object", "maxProperties": 1 }));
        let new = resolve(json!({ "not": { "minProperties": 2 } }));
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn negated_untyped_maxproperties_becomes_object_lower_count() {
        let old = resolve(json!({ "type": "object", "minProperties": 3 }));
        let new = resolve(json!({ "not": { "maxProperties": 2 } }));
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn mixed_oneof_disjoint_complement_is_subset_of_negated_side() {
        let old = resolve(json!({ "not": { "type": "array" } }));
        let new = resolve(json!({
            "oneOf": [ { "type": "array" }, { "not": { "const": {} } } ]
        }));
        assert!(is_subschema_of(&new, &old));

        let old_other = resolve(json!({ "not": { "const": {} } }));
        assert!(is_subschema_of(&new, &old_other));
    }

    #[test]
    fn mixed_oneof_disjoint_complement_fits_negated_union_arm() {
        let old = resolve(json!({
            "anyOf": [ { "type": "object" }, { "not": { "enum": [1, 2] } } ]
        }));
        let new = resolve(json!({
            "oneOf": [ { "type": "number" }, { "not": { "type": "object" } } ]
        }));
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn mixed_oneof_disjoint_complement_fits_negated_intersection_target() {
        let old = resolve(json!({
            "allOf": [ { "not": { "const": 0 } }, { "not": { "const": 1 } } ]
        }));
        let new = resolve(json!({
            "oneOf": [ { "type": "integer" }, { "not": { "const": [1] } } ]
        }));
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn mixed_oneof_disjoint_complement_fits_comparable_mixed_xor_target() {
        let old = resolve(json!({
            "oneOf": [ { "const": false }, { "not": { "type": "boolean" } } ]
        }));
        let new = resolve(json!({
            "oneOf": [ { "const": true }, { "not": { "const": {} } } ]
        }));
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn mixed_oneof_disjoint_complement_fits_union_with_finite_negated_remainder() {
        let old = resolve(json!({
            "anyOf": [ { "const": 1 }, { "not": { "enum": [1, 2] } } ]
        }));
        let new = resolve(json!({
            "oneOf": [ { "const": 2 }, { "not": { "type": "string" } } ]
        }));
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn comparable_positive_oneof_difference_fits_negated_union() {
        let old = resolve(json!({
            "not": { "anyOf": [ { "const": false }, { "const": "b" } ] }
        }));
        let new = resolve(json!({
            "oneOf": [ { "const": "b" }, { "type": "string" } ]
        }));
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn negated_double_complement_xor_fits_mixed_finite_difference() {
        let old = resolve(json!({
            "oneOf": [ { "enum": [1, "a", null] }, { "not": { "const": 1 } } ]
        }));
        let new = resolve(json!({
            "not": { "oneOf": [ { "not": { "type": "null" } }, { "not": { "type": "string" } } ] }
        }));
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn negated_disjoint_xor_fits_mixed_finite_difference() {
        let old = resolve(json!({
            "oneOf": [ { "enum": [[], {}] }, { "not": { "const": {} } } ]
        }));
        let new = resolve(json!({
            "not": { "oneOf": [ { "const": 3 }, { "type": "array" } ] }
        }));
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn negated_disjoint_xor_fits_mixed_opposite_finite_difference() {
        let old = resolve(json!({
            "oneOf": [ { "const": 1 }, { "not": { "enum": [1, 2] } } ]
        }));
        let new = resolve(json!({
            "not": { "oneOf": [ { "const": [2] }, { "enum": [2, 3] } ] }
        }));
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn negated_comparable_xor_excludes_mixed_finite_gap() {
        let old = resolve(json!({
            "oneOf": [ { "enum": [[], {}] }, { "not": { "const": {} } } ]
        }));
        let new = resolve(json!({
            "not": { "oneOf": [ { "const": 0 }, { "not": { "type": "object" } } ] }
        }));
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn negated_xor_covers_infinite_mixed_gap_with_finite_overlap() {
        let old = resolve(json!({
            "oneOf": [ { "type": "object" }, { "not": { "const": {} } } ]
        }));
        let new = resolve(json!({
            "not": { "oneOf": [ { "enum": [[], {}] }, { "type": "object" } ] }
        }));
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn constant_true_conditional_reduces_to_then_branch() {
        let sup = resolve(json!({
            "if": true,
            "then": { "allOf": [ { "type": "number" }, { "multipleOf": 1 } ] },
            "else": false
        }));
        let sub = resolve(json!({ "enum": [1, 3] }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn constant_false_conditional_reduces_to_else_branch() {
        let sup = resolve(json!({
            "if": false,
            "then": false,
            "else": { "type": "string", "enum": ["a", "b"] }
        }));
        let sub = resolve(json!({ "const": "a" }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn array_schema_fits_negated_numeric_union_by_type_mask() {
        let sup = resolve(json!({
            "not": { "anyOf": [ { "type": "number", "minimum": 0, "maximum": 2 } ] }
        }));
        let sub = resolve(json!({
            "type": "array",
            "prefixItems": [true],
            "items": { "type": "array" },
            "maxItems": 1
        }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn non_arrays_fit_negated_constant_true_conditional_array_branch() {
        let sup = resolve(json!({
            "not": {
                "if": true,
                "then": { "type": "array" },
                "else": { "const": 0 }
            }
        }));
        let sub = resolve(json!({
            "anyOf": [
                { "type": "integer" },
                { "type": "string" },
                { "type": "boolean" }
            ]
        }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn bounded_number_lattice_singleton_fits_integer_conditional_else() {
        let sup = resolve(json!({
            "if": { "type": "array", "minItems": 2 },
            "then": { "type": "object" },
            "else": { "type": "integer", "multipleOf": 2 }
        }));
        let sub = resolve(json!({
            "type": "number",
            "minimum": 0,
            "maximum": 0,
            "multipleOf": 2
        }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn nested_singleton_oneof_fits_negated_empty_enum_intersection() {
        let sub = resolve(json!({ "oneOf": [{ "oneOf": [true] }] }));
        let sup = resolve(json!({
            "not": {
                "allOf": [
                    { "type": "string", "minLength": 2, "maxLength": 2, "enum": ["b"] }
                ]
            }
        }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn impossible_contains_requirement_makes_array_branch_empty() {
        let sub = resolve(json!({
            "type": "array",
            "contains": false,
            "minContains": 1
        }));
        let sup = resolve(json!({ "type": "string" }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn disjoint_contains_matcher_makes_array_branch_empty_across_types() {
        let sup = resolve(json!({ "type": "object" }));
        let homogeneous = resolve(json!({
            "type": "array",
            "items": { "type": "string" },
            "contains": { "type": "number" },
            "minContains": 1
        }));
        assert!(is_subschema_of(&homogeneous, &sup));

        let closed_tuple = resolve(json!({
            "type": "array",
            "prefixItems": [{ "type": "string" }],
            "items": false,
            "minItems": 1,
            "contains": { "type": "number" },
            "minContains": 1
        }));
        assert!(is_subschema_of(&closed_tuple, &sup));
    }

    #[test]
    fn locally_impossible_unique_array_branch_is_vacuous_across_types() {
        let sup = resolve(json!({ "type": "string" }));
        let finite_tail = resolve(json!({
            "type": "array",
            "uniqueItems": true,
            "minItems": 2,
            "items": { "enum": [1] }
        }));
        assert!(is_subschema_of(&finite_tail, &sup));

        let repeated_prefix = resolve(json!({
            "type": "array",
            "uniqueItems": true,
            "prefixItems": [{ "const": 1 }, { "const": 1 }],
            "minItems": 2
        }));
        assert!(is_subschema_of(&repeated_prefix, &sup));
    }

    #[test]
    fn negated_split_allof_object_property_contradiction_is_universal() {
        let sup = resolve(json!({
            "not": {
                "allOf": [
                    { "type": "object", "required": ["a"], "properties": { "a": { "type": "string" } } },
                    { "type": "object", "required": ["a"], "properties": { "a": { "type": "number" } } }
                ]
            }
        }));
        let sub = resolve(json!(true));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn negated_split_allof_array_items_contains_contradiction_is_universal() {
        let sup = resolve(json!({
            "not": {
                "allOf": [
                    { "type": "array", "items": { "type": "string" } },
                    { "type": "array", "contains": { "type": "number" }, "minContains": 1 }
                ]
            }
        }));
        let sub = resolve(json!(true));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn negated_split_allof_array_length_contradiction_is_universal() {
        let sup = resolve(json!({
            "not": { "allOf": [
                { "type": "array", "minItems": 3 },
                { "type": "array", "maxItems": 1 }
            ] }
        }));
        assert!(is_subschema_of(&resolve(json!(true)), &sup));
    }

    #[test]
    fn negated_split_allof_object_count_contradiction_is_universal() {
        let sup = resolve(json!({
            "not": { "allOf": [
                { "type": "object", "minProperties": 3 },
                { "type": "object", "maxProperties": 1 }
            ] }
        }));
        assert!(is_subschema_of(&resolve(json!(true)), &sup));
    }

    #[test]
    fn negated_split_allof_numeric_range_contradiction_is_universal() {
        let sup = resolve(json!({
            "not": {
                "allOf": [
                    { "type": "number", "minimum": 3 },
                    { "type": "number", "maximum": 1 }
                ]
            }
        }));
        let sub = resolve(json!(true));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn negated_split_allof_string_length_contradiction_is_universal() {
        let sup = resolve(json!({
            "not": {
                "allOf": [
                    { "type": "string", "minLength": 3 },
                    { "type": "string", "maxLength": 1 }
                ]
            }
        }));
        let sub = resolve(json!(true));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn negated_duplicate_oneof_branches_are_universal() {
        let sup = resolve(json!({
            "not": { "oneOf": [ { "type": "string" }, { "type": "string" } ] }
        }));
        let sub = resolve(json!(true));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn negated_conditional_with_locally_impossible_branches_is_universal() {
        let sup = resolve(json!({
            "not": {
                "if": { "type": "string" },
                "then": {
                    "type": "array",
                    "uniqueItems": true,
                    "minItems": 3,
                    "items": { "type": "boolean" }
                },
                "else": {
                    "type": "object",
                    "properties": { "a": false },
                    "required": ["a"]
                }
            }
        }));
        let sub = resolve(json!(true));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn negated_locally_impossible_unique_array_is_universal() {
        let sup = resolve(json!({
            "not": {
                "type": "array",
                "uniqueItems": true,
                "minItems": 3,
                "items": { "type": "boolean" }
            }
        }));
        let sub = resolve(json!(true));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn locally_impossible_object_branch_is_vacuous_across_types() {
        let sub = resolve(json!({
            "type": "object",
            "properties": { "a": false },
            "required": ["a"]
        }));
        let sup = resolve(json!({ "type": "array" }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn locally_empty_contains_matcher_makes_array_branch_empty() {
        let sub = resolve(json!({
            "type": "array",
            "contains": { "type": "string", "enum": [1] },
            "minContains": 1
        }));
        let sup = resolve(json!({ "type": "number" }));
        assert!(is_subschema_of(&sub, &sup));

        let constrained = resolve(json!({
            "type": "array",
            "contains": { "type": "string", "minLength": 2, "maxLength": 2, "enum": ["b"] },
            "minContains": 1
        }));
        assert!(is_subschema_of(&constrained, &sup));
    }

    #[test]
    fn forbidden_optional_property_skips_superset_property_constraint() {
        let sub = resolve(json!({
            "type": "object",
            "propertyNames": { "enum": ["b"] }
        }));
        let sup = resolve(json!({
            "type": "object",
            "properties": { "k": { "type": "number" } }
        }));
        assert!(is_subschema_of(&sub, &sup));
    }

    #[test]
    fn oneof_object_presence_partition_allows_optional_property_target() {
        let sub = resolve(json!({
            "oneOf": [
                { "type": "object" },
                { "type": "object", "required": ["a"] }
            ]
        }));
        let sup = resolve(json!({
            "type": "object",
            "properties": { "a": { "type": "number" } }
        }));
        assert!(is_subschema_of(&sub, &sup));

        let conditional = resolve(json!({
            "if": { "type": "string", "minLength": 1 },
            "then": { "type": "integer" },
            "else": {
                "type": "object",
                "properties": { "a": { "type": "number" } }
            }
        }));
        assert!(is_subschema_of(&sub, &conditional));
    }

    #[test]
    fn oneof_object_presence_partition_does_not_ignore_other_requirements() {
        let sub = resolve(json!({
            "oneOf": [
                { "type": "object" },
                { "type": "object", "required": ["a"] }
            ]
        }));
        let sup = resolve(json!({
            "type": "object",
            "required": ["b"]
        }));
        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn oneof_array_nonempty_partition_is_empty_array() {
        let sub = resolve(json!({
            "oneOf": [
                { "type": "array" },
                { "type": "array", "minItems": 1 }
            ]
        }));
        let sup = resolve(json!({ "type": "array", "maxItems": 1 }));
        assert!(is_subschema_of(&sub, &sup));

        let conditional = resolve(json!({
            "if": { "type": "object" },
            "then": false,
            "else": { "type": "array", "maxItems": 0 }
        }));
        assert!(is_subschema_of(&sub, &conditional));
    }

    #[test]
    fn oneof_array_nonempty_partition_respects_contains_minimum() {
        let sub = resolve(json!({
            "oneOf": [
                { "type": "array" },
                { "type": "array", "minItems": 1 }
            ]
        }));
        let sup = resolve(json!({
            "type": "array",
            "contains": true,
            "minContains": 1
        }));
        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn oneof_string_nonempty_partition_is_empty_string() {
        let sub = resolve(json!({
            "oneOf": [
                { "type": "string" },
                { "type": "string", "minLength": 1 }
            ]
        }));
        let sup = resolve(json!({ "type": "string", "maxLength": 0 }));
        assert!(is_subschema_of(&sub, &sup));

        let conditional = resolve(json!({
            "if": { "type": "array" },
            "then": false,
            "else": { "type": "string", "maxLength": 0 }
        }));
        assert!(is_subschema_of(&sub, &conditional));
    }

    #[test]
    fn oneof_string_nonempty_partition_does_not_assume_patterns() {
        let sub = resolve(json!({
            "oneOf": [
                { "type": "string" },
                { "type": "string", "minLength": 1 }
            ]
        }));
        let sup = resolve(json!({ "type": "string", "pattern": "^x" }));
        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn oneof_object_nonempty_partition_is_empty_object() {
        let sub = resolve(json!({
            "oneOf": [
                { "type": "object" },
                { "type": "object", "minProperties": 1 }
            ]
        }));
        let sup = resolve(json!({
            "type": "object",
            "maxProperties": 0,
            "properties": { "a": false }
        }));
        assert!(is_subschema_of(&sub, &sup));

        let conditional = resolve(json!({
            "if": { "type": "array" },
            "then": false,
            "else": { "type": "object", "maxProperties": 0 }
        }));
        assert!(is_subschema_of(&sub, &conditional));
    }

    #[test]
    fn oneof_object_nonempty_partition_respects_required_target() {
        let sub = resolve(json!({
            "oneOf": [
                { "type": "object" },
                { "type": "object", "minProperties": 1 }
            ]
        }));
        let sup = resolve(json!({ "type": "object", "required": ["a"] }));
        assert!(!is_subschema_of(&sub, &sup));
    }

    #[test]
    fn oneof_exact_partition_shortcuts_preserve_extra_union_types() {
        let cases = [
            (
                json!({
                    "oneOf": [
                        { "type": "string" },
                        {
                            "anyOf": [
                                { "type": "string", "minLength": 1 },
                                { "type": "number" }
                            ]
                        }
                    ]
                }),
                json!({ "type": "string", "maxLength": 0 }),
            ),
            (
                json!({
                    "oneOf": [
                        { "type": "array" },
                        {
                            "anyOf": [
                                { "type": "array", "minItems": 1 },
                                { "type": "string" }
                            ]
                        }
                    ]
                }),
                json!({ "type": "array", "maxItems": 0 }),
            ),
            (
                json!({
                    "oneOf": [
                        { "type": "object" },
                        {
                            "anyOf": [
                                { "type": "object", "minProperties": 1 },
                                { "type": "string" }
                            ]
                        }
                    ]
                }),
                json!({ "type": "object", "maxProperties": 0 }),
            ),
            (
                json!({
                    "oneOf": [
                        { "type": "object" },
                        {
                            "anyOf": [
                                { "type": "object", "required": ["p"] },
                                { "type": "string" }
                            ]
                        }
                    ]
                }),
                json!({ "type": "object", "properties": { "p": false } }),
            ),
        ];

        for (sub, sup) in cases {
            assert!(!is_subschema_of(&resolve(sub), &resolve(sup)));
        }
    }
}
