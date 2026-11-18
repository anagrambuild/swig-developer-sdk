from __future__ import annotations

import base64
import json

import base58
import httpx
import pytest
from solders.hash import Hash
from solders.instruction import AccountMeta, Instruction
from solders.keypair import Keypair
from solders.message import Message, MessageV0, to_bytes_versioned
from solders.pubkey import Pubkey
from solders.signature import Signature
from solders.transaction import VersionedTransaction

from swig_developer_sdk import (
    PaymasterSignedTransaction,
    RetryOptions,
    SwigClient,
    SwigDeveloperSdkError,
    SwigResponseError,
)

# Portal's get_transaction_v1.json fixture, including its original signature.
V1_SIGNED = base64.b64decode(
    "gQEAAQwAAAAGhoWdOMgKlyFIGEFpF+C/j8oVfkZPmMz9D/qd6S/UggEFyoTiTXdhUBQKFFAB/"
    "JZaRp9/+qq44d72uHsnwhwnZ459Jbtx8Ujed9Z7sMeyVrIT3x6JCftdlPGzI9E24xRHK4Ie"
    "Tael0NiJg7w/YTr89LR+cV4kDlwt4UIZ7iM4gtBtAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
    "AAAAAAAAANDOlC4efFBuIY3w198cUvr9w1KeSNZ00dskx1tUzMvkANAwAAAAIABARQAAEAAg"
    "MAAAEAIAD7/6XyxGmB28d+vHdY/8iLZCfN+WeKs8alx3iWOAPnYunxyc+BG99zsbtOLOMEos"
    "AUgsfF3ODUnvLEINKijQG4URcIAAAACAAAAE2nfKtvTNbFcmLsydZHawTD8EIeCYSLYWzYzp"
    "MVZsWivSHo3XEyLXWe7nuHA4jCg9+LvCeArE4NYwP0++hIyAM="
)


def _transactions(version: str) -> tuple[bytes, bytes]:
    if version == "v1":
        return V1_SIGNED[:-64] + bytes(64), V1_SIGNED
    payer, cosigner = Keypair(), Keypair()
    instruction = Instruction(
        Pubkey.new_unique(), b"data", [AccountMeta(cosigner.pubkey(), True, False)]
    )
    message: Message | MessageV0
    if version == "legacy":
        message = Message.new_with_blockhash(
            [instruction], payer.pubkey(), Hash.default()
        )
    else:
        message = MessageV0.try_compile(
            payer.pubkey(), [instruction], [], Hash.default()
        )
    cosignature = cosigner.sign_message(to_bytes_versioned(message))
    original = VersionedTransaction.populate(
        message, [Signature.default(), cosignature]
    )
    signed = VersionedTransaction(message, [payer, cosigner])
    return bytes(original), bytes(signed)


@pytest.mark.parametrize("version", ["legacy", "v0", "v1"])
async def test_sign_adds_only_the_paymaster_signature(version: str) -> None:
    original, signed = _transactions(version)
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        return httpx.Response(
            200,
            json={
                "request_id": "req",
                "signed_transaction": base58.b58encode(signed).decode("ascii"),
            },
        )

    async with SwigClient(
        api_key="test", network="devnet", transport=httpx.MockTransport(handler)
    ) as client:
        encoded = base64.b64encode(original).decode("ascii")
        result = await client.transactions.sign(
            transaction=encoded, network="mainnet", external_id="order-23"
        )
        await client.transactions.sign(transaction=encoded)

    assert result == PaymasterSignedTransaction(
        request_id="req", signed_transaction=base64.b64encode(signed).decode("ascii")
    )
    wire = {"base58_encoded_transaction": base58.b58encode(original).decode("ascii")}
    assert [(r.method, r.url.path, json.loads(r.content)) for r in requests] == [
        (
            "POST",
            "/paymaster/sign",
            {**wire, "network": "mainnet", "external_id": "order-23"},
        ),
        ("POST", "/paymaster/sign", {**wire, "network": "devnet"}),
    ]


@pytest.mark.parametrize(
    "failure",
    ["missing_id", "missing_tx", "base58", "truncated", "message", "cosigner", "payer"],
)
async def test_sign_rejects_invalid_responses(failure: str) -> None:
    original, signed = _transactions("v0")
    altered = bytearray(signed)
    if failure == "truncated":
        altered.pop()
    elif failure == "message":
        altered[-1] ^= 1
    elif failure == "cosigner":
        altered[1 + Signature.LENGTH] ^= 1
    elif failure == "payer":
        altered[1] ^= 1
    body = {
        "request_id": "req",
        "signed_transaction": base58.b58encode(bytes(altered)).decode("ascii"),
    }
    if failure == "missing_id":
        del body["request_id"]
    elif failure == "missing_tx":
        del body["signed_transaction"]
    elif failure == "base58":
        body["signed_transaction"] = "0"

    async with SwigClient(
        api_key="test",
        transport=httpx.MockTransport(lambda r: httpx.Response(200, json=body)),
    ) as client:
        with pytest.raises(SwigResponseError):
            await client.transactions.sign(
                transaction=base64.b64encode(original).decode()
            )


@pytest.mark.parametrize(
    "transaction", ["?", "eA==", base64.b64encode(b"\x81" + bytes(41)).decode()]
)
async def test_sign_rejects_invalid_input_before_http(transaction: str) -> None:
    def unexpected_request(request: httpx.Request) -> httpx.Response:
        pytest.fail(f"unexpected request to {request.url.path}")

    async with SwigClient(
        api_key="test", transport=httpx.MockTransport(unexpected_request)
    ) as client:
        with pytest.raises(ValueError):
            await client.transactions.sign(transaction=transaction)


async def test_sign_does_not_replay_retryable_failure() -> None:
    original, _ = _transactions("legacy")
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        return httpx.Response(503)

    async with SwigClient(
        api_key="test",
        retry_options=RetryOptions(max_retries=2, retry_delay=0),
        transport=httpx.MockTransport(handler),
    ) as client:
        with pytest.raises(SwigDeveloperSdkError):
            await client.transactions.sign(
                transaction=base64.b64encode(original).decode(), external_id="order-23"
            )
    assert [r.url.path for r in requests] == ["/paymaster/sign"]
    assert "network" not in json.loads(requests[0].content)
