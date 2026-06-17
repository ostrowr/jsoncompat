//! Small result types shared by subset proof phases.
//!
//! Keeping the verdict/explanation plumbing separate from the dispatcher makes
//! individual proof rules easier to read: rules return a `SubschemaAnalysis`,
//! while `ExplanationMode` controls whether they spend work constructing a
//! diagnostic.

use super::SubschemaExplanation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ExplanationMode {
    VerdictOnly,
    Explain,
}

#[derive(Debug)]
pub(super) struct SubschemaAnalysis {
    pub(super) is_subschema: bool,
    pub(super) explanation: Option<SubschemaExplanation>,
}

impl SubschemaAnalysis {
    pub(super) fn compatible() -> Self {
        Self {
            is_subschema: true,
            explanation: None,
        }
    }

    pub(super) fn from_check(
        is_subschema: bool,
        mode: ExplanationMode,
        explanation: impl FnOnce() -> Option<SubschemaExplanation>,
    ) -> Self {
        if is_subschema {
            return Self::compatible();
        }
        Self {
            is_subschema: false,
            explanation: match mode {
                ExplanationMode::VerdictOnly => None,
                ExplanationMode::Explain => explanation(),
            },
        }
    }
}
