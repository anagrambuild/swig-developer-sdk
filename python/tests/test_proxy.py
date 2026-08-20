from __future__ import annotations

import json

import httpx
from x402.schemas import PaymentRequired

from swig_developer_sdk import (
    SwigProxyConfig,
    create_swig_proxy_handler,
)


async def test_proxy_prepares_transfer_with_server_resolvers() -> None:
    requests: list[httpx.Request] = []

    def backend(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        return httpx.Response(
            200,
            json={
                "data": {
                    "transaction": "prepared",
                    "transactionEncoding": "TRANSACTION_ENCODING_BASE64",
                    "network": "NETWORK_DEVNET",
                    "signatureRequests": [],
                }
            },
        )

    handler = create_swig_proxy_handler(
        SwigProxyConfig(
            api_key="secret",
            transaction_api_url="https://backend.test",
            network="devnet",
            fee_payer="server-payer",
            resolve_requester_pubkey=lambda _context: "server-user",
            transport=httpx.MockTransport(backend),
        )
    )
    response = await handler.handle(
        method="POST",
        path="/api/swig/transfer/sol",
        body={
            "wallet": {"swigConfigAddress": "swig"},
            "destination": "destination",
            "amount": "42",
        },
    )

    assert response.status == 200
    assert response.body == {
        "prepared": {
            "transaction": "prepared",
            "signatureRequests": [],
            "transactionEncoding": "base64",
            "network": "devnet",
        }
    }
    assert json.loads(requests[0].content) == {
        "network": "NETWORK_DEVNET",
        "feePayer": "server-payer",
        "swigAddress": "swig",
        "requesterAuthority": {"ed25519": {"publicKey": "server-user"}},
        "destination": "destination",
        "lamports": "42",
    }


async def test_proxy_requires_server_api_key() -> None:
    handler = create_swig_proxy_handler(
        SwigProxyConfig(network="devnet", fee_payer="payer")
    )
    response = await handler.handle(
        method="POST",
        path="/api/swig/wallet/create",
        body={
            "initialUser": {"ed25519": {"publicKey": "user"}},
        },
    )
    assert response.status == 500
    assert response.body == {"error": "SWIG_DEVELOPER_API_KEY is required"}


async def test_proxy_rejects_non_positive_amount_before_backend() -> None:
    handler = create_swig_proxy_handler(
        SwigProxyConfig(
            api_key="secret",
            network="devnet",
            fee_payer="payer",
            resolve_requester_pubkey=lambda _context: "user",
        )
    )
    response = await handler.handle(
        method="POST",
        path="/api/swig/transfer/sol",
        body={
            "wallet": {"swigConfigAddress": "swig"},
            "destination": "destination",
            "amount": "0",
        },
    )
    assert response.status == 400
    assert response.body == {"error": "amount must be a positive integer string"}


async def test_proxy_prepares_x402_without_requiring_a_fee_payer() -> None:
    requests: list[httpx.Request] = []

    def backend(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        return httpx.Response(
            200,
            json={
                "data": {
                    "acceptedIndex": 0,
                    "preparedTransaction": {
                        "transaction": "cHJlcGFyZWQ=",
                        "transactionEncoding": "TRANSACTION_ENCODING_BASE64",
                        "network": "NETWORK_DEVNET",
                        "kind": "PREPARED_TRANSACTION_KIND_X402_PAYMENT",
                        "wallet": {
                            "swigConfigAddress": "swig",
                            "walletAddress": "swig-wallet",
                        },
                    },
                }
            },
        )

    payment_required = PaymentRequired.model_validate(
        {
            "x402Version": 2,
            "resource": {"url": "https://resource.example/weather"},
            "accepts": [
                {
                    "scheme": "exact",
                    "network": "solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1",
                    "asset": "mint",
                    "amount": "1000",
                    "payTo": "resource-provider",
                    "maxTimeoutSeconds": 300,
                    "extra": {"feePayer": "facilitator"},
                }
            ],
        }
    )
    handler = create_swig_proxy_handler(
        SwigProxyConfig(
            api_key="secret",
            transaction_api_url="https://backend.test",
            network="devnet",
            resolve_requester_pubkey=lambda _context: "developer",
            transport=httpx.MockTransport(backend),
        )
    )
    response = await handler.handle(
        method="POST",
        path="/api/swig/x402/prepare",
        body={
            "wallet": {"swigConfigAddress": "swig"},
            "paymentRequired": payment_required.model_dump(by_alias=True),
        },
    )

    assert response.status == 200
    assert response.body["prepared"]["acceptedIndex"] == 0
    assert requests[0].url.path == "/transaction/payment/x402/prepare"


async def test_proxy_exposes_current_onramp_options_contract() -> None:
    requests: list[httpx.Request] = []

    def backend(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        return httpx.Response(
            200,
            json={
                "data": {
                    "countries": [],
                    "fiat_currency_codes": ["USD"],
                    "payment_method_types": ["CARD"],
                    "crypto_currency_codes": ["USDC_SOLANA"],
                }
            },
        )

    handler = create_swig_proxy_handler(
        SwigProxyConfig(
            api_key="secret",
            transaction_api_url="https://backend.test",
            transport=httpx.MockTransport(backend),
        )
    )
    response = await handler.handle(
        method="GET",
        path="/api/swig/ramp/onramp/options",
        query={
            "organizationMeldConfigurationId": "config-123",
            "environment": "sandbox",
            "countryCode": "US",
        },
    )

    assert response.status == 200
    assert response.body["cryptoCurrencyCodes"] == ["USDC_SOLANA"]
    assert requests[0].url.path == "/wallet/api/ramp/onramp/options"
    assert requests[0].url.params["organizationMeldConfigurationId"] == "config-123"
    assert requests[0].url.params["environment"] == "MELD_ENVIRONMENT_SANDBOX"


async def test_proxy_exposes_offramp_prepare_contract() -> None:
    requests: list[httpx.Request] = []

    def backend(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        return httpx.Response(
            200,
            json={
                "data": {
                    "authorization_id": "authorization-123",
                    "prepared_transaction": {
                        "transaction": "prepared",
                        "signature_requests": [],
                        "transaction_encoding": "TRANSACTION_ENCODING_BASE64",
                        "network": "NETWORK_MAINNET",
                    },
                    "display": {
                        "source_wallet_address": "source",
                        "destination_wallet_address": "destination",
                        "source_amount": "10",
                        "source_currency_code": "USDC_SOLANA",
                        "destination_amount": "9.75",
                        "destination_currency_code": "USD",
                        "service_provider": "TRANSAK",
                    },
                }
            },
        )

    handler = create_swig_proxy_handler(
        SwigProxyConfig(
            api_key="secret",
            transaction_api_url="https://backend.test",
            transport=httpx.MockTransport(backend),
        )
    )
    response = await handler.handle(
        method="POST",
        path="/api/swig/ramp/offramp/session/session-123/prepare",
        body={
            "requesterAuthority": {"ed25519": {"publicKey": "requester"}},
            "environment": "production",
            "feePayer": "payer",
        },
    )

    assert response.status == 200
    assert response.body["authorizationId"] == "authorization-123"
    assert requests[0].url.path.endswith("/session/session-123/prepare")
    assert json.loads(requests[0].content)["environment"] == (
        "MELD_ENVIRONMENT_PRODUCTION"
    )


async def test_proxy_reads_wallet_roles() -> None:
    requests: list[httpx.Request] = []

    def backend(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        return httpx.Response(
            200,
            json={
                "data": {
                    "swig_config_address": "swig-123",
                    "wallet_address": "wallet-123",
                    "roles": [
                        {
                            "role_id": 1,
                            "authority_type": 1,
                            "authority_value": "authority",
                            "actions": [],
                        }
                    ],
                }
            },
        )

    handler = create_swig_proxy_handler(
        SwigProxyConfig(
            api_key="secret",
            transaction_api_url="https://backend.test",
            network="devnet",
            transport=httpx.MockTransport(backend),
        )
    )
    response = await handler.handle(
        method="GET",
        path="/api/swig/wallet/swig-123/roles",
    )

    assert response.status == 200
    assert response.body["roles"] == [
        {
            "roleId": 1,
            "authorityType": 1,
            "authorityValue": "authority",
            "actions": [],
        }
    ]
    assert requests[0].url.path == "/wallet/swig/swig-123/roles"
    assert requests[0].url.params["network"] == "NETWORK_DEVNET"
