use crate::ResolvedNode;
use json_schema_ast::{
    ArrayContains, IntegerMultipleOf, ResolvedNodeId, ResolvedNodeKind, json_values_equal,
    property_name_matches_pattern,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

#[derive(Default)]
struct SubschemaCheckContext {
    active_pairs: HashSet<(ResolvedNodeId, ResolvedNodeId)>,
}

impl SubschemaCheckContext {
    fn superset_contains_value(&mut self, sup: &ResolvedNode, value: &Value) -> bool {
        sup.accepts_value(value)
    }

    fn superset_contains_value_set(&mut self, sup: &ResolvedNode, values: &[Value]) -> bool {
        values
            .iter()
            .all(|value| self.superset_contains_value(sup, value))
    }
}

/// Returns `true` if **every** instance that satisfies `sub` also satisfies
/// `sup`.
pub fn is_subschema_of(sub: &ResolvedNode, sup: &ResolvedNode) -> bool {
    is_subschema_of_with_context(sub, sup, &mut SubschemaCheckContext::default())
}

fn is_subschema_of_with_context(
    sub: &ResolvedNode,
    sup: &ResolvedNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    if sub == sup {
        return true;
    }

    let recursion_key = (sub.id(), sup.id());
    if !context.active_pairs.insert(recursion_key) {
        return true;
    }

    use ResolvedNodeKind::*;

    let sub_kind = sub.kind().clone();
    let sup_kind = sup.kind().clone();

    let is_subschema = match (&sub_kind, &sup_kind) {
        (BoolSchema(false), _) => true,
        (_, BoolSchema(true)) => true,
        (Any, Any) => true,
        (_, Any) => true,
        (Any, _) => false,

        // Keep sub-combinator handlers before sup-combinator handlers so when
        // both sides are unions we reason branch-wise on `sub` first.
        (AnyOf(subs), _) | (OneOf(subs), _) => subs
            .iter()
            .all(|branch| is_subschema_of_with_context(branch, sup, context)),
        (AllOf(subs), _) => subs
            .iter()
            .all(|schema| is_subschema_of_with_context(schema, sup, context)),

        (Enum(sub_e), Enum(sup_e)) => sub_e.iter().all(|value| {
            sup_e
                .iter()
                .any(|expected| json_values_equal(expected, value))
        }),
        (Enum(sub_e), _) => context.superset_contains_value_set(sup, sub_e),

        (Const(s_val), Const(p_val)) => json_values_equal(s_val, p_val),
        (Const(s_val), _) => context.superset_contains_value(sup, s_val),

        (_, AnyOf(sups)) => sups
            .iter()
            .any(|branch| is_subschema_of_with_context(sub, branch, context)),
        (_, OneOf(_)) => false,
        (_, AllOf(sups)) => sups
            .iter()
            .all(|schema| is_subschema_of_with_context(sub, schema, context)),

        (
            Number {
                enumeration: Some(sub_enum),
                ..
            },
            Enum(_),
        ) => context.superset_contains_value_set(sup, sub_enum),

        (_, Enum(_)) => false,

        (Not(subn), _) => match subn.kind() {
            Any | BoolSchema(true) => true,
            BoolSchema(false) => !matches!(sup_kind, Any | BoolSchema(true)),
            _ => false,
        },
        (_, Not(supn)) => match supn.kind() {
            Any | BoolSchema(true) => matches!(sub_kind, BoolSchema(false)),
            BoolSchema(false) => matches!(sub_kind, BoolSchema(true) | Any),
            _ => false,
        },

        (String { .. }, String { .. })
        | (Number { .. }, Number { .. })
        | (Integer { .. }, Integer { .. })
        | (Boolean { .. }, Boolean { .. })
        | (Null { .. }, Null { .. })
        | (Object { .. }, Object { .. })
        | (Array { .. }, Array { .. }) => type_constraints_subsumed_with_context(sub, sup, context),

        (Integer { .. }, Number { .. }) => integer_constraints_subsumed_by_number(sub, sup),
        (
            Number {
                enumeration: Some(sub_enum),
                ..
            },
            Integer { .. } | Const(_),
        ) => context.superset_contains_value_set(sup, sub_enum),

        (_, Const(_)) => false,

        _ => false,
    };

    context.active_pairs.remove(&recursion_key);
    is_subschema
}

fn integer_constraints_subsumed_by_number(sub: &ResolvedNode, sup: &ResolvedNode) -> bool {
    use ResolvedNodeKind::*;

    let sub_kind = sub.kind().clone();
    let sup_kind = sup.kind().clone();

    let (
        Integer {
            minimum: smin,
            maximum: smax,
            exclusive_minimum: sexmin,
            exclusive_maximum: sexmax,
            multiple_of: smul,
            enumeration: s_en,
            ..
        },
        Number {
            minimum: pmin,
            maximum: pmax,
            exclusive_minimum: pexmin,
            exclusive_maximum: pexmax,
            multiple_of: pmul,
            enumeration: p_en,
            ..
        },
    ) = (sub_kind, sup_kind)
    else {
        return false;
    };

    if !check_numeric_inclusion(smin.map(|value| value as f64), sexmin, pmin, pexmin, true) {
        return false;
    }
    if !check_numeric_inclusion(smax.map(|value| value as f64), sexmax, pmax, pexmax, false) {
        return false;
    }
    if !check_integer_multiple_of_inclusion_by_number(smul, pmul) {
        return false;
    }
    check_enum_inclusion(s_en.as_deref(), p_en.as_deref())
}

/// Compare the **constraints** of two nodes of the *same* basic type.
pub fn type_constraints_subsumed(sub: &ResolvedNode, sup: &ResolvedNode) -> bool {
    type_constraints_subsumed_with_context(sub, sup, &mut SubschemaCheckContext::default())
}

fn type_constraints_subsumed_with_context(
    sub: &ResolvedNode,
    sup: &ResolvedNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    use ResolvedNodeKind::*;

    let sub_kind = sub.kind().clone();
    let sup_kind = sup.kind().clone();

    match (sub_kind, sup_kind) {
        (
            String {
                min_length: smin,
                max_length: smax,
                enumeration: s_enum,
                ..
            },
            String {
                min_length: pmin,
                max_length: pmax,
                enumeration: p_enum,
                ..
            },
        ) => {
            if let Some(pm) = pmin
                && smin.unwrap_or(0) < pm
            {
                return false;
            }
            if let Some(px) = pmax
                && smax.unwrap_or(u64::MAX) > px
            {
                return false;
            }
            if let (Some(se), Some(pe)) = (s_enum, p_enum)
                && !se.iter().all(|v| pe.contains(v))
            {
                return false;
            }
            true
        }

        (
            Number {
                minimum: smin,
                maximum: smax,
                exclusive_minimum: sexmin,
                exclusive_maximum: sexmax,
                multiple_of: smul,
                enumeration: s_en,
                ..
            },
            Number {
                minimum: pmin,
                maximum: pmax,
                exclusive_minimum: pexmin,
                exclusive_maximum: pexmax,
                multiple_of: pmul,
                enumeration: p_en,
                ..
            },
        ) => {
            if !check_numeric_inclusion(smin, sexmin, pmin, pexmin, true) {
                return false;
            }
            if !check_numeric_inclusion(smax, sexmax, pmax, pexmax, false) {
                return false;
            }
            if !check_multiple_of_inclusion(smul, pmul) {
                return false;
            }
            check_enum_inclusion(s_en.as_deref(), p_en.as_deref())
        }

        (
            Integer {
                minimum: smin,
                maximum: smax,
                exclusive_minimum: sexmin,
                exclusive_maximum: sexmax,
                multiple_of: smul,
                enumeration: s_en,
                ..
            },
            Integer {
                minimum: pmin,
                maximum: pmax,
                exclusive_minimum: pexmin,
                exclusive_maximum: pexmax,
                multiple_of: pmul,
                enumeration: p_en,
                ..
            },
        ) => {
            if !check_int_inclusion(smin, sexmin, pmin, pexmin, true) {
                return false;
            }
            if !check_int_inclusion(smax, sexmax, pmax, pexmax, false) {
                return false;
            }
            if !check_integer_multiple_of_inclusion(smul, pmul) {
                return false;
            }
            check_enum_inclusion(s_en.as_deref(), p_en.as_deref())
        }

        (Boolean { enumeration: s_e }, Boolean { enumeration: p_e })
        | (Null { enumeration: s_e }, Null { enumeration: p_e }) => {
            check_enum_inclusion(s_e.as_deref(), p_e.as_deref())
        }

        (
            Object {
                properties: sprops,
                pattern_properties: s_pattern_props,
                required: sreq,
                additional: s_addl,
                property_names: s_prop_names,
                min_properties: smin,
                max_properties: smax,
                dependent_required: _s_deps,
                enumeration: s_en,
            },
            Object {
                properties: pprops,
                pattern_properties: p_pattern_props,
                required: preq,
                additional: p_addl,
                property_names: p_prop_names,
                min_properties: pmin,
                max_properties: pmax,
                dependent_required: p_deps,
                enumeration: p_en,
            },
        ) => {
            if let Some(pm) = pmin
                && smin.unwrap_or(0) < pm
            {
                return false;
            }
            if let Some(px) = pmax
                && smax.unwrap_or(usize::MAX) > px
            {
                return false;
            }

            if !check_enum_inclusion(s_en.as_deref(), p_en.as_deref()) {
                return false;
            }

            if !preq.is_subset(&sreq) {
                return false;
            }

            for (key, s_schema) in &sprops {
                if !object_property_schema_is_subsumed(
                    key,
                    s_schema,
                    pprops.get(key),
                    &p_pattern_props,
                    &p_addl,
                    context,
                ) {
                    return false;
                }
            }

            for (pattern, s_schema) in &s_pattern_props {
                let p_schema = match p_pattern_props.get(pattern) {
                    Some(p_schema) => p_schema,
                    None if p_pattern_props.is_empty() => &p_addl,
                    None => return false,
                };
                if !is_subschema_of_with_context(s_schema, p_schema, context) {
                    return false;
                }
            }

            if !object_additional_schema_is_subsumed(
                &s_addl,
                &s_pattern_props,
                &p_pattern_props,
                &p_addl,
                context,
            ) {
                return false;
            }

            if !is_subschema_of_with_context(&s_prop_names, &p_prop_names, context) {
                return false;
            }

            for (trigger, deps) in p_deps.iter() {
                // If the superset requires extra keys whenever `trigger` exists,
                // then the subset may only allow `trigger` when those keys are
                // unconditionally present.
                let trigger_allowed = object_property_name_can_be_present(
                    trigger,
                    &sprops,
                    &s_pattern_props,
                    &s_prop_names,
                    &s_addl,
                );
                if trigger_allowed && !deps.iter().all(|d| sreq.contains(d)) {
                    return false;
                }
            }

            true
        }

        (
            Array {
                prefix_items: s_prefix_items,
                items: sitems,
                min_items: smin,
                max_items: smax,
                contains: s_contains,
                unique_items: s_unique_items,
                enumeration: s_en,
            },
            Array {
                prefix_items: p_prefix_items,
                items: pitems,
                min_items: pmin,
                max_items: pmax,
                contains: p_contains,
                unique_items: p_unique_items,
                enumeration: p_en,
            },
        ) => {
            if let Some(pm) = pmin
                && smin.unwrap_or(0) < pm
            {
                return false;
            }
            if let Some(px) = pmax
                && smax.unwrap_or(u64::MAX) > px
            {
                return false;
            }
            if p_unique_items && !s_unique_items && smax.unwrap_or(u64::MAX) > 1 {
                return false;
            }
            if !array_item_constraints_subsumed(
                &s_prefix_items,
                &sitems,
                smax,
                &p_prefix_items,
                &pitems,
                context,
            ) {
                return false;
            }
            if !array_contains_constraints_subsumed(
                &s_prefix_items,
                &sitems,
                smin,
                smax,
                s_contains.as_ref(),
                p_contains.as_ref(),
                context,
            ) {
                return false;
            }
            if !check_enum_inclusion(s_en.as_deref(), p_en.as_deref()) {
                return false;
            }
            true
        }

        _ => false,
    }
}

fn check_numeric_inclusion(
    s_val: Option<f64>,
    s_excl: bool,
    p_val: Option<f64>,
    p_excl: bool,
    is_min: bool,
) -> bool {
    if p_val.is_none() {
        return true;
    }
    let supv = p_val.unwrap();
    let subv = s_val.unwrap_or(if is_min { f64::MIN } else { f64::MAX });

    if is_min {
        if p_excl {
            if s_excl { subv >= supv } else { subv > supv }
        } else {
            subv >= supv
        }
    } else if p_excl {
        if s_excl { subv <= supv } else { subv < supv }
    } else {
        subv <= supv
    }
}

fn array_item_constraints_subsumed(
    sub_prefix_items: &[ResolvedNode],
    sub_items: &ResolvedNode,
    sub_max_items: Option<u64>,
    sup_prefix_items: &[ResolvedNode],
    sup_items: &ResolvedNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    let checked_prefix_len = sub_prefix_items.len().max(sup_prefix_items.len());
    for index in 0..checked_prefix_len {
        if !array_index_can_exist(sub_max_items, index) {
            return true;
        }

        let sub_item = sub_prefix_items.get(index).unwrap_or(sub_items);
        let sup_item = sup_prefix_items.get(index).unwrap_or(sup_items);
        if !is_subschema_of_with_context(sub_item, sup_item, context) {
            return false;
        }
    }

    !array_index_can_exist(sub_max_items, checked_prefix_len)
        || is_subschema_of_with_context(sub_items, sup_items, context)
}

fn array_contains_constraints_subsumed(
    sub_prefix_items: &[ResolvedNode],
    sub_items: &ResolvedNode,
    sub_min_items: Option<u64>,
    sub_max_items: Option<u64>,
    sub_contains: Option<&ArrayContains<ResolvedNode>>,
    sup_contains: Option<&ArrayContains<ResolvedNode>>,
    context: &mut SubschemaCheckContext,
) -> bool {
    let Some(sup_contains) = sup_contains else {
        return true;
    };

    let lower_bound_ok = sup_contains.min_contains == 0
        || sub_contains.is_some_and(|sub_contains| {
            sub_contains.min_contains >= sup_contains.min_contains
                && is_subschema_of_with_context(&sub_contains.schema, &sup_contains.schema, context)
        })
        || (sub_min_items.unwrap_or(0) >= sup_contains.min_contains
            && all_array_item_schemas_subsumed_by(
                sub_prefix_items,
                sub_items,
                sub_max_items,
                &sup_contains.schema,
                context,
            ));

    if !lower_bound_ok {
        return false;
    }

    let Some(sup_max_contains) = sup_contains.max_contains else {
        return true;
    };

    sub_contains
        .filter(|sub_contains| {
            sub_contains
                .max_contains
                .is_some_and(|sub_max_contains| sub_max_contains <= sup_max_contains)
                && is_subschema_of_with_context(&sup_contains.schema, &sub_contains.schema, context)
        })
        .is_some()
        || (sub_max_items.is_some_and(|sub_max_items| sub_max_items <= sup_max_contains)
            && all_array_item_schemas_subsumed_by(
                sub_prefix_items,
                sub_items,
                sub_max_items,
                &sup_contains.schema,
                context,
            ))
}

fn all_array_item_schemas_subsumed_by(
    prefix_items: &[ResolvedNode],
    items: &ResolvedNode,
    max_items: Option<u64>,
    sup_schema: &ResolvedNode,
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

fn array_index_can_exist(max_items: Option<u64>, index: usize) -> bool {
    let Ok(index) = u64::try_from(index) else {
        return false;
    };
    max_items.is_none_or(|max_items| index < max_items)
}

fn check_multiple_of_inclusion(sub_multiple_of: Option<f64>, sup_multiple_of: Option<f64>) -> bool {
    let Some(sup_multiple_of) = sup_multiple_of else {
        return true;
    };
    let Some(sub_multiple_of) = sub_multiple_of else {
        return false;
    };
    if sub_multiple_of <= 0.0 || sup_multiple_of <= 0.0 {
        return false;
    }

    if let (Some(sub_multiple_of), Some(sup_multiple_of)) = (
        exact_positive_integer(sub_multiple_of),
        exact_positive_integer(sup_multiple_of),
    ) {
        return sub_multiple_of % sup_multiple_of == 0;
    }

    let ratio = sub_multiple_of / sup_multiple_of;
    (ratio - ratio.round()).abs() <= f64::EPSILON * ratio.abs().max(1.0) * 4.0
}

fn check_integer_multiple_of_inclusion(
    sub_multiple_of: Option<IntegerMultipleOf>,
    sup_multiple_of: Option<IntegerMultipleOf>,
) -> bool {
    let Some(sup_multiple_of) = sup_multiple_of else {
        return true;
    };
    let Some(sub_multiple_of) = sub_multiple_of else {
        return false;
    };

    if let (Some(sub_divisor), Some(sup_divisor)) = (
        sub_multiple_of.integer_divisor(),
        sup_multiple_of.integer_divisor(),
    ) {
        return sub_divisor.rem_euclid(sup_divisor) == 0;
    }

    let ratio = sub_multiple_of.as_f64() / sup_multiple_of.as_f64();
    (ratio - ratio.round()).abs() <= f64::EPSILON * ratio.abs().max(1.0) * 4.0
}

fn check_integer_multiple_of_inclusion_by_number(
    sub_multiple_of: Option<IntegerMultipleOf>,
    sup_multiple_of: Option<f64>,
) -> bool {
    let Some(sup_multiple_of) = sup_multiple_of else {
        return true;
    };
    let Some(sub_multiple_of) = sub_multiple_of else {
        return false;
    };
    if sup_multiple_of <= 0.0 {
        return false;
    }

    if let Some(sup_multiple_of) = exact_positive_integer(sup_multiple_of)
        && let Ok(sup_multiple_of) = i64::try_from(sup_multiple_of)
        && let Some(sub_multiple_of) = sub_multiple_of.integer_divisor()
    {
        return sub_multiple_of.rem_euclid(i128::from(sup_multiple_of)) == 0;
    }

    let ratio = sub_multiple_of.as_f64() / sup_multiple_of;
    (ratio - ratio.round()).abs() <= f64::EPSILON * ratio.abs().max(1.0) * 4.0
}

fn exact_positive_integer(value: f64) -> Option<u64> {
    if !value.is_finite() || value <= 0.0 || value.fract() != 0.0 || value > u64::MAX as f64 {
        return None;
    }

    let integer = value as u64;
    ((integer as f64) == value).then_some(integer)
}

fn check_enum_inclusion(sub_enum: Option<&[Value]>, sup_enum: Option<&[Value]>) -> bool {
    match (sub_enum, sup_enum) {
        (_, None) => true,
        (Some(sub_enum), Some(sup_enum)) => sub_enum.iter().all(|value| {
            sup_enum
                .iter()
                .any(|expected| json_values_equal(expected, value))
        }),
        (None, Some(_)) => false,
    }
}

fn check_int_inclusion(
    s_val: Option<i64>,
    s_excl: bool,
    p_val: Option<i64>,
    p_excl: bool,
    is_min: bool,
) -> bool {
    if p_val.is_none() {
        return true;
    }
    let supv = p_val.unwrap();
    let subv = s_val.unwrap_or(if is_min { i64::MIN } else { i64::MAX });

    if is_min {
        if p_excl {
            if s_excl { subv >= supv } else { subv > supv }
        } else {
            subv >= supv
        }
    } else if p_excl {
        if s_excl { subv <= supv } else { subv < supv }
    } else {
        subv <= supv
    }
}

fn object_property_schema_is_subsumed(
    property_name: &str,
    sub_schema: &ResolvedNode,
    sup_property_schema: Option<&ResolvedNode>,
    sup_pattern_properties: &HashMap<String, ResolvedNode>,
    sup_additional: &ResolvedNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    let mut matched = false;

    if let Some(sup_property_schema) = sup_property_schema {
        matched = true;
        if !is_subschema_of_with_context(sub_schema, sup_property_schema, context) {
            return false;
        }
    }

    for (pattern, sup_pattern_schema) in sup_pattern_properties {
        if !property_name_matches_pattern(pattern, property_name) {
            continue;
        }
        matched = true;
        if !is_subschema_of_with_context(sub_schema, sup_pattern_schema, context) {
            return false;
        }
    }

    if matched {
        true
    } else {
        !matches!(sup_additional.kind(), ResolvedNodeKind::BoolSchema(false))
            && is_subschema_of_with_context(sub_schema, sup_additional, context)
    }
}

fn object_property_name_can_be_present(
    property_name: &str,
    properties: &HashMap<String, ResolvedNode>,
    pattern_properties: &HashMap<String, ResolvedNode>,
    property_names: &ResolvedNode,
    additional: &ResolvedNode,
) -> bool {
    if !property_names.accepts_value(&Value::String(property_name.to_owned())) {
        return false;
    }

    let mut matched = false;

    if let Some(schema) = properties.get(property_name) {
        matched = true;
        if matches!(schema.kind(), ResolvedNodeKind::BoolSchema(false)) {
            return false;
        }
    }

    for (pattern, schema) in pattern_properties {
        if !property_name_matches_pattern(pattern, property_name) {
            continue;
        }
        matched = true;
        if matches!(schema.kind(), ResolvedNodeKind::BoolSchema(false)) {
            return false;
        }
    }

    matched || !matches!(additional.kind(), ResolvedNodeKind::BoolSchema(false))
}

fn object_additional_schema_is_subsumed(
    sub_additional: &ResolvedNode,
    sub_pattern_properties: &HashMap<String, ResolvedNode>,
    sup_pattern_properties: &HashMap<String, ResolvedNode>,
    sup_additional: &ResolvedNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    if !is_subschema_of_with_context(sub_additional, sup_additional, context) {
        return false;
    }

    sup_pattern_properties
        .iter()
        .filter(|(pattern, _)| !sub_pattern_properties.contains_key(*pattern))
        .all(|(_, sup_pattern_schema)| {
            is_subschema_of_with_context(sub_additional, sup_pattern_schema, context)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use json_schema_ast::ResolvedSchema;
    use serde_json::json;

    fn resolve(raw: Value) -> ResolvedNode {
        ResolvedSchema::from_json(&raw)
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
}
