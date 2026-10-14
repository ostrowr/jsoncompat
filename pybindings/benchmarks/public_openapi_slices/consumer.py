from collections.abc import Sequence
from typing import Literal, assert_type
from jsoncompat.http import Err
from jsoncompat.codegen.dataclasses import MissingType
from corpus_meilisearch_json_projection import CorpusClient as MeiliClient
from corpus_meilisearch_json_projection import CorpusAsyncClient as MeiliAsyncClient
from corpus_meilisearch_json_projection.operations.indexes import (
    GetdictionaryRequest,
    Getdictionary404,
    GetfacetSearchRequest,
    GetsearchCutoffMsRequest,
    GetprefixSearchRequest,
    PutdictionaryRequest,
)
from corpus_aivisspeech_json_projection import CorpusClient as SpeechClient
from corpus_aivisspeech_json_projection.models.accent_phrase import AccentPhrase
from corpus_aivisspeech_json_projection.operations.version import VersionRequest
from corpus_aivisspeech_json_projection.operations.core_versions import CoreVersionsRequest
from corpus_aivisspeech_json_projection.operations.mora_data import MoraDataRequest
from corpus_aivisspeech_json_projection.operations.initialize_speaker import InitializeSpeakerRequest
from corpus_aivisspeech_json_projection.operations.validate_kana import ValidateKanaRequest, ValidateKana400


def scalar_bodies(client: MeiliClient) -> None:
    assert_type(client.request(GetfacetSearchRequest(index_uid='books')).unwrap().body, bool)
    assert_type(client.request(GetsearchCutoffMsRequest(index_uid='books')).unwrap().body, int)
    assert_type(client.request(GetprefixSearchRequest(index_uid='books')).unwrap().body,
                Literal['indexingTime', 'disabled'])
    assert_type(client.request(GetdictionaryRequest(index_uid='books')).unwrap().body,
                Sequence[str])
    client.request(PutdictionaryRequest(index_uid='books', body=['red', 'green'])).unwrap()


async def async_array_body(client: MeiliAsyncClient) -> Sequence[str]:
    return (await client.request(GetdictionaryRequest(index_uid='books'))).unwrap().body


def declared_not_found(client: MeiliClient) -> str:
    result = client.request(GetdictionaryRequest(index_uid='missing')).result()
    if isinstance(result, Err) and isinstance(result.error, Getdictionary404):
        return result.error.body.message
    return ''


def speech_bodies(client: SpeechClient) -> None:
    assert_type(client.request(VersionRequest()).unwrap().body, str)
    assert_type(client.request(CoreVersionsRequest()).unwrap().body, Sequence[str])
    assert_type(client.request(ValidateKanaRequest(text='ア')).unwrap().body, bool)
    assert_type(client.request(InitializeSpeakerRequest(speaker=1)).unwrap().body, None)
    phrase = AccentPhrase(moras=[], accent=1)
    assert_type(client.request(MoraDataRequest(speaker=1, body=[phrase])).unwrap().body,
                Sequence[AccentPhrase])
    result = client.request(ValidateKanaRequest(text='?')).result()
    if isinstance(result, Err) and isinstance(result.error, ValidateKana400):
        assert_type(result.error.body.error_name, str)
        assert_type(result.error.body.error_args.get_additional_property("text"), str | MissingType)
