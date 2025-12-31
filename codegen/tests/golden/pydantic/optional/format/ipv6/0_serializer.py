"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "format": "ipv6"
}

Tests:
[
  {
    "data": 12,
    "description": "all string formats ignore integers",
    "valid": true
  },
  {
    "data": 13.7,
    "description": "all string formats ignore floats",
    "valid": true
  },
  {
    "data": {},
    "description": "all string formats ignore objects",
    "valid": true
  },
  {
    "data": [],
    "description": "all string formats ignore arrays",
    "valid": true
  },
  {
    "data": false,
    "description": "all string formats ignore booleans",
    "valid": true
  },
  {
    "data": null,
    "description": "all string formats ignore nulls",
    "valid": true
  },
  {
    "data": "::1",
    "description": "a valid IPv6 address",
    "valid": true
  },
  {
    "data": "12345::",
    "description": "an IPv6 address with out-of-range values",
    "valid": false
  },
  {
    "data": "::abef",
    "description": "trailing 4 hex symbols is valid",
    "valid": true
  },
  {
    "data": "::abcef",
    "description": "trailing 5 hex symbols is invalid",
    "valid": false
  },
  {
    "data": "1:1:1:1:1:1:1:1:1:1:1:1:1:1:1:1",
    "description": "an IPv6 address with too many components",
    "valid": false
  },
  {
    "data": "::laptop",
    "description": "an IPv6 address containing illegal characters",
    "valid": false
  },
  {
    "data": "::",
    "description": "no digits is valid",
    "valid": true
  },
  {
    "data": "::42:ff:1",
    "description": "leading colons is valid",
    "valid": true
  },
  {
    "data": "d6::",
    "description": "trailing colons is valid",
    "valid": true
  },
  {
    "data": ":2:3:4:5:6:7:8",
    "description": "missing leading octet is invalid",
    "valid": false
  },
  {
    "data": "1:2:3:4:5:6:7:",
    "description": "missing trailing octet is invalid",
    "valid": false
  },
  {
    "data": ":2:3:4::8",
    "description": "missing leading octet with omitted octets later",
    "valid": false
  },
  {
    "data": "1:d6::42",
    "description": "single set of double colons in the middle is valid",
    "valid": true
  },
  {
    "data": "1::d6::42",
    "description": "two sets of double colons is invalid",
    "valid": false
  },
  {
    "data": "1::d6:192.168.0.1",
    "description": "mixed format with the ipv4 section as decimal octets",
    "valid": true
  },
  {
    "data": "1:2::192.168.0.1",
    "description": "mixed format with double colons between the sections",
    "valid": true
  },
  {
    "data": "1::2:192.168.256.1",
    "description": "mixed format with ipv4 section with octet out of range",
    "valid": false
  },
  {
    "data": "1::2:192.168.ff.1",
    "description": "mixed format with ipv4 section with a hex octet",
    "valid": false
  },
  {
    "data": "::ffff:192.168.0.1",
    "description": "mixed format with leading double colons (ipv4-mapped ipv6 address)",
    "valid": true
  },
  {
    "data": "1:2:3:4:5:::8",
    "description": "triple colons is invalid",
    "valid": false
  },
  {
    "data": "1:2:3:4:5:6:7:8",
    "description": "8 octets",
    "valid": true
  },
  {
    "data": "1:2:3:4:5:6:7",
    "description": "insufficient octets without double colons",
    "valid": false
  },
  {
    "data": "1",
    "description": "no colons is invalid",
    "valid": false
  },
  {
    "data": "127.0.0.1",
    "description": "ipv4 is not ipv6",
    "valid": false
  },
  {
    "data": "1:2:3:4:1.2.3",
    "description": "ipv4 segment must have 4 octets",
    "valid": false
  },
  {
    "data": "  ::1",
    "description": "leading whitespace is invalid",
    "valid": false
  },
  {
    "data": "::1  ",
    "description": "trailing whitespace is invalid",
    "valid": false
  },
  {
    "data": "fe80::/64",
    "description": "netmask is not a part of ipv6 address",
    "valid": false
  },
  {
    "data": "fe80::a%eth1",
    "description": "zone id is not a part of ipv6 address",
    "valid": false
  },
  {
    "data": "1000:1000:1000:1000:1000:1000:255.255.255.255",
    "description": "a long valid ipv6",
    "valid": true
  },
  {
    "data": "100:100:100:100:100:100:255.255.255.255.255",
    "description": "a long invalid ipv6, below length limit, first",
    "valid": false
  },
  {
    "data": "100:100:100:100:100:100:100:255.255.255.255",
    "description": "a long invalid ipv6, below length limit, second",
    "valid": false
  },
  {
    "data": "1:2:3:4:5:6:7:৪",
    "description": "invalid non-ASCII '৪' (a Bengali 4)",
    "valid": false
  },
  {
    "data": "1:2::192.16৪.0.1",
    "description": "invalid non-ASCII '৪' (a Bengali 4) in the IPv4 portion",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Ipv60Serializer(SerializerRootModel):
    root: Any

