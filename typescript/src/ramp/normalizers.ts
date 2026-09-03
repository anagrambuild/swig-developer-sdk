import type {
  CryptoAmount,
  CryptoAmountWire,
  CryptoAsset,
  CryptoAssetWire,
  FiatAmount,
  FiatAmountWire,
  GetRampQuotesResponseWire,
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
  RampFiatCurrencyOption,
  RampFiatCurrencyOptionWire,
  RampOptions,
  RampOptionsWire,
  RampOrder,
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
} from '../types/index.js';
import { normalizePreparedTransaction } from '../wallets/normalizers.js';

export function normalizeRampOptions(response: RampOptionsWire): RampOptions {
  return {
    countries: (response.countries ?? []).map(normalizeRampCountry),
    fiatCurrencies: (
      response.fiatCurrencies ??
      response.fiat_currencies ??
      []
    ).map(normalizeRampFiatCurrencyOption),
    paymentMethods: response.paymentMethods ?? response.payment_methods ?? [],
    assets: (response.assets ?? []).map(normalizeRampAssetOption),
  };
}

function normalizeRampCountry(country: RampCountryWire): RampCountry {
  return {
    code: requiredString(country.code, 'code'),
    name: requiredString(country.name, 'name'),
    subdivisions: (country.subdivisions ?? []).map(normalizeRampSubdivision),
  };
}

function normalizeRampSubdivision(
  subdivision: RampSubdivisionWire,
): RampSubdivision {
  return {
    code: requiredString(subdivision.code, 'code'),
    name: requiredString(subdivision.name, 'name'),
  };
}

function normalizeRampFiatCurrencyOption(
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

function normalizeRampAssetOption(asset: RampAssetOptionWire): RampAssetOption {
  const iconUrl = asset.iconUrl ?? asset.icon_url;
  return {
    asset: normalizeCryptoAsset(asset.asset),
    name: requiredString(asset.name, 'name'),
    decimals: numberField(asset.decimals, 'decimals'),
    ...(iconUrl ? { iconUrl } : {}),
  };
}

export function normalizeRampQuotes(
  response: GetRampQuotesResponseWire,
): RampQuote[] {
  return (response.quotes ?? []).map(normalizeRampQuote);
}

function normalizeRampQuote(quote: RampQuoteWire): RampQuote {
  const route = normalizeRampRoute(quote.route);
  if (quote.buy) return { route, details: normalizeRampBuyQuote(quote.buy) };
  if (quote.sell) return { route, details: normalizeRampSellQuote(quote.sell) };
  throw new Error('Ramp response is missing quote');
}

function normalizeRampBuyQuote(quote: RampBuyQuoteWire): RampBuyQuote {
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

function normalizeRampSellQuote(quote: RampSellQuoteWire): RampSellQuote {
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

function normalizeRampRoute(route?: RampRouteWire): RampRoute {
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
  return normalizeRampOrder(requiredField(response.order, 'order'));
}

function normalizeRampOrder(order: RampOrderWire): RampOrder {
  const base = {
    id: requiredString(order.id, 'id'),
    status: normalizeRampOrderStatus(order.status),
    createdAt: requiredString(order.createdAt ?? order.created_at, 'createdAt'),
    updatedAt: requiredString(order.updatedAt ?? order.updated_at, 'updatedAt'),
  };

  if (order.buy) {
    const launchUrl = order.buy.launchUrl ?? order.buy.launch_url;
    return {
      ...base,
      type: 'buy',
      quote: normalizeRampBuyQuote(requiredField(order.buy.quote, 'quote')),
      ...(launchUrl ? { launchUrl } : {}),
    };
  }

  if (order.sell) {
    const launchUrl = order.sell.launchUrl ?? order.sell.launch_url;
    return {
      ...base,
      type: 'sell',
      quote: normalizeRampSellQuote(requiredField(order.sell.quote, 'quote')),
      ...(launchUrl ? { launchUrl } : {}),
      ...(order.sell.deposit
        ? { deposit: normalizeRampDeposit(order.sell.deposit) }
        : {}),
      ...(order.sell.transfer
        ? { transfer: normalizeRampTransfer(order.sell.transfer) }
        : {}),
    };
  }

  throw new Error('Ramp response is missing order details');
}

export function normalizePreparedRampTransfer(
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
    deposit: normalizeRampDeposit(requiredField(response.deposit, 'deposit')),
  };
}

export function normalizeRampTransfer(
  transfer: RampTransferWire,
): RampTransfer {
  const solanaSignature = transfer.solanaSignature ?? transfer.solana_signature;
  return {
    transferId: requiredString(
      transfer.transferId ?? transfer.transfer_id,
      'transferId',
    ),
    state: normalizeRampTransferState(transfer.state),
    expiresAt: requiredString(
      transfer.expiresAt ?? transfer.expires_at,
      'expiresAt',
    ),
    ...(solanaSignature ? { solanaSignature } : {}),
  };
}

function normalizeRampDeposit(deposit: RampDepositWire): RampDeposit {
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
    asset: normalizeCryptoAsset(wire.asset),
    baseUnits: unitsField(wire.baseUnits ?? wire.base_units, 'baseUnits'),
  };
}

/** Key presence, not truthiness: native SOL arrives as an empty object. */
function normalizeCryptoAsset(asset?: CryptoAssetWire): CryptoAsset {
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

function normalizeRampOrderStatus(value?: string): RampOrderStatus {
  const status = value ? ORDER_STATUSES[value] : undefined;
  if (!status) throw new Error('Ramp response has invalid status');
  return status;
}

function normalizeRampTransferState(value?: string): RampTransferState {
  const state = value ? TRANSFER_STATES[value] : undefined;
  if (!state) throw new Error('Ramp response has invalid state');
  return state;
}

export function requiredField<TValue>(
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
  if (typeof value === 'number' && Number.isFinite(value)) {
    return value;
  }
  // A blank string is not a number, matching readNumber in wallets.
  if (typeof value === 'string' && value.trim()) {
    const parsed = Number(value);
    if (Number.isFinite(parsed)) {
      return parsed;
    }
  }
  throw new Error(`Ramp response is missing ${field}`);
}
