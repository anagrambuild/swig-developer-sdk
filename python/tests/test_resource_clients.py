from __future__ import annotations

import base64
import json

import base58
import httpx

from swig_developer_sdk import (
    QuoteRampArgs,
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


async def test_onramp_contract_uses_current_routes_and_fields() -> None:
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        if request.url.path.endswith("/options"):
            return httpx.Response(
                200,
                json={
                    "data": {
                        "countries": [
                            {
                                "country_code": "US",
                                "country_name": "United States",
                                "subdivisions": [
                                    {
                                        "subdivision_code": "US-CA",
                                        "subdivision_name": "California",
                                    }
                                ],
                            }
                        ],
                        "fiat_currency_codes": ["USD"],
                        "payment_method_types": ["CARD"],
                        "crypto_currency_codes": ["USDC_SOLANA"],
                    }
                },
            )
        return httpx.Response(
            200,
            json={
                "data": {
                    "quotes": [
                        {
                            "quote_id": "quote-123",
                            "service_provider": "TRANSAK",
                            "payment_method_type": "CARD",
                            "source_amount": "100",
                            "source_currency_code": "USD",
                            "destination_amount": "99",
                            "destination_currency_code": "USDC_SOLANA",
                            "exchange_rate": "0.99",
                            "total_fee": "1",
                        }
                    ]
                }
            },
        )

    swig = SwigClient(
        api_key="secret",
        base_url="https://example.test",
        network="devnet",
        transport=httpx.MockTransport(handler),
    )
    options = await swig.ramp.onramp.get_options(
        organization_meld_configuration_id="config-123",
        environment="sandbox",
        country_code="US",
    )
    quote = await swig.ramp.onramp.quote(
        QuoteRampArgs(
            organization_meld_configuration_id="config-123",
            environment="sandbox",
            external_customer_id="customer-123",
            swig_config_address="swig-123",
            source_amount="100",
            source_currency_code="USD",
            destination_currency_code="USDC_SOLANA",
            country_code="US",
        )
    )

    assert options.countries[0].subdivisions[0].subdivision_code == "US-CA"
    assert options.crypto_currency_codes == ("USDC_SOLANA",)
    assert quote.quotes[0].quote_id == "quote-123"
    assert requests[0].url.path == "/wallet/api/ramp/onramp/options"
    assert requests[0].url.params["organizationMeldConfigurationId"] == "config-123"
    assert requests[0].url.params["environment"] == "MELD_ENVIRONMENT_SANDBOX"
    assert requests[1].url.path == "/wallet/api/ramp/onramp/quote"
    assert json.loads(requests[1].content)["network"] == "NETWORK_DEVNET"


async def test_ramp_rejects_an_invalid_environment_at_runtime() -> None:
    swig = SwigClient(
        api_key="secret",
        base_url="https://example.test",
        transport=httpx.MockTransport(lambda _: httpx.Response(200, json={})),
    )

    try:
        await swig.ramp.onramp.get_options(
            organization_meld_configuration_id="config-123",
            environment="staging",  # type: ignore[arg-type]
        )
    except ValueError as error:
        assert str(error) == 'environment must be "sandbox" or "production"'
    else:
        raise AssertionError("invalid ramp environment was accepted")


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


async def test_offramp_prepare_and_submit_contract() -> None:
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        if request.url.path.endswith("/prepare"):
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
                            "source_wallet_address": "source-wallet",
                            "destination_wallet_address": "destination-wallet",
                            "source_amount": "10",
                            "source_currency_code": "USDC_SOLANA",
                            "destination_amount": "9.75",
                            "destination_currency_code": "USD",
                            "service_provider": "TRANSAK",
                        },
                    }
                },
            )
        return httpx.Response(
            200,
            json={"data": {"solana_signature": "solana-signature"}},
        )

    swig = SwigClient(
        api_key="secret",
        base_url="https://example.test",
        network="mainnet",
        transport=httpx.MockTransport(handler),
    )
    prepared = await swig.ramp.offramp.prepare_authorization(
        session_id="session/123",
        requester_authority={"ed25519": {"publicKey": "requester"}},
        environment="production",
        fee_payer="payer",
    )
    submitted = await swig.ramp.offramp.submit_authorization(
        session_id="session/123",
        authorization_id=prepared.authorization_id,
        signed_transaction="signed-base64",
        environment="production",
    )

    assert prepared.prepared_transaction.transaction == "prepared"
    assert prepared.display.destination_wallet_address == "destination-wallet"
    assert submitted.solana_signature == "solana-signature"
    assert requests[0].url.raw_path.decode().endswith("/session/session%2F123/prepare")
    assert json.loads(requests[1].content)["authorizationId"] == "authorization-123"


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
