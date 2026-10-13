"""Focused tests for the opt-in public corpus audit (no downloads/builds)."""
from pathlib import Path
from io import BytesIO
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import check_public_openapi as audit


class PublicOpenApiAuditTests(unittest.TestCase):
    def test_source_download_checks_hash_before_caching(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            entry = {"id": "example", "source_url": "https://example.invalid/openapi.json", "sha256": audit.digest(b"{}")}
            with patch.object(audit.urllib.request, "urlopen", return_value=BytesIO(b"{\"changed\":true}")):
                with self.assertRaisesRegex(ValueError, "SHA-256"):
                    audit.source_bytes(entry, root / "manifest.json", root / "audit")
            self.assertFalse((root / "audit/sources/example.spec").exists())
            with patch.object(audit.urllib.request, "urlopen", return_value=BytesIO(b"{}")):
                self.assertEqual(audit.source_bytes(entry, root / "manifest.json", root / "audit"), b"{}")
            self.assertEqual((root / "audit/sources/example.spec").read_bytes(), b"{}")

    def test_source_pointers_escape_paths_and_media_types(self) -> None:
        self.assertEqual(audit.pointer_segment("/pets/~named/{id}"), "~1pets~1~0named~1{id}")
        self.assertEqual(audit.pointer_segment("application/problem+json"), "application~1problem+json")

    def test_yaml_preserves_json_strings_and_uses_core_numbers(self) -> None:
        actual = audit.load_yaml(b"""date: 2026-10-12
on: no
yes: off
true_value: true
false_value: FALSE
leading: 012
octal: 0o12
hex: -0xA
sexagesimal: 1:20
exponent: 1e3
""")
        self.assertEqual(actual, {
            "date": "2026-10-12", "on": "no", "yes": "off",
            "true_value": True, "false_value": False, "leading": 12,
            "octal": 10, "hex": -10, "sexagesimal": "1:20", "exponent": 1000.0,
        })

    def test_yaml_cannot_construct_python_objects(self) -> None:
        import yaml
        with self.assertRaises(yaml.constructor.ConstructorError):
            audit.load_yaml(b"!!python/object/apply:os.system ['false']")

    def test_yaml_normalization_rejects_number_changes(self) -> None:
        for value in (b"0.1234567890123456789012345", b"1e-1000", b"1e1000", b".nan", b".inf", b"-.Inf"):
            with self.subTest(value=value), self.assertRaises(ValueError):
                audit.load_yaml(b"minimum: " + value)
        self.assertEqual(audit.load_yaml(b"minimum: 0.1"), {"minimum": 0.1})
        self.assertEqual(audit.load_yaml(b'minimum: "0.1234567890123456789012345"'),
                         {"minimum": "0.1234567890123456789012345"})

    def test_reference_inspection_keeps_siblings_and_stops_cycles(self) -> None:
        document = {"components": {"schemas": {
            "Name": {"type": "string", "description": "original"},
            "Cycle": {"$ref": "#/components/schemas/Cycle"},
        }}}
        self.assertEqual(audit.resolve(document, {
            "$ref": "#/components/schemas/Name", "description": "overridden",
        }), {"type": "string", "description": "overridden"})
        self.assertEqual(audit.resolve(document, {"$ref": "#/components/schemas/Cycle"}),
                         {"$ref": "#/components/schemas/Cycle"})

    def test_status_examples_do_not_accidentally_match_a_different_variant(self) -> None:
        self.assertEqual(audit.response_status(404, {404, "4XX"}), 404)
        self.assertEqual(audit.response_status("4XX", {499, "4XX"}), 498)
        self.assertEqual(audit.response_status("default", {"5XX", 499}), 498)
        self.assertIsNone(audit.response_status("default", {f"{n}XX" for n in range(1, 6)}))

    def test_examples_are_declared_values_not_external_fetches_or_synthesis(self) -> None:
        document = {"components": {"examples": {"Array": {"value": [1, 2]}}}}
        examples = audit.example_values(document, {
            "examples": {"a": {"$ref": "#/components/examples/Array"},
                         "remote": {"externalValue": "https://example.invalid/body.json"}},
            "schema": {"type": "string", "example": "yes", "default": "not-an-example"},
        }, "#/response")
        self.assertEqual(examples, [("#/response/examples/a/value", [1, 2]),
                                    ("#/response/schema/example", "yes")])
        self.assertEqual(audit.example_values(document, {"type": "integer", "minimum": 10}, "#"), [])

    def test_inventory_distinguishes_errors_and_nonobject_roots(self) -> None:
        document = {"openapi": "3.1.0", "paths": {"/items": {"get": {"responses": {
            "200": {"content": {"application/json": {"schema": {"type": "array"}}}},
            "404": {"content": {"application/problem+json": {"schema": {"type": "string"}}}},
            "default": {"description": "empty"},
        }}}}}
        actual = audit.spec_inventory(document)
        self.assertEqual(actual["operations"], 1)
        self.assertEqual(actual["response_statuses"], {"200": 1, "404": 1, "default": 1})
        self.assertEqual(actual["response_root_shapes"], {"array": 1, "string": 1})


if __name__ == "__main__":
    unittest.main()
