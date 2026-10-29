from __future__ import annotations

import re
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import Literal, TypeAlias
from urllib.parse import quote, urlencode

from .common import Network, require_network, to_proto_network
from .core import HttpClient
from .errors import SwigResponseError

DexProtocol: TypeAlias = Literal[
    "pump-swap",
    "meteora-dlmm",
    "raydium-amm-v4",
    "raydium-cpmm",
    "raydium-clmm",
    "orca-whirlpool",
]

DexActionType: TypeAlias = Literal[
    "swap",
    "position-open",
    "liquidity-add",
    "liquidity-remove",
    "position-rebalance",
    "fee-claim",
    "reward-claim",
    "position-close",
]

DexPositionStatus: TypeAlias = Literal["open", "closed"]

_PROTOCOLS: dict[str, DexProtocol] = {
    "DEX_PROTOCOL_PUMP_SWAP": "pump-swap",
    "DEX_PROTOCOL_METEORA_DLMM": "meteora-dlmm",
    "DEX_PROTOCOL_RAYDIUM_AMM_V4": "raydium-amm-v4",
    "DEX_PROTOCOL_RAYDIUM_CPMM": "raydium-cpmm",
    "DEX_PROTOCOL_RAYDIUM_CLMM": "raydium-clmm",
    "DEX_PROTOCOL_ORCA_WHIRLPOOL": "orca-whirlpool",
}

_PROTOCOL_WIRE: dict[str, str] = {name: wire for wire, name in _PROTOCOLS.items()}

_ACTION_TYPE_WIRE: dict[str, str] = {
    "swap": "DEX_ACTION_TYPE_SWAP",
    "position-open": "DEX_ACTION_TYPE_POSITION_OPEN",
    "liquidity-add": "DEX_ACTION_TYPE_LIQUIDITY_ADD",
    "liquidity-remove": "DEX_ACTION_TYPE_LIQUIDITY_REMOVE",
    "position-rebalance": "DEX_ACTION_TYPE_POSITION_REBALANCE",
    "fee-claim": "DEX_ACTION_TYPE_FEE_CLAIM",
    "reward-claim": "DEX_ACTION_TYPE_REWARD_CLAIM",
    "position-close": "DEX_ACTION_TYPE_POSITION_CLOSE",
}

_POSITION_STATUS_WIRE: dict[str, str] = {
    "open": "POSITION_STATUS_OPEN",
    "closed": "POSITION_STATUS_CLOSED",
}

_SIGNED_INTEGER = re.compile(r"-?\d+")
_UNSIGNED_INTEGER = re.compile(r"\d+")


@dataclass(frozen=True, slots=True)
class DexAssetAmount:
    """Signed for the action's protocol actor: a negative amount left its accounts.

    A decimal integer string, so a protocol u128 survives exactly.
    """

    mint_address: str
    amount_raw: str
    #: Absent when the transaction never states the mint's decimals.
    decimals: int | None = None


@dataclass(frozen=True, slots=True)
class DexSwapAction:
    protocol_instruction: str
    #: The Swig vault on a direct call; the router's account when an aggregator
    #: holds the assets.
    protocol_actor_address: str
    dex_pool_address: str
    input: DexAssetAmount
    output: DexAssetAmount
    type: Literal["swap"] = "swap"


@dataclass(frozen=True, slots=True)
class DexPositionOpenAction:
    protocol_instruction: str
    protocol_actor_address: str
    dex_pool_address: str
    dex_position_address: str
    deposits: tuple[DexAssetAmount, ...]
    type: Literal["position-open"] = "position-open"


@dataclass(frozen=True, slots=True)
class DexLiquidityAddAction:
    protocol_instruction: str
    protocol_actor_address: str
    dex_pool_address: str
    dex_position_address: str
    deposits: tuple[DexAssetAmount, ...]
    type: Literal["liquidity-add"] = "liquidity-add"


@dataclass(frozen=True, slots=True)
class DexLiquidityRemoveAction:
    protocol_instruction: str
    protocol_actor_address: str
    dex_pool_address: str
    dex_position_address: str
    withdrawals: tuple[DexAssetAmount, ...]
    type: Literal["liquidity-remove"] = "liquidity-remove"


@dataclass(frozen=True, slots=True)
class DexPositionRebalanceAction:
    protocol_instruction: str
    protocol_actor_address: str
    dex_pool_address: str
    dex_position_address: str
    deposits: tuple[DexAssetAmount, ...]
    withdrawals: tuple[DexAssetAmount, ...]
    type: Literal["position-rebalance"] = "position-rebalance"


@dataclass(frozen=True, slots=True)
class DexFeeClaimAction:
    protocol_instruction: str
    protocol_actor_address: str
    dex_pool_address: str
    dex_position_address: str
    claimed: tuple[DexAssetAmount, ...]
    type: Literal["fee-claim"] = "fee-claim"


@dataclass(frozen=True, slots=True)
class DexRewardClaimAction:
    protocol_instruction: str
    protocol_actor_address: str
    dex_pool_address: str
    dex_position_address: str
    claimed: tuple[DexAssetAmount, ...]
    type: Literal["reward-claim"] = "reward-claim"


@dataclass(frozen=True, slots=True)
class DexPositionCloseAction:
    protocol_instruction: str
    protocol_actor_address: str
    dex_position_address: str
    #: Absent where the instruction names no pool, as Raydium CLMM's ClosePosition.
    dex_pool_address: str | None = None
    type: Literal["position-close"] = "position-close"


DexAction: TypeAlias = (
    DexSwapAction
    | DexPositionOpenAction
    | DexLiquidityAddAction
    | DexLiquidityRemoveAction
    | DexPositionRebalanceAction
    | DexFeeClaimAction
    | DexRewardClaimAction
    | DexPositionCloseAction
)


@dataclass(frozen=True, slots=True)
class DexTransactionAction:
    action_index: int
    swig_vault_address: str
    #: The Swig role that authorized the action.
    acting_role_id: int
    protocol: DexProtocol
    dex_program_address: str
    action: DexAction


@dataclass(frozen=True, slots=True)
class DexTransaction:
    """One Solana transaction the DEX index interpreted for a Swig."""

    swig_config_address: str
    transaction_signature: str
    slot: int
    #: Every DEX action the Swig executed in the transaction, in execution order.
    actions: tuple[DexTransactionAction, ...]
    block_time: str | None = None


@dataclass(frozen=True, slots=True)
class DexTransactionCursor:
    """The last transaction of a page; the next page starts after it."""

    slot: int
    transaction_signature: str


@dataclass(frozen=True, slots=True)
class ListDexTransactionsResult:
    transactions: tuple[DexTransaction, ...]
    #: None on the last page; otherwise pass it back as ``cursor`` with the same
    #: filters.
    next_cursor: DexTransactionCursor | None = None


@dataclass(frozen=True, slots=True)
class DexRaydiumClmmPositionState:
    """Raydium CLMM's ``PersonalPositionState``, named as the account names them."""

    liquidity_raw: str
    tick_lower_index: int
    tick_upper_index: int
    token_fees_owed_0: str
    token_fees_owed_1: str
    #: One per reward slot, in order.
    reward_amounts_owed: tuple[str, ...]
    type: Literal["raydium-clmm"] = "raydium-clmm"


DexPositionAccountState: TypeAlias = DexRaydiumClmmPositionState


@dataclass(frozen=True, slots=True)
class DexOpenPosition:
    swig_config_address: str
    swig_vault_address: str
    protocol: DexProtocol
    dex_position_address: str
    #: The slot of the on-chain read the state came from.
    updated_slot: int
    source_transaction_signature: str
    dex_pool_address: str
    state: DexPositionAccountState
    status: Literal["open"] = "open"


@dataclass(frozen=True, slots=True)
class DexClosedPosition:
    """The position account no longer exists on chain."""

    swig_config_address: str
    swig_vault_address: str
    protocol: DexProtocol
    dex_position_address: str
    updated_slot: int
    source_transaction_signature: str
    status: Literal["closed"] = "closed"


DexPosition: TypeAlias = DexOpenPosition | DexClosedPosition


@dataclass(frozen=True, slots=True)
class DexPositionCursor:
    """The last position of a page; the next page starts after it."""

    protocol: DexProtocol
    dex_position_address: str


@dataclass(frozen=True, slots=True)
class ListDexPositionsResult:
    positions: tuple[DexPosition, ...]
    #: None on the last page; otherwise pass it back as ``cursor`` with the same
    #: filters.
    next_cursor: DexPositionCursor | None = None


class DexTransactionsClient:
    def __init__(
        self, http: HttpClient, default_network: Network | None = None
    ) -> None:
        self._http = http
        self._default_network = default_network

    async def list(
        self,
        *,
        swig_config_address: str,
        network: Network | None = None,
        page_size: int | None = None,
        cursor: DexTransactionCursor | None = None,
        start_slot: int | None = None,
        end_slot: int | None = None,
        protocol: DexProtocol | None = None,
        action_type: DexActionType | None = None,
        swig_vault_address: str | None = None,
    ) -> ListDexTransactionsResult:
        """List the Swig's indexed DEX transactions, newest first, a page per call.

        Only transactions the index interpreted are listed. Every filter is
        optional and they combine with AND. Slots are inclusive; protocol,
        action_type and swig_vault_address must match the same action. Pass
        next_cursor back as cursor with the same filters; the last page has
        none, and nothing pages automatically. Network resolves from the call,
        then client.
        This list only reads, so it uses the client retry policy. API failures
        raise SDK errors."""
        filters: list[dict[str, str]] = []
        if start_slot is not None:
            filters.append({"startSlot": _slot_param(start_slot, "start_slot")})
        if end_slot is not None:
            filters.append({"endSlot": _slot_param(end_slot, "end_slot")})
        if protocol is not None:
            filters.append({"protocol": _protocol_wire(protocol)})
        if action_type is not None:
            filters.append({"actionType": _action_type_wire(action_type)})
        if swig_vault_address is not None:
            filters.append({"swigVaultAddress": swig_vault_address})
        cursor_wire: dict[str, str] | None = None
        if cursor is not None:
            cursor_wire = {
                "slot": _slot_param(cursor.slot, "cursor.slot"),
                "transactionSignature": cursor.transaction_signature,
            }
        body = _list_body(
            network, self._default_network, page_size, cursor_wire, filters
        )
        return _normalize_transactions_page(
            await self._http.post(
                f"{_swig_path(swig_config_address)}/dex/transactions",
                body,
                retry=True,
            )
        )

    async def get(
        self,
        *,
        swig_config_address: str,
        transaction_signature: str,
        network: Network | None = None,
    ) -> DexTransaction:
        """Fetch one indexed DEX transaction of the Swig, shaped like a list item.

        A transaction the index did not interpret answers 404. Network resolves
        from the call, then client.
        This GET uses the client retry policy. API failures raise SDK errors."""
        query = {
            "network": to_proto_network(require_network(network, self._default_network))
        }
        body = _mapping(
            await self._http.get(
                f"{_swig_path(swig_config_address)}/dex/transactions/"
                f"{quote(transaction_signature, safe='')}?{urlencode(query)}"
            ),
            "DEX transaction response",
        )
        return _normalize_transaction(_required(body.get("transaction"), "transaction"))


class DexPositionsClient:
    def __init__(
        self, http: HttpClient, default_network: Network | None = None
    ) -> None:
        self._http = http
        self._default_network = default_network

    async def list(
        self,
        *,
        swig_config_address: str,
        network: Network | None = None,
        page_size: int | None = None,
        cursor: DexPositionCursor | None = None,
        position_status: DexPositionStatus | None = None,
        protocol: DexProtocol | None = None,
        swig_vault_address: str | None = None,
    ) -> ListDexPositionsResult:
        """List the latest state of the Swig's DEX positions, a page per call.

        Positions held by the Swig's vault or any of its sub-accounts, ordered by
        protocol and position address. Every filter is optional, applies to the
        position, and they combine with AND. Pass next_cursor back as cursor
        with the same filters; nothing pages automatically. Network resolves
        from the call, then client.
        This list only reads, so it uses the client retry policy. API failures
        raise SDK errors."""
        filters: list[dict[str, str]] = []
        if position_status is not None:
            filters.append({"positionStatus": _position_status_wire(position_status)})
        if protocol is not None:
            filters.append({"protocol": _protocol_wire(protocol)})
        if swig_vault_address is not None:
            filters.append({"swigVaultAddress": swig_vault_address})
        cursor_wire: dict[str, str] | None = None
        if cursor is not None:
            cursor_wire = {
                "protocol": _protocol_wire(cursor.protocol),
                "dexPositionAddress": cursor.dex_position_address,
            }
        body = _list_body(
            network, self._default_network, page_size, cursor_wire, filters
        )
        return _normalize_positions_page(
            await self._http.post(
                f"{_swig_path(swig_config_address)}/dex/positions",
                body,
                retry=True,
            )
        )


class DexClient:
    """Read-only DEX history and latest positions of a Swig the API key's
    organization owns."""

    def __init__(
        self, http: HttpClient, default_network: Network | None = None
    ) -> None:
        self.transactions = DexTransactionsClient(http, default_network)
        self.positions = DexPositionsClient(http, default_network)


def _list_body(
    network: Network | None,
    default_network: Network | None,
    page_size: int | None,
    cursor: dict[str, str] | None,
    filters: list[dict[str, str]],
) -> dict[str, object]:
    body: dict[str, object] = {
        "network": to_proto_network(require_network(network, default_network))
    }
    if page_size is not None:
        if (
            isinstance(page_size, bool)
            or not isinstance(page_size, int)
            or page_size < 0
        ):
            raise ValueError("page_size must be a non-negative integer")
        body["pageSize"] = page_size
    if cursor is not None:
        body["cursor"] = cursor
    if filters:
        body["filters"] = filters
    return body


def _swig_path(swig_config_address: str) -> str:
    return f"/wallet/api/swig/{quote(swig_config_address, safe='')}"


def _slot_param(slot: int, field: str) -> str:
    """uint64 crosses the wire as a decimal string."""
    if isinstance(slot, bool) or not isinstance(slot, int) or slot < 0:
        raise ValueError(f"{field} must be a non-negative integer")
    return str(slot)


def _protocol_wire(protocol: DexProtocol) -> str:
    return _wire(_PROTOCOL_WIRE, protocol, "protocol must be a supported DEX protocol")


def _action_type_wire(action_type: DexActionType) -> str:
    return _wire(
        _ACTION_TYPE_WIRE,
        action_type,
        "action_type must be a supported DEX action type",
    )


def _position_status_wire(position_status: DexPositionStatus) -> str:
    return _wire(
        _POSITION_STATUS_WIRE,
        position_status,
        'position_status must be "open" or "closed"',
    )


def _wire(table: Mapping[str, str], value: str, message: str) -> str:
    wire = table.get(value)
    if wire is None:
        raise ValueError(message)
    return wire


def _normalize_transactions_page(response: object) -> ListDexTransactionsResult:
    body = _mapping(response, "DEX transactions response")
    transactions = _sequence(body.get("transactions") or [], "transactions")
    next_cursor = body.get("nextCursor")
    return ListDexTransactionsResult(
        transactions=tuple(_normalize_transaction(item) for item in transactions),
        next_cursor=None
        if next_cursor is None
        else _normalize_transaction_cursor(next_cursor),
    )


def _normalize_transaction_cursor(value: object) -> DexTransactionCursor:
    body = _mapping(value, "DEX cursor")
    return DexTransactionCursor(
        slot=_integer(body.get("slot"), "slot"),
        transaction_signature=_required_string(
            body.get("transactionSignature"),
            "transactionSignature",
        ),
    )


def _normalize_transaction(value: object) -> DexTransaction:
    body = _mapping(value, "DEX transaction")
    actions = _sequence(body.get("actions") or [], "actions")
    return DexTransaction(
        swig_config_address=_required_string(
            body.get("swigConfigAddress"),
            "swigConfigAddress",
        ),
        transaction_signature=_required_string(
            body.get("transactionSignature"),
            "transactionSignature",
        ),
        slot=_integer(body.get("slot"), "slot"),
        actions=tuple(_normalize_transaction_action(item) for item in actions),
        block_time=_optional_string(body.get("blockTime")),
    )


def _normalize_transaction_action(value: object) -> DexTransactionAction:
    body = _mapping(value, "DEX action")
    return DexTransactionAction(
        action_index=_integer(body.get("actionIndex"), "actionIndex"),
        swig_vault_address=_required_string(
            body.get("swigVaultAddress"), "swigVaultAddress"
        ),
        acting_role_id=_integer(body.get("actingRoleId"), "actingRoleId"),
        protocol=_normalize_protocol(body.get("protocol")),
        dex_program_address=_required_string(
            body.get("dexProgramAddress"),
            "dexProgramAddress",
        ),
        action=_normalize_action(body.get("action")),
    )


def _normalize_action(value: object) -> DexAction:
    """The payload's one key names the action, so the type cannot disagree with it."""
    payload = _mapping(_required(value, "action"), "DEX action payload")
    swap = payload.get("swap")
    if swap is not None:
        body = _mapping(swap, "swap")
        return DexSwapAction(
            protocol_instruction=_instruction(body),
            protocol_actor_address=_actor(body),
            dex_pool_address=_pool(body),
            input=_normalize_amount(body.get("input"), "input"),
            output=_normalize_amount(body.get("output"), "output"),
        )
    position_open = payload.get("positionOpen")
    if position_open is not None:
        body = _mapping(position_open, "positionOpen")
        return DexPositionOpenAction(
            protocol_instruction=_instruction(body),
            protocol_actor_address=_actor(body),
            dex_pool_address=_pool(body),
            dex_position_address=_position(body),
            deposits=_amounts(body.get("deposits")),
        )
    liquidity_add = payload.get("liquidityAdd")
    if liquidity_add is not None:
        body = _mapping(liquidity_add, "liquidityAdd")
        return DexLiquidityAddAction(
            protocol_instruction=_instruction(body),
            protocol_actor_address=_actor(body),
            dex_pool_address=_pool(body),
            dex_position_address=_position(body),
            deposits=_amounts(body.get("deposits")),
        )
    liquidity_remove = payload.get("liquidityRemove")
    if liquidity_remove is not None:
        body = _mapping(liquidity_remove, "liquidityRemove")
        return DexLiquidityRemoveAction(
            protocol_instruction=_instruction(body),
            protocol_actor_address=_actor(body),
            dex_pool_address=_pool(body),
            dex_position_address=_position(body),
            withdrawals=_amounts(body.get("withdrawals")),
        )
    rebalance = payload.get("positionRebalance")
    if rebalance is not None:
        body = _mapping(rebalance, "positionRebalance")
        return DexPositionRebalanceAction(
            protocol_instruction=_instruction(body),
            protocol_actor_address=_actor(body),
            dex_pool_address=_pool(body),
            dex_position_address=_position(body),
            deposits=_amounts(body.get("deposits")),
            withdrawals=_amounts(body.get("withdrawals")),
        )
    fee_claim = payload.get("feeClaim")
    if fee_claim is not None:
        body = _mapping(fee_claim, "feeClaim")
        return DexFeeClaimAction(
            protocol_instruction=_instruction(body),
            protocol_actor_address=_actor(body),
            dex_pool_address=_pool(body),
            dex_position_address=_position(body),
            claimed=_amounts(body.get("claimed")),
        )
    reward_claim = payload.get("rewardClaim")
    if reward_claim is not None:
        body = _mapping(reward_claim, "rewardClaim")
        return DexRewardClaimAction(
            protocol_instruction=_instruction(body),
            protocol_actor_address=_actor(body),
            dex_pool_address=_pool(body),
            dex_position_address=_position(body),
            claimed=_amounts(body.get("claimed")),
        )
    position_close = payload.get("positionClose")
    if position_close is not None:
        body = _mapping(position_close, "positionClose")
        return DexPositionCloseAction(
            protocol_instruction=_instruction(body),
            protocol_actor_address=_actor(body),
            dex_position_address=_position(body),
            dex_pool_address=_optional_string(body.get("dexPoolAddress")),
        )
    raise SwigResponseError("DEX response has an unknown action")


def _instruction(body: Mapping[str, object]) -> str:
    return _required_string(
        body.get("protocolInstruction"),
        "protocolInstruction",
    )


def _actor(body: Mapping[str, object]) -> str:
    return _required_string(
        body.get("protocolActorAddress"),
        "protocolActorAddress",
    )


def _pool(body: Mapping[str, object]) -> str:
    return _required_string(body.get("dexPoolAddress"), "dexPoolAddress")


def _position(body: Mapping[str, object]) -> str:
    return _required_string(
        body.get("dexPositionAddress"),
        "dexPositionAddress",
    )


def _amounts(value: object) -> tuple[DexAssetAmount, ...]:
    return tuple(
        _normalize_amount(item, "amount") for item in _sequence(value or [], "amounts")
    )


def _normalize_amount(value: object, field: str) -> DexAssetAmount:
    body = _mapping(_required(value, field), field)
    decimals = body.get("decimals")
    return DexAssetAmount(
        mint_address=_required_string(body.get("mintAddress"), "mintAddress"),
        amount_raw=_units(body.get("amountRaw"), "amountRaw", _SIGNED_INTEGER),
        decimals=None if decimals is None else _integer(decimals, "decimals"),
    )


def _normalize_positions_page(response: object) -> ListDexPositionsResult:
    body = _mapping(response, "DEX positions response")
    positions = _sequence(body.get("positions") or [], "positions")
    next_cursor = body.get("nextCursor")
    return ListDexPositionsResult(
        positions=tuple(_normalize_position(item) for item in positions),
        next_cursor=None
        if next_cursor is None
        else _normalize_position_cursor(next_cursor),
    )


def _normalize_position_cursor(value: object) -> DexPositionCursor:
    body = _mapping(value, "DEX cursor")
    return DexPositionCursor(
        protocol=_normalize_protocol(body.get("protocol")),
        dex_position_address=_required_string(
            body.get("dexPositionAddress"),
            "dexPositionAddress",
        ),
    )


def _normalize_position(value: object) -> DexPosition:
    body = _mapping(value, "DEX position")
    swig_config_address = _required_string(
        body.get("swigConfigAddress"), "swigConfigAddress"
    )
    swig_vault_address = _required_string(
        body.get("swigVaultAddress"), "swigVaultAddress"
    )
    protocol = _normalize_protocol(body.get("protocol"))
    dex_position_address = _required_string(
        body.get("dexPositionAddress"),
        "dexPositionAddress",
    )
    updated_slot = _integer(body.get("updatedSlot"), "updatedSlot")
    source_transaction_signature = _required_string(
        body.get("sourceTransactionSignature"),
        "sourceTransactionSignature",
    )
    opened = body.get("open")
    if opened is not None:
        state = _mapping(opened, "open")
        return DexOpenPosition(
            swig_config_address=swig_config_address,
            swig_vault_address=swig_vault_address,
            protocol=protocol,
            dex_position_address=dex_position_address,
            updated_slot=updated_slot,
            source_transaction_signature=source_transaction_signature,
            dex_pool_address=_required_string(
                state.get("dexPoolAddress"), "dexPoolAddress"
            ),
            state=_normalize_account_state(state.get("state")),
        )
    # Key presence, not truthiness: a closed position arrives as an empty object.
    if "closed" in body:
        return DexClosedPosition(
            swig_config_address=swig_config_address,
            swig_vault_address=swig_vault_address,
            protocol=protocol,
            dex_position_address=dex_position_address,
            updated_slot=updated_slot,
            source_transaction_signature=source_transaction_signature,
        )
    raise SwigResponseError("DEX response is missing position state")


def _normalize_account_state(value: object) -> DexPositionAccountState:
    state = _mapping(value or {}, "position account state")
    clmm = state.get("raydiumClmm")
    if clmm is None:
        raise SwigResponseError("DEX response is missing position account state")
    body = _mapping(clmm, "raydiumClmm")
    rewards = _sequence(
        body.get("rewardAmountsOwed") or [],
        "rewardAmountsOwed",
    )
    return DexRaydiumClmmPositionState(
        liquidity_raw=_units(
            body.get("liquidityRaw"),
            "liquidityRaw",
            _UNSIGNED_INTEGER,
        ),
        tick_lower_index=_integer(body.get("tickLowerIndex"), "tickLowerIndex"),
        tick_upper_index=_integer(body.get("tickUpperIndex"), "tickUpperIndex"),
        token_fees_owed_0=_units(
            body.get("tokenFeesOwed0"),
            "tokenFeesOwed0",
            _UNSIGNED_INTEGER,
        ),
        token_fees_owed_1=_units(
            body.get("tokenFeesOwed1"),
            "tokenFeesOwed1",
            _UNSIGNED_INTEGER,
        ),
        reward_amounts_owed=tuple(
            _units(item, "rewardAmountsOwed", _UNSIGNED_INTEGER) for item in rewards
        ),
    )


def _normalize_protocol(value: object) -> DexProtocol:
    protocol = _PROTOCOLS.get(value) if isinstance(value, str) else None
    if protocol is None:
        raise SwigResponseError("DEX response has invalid protocol")
    return protocol


def _mapping(value: object, label: str) -> Mapping[str, object]:
    if isinstance(value, Mapping):
        return value
    raise SwigResponseError(f"{label} must be an object")


def _sequence(value: object, field: str) -> Sequence[object]:
    if isinstance(value, Sequence) and not isinstance(value, (str, bytes)):
        return value
    raise SwigResponseError(f"DEX response has invalid {field}")


def _required(value: object, field: str) -> object:
    if value is None:
        raise SwigResponseError(f"DEX response is missing {field}")
    return value


def _required_string(value: object, field: str) -> str:
    if isinstance(value, str) and value:
        return value
    raise SwigResponseError(f"DEX response is missing {field}")


def _optional_string(value: object) -> str | None:
    return value if isinstance(value, str) and value else None


def _units(value: object, field: str, pattern: re.Pattern[str]) -> str:
    """A decimal integer string, kept as one so a value past 2^53 survives."""
    if not isinstance(value, str) or not pattern.fullmatch(value):
        raise SwigResponseError(f"DEX response has invalid {field}")
    return value


def _integer(value: object, field: str) -> int:
    """uint64 arrives as a decimal string and 32-bit integers as numbers."""
    if isinstance(value, bool):
        raise SwigResponseError(f"DEX response has invalid {field}")
    if isinstance(value, int):
        return value
    if isinstance(value, str) and _SIGNED_INTEGER.fullmatch(value):
        return int(value)
    raise SwigResponseError(f"DEX response has invalid {field}")
