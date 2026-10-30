from __future__ import annotations

import base64
import json
import struct
from collections.abc import Callable
from typing import cast

import httpx
import pytest
from solders.hash import Hash
from solders.instruction import AccountMeta, Instruction
from solders.message import Message
from solders.pubkey import Pubkey
from solders.transaction import Transaction, VersionedTransaction

from swig_developer_sdk import (
    AuthorizationLock,
    AuthorizationLocksResult,
    PasskeySigningResult,
    PreparedTransaction,
    RetryOptions,
    Secp256k1SigningResult,
    SwigClient,
    SwigDeveloperSdkError,
    SwigResponseError,
    WalletAuthority,
    sign_prepared_swig_transaction,
)

USDC = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
ENGINE = {"ed25519": {"publicKey": "engine"}}

LOCKS_RESPONSE: dict[str, object] = {
    "locks": [
        {
            "lockId": 0,
            "roleId": 1,
            "assetAccount": "swig-wallet",
            "amount": "18446744073709551615",
            "expirySlot": "500",
            "active": True,
        },
        {
            "lockId": 1,
            "roleId": 2,
            "assetAccount": "usdc-ata",
            "mint": USDC,
            "amount": "25",
            "expirySlot": "90",
            "active": False,
        },
    ],
    "currentSlot": "100",
    "nextLockId": 2,
}


def _client(
    handler: Callable[[httpx.Request], httpx.Response],
    *,
    network: str | None = "mainnet",
    max_retries: int = 0,
) -> SwigClient:
    return SwigClient(
        api_key="test-key",
        base_url="https://api.example.test",
        network=network,  # type: ignore[arg-type]
        retry_options=RetryOptions(max_retries=max_retries, retry_delay=0),
        transport=httpx.MockTransport(handler),
    )


def _prepared(kind: object) -> httpx.Response:
    return httpx.Response(
        200,
        json={
            "transaction": {
                "transaction": "backend-built-base64",
                "transactionEncoding": "TRANSACTION_ENCODING_BASE64",
                "network": "NETWORK_DEVNET",
                "recentBlockhash": "blockhash",
                "kind": kind,
                "wallet": {"swigConfigAddress": "config", "walletAddress": "wallet"},
            }
        },
    )


async def test_get_authorization_locks_uses_request_wallet_then_client_network() -> (
    None
):
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        return httpx.Response(200, json=LOCKS_RESPONSE)

    swig = _client(handler)
    wallet = swig.wallets.use("config/address", network="devnet")
    await wallet.get_authorization_locks()
    await wallet.get_authorization_locks(network="mainnet", include_expired=True)
    await swig.wallets.use("config").get_authorization_locks(include_expired=False)

    assert [str(request.url) for request in requests] == [
        "https://api.example.test/transaction/wallet/config%2Faddress/authorization-locks?network=NETWORK_DEVNET",
        "https://api.example.test/transaction/wallet/config%2Faddress/authorization-locks?network=NETWORK_MAINNET&include_expired=true",
        "https://api.example.test/transaction/wallet/config/authorization-locks?network=NETWORK_MAINNET&include_expired=false",
    ]
    assert all(request.method == "GET" for request in requests)
    assert all(
        request.headers["authorization"] == "Bearer test-key" for request in requests
    )


async def test_get_authorization_locks_normalizes_sol_and_token_locks() -> None:
    swig = _client(lambda _: httpx.Response(200, json=LOCKS_RESPONSE))
    assert await swig.wallets.use(
        "config"
    ).get_authorization_locks() == AuthorizationLocksResult(
        locks=(
            AuthorizationLock(
                lock_id=0,
                role_id=1,
                asset_account="swig-wallet",
                amount="18446744073709551615",
                expiry_slot="500",
                active=True,
            ),
            AuthorizationLock(
                lock_id=1,
                role_id=2,
                asset_account="usdc-ata",
                mint=USDC,
                amount="25",
                expiry_slot="90",
                active=False,
            ),
        ),
        current_slot="100",
        next_lock_id=2,
    )


@pytest.mark.parametrize(
    ("wire", "expected"),
    [
        (
            {"currentSlot": "7", "nextLockId": 0},
            AuthorizationLocksResult(locks=(), current_slot="7", next_lock_id=0),
        ),
        (
            {
                "locks": [
                    {
                        "lock_id": 3,
                        "role_id": 0,
                        "asset_account": "swig-wallet",
                        "amount": 10,
                        "expiry_slot": 20,
                        "active": True,
                    }
                ],
                "current_slot": 15,
                "next_lock_id": 4,
            },
            AuthorizationLocksResult(
                locks=(
                    AuthorizationLock(
                        lock_id=3,
                        role_id=0,
                        asset_account="swig-wallet",
                        amount="10",
                        expiry_slot="20",
                        active=True,
                    ),
                ),
                current_slot="15",
                next_lock_id=4,
            ),
        ),
    ],
)
async def test_get_authorization_locks_normalizes_absent_and_snake_case(
    wire: object, expected: AuthorizationLocksResult
) -> None:
    swig = _client(lambda _: httpx.Response(200, json=wire))
    assert await swig.wallets.use("config").get_authorization_locks() == expected


@pytest.mark.parametrize("status", [400, 401, 404, 503])
async def test_get_authorization_locks_propagates_api_failures(status: int) -> None:
    swig = _client(
        lambda _: httpx.Response(status, json={"message": "cannot inspect config"})
    )
    with pytest.raises(SwigDeveloperSdkError) as raised:
        await swig.wallets.use("config").get_authorization_locks()
    assert raised.value.status_code == status


async def test_get_authorization_locks_requires_network() -> None:
    swig = _client(lambda _: httpx.Response(200, json=LOCKS_RESPONSE), network=None)
    with pytest.raises(ValueError, match="network is required"):
        await swig.wallets.use("config").get_authorization_locks()


_LOCK = LOCKS_RESPONSE["locks"][1]  # type: ignore[index]


@pytest.mark.parametrize(
    "wire",
    [
        "not-an-object",
        [],
        {**LOCKS_RESPONSE, "locks": "not-a-list"},
        {**LOCKS_RESPONSE, "locks": [None]},
        {**LOCKS_RESPONSE, "currentSlot": None},
        {**LOCKS_RESPONSE, "currentSlot": "-1"},
        {**LOCKS_RESPONSE, "nextLockId": None},
        {**LOCKS_RESPONSE, "locks": [{**_LOCK, "amount": "1.5"}]},
        {**LOCKS_RESPONSE, "locks": [{**_LOCK, "expirySlot": None}]},
        {**LOCKS_RESPONSE, "locks": [{**_LOCK, "active": "yes"}]},
        {**LOCKS_RESPONSE, "locks": [{**_LOCK, "mint": ""}]},
        {**LOCKS_RESPONSE, "locks": [{**_LOCK, "assetAccount": None}]},
    ],
)
async def test_get_authorization_locks_rejects_malformed_responses(
    wire: object,
) -> None:
    swig = _client(lambda _: httpx.Response(200, json=wire))
    with pytest.raises(SwigResponseError):
        await swig.wallets.use("config").get_authorization_locks()


async def test_add_authorization_lock_prepares_an_unsigned_transaction() -> None:
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        return _prepared("PREPARED_TRANSACTION_KIND_ADD_AUTHORIZATION_LOCK")

    swig = _client(handler)
    wallet = swig.wallets.use("config", network="devnet", requester_authority=ENGINE)
    prepared = await wallet.add_authorization_lock(
        fee_payer="sponsor", amount=1_000_000, duration_slots=150
    )

    assert str(requests[0].url) == (
        "https://api.example.test/transaction/wallet/authorization-lock/add"
    )
    assert requests[0].method == "POST"
    assert requests[0].headers["authorization"] == "Bearer test-key"
    assert json.loads(requests[0].content) == {
        "network": "NETWORK_DEVNET",
        "swigAddress": "config",
        "feePayer": "sponsor",
        "requesterAuthority": {"ed25519": {"publicKey": "engine"}},
        "amount": "1000000",
        "durationSlots": "150",
    }
    assert prepared.transaction == "backend-built-base64"
    assert prepared.transaction_encoding == "base64"
    assert prepared.network == "devnet"
    assert prepared.kind == "add-authorization-lock"
    assert prepared.signature_requests == ()

    await wallet.add_authorization_lock(
        fee_payer="sponsor",
        mint=USDC,
        amount="18446744073709551615",
        expiry_slot=900,
        network="mainnet",
        requester_authority={"secp256r1": {"public_key": "passkey"}},
    )
    assert json.loads(requests[1].content) == {
        "network": "NETWORK_MAINNET",
        "swigAddress": "config",
        "feePayer": "sponsor",
        "requesterAuthority": {"secp256r1": {"publicKey": "passkey"}},
        "mint": USDC,
        "amount": "18446744073709551615",
        "expirySlot": "900",
    }


@pytest.mark.parametrize(
    ("method", "kwargs", "kind", "expected"),
    [
        (
            "add_authorization_lock",
            {"amount": 1, "duration_slots": 1},
            7,
            "add-authorization-lock",
        ),
        ("remove_authorization_lock", {"lock_id": 1}, 8, "remove-authorization-lock"),
    ],
)
async def test_authorization_lock_writes_normalize_numeric_kind(
    method: str, kwargs: dict[str, object], kind: int, expected: str
) -> None:
    swig = _client(
        lambda _: httpx.Response(
            200,
            json={
                "transaction": {
                    "unsigned_transaction": "prepared",
                    "transaction_encoding": 1,
                    "kind": kind,
                }
            },
        )
    )
    wallet = swig.wallets.use("config", requester_authority=ENGINE)
    prepared = await getattr(wallet, method)(fee_payer="sponsor", **kwargs)
    assert prepared.transaction == "prepared"
    assert prepared.transaction_encoding == "base64"
    assert prepared.kind == expected


@pytest.mark.parametrize(
    ("kwargs", "message"),
    [
        ({"amount": 1}, "exactly one authorization lock expiry"),
        (
            {"amount": 1, "expiry_slot": 900, "duration_slots": 150},
            "exactly one authorization lock expiry",
        ),
        ({"amount": 0, "duration_slots": 1}, "amount must be greater than zero"),
        ({"amount": "0", "duration_slots": 1}, "amount must be greater than zero"),
        ({"amount": -1, "duration_slots": 1}, "amount must be a non-negative"),
        ({"amount": "1e3", "duration_slots": 1}, "amount must be a non-negative"),
        ({"amount": "", "duration_slots": 1}, "amount must be a non-negative"),
        ({"amount": True, "duration_slots": 1}, "amount must be a non-negative"),
        (
            {"amount": 2**64, "duration_slots": 1},
            "amount must fit in an unsigned 64-bit integer",
        ),
        ({"amount": 1, "duration_slots": -5}, "duration_slots must be"),
        ({"amount": 1, "expiry_slot": "x"}, "expiry_slot must be"),
        ({"amount": 1, "duration_slots": 1, "mint": ""}, "mint must be"),
        ({"amount": 1, "duration_slots": 1, "mint": "not-base58!"}, "mint must be"),
        ({"amount": 1, "duration_slots": 1, "mint": "11111"}, "mint must be"),
        ({"amount": 1, "duration_slots": 1, "mint": f"{USDC}1"}, "mint must be"),
    ],
)
async def test_add_authorization_lock_validates_before_io(
    kwargs: dict[str, object], message: str
) -> None:
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        return httpx.Response(500)

    wallet = _client(handler).wallets.use("config", requester_authority=ENGINE)
    with pytest.raises(ValueError, match=message):
        await wallet.add_authorization_lock(fee_payer="sponsor", **kwargs)  # type: ignore[arg-type]
    assert requests == []


async def test_remove_authorization_lock_prepares_an_unsigned_transaction() -> None:
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        return _prepared("PREPARED_TRANSACTION_KIND_REMOVE_AUTHORIZATION_LOCK")

    swig = _client(handler)
    wallet = swig.wallets.use("config", network="devnet", requester_authority=ENGINE)
    prepared = await wallet.remove_authorization_lock(fee_payer="sponsor", lock_id=3)

    assert str(requests[0].url) == (
        "https://api.example.test/transaction/wallet/authorization-lock/remove"
    )
    assert requests[0].method == "POST"
    assert json.loads(requests[0].content) == {
        "network": "NETWORK_DEVNET",
        "swigAddress": "config",
        "feePayer": "sponsor",
        "requesterAuthority": {"ed25519": {"publicKey": "engine"}},
        "lockId": 3,
    }
    assert prepared.kind == "remove-authorization-lock"
    assert prepared.network == "devnet"

    await swig.wallets.remove_authorization_lock(
        wallet,
        fee_payer="sponsor",
        lock_id=0,
        network="mainnet",
        requester_authority={"ed25519": {"publicKey": "override"}},
    )
    body = json.loads(requests[1].content)
    assert body["network"] == "NETWORK_MAINNET"
    assert body["requesterAuthority"] == {"ed25519": {"publicKey": "override"}}
    assert body["lockId"] == 0


@pytest.mark.parametrize("lock_id", [-1, 1.5, 2**32, True, "1"])
async def test_remove_authorization_lock_validates_lock_id(lock_id: object) -> None:
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        return httpx.Response(500)

    wallet = _client(handler).wallets.use("config", requester_authority=ENGINE)
    with pytest.raises(ValueError, match="lock_id must be an unsigned 32-bit integer"):
        await wallet.remove_authorization_lock(fee_payer="sponsor", lock_id=lock_id)  # type: ignore[arg-type]
    assert requests == []


@pytest.mark.parametrize(
    ("method", "kwargs"),
    [
        ("add_authorization_lock", {"amount": 1, "duration_slots": 1}),
        ("remove_authorization_lock", {"lock_id": 1}),
    ],
)
async def test_authorization_lock_writes_require_authority_and_network(
    method: str, kwargs: dict[str, object]
) -> None:
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        return httpx.Response(500)

    with pytest.raises(ValueError, match="requester_authority is required"):
        await getattr(_client(handler).wallets.use("config"), method)(
            fee_payer="sponsor", **kwargs
        )
    with pytest.raises(ValueError, match="network is required"):
        await getattr(
            _client(handler, network=None).wallets.use(
                "config", requester_authority=ENGINE
            ),
            method,
        )(fee_payer="sponsor", **kwargs)
    assert requests == []


@pytest.mark.parametrize(
    ("method", "kwargs"),
    [
        ("add_authorization_lock", {"amount": 1, "duration_slots": 1}),
        ("remove_authorization_lock", {"lock_id": 1}),
    ],
)
@pytest.mark.parametrize("status", [400, 503])
async def test_authorization_lock_writes_do_not_retry_post(
    method: str, kwargs: dict[str, object], status: int
) -> None:
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        return httpx.Response(status, json={"message": "rejected"})

    wallet = _client(handler, max_retries=3).wallets.use(
        "config", requester_authority=ENGINE
    )
    with pytest.raises(SwigDeveloperSdkError) as raised:
        await getattr(wallet, method)(fee_payer="sponsor", **kwargs)
    assert raised.value.status_code == status
    assert len(requests) == 1


@pytest.mark.parametrize(
    ("method", "kwargs"),
    [
        ("add_authorization_lock", {"amount": 1, "duration_slots": 1}),
        ("remove_authorization_lock", {"lock_id": 1}),
    ],
)
async def test_authorization_lock_writes_require_prepared_transaction(
    method: str, kwargs: dict[str, object]
) -> None:
    wallet = _client(lambda _: httpx.Response(200, json={})).wallets.use(
        "config", requester_authority=ENGINE
    )
    with pytest.raises(SwigResponseError, match="missing transaction"):
        await getattr(wallet, method)(fee_payer="sponsor", **kwargs)


SWIG_PROGRAM_ID = Pubkey.from_string("swigypWHEksbC64pWKwah1WTeh9JXwx8H1rJHLdbQMB")
SECP256R1_PROGRAM_ID = Pubkey.from_string("Secp256r1SigVerify1111111111111111111111111")
SYSVAR_INSTRUCTIONS_ID = Pubkey.from_string(
    "Sysvar1nstructions1111111111111111111111111"
)
SYSTEM_PROGRAM_ID = Pubkey.from_string("11111111111111111111111111111111")
LOCK_CONFIG = Pubkey.new_unique()
LOCK_FEE_PAYER = Pubkey.new_unique()
LOCK_WALLET = Pubkey.new_unique()
LOCK_SLOT = 345_678_901
LOCK_COUNTER = 8
SECP_PUBLIC_KEY = bytes([3, *range(41, 73)])
SECP_MESSAGE_HASH = bytes(range(90, 122))
LOCK_ARGS_SIZE = {"add": 88, "remove": 16}


def _lock_args(route: str) -> bytes:
    """AddAuthorizationLockV1 / RemoveAuthorizationLockV1 fixed-size args."""
    if route == "add":
        return (
            struct.pack("<HHI", 25, 0, 1)
            + bytes(LOCK_WALLET)
            + bytes(32)
            + struct.pack("<QQ", 1_000_000, 9_000_000)
        )
    return struct.pack("<HHIII", 26, 0, 1, 3, 0)


def _lock_authority_payload(scheme: str, sysvar_index: int) -> bytes:
    head = struct.pack("<QI", LOCK_SLOT, LOCK_COUNTER)
    if scheme == "secp256k1":
        return head + bytes(65)
    return head + bytes((sysvar_index,)) + bytes(4)


def _r1_precompile(public_key: bytes, signature: bytes, message: bytes) -> bytes:
    message_offset = 16 + len(public_key) + len(signature)
    header = struct.pack(
        "<BB7H",
        1,
        0,
        16 + len(public_key),
        65535,
        16,
        65535,
        message_offset,
        len(message),
        65535,
    )
    return header + public_key + signature + message


def _lock_backend_transaction(scheme: str, route: str) -> str:
    accounts = [
        AccountMeta(LOCK_CONFIG, is_signer=False, is_writable=True),
        AccountMeta(LOCK_FEE_PAYER, is_signer=True, is_writable=True),
        AccountMeta(SYSTEM_PROGRAM_ID, is_signer=False, is_writable=False),
    ]
    if route == "add":
        accounts += [
            AccountMeta(LOCK_WALLET, is_signer=False, is_writable=False),
            AccountMeta(LOCK_WALLET, is_signer=False, is_writable=False),
        ]
    sysvar_index = len(accounts)
    instructions = []
    if scheme == "secp256r1":
        accounts.append(
            AccountMeta(SYSVAR_INSTRUCTIONS_ID, is_signer=False, is_writable=False)
        )
        instructions.append(
            Instruction(
                SECP256R1_PROGRAM_ID,
                _r1_precompile(SECP_PUBLIC_KEY, bytes(64), SECP_MESSAGE_HASH),
                [],
            )
        )
    instructions.append(
        Instruction(
            SWIG_PROGRAM_ID,
            _lock_args(route) + _lock_authority_payload(scheme, sysvar_index),
            accounts,
        )
    )
    message = Message.new_with_blockhash(instructions, LOCK_FEE_PAYER, Hash.default())
    return base64.b64encode(bytes(Transaction.new_unsigned(message))).decode()


def _lock_backend_response(scheme: str, route: str) -> dict[str, object]:
    """ProtoJSON as the dev portal returns it: enum names, uint64 slot string."""
    return {
        "transaction": {
            "transaction": _lock_backend_transaction(scheme, route),
            "transactionEncoding": "TRANSACTION_ENCODING_BASE64",
            "wallet": {
                "swigConfigAddress": str(LOCK_CONFIG),
                "walletAddress": str(LOCK_WALLET),
            },
            "expiresAt": "2026-10-30T12:00:00Z",
            "network": "NETWORK_DEVNET",
            "recentBlockhash": str(Hash.default()),
            "kind": (
                "PREPARED_TRANSACTION_KIND_ADD_AUTHORIZATION_LOCK"
                if route == "add"
                else "PREPARED_TRANSACTION_KIND_REMOVE_AUTHORIZATION_LOCK"
            ),
            "signatureRequests": [
                {
                    "scheme": (
                        "AUTHORITY_SIGNATURE_SCHEME_SECP256K1"
                        if scheme == "secp256k1"
                        else "AUTHORITY_SIGNATURE_SCHEME_SECP256R1"
                    ),
                    "signer": SECP_PUBLIC_KEY.hex(),
                    "messageHash": SECP_MESSAGE_HASH.hex(),
                    "slot": str(LOCK_SLOT),
                    "counter": LOCK_COUNTER,
                }
            ],
        }
    }


async def _prepare_lock(scheme: str, route: str) -> tuple[PreparedTransaction, object]:
    bodies: list[object] = []

    def handler(request: httpx.Request) -> httpx.Response:
        bodies.append(json.loads(request.content))
        return httpx.Response(200, json=_lock_backend_response(scheme, route))

    authority = cast(WalletAuthority, {scheme: {"public_key": SECP_PUBLIC_KEY.hex()}})
    wallet = _client(handler, network="devnet").wallets.use(
        str(LOCK_CONFIG), requester_authority=authority
    )
    if route == "add":
        prepared = await wallet.add_authorization_lock(
            fee_payer=str(LOCK_FEE_PAYER), amount=1_000_000, expiry_slot=9_000_000
        )
    else:
        prepared = await wallet.remove_authorization_lock(
            fee_payer=str(LOCK_FEE_PAYER), lock_id=3
        )
    return prepared, bodies[0]


def _signed_lock_instructions(transaction: str) -> tuple[bytes, bytes | None]:
    decoded = VersionedTransaction.from_bytes(base64.b64decode(transaction))
    keys = decoded.message.account_keys
    lock: bytes | None = None
    precompile: bytes | None = None
    for instruction in decoded.message.instructions:
        program_id = keys[instruction.program_id_index]
        if program_id == SWIG_PROGRAM_ID:
            lock = bytes(instruction.data)
        elif program_id == SECP256R1_PROGRAM_ID:
            precompile = bytes(instruction.data)
    assert lock is not None
    return lock, precompile


@pytest.mark.parametrize("route", ["add", "remove"])
@pytest.mark.parametrize("with_prefix", [False, True])
async def test_authorization_lock_secp256k1_signature_lands_after_slot_and_counter(
    route: str, with_prefix: bool
) -> None:
    prepared, body = await _prepare_lock("secp256k1", route)
    assert body["requesterAuthority"] == {  # type: ignore[index]
        "secp256k1": {"publicKey": SECP_PUBLIC_KEY.hex()}
    }
    assert prepared.kind == f"{route}-authorization-lock"
    assert prepared.signature_requests[0].scheme == "secp256k1"
    assert prepared.signature_requests[0].slot == LOCK_SLOT
    signature = bytes(200 - index for index in range(64)) + bytes((28,))
    prefix = b"\x19Ethereum Signed Message:\n64" if with_prefix else None

    def sign(message: bytes) -> Secp256k1SigningResult:
        assert message == SECP_MESSAGE_HASH.hex().encode("ascii")
        return Secp256k1SigningResult(signature=signature, prefix=prefix)

    signed = await sign_prepared_swig_transaction(
        prepared, secp256k1={SECP_PUBLIC_KEY.hex(): sign}
    )
    lock, _ = _signed_lock_instructions(signed.transaction)
    offset = LOCK_ARGS_SIZE[route]
    assert lock[:offset] == _lock_args(route)
    assert struct.unpack_from("<QI", lock, offset) == (LOCK_SLOT, LOCK_COUNTER)
    assert lock[offset + 12 : offset + 77] == signature
    assert lock[offset + 77 :] == (prefix or b"")


@pytest.mark.parametrize("route", ["add", "remove"])
@pytest.mark.parametrize("with_prefix", [False, True])
async def test_authorization_lock_secp256r1_signature_lands_in_preceding_precompile(
    route: str, with_prefix: bool
) -> None:
    prepared, body = await _prepare_lock("secp256r1", route)
    assert body["requesterAuthority"] == {  # type: ignore[index]
        "secp256r1": {"publicKey": SECP_PUBLIC_KEY.hex()}
    }
    signature = bytes(range(64))
    message = bytes((9, 8, 7, 6)) if with_prefix else None
    prefix = bytes((1, 3, 5, 7)) if with_prefix else None

    def sign(challenge: bytes) -> PasskeySigningResult:
        assert challenge == SECP_MESSAGE_HASH
        return PasskeySigningResult(signature=signature, message=message, prefix=prefix)

    signed = await sign_prepared_swig_transaction(prepared, secp256r1=sign)
    lock, precompile = _signed_lock_instructions(signed.transaction)
    assert precompile is not None
    signature_offset, public_key_offset, message_offset = (
        struct.unpack_from("<H", precompile, position)[0] for position in (2, 6, 10)
    )
    assert precompile[signature_offset : signature_offset + 64] == signature
    assert precompile[public_key_offset : public_key_offset + 33] == SECP_PUBLIC_KEY
    assert precompile[message_offset:] == (message or SECP_MESSAGE_HASH)
    offset = LOCK_ARGS_SIZE[route]
    assert lock[:offset] == _lock_args(route)
    assert lock[offset : offset + 17] == _lock_authority_payload(
        "secp256r1", 5 if route == "add" else 3
    )
    assert lock[offset + 17 :] == (prefix or b"")


@pytest.mark.parametrize(
    "authority",
    [
        {"program_exec_proof": {"role_id": 1, "zk_proof": "proof"}},
        {"participant_set": {"address": "set"}},
    ],
)
async def test_authorization_lock_writes_reject_unsupported_requesters(
    authority: dict[str, object],
) -> None:
    def handler(_: httpx.Request) -> httpx.Response:
        raise AssertionError("request should not be sent")

    wallet = _client(handler).wallets.use(
        "config", requester_authority=cast(WalletAuthority, authority)
    )
    message = "must use ed25519, secp256k1, or secp256r1"
    with pytest.raises(ValueError, match=message):
        await wallet.add_authorization_lock(
            fee_payer="payer", amount=1, duration_slots=10
        )
    with pytest.raises(ValueError, match=message):
        await wallet.remove_authorization_lock(fee_payer="payer", lock_id=0)
