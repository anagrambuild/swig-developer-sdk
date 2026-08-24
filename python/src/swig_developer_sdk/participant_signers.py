from __future__ import annotations

import inspect
from collections.abc import Awaitable, Callable
from dataclasses import dataclass, field
from typing import Literal, Protocol, TypeAlias

from .passkeys import (
    WebAuthnAssertionFn,
    secp256r1_der_to_raw_signature,
)
from .transactions import (
    ParticipantApprovalRequest,
    ParticipantSetApproval,
    Secp256k1ParticipantApproval,
    WebAuthnP256ParticipantApproval,
)

SECP256K1_ORDER = int(
    "fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141",
    16,
)
SECP256K1_HALF_ORDER = SECP256K1_ORDER >> 1

ParticipantPersonalSignFn: TypeAlias = Callable[
    [str], str | bytes | Awaitable[str | bytes]
]


class ParticipantSigner(Protocol):
    type: Literal["secp256k1", "webauthnP256"]
    public_key: str

    async def sign(
        self, request: ParticipantApprovalRequest
    ) -> ParticipantSetApproval: ...


@dataclass(frozen=True, slots=True)
class ParticipantPasskeySigner:
    public_key: str
    get_assertion: WebAuthnAssertionFn = field(repr=False)
    type: Literal["webauthnP256"] = field(default="webauthnP256", init=False)

    async def sign(
        self, request: ParticipantApprovalRequest
    ) -> WebAuthnP256ParticipantApproval:
        _assert_matching_request(request, self.type, self.public_key)
        challenge = _challenge_bytes(request.challenge)
        assertion = self.get_assertion(challenge)
        if inspect.isawaitable(assertion):
            assertion = await assertion
        return WebAuthnP256ParticipantApproval(
            member_index=request.member_index,
            counter=request.counter,
            authenticator_data=assertion.authenticator_data,
            client_data_json=assertion.client_data_json,
            signature=secp256r1_der_to_raw_signature(assertion.signature),
        )


@dataclass(frozen=True, slots=True)
class ParticipantPersonalSignSigner:
    public_key: str
    sign_message: ParticipantPersonalSignFn = field(repr=False)
    type: Literal["secp256k1"] = field(default="secp256k1", init=False)

    async def sign(
        self, request: ParticipantApprovalRequest
    ) -> Secp256k1ParticipantApproval:
        _assert_matching_request(request, self.type, self.public_key)
        challenge = _normalize_hex(request.challenge)
        if len(challenge) != 64:
            raise ValueError("Participant challenge must be 32 bytes")
        signed = self.sign_message(challenge)
        if inspect.isawaitable(signed):
            signed = await signed
        signature = _signature_bytes(signed)
        return Secp256k1ParticipantApproval(
            member_index=request.member_index,
            counter=request.counter,
            signature=_normalize_secp256k1_signature(signature),
        )


def create_participant_passkey_signer(
    *,
    public_key: str,
    get_assertion: WebAuthnAssertionFn,
) -> ParticipantPasskeySigner:
    return ParticipantPasskeySigner(
        public_key=public_key,
        get_assertion=get_assertion,
    )


def create_participant_personal_sign_signer(
    *,
    public_key: str,
    sign_message: ParticipantPersonalSignFn,
) -> ParticipantPersonalSignSigner:
    return ParticipantPersonalSignSigner(
        public_key=public_key,
        sign_message=sign_message,
    )


async def sign_participant_set_approval(
    request: ParticipantApprovalRequest,
    signer: ParticipantSigner,
) -> ParticipantSetApproval:
    _assert_matching_request(request, signer.type, signer.public_key)
    approval = await signer.sign(request)
    if (
        approval.member_index != request.member_index
        or approval.counter != request.counter
    ):
        raise ValueError("Participant signer returned approval for another request")
    if (
        signer.type == "secp256k1"
        and not isinstance(approval, Secp256k1ParticipantApproval)
    ) or (
        signer.type == "webauthnP256"
        and not isinstance(approval, WebAuthnP256ParticipantApproval)
    ):
        raise ValueError("Participant signer returned the wrong proof type")
    return approval


def _assert_matching_request(
    request: ParticipantApprovalRequest,
    signer_type: Literal["secp256k1", "webauthnP256"],
    public_key: str,
) -> None:
    if request.signer_type != signer_type:
        raise ValueError("Participant signer type does not match approval request")
    if _normalize_hex(request.public_key) != _normalize_hex(public_key):
        raise ValueError(
            "Participant signer public key does not match approval request"
        )


def _challenge_bytes(challenge: str) -> bytes:
    normalized = _normalize_hex(challenge)
    if len(normalized) != 64:
        raise ValueError("Participant challenge must be 32 bytes")
    return bytes.fromhex(normalized)


def _signature_bytes(signature: str | bytes) -> bytes:
    if isinstance(signature, bytes):
        return signature
    return bytes.fromhex(_normalize_hex(signature))


def _normalize_secp256k1_signature(signature: bytes) -> bytes:
    if len(signature) != 65:
        raise ValueError("Participant secp256k1 signature must be 65 bytes")
    normalized = bytearray(signature)
    recovery = _normalize_recovery_byte(normalized[64])
    r = int.from_bytes(normalized[:32], "big")
    s = int.from_bytes(normalized[32:64], "big")
    if not 0 < r < SECP256K1_ORDER or not 0 < s < SECP256K1_ORDER:
        raise ValueError("Participant secp256k1 signature has invalid scalars")
    if s > SECP256K1_HALF_ORDER:
        s = SECP256K1_ORDER - s
        recovery = 28 if recovery == 27 else 27
        normalized[32:64] = s.to_bytes(32, "big")
    normalized[64] = recovery
    return bytes(normalized)


def _normalize_recovery_byte(value: int) -> Literal[27, 28]:
    if value in (0, 27):
        return 27
    if value in (1, 28):
        return 28
    raise ValueError("Participant secp256k1 signature has invalid recovery byte")


def _normalize_hex(value: str) -> str:
    normalized = value[2:] if value.startswith("0x") else value
    if not normalized or any(
        character not in "0123456789abcdefABCDEF" for character in normalized
    ):
        raise ValueError("Invalid hex string")
    return normalized.lower()
