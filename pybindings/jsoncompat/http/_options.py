"""Apply per-call wire overrides without normalizing existing URL encoding."""

from __future__ import annotations

import re
from dataclasses import replace
from typing import cast
from urllib.parse import quote, unquote_plus

from ._metadata import EncodedRequest, HeaderPairs, RequestOptions, snapshot_pairs
from ._parameters import check_header


def _transformed(value: object, original: EncodedRequest) -> EncodedRequest:
    if not isinstance(value, EncodedRequest):
        raise TypeError("transform must return EncodedRequest")
    if value.operation is not original.operation:
        raise ValueError("transform must preserve the operation metadata")
    method = cast(object, value.method)
    target = cast(object, value.target)
    body = cast(object, value.body)
    if not isinstance(method, str) or not re.fullmatch(
        r"[!#$%&'*+.^_`|~0-9A-Za-z-]+", method
    ):
        raise ValueError("invalid transformed HTTP method")
    if (
        not isinstance(target, str)
        or not re.fullmatch(r"/[A-Za-z0-9\-._~!$&'()*+,;=:@/%?|]*", target)
        or target.startswith("//")
        or re.search(r"%(?![0-9a-fA-F]{2})", target)
        or any(part in (".", "..") for part in target.partition("?")[0].split("/"))
    ):
        raise ValueError("transform target must be an encoded origin-relative URL")
    if body is not None and not isinstance(body, bytes):
        raise TypeError("transform body must be bytes or None")
    headers = snapshot_pairs(value.headers)
    for name, item in headers:
        check_header(name, item, ascii_only=True)
    return replace(value, headers=headers)


def apply_options(
    request: EncodedRequest, options: RequestOptions | None
) -> EncodedRequest:
    if options is None:
        return request
    if not isinstance(cast(object, options), RequestOptions):
        raise TypeError("options must be RequestOptions or None")
    # RequestOptions snapshots both mappings and sequences into immutable tuples.
    extras = cast(HeaderPairs, options.extra_headers)
    for name, value in extras:
        check_header(name, value, ascii_only=True)
    for name in options.remove_headers:
        check_header(name, "", ascii_only=True)
    removed = {name.lower() for name in options.remove_headers} | {
        name.lower() for name, _ in extras
    }
    headers = (
        tuple(
            (name, value)
            for name, value in request.headers
            if name.lower() not in removed
        )
        + extras
    )
    target = request.target
    if options.remove_query or options.extra_query:
        path, _, query = target.partition("?")
        removed_query = set(options.remove_query)
        parts = [
            part
            for part in (query.split("&") if query else ())
            if unquote_plus(part.partition("=")[0], errors="strict")
            not in removed_query
        ]
        parts.extend(
            f"{quote(name, safe='')}={quote(value, safe='')}"
            for name, value in cast(HeaderPairs, options.extra_query)
        )
        target = path + ("?" + "&".join(parts) if parts else "")
    result = replace(request, target=target, headers=headers)
    if options.transform is not None:
        return _transformed(options.transform(result), request)
    return result
