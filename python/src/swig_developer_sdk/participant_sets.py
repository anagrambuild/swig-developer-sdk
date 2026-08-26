from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TypeAlias

from .common import (
    Network,
    WalletAuthority,
    require_network,
    to_proto_network,
    wallet_authority_to_wire,
)
from .core import HttpClient
from .transactions import PreparedTransaction, normalize_prepared_transaction

ParticipantSetMember: TypeAlias = WalletAuthority


@dataclass(frozen=True, slots=True)
class CreateParticipantSetResult:
    participant_set_address: str
    set_id: str
    transaction: PreparedTransaction


class ParticipantSetsClient:
    def __init__(
        self,
        http: HttpClient,
        default_network: Network | None = None,
    ) -> None:
        self._http = http
        self._default_network = default_network

    async def create(
        self,
        *,
        swig_config_address: str,
        fee_payer: str,
        threshold: int,
        members: Sequence[ParticipantSetMember],
        set_id: str | None = None,
        network: Network | None = None,
    ) -> CreateParticipantSetResult:
        body = _mapping(
            await self._http.post(
                "/transaction/wallet/participant-set/create",
                {
                    "network": to_proto_network(
                        require_network(network, self._default_network)
                    ),
                    "feePayer": fee_payer,
                    "swigAddress": swig_config_address,
                    "setId": set_id,
                    "threshold": threshold,
                    "members": [_member_to_wire(member) for member in members],
                },
            ),
            "Create ParticipantSet response",
        )
        transaction = body.get("transaction")
        if transaction is None:
            raise ValueError("Create ParticipantSet response is missing transaction")
        return CreateParticipantSetResult(
            participant_set_address=_required_string(
                _pick(body, "participantSetAddress", "participant_set_address"),
                "participantSetAddress",
            ),
            set_id=_required_string(_pick(body, "setId", "set_id"), "setId"),
            transaction=normalize_prepared_transaction(transaction),
        )


def _member_to_wire(member: ParticipantSetMember) -> dict[str, object]:
    authority = wallet_authority_to_wire(member)
    if not any(scheme in authority for scheme in ("ed25519", "secp256k1", "secp256r1")):
        raise ValueError(
            "ParticipantSet members must use ed25519, secp256k1, or secp256r1"
        )
    return authority


def _mapping(value: object, label: str) -> Mapping[str, object]:
    if isinstance(value, Mapping):
        return value
    raise ValueError(f"{label} must be an object")


def _pick(value: Mapping[str, object], *keys: str) -> object:
    for key in keys:
        if key in value:
            return value[key]
    return None


def _required_string(value: object, field: str) -> str:
    if isinstance(value, str) and value:
        return value
    raise ValueError(f"Create ParticipantSet response is missing {field}")
