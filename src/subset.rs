use crate::SchemaNode;
use json_schema_ast::{compile, SchemaNodeKind};

/// Returns `true` if **every** instance that satisfies `sub` also satisfies
/// `sup`.
pub fn is_subschema_of(sub: &SchemaNode, sup: &SchemaNode) -> bool {
    if sub == sup {
        return true;
    }

    use SchemaNodeKind::*;

    let sub_kind = sub.borrow().clone();
    let sup_kind = sup.borrow().clone();

    match (&sub_kind, &sup_kind) {
        (BoolSchema(false), _) => true,
        (_, BoolSchema(true)) => true,
        (Any, Any) => true,
        (_, Any) => true,
        (Any, _) => false,

        (Enum(sub_e), Enum(sup_e)) => sub_e.iter().all(|v| sup_e.contains(v)),
        (Enum(_), _) => false,
        (_, Enum(_)) => false,

        (AllOf(subs), _) => subs.iter().all(|s| is_subschema_of(s, sup)),
        (_, AllOf(sups)) => sups.iter().all(|s| is_subschema_of(sub, s)),

        (AnyOf(subs), _) => subs.iter().all(|branch| is_subschema_of(branch, sup)),
        (_, AnyOf(sups)) => sups.iter().any(|branch| is_subschema_of(sub, branch)),

        (OneOf(subs), _) => subs.iter().all(|branch| is_subschema_of(branch, sup)),
        (_, OneOf(sups)) => sups.iter().any(|branch| is_subschema_of(sub, branch)),

        (Not(subn), _) => match &*subn.borrow() {
            Any | BoolSchema(true) => true,
            BoolSchema(false) => !matches!(sup_kind, Any | BoolSchema(true)),
            _ => false,
        },
        (_, Not(supn)) => match &*supn.borrow() {
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
        | (Array { .. }, Array { .. }) => type_constraints_subsumed(sub, sup),

        (Const(s_val), Const(p_val)) => s_val == p_val,

        (Const(s_val), _) => {
            let schema_json = sup.to_json();
            compile(&schema_json)
                .map(|compiled| compiled.is_valid(s_val))
                .unwrap_or(false)
        }

        (_, Const(_)) => false,

        _ => false,
    }
}

/// Compare the **constraints** of two nodes of the *same* basic type.
pub fn type_constraints_subsumed(sub: &SchemaNode, sup: &SchemaNode) -> bool {
    use SchemaNodeKind::*;

    let sub_kind = sub.borrow().clone();
    let sup_kind = sup.borrow().clone();

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
            if let Some(pm) = pmin {
                if smin.unwrap_or(0) < pm {
                    return false;
                }
            }
            if let Some(px) = pmax {
                if smax.unwrap_or(u64::MAX) > px {
                    return false;
                }
            }
            if let (Some(se), Some(pe)) = (s_enum, p_enum) {
                if !se.iter().all(|v| pe.contains(v)) {
                    return false;
                }
            }
            true
        }

        (
            Number {
                minimum: smin,
                maximum: smax,
                exclusive_minimum: sexmin,
                exclusive_maximum: sexmax,
                enumeration: s_en,
                ..
            },
            Number {
                minimum: pmin,
                maximum: pmax,
                exclusive_minimum: pexmin,
                exclusive_maximum: pexmax,
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
            if let (Some(se), Some(pe)) = (s_en, p_en) {
                if !se.iter().all(|v| pe.contains(v)) {
                    return false;
                }
            }
            true
        }

        (
            Integer {
                minimum: smin,
                maximum: smax,
                exclusive_minimum: sexmin,
                exclusive_maximum: sexmax,
                enumeration: s_en,
                ..
            },
            Integer {
                minimum: pmin,
                maximum: pmax,
                exclusive_minimum: pexmin,
                exclusive_maximum: pexmax,
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
            if let (Some(se), Some(pe)) = (s_en, p_en) {
                if !se.iter().all(|v| pe.contains(v)) {
                    return false;
                }
            }
            true
        }

        (Boolean { enumeration: s_e }, Boolean { enumeration: p_e })
        | (Null { enumeration: s_e }, Null { enumeration: p_e }) => {
            if let (Some(se), Some(pe)) = (s_e, p_e) {
                if !se.iter().all(|v| pe.contains(v)) {
                    return false;
                }
            }
            true
        }

        (
            Object {
                properties: sprops,
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
                required: preq,
                additional: p_addl,
                property_names: p_prop_names,
                min_properties: pmin,
                max_properties: pmax,
                dependent_required: p_deps,
                enumeration: p_en,
            },
        ) => {
            if let Some(pm) = pmin {
                if smin.unwrap_or(0) < pm {
                    return false;
                }
            }
            if let Some(px) = pmax {
                if smax.unwrap_or(usize::MAX) > px {
                    return false;
                }
            }

            if let (Some(se), Some(pe)) = (s_en, p_en) {
                if !se.iter().all(|v| pe.contains(v)) {
                    return false;
                }
            }

            if !preq.is_subset(&sreq) {
                return false;
            }

            for (key, s_schema) in &sprops {
                if let Some(p_schema) = pprops.get(key) {
                    if !is_subschema_of(s_schema, p_schema) {
                        return false;
                    }
                } else {
                    // The new schema permits an additional property that the
                    // previous map did not list explicitly.  We must ensure the
                    // "additional" schema of the superset accepts whatever the
                    // subset would have produced (or, if `additionalProperties`
                    // was `false`, reject immediately).
                    let additional_allows =
                        !matches!(&*s_addl.borrow(), SchemaNodeKind::BoolSchema(false));
                    if !additional_allows || !is_subschema_of(s_schema, &p_addl) {
                        return false;
                    }
                }
            }

            if !is_subschema_of(&s_addl, &p_addl) {
                return false;
            }

            if !is_subschema_of(&s_prop_names, &p_prop_names) {
                return false;
            }

            for (trigger, deps) in p_deps.iter() {
                // If the superset requires extra keys whenever `trigger` exists,
                // then the subset may only allow `trigger` when those keys are
                // unconditionally present.
                let trigger_allowed = sprops.contains_key(trigger)
                    || !matches!(&*s_addl.borrow(), SchemaNodeKind::BoolSchema(false));
                if trigger_allowed && !deps.iter().all(|d| sreq.contains(d)) {
                    return false;
                }
            }

            true
        }

        (
            Array {
                items: sitems,
                min_items: smin,
                max_items: smax,
                enumeration: s_en,
                ..
            },
            Array {
                items: pitems,
                min_items: pmin,
                max_items: pmax,
                enumeration: p_en,
                ..
            },
        ) => {
            if let Some(pm) = pmin {
                if smin.unwrap_or(0) < pm {
                    return false;
                }
            }
            if let Some(px) = pmax {
                if smax.unwrap_or(u64::MAX) > px {
                    return false;
                }
            }
            if !is_subschema_of(&sitems, &pitems) {
                return false;
            }
            if let (Some(se), Some(pe)) = (s_en, p_en) {
                if !se.iter().all(|v| pe.contains(v)) {
                    return false;
                }
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
            if s_excl {
                subv >= supv
            } else {
                subv > supv
            }
        } else {
            subv >= supv
        }
    } else if p_excl {
        if s_excl {
            subv <= supv
        } else {
            subv < supv
        }
    } else {
        subv <= supv
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
            if s_excl {
                subv >= supv
            } else {
                subv > supv
            }
        } else {
            subv >= supv
        }
    } else if p_excl {
        if s_excl {
            subv <= supv
        } else {
            subv < supv
        }
    } else {
        subv <= supv
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use json_schema_ast::build_and_resolve_schema;
    use serde_json::json;

    #[test]
    fn allof_tighten_subset() {
        let old = build_and_resolve_schema(&json!({
            "allOf": [
                {"type": "integer", "minimum": 0},
                {"maximum": 10}
            ]
        }))
        .unwrap();
        let new = build_and_resolve_schema(&json!({
            "type": "integer",
            "minimum": 0,
            "maximum": 5
        }))
        .unwrap();
        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn exclusive_bounds_subset() {
        let old = build_and_resolve_schema(&json!({
            "minimum": 1,
            "exclusiveMinimum": 1
        }))
        .unwrap();
        let new = build_and_resolve_schema(&json!({
            "minimum": 1,
            "maximum": 3
        }))
        .unwrap();

        assert!(is_subschema_of(&new, &old));
    }
}
