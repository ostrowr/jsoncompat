"""Operation slices must not quietly loosen public contracts."""
import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import check_public_openapi_slices as slices
from check_public_openapi import digest


class PublicOpenApiSliceTests(unittest.TestCase):
    def test_projection_removes_whole_operations_and_preserves_everything_else(self) -> None:
        document = {
            "openapi": "3.1.0", "info": {"title": "Corpus", "version": "1"},
            "servers": [{"url": "https://example.invalid"}], "x-note": {"keep": True},
            "components": {"schemas": {
                "Names": {"type": "array", "items": {"type": "string"}, "uniqueItems": True},
                "Unused": {"type": "integer", "minimum": 7},
            }, "responses": {"Problem": {"description": "Bad", "content": {
                "application/problem+json": {"schema": {"type": "string", "minLength": 1}}
            }}}},
            "paths": {"/items/{id}": {
                "summary": "Retain path metadata", "parameters": [{"name": "id", "in": "path", "required": True, "schema": {"type": "string"}}],
                "get": {"operationId": "read", "responses": {
                    "200": {"description": "OK", "content": {"application/json": {"schema": {"$ref": "#/components/schemas/Names"}}}},
                    "400": {"$ref": "#/components/responses/Problem"},
                }},
                "post": {"operationId": "write", "requestBody": {"content": {
                    "application/json": {"schema": {"type": "string"}},
                    "application/octet-stream": {"schema": {"type": "string", "format": "binary"}},
                }}, "responses": {"204": {"description": "Empty"}}},
                "delete": {"operationId": "delete", "responses": {
                    "204": {"description": "Empty"},
                    "500": {"description": "HTML error", "content": {"text/html": {"schema": {"type": "string"}}}},
                }},
            }},
        }
        original = copy.deepcopy(document)
        projected, included, excluded = slices.project_document(document)
        self.assertEqual(document, original)
        self.assertEqual({key: value for key, value in projected.items() if key != "paths"},
                         {key: value for key, value in original.items() if key != "paths"})
        path = projected["paths"]["/items/{id}"]
        self.assertEqual(path, {key: value for key, value in original["paths"]["/items/{id}"].items() if key not in {"post", "delete"}})
        self.assertEqual([item["method"] for item in included], ["GET"])
        self.assertEqual([item["method"] for item in excluded], ["POST", "DELETE"])
        self.assertEqual([item["reasons"][0]["location"] for item in excluded], ["request", "response 500"])
        self.assertEqual([item["reasons"][0]["media_type"] for item in excluded], ["application/octet-stream", "text/html"])
        projected["components"]["schemas"]["Names"]["uniqueItems"] = False
        self.assertEqual(document, original, "projection must not mutate the source through aliases")

    def test_text_plain_requires_a_string_and_utf8_charset(self) -> None:
        def reasons(media: str, schema: dict) -> list:
            return slices.media_exclusions({}, "request", {media: {"schema": schema}})
        self.assertEqual(reasons('text/plain; charset="utf-8"', {"type": "string"}), [])
        self.assertEqual(reasons("text/plain", {"enum": ["a", "b"]}), [])
        self.assertTrue(reasons("text/plain", {"type": "boolean"}))
        self.assertTrue(reasons("text/plain", {"type": "integer"}))
        self.assertTrue(reasons("text/plain", {"type": ["string", "null"]}))
        self.assertTrue(reasons("application/json; charset=latin1", {"type": "string"}))

    def test_bodyless_operations_are_kept_and_empty_paths_are_removed(self) -> None:
        document = {"paths": {
            "/keep": {"delete": {"responses": {"204": {"description": "Empty", "content": {}}}}},
            "/drop": {"get": {"responses": {"200": {"content": {"audio/wav": {"schema": {"type": "string"}}}}}}},
        }}
        projected, included, excluded = slices.project_document(document)
        self.assertEqual(projected["paths"], {"/keep": document["paths"]["/keep"]})
        self.assertEqual(len(included), 1)
        self.assertEqual(len(excluded), 1)

    def test_snapshot_hash_is_verified_before_projection(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest = root / "manifest.json"
            manifest.write_text(json.dumps({"version": 1, "specs": [{
                "id": identifier, "sha256": digest(b"{}"), "source_url": "https://example.invalid"
            } for identifier in slices.SPEC_IDS]}))
            with patch.object(slices, "source_bytes", return_value=b'{"changed":true}'):
                with self.assertRaisesRegex(ValueError, "source SHA-256 mismatch"):
                    slices.build_slices(manifest, root / "output")
            self.assertFalse((root / "output/manifest.json").exists())


if __name__ == "__main__":
    unittest.main()
