from __future__ import annotations

import copy
import json
from collections.abc import Callable
from pathlib import Path
from typing import Any
from urllib.parse import parse_qsl

import httpx
import pytest

from swig_developer_sdk import (
    DexAssetAmount,
    DexClosedPosition,
    DexFeeClaimAction,
    DexLiquidityAddAction,
    DexLiquidityRemoveAction,
    DexOpenPosition,
    DexPositionCloseAction,
    DexPositionRebalanceAction,
    DexRaydiumClmmPositionState,
    DexRewardClaimAction,
    DexSwapAction,
    DexTransactionAction,
    SwigClient,
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
) -> SwigClient:
    def respond(request: httpx.Request) -> httpx.Response:
        return httpx.Response(200, json=handler(request))

    return SwigClient(
        api_key="secret",
        base_url="https://example.test",
        network=network,  # type: ignore[arg-type]
        transport=httpx.MockTransport(respond),
    )


def unreachable(request: httpx.Request) -> object:
    raise AssertionError("no request should be sent")


def query(request: httpx.Request) -> dict[str, str]:
    return dict(parse_qsl(request.url.query.decode()))


async def test_lists_transactions_with_every_filter_on_the_query() -> None:
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> object:
        requests.append(request)
        return capture("list_transactions_clmm_page")

    page = await client(handler).dex.transactions.list(
        swig_config_address=SWIG,
        network="mainnet",
        page_size=2,
        page_token="previous-token",
        start_slot=449_000_000,
        end_slot=450_000_000,
        protocol="raydium-clmm",
        action_type="liquidity-add",
        swig_vault_address=VAULT,
    )

    request = requests[0]
    assert request.method == "GET"
    assert request.url.path == f"/wallet/swig/{SWIG}/dex/transactions"
    assert query(request) == {
        "network": "NETWORK_MAINNET",
        "pageSize": "2",
        "pageToken": "previous-token",
        "startSlot": "449000000",
        "endSlot": "450000000",
        "protocol": "DEX_PROTOCOL_RAYDIUM_CLMM",
        "actionType": "DEX_ACTION_TYPE_LIQUIDITY_ADD",
        "swigVaultAddress": VAULT,
    }
    assert (
        page.next_page_token == capture("list_transactions_clmm_page")["nextPageToken"]
    )
    assert [transaction.slot for transaction in page.transactions] == [
        449776519,
        449775094,
    ]
    newest = page.transactions[0]
    assert newest.block_time == "2026-09-23T17:52:56Z"
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
                dex_position_address="H5v7baxxmcr6L4Sg8zkdArYsLAT5tbtfQYmdB7gie7wm",
                deposits=(
                    DexAssetAmount(
                        mint_address="USDvUSpnhCr9yBgj3UyVrD239HRUv4RsHwH2FxsWuMk",
                        amount_raw="-500000000000",
                        decimals=6,
                    ),
                ),
            ),
        ),
    )
    assert page.transactions[1].actions[0].action.type == "position-open"


async def test_sends_only_the_client_network_when_no_filter_is_set() -> None:
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> object:
        requests.append(request)
        return {"transactions": [], "nextPageToken": ""}

    page = await client(handler).dex.transactions.list(swig_config_address=SWIG)

    assert requests[0].url.query.decode() == "network=NETWORK_DEVNET"
    assert page.transactions == ()
    assert page.next_page_token is None


async def test_decodes_a_close_without_a_pool_and_keeps_other_actions() -> None:
    page = await client(
        lambda _: capture("list_transactions_position_close")
    ).dex.transactions.list(swig_config_address=SWIG, action_type="position-close")

    assert page.next_page_token is None
    transaction = page.transactions[0]
    assert [action.action.type for action in transaction.actions] == [
        "liquidity-remove",
        "position-close",
        "position-open",
    ]
    assert transaction.actions[1].action == DexPositionCloseAction(
        protocol_instruction="ClosePosition",
        protocol_actor_address=VAULT,
        dex_position_address="GbqbbAaDmypYedYrm9srwvYmd54hAmzqqo527KCDwoXv",
    )
    remove = transaction.actions[0].action
    assert isinstance(remove, DexLiquidityRemoveAction)
    assert remove.withdrawals == (
        DexAssetAmount(
            mint_address="USDvUSpnhCr9yBgj3UyVrD239HRUv4RsHwH2FxsWuMk",
            amount_raw="990788839009",
            decimals=6,
        ),
        DexAssetAmount(
            mint_address="EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
            amount_raw="401407",
            decimals=6,
        ),
    )


async def test_gets_one_transaction_by_signature() -> None:
    signature = capture("get_transaction_swap")["transaction"]["transactionSignature"]
    swig = "4m3ptUjYAYLXJUDSKiTDemKUWht98UbEFPgEGvxHnkBz"
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> object:
        requests.append(request)
        return capture("get_transaction_swap")

    transaction = await client(handler).dex.transactions.get(
        swig_config_address=swig,
        network="mainnet",
        transaction_signature=signature,
    )

    assert requests[0].url.path == f"/wallet/swig/{swig}/dex/transactions/{signature}"
    assert requests[0].url.query.decode() == "network=NETWORK_MAINNET"
    assert transaction.transaction_signature == signature
    assert transaction.slot == 452163114
    assert transaction.actions[0] == DexTransactionAction(
        action_index=0,
        swig_vault_address="EnU8v4oYX6dzedmBm78MvJNzJUiJGmyaXyQiTcspPUAe",
        acting_role_id=10,
        protocol="orca-whirlpool",
        dex_program_address="whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc",
        action=DexSwapAction(
            protocol_instruction="SwapV2",
            protocol_actor_address="D5YqVMoSxnqeZAKAUUE1Dm3bmjtdxQ5DCF356ozqN9cM",
            dex_pool_address="Ckp1kwZqosaLU1h3zWtuaMBubyWM7LX3cxYezRVin7p2",
            input=DexAssetAmount(
                mint_address="So11111111111111111111111111111111111111112",
                amount_raw="-12994222",
                decimals=9,
            ),
            output=DexAssetAmount(
                mint_address="6p6xgHyF7AeE6TZkSmFsko444wqoP15icUSqi2jfGiPN",
                amount_raw="701624",
                decimals=6,
            ),
        ),
    )


async def test_lists_positions_with_every_filter_and_decodes_open_and_closed() -> None:
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> object:
        requests.append(request)
        return capture("list_positions")

    page = await client(handler).dex.positions.list(
        swig_config_address=SWIG,
        network="mainnet",
        page_size=25,
        position_status="open",
        protocol="raydium-clmm",
        swig_vault_address=VAULT,
    )

    assert requests[0].url.path == f"/wallet/swig/{SWIG}/dex/positions"
    assert query(requests[0]) == {
        "network": "NETWORK_MAINNET",
        "pageSize": "25",
        "positionStatus": "POSITION_STATUS_OPEN",
        "protocol": "DEX_PROTOCOL_RAYDIUM_CLMM",
        "swigVaultAddress": VAULT,
    }
    assert page.next_page_token is None
    assert [position.status for position in page.positions] == [
        "open",
        "closed",
        "open",
    ]
    assert page.positions[0] == DexOpenPosition(
        swig_config_address=SWIG,
        swig_vault_address=VAULT,
        protocol="raydium-clmm",
        dex_position_address="BDk8oZeiUPjZMWhpiugc4MY1FvjLgXYu5fxfXGdwMqB7",
        observed_slot=454258554,
        source_transaction_signature=(
            "2RVmaGZYw4wvKTnEz4ueYYjKYAWgA2dHdMeos9EJMrkEqDQCnBtk8hLGf5AkRw42b9H7K2aXC8KDXCtd9MmZ6L5j"
        ),
        dex_pool_address=POOL,
        state=DexRaydiumClmmPositionState(
            liquidity_raw="18817287959235504",
            tick_lower_index=0,
            tick_upper_index=1,
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
        observed_slot=454258553,
        source_transaction_signature=(
            "4KheMDLmCjiasYy7vHX6VxFJEJR3gH69QDAifUqkE91HSMqEVksDAcxEraEY1UVzoS28yEwdA3BBgHAoWZUaMuhX"
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
            "actingRoleId": "1",
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
    with pytest.raises(ValueError, match=message):
        await client(lambda _: response).dex.transactions.list(swig_config_address=SWIG)


async def test_refuses_a_missing_transaction_or_position_state() -> None:
    stateless = {
        key: value
        for key, value in capture("list_positions")["positions"][1].items()
        if key != "closed"
    }

    with pytest.raises(ValueError, match="missing transaction"):
        await client(lambda _: {}).dex.transactions.get(
            swig_config_address=SWIG, transaction_signature="signature"
        )
    with pytest.raises(ValueError, match="missing position state"):
        await client(lambda _: {"positions": [stateless]}).dex.positions.list(
            swig_config_address=SWIG
        )


async def test_rejects_invalid_arguments_before_sending_a_request() -> None:
    swig = client(unreachable)

    with pytest.raises(ValueError, match="protocol must be a supported DEX protocol"):
        await swig.dex.transactions.list(
            swig_config_address=SWIG,
            protocol="uniswap",  # type: ignore[arg-type]
        )
    with pytest.raises(
        ValueError, match="action_type must be a supported DEX action type"
    ):
        await swig.dex.transactions.list(
            swig_config_address=SWIG,
            action_type="stake",  # type: ignore[arg-type]
        )
    with pytest.raises(ValueError, match="start_slot must be a non-negative integer"):
        await swig.dex.transactions.list(swig_config_address=SWIG, start_slot=-1)
    with pytest.raises(ValueError, match="end_slot must be a non-negative integer"):
        await swig.dex.transactions.list(
            swig_config_address=SWIG,
            end_slot=1.5,  # type: ignore[arg-type]
        )
    with pytest.raises(ValueError, match="page_size must be a non-negative integer"):
        await swig.dex.positions.list(swig_config_address=SWIG, page_size=-1)
    with pytest.raises(ValueError, match='position_status must be "open" or "closed"'):
        await swig.dex.positions.list(
            swig_config_address=SWIG,
            position_status="pending",  # type: ignore[arg-type]
        )
    with pytest.raises(ValueError, match="network is required"):
        await client(unreachable, network=None).dex.positions.list(
            swig_config_address=SWIG
        )
