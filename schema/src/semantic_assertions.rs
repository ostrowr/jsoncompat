//! Backend assertions whose meaning depends on exact JSON numeric semantics.
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Kind {
    Null,
    Boolean,
    Integer,
    Number,
    String,
    Array,
    Object,
}
impl Kind {
    fn accepts(&self, value: &Value) -> bool {
        match self {
            Self::Null => value.is_null(),
            Self::Boolean => value.is_boolean(),
            Self::Integer => value.as_number().is_some_and(|n| {
                n.is_i64() || n.is_u64() || crate::Decimal::from_number(n).is_integer()
            }),
            Self::Number => value.is_number(),
            Self::String => value.is_string(),
            Self::Array => value.is_array(),
            Self::Object => value.is_object(),
        }
    }
}
enum Assertion {
    Type(Vec<Kind>),
    Const(Value),
    Enum(Vec<Value>),
    Unique(bool),
}
impl jsonschema::Keyword for Assertion {
    fn is_valid(&self, value: &Value) -> bool {
        match self {
            Self::Type(kinds) => kinds.iter().any(|kind| kind.accepts(value)),
            Self::Const(expected) => crate::json_values_equal(expected, value),
            Self::Enum(choices) => choices
                .iter()
                .any(|expected| crate::json_values_equal(expected, value)),
            Self::Unique(enabled) => {
                !enabled
                    || value.as_array().is_none_or(|values| {
                        values.iter().enumerate().all(|(i, value)| {
                            values[..i]
                                .iter()
                                .all(|previous| !crate::json_values_equal(previous, value))
                        })
                    })
            }
        }
    }
    fn validate<'i>(&self, value: &'i Value) -> Result<(), jsonschema::ValidationError<'i>> {
        if self.is_valid(value) {
            Ok(())
        } else {
            let keyword = match self {
                Self::Type(_) => "type",
                Self::Const(_) => "const",
                Self::Enum(_) => "enum",
                Self::Unique(_) => "uniqueItems",
            };
            Err(jsonschema::ValidationError::custom(format!(
                "{keyword} constraint failed"
            )))
        }
    }
}
pub(crate) fn configure(
    mut options: jsonschema::ValidationOptions,
) -> jsonschema::ValidationOptions {
    for keyword in ["type", "const", "enum", "uniqueItems"] {
        options = options.with_keyword(keyword, move |_, value, _| {
            let assertion = match keyword {
                "type" => {
                    let values = value
                        .as_array()
                        .cloned()
                        .unwrap_or_else(|| vec![value.clone()]);
                    let kinds = values
                        .into_iter()
                        .map(serde_json::from_value)
                        .collect::<Result<Vec<Kind>, _>>()
                        .map_err(|error| jsonschema::ValidationError::custom(error.to_string()))?;
                    Assertion::Type(kinds)
                }
                "const" => Assertion::Const(value.clone()),
                "enum" => Assertion::Enum(
                    value
                        .as_array()
                        .ok_or_else(|| {
                            jsonschema::ValidationError::custom("enum must be an array")
                        })?
                        .clone(),
                ),
                "uniqueItems" => Assertion::Unique(value.as_bool().ok_or_else(|| {
                    jsonschema::ValidationError::custom("uniqueItems must be boolean")
                })?),
                _ => unreachable!(),
            };
            Ok(Box::new(assertion))
        });
    }
    options
}
