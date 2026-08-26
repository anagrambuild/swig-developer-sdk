from __future__ import annotations

from swig_developer_sdk import (
    Ed25519ParticipantApproval,
    ParticipantApprovalRequest,
    Secp256k1ParticipantApproval,
    WebAuthnAssertion,
    WebAuthnP256ParticipantApproval,
)
from swig_developer_sdk.signers import (
    create_participant_ed25519_signer,
    create_participant_passkey_signer,
    create_participant_personal_sign_signer,
    sign_participant_set_approval,
)

SECP256K1_ORDER = int(
    "fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141",
    16,
)


async def test_personal_signer_signs_lowercase_ascii_and_normalizes_low_s() -> None:
    public_key = "02" + "11" * 32
    challenge = "AB" * 32
    messages: list[str] = []
    signature = (
        (1).to_bytes(32, "big")
        + (SECP256K1_ORDER - 2).to_bytes(32, "big")
        + bytes([27])
    )

    def sign_message(message: str) -> bytes:
        messages.append(message)
        return signature

    approval = await sign_participant_set_approval(
        ParticipantApprovalRequest(
            member_index=3,
            authority={"secp256k1": {"publicKey": public_key.upper()}},
            challenge=challenge,
        ),
        create_participant_personal_sign_signer(
            public_key=public_key,
            sign_message=sign_message,
        ),
    )

    assert messages == [challenge.lower()]
    assert isinstance(approval, Secp256k1ParticipantApproval)
    assert approval.member_index == 3
    assert approval.signature == (
        (1).to_bytes(32, "big") + (2).to_bytes(32, "big") + bytes([28])
    )


async def test_ed25519_signer_signs_decoded_challenge_bytes() -> None:
    public_key = "ed25519-public-key"
    challenge = "44" * 32
    messages: list[bytes] = []
    signature = bytes(range(64))

    def sign_message(message: bytes) -> bytes:
        messages.append(message)
        return signature

    approval = await sign_participant_set_approval(
        ParticipantApprovalRequest(
            member_index=2,
            authority={"ed25519": {"publicKey": public_key}},
            challenge=challenge,
        ),
        create_participant_ed25519_signer(
            public_key=public_key,
            sign_message=sign_message,
        ),
    )

    assert messages == [bytes.fromhex(challenge)]
    assert isinstance(approval, Ed25519ParticipantApproval)
    assert approval.member_index == 2
    assert approval.signature == signature


async def test_passkey_signer_returns_raw_assertion_for_bound_challenge() -> None:
    public_key = "03" + "22" * 32
    challenge = "33" * 32
    challenges: list[bytes] = []
    authenticator_data = b"authenticator-data"
    client_data_json = b'{"type":"webauthn.get","challenge":"bound"}'
    der_signature = bytes.fromhex("3006020101020102")

    async def get_assertion(challenge_bytes: bytes) -> WebAuthnAssertion:
        challenges.append(challenge_bytes)
        return WebAuthnAssertion(
            authenticator_data=authenticator_data,
            client_data_json=client_data_json,
            signature=der_signature,
        )

    approval = await sign_participant_set_approval(
        ParticipantApprovalRequest(
            member_index=1,
            authority={"secp256r1": {"publicKey": public_key}},
            challenge=challenge,
        ),
        create_participant_passkey_signer(
            public_key=public_key,
            get_assertion=get_assertion,
        ),
    )

    assert challenges == [bytes.fromhex(challenge)]
    assert isinstance(approval, WebAuthnP256ParticipantApproval)
    assert approval.authenticator_data == authenticator_data
    assert approval.client_data_json == client_data_json
    assert approval.signature == bytes(31) + b"\x01" + bytes(31) + b"\x02"


async def test_signing_rejects_another_member_key_before_callback() -> None:
    called = False

    def sign_message(_: str) -> bytes:
        nonlocal called
        called = True
        return bytes(65)

    signer = create_participant_personal_sign_signer(
        public_key="02" + "11" * 32,
        sign_message=sign_message,
    )
    try:
        await sign_participant_set_approval(
            ParticipantApprovalRequest(
                member_index=0,
                authority={"secp256k1": {"publicKey": "02" + "22" * 32}},
                challenge="33" * 32,
            ),
            signer,
        )
    except ValueError as error:
        assert str(error) == (
            "Participant signer public key does not match approval request"
        )
    else:
        raise AssertionError("mismatched participant key was accepted")
    assert not called
