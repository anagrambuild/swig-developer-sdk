from __future__ import annotations

import base64
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import Literal, TypeAlias

import base58

from .common import (
    Network,
    WalletAddressInfo,
    WalletAuthority,
    normalize_network,
    wallet_address_info_from_wire,
    wallet_authority_to_wire,
)
from .core import HttpClient

TransactionEncoding = Literal["base64"]
PreparedTransactionKind = Literal[
    "create-swig-wallet",
    "add-authority",
    "configure-recovery",
    "create-participant-set",
]
SignatureScheme = Literal["secp256r1", "secp256k1"]


@dataclass(frozen=True, slots=True)
class ClientSignatureRequest:
    scheme: SignatureScheme
    signer: str
    message_hash: str
    slot: int
    counter: int


@dataclass(frozen=True, slots=True)
class ParticipantApprovalRequest:
    member_index: int
    authority: WalletAuthority
    challenge: str


@dataclass(frozen=True, slots=True)
class ParticipantSetApprovalPlan:
    participant_set_address: str
    role_id: int
    expiration_slot: str
    nonce: int
    transaction_digest: str
    compilation_envelope: str
    threshold: int
    members: tuple[ParticipantApprovalRequest, ...]
    type: Literal["participantSet"] = "participantSet"


@dataclass(frozen=True, slots=True)
class PreparedTransaction:
    transaction: str
    signature_requests: tuple[ClientSignatureRequest, ...]
    transaction_encoding: TransactionEncoding | None = None
    wallet: WalletAddressInfo | None = None
    expires_at: str | None = None
    network: Network | None = None
    recent_blockhash: str | None = None
    kind: PreparedTransactionKind | None = None
    participant_set_approval_plan: ParticipantSetApprovalPlan | None = None


@dataclass(frozen=True, slots=True)
class Secp256k1ParticipantApproval:
    member_index: int
    signature: bytes


@dataclass(frozen=True, slots=True)
class WebAuthnP256ParticipantApproval:
    member_index: int
    authenticator_data: bytes
    client_data_json: bytes
    signature: bytes


@dataclass(frozen=True, slots=True)
class Ed25519ParticipantApproval:
    member_index: int
    signature: bytes


ParticipantSetApproval: TypeAlias = (
    Ed25519ParticipantApproval
    | Secp256k1ParticipantApproval
    | WebAuthnP256ParticipantApproval
)


@dataclass(frozen=True, slots=True)
class CompileParticipantSetApprovalsResult:
    transaction: PreparedTransaction
    authorization_expiration_slot: str


@dataclass(frozen=True, slots=True)
class SignedPreparedTransaction:
    transaction: str
    transaction_encoding: TransactionEncoding | None = None
    network: Network | None = None


@dataclass(frozen=True, slots=True)
class SubmittedTransaction:
    request_id: str
    signature: str
    spent_by_paymaster: str


@dataclass(frozen=True, slots=True)
class SponsorSignedTransactionArgs:
    transaction: str
    transaction_encoding: TransactionEncoding | None = None
    network: Network | None = None
    idempotency_key: str | None = None


@dataclass(frozen=True, slots=True)
class SponsorSignedTransactionBundleArgs:
    transactions: tuple[str, ...]
    network: Network | None = None
    idempotency_key: str | None = None


@dataclass(frozen=True, slots=True)
class SubmittedTransactionBundle:
    request_id: str
    bundle_id: str
    signatures: tuple[str, ...]
    estimated_spent_by_paymaster: str


@dataclass(frozen=True, slots=True)
class PreparedTransactionsResult:
    transactions: tuple[PreparedTransaction, ...]
    client_authority_transactions: tuple[PreparedTransaction, ...]
    fee_payer_only_transactions: tuple[PreparedTransaction, ...]
    wallet: WalletAddressInfo | None = None
    network: Network | None = None


class TransactionsClient:
    def __init__(
        self,
        http: HttpClient,
        default_network: Network | None = None,
    ) -> None:
        self._http = http
        self._default_network = default_network

    async def sponsor(
        self,
        args: SponsorSignedTransactionArgs,
    ) -> SubmittedTransaction:
        transaction_bytes = base64.b64decode(args.transaction)
        response = await self._http.post(
            "/paymaster/sponsor",
            {
                "base58_encoded_transaction": base58.b58encode(
                    transaction_bytes
                ).decode("ascii"),
                "network": args.network or self._default_network,
                "idempotencyKey": args.idempotency_key,
            },
            retry=args.idempotency_key is not None,
        )
        return normalize_submitted_transaction(response)

    async def compile_participant_set_approvals(
        self,
        *,
        prepared_transaction: PreparedTransaction,
        approvals: Sequence[ParticipantSetApproval],
    ) -> CompileParticipantSetApprovalsResult:
        response = _mapping(
            await self._http.post(
                "/transaction/wallet/participant-set/compile",
                {
                    "preparedTransaction": _prepared_transaction_to_wire(
                        prepared_transaction
                    ),
                    "approvals": [
                        _participant_approval_to_wire(approval)
                        for approval in approvals
                    ],
                },
            ),
            "Compile ParticipantSet response",
        )
        transaction = response.get("transaction")
        if transaction is None:
            raise ValueError("Compile ParticipantSet response is missing transaction")
        authorization_expiration_slot = _pick(
            response,
            "authorizationExpirationSlot",
            "authorization_expiration_slot",
        )
        if not isinstance(authorization_expiration_slot, (str, int)):
            raise ValueError(
                "Compile ParticipantSet response is missing authorizationExpirationSlot"
            )
        return CompileParticipantSetApprovalsResult(
            transaction=normalize_prepared_transaction(transaction),
            authorization_expiration_slot=str(authorization_expiration_slot),
        )

    async def sponsor_bundle(
        self,
        args: SponsorSignedTransactionBundleArgs,
    ) -> SubmittedTransactionBundle:
        if not 1 <= len(args.transactions) <= 5:
            raise ValueError("transactions must contain between 1 and 5 items")
        network = args.network or self._default_network
        if network != "mainnet":
            raise ValueError("sponsor_bundle only supports mainnet")
        response = await self._http.post(
            "/paymaster/sponsor/bundle",
            {
                "base58_encoded_transactions": [
                    base58.b58encode(base64.b64decode(transaction)).decode("ascii")
                    for transaction in args.transactions
                ],
                "network": network,
                "idempotencyKey": args.idempotency_key,
            },
            retry=args.idempotency_key is not None,
        )
        body = _mapping(response, "Sponsor bundle response")
        signatures = body.get("signatures", [])
        if not isinstance(signatures, Sequence) or isinstance(signatures, (str, bytes)):
            raise ValueError("Sponsor bundle response has invalid signatures")
        return SubmittedTransactionBundle(
            request_id=_required_string(
                _pick(body, "requestId", "request_id"), "requestId"
            ),
            bundle_id=_required_string(
                _pick(body, "bundleId", "bundle_id"), "bundleId"
            ),
            signatures=tuple(
                _required_string(signature, "signature") for signature in signatures
            ),
            estimated_spent_by_paymaster=str(
                _pick(
                    body,
                    "estimatedSpentByPaymaster",
                    "estimated_spent_by_paymaster",
                )
                or 0
            ),
        )


def normalize_prepared_transaction(response: object) -> PreparedTransaction:
    body = _mapping(response, "Prepared transaction response")
    transaction = _pick(
        body, "transaction", "unsignedTransaction", "unsigned_transaction"
    )
    if not isinstance(transaction, str) or not transaction:
        raise ValueError("Prepared transaction response is missing transaction")

    wallet_value = body.get("wallet")
    return PreparedTransaction(
        transaction=transaction,
        transaction_encoding=_normalize_transaction_encoding(
            _pick(body, "transactionEncoding", "transaction_encoding")
        ),
        wallet=(
            wallet_address_info_from_wire(wallet_value)
            if wallet_value is not None
            else None
        ),
        expires_at=_optional_string(_pick(body, "expiresAt", "expires_at")),
        network=normalize_network(body.get("network")),
        recent_blockhash=_optional_string(
            _pick(body, "recentBlockhash", "recent_blockhash")
        ),
        kind=_normalize_kind(body.get("kind")),
        signature_requests=_normalize_signature_requests(
            _pick(body, "signatureRequests", "signature_requests")
        ),
        participant_set_approval_plan=_normalize_participant_set_approval_plan(
            _pick(
                body,
                "participantSetApprovalPlan",
                "participant_set_approval_plan",
            )
        ),
    )


def normalize_prepared_transactions_result(
    response: object,
) -> PreparedTransactionsResult:
    body = _mapping(response, "Prepare transactions response")
    network = normalize_network(body.get("network"))
    raw_transactions = body.get("transactions", [])
    if not isinstance(raw_transactions, Sequence) or isinstance(
        raw_transactions, (str, bytes)
    ):
        raise ValueError("Prepare transactions response has invalid transactions")
    transactions = tuple(
        normalize_prepared_transaction(
            {
                **_mapping(item, "Prepared transaction"),
                "network": _mapping(item, "Prepared transaction").get(
                    "network", body.get("network")
                ),
            }
        )
        for item in raw_transactions
    )
    wallet_value = body.get("wallet")
    wallet = (
        wallet_address_info_from_wire(wallet_value)
        if wallet_value is not None
        else None
    )
    if wallet is not None and wallet.network is None and network is not None:
        wallet = WalletAddressInfo(
            wallet.swig_config_address,
            wallet.wallet_address,
            network,
        )
    return PreparedTransactionsResult(
        wallet=wallet,
        transactions=transactions,
        client_authority_transactions=tuple(
            item for item in transactions if item.signature_requests
        ),
        fee_payer_only_transactions=tuple(
            item for item in transactions if not item.signature_requests
        ),
        network=network,
    )


def normalize_submitted_transaction(response: object) -> SubmittedTransaction:
    body = _mapping(response, "Sponsor response")
    signature = body.get("signature")
    if not isinstance(signature, str) or not signature:
        raise ValueError("Sponsor response is missing signature")
    return SubmittedTransaction(
        request_id=_required_string(
            _pick(body, "requestId", "request_id"), "requestId"
        ),
        signature=signature,
        spent_by_paymaster=str(
            _pick(body, "spentByPaymaster", "spent_by_paymaster") or 0
        ),
    )


def _normalize_signature_requests(value: object) -> tuple[ClientSignatureRequest, ...]:
    if value is None:
        return ()
    if not isinstance(value, Sequence) or isinstance(value, (str, bytes)):
        raise ValueError("Prepared transaction has invalid signature request")
    requests: list[ClientSignatureRequest] = []
    for item in value:
        request = _mapping(item, "Prepared transaction signature request")
        scheme = _normalize_signature_scheme(request.get("scheme"))
        signer = request.get("signer")
        message_hash = _pick(request, "messageHash", "message_hash")
        slot = request.get("slot")
        counter = request.get("counter")
        if isinstance(slot, str):
            try:
                slot = int(slot, 10)
            except ValueError as error:
                raise ValueError(
                    "Prepared transaction has invalid signature request"
                ) from error
        if (
            scheme is None
            or not isinstance(signer, str)
            or not isinstance(message_hash, str)
            or not isinstance(slot, int)
            or not isinstance(counter, int)
        ):
            raise ValueError("Prepared transaction has invalid signature request")
        requests.append(
            ClientSignatureRequest(
                scheme=scheme,
                signer=signer,
                message_hash=message_hash,
                slot=slot,
                counter=counter,
            )
        )
    return tuple(requests)


def _normalize_signature_scheme(value: object) -> SignatureScheme | None:
    if value in ("secp256r1", "AUTHORITY_SIGNATURE_SCHEME_SECP256R1", 1):
        return "secp256r1"
    if value in ("secp256k1", "AUTHORITY_SIGNATURE_SCHEME_SECP256K1", 2):
        return "secp256k1"
    return None


def _normalize_transaction_encoding(value: object) -> TransactionEncoding | None:
    if value in ("base64", "TRANSACTION_ENCODING_BASE64", 1):
        return "base64"
    return None


def _normalize_kind(value: object) -> PreparedTransactionKind | None:
    if value in (
        "create-swig-wallet",
        "PREPARED_TRANSACTION_KIND_CREATE_SWIG_WALLET",
        1,
    ):
        return "create-swig-wallet"
    if value in ("add-authority", "PREPARED_TRANSACTION_KIND_ADD_AUTHORITY", 2):
        return "add-authority"
    if value in (
        "configure-recovery",
        "PREPARED_TRANSACTION_KIND_CONFIGURE_RECOVERY",
        3,
    ):
        return "configure-recovery"
    if value in (
        "create-participant-set",
        "PREPARED_TRANSACTION_KIND_CREATE_PARTICIPANT_SET",
        4,
    ):
        return "create-participant-set"
    return None


def _normalize_participant_set_approval_plan(
    value: object,
) -> ParticipantSetApprovalPlan | None:
    if value is None:
        return None
    body = _mapping(value, "ParticipantSet approval plan")
    expiration_slot = _pick(body, "expirationSlot", "expiration_slot")
    if not isinstance(expiration_slot, (str, int)):
        raise ValueError("ParticipantSet approval plan is missing expirationSlot")
    members_value = body.get("members")
    if (
        not isinstance(members_value, Sequence)
        or isinstance(members_value, (str, bytes))
        or not members_value
    ):
        raise ValueError("ParticipantSet approval plan has invalid members")
    return ParticipantSetApprovalPlan(
        participant_set_address=_required_string(
            _pick(body, "participantSetAddress", "participant_set_address"),
            "participantSetAddress",
        ),
        role_id=_required_int(_pick(body, "roleId", "role_id"), "roleId"),
        expiration_slot=str(expiration_slot),
        nonce=_required_int(body.get("nonce"), "nonce"),
        transaction_digest=_required_string(
            _pick(body, "transactionDigest", "transaction_digest"),
            "transactionDigest",
        ),
        compilation_envelope=_required_string(
            _pick(body, "compilationEnvelope", "compilation_envelope"),
            "compilationEnvelope",
        ),
        threshold=_required_int(body.get("threshold"), "threshold"),
        members=tuple(
            _normalize_participant_approval_request(item) for item in members_value
        ),
    )


def _normalize_participant_approval_request(
    value: object,
) -> ParticipantApprovalRequest:
    body = _mapping(value, "Participant approval request")
    return ParticipantApprovalRequest(
        member_index=_required_int(
            _pick(body, "memberIndex", "member_index"), "memberIndex"
        ),
        authority=_normalize_participant_authority(body.get("authority")),
        challenge=_required_string(body.get("challenge"), "challenge"),
    )


def _normalize_participant_authority(value: object) -> WalletAuthority:
    body = _mapping(value, "Participant approval authority")
    selected = [
        (scheme, body.get(scheme))
        for scheme in ("ed25519", "secp256k1", "secp256r1")
        if body.get(scheme) is not None
    ]
    if len(selected) != 1:
        raise ValueError("Participant approval request has invalid authority")
    scheme, authority_value = selected[0]
    authority = _mapping(authority_value, "Participant approval authority value")
    public_key = _required_string(
        _pick(authority, "publicKey", "public_key"), "authority.publicKey"
    )
    return {scheme: {"publicKey": public_key}}


def _prepared_transaction_to_wire(
    transaction: PreparedTransaction,
) -> dict[str, object]:
    plan = transaction.participant_set_approval_plan
    return {
        "transaction": transaction.transaction,
        "transactionEncoding": (
            "TRANSACTION_ENCODING_BASE64"
            if transaction.transaction_encoding is not None
            else None
        ),
        "wallet": (
            {
                "swigConfigAddress": transaction.wallet.swig_config_address,
                "walletAddress": transaction.wallet.wallet_address,
            }
            if transaction.wallet is not None
            else None
        ),
        "expiresAt": transaction.expires_at,
        "network": _network_to_wire(transaction.network),
        "recentBlockhash": transaction.recent_blockhash,
        "kind": _prepared_transaction_kind_to_wire(transaction.kind),
        "signatureRequests": [
            {
                "scheme": (
                    "AUTHORITY_SIGNATURE_SCHEME_SECP256R1"
                    if request.scheme == "secp256r1"
                    else "AUTHORITY_SIGNATURE_SCHEME_SECP256K1"
                ),
                "signer": request.signer,
                "messageHash": request.message_hash,
                "slot": str(request.slot),
                "counter": request.counter,
            }
            for request in transaction.signature_requests
        ],
        "participantSetApprovalPlan": (
            {
                "participantSetAddress": plan.participant_set_address,
                "roleId": plan.role_id,
                "expirationSlot": plan.expiration_slot,
                "nonce": plan.nonce,
                "transactionDigest": plan.transaction_digest,
                "compilationEnvelope": plan.compilation_envelope,
                "threshold": plan.threshold,
                "members": [
                    {
                        "memberIndex": member.member_index,
                        "authority": wallet_authority_to_wire(member.authority),
                        "challenge": member.challenge,
                    }
                    for member in plan.members
                ],
            }
            if plan is not None
            else None
        ),
    }


def _participant_approval_to_wire(
    approval: ParticipantSetApproval,
) -> dict[str, object]:
    proof: dict[str, object]
    if isinstance(approval, Ed25519ParticipantApproval):
        proof = {
            "ed25519": {
                "signature": base64.b64encode(approval.signature).decode("ascii")
            }
        }
    elif isinstance(approval, Secp256k1ParticipantApproval):
        proof = {
            "secp256k1": {
                "signature": base64.b64encode(approval.signature).decode("ascii")
            }
        }
    else:
        proof = {
            "webauthnP256": {
                "authenticatorData": base64.b64encode(
                    approval.authenticator_data
                ).decode("ascii"),
                "clientDataJson": base64.b64encode(approval.client_data_json).decode(
                    "ascii"
                ),
                "signature": base64.b64encode(approval.signature).decode("ascii"),
            }
        }
    return {
        "memberIndex": approval.member_index,
        **proof,
    }


def _network_to_wire(network: Network | None) -> str | None:
    if network is None:
        return None
    return "NETWORK_MAINNET" if network == "mainnet" else "NETWORK_DEVNET"


def _prepared_transaction_kind_to_wire(
    kind: PreparedTransactionKind | None,
) -> str | None:
    values = {
        "create-swig-wallet": "PREPARED_TRANSACTION_KIND_CREATE_SWIG_WALLET",
        "add-authority": "PREPARED_TRANSACTION_KIND_ADD_AUTHORITY",
        "configure-recovery": "PREPARED_TRANSACTION_KIND_CONFIGURE_RECOVERY",
        "create-participant-set": "PREPARED_TRANSACTION_KIND_CREATE_PARTICIPANT_SET",
    }
    return values.get(kind) if kind is not None else None


def _mapping(value: object, label: str) -> Mapping[str, object]:
    if isinstance(value, Mapping):
        return value
    raise ValueError(f"{label} must be an object")


def _required_string(value: object, field: str) -> str:
    if not isinstance(value, str) or not value:
        raise ValueError(f"Response is missing {field}")
    return value


def _required_int(value: object, field: str) -> int:
    if isinstance(value, int):
        return value
    if isinstance(value, str):
        try:
            return int(value, 10)
        except ValueError:
            pass
    raise ValueError(f"Response is missing {field}")


def _pick(value: Mapping[str, object], *keys: str) -> object:
    for key in keys:
        if key in value:
            return value[key]
    return None


def _optional_string(value: object) -> str | None:
    return value if isinstance(value, str) else None
