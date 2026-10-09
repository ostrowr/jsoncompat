use super::*;
use json_schema_ast::ExactNumber;

pub(super) fn exact_numeric_subset(sub: &SchemaNode, sup: &SchemaNode) -> Option<bool> {
    if !matches!(sub.kind(), SchemaNodeKind::Validation(_))
        && !matches!(sup.kind(), SchemaNodeKind::Validation(_))
    {
        return None;
    }
    Some(ExactNumber::from_node(sub)?.is_subset_of(&ExactNumber::from_node(sup)?))
}
