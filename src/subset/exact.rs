use super::*;
use json_schema_ast::ExactNumber;

pub(super) fn exact_numeric_subset(sub: &SchemaNode, sup: &SchemaNode) -> Option<bool> {
    if !matches!(sub.kind(), SchemaNodeKind::Validation(_))
        && !matches!(sup.kind(), SchemaNodeKind::Validation(_))
    {
        return None;
    }
    Some(number(sub)?.is_subset_of(&number(sup)?))
}

fn number(schema: &SchemaNode) -> Option<ExactNumber> {
    use serde_json::json;
    let mut object = serde_json::Map::new();
    match schema.kind() {
        SchemaNodeKind::Validation(constraint) => return constraint.number().cloned(),
        SchemaNodeKind::Integer {
            bounds,
            multiple_of,
            enumeration,
        } => {
            object.insert("type".into(), json!("integer"));
            if let Some(value) = bounds.lower() {
                object.insert("minimum".into(), json!(value));
            }
            if let Some(value) = bounds.upper() {
                object.insert("maximum".into(), json!(value));
            }
            if let Some(value) = multiple_of {
                object.insert("multipleOf".into(), json!(value.as_f64()));
            }
            if let Some(values) = enumeration {
                object.insert("enum".into(), json!(values));
            }
        }
        SchemaNodeKind::Number {
            bounds,
            multiple_of,
            enumeration,
        } => {
            object.insert("type".into(), json!("number"));
            for (bound, inclusive, exclusive) in [
                (bounds.lower(), "minimum", "exclusiveMinimum"),
                (bounds.upper(), "maximum", "exclusiveMaximum"),
            ] {
                match bound {
                    NumberBound::Unbounded => {}
                    NumberBound::Inclusive(value) => {
                        object.insert(inclusive.into(), json!(value));
                    }
                    NumberBound::Exclusive(value) => {
                        object.insert(exclusive.into(), json!(value));
                    }
                }
            }
            if let Some(value) = multiple_of {
                object.insert("multipleOf".into(), json!(value.as_f64()));
            }
            if let Some(values) = enumeration {
                object.insert("enum".into(), json!(values));
            }
        }
        _ => return None,
    }
    ExactNumber::from_schema(&object)
}
