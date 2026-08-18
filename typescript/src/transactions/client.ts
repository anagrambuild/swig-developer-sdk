import bs58 from 'bs58';
import type { HttpClient } from '../core/index.js';
import type {
  Network,
  SponsorSignedTransactionArgs,
  SponsorSignedTransactionBundleArgs,
  SubmittedTransaction,
  SubmittedTransactionBundle,
  SubmittedTransactionBundleWire,
  SubmittedTransactionWire,
} from '../types/index.js';
import { normalizeSubmittedTransaction } from '../wallets/normalizers.js';

export class TransactionsClient {
  constructor(
    private readonly http: HttpClient,
    private readonly defaultNetwork?: Network,
  ) {}

  sponsor = async (
    args: SponsorSignedTransactionArgs,
  ): Promise<SubmittedTransaction> => {
    const response = await this.http.post<SubmittedTransactionWire>(
      '/paymaster/sponsor',
      {
        base58_encoded_transaction: bs58.encode(
          base64ToBytes(args.transaction),
        ),
        network: args.network ?? this.defaultNetwork,
        idempotencyKey: args.idempotencyKey,
      },
      { retry: Boolean(args.idempotencyKey) },
    );

    return normalizeSubmittedTransaction(response);
  };

  sponsorBundle = async (
    args: SponsorSignedTransactionBundleArgs,
  ): Promise<SubmittedTransactionBundle> => {
    if (args.transactions.length === 0 || args.transactions.length > 5) {
      throw new Error('transactions must contain between 1 and 5 items');
    }
    const network = args.network ?? this.defaultNetwork;
    if (network !== 'mainnet') {
      throw new Error('sponsorBundle only supports mainnet');
    }
    const response = await this.http.post<SubmittedTransactionBundleWire>(
      '/paymaster/sponsor/bundle',
      {
        base58_encoded_transactions: args.transactions.map((transaction) =>
          bs58.encode(base64ToBytes(transaction)),
        ),
        network,
        idempotencyKey: args.idempotencyKey,
      },
      { retry: Boolean(args.idempotencyKey) },
    );
    return {
      requestId: requiredString(
        response.requestId ?? response.request_id,
        'requestId',
      ),
      bundleId: requiredString(
        response.bundleId ?? response.bundle_id,
        'bundleId',
      ),
      signatures: response.signatures ?? [],
      estimatedSpentByPaymaster: String(
        response.estimatedSpentByPaymaster ??
          response.estimated_spent_by_paymaster ??
          '0',
      ),
    };
  };
}

function requiredString(value: unknown, field: string): string {
  if (typeof value !== 'string' || value.length === 0) {
    throw new Error(`Sponsor bundle response is missing ${field}`);
  }
  return value;
}

function base64ToBytes(value: string): Uint8Array {
  const binary = atob(value);
  const bytes = new Uint8Array(binary.length);

  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }

  return bytes;
}
