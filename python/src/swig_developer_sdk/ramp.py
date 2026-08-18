from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import Literal, TypeAlias, cast
from urllib.parse import quote, urlencode

from .common import Network, WalletAuthority, to_proto_network, wallet_authority_to_wire
from .core import HttpClient
from .transactions import PreparedTransaction, normalize_prepared_transaction

MeldEnvironment: TypeAlias = Literal["sandbox", "production"]
OnrampSessionStatus: TypeAlias = Literal[
    "unspecified",
    "created",
    "pending",
    "settling",
    "settled",
    "failed",
    "declined",
    "cancelled",
    "refunded",
]
OfframpSessionStatus: TypeAlias = Literal[
    "unspecified",
    "created",
    "provider-session-created",
    "transfer-required",
    "transfer-submitted",
    "pending",
    "settling",
    "settled",
    "declined",
    "cancelled",
    "failed",
    "refunded",
]


@dataclass(frozen=True, slots=True)
class RampSubdivisionOption:
    subdivision_code: str
    subdivision_name: str


@dataclass(frozen=True, slots=True)
class RampCountryOption:
    country_code: str
    country_name: str
    subdivisions: tuple[RampSubdivisionOption, ...]


@dataclass(frozen=True, slots=True)
class RampCryptoCurrencyOption:
    currency_code: str
    currency_name: str
    icon_url: str
    contract_address: str


@dataclass(frozen=True, slots=True)
class OnrampOptions:
    countries: tuple[RampCountryOption, ...]
    fiat_currency_codes: tuple[str, ...]
    payment_method_types: tuple[str, ...]
    crypto_currency_codes: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class OfframpOptions:
    countries: tuple[RampCountryOption, ...]
    fiat_currency_codes: tuple[str, ...]
    payment_method_types: tuple[str, ...]
    crypto_currencies: tuple[RampCryptoCurrencyOption, ...]


@dataclass(frozen=True, slots=True)
class _CommonRampOptions:
    countries: tuple[RampCountryOption, ...]
    fiat_currency_codes: tuple[str, ...]
    payment_method_types: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class QuoteRampArgs:
    organization_meld_configuration_id: str
    environment: MeldEnvironment
    external_customer_id: str
    swig_config_address: str
    source_amount: str
    source_currency_code: str
    destination_currency_code: str
    country_code: str
    network: Network | None = None
    subdivision: str | None = None
    payment_method_type: str | None = None


@dataclass(frozen=True, slots=True)
class RampQuote:
    quote_id: str
    service_provider: str
    payment_method_type: str
    source_amount: str
    source_currency_code: str
    destination_amount: str
    destination_currency_code: str
    exchange_rate: str
    total_fee: str


@dataclass(frozen=True, slots=True)
class QuoteRampResult:
    quotes: tuple[RampQuote, ...]


@dataclass(frozen=True, slots=True)
class CreateRampSessionResult:
    session_id: str
    launch_url: str


@dataclass(frozen=True, slots=True)
class OnrampSession:
    session_id: str
    status: OnrampSessionStatus
    created_at: str
    updated_at: str


@dataclass(frozen=True, slots=True)
class OfframpSession:
    session_id: str
    status: OfframpSessionStatus
    source_amount: str
    source_currency_code: str
    destination_amount: str
    destination_currency_code: str
    service_provider: str
    created_at: str
    updated_at: str
    payment_method_type: str | None = None
    solana_signature: str | None = None
    provider_destination_amount: str | None = None


@dataclass(frozen=True, slots=True)
class OfframpTransferDisplay:
    source_wallet_address: str
    destination_wallet_address: str
    source_amount: str
    source_currency_code: str
    destination_amount: str
    destination_currency_code: str
    service_provider: str
    payment_method_type: str | None = None
    provider_destination_amount: str | None = None


@dataclass(frozen=True, slots=True)
class PrepareOfframpAuthorizationResult:
    authorization_id: str
    prepared_transaction: PreparedTransaction
    display: OfframpTransferDisplay


@dataclass(frozen=True, slots=True)
class SubmitOfframpAuthorizationResult:
    solana_signature: str


class RampClient:
    def __init__(
        self, http: HttpClient, default_network: Network | None = None
    ) -> None:
        self.onramp = OnrampClient(http, default_network)
        self.offramp = OfframpClient(http, default_network)


class OnrampClient:
    def __init__(
        self, http: HttpClient, default_network: Network | None = None
    ) -> None:
        self._http = http
        self._default_network = default_network

    async def get_options(
        self,
        *,
        organization_meld_configuration_id: str,
        environment: MeldEnvironment,
        country_code: str | None = None,
        fiat_currency_code: str | None = None,
    ) -> OnrampOptions:
        response = await self._http.get(
            _options_path(
                "onramp",
                organization_meld_configuration_id,
                environment,
                country_code,
                fiat_currency_code,
            )
        )
        common = _normalize_common_options(response)
        body = _mapping(response, "Ramp options response")
        return OnrampOptions(
            countries=common.countries,
            fiat_currency_codes=common.fiat_currency_codes,
            payment_method_types=common.payment_method_types,
            crypto_currency_codes=_string_tuple(
                _pick(body, "cryptoCurrencyCodes", "crypto_currency_codes") or []
            ),
        )

    async def quote(self, args: QuoteRampArgs) -> QuoteRampResult:
        return _normalize_quote_result(
            await self._http.post(
                "/wallet/api/ramp/onramp/quote",
                _quote_request(args, self._default_network),
            )
        )

    async def create_session(
        self,
        *,
        organization_meld_configuration_id: str,
        quote_id: str,
        environment: MeldEnvironment,
    ) -> CreateRampSessionResult:
        return _normalize_create_session(
            await self._http.post(
                "/wallet/api/ramp/onramp/session",
                _session_request(
                    organization_meld_configuration_id, quote_id, environment
                ),
            )
        )

    async def get_session(
        self, *, session_id: str, environment: MeldEnvironment
    ) -> OnrampSession:
        return _normalize_onramp_session(
            await self._http.get(_session_path("onramp", session_id, environment))
        )


class OfframpClient:
    def __init__(
        self, http: HttpClient, default_network: Network | None = None
    ) -> None:
        self._http = http
        self._default_network = default_network

    async def get_options(
        self,
        *,
        organization_meld_configuration_id: str,
        environment: MeldEnvironment,
        country_code: str | None = None,
        fiat_currency_code: str | None = None,
    ) -> OfframpOptions:
        response = await self._http.get(
            _options_path(
                "offramp",
                organization_meld_configuration_id,
                environment,
                country_code,
                fiat_currency_code,
            )
        )
        common = _normalize_common_options(response)
        body = _mapping(response, "Ramp options response")
        currencies = _sequence(
            _pick(body, "cryptoCurrencies", "crypto_currencies") or [],
            "cryptoCurrencies",
        )
        return OfframpOptions(
            countries=common.countries,
            fiat_currency_codes=common.fiat_currency_codes,
            payment_method_types=common.payment_method_types,
            crypto_currencies=tuple(
                _normalize_crypto_currency(item) for item in currencies
            ),
        )

    async def quote(self, args: QuoteRampArgs) -> QuoteRampResult:
        return _normalize_quote_result(
            await self._http.post(
                "/wallet/api/ramp/offramp/quote",
                _quote_request(args, self._default_network),
            )
        )

    async def create_session(
        self,
        *,
        organization_meld_configuration_id: str,
        quote_id: str,
        environment: MeldEnvironment,
    ) -> CreateRampSessionResult:
        return _normalize_create_session(
            await self._http.post(
                "/wallet/api/ramp/offramp/session",
                _session_request(
                    organization_meld_configuration_id, quote_id, environment
                ),
            )
        )

    async def prepare_authorization(
        self,
        *,
        session_id: str,
        requester_authority: WalletAuthority,
        environment: MeldEnvironment,
        fee_payer: str,
    ) -> PrepareOfframpAuthorizationResult:
        body = _mapping(
            await self._http.post(
                _offramp_action_path(session_id, "prepare"),
                {
                    "requesterAuthority": wallet_authority_to_wire(requester_authority),
                    "environment": _environment_wire(environment),
                    "feePayer": fee_payer,
                },
            ),
            "Prepare offramp response",
        )
        prepared = _pick(body, "preparedTransaction", "prepared_transaction")
        display = _pick(body, "display")
        return PrepareOfframpAuthorizationResult(
            authorization_id=_required_string(
                _pick(body, "authorizationId", "authorization_id"),
                "authorizationId",
            ),
            prepared_transaction=normalize_prepared_transaction(prepared),
            display=_normalize_transfer_display(display),
        )

    async def submit_authorization(
        self,
        *,
        session_id: str,
        authorization_id: str,
        signed_transaction: str,
        environment: MeldEnvironment,
    ) -> SubmitOfframpAuthorizationResult:
        body = _mapping(
            await self._http.post(
                _offramp_action_path(session_id, "submit"),
                {
                    "authorizationId": authorization_id,
                    "signedTransaction": signed_transaction,
                    "environment": _environment_wire(environment),
                },
            ),
            "Submit offramp response",
        )
        return SubmitOfframpAuthorizationResult(
            solana_signature=_required_string(
                _pick(body, "solanaSignature", "solana_signature"),
                "solanaSignature",
            )
        )

    async def get_session(
        self, *, session_id: str, environment: MeldEnvironment
    ) -> OfframpSession:
        return _normalize_offramp_session(
            await self._http.get(_session_path("offramp", session_id, environment))
        )


def _options_path(
    direction: Literal["onramp", "offramp"],
    configuration_id: str,
    environment: MeldEnvironment,
    country_code: str | None,
    fiat_currency_code: str | None,
) -> str:
    query = {
        "organizationMeldConfigurationId": configuration_id,
        "environment": _environment_wire(environment),
    }
    if country_code is not None:
        query["countryCode"] = country_code
    if fiat_currency_code is not None:
        query["fiatCurrencyCode"] = fiat_currency_code
    return f"/wallet/api/ramp/{direction}/options?{urlencode(query)}"


def _session_path(
    direction: Literal["onramp", "offramp"],
    session_id: str,
    environment: MeldEnvironment,
) -> str:
    query = urlencode({"environment": _environment_wire(environment)})
    return f"/wallet/api/ramp/{direction}/session/{quote(session_id, safe='')}?{query}"


def _offramp_action_path(session_id: str, action: Literal["prepare", "submit"]) -> str:
    encoded_session_id = quote(session_id, safe="")
    return f"/wallet/api/ramp/offramp/session/{encoded_session_id}/{action}"


def _quote_request(
    args: QuoteRampArgs, default_network: Network | None
) -> dict[str, object]:
    network = args.network or default_network
    if network is None:
        raise ValueError("network is required")
    return {
        "organizationMeldConfigurationId": args.organization_meld_configuration_id,
        "externalCustomerId": args.external_customer_id,
        "swigConfigAddress": args.swig_config_address,
        "network": to_proto_network(network),
        "sourceAmount": args.source_amount,
        "sourceCurrencyCode": args.source_currency_code,
        "destinationCurrencyCode": args.destination_currency_code,
        "countryCode": args.country_code,
        "subdivision": args.subdivision,
        "paymentMethodType": args.payment_method_type,
        "environment": _environment_wire(args.environment),
    }


def _session_request(
    configuration_id: str, quote_id: str, environment: MeldEnvironment
) -> dict[str, object]:
    return {
        "organizationMeldConfigurationId": configuration_id,
        "quoteId": quote_id,
        "environment": _environment_wire(environment),
    }


def _environment_wire(environment: MeldEnvironment) -> str:
    if environment == "production":
        return "MELD_ENVIRONMENT_PRODUCTION"
    if environment == "sandbox":
        return "MELD_ENVIRONMENT_SANDBOX"
    raise ValueError('environment must be "sandbox" or "production"')


def _normalize_common_options(value: object) -> _CommonRampOptions:
    body = _mapping(value, "Ramp options response")
    return _CommonRampOptions(
        countries=tuple(
            _normalize_country(item)
            for item in _sequence(body.get("countries", []), "countries")
        ),
        fiat_currency_codes=_string_tuple(
            _pick(body, "fiatCurrencyCodes", "fiat_currency_codes") or []
        ),
        payment_method_types=_string_tuple(
            _pick(body, "paymentMethodTypes", "payment_method_types") or []
        ),
    )


def _normalize_country(value: object) -> RampCountryOption:
    body = _mapping(value, "Ramp country")
    return RampCountryOption(
        country_code=_required_string(
            _pick(body, "countryCode", "country_code"), "countryCode"
        ),
        country_name=_required_string(
            _pick(body, "countryName", "country_name"), "countryName"
        ),
        subdivisions=tuple(
            RampSubdivisionOption(
                subdivision_code=_required_string(
                    _pick(
                        _mapping(item, "Ramp subdivision"),
                        "subdivisionCode",
                        "subdivision_code",
                    ),
                    "subdivisionCode",
                ),
                subdivision_name=_required_string(
                    _pick(
                        _mapping(item, "Ramp subdivision"),
                        "subdivisionName",
                        "subdivision_name",
                    ),
                    "subdivisionName",
                ),
            )
            for item in _sequence(body.get("subdivisions", []), "subdivisions")
        ),
    )


def _normalize_crypto_currency(value: object) -> RampCryptoCurrencyOption:
    body = _mapping(value, "Ramp crypto currency")
    return RampCryptoCurrencyOption(
        currency_code=_required_string(
            _pick(body, "currencyCode", "currency_code"), "currencyCode"
        ),
        currency_name=_required_string(
            _pick(body, "currencyName", "currency_name"), "currencyName"
        ),
        icon_url=_required_string(_pick(body, "iconUrl", "icon_url"), "iconUrl"),
        contract_address=_required_string(
            _pick(body, "contractAddress", "contract_address"), "contractAddress"
        ),
    )


def _normalize_quote_result(value: object) -> QuoteRampResult:
    body = _mapping(value, "Ramp quote response")
    return QuoteRampResult(
        quotes=tuple(
            _normalize_quote(item)
            for item in _sequence(body.get("quotes", []), "quotes")
        )
    )


def _normalize_quote(value: object) -> RampQuote:
    body = _mapping(value, "Ramp quote")
    return RampQuote(
        quote_id=_wire_string(body, "quoteId"),
        service_provider=_wire_string(body, "serviceProvider"),
        payment_method_type=_wire_string(body, "paymentMethodType"),
        source_amount=_wire_string(body, "sourceAmount"),
        source_currency_code=_wire_string(body, "sourceCurrencyCode"),
        destination_amount=_wire_string(body, "destinationAmount"),
        destination_currency_code=_wire_string(body, "destinationCurrencyCode"),
        exchange_rate=_wire_string(body, "exchangeRate"),
        total_fee=_wire_string(body, "totalFee"),
    )


def _normalize_create_session(value: object) -> CreateRampSessionResult:
    body = _mapping(value, "Ramp session response")
    return CreateRampSessionResult(
        session_id=_wire_string(body, "sessionId"),
        launch_url=_wire_string(body, "launchUrl"),
    )


def _normalize_onramp_session(value: object) -> OnrampSession:
    body = _mapping(value, "Onramp session response")
    return OnrampSession(
        session_id=_wire_string(body, "sessionId"),
        status=cast(
            OnrampSessionStatus, _normalize_status(body.get("status"), "ONRAMP")
        ),
        created_at=_wire_string(body, "createdAt"),
        updated_at=_wire_string(body, "updatedAt"),
    )


def _normalize_offramp_session(value: object) -> OfframpSession:
    body = _mapping(value, "Offramp session response")
    return OfframpSession(
        session_id=_wire_string(body, "sessionId"),
        status=cast(
            OfframpSessionStatus, _normalize_status(body.get("status"), "OFFRAMP")
        ),
        source_amount=_wire_string(body, "sourceAmount"),
        source_currency_code=_wire_string(body, "sourceCurrencyCode"),
        destination_amount=_wire_string(body, "destinationAmount"),
        destination_currency_code=_wire_string(body, "destinationCurrencyCode"),
        service_provider=_wire_string(body, "serviceProvider"),
        payment_method_type=_optional_wire_string(body, "paymentMethodType"),
        solana_signature=_optional_wire_string(body, "solanaSignature"),
        created_at=_wire_string(body, "createdAt"),
        updated_at=_wire_string(body, "updatedAt"),
        provider_destination_amount=_optional_wire_string(
            body, "providerDestinationAmount"
        ),
    )


def _normalize_transfer_display(value: object) -> OfframpTransferDisplay:
    body = _mapping(value, "Offramp transfer display")
    return OfframpTransferDisplay(
        source_wallet_address=_wire_string(body, "sourceWalletAddress"),
        destination_wallet_address=_wire_string(body, "destinationWalletAddress"),
        source_amount=_wire_string(body, "sourceAmount"),
        source_currency_code=_wire_string(body, "sourceCurrencyCode"),
        destination_amount=_wire_string(body, "destinationAmount"),
        destination_currency_code=_wire_string(body, "destinationCurrencyCode"),
        service_provider=_wire_string(body, "serviceProvider"),
        payment_method_type=_optional_wire_string(body, "paymentMethodType"),
        provider_destination_amount=_optional_wire_string(
            body, "providerDestinationAmount"
        ),
    )


def _normalize_status(value: object, direction: Literal["ONRAMP", "OFFRAMP"]) -> str:
    if isinstance(value, int):
        statuses = (
            [
                "unspecified",
                "created",
                "pending",
                "settling",
                "settled",
                "failed",
                "declined",
                "cancelled",
                "refunded",
            ]
            if direction == "ONRAMP"
            else [
                "unspecified",
                "created",
                "provider-session-created",
                "transfer-required",
                "transfer-submitted",
                "pending",
                "settling",
                "settled",
                "declined",
                "cancelled",
                "failed",
                "refunded",
            ]
        )
        if 0 <= value < len(statuses):
            return statuses[value]
    if isinstance(value, str):
        prefix = f"{direction}_SESSION_STATUS_"
        return (
            value.removeprefix(prefix).lower().replace("_", "-")
            if value.startswith(prefix)
            else value
        )
    raise ValueError("Ramp response has invalid status")


def _wire_string(value: Mapping[str, object], field: str) -> str:
    return _required_string(
        _pick(value, field, _snake_case(field)),
        field,
    )


def _optional_wire_string(value: Mapping[str, object], field: str) -> str | None:
    candidate = _pick(value, field, _snake_case(field))
    return candidate if isinstance(candidate, str) and candidate else None


def _snake_case(value: str) -> str:
    result = ""
    for character in value:
        result += f"_{character.lower()}" if character.isupper() else character
    return result


def _mapping(value: object, label: str) -> Mapping[str, object]:
    if isinstance(value, Mapping):
        return value
    raise ValueError(f"{label} must be an object")


def _sequence(value: object, field: str) -> Sequence[object]:
    if isinstance(value, Sequence) and not isinstance(value, (str, bytes)):
        return value
    raise ValueError(f"Ramp response has invalid {field}")


def _string_tuple(value: object) -> tuple[str, ...]:
    sequence = _sequence(value, "string list")
    if not all(isinstance(item, str) for item in sequence):
        raise ValueError("Ramp response has invalid string list")
    return tuple(cast(str, item) for item in sequence)


def _pick(value: Mapping[str, object], *keys: str) -> object:
    for key in keys:
        if key in value:
            return value[key]
    return None


def _required_string(value: object, field: str) -> str:
    if isinstance(value, str) and value:
        return value
    raise ValueError(f"Ramp response is missing {field}")
