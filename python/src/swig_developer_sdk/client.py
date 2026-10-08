from __future__ import annotations

import math
from types import TracebackType

import httpx

from .common import DEFAULT_BACKEND_URL, Network, RetryOptions
from .core import HttpClient
from .participant_sets import ParticipantSetsClient
from .paymaster import PaymasterClient
from .ramp import RampClient
from .transactions import TransactionsClient
from .wallets import WalletsClient


class SwigClient:
    """Async client for hosted wallet operations.

    Reuse one instance within an event loop. Use ``async with`` or await
    ``aclose()`` at application shutdown. The client owns an injected transport
    and closes it with the client; do not share that transport across clients.
    Network defaults may be overridden on a wallet handle or an operation.
    ``timeout`` is the HTTP inactivity timeout in seconds (default 5); None
    disables it. It is not an overall operation or retry deadline.
    """

    def __init__(
        self,
        *,
        api_key: str,
        base_url: str = DEFAULT_BACKEND_URL,
        network: Network | None = None,
        retry_options: RetryOptions | None = None,
        transport: httpx.AsyncBaseTransport | None = None,
        timeout: float | None = 5.0,
    ) -> None:
        if timeout is not None and (not math.isfinite(timeout) or timeout <= 0):
            raise ValueError("timeout must be positive and finite, or None")
        self.default_network = network
        self._http = http = HttpClient(
            api_key=api_key,
            base_url=base_url,
            retry=retry_options or RetryOptions(),
            transport=transport,
            timeout=timeout,
        )
        self.paymaster = PaymasterClient(http, network)
        self.participant_sets = ParticipantSetsClient(http, network)
        self.ramp = RampClient(http, network)
        self.transactions = TransactionsClient(http, network)
        self.wallets = WalletsClient(http, network)

    async def aclose(self) -> None:
        """Close connections and the owned transport; repeated calls are safe.

        The client and its wallet handles cannot be reused after closing.
        """
        await self._http.aclose()

    async def __aenter__(self) -> SwigClient:
        return self

    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc: BaseException | None,
        traceback: TracebackType | None,
    ) -> None:
        await self.aclose()


SwigServerClient = SwigClient
