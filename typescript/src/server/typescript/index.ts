import { PublicKey } from '@solana/web3.js';
import {
  DEFAULT_BACKEND_URL,
  HttpClient,
  resolveRetryOptions,
} from '../../core/index.js';
import { DexClient } from '../../dex/index.js';
import { ParticipantSetsClient } from '../../participant-sets/index.js';
import { PaymasterClient } from '../../paymaster/index.js';
import { RoleTemplatesClient } from '../../policy-templates/roles.js';
import { WalletTemplatesClient } from '../../policy-templates/wallets.js';
import { RampClient } from '../../ramp/index.js';
import { TransactionsClient } from '../../transactions/index.js';
import type { SwigClientConfig } from '../../types/index.js';
import {
  type GetProjectSummaryArgs,
  type ProjectSummary,
  type RecentProjectWallet,
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
  readonly roleTemplates: RoleTemplatesClient;
  readonly walletTemplates: WalletTemplatesClient;
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
    this.roleTemplates = new RoleTemplatesClient(this.#http);
    this.walletTemplates = new WalletTemplatesClient(this.#http);
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
    const recent = 'recentWallets' in response ? response.recentWallets : [];
    if (!Array.isArray(recent) || recent.length > 5) {
      throw new Error('Invalid project summary recent wallets');
    }
    const recentWallets: RecentProjectWallet[] = [];
    const configurations = new Set<string>();
    for (const item of recent) {
      if (!item || typeof item !== 'object' || Array.isArray(item)) {
        throw new Error('Invalid project summary recent wallet');
      }
      const config: unknown = item.swigConfigAddress;
      const funds: unknown = item.walletAddress;
      const active: unknown = 'active' in item ? item.active : false;
      if (
        typeof config !== 'string' ||
        typeof funds !== 'string' ||
        typeof active !== 'boolean'
      ) {
        throw new Error('Invalid project summary recent wallet');
      }
      try {
        if (
          new PublicKey(config).toBase58() !== config ||
          new PublicKey(funds).toBase58() !== funds
        ) {
          throw new Error('Invalid address');
        }
      } catch {
        throw new Error('Invalid project summary recent wallet address');
      }
      if (configurations.has(config)) {
        throw new Error('Invalid project summary duplicate wallet');
      }
      configurations.add(config);
      recentWallets.push({
        swigConfigAddress: config,
        walletAddress: funds,
        active,
      });
    }
    return {
      organizationId,
      projectName,
      network,
      walletsCreated,
      tvlUsd,
      unpricedAssetCount,
      recentWallets,
    };
  };
}
