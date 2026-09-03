import type {
  CreateRampOrderArgs,
  CryptoAsset,
  GetRampQuotesArgs,
  Network,
  RampDirection,
  RampEnvironment,
  RampLocation,
  RampOrderRequest,
} from '../types/index.js';
import { toProtoNetwork } from '../wallets/normalizers.js';

const MAX_UINT64 = 18_446_744_073_709_551_615n;

export function quotesRequest(args: GetRampQuotesArgs) {
  return {
    configurationId: args.configurationId,
    environment: environmentWire(args.environment),
    location: locationRequest(args.location),
    ...orderRequestBody(args.order),
  };
}

export function orderRequest(
  args: CreateRampOrderArgs,
  defaultNetwork?: Network,
) {
  const network = args.context.network ?? defaultNetwork;
  if (!network) throw new Error('network is required');
  return {
    requestId: args.requestId,
    configurationId: args.configurationId,
    environment: environmentWire(args.environment),
    context: {
      customerId: args.context.customerId,
      swigConfigAddress: args.context.swigConfigAddress,
      network: toProtoNetwork(network),
      location: locationRequest(args.context.location),
    },
    route: {
      provider: args.route.provider,
      paymentMethod: args.route.paymentMethod,
    },
    ...orderRequestBody(args.order),
  };
}

function locationRequest(location: RampLocation) {
  return {
    countryCode: location.countryCode,
    ...(location.subdivisionCode
      ? { subdivisionCode: location.subdivisionCode }
      : {}),
  };
}

/** Direction lives in the oneof, so there is no flag for it to disagree with. */
function orderRequestBody(order: RampOrderRequest) {
  if (order.type === 'buy') {
    return {
      buy: {
        spend: {
          currencyCode: order.spend.currencyCode,
          minorUnits: normalizeRampAmount(order.spend.minorUnits),
        },
        receive: assetRequest(order.receive),
      },
    };
  }
  return {
    sell: {
      sell: {
        asset: assetRequest(order.sell.asset),
        baseUnits: normalizeRampAmount(order.sell.baseUnits),
      },
      receiveFiatCurrencyCode: order.receiveFiatCurrencyCode,
    },
  };
}

function normalizeRampAmount(amount: string | number | bigint): string {
  if (
    typeof amount === 'number' &&
    (!Number.isSafeInteger(amount) || amount < 0)
  ) {
    throw new Error(
      'Ramp amount numbers must be non-negative safe integers; use a bigint or decimal string for larger values',
    );
  }

  const normalized = amount.toString();
  if (!/^\d+$/.test(normalized)) {
    throw new Error('Ramp amounts must be non-negative decimal integers');
  }
  if (BigInt(normalized) > MAX_UINT64) {
    throw new Error('Ramp amounts must fit in an unsigned 64-bit integer');
  }
  return normalized;
}

/** Native SOL is an empty message, so it is `{ sol: {} }` on the wire. */
function assetRequest(asset: CryptoAsset) {
  if (asset.type === 'sol') return { sol: {} };
  return { token: { mint: asset.mint } };
}

export function environmentWire(environment: RampEnvironment): string {
  switch (environment) {
    case 'production':
      return 'RAMP_ENVIRONMENT_PRODUCTION';
    case 'sandbox':
      return 'RAMP_ENVIRONMENT_SANDBOX';
    default:
      throw new Error('environment must be "sandbox" or "production"');
  }
}

export function directionWire(direction: RampDirection): string {
  switch (direction) {
    case 'buy':
      return 'RAMP_DIRECTION_BUY';
    case 'sell':
      return 'RAMP_DIRECTION_SELL';
    default:
      throw new Error('direction must be "buy" or "sell"');
  }
}
