from __future__ import annotations

import base64
import hashlib
import json
import struct
from collections.abc import Callable
from typing import cast

import httpx
import pytest
from k1_reference import (
    compressed_public_key,
    keccak256,
    recover_compressed,
    sign_digest,
)
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
from swig_developer_sdk.signers import (
    create_participant_ed25519_signer,
    sign_participant_set_approval,
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
PERSONAL_SIGN_PREFIX = b"\x19Ethereum Signed Message:\n64"
LOCK_CONFIG = Pubkey.new_unique()
LOCK_WALLET = Pubkey.new_unique()
LOCK_FEE_PAYER = Pubkey.new_unique()
LOCK_RENT_CLAIMER = Pubkey.new_unique()
LOCK_PARTICIPANT_SET = Pubkey.new_unique()
LOCK_SLOT = 345_678_901
LOCK_COUNTER = 8
K1_PRIVATE_KEY = 0x1F2E3D4C5B6A79880102030405060708090A0B0C0D0E0F101112131415161718
K1_PUBLIC_KEY = compressed_public_key(K1_PRIVATE_KEY)
R1_PUBLIC_KEY = bytes([3, *range(41, 73)])
LOCK_ARGS_SIZE = {"add": 88, "remove": 16}
LOCK_KIND_WIRE = {
    "add": "PREPARED_TRANSACTION_KIND_ADD_AUTHORIZATION_LOCK",
    "remove": "PREPARED_TRANSACTION_KIND_REMOVE_AUTHORIZATION_LOCK",
}


def _lock_args(route: str) -> bytes:
    """AddAuthorizationLockV1 / RemoveAuthorizationLockV1 fixed-size args."""
    if route == "add":
        return (
            struct.pack("<HHI", 25, 0, 4)
            + bytes(LOCK_WALLET)
            + bytes(32)
            + struct.pack("<QQ", 1_000_000, 9_000_000)
        )
    return struct.pack("<HHIII", 26, 0, 4, 3, 0)


def _lock_accounts(route: str, claimer: Pubkey) -> list[AccountMeta]:
    """Add: swig, wallet, payer, system, asset. Remove: swig, wallet, payer,
    system, rent claimer. Flags are merged to transaction level, as the dev
    portal does before hashing."""
    accounts = [
        AccountMeta(LOCK_CONFIG, is_signer=False, is_writable=True),
        AccountMeta(LOCK_WALLET, is_signer=False, is_writable=False),
        AccountMeta(LOCK_FEE_PAYER, is_signer=True, is_writable=True),
        AccountMeta(SYSTEM_PROGRAM_ID, is_signer=False, is_writable=False),
        AccountMeta(LOCK_WALLET, is_signer=False, is_writable=False)
        if route == "add"
        else AccountMeta(claimer, is_signer=False, is_writable=True),
    ]
    return [
        AccountMeta(
            account.pubkey,
            is_signer=any(o.is_signer for o in accounts if o.pubkey == account.pubkey),
            is_writable=any(
                o.is_writable for o in accounts if o.pubkey == account.pubkey
            ),
        )
        for account in accounts
    ]


def _signed_payload(
    args: bytes, accounts: list[tuple[Pubkey, bool, bool]], slot: int, counter: int
) -> bytes:
    """args || 40-byte AccountsPayload per account || slot || counter."""
    metas = b"".join(
        bytes(pubkey) + bytes((writable, signer)) + bytes(6)
        for pubkey, writable, signer in accounts
    )
    return args + metas + struct.pack("<QI", slot, counter)


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


def _lock_backend_prepared(
    requester: str, route: str, claimer: Pubkey
) -> dict[str, object]:
    args = _lock_args(route)
    accounts = _lock_accounts(route, claimer)
    head = struct.pack("<QI", LOCK_SLOT, LOCK_COUNTER)
    instructions: list[Instruction] = []
    request: dict[str, object] | None = None
    if requester == "secp256k1":
        message_hash = hashlib.sha256(
            _signed_payload(
                args,
                [(a.pubkey, a.is_writable, a.is_signer) for a in accounts],
                LOCK_SLOT,
                LOCK_COUNTER,
            )
        ).digest()
        data = args + head + bytes(65) + PERSONAL_SIGN_PREFIX
        request = {"public_key": K1_PUBLIC_KEY, "hash": message_hash}
    else:
        sysvar_index = len(accounts)
        accounts.append(
            AccountMeta(SYSVAR_INSTRUCTIONS_ID, is_signer=False, is_writable=False)
        )
        if requester == "participant_set":
            data = args + bytes((sysvar_index,))
        else:
            message_hash = keccak256(
                _signed_payload(
                    args,
                    [(a.pubkey, a.is_writable, a.is_signer) for a in accounts],
                    LOCK_SLOT,
                    LOCK_COUNTER,
                )
            )
            data = args + head + bytes((sysvar_index,)) + bytes(4)
            instructions.append(
                Instruction(
                    SECP256R1_PROGRAM_ID,
                    _r1_precompile(R1_PUBLIC_KEY, bytes(64), message_hash),
                    [],
                )
            )
            request = {"public_key": R1_PUBLIC_KEY, "hash": message_hash}
    instructions.append(Instruction(SWIG_PROGRAM_ID, data, accounts))
    message = Message.new_with_blockhash(instructions, LOCK_FEE_PAYER, Hash.default())
    prepared: dict[str, object] = {
        "transaction": base64.b64encode(
            bytes(Transaction.new_unsigned(message))
        ).decode(),
        "transactionEncoding": "TRANSACTION_ENCODING_BASE64",
        "wallet": {
            "swigConfigAddress": str(LOCK_CONFIG),
            "walletAddress": str(LOCK_WALLET),
        },
        "expiresAt": "2026-10-30T12:00:00Z",
        "network": "NETWORK_DEVNET",
        "recentBlockhash": str(Hash.default()),
        "kind": LOCK_KIND_WIRE[route],
    }
    if request is not None:
        prepared["signatureRequests"] = [
            {
                "scheme": (
                    "AUTHORITY_SIGNATURE_SCHEME_SECP256K1"
                    if requester == "secp256k1"
                    else "AUTHORITY_SIGNATURE_SCHEME_SECP256R1"
                ),
                "signer": cast(bytes, request["public_key"]).hex(),
                "messageHash": cast(bytes, request["hash"]).hex(),
                "slot": str(LOCK_SLOT),
                "counter": LOCK_COUNTER,
            }
        ]
    else:
        prepared["participantSetApprovalPlan"] = {
            "participantSetAddress": str(LOCK_PARTICIPANT_SET),
            "roleId": 4,
            "expirationSlot": "345679000",
            "nonce": 12,
            "transactionDigest": "11" * 32,
            "compilationEnvelope": "sealed-envelope",
            "threshold": 1,
            "members": [
                {
                    "memberIndex": 0,
                    "authority": {"ed25519": {"publicKey": "member-ed25519"}},
                    "challenge": "22" * 32,
                }
            ],
        }
    return prepared


def _lock_requester(requester: str) -> WalletAuthority:
    if requester == "secp256k1":
        return {"secp256k1": {"public_key": K1_PUBLIC_KEY.hex()}}
    if requester == "secp256r1":
        return {"secp256r1": {"public_key": R1_PUBLIC_KEY.hex()}}
    return {"participant_set": {"address": str(LOCK_PARTICIPANT_SET), "role_id": 4}}


async def _prepare_lock(
    requester: str, route: str, claimer: Pubkey = LOCK_RENT_CLAIMER
) -> tuple[SwigClient, PreparedTransaction, list[dict[str, object]]]:
    bodies: list[dict[str, object]] = []

    def handler(request: httpx.Request) -> httpx.Response:
        body = json.loads(request.content)
        bodies.append({"path": request.url.path, **body})
        if request.url.path == "/transaction/wallet/participant-set/compile":
            prepared = body["preparedTransaction"]
            return httpx.Response(
                200,
                json={
                    "transaction": {
                        "transaction": "compiled-base64",
                        "transactionEncoding": "TRANSACTION_ENCODING_BASE64",
                        "wallet": prepared["wallet"],
                        "network": "NETWORK_DEVNET",
                        "recentBlockhash": prepared["recentBlockhash"],
                        "kind": prepared["kind"],
                    },
                    "authorizationExpirationSlot": "345679000",
                },
            )
        return httpx.Response(
            200,
            json={"transaction": _lock_backend_prepared(requester, route, claimer)},
        )

    swig = _client(handler, network="devnet")
    wallet = swig.wallets.use(
        str(LOCK_CONFIG), requester_authority=_lock_requester(requester)
    )
    if route == "add":
        prepared = await wallet.add_authorization_lock(
            fee_payer=str(LOCK_FEE_PAYER), amount=1_000_000, expiry_slot=9_000_000
        )
    else:
        prepared = await wallet.remove_authorization_lock(
            fee_payer=str(LOCK_FEE_PAYER), lock_id=3
        )
    return swig, prepared, bodies


def _signed_lock(
    transaction: str,
) -> tuple[bytes, list[tuple[Pubkey, bool, bool]], bytes | None]:
    """The lock data, its accounts with transaction-level flags, and the
    secp256r1 precompile data if present."""
    decoded = VersionedTransaction.from_bytes(base64.b64decode(transaction))
    message = decoded.message
    header = message.header
    keys = message.account_keys
    signers = header.num_required_signatures
    writable_signers = signers - header.num_readonly_signed_accounts
    writable_unsigned = len(keys) - header.num_readonly_unsigned_accounts

    def flags(index: int) -> tuple[Pubkey, bool, bool]:
        is_signer = index < signers
        writable = index < writable_signers if is_signer else index < writable_unsigned
        return keys[index], writable, is_signer

    lock: tuple[bytes, list[tuple[Pubkey, bool, bool]]] | None = None
    precompile: bytes | None = None
    for instruction in message.instructions:
        program_id = keys[instruction.program_id_index]
        if program_id == SWIG_PROGRAM_ID:
            lock = (bytes(instruction.data), [flags(i) for i in instruction.accounts])
        elif program_id == SECP256R1_PROGRAM_ID:
            precompile = bytes(instruction.data)
    assert lock is not None
    return lock[0], lock[1], precompile


def _recover_program_signer(transaction: str, route: str) -> bytes:
    """state/src/authority/secp256k1.rs: recover from
    keccak256(prefix || hex(sha256(args || accounts || slot || counter)))."""
    data, accounts, _ = _signed_lock(transaction)
    offset = LOCK_ARGS_SIZE[route]
    slot, counter = struct.unpack_from("<QI", data, offset)
    digest = hashlib.sha256(
        _signed_payload(data[:offset], accounts, slot, counter)
    ).digest()
    signed_digest = keccak256(data[offset + 77 :] + digest.hex().encode("ascii"))
    return recover_compressed(signed_digest, data[offset + 12 : offset + 77])


def _personal_sign(message: bytes) -> Secp256k1SigningResult:
    prefix = f"\x19Ethereum Signed Message:\n{len(message)}".encode("ascii")
    return Secp256k1SigningResult(
        signature=sign_digest(K1_PRIVATE_KEY, keccak256(prefix + message)),
        prefix=prefix,
    )


def _raw_sign(message: bytes) -> Secp256k1SigningResult:
    return Secp256k1SigningResult(
        signature=sign_digest(K1_PRIVATE_KEY, keccak256(message))
    )


@pytest.mark.parametrize("route", ["add", "remove"])
async def test_authorization_lock_secp256k1_keeps_the_server_prefix(
    route: str,
) -> None:
    _, prepared, bodies = await _prepare_lock("secp256k1", route)
    assert bodies[0]["requesterAuthority"] == {
        "secp256k1": {"publicKey": K1_PUBLIC_KEY.hex()}
    }
    assert prepared.kind == f"{route}-authorization-lock"
    assert prepared.signature_requests[0].scheme == "secp256k1"
    assert prepared.signature_requests[0].slot == LOCK_SLOT
    original, _, _ = _signed_lock(prepared.transaction)
    offset = LOCK_ARGS_SIZE[route]
    assert original[offset + 77 :] == PERSONAL_SIGN_PREFIX

    signed = await sign_prepared_swig_transaction(
        prepared, secp256k1={K1_PUBLIC_KEY.hex(): _personal_sign}
    )

    data, _, _ = _signed_lock(signed.transaction)
    assert len(data) == len(original)
    assert data[: offset + 12] == original[: offset + 12]
    assert data[offset + 12 : offset + 77] != bytes(65)
    assert data[offset + 76] in (27, 28)
    assert data[offset + 77 :] == PERSONAL_SIGN_PREFIX


@pytest.mark.parametrize("route", ["add", "remove"])
@pytest.mark.parametrize("signer", [_personal_sign, _raw_sign])
async def test_authorization_lock_secp256k1_signature_verifies_like_the_program(
    route: str, signer: Callable[[bytes], Secp256k1SigningResult]
) -> None:
    _, prepared, _ = await _prepare_lock("secp256k1", route)
    messages: list[bytes] = []

    def sign(message: bytes) -> Secp256k1SigningResult:
        messages.append(message)
        return signer(message)

    signed = await sign_prepared_swig_transaction(prepared, secp256k1=sign)

    assert messages == [prepared.signature_requests[0].message_hash.encode("ascii")]
    assert _recover_program_signer(signed.transaction, route) == K1_PUBLIC_KEY


async def test_authorization_lock_hashes_fee_payer_rent_claimer_as_signer() -> None:
    _, prepared, _ = await _prepare_lock("secp256k1", "remove", LOCK_FEE_PAYER)
    signed = await sign_prepared_swig_transaction(prepared, secp256k1=_personal_sign)
    assert _recover_program_signer(signed.transaction, "remove") == K1_PUBLIC_KEY


@pytest.mark.parametrize("route", ["add", "remove"])
@pytest.mark.parametrize("with_prefix", [False, True])
async def test_authorization_lock_secp256r1_signature_lands_in_preceding_precompile(
    route: str, with_prefix: bool
) -> None:
    _, prepared, bodies = await _prepare_lock("secp256r1", route)
    assert bodies[0]["requesterAuthority"] == {
        "secp256r1": {"publicKey": R1_PUBLIC_KEY.hex()}
    }
    message_hash = bytes.fromhex(prepared.signature_requests[0].message_hash)
    signature = bytes(range(64))
    message = bytes((9, 8, 7, 6)) if with_prefix else None
    prefix = bytes((1, 3, 5, 7)) if with_prefix else None

    def sign(challenge: bytes) -> PasskeySigningResult:
        assert challenge == message_hash
        return PasskeySigningResult(signature=signature, message=message, prefix=prefix)

    signed = await sign_prepared_swig_transaction(prepared, secp256r1=sign)
    data, accounts, precompile = _signed_lock(signed.transaction)
    assert precompile is not None
    signature_offset, public_key_offset, message_offset = (
        struct.unpack_from("<H", precompile, position)[0] for position in (2, 6, 10)
    )
    assert precompile[signature_offset : signature_offset + 64] == signature
    assert precompile[public_key_offset : public_key_offset + 33] == R1_PUBLIC_KEY
    assert precompile[message_offset:] == (message or message_hash)
    offset = LOCK_ARGS_SIZE[route]
    sysvar_index = len(accounts) - 1
    assert accounts[sysvar_index][0] == SYSVAR_INSTRUCTIONS_ID
    assert data[:offset] == _lock_args(route)
    assert data[offset : offset + 17] == struct.pack(
        "<QIB4x", LOCK_SLOT, LOCK_COUNTER, sysvar_index
    )
    assert data[offset + 17 :] == (prefix or b"")


@pytest.mark.parametrize("route", ["add", "remove"])
async def test_authorization_lock_compiles_participant_set_approvals(
    route: str,
) -> None:
    swig, prepared, bodies = await _prepare_lock("participant_set", route)
    assert bodies[0]["requesterAuthority"] == {
        "participantSet": {
            "participantSetAddress": str(LOCK_PARTICIPANT_SET),
            "roleId": 4,
        }
    }
    assert prepared.signature_requests == ()
    plan = prepared.participant_set_approval_plan
    assert plan is not None
    assert plan.participant_set_address == str(LOCK_PARTICIPANT_SET)
    assert plan.role_id == 4

    signer = create_participant_ed25519_signer(
        public_key="member-ed25519",
        sign_message=lambda _: bytes([1]) * 64,
    )
    approvals = [
        await sign_participant_set_approval(member, signer) for member in plan.members
    ]
    compiled = await swig.transactions.compile_participant_set_approvals(
        prepared_transaction=prepared, approvals=approvals
    )

    compile_body = bodies[1]
    assert compile_body["path"] == "/transaction/wallet/participant-set/compile"
    wire = cast(dict[str, object], compile_body["preparedTransaction"])
    assert wire["kind"] == LOCK_KIND_WIRE[route]
    assert wire["transaction"] == prepared.transaction
    assert wire["wallet"] == {
        "swigConfigAddress": str(LOCK_CONFIG),
        "walletAddress": str(LOCK_WALLET),
    }
    assert wire["participantSetApprovalPlan"] == {
        "participantSetAddress": str(LOCK_PARTICIPANT_SET),
        "roleId": 4,
        "expirationSlot": "345679000",
        "nonce": 12,
        "transactionDigest": "11" * 32,
        "compilationEnvelope": "sealed-envelope",
        "threshold": 1,
        "members": [
            {
                "memberIndex": 0,
                "authority": {"ed25519": {"publicKey": "member-ed25519"}},
                "challenge": "22" * 32,
            }
        ],
    }
    assert compile_body["approvals"] == [
        {
            "memberIndex": 0,
            "ed25519": {"signature": base64.b64encode(bytes([1]) * 64).decode()},
        }
    ]
    assert compiled.transaction.kind == f"{route}-authorization-lock"
    assert compiled.transaction.signature_requests == ()
    assert compiled.authorization_expiration_slot == "345679000"


async def test_authorization_lock_writes_reject_program_exec_proof() -> None:
    def handler(_: httpx.Request) -> httpx.Response:
        raise AssertionError("request should not be sent")

    wallet = _client(handler).wallets.use(
        "config",
        requester_authority={"program_exec_proof": {"role_id": 1, "zk_proof": "p"}},
    )
    message = "does not support programExecProof"
    with pytest.raises(ValueError, match=message):
        await wallet.add_authorization_lock(
            fee_payer="payer", amount=1, duration_slots=10
        )
    with pytest.raises(ValueError, match=message):
        await wallet.remove_authorization_lock(fee_payer="payer", lock_id=0)
