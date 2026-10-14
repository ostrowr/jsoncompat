"""Exact public body types for the new buffered media codecs."""
from typing import assert_type

from corpus_typesense_cloud.api import CorpusClient as TypesenseCloudClient
from corpus_typesense_cloud.operations.invoices import GetInvoicePdfRequest

def check_typesense_cloud(client: TypesenseCloudClient, request: GetInvoicePdfRequest) -> bytes:
    body = client.request(request).unwrap().body
    assert_type(body, bytes)
    return body

from corpus_agent_protocol.api import CorpusClient as AgentProtocolClient
from corpus_agent_protocol.operations.runs import CreateAndStreamRunRequest

def check_agent_protocol(client: AgentProtocolClient, request: CreateAndStreamRunRequest) -> str:
    body = client.request(request).unwrap().body
    assert_type(body, str)
    return body

from corpus_optimade.api import CorpusClient as OptimadeClient
from corpus_optimade.operations.versions import GetVersionsVersionsGetRequest

def check_optimade(client: OptimadeClient, request: GetVersionsVersionsGetRequest) -> str:
    body = client.request(request).unwrap().body
    assert_type(body, str)
    return body
