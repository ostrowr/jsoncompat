//! Compile model layouts and validation proofs without creating Python classes.

use super::{Annotation, ClassKind, ClassSpec, DataclassError, invalid_schema, types::Kind};
use crate::{
    model_plan::{GuardPatch, PreparedGuard, PreparedPlan},
    prepared_schema::{Node, NodeId as SchemaNodeId, PreparedSchema, PreparedSchemaContext, Rule},
};
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    ops::Deref,
};

#[derive(Clone, Copy)]
pub(super) struct NodeId(pub usize);
#[derive(Clone, Copy)]
pub(super) enum ScalarKind {
    Any,
    String,
    Integer,
    Number,
    Boolean,
    Null,
}
pub(super) struct FieldPlan {
    pub json_name: String,
    pub py_name: String,
    pub value_node: NodeId,
    pub presence: FieldPresence,
}
#[derive(Clone, Copy)]
pub(super) struct FieldPresence(pub bool);
impl FieldPresence {
    pub fn is_omittable(self) -> bool {
        self.0
    }
}
pub(super) struct ModelFields(pub Vec<FieldPlan>);
impl Deref for ModelFields {
    type Target = [FieldPlan];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl ModelFields {
    pub fn by_json_name(&self, name: &str) -> Option<&FieldPlan> {
        self.0
            .binary_search_by(|field| field.json_name.as_str().cmp(name))
            .ok()
            .map(|i| &self.0[i])
    }
}
pub(super) struct ExtraPropertiesPlan {
    pub value_node: NodeId,
}
pub(super) struct UnionPlan {
    pub branches: Vec<NodeId>,
    pub discriminator: Option<(String, Vec<(Value, usize)>)>,
}
impl Deref for UnionPlan {
    type Target = [NodeId];
    fn deref(&self) -> &Self::Target {
        &self.branches
    }
}
pub(super) enum ConversionNode {
    Scalar {
        kind: ScalarKind,
    },
    Literal {
        values: Vec<Value>,
    },
    List {
        item: NodeId,
    },
    Dict {
        value: NodeId,
    },
    Union(UnionPlan),
    Model {
        model: usize,
        fields: ModelFields,
        extra: Option<ExtraPropertiesPlan>,
    },
    Root {
        model: usize,
        value: NodeId,
    },
}
pub(super) struct ModelConverterPlan {
    pub nodes: Vec<ConversionNode>,
    pub schemas: Vec<PreparedSchema>,
    pub leaf_guards: Vec<Option<Node>>,
}
struct GuardCandidate {
    owner: usize,
    field: Option<usize>,
    original: usize,
    guard: Node,
}

impl ModelConverterPlan {
    pub fn node(&self, id: NodeId) -> &ConversionNode {
        &self.nodes[id.0]
    }
    fn schema(&self, node: &ConversionNode) -> Option<&PreparedSchema> {
        match node {
            ConversionNode::Model { model, .. } | ConversionNode::Root { model, .. } => {
                Some(&self.schemas[*model])
            }
            _ => None,
        }
    }
    fn guard_patches(&self) -> Vec<GuardCandidate> {
        let mut patches = Vec::new();
        for (owner, node) in self.nodes.iter().enumerate() {
            let Some(schema) = self.schema(node) else {
                continue;
            };
            let constraint = schema.node(SchemaNodeId(0));
            let candidates: Vec<(Option<usize>, NodeId, &Node)> = match node {
                ConversionNode::Model { fields, .. } => constraint
                    .rules
                    .iter()
                    .flat_map(|rule| {
                        let Rule::Object { properties, .. } = rule else {
                            return Vec::new();
                        };
                        fields
                            .iter()
                            .enumerate()
                            .filter_map(|(i, field)| {
                                properties
                                    .binary_search_by(|(name, _)| name.cmp(&field.json_name))
                                    .ok()
                                    .map(|j| {
                                        (Some(i), field.value_node, schema.node(properties[j].1))
                                    })
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect(),
                ConversionNode::Root { value, .. } => vec![(None, *value, constraint)],
                _ => unreachable!(),
            };
            for (field, original, guard) in candidates {
                if guard.is_leaf()
                    && !guard.rules.is_empty()
                    && matches!(self.node(original),ConversionNode::Scalar {kind} if !matches!(kind,ScalarKind::Any))
                {
                    patches.push(GuardCandidate {
                        owner,
                        field,
                        original: original.0,
                        guard: guard.clone(),
                    });
                }
            }
        }
        patches
    }
    fn prepare(&mut self) -> PreparedPlan {
        let base_nodes = self.nodes.len();
        // Intern equivalent checks during generation. A thousand constrained
        // fields may need only two scalar nodes, not a thousand copies.
        let mut guard_ids = HashMap::new();
        let mut guard_nodes = Vec::new();
        let guards = self
            .guard_patches()
            .into_iter()
            .map(|candidate| {
                let key = (
                    candidate.original,
                    serde_json::to_vec(&candidate.guard).expect("scalar guard"),
                );
                let id = *guard_ids.entry(key).or_insert_with(|| {
                    let id = guard_nodes.len();
                    guard_nodes.push(PreparedGuard {
                        original: candidate.original,
                        guard: candidate.guard,
                    });
                    id
                });
                GuardPatch {
                    owner: candidate.owner,
                    field: candidate.field,
                    guard: id,
                }
            })
            .collect::<Vec<_>>();
        let json_keys = self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(i, node)| {
                let ConversionNode::Model { fields, .. } = node else {
                    return None;
                };
                Some((
                    i,
                    fields
                        .iter()
                        .map(|f| {
                            format!("{}:", serde_json::to_string(&f.json_name).expect("string"))
                        })
                        .collect(),
                ))
            })
            .collect();
        for guard in &guard_nodes {
            let ConversionNode::Scalar { kind } = self.nodes[guard.original] else {
                unreachable!()
            };
            self.nodes.push(ConversionNode::Scalar { kind });
            self.leaf_guards.push(Some(guard.guard.clone()));
        }
        for patch in &guards {
            let id = NodeId(base_nodes + patch.guard);
            match &mut self.nodes[patch.owner] {
                ConversionNode::Model { fields, .. } => {
                    fields.0[patch.field.expect("field patch")].value_node = id
                }
                ConversionNode::Root { value, .. } => *value = id,
                _ => unreachable!(),
            }
        }
        let conversion_validates = self
            .nodes
            .iter()
            .enumerate()
            .map(|(i, node)| {
                self.schema(node).is_some_and(|schema| {
                    self.implies_prepared_schema(
                        NodeId(i),
                        schema,
                        SchemaNodeId(0),
                        &mut HashSet::new(),
                    )
                })
            })
            .collect();
        // Descriptors contain the original graph; guards are installed by the loader.
        for patch in &guards {
            match &mut self.nodes[patch.owner] {
                ConversionNode::Model { fields, .. } => {
                    fields.0[patch.field.expect("field patch")].value_node =
                        NodeId(guard_nodes[patch.guard].original)
                }
                ConversionNode::Root { value, .. } => {
                    *value = NodeId(guard_nodes[patch.guard].original)
                }
                _ => unreachable!(),
            }
        }
        self.nodes.truncate(base_nodes);
        PreparedPlan {
            version: 4,
            base_nodes,
            guard_nodes,
            guards,
            conversion_validates,
            json_keys,
        }
    }
}

pub(super) struct PreparedModule {
    pub graph: ModelConverterPlan,
    pub roots: Vec<NodeId>,
    pub decisions: PreparedPlan,
}

enum ContextDocument<'a> {
    Canonical(&'a json_schema_ast::SchemaDocument),
    Source(Option<json_schema_ast::SchemaDocument>),
}

impl PreparedModule {
    pub fn compile(classes: &[ClassSpec]) -> Result<Self, DataclassError> {
        Self::compile_with_document(classes, None)
    }

    pub fn compile_for_package(
        classes: &[ClassSpec],
        document: &json_schema_ast::SchemaDocument,
    ) -> Result<Self, DataclassError> {
        Self::compile_with_document(classes, Some(document))
    }

    fn compile_with_document(
        classes: &[ClassSpec],
        package_document: Option<&json_schema_ast::SchemaDocument>,
    ) -> Result<Self, DataclassError> {
        let mut builder = Builder {
            classes,
            class_ids: classes
                .iter()
                .enumerate()
                .map(|(i, c)| (c.name.as_str(), i))
                .collect(),
            nodes: Vec::new(),
            ids: BTreeMap::new(),
        };
        let roots = classes
            .iter()
            .map(|class| builder.add(&Annotation::model(class.name.clone())))
            .collect::<Result<_, _>>()?;
        // Validate and link each shared resource once. The context only accepts
        // references whose meaning survives promoting a subschema to a model
        // root; other schemas retain the standalone compiler's full semantics.
        let package_canonical =
            package_document.and_then(|document| document.canonical_schema_json().ok());
        let mut documents = HashMap::new();
        for class in classes {
            if let Some(context) = &class.schema.context {
                let key = std::sync::Arc::as_ptr(context) as usize;
                if let std::collections::hash_map::Entry::Vacant(entry) = documents.entry(key) {
                    // Check once per shared resource, not once per class. The
                    // context must borrow precisely the graph the models use.
                    if let Some(document) = package_document
                        && package_canonical == Some(&context.root)
                    {
                        entry.insert(ContextDocument::Canonical(document));
                        continue;
                    }
                    // A package-wide wrapper may exceed an analysis limit even
                    // when each model is supported. Let the standalone compiler
                    // decide in that case rather than narrowing accepted input.
                    entry.insert(ContextDocument::Source(
                        json_schema_ast::SchemaDocument::from_json(&context.root).ok(),
                    ));
                }
            }
        }
        let mut contexts = documents
            .iter()
            .map(|(key, document)| {
                let context = match document {
                    ContextDocument::Canonical(document) => {
                        PreparedSchemaContext::new_canonical(document)
                    }
                    ContextDocument::Source(Some(document)) => PreparedSchemaContext::new(document),
                    ContextDocument::Source(None) => Ok(None),
                };
                context.map(|context| (*key, context))
            })
            .collect::<Result<HashMap<_, _>, _>>()
            .map_err(|error| invalid_schema("#".into(), error))?;
        let schemas = classes
            .iter()
            .map(|class| {
                let shared = class.schema.context.as_ref().and_then(|context| {
                    let key = std::sync::Arc::as_ptr(context) as usize;
                    let value = context
                        .root
                        .pointer(class.schema.pointer.strip_prefix('#')?)?;
                    (value == &class.schema.value).then_some(key)
                });
                let result = match shared
                    .and_then(|key| contexts.get_mut(&key))
                    .and_then(Option::as_mut)
                {
                    Some(context) => context.compile_at(&class.schema.pointer),
                    None => PreparedSchema::compile(&class.schema.materialize()),
                };
                result.map_err(|error| invalid_schema(class.name.clone(), error))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let nodes = builder
            .nodes
            .into_iter()
            .map(|node| node.expect("completed converter graph"))
            .collect::<Vec<_>>();
        let mut graph = ModelConverterPlan {
            leaf_guards: vec![None; nodes.len()],
            nodes,
            schemas,
        };
        let decisions = graph.prepare();
        Ok(Self {
            graph,
            roots,
            decisions,
        })
    }
}
struct Builder<'a> {
    classes: &'a [ClassSpec],
    class_ids: BTreeMap<&'a str, usize>,
    nodes: Vec<Option<ConversionNode>>,
    ids: BTreeMap<String, NodeId>,
}
impl Builder<'_> {
    fn add(&mut self, annotation: &Annotation) -> Result<NodeId, DataclassError> {
        if let Some(id) = self.ids.get(annotation.as_ref() as &str) {
            return Ok(*id);
        }
        let id = NodeId(self.nodes.len());
        self.ids.insert(annotation.to_string(), id);
        self.nodes.push(None);
        let node = match &annotation.kind {
            Kind::Any => ConversionNode::Scalar {
                kind: ScalarKind::Any,
            },
            Kind::String => ConversionNode::Scalar {
                kind: ScalarKind::String,
            },
            Kind::Integer => ConversionNode::Scalar {
                kind: ScalarKind::Integer,
            },
            Kind::Number => ConversionNode::Scalar {
                kind: ScalarKind::Number,
            },
            Kind::Boolean => ConversionNode::Scalar {
                kind: ScalarKind::Boolean,
            },
            Kind::Null => ConversionNode::Scalar {
                kind: ScalarKind::Null,
            },
            Kind::Literal(value) => ConversionNode::Literal {
                values: vec![value.clone()],
            },
            Kind::Sequence(item) => ConversionNode::List {
                item: self.add(item)?,
            },
            Kind::Mapping(value) => ConversionNode::Dict {
                value: self.add(value)?,
            },
            Kind::Union(branches) => {
                let ids = branches
                    .iter()
                    .map(|b| self.add(b))
                    .collect::<Result<Vec<_>, _>>()?;
                if branches.iter().all(|b| matches!(b.kind, Kind::Literal(_))) {
                    ConversionNode::Literal {
                        values: branches
                            .iter()
                            .map(|b| {
                                let Kind::Literal(v) = &b.kind else {
                                    unreachable!()
                                };
                                v.clone()
                            })
                            .collect(),
                    }
                } else {
                    ConversionNode::Union(UnionPlan {
                        branches: ids,
                        discriminator: self.discriminator(branches),
                    })
                }
            }
            Kind::Model(name) => {
                let model = self
                    .class_ids
                    .get(name.as_str())
                    .copied()
                    .ok_or_else(|| invalid_schema(name.clone(), "missing generated model"))?;
                let kind = self.classes[model].kind.clone();
                match kind {
                    ClassKind::Root { annotation } => ConversionNode::Root {
                        model,
                        value: self.add(&annotation)?,
                    },
                    ClassKind::Object {
                        fields,
                        extra_annotation,
                    } => {
                        let mut fields = fields
                            .into_iter()
                            .map(|field| {
                                Ok(FieldPlan {
                                    value_node: self.add(&field.annotation)?,
                                    json_name: field.json_name,
                                    py_name: field.py_name,
                                    presence: FieldPresence(!field.required),
                                })
                            })
                            .collect::<Result<Vec<_>, DataclassError>>()?;
                        fields.sort_by(|a, b| a.json_name.cmp(&b.json_name));
                        let extra = extra_annotation
                            .map(|a| {
                                self.add(&a)
                                    .map(|value_node| ExtraPropertiesPlan { value_node })
                            })
                            .transpose()?;
                        ConversionNode::Model {
                            model,
                            fields: ModelFields(fields),
                            extra,
                        }
                    }
                }
            }
        };
        self.nodes[id.0] = Some(node);
        Ok(id)
    }
    fn discriminator(&self, branches: &[Annotation]) -> Option<(String, Vec<(Value, usize)>)> {
        let mut models = Vec::new();
        for (index, branch) in branches.iter().enumerate() {
            match &branch.kind {
                Kind::Null => {}
                Kind::Model(name) => {
                    let ClassKind::Object { fields, .. } =
                        &self.classes[*self.class_ids.get(name.as_str())?].kind
                    else {
                        return None;
                    };
                    models.push((index, fields));
                }
                _ => return None,
            }
        }
        if models.len() < 2 {
            return None;
        }
        for candidate in models[0].1.iter().filter(|f| f.required) {
            let mut entries = Vec::new();
            let mut valid = true;
            for (branch, fields) in &models {
                let Some(field) = fields
                    .iter()
                    .find(|f| f.required && f.json_name == candidate.json_name)
                else {
                    valid = false;
                    break;
                };
                let literals = match &field.annotation.kind {
                    Kind::Literal(value) => vec![value.clone()],
                    Kind::Union(values) => {
                        let Some(values) = values
                            .iter()
                            .map(|v| {
                                if let Kind::Literal(v) = &v.kind {
                                    Some(v.clone())
                                } else {
                                    None
                                }
                            })
                            .collect::<Option<Vec<_>>>()
                        else {
                            valid = false;
                            break;
                        };
                        values
                    }
                    _ => {
                        valid = false;
                        break;
                    }
                };
                for value in literals {
                    if value.is_number() && value.as_i64().is_none() {
                        valid = false;
                        break;
                    }
                    entries.push((value, *branch));
                }
            }
            if valid {
                let unique = entries
                    .iter()
                    .filter(|(v, _)| entries.iter().filter(|(other, _)| other == v).count() == 1)
                    .cloned()
                    .collect::<Vec<_>>();
                if !unique.is_empty() {
                    return Some((candidate.json_name.clone(), unique));
                }
            }
        }
        None
    }
}
