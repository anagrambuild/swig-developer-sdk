from __future__ import annotations

import json
from collections.abc import Awaitable, Callable
from pathlib import Path
from typing import Any

import httpx
import pytest

from swig_developer_sdk import (
    DexActionType,
    DexAssetAmount,
    DexClosedPosition,
    DexFeeClaimAction,
    DexLiquidityAddAction,
    DexLiquidityRemoveAction,
    DexOpenPosition,
    DexPositionCloseAction,
    DexPositionCursor,
    DexPositionOpenAction,
    DexPositionRebalanceAction,
    DexPositionStatus,
    DexProtocol,
    DexRaydiumClmmPositionState,
    DexRewardClaimAction,
    DexSwapAction,
    DexTransaction,
    DexTransactionAction,
    DexTransactionCursor,
    RetryOptions,
    SwigClient,
    SwigResponseError,
)

FIXTURES = Path(__file__).resolve().parents[2] / "fixtures" / "dex"


def capture(name: str) -> dict[str, Any]:
    """A verbatim backend response; fixtures/dex/README.md lists each request."""
    loaded: dict[str, Any] = json.loads((FIXTURES / f"{name}.json").read_text())
    return loaded


HISTORY = capture("list_transactions")
LIQUIDITY_ADDS = capture("list_transactions_filtered")
LIQUIDITY_ADDS_NEXT = capture("list_transactions_filtered_next")
NO_TRANSACTIONS = capture("list_transactions_empty")
SWAP = capture("get_transaction_swap")
ALL_POSITIONS = capture("list_positions_all")
OPEN_POSITIONS = capture("list_positions_open")
CLOSED_POSITIONS = capture("list_positions_closed")
POSITIONS_PAGE = capture("list_positions")
POSITIONS_NEXT_PAGE = capture("list_positions_next")
NO_POSITIONS = capture("list_positions_empty")

SWIG = "8NkNvqXK6BfLAeqHvcwHWK6b6wybcraj2J3vZHm3LarX"
VAULT = "4k9xWkFvjifracRGbBUHeKg2z8qzJBPwjqjCySu28Mej"
POOL = "EYJkpMH4KJy8XDQyK59DM2xBUodvPkerDxKMCwyL1ber"
CLMM_PROGRAM = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK"
USDV = "USDvUSpnhCr9yBgj3UyVrD239HRUv4RsHwH2FxsWuMk"
USDC = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"

# The request behind list_transactions_filtered.json.
LIQUIDITY_ADD_ARGS: dict[str, Any] = {
    "swig_config_address": SWIG,
    "page_size": 2,
    "start_slot": 449_000_000,
    "end_slot": 450_000_000,
    "protocol": "raydium-clmm",
    "action_type": "liquidity-add",
    "swig_vault_address": VAULT,
}
LIQUIDITY_ADD_BODY: dict[str, Any] = {
    "network": "NETWORK_MAINNET",
    "pageSize": 2,
    "filters": [
        {"startSlot": "449000000"},
        {"endSlot": "450000000"},
        {"protocol": "DEX_PROTOCOL_RAYDIUM_CLMM"},
        {"actionType": "DEX_ACTION_TYPE_LIQUIDITY_ADD"},
        {"swigVaultAddress": VAULT},
    ],
}

PROTOCOLS: list[tuple[DexProtocol, str]] = [
    ("pump-swap", "DEX_PROTOCOL_PUMP_SWAP"),
    ("meteora-dlmm", "DEX_PROTOCOL_METEORA_DLMM"),
    ("raydium-amm-v4", "DEX_PROTOCOL_RAYDIUM_AMM_V4"),
    ("raydium-cpmm", "DEX_PROTOCOL_RAYDIUM_CPMM"),
    ("raydium-clmm", "DEX_PROTOCOL_RAYDIUM_CLMM"),
    ("orca-whirlpool", "DEX_PROTOCOL_ORCA_WHIRLPOOL"),
]
ACTION_TYPES: list[tuple[DexActionType, str]] = [
    ("swap", "DEX_ACTION_TYPE_SWAP"),
    ("position-open", "DEX_ACTION_TYPE_POSITION_OPEN"),
    ("liquidity-add", "DEX_ACTION_TYPE_LIQUIDITY_ADD"),
    ("liquidity-remove", "DEX_ACTION_TYPE_LIQUIDITY_REMOVE"),
    ("position-rebalance", "DEX_ACTION_TYPE_POSITION_REBALANCE"),
    ("fee-claim", "DEX_ACTION_TYPE_FEE_CLAIM"),
    ("reward-claim", "DEX_ACTION_TYPE_REWARD_CLAIM"),
    ("position-close", "DEX_ACTION_TYPE_POSITION_CLOSE"),
]
POSITION_STATUSES: list[tuple[DexPositionStatus, str]] = [
    ("open", "POSITION_STATUS_OPEN"),
    ("closed", "POSITION_STATUS_CLOSED"),
]


def client(
    handler: Callable[[httpx.Request], object],
    network: str | None = "mainnet",
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


def clmm_action(
    action: object, acting_role_id: int = 1, action_index: int = 0
) -> DexTransactionAction:
    """A Raydium CLMM action of the Swig's vault, as the captures carry them."""
    return DexTransactionAction(
        action_index=action_index,
        swig_vault_address=VAULT,
        acting_role_id=acting_role_id,
        protocol="raydium-clmm",
        dex_program_address=CLMM_PROGRAM,
        action=action,  # type: ignore[arg-type]
    )


async def test_lists_transactions_with_every_filter_in_the_body() -> None:
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> object:
        requests.append(request)
        return LIQUIDITY_ADDS

    page = await client(handler).dex.transactions.list(**LIQUIDITY_ADD_ARGS)

    assert requests[0].method == "POST"
    assert requests[0].url.path == f"/wallet/api/swig/{SWIG}/dex/transactions"
    assert body(requests[0]) == LIQUIDITY_ADD_BODY
    assert page.next_cursor == DexTransactionCursor(
        slot=449761733,
        transaction_signature=(
            "1oQ6eAYNazkVJ4MCL9sohmt3uh6dfrQZvs5GVYDnuez9ZEipgXJ8UBR4igpK47ga9drVpPfokCV3wxEcaCJG4es"
        ),
    )
    assert [transaction.slot for transaction in page.transactions] == [
        449776519,
        449761733,
    ]
    assert page.transactions[0] == DexTransaction(
        swig_config_address=SWIG,
        transaction_signature=(
            "4nT1pQZcoX3ypGHGsM21EK6X7CFvdeNq7At8naf3mixX8JsvxztBecchqCWkn2JwErM9334s3NM5hpc2rYvSrjCJ"
        ),
        slot=449776519,
        block_time="2026-09-23T17:52:56Z",
        actions=(
            clmm_action(
                DexLiquidityAddAction(
                    protocol_instruction="IncreaseLiquidityV2",
                    protocol_actor_address=VAULT,
                    dex_pool_address=POOL,
                    dex_position_address="H5v7baxxmcr6L4Sg8zkdArYsLAT5tbtfQYmdB7gie7wm",
                    deposits=(
                        DexAssetAmount(
                            mint_address=USDV, amount_raw="-500000000000", decimals=6
                        ),
                    ),
                )
            ),
        ),
    )


async def test_sends_only_the_client_network_when_no_filter_is_set() -> None:
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> object:
        requests.append(request)
        return HISTORY

    page = await client(handler).dex.transactions.list(swig_config_address=SWIG)

    assert body(requests[0]) == {"network": "NETWORK_MAINNET"}
    # The whole history fits the default page, so there is no next cursor.
    assert page.next_cursor is None
    assert len(page.transactions) == 24


async def test_gets_one_transaction_by_signature() -> None:
    signature = SWAP["transaction"]["transactionSignature"]
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> object:
        requests.append(request)
        return SWAP

    transaction = await client(handler).dex.transactions.get(
        swig_config_address=SWIG, transaction_signature=signature
    )

    assert requests[0].method == "GET"
    assert requests[0].url.path == (
        f"/wallet/api/swig/{SWIG}/dex/transactions/{signature}"
    )
    assert requests[0].url.query.decode() == "network=NETWORK_MAINNET"
    assert transaction == DexTransaction(
        swig_config_address=SWIG,
        transaction_signature=signature,
        slot=449761692,
        block_time="2026-09-23T16:47:01Z",
        actions=(
            clmm_action(
                DexSwapAction(
                    protocol_instruction="SwapV2",
                    protocol_actor_address=VAULT,
                    dex_pool_address=POOL,
                    input=DexAssetAmount(
                        mint_address=USDV, amount_raw="-85238320", decimals=6
                    ),
                    output=DexAssetAmount(
                        mint_address=USDC, amount_raw="85263893", decimals=6
                    ),
                )
            ),
        ),
    )


async def test_lists_positions_with_every_filter_and_decodes_open_and_closed() -> None:
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> object:
        requests.append(request)
        return OPEN_POSITIONS if len(requests) == 1 else CLOSED_POSITIONS

    swig = client(handler)
    opened = await swig.dex.positions.list(
        swig_config_address=SWIG,
        page_size=25,
        position_status="open",
        protocol="raydium-clmm",
        swig_vault_address=VAULT,
    )
    closed = await swig.dex.positions.list(
        swig_config_address=SWIG, position_status="closed"
    )

    assert requests[0].method == "POST"
    assert requests[0].url.path == f"/wallet/api/swig/{SWIG}/dex/positions"
    assert [body(request) for request in requests] == [
        {
            "network": "NETWORK_MAINNET",
            "pageSize": 25,
            "filters": [
                {"positionStatus": "POSITION_STATUS_OPEN"},
                {"protocol": "DEX_PROTOCOL_RAYDIUM_CLMM"},
                {"swigVaultAddress": VAULT},
            ],
        },
        {
            "network": "NETWORK_MAINNET",
            "filters": [{"positionStatus": "POSITION_STATUS_CLOSED"}],
        },
    ]
    assert opened.next_cursor is None
    assert [position.status for position in opened.positions] == ["open"] * 3
    assert opened.positions[0] == DexOpenPosition(
        swig_config_address=SWIG,
        swig_vault_address=VAULT,
        protocol="raydium-clmm",
        dex_position_address="8aTLHPNxSmbyWwvjV71XdaFgfMZ6GuVS4duzNQWDPMuH",
        updated_slot=463003457,
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
    assert closed.positions == (
        DexClosedPosition(
            swig_config_address=SWIG,
            swig_vault_address=VAULT,
            protocol="raydium-clmm",
            dex_position_address="GbqbbAaDmypYedYrm9srwvYmd54hAmzqqo527KCDwoXv",
            updated_slot=463003458,
            source_transaction_signature=(
                "7EzDhDeqh6m5Z196MCCGuwXyJgXMyjYWHkSvAa4HiCVCfrFqTrc8NUHpE4gwUtZ56mFnyNpRMPzumvwoNZoz4Re"
            ),
        ),
        DexClosedPosition(
            swig_config_address=SWIG,
            swig_vault_address=VAULT,
            protocol="raydium-clmm",
            dex_position_address="HpFVYnzYLrLPU5djgURwv7ydeaJWRGAjx3Xkd5ZNGaKp",
            updated_slot=463003456,
            source_transaction_signature=(
                "5HsMkaQJXy1vZydQJXavx7keRtXV5BuR1pBQDJR93fhsfZMGYRTvFY13wm56JmEXg3uMP8ndUcUMeR8LpsznWhx1"
            ),
        ),
    )


async def test_pages_by_passing_next_cursor_back_as_cursor() -> None:
    replies = [LIQUIDITY_ADDS, LIQUIDITY_ADDS_NEXT, POSITIONS_PAGE, POSITIONS_NEXT_PAGE]
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> object:
        requests.append(request)
        return replies[len(requests) - 1]

    swig = client(handler)
    transactions = await swig.dex.transactions.list(**LIQUIDITY_ADD_ARGS)
    next_transactions = await swig.dex.transactions.list(
        **LIQUIDITY_ADD_ARGS, cursor=transactions.next_cursor
    )
    positions = await swig.dex.positions.list(swig_config_address=SWIG, page_size=2)
    next_positions = await swig.dex.positions.list(
        swig_config_address=SWIG, page_size=2, cursor=positions.next_cursor
    )

    # Each cursor goes back out exactly as the backend handed it over.
    assert [body(request) for request in requests] == [
        LIQUIDITY_ADD_BODY,
        {**LIQUIDITY_ADD_BODY, "cursor": LIQUIDITY_ADDS["nextCursor"]},
        {"network": "NETWORK_MAINNET", "pageSize": 2},
        {
            "network": "NETWORK_MAINNET",
            "pageSize": 2,
            "cursor": POSITIONS_PAGE["nextCursor"],
        },
    ]
    assert [transaction.slot for transaction in next_transactions.transactions] == [
        449760470,
        449755940,
    ]
    assert next_transactions.next_cursor is None
    assert positions.next_cursor == DexPositionCursor(
        protocol="raydium-clmm",
        dex_position_address="BDk8oZeiUPjZMWhpiugc4MY1FvjLgXYu5fxfXGdwMqB7",
    )
    assert [position.status for position in next_positions.positions] == [
        "closed",
        "open",
    ]
    assert next_positions.next_cursor == DexPositionCursor(
        protocol="raydium-clmm",
        dex_position_address="H5v7baxxmcr6L4Sg8zkdArYsLAT5tbtfQYmdB7gie7wm",
    )


async def test_decodes_every_action_kind_in_the_recorded_history() -> None:
    page = await client(lambda _: HISTORY).dex.transactions.list(
        swig_config_address=SWIG
    )
    actions = {
        transaction.transaction_signature: transaction.actions[0]
        for transaction in page.transactions
    }

    assert sorted(
        {
            action.action.type
            for transaction in page.transactions
            for action in transaction.actions
        }
    ) == [
        "liquidity-add",
        "liquidity-remove",
        "position-close",
        "position-open",
        "swap",
    ]
    assert actions[
        "2RVmaGZYw4wvKTnEz4ueYYjKYAWgA2dHdMeos9EJMrkEqDQCnBtk8hLGf5AkRw42b9H7K2aXC8KDXCtd9MmZ6L5j"
    ] == clmm_action(
        DexPositionOpenAction(
            protocol_instruction="OpenPositionWithToken22Nft",
            protocol_actor_address=VAULT,
            dex_pool_address=POOL,
            dex_position_address="BDk8oZeiUPjZMWhpiugc4MY1FvjLgXYu5fxfXGdwMqB7",
            deposits=(
                DexAssetAmount(
                    mint_address=USDV, amount_raw="-940793839009", decimals=6
                ),
            ),
        )
    )
    assert actions[
        "4CHrpkHNWm3CGdyB4CNNAcGW7TGdN1Vx3NyZaSvypZTiYHKbWUzEncajNmXharjULnaHS8GEuxfdcjU4eGUJm66V"
    ] == clmm_action(
        DexLiquidityRemoveAction(
            protocol_instruction="DecreaseLiquidityV2",
            protocol_actor_address=VAULT,
            dex_pool_address=POOL,
            dex_position_address="8aTLHPNxSmbyWwvjV71XdaFgfMZ6GuVS4duzNQWDPMuH",
            withdrawals=(
                DexAssetAmount(mint_address=USDC, amount_raw="1092908", decimals=6),
            ),
        ),
        acting_role_id=3,
    )
    # Raydium CLMM's ClosePosition names no pool. Role 0 is a real role.
    assert actions[
        "5HsMkaQJXy1vZydQJXavx7keRtXV5BuR1pBQDJR93fhsfZMGYRTvFY13wm56JmEXg3uMP8ndUcUMeR8LpsznWhx1"
    ] == clmm_action(
        DexPositionCloseAction(
            protocol_instruction="ClosePosition",
            protocol_actor_address=VAULT,
            dex_position_address="HpFVYnzYLrLPU5djgURwv7ydeaJWRGAjx3Xkd5ZNGaKp",
        ),
        acting_role_id=0,
    )


async def test_decodes_the_action_kinds_no_venue_records_yet() -> None:
    # Nothing in the recorded history carries these, so the bodies follow
    # swig_dex.proto rather than a capture. A close may also name its pool.
    position = {
        "protocolActorAddress": VAULT,
        "dexPoolAddress": POOL,
        "dexPositionAddress": "H5v7baxxmcr6L4Sg8zkdArYsLAT5tbtfQYmdB7gie7wm",
    }
    deposit = {"mintAddress": USDV, "amountRaw": "-1000000", "decimals": 6}
    received = {"mintAddress": USDC, "amountRaw": "1000", "decimals": 6}
    payloads = [
        {
            "positionRebalance": {
                "protocolInstruction": "Rebalance",
                **position,
                "deposits": [deposit],
                "withdrawals": [received],
            }
        },
        {
            "feeClaim": {
                "protocolInstruction": "DecreaseLiquidityV2",
                **position,
                "claimed": [received],
            }
        },
        {
            "rewardClaim": {
                "protocolInstruction": "CollectReward",
                **position,
                "claimed": [received],
            }
        },
        {"positionClose": {"protocolInstruction": "ClosePosition", **position}},
    ]
    swap = SWAP["transaction"]["actions"][0]
    response = {
        "transaction": {
            **SWAP["transaction"],
            "actions": [
                {**swap, "actionIndex": index, "action": payload}
                for index, payload in enumerate(payloads)
            ],
        }
    }

    transaction = await client(lambda _: response).dex.transactions.get(
        swig_config_address=SWIG,
        transaction_signature=SWAP["transaction"]["transactionSignature"],
    )

    fields = {
        "protocol_actor_address": VAULT,
        "dex_pool_address": POOL,
        "dex_position_address": "H5v7baxxmcr6L4Sg8zkdArYsLAT5tbtfQYmdB7gie7wm",
    }
    deposits = (DexAssetAmount(mint_address=USDV, amount_raw="-1000000", decimals=6),)
    receipts = (DexAssetAmount(mint_address=USDC, amount_raw="1000", decimals=6),)
    assert [action.action for action in transaction.actions] == [
        DexPositionRebalanceAction(
            protocol_instruction="Rebalance",
            **fields,
            deposits=deposits,
            withdrawals=receipts,
        ),
        DexFeeClaimAction(
            protocol_instruction="DecreaseLiquidityV2", **fields, claimed=receipts
        ),
        DexRewardClaimAction(
            protocol_instruction="CollectReward", **fields, claimed=receipts
        ),
        DexPositionCloseAction(protocol_instruction="ClosePosition", **fields),
    ]


async def test_sends_and_reads_every_enum_value_by_its_proto_name() -> None:
    # Only the request matters for the filters, so an empty page answers them.
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> object:
        requests.append(request)
        if request.url.path.endswith("/dex/positions"):
            return NO_POSITIONS
        return NO_TRANSACTIONS

    swig = client(handler)
    for protocol, _ in PROTOCOLS:
        await swig.dex.transactions.list(swig_config_address=SWIG, protocol=protocol)
    for action_type, _ in ACTION_TYPES:
        await swig.dex.transactions.list(
            swig_config_address=SWIG, action_type=action_type
        )
    for position_status, _ in POSITION_STATUSES:
        await swig.dex.positions.list(
            swig_config_address=SWIG, position_status=position_status
        )

    assert [body(request) for request in requests] == [
        *(
            {"network": "NETWORK_MAINNET", "filters": [{"protocol": wire}]}
            for _, wire in PROTOCOLS
        ),
        *(
            {"network": "NETWORK_MAINNET", "filters": [{"actionType": wire}]}
            for _, wire in ACTION_TYPES
        ),
        *(
            {"network": "NETWORK_MAINNET", "filters": [{"positionStatus": wire}]}
            for _, wire in POSITION_STATUSES
        ),
    ]

    # Only Raydium CLMM is recorded, so the other names go on a captured action.
    first = LIQUIDITY_ADDS["transactions"][0]
    response = {
        "transactions": [
            {
                **first,
                "actions": [
                    {**action, "protocol": wire} for action in first["actions"]
                ],
            }
            for _, wire in PROTOCOLS
        ]
    }
    page = await client(lambda _: response).dex.transactions.list(
        swig_config_address=SWIG
    )

    assert [transaction.actions[0].protocol for transaction in page.transactions] == [
        protocol for protocol, _ in PROTOCOLS
    ]


@pytest.mark.parametrize("resource", ["transactions", "positions"])
async def test_retries_a_list_on_a_transient_failure(resource: str) -> None:
    attempts = 0

    def handler(request: httpx.Request) -> object:
        nonlocal attempts
        attempts += 1
        if attempts == 1:
            return httpx.Response(503, text="temporary")
        return HISTORY if resource == "transactions" else ALL_POSITIONS

    swig = client(handler, retry_options=RetryOptions(max_retries=1, retry_delay=0))
    if resource == "transactions":
        await swig.dex.transactions.list(swig_config_address=SWIG)
    else:
        await swig.dex.positions.list(swig_config_address=SWIG)

    assert attempts == 2


def with_transaction(change: dict[str, Any]) -> dict[str, Any]:
    first = LIQUIDITY_ADDS["transactions"][0]
    return {"transactions": [{**first, **change}]}


def with_action(change: dict[str, Any]) -> dict[str, Any]:
    action = LIQUIDITY_ADDS["transactions"][0]["actions"][0]
    return with_transaction({"actions": [{**action, **change}]})


def with_deposit(amount_raw: str) -> dict[str, Any]:
    added = LIQUIDITY_ADDS["transactions"][0]["actions"][0]["action"]["liquidityAdd"]
    deposits = [{"mintAddress": USDV, "amountRaw": amount_raw}]
    return with_action({"action": {"liquidityAdd": {**added, "deposits": deposits}}})


@pytest.mark.parametrize(
    ("response", "message"),
    [
        (with_action({"protocol": "DEX_PROTOCOL_UNSPECIFIED"}), "invalid protocol"),
        (with_action({"protocol": "DEX_PROTOCOL_UNISWAP"}), "invalid protocol"),
        (with_action({"action": {}}), "an unknown action"),
        (with_deposit("1.5"), "invalid amountRaw"),
        # Python's \d would match these Arabic-Indic digits; the wire is ASCII.
        (with_deposit("-\u0663\u0664"), "invalid amountRaw"),
        (with_action({"actingRoleId": None}), "invalid actingRoleId"),
        (with_action({"actingRoleId": -1}), "invalid actingRoleId"),
        # Fullwidth digits, which \d matches too.
        (with_transaction({"slot": "\uff11\uff12"}), "invalid slot"),
        # 2^53, past the bound both SDKs hold slots to.
        (with_transaction({"slot": "9007199254740992"}), "invalid slot"),
        # Past int()'s digit limit, which raises a plain ValueError.
        (with_transaction({"slot": "9" * 5000}), "invalid slot"),
        (with_transaction({"blockTime": 1758649976}), "invalid blockTime"),
        (with_transaction({"blockTime": ""}), "invalid blockTime"),
    ],
)
async def test_refuses_a_transaction_outside_the_contract(
    response: dict[str, Any], message: str
) -> None:
    with pytest.raises(SwigResponseError, match=rf"^DEX response has {message}$"):
        await client(lambda _: response).dex.transactions.list(swig_config_address=SWIG)


async def test_refuses_a_missing_transaction_cursor_or_position_state() -> None:
    closed = CLOSED_POSITIONS["positions"][0]
    stateless = {key: value for key, value in closed.items() if key != "closed"}

    with pytest.raises(
        SwigResponseError, match=r"^DEX response is missing transaction$"
    ) as refused:
        await client(lambda _: {}).dex.transactions.get(
            swig_config_address=SWIG, transaction_signature="signature"
        )
    assert (refused.value.code, refused.value.status_code) == ("INVALID_RESPONSE", 502)
    with pytest.raises(
        SwigResponseError, match=r"^DEX response is missing transactionSignature$"
    ):
        await client(
            lambda _: {"transactions": [], "nextCursor": {"slot": "449761733"}}
        ).dex.transactions.list(swig_config_address=SWIG)
    for positions in ([stateless], [{**closed, "closed": None}]):
        with pytest.raises(
            SwigResponseError, match=r"^DEX response is missing position state$"
        ):
            reply = {"positions": positions}
            await client(lambda _, reply=reply: reply).dex.positions.list(
                swig_config_address=SWIG
            )
    unspecified_cursor = {
        "positions": [],
        "nextCursor": {
            "protocol": "DEX_PROTOCOL_UNSPECIFIED",
            "dexPositionAddress": closed["dexPositionAddress"],
        },
    }
    with pytest.raises(SwigResponseError, match=r"^DEX response has invalid protocol$"):
        await client(lambda _: unspecified_cursor).dex.positions.list(
            swig_config_address=SWIG
        )


async def read(resource: str, reply: object) -> Any:
    swig = client(lambda _: reply)
    if resource == "transactions":
        return await swig.dex.transactions.list(swig_config_address=SWIG)
    if resource == "positions":
        return await swig.dex.positions.list(swig_config_address=SWIG)
    return await swig.dex.transactions.get(
        swig_config_address=SWIG, transaction_signature="signature"
    )


def with_deposits(value: object) -> dict[str, Any]:
    added = LIQUIDITY_ADDS["transactions"][0]["actions"][0]["action"]["liquidityAdd"]
    return with_action({"action": {"liquidityAdd": {**added, "deposits": value}}})


def with_rewards(value: object) -> dict[str, Any]:
    opened = OPEN_POSITIONS["positions"][0]
    clmm = opened["open"]["state"]["raydiumClmm"]
    state = {"raydiumClmm": {**clmm, "rewardAmountsOwed": value}}
    return {"positions": [{**opened, "open": {**opened["open"], "state": state}}]}


@pytest.mark.parametrize("value", [False, {}, ""])
@pytest.mark.parametrize(
    ("resource", "response", "field"),
    [
        ("transactions", lambda value: {"transactions": value}, "transactions"),
        ("positions", lambda value: {"positions": value}, "positions"),
        ("transactions", lambda value: with_transaction({"actions": value}), "actions"),
        ("transactions", with_deposits, "deposits"),
        ("positions", with_rewards, "rewardAmountsOwed"),
    ],
)
async def test_refuses_a_collection_that_is_not_a_list(
    resource: str,
    response: Callable[[object], dict[str, Any]],
    field: str,
    value: object,
) -> None:
    with pytest.raises(SwigResponseError, match=rf"^DEX response has invalid {field}$"):
        await read(resource, response(value))


@pytest.mark.parametrize(
    ("resource", "label"),
    [
        ("transactions", "DEX transactions response"),
        ("positions", "DEX positions response"),
        ("get", "DEX transaction response"),
    ],
)
async def test_refuses_a_body_that_is_not_an_object(resource: str, label: str) -> None:
    # Each a 200: an empty body, and JSON that is not an object.
    for reply in (httpx.Response(200), False, []):
        with pytest.raises(SwigResponseError, match=rf"^{label} must be an object$"):
            await read(resource, reply)
    html = httpx.Response(
        200,
        text="<html>Upstream unavailable</html>",
        headers={"Content-Type": "text/html"},
    )
    with pytest.raises(SwigResponseError, match=r"^API response is not valid JSON$"):
        await read(resource, html)


@pytest.mark.parametrize("resource", ["transactions", "positions"])
async def test_refuses_a_falsy_cursor_instead_of_ending_the_list(
    resource: str,
) -> None:
    with pytest.raises(SwigResponseError, match=r"^DEX cursor must be an object$"):
        await read(resource, {resource: [], "nextCursor": False})


async def test_reads_an_omitted_or_null_field_as_absent() -> None:
    for resource in ("transactions", "positions"):
        for reply in ({}, {resource: None, "nextCursor": None}):
            page = await read(resource, reply)
            assert getattr(page, resource) == ()
            assert page.next_cursor is None
    page = await read(
        "transactions", with_transaction({"blockTime": None, "actions": None})
    )
    assert page.transactions[0].block_time is None
    assert page.transactions[0].actions == ()


@pytest.mark.parametrize(
    ("call", "message"),
    [
        (
            lambda swig: swig.dex.transactions.list(
                swig_config_address=SWIG, protocol="uniswap"
            ),
            "protocol must be a supported DEX protocol",
        ),
        (
            lambda swig: swig.dex.transactions.list(
                swig_config_address=SWIG, action_type="stake"
            ),
            "action_type must be a supported DEX action type",
        ),
        (
            lambda swig: swig.dex.transactions.list(
                swig_config_address=SWIG, start_slot=-1
            ),
            "start_slot must be an integer from 0 to 9007199254740991",
        ),
        (
            lambda swig: swig.dex.transactions.list(
                swig_config_address=SWIG, end_slot=1.5
            ),
            "end_slot must be an integer from 0 to 9007199254740991",
        ),
        (
            lambda swig: swig.dex.transactions.list(
                swig_config_address=SWIG,
                cursor=DexTransactionCursor(slot=-1, transaction_signature="signature"),
            ),
            "cursor.slot must be an integer from 0 to 9007199254740991",
        ),
        (
            lambda swig: swig.dex.positions.list(
                swig_config_address=SWIG, position_status="pending"
            ),
            'position_status must be "open" or "closed"',
        ),
        (
            lambda swig: swig.dex.positions.list(
                swig_config_address=SWIG, page_size=-1
            ),
            "page_size must be an integer from 0 to 4294967295",
        ),
        (
            lambda swig: swig.dex.positions.list(
                swig_config_address=SWIG, page_size=2**32
            ),
            "page_size must be an integer from 0 to 4294967295",
        ),
    ],
)
async def test_rejects_invalid_arguments_before_sending_a_request(
    call: Callable[[SwigClient], Awaitable[object]], message: str
) -> None:
    with pytest.raises(ValueError) as refused:
        await call(client(unreachable))
    # SwigResponseError is also a ValueError; an argument refusal must not be one.
    assert type(refused.value) is ValueError
    assert str(refused.value) == message


async def test_rejects_a_list_without_a_network() -> None:
    with pytest.raises(ValueError) as refused:
        await client(unreachable, network=None).dex.positions.list(
            swig_config_address=SWIG
        )
    assert type(refused.value) is ValueError
    assert str(refused.value) == "network is required"
