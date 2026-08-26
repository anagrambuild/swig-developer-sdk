from __future__ import annotations

import json

import httpx
import pytest

from swig_developer_sdk import (
    AllAction,
    AllButManageAuthorityAction,
    Ed25519ParticipantApproval,
    ManageAuthorityAction,
    ParticipantApprovalRequest,
    ParticipantSetApprovalPlan,
    PreparedTransaction,
    ProgramAction,
    ProgramAllAction,
    ProgramCuratedAction,
    Secp256k1ParticipantApproval,
    SolDestinationLimitAction,
    SolLimitAction,
    SolRecurringDestinationLimitAction,
    SolRecurringLimitAction,
    StakeAllAction,
    StakeLimitAction,
    StakeRecurringLimitAction,
    SubAccountAction,
    SwigClient,
    TokenDestinationLimitAction,
    TokenLimitAction,
    TokenRecurringDestinationLimitAction,
    TokenRecurringLimitAction,
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
                "nonce": 9,
                "transactionDigest": "11" * 32,
                "compilationEnvelope": "envelope-123",
                "threshold": 2,
                "members": [
                    {
                        "memberIndex": 1,
                        "authority": {"secp256r1": {"publicKey": "03" + "22" * 32}},
                        "challenge": "33" * 32,
                    }
                ],
            },
        }
    )

    assert prepared.participant_set_approval_plan is not None
    assert prepared.participant_set_approval_plan.expiration_slot == "12345"
    assert prepared.participant_set_approval_plan.nonce == 9
    assert prepared.participant_set_approval_plan.members[0].authority == {
        "secp256r1": {"publicKey": "03" + "22" * 32}
    }


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
            {"ed25519": {"publicKey": "ed25519-public-key"}},
            {"secp256k1": {"publicKey": "02" + "11" * 32}},
            {"secp256r1": {"publicKey": "03" + "22" * 32}},
        ),
        set_id="set-id-123",
    )

    assert created.participant_set_address == "participant-set-123"
    assert created.transaction.kind == "create-participant-set"
    assert requests[0].url.path == "/transaction/wallet/participant-set/create"
    assert json.loads(requests[0].content) == {
        "network": "NETWORK_DEVNET",
        "feePayer": "payer-123",
        "swigAddress": "swig-123",
        "setId": "set-id-123",
        "threshold": 2,
        "members": [
            {"ed25519": {"publicKey": "ed25519-public-key"}},
            {"secp256k1": {"publicKey": "02" + "11" * 32}},
            {"secp256r1": {"publicKey": "03" + "22" * 32}},
        ],
    }


async def test_wallet_roles_add_maps_general_authorities_and_actions() -> None:
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
        requester_authority={"ed25519": {"publicKey": "requester-public-key"}},
    )
    prepared = await wallet.roles.add(
        fee_payer="payer-123",
        authority={
            "participantSet": {"address": "participant-set-new"},
        },
        actions=(
            AllAction(),
            AllButManageAuthorityAction(),
            ManageAuthorityAction(),
            SolLimitAction(amount=1_000_000),
            SolRecurringLimitAction(recurring_amount=2, window=3),
            SolDestinationLimitAction(amount=4, destination="sol-destination"),
            SolRecurringDestinationLimitAction(
                recurring_amount=5,
                window=6,
                destination="recurring-sol-destination",
            ),
            TokenLimitAction(mint="mint-1", amount=7),
            TokenRecurringLimitAction(mint="mint-2", recurring_amount=8, window=9),
            TokenDestinationLimitAction(
                mint="mint-3", amount=10, destination="token-destination"
            ),
            TokenRecurringDestinationLimitAction(
                mint="mint-4",
                recurring_amount=11,
                window=12,
                destination="recurring-token-destination",
            ),
            ProgramAction(program_id="program-123"),
            ProgramAllAction(),
            ProgramCuratedAction(),
            StakeLimitAction(amount=13),
            StakeRecurringLimitAction(recurring_amount=14, window=15),
            StakeAllAction(),
            SubAccountAction(),
        ),
    )

    assert prepared.transaction == "add-role-base64"
    assert requests[0].url.path == "/transaction/wallet/role/add"
    assert json.loads(requests[0].content) == {
        "network": "NETWORK_DEVNET",
        "feePayer": "payer-123",
        "swigAddress": "swig-123",
        "requesterAuthority": {"ed25519": {"publicKey": "requester-public-key"}},
        "authority": {
            "participantSet": {
                "participantSetAddress": "participant-set-new",
            }
        },
        "actions": [
            {"all": {}},
            {"allButManageAuthority": {}},
            {"manageAuthority": {}},
            {"solLimit": {"amount": "1000000"}},
            {"solRecurringLimit": {"recurringAmount": "2", "window": "3"}},
            {
                "solDestinationLimit": {
                    "amount": "4",
                    "destination": "sol-destination",
                }
            },
            {
                "solRecurringDestinationLimit": {
                    "recurringAmount": "5",
                    "window": "6",
                    "destination": "recurring-sol-destination",
                }
            },
            {"tokenLimit": {"mint": "mint-1", "amount": "7"}},
            {
                "tokenRecurringLimit": {
                    "mint": "mint-2",
                    "recurringAmount": "8",
                    "window": "9",
                }
            },
            {
                "tokenDestinationLimit": {
                    "mint": "mint-3",
                    "amount": "10",
                    "destination": "token-destination",
                }
            },
            {
                "tokenRecurringDestinationLimit": {
                    "mint": "mint-4",
                    "recurringAmount": "11",
                    "window": "12",
                    "destination": "recurring-token-destination",
                }
            },
            {"program": {"programId": "program-123"}},
            {"programAll": {}},
            {"programCurated": {}},
            {"stakeLimit": {"amount": "13"}},
            {
                "stakeRecurringLimit": {
                    "recurringAmount": "14",
                    "window": "15",
                }
            },
            {"stakeAll": {}},
            {"subAccount": {}},
        ],
    }


async def test_wallet_roles_add_rejects_participant_set_requester() -> None:
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        return httpx.Response(500)

    swig = SwigClient(
        api_key="secret",
        base_url="https://example.test",
        network="devnet",
        transport=httpx.MockTransport(handler),
    )
    wallet = swig.wallets.use(
        "swig-123",
        requester_authority={
            "participantSet": {"address": "participant-set-requester"}
        },
    )

    with pytest.raises(
        ValueError,
        match="Add role requester_authority must use ed25519 or secp256r1",
    ):
        await wallet.roles.add(
            fee_payer="payer-123",
            authority={"ed25519": {"publicKey": "role-public-key"}},
            actions=(AllAction(),),
        )

    assert requests == []


async def test_wallet_roles_add_rejects_participant_set_role_id() -> None:
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        return httpx.Response(500)

    swig = SwigClient(
        api_key="secret",
        base_url="https://example.test",
        network="devnet",
        transport=httpx.MockTransport(handler),
    )
    wallet = swig.wallets.use(
        "swig-123",
        requester_authority={"ed25519": {"publicKey": "requester-public-key"}},
    )

    with pytest.raises(
        ValueError,
        match="Add role ParticipantSet authority must omit role_id",
    ):
        await wallet.roles.add(
            fee_payer="payer-123",
            authority={
                "participantSet": {
                    "address": "participant-set-new",
                    "roleId": 7,
                }
            },
            actions=(AllAction(),),
        )

    assert requests == []


async def test_rejects_participant_set_from_unsupported_endpoint_shapes() -> None:
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        return httpx.Response(500)

    swig = SwigClient(
        api_key="secret",
        base_url="https://example.test",
        network="devnet",
        transport=httpx.MockTransport(handler),
    )
    participant_set = {"participantSet": {"address": "participant-set-requester"}}

    with pytest.raises(
        ValueError,
        match="initial_user does not support ParticipantSet authority",
    ):
        await swig.wallets.create(
            fee_payer="payer-123",
            initial_user=participant_set,
        )

    wallet = swig.wallets.use(
        "swig-123",
        requester_authority=participant_set,
    )
    with pytest.raises(
        ValueError,
        match="requester_authority does not support ParticipantSet authority",
    ):
        await wallet.swap.jupiter(
            fee_payer="payer-123",
            input_mint="input-mint",
            output_mint="output-mint",
            amount=1,
        )
    with pytest.raises(
        ValueError,
        match="requester_authority does not support ParticipantSet authority",
    ):
        await swig.wallets.add_recovery_authority(
            wallet,
            fee_payer="payer-123",
        )
    with pytest.raises(
        ValueError,
        match="requester_authority does not support ParticipantSet authority",
    ):
        await wallet.recovery.cancel(fee_payer="payer-123")
    with pytest.raises(
        ValueError,
        match=(
            "ParticipantSet requester_authority does not support "
            "address_lookup_table_accounts"
        ),
    ):
        await wallet.build_transaction(
            fee_payer="payer-123",
            instructions=(),
            address_lookup_table_accounts=("lookup-table",),
        )

    assert requests == []


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
                    },
                    "authorizationExpirationSlot": "12345",
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
            nonce=7,
            transaction_digest="11" * 32,
            compilation_envelope="envelope-123",
            threshold=2,
            members=(
                ParticipantApprovalRequest(
                    member_index=0,
                    authority={"secp256k1": {"publicKey": "02" + "22" * 32}},
                    challenge="33" * 32,
                ),
                ParticipantApprovalRequest(
                    member_index=1,
                    authority={"ed25519": {"publicKey": "ed25519-public-key"}},
                    challenge="44" * 32,
                ),
            ),
        ),
    )
    compiled = await swig.transactions.compile_participant_set_approvals(
        prepared_transaction=prepared,
        approvals=(
            Secp256k1ParticipantApproval(
                member_index=0,
                signature=b"\x01\x02\x03",
            ),
            Ed25519ParticipantApproval(
                member_index=1,
                signature=b"\x04\x05\x06",
            ),
        ),
    )

    assert compiled.transaction.transaction == "compiled-base64"
    assert compiled.authorization_expiration_slot == "12345"
    assert requests[0].url.path == "/transaction/wallet/participant-set/compile"
    body = json.loads(requests[0].content)
    assert body["preparedTransaction"]["participantSetApprovalPlan"] == {
        "participantSetAddress": "participant-set-123",
        "roleId": 4,
        "expirationSlot": "12345",
        "nonce": 7,
        "transactionDigest": "11" * 32,
        "compilationEnvelope": "envelope-123",
        "threshold": 2,
        "members": [
            {
                "memberIndex": 0,
                "authority": {"secp256k1": {"publicKey": "02" + "22" * 32}},
                "challenge": "33" * 32,
            },
            {
                "memberIndex": 1,
                "authority": {"ed25519": {"publicKey": "ed25519-public-key"}},
                "challenge": "44" * 32,
            },
        ],
    }
    assert body["approvals"] == [
        {
            "memberIndex": 0,
            "secp256k1": {"signature": "AQID"},
        },
        {
            "memberIndex": 1,
            "ed25519": {"signature": "BAUG"},
        },
    ]
