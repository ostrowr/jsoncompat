#[cfg(test)]
mod test_compat {
    use json_schema_backcompat::{build_and_resolve_schema, check_compat, Role};
    use serde_json::json;
    use url::Url;

    fn to_ast(value: serde_json::Value) -> json_schema_backcompat::SchemaNode {
        let base = Url::parse("file:///test.json").unwrap();
        let mut ast = json_schema_backcompat::build_schema_ast(&value).unwrap();
        json_schema_backcompat::resolve_refs(&mut ast, &value, &base, &[]).unwrap();
        ast
    }

    /// 1) Identical schemas => no break in either direction
    #[test]
    fn test_same_schema() {
        let schema = json!({
            "type": "object",
            "properties": {
                "id": { "type": "integer" }
            },
            "required": ["id"]
        });
        let s1 = to_ast(schema.clone());
        let s2 = to_ast(schema.clone());
        assert!(check_compat(&s1, &s2, Role::Serializer));
        assert!(check_compat(&s1, &s2, Role::Deserializer));
        assert!(check_compat(&s1, &s2, Role::Both));
    }

    /// 2) Required -> optional field:
    /// old => { required: ["id"] }
    /// new => { required: [] }
    /// => break for serializer (new can produce something old won't accept)
    /// => no break for deserializer (old data is still valid in new).
    #[test]
    fn test_required_to_optional() {
        let old_schema = json!({
            "type": "object",
            "properties": {
                "id": {"type": "integer"}
            },
            "required": ["id"]
        });
        let new_schema = json!({
            "type": "object",
            "properties": {
                "id": {"type": "integer"}
            },
            "required": []
        });
        let s_old = to_ast(old_schema);
        let s_new = to_ast(new_schema);

        // Serializer => new must be subset of old => false
        assert_eq!(check_compat(&s_old, &s_new, Role::Serializer), false);

        // Deserializer => old must be subset of new => true
        assert_eq!(check_compat(&s_old, &s_new, Role::Deserializer), true);
    }

    /// 3) Optional -> required field:
    /// old => { required: [] }
    /// new => { required: ["id"] }
    /// => break for deserializer (old data might not have 'id')
    /// => no break for serializer
    #[test]
    fn test_optional_to_required() {
        let old_schema = json!({
            "type": "object",
            "properties": {
                "id": {"type": "integer"}
            },
            "required": []
        });
        let new_schema = json!({
            "type": "object",
            "properties": {
                "id": {"type": "integer"}
            },
            "required": ["id"]
        });
        let s_old = to_ast(old_schema);
        let s_new = to_ast(new_schema);

        // Serializer => new must be subset of old => true (since new requiring a field doesn't break producing old data)
        assert_eq!(check_compat(&s_old, &s_new, Role::Serializer), true);

        // Deserializer => old must be subset of new => false
        assert_eq!(check_compat(&s_old, &s_new, Role::Deserializer), false);
    }

    /// 4) AdditionalProperties: old allows them, new disallows them => break for serializer
    #[test]
    fn test_additionalprops_reduced() {
        // old => additionalProperties: true (default)
        let old_schema = json!({
            "type": "object"
        });
        // new => additionalProperties: false
        let new_schema = json!({
            "type": "object",
            "additionalProperties": false
        });
        let s_old = to_ast(old_schema);
        let s_new = to_ast(new_schema);

        // serializer => new must be subset of old => if new disallows unknown props while old allowed them,
        // new is narrower => subset => that alone is not break for serializer. Actually let's check carefully:
        //   "new" accepts fewer objects => means there's some object "old" would produce (with extra fields)
        //   that "new" wouldn't. That means from the serializer perspective, that's a break. So we expect false.
        assert_eq!(check_compat(&s_old, &s_new, Role::Serializer), false);

        // deserializer => old must be subset of new => old allows everything, new is more restrictive,
        // means old data is definitely accepted by new if old had random fields => new wouldn't accept them => break
        // Wait, from the perspective of "deserializer," if old accepted everything, new won't. So old is not a subset => false
        assert_eq!(check_compat(&s_old, &s_new, Role::Deserializer), false);
    }

    /// 5) AdditionalProperties: old disallows them, new allows them => break for deserializer
    #[test]
    fn test_additionalprops_expanded() {
        let old_schema = json!({
            "type": "object",
            "additionalProperties": false
        });
        let new_schema = json!({
            "type": "object"
            // default "additionalProperties": true
        });
        let s_old = to_ast(old_schema);
        let s_new = to_ast(new_schema);

        // serializer => new must be subset of old => new allows more => new is superset => so not subset => break
        // Actually, from the serializer perspective, if "new" can produce objects with unknown fields that "old" can't accept => break => false
        assert_eq!(check_compat(&s_old, &s_new, Role::Serializer), false);

        // deserializer => old must be subset of new => old disallows unknown fields. So old's data is a subset
        // of new's data => that should be true => no break for deserializer.
        // Wait, re-check carefully: old is subset => everything old accepts is an object with no unknown fields => new is fine with those => true
        assert_eq!(check_compat(&s_old, &s_new, Role::Deserializer), true);
    }

    /// 6) Type changed from string -> number => breaks in both directions
    #[test]
    fn test_string_to_number() {
        let old_schema = json!({"type":"string"});
        let new_schema = json!({"type":"number"});

        let s_old = to_ast(old_schema);
        let s_new = to_ast(new_schema);

        // serializer => new must be subset of old => "number" is not subset of "string" => false
        assert_eq!(check_compat(&s_old, &s_new, Role::Serializer), false);

        // deserializer => old must be subset of new => "string" not subset of "number" => false
        assert_eq!(check_compat(&s_old, &s_new, Role::Deserializer), false);
    }

    /// 7) Minimally different enumerations
    /// old => enum [1,2]
    /// new => enum [1,2,3]
    /// => from serializer perspective => new is superset => not subset => break
    /// => from deserializer perspective => old is subset => no break
    #[test]
    fn test_enumeration_expand() {
        let old_schema = json!({
            "enum": [1,2]
        });
        let new_schema = json!({
            "enum": [1,2,3]
        });
        let s_old = to_ast(old_schema);
        let s_new = to_ast(new_schema);

        // serializer => new must be subset => new actually allows [1,2,3], old only [1,2].
        // => not a subset => false
        assert_eq!(check_compat(&s_old, &s_new, Role::Serializer), false);

        // deserializer => old must be subset => old allows [1,2], new allows [1,2,3] => old is subset => true
        assert_eq!(check_compat(&s_old, &s_new, Role::Deserializer), true);
    }

    /// 8) allOf with references => final combination might be narrower or broader.
    /// For simplicity, we simulate a reference that merges constraints. We'll do a small example:
    #[test]
    fn test_allof_refs_trick() {
        // define some "root" with a definition
        let root_schema = json!({
            "definitions": {
                "PosInt": {
                    "type": "integer",
                    "minimum": 0
                }
            },
            // main is allOf: [ { $ref: "#/definitions/PosInt" }, { "maximum": 10 } ]
            "allOf": [
                {"$ref":"#/definitions/PosInt"},
                {"maximum":10}
            ]
        });
        let ast = to_ast(root_schema);

        // We'll compare it to a simpler single schema { "type":"integer", "minimum":0, "maximum":5 }
        let simpler = json!({
            "type":"integer",
            "minimum":0,
            "maximum":5
        });
        let ast2 = to_ast(simpler);

        // So "ast" => integer >=0 <=10
        // "ast2" => integer >=0 <=5
        // => ast2 is narrower than ast => ast2 ⊆ ast => from the "serializer" perspective, if new=ast2, old=ast => no break
        // We'll do a direct check: is_subschema_of(ast2, ast)? => true
        assert!(check_compat(&ast, &ast2, Role::Serializer) == false, "old=ast, new=ast2 => new is narrower => we want to see if new is subset => yes => we expect true => oh wait, let's track carefully");

        // Actually let's be precise:
        // For serializer => new must be subset of old
        // old=ast => integer 0..10
        // new=ast2 => integer 0..5
        // Is 0..5 a subset of 0..10? Yes, so that means no break => we expect check_compat(...) = true
        assert_eq!(check_compat(&ast, &ast2, Role::Serializer), true);

        // For deserializer => old=ast must be subset of new=ast2 => 0..10 is not subset of 0..5 => break => false
        assert_eq!(check_compat(&ast, &ast2, Role::Deserializer), false);
    }

    /// 9) "anyOf" with multiple types => new => anyOf(string, number)
    /// old => "string"
    /// => from serializer perspective, new might accept a number that old won't => break
    /// => from deserializer perspective, old is subset => no break
    #[test]
    fn test_anyof() {
        let old_schema = json!({"type":"string"});
        let new_schema = json!({"anyOf":[{"type":"string"},{"type":"number"}]});
        let s_old = to_ast(old_schema);
        let s_new = to_ast(new_schema);

        // serializer => new must be subset => "string or number" is bigger than "string" alone => not subset => false
        assert_eq!(check_compat(&s_old, &s_new, Role::Serializer), false);

        // deserializer => old must be subset => "string" is subset of "string or number" => true
        assert_eq!(check_compat(&s_old, &s_new, Role::Deserializer), true);
    }

    /// 10) Tighter 'minLength' => break for deserializer, not for serializer
    /// old => minLength=0
    /// new => minLength=5
    /// => if old data has short strings, new won't accept => break for deserializer
    /// => serializer => new is narrower => subset => no break
    #[test]
    fn test_minlength() {
        let old_schema = json!({"type":"string","minLength":0});
        let new_schema = json!({"type":"string","minLength":5});

        let s_old = to_ast(old_schema);
        let s_new = to_ast(new_schema);

        // serializer => new must be subset => yes, because ">=5" is narrower than ">=0"
        assert_eq!(check_compat(&s_old, &s_new, Role::Serializer), true);

        // deserializer => old must be subset => no, because old accepted length=3, new doesn't
        assert_eq!(check_compat(&s_old, &s_new, Role::Deserializer), false);
    }
}
