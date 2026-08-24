from __future__ import annotations

import json

import httpx

from swig_developer_sdk import (
    ParticipantApprovalRequest,
    ParticipantSetApprovalPlan,
    PreparedTransaction,
    Secp256k1ParticipantApproval,
    Secp256k1ParticipantSetMember,
    SwigClient,
    WebAuthnP256ParticipantSetMember,
)
from swig_developer_sdk.transactions import normalize_prepared_transaction


def test_normalizes_participant_set_approval_plan() -> None:
    prepared = normalize_prepared_transaction(
        {
            "transaction": "prepared-base64",
            "participantSetApprovalPlan": {
                "participantSetAddress": "participant-set-123",
                "roleId": 4,
                "expirationSlot": "12345",
                "transactionDigest": "11" * 32,
                "compilationEnvelope": "envelope-123",
                "threshold": 2,
                "members": [
                    {
                        "memberIndex": 1,
                        "signerType": ("PARTICIPANT_SET_SIGNER_TYPE_WEBAUTHN_P256"),
                        "publicKey": "03" + "22" * 32,
                        "counter": 9,
                        "challenge": "33" * 32,
                    }
                ],
            },
        }
    )

    assert prepared.participant_set_approval_plan is not None
    assert prepared.participant_set_approval_plan.expiration_slot == "12345"
    assert prepared.participant_set_approval_plan.members[0].signer_type == (
        "webauthnP256"
    )


async def test_create_mixed_participant_set_uses_typed_resource() -> None:
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        return httpx.Response(
            200,
            json={
                "data": {
                    "participantSetAddress": "participant-set-123",
                    "setId": "set-id-123",
                    "transaction": {
                        "transaction": "prepared-base64",
                        "transactionEncoding": "TRANSACTION_ENCODING_BASE64",
                        "network": "NETWORK_DEVNET",
                        "kind": "PREPARED_TRANSACTION_KIND_CREATE_PARTICIPANT_SET",
                    },
                }
            },
        )

    swig = SwigClient(
        api_key="secret",
        base_url="https://example.test",
        network="devnet",
        transport=httpx.MockTransport(handler),
    )
    created = await swig.participant_sets.create(
        swig_config_address="swig-123",
        fee_payer="payer-123",
        threshold=2,
        members=(
            Secp256k1ParticipantSetMember(public_key="02" + "11" * 32),
            WebAuthnP256ParticipantSetMember(public_key="03" + "22" * 32),
        ),
        set_id="set-id-123",
    )

    assert created.participant_set_address == "participant-set-123"
    assert created.transaction.kind == "create-participant-set"
    assert requests[0].url.path == "/transaction/participant-set/create"
    assert json.loads(requests[0].content) == {
        "network": "NETWORK_DEVNET",
        "feePayer": "payer-123",
        "swigAddress": "swig-123",
        "setId": "set-id-123",
        "threshold": 2,
        "members": [
            {"secp256k1PublicKey": "02" + "11" * 32},
            {"webauthnP256PublicKey": "03" + "22" * 32},
        ],
    }


async def test_wallet_roles_add_maps_participant_set_authorities() -> None:
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        return httpx.Response(
            200,
            json={
                "data": {
                    "transaction": {
                        "transaction": "add-role-base64",
                        "transactionEncoding": "TRANSACTION_ENCODING_BASE64",
                    }
                }
            },
        )

    swig = SwigClient(
        api_key="secret",
        base_url="https://example.test",
        network="devnet",
        transport=httpx.MockTransport(handler),
    )
    wallet = swig.wallets.use(
        "swig-123",
        requester_authority={
            "participantSet": {
                "address": "participant-set-requester",
                "roleId": 9,
            }
        },
    )
    prepared = await wallet.roles.add(
        fee_payer="payer-123",
        participant_set_address="participant-set-new",
        permissions=({"all": {}},),
    )

    assert prepared.transaction == "add-role-base64"
    assert requests[0].url.path == "/transaction/wallet/role/add"
    assert json.loads(requests[0].content) == {
        "network": "NETWORK_DEVNET",
        "feePayer": "payer-123",
        "swigAddress": "swig-123",
        "requesterAuthority": {
            "participantSet": {
                "participantSetAddress": "participant-set-requester",
                "roleId": 9,
            }
        },
        "authority": {
            "participantSet": {
                "participantSetAddress": "participant-set-new",
            }
        },
        "actions": [{"all": {}}],
    }


async def test_compile_participant_approvals_preserves_bound_plan() -> None:
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        return httpx.Response(
            200,
            json={
                "data": {
                    "transaction": {
                        "transaction": "compiled-base64",
                        "transactionEncoding": "TRANSACTION_ENCODING_BASE64",
                        "network": "NETWORK_DEVNET",
                    }
                }
            },
        )

    swig = SwigClient(
        api_key="secret",
        base_url="https://example.test",
        network="devnet",
        transport=httpx.MockTransport(handler),
    )
    prepared = PreparedTransaction(
        transaction="original-base64",
        transaction_encoding="base64",
        network="devnet",
        signature_requests=(),
        participant_set_approval_plan=ParticipantSetApprovalPlan(
            participant_set_address="participant-set-123",
            role_id=4,
            expiration_slot="12345",
            transaction_digest="11" * 32,
            compilation_envelope="envelope-123",
            threshold=2,
            members=(
                ParticipantApprovalRequest(
                    member_index=0,
                    signer_type="secp256k1",
                    public_key="02" + "22" * 32,
                    counter=7,
                    challenge="33" * 32,
                ),
            ),
        ),
    )
    compiled = await swig.transactions.compile_participant_set_approvals(
        prepared_transaction=prepared,
        approvals=(
            Secp256k1ParticipantApproval(
                member_index=0,
                counter=7,
                signature=b"\x01\x02\x03",
            ),
        ),
    )

    assert compiled.prepared_transaction.transaction == "compiled-base64"
    assert requests[0].url.path == "/transaction/participant-set/compile"
    body = json.loads(requests[0].content)
    assert body["preparedTransaction"]["participantSetApprovalPlan"] == {
        "participantSetAddress": "participant-set-123",
        "roleId": 4,
        "expirationSlot": "12345",
        "transactionDigest": "11" * 32,
        "compilationEnvelope": "envelope-123",
        "threshold": 2,
        "members": [
            {
                "memberIndex": 0,
                "signerType": "PARTICIPANT_SET_SIGNER_TYPE_SECP256K1",
                "publicKey": "02" + "22" * 32,
                "counter": 7,
                "challenge": "33" * 32,
            }
        ],
    }
    assert body["approvals"] == [
        {
            "memberIndex": 0,
            "counter": 7,
            "secp256k1": {"signature": "AQID"},
        }
    ]
