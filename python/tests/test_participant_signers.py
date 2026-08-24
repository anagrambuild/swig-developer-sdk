from __future__ import annotations

from swig_developer_sdk import ParticipantApprovalRequest, WebAuthnAssertion
from swig_developer_sdk.signers import (
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
            signer_type="secp256k1",
            public_key=public_key.upper(),
            counter=7,
            challenge=challenge,
        ),
        create_participant_personal_sign_signer(
            public_key=public_key,
            sign_message=sign_message,
        ),
    )

    assert messages == [challenge.lower()]
    assert approval.member_index == 3
    assert approval.counter == 7
    assert approval.signature == (
        (1).to_bytes(32, "big") + (2).to_bytes(32, "big") + bytes([28])
    )


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
            signer_type="webauthnP256",
            public_key=public_key,
            counter=4,
            challenge=challenge,
        ),
        create_participant_passkey_signer(
            public_key=public_key,
            get_assertion=get_assertion,
        ),
    )

    assert challenges == [bytes.fromhex(challenge)]
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
                signer_type="secp256k1",
                public_key="02" + "22" * 32,
                counter=0,
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
