"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "format": "idn-hostname"
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
    "data": "실례.테스트",
    "description": "a valid host name (example.test in Hangul)",
    "valid": true
  },
  {
    "data": "〮실례.테스트",
    "description": "illegal first char U+302E Hangul single dot tone mark",
    "valid": false
  },
  {
    "data": "실〮례.테스트",
    "description": "contains illegal char U+302E Hangul single dot tone mark",
    "valid": false
  },
  {
    "data": "실실실실실실실실실실실실실실실실실실실실실실실실실실실실실실실실실실실실실실실실실실실실실실실실실실실실례례테스트례례례례례례례례례례례례례례례례례테스트례례례례례례례례례례례례례례례례례례례테스트례례례례례례례례례례례례테스트례례실례.테스트",
    "description": "a host name with a component too long",
    "valid": false
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5890#section-2.3.2.1 https://tools.ietf.org/html/rfc5891#section-4.4 https://tools.ietf.org/html/rfc3492#section-7.1",
    "data": "-> $1.00 <--",
    "description": "invalid label, correct Punycode",
    "valid": false
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5890#section-2.3.2.1 https://tools.ietf.org/html/rfc5891#section-4.4",
    "data": "xn--ihqwcrb4cv8a8dqg056pqjye",
    "description": "valid Chinese Punycode",
    "valid": true
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.4 https://tools.ietf.org/html/rfc5890#section-2.3.2.1",
    "data": "xn--X",
    "description": "invalid Punycode",
    "valid": false
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.3.1 https://tools.ietf.org/html/rfc5890#section-2.3.2.1",
    "data": "XN--aa---o47jg78q",
    "description": "U-label contains \"--\" in the 3rd and 4th position",
    "valid": false
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.3.1",
    "data": "-hello",
    "description": "U-label starts with a dash",
    "valid": false
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.3.1",
    "data": "hello-",
    "description": "U-label ends with a dash",
    "valid": false
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.3.1",
    "data": "-hello-",
    "description": "U-label starts and ends with a dash",
    "valid": false
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.3.2",
    "data": "ःhello",
    "description": "Begins with a Spacing Combining Mark",
    "valid": false
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.3.2",
    "data": "̀hello",
    "description": "Begins with a Nonspacing Mark",
    "valid": false
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.3.2",
    "data": "҈hello",
    "description": "Begins with an Enclosing Mark",
    "valid": false
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.2 https://tools.ietf.org/html/rfc5892#section-2.6",
    "data": "ßς་〇",
    "description": "Exceptions that are PVALID, left-to-right chars",
    "valid": true
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.2 https://tools.ietf.org/html/rfc5892#section-2.6",
    "data": "۽۾",
    "description": "Exceptions that are PVALID, right-to-left chars",
    "valid": true
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.2 https://tools.ietf.org/html/rfc5892#section-2.6",
    "data": "ـߺ",
    "description": "Exceptions that are DISALLOWED, right-to-left chars",
    "valid": false
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.2 https://tools.ietf.org/html/rfc5892#section-2.6 Note: The two combining marks (U+302E and U+302F) are in the middle and not at the start",
    "data": "〱〲〳〴〵〮〯〻",
    "description": "Exceptions that are DISALLOWED, left-to-right chars",
    "valid": false
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.3.3 https://tools.ietf.org/html/rfc5892#appendix-A.3",
    "data": "a·l",
    "description": "MIDDLE DOT with no preceding 'l'",
    "valid": false
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.3.3 https://tools.ietf.org/html/rfc5892#appendix-A.3",
    "data": "·l",
    "description": "MIDDLE DOT with nothing preceding",
    "valid": false
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.3.3 https://tools.ietf.org/html/rfc5892#appendix-A.3",
    "data": "l·a",
    "description": "MIDDLE DOT with no following 'l'",
    "valid": false
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.3.3 https://tools.ietf.org/html/rfc5892#appendix-A.3",
    "data": "l·",
    "description": "MIDDLE DOT with nothing following",
    "valid": false
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.3.3 https://tools.ietf.org/html/rfc5892#appendix-A.3",
    "data": "l·l",
    "description": "MIDDLE DOT with surrounding 'l's",
    "valid": true
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.3.3 https://tools.ietf.org/html/rfc5892#appendix-A.4",
    "data": "α͵S",
    "description": "Greek KERAIA not followed by Greek",
    "valid": false
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.3.3 https://tools.ietf.org/html/rfc5892#appendix-A.4",
    "data": "α͵",
    "description": "Greek KERAIA not followed by anything",
    "valid": false
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.3.3 https://tools.ietf.org/html/rfc5892#appendix-A.4",
    "data": "α͵β",
    "description": "Greek KERAIA followed by Greek",
    "valid": true
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.3.3 https://tools.ietf.org/html/rfc5892#appendix-A.5",
    "data": "A׳ב",
    "description": "Hebrew GERESH not preceded by Hebrew",
    "valid": false
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.3.3 https://tools.ietf.org/html/rfc5892#appendix-A.5",
    "data": "׳ב",
    "description": "Hebrew GERESH not preceded by anything",
    "valid": false
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.3.3 https://tools.ietf.org/html/rfc5892#appendix-A.5",
    "data": "א׳ב",
    "description": "Hebrew GERESH preceded by Hebrew",
    "valid": true
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.3.3 https://tools.ietf.org/html/rfc5892#appendix-A.6",
    "data": "A״ב",
    "description": "Hebrew GERSHAYIM not preceded by Hebrew",
    "valid": false
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.3.3 https://tools.ietf.org/html/rfc5892#appendix-A.6",
    "data": "״ב",
    "description": "Hebrew GERSHAYIM not preceded by anything",
    "valid": false
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.3.3 https://tools.ietf.org/html/rfc5892#appendix-A.6",
    "data": "א״ב",
    "description": "Hebrew GERSHAYIM preceded by Hebrew",
    "valid": true
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.3.3 https://tools.ietf.org/html/rfc5892#appendix-A.7",
    "data": "def・abc",
    "description": "KATAKANA MIDDLE DOT with no Hiragana, Katakana, or Han",
    "valid": false
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.3.3 https://tools.ietf.org/html/rfc5892#appendix-A.7",
    "data": "・",
    "description": "KATAKANA MIDDLE DOT with no other characters",
    "valid": false
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.3.3 https://tools.ietf.org/html/rfc5892#appendix-A.7",
    "data": "・ぁ",
    "description": "KATAKANA MIDDLE DOT with Hiragana",
    "valid": true
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.3.3 https://tools.ietf.org/html/rfc5892#appendix-A.7",
    "data": "・ァ",
    "description": "KATAKANA MIDDLE DOT with Katakana",
    "valid": true
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.3.3 https://tools.ietf.org/html/rfc5892#appendix-A.7",
    "data": "・丈",
    "description": "KATAKANA MIDDLE DOT with Han",
    "valid": true
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.3.3 https://tools.ietf.org/html/rfc5892#appendix-A.8",
    "data": "ب٠۰",
    "description": "Arabic-Indic digits mixed with Extended Arabic-Indic digits",
    "valid": false
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.3.3 https://tools.ietf.org/html/rfc5892#appendix-A.8",
    "data": "ب٠ب",
    "description": "Arabic-Indic digits not mixed with Extended Arabic-Indic digits",
    "valid": true
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.3.3 https://tools.ietf.org/html/rfc5892#appendix-A.9",
    "data": "۰0",
    "description": "Extended Arabic-Indic digits not mixed with Arabic-Indic digits",
    "valid": true
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.3.3 https://tools.ietf.org/html/rfc5892#appendix-A.2 https://www.unicode.org/review/pr-37.pdf",
    "data": "क‍ष",
    "description": "ZERO WIDTH JOINER not preceded by Virama",
    "valid": false
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.3.3 https://tools.ietf.org/html/rfc5892#appendix-A.2 https://www.unicode.org/review/pr-37.pdf",
    "data": "‍ष",
    "description": "ZERO WIDTH JOINER not preceded by anything",
    "valid": false
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.3.3 https://tools.ietf.org/html/rfc5892#appendix-A.2 https://www.unicode.org/review/pr-37.pdf",
    "data": "क्‍ष",
    "description": "ZERO WIDTH JOINER preceded by Virama",
    "valid": true
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.3.3 https://tools.ietf.org/html/rfc5892#appendix-A.1",
    "data": "क्‌ष",
    "description": "ZERO WIDTH NON-JOINER preceded by Virama",
    "valid": true
  },
  {
    "comment": "https://tools.ietf.org/html/rfc5891#section-4.2.3.3 https://tools.ietf.org/html/rfc5892#appendix-A.1 https://www.w3.org/TR/alreq/#h_disjoining_enforcement",
    "data": "بي‌بي",
    "description": "ZERO WIDTH NON-JOINER not preceded by Virama but matches regexp",
    "valid": true
  },
  {
    "data": "hostname",
    "description": "single label",
    "valid": true
  },
  {
    "data": "host-name",
    "description": "single label with hyphen",
    "valid": true
  },
  {
    "data": "h0stn4me",
    "description": "single label with digits",
    "valid": true
  },
  {
    "data": "1host",
    "description": "single label starting with digit",
    "valid": true
  },
  {
    "data": "hostnam3",
    "description": "single label ending with digit",
    "valid": true
  },
  {
    "data": "",
    "description": "empty string",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Idnhostname0Serializer(SerializerRootModel):
    root: Any

