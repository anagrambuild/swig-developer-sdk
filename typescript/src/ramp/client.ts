import type { HttpClient } from '../core/index.js';
import type {
  CreateRampOrderArgs,
  GetRampOptionsArgs,
  GetRampOrderArgs,
  GetRampQuotesArgs,
  GetRampQuotesResponseWire,
  Network,
  PrepareRampTransferArgs,
  PrepareRampTransferResponseWire,
  PreparedRampTransfer,
  RampOptions,
  RampOptionsWire,
  RampOrder,
  RampOrderResponseWire,
  RampQuote,
  RampTransfer,
  SubmitRampTransferArgs,
  SubmitRampTransferResponseWire,
} from '../types/index.js';
import { walletAuthorityRequest } from '../wallets/requests.js';
import {
  normalizePreparedRampTransfer,
  normalizeRampOptions,
  normalizeRampOrderResponse,
  normalizeRampQuotes,
  normalizeRampTransfer,
  requiredField,
} from './normalizers.js';
import {
  directionWire,
  environmentWire,
  orderRequest,
  quotesRequest,
} from './requests.js';

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
        requesterAuthority: walletAuthorityRequest(args.requesterAuthority),
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
