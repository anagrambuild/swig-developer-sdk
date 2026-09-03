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


async def test_proxy_exposes_the_ramp_options_contract() -> None:
    requests: list[httpx.Request] = []

    def backend(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        return httpx.Response(
            200,
            json={
                "data": {
                    "countries": [],
                    "fiatCurrencies": [{"currencyCode": "USD", "exponent": 2}],
                    "paymentMethods": ["CARD"],
                    "assets": [
                        {
                            "asset": {"token": {"mint": "MINT"}},
                            "name": "USD Coin",
                            "decimals": 6,
                        }
                    ],
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
        path="/api/swig/ramp/options",
        query={
            "configurationId": "config-123",
            "environment": "sandbox",
            "direction": "buy",
            "countryCode": "US",
        },
    )

    assert response.status == 200
    assert response.body["assets"][0]["asset"] == {"type": "token", "mint": "MINT"}
    assert response.body["assets"][0]["decimals"] == 6
    assert requests[0].url.path == "/wallet/api/ramp/options"
    assert requests[0].url.params["configurationId"] == "config-123"
    assert requests[0].url.params["environment"] == "RAMP_ENVIRONMENT_SANDBOX"
    assert requests[0].url.params["direction"] == "RAMP_DIRECTION_BUY"


async def test_proxy_quotes_returns_an_object_on_success() -> None:
    def backend(request: httpx.Request) -> httpx.Response:
        return httpx.Response(
            200,
            json={
                "data": {
                    "quotes": [
                        {
                            "route": {"provider": "P", "paymentMethod": "CARD"},
                            "buy": {
                                "spend": {"currencyCode": "USD", "minorUnits": "10000"},
                                "receive": {
                                    "asset": {"token": {"mint": "MINT"}},
                                    "baseUnits": "95010000",
                                },
                                "totalFee": {
                                    "currencyCode": "USD",
                                    "minorUnits": "499",
                                },
                                "exchangeRate": "1.053",
                            },
                        }
                    ]
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
        path="/api/swig/ramp/quotes",
        body={
            "configurationId": "config-123",
            "environment": "sandbox",
            "location": {"countryCode": "US"},
            # A browser sends a JSON number, which the SDK's Amount type allows.
            "buy": {
                "spend": {"currencyCode": "USD", "minorUnits": 10000},
                "receive": {"token": {"mint": "MINT"}},
            },
        },
    )

    assert response.status == 200
    assert response.body["quotes"][0]["route"]["provider"] == "P"
    assert response.body["quotes"][0]["details"]["type"] == "buy"


async def test_proxy_creates_an_order() -> None:
    requests: list[httpx.Request] = []

    def backend(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        return httpx.Response(
            200,
            json={
                "data": {
                    "order": {
                        "id": "order-1",
                        "status": "RAMP_ORDER_STATUS_AWAITING_CUSTOMER",
                        "createdAt": "2026-09-01T00:00:00Z",
                        "updatedAt": "2026-09-01T00:00:00Z",
                        "sell": {
                            "quote": {
                                "sell": {
                                    "asset": {"sol": {}},
                                    "baseUnits": "1000000000",
                                },
                                "receive": {
                                    "currencyCode": "USD",
                                    "minorUnits": "15000",
                                },
                                "totalFee": {
                                    "currencyCode": "USD",
                                    "minorUnits": "300",
                                },
                                "exchangeRate": "150.00",
                            }
                        },
                    }
                }
            },
        )

    handler = create_swig_proxy_handler(
        SwigProxyConfig(
            api_key="secret",
            transaction_api_url="https://backend.test",
            network="mainnet",
            transport=httpx.MockTransport(backend),
        )
    )
    response = await handler.handle(
        method="POST",
        path="/api/swig/ramp/orders",
        body={
            "requestId": "request-1",
            "configurationId": "config-123",
            "environment": "sandbox",
            "context": {
                "customerId": "customer-1",
                "swigConfigAddress": "swig-config",
                "location": {"countryCode": "US"},
            },
            "route": {"provider": "P", "paymentMethod": "BANK"},
            "sell": {
                "sell": {"asset": {"sol": {}}, "baseUnits": 1000000000},
                "receiveFiatCurrencyCode": "USD",
            },
        },
    )

    assert response.status == 200
    assert response.body["id"] == "order-1"
    sent = json.loads(requests[0].content)
    # A JSON number is canonicalized, and native SOL keeps its empty message.
    assert sent["sell"]["sell"]["baseUnits"] == "1000000000"
    assert sent["sell"]["sell"]["asset"] == {"sol": {}}


async def test_proxy_submits_a_transfer() -> None:
    requests: list[httpx.Request] = []

    def backend(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        return httpx.Response(
            200,
            json={
                "data": {
                    "transfer": {
                        "transferId": "transfer-1",
                        "state": "TRANSFER_STATE_LANDED",
                        "expiresAt": "2026-09-01T00:01:00Z",
                    }
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
        path="/api/swig/ramp/orders/order-1/transfer/submit",
        body={"transferId": "transfer-1"},
    )

    assert response.status == 200
    assert response.body["state"] == "landed"
    # Omitted bytes resolve an attempt that was already broadcast.
    assert json.loads(requests[0].content)["signedTransaction"] == ""


async def test_proxy_matches_transfer_routes_with_participant_set_authority() -> None:
    requests: list[httpx.Request] = []

    def backend(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        return httpx.Response(
            200,
            json={
                "data": {
                    "preparedTransfer": {
                        "transfer": {
                            "transferId": "transfer-1",
                            "state": "TRANSFER_STATE_PREPARED",
                            "expiresAt": "2026-09-01T00:01:00Z",
                        },
                        "preparedTransaction": {
                            "transaction": "prepared-base64",
                            "transactionEncoding": "TRANSACTION_ENCODING_BASE64",
                            "network": "NETWORK_DEVNET",
                        },
                        "deposit": {
                            "address": "deposit-address",
                            "amount": {
                                "asset": {"sol": {}},
                                "baseUnits": "1000000000",
                            },
                        },
                    }
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
        path="/api/swig/ramp/orders/order%2F123/transfer/prepare",
        body={
            "requesterAuthority": {
                "participantSet": {"address": "participant-set", "roleId": 3}
            },
            "feePayer": "payer",
        },
    )

    assert response.status == 200
    assert (
        requests[0].url.raw_path.decode()
        == "/wallet/api/ramp/orders/order%2F123/transfer/prepare"
    )
    assert json.loads(requests[0].content)["requesterAuthority"] == {
        "participantSet": {
            "participantSetAddress": "participant-set",
            "roleId": 3,
        }
    }
    assert response.body["deposit"]["amount"]["asset"] == {"type": "sol"}


async def test_proxy_preserves_uint64_amounts_as_strings() -> None:
    def backend(request: httpx.Request) -> httpx.Response:
        return httpx.Response(
            200,
            json={
                "data": {
                    "order": {
                        "id": "order-1",
                        "status": "RAMP_ORDER_STATUS_SETTLED",
                        "createdAt": "2026-09-01T00:00:00Z",
                        "updatedAt": "2026-09-01T00:00:00Z",
                        "sell": {
                            "quote": {
                                "sell": {
                                    "asset": {"sol": {}},
                                    "baseUnits": "18446744073709551615",
                                },
                                "receive": {
                                    "currencyCode": "USD",
                                    "minorUnits": "15000",
                                },
                                "totalFee": {
                                    "currencyCode": "USD",
                                    "minorUnits": "300",
                                },
                                "exchangeRate": "150.00",
                            }
                        },
                    }
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
        method="GET", path="/api/swig/ramp/orders/order-1", query={}
    )

    assert response.status == 200
    assert response.body["quote"]["sell"]["baseUnits"] == "18446744073709551615"


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
