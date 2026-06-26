# /// script
# requires-python = ">=3.12"
# dependencies = ["jsoncompat[msgpack,yaml]"]
# [tool.uv.sources]
# jsoncompat = { path = "../../pybindings", editable = true }
# ///
"""Canonical end-to-end example for generated jsoncompat dataclasses."""

from jsoncompat import JsonValue
from jsoncompat.codegen import SerializationFormat

from reader_models import UserProfileReader, UserProfileV1Reader
from writer_models import UserProfileV2, UserProfileWriter


def describe(message: UserProfileReader) -> str:
    envelope = message.root
    if isinstance(envelope, UserProfileV1Reader):
        return f"v1 profile: {envelope.data.name}, age {envelope.data.age}"
    return (
        f"v2 profile: {envelope.data.name}, age {envelope.data.age}, "
        f"interests {envelope.data.interests}"
    )


def main() -> None:
    # Direct constructors validate by default.
    profile = UserProfileV2(name="Ada", age=37, interests=3)
    outbound = UserProfileWriter(version=2, data=profile)

    # Use to_value() when an integration wants a Python JSON value.
    value: JsonValue = outbound.to_value()
    assert value == {
        "version": 2,
        "data": {"name": "Ada", "age": 37, "interests": 3},
    }
    assert describe(UserProfileReader.from_value(value)) == (
        "v2 profile: Ada, age 37, interests 3"
    )

    # serialize()/deserialize() handle encoded wire formats.
    json_wire: str = outbound.serialize()
    yaml_wire: str = outbound.serialize(format=SerializationFormat.YAML)
    msgpack_wire: bytes = outbound.serialize(format=SerializationFormat.MSGPACK)

    print("JSON:", describe(UserProfileReader.deserialize(json_wire)))
    print(
        "YAML:",
        describe(
            UserProfileReader.deserialize(
                yaml_wire,
                format=SerializationFormat.YAML,
            )
        ),
    )
    print(
        "MessagePack:",
        describe(
            UserProfileReader.deserialize(
                msgpack_wire,
                format=SerializationFormat.MSGPACK,
            )
        ),
    )

    # Stamped readers accept historical writer versions too.
    historical = UserProfileReader.deserialize(
        '{"version":1,"data":{"name":"Grace","age":85}}'
    )
    print("Historical:", describe(historical))

    # skip_validation=True is an explicit promise that the value is valid.
    trusted_profile = UserProfileV2(
        name="Ada",
        age=37,
        interests=3,
        skip_validation=True,
    )
    trusted_outbound = UserProfileWriter(
        version=2,
        data=trusted_profile,
        skip_validation=True,
    )
    trusted_value = trusted_outbound.to_value(skip_validation=True)
    trusted_wire = trusted_outbound.serialize(skip_validation=True)
    trusted_from_value = UserProfileReader.from_value(
        trusted_value,
        skip_validation=True,
    )
    trusted_inbound = UserProfileReader.deserialize(
        trusted_wire,
        skip_validation=True,
    )
    assert trusted_value == value
    assert describe(trusted_from_value) == describe(trusted_inbound)
    assert describe(trusted_inbound) == describe(UserProfileReader.from_value(value))
    print("Trusted paths match checked paths")

    # Checked APIs reject schema-invalid values.
    try:
        UserProfileReader.from_value(
            {"version": 2, "data": {"name": "Ada", "age": -1}}
        )
    except ValueError:
        print("Invalid input rejected")
    else:
        raise AssertionError("checked reader accepted an invalid profile")

    # Writer envelopes only serialize; reader envelopes only deserialize.
    try:
        UserProfileWriter.deserialize(json_wire)
    except TypeError:
        pass
    else:
        raise AssertionError("writer envelope allowed deserialization")

    try:
        historical.serialize()
    except TypeError:
        print("Reader/writer direction guards enforced")
    else:
        raise AssertionError("reader envelope allowed serialization")


if __name__ == "__main__":
    main()
