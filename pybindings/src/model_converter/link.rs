//! Link already prepared dependency graphs without reflecting or recompiling models.
use super::*;

#[derive(Clone)]
pub(super) struct NodeOrigin(Arc<()>, usize);
impl NodeOrigin {
    fn key(&self) -> (usize, usize) {
        (Arc::as_ptr(&self.0) as usize, self.1)
    }
}

impl ModelConverterPlan {
    pub(super) fn link_dependencies(
        &mut self,
        py: Python<'_>,
        dependencies: Vec<(usize, ModelConverterPy)>,
    ) {
        let identity = Arc::new(());
        self.origins = (0..self.nodes.len())
            .map(|i| NodeOrigin(Arc::clone(&identity), i))
            .collect();
        let mut linked = HashMap::new();
        // Reserve every imported root first: shared children then resolve to
        // the same node even when reached through several dependency modules.
        for (local, external) in &dependencies {
            let origin = external.origins[external.root.0.0].clone();
            linked.insert(origin.key(), NodeId(*local));
            self.origins[*local] = origin;
        }
        for (local, external) in dependencies {
            let mut pending = vec![(NodeId(local), external.root.0)];
            while let Some((target, source)) = pending.pop() {
                let mut reference = |id: NodeId| {
                    let origin = &external.origins[id.0];
                    *linked.entry(origin.key()).or_insert_with(|| {
                        let target = NodeId(self.nodes.len());
                        self.nodes.push(ConversionNode::Scalar {
                            kind: ScalarKind::Any,
                        });
                        self.leaf_guards.push(None);
                        self.conversion_validates.push(false);
                        self.origins.push(origin.clone());
                        pending.push((target, id));
                        target
                    })
                };
                let node = external.node(source).linked_copy(py, &mut reference);
                self.nodes[target.0] = node;
                self.leaf_guards[target.0] = external.leaf_guards[source.0].clone();
                self.conversion_validates[target.0] = external.conversion_validates[source.0];
            }
        }
    }
}

impl ModelAttribute {
    fn clone_ref(&self, py: Python<'_>) -> Self {
        Self {
            name: self.name.clone_ref(py),
            storage: ValidatedSlot {
                owner: self.storage.owner.clone_ref(py),
                #[cfg(all(Py_3_11, not(any(PyPy, GraalPy, Py_GIL_DISABLED))))]
                offset: self.storage.offset,
                #[cfg(not(all(Py_3_11, not(any(PyPy, GraalPy, Py_GIL_DISABLED)))))]
                descriptor: self.storage.descriptor.clone_ref(py),
            },
        }
    }
}

impl ConversionNode {
    fn linked_copy(&self, py: Python<'_>, reference: &mut impl FnMut(NodeId) -> NodeId) -> Self {
        match self {
            Self::Scalar { kind } => Self::Scalar { kind: *kind },
            Self::Literal { values } => Self::Literal {
                values: values.iter().map(|v| v.clone_ref(py)).collect(),
            },
            Self::List { item } => Self::List {
                item: reference(*item),
            },
            Self::Dict { value } => Self::Dict {
                value: reference(*value),
            },
            Self::Union(union) => Self::Union(UnionPlan {
                branches: union.branches.iter().map(|id| reference(*id)).collect(),
                discriminator: union.discriminator.as_ref().map(|d| DiscriminatorPlan {
                    json_name: d.json_name.clone(),
                    branch_ordinals_by_value: d.branch_ordinals_by_value.clone(),
                }),
            }),
            Self::Model {
                model_type,
                branch_schema,
                fields,
                extra,
            } => Self::Model {
                model_type: model_type.clone_ref(py),
                branch_schema: BranchSchema {
                    program: Arc::clone(&branch_schema.program),
                },
                fields: ModelFields {
                    serialized: fields
                        .serialized
                        .iter()
                        .map(|f| FieldPlan {
                            json_name: f.json_name.clone(),
                            json_prefix: f.json_prefix.clone(),
                            attribute: f.attribute.clone_ref(py),
                            value_node: reference(f.value_node),
                            presence: f.presence,
                        })
                        .collect(),
                    by_json_name: fields.by_json_name.clone(),
                    by_py_name: fields.by_py_name.clone(),
                    omittable: fields.omittable.clone(),
                },
                extra: extra.as_ref().map(|e| ExtraPropertiesPlan {
                    value_node: reference(e.value_node),
                    attribute: e.attribute.clone_ref(py),
                }),
            },
            Self::Root {
                model_type,
                branch_schema,
                value,
                root_attribute,
            } => Self::Root {
                model_type: model_type.clone_ref(py),
                branch_schema: BranchSchema {
                    program: Arc::clone(&branch_schema.program),
                },
                value: reference(*value),
                root_attribute: root_attribute.clone_ref(py),
            },
        }
    }
}
