"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "format": "uri"
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
    "data": "http://foo.bar/?baz=qux#quux",
    "description": "a valid URL with anchor tag",
    "valid": true
  },
  {
    "data": "http://foo.com/blah_(wikipedia)_blah#cite-1",
    "description": "a valid URL with anchor tag and parentheses",
    "valid": true
  },
  {
    "data": "http://foo.bar/?q=Test%20URL-encoded%20stuff",
    "description": "a valid URL with URL-encoded stuff",
    "valid": true
  },
  {
    "data": "http://xn--nw2a.xn--j6w193g/",
    "description": "a valid puny-coded URL ",
    "valid": true
  },
  {
    "data": "http://-.~_!$&'()*+,;=:%40:80%2f::::::@example.com",
    "description": "a valid URL with many special characters",
    "valid": true
  },
  {
    "data": "http://223.255.255.254",
    "description": "a valid URL based on IPv4",
    "valid": true
  },
  {
    "data": "ftp://ftp.is.co.za/rfc/rfc1808.txt",
    "description": "a valid URL with ftp scheme",
    "valid": true
  },
  {
    "data": "http://www.ietf.org/rfc/rfc2396.txt",
    "description": "a valid URL for a simple text file",
    "valid": true
  },
  {
    "data": "ldap://[2001:db8::7]/c=GB?objectClass?one",
    "description": "a valid URL ",
    "valid": true
  },
  {
    "data": "mailto:John.Doe@example.com",
    "description": "a valid mailto URI",
    "valid": true
  },
  {
    "data": "news:comp.infosystems.www.servers.unix",
    "description": "a valid newsgroup URI",
    "valid": true
  },
  {
    "data": "tel:+1-816-555-1212",
    "description": "a valid tel URI",
    "valid": true
  },
  {
    "data": "urn:oasis:names:specification:docbook:dtd:xml:4.1.2",
    "description": "a valid URN",
    "valid": true
  },
  {
    "data": "//foo.bar/?baz=qux#quux",
    "description": "an invalid protocol-relative URI Reference",
    "valid": false
  },
  {
    "data": "/abc",
    "description": "an invalid relative URI Reference",
    "valid": false
  },
  {
    "data": "\\\\WINDOWS\\fileshare",
    "description": "an invalid URI",
    "valid": false
  },
  {
    "data": "abc",
    "description": "an invalid URI though valid URI reference",
    "valid": false
  },
  {
    "data": "http:// shouldfail.com",
    "description": "an invalid URI with spaces",
    "valid": false
  },
  {
    "data": ":// should fail",
    "description": "an invalid URI with spaces and missing scheme",
    "valid": false
  },
  {
    "data": "bar,baz:foo",
    "description": "an invalid URI with comma in scheme",
    "valid": false
  }
]
"""

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "format": "uri"
}
"""

_VALIDATE_FORMATS = False

class Uri0Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any

