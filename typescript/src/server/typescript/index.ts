import {
  DEFAULT_BACKEND_URL,
  HttpClient,
  resolveRetryOptions,
} from '../../core/index.js';
import { DexClient } from '../../dex/index.js';
import { ParticipantSetsClient } from '../../participant-sets/index.js';
import { PaymasterClient } from '../../paymaster/index.js';
import { RampClient } from '../../ramp/index.js';
import { TransactionsClient } from '../../transactions/index.js';
import type { SwigClientConfig } from '../../types/index.js';
import {
  type GetProjectSummaryArgs,
  type ProjectSummary,
} from '../../types/project.js';
import { WalletsClient } from '../../wallets/index.js';
import { YieldClient } from '../../yield/index.js';

export class SwigClient {
  readonly #http: HttpClient;
  readonly defaultNetwork: SwigClientConfig['network'];
  readonly dex: DexClient;
  readonly paymaster: PaymasterClient;
  readonly participantSets: ParticipantSetsClient;
  readonly ramp: RampClient;
  readonly transactions: TransactionsClient;
  readonly wallets: WalletsClient;
  /** Yield on Swig wallets; native staking is the first method. */
  readonly yield: YieldClient;

  constructor(config: SwigClientConfig) {
    this.defaultNetwork = config.network;
    this.#http = new HttpClient({
      apiKey: config.apiKey,
      baseUrl: config.baseUrl ?? DEFAULT_BACKEND_URL,
      fetch: config.fetch ?? fetch,
      retry: resolveRetryOptions(config.retryOptions),
    });
    this.dex = new DexClient(this.#http, this.defaultNetwork);
    this.paymaster = new PaymasterClient(this.#http, this.defaultNetwork);
    this.participantSets = new ParticipantSetsClient(
      this.#http,
      this.defaultNetwork,
    );
    this.ramp = new RampClient(this.#http, this.defaultNetwork);
    this.transactions = new TransactionsClient(this.#http, this.defaultNetwork);
    this.wallets = new WalletsClient(this.#http, this.defaultNetwork);
    this.yield = new YieldClient(this.#http, this.defaultNetwork);
  }

  getProjectSummary = async (
    args: GetProjectSummaryArgs = {},
  ): Promise<ProjectSummary> => {
    const network = args.network ?? this.defaultNetwork;
    if (network !== 'mainnet' && network !== 'devnet') {
      throw new Error('network must be mainnet or devnet');
    }
    const wireNetwork =
      network === 'mainnet' ? 'NETWORK_MAINNET' : 'NETWORK_DEVNET';
    const response = await this.#http.get<unknown>(
      `/wallet/api/project/summary?network=${wireNetwork}`,
    );
    if (!response || typeof response !== 'object' || Array.isArray(response)) {
      throw new Error('Invalid project summary response');
    }
    const organizationId =
      'organizationId' in response ? response.organizationId : undefined;
    const projectName = 'projectName' in response ? response.projectName : '';
    const returnedNetwork =
      'network' in response ? response.network : undefined;
    const walletsCreated =
      'walletsCreated' in response ? response.walletsCreated : '0';
    const tvlUsd = 'tvlUsd' in response ? response.tvlUsd : 0;
    const unpricedAssetCount =
      'unpricedAssetCount' in response ? response.unpricedAssetCount : '0';
    if (
      typeof organizationId !== 'string' ||
      !organizationId ||
      typeof projectName !== 'string' ||
      returnedNetwork !== wireNetwork ||
      typeof walletsCreated !== 'string' ||
      !/^(0|[1-9][0-9]*)$/.test(walletsCreated) ||
      BigInt(walletsCreated) > 18446744073709551615n ||
      typeof tvlUsd !== 'number' ||
      !Number.isFinite(tvlUsd) ||
      tvlUsd < 0 ||
      typeof unpricedAssetCount !== 'string' ||
      !/^(0|[1-9][0-9]*)$/.test(unpricedAssetCount) ||
      BigInt(unpricedAssetCount) > 18446744073709551615n
    ) {
      throw new Error('Invalid project summary response');
    }
    return {
      organizationId,
      projectName,
      network,
      walletsCreated,
      tvlUsd,
      unpricedAssetCount,
    };
  };
}
