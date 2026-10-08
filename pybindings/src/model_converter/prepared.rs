//! Validate and bind decisions already computed by the generator.
use super::{ConversionNode, HashSet, ModelConverterPlan, NodeId};
use jsoncompat_codegen::model_plan::PreparedPlan;

impl ModelConverterPlan {
    pub(super) fn install_prepared_plan(&mut self, bytes: &[u8]) -> super::PyResult<()> {
        let prepared: PreparedPlan = serde_json::from_slice(bytes).map_err(|error| {
            super::PyValueError::new_err(format!("Invalid prepared model plan: {error}"))
        })?;
        if prepared.version != 1
            || prepared.base_nodes != self.nodes.len()
            || prepared.conversion_validates.len() != self.nodes.len() + prepared.guards.len()
        {
            return Err(super::PyValueError::new_err(
                "Incompatible generated model plan; regenerate models",
            ));
        }
        let mut keyed = HashSet::new();
        for (owner, prefixes) in &prepared.json_keys {
            let Some(ConversionNode::Model { fields, .. }) = self.nodes.get_mut(*owner) else {
                return Err(super::PyValueError::new_err(
                    "Invalid prepared JSON key owner",
                ));
            };
            if !keyed.insert(*owner) || prefixes.len() != fields.len() {
                return Err(super::PyValueError::new_err(
                    "Prepared JSON keys do not match model fields",
                ));
            }
            for (field, prefix) in fields.serialized.iter_mut().zip(prefixes) {
                let name = prefix
                    .strip_suffix(':')
                    .and_then(|name| serde_json::from_str::<String>(name).ok());
                if name.as_deref() != Some(field.json_name.as_str()) {
                    return Err(super::PyValueError::new_err(
                        "Invalid prepared JSON field key",
                    ));
                }
                field.json_prefix = prefix.as_bytes().to_vec();
            }
        }
        if self.nodes.iter().enumerate().any(|(index, node)| {
            matches!(node, ConversionNode::Model { .. }) && !keyed.contains(&index)
        }) {
            return Err(super::PyValueError::new_err(
                "Prepared model is missing JSON field keys",
            ));
        }
        for patch in &prepared.guards {
            let Some(ConversionNode::Scalar { kind }) = self.nodes.get(patch.original) else {
                return Err(super::PyValueError::new_err(
                    "Prepared guard must reference a scalar converter",
                ));
            };
            if !patch.guard.is_leaf() {
                return Err(super::PyValueError::new_err(
                    "Prepared scalar guard contains non-scalar rules",
                ));
            }
            let node = ConversionNode::Scalar { kind: *kind };
            let id = NodeId(self.nodes.len());
            match self.nodes.get_mut(patch.owner) {
                Some(ConversionNode::Model { fields, .. }) => {
                    let field = patch
                        .field
                        .and_then(|index| fields.serialized.get_mut(index))
                        .ok_or_else(|| {
                            super::PyValueError::new_err("Invalid prepared field guard")
                        })?;
                    if field.value_node.0 != patch.original {
                        return Err(super::PyValueError::new_err(
                            "Prepared field guard does not match model",
                        ));
                    }
                    field.value_node = id;
                }
                Some(ConversionNode::Root { value, .. })
                    if patch.field.is_none() && value.0 == patch.original =>
                {
                    *value = id
                }
                _ => return Err(super::PyValueError::new_err("Invalid prepared guard owner")),
            }
            self.nodes.push(node);
            self.leaf_guards.push(Some(patch.guard.clone()));
            self.conversion_validates.push(false);
        }
        self.conversion_validates
            .clone_from(&prepared.conversion_validates);
        Ok(())
    }
}
