//! Ordered subset-dispatch pipeline and recursion entry point.
//!
//! This module owns the fragile rule ordering; leaf facts live in sibling modules.

use super::*;

/// Peel semantically-neutral singleton applicators.  `allOf`, `anyOf`, and
/// `oneOf` with exactly one child are all equivalent to that child; keeping
/// them wrapped tends to hide simple negation and type facts from the structural
/// prover.  Stop on cycles defensively (recursive refs can preserve wrappers).
pub(super) fn unwrap_singleton_applicators(mut node: &SchemaNode) -> &SchemaNode {
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

/// Try finite/value-level shortcuts before installing a recursion frame.
///
/// These checks are intentionally ordered before applicator dispatch: they
/// prove vacuity or finite coverage without creating misleading branch-level
/// explanations for schemas whose language is already known.
fn try_pre_recursion_shortcut(
    sub: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
    mode: ExplanationMode,
) -> Option<SubschemaAnalysis> {
    // A zero possible-type mask is a syntactic proof that the subset language
    // is empty (for example, an allOf intersection of disjoint JSON types).
    // The mask helper is an upper bound, so returning compatible here is safe
    // and avoids falling into branch-wise applicator explanations for
    // impossible schemas.
    if possible_json_type_mask(sub) == 0 {
        return Some(SubschemaAnalysis::compatible());
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
        return Some(SubschemaAnalysis::from_check(is_subschema, mode, || {
            Some(SubschemaExplanation::new(
                "enumerated values are not contained by the comparison target",
            ))
        }));
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
        && context.finite_upper_bound_fits_target(sub, sup, &values)
    {
        return Some(SubschemaAnalysis::compatible());
    }

    // Split allOf integer ranges can be finite even when no individual
    // conjunct is finite (e.g. `integer & minimum & maximum`). Enumerate a
    // small outward-rounded upper bound and require every live candidate to be
    // accepted by the target.
    if let Some(values) = finite_split_allof_integer_values(sub)
        && context.finite_upper_bound_fits_target(sub, sup, &values)
    {
        return Some(SubschemaAnalysis::compatible());
    }

    // Broad schemas such as `{}` hit the `Any` arm below before the normal
    // superset-`anyOf` handler. Give obvious whole-type unions a chance first.
    if let SchemaNodeKind::AnyOf(branches) = sup.kind()
        && any_of_obvious_type_cover_contains(sub, branches)
    {
        return Some(SubschemaAnalysis::compatible());
    }

    None
}

pub(super) fn analyze_subschema_with_context(
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

    if let Some(analysis) = try_pre_recursion_shortcut(sub, sup, context, mode) {
        return analysis;
    }

    let recursion_key = (sub.id(), sup.id());
    if let Some(is_guarded_reentry) = context.recursion_reentry_is_guarded(recursion_key) {
        // Productive recursion through object properties or array items can be
        // assumed coinductively. Same-value applicator cycles cannot: the raw
        // validator may distinguish `anyOf[self, T]` from `allOf[self, T]`.
        return SubschemaAnalysis::from_check(is_guarded_reentry, mode, || {
            explain_schema_kind_gap(sub, sup)
        });
    }

    context.with_recursion_pair(recursion_key, |context| {
        analyze_subschema_entered(sub, sup, context, mode)
    })
}

/// Run ordered shortcuts that require a registered recursion frame but happen
/// before the main `(sub.kind(), sup.kind())` dispatch.
fn try_before_kind_dispatch(
    sub: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
    mode: ExplanationMode,
) -> Option<SubschemaAnalysis> {
    // Check local array/object contradictions before JSON-type dispatch so they
    // remain vacuous even against a different target type.
    if subset_is_locally_vacuous_before_dispatch(sub) {
        return Some(SubschemaAnalysis::compatible());
    }

    if let Some(analysis) = try_normalize_trivial_one_of(sub, sup, context, mode) {
        return Some(analysis);
    }

    if let Some(analysis) = try_peel_exact_wrappers(sub, sup, context, mode) {
        return Some(analysis);
    }

    if predispatch_cover_proves_subset(sub, sup, context) {
        return Some(SubschemaAnalysis::compatible());
    }

    None
}

/// Analyze a pair after the recursion guard has been registered.
///
/// Keep all early exits inside this helper free of active-pair bookkeeping; the
/// outer wrapper is responsible for leaving the recursion stack exactly once.
pub(super) fn analyze_subschema_entered(
    sub: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
    mode: ExplanationMode,
) -> SubschemaAnalysis {
    if let Some(analysis) = try_before_kind_dispatch(sub, sup, context, mode) {
        return analysis;
    }

    analyze_kind_pair(sub, sup, context, mode)
}

/// Dispatch on the concrete schema-node kinds after all ordered shortcut phases.
fn analyze_kind_pair(
    sub: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
    mode: ExplanationMode,
) -> SubschemaAnalysis {
    use SchemaNodeKind::*;

    match (sub.kind(), sup.kind()) {
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
    }
}
