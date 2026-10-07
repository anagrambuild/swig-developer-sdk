from __future__ import annotations

import re
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import Literal, TypeAlias
from urllib.parse import quote, urlencode

from .common import Network, require_network, to_proto_network
from .core import HttpClient

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
class ListDexTransactionsResult:
    transactions: tuple[DexTransaction, ...]
    #: None on the last page; otherwise pass it back as ``page_token`` with the
    #: same filters.
    next_page_token: str | None = None


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
    observed_slot: int
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
    observed_slot: int
    source_transaction_signature: str
    status: Literal["closed"] = "closed"


DexPosition: TypeAlias = DexOpenPosition | DexClosedPosition


@dataclass(frozen=True, slots=True)
class ListDexPositionsResult:
    positions: tuple[DexPosition, ...]
    #: None on the last page; otherwise pass it back as ``page_token`` with the
    #: same filters.
    next_page_token: str | None = None


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
        page_token: str | None = None,
        start_slot: int | None = None,
        end_slot: int | None = None,
        protocol: DexProtocol | None = None,
        action_type: DexActionType | None = None,
        swig_vault_address: str | None = None,
    ) -> ListDexTransactionsResult:
        """Newest first. A transaction the index could not interpret is not listed.

        Every filter is optional and they combine; an action filter selects
        transactions with at least one matching action.
        """
        query = _page_query(network, self._default_network, page_size, page_token)
        if start_slot is not None:
            query["startSlot"] = _slot_param(start_slot, "start_slot")
        if end_slot is not None:
            query["endSlot"] = _slot_param(end_slot, "end_slot")
        if protocol is not None:
            query["protocol"] = _protocol_wire(protocol)
        if action_type is not None:
            query["actionType"] = _wire(
                _ACTION_TYPE_WIRE,
                action_type,
                "action_type must be a supported DEX action type",
            )
        if swig_vault_address is not None:
            query["swigVaultAddress"] = swig_vault_address
        return _normalize_transactions_page(
            await self._http.get(
                f"{_swig_path(swig_config_address)}/dex/transactions?{urlencode(query)}"
            )
        )

    async def get(
        self,
        *,
        swig_config_address: str,
        transaction_signature: str,
        network: Network | None = None,
    ) -> DexTransaction:
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
        page_token: str | None = None,
        position_status: DexPositionStatus | None = None,
        protocol: DexProtocol | None = None,
        swig_vault_address: str | None = None,
    ) -> ListDexPositionsResult:
        """Every filter is optional, they combine, and each applies to the position."""
        query = _page_query(network, self._default_network, page_size, page_token)
        if position_status is not None:
            query["positionStatus"] = _wire(
                _POSITION_STATUS_WIRE,
                position_status,
                'position_status must be "open" or "closed"',
            )
        if protocol is not None:
            query["protocol"] = _protocol_wire(protocol)
        if swig_vault_address is not None:
            query["swigVaultAddress"] = swig_vault_address
        return _normalize_positions_page(
            await self._http.get(
                f"{_swig_path(swig_config_address)}/dex/positions?{urlencode(query)}"
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


def _page_query(
    network: Network | None,
    default_network: Network | None,
    page_size: int | None,
    page_token: str | None,
) -> dict[str, str]:
    query = {"network": to_proto_network(require_network(network, default_network))}
    if page_size is not None:
        if (
            isinstance(page_size, bool)
            or not isinstance(page_size, int)
            or page_size < 0
        ):
            raise ValueError("page_size must be a non-negative integer")
        query["pageSize"] = str(page_size)
    if page_token:
        query["pageToken"] = page_token
    return query


def _swig_path(swig_config_address: str) -> str:
    return f"/wallet/swig/{quote(swig_config_address, safe='')}"


def _slot_param(slot: int, field: str) -> str:
    """uint64 crosses the wire as a decimal string."""
    if isinstance(slot, bool) or not isinstance(slot, int) or slot < 0:
        raise ValueError(f"{field} must be a non-negative integer")
    return str(slot)


def _protocol_wire(protocol: DexProtocol) -> str:
    return _wire(_PROTOCOL_WIRE, protocol, "protocol must be a supported DEX protocol")


def _wire(table: Mapping[str, str], value: str, message: str) -> str:
    wire = table.get(value)
    if wire is None:
        raise ValueError(message)
    return wire


def _normalize_transactions_page(response: object) -> ListDexTransactionsResult:
    body = _mapping(response, "DEX transactions response")
    transactions = _sequence(body.get("transactions") or [], "transactions")
    return ListDexTransactionsResult(
        transactions=tuple(_normalize_transaction(item) for item in transactions),
        next_page_token=_optional_string(
            _pick(body, "nextPageToken", "next_page_token")
        ),
    )


def _normalize_transaction(value: object) -> DexTransaction:
    body = _mapping(value, "DEX transaction")
    actions = _sequence(body.get("actions") or [], "actions")
    return DexTransaction(
        swig_config_address=_required_string(
            _pick(body, "swigConfigAddress", "swig_config_address"),
            "swigConfigAddress",
        ),
        transaction_signature=_required_string(
            _pick(body, "transactionSignature", "transaction_signature"),
            "transactionSignature",
        ),
        slot=_integer(body.get("slot"), "slot"),
        actions=tuple(_normalize_transaction_action(item) for item in actions),
        block_time=_optional_string(_pick(body, "blockTime", "block_time")),
    )


def _normalize_transaction_action(value: object) -> DexTransactionAction:
    body = _mapping(value, "DEX action")
    return DexTransactionAction(
        action_index=_integer(
            _pick(body, "actionIndex", "action_index"), "actionIndex"
        ),
        swig_vault_address=_required_string(
            _pick(body, "swigVaultAddress", "swig_vault_address"), "swigVaultAddress"
        ),
        acting_role_id=_integer(
            _pick(body, "actingRoleId", "acting_role_id"), "actingRoleId"
        ),
        protocol=_normalize_protocol(body.get("protocol")),
        dex_program_address=_required_string(
            _pick(body, "dexProgramAddress", "dex_program_address"),
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
    position_open = _pick(payload, "positionOpen", "position_open")
    if position_open is not None:
        body = _mapping(position_open, "positionOpen")
        return DexPositionOpenAction(
            protocol_instruction=_instruction(body),
            protocol_actor_address=_actor(body),
            dex_pool_address=_pool(body),
            dex_position_address=_position(body),
            deposits=_amounts(body.get("deposits")),
        )
    liquidity_add = _pick(payload, "liquidityAdd", "liquidity_add")
    if liquidity_add is not None:
        body = _mapping(liquidity_add, "liquidityAdd")
        return DexLiquidityAddAction(
            protocol_instruction=_instruction(body),
            protocol_actor_address=_actor(body),
            dex_pool_address=_pool(body),
            dex_position_address=_position(body),
            deposits=_amounts(body.get("deposits")),
        )
    liquidity_remove = _pick(payload, "liquidityRemove", "liquidity_remove")
    if liquidity_remove is not None:
        body = _mapping(liquidity_remove, "liquidityRemove")
        return DexLiquidityRemoveAction(
            protocol_instruction=_instruction(body),
            protocol_actor_address=_actor(body),
            dex_pool_address=_pool(body),
            dex_position_address=_position(body),
            withdrawals=_amounts(body.get("withdrawals")),
        )
    rebalance = _pick(payload, "positionRebalance", "position_rebalance")
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
    fee_claim = _pick(payload, "feeClaim", "fee_claim")
    if fee_claim is not None:
        body = _mapping(fee_claim, "feeClaim")
        return DexFeeClaimAction(
            protocol_instruction=_instruction(body),
            protocol_actor_address=_actor(body),
            dex_pool_address=_pool(body),
            dex_position_address=_position(body),
            claimed=_amounts(body.get("claimed")),
        )
    reward_claim = _pick(payload, "rewardClaim", "reward_claim")
    if reward_claim is not None:
        body = _mapping(reward_claim, "rewardClaim")
        return DexRewardClaimAction(
            protocol_instruction=_instruction(body),
            protocol_actor_address=_actor(body),
            dex_pool_address=_pool(body),
            dex_position_address=_position(body),
            claimed=_amounts(body.get("claimed")),
        )
    position_close = _pick(payload, "positionClose", "position_close")
    if position_close is not None:
        body = _mapping(position_close, "positionClose")
        return DexPositionCloseAction(
            protocol_instruction=_instruction(body),
            protocol_actor_address=_actor(body),
            dex_position_address=_position(body),
            dex_pool_address=_optional_string(
                _pick(body, "dexPoolAddress", "dex_pool_address")
            ),
        )
    raise ValueError("DEX response has an unknown action")


def _instruction(body: Mapping[str, object]) -> str:
    return _required_string(
        _pick(body, "protocolInstruction", "protocol_instruction"),
        "protocolInstruction",
    )


def _actor(body: Mapping[str, object]) -> str:
    return _required_string(
        _pick(body, "protocolActorAddress", "protocol_actor_address"),
        "protocolActorAddress",
    )


def _pool(body: Mapping[str, object]) -> str:
    return _required_string(
        _pick(body, "dexPoolAddress", "dex_pool_address"), "dexPoolAddress"
    )


def _position(body: Mapping[str, object]) -> str:
    return _required_string(
        _pick(body, "dexPositionAddress", "dex_position_address"),
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
        mint_address=_required_string(
            _pick(body, "mintAddress", "mint_address"), "mintAddress"
        ),
        amount_raw=_units(
            _pick(body, "amountRaw", "amount_raw"), "amountRaw", _SIGNED_INTEGER
        ),
        decimals=None if decimals is None else _integer(decimals, "decimals"),
    )


def _normalize_positions_page(response: object) -> ListDexPositionsResult:
    body = _mapping(response, "DEX positions response")
    positions = _sequence(body.get("positions") or [], "positions")
    return ListDexPositionsResult(
        positions=tuple(_normalize_position(item) for item in positions),
        next_page_token=_optional_string(
            _pick(body, "nextPageToken", "next_page_token")
        ),
    )


def _normalize_position(value: object) -> DexPosition:
    body = _mapping(value, "DEX position")
    swig_config_address = _required_string(
        _pick(body, "swigConfigAddress", "swig_config_address"), "swigConfigAddress"
    )
    swig_vault_address = _required_string(
        _pick(body, "swigVaultAddress", "swig_vault_address"), "swigVaultAddress"
    )
    protocol = _normalize_protocol(body.get("protocol"))
    dex_position_address = _required_string(
        _pick(body, "dexPositionAddress", "dex_position_address"),
        "dexPositionAddress",
    )
    observed_slot = _integer(
        _pick(body, "observedSlot", "observed_slot"), "observedSlot"
    )
    source_transaction_signature = _required_string(
        _pick(body, "sourceTransactionSignature", "source_transaction_signature"),
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
            observed_slot=observed_slot,
            source_transaction_signature=source_transaction_signature,
            dex_pool_address=_required_string(
                _pick(state, "dexPoolAddress", "dex_pool_address"), "dexPoolAddress"
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
            observed_slot=observed_slot,
            source_transaction_signature=source_transaction_signature,
        )
    raise ValueError("DEX response is missing position state")


def _normalize_account_state(value: object) -> DexPositionAccountState:
    state = _mapping(value or {}, "position account state")
    clmm = _pick(state, "raydiumClmm", "raydium_clmm")
    if clmm is None:
        raise ValueError("DEX response is missing position account state")
    body = _mapping(clmm, "raydiumClmm")
    rewards = _sequence(
        _pick(body, "rewardAmountsOwed", "reward_amounts_owed") or [],
        "rewardAmountsOwed",
    )
    return DexRaydiumClmmPositionState(
        liquidity_raw=_units(
            _pick(body, "liquidityRaw", "liquidity_raw"),
            "liquidityRaw",
            _UNSIGNED_INTEGER,
        ),
        tick_lower_index=_integer(
            _pick(body, "tickLowerIndex", "tick_lower_index"), "tickLowerIndex"
        ),
        tick_upper_index=_integer(
            _pick(body, "tickUpperIndex", "tick_upper_index"), "tickUpperIndex"
        ),
        token_fees_owed_0=_units(
            _pick(body, "tokenFeesOwed0", "token_fees_owed_0"),
            "tokenFeesOwed0",
            _UNSIGNED_INTEGER,
        ),
        token_fees_owed_1=_units(
            _pick(body, "tokenFeesOwed1", "token_fees_owed_1"),
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
        raise ValueError("DEX response has invalid protocol")
    return protocol


def _mapping(value: object, label: str) -> Mapping[str, object]:
    if isinstance(value, Mapping):
        return value
    raise ValueError(f"{label} must be an object")


def _sequence(value: object, field: str) -> Sequence[object]:
    if isinstance(value, Sequence) and not isinstance(value, (str, bytes)):
        return value
    raise ValueError(f"DEX response has invalid {field}")


def _pick(value: Mapping[str, object], *keys: str) -> object:
    for key in keys:
        if key in value:
            return value[key]
    return None


def _required(value: object, field: str) -> object:
    if value is None:
        raise ValueError(f"DEX response is missing {field}")
    return value


def _required_string(value: object, field: str) -> str:
    if isinstance(value, str) and value:
        return value
    raise ValueError(f"DEX response is missing {field}")


def _optional_string(value: object) -> str | None:
    return value if isinstance(value, str) and value else None


def _units(value: object, field: str, pattern: re.Pattern[str]) -> str:
    """A decimal integer string, kept as one so a value past 2^53 survives."""
    if not isinstance(value, str) or not pattern.fullmatch(value):
        raise ValueError(f"DEX response has invalid {field}")
    return value


def _integer(value: object, field: str) -> int:
    """uint64 arrives as a decimal string and 32-bit integers as numbers."""
    if isinstance(value, bool):
        raise ValueError(f"DEX response has invalid {field}")
    if isinstance(value, int):
        return value
    if isinstance(value, str) and _SIGNED_INTEGER.fullmatch(value):
        return int(value)
    raise ValueError(f"DEX response has invalid {field}")
