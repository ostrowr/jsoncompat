//! Back‑compatibility checking library.
//!
//! This crate depends on `json_schema_draft2020`, which provides a strict
//! in‑memory representation (`SchemaNode`) of a Draft 2020‑12 JSON Schema.  The
//! only responsibility of this crate is to offer algorithms that compare two
//! schemas and decide whether a change is backward‑compatible from the point
//! of view of a serializer or deserializer.


// Re‑export the fundamental building blocks from the core schema crate so that
// downstream crates can just depend on *this* crate for both parsing and
// compatibility checking if they wish.
pub use json_schema_draft2020::{
    build_and_resolve_schema, build_schema_ast, resolve_refs, SchemaNode,
};

mod subset;

pub use subset::{is_subschema_of, type_constraints_subsumed};

/// The role under which a compatibility check is performed.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Role {
    /// Evolving the *producer* (serializer).  A change is safe if every value
    /// produced by the _new_ schema is still accepted by the _old_ one.
    Serializer,
    /// Evolving the *consumer* (deserializer).  A change is safe if every value
    /// accepted by the _old_ schema is still valid under the _new_ one.
    Deserializer,
    /// We need to maintain full equivalence in both directions.
    Both,
}

/// Top‑level convenience wrapper:
///
/// * `Role::Serializer`   ⇒   `new ⊆ old`
/// * `Role::Deserializer` ⇒   `old ⊆ new`
/// * `Role::Both`         ⇒   bidirectional inclusion
pub fn check_compat(old: &SchemaNode, new: &SchemaNode, role: Role) -> bool {
    match role {
        Role::Serializer => is_subschema_of(new, old),
        Role::Deserializer => is_subschema_of(old, new),
        Role::Both => is_subschema_of(new, old) && is_subschema_of(old, new),
    }
}
