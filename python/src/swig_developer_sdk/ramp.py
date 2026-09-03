from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import Literal, TypeAlias, cast
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
from .transactions import PreparedTransaction, normalize_prepared_transaction

RampEnvironment: TypeAlias = Literal["sandbox", "production"]

# Only get_options needs this; every other call infers direction from the order.
RampDirection: TypeAlias = Literal["buy", "sell"]

RampOrderStatus: TypeAlias = Literal[
    "unspecified",
    "creating",
    "creation-uncertain",
    "awaiting-customer",
    "awaiting-transfer",
    "processing",
    "settling",
    "settled",
    "declined",
    "cancelled",
    "failed",
    "refunded",
]

RampTransferState: TypeAlias = Literal[
    "unspecified",
    "prepared",
    "submitted",
    "landed",
    "failed",
    "expired",
]


@dataclass(frozen=True, slots=True)
class NativeSolAsset:
    type: Literal["sol"] = "sol"


@dataclass(frozen=True, slots=True)
class SplTokenAsset:
    mint: str
    type: Literal["token"] = "token"


# Native SOL carries no mint, so the two cases cannot be mixed up.
CryptoAsset: TypeAlias = NativeSolAsset | SplTokenAsset


@dataclass(frozen=True, slots=True)
class FiatAmount:
    """Minor units: cents for USD, whole yen for JPY."""

    currency_code: str
    minor_units: str


@dataclass(frozen=True, slots=True)
class CryptoAmount:
    asset: CryptoAsset
    base_units: str


@dataclass(frozen=True, slots=True)
class FiatAmountInput:
    currency_code: str
    minor_units: Amount


@dataclass(frozen=True, slots=True)
class CryptoAmountInput:
    asset: CryptoAsset
    base_units: Amount


@dataclass(frozen=True, slots=True)
class RampRoute:
    provider: str
    payment_method: str


@dataclass(frozen=True, slots=True)
class RampLocation:
    country_code: str
    subdivision_code: str | None = None


@dataclass(frozen=True, slots=True)
class RampBuyOrderRequest:
    spend: FiatAmountInput
    receive: CryptoAsset
    type: Literal["buy"] = "buy"


@dataclass(frozen=True, slots=True)
class RampSellOrderRequest:
    sell: CryptoAmountInput
    receive_fiat_currency_code: str
    type: Literal["sell"] = "sell"


RampOrderRequest: TypeAlias = RampBuyOrderRequest | RampSellOrderRequest


@dataclass(frozen=True, slots=True)
class RampSubdivision:
    code: str
    name: str


@dataclass(frozen=True, slots=True)
class RampCountry:
    code: str
    name: str
    subdivisions: tuple[RampSubdivision, ...]


@dataclass(frozen=True, slots=True)
class RampAssetOption:
    asset: CryptoAsset
    name: str
    decimals: int
    icon_url: str | None = None


@dataclass(frozen=True, slots=True)
class RampFiatCurrencyOption:
    currency_code: str
    exponent: int


@dataclass(frozen=True, slots=True)
class RampOptions:
    countries: tuple[RampCountry, ...]
    fiat_currencies: tuple[RampFiatCurrencyOption, ...]
    payment_methods: tuple[str, ...]
    assets: tuple[RampAssetOption, ...]


@dataclass(frozen=True, slots=True)
class RampBuyQuote:
    spend: FiatAmount
    receive: CryptoAmount
    total_fee: FiatAmount
    #: Display-only exact decimal; a rate has no minor unit to scale to.
    exchange_rate: str
    type: Literal["buy"] = "buy"


@dataclass(frozen=True, slots=True)
class RampSellQuote:
    sell: CryptoAmount
    receive: FiatAmount
    total_fee: FiatAmount
    exchange_rate: str
    type: Literal["sell"] = "sell"


RampQuoteDetails: TypeAlias = RampBuyQuote | RampSellQuote


@dataclass(frozen=True, slots=True)
class RampQuote:
    """Quotes carry no identifier and must never be cached; the chosen route is
    re-priced when the order is created."""

    route: RampRoute
    details: RampQuoteDetails


@dataclass(frozen=True, slots=True)
class RampOrderContext:
    customer_id: str
    swig_config_address: str
    location: RampLocation
    network: Network | None = None


@dataclass(frozen=True, slots=True)
class RampDeposit:
    address: str
    amount: CryptoAmount


@dataclass(frozen=True, slots=True)
class RampTransfer:
    transfer_id: str
    state: RampTransferState
    expires_at: str
    solana_signature: str | None = None


@dataclass(frozen=True, slots=True)
class RampBuyOrder:
    id: str
    status: RampOrderStatus
    created_at: str
    updated_at: str
    quote: RampBuyQuote
    launch_url: str | None = None
    type: Literal["buy"] = "buy"


@dataclass(frozen=True, slots=True)
class RampSellOrder:
    id: str
    status: RampOrderStatus
    created_at: str
    updated_at: str
    quote: RampSellQuote
    launch_url: str | None = None
    #: Absent until the provider assigns one.
    deposit: RampDeposit | None = None
    #: Absent until a transfer is prepared.
    transfer: RampTransfer | None = None
    type: Literal["sell"] = "sell"


RampOrder: TypeAlias = RampBuyOrder | RampSellOrder


@dataclass(frozen=True, slots=True)
class PreparedRampTransfer:
    transfer: RampTransfer
    prepared_transaction: PreparedTransaction
    deposit: RampDeposit


class RampClient:
    def __init__(
        self, http: HttpClient, default_network: Network | None = None
    ) -> None:
        self._http = http
        self._default_network = default_network

    async def get_options(
        self,
        *,
        configuration_id: str,
        environment: RampEnvironment,
        direction: RampDirection,
        country_code: str | None = None,
        fiat_currency_code: str | None = None,
    ) -> RampOptions:
        return _normalize_options(
            await self._http.get(
                _options_path(
                    configuration_id,
                    environment,
                    direction,
                    country_code,
                    fiat_currency_code,
                )
            )
        )

    async def get_quotes(
        self,
        *,
        configuration_id: str,
        environment: RampEnvironment,
        location: RampLocation,
        order: RampOrderRequest,
    ) -> tuple[RampQuote, ...]:
        return _normalize_quotes(
            await self._http.post(
                "/wallet/api/ramp/quotes",
                {
                    "configurationId": configuration_id,
                    "environment": _environment_wire(environment),
                    "location": _location_request(location),
                    **_order_request_body(order),
                },
            )
        )

    async def create_order(
        self,
        *,
        request_id: str,
        configuration_id: str,
        environment: RampEnvironment,
        context: RampOrderContext,
        route: RampRoute,
        order: RampOrderRequest,
    ) -> RampOrder:
        network = require_network(context.network, self._default_network)
        # request_id makes a replay return the stored order, so a retry is safe.
        return _normalize_order_response(
            await self._http.post(
                "/wallet/api/ramp/orders",
                {
                    "requestId": request_id,
                    "configurationId": configuration_id,
                    "environment": _environment_wire(environment),
                    "context": {
                        "customerId": context.customer_id,
                        "swigConfigAddress": context.swig_config_address,
                        "network": to_proto_network(network),
                        "location": _location_request(context.location),
                    },
                    "route": {
                        "provider": route.provider,
                        "paymentMethod": route.payment_method,
                    },
                    **_order_request_body(order),
                },
                retry=True,
            )
        )

    async def get_order(self, *, order_id: str) -> RampOrder:
        return _normalize_order_response(await self._http.get(_order_path(order_id)))

    async def prepare_transfer(
        self,
        *,
        order_id: str,
        requester_authority: WalletAuthority,
        fee_payer: str,
    ) -> PreparedRampTransfer:
        response = _mapping(
            await self._http.post(
                f"{_order_path(order_id)}/transfer/prepare",
                {
                    "requesterAuthority": wallet_authority_to_wire(requester_authority),
                    "feePayer": fee_payer,
                },
            ),
            "Prepare transfer response",
        )
        return _normalize_prepared_transfer(
            _mapping(
                _required(
                    _pick(response, "preparedTransfer", "prepared_transfer"),
                    "preparedTransfer",
                ),
                "preparedTransfer",
            )
        )

    async def submit_transfer(
        self,
        *,
        order_id: str,
        transfer_id: str,
        signed_transaction: str = "",
    ) -> RampTransfer:
        response = _mapping(
            await self._http.post(
                f"{_order_path(order_id)}/transfer/submit",
                {
                    "transferId": transfer_id,
                    "signedTransaction": signed_transaction,
                },
            ),
            "Submit transfer response",
        )
        return _normalize_transfer(
            _mapping(_required(response.get("transfer"), "transfer"), "transfer")
        )


def _options_path(
    configuration_id: str,
    environment: RampEnvironment,
    direction: RampDirection,
    country_code: str | None,
    fiat_currency_code: str | None,
) -> str:
    query = {
        "configurationId": configuration_id,
        "environment": _environment_wire(environment),
        "direction": _direction_wire(direction),
    }
    if country_code:
        query["countryCode"] = country_code
    if fiat_currency_code:
        query["fiatCurrencyCode"] = fiat_currency_code
    return f"/wallet/api/ramp/options?{urlencode(query)}"


def _order_path(order_id: str) -> str:
    return f"/wallet/api/ramp/orders/{quote(order_id, safe='')}"


def _location_request(location: RampLocation) -> dict[str, object]:
    body: dict[str, object] = {"countryCode": location.country_code}
    if location.subdivision_code is not None:
        body["subdivisionCode"] = location.subdivision_code
    return body


def _order_request_body(order: RampOrderRequest) -> dict[str, object]:
    """Direction lives in the oneof, so there is no flag to disagree with it."""
    if isinstance(order, RampBuyOrderRequest):
        return {
            "buy": {
                "spend": {
                    "currencyCode": order.spend.currency_code,
                    "minorUnits": normalize_amount(order.spend.minor_units),
                },
                "receive": _asset_request(order.receive),
            }
        }
    return {
        "sell": {
            "sell": {
                "asset": _asset_request(order.sell.asset),
                "baseUnits": normalize_amount(order.sell.base_units),
            },
            "receiveFiatCurrencyCode": order.receive_fiat_currency_code,
        }
    }


def _asset_request(asset: CryptoAsset) -> dict[str, object]:
    """Native SOL is an empty message, so it is ``{"sol": {}}`` on the wire."""
    if isinstance(asset, NativeSolAsset):
        return {"sol": {}}
    return {"token": {"mint": asset.mint}}


def _environment_wire(environment: RampEnvironment) -> str:
    if environment == "production":
        return "RAMP_ENVIRONMENT_PRODUCTION"
    if environment == "sandbox":
        return "RAMP_ENVIRONMENT_SANDBOX"
    raise ValueError('environment must be "sandbox" or "production"')


def _direction_wire(direction: RampDirection) -> str:
    if direction == "buy":
        return "RAMP_DIRECTION_BUY"
    if direction == "sell":
        return "RAMP_DIRECTION_SELL"
    raise ValueError('direction must be "buy" or "sell"')


def _normalize_options(response: object) -> RampOptions:
    body = _mapping(response, "Ramp options response")
    countries = _sequence(body.get("countries") or [], "countries")
    currencies = _sequence(
        _pick(body, "fiatCurrencies", "fiat_currencies") or [], "fiatCurrencies"
    )
    assets = _sequence(body.get("assets") or [], "assets")
    return RampOptions(
        countries=tuple(_normalize_country(item) for item in countries),
        fiat_currencies=tuple(_normalize_fiat_currency(item) for item in currencies),
        payment_methods=_string_tuple(
            _pick(body, "paymentMethods", "payment_methods") or [], "paymentMethods"
        ),
        assets=tuple(_normalize_asset_option(item) for item in assets),
    )


def _normalize_country(value: object) -> RampCountry:
    body = _mapping(value, "country")
    subdivisions = _sequence(body.get("subdivisions") or [], "subdivisions")
    return RampCountry(
        code=_required_string(body.get("code"), "code"),
        name=_required_string(body.get("name"), "name"),
        subdivisions=tuple(_normalize_subdivision(item) for item in subdivisions),
    )


def _normalize_subdivision(value: object) -> RampSubdivision:
    body = _mapping(value, "subdivision")
    return RampSubdivision(
        code=_required_string(body.get("code"), "code"),
        name=_required_string(body.get("name"), "name"),
    )


def _normalize_fiat_currency(value: object) -> RampFiatCurrencyOption:
    body = _mapping(value, "fiat currency")
    return RampFiatCurrencyOption(
        currency_code=_required_string(
            _pick(body, "currencyCode", "currency_code"), "currencyCode"
        ),
        exponent=_number_field(body.get("exponent"), "exponent"),
    )


def _normalize_asset_option(value: object) -> RampAssetOption:
    body = _mapping(value, "asset option")
    return RampAssetOption(
        asset=_normalize_asset(body.get("asset")),
        name=_required_string(body.get("name"), "name"),
        decimals=_number_field(body.get("decimals"), "decimals"),
        icon_url=_optional_string(_pick(body, "iconUrl", "icon_url")),
    )


def _normalize_quotes(response: object) -> tuple[RampQuote, ...]:
    body = _mapping(response, "Ramp quotes response")
    quotes = _sequence(body.get("quotes") or [], "quotes")
    return tuple(_normalize_quote(item) for item in quotes)


def _normalize_quote(value: object) -> RampQuote:
    body = _mapping(value, "quote")
    route = _normalize_route(body.get("route"))
    if "buy" in body:
        return RampQuote(route=route, details=_normalize_buy_quote(body["buy"]))
    if "sell" in body:
        return RampQuote(route=route, details=_normalize_sell_quote(body["sell"]))
    raise ValueError("Ramp response is missing quote")


def _normalize_buy_quote(value: object) -> RampBuyQuote:
    body = _mapping(value, "buy quote")
    return RampBuyQuote(
        spend=_normalize_fiat_amount(body.get("spend"), "spend"),
        receive=_normalize_crypto_amount(body.get("receive"), "receive"),
        total_fee=_normalize_fiat_amount(
            _pick(body, "totalFee", "total_fee"), "totalFee"
        ),
        exchange_rate=_required_string(
            _pick(body, "exchangeRate", "exchange_rate"), "exchangeRate"
        ),
    )


def _normalize_sell_quote(value: object) -> RampSellQuote:
    body = _mapping(value, "sell quote")
    return RampSellQuote(
        sell=_normalize_crypto_amount(body.get("sell"), "sell"),
        receive=_normalize_fiat_amount(body.get("receive"), "receive"),
        total_fee=_normalize_fiat_amount(
            _pick(body, "totalFee", "total_fee"), "totalFee"
        ),
        exchange_rate=_required_string(
            _pick(body, "exchangeRate", "exchange_rate"), "exchangeRate"
        ),
    )


def _normalize_route(value: object) -> RampRoute:
    body = _mapping(_required(value, "route"), "route")
    return RampRoute(
        provider=_required_string(body.get("provider"), "provider"),
        payment_method=_required_string(
            _pick(body, "paymentMethod", "payment_method"), "paymentMethod"
        ),
    )


def _normalize_order_response(response: object) -> RampOrder:
    body = _mapping(response, "Ramp order response")
    return _normalize_order(_mapping(_required(body.get("order"), "order"), "order"))


def _normalize_order(body: Mapping[str, object]) -> RampOrder:
    order_id = _required_string(body.get("id"), "id")
    status = _normalize_order_status(body.get("status"))
    created_at = _required_string(_pick(body, "createdAt", "created_at"), "createdAt")
    updated_at = _required_string(_pick(body, "updatedAt", "updated_at"), "updatedAt")

    if "buy" in body:
        details = _mapping(body["buy"], "buy")
        return RampBuyOrder(
            id=order_id,
            status=status,
            created_at=created_at,
            updated_at=updated_at,
            quote=_normalize_buy_quote(_required(details.get("quote"), "quote")),
            launch_url=_optional_string(_pick(details, "launchUrl", "launch_url")),
        )

    if "sell" in body:
        details = _mapping(body["sell"], "sell")
        deposit = details.get("deposit")
        transfer = details.get("transfer")
        return RampSellOrder(
            id=order_id,
            status=status,
            created_at=created_at,
            updated_at=updated_at,
            quote=_normalize_sell_quote(_required(details.get("quote"), "quote")),
            launch_url=_optional_string(_pick(details, "launchUrl", "launch_url")),
            deposit=(
                _normalize_deposit(_mapping(deposit, "deposit"))
                if deposit is not None
                else None
            ),
            transfer=(
                _normalize_transfer(_mapping(transfer, "transfer"))
                if transfer is not None
                else None
            ),
        )

    raise ValueError("Ramp response is missing order details")


def _normalize_prepared_transfer(body: Mapping[str, object]) -> PreparedRampTransfer:
    return PreparedRampTransfer(
        transfer=_normalize_transfer(
            _mapping(_required(body.get("transfer"), "transfer"), "transfer")
        ),
        prepared_transaction=normalize_prepared_transaction(
            _required(
                _pick(body, "preparedTransaction", "prepared_transaction"),
                "preparedTransaction",
            )
        ),
        deposit=_normalize_deposit(
            _mapping(_required(body.get("deposit"), "deposit"), "deposit")
        ),
    )


def _normalize_transfer(body: Mapping[str, object]) -> RampTransfer:
    return RampTransfer(
        transfer_id=_required_string(
            _pick(body, "transferId", "transfer_id"), "transferId"
        ),
        state=_normalize_transfer_state(body.get("state")),
        expires_at=_required_string(
            _pick(body, "expiresAt", "expires_at"), "expiresAt"
        ),
        solana_signature=_optional_string(
            _pick(body, "solanaSignature", "solana_signature")
        ),
    )


def _normalize_deposit(body: Mapping[str, object]) -> RampDeposit:
    return RampDeposit(
        address=_required_string(body.get("address"), "address"),
        amount=_normalize_crypto_amount(body.get("amount"), "amount"),
    )


def _normalize_fiat_amount(value: object, field: str) -> FiatAmount:
    body = _mapping(_required(value, field), field)
    return FiatAmount(
        currency_code=_required_string(
            _pick(body, "currencyCode", "currency_code"), "currencyCode"
        ),
        minor_units=_units_field(
            _pick(body, "minorUnits", "minor_units"), "minorUnits"
        ),
    )


def _normalize_crypto_amount(value: object, field: str) -> CryptoAmount:
    body = _mapping(_required(value, field), field)
    return CryptoAmount(
        asset=_normalize_asset(body.get("asset")),
        base_units=_units_field(_pick(body, "baseUnits", "base_units"), "baseUnits"),
    )


def _normalize_asset(value: object) -> CryptoAsset:
    """Key presence, not truthiness: native SOL arrives as an empty object."""
    body = _mapping(_required(value, "asset"), "asset")
    if "sol" in body:
        return NativeSolAsset()
    if "token" in body:
        token = _mapping(body["token"], "token")
        return SplTokenAsset(mint=_required_string(token.get("mint"), "mint"))
    raise ValueError("Ramp response is missing asset")


_ORDER_STATUSES: Mapping[str, RampOrderStatus] = {
    "RAMP_ORDER_STATUS_UNSPECIFIED": "unspecified",
    "RAMP_ORDER_STATUS_CREATING": "creating",
    "RAMP_ORDER_STATUS_CREATION_UNCERTAIN": "creation-uncertain",
    "RAMP_ORDER_STATUS_AWAITING_CUSTOMER": "awaiting-customer",
    "RAMP_ORDER_STATUS_AWAITING_TRANSFER": "awaiting-transfer",
    "RAMP_ORDER_STATUS_PROCESSING": "processing",
    "RAMP_ORDER_STATUS_SETTLING": "settling",
    "RAMP_ORDER_STATUS_SETTLED": "settled",
    "RAMP_ORDER_STATUS_DECLINED": "declined",
    "RAMP_ORDER_STATUS_CANCELLED": "cancelled",
    "RAMP_ORDER_STATUS_FAILED": "failed",
    "RAMP_ORDER_STATUS_REFUNDED": "refunded",
}

_TRANSFER_STATES: Mapping[str, RampTransferState] = {
    "TRANSFER_STATE_UNSPECIFIED": "unspecified",
    "TRANSFER_STATE_PREPARED": "prepared",
    "TRANSFER_STATE_SUBMITTED": "submitted",
    "TRANSFER_STATE_LANDED": "landed",
    "TRANSFER_STATE_FAILED": "failed",
    "TRANSFER_STATE_EXPIRED": "expired",
}


def _normalize_order_status(value: object) -> RampOrderStatus:
    status = _ORDER_STATUSES.get(value) if isinstance(value, str) else None
    if status is None:
        raise ValueError("Ramp response has invalid status")
    return status


def _normalize_transfer_state(value: object) -> RampTransferState:
    state = _TRANSFER_STATES.get(value) if isinstance(value, str) else None
    if state is None:
        raise ValueError("Ramp response has invalid state")
    return state


def _mapping(value: object, label: str) -> Mapping[str, object]:
    if isinstance(value, Mapping):
        return value
    raise ValueError(f"{label} must be an object")


def _sequence(value: object, field: str) -> Sequence[object]:
    if isinstance(value, Sequence) and not isinstance(value, (str, bytes)):
        return value
    raise ValueError(f"Ramp response has invalid {field}")


def _string_tuple(value: object, field: str) -> tuple[str, ...]:
    sequence = _sequence(value, field)
    if not all(isinstance(item, str) for item in sequence):
        raise ValueError(f"Ramp response has invalid {field}")
    return tuple(cast(str, item) for item in sequence)


def _pick(value: Mapping[str, object], *keys: str) -> object:
    for key in keys:
        if key in value:
            return value[key]
    return None


def _required(value: object, field: str) -> object:
    if value is None:
        raise ValueError(f"Ramp response is missing {field}")
    return value


def _required_string(value: object, field: str) -> str:
    if isinstance(value, str) and value:
        return value
    raise ValueError(f"Ramp response is missing {field}")


def _optional_string(value: object) -> str | None:
    return value if isinstance(value, str) and value else None


def _units_field(value: object, field: str) -> str:
    """uint64 crosses the wire as a decimal string; keeping it one avoids 2^53."""
    if not isinstance(value, str) or not (value.isascii() and value.isdigit()):
        raise ValueError(f"Ramp response has invalid {field}")
    return value


def _number_field(value: object, field: str) -> int:
    if isinstance(value, bool) or not isinstance(value, (int, str)):
        raise ValueError(f"Ramp response has invalid {field}")
    try:
        return int(value)
    except ValueError as error:
        raise ValueError(f"Ramp response has invalid {field}") from error
