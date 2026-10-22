from __future__ import annotations

import json
from collections.abc import Callable
from urllib.parse import parse_qs, urlparse

import httpx
import pytest

from swig_developer_sdk import (
    ExistingYieldRole,
    PartialYieldExit,
    PreparedYieldRoleCreation,
    SwigClient,
)
from swig_developer_sdk.wallets import (
    ProgramAction,
    SolLimitAction,
    StakeAllAction,
    StakeRecurringLimitAction,
)

CONFIG = "Conf1g11111111111111111111111111111111111111"
PAYER = "Payer111111111111111111111111111111111111111"
VOTE = "Vote111111111111111111111111111111111111111"
POSITION = "Pos11111111111111111111111111111111111111111"
AUTHORITY = {"ed25519": {"public_key": "Auth1111111111111111111111111111111111111111"}}
PREPARED = {"transaction": "AQID", "kind": "PREPARED_TRANSACTION_KIND_YIELD_DEPOSIT"}


def client(
    reply: Callable[[httpx.Request], object], network: str | None = "devnet"
) -> tuple[SwigClient, list[httpx.Request]]:
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        return httpx.Response(200, json={"data": reply(request)})

    swig = SwigClient(
        api_key="secret",
        base_url="https://example.test",
        network=network,  # type: ignore[arg-type]
        transport=httpx.MockTransport(handler),
    )
    return swig, requests


def body(request: httpx.Request) -> dict[str, object]:
    return json.loads(request.content)  # type: ignore[no-any-return]


async def test_deposit_posts_the_wallet_scoped_request() -> None:
    swig, requests = client(
        lambda _: {
            "transaction": PREPARED,
            "positionAddress": POSITION,
            "validatorVoteAddress": VOTE,
            "delegatedLamports": "1000000000",
            "rentLamports": "2282880",
        }
    )
    deposit = await swig.yields.deposit(
        swig_config_address=CONFIG,
        fee_payer=PAYER,
        requester_authority=AUTHORITY,  # type: ignore[arg-type]
        amount=1_000_000_000,
        validator_vote_address=VOTE,
        position_seed="deposit-1",
        role_id=3,
    )
    request = requests[0]
    assert request.method == "POST"
    assert request.url.path == f"/api/yield/{CONFIG}/deposit/prepare"
    sent = body(request)
    assert sent["lamports"] == "1000000000"
    assert sent["positionSeed"] == "deposit-1"
    wallet = sent["wallet"]
    assert isinstance(wallet, dict)
    assert wallet["network"] == "NETWORK_DEVNET"
    assert wallet["roleId"] == 3
    assert wallet["swigConfigAddress"] == CONFIG
    assert "requesterAuthority" in wallet
    assert deposit.position_address == POSITION
    assert deposit.rent_lamports == "2282880"
    assert deposit.transaction.transaction == "AQID"


async def test_exit_encodes_all_and_partial_amounts() -> None:
    swig, requests = client(
        lambda _: {
            "transaction": PREPARED,
            "exiting_position_address": POSITION,
            "deactivating_lamports": 3,
        }
    )
    common = dict(
        swig_config_address=CONFIG,
        fee_payer=PAYER,
        requester_authority=AUTHORITY,
        position_address=POSITION,
    )
    full = await swig.yields.exit(amount="all", **common)  # type: ignore[arg-type]
    await swig.yields.exit(
        amount=PartialYieldExit(lamports=5, position_seed="exit-1"),
        **common,  # type: ignore[arg-type]
    )
    assert body(requests[0])["all"] == {}
    assert "roleId" not in body(requests[0])["wallet"]  # type: ignore[operator]
    assert body(requests[1])["partial"] == {"lamports": "5", "positionSeed": "exit-1"}
    assert full.exiting_position_address == POSITION
    assert full.deactivating_lamports == "3"
    assert full.additional_rent_lamports == "0"


async def test_withdraw_returns_the_destination_wallet() -> None:
    swig, requests = client(
        lambda _: {
            "transaction": PREPARED,
            "withdrawalLamports": "5000000000",
            "destinationWalletAddress": "Wallet",
        }
    )
    withdrawal = await swig.yields.withdraw(
        swig_config_address=CONFIG,
        fee_payer=PAYER,
        requester_authority=AUTHORITY,  # type: ignore[arg-type]
        position_address=POSITION,
    )
    assert requests[0].url.path.endswith("/withdraw/prepare")
    assert withdrawal.withdrawal_lamports == "5000000000"
    assert withdrawal.destination_wallet_address == "Wallet"


async def test_prepare_role_decodes_existing_and_created_roles() -> None:
    replies: list[object] = [
        {
            "existing": {
                "roleId": 2,
                "actions": [
                    {"program": {"programId": "11111111111111111111111111111111"}},
                    {"solLimit": {"amount": "9"}},
                    {"stake_all": {}},
                    {"stakeRecurringLimit": {"recurringAmount": "5", "window": "100"}},
                ],
            }
        },
        {
            "create": {
                "transaction": PREPARED,
                "yieldAuthority": AUTHORITY,
                "actions": [{"stakeAll": {}}],
            }
        },
        {},
    ]
    swig, requests = client(lambda _: replies.pop(0))
    args = dict(
        swig_config_address=CONFIG,
        fee_payer=PAYER,
        yield_authority=AUTHORITY,
        validator_vote_address=VOTE,
        max_deposit=5_000_000_000,
        role_manager_authority=AUTHORITY,
    )
    existing = await swig.yields.prepare_role(**args)  # type: ignore[arg-type]
    sent = body(requests[0])
    assert requests[0].url.path == f"/api/yield/{CONFIG}/role/prepare"
    assert sent["maxDepositLamports"] == "5000000000"
    assert "roleManagerAuthority" in sent and "roleId" not in sent
    assert isinstance(existing, ExistingYieldRole)
    assert existing.role_id == 2
    assert existing.actions == [
        ProgramAction(program_id="11111111111111111111111111111111"),
        SolLimitAction(amount="9"),
        StakeAllAction(),
        StakeRecurringLimitAction(recurring_amount="5", window="100"),
    ]
    created = await swig.yields.prepare_role(**args)  # type: ignore[arg-type]
    assert isinstance(created, PreparedYieldRoleCreation)
    assert created.transaction.transaction == "AQID"
    assert created.actions == [StakeAllAction()]
    with pytest.raises(Exception, match="neither existing nor create"):
        await swig.yields.prepare_role(**args)  # type: ignore[arg-type]


async def test_queries_use_get_with_query_strings() -> None:
    def reply(request: httpx.Request) -> object:
        path = request.url.path
        if path.endswith("/positions"):
            return {
                "positions": [
                    {
                        "positionAddress": POSITION,
                        "state": "YIELD_POSITION_STATE_ACTIVE",
                        "balanceLamports": "12",
                        "positionLamports": "10",
                    },
                    {"position_address": "Other", "state": 4},
                ],
                "nextPageToken": "next",
                "lastUpdatedAt": "2026-10-22T00:00:00Z",
            }
        if path.endswith("/summary"):
            return {"totalPositionLamports": "10", "recorded_rewards_lamports": "1"}
        if path.endswith("/rewards"):
            return {
                "rewards": [
                    {
                        "positionAddress": POSITION,
                        "amountLamports": "7",
                        "earnedAt": "t",
                    }
                ]
            }
        if path.endswith("/options"):
            return {
                "options": [
                    {
                        "voteAddress": VOTE,
                        "name": "Anagram",
                        "acceptsDeposits": True,
                        "rate": {"estimatedNetAprBps": 665, "commissionBps": 500},
                    }
                ]
            }
        return {
            "projection": {
                "principalLamports": "100",
                "horizonDays": 30,
                "estimatedRewardsLamports": "1",
                "estimatedTotalLamports": "101",
            }
        }

    swig, requests = client(reply)
    positions = await swig.yields.list_positions(
        swig_config_address=CONFIG, page_size=2, page_token="abc"
    )
    query = parse_qs(urlparse(str(requests[0].url)).query)
    assert requests[0].method == "GET"
    assert query == {
        "network": ["NETWORK_DEVNET"],
        "pageSize": ["2"],
        "pageToken": ["abc"],
    }
    assert [p.state for p in positions.positions] == ["active", "inactive"]
    assert positions.positions[1].balance_lamports == "0"
    assert positions.next_page_token == "next"

    summary = await swig.yields.get_summary(swig_config_address=CONFIG)
    assert (summary.total_position_lamports, summary.withdrawable_lamports) == (
        "10",
        "0",
    )
    rewards = await swig.yields.list_rewards(swig_config_address=CONFIG)
    assert rewards.rewards[0].amount_lamports == "7"
    assert rewards.next_page_token is None
    options = await swig.yields.list_options(swig_config_address=CONFIG)
    assert options[0].accepts_deposits is True
    assert options[0].rate is not None and options[0].rate.estimated_net_apr_bps == 665
    projection = await swig.yields.estimate(
        swig_config_address=CONFIG,
        validator_vote_address=VOTE,
        principal=100,
        horizon_days=30,
    )
    estimate_query = parse_qs(urlparse(str(requests[4].url)).query)
    assert estimate_query["principalLamports"] == ["100"]
    assert projection.estimated_total_lamports == "101"


async def test_validates_inputs_before_requests() -> None:
    swig, requests = client(lambda _: {}, network=None)
    with pytest.raises(Exception, match="network"):
        await swig.yields.get_summary(swig_config_address=CONFIG)
    with pytest.raises(ValueError, match="page_size"):
        await swig.yields.list_positions(
            swig_config_address=CONFIG, network="devnet", page_size=201
        )
    with pytest.raises(ValueError, match="horizon_days"):
        await swig.yields.estimate(
            swig_config_address=CONFIG,
            network="devnet",
            validator_vote_address=VOTE,
            principal=1,
            horizon_days=0,
        )
    with pytest.raises(ValueError, match="swig_config_address"):
        await swig.yields.list_options(swig_config_address="", network="devnet")
    assert requests == []
