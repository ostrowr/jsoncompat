//! Failure explanation types for subset checks.

use crate::json_pointer::JsonPointer;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SubschemaExplanation {
    segments: Vec<String>,
    reason: String,
    schema_path: JsonPointer,
    schema_side: ExplanationSchemaSide,
}

impl SubschemaExplanation {
    pub(super) fn new(reason: impl Into<String>) -> Self {
        Self {
            segments: Vec::new(),
            reason: reason.into(),
            schema_path: JsonPointer::root(),
            schema_side: ExplanationSchemaSide::Subset,
        }
    }

    pub(super) fn under(mut self, segment: impl Into<String>) -> Self {
        self.segments.insert(0, segment.into());
        self
    }

    pub(super) fn in_superset(mut self) -> Self {
        self.schema_side = ExplanationSchemaSide::Superset;
        self
    }

    pub(super) fn at_keyword(mut self, keyword: impl Into<String>) -> Self {
        self.schema_path.push(keyword);
        self
    }

    pub(super) fn at_dependent_required(mut self, trigger: &str) -> Self {
        self.schema_path.push("dependentRequired");
        self.schema_path.push(trigger);
        self
    }

    pub(super) fn at_pattern_property(mut self, pattern: &str) -> Self {
        self.schema_path.push("patternProperties");
        self.schema_path.push(pattern);
        self
    }

    pub(super) fn at_property(mut self, property: &str) -> Self {
        self.schema_path.push("properties");
        self.schema_path.push(property);
        self
    }

    pub(super) fn under_property(mut self, property: &str) -> Self {
        self.schema_path.prepend(["properties", property]);
        self.under(format!("property '{property}'"))
    }

    pub(super) fn under_property_names(mut self) -> Self {
        self.schema_path.prepend(["propertyNames"]);
        self.under("property names")
    }

    pub(super) fn under_any_of_branch(mut self, index: usize) -> Self {
        self.schema_path.prepend(["anyOf", &index.to_string()]);
        self.under(format!("anyOf branch {}", index + 1))
    }

    pub(super) fn under_subset_any_of_branch(mut self, index: usize) -> Self {
        if self.schema_side == ExplanationSchemaSide::Subset {
            self.schema_path.prepend(["anyOf", &index.to_string()]);
        }
        self.under(format!("anyOf branch {}", index + 1))
    }

    pub(super) fn under_superset_any_of_branch(mut self, index: usize) -> Self {
        if self.schema_side == ExplanationSchemaSide::Superset {
            self.schema_path.prepend(["anyOf", &index.to_string()]);
        }
        self.under("closest previous anyOf branch")
    }

    pub(super) fn under_superset_all_of_branch(mut self, index: usize) -> Self {
        if self.schema_side == ExplanationSchemaSide::Superset {
            self.schema_path.prepend(["allOf", &index.to_string()]);
        }
        self.under(format!("required allOf branch {}", index + 1))
    }

    pub(super) fn under_one_of_branch(mut self, index: usize) -> Self {
        self.schema_path.prepend(["oneOf", &index.to_string()]);
        self.under(format!("oneOf branch {}", index + 1))
    }

    pub(super) fn under_conditional_branch(mut self, keyword: &'static str) -> Self {
        self.schema_path.prepend([keyword]);
        self.under(format!("conditional {keyword} branch"))
    }

    pub(super) fn under_array_item(
        mut self,
        index: usize,
        subset_uses_prefix: bool,
        superset_uses_prefix: bool,
    ) -> Self {
        match self.schema_side {
            ExplanationSchemaSide::Subset if subset_uses_prefix => {
                self.schema_path
                    .prepend(["prefixItems", &index.to_string()]);
            }
            ExplanationSchemaSide::Superset if superset_uses_prefix => {
                self.schema_path
                    .prepend(["prefixItems", &index.to_string()]);
            }
            ExplanationSchemaSide::Subset | ExplanationSchemaSide::Superset => {
                self.schema_path.prepend(["items"]);
            }
        }
        self.under(format!("array item {}", index + 1))
    }

    pub(super) fn under_array_items(mut self) -> Self {
        self.schema_path.prepend(["items"]);
        self.under("array items")
    }

    pub(crate) fn render(&self, subset_label: &str, superset_label: &str) -> String {
        let reason = if self.segments.is_empty() {
            self.reason.clone()
        } else {
            format!("{}: {}", self.segments.join(" -> "), self.reason)
        };
        let schema_label = match self.schema_side {
            ExplanationSchemaSide::Subset => subset_label,
            ExplanationSchemaSide::Superset => superset_label,
        };
        format!(
            "{schema_label} schema {}: {reason}",
            self.schema_path.render()
        )
    }

    pub(super) fn depth(&self) -> usize {
        self.segments.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExplanationSchemaSide {
    Subset,
    Superset,
}
