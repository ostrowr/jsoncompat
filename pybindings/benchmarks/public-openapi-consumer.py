"""Concrete body/error inference against the pinned, unchanged public clients."""
from collections.abc import Sequence
from typing import Literal, assert_type

from jsoncompat.http import ClientResponse, Err
from corpus_prefect.api import CorpusClient, CorpusAsyncClient
from corpus_prefect.models.flow import Flow
from corpus_prefect.operations.health import HealthCheckHealthGetRequest
from corpus_prefect.operations.version import ServerVersionVersionGetRequest
from corpus_prefect.operations.flows import CountFlowsFlowsCountPostRequest, ReadFlowsFlowsFilterPostRequest
from corpus_prefect.operations.flow_runs import AverageFlowRunLatenessFlowRunsLatenessPostRequest
from corpus_prefect.operations.templates import ValidateTemplateTemplatesValidatePostRequest
from corpus_prefect.operations.logs import CreateLogsLogsPostRequest
from corpus_airflow_edge.models.http_exception_response import HTTPExceptionResponseBranch1
from corpus_airflow_edge.operations.edge_worker import FetchSuccess, FetchError, Fetch404


def scalar_and_array_bodies(client: CorpusClient) -> None:
    assert_type(client.request(HealthCheckHealthGetRequest()).unwrap().body, Literal[False, True])
    assert_type(client.request(ServerVersionVersionGetRequest()).unwrap().body, str)
    assert_type(client.request(CountFlowsFlowsCountPostRequest()).unwrap().body, int)
    assert_type(client.request(ReadFlowsFlowsFilterPostRequest()).unwrap().body, Sequence[Flow])
    assert_type(client.request(AverageFlowRunLatenessFlowRunsLatenessPostRequest()).unwrap().body, float | None)
    client.request(ValidateTemplateTemplatesValidatePostRequest(body="{{ name }}"))
    client.request(CreateLogsLogsPostRequest(body=[]))


async def async_body(client: CorpusAsyncClient) -> str:
    body = (await client.request(ServerVersionVersionGetRequest())).unwrap().body
    assert_type(body, str)
    return body


def expected_error(response: ClientResponse[FetchSuccess, FetchError]) -> str | None:
    result = response.result()
    if isinstance(result, Err) and isinstance(result.error, Fetch404):
        assert_type(result.error.status, Literal[404])
        detail = result.error.body.detail
        assert_type(detail, HTTPExceptionResponseBranch1 | str)
        return detail if isinstance(detail, str) else str(detail)
    return None
