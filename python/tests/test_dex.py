from __future__ import annotations

import json
from collections.abc import Callable
from typing import Any

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
    DexPositionOpenAction,
    DexPositionRebalanceAction,
    DexRaydiumClmmPositionState,
    DexRewardClaimAction,
    DexSwapAction,
    DexTransactionAction,
    RetryOptions,
    SwigClient,
    SwigDeveloperSdkError,
    SwigResponseError,
)

SWIG = "8NkNvqXK6BfLAeqHvcwHWK6b6wybcraj2J3vZHm3LarX"
VAULT = "4k9xWkFvjifracRGbBUHeKg2z8qzJBPwjqjCySu28Mej"
POOL = "EYJkpMH4KJy8XDQyK59DM2xBUodvPkerDxKMCwyL1ber"
POSITION = "GbqbbAaDmypYedYrm9srwvYmd54hAmzqqo527KCDwoXv"
CLMM_PROGRAM = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK"
USDV = "USDvUSpnhCr9yBgj3UyVrD239HRUv4RsHwH2FxsWuMk"
USDC = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"


def action_of(
    action_index: int, action: dict[str, Any], acting_role_id: int = 1
) -> dict[str, Any]:
    """A Raydium CLMM action as the backend returns it."""
    return {
        "actionIndex": action_index,
        "swigVaultAddress": VAULT,
        "actingRoleId": acting_role_id,
        "protocol": "DEX_PROTOCOL_RAYDIUM_CLMM",
        "dexProgramAddress": CLMM_PROGRAM,
        "action": action,
    }


# Responses below are copied from the backend over mainnet data.

TRANSACTIONS_PAGE: dict[str, Any] = {
    "transactions": [
        {
            "swigConfigAddress": SWIG,
            "transactionSignature": (
                "1oQ6eAYNazkVJ4MCL9sohmt3uh6dfrQZvs5GVYDnuez9ZEipgXJ8UBR4igpK47ga9drVpPfokCV3wxEcaCJG4es"
            ),
            "slot": "449761733",
            "blockTime": "2026-09-23T16:47:12Z",
            "actions": [
                action_of(
                    0,
                    {
                        "liquidityAdd": {
                            "protocolInstruction": "IncreaseLiquidityV2",
                            "protocolActorAddress": VAULT,
                            "dexPoolAddress": POOL,
                            "dexPositionAddress": POSITION,
                            "deposits": [
                                {
                                    "mintAddress": USDV,
                                    "amountRaw": "-679914761680",
                                    "decimals": 6,
                                }
                            ],
                        }
                    },
                )
            ],
        },
        {
            "swigConfigAddress": SWIG,
            "transactionSignature": (
                "5ic74krw5dwJAHpdagUqheKqj7cRE1wSTPTGzFjRSLFTewpGjsXA1DVk8C2d5QEJSefbjqBr7QczLLVfK7kbF9ra"
            ),
            "slot": "449761692",
            "blockTime": "2026-09-23T16:47:01Z",
            "actions": [
                action_of(
                    0,
                    {
                        "swap": {
                            "protocolInstruction": "SwapV2",
                            "protocolActorAddress": VAULT,
                            "dexPoolAddress": POOL,
                            "input": {
                                "mintAddress": USDV,
                                "amountRaw": "-85238320",
                                "decimals": 6,
                            },
                            "output": {
                                "mintAddress": USDC,
                                "amountRaw": "85263893",
                                "decimals": 6,
                            },
                        }
                    },
                )
            ],
        },
    ],
    "nextPageToken": "next-page-token",
}

POSITIONS_PAGE: dict[str, Any] = {
    "positions": [
        {
            "swigConfigAddress": SWIG,
            "swigVaultAddress": VAULT,
            "protocol": "DEX_PROTOCOL_RAYDIUM_CLMM",
            "dexPositionAddress": "8aTLHPNxSmbyWwvjV71XdaFgfMZ6GuVS4duzNQWDPMuH",
            "open": {
                "dexPoolAddress": POOL,
                "state": {
                    "raydiumClmm": {
                        "liquidityRaw": "22090506953617",
                        "tickLowerIndex": 8,
                        "tickUpperIndex": 9,
                        "tokenFeesOwed0": "0",
                        "tokenFeesOwed1": "0",
                        "rewardAmountsOwed": ["0", "0", "0"],
                    }
                },
            },
            "updatedSlot": "454625145",
            "sourceTransactionSignature": (
                "4CHrpkHNWm3CGdyB4CNNAcGW7TGdN1Vx3NyZaSvypZTiYHKbWUzEncajNmXharjULnaHS8GEuxfdcjU4eGUJm66V"
            ),
        },
        {
            "swigConfigAddress": SWIG,
            "swigVaultAddress": VAULT,
            "protocol": "DEX_PROTOCOL_RAYDIUM_CLMM",
            "dexPositionAddress": POSITION,
            "closed": {},
            "updatedSlot": "454625146",
            "sourceTransactionSignature": (
                "1oQ6eAYNazkVJ4MCL9sohmt3uh6dfrQZvs5GVYDnuez9ZEipgXJ8UBR4igpK47ga9drVpPfokCV3wxEcaCJG4es"
            ),
        },
    ],
    "nextPageToken": "",
}


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
        return TRANSACTIONS_PAGE

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

    assert requests[0].method == "POST"
    assert requests[0].url.path == f"/wallet/api/swig/{SWIG}/dex/transactions:search"
    assert body(requests[0]) == {
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
    assert page.next_page_token == "next-page-token"
    assert [transaction.slot for transaction in page.transactions] == [
        449761733,
        449761692,
    ]
    assert page.transactions[0].block_time == "2026-09-23T16:47:12Z"
    assert page.transactions[0].actions == (
        DexTransactionAction(
            action_index=0,
            swig_vault_address=VAULT,
            acting_role_id=1,
            protocol="raydium-clmm",
            dex_program_address=CLMM_PROGRAM,
            action=DexLiquidityAddAction(
                protocol_instruction="IncreaseLiquidityV2",
                protocol_actor_address=VAULT,
                dex_pool_address=POOL,
                dex_position_address=POSITION,
                deposits=(
                    DexAssetAmount(
                        mint_address=USDV,
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


async def test_gets_one_transaction_by_signature() -> None:
    swap = TRANSACTIONS_PAGE["transactions"][1]
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> object:
        requests.append(request)
        return {"transaction": swap}

    transaction = await client(handler).dex.transactions.get(
        swig_config_address=SWIG,
        network="mainnet",
        transaction_signature=swap["transactionSignature"],
    )

    assert requests[0].method == "GET"
    assert requests[0].url.path == (
        f"/wallet/api/swig/{SWIG}/dex/transactions/{swap['transactionSignature']}"
    )
    assert requests[0].url.query.decode() == "network=NETWORK_MAINNET"
    assert transaction.slot == 449761692
    assert transaction.actions[0].action == DexSwapAction(
        protocol_instruction="SwapV2",
        protocol_actor_address=VAULT,
        dex_pool_address=POOL,
        input=DexAssetAmount(
            mint_address=USDV,
            amount_raw="-85238320",
            decimals=6,
        ),
        output=DexAssetAmount(
            mint_address=USDC,
            amount_raw="85263893",
            decimals=6,
        ),
    )


async def test_searches_positions_with_every_filter_and_decodes_open_and_closed() -> (
    None
):
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> object:
        requests.append(request)
        return POSITIONS_PAGE

    page = await client(handler).dex.positions.list(
        swig_config_address=SWIG,
        network="mainnet",
        page_size=25,
        position_status="open",
        protocol="raydium-clmm",
        swig_vault_address=VAULT,
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
    assert page.positions == (
        DexOpenPosition(
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
        ),
        DexClosedPosition(
            swig_config_address=SWIG,
            swig_vault_address=VAULT,
            protocol="raydium-clmm",
            dex_position_address=POSITION,
            updated_slot=454625146,
            source_transaction_signature=(
                "1oQ6eAYNazkVJ4MCL9sohmt3uh6dfrQZvs5GVYDnuez9ZEipgXJ8UBR4igpK47ga9drVpPfokCV3wxEcaCJG4es"
            ),
        ),
    )


async def test_decodes_every_other_action_kind() -> None:
    amount = {"mintAddress": "MINT", "amountRaw": "5"}
    base = {
        "protocolInstruction": "Harvest",
        "protocolActorAddress": VAULT,
        "dexPoolAddress": POOL,
        "dexPositionAddress": POSITION,
    }
    response = {
        "transaction": {
            "swigConfigAddress": SWIG,
            "transactionSignature": "signature",
            "slot": "1",
            "actions": [
                action_of(
                    0,
                    {
                        "positionOpen": {
                            "protocolInstruction": "OpenPositionWithToken22Nft",
                            "protocolActorAddress": VAULT,
                            "dexPoolAddress": POOL,
                            "dexPositionAddress": POSITION,
                            "deposits": [amount],
                        }
                    },
                ),
                action_of(
                    1,
                    {
                        "liquidityRemove": {
                            "protocolInstruction": "DecreaseLiquidityV2",
                            "protocolActorAddress": VAULT,
                            "dexPoolAddress": POOL,
                            "dexPositionAddress": POSITION,
                            "withdrawals": [amount],
                        }
                    },
                ),
                # Raydium CLMM's ClosePosition names no pool. Role 0 is a real role.
                action_of(
                    2,
                    {
                        "positionClose": {
                            "protocolInstruction": "ClosePosition",
                            "protocolActorAddress": VAULT,
                            "dexPositionAddress": POSITION,
                        }
                    },
                    acting_role_id=0,
                ),
                action_of(
                    3,
                    {
                        "positionRebalance": {
                            **base,
                            "deposits": [amount],
                            "withdrawals": [amount],
                        }
                    },
                ),
                action_of(4, {"feeClaim": {**base, "claimed": [amount]}}),
                action_of(5, {"rewardClaim": {**base, "claimed": [amount]}}),
            ],
        }
    }

    transaction = await client(lambda _: response).dex.transactions.get(
        swig_config_address=SWIG, transaction_signature="signature"
    )

    amounts = (DexAssetAmount(mint_address="MINT", amount_raw="5"),)
    fields = {
        "protocol_instruction": "Harvest",
        "protocol_actor_address": VAULT,
        "dex_pool_address": POOL,
        "dex_position_address": POSITION,
    }
    assert transaction.block_time is None
    assert transaction.actions[2].acting_role_id == 0
    assert [action.action for action in transaction.actions] == [
        DexPositionOpenAction(
            protocol_instruction="OpenPositionWithToken22Nft",
            protocol_actor_address=VAULT,
            dex_pool_address=POOL,
            dex_position_address=POSITION,
            deposits=amounts,
        ),
        DexLiquidityRemoveAction(
            protocol_instruction="DecreaseLiquidityV2",
            protocol_actor_address=VAULT,
            dex_pool_address=POOL,
            dex_position_address=POSITION,
            withdrawals=amounts,
        ),
        DexPositionCloseAction(
            protocol_instruction="ClosePosition",
            protocol_actor_address=VAULT,
            dex_position_address=POSITION,
        ),
        DexPositionRebalanceAction(**fields, deposits=amounts, withdrawals=amounts),
        DexFeeClaimAction(**fields, claimed=amounts),
        DexRewardClaimAction(**fields, claimed=amounts),
    ]


@pytest.mark.parametrize("resource", ["transactions", "positions"])
async def test_retries_a_search_on_a_transient_failure(resource: str) -> None:
    attempts = 0

    def handler(request: httpx.Request) -> object:
        nonlocal attempts
        attempts += 1
        if attempts == 1:
            return httpx.Response(503, text="temporary")
        return {resource: []}

    swig = client(handler, retry_options=RetryOptions(max_retries=1, retry_delay=0))
    if resource == "transactions":
        await swig.dex.transactions.list(swig_config_address=SWIG)
    else:
        await swig.dex.positions.list(swig_config_address=SWIG)

    assert attempts == 2


def with_action(change: dict[str, Any]) -> dict[str, Any]:
    first = TRANSACTIONS_PAGE["transactions"][0]
    action = {**first["actions"][0], **change}
    return {"transactions": [{**first, "actions": [action]}]}


@pytest.mark.parametrize(
    ("response", "message"),
    [
        (with_action({"protocol": "DEX_PROTOCOL_UNSPECIFIED"}), "invalid protocol"),
        (with_action({"protocol": "DEX_PROTOCOL_UNISWAP"}), "invalid protocol"),
        (with_action({"action": {}}), "unknown action"),
        (
            with_action(
                {
                    "action": {
                        "swap": {
                            "protocolInstruction": "Buy",
                            "protocolActorAddress": VAULT,
                            "dexPoolAddress": POOL,
                            "input": {"mintAddress": "MINT", "amountRaw": "1.5"},
                            "output": {"mintAddress": "MINT", "amountRaw": "1"},
                        }
                    }
                }
            ),
            "invalid amountRaw",
        ),
        (with_action({"actingRoleId": None}), "invalid actingRoleId"),
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
        for key, value in POSITIONS_PAGE["positions"][1].items()
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
            protocol="uniswap",  # type: ignore[arg-type]
        )
    assert not isinstance(refused.value, SwigDeveloperSdkError)
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
    with pytest.raises(ValueError, match='position_status must be "open" or "closed"'):
        await swig.dex.positions.list(
            swig_config_address=SWIG,
            position_status="pending",  # type: ignore[arg-type]
        )
    with pytest.raises(ValueError, match="page_size must be a non-negative integer"):
        await swig.dex.positions.list(swig_config_address=SWIG, page_size=-1)
    with pytest.raises(ValueError, match="network is required"):
        await client(unreachable, network=None).dex.positions.list(
            swig_config_address=SWIG
        )
