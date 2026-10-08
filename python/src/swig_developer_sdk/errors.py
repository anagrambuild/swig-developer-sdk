"""Stable errors raised by the hosted Swig API client."""

from __future__ import annotations

from collections.abc import Mapping

import httpx


class SwigDeveloperSdkError(Exception):
    def __init__(
        self,
        message: str,
        code: str,
        status_code: int,
        details: object | None = None,
    ) -> None:
        super().__init__(message)
        self.code = code
        self.status_code = status_code
        self.details = details

    @classmethod
    def from_response(
        cls,
        response: httpx.Response,
        body: object | None,
    ) -> SwigDeveloperSdkError:
        if isinstance(body, Mapping):
            nested = body.get("error")
            if isinstance(nested, Mapping):
                details = nested.get("details")
                return cls(
                    _optional_string(nested.get("message"))
                    or f"Request failed with status {response.status_code}",
                    _optional_string(nested.get("code"))
                    or f"HTTP_{response.status_code}",
                    response.status_code,
                    details,
                )
            details = body.get("details")
            return cls(
                _optional_string(body.get("message"))
                or f"Request failed with status {response.status_code}",
                _optional_string(body.get("code")) or f"HTTP_{response.status_code}",
                response.status_code,
                details,
            )
        return cls(
            f"Request failed with status {response.status_code}",
            f"HTTP_{response.status_code}",
            response.status_code,
            None,
        )


class SwigConnectionError(SwigDeveloperSdkError):
    """The API could not be reached; the original transport error is the cause."""

    def __init__(self, message: str = "Could not connect to the Swig API") -> None:
        super().__init__(message, "NETWORK_ERROR", 0)


class SwigTimeoutError(SwigConnectionError):
    """An HTTP connect, read, write, or pool timeout expired."""

    def __init__(self) -> None:
        super().__init__("The Swig API request timed out")
        self.code = "TIMEOUT_ERROR"


class SwigResponseError(SwigDeveloperSdkError, ValueError):
    """A successful API response violates the expected response contract.

    Also inherits ValueError to preserve existing response-validation catches.
    The status is 502 so framework adapters can distinguish an upstream contract
    failure from invalid caller input.
    """

    def __init__(self, message: str) -> None:
        super().__init__(message, "INVALID_RESPONSE", 502)


def _optional_string(value: object) -> str | None:
    return value if isinstance(value, str) else None
