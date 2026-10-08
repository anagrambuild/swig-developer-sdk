from __future__ import annotations

import asyncio
import json
from collections.abc import Mapping

import httpx
import pytest

from swig_developer_sdk import (
    RetryOptions,
    SponsorSignedTransactionArgs,
    SponsorSignedTransactionBundleArgs,
    SwigClient,
    SwigConnectionError,
    SwigDeveloperSdkError,
    SwigProxyConfig,
    SwigResponseError,
    SwigTimeoutError,
    create_swig_proxy_handler,
)
from swig_developer_sdk.common import wallet_authority_to_wire

BALANCE = {"configured": True, "balanceLamports": "5000", "balanceSol": 0.000005}


class OwnedTransport(httpx.AsyncBaseTransport):
    """A transport that stops working when closed, like the real HTTP pool."""

    def __init__(self) -> None:
        self.closes = 0
        self.requests: list[httpx.Request] = []

    async def handle_async_request(self, request: httpx.Request) -> httpx.Response:
        assert self.closes == 0
        self.requests.append(request)
        return httpx.Response(200, json={"data": BALANCE})

    async def aclose(self) -> None:
        self.closes += 1


async def test_client_reuses_transport_and_closes_on_exception() -> None:
    transport = OwnedTransport()
    with pytest.raises(RuntimeError, match="application failed"):
        async with SwigClient(
            api_key="secret",
            transport=transport,
            network="devnet",
            base_url="https://example.test/api/v1",
        ) as client:
            await asyncio.gather(
                client.paymaster.get_balance(), client.paymaster.get_balance()
            )
            assert transport.closes == 0
            assert len(transport.requests) == 2
            assert transport.requests[0].url.path == "/api/v1/paymaster/balance"
            raise RuntimeError("application failed")
    assert transport.closes == 1
    await client.aclose()
    assert transport.closes == 1
    with pytest.raises(RuntimeError, match="closed"):
        await client.paymaster.get_balance()
    assert len(transport.requests) == 2


@pytest.mark.parametrize("timeout", [None, 0.5, 12.0])
async def test_timeout_configuration_reaches_transport(timeout: float | None) -> None:
    transport = OwnedTransport()
    async with SwigClient(
        api_key="secret",
        transport=transport,
        network="devnet",
        timeout=timeout,
    ) as client:
        await client.paymaster.get_balance()
    assert transport.requests[0].extensions["timeout"] == dict.fromkeys(
        ("connect", "read", "write", "pool"), timeout
    )


@pytest.mark.parametrize("timeout", [0, -1, float("nan"), float("inf")])
def test_invalid_timeout_is_rejected(timeout: float) -> None:
    with pytest.raises(ValueError, match="timeout"):
        SwigClient(api_key="secret", timeout=timeout)


@pytest.mark.parametrize(
    ("cause", "expected"),
    [
        (httpx.ConnectError("sensitive transport text"), SwigConnectionError),
        (httpx.ReadTimeout("sensitive timeout text"), SwigTimeoutError),
    ],
)
async def test_transport_errors_retry_and_preserve_cause(
    cause: httpx.TransportError,
    expected: type[SwigDeveloperSdkError],
) -> None:
    attempts = 0

    def handler(request: httpx.Request) -> httpx.Response:
        nonlocal attempts
        attempts += 1
        raise cause

    async with SwigClient(
        api_key="secret",
        transport=httpx.MockTransport(handler),
        network="devnet",
        retry_options=RetryOptions(max_retries=1, retry_delay=0),
    ) as client:
        with pytest.raises(expected) as raised:
            await client.paymaster.get_balance()
    assert raised.value.__cause__ is cause
    assert "sensitive" not in str(raised.value)
    assert attempts == 2


@pytest.mark.parametrize("cause", [TypeError("bad callback"), asyncio.CancelledError()])
async def test_programming_errors_and_cancellation_propagate(
    cause: BaseException,
) -> None:
    attempts = 0

    def handler(request: httpx.Request) -> httpx.Response:
        nonlocal attempts
        attempts += 1
        raise cause

    async with SwigClient(
        api_key="secret",
        transport=httpx.MockTransport(handler),
        network="devnet",
        retry_options=RetryOptions(max_retries=2, retry_delay=0),
    ) as client:
        with pytest.raises(type(cause)) as raised:
            await client.paymaster.get_balance()
    assert raised.value is cause
    assert attempts == 1


@pytest.mark.parametrize(
    "response",
    [httpx.Response(200, text="not json"), httpx.Response(200, json={"data": []})],
)
async def test_malformed_response_uses_sdk_error_without_retry(
    response: httpx.Response,
) -> None:
    attempts = 0

    def handler(request: httpx.Request) -> httpx.Response:
        nonlocal attempts
        attempts += 1
        return response

    async with SwigClient(
        api_key="secret",
        transport=httpx.MockTransport(handler),
        network="devnet",
        retry_options=RetryOptions(max_retries=2, retry_delay=0),
    ) as client:
        with pytest.raises(SwigDeveloperSdkError) as raised:
            await client.paymaster.get_balance()
    assert isinstance(raised.value, SwigResponseError)
    assert isinstance(raised.value, ValueError)
    assert raised.value.code == "INVALID_RESPONSE"
    assert raised.value.details is None
    assert attempts == 1


@pytest.mark.parametrize(
    "body",
    [
        "sensitive raw body",
        {"private": "sensitive"},
        {"error": {"message": "invalid", "private": "secret"}},
    ],
)
async def test_errors_do_not_expose_raw_response_details(body: object) -> None:
    response = (
        httpx.Response(400, json=body)
        if not isinstance(body, str)
        else httpx.Response(400, text=body)
    )
    async with SwigClient(
        api_key="secret",
        network="devnet",
        transport=httpx.MockTransport(lambda request: response),
    ) as client:
        with pytest.raises(SwigDeveloperSdkError) as raised:
            await client.paymaster.get_balance()
    assert raised.value.details is None
    assert "sensitive" not in str(raised.value)


@pytest.mark.parametrize(
    ("authority", "expected"),
    [
        ({"ed25519": {"public_key": "key"}}, {"ed25519": {"publicKey": "key"}}),
        ({"secp256k1": {"publicKey": "key"}}, {"secp256k1": {"publicKey": "key"}}),
        (
            {"program_exec_proof": {"role_id": 1, "zk_proof": "proof"}},
            {"programExecProof": {"roleId": 1, "zkProof": "proof"}},
        ),
        (
            {"participant_set": {"address": "set"}},
            {"participantSet": {"participantSetAddress": "set"}},
        ),
        (
            {"participantSet": {"participantSetAddress": "set", "roleId": 0}},
            {"participantSet": {"participantSetAddress": "set", "roleId": 0}},
        ),
    ],
)
def test_authority_aliases(authority: Mapping[str, object], expected: object) -> None:
    assert wallet_authority_to_wire(authority) == expected


@pytest.mark.parametrize(
    "authority",
    [
        {},
        {"ed25519": {"public_key": "one"}, "secp256k1": {"public_key": "two"}},
        {"ed25519": {"public_key": "one", "publicKey": "two"}},
        {"ed25519": {"publik_key": "key"}},
        {"participant_set": {"address": "set", "role_id": True}},
        {"program_exec_proof": {"role_id": "1", "zk_proof": "proof"}},
    ],
)
def test_invalid_authorities_are_rejected(authority: Mapping[str, object]) -> None:
    with pytest.raises(ValueError):
        wallet_authority_to_wire(authority)


async def test_keyword_and_legacy_sponsorship_have_the_same_wire_contract() -> None:
    bodies: list[object] = []

    def handler(request: httpx.Request) -> httpx.Response:
        bodies.append(json.loads(request.content))
        return httpx.Response(
            200,
            json={
                "data": {
                    "request_id": "request",
                    "signature": "signature",
                    "spent_by_paymaster": "5",
                    "bundle_id": "bundle",
                    "signatures": ["first", "second"],
                    "estimated_spent_by_paymaster": "10",
                }
            },
        )

    async with SwigClient(
        api_key="secret",
        network="mainnet",
        transport=httpx.MockTransport(handler),
    ) as client:
        modern = await client.transactions.sponsor(
            transaction="YQ==", network="devnet", idempotency_key="one"
        )
        legacy = await client.transactions.sponsor(
            SponsorSignedTransactionArgs(
                transaction="YQ==", network="devnet", idempotency_key="one"
            )
        )
        modern_bundle = await client.transactions.sponsor_bundle(
            transactions=["YQ==", "Yg=="], idempotency_key="two"
        )
        legacy_bundle = await client.transactions.sponsor_bundle(
            SponsorSignedTransactionBundleArgs(
                transactions=("YQ==", "Yg=="), idempotency_key="two"
            )
        )
    assert modern == legacy
    assert modern_bundle == legacy_bundle
    assert modern_bundle.signatures == ("first", "second")
    assert (
        bodies[0]
        == bodies[1]
        == {
            "base58_encoded_transaction": "2g",
            "network": "devnet",
            "idempotencyKey": "one",
        }
    )
    assert (
        bodies[2]
        == bodies[3]
        == {
            "base58_encoded_transactions": ["2g", "2h"],
            "network": "mainnet",
            "idempotencyKey": "two",
        }
    )


async def test_sponsorship_rejects_mixed_or_missing_arguments_before_io() -> None:
    transport = OwnedTransport()
    async with SwigClient(api_key="secret", transport=transport) as client:
        with pytest.raises(TypeError, match="either"):
            await client.transactions.sponsor(
                SponsorSignedTransactionArgs(transaction="YQ=="), transaction="Yg=="
            )
        with pytest.raises(TypeError, match="either"):
            await client.transactions.sponsor_bundle(
                SponsorSignedTransactionBundleArgs(transactions=("YQ==",)),
                network="mainnet",
            )
        with pytest.raises(TypeError, match="transaction is required"):
            await client.transactions.sponsor()
        with pytest.raises(TypeError, match="transactions is required"):
            await client.transactions.sponsor_bundle()
        with pytest.raises(TypeError, match="sequence"):
            await client.transactions.sponsor_bundle(transactions="YQ==")
    assert transport.requests == []


async def test_proxy_reuses_and_closes_client_without_sticky_network() -> None:
    transport = OwnedTransport()
    async with create_swig_proxy_handler(
        SwigProxyConfig(
            api_key="secret",
            network="devnet",
            transport=transport,
        )
    ) as handler:
        for network in ("mainnet", "devnet"):
            response = await handler.handle(
                method="GET", path="/paymaster/balance", query={"network": network}
            )
            assert response.status == 200
        assert transport.closes == 0
    assert [request.url.params["network"] for request in transport.requests] == [
        "mainnet",
        "devnet",
    ]
    await handler.aclose()
    assert transport.closes == 1


async def test_proxy_maps_invalid_upstream_response_to_502() -> None:
    async with create_swig_proxy_handler(
        SwigProxyConfig(
            api_key="secret",
            network="devnet",
            transport=httpx.MockTransport(
                lambda request: httpx.Response(200, json={"data": []})
            ),
        )
    ) as handler:
        response = await handler.handle(method="GET", path="/paymaster/balance")
    assert response.status == 502


async def test_proxy_closes_unused_owned_transport_once() -> None:
    transport = OwnedTransport()
    async with create_swig_proxy_handler(
        SwigProxyConfig(transport=transport)
    ) as handler:
        pass
    await handler.aclose()
    assert transport.closes == 1
    assert transport.requests == []


@pytest.mark.parametrize("idempotency_key", [None, "stable-key"])
@pytest.mark.parametrize("bundle", [False, True])
async def test_sponsorship_retries_only_with_idempotency(
    idempotency_key: str | None,
    bundle: bool,
) -> None:
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        if len(requests) == 1:
            return httpx.Response(503)
        return httpx.Response(
            200,
            json={
                "request_id": "request",
                "signature": "sig",
                "spent_by_paymaster": "5",
                "bundle_id": "bundle",
                "signatures": ["sig"],
                "estimated_spent_by_paymaster": "5",
            },
        )

    async with SwigClient(
        api_key="secret",
        network="mainnet",
        transport=httpx.MockTransport(handler),
        retry_options=RetryOptions(max_retries=1, retry_delay=0),
    ) as client:
        operation = (
            client.transactions.sponsor_bundle(
                transactions=["YQ=="], idempotency_key=idempotency_key
            )
            if bundle
            else client.transactions.sponsor(
                transaction="YQ==", idempotency_key=idempotency_key
            )
        )
        if idempotency_key is None:
            with pytest.raises(SwigDeveloperSdkError):
                await operation
        else:
            assert (await operation).request_id == "request"
            assert requests[0].content == requests[1].content
    assert len(requests) == (1 if idempotency_key is None else 2)
