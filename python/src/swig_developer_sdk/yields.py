"""Yield on Swig wallets. Native Solana staking is the first yield method.

Every call is scoped to one Swig wallet config. Prepare calls return unsigned
transactions: authorize them with the requester authority, sign with the fee
payer, then submit or sponsor them. The module is ``yields`` and the client
attribute is ``SwigClient.yields`` because ``yield`` is a Python keyword.
"""

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass, field
from typing import Literal, TypeAlias
from urllib.parse import quote, urlencode

from .common import (
    Amount,
    Network,
    WalletAuthority,
    normalize_amount,
    require_network,
    to_proto_network,
    wallet_authority_to_wire,
)
from .core import HttpClient
from .errors import SwigResponseError
from .transactions import PreparedTransaction, normalize_prepared_transaction
from .wallets import (
    AddRoleAction,
    AllAction,
    AllButManageAuthorityAction,
    ManageAuthorityAction,
    ProgramAction,
    ProgramAllAction,
    ProgramCuratedAction,
    SolDestinationLimitAction,
    SolLimitAction,
    SolRecurringDestinationLimitAction,
    SolRecurringLimitAction,
    StakeAllAction,
    StakeLimitAction,
    StakeRecurringLimitAction,
)

YieldPositionState: TypeAlias = Literal[
    "activating", "active", "deactivating", "inactive", "unspecified"
]


@dataclass(frozen=True, slots=True)
class ExistingYieldRole:
    role_id: int
    actions: list[AddRoleAction]
    type: Literal["existing"] = "existing"


@dataclass(frozen=True, slots=True)
class PreparedYieldRoleCreation:
    """Unsigned AddRole transaction for the role manager and fee payer."""

    transaction: PreparedTransaction
    yield_authority: Mapping[str, object]
    actions: list[AddRoleAction]
    type: Literal["create"] = "create"


YieldRoleResult: TypeAlias = ExistingYieldRole | PreparedYieldRoleCreation


@dataclass(frozen=True, slots=True)
class PartialYieldExit:
    """Delegated stake to exit, into a new position derived from the seed."""

    lamports: Amount
    position_seed: str


@dataclass(frozen=True, slots=True)
class PreparedYieldDeposit:
    transaction: PreparedTransaction
    position_address: str
    validator_vote_address: str
    delegated_lamports: str
    rent_lamports: str


@dataclass(frozen=True, slots=True)
class PreparedYieldExit:
    transaction: PreparedTransaction
    exiting_position_address: str
    deactivating_lamports: str
    additional_rent_lamports: str


@dataclass(frozen=True, slots=True)
class PreparedYieldWithdrawal:
    transaction: PreparedTransaction
    withdrawal_lamports: str
    destination_wallet_address: str


@dataclass(frozen=True, slots=True)
class YieldRate:
    validator_vote_address: str
    estimated_net_apr_bps: int
    commission_bps: int
    last_updated_at: str | None = None


@dataclass(frozen=True, slots=True)
class YieldOption:
    vote_address: str
    name: str
    accepts_deposits: bool
    rate: YieldRate | None = None


@dataclass(frozen=True, slots=True)
class YieldPosition:
    swig_config_address: str
    wallet_address: str
    position_address: str
    state: YieldPositionState
    balance_lamports: str
    rent_lamports: str
    position_lamports: str
    withdrawable_lamports: str
    earning_lamports: str
    validator_vote_address: str | None = None


@dataclass(frozen=True, slots=True)
class YieldPositionsPage:
    positions: list[YieldPosition] = field(default_factory=list)
    next_page_token: str | None = None
    last_updated_at: str | None = None


@dataclass(frozen=True, slots=True)
class YieldSummary:
    total_position_lamports: str
    withdrawable_lamports: str
    recorded_rewards_lamports: str


@dataclass(frozen=True, slots=True)
class YieldReward:
    swig_config_address: str
    position_address: str
    amount_lamports: str
    earned_at: str | None = None


@dataclass(frozen=True, slots=True)
class YieldRewardsPage:
    rewards: list[YieldReward] = field(default_factory=list)
    next_page_token: str | None = None


@dataclass(frozen=True, slots=True)
class YieldProjection:
    principal_lamports: str
    horizon_days: int
    estimated_rewards_lamports: str
    estimated_total_lamports: str
    rate: YieldRate | None = None


class YieldClient:
    """Yield calls for Swig wallets, available as ``SwigClient.yields``."""

    def __init__(
        self, http: HttpClient, default_network: Network | None = None
    ) -> None:
        self._http = http
        self._default_network = default_network

    async def prepare_role(
        self,
        *,
        swig_config_address: str,
        fee_payer: str,
        yield_authority: WalletAuthority,
        validator_vote_address: str,
        max_deposit: Amount,
        role_id: int | None = None,
        role_manager_authority: WalletAuthority | None = None,
        network: Network | None = None,
    ) -> YieldRoleResult:
        """Reuse a role that can deposit, exit and withdraw, or prepare one.

        A ``create`` result holds an unsigned AddRole transaction. After it
        confirms, call again without ``role_id`` to resolve the new role."""
        body: dict[str, object] = {
            "network": self._network(network),
            "swigConfigAddress": swig_config_address,
            "feePayer": fee_payer,
            "yieldAuthority": wallet_authority_to_wire(yield_authority),
            "validatorVoteAddress": validator_vote_address,
            "maxDepositLamports": normalize_amount(max_deposit),
        }
        if role_id is not None:
            body["roleId"] = role_id
        if role_manager_authority is not None:
            body["roleManagerAuthority"] = wallet_authority_to_wire(
                role_manager_authority
            )
        return normalize_yield_role(
            await self._http.post(f"{_base(swig_config_address)}/role/prepare", body)
        )

    async def deposit(
        self,
        *,
        swig_config_address: str,
        fee_payer: str,
        requester_authority: WalletAuthority,
        amount: Amount,
        validator_vote_address: str,
        position_seed: str,
        role_id: int | None = None,
        network: Network | None = None,
    ) -> PreparedYieldDeposit:
        """Fund a new native position from the wallet and delegate it."""
        response = await self._http.post(
            f"{_base(swig_config_address)}/deposit/prepare",
            {
                "wallet": self._wallet(
                    swig_config_address,
                    fee_payer,
                    requester_authority,
                    role_id,
                    network,
                ),
                "lamports": normalize_amount(amount),
                "validatorVoteAddress": validator_vote_address,
                "positionSeed": position_seed,
            },
        )
        body = _mapping(response, "Yield deposit response")
        return PreparedYieldDeposit(
            transaction=_transaction(body),
            position_address=_required(body, "positionAddress", "position_address"),
            validator_vote_address=_required(
                body, "validatorVoteAddress", "validator_vote_address"
            ),
            delegated_lamports=_lamports(
                body, "delegatedLamports", "delegated_lamports"
            ),
            rent_lamports=_lamports(body, "rentLamports", "rent_lamports"),
        )

    async def exit(
        self,
        *,
        swig_config_address: str,
        fee_payer: str,
        requester_authority: WalletAuthority,
        position_address: str,
        amount: Literal["all"] | PartialYieldExit,
        role_id: int | None = None,
        network: Network | None = None,
    ) -> PreparedYieldExit:
        """Deactivate a position, or split part of an active one and deactivate that."""
        body: dict[str, object] = {
            "wallet": self._wallet(
                swig_config_address, fee_payer, requester_authority, role_id, network
            ),
            "positionAddress": position_address,
        }
        if amount == "all":
            body["all"] = {}
        else:
            body["partial"] = {
                "lamports": normalize_amount(amount.lamports),
                "positionSeed": amount.position_seed,
            }
        response = _mapping(
            await self._http.post(f"{_base(swig_config_address)}/exit/prepare", body),
            "Yield exit response",
        )
        return PreparedYieldExit(
            transaction=_transaction(response),
            exiting_position_address=_required(
                response, "exitingPositionAddress", "exiting_position_address"
            ),
            deactivating_lamports=_lamports(
                response, "deactivatingLamports", "deactivating_lamports"
            ),
            additional_rent_lamports=_lamports(
                response, "additionalRentLamports", "additional_rent_lamports"
            ),
        )

    async def withdraw(
        self,
        *,
        swig_config_address: str,
        fee_payer: str,
        requester_authority: WalletAuthority,
        position_address: str,
        role_id: int | None = None,
        network: Network | None = None,
    ) -> PreparedYieldWithdrawal:
        """Withdraw an inactive position's balance back to the Swig wallet."""
        response = _mapping(
            await self._http.post(
                f"{_base(swig_config_address)}/withdraw/prepare",
                {
                    "wallet": self._wallet(
                        swig_config_address,
                        fee_payer,
                        requester_authority,
                        role_id,
                        network,
                    ),
                    "positionAddress": position_address,
                },
            ),
            "Yield withdrawal response",
        )
        return PreparedYieldWithdrawal(
            transaction=_transaction(response),
            withdrawal_lamports=_lamports(
                response, "withdrawalLamports", "withdrawal_lamports"
            ),
            destination_wallet_address=_required(
                response, "destinationWalletAddress", "destination_wallet_address"
            ),
        )

    async def list_options(
        self, *, swig_config_address: str, network: Network | None = None
    ) -> list[YieldOption]:
        """Allowlisted validators with live rates cached for 15 seconds."""
        body = _mapping(
            await self._http.get(
                f"{_base(swig_config_address)}/options?{self._query(network)}"
            ),
            "Yield options response",
        )
        return [
            _option(_mapping(item, "Yield option"))
            for item in _list(body.get("options"))
        ]

    async def list_positions(
        self,
        *,
        swig_config_address: str,
        page_size: int | None = None,
        page_token: str | None = None,
        network: Network | None = None,
    ) -> YieldPositionsPage:
        """Indexed positions sorted by address. Page size: default 50, max 200."""
        body = _mapping(
            await self._http.get(
                f"{_base(swig_config_address)}/positions?"
                f"{self._query(network, _page(page_size, page_token))}"
            ),
            "Yield positions response",
        )
        return YieldPositionsPage(
            positions=[
                _position(_mapping(item, "Yield position"))
                for item in _list(body.get("positions"))
            ],
            next_page_token=_optional(body, "nextPageToken", "next_page_token"),
            last_updated_at=_optional(body, "lastUpdatedAt", "last_updated_at"),
        )

    async def get_summary(
        self, *, swig_config_address: str, network: Network | None = None
    ) -> YieldSummary:
        """Totals across indexed positions and recorded rewards."""
        body = _mapping(
            await self._http.get(
                f"{_base(swig_config_address)}/summary?{self._query(network)}"
            ),
            "Yield summary response",
        )
        return YieldSummary(
            total_position_lamports=_lamports(
                body, "totalPositionLamports", "total_position_lamports"
            ),
            withdrawable_lamports=_lamports(
                body, "withdrawableLamports", "withdrawable_lamports"
            ),
            recorded_rewards_lamports=_lamports(
                body, "recordedRewardsLamports", "recorded_rewards_lamports"
            ),
        )

    async def list_rewards(
        self,
        *,
        swig_config_address: str,
        page_size: int | None = None,
        page_token: str | None = None,
        network: Network | None = None,
    ) -> YieldRewardsPage:
        """Recorded reward credits ordered by on-chain credit time."""
        body = _mapping(
            await self._http.get(
                f"{_base(swig_config_address)}/rewards?"
                f"{self._query(network, _page(page_size, page_token))}"
            ),
            "Yield rewards response",
        )
        rewards = []
        for item in _list(body.get("rewards")):
            reward = _mapping(item, "Yield reward")
            rewards.append(
                YieldReward(
                    swig_config_address=_optional(
                        reward, "swigConfigAddress", "swig_config_address"
                    )
                    or "",
                    position_address=_optional(
                        reward, "positionAddress", "position_address"
                    )
                    or "",
                    amount_lamports=_lamports(
                        reward, "amountLamports", "amount_lamports"
                    ),
                    earned_at=_optional(reward, "earnedAt", "earned_at"),
                )
            )
        return YieldRewardsPage(
            rewards=rewards,
            next_page_token=_optional(body, "nextPageToken", "next_page_token"),
        )

    async def estimate(
        self,
        *,
        swig_config_address: str,
        validator_vote_address: str,
        principal: Amount,
        horizon_days: int,
        network: Network | None = None,
    ) -> YieldProjection:
        """Project rewards at the validator's current net APR, rounded down."""
        if not 1 <= horizon_days <= 365:
            raise ValueError("horizon_days must be between 1 and 365")
        body = _mapping(
            await self._http.get(
                f"{_base(swig_config_address)}/estimate?"
                + self._query(
                    network,
                    {
                        "validatorVoteAddress": validator_vote_address,
                        "principalLamports": normalize_amount(principal),
                        "horizonDays": str(horizon_days),
                    },
                )
            ),
            "Yield estimate response",
        )
        projection = _mapping(body.get("projection"), "Yield projection")
        return YieldProjection(
            principal_lamports=_lamports(
                projection, "principalLamports", "principal_lamports"
            ),
            horizon_days=_int(_pick(projection, "horizonDays", "horizon_days")),
            estimated_rewards_lamports=_lamports(
                projection, "estimatedRewardsLamports", "estimated_rewards_lamports"
            ),
            estimated_total_lamports=_lamports(
                projection, "estimatedTotalLamports", "estimated_total_lamports"
            ),
            rate=_rate(projection.get("rate")),
        )

    def _network(self, network: Network | None) -> str:
        return to_proto_network(require_network(network, self._default_network))

    def _wallet(
        self,
        swig_config_address: str,
        fee_payer: str,
        requester_authority: WalletAuthority,
        role_id: int | None,
        network: Network | None,
    ) -> dict[str, object]:
        wallet: dict[str, object] = {
            "network": self._network(network),
            "swigConfigAddress": swig_config_address,
            "feePayer": fee_payer,
            "requesterAuthority": wallet_authority_to_wire(requester_authority),
        }
        if role_id is not None:
            wallet["roleId"] = role_id
        return wallet

    def _query(
        self, network: Network | None, extra: Mapping[str, str] | None = None
    ) -> str:
        return urlencode({"network": self._network(network), **(extra or {})})


def _base(swig_config_address: str) -> str:
    if not swig_config_address:
        raise ValueError("swig_config_address is required")
    return f"/api/yield/{quote(swig_config_address, safe='')}"


def _page(page_size: int | None, page_token: str | None) -> dict[str, str]:
    if page_size is not None and not 1 <= page_size <= 200:
        raise ValueError("page_size must be between 1 and 200")
    query: dict[str, str] = {}
    if page_size is not None:
        query["pageSize"] = str(page_size)
    if page_token:
        query["pageToken"] = page_token
    return query


def _mapping(value: object, label: str) -> Mapping[str, object]:
    if isinstance(value, Mapping):
        return value
    raise SwigResponseError(f"{label} must be an object")


def _list(value: object) -> list[object]:
    return list(value) if isinstance(value, list) else []


def _pick(value: Mapping[str, object], *keys: str) -> object:
    for key in keys:
        if key in value and value[key] is not None:
            return value[key]
    return None


def _optional(value: Mapping[str, object], *keys: str) -> str | None:
    picked = _pick(value, *keys)
    return picked if isinstance(picked, str) and picked else None


def _required(value: Mapping[str, object], *keys: str) -> str:
    picked = _optional(value, *keys)
    if picked is None:
        raise SwigResponseError(f"Yield response is missing {keys[0]}")
    return picked


def _lamports(value: Mapping[str, object], *keys: str) -> str:
    picked = _pick(value, *keys)
    return "0" if picked is None else str(picked)


def _int(value: object) -> int:
    if isinstance(value, bool):
        return 0
    if isinstance(value, int):
        return value
    if isinstance(value, str) and value.isdigit():
        return int(value)
    return 0


def _transaction(value: Mapping[str, object]) -> PreparedTransaction:
    transaction = value.get("transaction")
    if transaction is None:
        raise SwigResponseError("Yield response is missing transaction")
    return normalize_prepared_transaction(transaction)


def _rate(value: object) -> YieldRate | None:
    if not isinstance(value, Mapping):
        return None
    return YieldRate(
        validator_vote_address=_optional(
            value, "validatorVoteAddress", "validator_vote_address"
        )
        or "",
        estimated_net_apr_bps=_int(
            _pick(value, "estimatedNetAprBps", "estimated_net_apr_bps")
        ),
        commission_bps=_int(_pick(value, "commissionBps", "commission_bps")),
        last_updated_at=_optional(value, "lastUpdatedAt", "last_updated_at"),
    )


def _option(value: Mapping[str, object]) -> YieldOption:
    accepts = _pick(value, "acceptsDeposits", "accepts_deposits")
    return YieldOption(
        vote_address=_optional(value, "voteAddress", "vote_address") or "",
        name=_optional(value, "name") or "",
        accepts_deposits=accepts is True,
        rate=_rate(value.get("rate")),
    )


_POSITION_STATES: dict[str, YieldPositionState] = {
    "YIELD_POSITION_STATE_ACTIVATING": "activating",
    "YIELD_POSITION_STATE_ACTIVE": "active",
    "YIELD_POSITION_STATE_DEACTIVATING": "deactivating",
    "YIELD_POSITION_STATE_INACTIVE": "inactive",
    "1": "activating",
    "2": "active",
    "3": "deactivating",
    "4": "inactive",
}


def _position(value: Mapping[str, object]) -> YieldPosition:
    return YieldPosition(
        swig_config_address=_optional(value, "swigConfigAddress", "swig_config_address")
        or "",
        wallet_address=_optional(value, "walletAddress", "wallet_address") or "",
        position_address=_optional(value, "positionAddress", "position_address") or "",
        validator_vote_address=_optional(
            value, "validatorVoteAddress", "validator_vote_address"
        ),
        state=_POSITION_STATES.get(str(value.get("state")), "unspecified"),
        balance_lamports=_lamports(value, "balanceLamports", "balance_lamports"),
        rent_lamports=_lamports(value, "rentLamports", "rent_lamports"),
        position_lamports=_lamports(value, "positionLamports", "position_lamports"),
        withdrawable_lamports=_lamports(
            value, "withdrawableLamports", "withdrawable_lamports"
        ),
        earning_lamports=_lamports(value, "earningLamports", "earning_lamports"),
    )


def normalize_role_action(value: object) -> AddRoleAction:
    """Decode a transcoded ``api_transaction.v1.AddRoleAction`` oneof."""
    action = _mapping(value, "Role action")
    name, body = next(((k, v) for k, v in action.items() if v is not None), ("", {}))
    fields = _mapping(body, "Role action body")

    def amount(camel: str, snake: str) -> str:
        return _lamports(fields, camel, snake)

    def text(camel: str, snake: str) -> str:
        return _optional(fields, camel, snake) or ""

    match name:
        case "all":
            return AllAction()
        case "allButManageAuthority" | "all_but_manage_authority":
            return AllButManageAuthorityAction()
        case "manageAuthority" | "manage_authority":
            return ManageAuthorityAction()
        case "solLimit" | "sol_limit":
            return SolLimitAction(amount=amount("amount", "amount"))
        case "solRecurringLimit" | "sol_recurring_limit":
            return SolRecurringLimitAction(
                recurring_amount=amount("recurringAmount", "recurring_amount"),
                window=amount("window", "window"),
            )
        case "solDestinationLimit" | "sol_destination_limit":
            return SolDestinationLimitAction(
                amount=amount("amount", "amount"),
                destination=text("destination", "destination"),
            )
        case "solRecurringDestinationLimit" | "sol_recurring_destination_limit":
            return SolRecurringDestinationLimitAction(
                recurring_amount=amount("recurringAmount", "recurring_amount"),
                window=amount("window", "window"),
                destination=text("destination", "destination"),
            )
        case "program":
            return ProgramAction(program_id=text("programId", "program_id"))
        case "programAll" | "program_all":
            return ProgramAllAction()
        case "programCurated" | "program_curated":
            return ProgramCuratedAction()
        case "stakeLimit" | "stake_limit":
            return StakeLimitAction(amount=amount("amount", "amount"))
        case "stakeRecurringLimit" | "stake_recurring_limit":
            return StakeRecurringLimitAction(
                recurring_amount=amount("recurringAmount", "recurring_amount"),
                window=amount("window", "window"),
            )
        case "stakeAll" | "stake_all":
            return StakeAllAction()
    raise SwigResponseError(f"Unsupported role action in yield response: {name}")


def normalize_yield_role(response: object) -> YieldRoleResult:
    body = _mapping(response, "Yield role response")
    existing = body.get("existing")
    if isinstance(existing, Mapping):
        role_id = _pick(existing, "roleId", "role_id")
        if not isinstance(role_id, int):
            raise SwigResponseError("Yield role response is missing roleId")
        return ExistingYieldRole(
            role_id=role_id,
            actions=[normalize_role_action(a) for a in _list(existing.get("actions"))],
        )
    create = body.get("create")
    if isinstance(create, Mapping):
        authority = _pick(create, "yieldAuthority", "yield_authority")
        return PreparedYieldRoleCreation(
            transaction=_transaction(create),
            yield_authority=_mapping(authority, "Yield role authority"),
            actions=[normalize_role_action(a) for a in _list(create.get("actions"))],
        )
    raise SwigResponseError("Yield role response has neither existing nor create")
