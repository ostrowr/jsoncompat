//! Explanation construction and final structural constraint dispatch.
//!
//! Kept separate from the core recursion so verdict logic and diagnostics can evolve independently.

use super::*;

pub(super) fn explain_any_of_to_any_of_failure(
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

pub(super) fn explain_one_of_to_one_of_failure(
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

pub(super) fn explain_subset_union_failure(
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

pub(super) fn explain_branch_against_sup(
    branch: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> Option<SubschemaExplanation> {
    match sup.kind() {
        SchemaNodeKind::AnyOf(sups) => explain_branch_against_union(branch, sups, context),
        _ => explain_subschema_failure_with_context(branch, sup, context),
    }
}

pub(super) fn explain_branch_against_union(
    branch: &SchemaNode,
    sups: &[SchemaNode],
    context: &mut SubschemaCheckContext,
) -> Option<SubschemaExplanation> {
    explain_superset_any_of_failure(branch, sups, context)
}

pub(super) fn explain_superset_any_of_failure(
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

pub(super) fn explain_superset_all_of_failure(
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

pub(super) fn explain_type_constraint_failure(
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

pub(super) fn explain_schema_kind_gap(
    sub: &SchemaNode,
    sup: &SchemaNode,
) -> Option<SubschemaExplanation> {
    (schema_kind_name(sub.kind()) != schema_kind_name(sup.kind())).then(|| {
        SubschemaExplanation::new(format!(
            "new values may be {}, but the previous schema only accepted {}",
            schema_kind_name(sub.kind()),
            schema_kind_name(sup.kind()),
        ))
    })
}

pub(super) fn explain_string_constraints(
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

pub(super) fn explain_number_constraints(
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

pub(super) fn explain_integer_constraints(
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

pub(super) fn explain_enumeration_gap(
    sub_enum: Option<&[Value]>,
    sup_enum: Option<&[Value]>,
) -> Option<SubschemaExplanation> {
    (!scalar::check_enum_inclusion(sub_enum, sup_enum)).then(|| {
        SubschemaExplanation::new("enumerated values are not contained by the comparison target")
    })
}

pub(super) fn number_multiple_of_constraints_subsumed(
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

pub(super) fn integer_multiple_of_constraints_subsumed(
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

pub(super) fn format_count_range<T: std::fmt::Display + Copy + Ord>(
    range: CountRange<T>,
) -> String {
    match range.max() {
        Some(max) if max == range.min() => format!("{}", range.min()),
        Some(max) => format!("{}..={max}", range.min()),
        None => format!("{}..", range.min()),
    }
}

pub(super) fn format_number_bounds(bounds: NumberBounds) -> String {
    format!(
        "{}{}, {}{}",
        number_lower_delimiter(bounds.lower()),
        format_number_bound_value(bounds.lower(), "-inf"),
        format_number_bound_value(bounds.upper(), "+inf"),
        number_upper_delimiter(bounds.upper()),
    )
}

pub(super) fn number_lower_delimiter(bound: NumberBound) -> &'static str {
    match bound {
        NumberBound::Exclusive(_) => "(",
        NumberBound::Inclusive(_) | NumberBound::Unbounded => "[",
    }
}

pub(super) fn number_upper_delimiter(bound: NumberBound) -> &'static str {
    match bound {
        NumberBound::Exclusive(_) => ")",
        NumberBound::Inclusive(_) | NumberBound::Unbounded => "]",
    }
}

pub(super) fn format_number_bound_value(bound: NumberBound, unbounded: &str) -> String {
    match bound {
        NumberBound::Unbounded => unbounded.to_owned(),
        NumberBound::Inclusive(value) | NumberBound::Exclusive(value) => value.to_string(),
    }
}

pub(super) fn format_integer_bounds(bounds: IntegerBounds) -> String {
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

pub(super) fn format_optional_number_multiple_of(multiple_of: Option<&NumberMultipleOf>) -> String {
    multiple_of.map_or_else(
        || "<none>".to_owned(),
        |multiple_of| multiple_of.as_f64().to_string(),
    )
}

pub(super) fn format_optional_integer_multiple_of(
    multiple_of: Option<&IntegerMultipleOf>,
) -> String {
    multiple_of.map_or_else(
        || "<none>".to_owned(),
        |multiple_of| multiple_of.as_f64().to_string(),
    )
}

pub(super) fn array_index_can_exist(max_items: Option<u64>, index: usize) -> bool {
    let Ok(index) = u64::try_from(index) else {
        return false;
    };
    max_items.is_none_or(|max_items| index < max_items)
}

pub(super) fn guaranteed_array_item_matches_at_least_for_explanation(
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

pub(super) fn schema_kind_name(kind: &SchemaNodeKind) -> &'static str {
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

pub(super) fn type_constraints_subsumed_with_context(
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
