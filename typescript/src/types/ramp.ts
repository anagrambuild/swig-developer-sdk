import type { Network } from './common.js';
import type {
  PreparedTransaction,
  PreparedTransactionWire,
} from './transaction.js';
import type { WalletAuthority } from './wallet-actions.js';

export type MeldEnvironment = 'sandbox' | 'production';

export interface RampConfigurationArgs {
  organizationMeldConfigurationId: string;
  environment: MeldEnvironment;
}

export interface GetRampOptionsArgs extends RampConfigurationArgs {
  countryCode?: string;
  fiatCurrencyCode?: string;
}

export interface RampSubdivisionOption {
  subdivisionCode: string;
  subdivisionName: string;
}

export interface RampCountryOption {
  countryCode: string;
  countryName: string;
  subdivisions: RampSubdivisionOption[];
}

export interface RampCryptoCurrencyOption {
  currencyCode: string;
  currencyName: string;
  iconUrl: string;
  contractAddress: string;
}

export interface RampOptions {
  countries: RampCountryOption[];
  fiatCurrencyCodes: string[];
  paymentMethodTypes: string[];
}

export interface OnrampOptions extends RampOptions {
  cryptoCurrencyCodes: string[];
}

export interface OfframpOptions extends RampOptions {
  cryptoCurrencies: RampCryptoCurrencyOption[];
}

export interface QuoteRampArgs extends RampConfigurationArgs {
  externalCustomerId: string;
  swigConfigAddress: string;
  network?: Network;
  sourceAmount: string;
  sourceCurrencyCode: string;
  destinationCurrencyCode: string;
  countryCode: string;
  subdivision?: string;
  paymentMethodType?: string;
}

export interface RampQuote {
  quoteId: string;
  serviceProvider: string;
  paymentMethodType: string;
  sourceAmount: string;
  sourceCurrencyCode: string;
  destinationAmount: string;
  destinationCurrencyCode: string;
  exchangeRate: string;
  totalFee: string;
}

export interface QuoteRampResult {
  quotes: RampQuote[];
}

export interface CreateRampSessionArgs extends RampConfigurationArgs {
  quoteId: string;
}

export interface CreateRampSessionResult {
  sessionId: string;
  launchUrl: string;
}

export interface GetRampSessionArgs {
  sessionId: string;
  environment: MeldEnvironment;
}

export type OnrampSessionStatus =
  | 'unspecified'
  | 'created'
  | 'pending'
  | 'settling'
  | 'settled'
  | 'failed'
  | 'declined'
  | 'cancelled'
  | 'refunded';

export type OfframpSessionStatus =
  | OnrampSessionStatus
  | 'provider-session-created'
  | 'transfer-required'
  | 'transfer-submitted';

export interface OnrampSession {
  sessionId: string;
  status: OnrampSessionStatus;
  createdAt: string;
  updatedAt: string;
}

export interface OfframpSession extends Omit<OnrampSession, 'status'> {
  status: OfframpSessionStatus;
  sourceAmount: string;
  sourceCurrencyCode: string;
  destinationAmount: string;
  destinationCurrencyCode: string;
  serviceProvider: string;
  paymentMethodType?: string;
  solanaSignature?: string;
  providerDestinationAmount?: string;
}

export interface PrepareOfframpAuthorizationArgs extends GetRampSessionArgs {
  requesterAuthority: WalletAuthority;
  feePayer: string;
}

export interface OfframpTransferDisplay {
  sourceWalletAddress: string;
  destinationWalletAddress: string;
  sourceAmount: string;
  sourceCurrencyCode: string;
  destinationAmount: string;
  destinationCurrencyCode: string;
  serviceProvider: string;
  paymentMethodType?: string;
  providerDestinationAmount?: string;
}

export interface PrepareOfframpAuthorizationResult {
  authorizationId: string;
  preparedTransaction: PreparedTransaction;
  display: OfframpTransferDisplay;
}

export interface SubmitOfframpAuthorizationArgs extends GetRampSessionArgs {
  authorizationId: string;
  /** Base64-encoded signed Solana transaction. */
  signedTransaction: string;
}

export interface SubmitOfframpAuthorizationResult {
  solanaSignature: string;
}

export interface RampCountryOptionWire {
  country_code?: string;
  countryCode?: string;
  country_name?: string;
  countryName?: string;
  subdivisions?: Array<{
    subdivision_code?: string;
    subdivisionCode?: string;
    subdivision_name?: string;
    subdivisionName?: string;
  }>;
}

export interface RampCryptoCurrencyOptionWire {
  currency_code?: string;
  currencyCode?: string;
  currency_name?: string;
  currencyName?: string;
  icon_url?: string;
  iconUrl?: string;
  contract_address?: string;
  contractAddress?: string;
}

export interface OnrampOptionsWire {
  countries?: RampCountryOptionWire[];
  fiat_currency_codes?: string[];
  fiatCurrencyCodes?: string[];
  payment_method_types?: string[];
  paymentMethodTypes?: string[];
  crypto_currency_codes?: string[];
  cryptoCurrencyCodes?: string[];
}

export interface OfframpOptionsWire extends Omit<
  OnrampOptionsWire,
  'crypto_currency_codes' | 'cryptoCurrencyCodes'
> {
  crypto_currencies?: RampCryptoCurrencyOptionWire[];
  cryptoCurrencies?: RampCryptoCurrencyOptionWire[];
}

export interface RampQuoteWire {
  quote_id?: string;
  quoteId?: string;
  service_provider?: string;
  serviceProvider?: string;
  payment_method_type?: string;
  paymentMethodType?: string;
  source_amount?: string;
  sourceAmount?: string;
  source_currency_code?: string;
  sourceCurrencyCode?: string;
  destination_amount?: string;
  destinationAmount?: string;
  destination_currency_code?: string;
  destinationCurrencyCode?: string;
  exchange_rate?: string;
  exchangeRate?: string;
  total_fee?: string;
  totalFee?: string;
}

export interface QuoteRampResultWire {
  quotes?: RampQuoteWire[];
}

export interface CreateRampSessionResultWire {
  session_id?: string;
  sessionId?: string;
  launch_url?: string;
  launchUrl?: string;
}

export interface OnrampSessionWire {
  session_id?: string;
  sessionId?: string;
  status?: string | number;
  created_at?: string;
  createdAt?: string;
  updated_at?: string;
  updatedAt?: string;
}

export interface OfframpSessionWire extends OnrampSessionWire {
  source_amount?: string;
  sourceAmount?: string;
  source_currency_code?: string;
  sourceCurrencyCode?: string;
  destination_amount?: string;
  destinationAmount?: string;
  destination_currency_code?: string;
  destinationCurrencyCode?: string;
  service_provider?: string;
  serviceProvider?: string;
  payment_method_type?: string;
  paymentMethodType?: string;
  solana_signature?: string;
  solanaSignature?: string;
  provider_destination_amount?: string;
  providerDestinationAmount?: string;
}

export interface PrepareOfframpAuthorizationResultWire {
  authorization_id?: string;
  authorizationId?: string;
  prepared_transaction?: PreparedTransactionWire;
  preparedTransaction?: PreparedTransactionWire;
  display?: Record<string, unknown>;
}

export interface SubmitOfframpAuthorizationResultWire {
  solana_signature?: string;
  solanaSignature?: string;
}
