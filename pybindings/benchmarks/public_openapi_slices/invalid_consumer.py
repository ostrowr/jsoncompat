from collections.abc import Sequence
from corpus_meilisearch_json_projection import CorpusClient
from corpus_meilisearch_json_projection.operations.indexes import (
    GetdictionaryRequest, GetfacetSearchRequest, PutdictionaryRequest, PutprefixSearchRequest,
)


def invalid(client: CorpusClient) -> None:
    PutdictionaryRequest(index_uid='books', body=42)
    PutdictionaryRequest(index_uid='books', body=[42])
    PutdictionaryRequest(index_uid='books', body=None)
    PutprefixSearchRequest(index_uid='books', body='unknown')
    text: str = client.request(GetfacetSearchRequest(index_uid='books')).unwrap().body
    numbers: Sequence[int] = client.request(GetdictionaryRequest(index_uid='books')).unwrap().body
