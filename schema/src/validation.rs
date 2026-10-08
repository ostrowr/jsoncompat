//! Executable constraints retained when structural expansion would lose meaning.
use crate::{AstError, ExactNumber, JSONSchema, compile};
use serde_json::Value;
use std::rc::Rc;

#[derive(Clone)]
pub struct ValidationConstraint {
    source: Value,
    validator: Rc<JSONSchema>,
    number: Option<ExactNumber>,
    generation: std::cell::OnceCell<Option<crate::SchemaNode>>,
}

impl ValidationConstraint {
    pub(crate) fn new(source: Value) -> Result<Self, AstError> {
        let validator =
            Rc::new(compile(&source).map_err(|source| AstError::RawValidator { source })?);
        let number = source.as_object().and_then(ExactNumber::from_schema);
        Ok(Self {
            source,
            validator,
            number,
            generation: std::cell::OnceCell::new(),
        })
    }

    /// A relaxed graph for proposing candidates; callers must validate them.
    pub fn generation_schema(&self) -> Option<&crate::SchemaNode> {
        self.generation
            .get_or_init(|| {
                fn relax(schema: &Value) -> Value {
                    let mut result = schema.clone();
                    for (suffix, child) in crate::references::children(schema) {
                        if let Some(target) = result.pointer_mut(&format!("/{suffix}")) {
                            *target = relax(child);
                        }
                    }
                    if let Some(object) = result.as_object_mut() {
                        object.remove("unevaluatedProperties");
                        object.remove("unevaluatedItems");
                        object.remove(crate::options::ASSERT_FORMAT);
                    }
                    result
                }
                let document = crate::SchemaDocument::from_json(&relax(&self.source)).ok()?;
                let root = document.root().ok()?;
                (!matches!(root.kind(), crate::SchemaNodeKind::Validation(_))).then(|| root.clone())
            })
            .as_ref()
    }

    pub fn source(&self) -> &Value {
        &self.source
    }
    pub fn number(&self) -> Option<&ExactNumber> {
        self.number.as_ref()
    }
    pub fn accepts(&self, value: &Value) -> bool {
        self.number.as_ref().map_or_else(
            || self.validator.is_valid(value),
            |number| number.accepts(value),
        )
    }
}

impl std::fmt::Debug for ValidationConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ValidationConstraint")
            .field(&self.source)
            .finish()
    }
}

impl PartialEq for ValidationConstraint {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
    }
}
