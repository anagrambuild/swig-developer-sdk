import type { HttpClient } from '../core/index.js';
import type {
  DexTransaction,
  GetDexTransactionArgs,
  GetDexTransactionWire,
  ListDexPositionsArgs,
  ListDexPositionsResult,
  ListDexPositionsWire,
  ListDexTransactionsArgs,
  ListDexTransactionsResult,
  ListDexTransactionsWire,
  Network,
} from '../types/index.js';
import {
  normalizeDexPositionsPage,
  normalizeDexTransaction,
  normalizeDexTransactionsPage,
  requiredField,
} from './normalizers.js';
import {
  positionsSearch,
  transactionPath,
  transactionsSearch,
} from './requests.js';

/** Read-only DEX history and latest positions of a Swig the API key's organization owns. */
export class DexClient {
  readonly transactions: DexTransactionsClient;
  readonly positions: DexPositionsClient;

  constructor(http: HttpClient, defaultNetwork?: Network) {
    this.transactions = new DexTransactionsClient(http, defaultNetwork);
    this.positions = new DexPositionsClient(http, defaultNetwork);
  }
}

export class DexTransactionsClient {
  constructor(
    private readonly http: HttpClient,
    private readonly defaultNetwork?: Network,
  ) {}

  /** Newest first. A transaction the index could not interpret is not listed. */
  list = async (
    args: ListDexTransactionsArgs,
  ): Promise<ListDexTransactionsResult> => {
    const { path, body } = transactionsSearch(args, this.defaultNetwork);
    // A search only reads, so it retries like a GET.
    return normalizeDexTransactionsPage(
      await this.http.post<ListDexTransactionsWire>(path, body, {
        retry: true,
      }),
    );
  };

  get = async (args: GetDexTransactionArgs): Promise<DexTransaction> => {
    const response = await this.http.get<GetDexTransactionWire>(
      transactionPath(args, this.defaultNetwork),
    );
    return normalizeDexTransaction(
      requiredField(response.transaction, 'transaction'),
    );
  };
}

export class DexPositionsClient {
  constructor(
    private readonly http: HttpClient,
    private readonly defaultNetwork?: Network,
  ) {}

  list = async (
    args: ListDexPositionsArgs,
  ): Promise<ListDexPositionsResult> => {
    const { path, body } = positionsSearch(args, this.defaultNetwork);
    return normalizeDexPositionsPage(
      await this.http.post<ListDexPositionsWire>(path, body, { retry: true }),
    );
  };
}
