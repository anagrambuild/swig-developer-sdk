from __future__ import annotations

import inspect
import os
import re
from collections.abc import Awaitable, Callable, Mapping, Sequence
from dataclasses import dataclass, fields, is_dataclass
from typing import Literal, TypeAlias, TypeVar, cast
from urllib.parse import unquote

import httpx
from pydantic import BaseModel

from .client import SwigClient
from .common import DEFAULT_BACKEND_URL, Network, WalletAuthority, normalize_network
from .paymaster import PaymasterBalanceKind
from .ramp import (
    CryptoAmountInput,
    CryptoAsset,
    FiatAmountInput,
    NativeSolAsset,
    RampBuyOrderRequest,
    RampDirection,
    RampEnvironment,
    RampLocation,
    RampOrderContext,
    RampOrderRequest,
    RampRoute,
    RampSellOrderRequest,
    SplTokenAsset,
)
from .wallets import (
    RecoveryOptions,
    TransferSolOperation,
    TransferTokenOperation,
    WalletReference,
)
from .x402 import validate_payment_required

PostProxyRoute: TypeAlias = Literal[
    "wallet/create",
    "prepare",
    "transfer/sol",
    "transfer/spl-token",
    "swap/jupiter",
    "x402/prepare",
    "ramp/quotes",
    "ramp/orders",
    "ramp/transfer/prepare",
    "ramp/transfer/submit",
]
ReadProxyRoute: TypeAlias = Literal[
    "wallet/balance/usd",
    "wallet/token-balances",
    "wallet/token-transactions",
    "wallet/roles",
    "paymaster/balance",
    "ramp/options",
    "ramp/order",
]
ProxyRoute: TypeAlias = PostProxyRoute | ReadProxyRoute
T = TypeVar("T")
MaybeAwaitable: TypeAlias = T | Awaitable[T]


@dataclass(frozen=True, slots=True)
class ProxyWalletReference(WalletReference):
    role_id: int | None = None
    authority_public_key: str | None = None


@dataclass(frozen=True, slots=True)
class SwigRouteContext:
    method: Literal["GET", "POST"]
    path: str
    route: ProxyRoute
    body: Mapping[str, object]
    query: Mapping[str, str]
    wallet: ProxyWalletReference | None = None
    network: Network | None = None


@dataclass(frozen=True, slots=True)
class SwigProxyConfig:
    api_key: str | None = None
    transaction_api_url: str | None = None
    base_url: str | None = None
    network: Network | None = None
    fee_payer: str | Callable[[SwigRouteContext], MaybeAwaitable[str | None]] | None = (
        None
    )
    resolve_requester_authority: (
        Callable[[SwigRouteContext], MaybeAwaitable[WalletAuthority | None]] | None
    ) = None
    resolve_requester_pubkey: (
        Callable[[SwigRouteContext], MaybeAwaitable[str | None]] | None
    ) = None
    transport: httpx.AsyncBaseTransport | None = None


@dataclass(frozen=True, slots=True)
class ProxyResponse:
    status: int
    body: Mapping[str, object]


class SwigProxyRouteError(Exception):
    def __init__(self, message: str, status: int = 400) -> None:
        super().__init__(message)
        self.status = status


class SwigProxyHandler:
    def __init__(self, config: SwigProxyConfig | None = None) -> None:
        self._config = config or SwigProxyConfig()

    async def handle(
        self,
        *,
        method: Literal["GET", "POST"],
        path: str,
        body: Mapping[str, object] | None = None,
        query: Mapping[str, str] | None = None,
    ) -> ProxyResponse:
        try:
            if method == "POST":
                value = await self._handle_post(path, body or {}, query or {})
            elif method == "GET":
                value = await self._handle_get(path, query or {})
            else:
                raise SwigProxyRouteError("Unsupported method", 405)
            serialized = _to_wire(value)
            if not isinstance(serialized, Mapping):
                raise TypeError("Proxy response must be an object")
            return ProxyResponse(200, serialized)
        except SwigProxyRouteError as error:
            return ProxyResponse(error.status, {"error": str(error)})
        except Exception as error:
            return ProxyResponse(400, {"error": str(error)})

    async def _handle_post(
        self,
        path: str,
        body: Mapping[str, object],
        query: Mapping[str, str],
    ) -> Mapping[str, object]:
        route, order_id = _resolve_post_route(path)
        network = _read_network(body.get("network")) or self._config.network
        wallet = None if route.startswith("ramp/") else _read_wallet(body.get("wallet"))
        context = SwigRouteContext(
            method="POST",
            path=path,
            route=route,
            body=body,
            query=query,
            wallet=wallet,
            network=network,
        )
        swig = self._client(network)
        if route == "wallet/create":
            fee_payer = await self._fee_payer(context)
            policy_id = _optional_string(body.get("policyId"))
            initial_user = _read_authority(body.get("initialUser"))
            if policy_id is None and initial_user is None:
                raise SwigProxyRouteError("policyId or initialUser is required")
            created = await swig.wallets.create(
                fee_payer=fee_payer,
                policy_id=policy_id,
                initial_user=initial_user,
                recovery=_read_recovery_options(body.get("recovery")),
                network=network,
            )
            if not created.transactions:
                raise SwigProxyRouteError(
                    "Wallet creation response is missing transaction", 502
                )
            return {"prepared": created}
        if route == "ramp/quotes":
            # The client returns the quotes themselves; the proxy answers with an
            # object, so they are wrapped the way the backend wraps them.
            quotes = await swig.ramp.get_quotes(
                configuration_id=_required_string(body, "configurationId"),
                environment=_read_ramp_environment(body.get("environment")),
                location=_read_ramp_location(body.get("location")),
                order=_read_ramp_order(body),
            )
            return {"quotes": _to_wire(quotes)}
        if route == "ramp/orders":
            return cast(
                Mapping[str, object],
                _to_wire(
                    await swig.ramp.create_order(
                        request_id=_required_string(body, "requestId"),
                        configuration_id=_required_string(body, "configurationId"),
                        environment=_read_ramp_environment(body.get("environment")),
                        context=_read_ramp_context(body.get("context"), network),
                        route=_read_ramp_route(body.get("route")),
                        order=_read_ramp_order(body),
                    )
                ),
            )
        if route == "ramp/transfer/prepare":
            return cast(
                Mapping[str, object],
                _to_wire(
                    await swig.ramp.prepare_transfer(
                        order_id=_require_order_id(order_id),
                        requester_authority=_require_authority(
                            _read_authority(body.get("requesterAuthority"))
                        ),
                        fee_payer=_required_string(body, "feePayer"),
                    )
                ),
            )
        if route == "ramp/transfer/submit":
            return cast(
                Mapping[str, object],
                _to_wire(
                    await swig.ramp.submit_transfer(
                        order_id=_require_order_id(order_id),
                        transfer_id=_required_string(body, "transferId"),
                        signed_transaction=_optional_string(
                            body.get("signedTransaction")
                        )
                        or "",
                    )
                ),
            )

        required_wallet = _require_wallet(wallet)
        requester_authority = await self._requester_authority(context)
        handle = swig.wallets.use(
            required_wallet,
            network=network,
            requester_authority=requester_authority,
        )
        if route == "x402/prepare":
            accepted_index = body.get("acceptedIndex")
            if accepted_index is not None and (
                not isinstance(accepted_index, int) or isinstance(accepted_index, bool)
            ):
                raise SwigProxyRouteError("acceptedIndex must be a non-negative uint32")
            x402_prepared = await handle.x402._prepare_payment_required(
                validate_payment_required(body.get("paymentRequired")),
                accepted_index=accepted_index,
            )
            return {"prepared": x402_prepared}

        fee_payer = await self._fee_payer(context)
        prepared: object
        if route == "prepare":
            prepared = await handle.prepare(
                fee_payer=fee_payer,
                requester_authority=requester_authority,
                network=network,
                operations=_read_operations(body.get("operations")),
            )
        elif route == "transfer/sol":
            prepared = await handle.transfer.sol(
                fee_payer=fee_payer,
                requester_authority=requester_authority,
                network=network,
                destination=_required_string(body, "destination"),
                amount=_read_amount(body),
            )
        elif route == "transfer/spl-token":
            prepared = await handle.transfer.token(
                fee_payer=fee_payer,
                requester_authority=requester_authority,
                network=network,
                mint=_required_string(body, "mint"),
                destination_owner=_required_string(body, "destinationOwner"),
                amount=_read_amount(body),
            )
        else:
            prepared = await handle.swap.jupiter(
                fee_payer=fee_payer,
                requester_authority=requester_authority,
                network=network,
                input_mint=_required_string(body, "inputMint"),
                output_mint=_required_string(body, "outputMint"),
                amount=_read_amount(body),
                slippage_bps=_optional_int(body.get("slippageBps")),
                destination_account=_optional_string(body.get("destinationAccount")),
                wrap_and_unwrap_sol=_optional_bool(body.get("wrapAndUnwrapSol")),
                tip_amount_lamports=_optional_string(body.get("tipAmountLamports")),
                compute_unit_price_percentile=_optional_string(
                    body.get("computeUnitPricePercentile")
                ),
                max_accounts=_optional_int(body.get("maxAccounts")),
                mode=_optional_string(body.get("mode")),
                blockhash_slots_to_expiry=_optional_int(
                    body.get("blockhashSlotsToExpiry")
                ),
            )
        return {"prepared": prepared}

    async def _handle_get(
        self,
        path: str,
        query: Mapping[str, str],
    ) -> Mapping[str, object]:
        route, identifier = _resolve_read_route(path)
        network = _read_network(query.get("network")) or self._config.network
        swig = self._client(network)
        if route == "paymaster/balance":
            kind_value = query.get("kind", "").strip().upper()
            kind: PaymasterBalanceKind | None
            if kind_value in ("",):
                kind = None
            elif kind_value in ("API", "PAYMASTER_KIND_API"):
                kind = "api"
            elif kind_value in ("IDP", "PAYMASTER_KIND_IDP"):
                kind = "idp"
            else:
                raise SwigProxyRouteError("kind must be api or idp")
            return cast(
                Mapping[str, object],
                _to_wire(await swig.paymaster.get_balance(network=network, kind=kind)),
            )
        if route == "ramp/options":
            return cast(
                Mapping[str, object],
                _to_wire(
                    await swig.ramp.get_options(
                        configuration_id=_required_query_string(
                            query, "configurationId"
                        ),
                        environment=_read_ramp_environment(query.get("environment")),
                        direction=_read_ramp_direction(query.get("direction")),
                        country_code=query.get("countryCode"),
                        fiat_currency_code=query.get("fiatCurrencyCode"),
                    )
                ),
            )
        if route == "ramp/order":
            return cast(
                Mapping[str, object],
                _to_wire(await swig.ramp.get_order(order_id=identifier)),
            )

        handle = swig.wallets.use(identifier, network=network)
        result: object
        if route == "wallet/balance/usd":
            result = await handle.get_usd_balance(network=network)
        elif route == "wallet/token-balances":
            result = await handle.list_token_balances(network=network)
        elif route == "wallet/token-transactions":
            result = await handle.list_token_transactions(
                network=network,
                limit=_optional_int(query.get("limit")),
            )
        else:
            result = await handle.list_roles(network=network)
        return cast(Mapping[str, object], _to_wire(result))

    def _client(self, network: Network | None) -> SwigClient:
        api_key = self._config.api_key or _read_env(
            "SWIG_DEVELOPER_API_KEY", "SWIG_API_KEY"
        )
        if not api_key:
            raise SwigProxyRouteError("SWIG_DEVELOPER_API_KEY is required", 500)
        base_url = (
            self._config.transaction_api_url
            or self._config.base_url
            or _read_env(
                "SWIG_TRANSACTION_API_URL",
                "SWIG_BACKEND_URL",
                "NEXT_PUBLIC_SWIG_BACKEND_URL",
            )
        )
        return SwigClient(
            api_key=api_key,
            base_url=base_url or DEFAULT_BACKEND_URL,
            network=network,
            transport=self._config.transport,
        )

    async def _fee_payer(self, context: SwigRouteContext) -> str:
        configured = self._config.fee_payer
        if callable(configured):
            configured = await _resolve(configured(context))
        value = (
            configured
            or _optional_string(context.body.get("feePayer"))
            or _read_env(
                "SWIG_FEE_PAYER",
                "SWIG_TRANSFER_FEE_PAYER",
                "SWIG_TRANSACTION_FEE_PAYER",
            )
        )
        if not value:
            raise SwigProxyRouteError("feePayer is required")
        return value

    async def _requester_authority(self, context: SwigRouteContext) -> WalletAuthority:
        authority = (
            context.wallet.requester_authority if context.wallet else None
        ) or _read_authority(context.body.get("requesterAuthority"))
        if authority is None and self._config.resolve_requester_authority:
            authority = await _resolve(
                self._config.resolve_requester_authority(context)
            )
        if authority is None and self._config.resolve_requester_pubkey:
            public_key = await _resolve(self._config.resolve_requester_pubkey(context))
            if public_key:
                authority = {"ed25519": {"publicKey": public_key}}
        if authority is None:
            raise SwigProxyRouteError("requesterAuthority is required")
        return authority


def create_swig_proxy_handler(
    config: SwigProxyConfig | None = None,
) -> SwigProxyHandler:
    return SwigProxyHandler(config)


def _resolve_post_route(path: str) -> tuple[PostProxyRoute, str | None]:
    routes: tuple[PostProxyRoute, ...] = (
        "wallet/create",
        "x402/prepare",
        "prepare",
        "transfer/sol",
        "transfer/spl-token",
        "swap/jupiter",
        "ramp/quotes",
        "ramp/orders",
    )
    normalized = path.rstrip("/")
    action = re.search(r"/ramp/orders/([^/]+)/transfer/(prepare|submit)$", normalized)
    if action:
        route: PostProxyRoute = cast(PostProxyRoute, f"ramp/transfer/{action.group(2)}")
        return route, unquote(action.group(1))
    for route in routes:
        if normalized == f"/{route}" or normalized.endswith(f"/{route}"):
            return route, None
    raise SwigProxyRouteError("Unsupported Swig route", 404)


def _resolve_read_route(path: str) -> tuple[ReadProxyRoute, str]:
    normalized = path.rstrip("/")
    if normalized.endswith("/paymaster/balance"):
        return "paymaster/balance", ""
    if normalized.endswith("/ramp/options"):
        return "ramp/options", ""
    order = re.search(r"/ramp/orders/([^/]+)$", normalized)
    if order:
        return "ramp/order", unquote(order.group(1))
    match = re.search(
        r"/wallet/([^/]+)/(balance/usd|token-balances|token-transactions|roles)$",
        normalized,
    )
    if not match:
        raise SwigProxyRouteError("Unsupported Swig route", 404)
    route_map: dict[str, ReadProxyRoute] = {
        "balance/usd": "wallet/balance/usd",
        "token-balances": "wallet/token-balances",
        "token-transactions": "wallet/token-transactions",
        "roles": "wallet/roles",
    }
    return route_map[match.group(2)], unquote(match.group(1))


def _read_wallet(value: object) -> ProxyWalletReference | None:
    if value is None:
        return None
    body = _mapping(value, "wallet")
    return ProxyWalletReference(
        swig_config_address=_required_string(body, "swigConfigAddress"),
        wallet_address=_optional_string(body.get("walletAddress")),
        role_id=_optional_int(body.get("roleId")),
        authority_public_key=_optional_string(body.get("authorityPublicKey")),
        requester_authority=_read_authority(body.get("requesterAuthority")),
        network=_read_network(body.get("network")),
    )


def _read_authority(value: object) -> WalletAuthority | None:
    if value is None:
        return None
    body = _mapping(value, "authority")
    for scheme in ("ed25519", "secp256k1", "secp256r1"):
        nested = body.get(scheme)
        if isinstance(nested, Mapping):
            public_key = _optional_string(nested.get("publicKey"))
            if public_key:
                return {scheme: {"publicKey": public_key}}
    proof = body.get("programExecProof")
    if isinstance(proof, Mapping):
        role_id = _optional_int(proof.get("roleId"))
        zk_proof = _optional_string(proof.get("zkProof"))
        if role_id is not None and zk_proof:
            return {"programExecProof": {"roleId": role_id, "zkProof": zk_proof}}
    raise SwigProxyRouteError("authority must include a supported authority")


def _read_recovery_options(value: object) -> RecoveryOptions | None:
    if value is None:
        return None
    body = _mapping(value, "recovery")
    return RecoveryOptions(
        guardian_pubkey=_optional_string(body.get("guardianPubkey")),
        delay_seconds=_optional_int(body.get("delaySeconds")),
        target_role_id=_optional_int(body.get("targetRoleId")),
    )


def _read_operations(
    value: object,
) -> tuple[TransferSolOperation | TransferTokenOperation, ...]:
    if not isinstance(value, Sequence) or isinstance(value, (str, bytes)) or not value:
        raise SwigProxyRouteError("operations must be a non-empty array")
    operations: list[TransferSolOperation | TransferTokenOperation] = []
    for index, item in enumerate(value):
        body = _mapping(item, f"operations[{index}]")
        if body.get("type") == "transferSol":
            operations.append(
                TransferSolOperation(
                    _required_string(body, "destination"), _read_amount(body)
                )
            )
        elif body.get("type") == "transferToken":
            operations.append(
                TransferTokenOperation(
                    _required_string(body, "mint"),
                    _required_string(body, "destinationOwner"),
                    _read_amount(body),
                )
            )
        else:
            raise SwigProxyRouteError(
                f"operations[{index}].type must be transferSol or transferToken"
            )
    return tuple(operations)


def _read_amount(body: Mapping[str, object]) -> str:
    amount = _required_string(body, "amount")
    if not amount.isdigit() or int(amount) <= 0:
        raise SwigProxyRouteError("amount must be a positive integer string")
    return amount


def _require_wallet(value: ProxyWalletReference | None) -> ProxyWalletReference:
    if value is None:
        raise SwigProxyRouteError("wallet is required")
    return value


def _require_authority(value: WalletAuthority | None) -> WalletAuthority:
    if value is None:
        raise SwigProxyRouteError("requesterAuthority is required")
    return value


def _require_order_id(value: str | None) -> str:
    if not value:
        raise SwigProxyRouteError("orderId is required")
    return value


def _read_ramp_environment(value: object) -> RampEnvironment:
    if value in ("sandbox", "RAMP_ENVIRONMENT_SANDBOX"):
        return "sandbox"
    if value in ("production", "RAMP_ENVIRONMENT_PRODUCTION"):
        return "production"
    raise SwigProxyRouteError("environment must be sandbox or production")


def _read_ramp_direction(value: object) -> RampDirection:
    if value in ("buy", "RAMP_DIRECTION_BUY"):
        return "buy"
    if value in ("sell", "RAMP_DIRECTION_SELL"):
        return "sell"
    raise SwigProxyRouteError("direction must be buy or sell")


def _read_ramp_location(value: object) -> RampLocation:
    if not isinstance(value, Mapping):
        raise SwigProxyRouteError("location is required")
    country_code = value.get("countryCode")
    if not isinstance(country_code, str) or not country_code:
        raise SwigProxyRouteError("countryCode is required")
    return RampLocation(
        country_code=country_code,
        subdivision_code=_optional_string(value.get("subdivisionCode")),
    )


def _read_ramp_asset(value: object) -> CryptoAsset:
    if not isinstance(value, Mapping):
        raise SwigProxyRouteError("asset is required")
    if "sol" in value:
        return NativeSolAsset()
    token = value.get("token")
    if isinstance(token, Mapping):
        mint = token.get("mint")
        if isinstance(mint, str) and mint:
            return SplTokenAsset(mint=mint)
    raise SwigProxyRouteError("asset must be sol or token")


def _read_ramp_amount(value: object, field: str) -> str:
    """Amount is int | str in the SDK, so a JSON number is valid here."""
    if isinstance(value, str) and value:
        return value
    if isinstance(value, int) and not isinstance(value, bool):
        return str(value)
    raise SwigProxyRouteError(f"{field} must be an integer or a decimal string")


def _read_ramp_order(body: Mapping[str, object]) -> RampOrderRequest:
    buy = body.get("buy")
    if isinstance(buy, Mapping):
        spend = buy.get("spend")
        if not isinstance(spend, Mapping):
            raise SwigProxyRouteError("spend is required")
        return RampBuyOrderRequest(
            spend=FiatAmountInput(
                currency_code=_required_string(spend, "currencyCode"),
                minor_units=_read_ramp_amount(spend.get("minorUnits"), "minorUnits"),
            ),
            receive=_read_ramp_asset(buy.get("receive")),
        )
    sell = body.get("sell")
    if isinstance(sell, Mapping):
        inner = sell.get("sell")
        if not isinstance(inner, Mapping):
            raise SwigProxyRouteError("sell is required")
        return RampSellOrderRequest(
            sell=CryptoAmountInput(
                asset=_read_ramp_asset(inner.get("asset")),
                base_units=_read_ramp_amount(inner.get("baseUnits"), "baseUnits"),
            ),
            receive_fiat_currency_code=_required_string(
                sell, "receiveFiatCurrencyCode"
            ),
        )
    raise SwigProxyRouteError("a buy or sell order is required")


def _read_ramp_context(value: object, network: Network | None) -> RampOrderContext:
    if not isinstance(value, Mapping):
        raise SwigProxyRouteError("context is required")
    return RampOrderContext(
        customer_id=_required_string(value, "customerId"),
        swig_config_address=_required_string(value, "swigConfigAddress"),
        location=_read_ramp_location(value.get("location")),
        network=_read_network(value.get("network")) or network,
    )


def _read_ramp_route(value: object) -> RampRoute:
    if not isinstance(value, Mapping):
        raise SwigProxyRouteError("route is required")
    return RampRoute(
        provider=_required_string(value, "provider"),
        payment_method=_required_string(value, "paymentMethod"),
    )


def _read_network(value: object) -> Network | None:
    if isinstance(value, str) and value in ("1", "2"):
        value = int(value)
    return normalize_network(value)


def _required_string(body: Mapping[str, object], key: str) -> str:
    value = _optional_string(body.get(key))
    if not value:
        raise SwigProxyRouteError(f"{key} is required")
    return value


def _required_query_string(query: Mapping[str, str], key: str) -> str:
    value = _optional_string(query.get(key))
    if not value:
        raise SwigProxyRouteError(f"{key} is required")
    return value


def _optional_string(value: object) -> str | None:
    return value.strip() if isinstance(value, str) and value.strip() else None


def _optional_int(value: object) -> int | None:
    if isinstance(value, int) and not isinstance(value, bool):
        return value
    if isinstance(value, str):
        try:
            return int(value)
        except ValueError:
            return None
    return None


def _optional_bool(value: object) -> bool | None:
    return value if isinstance(value, bool) else None


def _mapping(value: object, label: str) -> Mapping[str, object]:
    if isinstance(value, Mapping):
        return value
    raise SwigProxyRouteError(f"{label} must be an object")


def _read_env(*names: str) -> str | None:
    for name in names:
        value = os.environ.get(name, "").strip()
        if value:
            return value
    return None


def _to_wire(value: object) -> object:
    if isinstance(value, BaseModel):
        return value.model_dump(by_alias=True, exclude_unset=True)
    if is_dataclass(value) and not isinstance(value, type):
        result: dict[str, object] = {}
        for field in fields(value):
            item = getattr(value, field.name)
            if item is not None:
                result[_snake_to_camel(field.name)] = _to_wire(item)
        return result
    if isinstance(value, Mapping):
        return {
            str(key): _to_wire(item) for key, item in value.items() if item is not None
        }
    if isinstance(value, (list, tuple)):
        return [_to_wire(item) for item in value]
    return value


def _snake_to_camel(value: str) -> str:
    head, *tail = value.split("_")
    return head + "".join(part.capitalize() for part in tail)


async def _resolve(value: MaybeAwaitable[T]) -> T:
    if inspect.isawaitable(value):
        return await cast(Awaitable[T], value)
    return value
