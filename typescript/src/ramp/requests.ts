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
import { normalizeAmount, toProtoNetwork } from '../wallets/normalizers.js';

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
