import type { HttpClient } from '../core/index.js';
import type {
  CreateRampOrderArgs,
  CryptoAmount,
  CryptoAmountWire,
  CryptoAsset,
  CryptoAssetWire,
  FiatAmount,
  FiatAmountWire,
  GetRampOptionsArgs,
  GetRampOrderArgs,
  GetRampQuotesArgs,
  GetRampQuotesResponseWire,
  Network,
  PrepareRampTransferArgs,
  PrepareRampTransferResponseWire,
  PreparedRampTransfer,
  PreparedRampTransferWire,
  RampAssetOption,
  RampAssetOptionWire,
  RampBuyQuote,
  RampBuyQuoteWire,
  RampCountry,
  RampCountryWire,
  RampDeposit,
  RampDepositWire,
  RampDirection,
  RampEnvironment,
  RampFiatCurrencyOption,
  RampFiatCurrencyOptionWire,
  RampOptions,
  RampOptionsWire,
  RampOrder,
  RampOrderRequest,
  RampOrderResponseWire,
  RampOrderStatus,
  RampOrderWire,
  RampQuote,
  RampQuoteWire,
  RampRoute,
  RampRouteWire,
  RampSellQuote,
  RampSellQuoteWire,
  RampSubdivision,
  RampSubdivisionWire,
  RampTransfer,
  RampTransferState,
  RampTransferWire,
  SubmitRampTransferArgs,
  SubmitRampTransferResponseWire,
} from '../types/index.js';
import {
  normalizeAmount,
  normalizePreparedTransaction,
} from '../wallets/normalizers.js';

export class RampClient {
  constructor(
    private readonly http: HttpClient,
    private readonly defaultNetwork?: Network,
  ) {}

  getOptions = async (args: GetRampOptionsArgs): Promise<RampOptions> =>
    normalizeRampOptions(
      await this.http.get<RampOptionsWire>(optionsPath(args)),
    );

  getQuotes = async (args: GetRampQuotesArgs): Promise<RampQuote[]> =>
    normalizeRampQuotes(
      await this.http.post<GetRampQuotesResponseWire>(
        '/wallet/api/ramp/quotes',
        quotesRequest(args),
      ),
    );

  createOrder = async (args: CreateRampOrderArgs): Promise<RampOrder> =>
    // requestId makes a replay return the stored order, so a retry is safe.
    normalizeRampOrderResponse(
      await this.http.post<RampOrderResponseWire>(
        '/wallet/api/ramp/orders',
        orderRequest(args, this.defaultNetwork),
        { retry: true },
      ),
    );

  getOrder = async (args: GetRampOrderArgs): Promise<RampOrder> =>
    normalizeRampOrderResponse(
      await this.http.get<RampOrderResponseWire>(orderPath(args.orderId)),
    );

  prepareTransfer = async (
    args: PrepareRampTransferArgs,
  ): Promise<PreparedRampTransfer> => {
    const response = await this.http.post<PrepareRampTransferResponseWire>(
      `${orderPath(args.orderId)}/transfer/prepare`,
      {
        requesterAuthority: args.requesterAuthority,
        feePayer: args.feePayer,
      },
    );
    return normalizePreparedRampTransfer(
      requiredField(
        response.preparedTransfer ?? response.prepared_transfer,
        'preparedTransfer',
      ),
    );
  };

  submitTransfer = async (
    args: SubmitRampTransferArgs,
  ): Promise<RampTransfer> => {
    const response = await this.http.post<SubmitRampTransferResponseWire>(
      `${orderPath(args.orderId)}/transfer/submit`,
      {
        transferId: args.transferId,
        signedTransaction: args.signedTransaction ?? '',
      },
    );
    return normalizeRampTransfer(requiredField(response.transfer, 'transfer'));
  };
}

function optionsPath(args: GetRampOptionsArgs): string {
  const query = new URLSearchParams({
    configurationId: args.configurationId,
    environment: environmentWire(args.environment),
    direction: directionWire(args.direction),
  });
  if (args.countryCode) query.set('countryCode', args.countryCode);
  if (args.fiatCurrencyCode)
    query.set('fiatCurrencyCode', args.fiatCurrencyCode);
  return `/wallet/api/ramp/options?${query.toString()}`;
}

function orderPath(orderId: string): string {
  return `/wallet/api/ramp/orders/${encodeURIComponent(orderId)}`;
}

function quotesRequest(args: GetRampQuotesArgs) {
  return {
    configurationId: args.configurationId,
    environment: environmentWire(args.environment),
    location: locationRequest(args.location),
    ...orderRequestBody(args.order),
  };
}

function orderRequest(args: CreateRampOrderArgs, defaultNetwork?: Network) {
  const network = args.context.network ?? defaultNetwork;
  if (!network) throw new Error('network is required');
  return {
    requestId: args.requestId,
    configurationId: args.configurationId,
    environment: environmentWire(args.environment),
    context: {
      customerId: args.context.customerId,
      swigConfigAddress: args.context.swigConfigAddress,
      network: network === 'mainnet' ? 'NETWORK_MAINNET' : 'NETWORK_DEVNET',
      location: locationRequest(args.context.location),
    },
    route: {
      provider: args.route.provider,
      paymentMethod: args.route.paymentMethod,
    },
    ...orderRequestBody(args.order),
  };
}

function locationRequest(location: {
  countryCode: string;
  subdivisionCode?: string;
}) {
  return {
    countryCode: location.countryCode,
    ...optionalString('subdivisionCode', location.subdivisionCode),
  };
}

/** Direction lives in the oneof, so there is no flag for it to disagree with. */
function orderRequestBody(order: RampOrderRequest) {
  if (order.type === 'buy') {
    return {
      buy: {
        spend: {
          currencyCode: order.spend.currencyCode,
          minorUnits: normalizeAmount(order.spend.minorUnits),
        },
        receive: assetRequest(order.receive),
      },
    };
  }
  return {
    sell: {
      sell: {
        asset: assetRequest(order.sell.asset),
        baseUnits: normalizeAmount(order.sell.baseUnits),
      },
      receiveFiatCurrencyCode: order.receiveFiatCurrencyCode,
    },
  };
}

/** Native SOL is an empty message, so it is `{ sol: {} }` on the wire. */
function assetRequest(asset: CryptoAsset) {
  if (asset.type === 'sol') return { sol: {} };
  return { token: { mint: asset.mint } };
}

function environmentWire(environment: RampEnvironment): string {
  switch (environment) {
    case 'production':
      return 'RAMP_ENVIRONMENT_PRODUCTION';
    case 'sandbox':
      return 'RAMP_ENVIRONMENT_SANDBOX';
    default:
      throw new Error('environment must be "sandbox" or "production"');
  }
}

function directionWire(direction: RampDirection): string {
  switch (direction) {
    case 'buy':
      return 'RAMP_DIRECTION_BUY';
    case 'sell':
      return 'RAMP_DIRECTION_SELL';
    default:
      throw new Error('direction must be "buy" or "sell"');
  }
}

export function normalizeRampOptions(response: RampOptionsWire): RampOptions {
  return {
    countries: (response.countries ?? []).map(normalizeCountry),
    fiatCurrencies: (
      response.fiatCurrencies ??
      response.fiat_currencies ??
      []
    ).map(normalizeFiatCurrency),
    paymentMethods: response.paymentMethods ?? response.payment_methods ?? [],
    assets: (response.assets ?? []).map(normalizeAssetOption),
  };
}

function normalizeCountry(country: RampCountryWire): RampCountry {
  return {
    code: requiredString(country.code, 'code'),
    name: requiredString(country.name, 'name'),
    subdivisions: (country.subdivisions ?? []).map(normalizeSubdivision),
  };
}

function normalizeSubdivision(
  subdivision: RampSubdivisionWire,
): RampSubdivision {
  return {
    code: requiredString(subdivision.code, 'code'),
    name: requiredString(subdivision.name, 'name'),
  };
}

function normalizeFiatCurrency(
  currency: RampFiatCurrencyOptionWire,
): RampFiatCurrencyOption {
  return {
    currencyCode: requiredString(
      currency.currencyCode ?? currency.currency_code,
      'currencyCode',
    ),
    exponent: numberField(currency.exponent, 'exponent'),
  };
}

function normalizeAssetOption(asset: RampAssetOptionWire): RampAssetOption {
  return {
    asset: normalizeAsset(asset.asset),
    name: requiredString(asset.name, 'name'),
    decimals: numberField(asset.decimals, 'decimals'),
    ...optionalString('iconUrl', asset.iconUrl ?? asset.icon_url),
  };
}

export function normalizeRampQuotes(
  response: GetRampQuotesResponseWire,
): RampQuote[] {
  return (response.quotes ?? []).map(normalizeQuote);
}

function normalizeQuote(quote: RampQuoteWire): RampQuote {
  const route = normalizeRoute(quote.route);
  if (quote.buy) return { ...normalizeBuyQuote(quote.buy), route };
  if (quote.sell) return { ...normalizeSellQuote(quote.sell), route };
  throw new Error('Ramp response is missing quote');
}

function normalizeBuyQuote(quote: RampBuyQuoteWire): RampBuyQuote {
  return {
    type: 'buy',
    spend: normalizeFiatAmount(quote.spend, 'spend'),
    receive: normalizeCryptoAmount(quote.receive, 'receive'),
    totalFee: normalizeFiatAmount(
      quote.totalFee ?? quote.total_fee,
      'totalFee',
    ),
    exchangeRate: requiredString(
      quote.exchangeRate ?? quote.exchange_rate,
      'exchangeRate',
    ),
  };
}

function normalizeSellQuote(quote: RampSellQuoteWire): RampSellQuote {
  return {
    type: 'sell',
    sell: normalizeCryptoAmount(quote.sell, 'sell'),
    receive: normalizeFiatAmount(quote.receive, 'receive'),
    totalFee: normalizeFiatAmount(
      quote.totalFee ?? quote.total_fee,
      'totalFee',
    ),
    exchangeRate: requiredString(
      quote.exchangeRate ?? quote.exchange_rate,
      'exchangeRate',
    ),
  };
}

function normalizeRoute(route?: RampRouteWire): RampRoute {
  const wire = requiredField(route, 'route');
  return {
    provider: requiredString(wire.provider, 'provider'),
    paymentMethod: requiredString(
      wire.paymentMethod ?? wire.payment_method,
      'paymentMethod',
    ),
  };
}

export function normalizeRampOrderResponse(
  response: RampOrderResponseWire,
): RampOrder {
  return normalizeOrder(requiredField(response.order, 'order'));
}

function normalizeOrder(order: RampOrderWire): RampOrder {
  const base = {
    id: requiredString(order.id, 'id'),
    status: normalizeOrderStatus(order.status),
    createdAt: requiredString(order.createdAt ?? order.created_at, 'createdAt'),
    updatedAt: requiredString(order.updatedAt ?? order.updated_at, 'updatedAt'),
  };

  if (order.buy) {
    return {
      ...base,
      type: 'buy',
      quote: normalizeBuyQuote(requiredField(order.buy.quote, 'quote')),
      ...optionalString(
        'launchUrl',
        order.buy.launchUrl ?? order.buy.launch_url,
      ),
    };
  }

  if (order.sell) {
    return {
      ...base,
      type: 'sell',
      quote: normalizeSellQuote(requiredField(order.sell.quote, 'quote')),
      ...optionalString(
        'launchUrl',
        order.sell.launchUrl ?? order.sell.launch_url,
      ),
      ...(order.sell.deposit
        ? { deposit: normalizeDeposit(order.sell.deposit) }
        : {}),
      ...(order.sell.transfer
        ? { transfer: normalizeRampTransfer(order.sell.transfer) }
        : {}),
    };
  }

  throw new Error('Ramp response is missing order details');
}

function normalizePreparedRampTransfer(
  response: PreparedRampTransferWire,
): PreparedRampTransfer {
  return {
    transfer: normalizeRampTransfer(
      requiredField(response.transfer, 'transfer'),
    ),
    preparedTransaction: normalizePreparedTransaction(
      requiredField(
        response.preparedTransaction ?? response.prepared_transaction,
        'preparedTransaction',
      ),
    ),
    deposit: normalizeDeposit(requiredField(response.deposit, 'deposit')),
  };
}

export function normalizeRampTransfer(
  transfer: RampTransferWire,
): RampTransfer {
  return {
    transferId: requiredString(
      transfer.transferId ?? transfer.transfer_id,
      'transferId',
    ),
    state: normalizeTransferState(transfer.state),
    expiresAt: requiredString(
      transfer.expiresAt ?? transfer.expires_at,
      'expiresAt',
    ),
    ...optionalString(
      'solanaSignature',
      transfer.solanaSignature ?? transfer.solana_signature,
    ),
  };
}

function normalizeDeposit(deposit: RampDepositWire): RampDeposit {
  return {
    address: requiredString(deposit.address, 'address'),
    amount: normalizeCryptoAmount(deposit.amount, 'amount'),
  };
}

function normalizeFiatAmount(
  amount: FiatAmountWire | undefined,
  field: string,
): FiatAmount {
  const wire = requiredField(amount, field);
  return {
    currencyCode: requiredString(
      wire.currencyCode ?? wire.currency_code,
      'currencyCode',
    ),
    minorUnits: unitsField(wire.minorUnits ?? wire.minor_units, 'minorUnits'),
  };
}

function normalizeCryptoAmount(
  amount: CryptoAmountWire | undefined,
  field: string,
): CryptoAmount {
  const wire = requiredField(amount, field);
  return {
    asset: normalizeAsset(wire.asset),
    baseUnits: unitsField(wire.baseUnits ?? wire.base_units, 'baseUnits'),
  };
}

/** Key presence, not truthiness: native SOL arrives as an empty object. */
function normalizeAsset(asset?: CryptoAssetWire): CryptoAsset {
  if (asset && 'sol' in asset) return { type: 'sol' };
  if (asset && 'token' in asset) {
    return { type: 'token', mint: requiredString(asset.token?.mint, 'mint') };
  }
  throw new Error('Ramp response is missing asset');
}

const ORDER_STATUSES: Record<string, RampOrderStatus> = {
  RAMP_ORDER_STATUS_UNSPECIFIED: 'unspecified',
  RAMP_ORDER_STATUS_CREATING: 'creating',
  RAMP_ORDER_STATUS_CREATION_UNCERTAIN: 'creation-uncertain',
  RAMP_ORDER_STATUS_AWAITING_CUSTOMER: 'awaiting-customer',
  RAMP_ORDER_STATUS_AWAITING_TRANSFER: 'awaiting-transfer',
  RAMP_ORDER_STATUS_PROCESSING: 'processing',
  RAMP_ORDER_STATUS_SETTLING: 'settling',
  RAMP_ORDER_STATUS_SETTLED: 'settled',
  RAMP_ORDER_STATUS_DECLINED: 'declined',
  RAMP_ORDER_STATUS_CANCELLED: 'cancelled',
  RAMP_ORDER_STATUS_FAILED: 'failed',
  RAMP_ORDER_STATUS_REFUNDED: 'refunded',
};

const TRANSFER_STATES: Record<string, RampTransferState> = {
  TRANSFER_STATE_UNSPECIFIED: 'unspecified',
  TRANSFER_STATE_PREPARED: 'prepared',
  TRANSFER_STATE_SUBMITTED: 'submitted',
  TRANSFER_STATE_LANDED: 'landed',
  TRANSFER_STATE_FAILED: 'failed',
  TRANSFER_STATE_EXPIRED: 'expired',
};

function normalizeOrderStatus(value?: string): RampOrderStatus {
  const status = value ? ORDER_STATUSES[value] : undefined;
  if (!status) throw new Error('Ramp response has invalid status');
  return status;
}

function normalizeTransferState(value?: string): RampTransferState {
  const state = value ? TRANSFER_STATES[value] : undefined;
  if (!state) throw new Error('Ramp response has invalid state');
  return state;
}

function requiredField<TValue>(
  value: TValue | undefined,
  field: string,
): TValue {
  if (value === undefined || value === null) {
    throw new Error(`Ramp response is missing ${field}`);
  }
  return value;
}

function requiredString(value: unknown, field: string): string {
  if (typeof value !== 'string' || value.length === 0) {
    throw new Error(`Ramp response is missing ${field}`);
  }
  return value;
}

/** uint64 crosses the wire as a decimal string; keeping it one avoids 2^53. */
function unitsField(value: number | string | undefined, field: string): string {
  if (typeof value !== 'string' || !/^\d+$/.test(value)) {
    throw new Error(`Ramp response has invalid ${field}`);
  }
  return value;
}

function numberField(
  value: number | string | undefined,
  field: string,
): number {
  const parsed = typeof value === 'string' ? Number(value) : value;
  if (typeof parsed !== 'number' || !Number.isFinite(parsed)) {
    throw new Error(`Ramp response is missing ${field}`);
  }
  return parsed;
}

function optionalString<TKey extends string>(key: TKey, value: unknown) {
  return typeof value === 'string' && value.length > 0
    ? ({ [key]: value } as Record<TKey, string>)
    : {};
}
