//! Finite-language approximations and enum-based emptiness helpers.
//!
//! Every helper here returns an upper bound or a one-sided proof; unknown stays `None`/`false`.

use super::*;

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

/// Return a finite (not necessarily minimal) superset of the JSON values that
/// can satisfy `schema`, when such a superset is syntactically obvious.
///
/// This is intentionally an *upper* bound, not an exact evaluator. Enum and
/// const keywords cap a language even when another conjunct uses an
/// unsupported regex or recursion, and an `allOf` language is capped by any
/// finite child. Callers use the result for pigeonhole/capacity arguments, so
/// over-approximating is safe while under-approximating would not be. Cycles
/// simply make the helper give up.
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
