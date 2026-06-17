//! Axis-cover and finite partition helpers for boolean applicators.
//!
//! These routines recognize common generated encodings of exhaustive ranges,
//! counts, property-presence splits, and small oneOf differences.

use super::*;

pub(super) fn u64_intervals_cover_nonnegative(intervals: &[(u64, Option<u64>)]) -> bool {
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

pub(super) fn usize_intervals_cover_nonnegative(intervals: &[(usize, Option<usize>)]) -> bool {
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
pub(super) fn any_of_integer_partition_cover_is_universal(branches: &[SchemaNode]) -> bool {
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
pub(super) fn any_of_numeric_range_cover_is_universal(branches: &[SchemaNode]) -> bool {
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
pub(super) fn any_of_string_length_cover_is_universal(branches: &[SchemaNode]) -> bool {
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
pub(super) fn any_of_array_count_cover_is_universal(branches: &[SchemaNode]) -> bool {
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
pub(super) fn any_of_object_count_cover_is_universal(branches: &[SchemaNode]) -> bool {
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
pub(super) fn oneof_object_empty_partition_subset_of(
    branches: &[SchemaNode],
    sup: &SchemaNode,
) -> bool {
    if branches.len() != 2 {
        return false;
    }

    fn object_with_min(schema: &SchemaNode, min: usize) -> bool {
        match schema.kind() {
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
            } if possible_json_type_mask(if_schema) & JSON_TYPE_OBJECT == 0 => {
                else_schema.as_ref().is_none_or(accepts_empty_object)
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
pub(super) fn oneof_string_empty_partition_subset_of(
    branches: &[SchemaNode],
    sup: &SchemaNode,
) -> bool {
    if branches.len() != 2 {
        return false;
    }

    fn string_with_min(schema: &SchemaNode, min: u64) -> bool {
        match schema.kind() {
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
            } if possible_json_type_mask(if_schema) & JSON_TYPE_STRING == 0 => {
                else_schema.as_ref().is_none_or(accepts_empty_string)
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
pub(super) fn oneof_array_empty_partition_subset_of(
    branches: &[SchemaNode],
    sup: &SchemaNode,
) -> bool {
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
            } if possible_json_type_mask(if_schema) & JSON_TYPE_ARRAY == 0 => {
                else_schema.as_ref().is_none_or(accepts_empty_array)
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
pub(super) fn oneof_object_absence_partition_subset_of(
    branches: &[SchemaNode],
    sup: &SchemaNode,
) -> bool {
    if branches.len() != 2 {
        return false;
    }

    fn accepts_all_objects(schema: &SchemaNode) -> bool {
        match schema.kind() {
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
            } if possible_json_type_mask(if_schema) & JSON_TYPE_OBJECT == 0 => {
                // If the guard cannot match objects, every object takes the else
                // branch (or an implicit true branch).
                else_schema
                    .as_ref()
                    .is_none_or(|branch| accepts_all_without(branch, name))
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
pub(super) fn any_of_property_presence_cover_is_universal(branches: &[SchemaNode]) -> bool {
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
pub(super) fn any_of_single_property_name_partition_is_universal(branches: &[SchemaNode]) -> bool {
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
pub(super) fn any_of_required_property_value_partition_is_universal(
    branches: &[SchemaNode],
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
pub(super) fn any_of_prefix_item_partition_is_universal(branches: &[SchemaNode]) -> bool {
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
pub(super) fn any_of_items_contains_partition_is_universal(branches: &[SchemaNode]) -> bool {
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
