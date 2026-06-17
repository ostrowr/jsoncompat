//! Boolean-applicator complement and partition helpers.
//!
//! These specialized `oneOf`/`anyOf`/`not` identities are proof helpers: false
//! means unknown, not incompatible.

use super::*;

pub(super) fn one_of_universal_arm_contains_subset(
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
pub(super) fn schema_disjoint_from_conditional(
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
pub(super) fn any_of_contains_known_universal_branch(
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
pub(super) fn any_of_complement_cover_is_universal(
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
pub(super) fn any_of_complement_union_cover_is_universal(
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
pub(super) fn complement_only_oneof_subset_of(
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
pub(super) fn comparable_oneof_difference_subset_of_negation(
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

pub(super) fn mixed_oneof_disjoint_complement_subset_of_target(
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
pub(super) fn mixed_oneof_disjoint_complement_subset_of_union_target(
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
pub(super) fn mixed_oneof_disjoint_complement_subset_of_mixed_target(
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
pub(super) fn mixed_oneof_disjoint_complement_subset_of_negation(
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
pub(super) fn negated_oneof_complement_pair_subset_of(
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
pub(super) fn negated_xor_covers_mixed_comparable_gap(
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
pub(super) fn negated_schema_excludes_mixed_finite_gap(
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
pub(super) fn negated_union_subset_of_mixed_difference(
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
pub(super) fn negated_complement_pair_subset_of_mixed_difference(
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
pub(super) fn negated_two_arm_oneof_contains(
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
pub(super) fn negated_oneof_complement_pair_contains(
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
pub(super) fn one_of_pair_is_syntactically_empty(branches: &[SchemaNode]) -> bool {
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
pub(super) fn one_of_trivial_live_branch(branches: &[SchemaNode]) -> Option<Option<&SchemaNode>> {
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
pub(super) fn one_of_pair_single_live_branch(branches: &[SchemaNode]) -> Option<&SchemaNode> {
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
pub(super) fn one_of_complement_pair_is_universal(
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
pub(super) fn one_of_complement_union_partition_is_universal(
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
pub(super) fn one_of_finite_complement_partition_is_universal(
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

pub(super) const JSON_TYPE_NULL: u8 = 1 << 0;
pub(super) const JSON_TYPE_BOOL: u8 = 1 << 1;
pub(super) const JSON_TYPE_NUMBER: u8 = 1 << 2;
pub(super) const JSON_TYPE_STRING: u8 = 1 << 3;
pub(super) const JSON_TYPE_ARRAY: u8 = 1 << 4;
pub(super) const JSON_TYPE_OBJECT: u8 = 1 << 5;
pub(super) const JSON_TYPE_ALL: u8 = JSON_TYPE_NULL
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

pub(super) fn complement_u64_count_halfline(range: CountRange<u64>) -> Option<CountRange<u64>> {
    match (range.min(), range.max()) {
        (0, None) => None,
        (min, None) => min
            .checked_sub(1)
            .and_then(|max| CountRange::new(0, Some(max))),
        (0, Some(max)) => max.checked_add(1).map(CountRange::unbounded_from),
        (_, Some(_)) => None,
    }
}

pub(super) fn complement_usize_count_halfline(
    range: CountRange<usize>,
) -> Option<CountRange<usize>> {
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
pub(super) fn negated_untyped_array_count_halfline_subset_of(
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
pub(super) fn negated_untyped_object_count_halfline_subset_of(
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
pub(super) fn negated_untyped_string_length_halfline_subset_of(
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
pub(super) fn negated_untyped_numeric_halfline_subset_of(
    branches: &[SchemaNode],
    sup: &SchemaNode,
) -> bool {
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

pub(super) fn negated_union_type_remainder_subset_of(
    branches: &[SchemaNode],
    sup: &SchemaNode,
) -> bool {
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
pub(super) fn any_of_obvious_type_cover_contains(
    sub: &SchemaNode,
    branches: &[SchemaNode],
) -> bool {
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
pub(super) fn schema_obviously_accepts_json_type(schema: &SchemaNode, bit: u8) -> bool {
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
pub(super) fn one_of_type_partition_contains(
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
pub(super) fn finite_subset_values_fit_anyof(
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
pub(super) fn finite_subset_values_fit_oneof_exactly(
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
