from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass
from typing import Literal, TypeAlias, TypedDict

from .errors import SwigResponseError

Network: TypeAlias = Literal["mainnet", "devnet"]
Amount: TypeAlias = int | str
JsonScalar: TypeAlias = str | int | float | bool | None
JsonValue: TypeAlias = JsonScalar | list["JsonValue"] | dict[str, "JsonValue"]
JsonObject: TypeAlias = dict[str, JsonValue]


class PublicKeyAuthority(TypedDict):
    """A public key encoded as required by the selected signature scheme."""

    public_key: str


class _LegacyPublicKeyAuthority(TypedDict):
    publicKey: str


class Ed25519Authority(TypedDict):
    ed25519: PublicKeyAuthority | _LegacyPublicKeyAuthority


class Secp256k1Authority(TypedDict):
    secp256k1: PublicKeyAuthority | _LegacyPublicKeyAuthority


class Secp256r1Authority(TypedDict):
    secp256r1: PublicKeyAuthority | _LegacyPublicKeyAuthority


class ProgramExecProof(TypedDict):
    role_id: int
    zk_proof: str


class _LegacyProgramExecProof(TypedDict):
    roleId: int
    zkProof: str


class ProgramExecAuthority(TypedDict):
    program_exec_proof: ProgramExecProof | _LegacyProgramExecProof


class _LegacyProgramExecAuthority(TypedDict):
    programExecProof: ProgramExecProof | _LegacyProgramExecProof


class _ParticipantSetRole(TypedDict, total=False):
    role_id: int
    roleId: int


class ParticipantSetAuthorityValue(_ParticipantSetRole):
    address: str


class _ParticipantSetSnakeAddress(_ParticipantSetRole):
    participant_set_address: str


class _ParticipantSetCamelAddress(_ParticipantSetRole):
    participantSetAddress: str


_ParticipantSetValue: TypeAlias = (
    ParticipantSetAuthorityValue
    | _ParticipantSetSnakeAddress
    | _ParticipantSetCamelAddress
)


class ParticipantSetAuthority(TypedDict):
    participant_set: _ParticipantSetValue


class _LegacyParticipantSetAuthority(TypedDict):
    participantSet: _ParticipantSetValue


WalletAuthority: TypeAlias = (
    Ed25519Authority
    | Secp256k1Authority
    | Secp256r1Authority
    | ProgramExecAuthority
    | ParticipantSetAuthority
    | _LegacyProgramExecAuthority
    | _LegacyParticipantSetAuthority
)

DEFAULT_BACKEND_URL = "https://api.onswig.com"


@dataclass(frozen=True, slots=True)
class RetryOptions:
    max_retries: int = 3
    retry_delay: float = 1.0
    backoff_multiplier: float = 2.0

    def __post_init__(self) -> None:
        if self.max_retries < 0:
            raise ValueError("max_retries must be non-negative")
        if self.retry_delay < 0:
            raise ValueError("retry_delay must be non-negative")
        if self.backoff_multiplier < 0:
            raise ValueError("backoff_multiplier must be non-negative")


@dataclass(frozen=True, slots=True)
class WalletAddressInfo:
    swig_config_address: str
    wallet_address: str
    network: Network | None = None


def wallet_address_info_from_wire(value: object) -> WalletAddressInfo:
    if not isinstance(value, Mapping):
        raise SwigResponseError("Response is missing wallet")
    return WalletAddressInfo(
        swig_config_address=_required_string(
            value.get("swigConfigAddress", value.get("swig_config_address")),
            "swigConfigAddress",
        ),
        wallet_address=_required_string(
            value.get("walletAddress", value.get("wallet_address")),
            "walletAddress",
        ),
        network=normalize_network(value.get("network")),
    )


def wallet_authority_to_wire(authority: Mapping[str, object]) -> dict[str, object]:
    """Validate one authority variant and convert Python or legacy field names."""
    schemes = {
        "ed25519",
        "secp256k1",
        "secp256r1",
        "programExecProof",
        "program_exec_proof",
        "participantSet",
        "participant_set",
    }
    if len(authority) != 1 or not set(authority).issubset(schemes):
        raise ValueError("authority must include exactly one supported authority")
    scheme, value = next(iter(authority.items()))
    if not isinstance(value, Mapping):
        raise ValueError("authority value must be an object")
    if scheme in ("ed25519", "secp256k1", "secp256r1"):
        public_key = _authority_field(value, "public_key", "publicKey")
        if not isinstance(public_key, str) or not public_key.strip():
            raise ValueError(f"{scheme} authority requires publicKey")
        return {scheme: {"publicKey": public_key}}
    if scheme in ("programExecProof", "program_exec_proof"):
        role_id = _authority_field(value, "role_id", "roleId")
        zk_proof = _authority_field(value, "zk_proof", "zkProof")
        if (
            isinstance(role_id, bool)
            or not isinstance(role_id, int)
            or not isinstance(zk_proof, str)
        ):
            raise ValueError("programExecProof requires roleId and zkProof")
        return {"programExecProof": {"roleId": role_id, "zkProof": zk_proof}}
    address = _authority_field(
        value, "address", "participant_set_address", "participantSetAddress"
    )
    role_id = _authority_field(value, "role_id", "roleId")
    if not isinstance(address, str) or not address.strip():
        raise ValueError("participantSet authority requires address")
    if role_id is not None and (
        isinstance(role_id, bool) or not isinstance(role_id, int)
    ):
        raise ValueError("participantSet authority roleId must be an integer")
    participant: dict[str, object] = {"participantSetAddress": address}
    if role_id is not None:
        participant["roleId"] = role_id
    return {"participantSet": participant}


def _authority_field(value: Mapping[str, object], *names: str) -> object:
    """Accept legacy aliases without silently choosing between conflicting values."""
    present = [value[name] for name in names if name in value]
    if present and any(item != present[0] for item in present[1:]):
        raise ValueError(f"Conflicting authority aliases for {names[0]}")
    return present[0] if present else None


def normalize_amount(amount: Amount) -> str:
    return str(amount)


def to_proto_network(network: Network) -> str:
    if network == "devnet":
        return "NETWORK_DEVNET"
    return "NETWORK_MAINNET"


def normalize_network(value: object) -> Network | None:
    if value in ("devnet", "NETWORK_DEVNET", 1):
        return "devnet"
    if value in ("mainnet", "NETWORK_MAINNET", 2):
        return "mainnet"
    return None


def require_network(*values: Network | None) -> Network:
    for value in values:
        if value is not None:
            return value
    raise ValueError("network is required")


def _required_string(value: object, field: str) -> str:
    if isinstance(value, str):
        return value
    raise SwigResponseError(f"Response is missing {field}")
