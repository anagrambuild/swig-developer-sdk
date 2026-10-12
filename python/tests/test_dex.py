from __future__ import annotations

import copy
import json
from collections.abc import Callable
from pathlib import Path
from typing import Any

import httpx
import pytest

from swig_developer_sdk import (
    DexActionTypeFilter,
    DexAssetAmount,
    DexClosedPosition,
    DexEndSlotFilter,
    DexFeeClaimAction,
    DexLiquidityAddAction,
    DexLiquidityRemoveAction,
    DexOpenPosition,
    DexPositionCloseAction,
    DexPositionOpenAction,
    DexPositionRebalanceAction,
    DexPositionStatusFilter,
    DexProtocolFilter,
    DexRaydiumClmmPositionState,
    DexRewardClaimAction,
    DexStartSlotFilter,
    DexSwapAction,
    DexSwigVaultAddressFilter,
    DexTransactionAction,
    RetryOptions,
    SwigClient,
    SwigDeveloperSdkError,
    SwigResponseError,
)

FIXTURES = Path(__file__).resolve().parents[2] / "fixtures" / "dex"
SWIG = "8NkNvqXK6BfLAeqHvcwHWK6b6wybcraj2J3vZHm3LarX"
VAULT = "4k9xWkFvjifracRGbBUHeKg2z8qzJBPwjqjCySu28Mej"
POOL = "EYJkpMH4KJy8XDQyK59DM2xBUodvPkerDxKMCwyL1ber"


def capture(name: str) -> dict[str, Any]:
    """A response captured from the Developer API over mainnet data."""
    loaded: dict[str, Any] = json.loads((FIXTURES / f"{name}.json").read_text())
    return loaded


def client(
    handler: Callable[[httpx.Request], object],
    network: str | None = "devnet",
    retry_options: RetryOptions | None = None,
) -> SwigClient:
    """The handler returns a body to send as JSON, or a whole response."""

    def respond(request: httpx.Request) -> httpx.Response:
        reply = handler(request)
        if isinstance(reply, httpx.Response):
            return reply
        return httpx.Response(200, json=reply)

    return SwigClient(
        api_key="secret",
        base_url="https://example.test",
        network=network,  # type: ignore[arg-type]
        retry_options=retry_options,
        transport=httpx.MockTransport(respond),
    )


def unreachable(request: httpx.Request) -> object:
    raise AssertionError("no request should be sent")


def body(request: httpx.Request) -> object:
    return json.loads(request.content)


async def test_searches_transactions_with_every_filter_in_the_body() -> None:
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> object:
        requests.append(request)
        return capture("list_transactions_clmm_page")

    page = await client(handler).dex.transactions.list(
        swig_config_address=SWIG,
        network="mainnet",
        page_size=2,
        page_token="previous-token",
        filters=[
            DexStartSlotFilter(449_000_000),
            DexEndSlotFilter(450_000_000),
            DexProtocolFilter("raydium-clmm"),
            DexActionTypeFilter("liquidity-add"),
            DexSwigVaultAddressFilter(VAULT),
        ],
    )

    request = requests[0]
    assert request.method == "POST"
    assert request.url.path == f"/wallet/api/swig/{SWIG}/dex/transactions:search"
    assert request.url.query == b""
    assert body(request) == {
        "network": "NETWORK_MAINNET",
        "pageSize": 2,
        "pageToken": "previous-token",
        "filters": [
            {"startSlot": "449000000"},
            {"endSlot": "450000000"},
            {"protocol": "DEX_PROTOCOL_RAYDIUM_CLMM"},
            {"actionType": "DEX_ACTION_TYPE_LIQUIDITY_ADD"},
            {"swigVaultAddress": VAULT},
        ],
    }
    assert (
        page.next_page_token == capture("list_transactions_clmm_page")["nextPageToken"]
    )
    assert [transaction.slot for transaction in page.transactions] == [
        449761733,
        449761692,
    ]
    newest = page.transactions[0]
    assert newest.block_time == "2026-09-23T16:47:12Z"
    assert newest.actions == (
        DexTransactionAction(
            action_index=0,
            swig_vault_address=VAULT,
            acting_role_id=1,
            protocol="raydium-clmm",
            dex_program_address="CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK",
            action=DexLiquidityAddAction(
                protocol_instruction="IncreaseLiquidityV2",
                protocol_actor_address=VAULT,
                dex_pool_address=POOL,
                dex_position_address="GbqbbAaDmypYedYrm9srwvYmd54hAmzqqo527KCDwoXv",
                deposits=(
                    DexAssetAmount(
                        mint_address="USDvUSpnhCr9yBgj3UyVrD239HRUv4RsHwH2FxsWuMk",
                        amount_raw="-679914761680",
                        decimals=6,
                    ),
                ),
            ),
        ),
    )
    assert page.transactions[1].actions[0].action.type == "swap"


async def test_sends_only_the_client_network_when_no_filter_is_set() -> None:
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> object:
        requests.append(request)
        return {"transactions": [], "nextPageToken": ""}

    page = await client(handler).dex.transactions.list(swig_config_address=SWIG)

    assert body(requests[0]) == {"network": "NETWORK_DEVNET"}
    assert page.transactions == ()
    assert page.next_page_token is None


async def test_decodes_a_close_without_a_pool_authorized_by_role_zero() -> None:
    page = await client(
        lambda _: capture("list_transactions_position_close")
    ).dex.transactions.list(
        swig_config_address=SWIG, filters=[DexActionTypeFilter("position-close")]
    )

    assert page.next_page_token is None
    transaction = page.transactions[0]
    assert transaction.slot == 449615165
    assert transaction.actions == (
        DexTransactionAction(
            action_index=0,
            swig_vault_address=VAULT,
            acting_role_id=0,
            protocol="raydium-clmm",
            dex_program_address="CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK",
            action=DexPositionCloseAction(
                protocol_instruction="ClosePosition",
                protocol_actor_address=VAULT,
                dex_position_address="HpFVYnzYLrLPU5djgURwv7ydeaJWRGAjx3Xkd5ZNGaKp",
            ),
        ),
    )


async def test_decodes_a_captured_position_open_and_liquidity_removal() -> None:
    opened = await client(
        lambda _: capture("list_transactions_position_open")
    ).dex.transactions.list(
        swig_config_address=SWIG,
        page_size=1,
        filters=[DexActionTypeFilter("position-open")],
    )

    assert (
        opened.next_page_token
        == capture("list_transactions_position_open")["nextPageToken"]
    )
    assert [transaction.slot for transaction in opened.transactions] == [449753334]
    assert opened.transactions[0].actions == (
        DexTransactionAction(
            action_index=0,
            swig_vault_address=VAULT,
            acting_role_id=1,
            protocol="raydium-clmm",
            dex_program_address="CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK",
            action=DexPositionOpenAction(
                protocol_instruction="OpenPositionWithToken22Nft",
                protocol_actor_address=VAULT,
                dex_pool_address=POOL,
                dex_position_address="GbqbbAaDmypYedYrm9srwvYmd54hAmzqqo527KCDwoXv",
                deposits=(
                    DexAssetAmount(
                        mint_address="USDvUSpnhCr9yBgj3UyVrD239HRUv4RsHwH2FxsWuMk",
                        amount_raw="-11120779129",
                        decimals=6,
                    ),
                ),
            ),
        ),
    )

    removed = await client(
        lambda _: capture("list_transactions_liquidity_remove")
    ).dex.transactions.list(
        swig_config_address=SWIG,
        page_size=1,
        filters=[DexActionTypeFilter("liquidity-remove")],
    )

    assert (
        removed.next_page_token
        == capture("list_transactions_liquidity_remove")["nextPageToken"]
    )
    assert [transaction.slot for transaction in removed.transactions] == [449691047]
    assert removed.transactions[0].actions == (
        DexTransactionAction(
            action_index=0,
            swig_vault_address=VAULT,
            acting_role_id=3,
            protocol="raydium-clmm",
            dex_program_address="CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK",
            action=DexLiquidityRemoveAction(
                protocol_instruction="DecreaseLiquidityV2",
                protocol_actor_address=VAULT,
                dex_pool_address=POOL,
                dex_position_address="8aTLHPNxSmbyWwvjV71XdaFgfMZ6GuVS4duzNQWDPMuH",
                withdrawals=(
                    DexAssetAmount(
                        mint_address="EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
                        amount_raw="1092908",
                        decimals=6,
                    ),
                ),
            ),
        ),
    )


async def test_gets_one_transaction_by_signature() -> None:
    signature = capture("get_transaction_swap")["transaction"]["transactionSignature"]
    swig = "H2JPxSKQJU22CbegDVZZGd6jGhTLGJYtCLXtKRDzJL6c"
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> object:
        requests.append(request)
        return capture("get_transaction_swap")

    transaction = await client(handler).dex.transactions.get(
        swig_config_address=swig,
        network="mainnet",
        transaction_signature=signature,
    )

    assert requests[0].method == "GET"
    assert (
        requests[0].url.path == f"/wallet/api/swig/{swig}/dex/transactions/{signature}"
    )
    assert requests[0].url.query.decode() == "network=NETWORK_MAINNET"
    assert transaction.transaction_signature == signature
    assert transaction.slot == 454177312
    assert transaction.actions[0] == DexTransactionAction(
        action_index=0,
        swig_vault_address="9BySxhQYrvGgJnDekxNWtkQWnZutiMBSH3wuH43HRhg2",
        acting_role_id=1,
        protocol="orca-whirlpool",
        dex_program_address="whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc",
        action=DexSwapAction(
            protocol_instruction="SwapV2",
            protocol_actor_address="D5YqVMoSxnqeZAKAUUE1Dm3bmjtdxQ5DCF356ozqN9cM",
            dex_pool_address="CUsHj4Xf1dAGoCHUX3YYgsmN1PWCn7aSgCUCJ3Wvz7tq",
            input=DexAssetAmount(
                mint_address="So11111111111111111111111111111111111111112",
                amount_raw="-8813489",
                decimals=9,
            ),
            output=DexAssetAmount(
                mint_address="GN3Njbg9dP13pBX2MvdXurr4cuxzVmsVtKwnhPPymWZh",
                amount_raw="470167",
                decimals=4,
            ),
        ),
    )


async def test_searches_positions_with_every_filter_and_decodes_open_and_closed() -> (
    None
):
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> object:
        requests.append(request)
        return capture("list_positions")

    page = await client(handler).dex.positions.list(
        swig_config_address=SWIG,
        network="mainnet",
        page_size=25,
        filters=[
            DexPositionStatusFilter("open"),
            DexProtocolFilter("raydium-clmm"),
            DexSwigVaultAddressFilter(VAULT),
        ],
    )

    assert requests[0].method == "POST"
    assert requests[0].url.path == f"/wallet/api/swig/{SWIG}/dex/positions:search"
    assert body(requests[0]) == {
        "network": "NETWORK_MAINNET",
        "pageSize": 25,
        "filters": [
            {"positionStatus": "POSITION_STATUS_OPEN"},
            {"protocol": "DEX_PROTOCOL_RAYDIUM_CLMM"},
            {"swigVaultAddress": VAULT},
        ],
    }
    assert page.next_page_token is None
    assert [position.status for position in page.positions] == [
        "open",
        "closed",
        "closed",
    ]
    assert page.positions[0] == DexOpenPosition(
        swig_config_address=SWIG,
        swig_vault_address=VAULT,
        protocol="raydium-clmm",
        dex_position_address="8aTLHPNxSmbyWwvjV71XdaFgfMZ6GuVS4duzNQWDPMuH",
        updated_slot=454625145,
        source_transaction_signature=(
            "4CHrpkHNWm3CGdyB4CNNAcGW7TGdN1Vx3NyZaSvypZTiYHKbWUzEncajNmXharjULnaHS8GEuxfdcjU4eGUJm66V"
        ),
        dex_pool_address=POOL,
        state=DexRaydiumClmmPositionState(
            liquidity_raw="22090506953617",
            tick_lower_index=8,
            tick_upper_index=9,
            token_fees_owed_0="0",
            token_fees_owed_1="0",
            reward_amounts_owed=("0", "0", "0"),
        ),
    )
    assert page.positions[1] == DexClosedPosition(
        swig_config_address=SWIG,
        swig_vault_address=VAULT,
        protocol="raydium-clmm",
        dex_position_address="GbqbbAaDmypYedYrm9srwvYmd54hAmzqqo527KCDwoXv",
        updated_slot=454625146,
        source_transaction_signature=(
            "1oQ6eAYNazkVJ4MCL9sohmt3uh6dfrQZvs5GVYDnuez9ZEipgXJ8UBR4igpK47ga9drVpPfokCV3wxEcaCJG4es"
        ),
    )


async def test_decodes_uncaptured_action_kinds_from_their_protojson_shape() -> None:
    base = {
        "protocolInstruction": "Harvest",
        "protocolActorAddress": VAULT,
        "dexPoolAddress": POOL,
        "dexPositionAddress": "HpFVYnzYLrLPU5djgURwv7ydeaJWRGAjx3Xkd5ZNGaKp",
    }
    amount = {"mintAddress": "MINT", "amountRaw": "5"}

    def action_of(index: int, action: dict[str, object]) -> dict[str, object]:
        return {
            "actionIndex": index,
            "swigVaultAddress": VAULT,
            "actingRoleId": 1,
            "protocol": "DEX_PROTOCOL_RAYDIUM_CLMM",
            "dexProgramAddress": "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK",
            "action": action,
        }

    response = {
        "transaction": {
            "swigConfigAddress": SWIG,
            "transactionSignature": "signature",
            "slot": "1",
            "actions": [
                action_of(0, {"feeClaim": {**base, "claimed": [amount]}}),
                action_of(1, {"reward_claim": {**base, "claimed": [amount]}}),
                action_of(
                    2,
                    {
                        "positionRebalance": {
                            **base,
                            "deposits": [amount],
                            "withdrawals": [amount],
                        }
                    },
                ),
            ],
        }
    }

    transaction = await client(lambda _: response).dex.transactions.get(
        swig_config_address=SWIG, transaction_signature="signature"
    )

    claimed = (DexAssetAmount(mint_address="MINT", amount_raw="5"),)
    fields = {
        "protocol_instruction": "Harvest",
        "protocol_actor_address": VAULT,
        "dex_pool_address": POOL,
        "dex_position_address": "HpFVYnzYLrLPU5djgURwv7ydeaJWRGAjx3Xkd5ZNGaKp",
    }
    assert transaction.block_time is None
    assert [action.action for action in transaction.actions] == [
        DexFeeClaimAction(**fields, claimed=claimed),
        DexRewardClaimAction(**fields, claimed=claimed),
        DexPositionRebalanceAction(**fields, deposits=claimed, withdrawals=claimed),
    ]
    assert transaction.actions[0].acting_role_id == 1


async def test_refuses_an_action_without_acting_role_id() -> None:
    response = capture("get_transaction_swap")
    del response["transaction"]["actions"][0]["actingRoleId"]

    with pytest.raises(SwigResponseError, match="invalid actingRoleId"):
        await client(lambda _: response).dex.transactions.get(
            swig_config_address=SWIG, transaction_signature="signature"
        )


@pytest.mark.parametrize("resource", ["transactions", "positions"])
async def test_retries_a_search_on_a_transient_failure(resource: str) -> None:
    attempts = 0

    def handler(request: httpx.Request) -> object:
        nonlocal attempts
        attempts += 1
        if attempts == 1:
            return httpx.Response(503, text="temporary")
        return {resource: [], "nextPageToken": ""}

    swig = client(handler, retry_options=RetryOptions(max_retries=1, retry_delay=0))
    if resource == "transactions":
        await swig.dex.transactions.list(swig_config_address=SWIG)
    else:
        await swig.dex.positions.list(swig_config_address=SWIG)

    assert attempts == 2


def _with_action(action: dict[str, object]) -> dict[str, Any]:
    listed = copy.deepcopy(capture("list_transactions_clmm_page"))
    first = listed["transactions"][0]
    first["actions"] = [{**first["actions"][0], **action}]
    listed["transactions"] = [first]
    return listed


@pytest.mark.parametrize(
    ("response", "message"),
    [
        (_with_action({"protocol": "DEX_PROTOCOL_UNSPECIFIED"}), "invalid protocol"),
        (_with_action({"protocol": "DEX_PROTOCOL_UNISWAP"}), "invalid protocol"),
        (_with_action({"action": {}}), "unknown action"),
        (
            _with_action(
                {
                    "action": {
                        "swap": {
                            "protocolInstruction": "Buy",
                            "protocolActorAddress": VAULT,
                            "dexPoolAddress": "POOL",
                            "input": {"mintAddress": "MINT", "amountRaw": "1.5"},
                            "output": {"mintAddress": "MINT", "amountRaw": "1"},
                        }
                    }
                }
            ),
            "invalid amountRaw",
        ),
        (_with_action({"actingRoleId": "two"}), "invalid actingRoleId"),
    ],
)
async def test_refuses_a_transaction_outside_the_contract(
    response: dict[str, Any], message: str
) -> None:
    with pytest.raises(SwigResponseError, match=message):
        await client(lambda _: response).dex.transactions.list(swig_config_address=SWIG)


async def test_refuses_a_missing_transaction_or_position_state() -> None:
    stateless = {
        key: value
        for key, value in capture("list_positions")["positions"][1].items()
        if key != "closed"
    }

    with pytest.raises(SwigResponseError, match="missing transaction") as refused:
        await client(lambda _: {}).dex.transactions.get(
            swig_config_address=SWIG, transaction_signature="signature"
        )
    assert (refused.value.code, refused.value.status_code) == ("INVALID_RESPONSE", 502)
    with pytest.raises(SwigResponseError, match="missing position state"):
        await client(lambda _: {"positions": [stateless]}).dex.positions.list(
            swig_config_address=SWIG
        )


async def test_rejects_invalid_arguments_before_sending_a_request() -> None:
    swig = client(unreachable)

    with pytest.raises(
        ValueError, match="protocol must be a supported DEX protocol"
    ) as refused:
        await swig.dex.transactions.list(
            swig_config_address=SWIG,
            filters=[DexProtocolFilter("uniswap")],  # type: ignore[arg-type]
        )
    assert not isinstance(refused.value, SwigDeveloperSdkError)
    with pytest.raises(
        ValueError, match="action_type must be a supported DEX action type"
    ):
        await swig.dex.transactions.list(
            swig_config_address=SWIG,
            filters=[DexActionTypeFilter("stake")],  # type: ignore[arg-type]
        )
    with pytest.raises(ValueError, match="start_slot must be a non-negative integer"):
        await swig.dex.transactions.list(
            swig_config_address=SWIG, filters=[DexStartSlotFilter(-1)]
        )
    with pytest.raises(ValueError, match="end_slot must be a non-negative integer"):
        await swig.dex.transactions.list(
            swig_config_address=SWIG,
            filters=[DexEndSlotFilter(1.5)],  # type: ignore[arg-type]
        )
    with pytest.raises(ValueError, match="transaction filters must be"):
        await swig.dex.transactions.list(
            swig_config_address=SWIG,
            filters=[DexPositionStatusFilter("open")],  # type: ignore[list-item]
        )
    with pytest.raises(ValueError, match="page_size must be a non-negative integer"):
        await swig.dex.positions.list(swig_config_address=SWIG, page_size=-1)
    with pytest.raises(ValueError, match='position_status must be "open" or "closed"'):
        await swig.dex.positions.list(
            swig_config_address=SWIG,
            filters=[DexPositionStatusFilter("pending")],  # type: ignore[arg-type]
        )
    with pytest.raises(ValueError, match="position filters must be"):
        await swig.dex.positions.list(
            swig_config_address=SWIG,
            filters=[DexActionTypeFilter("swap")],  # type: ignore[list-item]
        )
    with pytest.raises(ValueError, match="network is required"):
        await client(unreachable, network=None).dex.positions.list(
            swig_config_address=SWIG
        )
