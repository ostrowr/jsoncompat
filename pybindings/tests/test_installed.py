"""Smoke tests for the installed package; run via check_sdist.py."""

from importlib import machinery, resources
import json
from pathlib import Path
import subprocess
import sys
from typing import Literal
import unittest

import jsoncompat
from jsoncompat import _native


class InstalledDistributionTests(unittest.TestCase):
    def test_imports_come_from_the_virtualenv(self) -> None:
        prefix = Path(sys.prefix).resolve()
        for module in (jsoncompat, _native):
            self.assertIsNotNone(module.__file__)
            assert module.__file__ is not None
            self.assertTrue(Path(module.__file__).resolve().is_relative_to(prefix))
        self.assertTrue(
            any(
                str(_native.__file__).endswith(suffix)
                for suffix in machinery.EXTENSION_SUFFIXES
            )
        )

    def test_typing_files_are_packaged(self) -> None:
        package = resources.files("jsoncompat")
        self.assertTrue((package / "py.typed").is_file())
        self.assertIn("JSONCOMPAT_MISSING:", (package / "_native.pyi").read_text())

    def test_native_schema_apis(self) -> None:
        schema = '{"type":"integer","minimum":1,"maximum":10}'
        self.assertTrue(jsoncompat.check_compat(schema, schema, jsoncompat.Role.BOTH))
        self.assertFalse(
            jsoncompat.check_compat(schema, '{"type":"string"}', jsoncompat.Role.BOTH)
        )
        validator = jsoncompat.validator_for(schema)
        self.assertTrue(validator.is_valid_json("5"))
        self.assertFalse(validator.is_valid_json('"5"'))
        self.assertTrue(validator.is_valid_value(5))
        self.assertFalse(validator.is_valid_value(0))
        generator = jsoncompat.generator_for(schema)
        self.assertTrue(validator.is_valid_json(generator.generate_value(depth=2)))

    def test_keyword_support_and_three_way_verdicts(self) -> None:
        decimal = '{"type":"number","multipleOf":0.1}'
        validator = jsoncompat.validator_for(decimal)
        self.assertTrue(validator.is_valid_json("0.3"))
        self.assertFalse(validator.is_valid_json("0.30000000000000004"))
        self.assertFalse(validator.is_valid_value(0.30000000000000004))
        result = jsoncompat.analyze_compat(decimal, '{"type":"number"}', "serializer")
        self.assertEqual(result["status"], "incompatible")
        self.assertFalse(validator.is_valid_value(result["counterexample"]))
        self.assertTrue(jsoncompat.is_valid('{"type":"string","contentSchema":{"$ref":"https://example.com/not-fetched"}}', '"opaque"'))

    def run_example(self, name: Literal["dataclasses", "stamp"]) -> None:
        example = Path(__file__).resolve().parent / "examples" / name
        # Only copied example modules are added to sys.path, never the checkout.
        subprocess.run(
            [
                sys.executable,
                "-I",
                "-c",
                "import runpy, sys; sys.path.insert(0, sys.argv[1]); "
                "runpy.run_path(sys.argv[2], run_name='__main__')",
                str(example),
                str(example / "demo.py"),
            ],
            cwd=example,
            check=True,
        )

    def test_generated_dataclasses_and_codecs(self) -> None:
        self.run_example("dataclasses")

    def test_stamped_readers_and_writers(self) -> None:
        self.run_example("stamp")


if __name__ == "__main__":
    unittest.main()
