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
  positionsPath,
  transactionPath,
  transactionsPath,
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
  ): Promise<ListDexTransactionsResult> =>
    normalizeDexTransactionsPage(
      await this.http.get<ListDexTransactionsWire>(
        transactionsPath(args, this.defaultNetwork),
      ),
    );

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

  list = async (args: ListDexPositionsArgs): Promise<ListDexPositionsResult> =>
    normalizeDexPositionsPage(
      await this.http.get<ListDexPositionsWire>(
        positionsPath(args, this.defaultNetwork),
      ),
    );
}
