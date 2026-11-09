from __future__ import annotations

import base64
import json
from pathlib import Path

import httpx
import pytest
from solders.keypair import Keypair
from solders.transaction import Transaction

from swig_developer_sdk import Ed25519Authority, SwigClient, SwigResponseError
from swig_developer_sdk.signing import sign_prepared_swig_transaction

ROOT: Ed25519Authority = {"ed25519": {"public_key": "11111111111111111111111111111111"}}
WALLET = {
    "swigConfigAddress": "11111111111111111111111111111111",
    "walletAddress": "SysvarRent111111111111111111111111111111111",
}
RESERVATION = {"wallet": WALLET, "network": "NETWORK_DEVNET"}
TRANSACTION = {
    "transaction": "prepared-transaction",
    "transactionEncoding": "TRANSACTION_ENCODING_BASE64",
    "kind": "PREPARED_TRANSACTION_KIND_CREATE_SWIG_WALLET",
    **RESERVATION,
}


@pytest.mark.parametrize(
    "index,salt", [(None, None), (0, None), (0xFFFFFFFF, None), (None, bytes(32))]
)
async def test_lookup_and_creation_use_identical_commitment_inputs(
    index: int | None, salt: bytes | None
) -> None:
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        return httpx.Response(
            200, json={"data": {**RESERVATION, "transactions": [TRANSACTION]}}
        )

    async with SwigClient(
        api_key="test", network="devnet", transport=httpx.MockTransport(handler)
    ) as client:
        reserved = await client.wallets.reserve_address(
            root_authority=ROOT, account_index=index, salt=salt
        )
        created = await client.wallets.create_reserved(
            root_authority=ROOT, account_index=index, salt=salt, fee_payer="payer"
        )
    assert created.wallet == reserved.wallet
    assert [r.url.path for r in requests] == [
        "/transaction/api/wallet/reserve",
        "/transaction/api/wallet/create-reserved",
    ]
    lookup = json.loads(requests[0].content)
    assert json.loads(requests[1].content) == {**lookup, "feePayer": "payer"}
    assert lookup["rootAuthority"] == {
        "ed25519": {"publicKey": ROOT["ed25519"]["public_key"]}
    }
    if salt is not None:
        assert lookup["addressOptions"] == {
            "salt": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="
        }
    elif index is not None:
        assert lookup["addressOptions"] == {"accountIndex": index}
    else:
        assert "addressOptions" not in lookup


@pytest.mark.parametrize(
    "index,salt",
    [(-1, None), (0x100000000, None), (True, None), (0, bytes(32)), (None, bytes(31))],
)
async def test_invalid_options_fail_before_http(
    index: int | None, salt: bytes | None
) -> None:
    def unexpected(request: httpx.Request) -> httpx.Response:
        pytest.fail("invalid input reached HTTP")

    async with SwigClient(
        api_key="test", network="devnet", transport=httpx.MockTransport(unexpected)
    ) as client:
        with pytest.raises(ValueError):
            await client.wallets.reserve_address(
                root_authority=ROOT, account_index=index, salt=salt
            )


@pytest.mark.parametrize(
    "response",
    [
        None,
        {},
        {**RESERVATION, "wallet": {}},
        {**RESERVATION, "network": "NETWORK_MAINNET"},
    ],
)
async def test_lookup_rejects_incomplete_or_mismatched_response(
    response: object,
) -> None:
    async with SwigClient(
        api_key="test",
        network="devnet",
        transport=httpx.MockTransport(lambda _: httpx.Response(200, json=response)),
    ) as client:
        with pytest.raises(SwigResponseError):
            await client.wallets.reserve_address(root_authority=ROOT)


@pytest.mark.parametrize(
    "transactions",
    [
        [],
        [TRANSACTION, TRANSACTION],
        [{**TRANSACTION, "wallet": {**WALLET, "walletAddress": "other"}}],
        [{**TRANSACTION, "network": "NETWORK_MAINNET"}],
    ],
)
async def test_create_rejects_inconsistent_batches(
    transactions: list[dict[str, object]],
) -> None:
    async with SwigClient(
        api_key="test",
        network="devnet",
        transport=httpx.MockTransport(
            lambda _: httpx.Response(
                200, json={**RESERVATION, "transactions": transactions}
            )
        ),
    ) as client:
        with pytest.raises(SwigResponseError):
            await client.wallets.create_reserved(root_authority=ROOT, fee_payer="payer")


@pytest.mark.parametrize("invalid", ["", "not-a-solana-address"])
@pytest.mark.parametrize("field", ["swigConfigAddress", "walletAddress"])
async def test_rejects_invalid_addresses_for_lookup_and_creation(
    invalid: str, field: str
) -> None:
    wallet = {**WALLET, field: invalid}
    payload = {
        **RESERVATION,
        "wallet": wallet,
        "transactions": [{**TRANSACTION, "wallet": wallet}],
    }
    async with SwigClient(
        api_key="test",
        network="devnet",
        transport=httpx.MockTransport(lambda _: httpx.Response(200, json=payload)),
    ) as client:
        with pytest.raises(SwigResponseError, match="invalid addresses"):
            await client.wallets.reserve_address(root_authority=ROOT)
        with pytest.raises(SwigResponseError, match="invalid addresses"):
            await client.wallets.create_reserved(root_authority=ROOT, fee_payer="payer")


async def test_reserved_creation_can_be_signed_by_only_the_fee_payer() -> None:
    # Fixture generated by swig-transaction's canonical reservation builder (869f5eb).
    payload = json.loads(
        (Path(__file__).parents[2] / "test-fixtures/reserved-wallet.json").read_text()
    )
    payer = Keypair.from_seed(
        bytes.fromhex(
            "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"
        )
    )
    root: Ed25519Authority = {"ed25519": {"public_key": str(payer.pubkey())}}
    async with SwigClient(
        api_key="test",
        network="devnet",
        transport=httpx.MockTransport(lambda _: httpx.Response(200, json=payload)),
    ) as client:
        created = await client.wallets.create_reserved(
            root_authority=root, fee_payer=str(payer.pubkey())
        )
    assert created.creation_transaction is not None
    prepared = await sign_prepared_swig_transaction(created.creation_transaction)
    transaction = Transaction.from_bytes(base64.b64decode(prepared.transaction))
    assert transaction.message.header.num_required_signatures == 1
    assert transaction.message.account_keys[0] == payer.pubkey()
    assert bytes(transaction.message.instructions[1].data[:2]) == bytes([24, 0])
    message = bytes(transaction.message)
    transaction.partial_sign([payer], transaction.message.recent_blockhash)
    transaction.verify()
    assert bytes(transaction.message) == message
    assert bytes(transaction)
