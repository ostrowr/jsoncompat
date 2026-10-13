//! Stable component/resource modules, with dependency-directed prepared binding.
use super::{
    DataclassModule, DataclassPackage,
    package::{SchemaFragment, SchemaSource},
    plan::ConversionNode,
    render,
};
use crate::model_plan::PreparedPlan;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
};

impl DataclassModule {
    pub(super) fn resource_files(&self, owners: &BTreeMap<String, String>) -> DataclassPackage {
        let mut exports = BTreeMap::new();
        let mut members: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (i, name) in self.models.iter().enumerate() {
            let owner = if owners.contains_key(name) {
                Some(name)
            } else {
                let mut pointer = self.parts.pointers[i].as_str();
                loop {
                    if let Some(name) = self.parts.named_declarations.get(pointer) {
                        break Some(name);
                    }
                    if let Some((parent, _)) = pointer.rsplit_once('/') {
                        pointer = parent;
                    } else {
                        // Shared canonical helpers outside named declarations
                        // have stable storage independent of component order.
                        break None;
                    }
                }
            };
            let module = owner
                .map(|owner| owners[owner].as_str())
                .unwrap_or("models._shared");
            exports.insert(name.clone(), module.to_owned());
            members.entry(module.to_owned()).or_default().push(i);
        }
        let edges: BTreeMap<_, BTreeSet<_>> = members
            .iter()
            .map(|(module, indices)| {
                let refs = indices
                    .iter()
                    .flat_map(|i| &self.parts.references[*i])
                    .map(|name| exports[name].clone())
                    .filter(|target| target != module)
                    .collect();
                (module.clone(), refs)
            })
            .collect();
        let mut files = BTreeMap::new();
        let mut data = Data::default();
        let programs: Vec<_> = self
            .models
            .iter()
            .map(|m| format!("_jsoncompat_program_{m}"))
            .collect();
        let prepared = self.parts.prepared.as_ref().unwrap();
        let plan_index = PlanIndex::new(&prepared.decisions);
        for group in components(&edges) {
            let cyclic = group.len() > 1;
            let module = if cyclic {
                format!("_cycles.{}", group[0].replace('.', "_"))
            } else {
                group[0].clone()
            };
            let indices: Vec<_> = group
                .iter()
                .flat_map(|m| members[m].iter().copied())
                .collect();
            let local: BTreeSet<_> = indices.iter().copied().collect();
            let mut source = self
                .imports
                .lines()
                .filter(|line| {
                    indices.iter().any(|i| {
                        self.parts.imports[*i]
                            .lines()
                            .any(|candidate| candidate == *line)
                    })
                })
                .map(|line| format!("{line}\n"))
                .collect::<String>()
                .replace("from .", "from ..");
            if let Some(resource) = module.strip_prefix("operations.") {
                source = source.replace(
                    "from .. import api as _jsoncompat_api_types",
                    &format!("from ..operations import {resource} as _jsoncompat_api_types"),
                );
            }
            source.push_str("from builtins import bytes as _jsoncompat_bytes\n");
            if !source.contains("from jsoncompat.codegen import dataclasses as dc\n") {
                source.push_str("from jsoncompat.codegen import dataclasses as dc\n");
            }
            for &i in &indices {
                source.push_str(&self.parts.declarations[i]);
            }
            let references: BTreeSet<_> = indices
                .iter()
                .flat_map(|i| &self.parts.references[*i])
                .collect();
            for name in references {
                if !group.contains(&exports[name]) {
                    writeln!(source, "from ..{} import {name} as {name}", exports[name]).unwrap();
                }
            }
            let mut selected = BTreeSet::new();
            let mut pending: Vec<_> = indices.iter().map(|i| prepared.roots[*i].0).collect();
            while let Some(id) = pending.pop() {
                if !selected.insert(id) {
                    continue;
                }
                let node = &prepared.graph.nodes[id];
                if model(node).is_some_and(|m| !local.contains(&m)) {
                    continue;
                }
                pending.extend(children(node));
            }
            let remap: BTreeMap<_, _> = selected
                .iter()
                .enumerate()
                .map(|(i, old)| (*old, i))
                .collect();
            let plan = plan_index.project(&remap, |id| {
                model(&prepared.graph.nodes[id]).is_some_and(|m| !local.contains(&m))
            });
            source.push_str("\n# Prepared constructors and programs; no schema compilation on import.\n_jsoncompat_missing = dc.MissingType.MISSING\n");
            for &i in &indices {
                source.push_str(&data.schema(
                    &self.parts.sources[i],
                    &format!("_jsoncompat_schema_{}", self.models[i]),
                ));
                writeln!(
                    source,
                    "{} = {}",
                    programs[i],
                    render::bytes(&self.parts.programs[i])
                )
                .unwrap();
                source.push_str(&self.parts.initializers[i].replace(
                    &format!("_jsoncompat_init_{i}("),
                    &format!("_jsoncompat_init_{}(", self.models[i]),
                ));
                writeln!(
                    source,
                    "dc.install_model({0}, _jsoncompat_init_{0}, _jsoncompat_schema_{0}, globals())",
                    self.models[i]
                )
                .unwrap();
            }
            writeln!(
                source,
                "\ndc.bind_module(1, [{}], [",
                indices
                    .iter()
                    .map(|i| format!("({}, {})", self.models[*i], remap[&prepared.roots[*i].0]))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
            .unwrap();
            for id in selected {
                let node = &prepared.graph.nodes[id];
                let descriptor = match model(node) {
                    Some(m) if !local.contains(&m) => format!("('external', {})", self.models[m]),
                    _ => render::descriptor(node, &self.models, &programs, |id| remap[&id.0]),
                };
                writeln!(source, "    {descriptor},").unwrap();
            }
            writeln!(
                source,
                "], {})",
                render::bytes(&serde_json::to_vec(&plan).unwrap())
            )
            .unwrap();
            writeln!(
                source,
                "\n__all__: tuple[str, ...] = ({},)",
                indices
                    .iter()
                    .map(|i| format!("{:?}", self.models[*i]))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
            .unwrap();
            files.insert(format!("{}.py", module.replace('.', "/")), source);
            if cyclic {
                for owner in group {
                    let mut facade =
                        String::from("# Recursive declarations share one prepared binding.\n");
                    for i in &members[&owner] {
                        let name = &self.models[*i];
                        writeln!(facade, "from ..{module} import {name} as {name}").unwrap();
                    }
                    writeln!(
                        facade,
                        "__all__: tuple[str, ...] = ({},)",
                        members[&owner]
                            .iter()
                            .map(|i| format!("{:?}", self.models[*i]))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                    .unwrap();
                    files.insert(format!("{}.py", owner.replace('.', "/")), facade);
                }
            }
        }
        files.extend(data.sources);
        DataclassPackage {
            models: self.models.clone(),
            files,
            exports,
        }
    }
}

fn model(node: &ConversionNode) -> Option<usize> {
    match node {
        ConversionNode::Model { model, .. } | ConversionNode::Root { model, .. } => Some(*model),
        _ => None,
    }
}
fn children(node: &ConversionNode) -> Vec<usize> {
    match node {
        ConversionNode::Model { fields, extra, .. } => fields
            .iter()
            .map(|f| f.value_node.0)
            .chain(extra.iter().map(|e| e.value_node.0))
            .collect(),
        ConversionNode::Root { value, .. } | ConversionNode::Dict { value } => vec![value.0],
        ConversionNode::List { item } => vec![item.0],
        ConversionNode::Union(u) => u.branches.iter().map(|b| b.0).collect(),
        _ => Vec::new(),
    }
}
struct PlanIndex<'a> {
    plan: &'a PreparedPlan,
    guards: Vec<Vec<usize>>,
    keys: Vec<Vec<usize>>,
}
impl<'a> PlanIndex<'a> {
    fn new(plan: &'a PreparedPlan) -> Self {
        let mut guards = vec![Vec::new(); plan.base_nodes];
        let mut keys = vec![Vec::new(); plan.base_nodes];
        for (index, patch) in plan.guards.iter().enumerate() {
            guards[patch.owner].push(index);
        }
        for (index, (owner, _)) in plan.json_keys.iter().enumerate() {
            keys[*owner].push(index);
        }
        Self { plan, guards, keys }
    }

    fn project(
        &self,
        remap: &BTreeMap<usize, usize>,
        external: impl Fn(usize) -> bool,
    ) -> PreparedPlan {
        let plan = self.plan;
        // Visit local owners, rather than scanning the complete graph for every
        // output module. Retain the original order of patches and JSON keys.
        let selected = |index: &[Vec<usize>]| {
            let mut positions: Vec<_> = remap
                .keys()
                .filter(|id| !external(**id))
                .flat_map(|id| index[*id].iter().copied())
                .collect();
            positions.sort_unstable();
            positions
        };
        let mut guard_map = BTreeMap::new();
        let mut guard_nodes = Vec::new();
        let guards = selected(&self.guards)
            .into_iter()
            .map(|index| {
                let mut p = plan.guards[index].clone();
                p.owner = remap[&p.owner];
                p.guard = *guard_map.entry(p.guard).or_insert_with(|| {
                    let mut guard = plan.guard_nodes[p.guard].clone();
                    guard.original = remap[&guard.original];
                    guard_nodes.push(guard);
                    guard_nodes.len() - 1
                });
                p
            })
            .collect();
        let mut conversion_validates: Vec<_> = remap
            .keys()
            .map(|id| !external(*id) && plan.conversion_validates[*id])
            .collect();
        let mut guard_flags = vec![false; guard_nodes.len()];
        for (old, new) in guard_map {
            guard_flags[new] = plan.conversion_validates[plan.base_nodes + old];
        }
        conversion_validates.extend(guard_flags);
        PreparedPlan {
            version: plan.version,
            base_nodes: remap.len(),
            guard_nodes,
            guards,
            conversion_validates,
            json_keys: selected(&self.keys)
                .into_iter()
                .map(|index| {
                    let (id, keys) = &plan.json_keys[index];
                    (remap[id], keys.clone())
                })
                .collect(),
        }
    }
}

// Sixteen fixed hash buckets keep identical fragments shared without one file
// per fragment. This storage boundary never controls public module ownership.
#[derive(Default)]
struct Data {
    sources: BTreeMap<String, String>,
    names: BTreeMap<[u8; 32], String>,
    shared: BTreeMap<usize, String>,
}
impl Data {
    fn store(&mut self, bytes: &[u8]) -> String {
        let digest: [u8; 32] = Sha256::digest(bytes).into();
        self.names
            .entry(digest)
            .or_insert_with(|| {
                let name = format!("S_{:x}", Sha256::digest(bytes));
                writeln!(
                    self.sources
                        .entry(format!("_data/s_{:x}.py", digest[0] >> 4))
                        .or_default(),
                    "{name}: bytes = {}",
                    render::bytes(bytes)
                )
                .unwrap();
                name
            })
            .clone()
    }
    fn schema(&mut self, source: &SchemaSource, target: &str) -> String {
        let mut imports = String::new();
        let names = match source {
            SchemaSource::Whole(bytes) => vec![self.store(bytes)],
            SchemaSource::Fragments(fragments) => fragments
                .iter()
                .map(|fragment| match fragment {
                    SchemaFragment::Local(bytes) => self.store(bytes),
                    SchemaFragment::Shared(bytes) => {
                        let identity = std::sync::Arc::as_ptr(bytes) as usize;
                        if let Some(name) = self.shared.get(&identity) {
                            name.clone()
                        } else {
                            let name = self.store(bytes);
                            self.shared.insert(identity, name.clone());
                            name
                        }
                    }
                })
                .collect(),
        };
        let parts: Vec<_> = names
            .into_iter()
            .map(|name| {
                writeln!(
                    imports,
                    "from .._data.s_{} import {name} as _jsoncompat_{name}",
                    &name[2..3]
                )
                .unwrap();
                format!("(_jsoncompat_{name},)")
            })
            .collect();
        let expression = match source {
            SchemaSource::Whole(_) => parts[0].clone(),
            SchemaSource::Fragments(_) => format!("dc.SchemaSource(({},))", parts.join(", ")),
        };
        writeln!(imports, "{target} = {expression}").unwrap();
        imports
    }
}

// Iterative Kosaraju avoids a call-stack limit for long schema dependency chains.
fn components(edges: &BTreeMap<String, BTreeSet<String>>) -> Vec<Vec<String>> {
    let mut visited = BTreeSet::new();
    let mut order = Vec::new();
    for root in edges.keys() {
        let mut stack = vec![(root, false)];
        while let Some((node, exiting)) = stack.pop() {
            if exiting {
                order.push(node.clone());
            } else if visited.insert(node) {
                stack.push((node, true));
                stack.extend(edges[node].iter().rev().map(|next| (next, false)));
            }
        }
    }
    let mut reverse: BTreeMap<_, Vec<_>> = edges.keys().map(|n| (n.clone(), Vec::new())).collect();
    for (from, deps) in edges {
        for to in deps {
            reverse.get_mut(to).unwrap().push(from.clone());
        }
    }
    let mut visited = BTreeSet::new();
    let mut groups = Vec::new();
    for root in order.into_iter().rev() {
        if visited.contains(&root) {
            continue;
        }
        let mut group = Vec::new();
        let mut pending = vec![root];
        while let Some(node) = pending.pop() {
            if visited.insert(node.clone()) {
                pending.extend(reverse[&node].iter().cloned());
                group.push(node);
            }
        }
        group.sort();
        groups.push(group);
    }
    groups
}
