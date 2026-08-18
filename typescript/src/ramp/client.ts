import type { HttpClient } from '../core/index.js';
import type {
  CreateRampSessionArgs,
  CreateRampSessionResult,
  CreateRampSessionResultWire,
  GetRampOptionsArgs,
  GetRampSessionArgs,
  MeldEnvironment,
  Network,
  OfframpOptions,
  OfframpOptionsWire,
  OfframpSession,
  OfframpSessionStatus,
  OfframpSessionWire,
  OfframpTransferDisplay,
  OnrampOptions,
  OnrampOptionsWire,
  OnrampSession,
  OnrampSessionStatus,
  OnrampSessionWire,
  PrepareOfframpAuthorizationArgs,
  PrepareOfframpAuthorizationResult,
  PrepareOfframpAuthorizationResultWire,
  QuoteRampArgs,
  QuoteRampResult,
  QuoteRampResultWire,
  RampCountryOption,
  RampCountryOptionWire,
  RampCryptoCurrencyOption,
  RampCryptoCurrencyOptionWire,
  RampQuote,
  RampQuoteWire,
  SubmitOfframpAuthorizationArgs,
  SubmitOfframpAuthorizationResult,
  SubmitOfframpAuthorizationResultWire,
} from '../types/index.js';
import { normalizePreparedTransaction } from '../wallets/normalizers.js';

export class RampClient {
  readonly onramp: OnrampClient;
  readonly offramp: OfframpClient;

  constructor(http: HttpClient, defaultNetwork?: Network) {
    this.onramp = new OnrampClient(http, defaultNetwork);
    this.offramp = new OfframpClient(http, defaultNetwork);
  }
}

export class OnrampClient {
  constructor(
    private readonly http: HttpClient,
    private readonly defaultNetwork?: Network,
  ) {}

  getOptions = async (args: GetRampOptionsArgs): Promise<OnrampOptions> =>
    normalizeOnrampOptions(
      await this.http.get<OnrampOptionsWire>(optionsPath('onramp', args)),
    );

  quote = async (args: QuoteRampArgs): Promise<QuoteRampResult> =>
    normalizeQuoteRampResult(
      await this.http.post<QuoteRampResultWire>(
        '/wallet/api/ramp/onramp/quote',
        quoteRequest(args, this.defaultNetwork),
      ),
    );

  createSession = async (
    args: CreateRampSessionArgs,
  ): Promise<CreateRampSessionResult> =>
    normalizeCreateRampSessionResult(
      await this.http.post<CreateRampSessionResultWire>(
        '/wallet/api/ramp/onramp/session',
        sessionRequest(args),
      ),
    );

  getSession = async (args: GetRampSessionArgs): Promise<OnrampSession> =>
    normalizeOnrampSession(
      await this.http.get<OnrampSessionWire>(sessionPath('onramp', args)),
    );
}

export class OfframpClient {
  constructor(
    private readonly http: HttpClient,
    private readonly defaultNetwork?: Network,
  ) {}

  getOptions = async (args: GetRampOptionsArgs): Promise<OfframpOptions> =>
    normalizeOfframpOptions(
      await this.http.get<OfframpOptionsWire>(optionsPath('offramp', args)),
    );

  quote = async (args: QuoteRampArgs): Promise<QuoteRampResult> =>
    normalizeQuoteRampResult(
      await this.http.post<QuoteRampResultWire>(
        '/wallet/api/ramp/offramp/quote',
        quoteRequest(args, this.defaultNetwork),
      ),
    );

  createSession = async (
    args: CreateRampSessionArgs,
  ): Promise<CreateRampSessionResult> =>
    normalizeCreateRampSessionResult(
      await this.http.post<CreateRampSessionResultWire>(
        '/wallet/api/ramp/offramp/session',
        sessionRequest(args),
      ),
    );

  prepareAuthorization = async (
    args: PrepareOfframpAuthorizationArgs,
  ): Promise<PrepareOfframpAuthorizationResult> => {
    const response =
      await this.http.post<PrepareOfframpAuthorizationResultWire>(
        `/wallet/api/ramp/offramp/session/${encodeURIComponent(args.sessionId)}/prepare`,
        {
          requesterAuthority: args.requesterAuthority,
          environment: environmentWire(args.environment),
          feePayer: args.feePayer,
        },
      );
    const prepared =
      response.preparedTransaction ?? response.prepared_transaction;
    if (!prepared) {
      throw new Error('Ramp response is missing preparedTransaction');
    }
    return {
      authorizationId: requiredString(
        response.authorizationId ?? response.authorization_id,
        'authorizationId',
      ),
      preparedTransaction: normalizePreparedTransaction(prepared),
      display: normalizeTransferDisplay(response.display),
    };
  };

  submitAuthorization = async (
    args: SubmitOfframpAuthorizationArgs,
  ): Promise<SubmitOfframpAuthorizationResult> => {
    const response = await this.http.post<SubmitOfframpAuthorizationResultWire>(
      `/wallet/api/ramp/offramp/session/${encodeURIComponent(args.sessionId)}/submit`,
      {
        authorizationId: args.authorizationId,
        signedTransaction: args.signedTransaction,
        environment: environmentWire(args.environment),
      },
    );
    return {
      solanaSignature: requiredString(
        response.solanaSignature ?? response.solana_signature,
        'solanaSignature',
      ),
    };
  };

  getSession = async (args: GetRampSessionArgs): Promise<OfframpSession> =>
    normalizeOfframpSession(
      await this.http.get<OfframpSessionWire>(sessionPath('offramp', args)),
    );
}

export function normalizeOnrampOptions(
  response: OnrampOptionsWire,
): OnrampOptions {
  return {
    ...normalizeCommonOptions(response),
    cryptoCurrencyCodes:
      response.cryptoCurrencyCodes ?? response.crypto_currency_codes ?? [],
  };
}

export function normalizeOfframpOptions(
  response: OfframpOptionsWire,
): OfframpOptions {
  return {
    ...normalizeCommonOptions(response),
    cryptoCurrencies: (
      response.cryptoCurrencies ??
      response.crypto_currencies ??
      []
    ).map(normalizeCryptoCurrency),
  };
}

export function normalizeQuoteRampResult(
  response: QuoteRampResultWire,
): QuoteRampResult {
  return { quotes: (response.quotes ?? []).map(normalizeQuote) };
}

export function normalizeCreateRampSessionResult(
  response: CreateRampSessionResultWire,
): CreateRampSessionResult {
  return {
    sessionId: requiredString(
      response.sessionId ?? response.session_id,
      'sessionId',
    ),
    launchUrl: requiredString(
      response.launchUrl ?? response.launch_url,
      'launchUrl',
    ),
  };
}

export function normalizeOnrampSession(
  response: OnrampSessionWire,
): OnrampSession {
  return {
    ...normalizeSessionBase(response),
    status: normalizeOnrampStatus(response.status),
  };
}

export function normalizeOfframpSession(
  response: OfframpSessionWire,
): OfframpSession {
  const base = normalizeSessionBase(response);
  return {
    ...base,
    status: normalizeOfframpStatus(response.status),
    sourceAmount: requiredString(
      response.sourceAmount ?? response.source_amount,
      'sourceAmount',
    ),
    sourceCurrencyCode: requiredString(
      response.sourceCurrencyCode ?? response.source_currency_code,
      'sourceCurrencyCode',
    ),
    destinationAmount: requiredString(
      response.destinationAmount ?? response.destination_amount,
      'destinationAmount',
    ),
    destinationCurrencyCode: requiredString(
      response.destinationCurrencyCode ?? response.destination_currency_code,
      'destinationCurrencyCode',
    ),
    serviceProvider: requiredString(
      response.serviceProvider ?? response.service_provider,
      'serviceProvider',
    ),
    ...optionalString(
      'paymentMethodType',
      response.paymentMethodType ?? response.payment_method_type,
    ),
    ...optionalString(
      'solanaSignature',
      response.solanaSignature ?? response.solana_signature,
    ),
    ...optionalString(
      'providerDestinationAmount',
      response.providerDestinationAmount ??
        response.provider_destination_amount,
    ),
  };
}

function optionsPath(
  direction: 'onramp' | 'offramp',
  args: GetRampOptionsArgs,
): string {
  const query = new URLSearchParams({
    organizationMeldConfigurationId: args.organizationMeldConfigurationId,
    environment: environmentWire(args.environment),
  });
  if (args.countryCode) query.set('countryCode', args.countryCode);
  if (args.fiatCurrencyCode)
    query.set('fiatCurrencyCode', args.fiatCurrencyCode);
  return `/wallet/api/ramp/${direction}/options?${query.toString()}`;
}

function sessionPath(
  direction: 'onramp' | 'offramp',
  args: GetRampSessionArgs,
): string {
  const query = new URLSearchParams({
    environment: environmentWire(args.environment),
  });
  return `/wallet/api/ramp/${direction}/session/${encodeURIComponent(args.sessionId)}?${query.toString()}`;
}

function quoteRequest(args: QuoteRampArgs, defaultNetwork?: Network) {
  const network = args.network ?? defaultNetwork;
  if (!network) throw new Error('network is required');
  return {
    organizationMeldConfigurationId: args.organizationMeldConfigurationId,
    externalCustomerId: args.externalCustomerId,
    swigConfigAddress: args.swigConfigAddress,
    network: network === 'mainnet' ? 'NETWORK_MAINNET' : 'NETWORK_DEVNET',
    sourceAmount: args.sourceAmount,
    sourceCurrencyCode: args.sourceCurrencyCode,
    destinationCurrencyCode: args.destinationCurrencyCode,
    countryCode: args.countryCode,
    subdivision: args.subdivision,
    paymentMethodType: args.paymentMethodType,
    environment: environmentWire(args.environment),
  };
}

function sessionRequest(args: CreateRampSessionArgs) {
  return {
    organizationMeldConfigurationId: args.organizationMeldConfigurationId,
    quoteId: args.quoteId,
    environment: environmentWire(args.environment),
  };
}

function environmentWire(environment: MeldEnvironment): string {
  switch (environment) {
    case 'production':
      return 'MELD_ENVIRONMENT_PRODUCTION';
    case 'sandbox':
      return 'MELD_ENVIRONMENT_SANDBOX';
    default:
      throw new Error('environment must be "sandbox" or "production"');
  }
}

function normalizeCommonOptions(
  response: Pick<
    OnrampOptionsWire,
    | 'countries'
    | 'fiatCurrencyCodes'
    | 'fiat_currency_codes'
    | 'paymentMethodTypes'
    | 'payment_method_types'
  >,
) {
  return {
    countries: (response.countries ?? []).map(normalizeCountry),
    fiatCurrencyCodes:
      response.fiatCurrencyCodes ?? response.fiat_currency_codes ?? [],
    paymentMethodTypes:
      response.paymentMethodTypes ?? response.payment_method_types ?? [],
  };
}

function normalizeCountry(country: RampCountryOptionWire): RampCountryOption {
  return {
    countryCode: requiredString(
      country.countryCode ?? country.country_code,
      'countryCode',
    ),
    countryName: requiredString(
      country.countryName ?? country.country_name,
      'countryName',
    ),
    subdivisions: (country.subdivisions ?? []).map((subdivision) => ({
      subdivisionCode: requiredString(
        subdivision.subdivisionCode ?? subdivision.subdivision_code,
        'subdivisionCode',
      ),
      subdivisionName: requiredString(
        subdivision.subdivisionName ?? subdivision.subdivision_name,
        'subdivisionName',
      ),
    })),
  };
}

function normalizeCryptoCurrency(
  currency: RampCryptoCurrencyOptionWire,
): RampCryptoCurrencyOption {
  return {
    currencyCode: requiredString(
      currency.currencyCode ?? currency.currency_code,
      'currencyCode',
    ),
    currencyName: requiredString(
      currency.currencyName ?? currency.currency_name,
      'currencyName',
    ),
    iconUrl: requiredString(currency.iconUrl ?? currency.icon_url, 'iconUrl'),
    contractAddress: requiredString(
      currency.contractAddress ?? currency.contract_address,
      'contractAddress',
    ),
  };
}

function normalizeQuote(quote: RampQuoteWire): RampQuote {
  return {
    quoteId: requiredString(quote.quoteId ?? quote.quote_id, 'quoteId'),
    serviceProvider: requiredString(
      quote.serviceProvider ?? quote.service_provider,
      'serviceProvider',
    ),
    paymentMethodType: requiredString(
      quote.paymentMethodType ?? quote.payment_method_type,
      'paymentMethodType',
    ),
    sourceAmount: requiredString(
      quote.sourceAmount ?? quote.source_amount,
      'sourceAmount',
    ),
    sourceCurrencyCode: requiredString(
      quote.sourceCurrencyCode ?? quote.source_currency_code,
      'sourceCurrencyCode',
    ),
    destinationAmount: requiredString(
      quote.destinationAmount ?? quote.destination_amount,
      'destinationAmount',
    ),
    destinationCurrencyCode: requiredString(
      quote.destinationCurrencyCode ?? quote.destination_currency_code,
      'destinationCurrencyCode',
    ),
    exchangeRate: requiredString(
      quote.exchangeRate ?? quote.exchange_rate,
      'exchangeRate',
    ),
    totalFee: requiredString(quote.totalFee ?? quote.total_fee, 'totalFee'),
  };
}

function normalizeSessionBase(response: OnrampSessionWire) {
  return {
    sessionId: requiredString(
      response.sessionId ?? response.session_id,
      'sessionId',
    ),
    createdAt: requiredString(
      response.createdAt ?? response.created_at,
      'createdAt',
    ),
    updatedAt: requiredString(
      response.updatedAt ?? response.updated_at,
      'updatedAt',
    ),
  };
}

function normalizeTransferDisplay(
  value: Record<string, unknown> | undefined,
): OfframpTransferDisplay {
  if (!value) throw new Error('Ramp response is missing display');
  return {
    sourceWalletAddress: readWireString(value, 'sourceWalletAddress'),
    destinationWalletAddress: readWireString(value, 'destinationWalletAddress'),
    sourceAmount: readWireString(value, 'sourceAmount'),
    sourceCurrencyCode: readWireString(value, 'sourceCurrencyCode'),
    destinationAmount: readWireString(value, 'destinationAmount'),
    destinationCurrencyCode: readWireString(value, 'destinationCurrencyCode'),
    serviceProvider: readWireString(value, 'serviceProvider'),
    ...optionalString(
      'paymentMethodType',
      readOptionalWireString(value, 'paymentMethodType'),
    ),
    ...optionalString(
      'providerDestinationAmount',
      readOptionalWireString(value, 'providerDestinationAmount'),
    ),
  };
}

function readWireString(value: Record<string, unknown>, field: string): string {
  return requiredString(value[field] ?? value[toSnakeCase(field)], field);
}

function readOptionalWireString(
  value: Record<string, unknown>,
  field: string,
): string | undefined {
  const candidate = value[field] ?? value[toSnakeCase(field)];
  return typeof candidate === 'string' ? candidate : undefined;
}

function toSnakeCase(value: string): string {
  return value.replace(/[A-Z]/g, (character) => `_${character.toLowerCase()}`);
}

function normalizeOnrampStatus(
  value: string | number | undefined,
): OnrampSessionStatus {
  const statuses: OnrampSessionStatus[] = [
    'unspecified',
    'created',
    'pending',
    'settling',
    'settled',
    'failed',
    'declined',
    'cancelled',
    'refunded',
  ];
  return normalizeStatus(value, statuses, 'ONRAMP_SESSION_STATUS_');
}

function normalizeOfframpStatus(
  value: string | number | undefined,
): OfframpSessionStatus {
  const statuses: OfframpSessionStatus[] = [
    'unspecified',
    'created',
    'provider-session-created',
    'transfer-required',
    'transfer-submitted',
    'pending',
    'settling',
    'settled',
    'declined',
    'cancelled',
    'failed',
    'refunded',
  ];
  return normalizeStatus(value, statuses, 'OFFRAMP_SESSION_STATUS_');
}

function normalizeStatus<TStatus extends string>(
  value: string | number | undefined,
  statuses: TStatus[],
  prefix: string,
): TStatus {
  if (typeof value === 'number' && statuses[value]) return statuses[value];
  if (typeof value === 'string') {
    const normalized = value.startsWith(prefix)
      ? value.slice(prefix.length).toLowerCase().replaceAll('_', '-')
      : value;
    if (statuses.includes(normalized as TStatus)) return normalized as TStatus;
  }
  throw new Error('Ramp response has invalid status');
}

function requiredString(value: unknown, field: string): string {
  if (typeof value !== 'string' || value.length === 0) {
    throw new Error(`Ramp response is missing ${field}`);
  }
  return value;
}

function optionalString<TKey extends string>(key: TKey, value: unknown) {
  return typeof value === 'string' && value.length > 0
    ? ({ [key]: value } as Record<TKey, string>)
    : {};
}
