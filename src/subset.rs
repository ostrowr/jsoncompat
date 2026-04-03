use crate::SchemaNode;
use json_schema_ast::{JSONSchema, SchemaNodeId, SchemaNodeKind, compile};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Default)]
struct SubschemaCheckContext {
    compiled_sup_validators: HashMap<SchemaNodeId, Option<Rc<JSONSchema>>>,
}

impl SubschemaCheckContext {
    fn compile_sup_validator(&mut self, sup: &SchemaNode) -> Option<Rc<JSONSchema>> {
        let node_id = sup.id();
        if let Some(validator) = self.compiled_sup_validators.get(&node_id) {
            return validator.clone();
        }

        let validator = if schema_contains_cycle(sup, &mut Vec::new()) {
            None
        } else {
            compile(&sup.to_json()).ok().map(Rc::new)
        };
        self.compiled_sup_validators
            .insert(node_id, validator.clone());
        validator
    }
}

/// Returns `true` if **every** instance that satisfies `sub` also satisfies
/// `sup`.
pub fn is_subschema_of(sub: &SchemaNode, sup: &SchemaNode) -> bool {
    is_subschema_of_with_context(sub, sup, &mut SubschemaCheckContext::default())
}

fn is_subschema_of_with_context(
    sub: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    if sub == sup {
        return true;
    }

    use SchemaNodeKind::*;

    let sub_kind = sub.kind().clone();
    let sup_kind = sup.kind().clone();

    match (&sub_kind, &sup_kind) {
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

        (_, AnyOf(sups)) | (_, OneOf(sups)) => sups
            .iter()
            .any(|branch| is_subschema_of_with_context(sub, branch, context)),
        (_, AllOf(sups)) => sups
            .iter()
            .all(|schema| is_subschema_of_with_context(sub, schema, context)),

        (Enum(sub_e), Enum(sup_e)) => sub_e.iter().all(|v| sup_e.contains(v)),
        (Enum(sub_e), _) => context
            .compile_sup_validator(sup)
            .map(|compiled| sub_e.iter().all(|value| compiled.is_valid(value)))
            .unwrap_or(false),
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

        (Const(s_val), Const(p_val)) => s_val == p_val,

        (Const(s_val), _) => context
            .compile_sup_validator(sup)
            .map(|compiled| compiled.is_valid(s_val))
            .unwrap_or(false),

        (_, Const(_)) => false,

        _ => false,
    }
}

fn integer_constraints_subsumed_by_number(sub: &SchemaNode, sup: &SchemaNode) -> bool {
    use SchemaNodeKind::*;

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
    if !check_multiple_of_inclusion(smul, pmul) {
        return false;
    }
    check_enum_inclusion(s_en.as_deref(), p_en.as_deref())
}

/// Compare the **constraints** of two nodes of the *same* basic type.
pub fn type_constraints_subsumed(sub: &SchemaNode, sup: &SchemaNode) -> bool {
    type_constraints_subsumed_with_context(sub, sup, &mut SubschemaCheckContext::default())
}

fn type_constraints_subsumed_with_context(
    sub: &SchemaNode,
    sup: &SchemaNode,
    context: &mut SubschemaCheckContext,
) -> bool {
    use SchemaNodeKind::*;

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
            if !check_multiple_of_inclusion(smul, pmul) {
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
                if let Some(p_schema) = pprops.get(key) {
                    if !is_subschema_of_with_context(s_schema, p_schema, context) {
                        return false;
                    }
                } else {
                    // The new schema permits an additional property that the
                    // previous map did not list explicitly.  We must ensure the
                    // "additional" schema of the superset accepts whatever the
                    // subset would have produced (or, if `additionalProperties`
                    // was `false`, reject immediately).
                    let additional_allows =
                        !matches!(s_addl.kind(), SchemaNodeKind::BoolSchema(false));
                    if !additional_allows
                        || !is_subschema_of_with_context(s_schema, &p_addl, context)
                    {
                        return false;
                    }
                }
            }

            if !is_subschema_of_with_context(&s_addl, &p_addl, context) {
                return false;
            }

            if !is_subschema_of_with_context(&s_prop_names, &p_prop_names, context) {
                return false;
            }

            for (trigger, deps) in p_deps.iter() {
                // If the superset requires extra keys whenever `trigger` exists,
                // then the subset may only allow `trigger` when those keys are
                // unconditionally present.
                let trigger_allowed = sprops.contains_key(trigger)
                    || !matches!(s_addl.kind(), SchemaNodeKind::BoolSchema(false));
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
            if !is_subschema_of_with_context(&sitems, &pitems, context) {
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

    let ratio = sub_multiple_of / sup_multiple_of;
    (ratio - ratio.round()).abs() <= f64::EPSILON * ratio.abs().max(1.0) * 4.0
}

fn check_enum_inclusion(
    sub_enum: Option<&[serde_json::Value]>,
    sup_enum: Option<&[serde_json::Value]>,
) -> bool {
    match (sub_enum, sup_enum) {
        (_, None) => true,
        (Some(sub_enum), Some(sup_enum)) => sub_enum.iter().all(|value| sup_enum.contains(value)),
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

fn schema_contains_cycle(schema: &SchemaNode, path: &mut Vec<SchemaNode>) -> bool {
    if path.iter().any(|ancestor| ancestor.ptr_eq(schema)) {
        return true;
    }

    let children = schema_children(schema);
    if children.is_empty() {
        return false;
    }

    path.push(schema.clone());
    let contains_cycle = children
        .iter()
        .any(|child| schema_contains_cycle(child, path));
    path.pop();
    contains_cycle
}

fn schema_children(schema: &SchemaNode) -> Vec<SchemaNode> {
    use SchemaNodeKind::*;

    match schema.kind() {
        AllOf(children) | AnyOf(children) | OneOf(children) => children.clone(),
        Not(child) | AdditionalProperties(child) => vec![child.clone()],
        IfThenElse {
            if_schema,
            then_schema,
            else_schema,
        } => std::iter::once(if_schema.clone())
            .chain(then_schema.iter().cloned())
            .chain(else_schema.iter().cloned())
            .collect(),
        Object {
            properties,
            additional,
            property_names,
            ..
        } => properties
            .values()
            .cloned()
            .chain(std::iter::once(additional.clone()))
            .chain(std::iter::once(property_names.clone()))
            .collect(),
        Array {
            items, contains, ..
        } => std::iter::once(items.clone())
            .chain(contains.iter().cloned())
            .collect(),
        Defs(map) => map.values().cloned().collect(),
        _ => Vec::new(),
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
            "exclusiveMinimum": 1,
            "maximum": 3
        }))
        .unwrap();

        assert!(is_subschema_of(&new, &old));
    }

    #[test]
    fn enum_with_numeric_bound_is_not_subsumed_by_wider_enum() {
        let old = build_and_resolve_schema(&json!({
                "type": "number",
                "enum": [0, 1],
                "minimum": 1
        }))
        .unwrap();
        let new = build_and_resolve_schema(&json!({
                "enum": [0, 1]
        }))
        .unwrap();

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn integer_multiple_of_must_be_at_least_as_constrained_as_number_multiple_of() {
        let old = build_and_resolve_schema(&json!({
                "type": "number",
                "multipleOf": 2
        }))
        .unwrap();
        let new = build_and_resolve_schema(&json!({
                "type": "integer",
                "multipleOf": 3
        }))
        .unwrap();

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn integer_schema_without_enum_is_not_subsumed_by_old_number_enum() {
        let old = build_and_resolve_schema(&json!({
                "type": "number",
                "enum": [1],
                "minimum": 1
        }))
        .unwrap();
        let new = build_and_resolve_schema(&json!({
                "type": "integer",
                "minimum": 1,
                "maximum": 2
        }))
        .unwrap();

        assert!(!is_subschema_of(&new, &old));
    }

    #[test]
    fn enum_is_not_checked_by_serializing_recursive_sup_schema() {
        let sup = build_and_resolve_schema(&json!({
            "$defs": {
                "Node": {
                    "type": "object",
                    "properties": {
                        "next": { "$ref": "#/$defs/Node" }
                    }
                }
            },
            "$ref": "#/$defs/Node"
        }))
        .unwrap();
        let sub = build_and_resolve_schema(&json!({
            "enum": [1]
        }))
        .unwrap();

        assert!(!is_subschema_of(&sub, &sup));
    }
}
