from __future__ import annotations

import inspect
from collections.abc import Awaitable, Callable, Mapping
from dataclasses import dataclass, field
from typing import Literal, Protocol, TypeAlias

from .passkeys import (
    WebAuthnAssertionFn,
    secp256r1_der_to_raw_signature,
)
from .transactions import (
    Ed25519ParticipantApproval,
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
ParticipantEd25519SignFn: TypeAlias = Callable[[bytes], bytes | Awaitable[bytes]]
ParticipantSignerType: TypeAlias = Literal["ed25519", "secp256k1", "secp256r1"]


class ParticipantSigner(Protocol):
    type: ParticipantSignerType
    public_key: str

    async def sign(
        self, request: ParticipantApprovalRequest
    ) -> ParticipantSetApproval: ...


@dataclass(frozen=True, slots=True)
class ParticipantEd25519Signer:
    public_key: str
    sign_message: ParticipantEd25519SignFn = field(repr=False)
    type: Literal["ed25519"] = field(default="ed25519", init=False)

    async def sign(
        self, request: ParticipantApprovalRequest
    ) -> Ed25519ParticipantApproval:
        _assert_matching_request(request, self.type, self.public_key)
        signature = self.sign_message(_challenge_bytes(request.challenge))
        if inspect.isawaitable(signature):
            signature = await signature
        if len(signature) != 64:
            raise ValueError("Participant ed25519 signature must be 64 bytes")
        return Ed25519ParticipantApproval(
            member_index=request.member_index,
            signature=bytes(signature),
        )


@dataclass(frozen=True, slots=True)
class ParticipantPasskeySigner:
    public_key: str
    get_assertion: WebAuthnAssertionFn = field(repr=False)
    type: Literal["secp256r1"] = field(default="secp256r1", init=False)

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
            signature=_normalize_secp256k1_signature(signature),
        )


def create_participant_ed25519_signer(
    *,
    public_key: str,
    sign_message: ParticipantEd25519SignFn,
) -> ParticipantEd25519Signer:
    return ParticipantEd25519Signer(
        public_key=public_key,
        sign_message=sign_message,
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
    if approval.member_index != request.member_index:
        raise ValueError("Participant signer returned approval for another request")
    if (
        (
            signer.type == "ed25519"
            and not isinstance(approval, Ed25519ParticipantApproval)
        )
        or (
            signer.type == "secp256k1"
            and not isinstance(approval, Secp256k1ParticipantApproval)
        )
        or (
            signer.type == "secp256r1"
            and not isinstance(approval, WebAuthnP256ParticipantApproval)
        )
    ):
        raise ValueError("Participant signer returned the wrong proof type")
    return approval


def _assert_matching_request(
    request: ParticipantApprovalRequest,
    signer_type: ParticipantSignerType,
    public_key: str,
) -> None:
    authority_type, authority_public_key = _participant_approval_authority(request)
    if authority_type != signer_type:
        raise ValueError("Participant signer type does not match approval request")
    matches = (
        authority_public_key == public_key
        if signer_type == "ed25519"
        else _normalize_hex(authority_public_key) == _normalize_hex(public_key)
    )
    if not matches:
        raise ValueError(
            "Participant signer public key does not match approval request"
        )


def _participant_approval_authority(
    request: ParticipantApprovalRequest,
) -> tuple[ParticipantSignerType, str]:
    selected: list[tuple[ParticipantSignerType, object]] = []
    for scheme in ("ed25519", "secp256k1", "secp256r1"):
        authority = request.authority.get(scheme)
        if authority is not None:
            selected.append((scheme, authority))
    if len(selected) != 1:
        raise ValueError("Participant approval request has invalid authority")
    scheme, selected_authority = selected[0]
    if not isinstance(selected_authority, Mapping):
        raise ValueError("Participant approval request has invalid authority")
    public_key = selected_authority.get(
        "publicKey", selected_authority.get("public_key")
    )
    if not isinstance(public_key, str) or not public_key:
        raise ValueError("Participant approval request authority is missing publicKey")
    return scheme, public_key


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
