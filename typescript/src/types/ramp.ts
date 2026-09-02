import type { Amount, Network } from './common.js';
import type {
  PreparedTransaction,
  PreparedTransactionWire,
} from './transaction.js';
import type { WalletAuthority } from './wallet-actions.js';

export type RampEnvironment = 'sandbox' | 'production';

/** Only options needs this; every other call infers direction from the order. */
export type RampDirection = 'buy' | 'sell';

/** Native SOL carries no mint, so the two cases cannot be mixed up. */
export type CryptoAsset = { type: 'sol' } | { type: 'token'; mint: string };

/** Minor units: cents for USD, whole yen for JPY. */
export interface FiatAmount {
  currencyCode: string;
  minorUnits: string;
}

export interface CryptoAmount {
  asset: CryptoAsset;
  baseUnits: string;
}

export interface FiatAmountInput {
  currencyCode: string;
  minorUnits: Amount;
}

export interface CryptoAmountInput {
  asset: CryptoAsset;
  baseUnits: Amount;
}

export interface RampRoute {
  provider: string;
  paymentMethod: string;
}

export interface RampLocation {
  countryCode: string;
  subdivisionCode?: string;
}

export type RampOrderRequest =
  | { type: 'buy'; spend: FiatAmountInput; receive: CryptoAsset }
  | { type: 'sell'; sell: CryptoAmountInput; receiveFiatCurrencyCode: string };

export interface RampConfigurationArgs {
  configurationId: string;
  environment: RampEnvironment;
}

export interface GetRampOptionsArgs extends RampConfigurationArgs {
  direction: RampDirection;
  countryCode?: string;
  fiatCurrencyCode?: string;
}

export interface RampSubdivision {
  code: string;
  name: string;
}

export interface RampCountry {
  code: string;
  name: string;
  subdivisions: RampSubdivision[];
}

export interface RampAssetOption {
  asset: CryptoAsset;
  name: string;
  iconUrl?: string;
  decimals: number;
}

export interface RampFiatCurrencyOption {
  currencyCode: string;
  exponent: number;
}

export interface RampOptions {
  countries: RampCountry[];
  fiatCurrencies: RampFiatCurrencyOption[];
  paymentMethods: string[];
  assets: RampAssetOption[];
}

export interface GetRampQuotesArgs extends RampConfigurationArgs {
  location: RampLocation;
  order: RampOrderRequest;
}

export interface RampBuyQuote {
  type: 'buy';
  spend: FiatAmount;
  receive: CryptoAmount;
  totalFee: FiatAmount;
  /** Display-only exact decimal; a rate has no minor unit to scale to. */
  exchangeRate: string;
}

export interface RampSellQuote {
  type: 'sell';
  sell: CryptoAmount;
  receive: FiatAmount;
  totalFee: FiatAmount;
  exchangeRate: string;
}

export type RampQuoteDetails = RampBuyQuote | RampSellQuote;

/**
 * Quotes carry no identifier and must never be cached; the chosen route is
 * re-priced when the order is created.
 */
export type RampQuote = RampQuoteDetails & { route: RampRoute };

export interface RampOrderContext {
  customerId: string;
  swigConfigAddress: string;
  network?: Network;
  location: RampLocation;
}

export interface CreateRampOrderArgs extends RampConfigurationArgs {
  /** Caller-generated idempotency key, unique within the configuration. */
  requestId: string;
  context: RampOrderContext;
  route: RampRoute;
  order: RampOrderRequest;
}

export interface GetRampOrderArgs {
  orderId: string;
}

export type RampOrderStatus =
  | 'unspecified'
  | 'creating'
  | 'creation-uncertain'
  | 'awaiting-customer'
  | 'awaiting-transfer'
  | 'processing'
  | 'settling'
  | 'settled'
  | 'declined'
  | 'cancelled'
  | 'failed'
  | 'refunded';

export type RampTransferState =
  'unspecified' | 'prepared' | 'submitted' | 'landed' | 'failed' | 'expired';

export interface RampDeposit {
  address: string;
  amount: CryptoAmount;
}

export interface RampTransfer {
  transferId: string;
  state: RampTransferState;
  solanaSignature?: string;
  expiresAt: string;
}

interface RampOrderBase {
  id: string;
  status: RampOrderStatus;
  createdAt: string;
  updatedAt: string;
}

export interface RampBuyOrder extends RampOrderBase {
  type: 'buy';
  quote: RampBuyQuote;
  launchUrl?: string;
}

export interface RampSellOrder extends RampOrderBase {
  type: 'sell';
  quote: RampSellQuote;
  launchUrl?: string;
  /** Absent until the provider assigns one. */
  deposit?: RampDeposit;
  /** Absent until a transfer is prepared. */
  transfer?: RampTransfer;
}

export type RampOrder = RampBuyOrder | RampSellOrder;

export interface PrepareRampTransferArgs {
  orderId: string;
  requesterAuthority: WalletAuthority;
  feePayer: string;
}

export interface PreparedRampTransfer {
  transfer: RampTransfer;
  preparedTransaction: PreparedTransaction;
  deposit: RampDeposit;
}

export interface SubmitRampTransferArgs {
  orderId: string;
  transferId: string;
  /**
   * Base64-encoded signed Solana transaction. Omit to resolve an attempt that
   * was already broadcast; the prepared transaction is handed over only once.
   */
  signedTransaction?: string;
}

export interface CryptoAssetWire {
  sol?: Record<string, never>;
  token?: { mint?: string };
}

export interface FiatAmountWire {
  currency_code?: string;
  currencyCode?: string;
  minor_units?: number | string;
  minorUnits?: number | string;
}

export interface CryptoAmountWire {
  asset?: CryptoAssetWire;
  base_units?: number | string;
  baseUnits?: number | string;
}

export interface RampRouteWire {
  provider?: string;
  payment_method?: string;
  paymentMethod?: string;
}

export interface RampSubdivisionWire {
  code?: string;
  name?: string;
}

export interface RampCountryWire {
  code?: string;
  name?: string;
  subdivisions?: RampSubdivisionWire[];
}

export interface RampAssetOptionWire {
  asset?: CryptoAssetWire;
  name?: string;
  icon_url?: string;
  iconUrl?: string;
  decimals?: number | string;
}

export interface RampFiatCurrencyOptionWire {
  currency_code?: string;
  currencyCode?: string;
  exponent?: number | string;
}

export interface RampOptionsWire {
  countries?: RampCountryWire[];
  fiat_currencies?: RampFiatCurrencyOptionWire[];
  fiatCurrencies?: RampFiatCurrencyOptionWire[];
  payment_methods?: string[];
  paymentMethods?: string[];
  assets?: RampAssetOptionWire[];
}

export interface RampBuyQuoteWire {
  spend?: FiatAmountWire;
  receive?: CryptoAmountWire;
  total_fee?: FiatAmountWire;
  totalFee?: FiatAmountWire;
  exchange_rate?: string;
  exchangeRate?: string;
}

export interface RampSellQuoteWire {
  sell?: CryptoAmountWire;
  receive?: FiatAmountWire;
  total_fee?: FiatAmountWire;
  totalFee?: FiatAmountWire;
  exchange_rate?: string;
  exchangeRate?: string;
}

export interface RampQuoteWire {
  route?: RampRouteWire;
  buy?: RampBuyQuoteWire;
  sell?: RampSellQuoteWire;
}

export interface GetRampQuotesResponseWire {
  quotes?: RampQuoteWire[];
}

export interface RampDepositWire {
  address?: string;
  amount?: CryptoAmountWire;
}

export interface RampTransferWire {
  transfer_id?: string;
  transferId?: string;
  state?: string;
  solana_signature?: string;
  solanaSignature?: string;
  expires_at?: string;
  expiresAt?: string;
}

export interface RampBuyOrderDetailsWire {
  quote?: RampBuyQuoteWire;
  launch_url?: string;
  launchUrl?: string;
}

export interface RampSellOrderDetailsWire {
  quote?: RampSellQuoteWire;
  launch_url?: string;
  launchUrl?: string;
  deposit?: RampDepositWire;
  transfer?: RampTransferWire;
}

export interface RampOrderWire {
  id?: string;
  status?: string;
  created_at?: string;
  createdAt?: string;
  updated_at?: string;
  updatedAt?: string;
  buy?: RampBuyOrderDetailsWire;
  sell?: RampSellOrderDetailsWire;
}

export interface RampOrderResponseWire {
  order?: RampOrderWire;
}

export interface PreparedRampTransferWire {
  transfer?: RampTransferWire;
  prepared_transaction?: PreparedTransactionWire;
  preparedTransaction?: PreparedTransactionWire;
  deposit?: RampDepositWire;
}

export interface PrepareRampTransferResponseWire {
  prepared_transfer?: PreparedRampTransferWire;
  preparedTransfer?: PreparedRampTransferWire;
}

export interface SubmitRampTransferResponseWire {
  transfer?: RampTransferWire;
}
