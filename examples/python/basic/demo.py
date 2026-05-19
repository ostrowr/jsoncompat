# /// script
# requires-python = ">=3.12"
# dependencies = ["jsoncompat"]
# [tool.uv.sources]
# jsoncompat = { path = "../../../python", editable = true }
# ///
"""End-to-end demo for the Python bindings of jsoncompat.

Executed by `just python-demo` which automatically builds the extension
module via *maturin* and then runs this script.

Alternatively, run `uv run --prerelease=allow examples/python/basic/demo.py`
to grab jsoncompat from PyPI and run the script.
"""

import jsoncompat as jsc


def main() -> None:
    new_schema = """{
  "type": "object",
  "properties": {
    "name": { "type": "string", "minLength": 5 },
    "age": { "type": "integer", "minimum": 18 }
  },
  "required": ["name"]
}"""
    old_schema = """{
  "type": "object",
  "properties": {
    "name": { "type": "string", "minLength": 5 },
    "age": { "type": "integer", "minimum": 18 }
  }
}"""

    print("=== Compatibility checks ===")
    roles: list[jsc.RoleLiteral] = [
        "serializer",
        "deserializer",
        "both",
    ]
    for role in roles:
        ok = jsc.check_compat(old_schema, new_schema, role)
        print(f"{role:12}: {ok}")

    print("\n=== Example value generation ===")
    example = jsc.generate_value(old_schema, 3)
    print("example value:", example)

    print("\n=== Reusable validation ===")
    validator = jsc.validator_for(old_schema)
    assert validator.is_valid_json(example)
    assert validator.is_valid_value({"name": "Robbie", "age": 37})
    assert not validator.is_valid_value({"name": True})

    try:
        validator.is_valid_value({"age": float("nan")})
    except ValueError:
        pass
    else:
        raise AssertionError("non-finite JSON numbers must be rejected")

    try:
        validator.is_valid_value({1: "invalid"})
    except TypeError:
        pass
    else:
        raise AssertionError("JSON object keys must be strings")

    integer_validator = jsc.validator_for('{"type": "integer"}')
    assert integer_validator.is_valid_value(1)
    assert not integer_validator.is_valid_value(True)

    try:
        integer_validator.is_valid_value(2**2000)
    except ValueError:
        pass
    else:
        raise AssertionError("oversized Python integers must be rejected")

    print("generated JSON is valid:", validator.is_valid_json(example))
    print(
        "Python value is valid:",
        validator.is_valid_value({"name": "Robbie", "age": 37}),
    )


if __name__ == "__main__":
    main()
