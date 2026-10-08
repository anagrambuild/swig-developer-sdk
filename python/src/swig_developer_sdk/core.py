from __future__ import annotations

import asyncio
from collections.abc import Mapping
from typing import cast

import httpx

from .common import JsonValue, RetryOptions
from .errors import (
    SwigConnectionError,
    SwigResponseError,
    SwigTimeoutError,
)
from .errors import (
    SwigDeveloperSdkError as SwigDeveloperSdkError,
)


class HttpClient:
    def __init__(
        self,
        *,
        api_key: str,
        base_url: str,
        retry: RetryOptions,
        transport: httpx.AsyncBaseTransport | None = None,
        timeout: float | None = 5.0,
    ) -> None:
        self._retry = retry
        self._client = httpx.AsyncClient(
            base_url=base_url.rstrip("/"),
            headers={
                "Content-Type": "application/json",
                "Authorization": f"Bearer {api_key}",
            },
            transport=transport,
            timeout=timeout,
        )

    async def aclose(self) -> None:
        if not self._client.is_closed:
            await self._client.aclose()

    async def get(self, path: str) -> object:
        return await self._request(path, method="GET")

    async def post(
        self,
        path: str,
        body: Mapping[str, object],
        *,
        retry: bool = False,
    ) -> object:
        compacted = _compact(body)
        if not isinstance(compacted, Mapping):
            raise TypeError("Request body must be an object")
        return await self._request(path, method="POST", body=compacted, retry=retry)

    async def _request(
        self,
        path: str,
        *,
        method: str,
        body: Mapping[str, object] | None = None,
        retry: bool = True,
    ) -> object:
        max_retries = self._retry.max_retries if retry else 0
        for attempt in range(max_retries + 1):
            try:
                response = await self._client.request(method, path, json=body)
            except httpx.DecodingError as error:
                raise SwigResponseError("API response could not be decoded") from error
            except httpx.TimeoutException as error:
                if attempt == max_retries:
                    raise SwigTimeoutError() from error
            except httpx.TransportError as error:
                if attempt == max_retries:
                    raise SwigConnectionError() from error
            else:
                if 200 <= response.status_code < 300:
                    return _unwrap_data(_parse_response_body(response))
                try:
                    response_body = response.json() if response.content else None
                except ValueError:
                    response_body = None
                error_response = SwigDeveloperSdkError.from_response(
                    response, response_body
                )
                if response.status_code < 500 or attempt == max_retries:
                    raise error_response

            await asyncio.sleep(
                self._retry.retry_delay * self._retry.backoff_multiplier**attempt
            )
        raise AssertionError("retry loop must return or raise")


def _parse_response_body(response: httpx.Response) -> object:
    if not response.content:
        return None
    try:
        return cast(JsonValue, response.json())
    except ValueError as error:
        raise SwigResponseError("API response is not valid JSON") from error


def _unwrap_data(body: object) -> object:
    if isinstance(body, Mapping) and "data" in body:
        return body["data"]
    return body


def _compact(value: object) -> object:
    if isinstance(value, Mapping):
        return {
            str(key): _compact(item) for key, item in value.items() if item is not None
        }
    if isinstance(value, list):
        return [_compact(item) for item in value]
    if isinstance(value, tuple):
        return [_compact(item) for item in value]
    return value
