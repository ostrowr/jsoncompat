use crate::SchemaNode;
use json_schema_draft2020::compile;

/// Returns `true` if **every** instance that satisfies `sub` also satisfies
/// `sup`.
pub fn is_subschema_of(sub: &SchemaNode, sup: &SchemaNode) -> bool {
    // Quick win: structural equality ⇒ set equality.
    if sub == sup {
        return true;
    }

    match (sub, sup) {
        // `false` accepts no instance – it is a subset of every schema.
        (SchemaNode::BoolSchema(false), _) => true,

        // `true` accepts every instance – it is a *superset* of every schema.
        (_, SchemaNode::BoolSchema(true)) => true,

        // `Any` behaves like `true` with regard to validation (it accepts every
        // instance).  Therefore *every* schema is a subset of `Any`.
        (SchemaNode::Any, SchemaNode::Any) => true,
        (_, SchemaNode::Any) => true,
        (SchemaNode::Any, _) => false,

        // Enumeration rules -------------------------------------------------
        (SchemaNode::Enum(sub_e), SchemaNode::Enum(sup_e)) => {
            sub_e.iter().all(|v| sup_e.contains(v))
        }
        // Enum vs non‑enum ⇒ never a subset in either direction under our
        // simplified semantics.
        (SchemaNode::Enum(_), _) => false,
        (_, SchemaNode::Enum(_)) => false,

        // Boolean logic keywords -------------------------------------------
        (SchemaNode::AllOf(subs), sup_schema) => {
            subs.iter().all(|s| is_subschema_of(s, sup_schema))
        }
        (sub_schema, SchemaNode::AllOf(sups)) => {
            sups.iter().all(|s| is_subschema_of(sub_schema, s))
        }

        (SchemaNode::AnyOf(subs), sup_schema) => subs
            .iter()
            .all(|branch| is_subschema_of(branch, sup_schema)),
        (sub_schema, SchemaNode::AnyOf(sups)) => sups
            .iter()
            .any(|branch| is_subschema_of(sub_schema, branch)),

        (SchemaNode::OneOf(subs), sup_schema) => subs
            .iter()
            .all(|branch| is_subschema_of(branch, sup_schema)),
        (sub_schema, SchemaNode::OneOf(sups)) => sups
            .iter()
            .any(|branch| is_subschema_of(sub_schema, branch)),

        (SchemaNode::Not(subn), sup_schema) => match &**subn {
            SchemaNode::Any | SchemaNode::BoolSchema(true) => true, // empty set ⇒ subset of everything
            SchemaNode::BoolSchema(false) => match sup_schema {
                SchemaNode::Any | SchemaNode::BoolSchema(true) => false, // everything not allowed ⇒ never subset unless sup is false too
                _ => true,
            },
            _ => false, // not implemented properly
        },
        (sub_schema, SchemaNode::Not(supn)) => match &**supn {
            SchemaNode::Any | SchemaNode::BoolSchema(true) => {
                matches!(sub_schema, SchemaNode::BoolSchema(false))
            }
            SchemaNode::BoolSchema(false) => {
                matches!(sub_schema, SchemaNode::BoolSchema(true) | SchemaNode::Any)
            }
            _ => false,
        },

        // Structural / numeric constraints ---------------------------------
        (SchemaNode::String { .. }, SchemaNode::String { .. })
        | (SchemaNode::Number { .. }, SchemaNode::Number { .. })
        | (SchemaNode::Integer { .. }, SchemaNode::Integer { .. })
        | (SchemaNode::Boolean { .. }, SchemaNode::Boolean { .. })
        | (SchemaNode::Null { .. }, SchemaNode::Null { .. })
        | (SchemaNode::Object { .. }, SchemaNode::Object { .. })
        | (SchemaNode::Array { .. }, SchemaNode::Array { .. }) => {
            type_constraints_subsumed(sub, sup)
        }

        // --------------------------- Const -------------------------------
        (SchemaNode::Const(s_val), SchemaNode::Const(p_val)) => s_val == p_val,

        (SchemaNode::Const(s_val), p_node) => {
            // The subset allows exactly `s_val`.  Therefore we only need to
            // confirm that `p_node` accepts that single value.
            let schema_json = p_node.to_json();
            compile(&schema_json)
                .map(|compiled| compiled.is_valid(s_val))
                .unwrap_or(false)
        }

        (_, SchemaNode::Const(_)) => false,

        _ => false,
    }
}

// -------------------------------------------------------------------------
// Extra logic for `const` support
// -------------------------------------------------------------------------

// -------------------------------------------------------------------------
// Constraint‑level checks
// -------------------------------------------------------------------------

/// Compare the **constraints** of two nodes of the *same* basic type.
pub fn type_constraints_subsumed(sub: &SchemaNode, sup: &SchemaNode) -> bool {
    match (sub, sup) {
        // --------------------------- Strings ------------------------------
        (
            SchemaNode::String {
                min_length: smin,
                max_length: smax,
                enumeration: s_enum,
                ..
            },
            SchemaNode::String {
                min_length: pmin,
                max_length: pmax,
                enumeration: p_enum,
                ..
            },
        ) => {
            if let Some(pm) = pmin {
                if smin.unwrap_or(0) < *pm {
                    return false;
                }
            }
            if let Some(px) = pmax {
                if smax.unwrap_or(u64::MAX) > *px {
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

        // --------------------------- Numbers ------------------------------
        (
            SchemaNode::Number {
                minimum: smin,
                maximum: smax,
                exclusive_minimum: sexmin,
                exclusive_maximum: sexmax,
                enumeration: s_en,
            },
            SchemaNode::Number {
                minimum: pmin,
                maximum: pmax,
                exclusive_minimum: pexmin,
                exclusive_maximum: pexmax,
                enumeration: p_en,
            },
        ) => {
            if !check_numeric_inclusion(*smin, *sexmin, *pmin, *pexmin, true) {
                return false;
            }
            if !check_numeric_inclusion(*smax, *sexmax, *pmax, *pexmax, false) {
                return false;
            }
            if let (Some(se), Some(pe)) = (s_en, p_en) {
                if !se.iter().all(|v| pe.contains(v)) {
                    return false;
                }
            }
            true
        }

        // -------------------------- Integers ------------------------------
        (
            SchemaNode::Integer {
                minimum: smin,
                maximum: smax,
                exclusive_minimum: sexmin,
                exclusive_maximum: sexmax,
                enumeration: s_en,
            },
            SchemaNode::Integer {
                minimum: pmin,
                maximum: pmax,
                exclusive_minimum: pexmin,
                exclusive_maximum: pexmax,
                enumeration: p_en,
            },
        ) => {
            if !check_int_inclusion(*smin, *sexmin, *pmin, *pexmin, true) {
                return false;
            }
            if !check_int_inclusion(*smax, *sexmax, *pmax, *pexmax, false) {
                return false;
            }
            if let (Some(se), Some(pe)) = (s_en, p_en) {
                if !se.iter().all(|v| pe.contains(v)) {
                    return false;
                }
            }
            true
        }

        // --------------------------- Boolean / Null -----------------------
        (SchemaNode::Boolean { enumeration: s_e }, SchemaNode::Boolean { enumeration: p_e })
        | (SchemaNode::Null { enumeration: s_e }, SchemaNode::Null { enumeration: p_e }) => {
            if let (Some(se), Some(pe)) = (s_e, p_e) {
                se.iter().all(|v| pe.contains(v))
            } else {
                true
            }
        }

        // --------------------------- Objects ------------------------------
        (
            SchemaNode::Object {
                properties: sprops,
                required: sreq,
                additional: s_addl,
                enumeration: s_en,
                dependent_required: _s_deps,
            },
            SchemaNode::Object {
                properties: pprops,
                required: preq,
                 additional: p_addl,
                 enumeration: p_en,
                 dependent_required: p_deps,
            },
        ) => {
            if let (Some(se), Some(pe)) = (s_en, p_en) {
                if !se.iter().all(|v| pe.contains(v)) {
                    return false;
                }
            }

            for (k, ssub) in sprops {
                if let Some(psub) = pprops.get(k) {
                    if !is_subschema_of(ssub, psub) {
                        return false;
                    }
                } else if !is_subschema_of(ssub, p_addl) {
                    return false;
                }
            }

            for r in preq {
                if !sreq.contains(r) {
                    return false;
                }
            }

            if !is_subschema_of(s_addl, p_addl) {
                return false;
            }

            // dependentRequired inclusion: every dependency in the SUP schema must
            // already be guaranteed by SUB (i.e., either the triggering key is
            // impossible in SUB or the required dependents are unconditionally
            // required there).
            for (trigger, deps) in p_deps {
                // does SUB allow `trigger` property at all?
                let trigger_allowed = sprops.contains_key(trigger)
                    || !matches!(**s_addl, SchemaNode::BoolSchema(false));
                if trigger_allowed {
                    // then SUB must list *all* dependent keys as required.
                    if !deps.iter().all(|d| sreq.contains(d)) {
                        return false;
                    }
                }
            }

            true
        }

        // --------------------------- Arrays -------------------------------
        (
            SchemaNode::Array {
                items: sitems,
                min_items: smin,
                max_items: smax,
                enumeration: s_en,
            },
            SchemaNode::Array {
                items: pitems,
                min_items: pmin,
                max_items: pmax,
                enumeration: p_en,
            },
        ) => {
            if let Some(pm) = pmin {
                if smin.unwrap_or(0) < *pm {
                    return false;
                }
            }
            if let Some(px) = pmax {
                if smax.unwrap_or(u64::MAX) > *px {
                    return false;
                }
            }

            if !is_subschema_of(sitems, pitems) {
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

// -------------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------------

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
