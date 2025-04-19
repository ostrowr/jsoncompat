"""End-to-end demo for the Python bindings of jsoncompat.

Executed by `just python-demo` which automatically builds the extension
module via *maturin* and then runs this script.
"""

import json_schema_py as jsc


def main() -> None:  # pragma: no cover – demo only
    old_schema = '{"type": "string"}'
    new_schema = '{"type": "number"}'

    print("=== Compatibility checks ===")
    for role in ("serializer", "deserializer", "both"):
        ok = jsc.check_compat_py(old_schema, new_schema, role)
        print(f"{role:12}: {ok}")

    print("\n=== Example value generation ===")
    example = jsc.generate_value_py(old_schema, 3)
    print("example value:", example)


if __name__ == "__main__":
    main()
