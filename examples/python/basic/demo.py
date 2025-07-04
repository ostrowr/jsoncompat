# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "jsoncompat",
# ]
# ///
"""End-to-end demo for the Python bindings of jsoncompat.

Executed by `just python-demo` which automatically builds the extension
module via *maturin* and then runs this script.

Alternatively, run `uv run --prerelease=allow examples/python/basic/demo.py`
to grab jsoncompat from PyPI and run the script.
"""

import jsoncompat as jsc


def main() -> None:
    old_schema = '{"type": "string"}'
    new_schema = '{"type": "number"}'

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


if __name__ == "__main__":
    main()
