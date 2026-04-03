//! Versioned schema stamping for strict writers and historical union readers.
//!
//! A stamped schema stores payloads in a metadata envelope with a concrete
//! writer version and a `data` payload. Writers emit only the latest version,
//! while readers accept a tagged union of historical writer envelopes.

use crate::{Role, build_and_resolve_schema, check_compat};
use json_schema_ast::canonicalize_json;
use jsoncompat_codegen::{JSONCOMPAT_METADATA_KEY, JsoncompatMetadata};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::Path;

pub const STAMP_MANIFEST_VERSION: u32 = 1;
pub const ENVELOPE_VERSION_KEY: &str = "version";
pub const ENVELOPE_DATA_KEY: &str = "data";

const JSON_SCHEMA_DRAFT_2020_12: &str = "https://json-schema.org/draft/2020-12/schema";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StampManifest {
    pub manifest_version: u32,
    pub schemas: BTreeMap<String, SchemaHistory>,
}

impl StampManifest {
    pub fn empty() -> Self {
        Self {
            manifest_version: STAMP_MANIFEST_VERSION,
            schemas: BTreeMap::new(),
        }
    }

    pub fn validate(&self) -> Result<(), StampError> {
        if self.manifest_version != STAMP_MANIFEST_VERSION {
            return Err(StampError::UnsupportedManifestVersion {
                expected: STAMP_MANIFEST_VERSION,
                actual: self.manifest_version,
            });
        }

        for (key, history) in &self.schemas {
            if key.is_empty() {
                return Err(StampError::EmptyStableId);
            }
            if history.stable_id.is_empty() {
                return Err(StampError::EmptyStableId);
            }
            if key != &history.stable_id {
                return Err(StampError::StableIdKeyMismatch {
                    key: key.clone(),
                    stable_id: history.stable_id.clone(),
                });
            }
            if history.versions.is_empty() {
                return Err(StampError::EmptyHistory {
                    stable_id: history.stable_id.clone(),
                });
            }

            let mut previous_version = None;
            for version in &history.versions {
                if let Some(previous) = previous_version
                    && version.version <= previous
                {
                    return Err(StampError::NonIncreasingVersion {
                        stable_id: history.stable_id.clone(),
                        previous,
                        next: version.version,
                    });
                }
                previous_version = Some(version.version);

                let actual_hash = canonical_schema_hash(&version.schema)?;
                if version.schema_sha256 != actual_hash {
                    return Err(StampError::HashMismatch {
                        stable_id: history.stable_id.clone(),
                        version: version.version,
                        expected: version.schema_sha256.clone(),
                        actual: actual_hash,
                    });
                }

                build_and_resolve_schema(&version.schema).map_err(|source| {
                    StampError::InvalidHistoricalSchema {
                        stable_id: history.stable_id.clone(),
                        version: version.version,
                        source,
                    }
                })?;
            }
        }

        Ok(())
    }
}

impl Default for StampManifest {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaHistory {
    pub stable_id: String,
    pub versions: Vec<SchemaVersionEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaVersionEntry {
    pub version: u32,
    pub schema_sha256: String,
    pub schema: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StampStatus {
    Unchanged,
    CompatibleUpdate,
    BreakingChange,
    New,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StampBundle {
    pub manifest_version: u32,
    pub stable_id: String,
    pub status: StampStatus,
    pub version: u32,
    pub versions: Vec<SchemaVersionEntry>,
    pub writer: Value,
    pub reader: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StampResult {
    pub status: StampStatus,
    pub bundle: StampBundle,
    pub manifest: StampManifest,
}

#[derive(Debug, thiserror::Error)]
pub enum StampError {
    #[error("stable_id must be non-empty")]
    EmptyStableId,
    #[error("unsupported manifest_version {actual}; expected {expected}")]
    UnsupportedManifestVersion { expected: u32, actual: u32 },
    #[error("manifest key '{key}' does not match stable_id '{stable_id}'")]
    StableIdKeyMismatch { key: String, stable_id: String },
    #[error("schema history '{stable_id}' must contain at least one version")]
    EmptyHistory { stable_id: String },
    #[error("schema history '{stable_id}' has non-increasing versions: {previous} then {next}")]
    NonIncreasingVersion {
        stable_id: String,
        previous: u32,
        next: u32,
    },
    #[error(
        "schema history '{stable_id}' version {version} hash mismatch: expected {expected}, got {actual}"
    )]
    HashMismatch {
        stable_id: String,
        version: u32,
        expected: String,
        actual: String,
    },
    #[error("invalid historical schema '{stable_id}' version {version}: {source}")]
    InvalidHistoricalSchema {
        stable_id: String,
        version: u32,
        source: anyhow::Error,
    },
    #[error("invalid current schema '{stable_id}': {source}")]
    InvalidCurrentSchema {
        stable_id: String,
        source: anyhow::Error,
    },
    #[error("unsupported non-local $ref '{ref_value}'")]
    UnsupportedRef { ref_value: String },
    #[error("conflicting {metadata_key} metadata at '{pointer}'")]
    ConflictingMetadata {
        pointer: String,
        metadata_key: &'static str,
    },
    #[error("failed to serialize canonical schema: {0}")]
    Canonicalize(#[from] serde_json::Error),
    #[error("failed to write manifest '{path}': {source}")]
    WriteManifest {
        path: String,
        source: std::io::Error,
    },
}

pub fn canonical_schema_hash(schema: &Value) -> Result<String, StampError> {
    let canonical = canonicalize_json(schema);
    let bytes = serde_json::to_vec(&canonical)?;
    let digest = Sha256::digest(bytes);

    let mut out = String::from("sha256:");
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    Ok(out)
}

pub fn stamp_schema(
    manifest: &StampManifest,
    stable_id: &str,
    schema: Value,
) -> Result<StampResult, StampError> {
    if stable_id.is_empty() {
        return Err(StampError::EmptyStableId);
    }

    manifest.validate()?;
    build_and_resolve_schema(&schema).map_err(|source| StampError::InvalidCurrentSchema {
        stable_id: stable_id.to_owned(),
        source,
    })?;

    let schema_sha256 = canonical_schema_hash(&schema)?;
    let mut next_manifest = manifest.clone();
    let status = if let Some(history) = next_manifest.schemas.get_mut(stable_id) {
        let latest = history
            .versions
            .last_mut()
            .expect("validated non-empty history");
        if latest.schema_sha256 == schema_sha256 {
            StampStatus::Unchanged
        } else {
            let old_ast = build_and_resolve_schema(&latest.schema).map_err(|source| {
                StampError::InvalidHistoricalSchema {
                    stable_id: stable_id.to_owned(),
                    version: latest.version,
                    source,
                }
            })?;
            let new_ast = build_and_resolve_schema(&schema).map_err(|source| {
                StampError::InvalidCurrentSchema {
                    stable_id: stable_id.to_owned(),
                    source,
                }
            })?;

            if check_compat(&old_ast, &new_ast, Role::Both) {
                latest.schema_sha256 = schema_sha256;
                latest.schema = schema;
                StampStatus::CompatibleUpdate
            } else {
                let next_version = latest.version + 1;
                history.versions.push(SchemaVersionEntry {
                    version: next_version,
                    schema_sha256,
                    schema,
                });
                StampStatus::BreakingChange
            }
        }
    } else {
        next_manifest.schemas.insert(
            stable_id.to_owned(),
            SchemaHistory {
                stable_id: stable_id.to_owned(),
                versions: vec![SchemaVersionEntry {
                    version: 1,
                    schema_sha256,
                    schema,
                }],
            },
        );
        StampStatus::New
    };

    let history = next_manifest
        .schemas
        .get(stable_id)
        .expect("history inserted above");
    let latest = history.versions.last().expect("history is non-empty");
    let writer = build_writer_schema(stable_id, latest)?;
    let reader = build_reader_schema(stable_id, &history.versions)?;

    Ok(StampResult {
        status,
        bundle: StampBundle {
            manifest_version: STAMP_MANIFEST_VERSION,
            stable_id: stable_id.to_owned(),
            status,
            version: latest.version,
            versions: history.versions.clone(),
            writer,
            reader,
        },
        manifest: next_manifest,
    })
}

pub fn write_stamp_manifest_atomic(
    path: impl AsRef<Path>,
    manifest: &StampManifest,
) -> Result<(), StampError> {
    let path = path.as_ref();
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let temp_path = parent.join(format!(
        ".{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("stamp-manifest")
    ));

    let bytes = serde_json::to_vec_pretty(manifest)?;
    let write_result = (|| -> Result<(), std::io::Error> {
        fs::create_dir_all(parent)?;
        let mut file = fs::File::create(&temp_path)?;
        file.write_all(&bytes)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        match fs::rename(&temp_path, path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                fs::remove_file(path)?;
                fs::rename(&temp_path, path)
            }
            Err(err) => Err(err),
        }
    })();

    if let Err(source) = write_result {
        let _ = fs::remove_file(&temp_path);
        return Err(StampError::WriteManifest {
            path: path.display().to_string(),
            source,
        });
    }

    Ok(())
}

fn build_writer_schema(stable_id: &str, version: &SchemaVersionEntry) -> Result<Value, StampError> {
    let namespace = version_namespace(version.version);
    let payload = annotate_versioned_payload_schema(
        stable_id,
        version.version,
        &namespace,
        rewrite_local_refs(&version.schema, &namespace)?,
    )?;
    let payload_ref = format!("#/$defs/{namespace}");
    let mut writer = json!({
        "$schema": JSON_SCHEMA_DRAFT_2020_12,
        "title": format!("{stable_id} writer v{}", version.version),
        "type": "object",
        "properties": {
            ENVELOPE_VERSION_KEY: { "const": version.version },
            ENVELOPE_DATA_KEY: { "$ref": format!("#/$defs/{namespace}") }
        },
        "required": [ENVELOPE_VERSION_KEY, ENVELOPE_DATA_KEY],
        "additionalProperties": false,
        "$defs": {
            namespace.clone(): payload
        }
    });

    insert_codegen_metadata(
        writer
            .as_object_mut()
            .expect("writer schema is built as an object"),
        "#",
        JsoncompatMetadata::Writer {
            stable_id: stable_id.to_owned(),
            name: writer_model_name(stable_id),
            version: version.version,
            payload_ref,
        },
    )?;

    Ok(writer)
}

fn build_reader_schema(
    stable_id: &str,
    versions: &[SchemaVersionEntry],
) -> Result<Value, StampError> {
    let mut defs = Map::new();
    let mut branches = Vec::new();

    for version in versions {
        let namespace = version_namespace(version.version);
        defs.insert(
            namespace.clone(),
            annotate_versioned_payload_schema(
                stable_id,
                version.version,
                &namespace,
                rewrite_local_refs(&version.schema, &namespace)?,
            )?,
        );
    }

    for (index, version) in versions.iter().rev().enumerate() {
        let namespace = version_namespace(version.version);
        let mut branch = json!({
            "type": "object",
            "properties": {
                ENVELOPE_VERSION_KEY: { "const": version.version },
                ENVELOPE_DATA_KEY: { "$ref": format!("#/$defs/{namespace}") }
            },
            "required": [ENVELOPE_VERSION_KEY, ENVELOPE_DATA_KEY],
            "additionalProperties": false
        });
        insert_codegen_metadata(
            branch
                .as_object_mut()
                .expect("reader branch schema is built as an object"),
            &format!("#/oneOf/{index}"),
            JsoncompatMetadata::ReaderVariant {
                stable_id: stable_id.to_owned(),
                name: reader_variant_model_name(stable_id, version.version),
                version: version.version,
                payload_ref: format!("#/$defs/{namespace}"),
            },
        )?;
        branches.push(branch);
    }

    let mut reader = json!({
        "$schema": JSON_SCHEMA_DRAFT_2020_12,
        "title": format!("{stable_id} reader"),
        "oneOf": branches,
        "$defs": Value::Object(defs)
    });

    insert_codegen_metadata(
        reader
            .as_object_mut()
            .expect("reader schema is built as an object"),
        "#",
        JsoncompatMetadata::Reader {
            stable_id: stable_id.to_owned(),
            name: reader_model_name(stable_id),
        },
    )?;

    Ok(reader)
}

fn annotate_versioned_payload_schema(
    stable_id: &str,
    version: u32,
    namespace: &str,
    payload: Value,
) -> Result<Value, StampError> {
    let root_name = payload_model_name(stable_id, version);
    let mut used_names = BTreeSet::new();
    used_names.insert(root_name.clone());

    annotate_payload_declarations(
        payload,
        stable_id,
        version,
        &root_name,
        &format!("#/$defs/{namespace}"),
        Some(&root_name),
        &mut used_names,
    )
}

fn annotate_payload_declarations(
    schema: Value,
    stable_id: &str,
    version: u32,
    scope_name: &str,
    pointer: &str,
    declaration_name: Option<&str>,
    used_names: &mut BTreeSet<String>,
) -> Result<Value, StampError> {
    match schema {
        Value::Bool(value) => {
            if let Some(name) = declaration_name {
                let mut annotated = Map::new();
                if !value {
                    annotated.insert("not".to_owned(), Value::Object(Map::new()));
                }
                insert_codegen_metadata(
                    &mut annotated,
                    pointer,
                    JsoncompatMetadata::Declaration {
                        stable_id: stable_id.to_owned(),
                        name: name.to_owned(),
                        version,
                        schema_ref: pointer.to_owned(),
                    },
                )?;
                Ok(Value::Object(annotated))
            } else {
                Ok(Value::Bool(value))
            }
        }
        Value::Object(obj) => {
            let mut annotated = Map::new();
            for (key, value) in obj {
                if key == "$defs" {
                    let defs = value.as_object().expect("$defs is validated as an object");
                    let mut annotated_defs = Map::new();
                    for (def_key, nested_schema) in defs {
                        let nested_name =
                            allocate_payload_declaration_name(scope_name, def_key, used_names);
                        let nested_pointer =
                            format!("{pointer}/$defs/{}", escape_pointer_token(def_key));
                        let nested_schema = annotate_payload_declarations(
                            nested_schema.clone(),
                            stable_id,
                            version,
                            &nested_name,
                            &nested_pointer,
                            Some(&nested_name),
                            used_names,
                        )?;
                        annotated_defs.insert(def_key.clone(), nested_schema);
                    }
                    annotated.insert(key, Value::Object(annotated_defs));
                } else {
                    let child_pointer = format!("{pointer}/{}", escape_pointer_token(&key));
                    annotated.insert(
                        key,
                        annotate_payload_declarations(
                            value,
                            stable_id,
                            version,
                            scope_name,
                            &child_pointer,
                            None,
                            used_names,
                        )?,
                    );
                }
            }

            if let Some(name) = declaration_name {
                insert_codegen_metadata(
                    &mut annotated,
                    pointer,
                    JsoncompatMetadata::Declaration {
                        stable_id: stable_id.to_owned(),
                        name: name.to_owned(),
                        version,
                        schema_ref: pointer.to_owned(),
                    },
                )?;
            }

            Ok(Value::Object(annotated))
        }
        Value::Array(items) => items
            .into_iter()
            .enumerate()
            .map(|(index, item)| {
                annotate_payload_declarations(
                    item,
                    stable_id,
                    version,
                    scope_name,
                    &format!("{pointer}/{index}"),
                    None,
                    used_names,
                )
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        value => Ok(value),
    }
}

fn insert_codegen_metadata(
    schema: &mut Map<String, Value>,
    pointer: &str,
    metadata: JsoncompatMetadata,
) -> Result<(), StampError> {
    let metadata = serde_json::to_value(metadata).expect("jsoncompat metadata serializes");
    if let Some(existing) = schema.get(JSONCOMPAT_METADATA_KEY) {
        if existing != &metadata {
            return Err(StampError::ConflictingMetadata {
                pointer: pointer.to_owned(),
                metadata_key: JSONCOMPAT_METADATA_KEY,
            });
        }
        return Ok(());
    }
    schema.insert(JSONCOMPAT_METADATA_KEY.to_owned(), metadata);
    Ok(())
}

fn allocate_payload_declaration_name(
    parent_name: &str,
    def_key: &str,
    used_names: &mut BTreeSet<String>,
) -> String {
    let base_name = format!("{parent_name}{}", pascal_case(def_key));
    if used_names.insert(base_name.clone()) {
        return base_name;
    }

    let mut suffix = 2;
    loop {
        let candidate = format!("{base_name}{suffix}");
        if used_names.insert(candidate.clone()) {
            return candidate;
        }
        suffix += 1;
    }
}

fn version_namespace(version: u32) -> String {
    format!("v{version}")
}

fn payload_model_name(stable_id: &str, version: u32) -> String {
    format!("{}V{version}", pascal_case(stable_id))
}

fn writer_model_name(stable_id: &str) -> String {
    format!("{}Writer", pascal_case(stable_id))
}

fn reader_model_name(stable_id: &str) -> String {
    format!("{}Reader", pascal_case(stable_id))
}

fn reader_variant_model_name(stable_id: &str, version: u32) -> String {
    format!("{}V{version}Reader", pascal_case(stable_id))
}

fn escape_pointer_token(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

fn pascal_case(input: &str) -> String {
    let mut out = String::new();
    for part in input
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
    {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_ascii_uppercase());
            for ch in chars {
                out.push(ch);
            }
        }
    }

    if out.is_empty() {
        out.push_str("Schema");
    }

    if out
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_digit())
    {
        out.insert_str(0, "Schema");
    }

    out
}

fn rewrite_local_refs(schema: &Value, namespace: &str) -> Result<Value, StampError> {
    match schema {
        Value::Object(obj) => {
            let mut rewritten = Map::new();
            for (key, value) in obj {
                if key == "$ref" {
                    let Some(ref_value) = value.as_str() else {
                        rewritten.insert(key.clone(), value.clone());
                        continue;
                    };
                    rewritten.insert(
                        key.clone(),
                        Value::String(rewrite_ref_value(ref_value, namespace)?),
                    );
                } else {
                    rewritten.insert(key.clone(), rewrite_local_refs(value, namespace)?);
                }
            }
            Ok(Value::Object(rewritten))
        }
        Value::Array(items) => items
            .iter()
            .map(|item| rewrite_local_refs(item, namespace))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        _ => Ok(schema.clone()),
    }
}

fn rewrite_ref_value(ref_value: &str, namespace: &str) -> Result<String, StampError> {
    if ref_value == "#" {
        return Ok(format!("#/$defs/{namespace}"));
    }

    if let Some(pointer) = ref_value.strip_prefix("#/") {
        return Ok(format!("#/$defs/{namespace}/{pointer}"));
    }

    Err(StampError::UnsupportedRef {
        ref_value: ref_value.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn canonical_hash_ignores_object_key_order() {
        let left = json!({
            "type": "object",
            "properties": {
                "b": { "type": "string" },
                "a": { "type": "integer" }
            },
            "required": ["a"]
        });
        let right = json!({
            "required": ["a"],
            "properties": {
                "a": { "type": "integer" },
                "b": { "type": "string" }
            },
            "type": "object"
        });

        assert_eq!(
            canonical_schema_hash(&left).unwrap(),
            canonical_schema_hash(&right).unwrap()
        );
    }

    #[test]
    fn stamp_new_schema_initializes_manifest_and_bundle() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            },
            "required": ["name"]
        });

        let result = stamp_schema(&StampManifest::empty(), "user-profile", schema).unwrap();

        assert_eq!(result.status, StampStatus::New);
        assert_eq!(result.bundle.version, 1);
        assert_eq!(result.bundle.versions.len(), 1);
        assert_eq!(
            result.bundle.writer["properties"]["version"],
            json!({ "const": 1 })
        );
        assert_eq!(result.bundle.reader["oneOf"].as_array().unwrap().len(), 1);
        assert!(result.manifest.schemas.contains_key("user-profile"));
    }

    #[test]
    fn stamp_compatible_update_replaces_latest_version() {
        let initial = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            },
            "required": ["name"]
        });
        let compatible = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "nickname": { "type": "string" }
            },
            "required": ["name"]
        });

        let first = stamp_schema(&StampManifest::empty(), "user-profile", initial).unwrap();
        let second = stamp_schema(&first.manifest, "user-profile", compatible.clone()).unwrap();

        assert_eq!(second.status, StampStatus::CompatibleUpdate);
        assert_eq!(second.bundle.version, 1);
        assert_eq!(second.bundle.versions.len(), 1);
        assert_eq!(second.bundle.versions[0].schema, compatible);
    }

    #[test]
    fn stamp_breaking_change_appends_new_version() {
        let initial = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            },
            "required": ["name"]
        });
        let breaking = json!({
            "type": "object",
            "properties": {
                "name": { "type": "integer" }
            },
            "required": ["name"]
        });

        let first = stamp_schema(&StampManifest::empty(), "user-profile", initial).unwrap();
        let second = stamp_schema(&first.manifest, "user-profile", breaking).unwrap();

        assert_eq!(second.status, StampStatus::BreakingChange);
        assert_eq!(second.bundle.version, 2);
        assert_eq!(second.bundle.versions.len(), 2);
        assert_eq!(
            second.bundle.reader["oneOf"][0]["properties"]["version"],
            json!({ "const": 2 })
        );
        assert_eq!(
            second.bundle.reader["oneOf"][1]["properties"]["version"],
            json!({ "const": 1 })
        );
    }

    #[test]
    fn stamp_rewrites_recursive_refs_into_version_namespace() {
        let schema = json!({
            "$defs": {
                "Node": {
                    "type": "object",
                    "properties": {
                        "value": { "type": "integer" },
                        "next": { "$ref": "#/$defs/Node" }
                    },
                    "required": ["value"]
                }
            },
            "$ref": "#/$defs/Node"
        });

        let result = stamp_schema(&StampManifest::empty(), "node", schema).unwrap();

        assert_eq!(
            result.bundle.writer["$defs"]["v1"]["$ref"],
            json!("#/$defs/v1/$defs/Node")
        );
        assert_eq!(
            result.bundle.writer["$defs"]["v1"]["$defs"]["Node"]["properties"]["next"]["$ref"],
            json!("#/$defs/v1/$defs/Node")
        );
    }

    #[test]
    fn stamp_supports_scalar_root_payloads() {
        let schema = json!({
            "type": "string",
            "minLength": 1
        });

        let result = stamp_schema(&StampManifest::empty(), "name", schema.clone()).unwrap();

        assert_eq!(
            result.bundle.writer["$defs"]["v1"],
            json!({
                "type": "string",
                "minLength": 1,
                "x-jsoncompat": {
                    "kind": "declaration",
                    "stable_id": "name",
                    "name": "NameV1",
                    "version": 1,
                    "schema_ref": "#/$defs/v1"
                }
            })
        );
        assert_eq!(
            result.bundle.writer["properties"]["data"],
            json!({ "$ref": "#/$defs/v1" })
        );
    }

    #[test]
    fn stamp_rejects_remote_refs() {
        let schema = json!({
            "$ref": "https://example.com/schema.json"
        });

        let err = stamp_schema(&StampManifest::empty(), "remote", schema).unwrap_err();

        assert!(matches!(err, StampError::UnsupportedRef { .. }));
    }

    #[test]
    fn write_stamp_manifest_atomic_overwrites_existing_file() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "jsoncompat-stamp-manifest-{}-{unique}.json",
            std::process::id()
        ));

        let manifest = StampManifest::empty();
        write_stamp_manifest_atomic(&path, &manifest).unwrap();
        write_stamp_manifest_atomic(&path, &manifest).unwrap();

        let raw = fs::read_to_string(&path).unwrap();
        let parsed: StampManifest = serde_json::from_str(&raw).unwrap();
        assert_eq!(parsed, manifest);

        fs::remove_file(path).unwrap();
    }
}
