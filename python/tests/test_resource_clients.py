from __future__ import annotations

import base64
import json
from collections.abc import Callable

import base58
import httpx
import pytest

from swig_developer_sdk import (
    CryptoAmountInput,
    FiatAmountInput,
    NativeSolAsset,
    RampBuyOrderRequest,
    RampBuyQuote,
    RampLocation,
    RampOrderContext,
    RampRoute,
    RampSellOrder,
    RampSellOrderRequest,
    RampSellQuote,
    SplTokenAsset,
    SponsorSignedTransactionArgs,
    SponsorSignedTransactionBundleArgs,
    SwigClient,
)


async def test_sponsor_converts_base64_transaction_to_base58() -> None:
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        return httpx.Response(
            200,
            json={
                "data": {
                    "request_id": "request-123",
                    "signature": "chain-signature",
                    "spent_by_paymaster": "5000",
                }
            },
        )

    swig = SwigClient(
        api_key="secret",
        base_url="https://example.test",
        network="devnet",
        transport=httpx.MockTransport(handler),
    )
    transaction = b"\x00\x01\x02\xff"
    submitted = await swig.transactions.sponsor(
        SponsorSignedTransactionArgs(
            transaction=base64.b64encode(transaction).decode("ascii"),
            idempotency_key="sponsor-request-123",
        )
    )
    assert submitted.signature == "chain-signature"
    assert submitted.request_id == "request-123"
    assert submitted.spent_by_paymaster == "5000"
    assert json.loads(requests[0].content) == {
        "base58_encoded_transaction": base58.b58encode(transaction).decode("ascii"),
        "network": "devnet",
        "idempotencyKey": "sponsor-request-123",
    }


async def test_sponsor_bundle_converts_transactions_to_base58() -> None:
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        return httpx.Response(
            200,
            json={
                "data": {
                    "request_id": "request-bundle",
                    "bundle_id": "bundle-123",
                    "signatures": ["signature-1"],
                    "estimated_spent_by_paymaster": "9000",
                }
            },
        )

    swig = SwigClient(
        api_key="secret",
        base_url="https://example.test",
        network="mainnet",
        transport=httpx.MockTransport(handler),
    )
    transaction = b"\x00\x01\x02\xff"
    submitted = await swig.transactions.sponsor_bundle(
        SponsorSignedTransactionBundleArgs(
            transactions=(base64.b64encode(transaction).decode("ascii"),),
            idempotency_key="bundle-request-123",
        )
    )
    assert submitted.bundle_id == "bundle-123"
    assert submitted.estimated_spent_by_paymaster == "9000"
    assert json.loads(requests[0].content) == {
        "base58_encoded_transactions": [base58.b58encode(transaction).decode("ascii")],
        "network": "mainnet",
        "idempotencyKey": "bundle-request-123",
    }


BUY_QUOTE: dict[str, object] = {
    "route": {"provider": "PROVIDER", "paymentMethod": "CARD"},
    "buy": {
        "spend": {"currencyCode": "USD", "minorUnits": "10000"},
        "receive": {"asset": {"token": {"mint": "MINT"}}, "baseUnits": "99000000"},
        "totalFee": {"currencyCode": "USD", "minorUnits": "250"},
        "exchangeRate": "0.0000099",
    },
}

SELL_QUOTE: dict[str, object] = {
    "route": {"provider": "PROVIDER", "paymentMethod": "BANK"},
    "sell": {
        "sell": {"asset": {"sol": {}}, "baseUnits": "1000000000"},
        "receive": {"currencyCode": "USD", "minorUnits": "15000"},
        "totalFee": {"currencyCode": "USD", "minorUnits": "300"},
        "exchangeRate": "150.00",
    },
}


def _buy_order(status: str) -> dict[str, object]:
    return {
        "id": "order-1",
        "status": status,
        "createdAt": "2026-09-01T00:00:00Z",
        "updatedAt": "2026-09-01T00:00:00Z",
        "buy": {
            "quote": BUY_QUOTE["buy"],
            "launchUrl": "https://provider.test/session",
        },
    }


def _ramp_client(
    handler: Callable[[httpx.Request], object],
    requests: list[httpx.Request] | None = None,
) -> SwigClient:
    def transport(request: httpx.Request) -> httpx.Response:
        if requests is not None:
            requests.append(request)
        return httpx.Response(200, json={"data": handler(request)})

    return SwigClient(
        api_key="secret",
        base_url="https://example.test",
        network="devnet",
        transport=httpx.MockTransport(transport),
    )


async def test_ramp_options_query_and_four_lists() -> None:
    requests: list[httpx.Request] = []
    swig = _ramp_client(
        lambda request: {
            "countries": [
                {
                    "code": "US",
                    "name": "United States",
                    "subdivisions": [{"code": "US-CA", "name": "California"}],
                }
            ],
            "fiatCurrencies": [{"currencyCode": "USD", "exponent": 2}],
            "paymentMethods": ["CARD"],
            "assets": [
                {
                    "asset": {"token": {"mint": "MINT"}},
                    "name": "USD Coin",
                    "decimals": 6,
                    "iconUrl": "https://example.test/usdc.png",
                },
                {"asset": {"sol": {}}, "name": "Solana", "decimals": 9},
            ],
        },
        requests,
    )

    options = await swig.ramp.get_options(
        configuration_id="018f-config",
        environment="sandbox",
        direction="buy",
        country_code="US",
    )

    assert requests[0].url.path == "/wallet/api/ramp/options"
    assert requests[0].url.params["configurationId"] == "018f-config"
    assert requests[0].url.params["environment"] == "RAMP_ENVIRONMENT_SANDBOX"
    assert requests[0].url.params["direction"] == "RAMP_DIRECTION_BUY"
    assert options.countries[0].subdivisions[0].code == "US-CA"
    assert options.fiat_currencies[0].exponent == 2
    assert options.payment_methods == ("CARD",)
    assert options.assets[0].asset == SplTokenAsset(mint="MINT")
    assert options.assets[0].decimals == 6
    assert options.assets[1].asset == NativeSolAsset()
    assert options.assets[1].icon_url is None


async def test_ramp_quotes_send_a_buy_order_oneof() -> None:
    requests: list[httpx.Request] = []
    swig = _ramp_client(lambda request: {"quotes": [BUY_QUOTE]}, requests)

    quotes = await swig.ramp.get_quotes(
        configuration_id="018f-config",
        environment="sandbox",
        location=RampLocation(country_code="US"),
        order=RampBuyOrderRequest(
            spend=FiatAmountInput(currency_code="USD", minor_units=10000),
            receive=SplTokenAsset(mint="MINT"),
        ),
    )

    assert json.loads(requests[0].content) == {
        "configurationId": "018f-config",
        "environment": "RAMP_ENVIRONMENT_SANDBOX",
        "location": {"countryCode": "US"},
        "buy": {
            "spend": {"currencyCode": "USD", "minorUnits": "10000"},
            "receive": {"token": {"mint": "MINT"}},
        },
    }
    assert quotes[0].details.type == "buy"


async def test_ramp_native_sol_survives_compact() -> None:
    requests: list[httpx.Request] = []
    swig = _ramp_client(lambda request: {"quotes": [SELL_QUOTE]}, requests)

    quotes = await swig.ramp.get_quotes(
        configuration_id="018f-config",
        environment="sandbox",
        location=RampLocation(country_code="US", subdivision_code="US-CA"),
        order=RampSellOrderRequest(
            sell=CryptoAmountInput(asset=NativeSolAsset(), base_units=1000000000),
            receive_fiat_currency_code="USD",
        ),
    )

    body = json.loads(requests[0].content)
    assert body["sell"]["sell"]["asset"] == {"sol": {}}
    assert body["location"] == {"countryCode": "US", "subdivisionCode": "US-CA"}
    details = quotes[0].details
    assert isinstance(details, RampSellQuote)
    assert details.sell.asset == NativeSolAsset()


async def test_ramp_decodes_uint64_above_2_53_without_loss() -> None:
    requests: list[httpx.Request] = []
    buy = dict(BUY_QUOTE["buy"])  # type: ignore[arg-type]
    buy["receive"] = {
        "asset": {"token": {"mint": "MINT"}},
        "baseUnits": "18446744073709551615",
    }
    priced = {**BUY_QUOTE, "buy": buy}

    def handler(request: httpx.Request) -> object:
        if request.url.path.endswith("/quotes"):
            return {"quotes": [priced]}
        return {"order": _buy_order("RAMP_ORDER_STATUS_AWAITING_CUSTOMER")}

    swig = _ramp_client(handler, requests)

    quotes = await swig.ramp.get_quotes(
        configuration_id="018f-config",
        environment="sandbox",
        location=RampLocation(country_code="US"),
        order=RampBuyOrderRequest(
            spend=FiatAmountInput(currency_code="USD", minor_units="10000"),
            receive=SplTokenAsset(mint="MINT"),
        ),
    )
    details = quotes[0].details
    assert isinstance(details, RampBuyQuote)
    assert details.receive.base_units == "18446744073709551615"

    await swig.ramp.create_order(
        request_id="request-1",
        configuration_id="018f-config",
        environment="sandbox",
        context=RampOrderContext(
            customer_id="customer-1",
            swig_config_address="swig-config",
            location=RampLocation(country_code="US"),
        ),
        route=quotes[0].route,
        order=RampSellOrderRequest(
            sell=CryptoAmountInput(
                asset=NativeSolAsset(), base_units=details.receive.base_units
            ),
            receive_fiat_currency_code="USD",
        ),
    )

    body = json.loads(requests[1].content)
    assert body["sell"]["sell"]["baseUnits"] == "18446744073709551615"


async def test_ramp_rejects_a_uint64_that_is_not_a_decimal_string() -> None:
    buy = dict(BUY_QUOTE["buy"])  # type: ignore[arg-type]
    buy["spend"] = {"currencyCode": "USD", "minorUnits": 10000}
    swig = _ramp_client(lambda request: {"quotes": [{**BUY_QUOTE, "buy": buy}]})

    with pytest.raises(ValueError, match="invalid minorUnits"):
        await swig.ramp.get_quotes(
            configuration_id="018f-config",
            environment="sandbox",
            location=RampLocation(country_code="US"),
            order=RampBuyOrderRequest(
                spend=FiatAmountInput(currency_code="USD", minor_units="10000"),
                receive=SplTokenAsset(mint="MINT"),
            ),
        )


async def test_ramp_create_order_unwraps_the_envelope() -> None:
    requests: list[httpx.Request] = []
    swig = _ramp_client(
        lambda request: {"order": _buy_order("RAMP_ORDER_STATUS_AWAITING_CUSTOMER")},
        requests,
    )

    order = await swig.ramp.create_order(
        request_id="request-1",
        configuration_id="018f-config",
        environment="sandbox",
        context=RampOrderContext(
            customer_id="customer-1",
            swig_config_address="swig-config",
            location=RampLocation(country_code="US"),
        ),
        route=RampRoute(provider="PROVIDER", payment_method="CARD"),
        order=RampBuyOrderRequest(
            spend=FiatAmountInput(currency_code="USD", minor_units="10000"),
            receive=SplTokenAsset(mint="MINT"),
        ),
    )

    assert order.type == "buy"
    assert order.status == "awaiting-customer"
    assert order.launch_url == "https://provider.test/session"
    body = json.loads(requests[0].content)
    assert body["requestId"] == "request-1"
    assert body["context"] == {
        "customerId": "customer-1",
        "swigConfigAddress": "swig-config",
        "network": "NETWORK_DEVNET",
        "location": {"countryCode": "US"},
    }


async def test_ramp_missing_order_envelope_is_rejected() -> None:
    swig = _ramp_client(lambda request: {})

    with pytest.raises(ValueError, match="missing order"):
        await swig.ramp.get_order(order_id="order-1")


async def test_ramp_encodes_the_order_id_in_the_path() -> None:
    requests: list[httpx.Request] = []
    swig = _ramp_client(
        lambda request: {"order": _buy_order("RAMP_ORDER_STATUS_SETTLED")}, requests
    )

    await swig.ramp.get_order(order_id="order/123")

    assert (
        requests[0]
        .url.raw_path.decode()
        .endswith("/wallet/api/ramp/orders/order%2F123")
    )


async def test_ramp_rejects_an_unrecognised_status() -> None:
    swig = _ramp_client(
        lambda request: {"order": _buy_order("RAMP_ORDER_STATUS_CHARGEBACK")}
    )

    with pytest.raises(ValueError, match="invalid status"):
        await swig.ramp.get_order(order_id="order-1")


async def test_ramp_decodes_an_unspecified_status() -> None:
    swig = _ramp_client(
        lambda request: {"order": _buy_order("RAMP_ORDER_STATUS_UNSPECIFIED")}
    )

    order = await swig.ramp.get_order(order_id="order-1")

    assert order.status == "unspecified"


async def test_ramp_sell_order_without_deposit_or_transfer() -> None:
    swig = _ramp_client(
        lambda request: {
            "order": {
                "id": "order-1",
                "status": "RAMP_ORDER_STATUS_AWAITING_CUSTOMER",
                "createdAt": "2026-09-01T00:00:00Z",
                "updatedAt": "2026-09-01T00:00:00Z",
                "sell": {"quote": SELL_QUOTE["sell"]},
            }
        }
    )

    order = await swig.ramp.get_order(order_id="order-1")

    assert isinstance(order, RampSellOrder)
    assert order.deposit is None
    assert order.transfer is None


async def test_ramp_prepare_transfer_decodes_the_envelope() -> None:
    requests: list[httpx.Request] = []
    swig = _ramp_client(
        lambda request: {
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
                    "amount": {"asset": {"sol": {}}, "baseUnits": "1000000000"},
                },
            }
        },
        requests,
    )

    prepared = await swig.ramp.prepare_transfer(
        order_id="order-1",
        requester_authority={"ed25519": {"publicKey": "requester"}},
        fee_payer="payer",
    )

    assert requests[0].url.path == "/wallet/api/ramp/orders/order-1/transfer/prepare"
    assert json.loads(requests[0].content) == {
        "requesterAuthority": {"ed25519": {"publicKey": "requester"}},
        "feePayer": "payer",
    }
    assert prepared.transfer.state == "prepared"
    assert prepared.prepared_transaction.transaction == "prepared-base64"
    assert prepared.deposit.amount.asset == NativeSolAsset()


async def test_ramp_submit_sends_an_empty_signed_transaction_by_default() -> None:
    requests: list[httpx.Request] = []
    swig = _ramp_client(
        lambda request: {
            "transfer": {
                "transferId": "transfer-1",
                "state": "TRANSFER_STATE_LANDED",
                "expiresAt": "2026-09-01T00:01:00Z",
                "solanaSignature": "signature",
            }
        },
        requests,
    )

    transfer = await swig.ramp.submit_transfer(
        order_id="order-1", transfer_id="transfer-1"
    )

    assert json.loads(requests[0].content) == {
        "transferId": "transfer-1",
        "signedTransaction": "",
    }
    assert transfer.state == "landed"
    assert transfer.solana_signature == "signature"


async def test_ramp_rejects_an_invalid_environment_at_runtime() -> None:
    swig = _ramp_client(lambda request: {})

    with pytest.raises(ValueError, match="sandbox"):
        await swig.ramp.get_options(
            configuration_id="018f-config",
            environment="staging",  # type: ignore[arg-type]
            direction="buy",
        )


async def test_wallet_reads_preserve_asset_kind() -> None:
    def handler(request: httpx.Request) -> httpx.Response:
        if request.url.path.endswith("/token-balances"):
            return httpx.Response(
                200,
                json={
                    "data": {
                        "swig_config_address": "swig-123",
                        "wallet_address": "wallet-123",
                        "total_usd_value": 1,
                        "balances": [
                            {
                                "mint_address": "mint-123",
                                "token_program": 1,
                                "token_symbol": "USDC",
                                "token_name": "USD Coin",
                                "decimals": 6,
                                "amount_raw": "1000000",
                                "ui_amount": 1,
                                "usd_price": 1,
                                "usd_value": 1,
                                "asset_kind": "ASSET_KIND_TOKEN",
                            }
                        ],
                    }
                },
            )
        return httpx.Response(
            200,
            json={
                "data": {
                    "swig_config_address": "swig-123",
                    "wallet_address": "wallet-123",
                    "transactions": [
                        {
                            "transaction_signature": "signature-123",
                            "slot": "42",
                            "owner_address": "owner-123",
                            "wallet_address": "wallet-123",
                            "is_subaccount": False,
                            "token_account_address": "wallet-123",
                            "mint_address": "11111111111111111111111111111111",
                            "token_program": 0,
                            "direction": 1,
                            "amount_raw": "1000000000",
                            "decimals": 9,
                            "ui_amount": 1,
                            "usd_price": 100,
                            "usd_value": 100,
                            "token_symbol": "SOL",
                            "token_name": "Solana",
                            "asset_kind": 2,
                        }
                    ],
                }
            },
        )

    swig = SwigClient(
        api_key="secret",
        base_url="https://example.test",
        network="devnet",
        transport=httpx.MockTransport(handler),
    )
    wallet = swig.wallets.use("swig-123")

    balances = await wallet.list_token_balances()
    transactions = await wallet.list_token_transactions()

    assert balances.balances[0].asset_kind == "token"
    assert transactions.transactions[0].asset_kind == "native-sol"


async def test_paymaster_idp_balance_uses_typed_query() -> None:
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        return httpx.Response(
            200,
            json={
                "data": {
                    "configured": True,
                    "kind": "PAYMASTER_KIND_IDP",
                    "balance_lamports": "10",
                    "balance_sol": 0.00000001,
                }
            },
        )

    swig = SwigClient(
        api_key="secret",
        base_url="https://example.test",
        network="devnet",
        transport=httpx.MockTransport(handler),
    )
    balance = await swig.paymaster.get_idp_balance()
    assert balance.kind == "idp"
    assert balance.balance_lamports == "10"
    assert requests[0].url.params["kind"] == "PAYMASTER_KIND_IDP"
