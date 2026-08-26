from __future__ import annotations

import asyncio
import base64
import hashlib
import json
import os
import shutil
import struct
import subprocess
import sys
from collections.abc import Mapping, Sequence
from dataclasses import dataclass, replace
from uuid import uuid4

import httpx
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import ec
from cryptography.hazmat.primitives.asymmetric.utils import decode_dss_signature
from solders.hash import Hash
from solders.instruction import AccountMeta, Instruction
from solders.keypair import Keypair
from solders.message import Message, to_bytes_versioned
from solders.pubkey import Pubkey
from solders.signature import Signature
from solders.system_program import (
    ID as SYSTEM_PROGRAM_ID,
)
from solders.system_program import (
    CreateAccountParams,
    TransferParams,
    create_account,
    transfer,
)
from solders.token.associated import get_associated_token_address
from solders.transaction import Transaction, VersionedTransaction

from swig_developer_sdk import (
    AllAction,
    PasskeySigningResult,
    PreparedTransaction,
    SponsorSignedTransactionArgs,
    SwigClient,
    SwigDeveloperSdkError,
    SwigProxyConfig,
    TransferSolOperation,
    create_swig_proxy_handler,
    sign_prepared_swig_transaction,
    sign_prepared_transaction,
)
from swig_developer_sdk.signers import (
    create_participant_ed25519_signer,
    sign_participant_set_approval,
)
from swig_developer_sdk.transactions import normalize_prepared_transaction

API_BASE_URL = os.environ.get("SWIG_TRANSACTION_API_URL", "http://localhost:8080")
DATABASE_URL = os.environ.get(
    "SWIG_DATABASE_URL",
    "postgres://swig:swig@localhost:55432/swig",
)
RPC_URL = os.environ.get("SOLANA_RPC_URL", "http://localhost:8899")
SKIP_PAYMASTER = os.environ.get("SWIG_E2E_SKIP_PAYMASTER") == "1"
SWIG_PROGRAM_ID = "swigypWHEksbC64pWKwah1WTeh9JXwx8H1rJHLdbQMB"
MULTI_AUTHORITY_PROGRAM_ID = "BPgkm5iN5YJfbRS1nZ7G4rgfR3a4QYPaoXF7RQ9PBew"
LAMPORTS_PER_SOL = 1_000_000_000
TRANSFER_LAMPORTS = 1_000
TOKEN_TRANSFER_AMOUNT = 25
TOKEN_PROGRAM_ID = Pubkey.from_string("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")
ASSOCIATED_TOKEN_PROGRAM_ID = Pubkey.from_string(
    "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
)
MINT_ACCOUNT_SIZE = 82
LOCAL_DEMO_ORGANIZATION_ID = "clocaldashboarddemoorg001"
LOCAL_DEMO_USER_ID = "clocaldashboarduser001"
P256_ORDER = int(
    "ffffffff00000000ffffffffffffffffbce6faada7179e84f3b9cac2fc632551",
    16,
)
P256_HALF_ORDER = P256_ORDER // 2


@dataclass(frozen=True, slots=True)
class LocalFixture:
    api_key: str
    api_key_id: str
    organization_id: str
    paymaster_api_key: str
    paymaster_api_key_id: str
    user_id: str


@dataclass(frozen=True, slots=True)
class P256Authority:
    private_key: ec.EllipticCurvePrivateKey
    public_key_hex: str

    async def sign(self, message: bytes) -> PasskeySigningResult:
        der_signature = self.private_key.sign(message, ec.ECDSA(hashes.SHA256()))
        r, s = decode_dss_signature(der_signature)
        if s > P256_HALF_ORDER:
            s = P256_ORDER - s
        return PasskeySigningResult(
            signature=r.to_bytes(32, "big") + s.to_bytes(32, "big")
        )


class SolanaRpc:
    def __init__(self, client: httpx.AsyncClient) -> None:
        self._client = client
        self._request_id = 0

    async def health(self) -> None:
        result = await self._call("getHealth")
        if result != "ok":
            raise RuntimeError("Surfpool health check failed")

    async def balance(self, address: str) -> int:
        result = _mapping(
            await self._call("getBalance", [address, {"commitment": "confirmed"}]),
            "getBalance result",
        )
        return _required_int(result.get("value"), "getBalance value")

    async def minimum_rent_balance(self, space: int = 0) -> int:
        return _required_int(
            await self._call("getMinimumBalanceForRentExemption", [space]),
            "minimum rent balance",
        )

    async def latest_blockhash(self) -> str:
        result = _mapping(
            await self._call("getLatestBlockhash", [{"commitment": "confirmed"}]),
            "getLatestBlockhash result",
        )
        value = _mapping(result.get("value"), "getLatestBlockhash value")
        return _required_string(value.get("blockhash"), "latest blockhash")

    async def token_balance(self, address: str) -> int:
        result = _mapping(
            await self._call(
                "getTokenAccountBalance",
                [address, {"commitment": "confirmed"}],
            ),
            "getTokenAccountBalance result",
        )
        value = _mapping(result.get("value"), "getTokenAccountBalance value")
        return int(_required_string(value.get("amount"), "token account amount"))

    async def airdrop_to_balance(self, address: str, minimum_balance: int) -> None:
        current_balance = await self.balance(address)
        if current_balance >= minimum_balance:
            return
        signature = _required_string(
            await self._call(
                "requestAirdrop",
                [address, minimum_balance - current_balance],
            ),
            "airdrop signature",
        )
        await self.confirm(signature)

    async def send_transaction(self, transaction: str) -> str:
        signature = _required_string(
            await self._call(
                "sendTransaction",
                [
                    transaction,
                    {
                        "encoding": "base64",
                        "skipPreflight": False,
                        "preflightCommitment": "confirmed",
                    },
                ],
            ),
            "transaction signature",
        )
        await self.confirm(signature)
        return signature

    async def simulate_transaction(self, transaction: str) -> Mapping[str, object]:
        result = _mapping(
            await self._call(
                "simulateTransaction",
                [
                    transaction,
                    {
                        "encoding": "base64",
                        "sigVerify": True,
                        "commitment": "confirmed",
                    },
                ],
            ),
            "simulateTransaction result",
        )
        return _mapping(result.get("value"), "simulateTransaction value")

    async def confirm(self, signature: str) -> None:
        for _ in range(80):
            result = _mapping(
                await self._call(
                    "getSignatureStatuses",
                    [[signature], {"searchTransactionHistory": True}],
                ),
                "signature status result",
            )
            values = _sequence(result.get("value"), "signature statuses")
            status = values[0] if values else None
            if status is None:
                await asyncio.sleep(0.25)
                continue
            status_body = _mapping(status, "signature status")
            if status_body.get("err") is not None:
                raise RuntimeError("Surfpool rejected the submitted transaction")
            if status_body.get("confirmationStatus") in ("confirmed", "finalized"):
                return
            await asyncio.sleep(0.25)
        raise RuntimeError("Timed out waiting for Surfpool transaction confirmation")

    async def account_owner(self, address: str) -> str | None:
        result = _mapping(
            await self._call(
                "getAccountInfo",
                [address, {"commitment": "confirmed", "encoding": "base64"}],
            ),
            "getAccountInfo result",
        )
        value = result.get("value")
        if value is None:
            return None
        return _required_string(
            _mapping(value, "account info").get("owner"),
            "account owner",
        )

    async def wait_for_account_owner(self, address: str) -> str:
        for _ in range(20):
            owner = await self.account_owner(address)
            if owner is not None:
                return owner
            await asyncio.sleep(0.5)
        raise RuntimeError("Timed out waiting for the created Swig account")

    async def _call(
        self,
        method: str,
        params: Sequence[object] = (),
    ) -> object:
        self._request_id += 1
        response = await self._client.post(
            "",
            json={
                "jsonrpc": "2.0",
                "id": self._request_id,
                "method": method,
                "params": list(params),
            },
        )
        response.raise_for_status()
        body = _mapping(response.json(), "JSON-RPC response")
        error = body.get("error")
        if error is not None:
            error_body = _mapping(error, "JSON-RPC error")
            code = error_body.get("code")
            message = error_body.get("message")
            raise RuntimeError(f"Surfpool RPC {method} failed: {code} {message}")
        if "result" not in body:
            raise RuntimeError(f"Surfpool RPC {method} response is missing result")
        return body["result"]


async def main() -> None:
    fixture = seed_local_fixture()
    try:
        async with httpx.AsyncClient(base_url=RPC_URL, timeout=30) as rpc_client:
            rpc = SolanaRpc(rpc_client)
            await rpc.health()
            try:
                result = await run_e2e(rpc, fixture)
            except SwigDeveloperSdkError as error:
                print(
                    json.dumps(
                        {
                            "api_error_code": error.code,
                            "api_error_details": error.details,
                            "api_error_status": error.status_code,
                        },
                        sort_keys=True,
                    ),
                    file=sys.stderr,
                )
                raise
        print(json.dumps(result, indent=2, sort_keys=True))
    finally:
        try:
            cleanup_local_fixture(fixture)
        except RuntimeError:
            print("warning: local API fixture cleanup failed", file=sys.stderr)


async def run_e2e(rpc: SolanaRpc, fixture: LocalFixture) -> dict[str, object]:
    fee_payer = Keypair()
    requester = create_p256_authority()
    destination = Keypair()
    grouped_destination = Keypair()
    proxy_destination = Keypair()

    await rpc.airdrop_to_balance(str(fee_payer.pubkey()), LAMPORTS_PER_SOL)
    destination_rent = await rpc.minimum_rent_balance()
    for destination_keypair in (
        destination,
        grouped_destination,
        proxy_destination,
    ):
        await rpc.airdrop_to_balance(
            str(destination_keypair.pubkey()),
            destination_rent,
        )

    swig = SwigClient(
        api_key=fixture.api_key,
        base_url=API_BASE_URL,
        network="devnet",
    )
    requester_authority = {
        "secp256r1": {"publicKey": requester.public_key_hex},
    }
    created = await swig.wallets.create(
        fee_payer=str(fee_payer.pubkey()),
        initial_user=requester_authority,
    )
    creation_transaction = created.creation_transaction
    if creation_transaction is None:
        raise RuntimeError("Create wallet response is missing creation_transaction")

    signed_creation = await sign_with_keypairs(creation_transaction, [fee_payer])
    await rpc.send_transaction(signed_creation)
    account_owner = await rpc.wait_for_account_owner(created.wallet.swig_config_address)
    if account_owner != SWIG_PROGRAM_ID:
        raise RuntimeError("Created Swig account has the wrong program owner")

    await rpc.airdrop_to_balance(
        created.wallet.wallet_address,
        LAMPORTS_PER_SOL // 10,
    )
    participant_set_result = await run_participant_set_e2e(
        rpc=rpc,
        swig=swig,
        fee_payer=fee_payer,
        requester=requester,
        swig_config_address=created.wallet.swig_config_address,
        wallet_address=created.wallet.wallet_address,
    )
    wallet = swig.wallets.use(
        created.wallet.swig_config_address,
        network="devnet",
        requester_authority=requester_authority,
    )
    prepared_transfer = await wallet.transfer.sol(
        fee_payer=str(fee_payer.pubkey()),
        destination=str(destination.pubkey()),
        amount=TRANSFER_LAMPORTS,
    )
    destination_delta, wallet_delta = await submit_and_verify_sol_transfer(
        rpc,
        prepared_transfer,
        fee_payer,
        requester,
        created.wallet.wallet_address,
        str(destination.pubkey()),
        TRANSFER_LAMPORTS,
    )

    grouped = await wallet.prepare(
        fee_payer=str(fee_payer.pubkey()),
        operations=(
            TransferSolOperation(
                destination=str(grouped_destination.pubkey()),
                amount=TRANSFER_LAMPORTS,
            ),
        ),
    )
    if not grouped.transactions:
        raise RuntimeError("Grouped prepare response is missing transactions")
    grouped_destination_before = await rpc.balance(str(grouped_destination.pubkey()))
    grouped_wallet_before = await rpc.balance(created.wallet.wallet_address)
    for prepared in grouped.transactions:
        await sign_and_send_prepared(rpc, prepared, fee_payer, requester)
    grouped_destination_after = await rpc.balance(str(grouped_destination.pubkey()))
    grouped_wallet_after = await rpc.balance(created.wallet.wallet_address)
    grouped_destination_delta = grouped_destination_after - grouped_destination_before
    grouped_wallet_delta = grouped_wallet_after - grouped_wallet_before
    _require_transfer_deltas(
        grouped_destination_delta,
        grouped_wallet_delta,
        TRANSFER_LAMPORTS,
        "Grouped prepare",
    )

    (
        token_mint,
        wallet_token_account,
        destination_token_account,
    ) = await setup_test_token(
        rpc,
        fee_payer,
        Pubkey.from_string(created.wallet.wallet_address),
        destination.pubkey(),
    )
    token_before = await rpc.token_balance(str(destination_token_account))
    prepared_token_transfer = await wallet.transfer.token(
        fee_payer=str(fee_payer.pubkey()),
        mint=str(token_mint),
        destination_owner=str(destination.pubkey()),
        amount=TOKEN_TRANSFER_AMOUNT,
    )
    await sign_and_send_prepared(rpc, prepared_token_transfer, fee_payer, requester)
    token_after = await rpc.token_balance(str(destination_token_account))
    token_delta = token_after - token_before
    if token_delta != TOKEN_TRANSFER_AMOUNT:
        raise RuntimeError("Token transfer produced the wrong destination delta")
    wallet_token_balance = await rpc.token_balance(str(wallet_token_account))
    if wallet_token_balance != 0:
        raise RuntimeError("Token transfer did not debit the Swig token account")

    proxy = create_swig_proxy_handler(
        SwigProxyConfig(
            api_key=fixture.api_key,
            transaction_api_url=API_BASE_URL,
            network="devnet",
            fee_payer=str(fee_payer.pubkey()),
            resolve_requester_authority=lambda _context: requester_authority,
        )
    )
    proxy_transfer = await proxy.handle(
        method="POST",
        path="/api/swig/transfer/sol",
        body={
            "wallet": {
                "swigConfigAddress": created.wallet.swig_config_address,
                "walletAddress": created.wallet.wallet_address,
            },
            "network": "devnet",
            "destination": str(proxy_destination.pubkey()),
            "amount": str(TRANSFER_LAMPORTS),
        },
    )
    prepared_proxy_transfer = prepared_from_proxy(
        proxy_transfer.status, proxy_transfer.body
    )
    proxy_destination_delta, proxy_wallet_delta = await submit_and_verify_sol_transfer(
        rpc,
        prepared_proxy_transfer,
        fee_payer,
        requester,
        created.wallet.wallet_address,
        str(proxy_destination.pubkey()),
        TRANSFER_LAMPORTS,
    )
    paymaster_result = (
        {"paymaster_skipped": True}
        if SKIP_PAYMASTER
        else await run_paymaster_e2e(rpc, fixture)
    )

    return {
        "status": "ok",
        "api": API_BASE_URL,
        "rpc": RPC_URL,
        "authority_scheme": "secp256r1",
        "swig_config_address": created.wallet.swig_config_address,
        "wallet_address": created.wallet.wallet_address,
        "account_owner": account_owner,
        "destination_delta_lamports": destination_delta,
        "wallet_delta_lamports": wallet_delta,
        "grouped_destination_delta_lamports": grouped_destination_delta,
        "grouped_wallet_delta_lamports": grouped_wallet_delta,
        "token_delta": token_delta,
        "proxy_destination_delta_lamports": proxy_destination_delta,
        "proxy_wallet_delta_lamports": proxy_wallet_delta,
        **participant_set_result,
        **paymaster_result,
    }


async def run_participant_set_e2e(
    *,
    rpc: SolanaRpc,
    swig: SwigClient,
    fee_payer: Keypair,
    requester: P256Authority,
    swig_config_address: str,
    wallet_address: str,
) -> dict[str, object]:
    members = (Keypair(), Keypair())
    member_authorities = tuple(
        {"ed25519": {"publicKey": str(member.pubkey())}} for member in members
    )
    created_set = await swig.participant_sets.create(
        swig_config_address=swig_config_address,
        fee_payer=str(fee_payer.pubkey()),
        threshold=2,
        members=member_authorities,
    )
    create_transaction = await sign_with_keypairs(
        created_set.transaction,
        [fee_payer],
    )
    create_signature = await rpc.send_transaction(create_transaction)
    participant_owner = await rpc.wait_for_account_owner(
        created_set.participant_set_address
    )
    if participant_owner != MULTI_AUTHORITY_PROGRAM_ID:
        raise RuntimeError("Created ParticipantSet has the wrong program owner")

    requester_authority = {
        "secp256r1": {"publicKey": requester.public_key_hex},
    }
    setup_wallet = swig.wallets.use(
        swig_config_address,
        network="devnet",
        requester_authority=requester_authority,
    )
    add_role = await setup_wallet.roles.add(
        fee_payer=str(fee_payer.pubkey()),
        authority={"participantSet": {"address": created_set.participant_set_address}},
        actions=(AllAction(),),
    )
    add_role_signature = await sign_and_send_prepared(
        rpc,
        add_role,
        fee_payer,
        requester,
    )

    participant_authority = {
        "participantSet": {"address": created_set.participant_set_address}
    }
    participant_wallet = swig.wallets.use(
        swig_config_address,
        network="devnet",
        requester_authority=participant_authority,
    )
    destinations = (Keypair(), Keypair())
    destination_rent = await rpc.minimum_rent_balance()
    for destination in destinations:
        await rpc.airdrop_to_balance(str(destination.pubkey()), destination_rent)

    prepared_transactions = tuple(
        [
            await participant_wallet.transfer.sol(
                fee_payer=str(fee_payer.pubkey()),
                destination=str(destination.pubkey()),
                amount=TRANSFER_LAMPORTS,
            )
            for destination in destinations
        ]
    )
    plans = tuple(
        prepared.participant_set_approval_plan for prepared in prepared_transactions
    )
    if any(plan is None for plan in plans):
        raise RuntimeError("ParticipantSet transfer is missing an approval plan")
    first_plan, second_plan = plans
    if first_plan is None or second_plan is None:
        raise RuntimeError("ParticipantSet transfer is missing an approval plan")
    if first_plan.nonce != second_plan.nonce:
        raise RuntimeError("Parallel ParticipantSet plans did not share one nonce")
    if first_plan.threshold != 2 or len(first_plan.members) != 2:
        raise RuntimeError("ParticipantSet approval plan has the wrong threshold shape")

    member_signers = {
        str(member.pubkey()): create_participant_ed25519_signer(
            public_key=str(member.pubkey()),
            sign_message=lambda message, member=member: bytes(
                member.sign_message(message)
            ),
        )
        for member in members
    }

    async def compile_prepared(
        prepared: PreparedTransaction,
    ) -> PreparedTransaction:
        plan = prepared.participant_set_approval_plan
        if plan is None:
            raise RuntimeError("ParticipantSet transfer is missing an approval plan")
        approvals = []
        for approval_request in plan.members:
            authority = approval_request.authority.get("ed25519")
            if authority is None:
                raise RuntimeError("ParticipantSet E2E expected ed25519 members")
            public_key = _required_string(
                authority.get("publicKey", authority.get("public_key")),
                "ParticipantSet member public key",
            )
            signer = member_signers.get(public_key)
            if signer is None:
                raise RuntimeError("ParticipantSet plan returned an unknown member")
            approvals.append(
                await sign_participant_set_approval(approval_request, signer)
            )
        compiled = await swig.transactions.compile_participant_set_approvals(
            prepared_transaction=prepared,
            approvals=approvals,
        )
        return compiled.transaction

    compiled_transactions = tuple(
        [await compile_prepared(prepared) for prepared in prepared_transactions]
    )
    signed_transactions = tuple(
        [
            await sign_with_keypairs(compiled, [fee_payer])
            for compiled in compiled_transactions
        ]
    )
    first_simulation = await rpc.simulate_transaction(signed_transactions[0])
    if first_simulation.get("err") is not None:
        raise RuntimeError("ParticipantSet compiled transaction failed simulation")
    units_consumed = _required_int(
        first_simulation.get("unitsConsumed"),
        "ParticipantSet simulation unitsConsumed",
    )
    transaction_bytes = len(base64.b64decode(signed_transactions[0], validate=True))

    destination_before = await rpc.balance(str(destinations[0].pubkey()))
    wallet_before = await rpc.balance(wallet_address)
    execute_signature = await rpc.send_transaction(signed_transactions[0])
    destination_after = await rpc.balance(str(destinations[0].pubkey()))
    wallet_after = await rpc.balance(wallet_address)
    _require_transfer_deltas(
        destination_after - destination_before,
        wallet_after - wallet_before,
        TRANSFER_LAMPORTS,
        "ParticipantSet SOL transfer",
    )

    replay_simulation = await rpc.simulate_transaction(signed_transactions[1])
    replay_error = replay_simulation.get("err")
    if replay_error is None or '"Custom":7013' not in json.dumps(
        replay_error,
        separators=(",", ":"),
    ):
        raise RuntimeError("ParticipantSet shared nonce did not reject stale approval")

    return {
        "participant_set_address": created_set.participant_set_address,
        "participant_set_owner": participant_owner,
        "participant_set_threshold": first_plan.threshold,
        "participant_set_member_count": len(first_plan.members),
        "participant_set_nonce": first_plan.nonce,
        "participant_set_create_signature": create_signature,
        "participant_set_add_role_signature": add_role_signature,
        "participant_set_execute_signature": execute_signature,
        "participant_set_units_consumed": units_consumed,
        "participant_set_transaction_bytes": transaction_bytes,
        "participant_set_stale_nonce_rejected": True,
    }


async def run_paymaster_e2e(
    rpc: SolanaRpc,
    fixture: LocalFixture,
) -> dict[str, object]:
    swig = SwigClient(
        api_key=fixture.paymaster_api_key,
        base_url=API_BASE_URL,
        network="devnet",
    )
    paymaster = await swig.paymaster.get_balance()
    if not paymaster.configured or paymaster.kind != "api" or not paymaster.address:
        raise RuntimeError("Local API paymaster is not configured")

    await rpc.airdrop_to_balance(paymaster.address, LAMPORTS_PER_SOL // 10)
    funded_paymaster = await swig.paymaster.get_balance()
    if int(funded_paymaster.balance_lamports) < LAMPORTS_PER_SOL // 10:
        raise RuntimeError(
            "Paymaster balance endpoint did not reflect Surfpool funding"
        )

    user = Keypair()
    destination = Keypair()
    await rpc.airdrop_to_balance(str(user.pubkey()), LAMPORTS_PER_SOL // 100)
    await rpc.airdrop_to_balance(
        str(destination.pubkey()),
        await rpc.minimum_rent_balance(),
    )
    transaction = create_paymaster_transfer_transaction(
        paymaster=Pubkey.from_string(paymaster.address),
        user=user,
        destination=destination.pubkey(),
        amount=TRANSFER_LAMPORTS,
        blockhash=Hash.from_string(await rpc.latest_blockhash()),
    )

    destination_before = await rpc.balance(str(destination.pubkey()))
    user_before = await rpc.balance(str(user.pubkey()))
    paymaster_before = await rpc.balance(paymaster.address)
    idempotency_key = f"python-local-e2e-{uuid4()}"
    sponsor_args = SponsorSignedTransactionArgs(
        transaction=transaction,
        idempotency_key=idempotency_key,
    )
    submitted = await swig.transactions.sponsor(sponsor_args)
    await rpc.confirm(submitted.signature)
    destination_after = await rpc.balance(str(destination.pubkey()))
    user_after = await rpc.balance(str(user.pubkey()))
    paymaster_after = await rpc.balance(paymaster.address)

    destination_delta = destination_after - destination_before
    user_delta = user_after - user_before
    paymaster_fee = paymaster_before - paymaster_after
    if destination_delta != TRANSFER_LAMPORTS:
        raise RuntimeError("Sponsored transfer produced the wrong destination delta")
    if user_delta != -TRANSFER_LAMPORTS:
        raise RuntimeError("Sponsored transfer charged the user a transaction fee")
    if paymaster_fee <= 0:
        raise RuntimeError("Sponsored transfer did not charge the paymaster")

    replayed = await swig.transactions.sponsor(sponsor_args)
    if replayed != submitted:
        raise RuntimeError("Idempotent sponsor retry changed the response")
    if await rpc.balance(str(destination.pubkey())) != destination_after:
        raise RuntimeError("Idempotent sponsor retry repeated the transfer")
    if await rpc.balance(str(user.pubkey())) != user_after:
        raise RuntimeError("Idempotent sponsor retry charged the user again")
    if await rpc.balance(paymaster.address) != paymaster_after:
        raise RuntimeError("Idempotent sponsor retry charged the paymaster again")

    return {
        "paymaster_configured": True,
        "paymaster_idempotency_replayed": True,
        "paymaster_destination_delta_lamports": destination_delta,
        "paymaster_user_delta_lamports": user_delta,
        "paymaster_fee_lamports": paymaster_fee,
    }


def create_paymaster_transfer_transaction(
    *,
    paymaster: Pubkey,
    user: Keypair,
    destination: Pubkey,
    amount: int,
    blockhash: Hash,
) -> str:
    instruction = transfer(
        TransferParams(
            from_pubkey=user.pubkey(),
            to_pubkey=destination,
            lamports=amount,
        )
    )
    message = Message.new_with_blockhash([instruction], paymaster, blockhash)
    signatures = [
        Signature.default() for _ in range(message.header.num_required_signatures)
    ]
    signer_keys = message.account_keys[: message.header.num_required_signatures]
    signatures[signer_keys.index(user.pubkey())] = user.sign_message(
        to_bytes_versioned(message)
    )
    transaction = Transaction.populate(message, signatures)
    return base64.b64encode(bytes(transaction)).decode("ascii")


async def submit_and_verify_sol_transfer(
    rpc: SolanaRpc,
    prepared: PreparedTransaction,
    fee_payer: Keypair,
    requester: P256Authority,
    wallet_address: str,
    destination: str,
    amount: int,
) -> tuple[int, int]:
    destination_before = await rpc.balance(destination)
    wallet_before = await rpc.balance(wallet_address)
    await sign_and_send_prepared(rpc, prepared, fee_payer, requester)
    destination_after = await rpc.balance(destination)
    wallet_after = await rpc.balance(wallet_address)
    destination_delta = destination_after - destination_before
    wallet_delta = wallet_after - wallet_before
    _require_transfer_deltas(
        destination_delta,
        wallet_delta,
        amount,
        "SOL transfer",
    )
    return destination_delta, wallet_delta


async def sign_and_send_prepared(
    rpc: SolanaRpc,
    prepared: PreparedTransaction,
    fee_payer: Keypair,
    requester: P256Authority,
) -> str:
    if prepared.signature_requests:
        signature_schemes = tuple(
            request.scheme for request in prepared.signature_requests
        )
        if signature_schemes != ("secp256r1",):
            raise RuntimeError(
                "Prepared transaction did not request exactly one secp256r1 signature"
            )
        swig_signed = await sign_prepared_swig_transaction(
            prepared,
            secp256r1=requester.sign,
        )
        prepared = replace(
            prepared,
            transaction=swig_signed.transaction,
            signature_requests=(),
        )
    signed = await sign_with_keypairs(prepared, [fee_payer])
    return await rpc.send_transaction(signed)


async def setup_test_token(
    rpc: SolanaRpc,
    fee_payer: Keypair,
    wallet_owner: Pubkey,
    destination_owner: Pubkey,
) -> tuple[Pubkey, Pubkey, Pubkey]:
    mint = Keypair()
    wallet_token_account = get_associated_token_address(wallet_owner, mint.pubkey())
    destination_token_account = get_associated_token_address(
        destination_owner,
        mint.pubkey(),
    )
    instructions = (
        create_account(
            CreateAccountParams(
                from_pubkey=fee_payer.pubkey(),
                to_pubkey=mint.pubkey(),
                lamports=await rpc.minimum_rent_balance(MINT_ACCOUNT_SIZE),
                space=MINT_ACCOUNT_SIZE,
                owner=TOKEN_PROGRAM_ID,
            )
        ),
        initialize_mint_instruction(mint.pubkey(), fee_payer.pubkey()),
        create_associated_token_account_instruction(
            fee_payer.pubkey(),
            wallet_owner,
            mint.pubkey(),
            wallet_token_account,
        ),
        create_associated_token_account_instruction(
            fee_payer.pubkey(),
            destination_owner,
            mint.pubkey(),
            destination_token_account,
        ),
        mint_to_instruction(
            mint.pubkey(),
            wallet_token_account,
            fee_payer.pubkey(),
            TOKEN_TRANSFER_AMOUNT,
        ),
    )
    message = Message(instructions, fee_payer.pubkey())
    transaction = Transaction(
        [fee_payer, mint],
        message,
        Hash.from_string(await rpc.latest_blockhash()),
    )
    await rpc.send_transaction(base64.b64encode(bytes(transaction)).decode("ascii"))
    return mint.pubkey(), wallet_token_account, destination_token_account


def initialize_mint_instruction(mint: Pubkey, authority: Pubkey) -> Instruction:
    data = bytes((20, 0)) + bytes(authority) + struct.pack("<I", 0)
    return Instruction(
        TOKEN_PROGRAM_ID,
        data,
        [AccountMeta(mint, is_signer=False, is_writable=True)],
    )


def create_associated_token_account_instruction(
    payer: Pubkey,
    owner: Pubkey,
    mint: Pubkey,
    associated_token_account: Pubkey,
) -> Instruction:
    return Instruction(
        ASSOCIATED_TOKEN_PROGRAM_ID,
        b"",
        [
            AccountMeta(payer, is_signer=True, is_writable=True),
            AccountMeta(
                associated_token_account,
                is_signer=False,
                is_writable=True,
            ),
            AccountMeta(owner, is_signer=False, is_writable=False),
            AccountMeta(mint, is_signer=False, is_writable=False),
            AccountMeta(SYSTEM_PROGRAM_ID, is_signer=False, is_writable=False),
            AccountMeta(TOKEN_PROGRAM_ID, is_signer=False, is_writable=False),
        ],
    )


def mint_to_instruction(
    mint: Pubkey,
    destination: Pubkey,
    authority: Pubkey,
    amount: int,
) -> Instruction:
    return Instruction(
        TOKEN_PROGRAM_ID,
        bytes((7,)) + struct.pack("<Q", amount),
        [
            AccountMeta(mint, is_signer=False, is_writable=True),
            AccountMeta(destination, is_signer=False, is_writable=True),
            AccountMeta(authority, is_signer=True, is_writable=False),
        ],
    )


def prepared_from_proxy(
    status: int,
    body: Mapping[str, object],
) -> PreparedTransaction:
    if status != 200:
        raise RuntimeError("Python proxy failed to prepare the transaction")
    return normalize_prepared_transaction(body.get("prepared"))


def _require_transfer_deltas(
    destination_delta: int,
    wallet_delta: int,
    amount: int,
    label: str,
) -> None:
    if destination_delta != amount:
        raise RuntimeError(f"{label} produced the wrong destination balance delta")
    if wallet_delta != -amount:
        raise RuntimeError(f"{label} produced the wrong Swig wallet balance delta")


def create_p256_authority() -> P256Authority:
    private_key = ec.generate_private_key(ec.SECP256R1())
    public_key = private_key.public_key().public_bytes(
        encoding=serialization.Encoding.X962,
        format=serialization.PublicFormat.CompressedPoint,
    )
    return P256Authority(
        private_key=private_key,
        public_key_hex=public_key.hex(),
    )


async def sign_with_keypairs(
    prepared: PreparedTransaction,
    keypairs: Sequence[Keypair],
) -> str:
    def sign(transaction_value: str, _: PreparedTransaction) -> str:
        transaction = VersionedTransaction.from_bytes(
            base64.b64decode(transaction_value, validate=True)
        )
        message_bytes = to_bytes_versioned(transaction.message)
        signatures = list(transaction.signatures)
        required_signatures = transaction.message.header.num_required_signatures
        signer_keys = transaction.message.account_keys[:required_signatures]

        for keypair in keypairs:
            try:
                index = signer_keys.index(keypair.pubkey())
            except ValueError:
                continue
            signatures[index] = keypair.sign_message(message_bytes)

        missing_signers = [
            str(signer_keys[index])
            for index in range(required_signatures)
            if signatures[index] == Signature.default()
        ]
        if missing_signers:
            raise RuntimeError(
                "Prepared transaction requires unavailable signer(s): "
                + ", ".join(missing_signers)
            )

        signed = VersionedTransaction.populate(transaction.message, signatures)
        return base64.b64encode(bytes(signed)).decode("ascii")

    signed = await sign_prepared_transaction(prepared, sign_transaction=sign)
    return signed.transaction


def seed_local_fixture() -> LocalFixture:
    run_id = uuid4().hex
    fixture = LocalFixture(
        api_key=f"sk_local_python_e2e_{run_id}",
        api_key_id=f"local-python-e2e-key-{run_id}",
        organization_id=f"local-python-e2e-org-{run_id}",
        paymaster_api_key=f"sk_local_python_paymaster_e2e_{run_id}",
        paymaster_api_key_id=f"local-python-paymaster-e2e-key-{run_id}",
        user_id=f"local-python-e2e-user-{run_id}",
    )
    sql = f"""
BEGIN;

INSERT INTO "user" (id, email, "updatedAt")
VALUES (
  {_sql_literal(fixture.user_id)},
  {_sql_literal(f"{fixture.user_id}@local.test")},
  NOW()
);

INSERT INTO "organizations" (id, name, "ownerId", "updatedAt")
VALUES (
  {_sql_literal(fixture.organization_id)},
  'Local Python SDK E2E',
  {_sql_literal(fixture.user_id)},
  NOW()
);

INSERT INTO "api_keys" (id, key, name, "organizationId", "userId", "updatedAt")
VALUES (
  {_sql_literal(fixture.api_key_id)},
  {_sql_literal(hashlib.sha256(fixture.api_key.encode()).hexdigest())},
  'Local Python SDK E2E',
  {_sql_literal(fixture.organization_id)},
  {_sql_literal(fixture.user_id)},
  NOW()
);

INSERT INTO "api_keys" (id, key, name, "organizationId", "userId", "updatedAt")
VALUES (
  {_sql_literal(fixture.paymaster_api_key_id)},
  {_sql_literal(hashlib.sha256(fixture.paymaster_api_key.encode()).hexdigest())},
  'Local Python SDK Paymaster E2E',
  {_sql_literal(LOCAL_DEMO_ORGANIZATION_ID)},
  {_sql_literal(LOCAL_DEMO_USER_ID)},
  NOW()
);

COMMIT;
"""
    _run_psql(sql)
    return fixture


def cleanup_local_fixture(fixture: LocalFixture) -> None:
    sql = f"""
BEGIN;
DELETE FROM "api_keys" WHERE id = {_sql_literal(fixture.api_key_id)};
DELETE FROM "api_keys" WHERE id = {_sql_literal(fixture.paymaster_api_key_id)};
DELETE FROM "organizations" WHERE id = {_sql_literal(fixture.organization_id)};
DELETE FROM "user" WHERE id = {_sql_literal(fixture.user_id)};
COMMIT;
"""
    _run_psql(sql)


def _run_psql(sql: str) -> None:
    psql = os.environ.get("PSQL_BIN") or shutil.which("psql")
    if psql is None:
        raise RuntimeError("psql is required to seed the local API fixture")
    try:
        subprocess.run(
            [psql, DATABASE_URL, "-v", "ON_ERROR_STOP=1"],
            input=sql,
            text=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            check=True,
        )
    except subprocess.CalledProcessError as error:
        raise RuntimeError("Failed to update the local API fixture") from error


def _mapping(value: object, label: str) -> Mapping[str, object]:
    if not isinstance(value, Mapping):
        raise RuntimeError(f"{label} must be an object")
    return value


def _sequence(value: object, label: str) -> Sequence[object]:
    if not isinstance(value, Sequence) or isinstance(value, (str, bytes)):
        raise RuntimeError(f"{label} must be an array")
    return value


def _required_string(value: object, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise RuntimeError(f"{label} must be a non-empty string")
    return value


def _required_int(value: object, label: str) -> int:
    if not isinstance(value, int) or isinstance(value, bool):
        raise RuntimeError(f"{label} must be an integer")
    return value


def _sql_literal(value: str) -> str:
    return "'" + value.replace("'", "''") + "'"


if __name__ == "__main__":
    asyncio.run(main())
