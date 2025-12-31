use crate::error::{CodegenError, SchemaPath};
use crate::model::{
    AdditionalProperties, ArrayConstraints, FieldSpec, ModelGraph, ModelSpec, NumberConstraints,
    ObjectConstraints, SchemaType, StringConstraints, StringFormat,
};
use crate::strings::{sanitize_type_name, NameAllocator};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, HashMap, HashSet};

pub struct ModelGraphBuilder<'a> {
    root: &'a Value,
    names: NameAllocator,
    models: BTreeMap<String, ModelSpec>,
    ref_map: HashMap<String, String>,
}

impl<'a> ModelGraphBuilder<'a> {
    pub fn new(root: &'a Value) -> Self {
        Self {
            root,
            names: NameAllocator::default(),
            models: BTreeMap::new(),
            ref_map: HashMap::new(),
        }
    }

    pub fn build(mut self, root_name: &str) -> Result<ModelGraph, CodegenError> {
        let base_name = sanitize_type_name(root_name);
        let root_model_name = self.names.reserve_exact(&base_name)?;
        self.ref_map
            .insert("#".to_string(), root_model_name.clone());

        let path = SchemaPath::root();
        let root_type = self.parse_root_schema(self.root, &path, &root_model_name)?;

        Ok(ModelGraph {
            root_name: root_model_name,
            root_type,
            models: self.models,
        })
    }

    fn parse_root_schema(
        &mut self,
        schema: &Value,
        path: &SchemaPath,
        root_name: &str,
    ) -> Result<SchemaType, CodegenError> {
        if let Some(obj) = schema.as_object() {
            if let Some(Value::String(ref_path)) = obj.get("$ref") {
                let resolved = resolve_ref(self.root, ref_path, path)?;
                if let Some(existing) = self.ref_map.get(&resolved.canonical) {
                    return Ok(SchemaType::Object(existing.clone()));
                }
                if let Some(resolved_obj) = resolved.value.as_object() {
                    if is_object_like(resolved_obj) {
                        let requires_object = explicitly_object_type(resolved_obj);
                        self.ref_map
                            .insert(resolved.canonical.clone(), root_name.to_string());
                        let root_model = self.parse_object_model(
                            resolved.value,
                            &resolved.path,
                            root_name,
                            requires_object,
                        )?;
                        self.models.insert(root_name.to_string(), root_model);
                        return Ok(SchemaType::Object(root_name.to_string()));
                    }
                }
                return self.parse_schema_type(resolved.value, &resolved.path);
            }

            if is_object_like(obj) {
                let requires_object = explicitly_object_type(obj);
                let root_model =
                    self.parse_object_model(schema, path, root_name, requires_object)?;
                self.models.insert(root_name.to_string(), root_model);
                return Ok(SchemaType::Object(root_name.to_string()));
            }
        }

        self.parse_schema_type(schema, path)
    }

    fn parse_object_model(
        &mut self,
        schema: &Value,
        path: &SchemaPath,
        name: &str,
        requires_object: bool,
    ) -> Result<ModelSpec, CodegenError> {
        let obj = schema
            .as_object()
            .ok_or_else(|| CodegenError::InvalidSchema {
                location: path.clone(),
                message: "object schema must be a JSON object".to_string(),
            })?;

        let description = obj
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned());
        let title = obj
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned());

        let mut properties: BTreeMap<String, Value> = BTreeMap::new();
        if let Some(Value::Object(props)) = obj.get("properties") {
            for (k, v) in props {
                properties.insert(k.clone(), v.clone());
            }
        }

        let required: HashSet<String> = obj
            .get("required")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_owned()))
                    .collect()
            })
            .unwrap_or_default();

        for name in &required {
            properties
                .entry(name.clone())
                .or_insert_with(|| Value::Bool(true));
        }

        let mut fields = Vec::new();
        for (prop_name, prop_schema) in properties {
            let field_path = path.push("properties").push(prop_name.clone());
            let field = self.parse_field(prop_name, &prop_schema, &required, &field_path)?;
            fields.push(field);
        }

        let additional_properties = match obj.get("additionalProperties") {
            None => AdditionalProperties::Allow,
            Some(Value::Bool(false)) => AdditionalProperties::Forbid,
            Some(Value::Bool(true)) => AdditionalProperties::Allow,
            Some(other) => {
                let extra_path = path.push("additionalProperties");
                let extra_schema = self.parse_schema_type(other, &extra_path)?;
                match extra_schema {
                    SchemaType::Any => AdditionalProperties::Allow,
                    SchemaType::Null => AdditionalProperties::Typed(Box::new(SchemaType::Null)),
                    other => AdditionalProperties::Typed(Box::new(other)),
                }
            }
        };

        let min_properties = obj
            .get("minProperties")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);
        let max_properties = obj
            .get("maxProperties")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);

        let pattern_properties = obj
            .get("patternProperties")
            .and_then(|v| v.as_object())
            .map(|map| map.keys().cloned().collect())
            .unwrap_or_default();

        let property_name_max = obj
            .get("propertyNames")
            .and_then(|v| v.as_object())
            .and_then(|o| o.get("maxLength"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);

        let has_all_of = obj
            .get("__allOf_props__")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Ok(ModelSpec {
            name: name.to_string(),
            fields,
            additional_properties,
            min_properties,
            max_properties,
            pattern_properties,
            property_name_max,
            has_all_of,
            requires_object,
            description,
            title,
            schema_path: path.clone(),
        })
    }

    fn parse_field(
        &mut self,
        prop_name: String,
        schema: &Value,
        required: &HashSet<String>,
        path: &SchemaPath,
    ) -> Result<FieldSpec, CodegenError> {
        let required = required.contains(&prop_name);

        let (default, title, description, read_only, write_only) =
            if let Some(obj) = schema.as_object() {
                let default = obj.get("default").cloned();
                let title = obj
                    .get("title")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_owned());
                let description = obj
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_owned());
                let read_only = obj
                    .get("readOnly")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let write_only = obj
                    .get("writeOnly")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                (default, title, description, read_only, write_only)
            } else {
                (None, None, None, false, false)
            };

        let schema_type = self.parse_schema_type(schema, path)?;

        if let Some(def) = &default {
            self.validate_default(&schema_type, def, path)?;
        }

        Ok(FieldSpec {
            name: prop_name,
            schema: schema_type,
            required,
            default,
            description,
            title,
            read_only,
            write_only,
        })
    }

    fn parse_schema_type(
        &mut self,
        schema: &Value,
        path: &SchemaPath,
    ) -> Result<SchemaType, CodegenError> {
        match schema {
            Value::Bool(true) => Ok(SchemaType::Any),
            Value::Bool(false) => Err(CodegenError::UnsupportedFeature {
                location: path.clone(),
                feature: "false schema".to_string(),
            }),
            Value::Object(obj) => {
                if let Some(Value::String(ref_path)) = obj.get("$ref") {
                    return self.parse_ref(ref_path, path);
                }

                if let Some(const_val) = obj.get("const") {
                    return self.parse_literal(std::slice::from_ref(const_val), path);
                }

                if let Some(Value::Array(values)) = obj.get("enum") {
                    return self.parse_literal(values, path);
                }

                if let Some(Value::Array(subs)) = obj.get("anyOf") {
                    return self.parse_union(subs, path, "anyOf");
                }

                if let Some(Value::Array(subs)) = obj.get("oneOf") {
                    return self.parse_union(subs, path, "oneOf");
                }

                if let Some(Value::Array(subs)) = obj.get("allOf") {
                    return self.parse_all_of(subs, path);
                }

                if let Some(Value::Array(type_list)) = obj.get("type") {
                    return self.parse_type_union(obj, type_list, path);
                }

                if let Some(Value::String(type_str)) = obj.get("type") {
                    return self.parse_typed_schema(obj, type_str, path);
                }

                if is_object_like(obj) {
                    return self.parse_object_schema(obj, path, false);
                }
                if obj.contains_key("items")
                    || obj.contains_key("minItems")
                    || obj.contains_key("maxItems")
                    || obj.contains_key("contains")
                    || obj.contains_key("minContains")
                    || obj.contains_key("maxContains")
                {
                    return self.parse_array_schema(obj, path, false);
                }
                if obj.contains_key("minLength")
                    || obj.contains_key("maxLength")
                    || obj.contains_key("pattern")
                    || obj.contains_key("format")
                {
                    return self.parse_string_schema(obj, path, false);
                }
                if obj.contains_key("minimum")
                    || obj.contains_key("maximum")
                    || obj.contains_key("exclusiveMinimum")
                    || obj.contains_key("exclusiveMaximum")
                    || obj.contains_key("multipleOf")
                {
                    return self.parse_number_schema(obj, false, path, false);
                }

                Ok(SchemaType::Any)
            }
            _ => Err(CodegenError::InvalidSchema {
                location: path.clone(),
                message: "schema must be an object or boolean".to_string(),
            }),
        }
    }

    fn parse_ref(&mut self, ref_path: &str, path: &SchemaPath) -> Result<SchemaType, CodegenError> {
        let resolved = resolve_ref(self.root, ref_path, path)?;
        if let Some(existing) = self.ref_map.get(&resolved.canonical) {
            return Ok(SchemaType::Object(existing.clone()));
        }

        let title_hint = resolved
            .value
            .as_object()
            .and_then(|obj| obj.get("title"))
            .and_then(|v| v.as_str())
            .map(sanitize_type_name);
        let name_hint = title_hint
            .filter(|s| !s.is_empty())
            .or_else(|| resolved.segments.last().map(|s| sanitize_type_name(s)))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Model".to_string());
        let name = self.names.allocate(&name_hint)?;

        if is_object_schema(resolved.value) {
            self.ref_map
                .insert(resolved.canonical.clone(), name.clone());
            let requires_object = resolved
                .value
                .as_object()
                .is_some_and(explicitly_object_type);
            let model =
                self.parse_object_model(resolved.value, &resolved.path, &name, requires_object)?;
            self.models.insert(name.clone(), model);
            Ok(SchemaType::Object(name))
        } else {
            self.parse_schema_type(resolved.value, &resolved.path)
        }
    }

    fn parse_literal(
        &self,
        values: &[Value],
        _path: &SchemaPath,
    ) -> Result<SchemaType, CodegenError> {
        let mut literals = Vec::new();
        for value in values {
            let lit = match value {
                Value::Null => crate::model::LiteralValue::Null,
                Value::Bool(b) => crate::model::LiteralValue::Bool(*b),
                Value::String(s) => crate::model::LiteralValue::String(s.clone()),
                Value::Number(n) => crate::model::LiteralValue::Number(n.clone()),
                other => crate::model::LiteralValue::Json(other.clone()),
            };
            literals.push(lit);
        }
        Ok(SchemaType::Literal(literals))
    }

    fn parse_union(
        &mut self,
        subs: &[Value],
        path: &SchemaPath,
        keyword: &str,
    ) -> Result<SchemaType, CodegenError> {
        if subs.is_empty() {
            return Err(CodegenError::InvalidSchema {
                location: path.clone(),
                message: format!("{keyword} must contain at least one schema"),
            });
        }
        let mut variants = Vec::new();
        for (idx, sub) in subs.iter().enumerate() {
            let sub_path = path.push(keyword).push(idx.to_string());
            variants.push(self.parse_schema_type(sub, &sub_path)?);
        }
        Ok(normalize_union(variants))
    }

    fn parse_all_of(
        &mut self,
        subs: &[Value],
        path: &SchemaPath,
    ) -> Result<SchemaType, CodegenError> {
        if subs.is_empty() {
            return Err(CodegenError::InvalidSchema {
                location: path.clone(),
                message: "allOf must contain at least one schema".to_string(),
            });
        }

        let mut merged = Map::new();
        for (idx, sub) in subs.iter().enumerate() {
            let sub_path = path.push("allOf").push(idx.to_string());
            let resolved = if let Some(Value::String(ref_path)) =
                sub.as_object().and_then(|o| o.get("$ref"))
            {
                resolve_ref(self.root, ref_path, &sub_path)?
            } else {
                ResolvedRef {
                    value: sub,
                    canonical: sub_path.to_string(),
                    path: sub_path.clone(),
                    segments: sub_path.as_segments().to_vec(),
                }
            };
            let obj =
                resolved
                    .value
                    .as_object()
                    .ok_or_else(|| CodegenError::UnsupportedFeature {
                        location: resolved.path.clone(),
                        feature: "allOf with non-object schema".to_string(),
                    })?;
            if !is_object_like(obj) {
                return Err(CodegenError::UnsupportedFeature {
                    location: resolved.path.clone(),
                    feature: "allOf with non-object schema".to_string(),
                });
            }
            merge_object_schema(&mut merged, obj, &resolved.path)?;
        }
        merged.insert("__allOf_props__".to_string(), Value::Bool(true));
        let merged_value = Value::Object(merged);
        self.parse_schema_type(&merged_value, path)
    }

    fn parse_type_union(
        &mut self,
        obj: &Map<String, Value>,
        type_list: &[Value],
        path: &SchemaPath,
    ) -> Result<SchemaType, CodegenError> {
        let mut variants = Vec::new();
        for (idx, type_val) in type_list.iter().enumerate() {
            let type_str = type_val
                .as_str()
                .ok_or_else(|| CodegenError::InvalidSchema {
                    location: path.clone(),
                    message: "type entries must be strings".to_string(),
                })?;
            let mut cloned = obj.clone();
            cloned.insert("type".to_string(), Value::String(type_str.to_string()));
            cloned.remove("nullable");
            let sub_path = path.push("type").push(idx.to_string());
            variants.push(self.parse_typed_schema(&cloned, type_str, &sub_path)?);
        }

        let mut union = normalize_union(variants);
        if obj
            .get("nullable")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            union = normalize_union(vec![union, SchemaType::Null]);
        }
        Ok(union)
    }

    fn parse_typed_schema(
        &mut self,
        obj: &Map<String, Value>,
        type_str: &str,
        path: &SchemaPath,
    ) -> Result<SchemaType, CodegenError> {
        let mut schema = match type_str {
            "string" => self.parse_string_schema(obj, path, true)?,
            "number" => self.parse_number_schema(obj, false, path, true)?,
            "integer" => self.parse_number_schema(obj, true, path, true)?,
            "boolean" => SchemaType::Bool,
            "null" => SchemaType::Null,
            "array" => self.parse_array_schema(obj, path, true)?,
            "object" => self.parse_object_schema(obj, path, true)?,
            other => {
                return Err(CodegenError::UnsupportedFeature {
                    location: path.clone(),
                    feature: format!("type '{other}'"),
                })
            }
        };

        if obj
            .get("nullable")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            schema = normalize_union(vec![schema, SchemaType::Null]);
        }
        Ok(schema)
    }

    fn parse_object_schema(
        &mut self,
        obj: &Map<String, Value>,
        path: &SchemaPath,
        requires_object: bool,
    ) -> Result<SchemaType, CodegenError> {
        let has_properties = obj.get("properties").is_some() || obj.get("required").is_some();
        let additional = obj.get("additionalProperties");

        if !has_properties {
            if let Some(Value::Bool(false)) = additional {
                let name = self.names.allocate(&sanitize_type_name("EmptyObject"))?;
                let model = self.parse_object_model(
                    &Value::Object(obj.clone()),
                    path,
                    &name,
                    requires_object,
                )?;
                self.models.insert(name.clone(), model);
                return Ok(SchemaType::Object(name));
            }

            if matches!(additional, Some(Value::Object(_)) | Some(Value::Bool(true))) {
                let constraints = ObjectConstraints {
                    min_properties: obj
                        .get("minProperties")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as usize),
                    max_properties: obj
                        .get("maxProperties")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as usize),
                    type_enforced: requires_object,
                };
                let value_schema = match additional {
                    Some(Value::Object(extra)) => {
                        let extra_path = path.push("additionalProperties");
                        self.parse_schema_type(&Value::Object(extra.clone()), &extra_path)?
                    }
                    _ => SchemaType::Any,
                };
                return Ok(SchemaType::Map {
                    values: Box::new(value_schema),
                    constraints,
                });
            }
        }

        let name_hint = obj
            .get("title")
            .and_then(|v| v.as_str())
            .map(sanitize_type_name)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Model".to_string());
        let preferred = self.ref_map.get(&path.to_string()).cloned();
        let name = if let Some(existing) = preferred {
            existing
        } else {
            self.names.allocate(&name_hint)?
        };
        let model =
            self.parse_object_model(&Value::Object(obj.clone()), path, &name, requires_object)?;
        self.models.insert(name.clone(), model);
        Ok(SchemaType::Object(name))
    }

    fn parse_array_schema(
        &mut self,
        obj: &Map<String, Value>,
        path: &SchemaPath,
        type_enforced: bool,
    ) -> Result<SchemaType, CodegenError> {
        if obj.contains_key("prefixItems") || obj.contains_key("contains") {
            return Err(CodegenError::UnsupportedFeature {
                location: path.clone(),
                feature: "prefixItems/contains".to_string(),
            });
        }
        if obj.get("uniqueItems").and_then(|v| v.as_bool()) == Some(true) {
            return Err(CodegenError::UnsupportedFeature {
                location: path.clone(),
                feature: "uniqueItems".to_string(),
            });
        }

        let items = match obj.get("items") {
            None => SchemaType::Any,
            Some(Value::Array(_)) => {
                return Err(CodegenError::UnsupportedFeature {
                    location: path.clone(),
                    feature: "tuple-style items".to_string(),
                })
            }
            Some(other) => {
                let items_path = path.push("items");
                self.parse_schema_type(other, &items_path)?
            }
        };

        let constraints = ArrayConstraints {
            min_items: obj.get("minItems").and_then(|v| v.as_u64()),
            max_items: obj.get("maxItems").and_then(|v| v.as_u64()),
            type_enforced,
        };

        Ok(SchemaType::Array {
            items: Box::new(items),
            constraints,
        })
    }

    fn parse_string_schema(
        &self,
        obj: &Map<String, Value>,
        path: &SchemaPath,
        type_enforced: bool,
    ) -> Result<SchemaType, CodegenError> {
        if obj.contains_key("contentEncoding") || obj.contains_key("contentMediaType") {
            return Err(CodegenError::UnsupportedFeature {
                location: path.clone(),
                feature: "contentEncoding/contentMediaType".to_string(),
            });
        }

        let constraints = StringConstraints {
            min_length: obj.get("minLength").and_then(|v| v.as_u64()),
            max_length: obj.get("maxLength").and_then(|v| v.as_u64()),
            pattern: obj
                .get("pattern")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            format: obj
                .get("format")
                .and_then(|v| v.as_str())
                .and_then(parse_string_format),
            type_enforced,
        };

        Ok(SchemaType::String(constraints))
    }

    fn parse_number_schema(
        &self,
        obj: &Map<String, Value>,
        integer: bool,
        path: &SchemaPath,
        type_enforced: bool,
    ) -> Result<SchemaType, CodegenError> {
        let mut minimum = obj.get("minimum").and_then(|v| v.as_f64());
        let mut maximum = obj.get("maximum").and_then(|v| v.as_f64());
        let mut exclusive_minimum = false;
        let mut exclusive_maximum = false;

        if let Some(exclusive) = obj.get("exclusiveMinimum") {
            match exclusive {
                Value::Number(n) => {
                    minimum = n.as_f64();
                    exclusive_minimum = true;
                }
                Value::Bool(flag) => exclusive_minimum = *flag,
                _ => {
                    return Err(CodegenError::InvalidSchema {
                        location: path.clone(),
                        message: "exclusiveMinimum must be a number or boolean".to_string(),
                    })
                }
            }
        }

        if let Some(exclusive) = obj.get("exclusiveMaximum") {
            match exclusive {
                Value::Number(n) => {
                    maximum = n.as_f64();
                    exclusive_maximum = true;
                }
                Value::Bool(flag) => exclusive_maximum = *flag,
                _ => {
                    return Err(CodegenError::InvalidSchema {
                        location: path.clone(),
                        message: "exclusiveMaximum must be a number or boolean".to_string(),
                    })
                }
            }
        }

        let multiple_of = obj
            .get("multipleOf")
            .and_then(|v| v.as_f64())
            .filter(|v| *v > 0.0);

        let constraints = NumberConstraints {
            minimum,
            maximum,
            exclusive_minimum,
            exclusive_maximum,
            multiple_of,
            type_enforced,
        };

        Ok(if integer {
            SchemaType::Integer(constraints)
        } else {
            SchemaType::Number(constraints)
        })
    }

    fn validate_default(
        &self,
        schema: &SchemaType,
        value: &Value,
        path: &SchemaPath,
    ) -> Result<(), CodegenError> {
        if default_matches_schema(schema, value) {
            return Ok(());
        }
        Err(CodegenError::InvalidDefault {
            location: path.clone(),
            message: format!("default value {value} does not match {schema:?}"),
        })
    }
}

struct ResolvedRef<'a> {
    value: &'a Value,
    canonical: String,
    path: SchemaPath,
    segments: Vec<String>,
}

fn resolve_ref<'a>(
    root: &'a Value,
    ref_path: &str,
    location: &SchemaPath,
) -> Result<ResolvedRef<'a>, CodegenError> {
    if !ref_path.starts_with('#') {
        return Err(CodegenError::InvalidRef {
            location: location.clone(),
            ref_path: ref_path.to_string(),
            message: "only local $ref values are supported".to_string(),
        });
    }

    if ref_path == "#" {
        return Ok(ResolvedRef {
            value: root,
            canonical: "#".to_string(),
            path: SchemaPath::root(),
            segments: Vec::new(),
        });
    }

    let pointer = ref_path.trim_start_matches('#');
    if !pointer.starts_with('/') {
        return Err(CodegenError::InvalidRef {
            location: location.clone(),
            ref_path: ref_path.to_string(),
            message: "invalid JSON pointer".to_string(),
        });
    }

    let mut current = root;
    let mut segments = Vec::new();
    for raw in pointer.trim_start_matches('/').split('/') {
        let decoded = decode_ref_token(raw).map_err(|e| CodegenError::InvalidRef {
            location: location.clone(),
            ref_path: ref_path.to_string(),
            message: e,
        })?;
        segments.push(decoded.clone());
        current = match current {
            Value::Object(map) => map.get(&decoded).ok_or_else(|| CodegenError::RefNotFound {
                location: location.clone(),
                ref_path: ref_path.to_string(),
            })?,
            Value::Array(arr) => {
                let idx = decoded
                    .parse::<usize>()
                    .map_err(|_| CodegenError::InvalidRef {
                        location: location.clone(),
                        ref_path: ref_path.to_string(),
                        message: "array index must be an integer".to_string(),
                    })?;
                arr.get(idx).ok_or_else(|| CodegenError::RefNotFound {
                    location: location.clone(),
                    ref_path: ref_path.to_string(),
                })?
            }
            _ => {
                return Err(CodegenError::RefNotFound {
                    location: location.clone(),
                    ref_path: ref_path.to_string(),
                })
            }
        };
    }

    let path = segments
        .iter()
        .fold(SchemaPath::root(), |p, seg| p.push(seg.clone()));
    let canonical = path.to_string();

    Ok(ResolvedRef {
        value: current,
        canonical,
        path,
        segments,
    })
}

fn decode_ref_token(raw: &str) -> Result<String, String> {
    let decoded = percent_decode(raw)?;
    Ok(decoded.replace("~1", "/").replace("~0", "~"))
}

fn percent_decode(input: &str) -> Result<String, String> {
    let mut out = Vec::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return Err("incomplete percent escape".to_string());
            }
            let hex = &input[i + 1..i + 3];
            let value = u8::from_str_radix(hex, 16)
                .map_err(|_| format!("invalid percent escape '%{hex}'"))?;
            out.push(value);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| "invalid percent-encoded utf-8".to_string())
}

fn normalize_union(types: Vec<SchemaType>) -> SchemaType {
    let mut flat = Vec::new();
    for ty in types {
        collect_union_variants(ty, &mut flat);
    }

    let mut dedup = Vec::new();
    for ty in flat {
        if !dedup.contains(&ty) {
            dedup.push(ty);
        }
    }

    let has_null = dedup.iter().any(|t| matches!(t, SchemaType::Null));
    if has_null {
        let mut non_null: Vec<SchemaType> = dedup
            .into_iter()
            .filter(|t| !matches!(t, SchemaType::Null))
            .collect();
        if non_null.is_empty() {
            return SchemaType::Null;
        }
        if non_null.len() == 1 {
            return SchemaType::Nullable(Box::new(non_null.remove(0)));
        }
        non_null.push(SchemaType::Null);
        return SchemaType::Union(non_null);
    }

    if dedup.len() == 1 {
        dedup.remove(0)
    } else {
        SchemaType::Union(dedup)
    }
}

fn collect_union_variants(schema: SchemaType, out: &mut Vec<SchemaType>) {
    match schema {
        SchemaType::Union(variants) => {
            for v in variants {
                collect_union_variants(v, out);
            }
        }
        SchemaType::Nullable(inner) => {
            collect_union_variants(*inner, out);
            out.push(SchemaType::Null);
        }
        other => out.push(other),
    }
}

fn parse_string_format(value: &str) -> Option<StringFormat> {
    match value {
        "date-time" => Some(StringFormat::DateTime),
        "date" => Some(StringFormat::Date),
        "time" => Some(StringFormat::Time),
        "uuid" => Some(StringFormat::Uuid),
        "email" => Some(StringFormat::Email),
        "uri" => Some(StringFormat::Uri),
        _ => None,
    }
}

fn explicitly_object_type(obj: &Map<String, Value>) -> bool {
    if obj.get("type").and_then(|v| v.as_str()) == Some("object") {
        return true;
    }
    if let Some(types) = obj.get("type").and_then(|v| v.as_array()) {
        return types.iter().any(|v| v.as_str() == Some("object"));
    }
    false
}

fn is_object_like(obj: &Map<String, Value>) -> bool {
    explicitly_object_type(obj)
        || obj.contains_key("properties")
        || obj.contains_key("required")
        || obj.contains_key("additionalProperties")
        || obj.contains_key("minProperties")
        || obj.contains_key("maxProperties")
}

fn is_object_schema(schema: &Value) -> bool {
    schema.as_object().is_some_and(is_object_like)
}

fn merge_object_schema(
    target: &mut Map<String, Value>,
    source: &Map<String, Value>,
    path: &SchemaPath,
) -> Result<(), CodegenError> {
    if let Some(Value::Object(props)) = source.get("properties") {
        let entry = target
            .entry("properties".to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        let target_props = entry
            .as_object_mut()
            .ok_or_else(|| CodegenError::InvalidSchema {
                location: path.clone(),
                message: "properties must be an object".to_string(),
            })?;
        for (key, value) in props {
            if let Some(existing) = target_props.get(key) {
                if existing != value {
                    return Err(CodegenError::UnsupportedFeature {
                        location: path.clone(),
                        feature: format!("conflicting property '{key}' in allOf"),
                    });
                }
            } else {
                target_props.insert(key.clone(), value.clone());
            }
        }
    }

    if let Some(Value::Array(reqs)) = source.get("required") {
        let entry = target
            .entry("required".to_string())
            .or_insert_with(|| Value::Array(Vec::new()));
        let target_reqs = entry
            .as_array_mut()
            .ok_or_else(|| CodegenError::InvalidSchema {
                location: path.clone(),
                message: "required must be an array".to_string(),
            })?;
        for req in reqs {
            if !target_reqs.contains(req) {
                target_reqs.push(req.clone());
            }
        }
    }

    if let Some(additional) = source.get("additionalProperties") {
        if let Some(existing) = target.get("additionalProperties") {
            if existing != additional {
                return Err(CodegenError::UnsupportedFeature {
                    location: path.clone(),
                    feature: "conflicting additionalProperties in allOf".to_string(),
                });
            }
        } else {
            target.insert("additionalProperties".to_string(), additional.clone());
        }
    }

    for key in ["minProperties", "maxProperties", "title", "description"] {
        if let Some(value) = source.get(key) {
            target
                .entry(key.to_string())
                .or_insert_with(|| value.clone());
        }
    }

    Ok(())
}

fn default_matches_schema(schema: &SchemaType, value: &Value) -> bool {
    match schema {
        SchemaType::Any => true,
        SchemaType::Bool => matches!(value, Value::Bool(_)),
        SchemaType::String(_) => matches!(value, Value::String(_)),
        SchemaType::Integer(_) => {
            value.as_i64().is_some()
                || value.as_u64().is_some()
                || value
                    .as_f64()
                    .is_some_and(|v| (v.fract() - 0.0).abs() < f64::EPSILON)
        }
        SchemaType::Number(_) => matches!(value, Value::Number(_)),
        SchemaType::Null => matches!(value, Value::Null),
        SchemaType::Array { items, .. } => value
            .as_array()
            .map(|arr| arr.iter().all(|v| default_matches_schema(items, v)))
            .unwrap_or(false),
        SchemaType::Map { values, .. } => value
            .as_object()
            .map(|map| map.values().all(|v| default_matches_schema(values, v)))
            .unwrap_or(false),
        SchemaType::Object(_) => value.is_object(),
        SchemaType::Literal(values) => values.iter().any(|v| v.matches_value(value)),
        SchemaType::Union(variants) => variants.iter().any(|v| default_matches_schema(v, value)),
        SchemaType::Nullable(inner) => value.is_null() || default_matches_schema(inner, value),
    }
}
